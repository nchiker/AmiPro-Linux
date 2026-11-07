# Merge, labels, and envelopes in Ami Pro 3.1

Evidence for the Phase 5 queue's run 14 (`merge`). Written on 2026-11-03,
before the run, from three sources:

- **D**: the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`. Routines are
  given by address. No decompiler output is kept in Git.
- **G**: the user's guide, ch. 28 (merge), ch. 13 (envelopes), ch. 16
  (a table as data), ch. 32 (power fields), and `STLGUIDE.SAM`.
- **S**: the corpora's files (named here only when shipped with Ami Pro).

Confirmed in the VM on 2026-11-03 (fixtures MGM1–MGM9, MGMA–MGMF, MGN1–MGN7,
MGS4–MGS8, MGT1; section 11).

## 1. Commands and dialogs (D)

- **File › Merge… (0x30d)** opens `MRGWELCOME` (template 3029,
  `1458:0000`), the three steps:
  1. Make or edit the data file. If the active document is a data file,
     the dialog edits it (0x313); otherwise it selects one (0x30c).
  2. Make or edit the merge document. Without a data file, it asks
     whether to use the current document (0x314, `1558:0239`). That
     command selects a data file, stores it (0x236), and opens Insert
     Merge Field.
  3. Merge (0x6e).

  With a data file assigned, step 3 is preselected; without one, step 1.
- **Select Data File** (0x30c, template 3028): a file list, List files of
  type, New… (0x310), and Edit… (0x315).
  - An external file without field names opens Merge Data File Fields
    (template 3030): names in the first record, or names in a
    description file.
  - From there, Create Description File… (template 3031; its writer is
    `1558:0404`).
- **Editing a data file** (0x310, 0x313, 0x315 → `1558:0000`) opens the
  data document and runs the shipped macro `AMIMERGE.SMM`. The index
  card, Field Mgt, Sort, Go To, Create Data File and its delimiter
  options all live in that macro, not in native code.
- **Edit › Insert › Merge Field… (0xb6)** is modeless (`MRGMARKDLGPROC`,
  template 1106):
  - Its buttons are Insert, Data File… (0x30c, then 0x236), Close, and
    Continue Merge (back to step 3).
  - Insert runs 0xa6 (`1560:0346`). That builds the power-field code
    `MergeField NAME` and inserts it through the power-field path
    (`FIELDS_NOTES.md`).
- **The Merge dialog** (0x6e, `MERGEDLGPROC`, template 1096; its handler
  is `1398:0000`):
  - Merge & print, Merge, view & print, or Merge & save as (a file
    name).
  - With conditions, and As labels. As labels dims the view and save
    choices.
  - Print Opts… (the print dialog: copies, pages), Conditions…
    (template 1097), Labels… (template 1108), and Data File….
- **Merge, view & print** shows a modeless box (template 1105). Its
  buttons send 0x74 (`1550:0412`): Print and View Next (5), Skip and View
  Next (2), Print All (3), and Cancel (4).
- **File › Print Envelope… (0xed, `15b0:0348`)** only runs the macro
  `AMIENV.SMM` (§8).

## 2. What the document stores (D, S)

- **`[recfile]`**: its writer is in the header writer `11c8:1c7b`, its
  reader is `11a0:0418`. Written only when the document has a data file.
  The values, in order:
  1. The data file's path.
  2. The description file's path. For an external database through a
     filter it is `*` and the type (`*dBase` in the user's corpus). It
     can be empty.
  3. Flags. The last merge mode: 1 print, 2 view and print, 4 macro
     mode. 0x10 means As labels; 0x40 and 0x80 are workgroup indirect
     names.
  4. The Labels dialog's unit: 1 in., then cm., picas, pt. (VM: 1 shows
     in.; 0 shows no unit and misreads the indents; the button cycles in.,
     cm., picas, pt.).
  5. Labels across (default 3).
  6. Labels down (default 12).
  7. The first label's indent down, in twips (default 1440).
  8. The first label's indent right, in twips (default 1440).

  Values 5–8 mean something only with flag 0x10. Without it they are
  whatever memory held: a user file stores 26352, 30616, 14636, 4. A
  reader ignores them then, and the writer keeps them as read.
  - If the second string starts with `[`, the reader takes the defaults
    1, 1, 3, 12, 1440, 1440.
  - "Print each label N times" is not stored.
- **`[fldnames]`** holds the eight Doc Info field names, not merge names
  (`FIELDS_NOTES.md`). Merge field names come only from the data file.
- **Merge fields in the body:**
  - Ami Pro 3.1 writes a power field of kind 12:
    `<:X12,-32768;MergeField NAME>`, the result `<NAME>`, then the end
    mark (S: the shipped `_MERGLET.STY`).
  - Older files have `<:MNAME>` … `<:M>` (S: one user label document).
    The body parser (`11b8:030a`, case `M`; `11b8:16c6`) reads it as a
    field whose code is the name (characters other than letters and
    digits become `_`) and whose result is `<NAME>`.
- **Conditions** are not stored in the document.

## 3. Data files (D, G, S)

- **Ami Pro's own data document** (kind 0; its reader is in segments
  `13a8` over the body reader `1388`):
  - The first paragraph holds the delimiters, formatting stripped
    (`13a8:0ab7`). Two characters are the field delimiter, then the
    record delimiter. One character is the field delimiter, and a
    paragraph end ends each record. Anything else is "Data file is not
    in valid format" (3005).
  - The next record holds the field names; an empty name is invalid.
  - Then the records. A record may span paragraphs and ends at the
    record delimiter. An empty field is two delimiters together.
  - New files use `~` and `|` (S: the shipped `MERCDATA.SAM`). The
    delimiter options suggest `$ # ! % ( ) ~ | - ,`.
  - A record holds at most `GlossarySize` bytes (AMIPRO.INI; 2048 by
    default; values 1–32 are KB, larger values cap at 32000). Past it is
    error 3109.
- **A table document** (kind 4; `14b8`): a document holding only a table
  (`1388:0211` finds `[tbl]`). The first row holds the field names, and
  each later row is a record. G: on the first page, with no other text.
  It is edited by hand only.
- **Description-file kinds:** DIF (1, `1540`), fixed-width ASCII (2,
  `1390`), comma-delimited ASCII (3, `1548`).
  - The description file is an Ami Pro document. Its first paragraph is
    the kind (`DIF`, `FIXED`, `COMMA`, any case), and each later
    paragraph is a field name.
  - A FIXED field is written `name,width` (width 1–2047). Spaces in
    names become `_`. Errors 3096–3099.
  - Comma files: one record a line, fields split on commas outside
    double quotes, `""` an embedded quote, spaces trimmed at both ends.
  - Plain-text kinds must pass a control-character check on the first
    80 bytes (`1390:0241`).
  - S: a user's description file holds `DBASE` and then a list of names.
- **Other databases** (kind 5; `15f0`) are read through an import filter
  DLL (`OPENIMPORTFILTER`, `GetFilterToken`). G lists 1-2-3,
  Excel to 5.0, dBASE III and IV, Paradox to 3.5, and SuperCalc.
  - Several user documents merge from dBASE `.DBF` files.
  - Which filter DLL handles dBASE is not known.
- **Field names** are matched without regard to case
  (`1398:076f`, normalised by `13a0:0ef5`: upper case, spaces to `_`,
  other characters dropped).
  - A value spanning paragraphs is joined with a line feed.
  - An unknown name says "… is not a valid field name" (3098) and shows
    `<NAME>`.

## 4. The merge (D, G)

- The merge works in place on the one merge document. For each record
  that passes the conditions (`13a0:0461`), every power field is worked
  again with that record current (`1550:0599`, `0635`). The fields are
  in the main text, page tables, repeating frames, headers, footers, and
  cell frames.
- **Merge & print** (`1550:0145`) prints each record's document
  (`1550:08d1`).
- **View & print:** the document is shown with each record, and edits
  carry to later records because it is one document (G).
- **Save as** (`1550:0354`) appends the body for each record through the
  body writer (`11b0:1405`), with a page break between records, into one
  file. G: main text only, and no pictures.
- There is no record range; there are only conditions and the print
  dialog's pages.
- **Warnings:** the document's fields do not match the data file (3225);
  invalid fields, continue? (3226); the file already has a records file
  (3209).
- **Empty fields** (G): no space is left for an empty field, and a line
  holding only an empty field has no height. To confirm in the VM: where
  the line builder does this.
- **Power fields** (G ch. 32):
  - `MergeField name`.
  - `MergeRec`: the record number. Records left out by conditions are
    not counted; skipped records are. It is 0 outside a merge.
  - `NextRec`: the next record within the same document, for phone
    lists such as `_PHONLST.STY`.
  - `Skip`: leave the record out, usually inside `IF … Endif`.
  - `Defined`: true for a bookmark, a variable, or a merge field.

## 5. Conditions (D, G)

- **The dialog:** three rows. Each has If, a field (from the data file's
  names), an operator, a value, and And or Or to the next row.
- **Operators** (`13e8:072a`): `=` `<` `>` `!=` `<=` `>=`. An empty value
  is allowed only with `=` and `!=`.
- **Evaluation** (`13a0:0b69`): left to right, And binding tighter than
  Or.
- **Comparison** (`13a0:0bbc`):
  - The record's text is trimmed and read as a number (`1538:0000`: a
    currency sign, thousands separators, the decimal sign, a minus sign,
    parentheses, or a trailing minus).
  - When the condition's value is a number, the comparison is numeric.
    Otherwise both sides are tried as dates (`1538:02db`), then compared
    as text (`1150:019a`). Whether that text comparison ignores case is
    not known.
  - No wildcards (G).

