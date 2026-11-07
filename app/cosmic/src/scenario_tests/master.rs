//! Master documents as a user makes them (`UI_SPEC.md` §37; the
//! "Master documents" checklist of `UI_ACCEPTANCE.md` §10).

use super::*;
use crate::find_bar::{Field, FindAction, FindMenu, Toggle};
use crate::master_panel::MasterAction as M;
use std::path::Path;

/// A member of two pages with two footnotes and an index mark, as the
/// master tests' fixtures.
fn member(folder: &Path, name: &str) -> PathBuf {
    let edoc = [
        format!("@Title@{name} chapter one"),
        format!("Text of {name}.<:F\r\n@Footnote@First of {name}.\r\n\r\n>"),
        "The end of page one.<:p<* >>".to_string(),
        format!(
            "More text of {name}.<:F\r\n@Footnote@Second of {name}.\r\n\r\n><:X8,-32768;Index \"Common\" #>"
        ),
    ]
    .map(|paragraph| format!("{paragraph}\r\n\r\n"))
    .concat();
    let bytes = format!("[ver]\r\n\t4\r\n[edoc]\r\n{edoc}>\r\n\r\n");
    let path = folder.join(format!("{name}.SAM"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn tab_of(s: &Scenario, name: &str) -> usize {
    s.app()
        .sessions
        .iter()
        .position(|session| {
            session
                .file_path()
                .is_some_and(|path| path.file_name().is_some_and(|file| file == name))
        })
        .unwrap_or_else(|| s.fail("tab", &format!("no tab for {name}")))
}

fn go_to(s: &mut Scenario, index: usize) {
    s.act(&format!("click tab {index}"), move |rt| {
        rt.send(Message::ActivateTab(index));
    });
}

fn shown_pages(s: &Scenario, index: usize) -> Vec<u32> {
    s.app().sessions[index]
        .pages
        .layout()
        .pages
        .iter()
        .map(|page| page.shown)
        .collect()
}

fn file_names(s: &Scenario) -> Vec<String> {
    s.app()
        .master_files_for_tests()
        .iter()
        .map(|file| {
            file.rsplit(['/', '\\'])
                .next()
                .unwrap_or(file)
                .to_uppercase()
        })
        .collect()
}

fn inflate(data: &[u8], size: usize) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let mut out = Vec::with_capacity(size);
    flate2::read::DeflateDecoder::new(data)
        .take(size as u64 + 1)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

/// The text of an exported file read back by the app's importer.
fn exported_text(path: &Path, format: amipro_exchange::Format) -> String {
    let bytes = std::fs::read(path).unwrap();
    let read = amipro_exchange::import::read(
        &bytes,
        Some(format.extension()),
        &amipro_exchange::import::ImportOptions::default(),
        &inflate,
    )
    .unwrap()
    .document;
    let mut text = String::new();
    for section in &read.sections {
        for block in &section.blocks {
            if let amipro_exchange::Block::Paragraph(paragraph) = block {
                for inline in &paragraph.inlines {
                    if let amipro_exchange::Inline::Text { text: part, .. } = inline {
                        text.push_str(part);
                    }
                }
                text.push('\n');
            }
        }
    }
    text
}

/// The whole task: a master made of three files, ordered, numbered live,
/// the set's TOC and index, Find and Replace across it, printed and
/// exported, saved and reopened.
#[test]
fn a_master_document_is_made_ordered_numbered_searched_exported_and_reopened() {
    // The master's TOC levels apply to every file (Ami Pro's run).
    let master_document = amipro_formats::import_sam(
        b"[ver]\r\n\t4\r\n[toc]\r\n\tTitle\r\n\t\r\n\t\r\n\t.\r\n\t\r\n\t\r\n\t3\r\n[edoc]\r\nThe master.\r\n\r\n>\r\n\r\n",
        None,
        "Master",
    )
    .unwrap()
    .document;
    let mut s = Scenario::new("master-whole", master_document);
    let folder = s.clean_folder();
    let paths: Vec<PathBuf> = ["MA", "MB", "MC"]
        .iter()
        .map(|name| member(&folder, name))
        .collect();
    s.command_act(CommandId::MasterDocument);
    assert!(s.app().master_panel.is_some());
    let master = s.app().active_session;

    // Add… the three (one undo step); then the panel reads them.
    s.edit("Add the three files", {
        let paths = paths.clone();
        move |rt| {
            rt.send(Message::Master(M::Added(paths)));
        }
    });
    s.act("the files are read", |rt| {
        rt.send(Message::Repaint);
    });
    assert_eq!(file_names(&s), ["MA.SAM", "MB.SAM", "MC.SAM"]);
    let set = s.app().master_set.as_ref().expect("the set");
    assert_eq!(
        set.members
            .iter()
            .map(|member| member.pages)
            .collect::<Vec<_>>(),
        [Some((1, 2)), Some((3, 4)), Some((5, 6))]
    );

    // Order: the third up above the second, and so on, ending as it began.
    let step = |s: &mut Scenario, select: usize, up: bool, want: [&str; 3]| {
        // Select toggles; the moved file stays selected.
        if s.app().master_set.as_ref().and_then(|set| set.selected) != Some(select) {
            s.act("select a file", move |rt| {
                rt.send(Message::Master(M::Select(select)));
            });
        }
        s.edit(if up { "Up" } else { "Down" }, move |rt| {
            rt.send(Message::Master(if up { M::Up } else { M::Down }));
        });
        assert_eq!(file_names(s), want);
        s.act("the files are read", |rt| {
            rt.send(Message::Repaint);
        });
        let pages: Vec<_> = s
            .app()
            .master_set
            .as_ref()
            .unwrap()
            .members
            .iter()
            .map(|member| member.pages)
            .collect();
        assert_eq!(
            pages,
            [Some((1, 2)), Some((3, 4)), Some((5, 6))],
            "{want:?}"
        );
    };
    step(&mut s, 2, true, ["MA.SAM", "MC.SAM", "MB.SAM"]);
    step(&mut s, 1, true, ["MC.SAM", "MA.SAM", "MB.SAM"]);
    step(&mut s, 0, false, ["MA.SAM", "MC.SAM", "MB.SAM"]);
    step(&mut s, 1, false, ["MA.SAM", "MB.SAM", "MC.SAM"]);
    // Remove the last and add it again.
    if s.app().master_set.as_ref().and_then(|set| set.selected) != Some(2) {
        s.act("select the last", |rt| {
            rt.send(Message::Master(M::Select(2)));
        });
    }
    s.edit("Remove", |rt| {
        rt.send(Message::Master(M::Remove));
    });
    assert_eq!(file_names(&s), ["MA.SAM", "MB.SAM"]);
    s.edit("Add the removed file again", {
        let path = paths[2].clone();
        move |rt| {
            rt.send(Message::Master(M::Added(vec![path])));
        }
    });
    s.act("the files are read", |rt| {
        rt.send(Message::Repaint);
    });
    let mut names = file_names(&s);
    names.sort();
    assert_eq!(names, ["MA.SAM", "MB.SAM", "MC.SAM"]);
    if file_names(&s) != ["MA.SAM", "MB.SAM", "MC.SAM"] {
        let at = file_names(&s).iter().position(|n| n == "MC.SAM").unwrap();
        eprintln!("NOTE: Added puts the file at {at}, not at the end");
        if s.app().master_set.as_ref().and_then(|set| set.selected) != Some(at) {
            s.act("select it", move |rt| {
                rt.send(Message::Master(M::Select(at)));
            });
        }
        s.edit("Down", |rt| {
            rt.send(Message::Master(M::Down));
        });
    }
    assert_eq!(file_names(&s), ["MA.SAM", "MB.SAM", "MC.SAM"]);
    s.act("the files are read", |rt| {
        rt.send(Message::Repaint);
    });

    // MB opened from the panel continues from MA, live.
    s.act("Open MB", |rt| {
        rt.send(Message::Master(M::Open(1)));
    });
    let mb = tab_of(&s, "MB.SAM");
    assert_eq!(shown_pages(&s, mb), [3, 4]);
    assert!(
        !s.app().sessions[mb].dirty,
        "the set's numbers alone change nothing"
    );
    go_to(&mut s, master);
    s.act("Open MA", |rt| {
        rt.send(Message::Master(M::Open(0)));
    });
    let ma = tab_of(&s, "MA.SAM");

    // A page break added in MA moves MB's numbers at once.
    go_to(&mut s, ma);
    let main = s.main_flow();
    s.caret(main, 0, 0, 1, false);
    s.command(CommandId::PageBreak);
    s.act("the set is renumbered", |rt| {
        rt.send(Message::Repaint);
    });
    assert_eq!(
        shown_pages(&s, mb),
        [4, 5],
        "MB starts after MA's three pages"
    );
    // Undo in MA takes it back.
    s.act("Undo in MA", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    s.act("the set is renumbered", |rt| {
        rt.send(Message::Repaint);
    });
    assert_eq!(shown_pages(&s, mb), [3, 4]);

    // The set's TOC in a new document (every file with its pages in the
    // set), then the index from the master again.
    go_to(&mut s, master);
    let tabs = s.app().sessions.len();
    s.act("Generate Contents", |rt| {
        rt.send(Message::Master(M::GenerateContents));
    });
    assert_eq!(
        s.app().sessions.len(),
        tabs + 1,
        "the TOC opens in a new document"
    );
    let toc: Vec<String> = s
        .document()
        .flows()
        .map(|(_, flow)| flow.plain_text())
        .collect();
    let toc = toc.join("|");
    for want in ["MA chapter one", "MB chapter one", "MC chapter one"] {
        assert!(toc.contains(want), "{want}: {toc:?}");
    }
    assert_eq!(s.app().set_links.len(), 3, "a link for each row");
    let toc_tab = s.app().active_session;
    let toc_path = s.save_sam("SETTOC.SAM");
    s.reopen(&toc_path);
    go_to(&mut s, master);
    s.act("Generate Index", |rt| {
        rt.send(Message::Master(M::GenerateIndex));
    });
    assert_ne!(s.app().active_session, toc_tab);
    assert!(
        s.document()
            .paragraphs()
            .iter()
            .any(|paragraph| paragraph.text == "Common, 2, 4, 6"),
        "{:?}",
        s.document()
            .paragraphs()
            .iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
    );
    go_to(&mut s, master);

    // Find across the set: counts, Next into the next file, Replace All.
    s.act("Find", |rt| {
        rt.send(Message::Command(CommandId::Find));
    });
    s.act("type the word", |rt| {
        rt.send(Message::Find(FindAction::Text(
            Field::Find,
            "text of".into(),
        )));
    });
    s.act("In All Files of the Master", |rt| {
        rt.send(Message::Find(FindAction::Menu(FindMenu::Toggle(
            Toggle::AllFiles,
        ))));
    });
    let bar = s.app().find.as_ref().expect("the find bar");
    assert_eq!(bar.count_label(), "6 in 3 files");
    s.act("Find Next", |rt| {
        rt.send(Message::Find(FindAction::Next));
    });
    let shown = |s: &Scenario| {
        let session = &s.app().sessions[s.app().active_session];
        (
            session
                .file_path()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned()),
            session.status.clone(),
        )
    };
    assert_eq!(
        shown(&s),
        (Some("MA.SAM".into()), "1 of 2 in MA.SAM".into())
    );
    s.act("Find Next", |rt| {
        rt.send(Message::Find(FindAction::Next));
    });
    s.act("Find Next", |rt| {
        rt.send(Message::Find(FindAction::Next));
    });
    assert_eq!(
        shown(&s),
        (Some("MB.SAM".into()), "1 of 2 in MB.SAM".into())
    );
    s.act("Find Previous", |rt| {
        rt.send(Message::Find(FindAction::Previous));
    });
    assert_eq!(shown(&s).0.as_deref(), Some("MA.SAM"));
    go_to(&mut s, master);
    s.act("Replace…", |rt| {
        rt.send(Message::Command(CommandId::Replace));
    });
    s.act("the word", |rt| {
        rt.send(Message::Find(FindAction::Text(
            Field::Find,
            "text of".into(),
        )));
    });
    if !s.app().find.as_ref().unwrap().all_files {
        s.act("In All Files of the Master", |rt| {
            rt.send(Message::Find(FindAction::Menu(FindMenu::Toggle(
                Toggle::AllFiles,
            ))));
        });
    }
    s.act("the replacement", |rt| {
        rt.send(Message::Find(FindAction::Text(
            Field::Replace,
            "words of".into(),
        )));
    });
    s.act("Replace All", |rt| {
        rt.send(Message::Find(FindAction::ReplaceAll));
    });
    assert_eq!(s.app().active_session, master);
    assert_eq!(s.app().status, "Replaced 6 matches in 3 files");
    let text_of = |s: &Scenario, name: &str| -> String {
        s.app().sessions[tab_of(s, name)]
            .document
            .main_flow()
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.text.clone())
            .collect::<Vec<_>>()
            .join("|")
    };
    assert!(text_of(&s, "MA.SAM").contains("words of MA."));
    assert!(text_of(&s, "MC.SAM").contains("More words of MC."));
    // One undo step in MB takes MB's back and leaves the others.
    let mb = tab_of(&s, "MB.SAM");
    go_to(&mut s, mb);
    s.act("Undo in MB", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(
        text_of(&s, "MB.SAM").contains("Text of MB.")
            && text_of(&s, "MB.SAM").contains("More text of MB.")
    );
    assert!(text_of(&s, "MA.SAM").contains("words of MA."));
    s.act("close the find bar", |rt| {
        rt.send(Message::Find(FindAction::Close));
    });

    // The whole set exported in each format (the chooser's finished
    // message), every file in order, the master's own text left out.
    go_to(&mut s, master);
    use amipro_exchange::Format;
    for format in [Format::Docx, Format::Odt, Format::Rtf, Format::Text] {
        let path = folder.join(format!("Set.{}", format.extension()));
        s.act(&format!("Export {format:?}"), {
            let path = path.clone();
            move |rt| {
                let session_id = rt.app.id;
                rt.send(Message::ExportChosen {
                    session_id,
                    format,
                    result: FileDialogResult::Selected(path),
                });
                if rt.app.export_pending.is_some() {
                    rt.send(Message::ExportAnyway);
                }
            }
        });
        assert_eq!(
            s.app().exported.as_deref(),
            Some(path.as_path()),
            "{format:?}: {}",
            s.app().status
        );
        let text = exported_text(&path, format);
        let at = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{format:?}: {needle} in {text:?}"))
        };
        assert!(at("words of MA.") < at("Text of MB."), "{format:?}");
        assert!(at("Text of MB.") < at("words of MC."), "{format:?}");
        assert!(
            !text.contains("The master."),
            "{format:?}: the master's own text"
        );
    }
    // The whole set as one PDF; This document only.
    let job = s.app().pdf_job(crate::print::PdfOptions::default());
    // The TOC saved beside the files is the set's first file.
    let titles: Vec<_> = job.set.iter().map(|part| part.title.clone()).collect();
    assert_eq!(titles, ["SETTOC.SAM", "MA.SAM", "MB.SAM", "MC.SAM"]);
    assert_eq!(job.pages(), 7, "the TOC's page and six");
    assert!(job.render().unwrap().starts_with(b"%PDF"));
    s.act("This document only", |rt| {
        rt.send(Message::PrintPanel(
            crate::print_panel::PrintAction::WholeSet(false),
        ));
    });
    assert!(
        s.app()
            .pdf_job(crate::print::PdfOptions::default())
            .set
            .is_empty()
    );

    // The master saved beside its files and reopened: the set found by name.
    let path = s.save_sam("MS.SAM");
    let mut reopened = s.reopen(&path);
    reopened.command_act(CommandId::MasterDocument);
    reopened.act("the files are read", |rt| {
        rt.send(Message::Repaint);
    });
    let set = reopened.app().master_set.as_ref().expect("the set");
    assert_eq!(set.members.len(), 3);
    assert!(
        set.members.iter().all(|member| member.path.is_some()),
        "found"
    );
    assert_eq!(
        set.members
            .iter()
            .map(|member| member.pages)
            .collect::<Vec<_>>()
            .first(),
        Some(&Some((1, 2)))
    );
}

/// A member of a master is treated as a single document, and finds its
/// master; the panel's checks go to the file.
#[test]
fn a_member_opened_alone_finds_its_master_and_can_leave_the_set() {
    let mut s = Scenario::new_document("master-member");
    let folder = s.clean_folder();
    member(&folder, "MA");
    let path = member(&folder, "MB");
    // MB as Ami Pro's run left it: pages after 2, footnotes after 2.
    let bytes = std::fs::read_to_string(&path).unwrap().replace(
        "[edoc]",
        "[sequence]\r\n\t2\r\n\t2\r\n\t0\r\n\t0\r\n\t0\r\n[edoc]",
    );
    std::fs::write(&path, bytes).unwrap();
    std::fs::write(
        folder.join("MS.SAM"),
        "[ver]\r\n\t4\r\n[master]\r\n\t\r\n\t\r\n\t0\r\n\t30\r\n\t2\r\n\tc:\\book\\ma.sam\r\n\tc:\\book\\mb.sam\r\n[edoc]\r\nSource.\r\n\r\n>\r\n\r\n",
    )
    .unwrap();
    s.open(&path);
    s.command_act(CommandId::MasterDocument);
    assert_eq!(
        s.app()
            .master_offer
            .clone()
            .flatten()
            .and_then(|path| path.file_name().map(|name| name.to_os_string())),
        Some("MS.SAM".into())
    );
    s.act("Open Master", |rt| {
        rt.send(Message::Master(M::OpenMaster));
    });
    s.act("the files are read", |rt| {
        rt.send(Message::Repaint);
    });
    let master = tab_of(&s, "MS.SAM");
    let set = s.app().sessions[master].master_set.as_ref().unwrap();
    assert!(
        set.members.iter().all(|member| member.path.is_some()),
        "found by name"
    );
    let mb = tab_of(&s, "MB.SAM");
    assert!(s.app().sessions[mb].live_sequence.is_some());
    assert_eq!(shown_pages(&s, mb), [3, 4]);
    go_to(&mut s, mb);
    s.edit("Treat as Single Document", |rt| {
        rt.send(Message::Master(M::TreatAsSingle));
    });
    assert!(s.document().contents().sequence.is_none());
    s.act("the set is renumbered", |rt| {
        rt.send(Message::Repaint);
    });
    assert_eq!(shown_pages(&s, mb), [1, 2], "its own numbers again");
    s.save();
}

/// A master that names its files saves as Ami Pro's SAM with no loss list
/// (`[master]` is a section Ami Pro stores).
#[test]
fn a_master_with_files_saves_as_sam_without_a_loss_list() {
    let mut s = Scenario::new_document("master-saves");
    let folder = s.clean_folder();
    let paths = vec![member(&folder, "MA")];
    s.type_text("The master.");
    s.command_act(CommandId::MasterDocument);
    s.edit("Add the file", move |rt| {
        rt.send(Message::Master(M::Added(paths)));
    });
    let path = s.save_sam("MS.SAM");
    s.reopen(&path);
}
