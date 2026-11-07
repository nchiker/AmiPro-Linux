//! Import: the export's formats read back (`docs/research/IMPORT_NOTES.md`,
//! `docs/Design/UI_SPEC.md` §31).
//!
//! As Ami Pro's filters read their format into the same tokens they write,
//! each reader here produces an [`ExchangeDocument`], with what it resolved
//! (style inheritance, themes, numbering, table styles) worked out into its
//! flat terms, and [`crate::realise`] turns any exchange document into the
//! model. What cannot be carried is never dropped silently: it becomes a
//! [`Finding`], which the Import Report lists, with the places it concerns.

use crate::ExchangeDocument;

/// Ami Pro's Import Options (`rtf.flt` dialog 100; VM IMC, IMA, IMK, IMI).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum StyleOption {
    /// The file's styles, as the file defines them.
    #[default]
    Convert,
    /// Body Text, each style's look kept as formatting.
    Apply,
    /// The file's style names, defined by the document's own styles.
    Keep,
    /// Body Text with only the file's own formatting.
    Ignore,
}

/// Ami Pro's ASCII Options (VM IMT, IMT3; `15e0:0000`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextOptions {
    pub line_ends: LineEnds,
    /// `@Name@` at a paragraph's start applies the style `Name`.
    pub keep_style_names: bool,
    pub characters: Characters,
}

impl Default for TextOptions {
    /// Each line a paragraph, the characters detected (IM9).
    fn default() -> Self {
        Self {
            line_ends: LineEnds::Paragraphs,
            keep_style_names: false,
            characters: Characters::Detect,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LineEnds {
    /// Every line ends a paragraph (CR/LF at paragraph ends only).
    #[default]
    Paragraphs,
    /// Lines are joined into paragraphs and a blank line ends one (CR/LF at
    /// lines, Ami Pro's default).
    Joined,
}

/// How a text file's bytes are read.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Characters {
    /// By the byte order mark, else UTF-8 if the bytes are, else Windows
    /// ANSI.
    #[default]
    Detect,
    Utf8,
    Utf16Le,
    Utf16Be,
    /// Windows code page 1252 (Ami Pro's 8 bit ANSI).
    WindowsAnsi,
    /// Code page 437 (Ami Pro's 8 bit PC-ASCII).
    PcAscii,
    /// The low seven bits only (Ami Pro's 7 bit ASCII).
    Ascii7,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ImportOptions {
    pub styles: StyleOption,
    pub text: TextOptions,
}

/// The formats read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceFormat {
    Docx,
    Odt,
    /// OpenDocument in one XML file (`.fodt`).
    FlatOdt,
    Rtf,
    Text,
    /// CommonMark with GitHub's extensions (MD2); found by its extension
    /// only, as plain text looks like it.
    Markdown,
}

impl SourceFormat {
    /// The format's name as the interface shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Docx => "Word Document",
            Self::Odt | Self::FlatOdt => "OpenDocument Text",
            Self::Rtf => "Rich Text",
            Self::Text => "Plain Text",
            Self::Markdown => "Markdown",
        }
    }

    /// Whether the format has styles (and so Import Options' style choice).
    pub fn has_styles(self) -> bool {
        !matches!(self, Self::Text)
    }
}