## 6. Labels (D, G, S)

- **A label document is one label's page.** Its page layout is the
  label's size. G's custom rule: top and left margins 0, the right
  margin is the gap between labels across, and the bottom margin the gap
  down.
- **The Labels dialog** (template 1108) sets labels across, down, the
  first label's indent (down and right, with a unit), and Print each
  label N times.
- **Printing** (`1550:0145`): each label is printed with the printer's
  origin moved to (column × page width + indent right, row × page height
  + indent down). It fills across, then down, then starts a new sheet,
  and prints a partial sheet at the end.
- **Avery's sheets** come from `AMILABEL.INI`, an installed file (a copy
  is in the VM folder, outside Git). Its `[LabelTypes]` section has 119
  entries: part number, kind, height, width, across, down, top margin,
  side margins, pitches, and paper. The app's list is its own, from
  Avery's published sizes (MG7).
- **`_LABEL.STY`** runs `LABEL.SMM` on File › New. The macro shows the
  Labels list (Merge, Manual, Custom) and sets the page layout. Manual
  builds a table with one cell per label.

## 7. Style sheets and samples shipped (S; names only)

- **Style sheets:**
  - `_MERGLET.STY`: a merge letter.
  - `_MERGDAT.STY`: a data file with names and delimiters.
  - `_PHONLST.STY`: a phone list using NextRec.
  - `_LABEL.STY`: labels.
  - `_ENVELOP.STY`: an envelope with a return address area.
  - `_AMIENV.STY`: Print Envelope's sheet, with a `Recipient` style.
