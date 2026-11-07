//! Labels (`docs/research/MERGE_NOTES.md`, section 6; decision MG7): the
//! app's own sheet list from Avery's published sizes, a label document's
//! set-up as Ami Pro's custom rule makes it (the page is one label: top and
//! left margins 0, the right margin the gap across, the bottom margin the
//! gap down; `[recfile]`'s across, down, and first label's indent with As
//! labels), and where each label goes on a sheet as Ami Pro prints them
//! (`1550:0145`: across, then down, then a new sheet).

#[cfg(test)]
use amipro_document::Rect;
use amipro_document::merge::{RecordFile, record_flags};
use amipro_document::{Document, Twips};

use crate::Command;
use crate::frames::LayoutSettings;
use crate::layout_setup::{MarginSide, Pages, Refusal};

/// The paper a sheet is printed on.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SheetPaper {
    Letter,
    A4,
}

impl SheetPaper {
    /// Width and height in twips.
    pub fn size(self) -> (Twips, Twips) {
        match self {
            Self::Letter => (12_240, 15_840),
            Self::A4 => (11_906, 16_838),
        }
    }
}

/// A sheet of labels: one label's size, how many across and down, the
/// first label's place, and the pitches (a label's size plus its gap).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelSheet {
    /// Avery's part numbers for this sheet (the first is shown).
    pub parts: &'static str,
    pub kind: &'static str,
    pub paper: SheetPaper,
    /// The label's width and height, in twips.
    pub width: Twips,
    pub height: Twips,
    pub across: i32,
    pub down: i32,
    /// The first label's distance from the sheet's top and left edges.
    pub top: Twips,
    pub side: Twips,
    /// From one label's left (top) edge to the next one's.
    pub pitch_across: Twips,
    pub pitch_down: Twips,
}

const fn inches(value: f64) -> Twips {
    (value * 1_440.0 + 0.5) as Twips
}

const fn mm(value: f64) -> Twips {
    (value * 1_440.0 / 25.4 + 0.5) as Twips
}

#[allow(clippy::too_many_arguments)]
const fn letter(
    parts: &'static str,
    kind: &'static str,
    height: f64,
    width: f64,
    across: i32,
    down: i32,
    top: f64,
    side: f64,
    pitch_across: f64,
    pitch_down: f64,
) -> LabelSheet {
    LabelSheet {
        parts,
        kind,
        paper: SheetPaper::Letter,
        width: inches(width),
        height: inches(height),
        across,
        down,
        top: inches(top),
        side: inches(side),
        pitch_across: inches(pitch_across),
        pitch_down: inches(pitch_down),
    }
}

#[allow(clippy::too_many_arguments)]
const fn a4(
    parts: &'static str,
    kind: &'static str,
    height: f64,
    width: f64,
    across: i32,
    down: i32,
    top: f64,
    side: f64,
    pitch_across: f64,
    pitch_down: f64,
) -> LabelSheet {
    LabelSheet {
        parts,
        kind,
        paper: SheetPaper::A4,
        width: mm(width),
        height: mm(height),
        across,
        down,
        top: mm(top),
        side: mm(side),
        pitch_across: mm(pitch_across),
        pitch_down: mm(pitch_down),
    }
}

