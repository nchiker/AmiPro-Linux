//! Ami Pro's glossary data files (`docs/research/TOC_INDEX_NOTES.md`, Part
//! C; VM fixture GLO0): an ordinary SAM document whose text is a merge
//! records file. The first line holds the field delimiter and the record
//! delimiter, the second the two field names, and then each record is
//! `name<field>text<record>`, its text raw SAM body text (formatting and
//! styles kept). After the text Ami Pro keeps a hidden index of the records
//! (`[gloidx]`), which it rebuilds when it is missing.

use amipro_document::{Document, decode_windows_1252, windows_1252};

/// Names are cut at this many characters (Mark Glossary Record's box and
/// the index, `14e8:0a77`).
pub const NAME_LIMIT: usize = 27;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GlossaryError {
    /// "Not an Ami Pro document" (3119).
    NotAmiPro,
    /// "Cannot do that command to an encrypted file" (3148).
    Encrypted,
    /// "Data file is not in valid format" (3005).
    Format,
    /// "Glossary name already in use." (3165).
    NameInUse,
    /// A record needs a name and text.
    Empty,
}

impl std::fmt::Display for GlossaryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NotAmiPro => "Not an Ami Pro document",
            Self::Encrypted => "Cannot do that command to an encrypted file",
            Self::Format => "Data file is not in valid format",
            Self::NameInUse => "Glossary name already in use.",
            Self::Empty => "A glossary record needs a name and text",
        })
    }
}

/// One record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlossaryRecord {
    pub name: String,
    /// The text as SAM body text, Windows-1252 bytes as stored.
    pub text: Vec<u8>,
    /// Where the record's paragraph starts in the file.
    pub offset: usize,
}

/// A glossary data file as read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Glossary {
    pub field: u8,
    /// `None`: a record ends at its paragraph's end.
    pub record: Option<u8>,
    /// The field names of the header row (`id`, `text`).
    pub field_names: Vec<String>,
    pub records: Vec<GlossaryRecord>,
}

impl Glossary {
    /// The record named `name`, without regard to case (`14e8:0422`).
    pub fn find(&self, name: &str) -> Option<&GlossaryRecord> {
        self.records
            .iter()
            .find(|record| record.name.eq_ignore_ascii_case(name))
    }

    /// The names as Insert Glossary Record lists them: sorted, `***` for
    /// an empty one.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .records
            .iter()
            .map(|record| {
                if record.name.is_empty() {
                    "***".to_string()
                } else {
                    record.name.clone()
                }
            })
            .collect();
        names.sort_by_key(|name| name.to_lowercase());
        names
    }
}

/// Where the body text starts (after `[edoc]` and its line end) and where
/// the final `>` line starts, or `None`.
fn body_bounds(bytes: &[u8]) -> Option<(usize, usize)> {
    let start = find(bytes, b"[edoc]\r\n", 0).map(|at| at + 8)?;
    // The text ends at its first lone `>` line, as a SAM body does; what
    // follows (`[Embedded]`, the hidden index, another `>` with an index
    // Ami Pro added on choosing the file) is not text (GLO0).
    let mut end = None;
    let mut from = start;
    while let Some(at) = find(bytes, b">\r\n", from) {
        if at == start || bytes[at - 1] == b'\n' {
            end = Some(at);
            break;
        }
        from = at + 1;
    }
    Some((start, end?))
}

fn find(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if from > haystack.len() {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|at| at + from)
}

/// Checks the file is a SAM document Ami Pro could use.
fn check(bytes: &[u8]) -> Result<(), GlossaryError> {
    let first = bytes.split(|byte| *byte == b'\n').next().unwrap_or(b"");
    let first = String::from_utf8_lossy(first);
    match first.trim() {
        "[ver]" => Ok(()),
        "[encrypt]" => Err(GlossaryError::Encrypted),
        _ => Err(GlossaryError::NotAmiPro),
    }
}

