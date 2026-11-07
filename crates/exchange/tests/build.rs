//! The exchange document built from synthetic documents, and the filters'
//! output for them.

use amipro_document::{
    Bullets, CharacterFormat, CharacterRun, DateField, DateSource as AmiDate, Document,
    InlineObject, OBJECT_REPLACEMENT_CHARACTER, StyleId,
};
use amipro_exchange::build::{BuildOptions, NoPictures, build};
use amipro_exchange::{
    Block, CharProps, DateSource, FieldKind, Format, Inline, Underline, docx, losses, text, zip,
};

fn body(document: &amipro_exchange::ExchangeDocument) -> &[Block] {
    &document.sections[0].blocks
}

fn inlines(block: &Block) -> &[Inline] {
    match block {
        Block::Paragraph(paragraph) => &paragraph.inlines,
        Block::Table(_) => panic!("a table"),
    }
}

#[test]
fn styles_keep_ami_pros_names_and_body_text_comes_first() {
    let document = Document::with_paragraphs([("Title", StyleId(2)), ("Text", StyleId(1))]);
    let built = build(&document, &BuildOptions::default(), &mut NoPictures).document;
    assert_eq!(built.styles[0].name, "Body Text");
    assert!(built.styles.iter().any(|style| style.name == "Heading 1"));
    let Block::Paragraph(first) = &body(&built)[0] else {
        panic!("a paragraph");
    };
    assert_eq!(built.styles[first.style].name, "Heading 1");
}

#[test]
fn runs_carry_only_what_differs_from_their_style() {
    let mut document = Document::with_paragraphs([("plain bold plain", StyleId(1))]);
    document.paragraph_mut(0).runs = vec![CharacterRun {
        start: 6,
        end: 10,
        format: CharacterFormat {
            bold: Some(true),
            underline: Some(true),
            ..CharacterFormat::default()
        },
    }];
    let built = build(&document, &BuildOptions::default(), &mut NoPictures).document;
    let runs = inlines(&body(&built)[0]);
    assert_eq!(runs.len(), 3);
    assert_eq!(
        runs[1],
        Inline::Text {
            text: "bold".into(),
            props: CharProps {
                bold: Some(true),
                underline: Some(Underline::Single),
                ..CharProps::default()
            },
        }
    );
    assert_eq!(
        runs[0],
        Inline::Text {
            text: "plain ".into(),
            props: CharProps::default(),
        }
    );
}

#[test]
fn lower_case_and_initial_caps_are_written_as_shown() {
    let mut document = Document::with_paragraphs([("hello world", StyleId(1))]);
    document.paragraph_mut(0).runs = vec![CharacterRun {
        start: 0,
        end: 11,
        format: CharacterFormat {
            caps: Some(amipro_document::Caps::Initial),
            ..CharacterFormat::default()
        },
    }];
    let built = build(&document, &BuildOptions::default(), &mut NoPictures).document;
    let Inline::Text { text, .. } = &inlines(&body(&built)[0])[0] else {
        panic!("text");
    };
    assert_eq!(text, "Hello World");
}

