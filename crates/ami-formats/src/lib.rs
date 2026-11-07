//! Defensive, read-only discovery parser for text-based Ami Pro SAM/STY files.
//!
//! This intentionally preserves every section and reports unsupported structures;
//! it is not yet a compatibility writer.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use amipro_document::{
    Alignment, Breaks, Bullets, Caps, CharacterAttributes, Document, Indents, LineSpacing,
    NumberFormat, ParagraphRules, ParagraphStyle, RuleLength, RuleSide, Spacing, StyleId,
    StyleKept, StyleOrigin, StyleSheet, StyleSheetLink, TabKind, TabLeader, TabStop, Twips,
    default_styles,
};

mod body;
mod build;
mod frames;
pub mod glossary;
mod info;
pub mod oracle;
pub mod protect;
pub mod roundtrip;
mod sam;
mod sheet;
mod structure;
mod text;
mod writer;

pub use info::{
    take_contents, take_doc_info, take_footnote_options, take_line_numbering, take_proofing,
};
pub use roundtrip::{Loss, LossKind, ami_pro_path, read_back, sam_losses};
pub use sam::{SamOptions, sam_document};
pub use sheet::{SheetOptions, document_description, layout_record, style_sheet};
pub use writer::{escape_text, style_record};

pub use build::StructureStats;

pub const MAX_LEGACY_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_LEGACY_LINES: usize = 2_000_000;
pub const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LegacyKind {
    Document,
    StyleSheet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Section {
    pub name: String,
    pub values: Vec<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyFile {
    pub kind: LegacyKind,
    pub format_version: Option<u32>,
    pub sections: Vec<Section>,
    pub style_names: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportReport {
    pub recognized_sections: usize,
    pub unknown_sections: Vec<String>,
    pub warnings: Vec<String>,
    pub safe_to_overwrite_original: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    TooLarge,
    TooManyLines,
    LineTooLong(usize),
    DataBeforeSection(usize),
    InvalidSection(usize),
    /// A Lotus Word Pro document (Ami Pro's successor), whose binary format
    /// starts with `WordPro`. Such files are sometimes named `.SAM`.
    WordPro,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => f.write_str("legacy input exceeds 64 MiB"),
            Self::TooManyLines => f.write_str("legacy input has too many lines"),
            Self::LineTooLong(line) => write!(f, "legacy input line {line} exceeds 1 MiB"),
            Self::DataBeforeSection(line) => write!(f, "data before first section at line {line}"),
            Self::InvalidSection(line) => write!(f, "invalid section header at line {line}"),
            Self::WordPro => {
                f.write_str("this is a Lotus Word Pro document, not an Ami Pro document")
            }
        }
    }
}

impl std::error::Error for ParseError {}

const RECOGNIZED: &[&str] = &[
    "ver",
    "sty",
    "files",
    "charset",
    "revisions",
    "prn",
    "port",
    "lang",
    "desc",
    "fopts",
    "lnopts",
    "docopts",
    "gramstyle",
    "paranum",
    "tag",
    "fnt",
    "algn",
    "spc",
    "brk",
    "line",
    "spec",
    "nfmt",
    "text",
    "txt",
    "edoc",
    "embedded",
];

pub fn parse(input: &[u8], kind: LegacyKind) -> Result<(LegacyFile, ImportReport), ParseError> {
    if input.len() > MAX_LEGACY_BYTES {
        return Err(ParseError::TooLarge);
    }
    if input.starts_with(b"WordPro") {
        return Err(ParseError::WordPro);
    }
    // Textual sections precede embedded OLE/media payloads in observed format-v4
    // files. A NUL cannot occur in the section grammar and marks the binary
    // boundary in corpus files whose trailing [Embedded] directory comes later.
    let textual_end = input
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(input.len());
    let textual_input = &input[..textual_end];
    let (text, decoded_windows_1252) = match std::str::from_utf8(textual_input) {
        Ok(text) => (text.to_owned(), false),
        Err(_) => (decode_windows_1252(textual_input), true),
    };
    let mut sections: Vec<Section> = Vec::new();
    let mut in_body = false;
    let mut binary_after_body = None;
    for (zero_index, raw_line) in text.lines().enumerate() {
        let line_number = zero_index + 1;
        if line_number > MAX_LEGACY_LINES {
            return Err(ParseError::TooManyLines);
        }
        if raw_line.len() > MAX_LINE_BYTES {
            return Err(ParseError::LineTooLong(line_number));
        }
        let line = raw_line.trim_end_matches('\r');
        let trimmed = line.trim();
        // Section names are letters and digits; an indented line like
        // `[<*:>]` is a value (a bullet's text, which Ami Pro writes with
        // its brackets unescaped).
        let section_like = trimmed.len() >= 3
            && trimmed.starts_with('[')
            && trimmed.ends_with(']')
            && trimmed[1..trimmed.len() - 1]
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_');
        let indented_value = line.starts_with('\t') && !section_like;
        if trimmed.starts_with('[') && !indented_value {
            if !trimmed.ends_with(']') || trimmed.len() < 3 {
                // Text in the body escapes `[` as `<[>`, so after [edoc] this
                // is an embedded payload without a NUL byte (one corpus style
                // sheet), not a section.
                if in_body {
                    binary_after_body = Some(line_number);
                    break;
                }
                return Err(ParseError::InvalidSection(line_number));
            }
            in_body |= trimmed.eq_ignore_ascii_case("[edoc]");
            sections.push(Section {
                name: trimmed[1..trimmed.len() - 1].to_owned(),
                values: vec![],
                line: line_number,
            });
            if trimmed.eq_ignore_ascii_case("[embedded]") {
                break;
            }
        } else if !line.is_empty() && !(trimmed.is_empty() && sections.is_empty()) {
            // Ami Pro writes an empty field as an indented blank line
            // (`[nfmt]`'s currency symbol, `[spec]`'s text): it is a value,
            // so the fields after it keep their places. Only a line with
            // nothing on it is not.
            let Some(section) = sections.last_mut() else {
                return Err(ParseError::DataBeforeSection(line_number));
            };
            section
                .values
                .push(line.trim_start_matches([' ', '\t']).to_owned());
        }
    }

    let format_version = sections
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case("ver"))
        .and_then(|s| s.values.first())
        .and_then(|value| value.trim().parse().ok());
    let style_names = sections
        .iter()
        .filter(|s| s.name.eq_ignore_ascii_case("tag"))
        .filter_map(|s| s.values.first())
        .filter(|name| !name.is_empty())
        .cloned()
        .collect();
    let unknown_sections: Vec<String> = sections
        .iter()
        .filter(|section| {
            !RECOGNIZED
                .iter()
                .any(|known| section.name.eq_ignore_ascii_case(known))
        })
        .map(|section| section.name.clone())
        .collect();
    let recognized_sections = sections.len() - unknown_sections.len();
    let mut warnings = Vec::new();
    if format_version.is_none() {
        warnings.push("missing or nonnumeric [ver] section".into());
    }
    if decoded_windows_1252 {
        warnings.push("input was decoded as Windows-1252 after UTF-8 validation failed".into());
    }
    if let Some(line) = binary_after_body {
        warnings.push(format!(
            "binary data after the document body at line {line} was not read as sections"
        ));
    }
    if !unknown_sections.is_empty() {
        warnings.push("unknown sections retained in the intermediate representation".into());
    }
    Ok((
        LegacyFile {
            kind,
            format_version,
            sections,
            style_names,
        },
        ImportReport {
            recognized_sections,
            unknown_sections,
            warnings,
            safe_to_overwrite_original: false,
        },
    ))
}

pub(crate) fn decode_windows_1252(input: &[u8]) -> String {
    amipro_document::decode_windows_1252(input)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SamImportReport {
    pub format_version: Option<u32>,
    pub paragraphs_imported: usize,
    pub styles_imported: usize,
    pub associated_style: Option<String>,
    pub associated_style_resolved: bool,
    /// Where the STY was found, when the import searched paths for it.
    pub associated_style_path: Option<PathBuf>,
    pub unsupported_sections: Vec<String>,
    /// Inline controls the model does not map, by their first line. They are
    /// kept in the document as legacy marks.
    pub unsupported_controls: Vec<String>,
    /// Pages, layouts, frames, anchors, and what was kept or dropped.
    pub structure: StructureStats,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SamImport {
    pub document: Document,
    pub report: SamImportReport,
}

#[derive(Debug)]
pub enum ImportError {
    Io(String),
    Parse(ParseError),
    MissingDocumentBody,
    /// Ami Pro saved the document with a password: an `[encrypt]` section
    /// followed by coded data in place of the text sections (`protect`).
    /// Only File › Open asks for the password; other commands refuse the
    /// file with Ami Pro's message 3148.
    PasswordProtected,
    /// The password given does not open the file (Ami Pro's 3147).
    WrongPassword,
    /// A file offered as a style sheet holds no paragraph styles.
    NotAStyleSheet,
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(message) => f.write_str(message),
            Self::Parse(error) => error.fmt(f),
            Self::MissingDocumentBody => f.write_str("SAM file has no [edoc] body"),
            Self::PasswordProtected => f.write_str(
                "Cannot do that command to an encrypted file. Open it with its password and save it without one first.",
            ),
            Self::WrongPassword => f.write_str(
                "Invalid Password. Encrypted file will not open with incorrect password.",
            ),
            Self::NotAStyleSheet => {
                f.write_str("the file is not an Ami Pro style sheet: it has no paragraph styles")
            }
        }
    }
}

impl std::error::Error for ImportError {}

impl From<ParseError> for ImportError {
    fn from(value: ParseError) -> Self {
        Self::Parse(value)
    }
}

