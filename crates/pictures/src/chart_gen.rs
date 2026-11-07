//! The objects of a chart, generated from its data record the way
//! `AmiChart` generates them (`docs/research/OBJECTS_NOTES.md` §11.6.2 and
//! §12.1): ordinary drawing objects (lines, polygons, text; every one with
//! flag 4) in Ami Pro's order, the data record last. Generation is
//! deterministic and the objects draw through the drawing renderer
//! ([`crate::sdw::paint`]).
//!
//! What is known and what is not is kept in [`CONFIRMED`]: a family listed
//! as `Confirmed` was fitted to the corpus and to the CHV2 fixtures; an
//! `Unconfirmed` one is the best guess (the geometry was only partly read,
//! or the fixtures show a different shape), and is flagged so in the tests.

use crate::chart::{Axis, Chart, ChartType, Label, chart_object};
use crate::sdw::{self, Colour, Drawing, Fill, Object, Pen, Point, Shape, Text, TextKept};

/// Margin between the drawing's edge and what is drawn (CHV2).
const MARGIN: f64 = 144.0;

/// How sure a part of the generator is of Ami Pro's output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Confidence {
    /// Compared with Ami Pro's objects (corpus or CHV2).
    Confirmed,
    /// The formula in the notes or a guess; not compared, or compared and
    /// off by more than a few twips.
    Unconfirmed,
}

/// What each generated family is sure of, for the tests and the notes.
pub const CONFIRMED: &[(&str, Confidence)] = &[
    ("column 2D (clustered, stacked)", Confidence::Confirmed),
    ("bar 2D (clustered, stacked)", Confidence::Confirmed),
    ("column and bar oblique 3D", Confidence::Confirmed),
    ("axis (automatic)", Confidence::Confirmed),
    ("legend (up to two rows)", Confidence::Confirmed),
    ("line and picture markers", Confidence::Confirmed),
    ("picture and stacked picture", Confidence::Confirmed),
    ("area 2D", Confidence::Confirmed),
    ("pie and expanded pie 2D", Confidence::Confirmed),
    ("line ribbons", Confidence::Unconfirmed),
    (
        "perspective 3D, horizontal bar (CHV1)",
        Confidence::Confirmed,
    ),
    ("perspective 3D, other types", Confidence::Unconfirmed),
    ("area oblique 3D", Confidence::Confirmed),
    ("line oblique 3D", Confidence::Unconfirmed),
    ("pie 3D", Confidence::Unconfirmed),
    ("legend of three rows or more (CHV1)", Confidence::Confirmed),
    (
        "legend scale with the picture (five widths)",
        Confidence::Confirmed,
    ),
    (
        "frame to drawing rectangle (picture area, six sizes)",
        Confidence::Confirmed,
    ),
    ("flip data", Confidence::Confirmed),
    ("negative values, flat charts (CHV1)", Confidence::Confirmed),
    ("negative values, 3D", Confidence::Unconfirmed),
    ("pie slice order (three slices)", Confidence::Confirmed),
    (
        "expanded pie arcs and 3D pie thickness",
        Confidence::Unconfirmed,
    ),
    (
        "small frames (text 200), 3D column with grid and no legend",
        Confidence::Unconfirmed,
    ),
];

/// The drawing area and the text size a chart is generated for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metrics {
    /// The drawing rectangle in twips (4103 x 3395 for CHV2's 4608 x 3888
    /// frame).
    pub width: i32,
    pub height: i32,
    /// The height of the axis labels (233 in CHV2, 200 in its small frame).
    pub text: i32,
}

impl Metrics {
    /// The drawing rectangle Ami Pro gives a frame of this size: a line
    /// through the two frames CHV2 measured (4608 x 3888 gives 4103 x 3395,
    /// 2880 x 2160 gives 2472 x 1763) and the label size of each. An
    /// OK-candidate: other frames have not been measured.
    pub fn for_frame(width: i32, height: i32) -> Self {
        let fit = |frame: i32, small: (i32, i32), large: (i32, i32)| {
            let slope = f64::from(large.1 - small.1) / f64::from(large.0 - small.0);
            (f64::from(small.1) + slope * f64::from(frame - small.0)).round() as i32
        };
        let w = fit(width, (2880, 2472), (4608, 4103)).max(900);
        let h = fit(height, (2160, 1763), (3888, 3395)).max(700);
        let text = if height < 3000 { 200 } else { 233 };
        Self {
            width: w,
            height: h,
            text,
        }
    }

    /// The drawing rectangle for a frame whose picture area (the frame less
    /// its line and the picture inset: 288 twips for a frame the app makes
    /// with no line, the `[frmlay]` pair for one from Ami Pro) is `width` by
    /// `height` twips. CHV1 (notes 14.4): Ami Pro's rectangle is the area
    /// to within a few twips at six frame sizes (width +14 to -21, height
    /// -9 to +3; the 1.3 inch frame with a legend is 79 narrower).
    pub fn for_picture_area(width: i32, height: i32) -> Self {
        // Ami Pro's rectangle width against the picture area's, measured at
        // six sizes (notes 14.4; 4304 is the mean of 30 charts fitted at
        // 4320): between them the rectangle is interpolated.
        const WIDTHS: [(i32, i32); 6] = [
            (1872, 1793),
            (2592, 2597),
            (4032, 4011),
            (4320, 4304),
            (5472, 5475),
            (8352, 8353),
        ];
        // And the height (the lowest object's bottom plus 144).
        const HEIGHTS: [(i32, i32); 5] = [
            (1872, 1871),
            (2592, 2597),
            (3600, 3600),
            (4032, 4024),
            (5472, 5475),
        ];
        fn through(points: &[(i32, i32)], at: i32) -> i32 {
            let (first, last) = (points[0], points[points.len() - 1]);
            if at <= first.0 {
                return at + first.1 - first.0;
            }
            if at >= last.0 {
                return at + last.1 - last.0;
            }
            let pair = points.windows(2).find(|w| at <= w[1].0).unwrap();
            let (a, b) = (pair[0], pair[1]);
            let t = f64::from(at - a.0) / f64::from(b.0 - a.0);
            (f64::from(a.1) + t * f64::from(b.1 - a.1)).round() as i32
        }
        Self {
            width: through(&WIDTHS, width).max(900),
            height: through(&HEIGHTS, height).max(700),
            text: if height < 2700 { 200 } else { 233 },
        }
    }
}

// ----------------------------------------------------------------- helpers

/// Times Roman's advance widths per 1000 em for the printable ASCII.
const WIDTHS: [u16; 95] = [
    250, 333, 408, 500, 500, 833, 778, 333, 333, 333, 500, 564, 250, 304, 250, 278, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, 921, 722, 667, 667, 722, 611,
    556, 722, 722, 333, 389, 722, 611, 889, 722, 722, 556, 722, 667, 556, 611, 722, 722, 944, 722,
    722, 611, 333, 278, 333, 469, 500, 333, 444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500,
    278, 778, 500, 500, 500, 500, 333, 389, 278, 500, 500, 722, 500, 500, 444, 480, 200, 480, 541,
];

/// The width of `text` set in Times New Roman with a cell `height` tall
/// (a digit is 103 twips at 233, as Ami Pro measured).
pub fn text_width(text: &str, height: i32) -> i32 {
    text_width_em(text, f64::from(height) * 0.884)
}

/// The legend's smaller type is narrower for its height (288 twips for
/// "East" at 193).
fn legend_width(text: &str) -> i32 {
    text_width_em(text, 193.0 * 0.857)
}

/// How far the legend's horizontal sizes are from CHV2's (1.0 at a 4103
/// twip rectangle) for a rectangle `width` twips wide (CHV1, five widths).
fn legend_scale(width: f64) -> f64 {
    (1.0 + 2.576e-4 * (width - 4103.0)).clamp(0.3, 1.42)
}

fn text_width_em(text: &str, em: f64) -> i32 {
    let units: u32 = text
        .chars()
        .map(|c| {
            let code = c as usize;
            if (32..127).contains(&code) {
                u32::from(WIDTHS[code - 32])
            } else {
                500
            }
        })
        .sum();
    (f64::from(units) * em / 1000.0).round() as i32
}

fn clamp(value: f64) -> i16 {
    value.round().clamp(-32768.0, 32767.0) as i16
}

/// Rounds the way the pie's points are (half goes down).
fn half_down(value: f64) -> i16 {
    (value - 0.5).ceil().clamp(-32768.0, 32767.0) as i16
}

fn point(x: f64, y: f64) -> Point {
    Point::new(clamp(x), clamp(y))
}

fn pen(width: u8, style: u8) -> Pen {
    Pen {
        width,
        style,
        ..Pen::default()
    }
}

fn flagged(mut object: Object) -> Object {
    object.flags = sdw::CHART_FLAG;
    if let Shape::Group { objects, .. } = &mut object.shape {
        for inner in objects {
            inner.flags = sdw::CHART_FLAG;
        }
    }
    object
}

fn solid(rgb: [u8; 3]) -> Fill {
    Fill {
        background: Colour::WHITE,
        ..Fill::solid(Colour::rgb(rgb[0], rgb[1], rgb[2]))
    }
}

fn polygon(points: &[(f64, f64)], width: u8, rgb: [u8; 3]) -> Object {
    flagged(Object::polygon(
        points.iter().map(|&(x, y)| point(x, y)).collect(),
        pen(width, 0),
        solid(rgb),
    ))
}

fn polygon_points(points: Vec<Point>, width: u8, rgb: [u8; 3]) -> Object {
    flagged(Object::polygon(points, pen(width, 0), solid(rgb)))
}

/// A chart line: `(x0, y0)` to `(x1, y1)`, ten twips wide.
fn line(from: (f64, f64), to: (f64, f64), style: u8) -> Object {
    flagged(Object::line(
        point(from.0, from.1),
        point(to.0, to.1),
        pen(10, style),
        false,
    ))
}

/// A horizontal line, one twip lower at its end as Ami Pro draws them.
fn horizontal(x0: f64, x1: f64, y: f64, style: u8) -> Object {
    line((x0, y), (x1, y + 1.0), style)
}

/// A vertical line, one twip further right at its end.
fn vertical(x: f64, y0: f64, y1: f64, style: u8) -> Object {
    line((x, y0), (x + 1.0, y1), style)
}

/// A text object with its origin at `(x, y)`.
fn text_at(x: f64, y: f64, text: &str, height: i32, box_height: i32) -> Object {
    text_at_width(x, y, text, height, box_height, text_width(text, height))
}

