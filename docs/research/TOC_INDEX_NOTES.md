# TOC, index, and glossary in Ami Pro 3.1

The research behind the TOC, index, and glossary run (Phase 5 queue run 12,
`docs/playbooks/FIELDS_AND_REFERENCES.md`), written 2026-11-03. The sources
are the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`, read for routines by
address (decompiler output stays outside Git), the dialog resources
(`tools/ne_resources.py`), the user's guide (ch. 25), both corpora (shapes
and counts only), and fixtures made in the XP VM. The tags are:

- **D**: VERIFIED by the decompile.
- **G**: the guide.
- **C**: the corpus.
- **V**: VERIFIED by a VM fixture.
- **I**: INFERRED.

Parts A–C were mapped from the decompile and record their own open
questions. The fixtures below answer the main ones.

## Fixtures (V, 2026-11-03; `../AmiPro-Reference/vm-lab/IN`, `OUT`)

- **TI1** (`IN/TI1.SAM`, written on the host on `_DEFAULT.STY`): four pages,
  Title and Subhead headings, 20 index marks (primary only, secondary,
  Other with an empty secondary, no `#`, case variants, a leading digit),
  and one `TOC 3 "Marked entry"` mark. Tools › TOC, Index with Table of
  contents, Index, and separators, with no TOC output file and Index output
  `L:\OUT\TI1IDX`.
  - **TOC Options' first state:** Title at level 1 and Subhead at level 2
    (their outline levels), with Page number, Right align, and the dots
    leader. All other styles are None.
  - **TOC into the current document:** the body now starts
    `<:t0><:p</B>Standard>`. That is the page table's anchor, then a page
    break with flag byte 0x82: 0x80 (a layout name follows, `Standard`) plus
    0x02, a bit not seen before. The TOC page stands before the text. The
    document gained TOC1–TOC9 and TOCPG1–TOCPG9.
  - **Page numbers:** they are the pages from before the TOC was inserted
    (1–4, while the status bar shows the last heading on page 5).
    Regenerating (`OUT/TI1T2.SAM`) **replaces the table in place** (one
    `<:t0>`), and the numbers stay 1–4. So page numbering after the TOC
    page does not count it (Part A's `1588:06e4`; the 0x02 bit is the
    likely carrier, I).
  - **Cells:** page cells hold `1.00`, a number, in TOCPGn, with F3 16384
    (the dots leader) on both cells of a row. The mark made a level 3 row
    in TOC3 at its place in document order.
  - **Saving and closing:** generating the index made Ami Pro ask to save
    TI1 (Yes), then close it and open the index file.
  - **Marks rewritten:** the save rewrote every index mark's value with its
    page (`<:X8,1;…`, `<:X8,4;…`). The TOC mark kept -32768.
  - **`[toc]` written:** `Title`, `Subhead`, an empty name, `.`, `.`, an
    empty separator, then `15` (levels 1–2 with both bits).
  - **`[master]` written:** an empty TOC name, `L:\OUT\TI1IDX.SAM`, then
    `1` (separators), `0`, `0`.
- **TI1IDX** (`OUT/TI1IDX.SAM`, on `_INDEX.STY`) is the index as
  paragraphs:
  - Separator `1`, then `123 numbers, 3`.
  - Separator `A`, then `Apple` with Secondary `green, 3` and `red, 3`,
    then `apple, 3`. `Apple` and `apple` are separate entries, capital
    first, and the secondaries are sorted.
  - `Bear, 4 See also Animals`. Other text follows the pages, keeping the
    space typed before it.
  - `fonts, 2` and `frames, 3`, then `installing, 1`, then `No page` (no
    `#`, no number).
  - `Printer, 1, 4` with Secondary `setup, 1, 2, 4`. Pages are merged and
    ascending, with no ranges. `Primary` takes the page of marks with no
    secondary.
  - `Zebra, 2 See Animals`.
  - The title "Index" and three columns come from the sheet's layout. They
    are not in the text.
- **TI2** (`IN/TI2.SAM`; output `OUT/TI2TOC.SAM`): TOC Options with Title's
  Page number off, and Subhead's Right align off with the comma separator.
  - With Right align off, the Leader box becomes "Separator" and offers none
    or a comma.
  - **A TOC output file** is a new document on `_TOC.STY` holding only
    `<:t0>` and the two-column page table. The source document is not
    saved.
  - **The cells:** `Getting Started` with a page cell of one space;
    `Installing, 1` with a page cell of one space; `Marked top` (a
    `TOC 1` mark) at level 1 among the headings. Cells have no leader
    (F3 0).
  - Ami Pro wrote the `[data]` cell records in no row order (row 2 first).
    Our writer keeps its own order.
- **GLO0** (`OUT/GLO0.SAM`, written on the host): a glossary data file with
  `~|`, `id~text|`, and `hi~Hello <+!>there<-!>.|`.
  - **The Data File box needs an existing file.** For a missing one it says
    "Cannot find L:\OUT\GLO1.SAM." and stays open.
  - **Mark Glossary Record** with name `sy` appended
    `sy~Sincerely yours, Ami.|` after the last record. It then wrote the
    index footer straight after the final `>` (`[gloidx] 2`, `hi 604`,
    `sy 632`, `00000664`): the hidden index of Part C.
  - **Ctrl+K** after `Say hi` replaced `hi` with the record's text,
    keeping its bold. Ctrl+K after `x`, which is not a name, opened Insert
    Glossary Record with `hi` and `sy`.
  - **Insert from the dialog** put the record at the caret, before the `x`,
    without replacing it.
  - **Ami Pro's Undo** after that was erratic: it removed the inserted line
    and brought back `hi` before `Hello`.

# Part A: Tools › TOC, Index (generation, [toc], [master])

## Ami Pro 3.1: Tools > TOC, Index (map for queue run 12)

Sources: decompiled `AMIPRO.EXE` (amipro.c, amipro-cmds.c) and `AMIPROUI.DLL`
(amiproui.c); dialog resources (`tools/ne_resources.py`); the user's guide
chapter 25 (`/tmp/amipro-phase1-corpus/CDNAV/DOCONLIN/AMIPRO/AP3CH25.SAM`,
"Using Table of Contents", "Using Index"); both corpora; the `SORTTAB`
resource in `AMIPRO.EXE`; `_TOC.STY` and `_INDEX.STY` from the ISO. No VM
fixtures were run. Tags: **D** = VERIFIED (decompile), **R** = resources,
**G** = guide, **C** = corpus, **I** = INFERRED (reading of code or data, not
confirmed by a fixture).

Addresses are `segment:offset` (segment n = selector 0x1000 + 8(n-1)).
Findings are in our own words.

### 0. Overview

- Command 0x1d1 (Tools > TOC, Index...) handler `1568:0000` (D). The dialog
  runs in `AMIPROUI.DLL` (`MASTEROPTSDLGPROC` `1428:032e`; the same dialog
  also serves the master-document command, which is why its strings talk
  about "objects"; `1428:128d` tests "command == 0x1d1"). It returns one
  word: 0x65 = TOC only, 0x66 = Index only, 100 = both (`1428:032e`, OK
  branch). `1568:0000` turns that into a flag set: TOC = 2, Index = 1, plus 8
  (D) and calls the driver `1568:0466(flags, copyOfMasterRecord)`.
- Before generating, if the document is not in the "display as printed"
  state (`DAT_2ad7 & 8` clear or `DAT_28c7 & 0x10` set) a message box warns
  ("A Table of Contents / Index created when Display-as-printed is not
  selected may not contain accurate page numbers", strings 3129 / 3045 in
  the DLL) with OK/Cancel; Cancel aborts (D `1568:0000`, I for the exact
  flag meaning).
