//! Phase 4.11: documents edited through commands, written as Ami Pro
//! documents and read back. Every kind of content is edited (text in each
//! flow, styles, character formatting, a frame moved and resized, a table
//! cell, a page break); the copy read back must say what the edited
//! document says, and the loss check must find nothing.

use amipro_document::{
    BreakKind, Document, FlowId, FrameContent, FrameId, FrameStyle, Placement, Rect, Sides, Size,
};
use amipro_editing::{CharacterAttribute, Command, NewFrameContent, TextPosition, apply};
use amipro_formats::roundtrip::{differences, is_writer_addition};
use amipro_formats::{SamOptions, import_sam, sam_document, sam_losses};

const OPTIONS: SamOptions = SamOptions::at(1_791_958_088);

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

fn type_at(document: &Document, position: TextPosition, text: &str) -> Document {
    run(
        document,
        Command::ReplaceText {
            start: position,
            end: position,
            text: text.into(),
        },
    )
}

fn text_flow(document: &Document, frame: FrameId) -> FlowId {
    match document.frame(frame).unwrap().content {
        FrameContent::Text(flow) => flow,
        ref other => panic!("not a text frame: {other:?}"),
    }
}

/// Writes `document`, reads it back, and returns what differs beyond what
/// the writer adds, with the written file.
fn round_trip(document: &Document) -> (Vec<String>, Vec<u8>) {
    let written = sam_document(document, &OPTIONS);
    let copy = import_sam(&written, None, "Edited").unwrap();
    let mut found: Vec<String> = differences(document, &copy.document)
        .into_iter()
        // The fonts' printer line heights and kinds are measured from what
        // is written (the loss check leaves them out too): a cache an edit
        // dropped takes its calibration with it.
        .filter(|difference| {
            !is_writer_addition(difference)
                && !matches!(difference.aspect.as_str(), "face kinds" | "line heights")
        })
        .map(|difference| format!("{difference:?}"))
        .collect();
    found.extend(
        copy.report
            .warnings
            .iter()
            .filter(|warning| {
                warning.starts_with("table cell record") || warning.contains("reads on past")
            })
            .cloned(),
    );
    (found, written)
}

#[test]
fn a_document_made_and_edited_with_commands_reads_back_as_it_was() {
    let mut document =
        Document::from_plain_text("First paragraph of the report.\nSecond paragraph.\nThird.");
    let main = document.main_flow_id();

    // Text in the header every document has.
    let header = document.document_layout().header.right.unwrap();
    document = type_at(
        &document,
        at(text_flow(&document, header), 0, 0),
        "Report header",
    );

    // A Where placed text frame with lines, then moved and resized.
    let mut lined = FrameStyle::default();
    lined.border.sides = Sides::ALL;
    document = run(
        &document,
        Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: Rect::new(2880, 5040, 7200, 7200),
            },
            style: lined,
            content: NewFrameContent::Text,
            anchor: None,
            layout: None,
        },
    );
    let frame = document.frame_stack()[0];
    document = type_at(
        &document,
        at(text_flow(&document, frame), 0, 0),
        "Frame text",
    );
    document = run(
        &document,
        Command::MoveFrame {
            frame,
            dx: 720,
            dy: 360,
        },
    );
    document = run(
        &document,
        Command::ResizeFrame {
            frame,
            rect: Rect::new(3600, 5400, 8640, 8000),
        },
    );

    // An anchored table below the second paragraph, with text in one cell.
    document = run(
        &document,
        Command::CreateFrame {
            placement: Placement::BelowParagraph {
                x_offset: 0,
                size: Size {
                    width: 4086,
                    height: 1000,
                },
            },
            style: FrameStyle {
                wrap: amipro_document::Wrap::NoWrapAround,
                ..FrameStyle::default()
            },
            content: NewFrameContent::Table {
                columns: vec![2000, 2000],
                rows: vec![284, 284],
            },
            anchor: Some(at(main, 1, "Second paragraph.".len())),
            layout: None,
        },
    );
    let table = *document
        .frame_stack()
        .iter()
        .find(|id| {
            matches!(
                document.frame(**id).unwrap().content,
                FrameContent::Table(_)
            )
        })
        .unwrap();
    let FrameContent::Table(cells) = &document.frame(table).unwrap().content else {
        unreachable!()
    };
    let amipro_document::CellContent::Text(cell) = cells.cells[1].content;
    document = type_at(&document, at(cell, 0, 0), "B1");

    // A style made and applied, bold text, and a page break.
    let body = document.paragraphs()[0].style;
    document = run(
        &document,
        Command::CreateStyle {
            name: "Report Title".into(),
            based_on: body,
        },
    );
    let heading = document
        .styles()
        .values()
        .find(|style| style.name == "Report Title")
        .unwrap()
        .id;
    let mut modified = document.styles()[&heading].clone();
    modified.font_size_points = 18.0;
    document = run(&document, Command::ModifyStyle { style: modified });
    document = run(
        &document,
        Command::SetParagraphStyle {
            flow: main,
            first: 0,
            last: 0,
            style: heading,
        },
    );
    document = run(
        &document,
        Command::SetCharacterAttribute {
            start: at(main, 2, 0),
            end: at(main, 2, "Third".len()),
            attribute: CharacterAttribute::Bold,
            on: true,
        },
    );
    document = run(
        &document,
        Command::InsertBreak {
            at: at(main, 2, 0),
            kind: BreakKind::Page,
            centre: false,
            layout: None,
        },
    );

    let (found, written) = round_trip(&document);
    let written = String::from_utf8_lossy(&written);
    assert_eq!(found, Vec::<String>::new(), "{written}");
    assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
    // The frame with lines says so (0x10000), and the cell typed in says it
    // has text (its eighth value).
    assert!(
        written.contains("[frm]\r\n\t1\r\n\t66240\r\n\t3600\r\n\t5400\r\n\t8640\r\n\t8000\r\n")
    );
    assert!(written.contains("\t\t\t 0 1 0 0 0 0 0 1 0 0 0 0\r\nB1\r\n"));
}

