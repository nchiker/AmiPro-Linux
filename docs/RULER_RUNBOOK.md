# Ruler Runbook (tabs, indents, margins, and columns)

**Written:** 2026-10-13, after the user's answers to the ruler proposal
(`docs/Design/UI_SPEC.md` §19; decisions R1–R4 in
`docs/research/DECISIONS.md`).
**Purpose:** let one session, possibly resumed after context compaction,
carry the plan section "The ruler: tabs, indents, margins, and columns"
(`PROJECT_PLAN.md`) from start to finish **as a straight run**: every
decision it needs was taken before it starts, so it does not stop in the
middle to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the decisions
so far, `docs/Design/UI_SPEC.md` §19 the approved interface. This runbook
adds the order, the gates, and the stop rules.

**Live state** is `docs/RULER_PROGRESS.md`. Keep it current: it is how a
resumed session knows where it is.

**Goal.** The page surface's ruler shows what Ami Pro's current ruler
shows at the caret (the tabs and indents of the caret's paragraph, the
margins and every column of its box) and edits them as Ami Pro does: tabs
into the paragraph's own ruler, indents as paragraph formatting, margins
and columns into the box's layout, unequal columns included; each gesture
one undo step; the result written in both formats and opened by Ami Pro
in the VM as Ami Pro's own twin is.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/RULER_PROGRESS.md`, and
   `docs/Design/UI_SPEC.md` §11, §15, §16, §18, and §19; then
   `git status` and `git log --oneline -15`.
