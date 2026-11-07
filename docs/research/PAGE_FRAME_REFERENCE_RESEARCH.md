# Page, Layout, and Frame Model: Reference Research

**Date:** 2026-10-11. **Purpose:** the reference checkpoint that `AGENTS.md`
requires before substantial pagination and frame work, as listed in
`docs/Design/PAGE_FRAME_MODEL_BRIEF.md` section 5. Each question ends with a
concrete reuse answer. The decision that uses these answers is the
2026-10-11 Phase 2.2 entry in `DECISIONS.md`; the resulting architecture is
in `ARCHITECTURE.md`.

This is a narrow check, not a new engine audit. The 2026-10-09 reuse audit
(`WORD_PROCESSOR_ENGINE_REUSE_AUDIT.md`) already rejected adopting a whole
word-processing engine, and nothing here reopens that.

## 1. OpenDocument and LibreOffice Writer

**What they do.**

- ODF anchors a frame with `text:anchor-type` = `page`, `frame`,
  `paragraph`, `char`, or `as-char` (ODF 1.2 part 1, sections 19.754 and
  20.408). A page-anchored frame names its page number. Paragraph- and
  character-anchored frames are written inside their paragraph. An `as-char`
  frame "takes the place of a character" in the text (OpenOffice API,
  `TextContentAnchorType`).
- ODF page styles correspond to Ami Pro page layouts. ODF switches page
  style through a paragraph property (a master-page name with a break
  before the paragraph). Ami Pro switches through an inline page-break mark
  that names a layout and applies from the next page.
- Writer holds every text stream (body, headers and footers, frame
  contents, footnotes) as sections of one global node array. A position is
  a node index plus a character index.
- An `as-char` frame is a placeholder character in the paragraph
  (`CH_TXTATR_BREAKWORD`, U+0001) plus a text attribute without an end,
  `RES_TXTATR_FLYCNT`, at that index (`sw/inc/hintids.hxx`).
- Paragraph and character anchors are stored positions that Writer keeps
  up to date as text changes.
- `SwRootFrame::AssertFlyPages` "ensures that enough pages exist, so that
  all page bound frames and draw objects can be placed".

**Mapping to Ami Pro.**

