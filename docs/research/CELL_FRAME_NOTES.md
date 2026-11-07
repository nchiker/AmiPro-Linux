# Frames in table cells in Ami Pro 3.1: evidence (2026-10-18)

For the frames in table cells run (`docs/CELL_FRAMES_RUNBOOK.md`, slice
CF.1). Decompiler output stays outside Git (`../AmiPro-Reference/ghidra-work/out/`);
these are findings. Evidence levels as in `SAM_FORMAT_NOTES.md`: VERIFIED
(decompile), VERIFIED (fixture), VERIFIED (manual) for the 3.1 user's
guide, chapter 16 ("Using Tables for Columnar Text, Data, and Pictures"),
CORPUS for both corpora (aggregate counts only).

## What a frame in a cell is

A table cell holds text **or** one frame: a picture, drawing, chart, or
equation that fills the cell. The user's guide: "You can import a picture
into a table cell that does not contain data"; the same for a drawing, a
chart, and an equation; "To delete a picture in a cell: select the cell
that contains the picture, press DEL"; "Deleting columns, rows, and
pictures … cannot be undone". Every frame in a cell in both corpora is a
picture (108: 94 in 21 of the user's files, 14 in 2 ISO files), and every
one is in a page table (none in a table inside a frame). CORPUS.

## Where the code is

| Routine | Role |
|---|---|
| `1248:0347` (EXE) | Puts a frame in one of Ami Pro's three page-frame lists by kind: anchored (flag `0x80000`) to `DAT_1680_234a`, in a cell (`0x20`) to `DAT_1680_234c`, others to `DAT_1680_2348` in page order |
| `11c8:3ba0` (EXE) | The writer's frame part: the cell list first, then the Where placed list, then the anchored list, then the groups' lists (`DAT_1680_2342`) |
| `1058:06cc`, `1058:1a7d` (EXE) | The reader: each `[frm]` read by `1a7d` and listed by `1248:0347` with kind 2 for `0x20` (appended: file order kept) and 1 otherwise; `1a7d` hands the table reader the cell list's head |
| `1428:0000`, `1428:046b`, `1428:07ce`, `1428:0e03` (EXE) | The table reader: the `[tbl]` header, the `[h]`/`[w]` records, each cell record, and the frame a cell names (the n-th of the cell list) |
| `1490:07c0`, `1490:0708` (EXE) | The cell writer: values 11 and 12, the frame's place in the cell list |
| `14d0:0000` (EXE) | Puts a frame into a cell: the cell's rectangle, the frame's settings, the front of the cell list, the cell's flag and link |
| `14d0:05ec`, `14d0:06f6`, `14d0:06bb` (EXE) | A new frame in the caret's cell (for Import Picture, Drawing, Charting, Equations, Paste, New Object), the caret cell's frame, and removing a cell's frame |
| `1488:0000` (EXE) | A cell's rectangle (a joined cell's columns and rows with the gutters between them) |
| `1488:0e10`, `1488:14f0` (EXE) | The caret entering a frame's cell: the frame marked current, and the cell outlined (below) |
| `13b0:1ae9`, `13b0:1f29`, `13b0:1fd5` (EXE) | Pasting frames: the cell frames that follow a table on the clipboard are split off its list (`1f29`) and put at the front of the cell list (`1fd5`); a frame pasted with the caret in a table cell goes into the cell (`14d0:0000`) |

`1238:0ca4`, listed earlier as a test of the bit, tests bit `0x20` of a
**line** record (a page's extent for display), not of a frame: it has
nothing to do with cells. VERIFIED (decompile).

## In memory and in the file (VERIFIED (decompile), (fixture) TF1, TF11)

- A frame's flag word is at offset 2 (the file's field 2); bit `0x20` says
  the frame is in a cell, and the reader puts such a frame in the cell
  list (`DAT_1680_234c`), in file order.
- A cell record's flag `0x1000` (offset 0) says the cell holds a frame,
  and offset 6 is the frame. The cell writer writes them as the record's
  **values 11 and 12**: 1 and the frame's place in the cell list, counting
  from 0 (`1490:0708`); otherwise 0 and 0. The reader takes value 12 only
  when value 11 has bit 1, and finds the frame by its place in the list
  read so far; cell frames come first in the file, so the list is
  complete when the table is read.
- **The list's order:** a frame put into a cell goes to the **front**
  (`14d0:0000`), so the newest is written first and numbered 0 (TF11: the
  picture imported last into cell 0,2 is the first record and cell 0,2
  says `1 0`; cell 1,2 says `1 1`). README31's three pictures, `Frame9`,
  `Frame8`, `Frame7` in that order, are numbered 0, 1, 2 by cells (1,0),
  (0,1), (0,0).
- **The file's frame order** is the cell list, then the Where placed
  frames, then the anchored ones (TF15: a frame made with Create Frame
  after the pictures is written after them, before the table).
