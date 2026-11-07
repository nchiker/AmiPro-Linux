# Table Editing Progress

Live state of `docs/TABLE_EDIT_RUNBOOK.md` (tag `table-edit-run-start`).
Resume at the first slice that is not `DONE`.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| TE.1 Setup | DONE | (the setup commit) | `TABLE_EDIT_NOTES.md`; `UI_SPEC.md` §32; TD1–TD11 in `DECISIONS.md`; the baseline below | Tag `table-edit-run-start` |
| TE.2 The model | DONE | `d7beef4` | `TableLayout`, `CellFormat`; native `tablelayout`, `cellformat` (`table_layout_and_cell_formats_round_trip`) | TK1, TK3 |
| TE.3 SAM in and out | DONE | `d7beef4` | `cell_formats_and_table_options_are_read_and_written_as_ami_pro_writes_them`; the round-trip audit equal to the start on all three corpora | TK1, TK2 |
| TE.4 Table commands | DONE | `d7beef4` | `crates/editing/tests/tables.rs` (9 tests against TE1–TE19's records, one undo step each) | Convert text to table and back, and cell paste, come with TE.10 |
| TE.5 Layout | DONE | (the TE.5 commit) | `a_row_as_read_keeps_ami_pros_stored_height` (RC2, RC3), `a_centred_table_stands_in_the_middle_of_its_column_te13`, `cell_leaders_run_by_the_text_alignment_te10`; TE20's mid-paragraph table already laid out as Ami Pro's; measures below | TK4, TK5 |
| TE.6 Cell selection and keys | DONE | (the TE.6 commit) | `table_keys_and_cell_selections_through_messages`, `table_keys_and_cell_selections_through_the_window` (a click in a cell, Tab and Shift+Tab, a drag across cells, Delete, the column strip, Ctrl+Down); `double_clicks_enter_frame_text_and_a_click_in_a_table_enters_its_cell` | TK6, TK7 |
| TE.7 Menus and Insert Table | DONE | (the TE.7–TE.8 commit) | `insert_table_and_the_table_menu_through_messages` (every Table command one undo step, the checks, Disconnect Cells, the right-click items, Select ▸, text to table and back), `the_insert_table_dialog_takes_clicks_in_the_window`; `crates/editing/tests/tables.rs` `text_becomes_a_table_and_a_table_text`; screenshots light and dark (`table_screenshots`, ignored) | TK8 |
| TE.8 Table Properties | DONE | (the TE.7–TE.8 commit) | `table_properties_edit_the_table_live_in_the_rail` (sizes, lines, Center, Line around, Automatic off keeping the laid-out rows, fill and line colour through the colour panel, the panel closing outside a table; its buttons through the window); screenshots light and dark | |
| TE.9 Grid lines and the ruler | DONE | (the TE.9 commit) | `a_tables_column_line_drags_through_the_window` (that column only, one undo step, Disable mouse sizing); `in_a_table_cell_the_ruler_has_the_tables_columns_and_the_cells_tabs` (the table's columns, a right edge sizes its column, Distribute from the ruler's menu) | TK9 |
| TE.10 Clipboard and drag and drop | DONE | (the TE.10 commit) | `copied_cells_paste_over_cells_and_repeat_over_a_selection` (core); `cells_copy_cut_and_paste_through_the_clipboard` (the clipboard's text, cells pasted, tabbed text into cells, Cut, a whole table into the text), `selected_cells_drag_and_drop_through_the_window` (a row, a block) | TK10 |
| TE.11 Measures | DONE | (the handoff commit) | Round trip and summaries equal to the start (`te5b`, `te11`, `te12`); release: a 100 × 5 table's commands 2.4–3.2 ms median with undo 2.3–2.6 ms, typing in a cell 2.4 ms, Tab 1.2 ms; README31 typing 1.1–1.3 ms; the typing A/B (`tools/bench-ab.sh table-edit-run-start 2`) within noise; the ruler baseline 1.1 ms (indent) | |
| TE.12 Twins | DONE | (the TE.12 commit) | `table_twins` (ignored; `AMIPRO_VM_LAB`, `AMIPRO_TWIN_OUT`): ATE2, ATE3, ATE5, ATE8, ATE10, ATE11's cells and headers equal Ami Pro's TE2–TE11; Ami Pro 3.1 opened ATE1–ATE11, ATEL (lines style 3, a yellow row), ATES (Header Row), ATEM (a row moved, a column added with no room, TD4), ATEC (centred, Line around, 1.5 in columns), ATET (text to table) as the app made them (VM screenshots outside Git); found one layout fix (TK11) | TK11 |
| TE.13 Exit | DONE | (the handoff commit) | `tools/verify.sh --release --corpus` PASS at 1cb3933 (core 476 + 11 ignored; app 342, 115, 349 with 52, 6, 52 ignored; release builds; ISO corpus gate); the records | Physical check passed 2026-10-25 (the user: "tables pass") |

## Baseline (at `table-edit-run-start`, `target/table-edit-baseline/start/`)

Measured with `target/table-edit-baseline/baseline.sh` (outside Git; the
round-trip audit and the layout comparison over the user's corpus, the ISO
corpus, and the VM folder, each with `AMIPRO_STYLES`). The VM folder gained
the run's Ami Pro fixtures TE1–TE20.

- Round trip: user's corpus 417 of 417 read back equal (Save As would
  list nothing 417), ISO 159 of 160, VM folder 313 of 313.
- Layout comparison: user's corpus page counts equal 315 of 394,
  paragraph line counts 14,880 of 15,356, page ends on the same page 762
  of 1,108, Automatic rows at Ami Pro's height 2,639 of 2,777; ISO 69 of
  160, 22,773 of 24,971, 478 of 1,307, rows 1,225 of 2,056; VM folder 312
  of 312, 1,860 of 1,872, 112 of 122, rows 723 of 724.

**After TE.5** (`target/table-edit-baseline/te5b/`): the round trip and
every summary line equal to the start on all three corpora. Per file,
only the anchored tables' x moved (Center table on page, TK5): of 38
changed values in the three corpora, 27 fell to 0 (from 14 to 940 twips),
7 went from 0 to 3 (Ami Pro's centring passes through screen pixels), and
3 grew (1,728 to 1,811 twice; 1,080 to 1,221), in tables whose centred
frame Ami Pro stored elsewhere.

## Choices made in the run (TK…)

- **TK1** A cell's fill is F3 `0x200` with F10, black too; F6 (Ami Pro's
  palette index) is recomputed from F10 (`1490:0d0a`). VM TF0X (`0x200`
  with F10 0 is black; a corpus letter's heading row is black with white
  text, which the app had drawn unfilled) and TF0Y (F6 and F10 without
  the bit: nothing). 1,557 cells of the user's corpus are black fills.
- **TK2** The writer keeps the `[h]`/`[w]` records Ami Pro wrote as read:
  flag 1 (inside a joined block) only in records it writes itself (Ami
  Pro's reader sets it, `1458:041b`), flag 2 added only for a size the
  model makes its own. Own sizes are read from flag 2 alone; an Automatic
  table has none (Ami Pro flags every row it measured).
- **TK3** A table made in the text carries Ami Pro's `0x100000` as a kept
  frame flag (TE1's `0x1800C4`): With para above tables lack it (56 of 586
  anchored tables in the corpora), so it is not derived from placement.
- **TK4** The row cache's fingerprints are taken at the end of the import,
  after the derived ruler marks are dropped: rows whose cells held such
  marks never counted as read (one row of READMEBP's picture table).
- **TK5** Center table on page centres a page table in its column, by
  whole twips, whatever the paragraph's alignment (`1478:128a`).
- **TK6** A click in a table's cell puts the caret there, as Ami Pro does
  (TE1–TE19); before, a first click selected the table's frame. A table in
  the text is never selected by a click; a frame table is, on its edge
  outside the cells (and through Go To and Select Next Frame).
- **TK7** A cell selection is a page selection whose ends are in two cells
  of one table, a rectangle (`14a0:0000`). Typing with cells selected types
  in the focus's cell; Delete and Backspace empty the cells (one undo step,
  TD10). Home, Home and End, End are not built (the row's first and last
  cell are a Ctrl+arrow away).
- **TK8** Insert › Table… stays in the Insert menu only (the registry gives
  each command one menu); the Table menu starts with the row and column
  commands. The Insert Table dialog remembers the last size (Ami Pro offers
  5 × 5 each time). Table › Table Style ▸ and the rail's Style row apply
  Grid (style 1 everywhere), Outline (style 2 around), Header Row (row 0
  shaded E1E1E1, a heading, style 2 under it), Banded Rows (every second
  row F7F7F7, a grid), and Plain.
- **TK9** A grid line moves as far as the pointer does from where it was
  pressed (Ami Pro's outline follows the pointer); its hit zone is the
  gutter with two pixels' slack. On the ruler a table's columns show from
  their cells, and only a column's right edge is taken (left edges follow
  from the gutters); the lower half's menu offers Distribute Columns
  Evenly and Table Properties….
- **TK10** Copied cells are kept beside the clipboard's text (tabs between
  cells, a line per row, as Ami Pro's `1240:2636`); a paste of that text in
  a table pastes the cells with their look. Text with tabs or several
  lines pasted in a table fills cells from the caret's (Ami Pro asks before
  putting several lines in one cell; TD10: no question). A selection that
  does not match the copied block is refused in the status bar, as Ami
  Pro's "Shade range does not match copied data range." A dropped block
  that runs past the table is cut at its edge without asking (Ami Pro
  asks); a move empties the source first, then fills the target.
- **TK11** A table in the text stands on a line of its own however narrow
  (found by the twin ATEC, which the app drew beside "Above the table."):
  its anchor's word takes the column's width, so the text before it ends
  its line and the text after it starts the next. Measures: the round
  trip and the corpora's summaries unchanged; in the VM folder TE3's and
  TE16's narrow tables now stand where Ami Pro put them (284 → 8 twips
  down, 1,579 → 0 across), page ends in a table with the same rows 68 → 70
  of 71.

## Log

- 2026-10-24, after the run (the user): with the table selected as a
  frame, a right-click in its first cell showed only Frame Properties. The
  right press still selected a table's frame (TK6 covered the left button
  only), and a frame already selected kept its menu. Now a right press in
  a table's cells lets the frame go, puts the caret in the cell (or keeps
  the cell selection it is in), and the menu starts with Table Properties…
  (`a_right_click_in_a_table_cell_offers_the_table_commands`).