#[test]
fn a_date_is_a_field_with_ami_pros_result() {
    let mut document =
        Document::with_paragraphs([(format!("Today {OBJECT_REPLACEMENT_CHARACTER}"), StyleId(1))]);
    document.paragraph_mut(0).objects = vec![InlineObject::Date(DateField {
        source: AmiDate::Today,
        format: b'b',
    })];
    let options = BuildOptions {
        fields: amipro_document::FieldContext {
            now: amipro_document::WallClock::from_unix(1_792_521_238),
            ..amipro_document::FieldContext::default()
        },
        ..BuildOptions::default()
    };
    let built = build(&document, &options, &mut NoPictures).document;
    let Inline::Field(field) = &inlines(&body(&built)[0])[1] else {
        panic!("a field");
    };
    assert_eq!(
        field.kind,
        FieldKind::Date {
            source: DateSource::Now,
            picture: "MMMM d, yyyy".into(),
            upper: false,
        }
    );
    assert_eq!(field.result, "October 20, 2026");
    let package = docx::parts(&built);
    let main = package
        .iter()
        .find(|entry| entry.name == "word/document.xml")
        .unwrap();
    let xml = String::from_utf8_lossy(&main.bytes);
    assert!(xml.contains(r#"DATE \@ &quot;MMMM d, yyyy&quot;"#), "{xml}");
    assert!(xml.contains("October 20, 2026"));
}

#[test]
fn numbered_paragraphs_are_a_live_list_showing_ami_pros_numbers() {
    let mut document = Document::with_paragraphs([
        ("one", StyleId(1)),
        ("two", StyleId(1)),
        ("break", StyleId(2)),
        ("one again", StyleId(1)),
    ]);
    let body_text = document.styles_mut().get_mut(&StyleId(1)).unwrap();
    body_text.bullets = Bullets {
        text: "\u{1A}.".into(),
        ..Bullets::default()
    };
    let built = build(&document, &BuildOptions::default(), &mut NoPictures).document;
    let lists: Vec<_> = body(&built)
        .iter()
        .map(|block| match block {
            Block::Paragraph(paragraph) => (paragraph.list, paragraph.number_text.clone()),
            Block::Table(_) => panic!(),
        })
        .collect();
    // A style without a level counts its run: the fourth starts again, in
    // an instance of its own.
    let instance = |at: usize| lists[at].0.and_then(|(instance, _)| instance);
    assert_eq!(lists[0].1.as_deref(), Some("1."));
    assert_eq!(lists[1].1.as_deref(), Some("2."));
    assert_eq!(lists[2], (None, None));
    assert_eq!(lists[3].1.as_deref(), Some("1."));
    assert!(instance(0).is_some());
    assert_eq!(instance(0), instance(1));
    assert_ne!(instance(0), instance(3));
    let started = built.list_instances[instance(3).unwrap()].starts[0];
    assert_eq!(started, Some(1));
    assert_eq!(built.lists[0].levels[0].text, "%1.");
    assert_eq!(
        text::write(&built),
        b"1.\tone\n2.\ttwo\nbreak\n1.\tone again\n"
    );
}

#[test]
fn a_docx_is_a_zip_of_well_formed_parts() {
    let document = Document::with_paragraphs([("a < b & c", StyleId(1))]);
    let built = build(&document, &BuildOptions::default(), &mut NoPictures);
    let archive = docx::write(&built.document, None);
    let parts = zip::read_stored(&archive);
    let names: Vec<_> = parts.iter().map(|(name, _)| name.as_str()).collect();
    for wanted in [
        "[Content_Types].xml",
        "_rels/.rels",
        "word/document.xml",
        "word/styles.xml",
        "word/numbering.xml",
        "docProps/core.xml",
    ] {
        assert!(names.contains(&wanted), "{wanted} in {names:?}");
    }
    let document_xml = &parts
        .iter()
        .find(|(name, _)| name == "word/document.xml")
        .unwrap()
        .1;
    let xml = String::from_utf8_lossy(document_xml);
    assert!(xml.contains("a &lt; b &amp; c"));
    assert!(losses::losses(&built.features, Format::Docx).is_empty());
}

#[test]
fn plain_text_lists_what_it_leaves_out() {
    // EX7, EC13: a frame's text, pictures, and tables are what plain text
    // cannot hold; Word Document holds them.
    let mut document = Document::with_paragraphs([("Text", StyleId(1))]);
    let flow = document.create_flow(StyleId(1));
    let frame = amipro_document::Frame {
        id: document.allocate_frame_id(),
        placement: amipro_document::Placement::Fixed {
            page: 1,
            rect: amipro_document::Rect::new(1_440, 1_440, 2_880, 2_880),
        },
        style: amipro_document::FrameStyle::default(),
        group: None,
        content: amipro_document::FrameContent::Text(flow),
        legacy: Default::default(),
    };
    let id = frame.id;
    document.insert_frame(frame);
    document.frame_stack_mut().push(id);
    let built = build(&document, &BuildOptions::default(), &mut NoPictures);
    let text = losses::losses(&built.features, Format::Text);
    assert_eq!(text.len(), 1);
    assert!(
        text[0].message().starts_with("frames"),
        "{}",
        text[0].message()
    );
    assert!(losses::losses(&built.features, Format::Docx).is_empty());
}
