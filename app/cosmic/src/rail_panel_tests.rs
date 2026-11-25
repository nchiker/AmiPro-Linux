//! The rail shows one panel at a time (`rail_panels.rs`): every ordered
//! pair of panels through the real message path, the user-reported Graphics
//! Scaling case, and the guard that fails when a panel is added without
//! being registered. No portal or window is opened.

use amipro_document::{Offset, PictureScaling, Size};

use super::*;
use crate::page_surface_app_tests::{
    at, cell, draw_window, frame_action, magenta_picture_document, move_to, page_app, send,
    table_app,
};

/// A position of each panel in [`RailPanel`]: an exhaustive match, so that
/// a new panel stops the build here until it is registered.
fn index(panel: RailPanel) -> usize {
    match panel {
        RailPanel::Drawing => 0,
        RailPanel::Frame => 1,
        RailPanel::StyleProperties => 2,
        RailPanel::PageLayout => 3,
        RailPanel::Paragraph => 4,
        RailPanel::Table => 5,
        RailPanel::Fields => 6,
        RailPanel::References => 7,
        RailPanel::Contents => 8,
        RailPanel::Master => 9,
        RailPanel::Envelope => 10,
        RailPanel::Merge => 11,
        RailPanel::Glossary => 12,
        RailPanel::Proofing => 13,
        RailPanel::DocumentProperties => 14,
        RailPanel::ImportFolder => 15,
        RailPanel::ExportFolder => 16,
        RailPanel::ImportReport => 17,
        RailPanel::Print => 18,
        RailPanel::PasteSpecial => 19,
        RailPanel::Links => 20,
        RailPanel::Outline => 21,
        RailPanel::Sort => 22,
        RailPanel::Chart => 23,
        RailPanel::Equation => 24,
        RailPanel::Updates => 25,
        RailPanel::Recovery => 26,
        RailPanel::Help => 27,
        RailPanel::Styles => 28,
    }
}

/// The panels a command opens (Draw mode and Frame Properties are not
/// opened by a panel command), with the command that opens each.
fn open_command(panel: RailPanel) -> Option<CommandId> {
    Some(match panel {
        RailPanel::Drawing | RailPanel::Frame | RailPanel::Styles | RailPanel::ImportReport => {
            return None;
        }
        RailPanel::StyleProperties => CommandId::ModifyCurrentStyle,
        RailPanel::PageLayout => CommandId::PageLayout,
        RailPanel::Paragraph => CommandId::ParagraphProperties,
        RailPanel::Table => CommandId::TableProperties,
        RailPanel::Fields => CommandId::Field,
        RailPanel::References => CommandId::Notes,
        RailPanel::Contents => CommandId::ContentsIndex,
        RailPanel::Master => CommandId::MasterDocument,
        RailPanel::Envelope => CommandId::PrintEnvelope,
        RailPanel::Merge => CommandId::Merge,
        RailPanel::Glossary => CommandId::MarkGlossary,
        RailPanel::Proofing => CommandId::Language,
        RailPanel::DocumentProperties => CommandId::DocumentProperties,
        RailPanel::ImportFolder => CommandId::ImportFolder,
        RailPanel::ExportFolder => CommandId::ExportFolder,
        RailPanel::Print => CommandId::PrintOptions,
        RailPanel::PasteSpecial => CommandId::PasteSpecial,
        RailPanel::Links => CommandId::Links,
        RailPanel::Outline => CommandId::OutlineNavigator,
        RailPanel::Sort => CommandId::Sort,
        RailPanel::Chart => CommandId::Charting,
        RailPanel::Equation => CommandId::Equations,
        RailPanel::Updates => CommandId::CheckForUpdates,
        RailPanel::Recovery => CommandId::RecoverDocuments,
        RailPanel::Help => CommandId::HelpContents,
    })
}