fn text_at_width(x: f64, y: f64, text: &str, height: i32, box_height: i32, width: i32) -> Object {
    flagged(Object::text(Text {
        origin: point(x, y),
        size: Point::new(clamp(f64::from(width)), clamp(f64::from(box_height))),
        face: "Times New Roman".into(),
        pitch_and_family: 18,
        height: clamp(f64::from(height)),
        colour: Colour::BLACK,
        attributes: 0,
        attributes2: 0,
        angle: 0,
        text: text.into(),
        kept: TextKept::default(),
    }))
}

/// A channel scaled by `percent` (a zero channel stays zero below 100).
fn shade(rgb: [u8; 3], percent: u32) -> [u8; 3] {
    let channel = |c: u8| -> u8 {
        let value = u32::from(c) * percent / 100;
        if c == 0 && percent > 100 {
            0x20
        } else {
            value.min(255) as u8
        }
    };
    [channel(rgb[0]), channel(rgb[1]), channel(rgb[2])]
}

fn colour_of(chart: &Chart, index: usize) -> [u8; 3] {
    let entry = chart.palette[index % 16];
    [entry[0], entry[1], entry[2]]
}

/// The label of category `column` as drawn: its name, or its number.
fn category_label(chart: &Chart, column: usize) -> String {
    match chart.column_labels.get(column) {
        Some(label) if !label.is_empty() => label.shown(),
        _ => String::new(),
    }
}

fn series_label(chart: &Chart, row: usize) -> String {
    match chart.row_labels.get(row) {
        Some(label) if !label.is_empty() => label.shown(),
        _ => String::new(),
    }
}

/// A number as the axis shows it: no more digits than the step needs.
fn number(value: f64, step: f64) -> String {
    let mut decimals = 0;
    while decimals < 6 && ((step * 10f64.powi(decimals)).fract().abs() > 1e-9) {
        decimals += 1;
    }
    let text = format!("{value:.decimals$}", decimals = decimals as usize);
    if text == "-0" { "0".into() } else { text }
}

// -------------------------------------------------------------------- axis

fn is_horizontal(chart: &Chart) -> bool {
    chart.chart_type == ChartType::Bar
}

fn is_stacked(chart: &Chart) -> bool {
    match chart.chart_type {
        // Stacked is bit 0x0100 of the record's last word, not the
        // variation (CHV1: variation 0 with 0x1000 is regenerated clustered).
        ChartType::Column | ChartType::Bar | ChartType::Picture => chart.last_word & 0x0100 != 0,
        // Area charts are always stacked.
        ChartType::Line => chart.variation == 6,
        _ => false,
    }
}

/// The lowest and highest the data reaches (stacks added up where the chart
/// is stacked), including zero.
fn data_range(chart: &Chart) -> (f64, f64) {
    let mut low = 0.0f64;
    let mut high = 0.0f64;
    for column in 0..chart.columns {
        if is_stacked(chart) {
            let (mut up, mut down) = (0.0f64, 0.0f64);
            for row in 0..chart.rows {
                let value = f64::from(chart.value(row, column));
                if value >= 0.0 {
                    up += value;
                } else {
                    down += value;
                }
            }
            high = high.max(up);
            low = low.min(down);
        } else {
            for row in 0..chart.rows {
                let value = f64::from(chart.value(row, column));
                high = high.max(value);
                low = low.min(value);
            }
        }
    }
    (low, high)
}

/// The scale Ami Pro picks when Automatic is ticked (`1010:1a66`): the
/// tick count is the axis length over the label height (a third as many
/// along horizontal bars, `length` being the plot's width then), the step
/// the first digit of range over count made 1, 2 or 5 times a power of
/// ten (doubled on vertical charts), the minimum rounded down and the
/// maximum to the next step strictly above the data.
pub fn automatic_axis(chart: &Chart, length: i32) -> Axis {
    automatic_axis_for(chart, length, 233)
}

fn automatic_axis_for(chart: &Chart, length: i32, text: i32) -> Axis {
    let (low, high) = data_range(chart);
    let mut target = f64::from(length.max(1)) / f64::from(text.max(1));
    if is_horizontal(chart) {
        target /= 3.0;
    }
    let target = target.max(1.0);
    let mut raw = (high - low) / target;
    if !is_horizontal(chart) {
        raw *= 2.0;
    }
    let step = if raw <= 0.0 || !raw.is_finite() {
        1.0
    } else {
        let mut exponent = raw.log10().floor();
        let mut digit = (raw / 10f64.powf(exponent)).floor();
        // Guard the float edge (raw just under a power of ten).
        if digit >= 10.0 {
            digit = 1.0;
            exponent += 1.0;
        }
        let digit = match digit as u32 {
            1 => 1.0,
            2 => 2.0,
            3..=5 => 5.0,
            _ => {
                exponent += 1.0;
                1.0
            }
        };
        digit * 10f64.powf(exponent)
    };
    // The step does not shrink in a tall plot: ticks stay about 1.27 label
    // heights (296 twips: a step 2 at 294 is refused, a step 10 at 299 and 336 accepted) apart (CHV1: plot heights 2644, 3262 and 4700 all
    // give a step of 5 for data to 30, where the count rule says 2 at 4700).
    let ends = |step: f64| {
        let max = ((high / step).floor() + 1.0) * step;
        let min = if low < 0.0 {
            // The next step strictly below the lowest value (CHV1).
            ((low / step).ceil() - 1.0) * step
        } else {
            0.0
        };
        (min, max)
    };
    let mut step = step;
    // (Measured on plots 1867 twips and taller; the corpus's 807 twip plot
    // keeps the count rule.)
    if !is_horizontal(chart) && length >= 1500 && raw > 0.0 && raw.is_finite() {
        loop {
            let (min, max) = ends(step);
            let spacing = step * f64::from(length) / (max - min);
            if spacing >= 1.27 * f64::from(text) {
                break;
            }
            let exponent = step.log10().floor();
            let digit = (step / 10f64.powf(exponent)).round();
            step = match digit as u32 {
                1 => 2.0,
                2 => 5.0,
                _ => 10.0,
            } * 10f64.powf(exponent);
            if step > 1e12 {
                break;
            }
        }
    }
    let (min, max) = ends(step);
    Axis { min, max, step }
}

// -------------------------------------------------------------------- layout

struct Build<'a> {
    chart: &'a Chart,
    width: f64,
    height: f64,
    text: i32,
    objects: Vec<Object>,
    legend_left: Option<f64>,
    /// The axis the user typed (Automatic off); `None` computes it.
    axis: Option<Axis>,
}

impl Build<'_> {
    /// The value axis for a plot `length` twips long: the typed one, or
    /// the one Ami Pro computes.
    fn axis_for(&self, chart: &Chart, length: i32) -> Axis {
        self.axis
            .unwrap_or_else(|| automatic_axis_for(chart, length, self.text))
    }

    fn right(&self) -> f64 {
        self.legend_left
            .map_or(self.width - MARGIN, |left| left - MARGIN)
    }

    fn label_box(&self) -> i32 {
        self.text
    }

    /// The legend (`1010`): a shadow, a white box, a swatch and a label for
    /// each series; the first objects of the stream, one group.
    fn legend(&mut self) {
        let chart = self.chart;
        let rows = chart.rows;
        let names: Vec<String> = (0..rows).map(|row| series_label(chart, row)).collect();
        let widest = names
            .iter()
            .map(|name| legend_width(name))
            .max()
            .unwrap_or(0);
        // The legend's horizontal sizes grow with the picture (CHV1, notes
        // 14.2 item 6: x 0.405 at a 1793 twip rectangle, 1.0 at 4103, 1.055
        // at 4305, 1.37 at 5475, 1.42 at 8353); its heights do not.
        // The box's width is a share of the picture whatever the labels
        // (979 and 980 twips for "West" and "North" at 4320): the parts
        // are scaled to fill it, so a longer label shrinks them.
        let natural = 590.0 + f64::from(widest);
        let scale = (legend_scale(self.width) * 930.0 / natural).clamp(0.3, 1.42);
        let swatch = 193.0 * scale;
        let gap = 194.0 * scale;
        let inset = 96.0 * scale;
        let box_width = inset + swatch + gap + f64::from(widest) * scale + 107.0 * scale;
        // Three rows or more: the box is 1356 twips from top to bottom for
        // three (411 a row), the swatches 406.5 apart from 136 down (CHV1).
        let many = rows > 2;
        let pitch = if many { 406.5 } else { 405.0 };
        let first = if many { 136.0 } else { 135.0 };
        let box_height = if many {
            124.0 + 411.0 * rows as f64
        } else {
            135.0 + 405.0 * rows as f64
        };
        let right = self.width - MARGIN - 15.0;
        let left = (right - box_width - 10.0).round();
        let top = (self.height / 2.0 + 21.0 - box_height / 2.0).floor();
        self.legend_left = Some(left);
        let mut inner = vec![
            polygon(
                &[
                    (left + 10.0, top - 14.0),
                    (right, top - 14.0),
                    (right, top - 14.0 + box_height - 1.0),
                    (left + 10.0, top - 14.0 + box_height - 1.0),
                ],
                7,
                [0, 0, 0],
            ),
            polygon(
                &[
                    (left, top),
                    (left + box_width, top),
                    (left + box_width, top + box_height - 1.0),
                    (left, top + box_height - 1.0),
                ],
                7,
                [255, 255, 255],
            ),
        ];
        // The shadow's background is black too.
        if let Shape::Polygon { fill, .. } = &mut inner[0].shape {
            fill.background = Colour::BLACK;
        }
        if let Shape::Polygon { fill, .. } = &mut inner[1].shape {
            fill.background = Colour::BLACK;
        }
        // A clustered horizontal bar chart lists its series from the
        // bottom up (the first series lowest, as its bars sit in a
        // category; a stacked bar chart does not; CHV1).
        let upward = is_horizontal(chart) && !is_stacked(chart);
        for (row, name) in names.iter().enumerate() {
            let place = if upward { rows - 1 - row } else { row };
            let y = top + first + pitch * place as f64;
            let colour = if chart.colour_by_category() && chart.chart_type == ChartType::Column {
                colour_of(chart, 0)
            } else {
                colour_of(chart, row)
            };
            if matches!(
                chart.chart_type,
                ChartType::LinePicture | ChartType::Picture
            ) {
                // 2207's marker follows the category, so its legend shows the
                // first marker for every series.
                let kind = if chart.chart_type == ChartType::LinePicture && chart.variation == 3 {
                    0
                } else {
                    row
                };
                inner.push(picture_shape(
                    kind,
                    left + inset,
                    left + inset + swatch,
                    y,
                    y + 270.0,
                    colour,
                    7,
                ));
            } else {
                inner.push(polygon(
                    &[
                        (left + inset, y),
                        (left + inset + swatch, y),
                        (left + inset + swatch, y + 270.0),
                        (left + inset, y + 270.0),
                    ],
                    7,
                    colour,
                ));
            }
            inner.push(text_at_width(
                left + inset + swatch + gap,
                y,
                name,
                193,
                192,
                legend_width(name),
            ));
        }
        self.objects.push(flagged(Object::group(inner)));
    }

    fn push(&mut self, object: Object) {
        // A category without a name has no label.
        if matches!(&object.shape, Shape::Text(text) if text.text.is_empty()) {
            return;
        }
        self.objects.push(object);
    }
}