/// Imports supported SAM content into the GUI-independent document model.
///
/// The optional STY data is used before any styles embedded in the SAM. The
/// original input is never written and imported documents remain unsafe to
/// save over the source SAM.
pub fn import_sam(
    sam: &[u8],
    associated_style_data: Option<&[u8]>,
    title: impl Into<String>,
) -> Result<SamImport, ImportError> {
    let (legacy, discovery) = parse(sam, LegacyKind::Document)?;
    if legacy
        .sections
        .first()
        .is_some_and(|section| section.name.eq_ignore_ascii_case("encrypt"))
    {
        return Err(ImportError::PasswordProtected);
    }
    let associated_style = section_value(&legacy, "sty").filter(|value| !value.is_empty());
    let external = associated_style_data
        .map(|data| parse(data, LegacyKind::StyleSheet))
        .transpose()?;

    let mut style_specs = BTreeMap::new();
    if let Some((style_sheet, _)) = &external {
        collect_style_specs(style_sheet, &mut style_specs, StyleOrigin::Sheet);
    }
    // A SAM's own `[tag]`s are the styles stored in the document: created
    // or modified there (Style Management, VM fixture ST1).
    collect_style_specs(&legacy, &mut style_specs, StyleOrigin::Document);

    let mut styles = BTreeMap::new();
    let mut style_ids = BTreeMap::new();
    for (index, spec) in style_specs.values().enumerate() {
        let id = StyleId(index as u64 + 1);
        style_ids.insert(spec.name.to_lowercase(), id);
        styles.insert(
            id,
            ParagraphStyle {
                id,
                name: spec.name.clone(),
                parent: None,
                font_family: spec.font_family.clone(),
                font_size_points: spec.font_size_points,
                bold: spec.bold,
                italic: spec.italic,
                spacing: spec.spacing,
                indents: spec.indents,
                breaks: spec.breaks,
                alignment: spec.alignment,
                attributes: spec.attributes,
                rules: spec.rules,
                next: None,
                tabs: spec.tabs.clone(),
                hanging: spec.hanging,
                hyphenation: spec.hyphenation,
                bullets: spec.bullets.clone(),
                origin: spec.origin,
                both_sides: spec.both_sides,
                first_line_bold: spec.first_line_bold,
                kept: spec.kept.clone(),
            },
        );
    }
    // The next styles, now that every style has an ID; one naming itself or
    // a style no sheet defines is the style itself.
    for spec in style_specs.values() {
        let id = style_ids[&spec.name.to_lowercase()];
        let next = spec
            .next
            .as_ref()
            .and_then(|name| style_ids.get(&name.to_lowercase()))
            .copied()
            .filter(|next| *next != id);
        if let Some(style) = styles.get_mut(&id) {
            style.next = next;
            // A name that names no style here is kept as written.
            style.kept.next_name = spec
                .raw_next
                .clone()
                .filter(|raw| next.is_none() && !raw.trim().eq_ignore_ascii_case(&spec.name));
        }
    }
    if styles.is_empty() {
        styles = default_styles();
        // The sheet's Body Text is unknown: it keeps the 6 pt below that new
        // documents had until 2026-10-18, which the layout comparison was
        // measured with (as a missing style's spacing, below).
        for style in styles.values_mut().filter(|style| style.name == BODY_TEXT) {
            style.spacing = Spacing::below(120);
        }
        // A document that names a style sheet that is not here keeps its
        // styles there: the defaults stand for the sheet's and are never
        // written into the document.
        if associated_style.is_some() {
            for style in styles.values_mut() {
                style.origin = StyleOrigin::Sheet;
            }
        }
        for style in styles.values() {
            style_ids.insert(style.name.to_lowercase(), style.id);
        }
    }
    // The structure and the body are Windows-1252 text up to the binary
    // area; the embedded directory is read from the raw bytes.
    let textual_end = sam.iter().position(|byte| *byte == 0).unwrap_or(sam.len());
    let decoded = decode_windows_1252(&sam[..textual_end]);
    let lines: Vec<&str> = decoded
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let body_start = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case("[edoc]"))
        .ok_or(ImportError::MissingDocumentBody)?;
    let mut structure = structure::read_structure(&lines[..body_start]);
    // A SAM stores a page layout only when it was changed there; otherwise
    // the layout is the style sheet's (user's guide; VM fixture ST1). Its
    // standard layout is the one named Standard (Ami Pro renames every
    // other as it writes it, `11c8:3ba0`); 7 linked corpus files store only
    // inserted layouts and take Standard from the sheet.
    let standard = structure
        .layouts
        .iter()
        .position(|layout| is_standard_layout(&layout.name));
    if let Some(index) = standard.filter(|index| *index > 0) {
        let layout = structure.layouts.remove(index);
        structure.layouts.insert(0, layout);
    }
    let sheet_layouts = associated_style_data.map(|sheet| {
        let end = sheet
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(sheet.len());
        let decoded = decode_windows_1252(&sheet[..end]);
        let sheet_lines: Vec<&str> = decoded
            .lines()
            .map(|line| line.trim_end_matches('\r'))
            .collect();
        let sheet_end = sheet_lines
            .iter()
            .position(|line| line.trim().eq_ignore_ascii_case("[edoc]"))
            .unwrap_or(sheet_lines.len());
        structure::read_structure(&sheet_lines[..sheet_end]).layouts
    });
    // Without its own Standard, a document takes its style sheet's, as Ami
    // Pro does: every style sheet carries one (user's guide). Without the
    // sheet, the layouts it stores are all there is.
    let own_layout = match &sheet_layouts {
        Some(sheet) if standard.is_none() && !sheet.is_empty() => {
            for (index, layout) in sheet.iter().enumerate() {
                structure.layouts.insert(index, layout.clone());
            }
            false
        }
        _ => !structure.layouts.is_empty(),
    };
    let (body, _) = text::split_body(lines[body_start + 1..].iter().copied());
    let mid_line = text::controls_ending_mid_line(lines[body_start + 1..].iter().copied());
    let mut built = build::build(title.into(), styles, style_ids, &structure, &body, sam);
    if mid_line > 0 {
        built.warnings.push(format!(
            "{mid_line} note or floating header/footer controls end with text after their `>` line; Ami Pro reads on past them"
        ));
    }
    // Table Text generated for a table (`build.rs`) when the document's
    // style sheet is not here: the SAM stores no record for it, so in Ami
    // Pro it is the sheet's.
    if associated_style.is_some() && external.is_none() && !style_specs.contains_key("table text") {
        for style in built.document.styles_mut().values_mut() {
            if style.name.eq_ignore_ascii_case("Table Text") {
                style.origin = StyleOrigin::Sheet;
            }
        }
    }
    let mut votes = std::mem::take(&mut built.face_kinds);
    for spec in style_specs.values() {
        *votes
            .entry(spec.font_family.to_lowercase())
            .or_default()
            .entry(spec.face_kind)
            .or_default() += 1;
    }
    built.document.set_face_kinds(face_kinds(&votes));
    // The hyphenation zone, `[docopts]` value 1 (decompile: Ami Pro's
    // default 5).
    if let Some(zone) = section_value(&legacy, "docopts")
        .and_then(|value| value.trim().parse::<u16>().ok())
        .filter(|zone| *zone > 0)
    {
        built.document.set_hyphenation_zone(zone);
    }
    // The order of the document's own style records: Ami Pro writes the
    // styles without a function key in its list's order, which is not
    // alphabetical (147 of 171 corpus files), so the writer follows it.
    let own_order: Vec<String> = legacy
        .sections
        .iter()
        .filter(|section| section.name.eq_ignore_ascii_case("tag"))
        .filter_map(|section| section.values.first())
        .filter(|name| !name.trim().is_empty())
        .map(|name| text::decode_name(name.trim()))
        .collect();
    if !own_order.is_empty() {
        built
            .document
            .legacy_mut()
            .push("style order", own_order.join("\n"));
    }
    built
        .document
        .set_style_sheet(associated_style.clone().map(|name| StyleSheetLink {
            name,
            layout_from_sheet: !own_layout,
        }));
    // `[l1]`: the layout the document starts in, by name (`1058:3156`); `0`,
    // or a name no layout has, is the standard one.
    if let Some(name) = section_value(&legacy, "l1") {
        let start = built
            .document
            .layouts()
            .iter()
            .find(|layout| layout.name.trim().eq_ignore_ascii_case(name.trim()))
            .map(|layout| layout.id);
        built.document.set_start_layout(start);
    }
    built
        .document
        .legacy_mut()
        .values
        .retain(|value| !value.key.eq_ignore_ascii_case("section [l1]"));
    // A layout that is the style sheet's is written into the document only
    // once it changes: what it was is kept to tell (`sam.rs`).
    if associated_style.is_some() && !own_layout {
        let fingerprint = sam::layout_fingerprint(&built.document);
        built
            .document
            .legacy_mut()
            .push(sam::SHEET_LAYOUT_KEY, fingerprint);
    }
    // Values kept only where the writer would write something else.
    frames::drop_derived_values(&mut built.document);
    sheet::drop_derived_layout_values(&mut built.document);
    // Marks the writer derives are not kept, so not counted below.
    let derived_marks = body::drop_derived_ruler_marks(&mut built.document);
    built.stats.legacy_marks -= derived_marks;
    // The rows as read, once every derived value is gone.
    build::remember_read_rows(&mut built.document);
    if let Some(name) = associated_style.as_ref().filter(|_| external.is_none()) {
        built
            .document
            .legacy_mut()
            .push(amipro_document::STYLE_SHEET_MISSING_KEY, name.clone());
    }
    let document = built.document;
    document
        .validate()
        .map_err(|error| ImportError::Io(format!("imported model is invalid: {error}")))?;
    let mut warnings = discovery.warnings;
    warnings.extend(built.warnings);
    if associated_style.is_some() && external.is_none() {
        warnings.push(
            "associated STY file was not available; named fallback styles were created".into(),
        );
    }
    if built.stats.legacy_marks > 0 {
        warnings.push(format!(
            "{} inline controls without a model mapping were kept as legacy marks",
            built.stats.legacy_marks
        ));
    }
    let paragraphs_imported = document.paragraphs().len();
    let styles_imported = document.styles().len();
    Ok(SamImport {
        document,
        report: SamImportReport {
            format_version: legacy.format_version,
            paragraphs_imported,
            styles_imported,
            associated_style,
            associated_style_resolved: external.is_some(),
            associated_style_path: None,
            unsupported_sections: discovery.unknown_sections,
            unsupported_controls: built.unmapped_controls.into_iter().collect(),
            structure: built.stats,
            warnings,
        },
    })
}

/// Reads a style sheet (`.STY`) for Use Another Style Sheet: its paragraph
/// styles and its standard page layout. A file without paragraph styles,
/// a Word Pro file, or a password-protected one is refused with a message.
pub fn read_style_sheet(bytes: &[u8], name: impl Into<String>) -> Result<StyleSheet, ImportError> {
    let name = name.into();
    let (legacy, _) = parse(bytes, LegacyKind::StyleSheet)?;
    let mut specs = BTreeMap::new();
    collect_style_specs(&legacy, &mut specs, StyleOrigin::Sheet);
    if specs.is_empty() {
        return Err(ImportError::NotAStyleSheet);
    }
    // Two of the user's style sheets have no body section; the styles and
    // layout are all a sheet needs.
    let text_end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let mut text = bytes[..text_end].to_vec();
    if !legacy
        .sections
        .iter()
        .any(|section| section.name.eq_ignore_ascii_case("edoc"))
    {
        text.extend_from_slice(b"\r\n[edoc]\r\n>\r\n");
    }
    let document = import_sam(&text, None, name.clone())?.document;
    let styles = document
        .styles()
        .values()
        .filter(|style| style.origin != StyleOrigin::Missing)
        .map(|style| ParagraphStyle {
            origin: StyleOrigin::Sheet,
            ..style.clone()
        })
        .collect();
    Ok(StyleSheet {
        name,
        styles,
        layout: document.layouts().first().map(|layout| (**layout).clone()),
    })
}

/// Reads a SAM and resolves its declared STY beside the document or in a
/// nearby `styles` directory, using case-insensitive DOS-era name matching.
pub fn import_sam_path(path: impl AsRef<Path>) -> Result<SamImport, ImportError> {
    import_sam_path_with_style_directories(path, &[])
}

/// Like [`import_sam_path`], then also looks for the STY in
/// `style_directories`, in order, as Ami Pro looks in its style-sheet path.
pub fn import_sam_path_with_style_directories(
    path: impl AsRef<Path>,
    style_directories: &[PathBuf],
) -> Result<SamImport, ImportError> {
    import_sam_path_with_password(path, style_directories, None)
}

/// [`import_sam_path_with_style_directories`] for a file that may be
/// password protected (`protect`): with the password it opens; without it,
/// or with a wrong one, it is refused with [`ImportError::PasswordProtected`]
/// or [`ImportError::WrongPassword`].
pub fn import_sam_path_with_password(
    path: impl AsRef<Path>,
    style_directories: &[PathBuf],
    password: Option<&str>,
) -> Result<SamImport, ImportError> {
    let path = path.as_ref();
    let mut sam = fs::read(path).map_err(|error| ImportError::Io(error.to_string()))?;
    if protect::is_protected(&sam) {
        let Some(password) = password else {
            return Err(ImportError::PasswordProtected);
        };
        sam = protect::unprotect(&sam, password).map_err(|_| ImportError::WrongPassword)?;
    }
    let (legacy, _) = parse(&sam, LegacyKind::Document)?;
    let style_name = section_value(&legacy, "sty").filter(|value| !value.is_empty());
    let style_path = style_name
        .as_deref()
        .and_then(|name| resolve_style_path(path, name, style_directories));
    let style_data = style_path
        .as_ref()
        .map(fs::read)
        .transpose()
        .map_err(|error| ImportError::Io(error.to_string()))?;
    let title = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Imported document");
    let mut imported = import_sam(&sam, style_data.as_deref(), title)?;
    imported.report.associated_style_path = style_path;
    Ok(imported)
}

