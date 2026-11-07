//! Notes, bookmarks, footnotes, and Footnote Options as commands (the notes
//! run; `docs/Design/UI_SPEC.md` §34): each one step, Ami Pro's rules, and
//! written as Ami Pro writes them (read back equal).

use amipro_document::bookmarks::{BookmarkTarget, bookmarks, cell_bookmarks};
use amipro_document::{Document, FlowId, FootnoteOptions, FrameId, InlineObject, NoteInfo};
use amipro_editing::references::{NESTED, NewBookmark, note_text, notes};
use amipro_editing::tables::{NewTable, TableTarget};
use amipro_editing::{Command, TextPosition, apply};
use amipro_formats::roundtrip::{differences, is_writer_addition};
use amipro_formats::{SamOptions, import_sam, sam_document};

const OPTIONS: SamOptions = SamOptions::at(1_793_000_000);

fn run(document: &Document, command: Command) -> Document {
    apply(document, &command)
        .unwrap_or_else(|error| panic!("{command:?}: {error:?}"))
        .document
}

fn at(flow: FlowId, paragraph: usize, byte: usize) -> TextPosition {
    TextPosition {
        flow,
        paragraph,
        byte,
    }
}

fn main_at(document: &Document, paragraph: usize, byte: usize) -> TextPosition {
    at(document.main_flow_id(), paragraph, byte)
}

/// What differs once `document` is written and read back, beyond what the
/// writer adds.
fn round_trip(document: &Document) -> Vec<String> {
    let written = sam_document(document, &OPTIONS);
    let copy = import_sam(&written, None, "Edited").unwrap();
    differences(document, &copy.document)
        .into_iter()
        .filter(|difference| {
            !is_writer_addition(difference)
                && !matches!(difference.aspect.as_str(), "face kinds" | "line heights")
        })
        .map(|difference| format!("{difference:?}"))
        .collect()
}

fn note(initials: &str, colour: u32) -> NoteInfo {
    NoteInfo {
        time: 1_793_037_315,
        initials: Some(initials.into()),
        colour: Some(colour),
        number: None,
    }
}

#[test]
fn notes_insert_change_and_go_with_ami_pros_numbers_rnn2() {
    let document = Document::from_plain_text("One two three.");
    let document = run(
        &document,
        Command::InsertNote {
            at: main_at(&document, 0, "One two".len()),
            note: note("LS", NoteInfo::YELLOW),
            text: "Para one.\nPara two.".into(),
        },
    );
    let document = run(
        &document,
        Command::InsertNote {
            at: main_at(&document, 0, 0),
            note: note("QX", 255),
            text: "First now.".into(),
        },
    );
    let found = notes(&document);
    assert_eq!(found.len(), 2);
    // Numbered in text order, whichever was made first.
    assert_eq!(found[0].1.initials.as_deref(), Some("QX"));
    assert_eq!(found[0].1.number, Some(1));
    assert_eq!(found[1].1.number, Some(2));
    assert_eq!(note_text(&document, found[1].2), "Para one.\nPara two.");
    assert!(
        round_trip(&document).is_empty(),
        "{:?}",
        round_trip(&document)
    );
    let written = String::from_utf8_lossy(&sam_document(&document, &OPTIONS)).into_owned();
    assert!(written.contains("<:N1793037315,QX,255,1,1\r\nFirst now.\r\n\r\n>\r\n"));
    // Edited: the text and the colour, the fields kept.
    let mut changed = found[1].1.clone();
    changed.colour = Some(32_768);
    let document = run(
        &document,
        Command::SetNote {
            at: found[1].0,
            note: changed,
            text: Some("Rewritten.".into()),
        },
    );
    let found = notes(&document);
    assert_eq!(note_text(&document, found[1].2), "Rewritten.");
    assert_eq!(found[1].1.colour, Some(32_768));
    // Removed: the other renumbered; then Remove All Notes.
    let one = run(
        &document,
        Command::RemoveNotes {
            at: Some(found[0].0),
        },
    );
    assert_eq!(notes(&one)[0].1.number, Some(1));
    let none = run(&document, Command::RemoveNotes { at: None });
    assert!(notes(&none).is_empty());
    assert_eq!(none.paragraphs()[0].text, "One two three.");
    none.validate().unwrap();
}

