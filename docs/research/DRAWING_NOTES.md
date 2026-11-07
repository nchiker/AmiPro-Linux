# Drawings in Ami Pro 3.1: notes for the drawing run

**Written:** 2026-10-25, for the drawing run (`docs/playbooks/GRAPHICS.md`,
run 2; queue run 9). Evidence levels as in `SAM_FORMAT_NOTES.md`:
VERIFIED (decompile), VERIFIED (fixture), CORPUS (both corpora, counted),
DOCUMENTED (user's guide ch. 17), INFERRED. Decompiler output stays outside
Git (`../AmiPro-Reference/ghidra-work/out/drawman.c`, `amidraw.c`); routines
are named by address (selector:offset in the module named) and described in
my own words. Nothing of Ami Pro's code, icons, or symbol art is copied.

## The modules

- **`AmiDraw.dll`** is the picture driver for the `.sdw` type (`[AmiVISD]`
  in `AMIPRO.INI`). It exports the driver entry points every picture driver
  has (`LIBCREATE`, `LIBOPEN`, `LIBMESSAGE`, `LIBCLOSE`, `LIBSIZE`,
  `LIBPAINT`, `LIBDUPLICATE`, …) and owns Draw mode: the icon bar, the Draw
  menu, the dialogs (`FRAMEPROC` line style, `FILLPROC` fill pattern,
  `ROTATEPROC`, `GRIDPROC`, `OPENPROC`, `SAVEPROC`), and the gestures.
- **`DrawMan.dll`** (Samna, 1990) is the object store: a window of class
  `DrawMan` that `AmiDraw` drives with private messages 0x400–0x551
  (create, select, move, resize, flip, rotate, undo, read, write, paint).
  Its type names: LINE, PERPLINE, POLYLINE, POLYGON, RECT, SQUARE, RNDRECT,
  RNDSQUARE, OVAL, CIRCLE, TEXT, GROUP, CHART, METAFILE, METAFILEIMG.
- **`AMIPRO.EXE`** knows a drawing only as a picture: Tools › Drawing
  (`1168:1a87`), entering and leaving Draw mode (`1168:4495`, `1168:0eb1`),
  routing the document window's messages to `LIBMESSAGE` while a frame is
  being edited (`1168:0de0`).

## The stream (VERIFIED (decompile), VERIFIED (fixture), CORPUS)

A `.sdw` file and the source bytes of a drawing in a SAM's `[Embedded]`
section are the same byte stream; charts (driver `.sct`) store the same
stream. It is a near memory image of DrawMan's object lists. Reader
`DrawMan 1008:7eec` (blocks), `868e` (object headers), `8080` (per type);
writer `7e9a`, `864a`, `870c`.

**Block header** (22 bytes): `SM`, version (0x0102 in every corpus
drawing; older ones are converted on reading, `8080`), two runtime handles,
the **object count** (the only field the reader uses), a selection handle
(0 in files), the bounds (union of the objects' rectangles), the bytes
used. A writer may leave the handles, bounds, and bytes used 0: Ami Pro
reads such a stream and writes it back unchanged (DRW0, DRW0S).

**Object header** (16 bytes): type, flags (0; 4 on objects belonging to a
chart; 0x80 and 0x40 are runtime selection states), the record's length
including the header, its **bounding rectangle** (not recomputed on
reading: it drives culling and the natural size, so a writer must give it),
two runtime links (0). Coordinates are signed 16-bit **twips, y down**,
from the drawing's origin. **Stream order is painting order**, back to
front. A GROUP record is followed at once by its own block (nesting up to 4
in the corpus). No trailer, no checksum; the `[Embedded]` length is the
stream's length (five shipped symbol files carry stale bytes after it).

**Shapes are stored as points**, so rotation and flips are in the points;
only text keeps an angle.

| Type | Kind | Body (after the header) |
|---|---|---|
| 2, 3 | Line, line made with Shift (kept at 0, 45, 90°) | two points; pen width, endings, style, colour |
| 4 | Polyline | pen (width, endings, style, colour); a count; the points |
| 5 | Polygon | pen (width, style, colour); fill; a count; the points (closed implicitly; about half repeat the first point) |
| 6, 7 | Rectangle, square | the unturned rectangle (legacy); pen; fill; four corners |
| 8, 9 | Rounded rectangle, rounded square | the unturned rectangle; pen; fill; the corner radii (legacy); **16 points: four cubic Béziers**, one per corner, joined by straight sides; each corner's control points sit 3/20 of the radius from the corner |
| 10, 11 | Ellipse, circle | the unturned rectangle; pen; fill; **13 points: four cubic Béziers** sharing ends, handles 11/40 of the size (about the usual circle constant) |
| 12 | Arc | legacy rectangle and ends; pen and endings; **one cubic Bézier** (4 points); never filled |
| 13 | Text | the origin (the text cell's top-left), the box's unturned width and height, a 32-byte face, pitch and family, the height in twips (negative: character height; −240 is 12 pt), colour, two attribute bytes, the angle in tenths of a degree **clockwise as seen**, a stale spacing word, the text in CP-1252 with a NUL and one more byte |
| 14 | Group | a runtime handle; its block follows |
| 15 | Chart data | AmiChart's numbers and labels; never painted (the chart's look is the ordinary objects around it, flag 4); kept byte for byte |
| 16, 17 | Metafile, metafile image | not in any file; 17 is deleted on reading |
| 18 | Bitmap | a packed DIB stretched onto the rectangle (12 in the corpus) |

**Pen**: width in twips (a byte; 0 is no line), a GDI pen style (0 solid,
1 dash, 2 dot, 3 dash-dot, 4 dash-dot-dot, 5 none, 6 inside frame), a
`COLORREF`, and the endings (low nibble the start, high nibble the end).
**Fill**: foreground and background `COLORREF`s, a pattern number, and 8
bytes of a custom pattern (unused).

**Corpus** (CORPUS; 382 drawings in SAM files, 105 shipped symbol files,
all parsed): polygons 7,548, rectangles 4,669, ellipses 2,119, polylines
1,922, groups 1,496, lines 970, text 745, chart data 107, arcs 33, bitmaps
12, a few squares, circles, rounded rectangles; pen styles 0, 2, 5, 6;
fills: solid 14,298 of 14,351; text attributes 0, 1, 3 only; text angles 0
(622) and a few others. 238 of the 380 drawings in the user's corpus have
no rendering, so the app shows them as labelled boxes today. One damaged
user file's `[Embedded]` offsets are 2,048 bytes too high.

## Painting (VERIFIED (decompile) and the fixtures DRW1–DRW3)

- **Natural size** (`LIBSIZE`): each object's rectangle inflated by half
  its pen width (lines, polylines, and arcs with endings also by the
  ending's size); the union's **right and bottom measured from (0, 0)**; 3 ×
  3 in while empty. DR1: a 1,648 × 741 rectangle at (428, 165) with a 1 pt
  line gives `0 0 2086 −916`. CORPUS: 100 of 102 Original-size drawings.
- **A drawing is 1:1** (DR1): a new drawing is Original size, its twips are
  the page's; the drawing's (0, 0)–(R, B) is stretched onto the picture's
  rectangle (Ami Pro's scaling modes, as for any picture), plus the pan.
- **Fills** (DRW1): 0 hollow (what is behind shows through); 1–4 dot
  patterns of the foreground on the background (12.5, 25, 50, 75 %: rows
  `77 FF DD FF`, `EE BB`, `55 AA`, `11 44` with 0 bits the foreground); 5
  solid foreground; 6–11 the six Windows hatches (horizontal, vertical, `\`,
  `/`, cross, diagonal cross) in the foreground on the background, opaque,
  eight screen pixels apart. Brush origin the device's.
- **Pens** (DRW2): a styled pen (any style but solid, inside frame too) is
  one device pixel wide whatever its width; a width of 0 draws nothing.
  Solid lines have round ends. Shapes are filled first, then outlined.
  Polygons fill alternate (even-odd).
- **Endings** (DRW2), sized by n = width / 60 + 1: length 120 n, half-width
  60 n twips: 1 filled arrowhead, 2 notched arrowhead, 3 open arrowhead
  (two strokes), 4–6 the same three pointing back into the line (a tail),
  7 a bar across, 8 a filled square, 9 a filled circle.
- **Curves**: Ami Pro flattens each cubic into 16 pieces on screen and 32
  in print; the true curve is within a pixel.
- **Text** (DRW3): drawn transparent, in the record's font, **spread to
  fill its box** when unturned (the extra space shared between the
  characters), turned clockwise by the angle; attributes bold 1, italic 2,
  underline 4 or 8, strikeout 128. Bits 0x20 and the second byte's script
  bits (never in a file) draw at a wrong size.
- Graphics Scaling's Rotate turns the objects themselves through the
  driver's state (byte 0x8c, the angle already applied; 0 in every file).

## Writing (VERIFIED (fixture))

- **The driver's state may be left out** (count 0) and the rendering too:
  Ami Pro opens such a drawing, draws it, and writes back the stream
  unchanged with a fresh 384-word state and its natural size in `[isd]`
  (DRW0 → DRW0S). The state holds runtime pointers, the last picture
  rectangle and pan, the defaults for new objects, the grid (as
  `AmiDraw.ini`), and the applied rotation; nothing is needed to draw.
- A stream needs version 0x0102, the right count, records of exact length
  with correct bounding rectangles, and links 0.

## Draw mode (VERIFIED (decompile), DOCUMENTED; DR1, screenshots)

- **Entering**: Tools › Drawing (enabled with no frame selected, an empty
  frame, or a drawing frame; not a text frame) makes Ami Pro's own frame at
  the caret when nothing is selected (`1260:0509`: the remembered Create
  Frame size, Flow with text, the default frame's lines, shadow, and
  corners; DR1) or uses the selected one; a double-click on a drawing frame
  opens it. Ami Pro adds a **Draw menu** after Tools, Import Drawing… and
  Save As Drawing… to File, and an icon bar across the document window, and
  takes over Undo, Cut, Copy, Paste.
- **The icon bar**: ten tools (Selection, Hand, Line, Polyline, Polygon,
  Rectangle, Rounded rectangle, Ellipse, Arc, Text) and thirteen commands
  (Select All, Group/Ungroup, Bring to Front, Send to Back, Rotate by the
  step, Flip Horizontal, Flip Vertical, Show Grid, Snap To, Extract Line &
  Fill, Apply Line & Fill, Line Style with a sample, Fill Pattern with a
  swatch).
- **The Draw menu**: Line Style…, Fill Pattern…, Extract Line & Fill,
  Apply Line & Fill; Rotate…, Flip ▸; Snap To, Show Grid (Hide Grid), Grid
  Settings…; Select All (checked when all are), Group (checked on a group:
  ungroups), Bring To Front, Send To Back. No shortcuts.
- **Gestures**: start inside the frame only, clipped to it; snap to the
  grid from the drawing's origin when Snap To is on; a drag of 40 twips or
  less makes nothing; a right press cancels; the tool stays after making an
  object. Shift: lines at 0, 45, 90°; squares, rounded squares, circles
  (kept when resized: corner handles only). Polyline and polygon: click per
  vertex, double-click ends (a polygon also ends near its first vertex);
  Shift and drag draws freehand (a vertex every 40 twips). Arc: one end to
  the other, a quarter ellipse.
- **Selection**: click, Shift+click toggles, Ctrl+click the next object
  below, a marquee takes objects even partly inside; handles 8 px (two for
  lines, four for constrained shapes, eight otherwise); a drag moves (Shift
  at the release copies); a handle resizes from the opposite one, past it
  flips; a double-click on a handle flips; a double-click on the body enters
  point editing: rotation arrows at the corners (free rotation), a movable
  centre of rotation, the shape's points (a segment drag adds a vertex, a
  double-click on a vertex deletes it; an arc's points bend it).
- **Hand** pans the whole picture in its frame.
- **Text tool**: a click starts a text object (or edits one); Backspace,
  Delete, arrows; Enter starts another below; the font is the Text menu's
  (Times New Roman 12 pt by default); sizing a text object changes its size.
- **Dialogs**: Line Style (15 entries: dotted, ½ pt, 1–12 pt, Transparent;
  a colour; ten start and ten end endings); Fill Pattern (a colour; the six
  hatches, Solid, Transparent; a colour chosen with Transparent becomes
  Solid); Rotate (0–359°, clockwise or counter-clockwise, 10° clockwise by
  default; the icon repeats it); Grid Settings (Fine, Medium, Coarse at ⅛,
  ¼, ½ in, each editable; dots or dotted lines; Make default).
- **Defaults** (DR1): a 1 pt solid black line, no endings, no fill (black
  on white when a pattern is chosen); grid hidden, snap off, Medium.
- **Undo**: one level, toggling (Undo again redoes).
- **Leaving**: Esc or a click outside. DR1's frame (made by Ami Pro) kept
  its size.
- **Import Drawing… / Save As Drawing…**: AmiDraw, Windows metafile, or
  Windows bitmap; Save has Selected objects only. Paste takes AmiDraw
  objects, a metafile, a bitmap, or text (a text object per paragraph).

## Fixtures (`../AmiPro-Reference/vm-lab/OUT`, `IN`; 2026-10-25)

| Fixture | What | Shows |
|---|---|---|
| DR1 | Tools › Drawing with nothing selected, one rectangle | Ami Pro's own frame (3 × 1 in, default style), 1:1 twips, Original size, natural size `0 0 2086 −916`, 1 pt black, no fill |
| DRBIG | A 6 × 7 in frame, Tools › Drawing, one rectangle | The template for the synthetic fixtures |
| DRW0 (`IN`) and DRW0S | One rectangle, links, bounds, bytes used 0, no driver state, no rendering; Ami Pro's save | Drawn; written back unchanged with a fresh state |
| DRW1 (`IN`) | Fill patterns 0–11, red on yellow, over a blue band | 0 hollow, 1–4 dots, 5 solid, 6–11 hatches (8 `\`, 9 `/`) |
| DRW2 (`IN`) | Endings 1–9 at widths 0 and 120; pen styles 1–4, 6 at 60 | Width 0 draws nothing; endings as above; styled pens one pixel |
| DRW3 (`IN`) | Text: plain, 30°, a wide box, each attribute bit | Spread to the box; clockwise; bold, italic, underline 4 and 8, strikeout 128 |
| DR2, DR2B | A rounded rectangle, an ellipse, an arc drawn in Ami Pro's Draw mode | Corner radius 30 screen pixels, at most a third of the side (494 × 384 twips at 91 % for 1,648 × 1,154); ellipse handles 9/40; the arc's control points at 11/20 of the height below the start and 9/20 of the width from it, level with the end |
| ADR1–ADR3 (`IN`) | The app's twins: every tool and look; DR2B edited; an SVG and a picture imported | Ami Pro shows each as the app does |
| ADR3S | Ami Pro's Draw mode on ADR3, then Save As | The app's stream written back byte for byte but for runtime handles |

## Open

- Graphics Scaling's Rotate on a drawing (the objects turned in the saved
  stream?): no file has it; the app keeps turning drawings as pictures.
- Paste of other programs' formats into a drawing (Ami Pro: metafile,
  bitmap, text).