/// Generates the chart's drawing objects for `metrics`, the data record
/// last.
pub fn generate(chart: &Chart, metrics: &Metrics) -> Vec<Object> {
    generate_with_axis(chart, metrics, None)
}

/// [`generate`] with the value axis typed in the Charting dialog's Axis
/// group (Automatic off: Inc, Min, and Max) instead of computed; the
/// record keeps it. A pie has no axis.
pub fn generate_with_axis(chart: &Chart, metrics: &Metrics, axis: Option<Axis>) -> Vec<Object> {
    let mut build = Build {
        chart,
        width: f64::from(metrics.width),
        height: f64::from(metrics.height),
        text: metrics.text,
        objects: Vec::new(),
        legend_left: None,
        axis,
    };
    let mut record = chart.clone();
    if chart.legend_on() {
        build.legend();
    }
    match chart.chart_type {
        ChartType::Pie => pie(&mut build),
        ChartType::Column | ChartType::Picture => vertical_bars(&mut build, &mut record),
        ChartType::Bar => horizontal_bars(&mut build, &mut record),
        ChartType::Line | ChartType::LinePicture => lines(&mut build, &mut record),
        ChartType::Other(_) => {}
    }
    let mut objects = build.objects;
    objects.push(chart_object(&record));
    objects
}

/// The drawing for `chart`: [`generate`]'s objects as a new drawing.
pub fn generate_drawing(chart: &Chart, metrics: &Metrics) -> Drawing {
    generate_drawing_with_axis(chart, metrics, None)
}

/// [`generate_drawing`] with a typed value axis (see [`generate_with_axis`]).
pub fn generate_drawing_with_axis(chart: &Chart, metrics: &Metrics, axis: Option<Axis>) -> Drawing {
    Drawing {
        objects: generate_with_axis(chart, metrics, axis),
        ..Drawing::new()
    }
}

// ------------------------------------------------------------- 3D geometry

#[derive(Clone, Copy)]
struct Box3 {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
    /// The skew: the back wall is this much right and up of the front.
    dx: f64,
    dy: f64,
}

impl Box3 {
    /// The front plane inside an outer box that includes the skew
    /// (`depth` percent: 5 % of the front plane at depth 50).
    fn inside(left: f64, right: f64, top: f64, bottom: f64, depth: f64) -> Self {
        let k = 0.05 * depth / 50.0;
        let width = (right - left) / (1.0 + k);
        let height = (bottom - top) / (1.0 + k);
        Self {
            left,
            right: left + width,
            top: bottom - height,
            bottom,
            dx: k * width,
            dy: k * height,
        }
    }
}

/// Perspective (CHV1, notes 14.2 item 4): the back plane is the front plane
/// drawn toward a vanishing point, `back = V + k * (front - V)`, fitted on
/// eighteen corner pairs of the stacked bar (within 1 twip) at depth 50:
/// k 0.8802, V at 1.102 plane widths from the left edge and 0.0134 plane
/// heights below the top. Only horizontal bars use it (the one perspective
/// chart measured).
#[derive(Clone, Copy)]
struct Persp {
    left: f64,
    right: f64,
    top: f64,
    bottom: f64,
    k: f64,
    vx: f64,
    vy: f64,
}

impl Persp {
    fn new(left: f64, right: f64, top: f64, bottom: f64, depth: f64) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
            k: 1.0 - 0.1198 * depth / 50.0,
            vx: left + 1.1018 * (right - left),
            vy: top + 0.01336 * (bottom - top),
        }
    }

    fn at(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.vx + self.k * (x - self.vx),
            self.vy + self.k * (y - self.vy),
        )
    }
}

/// The floor, left wall and back wall of a horizontal chart in perspective.
fn walls_persp(build: &mut Build<'_>, p: &Persp) {
    let (lb, rb) = (p.at(p.left, p.bottom), p.at(p.right, p.bottom));
    let (lt, rt) = (p.at(p.left, p.top), p.at(p.right, p.top));
    build.push(polygon(
        &[(p.right, p.bottom), (p.left, p.bottom), lb, rb],
        10,
        [192, 192, 192],
    ));
    build.push(polygon(
        &[(p.left, p.bottom), (p.left, p.top), lt, lb],
        10,
        [255, 255, 255],
    ));
    build.push(polygon(&[rb, lb, lt, rt], 10, [255, 255, 255]));
}

fn walls(build: &mut Build<'_>, b: &Box3, horizontal_chart: bool) {
    if horizontal_chart {
        // Horizontal charts wind the floor and the back wall the other way
        // and list the floor first.
        build.push(polygon(
            &[
                (b.right, b.bottom),
                (b.left, b.bottom),
                (b.left + b.dx, b.bottom - b.dy),
                (b.right + b.dx, b.bottom - b.dy),
            ],
            10,
            [192, 192, 192],
        ));
        build.push(polygon(
            &[
                (b.left, b.bottom),
                (b.left, b.top),
                (b.left + b.dx, b.top - b.dy),
                (b.left + b.dx, b.bottom - b.dy),
            ],
            10,
            [255, 255, 255],
        ));
        build.push(polygon(
            &[
                (b.right + b.dx, b.bottom - b.dy),
                (b.left + b.dx, b.bottom - b.dy),
                (b.left + b.dx, b.top - b.dy),
                (b.right + b.dx, b.top - b.dy),
            ],
            10,
            [255, 255, 255],
        ));
        return;
    }
    build.push(polygon(
        &[
            (b.left, b.top),
            (b.left, b.bottom),
            (b.left + b.dx, b.bottom - b.dy),
            (b.left + b.dx, b.top - b.dy),
        ],
        10,
        [255, 255, 255],
    ));
    build.push(polygon(
        &[
            (b.left, b.bottom),
            (b.right, b.bottom),
            (b.right + b.dx, b.bottom - b.dy),
            (b.left + b.dx, b.bottom - b.dy),
        ],
        10,
        [192, 192, 192],
    ));
    build.push(polygon(
        &[
            (b.left + b.dx, b.top - b.dy),
            (b.left + b.dx, b.bottom - b.dy),
            (b.right + b.dx, b.bottom - b.dy),
            (b.right + b.dx, b.top - b.dy),
        ],
        10,
        [255, 255, 255],
    ));
}

/// One 3D bar: front, top and right faces as a group (100, 90 and 50 %;
/// the shades swap for a negative bar). `order_right_first` is how the
/// horizontal charts list them.
fn bar_3d(
    front: (f64, f64, f64, f64),
    dx: f64,
    dy: f64,
    rgb: [u8; 3],
    negative: bool,
    right_first: bool,
) -> Object {
    let (x0, y0, x1, y1) = front;
    let (top_pct, side_pct) = if negative { (50, 90) } else { (90, 50) };
    let face = polygon(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], 7, rgb);
    let top = polygon(
        &[(x1, y0), (x1 + dx, y0 - dy), (x0 + dx, y0 - dy), (x0, y0)],
        7,
        shade(rgb, top_pct),
    );
    let side = polygon(
        &[(x1, y0), (x1 + dx, y0 - dy), (x1 + dx, y1 - dy), (x1, y1)],
        7,
        shade(rgb, side_pct),
    );
    let parts = if right_first {
        vec![face, side, top]
    } else {
        vec![face, top, side]
    };
    flagged(Object::group(parts))
}

// ------------------------------------------------------------ vertical bars

