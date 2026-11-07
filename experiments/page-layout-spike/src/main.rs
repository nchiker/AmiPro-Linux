//! Runs the Phase 2.5a cases and prints a report
//! (`docs/research/LINE_BREAKING_SPIKE.md`).
//!
//! Usage: `cargo run --release -- [README31.SAM]` (default: `$AMIPRO_PERF_SAM`).

use std::time::Instant;

use cosmic_text::{Attrs, AttrsList, Family, FontSystem};
use page_layout_spike::{
    HARD_RETURN, Line, OBJECT, SOFT_HYPHEN, Word, break_lines, logical_words, place,
    reference_lines, shape,
};

const FONT_SIZE: f32 = 16.0;

fn attrs(family: &str) -> AttrsList {
    AttrsList::new(&Attrs::new().family(Family::Name(family)))
}

fn hyphen_width(fonts: &mut FontSystem, list: &AttrsList) -> f32 {
    let line = shape(fonts, "-", list);
    logical_words(&line, "-", FONT_SIZE, 0.0)
        .iter()
        .map(|word| word.width)
        .sum()
}

/// Compares our breaking at one width with COSMIC Text's own wrapping.
/// Returns (lines compared, line mismatches, largest x difference).
fn compare(
    fonts: &mut FontSystem,
    text: &str,
    list: &AttrsList,
    width: f32,
) -> (usize, usize, f32) {
    let line = shape(fonts, text, list);
    let words = logical_words(&line, text, FONT_SIZE, 0.0);
    let ours = break_lines(&words, |_| width, 0.0);
    let reference = reference_lines(&line, FONT_SIZE, width);
    let mut mismatches = usize::from(ours.len() != reference.len());
    let mut largest = 0.0f32;
    for (broken, expected) in ours.iter().zip(&reference) {
        let placed = place(&line, &words, broken, FONT_SIZE);
        // COSMIC Text also places a line's trailing spaces as glyphs; break
        // positions are what matter, so compare without them.
        let trimmed = |glyphs: &[page_layout_spike::Placed]| -> Vec<std::ops::Range<usize>> {
            let mut ranges: Vec<_> = glyphs.iter().map(|glyph| glyph.bytes.clone()).collect();
            ranges.sort_by_key(|range| range.start);
            while ranges
                .last()
                .is_some_and(|range| text[range.clone()].chars().all(char::is_whitespace))
            {
                ranges.pop();
            }
            ranges
        };
        if trimmed(&placed) != trimmed(expected) {
            mismatches += 1;
            continue;
        }
        // Compare positions relative to each line's first glyph.
        let ours_origin = placed.first().map_or(0.0, |glyph| glyph.x);
        let reference_origin = expected.first().map_or(0.0, |glyph| glyph.x);
        for (a, b) in placed.iter().zip(expected) {
            largest = largest.max(((a.x - ours_origin) - (b.x - reference_origin)).abs());
        }
    }
    (ours.len().max(reference.len()), mismatches, largest)
}

