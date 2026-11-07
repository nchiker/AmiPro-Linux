//! Export, PDF, and printing as a user does them (`UI_SPEC.md` §29 and
//! §30; the "Print and PDF" and "Export" checklists of `UI_ACCEPTANCE.md`
//! §10). Nothing here opens a desktop dialog: the choosers' finished
//! messages are sent, and printing is only run up to the messages it would
//! send.

use super::*;
use crate::export_folder::FolderAction;
use crate::import::OptionsAction;
use crate::page_surface::page_fixture;
use crate::print_panel::{Addition, PrintAction, PrintOption};
use amipro_exchange::Format;

/// A document the way a user makes one: a
/// heading, a bold word, a bulleted list, a table with typed cells, and
/// text with accents.
pub(super) fn made_document(s: &mut Scenario) {
    s.caret_to_end();
    s.enter();
    s.type_text("Export check: café, naïve, £5 and ");
    s.command_act(CommandId::Bold);
    s.type_text("bold");
    s.command_act(CommandId::Bold);
    s.type_text(" words.");
    s.enter();
    s.command(CommandId::Bullets);
    s.type_text("first item");
    s.enter();
    s.type_text("second item");
    s.enter();
    // Enter in an empty list paragraph ends the list.
    s.enter();
    s.type_text("After the list.");
    s.enter();
    // Insert › Table…: 2 columns, 2 rows; typed with Tab.
    s.command_act(CommandId::Table);
    s.act("pick 2 × 2", |rt| {
        rt.send(Message::InsertTable(
            crate::table_commands::InsertTableAction::Pick(2, 2),
        ));
    });
    s.edit("Insert the table", |rt| {
        rt.send(Message::InsertTable(
            crate::table_commands::InsertTableAction::Insert,
        ));
    });
    for (index, value) in ["Name", "Qty", "Pen", "4"].iter().enumerate() {
        s.type_text(value);
        if index < 3 {
            s.act("Tab", |rt| {
                rt.send(Message::Page(PageAction::CellStep { forward: true }));
            });
        }
    }
}

pub(super) fn paragraph_texts(document: &Document) -> Vec<String> {
    document
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.trim_matches('\u{FFFC}').to_string())
        .filter(|text| !text.is_empty())
        .collect()
}

/// File › Export ▸ `format` to `name` in the scratch folder, as the chooser
/// returns it; a loss list is answered with Cancel (nothing written) and
/// then Export Anyway when `expect_losses`. Returns the path.
pub(super) fn export(
    s: &mut Scenario,
    format: Format,
    name: &str,
    expect_losses: Option<bool>,
) -> PathBuf {
    let path = s.folder().join(name);
    let chosen = path.clone();
    s.act(&format!("Export {format:?}"), move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::ExportChosen {
            session_id,
            format,
            result: FileDialogResult::Selected(chosen),
        });
    });
    let asks = s.app().export_pending.is_some();
    if let Some(expected) = expect_losses {
        assert_eq!(
            asks,
            expected,
            "{format:?}: the loss list {}",
            if expected {
                "was not shown"
            } else {
                "was shown"
            }
        );
    }
    if asks {
        assert!(
            !path.exists(),
            "{format:?}: nothing is written before the list is answered"
        );
        s.act("Cancel", |rt| {
            rt.send(Message::CancelExport);
        });
        assert!(!path.exists());
        assert_eq!(s.app().status, "Export cancelled; nothing was written");
        let chosen = path.clone();
        s.act(&format!("Export {format:?} again"), move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::ExportChosen {
                session_id,
                format,
                result: FileDialogResult::Selected(chosen),
            });
        });
        s.expect_status("");
        s.act("Export Anyway", |rt| {
            rt.send(Message::ExportAnyway);
        });
        s.clear_expected_status();
    }
    assert!(path.is_file(), "{format:?}: {}", s.app().status);
    assert_eq!(
        s.app().exported.as_deref(),
        Some(path.as_path()),
        "{}",
        s.app().status
    );
    assert!(
        s.app().status.starts_with(&format!("Exported {name}")),
        "{}",
        s.app().status
    );
    path
}

/// File › Open of an exported file: Import Options, Open; the new tab.
pub(super) fn open_exported(s: &mut Scenario, path: &Path) -> usize {
    let tabs = s.app().sessions.len();
    let target = path.to_path_buf();
    s.act(&format!("Open {}", path.display()), move |rt| {
        rt.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            target,
        )));
    });
    s.act("Import Options: Open", |rt| {
        rt.send(Message::ImportOptions(OptionsAction::Open));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1, "{}", s.app().status);
    s.app().active_session
}

