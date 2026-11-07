# Page Layout Setup Runbook (headers, footers, and the page)

**Written:** 2026-10-13, after Phase 4 and the user's answers to the
proposal (`docs/Design/UI_SPEC.md` §18; decisions L1–L4 in
`docs/research/DECISIONS.md`).
**Purpose:** let one session, possibly resumed after context compaction,
carry the plan section "Page layout setup: headers, footers, and the page"
(`PROJECT_PLAN.md`) from start to finish **as a straight run**: every
decision it needs was taken before it starts, so it does not stop in the
middle to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the decisions
so far, `docs/Design/UI_SPEC.md` §18 the approved interface. This runbook
adds the order, the gates, and the stop rules.

**Live state** is `docs/LAYOUT_SETUP_PROGRESS.md`. Keep it current: it is
how a resumed session knows where it is.

**Goal.** The section's exit: "Page layouts, headers, and footers can be
set up as in Ami Pro and are saved in both formats." Concretely: every
setting of Ami Pro 3.1's Modify Page Layout (Margins & columns, Page
settings, Lines, Header, Footer; All, Right, or Left pages; Mirror),
inserted page layouts, floating headers and footers, and the page-number
field can be set in the app; the pages lay out again at once; Undo takes
each change back; and a document set up in the app, saved as `.amipro`,
as a SAM, or as a style sheet, opens in Ami Pro with the pages Ami Pro
gives the same settings made in Ami Pro.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/LAYOUT_SETUP_PROGRESS.md`, and
   `docs/Design/UI_SPEC.md` §11, §15, §16, and §18; then `git status` and
   `git log --oneline -15`.
2. If the tag `layout-run-start` does not exist, create it at HEAD. It is
   the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE`.
   Uncommitted changes belong to an interrupted run: inspect, verify, and
   finish them; never discard them.

## 2. Decisions already taken (do not ask again)

- **L1:** page layout editing is a rail panel, Page Layout Properties,
  live, one undo step per change, like Style Properties (§15).
- **L2:** its commands are under Format (Format › Page Layout…), as
  everything else; no Page menu. Floating headers and footers and the
  page-number field go under Insert (Insert › Floating Header/Footer…,
  Insert › Page Number…).
- **L3:** floating headers and footers and the page-number field are small
  one-shot dialogs: the interface keeps dialogs for one-shot actions with
  nothing to watch, and rail panels for what changes live.
- **L4:** the panel follows the caret's page while open.
- **Earlier decisions that apply:** Save As SAM writes new files only
  (Phase 4 decision A, revisit pending); the loss check lists what Ami Pro
  cannot store (B); rail inspectors over modal dialogs; a convention used
  on one item goes on all its siblings (check marks, toggles, units).
- **Already authorized** (`AGENTS.md`): decompiling any binary as needed,
  operating Ami Pro in the XP VM (including snapshots), and writing `.SAM`
  and `.STY` files under decisions A and D3.

## 3. Working rules

- **The disassemblies are authoritative for what Ami Pro does** (user
  direction): the decompiled `AMIPRO.EXE`
  (`../AmiPro-Reference/ghidra-work/out/amipro.c`, and
  `amipro-annotated.c` with section names) and, as the work needs them, the
  DLLs in `../AmiPro-Reference/Binaries/` (decompile any of them without
  asking; `docs/research/DECOMPILATION.md`; `w4w33f.c` is done) decide
  Ami Pro's behaviour: what each setting means, its units, ranges, limits,
  defaults, and validation messages; how a Right or Left change creates
  left pages; what Mirror, Begin on second page, Insert/Remove/Revert,
  floating headers and footers, and the page-number field's styles do;
  and how each is written (the layout writer `11c8:236e`, LAYOUT blocks
  `11c8:24d6`, the frame writer `11c8:2a0f`; routine map in
  `docs/research/SAM_FORMAT_NOTES.md`). Dialog resources in the NE
  resource table give the control IDs that tie a dialog procedure's code
  to its fields. Find the routine first; use corpus statistics and the VM
  to confirm, never in its place. Decompiler output stays outside Git;
  findings go into the notes as VERIFIED (decompile); the implementation
  is written independently, never transliterated.
