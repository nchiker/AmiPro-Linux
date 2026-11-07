//! Page layout setup's twins (`docs/LAYOUT_SETUP_RUNBOOK.md`, PL.4–PL.7):
//! VM fixtures G01–G23 each change one setting of G00 in Ami Pro's Modify
//! Page Layout. Here G00 gets the same change through `layout_setup`, as
//! Page Layout Properties makes it, is written as an Ami Pro document, and
//! its page layout records are compared with Ami Pro's.
//!
//! Private: `AMIPRO_VM_LAB` names the VM fixtures' folder; with
//! `AMIPRO_TWIN_OUT`, the written twins go there as `AGnn.SAM` for Ami Pro
//! to open.
//!
//! ```sh
//! AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test -p amipro-editing \
//!   --test layout_twins -- --ignored --nocapture
//! ```

use amipro_document::{Document, LayoutId, Sides};
use amipro_editing::layout_setup::{Band, MarginSide, Pages};
use amipro_editing::{Command, LayoutSettings, apply};
use amipro_formats::{SamOptions, import_sam_path, sam_document};

/// The `[lay]` … `[elay]` records of a SAM file, one value a line, with
/// the values that only say when it was saved left out.
fn layout_records(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        let line = line.trim();
        if line == "[lay]" {
            inside = true;
        }
        if inside && !line.is_empty() {
            out.push(line.to_owned());
        }
        if line == "[elay]" {
            inside = false;
        }
    }
    out
}

type Change = fn(&mut LayoutSettings);

/// Each fixture's change as the panel makes it (`SAM_FORMAT_NOTES.md`,
/// fixture table).
fn changes() -> Vec<(&'static str, Change)> {
    vec![
        ("G01", |s| {
            s.set_margin(Pages::All, MarginSide::Top, 2_160).unwrap()
        }),
        ("G02", |s| {
            s.set_margin(Pages::All, MarginSide::Bottom, 2_160).unwrap()
        }),
        ("G03", |s| {
            s.set_margin(Pages::All, MarginSide::Left, 2_160).unwrap()
        }),
        ("G04", |s| {
            s.set_margin(Pages::All, MarginSide::Right, 2_160).unwrap()
        }),
        ("G05", |s| s.set_columns(Pages::All, 2).unwrap()),
        ("G06", |s| {
            s.set_columns(Pages::All, 2).unwrap();
            s.set_balance(Pages::All, true).unwrap();
        }),
        ("G07", |s| {
            s.set_page_line_sides(Pages::All, Sides::ALL).unwrap()
        }),
        ("G08", |s| {
            s.set_page_line_sides(Pages::All, Sides::ALL).unwrap();
            s.set_page_line_position(Pages::All, amipro_document::LinePosition::Outside)
                .unwrap();
        }),
        ("G09", |s| {
            s.set_columns(Pages::All, 2).unwrap();
            s.set_column_line(Pages::All, true).unwrap();
        }),
        ("G10", |s| {
            s.set_page_line_sides(Pages::All, Sides::ALL).unwrap();
            s.set_line_colour(Pages::All, 255).unwrap();
        }),
        ("G11", |s| s.set_paper(2).unwrap()),
        ("G12", |s| s.set_landscape(true).unwrap()),
        ("G13", |s| s.set_paper(4).unwrap()),
        ("G14", |s| s.set_custom_size(8_640, 12_960).unwrap()),
        ("G15", |s| {
            s.set_margin(Pages::Left, MarginSide::Left, 2_160).unwrap()
        }),
        ("G16", |s| s.set_mirror(Pages::Right, true).unwrap()),
        ("G17", |s| {
            s.set_mirror(Pages::Right, true).unwrap();
            s.set_margin(Pages::Right, MarginSide::Left, 2_160).unwrap();
        }),
        ("G19", |s| s.set_from_second_page(Band::Header, true)),
        ("G23", |s| s.set_from_second_page(Band::Footer, true)),
    ]
}

fn twin(base: &Document, change: Change) -> Document {
    let layout: LayoutId = base.document_layout().id;
    let mut settings = LayoutSettings::of(base, layout).unwrap();
    change(&mut settings);
    apply(base, &Command::ModifyLayout { layout, settings })
        .unwrap()
        .document
}