/// Reads a glossary data file (`13a8:*`, the merge reader).
pub fn read_glossary(bytes: &[u8]) -> Result<Glossary, GlossaryError> {
    check(bytes)?;
    let (start, end) = body_bounds(bytes).ok_or(GlossaryError::Format)?;
    let body = &bytes[start..end];
    let line_end = find(body, b"\r\n", 0).unwrap_or(body.len());
    let delimiters = &body[..line_end];
    let (field, record) = match delimiters {
        [field] => (*field, None),
        [field, record] => (*field, Some(*record)),
        _ => return Err(GlossaryError::Format),
    };
    // Records (the header row first): up to the record delimiter, or a
    // paragraph's end; each starts after the blank lines before it.
    let mut pieces: Vec<(usize, &[u8])> = Vec::new();
    let mut at = line_end;
    loop {
        while body[at..].starts_with(b"\r\n") {
            at += 2;
        }
        if at >= body.len() {
            break;
        }
        let stop = match record {
            Some(record) => body[at..].iter().position(|byte| *byte == record),
            None => find(body, b"\r\n\r\n", at).map(|stop| stop - at),
        };
        let Some(length) = stop else {
            // The rest without a delimiter: a last record, if it has text.
            let rest = &body[at..];
            if rest.iter().any(|byte| !byte.is_ascii_whitespace()) {
                pieces.push((start + at, trim_end(rest)));
            }
            break;
        };
        pieces.push((start + at, &body[at..at + length]));
        at += length + usize::from(record.is_some());
    }
    let mut pieces = pieces.into_iter();
    let (_, header) = pieces.next().ok_or(GlossaryError::Format)?;
    let field_names = header
        .split(|byte| *byte == field)
        .map(|name| decode_windows_1252(trim_end(name)))
        .collect();
    let records = pieces
        .map(|(offset, piece)| {
            let split = piece.iter().position(|byte| *byte == field);
            let (name, text) = match split {
                Some(split) => (&piece[..split], &piece[split + 1..]),
                None => (piece, &[][..]),
            };
            GlossaryRecord {
                name: decode_windows_1252(name),
                text: text.to_vec(),
                offset,
            }
        })
        .collect();
    Ok(Glossary {
        field,
        record,
        field_names,
        records,
    })
}

fn trim_end(bytes: &[u8]) -> &[u8] {
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(0, |at| at + 1);
    &bytes[..end]
}

/// A record's text as a document (`13b0:2786`: the text with `>` after it
/// read as a SAM body, under the data file's own sections and styles).
pub fn record_document(bytes: &[u8], record: &GlossaryRecord) -> Option<Document> {
    let (start, _) = body_bounds(bytes)?;
    let mut sam = bytes[..start].to_vec();
    sam.extend_from_slice(&record.text);
    sam.extend_from_slice(b"\r\n\r\n>\r\n\r\n");
    crate::import_sam(&sam, None, "Glossary")
        .ok()
        .map(|imported| imported.document)
}

/// A document's text as SAM body text, for a record (`11b0:1405`): the
/// paragraphs as Ami Pro writes them, without the blank line after the
/// last.
pub fn fragment_text(document: &Document) -> Vec<u8> {
    let written = crate::sam_document(document, &crate::SamOptions::at(0));
    let Some((start, end)) = body_bounds(&written) else {
        return Vec::new();
    };
    trim_end(&written[start..end]).to_vec()
}

/// The hidden index Ami Pro keeps after the text (`14e8:0a77`): each
/// record's name (27 characters, the first of equal names) and offset,
/// sorted without regard to case, then the index's own offset.
fn index_footer(glossary: &Glossary, at: usize) -> Vec<u8> {
    let mut entries: Vec<(String, usize)> = Vec::new();
    for record in &glossary.records {
        let name: String = record.name.chars().take(NAME_LIMIT).collect();
        if !entries
            .iter()
            .any(|(other, _)| other.eq_ignore_ascii_case(&name))
        {
            entries.push((name, record.offset));
        }
    }
    entries.sort_by_key(|entry| entry.0.to_lowercase());
    let mut out = format!("[gloidx] {}\r\n", entries.len()).into_bytes();
    for (name, offset) in entries {
        out.extend(name.chars().map(windows_1252));
        out.extend(format!(" {offset}\r\n").bytes());
    }
    out.extend(format!("{at:08}\r\n").bytes());
    out
}

/// Mark Glossary Record (`14e8:115b`): `text` (SAM body text) added as
/// the record `name` after the last, then the hidden index; the file's
/// bytes before its text's end kept as they were.
pub fn append_record(bytes: &[u8], name: &str, text: &[u8]) -> Result<Vec<u8>, GlossaryError> {
    let glossary = read_glossary(bytes)?;
    let name: String = name.trim().chars().take(NAME_LIMIT).collect();
    if name.is_empty() || text.is_empty() {
        return Err(GlossaryError::Empty);
    }
    if glossary.find(&name).is_some() {
        return Err(GlossaryError::NameInUse);
    }
    let (_, end) = body_bounds(bytes).ok_or(GlossaryError::Format)?;
    let mut out = bytes[..end].to_vec();
    let offset = out.len();
    out.extend(name.chars().map(windows_1252));
    out.push(glossary.field);
    out.extend_from_slice(text);
    if let Some(record) = glossary.record {
        out.push(record);
    }
    out.extend_from_slice(b"\r\n\r\n>\r\n");
    let mut written = glossary;
    written.records.push(GlossaryRecord {
        name,
        text: text.to_vec(),
        offset,
    });
    let at = out.len();
    out.extend(index_footer(&written, at));
    Ok(out)
}

