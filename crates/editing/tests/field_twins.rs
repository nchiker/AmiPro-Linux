//! Power fields against Ami Pro (FL.7): every field in Ami Pro's own VM
//! fixtures FD3, FD6, and FD7, worked out by the app, gives the result Ami
//! Pro saved between its marks (`docs/research/FIELDS_NOTES.md`, "The
//! language"). Fields whose results depend on when they were inserted (the
//! clock, the counts so far, the unsaved size) are left out.
//!
//! Private: `AMIPRO_VM_LAB` names the VM folder (`vm-lab`, holding `OUT`):
//!
//! AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab cargo test -p amipro-editing \
//!     --test field_twins -- --ignored --nocapture
//!
//! With `AMIPRO_CORPUS` (and `AMIPRO_STYLES`), every document of a corpus
//! shows its codes and gives itself back when they hide.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use amipro_document::{FieldContext, WallClock, power_fields};
use amipro_editing::field_language::{FieldEnvironment, NoHost, Outcome, evaluate};
use amipro_editing::{Command, apply};
use amipro_formats::{import_sam_path, import_sam_path_with_style_directories};

/// Codes whose results Ami Pro gave at insertion, not at the save (NumEdits
/// counts the open, as Ami Pro's reader does).
const AT_INSERTION: [&str; 7] = [
    "NumEdits",
    "Now()",
    "NumWords",
    "NumChars",
    "FileSize",
    "TotalEditingTime",
    "EditDate",
];

#[test]
#[ignore = "needs AMIPRO_VM_LAB"]
fn power_fields_give_what_ami_pro_gave() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB").map(PathBuf::from) else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let mut compared = 0;
    for name in ["FD3", "FD6", "FD7"] {
        let path = lab.join("OUT").join(format!("{name}.SAM"));
        let document = import_sam_path(&path).unwrap().document;
        let info = document.info();
        let context = FieldContext {
            now: WallClock::from_ami_pro(info.saved.unwrap()),
            pages: 1,
            ..FieldContext::default()
        };
        // Set variables run in the document's order, as Ami Pro reads them;
        // FD6 asks whether FD3's `total` is defined, which Ami Pro still
        // had from FD3.
        let mut variables = BTreeMap::new();
        let bookmarks = BTreeSet::new();
        let mut host = NoHost::default();
        let paragraphs: Vec<_> = document.paragraphs().iter().map(AsRef::as_ref).collect();
        for field in power_fields(paragraphs.iter().copied()) {
            let mut environment = FieldEnvironment {
                document: &document,
                context: &context,
                variables: &mut variables,
                bookmarks: &bookmarks,
                host: &mut host,
            };
            let outcome = evaluate(&field.mark.code, &mut environment);
            let Some(end) = field.end else {
                continue;
            };
            if AT_INSERTION
                .iter()
                .any(|code| field.mark.code.starts_with(code))
                || field.mark.code == "Defined total" && name != "FD3"
            {
                continue;
            }
            assert_eq!(
                end.paragraph, field.start.paragraph,
                "{name}: {}",
                field.mark.code
            );
            let stored = &paragraphs[end.paragraph].text[field.start.byte..end.byte];
            let shown = match outcome {
                Outcome::Result(paragraphs) => paragraphs.join("\n"),
                Outcome::Nothing => String::new(),
                other => panic!("{name}: {}: {other:?}", field.mark.code),
            };
            assert_eq!(shown, stored, "{name}: {}", field.mark.code);
            compared += 1;
        }
    }
    eprintln!("power fields compared with Ami Pro's results: {compared}");
    assert!(compared > 40);
}

