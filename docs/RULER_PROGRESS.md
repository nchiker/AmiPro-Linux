# Ruler Progress

Live state of `docs/RULER_RUNBOOK.md`. Resume at the first slice that is
not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| RU.1 The map | DONE | `2ab6a26`, (this commit) | `docs/research/RULER_NOTES.md`; VM RUB, RU1–RU12; `tools/ghidra/command_table.py`, `DecompAt.java` | `AMIPRO.EXE`'s `0x32a` handler `1148:06e5` found through the command table (Ghidra had missed ~320 table-only handlers); every open question settled in the decompile |
| RU.2 Paragraph tabs and indents | DONE | (this commit) | `rulers.rs` (5 tests); the random undo/redo test covers both commands; `rulers_and_indents_set_by_commands_write_as_ami_pro_writes_them` (SAM after edits); `a_paragraph_after_a_ruler_paragraph_ends_the_ruler_ru3`; round-trip audit output identical before and after on all three folders | `SetParagraphTabs`, `SetParagraphIndents` (`IndentChange`); `ParagraphStyle::tabs_in_effect` (engine and ruler); Normal keeps the ruler; the writer derives `<:R>`, the importer drops the derived marks (re-fingerprinting caches that held) and leaves them out of the unmapped-controls count |
| RU.3 Unequal columns | DONE | (this commit) | Model, native (`edges` record, strictness), importer and both writers (RU2's edges as model data, near-even ones as offsets), engine (`uneven_columns_lay_out_at_their_edges_ru2`), `layout_setup` (margins move the outer edge or make alike gaps even, count and gutter make them even, Mirror reverses, paper keeps them), Page Layout Properties and Frame Properties (gutter only while gaps are alike, Even Columns); round-trip audit identical; layout comparison below | Ami Pro's caches confirm unequal widths (4200 and 4290 in one user layout); column balance moves lines without re-breaking them (finding) |
| RU.4 The ruler's model | DONE | (this commit) | `app/cosmic/src/ruler.rs` (7 tests: RU1–RU5, RU8–RU11, selections, menus); `layout_setup::set_box_edges`, `ruler_columns`, `set_page_edges`, `set_band_edges`, `set_ruler_columns` (RU2/RU9/RU10/RU12 test) | The caret's box (page, band, frame, or tabs-and-indents only in a cell or floating header), every column, tabs in effect, indents; hit areas; snap by truncation to 90 twips; Ami Pro's limits; one command per gesture |
| RU.5 The interactive ruler | DONE | (this commit) | `ruler_view.rs`; window tests `the_ruler_sets_tabs_and_indents_through_the_window_one_undo_step_each` (RU1, RU3–RU5, the kind button, focus kept, undo, View › Ruler) and `the_rulers_lower_half_moves_columns_with_a_guide_down_the_page_ru2`; View › Ruler saved (preferences test); screenshots `window-ruler-light`, `-dark`, `-guide-light` | 36 px: tabs and indents above, margins and columns below; hover names and positions in the status bar; the guide line on the page; release → one command; a press on the ruler leaves the caret's focus |
| RU.6 Right-click menus on the ruler | DONE | (this commit) | `the_rulers_right_click_menus_act_on_what_was_pressed` (a tab's kind and Remove, Clear Tabs, the kind button's menu, Page Layout… from the lower half, through the window) | One menu around the ruler (outside the scrollable) built for what the right press hit: a tab (kind, leader, Remove), the upper half (Clear Tabs, Remove Paragraph Ruler, new tabs' kind and leader), the kind button, the lower half (Columns ▸ 1–8, Even Columns, Page Layout… or Frame Properties…) |
| RU.7 Frames, headers and footers, table cells | DONE | (this commit) | `in_a_frame_the_ruler_edits_its_interior_and_its_texts_tabs`, `in_a_header_the_ruler_edits_the_band_through_the_page_layout`, `in_a_table_cell_the_ruler_has_tabs_and_indents_only` | A frame's interior (inside its corners' inset) through `SetFrameStyle`; a header or footer band through `ModifyLayout` (Mirror kept); a cell or a floating header: tabs and indents only (Ami Pro's ruler also drags table column widths: not built) |
| RU.8 Keyboard | DONE | (this commit) | `edit_go_to_ruler_gives_the_ruler_the_keyboard_as_ami_pro_does` (through the window) | Edit › Go To Ruler; Left/Right move the ruler's line a 1/16 in, Ctrl half an inch; Space adds a tab, Del removes the one at the line, 0–3 set All, First, Rest, Right, `[` `]` the caret column's edges; Esc or a click in the text returns. Keys from the user's guide, ch. 8 (VERIFIED (manual)); the ruler captures them, so neither the text nor shortcuts see them. Ami Pro's Space+arrow tab move is left out (drag instead) |
| RU.9 Twins | DONE | (this commit) | `ruler::tests::ruler_twins` (ignored; `AMIPRO_VM_LAB`): 12 of 12 bodies and layout records equal to Ami Pro's; Ami Pro shows each twin as its own file (22 of 22 screens identical in the document area) | `AMIPRO_TWIN_OUT` writes `ARUn.SAM` for the VM |
| RU.10 Both formats and performance | DONE | (this commit) | `uneven_columns_made_on_the_ruler_read_back_in_both_formats` and `rulers_and_indents_set_by_commands_write_as_ami_pro_writes_them` (SAM after edits: read back equal, nothing in the loss check, native v3 exact); round-trip audit identical to the baseline on all three folders; layout comparison as after RU.3 (no change since moves text); `ruler_release_performance_baseline`; typing A/B | Timings below |
| RU.11 Exit | DONE | (this commit) | `UI_SPEC.md` §19 implementation notes; `FEATURE_PARITY.md` RU-01, PG-01; `UI_COMMAND_MATRIX.md`; `DECISIONS.md`; `PROJECT_PLAN.md`; `PROJECT_STATUS.md`; `CLAUDE.md`; handoff | Physical check PASS (2026-10-17), after two fixes; below |

## The map (from RU.1)

What each slice builds, by `RULER_NOTES.md`:

- **RU.2.** `SetParagraphTabs { flow, first, last, tabs: Option<Vec<TabStop>> }`
  (the same list for every paragraph, as `1170:052d`; `None` removes the
  own ruler) and `SetParagraphIndent { flow, first, last, which, value }`
  (one of All, First, Rest, Right; each paragraph keeps its other three,
  as `12e8:056d`). One function for the tabs in effect (own ruler, else
  the style's own tabs, else the box's list) shared by the engine and the
  ruler, at most 22 stops. Text › Normal keeps the own ruler (`12e0:132b`;
  the app cleared it). The SAM writer derives `<:R>` after a ruler
  paragraph (`11b0:1405`) and the importer drops the marks it derives.
- **RU.3.** `LayoutBox::column_edges: Option<Vec<ColumnEdge>>` (left and
  right from the box's text-area left; `None` while widths and gaps are
  alike within a twip); the engine lays them out; native v3 record;
  importer and both writers (kept `.edges` offsets stay for near-even
  columns); `layout_setup`: a margin change makes alike-gap columns even
  and otherwise moves only the outer edge (`1238:02d0`); a count or gutter
  change makes them even; Even Columns in Page Layout Properties and Frame
  Properties.
- **RU.4.** `ruler.rs` (app, pure): the caret's box, its columns, the
  caret's column, tabs in effect, indents; hit areas (tab marker to 29
  twips past it; indent markers; edges, bodies, gaps in the bottom half);
  drags snapped by truncation to 90 twips of the page's x, clamped to
  Ami Pro's limits; a click adds an unsnapped tab between the indents;
  release turns into one command (column gestures: one `ModifyLayout`
  with the new edges; Cols: even with gutter 240).
- **RU.5–RU.8** as the runbook says; in a table cell, tabs and indents
  only (Ami Pro's ruler also drags table column widths there: noted, not
  built).

## Baseline (at `ruler-run-start`)

Measured at `ruler-run-start` (`1190157`), 2026-10-13:

- Round trip (`sam_round_trip`, read back equal): user's corpus 417 of
  417, ISO 159 of 160 (the malformed layout), VM folder 170 of 170.
- Layout comparison (`layout_comparison_report`): user's corpus page
  counts 302 of 394, paragraph line counts 14,880 of 15,356, page ends
  762 of 1,108; VM page counts 170 of 170, lines 945 of 956, page ends
  84 of 84; ISO page counts 65 of 160, lines 22,773 of 24,971, page ends
  492 of 1,307.

## Log

- 2026-10-13, RU.1 (session ended mid-slice): the ruler control is
  `AMIPROUI.DLL`'s `TABRACKPROC` and segment `10d0` (hit test, drag,
  snapping, release), with the tool row `TABBOXPROC` (dialog 2144);
  each gesture reaches `AMIPRO.EXE` as command `0x32a`, whose handler is
  not found yet. Fixtures RU3–RU11 in the VM agree with the DLL's rules
  (`RULER_NOTES.md`). The user was also using the VM during one attempt
  (a stray Alt+Tab); that attempt was discarded unsaved and every saved
  fixture was checked. Resume: the open questions at the end of
  `RULER_NOTES.md`, starting with reopening RU9 and setting Cols 3.
