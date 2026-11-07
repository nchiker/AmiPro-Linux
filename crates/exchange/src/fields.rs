//! Ami Pro's fields as the formats' own (EX8): dates, Doc Info entries,
//! page numbers, and the power fields whose function the formats share,
//! each with Ami Pro's current result (`docs/research/FIELDS_NOTES.md`).

use amipro_document::{DateField, DocInfoEntry, DocInfoField, PowerMark, power_kind as kinds};

use crate::{DateSource, FieldKind, NumberFormat};

/// Word's date picture for Ami Pro's date format `letter` (`1308:0619`;
/// US order and separators, as the app shows dates), and whether it shows
/// capitals.
pub fn date_picture(letter: u8) -> (&'static str, bool) {
    let picture = match letter | 0x20 {
        b'b' => "MMMM d, yyyy",
        b'c' => "d MMMM yyyy",
        b'd' => "dddd, MMMM d, yyyy",
        b'e' => "MMMM d",
        b'f' => "dddd MMMM d",
        b'g' => "MM/dd",
        b'h' => "MM/dd/yyyy",
        b'i' => "d. MMMM",
        b'j' => "d. MMMM yyyy",
        b'k' => "yyyy MMMM d",
        b'l' => "MMMM, yyyy",
        _ => "MM/dd/yy",
    };
    (picture, letter.is_ascii_uppercase())
}

/// The field a date object is (Insert Date/Time).
pub fn date_kind(field: &DateField) -> FieldKind {
    let source = match field.source {
        amipro_document::DateSource::Today => DateSource::Now,
        amipro_document::DateSource::LastSaved => DateSource::Saved,
        amipro_document::DateSource::Created => DateSource::Created,
        amipro_document::DateSource::Time => {
            return FieldKind::Time {
                source: DateSource::Now,
            };
        }
    };
    let (picture, upper) = date_picture(field.format);
    FieldKind::Date {
        source,
        picture: picture.to_owned(),
        upper,
    }
}

/// The field a Doc Info object is, or `None` for an entry the formats do
/// not have (its result is written as text).
pub fn doc_info_kind(field: &DocInfoField, names: &[String; 8]) -> Option<FieldKind> {
    Some(match DocInfoEntry::from_code(field.code)? {
        DocInfoEntry::FileName => FieldKind::FileName { path: false },
        DocInfoEntry::Path => return None,
        DocInfoEntry::StyleSheet => FieldKind::Template,
        DocInfoEntry::DateCreated => {
            let (picture, upper) = date_picture(field.format);
            FieldKind::Date {
                source: DateSource::Created,
                picture: picture.to_owned(),
                upper,
            }
        }
        DocInfoEntry::DateLastRevised => {
            let (picture, upper) = date_picture(field.format);
            FieldKind::Date {
                source: DateSource::Saved,
                picture: picture.to_owned(),
                upper,
            }
        }
        DocInfoEntry::TimeCreated => FieldKind::Time {
            source: DateSource::Created,
        },
        DocInfoEntry::TimeLastRevised => FieldKind::Time {
            source: DateSource::Saved,
        },
        DocInfoEntry::TotalRevisions => FieldKind::Revisions,
        DocInfoEntry::Description => FieldKind::Description,
        DocInfoEntry::Field(index) => {
            FieldKind::Custom(custom_name(names, usize::from(index.min(7))))
        }
        DocInfoEntry::EditingTime => FieldKind::EditingTime,
        DocInfoEntry::Pages => FieldKind::NumPages,
        DocInfoEntry::Words => FieldKind::Words,
        DocInfoEntry::Characters => FieldKind::Characters,
        DocInfoEntry::Size => return None,
    })
}

/// A user field's name as a property: its own, or Ami Pro's default.
pub fn custom_name(names: &[String; 8], index: usize) -> String {
    match names.get(index) {
        Some(name) if !name.trim().is_empty() => name.trim().to_owned(),
        _ => format!("Field{}", index + 1),
    }
}

