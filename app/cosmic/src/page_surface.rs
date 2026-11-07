//! The page editing surface (Phase 2.6, `page-surface` feature).
//!
//! It draws the layout engine's display list (pages, frames, and lines)
//! through one view transform ([`PageView`]) and edits any flow by publishing
//! [`PageAction`]s; the application turns them into commands. It owns no
//! text. Each line is drawn as Iced paragraphs (one per run of text between
//! inline objects) at the x the engine computed, and those paragraphs also
//! answer hit tests and caret positions, so what is drawn is what is hit.
//! A line's paragraphs are cached by its paragraph's shared value, so an edit
//! reshapes only the edited paragraph's lines.

use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

use amipro_document::{
    Colour, Document, FlowId, Frame, FrameContent, InlineObject, Paragraph, ParagraphId,
    ParagraphStyle, TabLeader, rule_strokes,
};
use amipro_editing::TextPosition;
use amipro_layout::{
    BulletBox, BulletPiece, Container, DocumentLayout, FrameBox, FrameBoxKind, Layer, LineBox,
    PageBox, display_spans, display_text,
};
use cosmic::iced::advanced;
use cosmic::iced::advanced::graphics::text::{Version, font_system};
use cosmic::iced::advanced::text::{self, Affinity, Paragraph as _, Span};
use cosmic::iced::advanced::widget::{self, Widget};
use cosmic::iced::advanced::{
    Clipboard, InputMethod, Layout, Shell, image, input_method, layout, renderer,
};
use cosmic::iced::keyboard::{self, Key, key::Named};
use cosmic::iced::{
    Background, Border, Color, Element, Event, Font, Length, Pixels, Point, Rectangle, Size,
    Vector, font, mouse,
};

use crate::command::{self, CommandId, Implementation};
use crate::frame_editing::{self, FrameAction, FrameSelection, Handle};
use crate::page_editing::{PageAction, PageSelection};
use crate::page_layout::{ascent_em, average_width_em, descent_em, display_family, tightness_em};
use crate::page_view::PageView;

const SELECTION: Color = Color::from_rgba(0.2, 0.45, 0.9, 0.28);
/// The find bar's matches (`UI_SPEC.md` §27), under the selection, which
/// shows the current one: a marker yellow, on the white page in both
/// themes.
const MATCH: Color = Color::from_rgba(1.0, 0.78, 0.0, 0.4);
/// View › Bookmarks' brackets (NR5).
const BOOKMARK_GREY: Color = Color::from_rgb(0.55, 0.55, 0.55);
/// A note mark's height on a line that has none to give it (twips).
const LINE_FALLBACK: f32 = 192.0;
/// Field Shading (`UI_SPEC.md` §28): a light grey behind each field's
/// shown text, on the page's white in both themes.
/// Spelling's underline, and grammar's (`UI_SPEC.md` §35).
const MISSPELLED: Color = Color::from_rgb(0.86, 0.1, 0.1);
const GRAMMAR: Color = Color::from_rgb(0.15, 0.4, 0.9);
const FIELD_SHADE: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.12);

/// Text to highlight: byte ranges by flow and paragraph (the find bar's
/// matches).
pub type Highlights = std::collections::HashMap<(FlowId, usize), Vec<std::ops::Range<usize>>>;
const PAGE_EDGE: Color = Color::from_rgb(0.55, 0.55, 0.55);
pub(crate) const PAGE_SHADOW: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.22);
/// The page's margins, as Ami Pro's Layout mode shades them (View Preferences
/// › Margins in color, on by default; VM: 223, 223, 223 on white paper).
pub(crate) const MARGIN_SHADE: Color = Color::from_rgb8(223, 223, 223);
/// View › Show ▸ Tabs & Returns' marks and Column Guides' lines.
const SHOW_MARK: Color = Color::from_rgb(0.35, 0.5, 0.75);
const COLUMN_GUIDE: Color = Color::from_rgba(0.35, 0.5, 0.75, 0.6);

/// View › Show ▸'s marks (`UI_SPEC.md` §39, CM10), on screen only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ViewMarks {
    pub tabs_and_returns: bool,
    pub column_guides: bool,
    pub margins_in_colour: bool,
    /// Letters over and numbers beside a page table's cells (TD.7, TN9).
    pub table_addresses: bool,
}

impl Default for ViewMarks {
    fn default() -> Self {
        Self {
            tabs_and_returns: false,
            column_guides: false,
            margins_in_colour: true,
            table_addresses: false,
        }
    }
}
const PLACEHOLDER: Color = Color::from_rgb(0.9, 0.9, 0.9);
const CELL_LINE: Color = Color::from_rgb(0.6, 0.6, 0.6);
/// Frame selection outlines and handles (`UI_SPEC.md` §11).
const ACCENT: Color = Color::from_rgb(0.2, 0.45, 0.9);
/// A handle's drawn side and its hit target, logical pixels at every zoom.
const HANDLE_SIZE: f32 = 7.0;
const HANDLE_TARGET: f32 = 12.0;
/// How far the pointer moves before a press on a frame becomes a drag.
const DRAG_THRESHOLD: f32 = 3.0;

/// What a frame drag does.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DragKind {
    Move,
    Resize(Handle),
    /// Picture mode: the picture moves in its frame (`UI_SPEC.md` §25, P6).
    Pan,
}

/// A frame drag in progress (page twips on one page).
#[derive(Clone, Copy, Debug, PartialEq)]
struct FrameDrag {
    kind: DragKind,
    page_index: usize,
    start: (f32, f32),
    current: (f32, f32),
    /// The press, in workspace pixels.
    press: Point,
    /// The pointer went past the drag threshold.
    moved: bool,
    /// The press was on a frame already selected (a click in a selected
    /// table enters the cell; the first click only selects).
    on_selected: bool,
}

impl FrameDrag {
    /// The drag's offset in twips.
    fn delta(self) -> (amipro_document::Twips, amipro_document::Twips) {
        (
            (self.current.0 - self.start.0).round() as amipro_document::Twips,
            (self.current.1 - self.start.1).round() as amipro_document::Twips,
        )
    }
}

/// Drawing a frame (`UI_SPEC.md` §20): from the press, on one page, in page
/// twips; the moving corner stays on the paper (`1268:191f`, `1268:1971`).
#[derive(Clone, Copy, Debug, PartialEq)]
struct FrameDraw {
    page_index: usize,
    start: (f32, f32),
    current: (f32, f32),
    /// The press, in workspace pixels.
    press: Point,
    /// The pointer went past the drag threshold: a smaller drag is a click,
    /// which draws nothing (FC4).
    moved: bool,
}

/// Which cached lines a key names: a paragraph's byte range at one scale.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct LineKey {
    flow: FlowId,
    paragraph: ParagraphId,
    bytes: Range<usize>,
    scale: u32,
    hyphen: bool,
    /// The page number, for lines whose page-number fields show it.
    field_page: u32,
    /// What the line's fields show (page numbers, dates, Doc Info), which
    /// may change while the line's text does not.
    field_texts: String,
    /// The caret is in this line's cell: its number shows as stored (TD.5).
    plain: bool,
}

/// One run of a line between inline objects, or a page-number field.
struct Segment<P> {
    /// The source bytes it shows.
    bytes: Range<usize>,
    /// Twips from the page's left edge.
    x: f32,
    paragraph: P,
    /// Display bytes that map one to one onto source bytes (a trailing hard
    /// return is not shown, a shown hyphen is extra).
    shown: usize,
    /// A page-number field: hits go to its start or end.
    field: bool,
    /// A table cell's number shown through its style: a hit goes to the
    /// end of the stored text.
    number: bool,
    /// Lines and shifts drawn with some of `paragraph`'s spans.
    decorations: Vec<Decoration>,
    /// The segment again with only its raised or lowered spans visible
    /// (superscript, subscript), drawn that many pixels higher: their glyphs
    /// keep `paragraph`'s positions, which draws them transparent.
    shifted: Vec<(f32, P)>,
    /// Overstrike characters, each shaped once with its offsets in pixels
    /// from the segment's position (`overstrike_glyphs`).
    overstrikes: Vec<(P, Vec<Vector>)>,
}

/// What the surface draws for a span of a segment besides its glyphs
/// (`docs/research/SAM_FORMAT_NOTES.md`, "Character attributes").
#[derive(Clone, Copy, Debug, PartialEq)]
struct Decoration {
    /// The span's index in the segment's paragraph.
    span: usize,
    underline: bool,
    double_underline: bool,
    strikethrough: bool,
    /// The character drawn over each of the span's (Special Effects'
    /// Overstrike; VM fixture O1).
    overstrike: Option<char>,
    /// How far the span's baseline is above the line's, in pixels.
    rise: f32,
    /// The size the span is drawn at, in pixels.
    size: f32,
    colour: Color,
}

/// The overstrike characters of a segment's spans (VM fixture O1): each
/// overstruck character gets the span's overstrike character in its font
/// and size, centred over it and raised with it; the text keeps its
/// widths. Offsets are in pixels from the segment's position.
fn overstrike_glyphs<P: text::Paragraph<Font = Font>>(
    spans: &[Span<'static, (), Font>],
    decorations: &[Decoration],
    size: f32,
    height: f32,
) -> Vec<(P, Vec<Vector>)> {
    if decorations
        .iter()
        .all(|decoration| decoration.overstrike.is_none())
    {
        return Vec::new();
    }
    let whole: P = shaped_text(spans, size, height);
    let total: usize = spans.iter().map(|span| span.text.chars().count()).sum();
    let x_of = |index: usize| {
        if index >= total {
            whole.min_bounds().width
        } else {
            whole
                .grapheme_position(0, index)
                .map_or(0.0, |point| point.x)
        }
    };
    let mut glyphs = Vec::new();
    for decoration in decorations {
        let Some(character) = decoration.overstrike else {
            continue;
        };
        let span = &spans[decoration.span];
        let first: usize = spans[..decoration.span]
            .iter()
            .map(|span| span.text.chars().count())
            .sum();
        let mut glyph_span = Span::new(character.to_string()).size(Pixels(decoration.size));
        glyph_span.font = span.font;
        glyph_span.color = Some(decoration.colour);
        let glyph: P = shaped_text(&[glyph_span], size, height);
        let glyph_width = glyph.min_bounds().width;
        let offsets = span
            .text
            .chars()
            .enumerate()
            .filter(|(_, shown)| !shown.is_whitespace())
            .map(|(index, _)| {
                let (left, right) = (x_of(first + index), x_of(first + index + 1));
                Vector::new(left + (right - left - glyph_width) / 2.0, -decoration.rise)
            })
            .collect();
        glyphs.push((glyph, offsets));
    }
    glyphs
}

/// Makes the paragraphs that draw `spans`' raised and lowered text, one per
/// shift, and makes those spans transparent in `spans`.
fn shifted_text<P: text::Paragraph<Font = Font>>(
    spans: &mut [Span<'static, (), Font>],
    decorations: &[Decoration],
    size: f32,
    height: f32,
) -> Vec<(f32, P)> {
    let mut rises: Vec<f32> = decorations
        .iter()
        .map(|decoration| decoration.rise)
        .filter(|rise| *rise != 0.0)
        .collect();
    rises.sort_by(f32::total_cmp);
    rises.dedup();
    let shifted = rises
        .into_iter()
        .map(|rise| {
            let visible: Vec<_> = spans
                .iter()
                .enumerate()
                .map(|(index, span)| {
                    let shown = decorations
                        .iter()
                        .any(|decoration| decoration.span == index && decoration.rise == rise);
                    if shown {
                        span.clone()
                    } else {
                        span.clone().color(Color::TRANSPARENT)
                    }
                })
                .collect();
            (rise, shaped_text(&visible, size, height))
        })
        .collect();
    for decoration in decorations
        .iter()
        .filter(|decoration| decoration.rise != 0.0)
    {
        spans[decoration.span].color = Some(Color::TRANSPARENT);
    }
    shifted
}

struct LineCache<P> {
    /// Keeps the paragraph alive so its pointer identifies its content.
    source: Option<Arc<Paragraph>>,
    style: Option<ParagraphStyle>,
    /// The line's word positions: segments hold absolute positions, so a
    /// line whose words moved (alignment, a new slot) is built again.
    word_x: Vec<f32>,
    segments: Vec<Segment<P>>,
    /// The line's bullet or number as laid out, and its text pieces shaped,
    /// with their x in twips from the page's left edge.
    bullet: Option<BulletBox>,
    bullet_text: Vec<BulletText<P>>,
    /// The bullets put into the line's text (`<:B…>`).
    inline_bullets: Vec<InlineBullet>,
    /// How far the text is drawn below where its shaped paragraph puts it,
    /// in pixels: in a line a Flow with text frame makes taller, COSMIC Text
    /// would centre the text, which stands on the baseline (PI1).
    text_shift: f32,
}

/// A piece of a bullet's or number's text, shaped: its x in twips from the
/// page's left edge and how far it is raised, in pixels.
struct BulletText<P> {
    x: f32,
    rise: f32,
    paragraph: P,
}

/// A bullet put into the text, as drawn: its place and width in twips
/// from the page's left edge, its code, the ascent of the font it stands
/// in (which sizes it as a style bullet), and that text's colour.
#[derive(Clone, Debug, PartialEq)]
struct InlineBullet {
    bytes: std::ops::Range<usize>,
    x: f32,
    width: f32,
    code: char,
    size: f32,
    colour: Color,
}

struct PageCache<P> {
    page: Arc<PageBox>,
    /// One per line of `page`, in its order.
    lines: Vec<Rc<LineCache<P>>>,
}

/// A page's line numbers (Page › Line Numbering) as laid out, and each
/// shaped with its top-left corner in twips from the page's corner.
struct NumberCache<P> {
    boxes: Vec<amipro_layout::LineNumberBox>,
    texts: Vec<(f32, f32, P)>,
}

struct State<P> {
    lines: HashMap<LineKey, Rc<LineCache<P>>>,
    pages: Vec<PageCache<P>>,
    /// One per page with line numbers, in page order.
    numbers: Vec<NumberCache<P>>,
    scale: u32,
    session: u64,
    fonts: Option<Version>,
    /// Whether protected text is hidden (printing on preprinted form).
    hide_protected: bool,
    /// The flow the caret is in alone: a numeric cell there shows its
    /// stored text (TD.5).
    editing: Option<FlowId>,
    /// Line caches built, for tests and measurement.
    built: usize,
    focused: bool,
    /// The last keyboard request seen (`PageSurface::keyboard`).
    keyboard_seen: u64,
    window_focused: bool,
    /// Where the pointer (or a drag over the page) last was, for a drop.
    pointer: Option<Point>,
    dragging: bool,
    drag_hit: Option<(TextPosition, u32)>,
    modifiers: keyboard::Modifiers,
    preedit: Option<input_method::Preedit>,
    viewport: Size,
    frame_drag: Option<FrameDrag>,
    frame_draw: Option<FrameDraw>,
    /// A table's grid line being dragged (`UI_SPEC.md` §32, TD6).
    track_drag: Option<TrackDrag>,
    /// Selected cells being dragged to another place (`1498:2d2a`, TD9):
    /// where the press was, and the cell under the pointer.
    cell_drag: Option<CellDrag>,
    /// Draw mode: the left button went down in the drawing's frame.
    draw_press: bool,
    last_click: Option<advanced::mouse::Click>,
}

/// Selected cells being dragged: the press, and the target cell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellDrag {
    pub frame: amipro_document::FrameId,
    pub press: Point,
    pub moved: bool,
    pub target: Option<(usize, usize)>,
}

/// A table's column or row line under the pointer, or being dragged
/// (`1498:25fa`, `1498:20b7`): the line after column (or row) `index`,
/// its column's (row's) text start, and how far it may go, in page twips.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackDrag {
    pub frame: amipro_document::FrameId,
    pub page_index: usize,
    /// A column's line when true, else a row's.
    pub column: bool,
    pub index: usize,
    /// Where the column's (row's) text starts.
    pub origin: f32,
    /// The line's place now: the track's far edge.
    pub at: f32,
    pub limits: (f32, f32),
    /// The line's place and the pointer's when pressed: the line moves as
    /// far as the pointer does.
    pub start: f32,
    pub press: Option<f32>,
}

impl<P> Default for State<P> {
    fn default() -> Self {
        Self {
            lines: HashMap::new(),
            pages: Vec::new(),
            numbers: Vec::new(),
            scale: 0,
            session: 0,
            fonts: None,
            hide_protected: false,
            editing: None,
            built: 0,
            focused: false,
            keyboard_seen: 0,
            window_focused: true,
            pointer: None,
            dragging: false,
            drag_hit: None,
            modifiers: keyboard::Modifiers::default(),
            preedit: None,
            viewport: Size::ZERO,
            frame_drag: None,
            frame_draw: None,
            track_drag: None,
            cell_drag: None,
            draw_press: false,
            last_click: None,
        }
    }
}

/// How the surface draws for paper (`UI_SPEC.md` §29): without what only
/// the screen shows (shading, highlights, selection, the caret, handles,
/// guides), and with Ami Pro's print options (`PRINT_NOTES.md`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct PrintMode {
    /// Without pictures: picture frames print empty, their lines kept.
    pub without_pictures: bool,
    /// On preprinted form: no frame or table lines, shadows, fills,
    /// pictures, page border or column lines, or protected text.
    pub preprinted_form: bool,
    /// On screen while the Print panel is open (PO5): the pages as they
    /// print, with their edges, the caret, and the selection still drawn,
    /// so that the document stays editable.
    pub preview: bool,
}

/// The surface over one document session's pages.
pub struct PageSurface<'a, Message> {
    document: &'a Document,
    layout: &'a DocumentLayout,
    view: PageView,
    session: u64,
    selection: PageSelection,
    on_action: Box<dyn Fn(PageAction) -> Message + 'a>,
    on_command: Box<dyn Fn(CommandId) -> Message + 'a>,
    /// The selected frames, if any: then there is no caret.
    frames: Option<&'a FrameSelection>,
    /// Without it, clicks in frames enter their text (as before §11).
    on_frame: Option<Box<dyn Fn(FrameAction) -> Message + 'a>>,
    /// While a ruler marker moves: a guide down that page at this x (page
    /// index, twips; `UI_SPEC.md` §19).
    guide: Option<(usize, f32)>,
    /// Insert › Draw Frame: the next press on a page draws a frame
    /// (`UI_SPEC.md` §20).
    drawing: bool,
    /// Fast Format is on: a release after a press in text says so
    /// ([`PageAction::PointerReleased`]).
    fast_format: bool,
    /// View › Show ▸'s marks.
    view_marks: ViewMarks,
    /// The session's decoded pictures and the document's folder, where
    /// linked pictures are looked for (`UI_SPEC.md` §25). Without them,
    /// pictures are boxes.
    pictures: Option<(&'a crate::pictures::Pictures, Option<&'a std::path::Path>)>,
    /// Picture mode on the selected picture frame (`UI_SPEC.md` §25, P6):
    /// a drag or an arrow key moves the picture in its frame.
    picture_mode: bool,
    /// Text highlighted under the selection: the find bar's matches.
    highlights: Option<&'a Highlights>,
    /// Field Shading's ranges (`UI_SPEC.md` §28).
    shading: Option<&'a Highlights>,
    /// Words underlined as you type (`UI_SPEC.md` §35): spelling in red,
    /// grammar in blue; never printed.
    misspelled: Option<&'a Highlights>,
    grammar: Option<&'a Highlights>,
    /// The pages as they print (`UI_SPEC.md` §29): for the PDF, and on
    /// screen while the Print panel is open (PO5).
    printing: Option<PrintMode>,
    /// Draw mode on a drawing frame (`UI_SPEC.md` §33): presses, moves,
    /// and keys go to it, and its overlay is drawn over the drawing.
    draw: Option<DrawView>,
    on_draw: Option<Box<dyn Fn(crate::draw_mode::Event) -> Message + 'a>>,
    /// View › Notes: note marks drawn (on by default, as Ami Pro's View
    /// Preferences › Notes; `UI_SPEC.md` §34).
    notes: bool,
    /// View › Bookmarks: bookmarked text bracketed (NR5).
    bookmarks: bool,
    /// The app's request for the keyboard: the page takes it when this
    /// differs from the last one seen.
    keyboard: u64,
    /// A modal dialog is open over the window: the page takes no keys or
    /// typed text, which are the dialog's.
    modal: bool,
    /// The Outline navigator holds the plain arrow keys (OB.6).
    arrows_elsewhere: bool,
    /// Outline mode (OM1): each main paragraph's icon, drawn left of its
    /// first line.
    outline: Option<&'a [crate::outline_view::OutlineIcon]>,
}

/// What the surface needs of Draw mode (`UI_SPEC.md` §33).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DrawView {
    pub placement: crate::draw_mode::Placement,
    pub overlay: crate::draw_mode::Overlay,
    /// Pointer moves without a button go to the mode (a polyline).
    pub tracking: bool,
    pub tool: crate::draw_mode::Tool,
    /// A text object is being typed: characters go to it.
    pub typing: bool,
}

pub fn page_surface<'a, Message>(
    document: &'a Document,
    layout: &'a DocumentLayout,
    view: PageView,
    session: u64,
    selection: PageSelection,
    on_action: impl Fn(PageAction) -> Message + 'a,
    on_command: impl Fn(CommandId) -> Message + 'a,
) -> PageSurface<'a, Message> {
    PageSurface {
        document,
        layout,
        view,
        session,
        selection,
        on_action: Box::new(on_action),
        on_command: Box::new(on_command),
        frames: None,
        on_frame: None,
        guide: None,
        drawing: false,
        fast_format: false,
        view_marks: ViewMarks::default(),
        pictures: None,
        picture_mode: false,
        highlights: None,
        shading: None,
        misspelled: None,
        grammar: None,
        printing: None,
        draw: None,
        on_draw: None,
        notes: true,
        bookmarks: false,
        keyboard: 0,
        modal: false,
        arrows_elsewhere: false,
        outline: None,
    }
}

impl<'a, Message> PageSurface<'a, Message> {
    /// Frames are selected, moved, and resized (`UI_SPEC.md` §11):
    /// `selection` is the current frame selection.
    pub fn frames(
        mut self,
        selection: Option<&'a FrameSelection>,
        on_frame: impl Fn(FrameAction) -> Message + 'a,
    ) -> Self {
        self.frames = selection;
        self.on_frame = Some(Box::new(on_frame));
        self
    }

    /// The page takes the keyboard when `request` differs from the last
    /// request it saw, as after a click on it.
    pub fn keyboard(mut self, request: u64) -> Self {
        self.keyboard = request;
        self
    }

    /// While a modal dialog is open the page ignores the keyboard.
    pub fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    /// While the Outline navigator holds the keys the page leaves the plain
    /// arrow keys to the application.
    pub fn arrows_elsewhere(mut self, elsewhere: bool) -> Self {
        self.arrows_elsewhere = elsewhere;
        self
    }

    /// Outline mode's icons, one per main paragraph.
    pub fn outline(mut self, icons: Option<&'a [crate::outline_view::OutlineIcon]>) -> Self {
        self.outline = icons;
        self
    }

    /// A guide line down a page while a ruler marker moves.
    pub fn guide(mut self, guide: Option<(usize, f32)>) -> Self {
        self.guide = guide;
        self
    }

    /// Insert › Draw Frame: a press on a page and a drag draw a frame
    /// (`UI_SPEC.md` §20).
    pub fn drawing_frame(mut self, drawing: bool) -> Self {
        self.drawing = drawing;
        self
    }

    /// View › Show ▸'s marks (`UI_SPEC.md` §39, CM10).
    pub fn view_marks(mut self, marks: ViewMarks) -> Self {
        self.view_marks = marks;
        self
    }

    /// Format › Fast Format is on (`UI_SPEC.md` §39, CM4).
    pub fn fast_format(mut self, on: bool) -> Self {
        self.fast_format = on;
        self
    }

    /// Highlights text under the selection: the find bar's matches.
    /// The fields' shown text, shaded light grey (Field Shading).
    /// View › Notes and View › Bookmarks (`UI_SPEC.md` §34).
    pub fn marks(mut self, notes: bool, bookmarks: bool) -> Self {
        self.notes = notes;
        self.bookmarks = bookmarks;
        self
    }

    pub fn shading(mut self, shading: Option<&'a Highlights>) -> Self {
        self.shading = shading;
        self
    }

    pub fn highlights(mut self, highlights: Option<&'a Highlights>) -> Self {
        self.highlights = highlights;
        self
    }

    /// Spelling and grammar underlines (`UI_SPEC.md` §35, SP2, SP8).
    pub fn underlines(
        mut self,
        misspelled: Option<&'a Highlights>,
        grammar: Option<&'a Highlights>,
    ) -> Self {
        self.misspelled = misspelled;
        self.grammar = grammar;
        self
    }

    /// Picture mode on the selected picture frame (P6).
    pub(crate) fn picture_mode(mut self, on: bool) -> Self {
        self.picture_mode = on;
        self
    }

    /// Draw mode on a drawing frame (`UI_SPEC.md` §33).
    pub(crate) fn draw_mode(
        mut self,
        view: Option<DrawView>,
        on_draw: impl Fn(crate::draw_mode::Event) -> Message + 'a,
    ) -> Self {
        self.draw = view;
        self.on_draw = Some(Box::new(on_draw));
        self
    }

    /// The press at `position` in Draw mode: on the drawing's page and in
    /// its frame, its point in the drawing's twips.
    fn draw_point(&self, position: Point) -> Option<(crate::draw_mode::At, bool)> {
        let draw = self.draw.as_ref()?;
        let placement = draw.placement;
        let (x, y) = self.view.to_page(placement.page_index, position);
        let inside =
            self.view.page_at(position) == placement.page_index && placement.contains(x, y);
        Some((placement.to_drawing(x, y), inside))
    }

    /// Draws Draw mode's overlay: the page around the frame dimmed a
    /// little, then handles, outlines, the grid, guides, over the drawing.
    fn draw_draw_mode<Renderer>(&self, renderer: &mut Renderer, origin: Vector)
    where
        Renderer: renderer::Renderer + image::Renderer<Handle = image::Handle>,
    {
        let Some(draw) = &self.draw else {
            return;
        };
        let placement = draw.placement;
        let Some(paper) = self.view.pages.get(placement.page_index) else {
            return;
        };
        let paper = *paper + origin;
        let frame = self.view.rect(placement.page_index, placement.clip) + origin;
        let dim = Color::from_rgba(0.0, 0.0, 0.0, 0.07);
        for band in [
            Rectangle::new(
                paper.position(),
                Size::new(paper.width, (frame.y - paper.y).max(0.0)),
            ),
            Rectangle::new(
                Point::new(paper.x, frame.y + frame.height),
                Size::new(
                    paper.width,
                    (paper.y + paper.height - frame.y - frame.height).max(0.0),
                ),
            ),
            Rectangle::new(
                Point::new(paper.x, frame.y),
                Size::new((frame.x - paper.x).max(0.0), frame.height),
            ),
            Rectangle::new(
                Point::new(frame.x + frame.width, frame.y),
                Size::new(
                    (paper.x + paper.width - frame.x - frame.width).max(0.0),
                    frame.height,
                ),
            ),
        ] {
            if band.width > 0.0 && band.height > 0.0 {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: band,
                        ..renderer::Quad::default()
                    },
                    dim,
                );
            }
        }
        // The overlay a little beyond the frame, so handles on its edge show.
        let pad = 8.0;
        let area = Rectangle::new(
            Point::new((frame.x - pad).floor(), (frame.y - pad).floor()),
            Size::new(
                (frame.width + 2.0 * pad).ceil(),
                (frame.height + 2.0 * pad).ceil(),
            ),
        );
        let to_pixel = |at: crate::draw_mode::At| {
            let (x, y) = placement.to_page(at);
            let point = self.view.point(placement.page_index, x, y) + origin;
            (point.x - area.x, point.y - area.y)
        };
        let pixel_twips = f64::from(1.0 / (self.view.scale * placement.scale.0.max(1e-6)));
        if let Some(pixmap) = crate::draw_mode::render_overlay(
            &draw.overlay,
            area.width as u32,
            area.height as u32,
            &to_pixel,
            pixel_twips,
        ) {
            let handle = image::Handle::from_rgba(
                pixmap.width(),
                pixmap.height(),
                crate::metafile_render::rgba_of(&pixmap),
            );
            renderer.draw_image(image::Image::new(handle), area, area);
        }
    }

    /// Draws the pages as they print (`UI_SPEC.md` §29).
    pub(crate) fn printing(mut self, mode: Option<PrintMode>) -> Self {
        self.printing = mode;
        self
    }

    /// Draws pictures from `pictures`, linked ones looked for in `folder`.
    pub(crate) fn pictures(
        mut self,
        pictures: &'a crate::pictures::Pictures,
        folder: Option<&'a std::path::Path>,
    ) -> Self {
        self.pictures = Some((pictures, folder));
        self
    }

    /// View › Show ▸ Pictures off: pictures are drawn as boxes.
    pub(crate) fn pictures_shown(mut self, shown: bool) -> Self {
        if !shown {
            self.pictures = None;
        }
        self
    }
}

/// A table and the cells (row, column) at two corners of a column or row a
/// press on its strip selects.
type TrackStrip = (amipro_document::FrameId, (usize, usize), (usize, usize));

/// The marks a tab's leader draws across its gap, on screen (VM fixture
/// TB2): hyphens at the middle of the lower-case letters, dots on the
/// baseline, or a line on it, for text `size` pixels high.
fn leader_quads(line: &LineBox, scale: f32, left: f32, top: f32, size: f32) -> Vec<Rectangle> {
    let baseline = top + (line.baseline - line.top) * scale;
    let mut quads = Vec::new();
    for word in &line.words {
        let (Some(leader), true) = (word.tab, word.width > 0.0) else {
            continue;
        };
        let start = left + word.x * scale;
        let end = start + word.width * scale;
        quads.extend(leader_marks(leader, start, end, baseline, size));
    }
    quads
}

/// A leader's marks from `start` to `end` on screen on `baseline`, for text
/// `size` pixels high: hyphens at the middle of the lower-case letters, dots
/// on the baseline, or a line on it, whole marks from the end back, as
/// typed leaders end at the text.
fn leader_marks(
    leader: TabLeader,
    start: f32,
    end: f32,
    baseline: f32,
    size: f32,
) -> Vec<Rectangle> {
    let thickness = (size / 14.0).round().max(1.0);
    let mut quads = Vec::new();
    let mut repeat = |length: f32, step: f32, y: f32, height: f32| {
        let count = ((end - start - length) / step).floor().max(-1.0) as i32 + 1;
        for index in 0..count {
            let x = end - length - index as f32 * step;
            quads.push(Rectangle::new(
                Point::new(x.round(), y.round()),
                Size::new(length.max(1.0), height),
            ));
        }
    };
    match leader {
        TabLeader::None => {}
        TabLeader::Hyphens => repeat(size * 0.3, size * 0.45, baseline - size * 0.25, thickness),
        TabLeader::Dots => repeat(thickness, size * 0.3, baseline - thickness, thickness),
        TabLeader::Line => {
            if end > start {
                quads.push(Rectangle::new(
                    Point::new(start.round(), (baseline - thickness).round()),
                    Size::new(end - start, thickness),
                ));
            }
        }
    }
    quads
}

/// Where a cell's leader runs on its last line (Table › Leaders,
/// `13e8:2422`): after the text to the cell's right edge (left or justified
/// text), from the cell's left edge to the text (right aligned), across the
/// cell when it is empty, and nowhere for centred text. In page twips.
pub(crate) fn cell_leader_span(
    text: amipro_layout::Bounds,
    line: &LineBox,
    alignment: amipro_document::Alignment,
) -> Option<(f32, f32)> {
    let words: Vec<_> = line.words.iter().filter(|word| word.width > 0.0).collect();
    let (Some(first), Some(last)) = (words.first(), words.last()) else {
        return Some((text.x, text.x + text.width));
    };
    let (start, end) = match alignment {
        amipro_document::Alignment::Centre => return None,
        amipro_document::Alignment::Right => (text.x, first.x),
        amipro_document::Alignment::Left | amipro_document::Alignment::Justify => {
            (last.x + last.width, text.x + text.width)
        }
    };
    (end > start).then_some((start, end))
}

/// The strokes of a line's paragraph rules on screen, each on whole pixel
/// rows and at least one pixel thick, from `top`, the line's top on screen,
/// and `left`, its page's left edge on screen.
fn rule_quads(line: &LineBox, scale: f32, left: f32, top: f32) -> Vec<(Rectangle, Color)> {
    let mut quads = Vec::new();
    for rule in &line.rules {
        let mut offset = rule.top(line) - line.top;
        for (index, stroke) in rule_strokes(rule.line).iter().enumerate() {
            let stroke = *stroke as f32;
            if index % 2 == 0 {
                quads.push((
                    Rectangle::new(
                        Point::new(left + rule.x * scale, (top + offset * scale).round()),
                        Size::new(rule.width * scale, (stroke * scale).round().max(1.0)),
                    ),
                    colour(rule.colour),
                ));
            }
            offset += stroke;
        }
    }
    quads
}

/// A page's layout lines on screen: whole pixels, at least one wide.
fn stroke_quads(
    view: &PageView,
    page_index: usize,
    strokes: &[amipro_layout::PageStroke],
    origin: Vector,
) -> Vec<(Rectangle, Color)> {
    strokes
        .iter()
        .map(|stroke| {
            let bounds = view.rect(page_index, stroke.bounds) + origin;
            let (x, y) = (bounds.x.round(), bounds.y.round());
            (
                Rectangle::new(
                    Point::new(x, y),
                    Size::new(
                        (bounds.width.round()).max(1.0),
                        (bounds.height.round()).max(1.0),
                    ),
                ),
                colour(stroke.colour),
            )
        })
        .collect()
}

/// A negative number under a Red number format.
const NEGATIVE_RED: Color = Color::from_rgb(0.85, 0.0, 0.0);

fn colour(value: Colour) -> Color {
    Color::from_rgb8(
        (value & 0xFF) as u8,
        ((value >> 8) & 0xFF) as u8,
        ((value >> 16) & 0xFF) as u8,
    )
}

fn display_font(family: &str, bold: bool, italic: bool) -> Font {
    Font {
        family: font::Family::Name(display_family(family)),
        weight: if bold {
            font::Weight::Bold
        } else {
            font::Weight::Normal
        },
        style: if italic {
            font::Style::Italic
        } else {
            font::Style::Normal
        },
        ..Font::default()
    }
}

/// The paragraph a line shows, if the layout still matches the document.
fn source_of(document: &Document, line: &LineBox) -> Option<Arc<Paragraph>> {
    let paragraph = document
        .flow(line.flow)?
        .paragraphs
        .get(line.paragraph_index)?;
    (paragraph.id == line.paragraph).then(|| Arc::clone(paragraph))
}

