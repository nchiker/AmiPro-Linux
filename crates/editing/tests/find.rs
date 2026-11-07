//! Find & Replace by Ami Pro's rules (`docs/research/FIND_NOTES.md`): each
//! test is a VM fixture (FR1–FR7) or an example from the user's guide, ch.
//! 20.

use amipro_document::{
    Document, FlowId, FrameContent, FrameStyle, HeaderFooterKind, PageSides, Placement, Rect,
    RepeatPages, Size, StyleId,
};
use amipro_editing::find::{
    Attributes, FindOptions, StreamKind, find_all, replace_all, replacement, search_streams,
    set_in_capitals_of, streams,
};
use amipro_editing::{Command, NewFrameContent, TextPosition, apply};

/// FR1's three paragraphs, as the fixture has them.
fn fr1() -> Document {
    Document::from_plain_text(
        "status bar. Status Bar. STATUS BAR. Status bar. sTatus bar.\n\
         the theater then The thesis THE the.\n\
         phone phones phoned phoning telephone.",
    )
}

fn main_texts(document: &Document) -> Vec<String> {
    document
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect()
}

/// Replace All in the main text as Ami Pro does it from the main text.
fn replace_everywhere(
    document: &Document,
    find: &str,
    replace: &str,
    options: &FindOptions,
) -> Document {
    let streams = search_streams(document, document.main_flow_id(), options);
    let found = find_all(document, find, &streams, options, None);
    match replace_all(document, &found, replace, options) {
        Some(command) => apply(document, &command).unwrap().document,
        None => document.clone(),
    }
}

#[test]
fn replace_all_takes_the_found_texts_capitals_fr1() {
    let replaced = replace_everywhere(&fr1(), "status bar", "window pane", &FindOptions::default());
    assert_eq!(
        main_texts(&replaced)[0],
        "window pane. Window Pane. WINDOW PANE. Window Pane. window pane."
    );
    replaced.validate().unwrap();
}

#[test]
fn whole_words_only_and_the_typed_case_dropped_fr1b() {
    let replaced = replace_everywhere(&fr1(), "the", "aN", &FindOptions::default());
    assert_eq!(main_texts(&replaced)[1], "an theater then An thesis AN an.");
    // Without Whole word only the words that hold "the" change too.
    let options = FindOptions {
        whole_word: false,
        ..FindOptions::default()
    };
    let found = find_all(&fr1(), "the", &[fr1().main_flow_id()], &options, None);
    assert_eq!(found.len(), 7);
}

#[test]
fn replace_exact_case_keeps_the_replacement_as_typed_fr2() {
    let options = FindOptions {
        keep_capitals: false,
        ..FindOptions::default()
    };
    let replaced = replace_everywhere(&fr1(), "status bar", "Window PANE", &options);
    assert_eq!(
        main_texts(&replaced)[0],
        "Window PANE. Window PANE. Window PANE. Window PANE. Window PANE."
    );
}

#[test]
fn find_exact_case_matches_capitals() {
    let options = FindOptions {
        match_case: true,
        ..FindOptions::default()
    };
    let document = fr1();
    let found = find_all(
        &document,
        "Status",
        &[document.main_flow_id()],
        &options,
        None,
    );
    assert_eq!(
        found.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(),
        ["Status", "Status"]
    );
}

#[test]
fn an_asterisk_takes_the_rest_of_the_word_into_the_replacement_fr3() {
    let options = FindOptions {
        whole_word: false,
        ..FindOptions::default()
    };
    let replaced = replace_everywhere(&fr1(), "phon*", "telephon*", &options);
    assert_eq!(
        main_texts(&replaced)[2],
        "telephone telephones telephoned telephoning teletelephone."
    );
}

#[test]
fn wildcards_follow_the_users_guide() {
    let options = FindOptions {
        whole_word: false,
        ..FindOptions::default()
    };
    let document = Document::from_plain_text("meeting sheets organize organized organizing");
    let main = document.main_flow_id();
    // Each wildcard of the replacement takes the one in the same place.
    let replaced = replace_everywhere(&document, "*eet*", "meet*", &options);
    assert_eq!(
        main_texts(&replaced)[0],
        "meetm meetsh organize organized organizing"
    );
    // An asterisk matches at least one character.
    let words = |find: &str| {
        find_all(&document, find, &[main], &options, None)
            .into_iter()
            .map(|found| found.text)
            .collect::<Vec<_>>()
    };
    assert_eq!(words("organize*"), ["organized"]);
    assert_eq!(words("organiz*"), ["organize", "organized", "organizing"]);
    // A question mark is one character; with Whole word only, th?n.
    let thin = Document::from_plain_text("than then thin thon thnn thorn then.");
    let found = find_all(
        &thin,
        "th?n",
        &[thin.main_flow_id()],
        &FindOptions::default(),
        None,
    );
    assert_eq!(
        found.iter().map(|m| m.text.as_str()).collect::<Vec<_>>(),
        ["than", "then", "thin", "thon", "thnn", "then"]
    );
    // <?> and <*> are the characters themselves.
    let marks = Document::from_plain_text("Why? 2*3 is six");
    let main = marks.main_flow_id();
    let literal = |find: &str| find_all(&marks, find, &[main], &options, None).len();
    assert_eq!(literal("y<?>"), 1);
    assert_eq!(literal("2<*>3"), 1);
    assert_eq!(literal("x<?>"), 0);
}

