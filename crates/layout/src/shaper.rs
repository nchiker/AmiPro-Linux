//! The shaping boundary: a paragraph's words in logical order, each ending at
//! a line-break opportunity (`docs/research/LINE_BREAKING_SPIKE.md`).

use std::ops::Range;

use amipro_document::{
    CharacterFormat, CharacterRun, FaceKey, LineHeights, OBJECT_REPLACEMENT_CHARACTER,
    ParagraphStyle, TabLeader,
};

use crate::display::display_spans;

pub const HARD_RETURN: char = '\u{2028}';
pub const SOFT_HYPHEN: char = '\u{AD}';

/// What to shape: one paragraph's text in its style, with direct formatting
/// and the format at its end, and the document's printer line heights,
/// which a font's lines take before its own metrics.
#[derive(Clone, Copy, Debug)]
pub struct ShapeRequest<'a> {
    pub text: &'a str,
    pub style: &'a ParagraphStyle,
    pub runs: &'a [CharacterRun],
    /// The paragraph's end format, whose font sets an empty line.
    pub end_format: &'a CharacterFormat,
    pub line_heights: &'a LineHeights,
    /// The document's language as an ISO 639-1 code (`en`, `fr`): its
    /// hyphenation patterns (`UI_SPEC.md` §35, SP6).
    pub language: &'a str,
}