fn shaped_text<P: text::Paragraph<Font = Font>>(
    spans: &[Span<'static, (), Font>],
    size: f32,
    height: f32,
) -> P {
    P::with_spans(text::Text {
        content: spans,
        bounds: Size::new(1.0e6, height.max(1.0)),
        size: Pixels(size.max(1.0)),
        line_height: text::LineHeight::Absolute(Pixels(height.max(1.0))),
        font: Font::default(),
        align_x: text::Alignment::Default,
        align_y: cosmic::iced::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
        ellipsize: text::Ellipsize::None,
    })
}

/// 1st line bold: a paragraph's first line (the line whose bytes start the
/// paragraph) is drawn bold whatever its runs say, and italic and the rest
/// combine with it; the layout broke it with the bold widths (decompile
/// `10d0:0309`, `FIRST_LINE_BOLD_NOTES.md`).
fn first_line_bold(
    display: &mut [amipro_layout::DisplaySpan<'_>],
    style: &ParagraphStyle,
    line: &Range<usize>,
) {
    if style.first_line_bold && line.start == 0 && !line.is_empty() {
        for span in display {
            span.bold = true;
        }
    }
}

/// Shapes one line's runs between objects.
fn build_line<P: text::Paragraph<Font = Font>>(
    document: &Document,
    line: &LineBox,
    scale: f32,
    hide_protected: bool,
    plain: bool,
) -> LineCache<P> {
    let word_x = line.words.iter().map(|word| word.x).collect();
    // A footnote's continued message is no text of the document: drawn as
    // its shown text in the Footnote style.
    let message = (line.paragraph_index == usize::MAX
        && matches!(line.container, Container::Footnote(_)))
    .then(|| {
        let style = document
            .styles()
            .values()
            .find(|style| style.name.eq_ignore_ascii_case("Footnote"))
            .map_or_else(|| document.body_text_style(), |style| style.id);
        Arc::new(Paragraph {
            id: line.paragraph,
            text: String::new(),
            style,
            format: Default::default(),
            runs: Vec::new(),
            objects: Vec::new(),
            marks: Vec::new(),
            end_format: Default::default(),
            cache: None,
        })
    });
    let Some(source) = message.or_else(|| source_of(document, line)) else {
        return LineCache {
            source: None,
            style: None,
            word_x,
            segments: Vec::new(),
            bullet: None,
            bullet_text: Vec::new(),
            inline_bullets: Vec::new(),
            text_shift: 0.0,
        };
    };
    let style = document
        .styles()
        .get(&source.style)
        .cloned()
        .unwrap_or_else(|| document.styles().values().next().cloned().expect("a style"));
    let height = line.height * scale;
    let base_size = style.font_size_points * 20.0 * scale;
    let style_font = display_font(&style.font_family, style.bold, style.italic);
    // The letters shown (caps) and the spans they are drawn in.
    let shown = display_text(&source.text, &style, &source.runs);
    let mut display = display_spans(&source.text, &style, &source.runs);
    first_line_bold(&mut display, &style, &line.bytes);
    let text_runs =
        |range: Range<usize>, extra: &str| -> (Vec<Span<'static, (), Font>>, Vec<Decoration>) {
            let mut spans = Vec::new();
            let mut decorations = Vec::new();
            for span in display
                .iter()
                .filter(|span| span.bytes.start < range.end && span.bytes.end > range.start)
            {
                let bytes = span.bytes.start.max(range.start)..span.bytes.end.min(range.end);
                // On preprinted form protected text is not printed, its
                // place kept (`PRINT_NOTES.md`; VM PT1OUT).
                let hidden = hide_protected && span.attributes.protected;
                let attributes = if hidden {
                    amipro_document::CharacterAttributes {
                        underline: false,
                        word_underline: false,
                        double_underline: false,
                        strikethrough: false,
                        overstrike: None,
                        ..span.attributes
                    }
                } else {
                    span.attributes
                };
                let size = span.size * 20.0 * scale;
                let colour = if hidden {
                    Color::TRANSPARENT
                } else {
                    colour(attributes.colour)
                };
                let rise = if attributes.is_script() {
                    let ascent = ascent_em(span.family, span.bold, span.italic) * span.size * 20.0;
                    span.baseline_shift(ascent) * scale
                } else {
                    0.0
                };
                // Word underline marks the words, not the spaces between them.
                let mut pieces = vec![bytes.clone()];
                if attributes.word_underline && !attributes.underline {
                    pieces.clear();
                    let mut start = bytes.start;
                    let mut blank = None;
                    for (offset, character) in shown[bytes.clone()].char_indices() {
                        let is_blank = character.is_whitespace();
                        if blank.is_some_and(|blank| blank != is_blank) {
                            pieces.push(start..bytes.start + offset);
                            start = bytes.start + offset;
                        }
                        blank = Some(is_blank);
                    }
                    pieces.push(start..bytes.end);
                }
                for piece in pieces {
                    let underline = attributes.underline
                        || (attributes.word_underline && !shown[piece.clone()].trim().is_empty());
                    if underline
                        || attributes.double_underline
                        || attributes.strikethrough
                        || attributes.overstrike.is_some()
                        || rise != 0.0
                    {
                        decorations.push(Decoration {
                            span: spans.len(),
                            underline,
                            double_underline: attributes.double_underline,
                            strikethrough: attributes.strikethrough,
                            overstrike: attributes.overstrike,
                            rise,
                            size,
                            colour,
                        });
                    }
                    spans.push(
                        Span::new(shown[piece].to_owned())
                            .font(display_font(span.family, span.bold, span.italic))
                            .size(Pixels(size))
                            .color(colour),
                    );
                }
            }
            if !extra.is_empty() {
                spans.push(
                    Span::new(extra.to_owned())
                        .font(style_font)
                        .size(Pixels(base_size)),
                );
            }
            (spans, decorations)
        };
    // Text Tightness: the twips added after the character at `byte`, its
    // drawn font's share (`page_layout::tightness_em`).
    let tracked = style.spacing.tightness != 100;
    let tracking = |byte: usize| -> f32 {
        display
            .iter()
            .find(|span| span.bytes.contains(&byte))
            .map_or(0.0, |span| {
                tightness_em(
                    average_width_em(span.family, span.bold, span.italic),
                    style.spacing.tightness,
                ) * span.size
                    * 20.0
            })
    };
    let mut words: Vec<_> = line.words.iter().collect();
    words.sort_by_key(|word| word.bytes.start);
    let mut segments = Vec::new();
    let mut inline_bullets = Vec::new();
    let mut index = 0;
    while index < words.len() {
        let word = words[index];
        if let Some(field) = word
            .text
            .as_ref()
            .filter(|_| !(plain && word.object.is_none() && !word.bytes.is_empty()))
        {
            // A page number, date, or Doc Info entry is drawn in the text
            // formatting where it stands, as Ami Pro draws it (VM fixtures
            // PF1, FD1).
            let (spans, decorations) = match display
                .iter()
                .find(|span| span.bytes.contains(&word.bytes.start))
            {
                Some(span) => {
                    let attributes = span.attributes;
                    let size = span.size * 20.0 * scale;
                    let colour = if word.red {
                        NEGATIVE_RED
                    } else {
                        colour(attributes.colour)
                    };
                    let underline = attributes.underline || attributes.word_underline;
                    let decorations =
                        if underline || attributes.double_underline || attributes.strikethrough {
                            vec![Decoration {
                                span: 0,
                                underline,
                                double_underline: attributes.double_underline,
                                strikethrough: attributes.strikethrough,
                                overstrike: None,
                                rise: 0.0,
                                size,
                                colour,
                            }]
                        } else {
                            Vec::new()
                        };
                    (
                        vec![
                            Span::new(field.clone())
                                .font(display_font(span.family, span.bold, span.italic))
                                .size(Pixels(size))
                                .color(colour),
                        ],
                        decorations,
                    )
                }
                None => (
                    vec![
                        Span::new(field.clone())
                            .font(style_font)
                            .size(Pixels(base_size))
                            .color(if word.red { NEGATIVE_RED } else { Color::BLACK }),
                    ],
                    Vec::new(),
                ),
            };
            // A footnote's reference is its number set as superscript:
            // 7/12 of the size, raised by 5/12 of it (`1308:0ebd`, the
            // Footnote style's numbering; VM RNF1).
            let reference = word
                .object
                .and_then(|object| source.objects.get(object))
                .is_some_and(|object| {
                    matches!(
                        object,
                        InlineObject::Footnote { .. } | InlineObject::TableFootnote { .. }
                    )
                });
            if reference {
                let size = display
                    .iter()
                    .find(|span| span.bytes.contains(&word.bytes.start))
                    .map_or(style.font_size_points, |span| span.size)
                    * 20.0
                    * scale;
                let spans: Vec<_> = spans
                    .into_iter()
                    .map(|span| span.size(Pixels(size * 7.0 / 12.0)))
                    .collect();
                segments.push(Segment {
                    bytes: word.bytes.clone(),
                    x: word.x,
                    paragraph: shaped_text(&[], base_size, height),
                    shown: 0,
                    field: true,
                    number: word.object.is_none(),
                    decorations: Vec::new(),
                    shifted: vec![(size * 5.0 / 12.0, shaped_text(&spans, base_size, height))],
                    overstrikes: Vec::new(),
                });
                index += 1;
                continue;
            }
            segments.push(Segment {
                bytes: word.bytes.clone(),
                x: word.x,
                paragraph: shaped_text(&spans, base_size, height),
                shown: 0,
                field: true,
                number: word.object.is_none(),
                decorations,
                shifted: Vec::new(),
                overstrikes: Vec::new(),
            });
            index += 1;
            continue;
        }
        if let Some(object) = word.object {
            // A bullet put into the text is drawn as the style bullet with
            // its code, in the font and colour where it stands (BL1).
            if let Some(InlineObject::Bullet(code)) = source.objects.get(object) {
                let span = display
                    .iter()
                    .find(|span| span.bytes.contains(&word.bytes.start));
                let size = span.map_or(style.font_size_points, |span| span.full_size);
                inline_bullets.push(InlineBullet {
                    bytes: word.bytes.clone(),
                    x: word.x,
                    width: word.width,
                    code: *code,
                    size: amipro_layout::bullet_metrics(size).0,
                    colour: colour(
                        span.map_or(style.attributes.colour, |span| span.attributes.colour),
                    ),
                });
            }
            index += 1;
            continue;
        }
        // A tab draws no text, only its leader (`leader_quads`); the caret
        // after it stands at its end.
        if word.tab.is_some() {
            segments.push(Segment {
                bytes: word.bytes.end..word.bytes.end,
                x: word.x + word.width,
                paragraph: shaped_text(&[], base_size, height),
                shown: 0,
                field: false,
                number: false,
                decorations: Vec::new(),
                shifted: Vec::new(),
                overstrikes: Vec::new(),
            });
            index += 1;
            continue;
        }
        // A run of words without objects; on a justified line, whose spaces
        // are wider than the font's, or a tightened one, each word on its
        // own.
        let start = word.bytes.start;
        let mut end = word.bytes.end;
        let mut x = word.x;
        index += 1;
        while !line.spaced
            && !tracked
            && index < words.len()
            && words[index].object.is_none()
            && words[index].tab.is_none()
            && words[index].text.is_none()
        {
            end = words[index].bytes.end;
            x = x.min(words[index].x);
            index += 1;
        }
        let last = index >= words.len();
        let shown_end = if source.text[start..end].ends_with('\u{2028}') {
            end - '\u{2028}'.len_utf8()
        } else {
            end
        };
        let hyphen = if last && line.hyphen { "-" } else { "" };
        if tracked {
            // Text Tightness: each character at its measured place, its own
            // advance plus the tightness's share after it (VM fixture TT1);
            // each word starts where the layout put it.
            let (spans, _) = text_runs(start..shown_end, hyphen);
            let whole: P = shaped_text(&spans, base_size, height);
            let characters: Vec<(usize, char)> = shown[start..shown_end].char_indices().collect();
            let mut at = x;
            for (number, (offset, character)) in characters.iter().enumerate() {
                let from = start + offset;
                let to = from + character.len_utf8();
                let final_character = number + 1 == characters.len();
                let (mut spans, decorations) =
                    text_runs(from..to, if final_character { hyphen } else { "" });
                let overstruck = overstrike_glyphs(&spans, &decorations, base_size, height);
                let shifted = shifted_text(&mut spans, &decorations, base_size, height);
                segments.push(Segment {
                    bytes: from..if final_character { end } else { to },
                    x: at,
                    paragraph: shaped_text(&spans, base_size, height),
                    shown: to - from,
                    field: false,
                    number: false,
                    decorations,
                    shifted,
                    overstrikes: overstruck,
                });
                let natural = match (
                    whole.grapheme_position(0, number),
                    whole.grapheme_position(0, number + 1),
                ) {
                    (Some(here), Some(next)) => (next.x - here.x) / scale,
                    _ => 0.0,
                };
                at += natural + tracking(from);
            }
            if characters.is_empty() {
                segments.push(Segment {
                    bytes: start..end,
                    x,
                    paragraph: shaped_text(&[], base_size, height),
                    shown: 0,
                    field: false,
                    number: false,
                    decorations: Vec::new(),
                    shifted: Vec::new(),
                    overstrikes: Vec::new(),
                });
            }
            continue;
        }
        let (mut spans, decorations) = text_runs(start..shown_end, hyphen);
        let overstrikes = overstrike_glyphs(&spans, &decorations, base_size, height);
        let shifted = shifted_text(&mut spans, &decorations, base_size, height);
        let paragraph: P = shaped_text(&spans, base_size, height);
        // A number shown as stored (the caret is in its cell) stands where
        // the shown number stood: by the paragraph's alignment in its box.
        let x = if plain && word.text.is_some() && word.object.is_none() {
            let stored = paragraph.min_bounds().width / scale;
            match source.format.alignment.unwrap_or(style.alignment) {
                amipro_document::Alignment::Right => word.x + word.width - stored,
                amipro_document::Alignment::Centre => word.x + (word.width - stored) / 2.0,
                _ => word.x,
            }
        } else {
            x
        };
        segments.push(Segment {
            bytes: start..end,
            x,
            paragraph,
            shown: shown_end - start,
            field: false,
            number: false,
            decorations,
            shifted,
            overstrikes,
        });
    }
    // Spaces that end a line hang past its measure and have no word box,
    // but the caret and a selection stand after them, so typing a space
    // moves the caret (as in Ami Pro).
    let from = words.last().map_or(line.bytes.start, |word| word.bytes.end);
    let tail = &source.text[from.min(line.bytes.end)..line.bytes.end];
    if !tail.is_empty() && tail.bytes().all(|byte| byte == b' ') {
        let extends = !tracked
            && segments
                .last()
                .is_some_and(|segment: &Segment<P>| !segment.field && segment.bytes.end == from);
        let (start, x) = match segments.last() {
            Some(segment) if extends => (segment.bytes.start, segment.x),
            _ => (
                from,
                words.last().map_or(line.x, |word| word.x + word.width),
            ),
        };
        if extends {
            segments.pop();
        }
        let (mut spans, decorations) = text_runs(start..line.bytes.end, "");
        let overstrikes = overstrike_glyphs(&spans, &decorations, base_size, height);
        let shifted = shifted_text(&mut spans, &decorations, base_size, height);
        segments.push(Segment {
            bytes: start..line.bytes.end,
            x,
            paragraph: shaped_text(&spans, base_size, height),
            shown: line.bytes.end - start,
            field: false,
            number: false,
            decorations,
            shifted,
            overstrikes,
        });
    }
    // A bullet's or number's text, in the paragraph's font with the
    // bullets' attributes (bold, italic, underlines), and superscript as
    // superscript text is set: 7/12 of the size, raised by 5/12 of it
    // (VM fixture BL3).
    let bullet_text = line.bullet.as_ref().map_or_else(Vec::new, |bullet| {
        let font = display_font(
            &style.font_family,
            style.bold || bullet.attributes & 1 != 0,
            style.italic || bullet.attributes & 2 != 0,
        );
        let (size, rise) = if bullet.superscript {
            (
                base_size * 7.0 / 12.0,
                style.font_size_points * 20.0 * 5.0 / 12.0 * scale,
            )
        } else {
            (base_size, 0.0)
        };
        bullet
            .pieces
            .iter()
            .filter_map(|piece| match piece {
                BulletPiece::Text { text, x, .. } => {
                    let span = Span::new(text.clone())
                        .font(font)
                        .size(Pixels(size))
                        .color(colour(style.attributes.colour))
                        .underline(bullet.attributes & (4 | 8) != 0);
                    Some(BulletText {
                        x: bullet.x + x,
                        rise,
                        paragraph: shaped_text(&[span], base_size, height),
                    })
                }
                BulletPiece::Shape { .. } => None,
            })
            .collect()
    });
    // A Flow with text frame makes its line taller above the text, which
    // stands on the line's baseline beside it; COSMIC Text would centre the
    // text in the line (its baseline half the line down plus half the
    // font's ascent less its descent), so it is moved to the baseline.
    let framed = line.words.iter().any(|word| {
        matches!(
            word.object.and_then(|object| source.objects.get(object)),
            Some(InlineObject::FrameAnchor(frame))
                if document.frame(*frame).is_some_and(|frame| {
                    matches!(frame.placement, amipro_document::Placement::Inline { .. })
                })
        )
    });
    let text_shift = if framed {
        let family = display_family(&style.font_family);
        let ascent = ascent_em(family, style.bold, style.italic) * base_size;
        let descent = descent_em(family, style.bold, style.italic) * base_size;
        let centred = height / 2.0 + (ascent - descent) / 2.0;
        ((line.baseline - line.top) * scale - centred).max(0.0)
    } else {
        0.0
    };
    LineCache {
        source: Some(source),
        style: Some(style),
        word_x,
        segments,
        bullet: line.bullet.clone(),
        bullet_text,
        inline_bullets,
        text_shift,
    }
}

/// One mark of a drawn bullet on screen: its bounds, its corner radius, and
/// whether it is filled (else outlined).
pub(crate) type BulletMark = (Rectangle, (f32, bool));

/// The marks of a line's drawn bullets on screen (decompile `1200:0af0`),
/// in a box whose bottom is the baseline (`shape_marks`).
fn bullet_marks(line: &LineBox, scale: f32, left: f32, top: f32) -> Vec<BulletMark> {
    let Some(bullet) = &line.bullet else {
        return Vec::new();
    };
    let baseline = top + (line.baseline - line.top) * scale;
    let mut marks = Vec::new();
    for piece in &bullet.pieces {
        let BulletPiece::Shape {
            code,
            x,
            width,
            size,
        } = piece
        else {
            continue;
        };
        marks.extend(shape_marks(
            *code,
            left + (bullet.x + x) * scale,
            width * scale,
            // Ami Pro draws in the ascent less the internal leading, about
            // three quarters of the ascent the width came from.
            size * scale * 0.75,
            baseline,
        ));
    }
    marks
}

/// The marks of the bullets put into a line's text, with their colours.
fn inline_bullet_marks(
    line: &LineBox,
    bullets: &[InlineBullet],
    scale: f32,
    left: f32,
    top: f32,
) -> Vec<(BulletMark, Color)> {
    let baseline = top + (line.baseline - line.top) * scale;
    bullets
        .iter()
        .flat_map(|bullet| {
            shape_marks(
                bullet.code,
                left + bullet.x * scale,
                bullet.width * scale,
                bullet.size * scale * 0.75,
                baseline,
            )
            .into_iter()
            .map(|mark| (mark, bullet.colour))
        })
        .collect()
}

/// The marks of drawn bullet `code` on screen, in a box `size` pixels high
/// and `width` wide from `start`, whose bottom is the baseline: dots and
/// squares, filled or open, as rounded or plain quads, diamonds as rows of
/// pixels, a check mark as pixel strokes, Ami Pro's pictures as shapes of
/// the app's own (`picture_marks`). Each is centred on the middle of the
/// lower-case letters.
pub(crate) fn shape_marks(
    code: char,
    start: f32,
    width: f32,
    size: f32,
    baseline: f32,
) -> Vec<BulletMark> {
    let mut marks = Vec::new();
    // (diameter or side as a share of the size, round, filled)
    let (share, round, filled) = match code {
        '\u{10}' => (0.25, true, true),
        '\u{11}' => (0.5, true, true),
        '\u{12}' => (1.0 / 3.0, false, true),
        '\u{13}' => (0.5, false, true),
        '\u{14}' => (0.75, false, false),
        '\u{15}' | '\u{16}' => {
            // A diamond, as rows narrowing from its middle.
            let half = (size * if code == '\u{15}' { 0.25 } else { 0.375 }).round();
            let centre_x = start + width / 2.0;
            let centre_y = (baseline - size * 0.45).round();
            for row in -half as i32..=half as i32 {
                let reach = half - (row as f32).abs();
                marks.push((
                    Rectangle::new(
                        Point::new((centre_x - reach).round(), centre_y + row as f32),
                        Size::new((2.0 * reach).max(1.0), 1.0),
                    ),
                    (0.0, true),
                ));
            }
            return marks;
        }
        '\u{17}' => (0.5, true, false),
        '\u{18}' => (0.75, true, false),
        '\u{19}' => {
            // A check mark: down from the middle of its left side to a
            // third of the way along, then up to its top right.
            let thickness = (size / 8.0).round().max(1.0);
            stroke(
                &mut marks,
                &[
                    (start, baseline - size * 0.45),
                    (start + width / 3.0, baseline),
                    (start + width, baseline - size),
                ],
                thickness,
            );
            return marks;
        }
        '\u{8}' | '\u{A}'..='\u{F}' => {
            picture_marks(&mut marks, code, start, width, size, baseline);
            return marks;
        }
        _ => return marks,
    };
    let side = (size * share).round().max(2.0);
    let centre_x = start + width / 2.0;
    let centre_y = baseline - size * 0.45;
    marks.push((
        Rectangle::new(
            Point::new(
                (centre_x - side / 2.0).round(),
                (centre_y - side / 2.0).round(),
            ),
            Size::new(side, side),
        ),
        (if round { side / 2.0 } else { 0.0 }, filled),
    ));
    marks
}

/// Square dots along the lines from point to point, `thickness` wide.
fn stroke(marks: &mut Vec<BulletMark>, points: &[(f32, f32)], thickness: f32) {
    for pair in points.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil().max(1.0) as i32;
        for step in 0..=steps {
            let t = step as f32 / steps as f32;
            marks.push((
                Rectangle::new(
                    Point::new(
                        (x0 + (x1 - x0) * t - thickness / 2.0).round(),
                        (y0 + (y1 - y0) * t - thickness / 2.0).round(),
                    ),
                    Size::new(thickness, thickness),
                ),
                (0.0, true),
            ));
        }
    }
}

/// Ami Pro's seven picture bullets, drawn as shapes of the app's own that
/// read as its pictures (VM fixture BL1; Ami Pro's metafiles are not
/// used): in a square `size` pixels high centred in `width`, its bottom on
/// the baseline.
fn picture_marks(
    marks: &mut Vec<BulletMark>,
    code: char,
    start: f32,
    width: f32,
    size: f32,
    baseline: f32,
) {
    let side = size.round().max(4.0);
    let left = (start + (width - side) / 2.0).round();
    let top = (baseline - side).round();
    let line = (side / 10.0).round().max(1.0);
    let filled = |marks: &mut Vec<BulletMark>, x: f32, y: f32, w: f32, h: f32| {
        marks.push((
            Rectangle::new(
                Point::new(x.round(), y.round()),
                Size::new(w.round().max(1.0), h.round().max(1.0)),
            ),
            (0.0, true),
        ));
    };
    let outline = |marks: &mut Vec<BulletMark>, x: f32, y: f32, w: f32, h: f32| {
        filled(marks, x, y, w, line);
        filled(marks, x, y + h - line, w, line);
        filled(marks, x, y, line, h);
        filled(marks, x + w - line, y, line, h);
    };
    match code {
        // A pointing hand: a cuff, a fist, and a finger to the right.
        '\u{8}' => {
            let (y, h) = (top + side * 0.3, side * 0.55);
            filled(marks, left, y, side * 0.12, h);
            marks.push((
                Rectangle::new(
                    Point::new((left + side * 0.16).round(), y.round()),
                    Size::new((side * 0.42).round(), h.round()),
                ),
                ((side * 0.1).round(), true),
            ));
            filled(
                marks,
                left + side * 0.5,
                y,
                side * 0.5,
                (side * 0.16).max(line),
            );
        }
        // A box with a shadow down its right and bottom sides.
        '\u{A}' => {
            let box_side = side - 2.0 * line;
            outline(marks, left, top, box_side, box_side);
            filled(
                marks,
                left + box_side,
                top + 2.0 * line,
                2.0 * line,
                box_side,
            );
            filled(
                marks,
                left + 2.0 * line,
                top + box_side,
                box_side,
                2.0 * line,
            );
        }
        // A box with a thick top.
        '\u{B}' => {
            outline(marks, left, top, side, side);
            filled(marks, left, top, side, (side * 0.25).max(2.0 * line));
        }
        // A box with a check mark in it.
        '\u{C}' => {
            outline(marks, left, top, side, side);
            stroke(
                marks,
                &[
                    (left + side * 0.22, top + side * 0.52),
                    (left + side * 0.42, top + side * 0.75),
                    (left + side * 0.8, top + side * 0.22),
                ],
                line,
            );
        }
        // A box with a cross in it.
        '\u{D}' => {
            outline(marks, left, top, side, side);
            let (near, far) = (side * 0.22, side * 0.78);
            stroke(
                marks,
                &[(left + near, top + near), (left + far, top + far)],
                line,
            );
            stroke(
                marks,
                &[(left + far, top + near), (left + near, top + far)],
                line,
            );
        }
        // An arrowhead pointing right, notched at the back: one half filled
        // and the other outlined, lit from above (0x0E) or below (0x0F).
        _ => {
            let middle = top + side / 2.0;
            let tip = (left + side, middle);
            let notch = (left + side * 0.3, middle);
            let rows = (side / 2.0).round() as i32;
            let lower_filled = code == '\u{E}';
            for row in 0..rows {
                let depth = row as f32 / rows as f32;
                let from = notch.0 - (notch.0 - left) * depth;
                let to = tip.0 - (tip.0 - left) * depth;
                let y = if lower_filled {
                    middle + row as f32
                } else {
                    middle - 1.0 - row as f32
                };
                filled(marks, from, y, to - from, 1.0);
            }
            let corner = if lower_filled {
                (left, top)
            } else {
                (left, top + side)
            };
            stroke(marks, &[corner, tip], line);
            stroke(marks, &[corner, notch], line);
        }
    }
}

/// Fills one bullet mark (`shape_marks`) in `colour`.
pub(crate) fn fill_bullet_mark<Renderer: renderer::Renderer>(
    renderer: &mut Renderer,
    (bounds, (radius, filled)): BulletMark,
    colour: Color,
) {
    renderer.fill_quad(
        renderer::Quad {
            bounds,
            border: Border {
                color: colour,
                width: if filled { 0.0 } else { 1.0 },
                radius: radius.into(),
            },
            ..renderer::Quad::default()
        },
        if filled { colour } else { Color::TRANSPARENT },
    );
}

fn line_key(page: &PageBox, line: &LineBox, scale: u32, editing: Option<FlowId>) -> LineKey {
    let has_field = line.words.iter().any(|word| word.text.is_some());
    LineKey {
        flow: line.flow,
        paragraph: line.paragraph,
        bytes: line.bytes.clone(),
        scale,
        hyphen: line.hyphen,
        field_page: if has_field { page.number } else { 0 },
        field_texts: line
            .words
            .iter()
            .filter_map(|word| word.text.as_deref())
            .collect::<Vec<_>>()
            .join("\u{1F}"),
        plain: editing == Some(line.flow)
            && line
                .words
                .iter()
                .any(|word| word.text.is_some() && word.object.is_none()),
    }
}

impl<P: text::Paragraph<Font = Font>> State<P> {
    /// Brings the caches up to date with `layout`: pages the layout reused
    /// keep their lines; other pages look their lines up by paragraph value.
    fn sync(
        &mut self,
        document: &Document,
        layout: &DocumentLayout,
        scale: f32,
        session: u64,
        hide_protected: bool,
        editing: Option<FlowId>,
    ) {
        let scale_bits = scale.to_bits();
        if self.editing != editing {
            self.editing = editing;
            self.pages.clear();
        }
        let fonts = font_system().read().ok().map(|fonts| fonts.version());
        if self.scale != scale_bits
            || self.session != session
            || self.fonts != fonts
            || self.hide_protected != hide_protected
        {
            self.lines.clear();
            self.pages.clear();
            self.numbers.clear();
            self.scale = scale_bits;
            self.session = session;
            self.fonts = fonts;
            self.hide_protected = hide_protected;
        }
        self.pages.truncate(layout.pages.len());
        for (index, page) in layout.pages.iter().enumerate() {
            if self
                .pages
                .get(index)
                .is_some_and(|cached| Arc::ptr_eq(&cached.page, page))
            {
                continue;
            }
            let mut lines = Vec::with_capacity(page.lines.len());
            for line in &page.lines {
                let key = line_key(page, line, scale_bits, editing);
                let current = source_of(document, line);
                let reusable =
                    self.lines
                        .get(&key)
                        .filter(|cached| match (&cached.source, &current) {
                            (Some(cached_source), Some(current)) => {
                                (Arc::ptr_eq(cached_source, current)
                                    || **cached_source == **current)
                                    && cached.style.as_ref()
                                        == document.styles().get(&current.style)
                                    && cached
                                        .word_x
                                        .iter()
                                        .copied()
                                        .eq(line.words.iter().map(|word| word.x))
                                    && cached.bullet == line.bullet
                            }
                            (None, None) => true,
                            _ => false,
                        });
                let cached = match reusable {
                    Some(cached) => Rc::clone(cached),
                    None => {
                        self.built += 1;
                        let built = Rc::new(build_line(
                            document,
                            line,
                            scale,
                            self.hide_protected,
                            key.plain,
                        ));
                        self.lines.insert(key, Rc::clone(&built));
                        built
                    }
                };
                lines.push(cached);
            }
            let cache = PageCache {
                page: Arc::clone(page),
                lines,
            };
            if index < self.pages.len() {
                self.pages[index] = cache;
            } else {
                self.pages.push(cache);
            }
        }
        self.lines.retain(|_, line| Rc::strong_count(line) > 1);
        self.numbers.truncate(layout.line_numbers.len());
        for (index, boxes) in layout.line_numbers.iter().enumerate() {
            if self
                .numbers
                .get(index)
                .is_some_and(|cached| cached.boxes == *boxes)
            {
                continue;
            }
            let cache = NumberCache {
                boxes: boxes.clone(),
                texts: boxes
                    .iter()
                    .filter_map(|number| line_number_text(document, number, scale))
                    .collect(),
            };
            if index < self.numbers.len() {
                self.numbers[index] = cache;
            } else {
                self.numbers.push(cache);
            }
        }
    }
}

/// The quads of an Outline mode icon `size` pixels across with its
/// top-left corner at `corner`: a plus, a heavier plus when contracted, a
/// minus, or a small open box for body text.
fn outline_icon_quads(
    icon: crate::outline_view::OutlineIcon,
    corner: Point,
    size: f32,
) -> Vec<(Rectangle, Color)> {
    use crate::outline_view::OutlineIcon;
    let size = size.round().max(5.0);
    let (x, y) = (corner.x.round(), corner.y.round());
    let grey = Color::from_rgb(0.35, 0.35, 0.35);
    let bar = |thickness: f32| -> (Rectangle, Rectangle) {
        let middle = ((size - thickness) / 2.0).round();
        (
            Rectangle::new(Point::new(x, y + middle), Size::new(size, thickness)),
            Rectangle::new(Point::new(x + middle, y), Size::new(thickness, size)),
        )
    };
    match icon {
        OutlineIcon::Plus | OutlineIcon::Contracted => {
            let thickness = if icon == OutlineIcon::Plus {
                (size / 5.0).round().max(1.0)
            } else {
                (size / 3.0).round().max(2.0)
            };
            let colour = if icon == OutlineIcon::Plus {
                grey
            } else {
                Color::BLACK
            };
            let (across, down) = bar(thickness);
            vec![(across, colour), (down, colour)]
        }
        OutlineIcon::Minus => vec![(bar((size / 5.0).round().max(1.0)).0, grey)],
        OutlineIcon::Body => {
            let side = (size * 0.6).round().max(3.0);
            let inset = ((size - side) / 2.0).round();
            let (left, top) = (x + inset, y + inset);
            vec![
                (
                    Rectangle::new(Point::new(left, top), Size::new(side, 1.0)),
                    grey,
                ),
                (
                    Rectangle::new(Point::new(left, top + side - 1.0), Size::new(side, 1.0)),
                    grey,
                ),
                (
                    Rectangle::new(Point::new(left, top), Size::new(1.0, side)),
                    grey,
                ),
                (
                    Rectangle::new(Point::new(left + side - 1.0, top), Size::new(1.0, side)),
                    grey,
                ),
            ]
        }
    }
}

/// A line number shaped in its style's font, with its top-left corner in
/// twips: the text's line is as tall as the font, so its baseline is the
/// font's ascent below the top.
fn line_number_text<P: text::Paragraph<Font = Font>>(
    document: &Document,
    number: &amipro_layout::LineNumberBox,
    scale: f32,
) -> Option<(f32, f32, P)> {
    let style = document.styles().get(&number.style)?;
    let family = display_family(&style.font_family);
    let size = style.font_size_points * 20.0 * scale;
    let ascent = ascent_em(family, style.bold, style.italic) * size;
    let descent = descent_em(family, style.bold, style.italic) * size;
    let span = Span::new(number.text.clone())
        .font(display_font(&style.font_family, style.bold, style.italic))
        .size(Pixels(size.max(1.0)))
        .color(colour(style.attributes.colour));
    let paragraph = shaped_text(&[span], size, ascent + descent);
    Some((number.x, number.baseline - ascent / scale, paragraph))
}

/// The byte of `line` nearest to `x` (twips from the page's left edge).
fn line_hit<P: text::Paragraph>(line: &LineBox, cache: &LineCache<P>, x: f32, scale: f32) -> usize {
    let mut nearest = (f32::INFINITY, line.bytes.start);
    // A bullet put into the text is one character: its nearer side.
    for bullet in &cache.inline_bullets {
        if x >= bullet.x && x <= bullet.x + bullet.width {
            return if x < bullet.x + bullet.width / 2.0 {
                bullet.bytes.start
            } else {
                bullet.bytes.end
            };
        }
        for (edge, byte) in [
            (bullet.x, bullet.bytes.start),
            (bullet.x + bullet.width, bullet.bytes.end),
        ] {
            let distance = (x - edge).abs();
            if distance < nearest.0 {
                nearest = (distance, byte);
            }
        }
    }
    for segment in &cache.segments {
        let width = segment.paragraph.min_bounds().width / scale;
        if x >= segment.x && x <= segment.x + width {
            if segment.field {
                return if x < segment.x + width / 2.0 && !segment.number {
                    segment.bytes.start
                } else {
                    segment.bytes.end
                };
            }
            let local = Point::new((x - segment.x) * scale, line.height * scale / 2.0);
            let offset = segment
                .paragraph
                .hit_test(local)
                .map_or(0, text::Hit::cursor)
                .min(segment.shown);
            return segment.bytes.start + offset;
        }
        for (edge, byte) in [
            (segment.x, segment.bytes.start),
            (
                segment.x + width,
                segment.bytes.start
                    + segment
                        .shown
                        .max(usize::from(segment.field) * segment.bytes.len()),
            ),
        ] {
            let distance = (x - edge).abs();
            if distance < nearest.0 {
                nearest = (distance, byte);
            }
        }
    }
    if cache.segments.is_empty() && cache.inline_bullets.is_empty() {
        return if x <= line.x || line.words.is_empty() {
            line.bytes.start
        } else {
            line.bytes.end
        };
    }
    nearest.1
}

/// The x (twips from the page's left edge) of `byte` on `line`.
fn line_x<P: text::Paragraph>(
    line: &LineBox,
    cache: &LineCache<P>,
    byte: usize,
    scale: f32,
) -> f32 {
    let mut x = line.x;
    // The start of the last segment the caret is at or after: a bullet put
    // into the text after it places the caret instead.
    let mut after = line.bytes.start;
    for segment in &cache.segments {
        if byte < segment.bytes.start {
            break;
        }
        after = segment.bytes.start;
        let width = segment.paragraph.min_bounds().width / scale;
        if segment.field {
            x = if byte == segment.bytes.start {
                segment.x
            } else {
                segment.x + width
            };
            continue;
        }
        let offset = (byte - segment.bytes.start).min(segment.shown);
        x = segment.x
            + segment
                .paragraph
                .cursor_position(0, offset, Affinity::Before)
                .map_or(0.0, |point| point.x)
                / scale;
    }
    for bullet in &cache.inline_bullets {
        if bullet.bytes.start < after || byte < bullet.bytes.start {
            continue;
        }
        x = if byte >= bullet.bytes.end {
            bullet.x + bullet.width
        } else {
            bullet.x
        };
    }
    x
}

/// Lines a selection or caret in `flow` may use on `page`: every line of the
/// main flow and of frames on one page, and for flows shown on many pages
/// (headers, footers, repeating frames) only the selected page instance.
fn shows_flow(
    line: &LineBox,
    flow: FlowId,
    page: &PageBox,
    selection_page: u32,
    main: FlowId,
) -> bool {
    line.flow == flow && (flow == main || page.number == selection_page || !repeats(line))
}

/// Lines shown on several pages: header, footer, and frame text. A table
/// cell's lines are drawn wherever its table shows them: a row continued on
/// the next page has its own lines there, and a repeated heading shows the
/// selection in each copy.
fn repeats(line: &LineBox) -> bool {
    !matches!(line.container, Container::Main | Container::Cell { .. })
}

/// Fills the text of `bytes` on `line` with `colour`, segment by segment:
/// the selection, and the find bar's matches under it.
#[allow(clippy::too_many_arguments)]
fn fill_bytes<Renderer: text::Renderer<Font = Font>>(
    renderer: &mut Renderer,
    cache: &LineCache<Renderer::Paragraph>,
    view: &PageView,
    page_index: usize,
    line: &LineBox,
    origin: Vector,
    bytes: std::ops::Range<usize>,
    colour: Color,
    whole_fields: bool,
) {
    for segment in &cache.segments {
        let segment_origin = view.point(page_index, segment.x, line.top) + origin;
        // A field's text (a page number, a date) is filled whole when its
        // object is in the range: Field Shading.
        if segment.field {
            if whole_fields && bytes.start <= segment.bytes.start && segment.bytes.end <= bytes.end
            {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(segment_origin, segment.paragraph.min_bounds()),
                        ..renderer::Quad::default()
                    },
                    colour,
                );
            }
            continue;
        }
        let a = bytes.start.max(segment.bytes.start);
        let b = bytes.end.min(segment.bytes.start + segment.shown);
        if a >= b {
            continue;
        }
        for rectangle in segment.paragraph.highlight(
            0,
            (a - segment.bytes.start, Affinity::Before),
            (b - segment.bytes.start, Affinity::Before),
        ) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: rectangle + (segment_origin - Point::ORIGIN),
                    ..renderer::Quad::default()
                },
                colour,
            );
        }
    }
}

