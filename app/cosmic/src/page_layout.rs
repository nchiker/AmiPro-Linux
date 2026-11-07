//! Pages for a document session: the layout engine (`amipro-layout`) over
//! COSMIC Text shaping (`docs/research/LINE_BREAKING_SPIKE.md`). Font sizes
//! are given to COSMIC Text in twips (points × 20), so every width and height
//! comes back in twips.

use std::collections::HashMap;

use amipro_document::{
    CharacterFormat, Document, FaceKey, LineHeights, OBJECT_REPLACEMENT_CHARACTER, ParagraphStyle,
};
use amipro_layout::{
    DisplaySpan, DocumentLayout, IncrementalLayout, LayoutStats, ShapeRequest, ShapedParagraph,
    ShapedWord, Shaper, display_spans, display_text,
};
use cosmic::iced::advanced::graphics::text::font_system;
use cosmic_text::{
    Attrs, AttrsList, Family, FontSystem, Metrics, ShapeLine, Shaping, Style, Weight,
};

/// The family COSMIC Text is asked for: metric-compatible substitutes for
/// the Windows core fonts, the family itself when installed, an installed
/// metric-compatible family for the PostScript core fonts (fontconfig's
/// groups, which COSMIC Text's font database does not apply), and otherwise
/// a substitute of the same kind.
pub(crate) fn substitute(fonts: &FontSystem, family: &str) -> String {
    let lower = family.to_ascii_lowercase();
    let known = match lower.as_str() {
        "times new roman" | "times" | "tms rmn" | "times roman" => Some("Liberation Serif"),
        "arial" | "helvetica" | "helv" | "swiss" => Some("Liberation Sans"),
        "courier new" | "courier" => Some("Liberation Mono"),
        _ => None,
    };
    if let Some(known) = known {
        return known.to_owned();
    }
    if installed(fonts, family) {
        return family.to_owned();
    }
    if let Some(compatible) = metric_compatible(&lower)
        .iter()
        .find(|name| installed(fonts, name))
    {
        return (*compatible).to_owned();
    }
    // Whole words, so that the foundry in "Monotype Corsiva" is not "mono".
    let words: Vec<&str> = lower
        .split(|character: char| !character.is_ascii_alphabetic())
        .filter(|word| !word.is_empty())
        .collect();
    let word_starts = |prefixes: &[&str]| {
        words
            .iter()
            .any(|word| prefixes.iter().any(|prefix| word.starts_with(prefix)))
    };
    if words.contains(&"mono") || word_starts(&["courier"]) {
        return "Liberation Mono".to_owned();
    }
    if lower.contains("sans")
        || lower.contains("arial")
        || lower.contains("gothic")
        || word_starts(&["goth", "helv", "swiss"])
    {
        return "Liberation Sans".to_owned();
    }
    // Script faces are narrow: the serif fits their lines best, whatever
    // kind Windows reported (Shelley Allegro Script's is swiss, and the sans
    // costs lines; Z003 makes Monotype Corsiva's lines 32% taller than Ami
    // Pro's caches, against 11%; `LAYOUT_COMPARISON.md`).
    if word_starts(&["script", "calligraph", "handwrit", "chancery", "corsiva"]) {
        return "Liberation Serif".to_owned();
    }
    // A name that tells nothing: the pitch and family Ami Pro stored for
    // the face, as Windows picks a substitute by it.
    let kind = face_kinds()
        .lock()
        .expect("face kinds")
        .get(&lower)
        .copied()
        .unwrap_or(0);
    match kind & 0xF0 {
        0x20 => "Liberation Sans".to_owned(),
        0x30 => "Liberation Mono".to_owned(),
        0x00 if kind & 3 == 1 => "Liberation Mono".to_owned(),
        _ => "Liberation Serif".to_owned(),
    }
}

/// Faces' Windows pitch and family bytes from the documents opened, by
/// lower-case name (`Document::face_kinds`).
fn face_kinds() -> &'static std::sync::Mutex<HashMap<String, u8>> {
    static KINDS: std::sync::OnceLock<std::sync::Mutex<HashMap<String, u8>>> =
        std::sync::OnceLock::new();
    KINDS.get_or_init(Default::default)
}

/// The substitutes `display_family` has resolved, by family name.
fn resolved_families() -> &'static std::sync::Mutex<HashMap<String, &'static str>> {
    static FAMILIES: std::sync::OnceLock<std::sync::Mutex<HashMap<String, &'static str>>> =
        std::sync::OnceLock::new();
    FAMILIES.get_or_init(Default::default)
}

/// Makes a document's face kinds known before it is laid out: a face that
/// is not installed and whose name gives no kind takes the substitute of
/// its stored kind (Ami Pro's rule for a missing face). The first document
/// to give a face a kind decides it.
pub fn register_face_kinds(kinds: &std::collections::BTreeMap<String, u8>) {
    let mut known = face_kinds().lock().expect("face kinds");
    for (family, kind) in kinds {
        if !known.contains_key(family) {
            known.insert(family.clone(), *kind);
            resolved_families()
                .lock()
                .expect("family names")
                .retain(|name, _| name.to_ascii_lowercase() != *family);
        }
    }
}

/// Whether a face of `family` is installed.
fn installed(fonts: &FontSystem, family: &str) -> bool {
    fonts.db().faces().any(|face| {
        face.families
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case(family))
    })
}

/// Installable families with the widths of the PostScript core font or
/// Windows font `lower` (lower case), preferred first. fontconfig's groups,
/// plus Book Antiqua (Palatino's widths) and Century Gothic (Avant Garde's),
/// which the comparison with Ami Pro's caches supports
/// (`docs/research/LAYOUT_COMPARISON.md`).
fn metric_compatible(lower: &str) -> &'static [&'static str] {
    match lower {
        "arial narrow" | "helvetica narrow" | "helvetica-narrow" => {
            &["Nimbus Sans Narrow", "Liberation Sans Narrow"]
        }
        "itc avant garde gothic" | "avant garde" | "avantgarde" | "century gothic" => {
            &["URW Gothic", "TeX Gyre Adventor"]
        }
        "itc bookman" | "bookman" | "bookman old style" => &["URW Bookman", "TeX Gyre Bonum"],
        "itc zapf chancery" | "zapf chancery" | "zapfchancery" => &["Z003", "TeX Gyre Chorus"],
        "palatino" | "palatino linotype" | "book antiqua" => &["P052", "TeX Gyre Pagella"],
        "new century schoolbook" | "newcenturyschlbk" | "century schoolbook" => {
            &["C059", "TeX Gyre Schola"]
        }
        _ => &[],
    }
}

