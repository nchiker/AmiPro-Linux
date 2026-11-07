//! The Chart panel in the rail (`docs/Design/UI_SPEC.md` §42.4, decisions
//! OD3 to OD6 and OD11; run D, DO.8): Ami Pro's Charting dialog (the twelve
//! type icons, Legend, Grid, 3D with Depth and Perspective, the colour set,
//! the Axis group, Flip data, Data…, Pictures…) as a panel beside the live
//! page. Every control writes through [`ChartAction`]; the app regenerates
//! the chart's drawing (`amipro_pictures::chart_gen`) and sets it on the
//! frame as one undo step, so the page shows each change at once.
//!
//! Values are typed where they are shown: the Data grid's cells, the Depth
//! and Axis boxes. A text box applies on Enter or when it loses the focus;
//! the Data text (the form Ami Pro's Charting Data box reads) applies with
//! Apply or when anything else is used.

use std::collections::BTreeMap;

use amipro_document::{FrameId, ObjectId};
use amipro_pictures::chart::{
    Axis, Chart, ChartType, Data, DataError, DataFormat, Icon, Label, MAX_CELLS, parse_data,
};
use amipro_pictures::chart_gen::{self, Confidence};
use cosmic::iced::Length;
use cosmic::iced::widget::text_editor;
use cosmic::widget;

use crate::Message;
use crate::colour_control::{self, Target};

/// The panel's width in the rail.
pub const WIDTH: f32 = 340.0;
/// The most grid rows and columns drawn; the Data text holds all of them.
pub const GRID_ROWS: usize = 30;
pub const GRID_COLUMNS: usize = 10;
/// A frame must be larger than this each way to hold a chart (message
/// 4012; `LIBOPEN` tests 0x2d1 twips).
pub const MIN_FRAME: i32 = 0x2d1;
/// What the panel says of a chart whose record is not in the known layout
/// (OD9).
pub const UNREADABLE: &str =
    "This chart's data record is not in a form the app can edit; it is kept as it is.";
/// What the panel says when the first look at the clipboard found no
/// tab-separated text (message 4011, without its question).
pub const SAMPLE_NOTE: &str =
    "The clipboard held no tab-separated data, so the chart starts with sample data.";

type Element<'a> = cosmic::Element<'a, Message>;

// ------------------------------------------------------------------- icons

/// The icon's name in the panel (Ami Pro's order, ids 2200 to 2211).
pub fn name(icon: Icon) -> &'static str {
    match icon {
        Icon::Column => "Column",
        Icon::StackedColumn => "Stacked Column",
        Icon::Bar => "Bar",
        Icon::StackedBar => "Stacked Bar",
        Icon::Line => "Line",
        Icon::Area => "Area",
        Icon::LinePicture => "Line and Picture",
        Icon::LineCategoryPicture => "Line and Picture by Category",
        Icon::Pie => "Pie",
        Icon::ExpandedPie => "Expanded Pie",
        Icon::Picture => "Picture",
        Icon::StackedPicture => "Stacked Picture",
    }
}

/// The app's own drawing of each type.
pub fn svg(icon: Icon) -> &'static [u8] {
    match icon {
        Icon::Column => include_bytes!("../icons/chart-column-symbolic.svg"),
        Icon::StackedColumn => include_bytes!("../icons/chart-stacked-column-symbolic.svg"),
        Icon::Bar => include_bytes!("../icons/chart-bar-symbolic.svg"),
        Icon::StackedBar => include_bytes!("../icons/chart-stacked-bar-symbolic.svg"),
        Icon::Line => include_bytes!("../icons/chart-line-symbolic.svg"),
        Icon::Area => include_bytes!("../icons/chart-area-symbolic.svg"),
        Icon::LinePicture => include_bytes!("../icons/chart-line-picture-symbolic.svg"),
        Icon::LineCategoryPicture => {
            include_bytes!("../icons/chart-line-picture-category-symbolic.svg")
        }
        Icon::Pie => include_bytes!("../icons/chart-pie-symbolic.svg"),
        Icon::ExpandedPie => include_bytes!("../icons/chart-expanded-pie-symbolic.svg"),
        Icon::Picture => include_bytes!("../icons/chart-picture-symbolic.svg"),
        Icon::StackedPicture => include_bytes!("../icons/chart-stacked-picture-symbolic.svg"),
    }
}

/// 3D and Depth are enabled for columns, bars, areas, and pies (`1058:09ac`).
pub fn supports_3d(icon: Icon) -> bool {
    matches!(
        icon,
        Icon::Column
            | Icon::StackedColumn
            | Icon::Bar
            | Icon::StackedBar
            | Icon::Area
            | Icon::Pie
            | Icon::ExpandedPie
    )
}

