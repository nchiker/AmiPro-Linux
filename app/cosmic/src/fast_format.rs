//! Format › Fast Format (Ctrl+T, and the toolbar's toggle; `UI_SPEC.md`
//! §39, CM4), as Ami Pro's Text › Fast Format (`COMMANDS_NOTES.md` §2):
//! choosing it picks up the format at the selection's start and turns the
//! mode on; every selection then made with the mouse takes it, each one
//! undo step; choosing it again, or Esc, turns it off. With nothing or a
//! whole paragraph selected it asks, as Ami Pro does, whether to take the
//! text's font and attributes or the paragraph's style (the default).
//!
//! The modern Format Painter (`UI_SPEC.md` §49) rides on the same mode: the
//! toolbar's and menu's Format Painter arms it for one selection only (a
//! second press within [`DOUBLE_PRESS`], or Ctrl+Shift+T, keeps it on), and
//! Copy Format and Paste Format (Ctrl+Alt+C, Ctrl+Alt+V) keep the format
//! apart from the mode. Ctrl+T is still Ami Pro's: on until it or Esc ends it.

use std::time::{Duration, Instant};

use amipro_document::{CharacterFormat, ParagraphFormat, StyleId};
use amipro_editing::{Command, IndentChange, TextPosition};
use cosmic::{Element, widget};

use crate::Message;

/// What the mode applies.
#[derive(Clone, Debug, PartialEq)]
pub struct FastFormat {
    /// The text's direct format, Protected and No Hyphenation left out (Ami
    /// Pro's mask), when text formats were picked up and there are some.
    pub text: Option<CharacterFormat>,
    /// The paragraph's style and own format, when picked up.
    pub paragraph: Option<(StyleId, ParagraphFormat)>,
    /// The picked-up style's name, for the status bar.
    pub style_name: Option<String>,
    /// Stays on after a selection takes it (Ami Pro's Fast Format, Keep On),
    /// not just for the next selection.
    pub sticky: bool,
    /// What the question chose, for the status bar: the style only (`true`)
    /// or the text's font and attributes only (`false`); `None` when both
    /// were taken without asking.
    pub only_style: Option<bool>,
}

/// Two presses of Format Painter this close together are a double-click.
pub const DOUBLE_PRESS: Duration = Duration::from_millis(500);

impl FastFormat {
    /// The picked-up format in words: the style, then the text's face, size,
    /// and attributes ("Heading 1, Arial 12 bold").
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(style) = &self.style_name {
            parts.push(style.clone());
        }
        if let Some(text) = &self.text {
            let mut words = Vec::new();
            if let Some(face) = &text.font_family {
                words.push(face.clone());
            }
            if let Some(size) = text.font_size_points {
                words.push(format!("{size}"));
            }
            for (on, name) in [
                (text.bold, "bold"),
                (text.italic, "italic"),
                (text.underline, "underline"),
                (text.word_underline, "word underline"),
                (text.double_underline, "double underline"),
                (text.strikethrough, "strikethrough"),
                (text.superscript, "superscript"),
                (text.subscript, "subscript"),
            ] {
                if on == Some(true) {
                    words.push(name.to_owned());
                }
            }
            if text.caps.is_some() {
                words.push("caps".into());
            }
            if text.colour.is_some() {
                words.push("colour".into());
            }
            if !words.is_empty() {
                parts.push(words.join(" "));
            }
        }
        if parts.is_empty() {
            "no format".into()
        } else {
            parts.join(", ")
        }
    }

    /// What the status bar says while this is armed.
    pub fn armed_status(&self) -> String {
        let choice = match self.only_style {
            Some(true) => " (style only)",
            Some(false) => " (text only)",
            None => "",
        };
        let end = if self.sticky {
            "Esc or Ctrl+T to end"
        } else {
            "Esc to cancel"
        };
        let stays = if self.sticky { " stays on" } else { "" };
        format!(
            "Format painter{choice}{stays}: {} - drag over text to apply, {end}",
            self.summary()
        )
    }
}