/// Column, stacked column, picture and stacked picture, flat or oblique.
fn vertical_bars(build: &mut Build<'_>, record: &mut Chart) {
    let chart = build.chart;
    let th = f64::from(build.text);
    let three_d = chart.three_d() != 0;
    let bottom_edge = build.height - MARGIN;
    let label_top = bottom_edge - th - if three_d { 0.0 } else { 2.0 };
    let named = (0..chart.columns).any(|c| !category_label(chart, c).is_empty());
    let axis_bottom = if named {
        label_top - (th / 2.0).floor()
    } else {
        bottom_edge
    };
    let top_edge = MARGIN;
    // The vertical extent of the value axis (2644 flat, 2627 oblique).
    let length = if three_d {
        let outer = axis_bottom - top_edge;
        outer / 1.05
    } else {
        axis_bottom - (top_edge + (th / 2.0).floor() - 4.0)
    };
    let axis = build.axis_for(chart, length.round() as i32);
    record.axis = axis;
    let range = axis.max - axis.min;
    let labels: Vec<String> = ticks(&axis)
        .iter()
        .map(|&value| number(value, axis.step))
        .collect();
    let widest = labels
        .iter()
        .map(|label| text_width(label, build.text))
        .max()
        .unwrap_or(0);
    // In 3D the labels are set two twips smaller (digits 101 wide), and the
    // gap is 10 wider without a legend (an OK-candidate fitted to CHV2).
    let widest = if three_d {
        widest - widest / 100
    } else {
        widest
    };
    let gap = if three_d {
        if chart.legend_on() { 102.0 } else { 112.0 }
    } else if chart.grid_on() || chart.chart_type == ChartType::Picture {
        111.0
    } else {
        122.0
    };
    let left = MARGIN + f64::from(widest) + gap;
    let right = build.right();
    let label_right = MARGIN + f64::from(widest);
    let categories = chart.columns.max(1);
    let depth = f64::from(chart.depth());

    let (front, plane_left, plane_right, tick_scale, data_scale, bottom);
    if three_d {
        let b = Box3::inside(left, right, top_edge, axis_bottom, depth);
        walls(build, &b, false);
        front = Some(b);
        plane_left = b.left;
        plane_right = b.right;
        bottom = b.bottom;
        let height = b.bottom - b.top;
        tick_scale = (height - 4.0) / range;
        data_scale = height / range;
    } else {
        front = None;
        plane_left = left;
        plane_right = right;
        bottom = axis_bottom;
        let top = top_edge + (th / 2.0).floor() - 4.0;
        data_scale = (axis_bottom - top) / range;
        tick_scale = (axis_bottom - top - 4.0) / range;
        build.push(vertical(left, top, axis_bottom, 0));
        build.push(horizontal(left, right, axis_bottom, 0));
    }
    // A negative axis draws its zero line last, and in 3D the back wall's
    // zero line first (CHV1).
    let negative_axis = axis.min < 0.0;
    let zero_tick_y = (bottom + 0.3 - (0.0 - axis.min) * tick_scale).floor();
    if negative_axis {
        if let Some(b) = front {
            build.push(horizontal(
                b.left + b.dx,
                b.right + b.dx,
                zero_tick_y - b.dy,
                0,
            ));
        }
    }
    // Ticks (or the grid) and the value labels.
    for (index, value) in ticks(&axis).into_iter().enumerate() {
        let y_true = bottom + 0.3 - (value - axis.min) * tick_scale;
        let y = y_true.floor();
        let at_max = value >= axis.max - axis.step * 1e-9;
        let is_zero = negative_axis && value == 0.0;
        if is_zero {
            if let Some(b) = front {
                build.push(line((b.left + b.dx, y - b.dy), (b.left, y), 0));
            }
        }
        if is_zero && chart.grid_on() {
            // The grid has no line at zero; the zero line comes last.
        } else if chart.grid_on() {
            if three_d {
                if !at_max {
                    let b = front.unwrap();
                    let back_y = y - b.dy;
                    build.push(horizontal(b.left + b.dx, b.right + b.dx, back_y, 2));
                    build.push(line((b.left, y), (b.left + b.dx, back_y), 2));
                }
            } else {
                build.push(horizontal(plane_left, plane_right, y, 2));
            }
        } else if three_d {
            build.push(horizontal(plane_left - 51.0, plane_left, y, 0));
        } else if !at_max || true {
            build.push(horizontal(
                plane_left
                    - if chart.chart_type == ChartType::Picture {
                        53.0
                    } else {
                        54.0
                    },
                plane_left,
                y,
                0,
            ));
        }
        if value != 0.0 || negative_axis {
            let label = &labels[index];
            let width = f64::from(text_width(label, build.text));
            build.push(text_at(
                label_right - width,
                (y_true - 116.25 * th / 233.0).floor(),
                label,
                build.text,
                build.label_box(),
            ));
        }
    }
    // Categories and bars.
    let cw = (plane_right - plane_left) / categories as f64;
    let rows = chart.rows.max(1);
    let stacked = is_stacked(chart);
    let picture = chart.chart_type == ChartType::Picture;
    let zero_y = bottom - (0.0 - axis.min) * data_scale;
    for column in 0..chart.columns {
        let centre = plane_left + cw * (column as f64 + 0.5);
        let label = category_label(chart, column);
        let width = f64::from(text_width(&label, build.text));
        let pad = if chart.chart_type == ChartType::Picture {
            10.0
        } else if chart.grid_on() {
            9.0
        } else {
            18.0
        };
        build.push(text_at(
            (centre - (width + pad) / 2.0).floor(),
            label_top,
            &label,
            build.text,
            build.label_box(),
        ));
        let group = cw * 0.7;
        let group_left = centre - group / 2.0;
        let mut positive = 0.0f64;
        let mut negative = 0.0f64;
        for row in 0..chart.rows {
            let value = f64::from(chart.value(row, column));
            let colour = if chart.colour_by_category() && !stacked && rows > 1
                || chart.colour_by_category() && chart.chart_type == ChartType::Column
            {
                colour_of(chart, column)
            } else {
                colour_of(chart, row)
            };
            let (x0, x1, from, to);
            if stacked {
                x0 = group_left;
                x1 = group_left + group;
                if value >= 0.0 {
                    from = positive;
                    positive += value;
                    to = positive;
                } else {
                    from = negative;
                    negative += value;
                    to = negative;
                }
            } else {
                let each = group / rows as f64;
                x0 = group_left + each * row as f64;
                x1 = x0 + each;
                from = 0.0;
                to = value;
            }
            let offset = |value: f64| {
                let raw = (value - axis.min) * data_scale;
                if three_d { raw } else { raw.trunc() }
            };
            let y_from = bottom - offset(from);
            let y_to = bottom - offset(to);
            let (y_from, y_to) = if stacked {
                (y_from, y_to)
            } else {
                (zero_y, y_to)
            };
            if picture {
                build.push(picture_shape(
                    row,
                    x0,
                    x1,
                    y_to.round(),
                    y_from.round(),
                    colour,
                    10,
                ));
            } else if let Some(b) = front {
                let (top, base) = if y_to <= y_from {
                    (y_to, y_from)
                } else {
                    (y_from, y_to)
                };
                build.push(bar_3d(
                    (x0.round(), top.round(), x1.round(), base.round()),
                    b.dx.round(),
                    b.dy.round(),
                    colour,
                    value < 0.0,
                    false,
                ));
            } else {
                let (top, base) = if y_to <= y_from {
                    (y_to, y_from)
                } else {
                    (y_from, y_to)
                };
                build.push(polygon(
                    &[
                        (x0.floor(), top.round()),
                        (x1.floor(), top.round()),
                        (x1.floor(), base.round()),
                        (x0.floor(), base.round()),
                    ],
                    10,
                    colour,
                ));
            }
        }
    }
    if negative_axis {
        if let Some(b) = front {
            build.push(horizontal(b.left, b.right, zero_tick_y, 0));
            build.push(line(
                (b.right, zero_tick_y),
                (b.right + b.dx, zero_tick_y - b.dy),
                0,
            ));
        } else {
            build.push(horizontal(left, right, zero_tick_y, 0));
        }
    }
}

/// A picture chart's shape for series `row` between `top` and `base`: a
/// triangle, a diamond, then squares.
fn picture_shape(
    row: usize,
    x0: f64,
    x1: f64,
    top: f64,
    base: f64,
    rgb: [u8; 3],
    pen: u8,
) -> Object {
    let (x0, x1) = (x0.floor(), x1.floor());
    let centre = ((x0 + x1) / 2.0).floor();
    let middle = ((top + base) / 2.0).floor();
    match row % 3 {
        0 => polygon(&[(centre, top), (x0, base), (x1, base)], pen, rgb),
        1 => polygon(
            &[(centre, top), (x0, middle), (centre, base), (x1, middle)],
            pen,
            rgb,
        ),
        _ => polygon(&[(x0, top), (x1, top), (x1, base), (x0, base)], pen, rgb),
    }
}

/// The values the axis labels, above its minimum. With a negative minimum
/// the order is Ami Pro's (CHV1): the ticks above zero from the lowest up,
/// zero, then the ticks below zero from the one under zero down to the
/// minimum.
fn ticks(axis: &Axis) -> Vec<f64> {
    let mut out = Vec::new();
    if axis.step <= 0.0 || !axis.step.is_finite() {
        return out;
    }
    let mut value = axis.min + axis.step;
    let mut guard = 0;
    while value <= axis.max + axis.step * 1e-9 && guard < 200 {
        out.push(value);
        value += axis.step;
        guard += 1;
    }
    if axis.min < 0.0 {
        let mut ordered: Vec<f64> = out
            .iter()
            .copied()
            .filter(|&v| v > axis.step * 1e-9)
            .collect();
        ordered.push(0.0);
        let mut down = -axis.step;
        while down > axis.min - axis.step * 1e-9 && ordered.len() < 400 {
            ordered.push(down);
            down -= axis.step;
        }
        return ordered;
    }
    out
}

// ---------------------------------------------------------- horizontal bars