/// The Axis group is enabled for every type but the pies.
pub fn has_axis(icon: Icon) -> bool {
    !matches!(icon, Icon::Pie | Icon::ExpandedPie)
}

pub fn is_pie(icon: Icon) -> bool {
    matches!(icon, Icon::Pie | Icon::ExpandedPie)
}

/// Pictures… is for the line and picture and the picture types.
pub fn has_pictures(icon: Icon) -> bool {
    matches!(
        icon,
        Icon::LinePicture | Icon::LineCategoryPicture | Icon::Picture | Icon::StackedPicture
    )
}

/// The icon a chart's record stands for (the type, the variation, and the
/// expanded-pie bit).
pub fn icon_of(chart: &Chart) -> Icon {
    match chart.chart_type {
        ChartType::Column if chart.variation == 0 => Icon::StackedColumn,
        ChartType::Column | ChartType::Other(_) => Icon::Column,
        ChartType::Bar if chart.variation == 0 => Icon::StackedBar,
        ChartType::Bar => Icon::Bar,
        ChartType::Line if chart.variation == 6 => Icon::Area,
        ChartType::Line => Icon::Line,
        ChartType::LinePicture if chart.variation == 3 => Icon::LineCategoryPicture,
        ChartType::LinePicture => Icon::LinePicture,
        ChartType::Pie if chart.last_word & 0x0100 != 0 => Icon::ExpandedPie,
        ChartType::Pie => Icon::Pie,
        ChartType::Picture if chart.variation == 0 => Icon::StackedPicture,
        ChartType::Picture => Icon::Picture,
    }
}

/// The two variations of the icon's family, as (icon, label), when it has
/// two.
pub fn variations(icon: Icon) -> Option<[(Icon, &'static str); 2]> {
    match icon {
        Icon::Column | Icon::StackedColumn => Some([
            (Icon::Column, "Clustered"),
            (Icon::StackedColumn, "Stacked"),
        ]),
        Icon::Bar | Icon::StackedBar => {
            Some([(Icon::Bar, "Clustered"), (Icon::StackedBar, "Stacked")])
        }
        Icon::Line | Icon::Area => Some([(Icon::Line, "Lines"), (Icon::Area, "Areas")]),
        Icon::LinePicture | Icon::LineCategoryPicture => Some([
            (Icon::LinePicture, "One marker per series"),
            (Icon::LineCategoryPicture, "A marker per category"),
        ]),
        Icon::Pie | Icon::ExpandedPie => {
            Some([(Icon::Pie, "Whole"), (Icon::ExpandedPie, "Expanded")])
        }
        Icon::Picture | Icon::StackedPicture => Some([
            (Icon::Picture, "Clustered"),
            (Icon::StackedPicture, "Stacked"),
        ]),
    }
}

// -------------------------------------------------------------------- data

/// The data a new chart starts with (CHV2's, which the generator is
/// checked against).
pub const SAMPLE: &str = "\tJan\tFeb\tMar\nEast\t10\t20\t30\nWest\t15\t5\t25\n";

/// A new column chart from `data`, or from the sample when there is none.
pub fn new_chart(data: Option<Data>) -> Chart {
    let data = data.unwrap_or_else(|| {
        parse_data(SAMPLE, &DataFormat::default()).expect("the sample data parses")
    });
    let mut chart = Chart::new(Icon::Column, data.columns, data.rows);
    chart.set_data(data);
    chart
}

/// Clipboard text taken as data: it needs a tab (tab-separated, as a
/// spreadsheet or a table copies) and numbers.
pub fn clipboard_data(text: &str) -> Option<Data> {
    if !text.contains('\t') {
        return None;
    }
    parse_data(text, &DataFormat::default()).ok()
}

/// What the Data box says of a refused text (messages 4011 and 4013).
pub fn data_problem(error: DataError) -> &'static str {
    match error {
        DataError::NoData => "No numbers to chart: enter at least one number.",
        DataError::TooMuch => "Too much data: a chart holds at most 5,000 values.",
    }
}

/// A value typed in a grid cell.
pub fn parse_value(text: &str) -> Result<f32, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(0.0);
    }
    let token = text.replace(['\t', '\r', '\n'], " ");
    match parse_data(&format!("x\t{token}"), &DataFormat::default()) {
        Ok(data) if data.columns == 1 && data.rows == 1 => Ok(data.values[0]),
        _ => Err(format!("\"{text}\" is not a number.")),
    }
}

