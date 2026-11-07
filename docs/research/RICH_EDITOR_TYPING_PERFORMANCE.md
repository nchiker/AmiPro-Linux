# Rich Editor Typing Performance

**Date:** 2026-10-10  
**Scope:** the feature-gated rich editor's unacceptable insertion latency in
substantial imported SAM documents. The accepted default editor and the
semantic document architecture remain unchanged.

## Reproduction and path trace

The initiating physical report was specific: typing in a new document was
normal, while inserting into an existing paragraph of the approximately
18-page `README31.SAM` was unacceptably slow. The prior layout cache fixed
selection/caret/scroll-only rebuilds; every text edit intentionally changes its
generation and therefore still takes the cold path.

Before the correction, the complete character path was:

`Surface::update` -> `Action::Insert` -> `AmiProApp::update_prototype` -> clone
the semantic `Document` into a short-lived `amipro_editing::Editor` -> apply an
atomic range replacement -> clone the pre-edit document into history -> clone
the edited document back -> rebuild `text_editor::Content` from
`Document::plain_text()` -> rebuild and shape the separate `DocumentLayout` ->
increment the generation -> rebuild the declarative view -> project and shape
every paragraph in `Surface::layout` -> draw.

The pinned Iced 0.14 implementation makes `Content::with_text` allocate a new
COSMIC Text buffer, call `set_text`, and immediately call
`shape_until_scroll`. `DocumentLayout::refresh` independently builds another
COSMIC Text buffer and also calls `shape_until_scroll`. The rendered surface
then calls `Paragraph::with_spans` for every paragraph. Thus one character pays
for three complete text-layout passes before it becomes visible. Document
cloning, plain-text reconstruction, projection, widget state, and draw must be
measured separately, but source inspection already establishes that the two
hidden layouts are redundant in the feature-gated surface.

## Applicable reference mechanisms

- COSMIC Text 0.19 retains dirty state per `BufferLine`; an edit need not
  replace or reshape an entire buffer.
- The pinned Iced editor retains its COSMIC Text editor and applies
  `Action::Edit` incrementally. Replacing `Content` discards that mechanism.
- The prior investigation established that Iced renderer paragraphs are cheap
  to retain and expensive to recreate through `with_spans`.
- COSMIC Edit relies on retained COSMIC Text state; Xi's minimal-invalidation
  model likewise keeps unchanged line results. Neither requires an editor or
  model rewrite here.

## Candidates and decision before correction

1. **Synchronize the hidden fallback content with the same local edit, skip the
   unused derived-buffer reshape in prototype builds, and retain unchanged
   renderer paragraphs.** Compare each cached paragraph with its semantic
   paragraph and resolved style when the document generation changes; reshape
   only changed paragraphs and repair following vertical offsets. Rebuild all
   paragraphs on session/geometry/font changes or paragraph-count changes.
   This preserves conservative invalidation while making ordinary insertion,
   deletion, and same-paragraph replacement local. **Recommended.**
2. Persist `amipro_editing::Editor` as the application model. This could remove
   document clones, but changes history ownership and the accepted command
   boundary. **Defer.**
3. Add a rope, viewport paginator, arena, or GPU cache. None addresses the three
   observed CPU layout passes and each widens the architecture. **Reject.**

The selected correction is candidate 1. It is deliberately paragraph-level:
split/join and paragraph-count-changing replacements may conservatively rebuild
the surface, while the reported existing-paragraph typing path must reshape one
paragraph. Tests must cover formatting, Unicode, selection replacement,
scrolling, width/scale, tab identity, undo/redo, and stale-cache behavior.

## Measurements

The correction is implemented as selected. After every semantic edit, the
retained hidden Iced content receives one UTF-8-safe common-prefix/suffix
replacement derived from the old and new document text rather than being
rebuilt. Deriving the delta from the semantic result also preserves its exact
CRLF/CR normalization and Rust-character deletion boundary instead of assuming
that Iced's grapheme edit semantics are identical. Prototype builds no longer
refresh the unused `DocumentLayout`; and a document generation change compares
cached semantic paragraphs/styles and reshapes only changed paragraphs.
Session, paragraph-count, geometry, scale, and renderer-font changes still
rebuild conservatively. Undo/Redo use the same synchronization, and the shared
menu/toolbar history path restores the rich selection from its saved Iced
cursor before any following edit. Explicitly moving to a caret now clears
Iced's previous selection: pinned Iced `move_to` does not clear a selection when
its `Cursor.selection` is `None`, which otherwise caused later edits to replace
stale ranges.

Release-mode measurements ran on Linux 7.1.5, an Intel i7-10610U (4 cores/8
threads), Rust 1.99.0, pinned libcosmic/Iced revision `dbc62f2`, and COSMIC Text
0.19. Each controller category samples beginning, middle, and end paragraphs;
ordinary categories have 36 samples, sustained typing 40, Undo 184, and Redo
36. Layout samples contain 300 alternating local edits. Reported values are
median / p95 / worst.