/// Opens `panel` as its menu does.
fn open(app: &mut AmiProApp, panel: RailPanel) {
    match panel {
        RailPanel::ImportReport => {
            app.import_report = Some(import::Report::default());
            send(
                app,
                Message::ImportReport(import_report::ReportAction::Show),
            );
        }
        _ => send(
            app,
            Message::Command(open_command(panel).expect("a panel with a command")),
        ),
    }
    assert_eq!(app.rail_panel(), panel, "{panel:?} opens");
}

/// Closes `panel` with its own X.
fn close(app: &mut AmiProApp, panel: RailPanel) {
    use crate::{
        contents_panel::ContentsAction, document_properties::PropertiesAction,
        envelope_panel::EnvelopeAction, fields_panel::FieldsAction, glossary_panel::GlossaryAction,
        links_panel::LinksAction, master_panel::MasterAction, merge_panel::MergeAction,
        outline_navigator::NavigatorAction, page_layout_properties::LayoutAction,
        paragraph_properties::ParagraphAction, paste_special::PasteSpecialAction,
        print_panel::PrintAction, proofing_panel::ProofingAction,
        references_panel::ReferenceAction, sort_panel::SortAction, style_properties::StyleAction,
        table_properties::TableAction,
    };
    let message = match panel {
        RailPanel::Drawing | RailPanel::Frame | RailPanel::Styles => return,
        RailPanel::StyleProperties => Message::StyleProperties(StyleAction::Close),
        RailPanel::PageLayout => Message::PageLayoutProperties(LayoutAction::Close),
        RailPanel::Paragraph => Message::ParagraphProperties(ParagraphAction::Close),
        RailPanel::Table => Message::TableProperties(TableAction::Close),
        RailPanel::Fields => Message::FieldsPanel(FieldsAction::Close),
        RailPanel::References => Message::References(ReferenceAction::Close),
        RailPanel::Contents => Message::Contents(ContentsAction::Close),
        RailPanel::Master => Message::Master(MasterAction::Close),
        RailPanel::Envelope => Message::Envelope(EnvelopeAction::Close),
        RailPanel::Merge => Message::Merge(MergeAction::Close),
        RailPanel::Glossary => Message::Glossary(GlossaryAction::Close),
        RailPanel::Proofing => Message::Proofing(ProofingAction::Close),
        RailPanel::DocumentProperties => Message::DocumentProperties(PropertiesAction::Close),
        RailPanel::ImportFolder => Message::ImportFolder(import_folder::FolderAction::Close),
        RailPanel::ExportFolder => Message::ExportFolder(export_folder::FolderAction::Close),
        RailPanel::ImportReport => Message::ImportReport(import_report::ReportAction::Close),
        RailPanel::Print => Message::PrintPanel(PrintAction::Close),
        RailPanel::PasteSpecial => Message::PasteSpecial(PasteSpecialAction::Close),
        RailPanel::Links => Message::Links(LinksAction::Close),
        RailPanel::Outline => Message::OutlineNavigator(NavigatorAction::Close),
        RailPanel::Sort => Message::Sort(SortAction::Close),
        RailPanel::Chart => Message::Chart(crate::chart_panel::ChartAction::Close),
        RailPanel::Equation => Message::Equation(crate::equation_panel::EquationAction::Close),
        RailPanel::Updates => Message::Updates(crate::updates_panel::UpdatesAction::Close),
        RailPanel::Recovery => Message::Recovery(crate::recovery_panel::RecoveryAction::Close),
        RailPanel::Help => Message::HelpPanel(crate::help_panel::PanelAction::Close),
    };
    send(app, message);
}

/// A table with the caret in its first cell, so that every panel can open.
fn app_for_every_panel() -> AmiProApp {
    let (mut app, frame) = table_app();
    let target = at(cell(&app.document, frame, 0, 0), 0, 0);
    move_to(&mut app, target, 1, false);
    app
}

