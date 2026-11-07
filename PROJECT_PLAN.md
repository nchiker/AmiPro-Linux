# Ami Pro for Linux — Autonomous Development Plan

From 2026-11-04 (SU1), each run adds one line here; its details are in its docs/*_PROGRESS.md.

- 2026-11-04, run A (`commands`), first session: CA.1–CA.11 done, CA.12 partial (New View and the Vertical Ruler open); next CA.13 line numbering, CA.14 passwords, CA.15–CA.16 (`docs/COMMANDS_PROGRESS.md`). First physical check passed 2026-11-05.
- 2026-11-05, run A (`commands`) finished: CA.13 line numbering and CA.14 password-protected files (full weight: decompile, VM fixtures, twins), CA.15 measures (corpora unchanged; Draft typing equal), CA.16 exit; physical check of CA.13–CA.14 and Move Up/Down passed 2026-11-05. Run B (`outline-mode`) finished 2026-11-06: OB.2–OB.7 built, measured (Outline typing +0.1–0.15 ms) and live-checked, gate green; physical check pending (`docs/OUTLINE_MODE_PROGRESS.md`).

**Owner:** User sets priorities and performs physical COSMIC checks. **Implementer:** the coding agent proceeds autonomously within `AGENTS.md`, including direct operation of original Ami Pro in the `Windows XP` VM (authorized 2026-10-11).

**Sequencing change (user decision, 2026-10-11):** Ami Pro is page- and frame-based, so the page, page-layout, and frame foundation moved forward. It is now Phase 2 and comes before the UI-3 Styles workflow, UI-4 workspace/zoom, style-sheet work, and further editor investment. The former Phase 2 (styles) is now Phase 3. Research documents written before this change call the pages-and-frames phase "Phase 3".

## Phase 0 — Reference inventory and architecture decision (10–18 hours; max 3 working days)

- [x] Locate reference ISO (task override: repository-local ignored `Reference/`).
- [x] Verify Ami Pro executable version and identify the media as a SmartSuite distribution rather than assuming a standalone disc.
- [x] Inventory EXE/DLL/HLP/SAM/STY/templates/manuals without executing binaries; record hashes and nonproprietary findings.
- [x] Assess available maintained editing, shaping and pagination libraries, including licensing.
- [ ] Prototype strongest viable editing stack; test styled editable text, page boundaries, anchored frame, cursor and save/reopen (or record measured limitations).
- [x] Choose architecture with rationale and update `ARCHITECTURE.md`.
  **Exit:** `docs/research/PHASE0_FINDINGS.md`, source evidence, decision, known gaps. Missing manuals do not block independent implementation.

## Phase 1 — Runnable COSMIC vertical slice

- [x] Create minimal Cargo app, native menu/toolbar, styles panel and page canvas.
- [x] Edit, select, undo/redo, save/reopen a simple native document.
- [x] Build on Pop!_OS and verify launch, keyboard editing, and native save/reopen manually.
- [x] Connect read-only SAM text/paragraph-style import to a graphical COSMIC Open workflow and explicit native Save As target.
- [x] Protect legacy SAM/STY sources, route first Save through Save As, confirm replacement, retain atomic native writes, and cover the workflow with automated regressions.
- [x] Remove the edit/preview switch and display supported model formatting in the editable COSMIC Text surface.
- [x] Add a derived lower-level COSMIC Text buffer adapter for variable metrics, paragraph spacing, exact family requests, and underline, with mapping regressions.
- [x] Physically verify the corrected baseline: graphical Open, safe Save As, subsequent Save, formatted editing, selection/undo, and reopen.
- [x] Audit mature reusable word-processing engines before substantial pagination work; retain the native semantic-model/COSMIC Text architecture (`docs/research/WORD_PROCESSOR_ENGINE_REUSE_AUDIT.md`).
- [x] Run an isolated BetterOffice measured-layout POC; defer adoption until a concrete Phase 2 (pages and frames) advanced-layout fixture demonstrates measurable need (`docs/research/BETTEROFFICE_LAYOUT_POC.md`).
- [x] Obtain final visual approval for revision 2 of `docs/Design/UI_SPEC.md` and `UI_MOCKUP.svg`; Gate A closed by explicit approval of commit `2037467` on 2026-10-10.
  - [x] Expand and scroll the page workspace responsively, preserve usable narrow-window horizontal scrolling, and derive paper/margins from `PageSetup`.
  - [x] Add the compact action toolbar, default-collapsed Styles panel, and synchronized toolbar/status/sidebar paragraph style state.
  - [x] Add a read-only horizontal ruler driven by the same document/display geometry as the page.
  - [x] Add conventional native vertical menus and scrollable document tabs with isolated content, caret/selection, history, dirty state, scroll position, and save target.
  - [x] Produce the reference-driven P1 approval candidate, complete command matrix, single-owner style workflow, annotated SVG, and explicit automated/physical acceptance gates.
  - [x] Revise the proposal with a distinct portrait paper surface, complete full-width toolbar grouping, realistic formatted selection, and one shared rail for Styles versus future Frame Properties.
  - [x] Implement UI-1 command discovery at `85086b8`: one command registry, complete conventional menus, visible pinned-libcosmic shortcuts, state-dependent enabling, unified default/rich editor dispatch, checked document list, safe quit, and native About. Automated verification passes.
  - [x] Physically accept UI-1 for the functionality tested (2026-10-10). Ctrl+Tab interception and untested checklist cases remain pending.
  - [x] Implement UI-2 chrome hierarchy (`2fa7a31`, `8fecd2e`, `e96f167`, `f80c178`, `96aeda4`): native title, grouped responsive action bar, single-line status without a style control, native document tabs with reachable New Document, frameless chrome rows, and the Ctrl+PageDown document-switch fallback. Automated verification passes.
  - [x] ~~Physically accept UI-2 on COSMIC (`docs/Design/UI_ACCEPTANCE.md` section 5) before UI-3.~~ Skipped by user decision (2026-10-12): folded into the combined physical check that ends the UI pass below.
  - [x] Remove the default build's per-edit whole-document `DocumentLayout` rebuild and full-content undo/redo rebuild (`8b78ecf`). `README31.SAM` typing went from 52–55 ms to 0.58–0.62 ms median end to end (headless); physical confirmation is pending.
  - [x] Verify untagged-paragraph and escape semantics against original Ami Pro, then correct the SAM importer with regression tests (2026-10-11, XP VM; also fixed notes truncating 27 corpus documents). Physical COSMIC confirmation with `README31.SAM` is pending.
- [ ] Add variable font metrics, paragraph spacing, and underline to the editable page surface without introducing a second document model.
  - [x] Retain a COSMIC Text font system with the derived buffer and expose shaped line/glyph geometry, decoration spans, and UTF-8-safe hit testing.
  - [x] Project one semantic paragraph into renderer-native rich spans with stable source ranges, variable size, underline, display scaling, and separate paragraph spacing.
  - [x] Implement an isolated, feature-gated one-paragraph custom surface using one renderer geometry for rich text, caret, selection, and pointer hits; route typing, IME commit, clipboard, navigation, and undoable replacement through the semantic pipeline.
  - [x] Physically verify the corrected one-paragraph interaction gate: focus, typing, caret alignment, selection, clipboard, undo/redo, and new/imported-document regression checks passed.
  - [x] Extend the feature-gated renderer projection and semantic controller across paragraphs with Enter split, Backspace/Delete join, multiline replacement, cross-paragraph selection/clipboard, vertical navigation routing, style-state synchronization, and native-save regression coverage.
  - [x] Physically exercise the corrected multiparagraph mouse-selection path. A COSMIC Text-backed production-widget regression reproduces the old cross-paragraph pointer error and passes the composite-paragraph correction; physical COSMIC selection also passed, with sluggishness retained as a performance follow-up.
  - [x] Before changing the performance path, conduct focused reference-driven research into caching and incremental layout in existing Rust text editors; evaluate only approaches applicable to the measured whole-document projection/reshape bottleneck.
  - [x] Retain shaped paragraphs across selection/caret/scroll-only view updates with explicit session, semantic-generation, geometry, scale, and renderer invalidation; verify stale-cache, selection-highlight, edit, formatting, undo/redo, scroll, tab, pointer, benchmark, and corpus regressions.
  - [x] Physically confirm the primary `README31.SAM` typing-responsiveness correction; typing now feels normal with `bf0eaa5`.
  - [ ] Retain unreported drag, scrolling, menu/toolbar history, caret-visibility, and multi-tab regression checks as pending; do not reopen P0 unless one identifies a problem.
  - [x] Instrument realistic sustained editing in `README31.SAM`, `AP31NAV.SAM`, and an empty document; remove the two hidden whole-document layout passes, reuse unchanged shaped paragraphs, and verify semantic/fallback synchronization through editing and history.
  - [ ] Close accessibility and visual-IME gaps before replacing the proven default editor; keep the custom surface feature-gated. These gates now apply to the Phase 2 page surface, which supersedes promoting the one-flow rich editor as is. (User decision 2026-10-12: the page surface became the normal build before these gates; they remain open for it, and the text editor stays buildable with `--no-default-features`.)
    **Exit:** Runnable application, repeatable build and automated tests, UI-2 physically accepted, and VM-verified SAM text/style import.

## Phase 2 — Pages, page layouts, and frames (moved forward 2026-10-11)

Ami Pro's document is a sequence of pages. Each page has one page layout: margins, columns and gutters, tabs, lines, and header/footer, from the style sheet or an inserted layout. Frames are mini-documents on the page. Text frames hold their own styled paragraphs, columns, and tabs; picture frames hold pictures, drawings, charts, and equations. Frames are anchored (where placed, with paragraph above, flow with text, repeating on all/left/right pages), choose a wrap mode, and can overlap, layer, and group. Evidence: 3.1 user's guide chapters 8, 14, 15, and 16, plus VM fixtures (`docs/research/SAM_FORMAT_NOTES.md`).

- [x] **2.1 Reference evidence** (complete for the model decision, 2026-10-11). Controlled fixtures authored in Ami Pro in the XP VM, one property at a time, plus synthetic read-back files and a decompiled cross-check. Independent specification: `docs/research/SAM_PAGE_FRAME_SPEC.md` (draft 2); evidence: `docs/research/SAM_FORMAT_NOTES.md`. Remaining unknown fields are preserved, not blocking. Importer fix: `<:t n>` table anchors no longer import as tabs.
- [x] **2.2 Model decision** (2026-10-11, Extra High effort). D1–D8 are recorded in `docs/research/DECISIONS.md` and `ARCHITECTURE.md`. Research checkpoint: `docs/research/PAGE_FRAME_REFERENCE_RESEARCH.md`. VM checks: V10–V23 and X06. In short:
  - typed IDs and flow-qualified positions;
  - U+FFFC inline objects plus legacy marks;
  - flows, ordered layouts, and frames, with one `LayoutBox` type;
  - pages as layout output;
  - `Arc`-shared root snapshots behind typed commands;
  - tables and pictures as frame content;
  - native v3 as single-file text.
- [x] **2.3 Model and native v3** (`amipro-document`, `amipro-editing`; done 2026-10-11, details and measurements in `docs/PHASE2_PROGRESS.md`). Each slice keeps the app buildable and its behaviour unchanged, and passes the existing tests.
  - [x] **2.3a Identity and shared roots** (`bbb39d6`; 5.2 µs per edit for command plus history on README31).
    - Add the ID allocator and `ParagraphId`. Add the flows map with `main_flow`, and `Arc`-shared roots.
    - Replace `PageSetup` with `layouts[0]` in integer twips, keeping conversion helpers for the current UI.
    - Move the app's snapshot history onto the shared roots.
    - Turn `amipro-editing` into the command layer: commands without inverses that return a new root plus a change report. Retire `Editor`'s inverse history, keeping its UTF-8, run, split, and join tests as command tests.
    - Gate: all tests pass. The default-editor and rich-editor release baselines are not worse (interleaved A/B on `README31.SAM`). Per-edit history cost is no higher than today's 0.07 ms whole-document clone.
  - [x] **2.3b Inline objects** (`45aa4b4`, `4bdc057`).
    - Add U+FFFC plus per-paragraph `objects`, and legacy marks. U+2028, U+00AD, and U+00A0 are text.
    - Range deletion removes objects, the frames they anchor, and their flows. U+FFFC input is sanitized.
    - Both one-flow editors keep objects intact. U+FFFC shows as a marker, and plain-text reconciliation and delta synchronization map deletions to the object rule. Cover this with widget-path tests.
  - [x] **2.3c Frames and layouts** (`8b617ef`; undo property test over 14 command kinds).
    - `Frame` gets placements, wrap, border, shadow, corners, background, interior `LayoutBox`, group, name, macro, and legacy data.
    - Content kinds: text flow; table grid with cell flows; picture by `ObjectId` or path; legacy.
    - Add `frame_stack`, groups, `PageLayout` (right/left geometry, mirror, header/footer frames per side, begin-on-second-page, repeating frames), break objects, floating header/footer objects, and the page-number field.
    - Typed commands: create, delete, move, resize, modify; placement change; z-order within a layer; group and ungroup; insert and modify layout.
    - Validate every invariant in `ARCHITECTURE.md`.
  - [x] **2.3d Native v3** (specification `docs/Design/NATIVE_FORMAT_V3.md`; the app saves v3; corpus round trips through v3).
    - Encoder and decoder for `AMIPRO-LINUX/3`, with base64 blobs, strict record rejection, and full validation on load.
    - v1 and v2 load into one flow and one layout.
    - Test exact round trip of every model feature, malformed input, and a decoder property test.
- [x] **2.4 SAM import** (read-only; `docs/research/SAM_PAGE_FRAME_SPEC.md`; done 2026-10-11, `1343b63`: 160/160 with 0 unresolved anchors; page tables keep their flagged placement, see `DECISIONS.md`).
  - Layouts: `[lay]`, `[rght]`/`[lft]`, header/footer frames, `[repfrm]`.
  - Frames: `[frm]` flags and fields, `[frmlay]` margins, columns, and tabs; `[txt]`, `[tbl]` (grid and cell text; other cell fields kept as legacy data), and `[isd]`.
  - Embedded objects: the `[Embedded]` directory, offsets, and trailer.
  - Marks: `<:A n>`/`<:t n>` through the shared anchored-record index; `<:p…>` flags and layout names; `<:c>`; `<:H`/`<:h` flags with their paragraphs; `<**>` as U+2028 and `<*7>` as U+00AD; `<:P…>` page-number fields.
  - Unmapped controls become legacy marks. Unknown fields and sections are kept with their object. `[pg]` and `<:#…>` are dropped as caches.
  - Corpus audit:
    - frames by kind and placement;
    - all anchors resolved;
    - unknown flag bits;
    - page columns that differ from the uniform derived edges;
    - exact v3 round trip of all 160 files.
  - Tests use synthetic fixtures modeled on the VM fixtures, plus private-fixture tests behind an environment variable.
- [x] **2.5 Layout engine** (`amipro-layout`, shaper behind a trait).
  - [x] **2.5a Bounded spike** (2026-10-11; keep the own breaker, `docs/research/LINE_BREAKING_SPIKE.md`). Break lines over COSMIC Text shaped words with per-line widths, inline boxes, U+2028, and U+00AD, on `README31.SAM` and the frame fixtures. Record whether to keep the project's own breaker or adopt Parley (see `PAGE_FRAME_REFERENCE_RESEARCH.md`).
  - [x] **2.5b Engine** (2026-10-11/12, `df0c48b`; 19 rule tests).
    - Pages and columns, with breaks and layout switching, and the page count rule including fixed frames.
    - Fixed, anchored (clamped; moving with paragraph or line), and repeating placement.
    - Wrap exclusions for main and header/footer text.
    - Headers and footers: right/left, mirror, begin on second page, floating overrides.
    - Frame interiors, with derived columns and corner insets.
    - A basic table grid and picture boxes.
    - A display list whose fragments carry flow, paragraph, and byte ranges.
    - Deterministic tests with a fixed-advance shaper.
  - [x] **2.5c Incremental layout** (2026-10-12, `8bb1216`; relayout after typing 0.09–0.10 ms median on `README31.SAM`).
    - Per-paragraph shaping cache, and re-pagination with convergence.
    - Measure on `README31.SAM`, `AP31NAV.SAM`, and `MERCURY.SAM`.
    - The status bar shows the real page number and count, which fixes the hard-coded "Page 1".
  - [x] **2.5d Original comparison** (2026-10-12, `412f9c7`; `docs/research/LAYOUT_COMPARISON.md`). Compare page breaks, frame rectangles, and header positions against the VM fixtures, with recorded tolerances.
- [ ] **2.6 Page editing surface.** Automated parts done (2026-10-12, `2c9a38e`, `page-surface` feature; README31 typing 0.42–0.48 ms median at the time); frame interaction approved and implemented (2026-10-12, `9319efd`); frames can be created since 2026-10-17 (the frame creation run); **the physical check of creating, moving, and resizing frames passed (2026-10-17)**. Still open: accessibility nodes for the page text and picture decoding.
  - [x] Render the display list. Hit testing, caret, and selection work in any flow: main text, frames, headers and footers (with page-instance view state), and table cells (basic).
  - [x] Frame selection follows Ami Pro's rules (several Where placed frames on one page; groups as units); move and resize, by pointer and keys, as one undo step each (`UI_SPEC.md` §11).
  - [x] The Frame Properties inspector sits in the shared context rail.
  - [ ] VM fixtures F14 onward compare frame gestures with Ami Pro by stored rectangle.
  
  - Includes UI-4 workspace, status, and zoom, with one transform shared by rendering and hit testing.
  - Accessibility and visual-IME gates apply before it replaces the default editor. The rich-editor prototype is then retired or folded in. (It replaced it by user decision on 2026-10-12; the gates remain open.)
- [ ] **2.7 Physical COSMIC acceptance** of the page surface and frame workflows, plus original-compatibility comparison against the VM fixtures.

- Linked text flow between frames does not exist in 3.1: the Frame menu and Modify Frame Layout have no linking command (VM, 2026-10-11).
- **Next:** 2.7 (frame interaction and creating frames passed their physical checks 2026-10-17). The record is `docs/PHASE2_PROGRESS.md`.
  **Exit:** Editable multipage documents with page layouts, headers/footers, and text/picture frames that import from representative SAM files and lay out comparably to Ami Pro.

## UI pass on the page surface (moved ahead of the rest of Phase 3, user decision 2026-10-12)

The user moved the UI work forward so that later work is built on the final
interface and not revisited (`docs/research/DECISIONS.md`, 2026-10-12, "UI
pass before the rest of Phase 3"). It targets the `page-surface` build.

- [x] Screenshot harness for the whole window in light and dark
  (`window_screenshots`) and the gap list against the approved mockup
  (`docs/Design/UI_GAP_LIST.md`).
- [ ] Close the gaps in slices (workspace canvas and paper edge; context rail
  with list rows, header, filter, default visibility, narrow overlay, and a
  readable Frame Properties inspector; Underline and paragraph alignment
  commands with their action-bar groups; recognizable Open/Save icons; ruler
  surface and indent markers; word count in the status bar), each with tests
  and new screenshots.
- [ ] UI-3 Styles workflow (below) as part of this pass, since the rail is
  rebuilt here.
- [ ] Editable toolbar (user requirement 2026-10-12; `UI_SPEC.md` §12):
  data-driven bar, Customize Toolbar dialog, sets kept in the user's COSMIC
  configuration (with the rail's remembered visibility and width).
- [ ] One combined physical check: the page surface (`UI_ACCEPTANCE.md` §10),
  the frame checklist, and the visual result against the mockup, replacing the
  UI-2 checklist (§5). The user then decides whether the page surface becomes
  the default build.

## Phase 3 — Styles and style sheets

- [x] Paragraph formatting from style sheets and from the Text menu (2026-10-12): indents, spacing, break rules, alignment, and `<:I>`/`<:S>`/`<+@ABC>` overrides, imported, stored in native v3, and laid out (VM fixtures I, S, B, O, LS1).
- [x] Decompile audit of the format and layout rules (2026-10-12, `0c078e4`): the SAM reader, writer, and layout routines mapped in `AMIPRO.EXE`; the format notes' INFERRED and unknown items settled or narrowed (`docs/research/SAM_FORMAT_NOTES.md`). The rules it gave are imported and laid out (`d0ebed3`, `071a8f7`): alignment marks by OR, `[spc]` Add in and spacing bits, 1 1/2 and Double from the line, the last line's space below, column breaks after their paragraph, and a final break's empty page. VM fixtures 121 of 121 page counts.
- [x] Line heights follow the document's printer (user decision 2026-10-12: calibrate from the document's own caches; `cc0b7fb`, `b18349d`; `docs/research/DECISIONS.md`, "Calibrated line heights"). Page ends: user's corpus 689 of 1,097, ISO corpus 371 of 1,306 (was 287).
- [x] Character overrides (2026-10-12, `d672f88` to `a55dd5b`): the model and native v3 (`sformat`/`rformat` records, `docs/Design/NATIVE_FORMAT_V3.md`), import of `<:f…>`, the `<+X>` attributes, and the styles' `[fnt]` colour and attributes (decompiled control parser and font parser), and layout and page-surface drawing (caps, script and small-caps sizes and offsets, colour, the three underlines, strikethrough). User's corpus 686 of 1,097 page ends (689 before; `docs/research/LAYOUT_COMPARISON.md`, "Character overrides"). Not yet: overstrike (`<:O…>`), protected text and no hyphenation (kept as legacy marks), a paragraph-end font for empty paragraphs that hold only a font change, editing commands for the new attributes, and the face's pitch-and-family byte.

- **The remaining items run from `docs/PHASE3_RUNBOOK.md`** (slices 3.1–3.6, live state in `docs/PHASE3_PROGRESS.md`; written 2026-10-12).

- [x] 3.1 Line heights for fonts the caches never give alone, and an empty paragraph's line in the font at its end (`Paragraph::end_format`; `901fba7`); empty paragraphs read as Ami Pro's reader does (`870c359`). User's corpus page ends 686 → 742 of 1,108.
- [x] 3.2 Paragraph rules (`[line]`), Next style, Text Tightness, tab stops with leaders, rulers, the style's own tabs and the hanging-indent stop, frame interiors' space below, bullets and numbers (`[spec]`) (`a6df120` to `d7deb91`; VM fixtures NX, TT, TB2, FS, BN).
- [x] 3.3 Overstrike, protected text (Ami Pro's caret rules, VM fixture PT1), no hyphenation, and the face's pitch-and-family byte (`47fcf12`).
- [x] 3.4 Style origins (document, sheet, missing), the style sheet link, typed commands to create, modify, rename, and delete styles and to use another style sheet, a style-sheet reader, and tests for save and reopen, kept values, and refused sheets (`842e5d3`; VM fixture ST1).
- [x] 3.5 Decision stop (2026-10-13): D1 approved as `docs/Design/UI_SPEC.md` §14; D2 and D3 yes (`docs/research/DECISIONS.md`).
- [x] 3.6 Hyphenation (D2, `dcf444c`): Ami Pro's rules, TeX's US English patterns through `hypher`, the zone; measured.
- [x] 3.6 STY style records written as Ami Pro writes them (D3 part 1, `44f8e7f`); corpus round trip byte for byte.
- [x] 3.6 The whole STY writer without contents: header sections, styles, the page layout with its header, footer, and repeating frames and their text, Save as a Style Sheet (new files only), and the VM check that Ami Pro opens the sheets (D3 part 2; `038a778`, `72ef5ee`; VM fixtures SS1N, SS2N, SS2W, SD1, SD2, AP2–AP5). With contents is left for the SAM writer (Phase 4).
- [x] 3.6 UI-3 Styles workflow and the approved style dialogs (D1, `UI_SPEC.md` §14): Format › Styles ▸, Create Style, Modify Style with its panels and a page-surface sample, Style Management, Use Another Style Sheet with the style-sheet folders, and the Styles panel as the management owner (`7921530`, `401b221`, `d92f189`). Physical acceptance per `UI_ACCEPTANCE.md` Gate C waits for the user.
  **Exit:** Stable styles and explicit fidelity reports. Met for the automated parts (2026-10-13): every style field Ami Pro's reader takes is imported, stored, laid out, editable in Ami Pro's dialogs, and written back as Ami Pro writes it; fidelity reports are in `docs/research/LAYOUT_COMPARISON.md` and the corpus audit. The user's physical check of the style interface passed (2026-10-13). Open: With contents (Phase 4, with the SAM writer) and the findings listed in `docs/PHASE3_PROGRESS.md`.

## UI requests (user, 2026-10-13)

- [x] **Right-click context menus** on the page surface: on text, Modify Style "…", the clipboard commands, Font…, Special Effects…, and the paragraph's style; on a selected frame, Frame Properties, Bring to Front, Send to Back, Group, Ungroup, and Delete. Built 2026-10-13 (`98a956f`; `docs/Design/UI_SPEC.md` §16). The user's first check found that the menu took no clicks on scrolled pages and that the rail sent the pages to the top; both fixed the same day (`b05da29`), and the user's recheck passed ("That all works").
- [x] **Style Properties in the rail** instead of the modal Modify Style dialog, as Frame Properties works, so that the document stays in view while a style changes: approved and built 2026-10-13 (`30df4e7`; `docs/Design/UI_SPEC.md` §15). Physical check passed (user, 2026-10-13).

## Phase 4 — SAM compatibility

**A straight run** (user request, 2026-10-13): `docs/PHASE4_RUNBOOK.md` gives the order and gates, `docs/PHASE4_PROGRESS.md` the live state. Its decisions were taken before it starts (`docs/research/DECISIONS.md`, 2026-10-13): **A** Save As writes new SAM files only, never over an existing file; **B** what cannot be written exactly is listed first, with Cancel and Save Anyway; **C** page layout setup follows Phase 4 (below).

- [x] Parse real SAM variants with bounded readers and fixtures. Every readable SAM in both corpora is format version 4 (417 and 160); Lotus Word Pro and encrypted files are refused with a message (Phases 1–3).
- [x] Map structures into document model; support editing and safe Save As (Phases 2–3; native `.amipro` Save As).
- [x] Add loss reporting and corpus round-trip tests; enable original-format write-back only where verified. The run's slices (2026-10-13, `d4c0cf4` to `dd0b3a0`; `docs/PHASE4_PROGRESS.md`):
  - [x] 4.1 The writer's map: the decompiled document writer (`11c8:1c7b`), and whether Ami Pro needs the layout caches (VM).
  - [x] 4.2 Loss inventory, the round-trip audit on both corpora, and the loss check's core.
  - [x] 4.3 The document head and plain text.
  - [x] 4.4 The main text in full: formats, overrides, breaks, fields, floating headers and footers, notes, legacy marks.
  - [x] 4.5 Page layouts.
  - [x] 4.6 Frames.
  - [x] 4.7 Tables.
  - [x] 4.8 Pictures and embedded objects.
  - [x] 4.9 Caches, as 4.1 decided (and three rules Ami Pro relies on, found in the VM).
  - [x] 4.10 Save As SAM in the app (decisions A and B; and the frame lines bit).
  - [x] 4.11 With contents in Save as a Style Sheet; edited documents.
  - [x] 4.12 Phase exit, and the proposal for page layout setup (`UI_SPEC.md` §18).
- [x] Revisit (user, 2026-10-13): writing over an existing SAM, with a backup of the original the first time (`docs/research/DECISIONS.md`). Done 2026-11-04 (the user's decision): Save writes over after the loss check, `NAME.SAM.bak` kept the first time; Save As onto an existing `.SAM` asks first.
  **Exit:** Representative original documents import and edit without silent loss: both corpora round trip without loss outside the loss check's list, Ami Pro in the VM opens written documents and lays them out as the originals, and Save As SAM works in the app. **Met for the automated parts and the VM (2026-10-13):** read back equal 416/416 (user's), 159/160 (ISO; one malformed layout), 136/136 (VM fixtures); written documents open in Ami Pro as their originals do; Save As SAM tested through the app's messages and checked in the VM. The user's physical check of Save As SAM passed (2026-10-13; a second check after the page layout run passed too). Frames in table cells, the model gap left open here, closed on 2026-10-18 (the frames in table cells run).

## Page layout setup: headers, footers, and the page (after Phase 4; user decision C, 2026-10-13)

The user noticed that the plan had no item for setting up headers and footers. Phases 2–3 import page layouts, lay them out, and edit header and footer text on the page; nothing yet changes a layout (Page Setup is disabled). Ami Pro does it in Page › Modify Page Layout and with floating headers and footers (user's guide chapters 8 and 14).

- [x] Proposal in `docs/Design/UI_SPEC.md` §18, written at the end of the Phase 4 run (2026-10-13): a Page Layout Properties inspector in the context rail (the user's preference for rail inspectors), from Ami Pro's Modify Page Layout panels (Margins & Columns, Page Settings, Lines, Header, Footer), with floating headers and footers, the page-number field, and inserted layouts, and four questions. **Approved 2026-10-13** (answers L1–L4 in `docs/research/DECISIONS.md`).
- [x] **A straight run**, set up 2026-10-13: `docs/LAYOUT_SETUP_RUNBOOK.md` gives the order (PL.1–PL.12), gates, and stop rules (none planned); `docs/LAYOUT_SETUP_PROGRESS.md` is its live state. The AMIPRO.EXE and DLL disassemblies are authoritative for what Ami Pro does; the approved interface (§18, with §15's conventions) decides how it is presented. **Done 2026-10-13** (`226850f` to `6b46fd1`, from tag `layout-run-start`).
- [x] Build it as approved: Page Layout Properties in the rail (page, margins and columns, tabs, lines, header and footer, Pages and Mirror, Insert Page Layout and Remove), Insert › Page Break, Column Break, Floating Header/Footer…, Page Number…, Edit › Go To Header and Go To Footer; each change one undo step, laid out at once; written back by the SAM writer, the native format, and style sheets; twinned with Ami Pro in the VM (19 of 19 layout changes, 10 of 10 edits; `UI_SPEC.md` §18 implementation notes).
- [x] Physical check: **passed 2026-10-13** ("I didn't find any issues. Nothing feels off"). From the user's first use (2026-10-13): a page number in a header did not take a font (fixed the same day, `d9dde32`: the field takes the text formatting where it stands, as in Ami Pro, VM fixture PF1).
  **Exit:** Page layouts, headers, and footers can be set up as in Ami Pro and are saved in both formats.

## The ruler: tabs, indents, margins, and columns (user request, 2026-10-13)

The user: "in ami pro you can see and control tabs and column from the ruler". The page surface's ruler shows the margins and the caret paragraph's indents and takes no input.

- [x] Proposal in `docs/Design/UI_SPEC.md` §19 (2026-10-13), from the user's guide (chapter 8) and VM fixtures RU1 (a tab set on the ruler becomes the paragraph's own ruler) and RU2 (dragging between columns makes them unequal), with four questions.
- [x] The user's answers (2026-10-13: the four recommendations; R1–R4 in `docs/research/DECISIONS.md`).
- [x] **A straight run** (`docs/RULER_RUNBOOK.md`, live state `docs/RULER_PROGRESS.md`, tag `ruler-run-start`; done 2026-10-17): the ruler's code mapped in `AMIPRO.EXE` and `AMIPROUI.DLL` (`docs/research/RULER_NOTES.md`); the paragraph tab and indent commands; unequal columns in the model, engine, native format, both writers, and the rail panels; the interactive ruler with right-click menus, Edit › Go To Ruler, and View › Ruler (`UI_SPEC.md` §19 implementation notes); twins in the VM (12 of 12); the round trip unchanged and the layout comparison explained.
- [x] The user's physical check of the ruler: two findings fixed and rechecked; **PASS (2026-10-17)**.
  **Exit:** The ruler shows and edits tabs, indents, margins, and columns as Ami Pro's does, each gesture one undo step, written in both formats; Ami Pro shows the app's twins as its own.

## Creating frames (ahead of frames in table cells, user decision 2026-10-17)

- [x] Proposal in `docs/Design/UI_SPEC.md` §20 from the user's guide (ch. 15), the decompile (Create Frame `0x19e`, drawing `0x199`/`0x19d`, `ADDFRAMEDLG`, Make Default `0x19f`; `docs/research/FRAME_CREATION_NOTES.md`), and VM fixtures FC1–FC11; the user's answers F1–F4 (`docs/research/DECISIONS.md`, 2026-10-17).
- [x] **A straight run** (`docs/FRAME_CREATION_RUNBOOK.md`, live state `docs/FRAME_CREATION_PROGRESS.md`, tag `frame-run-start`; done 2026-10-17): Insert › Frame… (the Create Frame dialog) and Insert › Draw Frame (also on the toolbar) make Ami Pro's default text frame, selected, one undo step; Make Default and Restore Ami Pro's Default in Frame Properties; Where placed frames above anchored ones; opaque frames hide what is behind them and rounded corners are drawn; Ami Pro's text area for rounded frames; twins in the VM (6 of 6 written as Ami Pro writes them and shown by Ami Pro as its own); round trip and layout comparison unchanged in their summaries. Pictures later (F4).
- [x] The user's physical check of creating frames (2026-10-17: drawing, the Create Frame dialog, and Frame Properties on new frames, after two fixes from first use: the cut-off Restore Default label and a frame's line between columns).
- [x] The user's check of moving and resizing frames (2026-10-17: moving, resizing, grouping, Bring to Front and Send to Back, deleting, undo; Make Default carries to the next frame; `docs/Design/UI_ACCEPTANCE.md` §10).

## Bullets and numbering commands (after creating frames, user decision 2026-10-17)

Bullets and numbers already work as Ami Pro's do, through paragraph styles (Phase 3.2e and 3.6: imported, counted, drawn, edited in Style Properties, written back). Ami Pro 3.1 has no Bullets or Numbering command: lists come from styles and from Edit › Insert › Bullet… (`docs/research/BULLETS_NOTES.md`).

- [x] Proposal in `docs/Design/UI_SPEC.md` §21 from the decompile (Insert Bullet `0x344`, `<:B…>`, Right align `1108:4107`, Space for as First) and VM fixture BL1; the user's answers B1–B5 (`docs/research/DECISIONS.md`, 2026-10-17), after asking for improvements where Ami Pro's commands are lacking.
- [x] **A straight run** (`docs/BULLETS_RUNBOOK.md`, live state `docs/BULLETS_PROGRESS.md`, tag `bullets-run-start`; done 2026-10-17): Format › Bullets (Ctrl+Shift+L) and Numbering (Ctrl+Shift+N) apply Ami Pro's list styles (made from `_DEFAULT.STY` when missing), toggles on the toolbar and the right-click menu; Enter ends a list; Insert › Bullet…; Format › Bullet and Number Style… opens Style Properties at Bullets and numbers, which gains Space for; inserted bullets imported, drawn, edited, and written; Right align, superscript bullet text, and the picture bullets drawn as Ami Pro draws them; twins BL1, BL2, BL4 written as Ami Pro writes them and shown by Ami Pro as its own.
- [ ] The user's physical check (`docs/Design/UI_ACCEPTANCE.md`, "Bullets and numbering").

## Outline levels and multi-level numbering (user decision 2026-10-17, before frames in table cells)

The user asked how a numbered list moves between levels (1, 2, 3 → A, B, C → i, ii, iii). Part of Phase 5's outline item, pulled forward; Outline mode itself stays in Phase 5.

- [x] Map and proposal: `docs/research/OUTLINE_NOTES.md` (decompiled `OUTLINESTYLEDLGPROC` `12c8:0000`, counting `1200:1842`; VM fixtures OL1, OL2), `docs/Design/UI_SPEC.md` §22; the user's answers O1–O4 (`docs/research/DECISIONS.md`), and the Format menu folded into submenus at the user's remark.
- [x] **A straight run** (done 2026-10-18; `docs/OUTLINE_RUNBOOK.md`, record `docs/OUTLINE_PROGRESS.md`, tag `outline-run-start`): counting as Ami Pro counts (reset options, cumulative numbers); Format › Promote and Demote (Alt+Left, Alt+Right, toolbar); nested numbered lists 1. → A. → i.; outline level, reset options, and cumulative numbers in Style Properties; Format › Styles ▸ Outline Numbering…; the Format menu's submenus; twins in the VM (OL1, OL2 written as Ami Pro writes them; Ami Pro shows the app's files and a nested list as its own).
- [x] The user's physical check (`docs/Design/UI_ACCEPTANCE.md`, "Outline levels"): passed 2026-10-18 ("All list items checked"), with bullets and numbering's. Bullets nest as the user answered (Bullet 2…, diamond, dot, small square; VM AOL4; 2026-10-18).

## Choosing colours (user request 2026-10-18, before frames in table cells)

The user: colour selection "more like modern color selectors. Easier to choose wide variety of colors, but maintain your current pallet so you don't have to keep finding the colors you are using", around frames too.

- [x] Map and proposal: `docs/research/COLOUR_NOTES.md` (`AMIUICNT.DLL`'s colour bar and palette; the dialogs with colour bars; VM: the 15 colours and the 120-colour grid), `docs/Design/UI_SPEC.md` §23; the user's answers C1–C4 (`docs/research/DECISIONS.md`).
- [x] **A straight run** (done 2026-10-18; `docs/COLOUR_RUNBOOK.md`, record `docs/COLOUR_PROGRESS.md`, tag `colour-run-start`): one colour button and panel (Ami Pro's 15, In this document, Recent, libcosmic's picker) for every colour, the frame's four included; live drags as one undo step; twins in the VM (ACS1: Ami Pro draws the app's mixed colours exactly).
- [x] The user's physical check (`docs/Design/UI_ACCEPTANCE.md`, "Choosing colours"): passed 2026-10-18 ("colour buttons all work"; nested bullets "look good").

## Frames in table cells

- [x] Map and proposal: `docs/research/CELL_FRAME_NOTES.md` (Ami Pro's three frame lists `1248:0347`/`11c8:3ba0`, a cell's values 11 and 12 `1490:07c0`/`1428:07ce`, putting a frame in a cell `14d0:0000`, the caret in a frame's cell `1488:0e10`, Ami Pro's table grid `1428:0000`/`046b`; VM fixtures TF1–TF15), `docs/Design/UI_SPEC.md` §24; the user's answers T1–T3 (`docs/research/DECISIONS.md`, 2026-10-18).
- [x] **A straight run** (done 2026-10-18; `docs/CELL_FRAMES_RUNBOOK.md`, record `docs/CELL_FRAMES_PROGRESS.md`, tag `cells-run-start`): tables on Ami Pro's grid (insets, own gutters); frames in cells in the model, importer, writer, native format, and layout, following their cells; a click puts the caret in the cell, Delete removes the picture (one undo step), typing does nothing there; twins in the VM (ATF2, ATF3, ATF7, ATF10, README31 with a page break: Ami Pro shows the pictures in their cells).
- [x] The user's physical check (`docs/Design/UI_ACCEPTANCE.md`, "Frames in table cells"): passed 2026-10-18 ("that all works").
- [x] Then pictures (F4), the Phase 5 queue's run 1 (below).

## Pictures (Phase 5, queue run 1)

- [x] Map and proposal: `docs/research/PICTURE_NOTES.md` (Import Picture `10d8:126c`, Graphics Scaling `1168:0000` and `FRAMESCALEPROC`, the rectangle every draw `1168:2dd2`, the `[isd]` record `11c8:2e29`, `AMIPAINT.DLL`, `AMIWMF.DLL`, Ami Pro's screen image `1168:2016`; VM fixtures PI1–PI9, PIR1–PIR5, PIRT30, PIRT90), `docs/Design/UI_SPEC.md` §25; the user's answers P1–P8 (`docs/research/DECISIONS.md`, 2026-10-18).
- [x] **A straight run** (done 2026-10-18; `docs/PICTURES_RUNBOOK.md`, record `docs/PICTURES_PROGRESS.md`, tag `pictures-run-start`): scaling, pan, rotation, and a description in the model and both writers; the core crate `amipro-pictures` (Windows bitmaps, metafiles as a display list, Ami Pro's screen image); pictures drawn on the page (every stored bitmap and metafile in both corpora; PNG, JPEG, GIF, TIFF, PCX, SVG; Ami Pro's screen image for drawings, charts, and OLE objects); Insert › Image… (copy or link; an empty frame, an empty cell, a new frame at the caret); Frame Properties › Picture; picture mode; paste and drop; Save As SAM converting what Ami Pro cannot hold; twelve twins open in Ami Pro.
- [x] The user's physical check (`docs/Design/UI_ACCEPTANCE.md`, "Pictures"): PASS 2026-10-18, after paste and drop were fixed.
- [x] Then tables across pages, the Phase 5 queue's run 2 (below).

## Tables across pages (Phase 5, queue run 2; overnight 2026-10-18/19)

- [x] Map: `docs/research/TABLE_PAGES_NOTES.md` (the user's guide ch. 16; the decompiled row records `1428:046b`, headings `1468:044f`, the page table `11c8:381e`; VM fixtures TP1–TP11); `docs/Design/UI_SPEC.md` §26; decisions TP1–TP5 **taken overnight by the rulebook, to confirm** (`docs/research/DECISIONS.md`).
- [x] **A straight run** (done 2026-10-19; `docs/TABLE_PAGES_RUNBOOK.md`, record `docs/TABLE_PAGES_PROGRESS.md`, tag `table-pages-run-start`): row options in the model and both formats; tables split across columns and pages as Ami Pro splits them; the page surface; Insert › Page Break in a cell; the comparison against Ami Pro's page table; twins ATP2, ATP5, ATP6.
- [x] The user's physical check (`docs/Design/UI_ACCEPTANCE.md`, "Tables across pages"): passed 2026-10-19; TP1–TP5 confirmed 2026-10-19.
- [x] Text on both sides of a Wrap around frame (the overnight branch, merged at the user's word, `f6ee32f`).
- [x] **The row cache** (2026-10-20, user decision 2026-10-19): an Automatic row as read keeps Ami Pro's stored height while its text measures here at most half its gutter taller (VM RC1–RC3: Ami Pro measures every Automatic row when it opens a file); measured and merged (`docs/TABLE_PAGES_PROGRESS.md`, "The row cache"). For the table editing run: edited rows fitting their text exactly (RC2) and empty cells taking a line (RC3).

## Find & Replace (Phase 5, queue run 3)

- [x] Map and proposal (prepared overnight 2026-10-19): `docs/research/FIND_NOTES.md` (Edit › Find & Replace `0x1c3`, its four dialogs in `AMIPROUI.DLL`, the user's guide ch. 20, VM fixtures FR1–FR7), `docs/Design/UI_SPEC.md` §27; the user's answers F1–F4 (2026-10-19).
- [x] **A straight run** (2026-10-20; `docs/FIND_RUNBOOK.md`, record `docs/FIND_PROGRESS.md`, tag `find-run-start`): the search in `amipro_editing::find` (Ami Pro's patterns, streams, options, capitals rule, attributes, styles; Replace All as one step); the find bar on the page surface with highlights, a count, Options ▾, and recent searches; twins: Ami Pro's input files give Ami Pro's results; every paragraph of both corpora found in its stream.
- [x] The user's physical check (`docs/Design/UI_ACCEPTANCE.md`, "Find & Replace"): passed 2026-10-20; the bar then moved above the ruler, styled as the toolbar, its fields aligned (the user's visual notes). The user's look at its new place passed the same day.
- [ ] Next in the Phase 5 queue: fields (run 4).

## Fields (Phase 5, queue run 4)

- [x] Map and proposal (2026-10-20): `docs/research/FIELDS_NOTES.md` (the decompiled `AMIPRO.EXE`, `AMIPROUI.DLL`, `AMIMACRO.DLL`; the user's guide ch. 5, 23, 32; VM fixtures FD1–FD6), `docs/Design/UI_SPEC.md` §28; the user's answers FL1–FL7 (2026-10-20: a Fields panel in the rail; Include and the prompts worked out; the rest as recommended).
- [x] **A straight run** (2026-10-20; `docs/FIELDS_RUNBOOK.md`, record `docs/FIELDS_PROGRESS.md`, tag `fields-run-start`): dates, times, Doc Info fields, and power fields shown and written as Ami Pro's; the field language (45 of Ami Pro's results reproduced); updating by Ami Pro's rule; the Fields panel and Document Properties in the rail; Ami Pro's time base, editing minutes, and counts in `[desc]`; twin AFD1 shown by Ami Pro as by the app.
- [x] Next in the Phase 5 queue: print and PDF (run 5), below.
- [x] The user's physical check: passed (the user, 2026-10-20: "Powerfields work"), after the codes became editable in the text and the commands moved into Edit › Power Fields ▸.

## Print and PDF (Phase 5, queue run 5)

- [x] Map and proposal (2026-10-20): `docs/research/PRINT_NOTES.md` (the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`; the user's guide ch. 13; Ami Pro printing VM fixtures to a PostScript file; the desktop's print portal and GTK's CUPS backend), `docs/Design/UI_SPEC.md` §29; the user's answers PO1–PO6 (2026-10-20: as recommended, but no bulk export).
- [x] **A straight run** (2026-10-20; `docs/PRINT_RUNBOOK.md`, record `docs/PRINT_PROGRESS.md`, tag `print-pdf-run-start`): File › Export PDF…, Print…, Print Current Page…, and Print Options… (the Print panel in the rail, the pages shown as they print); the PDF drawn by the page surface itself through krilla (fonts embedded and subset, text selectable, vector pictures, each page at its layout's size); Ami Pro's Crop marks, Without pictures, With doc description, On preprinted form, Update fields; bookmarks, Doc Info properties, page labels, PDF/A-2b; printing through the XDG print portal.
- [x] The user's physical check passed (the user, 2026-10-20: "printing works correctly", "pdf works correctly"; the Print panel "looks good", no issues with Print Current Page…); the panel's buttons span the rail since (the user's choice).
- [ ] Next in the Phase 5 queue: export (run 6).

## Export (Phase 5, queue run 6)

- [x] Map and proposal (2026-10-20): `docs/research/EXPORT_NOTES.md` (the decompiled `rtf.flt`; Ami Pro 3.1 exporting VM fixtures through its RTF and Word for Windows 2.0 filters, which drop frames; the formats; LibreOffice 24.2), `docs/Design/UI_SPEC.md` §30; the user's answers EX1–EX10 (2026-10-20: DOCX, ODT, RTF, and text through one pipeline; File › Export ▸; frames as floating boxes; LibreOffice; table lines mapped and drawn; one document and a whole folder; losses listed first; live lists and fields; Ami Pro's style names; properties, Open, remembered folder).
- [x] **A straight run** (2026-10-20; `docs/EXPORT_RUNBOOK.md`, record `docs/EXPORT_PROGRESS.md`, tag `export-run-start`): File › Export ▸ Word Document, OpenDocument Text, Rich Text, Plain Text, and Export Folder… (a whole folder with a report), through one pipeline with a filter per format (`amipro-exchange`); frames where Ami Pro places them, tables with their lines and shading, live lists and fields, Doc Info as properties; the loss list before writing; Open after an export. Table lines and shading (Ami Pro's Lines & Color) are drawn and printed since. Every corpus document exports without failure; LibreOffice shows 36 of 40 user and 21 of 30 ISO sample documents on the app's page count; Ami Pro opens the app's RTF (twins).
- [x] The user's physical check, in LibreOffice Writer (`UI_ACCEPTANCE.md`, "Export"): passed 2026-10-24.
- [x] **Import** (2026-10-24, the queue's run 7; `docs/IMPORT_RUNBOOK.md`, record `docs/IMPORT_PROGRESS.md`, tag `import-run-start`): File › Open reads Word Documents, OpenDocument Text, Rich Text, and text through Ami Pro's Import Options into a protected tab; Insert › File…; Paste of RTF; a dropped document opens; the Import Report in the rail; File › Import Folder… into `.amipro` documents; EMF pictures drawn; one exchange document read by a reader per format and made into the model by `realise` (IM1–IM12, IC1–IC33). Ami Pro opens the app's SAM of imported documents as the app shows them (twins).
- [x] The user's physical check of import (`UI_ACCEPTANCE.md`, "Import"): passed 2026-10-25; export's passed 2026-10-24.
- [x] Next in the Phase 5 queue: table editing (run 8), below.

## Table editing (Phase 5, queue run 8)

- [x] Map and proposal (2026-10-24): `docs/research/TABLE_EDIT_NOTES.md` (every Table command in the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`; Ami Pro 3.1 fixtures TE1–TE20, TF0X, TF0Y), `docs/Design/UI_SPEC.md` §32; the user's answers TD1–TD11 (2026-10-24: an always-present Table menu; Table Properties in the rail with direct row and column commands; Ami Pro's selection gestures and Shift+arrows; columns narrowing when there is no room; Tab in the last cell adds a row; grid lines and the ruler's table columns; Distribute, text to table and back, table styles; a native-only line colour; the clipboard and drag and drop; no questions, Undo instead; one run).
- [x] **A straight run** (2026-10-24; `docs/TABLE_EDIT_RUNBOOK.md`, record `docs/TABLE_EDIT_PROGRESS.md`, tag `table-edit-run-start`): `TableLayout` and `CellFormat` in the model, native v3, and SAM as Ami Pro writes them; `amipro_editing::tables` (Insert Table, every Table menu command, Paste of cells, text to table and back), each one undo step; layout (leaders, line colours, Center table on page, edited rows fitting their text, a table on a line of its own); the Table menu, right-click items, Insert Table's dialog, Table Properties in the rail, cell selections, the table keys, grid lines and the ruler, Copy, Cut, Paste, and drag and drop of cells. Twelve twins open in Ami Pro as the app made them.
- [x] The user's physical check (`UI_ACCEPTANCE.md`, "Table editing"): passed 2026-10-25, after the right-click fix (`a46224d`).
- [x] Next in the Phase 5 queue: drawing (run 9), below.

## Drawing (Phase 5, queue run 9)

- [x] Map and proposal (2026-10-25): `docs/research/DRAWING_NOTES.md` (the decompiled `AmiDraw.dll`, `DrawMan.dll`, and `AMIPRO.EXE`; Ami Pro 3.1 fixtures DR1, DR2B, DRBIG, DRW0–DRW3), `docs/Design/UI_SPEC.md` §33; the user's answers DW1–DW9 (2026-10-25: one run; Draw mode in place with Drawing Properties in the rail; the draw bar replacing the toolbar; an always-present Draw menu; the full set of additions; SDW, SVG, and PNG in and out and on the clipboard; charts drawn, edited later; the frame keeps its size; Esc, Done, or a click outside leaves).
- [x] **A straight run** (2026-10-25; `docs/DRAWING_RUNBOOK.md`, record `docs/DRAWING_PROGRESS.md`, tag `drawing-run-start`): `amipro_pictures::sdw` (the stream read and written, shapes built as Ami Pro builds them, natural size, painting) and `svg`; every drawing and chart drawn from its objects on screen, in PDF, and in exports; `Command::SetDrawing` and `amipro_editing::drawing`; Insert › Drawing…, Draw mode (tools, gestures, selection, handles, point editing, text objects, keys, smart guides), the draw bar, the Draw menu, Drawing Properties in the rail, Import Drawing…, Save Drawing As…, Copy, Cut, Paste (DK1–DK13). Twins open in Ami Pro as made; Ami Pro edits and re-saves the app's drawing unchanged.
- [x] The user's physical check (`UI_ACCEPTANCE.md`, "Drawings and Draw mode"): passed 2026-10-26.

## Notes, bookmarks, footnotes, endnotes (Phase 5, queue run 10)

- [x] Map and proposal (2026-10-26): `docs/research/NOTES_BOOKMARKS_FOOTNOTES.md` (the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`; Ami Pro 3.1 fixtures RNN1–RNN3, RNB1–RNB3, RNF1–RNF6, RNF71S–RNF74S), `docs/Design/UI_SPEC.md` §34; the user's answers NR1–NR10 (2026-10-26, all as recommended).
- [x] **A straight run** (2026-10-26; `docs/NOTES_RUNBOOK.md`, record `docs/NOTES_PROGRESS.md`, tag `notes-run-start`): notes, footnotes, table footnotes, Footnote Options, and bookmarks in the model, native v3, and SAM; footnotes laid out as Ami Pro lays them out; edited in place; the Notes, Bookmarks, and Footnotes panels; Go To's Next item; bookmark fields and cross-references; Find's footnote stream; With notes; PDF destinations; DOCX, ODT, RTF out and in (NK1–NK10). Twins open in Ami Pro as made; the user's corpus page counts 315 → 318 of 394.
- [x] The user's physical check (`UI_ACCEPTANCE.md`, "Notes, bookmarks, footnotes, endnotes"): passed 2026-10-26.

## Proofing (Phase 5, queue run 11)

- [x] Map and proposal (2026-10-26): `docs/research/PROOFING_NOTES.md` (the decompiled `AMIPRO.EXE`, `AMIPROUI.DLL`, `LTSSPL1.DLL`, `LTSTHES1.DLL`, `LTSGRM1.DLL`; Ami Pro 3.1 fixtures PSP1–PSP4B, PGR1, PLN1), `docs/Design/UI_SPEC.md` §35; the user's answers SP1–SP8 (2026-10-26, as recommended but en-US bundled too).
- [x] **A straight run** (2026-10-26; `docs/PROOFING_RUNBOOK.md`, record `docs/PROOFING_PROGRESS.md`, tag `proofing-run-start`): the document's language and grammar set typed (`[lang]`, `[GramStyle]`), `<:s>` dropped from edited paragraphs; `amipro_editing::proofing`; Spell Check, Thesaurus, Grammar Check, Readability Statistics, and Language in one Proofing panel; spelling as you type; `spellbook`, MyThes, Harper; hyphenation by language (SK1–SK5). Round trip and layout as at the start; twin APF1 opens in Ami Pro as made.
- [x] The user's physical check (`UI_ACCEPTANCE.md`, "Proofing"): passed 2026-11-03; Harper kept (SK4, the user's choice).
- [x] TOC, index, glossary (run 12): see below.

## TOC, index, glossary (Phase 5, queue run 12)

- [x] Map and proposal (2026-11-03): `docs/research/TOC_INDEX_NOTES.md` (the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`; Ami Pro 3.1 fixtures TI1, TI2, TI3, GLO0), `docs/Design/UI_SPEC.md` §36; the user's answers TX1–TX8 (2026-11-03, all as recommended).
- [x] **A straight run** (2026-11-03; `docs/TOC_INDEX_RUNBOOK.md`, record `docs/TOC_INDEX_PROGRESS.md`, tag `toc-index-run-start`): `[toc]`, `[master]`, and the marks typed; the TOC as Ami Pro's page table with its break (page numbers start again after it); the index by Ami Pro's sort; the Contents & Index panel; TOC links; glossary files, Ctrl+K, the Glossary panel (TQ1–TQ6). Twins ATI1, ATI1X, ATI3, AGL1 open in Ami Pro as made.
- [ ] The user's physical check (`UI_ACCEPTANCE.md`, "TOC, index, glossary").
- [x] Master documents (run 13) done for its automated parts (below).

## Master documents (Phase 5, queue run 13)

- [x] **A straight run** (2026-11-03; `docs/MASTER_RUNBOOK.md`, record `docs/MASTER_PROGRESS.md`, tag `master-run-start`; MS1–MS11): `[master]`'s list and `[sequence]` typed; a member's pages, footnotes, numbered styles, and Seq fields continue from `[sequence]` (Ami Pro's MD2 and MD3 reproduced); the Master Document panel; the set numbered live in the app; the set's TOC and index; print and PDF of the set; export of the set as one DOCX, ODT, RTF, or text (MS10); Find across the set (MS11); twins in Ami Pro (the app's master and members worked to MD3's TOC and index). **Physical check passed (2026-11-03).**
- [x] Run 14, `merge`, done for its automated parts (below).

## Merge, labels, envelopes (Phase 5, queue run 14)

- [x] **A straight run** (2026-11-03; `docs/MERGE_RUNBOOK.md`, record `docs/MERGE_PROGRESS.md`, tag `merge-run-start`; MG1–MG11, MC1–MC13): `[recfile]` typed and written as Ami Pro writes it, conditions native; data files of every kind (Ami Pro's, tables, DIF, fixed, comma, CSV/TSV, DBF, XLSX, ODS); the merge by Ami Pro's rules (MGM1–MGM7 reproduced); the Merge panel (fields, preview, conditions, labels, Start from); every output (New Document, PDF one or per record, Printer…, View & Print, Ami Pro File…); the record editor; labels on the app's Avery sheets; File › Print Envelope… and the Envelope panel; twins in Ami Pro (Ami Pro merges the app's letter to the app's result). **Physical check pending.**
- [x] **Run D, `objects`** (2026-11-07; `docs/OBJECTS_RUNBOOK.md`, record `docs/OBJECTS_PROGRESS.md`; DO.1-DO.12, OD1-OD12): OLE objects drawn and kept, Paste Special, image processing, charts, equations; gate green; physical check pending.
- [ ] Next: the closing pass (accessibility, IME), then Phase 6 packaging; no run E is planned.
  - CP.1 (2026-11-07): IME preedit is drawn at the caret and a commit is one undo step (automated; physical IME check pending). Accessibility of the page surface is blocked by the pinned iced_winit, which never sends widget a11y trees to the adapter (evidence in `UI_ACCEPTANCE.md` under the editor gate); icon-button names added.

## Phase 5 — Advanced Ami Pro functionality

- [ ] Tables (in text and in frames), graphics, drawing, charts, equations, references, indexes/TOC, fields, mail merge, templates, language tools and other verified features, built on the Phase 2 model.
  - **Playbook** (2026-10-18, at the user's request): `docs/PHASE5_PLAYBOOK.md` groups these into seven playbooks in `docs/playbooks/` (Graphics, Tables, Fields and references, Proofing, Output and exchange, Merge and templates, Views and commands), with Ami Pro 3.1's verified inventory, the recipe every run follows, and a queue of 23 straight runs, starting with pictures (F4). Any item may be improved on or extended with logical functions, and whatever format is chosen for export is also imported, in the run after it (user, 2026-10-18).
- [ ] Outline mode (user's guide ch. 11; user decision 2026-10-17): the outline view with level icons, collapse and expand, moving sections. (Levels, Promote and Demote, outline numbering, and Quick Numbering were pulled forward into the outline levels run, 2026-10-17.)
- [x] (done 2026-11-06, awaiting the physical check) After run B (user, 2026-11-05; `DECISIONS.md` DF1, MD1–MD3): a Documents folder in Preferences › Folders where the file choosers start, and Markdown (CommonMark + GFM) opened, inserted, exported, and saved as an exchange format.
- [x] Implement print/PDF from shared layout output; expand parity tests (the print-pdf run, 2026-10-20; physical check pending).
- [ ] If possible add RTF export or DOC/DOCX whichever is most feasible (queue runs 6 and 7): export DONE 2026-10-20 (DOCX, ODT, RTF, text); import DONE 2026-10-24 (the same four formats).
  **Exit:** Each verified feature has evidence and passing acceptance checks.

## Phase 6 — Quality and release

- [ ] Performance, accessibility, crash recovery, large-document tests, fuzzing and compatibility matrix.
- [ ] Produce COSMIC package and installation instructions; validate on user's machine.
  - P6.1 (2026-11-07): `tools/package-deb.sh` builds `dist/amipro-linux_<version>_amd64.deb`; `docs/INSTALL.md`. Package checked off-host (contents, ldd, lintian); the real install on the user's machine is pending.
- [ ] Publish accurate coverage and known limitations; defer macros until separately scheduled.
  **Exit:** Stable release with transparent parity status.

## Execution and reporting

Work in small independent increments, commit after tests, update parity matrix as evidence changes. Maintain `docs/research/DECISIONS.md` for nontrivial decisions. At the end of each session record last commit, commands/tests/results, known blockers and next unblocked task. Do not ask the user to approve routine next steps. Never mark a phase complete without executable proof.

For major new compatibility work, follow `docs/REFERENCE_DRIVEN_DEVELOPMENT.md`
and update `docs/research/REFERENCE_FEATURE_MATRIX.md`. Use focused original-VM
comparisons and reuse research only when they can affect the next decision; do
not repeat broad architecture audits.

## Current status (2026-10-11)

Phase 0 is complete and the initial COSMIC application is host-verified.

- **Sequencing:** the page, page-layout, and frame foundation is now Phase 2, ahead of UI-3, UI-4, and style-sheet work (user decision, 2026-10-11).
- **Reference access:** the agent may operate original Ami Pro 3.1 in the `Windows XP` VM directly, including snapshots (user authorization, 2026-10-11).
- **SAM import:** Phase 1 imports all 160 private SAM corpus files as editable text with named paragraph styles and 10,174 corpus-inferred bold/italic character runs. It resolves available sibling STY files case-insensitively, reports missing and unsupported structures, and disables legacy write-back.
- **Importer fix (2026-10-11), verified in original Ami Pro:** untagged paragraphs are Body Text, escapes and character codes decode, and multi-line notes no longer truncate documents. 28,748 corpus paragraphs (was 25,927).
- **Rich editor (feature-gated):** it has physical evidence for corrected interactions and normal-feeling `README31.SAM` typing.
- **Default editor:** its per-edit whole-document work was removed at `8b78ecf`. All measured operations are sub-millisecond (median) headless; physical confirmation is pending.
- **UI:** P1 Revision 2 is approved at `2037467`. UI-1 is physically accepted for the functionality tested. UI-2 is implemented and automated-green at `96aeda4`, awaiting physical COSMIC acceptance.
- **Phase 2.1 (2026-10-11):** page and frame format specified from VM fixtures (`docs/research/SAM_PAGE_FRAME_SPEC.md`); `<:t n>` table-anchor import defect fixed.
- **Phase 2.2 (2026-10-11):** model decided (D1–D8 in `docs/research/DECISIONS.md`, target architecture in `ARCHITECTURE.md`), after the reference checkpoint and VM checks V10–V23/X06.
- **Next unblocked task:** Phase 2.3a, identity and `Arc`-shared roots, with the app's history moved onto them and the performance gate above.
- **Still open:** accessibility, visual IME preedit, and later UI slices. Phase 1 is not complete.
