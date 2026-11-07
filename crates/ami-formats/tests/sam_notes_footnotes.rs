//! Notes, footnotes, and Footnote Options read and written as Ami Pro 3.1
//! writes them (`docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`; the shapes of
//! VM fixtures RNN2, RNN3, RNF1, RNF4).

use amipro_document::{FootnoteOptions, InlineObject};
use amipro_formats::{SamOptions, import_sam, sam_document};

const OPTIONS: SamOptions = SamOptions::at(1_793_000_000);

fn body(edoc: &str) -> String {
    format!("[ver]\r\n\t4\r\n[edoc]\r\n{edoc}>\r\n\r\n")
}

/// The `[edoc]` the writer gives `sam`, without the body's closing `>`
/// line and what follows it.
fn written_body(sam: &str) -> String {
    let document = import_sam(sam.as_bytes(), None, "Notes").unwrap().document;
    let written = String::from_utf8_lossy(&sam_document(&document, &OPTIONS)).into_owned();
    let start = written.find("[edoc]\r\n").unwrap() + "[edoc]\r\n".len();
    let rest = &written[start..];
    let end = rest.find("[Embedded]").unwrap_or(rest.len());
    rest[..end]
        .strip_suffix(">\r\n\r\n")
        .or_else(|| rest[..end].strip_suffix(">\r\n"))
        .unwrap()
        .to_owned()
}

#[test]
fn notes_are_objects_with_their_fields_and_paragraphs_rnn2_rnn3() {
    let edoc = "One two<:N1793037315,LS,65535,1,1\r\nPara one.\r\n\r\nPara two bold end.\r\n\r\n>\r\n three.<:N1793037329,LS,65535,2,1\r\nSecond note.\r\n\r\n>\r\n<:N1793037368,QX,255,3,1\r\nRed note.\r\n\r\n>\r\n\r\n";
    let sam = body(edoc);
    let document = import_sam(sam.as_bytes(), None, "Notes").unwrap().document;
    let paragraph = &document.paragraphs()[0];
    assert_eq!(paragraph.text, "One two\u{FFFC} three.\u{FFFC}\u{FFFC}");
    let notes: Vec<_> = paragraph
        .objects
        .iter()
        .map(|object| match object {
            InlineObject::Note { note, flow } => (note.clone(), document.flow(*flow).unwrap()),
            other => panic!("expected a note, found {other:?}"),
        })
        .collect();
    assert_eq!(notes[0].0.initials.as_deref(), Some("LS"));
    assert_eq!(notes[0].0.colour, Some(65_535));
    assert_eq!(notes[0].1.paragraphs.len(), 2);
    assert_eq!(notes[1].0.number, Some(2));
    assert_eq!(notes[2].0.colour, Some(255));
    assert_eq!(notes[2].0.initials.as_deref(), Some("QX"));
    assert_eq!(notes[2].1.paragraphs[0].text, "Red note.");
    // Ami Pro's bytes back, the main flag worked out from where they stand.
    assert_eq!(written_body(&sam), edoc);
}

#[test]
fn short_notes_and_notes_outside_the_main_text_keep_their_form() {
    let edoc = "Old <:N671837138\r\nAn old note.\r\n\r\n>\r\nnote.\r\n\r\n";
    let sam = body(edoc);
    let document = import_sam(sam.as_bytes(), None, "Notes").unwrap().document;
    let InlineObject::Note { note, .. } = &document.paragraphs()[0].objects[0] else {
        panic!("expected a note");
    };
    assert_eq!(
        (note.time, note.initials.as_ref(), note.number),
        (671_837_138, None, None)
    );
    assert_eq!(written_body(&sam), edoc);
}