/// Whether a layout is a document's standard layout: Ami Pro names it
/// Standard and gives every other a number when it writes it.
pub(crate) fn is_standard_layout(name: &str) -> bool {
    name.trim().eq_ignore_ascii_case("Standard")
}

/// The paragraph style Ami Pro applies to paragraphs that carry no `@tag@`.
/// Its formatting comes from the document's style sheet.
pub(crate) const BODY_TEXT: &str = "Body Text";

/// Finds a paragraph style by case-insensitive name, creating a named fallback
/// when neither the SAM nor its style sheet defines it.
pub(crate) fn resolve_style(
    styles: &mut BTreeMap<StyleId, ParagraphStyle>,
    style_ids: &mut BTreeMap<String, StyleId>,
    name: &str,
) -> StyleId {
    let key = name.to_lowercase();
    if let Some(id) = style_ids.get(&key) {
        return *id;
    }
    let id = StyleId(styles.keys().next_back().map_or(1, |id| id.0 + 1));
    let fallback = style_ids
        .get(&BODY_TEXT.to_lowercase())
        .or_else(|| styles.keys().next())
        .map(|id| styles[id].clone())
        .unwrap_or_else(|| default_styles()[&StyleId(1)].clone());
    // The font follows Body Text; the indents and rules do not: Ami Pro's generated
    // Table Text has none where Body Text is indented (44 of 49 such style
    // sheets in the user's corpus), and a missing style's are unknown. Its
    // spacing is unknown too: 6 pt below matches Ami Pro's caches better
    // than Body Text's spacing on the ISO corpus, whose style sheets are
    // missing (247 page ends against 188 of 1,306).
    styles.insert(
        id,
        ParagraphStyle {
            id,
            name: name.to_owned(),
            parent: None,
            indents: Indents::default(),
            spacing: Spacing::below(120),
            breaks: Breaks::default(),
            alignment: Alignment::Left,
            rules: ParagraphRules::default(),
            next: None,
            tabs: Vec::new(),
            hanging: false,
            bullets: Bullets::default(),
            // Defined nowhere: Ami Pro shows the name red (VM fixture ST1).
            origin: StyleOrigin::Missing,
            // A new style has no function key until Ami Pro gives it a
            // free one.
            kept: StyleKept {
                function_key: 0,
                ..fallback.kept.clone()
            },
            ..fallback
        },
    );
    style_ids.insert(key, id);
    id
}

fn section_value(legacy: &LegacyFile, name: &str) -> Option<String> {
    legacy
        .sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case(name))
        .and_then(|section| section.values.first())
        .map(|value| value.trim().to_owned())
}

#[derive(Clone)]
struct StyleSpec {
    name: String,
    font_family: String,
    font_size_points: f32,
    bold: bool,
    italic: bool,
    spacing: Spacing,
    indents: Indents,
    breaks: Breaks,
    alignment: Alignment,
    attributes: CharacterAttributes,
    rules: ParagraphRules,
    /// The next style's name (after `[nfmt]`'s own six values).
    next: Option<String>,
    /// The same as written, even empty, when the record has it.
    raw_next: Option<String>,
    tabs: Vec<TabStop>,
    hanging: bool,
    hyphenation: bool,
    bullets: Bullets,
    /// The face's Windows pitch and family byte from `[fnt]`'s kind bits.
    face_kind: u8,
    /// The document's own style, or the style sheet's.
    origin: StyleOrigin,
    both_sides: bool,
    /// `[fnt]` 4096: Modify Style's 1st line bold.
    first_line_bold: bool,
    kept: StyleKept,
}

/// A tab stop from a SAM type and position (decompile `10d0:1881`; VM
/// fixture TB2): the low 14 bits of the type are 1 left, 2 centre, 3
/// right, 4 decimal, anything else left; the top two bits the leader, 1
/// hyphens, 2 dots, 3 a line. `None` for a type or position that is not a
/// 16-bit value.
pub(crate) fn tab_stop(kind: i64, position: i64) -> Option<TabStop> {
    let kind = u16::try_from(kind).ok()?;
    let position = Twips::try_from(position)
        .ok()
        .filter(|at| (0..=0xFFFF).contains(at))?;
    Some(TabStop {
        kind: match kind & 0x3FFF {
            2 => TabKind::Centre,
            3 => TabKind::Right,
            4 => TabKind::Decimal,
            _ => TabKind::Left,
        },
        position,
        leader: match kind >> 14 {
            1 => TabLeader::Hyphens,
            2 => TabLeader::Dots,
            3 => TabLeader::Line,
            _ => TabLeader::None,
        },
    })
}

/// A style's own tabs: after `[nfmt]`'s own six values and the next
/// style's name, a count (at most 22 are kept) and that many type and
/// position pairs (decompile); the right indent follows them.
fn style_tabs(sections: &[Section]) -> Vec<TabStop> {
    let Some(values) = sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("nfmt"))
        .map(|section| &section.values)
    else {
        return Vec::new();
    };
    let number = |index: usize| {
        values
            .get(index)
            .and_then(|value| value.trim().parse::<i64>().ok())
    };
    let count = number(7).map_or(0, |count| count.clamp(0, 22) as usize);
    (0..count)
        .filter_map(|index| tab_stop(number(8 + 2 * index)?, number(9 + 2 * index)?))
        .collect()
}

/// A style's Bullets & numbers from `[spec]`'s ten values (decompile
/// `1058:0fac`; VM fixtures BN1, BN2): value 3 is the text before the
/// paragraph, where `<*X>` is the code X − 0x20 (bullets and number
/// formats) and the other escapes are as in the body; value 2 the outline
/// level, 7 the text's attributes, 8 flags (16 Right align). Missing or
/// malformed values are Ami Pro's defaults.
fn style_bullets(sections: &[Section]) -> Bullets {
    let defaults = Bullets::default();
    let Some(section) = sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("spec"))
    else {
        return defaults;
    };
    let number = |index: usize, default: u32| {
        section
            .values
            .get(index)
            .and_then(|value| value.trim().parse::<u32>().ok())
            .unwrap_or(default)
    };
    // Trailing spaces belong to the text ("1. "); the parser has dropped
    // the indentation.
    let raw = section.values.get(2).map_or("", String::as_str);
    let mut text = String::new();
    let mut rest = raw;
    while let Some(character) = rest.chars().next() {
        if character == '<' {
            if let Some(after) = rest.strip_prefix("<<") {
                text.push('<');
                rest = after;
                continue;
            }
            // Codes below 0x20 are bullets and number formats (`<*X>`).
            if let Some((byte, after)) = text::leading_escape(rest) {
                text.push(if byte < 0x20 {
                    char::from(byte)
                } else {
                    decode_windows_1252(&[byte])
                        .chars()
                        .next()
                        .unwrap_or(char::REPLACEMENT_CHARACTER)
                });
                rest = after;
                continue;
            }
        }
        text.push(character);
        rest = &rest[character.len_utf8()..];
    }
    Bullets {
        text,
        level: u8::try_from(number(1, 0)).unwrap_or(0),
        attributes: u16::try_from(number(6, 0)).unwrap_or(0),
        flags: u16::try_from(number(7, 0)).unwrap_or(0),
        kept: [
            number(0, defaults.kept[0]),
            number(3, defaults.kept[1]),
            number(4, defaults.kept[2]),
            number(5, defaults.kept[3]),
            number(8, defaults.kept[4]),
            number(9, defaults.kept[5]),
        ],
    }
}

/// The next style's name: the first value after `[nfmt]`'s own six
/// (decompile; a style without it is its own next style).
fn style_next(sections: &[Section]) -> Option<String> {
    sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("nfmt"))
        .and_then(|section| section.values.get(6))
        .map(|name| text::decode_name(name.trim()))
        .filter(|name| !name.is_empty())
}

/// A style's paragraph rules from `[line]`'s ten values (decompile
/// `1058:0fac`; Modify Style › Lines, VM fixtures NX1, NX2): flags (1 above,
/// 2 below, 4 Length Text, 8 Margins, else Other), the Other length,
/// a value kept, the colour, a value kept, the line styles above and below,
/// the space each takes, and the dialog's unit. Missing or malformed values
/// are Ami Pro's defaults.
fn style_rules(sections: &[Section]) -> ParagraphRules {
    let defaults = ParagraphRules::default();
    let Some(section) = sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("line"))
    else {
        return defaults;
    };
    let value = |index: usize| {
        section
            .values
            .get(index)
            .and_then(|value| value.trim().parse::<u32>().ok())
    };
    let small = |index: usize, default: u8| {
        value(index)
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(default)
    };
    let twips = |index: usize, default: Twips| {
        value(index)
            .filter(|value| *value <= 1_000_000)
            .map_or(default, |value| value as Twips)
    };
    let flags = value(0)
        .and_then(|flags| u16::try_from(flags).ok())
        .unwrap_or_else(|| defaults.flags());
    ParagraphRules {
        above: RuleSide {
            on: flags & 1 != 0,
            line: small(5, defaults.above.line),
            space: twips(7, defaults.above.space),
        },
        below: RuleSide {
            on: flags & 2 != 0,
            line: small(6, defaults.below.line),
            space: twips(8, defaults.below.space),
        },
        length: if flags & 4 != 0 {
            RuleLength::Text
        } else if flags & 8 != 0 {
            RuleLength::Margins
        } else {
            RuleLength::Other
        },
        other_length: twips(1, defaults.other_length),
        colour: value(3).map_or(defaults.colour, |colour| colour & 0x00FF_FFFF),
        other_flags: flags & !0xF,
        unknown: [small(2, defaults.unknown[0]), small(4, defaults.unknown[1])],
        unit: small(9, defaults.unit),
    }
}

/// A style's character attributes from `[fnt]`: the colour (value 3, a
/// `COLORREF`) and the flags (value 4): 4 underline, 8 word underline, 16
/// upper case, 32 small caps, 64 double underline, 128 strikethrough, 256
/// superscript, 512 subscript, 1024 lower case, 2048 initial caps (4096 is
/// 1st line bold, `ParagraphStyle::first_line_bold`, not protected text:
/// decompile `FUN_10d0_1aa6`, `FIRST_LINE_BOLD_NOTES.md`). Of
/// two caps, the
/// one Ami Pro's display applies first wins: small caps, upper, lower,
/// initial.
fn style_attributes(colour: u32, flags: u32) -> CharacterAttributes {
    let caps = [
        (32, Caps::Small),
        (16, Caps::Upper),
        (1024, Caps::Lower),
        (2048, Caps::Initial),
    ]
    .into_iter()
    .find(|(bit, _)| flags & bit != 0)
    .map(|(_, caps)| caps);
    CharacterAttributes {
        underline: flags & 4 != 0,
        word_underline: flags & 8 != 0,
        double_underline: flags & 64 != 0,
        strikethrough: flags & 128 != 0,
        superscript: flags & 256 != 0,
        subscript: flags & 512 != 0,
        caps,
        colour: colour & 0x00FF_FFFF,
        protected: false,
        no_hyphenation: false,
        overstrike: None,
    }
}