fn describe(text: &str, words: &[Word], lines: &[Line]) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            let start = words
                .get(line.words.start)
                .map_or(0, |word| word.bytes.start);
            let end = line
                .words
                .clone()
                .last()
                .map_or(start, |index| words[index].bytes.end);
            format!(
                "{:>6.1} {}{}{} {:?}",
                line.width,
                if line.hyphen { "H" } else { " " },
                if line.forced { "F" } else { " " },
                if line.overflow { "O" } else { " " },
                &text[start..end]
            )
        })
        .collect()
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("AMIPRO_PERF_SAM").ok());
    let mut fonts = FontSystem::new();
    let serif = attrs("Liberation Serif");
    let hyphen = hyphen_width(&mut fonts, &serif);
    println!("hyphen width {hyphen:.2} at {FONT_SIZE} px");

    // A. README31: our breaking against COSMIC Text's at uniform widths.
    if let Some(path) = path {
        let document = amipro_formats::import_sam_path(&path).unwrap().document;
        let paragraphs: Vec<String> = document
            .paragraphs()
            .iter()
            .map(|paragraph| paragraph.text.clone())
            .collect();
        let plain: Vec<&String> = paragraphs
            .iter()
            .filter(|text| !text.contains([OBJECT, HARD_RETURN, SOFT_HYPHEN]))
            .collect();
        println!(
            "\nA. {path}: {} paragraphs, {} without objects or special characters",
            paragraphs.len(),
            plain.len()
        );
        for width in [624.0, 300.0, 120.0] {
            let (mut lines, mut mismatches, mut largest) = (0, 0, 0.0f32);
            let mut failing = Vec::new();
            for text in &plain {
                let (count, bad, x) = compare(&mut fonts, text, &serif, width);
                lines += count;
                mismatches += bad;
                largest = largest.max(x);
                if bad > 0 && failing.len() < 3 {
                    failing.push(text.chars().take(50).collect::<String>());
                }
            }
            println!(
                "   width {width}: {lines} lines, {mismatches} differ from COSMIC Text, largest x difference {largest:.3}"
            );
            for text in failing {
                println!("     differs: {text:?}");
            }
        }

        // Timing: shaping and breaking every paragraph.
        let started = Instant::now();
        let shaped: Vec<_> = paragraphs
            .iter()
            .map(|text| (shape(&mut fonts, text, &serif), text))
            .collect();
        let shaping = started.elapsed();
        let started = Instant::now();
        let mut total_lines = 0;
        for _ in 0..20 {
            for (line, text) in &shaped {
                let words = logical_words(line, text, FONT_SIZE, 40.0);
                total_lines += break_lines(
                    &words,
                    |index| if index % 3 == 1 { 300.0 } else { 624.0 },
                    hyphen,
                )
                .len();
            }
        }
        let breaking = started.elapsed() / 20;
        let started = Instant::now();
        for _ in 0..20 {
            for (line, _) in &shaped {
                total_lines += reference_lines(line, FONT_SIZE, 624.0).len();
            }
        }
        let reference = started.elapsed() / 20;
        println!(
            "   timing over all {} paragraphs: shaping {shaping:?} once; our breaking (varying widths) {breaking:?}; COSMIC Text layout {reference:?} ({total_lines} lines in total)",
            paragraphs.len()
        );
    }

    // B. A width per line: text wrapping beside a frame.
    let text = "Ami Pro wraps the text of a paragraph around a frame placed beside it, so the lines next to the frame are shorter than the lines below it, and the breaker must accept a width for each line.";
    let line = shape(&mut fonts, text, &serif);
    let words = logical_words(&line, text, FONT_SIZE, 0.0);
    let widths = [400.0, 200.0, 200.0, 400.0, 400.0, 400.0];
    let lines = break_lines(&words, |index| widths[index.min(widths.len() - 1)], hyphen);
    println!("\nB. widths {widths:?}");
    let mut greedy = true;
    for (index, broken) in lines.iter().enumerate() {
        let limit = widths[index.min(widths.len() - 1)];
        // Greedy: the next non-blank word would not have fitted.
        if let Some(next) = (broken.words.end..words.len()).find(|word| !words[*word].blank) {
            let with_next: f32 = words[broken.words.start..=next]
                .iter()
                .map(|word| word.width)
                .sum();
            greedy &= with_next > limit;
        }
        greedy &= broken.width <= limit;
    }
    println!("   every line fits and is greedy: {greedy}");
    for row in describe(text, &words, &lines) {
        println!("   {row}");
    }

    // C. Inline objects: a Flow with text frame as a 90 px box.
    let text =
        "Before the frame \u{FFFC} after the frame, then \u{FFFC}\u{FFFC} two more boxes in a row.";
    let line = shape(&mut fonts, text, &serif);
    let words = logical_words(&line, text, FONT_SIZE, 90.0);
    let objects = words.iter().filter(|word| word.object).count();
    let lines = break_lines(&words, |_| 260.0, hyphen);
    println!("\nC. objects: {objects} words are boxes of 90 px");
    for row in describe(text, &words, &lines) {
        println!("   {row}");
    }

    // D. U+2028: hard returns force breaks, even when the line has room.
    let text = "Short\u{2028}lines\u{2028}from hard returns, then a longer one that wraps normally at the width.";
    let line = shape(&mut fonts, text, &serif);
    let words = logical_words(&line, text, FONT_SIZE, 0.0);
    let lines = break_lines(&words, |_| 300.0, hyphen);
    println!("\nD. hard returns");
    for row in describe(text, &words, &lines) {
        println!("   {row}");
    }

    // E. U+00AD: a break point that shows a hyphen only when used.
    let text =
        "Cooperation needs supercalifragi\u{AD}listic\u{AD}expialidocious co\u{AD}operation.";
    let line = shape(&mut fonts, text, &serif);
    let words = logical_words(&line, text, FONT_SIZE, 0.0);
    let unbroken: f32 = words.iter().map(|word| word.width).sum();
    let reference_width: f32 = {
        let plain = text.replace(SOFT_HYPHEN, "");
        let line = shape(&mut fonts, &plain, &serif);
        logical_words(&line, &plain, FONT_SIZE, 0.0)
            .iter()
            .map(|word| word.width)
            .sum()
    };
    println!(
        "\nE. soft hyphens: unbroken width {unbroken:.2} vs the same text without them {reference_width:.2}"
    );
    for width in [330.0, 260.0, 200.0] {
        let lines = break_lines(&words, |_| width, hyphen);
        println!("   width {width}:");
        for row in describe(text, &words, &lines) {
            println!("     {row}");
        }
    }

    // F. Bidi: Hebrew and Arabic runs inside a left-to-right paragraph.
    let sans = attrs("Noto Sans");
    let text = "English text before שלום עולם זה טקסט בעברית and then مرحبا بالعالم هذا نص عربي before the end of the sentence.";
    println!("\nF. bidi");
    for width in [500.0, 260.0, 160.0] {
        let (lines, mismatches, largest) = compare(&mut fonts, text, &sans, width);
        println!(
            "   width {width}: {lines} lines, {mismatches} differ from COSMIC Text, largest x difference {largest:.3}"
        );
    }

    // G. Ligatures: shaping per word keeps them; a soft hyphen splits a word.
    println!("\nG. ligatures");
    for (family, text) in [
        ("Noto Serif", "office efficient affluent"),
        ("Noto Serif", "of\u{AD}fice ef\u{AD}ficient"),
        ("DejaVu Serif", "office efficient affluent"),
    ] {
        let list = attrs(family);
        let line = shape(&mut fonts, text, &list);
        let glyphs: usize = line
            .spans
            .iter()
            .flat_map(|span| &span.words)
            .map(|word| word.glyphs.len())
            .sum();
        println!(
            "   {family}, {:?}: {} characters, {glyphs} glyphs",
            text,
            text.chars().count()
        );
        for width in [400.0, 60.0] {
            let (lines, mismatches, largest) = compare(&mut fonts, text, &list, width);
            println!(
                "     width {width}: {lines} lines, {mismatches} differ from COSMIC Text, largest x difference {largest:.3}"
            );
        }
    }
}