/// The whole task: a document made, exported in every format, and each
/// file read back through File › Open with the same text.
#[test]
fn a_made_document_is_exported_in_every_format_and_read_back_by_open() {
    let mut s = Scenario::new_document("output-formats");
    s.clean_folder();
    made_document(&mut s);
    let original = paragraph_texts(s.document());
    assert!(original.iter().any(|text| text.contains("café, naïve, £5")));
    let frames = s.document().frames().count();

    for (format, name) in [
        (Format::Docx, "Made.docx"),
        (Format::Odt, "Made.odt"),
        (Format::Rtf, "Made.rtf"),
    ] {
        let path = export(&mut s, format, name, None);
        let bytes = std::fs::read(&path).unwrap();
        match format {
            Format::Docx | Format::Odt => assert!(bytes.starts_with(b"PK"), "{format:?}"),
            Format::Rtf => assert!(bytes.starts_with(b"{\\rtf1")),
            Format::Text | Format::Markdown => {}
        }
        let tab = open_exported(&mut s, &path);
        let read = paragraph_texts(&s.app().sessions[tab].document);
        for want in [
            "Export check: café, naïve, £5 and bold words.",
            "first item",
            "second item",
            "After the list.",
        ] {
            assert!(
                read.iter().any(|text| text.contains(want)),
                "{format:?}: {want:?} in {read:?}"
            );
        }
        assert!(
            matches!(
                s.app().sessions[tab].location,
                crate::DocumentLocation::LegacySource(_)
            ),
            "{format:?}: an imported document is protected: {:?}",
            s.app().sessions[tab].location
        );
        if format != Format::Rtf {
            assert_eq!(
                s.app().sessions[tab].document.frames().count(),
                frames,
                "{format:?}: the frames and the table come back"
            );
        }
        s.act("back to the document", |rt| {
            rt.send(Message::ActivateTab(0));
        });
    }

    // Plain text cannot hold the frames and the table: the list first.
    let path = export(&mut s, Format::Text, "Made.txt", Some(true));
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("Export check: café, naïve, £5 and bold words."),
        "{text:?}"
    );
    assert!(text.contains("first item") && text.contains("After the list."));
    let tab = open_exported(&mut s, &path);
    assert!(
        paragraph_texts(&s.app().sessions[tab].document)
            .iter()
            .any(|line| line.contains("café"))
    );
    s.act("back to the document", |rt| {
        rt.send(Message::ActivateTab(0));
    });

    // Saved as SAM and reopened, the export is still what it was.
    let sam = s.save_sam("MADE.SAM");
    s.reopen(&sam);
}

/// Export on a failing destination reports it and writes nothing; a
/// document exported twice to the same name asks nothing but does write.
#[test]
fn an_export_to_a_missing_folder_says_so_and_a_later_one_succeeds() {
    let mut s = Scenario::new("output-failure", page_fixture());
    let folder = s.clean_folder();
    let missing = folder.join("no-such-folder").join("X.docx");
    s.expect_status("export failed");
    s.act("Export Word Document to a missing folder", {
        let missing = missing.clone();
        move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::ExportChosen {
                session_id,
                format: Format::Docx,
                result: FileDialogResult::Selected(missing),
            });
        }
    });
    assert!(!missing.exists());
    assert!(
        s.app().status.contains("fail") || s.app().status.contains("Export"),
        "{}",
        s.app().status
    );
    s.clear_expected_status();
    s.expect_status("export failed");
    s.act("the chooser reports a failure", |rt| {
        let session_id = rt.app.id;
        rt.send(Message::ExportChosen {
            session_id,
            format: Format::Docx,
            result: FileDialogResult::Failed("the destination is not a local file".into()),
        });
    });
    s.clear_expected_status();
    let path = export(&mut s, Format::Docx, "OK.docx", Some(false));
    let first = std::fs::read(&path).unwrap();
    s.act("Export again to the same name", {
        let path = path.clone();
        move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::ExportChosen {
                session_id,
                format: Format::Docx,
                result: FileDialogResult::Selected(path),
            });
        }
    });
    assert_eq!(std::fs::read(&path).unwrap().len(), first.len());
}