#[test]
#[ignore = "needs the private VM fixtures (AMIPRO_VM_LAB)"]
fn page_layout_changes_are_written_as_ami_pro_writes_them() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let out = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from);
    let base = import_sam_path(lab.join("G00.SAM")).unwrap().document;
    let options = SamOptions::at(0);
    let mut different = Vec::new();
    for (name, change) in changes() {
        let document = twin(&base, change);
        let written = sam_document(&document, &options);
        if let Some(out) = &out {
            std::fs::write(out.join(format!("A{name}.SAM")), &written).unwrap();
        }
        let ours = layout_records(&written);
        let mut theirs = layout_records(&std::fs::read(lab.join(format!("{name}.SAM"))).unwrap());
        if name == "G13" {
            // Ami Pro's dialog sets A4's columns from its table's width,
            // 11,905 twips, but stores the width its box reads back, 11,908
            // (`PAGE_LAYOUT_NOTES.md`): the column ends 3 twips inside the
            // right margin, the header's and footer's too. The app keeps
            // columns even with the margins.
            for value in theirs.iter_mut().filter(|value| *value == "10465") {
                *value = "10468".into();
            }
        }
        if ours == theirs {
            eprintln!("{name}: the same");
        } else {
            let at = ours
                .iter()
                .zip(&theirs)
                .position(|(a, b)| a != b)
                .unwrap_or(ours.len().min(theirs.len()));
            eprintln!(
                "{name}: differs at value {at}: ours {:?}, Ami Pro's {:?}",
                &ours[at.saturating_sub(2)..(at + 4).min(ours.len())],
                &theirs[at.saturating_sub(2)..(at + 4).min(theirs.len())]
            );
            different.push(name);
        }
    }
    // For Ami Pro to show: each line position with two-inch margins and a
    // 4 pt line (style 4), and two columns with the line between, so that
    // where Ami Pro draws them can be measured on its screen (PL.6).
    if let Some(out) = &out {
        use amipro_document::LinePosition;
        for (name, position) in [
            ("AP1", LinePosition::Inside),
            ("AP2", LinePosition::CloseToInside),
            ("AP3", LinePosition::Middle),
            ("AP4", LinePosition::CloseToOutside),
            ("AP5", LinePosition::Outside),
        ] {
            let layout = base.document_layout().id;
            let mut settings = LayoutSettings::of(&base, layout).unwrap();
            for side in [MarginSide::Left, MarginSide::Top, MarginSide::Right] {
                settings.set_margin(Pages::All, side, 2_880).unwrap();
            }
            settings
                .set_page_line_sides(Pages::All, Sides::ALL)
                .unwrap();
            settings.set_page_line_style(Pages::All, 4).unwrap();
            settings
                .set_page_line_position(Pages::All, position)
                .unwrap();
            settings.set_columns(Pages::All, 2).unwrap();
            settings.set_gutter(Pages::All, 1_440).unwrap();
            settings.set_column_line(Pages::All, true).unwrap();
            settings.set_column_line_style(Pages::All, 4).unwrap();
            let document = apply(&base, &Command::ModifyLayout { layout, settings })
                .unwrap()
                .document;
            std::fs::write(
                out.join(format!("{name}.SAM")),
                sam_document(&document, &options),
            )
            .unwrap();
        }
    }
    assert!(different.is_empty(), "differ: {different:?}");
}

/// The `[edoc]` body and the layout records, with Ami Pro's paragraph
/// layout caches (`<:#…>`) left out: they say how Ami Pro last laid a
/// paragraph out, not what it holds.
fn body_and_layouts(bytes: &[u8]) -> (String, Vec<String>) {
    let text = String::from_utf8_lossy(bytes);
    let body = text
        .split("[edoc]")
        .nth(1)
        .and_then(|rest| rest.split("\n>").next())
        .unwrap_or_default();
    let mut stripped = String::new();
    let mut rest = body;
    while let Some(start) = rest.find("<:#") {
        stripped.push_str(&rest[..start]);
        rest = rest[start..].split_once('>').map_or("", |(_, after)| after);
    }
    stripped.push_str(rest);
    let body = stripped
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    let layouts = layout_records(bytes)
        .into_iter()
        .map(|record| without_caches(&record))
        .collect();
    (body.trim().to_owned(), layouts)
}

/// `text` without Ami Pro's paragraph layout caches (`<:#…>`).
fn without_caches(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<:#") {
        out.push_str(&rest[..start]);
        rest = rest[start..].split_once('>').map_or("", |(_, after)| after);
    }
    out.push_str(rest);
    out
}