/// A small Ami Pro document with paragraph caches: two Body Text
/// paragraphs and a Title.
const CACHED: &str = "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n\
    [tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tTimes New Roman\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
    [tag]\r\n\tTitle\r\n\t3\r\n\t[fnt]\r\n\t\tArial\r\n\t\t360\r\n\t\t0\r\n\t\t16385\r\n\
    [edoc]\r\n@Title@<:#400,9360>A title\r\n\r\n<:#284,9360>Body one.\r\n\r\n<:#284,9360>Body two.\r\n\r\n>\r\n";

#[test]
fn edits_drop_exactly_the_caches_they_make_stale() {
    let document = import_sam(CACHED.as_bytes(), None, "Cached")
        .unwrap()
        .document;
    let main = document.main_flow_id();
    let written =
        |document: &Document| String::from_utf8(sam_document(document, &OPTIONS)).unwrap();
    assert!(written(&document).contains(
        "@Title@<:#400,9360>A title\r\n\r\n<:#284,9360>Body one.\r\n\r\n<:#284,9360>Body two."
    ));

    // Typing in one paragraph drops its cache only.
    let typed = type_at(&document, at(main, 1, "Body one".len()), " more");
    let text = written(&typed);
    assert!(text.contains("<:#400,9360>A title"), "{text}");
    assert!(text.contains("\r\nBody one more.\r\n"), "{text}");
    assert!(text.contains("<:#284,9360>Body two."), "{text}");
    assert_eq!(round_trip(&typed).0, Vec::<String>::new());

    // Changing Body Text drops the cache of every Body Text paragraph.
    let body = document.paragraphs()[1].style;
    let mut larger = document.styles()[&body].clone();
    larger.font_size_points = 14.0;
    let restyled = run(&document, Command::ModifyStyle { style: larger });
    let text = written(&restyled);
    assert!(text.contains("<:#400,9360>A title"), "{text}");
    assert!(!text.contains("<:#284"), "{text}");
    assert_eq!(round_trip(&restyled).0, Vec::<String>::new());

    // A paragraph given another style loses its cache; a split keeps it on
    // neither part that changed.
    let retagged = run(
        &document,
        Command::SetParagraphStyle {
            flow: main,
            first: 2,
            last: 2,
            style: document.paragraphs()[0].style,
        },
    );
    let text = written(&retagged);
    assert!(text.contains("@Title@Body two."), "{text}");
    let split = type_at(&document, at(main, 1, "Body".len()), "\n");
    let text = written(&split);
    assert!(text.contains("\r\nBody\r\n\r\n one.\r\n"), "{text}");
    assert_eq!(round_trip(&split).0, Vec::<String>::new());
}