fn horizontal_bars(build: &mut Build<'_>, record: &mut Chart) {
    let chart = build.chart;
    let th = f64::from(build.text);
    let three_d = chart.three_d() == 1;
    let perspective = chart.three_d() == 2;
    // The widest category label fixes the left edge.
    let names: Vec<String> = (0..chart.columns)
        .map(|c| category_label(chart, c))
        .collect();
    // With a legend the category labels are set narrower (x 0.935; CHV1:
    // all four bar charts with a legend, against one without).
    let squeeze = if chart.legend_on() { 0.935 } else { 1.0 };
    let text_size = build.text;
    let label_width = |name: &str| (f64::from(text_width(name, text_size)) * squeeze).round();
    let widest = names
        .iter()
        .map(|name| label_width(name) as i32)
        .max()
        .unwrap_or(0);
    let left = MARGIN + f64::from(widest) + 115.0;
    let label_row_top = build.height - MARGIN - th;
    let axis_y = label_row_top - th;
    let top = MARGIN;
    let depth = f64::from(chart.depth());
    // The widest value label halves into the right margin.
    let provisional = automatic_axis_for(chart, (build.right() - left).round() as i32, build.text);
    let last = number(provisional.max, provisional.step);
    let right = build.right() - f64::from(text_width(&last, build.text)) / 2.0 + 3.0;
    let length = if three_d || perspective {
        (right - left) / 1.05
    } else {
        right - left
    };
    // Perspective shrinks the far end (0.85: CHV1 fitted a step of 20 at 4304; CHV2: the step of the 3D
    // stacked bar is 20 where oblique would give 10).
    let length = if perspective { length * 0.85 } else { length };
    let axis = build.axis_for(chart, length.round() as i32);
    record.axis = axis;
    let range = axis.max - axis.min;
    let (front, plane_left, plane_right, plane_top, bottom);
    let (tick_scale, data_scale);
    let projection =
        perspective.then(|| Persp::new(left, right, top, axis_y, f64::from(chart.depth())));
    if three_d {
        let b = Box3::inside(left, right, top, axis_y, depth);
        walls(build, &b, true);
        front = Some(b);
        plane_left = b.left;
        plane_right = b.right;
        plane_top = b.top;
        bottom = b.bottom;
        data_scale = (b.right - b.left) / range;
        tick_scale = (b.right - b.left - 3.0) / range;
    } else {
        front = None;
        plane_left = left;
        plane_right = right;
        plane_top = top;
        bottom = axis_y;
        data_scale = (right - left) / range;
        tick_scale = (right - left - 3.0) / range;
        if let Some(p) = &projection {
            walls_persp(build, p);
        } else {
            build.push(line((right, axis_y), (left, axis_y + 1.0), 0));
            build.push(line((left, axis_y), (left + 1.0, top), 0));
        }
    }
    let all = ticks(&axis);
    let negative_axis = axis.min < 0.0;
    for value in all.iter().copied() {
        let x = (plane_left + (value - axis.min) * tick_scale).floor();
        let at_max = value >= axis.max - axis.step * 1e-9;
        if negative_axis && value == 0.0 && chart.grid_on() {
            // No grid line at zero: the zero line comes last.
        } else if chart.grid_on() {
            if !((three_d || perspective) && at_max) {
                if let Some(p) = &projection {
                    let (bx, by) = p.at(x, bottom);
                    let (tx, ty) = p.at(x, plane_top);
                    build.push(line((bx, by), (tx + 1.0, ty), 2));
                    build.push(line((x, bottom), (bx, by), 2));
                } else if let Some(b) = front {
                    build.push(line(
                        (x + b.dx, b.bottom - b.dy),
                        (x + b.dx + 1.0, b.top - b.dy),
                        2,
                    ));
                    build.push(line((x, b.bottom), (x + b.dx, b.bottom - b.dy), 2));
                } else {
                    build.push(vertical(x, plane_top, bottom, 2));
                }
            }
        } else {
            build.push(line((x, bottom + 51.0), (x + 1.0, bottom), 0));
        }
        let label = number(value, axis.step);
        let width = f64::from(text_width(&label, build.text));
        build.push(text_at(
            x - (width / 2.0).floor(),
            label_row_top,
            &label,
            build.text,
            build.label_box(),
        ));
    }
    let categories = chart.columns.max(1);
    let (plane_height, plane_base) = (bottom - plane_top, bottom);
    let ch = plane_height / categories as f64;
    let rows = chart.rows.max(1);
    let stacked = is_stacked(chart);
    for (column, label) in names.iter().enumerate() {
        let centre = plane_base - ch * (column as f64 + 0.5);
        let width = label_width(label);
        build.push(text_at(
            left - 115.0 + 1.0 - width,
            (centre - th / 2.0).round(),
            label,
            build.text,
            build.label_box(),
        ));
        let group = ch * 0.7;
        let group_bottom = centre + group / 2.0;
        let mut positive = 0.0f64;
        for row in 0..chart.rows {
            let value = f64::from(chart.value(row, column));
            let colour =
                if chart.colour_by_category() && chart.chart_type == ChartType::Bar && rows > 1 {
                    colour_of(chart, column)
                } else {
                    colour_of(chart, row)
                };
            let (y_bottom, y_top, from, to);
            if stacked {
                y_bottom = group_bottom;
                y_top = group_bottom - group;
                from = positive;
                positive += value;
                to = positive;
            } else {
                let each = group / rows as f64;
                y_bottom = group_bottom - each * row as f64;
                y_top = y_bottom - each;
                from = 0.0;
                to = value;
            }
            let x_from = plane_left + (from - axis.min) * data_scale;
            let x_to = plane_left + (to - axis.min) * data_scale;
            let (x0, x1) = (x_from.floor(), x_to.floor());
            if let Some(p) = &projection {
                build.push(bar_persp_horizontal(
                    (x0, y_top.round(), x1, y_bottom.round()),
                    p,
                    colour,
                ));
            } else if let Some(b) = front {
                build.push(bar_3d_horizontal(
                    (x0, y_top.round(), x1, y_bottom.round()),
                    b.dx.round(),
                    b.dy.round(),
                    colour,
                ));
            } else {
                // A negative bar lists its zero end first.
                let (xa, xb) = if value < 0.0 { (x0, x1) } else { (x1, x0) };
                build.push(polygon(
                    &[
                        (xa, y_bottom.round()),
                        (xa, y_top.round()),
                        (xb, y_top.round()),
                        (xb, y_bottom.round()),
                    ],
                    10,
                    colour,
                ));
            }
        }
    }
    if negative_axis {
        let zero_x = (plane_left + (0.0 - axis.min) * tick_scale).floor();
        build.push(line((zero_x, bottom), (zero_x + 1.0, plane_top), 0));
    }
    let _ = plane_right;
}

/// A horizontal bar in perspective: the face, the right side and the top,
/// the back corners projected.
fn bar_persp_horizontal(front: (f64, f64, f64, f64), p: &Persp, rgb: [u8; 3]) -> Object {
    let (x0, y0, x1, y1) = front;
    let face = polygon(&[(x1, y1), (x1, y0), (x0, y0), (x0, y1)], 7, rgb);
    let (tr, br, tl) = (p.at(x1, y0), p.at(x1, y1), p.at(x0, y0));
    let side = polygon(&[(x1, y0), tr, br, (x1, y1)], 7, shade(rgb, 50));
    let top = polygon(&[(x1, y0), tr, tl, (x0, y0)], 7, shade(rgb, 90));
    flagged(Object::group(vec![face, side, top]))
}

fn bar_3d_horizontal(front: (f64, f64, f64, f64), dx: f64, dy: f64, rgb: [u8; 3]) -> Object {
    let (x0, y0, x1, y1) = front;
    let face = polygon(&[(x1, y1), (x1, y0), (x0, y0), (x0, y1)], 7, rgb);
    let side = polygon(
        &[(x1, y0), (x1 + dx, y0 - dy), (x1 + dx, y1 - dy), (x1, y1)],
        7,
        shade(rgb, 50),
    );
    let top = polygon(
        &[(x1, y0), (x1 + dx, y0 - dy), (x0 + dx, y0 - dy), (x0, y0)],
        7,
        shade(rgb, 90),
    );
    flagged(Object::group(vec![face, side, top]))
}

// -------------------------------------------------------------------- lines

/// Line, line and picture (both kinds), and area.
fn lines(build: &mut Build<'_>, record: &mut Chart) {
    let chart = build.chart;
    let th = f64::from(build.text);
    let area = chart.chart_type == ChartType::Line && chart.variation == 6;
    let three_d = chart.three_d() != 0;
    let bottom_edge = build.height - MARGIN;
    let label_top = bottom_edge
        - th
        - if three_d {
            0.0
        } else if area {
            1.0
        } else {
            2.0
        };
    let named = (0..chart.columns).any(|c| !category_label(chart, c).is_empty());
    let axis_bottom = if named {
        label_top - (th / 2.0).floor()
    } else {
        bottom_edge
    };
    let top_edge = MARGIN;
    let length = if three_d {
        (axis_bottom - top_edge) / 1.05
    } else {
        axis_bottom - (top_edge + (th / 2.0).floor() - 4.0)
    };
    let axis = build.axis_for(chart, length.round() as i32);
    record.axis = axis;
    let range = axis.max - axis.min;
    let labels: Vec<String> = ticks(&axis).iter().map(|&v| number(v, axis.step)).collect();
    let widest = labels
        .iter()
        .map(|label| text_width(label, build.text))
        .max()
        .unwrap_or(0);
    let gap = if three_d {
        106.0
    } else if area {
        108.0
    } else {
        92.0
    };
    let left = MARGIN + f64::from(widest) + gap;
    let label_right = MARGIN + f64::from(widest);
    let last_label = category_label(chart, chart.columns.saturating_sub(1));
    let last_width = f64::from(text_width(&last_label, build.text));
    let limit = build.right();
    let right = if three_d {
        limit - 7.0
    } else if area {
        limit - ((last_width + 7.0) / 2.0).floor()
    } else {
        limit - (last_width * 0.97 / 2.0).round()
    };
    let depth = f64::from(chart.depth());
    let (front, plane_left, plane_right, bottom, tick_scale, data_scale);
    let top;
    if three_d {
        let b = Box3::inside(left, right, top_edge, axis_bottom, depth);
        walls(build, &b, false);
        front = Some(b);
        plane_left = b.left;
        plane_right = b.right;
        bottom = b.bottom;
        top = b.top;
        data_scale = (b.bottom - b.top) / range;
        tick_scale = (b.bottom - b.top - 4.0) / range;
    } else {
        front = None;
        plane_left = left;
        plane_right = right;
        bottom = axis_bottom;
        top = top_edge + (th / 2.0).floor() - 4.0;
        data_scale = (axis_bottom - top) / range;
        tick_scale = (axis_bottom - top - 4.0) / range;
        build.push(vertical(left, top, axis_bottom, 0));
        build.push(horizontal(left, right, axis_bottom, 0));
    }
    let _ = top;
    let all = ticks(&axis);
    // A negative axis: the back wall's zero line first in 3D, the zero line
    // last (CHV1).
    let negative_axis = axis.min < 0.0;
    let zero_tick_y = (bottom + 0.3 - (0.0 - axis.min) * tick_scale).floor();
    if negative_axis {
        if let Some(b) = front {
            build.push(horizontal(
                b.left + b.dx,
                b.right + b.dx,
                zero_tick_y - b.dy,
                0,
            ));
        }
    }
    for (index, value) in all.iter().copied().enumerate() {
        let y_true = bottom + 0.3 - (value - axis.min) * tick_scale;
        let y = y_true.floor();
        let at_max = value >= axis.max - axis.step * 1e-9;
        let is_zero = negative_axis && value == 0.0;
        if is_zero {
            if let Some(b) = front {
                build.push(line((b.left + b.dx, y - b.dy), (b.left, y), 0));
            }
        }
        if is_zero && chart.grid_on() {
            // No grid line at zero.
        } else if chart.grid_on() {
            if let Some(b) = front {
                if !at_max {
                    build.push(horizontal(b.left + b.dx, b.right + b.dx, y - b.dy, 2));
                    build.push(line((b.left, y), (b.left + b.dx, y - b.dy), 2));
                }
            } else {
                build.push(horizontal(plane_left, plane_right, y, 2));
            }
        } else if three_d {
            build.push(horizontal(plane_left - 51.0, plane_left, y, 0));
        } else {
            build.push(horizontal(
                plane_left - if area { 53.0 } else { 50.0 },
                plane_left,
                y,
                0,
            ));
        }
        if value != 0.0 || negative_axis {
            let label = &labels[index];
            let width = f64::from(text_width(label, build.text));
            build.push(text_at(
                label_right - width,
                (y_true - 116.25 * th / 233.0).floor(),
                label,
                build.text,
                build.label_box(),
            ));
        }
    }
    let n = chart.columns;
    let step = if n > 1 {
        (plane_right - plane_left) / (n - 1) as f64
    } else {
        0.0
    };
    let x_of = |k: usize| -> f64 { plane_left + step * k as f64 };
    // Stacked tops for areas.
    let top_of = |row: usize, column: usize| -> f64 {
        let total: f64 = if area {
            (0..=row).map(|r| f64::from(chart.value(r, column))).sum()
        } else {
            f64::from(chart.value(row, column))
        };
        let raw = (total - axis.min) * data_scale;
        bottom - if three_d { raw.round() } else { raw.trunc() }
    };
    let zero_base = bottom - {
        let raw = (0.0 - axis.min) * data_scale;
        if three_d { raw.round() } else { raw.trunc() }
    };
    let markers = matches!(chart.chart_type, ChartType::LinePicture);
    let by_category = markers && chart.variation == 3;
    let ribbons = !area && !markers;
    let label_shift = if area { -3.5 } else { 4.5 };
    for column in 0..n {
        // The category's label comes first, then the interval before it.
        let label = category_label(chart, column);
        let width = f64::from(text_width(&label, build.text));
        let label_object = |build: &mut Build<'_>, k: usize| {
            build.push(text_at(
                (x_of(k) - width / 2.0 + label_shift).floor(),
                label_top,
                &label,
                build.text,
                build.label_box(),
            ));
        };
        if column == 0 {
            label_object(build, 0);
        }
        if column > 0 {
            let k = column;
            if area {
                // An area's label comes before the interval it ends.
                label_object(build, k);
            }
            for row in 0..chart.rows {
                let colour = colour_of(chart, row);
                let (x0, x1) = (x_of(k - 1), x_of(k));
                if area {
                    let (a0, a1) = (top_of(row, k - 1), top_of(row, k));
                    let (b0, b1) = if row == 0 {
                        (zero_base, zero_base)
                    } else {
                        (top_of(row - 1, k - 1), top_of(row - 1, k))
                    };
                    if let Some(b) = front {
                        build.push(area_3d((x0, x1, a0, a1, b0, b1), (b.dx, b.dy), colour));
                    } else {
                        let pts = [
                            (x0.floor(), b0),
                            (x0.floor(), a0),
                            (x1.floor(), a1),
                            (x1.floor(), b1),
                        ];
                        build.push(polygon(&pts, 10, colour));
                    }
                } else if markers {
                    let (y0, y1) = (top_of(row, k - 1), top_of(row, k));
                    build.push(line((x0.floor(), y0), (x1.floor(), y1), 0));
                    let m = if by_category { k - 1 } else { row };
                    build.push(marker(m, x0, y0, colour));
                    if k == n - 1 {
                        let m = if by_category { k } else { row };
                        build.push(marker(m, x1, y1, colour));
                    }
                } else if ribbons {
                    let points: Vec<(f64, f64)> =
                        (0..n).map(|c| (x_of(c).floor(), top_of(row, c))).collect();
                    build.push(ribbon(&points, k - 1, colour, front.map(|b| (b.dx, b.dy))));
                }
            }
            if !area {
                label_object(build, k);
            }
        }
    }
    if negative_axis {
        if let Some(b) = front {
            build.push(horizontal(b.left, b.right, zero_tick_y, 0));
            build.push(line(
                (b.right, zero_tick_y),
                (b.right + b.dx, zero_tick_y - b.dy),
                0,
            ));
        } else {
            build.push(horizontal(plane_left, plane_right, zero_tick_y, 0));
        }
    }
}

