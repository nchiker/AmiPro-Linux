# BetterOffice Layout-Engine Proof of Concept

**Date:** 2026-10-09

**Time box:** one working day

**Decision:** **Defer the decision until a specific advanced-layout requirement demonstrates a measurable need.** Retain the current AmiPro-Linux architecture and production editing/layout path.

## Scope and isolation

This experiment evaluated BetterOffice's native Rust layout, text-measurement,
display-list, source-position, and hit-testing APIs. It did not evaluate a
browser embed and did not adopt BetterOffice's OOXML model or CRDT editor.

The implementation is in `experiments/betteroffice-layout-poc/`, which is
explicitly excluded from the root Cargo workspace. Neither the core production
workspace nor `app/cosmic` depends on it. The POC only borrows an immutable
`amipro_document::Document`; tests verify that projection leaves the source
document unchanged. The working COSMIC Text editor, document model, editing
commands, save path, and UI are untouched.

The committed POC pins the latest published Rust layout crate,
`betteroffice-docx-layout = =0.3.0`, and has its own lockfile. Source review used
BetterOffice commit `ac80b80dce5ce9257dedb24a598eec133064592c`, whose workspace
version is 0.4.3. The same five POC tests also compiled and passed against that
checkout through a temporary path dependency. No BetterOffice source, fonts,
fixtures, or generated artifacts were copied into this repository.

## Real API boundary

The useful native boundary is real and separable:

1. `measure_blocks(&mut [LayoutBlock], content_width, &MeasurementConfig)`
   shapes and measures block content.
2. `Input { measured: Vec<MeasuredBlock>, options: LayoutOptions }` is passed
   to `compute_layout_input`, producing paginated `Layout` pages/fragments.
3. `build_display_list_value_from_resident` produces renderer-independent
   text/glyph, decoration, rectangle, line, image, and shape primitives.
4. `hit_test`, `caret_rect`, and `range_rects` map between page-local geometry
   and absolute document positions.

The paginator itself does not require a DOM or browser. The published crate
does depend on `wasm-bindgen`, but its typed APIs work natively. The display
list retains block identity, paragraph identity, fragment ranges, UTF-16
document ranges, table-cell ownership, and region metadata.

This is a better separation seam than WordCraft's model-bound layout API and
rdocx's immutable OOXML-oriented output. It is not, however, a drop-in
libcosmic renderer or input method.

## AmiPro-Linux projection

The POC performs the following read-only conversion:

- `PageSetup` points become BetterOffice CSS pixels at `96 / 72`.
- Each semantic paragraph becomes one BetterOffice paragraph block with a
  stable string block id and paragraph id.
- Sparse direct-format runs are split into complete, non-overlapping segments.
- Effective family, point size, bold, italic, and underline values are copied
  into BetterOffice text runs.
- Paragraph after-spacing becomes measured pixel spacing.
- BetterOffice's UTF-16 source coordinates are mapped bidirectionally to the
  model's UTF-8 byte offsets. An emoji-containing paragraph exercises the
  non-BMP case.
- POC-only page-break markers can be inserted before selected paragraph
  indices. This overlay exists because the current Phase 1 model has no page
  break node; the document model was not changed for the experiment.

The projection is approximately 300 lines excluding tests and the benchmark.
The full isolated crate is 707 lines: 514 library/test lines and 193 benchmark
lines. That is meaningful adapter code even for the current small paragraph
model. Tables, frames, headers/footers, notes, fields, and anchored objects
would enlarge both the projection and the stable source-position map.

## Behavioral results

Five automated tests pass against published 0.3.0 and current-source 0.4.3:

| Test | Result |
|---|---|
| Immutable semantic-model projection | Passed; source document remains exactly equal |
| Mixed size/bold/italic/underline and spacing projection | Passed; attributes survive into typed BetterOffice runs/display primitives |
| Natural multipage flow plus explicit page break | Passed; the paragraph after the forced break starts on a later page |
| Caret, hit testing, UTF-16/UTF-8 mapping, and cross-page selection | Passed across paragraphs and a forced page boundary |
| Advanced block API probe | Table, text-box/frame, and anchored-image blocks deserialize and measure successfully |

