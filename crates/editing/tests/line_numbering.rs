//! Page › Line Numbering as one command with Ami Pro's dialog limits
//! (`docs/research/COMMANDS_NOTES.md` §8; VM LN17, LN18).

use amipro_document::{Document, LineNumbering, LineNumberingEvery, LineNumberingMode};
use amipro_editing::{Command, Editor, apply};
use amipro_formats::{SamOptions, import_sam, sam_document};

fn document() -> Document {
    let sam = "[ver]\r\n\t4\r\n[edoc]\r\nOne.\r\n\r\nTwo.\r\n\r\n>\r\n\r\n";
    import_sam(sam.as_bytes(), None, "Numbers")
        .unwrap()
        .document
}

#[test]
fn line_numbering_is_one_undo_step_and_written_to_lnopts() {
    let start = document();
    let numbering = start.line_numbering().with(
        LineNumberingMode::TextLines,
        LineNumberingEvery::FifthLine,
        true,
    );
    let mut history = Editor::new(start.clone());
    history
        .execute(&Command::SetLineNumbering(numbering.clone()))
        .unwrap();
    assert_eq!(
        history.document().line_numbering().flags,
        0x80 | 0x08 | 0x10
    );
    let written = String::from_utf8(sam_document(
        history.document(),
        &SamOptions::at(1_793_000_000),
    ))
    .unwrap();
    assert!(written.contains("[lnopts]\r\n\t152\r\n\tBody Text\r\n\t1\r\n[docopts]"));
    history.undo().unwrap();
    assert_eq!(history.document().line_numbering(), start.line_numbering());
    history.redo().unwrap();
    assert_eq!(history.document().line_numbering(), &numbering);
}

#[test]
fn the_dialogs_limits_are_kept_ln18() {
    let start = document();
    let with = |interval: u16, style: &str| LineNumbering {
        flags: LineNumbering::TEXT_LINES | LineNumbering::EVERY_N,
        style: style.into(),
        interval,
    };
    for refused in [
        with(0, "Body Text"),
        with(100, "Body Text"),
        with(3, "No Such"),
    ] {
        assert!(
            apply(&start, &Command::SetLineNumbering(refused.clone())).is_err(),
            "{refused:?}"
        );
    }
    let taken = apply(&start, &Command::SetLineNumbering(with(99, "Body Text"))).unwrap();
    assert_eq!(taken.document.line_numbering().interval, 99);
    // Off keeps the other choices (LN17).
    let off = taken.document.line_numbering().with(
        LineNumberingMode::None,
        LineNumberingEvery::Lines,
        false,
    );
    let off = apply(&taken.document, &Command::SetLineNumbering(off)).unwrap();
    assert_eq!(off.document.line_numbering().flags, LineNumbering::EVERY_N);
    assert_eq!(off.document.line_numbering().interval, 99);
}

/// Run A's twin (CA.15): a document made with the run's commands —
/// Paragraph Properties' indents and 1½ spacing, Protected Text, No
/// Hyphenation, a special character, Number text lines every fifth line —
/// written as `IN/CAT1.SAM` for Ami Pro to open (`AMIPRO_VM_LAB`).
#[test]
#[ignore = "writes into the VM folder (AMIPRO_VM_LAB)"]
fn writes_run_as_twin_for_ami_pro() {
    use amipro_editing::{CharacterAttribute, IndentChange, TextPosition};
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let path = std::path::PathBuf::from(lab).join("IN").join("CAT1.SAM");
    assert!(!path.exists(), "{} exists", path.display());
    let lines: Vec<String> = (1..=60)
        .map(|index| format!("Line {index:02} of the twin \u{a9} 2026, protected and plain text."))
        .collect();
    let sam = format!(
        "[ver]\r\n\t4\r\n[edoc]\r\n{}>\r\n\r\n",
        lines
            .iter()
            .map(|line| format!("{line}\r\n\r\n"))
            .collect::<String>()
    );
    // Windows-1252, as Ami Pro's files are: © is one byte.
    let bytes: Vec<u8> = sam.chars().map(|character| character as u8).collect();
    let mut document = import_sam(&bytes, None, "Twin").unwrap().document;
    let flow = document.main_flow_id();
    let at = |paragraph, byte| TextPosition {
        flow,
        paragraph,
        byte,
    };
    let commands = [
        Command::SetParagraphIndents {
            flow,
            first: 0,
            last: 2,
            indents: IndentChange {
                all: Some(720),
                first_line: Some(360),
                rest: None,
                right: Some(360),
            },
        },
        Command::SetParagraphLineSpacing {
            flow,
            first: 3,
            last: 5,
            spacing: Some(amipro_document::LineSpacing::OneAndAHalf(0)),
        },
        Command::SetCharacterAttribute {
            start: at(6, 0),
            end: at(6, 7),
            attribute: CharacterAttribute::Protected,
            on: true,
        },
        Command::SetCharacterAttribute {
            start: at(7, 0),
            end: at(7, 7),
            attribute: CharacterAttribute::NoHyphenation,
            on: true,
        },
        Command::SetLineNumbering(document.line_numbering().with(
            LineNumberingMode::TextLines,
            LineNumberingEvery::FifthLine,
            false,
        )),
    ];
    for command in &commands {
        document = apply(&document, command)
            .unwrap_or_else(|error| panic!("{command:?}: {error}"))
            .document;
    }
    std::fs::write(
        &path,
        sam_document(&document, &SamOptions::at(1_793_900_000)),
    )
    .unwrap();
    eprintln!("wrote {}", path.display());
}
