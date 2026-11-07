//! The Markdown reader and writer (MD2): constructs, round trips, hostile
//! input, and what the format cannot hold.

use amipro_exchange::import::{self, FindingKind, Findings, ImportOptions, SourceFormat};
use amipro_exchange::losses::{self, Feature, Features, Format};
use amipro_exchange::markdown;
use amipro_exchange::read_markdown::read_markdown;
use amipro_exchange::realise::{RealiseOptions, realise};
use amipro_exchange::{Block, CharProps, ExchangeDocument, Inline, NumberFormat, Paragraph, Table};
use amipro_layout::FixedShaper;

fn read(text: &str) -> (ExchangeDocument, Findings) {
    let mut findings = Findings::default();
    let document = read_markdown(text.as_bytes(), &mut findings);
    (document, findings)
}

fn blocks(document: &ExchangeDocument) -> &[Block] {
    &document.sections[0].blocks
}

/// A paragraph as "Style[list]: text", marks as Markdown's.
fn line(document: &ExchangeDocument, paragraph: &Paragraph) -> String {
    let mut text = String::new();
    for inline in &paragraph.inlines {
        match inline {
            Inline::Text { text: run, props } => {
                let mono = props.font.is_some();
                let marks = format!(
                    "{}{}{}{}",
                    if props.bold == Some(true) { "**" } else { "" },
                    if props.italic == Some(true) { "*" } else { "" },
                    if props.strike == Some(true) { "~~" } else { "" },
                    if mono { "`" } else { "" },
                );
                let closing: String = marks.chars().rev().collect();
                text.push_str(&format!("{marks}{run}{closing}"));
            }
            Inline::Tab { .. } => text.push('\t'),
            Inline::LineBreak => text.push('\u{23CE}'),
            Inline::Footnote(note) => {
                text.push_str("[^");
                for block in &note.blocks {
                    if let Block::Paragraph(inner) = block {
                        text.push_str(&line(document, inner));
                    }
                }
                text.push(']');
            }
            _ => text.push('?'),
        }
    }
    let list = paragraph
        .list
        .and_then(|(instance, level)| {
            let list = document.list_instances.get(instance?)?.list;
            let format = document.lists[list].levels[usize::from(level)].format;
            Some(format!(
                " {}{level}",
                if format == NumberFormat::Bullet {
                    "bullet"
                } else {
                    "number"
                }
            ))
        })
        .unwrap_or_default();
    format!("{}{list}: {text}", document.styles[paragraph.style].name)
}

fn lines(document: &ExchangeDocument) -> Vec<String> {
    blocks(document)
        .iter()
        .map(|block| match block {
            Block::Paragraph(paragraph) => line(document, paragraph),
            Block::Table(table) => format!("table {}x{}", table.rows.len(), table.columns.len()),
        })
        .collect()
}

fn round(document: &ExchangeDocument) -> ExchangeDocument {
    let written = markdown::write(document, "doc");
    read(&written.text).0
}

#[test]
fn headings_are_styles_with_outline_levels() {
    let (document, _) = read("# One\n\n###### Six\n\nBody\n\nSetext\n======\n");
    assert_eq!(
        lines(&document),
        [
            "Heading 1: One",
            "Heading 6: Six",
            "Body Text: Body",
            "Heading 1: Setext"
        ]
    );
    let heading = document
        .styles
        .iter()
        .find(|s| s.name == "Heading 6")
        .unwrap();
    assert_eq!(heading.outline_level, Some(6));
    assert_eq!(document.styles[0].name, "Body Text");
}

#[test]
fn emphasis_strikethrough_and_code() {
    let (document, _) = read("a **b** *c* ***d*** ~~e~~ `f`\n");
    assert_eq!(
        lines(&document),
        ["Body Text: a **b** *c* ***d*** ~~e~~ `f`"]
    );
    assert!(document.fonts.iter().any(|font| font.fixed));
}

#[test]
fn breaks_and_soft_breaks() {
    let (document, _) = read("one\ntwo  \nthree\\\nfour\n");
    assert_eq!(
        lines(&document),
        ["Body Text: one two\u{23CE}three\u{23CE}four"]
    );
}

#[test]
fn links_keep_their_text_and_address_and_pictures_their_description() {
    let (document, findings) = read("[text](http://example.com) <http://a.b> ![alt](pic.png)\n");
    assert_eq!(
        lines(&document),
        ["Body Text: text (http://example.com) http://a.b alt"]
    );
    let kinds: Vec<_> = findings.iter().map(|f| (f.kind.clone(), f.count)).collect();
    assert_eq!(
        kinds,
        [(FindingKind::Links, 2), (FindingKind::LinkedPictures, 1)]
    );
}

