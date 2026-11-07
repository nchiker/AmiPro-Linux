//! Doc Info in Ami Pro's files: `[desc]` and `[fldnames]` (`11c8:342b`
//! writes them, `1058:2732` and `1058:267c` read them;
//! `docs/research/FIELDS_NOTES.md`, "Doc Info").

use amipro_document::{Document, DocumentInfo, Statistics};

use crate::sheet::section_values;

pub(crate) const DESC: &str = "section [desc]";
pub(crate) const FIELD_NAMES: &str = "section [fldnames]";
pub(crate) const FOOTNOTE_OPTIONS: &str = "section [fopts]";
pub(crate) const LINE_NUMBERING: &str = "section [lnopts]";

/// Moves `[desc]` and `[fldnames]`, kept as read, into the document's Doc
/// Info, leaving their places empty so that the writers keep the sections'
/// order. The importer calls it; so should anything that opens a native
/// file written before the fields run, which kept the two sections raw.
pub fn take_doc_info(document: &mut Document) {
    let raw = |key: &str| {
        document
            .legacy()
            .values
            .iter()
            .find(|value| value.key.eq_ignore_ascii_case(key))
            .map(|value| value.value.clone())
            .filter(|value| !value.is_empty())
    };
    let (description, names) = (raw(DESC), raw(FIELD_NAMES));
    if description.is_none() && names.is_none() {
        return;
    }
    let mut info = document.info().clone();
    if let Some(raw) = description {
        read_description(&section_values(&raw), &mut info);
    }
    if let Some(raw) = names {
        for (index, name) in section_values(&raw).iter().take(8).enumerate() {
            info.field_names[index] = crate::text::decode_name(name);
        }
    }
    *document.info_mut() = info;
    for value in &mut document.legacy_mut().values {
        if value.key.eq_ignore_ascii_case(DESC) || value.key.eq_ignore_ascii_case(FIELD_NAMES) {
            value.value.clear();
        }
    }
}

/// Moves `[fopts]` from the kept sections into the document's Footnote
/// Options (`1058:228d`): flags, the starting number, the separator's
/// custom length and indent, and the two continued messages; older files
/// have only the four numbers, and a missing value is Ami Pro's default.
pub fn take_footnote_options(document: &mut Document) {
    let Some(raw) = document
        .legacy()
        .values
        .iter()
        .find(|value| value.key.eq_ignore_ascii_case(FOOTNOTE_OPTIONS))
        .map(|value| value.value.clone())
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let values = section_values(&raw);
    let number = |index: usize| {
        values
            .get(index)
            .and_then(|value| value.trim().parse::<i64>().ok())
    };
    let defaults = amipro_document::FootnoteOptions::default();
    let options = amipro_document::FootnoteOptions {
        flags: number(0).map_or(defaults.flags, |flags| flags as u8),
        start: number(1).map_or(defaults.start, |start| start.clamp(0, 0xFFFF) as u16),
        length: number(2).map_or(0, |length| length.clamp(0, 0xFFFF) as i32),
        indent: number(3).map_or(0, |indent| indent.clamp(0, 0xFFFF) as i32),
        continued_on: values.get(4).cloned().unwrap_or_default(),
        continued_from: values.get(5).cloned().unwrap_or_default(),
    };
    document.set_footnote_options(options);
    for value in &mut document.legacy_mut().values {
        if value.key.eq_ignore_ascii_case(FOOTNOTE_OPTIONS) {
            value.value.clear();
        }
    }
}

/// Reads `[lnopts]` into the document's Line Numbering (`1058:06cc`;
/// `docs/research/COMMANDS_NOTES.md` §8): the flag byte, the "Based on"
/// style, and the count, which is 1 when the line is missing (47 files of
/// the user's corpus). The section stays kept as read, so that a document
/// whose numbering is unchanged writes it back as it was
/// ([`line_numbering_values`]); a file without it has Ami Pro's default.
pub fn take_line_numbering(document: &mut Document) {
    let Some(values) = line_numbering_kept(document) else {
        return;
    };
    if let Some(numbering) = parse_line_numbering(&values) {
        document.set_line_numbering(numbering);
    }
}

fn line_numbering_kept(document: &Document) -> Option<Vec<String>> {
    document
        .legacy()
        .values
        .iter()
        .find(|value| value.key.eq_ignore_ascii_case(LINE_NUMBERING))
        .map(|value| section_values(&value.value))
}

