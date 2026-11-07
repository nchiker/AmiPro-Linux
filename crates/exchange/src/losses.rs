//! What a format cannot hold (EX7): the builder counts what the document
//! uses that some format lacks ([`Feature`]), and each format names what of
//! it is lost, before anything is written (`UI_SPEC.md` §30).

use std::collections::BTreeMap;

/// The formats the export writes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Format {
    Docx,
    Odt,
    Rtf,
    Text,
    /// CommonMark with GitHub's tables, strikethrough, and footnotes (MD2).
    Markdown,
}

impl Format {
    /// The formats the Export menu lists (MD1 added Markdown).
    pub const ALL: [Self; 5] = [Self::Docx, Self::Odt, Self::Rtf, Self::Text, Self::Markdown];

    /// The format's name as the menus give it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Docx => "Word Document",
            Self::Odt => "OpenDocument Text",
            Self::Rtf => "Rich Text",
            Self::Text => "Plain Text",
            Self::Markdown => "Markdown",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Self::Docx => "docx",
            Self::Odt => "odt",
            Self::Rtf => "rtf",
            Self::Text => "txt",
            Self::Markdown => "md",
        }
    }

    pub fn media_type(self) -> &'static str {
        match self {
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Odt => "application/vnd.oasis.opendocument.text",
            Self::Rtf => "application/rtf",
            Self::Text => "text/plain",
            Self::Markdown => "text/markdown",
        }
    }
}

/// Something the document uses that a format may not hold.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Feature {
    /// Protected text (Edit › Mark Text): written as ordinary text.
    ProtectedText,
    /// A style's 1st line bold: the first line's break depends on the layout,
    /// so no format holds it; the text is written in the style's own weight.
    FirstLineBold,
    /// Overstruck characters (Text › Special Effects).
    Overstrike,
    /// No hyphenation over part of a paragraph (it is kept per paragraph).
    PartlyUnhyphenated,
    /// Ami Pro's notes, kept in the app's files until the notes run.
    Notes,
    Footnotes,
    Bookmarks,
    /// Index and TOC entries and other marks that show nothing.
    Marks,
    /// Power fields the formats have no field for: their results are
    /// written as text.
    PowerFields,
    /// Frames with lines on some sides only: drawn all around.
    PartialFrameLines,
    /// Frame groups: their frames are written one by one.
    Groups,
    /// Left pages laid out unlike right pages beyond mirroring.
    LeftPageLayouts,
    /// Pages whose text is centred vertically (a page break's option).
    CentredPages,
    /// Text Tightness other than normal.
    Tightness,
    /// Paragraph rules as long as the text or another length: drawn across
    /// the paragraph.
    ShortRules,
    /// Column balance.
    ColumnBalance,
    /// A page number that shows only from a later page.
    LatePageNumbers,
    /// Pictures the app could not convert: their frames are written empty.
    MissingPictures,
    /// Drawings, charts, and equations: written as Ami Pro's picture of
    /// them.
    DrawingsAsPictures,
    /// Frame content the app does not read: written empty.
    UnknownFrames,
    /// Every frame's text and table, which plain text writes where the
    /// frame is anchored.
    Frames,
    /// Pictures, which plain text cannot hold.
    Pictures,
    /// Tables, written as text with tabs.
    Tables,
    /// Frames' text in more than one column.
    FrameColumns,
    /// Frames with rounded corners (ODF frames have square ones).
    RoundedCorners,
    /// A master document's files defining a style of one name otherwise:
    /// the set is written with the first file's (MQ4).
    DifferingStyles,
}

/// What a document uses, counted.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Features(pub BTreeMap<Feature, usize>);

impl Features {
    pub fn add(&mut self, feature: Feature) {
        *self.0.entry(feature).or_default() += 1;
    }

    pub fn count(&self, feature: Feature) -> usize {
        self.0.get(&feature).copied().unwrap_or(0)
    }
}

/// One thing a format cannot hold, and how often the document uses it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExportLoss {
    pub feature: Feature,
    pub count: usize,
}