- Generation runs in two stages (D `1568:0466`, `1568:08e0`):
  1. **Scan** the document ("Scanning document." status), writing the TOC
     entries and the index marks into temp files.
  2. **Build the output** ("Generating final TOC." / "Generating final
     Index." statuses, EXE strings 1405 / 1404) and save it.
- The generator for TOC is `1588:0239`; for Index `1378:0000` (scan), then
  `1568:07bb` (records to the temp file), `1378:05ec` (sorted list),
  `1378:0a25`, `1378:0b0e` (build).
- **TOC is built as a two-column page table** (G: "The table of contents is
  an Ami Pro page table"; D `1588:1463`, `14c0:081f`/`0a6b`; C: user document U1 (the corpus's one generated TOC)).
  **Index is built as plain paragraphs** in a new document based on
  `_INDEX.STY`.
- Master documents (`[master]` file list, `1660:054d` etc.) generate one TOC
  and index over several files. Out of scope here; the `[master]` section
  also carries this dialog's file names (section 2).

### 1. The `[toc]` section (D `11c8:1bbc` writer -> `1588:261d`; reader `1588:24ed`)

The document's TOC style list is a chain of 0x28-byte nodes (`DAT_1680_26fa`):
style name (+2, up to 26 bytes), separator string (+0x1c, up to 10 bytes),
flags word (+0x26). A style's **level is its position in the chain** (1 =
first). Level 1..9; the dialog never writes a gap (section 2), so levels are
dense.

Written (when the chain is not empty? see below), each value on its own
tab-led line (`\t%s` / `\t%d`, strings escaped as body text):

| Lines | Content |
|---|---|
| 1-3 | style names of levels 1, 2, 3 (empty strings for absent levels) |
| 4-6 | separator strings of levels 1, 2, 3 |
| 7 | one flags word: level i's 2-bit flags at bits 2(i-1)..2(i-1)+1 (levels 1-3) |
| then, for each level >= 4 | name, separator, flags (three lines each; flags a plain 0..3) |

Flag bits per level: **bit 0 = Page number, bit 1 = Right align page number**
(D `1450:0211`, `1588:24ed`). The reader keeps `(word >> 2i) & 3` for the
first three nodes that have a non-empty name and creates further nodes from
name/separator/flags triples until the next `[` line (D). Corpus:
`63` = three levels, both bits each; `3` = level 1 only; `12` = level 2
only (AP31NAV); `15` = levels 1-2 with page number and right align but empty
separators (`a user style sheet`); `16387`/`16399` (an extra 0x4000, older Ami Pro:
ignored by the reader, I).

Separator strings: what the TOC Options dialog stores (D `1450:0824`):
- Right align on: none = two spaces `"  "`; dots = `"."`; hyphens = `"-"`;
  line = `"_"`.
- Right align off (page number only): none = `"  "`; comma = `", "`.
- Unused levels: `""`.
Reader side (`1450:07ec`): first byte `_` = 3, `,` = 4, `-` = 2, `.` = 1, else 0.

Where it lives: 53 of 419 user-corpus SAMs (43 with exactly
`Heading 2 / . / 3`), 5 STYs (the style sheet supplies the document's
default TOC choices: `a user style sheet`, `a user style sheet` ...); new documents from
`_DEFAULT.STY` have none. Written only when the chain is non-empty (the
writer `11c8:1bbc` tests `DAT_26fa != 0`), after `[sty]`/the layout parts and
before `[master]` (D `11c8:14e0`).

#### `[master]` carries the TOC, Index dialog's file names (D `1660:0000` writer, `1660:00fe` reader, `1660:043c`)

```
[master]
	<TOC output file>          path, may be empty
	<Index output file>        path, may be empty
	<flags>                    bit 0 = Include alphabetical separators
	<bytes of the file list>   0 when none
	<count of files>           0 when none
	<files...>                 master document members, one per line
```
Corpus: `AP3CH02.SAM` `c:\pro20\docs\sitoc.SAM`, `c:\amipro\docs\2index.SAM`, 1,
0, 0; most index-bearing guide chapters have an empty TOC name, an index
name and flag 1; `a user document` has a 126-byte, 3-file list. (C, D.)
Record is created blank by `1660:043c` (flags 0, names empty): so
**alphabetical separators default OFF** for a document that never ran the
dialog (D). The dialog's two "Generate" check boxes are **not stored**: at
open, "Table of contents" is checked iff a TOC output file name is stored,
"Index" iff an Index file name is stored (D `1428:032e` init).

### 2. Dialogs (AMIPROUI.DLL)

#### TOC, Index (template 1094; `MASTEROPTSDLGPROC` `1428:032e`; OK checks `1428:0fe9` TOC, `1428:1136` Index)

| ID | Control | Meaning |
|---|---|---|
| 101 | check "Table of contents" | generate TOC |
| 109 | check "Index" | generate Index |
| 119 | check "Include alphabetical separators" | `[master]` flag bit 0 |
| 102 / 115 | edits "TOC output file" / "Index output file" | file name (typed in the dialog; 105/106 and 114/113 are the path label and directory/file lists, 8.3-style DlgDirList) |
| 118 | "TOC Options..." | opens TOC Options (id 0x76 in code) |

Rules (D):
- OK with Table of contents checked: name required **only if** a TOC name is
  already stored or the command is not 0x1d1; for 0x1d1 with nothing stored a
  blank name is allowed and means **"place the TOC at the beginning of the
  current document"** (G confirms; error text 3152 "Must specify an output
  file for the Table of Contents.").
- Index checked: the Index output file name is **always required** (error
  3144).
- The name is made absolute against the directory list, lower-cased
  extension rules per `1040:0804`; stored full-path in `[master]`.
- The check boxes 101/109 are disabled and cleared if the dialog is opened in
  the TOC command and the document has no stored names and the "object"
  query returns nothing (D `1428:032e`, end of init: details I).
- 119 (alphabetical) is enabled with Index.

#### TOC Options (template 1135; `TOCOPTSDLGPROC` `1450:0000`)

Controls: owner-drawn list 25350 with the column header
`1 2 3 4 5 6 7 8 9 None` (25700), check 25550 "Page number" (bit 0), check
25551 "Right align page number" (bit 1), combo 25975 with label 25701
(switches between "Leader:" when Right align is on, "Separator:" when off,
strings 2375 and 2374), Promote 25585, Demote 25586, OK, Cancel.
- Every paragraph style of the document appears in the column of its TOC
  level or under None (G, D). Promote/Demote (and dragging) move the
  selected style one column; selecting a None style disables the three
  controls (`1450:02b8`).
- Leader combo contents (strings 2307 "none", 2370 "...", 2371 "---", 2372
  "___", 2373 ","): right-align mode: none / dots / hyphens / line; off:
  none / comma. (D `1450:02b8`; storage in section 1.)
- Page number off disables right align and the leader (`1450:02b8`).
- **Initial state** (D `1450:0624`): the DLL asks the EXE for every
  style (name plus its outline level word); a style whose outline level
  (Style Properties > Bullets and numbers level) is not yet taken by another
  style starts at TOC level = that level with Page number + Right align on and
  the dots leader; otherwise None. Then the document's `[toc]` chain
  overrides: all reset to None, each listed style gets level = list position;
  a listed style that does not exist in the document raises "TOC style %s does
  not exist in document." (2366) and is dropped.
- **OK** (`1450:0937`): checks each level 1..9: two or more styles at the
  same level -> "Can only have one TOC style for level %d." (2365) and the
  dialog stays; otherwise returns one record per non-empty level, in level
  order: name, separator string, flags. The EXE (`1588:00c6`) rebuilds the
  chain from them (gaps close: **levels are renumbered densely**) and marks
  the document modified.

#### Mark Index Entry (template 1109; `INDEXENTRYDLGPROC` `1350:0000`) and TOC Entry (template 1115; `TOCENTRYDLGPROC` `1440:0000`)

Index Entry: Primary (combo 102), Secondary (combo 104), Reference group:
check "Page number" (106), check "Other:" (107) + edit (108); Example text
(114), buttons Mark, Remove, Next Mark, Prev. Mark, Next Same, Prev. Same.
The code text it writes (D `1350:0695`): `Index "<primary>" "<secondary>"`
with `# ` after the primary when Page number is checked, and a third quoted
string for Other when given; double quotes inside are doubled. Empty
secondary/other are written as `""`. TOC Entry: level number box (100, up to
9) and text edit (105, up to 240 characters, G).

### 3. TOC generation (`1588:0239`)

#### Collecting entries (D `1588:0239`, `1588:03bc`, `1588:0522`, `1588:0da8`)
1. The scan starts from the main-text paragraph chain (`DAT_2223`) and
   visits every paragraph once, in document order (no sorting).
2. Where the paragraph holds `<:X11,...;TOC n "text">` marks
   (`1588:0522`; `1630:1f0e` parses the code: skip the keyword, level 1..9,
   then the text with one pair of enclosing quotes removed), each mark makes
   one entry at level n with that text, and **the paragraph's style is then
   not used** (G: "includes only the text you select instead of the entire
   paragraph"). Entries from marks need that level to exist in the chain
   (I: the arrays for absent levels come from the default style).
3. Otherwise the paragraph makes an entry iff its style is one of the TOC
   styles in the chain (compared by style identity, the Windows atom of the
   style name; level = position). Entry text = the paragraph text from its
   start, with
   - the list number text prefixed (the paragraph number as shown, e.g.
     `1.1 `; `1588:17a3` expands number/bullet codes: counters, letters,
     roman numerals, `*`, dagger runs; a space is added if the prefix does
     not end in one),
   - the text cut at the first `<:T>` mark (`1588:0da8`: 0x03 `T`; a legacy
     "end of TOC text" mark, also found in user document U1 (the corpus's one generated TOC)),
   - controls handled by `1588:1083`: character-attribute pairs (control 2,
     two bytes) **kept**, so bold/underline runs carry over (BOB3 cells show
     `<+!>`, `<-!>` marks); font/size/colour switches (0x1b) and notes/other
     0x04-framed objects dropped; power-field marks (0x03 `X`) replaced by
     their result text when the kind shows a result; paragraphs whose text
     comes to fewer than 2 bytes including the terminator (empty) are
     skipped (G: "including blank lines" are placed into the TOC; a blank
     line that carries a TOC style is an entry of an empty string only if
     it is not skipped: D says empty text is skipped; G says blank lines are
     included; I: open question 4).
4. **Frames**: after each main-text paragraph, if the document has any text
   frame (`1588:1663`, frame flag 0x200), each frame anchor `<:A n>` in the
   paragraph (control 0x41 `A`) whose frame is a text frame has its
   paragraphs processed the same way, after the paragraph. G: only frames
   with Placement With para above or Flow with text; tables and other
   frames excluded. Headers, footers, floating headers/footers, notes,
   footnotes, Where-placed frames and table cell text are not visited
   (I from the code, G from the guide).
5. TOC Entry marks work in those frames too (same function).

#### Page numbers (D `1588:0c75`, `1588:1951`, `1588:1a0d`, `1308:0026`)
- Page = the page the paragraph starts on (`1588:0b54`, using the layout
  caches; for an entry in an anchored frame, the page of the anchor),
  formatted by **`1951`** exactly as Ami Pro's page-number object on that page
  would show it: the number is the physical page adjusted by page-number
  restarts of inserted page layouts; the style and prefix (Arabic, roman,
  letters, a chapter prefix such as `2-`) come from the page-number object
  (control 0x1b `P`, `11f8:023f`) found in the page's floating header/footer,
  then the layout's header/footer frames, then page frames (`1588:1a0d`,
  `1588:1dea`); with none found the number is plain Arabic with no prefix.
  The master-document page offset (`DAT_25ae`) is added for master runs.
  No ranges: one number per entry.
- user document U1 (the corpus's one generated TOC) shows page cells reading `1.00`, `3.00`, ... (the document's page
  numbers carry a custom format; the generator copies what the object shows).

#### Per-entry output, by the level's flags (D `1588:03bc`, `1588:0522`)
The TOC is a **table with one row per entry**, two columns: column 0 the
entry, column 1 the page cell. Each cell is a paragraph of a TOC style:

| Page number | Right align | Column 0 | Column 1 |
|---|---|---|---|
| off | (off) | entry text, style `TOCn`, no leader | a single space `" "`, style `TOCPGn` |
| on | off | entry text + separator (`"  "` or `", "`) + page text, style `TOCn`, no leader | a single space, style `TOCPGn` |
| on | on | entry text, style `TOCn`, leader from the separator | page text, style `TOCPGn`, the same leader code |

- n = the entry's level (1..9), `TOCPGn` the page-number style for that level.
- Leader code (D `1588:11c2`): first byte of the level's separator: 0 or
  space -> none (mask 0x3fff, "clear the leader bits"); `.` -> `0x4000`
  (dots); `-` -> `0x8000` (hyphens); `_` -> `0xC000` (line); anything else
  (comma) -> `0x4000` is also reachable but the non-right-align branch always
  passes none. These are the cell's **leader bits (cell field F3 `0x4000`,
  `0x8000`, `0xC000`; `docs/research/TABLE_EDIT_NOTES.md`)**.
- Observed in user document U1 (the corpus's one generated TOC) (Ami Pro-written): 11 rows, 2 columns, widths
  `9080` and `975` twips (a 10,055-twip text width), every cell F3 = 16384
  (dots), row heights 364 (Automatic), `<+!>` run marks kept in text, TOC1
  entries and `TOCPG1` page cells.

#### Where the table goes (D `1588:1463`, `1588:071e`, `1588:0694`; G)
- Output file name given: File > New with `_TOC.STY` (EXE string 3151) as
  the style sheet; the table is made there (the sheet already has TOC1-TOC9,
  TOCPG1-TOCPG9, Table Text, Body Text, Header, Footer; see section 7); then
  Save As the output name; the caret is on the first page of that document
  (G "insertion point is at the top of the first page"; `1568:0466`, `0170`).
- Blank name: the TOC goes into the **current document**: the generator
  first looks for an existing TOC table (a page-frame with frame flag
  `0x4000000`: D `1588:0694` tests word at frame +4 for 0x400, i.e. bit 26 of
  the 32-bit flags; BOB3's TOC frame is `68681796` = 0x4000000 | 0x100000 |
  0x80000 | 0x40 | 0x4: table frame, opaque, anchored With para above,
  `[chint]`, TOC mark) and **replaces its rows** (extra rows deleted, new rows
  set, `1588:1463`); if none exists, `1588:071e` creates the page table with
  its anchor `<:t n>` in front of the text and inserts a page break after it
  (BOB3: `<:t0>` followed by `<:p<* >>`; G: "at the beginning of the current
  document"). I: the exact first-time placement and layout choice (`0873`
  picks or makes a page layout) need a fixture.
- Because the TOC adds pages, `1588:06e4` adds the page difference to the
  page-number start of the layouts when the document gets longer (I: so
  body pages keep their numbers; needs a fixture).

#### Styles (D `1588:0a1f`, `1098:07e2`; G)
For each level 1..9 the generator looks for styles named `TOC1`..`TOC9` and
`TOCPG1`..`TOCPG9` (format `%s%d` over EXE strings 1177 "TOC1", 1181
"TOCPG1" cut at the digit) and **creates the missing ones in the current
document**, copying the base style's record:
- `TOCn` copies the style assigned to **level n+1** (level 9 copies Body
  Text), so level 1 entries use the font of the level-2 heading style (G says
  exactly this), indents: alignment left, left indent `(n-1) x 360` twips,
  second indent `n x 360` (matches `_TOC.STY`: TOC1 0/360, TOC2 360/720...).
- `TOCPGn` (index >= 9 in the call): alignment **right**, no indents.
In the corpus 2 documents hold all 18 styles; 36 hold only TOC2, TOC3,
TOCPG1-3 (inherited from a shared style sheet).
When a TOC is generated into a separate file, the source document probably
also gains these styles (`1568:08e0` runs `0239` in the source); I: fixture.
`1588:2013` deletes all 18 styles again (master-document path only: `08e0`
with argument 1).

### 4. Index generation (`1378:0000`, `1568:07bb`, `1378:05ec` ...)

#### Marks (grammar)
Power field kind 8: `<:X8,V;Index "primary" # "secondary" "other">` (no
result text, no end mark). V = `-32768` (0x8000) when made in the main text;
**generation overwrites V with the physical page index** of the mark
(0-based, `1378:020f` via `1630:0ed1` mode 2): the corpus has `<:X8,5;`,
`<:X8,24;`, etc. in 9 marks of generated documents, 219 `-32768`. (D, C)
Parser `1650:038e` (also `1350:0588`): first `"..."` = primary (trailing
spaces of each string trimmed); after it, scanning for either `#` or `"`:
any `#` sets the **page number flag**; the first further quoted string is
the **secondary**, the second is **Other**; strings may be empty `""`.
The corpus's `Index "A" # "B" #` (trailing `#`) form is read the same.
(D `1650:038e`) Quote doubling is read by `1650:012d` (kind 3 fields).

#### Scan (`1378:0000`, `1568:07bb`)
Walk page by page: for page p, append `", " + displayed page number`
(`1588:1951`) to a page list (string i = page i); then every Index mark on that
page (main text, and the frames in the page's frame list `DAT_2225`) is
written to the temp file as `<p-1>;<code text>` and its V set to p-1. The
guide: marks in headers/footers are not mentioned (I: not visited).
Pages are visited until the document ends (`1378:0000` loop).

#### Building (`1378:05ec`, `1378:06be`, `1378:04b0`, `1378:02db`, `1378:03d3`, `1378:0a25`)
For each record in file order:
- Parse the code (primary, secondary, Other, `#`).
- Find the primary in a sorted chain (`1378:04b0`, with per-letter head
  pointers `1378:0d38/0d8c`); create it if missing (`1378:06be`: a paragraph
  for the primary, plus, when a secondary is given, a paragraph for the
  secondary marked as secondary (flag 0x10), inserted among that primary's
  secondaries in the same order).
- With `#`: append the page string (`", "` + displayed number) to the
  **secondary if there is one, otherwise the primary** (`1378:02db`); a page
  already ending that entry (suffix match, so equal to the previous page
  string) is **not repeated**. Pages come in page order, so they ascend; **no
  ranges (5-7) are made**, no "passim". (D)
- Other: appended raw after the page numbers (`1378:03d3`); G: the first
  character you type is kept (",", " "): "If you want a comma or a space ...
  type it as the first character". I: `1378:0a25`'s final join drops one
  character at the Other boundary; needs a fixture (question 7).
- An entry with neither `#` nor Other is just the heading.
Each entry's text is held as separate zero-terminated strings (heading, pages,
Other); `1378:0a25` joins them, giving `Primary, 3, 5`.

#### Sorting (D `1378:0870` -> `1150:019a`, `1150:0163`, `1020:2020`; R `SORTTAB`)
- Compare by a 256-byte weight table (resource `SORTTAB`, type `STAB`, 256
  bytes in `AMIPRO.EXE`, loaded at start into DGROUP `0xdea` by `1018:0194`;
  not locale-dependent): control codes 0, space and `0xA0` = 2, **every other
  punctuation/symbol/high byte = 1** (so punctuation sorts before space,
  space before digits), digits `0`-`9` = 3-12 (each digit its own weight),
  letters case-insensitive 13-38 (a-z), accented letters share their base
  letter (`ÀÁÂÃÄàáâãä` = a, `Ç` = c, `ÈÉÊË` = e, `ÌÍÎÏ` = i, `Ð` = d, `Ñ` = n,
  `ÒÓÔÕÖ` = o, `ÙÚÛÜ` = u, `Ý` = y, `Þ` = b, `ß` = s), `Æ` = 0x27, `Ø` =
  0x28, `Å` = 0x29 (after z), `÷`/`×` = 1, `0x1F` and `0xFF` = 0x7F.
- Equal weights then fall back to a byte-for-byte compare (`1020:2020`,
  unsigned): "Memory" and "memory" are **different entries**, capital first.
- Whole strings are compared left to right (no word-by-word rule, no "ignore
  leading article").
- A secondary sorts under its primary with the same compare.

#### Alphabetical separators (flag bit 0 of `[master]`; D `1378:0b0e`, `1378:0be9`)
Run after the build when the box is checked. Walk the chain; for each
**non-secondary** entry compare the weight of its first byte with that of the
previous primary's; when it differs, insert a separator paragraph **before**
that entry, whose text is the first byte upper-cased (`ANSIUPPER`); a first
byte `>= 0x80` maps to the first capital A..Y with the same weight (so `é` ->
`E`, `ß` -> `S`; `Å`, `Æ`, `Ø` and Z stay as themselves). Consequences: each
digit starts its own group (`0`, `1`, ...), all punctuation shares one group
named by the first symbol of its first entry, and the very first entry always
gets a separator. Secondary entries never start a group.

#### Output document and styles
- File > New with `_INDEX.STY` (EXE string 3150); the three styles looked up
  are `Primary`, `Secondary`, `Separator` (EXE strings 3153-3155). The
  built chain **is** the document's text: separator paragraphs, primary
  paragraphs, secondary paragraphs, in that order; then Save As the Index
  output name (required).
- `_INDEX.STY` (ISO): Body Text Times 10 pt; `Primary` Times 10 pt,
  `Secondary` indented 720 twips (the `[algn]` first indent), `Primary` has no indent;
  `Letter` (Times 30 pt, unused by the code, I); `Separator` Arial 14 pt
  bold caps with line rules and 144-twip space above and below; `Title`
  Arial 30 pt; Header and Footer; layout `Standard`, portrait 12240 x 15840
  twips, 1-inch margins, a text frame sized by the sheet's columns, and a **page-frame in the layout holding the text
  "Index"** in the Title style (so the title is not in the document text).
  Details to copy from the sheet itself rather than from this note.

### 5. `<:X` marks in the parser (D, FIELDS_NOTES.md)

`11b8:030a` case `X` -> `1628:0000` / `1628:0115`; the kind byte K and value
word V; code text. Index: K 8, code starts `Index`; TOC: K 11, code starts
`TOC`, form `TOC n "text"` (level 1..9; the corpus contains unquoted and
partially garbled texts such as a level 4 text with an unclosed quote and a `</W>` escape,
read leniently: level, one space, rest; a trailing `"` removed). Neither
kind has a result run, so no end mark; V is `-32768` when inserted in main
text (any kind 8/11 mark), the page index after generation for Index (not
for TOC: all four corpus TOC marks still `-32768`).
`<:T>` (03 `T`) is the legacy end-of-TOC-text mark (section 3). Update All and
Update Fields skip kinds 8 and 11 (FIELDS_NOTES.md).

### 6. Corpus and ISO evidence

- 53 user documents and 3 ISO documents hold `[toc]`; none of the user's SAMs
  holds a generated index document (no `@Primary@` paragraphs); one (user document U1 (the corpus's one generated TOC))
  holds a generated TOC page table (section 3).
- 27 user and 202 ISO files hold `Index` marks (FIELDS_NOTES.md); 4 user
  files hold `TOC n "..."` marks.
- `[master]` in 56 files, mostly the ISO guide chapters with index names.

### 7. `_TOC.STY` (ISO `AMIPRO/STYLES/_TOC.STY`)

Description "Table of Contents - used every time you create a TOC". Styles:
`TOC1` Arial 14 pt bold (flags 16385), `TOC2`-`TOC9` Arial 12 pt, indents
(0,360), (360,720) ... (2880,3240) (`TOC9` is paragraph tag index 11: the
sheet lists TOC9 with function key slot 0), `Table Text` Times 10 pt,
`Body Text` Arial 12 pt, `TOCPG1`-`TOCPG9` Arial (14 pt bold for 1, 12 pt
for the rest), **right alignment**, no indents, and Header/Footer; no
`[toc]` section in the sheet. `[desc]` "Table of Contents - used every time
you create a TOC".

### 8. Open questions for the VM (see the summary)

1. Does the first TOC table into the current document go at the start of the
   text, and what does the document look like (anchor `<:t0>`, what page
   break, which layout: `1588:071e`/`0873`)? Does the source document gain
   TOCn/TOCPGn styles when the output is a separate file?
2. Column widths of the generated TOC table (BOB3: 9080 + 975 twips: is the
   second a fixed width or from the widest page text?), cell margins, row
   heights, frame flags, anchor.
3. Do marks-only runs (no styles listed) apply the Page number, Right align and
   Leader settings to all levels (G says so; the code suggests uninitialised
   per-level flags)? In BOB3 a `TOC 2` mark made a `TOC2` entry with a
   `TOCPG1` page cell; confirm which TOCPG a mark of level n gets.
4. Blank paragraphs in a TOC style (G includes them, D skips short text).
5. What "Software" (BOB3 row 5: one cell, F3 0, no page) means: a joined row
   or an entry without a page?
6. Index: marks in frames, tables, headers and footers: found? On which page?
   Marks inside a footnote?
7. Index Other text: exact joined result for `Index "A" "" " see B"` without
   `#`, and with `#`; is the first character kept?
8. Index separators: digits grouping, symbols; first entry always gets a
   separator; Z handling.
9. Page numbering variants in entries: roman/lettered pages, a chapter prefix,
   restarted numbering after an inserted layout, pages in a master document.
10. Behaviour when the Index output name is blank (error) and when the TOC is
    unchecked but a name is stored; are `[master]` names rewritten on OK?
11. `0x4000000` frame flag: confirm it marks the TOC table frame and that
    regeneration replaces the rows (and what happens to user edits of cells).
12. Warning dialog text when not in display-as-printed state (which view mode).


# Part B: index and TOC entry marks, Mark Index Entry, TOC Entry

## Ami Pro 3.1: Index Entry and TOC Entry marks (research for queue run 12)

Confidence tags: VD = VERIFIED-decompile, VC = VERIFIED-corpus, I = INFERRED.
Addresses: `AMIPROUI.DLL` is "UI", `AMIPRO.EXE` is "EXE" (selector:offset as in the decompile).
No decompiler code is copied; sample contents are not quoted.

### 0. How the two menu items are dispatched (why the command table missed them)

- Edit > Mark Text > Index Entry... (0xb4) and TOC Entry... (0x343) are NOT in the EXE's
  command table at DGROUP 0x46 (checked: neither id is present; 0x1d1 Tools > TOC, Index is). (VD)
- They are modal dialogs owned by `AMIPROUI.DLL`: its data segment holds a table at DS:0x1b0
  (count at DS:0x550 = 116 entries, 4 words each: command, template id, far proc). Entries:
  0xb4 -> template 1109, proc `INDEXENTRYDLGPROC` UI 1350:0000; 0x343 -> template 1115, proc
  `TOCENTRYDLGPROC` UI 1440:0000; also 0x1d1 -> 1094, 0x1d2 -> 1114 (Bookmarks). (VD; the table is
  walked by UI 1028:0000, which calls DialogBox with the proc.)
- The dialogs talk to the EXE two ways: callbacks through a word queue (codes handled by the
  EXE's `1130:0000` switch) and `WM_COMMAND` messages to the EXE's ordinary command table:
  0x23c = insert a new power field from a queued code string (EXE 1648:0000),
  0x23d = replace the code of an existing field (queued: field id, kind, string) (EXE 1648:0904),
  0xe3 = delete a field (queued: id, kind) (EXE 1648:082c; no confirmation),
  0x239 / 0x23a = select next / previous field of any kind (EXE 1648:0d22 / 1648:0f11).
  The kind comes from the code text's first keyword (AMIMACRO classifier): "Index" -> 8,
  "TOC" -> 11 (0xb). So these marks are ordinary power fields (`<:XK,V;CODE>`), no extra syntax. (VD)

### 1. Index entry mark syntax

Stored form: `<:X8,V;Index "primary" [# ]["secondary" ["other"]]>`. A point mark: no end
mark `<:X~8...>` exists anywhere in either corpus (0 of 233). (VC) The field has no result text
and draws nothing; with Show Power Fields it appears as a red code (FIELDS_NOTES). (VD)

Built by the dialog (UI 1350:0695, with the keyword from string 3142 "Index"):
1. `Index`, a space, `"`, the Primary text with trailing spaces trimmed, `"`, a space.
2. If "Page number" is checked: `#` and a space.
3. `"`, Secondary trimmed, `"`, a space (empty secondary: `""` and a space).
4. `"`, Other trimmed, `"` (empty: `""`).
5. The string is then CUT at the last space belonging to the last NON-EMPTY component (a pointer
   `local_e` is moved to the end of the secondary / other only when they are non-empty, then
   `*local_e = 0`). Net result: trailing empty `""` groups are dropped:
   - primary only, page number: `Index "p" #`
   - primary only, no page number: `Index "p"`
   - with secondary: `Index "p" # "s"` (or `Index "p" "s"` without page number)
   - with other: `Index "p" # "s" "o"`; if secondary is empty but other is not: `Index "p" # "" "o"`
   (VD for the builder; VC for `Index "p" #` and `Index "p" # "s"`; no corpus sample has other text.)
- Primary must be non-empty: otherwise message 3143 "Must specify a primary." and nothing is made. (VD)
- Limits: Primary and Secondary combos and the Other edit are limited to 79 characters
  (CB_LIMITTEXT / EM_LIMITTEXT 0x4f); the build buffer is 250. (VD)
- Quoting: there is NO escaping of `"` inside any string. A `"` typed into a field would break
  the entry. (VD) The text goes into the body like all field code: `<` as `<<`, `>` as `<;>`,
  accented / control characters as `</X>` or `<\X>`; discretionary hyphens occur inside entries
  in the corpus (15 soft hyphens in strings). (VC)
- Reading (dialog parser UI 1350:0588 and the generator's parser EXE 1650:038e agree): find the
  first `"`; the primary runs to the next `"`; afterwards scan: any `#` outside quotes (anywhere,
  repeatable) sets the page-number flag; the first following quoted string is the secondary, the
  second is "other"; the generator's parser trims trailing spaces of each string. The keyword is
  matched case-insensitively (3 corpus marks use lower-case `index`). (VD, VC)
- Corpus oddity: 24 marks in one user file read `Index "p" # "s" #` (a trailing `#` after the
  secondary). The tolerant parser reads that as page number = yes; 3.1's own dialog would write
  `Index "p" # "s"`. Probably another writer/macro (INDEXALL.SMM mentioned in the guide). (I)
- Value word V: 0x8000 (printed -32768) for a mark made in the main text. EXE 1378:020f, run in
  pagination, overwrites the value word of every kind-8 mark on a line with the page number
  (decompiled; the exact base, 1-based, is INFERRED from corpus values 1, 2, 5, 20, 22, 24 on the
  ISO files, none 0). So a paginated-then-saved document holds page numbers in V; a fresh one
  holds -32768. Only kind 8 is updated this way (TOC marks stay -32768). (VD for the update, I
  for the base.) Lock/auto-run flags do not apply to Index/TOC (Update All skips them).
- Semantics (guide ch. 25, VC from the shipped user's guide): Primary = main entry; Secondary =
  sub-entry; the page number goes next to the secondary when there is one, else next to the
  primary. "Other" is punctuation/text placed next to the entry or page number (for See / See
  also); a leading comma or space must be typed in Other. Mark is a POINT at the insertion point.

### 2. TOC entry mark syntax

`<:X11,V;TOC n "text">`, n = level 1..9, text in quotes, V = -32768 in the main text. No end
mark; point mark; draws nothing. (VD, VC: 4 marks in one user file, V -32768, shape `TOC 4 "..."`)
- Built by the dialog (UI 1440:019b, string 3205 "TOC"): `TOC`, space, level as decimal, space, `"`,
  text, `"`. Not trimmed. Text limit 240 chars (EM_LIMITTEXT 0xf0). Level control: number box
  (samnumeditbox id 100) range 1..9. OK is disabled while the text is empty. (VD)
- Read at generation (EXE 1630:1f0e): after the keyword, skip spaces, atoi -> must be 1..9 (level-1
  stored), then skip spaces; if the rest starts AND ends with `"` the quotes are removed (inner
  quotes survive), else the rest is taken verbatim. (VD)
- The TOC generator (EXE 1588:0522) scans kind-11 marks in each stream position, takes the
  text, uses the level, and applies it like a heading of that level (page number from where the
  mark is). Marks win over style-based TOC levels for that text (guide ch. 25). (VD for the scan; I
  for details.) The `<:T>` end mark is a separate control (FIELDS_NOTES).
- Default level in the dialog (callback 0x12d, EXE 1588:1f96): the 1-based position of the
  caret paragraph's style in the TOC-level style list (Tools > TOC, Index > TOC Options), else 1.
  Default text (callback 0xd4): the selected text (see 3). The dialog never edits an existing TOC
  mark: OK always inserts a NEW field (message 0x23c). Existing ones are edited through the Fields
  dialog / Show Power Fields. (VD)

### 3. Mark Index Entry dialog behaviour (template 1109, UI 1350:0000)

Controls: 102 Primary combo, 104 Secondary combo (both drop-down, sorted), 106 Page number check,
107 Other check, 108 Other edit, 114 Example box ("Text" group), buttons OK, Cancel, Mark (119),
Remove (117), Next Mark (115), Prev. Mark (116), Next Same (120), Prev. Same (121). (VD)
- Selected text (callback 0xd4, EXE 1128:1d13): the first line's visible characters of the
  selection; control marks are skipped, tabs kept, at most ~253 characters; later lines only
  contribute if the first yielded nothing. Insertion point only: empty string.
- Open: if there is no selected text and the caret is on/at an Index field (callback 0xb1 reports
  field id and kind; 0xb2 returns its code, decoded) the dialog loads that entry: Primary,
  Secondary, Other, Page number checked if `#` found, Other checked if the third string is non-
  empty, Remove enabled. Otherwise it starts a NEW entry: Primary = the selected text with
  trailing spaces trimmed, Page number CHECKED, Secondary and Other empty, Remove disabled. The
  Primary list is filled with the document's primaries. Focus goes to Primary.
- Lists (callbacks 0x112 primaries, 0x111 secondaries; EXE 1630:177b, callback 1630:17e6): the
  distinct, exactly-matching (byte compare, case-sensitive) primaries of all Index marks in the
  document; the Secondary list is filled when the Secondary combo drops down, with the distinct
  secondaries of marks whose primary equals the typed Primary (case-sensitive). The current
  value is then selected in the list. (VD; sorted display because of CBS_SORT.)
- Example box (UI 1350:0861/0954): primary alone: `primary, ##` (", ##" placeholder only if Page
  number is checked) followed by the Other text (Other text only counted when the Other box is
  checked). With a secondary: the secondary goes on a following line indented 5 spaces, then
  `,##` if checked, then Other. Updated on every change of the combos, edit, or checks. (VD, exact
  spacing I)
- Mark (0x77): primary required; builds the string; if the dialog holds an existing field (id)
  it sends 0x23d (replace that field's code), else 0x23c (insert new at the insertion point);
  then CLEARS Primary / Secondary / Other, re-checks Page number, unchecks Other, disables Remove,
  sets focus to Primary; the dialog STAYS OPEN. (VD)
- OK (1): empty Primary -> closes without making anything (returns 0); otherwise same as Mark
  (replaces the loaded field or inserts a new one) and closes. Cancel closes. (VD)
- Remove (0x75): sends 0xe3 for the loaded field (deletes the whole field, no prompt), clears
  the three fields, disables Remove, stays open. (VD)
- Next Mark / Prev. Mark (0x73 / 0x74): repeat 0x239 / 0x23a (select next/previous field of any
  kind, also across streams) until a kind-8 field is selected; that field is loaded (and becomes
  the one Mark/OK replace, Remove enabled). No wrap-around: when nothing is found the call just
  returns and the dialog is left as it was. (VD)
- Next Same / Prev. Same (0x78 / 0x79): the same search, but skipping marks whose primary is not
  byte-equal (lstrcmp) to the Primary currently in the box; Esc stops the search. (VD)
- Insertion: 0x23c collapses the selection (EXE 1590:02a5, text is NOT deleted) and inserts at the
  caret; after insertion the caret sits just after the mark, so repeated Mark presses give a
  run of adjacent marks (corpus: 90 of ~203 marks directly follow another mark and 90 precede
  another; they cluster at paragraph starts). Whether the mark goes before or after a selection
  is INFERRED (caret = the active end) and should be checked in the VM.

### 4. Counts in the corpora (VC)

| | ISO sample corpus (160 SAM) | User corpus (424 SAM) |
|---|---|---|
| Index marks (kind 8) | 202 in 6 files | 27 in 1 file |
| with primary only | 77 | 3 |
| with a secondary | 125 | 24 (all with trailing `#` oddity) |
| with "other" text (third string) | 0 | 0 |
| page number (`#`) | 202 (all) | 27 (all) |
| value word -32768 | 192 | 27 |
| value word = page number | 10 (values 1, 1, 1, 1, 1, 2, 5, 20, 22, 24) | 0 |
| TOC marks (kind 11) | 0 | 4 in 1 file (`TOC 4 "text"`; level 4, en-dash allowed in text) |
| end marks `<:X~8`/`<:X~11` | 0 | 0 |
| strings holding `@`, `~`, soft hyphen, no-break space | `@` 6, `~` 1, soft hyphen 15 | no-break space 1 |

(The keyword is case-insensitive: 3 of the ISO marks use lower-case `index`. ISO 202 = 192 + 10 paged.)

### 5. What the app currently does

`crates/document/src/power.rs` keeps `<:X8..>`/`<:X11..>` as power marks (kind constants INDEX 8,
TOC 11; `flag::MAIN_TEXT` 0x8000); `editing/field_language.rs` classifies "index"/"toc";
`exchange/realise.rs` writes imported index entries as `Index "p"` or `Index "p" # "s"` (note:
no `#` for a primary-only entry, whereas Ami Pro 3.1's own dialog defaults Page number ON) and TOC as
`TOC n "text"` with `"` replaced by `'` (unnecessary: Ami Pro strips only the outer quotes).
No code parses the three strings or the `#` flag yet.

### 6. Open questions for VM experiments

1. Mark placement with a selection: before the selection, after it, or at the caret (which end)?
   Does the selected text stay? (callback 0xd4 text vs 0x23c position)
2. After Mark with a selection, where is the caret and is the selection cleared?
3. Is a mark with Other text but Other box unchecked saved? (the builder reads the edit regardless
   of the check; the example hides it.) Does typing in Other auto-check the box?
4. Exact Example box text (spacing, `,##` vs `, ##`) for primary only and primary+secondary.
5. Exact value word written by a fresh Index and TOC mark in a body, a frame, a header, a table
   cell (0x8000 only in the main text?), and the page-number base after pagination + save.
6. Index generation output: format of the primary/secondary lines, the Other text placement, the
   `#` flag with/without secondary, alphabetisation and merging of equal primaries (generator
   code beyond the parser at EXE 1588:0522 / 1650 not yet read; this is the TOC/Index run's
   core and needs its own decompile pass).
7. TOC level default from the paragraph's style when the style has a TOC level (list order is
   the Tools > TOC Options order); behaviour when the caret is in a frame/table.
8. Does the generator honour marks inside frames/tables/headers? (guide: only main text and
   Flow-with-text / With-para-above frames).
9. Whether a `"` typed into a field really corrupts the entry on regeneration (no escaping seen).
10. Is the 24-mark `Index "p" # "s" #` shape from INDEXALL.SMM, and does 3.1 rewrite it on edit?


# Part C: glossary

## Ami Pro 3.1 Glossary: mapped from the decompile, the guide, and the user's corpus

Confidence tags: VERIFIED-decompile (read in the decompiled AMIPRO.EXE or AMIPROUI.DLL), VERIFIED-guide (user's guide ch. 25, `AP3CH25.SAM`, "Using Glossary"), VERIFIED-sample (a real file in the user corpus), INFERRED.
Addresses are `selector:offset`; segment n = selector 0x1000+8(n-1); DGROUP 1680. Decompiler output is in `../AmiPro-Reference/ghidra-work/out/` (`amipro.c`, `amipro-cmds.c`, `amiproui.c`; my one extra decompile, 14e8:0cb1, is in `amipro-glossary.c`). Nothing was copied into the repo.

### Where the code is (AMIPRO.EXE segment 158 = selector 14e8; AMIPROUI.DLL segment 13d0)

| What | Address |
|---|---|
| cmd 0x1d3 Insert Glossary Record (after the dialog; gets the chosen name) | 14e8:025c, calls `02b8` |
| cmd 0xc7 Mark Glossary Record (after the dialog) | 14e8:115b |
| cmd 0x1d4 set the current glossary data file (load its index) | 14e8:0289, calls `0145` |
| cmd 0x233 the Data File button's command (check the file, then 0x1d4, then write the INI) | 14e8:0000 |
| UI callback 0x9d: Ctrl+K quick expansion, or the list of names for the dialog | 14e8:0ee5 (dispatch in 1130:0000, case 0x9d) |
| UI callback 0x9c: the current data file's name | 14e8:10d7 |
| load the data file, keep an index in memory (`DAT_1680_1204` is the handle) | 14e8:0145 / 0640 / 055c (055c reads the INI at the first use) |
| read the index (footer) of a data file | 14e8:0745 |
| build the in-memory index (32 entries a block of 0x400 bytes) | 14e8:0871, 0d07 (get entry), 0e0a (parse a block) |
| write a fresh index into the file | 14e8:0a77 (entry line writer 14e8:0cb1) |
| binary search by name, case-insensitive (`LSTRCMPI`) | 14e8:06ce |
| insert a record's text at the caret | 14e8:0422 (record text from 1388:0e71, parsed and inserted by 13b0:2786) |
| create a new data file | 14e8:1867 (template copy 14e8:1a8e) |
| open a file, retrying a sharing violation | 14e8:1c2a |
| duplicate-name compare | 14e8:17d4 |
| the shared "word at the caret" routine | 1380:0000 (word class test 1380:0579, collect 1380:0418) |
| the data file reader (it is the merge records-file reader) | 13a8:0000 open, 0729 read delimiters, 07a3 first block, 04a4/04df next record, 015f get a field, 0204 split fields |
| record size limit (INI `GlossarySize`) | 13a0:0dd8 |
| dialog procedures | AMIPROUI `ADDGLOSDLGPROC` 13d0:0252 (dialog 2008, Mark), `GETGLOSNAMEDLGPROC` 13d0:04ec (dialog 2010, Glossary Data File), `GLOSDLGPROC` 13d0:0000 (dialog 1117, Insert) |

### 1. What a glossary data file is

- **It is a merge-style data file stored as an ordinary SAM document** (VERIFIED-decompile, VERIFIED-guide, VERIFIED-sample). Guide ch. 25: line 1 of the body = two delimiter characters (field delimiter then record delimiter), line 2 = the two field names ("ID title" and "data title") ending with the record delimiter, then one record per paragraph: `ID<fielddelim>text<recorddelim>`. Ch. 28 describes the same layout for merge data files. The glossary code reads it with the same reader as Merge (13a8:0000 and friends); that reader takes the delimiter line as 2 characters (field + record delimiter) or 1 character (the record delimiter is then the paragraph end, CR) and fails with string 3005 "Data file is not in valid format." for anything else (13a8:0729) (VERIFIED-decompile).
- **Real sample** (VERIFIED-sample): `user glossary file U2` (446 records, a macro-function glossary). Body: `|~` (field delimiter `|`, record delimiter `~`), a header row of two field names, then records whose names are function names with parentheses, one record per paragraph, blank line between paragraphs (as any SAM), `<[>` escapes for `[` in text. `another user document` in the same folder is NOT a glossary data file, just a document with the word in its name.
- **Format of the text**: a record's text is raw SAM body text, so it holds formatted text: character marks, `@Style@` paragraph tags, tabs, several paragraphs (guide: "records can be several words or multiple paragraphs long"; "type the text exactly as you want it to appear"). Insertion extracts the raw bytes after the first field delimiter up to the record delimiter (1388:0e71), appends `\r\n>\r\n` to make a mini SAM body, and parses it with the normal SAM body parser and inserts the parsed paragraphs (13b0:2786) (VERIFIED-decompile). So a record can carry frame anchors etc. only as far as a pasted SAM body can; frames themselves cannot be in a record (no `[frm]` sections; INFERRED).
- **Default name/location**: none. AMIPRO.INI `[AmiPro]` key `Glossary=` holds the path of the current data file (section "AmiPro", key "Glossary", file "AmiPro.ini": strings at AMIPROUI DS 0x184 and string id 2212 "Glossary"; AMIPRO.EXE reads it at 14e8:055c) (VERIFIED-decompile). It is empty by default (neither of the VM's AMIPRO.INI files has it). The guide only suggests a name such as `GLOSSARY.SAM`. A relative name without a drive gets the documents path prefixed (`DAT_1680_2ba3`, 115b) (VERIFIED-decompile; the exact meaning of `DAT_2ba3` is INFERRED). The value may be in a `@n` indirect form when the app is in a workgroup mode (`DAT_1680_2b9a` set), which I did not trace (INFERRED, ignore).
- **The hidden index footer** (VERIFIED-decompile, VERIFIED-sample, written by 14e8:0a77, read by 14e8:0745). The first time a data file is used, Ami Pro appends to the file (mode "at", so after the SAM's own `[Embedded]` trailer):
  ```
  [Embedded]\r\n00023815\r\n        <- the SAM's own trailer (unchanged)
  >\r\n
  [gloidx] 446\r\n                    <- literal "[gloidx]" (string at DGROUP 0x11f8) and the record count
  NAME 1322\r\n                    <- "%s %ld": name, then the file offset of the START OF THE RECORD'S PARAGRAPH
  ActivateApp() 1354\r\n              (the offset includes any `<:#n,n>` cache control in front of the name)
  ...
  00023840\r\n                        <- "%08ld": the offset of the "[gloidx]" line (10 bytes with CRLF)
  ```
  Reading: seek to size-10, `%ld` gives the index offset, seek there, read `[gloidx] N`, then N lines read 32 at a time (14e8:0e0a splits each line at its LAST space, so names may contain spaces; the number is the offset). Entries are sorted case-insensitively (LSTRCMPI order, the same order as the in-memory binary search at 06ce; the sample orders `ViewPreferences`, `ViewPreferences()`). The index is rebuilt (14e8:0640) when it is missing or does not parse, which is the case after Ami Pro itself saves the file (a normal save rewrites it without the footer), so the footer is a cache; a SAM reader that stops at `[Embedded]` never sees it. Staleness is judged only by the index offset recorded in the footer changing (02b8, 0ee5), not by a timestamp. Index build rules (0a77): the name is field 0 of each record with formatting codes and leading tabs removed (13a0:0e64); a name longer than 27 characters is cut to 27 and the first such name is reported once with string 3047 "%s: is too long a key for Glossary."; a duplicate name is ignored (the tree insert at 1598:00ee refuses it, so the first record with a name wins and the count excludes duplicates); an empty name is shown as `***` in the dialog's list. The first record (the field-names row) is consumed as the header, not indexed (INFERRED from the reader's open sequence at 13a8:07a3).
- **Record size limit**: INI `[AmiPro]` `GlossarySize` (a count of KB; `GlossarySize=2` is written by the installer's INI and also appears in `a user document`). 13a0:0dd8: missing or 0 gives 2048 bytes; 1 to 32 gives that many KB; above 32 gives 32000 bytes. Longer records fail with string 3109 "Record size exceeded. Increase AmiPro.ini variable GlossarySize" (13a8:0204). The same buffer is the merge reader's record buffer (INFERRED: merge uses the same limit).

### 2. Mark Glossary Record (Edit > Mark Text > Glossary..., cmd 0xc7, 14e8:115b; dialog 2008)

Dialog (VERIFIED-decompile, ADDGLOSDLGPROC): "Record name" edit (limit 27 characters: EM_LIMITTEXT 0x1b on control 25300), "Data file:" (shows the current INI `Glossary`, shortened to fit), "Record Text" group showing the FIRST LINE of the selection (guide), and a "Data File..." button (25585, id 0x63f1) that opens dialog 2010 "Glossary Data File" (one edit, limit 80 characters, no file browser in 3.1; the guide's drive/directory list is a 3.0 leftover or the dialog's description only). OK posts the name, the file name, and a flag to the EXE.

Handler rules (115b), in order (VERIFIED-decompile):
1. Does nothing, silently, if the name is empty, the file name is empty, or there is no selection (the selection start equals its end).
2. A file name without a drive letter gets the documents path (`DAT_2ba3`) in front.
3. Opens the file "r+b" (retrying five times, about 0.75 s apart, on a sharing violation, errno 1 or 13; then string 3031 "%s is currently protected. You cannot save any changes to it." (1c2a)).
4. **If the file does not exist (errno 2) it is created without asking** (14e8:1867): the header is copied from the INI template `defstyle` (default `_DEFAULT.STY`; strings 1291, 1292; "Cannot open %s", 3011, if the template is missing) up to and including its `[edoc]` line, forcing a `[sty]` section as the third line (14e8:1a8e), then the body gets `~|\r\n\r\n` (field delimiter `~`, record delimiter `|`) and `id~text|\r\n\r\n` (field names "id" and "text"), and the file is truncated there. So a new glossary file uses `~` and `|` and the names `id`/`text`, and carries the default style sheet's styles.
5. First line must be `[ver]`; `[encrypt]` gives string 3148 "Cannot do that command to an encrypted file."; anything else gives string 3119 "Not an Ami Pro document".
6. It skips to `[edoc]`, then reads paragraphs with the SAM paragraph reader (1118:0e55). The first paragraph's first two characters are the field and record delimiters. For every following paragraph it compares the record name (case-insensitive, ANSILOWER, 14e8:17d4) with the start of the paragraph's text and requires the next character to be the field delimiter. **A match refuses the record with string 3165 "Glossary name already in use."; there is no overwrite or replace.** (Side effect: in a file made by step 4 the name `id` is already taken by the field-names row; INFERRED from the loop.)
7. Appends: positions at the end of the body (backing up 3 bytes over the `>\r\n` when more follows), truncates the file there (which also removes `>`, `[Embedded]`, the offset, and any `[gloidx]` footer), then writes `name` + field delimiter, then the selection as SAM body text with Ami Pro's own body writer (11b0:1405, mode 2: tags, character marks, escapes, from the selection's first character to its last, so partial first and last paragraphs are exact), then moves back 4 bytes (before the final paragraph break) and writes the record delimiter followed by `\r\n\r\n` (a record delimiter of `<` or `>` is written doubled). The file is left WITHOUT the closing `>` and `[Embedded]` trailer; it is not a complete SAM until Ami Pro saves it or the index step (0a77) appends `>\n[gloidx] ...`. (INFERRED: that the trailer and the first `>` are lost; read from the truncate and write sequence.)
8. If the file is the current glossary file, the in-memory index is dropped and rebuilt (14e8:0145 via 0640 and 0a77), so the footer is written at once. The Data File button (cmd 0x233) validates the file with the merge reader (3005 if invalid), tells the app the new current file (cmd 0x1d4), and **writes the upper-cased path to `AmiPro.ini` `[AmiPro]` `Glossary=`**, i.e. choosing a data file in either dialog changes the global current glossary file, not just this record.

### 3. Insert Glossary Record (Edit > Insert > Glossary Record..., cmd 0x1d3, Ctrl+K)

- **Dialog 1117** (GLOSDLGPROC, VERIFIED-decompile): "Glossary items" list box (id 0x6306, LBS_NOTIFY | LBS_SORT, so the list is sorted by Windows' list box order, not by the index order), "Data file:" label (current file, shortened), "Data File..." (same command 0x233, the list is refilled afterward), Insert, Cancel. The first item is selected at open; double-click = Insert. Insert is disabled when the list is empty. Insert returns the selected name to the EXE (cmd 0x1d3 handler 025c calls 02b8).
- **The dialog does not open at all** when there is no usable data file (the callback returns 0 and the dialog ends at once: `GLOSDLGPROC` init 13d0:0167 with 0ee5's failure path). So with no `Glossary=` set, Insert Glossary Record does nothing; the file has to be set through Mark Glossary's Data File first (INFERRED from the code; check in the VM).
- **Insertion (02b8 then 0422)**: looks the name up case-insensitively in the index; if it is not found, nothing happens and no message appears. Otherwise reads the record's raw SAM text (see section 1) and inserts the parsed paragraphs at the caret, with their character formatting and paragraph styles as the SAM parser reads them; the document is marked changed; one undo/redo record is made through the undo module (1208:0558/05da, as other insertions; INFERRED that one Ctrl+Z removes the whole insertion).
- **Ctrl+K without the dialog** (guide: type the record name, press Ctrl+K; VERIFIED-decompile at 14e8:0ee5 with flag set): takes the word at the caret using the shared word routine (1380:0000 mode 1: a run of letters and digits only (class test 1380:0579; punctuation and spaces end it), at most 63 characters, the word ending at or containing the caret, selecting it); if the name is in the index, the selected word is deleted and replaced by the record's text (0422 with the "replace selection" flag), and no dialog is shown. If the word is not a record name (or there is no word), the selection collapses to the caret and the Insert dialog opens with the full list. Consequences (INFERRED, test in the VM): names with spaces or punctuation (a name with parentheses) cannot be expanded by Ctrl+K as typed; they can only be inserted from the dialog.
- The toolbar icon "Insert glossary record" (help string 467 "Insert a glossary record into the current document") opens the same dialog; the status help for Mark is string 199 ("Use selected text as a glossary record in the glossary data file you specify").

### 4. Interaction with merge data files

- Shared reader: the glossary's file open/parse/field split/record split are the merge records-file reader (13a8:*), including the delimiter line rules, the "Data file is not in valid format" error (3005), and the `GlossarySize` record buffer. A merge records file with the field in column 1 as a name could be used as a glossary (and vice versa) as far as the reader goes (INFERRED; untested).
- Differences: the glossary needs only two fields (the name, then everything up to the record delimiter, which may contain further field delimiters since only the first is searched), keeps formatting, and has its own `[gloidx]` footer and INI key; merge uses `[files]`/Merge dialogs and the document's records-file field (string 3209 "This document already has a records file assigned to it", 3225 "There are merge fields...").
- `FRAMGLOS.SMM`/`FRAMGLOS.SAM` on the ISO (macros) are an unrelated sample macro (Frame Glossary) that keeps frames in its own SAM and has its own `AMIPRO2.INI` entries; its data file is empty. Not the built-in feature.

### Open questions for VM experiments

1. Fresh INI, no `Glossary=`: does Insert > Glossary Record do nothing (no dialog)? Does Edit > Mark Text > Glossary... with no selection stay enabled/greyed?
2. Mark a selection with a style (a heading, a Body Text bullet) and character formatting, partial first and last paragraph, a table or frame in the selection: what bytes are written? Does the file still end with `>`/`[Embedded]`? Does it re-open in Ami Pro right after a Mark (before any index)?
3. Insert a multi-paragraph record with styles (`@Def List@`) that exist/do not exist in the target document: styles added or mapped to Body Text? Does the first record paragraph take the paragraph style of the paragraph at the caret?
4. Ctrl+K: caret directly after the word; caret inside the word; a selection; names with digits; names with `()`/space; the case of the name; Undo after Ctrl+K (one step restoring the typed name?).
5. Is the `[gloidx]` footer written even by Insert alone (touching a file the user only reads)? Is the file's date changed? Does a read-only file (`user glossary file U2` is mode 0400 here) show an error or silently work without the index?
6. Name length: type 28+ characters into the Mark dialog (27 limit), a hand-made record name longer than 27 (message 3047) and duplicate names (first wins?).
7. Does Edit > Insert's list order match the file or the list box's sort (e.g. `_x`, `a`, `B`, accented letters)?
8. Delimiter line with one character (CR as record delimiter): Mark's `%c\r\n\r\n` write with a CR delimiter.
9. The Data File dialog: only an edit? (the guide mentions drive/directory selection.) Is the upper-cased path stored in `AmiPro.ini` after choosing a file, and is a not-yet-existing file accepted (cmd 0x233 validates with the merge reader first, so a missing file should give 3005) while Mark's step 4 creates files?

# Found during the run (2026-11-03)

- **TI3** (`IN/TI3.SAM` on G18's header "Page N", TOC marks only, no
  `[toc]`) → **TI3T**:
  - The TOC page's header and the first body page's header both show
    "Page 1": the 0x02 break makes page numbers start again after it.
  - Without `[toc]` the TOC Options defaults are not used. Each mark's
    page follows its text with no separator ("FirstPage 1", the page-number
    field's own leading text).
  - The `[desc]` and `[master]` sections changed; the layouts did not.
- **GLO0IN** (`IN/GLO0IN.SAM`, GLO0 after Ami Pro chose it as the data
  file, before Mark): choosing the file alone appended `>` and a
  `[gloidx] 1` index after the text's `>`. On Mark, Ami Pro cut at the
  text's first `>` line.
- **Numeric table cells** (TNUM1 and the decompile; for the table-data
  run, TQ2):
  - A cell whose only paragraph is a number (digits, a sign, one decimal
    symbol) is numeric. Ami Pro keeps its value as a double and shows it
    through the cell style's `[nfmt]` (General drops trailing zeros).
  - It writes the value with the style's decimals (`5.00`). Typing a
    number into a cell with no alignment mark adds Right (`<+A>`), and the
    cell record's F3 0x38 bits are the first paragraph's alignment.
  - Corpora: 452 numeric-looking ISO cells and 1,298 user cells, 553 of
    them with two decimals. The app shows them as stored.
- **Twins** (`IN/ATI1.SAM`, `ATI1X.SAM`, `ATI3.SAM`, `AGL1.SAM`, made by
  `contents_twins`):
  - ATI1 shows as TI1T2. Ami Pro's TOC, Index regenerates its TOC in place
    (`OUT/ATI1R2.SAM`: one table, flag `0x41800C4`). Before TQ5 (ATI1 had
    no `[toc]`) Ami Pro kept only the marked entry (`OUT/ATI1R.SAM`).
  - ATI1X shows as TI1IDX.
  - ATI3's TOC stands at the caret.
  - AGL1's records are listed and inserted by Ami Pro's Ctrl+K, and the
    file is left unchanged.