#[test]
fn rulers_and_indents_set_by_commands_write_as_ami_pro_writes_them() {
    use amipro_document::{TabKind, TabStop};
    use amipro_editing::IndentChange;
    let document = import_sam(CACHED.as_bytes(), None, "Ruled")
        .unwrap()
        .document;
    let main = document.main_flow_id();
    let written =
        |document: &Document| String::from_utf8(sam_document(document, &OPTIONS)).unwrap();
    // RU1: the box's twelve stops copied with one added, unsnapped; the
    // paragraph after it ends the ruler (RU3).
    let mut tabs: Vec<TabStop> = (1..=12)
        .map(|stop| TabStop::new(TabKind::Left, 720 * stop))
        .collect();
    tabs.push(TabStop::new(TabKind::Left, 3228));
    let ruled = run(
        &document,
        Command::SetParagraphTabs {
            flow: main,
            first: 1,
            last: 1,
            tabs: Some(tabs),
        },
    );
    // RU5: First 0.5 in on the last paragraph.
    let indented = run(
        &ruled,
        Command::SetParagraphIndents {
            flow: main,
            first: 2,
            last: 2,
            indents: IndentChange {
                first_line: Some(720),
                ..IndentChange::default()
            },
        },
    );
    let text = written(&indented);
    assert!(
        text.contains(
            "\r\n<:R1,13,1,720,1,1440,1,2160,1,2880,1,3228,1,3600,1,4320,1,5040,1,5760,1,6480,1,7200,1,7920,1,8640,>Body one.\r\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("\r\n<:I0,720,0,0><:R>Body two.\r\n"),
        "{text}"
    );
    assert!(
        text.contains("<:#400,9360>A title"),
        "an untouched cache stays"
    );
    assert_eq!(round_trip(&indented).0, Vec::<String>::new());
    assert_eq!(sam_losses(&indented, &OPTIONS).unwrap(), []);
    // Removing the ruler removes the `<:R>` after it too.
    let removed = run(
        &indented,
        Command::SetParagraphTabs {
            flow: main,
            first: 1,
            last: 1,
            tabs: None,
        },
    );
    let text = written(&removed);
    assert!(!text.contains("<:R"), "{text}");
    assert_eq!(round_trip(&removed).0, Vec::<String>::new());
}

#[test]
fn uneven_columns_made_on_the_ruler_read_back_in_both_formats() {
    use amipro_editing::LayoutSettings;
    use amipro_editing::layout_setup::{self, Pages};
    let mut document = Document::from_plain_text("Main text\nmore text");
    // The page: RU2's columns, as the ruler's gap drag leaves them.
    let layout = document.document_layout().id;
    let mut settings = LayoutSettings::of(&document, layout).unwrap();
    settings.set_ruler_columns(None, Pages::All, 2).unwrap();
    settings
        .set_page_edges(Pages::All, &[(1_440, 7_050), (7_290, 10_800)])
        .unwrap();
    document = run(&document, Command::ModifyLayout { layout, settings });
    // A frame with three uneven columns inside.
    document = run(
        &document,
        Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: Rect::new(1_440, 5_000, 7_200, 8_000),
            },
            style: FrameStyle::default(),
            content: NewFrameContent::Text,
            anchor: None,
            layout: None,
        },
    );
    let frame = *document.frame_stack().last().unwrap();
    let mut style = document.frame(frame).unwrap().style.clone();
    layout_setup::ruler_columns(&mut style.interior, 3).unwrap();
    layout_setup::set_box_edges(
        &mut style.interior,
        5_760,
        &[(144, 1_500), (1_740, 3_900), (4_500, 5_616)],
    )
    .unwrap();
    document = run(&document, Command::SetFrameStyle { frame, style });
    assert!(
        document
            .document_layout()
            .right
            .layout
            .column_edges
            .is_some()
    );
    assert!(
        document
            .frame(frame)
            .unwrap()
            .style
            .interior
            .column_edges
            .is_some()
    );
    // SAM: read back as it was, nothing in the loss check.
    let (found, written) = round_trip(&document);
    assert_eq!(found, Vec::<String>::new());
    let text = String::from_utf8_lossy(&written);
    assert!(text.contains("\t\t2\r\n\t\t1440\r\n\t\t7050\r\n"), "{text}");
    assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
    // Native v3 keeps them exactly.
    let encoded = document.encode_native().unwrap();
    assert_eq!(Document::decode_native(&encoded).unwrap(), document);
}