- 2026-10-17, RU.1 (resumed; the user's prompt was the page layout run's,
  which had finished, so the session carried on with this run): RU12
  (made at the end of the last session, unrecorded) answers Cols. The
  `0x32a` handler was found through `AMIPRO.EXE`'s menu-command table
  (DGROUP 0x46), whose ~320 table-only handlers Ghidra had not found;
  decompiled with a new script. Settled in the decompile: one tab list for
  every selected paragraph, indents per paragraph, edges and margins, Cols
  (always 240), Normal keeps the ruler, Ruler Insert and Remove, and the
  writer's `<:R>`. Measured the corpora's unequal columns. RU.1 DONE.
- 2026-10-17, RU.2: the first audit after deriving `<:R>` showed four
  paragraph caches lost (dropping a mark after the cache fingerprint was
  taken) and three of the user's files with a new warning (their rulers,
  two in a syntax 3.1 does not write, were not followed by `<:R>`, so
  another writer made them; the warning counted marks the importer
  drops). Both fixed; the audit's output is now identical to the
  baseline's (user's corpus, ISO, VM folder). Layout cannot move (the
  engine reads the same stops), so no comparison run. RU.2 DONE.
- 2026-10-17, RU.3: layout comparison before and after (all with
  `AMIPRO_STYLES`): VM folder 181/181 page counts, 981/992 lines, 84/84
  page ends, unchanged; ISO 65/160, 22,773/24,971, 492/1,307, unchanged;
  user's corpus page counts 302 → 303 of 394, lines 14,880 of 15,356
  unchanged, page ends 762 → 761 of 1,108. Both changed rows are the same
  two-column layout (4,200 and 4,290 twips, even would be 4,245), now laid
  out as Ami Pro's caches show it: one document now has Ami Pro's page
  count; in the other a page end moved by one line, because two empty
  paragraphs are 284 twips tall in our layout and 144 in Ami Pro's (an
  older difference that the even columns had cancelled). Round-trip audit
  identical on all three folders. Found: column balance moves finished
  lines between columns without re-breaking them, so a line moved into a
  column of another width keeps its width (only with uneven columns and
  Column balance; for later). Ami Pro's dialog treats two columns' one gap
  as alike, so a margin change in the panel makes two uneven columns even
  (only the ruler moves one edge, RU10). RU.3 DONE.