The representative behavioral document contains 76 paragraphs, heading/body
sizes, direct 22-point text, bold, italic, underline, paragraph spacing, a
forced break, and enough text for multiple natural pages.

### Advanced structures

- **Tables:** `TableBlock`, rows, cells, nested blocks, table measurement, row
  fragmentation, and display-list cell metadata are public. A two-cell table
  measured successfully. AmiPro-Linux does not yet have a table model to map.
- **Frames:** `TextBoxBlock` accepts content, dimensions, page-relative
  position, wrapping, distances, fill, and outline. A positioned text box
  measured successfully. This is not evidence that DOCX text boxes implement
  Ami Pro linked-frame flow semantics.
- **Anchored objects:** image/shape blocks expose anchor position, wrapping,
  overlap, z-order, and source ranges. A square-wrapped anchored image measured
  successfully. A real image render was outside the POC because the current
  model has no image object and the test must not add a second model.
- **Headers/footers:** the engine supports header/footer bands and region-aware
  hit testing, but their full content enters display construction through a
  separate extras/region contract while pagination receives premeasured band
  heights. No end-to-end projection was attempted because the current model has
  no header/footer stories. This split would require additional state and
  source maps.

These results establish API availability, not Ami Pro fidelity.

## Performance and memory

Release measurements used a synthetic 300-paragraph document with body and
heading styles, direct bold/italic/underline/18-point formatting, US Letter
geometry, and the installed open Noto Serif regular/bold/italic/bold-italic
faces. The benchmark registers font bytes at runtime; it does not redistribute
them. Times are wall-clock observations on this development host, not product
budgets.

| Path | Result |
|---|---|
| BetterOffice full projection/layout/display, 20 runs | 315.8 ms mean; 21 pages |
| BetterOffice mean measurement component | 10.2 ms after warm-up |
| BetterOffice mean pagination component | 1.3 ms |
| BetterOffice mean display-list component | 224.6 ms |
| BetterOffice layout-only, one cold process | 53.7 ms; 29,984 KiB peak RSS |
| BetterOffice full, one cold process | 363.4 ms; 224,896 KiB peak RSS |
| Current estimator, 2,000 runs | 0.0143 ms mean; 19 pages; 3,084 KiB peak RSS |

The full BetterOffice result contained 38,950 primitives: 38,290 glyph-run
primitives and 660 decorations. Its serialized display list was 16,923,116
bytes. The release POC binary was 6.4 MiB and the dependency graph contained 44
external packages.

The current estimator is deliberately crude and does not shape glyphs, split a
paragraph between pages, or provide display/hit geometry, so the speed and page
count comparison is not feature-equivalent. Conversely, the POC performed a
full rebuild and serialized the list for measurement; BetterOffice also has
resident/incremental APIs which could reduce steady-state work. Integrating and
validating those APIs would require retained BetterOffice state alongside the
semantic model and was outside this bounded experiment.

### Pagination accuracy

The 300-paragraph benchmark produced 21 BetterOffice pages and 19 estimated
current pages. There is no authoritative reference rendering for this synthetic
document, so this difference is **not an accuracy score**. The POC proves that
BetterOffice honors measured font metrics, spacing, and explicit breaks. It
does not prove that BetterOffice matches Ami Pro pagination. A meaningful
accuracy comparison requires a future modeled Ami document with independently
measured reference page/line boundaries.

## Rendering and editor integration

The display list is renderer-neutral in data shape, but libcosmic cannot draw
it directly. A production integration would need to translate glyph outlines,
positions, decorations, clips, images, shapes, and region bands into Iced/
libcosmic renderer primitives. It would also need page-window caching to avoid
the measured full-list cost.

