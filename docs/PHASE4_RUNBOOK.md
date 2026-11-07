# Phase 4 Runbook (SAM compatibility: the SAM writer)

**Written:** 2026-10-13, after Phase 3's physical check passed (`36acf89`).
**Purpose:** let one session, possibly resumed after context compaction,
carry Phase 4 (`PROJECT_PLAN.md`) from start to finish **as a straight run**
(user request): every decision it needs was taken before it started
(`docs/research/DECISIONS.md`, 2026-10-13, "Phase 4 as a straight run"), so
it does not stop in the middle to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the decisions
so far. This runbook adds the order, the gates, and the stop rules.

**Live state** is `docs/PHASE4_PROGRESS.md`. Keep it current: it is how a
resumed session knows where it is.

**Goal.** Phase 4's exit: "Representative original documents import and
edit without silent loss." Concretely: a document opened from a SAM, edited
or not, can be saved as a new SAM that Ami Pro 3.1 opens and lays out as
it laid out the original, and anything that cannot be written exactly is
named before saving.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`, this runbook,
   and `docs/PHASE4_PROGRESS.md`; then `git status` and
   `git log --oneline -15`.
2. If the tag `phase4-run-start` does not exist, create it at HEAD. It is
   the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE`.
   Uncommitted changes belong to an interrupted run: inspect, verify, and
   finish them; never discard them.

## 2. Decisions already taken (do not ask again)

- **A. New files only.** File › Save As offers Ami Pro (`.SAM`) beside
  `.amipro` and never overwrites an existing file, including the SAM the
  document came from (as Save as a Style Sheet, Phase 3 D3). Save on a
  document whose location is a SAM keeps routing to Save As. The writer
  writes only to paths the user picks, and tests write only to temporary
  directories. Nothing ever writes into the user's corpus or the ISO
  extraction.
- **B. Warn, then let the user choose.** Before writing a SAM, the app
  checks what the writer cannot write exactly and, if anything, shows it
  in a dialog with Cancel and Save Anyway (the default is Cancel).
- **C. Page layout setup comes after Phase 4,** in its own plan section.
  This run only writes the proposal for it (slice 4.12); it builds none of
  it.
- **Version 4 only.** Every readable SAM in both corpora is format version
  4 (417 and 160); the rest are Lotus Word Pro files or encrypted, and stay
  refused with a message. The writer writes version 4.
- **Already authorized** (`AGENTS.md`): decompiling any binary as needed,
  operating Ami Pro in the XP VM (including snapshots), and writing `.STY`
  files under D3's rule.

## 3. Working rules

- **Decompiled code is the primary source** (user direction): the
  decompiled `AMIPRO.EXE` (`../AmiPro-Reference/ghidra-work/out/amipro.c`;
  routine map in `docs/research/SAM_FORMAT_NOTES.md`) comes first for what
  Ami Pro writes and in what order. The document writer is `11c8:1c7b`.
  The style sheet writer `11c8:14e0`, the style record writer `11c8:26b7`,
  the layout writer `11c8:236e`, the frame writer around `11c8:2a0f`, and
  the paragraph writer `11b0:1405` are already decoded, and the project's
  `writer.rs`, `sheet.rs`, and `body.rs` implement them. The XP VM
  (`docs/research/VM_LAB.md`, helpers in `tools/vm/`) confirms: Ami Pro
  opens what the writer wrote, and lays it out as it laid out the original.
- **Decompile whatever else the work needs** without asking: deeper
  passes over the writer's callees, the `[Embedded]` writer, the filters in
  `../AmiPro-Reference/Binaries/` as a cross-check. Methods are in
  `docs/research/DECOMPILATION.md`; decompiler output stays outside Git;
  findings go into the format notes as VERIFIED (decompile); the
  implementation is written independently, never transliterated.
- **The round trip is the measure.** From slice 4.2 on, every slice runs
  the round-trip audit on both corpora (user's corpus with
  `AMIPRO_STYLES` set, and the ISO corpus through `tools/corpus.sh`'s
  extraction) and records its aggregate numbers in the progress file.
  Commit numbers only, never the user's file names or text; keep any
  per-file output in the scratchpad.
- **Measure layout changes.** The writer does not touch layout. A slice
  that changes the importer or the model in a way that can move text
  (for example by keeping a field the importer used to drop) runs
  `layout_comparison_report` on the user's corpus and the VM fixtures
  before and after, as in Phase 3, and explains any change.