fn parse_line_numbering(values: &[String]) -> Option<amipro_document::LineNumbering> {
    let flags = values.first()?.trim().parse::<i64>().ok()?;
    let defaults = amipro_document::LineNumbering::default();
    Some(amipro_document::LineNumbering {
        flags: flags as u8,
        style: values
            .get(1)
            .map(|name| crate::text::decode_name(name))
            .unwrap_or(defaults.style),
        interval: values
            .get(2)
            .and_then(|value| value.trim().parse::<i64>().ok())
            .map_or(1, |count| count.clamp(0, 0xFFFF) as u16),
    })
}

/// `[lnopts]`'s three values as Ami Pro 3.1 writes them (`11c8:1c7b`), or
/// the section as read when that reads as the document's Line Numbering.
pub(crate) fn line_numbering_values(document: &Document) -> Vec<String> {
    let numbering = document.line_numbering();
    if let Some(kept) = line_numbering_kept(document)
        && parse_line_numbering(&kept).as_ref() == Some(numbering)
    {
        return kept;
    }
    vec![
        numbering.flags.to_string(),
        escaped(&numbering.style),
        numbering.interval.to_string(),
    ]
}

/// Moves `[lang]` and `[GramStyle]` from the kept sections into the
/// document's language and grammar set (`docs/research/PROOFING_NOTES.md`,
/// "Languages"; `1058:06cc`). A `[lang]` that is not a number stays kept and
/// is written back as read.
pub fn take_proofing(document: &mut Document) {
    let value = |document: &Document, key: &str| {
        document
            .legacy()
            .values
            .iter()
            .find(|value| value.key.eq_ignore_ascii_case(key))
            .map(|value| section_values(&value.value))
    };
    if let Some(number) = value(document, LANGUAGE).and_then(|values| {
        values
            .first()
            .and_then(|value| value.trim().parse::<u16>().ok())
    }) {
        document.set_language(amipro_document::Language::ami_pro(number));
    }
    if let Some(set) = value(document, GRAMMAR_SET) {
        document.set_grammar_set(set.first().cloned().unwrap_or_default());
    }
}

/// The keys of `[toc]` and `[master]` among the kept sections.
pub(crate) const TOC: &str = "section [toc]";
pub(crate) const MASTER: &str = "section [master]";
pub(crate) const SEQUENCE: &str = "section [sequence]";
pub(crate) const RECFILE: &str = "section [recfile]";

/// Moves `[toc]` and `[master]` from the kept sections into the document's
/// contents settings (`docs/research/TOC_INDEX_NOTES.md`, Part A, sections
/// 1–2). A section that is not in Ami Pro's form stays kept and is written
/// back as read; one taken is left as an empty value in its place, so the
/// unknown sections around it keep their order.
pub fn take_contents(document: &mut Document) {
    let mut settings = document.contents().clone();
    let mut merge = document.merge().clone();
    for value in &mut document.legacy_mut().values {
        if value.key.eq_ignore_ascii_case(TOC) && settings.toc.is_none() {
            if let Some(toc) =
                amipro_document::contents::TocSettings::from_values(&section_values(&value.value))
            {
                settings.toc = Some(toc);
                value.value.clear();
            }
        } else if value.key.eq_ignore_ascii_case(MASTER) && settings.master.is_none() {
            if let Some(master) = amipro_document::contents::MasterSettings::from_values(
                &section_values(&value.value),
            ) {
                settings.master = Some(master);
                value.value.clear();
            }
        } else if value.key.eq_ignore_ascii_case(RECFILE) && merge.record_file.is_none() {
            if let Some(file) =
                amipro_document::merge::RecordFile::from_values(&section_values(&value.value))
            {
                merge.record_file = Some(file);
                value.value.clear();
            }
        } else if value.key.eq_ignore_ascii_case(SEQUENCE) && settings.sequence.is_none() {
            if let Some(sequence) =
                amipro_document::contents::Sequence::from_values(&section_values(&value.value))
            {
                settings.sequence = Some(sequence);
                value.value.clear();
            }
        }
    }
    document.set_contents(settings);
    document.set_merge(merge);
}

/// A kept or typed contents section's values: the model's when it has the
/// section, else the kept values (none for a taken one).
pub(crate) fn contents_values(
    document: &Document,
    name: &str,
    kept: Option<Vec<String>>,
) -> Option<Vec<String>> {
    let typed = match name {
        "toc" => document.contents().toc.as_ref().map(|toc| toc.values()),
        "master" => document
            .contents()
            .master
            .as_ref()
            .map(|master| master.values()),
        "sequence" => document
            .contents()
            .sequence
            .as_ref()
            .map(|sequence| sequence.values()),
        "recfile" => document
            .merge()
            .record_file
            .as_ref()
            .map(|file| file.values()),
        _ => None,
    };
    typed.or_else(|| kept.filter(|values| values.iter().any(|value| !value.is_empty())))
}

