//! Drawing Properties in the context rail (`UI_SPEC.md` §33, DW2): Ami
//! Pro's Line Style, Fill Pattern, Rotate, and Grid Settings dialogs, and
//! the run's exact position and size (DW5), beside the drawing while Draw
//! mode is open. Each change applies at once to the selected objects as
//! one step, and with nothing selected sets the look of new objects; a
//! typed value applies on Enter or when its box loses focus.

use std::collections::BTreeMap;

use amipro_editing::drawing::{Align, StyleChange};
use amipro_pictures::sdw::{self, Colour, Fill, Object, Pen, Shape};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::colour_control::{Context, Target};
use crate::draw_mode::{DrawMode, Grid};

/// The rail's width with Drawing Properties.
pub const WIDTH: f32 = 300.0;

/// A box the user types a value into.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Field {
    Left,
    Top,
    Width,
    Height,
    Step,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyAction {
    Width(u8),
    Dash(u8),
    Start(u8),
    End(u8),
    Pattern(u16),
    Text(Field, String),
    Submit(Field),
    KeepRatio(bool),
    Clockwise(bool),
    Align(Align),
    Distribute(bool),
    Face(String),
    Size(i32),
    Bold(bool),
    Italic(bool),
    Underline(bool),
    ShowGrid(bool),
    Snap(bool),
    Spacing(i32),
    Dots(bool),
}

/// The panel's own state: what is being typed, and Keep proportions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawingProperties {
    pub texts: BTreeMap<Field, String>,
    pub keep_ratio: bool,
}

/// Ami Pro's Line Style list (`FRAMEPROC`): ½ pt, 1–12 pt, and no line;
/// the dotted entry is a dash style here.
pub const WIDTHS: [(u8, &str); 14] = [
    (0, "None"),
    (10, "½ pt"),
    (20, "1 pt"),
    (40, "2 pt"),
    (60, "3 pt"),
    (80, "4 pt"),
    (100, "5 pt"),
    (120, "6 pt"),
    (140, "7 pt"),
    (160, "8 pt"),
    (180, "9 pt"),
    (200, "10 pt"),
    (220, "11 pt"),
    (240, "12 pt"),
];

const DASHES: [(u8, &str); 5] = [
    (0, "Solid"),
    (1, "Dashes"),
    (2, "Dots"),
    (3, "Dash dot"),
    (4, "Dash dot dot"),
];

/// Ami Pro's ten endings (`FRAMEPROC`, DRW2).
const ENDINGS: [&str; 10] = [
    "None",
    "Arrow",
    "Notched arrow",
    "Open arrow",
    "Tail",
    "Notched tail",
    "Open tail",
    "Bar",
    "Square",
    "Dot",
];

/// The fill patterns (`FILLPROC`, DRW1), in Ami Pro's numbers.
const PATTERNS: [(u16, &str); 12] = [
    (0, "None"),
    (5, "Solid"),
    (1, "Dots 12½ %"),
    (2, "Dots 25 %"),
    (3, "Dots 50 %"),
    (4, "Dots 75 %"),
    (6, "Horizontal"),
    (7, "Vertical"),
    (8, "Diagonal \\"),
    (9, "Diagonal /"),
    (10, "Cross"),
    (11, "Diagonal cross"),
];

/// Grid Settings' spacings (Fine, Medium, Coarse).
const SPACINGS: [(i32, &str); 3] = [
    (180, "Fine (⅛ in)"),
    (360, "Medium (¼ in)"),
    (720, "Coarse (½ in)"),
];

const SIZES: [i32; 16] = [6, 8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72];

fn send(action: PropertyAction) -> Message {
    Message::DrawProperties(action)
}

/// A document colour (`0x00BBGGRR`) and back.
pub fn to_document(colour: Colour) -> amipro_document::Colour {
    let [r, g, b] = colour.rgb_bytes();
    u32::from(r) | u32::from(g) << 8 | u32::from(b) << 16
}

pub fn from_document(colour: amipro_document::Colour) -> Colour {
    Colour::rgb(
        (colour & 0xFF) as u8,
        ((colour >> 8) & 0xFF) as u8,
        ((colour >> 16) & 0xFF) as u8,
    )
}

