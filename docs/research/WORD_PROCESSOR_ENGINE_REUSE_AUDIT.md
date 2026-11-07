# Word-Processor Engine Reuse Audit

**Date:** 2026-10-09  
**Decision:** Retain the existing semantic document model and native COSMIC Text/Parley-based architecture.  
**Scope:** Bounded architecture audit before substantial custom pagination or advanced rich-text layout work.

## Executive summary

No reviewed component offers a reusable editing and page-layout engine that can be integrated into a native Rust/libcosmic application without either replacing AmiPro-Linux's semantic document model or maintaining two authoritative models.

LibreOffice Writer has by far the strongest established layout coverage, but its useful editing engine is embedded in the large UNO/VCL/Writer system. LibreOfficeKit exposes conversion and tiled bitmap rendering rather than a stable semantic editing/layout API. AbiWord and Calligra Words expose more source-level seams, but those seams remain coupled to their C++ UI, object, plugin, and document frameworks; their licensing and long-term ABI obligations also make them poor foundations for this application. Qt's `QTextDocument` is the cleanest separable C++ component reviewed, but it does not supply enough word-processor behavior to justify a second toolkit and document model. Typst is an excellent Rust typesetter, not an interactive word-processing editor.

A source-level addendum subsequently evaluated the omitted Rust projects WordCraft and rdocx. WordCraft is the closest architectural match found: it has separated Rust crates for a semantic model, editing session, pagination, hit testing, display lists, and rendering. Its layout and interaction identities are nevertheless coupled to its own model, and the inspected 0.3.0 workspace describes itself as not yet alpha. rdocx has a much larger, extensively tested native layout stack, but its public layout input is WordprocessingML-specific and its output is an immutable page/display model without an interactive editor contract. Neither provides a net integration advantage while preserving the AmiPro-Linux model as the sole authority.

The current architecture therefore remains the lowest-risk route:

- keep the existing AmiPro-Linux semantic model authoritative;
- keep COSMIC Text as the native interactive editing/shaping surface;
- use Parley only behind a narrow adapter where a measured COSMIC Text limitation requires it;
- build pagination, page objects, tables, headers/footers, and print layout as model-backed services rather than UI-owned state;
- consider LibreOffice only later as an out-of-process interoperability/reference oracle, not as the editor core.

There is not enough evidence to justify an integration experiment or an architectural escalation now.

## Audit boundary and method

This was intentionally limited to approximately one working day. The audit examined locally installed packages, package metadata, project source/API documentation, licenses, architectural seams, and the current AmiPro-Linux model and UI adapters. It did not fork, build, or embed any candidate and did not change the current architecture.

Local observations included:

- LibreOffice Writer 24.2.7.2 is installed. Its local installation is about 372 MB; major shared objects include an approximately 89 MB merged library and a 21 MB Writer library. LibreOfficeKit development headers are not installed.
- Distribution metadata for AbiWord 3.0.5 shows a C++ library coupled to GTK 3, GLib, Pango, GOffice, and related desktop dependencies.
- Distribution metadata for Calligra Words 3.2.1 shows dependencies on Qt 5, KDE Frameworks, and Calligra libraries.
- The repository already uses COSMIC Text 0.19, a Rust library under MIT OR Apache-2.0, through a tested native editor adapter.
- Addendum source snapshots: WordCraft `d7d9702bda1fe626952024dc37fd2624323dad0f` and rdocx `733ce6f7b2b816c5760692abf5c99d68b29ae897`, inspected from shallow temporary clones on 2026-10-09. No source was copied into this repository.

Scores below are comparative engineering judgments, not benchmark results. Five is most favorable to AmiPro-Linux. “Coverage” describes built-in word-processing layout breadth; all other score columns reward ease and fit.

## Decision matrix