#[test]
fn text_bookmarks_are_ami_pros_marks_and_cannot_nest_rnb1() {
    let document = Document::from_plain_text("Alpha beta gamma delta.");
    let document = run(
        &document,
        Command::AddBookmark {
            name: "bmone".into(),
            target: NewBookmark::Text {
                start: main_at(&document, 0, 6),
                end: main_at(&document, 0, 11),
            },
        },
    );
    let written = String::from_utf8_lossy(&sam_document(&document, &OPTIONS)).into_owned();
    assert!(written.contains("Alpha <:Zbmone>beta <:Z~bmone>gamma"));
    assert!(round_trip(&document).is_empty());
    // Inside it: refused, as Ami Pro refuses it.
    let error = apply(
        &document,
        &Command::AddBookmark {
            name: "inner".into(),
            target: NewBookmark::Text {
                start: main_at(&document, 0, 8),
                end: main_at(&document, 0, 8),
            },
        },
    )
    .unwrap_err();
    assert_eq!(error, NESTED);
    // A name in another case is taken.
    assert!(
        apply(
            &document,
            &Command::AddBookmark {
                name: "BMONE".into(),
                target: NewBookmark::Text {
                    start: main_at(&document, 0, 0),
                    end: main_at(&document, 0, 0),
                },
            },
        )
        .is_err()
    );
    let renamed = run(
        &document,
        Command::ChangeBookmark {
            name: "BMone".into(),
            to: Some("second".into()),
        },
    );
    assert_eq!(bookmarks(&renamed)[0].name, "second");
    let removed = run(
        &renamed,
        Command::ChangeBookmark {
            name: "second".into(),
            to: None,
        },
    );
    assert!(bookmarks(&removed).is_empty());
    assert_eq!(removed.paragraphs()[0].text, "Alpha beta gamma delta.");
}

fn table_document() -> (Document, FrameId) {
    let document = Document::from_plain_text("Before table.");
    let made = apply(
        &document,
        &Command::InsertTable {
            target: TableTarget::Text(main_at(&document, 0, "Before table.".len())),
            table: NewTable::fitting(3, 3, 9_360, 284),
        },
    )
    .unwrap()
    .document;
    let frame = *made.frame_stack().last().unwrap();
    (made, frame)
}

#[test]
fn cell_and_frame_bookmarks_are_stored_as_ami_pro_stores_them_rnb2_rnb3() {
    let (document, frame) = table_document();
    let document = run(
        &document,
        Command::AddBookmark {
            name: "cellbm".into(),
            target: NewBookmark::Cells {
                frame,
                row: 0,
                column: 1,
                rows: 2,
                columns: 2,
            },
        },
    );
    let document = run(
        &document,
        Command::AddBookmark {
            name: "incell".into(),
            target: NewBookmark::Cell {
                frame,
                row: 2,
                column: 0,
            },
        },
    );
    let document = run(
        &document,
        Command::AddBookmark {
            name: "frbm".into(),
            target: NewBookmark::Frame(frame),
        },
    );
    assert_eq!(cell_bookmarks(&document)[0].row, 2);
    let kinds: Vec<_> = bookmarks(&document)
        .iter()
        .map(|bookmark| (bookmark.name.clone(), bookmark.kind()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("cellbm".to_owned(), "cells"),
            ("frbm".to_owned(), "frame"),
            ("incell".to_owned(), "cell")
        ]
    );
    let BookmarkTarget::Cell { table, .. } = &bookmarks(&document)[2].target else {
        panic!("a cell bookmark");
    };
    assert_eq!(*table, Some(frame));
    let written = String::from_utf8_lossy(&sam_document(&document, &OPTIONS)).into_owned();
    assert!(written.contains("<:Z2,2;cellbm>"));
    assert!(written.contains("[frmname]\r\n\t\tfrbm\r\n"));
    let copy = import_sam(written.as_bytes(), None, "Bookmarks")
        .unwrap()
        .document;
    let names: Vec<_> = bookmarks(&copy)
        .into_iter()
        .map(|bookmark| bookmark.name)
        .collect();
    assert_eq!(names, ["cellbm", "frbm", "incell"]);
}