- **The new interface decides how it looks and works.** Ami Pro's dialogs
  are evidence of what can be set, not of how to present it. Follow
  `UI_SPEC.md` §18 as approved, with §15's conventions: folding sections;
  a check box, list, or radio choice applies at once and a typed value on
  Enter or when its box loses focus; each change one command and one undo
  step; a value that does not read or does not fit is refused in place
  with the reason (Ami Pro's limits and wording from the decompile, in the
  interface's voice); inches as Style Properties shows them; the document
  is the preview (no preview pane). Use COSMIC widgets and theme tokens,
  the registry for every command (menus, shortcuts, enablement, check
  marks), and the rail's place in the tree as it is. Where Ami Pro and the
  approved interface differ, the interface wins; record each such choice
  in §18's implementation notes.
- **The pinned libcosmic's traps** (`CLAUDE.md`, "Watch for this"): give a
  `responsive` an explicit height; chain a repaint after an operation-only
  task; never give a fixed place in the tree a changing id; never put a
  zero-width placeholder in a `Row`/`Column`/`Stack`; never put an
  in-window context menu inside a scrollable; check that icon names exist
  in the COSMIC theme.
- **The XP VM confirms** (`docs/research/VM_LAB.md`, helpers in
  `tools/vm/`): for each setting, a twin fixture: the same change made in
  Ami Pro on a copy of a fixture, and made in the app and saved as SAM;
  Ami Pro opens both, and the screens and the re-saved layout records are
  compared. Name fixtures `PL01`…; files go through `L:\IN` and `L:\OUT`
  with 8.3 names. After an Ami Pro fault, restart the guest before
  testing again (`VM_LAB.md`, "After an Ami Pro fault").
- **Measure.** The importer and the writers should not change; a slice
  that touches them runs the round-trip audit on both corpora
  (`sam_round_trip`, with the app's loss check per file; user's corpus with
  `AMIPRO_STYLES` set) and `layout_comparison_report` on the user's corpus
  and the VM fixtures, before and after, and explains any change. Every
  slice that changes the engine's input measures layout. Commit aggregate
  numbers only, never the user's file names or text.
- **Performance:** a layout change relays every page. Record the time from
  a change to the pages drawn on README31 and on the largest user
  document (release, `FrameHarness`-style); typing must stay as fast as it
  is (the page surface typing benchmark, interleaved A/B with
  `tools/bench-ab.sh`).
- **Small green commits** on `master`: fmt, strict Clippy (core and the
  app's normal, `--all-features`, and `--no-default-features` builds), and
  all suites before each commit (`tools/verify.sh`). Tests drive the app
  through its messages (`page_surface_app_tests.rs`'s `Runtime` for
  pointer paths); screenshots in light and dark through
  `window_screenshots`.
- **Native format:** any model addition gets native v3 records
  (`docs/Design/NATIVE_FORMAT_V3.md`) with round-trip and strictness tests;
  older files keep loading.
- **Timebox:** a problem that resists three distinct approaches is recorded
  in the progress file with its evidence, and the next independent slice
  starts. A blocked slice does not stop the run: it is listed for the
  handoff.
- **Push** only at the end of a session, after the handoff commit
  (`CLAUDE.md`, "Backup"); confirm `origin/master` equals `HEAD`.

## 4. Stop rules

There is no planned stop. Stop only for what `AGENTS.md` lists (a missing
reference, a destructive action, a licence question, or a material product
decision these documents do not cover). If such a question comes up,
record it in the progress file as `WAITING FOR USER` for the slice it
affects, take the conservative choice that keeps the approved interface,
carry on with every slice that does not depend on it, and ask at the end,
batched and ranked, with a recommendation each. Physical checks are
batched at the end too; never ask the user to test what the tests, the
screenshots, or the VM can show.

## 5. Slices, in order

### PL.1 The map (evidence, no product code)

- In the decompile: the Modify Page Layout dialog procedure(s) and their
  panels (Margins & columns, Page settings, Lines, Header, Footer; Pages
  All/Right/Left; Mirror), with each field's meaning, unit, range,
  default, and validation (message and limit); the page sizes' twips;
  the Lines panel's styles, positions, and colours; the Header and Footer
  panels' band margins and Begin on second page; Insert Page Layout's
  Insert, Remove, and Revert; the Floating Header/Footer dialog; the Page
  Numbering dialog's styles, Start on page, Start with number, and
  leading text, and how each is stored (`<:P…>`). Where the dialog
  resources are needed, read them from the NE resource table.
- In the model and commands: what each setting maps to (`PageLayout`,
  `PageGeometry`, `LayoutBox`, `Border`, the header and footer frames'
  style and interior, `PageNumberField`; `ModifyLayout`, `InsertLayout`,
  `RemoveLayout`, `InsertBreak`, `InsertFloatingHeaderFooter`,
  `SetFrameStyle`), and the gaps.
- VM: the panels' screenshots (2026-10-13 ones in `vm-lab/screens/pl_*`),
  and fixtures for what the decompile leaves open.
- Output: `docs/research/PAGE_LAYOUT_NOTES.md` (fields, rules, evidence
  levels) and the field-by-field map at the top of the progress file.

### PL.2 Model and command gaps

- Whatever PL.1 finds missing: for example a page-number command
  (insert, and change an existing field), Ami Pro's validation of
  margins, columns, gutters, and page sizes in the commands, and the
  first Right or Left change creating left pages as Ami Pro does.
- Each change one command with `ChangeReport` set so that the pages lay
  out again; editing tests; native v3 for any model addition.
- Measure layout if the engine's input changes.

### PL.3 The panel's frame

- Format › Page Layout… in the registry (File › Page Setup, a disabled
  placeholder, is retired: L2 puts the command under Format); the page's
  right-click menu in a margin (§16 reserved it) and a double-click in a
  margin open it; the rail switches to it as to Style Properties; ‹ Styles
  returns and × closes; about 300 px.
- The rail's priority: Frame Properties (a frame selected), then the last
  opened of Page Layout Properties and Style Properties, then Styles.
- The Layout list (the caret page's layout, following the caret: L4),
  Pages (All, Right, Left) and Mirror, and the folding sections, empty.
- Tests through messages; screenshots light and dark.

### PL.4 Page

- Size (Letter, Legal, A4, A3, A5, B5, Custom with width and height) and
  orientation, as `ModifyLayout`; refusals by Ami Pro's limits.
- Twin fixtures in the VM.

### PL.5 Margins, columns, and tabs

- Left, right, top, bottom; 1–8 columns; gutter; Column balance; the
  page's tabs (type, leader, position), as Style Properties' Tabs section
  edits a style's (one convention for both).
- Twin fixtures in the VM.

### PL.6 Lines

- Lines around the page (sides, style, position, colour) and the line
  between columns (style); one convention with Frame Properties' lines.
- Twin fixtures in the VM.

### PL.7 Header and footer

- Each band's margins, columns, gutter, tabs, and Begin on second page;
  with the caret in a header or footer the panel opens that band's
  section. Edit › Go To Header and Go To Footer put the caret in the fixed
  header or footer, as §18 says.
- Twin fixtures in the VM.

### PL.8 Inserted page layouts and breaks

- Insert Page Layout (a copy of the caret page's layout from the next page
  on, by a page break naming it), Remove (an inserted layout no break
  names; break and layout in one undo step), and Ami Pro's Revert as an
  Insert that picks an existing layout in the Layout list.
- Insert › Page Break and Column Break enabled (`InsertBreak`), as the
  registry already lists them.
- Twin fixtures in the VM (layout names `1`, `2`, … as Ami Pro writes
  them).

### PL.9 Floating headers and footers

- Insert › Floating Header/Footer…: insert a floating header or footer at
  the caret for All, Right, or Left pages, or remove one there, as Ami
  Pro's dialog does (L3: a small dialog).
- Twin fixtures in the VM.

### PL.10 The page-number field

- Insert › Page Number…: style, Start on page, Start with number, leading
  text, inserted at the caret (usually in a header or footer); the same
  dialog changes a field at the caret. The page surface draws the number
  as Ami Pro numbers pages.
- Twin fixtures in the VM.

### PL.11 Both formats, edited documents, and the VM

- Documents set up in the app (each section's settings, inserted layouts,
  floating headers and footers, page numbers) written as `.amipro`, as a
  SAM (Save As, decision A), and as a style sheet with and without
  contents, read back equal, with nothing in the loss check; opened in
  Ami Pro in the VM with the pages its twins have.
- The round-trip audit and the layout comparison unchanged on both
  corpora; the change-to-pages time recorded.

### PL.12 Exit

- `UI_SPEC.md` §18 implementation notes (every deviation from Ami Pro's
  dialogs, with the reason), screenshots, `FEATURE_PARITY.md`,
  `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, and the handoff,
  ending with a short, ranked list for the user (the physical check of the
  panel and the two dialogs, with exact steps).

## 6. Exit

`PROJECT_PLAN.md`: "Page layouts, headers, and footers can be set up as in
Ami Pro and are saved in both formats." Met when every setting PL.1 maps
can be made in the app as §18 describes, each change is one undo step and
lays out at once, documents set up in the app round-trip through both
formats with nothing in the loss check, and Ami Pro in the VM gives them
the pages it gives the same settings made in Ami Pro. The user's physical
check follows.