#[test]
fn every_panel_is_registered() {
    assert_eq!(RailPanel::ALL.len(), index(RailPanel::Styles));
    for (position, panel) in RailPanel::ALL.into_iter().enumerate() {
        assert_eq!(index(panel), position, "{panel:?} is in ALL, in order");
    }
    // Every panel the rail can show has a way to open it and one to close
    // it in this file, and a state `close_rail_panels` clears.
    let mut app = app_for_every_panel();
    for panel in RailPanel::ALL {
        if open_command(panel).is_none() && panel != RailPanel::ImportReport {
            continue;
        }
        open(&mut app, panel);
        assert!(app.rail_state_set(panel), "{panel:?} has its state set");
        app.close_rail_panels();
        assert!(
            RailPanel::ALL
                .into_iter()
                .filter(|other| *other != RailPanel::Drawing)
                .all(|other| !app.rail_state_set(other)),
            "close_rail_panels leaves {panel:?}'s state set"
        );
        assert_eq!(app.rail_panel(), RailPanel::Styles);
    }
}

/// Every ordered pair: A then B shows B, leaves no state of A, and B's X
/// returns to the Styles list.
#[test]
fn the_panel_opened_last_is_the_only_one_that_shows() {
    let mut app = app_for_every_panel();
    let panels: Vec<RailPanel> = RailPanel::ALL
        .into_iter()
        .filter(|panel| open_command(*panel).is_some() || *panel == RailPanel::ImportReport)
        .collect();
    assert_eq!(panels.len(), 26);
    for first in &panels {
        for second in &panels {
            if first == second {
                continue;
            }
            open(&mut app, *first);
            open(&mut app, *second);
            assert_eq!(
                app.rail_panel(),
                *second,
                "{first:?} then {second:?} shows {second:?}"
            );
            for other in &panels {
                // The Envelope panel keeps a shown Merge panel behind it.
                let behind = *first == RailPanel::Merge && *second == RailPanel::Envelope;
                if other != second && !(behind && *other == RailPanel::Merge) {
                    assert!(
                        !app.rail_state_set(*other),
                        "{first:?} then {second:?} leaves {other:?} set"
                    );
                }
            }
            assert!(app.frame_selection.is_none());
            close(&mut app, *second);
            let after = app.rail_panel();
            let behind = *first == RailPanel::Merge && *second == RailPanel::Envelope;
            assert_eq!(
                after,
                if behind {
                    RailPanel::Merge
                } else {
                    RailPanel::Styles
                },
                "{first:?} then {second:?}, then {second:?} closed"
            );
            app.close_rail_panels();
            if let Some(panel) = app.merge_panel.as_mut() {
                panel.shown = false;
            }
        }
    }
}

/// Each panel's view builds in the window.
#[test]
fn every_panel_builds_in_the_window() {
    let mut app = app_for_every_panel();
    for panel in RailPanel::ALL {
        if open_command(panel).is_none() && panel != RailPanel::ImportReport {
            continue;
        }
        open(&mut app, panel);
        assert!(app.rail_open());
        draw_window(&app);
    }
}

/// The frames' Frame Properties follows the selection: selecting a frame
/// shows it over an open panel, and opening a panel deselects the frames.
#[test]
fn frame_properties_follows_the_selection_and_a_panel_deselects() {
    use crate::frame_editing::FrameAction;
    let document = magenta_picture_document(
        20,
        10,
        PictureScaling::imported(Size {
            width: 1,
            height: 1,
        }),
        Offset::default(),
    );
    let frame = document.frame_stack()[0];
    let mut app = page_app(document);
    let select = |app: &mut AmiProApp| {
        frame_action(
            app,
            FrameAction::Select {
                frame,
                page: 1,
                extend: false,
            },
        );
    };
    send(&mut app, Message::Command(CommandId::Links));
    assert_eq!(app.rail_panel(), RailPanel::Links);
    select(&mut app);
    assert_eq!(app.rail_panel(), RailPanel::Frame);
    send(&mut app, Message::Command(CommandId::Sort));
    assert_eq!(app.rail_panel(), RailPanel::Sort);
    assert!(app.frame_selection.is_none());
    select(&mut app);
    send(&mut app, Message::Command(CommandId::PasteSpecial));
    assert_eq!(app.rail_panel(), RailPanel::PasteSpecial);
    draw_window(&app);
}

