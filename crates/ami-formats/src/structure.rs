//! The structured part of a SAM file: frames, page layouts, tables, pictures,
//! and the embedded-object directory, read as raw values before mapping
//! (`docs/research/SAM_PAGE_FRAME_SPEC.md`, sections 2–4 and 6).
//!
//! Sections nest by indentation, but frame text, table cells, and the body
//! are unindented paragraphs ended by a lone `>`, so this reader walks lines
//! with the knowledge of which sections hold text. Text lines can never start
//! with `[`, because SAM writes it as `<[>`.

use crate::text::split_body;

/// A frame record (`[frm]` or `[lyfrm]`) with its parts.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RawFrame {
    /// The record's fields, one per line.
    pub values: Vec<String>,
    pub name: Option<String>,
    pub macro_name: Option<String>,
    /// `[frmlay]` values.
    pub interior: Vec<String>,
    pub content: RawContent,
    /// Other sections inside the frame, raw, in order.
    pub unknown: Vec<(String, String)>,
    /// The names of the frame's parts in the order they came, as written.
    pub parts: Vec<String>,
    /// The 1-based line of the record's header.
    pub line: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum RawContent {
    #[default]
    None,
    /// `[txt]` paragraphs, raw.
    Text(Vec<String>),
    Table(RawTable),
    /// `[isd]` values.
    Picture(Vec<String>),
    /// A frame whose parts could not be read, kept raw so that the frame
    /// still holds its place in the stack and the anchor order.
    Unreadable(String),
}

/// A table cell: its record, its text, and any further bodies Ami Pro wrote
/// after the text (a formula follows a calculated cell's value).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RawCell {
    pub record: String,
    pub paragraphs: Vec<String>,
    pub extra: Vec<String>,
}

/// A `[tbl]` section: its header values, row and column records, and cells
/// (the cell record and its raw paragraphs).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RawTable {
    pub header: Vec<String>,
    pub rows: Vec<String>,
    pub columns: Vec<String>,
    pub cells: Vec<RawCell>,
    pub unknown: Vec<(String, String)>,
    /// Cell records whose eighth field disagrees with the text after them.
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Slot {
    HeaderRight,
    HeaderLeft,
    FooterRight,
    FooterLeft,
    Repeating,
}

