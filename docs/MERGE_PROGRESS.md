# Merge Progress

The live state of `docs/MERGE_RUNBOOK.md` (tag `merge-run-start`).
Resume at the first slice that is not `DONE`.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| ME.1 Setup | DONE | `838ff50` | `MERGE_NOTES.md` §11 (VM fixtures MGM1–MGM9, MGMA–MGMF, MGN1–MGN7, MGS4–MGS8, MGT1, made by a subagent from the run's inputs MGD1, MGL1–MGL9); `UI_SPEC.md` §38; the baseline below | Tag `merge-run-start` |
| ME.2 Model and SAM | DONE | `8643186` | `amipro_document::merge` (`RecordFile`, `Condition`, `Operator`, `Join`, `MergeSettings`); native v3 `recfile`, `mergecondition`, `withconditions`; `take_contents` types `[recfile]`, the writer writes it in Ami Pro's place; `LossKind::MergeConditions`; `Command::SetMerge`. Tests `tests/merge_sam.rs` | Measured (below) |
| ME.3 Data in the core | DONE | `8643186` | `merge::data`: `DataTable`, `field_key`, Ami Pro's data document, a table document, description files (DIF, FIXED, COMMA), CSV/TSV, DBF, `data_paragraphs` (the delimited writer), `fits_delimiters`; `merge::conditions` (Ami Pro's numbers, dates, case-sensitive text, And before Or) | MGT1 not yet read in a test |
| ME.4 Spreadsheets | DONE | `6cae55f` | `amipro_exchange::sheets::read_xlsx`, `read_ods` (shared and inline strings, date styles, repeated rows and columns capped) | |
| ME.5 The merge in the core | DONE | (this commit) | `FieldHost::merge_value`, `merge_record`, `next_record`, `skip_record`; `MergeField`, `MergeRec`, `NextRec`, `Skip`, `Defined`, bare names; an `IF` branch not taken runs no `NextRec` or `Skip`; `merge::merge_one`, `merge_all`, `joined`, `drop_empty_field_paragraphs` (MC2), `tidy_empty_fields` (MG9). Tests `tests/merge.rs`: MGM1, MGM2–MGM5, MGM6, MGS6, MGS7, MGM7 as Ami Pro made them | MC1 |
| ME.6 The Merge panel | DONE | `16327cb` | File › Merge…, Insert › Merge Field…; `merge.rs`, `merge_panel.rs`, `merge_commands.rs`: the data file read off the interface's thread, Open…, New Data File…, Edit records…, fields inserted one step each (written `<:X12,-32768;MergeField NAME>`), MergeRec, NextRec, Skip if…, the fields the data file lacks named (MGM8: they merge empty with Ami Pro's message), the preview with ◀ ▶ (display only, MC3), conditions one step each with the live count. Tests: `the_merge_panel_inserts_fields_previews_records_and_merges_mg1_mg5_mg6`, `fields_the_data_file_lacks_are_named_and_merge_empty_mgm8`, `the_merge_panels_buttons_take_clicks` (every section, the rail scrolled; `click_unrun`), `merge_screenshots` (ignored; light and dark, outside Git) | MC1, MC13 |
| ME.7 Outputs | DONE | `16327cb` | New Document; PDF… (one, a bookmark per record) and One PDF per record named from a field (never over a file, MC12); Printer… (the desktop's print dialog once, its pages across the output, MC9); View & Print (each record in place, edits carry, Print and View Next, Skip and View Next, Print All, Cancel; the records to print sent together, MC7); Ami Pro File… (new files only). The merge works through the app's field host (Include). Tests: `merged_to_an_ami_pro_file_every_record_follows_a_page_break_mg4`, `view_and_print_shows_each_record_in_place_and_carries_edits_mg4`, `one_pdf_per_record_is_named_from_a_field_mg4`, `merge_to_printer_asks_once_and_prints_the_chosen_pages_mg4` (print tasks never run) | MC7, MC9, MC12 |
| ME.8 The record editor | DONE | `16327cb` | `data_grid.rs`: the data file's tab as a grid (a click on a heading sorts, Find, pages of 200 records, + Record, − Record, + Field, Rename, − Field, Move ◀ ▶, a value box), each change one undo step through `merge::data::set_data_command` (the data document rewritten in Ami Pro's delimited form; a value holding a delimiter refused), Undo followed; open merge documents follow the data tab's records live; New Data File… asks only for names and writes a new SAM. Core: `DataTable`'s editing methods, `new_data_document`. Tests: `the_record_editor_makes_and_edits_a_data_file_one_step_each_mg3`, `the_record_editors_buttons_take_clicks`, `merge::data` tests | MC6 (Save over a SAM data file: answered 2026-11-04, written over with a backup the first time) |
| ME.9 Labels | DONE | `16327cb`, `ad9408f` | `merge::labels`: the app's sheet list (26 Avery sheets, letter and A4, published sizes; MC5), `set_up_labels` (one step: Ami Pro's custom rule, `[recfile]`'s As labels, across, down, indents; tabs past the label dropped), `LabelSheet::of`, `place` (`1550:0145`'s formula), `label_run` (copies, start label), `label_sheets` (New Document: whole sheets, MC8); the panel's sheet list, the sheet drawn small (a click picks the start label), copies; PDF and Printer place labels on sheets (`print::labels_pdf`). Tests: `merge::labels` tests, `merged_labels_go_on_sheets_from_the_start_label_mg7` | MC5, MC8 |
| ME.10 Envelopes | DONE | `16327cb`, `ad9408f` | File › Print Envelope… (a new command) opens `envelope_panel.rs`: Ami Pro's sizes and Custom (3–11 in), the recipient (the selection, else `merge::envelopes::first_address` on page 1), return addresses saved by name in the settings, Print return address, feed and offsets (in the settings), Print… (the desktop's dialog; the envelope at the feed's place on larger paper, MC10), PDF…, New Document, Merge Envelopes (`fill`: the Merge panel's data file and conditions, `<NAME>` in the recipient); on `_AMIENV.STY` (its `Recipient` style) when found. Tests: `print_envelope_finds_the_address_and_makes_the_envelope_mg8`, `merged_envelopes_take_the_documents_data_file_mg8`, `merge::envelopes` tests; screenshots | MC10 |
| ME.11 Merge sheets | DONE | `16327cb` | The Merge panel's Start from: Letter (a new document from `_MERGLET.STY` with its contents and the panel), Labels (`_LABEL.STY`, labels on), Envelope (the Envelope panel), until File › New's chooser (queue 15). Test `start_from_opens_the_merge_sheets_documents_mg11` | |
| ME.12 Measures | DONE | (exit) | Round trip and layout comparison equal to the first session (below); release timings (below) | |
| ME.13 Twins | DONE | `850655b` | `merge_twins` (ignored; `AMIPRO_TWIN_OUT`, `AMIPRO_VM_LAB`): AMD1 (New Data File… and the record editor), AML1 (fields clicked in the panel, text typed between), AMLB (Avery 5160) by Save As SAM. In Ami Pro (VM screenshots AMT-*, outside Git): all three open without a message; Ami Pro's Merge & save as of AML1 (AMT1) equals the app's merge paragraph for paragraph; AMLB's page is Custom 2.75 × 1 in. with Ami Pro's margins, As labels checked, Labels… 3 × 10 at 0.50 and 0.19 in. (after the fix below) | Found: text typed after a field went inside it (MC11); the Labels unit is 1 for in. (0 showed none) |
| ME.14 Exit | DONE | (handoff commit) | `tools/verify.sh --release --corpus` PASS: core 599 + 14 ignored; app 422 + 71 (default), 116 + 6 (text editor), 429 + 71 (all features); release builds; corpus audit. The records updated | Physical check pending; MC6 answered 2026-11-04 |

