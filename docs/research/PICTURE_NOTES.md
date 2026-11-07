# Pictures in Ami Pro 3.1: notes for the pictures run

**Written:** 2026-10-18, for the pictures run (`docs/playbooks/GRAPHICS.md`,
run 1). Evidence levels as in `SAM_FORMAT_NOTES.md`: VERIFIED (decompile),
VERIFIED (fixture), CORPUS (both corpora, counted), DOCUMENTED (user's
guide). Decompiler output stays outside Git; the routines are named by
address and described in my own words.

## Where it is

| Routine | What it does |
|---|---|
| Command `0x8d`, `10d8:126c` (EXE) | File › Import Picture: takes the dialog's answers, then puts the picture into the selected frame (`10d8:19d9`) or into a frame made at the caret (`1260:0509`) |
| `1260:0509` (EXE) | The frame Ami Pro makes itself (`FRAME_CREATION_NOTES.md`): the remembered Create Frame size (`frmcrt` in `AMIPRO.INI`), narrowed to the widest column and the page's text height, Flow with text at the caret |
| `IMPORTPICTUREPROC`, `12d0:0000` (`AMIPROUI.DLL`, dialog 2100) | The Import Picture dialog: File name, File type, Files, Directories, Drives, Copy image |
| Command `0x193`, `1168:0000` (EXE) | Frame › Graphics Scaling: stores the dialog's answers in the picture (below) |
| `FRAMESCALEPROC`, `11b0:0000`–`0b48` (`AMIPROUI.DLL`, dialog 1020) | The Graphics Scaling dialog: its starting values, limits, and the aspect-ratio arithmetic |
| `1168:2dd2` (EXE) | The picture's rectangle from its scaling mode, recomputed **every time the picture is drawn** |
| `1168:4083`, `1168:3fb4` (EXE) | Draw a picture and ask its natural size, through its driver |
| `1168:4f35` (EXE) | Finds a picture's driver by extension in a table filled from `AMIPRO.INI`'s `[AmiVISD]` |
| `1168:2016`, `1168:26e7` (EXE) | Write and read Ami Pro's rendering, a screen-bitmap cache (below) |
| `11c8:2e29` (EXE) | Writes a picture frame's `[isd]` record |
| `1530:0892` mode 2 (EXE) | The picture area of a frame (its text area, with a further clamp) |
| `LIBCREATE`, `LIBACTIVATE`, `LIBSIZE`, `LIBPAINT` (`AMIPAINT.DLL`, `AMIWMF.DLL`) | The bitmap and metafile drivers: their saved state, natural sizes, and drawing |

## Drivers and filters (VERIFIED (fixture), DOCUMENTED)

Ami Pro draws every picture through a driver DLL chosen by the picture's
type, listed in `AMIPRO.INI` under `[AmiVISD]`:

| Type | Driver | Kind |
|---|---|---|
| `.bmp` | `AmiPaint.dll` | Windows bitmap |
| `.wmf` | `AmiWMF.dll` | Windows metafile |
| `.sdw` | `AmiDraw.dll` | Ami Pro drawing (the drawing run) |
| `.sct` | `AmiChart.dll` | chart, stored as `.sdw` (the charts run) |
| `.tex` | `AmiEqn.dll` | equation (the equations run) |
| `.ole` | `AmiOLE.dll` | OLE object (the objects run) |
| `.tgf` | `AmiImage.dll` | grey-scale image for Image Processing |

Import filters (`application1`… in `AMIPRO.INI`) **convert at import**:
PCX and TIFF become Windows bitmaps (VM PI3, PI4: stored as `.bmp`), CGM
and HPGL become metafiles, and DrawPerfect, Freelance, and 1-2-3 PIC become
AmiDraw drawings (guide ch. 15). EPS is kept with its preview. So a
document stores only the driver types above.

## The `[isd]` record (`11c8:2e29`; VERIFIED (decompile), (fixture))

One value per line, in this order:

