# Tables Across Pages Progress

Live state of `docs/TABLE_PAGES_RUNBOOK.md`. Resume at the first slice that
is not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| TP.1 The map | DONE | (setup commit) | `docs/research/TABLE_PAGES_NOTES.md`; VM TP1–TP11 (with TP2B–TP2F, TP4B, TP5B); `UI_SPEC.md` §26; TP1–TP5 taken overnight | Row flags `0x10` heading and `0x20` page break, header bits `0x1` Automatic and `0x100` Rows span pages; `[pg]`'s first row and row count of the page-end table; `1428:046b`, `1468:0351`, `1468:044f`, `14e0:244f`, `11c8:381e` |
| TP.2 Model and native v3 | DONE | (this commit) | `table_row_options_validate_and_round_trip` (native `tablerows`, exact round trip, malformed lists and flags rejected, rows outside the table refused), `a_tables_heading_is_the_last_run_of_heading_rows_before_a_part` (TP2B, TP2F) | `Table::row_options` (`TableRows`: Automatic, Rows span pages, heading rows, page-break rows); `TableRows::heading_before` (`1468:044f`); native stage 12 `tablerows`, written only when not a new table's |
| TP.3 Import and the SAM writer | DONE | (this commit) | `tables_keep_automatic_rows_span_pages_headings_and_page_breaks` (read; written back as read; changed flags written as Ami Pro writes them, a new `[h]` record in Ami Pro's shape; read back equal); the round trip identical to the baseline on both corpora, the VM folder 257 of 257 (TP2F added) | Header value 7 bits 1 and `0x100`, `[h]` flags `0x10` and `0x20`; the round-trip facts include the row options |
| TP.4 Layout of rows | DONE | (this commit) | `rows_keep_their_size_without_automatic_tp2e`, `a_row_taller_than_a_page_is_cut_to_it_tp4`, `a_table_in_a_frame_shows_the_rows_its_frame_holds_tp8` (`crates/layout/tests/table_pages.rs`) | Without Automatic a row keeps its size and its text is cut; a row that does not fit an empty column is cut to it; a table that is not split (in a frame) shows the rows its frame holds |
| TP.5 Tables across columns and pages | DONE | (this commit) | 14 rule tests in `table_pages.rs` (TP1, TP3, TP5/TP4B, TP2/TP2B/TP2D, TP2F, TP6, TP9, TP10/TP11, TP7, Flow with text page tables, incremental equals fresh after cell, text, heading, and break edits); the layout crate's 51 rules and 5 incremental tests unchanged | `place_table`: parts of whole rows that fit with their gutter, the top inset in each, the heading and one more inset after the first, Rows span pages at lines (`row_lines`, `flow_lines`), page breaks on rows, joined rows kept together (`row_unit`); `FrameBoxKind::Table { part }` (`TablePart`); `table()` lays out a part's rows, a continued row from where the last part ended; a table on a line of its own (Ami Pro's page tables are Flow with text, `lone_table`) splits too; the table's empty line stays where it began; a cell edit never reuses a page holding that table (`Convergence::tables`) |
| TP.6 Cell text measurement | DONE (in part) | (this commit) | The comparison's table detail (`AMIPRO_COMPARE_DETAIL_TABLES`, `*` for every file); measured in `target/tp-empty/` | An empty cell needs no height (Ami Pro keeps a row of empty cells at its stored height, even 0). Found: the rows still taller than Ami Pro's are multi-line cells in faces no one-line main-text paragraph calibrates (Times New Roman 11 pt, Book Antiqua 12 pt), 5–7 twips a line taller with the substitute fonts; Ami Pro writes no paragraph caches in cells. Calibrating faces from one-line rows was tried and gave nothing (reverted); trusting Ami Pro's stored row heights while a row's cells are as read is parked for the user (morning report) |
| TP.7 The page surface | DONE | (this commit) | `clicks_in_a_continued_table_land_in_its_cells_on_their_page` (widget path: a press on page 2's part and on its repeated heading puts the caret in that cell on page 2, the caret drawn on that copy; cells of other parts have no box), `a_page_break_in_a_table_cell_starts_its_row_on_the_next_page` (messages: Insert › Page Break in a cell sets and clears the row's break, one undo step, checked there; the main text as before); `table_pages_screenshots` (ignored): `window-tablepages-light`, `-dark`, `-split-light` | A part's cells drawn, those of other parts skipped, no grid line at a split row's edge (`cell_edges`); hits skip empty cell boxes; the ruler finds the part showing the caret's cell; cell lines are not page copies for the selection; `Command::SetTableRows` (one undo step) and `page_editing::row_break_command`, `row_has_break`, `table_row_of`; `long_table_fixture`. The screenshot harness stayed on the last page whatever the caret or Next Page (noted, not chased) |
| TP.8 The comparison | DONE | (this commit) | `layout_comparison_report`'s `tables:` summaries (page-end tables against Ami Pro's page table; rows against `[h]`); `AMIPRO_COMPARE_DETAIL_TABLES`; `oracle.rs` `PageEnd::table_rows`, `FrameCache::table_rows`, `automatic` (`reads_the_table_rows_of_a_page_end`); `LAYOUT_COMPARISON.md`, "Tables across pages" | VM folder 41 of 42 page-end tables (BPS: one extra wrapped line in a cell); user's 69 of 106 and ISO 18 of 106, the differences upstream of the split (line breaking and line heights in cells, text before the table, missing ISO style sheets) |
| TP.9 Twins | DONE | (this commit) | `table_page_twins` (ignored, `AMIPRO_VM_LAB`; `AMIPRO_TWIN_OUT` writes `ATP2`, `ATP5`, `ATP6`): each pages as Ami Pro's own fixture and writes its flags | In the VM (`TABLE_PAGES_NOTES.md`, "Twins"): Ami Pro opens the three and pages them as TP2, TP5, and TP6 |
| TP.10 Both formats and performance | DONE | (this commit) | `table_row_options_read_back_in_both_formats` (native exact; Save As SAM read back with the same options and parts; no table or frame loss); `table_pages_performance_baseline` (ignored, release); `tools/bench-ab.sh table-pages-run-start 3`: verdict no regression | Release, to the drawn frame: typing in a continued table 1.08 ms median (fixture), 1.27 ms (README31); a page break in a row 1.04 / 2.81 ms, its undo 1.00 / 2.78 ms |
| TP.11 Exit | DONE | (the exit commit) | `tools/verify.sh --release --corpus` PASS (core 348 + 4; app 274, 108, 281 with 31, 4, 31 ignored; ISO corpus gate PASS); every VM twin test passes (layout, ruler, frame, bullet, outline, colour, cell frame, picture, table page) | §26's implementation notes; `FEATURE_PARITY.md` TB-01; `UI_ACCEPTANCE.md` "Tables across pages" and its sign-off row; `UI_COMMAND_MATRIX.md` (Page Break); `FORMAT_COMPATIBILITY.md`; `LAYOUT_COMPARISON.md`; `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`; the playbook's queue; the morning report in `docs/SESSION_HANDOFF.md` |

## Baseline (at `table-pages-run-start`)

Measured 2026-10-18 (`target/tp-baseline/`, outside Git) at `6d10613`
(the setup commit changes documents only); the VM folder now holds the
run's fixtures, 256 files.

- Round trip (read back equal; Save As lists nothing): user's corpus 417
  of 417, ISO 159 of 160, VM folder 256 of 256; sections equal to the
  original 51,216 of 53,352, 8,205 of 8,443, 26,483 of 26,497.
- Layout comparison: user's corpus page counts 302 of 394, paragraph line
  counts 14,880 of 15,356, page ends 760 of 1,108; ISO 65 of 160, 22,773
  of 24,971, 492 of 1,307; VM folder 247 of 255 page counts, 1,481 of
  1,492 line counts, page ends 99 of 113 (the misses are the run's own
  fixtures, whose tables do not continue yet).
- Frames in table cells against Ami Pro's stored rectangles: user's 94
  compared, 77 on the stored page, 76 in place; ISO 14, 11, 7; VM folder
  34, 31, 23.

## Log

- 2026-10-18, TP.1: the user's guide ch. 16 (Rows span pages, Headings,
  page breaks in a table); the decompile (the `[h]` reader `1428:046b`,
  the heading helpers `1468:0351`, `044f`, `05a8`, the caret past a
  repeated heading `14e0:244f`, the page table writer `11c8:381e`); the
  corpora's page tables (214 records end in a table; 48 continue the
  previous page's table at its next row, one at the same row); VM fixtures
  TP1–TP11 written as synthetic files and saved again by Ami Pro, TP2, TP5,
  TP6, TP7, TP8 changed in Ami Pro. Decisions TP1–TP5 taken overnight by
  the rulebook, to confirm.