/// Shapes paragraphs into words. Implementations: COSMIC Text in the app,
/// [`FixedShaper`] in tests.
pub trait Shaper {
    /// The paragraph's words in logical order. Every byte of `text` belongs
    /// to exactly one word, each U+FFFC is a word of its own, and a word ends
    /// at each break opportunity (after spaces, U+2028, and U+00AD).
    fn shape(&mut self, request: ShapeRequest<'_>) -> ShapedParagraph;

    /// The width of `text` set in `style`, in twips (bullet text).
    fn measure(&mut self, text: &str, style: &ParagraphStyle) -> f32;

    /// The width, ascent, and descent of `text` set in `format` over
    /// `style`, in twips: a page-number field, which takes the text
    /// formatting where it stands (VM fixture PF1). By default the width in
    /// the format's face and size, and no height of its own.
    fn measure_text(
        &mut self,
        text: &str,
        style: &ParagraphStyle,
        format: &CharacterFormat,
        _line_heights: &LineHeights,
    ) -> TextExtent {
        let mut styled = style.clone();
        if let Some(family) = &format.font_family {
            styled.font_family.clone_from(family);
        }
        styled.font_size_points = format.font_size_points.unwrap_or(style.font_size_points);
        styled.bold = format.bold.unwrap_or(style.bold);
        styled.italic = format.italic.unwrap_or(style.italic);
        TextExtent {
            width: self.measure(text, &styled),
            ascent: 0.0,
            descent: 0.0,
        }
    }

    /// Where a run of letters may be hyphenated, as byte offsets into it
    /// (`crate::hyphenation`). None by default.
    fn hyphenate(&mut self, _letters: &str) -> Vec<usize> {
        Vec::new()
    }
}

/// The extent of a piece of text set on its own, in twips.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextExtent {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapedParagraph {
    pub words: Vec<ShapedWord>,
    /// The width of the hyphen shown when a line ends at U+00AD.
    pub hyphen_width: f32,
    /// The paragraph font's average character width, the unit of the
    /// hyphenation zone.
    pub average_width: f32,
    /// Ascent and descent of a line without text: the font of the
    /// paragraph's end format over its style (decompile `10d0:0592`).
    pub ascent: f32,
    pub descent: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShapedWord {
    pub bytes: Range<usize>,
    /// Advance width in twips.
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    /// Whitespace only: it hangs at a line end.
    pub blank: bool,
    /// Bidi embedding level (even left to right, odd right to left).
    pub level: u8,
    /// Ends with U+2028: the line must end after it.
    pub forced_break: bool,
    /// Ends with U+00AD: a line that ends here shows a hyphen.
    pub soft_hyphen: bool,
    /// Ends at a point the hyphenator found inside a word: a line may end
    /// here with a hyphen where it has the hyphenation zone left.
    pub hyphenation: bool,
    /// The index of the inline object (the k-th U+FFFC) this word is.
    pub object: Option<usize>,
    /// A tab, a word of its own: the engine sizes it from the tab stops
    /// and records the stop's leader here.
    pub tab: Option<TabLeader>,
}

impl ShapedParagraph {
    /// Sets the flags that follow from the text: forced breaks, soft hyphens
    /// (none in text marked No hyphenation, which is never split), and
    /// object indexes. Shapers call this after measuring.
    pub fn classify(&mut self, request: &ShapeRequest<'_>) {
        let text = request.text;
        let mut objects_before = 0;
        let mut offset = 0;
        for word in &mut self.words {
            objects_before += text[offset..word.bytes.start]
                .matches(OBJECT_REPLACEMENT_CHARACTER)
                .count();
            offset = word.bytes.start;
            let slice = &text[word.bytes.clone()];
            word.forced_break = slice.ends_with(HARD_RETURN);
            word.soft_hyphen = slice.ends_with(SOFT_HYPHEN)
                && !request
                    .runs
                    .iter()
                    .find(|run| run.start < word.bytes.end && word.bytes.end <= run.end)
                    .is_some_and(|run| run.format.attributes(request.style).no_hyphenation);
            word.object =
                (slice == OBJECT_REPLACEMENT_CHARACTER.to_string()).then_some(objects_before);
            word.tab = (slice == "\t").then_some(TabLeader::None);
        }
    }
}

/// A deterministic shaper for tests: every character advances half the size
/// it is drawn at (its display span's), scaled by Text Tightness (the advance
/// is the font's average width), ascent is 0.8 and descent 0.2 of its
/// full size (or of the font's printer line height when the request has
/// one), an empty line is set in the end format's font, and breaks follow
/// spaces, hyphens (`-`, U+2010), U+00AD, U+2028, and both sides of U+FFFC
/// and tabs.
#[derive(Clone, Copy, Debug, Default)]
pub struct FixedShaper;

impl FixedShaper {
    /// The line height of text in `family` at `size` points: the printer's
    /// for the font, else the size.
    fn height(
        line_heights: &LineHeights,
        family: &str,
        size: f32,
        bold: bool,
        italic: bool,
    ) -> f32 {
        line_heights
            .get(&FaceKey::new(family, size, bold, italic))
            .map_or(size * 20.0, |height| height as f32)
    }
}

impl Shaper for FixedShaper {
    fn shape(&mut self, request: ShapeRequest<'_>) -> ShapedParagraph {
        let text = request.text;
        let style = request.style;
        let size = style.font_size_points * 20.0;
        let end = request.end_format;
        let height = Self::height(
            request.line_heights,
            end.font_family.as_deref().unwrap_or(&style.font_family),
            end.font_size_points.unwrap_or(style.font_size_points),
            end.bold.unwrap_or(style.bold),
            end.italic.unwrap_or(style.italic),
        );
        let points = crate::hyphenation::hyphenation_points(&request, &mut |letters| {
            FixedShaper.hyphenate(letters)
        });
        let spans = display_spans(text, style, request.runs);
        let mut span_index = 0;
        let mut words: Vec<ShapedWord> = Vec::new();
        let mut current: Option<ShapedWord> = None;
        let close = |current: &mut Option<ShapedWord>, words: &mut Vec<ShapedWord>| {
            if let Some(word) = current.take() {
                words.push(word);
            }
        };
        for (byte, character) in text.char_indices() {
            let end = byte + character.len_utf8();
            while spans[span_index].bytes.end <= byte {
                span_index += 1;
            }
            let span = &spans[span_index];
            let size_here = span.size * 20.0;
            let height_here = Self::height(
                request.line_heights,
                span.family,
                span.full_size,
                span.bold,
                span.italic,
            );
            let blank = character == ' ';
            // Text Tightness adds its share of the average width (the
            // advance here) to every character (VM fixture TT1).
            let advance = if matches!(character, SOFT_HYPHEN | HARD_RETURN) {
                0.0
            } else {
                size_here / 2.0 * f32::from(style.spacing.tightness) / 100.0
            };
            // Objects and tabs are words of their own, sized by the engine.
            if character == OBJECT_REPLACEMENT_CHARACTER || character == '\t' {
                close(&mut current, &mut words);
                words.push(ShapedWord {
                    bytes: byte..end,
                    width: 0.0,
                    ascent: 0.0,
                    descent: 0.0,
                    blank: false,
                    level: 0,
                    forced_break: false,
                    soft_hyphen: false,
                    hyphenation: false,
                    object: None,
                    tab: None,
                });
                continue;
            }
            // Blanks and other characters never share a word.
            if current.as_ref().is_some_and(|word| word.blank != blank) {
                close(&mut current, &mut words);
            }
            // A hyphenation point ends a piece of the word.
            if points.binary_search(&byte).is_ok()
                && let Some(mut piece) = current.take()
            {
                piece.hyphenation = true;
                words.push(piece);
            }
            let word = current.get_or_insert(ShapedWord {
                bytes: byte..byte,
                width: 0.0,
                ascent: 0.0,
                descent: 0.0,
                blank,
                level: 0,
                forced_break: false,
                soft_hyphen: false,
                hyphenation: false,
                object: None,
                tab: None,
            });
            word.bytes.end = end;
            word.width += advance;
            word.ascent = word.ascent.max(height_here * 0.8);
            word.descent = word.descent.max(height_here * 0.2);
            if matches!(character, '-' | '\u{2010}' | SOFT_HYPHEN | HARD_RETURN) {
                close(&mut current, &mut words);
            }
        }
        close(&mut current, &mut words);
        let mut shaped = ShapedParagraph {
            words,
            hyphen_width: size / 2.0,
            average_width: size / 2.0,
            ascent: height * 0.8,
            descent: height * 0.2,
        };
        shaped.classify(&request);
        shaped
    }

