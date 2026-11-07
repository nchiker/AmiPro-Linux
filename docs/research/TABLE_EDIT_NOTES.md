# Editing tables in Ami Pro 3.1: evidence (2026-10-24)

For the table editing run (the Phase 5 queue's run 8,
`docs/playbooks/TABLES.md`, "Run 2"). Decompiler output stays outside Git
(`../AmiPro-Reference/ghidra-work/out/`, with this run's extra decompiles
`structure-decomp.c` and `cells-decomp.c`); these are findings in my own
words. Evidence levels as in `SAM_FORMAT_NOTES.md`: VERIFIED (decompile),
VERIFIED (fixture) for the VM fixtures TE1–TE20 below, VERIFIED (manual)
for the 3.1 user's guide ch. 16, INFERRED otherwise. Earlier evidence this
builds on: `CELL_FRAME_NOTES.md` (the grid, frames in cells),
`TABLE_PAGES_NOTES.md` (headings, page breaks, Rows span pages, the row
cache), `EXPORT_NOTES.md` ("Lines & Color").

## The commands and where they are

| Command (ID) | Handler (`AMIPRO.EXE`) | Dialog (`AMIPROUI.DLL`) |
|---|---|---|
| Tools › Tables… (0x1c9) | `1478:0000` → `1478:010a` → create `1478:0afe` | `CREATETBLPROC` `1398:0000`, template 1139 |
| Table › Modify Table Layout… (0x272) | `1478:010a` → modify `1478:0397` | `TBLLAYOUTPROC` `13a0:0000`, template 1085 |
| Lines & Color… (0x278) | `14b0:0000`, per cell `14b0:067c` | `TBLLINESPROC` `13a8:0000`, template 1091 |
| Insert Column/Row… (0x273) | `1438:1afa`, worker `1438:1e1a` | `INSCOLROWPROC` `1388:0000`, template 1086 |
| Delete Column/Row… (0x274) | `1430:1cfe`, worker `1430:1f60` | `DELCOLROWDLGPROC` `1380:0000`, template 1087 |
| Delete Entire Table (0x28b) | `1430:2feb` → `1430:30a4` | — |
| Column/Row Size… (0x27a) | `13d0:0192` | `SIZECOLROWPROC` `1390:0000`, template 1103 |
| Select Column / Row / Entire Table (0x28c–0x28e) | `14a0:1273`, `14a0:1460`, `14a0:160f` | — |
| Connect Cells (0x27b) | `1458:0000` (`04df` join, `072f` split) | — |
| Headings (0x277) | `1468:0000` | — |
| Leaders ▸ (0x27f–0x282) | `13e8:2790` | — |
| Protect Cells (0x27e) | `1460:0000` | — |
| Edit Formula… (0x275), Quick Add ▸ (0x283, 0x284) | the table-data run (queue 20) | `TBLEDITFORMULAPROC` |

Menu state: `1180:2153` (in a table), `1180:133d` (Tools › Tables…),
`1180:1ce4` (check marks and Connect's text), `1180:1e80` (Connect with a
cell selection). The Table menu (MENU 1057) appears in the menu bar, after
Tools, only while the caret is in a table (TE1).

## In the file and in memory

**`[tbl]` header** `ROWS COLS ROWHEIGHT ROWGUTTER COLWIDTH COLGUTTER V7
TOPINSET LEFTINSET` (reader `1428:0000`; memory `+0x0e`, `+0x12` (a
byte: at most 255 columns), `+6` (0 in memory while Automatic), `+0xa`,
`+8`, `+0xc`, `+0x13`, `+0x19`, `+0x1b`). VERIFIED.

**Value 7 bits** (VERIFIED (decompile) `1478:0397`, `TBLLAYOUTPROC`;
VERIFIED (fixture) TE12–TE15, TE17):

| Bit | Meaning | Fixture |
|---|---|---|
| `0x1` | Automatic row height | TE17: off → 0 |
| `0x4` | Center table on page | TE13: 5 |
| `0x80` | Honor protection | TE12: 129 |
| `0x100` | Rows span pages (page tables, needs Automatic) | TP5 |
| `0x400` | Disable mouse col/row sizing | TE15: 1025 |
| `0x10`, `0x8`, `0x200` | in memory only: re-measure pending, first redraw, has headings | — |

