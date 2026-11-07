//! Page › Line Numbering drawn as Ami Pro 3.1 draws it (`1178:0000`, the
//! page counter `1068:353a`; VM fixtures LN2–LN20,
//! `docs/research/COMMANDS_NOTES.md` §8.5).
//!
//! The numbers are worked out from a finished layout, so they never change
//! where text goes: each page's numbers stand in its left margin, the
//! number and one space ending at the margin of the page's own layout.
//!
//! - **Number text lines** counts the main text's lines that show text, in
//!   the page's first column, in flow order: not empty paragraphs, table
//!   cells, frames, headers, footers, or footnotes (LN9, LN20). A line beside
//!   a frame counts wherever it stands, its number in the left margin like
//!   the others (Ami Pro draws those of both sides at the same heights).
//!   The numbers are Body Text's font, on the line's baseline (LN14, LN20).
//! - **Number all lines** numbers slots from the top margin to the bottom
//!   margin at the "Based on" style's line pitch (its line height and line
//!   spacing, without the space above and below), whatever the page holds,
//!   in that style's font (LN2, LN10, LN13).
//!
//! The count carries over from page to page unless Reset each page; which
//! counts show follows Every line, other, fifth, or N
//! ([`amipro_document::LineNumbering::shows`]).

use amipro_document::{CharacterFormat, Document, LineNumberingMode, ParagraphStyle, StyleId};

use crate::engine::{Container, DocumentLayout, PageBox, hyphenation_language, spaced_line};
use crate::shaper::{ShapeRequest, Shaper};

/// One line number on a page, in twips from the page's top-left corner.
#[derive(Clone, Debug, PartialEq)]
pub struct LineNumberBox {
    /// The digits (Ami Pro draws them with one space after).
    pub text: String,
    /// The left edge of the digits.
    pub x: f32,
    pub baseline: f32,
    /// The style whose font the number is set in.
    pub style: StyleId,
}

/// Each page's line numbers, in page order; none at all when numbering is
/// off.
pub fn line_numbers(
    document: &Document,
    layout: &DocumentLayout,
    shaper: &mut dyn Shaper,
) -> Vec<Vec<LineNumberBox>> {
    let numbering = document.line_numbering();
    let mode = numbering.mode();
    if mode == LineNumberingMode::None || document.styles().is_empty() {
        return Vec::new();
    }
    let body = document.body_text_style();
    let style_id = match mode {
        LineNumberingMode::AllLines => document
            .styles()
            .values()
            .find(|style| style.name == numbering.style)
            .map_or(body, |style| style.id),
        _ => body,
    };
    let style = document.styles()[&style_id].clone();
    let mut numbers = Numberer {
        shaper,
        style: &style,
        style_id,
        document,
        widths: std::collections::HashMap::new(),
    };
    let pitch = match mode {
        LineNumberingMode::AllLines => Some(numbers.pitch()),
        _ => None,
    };
    let mut count = 0u32;
    layout
        .pages
        .iter()
        .map(|page| {
            if numbering.reset_each_page() {
                count = 0;
            }
            let Some(text_area) = page.columns.first().copied() else {
                return Vec::new();
            };
            let margin = text_area.x;
            let baselines: Vec<f32> = match pitch {
                Some((height, ascent)) => {
                    let slots = (text_area.height / height + 1e-3).floor().max(0.0) as usize;
                    (0..slots)
                        .map(|slot| text_area.y + slot as f32 * height + ascent)
                        .collect()
                }
                None => counted_lines(document, page, text_area.right()).collect(),
            };
            baselines
                .into_iter()
                .filter_map(|baseline| {
                    count += 1;
                    numbering
                        .shows(count)
                        .then(|| numbers.number(count, margin, baseline))
                })
                .collect()
        })
        .collect()
}

/// The baselines of the page's lines that Number text lines counts: main
/// text lines in the first column that show a character other than a blank
/// or an object's mark.
fn counted_lines<'a>(
    document: &'a Document,
    page: &'a PageBox,
    first_column_right: f32,
) -> impl Iterator<Item = f32> + 'a {
    page.lines
        .iter()
        .filter(move |line| {
            if line.container != Container::Main || line.x >= first_column_right - 0.5 {
                return false;
            }
            let Some(paragraph) = document
                .flow(line.flow)
                .and_then(|flow| flow.paragraphs.get(line.paragraph_index))
            else {
                return false;
            };
            line.words.iter().any(|word| {
                word.text
                    .as_ref()
                    .is_some_and(|text| !text.trim().is_empty())
                    || paragraph.text.get(word.bytes.clone()).is_some_and(|text| {
                        text.chars().any(|character| {
                            !character.is_whitespace()
                                && character != amipro_document::OBJECT_REPLACEMENT_CHARACTER
                        })
                    })
            })
        })
        .map(|line| line.baseline)
}

struct Numberer<'a, 'b> {
    shaper: &'b mut dyn Shaper,
    style: &'a ParagraphStyle,
    style_id: StyleId,
    document: &'a Document,
    /// The widths of the digits and the space, measured once: figures are
    /// set at one width in the fonts Ami Pro's documents use.
    widths: std::collections::HashMap<char, f32>,
}

impl Numberer<'_, '_> {
    /// The style's line height and baseline from the line's top, as the
    /// engine sets an empty line in it.
    fn pitch(&mut self) -> (f32, f32) {
        let language = hyphenation_language(self.document);
        let shaped = self.shaper.shape(ShapeRequest {
            text: "",
            style: self.style,
            runs: &[],
            end_format: &CharacterFormat::default(),
            line_heights: self.document.line_heights(),
            language: &language,
        });
        let (ascent, descent) = spaced_line(self.style, &[], shaped.ascent, shaped.descent);
        let height = (ascent + descent).max(1.0);
        (height, ascent)
    }

    fn number(&mut self, count: u32, margin: f32, baseline: f32) -> LineNumberBox {
        let text = count.to_string();
        let width: f32 = format!("{text} ")
            .chars()
            .map(|character| self.width(character))
            .sum();
        LineNumberBox {
            x: margin - width,
            text,
            baseline,
            style: self.style_id,
        }
    }

    fn width(&mut self, character: char) -> f32 {
        if let Some(width) = self.widths.get(&character) {
            return *width;
        }
        let width = self
            .shaper
            .measure_text(
                character.encode_utf8(&mut [0; 4]),
                self.style,
                &CharacterFormat::default(),
                self.document.line_heights(),
            )
            .width;
        self.widths.insert(character, width);
        width
    }
}