/// A wavy line under the bytes `bytes` of a line: spelling's and grammar's
/// underline. Small steps up and down along the text's bottom, 2 px apart.
#[allow(clippy::too_many_arguments)]
fn wavy_bytes<Renderer: text::Renderer<Font = Font>>(
    renderer: &mut Renderer,
    cache: &LineCache<Renderer::Paragraph>,
    view: &PageView,
    page_index: usize,
    line: &LineBox,
    origin: Vector,
    bytes: std::ops::Range<usize>,
    colour: Color,
) {
    for segment in &cache.segments {
        if segment.field {
            continue;
        }
        let a = bytes.start.max(segment.bytes.start);
        let b = bytes.end.min(segment.bytes.start + segment.shown);
        if a >= b {
            continue;
        }
        let segment_origin = view.point(page_index, segment.x, line.top) + origin;
        for rectangle in segment.paragraph.highlight(
            0,
            (a - segment.bytes.start, Affinity::Before),
            (b - segment.bytes.start, Affinity::Before),
        ) {
            let bounds = rectangle + (segment_origin - Point::ORIGIN);
            let step: f32 = 2.0;
            let bottom = bounds.y + bounds.height - 1.5;
            let mut x = bounds.x;
            let mut up = false;
            while x < bounds.x + bounds.width {
                let width = step.min(bounds.x + bounds.width - x);
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(x, if up { bottom - 1.5 } else { bottom }),
                            Size::new(width, 1.5),
                        ),
                        ..renderer::Quad::default()
                    },
                    colour,
                );
                up = !up;
                x += step;
            }
        }
    }
}

/// A rectangle's four edges, one pixel wide: top, bottom, left, right.
fn cell_edges(bounds: Rectangle) -> [Rectangle; 4] {
    [
        Rectangle::new(bounds.position(), Size::new(bounds.width, 1.0)),
        Rectangle::new(
            Point::new(bounds.x, bounds.y + bounds.height - 1.0),
            Size::new(bounds.width, 1.0),
        ),
        Rectangle::new(bounds.position(), Size::new(1.0, bounds.height)),
        Rectangle::new(
            Point::new(bounds.x + bounds.width - 1.0, bounds.y),
            Size::new(1.0, bounds.height),
        ),
    ]
}

/// Whether `bounds` holds the page point `x`, `y`.
fn contains(bounds: amipro_layout::Bounds, x: f32, y: f32) -> bool {
    x >= bounds.x && x <= bounds.right() && y >= bounds.y && y <= bounds.bottom()
}

/// The smallest bounds around all of `boxes`.
fn union(boxes: impl Iterator<Item = amipro_layout::Bounds>) -> Option<amipro_layout::Bounds> {
    boxes.reduce(|a, b| {
        let (x, y) = (a.x.min(b.x), a.y.min(b.y));
        amipro_layout::Bounds {
            x,
            y,
            width: a.right().max(b.right()) - x,
            height: a.bottom().max(b.bottom()) - y,
        }
    })
}

fn bounds_of(rect: amipro_document::Rect) -> amipro_layout::Bounds {
    amipro_layout::Bounds {
        x: rect.left as f32,
        y: rect.top as f32,
        width: rect.width() as f32,
        height: rect.height() as f32,
    }
}

/// The pointer over a handle, by the edges it moves.
fn resize_cursor(handle: Handle) -> mouse::Interaction {
    match handle {
        Handle::Left | Handle::Right => mouse::Interaction::ResizingHorizontally,
        Handle::Top | Handle::Bottom => mouse::Interaction::ResizingVertically,
        Handle::TopLeft | Handle::BottomRight => mouse::Interaction::ResizingDiagonallyDown,
        Handle::TopRight | Handle::BottomLeft => mouse::Interaction::ResizingDiagonallyUp,
    }
}

/// What a key does in picture mode (`UI_SPEC.md` §25, P6; Ami Pro's guide,
/// ch. 19): arrows move the picture 0.05 in, with Ctrl 0.5 in; Esc leaves.
fn picture_key_action(key: &Key, modifiers: keyboard::Modifiers) -> Option<FrameAction> {
    let step = if modifiers.control() {
        frame_editing::PICTURE_LARGE_STEP
    } else {
        frame_editing::PICTURE_STEP
    };
    let (dx, dy) = match key {
        Key::Named(Named::ArrowLeft) => (-step, 0),
        Key::Named(Named::ArrowRight) => (step, 0),
        Key::Named(Named::ArrowUp) => (0, -step),
        Key::Named(Named::ArrowDown) => (0, step),
        Key::Named(Named::Escape) => return Some(FrameAction::LeavePicture),
        _ => return None,
    };
    Some(FrameAction::Pan { dx, dy, key: true })
}

/// What a key does to selected frames (`UI_SPEC.md` §11, "Keyboard access").
fn frame_key_action(key: &Key, modifiers: keyboard::Modifiers) -> Option<FrameAction> {
    let step = if modifiers.shift() {
        frame_editing::LARGE_STEP
    } else {
        frame_editing::STEP
    };
    let arrow = match key {
        Key::Named(Named::ArrowLeft) => Some((-step, 0)),
        Key::Named(Named::ArrowRight) => Some((step, 0)),
        Key::Named(Named::ArrowUp) => Some((0, -step)),
        Key::Named(Named::ArrowDown) => Some((0, step)),
        _ => None,
    };
    if let Some((dx, dy)) = arrow {
        return Some(if modifiers.alt() {
            FrameAction::ResizeBy { dx, dy }
        } else {
            FrameAction::Move { dx, dy, key: true }
        });
    }
    match key {
        Key::Named(Named::Tab) => Some(FrameAction::Cycle {
            backwards: modifiers.shift(),
        }),
        Key::Named(Named::Enter) => Some(FrameAction::EnterText { at: None }),
        Key::Named(Named::Delete | Named::Backspace) => Some(FrameAction::Delete),
        Key::Named(Named::Escape) => Some(FrameAction::Deselect),
        _ => None,
    }
}

impl<'a, Message> PageSurface<'a, Message> {
    /// The handles drawn on the selection: a single frame's resize handles,
    /// or all eight around a group or several frames (which do not resize).
    fn shown_handles(&self) -> Vec<Handle> {
        let Some(selection) = self.frames else {
            return Vec::new();
        };
        match selection
            .single(self.document)
            .and_then(|frame| self.document.frame(frame))
        {
            Some(frame) => frame_editing::resize_handles(frame.placement),
            None => Handle::ALL.to_vec(),
        }
    }

    /// The selected frame's resize handle under `point` (workspace pixels).
    fn handle_at<P>(&self, state: &State<P>, page_index: usize, point: Point) -> Option<Handle> {
        let selection = self.frames?;
        let frame = selection.single(self.document)?;
        let page = &state.pages.get(page_index)?.page;
        if page.number != selection.page {
            return None;
        }
        let frame_box = page
            .frames
            .iter()
            .find(|frame_box| frame_box.frame == frame)?;
        let placement = self.document.frame(frame)?.placement;
        frame_editing::resize_handles(placement)
            .into_iter()
            .find(|handle| {
                let (x, y) = handle.centre(frame_box.bounds);
                let centre = self.view.point(page_index, x, y);
                (point.x - centre.x).abs() <= HANDLE_TARGET / 2.0
                    && (point.y - centre.y).abs() <= HANDLE_TARGET / 2.0
            })
    }

    /// Whether `point` (workspace pixels) is inside a selected frame.
    fn in_selection<P>(&self, state: &State<P>, page_index: usize, point: Point) -> bool {
        let (Some(selection), Some(cached)) = (self.frames, state.pages.get(page_index)) else {
            return false;
        };
        if cached.page.number != selection.page {
            return false;
        }
        let members = selection.members(self.document);
        let (x, y) = self.view.to_page(page_index, point);
        cached
            .page
            .frames
            .iter()
            .any(|frame_box| members.contains(&frame_box.frame) && contains(frame_box.bounds, x, y))
    }

    /// A resize drag's rectangles: the frame as drawn and as dragged.
    fn resize_preview<P>(
        &self,
        state: &State<P>,
        drag: FrameDrag,
        handle: Handle,
    ) -> Option<(amipro_document::Rect, amipro_document::Rect)> {
        let frame = self.frames?.single(self.document)?;
        let frame_box = state
            .pages
            .get(drag.page_index)?
            .page
            .frames
            .iter()
            .find(|frame_box| frame_box.frame == frame)?;
        let from = frame_editing::rect_of(frame_box.bounds);
        let (dx, dy) = drag.delta();
        let to = frame_editing::resized(
            from,
            handle,
            dx,
            dy,
            state.modifiers.shift(),
            frame_editing::minimum_size(self.document, frame),
        );
        Some((from, to))
    }

    /// Whether the selection is one frame holding a picture.
    fn single_picture_selected(&self) -> bool {
        self.frames
            .and_then(|selection| selection.single(self.document))
            .and_then(|frame| self.document.frame(frame))
            .is_some_and(|frame| matches!(frame.content, FrameContent::Picture(_)))
    }

    /// A left press: selects, starts moving or resizing frames, or enters a
    /// frame's text. Returns false when the press is for text.
    fn press_frame<P: text::Paragraph>(
        &self,
        state: &mut State<P>,
        position: Point,
        click: advanced::mouse::Click,
        shell: &mut Shell<'_, Message>,
    ) -> bool {
        let Some(on_frame) = &self.on_frame else {
            return false;
        };
        if state.pages.is_empty() {
            return false;
        }
        let page_index = self.view.page_at(position).min(state.pages.len() - 1);
        let page = Arc::clone(&state.pages[page_index].page);
        let (x, y) = self.view.to_page(page_index, position);
        let shift = state.modifiers.shift();
        let drag = |kind, on_selected| FrameDrag {
            kind,
            page_index,
            start: (x, y),
            current: (x, y),
            press: position,
            moved: false,
            on_selected,
        };
        // Picture mode: a press in the frame moves its picture.
        if self.picture_mode && self.in_selection(state, page_index, position) {
            state.frame_drag = Some(drag(DragKind::Pan, true));
            return true;
        }
        // Shift and a handle size a picture with its frame (P6); other
        // Shift presses add to the selection.
        if (!shift || self.single_picture_selected())
            && let Some(handle) = self.handle_at(state, page_index, position)
        {
            state.frame_drag = Some(drag(DragKind::Resize(handle), true));
            return true;
        }
        if !shift && self.in_selection(state, page_index, position) {
            if click.kind() == advanced::mouse::click::Kind::Double {
                shell.publish(on_frame(FrameAction::EnterText {
                    at: self.hit(state, position),
                }));
            } else {
                state.frame_drag = Some(drag(DragKind::Move, true));
            }
            return true;
        }
        let Some(frame) = frame_editing::frame_at(self.document, &page, x, y) else {
            return false;
        };
        // A press in a table's cells puts the caret in the cell, as in Ami
        // Pro (VM TE1–TE19); a table in the text is never selected as a
        // frame by a click, a frame table only on its edge (TK6).
        if self.in_table_cells(&page, frame, x, y) {
            return false;
        }
        shell.publish(on_frame(FrameAction::Select {
            frame,
            page: page.number,
            extend: shift,
        }));
        if !shift {
            // Ami Pro: a drag from inside an unselected frame moves it.
            state.frame_drag = Some(drag(DragKind::Move, false));
        }
        true
    }

    /// The table grid line under `position` (`1498:25fa`): in a column's
    /// gutter (or a row's, only without Automatic), not inside a joined cell
    /// it crosses, and not with Disable mouse col/row sizing.
    fn track_line_at<P: text::Paragraph>(
        &self,
        state: &State<P>,
        position: Point,
    ) -> Option<TrackDrag> {
        if state.pages.is_empty() || self.printing.is_some() {
            return None;
        }
        let page_index = self.view.page_at(position).min(state.pages.len() - 1);
        let page = &state.pages[page_index].page;
        let (x, y) = self.view.to_page(page_index, position);
        let slack = 2.0 / self.view.scale;
        for frame_box in &page.frames {
            let FrameBoxKind::Table { cells, part } = &frame_box.kind else {
                continue;
            };
            let Some(FrameContent::Table(table)) = self
                .document
                .frame(frame_box.frame)
                .map(|frame| &frame.content)
            else {
                continue;
            };
            if table.layout.fixed_sizes {
                continue;
            }
            let bounds = frame_box.bounds;
            if !(bounds.x - slack..=bounds.x + bounds.width + slack).contains(&x)
                || !(bounds.y - slack..=bounds.y + bounds.height + slack).contains(&y)
            {
                continue;
            }
            let count = table.columns.len().max(1);
            // The row and column the point is in, from the cells' boxes.
            let row_at = part.rows.clone().find(|row| {
                cells.get(row * count).is_some_and(|cell| {
                    cell.width > 0.0 && y <= cell.y + cell.height + table.row_gutter(*row) as f32
                })
            });
            // Room the table has to grow (the frame's inside, or the
            // column it stands in).
            let room_right = match self
                .document
                .frame(frame_box.frame)
                .map(|frame| frame.placement)
            {
                Some(amipro_document::Placement::Fixed { .. }) => frame_box.interior.right(),
                _ => page
                    .columns
                    .iter()
                    .find(|column| bounds.x >= column.x - 1.0 && bounds.x < column.right())
                    .map_or(page.width, |column| column.right()),
            };
            let mut left = frame_box.interior.x + table.spacing.left as f32;
            let last_right = left
                + (0..count)
                    .map(|column| (table.columns[column] + table.column_gutter(column)) as f32)
                    .sum::<f32>()
                - table.column_gutter(count - 1) as f32;
            for column in 0..count {
                let right = left + table.columns[column] as f32;
                let gutter = table.column_gutter(column) as f32;
                if x >= right - slack && x <= right + gutter.max(slack) {
                    // In a joined cell only its last column's line.
                    let inside = row_at.is_some_and(|row| {
                        let (_, block_left, span) =
                            amipro_editing::tables::block_at(table, row, column);
                        block_left + usize::from(span.columns.max(1)) - 1 != column
                    });
                    if inside {
                        return None;
                    }
                    let room = (room_right - last_right).max(0.0);
                    return Some(TrackDrag {
                        frame: frame_box.frame,
                        page_index,
                        column: true,
                        index: column,
                        origin: left,
                        at: right,
                        limits: (left + 1.0, right + room),
                        start: right,
                        press: None,
                    });
                }
                left = right + gutter;
            }
            if !table.row_options.automatic {
                for row in part.rows.clone() {
                    let Some(cell) = cells.get(row * count).filter(|cell| cell.width > 0.0) else {
                        continue;
                    };
                    let bottom = cell.y + table.rows[row] as f32;
                    let gutter = table.row_gutter(row) as f32;
                    if y >= bottom - slack && y <= bottom + gutter.max(slack) {
                        return Some(TrackDrag {
                            frame: frame_box.frame,
                            page_index,
                            column: false,
                            index: row,
                            origin: cell.y,
                            at: bottom,
                            limits: (cell.y + 10.0, page.height),
                            start: bottom,
                            press: None,
                        });
                    }
                }
            }
        }
        None
    }

    /// Whether a press at (`x`, `y`) on `page` over `frame` goes to a
    /// table's cell rather than selecting the frame (TK6): any press on a
    /// table in the text, and a press inside a frame table's cells.
    fn in_table_cells(
        &self,
        page: &PageBox,
        frame: amipro_document::FrameId,
        x: f32,
        y: f32,
    ) -> bool {
        self.document.frame(frame).is_some_and(|holder| {
            matches!(holder.content, FrameContent::Table(_))
                && (holder.placement.is_anchored()
                    || page.frames.iter().any(|frame_box| {
                        frame_box.frame == frame
                            && matches!(&frame_box.kind, FrameBoxKind::Table { cells, .. }
                            if cells.iter().any(|cell| {
                                cell.width > 0.0
                                    && (cell.x..=cell.x + cell.width).contains(&x)
                                    && (cell.y..=cell.y + cell.height).contains(&y)
                            }))
                    }))
        })
    }

    /// Whether `position` is in a page's left or right margin, outside its
    /// columns and any frame.
    /// A press on the strip above a table's column or left of its row
    /// selects that column or row (`1498:151b`, `1498:16da`): the table's
    /// inset, and a few pixels outside its edge.
    fn track_strip<P: text::Paragraph>(
        &self,
        state: &State<P>,
        position: Point,
    ) -> Option<TrackStrip> {
        if state.pages.is_empty() {
            return None;
        }
        let page_index = self.view.page_at(position).min(state.pages.len() - 1);
        let page = &state.pages[page_index].page;
        let (x, y) = self.view.to_page(page_index, position);
        let slack = 4.0 / self.view.scale;
        for frame_box in &page.frames {
            let FrameBoxKind::Table { cells, part } = &frame_box.kind else {
                continue;
            };
            let Some(FrameContent::Table(table)) = self
                .document
                .frame(frame_box.frame)
                .map(|frame| &frame.content)
            else {
                continue;
            };
            let columns = table.columns.len().max(1);
            let rows = table.rows.len();
            let first_row = part.rows.start.min(rows.saturating_sub(1));
            let edge = frame_box.bounds;
            // Above a column: from a little over the table's top to its
            // first row's text.
            let top_text = cells
                .iter()
                .skip(first_row * columns)
                .take(columns)
                .filter(|cell| cell.width > 0.0)
                .map(|cell| cell.y)
                .fold(f32::MAX, f32::min);
            if part.rows.start == 0
                && (edge.y - slack..top_text.max(edge.y + slack)).contains(&y)
                && (edge.x..edge.x + edge.width).contains(&x)
                && rows >= 2
            {
                let column = (0..columns).find(|column| {
                    cells.get(first_row * columns + column).is_some_and(|cell| {
                        cell.width > 0.0
                            && x < cell.x + cell.width + table.column_gutter(*column) as f32
                    })
                })?;
                return Some((frame_box.frame, (0, column), (rows - 1, column)));
            }
            // Left of a row: from a little left of the table to its first
            // column's text.
            let left_text = cells
                .iter()
                .step_by(columns)
                .filter(|cell| cell.width > 0.0)
                .map(|cell| cell.x)
                .fold(f32::MAX, f32::min);
            if (edge.x - slack..left_text.max(edge.x + slack)).contains(&x)
                && (edge.y..edge.y + edge.height).contains(&y)
                && columns >= 2
            {
                let row = part.rows.clone().find(|row| {
                    cells.get(row * columns).is_some_and(|cell| {
                        cell.width > 0.0 && y < cell.y + cell.height + table.row_gutter(*row) as f32
                    })
                })?;
                return Some((frame_box.frame, (row, 0), (row, columns - 1)));
            }
        }
        None
    }

    fn in_side_margin<P: text::Paragraph>(&self, state: &State<P>, position: Point) -> bool {
        if state.pages.is_empty() {
            return false;
        }
        let page_index = self.view.page_at(position).min(state.pages.len() - 1);
        let page = &state.pages[page_index].page;
        let (x, y) = self.view.to_page(page_index, position);
        let (Some(first), Some(last)) = (page.columns.first(), page.columns.last()) else {
            return false;
        };
        let outside = (0.0..first.x).contains(&x) || (last.x + last.width < x && x <= page.width);
        outside
            && (0.0..=page.height).contains(&y)
            && frame_editing::frame_at(self.document, page, x, y).is_none()
    }

    /// A right press, before the right-click menu opens on release
    /// (`UI_SPEC.md` §16): a frame outside the selection is selected, and
    /// elsewhere the caret moves to the pointer unless it is in the selected
    /// text or frames, so that the menu acts on what was clicked. The
    /// menu's own state (`ContextAt`) comes after the move, so that the
    /// word it offers suggestions for is the one under the pointer (user,
    /// 2026-11-03: a suggestion chosen replaced nothing).
    fn press_right<P: text::Paragraph>(
        &self,
        state: &State<P>,
        position: Point,
        shell: &mut Shell<'_, Message>,
    ) {
        if state.pages.is_empty() {
            return;
        }
        self.select_under_right_press(state, position, shell);
        shell.publish((self.on_action)(PageAction::ContextAt {
            margin: self.in_side_margin(state, position),
        }));
    }

    /// What a right press selects (see [`Self::press_right`]).
    fn select_under_right_press<P: text::Paragraph>(
        &self,
        state: &State<P>,
        position: Point,
        shell: &mut Shell<'_, Message>,
    ) {
        let page_index = self.view.page_at(position).min(state.pages.len() - 1);
        let page = &state.pages[page_index].page;
        let (x, y) = self.view.to_page(page_index, position);
        // In a table's cells the right button acts on the cell, as a left
        // click does (TK6), even with the table's frame selected.
        let in_cells = frame_editing::frame_at(self.document, page, x, y)
            .is_some_and(|frame| self.in_table_cells(page, frame, x, y));
        if let Some(on_frame) = &self.on_frame
            && !in_cells
        {
            if self.in_selection(state, page_index, position) {
                return;
            }
            if let Some(frame) = frame_editing::frame_at(self.document, page, x, y) {
                shell.publish(on_frame(FrameAction::Select {
                    frame,
                    page: page.number,
                    extend: false,
                }));
                return;
            }
        }
        let Some((hit, page)) = self.hit(state, position) else {
            return;
        };
        let (start, end) = self.selection.ordered();
        let key = |position: TextPosition| (position.paragraph, position.byte);
        let in_selected_cells = self.frames.is_none()
            && crate::table_editing::cell_selection(self.document, self.selection).is_some_and(
                |cells| {
                    crate::table_editing::cell_of(self.document, hit.flow).is_some_and(
                        |(frame, row, column)| {
                            frame == cells.frame && cells.range().contains(row, column)
                        },
                    )
                },
            );
        let in_text = in_selected_cells
            || (self.frames.is_none()
                && start != end
                && hit.flow == start.flow
                && (key(start)..=key(end)).contains(&key(hit)));
        if !in_text {
            shell.publish((self.on_action)(PageAction::MoveTo {
                position: hit,
                page,
                extend: false,
            }));
        }
    }

    /// Ends a frame drag: a move or a resize as one action, or a click in a
    /// selected table that enters the cell under it.
    fn release_frame<P: text::Paragraph>(
        &self,
        state: &mut State<P>,
        shell: &mut Shell<'_, Message>,
    ) {
        let (Some(drag), Some(on_frame)) = (state.frame_drag.take(), &self.on_frame) else {
            return;
        };
        let (dx, dy) = drag.delta();
        match drag.kind {
            DragKind::Move if drag.moved => {
                shell.publish(on_frame(FrameAction::Move { dx, dy, key: false }));
            }
            DragKind::Move if drag.on_selected => {
                let table = self
                    .frames
                    .and_then(|selection| selection.single(self.document))
                    .and_then(|frame| self.document.frame(frame))
                    .is_some_and(|frame| matches!(frame.content, FrameContent::Table(_)));
                if table {
                    shell.publish(on_frame(FrameAction::EnterText {
                        at: self.hit(state, drag.press),
                    }));
                }
            }
            DragKind::Resize(handle) if drag.moved => {
                if let Some((from, to)) = self.resize_preview(state, drag, handle) {
                    // Shift and a corner handle size a picture with its
                    // frame (Ami Pro's guide, ch. 19; P6).
                    let with_picture = state.modifiers.shift()
                        && handle.is_corner()
                        && self.single_picture_selected();
                    shell.publish(on_frame(FrameAction::Resize {
                        from,
                        to,
                        with_picture,
                    }));
                }
            }
            DragKind::Pan if drag.moved => {
                shell.publish(on_frame(FrameAction::Pan { dx, dy, key: false }));
            }
            _ => {}
        }
    }