/// Edited copies of real documents for opening in Ami Pro
/// (`docs/research/VM_LAB.md`): each `NAME.SAM` in `AMIPRO_VM_EDIT` (the
/// lab's `IN` folder) gets text typed in its main text and in a table
/// cell, a Where placed frame moved and widened, a paragraph's style
/// changed and a style modified, bold text, and a page break, and is
/// written as `EDNAME.SAM` beside it after reading back as it was.
#[test]
#[ignore = "writes edited documents for the VM"]
fn edited_documents_for_ami_pro() {
    let Some(directory) = std::env::var_os("AMIPRO_VM_EDIT") else {
        eprintln!("AMIPRO_VM_EDIT is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let names = std::env::var("AMIPRO_VM_EDIT_FILES").unwrap_or_else(|_| "K1 K2".into());
    for name in names.split_whitespace() {
        let source = directory.join(format!("{name}.SAM"));
        let mut document = amipro_formats::import_sam_path(&source).unwrap().document;
        let main = document.main_flow_id();
        let texts: Vec<usize> = document
            .paragraphs()
            .iter()
            .enumerate()
            .filter(|(_, paragraph)| {
                paragraph.text.len() > 8 && !paragraph.text.contains('\u{FFFC}')
            })
            .map(|(index, _)| index)
            .collect();
        // Typed text at the end of the first two text paragraphs.
        for &index in texts.iter().take(2) {
            let end = document.paragraphs()[index].text.len();
            document = type_at(&document, at(main, index, end), " [edited]");
        }
        // Bold on the first word of the third.
        if let Some(&index) = texts.get(2) {
            let word = document.paragraphs()[index].text.find(' ').unwrap_or(1);
            document = run(
                &document,
                Command::SetCharacterAttribute {
                    start: at(main, index, 0),
                    end: at(main, index, word),
                    attribute: CharacterAttribute::Bold,
                    on: true,
                },
            );
        }
        // The fourth takes the first one's style; the first's style is made
        // italic.
        if let (Some(&first), Some(&fourth)) = (texts.first(), texts.get(3)) {
            let style = document.paragraphs()[first].style;
            document = run(
                &document,
                Command::SetParagraphStyle {
                    flow: main,
                    first: fourth,
                    last: fourth,
                    style,
                },
            );
            let mut italic = document.styles()[&style].clone();
            italic.italic = !italic.italic;
            document = run(&document, Command::ModifyStyle { style: italic });
        }
        // A page break before the sixth.
        if let Some(&index) = texts.get(5) {
            document = run(
                &document,
                Command::InsertBreak {
                    at: at(main, index, 0),
                    kind: BreakKind::Page,
                    centre: false,
                    layout: None,
                },
            );
        }
        // Text typed into the first table cell that has some.
        let cell = document.frame_stack().iter().find_map(|id| {
            let FrameContent::Table(table) = &document.frame(*id)?.content else {
                return None;
            };
            table.cells.iter().find_map(|cell| {
                let amipro_document::CellContent::Text(flow) = cell.content;
                let text = &document.flow(flow)?.paragraphs[0].text;
                (!text.is_empty() && !text.contains('\u{FFFC}')).then_some((flow, text.len()))
            })
        });
        if let Some((flow, end)) = cell {
            document = type_at(&document, at(flow, 0, end), "!");
        }
        // The first Where placed frame moved right and made wider.
        let fixed = document.frame_stack().iter().copied().find(|id| {
            matches!(
                document.frame(*id).map(|frame| frame.placement),
                Some(Placement::Fixed { .. })
            )
        });
        if let Some(frame) = fixed {
            document = run(
                &document,
                Command::MoveFrame {
                    frame,
                    dx: 360,
                    dy: 0,
                },
            );
            if let Some(Placement::Fixed { rect, .. }) =
                document.frame(frame).map(|frame| frame.placement)
            {
                document = run(
                    &document,
                    Command::ResizeFrame {
                        frame,
                        rect: Rect::new(rect.left, rect.top, rect.right + 360, rect.bottom),
                    },
                );
            }
        }
        let (found, written) = round_trip(&document);
        assert_eq!(found, Vec::<String>::new(), "{name}");
        let target = directory.join(format!("ED{name}.SAM"));
        std::fs::write(&target, &written).unwrap();
        eprintln!("wrote {}", target.display());
    }
}

/// PL.11: a page layout set up through Page Layout Properties' rules and
/// the page dialogs' commands, every section used: written as native v3,
/// as an Ami Pro document, and as a style sheet with and without
/// contents, it reads back as it was, and the loss check finds nothing.
#[test]
fn a_page_layout_set_up_in_the_app_reads_back_in_every_format() {
    use amipro_document::{
        HeaderFooterKind, LinePosition, PageNumberField, PageSides, TabKind, TabLeader, TabStop,
    };
    use amipro_editing::LayoutSettings;
    use amipro_editing::layout_setup::{Band, MarginSide, Pages};
    use amipro_formats::{SheetOptions, read_style_sheet, style_sheet};
    let mut document = Document::from_plain_text(
        "First paragraph of the set-up document, long enough to wrap.\n\
         Second paragraph.\nThird paragraph.\nFourth paragraph.\nFifth paragraph.",
    );
    let main = document.main_flow_id();
    let standard = document.document_layout().id;
    let mut settings = LayoutSettings::of(&document, standard).unwrap();
    // Page, margins and columns, tabs, lines.
    settings.set_paper(4).unwrap();
    settings
        .set_margin(Pages::All, MarginSide::Top, 1_800)
        .unwrap();
    settings
        .set_margin(Pages::All, MarginSide::Bottom, 1_000)
        .unwrap();
    settings.set_columns(Pages::All, 2).unwrap();
    settings.set_gutter(Pages::All, 432).unwrap();
    settings.set_balance(Pages::All, true).unwrap();
    let mut leader = TabStop::new(TabKind::Right, 3_000);
    leader.leader = TabLeader::Dots;
    settings
        .set_tabs(Pages::All, vec![TabStop::new(TabKind::Left, 1_440), leader])
        .unwrap();
    settings
        .set_page_line_sides(Pages::All, Sides::ALL)
        .unwrap();
    settings.set_page_line_style(Pages::All, 4).unwrap();
    settings
        .set_page_line_position(Pages::All, LinePosition::CloseToInside)
        .unwrap();
    settings.set_column_line(Pages::All, true).unwrap();
    settings.set_column_line_style(Pages::All, 2).unwrap();
    settings.set_line_colour(Pages::All, 255).unwrap();
    // Header and footer bands.
    settings
        .set_band_margin(Band::Header, Pages::All, MarginSide::Left, 1_728)
        .unwrap();
    settings
        .set_band_columns(Band::Header, Pages::All, 2)
        .unwrap();
    settings
        .set_band_tabs(
            Band::Footer,
            Pages::All,
            vec![TabStop::new(TabKind::Centre, 4_000)],
        )
        .unwrap();
    settings.set_from_second_page(Band::Footer, true);
    // Left pages, mirrored.
    settings.set_mirror(Pages::Right, true).unwrap();
    settings
        .set_margin(Pages::Right, MarginSide::Left, 2_160)
        .unwrap();
    document = run(
        &document,
        Command::ModifyLayout {
            layout: standard,
            settings,
        },
    );
    // Breaks, an inserted layout in landscape, a floating header for left
    // pages with text, and a page number in the header.
    document = run(
        &document,
        Command::InsertBreak {
            at: at(main, 0, 6),
            kind: BreakKind::Page,
            centre: false,
            layout: None,
        },
    );
    document = run(
        &document,
        Command::InsertBreak {
            at: at(main, 1, 0),
            kind: BreakKind::Column,
            centre: false,
            layout: None,
        },
    );
    document = run(
        &document,
        Command::InsertLayout {
            at: at(main, 2, 5),
            from: standard,
        },
    );
    let inserted = document.layouts()[1].id;
    let mut settings = LayoutSettings::of(&document, inserted).unwrap();
    settings.set_landscape(true).unwrap();
    document = run(
        &document,
        Command::ModifyLayout {
            layout: inserted,
            settings,
        },
    );
    document = run(
        &document,
        Command::InsertFloatingHeaderFooter {
            at: at(main, 3, 4),
            kind: HeaderFooterKind::Header,
            pages: PageSides::Left,
        },
    );
    let floating = document.paragraphs()[3]
        .objects
        .iter()
        .find_map(amipro_document::InlineObject::owned_flow)
        .unwrap();
    document = type_at(&document, at(floating, 0, 0), "Left pages from here");
    let header = text_flow(&document, document.document_layout().header.right.unwrap());
    document = run(
        &document,
        Command::InsertPageNumber {
            at: at(header, 0, 0),
            field: PageNumberField {
                style: 'A',
                start_page: 2,
                start_number: 3,
                leading_text: "Page ".into(),
            },
        },
    );
    document.validate().unwrap();

    // Native v3.
    let native = Document::decode_native(&document.encode_native().unwrap()).unwrap();
    assert_eq!(native, document, "native v3");
    // An Ami Pro document, and the loss check.
    let (found, written) = round_trip(&document);
    assert!(found.is_empty(), "SAM: {found:#?}");
    assert!(
        sam_losses(&document, &OPTIONS).unwrap().is_empty(),
        "the loss check"
    );
    // A style sheet without contents: its page layout is the document's.
    let options = |contents| SheetOptions {
        description: "Set up".into(),
        saved_at: OPTIONS.saved_at,
        contents,
    };
    let sheet = style_sheet(&document, &options(false));
    let read = read_style_sheet(&sheet, "SETUP.STY").unwrap();
    let theirs = read.layout.unwrap();
    let ours = document.document_layout();
    assert_eq!(
        (
            theirs.paper,
            theirs.landscape,
            &theirs.right,
            &theirs.left,
            theirs.mirror,
            theirs.header_from_second_page,
            theirs.footer_from_second_page
        ),
        (
            ours.paper,
            ours.landscape,
            &ours.right,
            &ours.left,
            ours.mirror,
            ours.header_from_second_page,
            ours.footer_from_second_page
        ),
        "style sheet"
    );
    // With contents, a document made from the sheet starts with the text,
    // the bands' settings, and their text.
    let sheet = style_sheet(&document, &options(true));
    let made = import_sam(&sheet, None, "From sheet").unwrap().document;
    let interior = |document: &Document, frame: Option<amipro_document::FrameId>| {
        document
            .frame(frame.unwrap())
            .unwrap()
            .style
            .interior
            .clone()
    };
    assert_eq!(
        interior(&made, made.document_layout().header.right),
        interior(&document, ours.header.right)
    );
    assert_eq!(
        interior(&made, made.document_layout().footer.left),
        interior(&document, ours.footer.left)
    );
    assert_eq!(made.plain_text(), document.plain_text());
    // For the VM (PL.11): Ami Pro opens the written document.
    if let Some(out) = std::env::var_os("AMIPRO_TWIN_OUT") {
        let out = std::path::PathBuf::from(out);
        std::fs::write(out.join("SETUP.SAM"), &written).unwrap();
        std::fs::write(
            out.join("SETUP.STY"),
            style_sheet(&document, &options(false)),
        )
        .unwrap();
        std::fs::write(out.join("SETUPC.STY"), &sheet).unwrap();
    }
}

/// Ami Pro stores no gutter: it works it out from the columns
/// (`1238:0000`), 240 for one column and the first gap otherwise, and a
/// column count spreads the new columns with that gap only while the gaps
/// are alike (`1238:008a`). Edits keep the model's gutter to what a file
/// gives back, so Save As lists no page layout for it.
#[test]
fn the_gutter_is_what_ami_pro_works_out_from_the_columns() {
    use amipro_editing::LayoutSettings;
    use amipro_editing::layout_setup::Pages;
    let base = Document::from_plain_text("Main text\nmore text");
    let layout = base.document_layout().id;
    let modified = |change: &dyn Fn(&mut LayoutSettings)| {
        let mut settings = LayoutSettings::of(&base, layout).unwrap();
        change(&mut settings);
        let document = run(&base, Command::ModifyLayout { layout, settings });
        assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
        document.document_layout().right.layout.clone()
    };
    // Two columns half an inch apart, then one: 240.
    let one = modified(&|settings| {
        settings.set_columns(Pages::All, 2).unwrap();
        settings.set_gutter(Pages::All, 720).unwrap();
        settings.set_columns(Pages::All, 1).unwrap();
    });
    assert_eq!((one.columns, one.gutter), (1, 240));
    // RU9's columns on the ruler: the gutter is their gap.
    let ru9 = |settings: &mut LayoutSettings| {
        settings.set_ruler_columns(None, Pages::All, 2).unwrap();
        settings
            .set_page_edges(Pages::All, &[(1_440, 5_490), (6_240, 10_800)])
            .unwrap();
    };
    let dragged = modified(&ru9);
    assert_eq!(dragged.gutter, 750);
    // Then three columns in the panel: the one gap is alike with itself.
    let three = modified(&|settings| {
        ru9(settings);
        settings.set_columns(Pages::All, 3).unwrap();
    });
    assert_eq!((three.column_edges, three.gutter), (None, 750));
    // Gaps that differ give 240.
    let differing = modified(&|settings| {
        settings.set_ruler_columns(None, Pages::All, 3).unwrap();
        settings
            .set_page_edges(
                Pages::All,
                &[(1_440, 3_000), (3_240, 6_000), (6_750, 10_800)],
            )
            .unwrap();
        settings.set_columns(Pages::All, 2).unwrap();
    });
    assert_eq!((differing.column_edges, differing.gutter), (None, 240));
}

/// FC6N, FC7: a Where placed frame made after an anchored one is written
/// before it, in Ami Pro's order (its lists), and reads back as made.
#[test]
fn a_where_placed_frame_made_after_an_anchored_one_is_written_first_fc7() {
    let document = Document::from_plain_text("One.\nTwo.");
    let main = document.main_flow_id();
    let document = run(
        &document,
        Command::CreateFrame {
            placement: Placement::BelowParagraph {
                x_offset: 720,
                size: Size {
                    width: 2880,
                    height: 2160,
                },
            },
            style: FrameStyle {
                wrap: amipro_document::Wrap::NoWrapBeside,
                ..FrameStyle::default()
            },
            content: NewFrameContent::Text,
            anchor: Some(at(main, 1, 4)),
            layout: None,
        },
    );
    let anchored = document.frame_stack()[0];
    let document = run(
        &document,
        Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: Rect::new(3600, 3600, 6480, 5760),
            },
            style: FrameStyle::default(),
            content: NewFrameContent::Text,
            anchor: None,
            layout: None,
        },
    );
    let fixed = document.frame_stack()[1];
    assert_eq!(document.frame_stack(), [anchored, fixed]);
    let (found, written) = round_trip(&document);
    assert_eq!(found, Vec::<String>::new());
    let text = String::from_utf8_lossy(&written);
    let records: Vec<&str> = text
        .split("[frm]\r\n")
        .skip(1)
        .map(|record| record.lines().nth(1).unwrap().trim())
        .collect();
    // The Where placed frame's flags first, then the anchored one's (0x80000).
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].parse::<u32>().unwrap() & 0x80000, 0);
    assert_ne!(records[1].parse::<u32>().unwrap() & 0x80000, 0);
    assert!(sam_losses(&document, &OPTIONS).unwrap().is_empty());
}

