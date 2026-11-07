//! Tools › TOC, Index…'s table of contents, against Ami Pro 3.1's own
//! output for the same input (VM fixtures TI1 → TI1T2, TI2 → TI2TOC, TI3 →
//! TI3T; `docs/research/TOC_INDEX_NOTES.md`).

use amipro_document::contents::is_toc_table;
use amipro_document::{CellContent, CellLeader, Document, FlowId, FrameContent, InlineObject};
use amipro_editing::contents::{
    ContentsHost, TocPlace, TocRequest, collect_toc, toc_levels, toc_table,
};
use amipro_editing::{Command, apply};
use amipro_formats::{SamOptions, import_sam, sam_document};

fn sam(sections: &str, edoc: &str) -> Vec<u8> {
    format!("[ver]\r\n\t4\r\n{sections}[edoc]\r\n{edoc}>\r\n\r\n").into_bytes()
}

fn read(bytes: &[u8]) -> Document {
    import_sam(bytes, None, "Contents").unwrap().document
}

/// Pages by main-text paragraph, as Ami Pro laid the fixtures out.
struct Pages {
    main: FlowId,
    pages: Vec<u32>,
    prefix: &'static str,
}

impl ContentsHost for Pages {
    fn page(&self, flow: FlowId, paragraph: usize, _byte: usize) -> Option<(usize, String)> {
        (flow == self.main)
            .then(|| self.pages.get(paragraph))
            .flatten()
            .map(|page| (*page as usize, format!("{}{page}", self.prefix)))
    }

    fn number_text(&self, _flow: FlowId, _paragraph: usize) -> Option<String> {
        None
    }
}

fn ix(primary: &str) -> String {
    format!("<:X8,-32768;Index \"{primary}\" #>")
}

/// TI1's input: Title and Subhead headings over four pages, a TOC mark.
fn ti1() -> Document {
    let edoc = [
        "@Title@Getting Started".to_string(),
        format!("The printer must be set up first.{}", ix("Printer")),
        "@Subhead@Installing".into(),
        "Install the program from the disk.<:p<* >>".into(),
        "@Subhead@Fonts".into(),
        "Fonts come with the printer.".into(),
        "An entry without a page.<:p<* >>".into(),
        "@Title@Advanced Topics".into(),
        "@Subhead@Frames".into(),
        "Frames hold text.<:X11,-32768;TOC 3 \"Marked entry\">".into(),
        "More text.<:p<* >>".into(),
        "@Subhead@Tables".into(),
        "Tables hold rows.".into(),
    ]
    .map(|paragraph| format!("{paragraph}\r\n\r\n"))
    .concat();
    read(&sam(
        "[toc]\r\n\tTitle\r\n\tSubhead\r\n\t\r\n\t.\r\n\t.\r\n\t\r\n\t15\r\n",
        &edoc,
    ))
}

/// Each row's cells: style, text, leader.
fn rows(
    document: &Document,
    frame: amipro_document::FrameId,
) -> Vec<[(String, String, CellLeader); 2]> {
    let FrameContent::Table(table) = &document.frame(frame).unwrap().content else {
        panic!("a table");
    };
    table
        .cells
        .chunks(2)
        .map(|row| {
            [0, 1].map(|column| {
                let CellContent::Text(flow) = row[column].content;
                let paragraph = &document.flow(flow).unwrap().paragraphs[0];
                (
                    document.styles()[&paragraph.style].name.clone(),
                    paragraph.text.clone(),
                    row[column].format.leader,
                )
            })
        })
        .collect()
}

fn cell(style: &str, text: &str, leader: CellLeader) -> (String, String, CellLeader) {
    (style.into(), text.into(), leader)
}

fn generate(document: &Document, place: TocPlace, host: &Pages) -> Document {
    let levels = toc_levels(document);
    let entries = collect_toc(document, &levels, host);
    apply(
        document,
        &Command::PutToc(Box::new(TocRequest {
            place,
            levels,
            entries,
            width: 9360,
        })),
    )
    .unwrap()
    .document
}