    /// The selection's outline and handles on one page, above every frame,
    /// and a dashed outline where a drag would put it.
    fn draw_frame_selection<P, Renderer: renderer::Renderer>(
        &self,
        renderer: &mut Renderer,
        state: &State<P>,
        origin: Vector,
        page_index: usize,
        selection: &FrameSelection,
    ) {
        let Some(cached) = state.pages.get(page_index) else {
            return;
        };
        if cached.page.number != selection.page {
            return;
        }
        let members = selection.members(self.document);
        let Some(outline) = union(
            cached
                .page
                .frames
                .iter()
                .filter(|frame_box| members.contains(&frame_box.frame))
                .map(|frame_box| frame_box.bounds),
        ) else {
            return;
        };
        renderer.fill_quad(
            renderer::Quad {
                bounds: self.view.rect(page_index, outline) + origin,
                border: Border {
                    color: ACCENT,
                    width: 1.0,
                    radius: 0.0.into(),
                },
                ..renderer::Quad::default()
            },
            Background::Color(Color::TRANSPARENT),
        );
        for handle in self.shown_handles() {
            let (x, y) = handle.centre(outline);
            let centre = self.view.point(page_index, x, y) + origin;
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle::new(
                        Point::new(centre.x - HANDLE_SIZE / 2.0, centre.y - HANDLE_SIZE / 2.0),
                        Size::new(HANDLE_SIZE, HANDLE_SIZE),
                    ),
                    border: Border {
                        color: Color::WHITE,
                        width: 1.0,
                        radius: 0.0.into(),
                    },
                    ..renderer::Quad::default()
                },
                // Grey in picture mode, as Ami Pro shows it (PI8).
                if self.picture_mode {
                    Color::from_rgb(0.62, 0.62, 0.62)
                } else {
                    ACCENT
                },
            );
        }
        let Some(drag) = state
            .frame_drag
            .filter(|drag| drag.moved && drag.page_index == page_index)
        else {
            return;
        };
        let preview = match drag.kind {
            DragKind::Move => {
                let (dx, dy) = drag.delta();
                let (dx, dy) = members
                    .first()
                    .and_then(|frame| self.document.frame(*frame))
                    .map_or((dx, dy), |frame| {
                        frame_editing::constrain(frame.placement, dx, dy)
                    });
                amipro_layout::Bounds {
                    x: outline.x + dx as f32,
                    y: outline.y + dy as f32,
                    ..outline
                }
            }
            DragKind::Resize(handle) => self
                .resize_preview(state, drag, handle)
                .map_or(outline, |(_, to)| bounds_of(to)),
            // The picture itself follows the drag (`draw_picture`).
            DragKind::Pan => return,
        };
        draw_dashed(renderer, self.view.rect(page_index, preview) + origin);
    }

    fn main(&self) -> FlowId {
        self.document.main_flow_id()
    }

    /// `point` (workspace pixels) in twips on page `page_index`, kept on its
    /// paper, as the corner of a frame being drawn (`1268:191f`).
    fn on_paper(&self, page_index: usize, point: Point) -> (f32, f32) {
        let (x, y) = self.view.to_page(page_index, point);
        let (width, height) = self
            .layout
            .pages
            .get(page_index)
            .map_or((0.0, 0.0), |page| (page.width, page.height));
        (x.clamp(0.0, width), y.clamp(0.0, height))
    }

    /// The page and rectangle a drawing drag gives (`drawn_rect`), or `None`
    /// without an area.
    fn drawn(&self, draw: FrameDraw) -> Option<(u32, amipro_document::Rect)> {
        let page = self.layout.pages.get(draw.page_index)?;
        let twips = |(x, y): (f32, f32)| (x.round() as i32, y.round() as i32);
        amipro_editing::frame_creation::drawn_rect(
            twips(draw.start),
            twips(draw.current),
            page.width.round() as i32,
            page.height.round() as i32,
        )
        .map(|rect| (page.number, rect))
    }

    /// The outline drawn while a frame is being drawn, in page twips.
    fn drawn_outline(&self, draw: FrameDraw) -> Option<amipro_layout::Bounds> {
        self.drawn(draw).map(|(_, rect)| bounds_of(rect))
    }

    /// The line showing `position` (preferring `page` for flows on several
    /// pages) and its page index: the line where the byte starts, or the
    /// line it ends.
    fn line_of<P>(
        &self,
        state: &State<P>,
        position: TextPosition,
        page: u32,
    ) -> Option<(usize, usize)> {
        let mut ending = None;
        let mut fallback = None;
        let pages = state.pages.iter().enumerate();
        for (page_index, cached) in pages {
            for (line_index, line) in cached.page.lines.iter().enumerate() {
                if line.flow != position.flow || line.paragraph_index != position.paragraph {
                    continue;
                }
                let here = position.flow == self.main() || cached.page.number == page;
                if line.bytes.start <= position.byte && position.byte < line.bytes.end {
                    if here {
                        return Some((page_index, line_index));
                    }
                    fallback.get_or_insert((page_index, line_index));
                } else if position.byte == line.bytes.end || line.bytes.is_empty() {
                    if here {
                        ending = Some((page_index, line_index));
                    } else {
                        fallback.get_or_insert((page_index, line_index));
                    }
                }
            }
        }
        ending.or(fallback)
    }

    /// Where the caret shows in text that does not fit its box, which is
    /// hidden (V14): at the end of the box's last visible line before it,
    /// on `page` for flows other than the main text (VM, HDR1 and HDR2).
    fn shown_end<P>(
        &self,
        state: &State<P>,
        position: TextPosition,
        page: u32,
    ) -> Option<(usize, usize)> {
        let main = position.flow == self.main();
        state
            .pages
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, cached)| main || cached.page.number == page)
            .find_map(|(page_index, cached)| {
                shown_before(&cached.page.lines, position).map(|line| (page_index, line))
            })
    }

    /// The caret rectangle of `position` in workspace pixels.
    fn caret<P: text::Paragraph>(
        &self,
        state: &State<P>,
        position: TextPosition,
        page: u32,
    ) -> Option<(usize, Rectangle)> {
        let (page_index, line_index, byte) = match self.line_of(state, position, page) {
            Some((page_index, line_index)) => (page_index, line_index, position.byte),
            None => {
                let (page_index, line_index) = self.shown_end(state, position, page)?;
                let end = state.pages[page_index].page.lines[line_index].bytes.end;
                (page_index, line_index, end)
            }
        };
        let cached = &state.pages[page_index];
        let line = &cached.page.lines[line_index];
        let x = line_x(line, &cached.lines[line_index], byte, self.view.scale);
        let top = self.view.point(page_index, x, line.top);
        // A line that takes no height (only With para above anchors) shows
        // the caret down to its baseline.
        let height = line.height.max(line.baseline - line.top);
        Some((
            page_index,
            Rectangle::new(top, Size::new(1.0, height * self.view.scale)),
        ))
    }

    /// The text position under `point` (workspace pixels): text in the
    /// front-most frame there, a header or footer, or the main text.
    fn hit<P: text::Paragraph>(
        &self,
        state: &State<P>,
        point: Point,
    ) -> Option<(TextPosition, u32)> {
        if state.pages.is_empty() {
            return None;
        }
        let page_index = self.view.page_at(point).min(state.pages.len() - 1);
        let cached = &state.pages[page_index];
        let page = &cached.page;
        let (x, y) = self.view.to_page(page_index, point);
        let inside = |frame: &FrameBox| {
            x >= frame.bounds.x
                && x <= frame.bounds.right()
                && y >= frame.bounds.y
                && y <= frame.bounds.bottom()
        };
        let mut wanted: Option<LineFilter> = None;
        for frame in page.frames.iter().rev() {
            // A cell's picture takes the caret to its cell, beneath (T1).
            if !inside(frame)
                || self
                    .document
                    .frame(frame.frame)
                    .is_some_and(|frame| frame.placement.is_in_cell())
            {
                continue;
            }
            let id = frame.frame;
            match &frame.kind {
                FrameBoxKind::Text if frame.layer == Layer::Layout => {
                    // Header and footer bands hold the layout frame's text or
                    // a floating header or footer.
                    let band = frame.bounds;
                    wanted = Some(Box::new(move |line: &LineBox| {
                        matches!(line.container, Container::Floating(_))
                            && line.top < band.bottom()
                            && line.top + line.height > band.y
                            || line.container == Container::Frame(id)
                    }));
                }
                FrameBoxKind::Text => {
                    wanted = Some(Box::new(move |line: &LineBox| {
                        line.container == Container::Frame(id)
                    }));
                }
                FrameBoxKind::Table { cells, .. } => {
                    if let Some(cell) = cells.iter().position(|cell| {
                        cell.width > 0.0
                            && x >= cell.x
                            && x <= cell.right()
                            && y >= cell.y
                            && y <= cell.bottom()
                    }) {
                        wanted = Some(Box::new(move |line: &LineBox| {
                            line.container == Container::Cell { frame: id, cell }
                        }));
                    }
                }
                FrameBoxKind::Picture | FrameBoxKind::Legacy => continue,
            }
            if wanted.is_some() {
                break;
            }
        }
        let column = page
            .columns
            .iter()
            .position(|column| x < column.right() + 120.0)
            .unwrap_or(page.columns.len().saturating_sub(1));
        // A click in the footnote area edits the footnotes there, as in Ami
        // Pro's Layout mode (NR8); its continued messages take no caret.
        let wanted = wanted.or_else(|| {
            let separator = page.footnote_separator?;
            (y >= separator.y).then(|| {
                Box::new(|line: &LineBox| {
                    matches!(line.container, Container::Footnote(_))
                        && line.paragraph_index != usize::MAX
                }) as Box<dyn Fn(&LineBox) -> bool>
            })
        });
        let wanted = wanted.unwrap_or_else(|| {
            let columns = page.columns.clone();
            Box::new(move |line: &LineBox| {
                line.container == Container::Main
                    && columns
                        .get(column)
                        .is_none_or(|bounds| line.x + 1.0 >= bounds.x && line.x < bounds.right())
            })
        });
        let candidates: Vec<usize> = page
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| wanted(line))
            .map(|(index, _)| index)
            .collect();
        let candidates = if candidates.is_empty() {
            // A column without text: any main line on the page.
            page.lines
                .iter()
                .enumerate()
                .filter(|(_, line)| line.container == Container::Main)
                .map(|(index, _)| index)
                .collect()
        } else {
            candidates
        };
        let Some(line_index) = candidates.into_iter().min_by(|a, b| {
            let distance = |index: &usize| {
                let line = &page.lines[*index];
                if y < line.top {
                    line.top - y
                } else if y > line.top + line.height {
                    y - line.top - line.height
                } else {
                    0.0
                }
            };
            distance(a).total_cmp(&distance(b))
        }) else {
            return self.nearest_main(state, page_index);
        };
        let line = &page.lines[line_index];
        let byte = line_hit(line, &cached.lines[line_index], x, self.view.scale);
        Some((
            TextPosition {
                flow: line.flow,
                paragraph: line.paragraph_index,
                byte,
            },
            page.number,
        ))
    }

    /// For a page without main text: the end of the main text before it, or
    /// the start of the main text after it.
    fn nearest_main<P>(&self, state: &State<P>, page_index: usize) -> Option<(TextPosition, u32)> {
        let main_line = |cached: &PageCache<P>, last: bool| {
            let mut lines = cached
                .page
                .lines
                .iter()
                .filter(|line| line.container == Container::Main);
            let line = if last {
                lines.next_back()
            } else {
                lines.next()
            }?;
            Some((
                TextPosition {
                    flow: line.flow,
                    paragraph: line.paragraph_index,
                    byte: if last {
                        line.bytes.end
                    } else {
                        line.bytes.start
                    },
                },
                cached.page.number,
            ))
        };
        state.pages[..page_index]
            .iter()
            .rev()
            .find_map(|cached| main_line(cached, true))
            .or_else(|| {
                state.pages[page_index..]
                    .iter()
                    .find_map(|cached| main_line(cached, false))
            })
    }

    /// The lines a caret on `line_of` moves through with Up and Down: the
    /// main text across pages, or the same container on the same page.
    fn sequence<P>(
        &self,
        state: &State<P>,
        page_index: usize,
        line_index: usize,
    ) -> Vec<(usize, usize)> {
        let line = &state.pages[page_index].page.lines[line_index];
        let container = line.container;
        if container == Container::Main {
            state
                .pages
                .iter()
                .enumerate()
                .flat_map(|(page, cached)| {
                    cached
                        .page
                        .lines
                        .iter()
                        .enumerate()
                        .filter(|(_, line)| line.container == Container::Main)
                        .map(move |(index, _)| (page, index))
                })
                .collect()
        } else {
            state.pages[page_index]
                .page
                .lines
                .iter()
                .enumerate()
                .filter(|(_, candidate)| candidate.container == container)
                .map(|(index, _)| (page_index, index))
                .collect()
        }
    }

    /// The target of Up, Down, Home, End, Page Up, or Page Down.
    fn navigate<P: text::Paragraph>(
        &self,
        state: &State<P>,
        key: &Named,
        control: bool,
    ) -> Option<(TextPosition, u32)> {
        let focus = self.selection.focus;
        if control && matches!(key, Named::Home | Named::End) {
            let position = if *key == Named::Home {
                TextPosition {
                    flow: focus.flow,
                    paragraph: 0,
                    byte: 0,
                }
            } else {
                crate::page_editing::flow_end(self.document, focus.flow)
            };
            return Some((position, self.selection.page));
        }
        let Some((page_index, line_index)) = self.line_of(state, focus, self.selection.page) else {
            // In hidden text: back to the line the caret is shown on.
            let (page_index, line_index) = self.shown_end(state, focus, self.selection.page)?;
            let cached = &state.pages[page_index];
            let line = &cached.page.lines[line_index];
            let byte = match key {
                Named::Home => line.bytes.start,
                Named::End | Named::ArrowUp | Named::PageUp => line.bytes.end,
                _ => return None,
            };
            let position = TextPosition {
                flow: line.flow,
                paragraph: line.paragraph_index,
                byte,
            };
            return Some((position, cached.page.number));
        };
        let cached = &state.pages[page_index];
        let line = &cached.page.lines[line_index];
        let position = |page: usize, index: usize, byte: usize| {
            let line = &state.pages[page].page.lines[index];
            (
                TextPosition {
                    flow: line.flow,
                    paragraph: line.paragraph_index,
                    byte,
                },
                state.pages[page].page.number,
            )
        };
        match key {
            Named::Home => Some(position(page_index, line_index, line.bytes.start)),
            Named::End => {
                let end = if source_of(self.document, line).is_some_and(|source| {
                    source.text[..line.bytes.end].ends_with([' ', '\u{2028}'])
                }) && line.bytes.end > line.bytes.start
                {
                    // Before a hanging space or hard return, so End stays
                    // on this line.
                    let text = &source_of(self.document, line)?.text;
                    text[..line.bytes.end].char_indices().next_back()?.0
                } else {
                    line.bytes.end
                };
                Some(position(page_index, line_index, end))
            }
            Named::ArrowUp | Named::ArrowDown | Named::PageUp | Named::PageDown => {
                let x = line_x(line, &cached.lines[line_index], focus.byte, self.view.scale);
                let sequence = self.sequence(state, page_index, line_index);
                let here = sequence
                    .iter()
                    .position(|entry| *entry == (page_index, line_index))?;
                let target = match key {
                    Named::ArrowUp => here.checked_sub(1)?,
                    Named::ArrowDown => (here + 1 < sequence.len()).then_some(here + 1)?,
                    _ => {
                        // A screenful: the line that far above or below.
                        let height =
                            (state.viewport.height.max(100.0) / self.view.scale).max(line.height);
                        let current = self.view.point(page_index, x, line.top).y;
                        let goal = if *key == Named::PageUp {
                            current - height * self.view.scale
                        } else {
                            current + height * self.view.scale
                        };
                        let by_distance = |entry: &(usize, usize)| {
                            let candidate = &state.pages[entry.0].page.lines[entry.1];
                            (self.view.point(entry.0, x, candidate.top).y - goal).abs()
                        };
                        let best = sequence
                            .iter()
                            .enumerate()
                            .min_by(|a, b| by_distance(a.1).total_cmp(&by_distance(b.1)))?
                            .0;
                        if best == here {
                            if *key == Named::PageUp {
                                0
                            } else {
                                sequence.len() - 1
                            }
                        } else {
                            best
                        }
                    }
                };
                let (target_page, target_line) = sequence[target];
                let line = &state.pages[target_page].page.lines[target_line];
                let byte = line_hit(
                    line,
                    &state.pages[target_page].lines[target_line],
                    x,
                    self.view.scale,
                );
                Some(position(target_page, target_line, byte))
            }
            _ => None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// A cell's leader (Table › Leaders) on its last line on this page, in
    /// the line's size, by [`cell_leader_span`].
    #[allow(clippy::too_many_arguments)]
    fn draw_cell_leader<Renderer: renderer::Renderer>(
        &self,
        renderer: &mut Renderer,
        origin: Vector,
        page_index: usize,
        frame_box: &FrameBox,
        index: usize,
        text: amipro_layout::Bounds,
        cell: &amipro_document::TableCell,
        page_lines: &[LineBox],
    ) {
        let Some(line) = page_lines
            .iter()
            .filter(|line| {
                line.container
                    == Container::Cell {
                        frame: frame_box.frame,
                        cell: index,
                    }
            })
            .max_by(|a, b| a.top.total_cmp(&b.top))
        else {
            return;
        };
        let Some(paragraph) = self
            .document
            .flow(line.flow)
            .and_then(|flow| flow.paragraphs.get(line.paragraph_index))
        else {
            return;
        };
        let Some(style) = self.document.styles().get(&paragraph.style) else {
            return;
        };
        let alignment = paragraph.format.alignment.unwrap_or(style.alignment);
        let Some((start, end)) = cell_leader_span(text, line, alignment) else {
            return;
        };
        let scale = self.view.scale;
        let from = self.view.point(page_index, start, line.baseline) + origin;
        let to = self.view.point(page_index, end, line.baseline) + origin;
        let leader = match cell.format.leader {
            amipro_document::CellLeader::None => return,
            amipro_document::CellLeader::Dots => TabLeader::Dots,
            amipro_document::CellLeader::Dashes => TabLeader::Hyphens,
            amipro_document::CellLeader::Line => TabLeader::Line,
        };
        let size = style.font_size_points * 20.0 * scale;
        for bounds in leader_marks(leader, from.x, to.x, from.y, size) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    ..renderer::Quad::default()
                },
                Color::BLACK,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_frame<
        Renderer: text::Renderer<Font = Font> + image::Renderer<Handle = image::Handle>,
    >(
        &self,
        renderer: &mut Renderer,
        origin: Vector,
        page_index: usize,
        frame_box: &FrameBox,
        frame: Option<&Frame>,
        page_lines: &[LineBox],
        clip: Rectangle,
        pan_preview: Option<(amipro_document::FrameId, (i32, i32))>,
    ) {
        let scale = self.view.scale;
        let bounds = self.view.rect(page_index, frame_box.bounds) + origin;
        let Some(frame) = frame else {
            return;
        };
        // Header and footer frames are bands of the page layout: Ami Pro
        // draws neither fill nor lines for them (README31 stores a black,
        // opaque background).
        if matches!(
            frame.placement,
            amipro_document::Placement::Header { .. } | amipro_document::Placement::Footer { .. }
        ) {
            return;
        }
        let style = &frame.style;
        // On a preprinted form a frame's lines, shadow, and fill are not
        // printed, and an opaque frame hides what is behind it in white
        // (`PRINT_NOTES.md`, `1530:0000`).
        let preprinted = self.printing.is_some_and(|mode| mode.preprinted_form);
        // Rounded corners: a radius of corner% × half the shorter side, as
        // Ami Pro rounds within the frame (user's guide ch. 15; F01, FC1).
        // Ami Pro draws an ellipse's quarter where the sides differ at
        // 100 %; a circle's is drawn here.
        let radius =
            f32::from(style.corners.min(100)) / 100.0 * bounds.width.min(bounds.height) / 2.0;
        let shadow = style.shadow;
        if !preprinted
            && (shadow.right > 0 || shadow.bottom > 0 || shadow.left > 0 || shadow.top > 0)
        {
            let offset = Vector::new(
                (shadow.right - shadow.left) as f32 * scale,
                (shadow.bottom - shadow.top) as f32 * scale,
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: bounds + offset,
                    border: Border {
                        radius: radius.into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                colour(shadow.colour),
            );
        }
        // A picture frame's background, lines, shadow, and corners are
        // drawn as any frame's, the picture between them (PI1).
        if style.opaque {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: Border {
                        radius: radius.into(),
                        ..Border::default()
                    },
                    ..renderer::Quad::default()
                },
                if preprinted {
                    Color::WHITE
                } else {
                    colour(style.background)
                },
            );
        }
        // Without pictures (and on a preprinted form) a picture frame
        // prints empty.
        let pictures_shown = self
            .printing
            .is_none_or(|mode| !mode.without_pictures && !mode.preprinted_form);
        if let FrameContent::Picture(picture) = &frame.content
            && pictures_shown
        {
            // A picture being moved in picture mode follows the drag.
            let pan = pan_preview
                .filter(|(moving, _)| *moving == frame_box.frame)
                .map(|(_, delta)| delta);
            self.draw_picture(renderer, origin, page_index, frame_box, picture, clip, pan);
        }
        // Ami Pro's Lines & Color (EX5; VM TL1, TL4): each cell's fill over
        // its box (gutter middle to gutter middle) and its black lines
        // centred on the box's edges, on screen and on paper; not on a
        // preprinted form.
        if let (FrameBoxKind::Table { cells, part }, FrameContent::Table(table)) =
            (&frame_box.kind, &frame.content)
            && !preprinted
        {
            for (index, text) in cells.iter().enumerate() {
                let Some(cell) = table.cells.get(index) else {
                    continue;
                };
                if text.width <= 0.0 || cell.span.is_covered() {
                    continue;
                }
                let (lines, fill) = (cell.lines(), cell.fill());
                if cell.format.leader != amipro_document::CellLeader::None && cell.frame.is_none() {
                    self.draw_cell_leader(
                        renderer, origin, page_index, frame_box, index, *text, cell, page_lines,
                    );
                }
                if lines.is_none() && fill.is_none() {
                    continue;
                }
                let boxed = amipro_layout::cell_box(table, part, index, *text);
                if let Some(fill) = fill {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: self.view.rect(page_index, boxed) + origin,
                            ..renderer::Quad::default()
                        },
                        colour(fill),
                    );
                }
                for stroke in amipro_layout::cell_line_strokes(boxed, lines) {
                    let mut bounds = self.view.rect(page_index, stroke) + origin;
                    if self.printing.is_none() {
                        bounds.width = bounds.width.max(1.0);
                        bounds.height = bounds.height.max(1.0);
                    }
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds,
                            ..renderer::Quad::default()
                        },
                        // Black, Ami Pro's; another is the app's own (TD8).
                        colour(cell.format.line_colour),
                    );
                }
            }
        }
        // Selected cells, whole, gutters included, as Ami Pro inverts them
        // (`14a0:0083`; VM TE8).
        if let (FrameBoxKind::Table { cells, part }, FrameContent::Table(table)) =
            (&frame_box.kind, &frame.content)
            && self.printing.is_none()
            && self.frames.is_none()
            && let Some(selected) =
                crate::table_editing::cell_selection(self.document, self.selection)
            && selected.frame == frame_box.frame
        {
            let range = selected.range();
            let columns = table.columns.len().max(1);
            for (index, text) in cells.iter().enumerate() {
                let (row, column) = (index / columns, index % columns);
                if text.width <= 0.0
                    || !range.contains(row, column)
                    || table
                        .cells
                        .get(index)
                        .is_some_and(|cell| cell.span.is_covered())
                {
                    continue;
                }
                let boxed = amipro_layout::cell_box(table, part, index, *text);
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: self.view.rect(page_index, boxed) + origin,
                        ..renderer::Quad::default()
                    },
                    SELECTION,
                );
            }
        }
        // The grid is the screen's guide to the cells, as Ami Pro's dotted
        // one; it does not print.
        if let FrameBoxKind::Table { cells, part } = &frame_box.kind
            && self.printing.is_none()
        {
            let columns = match &frame.content {
                FrameContent::Table(table) => table.columns.len().max(1),
                _ => 1,
            };
            for (index, cell) in cells.iter().enumerate() {
                // Cells of the table's other parts have no box here.
                if cell.width <= 0.0 {
                    continue;
                }
                let bounds = self.view.rect(page_index, *cell) + origin;
                // No grid line where a row continues on the next part or
                // from the last (Rows span pages, TP5).
                let row = index / columns;
                let open_below = part.cut && row + 1 == part.rows.end;
                let open_above = part.continued() && row == part.rows.start;
                if !open_below && !open_above {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds,
                            border: Border {
                                color: CELL_LINE,
                                width: 1.0,
                                radius: 0.0.into(),
                            },
                            ..renderer::Quad::default()
                        },
                        Background::Color(Color::TRANSPARENT),
                    );
                    continue;
                }
                for (edge, drawn) in
                    cell_edges(bounds)
                        .into_iter()
                        .zip([!open_above, !open_below, true, true])
                {
                    if drawn {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: edge,
                                ..renderer::Quad::default()
                            },
                            CELL_LINE,
                        );
                    }
                }
            }
        }
        // A formula cell has a thin box round it while the caret is in it
        // (Ami Pro; `UI_SPEC.md` §41).
        if let (FrameBoxKind::Table { cells, part }, FrameContent::Table(table)) =
            (&frame_box.kind, &frame.content)
            && self.printing.is_none()
            && self.frames.is_none()
            && crate::table_editing::cell_selection(self.document, self.selection).is_none()
            && let Some((caret_frame, row, column)) =
                crate::table_editing::caret_cell(self.document, self.selection)
            && caret_frame == frame_box.frame
        {
            let index = row * table.columns.len().max(1) + column;
            if let (Some(cell), Some(text)) = (table.cells.get(index), cells.get(index))
                && cell.formula.is_some()
                && text.width > 0.0
            {
                let boxed = amipro_layout::cell_box(table, part, index, *text);
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: self.view.rect(page_index, boxed) + origin,
                        border: Border {
                            color: ACCENT,
                            width: 1.0,
                            radius: 0.0.into(),
                        },
                        ..renderer::Quad::default()
                    },
                    Background::Color(Color::TRANSPARENT),
                );
            }
        }
        // View › Show ▸ Table row/column headings (TN9): letters over the
        // columns, numbers left of the rows of a page table, in the
        // table's margins; the screen only.
        if self.view_marks.table_addresses
            && self.printing.is_none()
            && frame.placement.is_anchored()
            && let (FrameBoxKind::Table { cells, part }, FrameContent::Table(table)) =
                (&frame_box.kind, &frame.content)
        {
            let columns = table.columns.len().max(1);
            let first_row = if part.rows.is_empty() {
                0
            } else {
                part.rows.start
            };
            let size = Pixels((7.0 * 20.0 * scale).max(6.0));
            let mut label = |text: String, area: Rectangle, align_x: text::Alignment| {
                renderer.fill_text(
                    text::Text {
                        content: text,
                        bounds: area.size(),
                        size,
                        line_height: text::LineHeight::default(),
                        font: Font::default(),
                        align_x,
                        align_y: cosmic::iced::alignment::Vertical::Center,
                        shaping: text::Shaping::Basic,
                        wrapping: text::Wrapping::None,
                        ellipsize: text::Ellipsize::None,
                    },
                    match align_x {
                        text::Alignment::Right => Point::new(area.x + area.width, area.center_y()),
                        _ => Point::new(area.center_x(), area.center_y()),
                    },
                    SHOW_MARK,
                    clip,
                );
            };
            let height = 10.0 * 20.0 * scale;
            let width = 16.0 * 20.0 * scale;
            for (index, cell) in cells.iter().enumerate() {
                if cell.width <= 0.0
                    || table
                        .cells
                        .get(index)
                        .is_some_and(|cell| cell.span.is_covered())
                {
                    continue;
                }
                let (row, column) = (index / columns, index % columns);
                let bounds = self.view.rect(page_index, *cell) + origin;
                if row == first_row && !part.continued() {
                    label(
                        amipro_document::formula::column_name(column),
                        Rectangle::new(
                            Point::new(bounds.x, bounds.y - height),
                            Size::new(bounds.width, height),
                        ),
                        text::Alignment::Center,
                    );
                }
                if column == 0 {
                    label(
                        (row + 1).to_string(),
                        Rectangle::new(
                            Point::new(bounds.x - width - 2.0, bounds.y),
                            Size::new(width, bounds.height),
                        ),
                        text::Alignment::Right,
                    );
                }
            }
        }
        // The line between a text frame's columns (C04).
        let strokes: &[_] = if preprinted { &[] } else { &frame_box.strokes };
        for (bounds, colour) in stroke_quads(&self.view, page_index, strokes, origin) {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    ..renderer::Quad::default()
                },
                colour,
            );
        }
        if preprinted {
            return;
        }
        // On screen a frame's lines are one or two pixels wide; on paper
        // (and in the Print panel's preview) each is its style's width,
        // inside, across, or outside the frame's edge as its position says
        // (PX9).
        let (lines, width, radius) = match self.printing {
            None => (
                bounds,
                if style.border.style > 1 { 2.0 } else { 1.0 },
                radius,
            ),
            Some(_) => {
                let width = (amipro_document::line_style_width(style.border.style) as f32 * scale)
                    .max(if self.printing.is_some_and(|mode| mode.preview) {
                        1.0
                    } else {
                        0.0
                    });
                let outward = match style.border.position {
                    amipro_document::LinePosition::Middle => width / 2.0,
                    amipro_document::LinePosition::Outside
                    | amipro_document::LinePosition::CloseToOutside => width,
                    amipro_document::LinePosition::Inside
                    | amipro_document::LinePosition::CloseToInside => 0.0,
                };
                // Square corners stay square.
                let radius = if radius > 0.0 { radius + outward } else { 0.0 };
                (bounds.expand(outward), width, radius)
            }
        };
        if matches!(frame.content, FrameContent::Picture(_)) {
            // The picture's layer comes first; the lines go above it in one
            // of their own (a layer draws after its parent's primitives).
            let reach = lines.expand(4.0);
            renderer.with_layer(reach.intersection(&clip).unwrap_or(reach), |renderer| {
                draw_frame_lines(renderer, lines, style, radius, width);
            });
        } else {
            draw_frame_lines(renderer, lines, style, radius, width);
        }
    }

    /// A picture in its frame (`UI_SPEC.md` §25): at the size its scaling
    /// gives in the picture area (the frame's text area), moved by its pan,
    /// clipped to the area; Ami Pro's screen image over the area for kinds
    /// no reader takes; else a labelled box.
    #[allow(clippy::too_many_arguments)]
    fn draw_picture<
        Renderer: text::Renderer<Font = Font> + image::Renderer<Handle = image::Handle>,
    >(
        &self,
        renderer: &mut Renderer,
        origin: Vector,
        page_index: usize,
        frame_box: &FrameBox,
        picture: &amipro_document::Picture,
        clip: Rectangle,
        drag: Option<(i32, i32)>,
    ) {
        let area = picture_area(frame_box);
        let (drag_x, drag_y) = drag.unwrap_or((0, 0));
        let area_bounds = self.view.rect(page_index, area) + origin;
        let Some(visible) = area_bounds.intersection(&clip) else {
            return;
        };
        let shown = self.pictures.map(|(pictures, folder)| {
            let natural = pictures.natural_size(self.document, picture, folder);
            let target = match natural {
                Some(natural) => {
                    let size = picture.scaling.drawn_size(
                        natural,
                        amipro_document::Size {
                            width: area.width.round() as i32,
                            height: area.height.round() as i32,
                        },
                    );
                    let placed = amipro_layout::Bounds {
                        x: area.x + (picture.placed_pan().x + drag_x) as f32,
                        y: area.y + (picture.placed_pan().y + drag_y) as f32,
                        width: size.width as f32,
                        height: size.height as f32,
                    };
                    self.view.rect(page_index, placed) + origin
                }
                None => area_bounds,
            };
            // On paper, pictures drawn from vectors are drawn at 300 dpi
            // when they cannot be handed over as vectors (one dot the
            // thinnest line); on screen at twice the screen's pixels.
            let (oversample, pixel) = if self.printing.is_some() {
                (PRINT_DPI / 1_440.0 / self.view.scale, 1.0)
            } else {
                (PICTURE_OVERSAMPLE, PICTURE_OVERSAMPLE)
            };
            let pixels = (
                (target.width * oversample).ceil() as u32,
                (target.height * oversample).ceil() as u32,
            );
            // Only the PDF draws the stand-ins (as vectors); the preview
            // on screen draws the pictures themselves.
            let printed = self
                .printing
                .filter(|mode| !mode.preview)
                .and_then(|_| pictures.printed(self.document, picture, folder));
            (
                printed.map_or_else(
                    || pictures.shown(self.document, picture, folder, pixels, pixel),
                    crate::pictures::Shown::Picture,
                ),
                target,
            )
        });
        match shown {
            Some((
                crate::pictures::Shown::Picture(handle) | crate::pictures::Shown::Screen(handle),
                target,
            )) => {
                // Clipped to the picture area, as Ami Pro clips it; a turned
                // picture comes turned into its box, which is stretched onto
                // the rectangle (PIRT30, PIRT90).
                renderer.with_layer(visible, |renderer| {
                    renderer.draw_image(
                        image::Image::new(handle).filter_method(image::FilterMethod::Linear),
                        target,
                        visible,
                    );
                });
            }
            // A picture the app cannot draw prints as an empty frame (PX1).
            _ if self.printing.is_some() => {}
            other => {
                let label = match other {
                    Some((crate::pictures::Shown::Box(label), _)) => label,
                    _ => picture.format().name().to_owned(),
                };
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: visible,
                        ..renderer::Quad::default()
                    },
                    PLACEHOLDER,
                );
                renderer.fill_text(
                    text::Text {
                        content: label,
                        bounds: area_bounds.size(),
                        size: Pixels((9.0 * 20.0 * self.view.scale).max(7.0)),
                        line_height: text::LineHeight::default(),
                        font: Font::default(),
                        align_x: text::Alignment::Center,
                        align_y: cosmic::iced::alignment::Vertical::Center,
                        shaping: text::Shaping::Basic,
                        wrapping: text::Wrapping::Word,
                        ellipsize: text::Ellipsize::None,
                    },
                    area_bounds.center(),
                    Color::from_rgb(0.35, 0.35, 0.35),
                    visible,
                );
            }
        }
    }

    /// Tabs & Returns: an arrow across each tab's gap and a pilcrow after a
    /// paragraph's last line, drawn with quads in the mark colour.
    fn draw_show_marks<Renderer: text::Renderer<Font = Font>>(
        &self,
        renderer: &mut Renderer,
        page_index: usize,
        line: &LineBox,
        origin: Vector,
    ) {
        let scale = self.view.scale;
        let height = line.height.max(LINE_FALLBACK) * scale;
        let mut bar = |x: f32, y: f32, width: f32, tall: f32| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: Rectangle::new(Point::new(x, y), Size::new(width, tall)),
                    ..renderer::Quad::default()
                },
                SHOW_MARK,
            );
        };
        for word in line.words.iter().filter(|word| word.tab.is_some()) {
            let start = self.view.point(page_index, word.x, line.top) + origin;
            let width = (word.width * scale).max(4.0);
            let middle = start.y + height * 0.55;
            let length = (width * 0.7).max(4.0);
            let x = start.x + (width - length) / 2.0;
            bar(x, middle, length, 1.0);
            bar(x + length - 1.0, middle - 2.0, 1.0, 5.0);
        }
        let ends_paragraph = self
            .document
            .flow(line.flow)
            .and_then(|flow| flow.paragraphs.get(line.paragraph_index))
            .is_some_and(|paragraph| line.bytes.end >= paragraph.text.len());
        if ends_paragraph {
            let at = self.view.point(page_index, line.x + line.width, line.top) + origin;
            let (x, y) = (at.x + 2.0, at.y + height * 0.2);
            let tall = (height * 0.65).max(6.0);
            // A pilcrow: its bowl, then two stems.
            bar(x, y, (tall * 0.35).max(3.0), (tall * 0.4).max(3.0));
            bar(x + (tall * 0.3).max(2.0), y, 1.0, tall);
            bar(x + (tall * 0.45).max(3.0), y, 1.0, tall);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_lines<Renderer: text::Renderer<Font = Font>>(
        &self,
        renderer: &mut Renderer,
        state: &State<Renderer::Paragraph>,
        origin: Vector,
        page_index: usize,
        wanted: impl Fn(&LineBox) -> bool,
        selection: Option<(TextPosition, TextPosition)>,
        clip: Rectangle,
    ) {
        let cached = &state.pages[page_index];
        let scale = self.view.scale;
        let main = self.main();
        for (line, cache) in cached.page.lines.iter().zip(&cached.lines) {
            if !wanted(line) {
                continue;
            }
            let top = self.view.point(page_index, 0.0, line.top) + origin;
            if top.y > clip.y + clip.height || top.y + line.height * scale < clip.y {
                continue;
            }
            // Field Shading: each field's shown text, on screen only.
            if let Some(ranges) = self
                .shading
                .filter(|_| self.printing.is_none())
                .and_then(|shading| shading.get(&(line.flow, line.paragraph_index)))
            {
                for range in ranges {
                    let from = range.start.max(line.bytes.start);
                    let to = range.end.min(line.bytes.end);
                    if from < to {
                        fill_bytes(
                            renderer,
                            cache,
                            &self.view,
                            page_index,
                            line,
                            origin,
                            from..to,
                            FIELD_SHADE,
                            true,
                        );
                    }
                }
            }
            // The find bar's matches, wherever their lines show.
            if let Some(ranges) = self
                .highlights
                .filter(|_| self.printing.is_none())
                .and_then(|highlights| highlights.get(&(line.flow, line.paragraph_index)))
            {
                for range in ranges {
                    // The current match is the selection: drawn as it alone.
                    let selected = selection.is_some_and(|(start, end)| {
                        start.flow == line.flow
                            && (start.paragraph, start.byte) <= (line.paragraph_index, range.start)
                            && (line.paragraph_index, range.end) <= (end.paragraph, end.byte)
                    });
                    let from = range.start.max(line.bytes.start);
                    let to = range.end.min(line.bytes.end);
                    if from < to && !selected {
                        fill_bytes(
                            renderer,
                            cache,
                            &self.view,
                            page_index,
                            line,
                            origin,
                            from..to,
                            MATCH,
                            false,
                        );
                    }
                }
            }
            // Spelling and grammar as you type: a wavy line under the
            // words, on screen only.
            for (marks, colour) in [(self.misspelled, MISSPELLED), (self.grammar, GRAMMAR)] {
                if let Some(ranges) = marks
                    .filter(|_| self.printing.is_none())
                    .and_then(|marks| marks.get(&(line.flow, line.paragraph_index)))
                {
                    for range in ranges {
                        let from = range.start.max(line.bytes.start);
                        let to = range.end.min(line.bytes.end);
                        if from < to {
                            wavy_bytes(
                                renderer,
                                cache,
                                &self.view,
                                page_index,
                                line,
                                origin,
                                from..to,
                                colour,
                            );
                        }
                    }
                }
            }
            if let Some((start, end)) = selection
                && shows_flow(line, start.flow, &cached.page, self.selection.page, main)
                && (start.paragraph, start.byte)
                    < (
                        line.paragraph_index,
                        line.bytes.end.max(line.bytes.start + 1),
                    )
                && (end.paragraph, end.byte) > (line.paragraph_index, line.bytes.start)
            {
                let from = if start.paragraph == line.paragraph_index {
                    start.byte.max(line.bytes.start)
                } else {
                    line.bytes.start
                };
                let to = if end.paragraph == line.paragraph_index {
                    end.byte.min(line.bytes.end)
                } else {
                    line.bytes.end
                };
                fill_bytes(
                    renderer,
                    cache,
                    &self.view,
                    page_index,
                    line,
                    origin,
                    from..to,
                    SELECTION,
                    true,
                );
            }
            if self.view_marks.tabs_and_returns && self.printing.is_none() {
                self.draw_show_marks(renderer, page_index, line, origin);
            }
            // Note marks, over their neighbours: a box in the note's
            // colour with a black outline, 5/9 of the line's height wide
            // and 5/6 high, centred on the note's place (`1108:35a6`; VM
            // RNN1–RNN3). Not printed (Print Options › With notes prints
            // the notes' numbers instead).
            if self.notes
                && self.printing.is_none()
                && let Some(source) = &cache.source
            {
                for word in &line.words {
                    let Some(InlineObject::Note { note, .. }) =
                        word.object.and_then(|object| source.objects.get(object))
                    else {
                        continue;
                    };
                    // h is the line's height: the box stands from h/6
                    // below the baseline upward.
                    let height = line.height.max(LINE_FALLBACK);
                    let (width, tall) = (height * 5.0 / 9.0, height * 5.0 / 6.0);
                    let corner = self.view.point(
                        page_index,
                        word.x - width / 2.0,
                        line.baseline + height / 6.0 - tall,
                    ) + origin;
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle::new(
                                corner,
                                Size::new((width * scale).max(4.0), (tall * scale).max(6.0)),
                            ),
                            border: Border {
                                color: Color::BLACK,
                                width: 1.0,
                                radius: 0.0.into(),
                            },
                            ..renderer::Quad::default()
                        },
                        colour(note.colour.unwrap_or(amipro_document::NoteInfo::YELLOW)),
                    );
                }
            }
            // View › Bookmarks: thin grey brackets at a text bookmark's ends
            // (NR5); never printed.
            if self.bookmarks
                && self.printing.is_none()
                && let Some(source) = &cache.source
            {
                for mark in &source.marks {
                    let Some(parsed) = amipro_document::bookmarks::BookmarkMark::parse(&mark.raw)
                    else {
                        continue;
                    };
                    if mark.byte < line.bytes.start
                        || mark.byte > line.bytes.end
                        || (mark.byte == line.bytes.end && !parsed.end)
                    {
                        continue;
                    }
                    let x = line_x(line, cache, mark.byte, scale);
                    let top = self.view.point(page_index, x, line.top) + origin;
                    let tall = (line.height * scale).max(4.0);
                    let arm = (60.0 * scale).max(2.0);
                    let (stem_x, arm_x) = if parsed.end {
                        (top.x, top.x - arm)
                    } else {
                        (top.x, top.x)
                    };
                    for bounds in [
                        Rectangle::new(Point::new(stem_x, top.y), Size::new(1.0, tall)),
                        Rectangle::new(Point::new(arm_x, top.y), Size::new(arm, 1.0)),
                        Rectangle::new(Point::new(arm_x, top.y + tall - 1.0), Size::new(arm, 1.0)),
                    ] {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds,
                                ..renderer::Quad::default()
                            },
                            BOOKMARK_GREY,
                        );
                    }
                }
            }
            // Rules one 15-twip pixel thick at 100 %, thicker when zoomed.
            let thickness = (15.0 * scale).round().max(1.0);
            let baseline = (line.baseline - line.top) * scale;
            for segment in &cache.segments {
                let position = self.view.point(page_index, segment.x, line.top) + origin;
                let text_position = position + Vector::new(0.0, cache.text_shift);
                renderer.fill_paragraph(&segment.paragraph, text_position, Color::BLACK, clip);
                for (rise, paragraph) in &segment.shifted {
                    let shifted = text_position - Vector::new(0.0, *rise);
                    renderer.fill_paragraph(paragraph, shifted, Color::BLACK, clip);
                }
                for (glyph, offsets) in &segment.overstrikes {
                    for offset in offsets {
                        renderer.fill_paragraph(glyph, text_position + *offset, Color::BLACK, clip);
                    }
                }
                for decoration in &segment.decorations {
                    let y = baseline - decoration.rise;
                    let mut rule = |offset: f32| {
                        // On whole pixel rows, so that rules stay sharp and
                        // a double underline shows two lines.
                        let top = (position.y + y + offset).round();
                        for bounds in segment.paragraph.span_bounds(decoration.span) {
                            renderer.fill_quad(
                                renderer::Quad {
                                    bounds: Rectangle::new(
                                        Point::new(position.x + bounds.x, top),
                                        Size::new(bounds.width, thickness),
                                    ),
                                    ..renderer::Quad::default()
                                },
                                decoration.colour,
                            );
                        }
                    };
                    if decoration.underline || decoration.double_underline {
                        rule(thickness);
                    }
                    if decoration.double_underline {
                        rule(3.0 * thickness);
                    }
                    if decoration.strikethrough {
                        // About where fonts put their strike-out stroke.
                        rule(-(decoration.size * 0.26).round());
                    }
                }
            }
            // Bullets and numbers before the first line.
            if line.bullet.is_some() {
                let page_left = self.view.point(page_index, 0.0, 0.0).x + origin.x;
                for text in &cache.bullet_text {
                    let position = self.view.point(page_index, text.x, line.top) + origin
                        - Vector::new(0.0, text.rise);
                    renderer.fill_paragraph(&text.paragraph, position, Color::BLACK, clip);
                }
                for mark in bullet_marks(line, scale, page_left, top.y) {
                    fill_bullet_mark(renderer, mark, Color::BLACK);
                }
            }
            // Bullets put into the text.
            if !cache.inline_bullets.is_empty() {
                let page_left = self.view.point(page_index, 0.0, 0.0).x + origin.x;
                for (mark, colour) in
                    inline_bullet_marks(line, &cache.inline_bullets, scale, page_left, top.y)
                {
                    fill_bullet_mark(renderer, mark, colour);
                }
            }
            // Tab leaders, in the paragraph style's size.
            let page_left = self.view.point(page_index, 0.0, 0.0).x + origin.x;
            if line.words.iter().any(|word| word.tab.is_some()) {
                let size = cache
                    .style
                    .as_ref()
                    .map_or(240.0, |style| style.font_size_points * 20.0)
                    * scale;
                for bounds in leader_quads(line, scale, page_left, top.y, size) {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds,
                            ..renderer::Quad::default()
                        },
                        Color::BLACK,
                    );
                }
            }
            // Paragraph rules (Modify Style › Lines).
            for (bounds, colour) in rule_quads(line, scale, page_left, top.y) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        ..renderer::Quad::default()
                    },
                    colour,
                );
            }
        }
    }
}

/// Which lines of a page a hit may land on.
type LineFilter = Box<dyn Fn(&LineBox) -> bool>;

/// Draws with `draw` in a new layer when `layer`, so that it is above what
/// earlier layers hold (frames drawn in layers of their own), else in the
/// current one.
fn in_layer<Renderer: renderer::Renderer>(
    renderer: &mut Renderer,
    layer: bool,
    bounds: Rectangle,
    draw: impl FnOnce(&mut Renderer),
) {
    if layer {
        renderer.with_layer(bounds, draw);
    } else {
        draw(renderer);
    }
}

/// The width of `content` shaped at `size` in a line `height` tall.
fn preedit_width<Renderer: text::Renderer<Font = Font>>(
    content: &str,
    size: f32,
    height: f32,
) -> f32 {
    if content.is_empty() {
        return 0.0;
    }
    let paragraph = Renderer::Paragraph::with_text(text::Text {
        content,
        bounds: Size::new(1.0e6, height),
        size: Pixels(size),
        line_height: text::LineHeight::Absolute(Pixels(height)),
        font: Font::default(),
        align_x: text::Alignment::Default,
        align_y: cosmic::iced::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
        ellipsize: text::Ellipsize::None,
    });
    paragraph.min_bounds().width
}