#[test]
fn footnotes_insert_with_ami_pros_footnote_style_and_their_options_rnf1_rnf4() {
    let document = Document::from_plain_text("Main text with a footnote here.\nSecond.");
    let made = apply(
        &document,
        &Command::InsertFootnote {
            at: main_at(&document, 0, "Main text with a footnote here.".len()),
        },
    )
    .unwrap();
    let caret = made.caret.unwrap();
    let document = made.document;
    let InlineObject::Footnote { flow } = &document.paragraphs()[0].objects[0] else {
        panic!("a footnote");
    };
    assert_eq!(caret.flow, *flow);
    let style = &document.styles()[&document.flow(*flow).unwrap().paragraphs[0].style];
    assert_eq!(style.name, "Footnote");
    assert_eq!(style.indents.first, 720);
    assert_eq!(style.bullets.text, "\u{1A}");
    let document = run(
        &document,
        Command::ReplaceText {
            start: caret,
            end: caret,
            text: "The footnote text.".into(),
        },
    );
    let written = String::from_utf8_lossy(&sam_document(&document, &OPTIONS)).into_owned();
    assert!(written.contains(
        "Main text with a footnote here.<:F\r\n@Footnote@The footnote text.\r\n\r\n>\r\n"
    ));
    assert!(
        round_trip(&document).is_empty(),
        "{:?}",
        round_trip(&document)
    );
    // Footnote Options with Ami Pro's rules.
    let mut options = FootnoteOptions::default();
    options.set(FootnoteOptions::ENDNOTES, true);
    options.set(FootnoteOptions::RESET_EACH_PAGE, true);
    let document = run(&document, Command::SetFootnoteOptions(options));
    assert!(document.footnote_options().endnotes());
    assert!(!document.footnote_options().reset_each_page());
    assert!(
        apply(
            &document,
            &Command::SetFootnoteOptions(FootnoteOptions {
                start: 0,
                ..FootnoteOptions::default()
            })
        )
        .is_err()
    );
    // Deleting the reference deletes the footnote.
    let start = main_at(&document, 0, "Main text with a footnote here.".len());
    let deleted = run(
        &document,
        Command::ReplaceText {
            start,
            end: TextPosition {
                byte: start.byte + 3,
                ..start
            },
            text: String::new(),
        },
    );
    assert!(deleted.flow(*flow).is_none());
    deleted.validate().unwrap();
}

#[test]
fn footnotes_are_find_and_replaces_third_stream_and_notes_are_never_searched() {
    use amipro_editing::find::{StreamKind, streams};
    let document = Document::from_plain_text("Text.");
    let document = run(
        &document,
        Command::InsertFootnote {
            at: main_at(&document, 0, 5),
        },
    );
    let document = run(
        &document,
        Command::InsertNote {
            at: main_at(&document, 0, 0),
            note: note("LS", NoteInfo::YELLOW),
            text: "A note.".into(),
        },
    );
    let kinds: Vec<StreamKind> = streams(&document)
        .iter()
        .map(|stream| stream.kind)
        .collect();
    assert_eq!(kinds[..2], [StreamKind::Main, StreamKind::Footnote]);
    let note_flow = notes(&document)[0].2;
    assert!(
        streams(&document)
            .iter()
            .all(|stream| stream.flow != note_flow)
    );
    // Word count leaves notes out, as Ami Pro's.
    assert_eq!(amipro_document::count_words_and_characters(&document).0, 1);
}

#[test]
fn a_footnote_in_a_cell_goes_in_a_joined_row_at_the_tables_end_rnf6() {
    let (document, frame) = table_document();
    let amipro_document::FrameContent::Table(table) = &document.frame(frame).unwrap().content
    else {
        panic!("a table");
    };
    let amipro_document::CellContent::Text(cell) = table.cells[1].content;
    let made = apply(&document, &Command::InsertFootnote { at: at(cell, 0, 0) }).unwrap();
    let document = made.document;
    let amipro_document::FrameContent::Table(table) = &document.frame(frame).unwrap().content
    else {
        panic!("a table");
    };
    assert_eq!(table.rows.len(), 4);
    let amipro_document::CellContent::Text(row) = table.cells[9].content;
    assert_eq!(made.caret.unwrap().flow, row);
    assert_eq!(
        document.styles()[&document.flow(row).unwrap().paragraphs[0].style].name,
        "Footnote"
    );
    let written = String::from_utf8_lossy(&sam_document(&document, &OPTIONS)).into_owned();
    assert!(written.contains("<:x1,3>"), "{written}");
    assert!(
        round_trip(&document).is_empty(),
        "{:?}",
        round_trip(&document)
    );
}