/// The app's sheet list (MG7): Avery's laser and inkjet sheets, letter and
/// A4, by their published sizes (height × width, across × down, top
/// margin, side margin, pitch across, pitch down). Continuous forms for
/// tractor-feed printers are left out (decision MC5).
pub const SHEETS: [LabelSheet; 26] = [
    letter(
        "5160, 5260, 5960, 8160",
        "Address",
        1.0,
        2.625,
        3,
        10,
        0.5,
        0.1875,
        2.75,
        1.0,
    ),
    letter(
        "5161, 5261, 8161",
        "Address",
        1.0,
        4.0,
        2,
        10,
        0.5,
        0.15625,
        4.1875,
        1.0,
    ),
    letter(
        "5162, 5262, 8162",
        "Address",
        4.0 / 3.0,
        4.0,
        2,
        7,
        0.8333,
        0.15625,
        4.1875,
        4.0 / 3.0,
    ),
    letter(
        "5159", "Address", 1.5, 4.0, 2, 7, 0.25, 0.15625, 4.1875, 1.5,
    ),
    letter(
        "5163, 5263, 8163",
        "Shipping",
        2.0,
        4.0,
        2,
        5,
        0.5,
        0.15625,
        4.1875,
        2.0,
    ),
    letter(
        "5164, 8164",
        "Shipping",
        10.0 / 3.0,
        4.0,
        2,
        3,
        0.5,
        0.15625,
        4.1875,
        10.0 / 3.0,
    ),
    letter(
        "5165, 8165",
        "Full sheet",
        11.0,
        8.5,
        1,
        1,
        0.0,
        0.0,
        8.5,
        11.0,
    ),
    letter(
        "5167, 5267, 8167",
        "Return address",
        0.5,
        1.75,
        4,
        20,
        0.5,
        0.28125,
        2.0625,
        0.5,
    ),
    letter(
        "5266, 8366",
        "File folder",
        2.0 / 3.0,
        3.4375,
        2,
        15,
        0.5,
        0.53125,
        4.0,
        2.0 / 3.0,
    ),
    letter(
        "5196, 5096, 5896",
        "Diskette 3.5\"",
        2.75,
        2.75,
        3,
        3,
        0.5,
        0.125,
        2.75,
        3.0,
    ),
    letter(
        "5197, 5097, 5897",
        "Diskette 5.25\"",
        1.5,
        4.0,
        2,
        6,
        1.0,
        0.15625,
        4.1875,
        1.5,
    ),
    letter(
        "5371, 8371",
        "Business card",
        2.0,
        3.5,
        2,
        5,
        0.5,
        0.75,
        3.5,
        2.0,
    ),
    letter(
        "5395, 8395",
        "Name badge",
        7.0 / 3.0,
        3.375,
        2,
        4,
        0.59375,
        0.6875,
        3.75,
        2.5,
    ),
    letter("5388", "Index card", 3.0, 5.0, 1, 3, 1.0, 1.75, 5.0, 3.0),
    letter("5389", "Post card", 4.0, 6.0, 1, 2, 1.5, 1.25, 6.0, 4.0),
    letter("5168", "Shipping", 5.0, 3.5, 2, 2, 0.5, 0.5, 4.0, 5.0),
    a4(
        "L7160, J8160",
        "Address",
        38.1,
        63.5,
        3,
        7,
        15.15,
        7.25,
        66.04,
        38.1,
    ),
    a4(
        "L7159, J8159",
        "Address",
        33.9,
        63.5,
        3,
        8,
        12.9,
        6.5,
        66.04,
        33.9,
    ),
    a4(
        "L7161, J8161",
        "Address",
        46.6,
        63.5,
        3,
        6,
        8.8,
        7.25,
        66.04,
        46.6,
    ),
    a4(
        "L7162, J8162",
        "Address",
        33.9,
        99.1,
        2,
        8,
        12.9,
        4.65,
        101.6,
        33.9,
    ),
    a4(
        "L7163, J8163",
        "Address",
        38.1,
        99.1,
        2,
        7,
        15.15,
        4.65,
        101.6,
        38.1,
    ),
    a4(
        "L7173, J8173",
        "Shipping",
        57.0,
        99.1,
        2,
        5,
        6.0,
        4.65,
        101.6,
        57.0,
    ),
    a4(
        "L7165, J8165",
        "Shipping",
        67.7,
        99.1,
        2,
        4,
        13.1,
        4.65,
        101.6,
        67.7,
    ),
    a4(
        "L7166, J8166",
        "Shipping",
        93.1,
        99.1,
        2,
        3,
        8.8,
        4.65,
        101.6,
        93.1,
    ),
    a4(
        "L7651, J8651",
        "Mini address",
        21.2,
        38.1,
        5,
        13,
        10.7,
        4.75,
        40.6,
        21.2,
    ),
    a4(
        "L7167, J8167",
        "Full sheet",
        289.1,
        199.6,
        1,
        1,
        3.95,
        5.2,
        199.6,
        289.1,
    ),
];

impl LabelSheet {
    /// The name the list shows: the first part, its kind, and its layout.
    pub fn name(&self) -> String {
        let part = self.parts.split(',').next().unwrap_or(self.parts);
        format!(
            "Avery {part} {} ({} × {})",
            self.kind, self.across, self.down
        )
    }

