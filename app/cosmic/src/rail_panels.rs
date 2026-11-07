//! The rail shows one panel at a time (user, 2026-11-07). This is the one
//! place that says which panels there are, in which order the view picks
//! among the states that are set ([`AmiProApp::rail_panel`]), and how they
//! give way to each other ([`DocumentSession::close_rail_panels`]).
//!
//! The rule:
//!
//! - Every opener (a menu command, a ruler action, a shortcut) calls
//!   `close_rail_panels` first and then sets its own state, so the panel
//!   opened last is the one that shows. Closing a panel (its X, Esc) clears
//!   its state, and the rail goes back to the Styles list.
//! - Opening a panel also deselects the frames, so that Frame Properties,
//!   which follows the selection by itself, does not hide it. Selecting a
//!   frame while another panel is open shows Frame Properties again (the
//!   selection is the newer choice); the other panel stays set and shows
//!   when the frames are deselected.
//! - Draw mode's properties belong to the mode, not to a panel: they win
//!   over everything while it lasts.
//! - The Envelope panel keeps a shown Merge panel behind it (Close shows
//!   Merge again); it is the one panel that does not close another.
//!
//! A new panel is added to [`RailPanel`] (the compiler then asks for it in
//! `rail_panel`, `is_open`, and the view), to `close_rail_panels`, and to
//! `rail_panel_tests`, whose sweep fails when an opener leaves another
//! panel's state set.

use crate::{AmiProApp, DocumentSession};

/// What the rail shows, in the order the view picks among the states that
/// are set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RailPanel {
    /// Draw mode's properties.
    Drawing,
    /// Frame Properties, while frames are selected.
    Frame,
    StyleProperties,
    PageLayout,
    Paragraph,
    Table,
    Fields,
    References,
    Contents,
    Master,
    Envelope,
    Merge,
    Glossary,
    Proofing,
    DocumentProperties,
    ImportFolder,
    ExportFolder,
    ImportReport,
    Print,
    PasteSpecial,
    Links,
    Outline,
    Sort,
    /// Tools › Charting… (DO.8).
    Chart,
    /// Tools › Equations… (DO.9): the template palette; the source is typed
    /// in a card on the page.
    Equation,
    /// The default: the Styles list.
    Styles,
}

impl RailPanel {
    /// Every panel but the default, in the view's order.
    #[cfg(test)]
    pub(crate) const ALL: [RailPanel; 25] = [
        RailPanel::Drawing,
        RailPanel::Frame,
        RailPanel::StyleProperties,
        RailPanel::PageLayout,
        RailPanel::Paragraph,
        RailPanel::Table,
        RailPanel::Fields,
        RailPanel::References,
        RailPanel::Contents,
        RailPanel::Master,
        RailPanel::Envelope,
        RailPanel::Merge,
        RailPanel::Glossary,
        RailPanel::Proofing,
        RailPanel::DocumentProperties,
        RailPanel::ImportFolder,
        RailPanel::ExportFolder,
        RailPanel::ImportReport,
        RailPanel::Print,
        RailPanel::PasteSpecial,
        RailPanel::Links,
        RailPanel::Outline,
        RailPanel::Sort,
        RailPanel::Chart,
        RailPanel::Equation,
    ];
}

impl DocumentSession {
    /// Whether `panel`'s state is set (Draw mode and the frame selection
    /// are not panels' own states, but count here).
    pub(crate) fn rail_state_set(&self, panel: RailPanel) -> bool {
        match panel {
            RailPanel::Drawing => self.draw.is_some(),
            RailPanel::Frame => self.frame_selection.is_some(),
            RailPanel::StyleProperties => self.style_properties.is_some(),
            RailPanel::PageLayout => self.page_layout_properties.is_some(),
            RailPanel::Paragraph => self.paragraph_properties.is_some(),
            RailPanel::Table => self.table_properties.is_some(),
            RailPanel::Fields => self.fields_panel.is_some(),
            RailPanel::References => self.references.is_some(),
            RailPanel::Contents => self.contents.is_some(),
            RailPanel::Master => self.master_panel.is_some(),
            RailPanel::Envelope => self.envelope_panel.is_some(),
            RailPanel::Merge => self.merge_panel.as_ref().is_some_and(|panel| panel.shown),
            RailPanel::Glossary => self.glossary.is_some(),
            RailPanel::Proofing => self.proofing.is_some(),
            RailPanel::DocumentProperties => self.document_properties.is_some(),
            RailPanel::ImportFolder => self.import_folder.is_some(),
            RailPanel::ExportFolder => self.export_folder.is_some(),
            RailPanel::ImportReport => self.import_report_shown && self.import_report.is_some(),
            RailPanel::Print => self.print_panel.is_some(),
            RailPanel::PasteSpecial => self.paste_special.is_some(),
            RailPanel::Links => self.links_panel.is_some(),
            RailPanel::Outline => self.outline_navigator.is_some(),
            RailPanel::Sort => self.sort_panel.is_some(),
            RailPanel::Chart => self.chart_panel.is_some(),
            RailPanel::Equation => self.equation_panel.is_some(),
            RailPanel::Styles => false,
        }
    }

    /// Every rail panel closed and the frames deselected: the first thing
    /// each opener does, before it sets its own state. The Merge panel's
    /// data is kept, hidden; the Sort panel's settings are kept for the
    /// next sort.
    pub(crate) fn close_rail_panels(&mut self) {
        self.style_properties = None;
        self.page_layout_properties = None;
        self.paragraph_properties = None;
        self.table_properties = None;
        self.fields_panel = None;
        self.references = None;
        self.contents = None;
        self.master_panel = None;
        self.envelope_panel = None;
        if let Some(panel) = self.merge_panel.as_mut() {
            panel.shown = false;
        }
        self.glossary = None;
        self.proofing = None;
        self.document_properties = None;
        self.import_folder = None;
        self.export_folder = None;
        self.import_report_shown = false;
        self.print_panel = None;
        self.paste_special = None;
        self.links_panel = None;
        self.outline_navigator = None;
        self.chart_panel = None;
        self.equation_panel = None;
        if let Some(panel) = self.sort_panel.take() {
            self.sort_form = panel.form;
        }
        self.frame_selection = None;
    }
}

impl AmiProApp {
    /// The panel the rail shows now: the first state set, in
    /// [`RailPanel`]'s order. The view builds exactly this one.
    pub(crate) fn rail_panel(&self) -> RailPanel {
        if !self.page_surface {
            return RailPanel::Styles;
        }
        if self.drawing_open() {
            return RailPanel::Drawing;
        }
        const CHAIN: [RailPanel; 24] = [
            RailPanel::Frame,
            RailPanel::StyleProperties,
            RailPanel::PageLayout,
            RailPanel::Paragraph,
            RailPanel::Table,
            RailPanel::Fields,
            RailPanel::References,
            RailPanel::Contents,
            RailPanel::Master,
            RailPanel::Envelope,
            RailPanel::Merge,
            RailPanel::Glossary,
            RailPanel::Proofing,
            RailPanel::DocumentProperties,
            RailPanel::ImportFolder,
            RailPanel::ExportFolder,
            RailPanel::ImportReport,
            RailPanel::Print,
            RailPanel::PasteSpecial,
            RailPanel::Links,
            RailPanel::Outline,
            RailPanel::Sort,
            RailPanel::Chart,
            RailPanel::Equation,
        ];
        CHAIN
            .into_iter()
            .find(|panel| self.rail_state_set(*panel))
            .unwrap_or(RailPanel::Styles)
    }
}