/// The chart's data as comma-separated values (OD11): the series name and
/// values a line, the categories on the first.
pub fn csv(chart: &Chart) -> String {
    fn field(text: &str) -> String {
        if text.contains([',', '"', '\n']) {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_owned()
        }
    }
    let format = DataFormat::default();
    let mut out = String::new();
    let mut header = vec![String::new()];
    header.extend(chart.column_labels.iter().map(|l| field(&l.shown())));
    out.push_str(&header.join(","));
    out.push('\n');
    for row in 0..chart.rows {
        let mut cells = vec![field(&chart.row_labels[row].shown())];
        for column in 0..chart.columns {
            cells.push(amipro_pictures::chart::format_value(
                chart.value(row, column),
                &format,
            ));
        }
        out.push_str(&cells.join(","));
        out.push('\n');
    }
    out
}

/// The chart with `cells` (grid coordinates: row 0 holds the column labels,
/// column 0 the series names) applied; the first value that is not a
/// number refuses all of them.
pub fn with_cells(
    chart: &Chart,
    cells: &BTreeMap<(usize, usize), String>,
) -> Result<Chart, String> {
    let mut chart = chart.clone();
    for (&(row, column), text) in cells {
        match (row, column) {
            (0, 0) => {}
            (0, column) => {
                if let Some(label) = chart.column_labels.get_mut(column - 1) {
                    *label = Label::new(text.trim());
                }
            }
            (row, 0) => {
                if let Some(label) = chart.row_labels.get_mut(row - 1) {
                    *label = Label::new(text.trim());
                }
            }
            (row, column) => {
                if row - 1 < chart.rows && column - 1 < chart.columns {
                    let value = parse_value(text)?;
                    chart.values[(row - 1) * chart.columns + column - 1] = value;
                }
            }
        }
    }
    chart.flags = vec![0; chart.values.len()];
    Ok(chart)
}

/// `chart` with one more series (a row of zeros).
pub fn add_series(chart: &mut Chart) -> bool {
    if chart.columns * (chart.rows + 1) > MAX_CELLS {
        return false;
    }
    chart.rows += 1;
    chart.values.extend(std::iter::repeat_n(0.0, chart.columns));
    chart.flags.extend(std::iter::repeat_n(0, chart.columns));
    chart.row_labels.push(Label::default());
    true
}

/// `chart` with one more category (a column of zeros).
pub fn add_category(chart: &mut Chart) -> bool {
    if (chart.columns + 1) * chart.rows > MAX_CELLS {
        return false;
    }
    let mut values = Vec::with_capacity((chart.columns + 1) * chart.rows);
    for row in 0..chart.rows {
        values.extend_from_slice(&chart.values[row * chart.columns..(row + 1) * chart.columns]);
        values.push(0.0);
    }
    chart.columns += 1;
    chart.flags = vec![0; values.len()];
    chart.values = values;
    chart.column_labels.push(Label::default());
    true
}

/// `chart` without its last series; a chart keeps at least one.
pub fn remove_series(chart: &mut Chart) -> bool {
    if chart.rows <= 1 {
        return false;
    }
    chart.rows -= 1;
    chart.values.truncate(chart.rows * chart.columns);
    chart.flags.truncate(chart.rows * chart.columns);
    chart.row_labels.truncate(chart.rows);
    true
}

/// `chart` without its last category; a chart keeps at least one.
pub fn remove_category(chart: &mut Chart) -> bool {
    if chart.columns <= 1 {
        return false;
    }
    let mut values = Vec::with_capacity((chart.columns - 1) * chart.rows);
    for row in 0..chart.rows {
        values.extend_from_slice(
            &chart.values[row * chart.columns..row * chart.columns + chart.columns - 1],
        );
    }
    chart.columns -= 1;
    chart.flags = vec![0; values.len()];
    chart.values = values;
    chart.column_labels.truncate(chart.columns);
    true
}

// -------------------------------------------------------------------- axis

/// The Axis group's boxes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisField {
    Inc,
    Min,
    Max,
}

/// The Axis group as typed: Automatic, or Inc, Min, and Max.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AxisForm {
    pub automatic: bool,
    pub inc: String,
    pub min: String,
    pub max: String,
    /// What is wrong with the boxes.
    pub error: Option<String>,
    /// Typed and not yet applied.
    pub edited: bool,
}

/// An axis value as the boxes show it.
pub fn axis_text(value: f64) -> String {
    format!("{value}")
}

