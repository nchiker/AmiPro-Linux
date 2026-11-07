//! Modify Style's "1st line bold" in files: `[fnt]` value 4, bit 4096
//! (`docs/research/FIRST_LINE_BOLD_NOTES.md`; VM fixtures FLB0, FLB1, FLB2,
//! FLB1.STY, `AMIPRO_VM_LAB`).

use amipro_formats::{import_sam, read_style_sheet, style_record};

/// The `[tag]` record of `name`, as the bytes between its line and the next
/// unindented bracket line.
fn record_of(file: &[u8], name: &str) -> Vec<u8> {
    let text = String::from_utf8_lossy(file).into_owned();
    let marker = format!("[tag]\r\n\t{name}\r\n");
    let start = text.find(&marker).expect("the record");
    let after = start + marker.len();
    let end = text[after..]
        .lines()
        .scan(after, |at, line| {
            let here = *at;
            *at += line.len() + 2;
            Some((here, line))
        })
        .find(|(_, line)| line.starts_with('['))
        .map_or(text.len(), |(here, _)| here);
    text.as_bytes()[start..end].to_vec()
}

fn body_text(document: &amipro_document::Document) -> amipro_document::ParagraphStyle {
    document
        .styles()
        .values()
        .find(|style| style.name == "Body Text")
        .cloned()
        .expect("Body Text")
}

#[test]
fn a_style_record_with_4096_reads_as_first_line_bold_and_writes_back() {
    let sample = |flags: u32| {
        format!(
            "[ver]\r\n4\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tTimes New Roman\r\n\t\t240\r\n\t\t0\r\n\t\t{flags}\r\n\
             [edoc]\r\nText\r\n\r\n>\r\n"
        )
    };
    for (flags, on) in [(49152, false), (53248, true), (53249, true)] {
        let document = import_sam(sample(flags).as_bytes(), None, "T")
            .unwrap()
            .document;
        let style = body_text(&document);
        assert_eq!(style.first_line_bold, on, "{flags}");
        assert!(!style.attributes.protected, "{flags}: never protected text");
        let written = style_record(&style, document.styles());
        let text = String::from_utf8(written).unwrap();
        assert!(
            text.contains(&format!("\t\t{flags}\r\n")),
            "{flags}: {text}"
        );
    }
}

#[test]
fn a_protected_run_is_still_a_run_attribute_not_a_style_one() {
    let sam =
        b"[ver]\r\n4\r\n[tag]\r\n\tBody Text\r\n\t2\r\n[edoc]\r\nA <+*>safe<-*> word\r\n\r\n>\r\n";
    let document = import_sam(sam, None, "T").unwrap().document;
    assert!(!body_text(&document).first_line_bold);
    assert_eq!(document.paragraphs()[0].format_at(3).protected, Some(true));
}

#[test]
#[ignore = "needs the VM folder (AMIPRO_VM_LAB)"]
fn ami_pros_flb_files_read_and_their_body_text_record_is_written_byte_for_byte() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    for (path, on) in [
        ("OUT/FLB0.SAM", false),
        ("OUT/FLB1.SAM", true),
        ("OUT/FLB2.SAM", true),
    ] {
        let bytes = std::fs::read(lab.join(path)).unwrap();
        let document = import_sam(&bytes, None, "FLB").unwrap().document;
        let style = body_text(&document);
        assert_eq!(style.first_line_bold, on, "{path}");
        assert!(!style.attributes.protected, "{path}");
        assert_eq!(
            style_record(&style, document.styles()),
            record_of(&bytes, "Body Text"),
            "{path}: the Body Text record"
        );
    }
    let sheet = std::fs::read(lab.join("STY/FLB1.STY")).unwrap();
    let read = read_style_sheet(&sheet, "FLB1").unwrap();
    let style = read
        .styles
        .iter()
        .find(|style| style.name == "Body Text")
        .unwrap();
    assert!(style.first_line_bold);
    assert!(!style.attributes.protected);
    let styles: std::collections::BTreeMap<_, _> = read
        .styles
        .iter()
        .map(|style| (style.id, style.clone()))
        .collect();
    assert_eq!(
        style_record(style, &styles),
        record_of(&sheet, "Body Text"),
        "FLB1.STY: the Body Text record"
    );
}