    /// Labels a sheet holds.
    pub fn per_sheet(&self) -> usize {
        (self.across.max(1) * self.down.max(1)) as usize
    }

    /// A sheet of typed values (Custom).
    #[allow(clippy::too_many_arguments)]
    pub fn custom(
        paper: SheetPaper,
        width: Twips,
        height: Twips,
        across: i32,
        down: i32,
        top: Twips,
        side: Twips,
        pitch_across: Twips,
        pitch_down: Twips,
    ) -> Self {
        Self {
            parts: "Custom",
            kind: "Custom",
            paper,
            width,
            height,
            across,
            down,
            top,
            side,
            pitch_across: pitch_across.max(width),
            pitch_down: pitch_down.max(height),
        }
    }

    /// The sheet a label document's page and `[recfile]` describe: one of
    /// the list when they match it, else a custom sheet on letter paper
    /// (A4 when the unit is centimetres, 2).
    pub fn of(document: &Document) -> Option<Self> {
        let file = document.merge().record_file.as_ref()?;
        if !file.as_labels() {
            return None;
        }
        let layout = document.layouts().first()?;
        let page = &layout.right;
        let margins = page.layout.margins;
        let (pitch_across, pitch_down) = (page.width, page.height);
        let (width, height) = (
            pitch_across - margins.left - margins.right,
            pitch_down - margins.top - margins.bottom,
        );
        let near = |a: Twips, b: Twips| (a - b).abs() <= 15;
        let listed = SHEETS.iter().find(|sheet| {
            sheet.across == file.across
                && sheet.down == file.down
                && near(sheet.pitch_across, pitch_across)
                && near(sheet.pitch_down, pitch_down)
                && near(sheet.top, file.indent_down)
                && near(sheet.side, file.indent_right)
        });
        Some(listed.copied().unwrap_or_else(|| {
            Self::custom(
                if file.unit == 2 {
                    SheetPaper::A4
                } else {
                    SheetPaper::Letter
                },
                width.max(1),
                height.max(1),
                file.across.max(1),
                file.down.max(1),
                file.indent_down,
                file.indent_right,
                pitch_across,
                pitch_down,
            )
        }))
    }

    /// Where label `index` of the run goes: its sheet, and its top-left
    /// corner on the sheet in twips (`1550:0145`: column × page width +
    /// indent right, row × page height + indent down).
    pub fn place(&self, index: usize) -> (usize, Twips, Twips) {
        let per = self.per_sheet();
        let sheet = index / per;
        let at = index % per;
        let across = self.across.max(1) as usize;
        let (row, column) = (at / across, at % across);
        (
            sheet,
            self.side + column as Twips * self.pitch_across,
            self.top + row as Twips * self.pitch_down,
        )
    }
}

/// The labels a run prints, in order: each merged record `copies` times,
/// starting at label `start` (1 for the sheet's first); `None` is a label
/// position left empty. Ami Pro's Print each label N times.
pub fn label_run(records: usize, copies: usize, start: usize) -> Vec<Option<usize>> {
    let mut run: Vec<Option<usize>> = vec![None; start.saturating_sub(1)];
    for record in 0..records {
        for _ in 0..copies.max(1) {
            run.push(Some(record));
        }
    }
    run
}

