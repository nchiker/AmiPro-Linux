//! Merge data files (`docs/research/MERGE_NOTES.md`, section 3; decision
//! MG2): Ami Pro's delimited data document, a document holding only a
//! table, the description-file kinds (DIF, fixed-width, comma), and the
//! app's additions: CSV and TSV, and dBASE `.DBF` read by its public
//! format. XLSX and ODS are read in `amipro-exchange` into the same
//! [`DataTable`].

use amipro_document::{Document, FrameContent, OBJECT_REPLACEMENT_CHARACTER, decode_windows_1252};

/// A data file's field names and records.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DataTable {
    pub names: Vec<String>,
    /// Each record's values in the names' order; a short record's missing
    /// values are empty.
    pub records: Vec<Vec<String>>,
}

impl DataTable {
    /// The field `name` is, matched as Ami Pro matches names
    /// ([`field_key`]).
    pub fn field(&self, name: &str) -> Option<usize> {
        let key = field_key(name);
        self.names.iter().position(|field| field_key(field) == key)
    }

    /// Record `record`'s value of field `name`; `None` when there is no
    /// such field or record.
    pub fn value(&self, record: usize, name: &str) -> Option<&str> {
        let field = self.field(name)?;
        let values = self.records.get(record)?;
        Some(values.get(field).map(String::as_str).unwrap_or(""))
    }

    /// The names as a merge field's code names them: Ami Pro's keys.
    pub fn field_keys(&self) -> Vec<String> {
        self.names.iter().map(|name| field_key(name)).collect()
    }

    fn from_rows(mut rows: Vec<Vec<String>>) -> Result<Self, DataError> {
        if rows.is_empty() {
            return Err(DataError::NoFieldNames);
        }
        let names = rows.remove(0);
        if names.is_empty() || names.iter().any(|name| name.trim().is_empty()) {
            return Err(DataError::EmptyFieldName);
        }
        let width = names.len();
        let records = rows
            .into_iter()
            .map(|mut record| {
                record.resize(width.max(record.len()), String::new());
                record
            })
            .collect();
        Ok(Self { names, records })
    }
}

/// A field name as Ami Pro matches it (`13a0:0ef5`): capitals, spaces as
/// `_`, other characters but letters, digits, and `_` dropped.
pub fn field_key(name: &str) -> String {
    name.trim()
        .chars()
        .filter_map(|character| {
            if character == ' ' {
                Some('_')
            } else if character.is_alphanumeric() || character == '_' {
                Some(character)
            } else {
                None
            }
        })
        .flat_map(char::to_uppercase)
        .collect()
}

/// Why a data file cannot be read, with Ami Pro's message where it has one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataError {
    /// Ami Pro's 3005.
    NotValid,
    NoFieldNames,
    EmptyFieldName,
    /// A document that holds no table.
    NoTable,
    /// A description file whose first paragraph is not a kind.
    UnknownDescription(String),
    /// A fixed-width field without a width 1–2047 (Ami Pro's 3096–3099).
    BadWidth(String),
    /// A text file with control characters in its first 80 bytes
    /// (`1390:0241`).
    NotText,
    /// A `.DBF` file that is not dBASE's.
    NotDbase,
    /// Anything else, said in words.
    Other(String),
}

impl DataError {
    pub fn message(&self) -> String {
        match self {
            Self::NotValid => "Data file is not in valid format.".into(),
            Self::NoFieldNames => "The data file has no field names.".into(),
            Self::EmptyFieldName => "The data file has an empty field name.".into(),
            Self::NoTable => "The document holds no table.".into(),
            Self::UnknownDescription(kind) => {
                format!(
                    "The description file's kind \u{201c}{kind}\u{201d} is not DIF, FIXED, or COMMA."
                )
            }
            Self::BadWidth(name) => {
                format!("The field \u{201c}{name}\u{201d} needs a width from 1 to 2047.")
            }
            Self::NotText => "The data file is not a text file.".into(),
            Self::NotDbase => "The file is not a dBASE file.".into(),
            Self::Other(text) => text.clone(),
        }
    }
}

/// An Ami Pro data document's delimiters: the field delimiter, and the
/// record delimiter or, when `None`, the paragraph end.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Delimiters {
    pub field: char,
    pub record: Option<char>,
}

impl Default for Delimiters {
    /// New files' (the shipped `MERCDATA.SAM`, `_MERGDAT.STY`).
    fn default() -> Self {
        Self {
            field: '~',
            record: Some('|'),
        }
    }
}

/// The delimiters Ami Pro's Create Data File offers.
pub const SUGGESTED_DELIMITERS: [char; 10] = ['$', '#', '!', '%', '(', ')', '~', '|', '-', ','];

/// The merge engine's own rule for a field's text (TD.9, VM TDV7; it does
/// not depend on the table's number format): when the text ends in a digit
/// (or a bare point), has a point after a digit, and has only digits, `$`,
/// `,`, `+`, `-` and points besides, its trailing zeros and then a bare
/// trailing point go: `$1,200.00` is `$1,200`, `75.50` is `75.5`, `1.5.0`
/// is `1.5`. Anything else (`007`, `-0`, `5%`, ` 5`, `1.00 `, a letter,
/// `.50`) merges as it stands.
pub fn merge_number_text(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let ends_right = characters
        .last()
        .is_some_and(|last| last.is_ascii_digit() || *last == '.');
    let only_number_marks = characters
        .iter()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | '$' | ',' | '+' | '-'));
    let point_after_digit = characters
        .iter()
        .position(|c| *c == '.')
        .is_some_and(|at| at > 0 && characters[at - 1].is_ascii_digit());
    if !(ends_right && only_number_marks && point_after_digit) {
        return text.to_string();
    }
    let zeros = text.trim_end_matches('0');
    zeros.strip_suffix('.').unwrap_or(zeros).to_string()
}