#[test]
fn lists_and_inserted_bullets_made_in_the_app_write_as_ami_pro_writes_them() {
    use amipro_editing::lists::ListKind;
    // A document whose sheet has no list styles: Numbering makes Number
    // List in it (B1), stored as a `[tag]` and named on its paragraphs,
    // and an inserted bullet is written as Ami Pro's `<:B…>` (BL1).
    let document = import_sam(CACHED.as_bytes(), None, "Lists")
        .unwrap()
        .document;
    let main = document.main_flow_id();
    let numbered = run(
        &document,
        Command::SetList {
            flow: main,
            first: 1,
            last: 2,
            list: Some(ListKind::Numbering),
        },
    );
    let bulleted = run(
        &numbered,
        Command::InsertBullet {
            at: at(main, 0, 0),
            code: '\u{19}',
        },
    );
    let text = String::from_utf8(sam_document(&bulleted, &OPTIONS)).unwrap();
    assert!(text.contains("@Number List@Body one."), "{text}");
    assert!(text.contains("@Number List@Body two."), "{text}");
    assert!(text.contains("<:B<*9>>"), "{text}");
    let tag = text
        .split("[tag]")
        .find(|record| record.starts_with("\r\n\tNumber List\r\n"))
        .expect("the made style is stored in the document");
    // The next free function key after Body Text (2) and Title (3).
    assert!(tag.starts_with("\r\n\tNumber List\r\n\t4\r\n"), "{tag}");
    assert!(
        tag.contains("[spec]\r\n\t\t0\r\n\t\t0\r\n\t\t<*:>.\r\n\t\t0\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t16\r\n"),
        "{tag}"
    );
    assert!(
        tag.contains("[algn]\r\n\t\t1\r\n\t\t1\r\n\t\t360\r\n\t\t360\r\n\t\t360\r\n"),
        "{tag}"
    );
    assert_eq!(round_trip(&bulleted).0, Vec::<String>::new());
    assert_eq!(sam_losses(&bulleted, &OPTIONS).unwrap(), []);
    // Native v3 keeps all of it.
    let native = Document::decode_native(&bulleted.encode_native().unwrap()).unwrap();
    assert_eq!(native, bulleted);
}