#[test]
fn a_toc_at_the_start_is_ami_pros_page_table_ti1t2() {
    let document = ti1();
    let host = Pages {
        main: document.main_flow_id(),
        pages: vec![1, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4],
        prefix: "",
    };
    let made = generate(&document, TocPlace::Start, &host);
    let frame = toc_table(&made).expect("the TOC table");
    assert!(is_toc_table(made.frame(frame).unwrap()));
    use CellLeader::{Dots, None as No};
    assert_eq!(
        rows(&made, frame),
        [
            [
                cell("TOC1", "Getting Started", Dots),
                cell("TOCPG1", "1", Dots)
            ],
            [cell("TOC2", "Installing", Dots), cell("TOCPG2", "1", Dots)],
            [cell("TOC2", "Fonts", Dots), cell("TOCPG2", "2", Dots)],
            [
                cell("TOC1", "Advanced Topics", Dots),
                cell("TOCPG1", "3", Dots)
            ],
            [cell("TOC2", "Frames", Dots), cell("TOCPG2", "3", Dots)],
            [cell("TOC3", "Marked entry", No), cell("TOCPG3", "3", No)],
            [cell("TOC2", "Tables", Dots), cell("TOCPG2", "4", Dots)],
        ]
    );
    // The body starts with the table's anchor and the TOC's break.
    let first = &made.main_flow().paragraphs[0];
    assert_eq!(first.text, "\u{FFFC}\u{FFFC}");
    assert!(matches!(first.objects[0], InlineObject::FrameAnchor(id) if id == frame));
    assert!(matches!(
        first.objects[1],
        InlineObject::Break {
            restart: true,
            layout: Some(_),
            ..
        }
    ));
    assert_eq!(made.styles()[&first.style].name, "Body Text");
    // Ami Pro's styles: TOCn indented by level, TOCPGn right aligned.
    let style = |name: &str| {
        made.styles()
            .values()
            .find(|style| style.name == name)
            .unwrap()
    };
    assert_eq!(
        (style("TOC2").indents.first, style("TOC2").indents.rest),
        (360, 720)
    );
    assert_eq!(style("TOCPG2").alignment, amipro_document::Alignment::Right);
    assert!((1..=9).all(|level| {
        made.styles()
            .values()
            .any(|style| style.name == format!("TOCPG{level}"))
    }));
    // Written as Ami Pro writes it: the anchor, the break with its flags.
    let written =
        String::from_utf8_lossy(&sam_document(&made, &SamOptions::at(1_793_000_000))).into_owned();
    assert!(
        written.contains("[edoc]\r\n<:t0><:p</B>"),
        "{}",
        &written[written.find("[edoc]").unwrap()..][..60]
    );
    assert!(written.contains("@TOCPG1@1\r\n"));
    let again = read(written.as_bytes());
    assert!(toc_table(&again).is_some(), "the TOC flag is kept");
    let native = Document::decode_native(&made.encode_native().unwrap()).unwrap();
    assert!(matches!(
        native.main_flow().paragraphs[0].objects[1],
        InlineObject::Break { restart: true, .. }
    ));
    assert!(toc_table(&native).is_some());

    // A second Generate replaces the table in place, and one undo step
    // brings the first back.
    let host = Pages {
        main: made.main_flow_id(),
        pages: vec![0, 1, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4],
        prefix: "",
    };
    let again = generate(&made, TocPlace::Start, &host);
    assert_eq!(
        again.main_flow().paragraphs.len(),
        made.main_flow().paragraphs.len()
    );
    let tables = again.frames().filter(|frame| is_toc_table(frame)).count();
    assert_eq!(tables, 1);
    assert_eq!(rows(&again, toc_table(&again).unwrap()), rows(&made, frame));
}

#[test]
fn toc_marks_without_levels_put_the_page_after_the_text_ti3t() {
    // TI3: TOC marks only, no [toc]; the header's page field reads "Page N".
    let edoc = [
        "One.<:X11,-32768;TOC 1 \"First\"><:p<* >>",
        "Two.<:X11,-32768;TOC 1 \"Second\"><:p<* >>",
        "Three.<:X11,-32768;TOC 1 \"Third\">",
    ]
    .map(|paragraph| format!("{paragraph}\r\n\r\n"))
    .concat();
    let document = read(&sam("", &edoc));
    let host = Pages {
        main: document.main_flow_id(),
        pages: vec![1, 2, 3],
        prefix: "Page ",
    };
    let made = generate(&document, TocPlace::Start, &host);
    use CellLeader::None as No;
    assert_eq!(
        rows(&made, toc_table(&made).unwrap()),
        [
            [cell("TOC1", "FirstPage 1", No), cell("TOCPG1", " ", No)],
            [cell("TOC1", "SecondPage 2", No), cell("TOCPG1", " ", No)],
            [cell("TOC1", "ThirdPage 3", No), cell("TOCPG1", " ", No)],
        ]
    );
}