/// A marker 150 twips wide and 153 tall: a triangle, a diamond, a square.
fn marker(kind: usize, x: f64, y: f64, rgb: [u8; 3]) -> Object {
    let (x, y) = (x.floor(), y.floor());
    match kind % 3 {
        0 => polygon(
            &[(x, y - 77.0), (x - 75.0, y + 76.0), (x + 75.0, y + 76.0)],
            10,
            rgb,
        ),
        1 => polygon(
            &[(x, y - 77.0), (x - 75.0, y), (x, y + 77.0), (x + 75.0, y)],
            10,
            rgb,
        ),
        _ => polygon(
            &[
                (x - 74.0, y - 77.0),
                (x + 75.0, y - 77.0),
                (x + 75.0, y + 76.0),
                (x - 74.0, y + 76.0),
            ],
            10,
            rgb,
        ),
    }
}

/// Half the thickness of a line chart's ribbon (an OK-candidate: 25
/// twips gave the 30-twip vertical ends and the mitres CHV2 shows to
/// within 2 twips).
const RIBBON: f64 = 25.0;

/// A line chart's segment `index` (between points `index` and `index + 1`)
/// as Ami Pro draws it: a ribbon of the polyline, 50 twips thick, cut
/// upright at the ends of the line and mitred where two segments meet.
fn ribbon(points: &[(f64, f64)], index: usize, rgb: [u8; 3], skew: Option<(f64, f64)>) -> Object {
    if let Some((sx, sy)) = skew {
        // Oblique: a strip with the skew as its depth (unconfirmed).
        let (a, b) = (points[index], points[index + 1]);
        return polygon(&[a, b, (b.0 + sx, b.1 - sy), (a.0 + sx, a.1 - sy)], 10, rgb);
    }
    // The upward unit normal of a segment.
    let normal = |i: usize| -> (f64, f64) {
        let (a, b) = (points[i], points[i + 1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = (dx * dx + dy * dy).sqrt().max(1e-9);
        let (nx, ny) = (dy / length, -dx / length);
        if ny > 0.0 { (-nx, -ny) } else { (nx, ny) }
    };
    // The two edge points at point `k`: (upper, lower).
    let edge = |k: usize, towards: usize| -> ((f64, f64), (f64, f64)) {
        let p = points[k];
        let first = k == 0;
        let last = k + 1 == points.len();
        if first || last {
            // Cut upright: the vertical thickness of the one segment.
            let n = normal(towards);
            let vertical = RIBBON / n.1.abs().max(1e-9);
            return ((p.0, p.1 - vertical), (p.0, p.1 + vertical));
        }
        let (n0, n1) = (normal(k - 1), normal(k));
        let denominator = 1.0 + n0.0 * n1.0 + n0.1 * n1.1;
        let (mx, my) = ((n0.0 + n1.0) / denominator, (n0.1 + n1.1) / denominator);
        (
            (p.0 + RIBBON * mx, p.1 + RIBBON * my),
            (p.0 - RIBBON * mx, p.1 - RIBBON * my),
        )
    };
    let (start_upper, start_lower) = edge(index, index);
    let (end_upper, end_lower) = edge(index + 1, index);
    polygon(&[start_lower, end_lower, end_upper, start_upper], 10, rgb)
}

/// One interval of an oblique area: `edges` is (x0, x1, top0, top1, base0,
/// base1), `skew` the depth across and up.
fn area_3d(edges: (f64, f64, f64, f64, f64, f64), skew: (f64, f64), rgb: [u8; 3]) -> Object {
    let (x0, x1, a0, a1, b0, b1) = edges;
    let (dx, dy) = skew;
    let front = polygon(&[(x0, b0), (x0, a0), (x1, a1), (x1, b1)], 10, rgb);
    let top = polygon(
        &[(x0, a0), (x0 + dx, a0 - dy), (x1 + dx, a1 - dy), (x1, a1)],
        10,
        shade(rgb, 90),
    );
    let side = polygon(
        &[(x1, a1), (x1 + dx, a1 - dy), (x1 + dx, b1 - dy), (x1, b1)],
        10,
        shade(rgb, 50),
    );
    // A band whose edge rises exactly as steeply as the skew has no top face
    // (CHV1: West over Feb to Mar, slope 0.80, is a group of two polygons;
    // CHV2's frame, 0.67 twips off, keeps all three).
    let slope_gap = (a1 - a0) + dy * (x1 - x0) / dx;
    if dx > 0.0 && slope_gap.abs() < 0.5 {
        return flagged(Object::group(vec![front, side]));
    }
    flagged(Object::group(vec![front, top, side]))
}

// ---------------------------------------------------------------------- pie

fn pie(build: &mut Build<'_>) {
    let chart = build.chart;
    let available = build.legend_left.unwrap_or(build.width);
    let three_d = chart.three_d() != 0;
    let expanded = chart.last_word & 0x0100 != 0 && chart.chart_type == ChartType::Pie;
    let cx = available / 2.0;
    let cy = build.height / 2.0;
    let radius = ((available - 2.0 * MARGIN * 2.0) / 2.0)
        .min((build.height - 2.0 * MARGIN * 2.0) / 2.0)
        .floor();
    let values: Vec<f64> = (0..chart.rows)
        .map(|r| f64::from(chart.value(r, 0)).abs())
        .collect();
    let total: f64 = values.iter().sum();
    if total <= 0.0 {
        return;
    }
    // 3D pies are tilted (the ellipse's short radius) and have a rim; their
    // numbers are a guess (OK-candidate): the corpus and CHV2 show each
    // slice as a group of its top, its rim and its cut sides.
    let squash = if three_d { 0.55 } else { 1.0 };
    let thickness = if three_d {
        radius * 0.2477 * f64::from(chart.depth().max(1)) / 50.0
    } else {
        0.0
    };
    let arc = |ox: f64, oy: f64, from: f64, to: f64, down: f64| -> Vec<Point> {
        let mut pts = Vec::new();
        let mut angle = from;
        while angle < to {
            pts.push(Point::new(
                half_down(cx + ox + radius * angle.cos()),
                half_down(cy + oy + down - radius * squash * angle.sin()),
            ));
            angle += 0.05;
        }
        pts.push(Point::new(
            half_down(cx + ox + radius * to.cos()),
            half_down(cy + oy + down - radius * squash * to.sin()),
        ));
        pts
    };
    let mut start = 0.0f64;
    let mut out = Vec::new();
    for (row, value) in values.iter().enumerate() {
        let sweep = value / total * std::f64::consts::TAU;
        let end = start + sweep;
        let (ox, oy) = if expanded && row == 0 {
            let mid = (start + end) / 2.0;
            (radius * 0.1 * mid.cos(), -radius * 0.1 * mid.sin() * squash)
        } else {
            (0.0, 0.0)
        };
        let rgb = colour_of(chart, row);
        let mut top = arc(ox, oy, start, end, 0.0);
        top.push(Point::new(half_down(cx + ox), half_down(cy + oy)));
        if !three_d {
            out.push(polygon_points(top, 10, rgb));
        } else {
            let mut parts = vec![polygon_points(top, 10, rgb)];
            // The rim: the front part of the arc (below the middle) down
            // and back up.
            let front_from = start.max(std::f64::consts::PI);
            let front_to = end.min(std::f64::consts::TAU);
            if front_to > front_from {
                let mut rim = arc(ox, oy, front_from, front_to, 0.0);
                let mut lower = arc(ox, oy, front_from, front_to, thickness);
                lower.reverse();
                rim.extend(lower);
                parts.push(polygon_points(rim, 10, shade(rgb, 75)));
            }
            // Only the first slice shows its cut sides.
            for angle in [start, end].into_iter().filter(|_| row == 0) {
                let outer = Point::new(
                    half_down(cx + ox + radius * angle.cos()),
                    half_down(cy + oy - radius * squash * angle.sin()),
                );
                let centre = Point::new(half_down(cx + ox), half_down(cy + oy));
                let below = |p: Point| Point::new(p.x, p.y + thickness as i16);
                parts.push(polygon_points(
                    vec![centre, outer, below(outer), below(centre)],
                    10,
                    shade(rgb, 50),
                ));
            }
            out.push(flagged(Object::group(parts)));
        }
        start = end;
    }
    // Three slices are listed first, third, second (CHV1; two in order).
    if out.len() == 3 {
        out.swap(1, 2);
    }
    for object in out {
        build.push(object);
    }
}

// ------------------------------------------------------------- comparison

/// How well generated objects match Ami Pro's stored ones.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Comparison {
    /// Counts by kind in the generated objects and the stored ones (groups
    /// counted with their children).
    pub generated: Counts,
    pub stored: Counts,
    /// Objects in the same place of both streams with the same kind.
    pub aligned: usize,
    /// Of those, how many have every coordinate within the tolerance.
    pub within: usize,
    /// The largest coordinate difference among aligned objects.
    pub worst: i32,
    /// The mean coordinate difference among aligned objects.
    pub mean: f64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub lines: usize,
    pub polygons: usize,
    pub texts: usize,
    pub groups: usize,
    pub other: usize,
}

impl Counts {
    pub fn total(&self) -> usize {
        self.lines + self.polygons + self.texts + self.groups + self.other
    }
}

fn flatten<'a>(objects: &'a [Object], out: &mut Vec<&'a Object>) {
    for object in objects {
        out.push(object);
        if let Shape::Group { objects, .. } = &object.shape {
            flatten(objects, out);
        }
    }
}

