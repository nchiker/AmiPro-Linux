# Table data in Ami Pro 3.1: evidence for run C (2026-11-06)

For `docs/TABLE_DATA_RUNBOOK.md` (TD.1 to TD.10). Prepared before the run
by a read-only study: the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`
(routines by segment:offset; no decompiler output is kept in Git), their
dialog and string resources (`tools/ne_resources.py`), the user's guide
chapters 7, 16, and 21 (the ISO's online documents, `AP3CH07/16/21.SAM`,
read as text), both corpora, and the earlier notes (`TABLE_EDIT_NOTES.md`,
`TOC_INDEX_NOTES.md` "Numeric table cells", `MERGE_NOTES.md` §13). Nothing
was run in the VM. Levels: **D** decompile, **C** corpus (aggregate counts
only), **H** the user's guide, **R** resources, **VM** an earlier fixture,
**?** unknown. Confidence high, medium, low. Section 9 lists what is left
open and the VM fixtures that would settle it.

## 0. What the area is (the inventory)

| Item | Ami Pro 3.1 | Evidence | In the app today |
|---|---|---|---|
| **Table › Edit Formula…** (0x275) | Dialog 1138 "Edit Formula" (`TBLEDITFORMULAPROC` `13b0:0000`): one edit "Formula:" (id 25300), a group "Operators" showing `+  -  *  /  %  @sum(A1:A5)`, OK and Cancel. Also a double-click on a formula cell (H). A box is drawn round a formula cell while the caret is in it; Del deletes the formula (H) | R, D, H | Not built; the formula is kept as a cell's legacy data |
| **Table › Quick Add ▸ Row, Column** (0x283, 0x284; the popup item 0x276, string 2291 "Quick &Add") | One routine for all three, `1420:0dc5`; writes `@sum(first..last)` into the caret cell | D, H | Not built |
| **Style › Modify Style › Table format** (dialog 1028, page "Table format") | "Cell format" list: General, Fixed Decimal, Currency, Percent (strings 2118–2121); Numeric Format: No. decimal places, Decimal symbol, Thousands separator (check + character); Negative format: Leading minus, Trailing minus, Red, Parentheses; Currency format: Symbol, Leading, Trailing. A property of the paragraph style (usually Table Text), so of every table in the document using the style (H) | R, H | `[nfmt]` kept as six strings in the style (`kept.number_format`); not shown, not edited, not applied |
| **Numbers typed in cells** | A cell whose text is only a number is numeric: right-aligned (`<+A>`), reformatted by the style when the caret leaves; a space, letter, comma, or Enter makes it text, which Quick Add and formulas then read as REF/incorrect (H); leading zeros are lost (H) | H, D, VM (TNUM1) | Shown as stored (`5.00`) |
| **Cell addresses**: letters for columns, numbers for rows; View › View Preferences › "Table row/column headings" shows them (H) | | H | Not built (no such preference) |
| **Tools › Sort…** (0x1c8) | Dialog 1077 (`DLGSORTPROC` `13c8:0000` in `AMIPROUI.DLL`; command `1360:0000`): Field/Type/Word for three levels (field number; Alphanumeric or Numeric; 1st, 2nd, or All words), Sort Order (Ascending, Descending), Delimiter (Tab delimited, or a Field delimiter character), "No. paras in record". In a table the columns are fields and rows records, whole rows move; cells with formulas stop the sort (messages 3113 "Formula cell detected in cell %s. Cannot sort across formula cells", 3184 connected cells, 3186 DDE escapes, 3112 page break or ruler in the selection, 3193 tables with footnotes, 3215 too large). The playbook put Sort in this run (`TABLES.md`, Run 3); the speed-up row C does not name it | R, H | Only Table › Sort Ascending/Descending by the caret's column (MR1, `tables::sort_order`: numbers by value, else lower-cased text); no dialog, no paragraphs |
| **Forms** | Not a feature: a recipe in ch. 16 (H). A table (in the text or a frame) sized to the form; label cells protected (Table › Protect Cells) with Honor protection on; Edit › Mark Text › Protected Text on the labels; Print › Options › On preprinted form leaves out protected text and the table's lines and colour; save the form as a style sheet With contents, then File › New with contents. No command of its own | H | **All parts built in earlier runs** (protect and Honor: table editing run; Protected Text: run A, CA.7; On preprinted form: print run; With contents: run A, CA.4). Run C adds only a scenario that walks the recipe |
| **Chart from table data**, **table as merge data file** | Run D (charts), merge run (done) | H | n/a |
| **Numbers in table cells, merged** | The open point of MERGE_NOTES §13 | VM (AMT3) | The app merges the cell's text as stored |

Menu state and the handler of 0x275 are not mapped: the command table dump
has 0x276 and 0x283 (and 0x284) at `1420:0dc5` but no 0x275, so Edit
Formula goes through another path (to find, TD.1). Enabling for the Table
menu is `1180:2153` (in a table).

## 1. The formula language

**Syntax (H, ch. 16).** Five operations: `+ - * / %`, parentheses to nest,
and `@sum(range)`. "Lotus 1-2-3 syntax": a formula may start with `@` or `=`
or just with the operation; a range is `A1..B5` or `A1:B5`; case does not
matter; spaces are allowed (the guide's examples have them). A cell address
is the column letter(s) then the row number; `$` before a part makes it
absolute: `$B$2:$B$5`. Columns run A to Z, AA onward (D, `1470:0f56`: 0 to
25 is A to Z, then two letters, 255 columns at most, so up to IV); rows
count from 1. Only a cell of the same table can be named (no sheet
syntax).

**Corpus use (C).** 291 formula cells: 224 in 48 of the user's 423 SAM files
(of 162 files that have tables) and 67 in 4 of the ISO corpus's files (of
84 with tables). The language used is small:
- products and differences of two cells (`D4*E4`): 132;
- `sum` of a range: 99 (both `@sum(A1..A9)` and `sum(A1:A9)` occur; the
  `@` form 58 times, a leading `=` 6 times, no prefix for the rest);
- a sum times a constant (`sum(B2:B5)*1.06`), sums plus cells, a cell times
  a constant (`D4*95`), differences, a ratio `(A/B)*100`, and one literal
  sum `(1870+240+150)*2`;
- never: `%`, `$` absolute addresses, any function but `sum`.
The formula text is kept as typed (mixed case, `=`, no spaces seen).

**Compiled form (D `1418:0432`, `0cd1`, `1442`).** Ami Pro compiles the text
to a postfix token string and evaluates it on a stack of doubles:
- token 1: a cell (row 16 bits, column 8 bits; 4 bytes); token 2: a range
  (7 bytes, the same two numbers twice); token 3: a literal (a double, 9
  bytes); token 4: unary minus; the operators are the characters `+ - * /`.
  No token for `%` appears in the evaluator or in the reference scanners:
  the compiler must turn `%` into something else (a literal `/100`?)
  (**?**, medium).
- A range sums along a row or a column only: the code walks the range when
  the rows are equal or the columns are equal; a rectangle of both adds
  nothing (low, D).
- A referenced cell that is empty or missing is 0; a cell that does not read
  as a number gives the status "REF" (`1418:12cc`): the formula's cell shows
  the text **REF** (string 1004) and its value is marked not-a-number
  (`DAT 1506`); further formulas reading it are REF too (H).
- **Errors:** divide by zero shows a message (3126 "There is a formula that
  tries to divide by zero.") unless the recalculation is a bulk one
  (flag `DAT 003d` bit 0x80 suppresses it); a loop shows 3127 "This field
  contains a circular formula." (`1418:1976` keeps a list of the cells being
  worked on; a cell met twice is a loop); a formula that does not parse
  shows 3125 "Cannot understand the formula in cell '%s'." (the raising
  routine is not located). What stays in the cell after a divide by zero or
  a loop is **?** (probably REF; the value cell is marked bad).

**Recalculation (D `1418:0840`, `171c`; H).** Automatic. Each cell has a
list of the cells that depend on it (`1418:0432` adds and removes the
links when a formula is set or cleared); when a cell changes, `1418:0840`
walks the dependants, evaluates each (`0cd1`), and if the new double
differs from the old, stores it (`1418:171c`: the number becomes the cell's
text through the style's decimals, the paragraph keeps the cell's
alignment, the cell is redrawn and re-measured), then recurses into that
cell's dependants. The guide: "Ami Pro automatically places the results of
the calculations into the formula cell after you enter the data", and a
formula may be made before its data. Whether a file is recalculated when
opened (a stale stored result) is **?** (VM TDV6).

**What changes when rows or columns change.**
- Copy and Paste or Cut and Paste of a formula cell shifts its relative
  addresses by the move; absolute ones stay (H). Drag and drop moves a
  single cell only when it holds a formula or picture (H).
- Insert and Delete Column/Row adjust every formula's addresses:
  `1420:0000` tests whether any formula refers to or past the place;
  `1420:04a8` rewrites them (called from the insert and delete workers
  `1438:1e1a`, `1430:1f60`). The rules for a deleted cell that a formula
  names (REF?) are **?**.
- Sort does not move formula cells and refuses when one is in the range
  (3113; H: "does not include any cells containing a formula").

## 2. How formulas are stored in SAM

**VERIFIED (C, VM, D `1428:07ce`, `1490:07c0`).** In `[tbl]` › `[data]`:
- The cell record's F3 has bit **`0x40`** (a formula follows; `0x800` is
  "re-measure pending" in memory only) and F8 is 1 (text follows). F3's
  `0x38` bits are the first paragraph's alignment as always (right is
  `0x10`: the corpus has F3 80 = `0x40 + 0x10`).
- The cell has **two bodies** (each ended by `>`): the first is the
  displayed result, the second the formula text. Both carry the paragraph's
  style prefix `@Style@` and the alignment tag:
  `@Amount@<+A>3250.00` then `@Amount@<+A>D4*E4`. The first body of a
  calculated cell is only a number; the app's reader and writer already
  keep the second body (`TableCell::legacy`, `structure.rs` "extra").
- **The result text is stored plain**: `%.*f` of the double with the
  style's decimal places (`1400:0a2f` formats; `-0.00` loses its minus),
  with the decimal symbol put in (`1400:0d98`), and **no thousands
  separator or currency symbol** (C: all 291 results are digits and a
  point, 43 of the user's results `9999.99`, 22 `999.99`…, also in cells
  whose style, "Table Dollar", has the Currency cell format). The display
  adds those (see 3). TNUM1 (VM): a typed `5` in a General cell is stored
  `5.00` with `<+A>`.
- Absolute `$` and ranges are stored as typed. Which spelling Ami Pro
  writes after Quick Add: `@sum(` first-cell `..` last-cell `)`, built by
  `1420:0bfe` from the strings "sum" (1002) and the address formatter, with
  `$` only if the flag bits ask (Quick Add passes none); the corpus's 46
  `@sum(X..Y)` are the Quick Add form (C, medium).
- No other formula storage exists: no tag in the style, no `[…]` section.
  Formula cells the app does not understand are kept byte for byte today.

## 3. Number formats (the style's `[nfmt]`)

**In the file (C, D).** After `[nfmt]` come six values, then the next style's
name: (1) a flag word; (2) the **cell format**: 1 General, 2 Fixed Decimal,
3 Currency, 4 Percent (the corpus: Table Dollar has 3, Percent 4, Table
Text 1; **I**, high); (3) the decimal places; (4) the decimal symbol; (5)
the thousands separator character; (6) the currency symbol. The user's
corpus has 26 distinct tuples (2,421 styles read `272 1 2 . , $`, then
`280 1 2`, `280 1 3`, `272 1 2 . , ` with no symbol, `280 3 2`, `328 1 2`,
`16 1 2`, `280 3 0`); the flag words seen are `0x10`, `0x18`, `0x108`,
`0x110`, `0x118`, `0x128`, `0x148`, `0x158`. The dialog's boxes are
Leading minus, Trailing minus, Red, Parentheses, Thousands separator,
Currency Leading, Trailing, so the word holds about seven bits; **which bit
is which is open (?)**: `MODTAGPROC` (the Modify Style procedure, dialog
1028, controls 25565–25569, 25472, 25473) or fixtures TDV1.

**The display (D, H, VM).** The number is kept as a double in the cell
(`13c8:0069` reads it, `13c8:0000` sets it); the cell's text paragraph holds
the `%.*f` form; the screen and the printer show it **through the style**
when the caret is not in the cell ("While the insertion point is in the
cell, only the number displays", H). General "drops trailing zeros"
(TOC_INDEX_NOTES; AMT3: `75.50` merges as `75.5`); Fixed Decimal shows the
decimals; Currency adds the symbol (leading or trailing) and Percent a
`%` (H: "Displays numbers with a percent symbol"; whether the value is
multiplied by 100 is **?**). Thousands separator on or off; negatives
leading, trailing, in parentheses, in red. How the display text is built
(the routine between the stored number and the drawn string) is not
located yet; the draw path for cell text is in the `13e0`/`13c8` and
`1408` segments (to find, TD.1).

**What reads as a number (D `1400:0ab6`).** A scanner takes the cell's text
(skipping control escapes such as the protected-text mark `ESC V S`) and
accepts only digits, `+`, `-`, `.` and the style's decimal symbol; any other
character makes the cell text. **Corrected in TD.1 (section 10.3):** the
thousands separator and the currency symbol are not accepted, so `$1,200.00`
typed in a cell is text. A typed space makes the cell text, which is the
documented way to keep leading zeros (H).

**Corpus (C).** 1,298 numeric-looking cells in the user's files and 452 in
the ISO's (TOC_INDEX_NOTES), 553 of the user's with two decimals; the
app shows them as stored, so a General cell holding `5.00` shows `5.00` here
and `5` in Ami Pro. **This changes text widths in the corpus: it is the one
part of the run that moves layout** (gate: the layout comparison).

## 4. Quick Add

**D `1420:0dc5` (shared by 0x276, 0x283, 0x284); H.**
- Needs the table's cell-selection mode word bit `0x80` to be the current
  state (the caret in a table) and a caret cell with a row above (Column)
  or a column to the left (Row); otherwise it does nothing (returns -1/-6).
- **Row (0x283):** the range is on the caret's row, ending at the cell left
  of the caret; **Column (0x284):** on the caret's column, ending at the
  cell above it. The start is found by `1420:0fe6`, a walk from that last
  cell back towards the table's edge (to column/row 0), stopping at a cell
  that fails a test about its content (covered by a joined block, or having
  text of its own; the exact test and whether the stopper itself is
  included are **?**: the guide only says text cells give "incorrect
  results"). TDV4 settles it.
- The text written is `@sum(` start `..` end `)` (see 2), set as the caret
  cell's formula by `1410:15c9` (the same entry Edit Formula's OK uses), so
  the cell gets the dependency links and the result text, and "can be
  edited the same way as other formula cells" (H).
- What Quick Add does when the caret cell already has a formula, or the
  range is a single cell, is **?**.

## 5. Sort (Tools › Sort…)

**R, H, D partial (`1360:0000` and neighbours; dialog 1077).**
- Up to three levels; per level a field number (a table's column), Type
  Alphanumeric or Numeric, Word 1st, 2nd, or All; Sort Order Ascending or
  Descending (one for all levels); Delimiter: Tab delimited, or a Field
  delimiter character; "No. paras in record" for records of several
  paragraphs (H).
- Alphanumeric (H): character by character, numbers before letters,
  symbols in ANSI order; a table's rows sort whole; heading rows? (**?**).
  Numeric (H): by value, text without numbers at the end in alphabetical
  order. Ami Pro's alphabetic order is the `SORTTAB` weight table the index
  already uses (`TOC_INDEX_NOTES.md`, `sort_weight`).
- Ami Pro cannot undo a sort (H); the app undoes it (one step).
- Refusals and questions: strings 3112, 3113, 3184, 3186, 3193, 3215 above;
  "Do you want to sort the entire stream? If not, click on Cancel." (2042,
  with no selection in text).
- Data model: sorting the text of a SAM is moving paragraphs, so the core
  needs `Command::SortParagraphs` (and rows, which exists as
  `sort_rows`, extended to the dialog's options, several keys, words, and
  a record of several rows).

## 6. Forms (scope found)

The guide's "forms" (ch. 16) is a recipe over commands that exist. Corpus
(C): Honor protection is on in 18 of the user's 389 tables (8 files) and in
none of the ISO's 214; 775 protected cells in 10 user files and 1 in the
ISO's. Open: whether anything about forms is missing — Tab skipping
protected cells under Honor protection is done (table editing run); print
without labels is done (print run, PPT1F/PACLF); "Save as style sheet With
contents" is done (run A). The run's work on forms is therefore a scenario
and nothing else, unless the user wants an improvement (decision TN7).

## 7. Numbers in table cells, merged (MERGE_NOTES §13)

AMT3 (VM): a letter merging AMOUNT from a table data file shows `75.50` as
`75.5`, `$1,200.00` as `$1,200`, `150` as `150` — Ami Pro reads a cell that
reads as a number (3) as a number and writes it as the General display
would, not as stored. From a delimited data file the text merges as typed.
The rule for the app: merge a table data file's cell through the same
function as the cell's display (once TD.5 has it). Open: what the other
formats merge (Currency, Fixed, Percent), how `$1,200` kept its `$` and
comma under General (the scanner accepts them; the display apparently does
not drop them), and whether the table's style or the letter's field
decides. TDV7 (extend AMT3) settles it.

## 8. Where the work goes in the code (the app today)

- `crates/document/src/frame.rs`: `TableCell { content, span, frame, format, legacy }`
  — a formula and its result text live in `legacy` and the cell's flow.
  Run C needs `TableCell::formula` (the typed text; a parsed form derived)
  and a cell value derived from the flow.
- `crates/ami-formats`: `structure.rs` (cell `extra` bodies), `frames.rs`
  (cell records), `writer.rs` (`[nfmt]` as kept strings),
  `lib.rs` (`number_format`). The style model needs a `NumberFormat` with
  the six values (the flag word as mapped).
- `crates/editing/src/tables.rs`: `sort_order`, `sort_rows`,
  `merge::conditions::ami_pro_number` (a number parser to unify with 3).
- App: `command.rs` (Table menu has no Edit Formula or Quick Add),
  `table_*.rs` (Table Properties), `style_properties.rs` (Styles rail),
  `merge*.rs`, `page_view.rs` (cell drawing), `View Preferences`.

## 9. Open points and the VM fixtures that would settle them

Decompile first (TD.1), the VM only for what the code does not show.

| # | Question | Source to try first | Fixture if still open |
|---|---|---|---|
| 1 | The `[nfmt]` flag word's bits; each option's stored form | `MODTAGPROC` (controls 25565–25569, 25472–25473) | **TDV1**: Table Text with one option changed per file |
| 2 | Display text per format (General, Fixed, Currency, Percent; negatives; red; parentheses; thousands) and the stored text | the cell draw path | **TDV2**: a column of typed `1234.5`, `-7.25`, `0.05`, `007`, `$1,200.00`, `5%`, ` 5`, under four styles; saved file and screenshot |
| 3 | Typing rules: when the cell is reformatted, `%` typed in a number, an empty cell, Enter | `1408:00ea` (typing in a table), `1400:0ab6` callers | covered by TDV2 |
| 4 | `%` in a formula; precedence and associativity; unary minus; spaces; case; `=` and `@` prefixes (kept or normalised on OK) | the compiler (find via 3125's raiser; `1410:0000`) | **TDV3**: formulas `2+3*4`, `(2+3)*4`, `10-4-3`, `-A1`, `A1%`, `A1*B1%`, `8/0`, `=A1`, `@SUM(A1:A3)`, a loop, a text-cell reference, a 2-D range, a forward reference |
| 5 | The cell after an error (divide by zero, loop, REF, unparsable): stored text | `1418:171c`, `0cd1` | covered by TDV3 |
| 6 | Quick Add's start rule and its odd cases | `1420:0fe6` | **TDV4**: column with a header, a blank, a joined block, existing formula; Row and Column |
| 7 | Reference shifting on Paste/Drag, Insert/Delete Column/Row (including a deleted referenced cell) | `1420:04a8`, `1478`/`1438` callers | **TDV5**: copy B1 to C1; insert a row above a range; delete a referenced row |
| 8 | Recalculation on open of a stale file | the open path calling `1418:0840` | **TDV6**: edit a stored result by hand, open, save |
| 9 | Merge of numbers in the other formats | the merge insert path | **TDV7** (extends AMT3) |
| 10 | Sort details: heading rows, "All" words, ties, numeric with text, field delimiter for paragraphs, record size | `1360:0000`.. | **TDV8**: 6 small files (table, paragraphs by tab, by delimiter, records of 2) |
| 11 | Handler of 0x275 and the menu enabling; what a double-click does; the box round a formula cell | cmd table, `1180:2153` | none |

Fixtures are small, synthetic, written to `IN`, opened and saved in Ami
Pro; files and screenshots stay outside Git. Work in batches (one session
of the VM; shut it down afterwards; memory is tight).

## 10. TD.1 decompile pass (2026-11-06; AMIPRO.EXE and AMIPROUI.DLL, no VM)

Evidence is cited as segment:offset of the routine only. Confidence: high,
medium, low. "Fixture" names the TDV item that still needs the VM.

### 10.1 The formula compiler (D `1410:0000`, `083a`, `119f`, `1019`, `13ef`, `18e3`, `12b1`, `12eb`, `1328`)

- **Where 3125 is raised.** In the setter `1410:0000`, when the compiler
  `1410:083a` returns "no" (not "REF"): the message is shown with the
  cell's address ("Cannot understand the formula in cell 'C4'."), the
  dependency links of that cell are removed again and the cell is not a
  formula. The caller `1410:15c9` (the one entry for Edit Formula's OK and
  Quick Add) restores the cell's old state. High.
- **Compiler.** A shunting-yard to postfix. A leading `=` is skipped (a `@`
  is part of a function name only). The text is split into words by
  `119f`: letters, digits, `%`, `$`, `.`, `@` and the decimal symbol make a
  word; a space inside the formula before a word is skipped and a space
  inside a word is ignored; any other character that is not an operator is
  an error. Operators: `+ - * /`, `(`, `)`, and an internal unary-minus
  token. High.
- **Precedence and association.** `* /` bind tighter than `+ -` (2 above
  1), unary minus binds tighter than both (3) and a unary `+` is dropped;
  the operator stack pops while the stacked priority is greater than or
  equal to the incoming one, so `10-4-3` and `a/b/c` are left to right.
  `(` has priority 0 on the stack. High.
- **Unary signs.** One sign (`-` or `+`) is allowed at the start of the
  formula and after any operator or `(`; two in a row (`--3`, `3*-+2`) are
  not understood. Implicit multiplication (`2(3)`, `A1(B1)`) is not
  understood (an operand followed by `(` is refused). Medium: `(-3)` after
  an opening parenthesis passes through a path that treats the sign by the
  count of operators since the last operand; fixture TDV3 confirms `(-A1)`.
- **Numbers.** Digits with at most one `.` or decimal symbol; no exponent,
  no thousands separators, no leading `+`. A trailing single `%` on a number
  multiplies it by 0.01 when parsed (`25%` is 0.25). A number that converts
  to 0 must really be zeros (checked by `1400:09f0`), else it is an error.
  High.
- **`%` on a cell.** `A1%` is accepted: the compiler emits the cell, pops
  every stacked operator of priority 2 or more, then pushes a division by
  the constant 100 (so `B1*A1%` is `(B1*A1)/100`, `10+A1%` is `10+A1/100`).
  There is no modulo and no postfix `%` on a bracket or a function result.
  High (from the code), the stored result's rounding is Ami Pro's double
  arithmetic.
- **Cell names (`18e3`).** Case-insensitive. Optional `$`, one or two
  letters, optional `$`, one to six digits, optional trailing `%`. The
  column must exist in the table (index below its column count) and the row
  must be 1 to the table's row count; a name outside the table is not
  understood (so no sheet or other-table references). The word REF (string
  1004) is accepted anywhere a cell name is: it makes the formula's result
  REF without a message. Two points are not settled by the code and go to
  TDV3: the decompile can be read as refusing an absolute row (`B$2`), and
  the stored `$B$2` form must be checked. Medium.
- **Function.** Only one: the name is string 1002 (`sum`), compared after
  lower-casing, with an optional `@` in front, so `@SUM(`, `Sum(` and `sum(`
  all work. After the name: spaces, `(`, a first cell name, then `:` or `.`
  (any number of them, so `A1..B5`, `A1:B5`, `A1.B5`), a second cell name
  and `)`. A single-cell argument (`sum(A1)`), a range that is neither in
  one row nor in one column (a rectangle), more than one argument, and
  anything after `)` that is not an operator are not understood. Any other
  word followed by `(` (`avg(`, `max(`) is parsed as a cell name and fails,
  so an unknown function is "cannot understand" (3125). High.
- **Evaluation (`1418:0cd1`).** A stack of doubles; `+ - * /` and unary
  minus; a range adds the cells along its row or column; an empty or
  missing cell is 0; a cell marked "not a number" (a text cell, an error,
  REF) makes the dependant not a number. Division by zero: message 3126
  unless the bulk flag (`DAT 003d` bit 0x80) is set; the evaluation then
  fails and the cell gets the "no value" state. Loop: `1418:12cc` asks
  `1418:1442`, which keeps the list of cells being worked on; a repeat shows
  3127 (silent when `1418:1442` answers 2). A formula cell whose result is
  not a number keeps its formula text; what its body shows (the string REF
  or blank) is not in the code I could follow: fixture TDV3, one
  screenshot. Medium.
- **Result text.** On success `1418:171c` turns the double into text by
  `%.*f` with the style's decimals (`1400:0a2f`; a result that prints as
  `-0.00` loses the minus), replaces the decimal point by the style's
  decimal symbol (`1400:0d98`), keeps the cell's alignment (right, `0x10`,
  when none was set), and redraws. High.

### 10.2 The `[nfmt]` flag word and the style record (D `AMIPROUI` Modify Style table page; style reader `20587`, `1428` area; `1448:463b`, `4773`)

The style record in memory holds at +0x8d the flag word, +0x8f the cell
format byte, +0x90 the decimals, +0x91 the decimal symbol, +0x93 the
thousands character, +0x95 the currency symbol (a string). The word, bit
by bit (the dialog's check boxes, each control read against the dialog
template; high):

| Bit | Meaning |
|---|---|
| 0x08 | Thousands separator on |
| 0x10 | Negative: Leading minus |
| 0x20 | Negative: Trailing minus (exclusive with 0x10) |
| 0x40 | Negative: Parentheses |
| 0x80 | Negative: Red |
| 0x100 | Currency symbol Leading (clear: Trailing) |

The cell format byte: 1 General, 2 Fixed Decimal, 3 Currency, 4 Percent.
The defaults of a new style: word 0x18 (thousands, leading minus), General,
2 decimals, the system decimal and thousands characters, the system
currency symbol, and bit 0x100 when the system's currency format puts the
symbol first. Leading minus and Trailing minus are exclusive in the dialog;
Parentheses and Red are independent. Reading an old-version style clears
the low three bits. The corpus words read this way: `0x118` = thousands,
leading minus, leading symbol (Table Dollar); `0x110` = leading minus,
leading symbol, no thousands; `0x10`/`0x18` = leading minus, symbol after;
`0x128` trailing minus; `0x148` parentheses; `0x158` parentheses with a
leading minus. The decimal symbol and thousands character are each a
character plus a second byte (written as the second number in the
section; kept as it is).

### 10.3 What reads as a number, and the display (D `1400:0ab6`, `095c`, `06bb`, `0448`, `0a2f`, `0d98`; `1448:463b`, `46f2`, `4773`, `49bf`)

- **The scanner** (`0ab6`) copies the cell text, skipping control escapes,
  and accepts digits, `+`, `-`, `.` and the style's decimal symbol and the
  system decimal symbol. Any other character (a space, a letter, a comma,
  `%`, `$`) makes the cell text; the protected-text mark `ESC V S` does too.
  The validator (`095c`) then requires: a sign only first and with at least
  one more character, at most one decimal mark, only digits otherwise. The
  number is read by `atof` after the decimal mark is turned into `.`; an
  all-zero text counts as 0 (`09f0`). A cell text is numeric only if it is
  one to 255 characters. High. This means a comma or a currency sign typed
  in makes the cell text (the guide says the same for a comma).
- **Formatting a typed number.** When the caret leaves a numeric cell
  `1400:0448` rewrites its text with `%.*f` at the style's decimals and the
  decimal symbol, and sets alignment right (`0x10`) if no other
  alignment is set. The stored text never has a thousands separator,
  currency symbol, parentheses or percent sign (the result and typed
  forms are identical). High.
- **The screen.** The code I followed does not build the decorated text
  (`1,234.50`, `$1,234.50`, `(7.25)`): the cell layout `1448:00cb` decides
  per cell whether it needs an adjustment (`463b`: the cell is numeric,
  has no tab, and either the value is negative with Trailing minus or
  Parentheses, or the format is Currency with a symbol shown after the
  number), and then reserves room at the right edge (`49bf`: the width of
  the trailing characters) and shifts the number left. That reserved room
  is where the trailing minus, the closing parenthesis, or the trailing
  currency symbol is drawn. The general rule for drawing thousands
  separators, a leading `$`, Percent (no multiplication by 100 appears in
  any routine; the Percent value is shown as stored and with a `%`), and
  red is not in the routines read. Low: TDV2 settles it with one
  screenshot per format.
- **Zero decimals.** If the style has no decimal symbol the formatter uses
  zero decimals (`1400:0d98`, `0448`); a style with the symbol blank is
  therefore integer.

### 10.4 Quick Add (D `1420:0dc5`, `0fe6`, `0bfe`, `1410:15c9`)

- **Which command is which.** 0x283 (Quick Add Row) sums the cells to the
  left of the caret cell along its row; 0x284 (Column) the cells above it.
  Enabled when the caret is in a table cell, the selection is the one
  cell, and (Row) the column is not the first or (Column) the row is not
  the first. 0x276 is the submenu's own entry, enabled by the same test as
  Row. High.
- **Start rule.** The range ends at the neighbour next to the caret and
  starts by walking back from it to the table's edge. Going back, a cell
  **stops** the walk if it holds real text that is not a number and has no
  formula (a label or heading); empty cells, numeric cells and formula
  cells are walked through (an empty cell with a style stays; a cell that is
  part of a connected block stops the walk too). The cell found is the one
  **after** the stopper (the stopper is not in the range); if the cell next
  to the caret is the stopper the range is that one cell; if no stopper
  exists the range starts at the first row or column. High (the stopper is
  excluded: the routine returns the previously examined cell).
- **What is written.** `@sum(first..last)` (the string 1002 for `sum`,
  addresses without `$`, upper-case column letters) into the **caret
  cell**, through the same setter as Edit Formula (`15c9`), so it replaces
  whatever the cell held and recalculates dependants. The result shows in
  that cell. An existing formula in the caret cell is replaced. High. A
  cell on the edge (no neighbour) does nothing. The corpus's 46
  `@sum(X..Y)` formulas are this form.

### 10.5 Edit Formula (0x275) and the table menu state (D `1180:2153`, `13b0:0000`)

- The command is enabled when a table has the caret, the caret cell exists,
  and the selection is the single cell (the same test as 0x273, 0x287, 0x288:
  the other cell-level commands). It does not need a formula to exist. High.
- The dialog "Edit Formula" (`TBLEDITFORMULAPROC`) is a one-line edit
  prefilled with the cell's stored formula text; OK sends the text to the
  setter (`1410:15c9`, via command 0x27d); Cancel closes. The caret
  cell's formula is edited as the typed string; it is not normalised
  (what the user typed is stored, minus a leading `=`? the setter strips
  one leading `=` while compiling but the stored text is the typed one:
  medium, TDV3 to check `=A1`).
- A double-click on a cell that has a formula sends the same command when
  the table cell mode is active (`27b` handler); a double-click on another
  cell does the ordinary thing. Medium.
- The box round a formula cell and Del's effect are not in this path
  (they are drawing and deletion code); the guide's statement stands.

### 10.6 Reference rewriting (D `1420:0000`, `024b`, `04a8`, `0ad3`, `0b6c`, `0bfe`)

One routine rewrites the **text** of every formula, token by token, and
stores it back; modes: 1 insert rows or columns, 2 delete, 3 copy or move
by a signed offset. A formula is touched only if a name in it lies at or
past the place of the change (`1420:0000` looks first). Names are written
back in canonical form (`$` kept where it was, column letters upper case);
other text keeps its form. High for the mechanism, medium for each case:
- **Insert** at row r (column c): every name whose row (column) is at or
  past r moves down (right) by the count. A range whose end moves stays
  stretched, so inserting inside a range lengthens it.
- **Delete**: a name past the band moves up; a name inside the band
  becomes the text REF; for a range, an end on the band's first row or
  column stays (the range shrinks), the last end inside the band moves to
  the row before the band; an end strictly inside the band is REF. The
  formula then calculates to REF when read (10.1).
- **Copy or move** (mode 3): relative parts shift by the offset, parts
  with `$` stay; a name that would leave the table (before row 1 or column
  A) becomes REF.
- The rewritten formulas are recorded in a list for the undo of the
  command (`024b`); the app has its own undo.
- Not seen in this routine: how a formula whose own cell moves or is
  deleted is kept (the cell moves with its text; deleting the formula's own
  row removes it). Not a question.

### 10.7 Tools › Sort (D `1360:0000`, `06fe`, `0e85`, `259c`, `1506`, `1662`, `1708`; `DLGSORTPROC` `13c8:0000`)

- **Settings (dialog 1077).** Three levels, each a field number 1 to 99, a
  Type (Alphanumeric or Numeric) and a Word (1st, 2nd, All; greyed for
  Numeric); one Sort Order; Delimiter (Tab delimited, or one character);
  "No. paras in record" 1 to 99. Delimiter is greyed out when the selection
  is a table. Level 2 and 3 are skipped when their field is empty or equal
  to level 1's with the same word. High.
- **Mode.** With the caret in a table and no text selected, the table is
  sorted (whole rows); with a text selection, paragraphs (the "sort the
  entire stream?" question, string 2042, when nothing is selected outside a
  table). Table: with a single-cell selection the **whole table** is
  sorted, otherwise the selected rows (all columns). A cell holding a
  formula stops the sort (3113, with the cell's address); a connected cell
  asks 3184 (Yes continues and the cells are un-connected); a table with
  footnotes is refused (3193); a selection that is too large is refused
  (3215). In text, a page or column break or a ruler in the selection asks
  3112 ("sort across it?"); a DDE escape asks 3186. High.
- **Algorithm.** A heap sort over records (`17f6` builds, `259c` sifts):
  it is **not stable**, so records with equal keys come out in an order
  Ami Pro does not define. The comparison goes level by level until one
  level differs. A field is a delimited piece of the record; a word is a
  piece of the field between spaces (a tab too, unless Tab delimited is
  chosen); only the chosen word is compared (All: the whole field).
  Alphanumeric uses the C-library byte comparison of the key text
  (`1150:013b`, or `019a` when bit 0x10 of the settings is set: not
  settable from the dialog; assumed the first); the numbers-before-letters
  rule is the ordinary byte order. Numeric: each key is reduced to its
  digits with the decimal point kept; the integer and fraction parts are
  padded with zeros to a common width and compared as text, a negative (a
  leading `-` or `(`) reverses; a key with no digit sorts after the numbers.
  Descending reverses the whole comparison. Medium (the byte order and the
  case rule are the doubtful part: fixture TDV8, one file).
- **Record size.** A record of n paragraphs moves as a unit and is sorted by
  its first paragraph. Medium.

### 10.8 Numbers in cells, merged (MERGE_NOTES section 13)

Nothing in the cell scanner or formatter explains AMT3: `$1,200.00` is
**text** in a cell (the scanner refuses the comma and the dollar), so the
merge of `75.50` as `75.5` and `$1,200.00` as `$1,200` comes from the
merge engine's own conversion of data-file fields, not from the table's
number display. Not settled by the code I read; TDV7 stays (one file, the
AMT3 cells plus `007`, `5%`, ` 5`, `-7.25`, under General and under
Currency).

### 10.9 Summary of the open points (section 9) after TD.1

| # | Answer | Fixture still needed |
|---|---|---|
| 1 | Bits mapped (10.2) | none for the bits; TDV1 dropped |
| 2 | Stored text rules and the scanner (10.3); decoration drawing not mapped | **TDV2** (one file, four styles, screenshots) |
| 3 | Typing rules (10.3) | covered by TDV2 |
| 4 | Syntax, precedence, `%`, prefixes, case, unknown function (10.1) | **TDV3**, narrowed to: `(-A1)`, `B$2`, `=A1` stored form, the text shown after divide by zero, a loop, a text-cell reference, Del on a formula cell |
| 5 | Error states (10.1) | covered by TDV3 |
| 6 | Quick Add (10.4) | none (a rectangle of 4 cases may be checked in TDV3's file) |
| 7 | Reference rewriting (10.6) | **TDV5**, narrowed to one file: delete a row inside a range, delete the row of a single reference, copy a formula with `$` |
| 8 | Recalculation on open | **TDV6**, one file (edit a result by hand) |
| 9 | Merge numbers | **TDV7** |
| 10 | Sort (10.7) | **TDV8**, one file: case, symbols, ties, numeric with text |
| 11 | 0x275 and enabling (10.5) | none |

## 12. TD.3 and TD.4 results (2026-11-06; model, SAM, formula engine, corpus audit; updated with TD.2)

**Built.** `amipro_document::number` (`NumberFormat` with the flag word's six
bits, `NumberKind`, `scan_number`, `stored_text`); `amipro_document::formula`
(compiler, evaluator, `calculate`, `dependants`, `rewrite`); `TableCell::formula`
(native record `cellformula`; SAM F3 bit `0x40` with the second body);
`LossKind::FormulaFunctions` (Save As lists `@avg @min @max @count`). The engine
sits in the document crate because editing, formats, and layout all depend on it
and it needs only a table's size and cell values.

**The second body (corpus, 261 + 15 + the VM folder's cells).** Its prefix is the
first body's: the style tag `@Name@` (none for Table Text) and the alignment tag
`<+A>`. **An untagged paragraph that starts with `@` writes it twice**
(`<+A>@@sum(A1:A4)`, VM folder), so that the text is not read as a style tag; a
tagged one does not (`@Table Dollar@<+A>@sum(...)`). The reader takes a second
body as the formula only when it is exactly that prefix, one line of ASCII with no
`<`; any other second body stays a legacy body (and keeps the record's `0x40`), so
nothing is dropped. The writer builds the prefix of a new formula from the result
body it just wrote.

**Audit (`crates/ami-formats/tests/formula_corpus.rs`, ignored).** User's corpus
417 files read, 53 with formulas, 261 cells (more than the 224 counted in the
preparation: that count missed subfolders); ISO corpus 160 files, 1 file, 15 cells
(the "67 in 4 files" of the preparation are the VM folder's). Every one of the 276
recomputes to Ami Pro's stored text: on the stored cells, as a whole-table chain
on doubles, and with each result rounded to its style first (`%.*f`, Rust's
formatting agrees with Ami Pro's on every stored value). No rounding mismatch
class exists in these corpora. The VM folder (synthetic fixtures, still growing)
has the error cases; see the list in the progress file.

**Applied from TD.2 (section 11).**
- The `(-` quirk is reproduced (Ami Pro's behaviour, one test each): a sign
  right after the `(` that opens the formula is dropped (`(-A1)` is +A1, `(-3)` is
  3); `2*(-A2)`, `-A1`, `-(A1)`, `((-A1))` keep it.
- Stored error forms: a divide by zero and a loop store F3 `0x50` and an empty
  result body (`<+A>` alone); REF (a text cell, a range holding one, the word
  `REF`) stores `0x2050` and is drawn REF. The writer sets bit `0x2000` from the
  engine's outcome (`Outcome::Ref`), clears it otherwise, and writes an empty
  result as `[@Style@]<+A>`, never a bare styled line. A divide-by-zero or loop
  cell reads as 0 (the loop's reader was not observed: unconfirmed). Reading never
  calculates: stored results stay (TDF6; test).
- Rewrite rules for rows match TDF3 (tests with the VM cases); the symmetric rule
  for Insert/Delete Column is **unconfirmed** (refused for lack of room), as is
  whether `$` stops an insert or delete (only names above the point were seen).

**Audit per class** (`formula_corpus`, after TD.2; the VM folder keeps growing).
- User's corpus: 261 cells, all "equal, number" (each formula alone on the stored
  cells, and the whole-table chain). ISO corpus: 15 of 15 the same.
- VM folder, 96 cells in 8 files. Equal: numbers 76 (chain), divide by zero 2,
  loop 2, REF 4 (empty result, bit set). Not equal, all explained: REF word with the
  old number left as the result and bit `0x2000` set, 6 (`REF`, `REF*2`: a deleted
  cell's formula keeps its stale result, as TDF3 stored it); stale hand-edited
  results, 4 (99 and 77 against 11 and 15, by design, TDF6); a loop's first-entered
  cell, 2 (Ami Pro stored 0.00 in the cell that was calculated before the loop
  closed; both loop cells compute as Loop, which only matters on a recalculation).

**Choices (TNK candidates).** TNK-a: a formula's text is kept as typed in
`TableCell::formula` and in SAM; names are canonicalised only when a rewrite
touches the formula. TNK-b: `$` on a row (`B$2`) is accepted. TNK-c: a range
`A1:A1` is accepted, `sum(A1)` is not. TNK-d: the extras skip empty cells, `@avg`,
`@min`, `@max` of no numbers are 0, `@count` counts numbers and never gives REF.
TNK-e: a reference to a formula cell that failed is REF. TNK-f: the delete rule
for a range follows the notes (10.6): the low end on the band's first line stays,
a low end inside the band beyond it is REF, the high end in the band moves to the
line before it; to be checked with TDV5. TNK-g: a number needs a digit (`.`, `-`
are text); a decimal mark of the style counts as well as `.`.

## 13. TD.5 results (2026-11-06; the display, typing, the Table format section)

**Built.** `NumberFormat::display` (`number.rs`; `Displayed { text, red }`) with the
VM table of section 11.1 as unit tests (`stored_text` now rounds a value exactly half
way up, TDT4: 1234.5 at no decimals is 1235). Layout (`crates/layout/src/engine.rs`):
a paragraph that is the only one of a table cell, with no objects or marks, whose
text the style's format reads as a number, is shaped as **one unbreakable word over
all the stored bytes** whose width is the shown text's (`ShapeCache::shaped_number`,
its own cache); the line's `WordBox` carries the shown text in `text` (as a page-number
field does) and `red`. Row heights, cell widths and the table layout see the shown
width; the stored text, positions, find, spell and the SAM are untouched.
Page surface: the word is a field segment (hits go to the start or end; a number's go
to the **end** of the stored text); when the selection is inside the cell's flow alone
the line is built **plain** (`LineKey::plain`, cache pages cleared when the editing
flow changes): the stored text shows, at the shown word's place by the paragraph's
alignment. Red is drawn as a colour of the span; PDF and print use the same drawing, so
they carry the shown text (test in `print.rs`).

**Alignment.** The notes (11.1) say a number is right-aligned only when the cell has
the alignment code (record flag 16 with `<+A>`; flag 0 draws it decorated and left
aligned), so the layout adds no alignment of its own: typing adds the code.

**Typing.** `DocumentSession::rewrite_number` (`main.rs`): when the caret leaves a cell
whose text it edited, a single-paragraph cell that the style's scanner reads as a
number (not a formula cell, not protected, not in a merge data file) gets `%.*f` at the
style's decimals and the paragraph's alignment Right (`<+A>`), as the VM TDT1-TDT4
stored it (007 to 7.00, -0 to 0.00, +5 to 5.00; `1,5`, `$1,200.00`, `5%`, ` 5` stay).
It is amended into the typing run's undo step when the leave is a plain caret move, else
its own step; the caret stays. SAM: the cell record gets F3 bit 0x10 when the cell holds
one right-aligned number (`frames.rs`, `right_aligned_number`; the corpus round trip
is unchanged).

**Style rail.** Style Properties has a **Table format** section (Cell format,
decimal places and symbol, Thousands and its character, Leading minus / Trailing minus
(exclusive), Parentheses, Red, currency symbol and Leading/Trailing, a sample), each
change one `ModifyStyle` step, stored as `[nfmt]` exactly as read (six values).

**Choices (TNK candidates).** TNK-TD5a: the display function is the document crate's
and the layout shapes the shown text as one word, so a number never wraps and a
number wider than its cell overflows to the right (Ami Pro clips; not seen).
TNK-TD5b: while the caret is in the cell the stored text shows (the runbook's rule),
done in the surface, not in the layout, so the layout is the same with or without
the caret. TNK-TD5c: merge data files keep the typed text (zip codes keep a leading
zero; Ami Pro would rewrite `02134` to `2134.00`). TNK-TD5d: a negative with no
negative mark set in the flags shows a leading minus. TNK-TD5e: only cells the caret
edited are rewritten; a visit, a Save, and a style change rewrite nothing (Ami Pro's
rewrite on a mere visit is not seen).

## 11. VM results (TD.2, 2026-11-06; Windows XP, Ami Pro 3.1)

Fixtures are synthetic SAM files written by a script (frame and style header
copied from an Ami Pro-written file, `[nfmt]` per style) or typed in Ami Pro;
inputs and outputs are in `../AmiPro-Reference/vm-lab/OUT/tabledata/`, the
screenshots in its `screens/` (none in Git). One property per file: TDA1 and
TDA2 (display per format), TDT1-TDT4 (typing under General, Currency,
Percent, General 0 decimals), TDF1 (formula entry and errors), TDF3 (row
delete, row insert, copy), TDF6 (stale results), AMD2B, AMD3B, AMD4B with
AML5-AML7 (merge), TDX1-TDX6 and TDY1 (Sort). Evidence level: VM (screens read,
stored files read).

### 11.1 TDV2: the display, per format (TDA1, TDA2; stored text `%.2f` in every case)

The screen shows the number through its paragraph style, whether the cell
is right-aligned (record flag 16) or not (flag 0: the same text, still
decorated, left-aligned); a stored `5` or `1234.5` is decorated on load too
(the display never needs the stored text to be normalised). Caret not in the cell.

- **General** (2 decimals, thousands on): trailing zeros dropped, thousands
  separators kept: `1,234.5`, `-7.25`, `0.05`, `1,234,567.89`, `0` (for `0.00`),
  `7` (for `007`), `12`. With 0 decimals the display rounds half up:
  `1,235` (from 1234.50), `-7`, `0`, `1,234,568`. With 3 decimals Fixed shows
  `1,234.500`.
- **Fixed Decimal**: the decimals always (`5.00`, `1,234.50`), thousands as set.
- **Currency**: the symbol is drawn, `$1,234.50` (leading), `1,234.50$` (trailing),
  zero `$0.00`. Negative, leading minus: `-$7.25`; trailing minus: `7.25-$`
  (the minus after the number, before the symbol); parentheses (bit 0x40
  without 0x10): `$(7.25)` and `$(1,234.50)`; with a leading minus as well
  (0x50): `-(7.25)%` for Percent, so the two marks add; red (0x80): the whole
  negative text in red (`-$7.25` red); positives are black. No-thousands
  (0x110): `$1234.50`.
- **Percent**: the stored value is not multiplied: `1,234.50%`, `5.00%`,
  `0.05%`, `-7.25%`; the `%` is drawn in the width reserved at the right edge
  (it clips at the edge when the cell is narrow, as the trailing symbol does).
- **Text cells** (anything with a space, `$`, `,`, `%`, letters: `$1,200.00`,
  `5%`, ` 5`) are drawn as typed, left-aligned, in every format.
- **Fixed with trailing minus and no thousands (0x20)**: `7.25-`, `1234.50`.

### 11.2 TDV2: typing (TDT1-TDT4; one column, Tab between cells)

On leaving the cell (Tab) a number is rewritten as `%.*f` with the style's
decimals, flag 16 (right) added, `<+A>` in the body. The stored text is
identical in General, Currency and Percent (decorations are display only).
General, 2 decimals: `1234.5`→`1234.50`, `-7.25`, `0.05`, `007`→`7.00`,
`1234567.891`→`1234567.89`, `0`→`0.00`, `.5`→`0.50`, `-0`→`0.00` (minus lost),
`+5`→`5.00`, `12`→`12.00`. Left as typed, flag 0, no reformat: `1,5`,
`$1,200.00`, `5%`, ` 5` (leading space), `5 ` (trailing space), `1e3`. With 0
decimals (TDT4): `1235` (1234.5 rounds half up, not to even), `-7`, `0`
(from 0.05), `7`, `1234568`, `1` (from 0.5), `12`, `5`. A tab-typed cell
in a style-carrying empty cell (a SAM cell whose body is only `@Style@`) is
two paragraphs on load and is not treated as a number: **generate empty cells
as absent records**, not as empty styled bodies.

### 11.3 TDV3: formulas (TDF1; entry through Table > Edit Formula…)

- **Stored text is as typed**: `=A1`, `A1 + A2` (spaces), `a1+a2` (lower
  case), `B$2`, `$B$2+1`, `sum(A1:A3)` (no `@`), `@sum(A1..A4)` (stored as
  `@@sum(...)`, the SAM escape of `@`), `REF`, `-(A1)`. The record F3 is `0x50`
  (formula + right), the first body the result, the second the text.
- **Accepted**: `B$2` and `$B$2` (absolute rows and columns), `=A1`, `sum(A1:A3)`
  without `@`, `A1%` (0.10 for A1=10) and `A2*A1%` (0.40), `10-4-3`=3, `2+3*4`=14,
  `-A1`, `2*(-A2)`=-8, `0-(-A2)`, `3*-A2`=-12 (a sign after an operator is
  accepted), the word `REF` (no message; shown REF).
- **Quirk: a formula that starts with `(-` loses the sign**: `(-A1)` gives
  +10 and `(-3)` gives +3 (A1=10), while `2*(-A2)` and `-(A1)` are right.
  Implement it (the unary minus right after a `(` that opens the formula is
  dropped) or flag it as TNK; the VM result is what Ami Pro stores.
- **Refused with a message and the cell not stored** (the cell stays empty):
  `foo(A1)` and `@sum(A1:B2)` (a rectangle) "Cannot understand the formula in
  cell 'B7'" (3125, the cell name in the text), `Z9` (outside the table, same
  message).
- **Divide by zero** (`A1/A4`, A4=0): the message "There is a formula that tries
  to divide by zero." (3126); the formula **is stored** (F3 `0x50`) with an
  **empty result** (`<+A>` alone); the cell shows blank (not REF). A formula
  that refers to it (`B5+1`) counts it as 0 and stores `1.00` (an empty result
  is not an error).
- **Loop** (`C1` = `C2`, then `C2` = `C1`): "This field contains a circular
  formula." (3127); the second formula is stored too (F3 `0x50`) with an empty
  result and the first stays calculated (0.00); no REF shown.
- **A reference to a text cell** (`A5+1`, A5=`abc`) and a range that holds one
  (`@sum(A1:A5)`): **no message**; the cell is stored with F3 **`0x2050`** (bit
  `0x2000` "not a number"), an empty result, and drawn as the word **REF**
  (left-aligned). A formula using the word `REF` (`REF*2`, `REF`) is stored
  with `0x2050` as well and a result text `1.00` (stale/odd, never shown).
- **Del** on a formula cell: the cell's record disappears from the file (the
  formula and the result go; the cell is empty and plain), with no question.
- **Display**: results are drawn through the style like any number (`10`,
  `-10`, `14`, `17`).

### 11.4 TDV5: reference rewriting (TDF3, saved after each step)

Start: A1..A6 = 1..6; formulas `@sum(A2..A5)`, `A3*2`, `A$1+$A2`,
`@sum(A1..A3)`, `@sum(A3..A6)`, `A2+A4`.
- **Delete row 3** (the row of A3): a range with A3 inside it shrinks
  (`A2..A5` becomes `A2..A4`); the end on the deleted row moves up
  (`A1..A3` becomes `A1..A2`); the start on the deleted row stays
  (`A3..A6` becomes `A3..A5`, the next row's old content); a single reference
  to the deleted cell becomes the word: `A3*2` is `REF*2`, stored F3
  `0x2050`, shown REF; names beyond move up (`A4` becomes `A3`); names above
  or fixed (`A$1`, `$A2`) are unchanged. Results are **not recalculated by the
  rewrite itself** (`@sum(A2..A4)` shows 11, the value it had), only text moves.
- **Insert a row** after row 3 (Table > Insert Column/Row, After): names at or
  past the new row move down, so a range ending below the point stretches
  (`A2..A4` becomes `A2..A5`, `A3..A5` becomes `A3..A6`); `A2`, `A$1` stay.
- **Insert Column** is refused when the table fills the frame ("Not enough room to
  insert a column", the Columns radio dimmed): not tested further.
- **Copy and paste a formula cell** (Ctrl+C, Ctrl+V; recalculated at once):
  offset (0,+2): `A$1+$A2` becomes `A$1+$A4` (the `$` parts stay); offset
  (+1,+1): `@sum(A1..A2)` becomes `@sum(B2..B3)`.
- The Insert Row and Delete Row dialogs' defaults: Rows, After (insert);
  Delete row (delete).

### 11.5 TDV6: recalculation on open (TDF6)

A file with stored results edited by hand (99 where the formula gives 11, 77
where 15): Ami Pro **shows the stored results** (99, 77), recalculates nothing
on open, does not mark the document changed (Save As is plain), and a re-save
writes the same bytes but for the `[desc]` time and revision count. TN5 (none on open) holds.

### 11.6 TDV7: merging table data cells (AMD2B General, AMD3B Currency; AML5, AML6; AMD4B, AML7)

Merge & save as, field AMOUNT. **The result does not depend on the style's
format** (General and Currency give identical text) and not on the cell being
a number: it is the merge engine's own **string rule**: when the field text
(unpadded) ends in a digit and contains a decimal `.` and every other
character is a digit, `$`, `,`, `+` or `-`, the trailing zeros after the point
are removed and then a bare point:
`$1,200.00`→`$1,200`, `75.50`→`75.5`, `1234.50`→`1234.5`, `0.00`→`0`, `0.000`→`0`,
`100.00`→`100`, `1.0`→`1`, `5.`→`5`, `10.50`→`10.5`, `$5.10`→`$5.1`,
`1,200.00`→`1,200`, `1.230`→`1.23`, `-0.50`→`-0.5`, `+5.00`→`+5`, `00.50`→`00.5`,
`1234567.00`→`1234567`, `1.5.0`→`1.5`.
**Unchanged**: `007`, `100`, `-0`, `1,200`, `12`, `-7.25`, `3.14159` (no trailing
zero), `5%`, `10.00%`, ` 5`, `1.00 ` (trailing space), `1.00abc`, `abc1.00`
(a letter anywhere), and `.50` (no digit before the point). Alignment, the
cell's flags and the style are irrelevant. The same text is merged from a
delimited file as typed (AMT3, earlier): TD.9 applies this string rule to the
cell text, not the display function.

### 11.7 TDV8: Sort (TDX1-TDX6 in text, TDY1 as a table)

- **Dialog**: Tools > Sort… (a selection of paragraphs; in a table with only a
  caret first the question "Do you want to sort the entire stream? If not,
  click on Cancel…" (OK to go on); in a table the box reads "No. rows in
  record" and Delimiter is dimmed). The dialog **keeps its last settings**
  (a Descending, Numeric choice came back next time).
- **Alphanumeric order is the `SORTTAB` weight order, then bytes** (the
  app's `sort_weight`/`index_order` match every pair seen): `-5`, `[bracket`,
  `#hash`, `~tilde`, `_under` (all punctuation, weight 1, so by the next
  character), `10`, `1abc`, `5`, `9`, `A`, `a`, `a,b`, `a-b`, `a.b`, `a b`
  (All words), `aardvark`, `ab`, `Apple`, `apple`, `apple`, `Banana`, `banana`,
  `cherry`, `Zed`: case-blind, upper before lower on a tie of weights, digits
  before letters, a space after punctuation. With Word 1st, `a b` is keyed by
  `a` (order after ties as below).
- **Numeric** (descending shown): keys reduce to their digits: `1,000` is 1000,
  `1e2` is 12, `007` is 7, `+4` is 4, `$3` is 3, `3.0`, `.5`, `-0.5`, `-5`, `(7)`
  negative (the parenthesised is -7); no digit (`zz`, `x 12` keyed by `x`, `abc`)
  sorts after all numbers ascending, before them descending.
- **Descending reverses the whole result**, ties included.
- **Ties are not stable**: paragraphs `k1 d05, d08, d02, d04, d12` (sorted on word
  1) came out `d02, d04, d05, d08, d12` (record order of the text) but the
  **table** rows `d05, d08, d02, d04, d12` came out `d05, d08, d04, d02, d12`
  (with the other keys likewise shuffled: k2 `d03 d09 d10 d07`, k3 `d06 d11
  d01`; originals k2 `d09 d03 d07 d10`, k3 `d01 d06 d11`). It is the heap sort
  of 10.7, deterministic: reproduce ties by implementing the same heap sort
  (`17f6`/`259c`) and use these vectors (inputs in `IN/TDX5.SAM`, `IN/TDY1.SAM`,
  rows in the screens `s_tiesa`, `s_tbl2`) as tests; an already sorted input
  (TDX2) comes out unchanged.
- Rows travel whole in a table sort (the third column followed its key).
