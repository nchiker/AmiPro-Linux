//! The Frame Properties inspector (`UI_SPEC.md` §11): the selected frame's
//! fields in the order of Ami Pro's Modify Frame Layout, and the commands a
//! new value makes. Lengths show in inches, as the ruler does; a typed value
//! may also end in `in`, `"`, `cm`, `mm`, or `pt`. Colours are `#RRGGBB`.

use amipro_document::{
    Border, Colour, ColumnLine, Document, FrameId, LinePosition, Placement, Rect, RepeatPages,
    Shadow, Size as FrameSize, Twips, Wrap,
};
use amipro_editing::{Command, TextPosition};
use amipro_layout::{Container, PageBox};

use crate::frame_editing::rect_of;

/// A field that takes typed text.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Field {
    Page,
    X,
    Y,
    Width,
    Height,
    LineStyle,
    LineColour,
    ShadowDepth,
    ShadowColour,
    Background,
    /// The line between columns' own colour (Ami Pro's Columns & tabs
    /// colour bar; `UI_SPEC.md` §23).
    ColumnLineColour,
    Corners,
    Columns,
    Gutter,
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,
    Name,
    /// Frame Properties › Picture (`UI_SPEC.md` §25; `picture_inspector`).
    PicturePercent,
    PictureWidth,
    PictureHeight,
    PictureRotation,
    PictureRight,
    PictureDown,
    PictureDescription,
    /// Frame Properties › Picture › Image (DO.5; `picture_inspector`).
    ImageBrightness,
    ImageContrast,
    ImageEdge,
    ImageSmoothing,
}

impl Field {
    /// Whether the field is in the Picture section (`picture_inspector`).
    pub fn is_picture(self) -> bool {
        matches!(
            self,
            Self::PicturePercent
                | Self::PictureWidth
                | Self::PictureHeight
                | Self::PictureRotation
                | Self::PictureRight
                | Self::PictureDown
                | Self::PictureDescription
                | Self::ImageBrightness
                | Self::ImageContrast
                | Self::ImageEdge
                | Self::ImageSmoothing
        )
    }
}

/// Ami Pro's placements, as the inspector offers them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementKind {
    WherePlaced,
    WithParaAbove,
    FlowWithText,
    RepeatAll,
    RepeatRightLeft,
}

impl PlacementKind {
    pub const ALL: [Self; 5] = [
        Self::WherePlaced,
        Self::WithParaAbove,
        Self::FlowWithText,
        Self::RepeatAll,
        Self::RepeatRightLeft,
    ];

    /// Labels in the order of `ALL`.
    pub const LABELS: [&'static str; 5] = [
        "Where placed",
        "With para above",
        "Flow with text",
        "Repeat all pages",
        "Repeat right/left",
    ];

    pub fn of(placement: Placement) -> Option<Self> {
        Some(match placement {
            Placement::Fixed { .. } => Self::WherePlaced,
            Placement::BelowParagraph { .. } => Self::WithParaAbove,
            Placement::Inline { .. } => Self::FlowWithText,
            Placement::Repeating {
                pages: RepeatPages::All,
                ..
            } => Self::RepeatAll,
            Placement::Repeating {
                pages: RepeatPages::RightLeft,
                ..
            } => Self::RepeatRightLeft,
            Placement::Header { .. } | Placement::Footer { .. } | Placement::InCell => return None,
        })
    }
}

pub const WRAPS: [Wrap; 3] = [Wrap::Around, Wrap::NoWrapAround, Wrap::NoWrapBeside];
pub const WRAP_LABELS: [&str; 3] = ["Wrap around", "No wrap around", "No wrap beside"];

pub const LINE_POSITIONS: [LinePosition; 5] = [
    LinePosition::Middle,
    LinePosition::Inside,
    LinePosition::Outside,
    LinePosition::CloseToInside,
    LinePosition::CloseToOutside,
];
pub const LINE_POSITION_LABELS: [&str; 5] = [
    "Middle",
    "Inside",
    "Outside",
    "Close to inside",
    "Close to outside",
];

/// The shadow list: none, Ami Pro's two depths, or a typed depth.
pub const SHADOW_LABELS: [&str; 4] = ["None", "Normal", "Deep", "Custom"];

/// Ami Pro's shadow depths: Normal is 100 twips, Deep 172.
pub const NORMAL_SHADOW: Twips = 100;
pub const DEEP_SHADOW: Twips = 172;

