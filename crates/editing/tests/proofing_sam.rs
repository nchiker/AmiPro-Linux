//! The proofing run's stored forms (`docs/research/PROOFING_NOTES.md`):
//! Ami Pro's spell-checked flag `<:s>`, the document's language `[lang]`,
//! and the grammar set `[GramStyle]`, read, changed by commands, and
//! written as Ami Pro writes them (VM fixtures PSP4B, PLN1, PGR1).

use amipro_document::{Document, Language};
use amipro_editing::{Command, TextPosition, apply};
use amipro_formats::{LossKind, SamOptions, import_sam, sam_document, sam_losses};

const OPTIONS: SamOptions = SamOptions::at(1_793_000_000);

fn sam(sections: &str, edoc: &str) -> Vec<u8> {
    format!("[ver]\r\n\t4\r\n{sections}[edoc]\r\n{edoc}>\r\n\r\n").into_bytes()
}

fn read(bytes: &[u8]) -> Document {
    import_sam(bytes, None, "Proofing").unwrap().document
}

fn run(document: &Document, command: Command) -> Document {
    apply(document, &command)
        .unwrap_or_else(|error| panic!("{command:?}: {error:?}"))
        .document
}

fn written(document: &Document) -> String {
    String::from_utf8_lossy(&sam_document(document, &OPTIONS)).into_owned()
}

fn body(written: &str) -> &str {
    let start = written.find("[edoc]\r\n").unwrap() + "[edoc]\r\n".len();
    &written[start..]
}

fn section<'a>(written: &'a str, name: &str) -> &'a str {
    let header = format!("[{name}]\r\n");
    let start = written.find(&header).unwrap() + header.len();
    let end = written[start..]
        .find("\r\n[")
        .map_or(written.len(), |end| start + end);
    &written[start..end]
}

#[test]
fn a_paragraph_whose_text_changes_loses_ami_pros_checked_flag_psp4b() {
    // PSP4B: Ami Pro flags a paragraph its check passed clean.
    let document = read(&sam(
        "",
        "<:s>Clean words only.\r\n\r\n<:s>Second clean line.\r\n\r\nLast line.\r\n\r\n",
    ));
    assert!(body(&written(&document)).starts_with("<:s>Clean words only.\r\n\r\n<:s>Second"));

    let main = document.main_flow_id();
    let typed = run(
        &document,
        Command::ReplaceText {
            start: TextPosition {
                flow: main,
                paragraph: 0,
                byte: 5,
            },
            end: TextPosition {
                flow: main,
                paragraph: 0,
                byte: 5,
            },
            text: "er".into(),
        },
    );
    let text = written(&typed);
    // The edited paragraph is checked again by Ami Pro; the other keeps it.
    assert!(body(&text).starts_with("Cleaner words only.\r\n\r\n<:s>Second clean line."));

    // A style change leaves the text as checked.
    let styled = run(
        &document,
        Command::SetParagraphAlignment {
            flow: main,
            first: 1,
            last: 1,
            alignment: Some(amipro_document::Alignment::Centre),
        },
    );
    assert!(written(&styled).contains("<:s>Second clean line."));
    // Splitting a flagged paragraph changes both halves' text.
    let split = run(
        &document,
        Command::ReplaceText {
            start: TextPosition {
                flow: main,
                paragraph: 1,
                byte: 6,
            },
            end: TextPosition {
                flow: main,
                paragraph: 1,
                byte: 6,
            },
            text: "\n".into(),
        },
    );
    assert!(!body(&written(&split)).contains("<:s>Second"));
}

#[test]
fn the_language_and_grammar_set_are_read_and_written_as_ami_pro_does_pln1_pgr1() {
    let document = read(&sam(
        "[lang]\r\n\t2\r\n[GramStyle]\r\n\tBusiness Writing\r\n",
        "Colour.\r\n\r\n",
    ));
    assert_eq!(*document.language(), Language::ami_pro(2));
    assert_eq!(document.language().locale(), Some("en_GB"));
    assert_eq!(document.grammar_set(), "Business Writing");
    let text = written(&document);
    assert_eq!(section(&text, "lang"), "\t2");
    assert_eq!(section(&text, "GramStyle"), "\tBusiness Writing");

    // A new document is American with no set chosen, as Ami Pro's.
    let new = Document::default();
    let text = written(&new);
    assert_eq!(section(&text, "lang"), "\t1");
    assert_eq!(section(&text, "GramStyle"), "\t");

    // Changed by commands, written, and read back with nothing lost.
    let changed = run(&new, Command::SetLanguage(Language::ami_pro(27)));
    let changed = run(&changed, Command::SetGrammarSet("All Rules".into()));
    let bytes = sam_document(&changed, &OPTIONS);
    let copy = read(&bytes);
    assert_eq!(*copy.language(), Language::ami_pro(27));
    assert_eq!(copy.grammar_set(), "All Rules");
    assert!(sam_losses(&changed, &OPTIONS).unwrap().is_empty());
}

#[test]
fn a_language_ami_pro_has_no_number_for_is_written_as_the_nearest_and_listed() {
    let document = run(
        &Document::default(),
        Command::SetLanguage(Language::from_locale("fr_BE")),
    );
    let bytes = sam_document(&document, &OPTIONS);
    assert_eq!(section(&String::from_utf8_lossy(&bytes), "lang"), "\t3");
    let losses = sam_losses(&document, &OPTIONS).unwrap();
    assert_eq!(
        losses.iter().map(|loss| loss.kind).collect::<Vec<_>>(),
        [LossKind::Language]
    );
    // The native format keeps it.
    let native = Document::decode_native(&document.encode_native().unwrap()).unwrap();
    assert_eq!(native.language().locale(), Some("fr_BE"));
    assert_eq!(native.language().number, 3);
}