#[test]
fn nested_lists_and_outline_levels_made_in_the_app_write_as_ami_pro_writes_them() {
    use amipro_editing::lists::ListKind;
    use amipro_editing::outline::LevelChange;
    // A flat numbered list nested by Demote (List 1, List 2), and a heading
    // demoted past the levels the document has (Outline#): stored as
    // `[tag]` records with their levels, read back equal, nothing lost.
    let document = import_sam(CACHED.as_bytes(), None, "Levels")
        .unwrap()
        .document;
    let main = document.main_flow_id();
    let numbered = run(
        &document,
        Command::SetList {
            flow: main,
            first: 1,
            last: 2,
            list: Some(ListKind::Numbering),
        },
    );
    let nested = run(
        &numbered,
        Command::ChangeLevel {
            flow: main,
            first: 2,
            last: 2,
            change: LevelChange::Demote,
        },
    );
    let leveled = run(
        &nested,
        Command::ChangeLevel {
            flow: main,
            first: 0,
            last: 0,
            change: LevelChange::Demote,
        },
    );
    let text = String::from_utf8(sam_document(&leveled, &OPTIONS)).unwrap();
    assert!(text.contains("@List 1@Body one."), "{text}");
    assert!(text.contains("@List 2@Body two."), "{text}");
    for (name, level, number) in [("List 1", 1, "<*:>."), ("List 2", 2, "<*=>.")] {
        let tag = text
            .split("[tag]")
            .find(|record| record.starts_with(&format!("\r\n\t{name}\r\n")))
            .unwrap_or_else(|| panic!("{name} stored"));
        assert!(
            tag.contains(&format!(
                "[spec]\r\n\t\t0\r\n\t\t{level}\r\n\t\t{number}\r\n\t\t0\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t18\r\n"
            )),
            "{tag}"
        );
    }
    // Title had no level: Demote gives it the level below none, 1, as
    // Outline1 (a copy of Title).
    assert!(text.contains("@Outline1@"), "{text}");
    assert_eq!(round_trip(&leveled).0, Vec::<String>::new());
    assert_eq!(sam_losses(&leveled, &OPTIONS).unwrap(), []);
    let native = Document::decode_native(&leveled.encode_native().unwrap()).unwrap();
    assert_eq!(native, leveled);
    // Cumulative numbers and After intervening style, as Style Properties
    // sets them (OL.5): value 8 is 4 | 8 | 16, kept in both formats.
    let list = leveled
        .styles()
        .values()
        .find(|style| style.name == "List 2")
        .unwrap()
        .id;
    let mut cumulative = leveled.styles()[&list].clone();
    cumulative.bullets.text = "\u{1A}.\u{1A}.".into();
    cumulative.bullets.flags = 4 | 8 | 16;
    let cumulative = run(&leveled, Command::ModifyStyle { style: cumulative });
    let text = String::from_utf8(sam_document(&cumulative, &OPTIONS)).unwrap();
    assert!(
        text.contains("[spec]\r\n\t\t0\r\n\t\t2\r\n\t\t<*:>.<*:>.\r\n\t\t0\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t28\r\n"),
        "{text}"
    );
    assert_eq!(round_trip(&cumulative).0, Vec::<String>::new());
    assert_eq!(sam_losses(&cumulative, &OPTIONS).unwrap(), []);
    let native = Document::decode_native(&cumulative.encode_native().unwrap()).unwrap();
    assert_eq!(native, cumulative);
}