/// The family to draw `family` with: the one shaping uses, as a name that
/// lives for the program (fonts are few).
#[cfg_attr(
    not(feature = "page-surface"),
    expect(dead_code, reason = "drawn by the page surface")
)]
pub fn display_family(family: &str) -> &'static str {
    let families = resolved_families();
    if let Some(name) = families.lock().expect("family names").get(family) {
        return name;
    }
    let resolved = {
        let mut fonts = font_system()
            .write()
            .expect("the font system lock is poisoned");
        substitute(fonts.raw(), family)
    };
    let name: &'static str = Box::leak(resolved.into_boxed_str());
    families
        .lock()
        .expect("family names")
        .insert(family.to_owned(), name);
    name
}

/// The ascent of `family` in ems, by its own metrics without the line gap:
/// where a subscript's baseline goes (half its font's ascent below the
/// line's).
#[cfg_attr(
    not(feature = "page-surface"),
    expect(dead_code, reason = "drawn by the page surface")
)]
pub fn ascent_em(family: &str, bold: bool, italic: bool) -> f32 {
    type Ascents = std::sync::Mutex<HashMap<(String, bool, bool), f32>>;
    static ASCENTS: std::sync::OnceLock<Ascents> = std::sync::OnceLock::new();
    let ascents = ASCENTS.get_or_init(Default::default);
    let key = (family.to_owned(), bold, italic);
    if let Some(ascent) = ascents.lock().expect("font ascents").get(&key) {
        return *ascent;
    }
    let ascent = {
        let mut fonts = font_system()
            .write()
            .expect("the font system lock is poisoned");
        let fonts = fonts.raw();
        let resolved = substitute(fonts, family);
        let attrs = Attrs::new()
            .family(Family::Name(&resolved))
            .weight(if bold { Weight::BOLD } else { Weight::NORMAL })
            .style(if italic { Style::Italic } else { Style::Normal });
        let line = ShapeLine::new(fonts, "x", &AttrsList::new(&attrs), Shaping::Advanced, 8);
        line.spans
            .iter()
            .flat_map(|span| &span.words)
            .flat_map(|word| &word.glyphs)
            .map(|glyph| glyph.ascent)
            .next()
            .unwrap_or(0.8)
    };
    ascents.lock().expect("font ascents").insert(key, ascent);
    ascent
}

/// The descent of `family` in ems, by its own metrics: with [`ascent_em`],
/// where COSMIC Text puts a line's baseline in a taller line.
#[cfg_attr(
    not(feature = "page-surface"),
    expect(dead_code, reason = "drawn by the page surface")
)]
pub fn descent_em(family: &str, bold: bool, italic: bool) -> f32 {
    type Descents = std::sync::Mutex<HashMap<(String, bool, bool), f32>>;
    static DESCENTS: std::sync::OnceLock<Descents> = std::sync::OnceLock::new();
    let descents = DESCENTS.get_or_init(Default::default);
    let key = (family.to_owned(), bold, italic);
    if let Some(descent) = descents.lock().expect("font descents").get(&key) {
        return *descent;
    }
    let descent = {
        let mut fonts = font_system()
            .write()
            .expect("the font system lock is poisoned");
        let fonts = fonts.raw();
        let resolved = substitute(fonts, family);
        let attrs = Attrs::new()
            .family(Family::Name(&resolved))
            .weight(if bold { Weight::BOLD } else { Weight::NORMAL })
            .style(if italic { Style::Italic } else { Style::Normal });
        let line = ShapeLine::new(fonts, "x", &AttrsList::new(&attrs), Shaping::Advanced, 8);
        line.spans
            .iter()
            .flat_map(|span| &span.words)
            .flat_map(|word| &word.glyphs)
            .map(|glyph| glyph.descent)
            .next()
            .unwrap_or(0.2)
    };
    descents.lock().expect("font descents").insert(key, descent);
    descent
}

/// A face's average character width in ems (`measure_average_width_em`),
/// for drawing tightened text as the shaper measured it.
#[cfg_attr(
    all(not(feature = "page-surface"), not(test)),
    expect(dead_code, reason = "drawn by the page surface")
)]
pub fn average_width_em(family: &str, bold: bool, italic: bool) -> f32 {
    type Averages = std::sync::Mutex<HashMap<(String, bool, bool), f32>>;
    static AVERAGES: std::sync::OnceLock<Averages> = std::sync::OnceLock::new();
    let averages = AVERAGES.get_or_init(Default::default);
    let key = (family.to_owned(), bold, italic);
    if let Some(average) = averages.lock().expect("font averages").get(&key) {
        return *average;
    }
    let average = {
        let mut fonts = font_system()
            .write()
            .expect("the font system lock is poisoned");
        let fonts = fonts.raw();
        let resolved = substitute(fonts, family);
        measure_average_width_em(fonts, &resolved, bold, italic)
    };
    averages.lock().expect("font averages").insert(key, average);
    average
}

/// The families of the installed fonts, sorted without regard to case: the
/// faces the Font dialog offers.
#[cfg_attr(
    not(feature = "page-surface"),
    expect(dead_code, reason = "offered by the page surface's Font dialog")
)]
pub fn installed_families() -> Vec<String> {
    let mut fonts = font_system()
        .write()
        .expect("the font system lock is poisoned");
    let mut families: Vec<String> = fonts
        .raw()
        .db()
        .faces()
        .filter_map(|face| face.families.first().map(|(name, _)| name.clone()))
        .collect();
    families.sort_by_key(|family| family.to_lowercase());
    families.dedup();
    families
}

/// One run's resolved font.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct FontKey {
    family: String,
    /// Twips.
    size_bits: u32,
    bold: bool,
    italic: bool,
    /// The document's printer line height for the font as named, in twips.
    line_height: Option<i32>,
    /// Text Tightness as letter spacing, in ems (`f32` bits; 0 for Normal).
    letter_spacing_bits: u32,
}

impl FontKey {
    fn size(&self) -> f32 {
        f32::from_bits(self.size_bits)
    }

    fn attrs(&self) -> Attrs<'_> {
        let size = self.size();
        Attrs::new()
            .family(Family::Name(&self.family))
            .weight(if self.bold {
                Weight::BOLD
            } else {
                Weight::NORMAL
            })
            .style(if self.italic {
                Style::Italic
            } else {
                Style::Normal
            })
            .metrics(Metrics::new(size, size * 1.2))
            .letter_spacing(f32::from_bits(self.letter_spacing_bits))
    }
}