| Ami Pro placement | Closest ODF anchor | Difference |
|---|---|---|
| Where placed | `page` + page number | none of substance |
| With para above | `paragraph` | Ami Pro writes the mark at the paragraph end and places the frame *below* the paragraph |
| Flow with text | `as-char` | none of substance |
| Repeat all pages, Repeat right/left | content of a page style | Ami Pro keeps them in the page layout and always paints them beneath page frames (V14) |
| — | `frame` (a frame anchored in a frame) | Ami Pro 3.1 has no counterpart: anchors occur only in the main text (user's guide chapter 15; all corpus marks) |

**Reuse answer: concepts only, no dependency.**

- Adopt the placeholder character plus attribute for inline objects (model
  decision D3).
- Adopt flow-qualified positions (D2), but address each flow separately
  rather than through one global node array, so that an edit in one flow
  never shifts positions in another.
- Adopt "add pages so that page-anchored frames have their page". Ami Pro
  does the same (V11, X06).
- Do not adopt stored character or paragraph anchors that every edit must
  update; that is the bug class D3 avoids.

## 2. COSMIC Text 0.19 (pinned through libcosmic and Iced)

Read from the registry source of `cosmic-text-0.19.0`:

- `Buffer::set_size(width, height)` and `ShapeLine::layout(font_size,
  width_opt, wrap, align, …)` take **one width for the whole paragraph**.
  There is no per-line width, no exclusion callback, and no inline box.
- Buffer lines split only at `\n` and `\r` (`line_ending.rs`). U+2028 does
  not split a buffer line, so a hard return stored as U+2028 cannot change
  the paragraph mapping in the current one-flow editors.
- `ShapeLine { spans: Vec<ShapeSpan> }`, `ShapeSpan { words: Vec<ShapeWord> }`,
  and `ShapeWord { blank, glyphs: Vec<ShapeGlyph> }`, with
  `ShapeGlyph { start, end, x_advance, … }`, are public. Words are cut at
  UAX #14 break opportunities (`unicode_linebreak`).

**Reuse answer.** Yes for shaping, font fallback, bidi analysis, and glyph
rendering: the renderer already uses COSMIC Text, so layout and drawing stay
on one font stack. No for page line breaking. One `Buffer` per flow or per
column region is not enough, because the available width changes within a
paragraph beside a wrap-around frame, and Flow with text frames are boxes
inside a line. The layout engine breaks lines itself from shaped words, with
a width for each line and boxes for inline frames.

## 3. Parley 0.12 (Linebender)

From the docs.rs API pages:

- `InlineBox { id, index, width, height, kind, baseline, vertical_align }`.
  `index` is a UTF-8 byte offset, which fits the U+FFFC inline objects of
  D3 directly.
- `BreakLines::break_next()` lays out one line at a time.
  `BreakerState::set_line_max_advance`, `set_line_x`, `set_line_y`, and
  `set_line_max_height` set each line's geometry, and `revert()` retries a
  line.

This is exactly the per-line width and inline-box contract the engine
needs.

**Cost.** Parley brings a second font and shaping stack (fontique, skrifa,
HarfRust) beside COSMIC Text. Iced's text renderer draws COSMIC Text
buffers, so Parley's output would need its own glyph-drawing path.

**Reuse answer: not now.** The engine's shaper boundary (`ARCHITECTURE.md`)
keeps Parley swappable. Phase 2.5 starts with a bounded spike of the
project's own line breaker over COSMIC Text shaped words. Parley is adopted
only if a concrete fixture fails there and passes with Parley, for example
bidi text, a ligature at a break, an inline frame, or a wrap band. This
keeps the existing decision to use Parley only for a measured gap.

## 4. Scribus (frames-first desktop publishing)

- Scribus keeps page items in one document-wide list in z-order, with the
  page as an item attribute. Layers sit above that.
- A Scribus group is a container item that holds its members.

Ami Pro differs:

- A group is a contiguous run of the page-frame order, on one page, with
  no nesting (Z03, Z04, V15).
- There are two fixed layers: layout frames always paint below page frames
  (V14).

**Reuse answer: concepts only.**

- Adopt one document-wide ordered list with the page as a frame attribute.
- Do not adopt group containers. A group ID on each member, with a
  contiguity rule, matches both the SAM format and Ami Pro's behaviour.
- Scribus is C++ under the GPL, so no code is reused.

## 5. Rust document crates

Briefly confirmed: no maintained Rust crate provides an editable page and
frame document model with anchors. WordCraft and rdocx (reuse audit
addendum) remain prior art only. No change.

## 6. Structural sharing for undo

Snapshot undo becomes cheap when documents share unchanged parts through
`std::sync::Arc` and `Arc::make_mut`. That needs no persistent-collection
crate (`im`, `rpds`). `AGENTS.md` forbids adding such dependencies
speculatively, and nothing measured calls for them.

## Conclusions used by the decision

1. Inline objects are a placeholder character plus an attribute (as in
   Writer); positions name their flow.
2. Pages are layout output, and the layout adds pages to host fixed frames
   (as in Writer and Ami Pro).
3. COSMIC Text shapes and renders. The project's layout engine breaks lines
   with per-line widths and inline boxes, behind a shaper boundary. Parley is
   the measured fallback.
4. Frames sit in one ordered list with the page as an attribute; groups are
   member IDs with a contiguity rule.
5. Undo uses `Arc` structural sharing, with no new dependency.

## Sources

- OASIS, *Open Document Format for Office Applications (OpenDocument) v1.2,
  Part 1*, sections 19.754 (`text:anchor-type`) and 20.408:
  <https://docs.oasis-open.org/office/v1.2/os/OpenDocument-v1.2-os-part1.html>
- OpenOffice API, `TextContentAnchorType`:
  <https://openoffice.org/api/docs/common/ref/com/sun/star/text/TextContentAnchorType.html>
- LibreOffice `SwRootFrame` (`AssertFlyPages`, `AssertPageFlys`):
  <https://docs.libreoffice.org/sw/html/classSwRootFrame.html>
- LibreOffice `sw/inc/hintids.hxx` (`CH_TXTATR_BREAKWORD`,
  `RES_TXTATR_FLYCNT`):
  <https://github.com/LibreOffice/core/blob/master/sw/inc/hintids.hxx>
- Parley 0.12 `InlineBox`, `BreakLines`, `BreakerState`:
  <https://docs.rs/parley/latest/parley/struct.InlineBox.html>,
  <https://docs.rs/parley/latest/parley/layout/struct.BreakLines.html>,
  <https://docs.rs/parley/latest/parley/layout/struct.BreakerState.html>
- COSMIC Text 0.19.0 source (`shape.rs`, `buffer.rs`, `line_ending.rs`) in
  the local Cargo registry.
- Ami Pro 3.1 user's guide chapters 14–16 (online-documentation SAM files)
  and the VM fixtures in `SAM_FORMAT_NOTES.md`.