/// `[lang]`'s value: the document's Ami Pro language number, or the kept
/// value when it was not a number and the language is as read.
pub(crate) fn language_values(document: &Document, kept: Option<Vec<String>>) -> Vec<String> {
    let parses = |values: &[String]| {
        values
            .first()
            .is_some_and(|value| value.trim().parse::<u16>().is_ok())
    };
    match kept {
        Some(values)
            if !parses(&values) && *document.language() == amipro_document::Language::default() =>
        {
            values
        }
        _ => vec![document.language().number.to_string()],
    }
}

pub(crate) const LANGUAGE: &str = "section [lang]";
pub(crate) const GRAMMAR_SET: &str = "section [GramStyle]";

/// `[fopts]`'s six values as Ami Pro 3.1 writes them (`11c8:1c7b`).
pub(crate) fn footnote_option_values(options: &amipro_document::FootnoteOptions) -> Vec<String> {
    vec![
        options.flags.to_string(),
        options.start.to_string(),
        options.length.to_string(),
        options.indent.to_string(),
        options.continued_on.clone(),
        options.continued_from.clone(),
    ]
}

/// `[desc]`'s values as Ami Pro's reader takes them (`1058:2732`). Older
/// files have 8 values (the description, four fields, the save time, the
/// revisions, the creation time); what they lack is zero or empty.
fn read_description(values: &[String], info: &mut DocumentInfo) {
    let text = |index: usize| {
        values
            .get(index)
            .map(|value| crate::text::decode_name(value))
            .unwrap_or_default()
    };
    let number = |index: usize| values.get(index).map(|value| number(value));
    info.description = text(0);
    for field in 0..4 {
        info.fields[field] = text(1 + field);
    }
    info.saved = number(5);
    // The reader counts the open (`1058:2732`).
    info.revisions = number(6).unwrap_or(0).saturating_add(1);
    info.created = number(7);
    info.editing_minutes = number(8).unwrap_or(0);
    info.statistics = Statistics {
        pages: number(9).unwrap_or(0),
        words: number(10).unwrap_or(0),
        characters: number(11).unwrap_or(0),
        size: number(12).unwrap_or(0),
        size_with_links: number(13).unwrap_or(0),
    };
    info.keywords = text(14);
    info.annotation_lock = text(15);
    for field in 4..8 {
        info.fields[field] = text(12 + field);
    }
    info.flags = number(20).unwrap_or(0);
}

/// A number as Ami Pro's `atol` reads one into its 32-bit value.
fn number(value: &str) -> u32 {
    value
        .trim()
        .parse::<i64>()
        .map_or(0, |number| number as u32)
}

fn escaped(text: &str) -> String {
    crate::decode_windows_1252(&crate::escape_text(text))
}

/// `[desc]`'s 21 values for `info` as Ami Pro's writer writes them
/// (`11c8:342b`), with the times and counts as they stand; the writers set
/// the save's own. A document without Doc Info is a new one, created now.
pub(crate) fn description_values(info: &DocumentInfo, now: u32) -> Vec<String> {
    let new;
    let info = if *info == DocumentInfo::default() {
        new = DocumentInfo::new(now);
        &new
    } else {
        info
    };
    let statistics = &info.statistics;
    let mut values = vec![escaped(&info.description)];
    values.extend(info.fields[..4].iter().map(|field| escaped(field)));
    values.extend(
        [
            info.saved.unwrap_or(now),
            info.revisions.max(1),
            info.created.unwrap_or(now),
            info.editing_minutes,
            statistics.pages,
            statistics.words,
            statistics.characters,
            statistics.size,
            statistics.size_with_links,
        ]
        .map(|number| number.to_string()),
    );
    values.push(escaped(&info.keywords));
    values.push(escaped(&info.annotation_lock));
    values.extend(info.fields[4..].iter().map(|field| escaped(field)));
    values.push(info.flags.to_string());
    values
}