impl DataTable {
    /// The table as a letter merges it when it came from a table data file
    /// (TD.9): every value through [`merge_number_text`].
    pub fn with_merged_numbers(&self) -> DataTable {
        DataTable {
            names: self.names.clone(),
            records: self
                .records
                .iter()
                .map(|record| record.iter().map(|v| merge_number_text(v)).collect())
                .collect(),
        }
    }
}

/// A paragraph's text without its objects and with its marks gone (the
/// formatting stripped, as `13a8:0ab7` reads it).
fn plain(text: &str) -> String {
    text.chars()
        .filter(|character| *character != OBJECT_REPLACEMENT_CHARACTER)
        .map(|character| {
            if character == '\u{2028}' {
                '\n'
            } else {
                character
            }
        })
        .collect()
}

/// Reads an Ami Pro data document (kind 0): the first paragraph its
/// delimiters, the next record its field names, then the records. A
/// record may span paragraphs (joined with a line feed) and ends at the
/// record delimiter; an empty field is two delimiters together.
pub fn read_ami_data(document: &Document) -> Result<(DataTable, Delimiters), DataError> {
    let paragraphs: Vec<String> = document
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| plain(&paragraph.text))
        .collect();
    let first = paragraphs.first().ok_or(DataError::NotValid)?;
    let marks: Vec<char> = first.trim().chars().collect();
    let delimiters = match marks.as_slice() {
        [field] => Delimiters {
            field: *field,
            record: None,
        },
        [field, record] if field != record => Delimiters {
            field: *field,
            record: Some(*record),
        },
        _ => return Err(DataError::NotValid),
    };
    let rest = &paragraphs[1..];
    let records: Vec<String> = match delimiters.record {
        None => rest
            .iter()
            .filter(|paragraph| !paragraph.is_empty())
            .cloned()
            .collect(),
        Some(end) => {
            let stream = rest.join("\n");
            let mut records: Vec<String> = stream
                .split(end)
                .map(|record| record.trim_start_matches('\n').to_string())
                .collect();
            // What follows the last record delimiter is not a record.
            if records.last().is_some_and(|last| last.trim().is_empty()) {
                records.pop();
            }
            records
        }
    };
    let rows = records
        .iter()
        .map(|record| {
            record
                .split(delimiters.field)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect();
    let table = DataTable::from_rows(rows).map_err(|error| match error {
        DataError::NoFieldNames => DataError::NotValid,
        other => other,
    })?;
    Ok((table, delimiters))
}

/// Reads a document holding a table (kind 4): the first table's first row
/// holds the field names, each later row a record; a cell's paragraphs
/// join with a line feed.
pub fn read_table_data(document: &Document) -> Result<DataTable, DataError> {
    let table = document
        .frames()
        .find_map(|frame| match &frame.content {
            FrameContent::Table(table) => Some(table),
            _ => None,
        })
        .ok_or(DataError::NoTable)?;
    let columns = table.columns.len();
    if columns == 0 {
        return Err(DataError::NoTable);
    }
    let rows = table
        .cells
        .chunks(columns)
        .map(|row| {
            row.iter()
                .map(|cell| {
                    let amipro_document::CellContent::Text(flow) = &cell.content;
                    document
                        .flow(*flow)
                        .map(|flow| {
                            flow.paragraphs
                                .iter()
                                .map(|paragraph| plain(&paragraph.text))
                                .collect::<Vec<_>>()
                                .join("\n")
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    DataTable::from_rows(rows)
}

/// A description file's kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DescriptionKind {
    Dif,
    Fixed,
    Comma,
    /// `DBASE`: an external database, which the app reads natively.
    Dbase,
}

/// A description file: its kind and fields (for FIXED, with widths).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Description {
    pub kind: DescriptionKind,
    pub names: Vec<String>,
    pub widths: Vec<usize>,
}

/// Reads a description file (an Ami Pro document): its first paragraph
/// the kind, any case; each later paragraph a field name (`name,width`
/// for FIXED); spaces in names become `_`.
pub fn read_description(document: &Document) -> Result<Description, DataError> {
    let mut paragraphs = document
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| plain(&paragraph.text).trim().to_string())
        .filter(|text| !text.is_empty());
    let kind_text = paragraphs.next().ok_or(DataError::NoFieldNames)?;
    let kind = match kind_text.to_ascii_uppercase().as_str() {
        "DIF" => DescriptionKind::Dif,
        "FIXED" => DescriptionKind::Fixed,
        "COMMA" => DescriptionKind::Comma,
        "DBASE" => DescriptionKind::Dbase,
        _ => return Err(DataError::UnknownDescription(kind_text)),
    };
    let mut names = Vec::new();
    let mut widths = Vec::new();
    for line in paragraphs {
        if kind == DescriptionKind::Fixed {
            let (name, width) = line
                .rsplit_once(',')
                .ok_or_else(|| DataError::BadWidth(line.clone()))?;
            let width: usize = width
                .trim()
                .parse()
                .ok()
                .filter(|width| (1..=2047).contains(width))
                .ok_or_else(|| DataError::BadWidth(name.trim().to_string()))?;
            names.push(name.trim().replace(' ', "_"));
            widths.push(width);
        } else {
            names.push(line.replace(' ', "_"));
        }
    }
    if names.is_empty() {
        return Err(DataError::NoFieldNames);
    }
    Ok(Description {
        kind,
        names,
        widths,
    })
}

/// Text from a plain-text data file, after Ami Pro's check that its first
/// 80 bytes hold no control characters but tabs and line ends
/// (`1390:0241`). UTF-8 when it is valid, else Windows-1252.
pub fn text_of(bytes: &[u8]) -> Result<String, DataError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if bytes
        .iter()
        .take(80)
        .any(|byte| *byte < 0x20 && !matches!(byte, b'\t' | b'\r' | b'\n' | 0x1A))
    {
        return Err(DataError::NotText);
    }
    Ok(match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => decode_windows_1252(bytes),
    })
}

fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split('\n')
        .map(|line| line.trim_end_matches('\r').trim_end_matches('\u{1A}'))
        .filter(|line| !line.trim().is_empty())
}

/// Splits one line of an Ami Pro comma file (`1548`): on commas outside
/// double quotes, `""` an embedded quote, spaces trimmed at both ends.
pub fn split_comma_line(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut value = String::new();
    let mut quoted = false;
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '"' if quoted && characters.peek() == Some(&'"') => {
                characters.next();
                value.push('"');
            }
            '"' => quoted = !quoted,
            ',' if !quoted => values.push(std::mem::take(&mut value).trim().to_string()),
            other => value.push(other),
        }
    }
    values.push(value.trim().to_string());
    values
}

