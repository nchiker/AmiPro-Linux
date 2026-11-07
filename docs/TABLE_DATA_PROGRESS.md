# Table Data Progress (run C)

The live state of `docs/TABLE_DATA_RUNBOOK.md` (tag `table-data-start`).
Resume at the first slice that is not `DONE`. Decisions TN1–TN11 answered 2026-11-06 (recommended options);
TD.1 done (decompile pass, baseline, `UI_SPEC.md` section 41 draft); nothing is built.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| TD.1 Setup | DONE (tag and commit left to the caller) | | `TABLE_DATA_NOTES.md` section 10 (decompile pass, no VM); `UI_SPEC.md` section 41 (draft); baseline in `target/table-data-baseline/start/` (not Git) | Open for TD.2: TDV2 (decoration drawing per format, one file, four styles, screenshots), TDV3 (narrow list, 10.9), TDV5 (one file), TDV6, TDV7, TDV8 (one file). TDV1 and TDV4 dropped. |
| TD.2 VM fixtures | DONE (2026-11-06; one sitting, VM shut down) | | `TABLE_DATA_NOTES.md` section 11 (TDV2, TDV3, TDV5, TDV6, TDV7, TDV8 answered; files and screenshots in `../AmiPro-Reference/vm-lab/OUT/tabledata/`, not Git) | Not answered: Insert/Delete Column (refused: no room), `%` on a bracket, tie order of the heap sort (needs the heap sort's own code), numeric tie order of keys without digits |
| TD.3 Model and SAM | DONE (core; not committed) | | `number.rs` (`NumberFormat`, `scan_number`, `stored_text`), `TableCell::formula`, native `cellformula`, SAM second body (`frames.rs` `split_formula`, `prefix_of`), `LossKind::FormulaFunctions`; `crates/ami-formats/tests/table_formulas.rs` (9 tests); round-trip audit equal to the start (user's and ISO identical; VM folder 465 read back equal, the same 39 files unequal in bodies as at the start; the folder has grown by 12 files) | Updated with TD.2: `(-` quirk, 0x2000 and empty-result forms, row-rewrite tests (notes section 12). Open: `app/cosmic` struct literals of `TableCell` need `formula: None` (page_surface.rs, three sites); the app does not yet use the number format or formula. An untagged second body starting `@` is written `@@` (notes section 11). |
| TD.4 Formula engine | DONE (core; not committed) | | `crates/document/src/formula.rs` (20 tests, 4 in `number.rs`): compile, evaluate, `calculate`, `dependants`, `rewrite`; audit `crates/ami-formats/tests/formula_corpus.rs` (ignored; `TEST_COMMANDS.md`): user's 261 of 261, ISO 15 of 15 recomputed equal to the stored text in all three modes | Audit after TD.2 (notes section 12): user's 261 and ISO 15 equal; VM folder 96 cells, every difference explained (stale REF-word results 6, hand-edited 4, loop first cell 2). Unconfirmed: Insert/Delete Column rewrite, `$` on insert/delete, a loop's reader. |
| TD.5 Number formats | DONE (not committed) | | `TABLE_DATA_NOTES.md` section 13; `number.rs` `display` (+8 tests); layout `shaped_number`; surface plain-in-caret-cell; `rewrite_number`; Styles rail Table format; scenario `table_data.rs` (2); layout comparison `target/table-data-baseline/td5/` equal to `start/` for the user's and ISO corpora (page counts, lines, page ends, table rows all identical; round trip identical); VM folder grew 460 to 505 files (new TD fixtures), its summary figures moved only through those files | TNK-TD5a..e (notes 13). Existing table scenarios now expect `10.00` for typed `10` (Ami Pro's rewrite). Open: Save/print do not rewrite a cell still holding the caret; REF drawing (TD.7). |
| TD.6 Commands and recalculation | DONE (core; not committed) | | `Command::SetCellFormula`, `QuickAdd`, `RecalculateTable` (`tables.rs`: `set_cell_formula`, `quick_add_text`, `recalculate_table`; refusals `FORMULA_REFUSED` 3125, `FORMULA_IN_SORT` 3113, `NOTHING_TO_ADD`, `NO_FORMULA`, `NOTHING_RECALCULATED`); every content-changing `TableEdit` (insert/delete rows and columns, ClearCells, Move, Sort, Paste) rewrites names and recalculates inside its step; `CellBlock::origin`/`CellData::formula` for paste shifting; `crates/editing/tests/formulas.rs` (15) and a `sam_after_edits` test; writer sets F3 0x10 on an empty formula result (0x50, TDF1) | TNK-TD6a..e below. The app needs TD.7: call `Command::RecalculateTable{frame, changed:[(row,col)]}` when the caret leaves an edited cell (amended into the typing run, as `rewrite_number`), show the refusals' messages, Del already goes through `ClearCells`. |
| TD.7 Interface | BUILT (not committed; core 628 + layout; COSMIC 595 default, 128 text editor, 602 all features, all green) | | `table_formula.rs`; Table menu (Edit Formula…, Quick Add ▸, Recalculate Table), the Formula section of Table Properties, `recalculate_cell` in `settle_numbers`, typing refusal, REF drawn (`Engine::note_refs`), cell box, headings, `Cell B3` in the status bar; scenarios `table_data.rs` (invoice, stored file, Quick Add, errors, rewriting, formula before data, frame and page tables, gestures); `UI_SPEC.md` §41 updated | TNK-TD7a..d below. Open: Paste/Cut into a formula cell are not refused. |
| TD.8 Sort | BUILT (not committed) | | `crates/editing/src/sort.rs` (`SortSettings`, Ami Pro's heap sort `1360:23de/17f6/259c` reproduced: Ascending is a min-heap written back last to first, Descending the max-heap likewise; ties match the VM's TDY1 vector exactly), `Command::SortParagraphs`, `TableEdit::SortRecords` (records of rows, whole table below the headings or the selected rows; 3113, 3184 ask, 3193), tests in `sort/tests.rs` and `tests/sort.rs`; app `sort_panel.rs`, `sort_commands.rs`, Tools › Sort…; scenario 9 in `scenario_tests/table_data.rs` | Not built: 3186 (DDE escape; no model), 3215 (too large). Descending's tie order is inferred from the decompile (the VM saw no tie case); the notes' "Descending reverses ties" is not what the heap gives. TDX5's paragraph vector has no ties (the whole text is the key), the tie evidence is TDY1 |
| TD.9 Merge numbers | BUILT (not committed) | | `merge_number_text` and `DataTable::with_merged_numbers` (`crates/editing/src/merge/data.rs`, unit test of every TDV7 case); `MergeData::for_merge` (app `merge.rs`) used by the preview, Merge to document and every merge output for a TABLE data file only; delimited, DIF, dBASE, XLSX and the like merge as read (AMT3: a delimited file's text as typed); scenario 10 in `table_data.rs` (AMD1's `150`, `75.50`, `$1,200.00`, `007`: preview and Merge to New Document, the data file's cells unchanged) | TNK-TD9a the rule is applied to every value of a table data file, not only to a field named Amount; TNK-TD9b `5.` (ends in a bare point) is accepted as the notes list it (`5`), though the task text says the field must end in a digit; conditions and Sort in the data editor still see the stored text. Not run against the VM's AMT3 file (no VM); the rule is the VM-measured TDV7 list, unit-tested case by case. The ignored `merge_twins` letter has no Amount field, so AMT3 itself is covered by scenario 10
| TD.10 Forms | BUILT (not committed) | | Scenario 11 in `table_data.rs` (`the_form_recipe_protects_labels_and_prints_on_a_preprinted_form_td10`): grid lines, label cells protected, Protected Text on the labels, Honor protection (Table Properties), Tab and Shift+Tab skip the labels, a blank takes a number, print on a preprinted form leaves out the lines and the labels (test accessors `PdfRenderer::printed_texts`, `print::drawn_page`) and the PDF is made, Save as a style sheet With contents (the dialog; the sheet written by the same writer, never the portal), File › New With contents (protection, Honor protection and Protected Text kept), Save As SAM and reopen | No gap found. Order note TNK-TD10a: Protected Text goes on before Honor protection, because Honor protection keeps the caret out of a protected cell (as Ami Pro does); the recipe order in the guide is the same.
| TD.11 Extras | BUILT (not committed) | | `crates/editing/tests/formulas.rs` (2 new): the four accepted, blanks skipped, text cell gives REF in `@avg @min @max`, `@count` counts numbers, empty ranges 0, recalculation on change; Save As lists them once, counted (`4 formulas with a function Ami Pro lacks (@avg, @min, @max, @count), which it cannot understand`), `@sum` not listed; SAM keeps text and result and reads back equal; native v3 keeps them; scenario in `table_data.rs` (Formula box, Save As loss list through `save_sam_expecting`, reopen); the Formula section shows `FORMULA_HINT` (extras marked) | TNK-TD11a as TNK-d (notes 11/12): extras skip blanks, text gives REF except `@count`; REF is stored as the empty result (TNK-TD6a). The untested question of Ami Pro's reaction to an unknown function stays TDV3's answer (3125).
| TD.12 Measures and twins | DONE (2026-11-06; not committed; VM shut down) | | Logs and audits in `target/table-data-baseline/td12/` (not Git); twins `ATD1..ATD5` in `vm-lab/IN`, Ami Pro's saves `TDT*S.SAM` in `vm-lab/OUT/td12`, screenshots `vm-lab/screens/td12-*` (not Git); writer `table_data_twins` (`scenario_tests/table_data.rs`, `AMIPRO_TWIN_OUT=DIR`), perf `table_formula_performance_baseline` (`AMIPRO_PERF_TABLE=1`), comparison `crates/ami-formats/tests/td12_twins.rs` (`AMIPRO_VM_LAB`) | Typing, README31, release, interleaved A,B,B,A,A,B vs `table-data-start` (medians start/middle/end ms): Layout A 1.02/1.23/1.13, B 1.03/1.25/1.15; Draft A 1.04/1.17/1.14, B 1.05/1.18/1.15; Outline (B only) 1.14/1.26/1.21 (+0.1 as OB.7): no regression beyond noise (0.01-0.02 ms). Table cell with 51 dependants (40 samples, 3 x 52 table): typing in A1 median 1.67 ms (p95 1.70; 1.35 without formulas), leaving A1 (rewrite + RecalculateTable) median 3.08 ms (p95 3.20; 1.37 without formulas), so recalculation costs about 1.7 ms. Corpus gate PASS (160 SAM, 0 failed); round trip and layout comparison for the user's and ISO corpora identical to `start`/`td5`; VM folder = td5 plus the nine twins (equal in all). Formula audit: user's 261 of 261, ISO 15 of 15 equal; VM folder 113 cells, same classes as TD.4 (nothing new differs). Twins: below |
| TD.13 Exit | DONE | | | records written (haiku); gate and push by the main session |

## Choices made in the run (TNK…)

None decided; candidates from TD.3/TD.4 (notes section 11): TNK-a formula text kept as typed; TNK-b `B$2` accepted; TNK-c `A1:A1` accepted, `sum(A1)` not; TNK-d extras skip empty cells, empty `@avg/@min/@max` is 0, `@count` never REF; TNK-e a reader of a failed formula cell is REF; TNK-f range-delete rule as notes 10.6 (check with TDV5); TNK-g a number needs a digit, the style's decimal mark counts.

TD.6 candidates: TNK-TD6a a result that is REF after Insert/Delete Row/Column keeps its old text (VM TDF3 stored it stale); every other command (and Recalculate Table) writes the empty result; TNK-TD6b recalculation is whole-table `calculate` with only the dependants of the change written, so stale results elsewhere stay until Recalculate Table; TNK-TD6c moving or copying rows or columns shifts the moved formulas' relative names by the distance moved, like Paste (the VM did not test it); TNK-TD6d Edit Formula with empty text is refused (3125), clearing is `formula: None` (refused with NO_FORMULA when the cell has none); TNK-TD6e SortRows is refused when a sorted row holds a formula (3113).

TD.7 candidates: TNK-TD7a typing, Enter, Backspace and Delete inside a formula cell do nothing and the status bar says how to change the formula (Del on a selected cell takes the formula out); TNK-TD7b a refused formula stays in the Formula box with its message (status bar and under the box) until Esc; TNK-TD7c REF is drawn as the word in the result's place and says nothing, a divide by zero or a loop is blank and the status bar tells; TNK-TD7d Recalculate Table is an addition (Ami Pro recalculates on edits only) and View › Show ▸ carries the headings check item.

TD.12 choice: **TNK-TD12a** a formula cell whose result is empty (divide by zero, loop) is drawn blank and reported in the status bar; Ami Pro draws it 0.00 on opening the file (twin ATD4); kept blank (TN8: errors visible). REF stays drawn as the word (TNK-TD7c), as Ami Pro's own screen shows it in the twin.

## Baseline (TD.1), taken 2026-11-06 at `6bcac18` (release builds, `-j2`)

`target/table-data-baseline/start/` (`baseline.sh` as run A's; logs `rt-*`,
`lc-*`). Equal to run A's exit (`target/commands-baseline/ca15`) in every
row of the layout comparison and in the round trip but for timing lines.

| Corpus | Round trip: read back equal | Save As lists nothing | Main text equal | Bodies equal | Files equal but for [desc] and [pg] |
|---|---|---|---|---|---|
| User's SAM (417 read back) | 417 of 417 | 417 of 417 | 417 of 417 | 203 of 417 | 49 of 417 |
| ISO corpus (160) | 159 of 160 (1: layout definition changed) | 159 of 160 | 160 of 160 | 28 of 160 | 4 of 160 |
| VM folder (453) | 453 of 453 | 453 of 453 | 453 of 453 | 414 of 453 | 383 of 453 |

| Corpus | Files | Page count equal | Lines equal (+-1) | Page ends same | Table rows same |
|---|---|---|---|---|---|
| User's SAM | 406 | 322 | 14,455 of 14,871 | 796 of 1,108 | 904 of 980 |
| ISO | 160 | 70 | 22,789 of 24,971 | 478 of 1,307 | 463 of 1,291 |
| VM | 427 | 401 | 3,840 of 3,853 | 163 of 173 | 29 of 30 |

TD.5 changes numeric cells' text widths (about 1,750 cells in the user's
and ISO's files); every changed row against this table is explained.

## Facts found in preparation (2026-11-06; detail in `TABLE_DATA_NOTES.md`)

- Formulas in the corpora: 291 calculated cells, 224 in 48 of the user's 423
  SAM files and 67 in 4 of the ISO corpus's files. Only `sum`, `+ - * /`
  and parentheses occur; no `%`, no `$` addresses.
- A calculated cell: record F3 bit `0x40`, two bodies (result, then
  formula text as typed); results stored plain with the style's decimals.
- Number format: the style's `[nfmt]` (cell format 1 General, 2 Fixed, 3
  Currency, 4 Percent; decimals; decimal symbol; thousands character;
  currency symbol; a flag word whose bits are still to be mapped).
- Forms need no new code: protect, Honor protection, Protected Text, On
  preprinted form, and With contents were built in earlier runs.
- Tools › Sort… is in the playbook's run 3 but not in the speed-up row C;
  decision TN3.

## Facts found in the TD.1 decompile pass (detail: `TABLE_DATA_NOTES.md` section 10)

- `[nfmt]` flag word: 0x08 thousands, 0x10 leading minus, 0x20 trailing
  minus, 0x40 parentheses, 0x80 red, 0x100 currency symbol leading.
- A cell is numeric only with digits, sign, `.` and the style's decimal
  symbol: a comma, space, `%`, or `$` makes it text (corrects the first
  notes). Typed numbers are rewritten as `%.*f` with the style's decimals;
  the stored text is never decorated.
- Formula compiler: `=` skipped, `@` only inside `@sum`, left-to-right
  operators with `* /` above `+ -`, unary minus above both, `n%` is `n/100`,
  `A1%` is `/100`, one function (`sum`, case-insensitive, single row or
  column range, `..` `:` or `.`), names must lie inside the table, unknown
  function is 3125.
- Quick Add: Row sums leftwards, Column upwards, from the cell after the
  nearest non-numeric text cell, written as `@sum(a..b)` in the caret cell.
- Reference rewriting rewrites formula text; a deleted referenced cell
  becomes the word REF, which makes the formula REF.
- Sort is a heap sort (not stable).

## Twins (TD.12), opened in Ami Pro 3.1 (2026-11-06)

All nine files (ATD1, ATD2G/F/C/N/P, ATD3, ATD4, ATD5) opened as made. Ami Pro's saves read back by the app: round trip 9 of 9 equal, bodies 9 of 9 equal to the original, Save As lists nothing.

- **TW1 invoice with Quick Add column and row (ATD1):** shown as the app stored it. Price 19.99 changed to 20, Tab: Ami Pro recomputed 60.00, 83.00, 37.50, 135.00, the same as the app's engine; formulas and `[nfmt]` kept in the saved file; the edited cell shows `60` (as typed) while the caret is in it, `60.00` after.
- **TW2 number formats (ATD2G/F/C/N/P):** General `1,234.5`, `7`; Fixed `1,234.50`; Currency `$1,234.50`, red `-$7.26`, `-$1,500.00` red; parentheses variant `$(7.26)`; Percent `1234.50%` (no scaling), as the app draws them. A number typed in Ami Pro (`-2500.456`) became `-2500.46`, as the app rewrites it. Ami Pro's save equals the twin in every cell and style format.
- **TW3 sorted table and sorted list (ATD3):** opened in the order the app wrote; saved equal.
- **TW4 errors (ATD4):** the REF cell is drawn REF, `@sum` and `D4*2` as computed. Ami Pro's message on changing B1 with C1 = 0 is "There is a formula that tries to divide by zero." (3126), as the app's. **Difference:** the app stores an empty result for a divide by zero, a loop, and REF; Ami Pro draws these cells `0.00` when the file is opened (Fixed) and writes `0.00` as their result when it saves (formulas kept). TD.2 saw blank for a freshly failed cell. The app draws blank. Not changed.
- **TW5 `@avg` via Save As (ATD5):** Ami Pro opens it with the message "Cannot understand the formula in cell 'A4'." (3125) and shows the formula's text `@avg(A1..A3)` in the cell; saved, the cell is plain text (formula and result lost), which the Save As loss list warns of.