| Candidate | License/distribution fit | Engine separability | Rust/libcosmic fit | WP coverage | Existing-model fit | Integration effort | Maintenance burden | Meaningful advantage? | Disposition |
|---|---:|---:|---:|---:|---:|---|---|---|---|
| Current COSMIC Text + model, with Parley adapter | 5 | 5 | 5 | 2 | 5 | Incremental | Moderate, owned in-tree | Yes: native editing and exact semantic control | **Retain** |
| WordCraft 0.3 source crates | 5 | 4 | 5 | 4 | 2 | High | High: young API and fast-moving stack | Strong capability breadth, but not a net advantage after model/render mapping | Reject dependency now; monitor as prior art |
| rdocx / `rdocx-layout` | 5 | 4 | 5 | 4 | 1 | High | Moderate–high: large Word-specific stack | Strong batch pagination, no interactive-editor advantage | Reject as editor core |
| LibreOffice Writer / LibreOfficeKit | 4 | 1 | 1 | 5 | 1 | Very high | Very high | Layout breadth only; inaccessible at the needed seam | Reject as editor core; possible later oracle |
| AbiWord | 1–2 | 2 | 1 | 4 | 1 | Very high | High | Mature WP behavior, offset by coupling and license | Reject |
| Calligra Words / Calligra text libraries | 2–3 | 2 | 1 | 4 | 2 | Very high | Very high | Useful frame/text concepts, not a stable embeddable engine | Reject |
| Qt `QTextDocument` | 3 | 4 | 2 | 3 | 2 | High | High | Clean API, but insufficient breadth for toolkit/model cost | Reject |
| Typst | 5 | 4 | 5 | 4 | 2 | High | Moderate | Excellent batch pagination, weak interactive-editor fit | Reject as editor core |
| Parley alone | 5 | 5 | 5 | 1 | 5 | Low for text layout | Moderate | Strong shaping/hit-testing fallback, not a WP engine | Keep narrow fallback |
| ONLYOFFICE document editor/core | 1 | 1 | 1 | 5 | 1 | Very high | Very high | Full editor, but web-stack and licensing mismatch | Reject |

License scores reflect the intended combined distribution, not whether source can be studied. This is an engineering assessment, not legal advice; any later adoption of copyleft code would require a focused license review.

## Candidate findings

### Addendum: WordCraft

