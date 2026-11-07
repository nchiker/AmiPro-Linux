# Table Editing Runbook

**Written:** 2026-10-24, the Phase 5 queue's run 8 (`docs/PHASE5_PLAYBOOK.md`,
`docs/playbooks/TABLES.md`, "Run 2: `table-editing`"). **Purpose:** carry
making and editing tables from start to finish as a straight run. Its
decisions were asked before the run and answered on 2026-10-24
(`docs/research/DECISIONS.md`, "Table editing: the user's answers",
TD1–TD11); the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §32 the interface. Evidence:
`docs/research/TABLE_EDIT_NOTES.md` (every Table command in the decompile
and Ami Pro 3.1 fixtures TE1–TE20), with `CELL_FRAME_NOTES.md`,
`TABLE_PAGES_NOTES.md`, and `EXPORT_NOTES.md` ("Lines & Color").

**Live state** is `docs/TABLE_EDIT_PROGRESS.md`.

**Goal.** Tables are made and edited with Ami Pro's commands, in the text
and in frames, each command one undo step, and written as Ami Pro writes
them: Insert › Table…; a Table menu; Table Properties in the rail (the
table's defaults and options, rows' and columns' sizes, cells' lines,
line colour, fill, leaders, protection, joins); cell selections; rows and
columns inserted, deleted, sized, distributed, dragged, and moved; Tab
between cells (a new row from the last); Copy, Cut, Paste, Delete, and
drag and drop of cells; text to table and back; table styles. Ami Pro
opens what the app writes as its own (twins).

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/TABLE_EDIT_PROGRESS.md`,
   `docs/research/TABLE_EDIT_NOTES.md`, and `docs/Design/UI_SPEC.md` §32;
   then `git status` and `git log --oneline -15`.
2. The tag `table-edit-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **TD1** A Table menu root after Format, always present, greyed outside a
  table; the row, column, and cell items on the right-click menu in a
  table.
- **TD2** Table Properties in the rail (Table, Rows and columns, Cells) in
  place of Modify Table Layout, Column/Row Size, Insert Column/Row, and
  Lines & Color; Insert Row Above/Below and Column Left/Right as direct
  commands (as many as are selected; the rail takes a count); Insert
  Table keeps a small dialog.
- **TD3** Ami Pro's selection gestures, and Shift+arrows crossing a cell's
  edge make a cell selection.
- **TD4** A column with no room: the columns narrow in proportion; the
  table keeps its width; the status bar says so.
- **TD5** Tab in the last cell adds a row; Ctrl+Down or Esc leaves.
- **TD6** Grid lines drag as Ami Pro's (no Shift variant); the ruler shows
  the table's columns and drags them the same way.
- **TD7** Distribute Rows/Columns Evenly, Convert Text to Table and back,
  table styles (Grid, Outline, Header row, Banded rows, Plain).
- **TD8** A table line colour, native only; Save As SAM writes black and
  lists it.
- **TD9** Delete empties cells; Copy, Cut, Paste of cell ranges as Ami
  Pro's; drag and drop of rows, columns, and blocks; Alt+Up/Down move rows.
- **TD10** No questions: every command acts at once and is one undo step.
- **TD11** One straight run.
- **Choices made in the run** are labelled TK1, TK2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** rail inspectors over modal dialogs;
  consistent conventions (check marks on every on/off item, Leaders too);
  the colour button and panel (§23, C1–C4); frames in cells (T1–T3);
  tables across pages and the row cache (TP1–TP5, the row cache); SAM
  sources protected; Save As SAM lists what Ami Pro cannot hold.

## 3. Working rules

- **Every command is one `amipro_editing::Command`** (several changes as
  `Command::Several` or one table command), applied to a new root, so
  Undo and Redo work as for text; tested through the app's message path
  and the widget path (real clicks and keys through the `Runtime`).
- **The model owns what Ami Pro stores.** Each cell's lines, fill,
  leader, protection, and joins, the table's defaults and options, and
  rows' and columns' own sizes and gutters are model fields, read from and
  written to SAM as Ami Pro writes them (TABLE_EDIT_NOTES.md), kept in
  native v3; the kept records (`cell.record`, `tbl.header`, `tbl.h`,
  `tbl.w`) keep only what the model does not hold, and files Ami Pro wrote
  still read back equal (the round-trip audit).
- **Ami Pro's rules for sizes:** new rows and columns at the table's
  defaults; own sizes flagged; the table frame follows its grid; an edited
  Automatic row fits its text (RC2), an empty cell takes a line (RC3).