/// The field a power field's code is, where the formats have one: its
/// function's name before any argument or format (`NumPages`, `CreateDate
/// %Db`, `EditDate`, `Now() %T2`, `FileName`, `Today`), case as Ami Pro
/// ignores it. Others keep their result as text.
pub fn power_kind(mark: &PowerMark) -> Option<FieldKind> {
    if mark.kind != kinds::GENERAL {
        return None;
    }
    let code = mark.code.trim();
    let name: String = code
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '$')
        .collect::<String>()
        .to_ascii_lowercase();
    let rest = code[name.len()..].trim_start_matches("()").trim();
    // Only a bare function and its format: anything else (arguments,
    // operators) is worked out by Ami Pro's language and kept as text.
    let format = rest.strip_prefix('%');
    if !rest.is_empty() && format.is_none() {
        return None;
    }
    let date = |source| match format {
        Some(format) if format.starts_with('T') || format.starts_with('t') => {
            FieldKind::Time { source }
        }
        Some(format) => {
            let letter = format.as_bytes().get(1).copied().unwrap_or(b'a');
            let (picture, upper) = date_picture(letter);
            FieldKind::Date {
                source,
                picture: picture.to_owned(),
                upper,
            }
        }
        None => {
            let (picture, upper) = date_picture(b'a');
            FieldKind::Date {
                source,
                picture: picture.to_owned(),
                upper,
            }
        }
    };
    Some(match name.as_str() {
        "numpages" => FieldKind::NumPages,
        "createdate" => date(DateSource::Created),
        "editdate" => date(DateSource::Saved),
        "now" | "today" => date(DateSource::Now),
        "filename" => FieldKind::FileName { path: false },
        "description" => FieldKind::Description,
        "keywords" => FieldKind::Keywords,
        "totaleditingtime" => FieldKind::EditingTime,
        "totalrevisions" => FieldKind::Revisions,
        _ => return None,
    })
}

/// Ami Pro's page-number style character as a number format.
pub fn page_format(style: char) -> NumberFormat {
    match style {
        'I' => NumberFormat::UpperRoman,
        'i' => NumberFormat::LowerRoman,
        'A' => NumberFormat::UpperLetter,
        'a' => NumberFormat::LowerLetter,
        _ => NumberFormat::Decimal,
    }
}

/// Word's field instruction for `kind` (DOCX and RTF share them).
pub fn instruction(kind: &FieldKind) -> String {
    let format_switch = |format: NumberFormat| match format {
        NumberFormat::UpperRoman => " \\* ROMAN",
        NumberFormat::LowerRoman => " \\* roman",
        NumberFormat::UpperLetter => " \\* ALPHABETIC",
        NumberFormat::LowerLetter => " \\* alphabetic",
        _ => " \\* Arabic",
    };
    match kind {
        FieldKind::Page { format } => format!("PAGE{}", format_switch(*format)),
        FieldKind::NumPages => "NUMPAGES".into(),
        FieldKind::Date {
            source,
            picture,
            upper,
        } => {
            let name = match source {
                DateSource::Now => "DATE",
                DateSource::Created => "CREATEDATE",
                DateSource::Saved => "SAVEDATE",
            };
            let upper = if *upper { " \\* Upper" } else { "" };
            format!("{name} \\@ \"{picture}\"{upper}")
        }
        FieldKind::Time { source } => {
            let name = match source {
                DateSource::Now => "TIME",
                DateSource::Created => "CREATEDATE",
                DateSource::Saved => "SAVEDATE",
            };
            format!("{name} \\@ \"h:mm am/pm\"")
        }
        FieldKind::Title => "TITLE".into(),
        FieldKind::Description => "COMMENTS".into(),
        FieldKind::Keywords => "KEYWORDS".into(),
        FieldKind::Custom(name) => format!("DOCPROPERTY \"{}\"", name.replace('"', "'")),
        FieldKind::FileName { path } => {
            if *path {
                "FILENAME \\p".into()
            } else {
                "FILENAME".into()
            }
        }
        FieldKind::Template => "TEMPLATE".into(),
        FieldKind::Revisions => "REVNUM".into(),
        FieldKind::EditingTime => "EDITTIME".into(),
        FieldKind::Words => "NUMWORDS".into(),
        FieldKind::Characters => "NUMCHARS".into(),
        FieldKind::Seq { name } => format!("SEQ {}", sequence_name(name)),
        FieldKind::IndexEntry { primary, secondary } => {
            let quote = |text: &str| text.replace('"', "'").replace(':', "\\:");
            if secondary.is_empty() {
                format!("XE \"{}\"", quote(primary))
            } else {
                format!("XE \"{}:{}\"", quote(primary), quote(secondary))
            }
        }
        FieldKind::ContentsEntry { text, level } => {
            format!(
                "TC \"{}\" \\l {}",
                text.replace('"', "'"),
                level.clamp(&1, &9)
            )
        }
    }
}