    fn measure(&mut self, text: &str, style: &ParagraphStyle) -> f32 {
        text.chars().count() as f32 * style.font_size_points * 20.0 / 2.0
    }

    fn measure_text(
        &mut self,
        text: &str,
        style: &ParagraphStyle,
        format: &CharacterFormat,
        line_heights: &LineHeights,
    ) -> TextExtent {
        let size = format.font_size_points.unwrap_or(style.font_size_points);
        let height = Self::height(
            line_heights,
            format.font_family.as_deref().unwrap_or(&style.font_family),
            size,
            format.bold.unwrap_or(style.bold),
            format.italic.unwrap_or(style.italic),
        );
        TextExtent {
            width: text.chars().count() as f32 * size * 20.0 / 2.0,
            ascent: height * 0.8,
            descent: height * 0.2,
        }
    }

    /// After every second letter, keeping two letters before and three
    /// after (TeX's English bounds): deterministic points for tests.
    fn hyphenate(&mut self, letters: &str) -> Vec<usize> {
        let starts: Vec<usize> = letters.char_indices().map(|(byte, _)| byte).collect();
        (2..starts.len().saturating_sub(2))
            .step_by(2)
            .map(|index| starts[index])
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{StyleId, default_styles};

    #[test]
    fn fixed_shaper_splits_words_at_break_opportunities() {
        let styles = default_styles();
        let style = &styles[&StyleId(1)];
        let text = "co\u{AD}op well-known\u{2028}x \u{FFFC}y";
        let shaped = FixedShaper.shape(ShapeRequest {
            text,
            style,
            runs: &[CharacterRun {
                start: 0,
                end: 2,
                format: CharacterFormat {
                    font_size_points: Some(24.0),
                    ..CharacterFormat::default()
                },
            }],
            end_format: &CharacterFormat::default(),
            line_heights: &LineHeights::default(),
            language: "en",
        });
        let words: Vec<_> = shaped
            .words
            .iter()
            .map(|word| &text[word.bytes.clone()])
            .collect();
        assert_eq!(
            words,
            [
                "co\u{AD}",
                "op",
                " ",
                "well-",
                "known\u{2028}",
                "x",
                " ",
                "\u{FFFC}",
                "y"
            ]
        );
        assert!(shaped.words[0].soft_hyphen);
        assert_eq!(
            shaped.words[0].width,
            240.0 + 240.0,
            "two 24 pt characters, the hyphen is free"
        );
        assert_eq!(shaped.words[0].ascent, 384.0);
        assert!(shaped.words[4].forced_break);
        assert!(shaped.words[2].blank);
        assert_eq!(shaped.words[7].object, Some(0));
        assert_eq!(shaped.hyphen_width, 120.0);
        let covered: usize = shaped.words.iter().map(|word| word.bytes.len()).sum();
        assert_eq!(covered, text.len());
    }

    #[test]
    fn text_marked_no_hyphenation_never_breaks_at_its_soft_hyphens() {
        // Decompile `1310:0000`: the hyphenator never splits `<+.>` text.
        let styles = default_styles();
        let style = &styles[&StyleId(1)];
        let text = "co\u{AD}op co\u{AD}op";
        let runs = [CharacterRun {
            start: 0,
            end: 6,
            format: CharacterFormat {
                no_hyphenation: Some(true),
                ..CharacterFormat::default()
            },
        }];
        let shaped = FixedShaper.shape(ShapeRequest {
            text,
            style,
            runs: &runs,
            end_format: &CharacterFormat::default(),
            line_heights: &LineHeights::default(),
            language: "en",
        });
        let breaks: Vec<bool> = shaped.words.iter().map(|word| word.soft_hyphen).collect();
        assert_eq!(breaks, [false, false, false, true, false]);
    }
}