/// What the panel shows: the first selected object's look, else the
/// current look for new objects.
pub struct Shown {
    pub pen: Pen,
    pub fill: Fill,
    pub endings: bool,
    pub filled: bool,
    pub text: Option<sdw::Text>,
    pub bounds: Option<sdw::Rect>,
    pub count: usize,
}

pub fn shown(mode: &DrawMode, drawing: &sdw::Drawing) -> Shown {
    let selected: Vec<&Object> = mode
        .selection
        .iter()
        .filter_map(|&index| drawing.objects.get(index))
        .collect();
    let first_pen = selected.iter().find_map(|object| object.pen().copied());
    let first_fill = selected.iter().find_map(|object| object.fill().copied());
    let text = selected.iter().find_map(|object| match &object.shape {
        Shape::Text(text) => Some(text.clone()),
        _ => None,
    });
    Shown {
        pen: first_pen.unwrap_or(mode.pen),
        fill: first_fill.unwrap_or(mode.fill),
        endings: selected.is_empty() || selected.iter().any(|object| object.has_endings()),
        filled: selected.is_empty() || first_fill.is_some(),
        text,
        bounds: amipro_editing::drawing::bounds(drawing, &mode.selection),
        count: selected.len(),
    }
}

impl DrawingProperties {
    /// The text a box shows: what is being typed, else the value.
    fn text(&self, field: Field, shown: &Shown, mode: &DrawMode) -> String {
        if let Some(text) = self.texts.get(&field) {
            return text.clone();
        }
        let inches = |twips: i32| crate::style_properties::inches(twips);
        match (field, shown.bounds) {
            (Field::Step, _) => format!("{}", mode.rotate_step.abs() / 10),
            (Field::Left, Some(rect)) => inches(i32::from(rect.left)),
            (Field::Top, Some(rect)) => inches(i32::from(rect.top)),
            (Field::Width, Some(rect)) => inches(rect.width()),
            (Field::Height, Some(rect)) => inches(rect.height()),
            _ => String::new(),
        }
    }

    /// The selection's new rectangle from a typed box, or the rotate step;
    /// `None` when the text does not read.
    pub fn submitted(&self, field: Field, shown: &Shown) -> Option<Submitted> {
        let text = self.texts.get(&field)?;
        if field == Field::Step {
            let degrees: i32 = text.trim().trim_end_matches('°').trim().parse().ok()?;
            return (0..=359)
                .contains(&degrees)
                .then_some(Submitted::Step(degrees * 10));
        }
        let value = signed_length(text)?;
        let rect = shown.bounds?;
        let (l, t, w, h) = (
            f64::from(rect.left),
            f64::from(rect.top),
            f64::from(rect.width().max(1)),
            f64::from(rect.height().max(1)),
        );
        let value = f64::from(value);
        let (l, t, w, h) = match field {
            Field::Left => (value, t, w, h),
            Field::Top => (l, value, w, h),
            Field::Width if self.keep_ratio => (l, t, value, h * value / w),
            Field::Width => (l, t, value, h),
            Field::Height if self.keep_ratio => (l, t, w * value / h, value),
            _ => (l, t, w, value),
        };
        (w >= 1.0 && h >= 1.0).then_some(Submitted::Place((l, t, l + w, t + h)))
    }
}

/// A position (which may be negative) or a size, as the boxes read one.
fn signed_length(text: &str) -> Option<i32> {
    let text = text.trim();
    match text.strip_prefix('-') {
        Some(rest) => crate::lengths::parse_length(rest).map(|twips| -twips),
        None => crate::lengths::parse_length(text),
    }
}

pub enum Submitted {
    Place((f64, f64, f64, f64)),
    Step(i32),
}