| Values | Meaning |
|---|---|
| 1 | `.X` and the frame's sequence number (field 13): the name of the rendering |
| 2 | The picture's type (`.bmp`, `.wmf`, `.sdw`, `.ole`; `.eps` for EPS) |
| 3 | `.X` and the sequence number when the picture is stored in the document, the source path when it is linked, or empty (a pasted or drawn picture) |
| 4 | Scaling flags: 1 Original size, 2 Fit in frame, 4 Percentage, 8 Custom, plus 0x10 Maintain aspect ratio (`FRAMESCALEPROC`) |
| 5 | The dialog's unit (1 inches; 3 seen) |
| 6–9 | The picture's rectangle: left, top, right, bottom, in twips with y growing upward, from the picture area's top-left corner: `0 0 W −H` after Graphics Scaling |
| 10 | The percentage (100 unless Percentage) |
| 11 | Rotation in degrees (0 in all 1,001 corpus pictures) |
| 12, … | A count of words and then the driver's own state, saved word by word (BMP 48, WMF 80, drawings 368–384, charts 688–704, OLE 16) |
| next | The driver's type again (`.bmp`, `.wmf`, `.sdw`, `.sct`, `.ole`, `.sdp`) |
| next two | The **pan**: how far the picture has been moved in its frame, x and y (y negative is down), in twips |
| next | A count of words of a second state block (0 in every fixture) |

The driver's state is a memory image that includes far pointers to Ami
Pro's callbacks, so most of it is meaningless in a file. What matters:

- **Bitmaps** (`AmiPaint`): words 16 and 17 are the natural width and
  −height in twips, which Ami Pro recomputes from the bitmap.
- **Metafiles** (`AmiWMF`): words 2 and 3 are the extent in twips, 16 and
  17 the pan, 18 and 19 the units per inch (1440), 20 the mapping mode
  (8, anisotropic). For a metafile **without a placeable header** (68 of
  the 72 distinct corpus metafiles, all pasted from the clipboard) this is
  the only record of its size: a plain metafile imported from a file shows
  nothing in Ami Pro (VM T1).

**A writer may leave the state out.** PIR1 (a bitmap) and PIR3 (a
placeable metafile) were written with a count of 0 and no rendering; Ami
Pro opened both, drew them, and wrote back exactly the state it writes
for its own import (PIR1S, PIR3S). VERIFIED (fixture).

## Sizes and drawing (VERIFIED (decompile), (fixture))

**Natural size.**
- A bitmap's is its pixels at its stored resolution (width × 56,693 /
  pixels per metre, as twips), halved together while either side passes
  32,767; with no resolution, at the screen's (96 dpi), keeping each side
  under 21 inches by lowering the resolution (`AmiPaint`). PI1: 120 × 80
  pixels with no resolution, 1,800 × 1,200 twips.
- A placeable metafile's is its bounding box at its units per inch; a
  plain one's is the extent in the driver's state.

**The picture's rectangle is recomputed every time it is drawn**
(`1168:2dd2`), from the flags: Original size, the natural size; Percentage,
the natural size × percent / 100 (at least 12 twips); Fit in frame, the
picture area, or with Maintain aspect ratio the natural size scaled by
the smaller of the two ratios; Custom, the stored rectangle. A size past
31,680 twips (22 in) resets that axis. So only Custom pictures use values
6–9 as stored; Fit in frame without aspect stores a stale rectangle in
view coordinates (85 corpus pictures), which no reader should use.

**Where it is drawn.** From the picture area's top-left corner plus the
pan, stretched to the rectangle (`StretchDIBits`, COLORONCOLOR; a metafile
played into it: its window mapped onto the rectangle), clipped to the
picture area. The frame's background, lines, shadow, and corners are drawn
as for any frame (PI1). The picture area is the frame's text area
(`1530:0892` mode 2) within a few twips: PI1's measured 1,108 × 544 inside
a 1,440 × 864 frame with 166-twip text insets, P02's 1,763 tall inside
2,160 with 201; the app uses its text area (`FrameStyle::text_insets`).

**Pan (cropping).** A double-click on a picture frame (or Enter on a
selected one) puts it in picture mode (grey handles, a hand); a drag moves
the picture; arrows move it 72 twips (0.05 in) and Ctrl+arrows 720 twips;
Esc twice or a click outside returns (guide ch. 19; PI8: three Right, one
Ctrl+Right, two Down stored as `936 −144`). Graphics Scaling keeps the pan.
Shift and a corner handle size the frame and the picture together and make
it Custom (guide ch. 19).

