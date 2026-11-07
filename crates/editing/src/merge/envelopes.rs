//! Envelopes (`docs/research/MERGE_NOTES.md`, section 8; decision MG8):
//! Ami Pro's sizes from `AMIENV.SMM`'s list, the recipient found in a
//! document (the selection, else the first address on page 1), and the
//! envelope as a document: its page the envelope's size, landscape, the
//! return address at its top left, the recipient in a Where placed frame
//! that the offsets move.

use amipro_document::{
    Border, Document, FrameStyle, LayoutBox, LinePosition, Margins, Placement, Rect, Shadow, Sides,
    Twips, Wrap,
};

use crate::frames::LayoutSettings;
use crate::layout_setup::{MarginSide, Pages};
use crate::{Command, NewFrameContent, TextPosition, apply};

/// An envelope size, its long side first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeSize {
    pub name: &'static str,
    /// Width (the long side) and height, in twips.
    pub width: Twips,
    pub height: Twips,
}

const fn inches(name: &'static str, width: f64, height: f64) -> EnvelopeSize {
    EnvelopeSize {
        name,
        width: (width * 1_440.0 + 0.5) as Twips,
        height: (height * 1_440.0 + 0.5) as Twips,
    }
}

const fn mm(name: &'static str, width: f64, height: f64) -> EnvelopeSize {
    EnvelopeSize {
        name,
        width: (width * 1_440.0 / 25.4 + 0.5) as Twips,
        height: (height * 1_440.0 / 25.4 + 0.5) as Twips,
    }
}

/// Ami Pro's sizes (`AMIENV.SMM`'s list, read in the VM; the progress
/// file's table). Size 10 is the default.
pub const SIZES: [EnvelopeSize; 11] = [
    inches("Monarch (7½ × 3⅞ in)", 7.5, 3.875),
    inches("Size 6¾ (6½ × 3⅝ in)", 6.5, 3.625),
    inches("Size 9 (8⅞ × 3⅞ in)", 8.875, 3.875),
    inches("Size 10 (9½ × 4⅛ in)", 9.5, 4.125),
    inches("Size 11 (10⅜ × 4½ in)", 10.375, 4.5),
    inches("Size 12 (11 × 4¾ in)", 11.0, 4.75),
    mm("DL (220 × 110 mm)", 220.0, 110.0),
    mm("C5 (229 × 162 mm)", 229.0, 162.0),
    mm("C6 (162 × 114 mm)", 162.0, 114.0),
    mm("C65 (230 × 110 mm)", 230.0, 110.0),
    mm("Italian (230 × 110 mm)", 230.0, 110.0),
];

/// Size 10's place in [`SIZES`].
pub const DEFAULT_SIZE: usize = 3;

/// Ami Pro's custom sizes: 3 to 11 inches each way.
pub const CUSTOM_RANGE: (Twips, Twips) = (3 * 1_440, 11 * 1_440);

/// The most lines a return address holds (`AMIENV.SMM`).
pub const RETURN_LINES: usize = 5;

/// What the envelope holds and where.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Envelope {
    pub width: Twips,
    pub height: Twips,
    pub recipient: Vec<String>,
    /// Empty when the return address is not printed.
    pub return_address: Vec<String>,
    /// Ami Pro's Options offsets: each address moved right and down.
    pub recipient_offset: (Twips, Twips),
    pub return_offset: (Twips, Twips),
}

/// The return address's distance from the envelope's top and left edges.
const RETURN_MARGIN: Twips = 360;

impl Envelope {
    /// Where the recipient's frame goes: a little left of the middle across
    /// and just below it down, as US and ISO practice place it, moved by
    /// the offset.
    pub fn recipient_rect(&self) -> Rect {
        let left = self.width * 2 / 5 + self.recipient_offset.0;
        let top = self.height * 9 / 20 + self.recipient_offset.1;
        let right = (self.width - RETURN_MARGIN).max(left + 1_440);
        let bottom = (self.height - RETURN_MARGIN / 2).max(top + 720);
        Rect::new(left, top, right, bottom)
    }
}

/// A paragraph that can be an address line: some text, not too long.
fn address_line(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && text.chars().count() <= 60 && !text.ends_with(':')
}