impl AxisForm {
    pub fn from_axis(automatic: bool, axis: Axis) -> Self {
        Self {
            automatic,
            inc: axis_text(axis.step),
            min: axis_text(axis.min),
            max: axis_text(axis.max),
            error: None,
            edited: false,
        }
    }

    /// The axis the boxes make, or what is wrong with them.
    pub fn parse(&self) -> Result<Axis, String> {
        let number = |text: &str, what: &str| -> Result<f64, String> {
            text.trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| format!("{what} must be a number."))
        };
        let step = number(&self.inc, "Inc")?;
        let min = number(&self.min, "Min")?;
        let max = number(&self.max, "Max")?;
        if step <= 0.0 {
            return Err("Inc must be above 0.".into());
        }
        if max <= min {
            return Err("Max must be above Min.".into());
        }
        if (max - min) / step > 100.0 {
            return Err("That axis would have more than 100 steps.".into());
        }
        Ok(Axis { min, max, step })
    }

    pub fn field_mut(&mut self, field: AxisField) -> &mut String {
        match field {
            AxisField::Inc => &mut self.inc,
            AxisField::Min => &mut self.min,
            AxisField::Max => &mut self.max,
        }
    }
}

// ------------------------------------------------------------------- state

/// What the panel's frame holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// A chart the app can edit.
    Chart,
    /// A chart frame whose data record is not in the known layout (OD9).
    Unreadable,
    /// A frame holding something else (a drawing, a picture) that Charting
    /// replaces once the user agrees.
    Replace(&'static str),
    /// A new chart waiting for the clipboard's answer (no frame yet).
    Starting,
}

/// A line under the header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Notice {
    Info(String),
    Refused(String),
}

/// The open panel.
pub struct ChartPanel {
    /// The chart's frame; none while a new chart waits for the clipboard.
    pub frame: Option<FrameId>,
    pub kind: Kind,
    /// The drawing object the state below was read from; another one (an
    /// Undo, a change from elsewhere) reads it again.
    pub object: Option<ObjectId>,
    pub chart: Option<Chart>,
    pub axis: AxisForm,
    /// The Data text as typed, and whether it differs from the chart's.
    pub data: text_editor::Content,
    pub data_edited: bool,
    pub data_error: Option<&'static str>,
    /// Grid cells typed and not yet applied (grid coordinates).
    pub cells: BTreeMap<(usize, usize), String>,
    pub cell_error: Option<String>,
    /// The Depth box as typed.
    pub depth: Option<String>,
    /// The colour-set entry the colour button changes.
    pub colour: usize,
    pub notice: Option<Notice>,
}

impl ChartPanel {
    pub fn new(frame: Option<FrameId>, kind: Kind) -> Self {
        Self {
            frame,
            kind,
            object: None,
            chart: None,
            axis: AxisForm::default(),
            data: text_editor::Content::new(),
            data_edited: false,
            data_error: None,
            cells: BTreeMap::new(),
            cell_error: None,
            depth: None,
            colour: 0,
            notice: None,
        }
    }

    /// Takes `chart` (read from the drawing object `object`) as the state
    /// and fills the text boxes from it. `automatic` is whether its axis is
    /// the one Ami Pro computes.
    pub fn load(&mut self, object: Option<ObjectId>, chart: Option<Chart>, automatic: bool) {
        self.object = object;
        self.cells.clear();
        self.cell_error = None;
        self.depth = None;
        self.data_edited = false;
        self.data_error = None;
        match chart {
            Some(chart) => {
                self.data =
                    text_editor::Content::with_text(&chart.data_text(&DataFormat::default()));
                // The axis boxes keep what is typed when the chart still
                // has the axis they stand for.
                self.axis = AxisForm::from_axis(automatic, chart.axis);
                self.colour = self.colour.min(15);
                if self.kind != Kind::Chart {
                    self.kind = Kind::Chart;
                }
                self.chart = Some(chart);
            }
            None => self.chart = None,
        }
    }

    /// The Data text as typed (without the editor's closing newline).
    pub fn data_text(&self) -> String {
        self.data.text()
    }
}

// ----------------------------------------------------------------- actions

#[derive(Clone, Debug)]
pub enum ChartAction {
    Close,
    Icon(Icon),
    Legend(bool),
    Grid(bool),
    ThreeD(bool),
    Perspective(bool),
    /// The Depth slider, and its release.
    Depth(u16),
    DepthEnd,
    DepthText(String),
    DepthSubmit,
    SelectColour(usize),
    AxisAutomatic(bool),
    AxisText(AxisField, String),
    AxisSubmit,
    Flip,
    Cell {
        row: usize,
        column: usize,
        text: String,
    },
    CommitCells,
    DataEdit(text_editor::Action),
    ApplyData,
    AddSeries,
    AddCategory,
    RemoveSeries,
    RemoveCategory,
    CopyCsv,
    SaveImage,
    FromTable,
    Drawing,
    /// The question under the header (Replace, or a new chart for an
    /// unreadable one).
    ConfirmReplace,
    CancelReplace,
}

