//! Edit › Paste Special… in the rail (`docs/Design/UI_SPEC.md` §42.2, OD10;
//! run D, DO.4). Ami Pro's dialog (2011, `PASTESPECIALPROC`) lists the
//! clipboard's own formats in the source's order, each kept when an
//! acceptance test allows it, and pastes the chosen one; 3.1 has no
//! ranking, no "As:" text, and no display as icon. This panel does the same
//! with the formats the desktop clipboard offers: text, rich text, HTML,
//! the image types, and copied files. The toolkit cannot list what the
//! clipboard holds, so each type is looked for in turn and the rows keep
//! this order. Paste Link and New Object are not offered (no servers, DDE
//! not planned). A chosen format the app cannot hold shows Ami Pro's
//! message 3092.

use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::picture_insert;

/// The panel's width in the rail.
pub const WIDTH: f32 = 300.0;

/// Ami Pro's message 3092.
pub const CANNOT_ACCEPT: &str = "Cannot accept an object that Ami Pro can not display";
/// What the panel says when the clipboard offers nothing.
pub const EMPTY: &str = "The clipboard holds nothing this program can paste.";
/// What the panel says about the rows.
pub const NOTE: &str =
    "Choose a format, then Paste (or double-click it). Paste Link and New Object are not offered.";

/// What a format is pasted as.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Text,
    RichText,
    Html,
    Image,
    Files,
}

/// The formats looked for, in the order the rows keep: the name Ami Pro
/// would show, what it pastes as, and the clipboard types that carry it.
/// Slot 0 is the clipboard's text, read as text.
pub const CANDIDATES: [(&str, Kind, &[&str]); 10] = [
    ("Text", Kind::Text, &[]),
    (
        "Rich Text Format",
        Kind::RichText,
        &["text/rtf", "application/rtf", "text/richtext"],
    ),
    ("HTML", Kind::Html, &["text/html"]),
    ("PNG image", Kind::Image, &["image/png"]),
    ("JPEG image", Kind::Image, &["image/jpeg"]),
    ("GIF image", Kind::Image, &["image/gif"]),
    ("Bitmap", Kind::Image, &["image/bmp"]),
    ("TIFF image", Kind::Image, &["image/tiff"]),
    ("SVG image", Kind::Image, &["image/svg+xml"]),
    ("Files", Kind::Files, &["text/uri-list"]),
];

/// A clipboard type of slot `K`, read by the toolkit as bytes.
#[derive(Clone, Debug)]
pub struct Offered<const K: usize>(pub Vec<u8>, pub String);

impl<const K: usize> TryFrom<(Vec<u8>, String)> for Offered<K> {
    type Error = ();

    fn try_from((bytes, mime): (Vec<u8>, String)) -> Result<Self, ()> {
        (!bytes.is_empty() && CANDIDATES[K].2.contains(&mime.as_str()))
            .then_some(Self(bytes, mime))
            .ok_or(())
    }
}

impl<const K: usize> cosmic::iced::clipboard::mime::AllowedMimeTypes for Offered<K> {
    fn allowed() -> std::borrow::Cow<'static, [String]> {
        CANDIDATES[K]
            .2
            .iter()
            .map(|&mime| mime.to_owned())
            .collect()
    }
}

/// One format the clipboard offers.
#[derive(Clone, Debug)]
pub struct Format {
    pub slot: usize,
    pub bytes: Vec<u8>,
    pub mime: String,
}

impl Format {
    pub fn name(&self) -> &'static str {
        CANDIDATES[self.slot].0
    }

    pub fn kind(&self) -> Kind {
        CANDIDATES[self.slot].1
    }

    /// What the row says it pastes as.
    pub fn pastes_as(&self) -> String {
        match self.kind() {
            Kind::Text => "as text".into(),
            Kind::RichText => "as formatted text".into(),
            Kind::Html => "as text".into(),
            Kind::Image => "as a picture".into(),
            Kind::Files => {
                let files = picture_insert::dropped_pictures(&self.bytes).len();
                match files {
                    0 => "not pictures".into(),
                    1 => "as a picture".into(),
                    _ => "as a picture (the first)".into(),
                }
            }
        }
    }

    /// The file extension an image row's picture has.
    pub fn extension(&self) -> &'static str {
        picture_insert::ClipboardImage {
            bytes: Vec::new(),
            mime: self.mime.clone(),
        }
        .extension()
    }
}