/// Ami Pro's "What formats do you want to extract?" while it asks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FastFormatAsk {
    pub style: bool,
    /// What the answer then does.
    pub then: Then,
}

/// What follows the question.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Then {
    /// Arm the mode, staying on (Ctrl+T, Ctrl+Shift+T).
    #[default]
    KeepOn,
    /// Arm it for one selection (Format Painter).
    OneShot,
    /// Only keep the format for Paste Format (Copy Format).
    Copy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FastAction {
    /// The paragraph style (true) or the text's font and attributes.
    Choose(bool),
    Ok,
    Cancel,
}

/// The format at `at`: what Fast Format would pick up, the text's and the
/// paragraph's as asked.
pub fn pick_up(
    document: &amipro_document::Document,
    at: TextPosition,
    text: bool,
    paragraph: bool,
) -> Option<FastFormat> {
    let source = document.flow(at.flow)?.paragraphs.get(at.paragraph)?;
    let text = text
        .then(|| {
            let mut format = source.format_at(at.byte).clone();
            format.protected = None;
            format.no_hyphenation = None;
            format
        })
        .filter(|format| !format.is_empty());
    let paragraph = paragraph.then(|| (source.style, source.format.clone()));
    let style_name = paragraph
        .as_ref()
        .and_then(|(style, _)| document.styles().get(style).map(|style| style.name.clone()));
    Some(FastFormat {
        text,
        paragraph,
        style_name,
        sticky: true,
        only_style: None,
    })
}

/// Whether Ami Pro asks: no text selected, or a whole paragraph.
pub fn asks(document: &amipro_document::Document, start: TextPosition, end: TextPosition) -> bool {
    if start == end {
        return true;
    }
    let length = document
        .flow(start.flow)
        .and_then(|flow| flow.paragraphs.get(start.paragraph))
        .map_or(0, |paragraph| paragraph.text.len());
    start.flow == end.flow
        && start.paragraph == end.paragraph
        && start.byte == 0
        && end.byte >= length
}

/// The one command that gives the selection `start..end` the picked-up
/// format: the text's over the selected text, the style and the
/// paragraph's own format over the paragraphs it touches.
pub fn apply(format: &FastFormat, start: TextPosition, end: TextPosition) -> Option<Command> {
    let mut commands = Vec::new();
    if let Some(text) = &format.text
        && start != end
    {
        commands.push(Command::FormatCharacters {
            start,
            end,
            format: text.clone(),
        });
    }
    if let Some((style, own)) = &format.paragraph
        && start.flow == end.flow
    {
        let (flow, first, last) = (start.flow, start.paragraph, end.paragraph);
        commands.push(Command::SetParagraphStyle {
            flow,
            first,
            last,
            style: *style,
        });
        commands.push(match own.indents {
            Some(indents) => Command::SetParagraphIndents {
                flow,
                first,
                last,
                indents: IndentChange {
                    all: Some(indents.all),
                    first_line: Some(indents.first),
                    rest: Some(indents.rest),
                    right: Some(indents.right),
                },
            },
            None => Command::RevertParagraphIndents { flow, first, last },
        });
        commands.push(Command::SetParagraphLineSpacing {
            flow,
            first,
            last,
            spacing: own.line_spacing,
        });
        commands.push(Command::SetParagraphAlignment {
            flow,
            first,
            last,
            alignment: own.alignment,
        });
        commands.push(Command::SetParagraphTabs {
            flow,
            first,
            last,
            tabs: own.tabs.clone(),
        });
    }
    match commands.len() {
        0 => None,
        1 => commands.pop(),
        _ => Some(Command::Several(commands)),
    }
}

fn send(action: FastAction) -> Message {
    Message::FastFormat(action)
}

