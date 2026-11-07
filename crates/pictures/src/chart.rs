//! Ami Pro's charts (`AmiChart`, `docs/research/OBJECTS_NOTES.md` §11.1 and
//! §12.1): the data record a chart keeps beside its drawing objects (object
//! type 15, flag 4), read into a [`Chart`] and written back byte for byte,
//! and the Charting dialog's data text (`1018:0078`) parsed as Ami Pro
//! parses it and written back from a chart.
//!
//! A chart is *series by categories*: `rows` rows (the series, named by the
//! row labels and listed in the legend) of `columns` values (the categories,
//! named by the column labels). The record's length is
//! `134 + 5 * A * B + 20 * (A + B)` with A the columns and B the rows.
//! Offsets in the comments are from the record's start, header included
//! (the body this module reads begins at `0x10`).
//!
//! The generated objects are in [`crate::chart_gen`].

use crate::sdw::{self, Drawing, Object, Shape};

/// The header of an object record, which the body follows.
const HEADER: usize = 0x10;
/// The record's fixed part: header, counts, words, the tail with the axis
/// and the colours.
const FIXED: usize = 134;
/// Most cells a chart holds (`LIBOPEN`: 0x1388).
pub const MAX_CELLS: usize = 5000;
/// Longest label, in characters (a label field is 20 bytes with its NUL).
pub const MAX_LABEL: usize = 19;

/// What kind of chart, the word at `0x1c`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ChartType {
    /// Vertical bars (type 0).
    Column,
    /// Lines, or areas when the variation is 6 (type 1; CHV2: area is
    /// variation 6, line 0).
    Line,
    Pie,
    /// Horizontal bars.
    Bar,
    LinePicture,
    Picture,
    /// A code Ami Pro 3.1 does not write; the record is kept.
    Other(u16),
}

impl ChartType {
    pub fn from_code(code: u16) -> Self {
        match code {
            0 => Self::Column,
            1 => Self::Line,
            2 => Self::Pie,
            3 => Self::Bar,
            4 => Self::LinePicture,
            5 => Self::Picture,
            other => Self::Other(other),
        }
    }

    pub fn code(self) -> u16 {
        match self {
            Self::Column => 0,
            Self::Line => 1,
            Self::Pie => 2,
            Self::Bar => 3,
            Self::LinePicture => 4,
            Self::Picture => 5,
            Self::Other(code) => code,
        }
    }
}

/// The twelve icons of the Charting dialog (ids 2200 to 2211, in the
/// dialog's grid: two columns by six rows, CHV2).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Icon {
    Column,
    StackedColumn,
    Bar,
    StackedBar,
    Line,
    Area,
    /// 2206: one marker per series.
    LinePicture,
    /// 2207: the marker follows the category.
    LineCategoryPicture,
    Pie,
    ExpandedPie,
    Picture,
    StackedPicture,
}

impl Icon {
    pub const ALL: [Icon; 12] = [
        Icon::Column,
        Icon::StackedColumn,
        Icon::Bar,
        Icon::StackedBar,
        Icon::Line,
        Icon::Area,
        Icon::LinePicture,
        Icon::LineCategoryPicture,
        Icon::Pie,
        Icon::ExpandedPie,
        Icon::Picture,
        Icon::StackedPicture,
    ];

    /// The dialog's control id (2200 to 2211).
    pub fn id(self) -> u16 {
        2200 + Self::ALL.iter().position(|icon| *icon == self).unwrap_or(0) as u16
    }
}

/// How the charting dialog's Axis group reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Axis {
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

/// A label as the record keeps it: 20 bytes, NUL padded. A space is
/// stored as `_` (the text form cannot hold spaces); the chart shows it as
/// a space.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct Label(pub [u8; 20]);

impl Label {
    /// A label from text: cut at 19 characters, spaces stored as `_`
    /// (characters Windows-1252 cannot hold become `?`).
    pub fn new(text: &str) -> Self {
        let mut bytes = [0u8; 20];
        for (slot, c) in bytes.iter_mut().zip(text.chars().take(MAX_LABEL)) {
            *slot = if c == ' ' { b'_' } else { sdw::to_cp1252(c) };
        }
        Self(bytes)
    }

