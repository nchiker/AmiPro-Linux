//! The Print panel in the context rail (`docs/Design/UI_SPEC.md` §29, PO1,
//! PO4, PO5): Ami Pro's Print Options, kept for the document until it
//! closes, and the PDF's additions, kept in the settings; the pages show
//! as they will print while the panel is open, and its buttons print and
//! export.

use std::collections::BTreeSet;

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::command::CommandId;
use crate::print::{PdfAdditions, PrintOptions};

/// The panel's folding sections.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Section {
    WhatPrints,
    Pdf,
}

impl Section {
    pub const ALL: [(Self, &'static str); 2] =
        [(Self::WhatPrints, "What prints"), (Self::Pdf, "PDF")];
}

/// One of Ami Pro's Print Options.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrintOption {
    CropMarks,
    WithoutPictures,
    DocDescription,
    WithNotes,
    PreprintedForm,
    UpdateFields,
}

impl PrintOption {
    /// In Ami Pro's order (dialog 1083), with what each does.
    pub const ALL: [(Self, &'static str, &'static str); 6] = [
        (
            Self::CropMarks,
            "Crop marks",
            "The page ½ in in from the paper's corner, marks at its corners.",
        ),
        (
            Self::WithoutPictures,
            "Without pictures",
            "Picture frames print empty.",
        ),
        (
            Self::DocDescription,
            "With doc description",
            "A cover sheet listing Doc Info.",
        ),
        (
            Self::WithNotes,
            "With notes",
            "The notes on pages after the document, under their pages' numbers.",
        ),
        (
            Self::PreprintedForm,
            "On preprinted form",
            "No frame or table lines, shading, pictures, or protected text.",
        ),
        (
            Self::UpdateFields,
            "Update fields first",
            "Update All before printing.",
        ),
    ];

    pub fn get(self, options: PrintOptions) -> bool {
        match self {
            Self::CropMarks => options.crop_marks,
            Self::WithoutPictures => options.without_pictures,
            Self::DocDescription => options.doc_description,
            Self::WithNotes => options.with_notes,
            Self::PreprintedForm => options.preprinted_form,
            Self::UpdateFields => options.update_fields,
        }
    }

    pub fn set(self, options: &mut PrintOptions, on: bool) {
        match self {
            Self::CropMarks => options.crop_marks = on,
            Self::WithoutPictures => options.without_pictures = on,
            Self::DocDescription => options.doc_description = on,
            Self::WithNotes => options.with_notes = on,
            Self::PreprintedForm => options.preprinted_form = on,
            Self::UpdateFields => options.update_fields = on,
        }
    }
}

/// One of the PDF's additions (PO4).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Addition {
    Bookmarks,
    Properties,
    Archival,
}

impl Addition {
    pub const ALL: [(Self, &'static str); 3] = [
        (Self::Bookmarks, "Bookmarks from outline levels"),
        (
            Self::Properties,
            "Doc Info as properties; Ami Pro's page numbers",
        ),
        (Self::Archival, "PDF/A-2b, for archiving"),
    ];

    pub fn get(self, additions: PdfAdditions) -> bool {
        match self {
            Self::Bookmarks => additions.bookmarks,
            Self::Properties => additions.properties,
            Self::Archival => additions.archival,
        }
    }

    pub fn set(self, additions: &mut PdfAdditions, on: bool) {
        match self {
            Self::Bookmarks => additions.bookmarks = on,
            Self::Properties => additions.properties = on,
            Self::Archival => additions.archival = on,
        }
    }
}

#[derive(Clone, Debug)]
pub enum PrintAction {
    Section(Section),
    Option(PrintOption, bool),
    Addition(Addition, bool),
    /// On a master document: print the whole set (MS6) or this document.
    WholeSet(bool),
    Back,
    Close,
}

/// The panel's state: the open sections.
pub struct PrintPanel {
    open: BTreeSet<Section>,
}

impl Default for PrintPanel {
    fn default() -> Self {
        Self {
            open: Section::ALL.iter().map(|(section, _)| *section).collect(),
        }
    }
}

impl PrintPanel {
    pub fn toggle(&mut self, section: Section) {
        if !self.open.remove(&section) {
            self.open.insert(section);
        }
    }
}

/// The rail's width while the panel shows.
pub const WIDTH: f32 = 300.0;

fn send(action: PrintAction) -> Message {
    Message::PrintPanel(action)
}

/// A button the rail's width with its label centred: libcosmic's text
/// buttons give their width to the label's row, not the button.
fn wide_button<'a>(
    label: &'static str,
    class: cosmic::theme::Button,
    command: CommandId,
) -> Element<'a, Message> {
    widget::button::custom(widget::text::body(label).width(Length::Fill).center())
        .class(class)
        .width(Length::Fill)
        .height(32)
        .padding([0, 12])
        .on_press(Message::Command(command))
        .into()
}

