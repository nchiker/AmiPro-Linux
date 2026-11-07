//! The TOC and index settings as Ami Pro's files keep them
//! (`docs/research/TOC_INDEX_NOTES.md`): `[toc]` and `[master]` read into
//! the model, written back as read, and kept by the native format (VM
//! fixtures TI1, TI1T2).

use amipro_document::Document;
use amipro_document::contents::{Leader, TocLevel, TocSettings};
use amipro_formats::{SamOptions, import_sam, sam_document, sam_losses};

const OPTIONS: SamOptions = SamOptions::at(1_793_000_000);

fn sam(sections: &str, edoc: &str) -> Vec<u8> {
    format!("[ver]\r\n\t4\r\n{sections}[edoc]\r\n{edoc}>\r\n\r\n").into_bytes()
}

fn read(bytes: &[u8]) -> Document {
    import_sam(bytes, None, "Contents").unwrap().document
}

fn written(document: &Document) -> String {
    String::from_utf8_lossy(&sam_document(document, &OPTIONS)).into_owned()
}

fn section<'a>(written: &'a str, name: &str) -> Option<&'a str> {
    let header = format!("[{name}]\r\n");
    let start = written.find(&header)? + header.len();
    let end = written[start..]
        .find("\r\n[")
        .map_or(written.len(), |end| start + end + 2);
    Some(&written[start..end])
}

const TI1_SECTIONS: &str = "[sty]\r\n\t_DEFAULT.STY\r\n[toc]\r\n\tTitle\r\n\tSubhead\r\n\t\r\n\t.\r\n\t.\r\n\t\r\n\t15\r\n[master]\r\n\t\r\n\tL:\\OUT\\TI1IDX.SAM\r\n\t1\r\n\t0\r\n\t0\r\n";

#[test]
fn toc_and_master_are_read_into_the_model_and_written_back_as_read_ti1() {
    let document = read(&sam(TI1_SECTIONS, "@Title@Getting Started\r\n\r\n"));
    let toc = document.contents().toc.as_ref().unwrap();
    let levels = toc.levels();
    assert_eq!(
        levels
            .iter()
            .map(|level| level.style.as_str())
            .collect::<Vec<_>>(),
        ["Title", "Subhead"]
    );
    assert!(
        levels
            .iter()
            .all(|level| level.page_number && level.right_align)
    );
    let master = document.contents().master.as_ref().unwrap();
    assert!(master.separators());
    assert_eq!(master.toc_file, "");
    assert_eq!(master.index_file, "L:\\OUT\\TI1IDX.SAM");

    let out = written(&document);
    assert_eq!(
        section(&out, "toc").unwrap(),
        "\tTitle\r\n\tSubhead\r\n\t\r\n\t.\r\n\t.\r\n\t\r\n\t15\r\n"
    );
    assert_eq!(
        section(&out, "master").unwrap(),
        "\t\r\n\tL:\\OUT\\TI1IDX.SAM\r\n\t1\r\n\t0\r\n\t0\r\n"
    );
    // Ami Pro's order: [toc] and [master] after [revisions]; nothing lost.
    assert!(out.find("[toc]").unwrap() < out.find("[master]").unwrap());
    assert!(sam_losses(&document, &OPTIONS).unwrap().is_empty());
    let again = read(out.as_bytes());
    assert_eq!(again.contents(), document.contents());
}

#[test]
fn a_documents_changed_toc_levels_are_written_and_kept_natively() {
    let mut document = read(&sam(TI1_SECTIONS, "@Title@Getting Started\r\n\r\n"));
    let mut settings = document.contents().clone();
    settings.toc = Some(TocSettings::from_levels(&[
        TocLevel {
            style: "Title".into(),
            page_number: false,
            right_align: false,
            leader: Leader::None,
        },
        TocLevel {
            style: "Subhead".into(),
            page_number: true,
            right_align: false,
            leader: Leader::Comma,
        },
    ]));
    document.set_contents(settings);
    let out = written(&document);
    // Level 1 without a page; level 2 with its page after a comma.
    assert_eq!(
        section(&out, "toc").unwrap(),
        "\tTitle\r\n\tSubhead\r\n\t\r\n\t  \r\n\t, \r\n\t\r\n\t4\r\n"
    );
    let native = Document::decode_native(&document.encode_native().unwrap()).unwrap();
    assert_eq!(native.contents(), document.contents());
}

#[test]
fn a_document_without_them_writes_neither() {
    let document = read(&sam("", "Text.\r\n\r\n"));
    let out = written(&document);
    assert!(section(&out, "toc").is_none() && section(&out, "master").is_none());
}
