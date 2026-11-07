//! Table Properties in the context rail (`UI_SPEC.md` §32, TD2): Ami Pro's
//! Modify Table Layout, Column/Row Size, Insert Column/Row, and Lines &
//! Color edited beside the document, as Page Layout Properties edits a page
//! layout (§18). Each change applies at once as one table command and one
//! undo step; a typed value applies on Enter or when its box loses focus,
//! and a value Ami Pro would refuse is refused in place. The panel follows
//! the caret's table and its cell selection.

use std::collections::{BTreeMap, BTreeSet};

use amipro_document::{CellLeader, CellLines, Colour, FrameId, Twips};
use amipro_editing::Command;
use amipro_editing::tables::{
    CellChange, LinesChange, TableDefaults, TableEdit, TableStyle, TrackSizes, block_at,
    line_around,
};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::colour_control::{Context, Target};
use crate::style_properties::{RULE_STYLES, inches, parse_length};
use crate::table_commands::TableAt;

/// The rail's width with Table Properties.
pub const WIDTH: f32 = 300.0;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Section {
    Table,
    Tracks,
    Cells,
    /// The caret cell's formula (`UI_SPEC.md` §41, TN1).
    Formula,
}

impl Section {
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::Table, "Table"),
        (Self::Tracks, "Rows and columns"),
        (Self::Cells, "Cells"),
        (Self::Formula, "Formula"),
    ];
}

/// A box the user types a value into.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Field {
    DefaultWidth,
    DefaultColumnGutter,
    DefaultHeight,
    DefaultRowGutter,
    Width,
    ColumnGutter,
    Height,
    RowGutter,
    InsertCount,
}