/// Ami Pro's weights for Windows' average character width (the OS/2
/// table's classic `xAvgCharWidth`: lower-case letters and the space, per
/// thousand). For Times New Roman it gives Windows' 821 of 2,048 units.
const AVERAGE_WIDTH_WEIGHTS: [(char, f32); 27] = [
    ('a', 64.0),
    ('b', 14.0),
    ('c', 27.0),
    ('d', 35.0),
    ('e', 100.0),
    ('f', 20.0),
    ('g', 14.0),
    ('h', 42.0),
    ('i', 63.0),
    ('j', 3.0),
    ('k', 6.0),
    ('l', 35.0),
    ('m', 20.0),
    ('n', 56.0),
    ('o', 56.0),
    ('p', 17.0),
    ('q', 4.0),
    ('r', 49.0),
    ('s', 56.0),
    ('t', 71.0),
    ('u', 31.0),
    ('v', 10.0),
    ('w', 18.0),
    ('x', 3.0),
    ('y', 18.0),
    ('z', 2.0),
    (' ', 166.0),
];

/// A font's average character width in ems, as Windows reports it for the
/// fonts of Ami Pro's time: the weighted average of `AVERAGE_WIDTH_WEIGHTS`
/// over the installed face (metric-compatible substitutes keep the widths).
fn measure_average_width_em(fonts: &mut FontSystem, family: &str, bold: bool, italic: bool) -> f32 {
    let attrs = Attrs::new()
        .family(Family::Name(family))
        .weight(if bold { Weight::BOLD } else { Weight::NORMAL })
        .style(if italic { Style::Italic } else { Style::Normal });
    let text: String = AVERAGE_WIDTH_WEIGHTS
        .iter()
        .map(|(character, _)| *character)
        .collect();
    let line = ShapeLine::new(fonts, &text, &AttrsList::new(&attrs), Shaping::Advanced, 8);
    let mut advances = vec![0.0f32; text.len()];
    for glyph in line
        .spans
        .iter()
        .flat_map(|span| &span.words)
        .flat_map(|word| &word.glyphs)
    {
        if let Some(advance) = advances.get_mut(glyph.start) {
            *advance += glyph.x_advance;
        }
    }
    AVERAGE_WIDTH_WEIGHTS
        .iter()
        .zip(text.char_indices())
        .map(|((_, weight), (byte, _))| weight * advances[byte])
        .sum::<f32>()
        / 1000.0
}

/// The letter spacing, in ems of a font whose average width is
/// `average_em`, that gives Text Tightness `percent`: Ami Pro adds
/// (`percent` − 100)% of the font's average character width to every
/// character, spaces included (VM fixture TT1: Tight −9.6 and Loose +14.4
/// twips for 12 pt Times New Roman, whose average is 96).
pub(crate) fn tightness_em(average_em: f32, percent: u16) -> f32 {
    average_em * (f32::from(percent) - 100.0) / 100.0
}

/// Ascent, descent, and hyphen width of a font.
#[derive(Clone, Copy, Debug)]
struct FontMetrics {
    ascent: f32,
    descent: f32,
    hyphen: f32,
}

/// What the shaper keeps between layouts; cleared when fonts change.
#[derive(Default)]
pub struct ShaperCache {
    families: HashMap<String, String>,
    metrics: HashMap<FontKey, FontMetrics>,
    /// Each font's line gap (the hhea or typo line gap), in ems.
    gaps: HashMap<cosmic_text::fontdb::ID, f32>,
    /// Each resolved face's average character width, in ems.
    averages: HashMap<(String, bool, bool), f32>,
}

/// Shapes with COSMIC Text. Holds the font system for one layout.
///
/// A line is as tall as its fonts' ascent, descent, and line gap: Windows'
/// single spacing (text height plus external leading), which Ami Pro uses
/// (`docs/research/LAYOUT_COMPARISON.md`). The gap goes above the text.
pub struct CosmicShaper<'a> {
    fonts: &'a mut FontSystem,
    cache: &'a mut ShaperCache,
}

impl CosmicShaper<'_> {
    fn key(
        &mut self,
        style: &ParagraphStyle,
        format: &CharacterFormat,
        line_heights: &LineHeights,
    ) -> FontKey {
        let requested = format.font_family.as_deref().unwrap_or(&style.font_family);
        let size = format.font_size_points.unwrap_or(style.font_size_points);
        let bold = format.bold.unwrap_or(style.bold);
        let italic = format.italic.unwrap_or(style.italic);
        self.font_key(requested, size, bold, italic, Some(line_heights))
    }

    /// A display span's font at its full size, which gives its line's
    /// metrics, and at the size it is drawn when that is smaller
    /// (superscript, subscript, small caps).
    fn span_keys(
        &mut self,
        span: &DisplaySpan<'_>,
        line_heights: &LineHeights,
    ) -> (FontKey, FontKey) {
        let full = self.font_key(
            span.family,
            span.full_size,
            span.bold,
            span.italic,
            Some(line_heights),
        );
        let drawn = if span.is_reduced() {
            self.font_key(span.family, span.size, span.bold, span.italic, None)
        } else {
            full.clone()
        };
        (drawn, full)
    }

    /// The font `requested` names at `size` points, with the printer's line
    /// height for it when `line_heights` has one.
    fn font_key(
        &mut self,
        requested: &str,
        size: f32,
        bold: bool,
        italic: bool,
        line_heights: Option<&LineHeights>,
    ) -> FontKey {
        let line_height = line_heights
            .and_then(|heights| heights.get(&FaceKey::new(requested, size, bold, italic)));
        let family = requested;
        let family = match self.cache.families.get(family) {
            Some(resolved) => resolved.clone(),
            None => {
                let resolved = substitute(self.fonts, family);
                self.cache
                    .families
                    .insert(family.to_owned(), resolved.clone());
                resolved
            }
        };
        FontKey {
            family,
            size_bits: (size * 20.0).to_bits(),
            bold,
            italic,
            line_height,
            letter_spacing_bits: 0.0f32.to_bits(),
        }
    }

    /// `key` with the letter spacing of Text Tightness `percent`.
    fn tightened(&mut self, mut key: FontKey, percent: u16) -> FontKey {
        if percent != 100 {
            let face = (key.family.clone(), key.bold, key.italic);
            let average = match self.cache.averages.get(&face) {
                Some(average) => *average,
                None => {
                    let average = measure_average_width_em(self.fonts, &face.0, face.1, face.2);
                    self.cache.averages.insert(face, average);
                    average
                }
            };
            key.letter_spacing_bits = tightness_em(average, percent).to_bits();
        }
        key
    }

    fn line(
        &mut self,
        text: &str,
        base: &FontKey,
        runs: &[(std::ops::Range<usize>, FontKey)],
    ) -> ShapeLine {
        let mut list = AttrsList::new(&base.attrs());
        for (range, key) in runs {
            if key != base {
                list.add_span(range.clone(), &key.attrs());
            }
        }
        ShapeLine::new(self.fonts, text, &list, Shaping::Advanced, 8)
    }

    /// The line gap of a glyph's font, in ems.
    fn gap(&mut self, glyph: &cosmic_text::ShapeGlyph) -> f32 {
        if let Some(gap) = self.cache.gaps.get(&glyph.font_id) {
            return *gap;
        }
        let gap = self
            .fonts
            .get_font(glyph.font_id, glyph.font_weight)
            .map_or(0.0, |font| {
                let metrics = font.metrics();
                (metrics.leading / f32::from(metrics.units_per_em.max(1))).max(0.0)
            });
        self.cache.gaps.insert(glyph.font_id, gap);
        gap
    }

    fn font_metrics(&mut self, key: &FontKey) -> FontMetrics {
        if let Some(metrics) = self.cache.metrics.get(key) {
            return *metrics;
        }
        let size = key.size();
        let line = self.line("x-", key, &[]);
        let glyphs: Vec<_> = line
            .spans
            .iter()
            .flat_map(|span| &span.words)
            .flat_map(|word| &word.glyphs)
            .cloned()
            .collect();
        let mut metrics = FontMetrics {
            ascent: size * 0.8,
            descent: size * 0.2,
            hyphen: size / 3.0,
        };
        for glyph in &glyphs {
            metrics.ascent = (glyph.ascent + self.gap(glyph)) * size;
            metrics.descent = glyph.descent * size;
            if glyph.start == 1 {
                metrics.hyphen = glyph.width(size);
            }
        }
        // The printer's line height, split as the font splits its own.
        if let Some(height) = key.line_height {
            let total = metrics.ascent + metrics.descent;
            let height = height as f32;
            metrics.ascent = if total > 0.0 {
                metrics.ascent * height / total
            } else {
                height * 0.8
            };
            metrics.descent = height - metrics.ascent;
        }
        self.cache.metrics.insert(key.clone(), metrics);
        metrics
    }
}