## Libraries (ME.1)

- **XLSX and ODS:** no new dependency. `calamine` (MIT) is the usual
  choice but brings `quick-xml`, `zip`, `encoding_rs`, and more, and is
  not in the offline cache; `amipro-exchange` already has the project's
  own zip reader (inflate from the caller) and namespaced XML tree, with
  limits, so the first sheet of each is read there (`sharedStrings.xml`
  and the first worksheet by `workbook.xml`'s relationships; ODS
  `content.xml`'s first `table:table`, with repeated rows and columns
  capped).
- **dBASE `.DBF`:** read by its public format (dBASE III/IV header and
  field descriptors, CP-1252 or the language driver's code page), no
  library.
- **CSV/TSV:** RFC 4180 by hand in the core (Ami Pro's comma rules are
  their own and kept apart).
- **Printing and PDF:** the print and PDF run's portal and `PdfRenderer`
  (`print::set_pdf` takes parts, as the master run's set).

## Envelope sizes (ME.1, from the shipped macro's list)

Monarch 7½ × 3⅞ in; 6¾ 6½ × 3⅝; 9 8⅞ × 3⅞; 10 9½ × 4⅛; 11 10⅜ × 4½; 12
11 × 4¾; Custom; DL 220 × 110 mm; C5 229 × 162; C6 162 × 114; C65 230 ×
110; Italian 230 × 110. (DL, C5, and C6 are Ami Pro's own, so MG8's
"plus DL, C5, C6" adds nothing.)

## Baseline (at `merge-run-start`, `target/merge-baseline/start/`)

Measured with `target/merge-baseline/baseline.sh` (outside Git): equal to
the master run's exit (`target/master-baseline/mr11/`) but for timings and
one new VM fixture (MGM1) that reached the VM folder during the run.

- **Round trip:** user's corpus 417 of 417; ISO 159 of 160; VM folder 388.
- **Layout comparison:** the corpora as at the master run's exit; VM folder
  360 of 361 page counts (MGM1, a merged file Ami Pro wrote without its
  lone `>` line: to look at in ME.12).

## After the first session (`target/merge-baseline/s1/`)

- **Round trip:** user's corpus and ISO equal to the start; VM folder 414
  of 414 read back equal, Save As lists nothing for 414 (the 26 new merge
  fixtures included).
- **Layout comparison:** both corpora equal to the start. VM folder 365 of
  387 page counts: the 22 misses are Ami Pro's merged files (MGM*, MGN*,
  MGS*), whose `[pg]` cache is the one-page merge document's (Ami Pro
  copies the merge document's header), so the oracle is stale there; no
  other row moved.

## At the exit (`target/merge-baseline/exit/`)

- **Round trip:** user's corpus, ISO, and VM folder equal to the first
  session (the VM folder now 415 files with Ami Pro's merge of the app's
  twin, AMT1, read back equal).
- **Layout comparison:** both corpora equal to the first session; VM
  folder 365 of 388 page counts (AMT1 added, a merged file whose `[pg]`
  cache is the letter's, as the other merged files), paragraph line counts
  2402 of 2414 (was 2399 of 2411), every other aggregate the same.
- **Release timings** (this host, `powersave`; `merge_performance_baseline`):
  1,000 records to a new document 177 ms; to one PDF 3.1 s (off the
  interface's thread); a preview step 1.9 ms median, 4.8 ms worst; a
  keystroke with the preview on 1.9 ms; a record editor value on 1,000
  records 11.0 ms median, 13.8 ms worst (step and frame).
- The full suite found a panic the merge tests missed: the record editor's
  changes and View & Print's records did not reach the hidden editor's
  text, so Undo there panicked in cosmic-text (fixed `d4d99cf`, Undo and
  Redo tested in both).

## Choices made in the run (MC…, also in `DECISIONS.md`)

- **MC1** A text ordered against a number holds for none (MGM7, MGS4).
- **MC2** Paragraphs holding only empty merge fields, NextRec, or Skip are
  left out of merged output (Ami Pro: `<:S+0>`, no height).
- **MC3** An edit or Undo during the preview turns it off and is not made.
- **MC4** Date conditions compare as dates; Ami Pro's odd results for
  `3/1/94`, `1/1/94`, `03/01/94` are open (`1538:02db`).
- **MC5** The sheet list holds Avery's laser and inkjet sheets (letter and
  A4) by their published sizes, 26 sheets with their equivalent parts;
  Ami Pro's continuous forms for tractor-feed printers are left out.
- **MC6** The record editor's data document follows the app's file rules;
  merge documents follow an open data tab's records live, so a merge needs
  no save. **Answered 2026-11-04:** the user allowed writing over SAM
  files with a backup the first time, so Save of an edited SAM data file
  writes over it (`NAME.SAM.bak` kept the first time), as Save on every
  SAM document does since (`docs/research/DECISIONS.md`, 2026-11-04).
- **MC7** View & Print keeps the records chosen to print and sends them
  together at the end, through the desktop's print dialog once (the
  portal asks for a printer per job).
- **MC8** Labels merged to New Document are whole sheets, each label a
  transparent Where placed frame at its place (MG7's whole sheet); As
  labels dims View & Print and Ami Pro File…, as Ami Pro dims view and
  save.
- **MC9** Merge to Printer…'s page range counts across the merged output,
  as the desktop's dialog shows it.
- **MC10** An envelope's page is its size; when the print dialog chooses
  larger paper, the envelope goes at the feed's place (left, centre,
  right) along the paper's top edge.
- **MC11** Text typed right after a power field goes after it (end marks
  stay before typed text), as Ami Pro's caret stands after a field it
  puts in. The fields run had typed text at a result's end go in, a rule
  of its own (FD3 typed inside a result) under which Update deleted text
  typed after a field; shown codes keep their own rule. Found by the
  twin: fields clicked in turn with text between nested.
- **MC12** One PDF per record is never written over a file: " (2)" and on
  is added; an empty name takes "Record N".
- **MC13** Fields the data file lacks are named in the Merge panel (Ami
  Pro's 3225 asks on choosing the file, 3226 before merging) and merge
  empty with Ami Pro's message 3098; the panel is not modal, so nothing
  asks to continue.

## Open points found

- Ami Pro's date parser (MC4).
- dBASE memo fields (`.DBT`) are not read.
- Labels and envelopes printed by Ami Pro to PostScript (positions) were
  not made in the VM; positions follow `1550:0145`'s formula.
- Found and fixed on the way: a field put in right after another went
  inside it; a NumPages field's marks were rewritten at stale places
  after its result changed length (the change order in `update_fields`).

## Rework after first use (2026-11-04, MR1, MR2)

The user's first use failed the record editor ("adding to the grid and
then typing doesn't put the values in the grid") and the panel ("overly
complicated, small and cramped"). Asked with a recommendation each, the
user chose data files entered as Ami Pro tables (MR1) and the panel as
Ami Pro's three steps (MR2); `UI_SPEC.md` §38, "Rework after first use";
`DECISIONS.md`, 2026-11-04.

| Slice | State | Evidence |
|---|---|---|
| MR.1 Core | DONE | `merge::data::table_data_document`, `data_document`, `delimiter_clash`; `TableEdit::SortRows`, `tables::sort_order`; tests `table_data_documents_hold_the_records_as_ami_pros_table_data_files`, `sorting_a_tables_rows_keeps_the_heading_rows_first` |
| MR.2 Data files as tables | DONE | `data_file.rs` (replaces `data_grid.rs`); New Data File… writes a table file; Save writes a delimited file back in its form; Table › Sort Ascending, Sort Descending; tests `data_files_are_entered_as_ami_pro_tables_mr1`, `delimited_data_files_are_edited_as_tables_and_saved_in_their_form_mr1` |
| MR.3 The three steps | DONE | `merge_panel.rs` (`Step`, Options, 340 px); `the_merge_panels_buttons_take_clicks` through each step; screenshots `window-merge-panel`, `-step-data`, `-step-merge`, `-labels`, `-record-editor` |
| MR.4 Twins | DONE | AMD1 (typed into the table) opens in Ami Pro as made; Ami Pro's merge over it (AMT2) equals the app's (`merge_twins` compares AMT1 and AMT2); AML2/AMT3: Ami Pro merges a table cell that reads as a number as a number (`MERGE_NOTES.md` §13, open) |
| MR.5 Exit | DONE | `tools/verify.sh --release` PASS: core 601 + 14, app 423 + 71, 117 + 6, 430 + 71; release: 1,000 records opened as a table 65 ms, a keystroke in that table 17 ms (the grid's value was 11 ms), the merge timings unchanged (178 ms to a new document, 3.0 s to a PDF, a preview step 1.9 ms) |
| MR.6 Fix from the second use | DONE | 2026-11-04: New Data File… onto an existing file did nothing after the chooser's Replace (the new-file-only write refused it; the refusal was a caption at the panel's foot). It now writes over the file, keeping `NAME.SAM.bak` the first time (MC6), refreshes a tab showing the old file, and refuses a tab with unsaved changes or the merge document itself, in the panel (its message now at the top) and the status bar; test `a_new_data_file_writes_over_a_confirmed_file_with_a_backup` |
| MR.7 Fields kept whole | DONE | 2026-11-04, the user's address block: the preview's caret put fields and Enter inside the letter's fields (and left empty fields after Zip). `merge::source_position` maps the caret back when the preview goes off; typing in the preview is made in the letter (MC3 changed); fields go in outside other fields; Backspace and Delete beside a field take it whole, empty ones first (`Command::DeleteFields`); no false "not a valid field name"; tests `an_address_block_typed_around_the_preview_stays_whole`, `backspace_and_delete_take_whole_fields_and_leave_none_invisible`, `a_place_in_the_preview_maps_back_to_the_letter` |

Open: a number in a table data file merges as Ami Pro shows it (`75.50`
as `75.5`) only once tables show numbers as Ami Pro does (the table-data
run); a keystroke in a 1,000-record table takes 17 ms.
