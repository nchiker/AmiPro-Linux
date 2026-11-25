//! The parts every modal dialog builds its form from (`UI_SPEC.md` §50,
//! Dialogs), so that labels line up at one width, sections and hints look
//! the same, lengths name their unit, and a dialog's third button looks like
//! its others. The frame itself (title, body, buttons, width, padding) is
//! libcosmic's `widget::dialog`; Esc, Enter, and the first focus are
//! `AmiProApp::dialog_keys`.

#![cfg_attr(not(feature = "page-surface"), allow(dead_code))]

use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget;

use crate::Message;

/// The label column of a form row: wide enough for "Start with number"
/// beside its check box.
pub const LABEL_WIDTH: f32 = 170.0;
/// A short field: a number or a length.
pub const SHORT_FIELD: f32 = 96.0;
/// The unit lengths are shown in (`UI_SPEC.md` §50: "in", no full stop).
pub const LENGTH_UNIT: &str = "in";

/// A form row: the label in the label column, the control beside it.
pub fn row<'a>(label: impl Into<String>, control: Element<'a, Message>) -> Element<'a, Message> {
    widget::row::with_children(vec![
        widget::text::body(label.into())
            .width(Length::Fixed(LABEL_WIDTH))
            .into(),
        control,
    ])
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// A form row whose control grows downwards when it opens (the colour
/// control's palette): the label stays level with the control's button.
pub fn row_top<'a>(
    label: impl Into<String>,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    widget::row::with_children(vec![
        widget::container(widget::text::body(label.into()))
            .width(Length::Fixed(LABEL_WIDTH))
            .padding([6, 0, 0, 0])
            .into(),
        control,
    ])
    .spacing(8)
    .align_y(Alignment::Start)
    .into()
}

/// A control in the field column with no label of its own (a check box or
/// buttons under a labelled field).
pub fn indented<'a>(control: Element<'a, Message>) -> Element<'a, Message> {
    widget::row::with_children(vec![
        widget::space::horizontal()
            .width(Length::Fixed(LABEL_WIDTH))
            .into(),
        control,
    ])
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// A length field with its unit after it.
pub fn length<'a>(field: Element<'a, Message>) -> Element<'a, Message> {
    widget::row::with_children(vec![field, widget::text::body(LENGTH_UNIT).into()])
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
}

/// A section's heading inside a dialog.
pub fn heading<'a>(text: impl Into<String>) -> Element<'a, Message> {
    widget::text::heading(text.into()).into()
}

/// A hint, or why a value is refused, under the field it is about.
pub fn hint<'a>(text: impl Into<String>) -> Element<'a, Message> {
    widget::text::caption(text.into()).into()
}

/// A dialog's third button (bottom left): an alternative to its main action,
/// drawn as a standard button whatever it does.
pub fn alternative<'a>(label: impl Into<String>, message: Option<Message>) -> Element<'a, Message> {
    widget::button::standard(label.into())
        .on_press_maybe(message)
        .into()
}
