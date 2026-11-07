# Table Data Runbook (run C)

**Written:** 2026-11-06 (a draft, before the decisions are asked), for the
speed-up plan's run C (`docs/SPEEDUP_PLAN.md`; `docs/PHASE5_PLAYBOOK.md`,
row C; `docs/playbooks/TABLES.md`, "Run 3: `table-data`").

**Purpose:** numbers in table cells as Ami Pro 3.1 has them, as one
straight run: cell addresses and formulas (Table › Edit Formula…, Quick
Add ▸ Row and Column), the paragraph style's number format and numbers
shown and typed through it, numbers in table cells merged as numbers (the
open point of `MERGE_NOTES.md` §13), forms (a scenario over commands built
in earlier runs), and, if the user agrees (TN3), Tools › Sort…. The
decisions TN1–TN11 (answered 2026-11-06, section 2) were asked before the run started, together
with run D's if D follows; the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work. `PROJECT_PLAN.md` defines
scope, `docs/research/DECISIONS.md` the decisions, `docs/Design/UI_SPEC.md`
a new section for the run (§41, written in TD.1). Evidence:
`docs/research/TABLE_DATA_NOTES.md` (decompile brief, corpus counts, the
open points and the fixtures TDV1–TDV8), with `TABLE_EDIT_NOTES.md`,
`TOC_INDEX_NOTES.md` ("Numeric table cells"), `MERGE_NOTES.md` §13.

**Weight (SU1).** Full: the run writes SAM structures the app has not
written before (a calculated cell's two bodies, `[nfmt]` edited, results
as Ami Pro stores them): decompile mapping, VM fixtures one property at a
time (only for what the decompile leaves open), twins.

