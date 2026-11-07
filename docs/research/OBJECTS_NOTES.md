# Objects in Ami Pro 3.1: evidence for run D (2026-11-06)

For `docs/OBJECTS_RUNBOOK.md` (DO.1 onward). Prepared before the run by a
read-only study: `AMIPRO.INI` from the VM lab, the three modules'
resources (`tools/ne_resources.py` on `amichart.dll`, `amieqn.dll`,
`amiimage.dll`, `amiproui.dll`, `amipro.exe`), their exported entry names,
the user's guide chapters 18, 19, 27, and 30 (the ISO's online documents,
read as text), the existing decompile notes (`DRAWING_NOTES.md`,
`PICTURE_NOTES.md`, `SAM_FORMAT_NOTES.md`), the decompiled `AMIPRO.EXE` for
the `[chint]`/`[ehint]` question, and aggregate scans of the three corpora
(scripts outside Git; no file names or contents recorded). **The new
modules' code (`amichart`, `amieqn`, `amiimage`) is not decompiled yet;
that is DO.1.** Nothing was run in the VM. Levels: **D** decompile, **C**
corpus (aggregate counts only), **H** the user's guide, **R** resources and
`AMIPRO.INI`, **VM** an earlier fixture, **?** unknown.

## 0. Corrections to the plan's counts (read first)

The speed-up plan and the playbook say "charts in 4 user files, equations
in 4, OLE objects in 17, 5 TIFFs". Measured now:

| Claim | Measured | Level |
|---|---|---|
| Charts in 4 user files (from `[chint]`) | `[chint]` is **not a chart**. It is the per-cell hint section of a table frame: each line is twelve numbers (row, column, flags, a width-like value, 65535 placeholders), closed by `[ehint]`. The frame reader asks for it when frame flag `0x100000` is set (reader near `1058:1f02`) and the table writer emits it per cell (`1490:0f5a`, closed by `[ehint]`). 4 sections, in 4 user files, each in a table frame that also holds `[tbl]` | D, C |
| Equations in 4 user files (from `[ehint]`) | **Zero.** `[ehint]` only closes `[chint]`. No picture of driver type `.tex` in the user's corpus (965 `[isd]` records), the ISO corpus (36), or the VM folder | C |
| Charts | **107 chart streams (108 pictures with the `.sct` driver) in 34 of the user's 423 SAM files**; none in the ISO corpus; 24 in 8 VM-lab files (the drawing run's twins). Only **7 distinct chart data records** over the 107: one 188-byte record (97 of 107), one 238-byte (5), one 263-byte (5) | C |
| OLE objects in 17 files | Confirmed: **34 objects in 17 user files**, none in the ISO corpus or the VM folder | C |
| 5 TIFFs | 2 stored as bitmaps after import (1 file), 3 linked TIFF pictures with an unknown driver `.sdp` (2 files), plus 2 `.tif` in the embedded directory. **No `.tgf` (AmiImage) picture anywhere** | C |
| (new) `.G0x` | 16 picture records in 3 user files name temporary metafile files (`.G00`...) that are not in the SAM; 2 frames with the old `[mf]` section (flag `0x40000`) point at `D:\TEMP\~BMP….TMP`. Pictures of an earlier Ami Pro version whose bytes live outside the file: not recoverable, they stay labelled boxes | C |
| (new) Linked frames | 3 frames in 3 user files carry flag `0x200000`: three strings after the frame record (application `123W`, a path, a range `A:G3..A:I16`): a 1-2-3 link | D, C |