/// The panel's state: what was found, the row chosen, and the looks not
/// yet answered.
#[derive(Clone, Debug, Default)]
pub struct PasteSpecial {
    pub formats: Vec<Format>,
    pub selected: Option<usize>,
    /// Reads sent and not answered.
    pub pending: usize,
    /// Whether the empty first look has been tried again (the clipboard
    /// can find nothing as the window gains focus).
    pub retried: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PasteSpecialAction {
    Select(usize),
    /// Paste the row (a double-click) or the chosen one (the button).
    Paste(Option<usize>),
    Close,
}

impl PasteSpecial {
    /// A format found: kept in the order of [`CANDIDATES`].
    pub fn found(&mut self, format: Format) {
        self.pending = self.pending.saturating_sub(1);
        let at = self
            .formats
            .iter()
            .position(|kept| kept.slot > format.slot)
            .unwrap_or(self.formats.len());
        self.formats.insert(at, format);
        // The first row is chosen until the user chooses.
        if self.selected.is_none() {
            self.selected = Some(0);
        } else if let Some(selected) = self.selected
            && at <= selected
            && self.formats.len() > 1
        {
            self.selected = Some(selected + 1);
        }
    }

    /// A look that found nothing.
    pub fn missed(&mut self) {
        self.pending = self.pending.saturating_sub(1);
    }

    /// Whether every look is answered and none found anything.
    pub fn empty(&self) -> bool {
        self.pending == 0 && self.formats.is_empty()
    }
}

/// The text of HTML markup: tags and the contents of script and style
/// elements dropped, the common entities read, line breaks at block
/// ends. Paste Special pastes HTML as text.
pub fn html_text(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        out.push_str(&entities(&rest[..open]));
        rest = &rest[open..];
        let Some(close) = rest.find('>') else {
            rest = "";
            break;
        };
        let tag = rest[1..close].trim().to_ascii_lowercase();
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        rest = &rest[close + 1..];
        match name.as_str() {
            "script" | "style" if !tag.starts_with('/') => {
                let end = format!("</{name}");
                match rest.to_ascii_lowercase().find(&end) {
                    Some(at) => {
                        rest = &rest[at..];
                        if let Some(close) = rest.find('>') {
                            rest = &rest[close + 1..];
                        } else {
                            rest = "";
                        }
                    }
                    None => rest = "",
                }
            }
            "br" | "p" | "div" | "li" | "tr" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                if !out.ends_with('\n') =>
            {
                out.push('\n');
            }
            _ => {}
        }
    }
    out.push_str(&entities(rest));
    out.trim_matches('\n').to_owned()
}

fn entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let Some(end) = rest.find(';').filter(|end| *end <= 8) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[1..end];
        let replaced = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => name
                .strip_prefix('#')
                .and_then(|number| match number.strip_prefix(['x', 'X']) {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => number.parse().ok(),
                })
                .and_then(char::from_u32),
        };
        match replaced {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn send(action: PasteSpecialAction) -> Message {
    Message::PasteSpecial(action)
}

/// The panel.
pub fn panel<'a>(state: &PasteSpecial) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Paste Special").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Paste Special")
            .on_press(send(PasteSpecialAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut children: Vec<Element<'a, Message>> = vec![header.into()];
    children.push(widget::text::caption("Formats on the clipboard").into());
    if state.empty() {
        children.push(widget::text::body(EMPTY).into());
    } else if state.formats.is_empty() {
        children.push(widget::text::body("Looking at the clipboard…").into());
    }
    for (index, format) in state.formats.iter().enumerate() {
        let selected = state.selected == Some(index);
        let content = widget::column::with_children(vec![
            widget::text::body(format.name()).into(),
            widget::text::caption(format.pastes_as()).into(),
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
                .on_press(send(PasteSpecialAction::Select(index)))
                .on_double_click(send(PasteSpecialAction::Paste(Some(index))))
                .into(),
        );
    }
    children.push(
        widget::button::standard("Paste")
            .on_press_maybe(
                state
                    .selected
                    .map(|_| send(PasteSpecialAction::Paste(None))),
            )
            .into(),
    );
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

    fn format(slot: usize) -> Format {
        Format {
            slot,
            bytes: b"x".to_vec(),
            mime: CANDIDATES[slot].2.first().copied().unwrap_or("").into(),
        }
    }

    #[test]
    fn rows_keep_the_candidates_order_whatever_the_order_of_the_answers() {
        let mut state = PasteSpecial {
            pending: 3,
            ..PasteSpecial::default()
        };
        state.found(format(3));
        assert_eq!(state.selected, Some(0));
        state.found(format(0));
        state.found(format(1));
        let slots: Vec<usize> = state.formats.iter().map(|f| f.slot).collect();
        assert_eq!(slots, [0, 1, 3]);
        // The row chosen before stays the one chosen: the picture.
        assert_eq!(state.formats[state.selected.unwrap()].slot, 3);
        assert_eq!(state.pending, 0);
        assert!(!state.empty());
    }

    #[test]
    fn html_is_pasted_as_its_text() {
        let html = "<html><head><style>p{x:y}</style></head><body><h1>Title</h1>\
            <p>A &amp; B &lt;c&gt; &#233;&nbsp;d</p><script>var x=1<2;</script>\
            <div>Two<br>lines</div></body></html>";
        assert_eq!(html_text(html), "Title\nA & B <c> é d\nTwo\nlines");
        assert_eq!(html_text("plain & simple"), "plain & simple");
    }
}