/// What a style record holds beyond the model's fields, as read (decompile
/// `11c8:26b7`): the function key (`[tag]` value 2), `[fnt]`'s face-kind
/// bits, the dialogs' units, `[spc]`'s Add in bits and Single height, and
/// `[nfmt]`'s six values. Missing values are Ami Pro's defaults.
fn style_kept(tag: &Section, sections: &[Section], font_flags: u32) -> StyleKept {
    let defaults = StyleKept::default();
    let find = |name: &str| {
        sections
            .iter()
            .find(|section| section.name.eq_ignore_ascii_case(name))
    };
    let number = |section: Option<&Section>, index: usize| {
        section
            .and_then(|section| section.values.get(index))
            .and_then(|value| value.trim().parse::<i64>().ok())
    };
    let byte = |section, index, default: u8| {
        number(section, index)
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(default)
    };
    let alignment = find("algn");
    let spacing = find("spc");
    let spacing_flags = number(spacing, 0).unwrap_or(1);
    // As `style_spacing` reads it: Single, or no other mode and no height.
    let single = spacing_flags & 1 != 0
        || (spacing_flags & 6 == 0 && number(spacing, 1).is_none_or(|height| height <= 0));
    let mut values = defaults.number_format.to_values();
    if let Some(section) = find("nfmt") {
        for (kept, value) in values.iter_mut().zip(&section.values) {
            value.clone_into(kept);
        }
    }
    // A value that is no number falls back to the default format.
    let number_format = NumberFormat::from_values(&values).unwrap_or(defaults.number_format);
    StyleKept {
        function_key: byte(Some(tag), 1, 0),
        face_bits: (font_flags & 0xC000) as u16,
        units: [
            byte(alignment, 1, 1),
            byte(spacing, 2, 1),
            byte(spacing, 5, 1),
        ],
        add_in: (spacing_flags & 48) as u8,
        single_height: if single {
            number(spacing, 1)
                .and_then(|height| Twips::try_from(height).ok())
                .unwrap_or(0)
        } else {
            0
        },
        number_format,
        // Set once the next style's name is resolved.
        next_name: None,
        alignment_bits: (number(alignment, 0).unwrap_or(0) & 0xE0) as u16,
    }
}

/// The Windows pitch and family byte Ami Pro makes of `[fnt]`'s kind bits
/// (decompile): `0x4000` variable-pitch swiss, `0xC000` variable-pitch
/// roman, `0x8000` fixed-pitch modern, neither fixed pitch of any family.
fn style_face_kind(flags: u32) -> u8 {
    match flags & 0xC000 {
        0x4000 => 0x22,
        0xC000 => 0x12,
        0x8000 => 0x31,
        _ => 0x01,
    }
}

/// Each face's pitch and family byte from what the styles and the text's
/// font changes gave it: the commonest byte that names a family (roman,
/// swiss, modern, script, decorative), else the commonest; the smaller on a
/// tie.
fn face_kinds(votes: &text::FaceKindVotes) -> BTreeMap<String, u8> {
    votes
        .iter()
        .filter_map(|(family, counts)| {
            let pick = |named: bool| {
                counts
                    .iter()
                    .filter(|(kind, _)| !named || (0x10..0x60).contains(&(**kind & 0xF0)))
                    .max_by_key(|(kind, count)| (**count, std::cmp::Reverse(**kind)))
                    .map(|(kind, _)| *kind)
            };
            Some((family.clone(), pick(true).or_else(|| pick(false))?))
        })
        .collect()
}

/// `[algn]` value 1: alignment flags 1 left, 2 right, 4 centre, 8
/// justified, 16 Both sides, 256 hyphenation, 512 hanging indent (I1–I3,
/// decompile). Left alone when the style has none.
fn alignment_flags(sections: &[Section]) -> u32 {
    sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("algn"))
        .and_then(|section| section.values.first())
        .and_then(|value| value.trim().parse::<u32>().ok())
        .unwrap_or(1)
}

/// A style's alignment (I1–I3; O2 for the same four over one paragraph).
/// Ami Pro's layout tests right, then centre, then justified; anything
/// else is left (decompile).
fn style_alignment(sections: &[Section]) -> Alignment {
    let flags = alignment_flags(sections);
    if flags & 2 != 0 {
        Alignment::Right
    } else if flags & 4 != 0 {
        Alignment::Centre
    } else if flags & 8 != 0 {
        Alignment::Justify
    } else {
        Alignment::Left
    }
}

/// A style's `[brk]` bits as written (VM fixtures B1–B8), or Ami Pro's
/// default (breaks allowed within) when the style has none.
fn style_breaks(sections: &[Section]) -> Breaks {
    sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("brk"))
        .and_then(|section| section.values.first())
        .and_then(|value| value.trim().parse::<u16>().ok())
        .map_or_else(Breaks::default, Breaks)
}

/// A style's `[spc]`: flags (1 single, 2 one and a half, 4 double, 8
/// custom; 16 "Add in: Always", 32 "When not at break"), the line height in
/// twips (stale for single spacing), a display unit, space above and below
/// in twips, a unit, and Text Tightness in percent (VM fixtures S1–S6).
/// Ami Pro's layout tests 1, then 2, then 4, and takes anything else as the
/// custom height; only "When not at break" keeps the space above off a
/// column's top, so a style with neither Add in bit adds it as Always does
/// (decompile). Missing or malformed values take Ami Pro's defaults.
fn style_spacing(sections: &[Section]) -> Spacing {
    let Some(section) = sections
        .iter()
        .find(|section| section.name.eq_ignore_ascii_case("spc"))
    else {
        return Spacing::default();
    };
    let value = |index: usize| {
        section
            .values
            .get(index)
            .and_then(|value| value.trim().parse::<i32>().ok())
            .filter(|value| (0..=1_000_000).contains(value))
    };
    let flags = value(0).unwrap_or(1);
    let height = value(1).filter(|height| *height > 0);
    // 1 1/2 and Double without a stored height derive it from the font (LS1).
    let line = if flags & 1 != 0 {
        LineSpacing::Single
    } else if flags & 2 != 0 {
        LineSpacing::OneAndAHalf(height.unwrap_or(0))
    } else if flags & 4 != 0 {
        LineSpacing::Double(height.unwrap_or(0))
    } else {
        height.map_or(LineSpacing::Single, LineSpacing::Custom)
    };
    Spacing {
        line,
        above: value(3).unwrap_or(0),
        below: value(4).unwrap_or(0),
        add_at_break: flags & 32 == 0,
        tightness: value(6)
            .filter(|percent| (50..=200).contains(percent))
            .map_or(100, |percent| percent as u16),
    }
}

/// A style's indents: `[algn]` values 3–5 are All, First, and Rest, and the
/// last of the values after `[nfmt]` (the style's own trailing values, which
/// the flat parse gives to `[nfmt]`) is the right indent (VM fixtures
/// I1–I3, AP31NAV's ruler). Missing or malformed values are 0.
fn style_indents(sections: &[Section]) -> Indents {
    let find = |name: &str| {
        sections
            .iter()
            .find(|section| section.name.eq_ignore_ascii_case(name))
    };
    let value = |section: Option<&Section>, index: usize| {
        section
            .and_then(|section| section.values.get(index))
            .and_then(|value| value.trim().parse::<i32>().ok())
            .filter(|value| (0..=1_000_000).contains(value))
            .unwrap_or(0)
    };
    let alignment = find("algn");
    let all = value(alignment, 2);
    // `[nfmt]` has six values of its own; then the next style's name, the
    // style's tab count and tabs, and the right indent last. Both sides
    // makes the right indent All whatever the record ends with (decompile).
    let right = if alignment_flags(sections) & 16 != 0 {
        all
    } else {
        find("nfmt")
            .filter(|section| section.values.len() >= 9)
            .map_or(0, |section| value(Some(section), section.values.len() - 1))
    };
    Indents {
        all,
        first: value(alignment, 3),
        rest: value(alignment, 4),
        right,
    }
}

fn collect_style_specs(
    legacy: &LegacyFile,
    output: &mut BTreeMap<String, StyleSpec>,
    origin: StyleOrigin,
) {
    for (index, section) in legacy.sections.iter().enumerate() {
        if !section.name.eq_ignore_ascii_case("tag") {
            continue;
        }
        let Some(name) = section
            .values
            .first()
            .filter(|name| !name.trim().is_empty())
        else {
            continue;
        };
        let end = legacy.sections[index + 1..]
            .iter()
            .position(|candidate| candidate.name.eq_ignore_ascii_case("tag"))
            .map_or(legacy.sections.len(), |offset| index + 1 + offset);
        let font = legacy.sections[index + 1..end]
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case("fnt"));
        let family = font
            .and_then(|section| section.values.first())
            .filter(|value| !value.is_empty())
            .cloned()
            // No corpus record lacks `[fnt]`; a malformed one takes Ami
            // Pro's default face, as a new document's Body Text does.
            .unwrap_or_else(|| "Times New Roman".into());
        let size = font
            .and_then(|section| section.values.get(1))
            .and_then(|value| value.parse::<f32>().ok())
            .map(|twentieths| twentieths / 20.0)
            .filter(|points| points.is_finite() && *points > 0.0)
            .unwrap_or(12.0);
        let number = |index: usize| {
            font.and_then(|section| section.values.get(index))
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(0)
        };
        let flags = number(3);
        let spec = StyleSpec {
            name: text::decode_name(name.trim()),
            font_family: family,
            font_size_points: size,
            bold: flags & 1 != 0,
            italic: flags & 2 != 0,
            spacing: style_spacing(&legacy.sections[index + 1..end]),
            indents: style_indents(&legacy.sections[index + 1..end]),
            breaks: style_breaks(&legacy.sections[index + 1..end]),
            alignment: style_alignment(&legacy.sections[index + 1..end]),
            attributes: style_attributes(number(2), flags),
            rules: style_rules(&legacy.sections[index + 1..end]),
            next: style_next(&legacy.sections[index + 1..end]),
            raw_next: legacy.sections[index + 1..end]
                .iter()
                .find(|section| section.name.eq_ignore_ascii_case("nfmt"))
                .and_then(|section| section.values.get(6))
                .map(|name| text::decode_name(name)),
            tabs: style_tabs(&legacy.sections[index + 1..end]),
            hanging: alignment_flags(&legacy.sections[index + 1..end]) & 512 != 0,
            hyphenation: alignment_flags(&legacy.sections[index + 1..end]) & 256 != 0,
            bullets: style_bullets(&legacy.sections[index + 1..end]),
            face_kind: style_face_kind(flags),
            origin,
            both_sides: alignment_flags(&legacy.sections[index + 1..end]) & 16 != 0,
            first_line_bold: flags & 4096 != 0,
            kept: style_kept(section, &legacy.sections[index + 1..end], flags),
        };
        output.insert(spec.name.to_lowercase(), spec);
    }
}

/// Finds the style sheet `name` as the importer does: beside the document
/// (when it has a path), in a `styles` directory there or one level up, then
/// in `folders` (Ami Pro's style-sheet path), the name in any letter case.
pub fn find_style_sheet(
    document: Option<&Path>,
    name: &str,
    folders: &[PathBuf],
) -> Option<PathBuf> {
    match document {
        Some(path) => resolve_style_path(path, name, folders),
        None => folders
            .iter()
            .find_map(|directory| entry_ignoring_case(directory, name)),
    }
}

/// A style sheet's description (`[desc]` value 1), as Ami Pro lists sheets
/// "by description"; `None` when the file has none or is not one.
pub fn style_sheet_description(bytes: &[u8]) -> Option<String> {
    let (legacy, _) = parse(bytes, LegacyKind::StyleSheet).ok()?;
    let description = section_value(&legacy, "desc")?;
    let description = text::decode_name(description.trim());
    (!description.is_empty()).then_some(description)
}

/// Whether a style sheet has contents File › New's With contents would
/// start a document from: text in its main body (not only codes or empty
/// styled paragraphs), or page frames (Save as a Style Sheet writes frames
/// only with contents).
pub fn style_sheet_has_contents(bytes: &[u8]) -> bool {
    let Ok(import) = import_sam(bytes, None, "sheet") else {
        return false;
    };
    let document = import.document;
    let written = |text: String| {
        text.chars()
            .any(|character| !character.is_whitespace() && !character.is_control())
    };
    written(document.plain_text())
        || document.frames().any(|frame| match frame.placement {
            // The layout's header and footer count when they hold text.
            amipro_document::Placement::Header { .. }
            | amipro_document::Placement::Footer { .. } => frame
                .owned_flows()
                .into_iter()
                .filter_map(|flow| document.flow(flow))
                .any(|flow| written(flow.plain_text())),
            _ => true,
        })
}