## Rotation (PIRT30, PIRT90; VERIFIED (fixture), 2026-10-18)

Graphics Scaling's Rotate (value 11, degrees) turns the picture
**clockwise as seen**, and the turned picture's bounding box is then
**stretched onto the picture's rectangle** as its scaling mode computes it
from the unturned picture (the stored rectangle stays 816 × 544 in both
fixtures). So a 90° turn of PI1's 1,800 × 1,200 bitmap shows its quadrants
turned (red to the top right) but squeezed into 816 × 544, and 30° looks
sheared. What lies outside the turned picture within its box shows the
frame's background (white in both fixtures). The dialog's Rotate box takes
0–359 and is enabled for bitmaps and drawings (`FRAMESCALEPROC`'s driver
query), not metafiles or equations.

## The Graphics Scaling dialog (`FRAMESCALEPROC`; PI5–PI7)

Original size, Fit in frame, Percentage [n], Custom [W] × [H] with a unit
button, Maintain aspect ratio, Rotate [n] degrees (0–359, enabled for
bitmaps and drawings, not for metafiles or equations; guide ch. 19), OK,
Cancel. The boxes show the size the chosen option gives; typing in W or H
selects Custom; with aspect kept, one box sets the other. Sizes are 1 to
31,680 twips. After import the picture is Fit in frame with aspect kept
(flags 18; PI1–PI4), the guide's "sized to fit in the frame and the aspect
ratio is maintained".

## Import Picture (`10d8:126c`; PI1–PI4, PI9)

- The dialog lists AmiDraw, AmiEquation, AutoCAD, CGM, DrawPerfect, EPS,
  Freelance, HPGL, Lotus PIC, PCX, TIFF, Windows Bitmap, Windows Metafile;
  it remembers the type and folder; **Copy image** is on.
