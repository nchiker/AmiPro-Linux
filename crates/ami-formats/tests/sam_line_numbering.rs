//! Line Numbering (`[lnopts]`) read and written as Ami Pro 3.1 writes it
//! (`docs/research/COMMANDS_NOTES.md` §8; VM fixtures LN1–LN20).

use amipro_document::{LineNumbering, LineNumberingEvery, LineNumberingMode};
use amipro_formats::{SamOptions, import_sam, import_sam_path, sam_document, sam_losses};

const OPTIONS: SamOptions = SamOptions::at(1_793_000_000);

fn sam(lnopts: &str) -> String {
    format!(
        "[ver]\r\n\t4\r\n[fopts]\r\n\t0\r\n\t1\r\n\t0\r\n\t0\r\n\t\r\n\t\r\n{lnopts}[docopts]\r\n\t5\r\n\t2\r\n[edoc]\r\nText.\r\n\r\n>\r\n\r\n"
    )
}

fn written_lnopts(document: &amipro_document::Document) -> String {
    let written = String::from_utf8_lossy(&sam_document(document, &OPTIONS)).into_owned();
    let start = written.find("[lnopts]\r\n").unwrap();
    let end = written[start..].find("[docopts]").unwrap();
    written[start..start + end].to_owned()
}

fn read(lnopts: &str) -> amipro_document::Document {
    import_sam(sam(lnopts).as_bytes(), None, "Numbers")
        .unwrap()
        .document
}

#[test]
fn the_dialogs_values_read_and_write_back_ln2_ln6_ln18() {
    for (section, flags, style, interval) in [
        (
            "[lnopts]\r\n\t3\r\n\tBody Text\r\n\t1\r\n",
            3,
            "Body Text",
            1,
        ),
        (
            "[lnopts]\r\n\t160\r\n\tBody Text\r\n\t7\r\n",
            160,
            "Body Text",
            7,
        ),
        (
            "[lnopts]\r\n\t176\r\n\tBody Text\r\n\t99\r\n",
            176,
            "Body Text",
            99,
        ),
        ("[lnopts]\r\n\t3\r\n\tTitle\r\n\t1\r\n", 3, "Title", 1),
    ] {
        let document = read(section);
        assert_eq!(
            document.line_numbering(),
            &LineNumbering {
                flags,
                style: style.into(),
                interval
            }
        );
        assert_eq!(written_lnopts(&document), section);
        assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
    }
}

#[test]
fn a_missing_count_or_section_is_ami_pros_default_and_kept_as_read() {
    // 47 files of the user's corpus have no third line; the ISO corpus has
    // 27 with `0 / Body Text / 0`.
    let two = "[lnopts]\r\n\t2\r\n\tBody Text\r\n";
    let document = read(two);
    assert_eq!(document.line_numbering(), &LineNumbering::default());
    assert_eq!(written_lnopts(&document), two);
    let zero = "[lnopts]\r\n\t0\r\n\tBody Text\r\n\t0\r\n";
    let document = read(zero);
    assert_eq!(document.line_numbering().interval, 0);
    assert!(!document.line_numbering().is_on());
    assert_eq!(written_lnopts(&document), zero);
    let none = read("");
    assert_eq!(none.line_numbering(), &LineNumbering::default());
    assert_eq!(
        written_lnopts(&none),
        "[lnopts]\r\n\t2\r\n\tBody Text\r\n\t1\r\n"
    );
}

#[test]
fn changed_numbering_is_written_with_all_three_values() {
    let mut document = read("[lnopts]\r\n\t2\r\n\tBody Text\r\n");
    let numbering = document.line_numbering().with(
        LineNumberingMode::TextLines,
        LineNumberingEvery::Line,
        true,
    );
    document.set_line_numbering(LineNumbering {
        style: "A <b>".into(),
        ..numbering
    });
    assert_eq!(
        written_lnopts(&document),
        "[lnopts]\r\n\t146\r\n\tA <<b>\r\n\t1\r\n"
    );
    assert_eq!(sam_losses(&document, &OPTIONS).unwrap(), []);
    let written = sam_document(&document, &OPTIONS);
    let again = import_sam(&written, None, "Again").unwrap().document;
    assert_eq!(again.line_numbering(), document.line_numbering());
}

/// Ami Pro's own files read as the dialog set them (`AMIPRO_VM_LAB`).
#[test]
#[ignore = "needs the VM folder (AMIPRO_VM_LAB)"]
fn ami_pros_fixtures_read_back_equal_ln1_ln20() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let out = std::path::PathBuf::from(lab).join("OUT");
    for (name, flags, style, interval) in [
        ("LN1", 2, "Body Text", 1),
        ("LN2", 3, "Body Text", 1),
        ("LN3", 130, "Body Text", 1),
        ("LN4", 132, "Body Text", 1),
        ("LN5", 136, "Body Text", 1),
        ("LN6", 160, "Body Text", 7),
        ("LN7", 146, "Body Text", 7),
        ("LN8", 19, "Body Text", 7),
        ("LN9", 130, "Body Text", 1),
        ("LN10", 3, "Body Text", 1),
        ("LN13", 3, "Title", 1),
        ("LN14", 130, "Title", 1),
        ("LN15", 130, "Body Text", 1),
        ("LN17", 24, "Body Text", 7),
        ("LN18", 176, "Body Text", 99),
        ("LN20", 130, "Body Text", 1),
    ] {
        let path = out.join(format!("{name}.SAM"));
        let document = import_sam_path(&path).unwrap().document;
        assert_eq!(
            document.line_numbering(),
            &LineNumbering {
                flags,
                style: style.into(),
                interval
            },
            "{name}"
        );
        assert_eq!(
            sam_losses(&document, &OPTIONS).unwrap(),
            [],
            "{name}: Save As lists nothing"
        );
    }
}

/// Writes the twin Ami Pro opens: `LNB.SAM` (the fixtures' body) numbered
/// by `SetLineNumbering`'s value (text lines, every fifth, reset each
/// page; then all lines, every other, based on Title) as `OUT`-bound
/// inputs `IN/LNT1.SAM` and `IN/LNT2.SAM` (`AMIPRO_VM_LAB`).
#[test]
#[ignore = "writes into the VM folder (AMIPRO_VM_LAB)"]
fn writes_the_twins_ami_pro_opens() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let folder = std::path::PathBuf::from(lab).join("IN");
    let base = import_sam_path(folder.join("LNB.SAM")).unwrap().document;
    for (name, numbering) in [
        (
            "LNT1",
            base.line_numbering().with(
                LineNumberingMode::TextLines,
                LineNumberingEvery::FifthLine,
                true,
            ),
        ),
        (
            "LNT2",
            LineNumbering {
                style: "Title".into(),
                ..base.line_numbering().with(
                    LineNumberingMode::AllLines,
                    LineNumberingEvery::OtherLine,
                    false,
                )
            },
        ),
    ] {
        let path = folder.join(format!("{name}.SAM"));
        assert!(
            !path.exists(),
            "{} exists; choose a new name",
            path.display()
        );
        let mut document = base.clone();
        document.set_line_numbering(numbering);
        std::fs::write(&path, sam_document(&document, &OPTIONS)).unwrap();
        eprintln!("wrote {}", path.display());
    }
}