fn counts(objects: &[&Object]) -> Counts {
    let mut counts = Counts::default();
    for object in objects {
        match &object.shape {
            Shape::Line { .. } | Shape::Polyline { .. } => counts.lines += 1,
            Shape::Polygon { .. } | Shape::Rectangle { .. } => counts.polygons += 1,
            Shape::Text(_) => counts.texts += 1,
            Shape::Group { .. } => counts.groups += 1,
            _ => counts.other += 1,
        }
    }
    counts
}

/// The coordinates an object is compared by (a text by its origin only:
/// Ami Pro's text rectangles are padded by a few percent that no rule
/// fits).
fn coordinates(object: &Object) -> Vec<i32> {
    match &object.shape {
        Shape::Line { points, .. } => points
            .iter()
            .flat_map(|p| [p.x.into(), p.y.into()])
            .collect(),
        Shape::Polygon { points, .. } | Shape::Polyline { points, .. } => points
            .iter()
            .flat_map(|p| [p.x.into(), p.y.into()])
            .collect(),
        Shape::Text(text) => vec![text.origin.x.into(), text.origin.y.into()],
        _ => Vec::new(),
    }
}

/// Compares the generated stream with the stored one: object counts by
/// kind, and, object for object in order where the kinds agree, how far the
/// coordinates are apart. `tolerance` is in twips.
pub fn compare(generated: &[Object], stored: &[Object], tolerance: i32) -> Comparison {
    let (mut a, mut b) = (Vec::new(), Vec::new());
    flatten(generated, &mut a);
    flatten(stored, &mut b);
    a.retain(|object| object.kind != sdw::kind::CHART);
    // Objects the user added to the chart (flag 0) are not generated.
    b.retain(|object| object.kind != sdw::kind::CHART && object.flags & sdw::CHART_FLAG != 0);
    let mut result = Comparison {
        generated: counts(&a),
        stored: counts(&b),
        ..Comparison::default()
    };
    let mut sum = 0.0;
    let mut samples = 0usize;
    for (x, y) in a.iter().zip(&b) {
        if x.kind != y.kind {
            continue;
        }
        let (cx, cy) = (coordinates(x), coordinates(y));
        if cx.len() != cy.len() {
            continue;
        }
        result.aligned += 1;
        let worst = cx
            .iter()
            .zip(&cy)
            .map(|(p, q)| (p - q).abs())
            .max()
            .unwrap_or(0);
        result.worst = result.worst.max(worst);
        if worst <= tolerance {
            result.within += 1;
        }
        for (p, q) in cx.iter().zip(&cy) {
            sum += f64::from((p - q).abs());
            samples += 1;
        }
    }
    result.mean = if samples == 0 {
        0.0
    } else {
        sum / samples as f64
    };
    result
}

/// How sure the generator is of Ami Pro's output for this chart (the
/// families in [`CONFIRMED`]): perspective except in a horizontal bar, 3D
/// lines, 3D pies, an expanded pie, and negative values in 3D are guesses
/// (legends of three rows or more, negative values in flat charts and the
/// bar's perspective were confirmed by CHV1).
pub fn confidence(chart: &Chart) -> Confidence {
    let guess = (chart.three_d() == 2 && chart.chart_type != ChartType::Bar)
        || (chart.three_d() == 1 && chart.chart_type == ChartType::Line && chart.variation != 6)
        || (chart.chart_type == ChartType::Pie
            && (chart.three_d() != 0 || chart.last_word & 0x0100 != 0))
        || (chart.three_d() != 0 && chart.values.iter().any(|v| *v < 0.0))
        || matches!(chart.chart_type, ChartType::Other(_));
    // Plain lines are drawn as ribbons whose end shape was read from two
    // segments only.
    let ribbon = chart.chart_type == ChartType::Line && chart.variation != 6;
    if guess || ribbon {
        Confidence::Unconfirmed
    } else {
        Confidence::Confirmed
    }
}

/// The row label text for the generated legend, for tests.
pub fn legend_labels(chart: &Chart) -> Vec<String> {
    (0..chart.rows)
        .map(|row| series_label(chart, row))
        .collect()
}