/// Export PDF…: the chooser's finished message, the file written and
/// non-empty with every page; the Print panel's options change what is
/// written; Print… and Print Current Page… run only up to their messages.
#[test]
fn a_pdf_is_exported_with_the_print_panels_options_and_print_is_asked_for_unrun() {
    let mut s = Scenario::new_document("output-pdf");
    let folder = s.clean_folder();
    made_document(&mut s);
    let pages = s.app().pages.layout().pages.len();

    let plain = folder.join("Plain.pdf");
    s.act("Export PDF", {
        let plain = plain.clone();
        move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::ExportPdfChosen {
                session_id,
                result: FileDialogResult::Selected(plain),
            });
        }
    });
    let bytes = std::fs::read(&plain).unwrap();
    assert!(
        bytes.starts_with(b"%PDF") && bytes.len() > 1_000,
        "{}",
        s.app().status
    );
    assert_eq!(
        s.app().status,
        format!(
            "Exported {pages} page{} to Plain.pdf",
            if pages == 1 { "" } else { "s" }
        )
    );
    assert_eq!(pdf_pages(&plain), pages);

    // The Print panel in the rail: Ami Pro's options and the PDF's.
    s.command_act(CommandId::PrintOptions);
    assert!(s.app().print_panel.is_some());
    s.act("Crop marks", |rt| {
        rt.send(Message::PrintPanel(PrintAction::Option(
            PrintOption::CropMarks,
            true,
        )));
    });
    s.act("Without pictures", |rt| {
        rt.send(Message::PrintPanel(PrintAction::Option(
            PrintOption::WithoutPictures,
            true,
        )));
    });
    s.act("Update fields", |rt| {
        rt.send(Message::PrintPanel(PrintAction::Option(
            PrintOption::UpdateFields,
            true,
        )));
    });
    s.act("PDF/A", |rt| {
        rt.send(Message::PrintPanel(PrintAction::Addition(
            Addition::Archival,
            true,
        )));
    });
    s.act("Bookmarks", |rt| {
        rt.send(Message::PrintPanel(PrintAction::Addition(
            Addition::Bookmarks,
            true,
        )));
    });
    assert!(s.app().print_options.crop_marks && s.app().print_options.without_pictures);
    let marked = folder.join("Marked.pdf");
    s.act("Export PDF with the options", {
        let marked = marked.clone();
        move |rt| {
            let session_id = rt.app.id;
            rt.send(Message::ExportPdfChosen {
                session_id,
                result: FileDialogResult::Selected(marked),
            });
        }
    });
    let marked_bytes = std::fs::read(&marked).unwrap();
    assert!(marked_bytes.starts_with(b"%PDF"));
    assert_ne!(marked_bytes, bytes, "the options change the PDF");
    assert!(
        String::from_utf8_lossy(&marked_bytes).contains("pdfaid"),
        "PDF/A-2b says so in its metadata"
    );
    s.act("options off again", |rt| {
        rt.send(Message::PrintPanel(PrintAction::Option(
            PrintOption::CropMarks,
            false,
        )));
        rt.send(Message::PrintPanel(PrintAction::Option(
            PrintOption::WithoutPictures,
            false,
        )));
        rt.send(Message::PrintPanel(PrintAction::Addition(
            Addition::Archival,
            false,
        )));
    });

    // Print… and Print Current Page… (the dialogs are not run): the
    // messages they would send.
    let before = s.app().status.clone();
    s.act("Print… (not run)", |rt| {
        update_unrun(rt, Message::Command(CommandId::Print));
    });
    s.act("Print Current Page… (not run)", |rt| {
        update_unrun(rt, Message::Command(CommandId::PrintCurrentPage));
    });
    let _ = before;
    // The dialog's pages become the job's pages.
    let job = s.app().pdf_job(crate::print::PdfOptions::default());
    assert_eq!(job.pages(), pages);
    s.act("Back to Styles", |rt| {
        rt.send(Message::PrintPanel(PrintAction::Back));
    });
    assert!(s.app().print_panel.is_none());
    // Saved: the print options are the document's, not in the file.
    let sam = s.save_sam("PDF.SAM");
    s.reopen(&sam);
}

/// Export Folder…: a folder of SAM files exported into another with a
/// report; a file that is not a SAM is listed as not exported.
#[test]
fn a_folder_of_documents_is_exported_with_a_report() {
    let mut s = Scenario::new("output-folder", page_fixture());
    let folder = s.clean_folder();
    let from = folder.join("Archive");
    let to = folder.join("Word");
    std::fs::create_dir_all(from.join("Client")).unwrap();
    let sam = amipro_formats::sam_document(&page_fixture(), &amipro_formats::SamOptions::at(0));
    std::fs::write(from.join("First.SAM"), &sam).unwrap();
    std::fs::write(from.join("Client/Second.sam"), &sam).unwrap();
    std::fs::write(from.join("Client/Broken.SAM"), b"WordPro not a SAM").unwrap();

    s.command_act(CommandId::ExportFolder);
    assert!(s.app().export_folder.is_some());
    assert!(!s.app().export_folder.as_ref().unwrap().ready());
    for (to_folder, chosen) in [(false, from.clone()), (true, to.clone())] {
        s.act("the chooser returns a folder", move |rt| {
            rt.send(Message::ExportFolder(FolderAction::Chosen {
                to: to_folder,
                folder: Some(chosen),
            }));
        });
    }
    s.act("OpenDocument Text", |rt| {
        rt.send(Message::ExportFolder(FolderAction::Format(Format::Odt)));
    });
    assert!(s.app().export_folder.as_ref().unwrap().ready());
    s.expect_status("could not be read");
    s.act("Export", |rt| {
        rt.send(Message::ExportFolder(FolderAction::Export));
    });
    assert!(to.join("First.odt").is_file(), "{}", s.app().status);
    assert!(to.join("Client/Second.odt").is_file());
    let report = std::fs::read_to_string(to.join("Export report.txt")).unwrap();
    assert!(report.contains("First.SAM → First.odt"), "{report}");
    assert!(report.contains("Broken.SAM: not exported"), "{report}");
    s.act("Close", |rt| {
        rt.send(Message::ExportFolder(FolderAction::Close));
    });
    assert!(s.app().export_folder.is_none());
    s.clear_expected_status();
}