/// Where a shadow falls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShadowSide {
    BottomRight,
    BottomLeft,
    TopLeft,
    TopRight,
}

impl ShadowSide {
    pub const ALL: [Self; 4] = [
        Self::BottomRight,
        Self::BottomLeft,
        Self::TopLeft,
        Self::TopRight,
    ];

    /// Labels in the order of `ALL`.
    pub const LABELS: [&'static str; 4] = ["Bottom right", "Bottom left", "Top left", "Top right"];

    fn of(shadow: Shadow) -> Self {
        match (shadow.left > 0, shadow.top > 0) {
            (false, false) => Self::BottomRight,
            (true, false) => Self::BottomLeft,
            (true, true) => Self::TopLeft,
            (false, true) => Self::TopRight,
        }
    }

    fn shadow(self, depth: Twips, colour: Colour) -> Shadow {
        let (left, top) = match self {
            Self::BottomRight => (false, false),
            Self::BottomLeft => (true, false),
            Self::TopLeft => (true, true),
            Self::TopRight => (false, true),
        };
        Shadow {
            left: if left { depth } else { 0 },
            right: if left { 0 } else { depth },
            top: if top { depth } else { 0 },
            bottom: if top { 0 } else { depth },
            colour,
        }
    }
}

/// A choice made with a control that applies at once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Choice {
    Placement(PlacementKind),
    Wrap(Wrap),
    Opaque(bool),
    /// Left, right, top, bottom.
    Side(usize, bool),
    LinePosition(LinePosition),
    ShadowDepth(Option<Twips>),
    ShadowSide(ShadowSide),
    Balance(bool),
    LineBetween(bool),
    /// Even Columns: uneven interior columns spread evenly again.
    EvenColumns,
    /// Frame Properties › Picture (`UI_SPEC.md` §25; `picture_inspector`).
    PictureSizing(amipro_document::PictureSizing),
    PictureAspect(bool),
    PictureReset,
    PictureFitFrame,
    /// Frame Properties › Object: Convert Object to Picture (`UI_SPEC.md`
    /// §42.1; `picture_inspector`).
    ObjectToPicture,
    /// Frame Properties › Picture › Image (DO.5): Invert, the halftone
    /// mode, and Revert.
    ImageInvert(bool),
    ImageHalftone(amipro_document::Halftone),
    ImageRevert,
}

impl Choice {
    /// Whether the choice is in the Picture section (`picture_inspector`).
    pub fn is_picture(self) -> bool {
        matches!(
            self,
            Self::PictureSizing(_)
                | Self::PictureAspect(_)
                | Self::PictureReset
                | Self::PictureFitFrame
                | Self::ObjectToPicture
                | Self::ImageInvert(_)
                | Self::ImageHalftone(_)
                | Self::ImageRevert
        )
    }
}

/// Twips shown as inches with at most two decimals.
pub fn format_length(twips: Twips) -> String {
    let inches = format!("{:.2}", f64::from(twips) / 1_440.0);
    let inches = inches.trim_end_matches('0').trim_end_matches('.');
    if inches == "-0" {
        "0".into()
    } else {
        inches.into()
    }
}

/// A typed length, as Ami Pro's boxes read one (`lengths.rs`).
pub fn parse_length(text: &str) -> Result<Twips, String> {
    crate::lengths::parse_length(text)
        .ok_or_else(|| format!("{:?} is not a length from 0 to 22 inches", text.trim()))
}

/// `#RRGGBB` for a Windows colour (`0x00BBGGRR`).
pub fn format_colour(colour: Colour) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        colour & 0xFF,
        (colour >> 8) & 0xFF,
        (colour >> 16) & 0xFF
    )
}

pub fn parse_colour(text: &str) -> Result<Colour, String> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return Err("a colour is #RRGGBB".into());
    }
    let value = u32::from_str_radix(hex, 16).map_err(|_| "a colour is #RRGGBB".to_owned())?;
    let (red, green, blue) = (value >> 16, (value >> 8) & 0xFF, value & 0xFF);
    Ok(red | (green << 8) | (blue << 16))
}