fn send(action: ChartAction) -> Message {
    Message::Chart(action)
}

// -------------------------------------------------------------------- view

fn header() -> Element<'static> {
    widget::row::with_children(vec![
        widget::text::heading("Chart").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close the Chart panel")
            .on_press(send(ChartAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn wide(label: &str, suggested: bool, message: Option<Message>) -> Element<'static> {
    widget::button::custom(
        widget::container(widget::text::body(label.to_owned())).center_x(Length::Fill),
    )
    .class(if suggested {
        cosmic::theme::Button::Suggested
    } else {
        cosmic::theme::Button::Standard
    })
    .width(Length::Fill)
    .on_press_maybe(message)
    .into()
}

fn small(label: &str, message: Option<Message>) -> Element<'static> {
    widget::button::standard(label.to_owned())
        .on_press_maybe(message)
        .into()
}

fn row(children: Vec<Element<'_>>) -> Element<'_> {
    widget::row::with_children(children)
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

fn section(title: &str) -> Element<'static> {
    widget::text::heading(title.to_owned()).into()
}

/// A palette entry as a document colour.
pub fn palette_colour(entry: [u8; 4]) -> u32 {
    u32::from(entry[0]) | u32::from(entry[1]) << 8 | u32::from(entry[2]) << 16
}

fn type_buttons<'a>(current: Icon) -> Element<'a> {
    let button = |icon: Icon| -> Element<'a> {
        let selected = icon == current;
        widget::button::icon(widget::icon::from_svg_bytes(svg(icon)).symbolic(true))
            .name(name(icon))
            .tooltip(name(icon))
            .selected(selected)
            .class(crate::toggle_button_class(selected))
            .width(36)
            .height(36)
            .on_press(send(ChartAction::Icon(icon)))
            .into()
    };
    let mut rows: Vec<Element<'a>> = Vec::new();
    for line in Icon::ALL.chunks(6) {
        rows.push(
            widget::row::with_children(line.iter().map(|icon| button(*icon)).collect::<Vec<_>>())
                .spacing(2)
                .into(),
        );
    }
    widget::column::with_children(rows).spacing(2).into()
}

fn check<'a>(
    label: &'static str,
    on: bool,
    enabled: bool,
    action: impl Fn(bool) -> ChartAction + 'a,
) -> Element<'a> {
    widget::checkbox(on)
        .label(label)
        .on_toggle_maybe(enabled.then_some(move |value| send(action(value))))
        .into()
}

fn variation_radios<'a>(icon: Icon) -> Option<Element<'a>> {
    let [first, second] = variations(icon)?;
    let radio = |(choice, label): (Icon, &'static str)| -> Element<'a> {
        widget::radio(widget::text::body(label), choice, Some(icon), |chosen| {
            send(ChartAction::Icon(chosen))
        })
        .into()
    };
    Some(
        widget::column::with_children(vec![radio(first), radio(second)])
            .spacing(2)
            .into(),
    )
}

fn grid<'a>(state: &'a ChartPanel, chart: &'a Chart) -> Element<'a> {
    let format = DataFormat::default();
    let rows = (chart.rows + 1).min(GRID_ROWS + 1);
    let columns = (chart.columns + 1).min(GRID_COLUMNS + 1);
    let mut lines: Vec<Element<'a>> = Vec::new();
    for r in 0..rows {
        let mut cells: Vec<Element<'a>> = Vec::new();
        for c in 0..columns {
            if r == 0 && c == 0 {
                cells.push(widget::space().width(Length::Fixed(58.0)).into());
                continue;
            }
            let shown = match (r, c) {
                (0, c) => chart.column_labels[c - 1].shown(),
                (r, 0) => chart.row_labels[r - 1].shown(),
                (r, c) => amipro_pictures::chart::format_value(chart.value(r - 1, c - 1), &format),
            };
            let value = state.cells.get(&(r, c)).cloned().unwrap_or(shown);
            let hint = match (r, c) {
                (0, _) => "category",
                (_, 0) => "series",
                _ => "0",
            };
            cells.push(
                widget::text_input(hint, value)
                    .on_input(move |text| {
                        send(ChartAction::Cell {
                            row: r,
                            column: c,
                            text,
                        })
                    })
                    .on_submit(|_| send(ChartAction::CommitCells))
                    .on_unfocus(send(ChartAction::CommitCells))
                    .width(Length::Fixed(58.0))
                    .into(),
            );
        }
        lines.push(widget::row::with_children(cells).spacing(2).into());
    }
    widget::scrollable(
        widget::column::with_children(lines)
            .spacing(2)
            .padding([0, 0, 8, 0]),
    )
    .direction(cosmic::iced::widget::scrollable::Direction::Both {
        vertical: cosmic::iced::widget::scrollable::Scrollbar::default(),
        horizontal: cosmic::iced::widget::scrollable::Scrollbar::default(),
    })
    .height(Length::Fixed((rows as f32 * 36.0 + 24.0).min(260.0)))
    .width(Length::Fill)
    .into()
}

/// What the panel needs besides its own state.
pub struct Context<'a> {
    pub colours: colour_control::Context<'a>,
    /// Whether the chart is in a state the app can say is only partly
    /// confirmed against Ami Pro.
    pub in_table: bool,
}