#[allow(dead_code)]
fn unused(_: Label) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart::{DataFormat, Icon, parse_data};

    /// CHV2's data: columns Jan, Feb, Mar; rows East 10 20 30, West 15 5 25.
    fn chv2(icon: Icon) -> Chart {
        let mut chart = Chart::new(icon, 3, 2);
        let data = parse_data(
            "\tJan\tFeb\tMar\nEast\t10\t20\t30\nWest\t15\t5\t25\n",
            &DataFormat::default(),
        )
        .unwrap();
        chart.set_data(data);
        chart
    }

    const BIG: Metrics = Metrics {
        width: 4103,
        height: 3395,
        text: 233,
    };

    fn count(objects: &[Object]) -> usize {
        let mut flat = Vec::new();
        flatten(objects, &mut flat);
        flat.iter().filter(|o| o.kind != sdw::kind::CHART).count()
    }

    fn near(a: i16, b: i32, tolerance: i32) -> bool {
        (i32::from(a) - b).abs() <= tolerance
    }

    #[test]
    fn a_typed_axis_is_used_and_kept_in_the_record() {
        let chart = chv2(Icon::Column);
        let typed = Axis {
            min: 0.0,
            max: 100.0,
            step: 20.0,
        };
        let objects = generate_with_axis(&chart, &BIG, Some(typed));
        let record = crate::chart::chart_of(&Drawing {
            objects: objects.clone(),
            ..Drawing::new()
        })
        .unwrap()
        .1;
        assert_eq!(record.axis, typed);
        let texts: Vec<String> = objects
            .iter()
            .filter_map(|o| match &o.shape {
                Shape::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"100".to_string()) && texts.contains(&"60".to_string()));
        // Without it the axis is computed, as before.
        assert_eq!(
            generate_with_axis(&chart, &BIG, None),
            generate(&chart, &BIG)
        );
    }

    #[test]
    fn the_automatic_axis_follows_the_notes() {
        let mut chart = chv2(Icon::Column);
        // The maximum is the next step strictly above the data.
        assert_eq!(
            automatic_axis(&chart, 2644),
            Axis {
                min: 0.0,
                max: 35.0,
                step: 5.0
            }
        );
        chart.set_icon(Icon::StackedColumn);
        assert_eq!(
            automatic_axis(&chart, 2627),
            Axis {
                min: 0.0,
                max: 60.0,
                step: 10.0
            }
        );
        chart.set_icon(Icon::Bar);
        assert_eq!(
            automatic_axis(&chart, 3259),
            Axis {
                min: 0.0,
                max: 40.0,
                step: 10.0
            }
        );
        // The corpus's 3D column: 12, 23, 5, 14 in a 807-twip plot.
        let mut corpus = Chart::new(Icon::Column, 4, 1);
        corpus.values = vec![12.0, 23.0, 5.0, 14.0];
        assert_eq!(
            automatic_axis(&corpus, 807),
            Axis {
                min: 0.0,
                max: 30.0,
                step: 10.0
            }
        );
        // Nothing to scale.
        let empty = Chart::new(Icon::Column, 2, 1);
        assert_eq!(automatic_axis(&empty, 2644).step, 1.0);
    }

    #[test]
    fn every_object_has_flag_4_the_record_is_last_and_it_is_deterministic() {
        for icon in Icon::ALL {
            let mut chart = chv2(icon);
            chart.grid = 1;
            let a = generate(&chart, &BIG);
            let b = generate(&chart, &BIG);
            assert_eq!(a, b, "{icon:?} is not deterministic");
            let mut flat = Vec::new();
            flatten(&a, &mut flat);
            assert!(flat.iter().all(|o| o.flags == sdw::CHART_FLAG), "{icon:?}");
            assert_eq!(a.last().unwrap().kind, sdw::kind::CHART, "{icon:?}");
            // The drawing writes, reads back, and draws.
            let drawing = generate_drawing(&chart, &BIG);
            let written = sdw::write(&drawing);
            let read = sdw::read(&written).unwrap();
            assert_eq!(read.objects.len(), drawing.objects.len());
            let (_, again) = crate::chart::chart_of(&read).unwrap();
            assert_eq!(again.values, chart.values);
            assert!(!sdw::paint(&read).ops.is_empty(), "{icon:?} draws nothing");
        }
    }

    #[test]
    fn object_counts_match_ami_pros_for_the_chv2_data() {
        let counts = |icon: Icon, three_d: u16, grid: u16, legend: bool| {
            let mut chart = chv2(icon);
            chart.oblique_depth = three_d;
            chart.grid = grid;
            chart.legend = u16::from(legend);
            count(&generate(&chart, &BIG))
        };
        // §12.1: column with legend and grid 31 + 1 group, bar 19 (no
        // legend), line 29 + 1, 2206 35 + 1, picture 31 + 1, stacked
        // picture 29 + 1, area 21, 3D column 44 (no legend).
        assert_eq!(counts(Icon::Column, 0, 1, true), 32);
        assert_eq!(counts(Icon::Bar, 0, 0, false), 19);
        assert_eq!(counts(Icon::Line, 0, 1, true), 30);
        assert_eq!(counts(Icon::LinePicture, 0, 0, true), 36);
        assert_eq!(counts(Icon::LineCategoryPicture, 0, 0, true), 36);
        assert_eq!(counts(Icon::Picture, 0, 0, true), 32);
        assert_eq!(counts(Icon::StackedPicture, 0, 0, true), 30);
        assert_eq!(counts(Icon::Area, 0, 0, false), 21);
        assert_eq!(counts(Icon::StackedColumn, 50, 0, true), 49);
        assert_eq!(counts(Icon::Column, 50, 0, false), 44);
        // A pie shows the first column: one slice a row.
        assert_eq!(counts(Icon::Pie, 0, 0, true), 9);
    }

    #[test]
    fn a_flat_column_chart_is_ami_pros_to_a_few_twips() {
        let mut chart = chv2(Icon::Column);
        chart.grid = 1;
        let objects = generate(&chart, &BIG);
        // The legend group first: shadow, box, then a swatch and a label a
        // series. Width 96 + 193 + 194 + 338 + 107 (+ a twip or two).
        let Shape::Group {
            objects: legend, ..
        } = &objects[0].shape
        else {
            panic!("the legend comes first");
        };
        assert_eq!(legend.len(), 6);
        assert!(near(legend[1].rect.left, 3005, 2) && near(legend[1].rect.right, 3933, 4));
        assert!(near(legend[1].rect.top, 1246, 1) && near(legend[1].rect.bottom, 2191, 2));
        // The axes, then a dotted grid line and a label a tick.
        let Shape::Line { points, pen } = &objects[1].shape else {
            panic!();
        };
        assert_eq!(
            (points[0].x, points[0].y, points[1].x, points[1].y),
            (461, 256, 462, 2900)
        );
        assert_eq!(pen.width, 10);
        let Shape::Line { points, pen } = &objects[3].shape else {
            panic!();
        };
        assert!(near(points[0].y, 2523, 1));
        assert_eq!(pen.style, 2);
        // Jan's first bar: 581..860 across, 2145 to 2900 down.
        let bars: Vec<&Object> = objects
            .iter()
            .filter(|o| matches!(&o.shape, Shape::Polygon { .. }) && o.rect.top >= 600)
            .collect();
        assert!(near(bars[0].rect.left, 581, 2) && near(bars[0].rect.top, 2145, 1));
        assert!(near(bars[0].rect.right, 860, 2) && near(bars[0].rect.bottom, 2900, 1));
        // Clustered columns take the colour of their category (blue,
        // light blue, green: 0000ff, 0080ff, 00c200).
        let Shape::Polygon { fill, .. } = &bars[0].shape else {
            panic!()
        };
        assert_eq!(fill.foreground.rgb_bytes(), [0, 0, 255]);
        // The record carries the axis Ami Pro stored.
        let Shape::Other(body) = &objects.last().unwrap().shape else {
            panic!()
        };
        let record = Chart::parse(body).unwrap();
        assert_eq!(
            record.axis,
            Axis {
                min: 0.0,
                max: 35.0,
                step: 5.0
            }
        );
    }

    #[test]
    fn the_corpus_3d_column_formulas_hold() {
        // 12, 23, 5, 14 in a 3D clustered column, depth 50, as the user's
        // corpus has it: bars 0.7 of the slot, skew 5 %, shades 100, 90, 50.
        let mut chart = Chart::new(Icon::Column, 4, 1);
        chart.values = vec![12.0, 23.0, 5.0, 14.0];
        chart.oblique_depth = 50;
        chart.legend = 0;
        chart.grid = 1;
        chart.palette[0] = [255, 0, 0, 6];
        let objects = generate(&chart, &BIG);
        let groups: Vec<&Object> = objects
            .iter()
            .filter(|o| matches!(o.shape, Shape::Group { .. }))
            .collect();
        assert_eq!(groups.len(), 4);
        let Shape::Group { objects: parts, .. } = &groups[0].shape else {
            panic!()
        };
        let fills: Vec<[u8; 3]> = parts
            .iter()
            .map(|p| match &p.shape {
                Shape::Polygon { fill, .. } => fill.foreground.rgb_bytes(),
                _ => panic!(),
            })
            .collect();
        assert_eq!(fills, [[255, 0, 0], [229, 0, 0], [127, 0, 0]]);
        // The front face is 0.7 of its slot wide; the skew is 5 % of the
        // front plane across and up.
        let front = parts[0].rect;
        let top = parts[1].rect;
        let side = parts[2].rect;
        let slot = f64::from(front.width()) / 0.7;
        assert!((slot * 4.0 - 4.0 * slot).abs() < 1e-9);
        let skew_x = i32::from(top.right) - i32::from(front.right);
        let skew_y = i32::from(front.top) - i32::from(top.top);
        assert!((skew_x - (slot * 4.0 * 0.05).round() as i32).abs() <= 2);
        assert!(skew_y > 0 && i32::from(side.top) == i32::from(top.top));
    }

    #[test]
    fn labels_and_values_come_from_the_data() {
        let mut chart = chv2(Icon::Column);
        chart.values[0] = 12.5;
        chart.axis = Axis {
            min: 0.0,
            max: 1.0,
            step: 1.0,
        };
        let texts: Vec<String> = generate(&chart, &BIG)
            .iter()
            .filter_map(|o| match &o.shape {
                Shape::Text(t) => Some(t.text.clone()),
                _ => None,
            })
            .collect();
        for wanted in ["5", "35", "Jan", "Feb", "Mar"] {
            assert!(
                texts.iter().any(|t| t == wanted),
                "{wanted} missing in {texts:?}"
            );
        }
        // A half step shows the digits the step needs.
        let mut small = Chart::new(Icon::Column, 2, 1);
        small.values = vec![0.5, 1.2];
        let axis = automatic_axis(&small, 2644);
        assert_eq!(axis.step, 0.2);
        assert_eq!(number(1.2, axis.step), "1.2");
        assert_eq!(number(1.0, axis.step), "1.0");
        assert_eq!(number(-0.0, 1.0), "0");
    }

    #[test]
    fn chv1_corrections_hold() {
        // Stacked is bit 0x0100 of the last word, not the variation.
        let mut chart = chv2(Icon::Column);
        chart.variation = 0;
        chart.last_word = 0x1000;
        assert_eq!(automatic_axis(&chart, 2644).max, 35.0);
        chart.last_word = 0x0100;
        assert_eq!(automatic_axis(&chart, 2644).max, 60.0);
        // A tall plot keeps the step (4700 twips gave 0/35/5, not step 2).
        chart.set_icon(Icon::Column);
        assert_eq!(automatic_axis(&chart, 4700).step, 5.0);
        // The minimum is the next step strictly below the lowest value.
        chart.values[1] = -20.0;
        chart.values[3] = 5.0;
        let axis = automatic_axis(&chart, 2844);
        assert!(axis.min <= -30.0, "{axis:?}");
        // Ticks: above zero, zero, then below zero down to the minimum.
        let ticks = ticks(&Axis {
            min: -30.0,
            max: 40.0,
            step: 10.0,
        });
        assert_eq!(ticks, [10.0, 20.0, 30.0, 40.0, 0.0, -10.0, -20.0, -30.0]);
        // The picture area gives Ami Pro's rectangle (4320 x 3600: 4304).
        let m = Metrics::for_picture_area(4320, 3600);
        assert_eq!((m.width, m.height), (4304, 3600));
        // Perspective: back = V + 0.88 (front - V).
        let p = Persp::new(579.0, 2913.0, 144.0, 2988.0, 50.0);
        let (x, y) = p.at(579.0, 2988.0);
        assert!(
            (x - 886.0).abs() < 2.0 && (y - 2652.0).abs() < 2.0,
            "{x} {y}"
        );
    }

    #[test]
    fn what_is_a_guess_says_so() {
        let mut chart = chv2(Icon::Column);
        assert_eq!(confidence(&chart), Confidence::Confirmed);
        chart.legend = 0;
        chart.perspective_depth = 50;
        assert_eq!(confidence(&chart), Confidence::Unconfirmed);
        chart.perspective_depth = 0;
        chart.set_icon(Icon::Line);
        assert_eq!(confidence(&chart), Confidence::Unconfirmed);
        chart.set_icon(Icon::Area);
        assert_eq!(confidence(&chart), Confidence::Confirmed);
        chart.set_icon(Icon::ExpandedPie);
        assert_eq!(confidence(&chart), Confidence::Unconfirmed);
        chart.set_icon(Icon::Pie);
        assert_eq!(confidence(&chart), Confidence::Confirmed);
        assert!(CONFIRMED.iter().any(|(_, c)| *c == Confidence::Unconfirmed));
    }

    #[test]
    fn pies_flip_negatives_and_empty_data_do_not_break() {
        let mut chart = chv2(Icon::Pie);
        let objects = generate(&chart, &BIG);
        // Slices begin at 3 o'clock and the first is 40 % (144 degrees:
        // 53 points).
        let Shape::Polygon { points, .. } = &objects
            .iter()
            .find(|o| matches!(o.shape, Shape::Polygon { .. }) && o.rect.left < 2800)
            .unwrap()
            .shape
        else {
            panic!()
        };
        assert_eq!(points.len(), 53);
        assert_eq!((points[0].x, points[0].y), (2716, 1697));
        chart.flip();
        assert_eq!(chart.rows, 3);
        assert!(!generate(&chart, &BIG).is_empty());
        let mut negative = chv2(Icon::Column);
        negative.values[1] = -8.0;
        let axis = automatic_axis(&negative, 2644);
        assert!(axis.min < 0.0);
        assert!(!generate(&negative, &BIG).is_empty());
        // All zeros: a record and the axes, no panic.
        let mut zero = Chart::new(Icon::Pie, 1, 2);
        zero.legend = 0;
        assert_eq!(generate(&zero, &BIG).len(), 1);
        for icon in Icon::ALL {
            let _ = generate(&Chart::new(icon, 1, 1), &BIG);
            let _ = generate(&Chart::new(icon, 40, 3), &Metrics::for_frame(2880, 2160));
        }
    }

    #[test]
    fn the_frame_rule_reproduces_the_two_measured_frames() {
        assert_eq!(Metrics::for_frame(4608, 3888), BIG);
        assert_eq!(
            Metrics::for_frame(2880, 2160),
            Metrics {
                width: 2472,
                height: 1763,
                text: 200
            }
        );
    }

    #[test]
    fn text_widths_match_the_measured_digits() {
        assert_eq!(text_width("5", 233), 103);
        assert_eq!(text_width("10", 233), 206);
        assert!((text_width("Jan", 233) - 276).abs() <= 2);
        assert!((legend_width("West") - 338).abs() <= 3);
    }
}