/// The format of `bytes`, from their contents first and the file's
/// extension second; `None` for none of the formats read (a SAM file, a
/// picture, a Word 97 document).
pub fn detect(bytes: &[u8], extension: Option<&str>) -> Option<SourceFormat> {
    let extension = extension.map(str::to_ascii_lowercase);
    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        // ODF packages begin with their stored `mimetype`.
        if bytes.get(30..38) == Some(b"mimetype")
            && bytes
                .get(38..38 + 39)
                .is_some_and(|kind| kind.starts_with(b"application/vnd.oasis.opendocument.text"))
        {
            return Some(SourceFormat::Odt);
        }
        return match extension.as_deref() {
            Some("odt" | "ott") => Some(SourceFormat::Odt),
            _ => Some(SourceFormat::Docx),
        };
    }
    let head = &bytes[..bytes.len().min(512)];
    let trimmed = trim_start(head);
    if trimmed.starts_with(b"{\\rtf") {
        return Some(SourceFormat::Rtf);
    }
    let text = String::from_utf8_lossy(head);
    if trimmed.starts_with(b"<?xml") || trimmed.starts_with(b"<office:document") {
        if text.contains("office:document") || extension.as_deref() == Some("fodt") {
            return Some(SourceFormat::FlatOdt);
        }
        return None;
    }
    // Ami Pro documents and style sheets, Word 97 and other binaries.
    if trimmed.starts_with(b"[ver]")
        || bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0])
        || bytes.starts_with(b"WordPro")
    {
        return None;
    }
    match extension.as_deref() {
        Some("md" | "markdown" | "mdown") => Some(SourceFormat::Markdown),
        Some("txt" | "text" | "asc" | "ascii") => Some(SourceFormat::Text),
        _ if looks_like_text(bytes) => Some(SourceFormat::Text),
        _ => None,
    }
}

fn trim_start(bytes: &[u8]) -> &[u8] {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    &bytes[start..]
}

/// Ami Pro's test (`15e0:0000`): at least 80% of the first 2 KB printable.
fn looks_like_text(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return true;
    }
    let head = &bytes[..bytes.len().min(2048)];
    if head.is_empty() {
        return true;
    }
    let printable = head
        .iter()
        .filter(|&&byte| byte >= 0x20 || matches!(byte, b'\t' | b'\n' | b'\r' | 0x0C))
        .count();
    printable * 5 >= head.len() * 4
}

/// What came in differently or was left out.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FindingKind {
    /// Kept as Ami Pro notes (IM6).
    Comments,
    /// Kept as numbered paragraphs at the document's end (IM6).
    Footnotes,
    Endnotes,
    /// Kept as their text (IM7).
    Links,
    Bookmarks,
    /// Accepted: the inserted text kept.
    Insertions,
    /// Accepted: the deleted text left out.
    Deletions,
    /// Formatting changes recorded as tracked changes (the new formatting
    /// kept).
    FormatChanges,
    /// Left out.
    HiddenText,
    /// Markdown pictures, which name a file: the file is not read, the
    /// description kept as text.
    LinkedPictures,
    /// Markdown's HTML, kept as plain text.
    Html,
    /// Text that was not valid UTF-8, read with replacement characters.
    BadCharacters,
    /// A character style applied as formatting.
    CharacterStyle(String),
    /// Text highlighting (a background colour), left out.
    Highlight,
    /// Letter spacing, text scaling, and the like, left out.
    CharacterSpacing,
    /// Paragraph borders on the sides Ami Pro has no rule for, or paragraph
    /// shading, left out.
    ParagraphBorders,
    /// A table inside a table cell, kept as its text.
    NestedTable,
    /// A floating table, kept in a frame.
    FloatingTable,
    /// A chart, kept as its picture where the file has one.
    Chart,
    /// A drawn shape (line, arrow, WordArt, group), kept as its picture
    /// where the file has one, else left out.
    Shape(String),
    /// An equation, kept as its picture or its text.
    Equation,
    /// A picture in a format the app cannot draw, kept as a box.
    Picture(String),
    /// A field with no Ami Pro counterpart, kept as its result.
    Field(String),
    /// A table of contents or an index, kept as its text.
    GeneratedText,
    /// Linked text boxes: each box keeps its own text.
    LinkedTextBoxes,
    /// A section that continues on the same page or column, started on a
    /// new page.
    SectionBreak,
    /// Columns of a section that continues on the page.
    Columns,
    /// Right-to-left or vertical text.
    TextDirection,
    /// A list number Ami Pro's counting cannot show, kept as text (IM5).
    ListNumber,
    /// A paragraph's own spacing, breaks, or rules, which Ami Pro holds only
    /// in styles, kept through a variant of its style.
    VariantStyle(String),
    /// Paragraphs' own space above and below, which Ami Pro holds only in
    /// styles, left out (Apply styles, as Ami Pro's Apply leaves it out).
    OwnSpacing,
    /// Headers and footers of an inserted file, left out (IM3).
    InsertedHeaders,
    /// A first page's own header or footer text: Ami Pro's pages all show
    /// the same (or none on the first, Begin on second page).
    FirstPageHeaders,
    /// Content controls and form fields, their content kept.
    FormFields,
    /// Embedded objects (OLE), kept as their picture.
    Objects,
    /// A part of the file that could not be read.
    Unreadable(String),
}