**Source and maturity.** WordCraft was absent from the original candidate list. The addendum inspected [repository commit `d7d9702`](https://github.com/storytold/wordcraft/tree/d7d9702bda1fe626952024dc37fd2624323dad0f). The workspace is version 0.3.0 and contains approximately 45,489 lines of Rust and 276 test attributes, including 26 in its layout crate. Its source has substantive implementations for paragraph shaping, pagination, tables, headers/footers, page/column rules, display lists, caret geometry, hit testing, selection rectangles, command editing, undo/redo, and raster/PDF output. This is not a placeholder repository.

The project's own [roadmap](https://github.com/storytold/wordcraft/blob/main/ROADMAP.md) still says it is approaching alpha, that real-world DOCX corpus fidelity has not started, and that accessibility, localization, RTL/complex-script depth, native printing, footnote continuation, and column balancing remain incomplete. GitHub showed 39 commits at inspection time. The combination of unusually broad functionality and a short, fast-moving history makes API and behavioral stability a material risk even though the source is substantial.

**License and obligations.** Code is MIT OR Apache-2.0 and is compatible with AmiPro-Linux. The [workspace manifest](https://github.com/storytold/wordcraft/blob/main/Cargo.toml) uses permissively licensed Rust dependencies including HarfRust, Skrifa, Vello CPU, Krilla, and egui. Bundled fonts are under SIL OFL-1.1; hyphenation/dictionary assets are public-domain or CC0; notices and attribution must accompany redistributed assets. ArtCraft brand assets are trademark-restricted and must not be reused. A layout-only dependency still brings `wordcraft-doc`, `wordcraft-fonts`, `wordcraft-proof`, and `wordcraft-geom`, so font/hyphenation asset and notice handling cannot be assumed to disappear. The internal crates use workspace path dependencies and the inspected source did not present a stable, independently versioned layout API commitment.

**Separation and capabilities.** The [layout crate](https://github.com/storytold/wordcraft/blob/main/crates/layout/src/lib.rs) is meaningfully separated from its egui front end. It accepts `wordcraft_doc::Document`, produces pages containing positioned lines, fills, rules, images, shapes, and cells, and exposes caret, hit-test, and selection geometry keyed by WordCraft `StoryRef`, `Path`, and `Pos`. `wordcraft-render` rasterizes that page/display model using Vello CPU; the engine crate owns the selection, pending formatting, undo/redo, commands, and view state.

This separation is better than the C++ suites reviewed earlier, but it is not a model-neutral layout interface. Reusing layout alone requires a continuously maintained conversion from every relevant AmiPro-Linux paragraph, style, table, frame, header/footer, field, note, and object into WordCraft's model, plus a stable bidirectional position map for editing. Reusing its engine would instead replace the existing command/model pipeline. Its raster renderer does not integrate with libcosmic's renderer, accessibility, or text input; direct native rendering would require translating WordCraft's display/glyph structures or displaying page bitmaps and separately overlaying interaction state.

**Advantage and decision.** WordCraft would substantially reduce green-field pagination work if AmiPro-Linux adopted its model and rendering ecosystem. Under the approved constraint that the existing semantic model remains authoritative, the mapping, renderer bridge, position synchronization, accessibility work, API pinning, and upstream churn remove that net advantage. Do not add it as a dependency or migrate to it. Continue to treat its layout/cache/hit-test organization as useful prior art. Reconsider only after WordCraft publishes a stable model-neutral layout contract or a future AmiPro-Linux blocker can be expressed as a small immutable input/output experiment without adopting its model.

### Addendum: rdocx

**Source and maturity.** rdocx was also absent from the original audit. The addendum inspected [repository commit `733ce6f`](https://github.com/tensorbee/rdocx/tree/733ce6f7b2b816c5760692abf5c99d68b29ae897). The current workspace manifest is version 0.16.0. The `rdocx`, `rdocx-layout`, and `oxml-layout` trees together contain approximately 622,406 lines of Rust and 3,668 test attributes, including 442 in the two layout crates. The source implements Word-specific shaping and pagination for paragraphs, tables, columns, sections, headers/footers, notes, fields, drawings, wrapping, page borders, line numbering, vertical text, and deterministic page/PDF/raster output. This is a large implemented conversion/layout stack, not merely a DOCX writer.

**License and obligations.** Code is MIT OR Apache-2.0. The repository enforces a permissive dependency allow-list, while documenting one temporary advisory exception for the unmaintained `ttf-parser` dependency reached directly and through `fontdb`. `oxml-layout` includes metric-compatible Liberation, Carlito, Caladea, and subset Noto fonts under their own open font licenses and notices; those files and attribution obligations ship even when system-font discovery is disabled. Default features enable system-font discovery; crypto/signing features are optional in the high-level facade. These obligations are compatible but add supply-chain and packaged-asset maintenance.

**Separation and capabilities.** [`rdocx-layout`](https://docs.rs/rdocx-layout/latest/rdocx_layout/) is a published, separable Rust crate, and `oxml-layout` provides a useful backend-neutral positioned-page representation. The actual pagination entry point takes a large `LayoutInput` containing `rdocx_oxml::CT_Document`, Word styles, numbering, compatibility settings, headers/footers, notes, drawings, charts, media relationships, and other WordprocessingML semantics. Output `PageFrame` elements retain source spans and are suitable for PDF/raster backends.

The inspected layout/output crates do not provide an interactive editing session, undo/redo, caret navigation, selection rectangles, IME contract, or general inverse hit-testing API. The high-level `rdocx::Document` edits DOCX content but is not a WYSIWYG editor state machine. Therefore integration would require translating AmiPro-Linux's model into a synthetic WordprocessingML model, translating page frames into libcosmic drawing primitives, and independently implementing all interactive geometry and editing behavior. Using the format-neutral `oxml-layout` crate alone supplies positioned output and font primitives, not pagination.

**Advantage and decision.** rdocx is credible for future optional DOCX import/export or batch PDF comparison work, if those features are separately scheduled. It does not reduce the immediate interactive rich-editor work, and encoding Ami Pro semantics through WordprocessingML would introduce the wrong intermediate model. Do not adopt it as the editor or pagination core.

### LibreOffice Writer and LibreOfficeKit

**License.** LibreOffice is primarily MPL-2.0, which is compatible with this repository's intended MPL distribution at the file level, although its bundled third-party components have additional licenses. The project's [official license page](https://www.libreoffice.org/licenses/) is the controlling overview.

**Separation and integration.** Writer has a sophisticated internal document model and layout system, but it lives inside the large LibreOffice UNO/VCL runtime. The [Writer source documentation](https://docs.libreoffice.org/sw.html) identifies `SwDoc` and a broad collection of layout, frame, header/footer, footnote, drawing, and UI subsystems. Source directories are separated, but that is not the same as a supported standalone engine boundary.

[LibreOfficeKit](https://docs.libreoffice.org/libreofficekit.html) offers a C/C++ API for loading/conversion and an explicitly experimental tiled-rendering interface. The latter returns rendered bitmap tiles and forwards commands/callbacks. It does not expose Writer's semantic layout tree as a stable library API. A Rust wrapper could call the C API, but libcosmic would then display remote-rendered tiles while cursor, selection, IME, accessibility, and command state crossed a protocol boundary.

**Capabilities.** Writer is the strongest candidate for pagination, tables, floating and anchored frames, page styles, headers/footers, footnotes, printing, and embedded objects. Those capabilities are precisely the parts not available at a clean embeddable seam.

**Model fit and cost.** Making Writer authoritative would replace the current model. Mirroring AmiPro-Linux semantics into Writer would create two models and require bidirectional identity, cursor, style, undo, object, and pagination synchronization. Either approach conflicts with the approved architecture. Shipping or depending on a full office suite also creates a large deployment, security-update, and compatibility surface.

**Decision.** Do not embed Writer or LibreOfficeKit as the editor. A future, separately approved tool may use headless LibreOffice out of process as an ODT/PDF interoperability check or comparison oracle. It must not become required for SAM/STY fidelity or native editing.

### AbiWord

**License.** AbiWord is GPL-2.0 licensed; see its [COPYING file](https://github.com/AbiWord/abiword/blob/master/COPYING). Directly linking its engine into the application would impose combined-work distribution obligations inconsistent with retaining the present MPL-only product boundary without relicensing and legal review.

**Separation and integration.** AbiWord's [text engine source](https://github.com/AbiWord/abiword/tree/master/src/text) contains an editable piece-table document under `ptbl/xp` and formatting/layout/view code under `fmt/xp`. That is a more promising source seam than most complete word processors. In practice it remains a C++ engine built around AbiWord abstractions, with platform and view layers connected to GTK, Pango, GOffice, plugins, graphics, and application services. No stable C ABI or maintained Rust binding was found.

**Capabilities.** AbiWord has mature paginated editing, tables, headers/footers, sections, fields, printing, and object support. Exact frame and complex-object behavior is less comprehensive than Writer, and none of it guarantees Ami Pro semantics.

**Model fit and cost.** Its piece table, style system, undo history, and layout objects would compete with the existing model. An FFI adapter would need ownership, callbacks, rendering, input-method, selection, accessibility, and graphics bridges, while the project would inherit an additional GTK-oriented desktop stack.

**Decision.** Reject. The layout functionality does not offset the license, model duplication, FFI, toolkit coupling, and upstream maintenance burden.

### Calligra Words and Calligra libraries

**License.** Calligra contains a mixture of LGPL and GPL code; the project [README](https://github.com/KDE/calligra) describes the suite as mostly LGPL-2-or-later with GPL components, while [Calligra Words](https://apps.kde.org/en-gb/calligra.words/) is GPL-2.0-or-later. Reuse would require a per-file and link-boundary audit.

**Separation and integration.** The [Calligra libraries tree](https://github.com/KDE/calligra/tree/master/libs) contains promising modules such as `flake`, `odf`, `pageapp`, `text`, and `textlayout`. It also warns that the libraries historically provide no source or binary compatibility guarantee. Words depends on the Calligra main libraries and text-shape/plugin machinery, as reflected by its build products and application source. The practical reusable unit is therefore a substantial Qt/KDE/Calligra object ecosystem, not a small layout library.

**Capabilities.** Calligra's frame-oriented design covers pages, flowing text shapes, tables, ODF styles, embedded objects, headers/footers, and printing. Its Flake shape system is conceptually relevant to Ami Pro frames.

**Model fit and cost.** Integration would require C++/Qt FFI, a Qt rendering/event bridge inside libcosmic, plugin discovery, font and print coordination, and mapping the current semantic model into Calligra's ODF- and shape-oriented model. Long-term ABI churn and a second GUI runtime outweigh the reuse.

**Decision.** Reject as an embedded engine. Its frame and shape architecture remains useful prior art, not a dependency.

### Qt `QTextDocument`

**License.** Qt is available under commercial terms and, for relevant open-source modules, LGPLv3/GPLv3 subject to module-specific terms; see [Qt licensing](https://doc.qt.io/qt-6/licensing.html). Dynamic LGPL use can be distribution-compatible, but it adds compliance and deployment constraints.

**Separation and integration.** [`QTextDocument`](https://doc.qt.io/QT-6/qtextdocument.html) is deliberately usable independently of Qt widgets. It provides a structured rich-text model edited through `QTextCursor`, and its [rich-text structure](https://doc.qt.io/QT-6/richtext-structure.html) includes blocks, lists, tables, frames, and images. It can paginate against a page size and [print to a paged device](https://doc.qt.io/qt-6/printing.html).

This is the cleanest external engine seam reviewed. It is still a C++/Qt object model and renderer. Rust bindings would not eliminate the need to translate events, glyph geometry, painting, accessibility, clipboard, IME, and model mutations between Qt and libcosmic.

**Capabilities.** It handles useful rich text, basic tables and frames, page breaks, and printing. It is not a full Writer engine: sophisticated sections, repeating headers/footers, page-style logic, anchored-object rules, cross-page table behavior, and publishing-grade pagination would still require custom work.

**Model fit and cost.** It would duplicate much of the semantic model without eliminating the hardest advanced layout work. A conversion-only layout copy would also make cursor and incremental reflow synchronization difficult.

**Decision.** Reject. It offers insufficient additional coverage to justify a second toolkit and model. Revisit only if the product abandons native libcosmic rendering, which is not approved.

### Typst

**License and integration.** [Typst](https://github.com/typst/typst) is Apache-2.0 and implemented as reusable Rust crates. It has excellent language/toolchain compatibility with this project and strong deterministic typesetting, pagination, tables, headers/footers, drawing, and PDF output.

**Editing fit.** Typst compiles a source/content tree into laid-out pages. It is not an interactive word-processor editing engine with a mutable rich-text buffer, stable selection and IME state, semantic command undo, or native cursor behavior. Using it live would require translating every model edit to Typst content, recompiling, and maintaining hit-test/source mappings and a separate interactive surface.

**Model fit and cost.** It could eventually be an export renderer for a deliberately constrained mapping, but its layout semantics would not automatically match Ami Pro. It would create two layout definitions if also used for interactive pages.

**Decision.** Reject as the editing/layout core. Do not add an export dependency until native page semantics are stable enough to define and test the mapping.

### COSMIC Text and Parley

COSMIC Text is already integrated, tested on physical COSMIC, written in Rust, and licensed under MIT OR Apache-2.0. It owns shaping, cursor movement, selection, IME-facing editing behavior, and rich spans while AmiPro-Linux retains semantic styles and file-format control. It does not provide pages, tables, frames, headers/footers, or printing, so those remain project responsibilities.

[Parley](https://github.com/linebender/parley/blob/main/doc/concept.md) is also MIT/Apache and provides rich text runs, shaping, line layout, and hit testing. It is a good implementation option behind the existing layout abstraction for a specific, reproducible gap such as metric control or non-editing print layout. It is not a document editor or pagination engine and should not become another document model.

**Decision.** Continue with COSMIC Text for interactive editing. Keep Parley as a narrow adapter/fallback selected by tests, not as a speculative rewrite.

### ONLYOFFICE and web-editor engines

ONLYOFFICE has broad word-processing behavior, but its [core license](https://github.com/ONLYOFFICE/core/blob/master/LICENSE) is AGPLv3 with supplementary terms, and the editor is built as a large web/native hybrid rather than an embeddable Rust layout component. Embedding it would replace the native editing surface, duplicate the model, and introduce a browser/JavaScript protocol and major deployment burden. Other browser-based editors have the same architectural mismatch even where their licenses differ.

**Decision.** Reject without a deeper prototype; no evidence justifies one.

## Why a mature suite does not remove the Ami Pro-specific work

Full Ami Pro parity is not equivalent to generic DOC/ODT editing. SAM/STY import needs explicit preservation of recognized and unknown structures, Ami Pro style inheritance, frame and pagination rules, deterministic save behavior, and diagnostics. A foreign engine would still require a semantic mapping for each feature. If its model became authoritative, unsupported Ami Pro state would need side channels; if it remained a renderer, synchronization and round trips would become a second source of defects.

The existing separation—legacy parser, semantic model, editing commands, layout services, and native UI—keeps those rules testable without tying file fidelity to another suite's internal representation.

## Recommendation and guardrails

Retain the current architecture and continue the approved UI/design sequence. Specifically:

1. Keep `amipro-core` (and its document/style abstractions) as the sole authoritative model.
2. Extend page geometry and pagination through explicit, deterministic layout inputs and outputs that can later serve screen and print rendering.
3. Keep COSMIC Text responsible for interactive text editing and shaping; transfer only semantic edits through existing commands.
4. Add Parley only behind a narrow interface when a failing regression test demonstrates a capability gap that Parley solves.
5. Keep page frames, tables, headers/footers, and anchored objects as model-backed structures rather than encoding them in toolkit widgets.
6. Do not add LibreOffice, AbiWord, Calligra, Qt, Typst, or a web editor as a runtime dependency based on this audit.
7. If later interoperability work needs an external reference, prefer an optional out-of-process LibreOffice comparison/conversion harness with fixtures and versioned expected results. That is a separate, bounded decision and must not affect native editing.

## Reconsideration triggers

An architectural experiment is warranted only if a candidate demonstrates all of the following against a concrete blocked feature:

- a supported library boundary with a stable API and compatible license;
- semantic, not bitmap-only, access to layout and hit-testing;
- preservation of the existing document model as the single authority;
- native cursor, selection, IME, undo, accessibility, and libcosmic rendering behavior;
- a measured reduction in pagination/object-layout implementation and maintenance cost.

No candidate reviewed in the original time box met those gates. A later bounded
BetterOffice experiment found a credible model-neutral measured-block,
pagination, display-list, and hit-testing seam, but not a present net Phase 1
advantage: full shaped display construction was expensive, libcosmic rendering
and interaction still require a substantial bridge, and its 0.x releases are
moving quickly. The decision is deferred until a concrete Phase 3 advanced
layout fixture can measure need and Ami Pro accuracy; see
`BETTEROFFICE_LAYOUT_POC.md`. Development should proceed without migration,
while treating pagination and advanced layout as incremental,
regression-tested services rather than a one-shot custom engine rewrite.