- A frame put into a cell (`14d0:0000`, TF1): flag `0x20` set; text
  (`0x200`), lines (`0x10000`), and anchored (`0x80000`) cleared; no
  shadow, square corners, no margins; opaque and wrap from the default
  frame (Import Picture: `0x200000E0`, opaque, Wrap around, picture).
  "Ami Pro makes the table cell transparent or opaque depending on the
  default frame layout setting" (manual).

## Where the frame goes (VERIFIED (fixture), CORPUS)

The frame covers its cell's **text area**: the cell's column widths and
the gutters between them, by its row heights and the gutters between
them, from the cell's text corner. The text corner of column c is the
table's left inset plus each earlier column's width and gutter; of row r,
the top inset plus each earlier row's height and gutter.

- **Insets:** `[tbl]` header values 8 and 9, top and left (43 and 43 in
  every table Ami Pro 3.1 made, 0 and 0 in 7 older ones; absent in 24,
  which the reader takes as 43: `1428:0000`). TF14X: value 8 set to 300
  moved the whole grid 257 twips down and nothing across, and Ami Pro
  wrote the frame 300 below the table's top (TF14XS).
- **Gutters:** each column's is its `[w]` record's value 3 when that
  record's flag word (value 4) has bit 4, else the header's value 6; each
  row's the same from `[h]` and the header's value 4 (`1428:046b`
  reads index, size, gutter, flags; READMEBP's rows have `[h]` gutter 72
  with flags 2, and Ami Pro lays them out at the header's 230 and writes
  them so: BPS, its re-save in the VM).
- TF1: a 3 × 2 table of 3017-twip columns, 284-twip rows, gutters 86, at
  1440, 1724: the picture in cell (0,1) is 4586, 1767 to 7603, 2051 —
  1440 + 43 + 3017 + 86 across, 1724 + 43 down, one column by one row.
  TF13 (two connected cells): 3017 + 86 + 3017 wide. TF14 (gutters 432):
  3355 = 1440 + 43 + 1440 + 432.
- **The first column's text is 43 twips in, whatever the gutter;** the
  gutter follows each column. The grid lines fall in the middle of each
  gutter (TF14, measured on screen), so a cell's text is not centred in
  it unless the gutter is 86: with 432, cell (0,0)'s text is 43 from the
  table's left edge and 216 from the line on its right.
- CORPUS, by this rule: 90 of the 108 frames' stored rectangles are their
  cells' text areas to the twip; 10 were never laid out (page 0, zero
  height, in two of the user's files); 6 are in rows the table continues
  on a later page; 2 are 10 and 288 twips off (one file each, not
  examined).

**The app's tables are laid out otherwise today:** each cell's text is
inset by half the header's gutters on every side, from the table's edge,
and `[w]`/`[h]` gutters and the insets are not read (`engine.rs`
`table_grid`, `table`). That agrees with Ami Pro only for gutters of 86
and insets of 43: 120 of the 389 tables in the user's corpus (79 files)
and 41 of the 214 ISO tables (23 files) have other gutters. CORPUS.

## How the frame follows its table and cell (VERIFIED (fixture))

- **The table moves:** two lines typed above the table moved it and the
  frame 568 twips down, and Ami Pro wrote the frame's new rectangle (TF2);
  a page break before the table moved both to page 2, the frame written
  with page 2 and its new place, while the anchored table kept its stale
  page field 1 (TF10).
- **The row grows:** three lines typed in the next cell made the row 852
  tall, and the frame with it (TF3: 4586, 2335 to 7603, 3187); the
  picture is drawn larger to fit (pictures are scaled into their frames,
  not part of this run).
- **On opening,** Ami Pro takes a cell frame's page and corner from its
  cell, whatever the file says, and keeps the stored width and height:
  TF1Y (page 3, 100, 100 to 1000, 1000) opened with the picture at its
  cell's corner, 900 × 900, over the row below, and was written with page
  1 and 4586, 1767 to 5486, 2667. A frame stored with no height (TF1Z, as
  the ten never laid out) shows nothing and is written back so.
- **Ami Pro fits one side at a time:** after two lines typed in the next
  cell, TF1Y's 900 × 900 frame became as tall as the row (568) and stayed
  900 wide (TF1YT). The width follows the column's (Column/Row Size) in
  the same way; not checked separately.

## What editing does (VERIFIED (fixture), (decompile), (manual))

- **A click on the picture puts the caret in the cell**, no frame is
  selected, and Ami Pro outlines the cell: four inverted bands 43 twips
  wide around the cell's text area (`1488:14f0`), while the frame is
  marked as the caret's (`1488:0e10`, frame byte 6 `0x20`). The status bar
  says Table Text.
- **Frame › Modify Frame Layout, Group, Bring to Front, and Send to Back
  are disabled** with the caret there; Graphics Scaling is enabled (TF5).
  The frame cannot be moved or resized except through its cell.