/// The panel.
pub fn panel<'a>(state: &'a ChartPanel, context: Context<'a>) -> Element<'a> {
    let mut body: Vec<Element<'a>> = vec![header()];
    match &state.notice {
        Some(Notice::Info(text)) => body.push(widget::text::caption(text.clone()).into()),
        Some(Notice::Refused(text)) => body.push(
            widget::text::body(text.clone())
                .class(cosmic::theme::Text::Accent)
                .into(),
        ),
        None => {}
    }
    match state.kind {
        Kind::Replace(what) => {
            body.push(
                widget::text::body(format!(
                    "This frame holds {what}. Charting replaces it with a chart."
                ))
                .into(),
            );
            body.push(wide(
                "Replace with a Chart",
                true,
                Some(send(ChartAction::ConfirmReplace)),
            ));
            body.push(wide(
                "Cancel",
                false,
                Some(send(ChartAction::CancelReplace)),
            ));
            return finish(body);
        }
        Kind::Starting => {
            body.push(widget::text::body("Looking at the clipboard for data…").into());
            return finish(body);
        }
        Kind::Unreadable => {
            body.push(widget::text::body(UNREADABLE).into());
            body.push(wide(
                "Replace with a New Chart",
                false,
                Some(send(ChartAction::ConfirmReplace)),
            ));
            body.push(wide("Drawing…", false, Some(send(ChartAction::Drawing))));
            return finish(body);
        }
        Kind::Chart => {}
    }
    let Some(chart) = state.chart.as_ref() else {
        body.push(widget::text::body("The chart is no longer there.").into());
        return finish(body);
    };
    let icon = icon_of(chart);

    // Type and variation.
    body.push(section("Type"));
    body.push(type_buttons(icon));
    body.push(widget::text::caption(name(icon)).into());
    if let Some(radios) = variation_radios(icon) {
        body.push(radios);
    }
    if has_pictures(icon) {
        body.push(
            widget::text::caption(
                "The app draws the markers and pictures itself; Ami Pro's picture list is not used.",
            )
            .into(),
        );
    }
    body.push(widget::divider::horizontal::default().into());

    // Options.
    body.push(section("Options"));
    let can_3d = supports_3d(icon);
    let three_d = chart.three_d() != 0 && can_3d;
    body.push(row(vec![
        check("Legend", chart.legend_on(), true, ChartAction::Legend),
        check("Grid", chart.grid_on(), true, ChartAction::Grid),
    ]));
    body.push(row(vec![
        check("3D", three_d, can_3d, ChartAction::ThreeD),
        check(
            "Perspective",
            chart.three_d() == 2 && !is_pie(icon),
            three_d && !is_pie(icon),
            ChartAction::Perspective,
        ),
    ]));
    if three_d {
        let depth = chart.depth().clamp(1, 100);
        let typed = state.depth.clone().unwrap_or_else(|| depth.to_string());
        body.push(row(vec![
            widget::text::body("Depth").into(),
            widget::text_input("50", typed)
                .on_input(|text| send(ChartAction::DepthText(text)))
                .on_submit(|_| send(ChartAction::DepthSubmit))
                .on_unfocus(send(ChartAction::DepthSubmit))
                .width(Length::Fixed(56.0))
                .into(),
            widget::slider(1..=100u16, depth, |value| send(ChartAction::Depth(value)))
                .on_release(send(ChartAction::DepthEnd))
                .width(Length::Fill)
                .into(),
        ]));
    }
    body.push(widget::divider::horizontal::default().into());

    // The colour set.
    body.push(section("Colour set"));
    let swatches: Vec<Element<'a>> = chart
        .palette
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            colour_control::swatch(
                palette_colour(*entry),
                index == state.colour,
                send(ChartAction::SelectColour(index)),
            )
        })
        .collect();
    body.push(
        widget::row::with_children(swatches)
            .spacing(1)
            .wrap()
            .into(),
    );
    body.push(row(vec![
        widget::text::body(format!("Colour {}", state.colour + 1)).into(),
        context.colours.control(
            Target::ChartColour,
            palette_colour(chart.palette[state.colour.min(15)]),
        ),
    ]));
    body.push(widget::divider::horizontal::default().into());

    // The axis.
    if has_axis(icon) {
        body.push(section("Axis"));
        body.push(check(
            "Automatic",
            state.axis.automatic,
            true,
            ChartAction::AxisAutomatic,
        ));
        let field = |label: &'static str, which: AxisField, value: &'a str| -> Element<'a> {
            let input = widget::text_input("", value).width(Length::Fixed(72.0));
            let input = if state.axis.automatic {
                input
            } else {
                input
                    .on_input(move |text| send(ChartAction::AxisText(which, text)))
                    .on_submit(|_| send(ChartAction::AxisSubmit))
                    .on_unfocus(send(ChartAction::AxisSubmit))
            };
            widget::column::with_children(vec![widget::text::caption(label).into(), input.into()])
                .spacing(2)
                .into()
        };
        body.push(row(vec![
            field("Inc", AxisField::Inc, &state.axis.inc),
            field("Min", AxisField::Min, &state.axis.min),
            field("Max", AxisField::Max, &state.axis.max),
        ]));
        if let Some(error) = &state.axis.error {
            body.push(
                widget::text::caption(error.clone())
                    .class(cosmic::theme::Text::Accent)
                    .into(),
            );
        }
        body.push(widget::divider::horizontal::default().into());
    }

    // Data.
    body.push(section("Data"));
    body.push(
        widget::text::caption(format!(
            "{} series by {} categories. Type in the cells; Enter applies.",
            chart.rows, chart.columns
        ))
        .into(),
    );
    body.push(grid(state, chart));
    if chart.rows > GRID_ROWS || chart.columns > GRID_COLUMNS {
        body.push(
            widget::text::caption("The grid shows the first values; the Data text below has all.")
                .into(),
        );
    }
    if let Some(error) = &state.cell_error {
        body.push(
            widget::text::caption(error.clone())
                .class(cosmic::theme::Text::Accent)
                .into(),
        );
    }
    body.push(row(vec![
        small("Add Series", Some(send(ChartAction::AddSeries))),
        small(
            "Remove",
            (chart.rows > 1).then(|| send(ChartAction::RemoveSeries)),
        ),
    ]));
    body.push(row(vec![
        small("Add Category", Some(send(ChartAction::AddCategory))),
        small(
            "Remove",
            (chart.columns > 1).then(|| send(ChartAction::RemoveCategory)),
        ),
    ]));
    body.push(wide("Flip Data", false, Some(send(ChartAction::Flip))));
    body.push(
        widget::text::caption(
            "Data text (tabs between values; labels in the first row and column)",
        )
        .into(),
    );
    body.push(
        text_editor(&state.data)
            .on_action(|action| send(ChartAction::DataEdit(action)))
            .height(Length::Fixed(130.0))
            .into(),
    );
    if let Some(error) = state.data_error {
        body.push(
            widget::text::caption(error)
                .class(cosmic::theme::Text::Accent)
                .into(),
        );
    }
    body.push(wide(
        "Apply Data",
        state.data_edited,
        (state.data_edited && state.data_error.is_none()).then(|| send(ChartAction::ApplyData)),
    ));
    body.push(widget::divider::horizontal::default().into());

    // The frame.
    if chart_gen::confidence(chart) == Confidence::Unconfirmed {
        body.push(
            widget::text::caption(
                "This kind of chart is drawn from the notes on Ami Pro's charts and has not been compared with Ami Pro's own drawing; it may differ a little.",
            )
            .into(),
        );
    }
    body.push(wide(
        "Copy Data as CSV",
        false,
        Some(send(ChartAction::CopyCsv)),
    ));
    body.push(wide(
        "Save Chart as Image…",
        false,
        Some(send(ChartAction::SaveImage)),
    ));
    if context.in_table {
        body.push(wide(
            "Chart from Table",
            false,
            Some(send(ChartAction::FromTable)),
        ));
    }
    body.push(wide("Drawing…", false, Some(send(ChartAction::Drawing))));
    body.push(
        widget::text::caption(
            "Drawing… opens the chart's pieces in Draw mode; a later change here generates the chart again.",
        )
        .into(),
    );
    finish(body)
}

