//! Format › Ruler ▸ and Format › Page Layout ▸ (`UI_SPEC.md` §39), Ami
//! Pro's Page › Ruler ▸ Insert and Remove (`0x173`, `0x174`) and Page ›
//! Insert Page Layout ▸ Insert, Remove, Revert (`0x177`, `0x175`, `0x176`;
//! `COMMANDS_NOTES.md` §7) as menu commands over what the ruler and Page
//! Layout Properties already do; each one undo step.

use crate::{AmiProApp, BreakSwitch};

impl AmiProApp {
    /// Ruler ▸ Insert (true) or Remove: the selected paragraphs' own ruler.
    pub(crate) fn paragraph_ruler(&mut self, insert: bool) {
        let session = &self.sessions[self.active_session];
        let Some(model) = crate::ruler::Ruler::at(
            &session.document,
            session.pages.layout(),
            session.page_selection,
        ) else {
            return;
        };
        let command = if insert {
            model.insert_ruler()
        } else {
            model.remove_ruler()
        };
        let before = self.history.undo_len();
        self.run_ruler_command(&command);
        self.status = match (insert, self.history.undo_len() > before) {
            (true, true) => "The paragraph has its own ruler".into(),
            (false, true) => "The paragraph uses its style's ruler".into(),
            (true, false) => "The paragraph already has its own ruler".into(),
            (false, false) => "The paragraph has no ruler of its own".into(),
        };
    }

    /// Insert Page Layout ▸ Insert: a copy of the caret page's layout from
    /// the next page, then Page Layout Properties on it, as Ami Pro opens
    /// Modify Page Layout at once.
    pub(crate) fn insert_page_layout_command(&mut self) {
        let Some(layout) = self.caret_layout() else {
            return;
        };
        let known: Vec<_> = self
            .document
            .layouts()
            .iter()
            .map(|layout| layout.id)
            .collect();
        if !self.insert_page_break(BreakSwitch::CopyOf(layout)) {
            return;
        }
        let new = self
            .document
            .layouts()
            .iter()
            .map(|layout| layout.id)
            .find(|id| !known.contains(id));
        self.open_page_layout_properties();
        if let Some(new) = new {
            let caret = self.caret_layout();
            self.page_layout_properties = crate::page_layout_properties::PageLayoutProperties::new(
                &self.document,
                new,
                caret,
            );
        }
        self.status = "Inserted a page layout from the next page".into();
    }

    /// Insert Page Layout ▸ Remove: the inserted layout of the caret's page.
    pub(crate) fn remove_page_layout_command(&mut self) {
        let standard = self.document.document_layout().id;
        match self.caret_layout() {
            Some(layout) if layout != standard => self.remove_inserted_layout(layout),
            _ => self.status = "The caret's page has the document's own page layout".into(),
        }
    }

    /// Insert Page Layout ▸ Revert: the document's own layout again from
    /// the next page.
    pub(crate) fn revert_page_layout_command(&mut self) {
        let standard = self.document.document_layout().id;
        if self.caret_layout().is_none_or(|layout| layout == standard) {
            self.status = "The caret's page has the document's own page layout".into();
            return;
        }
        if self.insert_page_break(BreakSwitch::To(standard)) {
            self.status = "The next page goes back to the document's page layout".into();
        }
    }
}