#[test]
fn a_toc_output_document_has_the_table_alone_ti2toc() {
    // TI2: Title's page number off, Subhead's after a comma.
    let edoc = [
        "@Title@Getting Started",
        "Text one.",
        "@Subhead@Installing",
        "Install it.<:p<* >>",
        "@Subhead@Fonts",
        "Fonts.<:X11,-32768;TOC 1 \"Marked top\">",
        "@Title@Advanced",
        "End.",
    ]
    .map(|paragraph| format!("{paragraph}\r\n\r\n"))
    .concat();
    let document = read(&sam(
        "[toc]\r\n\tTitle\r\n\tSubhead\r\n\t\r\n\t  \r\n\t, \r\n\t\r\n\t4\r\n",
        &edoc,
    ));
    let host = Pages {
        main: document.main_flow_id(),
        pages: vec![1, 1, 1, 1, 2, 2, 2, 2],
        prefix: "",
    };
    let levels = toc_levels(&document);
    let entries = collect_toc(&document, &levels, &host);
    // The output document: a new one (here an empty one) holding the table.
    let output = Document::from_plain_text("");
    let made = apply(
        &output,
        &Command::PutToc(Box::new(TocRequest {
            place: TocPlace::Output,
            levels,
            entries,
            width: 9360,
        })),
    )
    .unwrap()
    .document;
    let frame = made.frame_stack().last().copied().unwrap();
    assert!(!is_toc_table(made.frame(frame).unwrap()), "no TOC flag");
    use CellLeader::None as No;
    assert_eq!(
        rows(&made, frame),
        [
            [cell("TOC1", "Getting Started", No), cell("TOCPG1", " ", No)],
            [cell("TOC2", "Installing, 1", No), cell("TOCPG2", " ", No)],
            [cell("TOC2", "Fonts, 2", No), cell("TOCPG2", " ", No)],
            [cell("TOC1", "Marked top", No), cell("TOCPG1", " ", No)],
            [cell("TOC1", "Advanced", No), cell("TOCPG1", " ", No)],
        ]
    );
    assert_eq!(made.main_flow().paragraphs[0].objects.len(), 1, "no break");
}

fn ixs(primary: &str, secondary: Option<&str>, other: Option<&str>, page: bool) -> String {
    let mut code = format!("Index \"{primary}\"");
    if page {
        code.push_str(" #");
    }
    if secondary.is_some() || other.is_some() {
        code.push_str(&format!(" \"{}\"", secondary.unwrap_or("")));
    }
    if let Some(other) = other {
        code.push_str(&format!(" \"{other}\""));
    }
    format!("<:X8,-32768;{code}>")
}

#[test]
fn the_index_is_ami_pros_ti1idx() {
    use amipro_editing::contents::{IndexLine, IndexOptions, build_index, collect_index};
    // TI1's paragraphs that hold marks, one a page, as Ami Pro laid them.
    let edoc = [
        format!(
            "The printer.{}{}",
            ixs("Printer", None, None, true),
            ixs("Printer", Some("setup"), None, true)
        ),
        format!("Install.{}", ixs("installing", None, None, true)),
        format!(
            "Fonts.{}{}{}",
            ixs("fonts", None, None, true),
            ixs("Printer", Some("setup"), None, true),
            ixs("Zebra", None, Some(" See Animals"), true)
        ),
        format!("No page.{}", ixs("No page", None, None, false)),
        format!(
            "Frames.{}{}{}",
            ixs("frames", None, None, true),
            ixs("Apple", Some("red"), None, true),
            ixs("Apple", Some("green"), None, true)
        ),
        format!(
            "More.{}{}",
            ixs("123 numbers", None, None, true),
            ixs("apple", None, None, true)
        ),
        format!(
            "Tables.{}{}{}",
            ixs("Printer", Some("setup"), None, true),
            ixs("Printer", None, None, true),
            ixs("Bear", None, Some(" See also Animals"), true)
        ),
    ]
    .map(|paragraph| format!("{paragraph}\r\n\r\n"))
    .concat();
    let document = read(&sam("", &edoc));
    let host = Pages {
        main: document.main_flow_id(),
        pages: vec![1, 1, 2, 2, 3, 3, 4],
        prefix: "",
    };
    let marks = collect_index(&document, &host);
    assert_eq!(marks.len(), 15);
    let lines = build_index(
        &marks,
        IndexOptions {
            separators: true,
            ranges: false,
        },
    );
    use IndexLine::{Primary as P, Secondary as S, Separator as L};
    let expected = [
        L("1".into()),
        P("123 numbers, 3".into()),
        L("A".into()),
        P("Apple".into()),
        S("green, 3".into()),
        S("red, 3".into()),
        P("apple, 3".into()),
        L("B".into()),
        P("Bear, 4 See also Animals".into()),
        L("F".into()),
        P("fonts, 2".into()),
        P("frames, 3".into()),
        L("I".into()),
        P("installing, 1".into()),
        L("N".into()),
        P("No page".into()),
        L("P".into()),
        P("Printer, 1, 4".into()),
        S("setup, 1, 2, 4".into()),
        L("Z".into()),
        P("Zebra, 2 See Animals".into()),
    ];
    assert_eq!(lines, expected);

    // Ranges (TX5) join pages that follow one another.
    let ranged = build_index(
        &marks,
        IndexOptions {
            separators: false,
            ranges: true,
        },
    );
    assert!(ranged.contains(&S("setup, 1\u{2013}2, 4".into())));

    // As an output document: the paragraphs in Ami Pro's styles.
    let output = Document::from_plain_text("");
    let made = apply(
        &output,
        &Command::PutIndex(Box::new(amipro_editing::contents::IndexRequest {
            place: amipro_editing::contents::IndexPlace::Output,
            lines: lines.clone(),
            styles: Vec::new(),
        })),
    )
    .unwrap()
    .document;
    let paragraphs = &made.main_flow().paragraphs;
    assert_eq!(paragraphs.len(), expected.len());
    assert_eq!(made.styles()[&paragraphs[0].style].name, "Separator");
    assert_eq!(paragraphs[3].text, "Apple");
    assert_eq!(made.styles()[&paragraphs[4].style].name, "Secondary");
    // At the end of the document, after a page break.
    let at_end = apply(
        &document,
        &Command::PutIndex(Box::new(amipro_editing::contents::IndexRequest {
            place: amipro_editing::contents::IndexPlace::End,
            lines,
            styles: Vec::new(),
        })),
    )
    .unwrap()
    .document;
    assert_eq!(
        at_end.main_flow().paragraphs.len(),
        document.main_flow().paragraphs.len() + expected.len()
    );
}