impl Shaper for CosmicShaper<'_> {
    fn shape(&mut self, request: ShapeRequest<'_>) -> ShapedParagraph {
        let text = request.text;
        let heights = request.line_heights;
        let tightness = request.style.spacing.tightness;
        let base = self.key(request.style, &CharacterFormat::default(), heights);
        let base_metrics = self.font_metrics(&base);
        let base = self.tightened(base, tightness);
        // A line without text is set in the end format's font (decompile).
        let end = self.key(request.style, request.end_format, heights);
        let end_metrics = self.font_metrics(&end);
        // The letters shown (caps) and each span's font as drawn, with the
        // full-size font its line's metrics come from.
        let shown = display_text(text, request.style, request.runs);
        let spans: Vec<_> = display_spans(text, request.style, request.runs)
            .iter()
            .map(|span| {
                let (drawn, full) = self.span_keys(span, heights);
                let drawn = self.tightened(drawn, tightness);
                let full = if span.is_reduced() {
                    full
                } else {
                    drawn.clone()
                };
                (span.bytes.clone(), drawn, full)
            })
            .collect();
        let runs: Vec<_> = spans
            .iter()
            .map(|(bytes, drawn, _)| (bytes.clone(), drawn.clone()))
            .collect();
        let line = self.line(&shown, &base, &runs);
        let base_size = base.size();
        // Hyphenation points (decision D2): a word is shaped whole and split
        // into pieces there, each piece its glyphs' width.
        let language = request.language;
        let points = amipro_layout::hyphenation_points(&request, &mut |letters| {
            hyphenate_in(letters, language)
        });
        let mut words: Vec<ShapedWord> = Vec::new();
        for span in &line.spans {
            for word in &span.words {
                let (Some(start), Some(end)) = (
                    word.glyphs.iter().map(|glyph| glyph.start).min(),
                    word.glyphs.iter().map(|glyph| glyph.end).max(),
                ) else {
                    continue;
                };
                let mut shaped = ShapedWord {
                    bytes: start..end,
                    width: 0.0,
                    ascent: 0.0,
                    descent: 0.0,
                    blank: word.blank,
                    level: span.level.number(),
                    forced_break: false,
                    soft_hyphen: false,
                    hyphenation: false,
                    object: None,
                    tab: None,
                };
                for glyph in &word.glyphs {
                    if glyph.start > shaped.bytes.start
                        && points.binary_search(&glyph.start).is_ok()
                    {
                        let next = ShapedWord {
                            bytes: glyph.start..shaped.bytes.end,
                            width: 0.0,
                            ascent: 0.0,
                            descent: 0.0,
                            ..shaped.clone()
                        };
                        shaped.bytes.end = glyph.start;
                        shaped.hyphenation = true;
                        words.push(std::mem::replace(&mut shaped, next));
                    }
                    // Hard returns, soft hyphens, and objects show no glyph;
                    // a fallback font's glyph for them must not count.
                    if matches!(
                        text.get(glyph.start..glyph.end),
                        Some("\u{2028}" | "\u{AD}" | "\u{FFFC}" | "\t")
                    ) {
                        continue;
                    }
                    let size = glyph
                        .metrics_opt
                        .map_or(base_size, |metrics| metrics.font_size);
                    shaped.width += glyph.width(base_size);
                    // A font with a printer line height takes it, whatever
                    // face draws the glyph; text drawn smaller than its
                    // line takes its full-size font's metrics (decompile).
                    let (drawn, full) = spans
                        .iter()
                        .find(|(range, _, _)| range.contains(&glyph.start))
                        .map_or((&base, &base), |(_, drawn, full)| (drawn, full));
                    if drawn != full || full.line_height.is_some() {
                        let key = full.clone();
                        let metrics = self.font_metrics(&key);
                        shaped.ascent = shaped.ascent.max(metrics.ascent);
                        shaped.descent = shaped.descent.max(metrics.descent);
                        continue;
                    }
                    let gap = self.gap(glyph);
                    shaped.ascent = shaped.ascent.max((glyph.ascent + gap) * size);
                    shaped.descent = shaped.descent.max(glyph.descent * size);
                }
                words.push(shaped);
            }
        }
        words.sort_by_key(|word| word.bytes.start);
        let words = cover(text, words);
        let face = (base.family.clone(), base.bold, base.italic);
        let average = match self.cache.averages.get(&face) {
            Some(average) => *average,
            None => {
                let average = measure_average_width_em(self.fonts, &face.0, face.1, face.2);
                self.cache.averages.insert(face, average);
                average
            }
        };
        let mut shaped = ShapedParagraph {
            words,
            hyphen_width: base_metrics.hyphen,
            average_width: average * base_size,
            ascent: end_metrics.ascent,
            descent: end_metrics.descent,
        };
        shaped.classify(&request);
        shaped
    }

    fn measure(&mut self, text: &str, style: &ParagraphStyle) -> f32 {
        let base = self.key(style, &CharacterFormat::default(), &LineHeights::default());
        let size = base.size();
        self.line(text, &base, &[])
            .spans
            .iter()
            .flat_map(|span| &span.words)
            .map(|word| word.width(size))
            .sum()
    }

    fn measure_text(
        &mut self,
        text: &str,
        style: &ParagraphStyle,
        format: &CharacterFormat,
        line_heights: &LineHeights,
    ) -> amipro_layout::TextExtent {
        let key = self.key(style, format, line_heights);
        let metrics = self.font_metrics(&key);
        let size = key.size();
        let width = self
            .line(text, &key, &[])
            .spans
            .iter()
            .flat_map(|span| &span.words)
            .map(|word| word.width(size))
            .sum();
        amipro_layout::TextExtent {
            width,
            ascent: metrics.ascent,
            descent: metrics.descent,
        }
    }

    fn hyphenate(&mut self, letters: &str) -> Vec<usize> {
        hyphenate_english(letters)
    }
}

