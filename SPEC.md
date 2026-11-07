# Ami Pro for Linux — Product Specification

**Status:** Baseline requirements; original Ami Pro 3.1.0.0 reference verified, behavioral inventory in progress. **Target:** Pop!_OS COSMIC / Wayland. **Implementation:** Rust + libcosmic.

## Mission
Build a native Linux word processor with a modernized Ami Pro interface and the long-term goal of complete *documented* Ami Pro 3.1 functional parity. The goal is functionality and familiar workflow, not pixel-perfect copying. Never claim full parity until every verified feature has an acceptance test.

## Priority
- **P0:** Robust editing, styles, pagination, frames, tables and document integrity; import/edit/save original `.SAM` documents and `.STY` style sheets with transparent fidelity reporting.
- **P1:** Advanced publishing, graphics, headers/footers, references, mail merge, templates, print and PDF; modern native usability.
- **P2:** Original macro compatibility. Preserve a stable command architecture for future automation but defer the macro interpreter.
- Modern interchange (ODT/DOCX) follows core parity and reliability; PDF output can be earlier for testability.

## Feature families to verify against original manuals/help/binaries
Text and paragraph editing; selection and clipboard; find/replace; formatting and named styles; page setup and columns; sections; frames and linked text flow; tables; drawing and images; fields and variables; headers and footers; footnotes/endnotes; indexes and contents; spellcheck and language tools; mail merge; templates; printing; import/export filters; keyboard commands; document and application preferences; macros (deferred). This is a **candidate inventory**, not a verified list.

## Document model
Ami Pro is a page- and frame-based, desktop-publishing-style word processor (user direction and 3.1 user's guide chapters 8, 14–16; 2026-10-11). The model must represent from the start:
- pages, each with exactly one page layout (margins, columns and gutters, tabs, lines, header/footer) from the style sheet or an inserted page layout, including left/right pages;
- the main text flow, which breaks across page-layout columns and pages;
- frames as mini-documents on the page: text frames with their own styled paragraphs, columns, and tabs; picture frames holding pictures, drawings, charts, and equations;
- frame placement (where placed, with paragraph above, flow with text, repeating on all/left/right pages), wrap mode, layering, grouping, transparency, lines, fill, and shadow;
- tables in the main text or in frames, and frames inside headers, footers, and table cells.

## Original format compatibility
- Detect actual format variants from verified corpus rather than file extension alone.
- `.SAM`: aim for read, edit, save and high-fidelity round-trip, including layout, text, styles, frames, graphics and metadata where supported.
- `.STY`: aim for reading, applying, editing and writing original style sheets.
- Unknown legacy data: retain losslessly where practical; otherwise block destructive overwrite or provide explicit loss report and Save As.
- Never modify the user's only copy. Use backups, temporary files and atomic replacement.

## Interface
Traditional menu bar, toolbar, interactive rulers, centered WYSIWYG pages, styles panel, frame tools, status/zoom, keyboard shortcuts, native COSMIC appearance, accessibility and high-DPI support. Preserve user workflow where practical while modernizing visual presentation.

## Quality and performance
Correct editing and data preservation precede speed. Support large multi-page documents without excessive input latency; measurable performance targets will be set after representative corpus and baseline measurements. Test UTF-8/Unicode, complex text, cursor/selection boundaries, multi-page undo/redo and recovery.

## Definition of done
Every documented 3.1 feature has source evidence, implementation status and automated or manual acceptance criteria in `FEATURE_PARITY.md`; required SAM/STY fixtures pass import and round-trip checks; no known silent data loss; COSMIC UI passes real-machine verification. Macro compatibility may be an explicitly documented post-1.0 exception only if the user approves that release scope.

## Constraints
No reuse or distribution of proprietary Ami Pro code or assets. Original ISO and documents are private research inputs, not repository artifacts. No assertion that source compatibility, macros, or pixel-identical pagination are solved without evidence.
