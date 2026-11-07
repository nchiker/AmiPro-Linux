# Rich Editor Pointer Performance Baseline

**Date:** 2026-10-09  
**Scope:** Bounded follow-up to the physically accepted multiparagraph
mouse-selection correction. This is not an architecture review.

## Reproduction

The ignored test creates 250 independently authored paragraphs with wrapping,
UTF-8, mixed 16-point bold underlined runs, and the same COSMIC Text-backed Iced
paragraph implementation used by the graphical renderer:

```sh
cd app/cosmic
cargo test --offline --release --all-features \
  rich_editor_pointer_performance_baseline -- --ignored --nocapture
```

Initial optimized result on the development host:

| Operation | Result |
|---|---:|
| Paragraph projection plus complete reshape/layout | 32.588 ms per pass |
| Renderer/document hit test | 0.385 µs per call |
| Widget draw traversal with no-op raster backend | 0.036 ms per pass |

A repeat run after the interface integration measured 31.862 ms (about
31.9 ms) per projection/reshape pass, 0.351 µs per hit test, and 0.041 ms per
draw traversal. The conclusion is unchanged: complete projection and reshaping
dominates pointer-drag work.

These are diagnostic local measurements, not cross-machine performance promises.
The fixture and command are retained so future changes can be compared on the
same host.

## Finding and bounded next action

Raw pointer hit testing is not the observed bottleneck. Each distinct drag hit
publishes a selection action; the application rebuilds its view, and the custom
widget currently rebuilds paragraph projections and reshapes every paragraph in
`layout` even though the document content and width did not change. Duplicate
move events within one glyph can also publish the same hit repeatedly.

Duplicate drag-hit messages are now suppressed. The next task is research, not
implementation: use the project's reference-driven process to examine caching
and incremental-layout approaches in existing Rust text editors, then compare
applicable invalidation and cache-ownership designs against this measured path.
Preserve the current architecture and feature gate unless focused evidence
demonstrates a need for a narrow change. Re-run this baseline and the real-widget
selection regression after any later implementation.

## Incremental-layout result

The focused investigation in
`docs/research/RICH_EDITOR_INCREMENTAL_LAYOUT.md` selected a narrow reuse fix.
Renderer paragraphs and underline metadata now remain in the Iced widget tree,
keyed by explicit document-session identity, semantic layout generation,
display scale, resolved bounds, and renderer defaults. Selection, caret, focus,
and outer scrolling reuse the shaped paragraphs; document edits/formatting,
undo/redo replacements, tab changes, width, and display scale invalidate them.
The default editor is unchanged and the rich surface remains feature-gated.

The benchmark now distinguishes cold projection/reshape, warm surface
construction, warm selection-only layout, the current selection controller
plus document clone, hit testing, and draw traversal. With the same synthetic
250-paragraph fixture used above, the post-change optimized run measured:

| Operation | Post-change result |
|---|---:|
| Cold projection plus complete reshape | 27.026-30.363 ms/pass |
| Warm surface construction | 0.017 us/pass |
| Warm selection-only widget layout | 0.039-0.040 us/pass |
| Selection controller plus document clone | 28.306-37.390 us/action |
| Renderer/document hit test | 0.280-0.317 us/call |
| No-op draw traversal | 0.029 ms/pass |

Cold shaping is intentionally unchanged. The former 31.862 ms selection-time
projection/reshape is no longer on the warm selection path. These very small
microbenchmarked warm numbers describe only the isolated functions and should
not be treated as end-to-end input latency.

A second before/after measurement used the private read-only `AP31NAV.SAM`
fixture (315 imported paragraphs), selected because it is representative and
large enough to exercise the physically reported path. Its bytes were neither
copied nor changed:

| Operation | Before (two runs) | After (two final runs) |
|---|---:|---:|
| Projection/span construction | 0.119-0.121 ms/pass | absent on warm path |
| Complete projection/reshape | 9.314-9.553 ms/pass | absent on warm path |
| Cold projection/reshape | n/a | 8.323-8.612 ms/pass |
| Warm surface construction | n/a | 0.017-0.018 us/pass |
| Warm selection-only widget layout | n/a | 0.039-0.044 us/pass |
| Selection controller plus document clone | 14.206-14.304 us/action | 11.698-12.859 us/action |
| Hit test | 0.305-0.452 us/call | 0.291-0.303 us/call |
| No-op draw traversal | 0.001 ms/pass | below 0.001 ms/pass |

The earlier combined same-SAM runs were 9.351 and 9.432 ms/pass; the more
detailed pre-change split showed shaping at 9.624 and 9.195 ms/pass plus about
0.12 ms projection. Run the same private-fixture comparison with:

```sh
cd app/cosmic
AMIPRO_PERF_SAM=/private/path/AP31NAV.SAM \
  cargo test --offline --release --all-features \
  rich_editor_pointer_performance_baseline -- --ignored --nocapture
```

Automated cache regressions cover stale text/formatting, width, scale,
selection-highlight drawing, outer scroll, and independent tab identity. The
real shaped-widget wrapped mixed-format drag regression also passes. Physical
COSMIC dragging, typing, and scrolling remain required before performance
acceptance.
