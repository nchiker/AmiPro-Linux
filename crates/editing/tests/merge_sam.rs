//! The merge run's stored forms (`docs/research/MERGE_NOTES.md`, section 2):
//! `[recfile]` read, changed by `SetMerge`, and written as Ami Pro writes
//! it; conditions kept in the native format and listed by Save As (MG6);
//! Ami Pro data documents read through the SAM importer.

use amipro_document::Document;
use amipro_document::merge::{Condition, Join, MergeSettings, Operator, RecordFile, record_flags};
use amipro_editing::apply;
use amipro_editing::merge::data::{DataTable, Delimiters, data_paragraphs, read_ami_data};
use amipro_editing::merge::set_merge_command;
use amipro_formats::{LossKind, SamOptions, import_sam, sam_document, sam_losses};

const OPTIONS: SamOptions = SamOptions::at(1_793_000_000);

fn sam(sections: &str, edoc: &str) -> Vec<u8> {
    format!("[ver]\r\n\t4\r\n{sections}[edoc]\r\n{edoc}>\r\n\r\n").into_bytes()
}

fn read(bytes: &[u8]) -> Document {
    import_sam(bytes, None, "Merge").unwrap().document
}

fn section(written: &str, name: &str) -> Option<String> {
    let header = format!("[{name}]\r\n");
    let start = written.find(&header)? + header.len();
    let end = written[start..]
        .find("\r\n[")
        .map_or(written.len(), |end| start + end);
    Some(written[start..end].to_string())
}

fn written(document: &Document) -> String {
    String::from_utf8_lossy(&sam_document(document, &OPTIONS)).into_owned()
}

const RECFILE: &str =
    "[recfile]\r\n\tl:\\in\\mgd1.sam\r\n\t\r\n\t1\r\n\t1\r\n\t3\r\n\t12\r\n\t1440\r\n\t1440\r\n";

#[test]
fn a_record_file_is_typed_and_written_back_as_read() {
    let document = read(&sam(
        RECFILE,
        "Dear <:X12,-32768;MergeField LAST><<LAST<;><:X~12,-32768;MergeField LAST>:\r\n\r\n",
    ));
    let file = document.merge().record_file.clone().unwrap();
    assert_eq!(file, RecordFile::new("l:\\in\\mgd1.sam"));
    let out = written(&document);
    assert_eq!(
        section(&out, "recfile").unwrap(),
        "\tl:\\in\\mgd1.sam\r\n\t\r\n\t1\r\n\t1\r\n\t3\r\n\t12\r\n\t1440\r\n\t1440"
    );
    // Before `[toc]`, after `[revisions]`, as Ami Pro's writer orders them.
    assert!(out.find("[revisions]").unwrap() < out.find("[recfile]").unwrap());
    assert!(out.find("[recfile]").unwrap() < out.find("[prn]").unwrap());
    assert!(sam_losses(&document, &OPTIONS).unwrap().is_empty());
}

#[test]
fn set_merge_is_one_step_and_labels_write_flag_0x10() {
    let document = read(&sam("", "Hello\r\n\r\n"));
    assert!(section(&written(&document), "recfile").is_none());
    let mut file = RecordFile::new("c:\\data\\list.sam");
    file.flags |= record_flags::LABELS;
    file.across = 3;
    file.down = 10;
    file.indent_down = 720;
    file.indent_right = 270;
    let settings = MergeSettings {
        record_file: Some(file),
        ..Default::default()
    };
    let changed = apply(&document, &set_merge_command(settings.clone()))
        .unwrap()
        .document;
    assert_eq!(changed.merge(), &settings);
    assert_eq!(
        section(&written(&changed), "recfile").unwrap(),
        "\tc:\\data\\list.sam\r\n\t\r\n\t17\r\n\t1\r\n\t3\r\n\t10\r\n\t720\r\n\t270"
    );
    // The same settings again are no change.
    assert!(apply(&changed, &set_merge_command(settings)).is_err());
    let back = read(&sam_document(&changed, &OPTIONS));
    assert_eq!(back.merge().record_file, changed.merge().record_file);
}

#[test]
fn conditions_stay_in_the_native_format_and_save_as_lists_them() {
    let document = read(&sam(RECFILE, "Hello\r\n\r\n"));
    let mut settings = document.merge().clone();
    settings.conditions = vec![Condition {
        field: "CITY".into(),
        operator: Operator::Equal,
        value: "Boston".into(),
        join: Join::And,
    }];
    settings.with_conditions = true;
    let changed = apply(&document, &set_merge_command(settings))
        .unwrap()
        .document;
    let losses = sam_losses(&changed, &OPTIONS).unwrap();
    assert_eq!(
        losses
            .iter()
            .map(|loss| (loss.kind, loss.count))
            .collect::<Vec<_>>(),
        [(LossKind::MergeConditions, 1)]
    );
    assert_eq!(
        losses[0].message(),
        "1 merge condition, which Ami Pro does not store"
    );
    let native = Document::decode_native(&changed.encode_native().unwrap()).unwrap();
    assert_eq!(native.merge(), changed.merge());
}

#[test]
fn ami_pro_data_documents_read_through_the_importer_and_write_back() {
    // As the shipped MERCDATA.SAM: `<:s>` and caches before each record.
    let data = read(&sam(
        "[recfile]\r\n\t\r\n\t\r\n\t1\r\n\t1\r\n\t3\r\n\t8\r\n\t0\r\n\t0\r\n",
        "<:s><:#284,9360>~|\r\n\r\n<:s><:#284,9360>Title~First name~Last name~City, State Zip|\r\n\r\n<:s><:#284,9360>Mr.~Terry~Allen~Reston, VA  77013|\r\n\r\n<:s><:#284,9360>Ms.~Pam~Beck~Seattle, WA  98126|\r\n\r\n",
    ));
    let (table, delimiters) = read_ami_data(&data).unwrap();
    assert_eq!(delimiters, Delimiters::default());
    assert_eq!(table.records.len(), 2);
    assert_eq!(table.value(1, "LAST_NAME"), Some("Beck"));
    // An empty data file still has its `[recfile]` written as read.
    assert_eq!(
        section(&written(&data), "recfile").unwrap(),
        "\t\r\n\t\r\n\t1\r\n\t1\r\n\t3\r\n\t8\r\n\t0\r\n\t0"
    );
    // The app's own data file, through the writer and the importer.
    let new = DataTable {
        names: vec!["NAME".into(), "CITY".into()],
        records: vec![vec!["Ann".into(), "Boston".into()]],
    };
    let document =
        Document::from_plain_text(&data_paragraphs(&new, Delimiters::default()).join("\n"));
    let back = read(&sam_document(&document, &OPTIONS));
    assert_eq!(read_ami_data(&back).unwrap().0, new);
}
