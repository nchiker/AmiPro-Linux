//! The colour button and its panel (`docs/Design/UI_SPEC.md` §23,
//! decisions C1–C4): every place a colour is chosen shows one button with
//! the colour and its name; it opens a panel in place with Ami Pro's 15
//! colours, the colours the document uses, Recent, and More colours
//! (libcosmic's picker). One panel is open at a time; the app keeps which
//! colour it is for, because the picker's messages do not say.

use amipro_document::Colour;
use cosmic::iced::{Background, Border, Length};
use cosmic::widget;
use cosmic::widget::color_picker::{ColorPickerModel, ColorPickerUpdate};

use crate::Message;
use crate::character_dialogs::PALETTE;
use crate::colours;

/// The colours a panel can be for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// Text › Font…'s colour, applied with OK.
    FontText,
    /// Style Properties › Font: the style's text.
    StyleText,
    /// Style Properties › Lines: the paragraph rules.
    StyleRule,
    /// Page Layout Properties › Lines: the page's lines.
    PageLines,
    /// Frame Properties: the line, shadow, background, and line between
    /// columns (Ami Pro's Modify Frame Layout's four colour bars).
    FrameLine,
    FrameShadow,
    FrameBackground,
    FrameColumnLine,
    /// Table Properties › Cells: the selected cells' fill and their lines'
    /// colour (`UI_SPEC.md` §32, TD8).
    TableFill,
    TableLines,
    /// Drawing Properties: the objects' line, fill, fill background, and
    /// text (`UI_SPEC.md` §33).
    DrawLine,
    DrawFill,
    DrawBackground,
    DrawText,
    /// The Chart panel's colour set: the selected entry (DO.8).
    ChartColour,
}

/// An open colour panel.
pub struct Panel {
    pub target: Target,
    /// Whether More colours (the picker) shows.
    pub more: bool,
    pub picker: ColorPickerModel,
    /// The colours the document uses, as when the panel opened or last
    /// applied a colour (C2).
    pub in_use: Vec<Colour>,
}

impl Panel {
    pub fn new(target: Target, current: Colour, in_use: Vec<Colour>) -> Self {
        Self {
            target,
            more: false,
            picker: picker(current),
            in_use,
        }
    }
}

/// The picker, showing `current`.
pub fn picker(current: Colour) -> ColorPickerModel {
    let colour = Some(colours::to_color(current));
    ColorPickerModel::new("Hex", "RGB", colour, colour)
        .width(Length::Fill)
        .height(Length::Fixed(140.0))
}

/// What a view needs to show colour buttons: the open panel and the
/// app's Recent list.
#[derive(Clone, Copy)]
pub struct Context<'a> {
    pub open: Option<&'a Panel>,
    pub recent: &'a [u32],
}

impl<'a> Context<'a> {
    /// The colour button for `target` showing `current`, with its panel.
    pub fn control(self, target: Target, current: Colour) -> Element<'a> {
        control(target, current, self.open, self.recent, true)
    }

    /// The same, disabled when not `enabled` (no panel then).
    pub fn control_enabled(self, target: Target, current: Colour, enabled: bool) -> Element<'a> {
        control(target, current, self.open, self.recent, enabled)
    }
}

/// What a colour button and its panel send.
#[derive(Clone, Debug)]
pub enum ColourAction {
    /// The button: opens the panel for `Target` showing the colour, or
    /// closes it when it is open for it.
    Toggle(Target, Colour),
    /// A swatch: the colour applies and the panel closes.
    Choose(Colour),
    /// More colours: the picker shows or hides.
    More,
    /// The picker's own messages.
    Picker(ColorPickerUpdate),
}

fn send(action: ColourAction) -> Message {
    Message::Colour(action)
}

fn picked(update: ColorPickerUpdate) -> Message {
    Message::Colour(ColourAction::Picker(update))
}

/// One colour as a button, with its name as its tooltip.
pub(crate) fn swatch(colour: Colour, selected: bool, message: Message) -> Element<'static> {
    let fill = widget::container(widget::space::horizontal())
        .width(Length::Fixed(18.0))
        .height(Length::Fixed(18.0))
        .class(cosmic::theme::Container::custom(move |theme| {
            let cosmic = theme.cosmic();
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(colours::to_color(colour))),
                border: Border {
                    color: if selected {
                        cosmic.accent_color().into()
                    } else {
                        cosmic.bg_divider().into()
                    },
                    width: if selected { 3.0 } else { 1.0 },
                    radius: 2.0.into(),
                },
                ..Default::default()
            }
        }));
    widget::tooltip(
        widget::button::custom(fill)
            .padding(2)
            .class(cosmic::theme::Button::Icon)
            .on_press(message),
        widget::text::body(colours::name(colour)),
        widget::tooltip::Position::Bottom,
    )
    .into()
}