/// The change a choice in the panel makes.
pub fn change(action: &PropertyAction) -> Option<StyleChange> {
    let mut change = StyleChange::default();
    match action {
        PropertyAction::Width(width) => change.width = Some(*width),
        PropertyAction::Dash(style) => change.style = Some(*style),
        PropertyAction::Start(start) => change.start = Some(*start),
        PropertyAction::End(end) => change.end = Some(*end),
        PropertyAction::Pattern(pattern) => change.pattern = Some(*pattern),
        PropertyAction::Face(face) => change.face = Some(face.clone()),
        PropertyAction::Size(points) => change.height = Some(-(points * 20) as i16),
        PropertyAction::Bold(on) => change.bold = Some(*on),
        PropertyAction::Italic(on) => change.italic = Some(*on),
        PropertyAction::Underline(on) => change.underline = Some(*on),
        _ => return None,
    }
    Some(change)
}

/// A grid choice.
pub fn grid(grid: &mut Grid, action: &PropertyAction) -> bool {
    match action {
        PropertyAction::ShowGrid(on) => grid.show = *on,
        PropertyAction::Snap(on) => grid.snap = *on,
        PropertyAction::Spacing(spacing) => grid.spacing = *spacing,
        PropertyAction::Dots(on) => grid.dots = *on,
        _ => return false,
    }
    true
}

fn heading(text: &'static str) -> Element<'static, Message> {
    widget::text::heading(text).into()
}

fn row<'a>(label: &'static str, control: Element<'a, Message>) -> Element<'a, Message> {
    widget::row::with_children(vec![
        widget::text::body(label).width(Length::Fixed(84.0)).into(),
        control,
    ])
    .spacing(6)
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn choice<T: Copy + PartialEq + Send + Sync + 'static>(
    options: &'static [(T, &'static str)],
    current: T,
    enabled: bool,
    action: fn(T) -> PropertyAction,
) -> Element<'static, Message> {
    let labels: Vec<String> = options
        .iter()
        .map(|(_, label)| (*label).to_owned())
        .collect();
    let chosen = options.iter().position(|(value, _)| *value == current);
    let dropdown = widget::dropdown(labels, chosen, move |index| send(action(options[index].0)))
        .width(Length::Fill);
    if enabled {
        dropdown.into()
    } else {
        widget::container(widget::text::body(
            chosen.map_or("—", |index| options[index].1),
        ))
        .into()
    }
}

fn input<'a>(
    state: &DrawingProperties,
    shown: &Shown,
    mode: &DrawMode,
    field: Field,
    enabled: bool,
) -> Element<'a, Message> {
    let mut box_ =
        widget::text_input("", state.text(field, shown, mode)).width(Length::Fixed(64.0));
    if enabled {
        box_ = box_
            .on_input(move |text| send(PropertyAction::Text(field, text)))
            .on_submit(move |_| send(PropertyAction::Submit(field)))
            .on_unfocus(send(PropertyAction::Submit(field)));
    }
    box_.into()
}