- With an empty frame selected, the picture goes into it (P02). With the
  caret in an empty table cell, into the cell (`14d0:05ec`,
  `CELL_FRAME_NOTES.md`). With the caret in text, into a new frame at the
  caret (`1260:0509`): **the remembered Create Frame size** (VM:
  1,440 × 864 from `frmcrt=1440 864 …`; 1.00 × 1.00 in when unset), not
  the picture's, Flow with text, with the default frame's lines, shadow,
  corners, and margins (PI1's flags `0x200900C0`).
- **Import Picture is disabled** with a frame holding a picture selected
  (PI9) and in a cell holding text (TF12): there is no replace; the
  picture must be deleted first (double-click, Delete).

## Ami Pro's rendering (`1168:2016`, `1168:26e7`; VERIFIED (decompile), CORPUS)

The second object in `[Embedded]` is a cache of the picture area as last
drawn on screen: an 18-byte header (`SS`, width and height in pixels,
bytes per row, bits per pixel, planes, the screen's dots per inch twice,
the view's window extent) and the raw device bitmap (`GetBitmapBits`).
Ami Pro uses it only when the screen's resolution, bitmap format, and view
extent match, and redraws from the source otherwise: **a cache, never
needed** (488 of 937 user-corpus pictures have none). Formats seen: 4
planes of 1 bit (16 colours: each row's planes in turn, plane 0 red, 1
green, 2 blue, 3 intensity; calibrated against 22 bitmaps), 16 bits
(RGB 5-6-5; calibrated against 62 bitmaps), 8 bits (indices into an
unknown screen palette), 32 bits (the VM). It shows the picture at the
zoom it was last drawn (about 91 % in the VM), cropped to the picture
area.

## In the corpora (CORPUS, 2026-10-18; aggregate counts only)

| Kind | User's corpus (937 in `[Embedded]`) | ISO (36) |
|---|---|---|
| Bitmaps stored | 281 (99 with a rendering) | 18 |
| Metafiles stored | 230 (151 with a rendering) | 13 |
| Drawings stored | 380 (142 with a rendering) | 5 |
| OLE objects stored | 34 (13 with a rendering) | 0 |
| Linked (bitmap 2, metafile 4, TIFF 2, CGM 1, unknown 3) | 12 (7 with a rendering) | 0 |

Scaling flags over both corpora: Fit in frame with aspect 411, Original
size 274, Original with aspect 116, Fit in frame 94, Custom with aspect 61,
Percentage with aspect 39, Percentage 6. 123 pictures are panned.

## WMF libraries (2026-10-18; spike outside Git)

The 72 distinct corpus metafiles use about 30 record types: 60 are
bitmaps wrapped in a metafile (`StretchDIBits`, `DibStretchBlt`, with
palette records), 12 are drawings (polygons, polylines, `ExtTextOut`,
`PatBlt`, arcs, pattern brushes). Two crates were tried on all 72:
`wmf-core` 0.1.2 (MIT; WMF to SVG, drawn with resvg) drew 69, failing 3 on
a font field it rejects, and its bitmaps need a BMP decoder supplied to
resvg; one bitmap metafile drew stray patterns. `emfsdk` 0.2.0 (MIT or
Apache-2.0; its own rasterizer) drew 63, rejecting 9 on a `SetRelAbs`
record. Both are a few months old. An own interpreter of these records
(MS-WMF is an open specification) drawing onto tiny-skia, already in the
app's tree through resvg, covers all 72 and keeps text, raster
operations, and palettes in the app's hands.

## Fixtures (`../AmiPro-Reference/vm-lab/OUT`, `IN`; 2026-10-18)

| Fixture | What | Shows |
|---|---|---|
| PIC1–PIC5 (`IN`) | Synthetic pictures: 120 × 80 BMP without resolution, the same at 72 dpi, a 2 × 1 in placeable WMF, a 100 × 60 PCX, a 100 × 60 TIFF | — |
| PI1 | Import Picture of PIC1 with the caret in text | A Flow with text frame at the caret, the remembered size, default frame style; flags 18; natural size 1,800 × 1,200 |
| PI2 | The same with PIC3 | Metafile state: extent 2,880 × 1,440, 1,440 per inch, mode 8 |
| PI3, PI4 | The same with PIC4 (PCX), PIC5 (TIFF) | Both stored as `.bmp` |
| PI5, PI6, PI6B, PI7 | PI1 at Original size, Percentage 100, (Custom from the boxes), Custom 1.00 × 0.25 in without aspect | Flags 17, 20, 24, 8; drawn from the top-left, stretched, clipped |
| PI8 | PI7 panned | Pan `936 −144` |
| PI9 | A picture frame selected | Import Picture disabled |
| PIR1, PIR3 (`IN`) and PIR1S, PIR3S | PI1 and PI2 with no driver state and no rendering, and Ami Pro's saves | Drawn; the state rebuilt as Ami Pro's own |
| PIR2, PIR4, PIR5 (`IN`) | Zeroed state; a 72 dpi bitmap behind PI1's stale state | Drawn |
| API1, API2, API5, API7, API8, APIRT, APNG, AJPG, APCX, ATIF, ACELL, AFRM (`IN`) | The app's twins (`picture_twins`) | Ami Pro opens each with the picture where the app put it; its re-saves API8S, ACELLS (`OUT`) keep scaling and pan and fill in the driver's state |
| PIRT30, PIRT90 | PI1 rotated 30° and 90° (Graphics Scaling) | Clockwise as seen; the turned picture's box stretched onto the unchanged 816 × 544 rectangle; value 11 the degrees |
| T1 (`IN`) | A plain corpus metafile imported from a file | Draws nothing |

## Open

- The picture area's exact clamp (`1530:0892` mode 2) differs from the
  text area by a few twips (PI1, P02); the app uses the frame's interior.
- What Import Picture does with the caret in a header, footer, or frame's
  text (the app refuses: a Flow with text frame anchors in the main text).
- Eight-bit renderings need the screen palette of the machine that wrote
  them (17 drawings and OLE objects in the user's corpus).
- The `.G0x` pictures (14 linked metafiles from an earlier version) and
  the three linked pictures of unknown type.