/// The input method's composing text at the caret: the paper's white behind
/// it, the text, a thin underline under all of it and a thick one under the
/// part the input method has selected (`selection`, bytes of `content`).
fn draw_preedit<Renderer: text::Renderer<Font = Font>>(
    renderer: &mut Renderer,
    content: &str,
    selection: Option<std::ops::Range<usize>>,
    caret: Rectangle,
    clip: Rectangle,
) {
    let height = caret.height.max(1.0);
    let size = (height * 0.8).max(1.0);
    let width = preedit_width::<Renderer>(content, size, height).max(1.0);
    let area = Rectangle::new(caret.position(), Size::new(width, height));
    let paragraph = Renderer::Paragraph::with_text(text::Text {
        content,
        bounds: Size::new(width + 2.0, height),
        size: Pixels(size),
        line_height: text::LineHeight::Absolute(Pixels(height)),
        font: Font::default(),
        align_x: text::Alignment::Default,
        align_y: cosmic::iced::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
        ellipsize: text::Ellipsize::None,
    });
    renderer.with_layer(clip, |renderer| {
        let fill = |renderer: &mut Renderer, bounds: Rectangle, color: Color| {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    ..renderer::Quad::default()
                },
                color,
            );
        };
        fill(renderer, area, Color::WHITE);
        renderer.fill_paragraph(&paragraph, area.position(), Color::BLACK, clip);
        fill(
            renderer,
            Rectangle::new(
                Point::new(area.x, area.y + height - 1.0),
                Size::new(width, 1.0),
            ),
            Color::BLACK,
        );
        if let Some(range) = selection.filter(|range| range.start < range.end)
            && content.is_char_boundary(range.start)
            && content.is_char_boundary(range.end.min(content.len()))
        {
            let end = range.end.min(content.len());
            let before = preedit_width::<Renderer>(&content[..range.start], size, height);
            let through = preedit_width::<Renderer>(&content[..end], size, height);
            fill(
                renderer,
                Rectangle::new(
                    Point::new(area.x + before, area.y + height - 2.0),
                    Size::new((through - before).max(1.0), 2.0),
                ),
                Color::BLACK,
            );
        }
    });
}

/// A 1-pixel dashed outline: 4-pixel dashes with 3-pixel gaps.
fn draw_dashed<Renderer: renderer::Renderer>(renderer: &mut Renderer, outline: Rectangle) {
    const DASH: f32 = 4.0;
    const GAP: f32 = 3.0;
    let mut dash = |bounds: Rectangle| {
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                ..renderer::Quad::default()
            },
            ACCENT,
        );
    };
    let mut x = outline.x;
    while x < outline.x + outline.width {
        let width = DASH.min(outline.x + outline.width - x);
        dash(Rectangle::new(
            Point::new(x, outline.y),
            Size::new(width, 1.0),
        ));
        dash(Rectangle::new(
            Point::new(x, outline.y + outline.height - 1.0),
            Size::new(width, 1.0),
        ));
        x += DASH + GAP;
    }
    let mut y = outline.y;
    while y < outline.y + outline.height {
        let height = DASH.min(outline.y + outline.height - y);
        dash(Rectangle::new(
            Point::new(outline.x, y),
            Size::new(1.0, height),
        ));
        dash(Rectangle::new(
            Point::new(outline.x + outline.width - 1.0, y),
            Size::new(1.0, height),
        ));
        y += DASH + GAP;
    }
}

/// The keys a table takes from the caret in one of its cells (`1408:00ea`;
/// `UI_SPEC.md` §32).
fn table_key_action(key: &Key, modifiers: keyboard::Modifiers) -> Option<PageAction> {
    let Key::Named(named) = key else {
        return None;
    };
    match named {
        Named::Tab if !modifiers.command() && !modifiers.alt() => Some(PageAction::CellStep {
            forward: !modifiers.shift(),
        }),
        Named::ArrowUp | Named::ArrowDown if modifiers.alt() && !modifiers.command() => {
            Some(PageAction::MoveRows {
                down: *named == Named::ArrowDown,
            })
        }
        Named::ArrowUp | Named::ArrowDown | Named::ArrowLeft | Named::ArrowRight
            if modifiers.command() && !modifiers.alt() =>
        {
            let (down, right) = match named {
                Named::ArrowUp => (-1, 0),
                Named::ArrowDown => (1, 0),
                Named::ArrowLeft => (0, -1),
                _ => (0, 1),
            };
            Some(PageAction::CellMove {
                down,
                right,
                extend: modifiers.shift(),
            })
        }
        _ => None,
    }
}

fn key_action(key: &Key, modifiers: keyboard::Modifiers) -> Option<PageAction> {
    let extend = modifiers.shift();
    match key {
        // Alt+arrows are Format › Promote and Demote (`UI_SPEC.md` §22).
        Key::Named(Named::ArrowLeft) if !modifiers.alt() => Some(PageAction::MoveLeft { extend }),
        Key::Named(Named::ArrowRight) if !modifiers.alt() => Some(PageAction::MoveRight { extend }),
        Key::Named(Named::Backspace) => Some(PageAction::Backspace),
        Key::Named(Named::Delete) => Some(PageAction::Delete),
        Key::Named(Named::Enter) => Some(PageAction::ParagraphBreak),
        Key::Named(Named::Escape) => Some(PageAction::Escape),
        Key::Named(Named::Tab) if !modifiers.command() && !modifiers.alt() => {
            Some(PageAction::Insert("\t".into()))
        }
        _ => None,
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for PageSurface<'_, Message>
where
    Message: 'static,
    Renderer: text::Renderer<Font = Font> + image::Renderer<Handle = image::Handle>,
{
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(State::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(self.view.size.width),
            Length::Fixed(self.view.size.height),
        )
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        state.sync(
            self.document,
            self.layout,
            self.view.scale,
            self.session,
            self.printing.is_some_and(|mode| mode.preprinted_form),
            (self.selection.anchor.flow == self.selection.focus.flow)
                .then_some(self.selection.focus.flow),
        );
        layout::Node::new(self.view.size)
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
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let bounds = layout.bounds();
        if state.keyboard_seen != self.keyboard {
            state.keyboard_seen = self.keyboard;
            state.focused = true;
        }
        if state.viewport != viewport.size() {
            state.viewport = viewport.size();
            shell.publish((self.on_action)(PageAction::Viewport(viewport.size())));
        }
        if let Event::Mouse(mouse::Event::CursorMoved { .. }) = event {
            state.pointer = cursor.position_in(bounds);
        }
        match event {
            // A picture file dropped on the page goes in where it was
            // dropped (P7): the caret moves there first, and the file
            // follows (the drop destination's data).
            Event::Dnd(cosmic::iced::clipboard::dnd::DndEvent::Offer(
                _,
                cosmic::iced::clipboard::dnd::OfferEvent::Drop,
            )) => {
                if let Some(point) = state.pointer
                    && let Some((position, page)) = self.hit(state, point)
                {
                    shell.publish((self.on_action)(PageAction::MoveTo {
                        position,
                        page,
                        extend: false,
                    }));
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(bounds) {
                    state.focused = true;
                    let click = advanced::mouse::Click::new(
                        position,
                        mouse::Button::Left,
                        state.last_click,
                    );
                    state.last_click = Some(click);
                    // Draw mode: a press in the drawing's frame is the
                    // mode's; one outside ends it and then acts as usual
                    // (DW9).
                    if let (Some((at, inside)), Some(on_draw)) =
                        (self.draw_point(position), &self.on_draw)
                    {
                        if inside {
                            let keys = crate::draw_mode::Keys {
                                shift: state.modifiers.shift(),
                                ctrl: state.modifiers.control(),
                                alt: state.modifiers.alt(),
                            };
                            state.draw_press = true;
                            shell.publish(on_draw(crate::draw_mode::Event::Press {
                                at,
                                keys,
                                double: click.kind() == advanced::mouse::click::Kind::Double,
                            }));
                            shell.capture_event();
                            shell.request_redraw();
                            return;
                        }
                        shell.publish(on_draw(crate::draw_mode::Event::Leave));
                    }
                    if self.drawing {
                        // A press on a page starts the frame (frames there
                        // included, `1260:0000`); one outside every page
                        // ends drawing and then acts as usual.
                        let page_index = self.view.page_at(position);
                        if self
                            .view
                            .pages
                            .get(page_index)
                            .is_some_and(|paper| paper.contains(position))
                        {
                            let start = self.view.to_page(page_index, position);
                            state.frame_draw = Some(FrameDraw {
                                page_index,
                                start,
                                current: start,
                                press: position,
                                moved: false,
                            });
                            shell.capture_event();
                            shell.request_redraw();
                            return;
                        }
                        shell.publish((self.on_action)(PageAction::FrameDrawn(None)));
                    }
                    if self.press_frame(state, position, click, shell) {
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                    // A double-click on a note's mark opens the note (NR1).
                    if click.kind() == advanced::mouse::click::Kind::Double
                        && self.frames.is_none()
                        && let Some((at, _)) = self.hit(state, position)
                        && let Some(note) =
                            crate::reference_commands::note_beside(self.document, at)
                    {
                        shell.publish((self.on_action)(PageAction::OpenNote(note)));
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                    // A double-click on a footnote's reference goes to its
                    // text, and on a footnote's number back to its
                    // reference (NR8).
                    if click.kind() == advanced::mouse::click::Kind::Double
                        && self.frames.is_none()
                        && let Some((at, _)) = self.hit(state, position)
                        && let Some((position, page)) =
                            footnote_jump(self.document, self.layout, at)
                    {
                        shell.publish((self.on_action)(PageAction::MoveTo {
                            position,
                            page,
                            extend: false,
                        }));
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                    // A double-click on a formula cell is Table › Edit
                    // Formula… (`UI_SPEC.md` §41).
                    if click.kind() == advanced::mouse::click::Kind::Double
                        && self.frames.is_none()
                        && let Some((at, _)) = self.hit(state, position)
                        && let Some((frame, row, column)) =
                            crate::table_editing::cell_of(self.document, at.flow)
                        && crate::table_editing::table_of(self.document, frame).is_some_and(
                            |table| {
                                table
                                    .cells
                                    .get(row * table.columns.len().max(1) + column)
                                    .is_some_and(|cell| cell.formula.is_some())
                            },
                        )
                    {
                        shell.publish((self.on_action)(PageAction::EditFormula));
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                    if click.kind() == advanced::mouse::click::Kind::Double
                        && self.in_side_margin(state, position)
                    {
                        shell.publish((self.on_action)(PageAction::OpenPageLayout));
                        shell.capture_event();
                        return;
                    }
                    // A press in selected cells drags them (Ami Pro's Drag &
                    // Drop of cells, rows, and columns).
                    if self.frames.is_none()
                        && !state.modifiers.shift()
                        && let Some(selected) =
                            crate::table_editing::cell_selection(self.document, self.selection)
                        && let Some((at, _)) = self.hit(state, position)
                        && let Some((frame, row, column)) =
                            crate::table_editing::cell_of(self.document, at.flow)
                        && frame == selected.frame
                        && selected.range().contains(row, column)
                    {
                        state.cell_drag = Some(CellDrag {
                            frame,
                            press: position,
                            moved: false,
                            target: None,
                        });
                        shell.capture_event();
                        return;
                    }
                    if self.frames.is_none()
                        && let Some(mut line) = self.track_line_at(state, position)
                    {
                        let (x, y) = self.view.to_page(line.page_index, position);
                        line.press = Some(if line.column { x } else { y });
                        state.track_drag = Some(line);
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                    if self.frames.is_none()
                        && let Some((frame, anchor, focus)) = self.track_strip(state, position)
                    {
                        shell.publish((self.on_action)(PageAction::SelectCells {
                            frame,
                            anchor,
                            focus,
                        }));
                        shell.capture_event();
                        shell.request_redraw();
                        return;
                    }
                    state.dragging = true;
                    if let Some((position, page)) = self.hit(state, position) {
                        state.drag_hit = Some((position, page));
                        shell.publish((self.on_action)(PageAction::MoveTo {
                            position,
                            page,
                            extend: state.modifiers.shift(),
                        }));
                    }
                    shell.capture_event();
                    shell.request_redraw();
                } else if !shell.is_event_captured() {
                    // A press the ruler took leaves the caret where it is.
                    state.focused = false;
                    state.dragging = false;
                    state.drag_hit = None;
                }
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))
                if self.draw.is_some() =>
            {
                // Draw mode: the right button cancels a gesture (Ami Pro);
                // the right-click menu opens on the release.
                if let Some(position) = cursor.position_in(bounds)
                    && let (Some((_, true)), Some(on_draw)) =
                        (self.draw_point(position), &self.on_draw)
                {
                    state.draw_press = false;
                    shell.publish(on_draw(crate::draw_mode::Event::Cancel));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
                if state.draw_press || self.draw.as_ref().is_some_and(|draw| draw.tracking) =>
            {
                if let Some(position) = cursor.position()
                    && let Some(on_draw) = &self.on_draw
                {
                    let local = Point::new(position.x - bounds.x, position.y - bounds.y);
                    if let Some((at, _)) = self.draw_point(local) {
                        let keys = crate::draw_mode::Keys {
                            shift: state.modifiers.shift(),
                            ctrl: state.modifiers.control(),
                            alt: state.modifiers.alt(),
                        };
                        shell.publish(on_draw(crate::draw_mode::Event::Drag { at, keys }));
                        shell.request_redraw();
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) if state.draw_press => {
                state.draw_press = false;
                if let Some(position) = cursor.position()
                    && let Some(on_draw) = &self.on_draw
                {
                    let local = Point::new(position.x - bounds.x, position.y - bounds.y);
                    if let Some((at, _)) = self.draw_point(local) {
                        let keys = crate::draw_mode::Keys {
                            shift: state.modifiers.shift(),
                            ctrl: state.modifiers.control(),
                            alt: state.modifiers.alt(),
                        };
                        shell.publish(on_draw(crate::draw_mode::Event::Release { at, keys }));
                    }
                }
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                // Not captured: the right-click menu around the surface
                // opens on the release.
                if let Some(position) = cursor.position_in(bounds) {
                    state.focused = true;
                    self.press_right(state, position, shell);
                    shell.request_redraw();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.frame_draw.is_some() => {
                if let Some(position) = cursor.position()
                    && let Some(draw) = state.frame_draw.as_mut()
                {
                    let local = Point::new(position.x - bounds.x, position.y - bounds.y);
                    draw.current = self.on_paper(draw.page_index, local);
                    draw.moved |= local.distance(draw.press) > DRAG_THRESHOLD;
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.frame_draw.is_some() =>
            {
                if let Some(draw) = state.frame_draw.take() {
                    let drawn = draw.moved.then(|| self.drawn(draw)).flatten();
                    shell.publish((self.on_action)(PageAction::FrameDrawn(drawn)));
                }
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.cell_drag.is_some() => {
                if let Some(position) = cursor.position() {
                    let local = Point::new(position.x - bounds.x, position.y - bounds.y);
                    let target = self
                        .hit(state, local)
                        .and_then(|(at, _)| crate::table_editing::cell_of(self.document, at.flow));
                    if let Some(drag) = state.cell_drag.as_mut() {
                        drag.moved |= local.distance(drag.press) > DRAG_THRESHOLD;
                        drag.target = target
                            .filter(|(frame, ..)| *frame == drag.frame)
                            .map(|(_, row, column)| (row, column));
                    }
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.cell_drag.is_some() =>
            {
                if let Some(drag) = state.cell_drag.take() {
                    match (drag.moved, drag.target) {
                        (true, Some(to)) => {
                            shell.publish((self.on_action)(PageAction::DropCells {
                                frame: drag.frame,
                                to,
                                copy: state.modifiers.control(),
                            }))
                        }
                        // A click in the selection: the caret there.
                        _ => {
                            if let Some((position, page)) = self.hit(state, drag.press) {
                                shell.publish((self.on_action)(PageAction::MoveTo {
                                    position,
                                    page,
                                    extend: false,
                                }));
                            }
                        }
                    }
                }
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.track_drag.is_some() => {
                if let Some(position) = cursor.position()
                    && let Some(drag) = state.track_drag.as_mut()
                {
                    let local = Point::new(position.x - bounds.x, position.y - bounds.y);
                    let (x, y) = self.view.to_page(drag.page_index, local);
                    let pointer = if drag.column { x } else { y };
                    let wanted = drag.start + pointer - drag.press.unwrap_or(pointer);
                    drag.at = wanted.clamp(drag.limits.0, drag.limits.1.max(drag.limits.0));
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.track_drag.is_some() =>
            {
                if let Some(drag) = state.track_drag.take() {
                    let size = (drag.at - drag.origin).round() as amipro_document::Twips;
                    shell.publish((self.on_action)(PageAction::ResizeTrack {
                        frame: drag.frame,
                        column: drag.column,
                        index: drag.index,
                        size,
                    }));
                }
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.frame_drag.is_some() => {
                if let Some(position) = cursor.position()
                    && let Some(drag) = state.frame_drag.as_mut()
                {
                    let local = Point::new(
                        (position.x - bounds.x).clamp(0.0, bounds.width),
                        (position.y - bounds.y).clamp(0.0, bounds.height),
                    );
                    drag.current = self.view.to_page(drag.page_index, local);
                    drag.moved |= local.distance(drag.press) > DRAG_THRESHOLD;
                    shell.request_redraw();
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) if state.dragging => {
                if let Some(position) = cursor.position() {
                    let local = Point::new(
                        (position.x - bounds.x).clamp(0.0, bounds.width),
                        (position.y - bounds.y).clamp(0.0, bounds.height),
                    );
                    if let Some((position, page)) = self.hit(state, local)
                        && state.drag_hit != Some((position, page))
                    {
                        state.drag_hit = Some((position, page));
                        shell.publish((self.on_action)(PageAction::MoveTo {
                            position,
                            page,
                            extend: true,
                        }));
                        shell.request_redraw();
                    }
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
                if state.frame_drag.is_some() =>
            {
                self.release_frame(state, shell);
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging {
                    if self.fast_format {
                        shell.publish((self.on_action)(PageAction::PointerReleased));
                    }
                    shell.capture_event();
                    shell.request_redraw();
                }
                state.dragging = false;
                state.drag_hit = None;
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                state.modifiers = *modifiers;
                if state.frame_drag.is_some() {
                    // Shift keeps the aspect ratio of a resize.
                    shell.request_redraw();
                }
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
            Event::InputMethod(input_method::Event::Commit(content))
                if state.focused && !self.modal =>
            {
                state.preedit = None;
                shell.publish((self.on_action)(PageAction::Insert(content.clone())));
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
            // A key the ruler took (its keyboard mode) is not the text's.
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                physical_key,
                modifiers,
                text,
                ..
            }) if state.focused
                && !self.modal
                && !shell.is_event_captured()
                && !(self.arrows_elsewhere
                    && modifiers.is_empty()
                    && matches!(
                        key,
                        Key::Named(
                            Named::ArrowUp
                                | Named::ArrowDown
                                | Named::ArrowLeft
                                | Named::ArrowRight
                        )
                    )) =>
            {
                state.modifiers = *modifiers;
                // Draw mode takes the keys without Ctrl or Alt (and Alt
                // with the arrows); shortcuts go on to the application.
                if self.draw.is_some()
                    && let Some(on_draw) = &self.on_draw
                {
                    use crate::draw_mode::Key as DrawKey;
                    let keys = crate::draw_mode::Keys {
                        shift: modifiers.shift(),
                        ctrl: modifiers.control(),
                        alt: modifiers.alt(),
                    };
                    let named = match key {
                        Key::Named(Named::ArrowLeft) => Some(DrawKey::Left),
                        Key::Named(Named::ArrowRight) => Some(DrawKey::Right),
                        Key::Named(Named::ArrowUp) => Some(DrawKey::Up),
                        Key::Named(Named::ArrowDown) => Some(DrawKey::Down),
                        Key::Named(Named::Delete) => Some(DrawKey::Delete),
                        Key::Named(Named::Backspace) => Some(DrawKey::Backspace),
                        Key::Named(Named::Enter) => Some(DrawKey::Enter),
                        Key::Named(Named::Escape) => Some(DrawKey::Escape),
                        Key::Named(Named::Tab) => Some(DrawKey::Tab),
                        _ => None,
                    };
                    if !keys.ctrl {
                        if let Some(named) = named {
                            if !keys.alt
                                || matches!(
                                    named,
                                    DrawKey::Left | DrawKey::Right | DrawKey::Up | DrawKey::Down
                                )
                            {
                                shell.publish(on_draw(crate::draw_mode::Event::Key(named, keys)));
                                shell.capture_event();
                                shell.request_redraw();
                                return;
                            }
                        } else if !keys.alt {
                            if let Some(text) = text.as_ref().filter(|text| !text.is_empty())
                                && self.draw.as_ref().is_some_and(|draw| draw.typing)
                            {
                                shell.publish(on_draw(crate::draw_mode::Event::Text(
                                    text.to_string(),
                                )));
                            }
                            shell.capture_event();
                            return;
                        }
                    }
                    let _ = physical_key;
                }
                if (self.drawing || state.frame_draw.is_some())
                    && matches!(key, Key::Named(Named::Escape))
                {
                    // Esc ends drawing without a frame (FC11).
                    state.frame_draw = None;
                    shell.publish((self.on_action)(PageAction::FrameDrawn(None)));
                    shell.capture_event();
                    shell.request_redraw();
                    return;
                }
                if state.frame_draw.is_some() {
                    shell.capture_event();
                    return;
                }
                if state.frame_drag.is_some() {
                    // Esc cancels a drag; other keys wait for it to end.
                    if matches!(key, Key::Named(Named::Escape)) {
                        state.frame_drag = None;
                        shell.request_redraw();
                    }
                    shell.capture_event();
                    return;
                }
                // Selected frames keep their keys over a shortcut on the same
                // key (Alt+arrows resize a frame, and promote and demote
                // text; `UI_SPEC.md` §11, §22).
                if self.frames.is_some()
                    && let Some(on_frame) = &self.on_frame
                    && let Some(action) = if self.picture_mode {
                        picture_key_action(key, *modifiers)
                    } else {
                        frame_key_action(key, *modifiers)
                    }
                {
                    shell.publish(on_frame(action));
                    shell.capture_event();
                    return;
                }
                // In a table, Tab, Ctrl+arrows, and Alt+Up and Down are the
                // table's (`UI_SPEC.md` §32).
                if self.frames.is_none()
                    && crate::table_editing::caret_cell(self.document, self.selection).is_some()
                    && let Some(action) = table_key_action(key, *modifiers)
                {
                    shell.publish((self.on_action)(action));
                    shell.capture_event();
                    return;
                }
                let shortcut = command::command_for_shortcut(*modifiers, key, Some(physical_key))
                    .filter(|command| {
                        command::metadata(*command).implementation == Implementation::Implemented
                            && *command != CommandId::Delete
                    });
                if let Some(command) = shortcut {
                    shell.publish((self.on_command)(command));
                    shell.capture_event();
                    return;
                }
                if self.frames.is_some()
                    && let Some(on_frame) = &self.on_frame
                {
                    // Selected frames take the keys; typing does nothing.
                    if let Some(action) = frame_key_action(key, *modifiers) {
                        shell.publish(on_frame(action));
                        shell.capture_event();
                    }
                    return;
                }
                if let Key::Named(named) = key
                    && matches!(
                        named,
                        Named::ArrowUp
                            | Named::ArrowDown
                            | Named::Home
                            | Named::End
                            | Named::PageUp
                            | Named::PageDown
                    )
                {
                    if let Some((position, page)) = self.navigate(state, named, modifiers.command())
                    {
                        shell.publish((self.on_action)(PageAction::MoveTo {
                            position,
                            page,
                            extend: modifiers.shift(),
                        }));
                    }
                    shell.capture_event();
                    return;
                }
                if let Some(action) = key_action(key, *modifiers) {
                    shell.publish((self.on_action)(action));
                    shell.capture_event();
                } else if !modifiers.command()
                    && !modifiers.alt()
                    && let Some(value) = text
                        .as_ref()
                        .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
                {
                    shell.publish((self.on_action)(PageAction::Insert(value.to_string())));
                    shell.capture_event();
                }
            }
            Event::Window(cosmic::iced::window::Event::RedrawRequested(_))
                if state.focused && state.window_focused && self.frames.is_none() =>
            {
                let caret = self
                    .caret(state, self.selection.focus, self.selection.page)
                    .map_or(
                        Rectangle::new(bounds.position(), Size::new(1.0, 1.0)),
                        |(_, rectangle)| rectangle + (bounds.position() - Point::ORIGIN),
                    );
                shell.request_input_method(&InputMethod::Enabled {
                    cursor: caret,
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
        _defaults: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let bounds = layout.bounds();
        let origin = bounds.position() - Point::ORIGIN;
        let local_viewport = *viewport - origin;
        let printing = self.printing;
        let preprinted = printing.is_some_and(|mode| mode.preprinted_form);
        // Paper itself, for the PDF: nothing but what prints.
        let paper_only = printing.is_some_and(|mode| !mode.preview);
        // Selected cells are drawn as whole cells (`draw_frame`), not as
        // text.
        let selection = (!self.selection.is_caret()
            && self.frames.is_none()
            && !paper_only
            && crate::table_editing::cell_selection(self.document, self.selection).is_none())
        .then(|| self.selection.ordered());
        let visible: Vec<usize> = self.view.visible(local_viewport).collect();
        // A picture dragged in picture mode, and how far (P6).
        let pan_preview = state
            .frame_drag
            .filter(|drag| drag.kind == DragKind::Pan && drag.moved)
            .and_then(|drag| {
                let frame = self.frames?.single(self.document)?;
                Some((frame, drag.delta()))
            });
        // Whether a page drew frames in layers of their own.
        let mut layered = false;
        for page_index in visible {
            let Some(cached) = state.pages.get(page_index) else {
                continue;
            };
            let page = &cached.page;
            let paper = self.view.pages[page_index] + origin;
            // Paper is white and has no shadow or shaded margins (Ami Pro
            // draws them only without a print device, `1108:02a3`); the
            // preview keeps the shadow, on white paper.
            if printing.is_some_and(|mode| mode.preview) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: paper + Vector::new(4.0, 4.0),
                        ..renderer::Quad::default()
                    },
                    PAGE_SHADOW,
                );
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: paper,
                        ..renderer::Quad::default()
                    },
                    Color::WHITE,
                );
            }
            if printing.is_none() {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: paper + Vector::new(4.0, 4.0),
                        ..renderer::Quad::default()
                    },
                    PAGE_SHADOW,
                );
                // Shaded margins around the white text area (user decision,
                // 2026-10-18), so header and footer text stand on the shade;
                // View › Show ▸ Margins in Colour off leaves them white.
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: paper,
                        ..renderer::Quad::default()
                    },
                    if self.view_marks.margins_in_colour {
                        MARGIN_SHADE
                    } else {
                        Color::WHITE
                    },
                );
                if let Some(area) = text_area(page) {
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: self.view.rect(page_index, area) + origin,
                            ..renderer::Quad::default()
                        },
                        Color::WHITE,
                    );
                }
                if self.view_marks.column_guides {
                    for column in &page.columns {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: self.view.rect(page_index, *column) + origin,
                                border: Border {
                                    color: COLUMN_GUIDE,
                                    width: 1.0,
                                    radius: 0.0.into(),
                                },
                                ..renderer::Quad::default()
                            },
                            Color::TRANSPARENT,
                        );
                    }
                }
            }
            // The page layout's lines, around the page and between its
            // columns, under everything else on the page; each stroke on
            // whole pixels and at least one wide. Not on a preprinted form.
            let strokes: &[_] = if preprinted { &[] } else { &page.strokes };
            for (bounds, colour) in stroke_quads(&self.view, page_index, strokes, origin) {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        ..renderer::Quad::default()
                    },
                    colour,
                );
            }
            // The line above the footnotes, one device pixel thick with the
            // default pen (`1178:09f8`; VM RNF1, RNF4).
            if let Some(separator) = page.footnote_separator {
                let start = self.view.point(page_index, separator.x, separator.y) + origin;
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle::new(
                            Point::new(start.x.round(), start.y.round()),
                            Size::new((separator.width * self.view.scale).max(1.0), 1.0),
                        ),
                        ..renderer::Quad::default()
                    },
                    Color::BLACK,
                );
            }
            // Layout frames (repeating, header, footer) and their text, then
            // the main text, then page frames in stack order with their text
            // (V14).
            for frame_box in page
                .frames
                .iter()
                .filter(|frame| frame.layer == Layer::Layout)
            {
                self.draw_frame(
                    renderer,
                    origin,
                    page_index,
                    frame_box,
                    self.document.frame(frame_box.frame),
                    &page.lines,
                    *viewport,
                    pan_preview,
                );
            }
            let layout_frames: Vec<_> = page
                .frames
                .iter()
                .filter(|frame| frame.layer == Layer::Layout)
                .map(|frame| frame.frame)
                .collect();
            self.draw_lines(
                renderer,
                state,
                origin,
                page_index,
                |line| match line.container {
                    Container::Floating(_) => true,
                    Container::Frame(id) | Container::Cell { frame: id, .. } => {
                        layout_frames.contains(&id)
                    }
                    Container::Main | Container::Footnote(_) => false,
                },
                selection,
                *viewport,
            );
            self.draw_lines(
                renderer,
                state,
                origin,
                page_index,
                |line| matches!(line.container, Container::Main | Container::Footnote(_)),
                selection,
                *viewport,
            );
            // Outline mode's icons left of each main paragraph's first line
            // (Ami Pro's grey plus, minus, and box; VM OM1).
            if let Some(icons) = self.outline {
                for line in page
                    .lines
                    .iter()
                    .filter(|line| line.container == Container::Main && line.bytes.start == 0)
                {
                    let Some(icon) = icons.get(line.paragraph_index) else {
                        continue;
                    };
                    // A hidden paragraph's empty line shows nothing.
                    if line.height < 2.0 {
                        continue;
                    }
                    let left = line.words.first().map_or(line.x, |word| word.x);
                    let size = 150.0;
                    let centre_y = line.top + (line.baseline - line.top) * 0.62;
                    let corner =
                        self.view
                            .point(page_index, left - size - 110.0, centre_y - size / 2.0)
                            + origin;
                    for (bounds, colour) in
                        outline_icon_quads(*icon, corner, size * self.view.scale)
                    {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds,
                                ..renderer::Quad::default()
                            },
                            colour,
                        );
                    }
                }
            }
            // Page › Line Numbering, in the left margin.
            if let Some(numbers) = state.numbers.get(page_index) {
                for (x, top, paragraph) in &numbers.texts {
                    let position = self.view.point(page_index, *x, *top) + origin;
                    renderer.fill_paragraph(paragraph, position, Color::BLACK, *viewport);
                }
            }
            // Each page frame in a layer of its own, with its text: within
            // a layer the renderer draws every quad before any text, so a
            // frame's fill could not hide the text behind it, as an opaque
            // frame does in Ami Pro (user's guide ch. 15). What follows on
            // the page goes in a layer above them.
            let mut page_frames = page
                .frames
                .iter()
                .filter(|frame| frame.layer == Layer::Page)
                .peekable();
            let framed = page_frames.peek().is_some();
            layered |= framed;
            for frame_box in page_frames {
                renderer.with_layer(*viewport, |renderer| {
                    self.draw_frame(
                        renderer,
                        origin,
                        page_index,
                        frame_box,
                        self.document.frame(frame_box.frame),
                        &page.lines,
                        *viewport,
                        pan_preview,
                    );
                    let id = frame_box.frame;
                    self.draw_lines(
                        renderer,
                        state,
                        origin,
                        page_index,
                        |line| match line.container {
                            Container::Frame(frame) | Container::Cell { frame, .. } => frame == id,
                            _ => false,
                        },
                        selection,
                        *viewport,
                    );
                });
            }
            if paper_only {
                continue;
            }
            in_layer(renderer, framed, *viewport, |renderer| {
                // The paper's edge over everything, so frames that reach
                // the page's edge never hide it.
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: paper,
                        border: Border {
                            color: PAGE_EDGE,
                            width: 1.0,
                            radius: 0.0.into(),
                        },
                        ..renderer::Quad::default()
                    },
                    Background::Color(Color::TRANSPARENT),
                );
                if let Some((page, x)) = self.guide
                    && page == page_index
                {
                    let x = (paper.x + x * self.view.scale).round();
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle::new(
                                Point::new(x, paper.y),
                                Size::new(1.0, paper.height),
                            ),
                            ..renderer::Quad::default()
                        },
                        ACCENT,
                    );
                }
            });
        }
        if paper_only {
            return;
        }
        // The frame being drawn, the frame selection, and the caret above
        // every frame.
        in_layer(renderer, layered, *viewport, |renderer| {
            if let Some(draw) = state.frame_draw.filter(|draw| draw.moved)
                && let Some(drawn) = self.drawn_outline(draw)
            {
                draw_dashed(renderer, self.view.rect(draw.page_index, drawn) + origin);
            }
            // Cells being dragged: the cell they would go to.
            if let Some(CellDrag {
                frame,
                moved: true,
                target: Some((row, column)),
                ..
            }) = state.cell_drag
                && let Some(FrameContent::Table(table)) =
                    self.document.frame(frame).map(|holder| &holder.content)
            {
                let index = row * table.columns.len() + column;
                for page_index in self.view.visible(local_viewport) {
                    let Some(cached) = state.pages.get(page_index) else {
                        continue;
                    };
                    for frame_box in cached
                        .page
                        .frames
                        .iter()
                        .filter(|frame_box| frame_box.frame == frame)
                    {
                        if let FrameBoxKind::Table { cells, part } = &frame_box.kind
                            && let Some(text) = cells.get(index).filter(|text| text.width > 0.0)
                        {
                            let boxed = amipro_layout::cell_box(table, part, index, *text);
                            draw_dashed(renderer, self.view.rect(page_index, boxed) + origin);
                        }
                    }
                }
            }
            // A table grid line being dragged: its new place across the
            // table, as Ami Pro draws the outline while dragging.
            if let Some(drag) = state.track_drag
                && let Some(table_box) = state.pages.get(drag.page_index).and_then(|cached| {
                    cached
                        .page
                        .frames
                        .iter()
                        .find(|frame_box| frame_box.frame == drag.frame)
                })
            {
                let bounds = table_box.bounds;
                let line = if drag.column {
                    amipro_layout::Bounds {
                        x: drag.at,
                        y: bounds.y,
                        width: 0.0,
                        height: bounds.height,
                    }
                } else {
                    amipro_layout::Bounds {
                        x: bounds.x,
                        y: drag.at,
                        width: bounds.width,
                        height: 0.0,
                    }
                };
                let mut rect = self.view.rect(drag.page_index, line) + origin;
                rect.width = rect.width.max(1.0);
                rect.height = rect.height.max(1.0);
                draw_dashed(renderer, rect);
            }
            if self.draw.is_some() {
                self.draw_draw_mode(renderer, origin);
                return;
            }
            if let Some(frames) = self.frames {
                for page_index in self.view.visible(local_viewport) {
                    self.draw_frame_selection(renderer, state, origin, page_index, frames);
                }
                return;
            }
            if state.focused
                && state.window_focused
                && self.selection.is_caret()
                && let Some((_, caret)) =
                    self.caret(state, self.selection.focus, self.selection.page)
            {
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: caret + origin,
                        ..renderer::Quad::default()
                    },
                    Color::BLACK,
                );
                // The input method's composing text stands at the caret,
                // underlined, on the paper (CP.1).
                if let Some(preedit) = state.preedit.as_ref().filter(|p| !p.content.is_empty()) {
                    let (_, caret) = self
                        .caret(state, self.selection.focus, self.selection.page)
                        .expect("the caret was just found");
                    draw_preedit(
                        renderer,
                        &preedit.content,
                        preedit.selection.clone(),
                        caret + origin,
                        *viewport,
                    );
                }
            }
            // A cell holding a picture is outlined while the caret is in it,
            // as Ami Pro outlines it (`1488:14f0`; T1).
            if self.selection.is_caret()
                && let Some(frame) =
                    crate::page_editing::cell_frame(self.document, self.selection.focus.flow)
            {
                for page_index in self.view.visible(local_viewport) {
                    let Some(frame_box) = state.pages.get(page_index).and_then(|cached| {
                        cached
                            .page
                            .frames
                            .iter()
                            .find(|frame_box| frame_box.frame == frame)
                    }) else {
                        continue;
                    };
                    for band in cell_outline(frame_box.bounds) {
                        renderer.fill_quad(
                            renderer::Quad {
                                bounds: self.view.rect(page_index, band) + origin,
                                ..renderer::Quad::default()
                            },
                            ACCENT,
                        );
                    }
                }
            }
        });
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        if state.frame_draw.is_some() {
            return mouse::Interaction::Crosshair;
        }
        // Draw mode: each tool's pointer over the drawing's frame.
        if let Some(draw) = &self.draw
            && let Some(position) = cursor.position_in(layout.bounds())
            && let Some((_, true)) = self.draw_point(position)
        {
            use crate::draw_mode::Tool;
            return match draw.tool {
                Tool::Select => mouse::Interaction::Idle,
                Tool::Hand => mouse::Interaction::Grab,
                Tool::Text => mouse::Interaction::Text,
                _ => mouse::Interaction::Crosshair,
            };
        }
        if self.drawing
            && let Some(position) = cursor.position_in(layout.bounds())
            && self
                .view
                .pages
                .get(self.view.page_at(position))
                .is_some_and(|paper| paper.contains(position))
        {
            return mouse::Interaction::Crosshair;
        }
        if let Some(drag) = state.frame_drag {
            return match drag.kind {
                DragKind::Move | DragKind::Pan => mouse::Interaction::Grabbing,
                DragKind::Resize(handle) => resize_cursor(handle),
            };
        }
        if let Some(drag) = state.track_drag {
            return if drag.column {
                mouse::Interaction::ResizingHorizontally
            } else {
                mouse::Interaction::ResizingVertically
            };
        }
        let Some(position) = cursor.position_in(layout.bounds()) else {
            return mouse::Interaction::None;
        };
        if self.frames.is_none()
            && let Some(line) = self.track_line_at(state, position)
        {
            return if line.column {
                mouse::Interaction::ResizingHorizontally
            } else {
                mouse::Interaction::ResizingVertically
            };
        }
        if self.frames.is_some() && !state.pages.is_empty() {
            let page_index = self.view.page_at(position).min(state.pages.len() - 1);
            // Picture mode: a hand over the picture's frame (P6).
            if self.picture_mode && self.in_selection(state, page_index, position) {
                return mouse::Interaction::Grab;
            }
            if let Some(handle) = self.handle_at(state, page_index, position) {
                return resize_cursor(handle);
            }
            if self.in_selection(state, page_index, position) {
                return mouse::Interaction::Move;
            }
        }
        mouse::Interaction::Text
    }
}

impl<'a, Message, Theme, Renderer> From<PageSurface<'a, Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'a,
    Renderer: text::Renderer<Font = Font> + image::Renderer<Handle = image::Handle> + 'a,
{
    fn from(surface: PageSurface<'a, Message>) -> Self {
        Element::new(surface)
    }
}

/// The top of `page`'s main text, for the status bar's page arrows and Go
/// To: the first main line on that page, or, for a page without main text,
/// on the nearest page in the direction moved (`forward`). With the page it
/// landed on.
pub fn page_top(
    document: &Document,
    layout: &DocumentLayout,
    page: u32,
    forward: bool,
) -> Option<(TextPosition, u32)> {
    let main = document.main_flow_id();
    let count = layout.pages.len() as u32;
    let first_line = |number: u32| {
        layout
            .pages
            .get(number.checked_sub(1)? as usize)?
            .lines
            .iter()
            .filter(|line| line.container == Container::Main)
            .min_by(|a, b| a.top.total_cmp(&b.top))
            .map(|line| TextPosition {
                flow: main,
                paragraph: line.paragraph_index,
                byte: line.bytes.start,
            })
    };
    let page = page.clamp(1, count.max(1));
    let order: Vec<u32> = if forward {
        (page..=count).chain((1..page).rev()).collect()
    } else {
        (1..=page).rev().chain(page + 1..=count).collect()
    };
    order
        .into_iter()
        .find_map(|number| first_line(number).map(|position| (position, number)))
}

/// The page holding the caret, from the layout, for the status bar and the
/// ruler: the page instance for flows on several pages.
/// A frame's lines: all four rounding its corners as one border, else each
/// side's (`UI_SPEC.md` §11).
fn draw_frame_lines<Renderer: renderer::Renderer>(
    renderer: &mut Renderer,
    bounds: Rectangle,
    style: &amipro_document::FrameStyle,
    radius: f32,
    width: f32,
) {
    let border = style.border;
    let colour = colour(border.colour);
    let mut line = |rectangle: Rectangle| {
        renderer.fill_quad(
            renderer::Quad {
                bounds: rectangle,
                ..renderer::Quad::default()
            },
            colour,
        );
    };
    if border.style > 0 && radius > 0.0 && border.sides == amipro_document::Sides::ALL {
        // All four lines round the corners as one border.
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border: Border {
                    color: colour,
                    width,
                    radius: radius.into(),
                },
                ..renderer::Quad::default()
            },
            Background::Color(Color::TRANSPARENT),
        );
    } else if border.style > 0 {
        if border.sides.top {
            line(Rectangle::new(
                bounds.position(),
                Size::new(bounds.width, width),
            ));
        }
        if border.sides.bottom {
            line(Rectangle::new(
                Point::new(bounds.x, bounds.y + bounds.height - width),
                Size::new(bounds.width, width),
            ));
        }
        if border.sides.left {
            line(Rectangle::new(
                bounds.position(),
                Size::new(width, bounds.height),
            ));
        }
        if border.sides.right {
            line(Rectangle::new(
                Point::new(bounds.x + bounds.width - width, bounds.y),
                Size::new(width, bounds.height),
            ));
        }
    }
}

/// How many device pixels a vector picture is drawn with per logical pixel,
/// so that it stays sharp on high-density screens.
const PICTURE_OVERSAMPLE: f32 = 2.0;
/// Pictures drawn from vectors print at 300 dots an inch when they cannot
/// be handed to the PDF as vectors.
const PRINT_DPI: f32 = 300.0;

/// A picture's area in its frame, page twips: the frame's interior as laid
/// out, inside its margins and corner inset (`1530:0892` mode 2 is the text
/// area within a few twips; PI1, P02), or the frame where that leaves
/// nothing.
pub(crate) fn picture_area(frame_box: &FrameBox) -> amipro_layout::Bounds {
    let area = frame_box.interior;
    if area.width > 0.0 && area.height > 0.0 {
        area
    } else {
        frame_box.bounds
    }
}

/// The outline of a cell holding a picture (Ami Pro's, `1488:14f0`): four
/// bands 43 twips wide around the cell's text area, which the picture
/// covers.
pub fn cell_outline(area: amipro_layout::Bounds) -> [amipro_layout::Bounds; 4] {
    use amipro_layout::Bounds;
    const BAND: f32 = 43.0;
    let outer = Bounds {
        x: area.x - BAND,
        y: area.y - BAND,
        width: area.width + 2.0 * BAND,
        height: area.height + 2.0 * BAND,
    };
    [
        Bounds {
            height: BAND,
            ..outer
        },
        Bounds {
            y: area.bottom(),
            height: BAND,
            ..outer
        },
        Bounds {
            width: BAND,
            y: area.y,
            height: area.height,
            ..outer
        },
        Bounds {
            x: area.right(),
            y: area.y,
            width: BAND,
            height: area.height,
        },
    ]
}

/// What the status bar says with the caret in a cell holding a picture:
/// typing does nothing there, as in Ami Pro, and Delete removes the
/// picture (T1).
pub fn cell_frame_note(document: &Document, selection: PageSelection) -> Option<&'static str> {
    let frame = crate::page_editing::cell_frame(document, selection.focus.flow)?;
    Some(match document.frame(frame)?.content {
        FrameContent::Picture(_) => "This cell holds a picture; Delete removes it",
        _ => "This cell holds a frame; Delete removes it",
    })
}