- 2026-10-17, RU.4–RU.9: the ruler's model, widget, menus, frames and
  bands and cells, keyboard, and twins (rows above). In RU.5 a press on
  the ruler took the keyboard focus from the text (the page surface
  dropped its focus on any press outside it); it now keeps it when
  another widget took the press. RU.9: 12 of 12 twins equal; in the VM
  Ami Pro shows each twin as its own (22 of 22 screens).
- 2026-10-17, RU.10 timings (release, 1280 × 800, from the message to the
  next frame drawn): on README31 (344 paragraphs, 14 pages) an indent
  release 0.63 ms median (0.66 p95), a left-margin release 41 ms (44 ms,
  every page laid out again), undo 42 ms; on the user's largest document
  (798 paragraphs, 68 pages) 0.47 ms and 87 ms (94 ms), undo 89 ms (PL.11
  measured 123 ms for a layout change there). Typing (page surface
  baseline, README31, interleaved A/B against the run's start, three
  rounds, median): at the start 0.593–0.596 ms before, 0.622–0.627 after
  (+0.03 ms, the ruler's widget layout); in the middle 0.78–0.87 against
  0.80–0.81; at the end 0.69–0.70 against 0.69–0.70. Final checks:
  `tools/verify.sh --release --corpus` PASS (core 268 + 4 ignored; app
  201, 106, 208 with 13, 4, 13 ignored; ISO corpus gate); round-trip
  audit identical to the baseline (417/417, 159/160, 181/181).

## Exit (2026-10-17)

Met (`docs/RULER_RUNBOOK.md` §6): the ruler shows and edits tabs, indents,
margins, and columns as §19 describes, by the rules RU.1 mapped; each
gesture is one undo step; documents edited on the ruler round-trip through
both formats with nothing in the loss check; and Ami Pro in the VM shows
the app's twins as its own. The user's physical check follows.

## The physical check

- 2026-10-17, first report: after column gestures on the ruler, File ›
  Save As an Ami Pro document listed "1 page layout". Cause: Ami Pro
  stores no gutter; it works it out from the columns (`1238:0000`: 240 for
  one column, else the first gap, and 0 when the gaps differ, which a
  column count then takes as 240, `1238:008a`), so a file reads back with
  the first gap as the gutter, while a ruler drag left the model's gutter
  at its old value. A random-gesture test (ruler, Page Layout Properties,
  header, frame; 400 runs of 8 gestures) found it in 87 runs, and two
  more causes: one column after a wide gutter (since the page layout run:
  the file gives back 240) and edges whose widths are a twip apart (the
  importer takes them as even; `set_edges` kept them uneven). Fixed:
  `LayoutBox::settle_gutter` (one column 240, uneven columns their first
  gap), called by `set_edges` and Mirror; `set_edges` takes edges alike
  within a twip across the text area as even, as the importer does; a
  column count and Even Columns take Ami Pro's gutter (`count_gutter`: the
  shared gap, else 240); Frame Properties refuses and hides the gutter for
  one column, as Page Layout Properties does; the loss check settles the
  gutter it compares, so files saved before the fix list nothing for it.
  Evidence: `column_gestures_leave_nothing_for_save_as_to_list` (60 runs
  of 6 gestures; fails without the fix), `the_gutter_is_what_ami_pro_works_out_from_the_columns`,
  the document and Frame Properties tests; 0 of 800 random runs list
  anything; ruler twins 12 of 12 and layout twins unchanged; round-trip
  audit identical to the baseline on all three folders; layout comparison
  unchanged (user's corpus 303 of 394, 761 of 1,108; ISO 65 of 160, 492
  of 1,307; VM folder 181 of 181), since imported documents already kept
  the rule.