/// A side of the selected cells, for Lines.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    All,
    Outline,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TableAction {
    Section(Section),
    Text(Field, String),
    Submit(Field),
    Automatic(bool),
    SpanPages(bool),
    Center(bool),
    HonorProtection(bool),
    FixedSizes(bool),
    LineAround(bool),
    LineAroundStyle(usize),
    Style(TableStyle),
    Reset,
    DistributeRows,
    DistributeColumns,
    /// Insert the count of rows (or columns) before or after.
    Insert {
        rows: bool,
        after: bool,
    },
    Lines(Side),
    LineStyle(usize),
    RemoveLines,
    Leader(usize),
    Protect(bool),
    Connect,
    /// The Formula box typed in.
    FormulaText(String),
    /// Enter, or leaving the box.
    FormulaSubmit,
    /// Esc in the box: the cell's formula again.
    FormulaRestore,
    FormulaRemove,
    QuickAdd {
        along_row: bool,
    },
    Back,
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Stay,
    /// The Formula section's change of the caret cell.
    Formula(crate::table_formula::FormulaChange),
    Run(Box<Command>, &'static str),
    Back,
    Close,
}

/// The panel's state: the table shown, the sections open, what is typed.
#[derive(Clone, Debug, PartialEq)]
pub struct TableProperties {
    pub frame: FrameId,
    pub texts: BTreeMap<Field, String>,
    pub open: BTreeSet<Section>,
    pub error: Option<String>,
    /// The style Lines sets (1–12).
    pub line_style: u8,
    /// The formula as typed in the Formula section's box and not applied
    /// yet; `None` shows the cell's own.
    pub formula: Option<String>,
}

const LEADERS: [(CellLeader, &str); 4] = [
    (CellLeader::None, "None"),
    (CellLeader::Dots, "Dots . . ."),
    (CellLeader::Dashes, "Dashes - - -"),
    (CellLeader::Line, "Line ___"),
];

const STYLES: [(TableStyle, &str); 5] = [
    (TableStyle::Grid, "Grid"),
    (TableStyle::Outline, "Outline"),
    (TableStyle::HeaderRow, "Header"),
    (TableStyle::BandedRows, "Banded"),
    (TableStyle::Plain, "Plain"),
];

impl TableProperties {
    pub fn new(frame: FrameId) -> Self {
        Self {
            frame,
            texts: BTreeMap::new(),
            open: [
                Section::Table,
                Section::Tracks,
                Section::Cells,
                Section::Formula,
            ]
            .into(),
            error: None,
            line_style: 1,
            formula: None,
        }
    }

    /// What a box shows: what is typed, else the table's value (empty when
    /// the selected rows or columns differ, Ami Pro's grey box).
    pub fn text(&self, field: Field, at: &TableAt) -> String {
        if let Some(text) = self.texts.get(&field) {
            return text.clone();
        }
        let table = &at.table;
        let same = |values: Vec<Twips>| {
            values
                .first()
                .filter(|first| values.iter().all(|value| value == *first))
                .map_or_else(String::new, |value| inches(*value))
        };
        let columns = at.columns();
        let rows = at.rows();
        match field {
            Field::DefaultWidth => inches(table.layout.column_width),
            Field::DefaultColumnGutter => inches(table.cell_margins.horizontal),
            Field::DefaultHeight => inches(table.layout.row_height),
            Field::DefaultRowGutter => inches(table.cell_margins.vertical),
            Field::Width => same(columns.map(|column| table.columns[column]).collect()),
            Field::ColumnGutter => {
                same(columns.map(|column| table.column_gutter(column)).collect())
            }
            Field::Height if table.row_options.automatic => String::new(),
            Field::Height => same(rows.map(|row| table.rows[row]).collect()),
            Field::RowGutter => same(rows.map(|row| table.row_gutter(row)).collect()),
            Field::InsertCount => "1".into(),
        }
    }

    /// The table's Modify Table Layout values with one changed.
    fn defaults(at: &TableAt, line: Option<u16>) -> TableDefaults {
        let table = &at.table;
        TableDefaults {
            column_width: table.layout.column_width,
            column_gutter: table.cell_margins.horizontal,
            row_height: table.layout.row_height,
            row_gutter: table.cell_margins.vertical,
            automatic: table.row_options.automatic,
            span_pages: table.row_options.span_pages,
            center: table.layout.center,
            honor_protection: table.layout.honor_protection,
            fixed_sizes: table.layout.fixed_sizes,
            line_around: line,
            measured_rows: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        action: TableAction,
        at: &TableAt,
        line: Option<u16>,
        measured_rows: Vec<Twips>,
        cell_formula: Option<&str>,
    ) -> Outcome {
        let frame = at.frame;
        let edit = |edit: TableEdit, done: &'static str| {
            Outcome::Run(Box::new(Command::EditTable { frame, edit }), done)
        };
        let mut defaults = Self::defaults(at, line);
        self.error = None;
        match action {
            TableAction::Section(section) => {
                if !self.open.remove(&section) {
                    self.open.insert(section);
                }
                Outcome::Stay
            }
            TableAction::Text(field, text) => {
                self.texts.insert(field, text);
                Outcome::Stay
            }
            TableAction::Submit(field) => {
                let Some(text) = self.texts.get(&field).cloned() else {
                    return Outcome::Stay;
                };
                if field == Field::InsertCount {
                    return Outcome::Stay;
                }
                let Some(value) = parse_length(&text) else {
                    self.error = Some("Type a length such as 1.50, 3 cm, or 72 pt.".into());
                    return Outcome::Stay;
                };
                self.texts.remove(&field);
                let gutter = matches!(
                    field,
                    Field::DefaultColumnGutter
                        | Field::DefaultRowGutter
                        | Field::ColumnGutter
                        | Field::RowGutter
                );
                if (gutter && value < 0) || (!gutter && value < 10) {
                    self.error = Some(if gutter {
                        "A gutter is not less than 0.".into()
                    } else {
                        "A size is at least 0.01 in.".into()
                    });
                    return Outcome::Stay;
                }
                match field {
                    Field::DefaultWidth => defaults.column_width = value,
                    Field::DefaultColumnGutter => defaults.column_gutter = value,
                    Field::DefaultHeight => defaults.row_height = value,
                    Field::DefaultRowGutter => defaults.row_gutter = value,
                    Field::Width | Field::ColumnGutter | Field::Height | Field::RowGutter => {
                        let mut sizes = TrackSizes::default();
                        match field {
                            Field::Width => sizes.width = Some(value),
                            Field::ColumnGutter => sizes.column_gutter = Some(value),
                            Field::Height => sizes.height = Some(value),
                            _ => sizes.row_gutter = Some(value),
                        }
                        let (rows, columns) = match field {
                            Field::Width | Field::ColumnGutter => (0..0, at.columns()),
                            _ => (at.rows(), 0..0),
                        };
                        return edit(
                            TableEdit::SetSizes {
                                rows,
                                columns,
                                sizes,
                            },
                            "Sizes set",
                        );
                    }
                    Field::InsertCount => unreachable!(),
                }
                edit(TableEdit::SetDefaults(defaults), "Table layout set")
            }
            TableAction::Automatic(on) => {
                defaults.automatic = on;
                defaults.measured_rows = measured_rows;
                edit(TableEdit::SetDefaults(defaults), "Table layout set")
            }
            TableAction::SpanPages(on) => {
                defaults.span_pages = on;
                edit(TableEdit::SetDefaults(defaults), "Table layout set")
            }
            TableAction::Center(on) => {
                defaults.center = on;
                edit(TableEdit::SetDefaults(defaults), "Table layout set")
            }
            TableAction::HonorProtection(on) => {
                defaults.honor_protection = on;
                edit(TableEdit::SetDefaults(defaults), "Table layout set")
            }
            TableAction::FixedSizes(on) => {
                defaults.fixed_sizes = on;
                edit(TableEdit::SetDefaults(defaults), "Table layout set")
            }
            TableAction::LineAround(on) => {
                defaults.line_around = on.then_some(line.unwrap_or(1));
                edit(TableEdit::SetDefaults(defaults), "Line around table set")
            }
            TableAction::LineAroundStyle(index) => {
                defaults.line_around = Some(index as u16 + 1);
                edit(TableEdit::SetDefaults(defaults), "Line around table set")
            }
            TableAction::Style(style) => edit(TableEdit::ApplyStyle(style), "Table style applied"),
            TableAction::Reset => edit(
                TableEdit::ResetSizes {
                    rows: at.rows(),
                    columns: at.columns(),
                },
                "Sizes reset",
            ),
            TableAction::DistributeRows => edit(
                TableEdit::Distribute {
                    rows: Some(if at.selected {
                        at.rows()
                    } else {
                        0..at.table.rows.len()
                    }),
                    columns: None,
                },
                "Rows distributed",
            ),
            TableAction::DistributeColumns => edit(
                TableEdit::Distribute {
                    rows: None,
                    columns: Some(if at.selected {
                        at.columns()
                    } else {
                        0..at.table.columns.len()
                    }),
                },
                "Columns distributed",
            ),
            TableAction::Insert { rows, after } => {
                let count = self
                    .texts
                    .get(&Field::InsertCount)
                    .map_or(Some(1), |text| text.trim().parse::<usize>().ok())
                    .filter(|count| (1..=250).contains(count));
                let Some(count) = count else {
                    self.error = Some("Insert 1 to 250 at a time.".into());
                    return Outcome::Stay;
                };
                if rows {
                    let range = at.rows();
                    let (place, like) = if after {
                        (range.end, range.end - 1)
                    } else {
                        (range.start, range.start)
                    };
                    edit(
                        TableEdit::InsertRows {
                            at: place,
                            count,
                            like: Some(like),
                        },
                        "Rows inserted",
                    )
                } else {
                    let range = at.columns();
                    let (place, like) = if after {
                        (range.end, range.end - 1)
                    } else {
                        (range.start, range.start)
                    };
                    edit(
                        TableEdit::InsertColumns {
                            at: place,
                            count,
                            like: Some(like),
                            room: None,
                        },
                        "Columns inserted",
                    )
                }
            }
            TableAction::Lines(side) => {
                let state = side_state(at, side);
                // A side every selected cell has comes off; otherwise it is
                // set in the chosen style (Ami Pro's checked and unchecked
                // boxes; a mixed one sets).
                let on = state != Some(true);
                let mut change = LinesChange {
                    style: self.line_style,
                    ..LinesChange::default()
                };
                let value = Some(on);
                match side {
                    Side::All => change.all = value,
                    Side::Outline => change.outline = value,
                    Side::Left => change.left = value,
                    Side::Right => change.right = value,
                    Side::Top => change.top = value,
                    Side::Bottom => change.bottom = value,
                }
                edit(
                    TableEdit::FormatCells {
                        cells: at.raw.clone(),
                        change: CellChange::Lines(change),
                    },
                    "Lines set",
                )
            }
            TableAction::LineStyle(index) => {
                self.line_style = index as u8 + 1;
                Outcome::Stay
            }
            TableAction::RemoveLines => edit(
                TableEdit::FormatCells {
                    cells: at.raw.clone(),
                    change: CellChange::Lines(LinesChange {
                        all: Some(false),
                        style: 1,
                        ..LinesChange::default()
                    }),
                },
                "Lines removed",
            ),
            TableAction::Leader(index) => edit(
                TableEdit::FormatCells {
                    cells: at.raw.clone(),
                    change: CellChange::Leader(LEADERS[index.min(3)].0),
                },
                "Leaders set",
            ),
            TableAction::Protect(on) => edit(
                TableEdit::FormatCells {
                    cells: at.raw.clone(),
                    change: CellChange::Protect(on),
                },
                if on {
                    "Cells protected"
                } else {
                    "Cells unprotected"
                },
            ),
            TableAction::Connect => {
                if at.selected {
                    edit(TableEdit::Connect(at.expanded()), "Cells connected")
                } else {
                    let (row, column) = at.caret;
                    edit(TableEdit::Disconnect { row, column }, "Cells disconnected")
                }
            }
            TableAction::FormulaText(text) => {
                self.formula = Some(text);
                Outcome::Stay
            }
            TableAction::FormulaRestore => {
                self.formula = None;
                Outcome::Stay
            }
            TableAction::FormulaSubmit => {
                let Some(text) = self.formula.clone() else {
                    return Outcome::Stay;
                };
                if cell_formula == Some(text.as_str()) {
                    self.formula = None;
                    return Outcome::Stay;
                }
                if text.trim().is_empty() {
                    self.formula = None;
                    return Outcome::Stay;
                }
                Outcome::Formula(crate::table_formula::FormulaChange::Set(text))
            }
            TableAction::FormulaRemove => {
                self.formula = None;
                Outcome::Formula(crate::table_formula::FormulaChange::Remove)
            }
            TableAction::QuickAdd { along_row } => {
                self.formula = None;
                Outcome::Formula(crate::table_formula::FormulaChange::QuickAdd { along_row })
            }
            TableAction::Back => Outcome::Back,
            TableAction::Close => Outcome::Close,
        }
    }
}

/// The colour a cell change sets from a colour panel.
pub fn colour_change(target: Target, colour: Colour) -> Option<CellChange> {
    match target {
        Target::TableFill => Some(CellChange::Fill(Some(colour))),
        Target::TableLines => Some(CellChange::LineColour(colour)),
        _ => None,
    }
}

/// Whether every selected cell has `side` (`Some(true)`), none
/// (`Some(false)`), or some (`None`).
fn side_state(at: &TableAt, side: Side) -> Option<bool> {
    let table = &at.table;
    let columns = table.columns.len();
    let range = &at.raw;
    let mut found = Vec::new();
    for row in range.rows.clone() {
        for column in range.columns.clone() {
            let (top, left, span) = block_at(table, row, column);
            if (top, left) != (row, column) && table.cells[row * columns + column].span.is_covered()
            {
                continue;
            }
            let lines: CellLines = table.cells[top * columns + left].format.lines;
            let bottom = top + usize::from(span.rows.max(1)) - 1;
            let right = left + usize::from(span.columns.max(1)) - 1;
            let has = match side {
                Side::All => {
                    lines.left != 0 && lines.right != 0 && lines.top != 0 && lines.bottom != 0
                }
                Side::Left => lines.left != 0,
                Side::Right => lines.right != 0,
                Side::Top => lines.top != 0,
                Side::Bottom => lines.bottom != 0,
                Side::Outline => {
                    let outer = (column != range.columns.start || lines.left != 0)
                        && (right + 1 < range.columns.end || lines.right != 0)
                        && (row != range.rows.start || lines.top != 0)
                        && (bottom + 1 < range.rows.end || lines.bottom != 0);
                    let edge = column == range.columns.start
                        || right + 1 >= range.columns.end
                        || row == range.rows.start
                        || bottom + 1 >= range.rows.end;
                    if !edge {
                        continue;
                    }
                    outer
                }
            };
            found.push(has);
        }
    }
    if found.iter().all(|has| *has) {
        Some(true)
    } else if found.iter().all(|has| !*has) {
        Some(false)
    } else {
        None
    }
}

fn send(action: TableAction) -> Message {
    Message::TableProperties(action)
}

fn row<'a>(children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    widget::row::with_children(children)
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

fn label(text: &'static str, width: f32) -> Element<'static, Message> {
    widget::text::body(text).width(Length::Fixed(width)).into()
}

fn input(
    state: &TableProperties,
    at: &TableAt,
    field: Field,
    enabled: bool,
) -> Element<'static, Message> {
    let text = state.text(field, at);
    let placeholder = if enabled { "Mixed" } else { "auto" };
    let mut box_ = widget::text_input(placeholder, text).width(Length::Fixed(64.0));
    if enabled {
        box_ = box_
            .on_input(move |text| send(TableAction::Text(field, text)))
            .on_submit(move |_| send(TableAction::Submit(field)))
            .on_unfocus(send(TableAction::Submit(field)));
    }
    box_.into()
}

fn check(
    on: bool,
    text: &'static str,
    enabled: bool,
    action: fn(bool) -> TableAction,
) -> Element<'static, Message> {
    let check = widget::checkbox(on).label(text);
    if enabled {
        check.on_toggle(move |on| send(action(on))).into()
    } else {
        check.into()
    }
}