/// What the inspector shows for the selected frame on `page`.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameFields {
    pub placement: PlacementKind,
    /// Repeating frames: changes apply on every page of the layout.
    pub repeating: bool,
    /// Where placed frames only.
    pub page: Option<u32>,
    /// The labels for X and Y: page position, or the anchored offsets.
    pub x_label: &'static str,
    pub y_label: &'static str,
    /// Anchored frames take one offset; the other axis follows the anchor.
    pub x_editable: bool,
    pub y_editable: bool,
    pub values: Vec<(Field, String)>,
    pub wrap: Wrap,
    /// The wraps the placement allows.
    pub wraps: Vec<Wrap>,
    pub opaque: bool,
    pub sides: [bool; 4],
    pub line_position: LinePosition,
    /// The shadow's depth, `None` when there is no shadow.
    pub shadow_depth: Option<Twips>,
    pub shadow_side: ShadowSide,
    pub balance: bool,
    pub line_between: bool,
    /// The interior's columns differ in width or gap (ruler drags; RU2).
    pub uneven: bool,
    pub tabs: usize,
    pub macro_name: Option<String>,
}

impl FrameFields {
    pub fn value(&self, field: Field) -> &str {
        self.values
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map_or("", |(_, value)| value)
    }
}

/// The frame's box on `page`.
fn frame_box(page: &PageBox, frame: FrameId) -> Option<&amipro_layout::FrameBox> {
    page.frames
        .iter()
        .find(|frame_box| frame_box.frame == frame)
}

/// The fields of `frame` as drawn on `page`.
pub fn fields(document: &Document, page: &PageBox, frame: FrameId) -> Option<FrameFields> {
    let model = document.frame(frame)?;
    let placement = PlacementKind::of(model.placement)?;
    let drawn = rect_of(frame_box(page, frame)?.bounds);
    let style = &model.style;
    let (x, y, x_label, y_label, x_editable, y_editable) = match model.placement {
        Placement::BelowParagraph { x_offset, .. } => (
            x_offset,
            drawn.top,
            "Offset from left margin",
            "From top (follows the text)",
            true,
            false,
        ),
        Placement::Inline { y_offset, .. } => (
            drawn.left,
            y_offset,
            "From left (follows the text)",
            "Offset from the line",
            false,
            true,
        ),
        _ => (drawn.left, drawn.top, "From left", "From top", true, true),
    };
    let margins = style.interior.margins;
    let mut values = vec![
        (Field::X, format_length(x)),
        (Field::Y, format_length(y)),
        (Field::Width, format_length(drawn.width())),
        (Field::Height, format_length(drawn.height())),
        (Field::LineStyle, style.border.style.to_string()),
        (Field::LineColour, format_colour(style.border.colour)),
        (Field::ShadowColour, format_colour(style.shadow.colour)),
        (Field::Background, format_colour(style.background)),
        (Field::Corners, style.corners.to_string()),
        (Field::Columns, style.interior.columns.to_string()),
        (
            Field::Gutter,
            amipro_editing::LayoutSettings::gutter_shown(&style.interior)
                .map(format_length)
                .unwrap_or_default(),
        ),
        (Field::MarginTop, format_length(margins.top)),
        (Field::MarginRight, format_length(margins.right)),
        (Field::MarginBottom, format_length(margins.bottom)),
        (Field::MarginLeft, format_length(margins.left)),
        (Field::Name, style.name.clone().unwrap_or_default()),
    ];
    let shadow = style.shadow;
    let depth = shadow
        .left
        .max(shadow.right)
        .max(shadow.top)
        .max(shadow.bottom);
    values.push((Field::ShadowDepth, format_length(depth)));
    let page_number = match model.placement {
        Placement::Fixed { page, .. } => {
            values.push((Field::Page, page.to_string()));
            Some(page)
        }
        _ => None,
    };
    let wraps = match model.placement {
        Placement::Inline { .. } => vec![Wrap::Around],
        Placement::BelowParagraph { .. } => vec![Wrap::NoWrapAround, Wrap::NoWrapBeside],
        _ => WRAPS.to_vec(),
    };
    let sides = style.border.sides;
    Some(FrameFields {
        placement,
        repeating: matches!(model.placement, Placement::Repeating { .. }),
        page: page_number,
        x_label,
        y_label,
        x_editable,
        y_editable,
        values,
        wrap: style.wrap,
        wraps,
        opaque: style.opaque,
        sides: [sides.left, sides.right, sides.top, sides.bottom],
        line_position: style.border.position,
        shadow_depth: (depth > 0).then_some(depth),
        shadow_side: ShadowSide::of(shadow),
        balance: style.interior.balance,
        uneven: style.interior.column_edges.is_some(),
        line_between: style.interior.line_between.is_some(),
        tabs: style.interior.tabs.iter().map(Vec::len).sum(),
        macro_name: style.macro_name.clone(),
    })
}

