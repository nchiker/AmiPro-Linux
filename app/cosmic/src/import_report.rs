//! The Import Report (`UI_SPEC.md` §31, IM8): what an import changed or
//! left out, by kind with counts, in the rail; Go to walks a kind's places
//! in the text, and Copy puts the report on the clipboard.

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::import::Report;

/// The rail's width while the panel shows.
pub const WIDTH: f32 = 300.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReportAction {
    /// Opens the panel (the status bar's count).
    Show,
    /// The next place of finding `index`.
    GoTo(usize),
    Copy,
    Back,
    Close,
}

fn send(action: ReportAction) -> Message {
    Message::ImportReport(action)
}

/// The report as text: the file, then a line per finding.
pub fn text(report: &Report) -> String {
    format!(
        "Import report: {} ({})\n{}\n\n{}",
        report.file,
        report.format,
        report.heading,
        report.findings.report()
    )
}

/// The sentence above the findings.
pub fn summary(report: &Report) -> String {
    match report.findings.len() {
        0 => format!("{} Nothing is different.", report.heading),
        1 => format!("{} One thing is different:", report.heading),
        count => format!("{} {count} things are different:", report.heading),
    }
}

pub fn panel(report: &Report) -> Element<'_, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Import Report").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(ReportAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Import Report")
            .on_press(send(ReportAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut body: Vec<Element<'_, Message>> = vec![
        widget::text::caption(format!("{} ({})", report.file, report.format))
            .wrapping(cosmic::iced::widget::text::Wrapping::WordOrGlyph)
            .into(),
        widget::text::body(summary(report)).into(),
    ];
    for (index, finding) in report.findings.iter().enumerate() {
        let places = finding
            .markers
            .iter()
            .filter(|marker| report.markers.contains_key(marker))
            .count();
        let mut row: Vec<Element<'_, Message>> = vec![
            widget::text::body(format!("▸ {}", finding.kind.describe(finding.count)))
                .width(Length::Fill)
                .into(),
        ];
        if places > 0 {
            row.push(
                widget::button::text("Go to")
                    .padding([0, 4])
                    .tooltip(if places == 1 {
                        "Show its place".to_owned()
                    } else {
                        format!("Show the next of its {places} places")
                    })
                    .on_press(send(ReportAction::GoTo(index)))
                    .into(),
            );
        }
        body.push(
            widget::row::with_children(row)
                .spacing(4)
                .align_y(cosmic::iced::Alignment::Center)
                .into(),
        );
    }
    body.push(
        widget::text::caption(
            "Save As writes an Ami Pro or AmiPro-Linux document; the file read stays as it was.",
        )
        .into(),
    );
    let copy = widget::button::custom(widget::text::body("Copy").width(Length::Fill).center())
        .class(cosmic::theme::Button::Standard)
        .width(Length::Fill)
        .height(32)
        .on_press(send(ReportAction::Copy));
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
        widget::divider::horizontal::default().into(),
        copy.into(),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}
