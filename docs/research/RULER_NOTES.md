# The current ruler: evidence (RU.1, 2026-10-13 to 2026-10-17)

For `docs/RULER_RUNBOOK.md`. Decompiler output stays outside Git
(`../AmiPro-Reference/ghidra-work/out/amiproui.c`, `amipro.c`, and
`amipro-cmds.c`); these are findings. Evidence levels as in
`SAM_FORMAT_NOTES.md`: VERIFIED (decompile), VERIFIED (fixture).

## Where the code is

The ruler control is in `AMIPROUI.DLL`; what a gesture does to the
document is in `AMIPRO.EXE`.

| Routine | Role |
|---|---|
| `TABRACKPROC` `1070:0000` (DLL) | The ruler's window procedure (also the tab rulers inside dialogs); window word 8 = 1 is the current ruler |
| `10d0:090c` (DLL) | Press: hit test. Above half the height: tabs and indents; below: margins, column edges, column bodies, gaps |
| `10d0:17a2` (DLL) | Indent markers: All (the bar), First (upper triangle), Rest (lower triangle), Right |
| `10d0:120d` (DLL) | Drag: new position = snap(start + pointer delta), clamped to the marker's limits |
| `10d0:1a40` (DLL) | Snap: truncate to 90 twips (inches), 57 (cm), 120 (picas, points) |
| `10d0:1c0c`, `10d0:1dff` (DLL) | A tab released off the ruler (above it or below the tab area) is deleted |
| `1070:0935` (DLL) | Click to add a tab: unsnapped, at most 22, refused when its marker overlaps a neighbour's |
| `10d0:0458` (DLL) | Release: the change as a word list sent to `AMIPRO.EXE` as command `0x32a` |
| `10d8:0000` `TABBOXPROC`, dialog 2144 (DLL) | The tool row: tab types, leader, Cols, unit, Clear Tabs, position |
| `1148:06e5` (EXE) | Command `0x32a`: reads the word list's first word and dispatches |
| `1148:081a` (EXE) | Kind 1: column and margin edges (`1148:0c67` page, `1148:0d0c` sets the record; tables: `13d0:0192`) |
| `12e8:008a`, `12e8:01da` (EXE) | Kinds 2–5: indents, per selected paragraph |
| `1148:1329` (EXE) | Kind 6: tabs (move, delete, add, clear) |
| `1148:1833`, `1148:1937` (EXE) | Kind 7: Cols (the column count) |
| `1170:036c`, `1170:052d` (EXE) | One tab list made the selected paragraphs' own ruler |
| `1170:0245` (EXE) | Page › Ruler › Insert (`0x173`) and Remove (`0x174`) |
| `11b0:1405` (EXE) | The paragraph writer: `<:R…>`, or `<:R>` after a ruler paragraph |

**How the handler was found.** `AMIPRO.EXE` dispatches menu commands
through a table in DGROUP at 0x46 (command, offset, segment;
`1010:0b07`), and about 320 of its handlers are reached only through it,
so Ghidra's analysis missed them (`1148:06e5` was inside another
function). `tools/ghidra/command_table.py` lists the table and
`tools/ghidra/DecompAt.java` decompiles the listed addresses
(`DECOMPILATION.md`). Menu command IDs come from `AMIPROUI.DLL`'s menu
resources (Text › Normal `0xfd`, Indention `0x109`, Page › Ruler › Insert
`0x173`, Remove `0x174`, View › Hide Ruler `0xcf`).

## The word lists (`10d0:0458`, `1070:0935`, `1148:06e5`)

VERIFIED (decompile). Each release sends one list:

| First word | Then | Gesture | Drag kind |
|---|---|---|---|
| 1 | `1, column, left, right` | a column edge (2, 4) or a column body (0x100): that column's new edges; the left margin is column 1's left edge, the right margin the last column's right edge | 2, 4, 0x100 |
| 1 | `2, column, left, right, column + 1, left, right` | a gap: both columns' edges | 0x800 |
| 1 | `1, 0, top, bottom` | the vertical ruler's margins (column 0) | 0x200, 0x400 |
| 2, 3, 4 | `value` | All, First, Rest | 8, 0x10, 0x20 |
| 5 | `value` (not below 0) | Right | 0x80 |
| 6 | `index, position` | a tab moved | 1 |
| 6 | `index, 0xFFFF` | a tab deleted (released off the ruler) | 1 |
| 6 | `index, 0xFFFE, position, type` | a tab added | click |
| 6 | `index, 0xFFFD` | Clear Tabs | tool row |
| 7 | `count` | Cols | tool row |