pub fn ask_dialog(ask: FastFormatAsk) -> Element<'static, Message> {
    let choice = |label: &'static str, style: bool| -> Element<'static, Message> {
        widget::radio(label, style, Some(ask.style), |style| {
            send(FastAction::Choose(style))
        })
        .into()
    };
    widget::dialog()
        .title("Fast Format")
        .body("What formats do you want to extract?")
        .control(
            widget::column::with_children(vec![
                choice("Only text font and attributes", false),
                choice("Only paragraph style", true),
            ])
            .spacing(8),
        )
        .primary_action(widget::button::suggested("OK").on_press(send(FastAction::Ok)))
        .secondary_action(widget::button::standard("Cancel").on_press(send(FastAction::Cancel)))
        .into()
}

impl crate::AmiProApp {
    /// Format › Fast Format (Ctrl+T): on, staying on (picking up, or asking
    /// first), or off.
    pub(crate) fn toggle_fast_format(&mut self) {
        self.arm_painter(Then::KeepOn);
    }

    /// Ctrl+Shift+T: the painter on and staying on, or off.
    pub(crate) fn format_painter_keep_on(&mut self) {
        self.arm_painter(Then::KeepOn);
    }

    /// Format Painter (the toolbar's button and the menu's item): one
    /// selection takes the format and the mode ends. Pressed again within
    /// [`DOUBLE_PRESS`] (a double-click) it stays on; pressed later while it
    /// is armed it ends.
    pub(crate) fn format_painter(&mut self, now: Instant) {
        let session = &mut self.sessions[self.active_session];
        let double = session
            .painter_pressed
            .is_some_and(|before| now.saturating_duration_since(before) <= DOUBLE_PRESS);
        if session.fast_format.is_some() {
            if double {
                if let Some(format) = session.fast_format.as_mut() {
                    format.sticky = true;
                    session.status = format.armed_status();
                }
                session.painter_pressed = None;
            } else {
                session.fast_format = None;
                session.status = "Format painter off".into();
            }
            return;
        }
        session.painter_pressed = Some(now);
        self.arm_painter(Then::OneShot);
    }

    fn arm_painter(&mut self, then: Then) {
        let session = &mut self.sessions[self.active_session];
        if session.fast_format.take().is_some() {
            session.status = "Format painter off".into();
            return;
        }
        if session.frame_selection.is_some() {
            return;
        }
        let (start, end) = session.page_selection.ordered();
        if asks(&session.document, start, end) {
            self.fast_format_ask = Some(FastFormatAsk {
                style: session.painter_choice,
                then,
            });
        } else {
            self.start_fast_format(true, true, None, then);
        }
    }

    /// Copy Format (Ctrl+Alt+C): keeps the selection's format for Paste
    /// Format without arming the mouse.
    pub(crate) fn copy_format(&mut self) {
        let session = &mut self.sessions[self.active_session];
        if session.frame_selection.is_some() {
            return;
        }
        let (start, end) = session.page_selection.ordered();
        if asks(&session.document, start, end) {
            self.fast_format_ask = Some(FastFormatAsk {
                style: session.painter_choice,
                then: Then::Copy,
            });
        } else {
            self.start_fast_format(true, true, None, Then::Copy);
        }
    }

    /// Paste Format (Ctrl+Alt+V): the copied format onto the selection (or
    /// the caret's paragraph, for a style), as one undo step.
    pub(crate) fn paste_format(&mut self) {
        let session = &mut self.sessions[self.active_session];
        let Some(format) = session.copied_format.clone() else {
            session.status = "No format copied: use Copy Format (Ctrl+Alt+C) first".into();
            return;
        };
        if session.frame_selection.is_some() {
            return;
        }
        if session.paint_format(&format) {
            session.status = format!("Format pasted: {}", format.summary());
        } else {
            session.status = "Format paste: nothing to change here".into();
        }
    }

    fn start_fast_format(
        &mut self,
        text: bool,
        paragraph: bool,
        only_style: Option<bool>,
        then: Then,
    ) {
        let session = &mut self.sessions[self.active_session];
        let (start, _) = session.page_selection.ordered();
        let picked = pick_up(&session.document, start, text, paragraph).map(|mut format| {
            format.only_style = only_style;
            format
        });
        let Some(mut format) = picked else {
            session.status = "Format painter found nothing to pick up".into();
            return;
        };
        session.copied_format = Some(format.clone());
        match then {
            Then::Copy => {
                session.status =
                    format!("Format copied: {}; Ctrl+Alt+V pastes it", format.summary());
            }
            Then::KeepOn | Then::OneShot => {
                format.sticky = then == Then::KeepOn;
                session.status = format.armed_status();
                session.fast_format = Some(format);
            }
        }
    }