type Element<'a> = cosmic::Element<'a, Message>;

/// A labelled row of swatches; nothing when it has none.
fn swatch_row(
    label: &'static str,
    colours: &[Colour],
    current: Colour,
) -> Option<Element<'static>> {
    if colours.is_empty() {
        return None;
    }
    let swatches: Vec<Element<'static>> = colours
        .iter()
        .map(|colour| {
            swatch(
                *colour,
                *colour == current,
                send(ColourAction::Choose(*colour)),
            )
        })
        .collect();
    Some(
        widget::column::with_children(vec![
            widget::text::caption(label).into(),
            widget::row::with_children(swatches)
                .spacing(1)
                .wrap()
                .into(),
        ])
        .spacing(2)
        .into(),
    )
}

/// The colour button for `target` showing `current`, with its panel below
/// when `open` is the panel for it; `recent` is the app's Recent list.
pub fn control<'a>(
    target: Target,
    current: Colour,
    open: Option<&'a Panel>,
    recent: &[u32],
    enabled: bool,
) -> Element<'a> {
    let panel = open.filter(|panel| enabled && panel.target == target);
    let button = widget::button::custom(
        widget::row::with_children(vec![
            swatch_fill(current),
            widget::text::body(colours::name(current))
                .width(Length::Fill)
                .into(),
            widget::icon::from_name(if panel.is_some() {
                "pan-up-symbolic"
            } else {
                "pan-down-symbolic"
            })
            .size(16)
            .icon()
            .into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center),
    )
    .class(cosmic::theme::Button::Standard)
    .padding([4, 8])
    .width(Length::Fill)
    .on_press_maybe(enabled.then(|| send(ColourAction::Toggle(target, current))));
    let Some(panel) = panel else {
        return button.into();
    };
    let mut rows: Vec<Element<'a>> = Vec::new();
    let ami_pro: Vec<Colour> = PALETTE.iter().map(|(colour, _)| *colour).collect();
    rows.extend(swatch_row("Ami Pro", &ami_pro, current));
    rows.extend(swatch_row("In this document", &panel.in_use, current));
    let shown: Vec<Colour> = ami_pro.iter().chain(&panel.in_use).copied().collect();
    let recent = colours::Recent::from_numbers(recent).shown(&shown);
    rows.extend(swatch_row("Recent", &recent, current));
    rows.push(
        widget::button::custom(
            widget::row::with_children(vec![
                widget::icon::from_name(if panel.more {
                    "pan-down-symbolic"
                } else {
                    "pan-end-symbolic"
                })
                .size(16)
                .icon()
                .into(),
                widget::text::body("More colours").into(),
            ])
            .spacing(6)
            .align_y(cosmic::iced::Alignment::Center),
        )
        .class(cosmic::theme::Button::Text)
        .padding([2, 0])
        .on_press(send(ColourAction::More))
        .into(),
    );
    if panel.more {
        rows.push(
            panel
                .picker
                .builder(picked)
                .width(Length::Fill)
                .height(Length::Fixed(140.0))
                .build("Recent", "Copy", "Copied")
                .into(),
        );
    }
    let body = widget::container(widget::column::with_children(rows).spacing(8))
        .padding(8)
        .width(Length::Fill)
        .class(cosmic::theme::Container::Card);
    widget::column::with_children(vec![button.into(), body.into()])
        .spacing(4)
        .into()
}

/// The button's own swatch, not a button.
fn swatch_fill(colour: Colour) -> Element<'static> {
    widget::container(widget::space::horizontal())
        .width(Length::Fixed(18.0))
        .height(Length::Fixed(18.0))
        .class(cosmic::theme::Container::custom(move |theme| {
            cosmic::iced::widget::container::Style {
                background: Some(Background::Color(colours::to_color(colour))),
                border: Border {
                    color: theme.cosmic().bg_divider().into(),
                    width: 1.0,
                    radius: 2.0.into(),
                },
                ..Default::default()
            }
        }))
        .into()
}
