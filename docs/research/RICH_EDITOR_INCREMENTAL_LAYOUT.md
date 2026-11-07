# Rich Editor Incremental Layout Investigation

**Date:** 2026-10-09  
**Decision scope:** the feature-gated rich editor's measured pointer, caret,
scroll, and edit paths. This does not reopen the editor architecture or advance
pagination, tables, or frames.

## Measured path and root cause

The pointer path is `Surface::update` -> `document_hit` -> `Action::Click` ->
`AmiProApp::update_prototype`. Raw hit testing uses the already-shaped renderer
paragraph. A distinct drag hit updates only the semantic selection, but Iced
then constructs a new view. Before this investigation, `surface()` eagerly ran
`project_paragraph` for every paragraph and rebuilt every span vector. The next
widget `layout` pass unconditionally called `Renderer::Paragraph::with_spans`
for every paragraph and replaced `SurfaceState::paragraphs`. Selection, caret,
focus, outer-scroll position, and cursor blinking do not change any shaping
input, so all of that work was invalidation without a content or geometry
change.

Typing, replacement, paragraph split/join, character formatting, paragraph
style changes, undo, and redo replace or mutate the semantic `Document` and do
need fresh projections. A text-width or display-scale change also changes
wrapping/metrics. Switching tabs can reuse the same widget-tree position, so a
cache must identify the document session and must not rely on a borrowed
document address. Outer scrolling only translates/clips the already-shaped
surface and does not alter paragraph-local layout.

The traced event paths are:

| Interaction | Before the correction | Required invalidation |
|---|---|---|
| Pointer motion outside a drag | Cursor/mouse interaction only | None |
| Press or distinct drag hit | Hit test existing shaped geometry, publish `Click`, update selection, rebuild the declarative view, then eagerly reproject and reshape every paragraph | Selection highlight/caret redraw only |
| Left/Right/Home/End or shaped Up/Down | Publish navigation, update caret/anchor and the mirrored Iced cursor, rebuild the view, then reproject and reshape every paragraph | Caret/selection redraw only |
| Outer page scroll | Store the active tab's scroll offset and translate/clip the surface; the following layout still reprojected and reshaped everything | None; paragraph-local geometry is unchanged |
| Text insertion, replacement, split/join, Undo/Redo | Apply an editing command, replace the semantic document snapshot, rebuild fallback Iced content, and refresh derived rich layout | Full projection and shaping |
| Character or paragraph formatting | Mutate or replace semantic formatting and refresh derived rich layout | Full projection and shaping |
| Width, display scale, or renderer text defaults | Re-enter widget layout with changed shaping constraints | Full projection and shaping |
| Tab switch | Reuse the widget-tree slot for a different `DocumentSession` | Rebuild for the new session; never reuse another tab's geometry |

Iced still reconstructs the lightweight `Surface` value during declarative view
updates. The correction changes that reconstruction from an eager document
projection into a borrowed document reference plus cache key; shaped paragraphs
remain in persistent widget-tree state.

The retained optimized synthetic baseline (250 wrapped, UTF-8, mixed-format
paragraphs) measured about 31.9 ms for complete projection plus reshaping,
0.351 microseconds per hit, and 0.041 ms for no-op draw traversal. A second
baseline used the private, read-only `AP31NAV.SAM` fixture (315 imported
paragraphs; proprietary bytes remain outside Git). Two runs measured:

| Cost | Run 1 | Run 2 |
|---|---:|---:|
| Projection/span construction | 0.119 ms/pass | 0.121 ms/pass |
| COSMIC Text paragraph construction/shaping | 9.624 ms/pass | 9.195 ms/pass |
| Current selection controller/document clone | 14.206 us/action | 14.304 us/action |
| Hit test | 0.305 us/call | 0.452 us/call |
| No-op draw traversal | 0.001 ms/pass | 0.001 ms/pass |