- **Samples:** `MERCDATA.SAM`, `MERCLET1.SAM`, `MERCLET2.SAM`,
  `MAILMRG.SAM`, `DESCRIPT.SAM`.
- **Macros:** `AMIMERGE.SMM`, `LABEL.SMM`, `AMIENV.SMM`, `ENVSETUP.SMM`.

## 8. Envelopes (D, G)

All of Print Envelope is the macro `AMIENV.SMM`, with `ENVSETUP.SMM` for
Options.

- **The address** is the selection. Without one, it is the first address
  on page 1. If none is found, a dialog asks for one.
- **Sizes:** buttons for 10, 11, 12, 6¾, 9, and Custom, a list of more
  sizes, length and width fields (3–11 in), and units.
  - The macro's table gives inch pairs from 3.87 × 7.5 to 4.75 × 11, and
    metric pairs such as 4.33 × 8.66.
  - Which name goes with which size is not yet mapped.
- **Return address:** Print return address, a text field of up to 5
  lines, and saved names with Add Address and Delete Address.
- **Options, per printer:** the bin, portrait or landscape, the feed
  position (left, centre, right), an envelope feeder, and offsets that
  move each address.
- **Settings** are kept in `AMIPRO2.INI`'s `[AmiEnv]` and `[AdrsBook]`
  sections.
