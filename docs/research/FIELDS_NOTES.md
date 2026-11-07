# Fields in Ami Pro 3.1: evidence (2026-10-20)

Mapped for the Phase 5 queue's run 4 (`fields`,
`docs/playbooks/FIELDS_AND_REFERENCES.md`): dates and times, Doc Info,
and power fields. Decompiler output stays outside Git; these are findings
in my own words, routines by address. Evidence levels as in
`SAM_FORMAT_NOTES.md`: VERIFIED (decompile), VERIFIED (fixture) for the XP
VM, VERIFIED (manual) for the 3.1 user's guide (ch. 5, 23, 32), VERIFIED
(corpus), and INFERRED.

VM fixtures (in `../AmiPro-Reference/vm-lab/`, outside Git): FD1 (Insert
Date/Time, each choice), FD2 and FD2B (Doc Info, its fields, Insert Doc
Info Field's 22 entries), FD3 and FD3B (28 power fields typed into Insert
Power Fields; Lock, Auto run; typing in and formatting a result), FD4 and
FD5 (hand-written `IN/` files: all 21 date formats on 2 March 2005, the
time, stored times moved to a morning hour and to January).

## The commands and dialogs

| Ami Pro | Command | Dialog (`AMIPROUI.DLL`) | Notes |
|---|---|---|---|
| Edit › Insert › Date/Time… | `0x9e` (the DLL's own table; dialog 1110) | `INSVARDLGPROC` `1138:0000`; OK posts `0xa5` (`1560:0000`, `1560:0374`) | Insert: Today's date, System date, System time, Date of last revision, Date created; Style: 21 formats, each shown as a sample of today; System time dims the list. The choice is kept in AMIPRO.INI (`datetype`, default Today's date; `datestyle`, default format a) |
| Edit › Insert › Doc Info Field… | `0xb7` | 1123, `DESCMARKDLGPROC` `1338:0000`; inserts by `0xa7` (`1560:01ef`) | Modeless: Insert at the caret, again and again, until Cancel |
| File › Doc Info… | `0x6d` | 1013, `DESCRIPTIONPROC` `1328:0000` (OK `0531`, Update `05c7`); Other Fields… 2007 `OTHERFLDSDLGPROC` `1420:0000`; Rename Fields… 1122 `RENFLDSDLGPROC` `1330:0000` | Below |
| Edit › Power Fields ▸ Insert…, Update, Update All, Next Field, Prev Field | `0x238`, `0x23b`, `0x10b`, `0x239`, `0x23a` | Insert Power Fields; Save Power Field | Below |
| View › Show Power Fields / Hide Power Fields | `0x10a` | | Codes in red between `<` `>` in place of results (FD3) |

VERIFIED (decompile) for the wiring; VERIFIED (fixture) for every dialog's
contents.

## Dates and times

### Stored form

| Insert Date/Time choice | In a SAM (FD1) | Internal object (`11b8:030a`) | Shows |
|---|---|---|---|
| Today's date | plain text, formatted once | | the date of insertion, for ever |
| System date | `<:Dx>` | 3 `D` x, two filler bytes | today, in format x |
| System time | `<:b<* >>`, `<:b<*!>>` | 3 `b` byte | now; the byte is the list's selection index, stored raw and ignored |
| Date of last revision | `<:rx>` | 3 `r` x | the last save (`[desc]` value 6), format x |
| Date created | `<:kx>` | 3 `k` x | the creation time (`[desc]` value 8), format x |

The writer (`11b0:0660`) writes `<:`, the letter, the argument bytes
escaped (a byte below 0x20 other than a tab as `<*X>`, X the byte plus
0x20; `>` as `<;>`; `<` as `<<`), and `>`. VERIFIED (decompile, fixture).
The objects are one unit to every text walker (the writer, the line
builder, Find's matcher `1368:1807`, the word helper `1380:0579`): Find
never matches a date's shown text, and copied plain text carries the shown
text (`11f8:0048`). VERIFIED (decompile).

Corpus (user's / ISO, marks): `<:Db>` 56 / 0, `<:Dd>` 7 / 0, `<:DB>` 0 / 1,
`<:rd>` 9, `<:ra>` 1 / 1, `<:rb>` 1, `<:b…>` 15 / 0, `<:k…>` none. In 43
user files and 1 ISO file, mostly alone at the top of letters.

### The 21 formats (`1308:0619`)

The formatter switches on the letter with 0x20 set; an upper-case letter
passes the result through `AnsiUpper`. Month and weekday names are full
English names from AMIPRO.EXE's string table (never abbreviated, never
from Windows). VERIFIED (decompile); every letter VERIFIED (fixture) FD4
(2 March 2005, a Wednesday) and FD1 (20 October 2026, a Tuesday):

| List | Letter | 2 March 2005 (FD4) | Rule |
|---|---|---|---|
| 0 | a | 03/02/05 | numeric, two-digit year (below) |
| 1, 2 | b, B | March 2, 2005; MARCH 2, 2005 | month day, year |
| 3, 4 | c, C | 2 March 2005 | day month year |
| 5, 6 | d, D | Wednesday, March 2, 2005 | weekday, month day, year |
| 7, 8 | e, E | March 2 | month day (day month when the date order is day first) |
| 9, 10 | f, F | Wednesday March 2 | weekday month day (weekday day month, day first) |
| 11 | g | 03/02 | numeric month and day (day/month when day first) |
| 12 | h | 03/02/2005 | numeric, four-digit year |
| 13, 14 | i, I | 2. March | |
| 15, 16 | j, J | 2. March 2005 | |
| 17, 18 | k, K | 2005 March 2 | |
| 19, 20 | l, L | March, 2005 | |

Any other letter shows as a. The word formats keep their order whatever
the Windows date order, but e and f. Days have no leading zero in the word
formats; years have four digits.

**Numeric formats** read WIN.INI `[intl]` (`1018:0275`, again on
`WM_WININICHANGE`): `iDate` (0 month/day/year, 1 day/month/year, 2
year/month/day), `sDate` (the separator, default `/`), and `iLzero`, which
is **the number format's leading zero, not a date setting**: when set,
every part is two digits (03/02/05); when clear none is padded and the
two-digit year is the year modulo 100 unpadded (2005 shows `5`).
VERIFIED (decompile); the padded form VERIFIED (fixture) FD4 (the guest
has `iLZero` 1, `iDate` 0, `sDate` `/`, as XP's US defaults).

**The time** (`1280:021f`): the hour space-padded to two places, `sTime`
(`:`), two-digit minutes, a space, and `s1159` before noon or `s2359`
from noon (AM, PM): ` 9:05 AM` with a leading space (VERIFIED (fixture)
FD5), `11:25 AM`. `iTime` 0 gives hours 1–12 (0 shows as 12); `iTime` 1
gives 0–23 and still adds the suffix. No seconds; `iTLZero` is not read.
VERIFIED (decompile).

### The time base

Ami Pro's C runtime converts the DOS clock with its default time zone when
`TZ` is unset: US Pacific, 28,800 seconds, with daylight saving by the US
rule of its day (`1020:06bc`, `1020:1798`, `1020:3d2c`). So a stored time
(`[desc]` values 6 and 8, and every power field's seconds) is the PC's
wall clock read as Pacific time: **the wall clock plus 8 hours, or plus 7
in US summer time**, as seconds since 1970. VERIFIED (decompile);
VERIFIED (fixture): FD2's creation at 11:23 on 20 October 2026 is stored
as 18:23 UTC; FD5's stored 16:05 UTC on 20 October and 17:05 UTC on 15
January 2005 both show 9:05. To show a stored time as Ami Pro does,
subtract 7 or 8 hours by that rule and read the result as UTC; to write
one, do the reverse. **The app writes real UTC today** (`description` in
`crates/ami-formats/src/sam.rs`), so Ami Pro shows the app's save times 4
to 8 hours off: to be fixed in the run.

The runtime's US rule: daylight time from the first Sunday in April (the
last Sunday in April before 1987) to the last Sunday in October.
VERIFIED (decompile) for the defaults; the dates of the rule are the
runtime's, INFERRED from its code.

### When they change

`D` and `b` are formatted by the line builder (`10d0:2d0e`) each time
their line is built: on opening, editing, reflowing, and printing. FD4's
dates followed a change of the guest's date at the next layout. `r`
changes when the document is saved (the writer sets value 6 to now first,
`11c8:342b`); `k` never. No command updates them: Update and Update All
are for power fields, and Show Power Fields does not change them (only
the power field object reads that flag in the line builder). VERIFIED
(decompile, fixture FD1).

## Doc Info

### `[desc]` and `[fldnames]`

Written by `11c8:342b` (documents and style sheets), read by `1058:2732`
and `1058:267c`, **only when a document is opened**: File New and Use
Another Style Sheet skip both sections of a style sheet. VERIFIED
(decompile). This corrects `SAM_FORMAT_NOTES.md`, which called values 2–5
keywords.

| # | Meaning | Limit | Ami Pro | New document |
|---|---|---|---|---|
| 1 | Description | 119 | as edited | empty |
| 2–5 | Doc Info Fields 1–4 | 31 each | as edited | empty |
| 6 | Last save time | seconds (the time base above) | set to now at every write | the creation time |
| 7 | Total revisions | | one more each time the document is opened | 1 |
| 8 | Creation time | seconds | | now |
| 9 | Total editing time | minutes | at each save, plus (now − session start + 30 s) / 60; the session restarts | 0 |
| 10–13 | Pages, words, characters, size (K: the saved file's bytes / 1000, truncated) | | as of the last Update (pages also whenever layout reaches the last page) | 0 |
| 14 | Size with the style sheet and linked files (K) | | as 13; shown nowhere | 0 |
| 15 | Keywords | 499 | as edited | empty |
| 16 | The user who locked it for annotations | | written only while locked | empty |
| 17–20 | Doc Info Fields 5–8 | 31 each | as edited | empty |
| 21 | Flags: bit 0 Run frame macros | | | 1 |

VERIFIED (decompile); values 1–15 and 17–21 VERIFIED (fixture) FD2,
FD2B. Shorter forms from older versions: the reader stops at a value that
starts with `[`; with 7 values the creation time is value 5; with 8 the
rest is zero or empty. `[fldnames]` holds the eight field names, one per
document (FD2: "Client" after Rename Fields); an empty name falls back to
`[AmiPro] FieldN=` in AMIPRO.INI, then to "FieldN". Lock revision marking
on is `[revisions]`, not `[desc]`.

Corpus: `[desc]` with 21 values in 372 user and 158 ISO SAMs, 8 values in
46 and 2; `[fldnames]` in 147 and 58, renamed only in 4 ISO SAMs; a
description in 137 and 85; fields 1–4 in about 40 user SAMs (41 longer
than 31 characters, from writers before 3.1, which 3.1 cuts to 31);
keywords in 9 and 1; 15 files hold editing minutes near 2³²/60 (a clock
set back). The statistics match the files only loosely: they are as of an
old Update. VERIFIED (corpus).

### The Doc Info dialog (1013)

- Read only: File name ("Untitled"), Directory, Style sheet ("None" when
  there is none), Import files (the files linked pictures and DDE links
  come from), Date and Time created, Date and Time last revised, Total
  revisions, Total editing time (stored minutes plus this session's).
- Editable: Description (119), Keywords (499), Lock for annotations
  (needs a User Setup name; only the locker can clear it), Lock revision
  marking on, Run frame macros.
- Statistics (No. of pages, words, chars, Size (K)) show the stored
  figures **dimmed** until Update recounts them (FD2: 1, 6, 23, 4).
- Other Fields… edits the eight values (stored at once, whatever Doc
  Info's own OK or Cancel does later); its Rename Fields… edits the eight
  names (stored at once).
- **Ami Pro's dates in this dialog show the year as years since 1900**:
  "10/20/126" for 2026 (FD2; `1280:0187`, a year-2000 fault). Its date has
  no padding, in the `iDate` order.

VERIFIED (decompile, fixture).

### Insert Doc Info Field (1123)

22 entries; each inserts `<:d` + a date-format letter + a 16-bit code
(low byte, high byte) + `>` (internal 3 `d` f code), escaped as above (FD2:
byte 9 is written as a literal tab):

| Entry | Code | Shows (FD2) |
|---|---|---|
| Filename | 1 | FD1.SAM ("(Untitled)" before a save) |
| Path | 2 | L:\OUT |
| Style Sheet | 3 | None, or the sheet's name |
| Document Description | — | inserts the **power field** `Description` |
| Date Created | 4 | in the letter's format |
| Time Created | 0x14 | 11:23 AM |
| Date Last Revised | 5 | in the letter's format |
| Time Last Revised | 0x15 | |
| Total Revisions | 6 | |
| Total Editing Time | 0x16 | minutes |
| Number of Pages | 0x17 | |
| Number of Words | 0x18 | as of the last Update |
| Number of Chars | 0x19 | as of the last Update |
| Size of Document | 0x1a | as of the last Update |
| The eight fields, by their names | 8–0xf | the field's value |

The letter is the last Insert Date/Time format; only codes 4 and 5 use it.
The shown text is computed every time the line is built, so these follow
renames, saves, and layout, except the three statistics (`1308:035d`,
`10d0:2d0e`). Edit › Power Fields ▸ Update All also refreshed them
(FD2B). Show Power Fields does not change them. VERIFIED (decompile,
fixture). In the corpora: 3 marks (Filename twice, Number of Pages).

### Counting (`12a0:0e4f`, `12a0:0f9a`)

Characters are every text byte, spaces and tabs included; not paragraph
ends, formatting marks, or objects (dates, fields' marks, anchors). Words
are runs ended by a space, a tab, a no-break space, or a paragraph's end;
hyphens, hard returns, and discretionary hyphens count inside words, and
an object does not split a word. Every text stream is counted but notes.
VERIFIED (decompile); FD2's 6 words and 23 characters VERIFIED (fixture).
A power field's result is ordinary text and is counted (FD3: NumWords
counted the results before it).

## Power fields

### Where they live

Three modules share them. VERIFIED (decompile):
- `AMIPRO.EXE` keeps one record per field (a chain of blocks: size, id,
  the value word, the kept result while codes are shown, the kind, the
  code text; `1630:0b19`, `1630:0ed1`), the marks in the text, the
  commands, Show Power Fields, and Seq (segments `1628`–`1650`).
- `AMIMACRO.DLL` evaluates a field's code: the code is a macro expression
  run in a field mode (`EXECUTEMACRO`, opcode 5 to classify, 4 to
  evaluate). Ami Pro reaches it through `15b0:011a` and `15b0:00e9`; the
  interpreter asks Ami Pro for document data through a callback
  (`15b8:0cb3`, request `0xd9`).
- `AMIPROUI.DLL` has the dialogs (`FIELDDLG` `1438:0000`, `FIELDSAVEDLG`
  `1438:0f45`). The field keywords, their numbers, and the dialog's
  descriptions come from the install's `MACTOKEN.SAM`.

### Stored form

```
<:XK,V;CODE>                        kinds without a result in the text
<:XK,V;CODE>RESULT<:X~K,V;CODE>     kinds with one: the result is ordinary text
<:X4,V;Seq Name…><:X~4;N>           Seq: the mark, then its number cached
```

K is the kind, V the record's value word (signed decimal), CODE the code
as typed, escaped as body text (`>` is `<;>`). The end mark repeats
`K,V;CODE` (FD3). The reader (`11b8:030a`, `1628:0000`, `1628:0115`)
pairs an end mark with the last start whose code is the same; a mark
without its partner becomes a dead control that draws nothing and is not
written again. Seq's `<:X~4;N>` is dropped on reading: it is a cached
result for other readers; the number is drawn by the mark. `<:Mname>`, an
older merge mark, is read as a kind 3 field `name` with the result
`<name>`. VERIFIED (decompile); the forms VERIFIED (fixture) FD3, FD6.

| K | Kind | Result between marks | Chosen when the code starts with |
|---|---|---|---|
| 2 | bookmark (written `<:Z…>`; the notes run) | the marked text | Edit › Bookmarks |
| 3 | general field | yes | anything else: an expression, a name, `IF`, a function, `NumPages`, a quoted string |
| 4 | Seq | no (drawn by the mark) | `Seq` and a name |
| 5 | Set | no | `Set` |
| 6 | Button | the display text | `Button` |
| 7 | PrintEscape | no | `PrintEscape` |
| 8 | Index | no | `Index` |
| 9 | User | yes (empty) | `User` |
| 11 | TOC | no | `TOC` |
| 12 | MergeField | the merge value, or `<NAME>` | `MergeField` |

Kind 10 is a document variable, stored only in `[docvars]` as
`name=value` lines for the macro functions GetDocVar and SetDocVar
(`1630:1dd7`, `1630:1e12`), **not** for Set: Set defines a macro global
variable that lives while Ami Pro runs (FD6: a variable Set in FD3 was
still Defined in a new document). Set fields are evaluated as the file
is read, so their variables exist before other fields update. VERIFIED
(decompile, fixture).

**The value word**: 0x2000 Lock (Update and Update All skip the field;
the dialog's OK is disabled); 0x1000 Auto run (updated after the document
is opened, locked or not); 0x4000 update again after pagination (set when
the field read NumPages); 0x8000 set when the field is made in the main
text (INFERRED from the corpus: header and frame fields lack it). For Seq
the low 14 bits are its number, and for Index the word is overwritten by
the page last found, so on those kinds the bits collide: locking a Seq of
5 made 8197, shown as MMMMMMMMCXCVII (FD3, an Ami Pro fault). VERIFIED
(decompile, fixture FD3: Auto run −28672, both −20480).

### The language

A field's code is an expression (VERIFIED (decompile), results VERIFIED
(fixture) FD3, FD6):
- numbers, quoted strings (`""` a quote inside), `{expression}` inside a
  quoted string (`"x{NumPages}y"` gives x1y), names, keywords, macro
  functions with parenthesised arguments, `IF … ELSEIF … ELSE … ENDIF`;
- operators `+ - * /` (`3 / 2` gives 1.50), comparisons `= != <> < > <=
  >=`, `AND OR NOT` (`10 > 2 AND 1 = 1` gives 1), `&` and `|` (`"a" &
  "b"` gives 0);
- a **bare name** is looked up as a bookmark's text, then a global
  variable, then a merge field; when none, warning 553 and the result
  `<NAME>` (FD3: `undefinedvar`; on opening a file whose field has Auto
  run, Ami Pro shows "undefinedvar is not a valid field name.");
- a format code last: `%Dx` (the 21 date formats above), `%Tn` (below),
  `%Nn` (1 decimal, 2 and 3 Roman, 4 and 5 letters: FD6 `Seq A %N4` 14 is
  N, `%N5` 15 is o, `%N3` 16 is xvi), `%P` and `%W` for Include.

**Times** (`%Tn`, AMIMACRO `10a0:0374`; FD6 at 11:52): T0 nothing; T1
`11:52` (24-hour, zero-padded); T2 `11:52 AM`, T3 the same with the hour
zero-padded; T4/T5 `11:52 A`; T6/T7 `11:52 am`; T8/T9 `11:52 a`. The
guide's T0 "9:11" is wrong. VERIFIED (decompile, fixture).

**Keywords** (MACTOKEN.SAM's field class; the token numbers 0–26):

| Keyword | Returns | Needs |
|---|---|---|
| Bookmark | (the dialog inserts a bookmark name, which resolves as a bare name) | bookmarks |
| Set name expr | nothing; defines a global variable | |
| Seq Name[=e][-e][+e] [%Nn] | the number (below) | |
| PageRef bookmark | the bookmark's page | bookmarks, pagination |
| NextRec, Skip | nothing; merge control | merge |
| Button "text" expr | the text; a double-click runs expr | macros for the action |
| CreateDate, EditDate [%D…/%T…] | the creation or last save time; without a format, seconds since 1970 (FD3: 1792521238) | Doc Info |
| NumEdits | revisions ([desc] value 7) | |
| TotalEditingTime | stored minutes plus this session's | |
| NumPages, NumWords, NumChars, FileSize | the statistics, counted when asked | pagination |
| MergeRec | the merge record; 0 outside a merge | merge |
| Include "file[!bookmark]" [%P] [%W] | another file's main text or a bookmark's text | other files |
| Defined name | 1 or 0 | |
| PrintEscape "…[27]…" | nothing on screen; bytes sent at printing | printer |
| Index, TOC | nothing (marks; the TOC run) | |
| Void expr | nothing; runs expr | |
| User anything | nothing (a place for macros) | |
| Description | the Doc Info description | |
| GetBookMark name | a bookmark's text | bookmarks |
| MergeField name | the merge value; `<name>` outside a merge | merge |

The dialog also lists `IF`, `Call file!macro(…)`, and 14 macro functions:
Query$, Decide, Message (dialogs when updated), Exec (runs a program),
Now, Len, Mid$, Left$, Right$, LCase$, UCase$, StrCat$, StrField$,
FormatNum$ (FD3: `FormatNum$("$","",2,235.8)` gives $235.80). Any other
macro function may be typed too.

**Seq** (`1640`): the base is the previous Seq of the same name in the same
text stream, else 0 (or a master document's carried value); no operator or
`+` adds 1, `+e` adds e, `-` takes 1 away, `-e` takes e, `=e` sets e, a bare
`=` repeats the base. Inserting or updating one renumbers the later ones
of its name in its stream; Update All renumbers every stream; frames are
ordered left to right, then top to bottom. VERIFIED (decompile); FD6:
`Seq A`, `Seq A`, `=10`, (none), `-`, `=`, `+3 %N2`, `%N4`, `%N5`, `%N3`
give 1, 2, 10, 11, 10, 10, XIII, N, o, xvi.

### Display

Normally a field with a result shows the text between its marks (ordinary,
editable text); Seq's mark draws its number; Set, Index, TOC, PrintEscape,
User show nothing. **View › Show Power Fields** sets every field's result
aside and puts its code in **red** between `<` and `>` in every stream;
switching it off reads back the code (edited or not) and restores the
results. It is switched off before every save and when a document is
opened. Printing with codes shown prints them. VERIFIED (decompile,
fixture FD3, manual).

### Commands and when fields update

| Command | Behaviour |
|---|---|
| Insert… (`0x238`) | the dialog: Insert (the code), Fields (alphabetical; then the saved ones), Options (date and time formats for CreateDate, EditDate, Now; number styles for Seq; bookmarks for Bookmark), Description (the syntax and examples), Save…, Previous Field, Next Field, Lock, Auto run (enabled only on a field in the text), OK, Cancel. OK on a new field inserts it, evaluates it at once, and closes (`0x23c`); on a loaded field it changes it and stays open (`0x23d`); a locked field disables OK |
| Save… | a name (39) and a description (125) kept in AMIPRO.INI `[Fields]` as `Name=Description~Code`, listed after the keywords |
| Update (`0x23b`) | the field at the caret (on a mark or in its result), unless locked; with codes shown, the edited code first |
| Update All (`0x10b`) | codes off; every Seq renumbered; every stream in order (main text, headers and footers, floating headers and footers, layout frames, page frames); locked fields and Index, TOC, Button, PrintEscape, User skipped; Set evaluated; fields that read NumPages updated again at the end. Also refreshes the date and Doc Info objects as their lines are rebuilt (FD2B) |
| Next Field, Prev Field (`0x239`, `0x23a`) | selects the whole next or previous field, into other streams |
| Show Power Fields (`0x10a`) | above |

Fields update on insertion, by Update and Update All, when opened (Auto
run), Set when read, per record while merging, when printing with Print
Options › Update fields, and when a TOC or index is generated. Nothing
updates while typing. VERIFIED (decompile, manual).

**Editing.** The result is ordinary text: FD3, typing X inside "October
20, 2026" gave "OctobeXr 20, 2026", and Update put it back; bold over the
whole field was written around its marks and survived Update (FD3B). An
update deletes the text between the marks and inserts the new result (a
result of several paragraphs, from Include, becomes new paragraphs,
`1578:04a0`). Deleting a selection that holds a field asks once: "There is
a field in the selection to be deleted. Would you like to continue?"
(message 3158). Pasted fields get new records. VERIFIED (decompile,
fixture).

### In the files

User's corpus 7 files with `<:X`, ISO 8 (VERIFIED (corpus)):

| Shape | User | ISO |
|---|---|---|
| Index (`Index "…" # "…"` and shorter) | 27 | 202 |
| TOC `n "…"` | 4 | 0 |
| MergeField pairs, result `<NAME>` | 15 | 0 |
| Kind 3 bare-name pairs (merge variables), mostly `<NAME>` | 10 | 10 |
| NumPages in a header or footer (V 16384) | 1 | 1 |
| Seq, Set, Button, PrintEscape, User | 0 | 0 |

Style sheets (merge letters) hold more: MergeField, NextRec, IF, bare
merge names, NumPages, NumWords, NumEdits, EditDate, Description.

`<:>` (359 marks in 61 user files, 79 in 27 ISO files) is not a field: it
is an empty unknown control, kept by the parser, skipped by the line
builder, and written back unchanged; the app already keeps it so.
VERIFIED (decompile).

## Other controls the parser groups with these

- `<:T>`: a TOC end mark: TOC generation takes a heading's text up to the
  first `<:T>` (`1588:0da8`); it shows nothing, and nothing in 3.1
  inserts it (TOC Entry… makes a power field). 165 marks in 48 user files.
  For the TOC run (queue 12); kept as it is.
- `<:e app,topic,item>` … `<:e>`: a DDE link (Edit › Paste Link) around its
  linked text; not a field. For the objects run (queue 22); kept.
- `<:VS±>`, `<:VE±>`: revision marks (queue 23); kept.

VERIFIED (decompile) for each reading; the meanings of DDE's three strings
and of `+`/`-` INFERRED.

## The twin (VM, 2026-10-20)

AFD1, written by the app (`field_twins.rs`, `write_field_twin`: every
date kind, three Doc Info entries, seven power fields, Doc Info with a
renamed field, created at 10:00 and saved at 10:30 on 20 October 2026),
opens in Ami Pro showing every field as the app shows it: the time created
10:00 AM, Alpha under Client, Seq 1 and II, 1.50, ABC, yes. Doc Info shows
the description, the keywords, the created and saved times, 2 revisions
(the file's 1 and the open), 5 minutes, and the app's statistics; Ami Pro's
Update counts the same 21 words, 70 characters, and 2 K. Its Update All
gives every field the app's result; the one change it writes is NumPages's
value word, −16384: Ami Pro sets 0x4000 when an update reads NumPages, and
the app now does too (FL.13). VERIFIED (fixture).