Columns are numbered from 1; positions are twips in the box's
coordinates.

## What each change does (`AMIPRO.EXE`)

VERIFIED (decompile) unless marked.

- **Tabs** (`1148:1329`, `1148:14c5`, `1148:1c21`). The list edited is a
  copy of the tabs in effect at the caret's paragraph (or the paragraph
  before it when a selection ends at the very start of the caret's
  paragraph): its own ruler if it has one, else its style's own tabs,
  else the box's one list. Delete removes the stop; Clear Tabs empties
  the list; add inserts the stop at the given index, refused at 22
  stops; move sets the stop's position. The resulting list becomes the
  own ruler of **every** selected paragraph (`1170:052d`), the same list
  for each; a selection that ends at the start of a paragraph leaves that
  paragraph out. VERIFIED (fixture) RU1, RU3, RU4.
- **Indents** (`12e8:01da`, `12e8:056d`). One value changes; each selected
  paragraph keeps its own other three (its override's, else its style's),
  so the result is per paragraph. Written as `<:I…>`. VERIFIED (fixture)
  RU5–RU8.
- **Column edges** (`1148:0d0c`). Column k's left edge is set; when k is 1
  the left margin becomes that edge. Column k's right edge is set; when k
  is the last column the right margin becomes the page width less that
  edge. Nothing else moves: a margin drag moves only the outer edge
  (RU10), an edge drag only that edge (RU9), a gap drag the two edges
  around it (RU2), a column drag both its edges. Inside a frame, the
  values are offset by the frame's interior origin; inside a table
  (column ≠ 0), the change goes to the table's column widths instead
  (`13d0:0192`). With Mirror, the other page kind follows (`1148:1ba5`).
- **Cols** (`1148:1937`). Always even with a 240-twip gutter (0 when the
  gutters would not fit): w = (W − L − R − (n − 1)·240) / n, integer
  division, the first column at the left margin. The gaps in effect are
  not kept. VERIFIED (fixture) RU11 (750-twip gap, Cols 3: gutter 240)
  and RU12 (RU9 then Cols 3: `1440 4400`, `4640 7600`, `7840 10800`).
- **The gutter is not stored** (`AMIPROUI.DLL` `1238:0000`, `1238:008a`).
  A page or frame record holds each column's edges and no gutter; Ami
  Pro works it out when it needs one: 240 for one column, else the first
  gap, or 0 when the gaps are not all alike (within a twip). A column
  count change spreads the new columns with that gutter, 240 when it is 0.
  The app's reader takes the first gap (240 for one column), so the
  model's gutter is kept to that whenever it cannot be anything else (one
  column, uneven columns; `LayoutBox::settle_gutter`), or Save As lists a
  page layout (found in the physical check, 2026-10-17).
- **Page Layout's margins with unequal columns** (`AMIPROUI.DLL`
  `1238:02d0`, `1238:0000`): when every gap is alike (within a twip), a
  margin change makes the columns even again; otherwise it moves only the
  outer edge (column 1's left, or the last column's right). The ruler's
  margin drag never respaces.
- **Page › Ruler › Insert** copies the tabs in effect into the selected
  paragraphs' own ruler; **Remove** takes it off (`1170:0245`).
- **Text › Normal** (`12e0:11cc`, `12e0:132b`): reverts character
  formatting, alignment, indents, and spacing, and **keeps** the
  paragraph's own ruler: the override block is freed only when no ruler
  remains (`12e8:01da`, `1298:0167`).