/// Where a run of letters may be hyphenated in US English: TeX's patterns
/// through `hypher` (decision D2), as byte offsets into it.
fn hyphenate_english(letters: &str) -> Vec<usize> {
    hyphenate_in(letters, "en")
}

/// The same in the document's language, an ISO 639-1 code (SP6); English
/// where `hypher` has no patterns for it.
fn hyphenate_in(letters: &str, language: &str) -> Vec<usize> {
    let code: [u8; 2] = language
        .as_bytes()
        .get(..2)
        .and_then(|code| code.try_into().ok())
        .unwrap_or(*b"en");
    let lang = hypher::Lang::from_iso(code).unwrap_or(hypher::Lang::English);
    let mut offset = 0;
    hypher::hyphenate(letters, lang)
        .map(|syllable| {
            offset += syllable.len();
            offset
        })
        .filter(|point| *point < letters.len())
        .collect()
}

/// Makes the words cover every byte once, in order, with each U+FFFC and
/// each tab a word of its own and of no width (the engine sizes objects and
/// tabs).
fn cover(text: &str, words: Vec<ShapedWord>) -> Vec<ShapedWord> {
    let mut covered: Vec<ShapedWord> = Vec::with_capacity(words.len());
    let mut offset = 0;
    for mut word in words {
        if word.bytes.end <= offset {
            // A cluster already covered by the previous word.
            if let Some(previous) = covered.last_mut() {
                previous.width += word.width;
            }
            continue;
        }
        word.bytes.start = offset.max(word.bytes.start);
        if word.bytes.start > offset {
            // Bytes without glyphs belong to the word before them.
            match covered.last_mut() {
                Some(previous) => previous.bytes.end = word.bytes.start,
                None => word.bytes.start = offset,
            }
        }
        offset = word.bytes.end;
        covered.push(word);
    }
    match covered.last_mut() {
        Some(last) => last.bytes.end = text.len(),
        None if !text.is_empty() => covered.push(ShapedWord {
            bytes: 0..text.len(),
            width: 0.0,
            ascent: 0.0,
            descent: 0.0,
            blank: text.chars().all(char::is_whitespace),
            level: 0,
            forced_break: false,
            soft_hyphen: false,
            hyphenation: false,
            object: None,
            tab: None,
        }),
        None => {}
    }
    // Objects and tabs stand alone.
    let alone = |character: char| character == OBJECT_REPLACEMENT_CHARACTER || character == '\t';
    let mut result = Vec::with_capacity(covered.len());
    for word in covered {
        let slice = &text[word.bytes.clone()];
        if !slice.contains(alone) || slice.chars().count() == 1 {
            if slice.chars().all(alone) {
                result.push(ShapedWord {
                    width: 0.0,
                    blank: false,
                    ..word
                });
            } else {
                result.push(word);
            }
            continue;
        }
        let mut start = word.bytes.start;
        let pieces: Vec<_> = slice
            .char_indices()
            .filter(|(_, character)| alone(*character))
            .map(|(at, character)| (word.bytes.start + at, character.len_utf8()))
            .collect();
        let letters = slice.chars().filter(|c| !alone(*c)).count().max(1);
        let per_char = word.width / letters as f32;
        for (piece, length) in pieces {
            if piece > start {
                let rest = &text[start..piece];
                result.push(ShapedWord {
                    bytes: start..piece,
                    width: per_char * rest.chars().count() as f32,
                    blank: rest.chars().all(char::is_whitespace),
                    ..word.clone()
                });
            }
            result.push(ShapedWord {
                bytes: piece..piece + length,
                width: 0.0,
                blank: false,
                ..word.clone()
            });
            start = piece + length;
        }
        if start < word.bytes.end {
            let rest = &text[start..word.bytes.end];
            result.push(ShapedWord {
                bytes: start..word.bytes.end,
                width: per_char * rest.chars().count() as f32,
                blank: rest.chars().all(char::is_whitespace),
                ..word.clone()
            });
        }
    }
    result
}

/// A session's pages, kept up to date incrementally.
#[derive(Default)]
pub struct SessionPages {
    layout: IncrementalLayout,
    shaping: ShaperCache,
    font_version: Option<cosmic::iced::advanced::graphics::text::Version>,
}

impl SessionPages {
    /// Lays out `document`, reusing the previous layout where it can.
    pub fn update(&mut self, document: &Document) {
        register_face_kinds(document.face_kinds());
        let mut fonts = font_system()
            .write()
            .expect("the font system lock is poisoned");
        let version = fonts.version();
        if self.font_version != Some(version) {
            self.layout.invalidate();
            self.shaping = ShaperCache::default();
            self.font_version = Some(version);
        }
        let mut shaper = CosmicShaper {
            fonts: fonts.raw(),
            cache: &mut self.shaping,
        };
        self.layout.layout(document, &mut shaper);
    }

    /// What dates and Doc Info entries show from the next layout.
    pub fn set_field_context(&mut self, context: amipro_document::FieldContext) {
        self.layout.set_field_context(context);
    }

    #[cfg_attr(
        not(any(test, feature = "page-surface")),
        expect(dead_code, reason = "drawn by the page surface (Phase 2.6)")
    )]
    pub fn layout(&self) -> &DocumentLayout {
        self.layout.current()
    }

    #[cfg_attr(not(test), expect(dead_code, reason = "read by the layout benchmarks"))]
    pub fn stats(&self) -> LayoutStats {
        self.layout.stats()
    }

    pub fn page_count(&self) -> usize {
        self.layout.current().pages.len().max(1)
    }

    /// The page showing `byte` of main-flow paragraph `paragraph`.
    pub fn page_of(&self, paragraph: usize, byte: usize) -> u32 {
        self.layout.current().page_of(paragraph, byte).unwrap_or(1)
    }
}

