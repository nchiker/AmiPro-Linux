# Line-Breaking Spike (Phase 2.5a)

**Date:** 2026-10-11. **Code:** `experiments/page-layout-spike` (isolated,
not part of the workspace or the gates). **Decision:** recorded in
`DECISIONS.md` ("Line breaking over COSMIC Text words").

## Question

COSMIC Text's `Buffer` wraps every line of a paragraph to one width. Ami Pro's
layout needs more (`ARCHITECTURE.md`, "Layout engine"):

- a different width for each line, for text that wraps around frames;
- inline boxes for Flow with text frames (U+FFFC);
- U+2028 (hard return) as a forced break;
- U+00AD (discretionary hyphen) as a break point that shows a hyphen only when
  the line breaks there.

Can the project break lines itself over the words of a COSMIC Text
`ShapeLine`, or should it adopt Parley 0.12, which offers per-line widths and
inline boxes natively (`PAGE_FRAME_REFERENCE_RESEARCH.md`)? The runbook
bounds the spike to one slice and compares with Parley only if a case fails.

## Method

`ShapeLine::new` shapes a paragraph into bidi spans of words. Each word ends
at a UAX #14 break opportunity, and its glyphs carry byte ranges and
advances. The spike (about 250 lines, `src/lib.rs`):

1. collects the words in logical order with their widths and three flags: an
   object (U+FFFC, which takes the object's width), a forced break (ends with
   U+2028), and a soft hyphen (ends with U+00AD);
2. breaks greedily with a width supplied per line. Blanks at a line end hang
   outside the width, a word wider than its line is placed alone, and a line
   that ends at a soft hyphen adds the hyphen's width, backing off a word if
   it does not fit;
3. places each line's glyphs left to right, reversing right-to-left runs of
   words (rule L2 for levels 0 and 1).

As an oracle, the same `ShapeLine` was laid out by COSMIC Text itself
(`ShapeLine::layout`, `Wrap::Word`) at uniform widths, and the break
positions and glyph x positions were compared.

Fonts: Liberation Serif (metric-compatible with Times New Roman) at 16 px for
Latin text; Noto Sans for bidi; Noto Serif and DejaVu Serif for ligatures.
Host: Intel Core i7-10610U, 8 threads, `powersave` governor, release build.

## Results

| Case | Result |
|---|---|
| A. `README31.SAM`, 337 paragraphs without objects or special characters, at 624, 300, and 120 px (4,457 lines) | Every break position identical to COSMIC Text's; glyph x positions identical (largest difference 0.000 px). COSMIC Text also places a line's trailing spaces as glyphs; the spike leaves them out, so the comparison ignores trailing whitespace. |
| B. Widths 400, 200, 200, 400… px (text beside a frame) | Every line fits its own width and is greedy (the next word would not fit). |
| C. U+FFFC as 90 px boxes, alone and two in a row | Each object is its own word, so a box never splits and breaks fall before or after it. |
| D. U+2028 | Each forces a break even when the line has room; the rest wraps normally. |
| E. U+00AD | Zero width when unused (the text measures 419.42 px with or without the soft hyphens); a line ending at one shows a 5.33 px hyphen, counted against the width. |
| F. Hebrew and Arabic runs in a left-to-right paragraph, at 500, 260, and 160 px | Identical to COSMIC Text: break positions and glyph positions. |
| G. Ligatures (Noto Serif, DejaVu Serif) | Words keep their ligatures (25 characters, 19 glyphs in Noto Serif), identical to COSMIC Text. A soft hyphen splits a word into separately shaped pieces, so a ligature across it is lost even when the line does not break there (`of­fice`); COSMIC Text's own wrapping behaves the same way. |
| Timing, all 344 `README31.SAM` paragraphs | Shaping 14–17 ms once; breaking with varying widths 0.57–0.70 ms; COSMIC Text's own layout at one width 1.5–1.6 ms. |

No case failed, so Parley was not tried.

## Conclusions

- The project's own breaker over COSMIC Text words works for every required
  case and reproduces COSMIC Text's wrapping exactly where they overlap. It is
  also cheaper than COSMIC Text's layout, which matters for re-pagination.
- Shaping dominates: about 50 µs per paragraph on average. The per-paragraph
  shaping cache of D5 (2.5c) is therefore the main performance lever, and
  breaking can be redone freely when widths change.
- The engine (2.5b) breaks lines in `amipro-layout` over a word list behind
  the shaper trait (width, blank, object, forced break, soft hyphen, bidi
  level, byte range per word). The app's COSMIC Text shaper fills it, and core
  tests use a fixed-advance shaper.

## Limits and follow-ups

- Bidi reordering in the spike handles one level of right-to-left runs inside
  a left-to-right paragraph. The engine must apply rule L2 over the span
  levels COSMIC Text reports (right-to-left paragraphs, numbers inside
  right-to-left text). That is a known algorithm, not a reason to change
  libraries.
- Ligatures across a soft hyphen are lost (as in COSMIC Text). Acceptable for
  now; a future fix would shape the joined word and fall back only when the
  line breaks at the hyphen.
- A word wider than its line overflows. Ami Pro's behaviour for such words
  (character breaking) is not yet observed.
- Justification and tab stops are layout steps after breaking, not part of
  this spike.