- **Merging envelopes:** with merge fields selected, Print Envelope
  offers a merge from the document's data file, then Merge & print.
- **The style sheet:** `_AMIENV.STY`, a page about 9 × 4 in that the
  macro resizes, with the address in its `Recipient` style.

## 9. The corpora (S)

- **The user's corpus:**
  - 27 documents have a `[recfile]`. 15 name no data file; 3 name
    the same table data file; the rest name Ami Pro data
    documents, dBASE files, or an ASCII file.
  - dBASE files appear in 5 documents, an ASCII file with a description
    file in 1.
  - One label document uses `<:M` fields.
  - The sample data file `MERCDATA.SAM` is in the corpus.
- **The ISO:** 6 documents have a `[recfile]`.

## 10. Open points for the run's fixtures

1. Merging in the VM to a file: a data file, a merge letter, conditions
   (text, number, date), empty fields on their own line and among text,
   NextRec, Skip, and a table data file. Compare text and page breaks.
2. Labels: Avery 5160 merged to a file, then printed to PostScript for
   the positions.
3. Envelopes: Print Envelope to PostScript for sizes and positions.
4. Whether Ami Pro's text comparison in conditions ignores case.
5. dBASE: the `.DBF` format is public (dBASE III/IV headers). Read it
   natively, without Ami Pro's filter.

## 11. Confirmed in the VM (2026-11-03, the merge run's setup)

Inputs written by the run into `vm-lab/IN` (MGD1, a data file of five
records; MGL1–MGL9, merge letters), merged by Ami Pro 3.1 with Merge & save
as into `vm-lab/OUT`. Route: File › Merge… (click it: its mnemonic collides
with Master Document…), step 3, the Merge dialog's Merge & save as.

- **The merged file** keeps every power field's marks with the record's
  result between them (`<:X12,-32768;MergeField TITLE>Mr.<:X~12,…>`), and
  `MergeRec` likewise. Records are separated by a paragraph holding only
  `<:p<* >>`; none follows the last. `[recfile]` is copied as read. The
  file ends after the last paragraph, without the lone `>` line. An `IF …
  Skip …` field is not written (its paragraph is left empty).
- **Empty fields** (MGM1, view mode): an empty field leaves its spaces and
  commas ("Pam  Beck", "Seattle, , USA"). A paragraph holding only empty
  merge fields is written with `<:S+0>` before its first mark and shows
  with no height; so is a paragraph holding only `NextRec` (MGM6).
- **Conditions:** numbers read with a currency sign and separators
  (`$1,200.00` > 100; `(20)` is not); records left out are not counted by
  MergeRec (MGM2). **Text compares with case** (`smith` is not `Smith`,
  MGM3; `boston` is not `Boston`, MGM5) and by character order (MGME,
  MGMF). And binds tighter than Or (MGM5). **Dates** compare as dates for
  typed values `3/15/94`, `3/01/94`, `3/1/1994` (MGM9, MGMC, MGN1–MGN5,
  MGN7), but `3/1/94` (`>=` none, MGM4), `1/1/94` (`>` all, MGMA), and
  `03/01/94` (`>=` all, MGN6) give results no date or text order explains:
  open, to map in `1538:02db`. The app compares all of them as dates.