    pub(crate) fn update_fast_format(&mut self, action: FastAction) {
        let Some(ask) = self.fast_format_ask.as_mut() else {
            return;
        };
        match action {
            FastAction::Choose(style) => ask.style = style,
            FastAction::Cancel => self.fast_format_ask = None,
            FastAction::Ok => {
                let (style, then) = (ask.style, ask.then);
                self.fast_format_ask = None;
                // The answer is the next question's first choice.
                self.sessions[self.active_session].painter_choice = style;
                self.start_fast_format(!style, style, Some(style), then);
            }
        }
    }

    /// A selection made with the mouse while the mode is on takes the
    /// picked-up format, as one undo step; a one-shot mode then ends.
    pub(crate) fn fast_format_selection(&mut self) {
        let session = &mut self.sessions[self.active_session];
        let Some(format) = session.fast_format.clone() else {
            return;
        };
        if session.frame_selection.is_some() {
            return;
        }
        if session.paint_format(&format) {
            if format.sticky {
                session.status = format!(
                    "Format painted ({}); still on, Esc or Ctrl+T ends",
                    format.summary()
                );
            } else {
                session.fast_format = None;
                session.status = format!("Format painted ({})", format.summary());
            }
        }
    }
}

impl crate::DocumentSession {
    /// Gives the selection `format` as one undo step; whether anything
    /// changed. The selection stays as it was.
    pub(crate) fn paint_format(&mut self, format: &FastFormat) -> bool {
        let selection = self.page_selection;
        let (start, end) = selection.ordered();
        let Some(command) = apply(format, start, end) else {
            return false;
        };
        if !self.execute_page_command(&command) {
            return false;
        }
        self.page_selection = crate::page_editing::valid_selection(&self.document, selection);
        self.sync_editor_to_page();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bold_word_is_picked_up_without_protection_and_applied_as_one_command() {
        let mut document = amipro_document::Document::from_plain_text("Bold word\nPlain text");
        let flow = document.main_flow_id();
        let at = |paragraph, byte| TextPosition {
            flow,
            paragraph,
            byte,
        };
        document = amipro_editing::apply(
            &document,
            &Command::FormatCharacters {
                start: at(0, 0),
                end: at(0, 4),
                format: CharacterFormat {
                    bold: Some(true),
                    protected: Some(true),
                    ..CharacterFormat::PLAIN
                },
            },
        )
        .unwrap()
        .document;
        assert!(asks(&document, at(0, 1), at(0, 1)));
        assert!(asks(&document, at(1, 0), at(1, 10)));
        assert!(!asks(&document, at(1, 0), at(1, 5)));
        let picked = pick_up(&document, at(0, 1), true, false).unwrap();
        assert_eq!(
            picked.text,
            Some(CharacterFormat {
                bold: Some(true),
                ..CharacterFormat::PLAIN
            })
        );
        assert_eq!(picked.paragraph, None);
        let command = apply(&picked, at(1, 0), at(1, 5)).unwrap();
        let formatted = amipro_editing::apply(&document, &command).unwrap().document;
        let target = &formatted.main_flow().paragraphs[1];
        assert_eq!(target.format_at(2).bold, Some(true));
        assert_eq!(target.format_at(2).protected, None);
        // Plain text picked up as text: nothing to apply.
        let plain = pick_up(&document, at(1, 2), true, false).unwrap();
        assert_eq!(apply(&plain, at(0, 0), at(0, 4)), None);
        // The style: one Several over the touched paragraphs.
        let style = pick_up(&document, at(1, 2), false, true).unwrap();
        assert!(matches!(
            apply(&style, at(0, 0), at(0, 0)),
            Some(Command::Several(_))
        ));
    }
}