/// The twins for Ami Pro 3.1 (the notes run, NB.13; `AMIPRO_TWIN_OUT`):
/// notes (ANT1), bookmarks of each kind (ABK1), footnotes with Ami Pro's
/// options and a long one split across pages (AFN1), a table footnote
/// (AFN2), and endnotes (AFN3), each made with the commands and written by
/// the SAM writer, to open in Ami Pro as the app shows them.
#[test]
#[ignore]
fn reference_twins() {
    let Ok(out) = std::env::var("AMIPRO_TWIN_OUT") else {
        return;
    };
    let out = std::path::PathBuf::from(out);
    let write = |name: &str, document: &Document| {
        let path = out.join(name);
        let _ = std::fs::remove_file(&path);
        std::fs::write(&path, sam_document(document, &OPTIONS)).unwrap();
        assert!(
            round_trip(document).is_empty(),
            "{name}: {:?}",
            round_trip(document)
        );
    };
    // ANT1: three notes, two colours, one with two paragraphs.
    let mut document = Document::from_plain_text("One two three.\nSecond paragraph here.");
    for (byte, paragraph, initials, colour, text) in [
        (7, 0, "LS", NoteInfo::YELLOW, "Para one.\nPara two."),
        (14, 0, "QX", 255, "Red note."),
        (6, 1, "LS", 32_768, "Green note."),
    ] {
        document = run(
            &document,
            Command::InsertNote {
                at: main_at(&document, paragraph, byte),
                note: note(initials, colour),
                text: text.into(),
            },
        );
    }
    write("ANT1.SAM", &document);
    // ABK1: a text bookmark, a caret one, cells, one cell, and a frame.
    let (mut document, frame) = table_document();
    document = run(
        &document,
        Command::AddBookmark {
            name: "textbm".into(),
            target: NewBookmark::Text {
                start: main_at(&document, 0, 0),
                end: main_at(&document, 0, 6),
            },
        },
    );
    document = run(
        &document,
        Command::AddBookmark {
            name: "caretbm".into(),
            target: NewBookmark::Text {
                start: main_at(&document, 0, 7),
                end: main_at(&document, 0, 7),
            },
        },
    );
    for (name, target) in [
        (
            "cellsbm",
            NewBookmark::Cells {
                frame,
                row: 0,
                column: 1,
                rows: 2,
                columns: 2,
            },
        ),
        (
            "onecell",
            NewBookmark::Cell {
                frame,
                row: 2,
                column: 0,
            },
        ),
        ("framebm", NewBookmark::Frame(frame)),
    ] {
        document = run(
            &document,
            Command::AddBookmark {
                name: name.into(),
                target,
            },
        );
    }
    write("ABK1.SAM", &document);
    // AFN1: RNF5's text, its footnotes, and RNF4's options.
    let lines: Vec<String> = (1..=60)
        .map(|n| {
            format!(
                "Paragraph {n} of the main text, long enough to fill a good part of one line here."
            )
        })
        .collect();
    let mut document = Document::from_plain_text(&lines.join("\n"));
    for (index, paragraphs) in [(7usize, 1usize), (29, 25), (49, 1)] {
        let end = document.paragraphs()[index].text.len();
        let made = apply(
            &document,
            &Command::InsertFootnote {
                at: main_at(&document, index, end),
            },
        )
        .unwrap();
        let caret = made.caret.unwrap();
        let text: Vec<String> = match paragraphs {
            1 if index == 7 => vec!["Short footnote on page one.".into()],
            1 => vec!["Third footnote after the long one.".into()],
            count => (1..=count)
                .map(|k| format!("Long footnote paragraph {k} that keeps going for a while."))
                .collect(),
        };
        let mut next = made.document;
        for (k, line) in text.iter().enumerate() {
            let at_end = TextPosition {
                flow: caret.flow,
                paragraph: k,
                byte: 0,
            };
            next = run(
                &next,
                Command::ReplaceText {
                    start: at_end,
                    end: at_end,
                    text: if k + 1 < text.len() {
                        format!("{line}\n")
                    } else {
                        line.clone()
                    },
                },
            );
        }
        document = next;
    }
    let mut options = FootnoteOptions {
        start: 1,
        length: 2_880,
        indent: 720,
        continued_on: "Continued".into(),
        continued_from: "From before".into(),
        flags: 0,
    };
    for flag in [
        FootnoteOptions::CUSTOM_LENGTH,
        FootnoteOptions::CONTINUED_ON,
        FootnoteOptions::CONTINUED_FROM,
        FootnoteOptions::CONTINUED_ON_LEFT,
        FootnoteOptions::REPEAT_NUMBER,
    ] {
        options.set(flag, true);
    }
    document = run(&document, Command::SetFootnoteOptions(options.clone()));
    write("AFN1.SAM", &document);
    // AFN3: the same as endnotes.
    let mut endnotes = options;
    endnotes.set(FootnoteOptions::ENDNOTES, true);
    write(
        "AFN3.SAM",
        &run(&document, Command::SetFootnoteOptions(endnotes)),
    );
    // AFN2: a footnote in a table's cell.
    let (document, frame) = table_document();
    let amipro_document::FrameContent::Table(table) = &document.frame(frame).unwrap().content
    else {
        panic!("a table");
    };
    let amipro_document::CellContent::Text(cell) = table.cells[1].content;
    let document = run(
        &document,
        Command::ReplaceText {
            start: at(cell, 0, 0),
            end: at(cell, 0, 0),
            text: "b1 cell".into(),
        },
    );
    let made = apply(&document, &Command::InsertFootnote { at: at(cell, 0, 7) }).unwrap();
    let caret = made.caret.unwrap();
    let document = run(
        &made.document,
        Command::ReplaceText {
            start: caret,
            end: caret,
            text: "Cell footnote.".into(),
        },
    );
    write("AFN2.SAM", &document);
}