/// Reads a comma-delimited file: one record a line; names from the
/// description, or from the first line when there is none.
pub fn read_comma(text: &str, names: Option<&[String]>) -> Result<DataTable, DataError> {
    let mut rows: Vec<Vec<String>> = lines(text).map(split_comma_line).collect();
    if let Some(names) = names {
        rows.insert(0, names.to_vec());
    }
    DataTable::from_rows(rows)
}

/// Reads a fixed-width file: each line cut at the description's widths,
/// in characters, each value trimmed.
pub fn read_fixed(text: &str, description: &Description) -> Result<DataTable, DataError> {
    let mut rows = vec![description.names.clone()];
    for line in lines(text) {
        let characters: Vec<char> = line.chars().collect();
        let mut at = 0;
        let mut record = Vec::new();
        for width in &description.widths {
            let end = (at + width).min(characters.len());
            let start = at.min(end);
            record.push(
                characters[start..end]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_string(),
            );
            at += width;
        }
        rows.push(record);
    }
    DataTable::from_rows(rows)
}

/// Reads a DIF file (Data Interchange Format): its `DATA` part's tuples,
/// each a record (`1540`). Numbers keep the text they were written with.
pub fn read_dif(text: &str, names: Option<&[String]>) -> Result<DataTable, DataError> {
    let mut lines = text.split('\n').map(|line| line.trim_end_matches('\r'));
    // The header's entries come in threes (topic, number line, string line)
    // up to DATA.
    loop {
        let topic = lines.next().ok_or(DataError::NotValid)?;
        lines.next().ok_or(DataError::NotValid)?;
        lines.next().ok_or(DataError::NotValid)?;
        if topic.trim().eq_ignore_ascii_case("DATA") {
            break;
        }
    }
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Option<Vec<String>> = None;
    while let Some(kind_line) = lines.next() {
        let value_line = lines.next().unwrap_or_default();
        let (kind, number) = kind_line.split_once(',').unwrap_or((kind_line, ""));
        match kind.trim() {
            "-1" => match value_line.trim() {
                "BOT" => {
                    if let Some(done) = row.take() {
                        rows.push(done);
                    }
                    row = Some(Vec::new());
                }
                "EOD" => break,
                _ => {}
            },
            "0" => {
                let value = match value_line.trim() {
                    "V" => number.trim().to_string(),
                    "TRUE" | "FALSE" => value_line.trim().to_string(),
                    _ => String::new(),
                };
                if let Some(row) = &mut row {
                    row.push(value);
                }
            }
            "1" => {
                let value = value_line.trim();
                let value = value
                    .strip_prefix('"')
                    .and_then(|value| value.strip_suffix('"'))
                    .unwrap_or(value)
                    .replace("\"\"", "\"");
                if let Some(row) = &mut row {
                    row.push(value);
                }
            }
            _ => {}
        }
    }
    if let Some(done) = row.take() {
        rows.push(done);
    }
    if let Some(names) = names {
        rows.insert(0, names.to_vec());
    }
    DataTable::from_rows(rows)
}

/// Reads a CSV or TSV file (RFC 4180 with `separator`): the first row the
/// field names; quoted values may hold separators, quotes (`""`), and
/// line ends.
pub fn read_separated(text: &str, separator: char) -> Result<DataTable, DataError> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row = Vec::new();
    let mut value = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if quoted {
            match character {
                '"' if characters.peek() == Some(&'"') => {
                    characters.next();
                    value.push('"');
                }
                '"' => quoted = false,
                other => value.push(other),
            }
            continue;
        }
        match character {
            '"' if value.is_empty() => {
                quoted = true;
                started = true;
            }
            '\r' => {}
            '\n' => {
                if started || !value.is_empty() || !row.is_empty() {
                    row.push(std::mem::take(&mut value));
                    rows.push(std::mem::take(&mut row));
                }
                started = false;
            }
            character if character == separator => {
                row.push(std::mem::take(&mut value));
                started = true;
            }
            other => {
                value.push(other);
                started = true;
            }
        }
    }
    if started || !value.is_empty() || !row.is_empty() {
        row.push(value);
        rows.push(row);
    }
    // A header with trailing empty names (a spreadsheet's export) keeps
    // only the named columns.
    if let Some(names) = rows.first_mut() {
        while names.last().is_some_and(|name| name.trim().is_empty()) && names.len() > 1 {
            names.pop();
        }
    }
    DataTable::from_rows(rows)
}