/// An index or contents entry's mark as a field (`<:X8,…;Index "a" #
/// "b">`, `<:X11,…;TOC 1 "a">`; `docs/research/FIELDS_NOTES.md`).
pub fn entry_kind(mark: &PowerMark) -> Option<FieldKind> {
    let quoted: Vec<&str> = mark.code.split('"').skip(1).step_by(2).collect();
    match mark.kind {
        kinds::INDEX => Some(FieldKind::IndexEntry {
            primary: quoted.first()?.trim().to_owned(),
            secondary: quoted
                .get(1)
                .map_or_else(String::new, |text| text.trim().to_owned()),
        }),
        kinds::TOC => {
            let level = mark
                .code
                .split_whitespace()
                .nth(1)
                .and_then(|level| level.parse().ok())
                .unwrap_or(1);
            Some(FieldKind::ContentsEntry {
                text: quoted.first()?.trim().to_owned(),
                level,
            })
        }
        _ => None,
    }
}

/// A sequence's name as Word takes it: letters, digits, and underscores,
/// starting with a letter.
pub fn sequence_name(code: &str) -> String {
    let name: String = code
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect();
    match name.chars().next() {
        Some(first) if first.is_ascii_alphabetic() => name,
        _ => format!("Seq{name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(code: &str) -> PowerMark {
        PowerMark {
            end: false,
            kind: kinds::GENERAL,
            value: 0,
            code: code.into(),
        }
    }

    #[test]
    fn power_fields_with_a_counterpart_become_the_formats_fields() {
        assert_eq!(power_kind(&mark("NumPages")), Some(FieldKind::NumPages));
        assert_eq!(
            power_kind(&mark("CreateDate %Db")),
            Some(FieldKind::Date {
                source: DateSource::Created,
                picture: "MMMM d, yyyy".into(),
                upper: false,
            })
        );
        assert_eq!(
            power_kind(&mark("Now() %T2")),
            Some(FieldKind::Time {
                source: DateSource::Now
            })
        );
        assert_eq!(
            power_kind(&mark("EditDate %DD")),
            Some(FieldKind::Date {
                source: DateSource::Saved,
                picture: "dddd, MMMM d, yyyy".into(),
                upper: true,
            })
        );
        // Ami Pro's language works these out; their results stay text.
        assert_eq!(power_kind(&mark("NumPages + 1")), None);
        assert_eq!(power_kind(&mark("FileSize")), None);
    }

    #[test]
    fn instructions_are_words() {
        assert_eq!(
            instruction(&FieldKind::Page {
                format: NumberFormat::UpperRoman
            }),
            "PAGE \\* ROMAN"
        );
        assert_eq!(
            instruction(&FieldKind::Custom("Client".into())),
            "DOCPROPERTY \"Client\""
        );
        assert_eq!(sequence_name("Figure"), "Figure");
        let index = PowerMark {
            end: false,
            kind: kinds::INDEX,
            value: 0,
            code: "Index \"memory\" # \"extended\"".into(),
        };
        assert_eq!(
            instruction(&entry_kind(&index).unwrap()),
            "XE \"memory:extended\""
        );
        assert_eq!(sequence_name("1st"), "Seq1st");
    }
}
