//! Edit › Links… in the rail (`docs/Design/UI_SPEC.md` §42.1, OD7; run D,
//! DO.3): a read-only list of the document's OLE objects and linked frames,
//! each with its kind, source, and status. Ami Pro's Link Options updates,
//! unlinks, and edits links through the servers; this app has none, so the
//! list only shows what the file holds. A row selects its frame, whose
//! Frame Properties show the Object section.

use cosmic::iced::Length;
use cosmic::{Element, widget};

use amipro_document::{Document, FrameContent, FrameId, PictureFormat, PictureSource};
use amipro_pictures::ole::{self, Kind};

use crate::Message;

/// The panel's width in the rail.
pub const WIDTH: f32 = 300.0;

/// What the empty list says.
pub const EMPTY: &str = "This document holds no links or OLE objects.";
/// What the panel says about changing links.
pub const NOTE: &str = "Updating and unlinking need the program that made the data: this list only shows what the file holds. Select a row to see its frame's properties.";

/// The panel's state: the row shown selected.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LinksPanel {
    pub selected: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LinksAction {
    Select(usize),
    Close,
}

/// One link or object in the document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkRow {
    pub frame: FrameId,
    pub kind: &'static str,
    pub source: String,
    pub status: &'static str,
}

/// The document's links in frame order: OLE objects (embedded, linked, or
/// whose stream cannot be read) and frames Ami Pro marks as linked to a
/// DDE source (flag `0x200000`: application, topic, item).
pub fn rows(document: &Document) -> Vec<LinkRow> {
    let mut out = Vec::new();
    for frame in document.frames() {
        if let FrameContent::Picture(picture) = &frame.content
            && picture.format() == PictureFormat::Ole
        {
            let bytes = match &picture.source {
                PictureSource::Embedded(object) => {
                    document.objects().get(object).map(|object| &object.bytes)
                }
                PictureSource::Linked(_) => None,
            };
            let row = match bytes.and_then(|bytes| ole::read(bytes).map(|read| (bytes, read))) {
                Some((bytes, object)) if object.kind == Kind::Linked => LinkRow {
                    frame: frame.id,
                    kind: "OLE link",
                    source: [object.label(), object.topic.clone(), object.item.clone()]
                        .into_iter()
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>()
                        .join(" · "),
                    status: if object.metafile(bytes).is_some() {
                        "Not updated here; the stored picture is shown"
                    } else {
                        "Not updated here"
                    },
                },
                Some((bytes, object)) => LinkRow {
                    frame: frame.id,
                    kind: "OLE object",
                    source: object.label(),
                    status: if object.metafile(bytes).is_some() {
                        "Kept; drawn from its stored picture"
                    } else {
                        "Kept; no stored picture to draw"
                    },
                },
                None => LinkRow {
                    frame: frame.id,
                    kind: "OLE object",
                    source: "Unreadable stream".into(),
                    status: "Kept as stored",
                },
            };
            out.push(row);
            continue;
        }
        let linked = frame
            .legacy
            .get("frame.flags.unknown")
            .and_then(|flags| u32::from_str_radix(flags.trim_start_matches("0x"), 16).ok())
            .is_some_and(|flags| flags & 0x20_0000 != 0);
        if linked {
            let source = frame
                .legacy
                .get("frame.extra")
                .map(|extra| {
                    extra
                        .split('\n')
                        .filter(|part| !part.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join(" · ")
                })
                .unwrap_or_default();
            out.push(LinkRow {
                frame: frame.id,
                kind: "DDE link",
                source,
                status: "Not updated here; the stored copy is shown",
            });
        }
    }
    out
}

fn send(action: LinksAction) -> Message {
    Message::Links(action)
}

/// The panel.
pub fn panel<'a>(state: &LinksPanel, rows: &[LinkRow]) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Links").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close the Links panel")
            .on_press(send(LinksAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut children: Vec<Element<'a, Message>> = vec![header.into()];
    if rows.is_empty() {
        children.push(widget::text::body(EMPTY).into());
    }
    for (index, row) in rows.iter().enumerate() {
        let selected = state.selected == Some(index);
        let content = widget::column::with_children(vec![
            widget::text::body(row.kind).into(),
            widget::text::caption(row.source.clone()).into(),
            widget::text::caption(row.status).into(),
        ])
        .spacing(2);
        let item = widget::container(content)
            .padding([4, 6])
            .width(Length::Fill)
            .class(if selected {
                cosmic::theme::Container::Card
            } else {
                cosmic::theme::Container::Transparent
            });
        children.push(
            widget::mouse_area(item)
                .on_press(send(LinksAction::Select(index)))
                .into(),
        );
    }
    children.push(widget::text::caption(NOTE).into());
    widget::scrollable(
        widget::column::with_children(children)
            .spacing(4)
            .padding([8, 8]),
    )
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pictures::ole_fixture;

    #[test]
    fn the_list_has_the_ole_objects_in_frame_order() {
        let (document, frames) = ole_fixture::document(&[
            ole_fixture::stream("MSWordArt.2", &[1; 10], &[]),
            ole_fixture::stream("123Worksheet", &[1; 10], &[3; 5]),
            b"junk".to_vec(),
        ]);
        let listed = rows(&document);
        assert_eq!(listed.len(), 3);
        assert_eq!(listed[0].frame, frames[0]);
        assert_eq!(listed[0].kind, "OLE object");
        assert_eq!(listed[0].source, "WordArt");
        assert_eq!(listed[0].status, "Kept; drawn from its stored picture");
        assert_eq!(listed[1].source, "1-2-3 Worksheet");
        assert_eq!(listed[2].source, "Unreadable stream");
        assert_eq!(listed[2].status, "Kept as stored");
        assert!(rows(&Document::from_plain_text("none")).is_empty());
    }

    #[test]
    fn a_frame_with_the_link_flag_lists_its_application_topic_and_item() {
        let (mut document, frames) =
            ole_fixture::document(&[ole_fixture::stream("MSWordArt.2", &[], &[])]);
        // A picture frame Ami Pro marked linked (flag 0x200000): its three
        // strings are kept among the frame's extra values.
        let frame = document.frame_mut(frames[0]).unwrap();
        let amipro_document::FrameContent::Picture(picture) = &mut frame.content else {
            unreachable!();
        };
        picture.extension = ".bmp".into();
        frame.legacy.push("frame.flags.unknown", "0x200000");
        frame
            .legacy
            .push("frame.extra", "123W\nC:\\SHEET.WK4\nA:G3..A:I16");
        let listed = rows(&document);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, "DDE link");
        assert_eq!(listed[0].source, "123W · C:\\SHEET.WK4 · A:G3..A:I16");
    }
}