/// A side toggle showing Ami Pro's three states: on, off, or mixed (a dash).
fn side_toggle(at: &TableAt, side: Side, text: &'static str) -> Element<'static, Message> {
    let state = side_state(at, side);
    let mark = match state {
        Some(true) => "■",
        Some(false) => "□",
        None => "▣",
    };
    widget::button::text(format!("{mark} {text}"))
        .padding([2, 4])
        .on_press(send(TableAction::Lines(side)))
        .into()
}

pub fn panel<'a>(
    state: &TableProperties,
    at: &TableAt,
    line: Option<u16>,
    page_table: bool,
    colours: Context<'a>,
    cell: &crate::table_formula::CellInfo,
) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Table Properties").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(TableAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Table Properties")
            .on_press(send(TableAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let table = &at.table;
    let range = &at.raw;
    let place = if range.is_single() {
        format!("Row {}, column {}", at.caret.0 + 1, at.caret.1 + 1)
    } else {
        format!(
            "Rows {}–{}, columns {}–{}",
            range.rows.start + 1,
            range.rows.end,
            range.columns.start + 1,
            range.columns.end
        )
    };
    let mut body: Vec<Element<'a, Message>> = vec![
        widget::text::caption(format!(
            "{} × {} table; {place}",
            table.columns.len(),
            table.rows.len()
        ))
        .into(),
    ];
    if let Some(problem) = &state.error {
        body.push(widget::text::caption(problem.clone()).into());
    }
    let first = block_at(table, range.rows.start, range.columns.start);
    let first_cell = &table.cells[first.0 * table.columns.len() + first.1];
    for (section, title) in Section::ALL {
        let open = state.open.contains(&section);
        body.push(
            widget::button::custom(
                widget::row::with_children(vec![
                    widget::icon::from_name(if open {
                        "pan-down-symbolic"
                    } else {
                        "pan-end-symbolic"
                    })
                    .size(16)
                    .icon()
                    .into(),
                    widget::text::heading(title).into(),
                ])
                .spacing(6)
                .align_y(cosmic::iced::Alignment::Center),
            )
            .class(cosmic::theme::Button::Text)
            .padding([4, 0])
            .width(Length::Fill)
            .on_press(send(TableAction::Section(section)))
            .into(),
        );
        if !open {
            continue;
        }
        let automatic = table.row_options.automatic;
        let fields: Vec<Element<'a, Message>> = match section {
            Section::Table => vec![
                widget::text::caption("Style").into(),
                widget::row::with_children(
                    STYLES
                        .iter()
                        .map(|(style, name)| {
                            widget::button::standard(*name)
                                .padding([2, 4])
                                .on_press(send(TableAction::Style(*style)))
                                .into()
                        })
                        .collect::<Vec<Element<'a, Message>>>(),
                )
                .spacing(4)
                .wrap()
                .into(),
                widget::text::caption("Default columns").into(),
                row(vec![
                    label("Width", 60.0),
                    input(state, at, Field::DefaultWidth, true),
                    label("Gutter", 44.0),
                    input(state, at, Field::DefaultColumnGutter, true),
                ]),
                widget::text::caption("Default rows").into(),
                check(automatic, "Automatic height", true, TableAction::Automatic),
                row(vec![
                    label("Height", 60.0),
                    input(state, at, Field::DefaultHeight, !automatic),
                    label("Gutter", 44.0),
                    input(state, at, Field::DefaultRowGutter, true),
                ]),
                check(
                    table.row_options.span_pages,
                    "Rows span pages",
                    automatic && page_table,
                    TableAction::SpanPages,
                ),
                check(
                    table.layout.center,
                    "Center table on page",
                    page_table,
                    TableAction::Center,
                ),
                row(vec![
                    check(
                        line.is_some(),
                        "Line around",
                        page_table,
                        TableAction::LineAround,
                    ),
                    widget::dropdown(
                        RULE_STYLES.to_vec(),
                        line.map(|style| usize::from(style.clamp(1, 12)) - 1),
                        |index| send(TableAction::LineAroundStyle(index)),
                    )
                    .width(Length::Fill)
                    .into(),
                ]),
                check(
                    table.layout.honor_protection,
                    "Honor protection",
                    true,
                    TableAction::HonorProtection,
                ),
                check(
                    table.layout.fixed_sizes,
                    "Disable mouse sizing",
                    true,
                    TableAction::FixedSizes,
                ),
            ],
            Section::Tracks => vec![
                row(vec![
                    label("Width", 60.0),
                    input(state, at, Field::Width, true),
                    label("Gutter", 44.0),
                    input(state, at, Field::ColumnGutter, true),
                ]),
                row(vec![
                    label("Height", 60.0),
                    input(state, at, Field::Height, !automatic),
                    label("Gutter", 44.0),
                    input(state, at, Field::RowGutter, true),
                ]),
                widget::button::standard("Reset to the table's sizes")
                    .on_press(send(TableAction::Reset))
                    .into(),
                row(vec![
                    widget::button::standard("Even rows")
                        .on_press_maybe((!automatic).then_some(send(TableAction::DistributeRows)))
                        .into(),
                    widget::button::standard("Even columns")
                        .on_press(send(TableAction::DistributeColumns))
                        .into(),
                ]),
                row(vec![
                    label("Insert", 44.0),
                    input(state, at, Field::InsertCount, true),
                ]),
                row(vec![
                    widget::button::standard("Rows above")
                        .on_press(send(TableAction::Insert {
                            rows: true,
                            after: false,
                        }))
                        .into(),
                    widget::button::standard("Rows below")
                        .on_press(send(TableAction::Insert {
                            rows: true,
                            after: true,
                        }))
                        .into(),
                ]),
                row(vec![
                    widget::button::standard("Columns left")
                        .on_press(send(TableAction::Insert {
                            rows: false,
                            after: false,
                        }))
                        .into(),
                    widget::button::standard("Columns right")
                        .on_press(send(TableAction::Insert {
                            rows: false,
                            after: true,
                        }))
                        .into(),
                ]),
            ],
            Section::Cells => {
                let joined = first.2 != amipro_document::CellSpan::SINGLE;
                vec![
                    widget::text::caption("Lines").into(),
                    widget::row::with_children(vec![
                        side_toggle(at, Side::All, "All"),
                        side_toggle(at, Side::Outline, "Outline"),
                    ])
                    .spacing(4)
                    .into(),
                    widget::row::with_children(vec![
                        side_toggle(at, Side::Left, "Left"),
                        side_toggle(at, Side::Right, "Right"),
                        side_toggle(at, Side::Top, "Top"),
                        side_toggle(at, Side::Bottom, "Bottom"),
                    ])
                    .spacing(4)
                    .wrap()
                    .into(),
                    row(vec![
                        widget::dropdown(
                            RULE_STYLES.to_vec(),
                            Some(usize::from(state.line_style.clamp(1, 12)) - 1),
                            |index| send(TableAction::LineStyle(index)),
                        )
                        .width(Length::Fill)
                        .into(),
                        colours.control(Target::TableLines, first_cell.format.line_colour),
                    ]),
                    widget::button::standard("Remove lines")
                        .on_press(send(TableAction::RemoveLines))
                        .into(),
                    row(vec![
                        label("Fill", 60.0),
                        colours.control(
                            Target::TableFill,
                            first_cell.format.fill.unwrap_or(amipro_document::WHITE),
                        ),
                    ]),
                    row(vec![
                        label("Leader", 60.0),
                        widget::dropdown(
                            LEADERS.iter().map(|(_, name)| *name).collect::<Vec<_>>(),
                            LEADERS
                                .iter()
                                .position(|(leader, _)| *leader == first_cell.format.leader),
                            |index| send(TableAction::Leader(index)),
                        )
                        .width(Length::Fill)
                        .into(),
                    ]),
                    check(
                        first_cell.format.protected,
                        "Protected",
                        true,
                        TableAction::Protect,
                    ),
                    widget::button::standard(if joined && !at.selected {
                        "Disconnect cells"
                    } else {
                        "Connect cells"
                    })
                    .on_press_maybe(
                        ((at.selected && !at.expanded().is_single()) || joined)
                            .then_some(send(TableAction::Connect)),
                    )
                    .into(),
                ]
            }
            Section::Formula => formula_section(state, at, cell),
        };
        body.push(
            widget::container(widget::column::with_children(fields).spacing(6))
                .padding([0, 0, 8, 22])
                .into(),
        );
    }
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(6).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