**Line around table** is not a table bit: it is the table frame's own
line, frame flag `0x10000` with the line's style in the frame's line
fields, and the frame's four interior margins set to the line's width
(`14b0:133e`); page tables only. TE14 (style 3): flags `0x1900C4`, line
style 3, `[frmlay]` margins 40, the frame 80 taller. VERIFIED.

**`[h]` / `[w]` records** `INDEX SIZE GUTTER FLAGS x` (rows `… x COUNT 0`),
one setter `13d8:04ea`, created on demand from the header's defaults:

| Flag | Meaning | Evidence |
|---|---|---|
| `0x1` | the row or column holds part of a joined block of more than one row or column | VERIFIED (decompile `1458:041b`); TE8 |
| `0x2` | own size (Column/Row Size, a mouse drag, a measured Automatic row) | TE5 `[w] 1 2160 86 2 0` |
| `0x4` | own gutter | TE6 `[w] 1 3017 288 4 0`; TE7 `[h] 1 284 144 6 0 3 0` |
| `0x8` | (rows) set at reading when Automatic is on and `0x2` set | `1428:046b` |
| `0x10`, `0x20` | heading row, page break before the row | `TABLE_PAGES_NOTES.md` |
| `0x40`, `0x80` | inserted / deleted under revision marking | `1438:0b5e`, `1430:2579` |

A row record's sixth value is the number of cells starting in the row
(TE3: 2 of 2 columns; TE8: 2 and 1 with a 2 × 2 block); the reader
discards it (`1428:046b`), so it is a cache. Ami Pro writes `[h]` records
with flag 2 for rows it measured (TE2–TE7).

**Cell records** `ROW COL F3 F4 F5 F6 F7 F8 F9 F10 F11 F12` (reader
`1428:07ce`, writer `1490:07c0`):

| Value | Meaning | Evidence |
|---|---|---|
| F3 `0x180` / `0x80` | joined block's top-left / covered cell | TE8 |
| F3 `0x200` | the cell has its own fill (F10 kept when it is cleared) | VERIFIED (decompile `14b0:067c`) |
| F3 `0x4000`, `0x8000`, `0xC000` | leader: dots, dashes, line | TE10, TE10B, TE10C (None: 0, TE10D) |
| F3 `0x38` | the cell text's first paragraph's alignment, copied on writing | VERIFIED (decompile) |
| F3 `0x40`, `0x800` | a formula follows; re-measure pending | VERIFIED (decompile) |
| F4, F5 | block rows and columns (top-left), offset (covered) | TE8 |
| F6, F10 | fill: palette index, `COLORREF` | TL4 |
| F7 | line word: left, right, top, bottom, 4 bits each | TL1 |
| F8 | text follows | TE1 |
| **F9** | **protected** (1) | TE11 `1 1 0 0 0 0 0 1 1 0 0 0` |
| F11, F12 | frame in cell | `CELL_FRAME_NOTES.md` |

A cell with no record is an empty default cell; commands that set an
attribute make the record, commands that clear one delete a record left
with nothing (`13c0:2ab2`). A new table has no records until text is
typed (TE2's new row: none). VERIFIED.

## Tools › Tables… (making a table)

- **Dialog** (TE1): Number of columns 5, rows 5 (limits 1–250 and
  1–4000), OK, Cancel, Layout… (Modify Table Layout's dialog with a new
  table's values: column width 1.00 in, gutters 0.06 in, height 0.20 in
  greyed, Automatic on, every option off).
- **Enabled** in the main text, or with one empty text frame selected; greyed
  in table mode, header and footer bands, frame text, a picture frame, a
  frame with text. A selected frame that already holds a table only enters
  it. VERIFIED (decompile `1180:133d`).
- **In the text:** the selection is deleted first; the anchor `<:t n>` goes
  **at the caret, inside its paragraph**: TE1 at the end of "Above the
  table.", TE20 between "Above " and "the table." (the line breaks there:
  the text before, the table, the rest of the paragraph after). No
  paragraph is added. VERIFIED (decompile, fixture).
