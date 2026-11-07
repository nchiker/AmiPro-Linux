//! Outline › Move Up and Move Down move a heading with its section as one
//! undo step (`UI_SPEC.md` §40, OM3; `sections`).

use amipro_document::{Document, StyleId};
use amipro_editing::{Command, Editor};
use amipro_formats::{SamOptions, import_sam, sam_document};

/// Title, Subhead A, two bodies, Subhead B, body, Title 2, Subhead C, body.
fn outline() -> Document {
    let mut document = Document::from_plain_text(
        "First title\nSubhead A\nBody A1\nBody A2\nSubhead B\nBody B\nSecond title\nSubhead C\nBody C",
    );
    let body = document.body_text_style();
    for (id, name, level) in [(StyleId(801), "Title", 1), (StyleId(802), "Subhead", 2)] {
        let mut style = document.styles()[&body].clone();
        style.id = id;
        style.name = name.into();
        style.bullets.level = level;
        document.styles_mut().insert(id, style);
    }
    for (index, style) in [(0, 801), (1, 802), (4, 802), (6, 801), (7, 802)] {
        document.paragraph_mut(index).style = StyleId(style);
    }
    document.validate().unwrap();
    document
}

fn texts(document: &Document) -> Vec<String> {
    document
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect()
}

#[test]
fn a_section_moves_as_one_step_and_saves_as_it_shows() {
    let start = outline();
    let flow = start.main_flow_id();
    let mut editor = Editor::new(start.clone());
    editor
        .execute(&Command::MoveSection {
            flow,
            paragraph: 4,
            down: false,
        })
        .unwrap();
    assert_eq!(
        texts(editor.document())[..6],
        [
            "First title",
            "Subhead B",
            "Body B",
            "Subhead A",
            "Body A1",
            "Body A2"
        ]
    );
    // The second title goes above the first with its section.
    editor
        .execute(&Command::MoveSection {
            flow,
            paragraph: 6,
            down: false,
        })
        .unwrap();
    assert_eq!(
        texts(editor.document())[..4],
        ["Second title", "Subhead C", "Body C", "First title"]
    );
    let written = sam_document(editor.document(), &SamOptions::at(1_793_000_000));
    let again = import_sam(&written, None, "Moved").unwrap().document;
    assert_eq!(texts(&again), texts(editor.document()));
    editor.undo().unwrap();
    editor.undo().unwrap();
    assert_eq!(texts(editor.document()), texts(&start));
    assert!(
        amipro_editing::apply(
            &start,
            &Command::MoveSection {
                flow,
                paragraph: 0,
                down: false
            }
        )
        .is_err(),
        "at the start"
    );
}

#[test]
fn a_dropped_section_lands_before_its_target_as_one_step() {
    let start = outline();
    let flow = start.main_flow_id();
    let mut editor = Editor::new(start.clone());
    // Second title's section (6..9) before the first title.
    editor
        .execute(&Command::MoveSectionTo {
            flow,
            paragraph: 6,
            to: 0,
        })
        .unwrap();
    assert_eq!(
        texts(editor.document())[..4],
        ["Second title", "Subhead C", "Body C", "First title"]
    );
    editor.undo().unwrap();
    // Subhead A (1..4) to the end.
    editor
        .execute(&Command::MoveSectionTo {
            flow,
            paragraph: 1,
            to: 9,
        })
        .unwrap();
    assert_eq!(
        texts(editor.document())[5..],
        ["Body C", "Subhead A", "Body A1", "Body A2"]
    );
    // Inside itself or where it is: refused.
    for to in [1, 2, 4] {
        assert!(
            amipro_editing::apply(
                &start,
                &Command::MoveSectionTo {
                    flow,
                    paragraph: 1,
                    to
                }
            )
            .is_err(),
            "{to}"
        );
    }
}

/// Run B's twin (OB.7): Ami Pro's OL2 (Title one, Sub one, Sub two, Title
/// two, Sub three) reorganised by Move Up and a navigator drop, written as
/// `IN/OBT1.SAM` for Ami Pro to open (`AMIPRO_VM_LAB`).
#[test]
#[ignore = "writes into the VM folder (AMIPRO_VM_LAB)"]
fn writes_run_bs_twin_for_ami_pro() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let path = lab.join("IN").join("OBT1.SAM");
    assert!(!path.exists(), "{} exists", path.display());
    let mut document = amipro_formats::import_sam_path(lab.join("OUT").join("OL2.SAM"))
        .unwrap()
        .document;
    let flow = document.main_flow_id();
    for command in [
        Command::MoveSection {
            flow,
            paragraph: 2,
            down: false,
        },
        Command::MoveSectionTo {
            flow,
            paragraph: 3,
            to: 0,
        },
    ] {
        document = amipro_editing::apply(&document, &command).unwrap().document;
    }
    assert_eq!(
        texts(&document),
        ["Title two", "Sub three", "Title one", "Sub two", "Sub one"]
    );
    std::fs::write(
        &path,
        sam_document(&document, &SamOptions::at(1_793_900_000)),
    )
    .unwrap();
    eprintln!("wrote {}", path.display());
}
