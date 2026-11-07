//! Format › Fast Format (Ctrl+T, and the toolbar's toggle; `UI_SPEC.md`
//! §39, CM4), as Ami Pro's Text › Fast Format (`COMMANDS_NOTES.md` §2):
//! choosing it picks up the format at the selection's start and turns the
//! mode on; every selection then made with the mouse takes it, each one
//! undo step; choosing it again, or Esc, turns it off. With nothing or a
//! whole paragraph selected it asks, as Ami Pro does, whether to take the
//! text's font and attributes or the paragraph's style (the default).

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
}

/// Ami Pro's "What formats do you want to extract?" while it asks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FastFormatAsk {
    pub style: bool,
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
    /// Format › Fast Format: on (picking up, or asking first), or off.
    pub(crate) fn toggle_fast_format(&mut self) {
        let session = &mut self.sessions[self.active_session];
        if session.fast_format.take().is_some() {
            session.status = "Fast Format off".into();
            return;
        }
        if session.frame_selection.is_some() {
            return;
        }
        let (start, end) = session.page_selection.ordered();
        if asks(&session.document, start, end) {
            self.fast_format_ask = Some(FastFormatAsk { style: true });
        } else {
            self.start_fast_format(true, true);
        }
    }

    fn start_fast_format(&mut self, text: bool, paragraph: bool) {
        let session = &mut self.sessions[self.active_session];
        let (start, _) = session.page_selection.ordered();
        session.fast_format = pick_up(&session.document, start, text, paragraph);
        session.status = match session.fast_format.as_ref() {
            Some(format) => match &format.style_name {
                Some(style) => format!(
                    "Fast Format on ({style}): select text to format it; Ctrl+T or Esc ends"
                ),
                None => "Fast Format on: select text to format it; Ctrl+T or Esc ends".into(),
            },
            None => "Fast Format found nothing to pick up".into(),
        };
    }

    pub(crate) fn update_fast_format(&mut self, action: FastAction) {
        let Some(ask) = self.fast_format_ask.as_mut() else {
            return;
        };
        match action {
            FastAction::Choose(style) => ask.style = style,
            FastAction::Cancel => self.fast_format_ask = None,
            FastAction::Ok => {
                let style = ask.style;
                self.fast_format_ask = None;
                self.start_fast_format(!style, style);
            }
        }
    }

    /// A selection made with the mouse while the mode is on takes the
    /// picked-up format, as one undo step.
    pub(crate) fn fast_format_selection(&mut self) {
        let session = &mut self.sessions[self.active_session];
        let Some(format) = session.fast_format.clone() else {
            return;
        };
        if session.frame_selection.is_some() {
            return;
        }
        let selection = session.page_selection;
        let (start, end) = selection.ordered();
        let Some(command) = apply(&format, start, end) else {
            return;
        };
        if session.execute_page_command(&command) {
            session.page_selection =
                crate::page_editing::valid_selection(&session.document, selection);
            session.sync_editor_to_page();
            session.status = "Fast Format applied; Ctrl+T or Esc ends".into();
        }
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