- **Enter** carries the ruler on (user's guide, ch. 8).
- **Written** (`11b0:1405`): a paragraph with its own ruler writes
  `<:R1,COUNT,TYPE,POS,…,>`; a paragraph without one, after a paragraph
  with one, writes `<:R>` (which sets nothing when read).

## Limits (`10d0:090c`, `10d0:17a2`, `10d0:120d`)

- Tab hit area: its marker to 29 twips past it. **Tabs are tried before
  the indent markers** (`10d0:090c` loops over the stops, then calls
  `10d0:17a2`), so a press on a tab that stands under an indent marker
  takes the tab; only stops between the leftmost indent and the right
  indent are taken. A tab drags between its neighbours (the next one
  less 1) and within the indents.
- A click adds a tab only between the leftmost indent and the right
  indent; it is not snapped.
- First and Rest are offsets from All (not below it); All is not below
  the column's left edge; Right is not past the column's right edge.
- A column edge stays within its neighbours (the previous column's right,
  the next column's left, the page width); a gap or a column moves until
  a neighbouring gap or column closes.
- Drags snap: the marker's new position is truncated to a multiple of 90
  twips (1/16 in) in inches; a limit that is not on the grid is rounded
  inward to it, or taken exactly when the marker's flags say so.

## VM fixtures (base RUB: four Body Text paragraphs, Standard layout)

Mapping at 91% view: x = 195.6 + twips / 16.47 (page edge at 195.6 px);
ruler top half y ≈ 97, bottom half y ≈ 112.

| Fixture | Gesture | Ami Pro wrote | Agrees with the decompile |
|---|---|---|---|
| RU1 | click a tab at 3.24 in | `<:R1,13,…,1,3228,…>`: the column's 12 stops copied plus the new one, unsnapped | yes |
| RU2 | Cols 2, drag the gap to 5 in | edges `1440 7050`, `7290 10800` (gap kept, widths changed) | yes |
| RU3 | drag the 2.5 in tab right | `<:R1,12,…,1,2700,…>`; next paragraph `<:R>` | yes (snap) |
| RU4 | drag the 3 in tab off the ruler | the stop at 2880 removed | yes |
| RU5 | drag First 0.5 in | `<:I0,720,0,0>` (no ruler mark) | yes |
| RU6 | drag Rest 0.25 in | `<:I0,0,360,0>` | yes |
| RU7 | drag All 0.5 in | `<:I720,0,0,0>` | yes |
| RU8 | drag Right 0.5 in left | `<:I0,0,0,810>` (the absolute position truncated to 90) | yes |
| RU9 | Cols 2, drag column 1's right edge left | `1440 5490`, `6240 10800`: only that edge moved | yes |
| RU10 | then drag the left margin 0.25 in | left margin 1800; `1800 5490`, `6240 10800`: only the first edge moved | yes |
| RU11 | then Cols 3 | `1800 4640`, `4880 7720`, `7960 10800`: even again, gutter 240 (not the 750 gap) | yes |
| RU12 | RU9, then Cols 3 | `1440 4400`, `4640 7600`, `7840 10800` | yes |

The tool row appears only after a first click on the ruler (which does
not add a tab); drags work on the first press.

**Twins (RU.9, 2026-10-17).** The ignored test `ruler::tests::ruler_twins`
makes each gesture through the app's ruler model on the same document
(RUB, or RU1 without its ruler, or RU1 for RU2), writes it as an Ami Pro
document, and compares the body and the page layout records with Ami
Pro's file, caches left out: 12 of 12 the same. Ami Pro opened each twin
(`ARUn.SAM`) and its own file; the screens, at the start and two lines
down, are pixel-identical in the document area, the ruler included (22 of
22; only the title and the status bar's folder differ).

## Unequal columns in the corpora (2026-10-17)

Stored column edges compared with each other (widths and gaps alike
within a twip, or not): the user's corpus has 6 page layouts and 14 frame
interiors with unequal columns (some are table frames, whose interior
columns follow their table), 15 page layouts and 41 frame interiors with
even ones; two even page layouts start or end a few twips off the
margins. The VM folder has 3 (RU2, RU9, RU10). The model marks columns
unequal by the same test, so the even ones keep their exact stored edges
as offsets for the writer, as before.

## What the app does differently (approved, §19)

- No ruler mode and no tool row: the ruler is always live, with the tab
  type at its left end and right-click menus (R3).
- Every column of the box is shown, not only the caret's.
- In a table cell the ruler shows tabs and indents only; Ami Pro's ruler
  also drags the table's column widths there (found here; table cell
  formatting is not built).
- No vertical ruler (R4), so no column-0 changes.
