# Frames in Table Cells Runbook

**Written:** 2026-10-18, after the user's answers to the frames in table
cells proposal (`docs/Design/UI_SPEC.md` §24; decisions T1–T3 in
`docs/research/DECISIONS.md`).
**Purpose:** let one session, possibly resumed after context compaction,
carry the frames in table cells run from start to finish **as a straight
run**: every decision it needs was taken before it starts, so it does not
stop in the middle to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions so far, `docs/Design/UI_SPEC.md` §24 the approved interface (with
§11's frame interaction and §20's frame creation). This runbook adds the
order, the gates, and the stop rules.

**Live state** is `docs/CELL_FRAMES_PROGRESS.md`. Keep it current: it is
how a resumed session knows where it is.

**Goal.** A picture in a table cell belongs to its cell: it is laid out
over the cell's text area wherever the table goes, on Ami Pro's table
grid; a click puts the caret in the cell and outlines it; Delete removes
the picture as one undo step; typing and Backspace do nothing, with a note
in the status bar; deleting the table deletes its pictures; Save As SAM
writes them first in Ami Pro's order with each cell naming its frame, and
the native format keeps them; Ami Pro opens the app's files with the
pictures in their cells. Then the user's physical check.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/CELL_FRAMES_PROGRESS.md`,
   `docs/research/CELL_FRAME_NOTES.md`, and `docs/Design/UI_SPEC.md` §11,
   §20, and §24; then `git status` and `git log --oneline -15`.