- **The frame** (TE1): flags `0x1800C4` (`0x1900C4` with Line around), the
  default frame's colours, no shadow, square corners, margins 0; width the
  left inset plus each column's width and gutter, height the top inset plus
  each row's height and gutter (TE1: 43 + 3 × (3017 + 86) = 9352,
  43 + 3 × (284 + 86) = 1153).
- **Column width** without Layout…: the text column less the 43 inset,
  less each column's gutter (at least 86), divided by the columns, after
  Ami Pro's screen rounding (TE1: 3017 for three columns of a 9360 text
  width, 3019 exactly; TB1: 4569 against 4572). With Layout…: 1440 unless
  changed. Row height: one Table Text line (284 for 12 pt Times New
  Roman). Gutters 86, insets 43, Automatic on.
- **Table Text** is made from Body Text when missing (`1098:0509`): no
  indents, no space above or below, no own tabs, no rules, no bullets,
  Next style itself, the next free function key.
- **In a selected empty text frame** (TE19): the frame keeps its rectangle,
  lines, margins, and placement; its flags gain `0x4` and lose `0x200`
  (TE19: `0x100C4`); the columns fill its interior (1867 for two columns of a
  3-inch frame with 0.10 in margins). Line around and Rows span pages do not
  apply.
- After it the caret is in cell (0,0) and the status bar says Table Text.

## Modify Table Layout…

- Enabled whenever the caret is in a table. Fields: Default Columns Width
  and Gutter size; Default Rows Height (greyed while Automatic), Automatic,
  Gutter size; Options Center table on page, Line around table, Honor
  protection, Rows span pages (only with Automatic; cleared when Automatic
  goes off), Disable mouse col/row sizing; Style (12 line styles, for Line
  around); the unit button. Frame tables grey Center, Line around, Rows span
  pages, and Style; with Honor protection on, everything but Honor
  protection and Disable sizing is greyed. Limits: height at least 10
  twips, at most 22 in (page tables) or what fits (frame tables); width
  at most what fits. VERIFIED (decompile).
- **OK sets the header's defaults:** every row and column without its own
  size takes them; columns with their own `[w]` keep theirs (TE16: default
  width 1.50 in, all three columns 2160, the frame 8221 wide). Turning
  Automatic off keeps each row's measured height (TE17: `[h]` 284 flag 2,
  the default 720 only for new rows); turning it on measures again and
  resets the default height to a Table Text line.
- **The dialog rounds in its unit:** OK with nothing changed wrote 3024
  (2.10 in) for 3017 (TE12–TE15). An artefact of the dialog, not a rule.
- Center moves a page table to the middle of its column (Layout mode
  only, `1478:128a`); Line around adds the frame line (above).

## Insert Column/Row…

- Dialog (TE1, TE3): Insert Columns or Rows, Number to insert (1), Position
  Before or After (After). Defaults to Columns when one fits.
- **A column is inserted only where it fits**: the room right of the last
  column, to the column's (or frame's) right edge, divided by the default
  width and gutter. A table filling its column offers only Rows, with "Not
  enough room to insert a column." (TE1). Frame tables offer rows only
  where they fit. Limits 250 columns, 4000 rows. VERIFIED (decompile,
  fixture).
- **The table grows; existing columns never shrink** (TE4: the new column
  3017 wide, the default; the frame 3103 wider). New rows and columns take
  the defaults (no record). Joined blocks containing the place grow; later
  `[h]`/`[w]` records and cells move up their indices. New cells copy the
  caret row's or column's lines and fill (manual; `1438:0000`,
  `1438:1181`). With Before the caret keeps its cell.
- **No undo:** Edit › Undo is greyed right after it (TE2). VERIFIED
  (fixture).

## Delete Column/Row… and Delete Entire Table

- Dialog: Delete column, Delete row (row by default). With a cell
  selection every row or column it covers goes; deleting them all deletes
  the table.
- **A warning first:** "Deleting columns and rows in a table can not be
  undone. Would you like to continue?" (TE3), and indeed no undo.
- Deleting a column narrows the table: the frame loses the column's width
  and gutter, no column widens (TE3: 10792 → 7689). The caret goes to the
  cell now in its place. Joined blocks shrink or go; cells (and their
  pictures) in the deleted rows or columns go.
- **Delete Entire Table** (TE18): no question; the anchor leaves the text,
  its paragraph stays ("Above the table." with nothing after it), the frame
  and everything in it go. A frame table's whole frame goes (as Delete on a
  frame, `1430:30a4`).