/// Runs `run` with a COSMIC Text shaper of its own, over a copy of the
/// global font system's fonts: for work off the interface's thread (an
/// import's layouts), which must not hold the global lock while the pages
/// are drawn.
pub(crate) fn with_private_shaper<T>(run: impl FnOnce(&mut CosmicShaper<'_>) -> T) -> T {
    let (locale, fonts) = {
        let mut global = font_system()
            .write()
            .expect("the font system lock is poisoned");
        let raw = global.raw();
        (raw.locale().to_owned(), raw.db().clone())
    };
    let mut fonts = FontSystem::new_with_locale_and_db(locale, fonts);
    let mut cache = ShaperCache::default();
    let mut shaper = CosmicShaper {
        fonts: &mut fonts,
        cache: &mut cache,
    };
    run(&mut shaper)
}

/// The family that shaping uses for `family`.
#[cfg(test)]
pub(crate) fn substitute_family(family: &str) -> String {
    let mut fonts = font_system().write().unwrap();
    substitute(fonts.raw(), family)
}

/// Runs `run` with a COSMIC Text shaper over the global font system.
#[cfg(test)]
pub(crate) fn with_shaper<T>(run: impl FnOnce(&mut CosmicShaper<'_>) -> T) -> T {
    let mut fonts = font_system().write().unwrap();
    let mut cache = ShaperCache::default();
    let mut shaper = CosmicShaper {
        fonts: fonts.raw(),
        cache: &mut cache,
    };
    run(&mut shaper)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{Caps, CharacterRun, StyleId, default_styles};

    #[test]
    fn printer_line_heights_replace_the_fonts_own() {
        // DECISIONS.md, 2026-10-12, "Line heights follow the printer": a
        // font with a printer line height takes it, split as the font splits
        // its own; a run in another font keeps its metrics.
        let styles = default_styles();
        let mut style = styles[&StyleId(1)].clone();
        style.font_family = "Times New Roman".into();
        style.font_size_points = 12.0;
        let text = "Printer line heights";
        let runs = [CharacterRun {
            start: 8,
            end: 12,
            format: CharacterFormat {
                font_family: Some("Arial".into()),
                ..CharacterFormat::default()
            },
        }];
        let mut heights = LineHeights::default();
        heights.insert(FaceKey::new("times new roman", 12.0, false, false), 284);
        let shape = |heights: &LineHeights, runs: &[CharacterRun]| {
            with_shaper(|shaper| {
                shaper.shape(ShapeRequest {
                    text,
                    style: &style,
                    runs,
                    end_format: &CharacterFormat::default(),
                    line_heights: heights,
                    language: "en",
                })
            })
        };
        let natural = shape(&LineHeights::default(), &[]);
        let printer = shape(&heights, &[]);
        assert!((printer.ascent + printer.descent - 284.0).abs() < 0.01);
        assert!(
            (printer.ascent / 284.0 - natural.ascent / (natural.ascent + natural.descent)).abs()
                < 0.001
        );
        for word in printer.words.iter().filter(|word| !word.blank) {
            assert!(
                (word.ascent + word.descent - 284.0).abs() < 0.01,
                "{word:?}"
            );
        }
        // The Arial word keeps its own (uncalibrated) height.
        let mixed = shape(&heights, &runs);
        let arial = mixed
            .words
            .iter()
            .find(|word| &text[word.bytes.clone()] == "line")
            .unwrap();
        let arial_natural = shape(&LineHeights::default(), &runs)
            .words
            .into_iter()
            .find(|word| &text[word.bytes.clone()] == "line")
            .unwrap();
        assert_eq!(arial.ascent, arial_natural.ascent);
        assert_eq!(arial.descent, arial_natural.descent);
    }

    #[test]
    fn text_tightness_adds_a_share_of_the_average_width_to_every_character_tt1() {
        // VM fixture TT1: thirty Ms and thirty x's with spaces in Times New
        // Roman 12 grow by about 9.6 twips a character for Tight (90) and 14.4
        // for Loose (115); the average width is Windows' 821 of 2,048 ems.
        let average = average_width_em("Times New Roman", false, false);
        assert!((average - 821.0 / 2048.0).abs() < 0.002, "{average}");
        let styles = default_styles();
        let mut style = styles[&StyleId(1)].clone();
        style.font_family = "Times New Roman".into();
        style.font_size_points = 12.0;
        let width = |text: &str, tightness: u16| {
            let mut style = style.clone();
            style.spacing.tightness = tightness;
            with_shaper(|shaper| {
                shaper
                    .shape(ShapeRequest {
                        text,
                        style: &style,
                        runs: &[],
                        end_format: &CharacterFormat::default(),
                        line_heights: &LineHeights::default(),
                        language: "en",
                    })
                    .words
                    .iter()
                    .map(|word| word.width)
                    .sum::<f32>()
            })
        };
        let per_character = 240.0 * 821.0 / 2048.0 / 100.0;
        for text in ["M".repeat(30), vec!["x"; 30].join(" ")] {
            let count = text.chars().count() as f32;
            let normal = width(&text, 100);
            let tight = width(&text, 90);
            let loose = width(&text, 115);
            assert!(
                (normal - tight - 10.0 * per_character * count).abs() < count * 0.5,
                "{text}: {normal} {tight}"
            );
            assert!(
                (loose - normal - 15.0 * per_character * count).abs() < count * 0.5,
                "{text}: {normal} {loose}"
            );
        }
    }

    #[test]
    fn caps_and_scripts_change_widths_but_not_line_heights() {
        // SAM_FORMAT_NOTES.md, "Character attributes": caps change the
        // letters measured; superscript, subscript, and small caps are set
        // smaller, but their lines keep the full-size font's height.
        let styles = default_styles();
        let mut style = styles[&StyleId(1)].clone();
        style.font_family = "Times New Roman".into();
        style.font_size_points = 12.0;
        let text = "measured words";
        let format = |format: CharacterFormat| {
            [CharacterRun {
                start: 0,
                end: text.len(),
                format,
            }]
        };
        let mut heights = LineHeights::default();
        heights.insert(FaceKey::new("times new roman", 12.0, false, false), 284);
        let shape = |runs: &[CharacterRun], heights: &LineHeights| {
            with_shaper(|shaper| {
                shaper.shape(ShapeRequest {
                    text,
                    style: &style,
                    runs,
                    end_format: &CharacterFormat::default(),
                    line_heights: heights,
                    language: "en",
                })
            })
        };
        let width =
            |shaped: &ShapedParagraph| -> f32 { shaped.words.iter().map(|word| word.width).sum() };
        let caps = |caps| {
            format(CharacterFormat {
                caps: Some(caps),
                ..CharacterFormat::default()
            })
        };
        let plain = shape(&[], &LineHeights::default());
        let upper = shape(&caps(Caps::Upper), &LineHeights::default());
        let small = shape(&caps(Caps::Small), &LineHeights::default());
        let lower = shape(&caps(Caps::Lower), &LineHeights::default());
        assert!(width(&upper) > width(&plain) * 1.2, "capitals are wider");
        assert!(width(&small) < width(&upper) && width(&small) > width(&plain));
        assert_eq!(width(&lower), width(&plain), "already lower case");
        let superscript = format(CharacterFormat {
            superscript: Some(true),
            ..CharacterFormat::default()
        });
        let raised = shape(&superscript, &LineHeights::default());
        assert!((width(&raised) / width(&plain) - 7.0 / 12.0).abs() < 0.03);
        let measured = |shaped: &ShapedParagraph| {
            let word = &shaped.words[0];
            word.ascent + word.descent
        };
        assert!((measured(&raised) - measured(&plain)).abs() < 0.01);
        assert!((measured(&small) - measured(&plain)).abs() < 0.01);
        // With a printer line height, scripts take it too.
        let raised = shape(&superscript, &heights);
        assert!((measured(&raised) - 284.0).abs() < 0.01);
        // Word boundaries are those of the text as typed.
        let words: Vec<_> = upper
            .words
            .iter()
            .map(|word| &text[word.bytes.clone()])
            .collect();
        assert_eq!(words, ["measured", " ", "words"]);
    }

    #[test]
    fn cosmic_shaper_keeps_the_word_contract_in_twips() {
        let styles = default_styles();
        let mut style = styles[&StyleId(1)].clone();
        style.font_family = "Times New Roman".into();
        style.font_size_points = 12.0;
        let text = "Ami Pro co\u{AD}op well-known\u{2028}x \u{FFFC}y שלום";
        let runs = [CharacterRun {
            start: 0,
            end: 3,
            format: CharacterFormat {
                font_size_points: Some(24.0),
                ..CharacterFormat::default()
            },
        }];
        let shaped = with_shaper(|shaper| {
            shaper.shape(ShapeRequest {
                text,
                style: &style,
                runs: &runs,
                end_format: &CharacterFormat::default(),
                line_heights: &LineHeights::default(),
                language: "en",
            })
        });
        // Every byte in exactly one word, in order.
        let mut offset = 0;
        for word in &shaped.words {
            assert_eq!(word.bytes.start, offset, "{:?}", shaped.words);
            offset = word.bytes.end;
        }
        assert_eq!(offset, text.len());
        let word = |slice: &str| {
            shaped
                .words
                .iter()
                .find(|word| &text[word.bytes.clone()] == slice)
                .unwrap_or_else(|| panic!("no word {slice:?} in {:?}", shaped.words))
        };
        // Twips: 24 pt "Ami" is about twice as wide as 12 pt would be. A
        // line is Windows' single spacing: Liberation Serif's ascent,
        // descent, and line gap, (1825 + 443 + 87) / 2048 em, with the gap
        // above the text.
        let single = 2_355.0 / 2_048.0;
        let ami = word("Ami");
        assert!(
            (ami.ascent + ami.descent - 480.0 * single).abs() < 1.0,
            "{ami:?}"
        );
        assert!(
            (ami.descent - 480.0 * 443.0 / 2_048.0).abs() < 1.0,
            "{ami:?}"
        );
        assert!(ami.width > 600.0 && ami.width < 900.0, "{ami:?}");
        assert!((shaped.ascent + shaped.descent - 240.0 * single).abs() < 1.0);
        assert!(word("co\u{AD}").soft_hyphen);
        let hard_return = word("\u{2028}");
        assert!(hard_return.forced_break && hard_return.width == 0.0);
        let object = word("\u{FFFC}");
        assert_eq!(object.object, Some(0));
        assert_eq!((object.width, object.ascent), (0.0, 0.0));
        assert!(word(" ").blank);
        assert_eq!(word("שלום").level % 2, 1, "right to left");
        assert!(shaped.hyphen_width > 50.0 && shaped.hyphen_width < 120.0);
    }

    #[test]
    fn session_pages_follow_edits_incrementally() {
        let text: Vec<String> = (0..300)
            .map(|index| format!("Paragraph {index} with enough words to wrap onto a second line of the page when set in twelve point Times."))
            .collect();
        let mut document = Document::from_plain_text(&text.join("\n"));
        let mut pages = SessionPages::default();
        pages.update(&document);
        assert_eq!(pages.stats().kind, amipro_layout::LayoutKind::Full);
        let count = pages.page_count();
        assert!(count > 5, "{count} pages");
        assert_eq!(pages.page_of(0, 0), 1);
        assert_eq!(pages.page_of(299, 0) as usize, count);
        let flow = document.main_flow_id();
        document = amipro_editing::apply(
            &document,
            &amipro_editing::Command::insert(
                amipro_editing::TextPosition {
                    flow,
                    paragraph: 150,
                    byte: 0,
                },
                "typed ",
            ),
        )
        .unwrap()
        .document;
        pages.update(&document);
        let stats = pages.stats();
        assert_eq!(stats.kind, amipro_layout::LayoutKind::Incremental);
        assert!(stats.pages_reused > stats.pages_laid_out, "{stats:?}");
        let mut fresh = SessionPages::default();
        fresh.update(&document);
        assert_eq!(pages.layout(), fresh.layout());
    }

    /// Release-mode layout baseline over private SAM files:
    /// `AMIPRO_LAYOUT_SAMS` (paths separated by `:`), else `AMIPRO_PERF_SAM`.
    #[test]
    #[ignore = "manual release-mode page layout performance baseline"]
    fn page_layout_performance_baseline() {
        use std::time::{Duration, Instant};

        fn summary(samples: &mut [Duration]) -> String {
            samples.sort_unstable();
            let at = |percent: usize| {
                samples[(samples.len() - 1) * percent / 100].as_secs_f64() * 1_000.0
            };
            format!(
                "median {:.3} ms, p95 {:.3} ms, worst {:.3} ms",
                at(50),
                at(95),
                samples.last().unwrap().as_secs_f64() * 1_000.0
            )
        }

        let paths: Vec<std::path::PathBuf> = std::env::var_os("AMIPRO_LAYOUT_SAMS")
            .or_else(|| std::env::var_os("AMIPRO_PERF_SAM"))
            .map(|paths| std::env::split_paths(&paths).collect())
            .unwrap_or_default();
        if paths.is_empty() {
            eprintln!("AMIPRO_LAYOUT_SAMS and AMIPRO_PERF_SAM are not set; skipping");
            return;
        }
        for path in paths {
            let document = amipro_formats::import_sam_path(&path).unwrap().document;
            let flow = document.main_flow_id();
            let mut full = Vec::new();
            let mut pages = SessionPages::default();
            for _ in 0..5 {
                pages = SessionPages::default();
                let started = Instant::now();
                pages.update(&document);
                full.push(started.elapsed());
            }
            let count = pages.page_count();
            // Typing in the middle of the longest paragraph near the middle.
            let middle = document.paragraphs().len() / 2;
            let paragraph = (middle..document.paragraphs().len())
                .find(|index| document.paragraphs()[*index].text.len() > 40)
                .unwrap_or(middle);
            let mut current = document.clone();
            let mut typing = Vec::new();
            let (mut laid_out, mut reused) = (0, 0);
            for sample in 0..40 {
                let text = &current.paragraphs()[paragraph].text;
                let byte = (0..=text.len() / 2)
                    .rev()
                    .find(|byte| text.is_char_boundary(*byte))
                    .unwrap();
                current = amipro_editing::apply(
                    &current,
                    &amipro_editing::Command::insert(
                        amipro_editing::TextPosition {
                            flow,
                            paragraph,
                            byte,
                        },
                        if sample % 6 == 5 { " " } else { "e" },
                    ),
                )
                .unwrap()
                .document;
                let started = Instant::now();
                pages.update(&current);
                typing.push(started.elapsed());
                laid_out += pages.stats().pages_laid_out;
                reused += pages.stats().pages_reused;
            }
            // Shaping the edited paragraph alone, the part of a relayout
            // that the shape cache cannot save.
            let mut shaping = Vec::new();
            let edited = current.paragraphs()[paragraph].clone();
            let style = current.styles()[&edited.style].clone();
            with_shaper(|shaper| {
                for _ in 0..40 {
                    let started = Instant::now();
                    std::hint::black_box(shaper.shape(ShapeRequest {
                        text: &edited.text,
                        style: &style,
                        runs: &edited.runs,
                        end_format: &edited.end_format,
                        line_heights: current.line_heights(),
                        language: "en",
                    }));
                    shaping.push(started.elapsed());
                }
            });
            // Enter: a new paragraph shifts every index after it.
            let mut enter = Vec::new();
            for _ in 0..20 {
                current = amipro_editing::apply(
                    &current,
                    &amipro_editing::Command::split_paragraph(amipro_editing::TextPosition {
                        flow,
                        paragraph,
                        byte: 0,
                    }),
                )
                .unwrap()
                .document;
                let started = Instant::now();
                pages.update(&current);
                enter.push(started.elapsed());
            }
            eprintln!(
                "{}: {} paragraphs, {count} pages; full layout {}; relayout after typing {} ({:.1} pages set, {:.1} reused on average; shaping the edited paragraph of {} bytes alone {}); after Enter {}",
                path.file_name().unwrap().to_string_lossy(),
                document.paragraphs().len(),
                summary(&mut full),
                summary(&mut typing),
                laid_out as f64 / 40.0,
                reused as f64 / 40.0,
                edited.text.len(),
                summary(&mut shaping),
                summary(&mut enter),
            );
        }
    }

    #[test]
    fn hyphenation_follows_the_documents_language() {
        // SP6: a French document takes French patterns; a language hypher
        // has none for falls back to English.
        assert_ne!(
            hyphenate_in("université", "fr"),
            hyphenate_in("université", "en")
        );
        assert_eq!(
            hyphenate_in("hyphenation", "xx"),
            hyphenate_english("hyphenation")
        );
    }

    #[test]
    fn hyphenation_points_come_from_tex_patterns() {
        // hypher's US English patterns with TeX's bounds (two letters
        // before, three after); byte offsets into the run of letters.
        assert_eq!(hyphenate_english("extensive"), [2, 5]);
        assert_eq!(hyphenate_english("hyphenation"), [2, 6]);
        assert!(hyphenate_english("cat").is_empty());
        assert_eq!(
            hyphenate_english("Hyphenation"),
            [2, 6],
            "letter case aside"
        );
    }

    #[test]
    fn substitutes_follow_the_core_fonts() {
        let mut fonts = font_system().write().unwrap();
        let raw = fonts.raw();
        assert_eq!(substitute(raw, "Times New Roman"), "Liberation Serif");
        assert_eq!(substitute(raw, "Arial"), "Liberation Sans");
        assert_eq!(substitute(raw, "Courier New"), "Liberation Mono");
        assert_eq!(substitute(raw, "No Such Sans Font"), "Liberation Sans");
        assert_eq!(substitute(raw, "No Such Font"), "Liberation Serif");
        // Names from the user's corpus that are not installed.
        assert_eq!(substitute(raw, "Monotype Corsiva"), "Liberation Serif");
        assert_eq!(substitute(raw, "Swiss742SWC"), "Liberation Sans");
        assert_eq!(substitute(raw, "Frank Goth Cd"), "Liberation Sans");
        assert_eq!(
            substitute(raw, "FranklinGothic-Condensed"),
            "Liberation Sans"
        );
        assert_eq!(substitute(raw, "Sans Mono Test"), "Liberation Mono");
        // A name that tells nothing takes the kind its documents stored
        // (Windows' pitch and family); script faces keep the serif.
        register_face_kinds(&std::collections::BTreeMap::from([
            ("no such kind swiss".to_owned(), 0x22),
            ("no such kind modern".to_owned(), 0x31),
            ("no such kind fixed".to_owned(), 0x01),
            ("no such kind script".to_owned(), 0x22),
            ("no such kind roman".to_owned(), 0x12),
        ]));
        for (family, expected) in [
            ("No Such Kind Swiss", "Liberation Sans"),
            ("No Such Kind Modern", "Liberation Mono"),
            ("No Such Kind Fixed", "Liberation Mono"),
            ("No Such Kind Script", "Liberation Serif"),
            ("No Such Kind Roman", "Liberation Serif"),
        ] {
            assert_eq!(substitute(raw, family), expected, "{family}");
        }
        // Metric-compatible faces when installed (fonts-urw-base35 and
        // Liberation Sans Narrow on Pop!_OS), else a substitute by kind.
        for (family, by_kind) in [
            ("Book Antiqua", "Liberation Serif"),
            ("Century Schoolbook", "Liberation Serif"),
            ("Century Gothic", "Liberation Sans"),
            ("Helvetica-Narrow", "Liberation Sans"),
            ("ITC Bookman", "Liberation Serif"),
        ] {
            let expected = metric_compatible(&family.to_ascii_lowercase())
                .iter()
                .find(|name| installed(raw, name))
                .copied()
                .unwrap_or(by_kind);
            assert_eq!(substitute(raw, family), expected, "{family}");
        }
    }

    #[test]
    fn metric_compatible_faces_follow_fontconfig_groups() {
        assert_eq!(metric_compatible("book antiqua")[0], "P052");
        assert_eq!(metric_compatible("palatino")[0], "P052");
        assert_eq!(metric_compatible("newcenturyschlbk")[0], "C059");
        assert_eq!(metric_compatible("bookman old style")[0], "URW Bookman");
        assert_eq!(metric_compatible("century gothic")[0], "URW Gothic");
        assert_eq!(metric_compatible("arial narrow")[0], "Nimbus Sans Narrow");
        assert!(metric_compatible("times new roman").is_empty());
    }
}