/// What the Formula box accepts (TN4): Ami Pro's set, then the extras that
/// only the native format keeps (Save As SAM lists them).
pub const FORMULA_HINT: &str = "Operators + - * / %, parentheses, @sum(B2..B5). \
Extras, not kept in Ami Pro files: @avg @min @max @count.";

/// The Formula section (`UI_SPEC.md` §41, TN1): the caret cell's formula
/// typed where it is shown, its result as the style displays it, Quick Add,
/// Remove formula, and the message under the box.
fn formula_section<'a>(
    state: &TableProperties,
    at: &TableAt,
    cell: &crate::table_formula::CellInfo,
) -> Vec<Element<'a, Message>> {
    if at.selected && !at.expanded().is_single() {
        return vec![widget::text::caption("Select one cell to edit its formula.").into()];
    }
    let shown = state
        .formula
        .clone()
        .unwrap_or_else(|| cell.formula.clone().unwrap_or_default());
    let mut fields: Vec<Element<'a, Message>> = vec![
        widget::text::caption(format!("Cell {}", cell.name)).into(),
        widget::text_input("D4*E4 or @sum(B2..B5)", shown)
            .id(widget::Id::new(crate::table_formula::FORMULA_FIELD))
            .on_input(|text| send(TableAction::FormulaText(text)))
            .on_submit(|_| send(TableAction::FormulaSubmit))
            .on_unfocus(send(TableAction::FormulaSubmit))
            .into(),
    ];
    fields.push(widget::text::caption(FORMULA_HINT).into());
    if cell.formula.is_some() {
        fields.push(row(vec![
            label("Result", 60.0),
            widget::text::body(cell.result.clone()).into(),
        ]));
    }
    // The refusal of the last try, else what the cell's own result says.
    if let Some(problem) = state.error.clone().or_else(|| cell.message()) {
        fields.push(widget::text::caption(problem).into());
    }
    let (row_ok, column_ok) = (at.caret.1 > 0, at.caret.0 > 0);
    fields.push(widget::text::caption("Quick Add").into());
    fields.push(row(vec![
        widget::button::standard("Row")
            .on_press_maybe(row_ok.then_some(send(TableAction::QuickAdd { along_row: true })))
            .into(),
        widget::button::standard("Column")
            .on_press_maybe(column_ok.then_some(send(TableAction::QuickAdd { along_row: false })))
            .into(),
    ]));
    fields.push(
        widget::button::standard("Remove formula")
            .on_press_maybe(
                cell.formula
                    .is_some()
                    .then_some(send(TableAction::FormulaRemove)),
            )
            .into(),
    );
    fields
}

/// A frame's Line around table, for the panel.
pub fn frame_line(document: &amipro_document::Document, frame: FrameId) -> Option<u16> {
    document.frame(frame).and_then(line_around)
}