2. If the tag `ruler-run-start` does not exist, create it at HEAD. It is
   the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE`.
   Uncommitted changes belong to an interrupted run: inspect, verify, and
   finish them; never discard them.

## 2. Decisions already taken (do not ask again)

- **R1:** tabs set on the ruler go into the paragraph's own ruler (a copy
  of the stops in effect plus the change), on the caret's paragraph or
  the selected ones; Enter carries it on. The style's and the page
  layout's tabs stay in Style Properties and Page Layout Properties.
- **R2:** unequal columns are allowed (drag a gap or a column), with a
  model change: a layout box keeps its column edges when they differ, the
  engine lays them out, native and SAM keep them, and the rail panels show
  them with an Even Columns action.
- **R3:** the ruler is always live: no ruler mode; a tab-type selector
  (with leaders) at its left end; right-click menus; changes apply on
  release, one command and one undo step each, with a guide line on the
  page during a drag.
- **R4:** no vertical ruler in this run.
- **Earlier decisions that apply:** rail inspectors over modal dialogs;
  a convention used on one item goes on all its siblings; Save As SAM
  writes new files only (Phase 4 decision A); the loss check (B); one
  length rule for every panel (`lengths.rs`).
- **Already authorized** (`AGENTS.md`): decompiling any binary as needed,
  operating Ami Pro in the XP VM (including snapshots), and writing
  `.SAM` files under decision A.

## 3. Working rules

The page layout setup run's rules apply unchanged
(`docs/LAYOUT_SETUP_RUNBOOK.md` §3): the disassemblies are authoritative
for what Ami Pro does (the decompiled `AMIPRO.EXE`, `AMIPROUI.DLL`, and
`AMIUICNT.DLL` in `../AmiPro-Reference/ghidra-work/out/`; findings into
the notes as VERIFIED (decompile), the implementation written
independently); the approved interface decides how it looks and works
(deviations from Ami Pro recorded in §19's implementation notes); the
pinned libcosmic's traps (`CLAUDE.md`, "Watch for this"); twin fixtures
in the XP VM for every gesture (name them `RU…`; 8.3 names through
`L:\IN` and `L:\OUT`); measure before and after (the round-trip audit on
both corpora with `AMIPRO_STYLES` set, and `layout_comparison_report` on
the user's corpus, the ISO corpus, and the VM fixtures); performance (a
drag's release to the pages drawn on README31 and the largest user
document; typing unchanged, interleaved A/B); small green commits
(`tools/verify.sh`); native v3 records for any model addition with
round-trip and strictness tests, older files loading; the three-approach
timebox; push only at the end of the session.

Ruler specifics:

- **The ruler is evidence-led too.** Hit areas, what a click or drag
  does at each place, limits (narrowest column, gutter, indent ranges,
  tabs not passing their neighbours), what Clear Tabs and Cols do, and
  what a margin drag does to unequal columns come from the decompile
  first and the VM second.
- **Commit aggregate numbers only**, never the user's file names or text.

## 4. Stop rules

There is no planned stop. Stop only for what `AGENTS.md` lists (a missing
reference, a destructive action, a licence question, or a material
product decision these documents do not cover). If such a question comes
up, record it in the progress file as `WAITING FOR USER` for the slice it
affects, take the conservative choice that keeps the approved interface,
carry on with every slice that does not depend on it, and ask at the end,
batched and ranked, with a recommendation each. Physical checks are
batched at the end too; never ask the user to test what the tests, the
screenshots, or the VM can show.

## 5. Slices, in order

### RU.1 The map (evidence, no product code)

- In the decompile: the current ruler's code in `AMIPRO.EXE` (drawing,
  hit testing, the click and drag handlers, the keyboard), how a tab
  change makes the paragraph's ruler (the copy rule, which paragraphs,
  table cells), the indent drags (which marker sets what, limits), the
  margin and column drags (gap move, column move, limits, what happens to
  unequal columns), Cols, Clear Tabs, the leader and type buttons, and
  how a frame's, header's, or cell's ruler differs; Page › Ruler ›
  Insert and Remove; whether Text › Normal and Revert to style touch the
  paragraph's ruler.
- VM: fixtures for what the decompile leaves open (RU1 and RU2 exist).
- Output: `docs/research/RULER_NOTES.md` and the map at the top of the
  progress file.

### RU.2 Paragraph tabs and indents

- Commands to set the paragraph's own ruler (or remove it) and its
  indents over a range of paragraphs, with Ami Pro's rules from RU.1;
  one function for "the tabs in effect" shared by the engine and the
  ruler; editing tests; SAM after edits.

### RU.3 Unequal columns

- The model (column edges kept when they differ), validation, native v3,
  the importer and both writers (the kept `.edges` offsets become model
  data), the engine, `layout_setup`'s rules (which changes make columns
  even again), and Page Layout Properties and Frame Properties (unequal
  shown, Even Columns). Measure the round trip and the layout comparison
  before and after, and explain every changed row.

### RU.4 The ruler's model

- A pure module: what the ruler shows for a caret (its box, columns, the
  caret's column, tabs in effect, indents), hit testing, and each gesture
  turned into a command. Tested without the widget.

### RU.5 The interactive ruler

- Drawing (every column, the caret column's tabs and indents, margins),
  hover names and positions, drags with a guide line on the page, release
  to one command and one undo step, the tab-type selector, View › Ruler
  as a saved check item. Tests through messages and pointer events;
  screenshots light and dark.

### RU.6 Right-click menus on the ruler

- On a tab (type, leader, Remove), on the top half (Clear Tabs, Remove
  Paragraph Ruler), on the bottom half (Columns ▸, Even Columns, Page
  Layout… or Frame Properties). Never inside a scrollable.

### RU.7 Frames, headers and footers, and table cells

- The ruler follows the caret's box: a text frame's interior, a band, a
  table cell (tabs and indents only), as RU.1 finds.

### RU.8 Keyboard

- Edit › Go To › Ruler; the ruler cursor and Ami Pro's keys (arrows,
  Ctrl+arrows, Space, Delete, 0–3, `[` `]`, Esc), as RU.1 finds.

### RU.9 Twins

- Every gesture made in Ami Pro and in the app on the same fixture,
  saved as SAM and compared (an ignored `ruler_twins` test like
  `layout_twins`); Ami Pro opens the app's twins as its own.

### RU.10 Both formats and performance

- Documents edited on the ruler written as `.amipro` and SAM, read back
  equal, nothing in the loss check; the round trip and the layout
  comparison on every corpus; the release-to-pages time; typing A/B.

### RU.11 Exit

- §19's implementation notes, screenshots, `FEATURE_PARITY.md`,
  `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, and the handoff,
  ending with a short, ranked list for the user (the physical check, with
  exact steps).

## 6. Exit

Met when the ruler shows and edits tabs, indents, margins, and columns as
§19 describes, by the rules RU.1 maps; each gesture is one undo step;
documents edited on the ruler round-trip through both formats with
nothing in the loss check; and Ami Pro in the VM gives the app's twins
the pages it gives its own. The user's physical check follows.
