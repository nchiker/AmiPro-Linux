# Default Editor Typing Performance

**Date:** 2026-10-11
**Scope:** the accepted default editor (Iced `text_editor` with
`DocumentHighlighter`, no `rich-editor-prototype` feature). The P0 correction
at `bf0eaa5` applied only to the feature-gated rich surface; the default build
was never measured end to end.
**Result:** fixed at `8b78ecf`. Every measured editing operation on both
substantial imports went from tens of milliseconds to sub-millisecond medians.
Physical COSMIC confirmation is pending.

## Path trace

Each default-editor edit (`Message::Edit` → `apply_standard_action`) did the
following:

1. Snapshot the whole `Document` for undo.
2. Apply the action to the visible Iced content.
3. Rebuild the plain text and reconcile it into the model.
4. Call `DocumentSession::refresh_rich_layout`. In default builds this rebuilt
   a complete COSMIC Text `DocumentLayout` and shaped every paragraph.

The only consumer of that layout was a `debug_assert!` in `view()` comparing
paragraph counts. Undo and redo also replaced the visible content with
`Content::with_text`, which rebuilt and reshaped every editor line.

Iced re-highlights only from the highlighter's current line to the last
visible line. The `DocumentHighlighter` settings rebuild in `view` is
therefore bounded by viewport position rather than document length, and it
measured well under 1 ms.

## Workload

`default_editor_typing_performance_baseline` lives in `app/cosmic/src/main.rs`,
test module `default_editor_performance`. It is ignored by default and runs in
release mode only.

- **Fixtures:** a new empty document, plus the private SAM named by
  `AMIPRO_PERF_SAM`. With no SAM given it uses a synthetic 350-paragraph
  document.
- **Positions:** a caret in the middle of the first, middle, and last
  paragraphs. Before measuring, the editor is scrolled so the edited line is
  visible.
- **Operations:** 40 samples each of insertion, Backspace, three-character
  selection replacement, Undo, and Redo.
- **Per sample:** `Application::update(message)`, then one headless frame.
  The frame builds `view()`, diffs the **retained** widget tree, lays out the
  real application view at 1280×800, and records draw primitives with the
  pinned tiny-skia renderer after `Renderer::reset`.
- **Reported:** total, update, view, layout, and draw, each as median/p95/worst.

Reproduce with:

```sh
cd app/cosmic
AMIPRO_PERF_SAM=/private/path/README31.SAM \
  cargo test --offline --release default_editor_typing_performance_baseline \
  -- --ignored --nocapture
```

**Host:** Intel i7-10610U (8 threads), 15 GiB RAM, Pop!_OS with cosmic-comp
1.0.0, `powersave` governor, release profile.

**Load:** emulator processes from an unrelated project were running throughout
(load average about 4–6). Absolute numbers carry that noise. Before/after
ratios and the interleaved A/B below are the reliable comparisons.

## Results

Ranges cover the start, middle, and end positions. Values are milliseconds,
median / p95.

### `README31.SAM` (344 imported paragraphs)

| Operation | Before (`7f12d24`) | After (`8b78ecf`) |
|---|---|---|
| Typing | 52.1–55.1 / 58.8–62.1 | 0.58–0.62 / 0.95–1.04 |
| Backspace | 56.3–58.1 / 63.9–66.7 | 0.58–0.59 / 0.61–0.94 |
| Selection replacement | 57.3–61.0 / 63.0–68.3 | 0.58–0.60 / 0.64–0.96 |
| Undo | 79.1–85.6 / 90.7–95.2 | 0.45–0.67 / 0.69–1.12 |
| Redo | 81.0–85.6 / 89.9–91.9 | 0.43–0.66 / 0.70–0.90 |

Before the fix, `update` accounted for 51–60 ms of each typing sample. View,
layout, and draw together stayed under 1 ms. Undo/redo layout and draw each
cost about 1–2.5 ms because the whole content was reshaped.

### `AP31NAV.SAM` (315 paragraphs) and the empty document

After the fix, every operation on `AP31NAV.SAM` measures 0.38–0.52 ms median,
with p95 at or below 0.87 ms. The empty document measures 0.27–0.35 ms median
before and after.

### Intermediate step

Removing only the dead layout brought README31 typing to 0.58–0.66 ms. Undo
and redo were still 23–26 ms, because `Content::with_text` cost about 20 ms in
`update` plus a full reshape. Undo/redo now use the same semantic delta
synchronizer as the rich editor's hidden content.

## Changes

- Removed the per-session `DocumentLayout` and its per-edit rebuild. This also
  removes one `FontSystem` construction, a system font scan, per opened
  document in both builds. `rich_buffer.rs` and its tests remain as the dormant
  derived-layout foundation.
- Generalized the rich editor's `synchronize_prototype_fallback` into
  `synchronize_editor_text`. It replaces only the semantic text delta and
  falls back to a full rebuild if a boundary cannot be mapped. Both builds now
  use it for history restores.
- Interleaved A/B of `rich_editor_typing_performance_baseline` on README31
  (`96aeda4` vs. the fix, five alternating runs each), as median of run
  medians: insert 0.511 vs 0.483 ms, sustained typing 0.529 vs 0.494 ms, undo
  0.291 vs 0.279 ms. There is no regression.

## Correctness coverage

`history_restores_editor_text_by_delta_and_keeps_it_in_sync` runs in both
builds. Through `update` it performs:

- a multi-line Unicode paste;
- a cross-paragraph selection delete;
- two Undo and two Redo steps;
- a further edit after undo.

After every step it asserts that the visible editor text equals the model's
plain text, the model validates, and the restored caret receives subsequent
typing. The existing default-editor history, tab isolation, formatting, and
clipboard tests continue to pass.

## Limitations

- The headless frame records draw primitives but does not rasterize or
  present. GPU rendering, compositor latency, and input-method delivery are
  not included.
- Physical COSMIC typing, undo, and scrolling in `README31.SAM` with the
  default editor remain a required physical check.
- 40 samples per operation make p99 indistinguishable from the worst case.