/// `[fldnames]`'s eight names, the defaults for unnamed fields.
pub(crate) fn field_name_values(info: &DocumentInfo) -> Vec<String> {
    (0..8)
        .map(|index| escaped(&info.field_name(index)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_sam;

    fn sam(desc: &str, names: &str) -> String {
        format!(
            "[ver]\r\n\t4\r\n[fldnames]\r\n{names}[desc]\r\n{desc}[fopts]\r\n\t0\r\n\
             [tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             [edoc]\r\nText\r\n\r\n>\r\n"
        )
    }

    fn tabbed(values: &[&str]) -> String {
        values
            .iter()
            .map(|value| format!("\t{value}\r\n"))
            .collect()
    }

    #[test]
    fn doc_info_reads_as_ami_pro_reads_it_fd2b() {
        // VM fixture FD2B's [desc] and [fldnames] (Field1 renamed Client).
        let desc = tabbed(&[
            "My description",
            "Alpha",
            "Beta",
            "F3",
            "F4",
            "1792521120",
            "1",
            "1792520631",
            "8",
            "1",
            "6",
            "23",
            "4",
            "4",
            "kw one, kw two",
            "",
            "F5",
            "F6",
            "F7",
            "F8",
            "1",
        ]);
        let names = tabbed(&[
            "Client", "Field2", "Field3", "Field4", "Field5", "Field6", "Field7", "Field8",
        ]);
        let document = import_sam(sam(&desc, &names).as_bytes(), None, "D")
            .unwrap()
            .document;
        let info = document.info();
        assert_eq!(info.description, "My description");
        assert_eq!(info.keywords, "kw one, kw two");
        assert_eq!(
            info.fields,
            ["Alpha", "Beta", "F3", "F4", "F5", "F6", "F7", "F8"].map(String::from)
        );
        assert_eq!(info.field_name(0), "Client");
        assert_eq!(info.saved, Some(1_792_521_120));
        assert_eq!(info.created, Some(1_792_520_631));
        // Opening counts a revision, as Doc Info shows it (FD5: stored 1,
        // shown 2).
        assert_eq!(info.revisions, 2);
        assert_eq!(info.editing_minutes, 8);
        assert_eq!(
            info.statistics,
            Statistics {
                pages: 1,
                words: 6,
                characters: 23,
                size: 4,
                size_with_links: 4,
            }
        );
        assert!(info.runs_frame_macros());
        // The sections keep their places, empty.
        let legacy = &document.legacy().values;
        let place = |key: &str| legacy.iter().position(|value| value.key == key);
        assert!(place(FIELD_NAMES) < place(DESC));
        assert!(
            legacy
                .iter()
                .all(|value| !value.key.contains("desc") || value.value.is_empty())
        );
        // Written back as read, but the save's own values.
        let mut values = description_values(info, 0);
        assert_eq!(values.len(), 21);
        values[6] = "1".into();
        assert_eq!(
            values.join("\n"),
            desc.replace("\r\n", "\n").replace('\t', "").trim_end()
        );
        assert_eq!(field_name_values(info)[0], "Client");
    }

    #[test]
    fn older_shorter_doc_info_reads_with_ami_pros_defaults() {
        // The 8-value form of writers before 3.1: no statistics, keywords,
        // or flags (Run frame macros off).
        let desc = tabbed(&["Old", "a", "b", "c", "d", "700", "3", "600"]);
        let document = import_sam(sam(&desc, "").as_bytes(), None, "D")
            .unwrap()
            .document;
        let info = document.info();
        assert_eq!(info.fields[3], "d");
        assert_eq!(
            (info.saved, info.revisions, info.created),
            (Some(700), 4, Some(600))
        );
        assert_eq!(info.statistics, Statistics::default());
        assert!(!info.runs_frame_macros());
        assert_eq!(info.field_name(7), "Field8");
    }

    #[test]
    fn a_new_document_is_created_now_with_ami_pros_defaults() {
        let values = description_values(&DocumentInfo::default(), 99);
        assert_eq!(
            values,
            [
                "", "", "", "", "", "99", "1", "99", "0", "0", "0", "0", "0", "0", "", "", "", "",
                "", "", "1"
            ]
        );
    }

    #[test]
    fn escaped_text_reads_and_writes_back() {
        let desc = tabbed(&["A <<b<;> and <\\i>", "", "", "", "", "1", "1", "1"]);
        let document = import_sam(sam(&desc, "").as_bytes(), None, "D")
            .unwrap()
            .document;
        assert_eq!(document.info().description, "A <b> and \u{e9}");
        assert_eq!(
            description_values(document.info(), 0)[0],
            "A <<b> and <\\i>"
        );
    }
}