#[test]
fn lists_nest_and_number_and_task_boxes_are_characters() {
    let (document, _) = read("- a\n  - b\n    1. c\n- [x] d\n- [ ] e\n\n3. f\n4. g\n");
    assert_eq!(
        lines(&document),
        [
            "Body Text bullet0: a",
            "Body Text bullet1: b",
            "Body Text number2: c",
            "Body Text bullet0: \u{2611} d",
            "Body Text bullet0: \u{2610} e",
            "Body Text number0: f",
            "Body Text number0: g",
        ]
    );
    // The numbered list starts at three.
    let instance = match &blocks(&document)[5] {
        Block::Paragraph(p) => p.list.unwrap().0.unwrap(),
        _ => unreachable!(),
    };
    assert_eq!(document.list_instances[instance].starts[0], Some(3));
}

#[test]
fn quotes_code_and_rules() {
    let (document, _) =
        read("> q1\n>\n> > q2\n\n```rust\nfn a() {}\n\n\tb\n```\n\n    indented\n\n---\n");
    assert_eq!(
        lines(&document),
        [
            "Quote: q1",
            "Quote: q2",
            "Code: fn a() {}",
            "Code: ",
            "Code: \tb",
            "Code: indented",
            "Body Text: ",
        ]
    );
    let Block::Paragraph(second) = &blocks(&document)[1] else {
        panic!()
    };
    assert_eq!(second.props.indent_left, Some(1440));
    let Block::Paragraph(rule) = &blocks(&document)[6] else {
        panic!()
    };
    assert!(matches!(rule.props.border_bottom, Some(Some(_))));
}

#[test]
fn tables_become_tables_with_alignments() {
    let (document, _) = read("| a | b |\n|:--|--:|\n| 1 | **2** |\n\nafter\n");
    assert_eq!(lines(&document), ["table 2x2", "Body Text: after"]);
    let Block::Table(table) = &blocks(&document)[0] else {
        panic!()
    };
    assert!(table.rows[0].heading && !table.rows[1].heading);
    let alignment = |row: usize, cell: usize| match &table.rows[row].cells[cell].blocks[0] {
        Block::Paragraph(p) => p.props.alignment,
        _ => None,
    };
    assert_eq!(alignment(1, 0), Some(amipro_exchange::Alignment::Left));
    assert_eq!(alignment(1, 1), Some(amipro_exchange::Alignment::Right));
}

#[test]
fn footnotes_are_footnotes_even_defined_after_use() {
    let (document, _) = read("Text[^1] more[^2].\n\n[^1]: First note.\n[^2]: Second.\n");
    assert_eq!(
        lines(&document),
        ["Body Text: Text[^Body Text: First note.] more[^Body Text: Second.]."]
    );
}

#[test]
fn html_stays_as_text_and_is_listed() {
    let (document, findings) = read("<div>\nraw\n</div>\n\ninline <b>x</b>\n");
    let all = lines(&document).join("|");
    assert!(
        all.contains("<div>") && all.contains("raw") && all.contains("<b>"),
        "{all}"
    );
    let html = findings
        .iter()
        .find(|f| f.kind == FindingKind::Html)
        .unwrap();
    assert!(html.count >= 3);
}

#[test]
fn byte_order_mark_and_bad_utf8_are_tolerated() {
    let mut findings = Findings::default();
    let document = read_markdown(b"\xEF\xBB\xBF# T\n\nbad \xFF\xFE end\n", &mut findings);
    let lines = lines(&document);
    assert_eq!(lines[0], "Heading 1: T");
    assert!(lines[1].starts_with("Body Text: bad \u{FFFD}"), "{lines:?}");
    assert_eq!(
        findings
            .iter()
            .find(|f| f.kind == FindingKind::BadCharacters)
            .unwrap()
            .count,
        2
    );
}

