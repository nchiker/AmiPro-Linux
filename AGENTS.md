# Codex Agent Instructions — Ami Pro for Linux

## Mission and authority
Implement a native Rust word processor for Pop!_OS COSMIC, aiming for verified functional parity with Lotus Ami Pro 3.1. Use a modernized Ami Pro interface. Original `.SAM` document and `.STY` style-sheet compatibility are high priority; original macros are lowest priority. Ami Pro is a page- and frame-based, desktop-publishing-style word processor, not a Word clone: pages, page layouts, and frames are core document structure. `SPEC.md` governs requirements; `PROJECT_PLAN.md` governs sequencing; `FEATURE_PARITY.md` records evidence and acceptance.

## Autonomous operation
- Continue through the next unblocked approved task without seeking permission after each step. Read all root Markdown files at session start.
- Stop only for an actual dependency requiring user action (e.g. missing reference path, physical COSMIC acceptance), an irreversible/destructive action, a licensing issue, or a material product decision not covered by these documents.
- Do not mistake a failed test or implementation uncertainty for a user decision: investigate, repair, and retry within the timebox. Record genuine blockers and continue other independent work.
- Keep the repository buildable at milestone boundaries; run targeted tests after each change. Do not claim success based only on compilation.
- Make small, coherent commits with test evidence. Do not commit generated binaries, original software, personal documents, or reference ISO content.
- At each session end, update `PROJECT_STATUS.md`, `PROJECT_PLAN.md` (progress, next step, blockers), and `FEATURE_PARITY.md` (evidence/status), and summarize exact tests and outstanding risks.
- On a new session, inspect `git status`, recent commits, and project status before changing code. Preserve unfinished work and WIP commits; never assume a WIP commit is verified. Do not reset, clean, amend, or discard work without a justified, documented reason and user approval for destructive actions.

## Reference material and legal separation
- Default private reference path: `../AmiPro-Reference/` relative to repository root; permit override with `AMIPRO_REFERENCE_DIR`.
- Read-only inspection of ISO, EXE, DLL, HLP, SAM, STY, templates and manuals is allowed.
- Decompiling and disassembling the reference EXE, DLL, and FLT files is authorized as broadly as the work needs, with any suitable tool (user authorization, 2026-10-11).
- Do not execute unknown binaries on the host. The `Windows XP` VirtualBox VM is the authorized isolated environment for running original Ami Pro (see Reference-driven development checkpoints).
- Never commit, redistribute, or embed proprietary binaries, fonts, dictionaries, icons, images, or original code (the bundled en_US Hunspell dictionary is a separately licensed, redistributable exception; see its COPYRIGHT file). Build independently written behavior and file-format implementations. Keep proprietary test fixtures outside Git.
- Do not assume the standalone ISO is 3.1: verify version from independent evidence. Treat SmartSuite separately until its contents are verified.
- Record evidence and uncertainty explicitly; don't label inferred features as confirmed.

## Engineering rules
- Rust stable, Cargo workspace; libcosmic/Iced for COSMIC UI unless a documented prototype disproves suitability.
- Keep document model, editing commands, layout, file formats, and UI as independent crates/modules with explicit interfaces.
- Prefer maintained, suitably licensed libraries for Unicode shaping, spellcheck, printing, image formats and other generic capabilities.
- No premature macro engine, plugin platform, broad cross-desktop packaging, or DOCX implementation.
- Validate all legacy input with bounds checks, fuzz/property tests where practical, and explicit diagnostics. Unknown structures must not be silently dropped on save.
- Require deterministic document-model tests, editing/undo tests, round-trip tests, and render/layout regression tests for affected functionality.
- Prefer vertical slices delivering a runnable COSMIC app over broad unfinished scaffolding.

## Interactive editor verification
- Verify interactive changes at three distinct layers: (1) deterministic document/model/controller behavior, (2) real COSMIC/Iced widget event handling and rendering/coordinate mapping, and (3) physical COSMIC acceptance for compositor-, portal-, visual-, or IME-dependent behavior.
- Widget integration tests must exercise the actual widget and event-handling path, not only controller methods or a test-only approximation. Use representative nonempty multiparagraph content, UTF-8, mixed character sizes/styles, scrolling, and realistic shaped-text geometry when relevant. Null-renderer/empty-document tests are useful for limited checks but are not evidence of correct hit testing in real documents.
- For pointer interactions, test press, movement/drag, release, focus/capture, hit testing, selection geometry, and coordinate transforms across lines and paragraphs. For keyboard interactions, test focus, insertion, navigation, selection, clipboard, undo/redo, and IME where feasible. Verify that state changes reach the displayed editor.
- Reproduce reported physical defects with a failing automated regression test whenever feasible, then demonstrate that it passes after the fix. If a reliable automated reproduction is impossible, record why and the exact remaining physical check; never invent test coverage.
- Report each verification layer separately. Passing model tests, compilation, or a headless widget test does not establish physical COSMIC acceptance. Explore proportionate headless compositor integration testing when it materially improves coverage; do not make it an open-ended infrastructure project.
- Keep experimental editor work feature-gated and preserve the accepted default editor until the new editor's specified automated and physical acceptance gates pass. (User decision 2026-10-12: the page surface is the normal build ahead of its accessibility and IME gates, which stay open; the text editor is preserved behind `--no-default-features` and stays tested.)