- **NextRec** (MGM6): a document takes the next record at each NextRec;
  documents start at records 1, 3, 5; past the last record the fields are
  empty.
- **Skip** (MGS6, MGS7, MGS8): `if LAST = "Beck" Skip endif` and `if
  MergeField LAST = "Beck" Skip endif` leave Beck out, and MergeRec counts
  him (1, 3, 4, 5). `if MergeField AMOUNT < 100 Skip endif` skips no
  record (MGM7, MGS4): an ordering of a text against a number holds for
  none (MC1).
- **A table data file** (MGT1, made in Ami Pro: `[edoc]` holds only
  `<:t0>`): its first row names the fields. Choosing it for MGL1 asks
  "There are merge fields in the current document that do not match this
  records file.  Continue?"; the merge says "TITLE is not a valid field
  name." for each unknown field and asks "There are invalid merge fields
  in this document.  Continue merging?"; unknown fields come out empty
  (MGM8).
- **Merge, view & print** shows each record in the document with a small
  Merge box (Print and View Next, Print All, Skip and View Next, Cancel);
  after Cancel the document shows MergeRec 0.
- **Envelope sizes** (from `AMIENV.SMM`'s list, read as text): Monarch 7½
  × 3⅞ in, Size 6¾ 6½ × 3⅝, Size 9 8⅞ × 3⅞, Size 10 9½ × 4⅛, Size 11 10⅜
  × 4½, Size 12 11 × 4¾, Custom, DL 220 × 110 mm, C5 229 × 162, C6 162 ×
  114, C65 230 × 110, Italian 230 × 110.
- Not yet made: labels to PostScript (positions), Print Envelope to
  PostScript.

## 12. The run's twins (2026-11-03, ME.13)

The app's data file AMD1 (made with New Data File… and the record editor),
merge letter AML1 (fields clicked in the Merge panel, text typed between),
and label document AMLB (Avery 5160 from the sheet list), written by Save
As SAM:

- Ami Pro opens all three without a message. AMD1 shows as its data file;
  AML1's Merge dialog names it.
- Ami Pro's Merge & save as of AML1 (AMT1) gives the app's merge of it,
  paragraph for paragraph: every field's marks with the record's result,
  `<:p<* >>` between records, MergeRec 1–3.
- AMLB: Modify Page Layout shows Custom 2.75 × 1.00 in., margins 0, 0.12,
  0, 0; the Merge dialog has As labels checked and Merge, view & print
  dimmed; Labels… shows 3 across, 10 down. The first run wrote unit 0,
  which Ami Pro shows as no unit with the indents misread; unit 1 is in.
  (MGL1 shows in. for 1); rewritten with 1, Ami Pro shows 0.50 and 0.19
  in.

## 13. Table data files after first use (2026-11-04, MR1)

The record editor became the data file's table (`UI_SPEC.md` §38, "Rework
after first use"). New Data File… writes a table data file (kind 4: one
table, the names in a shaded heading row). The twins again:

- AMD1, now made by typing into the table's cells (Tab between values, a
  record from the last cell), opens in Ami Pro without a message as the
  app shows it: the heading row shaded, lines around every cell. Its cell
  records are Ami Pro's own (MGT1's `0 0 0 0 0 0 1 0 0 0 0` for text;
  `512 … 14803425` for the shaded heading cells).
- Ami Pro's Merge & save as of AML1 over it (AMT2) is the app's merge,
  paragraph for paragraph (the `merge_twins` test compares AMT1 and AMT2).
- **Numbers in table cells.** A letter merging AMOUNT from AMD1 (AML2, AMT3)
  shows that Ami Pro takes a table cell that reads as a number as a
  number: `75.50` merges as `75.5` and `$1,200.00` as `$1,200` (`150` as
  `150`), as its table also shows them. From a delimited data file the
  text merges as typed. The app merges the cell's text. This is the
  table's number display, not the merge's: it belongs to the table-data
  run (number formats, formulas; queue 20), and is an open point until
  then.