/// Reads a dBASE III or IV `.DBF` file by its public format: the header's
/// record count, header and record sizes, the field descriptors to the
/// 0x0D terminator, then fixed-width records whose first byte marks a
/// deleted record (`*`, left out). Character values are trimmed, numbers
/// keep their text, dates (`YYYYMMDD`) show as M/D/YYYY so conditions read
/// them as dates (MC1), logical values as `T` or `F`, and memo fields are
/// empty (MC2). Text is read in the language driver's code page
/// (Windows-1252 unless it names DOS 437 or 850, read as 1252 too).
pub fn read_dbase(bytes: &[u8]) -> Result<DataTable, DataError> {
    if bytes.len() < 33 || !matches!(bytes[0] & 0x07, 0x03..=0x05) {
        return Err(DataError::NotDbase);
    }
    let count = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]) as usize;
    let header = usize::from(u16::from_le_bytes([bytes[8], bytes[9]]));
    let size = usize::from(u16::from_le_bytes([bytes[10], bytes[11]]));
    if header < 33 || header > bytes.len() || size == 0 {
        return Err(DataError::NotDbase);
    }
    struct Field {
        name: String,
        kind: u8,
        length: usize,
    }
    let mut fields = Vec::new();
    let mut at = 32;
    while at + 32 <= header && bytes[at] != 0x0D {
        let raw = &bytes[at..at + 32];
        let end = raw[..11].iter().position(|byte| *byte == 0).unwrap_or(11);
        fields.push(Field {
            name: decode_windows_1252(&raw[..end]).trim().to_string(),
            kind: raw[11],
            length: usize::from(raw[16]),
        });
        at += 32;
    }
    if fields.is_empty() || fields.iter().map(|field| field.length).sum::<usize>() + 1 > size {
        return Err(DataError::NotDbase);
    }
    let mut rows = vec![fields.iter().map(|field| field.name.clone()).collect()];
    for index in 0..count {
        let start = header + index * size;
        let Some(record) = bytes.get(start..start + size) else {
            break;
        };
        if record[0] == b'*' {
            continue;
        }
        if record[0] == 0x1A {
            break;
        }
        let mut at = 1;
        let mut values = Vec::new();
        for field in &fields {
            let raw = &record[at..at + field.length];
            at += field.length;
            let text = decode_windows_1252(raw).trim().to_string();
            values.push(match field.kind {
                b'D' if text.len() == 8 && text.bytes().all(|byte| byte.is_ascii_digit()) => {
                    let year = &text[..4];
                    let month: u32 = text[4..6].parse().unwrap_or(0);
                    let day: u32 = text[6..].parse().unwrap_or(0);
                    format!("{month}/{day}/{year}")
                }
                b'L' => match text.as_str() {
                    "T" | "t" | "Y" | "y" => "T".into(),
                    "F" | "f" | "N" | "n" => "F".into(),
                    _ => String::new(),
                },
                b'M' => String::new(),
                _ => text,
            });
        }
        rows.push(values);
    }
    DataTable::from_rows(rows)
}

/// An Ami Pro data document's paragraphs for `table` (MG3): the
/// delimiters, the names record, then a record a paragraph, each ending
/// with the record delimiter; a value's line feeds become paragraph ends,
/// as Ami Pro reads them back.
pub fn data_paragraphs(table: &DataTable, delimiters: Delimiters) -> Vec<String> {
    let mut paragraphs = vec![match delimiters.record {
        Some(record) => format!("{}{}", delimiters.field, record),
        None => delimiters.field.to_string(),
    }];
    let field = delimiters.field.to_string();
    let mut push = |values: &[String]| {
        let mut record = values.join(&field);
        if let Some(end) = delimiters.record {
            record.push(end);
        }
        if delimiters.record.is_some() {
            paragraphs.extend(record.split('\n').map(str::to_string));
        } else {
            paragraphs.push(record.replace('\n', " "));
        }
    };
    push(&table.names);
    for record in &table.records {
        let mut values = record.clone();
        values.resize(table.names.len(), String::new());
        push(&values);
    }
    paragraphs
}

/// Whether `table` can be written with `delimiters`: no name or value
/// holds either delimiter.
pub fn fits_delimiters(table: &DataTable, delimiters: Delimiters) -> bool {
    delimiter_clash(table, delimiters).is_none()
}

/// The first value that holds one of `delimiters`, which Ami Pro could
/// not read back: `(None, field)` for a field name, `(Some(record), field)`
/// for a record's value.
pub fn delimiter_clash(
    table: &DataTable,
    delimiters: Delimiters,
) -> Option<(Option<usize>, usize)> {
    let holds = |text: &String| {
        text.contains(delimiters.field)
            || delimiters
                .record
                .is_some_and(|record| text.contains(record))
    };
    if let Some(field) = table.names.iter().position(holds) {
        return Some((None, field));
    }
    table
        .records
        .iter()
        .enumerate()
        .find_map(|(record, values)| {
            values
                .iter()
                .position(holds)
                .map(|field| (Some(record), field))
        })
}

/// The record editor's changes to a table (MG3); each becomes one
/// [`set_data_command`] and so one undo step.
impl DataTable {
    /// A record of empty values inserted at `at` (the end when past it).
    pub fn insert_record(&mut self, at: usize) {
        let at = at.min(self.records.len());
        self.records
            .insert(at, vec![String::new(); self.names.len()]);
    }