#[test]
fn footnotes_own_their_paragraphs_rnf1() {
    let edoc = "Main text with a footnote here.<:F\r\n@Footnote@The footnote text.\r\n\r\n>\r\n\r\nSecond paragraph.\r\n\r\n";
    let sam = body(edoc);
    let document = import_sam(sam.as_bytes(), None, "Footnotes")
        .unwrap()
        .document;
    let InlineObject::Footnote { flow } = &document.paragraphs()[0].objects[0] else {
        panic!("expected a footnote");
    };
    let footnote = &document.flow(*flow).unwrap().paragraphs[0];
    assert_eq!(footnote.text, "The footnote text.");
    assert_eq!(document.styles()[&footnote.style].name, "Footnote");
    assert_eq!(written_body(&sam), edoc);
}

#[test]
fn footnote_options_are_the_documents_rnf4() {
    let sam = format!(
        "[ver]\r\n\t4\r\n[fopts]\r\n\t188\r\n\t1\r\n\t2880\r\n\t720\r\n\tContinued\r\n\tFrom before\r\n{}",
        &body("Text.\r\n\r\n")["[ver]\r\n\t4\r\n".len()..]
    );
    let document = import_sam(sam.as_bytes(), None, "Options")
        .unwrap()
        .document;
    let options = document.footnote_options();
    assert_eq!(
        *options,
        FootnoteOptions {
            flags: 188,
            start: 1,
            length: 2_880,
            indent: 720,
            continued_on: "Continued".into(),
            continued_from: "From before".into(),
        }
    );
    assert!(
        options.has(FootnoteOptions::CUSTOM_LENGTH) && options.has(FootnoteOptions::REPEAT_NUMBER)
    );
    assert_eq!(options.continued_on_message(), Some("Continued"));
    assert!(!options.endnotes() && !options.reset_each_page());
    let written = String::from_utf8_lossy(&sam_document(&document, &OPTIONS)).into_owned();
    assert!(written.contains(
        "[fopts]\r\n\t188\r\n\t1\r\n\t2880\r\n\t720\r\n\tContinued\r\n\tFrom before\r\n"
    ));
    // An older file's four numbers read as Ami Pro's defaults for the rest,
    // and a new document writes Ami Pro 3.1's six values.
    let old = sam.replace("\tContinued\r\n\tFrom before\r\n", "");
    let document = import_sam(old.as_bytes(), None, "Options")
        .unwrap()
        .document;
    assert_eq!(document.footnote_options().continued_on, "");
    let written = String::from_utf8_lossy(&sam_document(
        &import_sam(body("Text.\r\n\r\n").as_bytes(), None, "New")
            .unwrap()
            .document,
        &OPTIONS,
    ))
    .into_owned();
    assert!(written.contains("[fopts]\r\n\t0\r\n\t1\r\n\t0\r\n\t0\r\n\t\r\n\t\r\n"));
}

#[test]
fn bookmarks_are_read_in_ami_pros_three_forms_rnb1_rnb3() {
    use amipro_document::bookmarks::{BookmarkTarget, bookmarks, cell_bookmarks};
    let sam = format!(
        "[ver]\r\n\t4\r\n[book]\r\n\tincell 3 2 0\r\n{}",
        &body(
            "Alpha <:Zbmone>beta <:Z~bmone>gamma.\r\n\r\nSecond par<:ZMixTwo><:Z~MixTwo>a.\r\n\r\n"
        )["[ver]\r\n\t4\r\n".len()..]
    );
    let document = import_sam(sam.as_bytes(), None, "Bookmarks")
        .unwrap()
        .document;
    assert_eq!(cell_bookmarks(&document)[0].name, "incell");
    let found = bookmarks(&document);
    let names: Vec<_> = found
        .iter()
        .map(|bookmark| bookmark.name.as_str())
        .collect();
    assert_eq!(names, ["bmone", "incell", "MixTwo"]);
    let BookmarkTarget::Text { start, end } = &found[0].target else {
        panic!("a text bookmark");
    };
    assert_eq!((start.byte, end.unwrap().byte), (6, 11));
    // Written back where they stood.
    let written = String::from_utf8_lossy(&sam_document(&document, &OPTIONS)).into_owned();
    assert!(written.contains("[book]\r\n\tincell 3 2 0\r\n"));
    assert!(written.contains("Alpha <:Zbmone>beta <:Z~bmone>gamma."));
}