#[test]
fn nested_bullets_made_in_the_app_write_as_ami_pro_writes_them() {
    use amipro_editing::lists::ListKind;
    use amipro_editing::outline::LevelChange;
    // A bulleted list nested twice by Demote (the user, 2026-10-18):
    // Bullet 2 with Ami Pro's dot, Bullet 3 with its small square, stored
    // as `[tag]` records without a level, read back equal, nothing lost.
    let document = import_sam(CACHED.as_bytes(), None, "Bullets")
        .unwrap()
        .document;
    let main = document.main_flow_id();
    let bulleted = run(
        &document,
        Command::SetList {
            flow: main,
            first: 0,
            last: 2,
            list: Some(ListKind::Bullets),
        },
    );
    let demote = |document: &Document, paragraph: usize| {
        run(
            document,
            Command::ChangeLevel {
                flow: main,
                first: paragraph,
                last: paragraph,
                change: LevelChange::Demote,
            },
        )
    };
    let nested = demote(&demote(&demote(&bulleted, 1), 2), 2);
    let text = String::from_utf8(sam_document(&nested, &OPTIONS)).unwrap();
    assert!(text.contains("@Bullet 2@Body one."), "{text}");
    assert!(text.contains("@Bullet 3@Body two."), "{text}");
    for (name, bullet) in [("Bullet 2", "<*1>"), ("Bullet 3", "<*2>")] {
        let tag = text
            .split("[tag]")
            .find(|record| record.starts_with(&format!("\r\n\t{name}\r\n")))
            .unwrap_or_else(|| panic!("{name} stored"));
        assert!(
            tag.contains(&format!("[spec]\r\n\t\t0\r\n\t\t0\r\n\t\t{bullet}\r\n")),
            "{tag}"
        );
    }
    assert_eq!(round_trip(&nested).0, Vec::<String>::new());
    assert_eq!(sam_losses(&nested, &OPTIONS).unwrap(), []);
    let native = Document::decode_native(&nested.encode_native().unwrap()).unwrap();
    assert_eq!(native, nested);
}