/// Format ▸ Graphics Scaling… on a picture frame: the rail shows Frame
/// Properties (its Picture section) whatever the rail was doing before:
/// closed, another panel open, a narrow window (the rail over the page), a
/// resize across the docking width.
#[test]
fn graphics_scaling_shows_frame_properties_in_every_rail_state() {
    use crate::frame_editing::FrameAction;
    let document = magenta_picture_document(
        20,
        10,
        PictureScaling::imported(Size {
            width: 1,
            height: 1,
        }),
        Offset::default(),
    );
    let frame = document.frame_stack()[0];
    let mut app = page_app(document);
    let select = |app: &mut AmiProApp| {
        frame_action(
            app,
            FrameAction::Select {
                frame,
                page: 1,
                extend: false,
            },
        );
    };
    let shows_picture = |app: &AmiProApp| {
        assert_eq!(app.rail_panel(), RailPanel::Frame);
        assert!(app.rail_open(), "the rail shows");
        let (inspected, _) = app.inspected_frame().expect("one frame inspected");
        assert_eq!(inspected, frame);
        assert!(matches!(
            app.document.frame(inspected).unwrap().content,
            amipro_document::FrameContent::Picture(_)
        ));
        draw_window(app);
    };
    // Docked and open.
    select(&mut app);
    send(&mut app, Message::Command(CommandId::GraphicsScaling));
    shows_picture(&app);
    // The rail closed.
    send(&mut app, Message::Command(CommandId::StylesPanel));
    assert!(!app.rail_open());
    send(&mut app, Message::Command(CommandId::GraphicsScaling));
    shows_picture(&app);
    // Another panel open before the frame is selected.
    send(&mut app, Message::Command(CommandId::Links));
    select(&mut app);
    send(&mut app, Message::Command(CommandId::GraphicsScaling));
    shows_picture(&app);
    // A narrow window: the rail starts closed and opens over the page.
    send(&mut app, Message::Command(CommandId::StylesPanel));
    send(&mut app, Message::WindowResized(1_400.0));
    send(&mut app, Message::WindowResized(900.0));
    assert!(app.rail_overlays());
    assert!(!app.rail_open(), "the closed rail stays closed");
    send(&mut app, Message::Command(CommandId::GraphicsScaling));
    shows_picture(&app);
    assert!(app.rail.overlay_open);
    // Across the docking width and back: the same panel, still open.
    send(&mut app, Message::WindowResized(1_400.0));
    assert!(!app.rail_overlays());
    shows_picture(&app);
    send(&mut app, Message::WindowResized(900.0));
    shows_picture(&app);
    // Closed over the page, then asked for again.
    send(&mut app, Message::Command(CommandId::StylesPanel));
    assert!(!app.rail_open());
    send(&mut app, Message::WindowResized(1_400.0));
    assert!(!app.rail_open());
    send(&mut app, Message::Command(CommandId::GraphicsScaling));
    shows_picture(&app);
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: every rail panel, every Preferences page,
/// and the dialogs a command opens, light and dark (the UI review,
/// `docs/research/UI_REVIEW_2026-11.md`; not committed images).
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn ui_review_screenshots() {
    use crate::page_surface_app_tests::write_window_ppm;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let write = |app: &AmiProApp, name: &str| {
        for dark in [false, true] {
            let suffix = if dark { "dark" } else { "light" };
            write_window_ppm(app, &directory.join(format!("{name}-{suffix}.ppm")), dark);
        }
    };
    let mut app = app_for_every_panel();
    send(&mut app, Message::Command(CommandId::StylesPanel));
    if !app.rail_open() {
        send(&mut app, Message::Command(CommandId::StylesPanel));
    }
    write(&app, "panel-Styles");
    for panel in RailPanel::ALL {
        if open_command(panel).is_none() && panel != RailPanel::ImportReport {
            continue;
        }
        open(&mut app, panel);
        write(&app, &format!("panel-{panel:?}"));
        app.close_rail_panels();
    }
    // Preferences, each page.
    for (page, name) in crate::preferences_dialog::Page::ALL {
        let mut app = app_for_every_panel();
        send(&mut app, Message::Command(CommandId::Preferences));
        send(
            &mut app,
            Message::Preferences(crate::preferences_dialog::PreferencesAction::Page(page)),
        );
        write(&app, &format!("prefs-{name}"));
    }
    // The dialogs that commands open (none is a portal).
    for command in [
        CommandId::Font,
        CommandId::SpecialEffects,
        CommandId::SpecialCharacter,
        CommandId::NewFromStyleSheet,
        CommandId::Password,
        CommandId::GoToPage,
        CommandId::HeaderFooter,
        CommandId::PageNumber,
        CommandId::Frame,
        CommandId::InsertBullet,
        CommandId::OutlineNumbering,
        CommandId::CreateStyle,
        CommandId::StyleManagement,
        CommandId::UseAnotherStyleSheet,
        CommandId::SaveAsStyleSheet,
        CommandId::CustomizeToolbar,
        CommandId::LineNumbering,
        CommandId::BulletNumberStyle,
        CommandId::WordCount,
        CommandId::ReadabilityStatistics,
        CommandId::Bookmarks,
        CommandId::FootnoteOptions,
        CommandId::DateTime,
        CommandId::CrossReference,
        CommandId::MergeField,
        CommandId::InsertGlossary,
        CommandId::MarkIndexEntry,
        CommandId::Spelling,
        CommandId::Thesaurus,
        CommandId::GrammarCheck,
        CommandId::EditFormula,
        CommandId::Find,
        CommandId::Replace,
    ] {
        let mut app = app_for_every_panel();
        send(&mut app, Message::Command(CommandId::StylesPanel));
        if !app.rail_open() {
            send(&mut app, Message::Command(CommandId::StylesPanel));
        }
        send(&mut app, Message::Command(command));
        write(&app, &format!("cmd-{command:?}"));
    }
    // A plain text caret (not in a table), with the Format Painter armed.
    let mut app = page_app(crate::page_surface::page_fixture());
    send(&mut app, Message::Command(CommandId::VerticalRuler));
    send(&mut app, Message::Command(CommandId::FormatPainter));
    write(&app, "window-painter-vruler");
    send(&mut app, Message::Command(CommandId::Table));
    write(&app, "cmd-Table");
}

/// Preferences › Folders and View with real-looking content: a long
/// Documents folder, two remembered style-sheet folders (one long), and
/// whatever `AMIPRO_STYLES` holds; light and dark. Ignored; writes
/// `prefs-folders-*.ppm` and `prefs-view-*.ppm` to `AMIPRO_SCREENSHOT_DIR`.
#[test]
#[ignore = "writes screenshots for review; set AMIPRO_SCREENSHOT_DIR"]
fn preferences_folders_screenshots() {
    use crate::page_surface_app_tests::write_window_ppm;
    use crate::preferences_dialog::{Page, PreferencesAction};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = app_for_every_panel();
    app.preferences.documents_folder = Some(std::path::PathBuf::from(
        "/home/someone/Documents/Correspondence/2026/Clients and Suppliers/Letters",
    ));
    app.preferences.style_folders = vec![
        std::path::PathBuf::from("/home/someone/Styles"),
        std::path::PathBuf::from(
            "/home/someone/Documents/Lotus/AmiPro/Old Machine Backup/Styles/Letterheads",
        ),
    ];
    send(&mut app, Message::Command(CommandId::Preferences));
    for (page, name) in [(Page::Folders, "folders"), (Page::View, "view")] {
        send(&mut app, Message::Preferences(PreferencesAction::Page(page)));
        for dark in [false, true] {
            let suffix = if dark { "dark" } else { "light" };
            write_window_ppm(
                &app,
                &directory.join(format!("prefs-{name}-{suffix}.ppm")),
                dark,
            );
        }
    }
}

/// Every modal dialog, light and dark, for the modal dialog review
/// (`docs/research/MODAL_DIALOG_REVIEW_2026-11.md`): the ones a command
/// opens and the questions and problems the app asks or reports. Ignored;
/// writes `dialog-*.ppm` to `AMIPRO_SCREENSHOT_DIR`. No portal is opened.
#[test]
#[ignore = "writes screenshots for review; set AMIPRO_SCREENSHOT_DIR"]
fn modal_dialog_screenshots() {
    use crate::page_surface_app_tests::write_window_ppm;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let write = |app: &AmiProApp, name: &str| {
        assert!(
            cosmic::Application::dialog(app).is_some(),
            "{name} shows a dialog"
        );
        for dark in [false, true] {
            let suffix = if dark { "dark" } else { "light" };
            write_window_ppm(
                app,
                &directory.join(format!("dialog-{name}-{suffix}.ppm")),
                dark,
            );
        }
    };
    for command in [
        CommandId::Font,
        CommandId::SpecialEffects,
        CommandId::SpecialCharacter,
        CommandId::NewFromStyleSheet,
        CommandId::Password,
        CommandId::GoToPage,
        CommandId::HeaderFooter,
        CommandId::PageNumber,
        CommandId::Frame,
        CommandId::Table,
        CommandId::InsertBullet,
        CommandId::OutlineNumbering,
        CommandId::CreateStyle,
        CommandId::StyleManagement,
        CommandId::UseAnotherStyleSheet,
        CommandId::SaveAsStyleSheet,
        CommandId::CustomizeToolbar,
        CommandId::Preferences,
    ] {
        let mut app = page_app(crate::page_surface::page_fixture());
        send(&mut app, Message::Command(command));
        if cosmic::Application::dialog(&app).is_none() {
            eprintln!("{command:?} opened no dialog here");
            continue;
        }
        write(&app, &format!("{command:?}"));
    }
    // The questions and problems, set as the app sets them.
    let fresh = || page_app(crate::page_surface::page_fixture());
    let mut app = fresh();
    app.pending_close = Some(0);
    write(&app, "DiscardChanges");
    let mut app = fresh();
    let style = *app.document.styles().keys().next().expect("a style");
    app.rename_style = Some((style, "Body Text".into()));
    write(&app, "RenameStyle");
    let mut app = fresh();
    app.pending_frame_delete = true;
    write(&app, "DeleteFrame");
    let mut app = fresh();
    app.revert_pending = true;
    write(&app, "RevertToSaved");
    let mut app = fresh();
    app.password_dialog = Some(password_dialog::PasswordDialog {
        purpose: password_dialog::Purpose::Open("/tmp/LETTER.SAM".into()),
        password: String::new(),
        verify: String::new(),
        error: None,
    });
    write(&app, "PasswordOpen");
    let mut app = fresh();
    app.fast_format_ask = Some(fast_format::FastFormatAsk {
        style: false,
        then: fast_format::Then::KeepOn,
    });
    write(&app, "FastFormatAsk");
    let mut app = fresh();
    app.open_problem = Some(open_problem(
        std::path::Path::new("/tmp/REPORT.SAM"),
        &"the file is not an Ami Pro document",
    ));
    write(&app, "OpenProblem");
    let mut app = fresh();
    app.save_problem = Some(SaveProblem {
        title: "Could not save REPORT.SAM".into(),
        body: "The folder is read-only.".into(),
        retry: SaveRetry::SaveAs,
    });
    write(&app, "SaveProblem");
    let mut app = fresh();
    app.sam_save = Some(PendingSamSave {
        path: "/tmp/REPORT.SAM".into(),
        losses: vec!["Colours beyond Ami Pro's palette".into()],
        saved_at: 0,
        replace: false,
    });
    write(&app, "SamLosses");
    let mut app = fresh();
    app.overwrite_confirmation.begin("/tmp/REPORT.SAM".into());
    write(&app, "Overwrite");
    let mut app = fresh();
    app.import_dialog = Some(import::OptionsDialog::new(
        import::Source::File("/tmp/notes.txt".into()),
        amipro_exchange::import::SourceFormat::Text,
        import::Purpose::Open,
        amipro_exchange::import::ImportOptions::default(),
    ));
    write(&app, "ImportOptions");
}
