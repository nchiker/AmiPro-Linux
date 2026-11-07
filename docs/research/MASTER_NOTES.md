# Master documents in Ami Pro 3.1

**Written:** 2026-11-03, for the Phase 5 queue's run 13 (`master`;
`docs/playbooks/FIELDS_AND_REFERENCES.md`, "Run 4").

**Sources.**
- The decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`, with the dialog and
  string resources (`tools/ne_resources.py`). Where the decompile hid an
  argument, the unpacked EXE was read as disassembly.
- The 3.1 user's guide, chapter 26 ("Working with Multiple Documents").
- Ami Pro 3.1 fixtures made in the XP VM on 2026-11-03: MD1, MD2, MD3
  (below), kept in `../AmiPro-Reference/vm-lab/OUT/MD*`.
- Both corpora (counts only).

Findings are in our own words; no decompiler output is copied. Tags:
**D** = decompile, **R** = resources, **G** = guide, **V** = VM fixture,
**C** = corpus, **I** = inferred.

Addresses are `segment:offset` (segment n = selector 0x1000 + 8(n−1)).

## 1. What a master document is

- Any document whose `[master]` section lists files. It is "the source
  document" in the guide's words. It need not be one of its own files, but
  may be (D, V MD1). (G: "You can use a new Ami Pro document or any of the
  documents you want to include".)
- Its files ("members") are separate SAM documents. Ami Pro never joins
  them into one file (G).
- A member knows it is a member only through its own `[sequence]` section,
  which a master run writes into it (D `1660:023e`, V MD2).
- Three things walk the members (D): Tools › TOC, Index (`1568:0000` →
  `1568:0466`, per file `1568:0976`), File › Print (`1238:0000` →
  `1568:0af3`, per file `1568:0c93`), and nothing else (no Find, Spell
  Check, Doc Info, Export). The guide says the same: numbers run on across
  the files only when the master is printed or a TOC or index is made for it.

**In the files.** User's corpus: `[master]` in 50 of 424 SAM files, one of
them with a list (three files); no `[sequence]`. ISO corpus: `[master]` in
56 files, none with a list (the user's guide chapters keep only their TOC
and index output names); `[sequence]` in 59 (the guide's chapters were
members of a master on the authors' machines). So "the user's guide is a
master document" holds only as members: the source document is not on the
ISO.

## 2. File › Master Document… (command 0x92; `MASTERDLGPROC` `1428:0000`)

The EXE has no handler for 0x92; the DLL's dialog table runs template 1175
(D, R). Menu: File › Mast**e**r Document…, between File Management and
Merge in both File menus; help line "Create one table of contents and
index for multiple Ami Pro documents." (R).

**Controls** (R): Directory text, Files list (the current directory's
documents), Directories list, `>> Include >>`, Remove, "Master doc files"
list, "Selected doc path:" (the directory of the selected member), OK,
Cancel, Options….

**Rules** (D unless marked):
- **A member gets a question instead of the dialog.** If the document has
  a `[sequence]`, a Yes/No box asks "This document is part of a Master
  document. Should it be considered a single document?" Yes drops both
  `[master]` and `[sequence]` and marks the document changed; either answer
  closes the dialog without showing it (D, V MD2: Yes put MB's header
  page number back to 1; the saved copy had no `[sequence]`).
- **Include** makes a full path from the directory and the selected name,
  lower-cases the whole path, and inserts it before the selected member,
  or at the end when none is selected. No duplicate check, no existence
  check (D; V: `l:\out\md2\ma.sam`).
- **Remove** deletes the selected member.
- **OK** replaces the list (command 0x11c, `1660:039c`). OK with an empty
  list removes the list but keeps the output names. Cancel changes nothing.
- **Options…** opens Master Document Options, which changes the record at
  once (Cancel in the master dialog afterwards does not undo it).
- Every change marks the document changed.
- The menu item is disabled in some states (`DAT_223d`, `DAT_21b5 & 0x10`;
  I: no document, or read-only or busy).

## 3. Master Document Options (command 0x118, template 1176)

The same procedure as Tools › TOC, Index (`MASTEROPTSDLGPROC` `1428:032e`;
`TOC_INDEX_NOTES.md` §2) with a larger template (R): Table of Contents
(Generate TOC, Output file, Directory, Directories), Index (Generate index,
Include alphabetical separators, Output file, Directory, Directories), TOC
Options….

- The Generate boxes are not stored: each is checked when its output name
  is stored (D).
- **Under this command a blank TOC name is an error** (string 3152), unlike
  Tools › TOC, Index on a single document, where a blank name puts the TOC
  into the document (D). An index name is always required (3144).
- **An output file may not be the master itself** ("Output file cannot be
  the master document.", 2340; compared without case) (D).
- A name without a drive takes the directory list's directory; a missing
  extension gets `.SAM` (D, G: "Ami Pro automatically adds a .SAM
  extension"). Output names keep the case typed (V).
- TOC Options… edits `[toc]`. **If TOC Options is never confirmed, no
  `[toc]` is stored and the TOC comes out empty** (V MD2: the TOC output
  was a table with no rows; MD3, with TOC Options confirmed, had all six
  entries).

## 4. `[master]` and `[sequence]` in the file

### `[master]` (writer `1660:0000`, reader `1660:00fe`)

```
[master]
	TOC output file        as typed, may be empty
	Index output file      as typed, may be empty
	flags                  bit 0: Include alphabetical separators
	byte count             sum over the files of (path length + 1); 0: none
	file count
	file 1                 full, lower-cased path
	…