#[test]
fn a_frames_line_between_columns_reads_back_in_any_style_and_colour() {
    // A frame made in the app with a line between its columns in style 1
    // (not Ami Pro's frame default, 2) and a colour of its own: the
    // importer kept the line's style as a raw value though the writer
    // writes it, so the loss check listed the frame (2026-10-18).
    let document = import_sam(CACHED.as_bytes(), None, "Column line")
        .unwrap()
        .document;
    let mut style = FrameStyle::default();
    style.interior.columns = 2;
    style.interior.tabs = vec![Vec::new(), Vec::new()];
    style.interior.line_between = Some(amipro_document::ColumnLine {
        style: 1,
        colour: 0x0000_8000,
    });
    let framed = run(
        &document,
        Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: Rect::new(2880, 5040, 7200, 7200),
            },
            style,
            content: NewFrameContent::Text,
            anchor: None,
            layout: None,
        },
    );
    assert_eq!(round_trip(&framed).0, Vec::<String>::new());
    assert_eq!(sam_losses(&framed, &OPTIONS).unwrap(), []);
    let copy = import_sam(&sam_document(&framed, &OPTIONS), None, "Copy")
        .unwrap()
        .document;
    let line = copy
        .frames()
        .find(|frame| matches!(frame.placement, Placement::Fixed { .. }))
        .unwrap()
        .style
        .interior
        .line_between;
    assert_eq!(
        line,
        Some(amipro_document::ColumnLine {
            style: 1,
            colour: 0x0000_8000
        })
    );
}

/// Find & Replace (`UI_SPEC.md` §27): a Replace All that joins and splits
/// paragraphs, changes tabs, and sets attributes is ordinary text for the
/// writer; read back, it says what was replaced, and Save As lists nothing.
#[test]
fn a_replace_all_writes_and_reads_back_with_nothing_lost() {
    use amipro_editing::find::{Attributes, FindOptions, find_all, replace_all, search_streams};
    let document =
        Document::from_plain_text("status bar\tone\nStatus bar two\nSTATUS BAR three\nthe end");
    let mut replaced = document.clone();
    for (find, replace, options) in [
        (
            "status bar",
            "window pane",
            FindOptions {
                replace_attributes: Some(Attributes {
                    bold: true,
                    ..Attributes::default()
                }),
                ..FindOptions::default()
            },
        ),
        (
            "»",
            "¶",
            FindOptions {
                whole_word: false,
                ..FindOptions::default()
            },
        ),
        (
            "two¶",
            "two, ",
            FindOptions {
                whole_word: false,
                ..FindOptions::default()
            },
        ),
    ] {
        let streams = search_streams(&replaced, replaced.main_flow_id(), &options);
        let found = find_all(&replaced, find, &streams, &options, None);
        assert!(!found.is_empty(), "{find}");
        let command = replace_all(&replaced, &found, replace, &options).unwrap();
        replaced = run(&replaced, command);
    }
    let texts: Vec<&str> = replaced
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .collect();
    assert_eq!(
        texts,
        [
            "window pane",
            "one",
            "Window Pane two, WINDOW PANE three",
            "the end"
        ]
    );
    let (found, _) = round_trip(&replaced);
    assert_eq!(found, Vec::<String>::new());
    assert_eq!(sam_losses(&replaced, &OPTIONS).unwrap(), []);
}

#[test]
fn a_table_with_formulas_made_in_the_app_writes_and_reads_back_equal() {
    use amipro_editing::tables::FORMULA_REFUSED;
    use amipro_editing::tables::{CellRange, NewTable, TableEdit};
    let document = Document::with_paragraphs([("Invoice", amipro_document::StyleId(1))]);
    let end = at(document.main_flow_id(), 0, "Invoice".len());
    let mut document = run(
        &document,
        Command::InsertTable {
            target: amipro_editing::tables::TableTarget::Text(end),
            table: NewTable::fitting(3, 4, 9_360, 284),
        },
    );
    let frame = *document.frame_stack().last().unwrap();
    let flow_of = |document: &Document, row: usize, column: usize| match &document
        .frame(frame)
        .unwrap()
        .content
    {
        FrameContent::Table(table) => {
            let amipro_document::CellContent::Text(flow) = table.cells[row * 3 + column].content;
            flow
        }
        _ => unreachable!(),
    };
    for (row, value) in ["Total", "10.00", "5.50", ""].iter().enumerate().take(3) {
        let flow = flow_of(&document, row, 0);
        document = type_at(&document, at(flow, 0, 0), value);
    }
    document = run(
        &document,
        Command::QuickAdd {
            frame,
            row: 3,
            column: 0,
            along_row: false,
        },
    );
    document = run(
        &document,
        Command::SetCellFormula {
            frame,
            row: 1,
            column: 1,
            formula: Some("A2 * 2".into()),
        },
    );
    assert_eq!(
        apply(
            &document,
            &Command::SetCellFormula {
                frame,
                row: 1,
                column: 2,
                formula: Some("A2 +".into()),
            },
        )
        .unwrap_err(),
        FORMULA_REFUSED
    );
    document = run(
        &document,
        Command::EditTable {
            frame,
            edit: TableEdit::InsertRows {
                at: 1,
                count: 1,
                like: None,
            },
        },
    );
    let (found, written) = round_trip(&document);
    assert_eq!(found, Vec::<String>::new());
    let text = String::from_utf8_lossy(&written).into_owned();
    assert!(text.contains("@sum(A3..A4)"), "{text}");
    assert!(text.contains("A3 * 2"), "formula text as typed");
    assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
    // Del on the sum leaves a plain cell and no formula in the file.
    let cleared = run(
        &document,
        Command::EditTable {
            frame,
            edit: TableEdit::ClearCells(CellRange::cell(4, 0)),
        },
    );
    let (found, written) = round_trip(&cleared);
    assert_eq!(found, Vec::<String>::new());
    assert!(!String::from_utf8_lossy(&written).contains("sum("));
}
