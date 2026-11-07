# Creating frames in Ami Pro 3.1: evidence (2026-10-17)

For the frame creation run (`docs/FRAME_CREATION_RUNBOOK.md`, slice FC.1).
Decompiler output stays outside Git (`../AmiPro-Reference/ghidra-work/out/`:
`amipro.c`, `amipro-cmds.c`, `amiproui.c`, `amiuicnt.c`); these are
findings. Evidence levels as in `SAM_FORMAT_NOTES.md`: VERIFIED
(decompile), VERIFIED (fixture), VERIFIED (manual) for the 3.1 user's
guide, chapter 15 ("Creating a frame").

## Where the code is

| Routine | Role |
|---|---|
| Menu command `0x19e` (`AMIPROUI.DLL` menu) | Frame › Create Frame…; help string 414 "Create a frame manually or specify the size and position for the frame." |
| Command `0x199` | "Create a frame manually." (help string 409): the frame SmartIcon, and the dialog's Manual button |
| Dialog 1035 "Create Frame" (DLL) | Width (25300), Height (25301), Down from top (25302), In from left (25303), the unit button (25650), OK, Cancel, Manual (25586) |
| `ADDFRAMEDLG` `1360:0000` (DLL) | The dialog procedure: fill `1360:01d7`, each box's limits `1360:0382`, OK `1360:0411` |
| `1260:0a4e` (EXE) | Command `0x19e`: after the dialog, OK posts `0x19d`, Manual posts `0x199` |
| `1110:0af9` (EXE) | Command `0x199`: frame drawing mode (Layout mode first) |
| `1260:0000`, `1260:00ef`, `1048:1fdb` (EXE) | Drawing: press, move (clamped by `1268:191f` and `1268:1971`), release |
| `1260:01cf` (EXE) | Command `0x19d`: the rectangle from the word queue becomes a frame |
| `1260:044a`, `1260:0827` (EXE) | A new frame record, and its settings from the default frame |
| `10a0:0523` (EXE) | The new frame joins the end of its list |
| `1018:0eeb` (EXE) | The default frame at start-up: AMIPRO.INI `FrDefs`, else built in |
| Command `0x19f`, `12b0:0465` (EXE) | Modify Frame Layout's Make Default |
| Callback codes `0x52` (`1128:0000`), `0x53`, `0x54` (`1038:0376`) | The dialog's starting values; the page's height and width |
| `1260:0509` (EXE) | A frame Ami Pro makes itself (Import Picture, Paste, Drawing, Charting, Equations with no frame selected) |

Found through `AMIPRO.EXE`'s menu-command table (`tools/ghidra/command_table.py`;
`DECOMPILATION.md`). Menu command IDs come from `AMIPROUI.DLL`'s menu
resources, now listed by `tools/ne_resources.py FILE menus`.

## The two ways (VERIFIED (manual), (decompile), (fixture))

Frame › Create Frame… opens the Create Frame dialog. **OK** makes a frame
of the dialog's width and height at its position, measured from the
paper's top and left edges. **Manual** closes the dialog and turns the
pointer into a frame: a drag on the page draws the frame. The frame
SmartIcon goes straight to drawing (FC10). Ami Pro also has a keyboard
way to draw (arrows move the frame pointer, Space with arrows sizes it;
manual only).

## The dialog (`1360:01d7`, `1360:0382`, `1360:0411`, `1128:0000`)

VERIFIED (decompile) unless marked.

- **Starting values** are the last frame made, however it was made
  (AMIPRO.INI key `frmcrt` in `[AmiPro]`: width, height, top, left in
  twips, written by every creation). With no entry: 2.00 × 2.00 in at
  4.50 in down and 3.25 in in (2880 × 2880 at 6480 and 4680, a frame
  centred on a letter page). A top below the page becomes the page height
  less 72 twips, a left past it the page width less 144. VERIFIED
  (fixture) FC1–FC3: the dialog showed F-series values, then FC1's, then
  FC2's drawn frame (1.72 × 1.49 at 3.52, 2.91), then FC3's.
- **The remembered size is at least 0.25 in** (360 twips) each way, even
  when the frame made was smaller (FC3: a 50 × 33-twip frame, then the
  dialog showed 0.25 × 0.25). VERIFIED (fixture).
- **Clamping on open:** a width that would run past the paper's right edge
  becomes page width − left; a height past the bottom, page height − top.
- **Limits while editing:** Width at most page width − In from left,
  Height at most page height − Down from top, Down from top at most page
  height − Height, In from left at most page width − Width, each limit
  recomputed when another box changes; Width and Height at least 1 twip,
  the positions at least 0. A value over its limit is refused in place:
  the box keeps the focus and OK does nothing (FC5, a 9.00 in width at
  1.25 in; no message). VERIFIED (fixture).