    fn used(&self) -> &[u8] {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(20);
        &self.0[..end]
    }

    /// The label as stored (underscores for spaces), the Data box's form.
    pub fn stored(&self) -> String {
        self.used().iter().map(|&b| crate::wmf::cp1252(b)).collect()
    }

    /// The label as drawn: underscores shown as spaces.
    pub fn shown(&self) -> String {
        self.stored().replace('_', " ")
    }

    pub fn is_empty(&self) -> bool {
        self.0[0] == 0
    }
}

/// A chart's data record.
#[derive(Clone, Debug, PartialEq)]
pub struct Chart {
    /// Values per row (A): the categories.
    pub columns: usize,
    /// Rows (B): the series.
    pub rows: usize,
    pub chart_type: ChartType,
    /// The variation byte at `0x1e` (CHV2: clustered 6, stacked 0, area 6,
    /// line 0, 2207 3).
    pub variation: u16,
    /// `0x20`: the Legend box.
    pub legend: u16,
    /// `0x22`: 1 in every record seen.
    pub plane: u16,
    /// `0x24`: the Grid box.
    pub grid: u16,
    /// `0x26`: the depth of oblique 3D, 0 when 3D is off or perspective.
    pub oblique_depth: u16,
    /// `0x28`: the depth of perspective 3D.
    pub perspective_depth: u16,
    /// Row-major with `columns` to a row: `values[row * columns + column]`.
    pub values: Vec<f32>,
    /// One byte a value, written 0 by Ami Pro.
    pub flags: Vec<u8>,
    pub row_labels: Vec<Label>,
    pub column_labels: Vec<Label>,
    /// The word before the axis.
    pub tail_word: u16,
    /// The axis Ami Pro computed (or the Inc, Min, and Max typed).
    pub axis: Axis,
    /// The colour set: 16 entries of red, green, blue, and the pattern
    /// byte (6 solid, 7 hollow, else hatches).
    pub palette: [[u8; 4]; 16],
    /// The last word: the low byte holds flags (stacked, the 2207 marker
    /// rule), the high byte the colour-set index and the colour-by-
    /// category bit (0x10).
    pub last_word: u16,
}

/// The default colour set (`AmiChart.pal` is not in the install set; this
/// is the set the CHV2 charts used for their first entries and the
/// corpus's records for the rest).
pub const DEFAULT_PALETTE: [[u8; 4]; 16] = [
    [0x00, 0x00, 0xff, 6],
    [0x00, 0x80, 0xff, 6],
    [0x00, 0xc2, 0x00, 6],
    [0xff, 0x00, 0x00, 6],
    [0xff, 0xff, 0x00, 6],
    [0xff, 0x81, 0x00, 6],
    [0x00, 0x00, 0x80, 6],
    [0x00, 0x80, 0x00, 6],
    [0x81, 0x00, 0x00, 6],
    [0x00, 0x00, 0x80, 6],
    [0x82, 0x82, 0x82, 6],
    [0x00, 0x00, 0xff, 6],
    [0x00, 0x80, 0xff, 6],
    [0x00, 0xc2, 0x00, 6],
    [0x00, 0xff, 0xff, 6],
    [0xff, 0xff, 0x00, 6],
];

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn f64_at(bytes: &[u8], at: usize) -> f64 {
    let mut raw = [0u8; 8];
    raw.copy_from_slice(&bytes[at..at + 8]);
    f64::from_le_bytes(raw)
}

impl Chart {
    /// The record's length for `columns` by `rows`.
    pub fn record_length(columns: usize, rows: usize) -> usize {
        FIXED + 5 * columns * rows + 20 * (columns + rows)
    }