/// The macro a style sheet runs on File › New (`[newmac]`, written by Save
/// as a Style Sheet with Run macro); `None` when it runs none (CM2).
pub fn style_sheet_macro(bytes: &[u8]) -> Option<String> {
    let (legacy, _) = parse(bytes, LegacyKind::StyleSheet).ok()?;
    let name = section_value(&legacy, "newmac")?;
    let name = text::decode_name(name.trim());
    (!name.is_empty()).then_some(name)
}

fn resolve_style_path(
    sam_path: &Path,
    style_name: &str,
    style_directories: &[PathBuf],
) -> Option<PathBuf> {
    let parent = sam_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    // Ami Pro installs its style sheets in a STYLES directory beside the
    // documents' directory; copies of DOS trees keep the upper-case name,
    // and documents filed one folder deeper (DOCS\CLIENT) have it two
    // levels up (the user's corpus).
    let grandparent = parent.parent().and_then(Path::parent);
    let nearby = [Some(parent), parent.parent(), grandparent]
        .into_iter()
        .flatten()
        .filter_map(|directory| entry_ignoring_case(directory, "styles"));
    std::iter::once(parent.to_path_buf())
        .chain(nearby)
        .chain(style_directories.iter().cloned())
        .find_map(|directory| entry_ignoring_case(&directory, style_name))
}

