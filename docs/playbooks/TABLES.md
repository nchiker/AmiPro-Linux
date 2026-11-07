# Tables Playbook

Part of `docs/PHASE5_PLAYBOOK.md` (queue rows 2, 8, 20). Follow its recipe
and standing rules; this file says what each run covers, what to map, and
what to expect.

**Why one playbook.** Ami Pro has two kinds of table, and both are table
frames with cells that hold their own paragraphs:
- tables in the text, anchored by `<:t n>`;
- tables in frames: a table made on a selected frame becomes a frame
  table, and a Where placed page table is one too (V21).

Both kinds share one grid model (`TableSpacing`, spans, cells as flows,
pictures in cells) and one layout path, so every table run builds on the
last one.

**State today** (`TB-01` PARTIAL; the table editing run, 2026-10-24, made
every Table command but the table-data run's; before it:
- Tables continue across columns and pages as Ami Pro continues them
  (whole rows, headings repeated, page breaks on rows, Rows span pages),
  and Insert › Page Break in a cell sets a row's break
  (`docs/research/TABLE_PAGES_NOTES.md`, `UI_SPEC.md` §26).
- Table › Headings and Modify Table Layout's Automatic and Rows span pages
  are in the model (`Table::row_options`, `Command::SetTableRows`) but have
  no command yet: run 2 builds them.
- Rows in cells of faces no one-line main-text paragraph calibrates
  measure 5–7 twips a line taller than Ami Pro's (the substitute fonts);
  whether to trust Ami Pro's stored row heights while a row is as read is
  a question for the user (the handoff, 2026-10-19).

**Before the tables across pages run** (the frames in table cells run):
- Tables import and round-trip, and lay out on Ami Pro's grid (the
  insets and each row's and column's own gutter).
- Joined cells, Table Text, and row growth within a column work.
- Cell text is editable, and pictures sit in their cells.
- Tables are not split across pages, and there are no table commands.
- Cell fields beyond the grid (lines, shading, protection, formulas,
  number formats) are kept as legacy data and written back. Whether each
  cell's lines and shading are drawn as stored is to be checked at the
  start of run 2.
- The registry has Insert › Table… as a Future command.

**Evidence to start from.**
- **Guide:** ch. 16. It covers creating a table and typing text and
  numbers, pasting, and linking. It covers pictures, drawings, charts,
  and equations in cells, moving around, selecting, and formatting. It
  covers editing: undo, moving and copying with Drag & Drop, deleting
  extra spaces, and deleting a picture or a table. It covers Modify Table
  Layout, Automatic row height, Honor protection, Lines and color, rows
  and columns, sizes, Connect cells, Headings, Leaders, and protecting
  cells. It also covers cell addresses, formulas and Quick Add, footnotes
  in a table, tabs, page breaks in a table, sorting, a table as a merge
  data file, a chart from table data, and forms. Ch. 21 covers Sort.
- **Commands:** Tools › Tables… (0x1c9) and the Table menu (1057):
  - Modify Table Layout… (0x272), Lines & Color… (0x278);
  - Insert Column/Row… (0x273), Delete Column/Row… (0x274), Delete
    Entire Table (0x28b), Column/Row Size… (0x27a);
  - Select Column (0x28c), Select Row (0x28d), Select Entire Table
    (0x28e);
  - Connect Cells (0x27b), Headings (0x277), Leaders ▸ (0x27f–0x282),
    Protect Cells (0x27e);
  - Edit Formula… (0x275), Quick Add ▸ Row (0x283) and Column (0x284).

  Tools › Sort… is 0x1c8.
- **Decompile already mapped:** the grid (`1428:0000`, `046b`) and a
  cell's values 11 and 12 (`1490:07c0`, `1428:07ce`). The cell fields
  that hold lines, shading, protection, formulas, and number formats are
  still to map (`[data]` records), as are `[e]` and `[tble]`.