    /// Reads the body of a type-15 object (the bytes after its 16-byte
    /// header). `None` when it is not a record of the layout seen in every
    /// file (the layout's derived words must be the derived values), so
    /// what the reader does not understand stays as stored.
    pub fn parse(body: &[u8]) -> Option<Chart> {
        if body.len() < FIXED - HEADER {
            return None;
        }
        let columns = usize::from(u16_at(body, 0));
        let rows = usize::from(u16_at(body, 2));
        let cells = columns.checked_mul(rows)?;
        if cells > MAX_CELLS || columns == 0 || rows == 0 {
            return None;
        }
        if body.len() + HEADER != Self::record_length(columns, rows) {
            return None;
        }
        // 0x14, 0x16, 0x18, 0x1a: the memory offsets of the data, the
        // flags, and the labels, and a zero.
        let derived = [0x36 + 4 * cells, 0, 0x36 + 5 * cells];
        if usize::from(u16_at(body, 4)) != 0x36
            || usize::from(u16_at(body, 6)) != derived[0]
            || usize::from(u16_at(body, 8)) != derived[1]
            || usize::from(u16_at(body, 10)) != derived[2]
        {
            return None;
        }
        let values_at = 0x1a;
        let values = (0..cells)
            .map(|index| {
                let at = values_at + 4 * index;
                f32::from_le_bytes([body[at], body[at + 1], body[at + 2], body[at + 3]])
            })
            .collect();
        let flags_at = values_at + 4 * cells;
        let flags = body[flags_at..flags_at + cells].to_vec();
        let labels_at = flags_at + cells;
        let label = |index: usize| {
            let at = labels_at + 20 * index;
            let mut bytes = [0u8; 20];
            bytes.copy_from_slice(&body[at..at + 20]);
            Label(bytes)
        };
        let row_labels = (0..rows).map(label).collect();
        let column_labels = (rows..rows + columns).map(label).collect();
        let tail = labels_at + 20 * (columns + rows);
        let mut palette = [[0u8; 4]; 16];
        for (index, entry) in palette.iter_mut().enumerate() {
            entry.copy_from_slice(&body[tail + 26 + 4 * index..tail + 30 + 4 * index]);
        }
        Some(Chart {
            columns,
            rows,
            chart_type: ChartType::from_code(u16_at(body, 0x0c)),
            variation: u16_at(body, 0x0e),
            legend: u16_at(body, 0x10),
            plane: u16_at(body, 0x12),
            grid: u16_at(body, 0x14),
            oblique_depth: u16_at(body, 0x16),
            perspective_depth: u16_at(body, 0x18),
            values,
            flags,
            row_labels,
            column_labels,
            tail_word: u16_at(body, tail),
            axis: Axis {
                min: f64_at(body, tail + 2),
                max: f64_at(body, tail + 10),
                step: f64_at(body, tail + 18),
            },
            palette,
            last_word: u16_at(body, tail + 90),
        })
    }

    /// The body of the type-15 object, as Ami Pro writes it.
    pub fn to_body(&self) -> Vec<u8> {
        let cells = self.columns * self.rows;
        let mut out = Vec::with_capacity(Self::record_length(self.columns, self.rows) - HEADER);
        let word = |out: &mut Vec<u8>, value: u16| out.extend_from_slice(&value.to_le_bytes());
        word(&mut out, self.columns as u16);
        word(&mut out, self.rows as u16);
        word(&mut out, 0x36);
        word(&mut out, (0x36 + 4 * cells) as u16);
        word(&mut out, 0);
        word(&mut out, (0x36 + 5 * cells) as u16);
        word(&mut out, self.chart_type.code());
        for value in [
            self.variation,
            self.legend,
            self.plane,
            self.grid,
            self.oblique_depth,
            self.perspective_depth,
        ] {
            word(&mut out, value);
        }
        for index in 0..cells {
            let value = self.values.get(index).copied().unwrap_or(0.0);
            out.extend_from_slice(&value.to_le_bytes());
        }
        for index in 0..cells {
            out.push(self.flags.get(index).copied().unwrap_or(0));
        }
        for index in 0..self.rows {
            let label = self.row_labels.get(index).copied().unwrap_or_default();
            out.extend_from_slice(&label.0);
        }
        for index in 0..self.columns {
            let label = self.column_labels.get(index).copied().unwrap_or_default();
            out.extend_from_slice(&label.0);
        }
        word(&mut out, self.tail_word);
        for value in [self.axis.min, self.axis.max, self.axis.step] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        for entry in &self.palette {
            out.extend_from_slice(entry);
        }
        word(&mut out, self.last_word);
        out
    }