impl FindingKind {
    /// How the report names `count` of them.
    pub fn describe(&self, count: u32) -> String {
        let plural = |one: &str, many: &str| {
            if count == 1 {
                one.to_owned()
            } else {
                format!("{count} {many}")
            }
        };
        match self {
            Self::Comments => format!("{} kept as Ami Pro notes", plural("A comment", "comments")),
            Self::Footnotes => format!(
                "{} kept as numbered paragraphs at the end",
                plural("A footnote", "footnotes")
            ),
            Self::Endnotes => format!(
                "{} kept as numbered paragraphs at the end",
                plural("An endnote", "endnotes")
            ),
            Self::Links => format!("{} kept as their text", plural("A link", "links")),
            Self::Bookmarks => format!("{} left out", plural("A bookmark", "bookmarks")),
            Self::Insertions => format!(
                "Tracked changes accepted: {} kept",
                plural("an insertion", "insertions")
            ),
            Self::Deletions => format!(
                "Tracked changes accepted: {} left out",
                plural("a deletion", "deletions")
            ),
            Self::FormatChanges => format!(
                "Tracked formatting accepted: {}",
                plural("a change", "changes")
            ),
            Self::HiddenText => format!("{} left out", plural("Hidden text", "hidden passages")),
            Self::CharacterStyle(name) => {
                format!("Character style \u{201C}{name}\u{201D} applied as formatting")
            }
            Self::LinkedPictures => format!(
                "{} named a file, kept as its description",
                plural("A picture", "pictures")
            ),
            Self::Html => format!(
                "{} kept as text",
                plural("A piece of HTML", "pieces of HTML")
            ),
            Self::BadCharacters => format!(
                "{} not valid UTF-8, read with replacement characters",
                plural("A byte", "bytes")
            ),
            Self::Highlight => format!("{} left out", plural("A highlight", "highlights")),
            Self::CharacterSpacing => format!(
                "{} left out",
                plural(
                    "Letter spacing or scaling",
                    "places with letter spacing or scaling"
                )
            ),
            Self::ParagraphBorders => format!(
                "{} left out",
                plural(
                    "Paragraph borders or shading",
                    "paragraphs' borders or shading"
                )
            ),
            Self::NestedTable => format!(
                "{} kept as text in its cell",
                plural("A table inside a table", "tables inside tables")
            ),
            Self::FloatingTable => {
                format!(
                    "{} kept in a frame",
                    plural("A floating table", "floating tables")
                )
            }
            Self::Chart => format!("{} kept as a picture", plural("A chart", "charts")),
            Self::Shape(kind) => format!(
                "{} ({kind}) kept as a picture or left out",
                plural("A shape", "shapes")
            ),
            Self::Equation => format!(
                "{} kept as a picture or text",
                plural("An equation", "equations")
            ),
            Self::Picture(kind) => format!(
                "{} ({kind}) shown as a box",
                plural("A picture", "pictures")
            ),
            Self::Field(name) => format!(
                "{} ({name}) kept as its result",
                plural("A field", "fields")
            ),
            Self::GeneratedText => format!(
                "{} kept as text",
                plural(
                    "A table of contents or index",
                    "tables of contents or indexes"
                )
            ),
            Self::LinkedTextBoxes => format!(
                "{} each keep their own text",
                plural("Linked text boxes", "chains of linked text boxes")
            ),
            Self::SectionBreak => format!(
                "{} on the same page started on a new page",
                plural("A section", "sections")
            ),
            Self::Columns => format!(
                "{} on the same page as the text before kept without them",
                plural("Columns", "column changes")
            ),
            Self::TextDirection => format!(
                "{} left to right",
                plural(
                    "Right-to-left or vertical text",
                    "places of right-to-left or vertical text"
                )
            ),
            Self::ListNumber => format!("{} kept as text", plural("A list number", "list numbers")),
            Self::VariantStyle(name) => {
                format!("Paragraphs' own spacing or breaks kept as style \u{201C}{name}\u{201D}")
            }
            Self::OwnSpacing => format!(
                "{} own space above or below left out",
                plural("A paragraph's", "paragraphs'")
            ),
            Self::InsertedHeaders => "The inserted file's headers and footers left out".to_owned(),
            Self::FirstPageHeaders => {
                "The first page's own header or footer shown as the other pages'".to_owned()
            }
            Self::FormFields => format!(
                "{} kept as their content",
                plural(
                    "A content control or form field",
                    "content controls and form fields"
                )
            ),
            Self::Objects => format!(
                "{} kept as a picture",
                plural("An embedded object", "embedded objects")
            ),
            Self::Unreadable(part) => format!("A part could not be read: {part}"),
        }
    }
}