- **Small green commits** on `master`: fmt, strict Clippy (core and the
  app's normal, `--all-features`, and `--no-default-features` builds), and
  all suites before each commit (`tools/verify.sh`); `tools/corpus.sh`
  after importer changes. The VM fixtures stay at their page counts and
  page ends unless a change is explained.
- **Native format:** any model addition gets native v3 records
  (`docs/Design/NATIVE_FORMAT_V3.md`), with round-trip and strictness
  tests; older files keep loading.
- **Unknown structures are never dropped silently** (`AGENTS.md`): what the
  importer keeps as legacy data is written back in place; what the model
  cannot carry is reported by the loss check (decision B).
- **Performance:** saving is not on the typing path. Record the time to
  write the largest corpus document (release); it should stay well under
  the time to read it.
- **Timebox:** a problem that resists three distinct approaches is recorded
  in the progress file with its evidence, and the next independent slice
  starts. In a straight run, a blocked slice does not stop the run: it is
  listed for the handoff.
- **Push** only at the end of a session, after the handoff commit
  (`CLAUDE.md`, "Backup").

## 4. Stop rules

There is no planned stop. Stop only for what `AGENTS.md` lists (a missing
reference, a destructive action, a licence question, or a material product
decision these documents do not cover). If such a question comes up, record
it in the progress file as `WAITING FOR USER` for the slice it affects,
carry on with every slice that does not depend on it, and ask at the end,
batched and ranked, with a recommendation each. Physical checks are
batched at the end too; never ask the user to test what the round trip,
the VM, or the tests can show.

## 5. Slices, in order

### 4.1 The writer's map (evidence, no product code)

- Decode the document writer `11c8:1c7b` and its callees: every section it
  writes, in order, with its source in Ami Pro's state and the condition
  for writing it. Header sections (`[ver]` to `[GramStyle]`, `[desc]`,
  `[sty]`), which styles a SAM stores as `[tag]` records (document styles;
  what a style changed from the sheet writes), when `[lay]` is written
  ("only when its page layout was changed there") and inserted layouts,
  frames, tables, pictures and the `[Embedded]` directory (offsets,
  trailer, linked files in `[files]`), notes, `[pg]`, the `<:#…>`
  paragraph caches, and the end of the file.
- Write it as "Writing documents" in `SAM_FORMAT_NOTES.md`, with a table
  like "Writing style sheets", and for each item its model source: typed
  data, legacy data, derived, or a cache.
- **Caches** (VM): take VM fixtures and corpus samples, strip `[pg]` and
  `<:#…>`, open them in Ami Pro, and check whether it opens them without
  complaint and lays them out the same (re-save, compare the caches it
  writes with the original's). That decides whether the writer omits the
  caches or must compute them.
- Done when: every section and control in both corpora is in the table
  with its routine and model source, and the cache question is answered
  with evidence.

### 4.2 Loss inventory and the round-trip audit

- List what the importer drops or derives that a writer needs (the
  mapping table in `docs/research/FORMAT_COMPATIBILITY.md`: frame fields
  13–15, table frames' field 16, `[frmlay]` bottom edge, width, and values
  12–14, the `[Embedded]` offsets and trailer, the caches), and decide each
  from 4.1: keep as read, recompute, or omit. Keep what must be kept
  (model, native v3, importer), measuring layout if it can move text.
- The round-trip audit (an example in `amipro-formats`, or a mode of
  `corpus_audit`): import, write, import again, and compare the two models
  (structure and text, ignoring caches); plus a section-by-section
  comparison with the original file (sections equal, values equal, order).
  Aggregate counts only. Its numbers are the measure of every later slice;
  until the writer exists they are zero.
- The loss check's core: a function that lists what a document holds that
  the writer cannot write exactly (decision B). It starts from the
  inventory and shrinks as slices land.
- Done when: the inventory is in `FORMAT_COMPATIBILITY.md`, the audit runs
  on both corpora, and the baseline is recorded.

### 4.3 The document head and plain text

- The writer's frame: header sections from the model (kept values where
  the importer kept them; Ami Pro's defaults for new documents, from 4.1),
  `[desc]` (description, keywords, save times as Ami Pro writes them),
  `[sty]`, the document's `[tag]` records (`writer.rs`), unknown top-level
  sections written back in their place, `[edoc]` with the main text's
  paragraphs (`body.rs`), and the end.
- VM: the plain fixtures (T01–T05, ST1, EP1) written by the writer open in
  Ami Pro and look as the originals do.
- Done when: documents with only text and styles round trip exactly in the
  audit, and the VM check passes.

### 4.4 The main text in full

- Character runs and formats (`<+X>`/`<-X>`, `<:f…>`), paragraph overrides,
  end formats, rulers and tabs (`<:R…>`), protected text, no hyphenation,
  overstrike, hard returns and discretionary hyphens, page and column
  breaks (`<:p…>`, `<:c>`), page-number fields, floating headers and
  footers (`<:H…>`, `<:h…>` with their flows), notes, and legacy marks
  written back verbatim. The decompiled control writers give the order and
  the spelling (Ami Pro writes each Text menu attribute as an on and off
  pair).
- Done when: the audit shows main text round tripping in every corpus
  document whose other content is not yet written; VM fixtures for
  breaks, fields, and floating headers open and lay out as the originals.

### 4.5 Page layouts

- `[lay]` as the SAM writes it (when, and which layouts: inserted layouts
  named by `<:p…>`), right and left pages, header and footer frames with
  their text, and repeating frames, reusing `sheet.rs`'s layout writer.
- VM: fixtures G00–G23 written and opened; page counts and header
  positions as the originals.
- Done when: the audit's layout sections match, and the VM check passes.

### 4.6 Frames

- `[frm]` records for page frames: placement, wrap, lines, shadow, corners,
  background, groups and stacking order, names, macros, `[frmlay]`, `[txt]`
  (no paragraph, one empty line, or text, as read), and the fields kept as
  read; the anchors `<:A n>` and their one numbering with tables.
- VM: F01–F20 and V10–V23 written and opened; the frames stand where they
  stood (Ami Pro's re-saved rectangles equal the originals').
- Done when: the audit's frame records match, and the VM check passes.

### 4.7 Tables

- `[tbl]` frames: the grid, row heights and column widths, joined cells,
  cell text, each cell's record and formulas kept as read, the header, and
  `<:t n>` anchors.
- VM: TB1, V20, and corpus tables (MERCURY.SAM's centred table among them)
  written and opened.
- Done when: the audit's table records match, and the VM check passes.

### 4.8 Pictures and embedded objects

- `[isd]` records, linked pictures in `[files]`, and the `[Embedded]`
  directory with its offsets and trailer recomputed for the bytes written;
  the objects' bytes copied verbatim; OLE objects as read.
- VM: the picture fixtures and corpus documents with pictures and OLE
  objects open with their pictures.
- Done when: the audit's picture records and embedded bytes match, and the
  VM check passes.

### 4.9 Caches, as 4.1 decided

- Omit `[pg]` and `<:#…>` if Ami Pro rebuilds them on opening; otherwise
  compute them from the layout engine in Ami Pro's units and test them
  against the originals.
- Done when: written documents carry what 4.1 found Ami Pro needs, and the
  VM's re-saved caches of written documents equal the originals'.

### 4.10 Save As SAM in the app (decisions A and B)

- Save As offers Ami Pro (`.SAM`) beside `.amipro` (the portal's file type
  filters; the name's extension decides when typed), new files only: an
  existing target is refused with a message, never replaced. An imported
  SAM's Save still routes to Save As. The loss check runs first; if it
  finds anything, a dialog lists it with Cancel (default) and Save Anyway.
  Writes go to a temporary file and are renamed, as native saves are.
  The status bar says what was written.
- Record the design in `UI_SPEC.md` as implementation notes (a file type
  choice and one dialog, as decided; no new screens).
- Tests: the message path (both types, refusal of an existing file, the
  loss dialog's two answers), screenshots in light and dark, and a written
  file read back.
- Done when: tests pass and a document saved as SAM from the app opens in
  Ami Pro in the VM.

### 4.11 With contents, and edited documents

- Save as a Style Sheet's With contents (Ami Pro's default, on): the
  dialog's check box, and the sheet writer writes the text, the frames, and
  the `[Embedded]` directory as 4.3–4.8 do (`SAM_FORMAT_NOTES.md`, "With
  contents off" gives the rule for layout frames with 0x8). VM: a sheet
  written with contents starts a new document with its text in Ami Pro.
- Edited documents: tests that edit each kind of content (text in every
  flow, styles, a frame moved and resized, a table cell, a page break) and
  then write, read back, and compare; a few edited files opened in the VM.
- Done when: both are tested and VM-checked.

### 4.12 Phase exit and the page layout setup proposal

- Final audit numbers on both corpora, the VM results, and the loss
  check's remaining list in `FORMAT_COMPATIBILITY.md` and
  `FEATURE_PARITY.md` (IO-01, IO-02, IO-04).
- Write the proposal for page layout setup (decision C) as a new
  `UI_SPEC.md` section: a Page Layout Properties inspector in the context
  rail, from Ami Pro's Modify Page Layout (Margins & Columns, Page Settings,
  Lines, Header, Footer; VM screenshots), Insert › Floating Header/Footer,
  the page-number field, inserted layouts, how the rail chooses between
  Style, Frame, and Page Layout Properties, and the questions it needs
  answered. Build none of it.
- Update `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, and the
  handoff, and end with a short, ranked list for the user: the page layout
  proposal to approve first, then a short physical check of Save As SAM.

## 6. Phase exit

`PROJECT_PLAN.md`: "Representative original documents import and edit
without silent loss." Met when the round-trip audit shows both corpora's
documents written and read back without loss outside the loss check's
list, the VM shows Ami Pro opening written documents and laying them out
as the originals, and Save As SAM works in the app under decisions A and
B.
