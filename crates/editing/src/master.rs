//! Master documents (`docs/research/MASTER_NOTES.md`; `UI_SPEC.md` §37):
//! the file list as Ami Pro stores it, finding the files (MS2), Treat as
//! Single Document, the Seq values a member passes on, and the checks of
//! numbering the set would get wrong (MS9).

use amipro_document::contents::{ContentsSettings, Counter, MasterSettings};
use amipro_document::{Document, InlineObject, PageNumberField};

use crate::Command;
use crate::field_language::seq_name;

/// A file's name in a stored path, DOS or Linux (`l:\out\md2\ma.sam` is
/// `ma.sam`).
pub fn file_name(stored: &str) -> &str {
    stored.rsplit(['\\', '/']).next().unwrap_or(stored)
}

/// How a new member's path is stored: Ami Pro keeps full DOS paths
/// lower-cased (`1428:12ec`); the app keeps a Linux path as it is, because
/// Linux names have case.
pub fn stored_path(path: &str) -> String {
    let dos = path.len() > 1 && path.as_bytes()[1] == b':' || path.contains('\\');
    if dos {
        path.to_lowercase()
    } else {
        path.to_string()
    }
}

/// Which of `names` (the entries of the master's folder) is the stored
/// file, when its own path is not found (MS2): the one of the same name,
/// without case. `None` when none or more than one matches.
pub fn by_name<'n>(stored: &str, names: impl IntoIterator<Item = &'n str>) -> Option<&'n str> {
    let wanted = file_name(stored);
    let mut found = names
        .into_iter()
        .filter(|name| name.eq_ignore_ascii_case(wanted));
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// The command that gives the document the file list `files` (Ami Pro's
/// Master Document OK, command 0x11c): a document without `[master]` gets
/// one with empty output names; an empty list keeps the output names.
pub fn set_files(document: &Document, files: Vec<String>) -> Command {
    let mut contents = document.contents().clone();
    let master = contents.master.get_or_insert_with(MasterSettings::default);
    master.files = files;
    Command::SetContents(contents)
}

/// The command that sets the master's output files and separators (Master
/// Document Options, commands 0x11b and 0x11d).
pub fn set_outputs(document: &Document, toc: &str, index: &str, separators: bool) -> Command {
    let mut contents = document.contents().clone();
    let master = contents.master.get_or_insert_with(MasterSettings::default);
    master.toc_file = toc.to_string();
    master.index_file = index.to_string();
    if separators {
        master.flags |= MasterSettings::SEPARATORS;
    } else {
        master.flags &= !MasterSettings::SEPARATORS;
    }
    Command::SetContents(contents)
}

/// Treat as Single Document (Ami Pro's Yes to "Should it be considered a
/// single document?", callback 0x117): the member drops its `[sequence]`
/// and any `[master]`, so its numbers start from 1 again.
pub fn treat_as_single(document: &Document) -> Command {
    Command::SetContents(ContentsSettings {
        master: None,
        sequence: None,
        ..document.contents().clone()
    })
}

/// Whether the document is a member: it carries a `[sequence]`.
pub fn is_member(document: &Document) -> bool {
    document.contents().sequence.is_some()
}

/// Whether the document lists files.
pub fn is_master(document: &Document) -> bool {
    document
        .contents()
        .master
        .as_ref()
        .is_some_and(MasterSettings::is_master)
}

/// The highest value each Seq field name reaches in the main text, as a
/// master run collects them for the next member (`1640:0af7`).
pub fn seq_counters(document: &Document) -> Vec<Counter> {
    let mut counters: Vec<Counter> = Vec::new();
    for paragraph in document.paragraphs() {
        for object in &paragraph.objects {
            let InlineObject::Seq(field) = object else {
                continue;
            };
            let name = seq_name(&field.code);
            let value = field.number();
            match counters
                .iter_mut()
                .find(|counter| counter.name.eq_ignore_ascii_case(&name))
            {
                Some(counter) => counter.value = counter.value.max(value),
                None => counters.push(Counter { value, name }),
            }
        }
    }
    counters
}

/// What would make a member's numbers wrong in the set, as the guide warns
/// (chapter 26): a page-number start or leading text, a footnote starting
/// number, a TOC break that restarts the page numbers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Check {
    /// A page-number field starts on a page or with a number.
    PageNumberStart,
    /// A page-number field has leading text, which its TOC and index pages
    /// then show.
    LeadingText,
    /// Footnote Options sets a starting number other than 1.
    FootnoteStart(u16),
    /// A TOC's page break starts the page numbers again.
    NumbersRestart,
}

/// The checks of `document` as a member (MS9).
pub fn checks(document: &Document) -> Vec<Check> {
    let mut found = Vec::new();
    let mut add = |check: Check| {
        if !found.contains(&check) {
            found.push(check);
        }
    };
    for (_, flow) in document.flows() {
        for paragraph in &flow.paragraphs {
            for object in &paragraph.objects {
                match object {
                    InlineObject::PageNumber(PageNumberField {
                        start_page,
                        start_number,
                        leading_text,
                        ..
                    }) => {
                        if *start_page != 0 || *start_number != 0 {
                            add(Check::PageNumberStart);
                        }
                        if !leading_text.is_empty() {
                            add(Check::LeadingText);
                        }
                    }
                    InlineObject::Break { restart: true, .. } => add(Check::NumbersRestart),
                    _ => {}
                }
            }
        }
    }
    let start = document.footnote_options().start;
    if start != 1 {
        add(Check::FootnoteStart(start));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply;
    use amipro_document::contents::Sequence;

    #[test]
    fn stored_paths_and_names_are_found_as_ami_pro_keeps_them() {
        assert_eq!(file_name("l:\\out\\md2\\ma.sam"), "ma.sam");
        assert_eq!(file_name("/home/a/Book/ch1.sam"), "ch1.sam");
        assert_eq!(stored_path("L:\\OUT\\MA.SAM"), "l:\\out\\ma.sam");
        assert_eq!(stored_path("/home/a/Ch1.sam"), "/home/a/Ch1.sam");
        let names = ["MA.SAM", "mb.sam", "Notes.txt"];
        assert_eq!(by_name("c:\\docs\\ma.sam", names), Some("MA.SAM"));
        assert_eq!(by_name("c:\\docs\\mc.sam", names), None);
        assert_eq!(
            by_name("x\\ma.sam", ["ma.sam", "MA.SAM"]),
            None,
            "ambiguous"
        );
    }

    #[test]
    fn the_list_treat_as_single_and_checks_are_each_one_command() {
        let document = Document::from_plain_text("one\ntwo");
        let listed = apply(&document, &set_files(&document, vec!["l:\\a.sam".into()]))
            .unwrap()
            .document;
        assert!(is_master(&listed));
        let named = apply(&listed, &set_outputs(&listed, "toc.sam", "idx.sam", true))
            .unwrap()
            .document;
        let master = named.contents().master.as_ref().unwrap();
        assert_eq!(
            (master.toc_file.as_str(), master.separators()),
            ("toc.sam", true)
        );
        assert_eq!(master.files.len(), 1);
        let mut member = Document::from_plain_text("one");
        member.set_contents(ContentsSettings {
            sequence: Some(Sequence {
                page_base: 2,
                ..Sequence::default()
            }),
            ..ContentsSettings::default()
        });
        assert!(is_member(&member));
        let single = apply(&member, &treat_as_single(&member)).unwrap().document;
        assert!(!is_member(&single));
        assert!(checks(&document).is_empty());
    }
}
