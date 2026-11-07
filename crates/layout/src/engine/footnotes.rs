//! Footnotes and endnotes as Ami Pro lays them out (decompile `1068:141e`,
//! segment `12f8`; VM fixtures RNF1–RNF6, RNF71S–RNF74S;
//! `docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`, "Layout"):
//!
//! - each page has one footnote area across the text area, growing up from
//!   the bottom margin and pulling up the bottom of every column;
//! - the area's first entry is a 100-twip separator, drawn at its top from
//!   the left plus the indent to the right margin or the custom length;
//! - a footnote's lines go into the area right after the main-text line
//!   that refers to it; when not even the separator and its first line fit
//!   below that line, the line moves to the next page (RNF72);
//! - a footnote whose lines do not all fit continues on the next page,
//!   after the "Continued from" message, with "Continued on" at the foot of
//!   the page it leaves (RNF5); footnotes left when the text ends go on
//!   pages of their own;
//! - with Make endnotes there are no areas: the footnotes follow the text
//!   on a new page, without a separator (RNF2);
//! - only a footnote's first paragraph shows its number: its place plus the
//!   starting number less one, counted from each page with Reset number on
//!   each page (`1308:0ebd`, `1108:4107`).

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use amipro_document::{FlowId, FootnoteOptions, InlineObject, ParagraphStyle};

use super::{
    Bounds, Container, Engine, LineBox, LineSpec, WordBox, first_line_bullet, indented,
    line_metrics, ruled_line, spaced_line,
};
use crate::breaking::next_line;
use crate::bullets::{BulletBox, ParagraphNumber, bullet_box};
use crate::tabs::{size_tabs, tab_stops};

/// The separator's place in the area (`12f8:06d0`).
pub(crate) const SEPARATOR: f32 = 100.0;

/// What a page's footnote area holds while the page is laid out.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct FootPage {
    /// The area's top: the bottom of every column (the text area's bottom
    /// while the page has no footnote).
    pub(crate) top: f32,
    /// The text area's bottom, where the area ends.
    pub(crate) bottom: f32,
    pub(crate) entries: Vec<FootEntry>,
    /// The ordinal of the first footnote referred to on the page, which
    /// Reset number on each page counts from.
    pub(crate) first_ordinal: Option<u32>,
}

/// A footnote's lines in a page's area.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FootEntry {
    pub(crate) flow: FlowId,
    pub(crate) ordinal: u32,
    pub(crate) lines: Range<usize>,
    /// Continued from the page before: "Continued from" goes first.
    pub(crate) continued: bool,
    /// Continues on the next page: "Continued on" goes after it.
    pub(crate) continues: bool,
}

/// A footnote, or the rest of one, waiting for the next page.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FootPending {
    pub(crate) flow: FlowId,
    pub(crate) ordinal: u32,
    pub(crate) from_line: usize,
}

/// The engine's footnote state.
#[derive(Clone, Debug, Default)]
pub(crate) struct Footnotes {
    /// Each footnote's place in the main text, from 1.
    pub(crate) ordinals: HashMap<FlowId, u32>,
    pub(crate) pages: Vec<FootPage>,
    pub(crate) pending: Vec<FootPending>,
    /// Each footnote's line heights at an area width, measured once.
    heights: HashMap<(FlowId, i32), Arc<Vec<f32>>>,
}

impl Footnotes {
    /// The main text's footnotes, by place.
    pub(crate) fn of(document: &amipro_document::Document) -> Option<Self> {
        let mut ordinals = HashMap::new();
        for paragraph in document.paragraphs() {
            for object in &paragraph.objects {
                if let InlineObject::Footnote { flow } = object {
                    let next = ordinals.len() as u32 + 1;
                    ordinals.entry(*flow).or_insert(next);
                }
            }
        }
        (!ordinals.is_empty()).then(|| Self {
            ordinals,
            ..Self::default()
        })
    }
}