/// The label document's set-up for `sheet`, one undo step: the first
/// page layout becomes one label (Ami Pro's custom rule), and `[recfile]`
/// takes As labels, across, down, and the first label's indent. A document
/// without a data file gets a `[recfile]` naming none.
pub fn set_up_labels(document: &Document, sheet: &LabelSheet) -> Result<Command, Refusal> {
    let layout = document
        .layouts()
        .first()
        .ok_or("the document has no page layout")?;
    let mut settings = LayoutSettings::of(document, layout.id).ok_or("no such layout")?;
    for side in [
        MarginSide::Top,
        MarginSide::Left,
        MarginSide::Right,
        MarginSide::Bottom,
    ] {
        settings.set_margin(Pages::All, side, 0)?;
    }
    settings.set_landscape(false)?;
    settings.set_custom_size(sheet.pitch_across, sheet.pitch_down)?;
    settings.set_margin(
        Pages::All,
        MarginSide::Right,
        sheet.pitch_across - sheet.width,
    )?;
    settings.set_margin(
        Pages::All,
        MarginSide::Bottom,
        sheet.pitch_down - sheet.height,
    )?;
    // Tab stops past the label's text are dropped.
    let stops: Vec<amipro_document::TabStop> = settings
        .right
        .layout
        .tabs
        .first()
        .map(|stops| {
            stops
                .iter()
                .filter(|stop| stop.position < sheet.width)
                .copied()
                .collect()
        })
        .unwrap_or_default();
    settings.set_tabs(Pages::All, stops)?;
    let mut merge = document.merge().clone();
    let mut file = merge.record_file.clone().unwrap_or_default();
    file.flags |= record_flags::LABELS;
    // The Labels dialog's unit: 1 in., 2 cm. (VM twin AMLB: 0 is none).
    file.unit = match sheet.paper {
        SheetPaper::Letter => 1,
        SheetPaper::A4 => 2,
    };
    file.across = sheet.across;
    file.down = sheet.down;
    file.indent_down = sheet.top;
    file.indent_right = sheet.side;
    merge.record_file = Some(file);
    Ok(Command::Several(vec![
        Command::ModifyLayout {
            layout: layout.id,
            settings,
        },
        Command::SetMerge(Box::new(merge)),
    ]))
}

/// Merged labels as sheets (MG7's whole sheet, decision MC8): a document
/// on the sheet's paper, each label a transparent Where placed frame at its
/// place (as [`LabelSheet::place`]) holding its record's text.
pub fn label_sheets(sheet: &LabelSheet, labels: &[Option<&Document>]) -> Document {
    let mut document = Document::from_plain_text("");
    document.title = "Labels".into();
    let (width, height) = sheet.paper.size();
    let layout = document.layouts()[0].id;
    if let Some(mut settings) = LayoutSettings::of(&document, layout) {
        let set = (|| {
            for side in [
                MarginSide::Top,
                MarginSide::Left,
                MarginSide::Right,
                MarginSide::Bottom,
            ] {
                settings.set_margin(Pages::All, side, 0)?;
            }
            settings.set_custom_size(width, height)
        })();
        if set.is_ok()
            && let Ok(applied) =
                crate::apply(&document, &Command::ModifyLayout { layout, settings })
        {
            document = applied.document;
        }
    }
    let style = amipro_document::FrameStyle {
        wrap: amipro_document::Wrap::NoWrapAround,
        opaque: false,
        border: amipro_document::Border {
            sides: amipro_document::Sides::NONE,
            ..amipro_document::FrameStyle::default().border
        },
        shadow: amipro_document::Shadow::default(),
        corners: 0,
        interior: amipro_document::LayoutBox::single_column(amipro_document::Margins::uniform(0)),
        ..amipro_document::FrameStyle::default()
    };
    for (index, label) in labels.iter().enumerate() {
        let Some(label) = label else { continue };
        let (page, left, top) = sheet.place(index);
        let text: Vec<String> = label
            .main_flow()
            .paragraphs
            .iter()
            .map(|paragraph| {
                paragraph
                    .text
                    .chars()
                    .filter(|character| *character != amipro_document::OBJECT_REPLACEMENT_CHARACTER)
                    .collect()
            })
            .collect();
        let created = crate::apply(
            &document,
            &Command::CreateFrame {
                placement: amipro_document::Placement::Fixed {
                    page: page as u32 + 1,
                    rect: amipro_document::Rect::new(
                        left,
                        top,
                        left + sheet.width,
                        top + sheet.height,
                    ),
                },
                style: style.clone(),
                content: crate::NewFrameContent::Text,
                anchor: None,
                layout: None,
            },
        );
        let Ok(applied) = created else { continue };
        let frame = applied.report.frames.iter().next().copied();
        document = applied.document;
        if let Some(amipro_document::FrameContent::Text(flow)) = frame
            .and_then(|frame| document.frame(frame))
            .map(|frame| frame.content.clone())
        {
            let at = crate::TextPosition {
                flow,
                paragraph: 0,
                byte: 0,
            };
            if let Ok(applied) = crate::apply(
                &document,
                &Command::ReplaceText {
                    start: at,
                    end: at,
                    text: text.join("\n"),
                },
            ) {
                document = applied.document;
            }
        }
    }
    document
}