## Interface implementation
- Follow the approved interface design specification and reference artwork when present; do not substitute a generic scaffold for an approved design. Treat functioning vertical dropdown menus, document tabs with independent state, synchronized style selectors, and a page-integrated editing surface as functional interface requirements, not merely cosmetic polish.
- Do not add decorative controls that imply unsupported functionality; disable or clearly defer unavailable commands. Preserve safe file operations and existing editing behavior during interface work.

## Performance and representative workload gates
- Interactive editing must be measured on both a new empty document and substantial imported documents (including approximately 18-page and 300+-paragraph SAM cases when available). Cover sustained typing, insertion/deletion/replacement near the start/middle/end, Undo/Redo, selection dragging, scrolling, and switching tabs.
- Instrument end-to-end key-event-to-visible-update latency, not just isolated hit testing or layout microbenchmarks. Break down mutation, projection, shaping, invalidation, widget updates, and rendering where measurable. Report test hardware, build mode, document size, repetitions, and latency distribution (including tail latency), or explicitly state measurement limitations.
- Before optimizing, write a reproducible workload and baseline. Fix measured bottlenecks using the smallest safe change; prefer retained shaped paragraphs and paragraph-level invalidation when evidence supports them. Do not add rope/arena/GPU dependencies speculatively.
- Every optimization must preserve formatting, text fidelity, caret/selection, scrolling, undo/redo, and tab isolation, and test stale-cache behavior after actual edits, formatting, resize, and zoom. Microbenchmark wins do not establish user-perceived performance.
- Severe physical responsiveness failures block milestone acceptance even if all automated tests pass. Record **PASS**, **PARTIAL PASS**, **FAIL**, or **PENDING**, with the user's observed results and explicit remaining checks; never silently convert a partial pass into a pass.

## UI design, command discoverability, and acceptance
- Before major interface implementation, maintain a user-approved visual specification with annotated screen states, menu inventory, command ownership matrix, layout dimensions/spacing, and acceptance criteria. Use original Ami Pro 3.1 in the Windows XP VM as a behavioral reference and the supplied Gemini interactive HTML mockup as visual/interaction inspiration, not as production code or an authoritative description of Ami Pro internals.
- Keep standard menus discoverable and functionally complete for implemented commands. Show keyboard shortcuts alongside relevant menu items; ensure accelerators, enabled/disabled state, focus, and keyboard navigation behave correctly. Unimplemented commands must not masquerade as functional commands.
- Give each piece of style information a primary home. Synchronize contextual conveniences without duplicating competing controls or ambiguous state. Define clear roles for menus, compact toolbar, style selector, Styles panel, future Frame Properties inspector, ruler, document tabs, page canvas, zoom, and status bar.
- Require visual checks in light and dark COSMIC themes, typical desktop window sizes, representative imported documents, empty documents, scrolling, and long filenames. Check compactness, alignment, page viewport clipping, readable contrast, and status-bar spacing.
- Screenshots and user physical acceptance are required for material visual changes. Passing GUI tests or reaching the COSMIC event loop is not visual acceptance. Record explicit deviations and fix them before closing the UI milestone.
- Do not invent functionality to match a mockup. Prototype controls may be illustrative; distinguish real behaviors from visual demonstrations and use native libcosmic controls where suitable.