impl Slot {
    /// The section names, in the order Ami Pro usually writes them.
    pub(crate) const NAMES: [&'static str; 5] = ["hlft", "hrght", "flft", "frght", "repfrm"];

    /// The section name that introduces a frame in this slot.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::HeaderLeft => "hlft",
            Self::HeaderRight => "hrght",
            Self::FooterLeft => "flft",
            Self::FooterRight => "frght",
            Self::Repeating => "repfrm",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RawLayout {
    pub name: String,
    pub number: String,
    pub right: Option<Vec<String>>,
    pub left: Option<Vec<String>>,
    pub frames: Vec<(Slot, RawFrame)>,
    pub unknown: Vec<(String, String)>,
    pub line: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SamStructure {
    /// Top-level frames in file order: back to front.
    pub frames: Vec<RawFrame>,
    pub layouts: Vec<RawLayout>,
    /// Other top-level sections before the body, raw, in order. The `[pg]`
    /// page table is a cache and is left out.
    pub sections: Vec<(String, String)>,
    pub warnings: Vec<String>,
}

fn header(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    (trimmed.len() >= 3 && trimmed.starts_with('[') && trimmed.ends_with(']'))
        .then(|| &trimmed[1..trimmed.len() - 1])
}

fn is_top_level_header(line: &str) -> bool {
    !line.starts_with('\t') && !line.starts_with(' ') && header(line).is_some()
}

/// Reads the values after a header: lines up to the next header.
fn values(lines: &[&str], mut index: usize) -> (Vec<String>, usize) {
    let mut values = Vec::new();
    while let Some(line) = lines.get(index) {
        if header(line).is_some() {
            break;
        }
        values.push(line.trim_start_matches(['\t', ' ']).trim_end().to_owned());
        index += 1;
    }
    (values, index)
}

fn raw_section(name: &str, values: &[String]) -> (String, String) {
    (name.to_owned(), values.join("\n"))
}

/// Reads the structure of the lines before the body (`[edoc]`).
pub(crate) fn read_structure(lines: &[&str]) -> SamStructure {
    let mut structure = SamStructure::default();
    let starts: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| is_top_level_header(line))
        .map(|(index, _)| index)
        .collect();
    for (position, start) in starts.iter().enumerate() {
        let name = header(lines[*start]).unwrap_or_default();
        if name.eq_ignore_ascii_case("edoc") || name.eq_ignore_ascii_case("embedded") {
            break;
        }
        let end = starts.get(position + 1).copied().unwrap_or(lines.len());
        let block = &lines[start + 1..end];
        match name.to_ascii_lowercase().as_str() {
            "frm" => match read_frame(block, 0, start + 1) {
                Ok((frame, consumed)) if consumed == block.len() => structure.frames.push(frame),
                Ok(_) | Err(_) => {
                    structure.warnings.push(format!(
                        "frame at line {} has an unexpected shape; its content was kept as raw data",
                        start + 1
                    ));
                    // Keep the record so that later frames keep their place
                    // in the stack and in the anchor order.
                    let (values, _) = values(block, 0);
                    structure.frames.push(RawFrame {
                        values,
                        content: RawContent::Unreadable(block.join("\n")),
                        line: start + 1,
                        ..RawFrame::default()
                    });
                }
            },
            "lay" => match read_layout(block, start + 1) {
                Ok(layout) => structure.layouts.push(layout),
                Err(message) => {
                    structure.warnings.push(format!(
                        "page layout at line {}: {message}; kept as raw data",
                        start + 1
                    ));
                    structure.sections.push((name.to_owned(), block.join("\n")));
                }
            },
            "pg" => {}
            _ => structure.sections.push((name.to_owned(), block.join("\n"))),
        }
    }
    structure
}

/// Reads a frame record and its parts, starting at the line after its header,
/// and returns it with the number of lines read. It stops at a header that is
/// not a frame part.
fn read_frame(
    lines: &[&str],
    start: usize,
    header_line: usize,
) -> Result<(RawFrame, usize), String> {
    let (values, mut index) = values(lines, start);
    if values.len() < 13 {
        return Err("a frame record has at least 13 fields".into());
    }
    let mut frame = RawFrame {
        values,
        line: header_line,
        ..RawFrame::default()
    };
    while let Some(name) = lines.get(index).and_then(|line| header(line)) {
        let lower = name.to_ascii_lowercase();
        if !matches!(
            lower.as_str(),
            "lyfrm" | "hrght" | "hlft" | "frght" | "flft" | "repfrm" | "rght" | "lft"
        ) {
            frame.parts.push(name.to_owned());
        }
        match lower.as_str() {
            "frmname" | "frmmac" => {
                let (found, next) = values_of(lines, index + 1);
                let value = found.into_iter().next().unwrap_or_default();
                if name.eq_ignore_ascii_case("frmname") {
                    frame.name = Some(value);
                } else {
                    frame.macro_name = Some(value);
                }
                index = next;
            }
            "frmlay" => {
                let (found, next) = values_of(lines, index + 1);
                frame.interior = found;
                index = next;
            }
            "txt" => {
                let (paragraphs, consumed) = split_body(lines[index + 1..].iter().copied());
                frame.content = RawContent::Text(paragraphs);
                index += 1 + consumed;
            }
            "tbl" => {
                let (table, next) = read_table(lines, index + 1)?;
                frame.content = RawContent::Table(table);
                index = next;
            }
            "isd" => {
                let (found, next) = values_of(lines, index + 1);
                frame.content = RawContent::Picture(found);
                index = next;
            }
            "lyfrm" | "hrght" | "hlft" | "frght" | "flft" | "repfrm" | "rght" | "lft" => break,
            _ => {
                let (found, next) = values_of(lines, index + 1);
                frame.unknown.push(raw_section(name, &found));
                index = next;
            }
        }
    }
    Ok((frame, index))
}

fn values_of(lines: &[&str], index: usize) -> (Vec<String>, usize) {
    values(lines, index)
}

/// Reads a `[tbl]` section after its header and returns it with the index of
/// the line after its closing `[tble]`.
fn read_table(lines: &[&str], start: usize) -> Result<(RawTable, usize), String> {
    let (header_values, mut index) = values(lines, start);
    let mut table = RawTable {
        header: header_values,
        ..RawTable::default()
    };
    loop {
        let Some(name) = lines.get(index).and_then(|line| header(line)) else {
            return Err("a table ends with [tble]".into());
        };
        match name.to_ascii_lowercase().as_str() {
            "tble" => return Ok((table, index + 1)),
            "e" => index += 1,
            "h" | "w" => {
                let (records, next) = values(lines, index + 1);
                if name.eq_ignore_ascii_case("h") {
                    table.rows.extend(records);
                } else {
                    table.columns.extend(records);
                }
                index = next;
            }
            "data" => {
                index += 1;
                while let Some(line) = lines.get(index) {
                    if header(line).is_some() {
                        break;
                    }
                    if !is_cell_record(line) {
                        return Err("a table cell needs its record line".into());
                    }
                    let record = line.trim().to_owned();
                    // A covered cell writes its record with no text.
                    let next = lines.get(index + 1).copied().unwrap_or_default();
                    let mut cell = RawCell {
                        record,
                        ..RawCell::default()
                    };
                    index += 1;
                    let has_text = !(header(next).is_some() || is_cell_record(next));
                    if has_text {
                        let (paragraphs, consumed) = split_body(lines[index..].iter().copied());
                        cell.paragraphs = paragraphs;
                        index += consumed;
                    }
                    // The eighth field is 1 exactly when text follows (21,588
                    // corpus records); Ami Pro refuses a file where it is 0
                    // and text follows.
                    let says_text = cell
                        .record
                        .split_whitespace()
                        .nth(7)
                        .is_some_and(|field| field != "0");
                    if says_text != has_text {
                        table.warnings.push(format!(
                            "table cell record {:?} {} text, but {}",
                            cell.record,
                            if says_text { "has" } else { "has no" },
                            if has_text {
                                "text follows"
                            } else {
                                "none follows"
                            }
                        ));
                    }
                    // Further bodies before the next record belong to the cell.
                    while let Some(line) = lines.get(index) {
                        if header(line).is_some() || is_cell_record(line) {
                            break;
                        }
                        let (paragraphs, consumed) = split_body(lines[index..].iter().copied());
                        cell.extra.push(paragraphs.join("\n\n"));
                        index += consumed;
                    }
                    table.cells.push(cell);
                }
            }
            _ => {
                let (found, next) = values(lines, index + 1);
                table.unknown.push(raw_section(name, &found));
                index = next;
            }
        }
    }
}

/// A table cell record: an indented line of integers, starting with the
/// cell's row and column.
fn is_cell_record(line: &str) -> bool {
    line.starts_with('\t') && {
        let fields: Vec<&str> = line.split_whitespace().collect();
        fields.len() >= 2 && fields.iter().all(|field| field.parse::<i64>().is_ok())
    }
}

fn read_layout(lines: &[&str], header_line: usize) -> Result<RawLayout, String> {
    let (head, mut index) = values(lines, 0);
    let mut layout = RawLayout {
        name: head.first().cloned().unwrap_or_default(),
        number: head.get(1).cloned().unwrap_or_default(),
        line: header_line,
        ..RawLayout::default()
    };
    let mut slot = None;
    while let Some(name) = lines.get(index).and_then(|line| header(line)) {
        match name.to_ascii_lowercase().as_str() {
            "rght" | "lft" => {
                let (found, next) = values(lines, index + 1);
                if name.eq_ignore_ascii_case("rght") {
                    layout.right = Some(found);
                } else {
                    layout.left = Some(found);
                }
                index = next;
            }
            "hrght" => {
                slot = Some(Slot::HeaderRight);
                index += 1;
            }
            "hlft" => {
                slot = Some(Slot::HeaderLeft);
                index += 1;
            }
            "frght" => {
                slot = Some(Slot::FooterRight);
                index += 1;
            }
            "flft" => {
                slot = Some(Slot::FooterLeft);
                index += 1;
            }
            "repfrm" => {
                slot = Some(Slot::Repeating);
                index += 1;
            }
            "lyfrm" => {
                let slot = slot.ok_or("a layout frame comes after its slot")?;
                let (frame, next) = read_frame(lines, index + 1, header_line + index + 1)?;
                layout.frames.push((slot, frame));
                index = next;
            }
            _ => {
                let (found, next) = values(lines, index + 1);
                layout.unknown.push(raw_section(name, &found));
                index = next;
            }
        }
    }
    if index != lines.len() {
        return Err("unexpected text in a page layout".into());
    }
    Ok(layout)
}

/// One `[Embedded]` directory entry: a picture frame's original file and Ami
/// Pro's own rendering of it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EmbeddedEntry {
    /// The frame's sequence number (field 13).
    pub sequence: u32,
    pub extension: String,
    pub original: Option<(usize, usize)>,
    pub rendering: Option<(usize, usize)>,
    pub path: Option<String>,
}

/// Reads the `[Embedded]` directory from the raw file: the last line is the
/// eight-digit offset of the `[Embedded]` line, and each directory line is
/// `SEQ EXT OFFSET LENGTH ROFFSET RLENGTH [PATH]`. Every range must lie
/// before the directory.
pub(crate) fn embedded_directory(bytes: &[u8]) -> Result<Vec<EmbeddedEntry>, String> {
    let trimmed_end = bytes
        .iter()
        .rposition(|byte| !matches!(byte, b'\r' | b'\n'))
        .map_or(0, |last| last + 1);
    let trailer_start = bytes[..trimmed_end]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |newline| newline + 1);
    let trailer = std::str::from_utf8(&bytes[trailer_start..trimmed_end])
        .map_err(|_| "the trailer is not text")?
        .trim();
    if trailer.len() != 8 || !trailer.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("no [Embedded] trailer".into());
    }
    let directory: usize = trailer.parse().map_err(|_| "malformed trailer")?;
    let marker = b"[Embedded]";
    if bytes.get(directory..directory + marker.len()) != Some(marker.as_slice())
        || directory + marker.len() > trailer_start
    {
        return Err("the trailer does not point at [Embedded]".into());
    }
    let listing = crate::decode_windows_1252(&bytes[directory + marker.len()..trailer_start]);
    let mut entries = Vec::new();
    for line in listing
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 6 {
            return Err(format!("malformed directory line {line:?}"));
        }
        let number = |index: usize| -> Result<usize, String> {
            fields[index]
                .parse()
                .map_err(|_| format!("malformed directory line {line:?}"))
        };
        let range = |offset: usize, length: usize| -> Result<Option<(usize, usize)>, String> {
            if length == 0 {
                return Ok(None);
            }
            match offset.checked_add(length) {
                Some(end) if end <= directory => Ok(Some((offset, length))),
                _ => Err(format!("directory range out of bounds in {line:?}")),
            }
        };
        let path = (fields.len() > 6).then(|| {
            // The path may contain spaces: it is the rest of the line.
            let mut rest = line;
            for _ in 0..6 {
                rest = rest.trim_start();
                rest = &rest[rest.find(char::is_whitespace).unwrap_or(rest.len())..];
            }
            rest.trim().to_owned()
        });
        entries.push(EmbeddedEntry {
            sequence: u32::try_from(number(0)?).map_err(|_| "sequence out of range")?,
            extension: fields[1].to_owned(),
            original: range(number(2)?, number(3)?)?,
            rendering: range(number(4)?, number(5)?)?,
            path,
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(text: &str) -> Vec<&str> {
        text.lines().collect()
    }

    #[test]
    fn reads_frames_with_parts_and_layouts_with_slots() {
        let source = "[ver]\n\t4\n[frm]\n\t1\n\t66240\n\t2160\n\t2880\n\t5040\n\t5040\n\t1\n\t1\n\t1\n\t1 0 0 100 100 18 0\n\t0\n\t16777215\n\t3\n\t[frmname]\n\t\tFrame3\n\t[frmlay]\n\t\t5040\n\t\t2880\n\t[txt]\nFrame text\n\n>\n[lay]\n\tStandard\n\t513\n\t[rght]\n\t\t15840\n\t[hrght]\n\t[lyfrm]\n\t\t1\n\t\t11200\n\t\t0\n\t\t0\n\t\t12240\n\t\t1440\n\t\t0\n\t\t1\n\t\t3\n\t\t1 0 0 0 0 0 0\n\t\t0\n\t\t0\n\t\t1\n\t[frmlay]\n\t\t1440\n\t[txt]\n>\n\t[repfrm]\n\t[lyfrm]\n\t\t1\n\t\t16832\n\t\t0\n\t\t0\n\t\t100\n\t\t100\n\t\t0\n\t\t1\n\t\t1\n\t\t1 0 0 0 0 0 0\n\t\t0\n\t\t0\n\t\t2\n\t[txt]\nRepeated\n\n>\n[pg]\n\t1\n[edoc]\nBody\n>\n";
        let structure = read_structure(&lines(source));
        assert!(structure.warnings.is_empty(), "{:?}", structure.warnings);
        assert_eq!(structure.sections, [("ver".to_owned(), "\t4".to_owned())]);
        let frame = &structure.frames[0];
        assert_eq!(frame.values[1], "66240");
        assert_eq!(frame.name.as_deref(), Some("Frame3"));
        assert_eq!(frame.interior, ["5040", "2880"]);
        assert_eq!(frame.content, RawContent::Text(vec!["Frame text".into()]));
        let layout = &structure.layouts[0];
        assert_eq!(
            (layout.name.as_str(), layout.number.as_str()),
            ("Standard", "513")
        );
        assert_eq!(layout.right.as_deref(), Some(&["15840".to_owned()][..]));
        assert_eq!(layout.frames.len(), 2);
        assert_eq!(layout.frames[0].0, Slot::HeaderRight);
        assert_eq!(layout.frames[0].1.content, RawContent::Text(Vec::new()));
        assert_eq!(layout.frames[1].0, Slot::Repeating);
    }

    #[test]
    fn reads_tables_with_row_and_column_records_and_cell_text() {
        let source = "\t\t 2 2 284 86 4569 86 1 43 43\n\t\t[h]\n\t\t\t 0 568 86 6 0 1 0\n\t\t[e]\n\t\t[w]\n\t\t\t 1 6592 0 6 0\n\t\t[e]\n\t\t[data]\n\t\t\t 0 0 0 0 0 0 0 1 0 0 0 0\nA1\n\nsecond\n\n>\n\t\t\t 1 1 0 0 0 0 0 1 0 0 0 0\nB2\n\n>\n\t\t[e]\n\t[tble]\n[lay]\n";
        let (table, next) = read_table(&lines(source), 0).unwrap();
        assert_eq!(table.header, ["2 2 284 86 4569 86 1 43 43"]);
        assert_eq!(table.rows, ["0 568 86 6 0 1 0"]);
        assert_eq!(table.columns, ["1 6592 0 6 0"]);
        assert_eq!(
            table
                .cells
                .iter()
                .map(|cell| (cell.record.as_str(), cell.paragraphs.clone()))
                .collect::<Vec<_>>(),
            [
                (
                    "0 0 0 0 0 0 0 1 0 0 0 0",
                    vec!["A1".to_owned(), "second".to_owned()]
                ),
                ("1 1 0 0 0 0 0 1 0 0 0 0", vec!["B2".to_owned()]),
            ]
        );
        assert_eq!(lines(source)[next], "[lay]");

        // A covered cell has no text; a calculated cell's formula follows its
        // value as a second body (MERCINV-like).
        let covered = "\t\t 1 3 0 0 1440 0\n\t\t[data]\n\t\t\t 0 0 384 1 2 0 0 1 0 0 0 0\nJoined text\n\n>\n\t\t\t 0 1 128 0 1 0 0 0 0 0 0 0\n\t\t\t 0 2 80 0 0 0 17 1 0 0 0 0\n@Amount@<+A>3250.00\n\n>\n@Amount@<+A>d4*e4\n\n>\n\t\t[e]\n\t[tble]\n";
        let (table, _) = read_table(&lines(covered), 0).unwrap();
        assert_eq!(table.cells[0].paragraphs, ["Joined text"]);
        assert_eq!(table.cells[1].record, "0 1 128 0 1 0 0 0 0 0 0 0");
        assert!(table.cells[1].paragraphs.is_empty());
        assert_eq!(table.cells[2].paragraphs, ["@Amount@<+A>3250.00"]);
        assert_eq!(table.cells[2].extra, ["@Amount@<+A>d4*e4"]);
        assert!(read_table(&lines("\t\t 1 1\n\t\t[data]\n"), 0).is_err());
    }

    #[test]
    fn reads_the_embedded_directory_within_bounds() {
        let mut file = b"[edoc]\r\nx\r\n>\r\n".to_vec();
        let original = file.len();
        file.extend_from_slice(b"BMDATA");
        let rendering = file.len();
        file.extend_from_slice(b"SS");
        let directory = file.len();
        file.extend_from_slice(
            format!(
                "[Embedded]\r\n3 .bmp {original} 6 {rendering} 2 \r\n4 .pcx 0 0 {rendering} 2 C:\\MY PICS\\A.PCX\r\n{directory:08}\r\n"
            )
            .as_bytes(),
        );
        let entries = embedded_directory(&file).unwrap();
        assert_eq!(entries[0].original, Some((original, 6)));
        assert_eq!(entries[0].rendering, Some((rendering, 2)));
        assert_eq!(entries[1].original, None);
        assert_eq!(entries[1].path.as_deref(), Some("C:\\MY PICS\\A.PCX"));

        let mut out_of_bounds = file.clone();
        let text = String::from_utf8_lossy(&out_of_bounds).replace(" 6 ", " 600 ");
        out_of_bounds = text.into_bytes();
        assert!(embedded_directory(&out_of_bounds).is_err());
        assert!(embedded_directory(b"[edoc]\r\n>\r\n").is_err());
    }
}
