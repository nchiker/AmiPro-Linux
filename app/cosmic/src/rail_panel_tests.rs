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