/// The twin: a document the app makes with every kind of field and Doc
/// Info, written as Ami Pro writes it, for Ami Pro to open (FL.13):
///
/// AMIPRO_TWIN_OUT=$PWD/../AmiPro-Reference/vm-lab/IN cargo test -p amipro-editing \
///     --test field_twins write_field_twin -- --ignored
#[test]
#[ignore = "needs AMIPRO_TWIN_OUT"]
fn write_field_twin() {
    use amipro_document::{
        DateField, DateSource, DocInfoField, Document, DocumentInfo, InlineObject, Statistics,
    };
    use amipro_editing::fields::{FieldInputs, insert_power_field};
    use amipro_editing::{Command, TextPosition, apply};
    let Some(out) = std::env::var_os("AMIPRO_TWIN_OUT").map(PathBuf::from) else {
        eprintln!("AMIPRO_TWIN_OUT is not set; skipping");
        return;
    };
    let clock = |hour, minute| WallClock {
        year: 2026,
        month: 10,
        day: 20,
        hour,
        minute,
        second: 0,
    };
    let labels = [
        "S: ", "H: ", "R: ", "C: ", "F: ", "1: ", "T: ", "P: ", "D: ", "E: ", "Q: ", "Q2: ", "U: ",
        "I: ",
    ];
    let mut document = Document::from_plain_text(&labels.join("\n"));
    let mut info = DocumentInfo::new(clock(10, 0).to_ami_pro());
    info.description = "Twin of fields".into();
    info.keywords = "fields, twin".into();
    info.fields[0] = "Alpha".into();
    info.field_names[0] = "Client".into();
    *document.info_mut() = info;
    let main = document.main_flow_id();
    let end = |document: &Document, paragraph: usize| TextPosition {
        flow: main,
        paragraph,
        byte: document.paragraphs()[paragraph].text.len(),
    };
    let objects = [
        InlineObject::Date(DateField {
            source: DateSource::Today,
            format: b'b',
        }),
        InlineObject::Date(DateField {
            source: DateSource::Time,
            format: 1,
        }),
        InlineObject::Date(DateField {
            source: DateSource::LastSaved,
            format: b'd',
        }),
        InlineObject::Date(DateField {
            source: DateSource::Created,
            format: b'c',
        }),
        InlineObject::DocInfo(DocInfoField {
            code: 1,
            format: b'a',
        }),
        InlineObject::DocInfo(DocInfoField {
            code: 8,
            format: b'a',
        }),
        InlineObject::DocInfo(DocInfoField {
            code: 0x14,
            format: b'a',
        }),
    ];
    for (paragraph, object) in objects.into_iter().enumerate() {
        let at = end(&document, paragraph);
        document = apply(&document, &Command::InsertFieldObject { at, object })
            .unwrap()
            .document;
    }
    let context = FieldContext {
        now: clock(10, 30),
        pages: 1,
        ..FieldContext::default()
    };
    let mut variables = BTreeMap::new();
    let mut host = NoHost::default();
    for (paragraph, code) in [
        (7, "NumPages"),
        (8, "CreateDate %Db"),
        (9, "3 / 2"),
        (10, "Seq A"),
        (11, "Seq A %N2"),
        (12, "UCase$(\"abc\")"),
        (13, "IF 2 > 1 \"yes\" ELSE \"no\" ENDIF"),
    ] {
        let at = end(&document, paragraph);
        let mut inputs = FieldInputs {
            context: &context,
            variables: &mut variables,
            host: &mut host,
        };
        let command = insert_power_field(&document, at, code, 0, &mut inputs).unwrap();
        document = apply(&document, &command).unwrap().document;
    }
    document.validate().unwrap();
    let (words, characters) = amipro_document::count_words_and_characters(&document);
    let options = amipro_formats::SamOptions {
        editing_minutes: 5,
        statistics: Some(Statistics {
            pages: 1,
            words,
            characters,
            ..Statistics::default()
        }),
        ..amipro_formats::SamOptions::at(clock(10, 30).to_ami_pro())
    };
    let bytes = amipro_formats::sam_document(&document, &options);
    std::fs::write(out.join("AFD1.SAM"), bytes).unwrap();
}

fn sam_files(folder: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sam_files(&path, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
        {
            found.push(path);
        }
    }
}

/// View › Show Power Fields on and off gives every document back as it was.
#[test]
#[ignore = "needs a corpus (AMIPRO_CORPUS)"]
fn showing_and_hiding_codes_gives_every_document_back() {
    let Some(corpus) = std::env::var_os("AMIPRO_CORPUS").map(PathBuf::from) else {
        eprintln!("AMIPRO_CORPUS is not set; skipping");
        return;
    };
    let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    let mut files = Vec::new();
    sam_files(&corpus, &mut files);
    files.sort();
    let (mut documents, mut with_fields, mut differ) = (0, 0, 0);
    for path in &files {
        let Ok(imported) = import_sam_path_with_style_directories(path, &styles) else {
            continue;
        };
        let document = imported.document;
        documents += 1;
        let shown = apply(&document, &Command::ShowPowerFields)
            .unwrap()
            .document;
        shown.validate().unwrap();
        if shown.paragraphs() != document.paragraphs()
            || document.flows().any(|(id, flow)| {
                shown.flow(id).map(|shown| &shown.paragraphs) != Some(&flow.paragraphs)
            })
        {
            with_fields += 1;
        }
        let hidden = apply(&shown, &Command::HidePowerFields).unwrap().document;
        hidden.validate().unwrap();
        if hidden != document {
            differ += 1;
            eprintln!("differs after hiding: document {documents}");
        }
    }
    eprintln!("{documents} documents, {with_fields} showing codes, {differ} differing");
    assert_eq!(differ, 0);
}