pub fn panel<'a>(
    state: &PrintPanel,
    options: PrintOptions,
    additions: PdfAdditions,
    whole_set: Option<bool>,
) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Print").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(PrintAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Print")
            .on_press(send(PrintAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut body: Vec<Element<'a, Message>> = Vec::new();
    // A master document prints its set, as Ami Pro's does (MS6).
    if let Some(whole) = whole_set {
        body.push(widget::text::heading("Master document").into());
        for (value, label) in [
            (
                true,
                "The whole master document (TOC file, files, index file)",
            ),
            (false, "This document only"),
        ] {
            body.push(
                widget::radio(widget::text::body(label), value, Some(whole), |value| {
                    send(PrintAction::WholeSet(value))
                })
                .into(),
            );
        }
    }
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
            .on_press(send(PrintAction::Section(section)))
            .into(),
        );
        if !open {
            continue;
        }
        let rows: Vec<Element<'a, Message>> = match section {
            Section::WhatPrints => {
                let mut rows: Vec<Element<'a, Message>> = PrintOption::ALL
                    .into_iter()
                    .map(|(option, label, help)| {
                        widget::column::with_children(vec![
                            widget::checkbox(option.get(options))
                                .label(label)
                                .on_toggle(move |on| send(PrintAction::Option(option, on)))
                                .into(),
                            widget::container(widget::text::caption(help))
                                .padding([0, 0, 0, 26])
                                .into(),
                        ])
                        .spacing(2)
                        .into()
                    })
                    .collect();
                rows.push(
                    widget::text::caption(
                        "For this document until it closes, as Ami Pro keeps them. Copies, \
                         collate, reverse order, odd and even pages, and trays are in the \
                         print dialog.",
                    )
                    .into(),
                );
                rows
            }
            Section::Pdf => Addition::ALL
                .into_iter()
                .map(|(addition, label)| {
                    widget::checkbox(addition.get(additions))
                        .label(label)
                        .on_toggle(move |on| send(PrintAction::Addition(addition, on)))
                        .into()
                })
                .collect(),
        };
        body.push(
            widget::container(widget::column::with_children(rows).spacing(6))
                .padding([0, 0, 8, 22])
                .into(),
        );
    }
    body.push(
        widget::text::caption("The pages show as they print while this panel is open.").into(),
    );
    // The buttons stay in view below the options, each the rail's width
    // (the user's check, 2026-10-20).
    let buttons = widget::column::with_children(vec![
        wide_button("Print…", cosmic::theme::Button::Suggested, CommandId::Print),
        wide_button(
            "Print Current Page…",
            cosmic::theme::Button::Standard,
            CommandId::PrintCurrentPage,
        ),
        wide_button(
            "Export PDF…",
            cosmic::theme::Button::Standard,
            CommandId::ExportPdf,
        ),
    ])
    .spacing(6);
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(6).into()),
        widget::divider::horizontal::default().into(),
        buttons.into(),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_and_additions_read_and_set_by_their_switches() {
        let mut options = PrintOptions::default();
        for (option, _, _) in PrintOption::ALL {
            assert!(!option.get(options));
            option.set(&mut options, true);
            assert!(option.get(options));
        }
        let mut additions = PdfAdditions::default();
        assert!(Addition::Bookmarks.get(additions));
        assert!(!Addition::Archival.get(additions));
        Addition::Archival.set(&mut additions, true);
        assert!(additions.archival);
        let mut panel = PrintPanel::default();
        panel.toggle(Section::Pdf);
        assert!(!panel.open.contains(&Section::Pdf));
        assert!(panel.open.contains(&Section::WhatPrints));
    }
}