    /// The value of series `row` at category `column` (0 outside).
    pub fn value(&self, row: usize, column: usize) -> f32 {
        if row < self.rows && column < self.columns {
            self.values[row * self.columns + column]
        } else {
            0.0
        }
    }

    pub fn legend_on(&self) -> bool {
        self.legend != 0
    }

    pub fn grid_on(&self) -> bool {
        self.grid != 0
    }

    /// 3D: 0 off, 1 oblique, 2 perspective.
    pub fn three_d(&self) -> u8 {
        if self.perspective_depth != 0 {
            2
        } else if self.oblique_depth != 0 {
            1
        } else {
            0
        }
    }

    /// The depth in percent (50 default), of whichever 3D mode is on.
    pub fn depth(&self) -> u16 {
        if self.perspective_depth != 0 {
            self.perspective_depth
        } else {
            self.oblique_depth
        }
    }

    /// Each point takes its own colour (a clustered column, and a chart of
    /// one series with more than one category when it is a pie): bit 0x10
    /// of the last word's high byte.
    pub fn colour_by_category(&self) -> bool {
        self.last_word & 0x1000 != 0
    }

    /// Whether the axis is the one Ami Pro would compute for the data
    /// (Automatic ticked), judged by comparing.
    pub fn axis_is_automatic(&self, plot_height: i32) -> bool {
        let auto = crate::chart_gen::automatic_axis(self, plot_height);
        let same = |a: f64, b: f64| (a - b).abs() < 1e-9 * (1.0 + b.abs());
        same(auto.min, self.axis.min)
            && same(auto.max, self.axis.max)
            && same(auto.step, self.axis.step)
    }

    /// A new chart of `icon` with the data: Ami Pro's defaults (legend on,
    /// grid off, no 3D, depth 50, the default colours). The words the
    /// dialog leaves in `last_word` follow what CHV2 saw (OK-candidate:
    /// the meaning of each bit is not decoded).
    pub fn new(icon: Icon, columns: usize, rows: usize) -> Chart {
        let cells = columns * rows;
        let mut chart = Chart {
            columns,
            rows,
            chart_type: ChartType::Column,
            variation: 6,
            legend: 1,
            plane: 1,
            grid: 0,
            oblique_depth: 0,
            perspective_depth: 0,
            values: vec![0.0; cells],
            flags: vec![0; cells],
            row_labels: vec![Label::default(); rows],
            column_labels: vec![Label::default(); columns],
            tail_word: 0,
            axis: Axis {
                min: 0.0,
                max: 1.0,
                step: 1.0,
            },
            palette: DEFAULT_PALETTE,
            last_word: 0,
        };
        chart.set_icon(icon);
        chart
    }

    /// Chooses one of the twelve types (the icon's type, variation, and
    /// flag words as CHV2 saw them).
    pub fn set_icon(&mut self, icon: Icon) {
        let (chart_type, variation, last) = match icon {
            Icon::Column => (ChartType::Column, 6, 0x1006),
            Icon::StackedColumn => (ChartType::Column, 0, 0x0100),
            Icon::Bar => (ChartType::Bar, 6, 0x0006),
            Icon::StackedBar => (ChartType::Bar, 0, 0x0100),
            Icon::Line => (ChartType::Line, 0, 0x0000),
            Icon::Area => (ChartType::Line, 6, 0x0106),
            Icon::LinePicture => (ChartType::LinePicture, 0, 0x0000),
            Icon::LineCategoryPicture => (ChartType::LinePicture, 3, 0x2003),
            Icon::Pie => (ChartType::Pie, 0, 0x0000),
            Icon::ExpandedPie => (ChartType::Pie, 0, 0x0100),
            Icon::Picture => (ChartType::Picture, 6, 0x0006),
            Icon::StackedPicture => (ChartType::Picture, 0, 0x0100),
        };
        self.chart_type = chart_type;
        self.variation = variation;
        self.last_word = last;
    }