#[test]
fn hostile_input_never_panics() {
    let mut inputs: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"\n\n\n".to_vec(),
        b">".repeat(20_000),
        "- ".repeat(20_000).into_bytes(),
        "1. ".repeat(5_000).into_bytes(),
        "[".repeat(20_000).into_bytes(),
        "*a ".repeat(20_000).into_bytes(),
        "![a](".repeat(5_000).into_bytes(),
        "| a |\n|---|\n".repeat(2_000).into_bytes(),
        "[^1]: ".repeat(2_000).into_bytes(),
        "[^1]\n\n[^1]: [^1]\n".into(),
        b"```\n```\n````\n~~~\n".to_vec(),
        b"\0\0\x01\x1b\t\t# \t#\r\r\n".to_vec(),
        "<!--".repeat(1_000).into_bytes(),
        "# ".repeat(1_000).into_bytes(),
        "\u{202E}\u{FFFF}\u{10FFFF}".as_bytes().to_vec(),
    ];
    // A deterministic spread of bytes and Markdown's own punctuation.
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let alphabet = b"#*_`[]()!<>|-+~:\\ \n\t1.a^&=";
    for length in [1usize, 7, 40, 300, 3000] {
        for _ in 0..30 {
            let mut bytes = Vec::new();
            for _ in 0..length {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let pick = (state >> 11) as usize;
                bytes.push(if pick % 5 == 0 {
                    (pick >> 8) as u8
                } else {
                    alphabet[pick % alphabet.len()]
                });
            }
            inputs.push(bytes);
        }
    }
    for input in &inputs {
        let (document, _) = {
            let mut findings = Findings::default();
            (read_markdown(input, &mut findings), findings)
        };
        assert!(!blocks(&document).is_empty());
        // What is read can be written, and read again, and realised.
        let written = markdown::write(&document, "x");
        let again = read(&written.text).0;
        assert!(!blocks(&again).is_empty());
    }
}

#[test]
fn a_read_document_is_realised() {
    let (document, _) = read(
        "# T\n\n- a\n  - b\n\n1. x\n\n> q\n\n| a |\n|---|\n| b |\n\ntext[^1]\n\n[^1]: n\n\n```\ncode\n```\n\n---\n",
    );
    let base = amipro_document::Document::default();
    let options = RealiseOptions {
        styles: import::StyleOption::Convert,
        base: &base,
        inserting: false,
        into_text: false,
        title: "T".into(),
        now: 0,
    };
    let mut findings = Findings::default();
    let realised = realise(&document, &options, &mut FixedShaper, &mut findings);
    assert!(realised.document.paragraphs().len() >= 8);
}

fn text(content: &str) -> Inline {
    Inline::Text {
        text: content.to_owned(),
        props: CharProps::default(),
    }
}