pub fn caret_page(document: &Document, layout: &DocumentLayout, selection: PageSelection) -> u32 {
    let focus = selection.focus;
    if focus.flow != document.main_flow_id() {
        return selection.page;
    }
    // Where the caret is drawn (`line_of`): the line holding the character
    // after it, else the line it ends; so after a page break it is on the
    // next page, as Ami Pro shows it (VM fixture IL1).
    let lines = || {
        layout.pages.iter().flat_map(|page| {
            page.lines
                .iter()
                .filter(|line| {
                    line.container == Container::Main && line.paragraph_index == focus.paragraph
                })
                .map(move |line| (page.number, line))
        })
    };
    lines()
        .find(|(_, line)| line.bytes.start <= focus.byte && focus.byte < line.bytes.end)
        .or_else(|| lines().find(|(_, line)| focus.byte == line.bytes.end || line.bytes.is_empty()))
        .map_or(1, |(number, _)| number)
}

/// A page's text area: its main text's columns and the gaps between them,
/// which Ami Pro leaves white when it shades the margins (G05).
fn text_area(page: &PageBox) -> Option<amipro_layout::Bounds> {
    let first = page.columns.first()?;
    let (mut left, mut top, mut right, mut bottom) =
        (first.x, first.y, first.right(), first.bottom());
    for column in &page.columns[1..] {
        left = left.min(column.x);
        top = top.min(column.y);
        right = right.max(column.right());
        bottom = bottom.max(column.bottom());
    }
    Some(amipro_layout::Bounds {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

/// The status bar's note while the caret is in text that does not fit its
/// box: Ami Pro hides such text without a word (V14; VM, HDR1 and HDR2),
/// and the caret then stands still at the end of the last visible line, so
/// typing seems to stop. Main text always finds room on a new page.
pub fn hidden_text_note(
    document: &Document,
    layout: &DocumentLayout,
    selection: PageSelection,
) -> Option<&'static str> {
    let focus = selection.focus;
    if focus.flow == document.main_flow_id() {
        return None;
    }
    let page = layout
        .pages
        .iter()
        .find(|page| page.number == selection.page)?;
    let shown = page.lines.iter().any(|line| {
        line.flow == focus.flow
            && line.paragraph_index == focus.paragraph
            && line.bytes.start <= focus.byte
            && focus.byte <= line.bytes.end
    });
    if shown {
        return None;
    }
    let owner = document.frames().find_map(|frame| match &frame.content {
        FrameContent::Text(flow) if *flow == focus.flow => Some(Some(&frame.placement)),
        FrameContent::Table(table)
            if table.cells.iter().any(|cell| {
                matches!(cell.content, amipro_document::CellContent::Text(flow) if flow == focus.flow)
            }) =>
        {
            Some(None)
        }
        _ => None,
    });
    Some(match owner {
        Some(Some(amipro_document::Placement::Header { .. })) => {
            "This text doesn't fit in the header, so it's hidden; a larger top margin makes room"
        }
        Some(Some(amipro_document::Placement::Footer { .. })) => {
            "This text doesn't fit in the footer, so it's hidden; a larger bottom margin makes room"
        }
        Some(Some(_)) => {
            "This text doesn't fit in the frame, so it's hidden; a larger frame shows it"
        }
        Some(None) => "This text doesn't fit in the table cell, so it's hidden",
        None => "This text doesn't fit, so it's hidden",
    })
}

/// The last of `lines` in `position`'s flow that ends at or before it: the
/// line a caret in hidden text (text that does not fit its box, V14) is
/// shown at the end of.
fn shown_before(lines: &[LineBox], position: TextPosition) -> Option<usize> {
    lines.iter().rposition(|line| {
        line.flow == position.flow
            && (line.paragraph_index, line.bytes.end) <= (position.paragraph, position.byte)
    })
}

/// The rectangle (workspace pixels) of the line holding `position`, without
/// shaped geometry, for scrolling the caret into view.
pub fn caret_line_rect(
    document: &Document,
    layout: &DocumentLayout,
    view: &PageView,
    selection: PageSelection,
) -> Option<Rectangle> {
    let focus = selection.focus;
    let page = caret_page(document, layout, selection);
    let index = layout
        .pages
        .iter()
        .position(|candidate| candidate.number == page)?;
    let lines = &layout.pages[index].lines;
    let line = lines
        .iter()
        .find(|line| {
            line.flow == focus.flow
                && line.paragraph_index == focus.paragraph
                && line.bytes.start <= focus.byte
                && focus.byte <= line.bytes.end
        })
        .or_else(|| shown_before(lines, focus).map(|index| &lines[index]))?;
    Some(view.rect(
        index,
        amipro_layout::Bounds {
            x: line.x,
            y: line.top,
            width: line.width.max(1.0),
            height: line.height,
        },
    ))
}

/// Where Esc from frame, header, footer, or cell text puts the caret: at the
/// frame's anchor in the main text, or at the start of the main text on the
/// caret's page.
/// Where a double-click at `at` jumps: from a footnote's reference (the
/// caret just before or after it) to the start of its text, and from the
/// start of a footnote's text (its number) to its reference; with the page.
pub fn footnote_jump(
    document: &Document,
    layout: &DocumentLayout,
    at: TextPosition,
) -> Option<(TextPosition, u32)> {
    let object_len = amipro_document::OBJECT_REPLACEMENT_CHARACTER.len_utf8();
    if at.flow == document.main_flow_id() {
        let paragraph = document.paragraphs().get(at.paragraph)?;
        let flow = paragraph
            .object_offsets()
            .zip(&paragraph.objects)
            .find_map(|(byte, object)| match object {
                InlineObject::Footnote { flow }
                    if byte == at.byte || byte + object_len == at.byte =>
                {
                    Some(*flow)
                }
                _ => None,
            })?;
        let page = layout.pages.iter().find(|page| {
            page.lines
                .iter()
                .any(|line| line.container == Container::Footnote(flow))
        })?;
        return Some((
            TextPosition {
                flow,
                paragraph: 0,
                byte: 0,
            },
            page.number,
        ));
    }
    if at.paragraph != 0 || at.byte != 0 {
        return None;
    }
    let reference = footnote_reference(document, at.flow)?;
    let page = layout
        .page_of(reference.paragraph, reference.byte)
        .unwrap_or(1);
    Some((reference, page))
}

/// Where the reference to the footnote whose text is `flow` stands in the
/// main text.
pub fn footnote_reference(document: &Document, flow: FlowId) -> Option<TextPosition> {
    document
        .paragraphs()
        .iter()
        .enumerate()
        .find_map(|(index, paragraph)| {
            paragraph
                .object_offsets()
                .zip(&paragraph.objects)
                .find(|(_, object)| {
                    matches!(object, InlineObject::Footnote { flow: owned } if *owned == flow)
                })
                .map(|(byte, _)| TextPosition {
                    flow: document.main_flow_id(),
                    paragraph: index,
                    byte,
                })
        })
}

pub fn escape_target(
    document: &Document,
    layout: &DocumentLayout,
    selection: PageSelection,
) -> PageSelection {
    let main = document.main_flow_id();
    if selection.flow() == main {
        return selection;
    }
    let flow = selection.flow();
    // A footnote's text: back to the text after its reference, as Ami Pro's
    // Esc and Return to Text (NR8).
    if let Some(position) = footnote_reference(document, flow) {
        let after = TextPosition {
            byte: position.byte + amipro_document::OBJECT_REPLACEMENT_CHARACTER.len_utf8(),
            ..position
        };
        let page = layout
            .page_of(position.paragraph, position.byte)
            .unwrap_or(selection.page);
        return PageSelection::caret(after, page);
    }
    let owner = document.frames().find(|frame| match &frame.content {
        FrameContent::Text(text) => *text == flow,
        FrameContent::Table(table) => table
            .cells
            .iter()
            .any(|cell| matches!(cell.content, amipro_document::CellContent::Text(cell_flow) if cell_flow == flow)),
        _ => false,
    });
    if let Some(frame) = owner {
        for (index, paragraph) in document.paragraphs().iter().enumerate() {
            if let Some(byte) = paragraph
                .object_offsets()
                .zip(&paragraph.objects)
                .find(|(_, object)| matches!(object, InlineObject::FrameAnchor(id) if *id == frame.id))
                .map(|(byte, _)| byte)
            {
                let position = TextPosition {
                    flow: main,
                    paragraph: index,
                    byte,
                };
                let page = layout.page_of(index, byte).unwrap_or(selection.page);
                return PageSelection::caret(position, page);
            }
        }
    }
    let first_on_page = layout
        .pages
        .iter()
        .find(|page| page.number == selection.page)
        .and_then(|page| {
            page.lines
                .iter()
                .find(|line| line.container == Container::Main)
        });
    match first_on_page {
        Some(line) => PageSelection::caret(
            TextPosition {
                flow: main,
                paragraph: line.paragraph_index,
                byte: line.bytes.start,
            },
            selection.page,
        ),
        None => PageSelection::start(document),
    }
}

/// A document whose page table (Flow with text on a line of its own, as Ami
/// Pro stores them) of 2 × 60 one-line cells, row 0 a heading, starts after
/// 30 lines and continues on page 2 (`UI_SPEC.md` §26): the document, the
/// table, and the cells' flows in order.
#[cfg(test)]
pub(crate) fn long_table_fixture() -> (
    Document,
    amipro_document::FrameId,
    Vec<amipro_document::FlowId>,
) {
    use amipro_document::{
        CellContent, CellMargins, CellSpan, LegacyData, Placement, Table, TableCell, TableRows,
    };
    let mut lines: Vec<String> = (1..=30).map(|line| format!("Line {line}")).collect();
    lines.push(String::new());
    lines.push("After the table.".into());
    let mut document = Document::from_plain_text(&lines.join("\n"));
    let mut cells = Vec::new();
    let mut flows = Vec::new();
    for row in 0..60 {
        for column in 0..2 {
            let flow = document.create_flow(amipro_document::StyleId(1));
            document
                .flow_mut(flow)
                .unwrap()
                .paragraph_mut(0)
                .unwrap()
                .text = format!("R{row}C{column}");
            flows.push(flow);
            cells.push(TableCell {
                content: CellContent::Text(flow),
                span: CellSpan::SINGLE,
                frame: None,
                formula: None,
                format: Default::default(),
                legacy: LegacyData::default(),
            });
        }
    }
    let mut row_options = TableRows::default();
    row_options.headings.insert(0);
    let table = document.allocate_frame_id();
    document.insert_frame(amipro_document::Frame {
        id: table,
        placement: Placement::Inline {
            y_offset: 0,
            size: amipro_document::Size {
                width: 6_400,
                height: 700,
            },
        },
        style: amipro_document::FrameStyle {
            interior: amipro_document::LayoutBox::single_column(amipro_document::Margins::uniform(
                0,
            )),
            wrap: amipro_document::Wrap::Around,
            // Page tables have no frame lines (TB1).
            border: amipro_document::Border {
                sides: amipro_document::Sides::NONE,
                ..amipro_document::FrameStyle::default().border
            },
            ..amipro_document::FrameStyle::default()
        },
        group: None,
        content: FrameContent::Table(Table {
            columns: vec![3_000, 3_000],
            rows: vec![284; 60],
            cell_margins: CellMargins::default(),
            spacing: Default::default(),
            row_options,
            layout: amipro_document::TableLayout::inferred(&[284; 60], &[3_000, 3_000]),
            read_rows: Default::default(),
            cells,
            legacy: LegacyData::default(),
        }),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().push(table);
    let anchor = document.paragraph_mut(30);
    anchor.text = amipro_document::OBJECT_REPLACEMENT_CHARACTER.to_string();
    anchor.objects = vec![InlineObject::FrameAnchor(table)];
    (document, table, flows)
}

/// A document with a Where placed 2 × 1 table whose second cell holds a
/// picture (`UI_SPEC.md` §24): the document, the table, the picture, and
/// the two cells' flows.
#[cfg(test)]
pub(crate) fn cell_picture_fixture() -> (
    Document,
    amipro_document::FrameId,
    amipro_document::FrameId,
    [amipro_document::FlowId; 2],
) {
    use amipro_document::{
        CellContent, CellMargins, CellSpan, LegacyData, Picture, PictureSource, Placement, Table,
        TableCell,
    };
    let mut document = Document::from_plain_text("Main text");
    let cells: Vec<TableCell> = (0..2)
        .map(|_| TableCell {
            content: CellContent::Text(document.create_flow(amipro_document::StyleId(1))),
            span: CellSpan::SINGLE,
            frame: None,
            formula: None,
            format: Default::default(),
            legacy: LegacyData::default(),
        })
        .collect();
    let flows = [0, 1].map(|index| {
        let CellContent::Text(flow) = cells[index].content;
        flow
    });
    let table = document.allocate_frame_id();
    document.insert_frame(amipro_document::Frame {
        id: table,
        placement: Placement::Fixed {
            page: 1,
            rect: amipro_document::Rect::new(1_440, 4_000, 7_669, 4_413),
        },
        style: amipro_document::FrameStyle {
            interior: amipro_document::LayoutBox::single_column(amipro_document::Margins::uniform(
                0,
            )),
            ..amipro_document::FrameStyle::default()
        },
        group: None,
        content: FrameContent::Table(Table {
            columns: vec![3_017, 3_017],
            rows: vec![284],
            cell_margins: CellMargins::default(),
            spacing: Default::default(),
            row_options: Default::default(),
            layout: Default::default(),
            read_rows: Default::default(),
            cells,
            legacy: LegacyData::default(),
        }),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().push(table);
    let object = document.add_object(std::sync::Arc::from(&b"BM cell picture"[..]));
    let picture = document.allocate_frame_id();
    document.insert_frame(amipro_document::Frame {
        id: picture,
        placement: Placement::InCell,
        style: amipro_document::FrameStyle {
            border: amipro_document::Border {
                sides: amipro_document::Sides::NONE,
                ..amipro_document::FrameStyle::default().border
            },
            interior: amipro_document::LayoutBox::single_column(amipro_document::Margins::uniform(
                0,
            )),
            ..amipro_document::FrameStyle::default()
        },
        group: None,
        content: FrameContent::Picture(Picture::new(
            PictureSource::Embedded(object),
            ".bmp",
            amipro_document::Size {
                width: 600,
                height: 400,
            },
        )),
        legacy: LegacyData::default(),
    });
    document.frame_stack_mut().insert(0, picture);
    let FrameContent::Table(content) = &mut document.frame_mut(table).unwrap().content else {
        unreachable!()
    };
    content.cells[1].frame = Some(picture);
    document.validate().unwrap();
    (document, table, picture, flows)
}

/// A document for page-surface tests: several pages of UTF-8 text with a
/// 24 pt run, header text, a text frame on page 1, and a table.
#[cfg(test)]
pub(crate) fn page_fixture() -> Document {
    use amipro_document::{CharacterFormat, CharacterRun, FrameStyle, Placement, Rect, StyleId};
    let text: Vec<String> = (0..90)
        .map(|index| {
            format!(
                "Paragraph {index} café naïve 東京 with enough words to wrap onto a second line of a Letter page in Times New Roman."
            )
        })
        .collect();
    let mut document = Document::from_plain_text(&text.join("\n"));
    document.paragraph_mut(1).runs.push(CharacterRun {
        start: 0,
        end: 9,
        format: CharacterFormat {
            font_size_points: Some(24.0),
            ..CharacterFormat::default()
        },
    });
    let header = document
        .document_layout()
        .header
        .right
        .expect("a header frame");
    let FrameContent::Text(header_flow) = document.frame(header).expect("header").content else {
        unreachable!("header frames hold text")
    };
    document
        .flow_mut(header_flow)
        .unwrap()
        .paragraph_mut(0)
        .unwrap()
        .text = "Header ü".into();
    let frame = document.create_text_frame(
        Placement::Fixed {
            page: 1,
            rect: Rect::new(6_480, 7_200, 10_080, 9_360),
        },
        FrameStyle::default(),
    );
    document.frame_stack_mut().push(frame);
    let FrameContent::Text(frame_flow) = document.frame(frame).unwrap().content else {
        unreachable!()
    };
    document
        .flow_mut(frame_flow)
        .unwrap()
        .paragraph_mut(0)
        .unwrap()
        .text = "Frame é text".into();
    let _ = StyleId(1);
    document.validate().unwrap();
    document
}

#[cfg(test)]
mod tests {
    use amipro_document::{FrameId, Placement};
    use cosmic::iced::advanced::Renderer as _;

    use super::*;
    use crate::page_layout::SessionPages;
    use crate::page_view::Zoom;

    #[derive(Clone, Debug, PartialEq)]
    enum Sent {
        Page(PageAction),
        Command(CommandId),
        Frame(FrameAction),
    }

    type Renderer = cosmic::Renderer;

    fn renderer() -> Renderer {
        cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(Font::default(), Pixels(14.0)))
    }

    struct Fixture {
        document: Document,
        pages: SessionPages,
    }

    impl Fixture {
        fn new(document: Document) -> Self {
            let mut pages = SessionPages::default();
            pages.update(&document);
            Self { document, pages }
        }

        fn view(&self, zoom: Zoom) -> PageView {
            PageView::new(self.pages.layout(), zoom, Size::new(900.0, 700.0))
        }

        fn surface(&self, view: PageView, selection: PageSelection) -> PageSurface<'_, Sent> {
            page_surface(
                &self.document,
                self.pages.layout(),
                view,
                7,
                selection,
                Sent::Page,
                Sent::Command,
            )
        }

        /// The surface with frame interaction (`UI_SPEC.md` §11).
        fn framed<'a>(
            &'a self,
            view: PageView,
            selection: PageSelection,
            frames: Option<&'a FrameSelection>,
        ) -> PageSurface<'a, Sent> {
            self.surface(view, selection).frames(frames, Sent::Frame)
        }

        fn frame_flow(&self, frame: FrameId) -> FlowId {
            let FrameContent::Text(flow) = self.document.frame(frame).unwrap().content else {
                panic!("a text frame")
            };
            flow
        }

        /// The workspace point (pixels) of a line's middle, `x` twips in.
        fn point(&self, view: &PageView, page: usize, line: &LineBox, x: f32) -> Point {
            view.point(page, x, line.top + line.height / 2.0)
        }
    }

    struct Driver {
        renderer: Renderer,
        tree: widget::Tree,
        node: layout::Node,
        /// Where the widget sits on screen, as if scrolled.
        offset: Vector,
    }

    impl Driver {
        fn new(surface: &mut PageSurface<'_, Sent>, offset: Vector) -> Self {
            let renderer = renderer();
            let mut tree = widget::Tree {
                tag: Widget::<Sent, cosmic::Theme, Renderer>::tag(surface),
                id: None,
                state: Widget::<Sent, cosmic::Theme, Renderer>::state(surface),
                children: Vec::new(),
            };
            let node = Widget::<Sent, cosmic::Theme, Renderer>::layout(
                surface,
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, f32::INFINITY)),
            )
            .move_to(Point::ORIGIN + offset);
            Self {
                renderer,
                tree,
                node,
                offset,
            }
        }

        fn relayout(&mut self, surface: &mut PageSurface<'_, Sent>) {
            self.node = Widget::<Sent, cosmic::Theme, Renderer>::layout(
                surface,
                &mut self.tree,
                &self.renderer,
                &layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, f32::INFINITY)),
            )
            .move_to(Point::ORIGIN + self.offset);
        }

        fn state(&self) -> &State<<Renderer as text::Renderer>::Paragraph> {
            self.tree
                .state
                .downcast_ref::<State<<Renderer as text::Renderer>::Paragraph>>()
        }

        /// Sends `event` with the pointer at `point` (workspace pixels).
        fn send(
            &mut self,
            surface: &mut PageSurface<'_, Sent>,
            event: Event,
            point: Point,
        ) -> Vec<Sent> {
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            let cursor = mouse::Cursor::Available(point + self.offset);
            let viewport = Rectangle::new(Point::ORIGIN, Size::new(900.0, 700.0));
            Widget::<Sent, cosmic::Theme, Renderer>::update(
                surface,
                &mut self.tree,
                &event,
                Layout::new(&self.node),
                cursor,
                &self.renderer,
                &mut cosmic::iced::advanced::clipboard::Null,
                &mut shell,
                &viewport,
            );
            drop(shell);
            messages
                .into_iter()
                .filter(|message| !matches!(message, Sent::Page(PageAction::Viewport(_))))
                .collect()
        }

        fn draw(&mut self, surface: &PageSurface<'_, Sent>) {
            let viewport = Rectangle::new(Point::ORIGIN, Size::new(900.0, 700.0));
            self.renderer.reset(viewport);
            Widget::<Sent, cosmic::Theme, Renderer>::draw(
                surface,
                &self.tree,
                &mut self.renderer,
                &cosmic::Theme::default(),
                &renderer::Style::default(),
                Layout::new(&self.node),
                mouse::Cursor::Unavailable,
                &viewport,
            );
        }
    }

    fn press() -> Event {
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
    }

    fn release() -> Event {
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left))
    }

    fn moved(point: Point) -> Event {
        Event::Mouse(mouse::Event::CursorMoved { position: point })
    }

    fn key(named: Named, modifiers: keyboard::Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Named(named),
            modified_key: Key::Named(named),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }

    fn typed(text: &str) -> Event {
        let key = Key::Character(text.into());
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::default(),
            text: Some(text.into()),
            repeat: false,
        })
    }

    fn main_lines(layout: &DocumentLayout, page: usize) -> Vec<&LineBox> {
        layout.pages[page]
            .lines
            .iter()
            .filter(|line| line.container == Container::Main)
            .collect()
    }

    fn move_to(message: &Sent) -> (TextPosition, u32, bool) {
        match message {
            Sent::Page(PageAction::MoveTo {
                position,
                page,
                extend,
            }) => (*position, *page, *extend),
            other => panic!("expected MoveTo, got {other:?}"),
        }
    }

    #[test]
    fn press_drag_and_release_select_main_text_through_the_widget() {
        let fixture = Fixture::new(page_fixture());
        let view = fixture.view(Zoom::Percent(100));
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view.clone(), selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let layout = fixture.pages.layout();
        let lines = main_lines(layout, 0);
        let start = fixture.point(&view, 0, lines[0], lines[0].x + 1.0);
        let sent = driver.send(&mut surface, press(), start);
        let (position, page, extend) = move_to(&sent[0]);
        assert_eq!(
            (position.paragraph, position.byte, page, extend),
            (0, 0, 1, false)
        );
        assert_eq!(position.flow, fixture.document.main_flow_id());

        // Drag to the middle of the fourth line, then release.
        let target = fixture.point(&view, 0, lines[3], lines[3].x + 2_000.0);
        let sent = driver.send(
            &mut surface,
            Event::Mouse(mouse::Event::CursorMoved { position: target }),
            target,
        );
        let (position, _, extend) = move_to(&sent[0]);
        assert!(extend, "dragging extends the selection");
        assert_eq!(position.paragraph, lines[3].paragraph_index);
        assert!(
            lines[3].bytes.contains(&position.byte),
            "{position:?} on {:?}",
            lines[3].bytes
        );
        let released = driver.send(
            &mut surface,
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            target,
        );
        assert!(released.is_empty());
        assert!(!driver.state().dragging);

        // The caret drawn for a hit position is where the hit was.
        let state = driver.state();
        let (_, caret) = surface.caret(state, position, 1).unwrap();
        assert!(
            (caret.x - target.x).abs() < 12.0,
            "caret {caret:?} for a click at {target:?}"
        );
        assert!(caret.y <= target.y && target.y <= caret.y + caret.height);
        driver.draw(&surface);
    }

    /// A paragraph that holds only a With para above anchor takes no height
    /// (MERCURY.SAM): it keeps a visible caret, arrows pass through it, and
    /// clicks on the text beside its frame land in that text.
    #[test]
    fn anchor_only_paragraphs_keep_a_caret_but_take_no_clicks() {
        let mut document = Document::from_plain_text("Subhead\n\nText beside the icon");
        let frame = document.create_text_frame(
            Placement::BelowParagraph {
                x_offset: 0,
                size: amipro_document::Size {
                    width: 576,
                    height: 576,
                },
            },
            amipro_document::FrameStyle {
                wrap: amipro_document::Wrap::NoWrapAround,
                ..amipro_document::FrameStyle::default()
            },
        );
        document.frame_stack_mut().push(frame);
        let paragraph = document.paragraph_mut(1);
        paragraph
            .text
            .push(amipro_document::OBJECT_REPLACEMENT_CHARACTER);
        paragraph.objects.push(InlineObject::FrameAnchor(frame));
        let fixture = Fixture::new(document);
        let layout = fixture.pages.layout();
        let lines = main_lines(layout, 0);
        assert_eq!(lines[1].height, 0.0);

        let view = fixture.view(Zoom::Percent(100));
        let main = fixture.document.main_flow_id();
        let on_anchor = TextPosition {
            flow: main,
            paragraph: 1,
            byte: 0,
        };
        let mut surface = fixture.surface(view.clone(), PageSelection::caret(on_anchor, 1));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.draw(&surface);
        let (_, caret) = surface.caret(driver.state(), on_anchor, 1).unwrap();
        assert!(caret.height > 10.0, "a visible caret: {caret:?}");

        // A click on the text beside the frame lands in that text.
        let beside = fixture.point(&view, 0, lines[2], lines[2].x + 2_000.0);
        let (position, _, _) = move_to(&driver.send(&mut surface, press(), beside)[0]);
        assert_eq!(position.paragraph, 2);

        // Up from there stops on the anchor paragraph, then the subhead.
        let mut surface = fixture.surface(view.clone(), PageSelection::caret(position, 1));
        let up = key(Named::ArrowUp, keyboard::Modifiers::default());
        let (position, _, _) = move_to(&driver.send(&mut surface, up.clone(), beside)[0]);
        assert_eq!(position.paragraph, 1);
        let mut surface = fixture.surface(view, PageSelection::caret(position, 1));
        let (position, _, _) = move_to(&driver.send(&mut surface, up, beside)[0]);
        assert_eq!(position.paragraph, 0);
    }

    /// While the Outline navigator holds the keys (OB.6) the page leaves the
    /// plain arrows to the application; with Shift or without the flag it
    /// moves the caret as before (the live check: after a click on a row
    /// the arrows moved the page's caret instead of the tree's selection).
    #[test]
    fn the_page_leaves_plain_arrows_to_the_navigator_that_holds_them() {
        let document = Document::from_plain_text("One\nTwo\nThree");
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let main = fixture.document.main_flow_id();
        let at = TextPosition {
            flow: main,
            paragraph: 1,
            byte: 0,
        };
        let lines = main_lines(fixture.pages.layout(), 0);
        let point = fixture.point(&view, 0, lines[1], lines[1].x + 2.0);
        let down = key(Named::ArrowDown, keyboard::Modifiers::default());

        let mut surface = fixture.surface(view.clone(), PageSelection::caret(at, 1));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.draw(&surface);
        driver.send(&mut surface, press(), point);
        assert!(
            !driver.send(&mut surface, down.clone(), point).is_empty(),
            "the page moves the caret on Down"
        );

        let mut surface = fixture
            .surface(view.clone(), PageSelection::caret(at, 1))
            .arrows_elsewhere(true);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.draw(&surface);
        driver.send(&mut surface, press(), point);
        assert!(
            driver.send(&mut surface, down, point).is_empty(),
            "the navigator has the plain arrows"
        );
        let shifted = key(Named::ArrowDown, keyboard::Modifiers::SHIFT);
        assert!(
            !driver.send(&mut surface, shifted, point).is_empty(),
            "Shift+Down still selects on the page"
        );
    }

    /// A joined cell (README31.SAM's merge-field table) is one cell on the
    /// page: a click over the cell it covers lands in its text.
    #[test]
    fn clicks_over_a_covered_cell_land_in_the_joined_cell() {
        use amipro_document::{CellContent, CellMargins, CellSpan, LegacyData, Table, TableCell};

        let mut document = Document::from_plain_text("Main text");
        let cells: Vec<TableCell> = [CellSpan::SINGLE, CellSpan::SINGLE]
            .into_iter()
            .chain([
                CellSpan {
                    rows: 1,
                    columns: 2,
                },
                CellSpan::COVERED,
            ])
            .map(|span| TableCell {
                content: CellContent::Text(document.create_flow(amipro_document::StyleId(1))),
                span,
                frame: None,
                formula: None,
                format: Default::default(),
                legacy: LegacyData::default(),
            })
            .collect();
        let CellContent::Text(joined) = cells[2].content;
        document
            .flow_mut(joined)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "Joined".into();
        let frame = document.allocate_frame_id();
        document.insert_frame(amipro_document::Frame {
            id: frame,
            placement: Placement::Fixed {
                page: 1,
                rect: amipro_document::Rect::new(1_440, 4_000, 6_440, 5_000),
            },
            style: amipro_document::FrameStyle::default(),
            group: None,
            content: FrameContent::Table(Table {
                columns: vec![2_000, 2_000],
                rows: vec![240, 240],
                cell_margins: CellMargins::default(),
                spacing: Default::default(),
                row_options: Default::default(),
                layout: Default::default(),
                read_rows: Default::default(),
                cells,
                legacy: LegacyData::default(),
            }),
            legacy: LegacyData::default(),
        });
        document.frame_stack_mut().push(frame);
        let fixture = Fixture::new(document);
        let layout = fixture.pages.layout();
        let table = layout.pages[0]
            .frames
            .iter()
            .find(|frame_box| frame_box.frame == frame)
            .unwrap();
        let FrameBoxKind::Table { cells, .. } = &table.kind else {
            panic!("a table box");
        };
        assert_eq!(cells[3], cells[2]);

        let view = fixture.view(Zoom::Percent(100));
        let mut surface = fixture.surface(view.clone(), PageSelection::start(&fixture.document));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let covered_half = view.point(
            0,
            cells[2].right() - 300.0,
            cells[2].y + cells[2].height / 2.0,
        );
        let (position, _, _) = move_to(&driver.send(&mut surface, press(), covered_half)[0]);
        assert_eq!(position.flow, joined);
    }

    /// T1 (`UI_SPEC.md` §24, VM TF5): a press on a cell's picture puts the
    /// caret in its cell; the picture is never selected as a frame, and the
    /// cell's outline is Ami Pro's 43-twip bands around its text area.
    #[test]
    fn a_click_on_a_cells_picture_puts_the_caret_in_the_cell() {
        let (document, table, picture, flows) = cell_picture_fixture();
        let fixture = Fixture::new(document);
        let layout = fixture.pages.layout();
        let frames = &layout.pages[0].frames;
        let at = frames
            .iter()
            .position(|frame_box| frame_box.frame == table)
            .unwrap();
        assert_eq!(frames[at + 1].frame, picture);
        let bounds = frames[at + 1].bounds;
        assert!(!frame_editing::selectable(&fixture.document, picture));

        let view = fixture.view(Zoom::Percent(100));
        let mut surface = fixture.surface(view.clone(), PageSelection::start(&fixture.document));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let middle = view.point(
            0,
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        );
        let sent = driver.send(&mut surface, press(), middle);
        let (position, _, _) = move_to(&sent[0]);
        assert_eq!(position.flow, flows[1], "the caret in the picture's cell");
        assert!(
            sent.iter()
                .all(|message| !matches!(message, Sent::Frame(_))),
            "{sent:?}"
        );

        let selection = PageSelection::caret(position, 1);
        assert_eq!(
            cell_frame_note(&fixture.document, selection),
            Some("This cell holds a picture; Delete removes it")
        );
        assert_eq!(
            cell_frame_note(&fixture.document, PageSelection::start(&fixture.document)),
            None
        );
        let [top, bottom, left, right] = cell_outline(bounds);
        assert_eq!(
            (top.y, top.height, top.width),
            (bounds.y - 43.0, 43.0, bounds.width + 86.0)
        );
        assert_eq!(bottom.y, bounds.bottom());
        assert_eq!((left.x, left.width), (bounds.x - 43.0, 43.0));
        assert_eq!(right.x, bounds.right());
        // Drawn with the caret there.
        let surface = fixture.surface(view, selection);
        driver.draw(&surface);
    }

    /// TD.5: a numeric cell shows its number through the style; a click
    /// puts the caret at the end of the stored text, and with the caret in
    /// the cell the stored number shows (right-aligned cells keep their
    /// right edge); other cells and text cells are unchanged.
    #[test]
    fn a_numeric_cell_shows_through_its_style_until_the_caret_is_in_it_td5() {
        let (mut document, table, _, flows) = cell_picture_fixture();
        let style = document.flow(flows[0]).unwrap().paragraphs[0].style;
        document
            .styles_mut()
            .get_mut(&style)
            .unwrap()
            .kept
            .number_format = amipro_document::NumberFormat {
            flags: 0x198,
            kind: amipro_document::NumberKind::Currency,
            ..Default::default()
        };
        document
            .flow_mut(flows[0])
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "1234.5".into();
        document
            .flow_mut(flows[1])
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "-7.25".into();
        document
            .flow_mut(flows[1])
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .format
            .alignment = Some(amipro_document::Alignment::Right);
        let _ = table;
        let fixture = Fixture::new(document);
        let cell_line = |flow: FlowId| {
            fixture.pages.layout().pages[0]
                .lines
                .iter()
                .find(|line| line.flow == flow)
                .cloned()
                .unwrap()
        };
        let (first, second) = (cell_line(flows[0]), cell_line(flows[1]));
        assert_eq!(first.words[0].text.as_deref(), Some("$1,234.50"));
        assert_eq!(second.words[0].text.as_deref(), Some("-$7.25"));
        assert!(second.words[0].red && !first.words[0].red);

        let view = fixture.view(Zoom::Percent(100));
        let mut surface = fixture.surface(view.clone(), PageSelection::start(&fixture.document));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let segments = |driver: &Driver, flow: FlowId| {
            let state = driver.state();
            state
                .pages
                .iter()
                .flat_map(|page| page.lines.iter())
                .find(|line| {
                    line.segments
                        .first()
                        .is_some_and(|segment| segment.bytes.start == 0)
                        && line.source.as_ref().is_some_and(|source| {
                            fixture
                                .document
                                .flow(flow)
                                .unwrap()
                                .paragraphs
                                .iter()
                                .any(|paragraph| paragraph.id == source.id)
                        })
                })
                .cloned()
                .unwrap()
        };
        let shown = segments(&driver, flows[0]);
        assert!(shown.segments[0].field && shown.segments[0].number);
        // A press on the left half of the formatted number still lands at
        // the end of the stored text.
        let point = fixture.point(&view, 0, &first, first.words[0].x + 10.0);
        let sent = driver.send(&mut surface, press(), point);
        let (position, _, _) = move_to(&sent[0]);
        assert_eq!((position.flow, position.byte), (flows[0], 6));

        // With the caret there the stored text shows, plain.
        let selection = PageSelection::caret(position, 1);
        let mut surface = fixture.surface(view.clone(), selection);
        driver.relayout(&mut surface);
        let plain = segments(&driver, flows[0]);
        assert!(!plain.segments[0].field);
        assert_eq!(plain.segments[0].bytes, 0..6);
        // The other cell is still formatted.
        assert!(segments(&driver, flows[1]).segments[0].field);
        // A right-aligned cell keeps its right edge when shown as stored.
        let right = PageSelection::caret(
            TextPosition {
                flow: flows[1],
                paragraph: 0,
                byte: 5,
            },
            1,
        );
        let mut surface = fixture.surface(view.clone(), right);
        driver.relayout(&mut surface);
        let stored = segments(&driver, flows[1]);
        let width = stored.segments[0].paragraph.min_bounds().width / view.scale;
        let word = &second.words[0];
        assert!(
            (stored.segments[0].x + width - (word.x + word.width)).abs() < 0.5,
            "{} {width} {word:?}",
            stored.segments[0].x
        );
        // And back to formatted when the caret leaves.
        let mut surface = fixture.surface(view, PageSelection::start(&fixture.document));
        driver.relayout(&mut surface);
        assert!(segments(&driver, flows[1]).segments[0].field);
    }

    /// §26: a table continued on page 2 takes clicks there, its repeated
    /// heading included (TP4: the caret goes into the heading row on that
    /// page), and its cells of other parts have no box on a page.
    #[test]
    fn clicks_in_a_continued_table_land_in_its_cells_on_their_page() {
        let (document, table, flows) = long_table_fixture();
        let fixture = Fixture::new(document);
        let layout = fixture.pages.layout();
        let part = |page: usize| {
            layout.pages[page]
                .frames
                .iter()
                .find(|frame_box| frame_box.frame == table)
                .unwrap()
        };
        let (first, second) = (part(0), part(1));
        let (
            FrameBoxKind::Table {
                part: one,
                cells: first_cells,
            },
            FrameBoxKind::Table { part: two, cells },
        ) = (&first.kind, &second.kind)
        else {
            panic!("table boxes");
        };
        assert_eq!(one.rows.start, 0);
        assert_eq!(two.rows.start, one.rows.end);
        assert_eq!(two.heading, Some(0..1));
        // The first row's cells are on page 1 and repeated on page 2; a row
        // of page 2 has no box on page 1.
        let row = two.rows.start + 2;
        assert_eq!(first_cells[row * 2].width, 0.0);
        assert!(cells[row * 2].width > 0.0);

        let view = fixture.view(Zoom::Percent(100));
        let mut surface = fixture.surface(view.clone(), PageSelection::start(&fixture.document));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.draw(&surface);
        let middle = |bounds: amipro_layout::Bounds| {
            view.point(1, bounds.x + 200.0, bounds.y + bounds.height / 2.0)
        };
        let sent = driver.send(&mut surface, press(), middle(cells[row * 2 + 1]));
        let (position, page, _) = move_to(&sent[0]);
        assert_eq!((position.flow, page), (flows[row * 2 + 1], 2));
        // The repeated heading on page 2: the heading row's cell, on page 2.
        let sent = driver.send(&mut surface, press(), middle(cells[0]));
        let (position, page, _) = move_to(&sent[0]);
        assert_eq!((position.flow, page), (flows[0], 2));
        let caret = PageSelection::caret(position, 2);
        let surface = fixture.surface(view.clone(), caret);
        driver.draw(&surface);
        let (page_index, rectangle) = surface.caret(driver.state(), position, 2).unwrap();
        assert_eq!(page_index, 1, "the caret on page 2's copy");
        let top = view.point(1, cells[0].x, cells[0].y);
        assert!(rectangle.y >= top.y, "{rectangle:?} {top:?}");
        // On page 1 the same cell shows the caret there.
        let surface = fixture.surface(view, PageSelection::caret(position, 1));
        driver.draw(&surface);
        let (page_index, _) = surface.caret(driver.state(), position, 1).unwrap();
        assert_eq!(page_index, 0);
    }

    /// Spaces that end a line hang past its measure without word boxes; the
    /// caret still moves past each one as it is typed (the user's physical
    /// check: the caret stood still at the space bar, in a blank document
    /// and in MERCURY.SAM).
    /// Text that does not fit its box is hidden (V14), but the caret in it
    /// stays at the end of the box's last visible line, as Ami Pro shows it
    /// (VM, HDR1 and HDR2: a third header line typed into a band that holds
    /// two). The user's report, 2026-10-18: after Enter in a header the caret
    /// vanished and typing seemed to stop.
    #[test]
    fn the_caret_in_hidden_text_stays_at_the_end_of_the_last_visible_line() {
        let document = Document::default();
        let header = document.document_layout().header.right.unwrap();
        let FrameContent::Text(header_flow) = document.frame(header).unwrap().content else {
            panic!("a text header")
        };
        let start = TextPosition {
            flow: header_flow,
            paragraph: 0,
            byte: 0,
        };
        let applied = amipro_editing::apply(
            &document,
            &amipro_editing::Command::ReplaceText {
                start,
                end: start,
                text: "abc\ndef\nghi".into(),
            },
        )
        .unwrap();
        let fixture = Fixture::new(applied.document);
        let layout = fixture.pages.layout();
        let shown: Vec<_> = layout.pages[0]
            .lines
            .iter()
            .filter(|line| line.flow == header_flow)
            .map(|line| line.paragraph_index)
            .collect();
        assert_eq!(shown, [0, 1], "the band holds two lines, as in Ami Pro");
        let view = fixture.view(Zoom::Percent(100));
        let at = |paragraph: usize, byte: usize| TextPosition {
            flow: header_flow,
            paragraph,
            byte,
        };
        let hidden = at(2, 3);
        let band = view.point(0, 2_000.0, 900.0);
        let mut surface = fixture.surface(view, PageSelection::caret(hidden, 1));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.draw(&surface);
        let (page, caret) = surface
            .caret(driver.state(), hidden, 1)
            .expect("a caret in hidden text");
        let (_, end_of_shown) = surface.caret(driver.state(), at(1, 3), 1).unwrap();
        assert_eq!((page, caret), (0, end_of_shown));
        // The status bar says why, where Ami Pro says nothing.
        let note = |position| {
            hidden_text_note(&fixture.document, layout, PageSelection::caret(position, 1))
        };
        assert!(note(hidden).is_some_and(|note| note.contains("header")));
        assert!(note(at(2, 0)).is_some());
        assert_eq!(note(at(1, 3)), None);
        assert_eq!(note(at(0, 0)), None);
        assert_eq!(
            hidden_text_note(
                &fixture.document,
                layout,
                PageSelection::start(&fixture.document)
            ),
            None
        );
        // Up, Home, and End go to the line the caret is shown on; there is
        // nothing visible below it.
        driver.send(&mut surface, press(), band);
        driver.send(&mut surface, release(), band);
        let mut navigate = |named: Named| match driver
            .send(
                &mut surface,
                key(named, keyboard::Modifiers::default()),
                Point::ORIGIN,
            )
            .as_slice()
        {
            [Sent::Page(PageAction::MoveTo { position, .. })] => Some(*position),
            _ => None,
        };
        assert_eq!(navigate(Named::ArrowUp), Some(at(1, 3)));
        assert_eq!(navigate(Named::Home), Some(at(1, 0)));
        assert_eq!(navigate(Named::End), Some(at(1, 3)));
        assert_eq!(navigate(Named::ArrowDown), None);
        // The ruler shows the header's band, not the page.
        let ruler =
            crate::ruler::Ruler::at(&fixture.document, layout, PageSelection::caret(hidden, 1))
                .unwrap();
        assert!(
            matches!(ruler.target, crate::ruler::RulerBox::Band { .. }),
            "{:?}",
            ruler.target
        );
    }

    /// The margins are shaded and the text area is white, the gap between
    /// columns included, as Ami Pro's Layout mode shows a page (G05).
    #[test]
    fn the_text_area_is_the_columns_with_their_gaps() {
        let plain = Fixture::new(Document::default());
        assert_eq!(
            text_area(&plain.pages.layout().pages[0]),
            Some(amipro_layout::Bounds {
                x: 1_440.0,
                y: 1_440.0,
                width: 9_360.0,
                height: 12_960.0
            })
        );
        let mut document = Document::default();
        let geometry = &mut document.document_layout_mut().right;
        geometry.layout.margins.left = 2_160;
        geometry.layout.columns = 2;
        geometry.layout.gutter = 1_440;
        geometry.layout.tabs = vec![Vec::new(), Vec::new()];
        let columns = Fixture::new(document);
        let page = &columns.pages.layout().pages[0];
        assert_eq!(page.columns.len(), 2);
        assert_eq!(
            text_area(page),
            Some(amipro_layout::Bounds {
                x: 2_160.0,
                y: 1_440.0,
                width: 8_640.0,
                height: 12_960.0
            })
        );
    }

    #[test]
    fn the_caret_moves_past_trailing_spaces() {
        for text in ["a  ", "   ", "word word  "] {
            let fixture = Fixture::new(Document::from_plain_text(text));
            let view = fixture.view(Zoom::Percent(100));
            let main = fixture.document.main_flow_id();
            let mut surface = fixture.surface(view, PageSelection::start(&fixture.document));
            let mut driver = Driver::new(&mut surface, Vector::ZERO);
            driver.draw(&surface);
            let x = |byte: usize| {
                let position = TextPosition {
                    flow: main,
                    paragraph: 0,
                    byte,
                };
                surface.caret(driver.state(), position, 1).unwrap().1.x
            };
            let first_space = text.find(' ').unwrap();
            for byte in first_space..text.len() {
                assert!(
                    x(byte + 1) > x(byte) + 2.0,
                    "{text:?}: the caret after byte {byte} stays at {}",
                    x(byte)
                );
            }
        }
    }

    #[test]
    fn arrows_cross_pages_and_clicks_enter_frames_headers_and_cells() {
        let mut document = page_fixture();
        // A 2 × 1 table frame on page 1.
        let mut command_document = document.clone();
        let applied = amipro_editing::apply(
            &command_document,
            &amipro_editing::Command::CreateFrame {
                placement: Placement::Fixed {
                    page: 1,
                    rect: amipro_document::Rect::new(1_440, 11_520, 5_760, 12_960),
                },
                style: amipro_document::FrameStyle::default(),
                content: amipro_editing::NewFrameContent::Table {
                    columns: vec![2_160, 2_160],
                    rows: vec![720],
                },
                anchor: None,
                layout: None,
            },
        )
        .unwrap();
        command_document = applied.document;
        document = command_document;
        let fixture = Fixture::new(document);
        let layout = fixture.pages.layout();
        assert!(layout.pages.len() >= 3, "{} pages", layout.pages.len());
        let view = fixture.view(Zoom::Percent(100));
        let main = fixture.document.main_flow_id();

        // ArrowDown on page 1's last main line goes to page 2's first.
        let last = *main_lines(layout, 0).last().unwrap();
        let selection = PageSelection::caret(
            TextPosition {
                flow: main,
                paragraph: last.paragraph_index,
                byte: last.bytes.start,
            },
            1,
        );
        let mut surface = fixture.surface(view.clone(), selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.send(
            &mut surface,
            press(),
            fixture.point(&view, 0, last, last.x + 1.0),
        );
        let sent = driver.send(
            &mut surface,
            key(Named::ArrowDown, keyboard::Modifiers::default()),
            Point::ORIGIN,
        );
        let (position, page, _) = move_to(&sent[0]);
        let first = main_lines(layout, 1)[0];
        assert_eq!(page, 2);
        assert_eq!(position.paragraph, first.paragraph_index);
        assert!(first.bytes.start <= position.byte && position.byte <= first.bytes.end);

        // Into the text frame, the header of page 2, and the table's second cell.
        let frame_box = layout.pages[0]
            .frames
            .iter()
            .find(|frame| matches!(frame.kind, FrameBoxKind::Text) && frame.layer == Layer::Page)
            .unwrap();
        let frame_flow = fixture.frame_flow(frame_box.frame);
        let inside = view.point(0, frame_box.interior.x + 10.0, frame_box.interior.y + 100.0);
        let sent = driver.send(&mut surface, press(), inside);
        let (position, page, _) = move_to(&sent[0]);
        assert_eq!(
            (position.flow, position.paragraph, page),
            (frame_flow, 0, 1)
        );

        let header = fixture.document.document_layout().header.right.unwrap();
        let header_flow = fixture.frame_flow(header);
        let header_line = layout.pages[1]
            .lines
            .iter()
            .find(|line| line.flow == header_flow)
            .unwrap();
        let sent = driver.send(
            &mut surface,
            press(),
            fixture.point(&view, 1, header_line, header_line.x + 1.0),
        );
        let (position, page, _) = move_to(&sent[0]);
        assert_eq!(
            (position.flow, page),
            (header_flow, 2),
            "the header instance on page 2"
        );

        let table = layout.pages[0]
            .frames
            .iter()
            .find(|frame| matches!(frame.kind, FrameBoxKind::Table { .. }))
            .unwrap();
        let FrameBoxKind::Table { cells, .. } = &table.kind else {
            unreachable!()
        };
        let second = cells[1];
        let sent = driver.send(
            &mut surface,
            press(),
            view.point(0, second.x + 50.0, second.y + second.height / 2.0),
        );
        let (position, _, _) = move_to(&sent[0]);
        let FrameContent::Table(table_content) =
            &fixture.document.frame(table.frame).unwrap().content
        else {
            unreachable!()
        };
        let amipro_document::CellContent::Text(cell_flow) = table_content.cells[1].content;
        assert_eq!(position.flow, cell_flow);

        // Typing and Esc are actions for the application.
        assert_eq!(
            driver.send(&mut surface, typed("é"), Point::ORIGIN),
            [Sent::Page(PageAction::Insert("é".into()))]
        );
        assert_eq!(
            driver.send(
                &mut surface,
                key(Named::Escape, keyboard::Modifiers::default()),
                Point::ORIGIN
            ),
            [Sent::Page(PageAction::Escape)]
        );
        assert_eq!(
            escape_target(&fixture.document, layout, PageSelection::caret(position, 1)).flow(),
            main
        );
    }

    #[test]
    fn hits_land_on_the_same_text_at_every_zoom_and_scroll_offset() {
        let fixture = Fixture::new(page_fixture());
        let layout = fixture.pages.layout();
        let line = main_lines(layout, 1)[4];
        let x = line.x + 1_800.0;
        let mut expected = None;
        for zoom in [
            Zoom::Percent(50),
            Zoom::Percent(100),
            Zoom::Percent(200),
            Zoom::FitWidth,
        ] {
            for offset in [Vector::ZERO, Vector::new(-130.0, -1_250.0)] {
                let view = fixture.view(zoom);
                let selection = PageSelection::start(&fixture.document);
                let mut surface = fixture.surface(view.clone(), selection);
                let mut driver = Driver::new(&mut surface, offset);
                let sent = driver.send(&mut surface, press(), fixture.point(&view, 1, line, x));
                let (position, page, _) = move_to(&sent[0]);
                assert_eq!(page, 2);
                let found = (position.paragraph, position.byte);
                // Rounding at small sizes may move a hit by one character.
                if let Some((paragraph, byte)) = expected {
                    assert_eq!(found.0, paragraph);
                    assert!(
                        found.1.abs_diff(byte) <= 3,
                        "{zoom:?} {offset:?}: {found:?} vs {byte}"
                    );
                } else {
                    expected = Some(found);
                }
                assert!(line.bytes.contains(&position.byte));
            }
        }
    }

    #[test]
    fn character_attributes_draw_colour_rules_and_shifted_scripts() {
        use amipro_document::{Caps, CharacterFormat};
        let text = "red under word under double struck x2 H2O caps end";
        let mut document = Document::from_plain_text(text);
        let paragraph = document.paragraph_mut(0);
        let at = |word: &str| {
            let start = text.find(word).unwrap();
            (start, start + word.len())
        };
        for (word, format) in [
            (
                "red",
                CharacterFormat {
                    colour: Some(0x0000_00FF),
                    ..CharacterFormat::default()
                },
            ),
            (
                "under ",
                CharacterFormat {
                    underline: Some(true),
                    ..CharacterFormat::default()
                },
            ),
            (
                "word under",
                CharacterFormat {
                    word_underline: Some(true),
                    ..CharacterFormat::default()
                },
            ),
            (
                "double",
                CharacterFormat {
                    double_underline: Some(true),
                    ..CharacterFormat::default()
                },
            ),
            (
                "struck",
                CharacterFormat {
                    strikethrough: Some(true),
                    ..CharacterFormat::default()
                },
            ),
            (
                "2 ",
                CharacterFormat {
                    superscript: Some(true),
                    ..CharacterFormat::default()
                },
            ),
            (
                "2O",
                CharacterFormat {
                    subscript: Some(true),
                    ..CharacterFormat::default()
                },
            ),
            (
                "caps",
                CharacterFormat {
                    caps: Some(Caps::Upper),
                    ..CharacterFormat::default()
                },
            ),
        ] {
            // Search after the words already formatted.
            let (start, end) = if word == "2O" {
                let start = text.find("H2O").unwrap() + 1;
                (start, start + 2)
            } else {
                at(word)
            };
            paragraph
                .apply_character_format(start, end, &format)
                .unwrap();
        }
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let scale = view.scale;
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let (line, cache) = state.pages[0]
            .page
            .lines
            .iter()
            .zip(&state.pages[0].lines)
            .find(|(line, _)| line.flow == fixture.document.main_flow_id())
            .unwrap();
        assert_eq!(line.bytes, 0..text.len(), "one line");
        let decorations: Vec<_> = cache
            .segments
            .iter()
            .flat_map(|segment| &segment.decorations)
            .collect();
        let count = |test: fn(&Decoration) -> bool| {
            decorations
                .iter()
                .filter(|decoration| test(decoration))
                .count()
        };
        // "under " (with its space) and the two words of "word under",
        // whose space is not underlined.
        assert_eq!(count(|decoration| decoration.underline), 3);
        assert_eq!(count(|decoration| decoration.double_underline), 1);
        assert_eq!(count(|decoration| decoration.strikethrough), 1);
        let red = Color::from_rgb8(255, 0, 0);
        assert!(
            decorations
                .iter()
                .all(|decoration| decoration.colour != red)
        );
        // A superscript is raised by 5/12 of 12 pt; a subscript lowered.
        let rises: Vec<f32> = cache
            .segments
            .iter()
            .flat_map(|segment| segment.shifted.iter().map(|(rise, _)| *rise))
            .collect();
        assert_eq!(rises.len(), 2, "{rises:?}");
        assert!(rises.iter().any(|rise| (rise - 100.0 * scale).abs() < 0.01));
        assert!(
            rises
                .iter()
                .any(|rise| *rise < 0.0 && *rise > -100.0 * scale)
        );
        // What is drawn is what was measured: the capitals are as wide as the
        // layout's words, so the line ends where the layout says.
        let segment = cache.segments.last().unwrap();
        let drawn_end = segment.x + segment.paragraph.min_bounds().width / scale;
        let measured_end = line.x + line.width;
        assert!(
            (drawn_end - measured_end).abs() < 30.0,
            "drawn to {drawn_end}, measured to {measured_end}"
        );
        // Carets still land on the text as typed.
        let caps = text.find("caps").unwrap();
        let before = line_x(line, cache, caps, scale);
        let after = line_x(line, cache, caps + 4, scale);
        // In 12 pt Times New Roman "CAPS" is about 600 twips, "caps" about 430.
        assert!(after - before > 550.0, "CAPS is wide: {}", after - before);
        driver.draw(&surface);
    }

    #[test]
    fn tabs_draw_their_leaders_and_place_the_caret_after_them_tb2() {
        use amipro_document::{TabKind, TabLeader, TabStop};
        let mut document = Document::from_plain_text("Name\tPage");
        document.document_layout_mut().right.layout.tabs = vec![vec![TabStop {
            leader: TabLeader::Dots,
            ..TabStop::new(TabKind::Right, 9360)
        }]];
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let scale = view.scale;
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let (line, cache) = state.pages[0]
            .page
            .lines
            .iter()
            .zip(&state.pages[0].lines)
            .find(|(line, _)| line.flow == fixture.document.main_flow_id())
            .unwrap();
        let tab = line.words.iter().find(|word| word.tab.is_some()).unwrap();
        assert_eq!(tab.tab, Some(TabLeader::Dots));
        // The dots fill the gap and stay inside it.
        let dots = leader_quads(line, scale, 0.0, 0.0, 12.0);
        assert!(dots.len() > 20, "{}", dots.len());
        assert!(dots.iter().all(|dot| dot.x >= tab.x * scale - 1.0
            && dot.x + dot.width <= (tab.x + tab.width) * scale + 1.0));
        // "Page" ends at the right stop, and the caret after the tab
        // stands where it starts.
        let page = line.words.last().unwrap();
        assert!((page.x + page.width - (1440.0 + 9360.0)).abs() < 1.0);
        let after_tab = line_x(line, cache, tab.bytes.end, scale);
        assert!((after_tab - page.x).abs() < 1.0, "{after_tab} {}", page.x);
        driver.draw(&surface);
    }

    /// Table › Leaders (`13e8:2422`, VM TE10): after left text to the cell's
    /// edge, before right-aligned text from the cell's edge, none for centred
    /// text, across an empty cell.
    #[test]
    fn cell_leaders_run_by_the_text_alignment_te10() {
        use amipro_document::{Alignment, CellLeader};
        use amipro_editing::tables::{CellChange, CellRange, NewTable, TableEdit, TableTarget};
        let document = Document::from_plain_text("Above");
        let at = amipro_editing::TextPosition {
            flow: document.main_flow_id(),
            paragraph: 0,
            byte: 5,
        };
        let mut document = amipro_editing::apply(
            &document,
            &amipro_editing::Command::InsertTable {
                target: TableTarget::Text(at),
                table: NewTable::fitting(4, 1, 9_360, 284),
            },
        )
        .unwrap()
        .document;
        let frame = *document.frame_stack().last().unwrap();
        let flows: Vec<_> = match &document.frame(frame).unwrap().content {
            FrameContent::Table(table) => table
                .cells
                .iter()
                .map(|cell| match cell.content {
                    amipro_document::CellContent::Text(flow) => flow,
                })
                .collect(),
            _ => unreachable!(),
        };
        for (flow, text, alignment) in [
            (flows[0], "abc", Alignment::Left),
            (flows[1], "abc", Alignment::Right),
            (flows[2], "abc", Alignment::Centre),
        ] {
            let paragraph = Arc::make_mut(&mut document.flow_mut(flow).unwrap().paragraphs[0]);
            paragraph.text = text.into();
            paragraph.format.alignment = Some(alignment);
        }
        document = amipro_editing::apply(
            &document,
            &amipro_editing::Command::EditTable {
                frame,
                edit: TableEdit::FormatCells {
                    cells: CellRange {
                        rows: 0..1,
                        columns: 0..4,
                    },
                    change: CellChange::Leader(CellLeader::Dots),
                },
            },
        )
        .unwrap()
        .document;
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let page = &state.pages[0].page;
        let cells = page
            .frames
            .iter()
            .find_map(|frame_box| match &frame_box.kind {
                FrameBoxKind::Table { cells, .. } if frame_box.frame == frame => {
                    Some(cells.clone())
                }
                _ => None,
            })
            .unwrap();
        let line_of = |cell: usize| {
            page.lines
                .iter()
                .find(|line| line.container == Container::Cell { frame, cell })
                .unwrap()
        };
        let text = |cell: usize| {
            let line = line_of(cell);
            let words: Vec<_> = line.words.iter().filter(|word| word.width > 0.0).collect();
            (
                words[0].x,
                words.last().map(|word| word.x + word.width).unwrap(),
            )
        };
        let span = |cell: usize, alignment| cell_leader_span(cells[cell], line_of(cell), alignment);
        assert_eq!(
            span(0, Alignment::Left),
            Some((text(0).1, cells[0].x + cells[0].width))
        );
        assert_eq!(span(1, Alignment::Right), Some((cells[1].x, text(1).0)));
        assert_eq!(span(2, Alignment::Centre), None);
        assert_eq!(
            span(3, Alignment::Left),
            Some((cells[3].x, cells[3].x + cells[3].width))
        );
        driver.draw(&surface);
    }

    #[test]
    fn tightened_lines_draw_each_character_where_it_was_measured_tt1() {
        let mut document = Document::from_plain_text("Tight words drawn close");
        let id = document.paragraphs()[0].style;
        document
            .styles_mut()
            .get_mut(&id)
            .unwrap()
            .spacing
            .tightness = 90;
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let scale = view.scale;
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let (line, cache) = state.pages[0]
            .page
            .lines
            .iter()
            .zip(&state.pages[0].lines)
            .find(|(line, _)| line.flow == fixture.document.main_flow_id())
            .unwrap();
        // One segment per character, in order.
        let shown: usize = cache.segments.iter().map(|segment| segment.shown).sum();
        assert_eq!(cache.segments.len(), "Tight words drawn close".len());
        assert_eq!(shown, cache.segments.len());
        assert!(cache.segments.windows(2).all(|pair| pair[0].x < pair[1].x));
        // Each word starts where the layout put it, and its characters end
        // where the layout's word does.
        for word in &line.words {
            let mut inside = cache
                .segments
                .iter()
                .filter(|segment| word.bytes.contains(&segment.bytes.start));
            let first = inside.next().unwrap();
            assert_eq!(first.x, word.x);
            let last = inside.next_back().unwrap_or(first);
            let end = last.x + last.paragraph.min_bounds().width / scale;
            assert!(
                (end - (word.x + word.width)).abs() < 30.0,
                "drawn to {end}, measured to {}",
                word.x + word.width
            );
        }
        driver.draw(&surface);
    }

    #[test]
    fn paragraph_rules_draw_their_strokes_in_their_colour_nx1() {
        use amipro_document::{ParagraphRules, RuleLength, RuleSide};
        // NX1: a red thick-over-thin rule below (60, 20, 20 twips) and a
        // double rule above (20, 20, 20), drawn from the space's outer edge.
        let mut document = Document::from_plain_text("Ruled paragraph");
        let id = document.paragraphs()[0].style;
        document.styles_mut().get_mut(&id).unwrap().rules = ParagraphRules {
            above: RuleSide {
                on: true,
                line: 8,
                space: 150,
            },
            below: RuleSide {
                on: true,
                line: 11,
                space: 250,
            },
            length: RuleLength::Margins,
            colour: 0x0000_00FF,
            ..ParagraphRules::default()
        };
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let scale = view.scale;
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let line = state.pages[0]
            .page
            .lines
            .iter()
            .find(|line| line.flow == fixture.document.main_flow_id())
            .unwrap();
        let quads = rule_quads(line, scale, 0.0, 0.0);
        let red = Color::from_rgb8(255, 0, 0);
        assert_eq!(quads.len(), 4, "two strokes each");
        assert!(quads.iter().all(|(_, colour)| *colour == red));
        let tops: Vec<f32> = quads.iter().map(|(bounds, _)| bounds.y).collect();
        let bottom = line.height * scale;
        assert_eq!(tops[0], 0.0);
        assert_eq!(tops[1], (40.0 * scale).round());
        assert_eq!(tops[2], (bottom - 100.0 * scale).round());
        assert_eq!(tops[3], (bottom - 20.0 * scale).round());
        assert_eq!(quads[2].0.height, (60.0 * scale).round(), "thick over thin");
        assert!(quads.iter().all(|(bounds, _)| bounds.height >= 1.0));
        assert_eq!(quads[0].0.width, line.rules[0].width * scale);
        driver.draw(&surface);
    }

    #[test]
    fn overstrike_draws_its_character_over_each_character_o1() {
        // O1: "over" with "/" over each of its four letters; the text keeps
        // its widths.
        use amipro_document::{CharacterFormat, CharacterRun};
        let mut document = Document::from_plain_text("over end");
        document.paragraph_mut(0).runs.push(CharacterRun {
            start: 0,
            end: 4,
            format: CharacterFormat {
                overstrike: Some('/'),
                ..CharacterFormat::default()
            },
        });
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let cache = state.pages[0]
            .page
            .lines
            .iter()
            .zip(&state.pages[0].lines)
            .find(|(line, _)| line.flow == fixture.document.main_flow_id())
            .unwrap()
            .1;
        let glyphs: Vec<_> = cache
            .segments
            .iter()
            .flat_map(|segment| &segment.overstrikes)
            .collect();
        assert_eq!(glyphs.len(), 1, "one character, shaped once");
        let offsets = &glyphs[0].1;
        assert_eq!(offsets.len(), 4);
        assert!(offsets.windows(2).all(|pair| pair[0].x < pair[1].x));
        let segment = cache
            .segments
            .iter()
            .find(|segment| !segment.overstrikes.is_empty())
            .unwrap();
        let width = segment.paragraph.min_bounds().width;
        assert!(
            offsets
                .iter()
                .all(|offset| offset.x > -2.0 && offset.x < width)
        );
        driver.draw(&surface);
    }

    #[test]
    fn bullets_and_numbers_draw_before_the_first_line_bn1() {
        use amipro_document::Bullets;
        // BN1: a dot, a check mark, and a number before three paragraphs.
        let mut document = Document::from_plain_text("dot\ncheck\nnumber");
        let body = document.paragraphs()[0].style;
        for (index, text) in ["\u{11}", "\u{19}", "\u{1A}."].into_iter().enumerate() {
            let mut style = document.styles()[&body].clone();
            style.id = amipro_document::StyleId(50 + index as u64);
            style.name = format!("Bullet {index}");
            style.bullets = Bullets {
                text: text.into(),
                ..Bullets::default()
            };
            style.indents.first = 360;
            document.styles_mut().insert(style.id, style.clone());
            document.paragraph_mut(index).style = style.id;
        }
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let scale = view.scale;
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let lines: Vec<_> = state.pages[0]
            .page
            .lines
            .iter()
            .zip(&state.pages[0].lines)
            .filter(|(line, _)| line.flow == fixture.document.main_flow_id())
            .collect();
        // The dot: one round, filled quad in the bullet's box, its bottom
        // above the baseline.
        let (dot, dot_cache) = lines[0];
        let bullet = dot.bullet.as_ref().unwrap();
        let marks = bullet_marks(dot, scale, 0.0, 0.0);
        assert_eq!(marks.len(), 1);
        let (bounds, (radius, true)) = marks[0] else {
            panic!("{marks:?}");
        };
        assert_eq!(radius, bounds.width / 2.0);
        assert!(bounds.x >= bullet.x * scale - 1.0);
        assert!(bounds.x + bounds.width <= (bullet.x + bullet.width) * scale + 1.0);
        assert!(bounds.y + bounds.height < (dot.baseline - dot.top) * scale);
        assert!(dot_cache.bullet_text.is_empty());
        // The check mark: pixel strokes; the number: shaped text at the
        // left indent, where no mark is drawn.
        assert!(bullet_marks(lines[1].0, scale, 0.0, 0.0).len() > 3);
        let (number, number_cache) = lines[2];
        assert!(bullet_marks(number, scale, 0.0, 0.0).is_empty());
        assert_eq!(number_cache.bullet_text.len(), 1);
        assert_eq!(number_cache.bullet_text[0].x, 1440.0);
        assert_eq!(number.words[0].x, 1440.0 + 360.0);
        driver.draw(&surface);
    }

    #[test]
    fn inserted_bullets_draw_in_their_box_and_take_the_caret_as_one_character_bl1() {
        // BL1: a dot inside a paragraph and one ending another, in red.
        let mut document = Document::from_plain_text("A dot\nEnd");
        let paragraph = document.paragraph_mut(0);
        paragraph.text = "A\u{FFFC}dot".into();
        paragraph.objects.push(InlineObject::Bullet('\u{11}'));
        let paragraph = document.paragraph_mut(1);
        paragraph.text = "End\u{FFFC}".into();
        paragraph.objects.push(InlineObject::Bullet('\u{13}'));
        paragraph.runs.push(amipro_document::CharacterRun {
            start: 3,
            end: 6,
            format: amipro_document::CharacterFormat {
                colour: Some(255),
                ..Default::default()
            },
        });
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let scale = view.scale;
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let lines: Vec<_> = state.pages[0]
            .page
            .lines
            .iter()
            .zip(&state.pages[0].lines)
            .filter(|(line, _)| line.flow == fixture.document.main_flow_id())
            .collect();
        let (inside, inside_cache) = lines[0];
        let word = inside
            .words
            .iter()
            .find(|word| word.object == Some(0))
            .unwrap();
        assert_eq!(inside_cache.inline_bullets.len(), 1);
        let marks = inline_bullet_marks(inside, &inside_cache.inline_bullets, scale, 0.0, 0.0);
        assert_eq!(marks.len(), 1);
        let ((bounds, (_, true)), colour) = marks[0] else {
            panic!("{marks:?}");
        };
        assert_eq!(colour, Color::BLACK);
        assert!(bounds.x >= word.x * scale - 1.0);
        assert!(bounds.x + bounds.width <= (word.x + word.width) * scale + 1.0);
        // The caret before and after it, and clicks on either half.
        let end = word.bytes.end;
        assert_eq!(line_x(inside, inside_cache, 1, scale), word.x);
        assert_eq!(
            line_x(inside, inside_cache, end, scale),
            word.x + word.width
        );
        assert_eq!(
            line_hit(inside, inside_cache, word.x + word.width * 0.25, scale),
            1
        );
        assert_eq!(
            line_hit(inside, inside_cache, word.x + word.width * 0.75, scale),
            end
        );
        // At the end of a line, in the colour of its text.
        let (last, last_cache) = lines[1];
        let bullet = &last_cache.inline_bullets[0];
        assert_eq!(bullet.colour, Color::from_rgb8(255, 0, 0));
        assert_eq!(line_x(last, last_cache, 6, scale), bullet.x + bullet.width);
        assert_eq!(line_x(last, last_cache, 3, scale), bullet.x);
        assert_eq!(
            line_hit(last, last_cache, bullet.x + bullet.width + 50.0, scale),
            6
        );
        driver.draw(&surface);
    }

    #[test]
    fn picture_bullets_are_drawn_as_shapes_in_their_box_bl1() {
        // Ami Pro's seven pictures, each as marks inside its square, none
        // past the baseline or outside the bullet's width.
        for code in [
            '\u{8}', '\u{A}', '\u{B}', '\u{C}', '\u{D}', '\u{E}', '\u{F}',
        ] {
            let (start, width, size, baseline) = (100.0, 24.0, 18.0, 50.0);
            let marks = shape_marks(code, start, width, size, baseline);
            assert!(marks.len() >= 3, "{code:?}");
            for (bounds, _) in &marks {
                assert!(bounds.x >= start - 1.0, "{code:?} {bounds:?}");
                assert!(bounds.x + bounds.width <= start + width + 1.0, "{code:?}");
                assert!(bounds.y >= baseline - size - 1.0, "{code:?} {bounds:?}");
                assert!(
                    bounds.y + bounds.height <= baseline + 1.0,
                    "{code:?} {bounds:?}"
                );
            }
        }
        // The two arrowheads differ: lit from above or from below.
        assert_ne!(
            shape_marks('\u{E}', 0.0, 24.0, 18.0, 50.0),
            shape_marks('\u{F}', 0.0, 24.0, 18.0, 50.0)
        );
    }

    #[test]
    fn superscript_numbers_are_drawn_raised_and_right_aligned_bl2_bl3() {
        use amipro_document::Bullets;
        // BL3: Number List with Superscript and Right align.
        let mut document = Document::from_plain_text("one\ntwo");
        let body = document.paragraphs()[0].style;
        let mut style = document.styles()[&body].clone();
        style.id = amipro_document::StyleId(60);
        style.name = "Number List".into();
        style.bullets = Bullets {
            text: "\u{1A}.".into(),
            attributes: 256,
            flags: 16,
            ..Bullets::default()
        };
        style.indents.all = 360;
        style.indents.first = 360;
        document.styles_mut().insert(style.id, style.clone());
        document.paragraph_mut(0).style = style.id;
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let selection = PageSelection::start(&fixture.document);
        let mut surface = fixture.surface(view, selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let state = driver.state();
        let (line, cache) = state.pages[0]
            .page
            .lines
            .iter()
            .zip(&state.pages[0].lines)
            .find(|(line, _)| line.flow == fixture.document.main_flow_id())
            .unwrap();
        let bullet = line.bullet.as_ref().unwrap();
        assert!(bullet.superscript);
        // It ends a space before the text, which keeps its indent.
        assert!(bullet.x > 1440.0 + 360.0);
        assert_eq!(line.words[0].x, 1440.0 + 720.0);
        assert_eq!(cache.bullet_text.len(), 1);
        assert!(cache.bullet_text[0].rise > 0.0);
        driver.draw(&surface);
    }

    #[test]
    fn edits_reshape_only_the_edited_lines_and_never_draw_stale_text() {
        let mut fixture = Fixture::new(page_fixture());
        let view = fixture.view(Zoom::Percent(100));
        let selection = PageSelection::start(&fixture.document);
        let built = {
            let mut surface = fixture.surface(view.clone(), selection);
            let driver = Driver::new(&mut surface, Vector::ZERO);
            driver.state().built
        };
        assert!(built > 50, "the first layout shapes every line ({built})");

        // Keep one widget tree across an edit, as Iced does.
        let mut surface = fixture.surface(view.clone(), selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let before = driver.state().built;
        drop(surface);
        let main = fixture.document.main_flow_id();
        let applied = amipro_editing::apply(
            &fixture.document,
            &amipro_editing::Command::insert(
                TextPosition {
                    flow: main,
                    paragraph: 40,
                    byte: 0,
                },
                "Edited ",
            ),
        )
        .unwrap();
        fixture.document = applied.document;
        fixture.pages.update(&fixture.document);
        let view = fixture.view(Zoom::Percent(100));
        let mut surface = fixture.surface(view, selection);
        driver.relayout(&mut surface);
        let rebuilt = driver.state().built - before;
        assert!(rebuilt <= 4, "{rebuilt} lines reshaped for one paragraph");
        // Every cached line shows the document's current paragraph.
        for cached in &driver.state().pages {
            for (line, cache) in cached.page.lines.iter().zip(&cached.lines) {
                let current = source_of(&fixture.document, line);
                match (&cache.source, current) {
                    (Some(cached), Some(current)) => assert_eq!(cached.text, current.text),
                    (None, None) => {}
                    _ => panic!("stale line {line:?}"),
                }
            }
        }
        driver.draw(&surface);
    }

    #[test]
    fn ime_commit_inserts_and_navigation_keys_stay_in_their_flow() {
        let fixture = Fixture::new(page_fixture());
        let view = fixture.view(Zoom::Percent(100));
        let layout = fixture.pages.layout();
        let main = fixture.document.main_flow_id();
        let line = main_lines(layout, 0)[2];
        let selection = PageSelection::caret(
            TextPosition {
                flow: main,
                paragraph: line.paragraph_index,
                byte: line.bytes.start + 4,
            },
            1,
        );
        let mut surface = fixture.surface(view.clone(), selection);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.send(
            &mut surface,
            press(),
            fixture.point(&view, 0, line, line.x + 1.0),
        );
        driver.send(
            &mut surface,
            Event::InputMethod(input_method::Event::Preedit("にほ".into(), Some(0..3))),
            Point::ORIGIN,
        );
        assert!(driver.state().preedit.is_some());
        // The composing text is drawn at the caret (no panic with a
        // selection inside it, nor one past its end).
        driver.draw(&surface);
        driver.send(
            &mut surface,
            Event::InputMethod(input_method::Event::Preedit("にほ".into(), Some(3..99))),
            Point::ORIGIN,
        );
        driver.draw(&surface);
        let sent = driver.send(
            &mut surface,
            Event::InputMethod(input_method::Event::Commit("日本".into())),
            Point::ORIGIN,
        );
        assert_eq!(sent, [Sent::Page(PageAction::Insert("日本".into()))]);
        assert!(driver.state().preedit.is_none());

        let sent = driver.send(
            &mut surface,
            key(Named::Home, keyboard::Modifiers::SHIFT),
            Point::ORIGIN,
        );
        let (position, _, extend) = move_to(&sent[0]);
        assert_eq!(position.byte, line.bytes.start);
        assert!(extend);
        let sent = driver.send(
            &mut surface,
            key(Named::End, keyboard::Modifiers::default()),
            Point::ORIGIN,
        );
        let (position, _, _) = move_to(&sent[0]);
        assert!(position.byte <= line.bytes.end && position.byte + 1 >= line.bytes.end);
        let sent = driver.send(
            &mut surface,
            key(Named::PageDown, keyboard::Modifiers::default()),
            Point::ORIGIN,
        );
        let (position, page, _) = move_to(&sent[0]);
        assert_eq!(position.flow, main);
        assert!(position.paragraph > line.paragraph_index && page >= 1);
        // Shortcuts go to the command registry.
        let control_z = Event::Keyboard(keyboard::Event::KeyPressed {
            key: Key::Character("z".into()),
            modified_key: Key::Character("z".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyZ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::CTRL,
            text: None,
            repeat: false,
        });
        assert_eq!(
            driver.send(&mut surface, control_z, Point::ORIGIN),
            [Sent::Command(CommandId::Undo)]
        );
    }

    /// The page fixture's text frame on page 1: 4.5 × 5 in to 7 × 6.5 in.
    const FRAME_RECT: amipro_document::Rect =
        amipro_document::Rect::new(6_480, 7_200, 10_080, 9_360);

    fn fixture_frame(document: &Document) -> FrameId {
        *document
            .frame_stack()
            .iter()
            .find(|id| {
                matches!(
                    document.frame(**id).unwrap().placement,
                    Placement::Fixed { .. }
                )
            })
            .unwrap()
    }

    #[test]
    fn a_right_press_selects_what_is_under_the_pointer_for_the_menu() {
        // `UI_SPEC.md` §16: the menu opens on the release and acts on what
        // the press selected.
        let right = || Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
        // Each right press ends saying whether it was in a margin (below),
        // after what it selected.
        let without_context = |mut sent: Vec<Sent>| -> Vec<Sent> {
            assert!(matches!(
                sent.pop(),
                Some(Sent::Page(PageAction::ContextAt { margin: false }))
            ));
            sent
        };
        let fixture = Fixture::new(page_fixture());
        let frame = fixture_frame(&fixture.document);
        let caret = PageSelection::start(&fixture.document);
        let view = fixture.view(Zoom::Percent(100));
        let inside = view.point(0, 8_000.0, 8_000.0);
        // An unselected frame is selected.
        let mut surface = fixture.framed(view.clone(), caret, None);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        assert_eq!(
            without_context(driver.send(&mut surface, right(), inside)),
            [Sent::Frame(FrameAction::Select {
                frame,
                page: 1,
                extend: false
            })]
        );
        assert!(driver.state().frame_drag.is_none(), "no drag starts");
        // A selected frame stays selected.
        let selection = FrameSelection::one(frame, 1);
        let mut surface = fixture.framed(view.clone(), caret, Some(&selection));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        assert!(without_context(driver.send(&mut surface, right(), inside)).is_empty());
        // In text the caret moves to the pointer...
        let lines = main_lines(fixture.pages.layout(), 0);
        let third = fixture.point(&view, 0, lines[2], lines[2].x + 1_000.0);
        let mut surface = fixture.framed(view.clone(), caret, None);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let sent = without_context(driver.send(&mut surface, right(), third));
        let (position, page, extend) = move_to(&sent[0]);
        assert_eq!(
            (position.paragraph, page, extend),
            (lines[2].paragraph_index, 1, false)
        );
        assert!(!driver.state().dragging, "a right press does not drag");
        // ... but a press in the selected text keeps the selection.
        let main = fixture.document.main_flow_id();
        let selected = PageSelection {
            anchor: TextPosition {
                flow: main,
                paragraph: 0,
                byte: 0,
            },
            focus: TextPosition {
                flow: main,
                paragraph: lines[3].paragraph_index,
                byte: lines[3].bytes.end,
            },
            page: 1,
        };
        let mut surface = fixture.framed(view.clone(), selected, None);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        assert!(without_context(driver.send(&mut surface, right(), third)).is_empty());
    }

    #[test]
    fn presses_in_a_side_margin_offer_and_open_page_layout_properties() {
        // `UI_SPEC.md` §16 and §18: Ami Pro's right button in a margin opens
        // Modify Page Layout, so the menu offers Page Layout… first there,
        // and a double-click in a margin opens it.
        let right = || Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right));
        let left = || Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let release = || Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        let fixture = Fixture::new(page_fixture());
        let caret = PageSelection::start(&fixture.document);
        let view = fixture.view(Zoom::Percent(100));
        let lines = main_lines(fixture.pages.layout(), 0);
        let margin = view.point(0, 500.0, lines[2].top + 10.0);
        let text = fixture.point(&view, 0, lines[2], lines[2].x + 1_000.0);
        let mut surface = fixture.framed(view.clone(), caret, None);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let sent = driver.send(&mut surface, right(), margin);
        assert!(matches!(
            sent.last(),
            Some(Sent::Page(PageAction::ContextAt { margin: true }))
        ));
        let sent = driver.send(&mut surface, right(), text);
        assert!(matches!(
            sent.last(),
            Some(Sent::Page(PageAction::ContextAt { margin: false }))
        ));
        driver.send(&mut surface, left(), margin);
        driver.send(&mut surface, release(), margin);
        let sent = driver.send(&mut surface, left(), margin);
        assert!(
            sent.iter()
                .any(|sent| matches!(sent, Sent::Page(PageAction::OpenPageLayout))),
            "{sent:?}"
        );
        // A double-click in the text does not.
        let mut surface = fixture.framed(view.clone(), caret, None);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.send(&mut surface, left(), text);
        driver.send(&mut surface, release(), text);
        let sent = driver.send(&mut surface, left(), text);
        assert!(
            !sent
                .iter()
                .any(|sent| matches!(sent, Sent::Page(PageAction::OpenPageLayout)))
        );
    }

    /// Facing Pages (CM8): a press on the right-hand page of a pair puts
    /// the caret on that page, not on the left one beside it.
    #[test]
    fn a_press_on_a_facing_page_finds_the_page_across() {
        let left = || Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let fixture = Fixture::new(page_fixture());
        assert!(fixture.pages.layout().pages.len() >= 3);
        let caret = PageSelection::start(&fixture.document);
        let view = PageView::arranged(
            fixture.pages.layout(),
            Zoom::Percent(50),
            Size::new(900.0, 700.0),
            true,
        );
        assert!((view.pages[1].y - view.pages[2].y).abs() < 0.5);
        for page in [1, 2] {
            let lines = main_lines(fixture.pages.layout(), page);
            let point = fixture.point(&view, page, lines[0], lines[0].x + 200.0);
            let mut surface = fixture.surface(view.clone(), caret);
            let mut driver = Driver::new(&mut surface, Vector::ZERO);
            let sent = driver.send(&mut surface, left(), point);
            assert!(
                sent.iter().any(|sent| matches!(
                    sent,
                    Sent::Page(PageAction::MoveTo { page: shown, .. }) if *shown == page as u32 + 1
                )),
                "page {page}: {sent:?}"
            );
        }
    }

    /// Fast Format (CM4): a press and release in text tells the app the
    /// mouse made a selection, only while the mode is on.
    #[test]
    fn a_release_after_a_press_in_text_is_sent_only_while_fast_format_is_on() {
        let left = || Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left));
        let release = || Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left));
        let fixture = Fixture::new(page_fixture());
        let caret = PageSelection::start(&fixture.document);
        let view = fixture.view(Zoom::Percent(100));
        let lines = main_lines(fixture.pages.layout(), 0);
        let text = fixture.point(&view, 0, lines[2], lines[2].x + 1_000.0);
        for on in [false, true] {
            let mut surface = fixture.surface(view.clone(), caret).fast_format(on);
            let mut driver = Driver::new(&mut surface, Vector::ZERO);
            driver.send(&mut surface, left(), text);
            let sent = driver.send(&mut surface, release(), text);
            assert_eq!(
                sent.iter()
                    .any(|sent| matches!(sent, Sent::Page(PageAction::PointerReleased))),
                on,
                "{sent:?}"
            );
        }
    }

    /// A drop on the page (P7): the drag arrives as the pointer, and the
    /// drop moves the caret there before the dropped file arrives.
    #[test]
    fn a_drop_moves_the_caret_to_where_it_was_dropped() {
        let fixture = Fixture::new(page_fixture());
        let caret = PageSelection::start(&fixture.document);
        let view = fixture.view(Zoom::Percent(100));
        let lines = main_lines(fixture.pages.layout(), 0);
        let target = fixture.point(&view, 0, lines[2], lines[2].x + 1_000.0);
        let mut surface = fixture.surface(view.clone(), caret);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        driver.send(&mut surface, moved(target), target);
        let sent = driver.send(
            &mut surface,
            Event::Dnd(cosmic::iced::clipboard::dnd::DndEvent::Offer(
                None,
                cosmic::iced::clipboard::dnd::OfferEvent::Drop,
            )),
            target,
        );
        let [Sent::Page(PageAction::MoveTo { position, .. })] = sent.as_slice() else {
            panic!("{sent:?}");
        };
        assert_eq!(position.paragraph, lines[2].paragraph_index);
    }

    /// Picture mode through the widget (`UI_SPEC.md` §25, P6): a drag in
    /// the frame moves its picture, arrows by 0.05 in and with Ctrl by 0.5
    /// in, Esc leaves; the pointer is a hand. Outside picture mode, Shift
    /// and a corner handle size the picture with its frame.
    #[test]
    fn picture_mode_moves_the_picture_through_the_widget() {
        let mut document = page_fixture();
        let frame = fixture_frame(&document);
        let object = document.add_object(Arc::from(&b"BM"[..]));
        document.frame_mut(frame).unwrap().content =
            FrameContent::Picture(amipro_document::Picture::new(
                amipro_document::PictureSource::Embedded(object),
                ".bmp",
                amipro_document::Size {
                    width: 600,
                    height: 400,
                },
            ));
        let fixture = Fixture::new(document);
        let caret = PageSelection::start(&fixture.document);
        let selection = FrameSelection::one(frame, 1);
        let view = fixture.view(Zoom::Percent(100));
        let twips = |pixels: f32| (pixels / view.scale).round() as amipro_document::Twips;
        let inside = view.point(0, 8_000.0, 8_000.0);
        let mut surface = fixture
            .framed(view.clone(), caret, Some(&selection))
            .picture_mode(true);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        assert_eq!(
            Widget::<Sent, cosmic::Theme, Renderer>::mouse_interaction(
                &surface,
                &driver.tree,
                Layout::new(&driver.node),
                mouse::Cursor::Available(inside),
                &Rectangle::new(Point::ORIGIN, Size::new(900.0, 700.0)),
                &driver.renderer,
            ),
            mouse::Interaction::Grab
        );
        assert!(driver.send(&mut surface, press(), inside).is_empty());
        let to = inside + Vector::new(30.0, -12.0);
        driver.send(&mut surface, moved(to), to);
        driver.draw(&surface);
        assert_eq!(
            driver.send(&mut surface, release(), to),
            [Sent::Frame(FrameAction::Pan {
                dx: twips(30.0),
                dy: twips(-12.0),
                key: false
            })]
        );
        let none = keyboard::Modifiers::empty();
        assert_eq!(
            driver.send(&mut surface, key(Named::ArrowRight, none), inside),
            [Sent::Frame(FrameAction::Pan {
                dx: 72,
                dy: 0,
                key: true
            })]
        );
        assert_eq!(
            driver.send(
                &mut surface,
                key(Named::ArrowDown, keyboard::Modifiers::CTRL),
                inside
            ),
            [Sent::Frame(FrameAction::Pan {
                dx: 0,
                dy: 720,
                key: true
            })]
        );
        assert_eq!(
            driver.send(&mut surface, key(Named::Escape, none), inside),
            [Sent::Frame(FrameAction::LeavePicture)]
        );

        // Not in picture mode: Shift and the bottom-right handle.
        let mut surface = fixture.framed(view.clone(), caret, Some(&selection));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let corner = view.point(0, 10_080.0, 9_360.0) + Vector::new(2.0, -2.0);
        driver.send(
            &mut surface,
            Event::Keyboard(keyboard::Event::ModifiersChanged(
                keyboard::Modifiers::SHIFT,
            )),
            corner,
        );
        assert!(driver.send(&mut surface, press(), corner).is_empty());
        let to = corner + Vector::new(40.0, 40.0);
        driver.send(&mut surface, moved(to), to);
        let sent = driver.send(&mut surface, release(), to);
        assert!(
            matches!(
                sent.as_slice(),
                [Sent::Frame(FrameAction::Resize {
                    with_picture: true,
                    ..
                })]
            ),
            "{sent:?}"
        );
    }

    #[test]
    fn frames_select_move_and_resize_through_the_widget_at_every_zoom() {
        let fixture = Fixture::new(page_fixture());
        let frame = fixture_frame(&fixture.document);
        let caret = PageSelection::start(&fixture.document);
        let selection = FrameSelection::one(frame, 1);
        for (zoom, offset) in [
            (Zoom::Percent(50), Vector::ZERO),
            (Zoom::Percent(100), Vector::new(0.0, -300.0)),
            (Zoom::Percent(200), Vector::new(-500.0, -1_100.0)),
        ] {
            let view = fixture.view(zoom);
            let scale = view.scale;
            let twips = |pixels: f32| (pixels / scale).round() as amipro_document::Twips;
            let inside = view.point(0, 8_000.0, 8_000.0);

            // Text state: the first click selects; releasing does nothing.
            let mut surface = fixture.framed(view.clone(), caret, None);
            let mut driver = Driver::new(&mut surface, offset);
            assert_eq!(
                driver.send(&mut surface, press(), inside),
                [Sent::Frame(FrameAction::Select {
                    frame,
                    page: 1,
                    extend: false
                })],
                "{zoom:?}"
            );
            assert!(driver.send(&mut surface, release(), inside).is_empty());

            // Selected: a drag from inside moves it by the pointer's offset.
            let mut surface = fixture.framed(view.clone(), caret, Some(&selection));
            let mut driver = Driver::new(&mut surface, offset);
            assert!(driver.send(&mut surface, press(), inside).is_empty());
            let to = inside + Vector::new(40.0, 20.0);
            assert!(driver.send(&mut surface, moved(to + offset), to).is_empty());
            assert!(driver.state().frame_drag.is_some_and(|drag| drag.moved));
            driver.draw(&surface);
            assert_eq!(
                driver.send(&mut surface, release(), to),
                [Sent::Frame(FrameAction::Move {
                    dx: twips(40.0),
                    dy: twips(20.0),
                    key: false
                })],
                "{zoom:?}"
            );

            // The bottom-right handle, hit within its 12-pixel target.
            let corner = view.point(0, 10_080.0, 9_360.0) + Vector::new(5.0, -5.0);
            assert_eq!(
                Widget::<Sent, cosmic::Theme, Renderer>::mouse_interaction(
                    &surface,
                    &driver.tree,
                    Layout::new(&driver.node),
                    mouse::Cursor::Available(corner + offset),
                    &Rectangle::new(Point::ORIGIN, Size::new(900.0, 700.0)),
                    &driver.renderer,
                ),
                mouse::Interaction::ResizingDiagonallyDown
            );
            assert!(driver.send(&mut surface, press(), corner).is_empty());
            let to = corner + Vector::new(30.0, 16.0);
            driver.send(&mut surface, moved(to + offset), to);
            assert_eq!(
                driver.send(&mut surface, release(), to),
                [Sent::Frame(FrameAction::Resize {
                    from: FRAME_RECT,
                    to: amipro_document::Rect::new(
                        6_480,
                        7_200,
                        10_080 + twips(30.0),
                        9_360 + twips(16.0)
                    ),
                    with_picture: false,
                })],
                "{zoom:?}"
            );

            // Seven pixels past the corner is neither handle nor frame.
            let outside = view.point(0, 10_080.0, 9_360.0) + Vector::new(7.0, 7.0);
            let sent = driver.send(&mut surface, press(), outside);
            assert!(matches!(
                sent.as_slice(),
                [Sent::Page(PageAction::MoveTo { .. })]
            ));
            driver.send(&mut surface, release(), outside);
        }
    }

    #[test]
    fn alt_arrows_promote_and_demote_text_and_resize_a_selected_frame() {
        // `UI_SPEC.md` §22: with the caret in text Alt+Left and Alt+Right
        // are Promote and Demote; plain arrows still move the caret.
        let fixture = Fixture::new(page_fixture());
        let caret = PageSelection::start(&fixture.document);
        let view = fixture.view(Zoom::Percent(100));
        let mut surface = fixture.surface(view.clone(), caret);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let inside = view.point(0, 3_000.0, 3_000.0);
        // A click puts the focus (and the caret) in the text.
        driver.send(&mut surface, press(), inside);
        driver.send(&mut surface, release(), inside);
        let alt = keyboard::Modifiers::ALT;
        assert_eq!(
            driver.send(&mut surface, key(Named::ArrowLeft, alt), inside),
            [Sent::Command(CommandId::Promote)]
        );
        assert_eq!(
            driver.send(&mut surface, key(Named::ArrowRight, alt), inside),
            [Sent::Command(CommandId::Demote)]
        );
        assert!(matches!(
            driver.send(
                &mut surface,
                key(Named::ArrowRight, keyboard::Modifiers::default()),
                inside
            )[..],
            [Sent::Page(PageAction::MoveRight { extend: false })]
        ));
    }

    #[test]
    fn esc_cancels_a_drag_and_keys_act_on_selected_frames() {
        let fixture = Fixture::new(page_fixture());
        let frame = fixture_frame(&fixture.document);
        let caret = PageSelection::start(&fixture.document);
        let selection = FrameSelection::one(frame, 1);
        let view = fixture.view(Zoom::Percent(100));
        let mut surface = fixture.framed(view.clone(), caret, Some(&selection));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let inside = view.point(0, 8_000.0, 8_000.0);

        driver.send(&mut surface, press(), inside);
        let to = inside + Vector::new(50.0, 0.0);
        driver.send(&mut surface, moved(to), to);
        let none = keyboard::Modifiers::default();
        assert!(
            driver
                .send(&mut surface, key(Named::Escape, none), to)
                .is_empty()
        );
        assert_eq!(driver.state().frame_drag, None);
        assert!(
            driver.send(&mut surface, release(), to).is_empty(),
            "a cancelled drag moves nothing"
        );

        let shift = keyboard::Modifiers::SHIFT;
        let alt = keyboard::Modifiers::ALT;
        for (event, expected) in [
            (
                key(Named::ArrowRight, none),
                FrameAction::Move {
                    dx: 20,
                    dy: 0,
                    key: true,
                },
            ),
            (
                key(Named::ArrowUp, shift),
                FrameAction::Move {
                    dx: 0,
                    dy: -144,
                    key: true,
                },
            ),
            (
                key(Named::ArrowLeft, alt),
                FrameAction::ResizeBy { dx: -20, dy: 0 },
            ),
            (
                key(Named::Tab, none),
                FrameAction::Cycle { backwards: false },
            ),
            (
                key(Named::Tab, shift),
                FrameAction::Cycle { backwards: true },
            ),
            (key(Named::Enter, none), FrameAction::EnterText { at: None }),
            (key(Named::Delete, none), FrameAction::Delete),
            (key(Named::Escape, none), FrameAction::Deselect),
        ] {
            assert_eq!(
                driver.send(&mut surface, event, inside),
                [Sent::Frame(expected)]
            );
        }
        assert!(
            driver.send(&mut surface, typed("x"), inside).is_empty(),
            "typing does nothing while a frame is selected"
        );
        let mut undo = key(Named::Enter, keyboard::Modifiers::CTRL);
        if let Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) = &mut undo {
            *key = Key::Character("z".into());
        }
        assert_eq!(
            driver.send(&mut surface, undo, inside),
            [Sent::Command(CommandId::Undo)]
        );
        // No caret or IME while a frame is selected; the frame is drawn.
        driver.draw(&surface);
    }

    #[test]
    fn double_clicks_enter_frame_text_and_a_click_in_a_table_enters_its_cell() {
        let fixture = Fixture::new(page_fixture());
        let frame = fixture_frame(&fixture.document);
        let caret = PageSelection::start(&fixture.document);
        let selection = FrameSelection::one(frame, 1);
        let view = fixture.view(Zoom::Percent(100));
        let mut surface = fixture.framed(view.clone(), caret, Some(&selection));
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let frame_box = &fixture.pages.layout().pages[0]
            .frames
            .iter()
            .find(|frame_box| frame_box.frame == frame)
            .unwrap()
            .clone();
        let line = fixture.pages.layout().pages[0]
            .lines
            .iter()
            .find(|line| line.container == Container::Frame(frame))
            .unwrap();
        let on_text = fixture.point(&view, 0, line, frame_box.interior.x + 30.0);
        driver.send(&mut surface, press(), on_text);
        assert!(driver.send(&mut surface, release(), on_text).is_empty());
        let sent = driver.send(&mut surface, press(), on_text);
        let [
            Sent::Frame(FrameAction::EnterText {
                at: Some((position, 1)),
            }),
        ] = sent.as_slice()
        else {
            panic!("expected EnterText, got {sent:?}");
        };
        assert_eq!(position.flow, fixture.frame_flow(frame));

        // A table: the first click selects, a click on the selected table
        // enters the cell under the pointer.
        let mut document = page_fixture();
        let table = amipro_editing::apply(
            &document,
            &amipro_editing::Command::CreateFrame {
                placement: Placement::Fixed {
                    page: 1,
                    rect: amipro_document::Rect::new(1_440, 10_080, 5_760, 11_520),
                },
                style: amipro_document::FrameStyle::default(),
                content: amipro_editing::NewFrameContent::Table {
                    columns: vec![1_440, 1_440],
                    rows: vec![576, 576],
                },
                anchor: None,
                layout: None,
            },
        )
        .unwrap();
        document = table.document;
        let table = *document.frame_stack().last().unwrap();
        let fixture = Fixture::new(document);
        let view = fixture.view(Zoom::Percent(100));
        let table_box = fixture.pages.layout().pages[0]
            .frames
            .iter()
            .find(|frame_box| frame_box.frame == table)
            .unwrap()
            .clone();
        let FrameBoxKind::Table { cells, .. } = &table_box.kind else {
            panic!("a table box")
        };
        let second = cells[1];
        let in_second = view.point(
            0,
            second.x + second.width / 2.0,
            second.y + second.height / 2.0,
        );
        // A click in a table's cell puts the caret there at once, as in
        // Ami Pro (TK6; VM TE1–TE19): no frame selection first.
        let caret = PageSelection::start(&fixture.document);
        let mut surface = fixture.framed(view.clone(), caret, None);
        let mut driver = Driver::new(&mut surface, Vector::ZERO);
        let sent = driver.send(&mut surface, press(), in_second);
        let [Sent::Page(PageAction::MoveTo { position, .. })] = sent.as_slice() else {
            panic!("expected MoveTo, got {sent:?}");
        };
        let FrameContent::Table(grid) = &fixture.document.frame(table).unwrap().content else {
            unreachable!()
        };
        let amipro_document::CellContent::Text(cell_flow) = grid.cells[1].content;
        assert_eq!(position.flow, cell_flow);
    }

    /// Release-mode drag baseline (`UI_SPEC.md` §11, "Performance": at least
    /// 60 redraws a second): a frame on `AMIPRO_PERF_SAM` (README31) or the
    /// fixture is dragged by 200 pointer moves, each the widget's update and
    /// a draw of the visible pages with the outline.
    #[test]
    #[ignore = "manual release-mode frame drag performance baseline"]
    fn frame_drag_performance_baseline() {
        let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
            amipro_formats::import_sam_path(path).unwrap().document
        });
        let fixture = Fixture::new(document);
        let layout = fixture.pages.layout();
        let (page_index, frame_box) = layout
            .pages
            .iter()
            .enumerate()
            .find_map(|(index, page)| {
                page.frames
                    .iter()
                    .find(|frame_box| frame_editing::selectable(&fixture.document, frame_box.frame))
                    .map(|frame_box| (index, frame_box.clone()))
            })
            .expect("a document with a page frame");
        let view = fixture.view(Zoom::Percent(100));
        let page_top = view.pages[page_index].y;
        let selection = FrameSelection::one(frame_box.frame, layout.pages[page_index].number);
        let caret = PageSelection::start(&fixture.document);
        let mut surface = fixture.framed(view.clone(), caret, Some(&selection));
        // Scrolled so that the frame's page fills the viewport.
        let mut driver = Driver::new(&mut surface, Vector::new(0.0, 20.0 - page_top));
        let start = view.point(
            page_index,
            frame_box.bounds.x + frame_box.bounds.width / 2.0,
            frame_box.bounds.y + frame_box.bounds.height / 2.0,
        );
        let visible: Vec<usize> = view
            .visible(Rectangle::new(
                Point::new(0.0, page_top - 20.0),
                Size::new(900.0, 700.0),
            ))
            .collect();
        assert!(visible.contains(&page_index), "the frame's page is drawn");
        driver.draw(&surface);
        driver.send(&mut surface, press(), start);
        let (mut update, mut draw, mut total) = (Vec::new(), Vec::new(), Vec::new());
        for step in 0..200 {
            let to = start + Vector::new((step % 40) as f32 * 3.0, (step / 40) as f32 * 4.0);
            let started = std::time::Instant::now();
            driver.send(&mut surface, moved(to + driver.offset), to);
            let updated = started.elapsed();
            driver.draw(&surface);
            total.push(started.elapsed());
            update.push(updated);
            draw.push(total.last().unwrap().saturating_sub(updated));
        }
        assert!(driver.state().frame_drag.is_some_and(|drag| drag.moved));
        let summary = |samples: &mut Vec<std::time::Duration>| {
            samples.sort_unstable();
            let at = |percent: usize| {
                samples[(samples.len() - 1) * percent / 100].as_secs_f64() * 1_000.0
            };
            format!(
                "median {:.3} ms, p95 {:.3} ms, worst {:.3} ms",
                at(50),
                at(95),
                samples.last().unwrap().as_secs_f64() * 1_000.0
            )
        };
        eprintln!(
            "frame drag on page {} ({} pages, pages drawn {visible:?}, {:?} frame): total {}; update {}; draw {}",
            layout.pages[page_index].number,
            layout.pages.len(),
            frame_box.kind,
            summary(&mut total),
            summary(&mut update),
            summary(&mut draw)
        );
    }
}