    /// Swaps rows and columns, with their labels (Flip data).
    pub fn flip(&mut self) {
        let (columns, rows) = (self.columns, self.rows);
        let mut values = vec![0.0f32; columns * rows];
        for row in 0..rows {
            for column in 0..columns {
                values[column * rows + row] = self.values[row * columns + column];
            }
        }
        self.values = values;
        self.flags = vec![0; columns * rows];
        std::mem::swap(&mut self.columns, &mut self.rows);
        std::mem::swap(&mut self.row_labels, &mut self.column_labels);
    }
}

/// The chart a drawing holds: the index of its type-15 object and the
/// record read.
pub fn chart_of(drawing: &Drawing) -> Option<(usize, Chart)> {
    let (index, object) = drawing
        .objects
        .iter()
        .enumerate()
        .find(|(_, object)| object.kind == sdw::kind::CHART)?;
    match &object.shape {
        Shape::Other(body) => Chart::parse(body).map(|chart| (index, chart)),
        _ => None,
    }
}

/// The type-15 object for `chart` (flag 4, its rectangle Ami Pro's
/// `(0, 0, 1, 1)`).
pub fn chart_object(chart: &Chart) -> Object {
    Object {
        kind: sdw::kind::CHART,
        flags: sdw::CHART_FLAG,
        rect: sdw::Rect::new(0, 0, 1, 1),
        shape: Shape::Other(chart.to_body()),
    }
}

// ---------------------------------------------------------------- data text

/// The international settings the data parser reads numbers by.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataFormat {
    pub decimal: char,
    pub thousands: char,
    pub currency: String,
}

impl Default for DataFormat {
    fn default() -> Self {
        Self {
            decimal: '.',
            thousands: ',',
            currency: "$".into(),
        }
    }
}

/// Why the Charting dialog refuses the data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataError {
    /// Message 4011: no numbers to chart.
    NoData,
    /// Message 4013: more than 5000 cells.
    TooMuch,
}

/// A parsed data text: the cells, row-major, and the labels.
#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    pub columns: usize,
    pub rows: usize,
    pub values: Vec<f32>,
    pub row_labels: Vec<Label>,
    pub column_labels: Vec<Label>,
}

/// Reads a token as a number: it needs a digit and only digits, the
/// decimal symbol, the thousands symbol (skipped), the currency string
/// (skipped), and `+ - e E`.
fn number(token: &str, format: &DataFormat) -> Option<f32> {
    if !token.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut text = String::new();
    let mut rest = token;
    while !rest.is_empty() {
        if !format.currency.is_empty() && rest.starts_with(format.currency.as_str()) {
            rest = &rest[format.currency.len()..];
            continue;
        }
        let c = rest.chars().next()?;
        rest = &rest[c.len_utf8()..];
        if c == format.thousands && c != format.decimal {
            continue;
        }
        if c == format.decimal {
            text.push('.');
        } else if c.is_ascii_digit() || matches!(c, '+' | '-' | 'e' | 'E') {
            text.push(c);
        } else {
            return None;
        }
    }
    let value: f64 = text.parse().ok()?;
    let max = f64::from(f32::MAX);
    let clamped = value.clamp(-max, max);
    if clamped.abs() < f64::from(f32::MIN_POSITIVE) {
        return Some(0.0);
    }
    Some(clamped as f32)
}