    pub fn remove_record(&mut self, at: usize) {
        if at < self.records.len() {
            self.records.remove(at);
        }
    }

    /// A field added after the others, empty in every record.
    pub fn add_field(&mut self, name: &str) {
        self.names.push(name.to_string());
        for record in &mut self.records {
            record.resize(self.names.len(), String::new());
        }
    }

    pub fn remove_field(&mut self, field: usize) {
        if field < self.names.len() && self.names.len() > 1 {
            self.names.remove(field);
            for record in &mut self.records {
                if field < record.len() {
                    record.remove(field);
                }
            }
        }
    }

    pub fn rename_field(&mut self, field: usize, name: &str) {
        if let Some(old) = self.names.get_mut(field) {
            *old = name.to_string();
        }
    }

    /// A field moved one place left (`-1`) or right (`1`).
    pub fn move_field(&mut self, field: usize, by: isize) {
        let to = field as isize + by;
        if field >= self.names.len() || to < 0 || to as usize >= self.names.len() {
            return;
        }
        let to = to as usize;
        self.names.swap(field, to);
        for record in &mut self.records {
            record.resize(self.names.len(), String::new());
            record.swap(field, to);
        }
    }

    pub fn set_value(&mut self, record: usize, field: usize, value: &str) {
        if let Some(values) = self.records.get_mut(record) {
            values.resize(self.names.len().max(values.len()), String::new());
            if let Some(old) = values.get_mut(field) {
                *old = value.to_string();
            }
        }
    }

    /// Sorted by a field as Ami Pro's data file Sort does: numbers by
    /// value, the rest by text without regard to case; equal values keep
    /// their order.
    pub fn sort_by(&mut self, field: usize, ascending: bool) {
        if field >= self.names.len() {
            return;
        }
        let key = |record: &Vec<String>| record.get(field).cloned().unwrap_or_default();
        self.records.sort_by(|a, b| {
            let order = crate::tables::sort_order(&key(a), &key(b));
            if ascending { order } else { order.reverse() }
        });
    }
}

/// The command that makes `document` (an Ami Pro data document) hold
/// `table` with `delimiters`, one undo step: its main text replaced with
/// [`data_paragraphs`]. `None` when nothing would change, or when a value
/// holds a delimiter (Ami Pro could not read it back).
pub fn set_data_command(
    document: &Document,
    table: &DataTable,
    delimiters: Delimiters,
) -> Option<crate::Command> {
    if !fits_delimiters(table, delimiters) {
        return None;
    }
    let text = data_paragraphs(table, delimiters).join("\n");
    let main = document.main_flow();
    let last = main.paragraphs.len().checked_sub(1)?;
    let current: Vec<&str> = main
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .collect();
    if current.join("\n") == text {
        return None;
    }
    let flow = document.main_flow_id();
    Some(crate::Command::ReplaceText {
        start: crate::TextPosition {
            flow,
            paragraph: 0,
            byte: 0,
        },
        end: crate::TextPosition {
            flow,
            paragraph: last,
            byte: main.paragraphs[last].text.len(),
        },
        text,
    })
}

/// A new Ami Pro data document with `names` and no records (New Data
/// File…, MG3), with `delimiters` (`~` and `|` by default).
pub fn new_data_document(names: &[String], delimiters: Delimiters) -> Document {
    data_document(
        &DataTable {
            names: names.to_vec(),
            records: Vec::new(),
        },
        delimiters,
    )
}

/// Ami Pro's delimited data document (kind 0) holding `table`; the caller
/// checks [`delimiter_clash`] first.
pub fn data_document(table: &DataTable, delimiters: Delimiters) -> Document {
    let mut document = Document::from_plain_text(&data_paragraphs(table, delimiters).join("\n"));
    document.title = "Data File".into();
    document
}

/// More fields than this put a table data document on a landscape page.
pub const PORTRAIT_FIELDS: usize = 5;