/// One line of the Import Report: what, how often, and where (markers the
/// readers put into the text, [`crate::Inline::Marker`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub kind: FindingKind,
    pub count: u32,
    pub markers: Vec<u32>,
}

/// The findings of one import, by kind in the order first found.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Findings {
    findings: Vec<Finding>,
    next_marker: u32,
}

impl Findings {
    /// Counts one of `kind`, without a place.
    pub fn count(&mut self, kind: FindingKind) {
        self.add(kind, 1, None);
    }

    /// Counts one of `kind` at a new marker, which the caller puts into the
    /// text where it happened.
    pub fn at(&mut self, kind: FindingKind) -> u32 {
        let marker = self.next_marker;
        self.next_marker += 1;
        self.add(kind, 1, Some(marker));
        marker
    }

    pub fn add(&mut self, kind: FindingKind, count: u32, marker: Option<u32>) {
        if count == 0 {
            return;
        }
        match self
            .findings
            .iter_mut()
            .find(|finding| finding.kind == kind)
        {
            Some(finding) => {
                finding.count += count;
                finding.markers.extend(marker);
            }
            None => self.findings.push(Finding {
                kind,
                count,
                markers: marker.into_iter().collect(),
            }),
        }
    }

    /// Takes in another import's findings (a part read on its own).
    pub fn merge(&mut self, other: Findings) {
        for finding in other.findings {
            self.add(finding.kind, finding.count, None);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Finding> {
        self.findings.iter()
    }

    pub fn len(&self) -> usize {
        self.findings.len()
    }

    /// The report as text (`Import report.txt`, the panel's Copy).
    pub fn report(&self) -> String {
        self.findings
            .iter()
            .map(|finding| format!("{}\n", finding.kind.describe(finding.count)))
            .collect()
    }
}

/// A file read into an exchange document.
#[derive(Clone, Debug, PartialEq)]
pub struct Imported {
    pub document: ExchangeDocument,
    pub findings: Findings,
    pub format: SourceFormat,
}

/// Why a file is not read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImportError {
    /// None of the formats read.
    UnknownFormat,
    /// The package is password-protected.
    Encrypted,
    /// The file is not what its format says (with what was wrong).
    Damaged(String),
    /// Larger than the limits allow.
    TooLarge,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownFormat => write!(f, "the file is not a document the app reads"),
            Self::Encrypted => write!(f, "the file is password-protected"),
            Self::Damaged(what) => write!(f, "the file is damaged: {what}"),
            Self::TooLarge => write!(f, "the file is too large"),
        }
    }
}