- **In the files:** 389 tables in 172 user files and 214 in 72 ISO files.
  About 80 of 220 continue on a later page (an earlier count; re-measure).
  108 pictures sit in cells, six of them in rows on a later page. A
  table's natural height equals Ami Pro's stored one in 159 of 389 user
  tables (`CELL_FRAMES_PROGRESS.md`, CF.2).

## Run 1: `table-pages`, queue 2

**Goal.** Tables continue across pages as Ami Pro continues them: rows
move whole or split as Ami Pro does (to map), Headings rows repeat on each
page, and page breaks in a table work. Cell text is measured as Ami Pro
measures it, so a row's height equals Ami Pro's stored one.

**Map.**
- The pagination step's rule for table rows (`12f8:1321` and the table
  routines): what moves to the next page, whether a row splits, and what
  happens to a row taller than a page.
- Headings rows, and a page break inside a table (ch. 16).
- Why cell text measures taller here: Table Text's spacing, the cell's
  line height, and the cell margins (READMEBP's long cells; one table 75
  twips taller).

**VM fixtures.**
- A table crossing a page end, with and without Headings.
- A row taller than the rest of the page.
- A page break in a table.
- Pictures in cells on the second page.
- A table in a frame that does not fit its frame.

**Decisions to expect.** Probably none beyond behaviour. If Ami Pro
cannot split a row, consider an improvement that is visible only in the
app and never written.

**Measures.** This run moves text, so the layout comparison is its main
gate: page counts, page ends, and the table rows against Ami Pro's stored
heights. It also covers frames in cells on later pages, which the CF.5
report counts.

**Exit.** Corpus page ends up (each changed row explained); the
continuing tables of both corpora compared with Ami Pro's page caches;
twins.

**Done 2026-10-19** (overnight, `docs/TABLE_PAGES_PROGRESS.md`). Learned
for later runs: Ami Pro's page tables are Flow with text frames on a line
of their own; the page table (`[pg]`) names each page-end table's first
row and row count, a ready oracle; the stored table rectangle is the part
laid out last; a repeated heading costs one more 43-twip inset.

## Run 2: `table-editing`, queue 8

**Goal.** Tables are made and edited with Ami Pro's commands in the text
and in frames, each command one undo step, and written as Ami Pro writes
them.

**Scope.**
- **Making a table:** Tools › Tables… makes one at the caret, or a frame
  table on a selected frame.
- **Layout:** Modify Table Layout (default sizes, gutters, Automatic row
  height, Honor protection, lines around cells).
- **Structure:** Insert and Delete Column/Row, Delete Entire Table, and
  Column/Row Size, also by dragging the grid lines and the ruler's column
  markers in a table (the ruler run left a cell's box at tabs and indents
  only).
- **Selection and moving:** Select Column, Row, and Entire Table, which
  needs a rectangular cell selection; cut, copy, and paste of cell
  ranges; Drag & Drop of rows and columns; and moving a picture between
  cells.
- **Cell formatting:** Connect Cells (spans), Headings, Leaders, Protect
  Cells, and Lines & Color (each cell's sides, line style, colour, and
  shading, through the colour button). Headings and Modify Table Layout's
  Automatic and Rows span pages need only their commands: the model,
  files, and layout are done (`Command::SetTableRows`, run 1).
- **Moving around:** Tab and Shift+Tab between cells, as Ami Pro does (to
  map whether Tab in the last cell adds a row).

**Map.** Each command's handler and dialog in `AMIPRO.EXE` and
`AMIPROUI.DLL`, the cell fields each one writes, and Ami Pro's limits.

**Decisions to expect.**
- **Where the commands live.** Recommend a Table Properties panel in the
  rail for the table, row, column, and cell, and Ami Pro's Table menu for
  the commands, shown while the caret is in a table.
- **The cell selection gestures.**
- **Lines & Color.** Recommend in the rail with the colour panel.
- **Whether to split the work** into structure, then formatting, if it is
  large.