/// A table data document (kind 4, MR1) holding `table`: one table at the
/// start of the text, the field names in its first row (a heading row,
/// shaded, so it repeats on each page), each record a row, lines around
/// every cell, the columns filling the text width (a landscape page past
/// [`PORTRAIT_FIELDS`] fields), one empty record when there is none; rows
/// `row_height` high at least (Automatic). A value's line feeds become
/// paragraphs of its cell.
pub fn table_data_document(table: &DataTable, row_height: amipro_document::Twips) -> Document {
    use crate::tables::{CellBlock, CellData, NewTable, TableEdit, TableStyle, TableTarget};
    let mut document = Document::from_plain_text("");
    document.title = "Data File".into();
    let columns = table.names.len().max(1);
    if columns > PORTRAIT_FIELDS
        && let Some(id) = document.layouts().first().map(|layout| layout.id)
        && let Some(layout) = document.layout_mut(id)
    {
        layout.landscape = true;
        let page = &mut layout.right;
        std::mem::swap(&mut page.width, &mut page.height);
    }
    let width = document
        .layouts()
        .first()
        .map_or(9_360, |layout| layout.right.text_width());
    let rows = table.records.len().max(1) + 1;
    let flow = document.main_flow_id();
    let mut spec = NewTable::fitting(columns, rows.min(NewTable::MAX_ROWS), width, row_height);
    spec.rows = rows;
    let at = crate::TextPosition {
        flow,
        paragraph: 0,
        byte: 0,
    };
    let Ok(applied) = crate::apply(
        &document,
        &crate::Command::InsertTable {
            target: TableTarget::Text(at),
            table: spec,
        },
    ) else {
        return document;
    };
    let mut document = applied.document;
    let Some(frame) = document.frame_stack().last().copied() else {
        return document;
    };
    let style = match document.frame(frame).map(|frame| &frame.content) {
        Some(FrameContent::Table(table)) => {
            let amipro_document::CellContent::Text(cell) = table.cells[0].content;
            document
                .flow(cell)
                .and_then(|flow| flow.paragraphs.first())
                .map(|paragraph| paragraph.style)
        }
        _ => None,
    }
    .unwrap_or(amipro_document::StyleId(0));
    let cell = |document: &mut Document, value: &str| CellData {
        paragraphs: value
            .split('\n')
            .map(|line| document.new_paragraph(line, style))
            .collect(),
        format: Default::default(),
        formula: None,
    };
    let mut block_rows = vec![
        table
            .names
            .iter()
            .map(|name| cell(&mut document, name))
            .collect::<Vec<_>>(),
    ];
    for record in &table.records {
        let mut row: Vec<CellData> = record
            .iter()
            .take(columns)
            .map(|value| cell(&mut document, value))
            .collect();
        while row.len() < columns {
            row.push(cell(&mut document, ""));
        }
        block_rows.push(row);
    }
    let edits = [
        TableEdit::PasteCells {
            at: (0, 0),
            block: CellBlock {
                rows: block_rows,
                origin: (0, 0),
            },
            area: None,
            add_rows: false,
        },
        TableEdit::ApplyStyle(TableStyle::Grid),
        TableEdit::Headings {
            rows: 0..1,
            on: true,
        },
    ];
    for edit in edits {
        if let Ok(applied) = crate::apply(&document, &crate::Command::EditTable { frame, edit }) {
            document = applied.document;
        }
    }
    // The heading row shaded, as Table Style's Header row shades it.
    if let Some(slot) = document.frame_mut(frame)
        && let FrameContent::Table(grid) = &mut slot.content
    {
        let columns = grid.columns.len();
        for cell in grid.cells.iter_mut().take(columns) {
            cell.format.fill = Some(HEADING_SHADE);
        }
    }
    document
}