pub fn panel<'a>(
    state: &DrawingProperties,
    mode: &DrawMode,
    drawing: &sdw::Drawing,
    colours: Context<'a>,
) -> Element<'a, Message> {
    let shown = shown(mode, drawing);
    let header = widget::row::with_children(vec![
        widget::text::heading("Drawing Properties").into(),
        widget::space::horizontal().into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut body: Vec<Element<'a, Message>> = vec![
        widget::text::caption(match shown.count {
            0 => "Nothing selected: the look of new objects".to_owned(),
            1 => "1 object selected".to_owned(),
            count => format!("{count} objects selected"),
        })
        .into(),
        heading("Line"),
    ];
    let width = WIDTHS
        .iter()
        .map(|(value, _)| *value)
        .min_by_key(|value| (i32::from(*value) - i32::from(shown.pen.width)).abs())
        .unwrap_or(20);
    body.push(row(
        "Width",
        choice(&WIDTHS, width, true, PropertyAction::Width),
    ));
    body.push(row(
        "Dashes",
        choice(&DASHES, shown.pen.style.min(4), true, PropertyAction::Dash),
    ));
    body.push(row(
        "Colour",
        colours.control(Target::DrawLine, to_document(shown.pen.colour)),
    ));
    let endings: &'static [(u8, &'static str)] = &ENDING_CHOICES;
    body.push(row(
        "Start",
        choice(
            endings,
            shown.pen.start,
            shown.endings,
            PropertyAction::Start,
        ),
    ));
    body.push(row(
        "End",
        choice(endings, shown.pen.end, shown.endings, PropertyAction::End),
    ));
    body.push(heading("Fill"));
    body.push(row(
        "Pattern",
        choice(
            &PATTERNS,
            shown.fill.pattern,
            shown.filled,
            PropertyAction::Pattern,
        ),
    ));
    body.push(row(
        "Colour",
        colours.control_enabled(
            Target::DrawFill,
            to_document(shown.fill.foreground),
            shown.filled,
        ),
    ));
    body.push(row(
        "Background",
        colours.control_enabled(
            Target::DrawBackground,
            to_document(shown.fill.background),
            shown.filled,
        ),
    ));
    body.push(heading("Position and size"));
    let placed = shown.bounds.is_some();
    for (field, label) in [
        (Field::Left, "From left"),
        (Field::Top, "From top"),
        (Field::Width, "Width"),
        (Field::Height, "Height"),
    ] {
        body.push(row(
            label,
            widget::row::with_children(vec![
                input(state, &shown, mode, field, placed),
                widget::text::body("in").into(),
            ])
            .spacing(4)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        ));
    }
    let keep = widget::checkbox(state.keep_ratio).label("Keep proportions");
    body.push(
        keep.on_toggle(|on| send(PropertyAction::KeepRatio(on)))
            .into(),
    );
    let command = |id: crate::command::CommandId, label: &'static str, enabled: bool| {
        widget::button::standard(label)
            .on_press_maybe(enabled.then_some(Message::Command(id)))
            .into()
    };
    body.push(
        widget::row::with_children(vec![
            command(crate::command::CommandId::DrawRotateLeft, "⟲ 90°", placed),
            command(crate::command::CommandId::DrawRotateRight, "⟳ 90°", placed),
        ])
        .spacing(6)
        .into(),
    );
    body.push(row(
        "Rotate by",
        widget::row::with_children(vec![
            input(state, &shown, mode, Field::Step, true),
            widget::text::body("°").into(),
            widget::checkbox(mode.rotate_step >= 0)
                .label("Clockwise")
                .on_toggle(|on| send(PropertyAction::Clockwise(on)))
                .into(),
        ])
        .spacing(4)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    ));
    body.push(row(
        "Align",
        widget::row::with_children(
            [
                (Align::Left, "L"),
                (Align::Centre, "C"),
                (Align::Right, "R"),
                (Align::Top, "T"),
                (Align::Middle, "M"),
                (Align::Bottom, "B"),
            ]
            .into_iter()
            .map(|(how, label)| {
                widget::button::text(label)
                    .padding([2, 6])
                    .on_press_maybe((shown.count >= 2).then_some(send(PropertyAction::Align(how))))
                    .into()
            })
            .collect::<Vec<_>>(),
        )
        .spacing(2)
        .into(),
    ));
    body.push(row(
        "Distribute",
        widget::row::with_children(vec![
            widget::button::text("Across")
                .padding([2, 6])
                .on_press_maybe(
                    (shown.count >= 3).then_some(send(PropertyAction::Distribute(true))),
                )
                .into(),
            widget::button::text("Down")
                .padding([2, 6])
                .on_press_maybe(
                    (shown.count >= 3).then_some(send(PropertyAction::Distribute(false))),
                )
                .into(),
        ])
        .spacing(2)
        .into(),
    ));
    body.push(heading("Text"));
    let (face, height, colour, attributes) = match &shown.text {
        Some(text) => (text.face.clone(), text.height, text.colour, text.attributes),
        None => (
            mode.text.face.clone(),
            mode.text.height,
            mode.text.colour,
            mode.text.attributes,
        ),
    };
    let faces = crate::draw_commands::draw_faces(&face);
    let chosen = faces.iter().position(|candidate| *candidate == face);
    let list = faces.clone();
    body.push(row(
        "Face",
        widget::dropdown(faces, chosen, move |index| {
            send(PropertyAction::Face(list[index].clone()))
        })
        .width(Length::Fill)
        .into(),
    ));
    let points = i32::from(height.unsigned_abs()) / 20;
    body.push(row(
        "Size",
        widget::dropdown(
            SIZES
                .iter()
                .map(|size| format!("{size} pt"))
                .collect::<Vec<_>>(),
            SIZES.iter().position(|&size| size == points),
            |index| send(PropertyAction::Size(SIZES[index])),
        )
        .width(Length::Fill)
        .into(),
    ));
    body.push(row(
        "Colour",
        colours.control(Target::DrawText, to_document(colour)),
    ));
    body.push(
        widget::row::with_children(vec![
            widget::checkbox(attributes & sdw::Text::BOLD != 0)
                .label("Bold")
                .on_toggle(|on| send(PropertyAction::Bold(on)))
                .into(),
            widget::checkbox(attributes & sdw::Text::ITALIC != 0)
                .label("Italic")
                .on_toggle(|on| send(PropertyAction::Italic(on)))
                .into(),
            widget::checkbox(attributes & (sdw::Text::UNDERLINE | sdw::Text::WORD_UNDERLINE) != 0)
                .label("Underline")
                .on_toggle(|on| send(PropertyAction::Underline(on)))
                .into(),
        ])
        .spacing(8)
        .wrap()
        .into(),
    );
    body.push(heading("Grid"));
    body.push(
        widget::row::with_children(vec![
            widget::checkbox(mode.grid.show)
                .label("Show")
                .on_toggle(|on| send(PropertyAction::ShowGrid(on)))
                .into(),
            widget::checkbox(mode.grid.snap)
                .label("Snap to")
                .on_toggle(|on| send(PropertyAction::Snap(on)))
                .into(),
        ])
        .spacing(12)
        .into(),
    );
    body.push(row(
        "Spacing",
        choice(&SPACINGS, mode.grid.spacing, true, PropertyAction::Spacing),
    ));
    body.push(row(
        "Shown as",
        choice(&GRID_STYLES, mode.grid.dots, true, PropertyAction::Dots),
    ));
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(6).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