fn finish<'a>(body: Vec<Element<'a>>) -> Element<'a> {
    widget::scrollable(
        widget::column::with_children(body)
            .spacing(6)
            .padding([8, 8]),
    )
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_maps_back_from_its_record() {
        for icon in Icon::ALL {
            let mut chart = Chart::new(Icon::Column, 3, 2);
            chart.set_icon(icon);
            assert_eq!(icon_of(&chart), icon, "{}", name(icon));
        }
    }

    #[test]
    fn the_enabling_rules_follow_the_dialog() {
        let three_d: Vec<_> = Icon::ALL.iter().filter(|i| supports_3d(**i)).collect();
        assert_eq!(three_d.len(), 7);
        assert!(!supports_3d(Icon::Line) && !supports_3d(Icon::Picture));
        assert!(has_axis(Icon::Line) && !has_axis(Icon::Pie) && !has_axis(Icon::ExpandedPie));
        assert!(has_pictures(Icon::Picture) && !has_pictures(Icon::Column));
    }

    #[test]
    fn grid_values_are_numbers_or_refused() {
        assert_eq!(parse_value(" 12.5 "), Ok(12.5));
        assert_eq!(parse_value(""), Ok(0.0));
        assert_eq!(parse_value("$1,200"), Ok(1200.0));
        assert!(parse_value("many").is_err());
        assert!(parse_value("1 2").is_err());
    }

    #[test]
    fn cells_edit_labels_and_values() {
        let chart = new_chart(None);
        let mut cells = BTreeMap::new();
        cells.insert((0, 1), "January".to_owned());
        cells.insert((1, 0), "North East".to_owned());
        cells.insert((2, 3), "99".to_owned());
        let edited = with_cells(&chart, &cells).unwrap();
        assert_eq!(edited.column_labels[0].shown(), "January");
        assert_eq!(edited.row_labels[0].shown(), "North East");
        assert_eq!(edited.value(1, 2), 99.0);
        cells.insert((1, 1), "x".to_owned());
        assert!(with_cells(&chart, &cells).is_err());
    }

    #[test]
    fn series_and_categories_are_added_and_removed() {
        let mut chart = new_chart(None);
        assert_eq!((chart.rows, chart.columns), (2, 3));
        assert!(add_series(&mut chart) && add_category(&mut chart));
        assert_eq!((chart.rows, chart.columns), (3, 4));
        assert_eq!(chart.values.len(), 12);
        assert_eq!(chart.value(0, 2), 30.0);
        assert_eq!(chart.value(0, 3), 0.0);
        assert!(remove_series(&mut chart) && remove_category(&mut chart));
        assert_eq!(chart.values, new_chart(None).values);
        let mut one = new_chart(Some(parse_data("1", &DataFormat::default()).unwrap()));
        assert!(!remove_series(&mut one) && !remove_category(&mut one));
    }

    #[test]
    fn the_axis_boxes_are_checked() {
        let mut form = AxisForm::from_axis(
            false,
            Axis {
                min: 0.0,
                max: 50.0,
                step: 10.0,
            },
        );
        assert_eq!(form.parse().unwrap().max, 50.0);
        form.max = "0".into();
        assert!(form.parse().unwrap_err().contains("above Min"));
        form.max = "50".into();
        form.inc = "0".into();
        assert!(form.parse().unwrap_err().contains("above 0"));
        form.inc = "0.1".into();
        assert!(form.parse().unwrap_err().contains("100 steps"));
        form.inc = "x".into();
        assert!(form.parse().unwrap_err().contains("Inc"));
    }

    #[test]
    fn the_clipboard_is_data_only_when_it_is_tabbed_numbers() {
        assert!(clipboard_data("a b 5").is_none());
        assert!(clipboard_data("a\tb").is_none());
        let data = clipboard_data("\tQ1\tQ2\nSales\t4\t5\n").unwrap();
        assert_eq!((data.columns, data.rows), (2, 1));
    }

    #[test]
    fn the_csv_has_the_categories_first() {
        let chart = new_chart(None);
        assert_eq!(csv(&chart), ",Jan,Feb,Mar\nEast,10,20,30\nWest,15,5,25\n");
    }
}