/// The heading row's fill in a table data document (Table Style's).
pub const HEADING_SHADE: amipro_document::Colour = 0x00E1_E1E1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_merge_engines_number_rule_is_the_vms_tdv7() {
        for (from, to) in [
            ("$1,200.00", "$1,200"),
            ("75.50", "75.5"),
            ("1234.50", "1234.5"),
            ("0.00", "0"),
            ("0.000", "0"),
            ("100.00", "100"),
            ("1.0", "1"),
            ("5.", "5"),
            ("10.50", "10.5"),
            ("$5.10", "$5.1"),
            ("1,200.00", "1,200"),
            ("1.230", "1.23"),
            ("-0.50", "-0.5"),
            ("+5.00", "+5"),
            ("00.50", "00.5"),
            ("1234567.00", "1234567"),
            ("1.5.0", "1.5"),
        ] {
            assert_eq!(merge_number_text(from), to, "{from}");
        }
        for same in [
            "007", "100", "-0", "1,200", "12", "-7.25", "3.14159", "5%", "10.00%", " 5", "1.00 ",
            "1.00abc", "abc1.00", ".50", "", "150",
        ] {
            assert_eq!(merge_number_text(same), same, "{same:?}");
        }
    }

    fn document(paragraphs: &[&str]) -> Document {
        Document::from_plain_text(&paragraphs.join("\n"))
    }

    #[test]
    fn table_data_documents_hold_the_records_as_ami_pros_table_data_files() {
        let table = DataTable {
            names: vec!["Title".into(), "Last".into(), "City".into()],
            records: vec![
                vec!["Mr.".into(), "Beck".into(), "Boston".into()],
                vec!["Ms.".into(), "Cole".into(), "Two\nlines".into()],
                vec!["Dr.".into()],
            ],
        };
        let document = table_data_document(&table, 284);
        let mut expected = table.clone();
        expected.records[2].resize(3, String::new());
        assert_eq!(read_table_data(&document).unwrap(), expected);
        document.validate().unwrap();
        let frame = document.frame_stack()[0];
        let FrameContent::Table(grid) = &document.frame(frame).unwrap().content else {
            panic!("a table");
        };
        assert_eq!(grid.rows.len(), 4);
        assert!(grid.row_options.headings.contains(&0));
        assert_eq!(grid.cells[0].format.fill, Some(HEADING_SHADE));
        assert_eq!(grid.cells[3].format.fill, None);
        assert!(!document.layouts()[0].landscape);
        // Without records: one empty record to type in.
        let empty = table_data_document(
            &DataTable {
                names: table.names.clone(),
                records: Vec::new(),
            },
            284,
        );
        assert_eq!(
            read_table_data(&empty).unwrap().records,
            vec![vec![String::new(); 3]]
        );
        // More than five fields: a landscape page, the columns filling it.
        let wide = table_data_document(
            &DataTable {
                names: (1..=6).map(|n| format!("F{n}")).collect(),
                records: Vec::new(),
            },
            284,
        );
        let layout = &wide.layouts()[0];
        assert!(layout.landscape);
        assert_eq!(layout.right.width, 15_840);
        // Back to Ami Pro's delimited form, and what cannot go there.
        let delimited = data_document(&expected, Delimiters::default());
        assert_eq!(read_ami_data(&delimited).unwrap().0, expected);
        assert_eq!(delimiter_clash(&expected, Delimiters::default()), None);
        let mut clash = expected.clone();
        clash.records[1][0] = "a~b".into();
        assert_eq!(
            delimiter_clash(&clash, Delimiters::default()),
            Some((Some(1), 0))
        );
    }

    #[test]
    fn sorting_a_tables_rows_keeps_the_heading_rows_first() {
        let table = DataTable {
            names: vec!["Name".into(), "Amount".into()],
            records: vec![
                vec!["cy".into(), "100".into()],
                vec!["Al".into(), "9".into()],
                vec!["bea".into(), "75.5".into()],
            ],
        };
        let document = table_data_document(&table, 284);
        let frame = document.frame_stack()[0];
        let sorted = |column, descending| {
            let applied = crate::apply(
                &document,
                &crate::Command::EditTable {
                    frame,
                    edit: crate::tables::TableEdit::SortRows { column, descending },
                },
            )
            .unwrap();
            read_table_data(&applied.document).unwrap()
        };
        let names = |table: DataTable| {
            table
                .records
                .iter()
                .map(|record| record[0].clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names(sorted(0, false)), ["Al", "bea", "cy"]);
        assert_eq!(names(sorted(1, false)), ["Al", "bea", "cy"]);
        assert_eq!(names(sorted(1, true)), ["cy", "bea", "Al"]);
        assert_eq!(sorted(0, true).names, table.names);
    }

    #[test]
    fn the_record_editors_changes_read_back_as_made() {
        let mut document = new_data_document(
            &["Title".into(), "Last".into(), "Amount".into()],
            Delimiters::default(),
        );
        let (mut table, delimiters) = read_ami_data(&document).unwrap();
        assert!(table.records.is_empty());
        for (index, (title, last, amount)) in [
            ("Mr.", "allen", "150"),
            ("Ms.", "Beck", "75.5"),
            ("Dr.", "Cole", "1,200"),
        ]
        .into_iter()
        .enumerate()
        {
            table.insert_record(index);
            table.set_value(index, 0, title);
            table.set_value(index, 1, last);
            table.set_value(index, 2, amount);
        }
        table.add_field("City");
        table.set_value(1, 3, "Seattle\nnorth");
        table.sort_by(2, false);
        assert_eq!(table.records[0][1], "Cole");
        table.sort_by(1, true);
        assert_eq!(table.records[0][1], "allen");
        table.move_field(3, -1);
        table.rename_field(2, "Town");
        table.remove_record(2);
        let command = set_data_command(&document, &table, delimiters).unwrap();
        document = crate::apply(&document, &command).unwrap().document;
        let (back, _) = read_ami_data(&document).unwrap();
        assert_eq!(back, table);
        assert!(set_data_command(&document, &table, delimiters).is_none());
        table.set_value(0, 0, "a~b");
        assert!(set_data_command(&document, &table, delimiters).is_none());
    }

    #[test]
    fn ami_pro_data_documents_read_as_the_shipped_one() {
        let data = document(&[
            "~|",
            "Title~First name~Last name~City, State Zip|",
            "Mr.~Terry~Allen~Reston, VA  77013|",
            "Ms.~Pam~~Seattle|",
            "",
        ]);
        let (table, delimiters) = read_ami_data(&data).unwrap();
        assert_eq!(delimiters, Delimiters::default());
        assert_eq!(table.names[1], "First name");
        assert_eq!(table.records.len(), 2);
        assert_eq!(table.value(0, "first_name"), Some("Terry"));
        assert_eq!(table.value(0, "CITY_STATE_ZIP"), Some("Reston, VA  77013"));
        // An empty field is two delimiters together; a short record's
        // missing value is empty.
        assert_eq!(table.value(1, "Last name"), Some(""));
        assert_eq!(table.value(1, "City, State Zip"), Some("Seattle"));
        assert_eq!(table.value(0, "Nothing"), None);
    }

    #[test]
    fn records_span_paragraphs_and_single_delimiters_end_at_paragraphs() {
        let data = document(&[
            "~|",
            "NAME~ADDRESS|",
            "Ann~1 Main St.",
            "Apt. 2|",
            "Bob~3 Elm|",
        ]);
        let (table, _) = read_ami_data(&data).unwrap();
        assert_eq!(table.value(0, "address"), Some("1 Main St.\nApt. 2"));
        assert_eq!(table.value(1, "NAME"), Some("Bob"));
        let single = document(&["~", "NAME~CITY", "Ann~Boston", "", "Bob~Dallas"]);
        let (table, delimiters) = read_ami_data(&single).unwrap();
        assert_eq!(delimiters.record, None);
        assert_eq!(table.records.len(), 2);
        assert_eq!(
            read_ami_data(&document(&["~|#", "A|"])),
            Err(DataError::NotValid)
        );
        assert_eq!(
            read_ami_data(&document(&["~|", "A~~B|"])).unwrap_err(),
            DataError::EmptyFieldName
        );
    }

    #[test]
    fn written_data_reads_back() {
        let table = DataTable {
            names: vec!["NAME".into(), "ADDRESS".into()],
            records: vec![
                vec!["Ann".into(), "1 Main St.\nApt. 2".into()],
                vec!["Bob".into(), String::new()],
            ],
        };
        let paragraphs = data_paragraphs(&table, Delimiters::default());
        assert_eq!(paragraphs[0], "~|");
        let written = Document::from_plain_text(&paragraphs.join("\n"));
        assert_eq!(read_ami_data(&written).unwrap().0, table);
        assert!(fits_delimiters(&table, Delimiters::default()));
        assert!(!fits_delimiters(
            &table,
            Delimiters {
                field: 'A',
                record: None
            }
        ));
    }

    #[test]
    fn field_keys_are_ami_pros() {
        assert_eq!(field_key("First name"), "FIRST_NAME");
        assert_eq!(field_key("City, State Zip"), "CITY_STATE_ZIP");
        assert_eq!(field_key("e-mail"), "EMAIL");
    }

    #[test]
    fn description_files_and_their_kinds_read() {
        let description =
            read_description(&document(&["fixed", "Last Name,10", "City,8"])).unwrap();
        assert_eq!(description.kind, DescriptionKind::Fixed);
        assert_eq!(description.names, ["Last_Name", "City"]);
        let table =
            read_fixed("Allen     Boston\r\nBeck      Seattle  \r\n", &description).unwrap();
        assert_eq!(table.value(1, "CITY"), Some("Seattle"));
        assert!(matches!(
            read_description(&document(&["FIXED", "Name,0"])),
            Err(DataError::BadWidth(_))
        ));
        assert!(matches!(
            read_description(&document(&["Lotus"])),
            Err(DataError::UnknownDescription(_))
        ));
        let comma = read_comma(
            "\"Allen, Terry\" , Boston\n  Beck,\"Say \"\"hi\"\"\"  \n",
            Some(&["NAME".into(), "CITY".into()]),
        )
        .unwrap();
        assert_eq!(comma.value(0, "NAME"), Some("Allen, Terry"));
        assert_eq!(comma.value(0, "CITY"), Some("Boston"));
        assert_eq!(comma.value(1, "CITY"), Some("Say \"hi\""));
        assert_eq!(text_of(b"A\x01B"), Err(DataError::NotText));
    }

    #[test]
    fn dif_files_read_their_tuples() {
        let dif = "TABLE\r\n0,1\r\n\"\"\r\nVECTORS\r\n0,2\r\n\"\"\r\nTUPLES\r\n0,2\r\n\"\"\r\nDATA\r\n0,0\r\n\"\"\r\n-1,0\r\nBOT\r\n1,0\r\n\"Allen\"\r\n0,150\r\nV\r\n-1,0\r\nBOT\r\n1,0\r\n\"Beck\"\r\n0,75.5\r\nV\r\n-1,0\r\nEOD\r\n";
        let table = read_dif(dif, Some(&["LAST".into(), "AMOUNT".into()])).unwrap();
        assert_eq!(table.records, [vec!["Allen", "150"], vec!["Beck", "75.5"]]);
    }

    #[test]
    fn csv_and_tsv_read_quoted_values() {
        let table = read_separated(
            "Name,Note,,\r\n\"Allen, T\",\"two\nlines\"\r\nBeck,\"\"\"q\"\"\"\r\n",
            ',',
        )
        .unwrap();
        assert_eq!(table.names, ["Name", "Note"]);
        assert_eq!(table.value(0, "NAME"), Some("Allen, T"));
        assert_eq!(table.value(0, "note"), Some("two\nlines"));
        assert_eq!(table.value(1, "note"), Some("\"q\""));
        let tsv = read_separated("A\tB\n1\t2\n", '\t').unwrap();
        assert_eq!(tsv.value(0, "b"), Some("2"));
    }

    /// A dBASE III file with a character, numeric, date, and logical field
    /// and a deleted record.
    fn dbase_file() -> Vec<u8> {
        let fields: [(&str, u8, u8); 4] = [
            ("NAME", b'C', 8),
            ("AMOUNT", b'N', 6),
            ("DUE", b'D', 8),
            ("PAID", b'L', 1),
        ];
        let header = 32 + 32 * fields.len() + 1;
        let size = 1 + fields
            .iter()
            .map(|field| usize::from(field.2))
            .sum::<usize>();
        let records = [
            (" ", "Allen", "150", "19940315", "T"),
            ("*", "Gone", "1", "19940101", "F"),
            (" ", "Beck", "75.50", "19940102", "?"),
        ];
        let mut bytes = vec![0x03, 94, 3, 15];
        bytes.extend((records.len() as u32).to_le_bytes());
        bytes.extend((header as u16).to_le_bytes());
        bytes.extend((size as u16).to_le_bytes());
        bytes.resize(32, 0);
        for (name, kind, length) in fields {
            let mut descriptor = [0u8; 32];
            descriptor[..name.len()].copy_from_slice(name.as_bytes());
            descriptor[11] = kind;
            descriptor[16] = length;
            bytes.extend(descriptor);
        }
        bytes.push(0x0D);
        for (flag, name, amount, due, paid) in records {
            bytes.extend(flag.as_bytes());
            bytes.extend(format!("{name:<8}{amount:>6}{due}{paid}").as_bytes());
        }
        bytes.push(0x1A);
        bytes
    }

    #[test]
    fn dbase_files_read_by_their_public_format() {
        let table = read_dbase(&dbase_file()).unwrap();
        assert_eq!(table.names, ["NAME", "AMOUNT", "DUE", "PAID"]);
        assert_eq!(table.records.len(), 2);
        assert_eq!(table.records[0], ["Allen", "150", "3/15/1994", "T"]);
        assert_eq!(table.records[1], ["Beck", "75.50", "1/2/1994", ""]);
        assert_eq!(
            read_dbase(b"not a dbase file at all, not at all"),
            Err(DataError::NotDbase)
        );
        let mut short = dbase_file();
        short.truncate(40);
        assert_eq!(read_dbase(&short), Err(DataError::NotDbase));
    }
}