**Better ways and logical additions to offer.**
- Insert Row Above/Below and Column Left/Right as direct commands.
- Distribute rows and columns evenly.
- Table styles from Ami Pro's line and shading options.
- Convert text to a table and back.
- Repeat Headings on by default for new long tables, if Ami Pro allows
  it.

**Exit.** Message-path and widget-path tests of every command; the round
trip of edited corpus tables (Save As lists nothing); twins of each
command opened in Ami Pro.

**Done 2026-10-24** (`docs/TABLE_EDIT_PROGRESS.md`, TD1–TD11, TK1–TK11).
Learned for later runs: Ami Pro undoes none of its table commands (the app
undoes every one); a cell's fill is F3 `0x200` with F10, black too; F9 is
protection and F3 `0xC000` the leader; covered cells keep their text in the
file; a table made in the text carries frame flag `0x100000`; a page table
always stands on a line of its own; the row cache's fingerprints must be
taken after the import's last derived-value pass. For run 3: the cell
records' F3 `0x40` marks a formula body after the text (MERCINV); Edit
Formula's dialog is `TBLEDITFORMULAPROC`; Quick Add is `1420:0dc5`; Sort is
`1360:0000`.

## Run 3: `table-data`, queue 20

**Goal.** Cell addresses and formulas (Edit Formula, Quick Add for a row
or column), number formats, Tools › Sort… for paragraphs and table rows,
and forms (protected label cells, printing without labels), each as Ami
Pro has them.

**Map.** The formula syntax and when it recalculates, the number formats
and their storage, Sort's options (keys, alphanumeric or numeric,
ascending or descending, field delimiter for paragraphs), and form
printing.

**Decisions to expect.** Recalculating formulas as you type, if Ami Pro
recalculates on demand (recommend automatic, an improvement Ami Pro's
files can hold since the results are cell text). Sort as a dialog or in
the rail.

**Better ways and logical additions to offer.** More functions than Ami
Pro's if the syntax can hold them (the native format only, listed by Save
As), a sort preview, and a numeric column's alignment on the decimal.

**Built 2026-11-06** (`docs/TABLE_DATA_PROGRESS.md`, TD.1–TD.12; physical
check pending; `UI_SPEC.md` §41):
- **Formulas** (`@sum`, `+ - * / %`, names, `$`): `crates/document/src/formula.rs`;
  Edit Formula… in the Table menu and the rail's Formula section; the
  engine recomputes every corpus formula as Ami Pro's (261 of 261 user,
  15 of 15 ISO).
- **Quick Add** Row and Column, and Recalculate Table (`tables.rs`,
  `table_formula.rs`).
- **Number formats** through the style (`number.rs`; the Styles rail's
  Table format section); a typed number is rewritten on leaving the cell.
- **Tools › Sort…**: `crates/editing/src/sort.rs`, Ami Pro's heap sort;
  `sort_panel.rs`.
- **Merge of numbers**: a table data file's values merge as Ami Pro merges
  them (`crates/editing/src/merge/data.rs`).
- **Forms**: no new code; the recipe is scenario 11 (`scenario_tests/table_data.rs`).
- **Extras** (native and SAM, listed by Save As SAM): `@avg`, `@min`,
  `@max`, `@count`.

Learned for later runs: a calculated cell is F3 `0x40` with two bodies
(the result, then the formula text as typed); the `[nfmt]` flag word is
0x08 thousands, 0x10 leading minus, 0x20 trailing minus, 0x40
parentheses, 0x80 red, 0x100 leading currency symbol. Not built: Sort's
3186 (DDE escape) and 3215 (too large); Paste and Cut into a formula cell
are not refused yet (TD.7 open item).

## Related runs elsewhere

Charts from table data (Graphics, `charts`), a table as a merge data file
(Merge and templates, `merge`), and footnotes in a table (Fields and
references, `notes`).