- **Del deletes the picture** at once, without a question (TF7); Undo does
  not bring it back (TF7, and the manual's caution). Backspace does
  nothing. The cell becomes an ordinary empty cell (its record `… 0 0`).
- **Typing in a picture's cell does nothing:** nothing appears and the
  cell gets no text (TF12). Three of the user's 94 cells hold both a frame
  and text (made some other way); Ami Pro draws the frame over the text.
- **Import Picture is disabled in a cell with text** (TF12: the File menu),
  so pictures go only into empty cells.
- **Create Frame and drawing a frame always make a page frame**, even with
  the caret in a cell; a Where placed frame over a cell's picture is drawn
  above it (TF15).
- **Deleting a table** deletes its frames (Table › Delete Entire Table; and
  the cell list holds nothing else).
- Moving pictures between cells is Cut, Copy, Paste, and Drag & Drop of
  cells (manual; `13b0:1ae9` keeps a copied table's cell frames with it).
  Table editing is not built in the app.

## The app before this run

The importer reads a frame in a cell as a Where placed frame on its stored
page and rectangle, keeping flag `0x20` and the cell's values 11 and 12 as
read. So: a table the app moves leaves its pictures behind; the frames are
Wrap around frames on the page, so the main text wraps around them where
they were; they can be selected, dragged, resized, grouped, restacked, and
deleted like page frames; and a frame deleted or restacked in the app
breaks the kept numbers of the cells after it (Ami Pro would then show
another picture in a cell, or none).

## Fixtures (`../AmiPro-Reference/vm-lab`, 2026-10-18)

| Fixture | What | Shows |
|---|---|---|
| TF1 | A 3 × 2 table under a line; `PIC.BMP` imported into cell (0,1) | The frame record (`0x200000E0`, no lines, margins, or shadow) and its rectangle, the cell's `1 0` |
| TF2 | TF1 with two lines added above | The frame moves with the table and is written so |
| TF3 | TF2 with three lines typed in cell (0,0) | The frame grows with its row |
| TF7 | TF3 with the picture deleted (Del in its cell) | No frame; the cell `0 0`; no undo |
| TF10 | TF11 with a page break before the table | Both frames on page 2 with the table |
| TF11 | TF7 with pictures in cells (1,2) then (0,2) | The newest is first and numbered 0 |
| TF12 | TF10 after typing in a picture's cell | Typing ignored; Import Picture disabled in a cell with text |
| TF13 | Two connected cells with a picture | The frame spans both columns and the gutter |
| TF14 | Gutters of 0.30 in (432), text in cell (0,0), a picture in (0,1) | The first column 43 in, then width and gutter; grid lines mid-gutter |
| TF14X, TF14XS | TF14 with header value 8 set to 300 (synthetic, in `IN`), and Ami Pro's save | Value 8 is the top inset |
| TF15 | TF11 with Create Frame over the picture in cell (0,2) | A page frame, drawn above the cell's frame; file order |
| TF1Y, TF1YS, TF1YT | TF1 with the frame's page and rectangle changed (synthetic), Ami Pro's save, and two lines typed in the next cell | Page and corner from the cell, size kept; the height refitted to the row, the width not |
| TF1Z, TF1ZS | TF1 with the frame on page 0 with no height (synthetic) and Ami Pro's save | Shows nothing; written back without a height |
| BPS | READMEBP.SAM (ISO) saved again by Ami Pro | Rows laid out at the header's gutter where `[h]` has no bit 4 |

Screenshots `tf*`, `bp-*` in `screens/`. The picture is the lab's synthetic
`IN/PIC.BMP` (64 × 48).

## Twins (CF.7, VM, 2026-10-18)

Made in the app (`cell_frame_twins`, ignored) from the fixtures above and
written as `ATF*.SAM` in `IN`:

| Twin | Made from | Against Ami Pro's | In Ami Pro |
|---|---|---|---|
| ATF2 | TF1, two lines above the table | TF2: the same (flags, sizes, cell records, text) | The picture in its cell, the table two lines down (`atf2`) |
| ATF3 | TF2, three lines typed beside the picture | TF3 but for the row and the picture's heights, which the app's file leaves at 284 | Ami Pro grows the row and fits the picture to it when it opens the file; saved again (ATF3S) it is TF3 to the twip (`atf3`) |
| ATF7 | TF3, Delete in the picture's cell | TF7: the same | The cell empty (`atf7`) |
| ATF10 | TF11, a page break before the table | TF10 but for a row record at the default height | Both pictures in their cells on page 2 (`atf10-p2`) |
| ARM31 | README31, a page break before the SmartIcon table (as EDK1) | — | The three icons in their cells at the top of page 13 (`arm31-p13`) |

A frame's page and corner, and the heights of rows grown by typing, are
Ami Pro's layout: it recomputes them from the cell when it opens a file
(TF1Y, ATF3S), so the app writes them as read and its files open as Ami
Pro's do.
