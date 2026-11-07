# Frames in Table Cells Progress

Live state of `docs/CELL_FRAMES_RUNBOOK.md`. Resume at the first slice
that is not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| CF.1 The map | DONE | (setup commit) | `docs/research/CELL_FRAME_NOTES.md`; VM TF1–TF15, TF1Y/TF1Z variants, BPS; `UI_SPEC.md` §24; T1–T3 | The three lists (`1248:0347`, `11c8:3ba0`); cell values 11 and 12 (`1490:07c0`, `1428:07ce`); `14d0:0000`; the caret in a frame's cell (`1488:0e10`, `14f0`); Ami Pro's table grid (`1428:0000`, `046b`) |
| CF.2 Ami Pro's table grid | DONE | (this commit) | `tables_lay_out_on_ami_pros_grid_tf14` (TF14's 3355 and 1475, TF14X's top inset, own gutters, grid lines mid-gutter), `tables_keep_their_insets_and_own_gutters` (header values 8 and 9, `[h]`/`[w]` gutters with flag 4 and READMEBP's without, written back byte for byte; new records in Ami Pro's full shape), `table_spacing_validates_and_round_trips` (native); the TB1/README31 test with the inset above a gutterless first row | `TableSpacing` (insets, own gutters by index) on `Table`; native `content table` gains the insets and `column`/`row` an own gutter, only when not Ami Pro's defaults; the engine's `TableGrid` and `track_span`. **Found and fixed:** a `[h]`/`[w]` record the writer had to add had two values, which Ami Pro's reader rejects (`1428:046b`); no command reached it yet |
| CF.3 Frames in cells in the model | DONE | (this commit) | `frames_in_cells_are_held_by_exactly_one_cell_and_round_trip` (validation: held by exactly one uncovered cell, not grouped or a table; native `incell` and the cell's frame; removing the frame empties its cell), `frames_in_cells_delete_with_undo_and_follow_their_table` (Delete as one undo step, the table relaid; deleting the table's anchor deletes its picture, one undo; move, resize, restack, and placement refused) | `Placement::InCell`, `TableCell::frame`, `Document::cell_holding`, `Frame::owned_frames`, `Table::cell_text_area`; `delete_frame_tree` deletes a table's cell frames; restacking keeps the three kinds apart |
| CF.4 Import and the SAM writer | DONE | (this commit) | `cells_hold_the_frames_their_records_name_tf1` (TF1's records written back byte for byte), `cell_frames_keep_ami_pros_order_and_numbers_readme31` (README31's order and numbers; deleting one renumbers and writes its cell `… 0 0`, as TF7), `never_sized_and_unnamed_cell_frames_tf1z` (T2: written at the cell's size at the corner read; a `0x20` frame no cell names stays where its record put it, first; a cell naming a missing frame keeps its numbers) | `cells_hold_frames` in the importer; `cell_frame_list`, `cell_frame_numbers`, `cell_frame_place` in the writer; the round-trip facts name each cell's frame and treat the stored rectangle as a cache. Save As lists nothing for any file in the three folders |
| CF.5 Layout | DONE | (this commit) | `cell_frames_cover_their_cells_text_area_tf1_tf13_tf14` (TF1's second cell, TF14's gutter, TF13's joined cells; right above the table), `table_cell_edits_move_the_text_after_the_table` (incremental equals full layout with a picture in a cell; it follows its table, grows with its row, TF3, and goes with Delete) | `table()` returns its cells' frames' boxes, which `frame_interiors` puts right after the table's; not text-wrap exclusions. The layout comparison reports frames in cells (page and place within their table against Ami Pro's stored rectangles) |
| CF.6 The page surface | DONE | (this commit) | `a_click_on_a_cells_picture_puts_the_caret_in_the_cell` (widget path: a press on the picture puts the caret in its cell, no frame message; the note; Ami Pro's outline bands), `a_cells_picture_takes_delete_and_ignores_typing` (messages: typing, Enter, Backspace change nothing; Delete and Edit › Delete remove the picture, one undo step; Select Next Frame passes it by); `cell_picture_screenshots` (ignored): `window-cellpicture-light`, `-dark`, and README31's page 12 | Frames in cells are not selectable; the text hit test passes over them to their cell; `page_editing::cell_frame` and `edit_command`'s T1 rule; Edit › Delete enabled in a picture's cell; `cell_outline` drawn in the accent colour with the caret in the cell; `cell_frame_note` in the status bar; a frame in a cell lays out no text of its own. The runtime harness can take screenshots (`Runtime::screenshot`) |
| CF.7 Twins | DONE | (this commit) | `cell_frame_twins` (ignored, `AMIPRO_VM_LAB`; `AMIPRO_TWIN_OUT` writes `ATF*.SAM` and `ARM31.SAM`): ATF2 and ATF7 the same as Ami Pro's TF2 and TF7; ATF3 and ATF10 the same but for row and picture heights and a default row record, which Ami Pro recomputes on opening (ATF3 saved again by Ami Pro is TF3); README31's table and pictures to the next page together | In the VM (`CELL_FRAME_NOTES.md`, "Twins"): every twin opens with its pictures in their cells as Ami Pro's own fixtures show them; ARM31's icons in their cells on page 13 |
| CF.8 Both formats and performance | DONE | `53a7430` | `cell_pictures_read_back_in_both_formats` (a TF1-shaped picture in a cell and its cell after Delete: native exact, SAM read back equal, nothing in the loss check, the copy laid out with the picture over its cell); `cell_frame_performance_baseline` (ignored, release) | README31 to the drawn frame: typing beside a picture 0.64 ms, Delete on a picture 1.9 ms (undo 1.9), a page break before its table 0.9 ms (undo 0.9). The round-trip facts describe a cell record as the writer writes it (values 8, 11, and 12 from the model), so a deleted picture's cell lists nothing |
| CF.9 Exit | DONE | (the handoff commit) | `tools/verify.sh --release --corpus` PASS at `53a7430` plus the handoff (core 311 + 4; app 248, 106, 255 with 25, 4, 25 ignored) | §24's implementation notes; `FEATURE_PARITY.md` TB-01 and IO-01; `UI_COMMAND_MATRIX.md` (Delete); `UI_ACCEPTANCE.md` "Frames in table cells" and its sign-off row; `FORMAT_COMPATIBILITY.md`; `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md` |

## The user's physical check

- **PASS (2026-10-18)** (`docs/Design/UI_ACCEPTANCE.md`, "Frames in table
  cells"): README31's page 12 with the pictures in their cells, a click
  putting the caret in the outlined cell with the status note, Delete and
  Ctrl+Z, and a page break taking the table and its pictures to page 13.
  The user: "that all works". No decision is open.

## Baseline (at `cells-run-start`)

Measured 2026-10-18 (`target/cf-baseline/`, outside Git), at `624b904` with
only documents changed; the VM folder now holds the run's fixtures (TF*,
BPS).

- Round trip (read back equal): user's corpus 417 of 417, ISO 159 of 160,
  VM folder 223 of 223; sections equal to the original 51,218 of 53,352,
  8,205 of 8,443, 22,865 of 22,871.
- Layout comparison: user's corpus page counts 303 of 394, paragraph line
  counts 14,880 of 15,356, page ends 761 of 1,108; ISO 65 of 160, 22,773
  of 24,971, 492 of 1,307; VM folder 222 of 222 page counts, 1,010 of
  1,021 line counts, page ends 89 of 89.

## Log

- 2026-10-18, CF.1: the decompile and the VM (TF1–TF15), the corpora's
  108 cell frames against the grid rule (90 exact; 10 never laid out; 6
  in rows on a later page; 2 off), the user's answers T1–T3.
- 2026-10-18, CF.2 (`target/cf-grid/`): round trip identical to the
  baseline on all three folders. Layout comparison: ISO and VM folder
  identical in their summaries; the user's corpus page counts 303 → 302
  and page ends 761 → 759 of 1,108, from three files: one gains three
  page ends (41 of 44), one loses one, and one loses four and gains a
  page (9 → 10 against Ami Pro's 9). In the last two every table's natural
  height is now exactly Ami Pro's stored height (it was 760 and up to 249
  off), so the old, too tall heights made the engine keep the stored
  height, which hid that our cell text measures taller than Ami Pro's
  rows (75 twips in the one table); the grid is right and the cell text
  measurement is a finding for later. Across the corpora a table's
  natural height on the new grid equals Ami Pro's stored one in 159 of
  389 user tables (58 before), 141 of 214 ISO (135), 20 of 22 VM (19), and
  never where the old rule did and the new does not. The anchored tables'
  size differences against Ami Pro's rectangles moved closer in most rows
  (e.g. 9,289 → 92, 472 → 0, 594 → 5).
- 2026-10-18, CF.3–CF.4 (`target/cf-model/`): read back equal and the
  loss check unchanged on all three folders (417/417, 159/160, 223/223).
  Sections equal to the original: user's 51,218 → 51,216 (the two files
  whose ten frames Ami Pro never sized, now written at their cells' 518
  twips, T2), VM folder 22,865 → 22,861 (the synthetic TF1YS, TF1YT,
  TF1ZS, written at their cells' size). Layout comparison: the cells'
  pictures no longer stand on the page as Wrap around frames at their
  stored places, so tables are no longer pushed beside their own pictures
  (eight small files' tables |dx| 6,163 or 3,355 or 4,046 → 0; one long
  file's 67 anchored frames |dy| 10,373 → 734 and a page end more, 42 of
  44; user's page ends 759 → 760). The pictures are drawn again from
  CF.5.
- 2026-10-18, CF.5 (`target/cf-layout/`): round trip and every row of
  the layout comparison identical to CF.3–CF.4. Frames in cells against
  Ami Pro's stored rectangles: user's corpus 94 compared, 77 on the
  stored page, 76 exactly where the stored rectangle puts them in their
  table; ISO 14, 11, 7; VM folder 31, 28, 21. The others: the ten frames
  Ami Pro never sized (page 0, no height; T2), rows Ami Pro continues on
  a later page (tables are not split yet), the synthetic TF1Y/TF1Z
  variants, and rows whose cell text measures taller here than Ami Pro's
  stored row height (READMEBP's long cells; the CF.2 finding).
- 2026-10-18, CF.8: typing, interleaved release A/B of
  `page_surface_typing_performance_baseline` against `cells-run-start`
  (three rounds, README31's start, middle, and end; both built in this
  tree, the base's `app/` and `crates/` checked out in turn, as the disk
  has no room for a second build): 0.673/0.847/0.742, 0.675/0.850/0.778,
  0.674/0.850/0.745 ms (base) and 0.771/0.869/0.775, 0.676/0.872/0.783,
  0.717/0.868/0.778 ms (now). The update phase is 0.02–0.04 ms slower in
  every round at the middle and end; the typing path gained only a look
  through the document's tables for a picture cell (a handful of cells in
  README31), so this is likely code placement, as the outline run found;
  far below a frame, left as is.
- 2026-10-18, final measures (`target/cf-final/`): identical to CF.5 on
  both corpora; the VM folder has Ami Pro's re-save of the ATF3 twin
  (ATF3S) besides, read back equal, its picture exactly in place (22 of
  32 there).