2. If the tag `cells-run-start` does not exist, create it at HEAD. It is
   the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE`.
   Uncommitted changes belong to an interrupted run: inspect, verify, and
   finish them; never discard them.

## 2. Decisions already taken (do not ask again)

- **T1:** a click on a cell's picture puts the caret in the cell and
  outlines the cell (a 43-twip band around its text area, as Ami Pro's
  `1488:14f0`); no handles, no drag, no Frame Properties; Format › Frame ▸
  and Select Next Frame leave it out; Delete removes it as one undo step;
  Backspace and typing do nothing, and the status bar says "This cell
  holds a picture; Delete removes it."
- **T2:** every cell's picture is laid out at its cell's text area and
  written at that size (the ten stored with no height included).
- **T3:** no way to put a picture in a cell in this run (Insert › Image…
  does it in the pictures run); Insert › Frame… and Draw Frame keep making
  page frames.
- **From the evidence:** tables on Ami Pro's grid (the insets, header
  values 8 and 9; each row's and column's gutter, from `[h]`/`[w]` with
  flag bit 4, else the header's values 4 and 6; column c's text at the
  left inset plus each earlier column's width and gutter); cell frames
  kept at the front of the frame stack in Ami Pro's list order (the newest
  first), written first, numbered from it.
- **Earlier decisions that apply:** Save As SAM to new files only with the
  loss check (A, B); rail inspectors over modal dialogs; a convention used
  on one item goes on all its siblings; frame interaction as §11.
- **Already authorized** (`AGENTS.md`): decompiling any binary as needed,
  operating Ami Pro in the XP VM (including snapshots), writing `.SAM`
  files under Phase 4's decision A.

## 3. Working rules

The outline run's rules apply unchanged (`docs/OUTLINE_RUNBOOK.md` §3):
the disassemblies first, findings into `CELL_FRAME_NOTES.md`; the approved
interface; the pinned libcosmic's traps (`CLAUDE.md`); twin fixtures in
the VM named `ATF…`; measure before and after (the round-trip audit on the
user's corpus with `AMIPRO_STYLES`, the ISO corpus, and the VM folder, and
`layout_comparison_report` on both corpora and the VM folder) for any
change that can move text or change written files, and explain every
difference; timings of the new behaviour and typing A/B; small green
commits (`tools/verify.sh`); native v3 records for any model addition;
aggregate numbers only, never the user's file names; push only at the end
of the session.

## 4. Stop rules

There is no planned stop. Stop only for what `AGENTS.md` lists. Anything
new goes in the progress file as `WAITING FOR USER` with the conservative
choice taken, and is asked at the end, batched and ranked.

## 5. Slices, in order

### CF.1 The map (evidence, no product code)

Done before the run was set up: `CELL_FRAME_NOTES.md` (the three lists
`1248:0347`/`11c8:3ba0`, the cell's values 11 and 12 `1490:07c0`/`1428:07ce`,
putting a frame in a cell `14d0:0000`, the caret in a frame's cell
`1488:0e10`/`14f0`, the table grid `1428:0000`/`046b`), VM fixtures
TF1–TF15, §24, T1–T3.

### CF.2 Ami Pro's table grid

`amipro-document` `Table`: the insets and each row's and column's gutter
(validation; native v3 records, older files reading the defaults);
`amipro-formats`: read them (header values 8 and 9, `[h]`/`[w]` gutters
with bit 4), write them back as read; `amipro-layout`: each cell's text at
the inset plus earlier sizes and gutters, the cell's box to the middle of
the gutters (the first column and row from the table's edge), the table's
height and the anchored table's height from them. Tests from TF1, TF13,
TF14, TF14X, BPS. Measure before and after: the layout comparison moves
only in tables with other gutters or insets; explain.

### CF.3 Frames in cells in the model

`Placement::InCell`; `TableCell::frame`; validation (each cell frame
named by exactly one cell, in the frame stack, before the other page
frames, not grouped, not a table); native v3; `amipro-editing`: deleting
a table's anchor deletes its cell frames (one step, undone together),
`Command::DeleteFrame` on a cell frame clears its cell; tests (the random
undo test included).

### CF.4 Import and the SAM writer

Import frames named by cells as `InCell` (a `0x20` frame no cell names
stays as read, with a warning); write the cell frames first, in stack
order, flag `0x20` from the placement, each cell's values 11 and 12 from
it, the frame's size its cell's text area from the model, the page and
corner as read while the table and cell are unchanged, else from the
table's written rectangle. Round trip on all three folders before and
after (read back equal, sections equal); the loss check lists nothing.

### CF.5 Layout

The engine places each cell's frame over its cell's text area (T2), right
above its table in the page's frames, on the table's page; no wrap
exclusion; incremental layout relays them with their table. The layout
comparison gains a cell-frame column (same page, |dx|, |dy|, size against
the stored rectangles); measure.

### CF.6 The page surface

Drawn above the cells; a click on a cell's frame puts the caret in its
cell (widget path), the cell outlined while the caret is there; no
selection, drag, or handles; Select Next Frame and Format › Frame ▸ leave
cell frames out; Delete in a frame's cell removes the frame (one undo
step); Backspace and typing do nothing, with the status note; tests
through messages and the window; screenshots light and dark (README31's
page 12, a TF fixture).

### CF.7 Twins

In the VM: the app's versions of TF2 (lines above the table), TF3 (a row
grown by typing), TF10 (a page break before the table), and TF7 (the
picture deleted), made from TF1 or TF11 in an ignored `cell_frame_twins`
test and written as `ATF…`, their frame and cell records compared with
Ami Pro's; Ami Pro opens each with the pictures in their cells; README31
edited in the app (a page break before its SmartIcon table, as EDK1) opens
in Ami Pro with the pictures in their cells.

### CF.8 Both formats and performance

Documents with cell frames read back from `.amipro` and SAM with nothing
in the loss check; timings (typing in a picture's row, Delete on a
picture, a page break before README31's table, to the drawn frame);
typing A/B against `cells-run-start`; the final measures.

### CF.9 Exit

§24's implementation notes, `FEATURE_PARITY.md`, `UI_COMMAND_MATRIX.md`,
`UI_ACCEPTANCE.md` (a section for frames in cells), `PROJECT_PLAN.md`,
`PROJECT_STATUS.md`, `CLAUDE.md`, `docs/research/FORMAT_COMPATIBILITY.md`
(the open row), the handoff with a short ranked list (the physical check
with exact steps).

## 6. Exit

Met when pictures in cells follow their cells as §24 describes under
T1–T3, tables are laid out on Ami Pro's grid, Delete and Undo work on them,
documents round-trip through both formats with nothing in the loss check,
and Ami Pro shows the app's twins with the pictures in their cells. The
user's physical check follows.