/// Merge as labels off: `[recfile]` loses As labels (the page stays as it
/// is, as in Ami Pro, where the label values stay as written).
pub fn labels_off(document: &Document) -> Option<Command> {
    let mut merge = document.merge().clone();
    let file: &mut RecordFile = merge.record_file.as_mut()?;
    if !file.as_labels() {
        return None;
    }
    file.flags &= !record_flags::LABELS;
    Some(Command::SetMerge(Box::new(merge)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply;

    #[test]
    fn every_sheet_fits_its_paper() {
        for sheet in SHEETS {
            let (width, height) = sheet.paper.size();
            let right = sheet.side + (sheet.across - 1) as Twips * sheet.pitch_across + sheet.width;
            let bottom = sheet.top + (sheet.down - 1) as Twips * sheet.pitch_down + sheet.height;
            assert!(right <= width + 30, "{} is too wide", sheet.parts);
            assert!(bottom <= height + 30, "{} is too tall", sheet.parts);
            assert!(sheet.pitch_across >= sheet.width && sheet.pitch_down >= sheet.height);
        }
    }

    #[test]
    fn merged_labels_as_sheets_put_each_label_at_its_place() {
        let one = Document::from_plain_text("Terry Allen\nBoston");
        let two = Document::from_plain_text("Pam Beck\nSeattle");
        let sheet = SHEETS[0];
        let mut labels: Vec<Option<&Document>> = vec![None; 29];
        labels.extend([Some(&one), Some(&two)]);
        let document = label_sheets(&sheet, &labels);
        let page = &document.layouts()[0].right;
        assert_eq!((page.width, page.height), (12_240, 15_840));
        let placed: Vec<(u32, Rect)> = document
            .frames()
            .filter_map(|frame| match frame.placement {
                amipro_document::Placement::Fixed { page, rect } => Some((page, rect)),
                _ => None,
            })
            .collect();
        assert_eq!(placed.len(), 2);
        // Label 30 is the first sheet's last; label 31 the next sheet's first.
        assert_eq!(
            placed[0],
            (
                1,
                Rect::new(
                    270 + 2 * 3_960,
                    720 + 9 * 1_440,
                    270 + 2 * 3_960 + 3_780,
                    720 + 10 * 1_440
                )
            )
        );
        assert_eq!(placed[1].0, 2);
    }

    #[test]
    fn labels_go_across_then_down_then_to_a_new_sheet() {
        let sheet = SHEETS[0];
        assert_eq!(sheet.place(0), (0, 270, 720));
        assert_eq!(sheet.place(1), (0, 270 + 3_960, 720));
        assert_eq!(sheet.place(3), (0, 270, 720 + 1_440));
        assert_eq!(sheet.place(30), (1, 270, 720));
        assert_eq!(
            label_run(2, 2, 3),
            vec![None, None, Some(0), Some(0), Some(1), Some(1)]
        );
    }

    #[test]
    fn a_label_document_is_one_label_with_ami_pros_custom_rule() {
        let document = Document::from_plain_text("<NAME>");
        let sheet = SHEETS[0];
        let command = set_up_labels(&document, &sheet).unwrap();
        let labelled = apply(&document, &command).unwrap().document;
        let page = &labelled.layouts()[0].right;
        assert_eq!((page.width, page.height), (3_960, 1_440));
        let margins = page.layout.margins;
        assert_eq!(
            (margins.top, margins.left, margins.right, margins.bottom),
            (0, 0, 180, 0)
        );
        assert!(
            page.layout
                .tabs
                .iter()
                .flatten()
                .all(|stop| stop.position < 3_780)
        );
        let file = labelled.merge().record_file.as_ref().unwrap();
        assert!(file.as_labels());
        assert_eq!(
            (file.across, file.down, file.indent_down, file.indent_right),
            (3, 10, 720, 270)
        );
        assert_eq!(LabelSheet::of(&labelled), Some(sheet));
        let off = apply(&labelled, &labels_off(&labelled).unwrap())
            .unwrap()
            .document;
        assert!(LabelSheet::of(&off).is_none());
    }
}
