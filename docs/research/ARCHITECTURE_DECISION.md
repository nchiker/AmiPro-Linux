# Architecture Decision: Semantic Model with Reusable Text Primitives

**Decision:** Accepted for Phase 1 on 2026-10-09. Revisit after an editable-frame and complex-script spike.

## Context

Ami Pro compatibility requires semantic styles, deterministic pagination, positioned and linked frames, legacy provenance, safe unknown-data handling, and editing across page/frame boundaries. A plain text editor widget cannot be the source of truth, while building shaping, font discovery, and Unicode boundary logic from scratch would be wasteful.

## Decision

Use a project-owned Rust document model and typed command layer. Use libcosmic/Iced for the native COSMIC shell and custom page widget integration. Adopt maintained text primitives—initially COSMIC Text because it is already aligned with the COSMIC stack—for shaping, bidi, selection geometry, font fallback, and glyph output. Keep layout/pagination and SAM/STY adapters behind independent interfaces.

The first implementation deliberately has four independent crates: `document`, `editing`, `layout`, and `ami-formats`. The libcosmic application consumes them but does not own document semantics.

The current paginator uses deterministic estimated metrics only. It proves the page-break interface, not typographic fidelity. Replacing estimated lines with shaped fragments must not change the document or command APIs.

## Evaluated options

| Option | Strengths | Blocking gaps / cost | Result |
|---|---|---|---|
| Iced/libcosmic `text_editor` as engine | Native integration, cursor and plain multiline editing | No semantic paragraph/character style model, pagination, frames, or linked flow API | Use only as temporary Phase 1 input widget |
| COSMIC Text | Active COSMIC-aligned project; shaping, bidi, fallback, rich spans, selection/editing; MIT/Apache-2.0 | No document pagination or frame model; APIs evolve | Preferred shaping/editing primitive |
| Parley | Active rich-text layout, selection/editing utilities, ICU4X/HarfRust stack; MIT/Apache-2.0 | Renderer/toolkit integration work; no document/frame pagination | Retain as fallback/benchmark |
| COSMIC Edit | Real libcosmic application and useful integration reference | Pre-alpha code editor, not a reusable publishing engine; GPL-3.0-only | Reference only |
| Typst engine | Mature page composition, text, blocks, PDF-oriented layout | Batch/typesetting semantics do not provide interactive WYSIWYG editing, stable source positions, or Ami Pro object mapping without a large adapter | Reject as editor core; concepts may inform pagination |
| Fully custom text stack | Maximum control | Unacceptable Unicode shaping, bidi, fallback, accessibility, and maintenance burden | Reject |

## Evidence reviewed

- libcosmic official repository and template: <https://github.com/pop-os/libcosmic>, <https://github.com/pop-os/cosmic-app-template> (MPL-2.0; Iced-based COSMIC application toolkit).
- Iced 0.14 text editor API: <https://docs.rs/iced/latest/iced/widget/text_editor/>.
- COSMIC Text official repository: <https://github.com/pop-os/cosmic-text> (MIT/Apache-2.0; shaping, bidi, fallback, rich text and editing).
- Parley official repository: <https://github.com/linebender/parley> (MIT/Apache-2.0 rich-text layout and editing utilities).
- COSMIC Edit official repository: <https://github.com/pop-os/cosmic-edit> (GPL-3.0-only application).
- Typst official repository layout sources: <https://github.com/typst/typst>.

Maintenance was evaluated from current upstream repositories on 2026-10-09. Dependency revisions will be pinned once the first COSMIC build is reproducible.

## Consequences and validation gates

- Legacy bytes never become the live model without bounded parsing and a fidelity report.
- Native save/load is allowed now; original SAM/STY write-back remains disabled.
- Text shaping is replaceable and independently testable.
- Before Phase 3, prototype shaped styled spans, IME/cursor motion, page continuation, and one anchored/linked frame in the custom page widget.
- If COSMIC Text cannot provide stable hit testing across fragmented runs, run one bounded Parley comparison rather than building competing applications.