- The evidence first: a rule not in the notes is looked up in the
  decompile or made as a VM fixture before it is built.
- The pinned libcosmic's traps (`CLAUDE.md`): no zero-size children in a
  row or column; an operation-only task chains a repaint; never an id that
  changes at a fixed place; no `context_menu` inside a scrollable; a text
  field focused by a task hears no focus message; **tests click portal
  commands with `Runtime::click_unrun`** so no real dialog opens on the
  user's screen; tests never touch the user's settings.
- Small green commits: targeted tests, strict Clippy, rustfmt, a commit
  per slice. Routine full suites, Clippy, fmt, and corpus measures run
  through lower-cost subagents (or background scripts) that report back.
- Aggregate numbers only, never the corpora's file names.
- **Text that moves is measured.** The round-trip audit and
  `layout_comparison_report` (`target/table-edit-baseline/baseline.sh`)
  run at the start and the exit, and before and after any slice that
  touches the layout, model, or SAM crates; every changed row is explained.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour, is recorded (TK…), and goes into the handoff. If the run grows
past one long session, hand off at a slice boundary with the progress file
current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| TE.1 Setup | `TABLE_EDIT_NOTES.md`, `UI_SPEC.md` §32, TD1–TD11, this runbook, the progress file, the baseline (`target/table-edit-baseline/`), the tag | Committed; `table-edit-run-start` |
| TE.2 The model | `Table`: defaults (row height, column width), own sizes, options (Center, Honor protection, Disable sizing), the header's other values; `TableCell`: `CellFormat` (lines, line colour, fill, leader, protected); a row's and column's join flag as derived; validation; native v3 | Unit tests; native round trip; the corpora read back equal |
| TE.3 SAM in and out | The reader fills the new fields; the writer writes them as Ami Pro (records only where Ami Pro writes them, flags, F3/F7/F9/F6/F10, the `[h]` cell counts); Line around as the frame's line; Save As lists line colours | The round-trip audit equal on all three corpora; writer tests against TE fixtures |
| TE.4 Table commands | `amipro_editing::tables`: InsertTable (text and frame), InsertRows/InsertColumns (TD4), DeleteRows/DeleteColumns/DeleteTable, SetTableOptions, SetSizes (own and Reset), Distribute, Connect/Disconnect, SetCellFormat (lines with Ami Pro's All/Outline/sides rules and gutter widening, fill, line colour, leader, protect), Headings (SetTableRows), ClearCells, PasteCells, MoveRows/MoveColumns/MoveBlock, ConvertTextToTable/TableToText, ApplyTableStyle; Table Text as Ami Pro makes it | Unit tests per command, undo by root; SAM after edits read back (`tests/sam_after_edits.rs`) |
| TE.5 Layout | Leaders drawn by Ami Pro's rule; line colour; Center table on page; edited Automatic rows fit their text and empty cells take a line; the anchor mid-paragraph (TE20) | Layout tests; measures before and after |
| TE.6 Cell selection and keys | The surface's cell selection (gestures TD3), drawing, Tab/Shift+Tab (TD5), Ctrl+arrows, Home Home/End End, Honor protection, Alt+Up/Down | Message-path and widget-path tests |
| TE.7 Menus and Insert Table | The Table menu (TD1), right-click items, Insert › Table… dialog, the toolbar item, check marks | Tests; screenshots light and dark |
| TE.8 Table Properties | The rail panel (TD2): Table, Rows and columns, Cells sections; mixed values; colour buttons; table styles | Message-path and widget-path tests; screenshots light and dark |
| TE.9 Grid lines and the ruler | Dragging column and row lines (TD6); the ruler's table columns | Widget-path tests; the ruler benchmark |
| TE.10 Clipboard and drag and drop | Delete, Copy, Cut, Paste of cells (and text in and out), drag and drop of rows, columns, blocks (TD9) | Tests |
| TE.11 Measures | The round-trip audit and layout comparison against the start; timings (release) of the commands on README31's tables and a 100-row table; the typing A/B | Numbers in the progress file |
| TE.12 Twins | The app makes and edits tables (each command family) and writes SAM; Ami Pro opens them as the app shows them | VM screenshots outside Git |
| TE.13 Exit | `tools/verify.sh --release --corpus`, the measures, the records (`UI_SPEC.md` §32 notes, `FEATURE_PARITY.md` TB-01, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue and `TABLES.md`), the handoff, the push | Green; `origin/master` equals `HEAD` |