#[test]
#[ignore = "needs the private VM fixtures (AMIPRO_VM_LAB)"]
fn breaks_and_inserted_layouts_are_written_as_ami_pro_writes_them() {
    use amipro_document::BreakKind;
    use amipro_editing::TextPosition;
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let options = SamOptions::at(0);
    let read = |name: &str| {
        import_sam_path(lab.join(format!("{name}.SAM")))
            .unwrap()
            .document
    };
    let run = |document: &Document, command: Command| apply(document, &command).unwrap().document;
    let at = |document: &Document, paragraph: usize, byte: usize| TextPosition {
        flow: document.main_flow_id(),
        paragraph,
        byte,
    };
    let page_break = |document: &Document, paragraph, byte, layout| {
        run(
            document,
            Command::InsertBreak {
                at: at(document, paragraph, byte),
                kind: BreakKind::Page,
                centre: false,
                layout,
            },
        )
    };
    // At the end of the text the app adds an empty paragraph, as Ami Pro.
    let new_paragraph = |document: &Document, paragraph: usize| {
        let end = document.paragraphs()[paragraph].text.len();
        let position = at(document, paragraph, end);
        run(
            document,
            Command::ReplaceText {
                start: position,
                end: position,
                text: "\n".into(),
            },
        )
    };
    let g00 = read("G00");
    let mut cases: Vec<(&str, Document)> = Vec::new();
    // PB1: a page break after "Main text"; PB3: at the end.
    cases.push(("PB1", page_break(&g00, 0, 9, None)));
    let end = g00.paragraphs()[1].text.len();
    cases.push(("PB3", new_paragraph(&page_break(&g00, 1, end, None), 1)));
    // CB1: a column break, at its paragraph's start, in G05.
    let g05 = read("G05");
    cases.push((
        "CB1",
        run(
            &g05,
            Command::InsertBreak {
                at: at(&g05, 0, 0),
                kind: BreakKind::Column,
                centre: false,
                layout: None,
            },
        ),
    ));
    // IL1: Insert Page Layout after "Main text", then two columns in it.
    let inserted = run(
        &g00,
        Command::InsertLayout {
            at: at(&g00, 0, 9),
            from: g00.document_layout().id,
        },
    );
    let layout = inserted.layouts()[1].id;
    let mut settings = LayoutSettings::of(&inserted, layout).unwrap();
    settings.set_columns(Pages::All, 2).unwrap();
    cases.push((
        "IL1",
        run(&inserted, Command::ModifyLayout { layout, settings }),
    ));
    // IR1: Remove on IL1's second page; IV1: Revert at its end.
    let il1 = read("IL1");
    let removed = run(
        &il1,
        Command::DeleteCharacter {
            at: at(&il1, 0, 9),
            forward: true,
        },
    );
    let layout = removed.layouts()[1].id;
    cases.push(("IR1", run(&removed, Command::RemoveLayout { layout })));
    let end = il1.paragraphs()[1].text.len();
    let standard = il1.document_layout().id;
    cases.push((
        "IV1",
        new_paragraph(&page_break(&il1, 1, end, Some(standard)), 1),
    ));
    // PN1: Page Numbering in G00's header: style i from page 2 with 1,
    // "Page " before it (`<:Pi2,1,Page >`).
    let header = g00.document_layout().header.right.unwrap();
    let amipro_document::FrameContent::Text(header_flow) = g00.frame(header).unwrap().content
    else {
        panic!("a text header");
    };
    cases.push((
        "PN1",
        run(
            &g00,
            Command::InsertPageNumber {
                at: TextPosition {
                    flow: header_flow,
                    paragraph: 0,
                    byte: 0,
                },
                field: amipro_document::PageNumberField {
                    style: 'i',
                    start_page: 2,
                    start_number: 1,
                    leading_text: "Page ".into(),
                },
            },
        ),
    ));
    // FH1: a floating header for all pages at the end of G00; FH2: a
    // floating footer for right pages after "Main text"; FH3: G22's
    // floating header removed at its mark.
    use amipro_document::{HeaderFooterKind, PageSides};
    let floating = |document: &Document, paragraph, byte, kind, pages| {
        run(
            document,
            Command::InsertFloatingHeaderFooter {
                at: at(document, paragraph, byte),
                kind,
                pages,
            },
        )
    };
    let end = g00.paragraphs()[1].text.len();
    cases.push((
        "FH1",
        floating(&g00, 1, end, HeaderFooterKind::Header, PageSides::All),
    ));
    cases.push((
        "FH2",
        floating(&g00, 0, 9, HeaderFooterKind::Footer, PageSides::Right),
    ));
    let g22 = read("G22");
    let mark = g22.paragraphs()[1].object_offsets().next().unwrap();
    cases.push((
        "FH3",
        run(
            &g22,
            Command::DeleteCharacter {
                at: at(&g22, 1, mark),
                forward: true,
            },
        ),
    ));
    let mut different = Vec::new();
    for (name, document) in cases {
        let ours = body_and_layouts(&sam_document(&document, &options));
        let theirs = body_and_layouts(&std::fs::read(lab.join(format!("{name}.SAM"))).unwrap());
        if ours == theirs {
            eprintln!("{name}: the same");
        } else {
            eprintln!(
                "{name}: body ours {:?}\n      Ami Pro's {:?}",
                ours.0, theirs.0
            );
            if ours.1 != theirs.1 {
                let at = ours
                    .1
                    .iter()
                    .zip(&theirs.1)
                    .position(|(a, b)| a != b)
                    .unwrap_or(ours.1.len().min(theirs.1.len()));
                eprintln!(
                    "      layouts differ at {at}: {:?} / {:?}",
                    ours.1.get(at),
                    theirs.1.get(at)
                );
            }
            different.push(name);
        }
    }
    assert!(different.is_empty(), "differ: {different:?}");
}