The fixture-dependent absolute reshape time differs from the synthetic test,
but both isolate the same cause: full `with_spans` paragraph construction and
shaping dominates selection-only work. Span projection is the next largest
layout component but is still roughly two orders of magnitude smaller on this
SAM. Controller cloning, hit testing, and traversal are not the first target.

## Version-matched source findings

The checked-in lockfile pins libcosmic/Iced 1.0.0/0.14 to revision
`dbc62f2d899cdbfbe43d170689ddc0290db663f4` and COSMIC Text 0.19.0. The exact
pinned Iced `graphics::text::Paragraph::with_spans` implementation creates a
new COSMIC Text `Buffer`, sets its rich text and bounds, calls
`shape_until_scroll`, and stores the result behind an `Arc`. Cloning the
finished paragraph is therefore cheap, while calling `with_spans` again is the
expensive operation. Its `compare` method distinguishes a font-system or shape
parameter change from a bounds-only change.

The pinned Iced text editor provides the relevant ownership model: its
`Content` retains a COSMIC Text editor across declarative view/widget rebuilds;
widget layout calls `Editor::update`, which changes font, metrics, wrapping, or
bounds only when those inputs differ and then invokes `shape_as_needed`.
COSMIC Text 0.19 likewise retains shaping/layout on each `BufferLine`. Its lazy
dirty flags distinguish text replacement, reshaping, relayout, and scrolling;
`shape_until_scroll` returns without work when nothing is dirty. `set_size`
marks relayout only when dimensions actually change. These APIs support reuse,
but the prototype discarded the reusable object one layer above them.

Primary sources:

- [COSMIC Text 0.19 `Buffer` documentation](https://docs.rs/cosmic-text/0.19.0/cosmic_text/struct.Buffer.html)
- [COSMIC Text 0.19 `BufferLine` documentation](https://docs.rs/cosmic-text/0.19.0/cosmic_text/struct.BufferLine.html)
- [COSMIC Text 0.19 changelog](https://github.com/pop-os/cosmic-text/blob/main/CHANGELOG.md)
- [libcosmic/Iced paragraph API](https://pop-os.github.io/libcosmic/cosmic/iced/daemon/program/graphics/text/struct.Paragraph.html)
- Pinned source inspected locally from Cargo's checkout:
  `iced/graphics/src/text/paragraph.rs`,
  `iced/graphics/src/text/editor.rs`, and
  `iced/widget/src/text_editor.rs` at revision `dbc62f2d`.
- [COSMIC Edit](https://github.com/pop-os/cosmic-edit) uses COSMIC Text 0.19
  and libcosmic rather than introducing a separate layout cache contract.

Xi's [minimal invalidation design](https://xi-editor.io/docs/rope_science_12.html)
provides a useful longer-term model: preserve valid line results, explicitly
invalidate changed ranges, prioritize the visible viewport, and update cursor
layers without rerendering text. Its own documentation also notes that this
machinery is elaborate and that full redraw is reasonable when rendering is
already cheap. Adopting Xi-style deltas or incremental rewrap now would require
new edit-range/version plumbing and y-offset propagation beyond the measured
problem.

## Candidates, expected benefit, and risk

1. **Retain complete shaped paragraphs in Iced widget-tree state.** Key them by
   explicit session ID, semantic layout generation, display scale, resolved
   width/height, and renderer text defaults. Selection/caret/scroll rebuilds
   become cache hits; edits and geometry changes rebuild conservatively. This
   removes both eager projection and `with_spans` from the hot path. Expected
   benefit is nearly all of the measured 9-32 ms selection-only layout cost.
   Main risk is stale cache state, addressable with explicit invalidation and
   focused tests. **Recommended.**
2. **Per-paragraph dirty generations and incremental y-offset repair.** This
   could reduce edit-time reshaping to changed paragraphs, but split/join,
   wrapping, paragraph spacing, style inheritance, and following y offsets
   expand the correctness surface. It is valuable only after selection-only
   reuse is measured. **Defer.**
3. **Move editing into one persistent COSMIC Text `Editor`/`Buffer`.** This
   gains its line-level dirty machinery but risks a second editable document
   model and does not directly represent the existing semantic paragraph
   spacing/style contract. **Reject for this correction.**
4. **Viewport-only/Xi-style line cache.** This can cap very-large-document work,
   but the current surface needs stable paragraph heights for scroll and hit
   mapping. It adds eviction, height estimation, and delta protocols before the
   simple reuse case is solved. **Defer.**
5. **Iced canvas/geometry or renderer glyph caching.** These caches reduce
   raster/draw cost after layout. The measured no-op traversal is negligible,
   and `with_spans` shapes before renderer caching can help. **Reject as the
   root-cause fix.**

## Implementation and verification decision

Implement candidate 1 only. Store renderer-generic shaped paragraphs and
underline metadata in `SurfaceState`, which already persists across Iced view
reconstruction. Add a per-session semantic layout generation and bump it on
plain edits, rich replacement/split/join, formatting, style application,
document replacement, undo, and redo. Include session identity so tab changes
cannot reuse another document's cache. Rebuild when generation, display scale,
resolved bounds, renderer defaults, or Iced font-system comparison says layout
is stale. Do not put selection, caret, focus, scroll offset, or history depth in
the key.

Acceptance requires stale-cache regressions for selection-only reuse, width and
scale changes, text replacement, mixed formatting/style changes, undo/redo,
scrolling, and independent tabs; the real shaped-widget pointer regression;
same-SAM before/after measurements with projection, shaping, controller, hit,
and draw costs separated; the complete core/COSMIC/SAM/STY automated suites;
and a final physical COSMIC drag/type/scroll retest. The production editor
remains the default and the rich surface remains feature-gated.

## Implemented and measured result

The recommended state cache is implemented. The synthetic 250-paragraph cold
cost remains 27.026-30.363 ms/pass, as expected, while its warm selection-only
widget layout measures 0.039-0.040 us/pass and surface construction 0.017
us/pass. On the same private 315-paragraph `AP31NAV.SAM`, the pre-change
complete pass was 9.351-9.432 ms (the detailed split attributed 9.195-9.624 ms
to shaping and 0.119-0.121 ms to projection). Verified post-change runs measured
8.323-8.612 ms cold, 0.017-0.018 us to construct the lightweight surface, and
0.039-0.044 us for warm selection layout. Controller-plus-document-clone cost
was 11.698-12.859 us/action in those final runs and is now the largest isolated
selection cost, but it is not large enough to justify widening this correction.

The private-fixture measurement is reproducible without committing the source
document:

```sh
cd app/cosmic
AMIPRO_PERF_SAM=/private/path/AP31NAV.SAM \
  cargo test --offline --release --all-features \
  rich_editor_pointer_performance_baseline -- --ignored --nocapture
```

All cache and real-widget regressions pass. The complete core and COSMIC test
and strict-Clippy suites pass, and the private compatibility audit remains
160/160 SAM imports with exact native-model round trips and 63/63 STY parses.
Physical COSMIC testing now reports normal-feeling `README31.SAM` typing, so
the primary P0 responsiveness defect is resolved. Untested drag, scrolling,
history, caret, and multi-tab regression checks remain pending rather than
being inferred as passed.

## Screenshot follow-up observations

The acceptance screenshot also raised four separate observations. They are not
part of the cache correction and are not established importer defects:

- status-bar text and spacing need a focused visual review;
- apparently malformed text in `README31.SAM` needs comparison with the
  original source document and, if that is inconclusive, Ami Pro running in the
  authorized Windows XP environment;
- excessive text weight may be an interface-font issue or incorrect imported
  formatting and must be distinguished with source/reference evidence;
- overall interface compactness and alignment need a focused COSMIC review.

No SAM importer behavior was changed from screenshot evidence. These items stay
open until the source document and, where necessary, original application have
been compared.
