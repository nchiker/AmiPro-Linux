# Phase 0 Findings

**Date:** 2026-10-09  
**Outcome:** Architecture selected; Phase 1 implementation started. Original-app behavioral comparison and real COSMIC UI testing remain pending.

## Findings

The supplied ISO is a SmartSuite Windows CD that contains a directly verified Ami Pro 3.1.0.0 16-bit Windows executable, complete help, numerous samples, 63 Ami Pro STY files, and varied supporting resources. The corpus is sufficient to start defensive format discovery. Full feature confirmation still requires systematic HLP/manual extraction and, where ambiguity remains, isolated execution on an authorized Windows environment.

SAM/STY files sampled so far are section-oriented text with format version `4`. This makes incremental, loss-aware parsing practical. A checked-in bounded parser now identifies sections and styles, decodes Windows CP-1252, and retains unknown sections; it does not claim semantic import/write compatibility.

Existing Rust libraries solve shaping and Unicode editing primitives, but none evaluated supplies Ami Pro's interactive semantic document model, pagination, and linked positioned frames. The selected hybrid is documented in `ARCHITECTURE_DECISION.md`.

## Prototype evidence

- Independent `Document` model with named paragraph styles, page geometry, validation, and deterministic native serialization.
- Typed insert/delete/style commands with Unicode-boundary checks and exact undo/redo tests.
- Deterministic basic multipage layout interface.
- Bounded read-only SAM/STY discovery parser tested with synthetic data and exercised on private real files.
- libcosmic application source with Ami Pro-style command strip, toolbar, styles panel, editable canvas, pagination status, and native save/reopen wiring.

The core workspace passes all nine automated tests and strict Clippy. The COSMIC crate now compiles, links, and passes strict Clippy against pinned libcosmic 1.0.0 revision `dbc62f2d899cdbfbe43d170689ddc0290db663f4`. A bounded launch reached event-loop creation but the automation context could not reach a compositor, so interactive and visual verification remain pending.

## Open risks

- Exact SAM/STY section grammar, binary/embedded payloads, and lossless byte provenance after text decoding.
- Shaped fragment pagination, widow/orphan logic, footnotes, columns, and linked frame reflow.
- Stable cursor/selection and IME behavior across page fragments.
- libcosmic upstream API revision and required native build packages.
- Original behavior evidence still needs HLP/manual extraction and isolated comparison.

These risks do not block continued independent model and parser work.

