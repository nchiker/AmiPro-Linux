//! View › Draft Mode and View › Show Clean Screen (`UI_SPEC.md` §39, CM9).
//!
//! Draft lays the document out on pages as wide as the window, one column
//! with narrow margins, so the text reads at the window's width; the text
//! and its positions are the document's own, so editing is unchanged, and
//! nothing about the layout is saved. Clean Screen hides the menu, toolbar,
//! tabs, and status bar and fills the screen; Esc or its Return button
//! ends it.

use std::borrow::Cow;

use amipro_document::{Document, Twips};
use amipro_editing::{Command, LayoutSettings};

/// Draft's margins on each side.
pub const DRAFT_MARGIN: Twips = 360;
/// The narrowest page Draft lays out.
const NARROWEST: Twips = 2_880;

/// `document` with every page layout `width` twips wide, one column, and
/// Draft's margins; the same document when that changes nothing.
pub fn draft_document(document: &Document, width: Twips) -> Cow<'_, Document> {
    let mut draft = window_pages(document, width, DRAFT_MARGIN);
    // Ami Pro's Draft view shows no line numbers (VM, COMMANDS_NOTES §8.5).
    if document.line_numbering().is_on() {
        let mut numbering = document.line_numbering().clone();
        numbering.flags &= !(amipro_document::LineNumbering::ALL_LINES
            | amipro_document::LineNumbering::TEXT_LINES);
        draft.to_mut().set_line_numbering(numbering);
    }
    draft
}

/// `document` with every page layout `width` twips wide, one column, a
/// `left` margin and Draft's on the right: the pages Draft and Outline mode
/// lay the text out on.
pub fn window_pages(document: &Document, width: Twips, left: Twips) -> Cow<'_, Document> {
    let width = width.max(NARROWEST);
    let mut draft = Cow::Borrowed(document);
    let layouts: Vec<_> = document.layouts().iter().map(|layout| layout.id).collect();
    for layout in layouts {
        let Some(mut settings) = LayoutSettings::of(&draft, layout) else {
            continue;
        };
        let narrow = |geometry: &mut amipro_document::PageGeometry| {
            geometry.width = width;
            geometry.layout.margins.left = left;
            geometry.layout.margins.right = DRAFT_MARGIN;
            geometry.layout.columns = 1;
            geometry.layout.column_edges = None;
        };
        narrow(&mut settings.right);
        if let Some(left) = settings.left.as_mut() {
            narrow(left);
        }
        if let Ok(applied) =
            amipro_editing::apply(&draft, &Command::ModifyLayout { layout, settings })
        {
            draft = Cow::Owned(applied.document);
        }
    }
    draft
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_pages_are_the_given_width_with_one_column_and_the_same_text() {
        let document = Document::from_plain_text("One\nTwo");
        let draft = draft_document(&document, 9_000);
        let layout = draft.document_layout();
        assert_eq!(layout.right.width, 9_000);
        assert_eq!(layout.right.layout.columns, 1);
        assert_eq!(layout.right.layout.margins.left, DRAFT_MARGIN);
        assert_eq!(draft.plain_text(), document.plain_text());
        assert_eq!(
            draft_document(&document, 10).document_layout().right.width,
            NARROWEST
        );
    }
}