- **Units:** the unit button (inches, centimetres, picas, points) starts
  at the default frame's unit.
- **OK** sends the rectangle left, top, left + width, top + height.

## Drawing (`1260:0000`, `1260:00ef`, `1048:1fdb`)

VERIFIED (decompile) unless marked.

- A **press outside the paper** leaves drawing mode and acts as an
  ordinary click there.
- While dragging, a dotted outline follows the pointer; the moving corner
  stays on the paper (it can reach the paper's edges, margins included).
- A **release at the press point** (a click) makes nothing and leaves
  drawing mode (FC4, VERIFIED (fixture)). Any other release makes the
  frame from the two corners, in either direction (FC2 was dragged from
  bottom right to top left; its rectangle is 4184, 5076, 6656, 7218, to
  the twip of the drag). VERIFIED (fixture).
- **No minimum size**: a 3-pixel drag made a 50 × 33-twip frame (FC3).
  VERIFIED (fixture).
- **Esc** leaves drawing mode (FC11). VERIFIED (fixture).

## The new frame (`1260:01cf`, `1260:044a`, `1260:0827`)

VERIFIED (decompile) and (fixture) FC1, FC2, FC6N.

- **A text frame**, Where placed, on the **page in view** (Ami Pro's
  current page; FC9: with the caret on page 1 and page 2 scrolled into
  view, the dialog's frame went on page 2); a drawn frame is on the page
  it was drawn on. Its text starts empty (`[txt]` with no paragraph); text
  typed in it starts as Body Text (manual).
- **Settings from the default frame** (below); flags are the default's
  plus `0x200` (has text): `0x102C0` for the built-in default (FC1).
- **One column** from the margins, inset by the rounded corners as usual,
  and **no tabs**, whatever the default frame's columns and tabs were
  (FC6N).
- **Interior margins that do not fit are dropped:** the left and right
  margins become 0 when left + bottom margin ≥ the width (Ami Pro adds the
  bottom margin, not the right; they are equal by default); the top and
  bottom margins become 0 when top + bottom ≥ the frame's bottom edge
  measured from the paper's top (not its height), so in practice they are
  always kept. FC3's 50 × 33-twip frame kept 144 above and below and lost
  its left and right margins. VERIFIED (decompile), (fixture).
- **Stack:** the frame joins the **end** of the Where placed frames' list,
  above every other Where placed frame (FC2 drew over FC1). VERIFIED
  (fixture).
- **Selected** afterwards, with black handles; the caret stays where it
  was: Esc deselects the frame and the view returns to the caret (FC9).
  Enter, or a double-click, puts the caret in the frame (manual).
- **Remembered:** the frame's width, height, top, and left go to
  `frmcrt` (at least 360 each way for width and height).
- **One undo step** (an undo record of kind `0xd`).
- With a global option on (DGROUP 0x2a3a bit 0x40; probably revision
  marking, not checked) the new frame also gets flag `0x10`.

## The default frame (`1018:0eeb`, `12b0:0465`)

VERIFIED (decompile); the built-in values VERIFIED (fixture) FC1, F01.

At start-up Ami Pro reads AMIPRO.INI `FrDefs` (17 numbers); without it
the default frame is built in:

| # | Setting | Built in | SAM field |
|---|---|---|---|
| 1 | Line sides | 1 (All) | 7 |
| 2 | Line style | 1 | 10, first value |
| 3 | Line position | 1 (Middle) | 9 |
| 4 | Unit | 1 (inches) | 8; `[frmlay]` 3 and 6 |
| 5 | Line colour | 0 (black) | 11 |
| 6 | Background | 16777215 (white) | 12 |
| 7 | Flags | `0x100C0`: opaque, Wrap around, lines on | 2 |
| 8–11 | Margins: left, bottom, top, right | 144 each (0.10 in) | `[frmlay]` 4, 5, 7, 8 |
| 12–15 | Shadow extents: left, top, right, bottom | 0, 0, 100, 100 (Normal, falling right and down) | 10 |
| 16 | Shadow colour | 0 (black) | 10, last value |
| 17 | Rounded corners | 18 % | 10, sixth value |

This matches the manual ("a thin line with a .10 inch margin and a shadow
around the frame") and every default fixture since F01.

**Make Default** (Modify Frame Layout's button, command `0x19f`) stores
the selected frame's 17 values as the default and writes them to
`FrDefs`, so the default outlives the session and every document. Of the
flags it keeps only Opaque (`0x40`), Wrap around (`0x80`), lines on
(`0x10000`), and No wrap beside (`0x20000`): **placement, size, columns,
and tabs are never part of the default** ("Ami Pro uses Where placed as
the Placement default whenever you create a frame", manual). VERIFIED
(fixture) FC6/FC6N: after Make Default on a With para above, No wrap
around, Transparent, square-cornered frame with only a top line of style
3, no shadow, 0.20 in margins, and four columns, the next frame was Where
placed, No wrap around, Transparent, square, top line style 3, no shadow,
0.20 in margins, one column, no tabs. (The VM's default was put back with
Make Default on a frame with the built-in settings.)

## Stacking between kinds (FC7G, FC8)

VERIFIED (fixture), with the writer VERIFIED (decompile) `11c8:3ba0`.

- Ami Pro keeps page frames in three lists and writes them in a fixed
  order, the Where placed frames first and the anchored ones after, so a
  frame made after an anchored one is still written before it (FC6N,
  FC7).
- **Where placed frames are drawn above anchored frames**, whatever the
  order they were made in: FC7G (an anchored frame, then a Where placed
  one over it: the Where placed one on top) and FC8 (a Where placed frame,
  then an anchored one over it: still the Where placed one on top).
  The order of the file is therefore z-order only within a kind (Z01/Z02
  were Where placed frames).
- The app kept one stack for both kinds in file order and drew it back to
  front, so imported anchored frames were drawn over Where placed ones
  (the opposite of FC8). Fixed in this run (`UI_SPEC.md` §20).
- Seen on the way, not part of this run: a With para above frame is made
  narrower to fit beside a Wrap around frame (FC7, FC7E: its rectangle's
  right edge stopped 100 twips left of the other frame while its stored
  width stayed 2880), and widened again when the other frame is No wrap
  around (FC7G).

## Frames Ami Pro makes itself (`1260:0509`)

VERIFIED (decompile); not built in this run. Import Picture (`0x8d`),
Paste and Paste Special of a picture or object, and Drawing, Charting,
and Equations with no frame selected make a frame at the caret: the
remembered width and height (`frmcrt`; 1.00 × 1.00 in with no entry),
narrowed to the widest column and shortened to the page's text height
(less the default shadow), **Flow with text** (an anchor `<:A…>` at the
caret, Wrap around), and, by the caller, without lines, margins, shadow,
or rounded corners. Create Frame itself always makes an empty text frame;
a picture goes into a selected empty frame with Import Picture.

## Fixtures (`../AmiPro-Reference/vm-lab/OUT`, 2026-10-17)

| Fixture | What | Shows |
|---|---|---|
| FC1 | Create Frame: 2.50 × 1.00 in at 3.00 in down, 1.25 in in | The built-in default frame, record by record; the dialog remembers it |
| FC2 | FC1 plus a frame drawn from bottom right to top left | Drawing to the twip; the new frame last in the file and drawn on top; the dialog remembers it |
| FC3 | FC2 plus a 3-pixel drag | No minimum; margins that do not fit; the remembered 0.25 in |
| FC4 | FC3 plus a click in drawing mode | Nothing made |
| FC5 | (screens only) a 9.00 in width at 1.25 in | Refused in place |
| FC6, FC6N | Make Default on a changed frame, then a new frame | What the default keeps and what it never keeps |
| FC7, FC7B, FC7C, FC7E, FC7G | An anchored frame, then a Where placed frame over it | File order; Where placed drawn above anchored |
| FC8 | A Where placed frame, then an anchored frame over it | Where placed still above |
| FC9 | Two pages, caret on page 1, page 2 in view | The dialog uses the page in view; Esc returns to the caret |
| FC10, FC11 | (screens only) the frame SmartIcon; Esc in drawing mode | The SmartIcon draws; Esc cancels |

Screenshots `fc*` in `../AmiPro-Reference/vm-lab/screens/`.

## Written as Ami Pro writes it (FC.7)

- A new page frame's `[frmlay]` values 10 to 14 are `2 0 0 0 0` (FC1,
  FC9), not a header's `1 0 1 1 0`; values 12 to 14 are stale memory in
  later frames of a session (FC2 wrote `114 100 120`), so twins compare
  without them.
- A new frame's text is a `[txt]` without paragraphs until typed in.
- Its column edges follow Ami Pro's integer corner inset and line width
  (`SAM_FORMAT_NOTES.md`, "Derived columns"): FC3's 10 twips are its
  line's width.
- Paragraph caches (`<:#…>`) appear once Ami Pro has laid a paragraph out:
  FCB (FC1 opened and saved again) has them, FC1 not.

## Open

- Ami Pro's keyboard way of drawing (manual only; not traced).