#[test]
fn tabs_and_returns_are_found_and_replaced() {
    let document = Document::from_plain_text("one\ttwo\nthree\nfour");
    let main = document.main_flow_id();
    let options = FindOptions {
        whole_word: false,
        ..FindOptions::default()
    };
    // Ami Pro's marks for a tab and a return.
    let found = find_all(&document, "»", &[main], &options, None);
    assert_eq!(found.len(), 1);
    let found = find_all(&document, "e¶f", &[main], &options, None);
    assert_eq!(found.len(), 1);
    // Deleting a return joins the paragraphs (the guide, "To delete
    // returns").
    let joined = replace_everywhere(&document, "¶", " ", &options);
    assert_eq!(main_texts(&joined), ["one\ttwo three four"]);
    // A return in the replacement splits.
    let split = replace_everywhere(&document, "»", "¶", &options);
    assert_eq!(main_texts(&split), ["one", "two", "three", "four"]);
}

#[test]
fn the_capitals_rule() {
    assert_eq!(
        set_in_capitals_of("window pane", "STATUS BAR"),
        "WINDOW PANE"
    );
    assert_eq!(
        set_in_capitals_of("window pane", "Status bar"),
        "Window Pane"
    );
    assert_eq!(
        set_in_capitals_of("WINDOW pane", "sTatus bar"),
        "window pane"
    );
    assert_eq!(set_in_capitals_of("aN", "The"), "An");
    assert_eq!(set_in_capitals_of("x-ray", "A"), "X-Ray");
    assert_eq!(set_in_capitals_of("12 pt", "10"), "12 pt");
}