Consequence: **equations and `.tgf` image processing occur in no corpus
file at all.** Charts are real (about 8 % of the user's files) but they are
copies of seven charts. OLE is the most common object (4 %). The user
decided SU2 (2026-11-04) that run D builds full editors for charts,
equations, and image processing; these counts are the reason to confirm it
once more (OD1).

## 1. The module map (R, D)

`AMIPRO.INI` `[AmiVISD]` lists the picture drivers, one DLL per kind:

| Type stored | Driver | Name in the INI | Notes |
|---|---|---|---|
| `.ole` | `AmiOLE.dll` | AmiOLE, formats `Native`, `ObjectLink` | **The DLL is not in the install set** (`Binaries/`); the client code is in `AMIPRO.EXE` (strings `OLE Embed`, `OLE Link`, `DDE Link`, `ObjectLink`, `OwnerLink`, `\protocol\StdFileEditing\server`, NewWave strings) |
| `.sdw` / `.sct` | `AmiDraw.dll` / `AmiChart.dll` | AmiChart stores `.sdw` streams with the `.sct` driver name | done for drawing (run 9) |
| `.tex` | `AmiEqn.dll` (830 KB) | AmiEquation | stored form **?** |
| `.tgf` | `AmiImage.dll` (232 KB) | AmiImage; `application11=.BMP,?,.TGF,TIFF2.FLT` converts to it | `VISDNonImportable` = AmiImage, AmiChart, AmiOLE, Aminw: these cannot be chosen in Import Picture |

`isd1`…`isd4` in `[AmiPro]` add the Tools menu items: Image P&rocessing…
(isd1, "Adjust the appearance of gray level scanned images"), Drawing…
(isd2), Charting… (isd3, "Create and edit a chart in a selected frame"),
Equations… (isd4, "Create or edit scientific and mathematical formulas and
equations"). The EXE's strings 3174–3177 are the same four. The menu slot
in the command table is `ISD PlaceHolder` (0x1a9). Each module exports the
driver entry points `LIBOPEN`, `LIBACTIVATE`, `LIBPAINT`, `LIBSIZE`,
`LIBPROPERTY`, `LIBPROPERTYSET`, `LIBDUPLICATE`, `LIBDESTROY`,
`LIBTERMINATE` (and `LIBMESSAGE` in chart, image); `LIBPROPERTYSET` is what
fills the `[isd]` record's saved state (the driver's own words, section
`PICTURE_NOTES.md`).

## 2. Charts

### 2.1 What Ami Pro 3.1 offers (R, H)

- **Tools › Charting…** (module `AmiChart`): works on a selected empty
  frame, a chart frame, or an empty cell; with no frame it makes one from
  the Create Frame settings (H ch. 18). Double-click on a chart opens the
  last function used on it (Charting dialog, or Draw mode if the chart was
  last edited with Drawing) (H).
- **Charting Data** dialog (dialog 1, "Charting Data"): one multi-line edit
  box. Data is numbers separated by tabs or spaces, labels in the first row
  (first column for a pie), `_` for a space or to make a number a label,
  currency symbols and commas accepted but not used, up to 5,000 rows and
  columns; Ctrl+Enter inserts a row. With the clipboard holding text, Tools
  › Charting uses it; with none it asks (strings 4011: "The clipboard
  contains no text data. Do you wish to enter the data now?"; 4013 too
  much data; 4012 frame too small) (R, H).
- **Charting** dialog (dialog 4): *Chart Type* group of twelve icons
  (ids 2200–2211; the guide names eleven: column, stacked column, bar,
  stacked bar, line, area, line and picture, pie, expanded pie, picture,
  stacked picture; the twelfth is **?**); each type has colour and text
  variations (row or column colours; pie labels inside a legend, outside,
  or none); *Options*: Legend, Grid, 3D with Depth 1–100, Perspective;
  *Color set* bar (sixteen colours, drag to reorder; defaults from
  `AmiChart.pal`); *Axis*: Automatic, Inc, Min, Max; **Flip data**;
  buttons **Data…**, **Pictures…**, OK, Cancel; a live example of the chart
  in the dialog (R, H).
- **Charting Pictures** (dialog 2): sixteen pictures used by the line-and-
  picture, picture and stacked-picture types; Default; **Drawing…** picks a
  `.sdw` file to use as a picture (dialog 3) (R, H). Defaults live in
  `AmiChart.ini` (strings 4000–4018 names).
- After charting, "use Drawing to add text or modify the chart segments"
  (H). A chart from another application is a pasted picture (H).

### 2.2 How it is stored (D, C)

- The frame is a picture frame (`0x20000000`); `[isd]` carries the picture
  type `.sdw` and, after the driver's saved state (688–704 words, charts),
  the driver name **`.sct`**; the bytes in `[Embedded]` are the **same
  stream as a drawing** (`DRAWING_NOTES.md`): a block of objects plus one
  **type 15 object, the chart data record** (reader `DrawMan 1008:87f0`,
  case 0xf; three counts, a numeric-text list, a byte list, and string
  lists read with `1008:8c0e/8d1e/8ba0/8ca8`). The ordinary objects around
  it carry flag 4 and are the chart's look; the data record is **never
  painted** (D, run 9).
- Corpus (C): 107 chart streams each with one type-15 record; its first six
  words are `(1,2,54,62,0,64)` in 96, `(4,1,54,70,0,74)` in 5,
  `(5,1,54,74,0,79)` in 5, `(2,1,54,62,0,64)` in 1; record sizes 188, 238,
  263 bytes. Words 3 and 4 (54, 62/70/74) look like offsets; the meaning of
  words 0 and 1 (series and point counts, or chart type and variation) is
  **?** until `amichart.dll` is read.
- What Ami Pro does on screen (D, run 9): `AmiChart`'s `LIBPAINT` draws the
  stored objects through DrawMan; **the data is only used when Charting is
  opened and OK'd, which rebuilds the objects.** Not yet seen, and the
  crux of the run: that Ami Pro accepts objects the app generated itself
  (**VM fixture CHV1**, section 7).
- What the app does today (code, C): `PictureFormat::Chart` is read as a
  drawing and drawn from its objects (`sdw::paint`), on screen, in PDF and
  in exports (run 9: 371 of 371 corpus drawings and charts); the type-15
  record is kept byte for byte, and `Picture::format` says Chart from the
  `.sct` word. **Not read:** the data record's contents. No Charting
  command, no data view.

### 2.3 Unknown (?)

The record layout (counts, floats, label lists); the meaning of the
twelfth icon; the layout rules that turn data into objects (axis ticks,
bar widths, 3D depth and perspective, colour sets, label placement; all in
`amichart.dll`, 162 KB); `AmiChart.pal`'s format; the sixteen default
pictures (Ami Pro's own artwork: not copied; the app draws its own). Order
of work: decode the record from the seven distinct corpus records, read the
object generator by the decompile, check by fixture (CHV1 to CHV4).

## 3. Equations

### 3.1 What Ami Pro 3.1 offers (R, H)

- **Tools › Equations…** makes a frame at the caret (or uses the selected
  one), enters **Equation mode** (an `Equation` menu in the menu bar and
  two rows of icons across the top); double-click on the frame, or select
  it and press Enter, re-enters (H ch. 30).
- The `Equation` menu (R, menu 10101): Math Mode / Text Mode (Ctrl+T);
  Limits & Size Big/Small; **Insert** ▸ Fraction (Ctrl+1), Radical
  (Ctrl+2), Superscript (Ctrl+3), Subscript (Ctrl+4), Operator…,
  Brackets…, Matrix…, Function…, Space…, Label…, Over/Under…, Revise
  Character…, TeX Command…; Greek Keyboard (Ctrl+G), Symbol Keyboard
  (Ctrl+Y); Show Matrix Lines; Show Input Boxes; Preferences…; the
  File-side items **Save As Equation…** (a TeX file) and **Import
  Equation…** (a TeX file) (strings 4001, 4002; H "Saving as a TeX file").
- Dialogs (R): Preferences, Import Equation, Save As Equation, Revise
  Character (accent, negate, bold; one character at a time), TeX Command,
  Brackets, Function, Label, Revise Matrix, Create Matrix (to 10 x 10,
  error 5102), Operator, Space, Over/Under, Revise Fraction, Revise
  Radical. Icon palettes (template, operator, toggle, dialog, character
  and symbol, pull-down) are customisable (H; `slot1`…`slot18`).
- Preferences (R, `AMIEQN.INI` key names): colours of the equation, math,
  text, function; italics in math; fraction line and space; inter-column
  and inter-row space; matrix margins; root width and depth; radical
  space; empty-box size; script space; text-to-script and text-to-operator
  ratios; font metrics (`EQFontMetrics`); the font face, size, colour,
  attributes; `TeXSavePath`, `TeXImportPath`. The module wants a Symbol-
  compatible font (H).
- Errors worth reusing (R): structure too complex (5100), matrices
  restricted to 10 x 10, equation too big (5105), error parsing TeX
  (5106), text attributes on one character only (5107).

### 3.2 How it is stored (?)

The driver type is `.tex` (R); that name, `CF_TEX` (string 4031, the
clipboard format), and Save As / Import being TeX files suggest the stored
bytes are **TeX-like text or a tree the module can write as TeX**. No file
in any corpus has one, so **nothing is known**: not the stored bytes, nor
the saved state words in `[isd]`, nor whether Ami Pro keeps a screen image
(the cache of `PICTURE_NOTES.md` says only that a rendering is never
needed) or redraws from the data every time (it must when the font or zoom
changes; the module carries bitmap fonts for 8514 and EGA operators,
`BINOPS_BASE14_8514`…). This is the single largest unknown of the run: an
unknown storage and an unseen layout engine.

### 3.3 What the app does today

`PictureFormat::Equation` exists in the model; `decode` returns
`Content::Unreadable`, so an equation would be a labelled box. Rotation is
off for equations (Graphics Scaling, `FRAMESCALEPROC`). Nothing else.

### 3.4 Fixtures needed (VM)

EQV1: a fraction typed by the palette, saved, read from `[Embedded]`
(bytes, the `[isd]` words, whether a rendering is stored). EQV2: the same
equation at two zoom levels (does the stored rendering change). EQV3:
Save As Equation of EQV1 (the TeX Ami Pro writes). EQV4: a matrix, a
radical, a sum with limits (coverage of the format). Only after DO.1's
decompile of `amieqn.dll` says what is still open.

## 4. OLE and linked objects

### 4.1 What Ami Pro 3.1 offers (R, H)

- **Edit › Insert ▸ New Object…** (0x24d, `1168:7055`; dialog 2012 "Insert
  New Object", a list of object types): needs an **empty selected frame**
  (3223: "This frame already contains data"; 3089 not with several frames
  selected). The server starts, the user edits, File › Update, Exit &
  Return. The object type list comes from the registry (H).
- **Edit › Paste Special…** (0x24c, `13b0:1563`; dialog 2011 "Paste
  Special", a Formats list): the clipboard's formats (Metafile/Picture,
  Bitmap, DIB, Rich Text, Text, BIFF, DDE Link, OLE Link, OLE Embed; EXE
  strings 1450–1452); plain Paste picks Ami Text, RTF, BIFF, Text,
  Metafile, DIB, Bitmap in that order in text and Metafile, DIB, Bitmap in
  a frame (H ch. 27). 3092: "Cannot accept an object that Ami Pro can not
  display"; 3185 asks for a visual or a data-passing view.
- **Edit › Paste Link** (0x9d) and **Edit › Link Options…** (0x73,
  `1160:0000`; dialog 1129 "Link Options": a Links list, Update,
  Unlink, Deactivate, Edit…, Create…; the **Link** dialog 2005 edits
  Application, Topic, Item) (R, H). DDE text links show blue brackets with
  Marks on; File › Doc Info lists links; strings 3076, 3081, 3213 warn
  when cutting a selection with a link; 3104 asks to update DDE links on
  open, 3106 NewWave links.
- **Double-click or Enter** on an OLE frame starts the server on its
  data (H). Editing is the server's: Ami Pro is only the client.
- Not used by this machine: Mail (3.1 needs MAPI/VIM), DDE conversations,
  NewWave (`[nw]`, `NW-Objects`), Notes/FX. `PHASE5_PLAYBOOK.md`: "Not
  planned (live OLE editing, mail, DDE)".

### 4.2 How it is stored (C)

- A picture frame whose `[isd]` has picture type `.ole` and the driver
  `.ole`, driver state 16 words (`PICTURE_NOTES.md`); the object's bytes
  are in `[Embedded]` with, in 13 of 34, Ami Pro's screen image beside
  them.
- **The bytes are an OLE 1.0 object stream (C, measured on all 34):**
  version `0x00000501`, format 2 (embedded), class name (length-prefixed),
  topic name (empty in all 34), item name (empty), native data (size then
  bytes), then a **presentation object**: version `0x501`, a format id `5`
  (a clipboard format), the class string **`METAFILEPICT`**, a width and
  height, a data size, and the data: a Windows `METAFILEPICT` (mapping mode
  8 = anisotropic in all 34, x and y extents) followed by **a metafile
  whose header starts `01 00 09 00`** (a plain metafile, not a placeable
  one). In 22 of 34 the stream ends there; in 12 more bytes follow the
  presentation (160 to 12,960 bytes; **?** a second presentation or link
  update data).
- Classes (C): `MSWordArt.2` 21, `WordArt` 10, `123Worksheet` 2,
  `MS_ClipArt_Gallery` 1. Native data: under 1 KB for WordArt and ClipArt,
  over 10 KB for the two worksheets (WordArt 2 native up to 10 KB).
  Sizes 2.6 KB to 28 KB, median 6.4 KB.
- 3 frames in 3 files have the link flag `0x200000` and strings
  (`123W`, a path, `A:G3..A:I16`): a **linked** 1-2-3 range (C, D flag
  meaning). Whether these are 3 of the 34 or separate frames is **?**
  (the two `123Worksheet` embedded objects against three link frames does
  not match one to one).

### 4.3 What Ami Pro draws, and what the app does

Ami Pro's `AmiOLE` driver paints the object from the stored presentation
(or from the screen image cache when valid); it never needs the server to
display (H: "the data appears as an OLE object" after the server closes;
inferred, **VM to confirm**). The app today (code): `PictureFormat::Ole`
decodes to Unreadable, so it draws Ami Pro's screen image where the file
has one (13 of 34) and a **labelled box otherwise (21 of 34)**. **All 34
carry a presentation metafile that `amipro-pictures::wmf` can play**: the
cheap, common win of the run (OLE draws 34 of 34, on the page, in PDF and
in exports).

### 4.4 Options for OLE editing: none here

No OLE servers exist on Linux for WordArt, 1-2-3, or ClipArt Gallery
(Windows 3.x 16-bit applications), and `AmiOLE.dll` is not in the install.
Editing means the Windows server. Live OLE stays not planned (as the
playbook); the object is kept byte for byte, drawn from its presentation.

## 5. Image processing

### 5.1 What Ami Pro 3.1 offers (R, H)

- **Tools › Image Processing…** (isd1) on a selected frame holding a
  **gray-scale TIFF of 4, 6 or 8 bits** (16, 64, 256 grays) that was
  imported (H ch. 19); not for colour or black-and-white pictures. Double-
  click on the frame does the same. It adds an **Image** menu to the bar
  (menu 1, `AmiImage`): **Processing…** (0x2710), **Revert** (0x2711),
  **Halftone** ▸ Fastest Printing, Best Picture Quality, Automatically
  Selected, Printer Driver, then **Posterize**; **Preferences…**,
  **Leave Image** (R).
- **Image Processing** dialog (R): Brightness, Contrast, Edge Enhancement,
  Smoothing (four scroll bars with a value readout), Invert Image (a
  check), Apply (the dialog stays), OK, Cancel. Importing a TIFF
  auto-adjusts these (H); **Image Preferences**: "AutoContrast Images When
  Loaded" (R). Halftone settings only affect printing; Posterize turns the
  darker grays black and the rest white (H).
- Revert restores the original TIFF image (H).

### 5.2 How it is stored (?)

The driver type is `.tgf` (R, `application11` converts a bitmap to it with
`TIFF2.FLT`); **no `.tgf` exists in any corpus** (C). Whether the settings
(six numbers and the halftone mode) and the original are kept in the
`[isd]` state or inside the `.tgf` bytes is **?** (the module has
`LIBPROPERTY`/`LIBPROPERTYSET`). The five TIFFs in the user's corpus went
through `TIFF.FLT` to ordinary bitmaps (`PICTURE_NOTES.md`), so even a
gray TIFF may not become `.tgf` unless imported through a particular path:
**VM fixture IMV1** (import a gray 8-bit TIFF; look at the stored type).

### 5.3 What the app does today

TIFF, PNG, JPEG, GIF, PCX, and bitmaps load through `image` and are
drawn; Save As SAM converts non-Ami formats to bitmaps. No adjustments of
any kind. The Picture section of Frame Properties has scaling, rotation,
pan, and description only.

## 6. Menus and commands this run touches (summary)

| Item | Ami Pro | App today |
|---|---|---|
| Tools › Charting… | opens the Charting dialog on a frame | missing |
| Tools › Equations… | Equation mode | missing |
| Tools › Image Processing… | Image menu + dialog | missing |
| Edit › Insert ▸ New Object… (0x24d) | server list | missing; no servers: not offered |
| Edit › Paste Special… (0x24c) | clipboard format list | missing |
| Edit › Paste Link (0x9d) | DDE/OLE link | not planned |
| Edit › Link Options… (0x73) | links list | missing; read-only list possible |
| Double-click a chart / equation / image / OLE frame | the module's editor / the server | chart: Draw mode (run 9); the others nothing |

## 7. VM fixtures (only what the decompile cannot answer; DO.2)

Reduced after the 2026-11-06 follow-ups (11.6). Dropped, answered by the
decompile: CHV3 (Flip), CHV4 (clipboard parsing, 11.1), EQV3 (the wrap rule
is known), EQV2, OLV2 (Link Options is read; the app keeps link strings and
lists them read-only; no 1-2-3 or other DDE server is in the VM anyway).
Kept, each as one sitting item:
- **CHV1** (make or break): a chart made by the app (3D clustered column,
  the corpus data 12, 23, 5, 14, record rebuilt, its flag-4 objects from the
  11.6.2 formulas) opened in Ami Pro: shown from the stored objects? Charting
  dialog opens with the right data, type and axis? OK regenerates the same
  objects (compare coordinates with the app's)? Saved by Ami Pro, is the
  record byte-identical?
- **CHV2** (one file, one chart per remaining family from one 3 x 2 data
  set, saved by Ami Pro, read back): 2D column, stacked bar, line, area, line
  and picture (2206), **icon 2207**, pie, expanded pie, 3D line and 3D area,
  perspective column. Question: the objects each produces (count and order,
  coordinates), and what 2207 is; fits the formulas for the families still
  at L (lines, areas, pictures, pie slice and label placement, legend box).
- **EQV1** (one file, two parts): (a) one equation per construct (a
  fraction, an indexed radical, a script pair, a sum with limits, a 2 x 2
  matrix, a bracket pair, text and a font switch) saved: the bytes in
  `[Embedded]` and the host's `[isd]` words, the 47-byte settings bytes, the
  text as stored; (b) the natural size Ami Pro writes for each, to fit the
  layout model (the fraction gap, script shift and rule thickness could not
  be recovered from the code). Replaces EQV1, EQV4.
- **IMV1** as before (not touched by this pass).
- **OLV1** as before (an OLE object with the presentation removed: redrawn
  or blank).
Files and screenshots outside Git, in `vm-lab/IN` and `OUT`, VM shut down
afterwards.

## 8. What is cheap and common (ranked by corpus value per cost)

1. **OLE objects drawn from their presentation metafile**: 34 of 34
   objects in 17 files (21 are boxes today). Parse the OLE 1.0 stream, play
   the METAFILEPICT with the existing WMF reader, keep every byte. A few
   hours.
2. **An Object section in the picture inspector** (class, native size,
   link strings) and a read-only **Edit › Link Options…** list: shows what
   each object is; almost free after item 1.
3. **Chart data shown as a table** in the rail (read the type-15 record):
   cheap once the record layout is decoded; useful for the 34 files.
4. **Paste Special…** listing the desktop clipboard's formats (text, RTF,
   HTML, PNG, DIB): small; also fixes the cases where plain Paste picks the
   wrong format.
5. Everything else (chart generation, the equation editor, `.tgf`) is
   large and has no corpus evidence beyond the charts.

## 9. Questions the decompile (DO.1) must answer

Charts: the record layout; the twelve types and their variations; how
objects are generated (axes, ticks, bars, 3D depth and perspective,
legend placement, label fitting, colour sets, pictures); `AmiChart.ini` and
`.pal` keys; which routine marks the objects with flag 4; how Flip data is
stored. Equations: the `.tex` payload and `LIBPROPERTYSET` words; the
parser and layout of templates; the character and symbol tables; how
`LIBPAINT` uses the font metrics. Image: the `.tgf` payload; the filters
(brightness, contrast, edge, smoothing, invert, posterize) as formulas; the
state in `[isd]`. EXE: the double-click dispatch for ISD frames (who picks
Draw, Charting, Equation, Image, OLE), the Edit › Link Options list rows,
Paste Special's format ranking, the Unlink effect on the file. Method:
`DECOMPILATION.md` (the NE modules `amichart.dll`, `amieqn.dll`,
`amiimage.dll` import as `amidraw.dll` did).

## 10. Libraries and licences to evaluate

Charts: none needed (the layout writes draw objects; `tiny-skia` paints
them already). Equations: a maintained math layout in Rust or the TeX
rules of Appendix G by hand; a free font with an OpenType MATH table (Latin
Modern Math, STIX Two; SIL OFL) as the default, never Ami Pro's fonts.
Image processing: the `image` crate (already in the tree) for the pixel
work; the formulas come from the decompile, written independently.

## 11. Decompile results (DO.1, 2026-11-06)

Modules decompiled (outside Git, `ghidra-work/out/`): `amichart.dll`
(NE, not packed, 16 segments, 13.4k lines), `amiimage.dll` (22 segments,
5.3k lines), `amieqn.dll` (98 segments, 23.8k lines); each imported with
Ghidra's stock NE loader (no unpacking needed: flags 0x8301/0x8309, no
OPTLOADER), about two minutes each. Evidence is the module's segment:offset
of a routine. Confidence: H, M, L. Baseline numbers are in
`OBJECTS_PROGRESS.md`.

### 11.1 Charts (`amichart.dll`)

**Architecture (H).** The module keeps its own instance block (offsets from
the block base) and talks to DrawMan by window messages (0x52c create an
object of a type and size, 0x543 get its data, 0x544 commit, 0x4c9/0x4cc walk
the objects, 0x542 test flag 4, 0x52d read the type, 0x4d0/0x4d3 delete).
`LIBOPEN` (1008:0c10) checks the frame is larger than 0x2d1 twips each way
(message 4012 otherwise), reads the clipboard text (`1018:0000`, message
4011 asks to enter data if none, 4013 too much: product of rows and columns
over 0x1388 = 5000), finds the existing type-15 record (`1028:0000`, read by
`1028:0aa8`), runs the Charting dialog (`OPTIONSPROC` 1058:005c), then
generates (`1010:0000`). `LIBPROPERTYSET` does nothing (the module has no
saved state words beyond what the host keeps). `LIBPAINT` redraws from the
objects. Charts hold no screen image of their own.

**Type-15 record, file form (H for layout, M for some field meanings).**
The record is the object header (16 bytes, type 15, flags 4, length) and a
body that is the module's memory image minus 12 bytes: a memory offset minus
0x0c is the file offset. With A = values per row (columns) and B = rows
(series) the length is 134 + 5AB + 20(A+B), checked on all 14 distinct
user-corpus records (204: A=1,B=2 or A=2,B=1; 254: A=4,B=1; 279: A=5,B=1).
File offsets from the record start:
- 0x10 A, 0x12 B (counts; a pie of n values has A=n, B=1; in the first
  samples B=2,A=1 means two points in one column).
- 0x14 = 0x36 (offset of the data in memory terms), 0x16 = 0x36+4AB (offset
  of the flag bytes), 0x18 = 0, 0x1a = 0x36+5AB (offset of the labels).
- 0x1c chart type code: 0 column, 3 bar (horizontal), 1 line or area (area
  when the variation byte at 0x1e is odd), 4 line and picture, 2 pie, 5
  picture. 0x1e the variation byte (colour or text variation, 0 to 6).
- 0x20, 0x22 two words copied to and from the options block (default 0 and
  1; their use is in the 3D code, unknown, L). 0x24: 0 when the options word
  at 0x72 of the block is 5, else 1 (the pie/legend-style flag, M).
- 0x26 axis Inc and 0x28 axis Min stored as words... (the writer puts the
  axis value in 0x32 when the axis mode is 1 and in 0x34 when it is 2; in
  the file these are the words at 0x26 and 0x28; the default in files is
  50, which is the depth default 0x32, so this word is shared with the 3D
  depth: M).
- 0x2a: A*B IEEE single floats, row-major with stride A (series rows of
  values), then A*B flag bytes (written 0), then B row labels of 20 bytes
  (NUL padded; a space is stored as `_` in the text form only), then A
  column labels of 20 bytes, then a tail: nine words (axis min and max as
  doubles in older versions; all zero in the corpus except the last, a
  float-looking word), a double (the axis step multiplied by a scale), the
  colour set of 16 PALETTEENTRY values (red, green, blue, flags 6), and a
  last word that repeats the variation (0x100 or the variation).
- Versions: the module treats a record shorter than the fields above as the
  old layout (0x101) and fills defaults (depth 0x32, scale 10, the 16 default
  colours); longer records are 0x102 and 0x103.
Corpus meaning: 96 of the user's records are an empty two-value pie
(values like 60/40) and the 5-record and 3-record groups are the 4- and
5-column charts; the 14 distinct records differ only in values, labels,
colours and the leading object id. Writing them back is byte for byte.

**Data text parsing (`1018:0078`, H).** Separator is a tab if the text has
any tab, else a space. Blank lines are skipped. Per line: tokens that parse
as numbers are values, the others are labels; the first token that is not a
number on a line is its row label, labels in a line with no numbers make
the column labels (the first such line only). A token is a number if it has
at least one digit and only digits, the decimal symbol, the thousands
symbol (skipped), the currency string (skipped), `+ - e E`; otherwise a
label. Labels are cut at 19 characters and spaces in them become `_`
(typing `_` gives a space on display). A value beyond the float range is
clamped; values under the minimum normal float become 0. A = the largest
count of numbers on any line, B = the count of lines with numbers; short
lines are padded with zeros. Up to 5000 cells (A*B).

**Charting dialog (dialog 4; H).** Controls: type icons 2200–2211 (ids
0x898–0x8a3), Legend 2000, Grid 2001, 3D 2002, Perspective 2003, Flip
data 2004, Automatic 2005, Depth 2700, Inc 2703, Min 2701, Max 2702, Data…
2212, Pictures… 2213, colour set bar 2400. Defaults (`LIBACTIVATE`,
1008:0332): column chart, clustered, no 3D, depth 50, legend on, grid
off, axis automatic, sixteen default colours, 10 twips of margin. Types
(options block fields): type 0 column or bar (orientation field), 1 area, 2
line (variation 2) or line and picture (variation 3), 3 picture or stacked
picture, 4 pie or expanded pie. Icon mapping from the dialog start-up
(M): 2200 column, 2201 stacked column, 2202 bar, 2203 stacked bar, 2204 line,
2205 area, 2206 line and picture, 2208 pie, 2209 expanded pie, 2210 picture,
2211 stacked picture; **2207 is never selected by the start-up code**
(L; probably a second line-and-picture variant, a fixture question CHV2).
Enabling rules (`1058:09ac`): 3D and Depth for column, stacked column, bar,
stacked bar, area, and the pies; Perspective only with 3D; Axis box for all
but the pies; Pictures… for the line-and-picture and picture types.
Pie icons disable the axis group.

**Generation (partly read).** Entry `1010:0000` (H): copies the frame
rectangle into the block, shrinks it by 0x90 twips when a title/legend row
is shown, picks the generator by type (pie: `1010:3d21`; the others:
`1010:248b`), then deletes the old flag-4 objects, adds the new ones as
flag 4, and writes the record (`1028:0640`). The bar/line/area generator
(M): finds the data range per series (stacked adds positives and negatives
separately), the axis scale routine `1010:1a66` picks a "nice" step from
the span divided by (frame height / 240 twips, or /200 when the legend is
on, ×3 for 3D) using the step table 1, 2, 5, 10 (and 3, 4 become 5) and
rounds min down, max up to multiples of it (Automatic), or takes Inc/Min/
Max when typed; then draws, in order: the frame fill and axis lines
(`1010:1050`, ticks and value labels), the category labels, per category
the bars/points (`1010:0267` bars, `06bd` lines, `2152` areas, `0b9d`
pictures) with 3D skew helpers (`1010:3715`). Everything is emitted as
ordinary draw objects (lines, rectangles, polygons, text 13) with flag 4.
**Not yet read in detail: the exact bar width and gap ratios, 3D depth and
perspective geometry, legend box placement, label font sizes and the pie
slice labels** — read them in DO.7 before building (they are needed to
match Ami Pro's objects; the corpus's seven distinct charts are the
oracle, as the runbook says).

**AmiChart.ini (H).** Section `Picture` (or `Scatter` for the XY kind), keys
`Picture1`…`Picture16` = a `.SDW` path (empty: the built-in index modulo 3);
section `Directory`, key `Directory` = the last folder of the Drawing…
picker. The colour set default comes from `AmiChart.pal` (string 4005; the
file is not in the install set).

**Flip data (M).** Flag at the options block word 0 of the "Data" state:
Flip swaps A and B and the two label arrays (`FUN_1050`/`1058` OK handler
`1058:061a`); it is not stored as a flag in the record, the stored arrays
are already flipped.

**Open for a VM fixture (minimum):** CHV1 (does Ami Pro accept generated
objects and reopen the data), CHV2 (which objects and words change per type,
incl. icon 2207, 3D depth/perspective: one 3D chart per type family,
read from the saved file), CHV4 (clipboard parsing edge cases; the decompile
settles most, so one text with labels, underscores, currency suffices).
CHV3 (Flip) is answered by the decompile (M) and can be dropped.

### 11.2 Equations (`amieqn.dll`)

**Coverage limit (H).** The decompile dump holds 596 functions in about half
of the DLL's 97 code segments; 48 segments are missing (the largest, 1218
at 57 KB, and 11f8, 1228–1248, 12c0–12d8, 1190, 1278 ...). The equation
tree, parser, TeX writer and layout engine live there, so **the layout
rules and the surface syntax of fractions, radicals, scripts, sums and
matrices are not yet read.** Done in 11.6 (853 functions created; syntax read, layout rules still open).

**Stored form (H for text, M for envelope).** The driver (`.tex`) holds the
equation as TeX-like **text**, parsed and laid out again at every
`LIBPAINT`/`LIBSIZE`; no bitmap or metafile is stored. `LIBPROPERTYSET`
(1008:1bcd) is a no-op: the module writes no `[isd]` words (the host does).
Per-object block: state byte (0x1c new, 0x1d loaded, 0x1e editing), a
47-byte settings record, a text length word, the text. Saved blob (M): state
byte 0x1d, the 47 settings bytes, the text from offset 0x30 (trailing NULs
dropped). Settings record: 32-byte face name (default `TimesNewRomanPS`),
pitch/family byte (18), size word (12), colour long, attributes word,
charset word, rotation word, one more word. A stream form exists (duplicate,
1008:05ad): one text line `face,pitch,size,colour,attrs,charset,rotation,x`
then CR LF and the equation text; which form is in `[Embedded]` is open.

**TeX dialect (H for the table).** Control sequences are in custom
resource 0xb2c8 (11 KB, sorted, no backslash): about 330 symbol names,
`matrix`/`endmatrix`, `root`/`of`, `sqrt`, `frac`, `binom` and their
text/display variants, big operators in three styles (`limits`/`nolimits`),
left/right brackets, over/under decorations, 27 function names, accents,
font switches (BF, IT, RM, CAL, FRAK, BBB, SL, SMC, TT, NEG), labels and
references, AmiTeX tokens (QTO, FRAME, MALIGN ...). Matrix cells are
separated by ` & `, rows by ` \\ `. Save As/Import: a TeX file is one
expression (no preamble or `$`); the writer re-wraps at the last space
before column 79 once a line reaches 70 characters (11a8:012e); the reader
turns LF into space and trims (11a8:03d9); clipboard format CF_TEX.
Keys `TeXSavePath`, `TeXImportPath`.

**Menu/ids (H).** Equation menu ids 0x2801–0x281e map to actions
(fraction 0x22, radical 0x3e, super/subscript 0x0e, operator 0x1a, brackets
0x19, matrix 0x18, function 0x21, space 0x20, label 0x1d, over/under 0x1b,
revise character 0x3a, TeX command 0x37, math/text 0x23, keyboards 0x3b,
matrix lines/input boxes 0x3d, preferences 0x3f). `[EQCacheBar]` slot1–
slot18 default ids: 7115, 7107, 7360, 7253, 7302, 7359, 7151, 7363, 7250,
7255, 7254, 7200, 7201, 7202, 7362, 7207, 7208, 7252.

**Preference defaults (H).** `[EQStyle]` ItalicsInMath 1, ItalicsMode 2,
ExtraFracLine 40, ExtraFracSpace 20, InterColSpace 180, InterRowSpace 20,
MatrixLeft/RightSpace 120, MatrixUp/DownSpace 40, RootWidth 120, RootDepth
40, RadicalRightSpace 40, ExtraRadicalLine 20, EmptyWidth 220, EmptyHeight
300, EmptyBorder 60, ScriptSpace 0, TextToScriptsRatio 100:70:50,
TextToOpsRatio 100:115:150 (units unknown, probably thousandths of an em);
`[EQColor]` math 255,0,0, text 0,0,0, function 128,128,128; `[EQConfig]`
MatrixLines 1, InputBoxes 1, PrinterMetrics 1; `[EQFontMetrics]`
default `tr21.efm`. Errors: 5102 matrix over 10 x 10 and 5013 (H); the
bounds of 5100 and 5105 are in the missing segments.

**Gate (OD2).** A plain-text or symbol equation is writable now. Fractions
and the rest need the missing-segment pass or EQV1/EQV3/EQV4. Minimum
fixtures: EQV1 (a fraction, saved: the envelope and syntax in `[Embedded]`
and the host's `[isd]` words), EQV4 (radical, script, sum with limits, 2x2
matrix, bracket pair in one document), EQV3 (Save As Equation of a long
one: wrap). EQV2 (zoom) is not needed for storage.

### 11.3 Image processing (`amiimage.dll`)

**Coverage limit (H).** `amiimage.c` holds the C runtime and the shell
around the image object (segments 1–4, 7, 14, 16, 17, 21); the engine
(segments 5, 6, 8–13, 15, 18–20) was read with an i8086 disassembly of the
DLL, so items marked M or L are weaker.

- **.tgf (M).** The module has no file layout of its own: it is a filter
  chain (stages Import: TIF, Import: BMP, Lookup, Negate, Smooth, Edge
  Enhance, Convolve, View dither, Quick/Smooth Resize); error strings "Bad
  Magic", "LZW" show the original is an embedded TIFF. Settings are never
  baked into the pixels: the chain is rebuilt at paint and print time;
  Revert resets the five settings to defaults. When `.tgf` is chosen over a
  bitmap, and the role of TIFF2.FLT, are not in the module (L): IMV1.
- **State blob (H, 1018:1691–1a4d, 153a).** Bytes DB DB, a count word (8),
  records of (tag word, length word, data): 1 version 1; 2 word 0x0310; 3
  brightness; 4 contrast; 5 edge; 6 smoothing; 7 invert; 8 halftone code
  (1 Fastest 0x2712, 2 Best 0x2713, 3 Automatic 0x2714, 4 Posterize 0x2715,
  5 Printer Driver 0x2716); terminator DF DF. Whether this is the `[isd]`
  state or inside the `.tgf` is open (M).
- **Dialog (H).** Four scroll bars (ids 258, 261, 264, 267) 0..100; defaults
  brightness 50, contrast 50, edge 0, smoothing 0, invert off; line 1, page
  10, Home 0, End 100. Apply refreshes with the dialog open, OK closes,
  Cancel restores the copy taken at start. Preferences: `AMIIMAGE.INI`
  section `LoadOptions`, keys `AutoContrast`, `DefaultHalftone`; also
  `ScreenCalibration`, `Gamma`, `PrinterCalibration`.
- **Order (H).** Import, Lookup (when brightness or contrast is not 50),
  Smooth (if non-zero), Edge Enhance (if non-zero), Negate (if inverted),
  then the view or print stage.
- **Lookup curve (H).** A 256-entry table from the polyline (0,0), (x1,y1),
  (x2,y2), (255,255), integer arithmetic with truncating division, B =
  brightness, C = contrast. C = 100: x1 = x2 = 255*(100-B)/100, y1 = 0,
  y2 = 255. C > 50: s = 510 - 255*C/50, u = (s+255)*(100-B)/100, x1 = u-s,
  x2 = u, y1 = 0, y2 = 255; if x1 < 0 then y1 = 255*(-x1)/s and x1 = 0; if
  x2 > 255 then y2 = 255*(255-x1)/s and x2 = 255. C <= 50: s = 255*C/50, y2 =
  B*(s+255)/100, y1 = y2-s, x1 = 0, x2 = 255, clipped alike on the y side.
  50/50 is the identity. Interpolation and rounding between points: open (L).
- **Auto contrast at load (M).** Min and max gray; mid = (min+max)/2,
  half = (max-min)/2; brightness = 100 - 100*mid/255, contrast = 100 -
  100*half/255; min = max gives brightness min*100/255, contrast 100.
- **Smooth, Edge, Posterize, halftone (L).** Strength = value/100.0 into
  "Convolve"-based stages; kernels, pass counts and the posterize threshold
  not found. Invert is "Negate" (255-x, M). Halftone affects printing only.
- **Fixture IMV1 (minimum):** import a gray 8-bit TIFF, record the stored
  type and state bytes; save with defaults, then each control alone
  (contrast 0 and 100 included; smoothing 100 and edge 100 on an image with
  a few gray runs) and each halftone item: settles the layout, curve
  rounding, kernels and threshold. Under OD8(a) the app needs only the
  five settings and its own filters; kernels are an app choice if IMV1 is
  skipped (OD8(b), the `.tgf` writer, needs IMV1 first).

### 11.4 AMIPRO.EXE: objects, Paste Special, links

- **Driver table (H).** `1018:1370` builds 0xEC-byte rows from `[AmiVISD]`
  and the `isdN` keys of `[AmiPro]`: kind ('I' driver, 'F' filter), menu
  command id, menu text, DLL name, picture type (`.sdw`, `.ole` ...), driver
  sub-type (`.sdw`, `.sct`, `.tex`, `.tgf`, `.ole`), four clipboard format
  names, capability bits. A frame stores the DLL name, type and sub-type;
  the row is matched by type and sub-type (1168:5002, 4f6e, 035a). **The
  frame's sub-type string selects the module; the EXE has no per-module
  branch** and calls the same entry points (resolved at 1168:4aa8) for all.
- **OLE (H).** `1168:6dcd`: frame flag 0x2000 with type `.ole` routes every
  driver call to a built-in OLE client (6650 activate, 6ab3 duplicate,
  6b8e open, 68a3 size); strings OwnerLink, ObjectLink, Native,
  `\protocol\StdFileEditing\server`. Painting goes through the client from
  the stored presentation; the screen-image cache is used when valid (M).
- **Edit mode (H flow).** Enter: `1110:0000(0x2000)` then 1168:0d04 (load,
  LibActivate code 1) and 1168:4495 (LibOpen); keys/mouse while editing go
  to LibMessage; leave: 1110:0a66 -> 1168:0eb1 (LibClose, LibDuplicate to
  commit, LibSize, repaint, document-changed). The handler of
  double-click/Enter on an existing frame was **not found** (candidates 1060:0000,
  15d8:309d, 12b0:0c52); "last function used" on a chart is the driver's own
  state (inside `AMICHART.DLL`'s LibOpen, not read) (M).
- **Tools menu isd1..isd4 (H).** Commands from 0x1aa; enabled when at most
  one frame is selected and it is empty or an ISD frame of that module
  (1168:17b6). Launch (1168:1a87): no frame: create one at the caret; a
  non-ISD frame: clear and convert; an ISD frame of another sub-type loses
  its data and is converted (Charting on a drawing frame replaces the
  drawing). Row bit 0x10 = can edit an existing frame (probe LibProperty
  codes 2–7).
- **Paste (M).** Plain Paste takes in order: Ami Pro's own copy, the Ami Pro
  link format, ObjectLink, Ami Text, Ami Tab Ruler (not in a frame), Ami
  Table/Frame, a format registered by a driver row (metafile -> `.wmf` row,
  DIB and bitmap -> `.bmp` row, Native/ObjectLink -> `.ole`; 1168:035a),
  CF_TEXT, NW-Objects; RTF and BIFF are handled after GetClipboardData
  (13b0:0457, rank unread). Inside a frame the Ami Text and ruler formats are
  skipped. 0x24c (13b0:1563) and 0x24d (1168:7055) are the macro forms; the
  Paste Special list is dialog 2011 (procedure in `AMIPROUI.DLL`, unread);
  messages 3089, 3092, 3185, 3223 are not raised in the EXE code read.
- **New Object (H).** Needs fewer than 2 frames selected; the type list is
  every registry class with a `\protocol\StdFileEditing\server` key; on a
  choice the frame is cleared, driver AmiOLE, type `.ole`, and the client's
  create-new runs (1168:0591 code 0x1002). The stream bytes are written by
  the OLE client library, not the EXE; the trailing bytes after the
  presentation in 12 of 34 corpus objects remain unexplained.
- **Link Options (not found).** `0x73` at 1160:0000 is the macro Link
  function (ops update, unlink/activate, create, paste-link 0x9d), not the
  dialog. Dialog 1129 (list 25350; Update 25586, Unlink 25585, Deactivate
  25587, Edit 25588, Create 25589) and the Link dialog 2005 (Application
  25300, Topic 25301, Item 25302) are in `AMIPROUI.DLL`; procedures unread.
  The row text, the Unlink effect on the file and the 0x200000 string writer
  (near 11c8:2e29) are open: a `DecompAt` pass at 1160:2afd/4072/03ca/
  3ec9/3ff7 and the UI DLL, then OLV2.

### 11.5 What DO.2 needs (minimum), and effects on the runbook

1. **Decompile follow-ups (no VM):** (a) the 48 missing `amieqn.dll`
   segments; (b) chart layout details (bar width and gap, 3D geometry,
   legend and label placement, pie: `amichart` 1010:1050, 0267, 06bd, 2152,
   0b9d, 3715, 3d21, 54b4, 1ffd) — a sonnet sweep in DO.7 with the corpus
   charts as the oracle; (c) Link Options, Paste Special and double-click
   procedures (UI DLL and the handlers named in 11.4).
2. **VM fixtures (after 11.6):** CHV1, CHV2, EQV1 (merged with EQV4), IMV1,
   OLV1. Dropped: CHV3, CHV4, EQV2, EQV3, OLV2.
3. Slice order: no change; DO.9's gate moves to "after the missing-segment
   pass and EQV1"; DO.5's `.tgf` branch (OD8b) stays behind IMV1.

### 11.6 DO.1 follow-ups (2026-11-06; decompile only, no VM)

Method: `scripts/CreateMissing.java` (outside Git, `ghidra-work/scripts`)
disassembled every code segment of `amieqn.dll` that had no function and
created functions at the segment start and at each `push bp; mov bp,sp`
prologue (also the `inc bp` far-proc form), then decompiled them: 853 new
functions in 48 segments, `out/amieqn-missing.c`. Charts were re-read with
an i8086 disassembly (FPU instructions decode) of segment 3, because the
decompiler loses the 8087 stack. Evidence: segment:offset; H high, M
medium, L low.

#### 11.6.1 Equations: syntax (H), preferences (M), layout (open)

**Reader** (lexer `11b8:08e2`, H). Plain TeX tokens: `{` `}` group, `$` and
`$$` toggle math/display, `^` superscript, `_` subscript, `&` cell break,
`%` line comment, `\` control word (letters) or control symbol; `--`, `---`,
`''`, `` `` ``, `?``, `!`` are ligatures outside math; a run of letters in
text mode is one word. Control words are looked up by name in resource 0xb2c8
(`1070:005f`); the table has 887 names with a class (symbol, operator,
relation, big operator, delimiter, accent, font switch, structure word).

**Writer** (what Ami Pro stores and exports; the node classes in segments
11f8, 12b8, 12c0, 12c8, 12d0, 12d8, 1218, 1228, 1248; H). A control word is
`\name`; arguments are braced; a space follows a name that ends in a letter.
- Fraction (`11f8:278e`): `\frac{num}{den}`; `\binom` the same; the style
  variants are `\tfrac`/`\dfrac`/`\tbinom`/`\dbinom` (node style 1 = d,
  2 = t). Braces around an argument are dropped for the `\atop`-family names
  (`QATOP`, `QABOVE`, `QOVERD`, ...), which are written infix with a space.
- Radical (`12d8:12d7`): no index `\sqrt{body}`; with index
  `\root{index} \of{body}` (AMS-TeX style, not `\sqrt[n]`).
- Scripts (`1248:27eb`, `1228:2868`): base, then `_{sub}`, then `^{sup}`
  (subscript always first); braces only when the script is more than one
  atom (`12f8:0000` decides); an empty group is `{}`.
- Big operator (`12c0:2de4`): `\sum` (names: `int iint iiint iiiint idotsint
  oint sum prod bigcap bigwedge bigoplus bigodot bigsqcup coprod bigcup bigvee
  bigotimes biguplus`, text-style `t...`, display-style `d...`) then a space,
  then `\limits` or `\nolimits` when the operator has an explicit mode, then
  `_{lower}^{upper}` with the same brace rule. Functions (`lim`, `sin`, `max`
  ... 27 names) are `\name `; an unnamed function is `\func{name}`
  (`\limfunc{name}` when limits go under it).
- Matrix (`12d0`, before `4c5d`): `\matrix ` cells separated by ` & `, rows by
  ` \\ `, then `\endmatrix `. A matrix whose columns are not all aligned the
  same way gets `\format \l & \c & \r \\ ` first (`\l \c \r`). Stacked scripts
  are `\Sb ... \endSb ` and `\Sp ... \endSp ` (same cell syntax). Matrix
  lines and input-box options are not in the text.
- Brackets (`12b8:0e41`, delimiter writer `10b8:068c`): `\left<delim> body
  \right<delim> ` with the delimiter followed by a space: `( ) [ ] \{ \} | /
  .` and the words `\langle \rangle \lfloor \rfloor \lceil \rceil` plus
  double bars and arrows; `.` is an invisible side.
- Over/under (`12c8:23d2`): `\overset{top}\to{base}` and `\underset{below}\to
  {base}`; accents and decorations `\name{arg}` (`\overline \underline
  \overbrace \underbrace \widehat \widetilde \overleftarrow ...`, `\hat \bar
  \vec \dot ...`); boxes `\fbox`, `\frame`.
- Text and fonts: `\text{...}`; font switches `\BF \IT \RM \SL \SMC \TT \CAL
  \FRAK \BBB \NEG` written as `{\BF ...}` groups (`10b8:0576`: bold then the
  negative-image switch).
- Spaces `\thinspace \negthinspace \quad \qquad`; AmiTeX extras `\label
  \ref \pageref \cite \glossary \QTO \FRAME \MALIGN ...` are reader-and-writer
  names with no layout of their own.
Save As Equation wraps the line as already noted (70/79 columns, `11a8:012e`).

**Preferences.** `[EQStyle]` is read once at start (`1180` region of
`amieqn.c`, 20 keys in resource order). Per entry a 4-byte slot: the first
two (ItalicsInMath, ItalicsMode) hold a flag in the low word; the next 16
(ExtraFracLine ... ScriptSpace) hold the value in the high word; the two
ratio keys are three numbers each (script ratios 100:70:50 are percent of
the text size for text, script, script-script; operator ratios 100:115:150
percent for text, display, display-large). The 16 spacing values are most
likely thousandths of the current font size (empty box 220 x 300 with a 60
border fits an em of 1000); not confirmed (L).

**Not found (open).** The `Measure`/`Draw` methods of the node classes are
virtual calls through class tables and none of the 853 functions names a
GDI drawing call, so the fraction-gap, script-shift, rule-thickness and
radical rules in words **were not recovered**; the preference names are the
only statement of them. The 47-byte settings record is copied unchanged
between the object block and the saved blob by `1008:1849`/`05ad` (face,
pitch, size, colour, attributes, charset, rotation, one word); nothing reads
it in the missing segments except the font setup. Pictures: confirmed that
nothing is stored besides the settings and the text (repainted at every
`LIBPAINT`, no cache), so the app can write any equation text and Ami Pro
re-lays it out on open (M until EQV1 shows the envelope bytes).
What Ami Pro does on open/edit: a double-click on the frame enters edit mode
(11.6.3); `LIBOPEN` loads state 0x1d and the Equation editor shows the text
parsed (parse errors 5013/5100-5105 reject the load and leave the text).

#### 11.6.2 Charts: geometry (formulas verified on the corpus where noted)

Options block words (as `w[i]`; the file record is a different, smaller
image, written by `1028:0640`, read by `1028:0aa8`): `w0` flip (1 = bars
horizontal; the generator draws a vertical chart with width/height swapped
through `1028:1002` and `3715`'s swap); `w1` variation (1 = clustered
bars, 0 = stacked; for lines 2/3/4); `w2` 3D (0 none, 1 oblique, 2
perspective); `w3` type (0 bar/column, 1 line, 2 area, 3 picture, 4 pie);
`w4` generate flag; `w5` low byte flags (bit 0 stacked-picture, bits 1,2
set for stacked column/bar, 0x2003 on icon 2207), high byte colour-set
index; `w6..w9` left, top, right, bottom of the frame rectangle; `w10`,
`w11` skew percentages (both 10); `w15` depth (default 50, 30 for pies);
`w16` columns A; `w17` series B; `w19` bar width percent (70 for column
and bar, 100 for line, area, picture, `LIBACTIVATE` `1058:0695` and
`1028:0aa8`); `w31` grid pen style (2 grid on, 5 off); `w33` legend flag.
The file record keeps: type (+0x1c), variation byte (+0x1e), grid flag
(+0x24), depth in +0x26 (oblique) or +0x28 (perspective), and **the
computed axis**: after the labels a word, then axis minimum, axis maximum,
and step as doubles, the 16 colours (RGB + pattern byte 6), a last word.
Corpus column chart: 0.0, 30.0, 10.0.
Icon-to-options map (`1058` start-up, H, corrects 11.1): 2200 clustered
column (w1 = 1), 2201 stacked column (w1 = 0), 2202 stacked bar (flip, w1 =
1... the code sets w1 = 1 with flag 6), 2203 clustered bar; 2204 line, 2205
area (w19 = 100), 2206 line and picture (variation 3), **2207 = the same
with extra flags 0x2003 (the XY/scatter variant: `AmiChart.ini` has a
Scatter section)**, 2208/2209 pie / expanded pie (w15 = 30), 2210 picture,
2211 stacked picture.

*Axis scale (`1010:1a66`, H for structure).* Target tick count = plot height
/ label height (label height measured from a sample string, 240 twips when
no device; three times larger divisor, i.e. a third of the ticks, when the
bars are horizontal). raw = (max - min) / target; for a vertical chart in
Automatic raw is doubled. Take the first digit d and exponent e of raw:
d = 1, 2, 5 keep; 3, 4 become 5; 6..9 become 1 with e + 1. step = d * 10^e.
min is rounded down and max up to multiples of the step (min starts at 0
when all values are positive; max starts at the largest sum for stacked
bars). Typed Inc replaces the step (never below raw), typed Min/Max replace
those ends. Corpus: values 12, 23, 5, 14 -> 0, 30, 10 (raw about 12, d = 1).

*Plot box and 3D (`248b`, `3715`, H; ±1 twip rounding).* Skew dx =
depth/100 * 10% * (right - left), dy = depth/100 * 10% * (bottom - top); the
back wall is the plot box shifted by (dx, -dy) from the front plane;
perspective (w2 = 2) instead moves each point by ((R - x + sx) * (|depth|/4)
/ 100, (y + T - sy) * (|depth|/4) / 100). Objects in order (all flag 4):
left wall, floor, back wall (polygons, grey ramp), then per tick from the
lowest: gridline across the back wall and a diagonal on the side wall (none
at the maximum, which is the wall edge), then the value label (not at 0),
labels left of the plot (right-aligned, about 24 twips lower than the tick),
then the data shapes, then the type-15 record last.

*Bars (`0267`, H).* Category width cw = plot width / A (lines and areas use
A - 1 intervals); centre cx = left + cw * (k + 1/2); bar width W =
cw * w19 / 100 (gap = the rest, half each side); clustered: each of B bars
is W / B wide, side by side from cx - W/2; stacked: one bar W wide, each
series starting at the previous top (positives and negatives stacked
separately). Heights: y = baseline - (v - min) * plotHeight / (max - min),
integers by truncation. 3D bar: front face at x - dx (front plane), a top
face (top edge plus the skew) and a right face; fills use the series colour
at 100 % (front), 90 % (top) and 50 % (right) (`54b4`; negative bars swap
90/50); each bar is a group of the three polygons. Colour = palette[series
mod 16] (palette[category mod 16] when the vary-by-category bit 0x10 of the
type byte is set, always for a single series pie); a palette pattern byte 6
= solid, 7 = hollow, else hatch 6 + n; channel = byte * pct / 100, a zero
channel becomes 0x20 when pct > 100, clamped at 255.

| Chart (corpus) | Formula | Corpus result |
|---|---|---|
| 3D clustered column, 4 values, depth 50 | cw = 1338 / 4 = 334.5, W = 0.7 cw = 234 | front faces 234 wide, x0 481, 816, 1150, 1484 (formula 481.2, 815.8, 1150.3, 1484.8) |
| same, heights | unit = 807 / 30 = 26.9; y = 1071 - 26.9 v | tops 748, 452, 936, 694 (formula 748.2, 452.3, 936.5, 694.4) |
| same, shading | 100 / 90 / 50 % of (255,0,0) | 255, 229, 127 |
| same, skew | 0.05 * width, 0.05 * height | dx 67, dy 40 on bars, 41 on the floor (±1) |
| same, axis | step table | 0, 30, 10 stored; labels 10, 20, 30 |
| same, object count | 3 walls + 3 labels + 6 lines + 4 groups of 3 + record | 15 top-level objects, 12 nested polygons |
| line, area, picture, stacked picture | see below | 40, 42 and 60 object streams not yet fitted (L) |
| pie (96 empty 60/40 pies) | angles | not fitted (L) |

*Lines (`06bd`, M).* From category 1 on, a segment from the previous point
(needs two categories); with 3D a ribbon of polygons using the same skew;
the segment is clipped to the plot (`38ff` clips a polyline to the top and
bottom edges). Areas (`2152`, M): a polygon from the line down to the
baseline per series, 3D adds side and top strips, stacked by cumulative top.
Pictures (`0b9d`, L): bar-like extents filled with the picture slot
(`AmiChart.ini` Picture1..16). Pie (`3d21`, `46c8`, `5239`, M/L): slice
angle = |v| / sum |v| * 2 pi, cumulative from angle 0 with the slice that
contains 90 degrees drawn first; the ellipse is fitted to the frame (radius
from the width, shrunk to (height - depth term) / (2 + depth); the legend
text width, measured over all labels, is cut from the width when a legend is
on); depth and tilt come from w15 and w32; slice labels are placed beyond
the rim at the mid-angle; angle tests use 1.5708 and 6.283184 constants
(`DAT 0760, 0778`). Donut and scatter: not found as separate generators
(L). Legend (`LIBOPEN` shrink): when on, the frame bottom is cut by 0x90
twips (144) and the legend row (colour square + label) is built from the
record labels in order.
Number labels: prefix string (record), value formatted by the C runtime to
at most the digits needed for the step, trailing zeros trimmed
(`1020:0564`, `1010:0e60`). Objects per type: column/bar 2D = frame fill +
axis lines + per tick (line + label) + one rectangle per point; 3D as in
the table; pie = one polygon per slice (plus rim polygons in 3D) + labels.

#### 11.6.3 UI DLL and EXE: links, Paste Special, double-click (H unless noted)

- **Link Options** (command 0x73, handler `1160:0000`, dialog procedure
  `LINKSDLGPROC` 13b8:0000, dialog 1129). The list rows are supplied by the
  EXE (callback 0x39), one per link, each a line whose first number is the
  link id and whose text is "OLE class server doc" or the DDE triple
  application, topic, item (separated at spaces; `13b8:0492` splits it). Selecting
  a row enables Unlink, Update, Deactivate and Edit (Edit and Deactivate
  only when the id is active). Buttons return a code and the selected ids to
  the EXE: 1 Unlink, 2 Update, 3 Deactivate, 4 Edit, 5 Create. Unlink: DDE
  link, remove the conversation, the link record (`4072`) and the link
  markers in the text (`03ca`: the range of the link in the text is found by
  id, type 0x65, its markers deleted, the shown data stays); OLE link, the
  OLE client's unlink. Update: ask the server for the data now (DDE `2b84`;
  OLE update). Deactivate: end the conversation (DDE `2afd`) or close the OLE
  link, the data stays. Edit: opens the Link dialog 2005 (Application, Topic,
  Item) prefilled; on OK terminates, deletes and recreates the link with the
  same id with the new strings and requests the data. Create: dispatches
  Paste Link (0x9d). Links of frames (flag 0x200000) are *not* in this list
  as frames; the list holds DDE links (text markers) and OLE objects.
  The 0x200000 strings are written by the picture-frame writer `11c8:2e29`
  after the frame record (lines for the three strings, the picture file
  names and the `.eps/.ole` extension in the same routine); M, not read line
  by line.
- **Paste Special** (command 0x24c, dialog 2011, `PASTESPECIALPROC` 1348:0000;
  filled by EXE callback 0x105, `13b0:15c7`). The list is the clipboard's own
  format list in enumeration order (the source's priority), each format
  kept if the acceptance test `13b0:1681` allows it (own Ami Pro format,
  Link format, text, metafile/bitmap/DIB when a frame row accepts them, a
  driver row's formats, others not), shown by name (`13b0:179b`): standard
  ids 1..8 map to resource strings (Text, Bitmap, Metafile, SYLK, DIF, TIFF,
  OEM text, DIB) and registered formats by their clipboard name (Rich Text
  Format, Native, ObjectLink, and so on). There is no ranking beyond the
  clipboard order, no "As:" text, and no "display as icon" in 3.1; OK returns
  the chosen name and the EXE pastes that format; a double-click on a row is
  OK. **Paste Link** is a separate dialog (`PASTELINKPROC` 1370:0000, the same
  three fields Application, Topic, Item, OK only when all three are
  filled, prefilled from the clipboard's link format).
- **Double-click on a frame** (`1048:067e` mouse procedure, case 0x203, then
  `1258:08cd`; H for the path, M for the last step). With exactly one frame
  selected that is not a table: the frame is selected; if its type flag
  (0x2000, "special": ISD, OLE, chart, equation, image) is set and the driver
  row allows editing (`1168:386b`, the can-edit probe), it enters the module's
  edit mode with `1110:0000(0x2000)` (the same entry as Edit mode on the
  Tools menu); no WM_COMMAND is sent. For a plain text or picture frame the
  frame is selected and `1258:0b66` follows (the frame properties).
  With a table cell and the table menu state it posts command 0x275 (Edit
  Formula) or enters table edit. Enter on a selected special frame: not
  found in this pass (it probably maps to the same `1110:0000(0x2000)`).

## 12. VM results (DO.2, 2026-11-06)

One sitting in the `Windows XP` guest (Ami Pro 3.1 from the ISO). Files and
screenshots stay in `../AmiPro-Reference/vm-lab/OUT/objects/` (`CHV2A-C`,
`EQV1A-B`, `IMV1A-L`, `EQnn.TEX`, `OLV1A`) and the screens folder. The
saved bytes were read with a small script (outside Git) that walks the
`[Embedded]` directory and the SDW object stream (notes 3.x, DRAWING_NOTES).
Evidence: V = read from a saved file, S = read from a screenshot, F = fitted
to several files, L = a guess. CHV1 was skipped (the app has no generator).

### 12.1 CHV2: eighteen charts, one 3 x 2 data set

Data: columns Jan, Feb, Mar; rows East 10 20 30 and West 15 5 25 (clipboard
text typed with tabs; the Charting dialog reads it by itself, no Data box
needed). Frames 3.2 x 2.7 in (4608 x 3888 twips; one 2.0 x 1.5). The icon
grid is two columns by six rows; by position (V): column, stacked column;
clustered bar, stacked bar; line, area; line and picture, **2207**; pie,
expanded pie; picture, stacked picture. Three documents hold 6 charts each
(`CHV2A` column, stacked column 3D, bar, stacked bar 3D with perspective,
line, area 3D; `CHV2B` 2206, 2207, pie, expanded pie 3D, picture, stacked
picture; `CHV2C` column 3D, area, expanded pie, stacked column, column in a
small frame, and one chart whose 3D click did not take, so perspective on a
plain column is untested). There is no percent chart: the twelve icons are
all the variants. No `AMICHART.INI` is written (the guest has none after 18
charts); only AmiDraw, AmiEqn and AMIPRO ini files exist.

**The data record (V, all 18).** Length 264 = 134 + 5AB + 20(A+B) with
A = 3, B = 2, as 11.1 says. Fields read back, by offset from the record
start: +0x1c type (column 0, line or area 1, pie 2, bar 3, line and
picture 4, picture 5); +0x1e variation (6 clustered, 0 stacked, 3 on 2207;
line 0, area 6, pies 0, 2206 0); **+0x20 is the legend flag** (1 when the
Legend box was ticked, 0 otherwise; 11.1 left it unnamed); +0x22 always 1;
+0x24 the grid flag; **+0x26 depth for oblique 3D (50), +0x28 depth for
perspective (50)**, 0 when 3D is off. After the labels: a word, then axis
minimum, maximum and step as three doubles at tail +2, +10, +18, the 64
colour bytes, a last word. Last words seen: clustered column 0x1006 (0x10 =
colour by category), clustered bar and picture 0x0006, stacked 0x0100, area
0x0106, line 0x0000, **2207 0x2003**, expanded pie 0x0100. Axis stored:
column 0 / 35 / 5, stacked column and area 0 / 60 / 10, clustered bar 0 / 40 /
10, stacked bar 0 / 60 / 20 (horizontal bars use a third of the ticks, 11.6.2
confirmed). **The maximum is the next step strictly above the largest value
or stack** (30 gives 35, 55 gives 60, 23 gives 30 in the corpus), not the
rounded-up value.

**2207 (V).** It is the line-and-picture chart whose marker follows the
category: triangle at Jan, diamond at Feb, square at Mar for both series,
where 2206 gives each series one marker (triangle for East, diamond for
West). Record variation 3 and last word 0x2003. Not an XY scatter; there is
no scatter generator.

**Common geometry (V, F).** The drawing rectangle for a 4608 x 3888 frame is
4103 x 3395 (a 144 twip margin on all four sides of what is drawn: the
first value label starts at x = 144, the top label at y = 144, the lowest
text ends at 3395 - 144). In the 2880 x 2160 frame the labels shrink (box
height 199, not 233) and the rectangle is not in proportion; the rule for
the frame-to-rectangle step is open (L). Text objects: Times New Roman,
height 233 for the axis labels (193 for the legend, 200 in the small
frame), box width the text width, drawn from its top-left. Axis lines and
tick marks are 10 twip lines; grid lines are dotted (style 2); all objects
carry flag 4.

*Ticks and data use different heights (F):* data y = base - v * H / max and
ticks y = base - v * (H - 4) / max, with H the plot height (2644 in the
2D charts, from y 256 to 2900; 2627 for the oblique 3D front plane, 275 to
2902; ticks start 4 twips lower, e.g. 260 not 256). Data values are rounded
to the nearest twip (10 gives 2145 from 2900 - 754.4... = 2145.6), ticks
truncated.

**Plot left edge (F).** The value labels are right-aligned; the plot's left
edge is 442 with tick marks (the label box ends 99 left of the axis, ticks
are 50 long) and 461 with a grid (label right edge 355). Right edge: last
category label centred on its slot. The plot top is 144 + half the
label height (260) when value labels run to the top.

**Column, bar (V; confirms 11.6.2).** cw = plot width / A; bar group width
0.7 cw, centred (gap 0.15 cw each side); clustered bars side by side,
W/B wide (279.5 for cw 800: 581..860, 860..1140); order in the stream: left
wall (3D), floor, back wall, axis lines, per tick the grid line then its
label, then per category its label followed by its bars. Clustered column
colours go by category (palette 0, 1, 2 = 0000ff, 0080ff, 00c200 for Jan,
Feb, Mar; legend swatches both blue); stacked and line types go by series.
Horizontal bars: same rules turned; within a category the second series is
drawn above the first. 3D oblique (V): skew dx = 0.05 * plot width and dy =
0.05 * plot height at depth 50 (the front plane left 458, the back wall
shifted by +167, -131), bar groups of three polygons (front, top, right)
painted 100, 90 (0000e5) and 50 % (00007f); the bars begin 0.15 cw inside the
front plane, 8 twip pens, the floor grey c0c0c0. Matches the corpus
formula, so no change to 11.6.2.

**Line (V, F).** Plot x from 442 to 2696; point k at left + k * width /
(A - 1) (442, 1569, 2695: two intervals of 1127). Per category after the first
the stream holds the segment of each series then the markers: east segment,
east marker at the previous point, west segment, west marker, then the next
category's label, and so on (the last markers come after the last segment). Markers 150 wide by 153 tall: triangle
(x, y-77) (x-75, y+76) (x+75, y+76); diamond (x, y-77) (x-75, y) (x, y+77)
(x+75, y); square 149 x 153. Segments are 10 twip lines from marker centre
to marker centre.

**Area (V).** Always stacked (East at the bottom, West above it): per
interval between two categories one quadrilateral per series, East first,
then West between East's line and the sum; the label of the next category
is between intervals; axis 0 / 60 / 10; height 2644 / 60 per unit (44.07).

**Picture charts (V).** Bars replaced by shapes of the bar's size: series 1
a triangle (apex at the top centre, base on the baseline), series 2 a
diamond; clustered side by side as bars (W/B wide each), stacked picture
stacks them with W = 0.7 cw and the diamond's top at the sum. Pictures from
AmiChart.ini were not chosen.

**Pie (V, F).** One series is shown: the first column (Jan: 10 and 15, so
40 and 60 %), one slice per row, the legend lists the rows. Centre: x = half
the width left of the legend (3005 / 2 = 1502.5), y = half the rectangle
height (1697.5); radius 1214 (= (3005 - 2 * 288) / 2, limited by the
width). Slices begin at 3 o'clock and run counter-clockwise in row order.
Each slice is one polygon: arc points every 0.05 radian from the slice's
start angle, a last arc point at the slice's end angle, then the centre
(53 points for 144 degrees, 78 for 216). Expanded pie: two groups, the
East slice moved out along its mid-angle; 3D adds a top face, two straight
sides and a rim strip (about 25 points a rim); the 3D variant has 11 objects
and 3 groups, the flat one 2. Pie colours 0000ff, 0080ff.

**Legend (V).** Present when ticked: first objects of the stream, a group of
a black shadow polygon (shifted 10 right and 14 down), a white box, and per
series a swatch (193 wide, 270 tall) and a label. Width = 96 + 193 + 194 +
widest label (338 for "West") + 107 = 928; height = 135 + 405 * rows;
swatch left 96 inside, row pitch 405, text 194 right of the swatch. Right
edge 3944 (the rectangle's 3959 less 15); vertical placement about the
rectangle's middle (box top 1246 for 2 rows in 3395: the centre is 21
below the middle; with three or more rows still to check, L).

**What changes the build.** (a) The record's +0x20 is the legend flag, +0x26
and +0x28 the 3D depths: write them. (b) The axis maximum is strictly above
the data. (c) Data and tick heights differ by 4 twips as above. (d) 2207 is
a category-marker line, area is always stacked, pie shows the first column.
(e) Object counts for the 3 x 2 set, flag 4, stream order as above: 2D column
with legend and grid 31 (+1 group), bar 19, line 29 (+1 group), 2206/2207
35 (+1), picture 31 (+1), stacked picture 29 (+1), area 21, 3D column 38 (6
groups) or 31-42, stacked bar 3D 36 (7 groups).
Still open: the frame-to-rectangle step for other frame sizes, the 3D pie and
perspective geometry, the legend for 3+ series, flip, pictures from the
`.ini`, and whether Ami Pro accepts objects the app wrote (CHV1).

### 12.2 EQV1: equations (V; layout fits are L)

Method: Tools › Equations opens Equation mode in place; **File › Import
Equation reads a TeX file**, File › Save As Equation writes Ami Pro's own
normalised text. The Equation menu has Text Mode, Limits & Size Big, Insert
(Fraction, Radical, Superscript, Subscript, Operator, Brackets, Matrix,
Function, Space, Label, Over/Under, Revise Character, TeX Command), Greek and
Symbol keyboards, Hide Matrix Lines, Hide Input Boxes, Preferences. **TeX
Command is a picker of command names (a list from `acute` on), not a text
box.** Twenty-seven equations were imported from files in `IN` and saved.

**Stored form (V).** The picture directory entry is `N .tex OFF LEN ROFF
RLEN`; the object is `LEN` bytes of **two text lines**: `TimesNewRomanPS,18,
12,0,0,0,0,0` (face, pitch and family 18, size 12, colour, attributes,
charset, rotation, one more zero) then CR LF and the TeX text; no NULs, no
47-byte binary settings (11.2's record is the in-memory block). The settings
come from `[EQEnvironment]` in `AMIEQN.INI` (written at first use with all
the defaults of 11.2, `RootWidth=120 (to go)` and `RootDepth=40  (to go)`
with those words kept in the file). The rendering after it is a screen bitmap
(954 bytes for a fraction). The frame is an ordinary anchored frame, driver
`.tex`; `[isd]` is `.XN .tex .XN 1 1 0 0 W -H 100 0 0 .tex 0 -B 0` with the
natural width W, the natural height H as a negative 16-bit word, and B the
baseline height (ascent) also negative; the frame record's last field is
`W B` and the frame is exactly W by H twips.

**Syntax as Ami Pro writes it (V).** Math runs are wrapped in `$ ... $`; a
single-atom argument loses its braces: `\frac{a}{b}` becomes `$\frac ab$`,
`\binom{n}{k}` `$\binom nk$`, `\sum_{i=1}^{n}` `$\sum_{i=1}^n$`,
`\int_{0}^{1}` `$\int_0^1$`, `x_{i}^{2}` `x$_i^2$`; `\sqrt{x}` and
`\overline{x}` keep braces (`$\sqrt{x}$`); index radical `$\root{3} \of{x}$`;
matrix `$\matrix a & b \\ c & d\endmatrix $` (no space before `\endmatrix`, one
after); brackets `$\left( x\right) $`; bare `\sum` and `\int` get a trailing
space (`$\sum $`); text `\text{abc}`; a font switch `\BF` outside `$` stays
plain text (`\BF x`). Words outside `$` are text mode (roman, black); inside
`$` math mode (italic, red on screen).

**Natural sizes at 12 pt (twips, V).** Plain text `x`: 162 x 257, baseline 192;
math `x` 138 x 257 / 192; `xyz` 371; line height 257 = ascent 192 + depth 65.
`x^2` 255 x 268 (ascent 203); `x_2` 255 x 257; `x_2^2` 279 x 286 (ascent 221).
`\frac ab` 209 x 287, `\binom nk` 262 x 287, `\frac xx` 228 x 257,
`\frac{xxx}x` 346 x 257; nested `\frac{\frac xx}x` 225 x 332 (ascent 267).
`\sqrt{x}` 423 x 258 (ascent 193), `\sqrt{xxx}` 610 x 257; `\root{3}\of{x}` 504 x
259. `\sum` 172 x 257; `\sum_a^b` 351 x 266 (ascent 201); `\int` 90 x 257;
`\int_a^b` 269 x 330 (ascent 240). 2 x 2 matrix of `x`: 1098 x 614, baseline 377
(a, b, c, d: 1126 x 614). `\left[ x\right]` 288 x 257; `\overline x` with
`\underbrace y`: 443 x 408.

Fits (F or L, from these only): horizontal padding about 44 total around
a one-glyph box (138 = 94 for an `x` + 44 pad); each extra `x` adds 93.5.
Scripts add 117 to the width and raise the ascent by 11 (superscript alone)
or 29 (both scripts present). A fraction's width is the widest operand at about
63 % of the text width plus about 170, and its height does not grow beyond
the line height for operands without ascenders (the natural height tracks
the ink and the line box, not an abstract stack). The radical adds 285
to the width and nothing to the height. Operators with limits at 12 pt put
the limits at the side (text style) and add 179 to the width. These are
descriptive, not a layout model: the sizes are enough to set a frame's natural
size within a few per cent for simple equations, and the app may use its own
TeX-style layout.

**Not measured:** other sizes and faces (the size is fixed by the settings
line), the `\left` side pairs other than `( )` and `[ ]`, Over/Under
variants, display style, `limits` placed above and below, `Limits & Size Big`.

### 12.3 IMV1: Image Processing (V, S)

A 64 x 64 8-bit gray TIFF (a gradient over the top half, four bands 0, 85,
170, 255 below) imported into a 3 x 3 in frame with Copy image: the saved
frame has driver `.tgf`, and **the stored picture is the TIFF's own bytes,
unchanged, 4270 of 4270 bytes, named `.tgf`**; the effect settings are never
written into it (non-destructive). The frame's `[isd]` carries the state
blob as 16-bit words: `.XN .tgf .XN 18 1 0 0 W -H 100 0 32` then `0xDBDB`
(56283), the record count 8, then records (tag word, length word, data
bytes) and `DF DF`: tag 1 version (1), tag 2 `10 03`, 3 brightness, 4
contrast, 5 edge enhancement, 6 smoothing, 7 invert, 8 halftone code, each
one byte; then twelve zero or name words (`.tgf`). Confirmed by saving
each control alone: brightness 100 or 0, contrast 100 or 0, edge 100,
smoothing 100, invert, brightness 30 with contrast 70 each changed exactly
that byte. **Halftone codes** (menu order differs): Fastest 1, Best Picture
Quality 2, Automatically Selected 3, Posterize 4, Printer Driver 5 (the
`DF` high byte in the last record word). The first save after import holds
a 213462-byte screen rendering; saves made from Image mode hold none (the
directory gives 0 0) and Ami Pro redraws from the TIFF.

Dialog start state for this image: **brightness 49, contrast 50** (min 0 and
max 255 at load; 11.3's auto-contrast formula gives 51 for the contrast, so
use these observed values for a full-range image). Menu: Image ›
Processing…, Revert, Halftone ›, Preferences…, Leave Image; Tools › Image
Processing enters it. Revert returns to the loaded state.

Screen pixels (the display applies its own gamma and 16-level steps: raw 0,
85, 170, 255 show as 0, 139, 197, 246, so white is 246): brightness 100 and
contrast 50 give all white, brightness 0 all black, **contrast 100 at
brightness 49 a threshold at 130** (raw 129 black, 161 white; 11.3's
`255 * (100 - B) / 100` = 130), contrast 0 a flat 124 (= 49 * 255 / 100),
brightness 30 with contrast 70 a ramp from 0 at 132 to 205 at 255 (shown
222 = the curve's 205). 11.3's lookup curve is therefore right at its end
points. Edge enhancement 100: a ringing of one source pixel on each side of
a step (a step 139 | 197 shows 0 and 246 beside it) and a lift of about 8 display
levels on a ramp, so a sharpening kernel with overshoot to saturation.
Smoothing 100: a blur of about one source pixel (the 0 | 139 step gets 74 and
106 between). Invert is 255 - x (246 shows 32 and 0 shows 246).
Posterize and Printer Driver change no screen pixels (print only). Not
fitted: the kernels' weights and the intermediate strengths (needs a screen
capture of a wider test image at several strengths, or the print path); no
`AMIIMAGE.INI` exists in the guest.

### 12.4 OLV1: no usable OLE server (V)

Edit › Insert › New Object… lists Video Clip, Lotus Annotator Note, Lotus
Media, Lotus Sound, MIDI Sequence, Media Clip, Package, Bitmap Image,
Paintbrush Picture, Wave Sound, WordPad Document (the registry's OLE 1
servers, no WordArt, 1-2-3 or ClipArt). Choosing Paintbrush or WordPad made an
**empty ordinary text frame**: no server window opened and the saved file
(`OLV1A`) holds no `.ole` entry, no `[Embedded]` data. The guest cannot make
an OLE object, so the "presentation removed" test and the double-click
test could not be run; Edit › Link Options is enabled with no links. OLV1
is skipped; OLE stays as decided (drawn from the stored presentation, kept
byte for byte).

## 13. The chart model and generator (DO.6, DO.7; 2026-11-07)

Code: `amipro_pictures::chart` (record, data text) and
`amipro_pictures::chart_gen` (objects); tests in both, in
`crates/pictures/tests/chart_oracle.rs` (ignored; they read the corpus and the
CHV2 files) and `crates/editing/tests/charts.rs` (SAM, native v3, the twin).
Evidence letters as in 12: V = the record or objects read from a saved file,
F = fitted to several of them, G = a guess.

### 13.1 The record (V, 104 of 104 corpus charts and 18 of 18 CHV2 charts read and write back equal)

`Chart` keeps every word of the type-15 body: counts A (columns, the
categories) and B (rows, the series), type (+0x1c), variation (+0x1e), legend
(+0x20), the word at +0x22 (1 in all 122 records), grid (+0x24), oblique depth
(+0x26), perspective depth (+0x28), A x B float32 values, A x B flag bytes,
B row labels then A column labels (20 bytes each, NUL padded; the bytes after
the NUL are kept), the tail word, the axis minimum, maximum and step (three
doubles), the 16 colour entries (r, g, b, pattern; the pattern byte is kept
as stored), and the last word. The offsets 0x14 to 0x1a are derived
(0x36, 0x36 + 4AB, 0, 0x36 + 5AB); a record where they are not, or whose
length is not 134 + 5AB + 20(A + B), is not parsed and stays an opaque
`Shape::Other` (so an unknown version is kept as stored). Type codes, as
12.1: 0 column, 1 line (area when the variation is 6), 2 pie, 3 bar,
4 line and picture, 5 picture.
Corpus: **104 charts read** (the runbook's 107 includes three in files the
reader cannot open: one password-protected, one without a document body, and
Word Pro files), **6 distinct records** here (the seventh is in an unreadable
file); lengths 204 (A = 1, B = 2 or A = 2, B = 1), 254 (A = 4, B = 1), 279
(A = 5, B = 1). Observed in the 122 records: legend, grid, depths and
the last word as in 12.1; the corpus pies carry perspective depth 50 with
oblique 0 (they are 3D pies); **the tail word was 0 in all 122**.
Open: where Automatic is stored (nowhere found: `axis_is_automatic` compares
the stored axis with the one the data gives); the meaning of the last word's
bits (copied from the icon the dialog was last given: the stacked column in
CHV2C-2 kept 0x1000 from the clustered column before it, so that chart is
drawn clustered while its record says stacked: OK-DO7-1).

### 13.2 The data text (H from `1018:0078`; no VM check, CHV4 not run)

`parse_data`: tab separator if the text has a tab, else space; empty
tokens skipped; a token is a number when it has a digit and only digits, the
decimal symbol, the thousands symbol, the currency string, `+ - e E` (the
last four as the notes say); the first non-number of a line is the row
label, a line with no numbers makes the column labels (the first only);
labels cut at 19 characters, spaces stored as `_` (and shown as spaces);
A the most numbers on a line, B the lines with numbers, short lines padded
with zeros; over 5000 cells is `TooMuch` (message 4013), no numbers is
`NoData` (4011); beyond-float values clamp, values below the smallest normal
float become 0. `Chart::data_text` is the inverse (a tab then the column
labels, then each series as label and values; `format_value` is the shortest
text that reads back as the same `f32`); the parse of the text of a chart
gives its values and labels back (unit test). OK-DO6-1: an empty token
(two tabs together) is skipped, not read as a zero; OK-DO6-2: the number
format comes from a `DataFormat` the caller fills (the app's locale).

### 13.3 Generation: formula against Ami Pro's objects

`generate(chart, metrics)` gives the objects (every one flag 4, the record
last with the computed axis); `Metrics` is the drawing rectangle (4103 x 3395
for CHV2's 4608 x 3888 frame) and the label height (233). `compare` matches
two streams object by object (a group counts as an object with its children;
a text by its origin, because Ami Pro's text rectangles are padded by
amounts no rule fitted: +9, +18, -9 twips on three charts) and counts the
coordinates within a tolerance. Results on the 18 CHV2 charts (tolerance 3
twips; "n/m" is aligned objects in order with the same kind and point count
over Ami Pro's objects):

| Chart (CHV2) | Counts by kind | Within 3 twips | Worst | Mean | Status |
|---|---|---|---|---|---|
| column, grid, legend | equal (32) | 32 of 32 | 2 | 0.3 | confirmed |
| stacked column 3D oblique, legend | equal (49) | 46 of 49 | 14 | 0.9 | confirmed |
| clustered bar | equal (19) | 18 of 19 | 5 | 0.9 | confirmed |
| line (ribbons), grid, legend | equal (30) | 29 of 30 | 4 | 0.6 | ribbon shape from two segments |
| area 3D oblique | equal (34) | 33 of 34 | 4 | 0.5 | confirmed |
| line and picture (2206) | equal (36) | 35 of 36 | 4 | 0.4 | confirmed |
| 2207 (marker follows the category) | equal (36) | 34 of 36 | 4 | 0.5 | confirmed |
| pie | equal (9) | 9 of 9 | 2 | 0.5 | confirmed |
| picture, stacked picture | equal (32, 30) | 32 of 32, 30 of 30 | 3 | 0.3, 0.6 | confirmed |
| column 3D oblique, no legend | equal (44) | 32 of 44 | 6 | 1.5 | confirmed |
| area (flat) | equal (21) | 21 of 21 | 2 | 0.2 | confirmed |
| stacked column, small frame (2880 x 2160) | equal (23) | 23 of 23 | 3 | 0.5 | confirmed |
| clustered column, small frame | equal (19) | 6 of 19 | 13 | 4.3 | frame rule is a line through two frames (G) |
| stacked bar 3D perspective | equal (43) | 13 of 43 | 189 | 53.5 | perspective drawn as oblique (G) |
| expanded pie 3D | equal (14) | 9 of 11 aligned | 2218 | 218.9 | 3D pie is a guess |
| expanded pie flat | equal (2) | not aligned | | | its slices are offset by a rule not fitted |
| stacked column 3D click missed | no | | | | record disagrees with the objects (OK-DO7-1) |

Over the 14 confirmed charts: **347 of 380 objects within 3 twips**, every one
in Ami Pro's order. The 3D column of the user's corpus (12, 23, 5, 14) keeps
11.6.2's checks (bar width 0.7, skew 5 %, shades 100, 90, 50 %, axis 0 / 30 /
10 for a 807-twip plot) in `the_corpus_3d_column_formulas_hold`. Against
the **corpus charts regenerated from their own records** (10 distinct
drawings; the rectangle and label size taken from each drawing's own extent):
the object counts by kind equal for all 10 (the 3D columns 26, the 3D bars 44
and 56, the perspective pies 7 and 8), but **coordinates do not**: the
corpus was made with another label font ("Tms Rmn", boxes 192 x 206) and its
3D plot starts at a different top (223 against 144), so only the
formulas of 11.6.2 (bar width, skew, shades, axis) and the structure carry
over; mean difference 17 twips (3D column) to 74 twips (3D bar). The ink
measure against Ami Pro's screen images was not used: the stored objects are
the better oracle.

Rules found on the way (V unless marked):
- Layout: margin 144; category labels' top at H - 144 - 233 - 2 (-1 for
  area, 0 in 3D); the axis 116 above them; the plot top 144 + 116 - 4; the
  data height 2644 (2627 in oblique 3D, the box over 1.05).
- **Data heights truncate** (`y = base - trunc(v * H / range)`); ticks are
  placed at `base + 0.3 - v * (H - 4) / range` and floored; the labels are
  `floor(tick - 116.25)` above them (F: all seven ticks of the CHV2 column).
- Left edge: the widest value label + 111 (grid, column and picture), 122
  (column without grid), 92 (lines), 108 (area), 102 in 3D with a legend
  and 112 without (G; the widest label measured two twips smaller in 3D).
  Tick marks 50 to 54 long outside the axis; none with a grid.
- Right edge: the legend's left minus 144, or the margin; lines and areas end
  half the last label inside (rect widths 0.97 and +7); in 3D lines and areas
  the outer box ends 7 short.
- **Lines are ribbons**: a plain line chart draws each segment as a
  quadrilateral 50 twips thick (perpendicular half-thickness 25), the ends of
  the line cut upright (30 twips vertical), the joints mitred; the line-and-
  picture types draw 10-twip lines and markers (the last markers follow the
  last segment of their series); **areas are three polygons in 3D** (front,
  top 90 %, side 50 %) and one in 2D, the label before the polygons of the
  interval it ends.
- Horizontal 3D charts list the floor first and wind floor and back wall the
  other way; their grid lines run bottom to top; a bar group lists the front,
  the right side, then the top.
- No category names: no labels, and the plot reaches the bottom margin
  (corpus). A chart whose series have no names has a legend of swatches.
- 2207's legend shows the first marker for every series.

### 13.4 What the app slice needs, and the choices (OK candidates)

- OK-DO7-1: the record's variation and last word are copied from the dialog's
  state, not decided by the chart, so a record can disagree with its objects
  (CHV2C-2). The generator trusts the variation (0 = stacked for columns,
  bars and pictures; 6 = area for lines) and writes `Chart::set_icon`'s words
  (13.1); a document read from Ami Pro is regenerated only when the user
  changes something.
- OK-DO7-2: the frame-to-rectangle rule is `Metrics::for_frame`, a line
  through the two frames measured; the label height 233, or 200 under 3000
  twips of frame height.
- OK-DO7-3: perspective is drawn as oblique with the 0.87 factor on the axis
  length that gives CHV2's step of 20 (G); 3D pies, the expanded pie, legends
  of three or more series, negative values, and Flip data are best guesses.
  `chart_gen::confidence(chart)` says which charts are guesses so the panel
  can say so; `CONFIRMED` lists the families.
- OK-DO7-4: a series without a name has a legend swatch and no text; a
  category without a name has no label.
- Not done: horizontal-bar legends for three series, picture shapes from
  `AmiChart.ini`, the 3D pie's rim, scatter. Ami Pro taking the objects the
  app writes is CHV1 (the twin `CHT1.SAM`: `AMIPRO_TWIN_OUT=DIR cargo test -p
  amipro-editing --test charts` writes it: data 12, 23, 5, 14 as a 3D
  clustered column with a grid, in a 3.2 by 2.7 inch frame).
- For the UI: `chart_of(&drawing)` finds the chart; `Chart::set_data(
  parse_data(text)?)`, `data_text`, `set_icon`, `flip`, the public fields for
  the options; then `generate_drawing(&chart, &Metrics::for_frame(w, h))` and
  `Command::SetDrawing` (the frame keeps the `.sct` extension). Axis Inc, Min
  and Max are typed into `chart.axis` (generation recomputes it, so the UI
  must keep manual values apart and write them after generating, or add an
  override: not built).
- Commands: `AMIPRO_STYLES=<corpus STYLES> cargo test --offline -p
  amipro-pictures --test chart_oracle -- --ignored --nocapture` (the corpus
  census, the CHV2 table, the corpus table; `CHART_DIFF=n` lists the objects
  of chart n); `cargo test --offline -p amipro-editing --test charts`.

## 14. VM results (CHV1, 2026-11-07)

One sitting in the `Windows XP` guest (Ami Pro 3.1). The app's twins were
written by `AMIPRO_TWIN_OUT=DIR cargo test -p amipro-editing --test charts`
(seven files: `CHT1` the 12/23/5/14 3D column; `CHT2A-C` CHV2's 18 charts
with the same records; `CHT3` three series; `CHT4` negative values; `CHT5` four
frame sizes) and opened from `L:\IN`. In each document every chart was
selected and Tools › Charting › OK pressed (Ami Pro regenerates the objects
from the record), the file saved as `CHTnR.SAM` (`CHT6R`: Flip data ticked
on three charts; `CHT7R`: one chart regenerated at the 91 % view instead of
Full Page). Files and screens are in `vm-lab/OUT/objects/` and `screens/chv1_*`,
`CHT*`. The ignored test `chv1_ami_pro_regenerations_against_the_app` reads
the saved files and prints the table below (`CHV1_DIFF=all` or `CHT3:1` lists
the objects side by side). Evidence: V = read from a saved file, F = fitted.

### 14.1 Does Ami Pro accept what the app writes? (V)

- **Only with the picture type `.sdw`.** The twin as the app writes it (the
  frame's picture type `.sct`, the chart marker the app keeps in memory) opens
  with "Cannot find a library for frame. Frame contents will not be
  displayed." and a crossed box. The same bytes with `.sct` replaced by
  `.sdw` in the three places (the frame record's `[isd]`, its trailing name,
  and `[Embedded]`; the same length) open and **display from the stored
  objects**, as drawn: all 35 twin charts (every type, 3D, pies, pictures,
  negatives, a 1.5 inch frame) showed, nothing reported as damaged. Ami Pro
  itself writes `.sdw` for its own charts (CHV2) and the corpus has `.sct`:
  whether the corpus files show in this guest was not tested (the two
  corpus files tried were password-protected). **Fix:** the SAM writer must
  write `.sdw` for a chart frame (keep `.sct` only as the in-memory marker).
- **The Charting dialog opens with the right state (V):** Data… shows
  `12 23 5 14`; Legend, Grid, 3D, Depth 50 and the axis (Automatic, 0 / 25 /
  5) are as written; Flip data works and was read back. The dialog's type
  icon did not show a highlight in the screenshots (not checked further).
- **OK regenerates the objects** (Ami Pro adds its own 700-word driver state to
  `[isd]` and the screen image to `[Embedded]`; the file grows from 5 KB to
  240 KB for one chart). Of the 38 charts regenerated:
  **the saved data record equals ours byte for byte in 30**. The 8 others:
  six negative-value charts (the axis minimum, 14.2), the 2207 chart (two
  bytes, `+0x0e` and the last word's low byte, 03 to 00: Ami Pro writes
  `0x2000` where CHV2's saved record held `0x2003`), and CHV2C-2 (14.3).
- **Object counts by kind equal ours in 30 of 38**; unequal: CHV2C-2 (14.3),
  the six negative charts (one more line, two more texts: 14.2) and the 3D
  area (one polygon fewer, 14.5). Flip data: record equal, counts equal.

### 14.2 What Ami Pro changed and the corrections to make (V, F)

Numbers are twips at the rectangle Ami Pro used. After fitting each chart's
rectangle (14.4), the 2D column, stacked column, bar, picture, area and line
families match within 3 twips for 24 to 40 of 30 to 49 objects (the misses are
the legend box and the right plot edge, below).

1. **Frame to rectangle:** the rectangle is the frame's picture area, not a
   line through two frames (14.4).
2. **Negative values (CHT4, all six charts):** the minimum is the next step
   **strictly below** the lowest value, as the maximum is above the highest
   (-20 gives -30; data -20..30 gives -30 / 40 / 10, 7 intervals); a unit is
   `H / (max - min)` (40.6 twips for H 2844), the zero line at
   `base - (0 - min) * unit` (1886). Stream order: vertical axis, the
   bottom axis (at the minimum), the ticks above zero from the lowest up
   (grid line then label), then **the "0" label alone** (right-aligned like
   the others, origin x 316 for one digit; no grid line), the ticks below zero
   from -10 down (line, label), the categories (label at the bottom, below
   the minimum; then its bars from the zero line), and **the zero line last**
   (after the bars). Ours omits the zero label and the zero line and uses the
   lowest value as the minimum.
3. **Automatic axis step does not shrink in tall plots:** plot height 2644
   gives 0/35/5 (CHV2), 3262 gives 0/35/5, 4700 gives 0/35/5, 1867 gives
   0/40/10 (data 5..30); ours picks step 2 at 4700. Consistent with a minimum
   tick spacing of about 370 twips (1.6 label heights): step 2 at 4700 would
   be 294. Rule to try: the smallest of 1, 2, 5, 10, ... x 10^n whose spacing is
   at least 370.
4. **Perspective (stacked bar 3D, depth 50), not oblique:** each face is
   projected toward a vanishing point, `back = VP + 0.884 * (front - VP)`
   with VP about (3200, 144) at rectangle 4305 x 3600 (fitted on three bars:
   VPx 3193 to 3226, VPy 114 to 183; the factor from the top faces' width
   ratio 343/388). Bar front faces equal the oblique ones. The first bar's
   depth vector is (262, -240), the second's (215, -126), the third's (168,
   -12). Ours (oblique, 110 x 130) is off by up to 265 twips.
5. **3D area:** the formulas hold within 3 twips; **a band's top face is
   dropped when it would be degenerate** (the surface slope equals the skew
   slope: West over Feb to Mar, 0.80): a group of 2 polygons, not 3.
6. **Legend of three or more series (CHT3, CHT6; within 2 to 5 twips):**
   confirmed, with the box height 1355 for 3 rows (ours 1350: pitch 406.5),
   the box 4 wider for "North". **The legend's horizontal sizes scale with
   the picture area:** at area 4320 the swatch is 204 to 205 wide (CHV2's 4112
   gave 193), swatch to text 204 to 206 (194), box 979 (Full Page view) and
   986 (91 % view) wide (928): about x 1.05 to 1.06; heights do not change.
   Vertical placement unchanged. **A clustered horizontal bar chart lists
   its series in reverse** (3 series: North first, East last; the 2 series
   stacked bar of CHT2A did not reverse).
7. **Pie, three slices:** the stream lists the slices in the order 1, 3, 2
   (two slices: 1, 2). Arc points as before (the plain pie matches within 2).
   **Expanded pie, flat:** the slice polygons have 49 and 74 points (ours 53
   and 78), starting at (3659, 1663) and (847, 1035) (ours (3719, 1656) and
   (937, 911)): the slices are offset by a smaller amount along the mid-angle
   (about 60 twips less) and the arc step is coarser (about 0.054 rad). **3D
   expanded pie:** slice top polygons 51 and 73 points (ours 53 and 78), the
   rim 126 (128); the thickness is 322 twips (ours 260); the two cut sides are
   the first group's other children; every coordinate is 100 to 250 twips off,
   so the 3D pie stays a guess (not refitted).
8. **Plain line (ribbons):** matches (24 of 30 within 3; only the legend box
   differs). **Flip data:** `Chart::flip()` gives the same record as Ami Pro
   (byte for byte) and the same objects; the legend lists the former columns
   (Jan, Feb, Mar), the categories are East and West.
9. **Stacked is a bit of the record's last word, not the variation:** CHV2C-2
   (variation 0, last word 0x1000, "stacked column") is regenerated by Ami Pro
   as a clustered column (axis 0 / 35 / 5, 9 texts, 9 lines), while 0x0100 gives
   stacked. The generator trusts the variation (OK-DO7-1): it should read
   0x0100 of the last word.
10. **Small and other frames:** the 2.0 x 1.5 inch column (14.4) matches 7 of
    19 only (label height 200 and legend scale not fitted); the 4.0 x 4.0 inch
    legend column matches 26 of 32 within 3.

### 14.3 CHV2C-2 (variation 0, last word 0x1000)

Ami Pro regenerated it as clustered (see 14.2 item 9); no damage.

### 14.4 The frame-to-rectangle rule (V, F; replaces the line of `Metrics::for_frame`)

Frames the app writes have the picture area `[frmlay]` words 1440 and 5760
(= left + 144 and right - 144) and a frame without a line. Ami Pro's rectangle
width is `[isd]` word 7 of the regenerated frame, its height the lowest
object's bottom plus 144:

| Frame (twips) | Picture area | Rectangle width (`[isd]` word 7) | Rectangle height |
|---|---|---|---|
| 4608 x 3888 | 4320 x 3600 | 4305 (2D), 4334 (3D no legend) | 3603 (2D), 3592 (3D) |
| 2880 x 2160 | 2592 x 1872 | 2597 | 1871 |
| 8640 x 5760 | 8352 x 5472 | 8353 | 5475 |
| 5760 x 2880 | 5472 x 2592 | 5475 | 2597 |
| 4320 x 4320 | 4032 x 4032 | 4011 | 4024 |
| 2160 x 4320 | 1872 x 4032 | 1793 | 4024 |

CHV2's frames (made in Ami Pro, with a frame line: `[frm]` has `100 100`) have
a picture area inset 248 (4608: 4112 wide) and 201 (2880: 2478 wide) and
rectangles 4103 x 3395 and 2472 x 1763, that is, area less 9 or 6 (and the
height less 9 to 10). **Rule: the rectangle is the picture area width (+14 to
-21, -79 with a legend in a 1.3 inch frame) by its height (-9 to +3)**; the
generator must be given the area (frame less 288 for the app's frames, or
the `[frmlay]` pair for a frame from Ami Pro), not the frame. The view
(Full Page against 91 %) changed the legend by 1 % and nothing else.

### 14.5 Unknown or not tested

The Charting dialog's type icon highlight; Pictures… (no picture chart
was given pictures); the corpus's `.sct` charts in this guest; the 3D
pie's thickness rule, the arc step of the expanded pie and the legend's
x scale as a formula of the area (three points only); the row order of
a clustered bar legend with two series; picture charts and lines with three
or more series in 3D; Automatic off (typed Inc, Min, Max); a chart with
negative values in 3D line (the other negative charts have the same fixes).

### 14.6 DO.7b: the writer and the corrections (2026-11-07; V, F)

**The writer.** `amipro_formats` writes `.sdw` as a chart frame's picture type
in `[isd]` and in `[Embedded]` (`frames::stored_extension`; the in-memory
marker stays `.sct`, the loss check compares the written type). Ami Pro's own
file (CHV2A) has: `[isd]` `.X3`, `.sdw`, `.X3` (the stored name equals the
frame's), flags 1 (Original, no aspect), unit 1, then the saved state (a count,
704 words) and the driver `.sct`; its `[Embedded]` entries read `n .sdw ...`.
The app now writes the same: `.sdw`, the stored name, flags 1 (a new chart or
drawing frame is Original, `pictures::fresh`), unit 1, an empty state (`0`) and
the driver `.sct`. Proof: `a_chart_frame_is_written_as_ami_pro_writes_its_own`
(`crates/editing/tests/charts.rs`) compares those fields with CHV2A when the
reference folder is present. The size and the saved state's words differ (Ami
Pro adds its own on regeneration); the all-`.sdw` files that opened in CHV1 had
`.sdw` as the driver too, the app now keeps `.sct` there as Ami Pro does (not
tried in the guest: first thing to check at the next sitting). Round trips: the
user and ISO audits equal the baseline; the VM folder has 28 more files than at
the baseline (the CHT*R files) and all read, write and read back equal.

**The generator**, against the 39 charts Ami Pro regenerated (CHT1, 2A-C, 3-5,
three of 6 and one of 7). "Within 3" is objects of Ami Pro's stream whose
coordinates all lie within 3 twips of ours, in stream order; "counts" is charts
whose object counts by kind equal Ami Pro's. Measured by the ignored test
`chv1_summary_before_and_after` (`Metrics::for_frame` before, the new
`Metrics::for_picture_area(frame - 288)` after). Cumulative, as each part was
added:

| Step | Counts equal | Within 3 of 1249 |
|---|---|---|
| Start (rectangle of `for_frame`, old rules) | 31 of 39 | 100 |
| 1 rectangle = picture area (table of six sizes, 14.4) | 29 | 327 |
| 6 legend: x scaled to fill a box of 0.2266 of the width, 3+ rows (pitch 406.5, box 411 a row), clustered bar from the bottom up | 30 | 342 |
| 1 again: widths 4304, 4011, 5475... by interpolation | 31 | 712 |
| 4 perspective (horizontal bar): back = V + 0.8802 (front - V), V = (left + 1.102 w, top + 0.0134 h) fitted on 18 corner pairs (within 1 twip); the plane is the flat bar's; category labels x 0.935 with a legend | 32 | 761 |
| 5 area band without a top face (slope gap < 0.5 twip), 7 three pies in the order 1, 3, 2, thickness x 1.24 | 33 | 763 |
| 3 axis step floor (spacing >= 296 twips, plots >= 1500 twips only) | 36 | 823 |
| 2 negatives: minimum strictly below, tick order, zero label and line (2D, 3D, bar, line, area), hyphen 304 | 36 | 853 |
| 2 and 3 settled (floor 296 not 326; area base at zero) | 39 | 881 |
| 9 stacked is bit 0x0100 of the last word (in all of the above) | included | included |
| End | 39 of 39 | 880 of 1249 |

Per file at the end (within 3 of Ami Pro's objects): CHT2A 178 of 206, CHT2B
146 of 157, CHT2C 111 of 134, CHT3 177 of 217, CHT4 (negatives) 144 of 234,
CHT6 (flip) 105 of 115, CHT1 5 of 32, CHT5 (four sizes) 10 of 122, CHT7 (91 %
view) 4 of 32. Every chart confirmed before still passes (the CHV2 oracle:
`chart_oracle`, 18 of 18 counts equal, the corpus table unchanged).

New facts: the legend box is a share of the picture width whatever the label
("West" 979, "North" 980 at 4320), so its parts scale to fill it (x 1.05 for
two rows, 1.0 for "North"); a clustered horizontal bar's legend is drawn from
the bottom up for any series count (2 rows too); a negative axis writes the zero
line last and, in 3D, the back wall's zero line first; the hyphen is 62 twips
(304 per 1000 em); with a legend the horizontal bar's category labels are 0.935
as wide. The stacked-vs-variation fix is in the generator, not in the record
(the record keeps what the dialog set); `Chart::set_icon` already pairs stacked
with 0x0100.

**Still unconfirmed:** the 2207 record Ami Pro rewrites (variation 0, 0x2000 at
+0x0e and the last word); the expanded pie (the arcs lie on one circle centred
(2152, 1801), R 1513, from 0.0914 rad, step 0.0499, the apex of each slice
moved out by about 150 twips; not implemented, ours has 53 and 78 points to
Ami Pro's 49 and 74); the 3D pies; perspective for column, line and area;
CHT1 (3D column, grid, no legend: Ami Pro's rectangle 4331 not 4304 and the
labels 15 twips higher); the 2.0 x 1.5 inch frame (text 200: labels 294
apart); the four sizes of CHT5 (legend and label sizes at 8.7 and 1.3 inches);
the 91 % view; negative 3D stacked bars within 3 twips (25 of 61); the axis
floor is a threshold fitted between 294 and 299 twips; the horizontal-bar
label squeeze rests on four charts with a legend.

## 15. DO.9 results: the equation editor (2026-11-07)

Code: `amipro_pictures::equation` (`parse`, `write`, `layout`, `metrics`,
`symbols`, `Stored`), `Command::SetEquation`, `equation_panel.rs`,
`equation_commands.rs`. Oracle: `crates/pictures/tests/equation_oracle.rs`
(ignored; commands in `docs/TEST_COMMANDS.md`).

### 15.1 Stored form and syntax (V: all 27 equations of EQV1A/B)

- The 27 objects read as two lines (`Stored::read`) and write back **byte
  equal**; the text of each is Ami Pro's normal form: `write(parse(text))` is
  the identity for all 27, and the 27 inputs (the IN files) normalise to the
  27 Save As texts (`import_tex`, the unit test `ami_pro_normal_form_...`).
- The rule for plain text in an import: with no `$` in the text, each
  construct (a control word that builds math, a script, a `\left`...`\right`)
  is a math run of its own and the plain characters around it stay text, so
  `x_{i}^{2}` is `x$_i^2$`, `\sum_{i=1}^{n} i` is `$\sum_{i=1}^n$ i`, and a
  `{\BF x}` group becomes the plain `\BF x`. A control word swallows the
  spaces after it.
- The picture's words: width in the scaling, height as a negative word in the
  scaling, **the baseline (ascent) as a negative word in the pan's y**
  (`placed_pan` ignores it for drawing). New pictures write them as the VM's
  do: `.tex .X1 1 1 0 0 W -H 100 0 0 .tex 0 -B 0`.
- Error classes returned as `EquationError { code, message, position }`:
  5100 (nesting past 40, or more than 20,000 nodes), 5102 (a matrix of more
  than 10 rows or columns), 5105 (a text past 8,000 bytes, or an equation
  wider or taller than the 22 inch frame limit), 5106 (every other parse
  error: unknown command, missing argument, unbalanced braces, misplaced
  `\right` or `\endmatrix`). 5101, 5103, 5104, 5107 and 5108 are not raised
  (their conditions are not known: 5107 is a text-attributes limit of the
  character dialog).

### 15.2 Layout and the fit to Ami Pro's sizes (F, L)

The engine lays out by TeX's rules in twips (styles display, text, script and
scriptscript at 100, 100, 70 and 50 % as `TextToScriptsRatio`; fraction bar
on the axis with the numerator and denominator shifted by a gap; radical
sign and bar; scripts shifted and padded; big operators with limits at the
side or above and below; matrices by column width and row height;
delimiters drawn to the height they enclose; over and under decorations).
Glyph advances are the Times family's, so no font file is needed and the
result is the same on every machine (checked by a determinism test over
4,000 odd inputs, none of which panics).

Constants fitted to the table (`Params::default`; everything else is TeX's):
the line box is 0.80 em above and 0.2708 em below the baseline (192 and 65
twips at 12 pt) and the natural height never falls below it; 44 twips of
padding across; italic `x` 0.433 em, `y` 0.52, `z` 0.56 (times 0.9, Ami
Pro's math italic is narrower than Times Italic); a text space 0.49 em;
axis 0.29 em (matrix baseline = half the height + 70); fraction gaps 0.13 em
above and 0.185 below the bar, side space 0.215 em (0.1 inside a script
style); superscript raised 0.41 em (0.49 with a subscript), subscript
dropped 0.2, a script adds its width plus 0.1375 em (0.2375 with both);
big-operator scripts add 0.43 em and sit 0.36/0.2 em (sum) or 0.52/0.375
(integral) from the baseline; limits stack only for a sum-like operator
whose limits are wider than it; radical extra width 1.12 em, an index at
script size; parentheses 0.42 em wide, brackets 0.31, a binomial's 0.11; a
matrix cell has 0.93 em of space across, columns 0.75 em apart, 0.5 em at the
sides, rows 0.083 em apart and 0.167 em above and below.

Natural sizes against the 27 (Ami Pro in the second figure; width, height,
baseline; error in percent of width and of height):

| Text | W ours/Ami | H ours/Ami | B ours/Ami | W % | H % |
|---|---|---|---|---|---|
| `x` | 162/162 | 257/257 | 192/192 | 0.0 | 0.0 |
| `$x$` | 138/138 | 257/257 | 192/192 | 0.0 | 0.0 |
| `$xyz$` | 371/371 | 257/257 | 192/192 | 0.0 | 0.0 |
| `$x^2$` | 253/255 | 268/268 | 203/203 | -0.8 | 0.0 |
| `$x_2$` | 253/255 | 257/257 | 192/192 | -0.8 | 0.0 |
| `$x_2^2$` | 277/279 | 287/286 | 222/221 | -0.7 | +0.3 |
| `x$_i^2$` | 302/303 | 287/286 | 222/221 | -0.3 | +0.3 |
| `$\frac ab$` | 223/209 | 288/287 | 192/192 | +6.7 | +0.3 |
| `$\binom nk$` | 276/262 | 288/287 | 192/192 | +5.3 | +0.3 |
| `$\frac xx$` | 213/228 | 257/257 | 192/192 | -6.6 | 0.0 |
| `$\frac{xxx}x$` | 344/346 | 257/257 | 192/192 | -0.6 | 0.0 |
| `$\frac{\frac xx}x$` | 228/225 | 335/332 | 270/267 | +1.3 | +0.9 |
| `$\sqrt{x}$` | 416/423 | 257/258 | 192/193 | -1.7 | -0.4 |
| `$\sqrt{xxx}$` | 603/610 | 257/257 | 192/192 | -1.1 | 0.0 |
| `$\root{3} \of{x}$` | 499/504 | 257/259 | 192/192 | -1.0 | -0.8 |
| `$\sum $` | 173/172 | 257/257 | 192/192 | +0.6 | 0.0 |
| `$\sum_a^b$` | 352/351 | 266/266 | 201/201 | +0.3 | 0.0 |
| `$\sum_{i=1}^n$ i` | 539/555 | 444/447 | 264/268 | -2.9 | -0.7 |
| `$\int $` | 89/90 | 257/257 | 192/192 | -1.1 | 0.0 |
| `$\int_a^b$` | 268/269 | 331/330 | 239/240 | -0.4 | +0.3 |
| `$\int_0^1$ f(x) dx` | 1101/1101 | 321/320 | 229/230 | 0.0 | +0.3 |
| `$\matrix x & x \\ x & x\endmatrix $` | 1097/1098 | 614/614 | 377/377 | -0.1 | 0.0 |
| `$\matrix a & b \\ c & d\endmatrix $` | 1126/1126 | 614/614 | 377/377 | 0.0 | 0.0 |
| `$\left[ x\right] $` | 286/288 | 257/257 | 192/192 | -0.7 | 0.0 |
| `$\left( x\right) $` | 339/337 | 257/257 | 192/192 | +0.6 | 0.0 |
| `$\overline{x}$ $\underbrace{y}$` | 444/443 | 401/408 | 192/192 | +0.2 | -1.7 |
| `$\text{abc}$ \BF x` | 613/738 | 257/274 | 192/202 | -16.9 | -6.2 |

**Mean error 1.9 % of width and 0.5 % of height; worst 16.9 % and 6.2 %**
(both the `\text{abc}` equation, where a bold switch follows text in math;
nothing in the notes says why Ami Pro's text is wider and taller). Beyond 5 %:
`\frac ab` (+6.7), `\frac xx` (-6.6) and `\binom nk` (+5.3): two single
glyph fractions that no one fraction side space fits together (the numbers
imply 0.185 em for `ab` and 0.247 em for `xx`, so Ami Pro's glyph widths at
script size differ from the Times widths used), the `\underbrace` height (401 against 408, a guess) and `\text{abc}`. These are OK candidates
(OK-DO9-1). The widths beyond these 27 (other glyphs, other faces and sizes,
display style, `\limits` above and below, `\left` with side pairs other than
`( )` and `[ ]`, accents, `\overset`) are **not measured**: they follow
TeX's rules with the constants above and are a guess until a second VM
sitting (EQV3: sizes of 20 to 30 more equations, and the repaint of
EQT2.SAM).

### 15.3 What the app does and what still needs Ami Pro

- Tools › Equations…, the in-place card and the palette, Import and Save As
  Equation, Copy as LaTeX and the LaTeX paste: `UI_SPEC.md` §42.5 notes.
- Equation frames the app makes have no line, shadow or margin and are
  exactly the natural size (Ami Pro's `[frm]` has side mask 0, no shadow).
- Not yet checked against Ami Pro (the EQV1 twin, `EQT2.SAM` with eight
  equations, `AMIPRO_TWIN_OUT=DIR cargo test -p amipro-editing --test
  equations`): that Ami Pro opens a frame with no screen image after the
  text (the directory entry's rendering offset and length written as 0), that
  its repaint of the same text is the size we computed (W, H, B are
  recomputed by it, so a mismatch shows as a different frame size once the
  equation is opened), the frame record's trailing `W B` words of a new
  frame, `\format`, `\Sb` and `\Sp` and the `\limits` spacing the writer
  chooses (`\sum \limits_a^b`), and the wider text and symbols (Greek, the
  radical in Symbol).

## 16. DO.11 results: twins in Ami Pro 3.1 and measures (2026-11-07)

One sitting in the `Windows XP` guest (Ami Pro 3.1), files from `L:\IN`, screens
in `vm-lab/screens/o_*.png` and Ami Pro's saved files in `vm-lab/DO11/` (both
outside Git); the guest was shut down afterwards, `Windows11` untouched.

### 16.1 Twins (V: opened and looked at in Ami Pro)

| Twin | Written by | Result |
|---|---|---|
| `CHT1.SAM` (chart, 3D column 12/23/5/14) | `charts.rs`, `AMIPRO_TWIN_OUT` | **Opens and displays the stored objects** (no library error: the frame record's picture type is `.sdw` in `[isd]` and `[Embedded]`; the `.sct` that remains in the file is the one word after the state words, the chart driver name, and Ami Pro accepts it). Charting dialog opens on the record: Grid, 3D, Depth 50, Axis Automatic 0/25/5, Data. OK regenerates; the bars moved by about 1 to 2 pixels (the red bar 479 to 481 at 100 %), the file grew from 4.6 KB to 240 KB (Ami Pro's own objects and screen image, as in CHV1). Save worked. |
| `EQT2.SAM` (8 equations, no screen image after the text) | `equations.rs` | **Opens**; every frame is repainted by Ami Pro from the text alone, in the same red/black look as Ami Pro's own EQV1A. Tools > Equations opens the Ami Pro editor on the equation (the envelope is accepted). Saving from the editor added Ami Pro's screen images (5.5 KB to 16 KB) and recomputed W, H and B (table below). |
| `AIMG1.SAM` (gray TIFF as `.tgf`, brightness 70, contrast 80, edge 40, smoothing 30, invert, Posterize) | scenario `a_gray_tiff_is_adjusted...` (its `Gray.SAM`) | **Opens**, draws the inverted ramp, and enters Image Processing mode (the Image menu appears) on selecting the frame: the `[isd]` blob is taken as Ami Pro's own. Not compared pixel for pixel (the host's drawing of the ramp was not captured). |
| `OLR1.SAM`, `OLR2.SAM` (two user-corpus invoices with a WordArt object, read and written by `sam_document`) and `OLO1.SAM` (the corpus original) | temporary read/write test | **OLR1 opens and looks identical to the original OLO1** (WordArt "Invoice" heading, table, text): the OLE stream and presentation survive our writer byte for byte. OLR2 was written but not opened (same path). |

### 16.2 Equation size: Ami Pro's recomputed W, H, B against the ones the app wrote

| Text | App W, H, B | Ami Pro W, H, B | dW |
|---|---|---|---|
| `\frac ab` | 223, 288, 192 | 209, 287, 192 | -6.3 % |
| `\sqrt{x}` | 416, 257, 192 | 370, 257, 192 | -11.1 % |
| `x_2^2` | 277, 287, 222 | 265, 286, 222 | -4.3 % |
| `\sum_a^b` | 352, 266, 201 | 351, 278, 201 | -0.3 % (dH +12) |
| `\int_0^1` f(x) dx | 1101, 321, 229 | 1066, 320, 229 | -3.2 % |
| matrix 2x2 | 1126, 614, 377 | 1150, 614, 378 | +2.1 % |
| `\left[ x\right]` | 286, 257, 192 | 326, 257, 192 | +14.0 % |
| `x$_i^2$` | 302, 287, 222 | 303, 286, 222 | +0.3 % |

Heights and baselines agree within 1 twip except `\sum_a^b` (12 twips). Widths
differ up to 14 %, the known width fit error (worst 16.9 % in 15.2); the two
largest are `\sqrt{x}` (the radical is narrower in Ami Pro) and
`\left[ x\right]` (Ami Pro's bracket pair is wider than ours). A frame fitted to the
app's width clips the right of the drawing in Ami Pro until it recomputes
(the missing right bracket in the first screen), which does not damage the
file. No defect in the writer or the reader; a refit of the sqrt and
`\left`...`\right` widths is a possible follow-up (not done).

### 16.3 Typing latency, a document with a chart and an equation against without (release, -j2)

`page_surface_typing_performance_baseline` (14 pages, 344 paragraphs, README31
plus a 3.2 x 2.7 inch 4 x 3 column chart and a `\sum \frac` equation on page 1),
3 rounds interleaved (without, with / with, without / without, with), 40
samples per operation, typing then frame (view, layout, draw), medians in ms:

| Caret | Without | With | p95 without / with |
|---|---|---|---|
| paragraph 0 (page 1) | 0.98, 1.08, 1.08 | 1.15, 1.13, 1.13 | 1.0 to 1.1 / 1.2 to 1.4 |
| middle | 1.13, 1.27, 1.28 | 1.29, 1.29, 1.28 | 1.2 to 1.4 / 1.3 |
| end | 1.05, 1.17, 1.18 | 1.18, 1.19, 1.18 | 1.1 to 1.4 / 1.2 |

The objects cost about 0.05 to 0.15 ms a keystroke (view 0.45 to 0.51 ms, layout +0.03
ms); worst case 4.4 ms once (the first sample at paragraph 0, a cache fill).
Limits: the harness draws to the null-like tiny-skia path, drawing cost is
about 0.03 to 0.04 ms either way (the chart's paths are cached); a noisy shared
host.

### 16.4 Round trip and layout comparison against the DO.1 baseline

`target/objects-baseline/baseline.sh` (release, -j2) to `target/objects-baseline/do11/`:
user corpus and ISO **identical** to `start/` in every line except the
timings (round trip: 417 read, bodies 203, sections 51211 of 53352, main
text 417 of 417; ISO 160 read, sections 8204 of 8443; layout: user pages
equal 82 % (322 of 394), within one 94 %, line counts 97 %; ISO pages 44 %, lines 91 %).
The VM folder grew by 9 files since DO.5 (542 to 551: the nine CHV1 `CHT*R.SAM`
regenerations in `OUT/objects`): read back equal 544 of 544, sections 50520
of 50547 (missing 27, added 73, the same 27 and 73 as before).