#[test]
fn constructs_survive_write_then_read() {
    let source = "# Title\n\n## Sub\n\nPlain **bold** and *italic* and ***both*** and ~~gone~~ and `code`.\n\n\
- one\n    - nested\n        1. deep\n- two\n\n3. three\n4. four\n\n> quoted\n>\n> > deeper\n\n\
```\nline 1\n\nline 3\n```\n\n---\n\n| h1 | h2 |\n| :--- | ---: |\n| a | b |\n\nNote[^1] here.\n\n[^1]: The note.\n";
    let (first, _) = read(source);
    let second = round(&first);
    assert_eq!(lines(&first), lines(&second));
    let again = round(&second);
    assert_eq!(lines(&second), lines(&again));
    // The table's text and alignments are kept.
    let tables = |document: &ExchangeDocument| -> Vec<Table> {
        blocks(document)
            .iter()
            .filter_map(|b| {
                if let Block::Table(t) = b {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .collect()
    };
    assert_eq!(tables(&first).len(), 1);
    let (a, b) = (&tables(&first)[0], &tables(&second)[0]);
    assert_eq!(a.rows.len(), b.rows.len());
    assert_eq!(a.rows[1].cells[1], b.rows[1].cells[1]);
}

#[test]
fn the_writer_makes_the_markdown_one_expects() {
    let source = "# Title\n\nSome *text* with **bold**.\n\n- a\n    - b\n- c\n\n1. x\n2. y\n\n> q\n\n```\ncode\n```\n\n---\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n\nText[^1].\n\n[^1]: Note.\n";
    let (document, _) = read(source);
    let written = markdown::write(&document, "doc");
    assert_eq!(written.text, source);
    assert!(written.pictures.is_empty());
}

#[test]
fn text_is_escaped_so_that_it_reads_back_as_text() {
    let tricky = "*a* _b_ [c](d) <e> `f` # g - h 1. i | j ~k~ \\ & ![l](m)";
    let mut document = read("x\n").0;
    document.sections[0].blocks = vec![
        Block::Paragraph(Paragraph {
            inlines: vec![text(tricky)],
            ..Paragraph::default()
        }),
        Block::Paragraph(Paragraph {
            inlines: vec![text("# not a heading")],
            ..Paragraph::default()
        }),
        Block::Paragraph(Paragraph {
            inlines: vec![text("1. not a list")],
            ..Paragraph::default()
        }),
        Block::Paragraph(Paragraph {
            inlines: vec![text("- not a bullet")],
            ..Paragraph::default()
        }),
    ];
    let back = round(&document);
    let plain = |d: &ExchangeDocument| -> Vec<String> {
        lines(d).iter().map(|l| l.replace('?', "")).collect()
    };
    assert_eq!(plain(&back).len(), 4);
    assert_eq!(plain(&back)[0], format!("Body Text: {tricky}"));
    assert_eq!(plain(&back)[1], "Body Text: # not a heading");
    assert_eq!(plain(&back)[2], "Body Text: 1. not a list");
    assert_eq!(plain(&back)[3], "Body Text: - not a bullet");
}

#[test]
fn pictures_are_written_beside_the_file() {
    use amipro_exchange::{Anchor, Image, ImageKind, PlacedPicture, Shape, ShapeContent};
    let mut document = read("x\n").0;
    document.images.push(Image {
        bytes: vec![1, 2, 3].into(),
        kind: ImageKind::Png,
        fallback: None,
        pixels: None,
    });
    let shape = Shape {
        id: 1,
        name: String::new(),
        anchor: Anchor::Inline { drop: 0 },
        width: 100,
        height: 100,
        z: 0,
        line: None,
        fill: None,
        shadow: None,
        corner_radius: 0,
        content: ShapeContent::Picture(PlacedPicture {
            image: 0,
            left: 0,
            top: 0,
            width: 100,
            height: 100,
            description: "A [cat]".into(),
        }),
    };
    document.sections[0].blocks = vec![Block::Paragraph(Paragraph {
        inlines: vec![
            text("See "),
            Inline::Shape(Box::new(shape.clone())),
            Inline::Shape(Box::new(shape)),
        ],
        ..Paragraph::default()
    })];
    let written = markdown::write(&document, "my doc");
    assert_eq!(
        written.text,
        "See ![A \\[cat\\]](my-doc-1.png)![A \\[cat\\]](my-doc-1.png)\n"
    );
    assert_eq!(written.pictures.len(), 1);
    assert_eq!(written.pictures[0].0, "my-doc-1.png");
    assert_eq!(&*written.pictures[0].1, &[1, 2, 3]);
}

#[test]
fn written_markdown_reparses_to_the_same_events() {
    use pulldown_cmark::{Event, Options, Parser};
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS;
    let source = "# T\n\nA **b** *c* ~~d~~ `e` \\* f.\n\n- x\n    - y\n- [x] z\n\n1. p\n2. q\n\n> r\n\n```\ns\n```\n\n---\n\n| a | b |\n| --- | ---: |\n| 1 | 2 |\n\nN[^1]\n\n[^1]: Note.\n";
    let events = |text: &str| -> Vec<String> {
        Parser::new_ext(text, options)
            .map(|event| match event {
                // Soft breaks and spacing are not compared.
                Event::SoftBreak => " ".to_owned(),
                other => format!("{other:?}"),
            })
            .collect()
    };
    let (document, _) = read(source);
    let written = markdown::write(&document, "d").text;
    assert_eq!(events(&written), events(source));
}

#[test]
fn markdown_is_found_by_its_extension_only() {
    for extension in ["md", "MD", "markdown", "mdown"] {
        assert_eq!(
            import::detect(b"# T\n", Some(extension)),
            Some(SourceFormat::Markdown)
        );
    }
    // By its contents it is plain text.
    assert_eq!(import::detect(b"# T\n", None), Some(SourceFormat::Text));
    assert_eq!(
        import::detect(b"# T\n", Some("txt")),
        Some(SourceFormat::Text)
    );
    let read = import::read(
        b"# T\n\n[a](b)\n",
        Some("md"),
        &ImportOptions::default(),
        &|_, _| None,
    )
    .unwrap();
    assert_eq!(read.format, SourceFormat::Markdown);
    assert_eq!(read.format.name(), "Markdown");
    assert!(read.findings.iter().any(|f| f.kind == FindingKind::Links));
}

#[test]
fn what_markdown_cannot_hold_is_listed() {
    let mut features = Features::default();
    for feature in [
        Feature::Notes,
        Feature::Bookmarks,
        Feature::Frames,
        Feature::Tables,
        Feature::Pictures,
        Feature::Footnotes,
        Feature::PowerFields,
        Feature::Overstrike,
    ] {
        features.add(feature);
    }
    let lost: Vec<Feature> = losses::losses(&features, Format::Markdown)
        .iter()
        .map(|loss| loss.feature)
        .collect();
    assert_eq!(
        lost,
        [
            Feature::Overstrike,
            Feature::Notes,
            Feature::Bookmarks,
            Feature::Frames
        ]
    );
    let always = losses::always_lost(Format::Markdown);
    assert!(always.iter().any(|line| line.starts_with("page layout")));
    assert!(always.iter().any(|line| line.starts_with("fonts")));
    assert!(losses::always_lost(Format::Rtf).is_empty());
    assert_eq!(Format::Markdown.extension(), "md");
    assert_eq!(Format::Markdown.name(), "Markdown");
}