const GRID_STYLES: [(bool, &str); 2] = [(true, "Dots"), (false, "Dotted lines")];

const ENDING_CHOICES: [(u8, &str); 10] = [
    (0, ENDINGS[0]),
    (1, ENDINGS[1]),
    (2, ENDINGS[2]),
    (3, ENDINGS[3]),
    (4, ENDINGS[4]),
    (5, ENDINGS[5]),
    (6, ENDINGS[6]),
    (7, ENDINGS[7]),
    (8, ENDINGS[8]),
    (9, ENDINGS[9]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_sizes_place_the_selection_keeping_proportions() {
        let mut state = DrawingProperties::default();
        let shown = Shown {
            pen: Pen::default(),
            fill: Fill::default(),
            endings: false,
            filled: true,
            text: None,
            bounds: Some(sdw::Rect::new(0, 0, 1440, 720)),
            count: 1,
        };
        state.texts.insert(Field::Width, "2".into());
        let Some(Submitted::Place(rect)) = state.submitted(Field::Width, &shown) else {
            panic!();
        };
        assert_eq!(rect, (0.0, 0.0, 2880.0, 720.0));
        state.keep_ratio = true;
        let Some(Submitted::Place(rect)) = state.submitted(Field::Width, &shown) else {
            panic!();
        };
        assert_eq!(rect, (0.0, 0.0, 2880.0, 1440.0));
        state.texts.insert(Field::Left, "-0.5".into());
        let Some(Submitted::Place(rect)) = state.submitted(Field::Left, &shown) else {
            panic!();
        };
        assert_eq!(rect.0, -720.0);
        state.texts.insert(Field::Step, "45".into());
        assert!(matches!(
            state.submitted(Field::Step, &shown),
            Some(Submitted::Step(450))
        ));
        assert_eq!(
            from_document(to_document(Colour::rgb(1, 2, 3))),
            Colour::rgb(1, 2, 3)
        );
    }
}