| `README31.SAM` (344 paragraphs) | Median | p95 | Worst |
|---|---:|---:|---:|
| Empty-document sustained typing | 0.004 ms | 0.005 ms | 0.023 ms |
| Insert controller/model/widget-state update | 0.294 ms | 0.432 ms | 0.491 ms |
| Backspace | 0.282 ms | 0.432 ms | 0.444 ms |
| Selection replacement | 0.288 ms | 0.432 ms | 0.436 ms |
| Sustained typing | 0.285 ms | 0.300 ms | 0.478 ms |
| Undo | 0.168 ms | 0.257 ms | 0.291 ms |
| Redo | 0.158 ms | 0.239 ms | 0.246 ms |
| Edited-paragraph projection/layout | 0.040 ms | 0.047 ms | 0.126 ms |

The same fixture measured 0.042 ms for whole-document span projection, 23.960
ms for cold whole-document rich projection/reshape, 0.044 microseconds for an
unchanged warm layout, and 0.001 ms per no-op renderer draw traversal. Diagnostic
document clone and plain-text projection medians were 0.067 ms and 0.011 ms;
they remain small constituents of the controller timing. Diagnostic
reconstruction of the removed operations measured 17.558 / 20.754 / 50.188 ms
for `Content::with_text` and 84.695 / 93.262 / 97.674 ms for the unused derived
layout rebuild. These are isolated components on the same current build, not a
claim that component medians sum to an exact historical event latency. Source
tracing establishes that both components ran on every pre-correction edit.
Selection-only controller work including its document clone measured 27.232
microseconds and raw hit testing measured 0.537 microseconds per call.

| `AP31NAV.SAM` (315 paragraphs) | Median | p95 | Worst |
|---|---:|---:|---:|
| Insert | 0.147 ms | 0.202 ms | 0.255 ms |
| Backspace | 0.140 ms | 0.189 ms | 0.318 ms |
| Selection replacement | 0.138 ms | 0.182 ms | 0.303 ms |
| Sustained typing | 0.144 ms | 0.253 ms | 0.263 ms |
| Undo | 0.077 ms | 0.126 ms | 0.255 ms |
| Redo | 0.074 ms | 0.119 ms | 0.185 ms |
| Edited-paragraph layout | 0.017 ms | 0.040 ms | 0.054 ms |

Its cold whole-document rich reshape remains 8.790 ms, while the removed
fallback and derived-layout rebuilds measured 5.630 ms and 50.074 ms median.
Document clone and plain-text projection medians were 0.038 ms and 0.006 ms.
Its unchanged warm layout measured 0.044 microseconds and hit testing 0.391
microseconds per call.
The retained benchmark commands are:

```sh
cd app/cosmic
AMIPRO_PERF_SAM=/private/path/README31.SAM \
  cargo test --offline --release --all-features \
  rich_editor_typing_performance_baseline -- --ignored --nocapture
AMIPRO_PERF_SAM=/private/path/README31.SAM \
  cargo test --offline --release --all-features \
  rich_editor_pointer_performance_baseline -- --ignored --nocapture
```

Private SAM bytes remain outside Git. The synthetic fallback is 350 paragraphs
and permits comparison when the corpus is unavailable. Its final retained run
measured 0.115 ms median insertion, 0.127 ms sustained typing, and 0.104 ms
edited-paragraph layout; the synthetic pointer fixture retained its deliberately
heavier 29.513 ms cold whole-document reshape while local edit layout was 0.104
ms median.

## Limitations and acceptance

The headless path separately measures controller/model/history/fallback-widget
state, rich projection/shaping, hit testing, and renderer draw submission. It
does not measure COSMIC/libcosmic event-loop scheduling, GPU rasterization,
compositor presentation, or the instant pixels become visible; the stage
medians imply approximately 0.33 ms of measured CPU work for a README31
character edit, but this is not a physical latency claim. Physical COSMIC
testing on 2026-10-10 reported that typing in `README31.SAM` now feels normal,
which resolves the primary P0 responsiveness defect. Drag, scrolling,
menu/toolbar history, caret, and multi-tab regression checks not included in
that report remain pending and are not inferred as passed. P0 is not reopened
unless one of those checks identifies a problem.

The automated suite covers tab identity, formatting invalidation, downstream
vertical-offset repair, and prototype edits in separate focused regressions.
It does not yet combine rich edits across two tabs in one app-level test, assert
a following paragraph's hit geometry after a first-paragraph height change in
one dedicated case, or combine prototype formatting with Undo in one test.
Independent review found no defect in those shared mechanisms; these are
coverage limits rather than claimed physical evidence.