/// Parses the Data box's text as `1018:0078` does: the separator is a tab
/// when the text has one, else a space; blank lines are skipped; on each
/// line the tokens that are numbers are values and the others labels, the
/// first label being the row's; a line of labels only makes the column
/// labels (the first such line); A is the most numbers on a line, B the
/// lines with numbers; short lines are padded with zeros.
pub fn parse_data(text: &str, format: &DataFormat) -> Result<Data, DataError> {
    let separator = if text.contains('\t') { '\t' } else { ' ' };
    let mut rows: Vec<(Label, Vec<f32>)> = Vec::new();
    let mut column_labels: Option<Vec<Label>> = None;
    for line in text.lines() {
        let tokens: Vec<&str> = line
            .split(separator)
            .map(|token| token.trim_matches(|c| c == '\r' || c == '\n'))
            .filter(|token| !token.is_empty())
            .collect();
        if tokens.is_empty() {
            continue;
        }
        let mut label = None;
        let mut values = Vec::new();
        let mut labels = Vec::new();
        for token in tokens {
            match number(token, format) {
                Some(value) => values.push(value),
                None => {
                    let made = Label::new(token);
                    if label.is_none() {
                        label = Some(made);
                    }
                    labels.push(made);
                }
            }
        }
        if values.is_empty() {
            if column_labels.is_none() {
                column_labels = Some(labels);
            }
        } else {
            rows.push((label.unwrap_or_default(), values));
        }
    }
    if rows.is_empty() {
        return Err(DataError::NoData);
    }
    let columns = rows
        .iter()
        .map(|(_, values)| values.len())
        .max()
        .unwrap_or(0);
    if columns.saturating_mul(rows.len()) > MAX_CELLS {
        return Err(DataError::TooMuch);
    }
    let mut values = Vec::with_capacity(columns * rows.len());
    let mut row_labels = Vec::with_capacity(rows.len());
    for (label, row) in &rows {
        row_labels.push(*label);
        values.extend(row.iter().copied());
        values.extend(std::iter::repeat_n(0.0, columns - row.len()));
    }
    let mut column_labels = column_labels.unwrap_or_default();
    column_labels.resize(columns, Label::default());
    Ok(Data {
        columns,
        rows: rows.len(),
        values,
        row_labels,
        column_labels,
    })
}

/// A value as the Data box shows it: the shortest text that reads back as
/// the same `f32`, in the format's decimal symbol.
pub fn format_value(value: f32, format: &DataFormat) -> String {
    let text = format!("{value}");
    if format.decimal == '.' {
        text
    } else {
        text.replace('.', &format.decimal.to_string())
    }
}

impl Chart {
    /// The Data box's text for this chart: a line of column labels (when
    /// there are any), then each series as its label and values, separated
    /// by tabs.
    pub fn data_text(&self, format: &DataFormat) -> String {
        let mut out = String::new();
        if self.column_labels.iter().any(|label| !label.is_empty()) {
            out.push('\t');
            let labels: Vec<String> = self
                .column_labels
                .iter()
                .map(|label| label.stored())
                .collect();
            out.push_str(&labels.join("\t"));
            out.push('\n');
        }
        for row in 0..self.rows {
            let mut cells = vec![self.row_labels[row].stored()];
            for column in 0..self.columns {
                cells.push(format_value(self.value(row, column), format));
            }
            out.push_str(&cells.join("\t"));
            out.push('\n');
        }
        out
    }