**Live state** is in `docs/TABLE_DATA_PROGRESS.md`. The tag is
`table-data-start`.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`, this runbook,
   `docs/TABLE_DATA_PROGRESS.md`, `docs/research/TABLE_DATA_NOTES.md`, and
   the run's `UI_SPEC.md` section; then `git status` and
   `git log --oneline -15`.
2. Never move the tag `table-data-start`.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions already taken

Answered by the user 2026-11-06 (TN1–TN4 asked, each the recommended
option; the user's standing rule is to continue unless input is needed, so
TN5–TN11 take their recommended option too). The full text of each option
is in "Decisions to ask" at the end of this file and in
`docs/research/DECISIONS.md`.

- **TN1:** formulas are edited in the rail (Table Properties › Formula).
- **TN2:** numbers show through the style's number format, as Ami Pro.
- **TN3:** Tools › Sort… is in the run, as a rail Sort panel.
- **TN4:** Ami Pro's set plus `@avg`, `@min`, `@max`, `@count` (native-only).
- **TN5–TN11 (recommended options taken):** as Ami Pro's recalculation (on
  leaving a cell or applying a command, none on open); the number format
  edited in the style's Table format section; forms get a scenario and no
  new feature; errors in the cell with messages in the status bar;
  View Preferences › Table row/column headings (off by default); merge
  numbers as Ami Pro once TD.5 has the display; one straight run.

Choices made during the run are TNK1, TNK2, … in the progress file and
`DECISIONS.md`, each keeping Ami Pro's behaviour where it has one.

**Earlier decisions that apply:** rail inspectors over modal dialogs; edit
in place (values typed where they are shown, Ami Pro's tables reused);
consistent conventions (check marks on every on/off item); SU4 (typing
runs are one undo step); the table editing run's TD1–TD11 (the Table
menu, Table Properties in the rail, Tab adds a row, the app undoes every
table command); SAM sources written over with a `.bak`; Save As lists what
Ami Pro cannot hold.

## 3. Working rules

As run A's (`docs/COMMANDS_RUNBOOK.md` §3) and the table editing run's:
- Every document change is one `amipro_editing::Command` and one undo step
  through the app's message path, **recalculated dependants included**
  (a formula edit, a cell edit, a paste, a delete, an insert or delete of
  rows or columns, a sort), written as Ami Pro writes it (round trip
  unchanged unless the slice is meant to change it).
- Evidence first: a rule not in the notes is looked up in the decompile
  (briefly) or made a VM fixture before it is built.
- Scenarios first, in `app/cosmic/src/scenario_tests/table_data.rs`,
  written by a lower-cost agent from the run's `UI_SPEC.md` section and
  reviewed; they pass before the physical check.
- Tests never open real dialogs (`Runtime::click_unrun`) and never touch
  the user's settings.
- **Text that moves is measured.** Showing numbers through the style
  changes text widths in the corpora: the round-trip audit and
  `layout_comparison_report` run at the start and the exit and after
  TD.5; every changed row is explained.
- Routine suites, Clippy, fmt, corpus measures, and decompile sweeps go
  through lower-cost subagents that report back.
- Aggregate numbers only; never the corpora's file names or contents.
  The decompiler's output stays outside Git.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on. A question found during the run takes
Ami Pro's behaviour, is recorded (TNK…), and goes to the handoff. VM
sessions are batched (fixtures TDV1–TDV8 in one or two sittings), the VM
is shut down afterwards. Hand off at a slice boundary if the run outgrows
a session.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| TD.1 Setup | `UI_SPEC.md` §41 (draft), this runbook and the progress file with the answers, the baseline (round-trip audit, layout comparison), the tag; the decompile pass on the open points that the code can answer: the compiler and its `%` (find the raiser of string 3125), the `[nfmt]` flag bits (`MODTAGPROC`), the display routine for a numeric cell, Quick Add's start rule (`1420:0fe6`), the 0x275 handler, reference rewriting (`1420:04a8`), Sort (`1360:0000`) | Committed; `table-data-start`; notes updated |
| TD.2 VM fixtures | Only what TD.1 leaves open (notes §9: TDV1–TDV8), one property per file, in one or two VM sittings | Results in the notes; the VM shut down |
| TD.3 Model and SAM | `NumberFormat` in the style (the six values, the flag word as mapped), kept in native v3 and written back as read; `TableCell::formula` (text as typed) with the second body read and written as Ami Pro does; a cell's numeric value derived from its text (the scanner of `1400:0ab6`); validation | Unit tests; native round trip; the three corpora's round-trip audit equal to the start |
| TD.4 Formula engine | In the core (no UI): the lexer and parser (A1, `$`, `..` and `:`, `@`/`=` prefix, `+ - * / %`, parentheses, unary minus, `@sum`); evaluation on doubles; REF, divide by zero, loops; dependants in order; address shifting for copy, insert, delete | Unit tests; **every corpus formula recomputed equals Ami Pro's stored result** (the audit, 291 cells) wherever its inputs are numeric; fixtures TDV3 |
| TD.5 Number formats | The format function (General, Fixed, Currency, Percent; decimal and thousands characters; the negative forms; red as a display colour) and the parse; cell text shown through the style (screen and PDF), the stored `%.*f` text unchanged; typing a number: right alignment, reformatted when the caret leaves, leading zeros and text cells as Ami Pro's rules | Layout comparison against the TD.1 baseline (changed rows explained); the corpus's numeric cells show as Ami Pro's stored heights imply; fixtures TDV1, TDV2 |
| TD.6 Commands and recalculation | `Command::SetCellFormula` (Edit Formula's OK; Quick Add Row and Column through the same command), Del on a formula, recalculation of dependants inside every command that changes a cell, relative shifting on Paste and Cut/Paste, address rewriting on Insert and Delete Column/Row, refusals (3113 and the like); SAM written after edits | `crates/editing/tests/formulas.rs` (one undo step each); `sam_after_edits` for formulas |
| TD.7 Interface | As decided (TN1, TN6, TN9): Table menu items (Edit Formula…, Quick Add ▸ Row and Column), the formula editor, the formula cell's box, double-click, cell-address headings (View Preferences), the style's Table format section, errors in the status bar, REF shown | Message-path and widget-path tests; screenshots light and dark |
| TD.8 Sort (if TN3) | Tools › Sort… as decided: three levels, Alphanumeric or Numeric, Word, order, delimiter, record size; paragraphs and table rows; refusals as Ami Pro's; `Command::SortParagraphs`; one undo step | Core tests with `SORTTAB` weights; fixtures TDV8; scenario |
| TD.9 Merge numbers | A table data file's numeric cells merge as Ami Pro shows them (notes §7); the AMT3 twin test extended | Test against AMT3 and TDV7 |
| TD.10 Forms | The recipe of ch. 16 walked end to end as a scenario; any gap found (TN7) | Scenario |
| TD.11 Extras (if TN4) | The extra functions, native only, listed by Save As | Tests |
| TD.12 Measures and twins | Round trip and layout comparison against TD.1; typing in a cell with 50 dependants (release A/B); twins: a table with formulas, Quick Add, and number formats written by the app, opened in Ami Pro with the results Ami Pro recomputes equal to the app's | Numbers here; VM screenshots outside Git |
| TD.13 Exit | `tools/verify.sh --release --corpus`; the records (progress, `UI_SPEC.md` §41, `FEATURE_PARITY.md` TB-01, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, the playbook's row and `TABLES.md`, one line in `PROJECT_STATUS.md` and `PROJECT_PLAN.md`, `CLAUDE.md` if its state changes, the Not implemented list: formulas, Quick Add, forms); the handoff; the push | Green; `origin/master` equals `HEAD` |

## 6. Scenarios (`scenario_tests/table_data.rs`; one checker after every step)

1. An invoice: type quantities and prices, a product formula per row, a
   `@sum` for the total; change a price; every dependent changes in the
   same step; Undo returns all of them in one step; Save As SAM, reopen,
   the results equal.
2. Quick Add Column and Row on a table with a heading row and a blank
   cell; the written formula; edit it; Del removes it.
3. Typing numbers under General, Fixed Decimal, Currency, Percent
   (negatives, red, parentheses); the cell right-aligned and reformatted
   on leaving; a space makes it text and the sum shows REF.
4. Changing the Table Text style's number format changes every table cell
   using it; Revert; one undo step; the file keeps `[nfmt]` as Ami Pro
   writes it.
5. Divide by zero and a loop: the message in the status bar, no crash, the
   rest of the table still calculates; fixing the cause clears it.
6. Copy a formula cell right and down; absolute addresses stay; Insert Row
   above a range and Delete Column inside one rewrite the formulas.
7. A formula before its data: empty cells count as 0 and fill in.
8. A formula in a table in a frame and in a page table across pages.
9. Sort refuses a range with a formula cell; sorts one without (and the
   rest of Tools › Sort… if TN3).
10. Merge a letter from a table data file whose cells are numbers
    (AMT3's values): `75.50`, `$1,200.00`, `150` as Ami Pro merged them.
11. The form recipe: protect labels, Honor protection, the caret skips
    them, Protected Text on the labels, print on a preprinted form leaves
    out labels and lines, Save as a style sheet With contents, File › New
    With contents.
12. Open an Ami Pro invoice with formulas (the install's sample): results
    unchanged, nothing marked modified, Save As lists nothing.

## 7. VM fixtures (only if TD.1 leaves them open)

TDV1 `[nfmt]` flag bits; TDV2 typed numbers per format (stored text and
display); TDV3 formula syntax and errors; TDV4 Quick Add's rule; TDV5
reference shifting; TDV6 recalculation on open; TDV7 merge of numbers in
the other formats; TDV8 Sort's details. Each is listed with the question
it answers in `TABLE_DATA_NOTES.md` §9. Written to `IN`, opened and saved
in Ami Pro; files and screenshots outside Git.

## Decisions to ask (before the run; recommendation first)

Ranked by how much each changes the build.

1. **TN1 Where formulas are edited.**
   (a) *Recommended:* in the rail. Table Properties gets a Formula section
   for the caret cell (the formula text typed where it is shown, the
   result beside it, Quick Add Row and Column buttons); Table › Edit
   Formula… and a double-click on a formula cell move focus to it; a box
   round a formula cell as Ami Pro. (b) A modal Edit Formula dialog as Ami
   Pro's. (c) a formula bar above the page, like a spreadsheet. (d)
   typing `=` at the start of a cell makes it a formula (not Ami Pro's;
   `=` would no longer be plain text).
2. **TN2 Numbers shown through the style.**
   (a) *Recommended:* as Ami Pro: a numeric cell shows through its
   paragraph style's number format on the screen and in PDF (General
   drops `.00`, Currency adds `$` and commas), the file keeps Ami Pro's
   `%.*f` text, and the caret in the cell shows the number only. Changes
   the width of about 1,750 corpus cells; gated by the layout comparison.
   (b) Show the stored text (as today) and format only newly typed or
   calculated numbers. (c) (a) with an Off switch in Preferences.
3. **TN3 Tools › Sort… in this run.** (a) *Recommended:* yes, Ami Pro's
   dialog's options as a Sort panel in the rail (three levels, type,
   word, order, delimiter, record size) for paragraphs and table rows;
   the existing Table › Sort Ascending/Descending stay as shortcuts. (b)
   Table rows only. (c) Leave out (as the speed-up row reads), keep the
   column sort.
4. **TN4 Functions.** (a) *Recommended:* Ami Pro's set only in SAM files
   (`+ - * / %`, parentheses, `@sum`), plus `@avg`, `@min`, `@max`,
   `@count` as native-only extras that Save As SAM lists (the text is
   kept, the results as text; Ami Pro's reaction to an unknown function is
   to be seen first, TDV3). (b) Ami Pro's set only. (c) A larger set
   (`round`, `abs`, `if`).
5. **TN5 Recalculation timing.** (a) *Recommended:* as Ami Pro: when the
   cell is left or the command is applied (never per keystroke), automatic
   for all dependants, none on open (stored results are kept byte for
   byte, TDV6 permitting); a Recalculate Table command for the doubtful
   case. (b) live as you type. (c) always on open.
6. **TN6 Where the number format is edited.** (a) *Recommended:* the style
   (as Ami Pro): a "Table format" section in the Styles rail's style
   properties (Cell format, decimals, symbols, negatives, currency), with
   the Table Properties cell section linking to it ("Number format comes
   from Table Text — Edit style"). (b) Also a per-column or per-cell
   override (native only; SAM cannot hold it, Save As lists it).
7. **TN7 Forms.** (a) *Recommended:* no new feature; the recipe is already
   built, the run adds the scenario (and fixes what it finds). (b) Add a
   "Lock labels" one-click that protects the selected cells and turns
   Honor protection on, and a Form mode that fills in cells with Tab
   (Honor protection is on already). (c) Fill-in form fields beyond tables
   (outside Ami Pro's model; not recommended).
8. **TN8 Errors and messages.** (a) *Recommended:* Ami Pro's results (REF
   in the cell, the stored number kept) with the three messages (cannot
   understand, divide by zero, circular) in the status bar and a mark on
   the cell instead of modal boxes. (b) Modal message boxes as Ami Pro.
9. **TN9 Cell addresses.** (a) *Recommended:* View › View Preferences ›
   Table row/column headings as Ami Pro (letters and numbers in the table
   margins), off by default, and the caret cell's address in the status
   bar while in a table. (b) Without the headings.
10. **TN10 Merge of numbers.** (a) *Recommended:* as Ami Pro (a numeric
    cell of a table data file merges as the style shows it, AMT3) once
    TD.5 has the display; (b) keep merging the stored text.
11. **TN11 Pace.** (a) *Recommended:* one straight run, no stop, VM
    sittings batched in TD.2; (b) stop after TD.5 (layout moves) for a
    look at the numbers.