## Column/Row Size…

- Dialog (TE5): Columns Width and Gutter size, Rows Height (greyed while
  Automatic) and Gutter size, each with a check box that typing checks;
  the values are the caret cell's; a box is grey when the selected rows or
  columns differ. OK sets only the checked values, for the caret's column
  and row or every selected one: `[w]` width flag 2 (TE5: column 1 2160,
  the frame 857 narrower), gutter flag 4 (TE6: 288; the frame grows),
  `[h]` gutter flag 4 (TE7: 144). Header defaults are never touched.
- Width's maximum is what fits ("Cannot increase table column width if
  the table fills the entire page."); a gutter can push the table past
  the margin (TE6: the frame stored at the margin, the last column's line
  beyond it).
- **By mouse** (`1498:25fa`, `1498:2d2a`): a press in a column's gutter
  (or a row's, only without Automatic; in a joined block only its last
  line) drags that one column's right edge (or row's bottom); release runs
  Column/Row Size on it: the later columns move, the table gets wider or
  narrower. Disable mouse col/row sizing turns it off. The ruler does not
  size table columns.

## Selecting cells

- **A cell selection** is two cells, a rectangle (`14a0:0000`): a drag
  from one cell into another (TE8's drag from B1 to C2 inverted the four
  whole cells, gutters included), Shift+click, Shift+Ctrl+arrows, a strip
  above a column or left of a row (that column or row), Select Column,
  Select Row, Select Entire Table. Select Column needs two rows and Select
  Row two columns. Connect, Headings, and the Select commands grow the
  rectangle to whole joined blocks (`14a0:1052`); Protect, Leaders, and
  Lines & Color take it as it is.
- **Typing** with cells selected drops the selection and types in the
  caret's cell. **Delete** asks, then empties every cell (text, picture,
  formula) but rows and columns stay. **Copy** keeps the cells relative to
  the top-left (and the table itself when all of it is selected) plus a
  text version (tabs between cells, returns between rows); **Cut** is Copy
  then Delete. **Paste** of cells overwrites from the caret's cell (or a
  selected range, repeating the block when the range is a multiple of it,
  else "Shade range does not match copied data range."); columns past the
  last are dropped; rows are added to a page table, a frame table stops.
  Cells pasted outside a table are tabbed text, a whole copied table a new
  table. Tabbed text pasted into a table fills cells from the caret's.
  `1240:2008`, `13b0:4065`, `14c8:044f`. VERIFIED (decompile).
- **Drag and drop** of a selection (`1498:2d2a` → `14e0:0000`): whole
  columns or rows are moved (or with Ctrl copied) between the columns or
  rows on either side of the drop; a block of cells overwrites the cells
  at the drop ("Data in the destination cells will be lost. Continue?"),
  never adding rows or columns. No undo (manual).

## Connect Cells, Headings, Leaders, Protect Cells

- **Connect Cells** (TE8, TE9): greyed without a cell selection unless the
  caret is in a joined block, where the item reads **Disconnect Cells**.
  The top-left cell gets `0x180` with the block's size; the others `0x80`
  with their offsets; the rows and columns flag `0x1`. **The covered cells
  keep their text**, written in the file and hidden; Disconnect shows it
  again (TE9). Blocks the selection crosses are taken in whole. Greyed with
  a protected cell under Honor protection, a mix of heading and other rows,
  rows of two page parts.
- **Headings**: page tables only; the first row of the range decides on or
  off for all its rows (`[h]` `0x10`); checked by the first row.
- **Leaders ▸** `...`, `- - -`, `___`, None (F3 `0x4000`, `0x8000`,
  `0xC000`, 0; TE10–TE10D); no check marks. Drawn on the cell's last line:
  after the text to the cell's right edge (left or justified), from the
  cell's left edge to the text (right aligned), none for centred text,
  across the whole cell when it is empty, none with a picture (`13e8:2422`).
  TE10: dots after "B2" to the column line.
- **Protect Cells** (TE11): F9 = 1; a toggle decided by the first cell;
  checked by the caret's. **Honor protection** (TE12): the caret never
  enters a protected cell (a click or Tab goes to the next one: typing went
  to C2), Leaders, Lines & Color, Connect, paste, and drop skip or refuse
  protected cells.

