//! What a member of a master document passes on to the next one: the
//! `[sequence]` a master run builds after it (`1660:054d`;
//! `docs/research/MASTER_NOTES.md` §4–5; VM MD2, MD3).

use amipro_document::contents::{Counter, Sequence};
use amipro_document::{Document, InlineObject, ParagraphStyle};

use crate::bullets::end_counters;
use crate::engine::DocumentLayout;

/// The document's styles in Ami Pro's list order: by function key, the
/// styles without a key last (as its style list keeps them, `11c8:1c7b`).
pub fn styles_in_order(document: &Document) -> Vec<&ParagraphStyle> {
    let mut styles: Vec<&ParagraphStyle> = document.styles().values().collect();
    styles.sort_by_key(|style| {
        let key = style.kept.function_key;
        (key == 0, key, style.id)
    });
    styles
}

/// The footnotes of the main text (Ami Pro's footnote list, `1350:050c`).
pub fn footnote_count(document: &Document) -> u32 {
    document
        .paragraphs()
        .iter()
        .flat_map(|paragraph| &paragraph.objects)
        .filter(|object| matches!(object, InlineObject::Footnote { .. }))
        .count() as u32
}

/// The sequence the member after `document` continues from, given
/// `document`'s own layout (laid out with its sequence) and the highest
/// value of each Seq field in it (`seq`):
/// - the page number its last page shows;
/// - its footnotes added to those carried;
/// - Seq counters: the carried ones, each raised to the member's highest
///   value, new names added (`1660:0913`, `1660:097e`);
/// - each style with bullets or numbers and its counter at the end
///   (`10c8:2b98`).
pub fn next_sequence(document: &Document, layout: &DocumentLayout, seq: &[Counter]) -> Sequence {
    let carried = document.contents().sequence.as_ref();
    let page_base = layout.pages.last().map_or_else(
        || carried.map_or(0, |sequence| sequence.page_base),
        |page| page.shown,
    );
    let footnotes = carried.map_or(0, |sequence| sequence.footnotes) + footnote_count(document);
    let mut seq_counters: Vec<Counter> = carried
        .map(|sequence| sequence.seq.clone())
        .unwrap_or_default();
    for counter in seq {
        match seq_counters
            .iter_mut()
            .find(|old| old.name.eq_ignore_ascii_case(&counter.name))
        {
            Some(old) => old.value = old.value.max(counter.value),
            None => seq_counters.push(counter.clone()),
        }
    }
    let styles = end_counters(
        document.paragraphs(),
        styles_in_order(document),
        document.styles(),
        carried,
    )
    .into_iter()
    .map(|(name, value)| Counter { value, name })
    .collect();
    Sequence {
        page_base,
        footnotes,
        reserved: carried.map_or(0, |sequence| sequence.reserved),
        seq: seq_counters,
        styles,
    }
}

/// [`next_sequence`] for a member laid out with page base `laid_base`
/// while it now continues from `document`'s sequence: a member's page
/// numbers move with its base, so its last page shows the same number less
/// the old base plus the new one (a member read once is not laid out again
/// for each new base).
pub fn next_sequence_rebased(
    document: &Document,
    layout: &DocumentLayout,
    laid_base: u32,
    seq: &[Counter],
) -> Sequence {
    let mut next = next_sequence(document, layout, seq);
    if layout.pages.last().is_some() {
        let base = document
            .contents()
            .sequence
            .as_ref()
            .map_or(0, |sequence| sequence.page_base);
        next.page_base = next.page_base - laid_base.min(next.page_base) + base;
    }
    next
}