- 2026-10-17, second report: a tab's right-click menu no longer changed
  its kind. Cause: the first-line indent, dragged onto the stop at 0.5 in
  in step 3, stood on the tab, and the hit test took indent markers
  before tabs, so the right-click opened the ruler's general menu (its
  New Tabs ▸ kinds set only the next tabs' kind). Ami Pro tries tabs
  first (`10d0:090c`). Fixed: a right-click in the upper half takes the
  nearest tab (`Ruler::tab_near`); a left press takes the tab first
  except on the first line's triangle and the bar. Evidence:
  `a_tab_under_an_indent_marker_keeps_its_menu_and_its_drag` (window:
  the menu, Centre, a drag from the foot moves the tab, from the
  triangle the indent; fails without the fix) and the hit order in
  `indent_drags_set_one_value_ru5_to_ru8`. Before the fix the menu
  worked in seven other states tried (zooms, two columns, a second
  change, a new tab, a reopened SAM).

- 2026-10-17: the user's rechecks of both passed ("both test passed"):
  Save As lists nothing after column drags, and a tab's right-click menu
  changes its kind.

- 2026-10-17: steps 6 and 7 passed ("step 6 and 7 passed"): Edit › Go
  To Ruler's keys, the ruler in a text frame and a header, and View ›
  Ruler. **The ruler's physical check: PASS.**

## Waiting for the user

Nothing. The run and its physical check are done; next is creating frames,
then frames in table cells (`PROJECT_PLAN.md`).