    /// Puts parsed data into the chart, keeping its type and options; the
    /// flags and the computed axis are reset (the axis is made again
    /// when generating).
    pub fn set_data(&mut self, data: Data) {
        self.columns = data.columns;
        self.rows = data.rows;
        self.flags = vec![0; data.values.len()];
        self.values = data.values;
        self.row_labels = data.row_labels;
        self.column_labels = data.column_labels;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Chart {
        let mut chart = Chart::new(Icon::Column, 3, 2);
        chart.values = vec![10.0, 20.0, 30.0, 15.0, 5.0, 25.0];
        chart.row_labels = vec![Label::new("East"), Label::new("West")];
        chart.column_labels = vec![Label::new("Jan"), Label::new("Feb"), Label::new("Mar")];
        chart.axis = Axis {
            min: 0.0,
            max: 35.0,
            step: 5.0,
        };
        chart.grid = 1;
        chart.oblique_depth = 50;
        chart
    }

    #[test]
    fn a_record_writes_back_as_read_with_every_field() {
        let chart = sample();
        let body = chart.to_body();
        assert_eq!(body.len() + HEADER, 264);
        let read = Chart::parse(&body).unwrap();
        assert_eq!(read, chart);
        assert_eq!(read.to_body(), body);
        assert_eq!(read.three_d(), 1);
        assert_eq!(read.depth(), 50);
        assert!(read.grid_on() && read.legend_on());
        assert_eq!(read.value(1, 2), 25.0);
        assert_eq!(read.value(5, 5), 0.0);
        assert_eq!(read.column_labels[2].shown(), "Mar");
    }

    #[test]
    fn unknown_words_and_odd_bytes_survive() {
        let mut chart = sample();
        chart.chart_type = ChartType::Other(9);
        chart.plane = 7;
        chart.tail_word = 0xbeef;
        chart.last_word = 0x2003;
        chart.flags[1] = 3;
        chart.palette[5] = [1, 2, 3, 9];
        chart.row_labels[0].0[15] = 0x41; // bytes past the NUL
        let body = chart.to_body();
        assert_eq!(Chart::parse(&body).unwrap().to_body(), body);
        assert_eq!(Chart::parse(&body).unwrap(), chart);
    }

    #[test]
    fn a_record_of_another_shape_is_left_alone() {
        let body = sample().to_body();
        assert!(Chart::parse(&body[..body.len() - 1]).is_none());
        let mut longer = body.clone();
        longer.push(0);
        assert!(Chart::parse(&longer).is_none());
        let mut damaged = body.clone();
        damaged[4] = 0x37;
        assert!(Chart::parse(&damaged).is_none());
        let mut empty = body;
        empty[0] = 0;
        empty[1] = 0;
        assert!(Chart::parse(&empty).is_none());
    }

    #[test]
    fn the_icons_set_type_variation_and_flags() {
        let mut chart = sample();
        for (icon, code, variation, last) in [
            (Icon::StackedBar, 3, 0, 0x0100),
            (Icon::Area, 1, 6, 0x0106),
            (Icon::LineCategoryPicture, 4, 3, 0x2003),
            (Icon::Pie, 2, 0, 0x0000),
            (Icon::ExpandedPie, 2, 0, 0x0100),
        ] {
            chart.set_icon(icon);
            assert_eq!(
                (chart.chart_type.code(), chart.variation, chart.last_word),
                (code, variation, last)
            );
        }
        assert_eq!(Icon::Column.id(), 2200);
        assert_eq!(Icon::StackedPicture.id(), 2211);
        chart.set_icon(Icon::Column);
        assert!(chart.colour_by_category());
    }

    #[test]
    fn flip_swaps_rows_columns_and_labels() {
        let mut chart = sample();
        chart.flip();
        assert_eq!((chart.columns, chart.rows), (2, 3));
        assert_eq!(chart.values, vec![10.0, 15.0, 20.0, 5.0, 30.0, 25.0]);
        assert_eq!(chart.row_labels[1].shown(), "Feb");
        assert_eq!(chart.column_labels[1].shown(), "West");
        chart.flip();
        assert_eq!(chart.values, sample().values);
    }

    fn parse(text: &str) -> Data {
        parse_data(text, &DataFormat::default()).unwrap()
    }

    #[test]
    fn tabs_separate_and_labels_come_first() {
        let data = parse("\tJan\tFeb\tMar\nEast\t10\t20\t30\nWest\t15\t5\t25\n");
        assert_eq!((data.columns, data.rows), (3, 2));
        assert_eq!(data.values, vec![10.0, 20.0, 30.0, 15.0, 5.0, 25.0]);
        assert_eq!(data.row_labels[1].shown(), "West");
        assert_eq!(data.column_labels[2].shown(), "Mar");
        // Spaces inside a tab-separated label become underscores.
        let data = parse("Big Sales\t1\t2\n");
        assert_eq!(data.row_labels[0].stored(), "Big_Sales");
        assert_eq!(data.row_labels[0].shown(), "Big Sales");
    }

    #[test]
    fn spaces_separate_when_there_is_no_tab() {
        let data = parse("Q1 Q2\nA 1 2\n\nB 3 4\n");
        assert_eq!((data.columns, data.rows), (2, 2));
        assert_eq!(data.column_labels[0].shown(), "Q1");
        assert_eq!(data.values, vec![1.0, 2.0, 3.0, 4.0]);
        // Only the first line of labels names the columns.
        let data = parse("X Y\nZ W\nA 1 2\n");
        assert_eq!(data.column_labels[0].shown(), "X");
    }

    #[test]
    fn numbers_take_currency_thousands_signs_and_exponents() {
        let data = parse("$1,234.50 -2 +3 1e3 4E-1\n");
        assert_eq!(data.values, vec![1234.5, -2.0, 3.0, 1000.0, 0.4]);
        assert!(data.row_labels[0].is_empty());
        // "e" alone, "-" alone, and words with digits are labels.
        let data = parse("e - Q1 5\n");
        assert_eq!(data.values, vec![5.0]);
        assert_eq!(data.row_labels[0].stored(), "e");
        // A comma decimal and a point for thousands.
        let format = DataFormat {
            decimal: ',',
            thousands: '.',
            currency: "EUR".into(),
        };
        let data = parse_data("EUR1.234,5 2,25\n", &format).unwrap();
        assert_eq!(data.values, vec![1234.5, 2.25]);
    }

    #[test]
    fn short_lines_pad_labels_cut_and_limits_hold() {
        let data = parse("A 1 2 3\nB 4\n");
        assert_eq!(data.values, vec![1.0, 2.0, 3.0, 4.0, 0.0, 0.0]);
        let data = parse("abcdefghijklmnopqrstuvwxyz 1\n");
        assert_eq!(data.row_labels[0].stored(), "abcdefghijklmnopqrs");
        assert_eq!(
            parse_data("a b c\n", &DataFormat::default()),
            Err(DataError::NoData)
        );
        assert_eq!(
            parse_data("", &DataFormat::default()),
            Err(DataError::NoData)
        );
        let big = format!("{}\n", "1 ".repeat(5001));
        assert_eq!(
            parse_data(&big, &DataFormat::default()),
            Err(DataError::TooMuch)
        );
        let edge = format!("{}\n", "1 ".repeat(5000));
        assert_eq!(
            parse_data(&edge, &DataFormat::default()).unwrap().columns,
            5000
        );
        // Out of range clamps; tiny values are zero.
        let data = parse("1e99 1e-99\n");
        assert_eq!(data.values, vec![f32::MAX, 0.0]);
    }

    #[test]
    fn the_data_text_reads_back_as_the_chart() {
        let chart = sample();
        let text = chart.data_text(&DataFormat::default());
        assert_eq!(text, "\tJan\tFeb\tMar\nEast\t10\t20\t30\nWest\t15\t5\t25\n");
        let data = parse(&text);
        let mut again = chart.clone();
        again.set_data(data);
        assert_eq!(again.values, chart.values);
        assert_eq!(again.row_labels, chart.row_labels);
        assert_eq!(again.column_labels, chart.column_labels);
        // Fractions and underscores survive.
        let mut chart = Chart::new(Icon::Bar, 2, 1);
        chart.values = vec![0.1, 12345.678];
        chart.row_labels = vec![Label::new("Big Sales")];
        let text = chart.data_text(&DataFormat::default());
        let mut again = chart.clone();
        again.set_data(parse(&text));
        assert_eq!(again.values, chart.values);
        assert_eq!(again.row_labels, chart.row_labels);
    }

    #[test]
    fn the_object_form_is_flag_4_and_found_in_a_drawing() {
        let chart = sample();
        let mut drawing = Drawing::new();
        drawing.objects.push(chart_object(&chart));
        let written = sdw::write(&drawing);
        let read = sdw::read(&written).unwrap();
        let (index, found) = chart_of(&read).unwrap();
        assert_eq!(index, 0);
        assert_eq!(found, chart);
        assert_eq!(read.objects[0].flags, 4);
    }
}
