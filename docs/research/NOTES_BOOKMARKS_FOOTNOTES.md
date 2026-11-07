# Notes, bookmarks, footnotes, and endnotes in Ami Pro 3.1

**Written 2026-10-26** for the Phase 5 queue's run 10 (`notes`,
`docs/playbooks/FIELDS_AND_REFERENCES.md`), from the decompiled
`AMIPRO.EXE` and `AMIPROUI.DLL` (`docs/research/DECOMPILATION.md`; new
`DecompAt.java` output for `1358:0000`, `14d8:0000`, `1628:03ac`,
`1580:0629`, `067f`, `06ca`, `0abe`, outside Git), the user's guide (ch.
20, 24, 25), both corpora (counted only; no file names), and Ami Pro 3.1
fixtures made in the XP VM on 2026-10-26 (below). Findings are in our own
words: routine addresses, rules, and record layouts, never decompiler
output. Evidence: **V** VM fixture, **D** decompile, **R** resources,
**M**/**G** the user's guide, **C** corpus, **?** a guess.

## The VM fixtures (2026-10-26; `../AmiPro-Reference/vm-lab/OUT`)

| Fixture | Made how | Found |
|---|---|---|
| RNN1 | Insert › Note at the end of a paragraph | `<:N1793037253,LS,65535,1,1`, a line end, the note's paragraph, a blank line, `>`; the note window is yellow, its text bold, titled "LS 10/26/126 9:54 AM" (the year as year − 1900) |
| RNN2 | Two notes, the first with two paragraphs (Ctrl+B in the note did nothing) | numbers 1 and 2 in text order; paragraphs separated by blank lines |
| RNN3 | User Setup: Display initials in text, red, initials QX; a third note | `<:N…,QX,255,3,1`: colour and initials belong to each note; with Display initials on the text shows `[LS1]` after each mark (taking width); the mark is a box in the note's colour with a black outline, centred on its position, taking no width |
| RNB1 | Edit › Bookmarks: "beta " selected, Add `bmone`; the caret, Add `MixTwo` | `Alpha <:Zbmone>beta <:Z~bmone>gamma`; `<:ZMixTwo><:Z~MixTwo>`; no `[book]` |
| RNB2 | A frame selected, Add `frbm` | the frame's `[frmname]` is `frbm` |
| RNB3 | A 2 × 2 cell selection, Add `cellbm`; the caret in a cell, Add `incell` | `<:Z2,2;cellbm>` at the start of the top-left cell's text; `[book]` `incell 3 2 0` (table id 3, row 2, column 0) |
| RNF1 | Tools › Footnotes › OK at the end of a paragraph, text typed | `<:F`, a line end, `@Footnote@The footnote text.`, a blank line, `>`; `[fopts]` 0, 1, 0, 0, empty, empty; a new Footnote `[tag]`; the area at the page's foot under a full-width separator, the number superscript at the left, the text at 0.5 in |
| RNF2 | RNF1 with Make endnotes | `[fopts]` flags 1; the footnote on a page of its own after the text, at its top, with no separator |
| RNF3 | Reset number on each page, Starting number 3 | flags 2, start 3; the number shows 3 |
| RNF4 | Custom 2.00 in, indent 0.50 in, Continued on "Continued" Left, Continued from "From before" Right, Repeat footnote number | flags 188 (`0x04` custom, `0x08` on, `0x10` from, `0x20` on-left, `0x80` repeat), length 2880, indent 720 (twips), the two texts |
| RNF5 (from RNF5IN) | 60 paragraphs; footnotes at 8 (short), 30 (25 paragraphs), 50; RNF4's options | page 1 ends with the referencing paragraph 30 and the long footnote's first 13 paragraphs, then "Continued" at the left foot; page 2's area starts with "From before" at the right (the number repeated at the left), the rest of the footnote, then footnote 3; `[pg]`'s second part gives the footnote stream's position at each page end (paragraph 13 of footnote 2) |
| RNF71S–RNF74S | 70 paragraphs; a three-paragraph footnote on paragraph 44, 45, 46, or 47 | ref on page 1's last line with room for the separator and one line: the footnote splits, its first line on page 1 (RNF71); ref where the separator and one line no longer fit: **the referencing line moves to page 2** with its whole footnote (RNF72–RNF74) |
| RNF6 | A 3 × 2 table, a footnote in B1 | `b1 cell<:x1,2>`; a third row added, one joined cell across the three columns (cell field 3 `384`, the covered cells `128`), `@Footnote@Cell footnote.`; the row's `[h]` entry `0 284 86 2 0 2 0` |


## Notes and bookmarks (map)
### 1. Notes: `<:N…>` grammar, storage, reader, writer

### In memory
- In paragraph text, a note is a 5-byte control: byte 0x03, letter `N`,
  then a handle to a **note record** (D: `10d0:0bfc` skips 5 bytes for
  controls; `1158:09a8` and `1158:0c6e` scan for 0x03 `N`).
- The note record (allocated by `1158:1293`, 11 words or more) holds (D,
  reader `11b8:030a` case `N`, writer `11b0:0660` case `N`):

  | Offset | Field | File field |
  |---|---|---|
  | +0 | handle of the note's first paragraph (a chain of ordinary paragraphs) | the lines after the control |
  | +2 (long) | time, Ami Pro's time base (seconds; `FIELDS_NOTES.md`) | 1 |
  | +6 (word) | note number | 4 |
  | +0x0a (long) | colour, a COLORREF | 3 |
  | +0x0e (string) | author's initials | 2 |
  | +0x16 (byte) | 1 = the note is in the main text, 0 = elsewhere | 5 |
  | +0x17 (word) | next note in the document's note chain (`DAT_1680_2221`) | — |

### Grammar (D for the order; C and V for the values)
`<:N` TIME `,` INITIALS `,` COLOUR `,` NUMBER `,` MAIN, then a line end,
then the note's paragraphs, then a line that is only `>`.

- TIME: decimal (atol). If there is no field at all (the control is just
  `<:N`), the reader uses the current time.
- **The short form `<:N`TIME** (no commas: 10 corpus notes, all in older
  documents) is legal. Then the reader fills in the other fields: initials
  from `AMIPRO.INI` `UserInitials` (string 1209; `1158:1310`), colour from
  `NoteColor` (string 1210; `1158:12ba`; if absent, 0xFFFF → 65535, yellow),
  the number as the note's position in the note chain, and MAIN as "not in
  a frame" (D). Fields are split on commas only, with no escape, so
  initials cannot hold a comma (D, ?: User Setup probably accepts one, which
  would break the file: a VM fixture could test this).
- INITIALS: text up to the next comma. User Setup limits it to 6 characters
  (G, AP3CH03); the edit box is 30 dialog units wide. It is empty when no
  initials are set (?).
- COLOUR: a decimal COLORREF: 65535 yellow, 32768 green (MERCURY), 255 red
  (V).
- NUMBER: the note's number. In the main text notes count 1, 2, 3 in
  document order. Notes in tables and text frames are numbered **separately**
  from the main text. **Notes in headers, footers, and footnotes are
  numbered 0** (G, AP3CH24; C: frame and table notes carry 0–3). Ami Pro
  **renumbers** later notes whenever one is inserted or removed (G), so the
  stored value is a cache of that numbering.
- MAIN: 1 for a note in the main text, 0 for one in a frame, table, header,
  or footer. C: 33 of 33 body notes have 1, and 57 of 57 notes in frame or
  table text have 0. D: the chain keeps main-text notes (bit 1 set) first,
  which is how numbering separates them. (This corrects the guess that the
  last field is "Display initials in text"; V agrees.)

### The note's text (D, C)
- The text after the control's line is read by the ordinary paragraph
  reader (`1118:0e55`, called in a loop from `11b8:030a`) until the lone `>`
  line. Each paragraph is a full SAM paragraph, linked through the
  paragraph's +6 "next" word, so **styles and character marks are legal**:
  older files tag every note paragraph `@Body Text@` (C), and the reader
  would take `<+!>` and the rest like body text.
- Paragraphs are separated by blank lines, as in the body. An empty
  paragraph shows as an extra blank line (MERCURY: two blank lines between
  two text paragraphs).
- **Escaping** is the body's: the writer sends each note paragraph through
  the body writer `11b0:1405` (D), so `<` is `<<`, `>` is `<;>`, `[` is
  `<[>`, `@@` at a paragraph's start, CP-1252 escapes `</X>` and `<\X>`, and
  the same 65/256-byte line wrapping as body paragraphs.
- When the reader is called with flag 8 (its fourth argument), it parses and
  discards the note (?: probably Insert › File or a paste without notes).

### The writer (`11b0:0660`, case `N`; D)
1. `<:N`, then the time (ltoa), `,`, the initials (written through the
   escaping character writer `11b0:05cf`), `,`, the colour (ltoa), `,`, the
   number, `,`, the MAIN byte.
2. The line end.
3. Each note paragraph through `11b0:1405`.
4. `>` on its own line, then the line end (already in `SAM_FORMAT_NOTES.md`:
   the paragraph's text goes on on the next line). If the control is not
   the last thing in its paragraph, one more line end is written.

### Editing in the note window (D, G, V)
- **The note window is a plain multi-line edit control** (class "edit",
  `ES_MULTILINE` with a vertical scroll bar) inside a captioned pop-up
  window of class `AmiProNoteA` (string 1279). `1158:05b4` creates it with
  the note's colour as its background brush, a 0.1 in margin, a client area
  of about **3.75 × 1.5 in**, centred on the screen. A second note opened
  in the same place is offset by 0.1 in per step, cascading. The edit limit
  is **10,240 characters** (`EM_LIMITTEXT`).
- Text goes in as the paragraphs joined with CR LF (`1158:0aff`) and comes
  back split at line ends into Body Text paragraphs (`1158:0e45`; ?: the
  style). So **a note's formatting is lost once the note is edited in Ami
  Pro**: plain text only (V: Ctrl+B does nothing).
- **The title** (`1158:08e5`) is `INITIALS+NUMBER DATE TIME`. A new note
  has no number yet (V: "LS 10/26/126 9:54 AM"). A reopened note shows the
  number appended to the initials ("LS1 …", ?: V should confirm). The date
  prints the year as year − 1900 (V: 126), Ami Pro's year-2000 defect. Do
  not copy it: show four-digit years.
- The window's Control menu loses Restore, Minimize, Maximize, and so on
  (`REMOVEMENU` by position), keeping Move, Size, and Close, and gains
  **"&Remove This Note"** (string 1339, id 2) and **"Remove &All Notes"**
  (string 1340, id 3). Remove All warns "Deleting all notes can not be
  undone. Would you like to continue?" (AMIPROUI string 3180) and cannot be
  undone (G).
- Clicking outside the window closes it and saves the text (`1158:0db4`).
  The time is set to now when the text changed (`1158:0e45` takes the
  current time; G: "created or last edited"). An unchanged note keeps its
  time (?).
- **Several notes at one position** are consecutive 0x03 `N` controls.
  `1158:09a8` counts the run that ends at the caret, and `1158:030c` (mode
  2) closes the shown note and opens the next one in the run (D).
  Double-clicking the mark opens the note (G).

### Insert › Note (command 0x11a → `1158:001e` → `1158:01af`; D)
- In the main window, a table selection is collapsed first (`1068:0e52`).
  In a frame, the caret goes into the frame's text. The note's position is
  the caret (`DAT_1680_231f`). The note record is created only when the
  window is closed with text (?: an empty note is discarded; fixture NNEMP).
- Initials and colour come from User Setup (`UserInitials`, `NoteColor` in
  `AMIPRO.INI`) at the time the note is created. **Colour and initials
  belong to each note**, not to the reader (V: red after changing the
  setup). Notes can be inserted in the main text, tables, text frames,
  headers, footers, and footnotes (G).
- Commands 0xb2 and 0xb5 share these routines (a toolbar icon and a macro
  form, ?).

### 2. Notes on screen and on paper

### The note mark (`1108:35a6`, case `N`; D, V)
- It is drawn only when **View Preferences › Notes** is on: bit 0x0800 of
  the view flags word at `DAT_1680_28c6` (tested as `DAT_1680_28c7 & 8`).
  The other marks use bit 0x40 (Marks).
- It is a rectangle filled with **the note's own colour**, outlined with the
  black stock pen. It is centred horizontally on the mark's x position. Its
  width is 5h/9 and its height 5h/6, where h is the line metric passed in
  (?: the font height). It stands from h/6 above the reference line upward.
  The mark takes **no width**: the line builder skips the 5-byte control
  when Notes is off (`10d0:0bfc`), and the box overhangs its neighbours
  (the routine widens the line's repaint extents).
- With User Setup › **Display initials in text** on, `[INITIALS+NUMBER]`
  appears after the mark and takes width (V: "[LS1]"). Without initials but
  with the option on, only the number shows (G).

### Printing (G, AP3CH24; Print Options dialog 1083, control 25556 "With
&notes")
- With notes prints `[number]` (initials before the number when set) in the
  text where each note is.
- **On separate pages after the last page**, it prints each note's number,
  initials, and contents, grouped by "Page N" headings for the pages the
  notes are on, in the font of the current Body Text style.
- Notes are not printed when Reverse order is chosen. Without With notes,
  marks and notes do not print.

### Other settings
- **User Setup** (dialog 1019): Name (25301; G: "Type your name to allow
  other people to add notes but prevent them from editing", tied to the
  annotation lock, value 16 of `[desc]`, `FIELDS_NOTES.md`), Initials
  (25302, up to 6 characters), **Display initials in text** (25558), **Note
  color** (25590, a colour bar). G: the Options… button sets the rest.
- **View Preferences** (dialog 1018): **Notes** (25561), "Displays notes as
  colored numbered rectangles" (G).
- Deleting a selection that holds a note asks "There is a note in the
  selection to be deleted. Would you like to continue?" (string 3075)
  unless warnings are disabled (G, AP3CH03).
- **Find & Replace does not search notes** (G, AP3CH20; already in
  `SAM_FORMAT_NOTES.md`). Word count skips notes (`FIELDS_NOTES.md`).
- **Go To** (dialog 1015, Next item list; strings 2100–2112 include
  "Note" (2109) and "Bookmark" (2111)): Next item › Note **opens the next
  note's window** (G; D: `12d8:0aba` scans for 0x03 `N`).

### 3. Bookmarks

### Four kinds, one list (D: segment `1580`, the list at `DAT_1680_21aa`)
The document keeps **one list of bookmark records**, created by
`1580:04ba` and changed by `1580:08d0` (add, find, remove by name) and
`1580:0b37` (get, set, remove by index). The record:

| Offset | Field |
|---|---|
| +0 | record size |
| +2 | index (the id that marks refer to; never reused in a session) |
| +4 | flags: 0 text, 2 cell range, 4 frame, 8 one cell; 1 = a DDE link item (hidden), 0x10 = a temporary copy (hidden) |
| +6 | table id (cell kinds), frame handle (frame), or a DDE link |
| +8 | row (one cell) or row count (range) |
| +10 | column (one cell) or column count (range) |
| +12 | name; a first byte of 1 marks a removed record |

How each kind is stored in the file (D, C, V):

| Kind | Made when Add is chosen with… | Stored as |
|---|---|---|
| **Text** (flag 0) | a selection or the caret in text (main, header, footer, frame, cell text) | two kind-2 power-field marks: `<:Zname>` at the start and `<:Z~name>` at the end (V RNB1; at the caret both are adjacent) |
| **Cell range** (flag 2) | a selection of several table cells | one mark `<:ZROWS,COLS;name>` at the start of the top-left cell's text, **with no end mark** |
| **One cell** (flag 8) | the caret in a table cell while the table is in cell-selection mode (`DAT_1680_2a39 & 0x80`) with both selection ends in one cell | a `[book]` line, **with no mark** |
| **Frame** (flag 4) | one selected frame | the frame's `[frmname]` (V RNB2); the dialog opens with the frame's name |

### `<:ZROWS,COLS;name>` (D: `1580:11ed`, `1580:0629`, `1580:06ca`,
`1580:0d1a`, `1580:16de`; C, V)
- The reader of a kind-2 start mark (`1628:03ac` op 0 → `1580:0629`) looks
  for `N,M;` before the name (`1580:11ed`: a comma before a semicolon). If
  it is there, the record gets flag 2, **+8 = the first number** (stored as
  a byte) and **+10 = the second**.
- Go To and the cell lookups (`1580:0d1a`, `1580:16de`) treat +8 as a
  **row count** and +10 as a **column count** from the mark's cell, and +8
  and +10 of a one-cell record as its row and column. So the mark is
  **`<:Zrows,cols;name>`**. C confirms the order: a corpus file with a
  72 × 3 table has a whole-row bookmark `<:Z1,3;…>` in column 0 of each row
  (1 row by 3 columns). V RNB3's `2,2` is square and cannot tell.
- The writer (`1580:06ca`, op 2) writes `ROWS,COLS;` before the name for
  flag 2 (wsprintf, then the name).
- A start mark's name cannot be bare digits followed by a comma and
  semicolon. A text bookmark named like `1,3;x` would read back as a range
  (?, but the dialog has no rule against it).

### `[book]` (D: writer `1580:1368`, reader `1580:145c`; C, V)
- One line per **one-cell** bookmark: `NAME TABLE ROW COLUMN`, separated by
  spaces (format `%s %d %d %d`), with the usual tab indent:
  - TABLE: the table's frame id, which is **field 13 of the table's `[frm]`**
    (the frame's +0x55 word, the same number `[frmname]` defaults to,
    "Frame93"). If the frame has none, Add gives it one first (`10b8:071f`).
  - ROW and COLUMN: 0-based, in `[data]` cell order (first number the row).
    C: a corpus template's five one-cell bookmarks each match their
    cells. V RNB3: `incell 3 2 0`, table 3, row 2, column 0.
- The reader splits a line at its first three spaces (so **names hold no
  spaces**), reads three integers, and adds the record with flag 8. It
  stops at the next line that starts with `[`.
- The writer writes only flag-8 records that have not been removed. It
  writes `[book]` once, before the first line, and nothing when there are
  none. It is called from both the document writer `11c8:1c7b` and the
  style-sheet writer `11c8:14e0` (so a sheet with contents keeps them).
- **No offset inside the cell is stored**: Go To on a one-cell bookmark
  puts the caret in that cell (`1590:0022` with offset 0: probably the
  cell's start, ?; fixture NBGOC).
- The table is found by id across the page's frame lists (`1580:12ac`). **If
  the table no longer exists, Go To removes the bookmark** (`1580:0d1a`,
  D).

### Text bookmark marks (D, V)
- A kind-2 field mark (FIELDS_NOTES kind 2). Its letter comes from the table
  at DGROUP 0x11c2 (`1628:0040`: kinds 0 and 1 none, 2 `Z`, others `X`).
  Its handler is `1628:03ac` (from DGROUP 0x1196): op 0 reads a start
  (`1580:0629`), op 1 reads an end (`1580:067f`, the name after `~`, matched
  case-insensitively), op 2 gives the text to write (`1580:06ca`), op 3
  removes (`1580:0abe`), and op 9 makes a temporary copy (below).
- The start mark carries +id and the end mark −id of the same record
  (`1580:0000`, `1580:0d1a`: the end is found by `1638:0438(−id, kind 2)`).
  The bookmark's text is what lies **between the marks**.
- Add inserts the end mark at the selection's other end, then the start
  (`1070:350d`), and redraws. Both marks are 5-byte controls with no width
  (?) and are not shown (G: "Bookmarks do not appear on the screen and do
  not print").
- **No nesting**: before Add, `1580:1555` checks whether the caret, or
  either end of the selection, lies inside an existing text bookmark. If
  so, Add is refused with "Bookmarks cannot be placed inside one another."
  (AMIPROUI string 3161; D).
- Remove (`1580:0d1a`, op 0xfa4) deletes both marks and marks the record
  removed. The text is unchanged (G).
- Go To (op 0xfa2) **selects the text between the marks**, or moves the
  caret there when they are adjacent. For a range it selects the
  rows × cols block from the mark's cell (`14a0:0000`). A frame bookmark
  selects the frame (`1248:0f9a`).

### Names (D: `BOOKDLGPROC` `13d8:0000`, `1128:0000` code 0x24,
`1580:08d0`, `1580:082c`, `1580:1079`)
- **Compared case-insensitively** (`LSTRCMPI`). The dialog's list is a
  **sorted** list box (LBS_SORT) of every record that is not removed and
  not hidden (flags 1 and 0x10 excluded, `1580:0c90`).
- **No spaces.** The name box checks on every change: a space beeps and
  cuts the name at that space (`strchr(name, ' ')`, read from the
  disassembly: `push 0x20`). Leading and trailing spaces are trimmed before
  Add (`1580:1079`); an empty name is refused.
- **Length**: the dialog reads at most 19 characters (`GetDlgItemText`
  with 20). The guide says "up to 17 characters" and that names "can
  neither start with numbers nor consist only of numbers". The decompile
  shows no such check in the dialog (?: perhaps a power-field rule, since
  a numeric name is read as a number in a power field; fixtures NBLEN,
  NBDIG).
- Other characters are allowed: the corpus has names with `:`, `.`, `_`, and
  digits (a colon, a full stop, an underscore).
- **Add with an existing name** (callback 0x24 finds it) does nothing, and
  the dialog stays open (D, ?: no message seen in the code; fixture NBDUP).
- **Duplicate names on reading**: the reader adds a start mark's record
  through `1580:082c`, which, when the name exists, **appends 1, 2, … up
  to 10000** until it is unique (D). C shows one case,
  a name ending in an added 1 (?: from a copy). A copied bookmark goes through op 9: it
  becomes a temporary record (flag 0x10) named with `_` appended, and on
  paste `1580:17ac` strips the `_` and re-adds it (so the copy gets the
  next free numbered name; ?, fixture NBCOPY).

### The Bookmarks dialog (AMIPROUI dialog 1114, `BOOKDLGPROC`; D)
- Controls: Bookmark (edit 25300), Current bookmark (list 25350, sorted),
  **Go To** (25586, the default button), Cancel, **Add** (25585), and
  **Remove** (25587).
- On opening, the list is filled through callback 0x25 (one name per
  index). The edit box gets callback 0xf2: **the selected frame's name**
  when exactly one frame is selected, else empty. With an empty box, Add,
  Go To, and Remove are disabled.
- When typing, the default button follows the text: **Go To** if the text
  matches a listed name exactly (prefix search, then a case-insensitive
  compare: `13d8:0373`), **Add** otherwise.
- A click in the list copies the name to the box. A double-click is Go To.
- Add checks the name (callback 0x24) and, when that passes, closes the
  dialog with 0xfa3. Go To closes with 0xfa2 and Remove with 0xfa4. The
  command `1580:0000` (0x1d2) then acts. There are no Next or Previous
  buttons in this dialog: Next is in Go To.

### Behaviour when text is edited (D for the mechanism; the rest ?)
- The marks are characters in the paragraph, so editing between them grows
  or shrinks the bookmark. Typing exactly at the start or end edge goes
  outside or inside depending on how the caret sits relative to the mark
  (?, fixture NBEDGE).
- Deleting a selection that holds a mark warns "There is a field in the
  selection to be deleted" (AMIPROUI string 3158; ?, D for the string). A
  deleted start or end mark leaves an unpaired mark: FIELDS_NOTES says an
  unpaired field mark is dead and is not written again. The record stays in
  the list until the session ends, and Go To on it finds no mark and does
  nothing (?, fixture NBDEL).
- The file always holds what is in the text: marks as written, `[book]`
  from the list (flag 8 only), and frame names in `[frmname]`. **Nothing is
  "recomputed" on save** except that removed records are skipped.

### Other uses of bookmarks (D, G)
- Power fields (`FIELDS_NOTES.md`): a bare name and `Bookmark name` give
  the text (`1580:1051` checks the name; `1580:0594` → `1580:0d1a` op 4000
  copies the text). `GetBookMark`, `Include "file!bookmark"`, and
  `PageRef name` give the page: `1580:10c8` works for all kinds (frame →
  the frame's page, text → the start mark's page, cell → the cell's page).
  `Defined name` is true for a bookmark.
- **Go To › Next item › Bookmark** moves to the next **text** bookmark in
  the main text or a header or footer (G, AP3CH20).
- **DDE**: Edit › Link copy makes hidden records (flag 1) named from string
  3137 `DDE_BOOKMARK` plus a number (`1240:2636`). Their DDE item names look
  like `Table<id>,R<r>C<c>:R<r>C<c>` for table ranges (`1570:02d6` parses
  them). Hidden from the dialog. Not needed for the first implementation.
- **Find & Replace**: no bookmark-specific behaviour was found.

### 4. Summary of what to implement

1. Notes are an inline object holding: the time, initials, a colour, the
   number (computed: 1… per main text, separately per frame or table, 0 in
   headers, footers, and footnotes), the main flag, and paragraphs. Read
   both `<:N` forms. Write the five-field form, paragraphs through the body
   writer, `>` on its own line.
2. Bookmarks are four kinds over one name space (case-insensitive, no
   spaces, at most 19 characters): text (`<:Z`/`<:Z~` pairs), cell range
   (`<:Zr,c;`), one cell (`[book]` `name table row col`), frame
   (`[frmname]`).

---

### VM fixtures to settle the open points (one property each; 8.3 names)

| Fixture | Property |
|---|---|
| NBRECT.SAM | Bookmark a **2-row × 3-column** cell selection: confirms `<:Z2,3;…>` is rows,cols |
| NBGOC.SAM | One-cell bookmark made with the caret mid-text; after reopen, Go To: is the caret at the cell's start, its end, or the old offset? |
| NBTDEL.SAM | Delete the table holding a one-cell bookmark, save: is the `[book]` line gone? |
| NBLEN.SAM | Type a 20-character name: is it kept at 19, or cut to 17? |
| NBDIG.SAM | Add a name starting with a digit (`1abc`): accepted? |
| NBNUM.SAM | Add the name `27`: accepted? |
| NBDUP.SAM | Add `alpha` when `Alpha` exists: message, or nothing? Does the old one move? |
| NBNEST.SAM | Select text overlapping a bookmark and Add: confirms message 3161 |
| NBEDGE1.SAM | Type at the caret just after `<:Zname>`: is the text inside the bookmark? |
| NBEDGE2.SAM | Type just before `<:Z~name>`: inside? |
| NBDEL.SAM | Delete text holding only the start mark, save: what is written (a lone `<:Z~name>`?) |
| NBCOPY.SAM | Copy bookmarked text and paste it elsewhere: the second bookmark's name (`name1`?) |
| NBHDR.SAM | Bookmark in header text: marks inside the header frame's text |
| NBRDUP.SAM | (hand-made input) two `<:Zx>` pairs with the same name: after opening, Bookmarks lists `x` and `x1`? |
| NBCOMMA.SAM | (hand-made input) a text bookmark `<:Z1,2;n>` in body text: what does Go To do? |
| NNSTK.SAM | Two notes at one position: file order, and does the window cycle? |
| NNFRM.SAM | Notes in a frame, a table, and a header: numbers and the last field (expected x,0; header 0) |
| NNPARA.SAM | A note with three lines including an empty one: confirms the empty paragraph as an extra blank line |
| NNWRAP.SAM | A note line over 65 characters: wrapping in the file, as in the body |
| NNESC.SAM | Note text with `< > [ @`: escapes as in the body |
| NNSTY.SAM | (hand-made input) a note with `@Title@` and `<+!>`: does Ami Pro keep them when the note is only viewed? When it is edited? |
| NNOLD.SAM | (hand-made input) `<:N682726199` alone: the initials and colour shown, and the five fields written on save |
| NNEMP.SAM | Insert › Note, type nothing, click out: is a note saved? |
| NNSEL.SAM | Insert › Note with text selected: is the note at the selection's start or end, and is the text kept? |
| NNTIME.SAM | Open a note without changing it, close it, save: the time unchanged? |
| NNTTL.SAM | Reopen a saved note: the title shows `LS1` (initials + number)? |
| NNCOMMA.SAM | Initials containing a comma (if User Setup allows it): what is written, and does it read back? |
| NNPRT.PS | Print With notes to a PostScript file: the in-text `[LS1]` and the end pages (the Page headings, the font) |
| NNRMALL.SAM | Remove All Notes: confirms message 3180 and Undo disabled |

## Footnotes and endnotes (map)
### 1. Storage

#### 1.1 The body control `<:F` (D, C)

- **Grammar.** `<:F` followed at once by a line end; then the footnote's
  own paragraphs, written exactly as body paragraphs (each `@Style@`
  tagged unless Body Text, paragraphs separated by a blank line, their
  own controls and attribute marks); then a line holding only `>`. The
  host paragraph's text goes on at the start of the next line (C: `>`
  then `<:f>` on the next line). It is the same shape as notes and
  floating headers: writer `11b0:0660` cases `F`/`H`/`N`/`h` (shared
  tail: write `<:F`, end the line, write each paragraph through the body
  writer `11b0:1405` in its sub-stream mode, then `>` and a line end;
  one more line end when the control's record says so). `<:F` has **no
  fields** (unlike `<:N`): no number, no flag byte.
- **Reader** `11b8:030a` case `F`: allocates a 10-byte footnote record
  (`1350:04e5`), appends it to the document's footnote list and gives it
  the next ordinal (`1350:0000`; list head `DAT_1680_2225`), stores a
  5-byte control in the host paragraph (code 3, `F`, 0, the record's
  handle), sets the host paragraph's "has a footnote" flag (paragraph
  byte +4, bit 4; the counting and search routines skip paragraphs
  without it), then reads paragraphs with the paragraph reader
  (`1118:0e55`) until the lone `>` and chains them as the footnote's text
  (record +0 = first paragraph; paragraphs linked by their next field).
  Afterwards `11b8:156e` is run on the chain (G: settles the chain's
  owner/style; not traced).
- **Footnote record (10 bytes, D):** +0 first paragraph, +2 next
  footnote, +4 its ordinal in the document (1-based). Insertion at
  position n relinks and renumbers the rest (`1350:0098`, `1350:0284`);
  deletion frees the record and its paragraphs and renumbers
  (`1350:01b0`). Counting the marks before a position: `1350:03e0`.
- **Number not stored.** The text never contains the number; it is
  computed (section 2). The footnote's first paragraph shows it as the
  Footnote style's bullet/number.
- **The footnote's style (D, C, M).** Its paragraphs are normally in the
  **Footnote** style (string 1169 = "Footnote", `AMIPRO.EXE` string
  table; resource `0x491`). `1098:0643` finds it, or creates it the first
  time a footnote is inserted: a copy of Body Text renamed Footnote,
  with First indent (`+0x30`) 720 twips, the bullet text set to the
  arabic number code alone (0x1A, i.e. `[spec]` text `<*:>`), `[spec]`
  value 6 = 1, and the number's attributes set to the internal bit that
  writes as superscript (G: file value 256; both corpus documents have
  `[spec]` value 7 = 256 or 257 = superscript (+bold)). Corpus Footnote
  styles: `[spec]` = `0, 0, <*:>, 0, 1, 1, 256|257, 0, 0, 0`.
  The user changes the number's kind (1, I, i, A, a, *, †), Text (e.g. a
  full stop), attributes, Right align, and Space for through Modify
  Style on the Footnote style (M).
- **Endnotes are not a different control.** Endnotes are only the
  document option Make endnotes (`[fopts]` flag 1); the same `<:F`
  controls are laid out at the end instead (D, M).

#### 1.2 Table footnotes `<:xN,ROW>` (D, C)

- In a table cell the reference is a different control, `<:xN,ROW>`
  (in memory code 4, `x`, length 4, then two words): N is the footnote's
  number **within the table**, ROW the table row that holds its text
  (C: `<:x1,6>` with row 6 the table's last row). The line builder
  handles it beside `F` (`10d0:3b66`), its mark is measured by
  `1308:1040` with the number from the table (`14d8:2114`).
- **The text lives in the table**: a row added at the table's end
  (M: "a single-column row for the footnote text at the end of the
  table"), stored as an ordinary joined cell spanning every column
  (cell field 3 `0x180` with the column count, the covered cells `0x80`;
  C) whose paragraph is `@Footnote@`. Several table footnotes: G, one
  row each (VM).
- Insertion `14d8:0000`. Make endnotes and Reset number on each page
  do not apply to table footnotes (M). Strings 3193, 3216–3220: tables
  with footnotes cannot be sorted; columns cannot be dropped before
  column A, column A cannot be dragged, references cannot be dropped
  below the first footnote row, rows holding footnotes cannot be dragged,
  nothing can be dropped into a footnote (R).

#### 1.3 The options: `[fopts]` (D, C)

A document section (reader `1058:228d`, called from the section reader
for `DAT_1680_0bfa` = `[fopts]`; writer in the document writer
`11c8:1c7b` after `[lang]`, before `[newmac]`), Ami Pro 3.1's six values:

| # | Global | Meaning | Default |
|---|---|---|---|
| 1 | `236c` byte | Flags (below) | 0 |
| 2 | `2362` | Starting number (1–9999 in the dialog) | 1 |
| 3 | `2364` | Separator length when Custom (twips; written 0 when Margins) | 0 |
| 4 | `2366` | Separator indent from left (twips) | 0 |
| 5 | `2368` | Continued on message text (empty = none; ≤ 64 characters) | empty |
| 6 | `236a` | Continued from message text | empty |

Flags (from the options dialog's OK, `AMIPROUI` `1128:06b6`, D):
`0x01` Make endnotes, `0x02` Reset number on each page, `0x04` Custom
separator length (else Margins), `0x08` Continued on message on,
`0x10` Continued from message on, `0x20` Continued on aligned **Left**
(clear = Right), `0x40` Continued from aligned **Left** (clear = Right),
`0x80` Repeat footnote number (in the Continued from message).

- Values are written tab-indented, one per line, the two strings as
  empty indented lines when absent (`T%u` ×4, `T%s` ×2). Older files have
  only the four numbers (431 corpus files); a string missing (the next
  line is a section) reads as no message.
- Missing `[fopts]`: flags 0, start 1, length 0, indent 0, no messages
  (`1058` loader tail; also the new-document reset in `11a8`).
- Corpus values: flags 0 (almost all), 2 (2 files), 1 (1 file), 96
  (`0x60`, both messages left, messages off: 9 files with
  "Continued on next page..." / "Continued from previous page..." as
  texts). Starting number always 1; length and indent always 0 in the
  corpus.
- Changing the options (`1358:0351`) stores them and marks the document
  changed; if anything differs it reformats (`1358:0678`). G: not an
  undo step (globals set directly; VM to confirm).
- Not in `[desc]`, `[lay]`, or the style sheet's `[lay]`. A style
  sheet's own `[fopts]` (60 STY files have one) is read by the same
  reader (G: Use Another Style Sheet takes it? VM).

---

### 2. Numbering (D, M)

- **Ordinal:** a footnote's place in document order (the list is kept in
  text order; insertion uses the count of `F` marks before the caret +1).
- **Shown number** (reference mark `1308:0ebd`; footnote paragraph's
  number `1108:4107`, `10d0:1b5f`): ordinal + Starting number − 1. With
  Reset number on each page, in Layout mode (and when printing, G for
  `2a49`), the count of footnotes before this page is subtracted first
  (`DAT_1680_2a9d`), so each page starts at the starting number. In
  Draft mode no page base is subtracted.
- **Asterisks and daggers** (bullet code 0x1F `*`, 0x07 `†`): the count
  is always per page (ordinal − footnotes before the page) and ignores
  the starting number; in Draft mode always one. The count is drawn as
  that many characters (`*`, `**`, `***`; M).
- **Format** comes from the footnote's first paragraph's style (normally
  Footnote) and its `[spec]`: the number kind, the Text before/after it,
  and the attributes (superscript by default). The **reference in the
  text** is drawn with that same bullet/number formatting (code
  `1308:0ebd` takes the style of the footnote's first paragraph and its
  bullet record), so changing the Footnote style's number changes both
  marks (M: "Ami Pro automatically places periods after all the
  footnote reference numbers"). G: the reference uses the host text's
  font with the bullet's attributes; the footnote's own number uses the
  Footnote paragraph's font (VM).
- Only the **first paragraph** of a footnote shows the number
  (`1108:4107` draws it only when the line's paragraph is the
  footnote's first); later paragraphs in the Footnote style show none.
  The number is placed as any bullet: at All, text at All + First
  ("Space for"), Right align available (M; `BULLETS_NOTES.md`).
- `11f8:0048` gives a footnote mark's plain text as ordinal + start − 1
  in arabic digits (G: used for text conversion, copy, or export).
- **Endnotes** number continuously through the document (the page base
  makes no sense for them; M advises turning Reset off with asterisks
  and endnotes).
- **Table footnotes** number within their table (G: from 1, ignoring
  the starting number; VM).

---

### 3. Layout

#### 3.1 The footnote area (D, M)

- Each page (more exactly each page flow: the main text's set of
  columns) has **one** footnote area record, reached from the flow
  context (+2; `1090:0518`). It is **full width**: from the text area's
  left to its right, below all columns. When it grows, every column's
  bottom is pulled up to the area's top (`12f8:029e`); when it shrinks,
  columns whose bottom was at the area's top are extended
  (`12f8:033f`). D for the mechanism; G that left/right are the
  page's margins rather than the reference's column (M: "Ami Pro uses
  the margins set in the current page layout to calculate the Separator
  Line"; VM with two columns).
- The area **grows up from the bottom margin** (M), and is a list of
  line records (first +0x16, last +0x18; each line: height +0x1c, y
  +0x1e, next +0x22, previous +0x24, its footnote's ordinal +0x26). Area
  record: +0 left, +2 its top (the remaining room's bottom), +4 right,
  +6 bottom, +0x1a footnote count before the page (the page base).
- **Separator:** the area's first entry is a separator record of
  **100 twips** height (`12f8:06d0`: created when the first footnote
  line comes to a page, only if the area's top − 100 is still above the
  main text's lowest line, otherwise the page is marked full for
  footnotes). Drawn (`1178:09f8`, from the area painter `1178:0ae3`) as
  one line with the current pen at the separator's y, from the area's
  left + Indent from left to either the area's right (Margins) or left +
  indent + Custom length, clipped to the page's right limit. **No
  separator for endnotes.** Thickness: the default pen (G: 1 device
  pixel / hairline; VM). Exact y within the 100 twips: G (top), VM.
- Footnote lines are built by the ordinary line builder for the
  footnote stream (`14f0:087a` → `10d0:0d82`) at the area's width, with
  the paragraph's space above added to its first line and space below to
  its last (`12f8:1321`). Paragraph spacing, indents, and alignment of
  the Footnote style therefore apply.

#### 3.2 How text and footnotes share the page (D; VM to confirm)

The pagination state machine (`1068:141e`) interleaves:
1. Lay out main-text lines (`1120:0000`). Each line records its first
   footnote ordinal and how many references it holds (+0x26, +0x28;
   counted in the line builder `10d0:3b66`).
2. When a placed line holds references not yet laid out (`1120:284a`;
   page targets +0x36 = last referenced, +0x38 = last laid out), stop
   and lay out that footnote's lines into the area (states 9–11:
   `12f8:05d8`, then line by line `12f8:1321`), then go on with the main
   text. So a footnote is placed **right after the line that
   references it** and everything below on that page gets the room that
   is left.
3. A footnote line fits while the area's top minus the line's height
   stays at or below the main text's current bottom (the referencing
   line's bottom). When the next line does not fit, the footnote is
   **split**: the area is marked full, the "Continued on" message (if
   on) is appended at its foot, and the rest of the footnote continues
   on the next page's area, after the "Continued from" message (if on).
   The guide agrees: long footnotes, or too little room below a frame,
   "may split the footnote over multiple pages" (M).
4. If not even the separator and one line fit, nothing of the footnote
   goes on this page (G from `12f8:06d0`'s full mark): the reference
   line stays and the footnote starts on the next page. **Whether Ami
   Pro instead moves the referencing line to the next page is open
   (VM).**
5. When a column ends (`10c8:01f7`): footnote lines laid out for
   references whose lines were then pushed to the next page are taken
   back out of the area (`12f8:00ce`), so a footnote never stays on a
   page its reference left.
6. At the end of the main text (state 4), footnotes still pending go on
   following pages that hold only footnotes (state 9 again) (D).

#### 3.3 Continued messages (D)

- Each message is a one-paragraph text in the **Footnote** style
  (`12f8:1a8a`: the paragraph made from the option text, aligned left or
  right by flag `0x20`/`0x40`), built by the line builder (`12f8:1b8e`)
  into the area as its own line(s).
- "Continued on" goes at the bottom of a page whose last footnote
  continues (`12f8:1a48`, needs flag `0x08` and a non-empty text);
  "Continued from" at the top of the next page's area, under the
  separator (`12f8:1dc7`, flag `0x10`).
- The Footnote style's number is suppressed on both messages except
  that Repeat footnote number (`0x80`) keeps it on the Continued from
  message, showing the continued footnote's number (`10d0:1b5f`,
  `1108:4107`). The dialog allows Repeat footnote number only with
  Continued from on and Reset number on each page off.

#### 3.4 Where footnotes can be

- Main text: yes. Table cells (page tables and frame tables): yes, as
  table footnotes at the table's end (M; insertion `14d8:0000` when the
  caret is in a table). Frames other than tables, headers, footers,
  notes, and inside a footnote: **no** (M lists only main text and
  table cells; insertion `1358:0a09` requires the main text stream
  (`DAT_1680_2a21 == 1`) or a table; the dialog disables Insert
  elsewhere, see 4.1).
- **Multiple columns:** one full-width area for the page (3.1); G that
  the footnote of a reference in column 1 goes into the same bottom area
  and shortens column 2 as well (VM).
- **Draft mode** shows no footnotes (string 211: "Display the current
  document without page breaks, headers, footers, and footnotes") (R).
- **Endnotes** (flag 1): no per-page areas; after the main text ends,
  the footnote stream is laid out as a flow of its own (`12f8:1e58`
  positions at the first footnote's first paragraph and lays the
  stream's lines out like body text, `1068:0000`), drawn without a
  separator. **Whether they start right after the last line or on a new
  page, and with any heading, is open (VM).** M: "Places all the
  footnotes at the end of the document instead of at the bottom of each
  page."

---

### 4. Editing

#### 4.1 Tools › Footnotes… (command `0xa0`, `1358:098f`; dialog 1078 "Footnotes", `FOOTNOTEPROC` `AMIPROUI 1130:0000`) (D, R)

- Controls: radio **Insert footnote** (25450), **Edit footnote**
  (25451); OK, Cancel, **Options…** (25585, sends command `0xab`).
- Enablement (from five answers Ami Pro gives the dialog; meanings G
  from their use): Insert is off when the caret is outside the main text
  and not in a table, or in footnote text; Edit is off when the document
  has no footnote (or none from the caret on, G) or the caret is in
  another stream; with the caret **in footnote text** Edit becomes
  **"Return to Text"** (string 2033) and is selected. Options… is off in
  a table. Default: Insert when allowed, else Edit, else focus on
  Options….
- **Insert** (`1358:0a09` → `1358:0000` in the main text; `14d8:0000` in
  a table): makes the footnote record and its first, empty paragraph in
  the Footnote style (`1358:016f`, creating the style if needed), gives
  it ordinal = marks before the caret + 1 and renumbers the rest,
  inserts the 5-byte `F` control at the caret through the text-insert
  routine (`1070:350d`, G: undoable as one step), and **moves the caret
  into the footnote text** (stream 3) at its first paragraph; the view
  goes to the footnote area at the page's bottom (Layout mode) (D, M).
  Esc, or a click in the main text, returns (M).
- **Edit** (`1358:0209`): in the main text, finds the first footnote
  reference at or after the caret (`1358:074a`, `1358:085e`, searching
  paragraphs flagged as holding footnotes) and puts the caret on the
  first character of its text; in footnote text, returns the caret to
  the main text (to the reference, G). Shortcut: click in the footnote
  text in Layout mode (M). Up and Down move from footnote to footnote;
  **a selection cannot span two footnotes** (M).
- Edit › Go To has **Footnote Mark** (string 2106) and **Footnote Text**
  (2107) (R, M).

#### 4.2 Footnote Options… (command `0xab`; dialog 1101 "Footnote Options", `FOOTOPTSPROC` `AMIPROUI 1128:0000`) (D, R)

- Make endnotes (25550); Reset number on each page (25551); Starting
  number (25300, 1–9999); group Separator Line Options: Length: Margins
  (25450) / Custom: (25301), Indent from left (25302), a unit button
  (25650: inches, cm, picas, points; it converts both boxes); group
  Continued Message Options: Continued on message (25552) + text (25303,
  64 chars) + Left/Right (25452/25453); Continued from message (25553)
  + text (25304) + Left/Right (25454/25455); Repeat footnote number
  (25554); OK, Cancel.
- Rules: **Make endnotes disables everything else** in the dialog (and
  unchecks Reset number on each page); Reset number on each page unchecks
  and disables Repeat footnote number; each Continued check enables its
  alignment radios (and From also Repeat number unless Reset is on);
  typing in Custom's box selects Custom. Initial alignment Right when the
  flag bit is clear.

#### 4.3 Deleting, undo, converting

- Deleting a selection holding a reference asks "There is a footnote in
  the selection to be deleted. Would you like to continue?" (string
  3074), then deletes the footnote with its reference (record freed,
  others renumbered, `1350:01b0`) (R, D). Delete on the reference (M:
  "Select the footnote reference number … Press DEL"; a message).
- Undo of insert/delete: G (the insert goes through the general text
  insert with an undo kind; Ami Pro's undo levels; VM).
- No command converts footnotes to endnotes individually: the whole
  document switches by Make endnotes (M, R). Table footnotes never
  become endnotes.
- Find & Replace searches footnotes as the third stream (main text,
  Where placed frames, **footnotes**, …) (`FIND_NOTES.md`; M ch. 20).
- Ami Pro's RTF filter reads `\footnote` (renumbered; endnotes become
  footnotes) (`IMPORT_NOTES.md`); spell check walks them ("Checking
  Footnotes", string 2133) (R).

#### 4.4 Printing

Printed as laid out in Layout mode (the same area painter; page base
for Reset applies when printing, G for `2a49`). Superscript numbers
print superscript only when the printer has the Footnote style's size
(M).

---

### 5. Open questions and the VM fixtures that would settle them

One property per fixture; 8.3 names; each a short synthetic document
made in Ami Pro 3.1 (`_DEFAULT.STY`), saved to `L:\OUT`, with a screen
capture of the page (Layout mode) and, where noted, a print to file.

| Fixture | Made how | Settles |
|---|---|---|
| `FN1.SAM` | New document, one paragraph, Tools › Footnotes › Insert, type "One." | The new Footnote style's record as written (`[algn]`, `[spec]` values 4–8, `[spc]`); `<:F` with one paragraph; the reference's look (superscript size, font); the number's placement; separator position and thickness, gap above/below (measure on the capture) |
| `FN2.SAM` | Three footnotes, the middle one inserted last | Ordinal by text order; renumbering on insertion |
| `FN3.SAM` | FN2, then delete the first reference (and note the message), Undo | Delete message; renumbering; whether Undo restores the footnote |
| `FN4.SAM` | A footnote of ~40 lines on page 1 | Split across pages; where page 1's text ends; Continued messages off |
| `FN4C.SAM` | FN4 with both Continued messages on (Left / Right), Repeat footnote number | Message placement, alignment, style, the repeated number |
| `FN5.SAM` | A reference on page 1's last line with a 5-line footnote | Whether the referencing line moves to page 2 or the footnote starts on page 2 alone |
| `FN6.SAM` | Footnotes on three pages, Reset number on each page | Per-page restart in Layout mode and in the print; Draft mode view |
| `FN7.SAM` | FN2 with Starting number 3 | Start offset on both marks; `[fopts]` value 2 |
| `FN8.SAM` | FN2 with Custom separator 2 in, indent 0.5 in | Length/indent units in `[fopts]` (twips?) and drawing |
| `FN9.SAM` | FN6 with Make endnotes | Endnotes' placement (after last line or new page), separator, numbering, heading |
| `FN10.SAM` | Two-column page layout, a reference in column 1 | Area width (full text width or the column) and its effect on column 2 |
| `FN11.SAM` | Footnote style number changed to `*` (and one with `a`), footnotes on two pages, Reset off | Asterisk counts per page whatever Reset; letters |
| `FN12.SAM` | Footnote style Text "." and Right align, Space for 0.25 in | Reference shows the period; number placement |
| `FN13.SAM` | A footnote with two paragraphs, the second in Body Text | Second paragraph unnumbered; untagged paragraphs inside `<:F` |
| `FNT1.SAM` | Page table 3×3, footnote in B2, then a second in C3 | `<:xN,ROW>` fields, the footnote row(s), their cell records and `[h]` flags, numbering in the table |
| `FNT2.SAM` | A Where placed text frame and a header: try Tools › Footnotes | Insert disabled (dialog state) |
| `FN14.SAM` | FN1, then Tools › Footnotes with the caret in the footnote text | "Return to Text" and where the caret returns |
| `FN15.SAM` | FN1 opened by the app's writer (twin, later) | That Ami Pro reads the app's `<:F` and `[fopts]` |
| `FNSTY.STY` | Save as a Style Sheet from FN6, then Use Another Style Sheet on a new document | Whether a sheet's `[fopts]` is applied |

Also: whether changing Footnote Options can be undone (any fixture,
Edit › Undo after OK), and Up/Down between footnotes (observation).