impl From<crate::zip::ZipError> for ImportError {
    fn from(error: crate::zip::ZipError) -> Self {
        match error {
            crate::zip::ZipError::Encrypted => Self::Encrypted,
            crate::zip::ZipError::TooLarge => Self::TooLarge,
            other => Self::Damaged(other.to_string()),
        }
    }
}

impl From<crate::xml_tree::XmlError> for ImportError {
    fn from(error: crate::xml_tree::XmlError) -> Self {
        Self::Damaged(error.to_string())
    }
}

/// The largest file read (the SAM reader's limit).
pub const MAX_FILE_SIZE: usize = 64 << 20;

/// Reads `bytes` (a file named with `extension`) in whichever of the
/// formats it is, packages inflated by `inflate`.
pub fn read(
    bytes: &[u8],
    extension: Option<&str>,
    options: &ImportOptions,
    inflate: crate::zip::Inflate,
) -> Result<Imported, ImportError> {
    if bytes.len() > MAX_FILE_SIZE {
        return Err(ImportError::TooLarge);
    }
    let format = detect(bytes, extension).ok_or(ImportError::UnknownFormat)?;
    let mut findings = Findings::default();
    let document = match format {
        SourceFormat::Text => crate::read_text::read_text(bytes, &options.text),
        SourceFormat::Rtf => crate::read_rtf::read_rtf(bytes, &mut findings)?,
        SourceFormat::Markdown => crate::read_markdown::read_markdown(bytes, &mut findings),
        SourceFormat::Docx => crate::read_docx::read_docx(bytes, inflate, &mut findings)?,
        SourceFormat::Odt => crate::read_odt::read_odt(bytes, inflate, &mut findings)?,
        SourceFormat::FlatOdt => crate::read_odt::read_fodt(bytes, &mut findings)?,
    };
    Ok(Imported {
        document,
        findings,
        format,
    })
}

/// The style option `realise` takes for a file of `format`: a text file's
/// style names (Keep style names) name the document's styles.
pub fn style_option(format: SourceFormat, options: &ImportOptions) -> StyleOption {
    if format == SourceFormat::Text {
        StyleOption::Keep
    } else {
        options.styles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_are_detected_by_their_contents() {
        assert_eq!(detect(b"{\\rtf1\\ansi}", None), Some(SourceFormat::Rtf));
        assert_eq!(
            detect(b"\r\n {\\rtf1", Some("doc")),
            Some(SourceFormat::Rtf)
        );
        let odt = crate::zip::write(
            &[crate::zip::Entry::stored(
                "mimetype",
                "application/vnd.oasis.opendocument.text",
            )],
            None,
        );
        assert_eq!(detect(&odt, Some("zip")), Some(SourceFormat::Odt));
        let docx = crate::zip::write(
            &[crate::zip::Entry::new("[Content_Types].xml", "<a/>")],
            None,
        );
        assert_eq!(detect(&docx, Some("docx")), Some(SourceFormat::Docx));
        assert_eq!(
            detect(
                b"<?xml version=\"1.0\"?><office:document xmlns:office=\"o\"/>",
                None
            ),
            Some(SourceFormat::FlatOdt)
        );
        assert_eq!(
            detect(b"Hello\r\nworld\r\n", None),
            Some(SourceFormat::Text)
        );
        assert_eq!(detect(b"[ver]\r\n\t4\r\n", Some("sam")), None);
        assert_eq!(detect(&[0xD0, 0xCF, 0x11, 0xE0, 0, 0], Some("doc")), None);
        assert_eq!(detect(&[0u8; 100], None), None);
    }

    #[test]
    fn findings_count_by_kind_and_keep_their_places() {
        let mut findings = Findings::default();
        let first = findings.at(FindingKind::Links);
        findings.count(FindingKind::Deletions);
        let second = findings.at(FindingKind::Links);
        let links = findings.iter().next().unwrap();
        assert_eq!(
            (links.count, links.markers.clone()),
            (2, vec![first, second])
        );
        assert_eq!(
            findings.report(),
            "2 links kept as their text\nTracked changes accepted: a deletion left out\n"
        );
    }
}