fn set_style(
    document: &Document,
    frame: FrameId,
    change: impl FnOnce(&mut amipro_document::FrameStyle) -> Result<(), String>,
) -> Result<Vec<Command>, String> {
    let mut style = document
        .frame(frame)
        .ok_or("the frame is gone")?
        .style
        .clone();
    change(&mut style)?;
    Ok(vec![Command::SetFrameStyle { frame, style }])
}

fn non_negative(value: Twips, what: &str) -> Result<Twips, String> {
    if value < 0 {
        Err(format!("{what} cannot be negative"))
    } else {
        Ok(value)
    }
}

/// The commands for a typed value in `field`, or why it is refused.
pub fn text_command(
    document: &Document,
    page: &PageBox,
    frame: FrameId,
    field: Field,
    text: &str,
) -> Result<Vec<Command>, String> {
    let model = document.frame(frame).ok_or("the frame is gone")?;
    let drawn = rect_of(
        frame_box(page, frame)
            .ok_or("the frame is not on this page")?
            .bounds,
    );
    let minimum = crate::frame_editing::minimum_size(document, frame);
    match field {
        Field::Page => {
            let number: u32 = text
                .trim()
                .parse()
                .map_err(|_| "a page is a number from 1".to_owned())?;
            if number == 0 {
                return Err("a page is a number from 1".into());
            }
            Ok(vec![Command::SetFramePage {
                frame,
                page: number,
            }])
        }
        Field::X | Field::Y => {
            let value = parse_length(text)?;
            let (dx, dy) = match (field, model.placement) {
                (Field::X, Placement::BelowParagraph { x_offset, .. }) => (value - x_offset, 0),
                (Field::Y, Placement::Inline { y_offset, .. }) => (0, value - y_offset),
                (Field::X, Placement::Fixed { .. } | Placement::Repeating { .. }) => {
                    (value - drawn.left, 0)
                }
                (Field::Y, Placement::Fixed { .. } | Placement::Repeating { .. }) => {
                    (0, value - drawn.top)
                }
                _ => return Err("this position follows the frame's anchor".into()),
            };
            Ok(crate::frame_editing::move_commands(
                document,
                &crate::frame_editing::FrameSelection::one(frame, page.number),
                page,
                dx,
                dy,
            ))
        }
        Field::Width | Field::Height => {
            let value = parse_length(text)?;
            let (least, what) = if field == Field::Width {
                (minimum.0, "width")
            } else {
                (minimum.1, "height")
            };
            if value < least {
                return Err(format!(
                    "the {what} must be at least {} in (the margins plus 0.10 in)",
                    format_length(least)
                ));
            }
            let to = if field == Field::Width {
                Rect::new(drawn.left, drawn.top, drawn.left + value, drawn.bottom)
            } else {
                Rect::new(drawn.left, drawn.top, drawn.right, drawn.top + value)
            };
            Ok(crate::frame_editing::resize_commands(
                document, frame, page, drawn, to,
            ))
        }
        Field::LineStyle => {
            let value: u16 = text
                .trim()
                .parse()
                .map_err(|_| "a line style is a number from 1 to 8".to_owned())?;
            if !(1..=8).contains(&value) {
                return Err("a line style is a number from 1 to 8".into());
            }
            set_style(document, frame, |style| {
                style.border.style = value;
                Ok(())
            })
        }
        Field::LineColour => {
            let colour = parse_colour(text)?;
            set_style(document, frame, |style| {
                style.border.colour = colour;
                Ok(())
            })
        }
        Field::ShadowColour => {
            let colour = parse_colour(text)?;
            set_style(document, frame, |style| {
                style.shadow.colour = colour;
                Ok(())
            })
        }
        Field::ShadowDepth => {
            let depth = non_negative(parse_length(text)?, "a shadow")?;
            if depth > 1_440 {
                return Err("a shadow is at most 1 in deep".into());
            }
            set_style(document, frame, |style| {
                style.shadow = ShadowSide::of(style.shadow).shadow(depth, style.shadow.colour);
                Ok(())
            })
        }
        Field::Background => {
            let colour = parse_colour(text)?;
            set_style(document, frame, |style| {
                style.background = colour;
                Ok(())
            })
        }
        Field::ColumnLineColour => {
            let colour = parse_colour(text)?;
            set_style(document, frame, |style| {
                match style.interior.line_between.as_mut() {
                    Some(line) => {
                        line.colour = colour;
                        Ok(())
                    }
                    None => Err("the frame has no line between columns".into()),
                }
            })
        }
        Field::Corners => {
            let value: u8 = text
                .trim()
                .trim_end_matches('%')
                .trim()
                .parse()
                .map_err(|_| "rounded corners are 0 to 100 percent".to_owned())?;
            if value > 100 {
                return Err("rounded corners are 0 to 100 percent".into());
            }
            set_style(document, frame, |style| {
                style.corners = value;
                Ok(())
            })
        }
        Field::Columns => {
            let value: u8 = text
                .trim()
                .parse()
                .map_err(|_| "columns are 1 to 8".to_owned())?;
            if !(1..=8).contains(&value) {
                return Err("columns are 1 to 8".into());
            }
            set_style(document, frame, |style| {
                let interior = &mut style.interior;
                interior.gutter = amipro_editing::layout_setup::count_gutter(interior);
                interior.columns = value;
                interior.column_edges = None;
                interior.settle_gutter();
                // One tab list per column: new columns start with the first
                // column's tabs, as Ami Pro copies them.
                let first = interior.tabs.first().cloned().unwrap_or_default();
                interior.tabs.resize(usize::from(value), first);
                Ok(())
            })
        }
        Field::Gutter => {
            let gutter = non_negative(parse_length(text)?, "the gutter")?;
            set_style(document, frame, |style| {
                // One column has no gutter (`1238:008a`).
                if style.interior.columns < 2 {
                    return Err("This needs two or more columns".into());
                }
                style.interior.gutter = gutter;
                style.interior.column_edges = None;
                Ok(())
            })
        }
        Field::MarginTop | Field::MarginRight | Field::MarginBottom | Field::MarginLeft => {
            let value = non_negative(parse_length(text)?, "a margin")?;
            set_style(document, frame, |style| {
                let before = style.interior.margins;
                let margins = &mut style.interior.margins;
                let slot = match field {
                    Field::MarginTop => &mut margins.top,
                    Field::MarginRight => &mut margins.right,
                    Field::MarginBottom => &mut margins.bottom,
                    _ => &mut margins.left,
                };
                *slot = value;
                if margins.left + margins.right + crate::frame_editing::MINIMUM_INTERIOR
                    > drawn.width()
                    || margins.top + margins.bottom + crate::frame_editing::MINIMUM_INTERIOR
                        > drawn.height()
                {
                    return Err("the margins leave less than 0.10 in inside the frame".into());
                }
                // Uneven columns as Ami Pro's dialog treats them (`1238:02d0`).
                let width = drawn.width();
                amipro_editing::layout_setup::refit_columns(
                    &mut style.interior,
                    before,
                    width,
                    width,
                )
                .map_err(|refusal| refusal.to_owned())
            })
        }
        Field::Name => {
            let name = text.trim();
            set_style(document, frame, |style| {
                style.name = (!name.is_empty()).then(|| name.to_owned());
                Ok(())
            })
        }
        // The Picture section's fields need the picture's natural size.
        _ => Err("use picture_inspector::text_command".into()),
    }
}