/// A new glossary data file, as Mark Glossary Record makes one for a
/// missing file: `sheet`'s sections up to its text, then `~|` and
/// `id~text|`.
pub fn new_glossary(sheet: &[u8]) -> Vec<u8> {
    let start = find(sheet, b"[edoc]\r\n", 0).map_or_else(
        || {
            let mut header = sheet.to_vec();
            if !header.is_empty() && !header.ends_with(b"\r\n") {
                header.extend_from_slice(b"\r\n");
            }
            header.extend_from_slice(b"[edoc]\r\n");
            header
        },
        |at| sheet[..at + 8].to_vec(),
    );
    let mut out = if start.starts_with(b"[ver]") {
        start
    } else {
        let mut with_version = b"[ver]\r\n\t4\r\n".to_vec();
        with_version.extend(start);
        with_version
    };
    out.extend_from_slice(b"~|\r\n\r\nid~text|\r\n\r\n>\r\n");
    out
}

/// The file with its records replaced by `records` (name, text), the
/// sections, delimiters, and header row kept, then the hidden index: the
/// panel's Rename and Delete, which Ami Pro does not have (TX7).
pub fn rewrite_records(
    bytes: &[u8],
    records: &[(String, Vec<u8>)],
) -> Result<Vec<u8>, GlossaryError> {
    let glossary = read_glossary(bytes)?;
    let (start, _) = body_bounds(bytes).ok_or(GlossaryError::Format)?;
    // Everything to the first record: the delimiter line and header row.
    let first = glossary
        .records
        .first()
        .map(|record| record.offset)
        .unwrap_or_else(|| body_bounds(bytes).map_or(start, |(_, end)| end));
    let mut out = bytes[..first].to_vec();
    let mut written = Glossary {
        records: Vec::new(),
        ..glossary
    };
    for (name, text) in records {
        let offset = out.len();
        out.extend(name.chars().map(windows_1252));
        out.push(written.field);
        out.extend_from_slice(text);
        if let Some(record) = written.record {
            out.push(record);
        }
        out.extend_from_slice(b"\r\n\r\n");
        written.records.push(GlossaryRecord {
            name: name.clone(),
            text: text.clone(),
            offset,
        });
    }
    out.extend_from_slice(b">\r\n");
    let at = out.len();
    out.extend(index_footer(&written, at));
    Ok(out)
}

/// Renames the record `old` (refused when `new` is in use by another).
pub fn rename_record(bytes: &[u8], old: &str, new: &str) -> Result<Vec<u8>, GlossaryError> {
    let glossary = read_glossary(bytes)?;
    let new: String = new.trim().chars().take(NAME_LIMIT).collect();
    if new.is_empty() {
        return Err(GlossaryError::Empty);
    }
    if glossary.records.iter().any(|record| {
        record.name.eq_ignore_ascii_case(&new) && !record.name.eq_ignore_ascii_case(old)
    }) {
        return Err(GlossaryError::NameInUse);
    }
    let records: Vec<(String, Vec<u8>)> = glossary
        .records
        .iter()
        .map(|record| {
            let name = if record.name.eq_ignore_ascii_case(old) {
                new.clone()
            } else {
                record.name.clone()
            };
            (name, record.text.clone())
        })
        .collect();
    rewrite_records(bytes, &records)
}