#[test]
fn ami_pros_sort_folds_case_and_accents_and_puts_punctuation_first() {
    use amipro_editing::contents::index_order;
    use std::cmp::Ordering::Less;
    for (left, right) in [
        ("Apple", "apple"),
        ("apple", "Banana"),
        ("École", "Ferry"),
        ("Eagle", "École"),
        ("#tag", "1st"),
        ("a b", "ab"),
        ("9", "a"),
        ("zebra", "Ærø"),
    ] {
        assert_eq!(index_order(left, right), Less, "{left} < {right}");
    }
}

#[test]
fn marks_go_after_the_marks_at_their_place_and_come_out_in_one_step() {
    use amipro_document::contents::{IndexEntry, TocMark};
    use amipro_editing::TextPosition;
    use amipro_editing::contents::{MarkChanges, index_mark, toc_mark};
    let document = read(&sam("", "Printer setup.\r\n\r\n"));
    let at = TextPosition {
        flow: document.main_flow_id(),
        paragraph: 0,
        byte: 7,
    };
    let first = IndexEntry {
        primary: "Printer".into(),
        page_number: true,
        ..IndexEntry::default()
    };
    let second = IndexEntry {
        secondary: "setup".into(),
        ..first.clone()
    };
    let marked = apply(
        &document,
        &Command::PutMarks(MarkChanges {
            remove: Vec::new(),
            add: vec![
                (at, index_mark(&first)),
                (at, index_mark(&second)),
                (
                    at,
                    toc_mark(&TocMark {
                        level: 2,
                        text: "Printer setup".into(),
                    }),
                ),
            ],
        }),
    )
    .unwrap()
    .document;
    let raws: Vec<&str> = marked.main_flow().paragraphs[0]
        .marks
        .iter()
        .map(|mark| mark.raw.as_str())
        .collect();
    assert_eq!(
        raws,
        [
            "<:X8,-32768;Index \"Printer\" #>",
            "<:X8,-32768;Index \"Printer\" # \"setup\">",
            "<:X11,-32768;TOC 2 \"Printer setup\">",
        ]
    );
    let written = String::from_utf8_lossy(&sam_document(&marked, &SamOptions::at(1_793_000_000)))
        .into_owned();
    assert!(written.contains("Printer<:X8,-32768;Index \"Printer\" #><:X8,-32768;Index \"Printer\" # \"setup\"><:X11,-32768;TOC 2 \"Printer setup\"> setup."));
    let removed = apply(
        &marked,
        &Command::PutMarks(MarkChanges {
            remove: vec![(marked.main_flow_id(), 0, 1), (marked.main_flow_id(), 0, 0)],
            add: Vec::new(),
        }),
    )
    .unwrap()
    .document;
    assert_eq!(removed.main_flow().paragraphs[0].marks.len(), 1);
}