/// Where a frame that becomes anchored is anchored: the start of the first
/// main-text paragraph on `page` whose line starts at or below the frame's
/// top (a With para above frame then stands after the text above it), else
/// the end of the page's last main-text line.
fn anchor_for(document: &Document, page: &PageBox, top: f32) -> Option<TextPosition> {
    let main = document.main_flow_id();
    let lines: Vec<_> = page
        .lines
        .iter()
        .filter(|line| line.container == Container::Main)
        .collect();
    if let Some(line) = lines.iter().find(|line| line.top + 0.5 >= top) {
        return Some(TextPosition {
            flow: main,
            paragraph: line.paragraph_index,
            byte: 0,
        });
    }
    lines.last().map(|line| TextPosition {
        flow: main,
        paragraph: line.paragraph_index,
        byte: line.bytes.end,
    })
}

/// The commands for a choice, or why it is refused.
pub fn choice_command(
    document: &Document,
    page: &PageBox,
    frame: FrameId,
    choice: Choice,
) -> Result<Vec<Command>, String> {
    let model = document.frame(frame).ok_or("the frame is gone")?;
    let frame_box = frame_box(page, frame).ok_or("the frame is not on this page")?;
    let drawn = rect_of(frame_box.bounds);
    match choice {
        Choice::Placement(kind) => {
            if PlacementKind::of(model.placement) == Some(kind) {
                return Ok(Vec::new());
            }
            if model.group.is_some() {
                return Err("ungroup the frame before changing its placement".into());
            }
            let size = FrameSize {
                width: drawn.width(),
                height: drawn.height(),
            };
            let column_left = page.columns.first().map_or(0.0, |column| column.x);
            let (placement, anchor) = match kind {
                PlacementKind::WherePlaced => (
                    Placement::Fixed {
                        page: page.number,
                        rect: drawn,
                    },
                    None,
                ),
                PlacementKind::WithParaAbove => (
                    Placement::BelowParagraph {
                        x_offset: (frame_box.bounds.x - column_left).round() as Twips,
                        size,
                    },
                    Some(
                        anchor_for(document, page, frame_box.bounds.y)
                            .ok_or("there is no text on this page to anchor to")?,
                    ),
                ),
                PlacementKind::FlowWithText => (
                    Placement::Inline { y_offset: 0, size },
                    Some(
                        anchor_for(document, page, frame_box.bounds.y)
                            .ok_or("there is no text on this page to anchor to")?,
                    ),
                ),
                PlacementKind::RepeatAll | PlacementKind::RepeatRightLeft => (
                    Placement::Repeating {
                        pages: if kind == PlacementKind::RepeatAll {
                            RepeatPages::All
                        } else {
                            RepeatPages::RightLeft
                        },
                        rect: drawn,
                    },
                    None,
                ),
            };
            Ok(vec![Command::ChangePlacement {
                frame,
                placement,
                anchor,
                layout: Some(page.layout),
            }])
        }
        Choice::Wrap(wrap) => set_style(document, frame, |style| {
            style.wrap = wrap;
            Ok(())
        }),
        Choice::Opaque(opaque) => set_style(document, frame, |style| {
            style.opaque = opaque;
            Ok(())
        }),
        Choice::Side(index, on) => set_style(document, frame, |style| {
            let sides = &mut style.border.sides;
            let side = match index {
                0 => &mut sides.left,
                1 => &mut sides.right,
                2 => &mut sides.top,
                _ => &mut sides.bottom,
            };
            *side = on;
            Ok(())
        }),
        Choice::LinePosition(position) => set_style(document, frame, |style| {
            style.border = Border {
                position,
                ..style.border
            };
            Ok(())
        }),
        Choice::ShadowDepth(depth) => set_style(document, frame, |style| {
            style.shadow =
                ShadowSide::of(style.shadow).shadow(depth.unwrap_or(0), style.shadow.colour);
            Ok(())
        }),
        Choice::ShadowSide(side) => set_style(document, frame, |style| {
            let shadow = style.shadow;
            let depth = shadow
                .left
                .max(shadow.right)
                .max(shadow.top)
                .max(shadow.bottom);
            style.shadow = side.shadow(depth, shadow.colour);
            Ok(())
        }),
        Choice::Balance(balance) => set_style(document, frame, |style| {
            style.interior.balance = balance;
            Ok(())
        }),
        Choice::LineBetween(on) => set_style(document, frame, |style| {
            style.interior.line_between = on.then_some(ColumnLine {
                style: style.border.style.max(1),
                colour: style.border.colour,
            });
            Ok(())
        }),
        Choice::EvenColumns => set_style(document, frame, |style| {
            amipro_editing::layout_setup::even_columns(&mut style.interior);
            Ok(())
        }),
        // The Picture section's choices need the picture's natural size.
        _ => Err("use picture_inspector::choice_command".into()),
    }
}