/// Deletes the record `name`.
pub fn delete_record(bytes: &[u8], name: &str) -> Result<Vec<u8>, GlossaryError> {
    let glossary = read_glossary(bytes)?;
    let records: Vec<(String, Vec<u8>)> = glossary
        .records
        .iter()
        .filter(|record| !record.name.eq_ignore_ascii_case(name))
        .map(|record| (record.name.clone(), record.text.clone()))
        .collect();
    rewrite_records(bytes, &records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(body: &str) -> Vec<u8> {
        format!("[ver]\r\n\t4\r\n[edoc]\r\n{body}").into_bytes()
    }

    #[test]
    fn records_are_read_with_their_names_text_and_offsets_glo0() {
        let bytes = file("~|\r\n\r\nid~text|\r\n\r\nhi~Hello <+!>there<-!>.|\r\n\r\n>\r\n\r\n");
        let glossary = read_glossary(&bytes).unwrap();
        assert_eq!((glossary.field, glossary.record), (b'~', Some(b'|')));
        assert_eq!(glossary.field_names, ["id", "text"]);
        assert_eq!(glossary.records.len(), 1);
        let record = glossary.find("HI").unwrap();
        assert_eq!(record.text, b"Hello <+!>there<-!>.");
        assert_eq!(&bytes[record.offset..record.offset + 3], b"hi~");
        let document = record_document(&bytes, record).unwrap();
        assert_eq!(document.paragraphs()[0].text, "Hello there.");
        assert_eq!(
            document.paragraphs().len(),
            1,
            "{:?}",
            document
                .paragraphs()
                .iter()
                .map(|p| p.text.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(document.paragraphs()[0].runs.len(), 1, "the bold kept");
    }

    #[test]
    fn a_record_is_appended_with_ami_pros_index_glo0() {
        let bytes = file("~|\r\n\r\nid~text|\r\n\r\nhi~Hello <+!>there<-!>.|\r\n\r\n>\r\n");
        let out = append_record(&bytes, "sy", b"Sincerely yours, Ami.").unwrap();
        let text = String::from_utf8(out.clone()).unwrap();
        let body = &text[text.find("[edoc]").unwrap()..];
        let hi = text.find("hi~").unwrap();
        let sy = text.find("sy~").unwrap();
        let index = text.find("[gloidx]").unwrap();
        assert_eq!(
            body,
            format!(
                "[edoc]\r\n~|\r\n\r\nid~text|\r\n\r\nhi~Hello <+!>there<-!>.|\r\n\r\nsy~Sincerely yours, Ami.|\r\n\r\n>\r\n[gloidx] 2\r\nhi {hi}\r\nsy {sy}\r\n{index:08}\r\n"
            )
        );
        // Read again: both records, the index ignored; a name in use and
        // a new one after it.
        let glossary = read_glossary(&out).unwrap();
        assert_eq!(glossary.names(), ["hi", "sy"]);
        assert_eq!(
            append_record(&out, "SY", b"x"),
            Err(GlossaryError::NameInUse)
        );
        let again = append_record(&out, "zz", b"Last.").unwrap();
        assert_eq!(read_glossary(&again).unwrap().records.len(), 3);
    }

    #[test]
    fn a_new_glossary_file_is_ami_pros() {
        let made =
            new_glossary(b"[ver]\r\n\t4\r\n[sty]\r\n\t_DEFAULT.STY\r\n[edoc]\r\nIgnored\r\n>\r\n");
        assert!(made.ends_with(b"[edoc]\r\n~|\r\n\r\nid~text|\r\n\r\n>\r\n"));
        assert!(read_glossary(&made).unwrap().records.is_empty());
        let added = append_record(&made, "a", b"A.").unwrap();
        assert_eq!(read_glossary(&added).unwrap().records[0].name, "a");
        assert_eq!(
            read_glossary(b"[encrypt]\r\n"),
            Err(GlossaryError::Encrypted)
        );
        assert_eq!(read_glossary(b"hello"), Err(GlossaryError::NotAmiPro));
        assert_eq!(
            read_glossary(&file("abc\r\n\r\n>\r\n")),
            Err(GlossaryError::Format)
        );
    }

    #[test]
    fn records_are_renamed_and_deleted_keeping_the_rest() {
        let bytes = file("~|\r\n\r\nid~text|\r\n\r\nhi~Hello.|\r\n\r\nsy~Yours.|\r\n\r\n>\r\n");
        let renamed = rename_record(&bytes, "HI", "hello").unwrap();
        let glossary = read_glossary(&renamed).unwrap();
        assert_eq!(glossary.names(), ["hello", "sy"]);
        assert_eq!(glossary.find("hello").unwrap().text, b"Hello.");
        assert_eq!(
            rename_record(&bytes, "hi", "SY"),
            Err(GlossaryError::NameInUse)
        );
        let deleted = delete_record(&renamed, "sy").unwrap();
        assert_eq!(read_glossary(&deleted).unwrap().names(), ["hello"]);
        let text = String::from_utf8(deleted).unwrap();
        assert!(text.contains("id~text|\r\n\r\nhello~Hello.|\r\n\r\n>\r\n[gloidx] 1\r\n"));
    }

    #[test]
    fn paragraph_end_records_and_fragments() {
        let bytes = file("|\r\n\r\nID|Text\r\n\r\nfirst|One.\r\n\r\nsecond|Two.\r\n\r\n>\r\n");
        let glossary = read_glossary(&bytes).unwrap();
        assert_eq!(glossary.record, None);
        assert_eq!(glossary.names(), ["first", "second"]);
        let document = Document::from_plain_text("Line one.\nLine two.");
        assert_eq!(fragment_text(&document), b"Line one.\r\n\r\nLine two.");
    }
}
