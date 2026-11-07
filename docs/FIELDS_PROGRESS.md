# Fields Progress

Live state of `docs/FIELDS_RUNBOOK.md`. Resume at the first slice that is
not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| FL.1 Setup | DONE | (setup commit) | The runbook; this file; the baseline below | Tag `fields-run-start` |
| FL.2 Dates in the model | DONE | (this commit) | `fields.rs` rule tests (all 21 formats on FD4's date, FD1's; the numeric settings by the decompile; the time's padding, FD5; the time base, FD2 and FD5; the old US summer-time rule; Doc Info codes); native `inline date`, `inline docinfo`; importer and writer tests from FD1 and FD2 (a tab written as it is); round trip unchanged (user 417/417, ISO 159/160, VM 278/278 with the six FD files; `target/fl2/`) | `amipro_document::fields`: `DateField` (`DateSource` Today, LastSaved, Created, Time), `DocInfoField`, `DocInfoEntry`, `WallClock` (`from_ami_pro`, `to_ami_pro`), `DateSettings::US`, `format_date`, `format_time`; `InlineObject::Date`, `InlineObject::DocInfo` (any flow; zero-width until FL.3); `writer::escape_control_bytes` |
| FL.3 Dates on the page | DONE | (this commit) | `fields.rs` `dates_and_doc_info_entries_show_what_ami_pro_shows_fd1_fd2`; `rules.rs` `dates_and_doc_info_entries_show_their_text_fd1_fd2` (one piece, measured in the paragraph's style; a new context lays a document with fields out again); screenshots of FD1 and FD2B in the app as Ami Pro shows them (the time's leading space) | `FieldContext` (the clock to the minute, the file, this session's save and minutes, the last page count; `date_text`, `doc_info_text`); `layout_document_with`, `IncrementalLayout::set_field_context` (a document with fields is laid out whole again when it changes; `Document::has_fields`); the engine's `object_text` for page numbers, dates, and Doc Info entries; the session's `field_context`; the surface's line key holds the fields' texts; the layout comparison shows dates at each file's last save. Measured (`target/fl3/`): the user's corpus and the ISO corpus unchanged in every summary and every file's row (their dates stand alone on their lines, mostly at the top of letters, so no line count moves); the VM folder unchanged in every earlier file's row, the six FD fixtures added with every cached line equal |
| FL.4 Doc Info | DONE | (this commit) | `info.rs` tests (FD2B's `[desc]` and `[fldnames]` read and written back; the 8-value form; a new document; escapes); native `docinfo`, `docfield`; the sheet test now writes the revisions as held (2, Ami Pro's `2375`); app: `new_documents_have_ami_pros_doc_info_and_saves_count_the_editing_time`, `clock.rs` tests; round trip of the VM folder unchanged (278/278) | `DocumentInfo`, `Statistics` in the model (`Document::info`), `amipro_formats::take_doc_info` (the importer; older native files at opening), `info::description_values`, `field_name_values`; both writers from it; `SamOptions { editing_minutes, statistics }`, `SamOptions::at`; the app's `clock` (`jiff`): saves in Ami Pro's time base, minutes since the session opened, a revision counted when a native file opens, Ami Pro's Doc Info for new documents. The entries' shown text came with FL.3, the statistics with FL.5 |
| FL.5 Statistics | DONE | (this commit) | `words_and_characters_are_counted_as_ami_pro_counts_them_fd2` (FD2's 6 and 23; tabs, no-break spaces, hyphens, hard returns, objects); `counted_statistics_are_written_with_the_files_own_size`; the app's options carry them | `count_words_and_characters` (every text stream); `sam_document` writes the counted statistics with the file's own size in K (written again until it is); value 14 the same (FX2: Ami Pro adds the style sheet and linked files to it and shows it nowhere); the native file's Doc Info gets them too; the loss check leaves out what each save rewrites (values 9–14) |
| FL.6 Power fields in the model | DONE | (this commit) | `power.rs` tests (FD3's marks read and written back byte for byte, escapes, flags; FD6's Seq numbers in their styles and FD3's locked 8197; pairing over paragraphs, fields without results, nesting, a start whose end is gone); `seq_fields_are_objects_written_as_ami_pro_writes_them_fd6` (and FD3B's order); native `inline seq`; round trip unchanged (417/417, 159/160, 278/278), bodies equal 274 and 28 (one more each: FD3B and an ISO file) | `PowerMark` (`parse`, `raw`), `power_fields` (`FieldSpan`, `MarkPlace`), `power_kind`, `power_flag`, `has_result`, `SeqField` (`InlineObject::Seq`; the cached `<:X~4;N>` dropped on reading as Ami Pro's reader drops it and written back), `number_in_style`; results stay ordinary text between kept marks; the writer puts a field's start mark after the formatting that starts with it (keeping the marks' order); `decode_windows_1252` and `windows_1252`, Ami Pro's roman numerals and letters moved into the document crate |
| FL.7 The field language | DONE | (this commit) | `field_language` tests: FD3's 21 fields, FD6's and FD7's 40 expressions, what keeps its result or waits, the prompts and Include through a host, Seq as FD6 counts it, the kinds; `field_twins.rs` (ignored, `AMIPRO_VM_LAB`): 45 fields of Ami Pro's FD3, FD6, and FD7 worked out by the app give the results Ami Pro saved | `amipro_editing::field_language`: `evaluate` (`Outcome`: a result, nothing, kept, waiting), `FieldEnvironment` (the document, the context, the session's variables, the bookmark names, a `FieldHost` for Include, Query$, Decide, and warnings), `field_kind`, `seq_number`, `seq_name`, `number_text`, `time_code`; VM fixture FD7 (numbers, operators, text functions) |
| FL.8 Updating | DONE | (this commit) | `fields.rs` tests: inserting worked out at once between the marks (FD3), a Seq numbered after the one before it; Update at the caret puts the new result between the marks (text typed at a result's end is in it, at its start before the field); an up-to-date or locked field makes no change; Update All in order with the variables and the Seq renumbered, one step; the field at the caret; Next and Previous Field round the ends | `amipro_editing::fields`: `update_fields` (`UpdateScope` All, At, Between, AutoRun; `FieldInputs`; `FieldUpdate` with the waiting prompts), `insert_power_field`, `field_at`, `all_fields`, `next_field`, `bookmark_names`; commands `InsertPowerField`, `SetFieldResult`, `SetFieldMarks`, `InsertFieldObject`, `SetFieldObject` |
| FL.9 On the page | DONE | `642ce00` | `fields_are_inserted_updated_and_shown_through_the_fields_panel` (message path: shading over the result; screenshots `field_panel_screenshots` (ignored): `window-fields-*`, `window-document-properties-*`, light and dark; Show Power Fields shows the code in red and the text is not edited until it is off; Next Field selects the whole field; Lock Field checked) | `field_commands.rs`: `refresh_field_views` (the shading ranges, `codes_document` while codes show), `shown_document`, the surface's `shading` (a light grey, `FIELD_SHADE`), page events ignored while codes show, the status bar naming the caret's field, Show Power Fields off before a save, Auto run when a document opens, the Query$ and Decide dialog (`FieldPrompt`, the update run again with each answer), Include from the document's folder (a SAM's main text, a text file's lines) |
| FL.10 The Fields panel | DONE | `642ce00`, the screenshots' commit | `fields_panel` `insertions_are_ami_pros` (Today's date as text, System date with its format, the time keeping the list's position, Document Description as a power field, a Doc Info entry, a power field with its format and Lock, saved fields); the message-path test above | `fields_panel.rs`: Insert (Date and time with Ami Pro's 5 choices and 21 formats shown as samples, Document info with the 22 entries and their values, Power field with Ami Pro's 41 fields (8 marked not run), options, a syntax line, Lock, Update when opened, saved fields), Shows (a live preview), This field (format, code, Lock, Update when opened, Update) |
| FL.11 Document Properties | DONE | `642ce00` | `document_properties` tests (typed texts one change on commit; untouched names stay; FD's counts; times with their year); `document_properties_edit_doc_info_one_step_at_a_time` (message path) | `document_properties.rs`: Description (119), Keywords, Fields (names and values, 31 each), Statistics (counted as the document changes), History (created, last saved with this session's, revisions, editing time), File (name, folder, style sheet, Run frame macros, the annotation lock); `Command::SetDocumentInfo` (`ChangeReport::info`) |
| FL.12 Commands | DONE | `642ce00` | Registry tests (every command in one menu, the approved order); the right-click menu tests (the field items on a field only) | Registered: Insert › Date/Time…, Field… (implemented); Tools › Update Fields (F9), Update All Fields (Ctrl+F9), Lock Field; Edit › Next Field, Previous Field; View › Show Power Fields (Alt+F9), Field Shading (checked); File › Document Properties… (Alt+Enter), Tools › Word Count; the right-click menu's Edit Field…, Update Fields, Lock Field (`EnableRule::OnField`); the preferences `field_shading` (on), `saved_fields` |
| FL.13 Twins | DONE | `642ce00` | VM twin AFD1 (`field_twins.rs` `write_field_twin`): Ami Pro shows the app's fields, Doc Info, times, and counts as the app does; its Update and Update All agree (`FIELDS_NOTES.md`, "The twin") | The after-pagination flag an update sets on NumPages, found by the twin |
| FL.14 Timings | DONE | `145304e`, `27dbae8` | `field_performance_baseline` (ignored, release; `AMIPRO_PERF_SAM`), `field_refresh_cost` | README31 with 100 fields (NumWords and a System date in each of the first 50 paragraphs), to the drawn frame: a keystroke 1.36 ms median (1.93 worst); Update All 22.9 ms (26.4; each NumWords counts the whole document); its undo 11.7 ms; Show Power Fields on and off 39.5 ms (two whole layouts). Without fields a keystroke 1.06 ms (0.9 at the Find & Replace exit, a busy host); the fields' own work after each change 24 µs (the shading walk) and 0.2 µs (the context and the clock). Typing A/B against `fields-run-start` (`tools/bench-ab.sh`): three rounds (`target/bench-ab/20261020-141714/`) showed the default build about 0.1 ms slower an edit at the fixture's end (the view asked for the caret's field several times a frame, each walking the flow), the rich editor prototype 0.03–0.07 ms; after `27dbae8` (the session's fields of the last change answer it) two rounds (`…/20261020-142300/`) overlap: typing at the fixture's start 0.87 against 0.78–0.84 ms, middle and end mixed (0.85–0.92 against 0.83–0.88; 0.80–0.84 against 0.69–0.88), the rich editor no slower. Far below a frame |
| FL.15 Exit | DONE | (the exit commit) | `tools/verify.sh --release --corpus` PASS (core 404 + 8; app 289, 111, 296 with 36, 4, 36 ignored; ISO corpus gate PASS); the measures at the exit (`target/fields-exit/`) equal those of FL.3 in both corpora, the VM folder with this run's fixtures all read back equal; the typing A/B (FL.14) | `UI_SPEC.md` §28 notes; `FEATURE_PARITY.md` RF-03; `UI_ACCEPTANCE.md` "Fields" and its sign-off row; `UI_COMMAND_MATRIX.md`; `FORMAT_COMPATIBILITY.md`; `SAM_FORMAT_NOTES.md`; `DECISIONS.md` FX1–FX13; `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`; the playbook's queue (print and PDF NEXT); `docs/SESSION_HANDOFF.md` |

## Baseline (at `fields-run-start`)

The Find & Replace exit's measures (`target/find-exit/`, at `6ae74ea`): no
crate or layout code has changed since (`git diff 6ae74ea HEAD -- crates`
is empty).

- Round trip: user's corpus 417 of 417 read back equal, ISO 159 of 160, VM
  folder 272 of 272.
- Layout comparison: user's corpus page counts 315 of 394, page ends 762 of
  1,108; ISO 69 of 160, 478 of 1,307; VM folder 271 of 271, 111 of 121.
- Tests: core 370 + 6 ignored; app 281, 108, 288 (33, 4, 33 ignored).
- The VM folder has six new files since (FD1, FD2, FD2B, FD3, FD3B, FD6,
  this run's fixtures); its round trip is measured again with them in FL.2.

## Log
- 2026-10-20, FL.2: native v3 is extended in place, as the bullets run did
  (`inline bullet`), not bumped to v4: the runbook said v4; the records are
  new kinds of an existing record, and older files read as before. Found:
  bold that starts where a power field starts is written after the field's
  start mark (FD3B, `<:X…><+!>` for Ami Pro's `<+!><:X…>`); the same text
  reads back, and FL.6, which makes the marks typed, writes it Ami Pro's
  way.
- 2026-10-20, FL.4 (model): Doc Info is the document's (`DocumentInfo`);
  the kept `[desc]` and `[fldnames]` stay only as empty places, so the
  writers keep the sections' order and unknown sections stay where they
  were. Found: Save as a Style Sheet from an opened document wrote the
  file's revisions; Ami Pro writes its count in memory, one more (its
  writer writes `2375` for sheets too), so the sheet now does (FX1). The
  app's saves were in real UTC; they are in Ami Pro's time base now (the
  wall clock read as US Pacific time), so Ami Pro shows the app's save and
  creation times as the clock showed them.
- 2026-10-20, FL.5: FX2, `[desc]` value 14 (the size with the style sheet
  and linked files) is written as the file's own size: Ami Pro shows it
  nowhere, and 358 of 370 user files hold the two equal. The Doc Info
  entries Number of Words, Characters, and Size show the stored figures,
  as of the last save (Ami Pro: as of the last Update); Number of Pages
  follows the layout, as Ami Pro's does (FX3).
- 2026-10-20, FL.6: FX4, a power field's marks stay legacy marks (their
  text read on demand by `PowerMark`), so they move, are deleted, are
  copied, and are written with the text as every kept control is; only
  Seq is an object. FX5, a mark left without its partner (part of a field
  deleted) is written as it is: Ami Pro's reader turns it into a dead
  control and drops it, and the app ignores it. Moving every field start
  after the formatting at its place reordered marks in three user files
  (read back no longer equal); only the run of start marks that ends the
  marks at a place moves, and the three read back equal again.
- 2026-10-20, FL.7: VM fixture FD7 settled the numbers (a whole result as
  a whole number; up to two places shown with two; others with sixteen,
  read to sixteen significant digits), `+` on text (a number: "a" + "b" is
  0, "10" + 5 is 15), `|` and `&` as bitwise, and the text functions
  counting from 1. FX6: a name the document has as a bookmark keeps its
  field's result (bookmarks come with the notes run); a bare name nothing
  defines shows `<NAME>` with Ami Pro's warning, as Ami Pro outside a
  merge. FX7: an unknown function is a macro's and keeps its result.
- 2026-10-20, FL.8: FX8, a result of several paragraphs (Include) is put
  in as lines of one paragraph (hard returns), not as new paragraphs;
  FX9, Update All walks the streams in Find's order (Ami Pro's Update All
  takes the main text, headers and footers, floating headers and footers,
  layout frames, then page frames: the order matters only for Set
  variables used across streams); FX10, a field's result changed by an
  update keeps the formatting at its start (as typing there would).
- 2026-10-20, FL.9–FL.12: FX11, while Show Power Fields shows the codes,
  the pages show a copy of the document (`codes_document`) and nothing on
  them is edited (the status bar says why; Ami Pro lets the codes be typed
  over, which the proposal left out); the caret waits at the copy's start.
  FX12, a Seq's code changes in This field but not its flags (Lock on a Seq
  is Ami Pro's fault, FD3). FX13, a field's prompts are asked when it is
  inserted, as Ami Pro works a field out at once.
- 2026-10-20, after the exit (the user's answer to the note on FX11, and
  "shouldn't there be menu items to show and update powerfields?"): Show
  Power Fields now puts the codes in the text itself, to be typed over
  (`Command::ShowPowerFields`, `HidePowerFields`, `fields::swap_codes`,
  `Document::shown_codes`, `SetAside`); Update, insertions, and Edit
  Field's changes read the codes back first and show them again
  (`fields::codes_hidden`, `CodesHidden::step`); Update All and saves
  leave them off. Edit › Power Fields ▸ holds Update Fields, Update All
  Fields, Next Field, Previous Field, Edit Field…, and Lock Field (moved
  from Tools and Insert). Evidence: `fields.rs` tests (the codes in red,
  set aside and given back with formatting, objects, nested fields, and
  runs split or whole; edited, emptied, and Seq codes; updates and
  insertions with codes shown; the caret through the swap);
  `field_twins.rs` `showing_and_hiding_codes_gives_every_document_back`
  (ignored, `AMIPRO_CORPUS`): the user's 417 documents, the ISO's 160,
  and the VM folder's 280 show and hide back to themselves (7, 8, and 7
  with power fields); the app's message-path test types over a code,
  updates it, hides it, and saves with codes shown.
  `tools/verify.sh` (run by a subagent): core 410 + 9 ignored, the app's
  289, 111, and 296 with 36, 4, and 36 ignored; core Clippy found a type
  too complex and a late initialisation in `fields.rs`, both fixed, Clippy
  then clean and the sweeps again equal.
- 2026-10-20: the user's physical check passed (the user, 2026-10-20: "Powerfields work").