- 2026-10-18, TP.1 (later): TP2F (headings 0 and 11, page 2 starting at
  row 11) repeats row 0 above row 11, so the heading is the last run
  strictly before a part's first row.
- 2026-10-18, TP.2–TP.3 (`target/tp-model/`): round trip identical to the
  baseline (user's 417 of 417, ISO 159 of 160, sections unchanged); the
  VM folder 257 of 257 with TP2F. No layout change yet.
- 2026-10-18, TP.4–TP.5 (`target/tp-split2/`; the first measure,
  `target/tp-split/`, split only With para above tables, and the corpus's
  page tables are Flow with text frames, so it changed little). Round trip
  identical. Layout comparison, with the new table measures (TP.8's first
  part: each page-end table's first row and row count against Ami Pro's
  page table, and rows of Automatic tables laid out whole against their
  stored heights):
  - VM folder: page counts 247 → 256 of 256, page ends 99 of 113 → 105 of
    114 (TP2F added); every table fixture's page count and table page ends
    now Ami Pro's (41 of 42 page ends in a table; rows 534 of 535).
  - User's corpus: page counts 302 → 311 of 394, page ends 760 → 775 of
    1,108 (33 gained in 14 files, one file 16; 20 lost in 8 files, whose
    rows measure 70–90 twips taller than Ami Pro's on average: TP.6);
    page ends in a table with Ami Pro's rows 68 of 106; rows at Ami Pro's
    stored height 2,488 of 2,777.
  - ISO corpus: page counts 65 → 68 of 160, page ends 492 → 474 of 1,307
    (9 gained, 28 lost). Every file that lost uses a style sheet the ISO
    does not have (smartext.sty, pro21.sty): its cells' styles are
    fallbacks, so their rows measure 20, 40, or 120 twips taller per row
    (a guessed paragraph spacing), where before the table kept Ami Pro's
    stored height. Rows at Ami Pro's height 666 of 2,056 there; table page
    ends 18 of 106.
  - Over the three folders page ends 1,351 → 1,354 and page counts 614 →
    635: merged, the ISO loss explained by its missing sheets.
- 2026-10-19, TP.6 (`target/tp-diag/rows-user.txt`, `target/tp-calib/`,
  `target/tp-empty/`): of the user's 289 rows not at Ami Pro's height, 223
  are taller by more than 15 twips, 51 within 15, 15 rows of empty cells
  that Ami Pro keeps at 0, and 2 split rows. The taller ones are multi-line
  cells in faces without a calibrated printer height (Table Text in Times
  New Roman 11 pt, Book Antiqua 12 pt, and the like: our substitutes' lines
  are 5–7 twips taller), and a few rows whose stored height is stale (a
  10-paragraph cell stored at 255). Calibrating such faces from one-line
  rows (rows at the table's default height left out) changed nothing
  measurable (user's page ends 775 → 774, rows 2,488 → 2,483): reverted.
  Empty cells needing no height: user's rows at Ami Pro's height 2,488 →
  2,597 of 2,777, page ends 775 → 777, page counts 311 → 312, page ends in
  a table with Ami Pro's rows 68 → 69; the ISO corpus and the VM folder
  unchanged; round trip unchanged.
- 2026-10-19, TP.10: the interleaved A/B of both editors' typing
  baselines against `table-pages-run-start` (three rounds, README31;
  `target/bench-ab/20261019-001908/`): no operation slower beyond max(10%,
  0.05 ms) in a majority of rounds; verdict no regression.

## Fill-in work (the rulebook's step 2, 2026-10-19)

- **With para above frames beside Wrap around frames** (FC7, FC7E, FC7G;
  `FRAME_CREATION_NOTES.md`): the frame takes the free part of the column
  that holds its left edge, narrowed so that it and its right shadow end
  at the Wrap around frame; beside a No wrap around frame it keeps its
  place and width (`Engine::free_parts`;
  `with_para_above_frames_narrow_beside_wrap_around_frames_fc7`).
  Measured (`target/tp-fc7/`): both corpora identical row for row; the VM
  folder's FC7, FC7B, FC7C, FC7E now at Ami Pro's place and size (|dx|
  720 → 0, size 820 and 1,644 → 0, text-frame columns 1,538 and 2,339 →
  0); round trip unchanged (the VM folder adds tonight's fixtures, 266 of
  266 read back equal).

- **Column balance with unequal columns** (`RULER_PROGRESS.md`): balance
  moves finished lines, so a line moved into a column of another width
  keeps its width. Both corpora hold 21 multi-column layouts, 6 of them
  balanced with uneven columns, all within 90 twips of even. Re-breaking
  needs a second layout pass of each balanced page, a large change for
  little effect: parked, not built.
- **Text on both sides of a Wrap around frame** (`PHASE4_PROGRESS.md`'s
  finding): VM WB1–WB3 (synthetic, a frame in mid-column; parts of 2,160
  and 4,320, 720 and 1,440, 360 and 900 twips) and page 4 of one of the
  user's manuals show Ami Pro's rule: the text fills the part left of the
  frame down the frame's band (to its bottom shadow), then the part right
  of it (past its right shadow) from the band's top, then goes on across
  the column below; even a 360-twip part takes text, a word too wide for
  it split ("bes|ide", "ma|ke"). Built (strips on the cursor, a rule test
  from WB1) and measured (`target/tp-wb/`): VM folder unchanged; user's
  corpus page counts 312 → 315 (three files to Ami Pro's count), one
  file's page ends 5 → 10 of 10, but one long manual already 16 pages
  longer than Ami Pro's 34 → 14 of 51, so page ends 777 → 762 overall.
  Kept off master by the rulebook on branch `overnight-wip-strips`
  (`0961574`); **merged on 2026-10-19 at the user's word** (`f6ee32f`;
  master had changed only records since, so the branch's measures stand).
  Not built: splitting a word too wide for a narrow part.
- Found: a document whose caches hold no one-line paragraph keeps the
  font's own line height (WB1S: 276 twips against Ami Pro's 284, from
  six-line caches of 1,704); calibrating from multi-line caches whose
  line count is clear would close it (for later).

## The row cache (2026-10-20, user decision 2026-10-19)

- **VM RC1–RC3** (`TABLE_PAGES_NOTES.md`, "The row cache"): Ami Pro
  measures every Automatic row when it opens a file, growing a stored
  height too small for the text (RC1), shrinking one too large (RC2), and
  giving empty cells one line (RC3); so a stored height is Ami Pro's
  measure, a cache.
- **Built:** `Table::read_rows` (each row's fingerprint at import, native v3
  `readrow`), `Document::table_row_fingerprint`; the engine keeps a row's
  stored height while it is as read and its text measures here at most
  half its gutter taller, the text running into the gutter
  (`a_row_as_read_keeps_ami_pros_stored_height`). The comparison counts the
  rows equal only through the cache.
- **Measured** (`target/rc-baseline/` → `target/rc-after/`): user's corpus
  rows at Ami Pro's height 2,597 → 2,639 of 2,777 (42 by the cache), page
  counts 315 and page ends 762 of 1,108 unchanged, table page ends 69 of
  106 unchanged; ISO corpus rows 666 → 1,225 of 2,056 (559 by the cache),
  page counts 68 → 69, page ends 474 → 478, table page ends 18 → 19; VM
  folder unchanged (271 of 271 pages, 111 of 121 page ends); round trip
  unchanged (417 of 417, 159 of 160; the VM folder 272 of 272 with RC1A–RC3A).
  Merged: nothing worse anywhere.
- **What the cache cannot reach:** of the user's 140 rows still not at Ami
  Pro's height, 38 measure 44–86 twips taller (beyond half the gutter, so
  their text would cross the line under the row), 19 measure 87–200, and
  81 more than 200: a line more here (line breaks, not line heights) or a
  stale height; one is shorter and one within 43 is split.
- **For the table editing run:** an edited Automatic row should fit its
  text, shrinking too (RC2), and an empty cell take one line (RC3).