```

- V MD2: `L:\OUT\MD2\MTOC.SAM`, `L:\OUT\MD2\MIDX.SAM`, `1`, `54`, `3`,
  `l:\out\md2\ma.sam`, `mb.sam`, `mc.sam` (3 × 18 = 54). C: the user's one
  list has 126 for three 41-character paths.
- Written whenever the record exists, after `[toc]`, before `[sequence]`.
- The reader reads no file when the byte count is 0, whatever the count
  says; it stops early at a line it cannot read.
- Names are never checked at open.

### `[sequence]` (writer `1660:0748`, reader `1660:05fb`, built by `1660:054d`)

```
[sequence]
	A        page base: the pages of the members before this one
	F        footnotes before this one
	0        always 0
	N1       Seq counters that follow
	N2       style counters that follow
	value    N1 entries, then N2 entries: a number line, then a name line
	name
	…
```

- **A** is the page number shown on the last page of the members before
  this one (built through `1588:1951` → `1308:0026`, the routine that turns
  a page into the number it shows). A member's shown page numbers are its
  own plus A, everywhere: page-number fields, the TOC, the index (D). V
  MD2 and MD3 (three two-page members): MB `A = 2`, MC `A = 4`; MB's header
  shows 3 and 4, MC's 5 and 6, printed and on screen. V MD1, whose page
  numbers start on page 2 (`<:Pi2,1,Page >`), gave `A = 1` and `2`: the
  members' own numbering applies, and the guide warns against a start page
  or leading text in members.
- **F** is the number of footnotes in the members before. At load, Ami
  Pro's footnote starting number becomes F + 1, over the file's own
  `[fopts]` (D `1058:0000`). V MD3 printed footnotes 3 and 4 in MB, 5 and 6
  in MC.
- **The Seq list** (N1): each Seq power field's name and the highest value
  seen in any member before (names without case). A Seq with no earlier
  occurrence in its own member continues from it (D `1640:03c3`,
  `1660:09df`).
- **The style list** (N2): each numbered style's counter at the end of the
  member before (style names with case), counted by Ami Pro's numbering
  rules: an outline-level style by its reset options, any other numbered
  style by consecutive paragraphs, reset to 0 when another style comes
  between (D `10c8:2b98`). The first paragraph of a numbered style in a
  member continues from the carried value when nothing before it in that
  member resets it (D `1200:19xx`). V MD2: members ending in Body Text
  carried 0 for every list style; MD3, ending in one Number List
  paragraph after Body Text, carried `1` for Number List.
  The list holds every numbered style of the document: V MD2's
  `Bullet`, `Bullet 1`, `Number List`, `Footnote`.
- The first member gets no `[sequence]` (V MD2: MA has none).
- **A member opened alone keeps its stored values.** Nothing recomputes
  them when an earlier member changes; a run must be redone (G, "Updating a
  master document").

## 5. A master run (TOC, index, or print)

**Common steps** (D `1568:0466`, `0976`, `0af3`, `0c93`):
1. The master is saved first if it has changes.
2. Its list is taken for the run; the master's `[toc]` settings apply to
   every member (members' own `[toc]` is ignored).
3. Each member in list order is opened in the same window, replacing the
   one before (Open flags 0x81, plus 0x200: no auto-run macro). Its stored
   `[sequence]` is replaced by the carried one.
4. The member is worked (scanned or printed), **saved**, and the carried
   record is built for the next one.
5. TOC1–9 and TOCPG1–9 are removed from each member after its scan
   (`1588:2013`).

**TOC and index** (`1568:0976`, finish `1568:0170`):
- Update All Fields runs on each member first.
- Index marks get the shown page (with A). TOC entries likewise.
- At the end the TOC goes into a new document on `_TOC.STY` saved under
  the TOC output name, and the index into a new document on `_INDEX.STY`
  saved under the index name; both stay open (D, V).
- V MD3 (TOC and index, alphabetical separators): TOC rows `MA chapter
  one … 1`, `MA section two … 2`, `MB chapter one … 3`, … `MC section two …
  6`; index `C` / `Common, 2, 4, 6` / `T` / `Topic MA, 1` / `Topic MB, 3` /
  `Topic MC, 5`. The members' marks hold their member's own page (1, 2),
  not the shown one.
- The master need not be open at the end: V MD2 left MTOC.SAM in the
  window (the master and the members had been replaced and saved).

**Print** (`1568:0af3`, step `1568:0c93`):
- Order: the TOC output file (when named), the members, the index output
  file (when named) (D, V MD3: five print jobs in that order).
- Each file prints through its own printer setting (V: the TOC, members,
  and index printed to XPS, which their `[prn]` named, while the master's
  was PS File).
- Each file prints all its pages whatever range the Print dialog gave;
  Update fields applies per file.
- Each member is saved after printing, so printing rewrites `[sequence]`.
- No Print or Print Options control mentions master documents (R).

**Missing or unreadable files** (D, V MD2):
- Ami Pro's own Open message appears ("MC.SAM does not exist. Do you want
  to create it?", the File Open question). No ends the run; the TOC and
  index are built from the members scanned so far (V: the index lost MC's
  entries).
- A missing TOC output file at print time asks the same question; No
  stops the print (V MD3).
- Files are found by their full path only; there is no relative lookup
  and no check of date or size.

**Pitfall seen in the VM.** With the master in its own list (MD1), the
member open found the master's file protected (already open): "…is
currently protected. You cannot save any changes to it." The master kept
its uncached marks and its index entry had no page. The guide's method,
a separate source document, avoids it.

## 6. Open points

1. Seq counters across members were read from the decompile only (no
   fixture). The run reproduces the rule and may add a fixture.
2. A numbered style that starts a member (no intervening style) continues
   from the carried value (decompile); the fixtures had intervening
   styles. A twin can check it.
3. Ami Pro's paths are DOS paths: on Linux, `[master]`'s files are
   resolved by the run's rule (decision).