Using BetterOffice only for pagination while retaining COSMIC Text for live
editing risks two measurement authorities: BetterOffice would choose line/page
breaks while COSMIC Text would paint and hit-test potentially different glyph
geometry. Using BetterOffice's shaped display list would instead displace much
of the current COSMIC Text buffer/render path and require new IME,
accessibility, caret, selection, and clipboard integration. Either direction
is beyond a layout-only dependency addition and conflicts with the current
Phase 1 goal of preserving the proven editor.

The native hit APIs themselves are strong. They correctly resolved tested
caret positions and selection rectangles across pages, including conversion
from UTF-16 positions back to UTF-8 model offsets. That capability should be
reconsidered when Phase 3 requires stable page/frame selection.

## Licensing and dependencies

BetterOffice's reviewed source and three used crates are Apache-2.0. The POC's
44 external packages report permissive licenses: MIT, Apache-2.0, BSD-like/Zlib,
Unlicense, or compatible combinations. `unicode-ident` additionally carries
Unicode-3.0 terms. A distribution would need to preserve Apache licenses,
NOTICE/attribution material, and applicable Unicode notices. No copyleft or
known distribution blocker was found.

The benchmark used system-installed Noto fonts and does not make them project
dependencies. Adopting BetterOffice's optional bundled font packages would
require separate OFL/font attribution review.

## Version stability

The Rust crate available on crates.io is 0.3.0 while the inspected main branch
is already workspace version 0.4.3. Repository release commits show 0.3.0 on
2026-09-29, 0.4.0 on 2026-09-30, and 0.4.3 on 2026-10-03. That rapid pre-1.0
cadence is a maintenance risk, and the Rust publication currently lags the
repository/package version.

The POC's used typed boundaries remained source-compatible between published
0.3.0 and commit `ac80b80`/workspace 0.4.3, which is positive but insufficient
to infer a stability guarantee. Any future production experiment should pin an
exact version or commit and budget for adapter maintenance.

## Decision

**Defer the decision until a specific advanced-layout requirement demonstrates
a measurable need.**

BetterOffice demonstrates a credible advantage for Phase 3 pagination: it has
working multi-page placement, tables, floating/text-box structures,
headers/footers, source-bearing display primitives, and cross-page hit testing.
The core pagination time is promising. It does not presently offer a net Phase
1 advantage because:

1. the current model lacks most structures that would benefit;
2. accurate layout and the existing COSMIC Text paint path would have competing
   shaping authorities;
3. full display construction and memory use are high without an incremental
   integration;
4. the libcosmic renderer/input/accessibility bridge remains substantial; and
5. the rapidly changing 0.x release surface adds maintenance risk.

No migration is warranted. Continue the approved Phase 1 rich-editor work.
Reopen this decision in Phase 3 when a concrete table, linked-frame,
header/footer, anchored-object, or cross-page-selection fixture exists. At that
point compare a narrowly mapped fixture against independently observed Ami Pro
line/page geometry and test BetterOffice's resident incremental APIs. Adopt the
component only if it measurably improves correctness without creating a second
editable model or regressing COSMIC interaction behavior.

## Reproduction

```sh
cargo test --manifest-path experiments/betteroffice-layout-poc/Cargo.toml
cargo clippy --manifest-path experiments/betteroffice-layout-poc/Cargo.toml --all-targets -- -D warnings
cargo build --release --manifest-path experiments/betteroffice-layout-poc/Cargo.toml
experiments/betteroffice-layout-poc/target/release/betteroffice-layout-poc
POC_MODE=layout-only POC_ITERATIONS=1 /usr/bin/time -v \
  experiments/betteroffice-layout-poc/target/release/betteroffice-layout-poc
POC_MODE=current /usr/bin/time -v \
  experiments/betteroffice-layout-poc/target/release/betteroffice-layout-poc
```

The current-source compatibility check used a temporary manifest pointing at
the shallow BetterOffice checkout and ran the same five tests with `--offline`.