#[cfg(test)]
mod tests {
    use amipro_document::FrameStyle;
    use amipro_editing::apply;

    use super::*;
    use crate::page_layout::SessionPages;

    fn fixture() -> (Document, FrameId, SessionPages) {
        let mut document = Document::from_plain_text(
            &(0..12)
                .map(|index| format!("Paragraph {index} with a few words."))
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let frame = document.create_text_frame(
            Placement::Fixed {
                page: 1,
                rect: Rect::new(2_880, 4_320, 7_200, 7_200),
            },
            FrameStyle::default(),
        );
        document.frame_stack_mut().push(frame);
        let mut pages = SessionPages::default();
        pages.update(&document);
        (document, frame, pages)
    }

    fn run(document: &Document, commands: Vec<Command>) -> Document {
        commands.iter().fold(document.clone(), |document, command| {
            apply(&document, command).unwrap().document
        })
    }

    #[test]
    fn uneven_interior_columns_show_and_even_out() {
        let (mut document, frame, _) = fixture();
        // Three columns in a 3 in frame with 0.1 in margins (4,032 twips of
        // text), gaps 144 and 360.
        let interior = &mut document.frame_mut(frame).unwrap().style.interior;
        interior.set_edges(
            vec![
                amipro_document::ColumnEdges {
                    left: 0,
                    right: 1_000,
                },
                amipro_document::ColumnEdges {
                    left: 1_144,
                    right: 2_500,
                },
                amipro_document::ColumnEdges {
                    left: 2_860,
                    right: 4_032,
                },
            ],
            4_032,
        );
        document.validate().unwrap();
        let mut pages = SessionPages::default();
        pages.update(&document);
        let page = &pages.layout().pages[0];
        let shown = fields(&document, page, frame).unwrap();
        assert!(shown.uneven);
        assert_eq!(shown.value(Field::Gutter), "", "gaps differ");
        // A wider left margin moves only the first column's left edge.
        let margin = run(
            &document,
            text_command(&document, page, frame, Field::MarginLeft, "0.2").unwrap(),
        );
        let edges = margin
            .frame(frame)
            .unwrap()
            .style
            .interior
            .column_edges
            .clone()
            .unwrap();
        assert_eq!((edges[0].left, edges[0].right), (0, 856));
        assert_eq!((edges[2].left, edges[2].right), (2_716, 3_888));
        let even = run(
            &document,
            choice_command(&document, page, frame, Choice::EvenColumns).unwrap(),
        );
        assert_eq!(even.frame(frame).unwrap().style.interior.column_edges, None);
    }

    #[test]
    fn lengths_and_colours_parse_with_units_and_refuse_nonsense() {
        assert_eq!(parse_length("1.5"), Ok(2_160));
        assert_eq!(parse_length("1.5 in"), Ok(2_160));
        assert_eq!(parse_length("2\""), Ok(2_880));
        assert_eq!(parse_length("2.54cm"), Ok(1_440));
        assert_eq!(parse_length("10 mm"), Ok(567));
        assert_eq!(parse_length("72pt"), Ok(1_440));
        assert!(parse_length("wide").is_err());
        assert!(parse_length("1e9").is_err());
        assert_eq!(format_length(2_160), "1.5");
        assert_eq!(format_length(1_440), "1");
        assert_eq!(format_length(144), "0.1");
        assert_eq!(
            format_colour(0x00FF_0000),
            "#0000FF",
            "Windows colours are BGR"
        );
        assert_eq!(parse_colour("#0000ff"), Ok(0x00FF_0000));
        assert_eq!(parse_colour("FF8000"), Ok(0x0000_80FF));
        assert!(parse_colour("#12345").is_err());
    }

    #[test]
    fn fields_show_the_frame_and_typed_values_become_commands() {
        let (document, frame, pages) = fixture();
        let page = &pages.layout().pages[0];
        let shown = fields(&document, page, frame).unwrap();
        assert_eq!(shown.placement, PlacementKind::WherePlaced);
        assert_eq!(shown.page, Some(1));
        assert_eq!(shown.value(Field::X), "2");
        assert_eq!(shown.value(Field::Y), "3");
        assert_eq!(shown.value(Field::Width), "3");
        assert_eq!(shown.value(Field::Height), "2");
        assert_eq!(shown.value(Field::MarginTop), "0.1");
        assert_eq!(shown.value(Field::Background), "#FFFFFF");

        let moved = run(
            &document,
            text_command(&document, page, frame, Field::X, "2.5").unwrap(),
        );
        assert_eq!(
            moved.frame(frame).unwrap().placement,
            Placement::Fixed {
                page: 1,
                rect: Rect::new(3_600, 4_320, 7_920, 7_200)
            }
        );
        let wider = run(
            &document,
            text_command(&document, page, frame, Field::Width, "4 in").unwrap(),
        );
        assert_eq!(
            wider.frame(frame).unwrap().placement,
            Placement::Fixed {
                page: 1,
                rect: Rect::new(2_880, 4_320, 8_640, 7_200)
            }
        );
        let error = text_command(&document, page, frame, Field::Height, "0.2").unwrap_err();
        assert!(error.contains("at least 0.3"), "{error}");
        assert!(text_command(&document, page, frame, Field::Corners, "150").is_err());
        assert!(text_command(&document, page, frame, Field::Columns, "9").is_err());
        // One column has no gutter: Ami Pro works out 240 (`1238:0000`).
        assert!(text_command(&document, page, frame, Field::Gutter, "0.5").is_err());
        assert!(text_command(&document, page, frame, Field::MarginLeft, "-1").is_err());
        assert!(
            text_command(&document, page, frame, Field::MarginLeft, "2.9").is_err(),
            "margins that leave no room are refused"
        );

        let columns = run(
            &document,
            text_command(&document, page, frame, Field::Columns, "3").unwrap(),
        );
        let interior = &columns.frame(frame).unwrap().style.interior;
        assert_eq!((interior.columns, interior.tabs.len()), (3, 3));
        columns.validate().unwrap();
        // A wide gutter, then one column again: 240, as a file gives back.
        let wide = run(
            &columns,
            text_command(&columns, page, frame, Field::Gutter, "0.5").unwrap(),
        );
        let single = run(
            &wide,
            text_command(&wide, page, frame, Field::Columns, "1").unwrap(),
        );
        assert_eq!(single.frame(frame).unwrap().style.interior.gutter, 240);
        let named = run(
            &document,
            text_command(&document, page, frame, Field::Name, " Logo ").unwrap(),
        );
        assert_eq!(
            named.frame(frame).unwrap().style.name.as_deref(),
            Some("Logo")
        );
        let moved_page = run(
            &document,
            text_command(&document, page, frame, Field::Page, "2").unwrap(),
        );
        assert!(matches!(
            moved_page.frame(frame).unwrap().placement,
            Placement::Fixed { page: 2, .. }
        ));
    }

    #[test]
    fn choices_set_the_look_and_change_the_placement_with_an_anchor() {
        let (document, frame, pages) = fixture();
        let page = &pages.layout().pages[0];
        let shadowed = run(
            &document,
            choice_command(
                &document,
                page,
                frame,
                Choice::ShadowDepth(Some(DEEP_SHADOW)),
            )
            .unwrap(),
        );
        let shadowed = run(
            &shadowed,
            choice_command(
                &shadowed,
                page,
                frame,
                Choice::ShadowSide(ShadowSide::TopLeft),
            )
            .unwrap(),
        );
        let shadow = shadowed.frame(frame).unwrap().style.shadow;
        assert_eq!(
            (shadow.left, shadow.top, shadow.right, shadow.bottom),
            (172, 172, 0, 0)
        );
        let open = run(
            &document,
            choice_command(&document, page, frame, Choice::Side(2, false)).unwrap(),
        );
        assert!(!open.frame(frame).unwrap().style.border.sides.top);

        // Where placed to With para above: anchored before the paragraph
        // at the frame's top, keeping the size and the left offset.
        let commands = choice_command(
            &document,
            page,
            frame,
            Choice::Placement(PlacementKind::WithParaAbove),
        )
        .unwrap();
        let anchored = run(&document, commands);
        let placement = anchored.frame(frame).unwrap().placement;
        let column_left = page.columns[0].x.round() as Twips;
        assert_eq!(
            placement,
            Placement::BelowParagraph {
                x_offset: 2_880 - column_left,
                size: FrameSize {
                    width: 4_320,
                    height: 2_880
                }
            }
        );
        assert_eq!(
            anchored.frame(frame).unwrap().style.wrap,
            Wrap::NoWrapBeside,
            "as Ami Pro chooses (F16)"
        );
        anchored.validate().unwrap();
        // The wraps a placement allows.
        let mut anchored_pages = SessionPages::default();
        anchored_pages.update(&anchored);
        let anchored_page = anchored_pages
            .layout()
            .pages
            .iter()
            .find(|page| page.frames.iter().any(|frame_box| frame_box.frame == frame))
            .unwrap();
        let shown = fields(&anchored, anchored_page, frame).unwrap();
        assert_eq!(shown.wraps, [Wrap::NoWrapAround, Wrap::NoWrapBeside]);
        assert!(!shown.y_editable);
        assert!(
            choice_command(&anchored, anchored_page, frame, Choice::Wrap(Wrap::Around))
                .and_then(|commands| commands.iter().try_for_each(|command| apply(
                    &anchored, command
                )
                .map(drop)
                .map_err(str::to_owned)))
                .is_err(),
            "With para above frames do not wrap around"
        );
        // And to Repeat all pages: on every page of the layout.
        let repeating = run(
            &document,
            choice_command(
                &document,
                page,
                frame,
                Choice::Placement(PlacementKind::RepeatAll),
            )
            .unwrap(),
        );
        assert!(repeating.document_layout().repeating.contains(&frame));
        repeating.validate().unwrap();
    }
}