/// The first address in `paragraphs` (Ami Pro's "the first address on page
/// 1"): the first run of two to six short lines whose last line holds a
/// digit (a postcode or a street number), as a letter's inside address.
pub fn first_address(paragraphs: &[String]) -> Option<Vec<String>> {
    let mut start = 0;
    while start < paragraphs.len() {
        if !address_line(&paragraphs[start]) {
            start += 1;
            continue;
        }
        let mut end = start;
        while end < paragraphs.len() && address_line(&paragraphs[end]) && end - start < 6 {
            end += 1;
        }
        let run = &paragraphs[start..end];
        if run.len() >= 2 {
            // The address ends at its last line with a digit.
            if let Some(last) = run
                .iter()
                .rposition(|line| line.chars().any(|character| character.is_ascii_digit()))
                && last >= 1
            {
                return Some(
                    run[..=last]
                        .iter()
                        .map(|line| line.trim().to_string())
                        .collect(),
                );
            }
        }
        start = end.max(start + 1);
    }
    None
}

/// The document's main text as lines, objects left out, for
/// [`first_address`]: its first page's paragraphs (Ami Pro looks on page 1;
/// the first 60 paragraphs stand for it).
pub fn main_lines(document: &Document) -> Vec<String> {
    document
        .main_flow()
        .paragraphs
        .iter()
        .take(60)
        .flat_map(|paragraph| {
            paragraph
                .text
                .chars()
                .filter(|character| *character != amipro_document::OBJECT_REPLACEMENT_CHARACTER)
                .collect::<String>()
                .split('\u{2028}')
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Merged envelopes (MG8): each `<NAME>` in `lines` (a merge field as the
/// document shows it outside a merge) given record `record`'s value; a
/// line that holds only empty fields is left out (MG9), and a doubled
/// space closes up.
pub fn fill(lines: &[String], table: &super::data::DataTable, record: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in lines {
        let mut filled = String::new();
        let mut had_field = false;
        let mut rest = line.as_str();
        while let Some(open) = rest.find('<') {
            let Some(close) = rest[open..].find('>') else {
                break;
            };
            let name = &rest[open + 1..open + close];
            filled.push_str(&rest[..open]);
            match table.value(record, name) {
                Some(value) => {
                    had_field = true;
                    filled.push_str(value);
                }
                None => filled.push_str(&rest[open..=open + close]),
            }
            rest = &rest[open + close + 1..];
        }
        filled.push_str(rest);
        while filled.contains("  ") {
            filled = filled.replace("  ", " ");
        }
        let filled = filled.trim().trim_start_matches(',').trim().to_string();
        if had_field && filled.is_empty() {
            continue;
        }
        out.push(filled);
    }
    out
}

/// The envelope as a document: its page the envelope's size, landscape,
/// margins of a quarter inch, the return address in the main text, the
/// recipient in a transparent Where placed frame without lines.
pub fn envelope_document(envelope: &Envelope) -> Document {
    envelope_on(Document::from_plain_text(""), envelope)
}

/// The envelope on `base` (a new document from Ami Pro's `_AMIENV.STY`
/// when it is found): its styles kept, the recipient in its `Recipient`
/// style when it has one.
pub fn envelope_on(base: Document, envelope: &Envelope) -> Document {
    let mut document = base;
    let main = document.main_flow_id();
    let last = document.main_flow().paragraphs.len() - 1;
    let end = TextPosition {
        flow: main,
        paragraph: last,
        byte: document.main_flow().paragraphs[last].text.len(),
    };
    let start = TextPosition {
        flow: main,
        paragraph: 0,
        byte: 0,
    };
    if let Ok(applied) = apply(
        &document,
        &Command::ReplaceText {
            start,
            end,
            text: envelope.return_address.join("\n"),
        },
    ) {
        document = applied.document;
    }
    document.title = "Envelope".into();
    let layout = document.layouts()[0].id;
    if let Some(mut settings) = LayoutSettings::of(&document, layout) {
        let changed = (|| {
            for side in [
                MarginSide::Top,
                MarginSide::Left,
                MarginSide::Right,
                MarginSide::Bottom,
            ] {
                settings.set_margin(Pages::All, side, 0)?;
            }
            settings.set_custom_size(envelope.width, envelope.height)?;
            settings.landscape = envelope.width > envelope.height;
            let (right, down) = envelope.return_offset;
            settings.set_margin(Pages::All, MarginSide::Left, (RETURN_MARGIN + right).max(0))?;
            settings.set_margin(Pages::All, MarginSide::Top, (RETURN_MARGIN + down).max(0))?;
            settings.set_margin(Pages::All, MarginSide::Right, RETURN_MARGIN)?;
            settings.set_margin(Pages::All, MarginSide::Bottom, RETURN_MARGIN / 2)?;
            Ok::<_, crate::layout_setup::Refusal>(())
        })();
        if changed.is_ok()
            && let Ok(applied) = apply(&document, &Command::ModifyLayout { layout, settings })
        {
            document = applied.document;
        }
    }
    let style = FrameStyle {
        wrap: Wrap::NoWrapAround,
        opaque: false,
        border: Border {
            sides: Sides::NONE,
            style: 1,
            colour: 0,
            position: LinePosition::Middle,
        },
        shadow: Shadow::default(),
        corners: 0,
        interior: LayoutBox::single_column(Margins::uniform(0)),
        name: Some("Recipient".into()),
        ..FrameStyle::default()
    };
    let created = apply(
        &document,
        &Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: envelope.recipient_rect(),
            },
            style,
            content: NewFrameContent::Text,
            anchor: None,
            layout: None,
        },
    );
    if let Ok(applied) = created {
        let frame = applied.report.frames.iter().next().copied();
        document = applied.document;
        if let Some(amipro_document::FrameContent::Text(flow)) = frame
            .and_then(|frame| document.frame(frame))
            .map(|frame| frame.content.clone())
        {
            let at = TextPosition {
                flow,
                paragraph: 0,
                byte: 0,
            };
            if let Ok(applied) = apply(
                &document,
                &Command::ReplaceText {
                    start: at,
                    end: at,
                    text: envelope.recipient.join("\n"),
                },
            ) {
                document = applied.document;
            }
            let recipient = document
                .styles()
                .iter()
                .find(|(_, style)| style.name.eq_ignore_ascii_case("Recipient"))
                .map(|(id, _)| *id);
            let count = document.flow(flow).map_or(0, |flow| flow.paragraphs.len());
            if let Some(style) = recipient
                && count > 0
                && let Ok(applied) = apply(
                    &document,
                    &Command::SetParagraphStyle {
                        flow,
                        first: 0,
                        last: count - 1,
                        style,
                    },
                )
            {
                document = applied.document;
            }
        }
    }
    document
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &[&str]) -> Vec<String> {
        text.iter().map(|line| line.to_string()).collect()
    }

    #[test]
    fn the_first_address_is_a_run_of_short_lines_ending_with_a_number() {
        let letter = lines(&[
            "November 3, 2026",
            "",
            "Ms. Pam Beck",
            "Vega Corp",
            "41 West St.",
            "Seattle, WA 98101",
            "",
            "Dear Ms. Beck:",
            "Thank you for your letter of the 12th, which reached us today and which we read with care.",
        ]);
        assert_eq!(
            first_address(&letter).unwrap(),
            [
                "Ms. Pam Beck",
                "Vega Corp",
                "41 West St.",
                "Seattle, WA 98101"
            ]
        );
        assert!(first_address(&lines(&["Dear Sir:", "", "No address here."])).is_none());
    }

    #[test]
    fn merged_envelopes_take_each_records_values() {
        let table = super::super::data::DataTable {
            names: lines(&["First", "Last", "Company", "City"]),
            records: vec![
                lines(&["Terry", "Allen", "Vega Corp", "Boston"]),
                lines(&["Pam", "Beck", "", "Seattle"]),
            ],
        };
        let recipient = lines(&["<FIRST> <LAST>", "<COMPANY>", "<CITY>, <STATE>"]);
        assert_eq!(
            fill(&recipient, &table, 0),
            ["Terry Allen", "Vega Corp", "Boston, <STATE>"]
        );
        assert_eq!(
            fill(&recipient, &table, 1),
            ["Pam Beck", "Seattle, <STATE>"]
        );
    }

    #[test]
    fn the_envelope_is_its_size_landscape_with_both_addresses() {
        let envelope = Envelope {
            width: SIZES[DEFAULT_SIZE].width,
            height: SIZES[DEFAULT_SIZE].height,
            recipient: lines(&["Ms. Pam Beck", "41 West St.", "Seattle, WA 98101"]),
            return_address: lines(&["Terry Allen", "1 Main St.", "Boston, MA 02110"]),
            recipient_offset: (144, 0),
            return_offset: (0, 0),
        };
        let document = envelope_document(&envelope);
        let layout = &document.layouts()[0];
        assert_eq!((layout.right.width, layout.right.height), (13_680, 5_940));
        assert!(layout.landscape);
        assert_eq!(layout.right.layout.margins.left, RETURN_MARGIN);
        assert_eq!(document.main_flow().paragraphs[0].text, "Terry Allen");
        let frame = document
            .frames()
            .find(|frame| frame.style.name.as_deref() == Some("Recipient"))
            .unwrap();
        let Placement::Fixed { rect, .. } = frame.placement else {
            panic!("Where placed");
        };
        assert_eq!(rect.left, 13_680 * 2 / 5 + 144);
        let amipro_document::FrameContent::Text(flow) = frame.content else {
            panic!("text");
        };
        let texts: Vec<&str> = document
            .flow(flow)
            .unwrap()
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.text.as_str())
            .collect();
        assert_eq!(texts, ["Ms. Pam Beck", "41 West St.", "Seattle, WA 98101"]);
    }
}