## Lines & Color…

Mapped in the export run (`EXPORT_NOTES.md`): line position All, Left,
Right, Top, Bottom, Outline; 12 styles; Fill color; lines black. Added
here (`14b0:067c`, `13a8:0159`):
- A side's box starts grey when any selected cell has that line, clear
  otherwise; All and Outline start clear; the style shown is the last one
  found. Grey keeps, checked sets, unchecked removes.
- Lines belong to each cell; a joined block's live in its top-left record;
  Outline sets the outer sides of the rectangle.
- A line wider than its gutter or inset widens it (own gutter, flag 4;
  never narrowed); widths in twips by style: 1 10, 2 20, 3 40, 4 80, 5 120,
  6 160, 7 240, 8 60, 9 100, 10 120, 11 100, 12 100.
- Fill sets F3 `0x200` and the colour; unchecked clears the bit only.

## Moving around (`1408:0000`)

- **Tab, Shift+Tab** go to the next or previous cell row by row, skipping
  covered cells (and protected ones under Honor protection). **Tab in the
  last cell leaves the table** to the text after its anchor (TE1: "X" typed
  there landed below the table); no row is added. In a frame table it does
  nothing. VERIFIED (fixture, decompile).
- Ctrl+arrows go a cell at a time; Home, Home and End, End to the row's
  first and last cell; plain arrows move in the text and on into the next
  cell. Enter adds a paragraph in the cell; Backspace at a cell's start and
  Delete at its end join nothing (TE3: Backspace at "C2"'s start removed the
  empty paragraph above it, in the cell). A typed tab is swallowed in a
  table.

## Undo

Ami Pro undoes none of the table commands: Undo is greyed right after
Insert Row (TE2) and Protect Cells (TE11); deleting rows and columns warns
that it cannot be undone (TE3); the manual says moving cells, rows,
columns, and pictures cannot be undone either.

## Fixtures (`../AmiPro-Reference/vm-lab/OUT`, 2026-10-24)

Each from TE1 (a 3 × 3 table under "Above the table.", A1…C3 typed with
Tab), one change, saved under a new name. Screenshots `te*`, `c-te*` in
`screens/`; the helper `vm-lab/te.sh` (outside Git).

| Fixture | Change | Shows |
|---|---|---|
| TE1 | Tools › Tables 3 × 3, text typed | the new table's header, frame, records; Tab in the last cell leaves the table |
| TE2 | Insert 1 row After in row 1 | no records for the new row; frame 370 taller; no undo |
| TE3 | Delete column 1 | the warning; the frame narrows; `[h]` records with their cell counts |
| TE4 | TE3, Insert 1 column Before | the default width, the table wider |
| TE5 | Column/Row Size: column 1 Width 1.50 in | `[w]` flag 2 |
| TE6 | column 1 Gutter 0.20 in | `[w]` flag 4; the table past the margin |
| TE7 | row 1 Gutter 0.10 in | `[h]` flags 6 |
| TE8 | B1–C2 connected | `0x180`/`0x80`, the covered cells' text kept; `[h]`/`[w]` flag 1 |
| TE9 | TE8 disconnected | the cells and their text back |
| TE10–TE10D | Leaders dots, dashes, line, None on B2 | F3 `0x4000`, `0x8000`, `0xC000`, 0 |
| TE11 | Protect Cells on B2 | F9 1; no undo |
| TE12 | TE11, Honor protection | value 7 `0x80`; the caret skips B2 |
| TE13 | Center table on page | value 7 `0x4` |
| TE14 | Line around table, style 3 | frame `0x1900C4`, margins 40 |
| TE15 | Disable mouse col/row sizing | value 7 `0x400` |
| TE16 | TE5, default width 1.50 in | defaults apply to the other columns |
| TE17 | Automatic off, height 0.50 in | rows keep their measured heights |
| TE18 | Delete Entire Table | no question; the anchor gone, the paragraph kept |
| TE19 | Tools › Tables 2 × 3 in an empty selected frame | a frame table: flags `0x100C4`, the frame kept |
| TE20 | TE18, Tools › Tables 2 × 2 with the caret mid-paragraph | the anchor mid-paragraph; the line breaks around the table |