## Reference-driven development checkpoints
- Before substantial pagination, frames, tables, text flow, or complex style work, research existing maintained, license-compatible implementations (e.g., COSMIC Text, Iced/libcosmic, Xi editor, LibreOffice) and the current repository's APIs. Record reusable concepts, dependency/integration cost, alternatives, and a narrow decision. Prefer targeted investigation over open-ended architecture audits.
- Operate the original Ami Pro 3.1 in the isolated `Windows XP` VirtualBox VM directly (user authorization, 2026-10-11). Agents may start and stop it, take and restore snapshots, attach exchange media or transient shared folders, drive it through keyboard/mouse input, guest control, and screenshots, and use Ami Pro there both for behavioral comparisons and to author controlled SAM/STY test documents.
- Keep the clean baseline snapshot intact and restore it whenever the guest state is doubtful. Do not enable guest networking or add permanent host shares. Keep VM disks, guest-generated documents, and screenshots of private content outside Git; commit only independently written findings and synthetic fixtures.
- Decompile and disassemble the reference binaries as much as needed (Ghidra or other tools); there is no per-question gate (user authorization, 2026-10-11). Prefer whichever source answers a question most reliably: observed behavior in the XP VM, manuals and help files, format samples, or decompilation.
- Keep binaries, decompiler projects and output, and reconstructed code outside Git and separate from the implementation. Transfer documented behavior, tests, and specifications into the implementation, not copied or transliterated code.
- Each reference investigation must yield a decision, test, specification, or documented limitation. Do not assume the Gemini mockup's CSS frame movement, link indicators, zoom, or performance claims implement real document layout.
- Model Ami Pro's page structure from the start (user direction, 2026-10-11; 3.1 user's guide chapters 8 and 14–16):
  - Every page has exactly one page layout (margins, columns and gutters, tabs, lines, header/footer), taken from the style sheet or from an inserted page layout.
  - A frame is "a mini-document within the main document". Text frames hold their own styled paragraphs, columns, and tabs; picture frames hold pictures, drawings, charts, and equations. Tables live in the main text or in frames.
  - Frames are placed where placed, with the paragraph above, flowing with text, or repeating on all/left/right pages; they choose a wrap mode and can overlap, layer, and group.
  - Design document positions, object identity, undo, layout, hit testing, and SAM import around pages, page layouts, the main text flow, and frame text flows together. Do not treat frames as a later add-on to a single text flow.
- Decide stable object IDs and incremental layout as part of that model decision, after examining current mechanisms and compatibility needs. Defer speculative engine replacement, custom GPU rendering, and scripting.

## Phase 0 historical requirements (completed; do not restart)
- Target 10–18 focused engineering hours, maximum three working days, for initial inventory and architecture selection. This is a decision timebox, not a claim that all reference research is finished.
- Inventory private reference files and hashes; verify version; classify documentation and sample coverage.
- Evaluate existing Rust text editing/layout components quickly by source/API/license/maintenance review. Prototype only the strongest viable approach.
- Demonstrate or explicitly document limits for editable styled text, pagination, anchored frame, cursor navigation, and save/reopen. If insufficient, select the documented custom-model fallback.
- Produce `docs/research/PHASE0_FINDINGS.md` with evidence, chosen architecture, rejected alternatives and open risks; proceed to Phase 1 without waiting for approval unless blocked.

## Current handoff and priority
- Derive the active task from `PROJECT_STATUS.md`, `PROJECT_PLAN.md`, current acceptance reports, `git status`, and recent commits; do not follow stale commit-specific handoffs. If those sources disagree, reconcile them from evidence before changing code.
- The primary P0 typing defect is physically resolved at `bf0eaa5`: typing in imported `README31.SAM` feels normal. Dragging, scrolling, history, caret, and multi-tab checks not reported in that retest remain pending; do not infer or mark them passed, and reopen P0 only if a pending regression fails.
- P1 Revision 2 at `2037467` is the approved visual and architectural baseline. Its single-owner Styles workflow, compact menu/action-bar/tab/ruler hierarchy, modern COSMIC shortcuts, and responsive remembered Styles-panel policy are settled unless a specific technical or usability conflict is documented.
- **UI-1 is physically accepted for the functionality tested:** tested menus and editing commands worked, menu presentation and shortcut discovery improved substantially, and Ctrl+Shift+Tab switched documents. Ctrl+Tab appears to be intercepted before application dispatch; retain that conflict and every untested checklist case as pending rather than claiming complete UI-1 acceptance.
- **UI-2 is implemented and automated-green** (`96aeda4`; default-editor performance fix `8b78ecf`). Its physical COSMIC acceptance is pending with the user.
- **Sequencing (user decision, 2026-10-11):** after UI-2 acceptance and the VM-verified SAM import-fidelity fix, build the page, page-layout, and frame foundation (`PROJECT_PLAN.md` Phase 2) before the UI-3 Styles workflow, UI-4 workspace/zoom, or further editor work. Its model decision follows `docs/Design/UI_SPEC.md` section 9.
- Preserve the P0 incremental editing, retained hidden-editor synchronization, and changed-paragraph shaping improvements, the accepted default editor, and the rich-editor feature gate until the page surface replaces them through its own acceptance gates. Change the SAM importer only with original-application evidence (corpus, manual, VM, or decompilation).
- Tables, drawings, charts, and other Phase 5 content come later, but the Phase 2 model must leave room for them. Treat current acceptance records and user feedback as authoritative over older completion claims.

## Communication
- Report actual outcomes, not aspirational completion. Distinguish 'build passed', 'automated tests passed', 'manual COSMIC check pending', and 'original Ami Pro comparison pending'.
- If a user-only action is needed, provide a single precise command or request and continue independent work when possible.