impl Engine<'_> {
    fn options(&self) -> &FootnoteOptions {
        self.document.footnote_options()
    }

    /// Whether the footnotes go after the text.
    pub(crate) fn endnotes(&self) -> bool {
        self.footnotes.is_some() && self.options().endnotes()
    }

    /// A new page's area, and the rest of any footnote waiting for it.
    pub(crate) fn footnote_page(&mut self, page: usize) {
        let Some(footnotes) = self.footnotes.as_mut() else {
            return;
        };
        let bottom = self.pages[page]
            .columns
            .iter()
            .map(|column| column.bottom())
            .fold(0.0, f32::max);
        while footnotes.pages.len() <= page {
            footnotes.pages.push(FootPage::default());
        }
        footnotes.pages[page] = FootPage {
            top: bottom,
            bottom,
            entries: Vec::new(),
            first_ordinal: None,
        };
        if self.endnotes() {
            return;
        }
        // The rest of footnotes from the page before, as far as the page
        // holds them (the main text then starts below nothing of theirs).
        let pending = std::mem::take(&mut self.footnotes.as_mut().expect("above").pending);
        let top = self.pages[page].columns[0].y;
        for (index, waiting) in pending.iter().enumerate() {
            let heights = self.footnote_heights(waiting.flow, page);
            let outcome = self.place_footnote(
                page,
                waiting.flow,
                waiting.ordinal,
                waiting.from_line,
                &heights,
                top,
                index == 0,
            );
            let rest = match outcome {
                Placed::All => continue,
                Placed::Part(next) => Some(FootPending {
                    from_line: next,
                    ..waiting.clone()
                }),
                Placed::None => Some(waiting.clone()),
            };
            let footnotes = self.footnotes.as_mut().expect("above");
            footnotes.pending.extend(rest);
            footnotes
                .pending
                .extend(pending[index + 1..].iter().cloned());
            return;
        }
    }

    /// The column clipped to the page's footnote area.
    pub(crate) fn column_on(&self, page: usize, column: usize) -> Bounds {
        let mut bounds = self.pages[page].columns[column];
        if let Some(foot) = self
            .footnotes
            .as_ref()
            .and_then(|footnotes| footnotes.pages.get(page))
            && foot.top < bounds.bottom()
        {
            bounds.height = (foot.top - bounds.y).max(0.0);
        }
        bounds
    }

    /// The width footnotes are set in on `page`: the text area's.
    fn footnote_width(&self, page: usize) -> (f32, f32) {
        let columns = &self.pages[page].columns;
        let left = columns.first().map_or(0.0, |column| column.x);
        let right = columns
            .iter()
            .map(|column| column.right())
            .fold(left, f32::max);
        (left, right - left)
    }

    /// Each line's height in a footnote on `page`: its paragraph's space
    /// above on its first line and below on its last (`12f8:1321`).
    fn footnote_heights(&mut self, flow: FlowId, page: usize) -> Arc<Vec<f32>> {
        let width = self.footnote_width(page).1;
        let key = (flow, width.round() as i32);
        if let Some(heights) = self
            .footnotes
            .as_ref()
            .and_then(|footnotes| footnotes.heights.get(&key))
        {
            return heights.clone();
        }
        let mut heights = Vec::new();
        self.footnote_lines(flow, page, 0, usize::MAX, 0.0, None, &mut heights);
        let heights = Arc::new(heights);
        if let Some(footnotes) = self.footnotes.as_mut() {
            footnotes.heights.insert(key, heights.clone());
        }
        heights
    }

    /// Fits the footnotes `line` refers to below `bottom` (the line's
    /// bottom) on `page`. False when the line must move to the next page:
    /// the separator and the first footnote's first line do not fit.
    pub(crate) fn fit_footnotes(
        &mut self,
        page: usize,
        flows: &[FlowId],
        bottom: f32,
        force: bool,
    ) -> bool {
        if self.endnotes() || flows.is_empty() {
            return true;
        }
        let Some(footnotes) = self.footnotes.as_ref() else {
            return true;
        };
        let placed: Vec<FlowId> = footnotes
            .pages
            .iter()
            .flat_map(|foot| foot.entries.iter().map(|entry| entry.flow))
            .chain(footnotes.pending.iter().map(|waiting| waiting.flow))
            .collect();
        let new: Vec<(FlowId, u32)> = flows
            .iter()
            .filter(|flow| !placed.contains(flow))
            .filter_map(|flow| {
                footnotes
                    .ordinals
                    .get(flow)
                    .map(|ordinal| (*flow, *ordinal))
            })
            .collect();
        if new.is_empty() {
            return true;
        }
        let state = self.footnotes.clone();
        for (index, (flow, ordinal)) in new.iter().enumerate() {
            let heights = self.footnote_heights(*flow, page);
            // A footnote waiting for the next page keeps the ones after it
            // waiting too.
            let waiting = self
                .footnotes
                .as_ref()
                .is_some_and(|footnotes| !footnotes.pending.is_empty());
            let outcome = if waiting {
                Placed::None
            } else {
                self.place_footnote(
                    page,
                    *flow,
                    *ordinal,
                    0,
                    &heights,
                    bottom,
                    force && index == 0,
                )
            };
            let from_line = match outcome {
                Placed::All => continue,
                Placed::Part(next) => next,
                Placed::None if index == 0 && !force && !waiting => {
                    self.footnotes = state;
                    return false;
                }
                Placed::None => 0,
            };
            let footnotes = self.footnotes.as_mut().expect("checked");
            footnotes.pending.push(FootPending {
                flow: *flow,
                ordinal: *ordinal,
                from_line,
            });
        }
        true
    }

    /// Puts a footnote's lines from `from` into `page`'s area while each
    /// stays below `bottom`.
    #[allow(clippy::too_many_arguments)]
    fn place_footnote(
        &mut self,
        page: usize,
        flow: FlowId,
        ordinal: u32,
        from: usize,
        heights: &[f32],
        bottom: f32,
        force: bool,
    ) -> Placed {
        let message = self.message_height(page);
        let options = self.options().clone();
        let footnotes = self.footnotes.as_mut().expect("footnotes");
        let foot = &mut footnotes.pages[page];
        let continued = from > 0;
        let mut top = foot.top;
        if foot.entries.is_empty() {
            top -= SEPARATOR;
        }
        if continued && options.continued_from_message().is_some() {
            top -= message;
        }
        let mut end = from;
        while end < heights.len() {
            let more = end + 1 < heights.len();
            let reserve = if more && options.continued_on_message().is_some() {
                message
            } else {
                0.0
            };
            if top - heights[end] - reserve < bottom && !(force && end == from) {
                break;
            }
            top -= heights[end];
            end += 1;
        }
        if end == from {
            return Placed::None;
        }
        let continues = end < heights.len();
        if continues && options.continued_on_message().is_some() {
            top -= message;
        }
        foot.top = top;
        foot.first_ordinal = Some(
            foot.first_ordinal
                .map_or(ordinal, |first| first.min(ordinal)),
        );
        foot.entries.push(FootEntry {
            flow,
            ordinal,
            lines: from..end,
            continued,
            continues,
        });
        if continues {
            Placed::Part(end)
        } else {
            Placed::All
        }
    }

    /// A continued message's height: one line of the Footnote style.
    fn message_height(&mut self, _page: usize) -> f32 {
        let style = self.footnote_style();
        let size = style.font_size_points * 20.0;
        (size * 1.15).round()
    }

    fn footnote_style(&self) -> ParagraphStyle {
        self.document
            .styles()
            .values()
            .find(|style| style.name.eq_ignore_ascii_case("Footnote"))
            .cloned()
            .unwrap_or_else(|| self.style(self.document.body_text_style()))
    }

    /// The number footnote `ordinal` shows on `page`.
    pub(crate) fn footnote_number(&self, ordinal: u32, page: usize) -> u32 {
        let options = self.options();
        let mut number = ordinal;
        if options.reset_each_page() && !options.endnotes() {
            let first = self
                .footnotes
                .as_ref()
                .and_then(|footnotes| footnotes.pages.get(page))
                .and_then(|foot| foot.first_ordinal)
                .unwrap_or(ordinal);
            number = ordinal.saturating_sub(first.min(ordinal)) + 1;
        }
        // A member of a master document numbers its footnotes after the
        // members before, over its own starting number (`1058:0000`; VM MD3).
        let start = match self.document.contents().sequence.as_ref() {
            Some(sequence) => sequence.footnotes + 1,
            None => u32::from(options.start),
        };
        number + start.saturating_sub(1)
    }

    /// The mark a footnote's number makes with the Footnote style's
    /// numbering: its text and width.
    pub(crate) fn footnote_mark(&mut self, flow: FlowId, number: u32) -> Option<BulletBox> {
        let paragraph = self.document.flow(flow)?.paragraphs.first()?.clone();
        let (_, style) = self.shaped(&paragraph);
        self.mark_in(&style, number)
    }

    /// A number's mark in `style`'s numbering (the Footnote style's for a
    /// table footnote).
    pub(crate) fn mark_in(&mut self, style: &ParagraphStyle, number: u32) -> Option<BulletBox> {
        bullet_box(
            &style.bullets,
            ParagraphNumber::own(number),
            style,
            self.shaper,
        )
    }

    /// A table footnote's mark: its number in the Footnote style.
    pub(crate) fn table_footnote_mark(&mut self, number: u32) -> Option<BulletBox> {
        let style = self.footnote_style();
        self.mark_in(&style, number)
    }

    /// The text of a mark.
    pub(crate) fn mark_text(mark: &BulletBox) -> String {
        mark.pieces
            .iter()
            .filter_map(|piece| match piece {
                crate::bullets::BulletPiece::Text { text, .. } => Some(text.as_str()),
                crate::bullets::BulletPiece::Shape { .. } => None,
            })
            .collect()
    }

    /// Sets the footnotes of every page into their areas: the separator,
    /// each entry's lines and messages, from the area's top.
    pub(crate) fn footnote_areas(&mut self) {
        let Some(footnotes) = self.footnotes.clone() else {
            return;
        };
        if self.endnotes() {
            return;
        }
        let options = self.options().clone();
        for (page, foot) in footnotes.pages.iter().enumerate() {
            if foot.entries.is_empty() || page >= self.pages.len() {
                continue;
            }
            let (left, width) = self.footnote_width(page);
            let length = if options.has(FootnoteOptions::CUSTOM_LENGTH) {
                (options.length as f32).min(width - options.indent as f32)
            } else {
                width - options.indent as f32
            };
            self.page_mut(page).footnote_separator = Some(Bounds {
                x: left + options.indent as f32,
                y: foot.top,
                width: length.max(0.0),
                height: 0.0,
            });
            let mut y = foot.top + SEPARATOR;
            for entry in &foot.entries {
                if entry.continued
                    && let Some(text) = options.continued_from_message()
                {
                    let number = options
                        .has(FootnoteOptions::REPEAT_NUMBER)
                        .then(|| self.footnote_number(entry.ordinal, page));
                    y = self.footnote_message(
                        page,
                        entry.flow,
                        text,
                        options.has(FootnoteOptions::CONTINUED_FROM_LEFT),
                        number,
                        y,
                    );
                }
                let number = self.footnote_number(entry.ordinal, page);
                let mut heights = Vec::new();
                y = self.footnote_lines(
                    entry.flow,
                    page,
                    entry.lines.start,
                    entry.lines.end,
                    y,
                    Some(number),
                    &mut heights,
                );
                if entry.continues
                    && let Some(text) = options.continued_on_message()
                {
                    y = self.footnote_message(
                        page,
                        entry.flow,
                        text,
                        options.has(FootnoteOptions::CONTINUED_ON_LEFT),
                        None,
                        y,
                    );
                }
            }
        }
    }

    /// A continued message: one line in the Footnote style, at the left or
    /// right of the area, with the footnote's number when it repeats.
    fn footnote_message(
        &mut self,
        page: usize,
        flow: FlowId,
        text: &str,
        left: bool,
        number: Option<u32>,
        y: f32,
    ) -> f32 {
        let height = self.message_height(page);
        let (x, width) = self.footnote_width(page);
        let style = self.footnote_style();
        let text_width = self.shaper.measure(text, &style);
        let mark = number.and_then(|number| self.footnote_mark(flow, number));
        let paragraph = Arc::new(amipro_document::Paragraph {
            id: amipro_document::ParagraphId::from_raw(u64::MAX),
            text: text.to_owned(),
            style: style.id,
            format: amipro_document::ParagraphFormat::default(),
            runs: Vec::new(),
            objects: Vec::new(),
            marks: Vec::new(),
            end_format: amipro_document::CharacterFormat::default(),
            cache: None,
        });
        let shaped = self.cache.shaped(self.shaper, &paragraph, &style);
        let line_x = if left { x } else { x + width - text_width };
        // The message is not in the document's text: its line carries it
        // as shown text, in the Footnote style.
        self.page_mut(page).lines.push(LineBox {
            container: Container::Footnote(flow),
            flow,
            paragraph: paragraph.id,
            paragraph_index: usize::MAX,
            bytes: 0..0,
            x: line_x,
            top: y,
            baseline: y + shaped.ascent,
            height,
            slot_width: text_width,
            width: text_width,
            words: vec![WordBox {
                red: false,
                bytes: 0..0,
                x: line_x,
                width: text_width,
                object: None,
                text: Some(text.to_owned()),
                tab: None,
            }],
            hyphen: false,
            spaced: false,
            rules: Vec::new(),
            bullet: mark.map(|mark| BulletBox { x, ..mark }),
        });
        y + height
    }

    /// Sets lines `from..to` of a footnote at `y` on `page` (measuring
    /// only, into `heights`, when `number` is `None`), its first line with
    /// the number; returns the y after them.
    #[allow(clippy::too_many_arguments)]
    fn footnote_lines(
        &mut self,
        flow: FlowId,
        page: usize,
        from: usize,
        to: usize,
        mut y: f32,
        number: Option<u32>,
        heights: &mut Vec<f32>,
    ) -> f32 {
        let Some(paragraphs) = self.document.flow(flow).map(|flow| flow.paragraphs.clone()) else {
            return y;
        };
        let (left, width) = self.footnote_width(page);
        let page_number = self.pages[page].number;
        let area = Bounds {
            x: left,
            y: 0.0,
            width,
            height: 0.0,
        };
        let count = paragraphs.len();
        let mut line_index = 0;
        for (index, paragraph) in paragraphs.iter().enumerate() {
            let (shaped, style) = self.shaped(paragraph);
            let mut words = self.words_for(paragraph, &shaped, &style, area, page_number);
            let stops = tab_stops(&style, style.indents, &[], width);
            // Only the first paragraph shows the number (`1108:4107`).
            let bullet = if index == 0 {
                self.footnote_mark(flow, number.unwrap_or(1))
            } else {
                None
            };
            let mut start = 0;
            loop {
                let slot = indented(area, style.indents, start == 0);
                let (line_bullet, shift) =
                    first_line_bullet(bullet.as_ref(), start, slot.x, style.indents);
                let line_width = (slot.width - shift).max(1.0);
                let x = slot.x + shift;
                size_tabs(
                    &mut words,
                    start,
                    x - left,
                    line_width,
                    &stops,
                    &paragraph.text,
                );
                let zone = self.hyphenation_zone(&shaped);
                let line = next_line(&words, start, line_width, shaped.hyphen_width, zone, |_| {
                    false
                });
                let first = start == 0;
                let last = line.end >= words.len();
                let (ascent, descent) = line_metrics(&words[start..line.end], &shaped);
                let (ascent, descent) = ruled_line(
                    &style.rules,
                    first,
                    last,
                    spaced_line(&style, &words[start..line.end], ascent, descent),
                );
                // Each paragraph's space above with its first line and below
                // with its last, the footnote's own included (`12f8:1321`;
                // Ami Pro's display of a user document: footnotes apart by
                // their style's space below).
                let above = if first && index > 0 {
                    style.spacing.above as f32
                } else {
                    0.0
                };
                let below = if last {
                    style.spacing.below as f32
                } else {
                    0.0
                };
                let _ = count;
                if number.is_none() {
                    heights.push(above + ascent + descent + below);
                } else if (from..to).contains(&line_index) {
                    self.push_line(
                        page,
                        LineSpec {
                            shown: None,
                            container: Container::Footnote(flow),
                            flow,
                            paragraph,
                            paragraph_index: index,
                            words: &words,
                            range: start..line.end,
                            x,
                            top: y + above,
                            ascent,
                            height: ascent + descent,
                            slot_width: line_width,
                            width: line.width,
                            hyphen: line.hyphen,
                            page_number,
                            alignment: style.alignment,
                            first,
                            last,
                            rules: &style.rules,
                            column: Bounds {
                                x: left,
                                y,
                                width,
                                height: ascent + descent,
                            },
                            bullet: line_bullet,
                        },
                    );
                    y += above + ascent + descent + below;
                }
                line_index += 1;
                if last {
                    break;
                }
                start = line.end;
            }
        }
        y
    }

    /// The footnotes left when the text ends go on pages of their own; with
    /// Make endnotes every footnote follows the text from a new page.
    pub(crate) fn footnotes_after_text(&mut self) {
        let Some(footnotes) = self.footnotes.clone() else {
            return;
        };
        let layout = self
            .pages
            .last()
            .map_or(self.document.start_layout().id, |page| page.layout);
        if self.endnotes() {
            let mut order: Vec<(u32, FlowId)> = footnotes
                .ordinals
                .iter()
                .map(|(flow, ordinal)| (*ordinal, *flow))
                .collect();
            order.sort_unstable();
            let mut page = self.new_page(layout);
            let mut y = self.pages[page].columns[0].y;
            for (ordinal, flow) in order {
                let number = self.footnote_number(ordinal, page);
                let mut heights = Vec::new();
                self.footnote_lines(flow, page, 0, usize::MAX, 0.0, None, &mut heights);
                let bottom = self.pages[page].columns[0].bottom();
                for (line, height) in heights.iter().enumerate() {
                    if y + height > bottom && y > self.pages[page].columns[0].y {
                        page = self.new_page(layout);
                        y = self.pages[page].columns[0].y;
                    }
                    let mut scratch = Vec::new();
                    self.footnote_lines(flow, page, line, line + 1, y, Some(number), &mut scratch);
                    y += height;
                }
            }
            return;
        }
        let mut guard = 0;
        while self
            .footnotes
            .as_ref()
            .is_some_and(|footnotes| !footnotes.pending.is_empty())
            && guard < 1_000
        {
            guard += 1;
            self.new_page(layout);
        }
    }
}

/// How much of a footnote went onto a page.
enum Placed {
    All,
    /// Up to this line; the rest continues.
    Part(usize),
    None,
}