/// The entry of `directory` named `name` in any letter case.
fn entry_ignoring_case(directory: &Path, name: &str) -> Option<PathBuf> {
    fs::read_dir(directory)
        .ok()?
        .flatten()
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(name)
        })
        .map(|entry| entry.path())
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{CharacterFormat, CharacterRun, Document, FaceKey};

    #[test]
    fn discovers_version_styles_and_unknown_data() {
        let sample = b"[ver]\r\n\t4\r\n[tag]\r\n\tBody Text\r\n\t2\r\n[mystery]\r\n\tkeep me\r\n";
        let (legacy, report) = parse(sample, LegacyKind::StyleSheet).unwrap();
        assert_eq!(legacy.format_version, Some(4));
        assert_eq!(legacy.style_names, ["Body Text"]);
        assert_eq!(report.unknown_sections, ["mystery"]);
        assert!(!report.safe_to_overwrite_original);
        assert_eq!(legacy.sections.last().unwrap().values, ["keep me"]);
    }

    #[test]
    fn rejects_unstructured_prefix() {
        assert_eq!(
            parse(b"oops\n[ver]\n4", LegacyKind::Document),
            Err(ParseError::DataBeforeSection(1))
        );
    }

    #[test]
    fn decodes_windows_1252_without_dropping_bytes() {
        let sample = b"[ver]\n4\n[desc]\nAmi Pro \x93document\x94 \x97 test\n";
        let (legacy, report) = parse(sample, LegacyKind::Document).unwrap();
        assert_eq!(legacy.sections[1].values, ["Ami Pro “document” — test"]);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("Windows-1252"))
        );
    }

    #[test]
    fn imports_sam_body_and_embedded_paragraph_styles() {
        let sample = b"[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t200\r\n\t\t0\r\n\t\t16384\r\n[tag]\r\n\tHeading\r\n\t3\r\n\t[fnt]\r\n\t\tArial\r\n\t\t280\r\n\t\t0\r\n\t\t16385\r\n[edoc]\r\n@Heading@<:s>Release Notes\r\n\r\n@Body Text@A wrapped \r\nline with <+!>formatting<-!>.\r\n\r\n>\r\n[Embedded]\r\n00000000\r\n";

        let imported = import_sam(sample, None, "Readme").unwrap();

        assert_eq!(imported.document.title, "Readme");
        assert_eq!(
            imported.document.plain_text(),
            "Release Notes\nA wrapped line with formatting."
        );
        assert_eq!(imported.document.paragraphs().len(), 2);
        let heading = &imported.document.styles()[&imported.document.paragraphs()[0].style];
        assert_eq!(heading.name, "Heading");
        assert_eq!(heading.font_size_points, 14.0);
        assert!(heading.bold);
        assert_eq!(
            imported.document.paragraphs()[1].runs,
            [CharacterRun {
                start: 20,
                end: 30,
                format: CharacterFormat {
                    bold: Some(true),
                    ..CharacterFormat::default()
                },
            }]
        );
        assert!(!imported.report.unsupported_controls.contains(&"+!".into()));
        assert!(!imported.report.unsupported_controls.contains(&"-!".into()));
    }

    #[test]
    fn external_style_sheet_supplies_named_style() {
        let sam = b"[ver]\n4\n[sty]\nCUSTOM.STY\n[edoc]\n@Special@Hello\n\n>\n";
        let sty = b"[ver]\n4\n[tag]\nSpecial\n7\n[fnt]\nDejaVu Serif\n360\n0\n2\n[edoc]\n>\n";

        let imported = import_sam(sam, Some(sty), "External").unwrap();
        let style = &imported.document.styles()[&imported.document.paragraphs()[0].style];
        assert_eq!(style.name, "Special");
        assert_eq!(style.font_size_points, 18.0);
        assert!(style.italic);
        assert!(imported.report.associated_style_resolved);
    }

    #[test]
    fn imports_nested_bold_and_italic_character_runs() {
        let sam = b"[ver]\n4\n[edoc]\nPlain <+!>bold <+\">both<-\"> bold<-!> plain\n\n>\n";
        let imported = import_sam(sam, None, "Runs").unwrap();
        let paragraph = &imported.document.paragraphs()[0];

        assert_eq!(paragraph.text, "Plain bold both bold plain");
        assert_eq!(paragraph.runs.len(), 3);
        assert_eq!((paragraph.runs[0].start, paragraph.runs[0].end), (6, 11));
        assert_eq!(paragraph.runs[0].format.bold, Some(true));
        assert_eq!(paragraph.runs[0].format.italic, None);
        assert_eq!((paragraph.runs[1].start, paragraph.runs[1].end), (11, 15));
        assert_eq!(paragraph.runs[1].format.bold, Some(true));
        assert_eq!(paragraph.runs[1].format.italic, Some(true));
        assert_eq!((paragraph.runs[2].start, paragraph.runs[2].end), (15, 20));
        assert_eq!(paragraph.runs[2].format.bold, Some(true));
        assert_eq!(paragraph.runs[2].format.italic, None);
        assert!(imported.report.unsupported_controls.is_empty());
    }

    #[test]
    fn imported_paragraph_styles_survive_native_save_and_reopen() {
        let sam = b"[ver]\n4\n[tag]\nBody Text\n2\n[fnt]\nArial\n200\n0\n0\n[tag]\nHeading\n3\n[fnt]\nArial\n280\n0\n1\n[edoc]\n@Heading@Title\n\n@Body Text@Body\n\n>\n";
        let imported = import_sam(sam, None, "Round trip").unwrap();

        let native = imported.document.encode_native().unwrap();
        let reopened = Document::decode_native(&native).unwrap();

        assert_eq!(reopened, imported.document);
        assert_eq!(
            reopened.styles()[&reopened.paragraphs()[0].style].name,
            "Heading"
        );
        assert_eq!(
            reopened.styles()[&reopened.paragraphs()[1].style].name,
            "Body Text"
        );
    }

    /// Mirrors a document Ami Pro 3.1 saved with `_TOC.STY`, where Body Text is
    /// neither the first style nor assigned a function key: untagged
    /// paragraphs are Body Text, and a tag applies to its own paragraph only.
    #[test]
    fn untagged_paragraphs_are_body_text_wherever_the_sheet_defines_it() {
        let sam = b"[ver]\n4\n[tag]\nTOC1\n2\n[fnt]\nArial\n280\n0\n1\n\
            [tag]\nBody Text\n0\n[fnt]\nArial\n240\n0\n0\n\
            [edoc]\n@TOC1@Contents\n\nFirst body.\n\nSecond body.\n\n@TOC1@More\n\nLast body.\n\n>\n";
        let imported = import_sam(sam, None, "Body").unwrap();
        let names: Vec<_> = imported
            .document
            .paragraphs()
            .iter()
            .map(|paragraph| imported.document.styles()[&paragraph.style].name.as_str())
            .collect();

        assert_eq!(
            names,
            ["TOC1", "Body Text", "Body Text", "TOC1", "Body Text"]
        );
        let body = imported
            .document
            .styles()
            .values()
            .find(|style| style.name == "Body Text")
            .unwrap();
        assert_eq!(body.font_size_points, 12.0);
        assert!(!body.bold);
    }

    #[test]
    fn untagged_paragraphs_get_one_body_text_style_without_definitions() {
        let sam = b"[ver]\n4\n[sty]\nMISSING.STY\n[edoc]\n@Heading@Title\n\nBody one.\n\nBody two.\n\n>\n";
        let imported = import_sam(sam, None, "Missing").unwrap();
        let document = &imported.document;
        let body_styles: Vec<_> = document
            .styles()
            .values()
            .filter(|style| style.name == "Body Text")
            .collect();

        assert_eq!(body_styles.len(), 1);
        assert_eq!(
            document.styles()[&document.paragraphs()[0].style].name,
            "Heading"
        );
        assert_eq!(document.paragraphs()[1].style, body_styles[0].id);
        assert_eq!(document.paragraphs()[2].style, body_styles[0].id);
    }

    /// Escapes and character codes as Ami Pro 3.1 wrote them for typed text.
    #[test]
    fn decodes_verified_escapes_and_high_characters() {
        let sam = b"[ver]\n4\n[edoc]\n@@not a tag@ start.\n\n\
            Beta <[>bracket] <<angle<;> back\\slash.\n\n\
            Ami</`>Pro Together</n> caf<\\i> </@> <\\\\> </\x7f> <\\\x7f>\n\n\
            Tab\tafter <+!>b</n>ld<-!>.\n\n\
            <:#284,9360><+!>@@lead<-!> mid@@dle\n\n@Body Text@@@tagged\n\n@@@three\n\n>\n";
        let imported = import_sam(sam, None, "Escapes").unwrap();
        let paragraphs = &imported.document.paragraphs();

        assert_eq!(paragraphs[0].text, "@not a tag@ start.");
        assert_eq!(
            imported.document.styles()[&paragraphs[0].style].name,
            "Body Text"
        );
        assert_eq!(paragraphs[1].text, "Beta [bracket] <angle> back\\slash.");
        assert_eq!(
            paragraphs[2].text,
            "Ami\u{a0}Pro Together\u{ae} caf\u{e9} \u{20ac} \u{dc} \u{bf} \u{ff}"
        );
        assert_eq!(paragraphs[3].text, "Tab\tafter b\u{ae}ld.");
        assert_eq!(
            (paragraphs[3].runs[0].start, paragraphs[3].runs[0].end),
            (10, 15)
        );
        assert_eq!(paragraphs[4].text, "@lead mid@@dle");
        assert_eq!(paragraphs[5].text, "@tagged");
        assert_eq!(paragraphs[6].text, "@@three");
        // `<:#…>` is a layout cache: dropped, not reported or kept.
        assert!(imported.report.unsupported_controls.is_empty());
        assert_eq!(imported.report.structure.dropped_caches, 1);
    }

    /// Forms Ami Pro 3.1 wrote for a table, an anchored frame, and a floating
    /// header (VM fixtures TB1, F07, G22). `<:t n>` anchors table `n`; it is
    /// not a tab, which Ami Pro writes as a literal tab character.
    #[test]
    fn table_and_frame_anchors_and_floating_headers_are_not_text() {
        let sam = b"[ver]\n4\n[edoc]\nBefore.<:A0>\n\n<:t0>\n\nTab\there.<:H<*->\n\
            Floating header.\n\n>\n\n>\n";
        let imported = import_sam(sam, None, "Anchors").unwrap();
        let texts: Vec<_> = imported
            .document
            .paragraphs()
            .iter()
            .map(|paragraph| paragraph.text.as_str())
            .collect();

        // There are no frame records, so the anchors name nothing and stay as
        // legacy marks; the floating header becomes an object with its flow.
        assert_eq!(texts, ["Before.", "", "Tab\there.\u{FFFC}"]);
        let paragraphs = imported.document.paragraphs();
        assert_eq!(paragraphs[0].marks[0].raw, "<:A0>");
        assert_eq!(paragraphs[1].marks[0].raw, "<:t0>");
        assert_eq!(imported.report.structure.anchors_unresolved, 2);
        let amipro_document::InlineObject::FloatingHeaderFooter { kind, pages, flow } =
            &paragraphs[2].objects[0]
        else {
            panic!("expected a floating header");
        };
        assert_eq!(*kind, amipro_document::HeaderFooterKind::Header);
        assert_eq!(*pages, amipro_document::PageSides::All);
        assert_eq!(
            imported.document.flow(*flow).unwrap().plain_text(),
            "Floating header."
        );
    }

    /// Notes span lines and contain their own blank lines and closing `>`
    /// line; neither may end the paragraph or the document body.
    #[test]
    fn multi_line_notes_do_not_end_or_split_the_body() {
        let sam = b"[ver]\n4\n[edoc]\nBefore the note.\n<:N1,AB,32768,1,1\nNote text.\n\n\nMore note.\n\n>\n\n\
            @Heading@After<:h<*.>\n@Body Text@\n>\n\nLast.\n\n>\n\n[Embedded]\n";
        let imported = import_sam(sam, None, "Notes").unwrap();
        let texts: Vec<_> = imported
            .document
            .paragraphs()
            .iter()
            .map(|paragraph| paragraph.text.as_str())
            .collect();

        assert_eq!(
            texts,
            ["Before the note.\u{FFFC}", "After\u{FFFC}", "Last."]
        );
        // The note is an object holding its own paragraphs (an empty one
        // between the two, as Ami Pro's blank lines give it).
        let amipro_document::InlineObject::Note { note, flow } =
            &imported.document.paragraphs()[0].objects[0]
        else {
            panic!("expected a note");
        };
        assert_eq!(note.time, 1);
        assert_eq!(note.initials.as_deref(), Some("AB"));
        assert_eq!(note.colour, Some(32768));
        assert_eq!(note.number, Some(1));
        let texts: Vec<_> = imported
            .document
            .flow(*flow)
            .unwrap()
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.text.clone())
            .collect();
        assert_eq!(texts, ["Note text.", "", "More note."]);
        assert!(imported.report.unsupported_controls.is_empty());
    }

    #[test]
    fn parser_stops_before_embedded_binary_payload() {
        let sample = b"[ver]\n4\n[edoc]\nText\n\0[not a valid section\xff";
        let (legacy, _) = parse(sample, LegacyKind::Document).unwrap();
        assert_eq!(legacy.sections.last().unwrap().name, "edoc");
        assert_eq!(legacy.sections.last().unwrap().values, ["Text"]);
    }

    #[test]
    fn style_indents_come_from_algn_and_the_last_trailing_value() {
        // I1: All 0.50, First 1.00, Rest 0.25, From right 1.50 in; a second
        // style has no trailing values after [nfmt] (1,062 corpus records).
        let sample = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n\
            \t[algn]\n\t\t1\n\t\t1\n\t\t720\n\t\t1440\n\t\t360\n\t[spc]\n\t\t33\n\t\t273\n\t\t1\n\t\t0\n\t\t0\n\t\t1\n\t\t100\n\
            \t[nfmt]\n\t\t280\n\t\t1\n\t\t2\n\t\t.\n\t\t,\n\t\t$\n\tBody Text\n\t0\n\t2160\n\
            [tag]\n\tBullet\n\t0\n\t[fnt]\n\t\tArial\n\t\t200\n\t\t0\n\t\t16384\n\t[algn]\n\t\t1\n\t\t1\n\t\t0\n\t\t288\n\t\t288\n\
            \t[nfmt]\n\t\t280\n\t\t1\n\t\t2\n\t\t.\n\t\t,\n\t\t$\n\
            [edoc]\nText\n\n@Bullet@Item\n\n>\n";
        let imported = import_sam(sample, None, "Indents").unwrap();
        let indents = |name: &str| {
            imported
                .document
                .styles()
                .values()
                .find(|style| style.name == name)
                .unwrap()
                .indents
        };
        assert_eq!(
            indents("Body Text"),
            Indents {
                all: 720,
                first: 1_440,
                rest: 360,
                right: 2_160
            }
        );
        assert_eq!(
            indents("Bullet"),
            Indents {
                all: 0,
                first: 288,
                rest: 288,
                right: 0
            }
        );
        // A style the file does not define borrows Body Text's font, not its
        // indents.
        let unknown = import_sam(
            &[&sample[..sample.len() - 2], b"@Mystery@Unknown\n\n>\n"].concat(),
            None,
            "Indents",
        )
        .unwrap();
        let mystery = unknown
            .document
            .styles()
            .values()
            .find(|style| style.name == "Mystery")
            .unwrap();
        assert!(mystery.indents.is_zero());
        assert_eq!(mystery.font_family, "Times New Roman");
    }

    #[test]
    fn styles_carry_their_colour_and_character_attributes() {
        // `[fnt]` value 3 is the colour, value 4 the flags (decompile).
        let sample = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n\
            [tag]\n\tTitle\n\t0\n\t[fnt]\n\t\tArial\n\t\t360\n\t\t255\n\t\t16405\n\
            [tag]\n\tEffects\n\t0\n\t[fnt]\n\t\tArial\n\t\t240\n\t\t33488896\n\t\t62664\n\
            [tag]\n\tCaps\n\t0\n\t[fnt]\n\t\tArial\n\t\t240\n\t\t0\n\t\t19248\n\
            [edoc]\n@Title@Upper and underlined\n\n>\n";
        let imported = import_sam(sample, None, "Attributes").unwrap();
        let style = |name: &str| {
            imported
                .document
                .styles()
                .values()
                .find(|style| style.name == name)
                .unwrap()
                .clone()
        };
        assert!(style("Body Text").attributes.is_default());
        let title = style("Title");
        assert!(title.bold);
        assert_eq!(
            title.attributes,
            CharacterAttributes {
                underline: true,
                caps: Some(Caps::Upper),
                colour: 0x0000_00FF,
                ..CharacterAttributes::default()
            }
        );
        // 62664 = 8 + 64 + 128 + 1024 + 4096 + 0x2000 + 0xC000: word and
        // double underline, strikethrough, lower case, 1st line bold (not
        // protected text: FIRST_LINE_BOLD_NOTES.md), the bit Ami Pro drops,
        // and the face's kind; the colour's top byte is not part of the RGB
        // value.
        assert_eq!(
            style("Effects").attributes,
            CharacterAttributes {
                word_underline: true,
                double_underline: true,
                strikethrough: true,
                caps: Some(Caps::Lower),
                colour: 0x00FF_0000,
                ..CharacterAttributes::default()
            }
        );
        assert!(
            imported
                .document
                .styles()
                .values()
                .any(|s| s.name == "Effects" && s.first_line_bold)
        );
        // 19248 = 16 + 32 + 256 + 512 + 2048 + 0x4000: small caps wins over
        // upper and initial caps; both scripts are kept.
        let caps = style("Caps").attributes;
        assert_eq!(caps.caps, Some(Caps::Small));
        assert!(caps.superscript && caps.subscript);
        let decoded = Document::decode_native(&imported.document.encode_native().unwrap()).unwrap();
        assert_eq!(decoded, imported.document);
    }

    #[test]
    fn next_styles_come_from_the_name_after_nfmt() {
        let style = |name: &str, brk: u16, next: &str| {
            format!(
                "[tag]\n\t{name}\n\t0\n\t[fnt]\n\t\tArial\n\t\t240\n\t\t0\n\t\t16384\n\t[brk]\n\t\t{brk}\n\
                 \t[nfmt]\n\t\t280\n\t\t1\n\t\t2\n\t\t.\n\t\t,\n\t\t$\n\t{next}\n\t0\n\t0\n"
            )
        };
        let sample = format!(
            "[ver]\n4\n{}{}{}{}[edoc]\nText\n\n>\n",
            style("Body Text", 4, "Body Text"),
            style("Head", 68, "Body Text"),
            style("Self", 68, "Self"),
            style("Lost", 68, "Nowhere"),
        );
        let imported = import_sam(sample.as_bytes(), None, "Next").unwrap();
        let next = |name: &str| {
            let styles = imported.document.styles();
            let style = styles.values().find(|style| style.name == name).unwrap();
            (
                style.breaks.next_style(),
                style.next.map(|id| styles[&id].name.clone()),
            )
        };
        assert_eq!(next("Head"), (true, Some("Body Text".to_owned())));
        assert_eq!(next("Self"), (true, None), "naming itself");
        assert_eq!(next("Lost"), (true, None), "a style no sheet defines");
        assert_eq!(next("Body Text"), (false, None));
    }

    #[test]
    fn style_rules_come_from_line_nx1_nx2() {
        // Modify Style › Lines (VM fixtures NX1, NX2): flags 1 above, 2
        // below, 4 Text, 8 Margins, neither Other; values 6 and 7 the line
        // styles, 8 and 9 the space each takes.
        let sample = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n\
            \t[line]\n\t\t8\n\t\t0\n\t\t1\n\t\t0\n\t\t1\n\t\t1\n\t\t1\n\t\t10\n\t\t10\n\t\t1\n\
            [tag]\n\tRuleB\n\t0\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n\
            \t[line]\n\t\t6\n\t\t0\n\t\t1\n\t\t0\n\t\t1\n\t\t1\n\t\t5\n\t\t10\n\t\t300\n\t\t1\n\
            [tag]\n\tRuleC\n\t0\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n\
            \t[line]\n\t\t19\n\t\t2880\n\t\t1\n\t\t255\n\t\t1\n\t\t8\n\t\t11\n\t\t150\n\t\t250\n\t\t2\n\
            [edoc]\nText\n\n>\n";
        let imported = import_sam(sample, None, "Rules").unwrap();
        let rules = |name: &str| {
            imported
                .document
                .styles()
                .values()
                .find(|style| style.name == name)
                .unwrap()
                .rules
        };
        assert_eq!(rules("Body Text"), ParagraphRules::default());
        let below = rules("RuleB");
        assert_eq!((below.length, below.space()), (RuleLength::Text, (0, 300)));
        assert_eq!(below.below.line, 5);
        let both = rules("RuleC");
        assert_eq!(both.length, RuleLength::Other);
        assert_eq!((both.other_length, both.colour, both.unit), (2880, 255, 2));
        assert_eq!(both.space(), (150, 250));
        assert_eq!((both.above.line, both.below.line), (8, 11));
        assert_eq!(both.other_flags, 16);
        assert_eq!(both.flags(), 19, "the flag word is kept");
    }

    #[test]
    fn line_heights_are_calibrated_from_one_line_caches() {
        // DECISIONS.md, "Calibrated line heights": single-spaced one-line
        // paragraphs set in one font vote for its height; then one-line
        // paragraphs that mix known fonts with one other give it theirs when
        // taller than the known ones.
        let sample = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n\
            [tag]\n\tHeading\n\t0\n\t[fnt]\n\t\tArial\n\t\t280\n\t\t0\n\t\t16385\n\
            [tag]\n\tLoose\n\t0\n\t[fnt]\n\t\tCourier New\n\t\t200\n\t\t0\n\t\t32768\n\
            \t[spc]\n\t\t34\n\t\t0\n\t\t1\n\t\t0\n\t\t0\n\t\t1\n\t\t100\n\
            [tag]\n\tRuled\n\t0\n\t[fnt]\n\t\tArial\n\t\t240\n\t\t0\n\t\t16384\n\
            \t[line]\n\t\t9\n\t\t0\n\t\t1\n\t\t12632256\n\t\t0\n\t\t6\n\t\t12\n\t\t160\n\t\t100\n\t\t2\n\
            [edoc]\n<:#284,9360>One line.\n\n<:#284,9360><:s>Another line.\n\n<:#278,9360>Odd one.\n\n\
            <:#568,9360>Two lines of text that wrapped.\n\n<:#300,9360>With <+!>bold<-!> in it.\n\n\
            <:#284,9360>With <+\">italic<-\"> no taller.\n\n\
            <:#284,9360><+#>Underlined<-#> only.\n\n<:#240,9360><:f200,,>\n\n\
            <:#249,9360><:f220,BHelv,>All in Helv<:f>\n\n\
            @Heading@<:#332,9360>Heading\n\n@Heading@<:#324,9360>Heading\n\n\
            @Loose@<:#344,9360>One and a half\n\n@Ruled@<:#437,9360>Ruled above\n\n>\n";
        let imported = import_sam(sample, None, "Heights").unwrap();
        let heights = imported.document.line_heights();
        assert_eq!(
            heights.get(&FaceKey::new("Times New Roman", 12.0, false, false)),
            Some(284),
            "the most frequent one-line height; marks and runs that change no font count"
        );
        assert_eq!(
            heights.get(&FaceKey::new("Times New Roman", 12.0, true, false)),
            Some(300),
            "a mixed line taller than its known font"
        );
        assert_eq!(
            heights.get(&FaceKey::new("Times New Roman", 12.0, false, true)),
            None,
            "a mixed line no taller than its known font says nothing"
        );
        assert_eq!(
            heights.get(&FaceKey::new("Times New Roman", 10.0, false, false)),
            Some(240),
            "an empty paragraph's line is its end format's font"
        );
        assert_eq!(
            heights.get(&FaceKey::new("Helv", 11.0, false, false)),
            Some(249),
            "a paragraph set wholly in a font change"
        );
        assert_eq!(
            heights.get(&FaceKey::new("Arial", 14.0, true, false)),
            Some(324),
            "a tie goes to the smaller height; bold is its own font"
        );
        assert_eq!(
            heights.get(&FaceKey::new("Arial", 14.0, false, false)),
            None
        );
        assert_eq!(
            heights.get(&FaceKey::new("Courier New", 10.0, false, false)),
            None,
            "1 1/2 spacing does not vote"
        );
        assert_eq!(
            heights.get(&FaceKey::new("Arial", 12.0, false, false)),
            Some(277),
            "without the 160 twips a rule above adds (only flag 1 is set)"
        );
        assert_eq!(heights.len(), 6);
        assert_eq!(imported.report.structure.calibrated_line_heights, 6);
        // The native file keeps them.
        let encoded = imported.document.encode_native().unwrap();
        let decoded = Document::decode_native(&encoded).unwrap();
        assert_eq!(decoded.line_heights(), heights);
    }

    #[test]
    fn styles_know_where_they_are_stored_and_sheets_read_alone_st1() {
        // ST1: the SAM's own [tag]s are stored in the document, the STY's
        // in the sheet, and a name defined nowhere is missing; a SAM
        // without [lay] takes the sheet's layout.
        let sheet =
            b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tArial\n\t\t200\n\t\t0\n\t\t16384\n\
            [tag]\n\tTitle\n\t0\n\t[fnt]\n\t\tArial\n\t\t360\n\t\t0\n\t\t16385\n[edoc]\n>\n";
        let sam = b"[ver]\n4\n[sty]\n\t_DEFAULT.STY\n[tag]\n\tMine\n\t0\n\t[fnt]\n\t\tCourier New\n\t\t240\n\t\t0\n\t\t32768\n\
            [edoc]\n@Title@a\n\n@Mine@b\n\n@Ghost@c\n\nd\n\n>\n";
        let imported = import_sam(sam, Some(sheet), "ST1").unwrap();
        let origin = |name: &str| {
            imported
                .document
                .styles()
                .values()
                .find(|style| style.name == name)
                .unwrap()
                .origin
        };
        assert_eq!(origin("Mine"), StyleOrigin::Document);
        assert_eq!(origin("Title"), StyleOrigin::Sheet);
        assert_eq!(origin("Body Text"), StyleOrigin::Sheet);
        assert_eq!(origin("Ghost"), StyleOrigin::Missing);
        assert_eq!(
            imported.document.style_sheet(),
            Some(&StyleSheetLink {
                name: "_DEFAULT.STY".into(),
                layout_from_sheet: true,
            })
        );
        let read = read_style_sheet(sheet, "_basic.sty").unwrap();
        assert_eq!(read.name, "_basic.sty");
        assert_eq!(
            read.styles
                .iter()
                .map(|style| (style.name.as_str(), style.origin))
                .collect::<Vec<_>>(),
            [
                ("Body Text", StyleOrigin::Sheet),
                ("Title", StyleOrigin::Sheet)
            ]
        );
        assert!(read.layout.is_some());
        // Hyphenation: `[algn]` 256 in a style, the zone from `[docopts]`.
        let hyphenated = b"[ver]\n4\n[docopts]\n\t3\n\t2\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tArial\n\t\t200\n\t\t0\n\t\t16384\n\
            \t[algn]\n\t\t257\n\t\t1\n\t\t0\n\t\t0\n\t\t0\n[edoc]\ntext\n\n>\n";
        let imported = import_sam(hyphenated, None, "HY").unwrap();
        assert_eq!(imported.document.hyphenation_zone(), 3);
        assert!(
            imported
                .document
                .styles()
                .values()
                .all(|style| style.hyphenation)
        );
        assert_eq!(
            import_sam(sam, Some(sheet), "ST1")
                .unwrap()
                .document
                .hyphenation_zone(),
            5
        );
        // A sheet without a body section still reads; files without
        // styles, Word Pro files, and protected files are refused.
        let bodiless = &sheet[..sheet.len() - b"[edoc]\n>\n".len()];
        assert_eq!(read_style_sheet(bodiless, "x.sty").unwrap().styles.len(), 2);
        let error = read_style_sheet(b"[ver]\n4\n[edoc]\ntext\n\n>\n", "x.sty").unwrap_err();
        assert!(error.to_string().contains("not an Ami Pro style sheet"));
        assert!(read_style_sheet(b"WordPro\0\x01", "x.sty").is_err());
        assert!(read_style_sheet(b"[encrypt]\n\xf6", "x.sty").is_err());
    }

    /// Protected text and no hyphenation in the middle of a paragraph of a
    /// style without them: the off marks read back as the style's, so the
    /// document reads back equal and Save As lists nothing.
    #[test]
    fn ended_attributes_in_mid_paragraph_read_back_as_the_style() {
        let mut document = Document::from_plain_text("Keep this word safe.");
        let paragraph = document.main_flow_mut().paragraph_mut(0).unwrap();
        paragraph.runs.push(amipro_document::CharacterRun {
            start: 5,
            end: 9,
            format: amipro_document::CharacterFormat {
                protected: Some(true),
                no_hyphenation: Some(true),
                strikethrough: Some(true),
                ..amipro_document::CharacterFormat::PLAIN
            },
        });
        let options = SamOptions::at(0);
        let written = String::from_utf8_lossy(&sam_document(&document, &options)).into_owned();
        assert!(written.contains("<-*>") && written.contains("<-.>") && written.contains("<-%>"));
        let read = read_back(&document, &options).unwrap();
        let runs = &read.main_flow().paragraphs[0].runs;
        assert_eq!(runs.len(), 1, "{runs:?}");
        assert_eq!((runs[0].start, runs[0].end), (5, 9));
        assert!(read.main_flow().paragraphs[0].end_format.is_empty());
        assert_eq!(sam_losses(&document, &options).unwrap(), []);
        // An off mark with nothing on before it, as Ami Pro's files have
        // them, stays as read, so that it is written back.
        let redundant = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n[edoc]\n<-*>Plain text\n\n>\n";
        let read = import_sam(redundant, None, "R").unwrap().document;
        assert_eq!(
            read.main_flow().paragraphs[0].format_at(2).protected,
            Some(false)
        );
    }

    #[test]
    fn a_style_sheets_macro_is_read_from_newmac() {
        let with = b"[ver]\n4\n[newmac]\n\tLETTER.SMM\n[tag]\n\tBody Text\n\t2\n[edoc]\n>\n";
        assert_eq!(style_sheet_macro(with).as_deref(), Some("LETTER.SMM"));
        let without = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n[edoc]\n>\n";
        assert_eq!(style_sheet_macro(without), None);
        let empty = b"[ver]\n4\n[newmac]\n\t\n[tag]\n\tBody Text\n\t2\n[edoc]\n>\n";
        assert_eq!(style_sheet_macro(empty), None);
        assert_eq!(style_sheet_macro(b"WordPro\0\x01"), None);
        assert!(!style_sheet_has_contents(without));
        let contents = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n[edoc]\nDear Sir,\n\n>\n";
        assert!(style_sheet_has_contents(contents));
        assert!(!style_sheet_has_contents(b"WordPro\0\x01"));
    }

    #[test]
    fn face_kinds_come_from_style_bits_and_font_changes() {
        // `[fnt]`'s kind bits (decompile) and each `<:f…>`'s K − 0x20: the
        // commonest byte that names a family wins over "don't care".
        let sample = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n\
            [tag]\n\tCode\n\t0\n\t[fnt]\n\t\tCourier New\n\t\t240\n\t\t0\n\t\t32768\n\
            [edoc]\n<:f240,\"Haettenschweiler,0,0,0>a<:f> <:f240,BHaettenschweiler,0,0,0>b<:f>\
            <:f240,\"Haettenschweiler,0,0,0>c<:f> <:f240,rZapfDingbats,0,0,0>d<:f>\n\n>\n";
        let imported = import_sam(sample, None, "Kinds").unwrap();
        assert_eq!(
            imported.document.face_kinds(),
            &BTreeMap::from([
                ("courier new".to_owned(), 0x31),
                ("haettenschweiler".to_owned(), 0x22),
                ("times new roman".to_owned(), 0x12),
                ("zapfdingbats".to_owned(), 0x52),
            ])
        );
    }

    #[test]
    fn style_bullets_come_from_spec_and_empty_fields_keep_their_places() {
        // VM fixtures BN1-BN3: `[spec]`'s ten values; `<*X>` is code
        // X - 0x20, `<*>>` (0x1E, lower-case letters) included. Ami Pro
        // writes an empty field as an indented blank line, here `[spec]`'s
        // text and `[nfmt]`'s currency symbol: the values after it keep
        // their places (the right indent stays 1440).
        let sample = b"[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tArial\n\t\t240\n\t\t0\n\t\t16384\n\
            \t[spec]\n\t\t0\n\t\t0\n\t\t\n\t\t0\n\t\t1\n\t\t1\n\t\t0\n\t\t0\n\t\t0\n\t\t0\n\
            \t[nfmt]\n\t\t280\n\t\t1\n\t\t2\n\t\t.\n\t\t,\n\t\t\n\tBody Text\n\t0\n\t1440\n\
            [tag]\n\tOutline 5\n\t8\n\t[fnt]\n\t\tArial\n\t\t240\n\t\t0\n\t\t16384\n\
            \t[spec]\n\t\t0\n\t\t5\n\t\t<*>>.\n\t\t28948\n\t\t1\n\t\t1\n\t\t1\n\t\t2\n\t\t0\n\t\t0\n\
            [tag]\n\tNote\n\t0\n\t[fnt]\n\t\tArial\n\t\t240\n\t\t0\n\t\t16384\n\
            \t[spec]\n\t\t0\n\t\t0\n\t\t<*1>Note<<1<;>:\n\t\t1008\n\t\t1\n\t\t1\n\t\t16\n\t\t16\n\t\t0\n\t\t0\n\
            [edoc]\nText\n\n>\n";
        let imported = import_sam(sample, None, "Bullets").unwrap();
        let style = |name: &str| {
            imported
                .document
                .styles()
                .values()
                .find(|style| style.name == name)
                .unwrap()
                .clone()
        };
        let body = style("Body Text");
        assert_eq!(body.bullets, Bullets::default());
        assert_eq!(body.indents.right, 1440);
        let outline = style("Outline 5").bullets;
        assert_eq!(
            outline,
            Bullets {
                text: "\u{1E}.".into(),
                level: 5,
                attributes: 1,
                flags: 2,
                kept: [0, 28948, 1, 1, 0, 0],
            }
        );
        assert!(outline.numbered());
        let note = style("Note").bullets;
        assert_eq!(note.text, "\u{11}Note<1>:");
        assert_eq!((note.attributes, note.flags), (16, 16));
        assert!(!note.numbered());
    }

    #[test]
    fn style_tabs_precede_the_right_indent_and_both_sides_copies_all() {
        // Decompile: after the next style's name come the style's tab count
        // and type/position pairs, then the right indent; with Both sides
        // (16) the right indent is All whatever the record ends with.
        let sample = b"[ver]
4
[tag]
	Body Text
	2
	[fnt]
		Arial
		240
		0
		16384
            	[algn]
		1
		1
		360
		0
		0
            	[nfmt]
		280
		1
		2
		.
		,
		$
	Body Text
	2
	1 720
	3 9360
	1440
            [tag]
	Quote
	0
	[fnt]
		Arial
		240
		0
		16384
            	[algn]
		17
		1
		720
		0
		0
            	[nfmt]
		280
		1
		2
		.
		,
		$
	Quote
	0
	0
            [edoc]
Text

>
";
        let imported = import_sam(sample, None, "Tabs").unwrap();
        let right = |name: &str| {
            imported
                .document
                .styles()
                .values()
                .find(|style| style.name == name)
                .unwrap()
                .indents
                .right
        };
        assert_eq!(right("Body Text"), 1_440);
        assert_eq!(right("Quote"), 720);
    }

    #[test]
    fn style_alignment_comes_from_algn() {
        // I1–I3 and O2: flags 1, 2, 4, 8; 16 (Both sides), 256, and 512
        // (Hanging indent) are other settings.
        let alignment = |flags: &str| {
            let sample = format!(
                "[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tArial\n\t\t240\n\t\t0\n\t\t16384\n\
                 \t[algn]\n\t\t{flags}\n\t\t1\n\t\t0\n\t\t0\n\t\t0\n[edoc]\nText\n\n>\n"
            );
            let imported = import_sam(sample.as_bytes(), None, "Alignment").unwrap();
            imported
                .document
                .styles()
                .values()
                .next()
                .unwrap()
                .alignment
        };
        assert_eq!(alignment("1"), Alignment::Left);
        assert_eq!(alignment("2"), Alignment::Right);
        assert_eq!(alignment("260"), Alignment::Centre);
        assert_eq!(alignment("536"), Alignment::Justify);
        assert_eq!(alignment("513"), Alignment::Left);
        // Several alignment bits: right, then centre, then justified
        // (decompile).
        assert_eq!(alignment("6"), Alignment::Right);
        assert_eq!(alignment("12"), Alignment::Centre);
        assert_eq!(alignment("9"), Alignment::Justify);
    }

    #[test]
    fn style_spacing_comes_from_spc() {
        // VM fixtures S1–S6: flags (line spacing and Add in), line height,
        // unit, above, below, unit, tightness.
        let style = |spc: &str| {
            let sample = format!(
                "[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n\
                 \t[spc]\n{}\n[edoc]\nText\n\n>\n",
                spc.split(' ')
                    .map(|value| format!("\t\t{value}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            let imported = import_sam(sample.as_bytes(), None, "Spacing").unwrap();
            imported.document.styles().values().next().unwrap().spacing
        };
        assert_eq!(
            style("17 288 1 144 288 1 90"),
            Spacing {
                line: LineSpacing::Single,
                above: 144,
                below: 288,
                add_at_break: true,
                tightness: 90
            }
        );
        assert_eq!(
            style("34 432 1 0 0 1 100").line,
            LineSpacing::OneAndAHalf(432)
        );
        assert_eq!(style("36 561 1 0 0 1 100").line, LineSpacing::Double(561));
        let custom = style("40 360 1 0 0 1 100");
        assert_eq!(custom.line, LineSpacing::Custom(360));
        assert!(!custom.add_at_break, "32 is When not at break");
        assert_eq!(style("40 0 1 0 0 1 100").line, LineSpacing::Single);
        // 1 1/2 and Double without a height derive it from the font (LS1).
        assert_eq!(style("34 0 1 0 0 1 100").line, LineSpacing::OneAndAHalf(0));
        assert_eq!(style("36 0 1 0 0 1 100").line, LineSpacing::Double(0));
        // Decompile: neither Add in bit adds the space as Always does, and
        // the spacing bits are tested 1, 2, 4 in turn.
        assert!(style("1 273 1 144 0 1 100").add_at_break);
        assert_eq!(style("3 432 1 0 0 1 100").line, LineSpacing::Single);
        assert_eq!(
            style("6 432 1 0 0 1 100").line,
            LineSpacing::OneAndAHalf(432)
        );
        assert_eq!(style("0 360 1 0 0 1 100").line, LineSpacing::Custom(360));
    }

    #[test]
    fn style_breaks_come_from_brk_as_written() {
        // VM fixtures B1–B8; a style without [brk] may break within.
        let breaks = |brk: Option<&str>| {
            let sample = format!(
                "[ver]\n4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tTimes New Roman\n\t\t240\n\t\t0\n\t\t49152\n{}[edoc]\nText\n\n>\n",
                brk.map_or(String::new(), |value| format!("\t[brk]\n\t\t{value}\n"))
            );
            let imported = import_sam(sample.as_bytes(), None, "Breaks").unwrap();
            imported.document.styles().values().next().unwrap().breaks
        };
        assert_eq!(breaks(None), Breaks::default());
        assert!(breaks(Some("5")).page_before());
        assert!(breaks(Some("6")).page_after());
        assert!(breaks(Some("132")).column_before());
        assert!(breaks(Some("260")).column_after());
        assert!(!breaks(Some("0")).within());
        assert!(breaks(Some("16")).keep_with_next());
        assert!(breaks(Some("8")).keep_with_previous());
        assert_eq!(
            breaks(Some("68")),
            Breaks(68),
            "Next style and unknown bits are kept"
        );
    }

    #[test]
    fn binary_after_the_body_ends_the_sections() {
        // A style sheet whose embedded drawing has no NUL byte (its NULs were
        // stripped in a copy): the styles before the body still import.
        let sample = b"[ver]\r\n\t4\r\n[tag]\r\n\tSpecial\r\n\t7\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t0\r\n[edoc]\r\n@Special@Text\r\n\r\n>\r\nSM1!Z*\xac\r\n[\xe9\xff\x8a\r\n7\r\n[Embedded]\r\n4 .sdw 6709 10614 0 0 \r\n00017325\r\n\x1a";
        let (legacy, report) = parse(sample, LegacyKind::StyleSheet).unwrap();
        assert_eq!(legacy.sections.last().unwrap().name, "edoc");
        assert_eq!(legacy.style_names, ["Special"]);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("binary data after the document body at line 16"))
        );
        let imported = import_sam(
            b"[ver]\n4\n[edoc]\n@Special@Hello\n\n>\n",
            Some(sample),
            "x",
        )
        .unwrap();
        assert_eq!(
            imported.document.styles()[&imported.document.paragraphs()[0].style].font_size_points,
            12.0
        );
        // Before the body a malformed header is still an error.
        assert_eq!(
            parse(b"[ver]\n4\n[\xe9\xff\n[edoc]\n>\n", LegacyKind::Document),
            Err(ParseError::InvalidSection(3))
        );
    }

    #[test]
    fn word_pro_and_password_protected_files_are_named() {
        // Synthetic: Word Pro's signature, then arbitrary bytes.
        let word_pro = b"WordPro\r\x01\x02\x03\x04\0\0\0\0";
        assert_eq!(
            parse(word_pro, LegacyKind::Document),
            Err(ParseError::WordPro)
        );
        let error = import_sam(word_pro, None, "x").unwrap_err();
        assert!(matches!(error, ImportError::Parse(ParseError::WordPro)));
        assert!(error.to_string().contains("Lotus Word Pro"));

        // Synthetic: the marker, then arbitrary coded bytes.
        let protected = b"[encrypt]\r\n\x11\x22\x33\x44\x55\x66\x77";
        let error = import_sam(protected, None, "x").unwrap_err();
        assert!(matches!(error, ImportError::PasswordProtected));
        assert!(error.to_string().contains("encrypted file"));
    }

    #[test]
    fn path_import_searches_upper_case_and_given_style_directories() {
        let root = std::env::temp_dir().join(format!(
            "amipro-formats-styles-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let (documents, client, year, styles) = (
            root.join("SAM"),
            root.join("SAM").join("CLIENT"),
            root.join("SAM").join("CLIENT").join("1995"),
            root.join("STYLES"),
        );
        fs::create_dir_all(&year).unwrap();
        fs::create_dir_all(&styles).unwrap();
        let sam = b"[ver]\n4\n[sty]\n~MEMO.STY\n[edoc]\n@Special@Hello\n\n>\n";
        fs::write(documents.join("top.sam"), sam).unwrap();
        fs::write(client.join("client.sam"), sam).unwrap();
        fs::write(year.join("deep.sam"), sam).unwrap();
        fs::write(
            styles.join("~memo.sty"),
            b"[ver]\n4\n[tag]\nSpecial\n7\n[fnt]\nArial\n240\n0\n0\n[edoc]\n>\n",
        )
        .unwrap();

        // DOS trees keep STYLES in upper case beside the documents' directory.
        assert!(
            import_sam_path(documents.join("top.sam"))
                .unwrap()
                .report
                .associated_style_resolved
        );
        // Documents filed one folder deeper find it two levels up (the
        // user's corpus: SAM\CLIENT beside STYLES).
        assert!(
            import_sam_path(client.join("client.sam"))
                .unwrap()
                .report
                .associated_style_resolved
        );
        // Deeper still, they need the style-sheet path.
        assert!(
            !import_sam_path(year.join("deep.sam"))
                .unwrap()
                .report
                .associated_style_resolved
        );
        let imported = import_sam_path_with_style_directories(
            year.join("deep.sam"),
            std::slice::from_ref(&styles),
        )
        .unwrap();
        assert!(imported.report.associated_style_resolved);
        assert_eq!(
            imported.report.associated_style_path,
            Some(styles.join("~memo.sty"))
        );
        assert_eq!(
            imported.document.styles()[&imported.document.paragraphs()[0].style].font_size_points,
            12.0
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn path_import_resolves_dos_style_name_case_insensitively() {
        let directory = std::env::temp_dir().join(format!(
            "amipro-formats-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&directory).unwrap();
        let sam_path = directory.join("sample.sam");
        let style_path = directory.join("Custom.Sty");
        fs::write(
            &sam_path,
            b"[ver]\n4\n[sty]\nCUSTOM.STY\n[edoc]\n@Special@Hello\n\n>\n",
        )
        .unwrap();
        fs::write(
            &style_path,
            b"[ver]\n4\n[tag]\nSpecial\n7\n[fnt]\nArial\n240\n0\n0\n[edoc]\n>\n",
        )
        .unwrap();

        let imported = import_sam_path(&sam_path).unwrap();

        assert!(imported.report.associated_style_resolved);
        assert_eq!(
            imported.document.styles()[&imported.document.paragraphs()[0].style].name,
            "Special"
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