/// A document with one text in every kind of stream, made through the
/// commands: "alpha" everywhere (FR5).
fn every_stream() -> (Document, Vec<(StreamKind, FlowId)>) {
    let mut document = Document::from_plain_text("alpha in the main text\nanchor here\nlast");
    let mut made = Vec::new();
    let main = document.main_flow_id();
    let at = |paragraph: usize| TextPosition {
        flow: main,
        paragraph,
        byte: 0,
    };
    let create = |document: &mut Document, placement: Placement, content, anchor, layout| {
        let applied = apply(
            document,
            &Command::CreateFrame {
                placement,
                style: FrameStyle {
                    wrap: if matches!(placement, Placement::BelowParagraph { .. }) {
                        amipro_document::Wrap::NoWrapAround
                    } else {
                        amipro_document::Wrap::Around
                    },
                    ..FrameStyle::default()
                },
                content,
                anchor,
                layout,
            },
        )
        .unwrap();
        *document = applied.document;
        *applied.report.frames.iter().next().unwrap()
    };
    let size = Size {
        width: 2_880,
        height: 1_440,
    };
    let rect = Rect::new(1_440, 1_440, 4_320, 2_880);
    let fixed = create(
        &mut document,
        Placement::Fixed { page: 1, rect },
        NewFrameContent::Text,
        None,
        None,
    );
    let table = create(
        &mut document,
        Placement::BelowParagraph { x_offset: 0, size },
        NewFrameContent::Table {
            columns: vec![1_440, 1_440],
            rows: vec![284],
        },
        Some(at(1)),
        None,
    );
    let layout = document.document_layout().id;
    // A new document's layout has its header and footer frames.
    let header = document.document_layout().header.right.unwrap();
    let footer = document.document_layout().footer.right.unwrap();
    let repeating = create(
        &mut document,
        Placement::Repeating {
            pages: RepeatPages::All,
            rect,
        },
        NewFrameContent::Text,
        None,
        Some(layout),
    );
    let floating = apply(
        &document,
        &Command::InsertFloatingHeaderFooter {
            at: TextPosition {
                flow: main,
                paragraph: 2,
                byte: 4,
            },
            kind: HeaderFooterKind::Footer,
            pages: PageSides::All,
        },
    )
    .unwrap();
    let floating_flow = floating.report.added_flows[0];
    document = floating.document;
    let flows_of = |document: &Document, frame| match &document.frame(frame).unwrap().content {
        FrameContent::Text(flow) => vec![*flow],
        FrameContent::Table(table) => table
            .cells
            .iter()
            .map(|cell| match cell.content {
                amipro_document::CellContent::Text(flow) => flow,
            })
            .collect(),
        _ => Vec::new(),
    };
    made.push((StreamKind::Main, main));
    for flow in flows_of(&document, fixed) {
        made.push((StreamKind::PlacedFrame, flow));
    }
    for flow in flows_of(&document, table) {
        made.push((StreamKind::AnchoredFrame, flow));
    }
    made.push((StreamKind::FloatingHeaderFooter, floating_flow));
    for flow in flows_of(&document, header) {
        made.push((StreamKind::Header, flow));
    }
    for flow in flows_of(&document, footer) {
        made.push((StreamKind::Footer, flow));
    }
    for flow in flows_of(&document, repeating) {
        made.push((StreamKind::RepeatingFrame, flow));
    }
    for (_, flow) in &made[1..] {
        document
            .flow_mut(*flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "alpha here".into();
    }
    document.validate().unwrap();
    (document, made)
}

#[test]
fn streams_go_in_ami_pros_order() {
    let (document, made) = every_stream();
    let found: Vec<(StreamKind, FlowId)> = streams(&document)
        .into_iter()
        .map(|stream| (stream.kind, stream.flow))
        .collect();
    assert_eq!(found, made);
}

#[test]
fn other_text_streams_are_searched_after_the_carets_fr5_fr6() {
    let (document, made) = every_stream();
    let main = document.main_flow_id();
    // From the main text with Include other text streams: everywhere (FR5).
    let replaced = replace_everywhere(&document, "alpha", "beta", &FindOptions::default());
    for (_, flow) in &made {
        let text = &replaced.flow(*flow).unwrap().paragraphs[0].text;
        assert!(text.starts_with("beta"), "{text}");
    }
    // Without it, the main text alone (FR6).
    let options = FindOptions {
        other_streams: false,
        ..FindOptions::default()
    };
    assert_eq!(search_streams(&document, main, &options), [main]);
    // From another stream, that stream and those after it.
    let header = made
        .iter()
        .find(|(kind, _)| *kind == StreamKind::Header)
        .unwrap()
        .1;
    let after = search_streams(&document, header, &FindOptions::default());
    assert_eq!(after.len(), 3);
    assert_eq!(after[0], header);
    // With Beginning of document, the whole document (the guide).
    let whole = FindOptions {
        from_beginning: true,
        ..FindOptions::default()
    };
    assert_eq!(search_streams(&document, header, &whole).len(), made.len());
    assert_eq!(search_streams(&document, header, &whole)[0], main);
    // Every match in stream order (FR7: the cell's, then the header's).
    let found = find_all(
        &document,
        "alpha",
        &search_streams(&document, main, &FindOptions::default()),
        &FindOptions::default(),
        None,
    );
    let flows: Vec<FlowId> = found.iter().map(|found| found.start.flow).collect();
    assert_eq!(
        flows,
        made.iter().map(|(_, flow)| *flow).collect::<Vec<_>>()
    );
}

#[test]
fn replace_all_is_one_undoable_step() {
    let document = fr1();
    let options = FindOptions::default();
    let found = find_all(&document, "the", &[document.main_flow_id()], &options, None);
    let command = replace_all(&document, &found, "a", &options).unwrap();
    let applied = apply(&document, &command).unwrap();
    // One command, one new root: the original is untouched.
    assert_eq!(
        main_texts(&document)[1],
        "the theater then The thesis THE the."
    );
    assert_eq!(
        main_texts(&applied.document)[1],
        "a theater then A thesis A a."
    );
    assert!(!applied.report.is_empty());
    // Nothing found, nothing to do.
    assert!(replace_all(&document, &[], "a", &options).is_none());
}

#[test]
fn a_selection_limits_the_search() {
    let document = fr1();
    let main = document.main_flow_id();
    let within = (
        TextPosition {
            flow: main,
            paragraph: 0,
            byte: 12,
        },
        TextPosition {
            flow: main,
            paragraph: 1,
            byte: 3,
        },
    );
    let options = FindOptions::default();
    let found = find_all(&document, "status bar", &[main], &options, Some(within));
    assert_eq!(found.len(), 4);
    let found = find_all(&document, "the", &[main], &options, Some(within));
    assert_eq!(found.len(), 1);
}

#[test]
fn paragraph_styles_are_found_and_replaced() {
    let document = Document::with_paragraphs([
        ("one", StyleId(1)),
        ("two", StyleId(2)),
        ("three", StyleId(1)),
    ]);
    let body = document.styles()[&StyleId(1)].name.clone();
    let other = document.styles()[&StyleId(2)].name.clone();
    let options = FindOptions {
        styles: true,
        ..FindOptions::default()
    };
    let main = document.main_flow_id();
    let found = find_all(&document, &body.to_uppercase(), &[main], &options, None);
    assert_eq!(found.len(), 2);
    let command = replace_all(&document, &found, &other, &options).unwrap();
    let replaced = apply(&document, &command).unwrap().document;
    assert!(
        replaced
            .paragraphs()
            .iter()
            .all(|paragraph| paragraph.style == StyleId(2))
    );
    // A style that does not exist finds nothing.
    assert!(find_all(&document, "No such style", &[main], &options, None).is_empty());
}

#[test]
fn replacements_fill_wildcards_and_codes() {
    let document = Document::from_plain_text("phones");
    let options = FindOptions {
        whole_word: false,
        keep_capitals: false,
        ..FindOptions::default()
    };
    let found = &find_all(&document, "ph*", &[document.main_flow_id()], &options, None)[0];
    assert_eq!(found.captures, ["ones"]);
    assert_eq!(replacement(found, "f*»x¶", &options), "fones\tx\n");
    // A wildcard with nothing to take is left out.
    assert_eq!(replacement(found, "a*b*", &options), "aonesb");
}

#[test]
fn attributes_default_to_none() {
    assert_eq!(FindOptions::default().find_attributes, None);
    assert_eq!(Attributes::default(), Attributes::default());
}

/// "status bar" three times, the second in italics and the third word
/// underlined, as the guide's examples have it.
fn attributed() -> Document {
    let document = Document::from_plain_text("status bar, status bar, status bar.");
    let main = document.main_flow_id();
    let at = |byte| TextPosition {
        flow: main,
        paragraph: 0,
        byte,
    };
    let mut document = document;
    for (start, attribute) in [
        (12, amipro_editing::CharacterAttribute::Italic),
        (24, amipro_editing::CharacterAttribute::WordUnderline),
    ] {
        document = apply(
            &document,
            &Command::SetCharacterAttribute {
                start: at(start),
                end: at(start + 10),
                attribute,
                on: true,
            },
        )
        .unwrap()
        .document;
    }
    document
}

#[test]
fn find_exact_attributes_finds_only_text_with_them() {
    let document = attributed();
    let main = document.main_flow_id();
    let with = |attributes: Attributes| FindOptions {
        find_attributes: Some(attributes),
        ..FindOptions::default()
    };
    let starts = |options: &FindOptions| {
        find_all(&document, "status bar", &[main], options, None)
            .iter()
            .map(|found| found.start.byte)
            .collect::<Vec<_>>()
    };
    let italic = Attributes {
        italic: true,
        ..Attributes::default()
    };
    let word_underline = Attributes {
        word_underline: true,
        ..Attributes::default()
    };
    assert_eq!(starts(&with(italic)), [12]);
    assert_eq!(starts(&with(word_underline)), [24]);
    // Normal is none of them.
    assert_eq!(starts(&with(Attributes::default())), [0]);
    assert_eq!(starts(&FindOptions::default()), [0, 12, 24]);
}

#[test]
fn replace_exact_attributes_sets_exactly_them() {
    let document = attributed();
    // Find Word underline, replace with Bold and Underline (the guide).
    let options = FindOptions {
        find_attributes: Some(Attributes {
            word_underline: true,
            ..Attributes::default()
        }),
        replace_attributes: Some(Attributes {
            bold: true,
            underline: true,
            ..Attributes::default()
        }),
        ..FindOptions::default()
    };
    let replaced = replace_everywhere(&document, "status bar", "status bar", &options);
    let paragraph = &replaced.paragraphs()[0];
    let style = &replaced.styles()[&paragraph.style];
    let shown = |byte| amipro_editing::find::shown_attributes(paragraph.format_at(byte), style);
    assert_eq!(
        shown(26),
        Attributes {
            bold: true,
            underline: true,
            ..Attributes::default()
        }
    );
    // The others keep theirs.
    assert!(shown(14).italic);
    assert_eq!(shown(2), Attributes::default());
    // Every "status bar" given word underline whatever it had.
    let options = FindOptions {
        replace_attributes: Some(Attributes {
            word_underline: true,
            ..Attributes::default()
        }),
        ..FindOptions::default()
    };
    let replaced = replace_everywhere(&document, "status bar", "status bar", &options);
    let paragraph = &replaced.paragraphs()[0];
    let style = &replaced.styles()[&paragraph.style];
    for byte in [2, 14, 26] {
        assert_eq!(
            amipro_editing::find::shown_attributes(paragraph.format_at(byte), style),
            Attributes {
                word_underline: true,
                ..Attributes::default()
            },
            "{byte}"
        );
    }
}