impl ExportLoss {
    /// A line of the loss list ("protected text, written as ordinary text
    /// (3 places)").
    pub fn message(&self) -> String {
        let what = match self.feature {
            Feature::ProtectedText => "protected text, written as ordinary text",
            Feature::FirstLineBold => "1st line bold styles, written without the bold first line",
            Feature::Overstrike => "overstruck characters, written without the overstrike",
            Feature::PartlyUnhyphenated => "no hyphenation over part of a paragraph",
            Feature::Notes => "notes",
            Feature::Footnotes => "footnotes",
            Feature::Bookmarks => "bookmarks",
            Feature::Marks => "index and contents entries",
            Feature::PowerFields => "power fields, written as their results",
            Feature::PartialFrameLines => "frame lines on some sides only, drawn all around",
            Feature::Groups => "frame groups, written as separate frames",
            Feature::LeftPageLayouts => "left pages laid out apart from right pages",
            Feature::CentredPages => "text centred vertically on a page",
            Feature::Tightness => "text tightness",
            Feature::ShortRules => "paragraph rules shorter than the paragraph, drawn across it",
            Feature::ColumnBalance => "column balance",
            Feature::LatePageNumbers => "page numbers that start on a later page",
            Feature::MissingPictures => "pictures that could not be converted, written empty",
            Feature::DrawingsAsPictures => "drawings and charts, written as pictures",
            Feature::UnknownFrames => "frames with content the app does not read, written empty",
            Feature::Frames => "frames, their text written where they are anchored",
            Feature::Pictures => "pictures",
            Feature::Tables => "tables, written as text separated by tabs",
            Feature::FrameColumns => "columns in frames",
            Feature::RoundedCorners => "rounded frame corners, drawn square",
            Feature::DifferingStyles => {
                "styles defined otherwise in a later file, written as the first file's"
            }
        };
        if self.count == 1 {
            format!("{what} (1 place)")
        } else {
            format!("{what} ({} places)", self.count)
        }
    }
}

/// What `format` never holds of any document, whatever it uses: a line each
/// for the Save As and Export loss lists, after [`losses`]' own. The other
/// formats hold all of these.
pub fn always_lost(format: Format) -> &'static [&'static str] {
    match format {
        Format::Markdown => &[
            "page layout: paper, margins, columns, headers and footers, page numbers",
            "fonts, sizes, colours, underlining, and highlighting (only bold, italic, strikethrough, and code are kept)",
            "tab stops, indents, alignment, spacing, and paragraph rules",
            "styles other than headings, lists, quotes, and code, which keep their names no longer",
            "table cells that span rows or columns, hold several paragraphs, or have borders and shading",
        ],
        Format::Docx | Format::Odt | Format::Rtf | Format::Text => &[],
    }
}

/// What `format` cannot hold of what the document uses, in a fixed order.
pub fn losses(features: &Features, format: Format) -> Vec<ExportLoss> {
    features
        .0
        .iter()
        .filter(|(feature, count)| **count > 0 && lost(**feature, format))
        .map(|(feature, count)| ExportLoss {
            feature: *feature,
            count: *count,
        })
        .collect()
}

/// Whether `format` loses `feature`.
fn lost(feature: Feature, format: Format) -> bool {
    use Feature::*;
    match format {
        Format::Text => !matches!(feature, PowerFields | DifferingStyles),
        // Tables, footnotes, and pictures (written beside the file) have
        // their Markdown forms; frames' text is written where they anchor.
        Format::Markdown => !matches!(
            feature,
            PowerFields | DifferingStyles | Pictures | Tables | Footnotes
        ),
        Format::Docx => !matches!(
            feature,
            Frames | Pictures | Tables | RoundedCorners | Notes | Footnotes | Bookmarks
        ),
        Format::Odt => !matches!(
            feature,
            Frames
                | Pictures
                | Tables
                | PartialFrameLines
                | FrameColumns
                | Notes
                | Footnotes
                | Bookmarks
        ),
        Format::Rtf => !matches!(
            feature,
            Frames | Pictures | Tables | Notes | Footnotes | Bookmarks
        ),
    }
}
