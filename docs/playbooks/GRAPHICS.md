# Graphics Playbook

Part of `docs/PHASE5_PLAYBOOK.md` (queue rows 1, 9, 18, 19, 22). Follow
its recipe and standing rules; this file says what each run covers, what
to map, and what to expect.

**Why one playbook.** In Ami Pro every picture lives in a picture frame:
an imported image, a drawing, a chart, an equation, or an OLE object. The
frame's `[isd]` record describes it, and its bytes sit after the body with
Ami Pro's own rendering beside them (`[Embedded]`). One decoding and
drawing path serves all of them, and the screen, PDF, and export share it.
Drawing is also the editor charts use: the guide's "use Drawing to modify
a chart".

**State after the pictures run (2026-10-18).** Bitmaps and metafiles are
drawn (the core crate `amipro-pictures`: `dib`, `wmf` as a display list,
`screen` for Ami Pro's screen image; the app's `pictures.rs` and
`metafile_render.rs` with tiny-skia), as are PNG, JPEG, GIF, TIFF, PCX,
and SVG; drawings, charts, and OLE objects show Ami Pro's screen image
where the file has one (16-bit as 5-6-5 or 5-5-5 by the top bit; 4 planes;
32 bits) and a labelled box otherwise (eight-bit screen images need the
writing screen's palette). `Picture` has typed scaling, pan, rotation, and
a description; Insert › Image…, Frame Properties › Picture, picture mode,
paste, and drop exist; Save As SAM converts other formats to bitmaps.
Insert › Drawing… is still Future.

**Learned for the later runs.**
- A chart is a drawing whose `[isd]` names the `.sct` driver after its
  state (108 in the user's corpus; `Picture::format`).
- A rebuilt `[isd]` must keep the kept driver state: a plain metafile's
  only record of its size is there (words 2 and 3), and drawings' and
  charts' states are 368–704 words that Ami Pro's drivers own.
- Ami Pro's screen image is a cache of the picture area at the zoom it was
  last drawn; a rebuilt record drops it (stale), so a drawing edited only
  in scaling loses its screen image in the written file until the drawing
  run draws drawings from their objects.
- Rotation turns the picture clockwise and stretches the turned box onto
  the picture's rectangle (PIRT30, PIRT90); Draw mode's own rotation is
  separate.

**Evidence to start from.**
- **Guide:** ch. 15 ("Using a picture in a frame": import, CGM and HPGL
  options, paste, delete), ch. 17 (Drawing), ch. 18 (Charting), ch. 19
  (Graphics Scaling, cropping, Image Processing), ch. 30 (Equations),
  App. B (Draw symbols).
- **Commands:** File › Import Picture… (0x8d), Frame › Graphics
  Scaling… (0x193), the Tools menu's `ISD PlaceHolder` (0x1a9), where the
  modules add Drawing, Charting, Equations, and Image Processing, and Edit
  › Paste Special… (0x24c), Paste Link (0x9d), Link Options… (0x73),
  Insert › New Object… (0x24d).
- **Modules:** `amidraw.dll`, `drawman.dll`, `amichart.dll`, `amieqn.dll`,
  `amiimage.dll`, `amiwmf.dll`, `amipaint.dll` (to identify), the graphics
  filters (`pcx.flt`, `tiff.flt`, `tiff2.flt`, `cgm.flt`, `eps.flt`,
  `hpgl.flt`, `wpg.flt`, `dxf.flt`, `pic.flt`, …), and the `g4g*.dll`
  pairs (to identify).
- **Storage** (`SAM_PAGE_FRAME_SPEC.md` §4.6, §6; `SAM_FORMAT_NOTES.md`):
  - `[isd]`: the rendering's extension (`.X3`, `.X4`, …, numbered per
    picture), the source extension, copied or linked, and about sixty
    numbers not yet decoded (scaling and cropping likely).
  - `[Embedded]`: one line per picture, `SEQ EXT OFFSET LENGTH ROFFSET
    RLENGTH [PATH]`, with the source bytes and Ami Pro's rendering, which
    begins `53 53`.
  - `[files]`: the linked paths.
  - `[chint]`, `[ehint]`, `[mf]`: kept, not decoded.
- **In the files** (pictures, user's corpus / ISO):

  | Kind | Pictures |
  |---|---|
  | Ami Pro drawings (`.SDW`) | 380 in 107 files / 5 in 3 |
  | BMP | 283 in 24 files / 18 in 6 |
  | WMF | 239 in 69 files / 13 in 2 |
  | OLE | 34 in 17 files / 0 |
  | `.G0x` (pictures of an earlier Ami Pro version whose bytes are not in the file; not charts or equations) | 16 in 3 files / 0 |
  | TIFF | 5 in 3 files / 0 |
  | CGM | 1 / 0 |

  Corrected 2026-11-06: `[chint]` and `[ehint]` are a table frame's per-cell
  hint section (4 sections in 4 files), not charts or equations. Charts:
  107 in 34 user files (7 distinct data records, driver `.sct`). Equations
  (`.tex`) and image-processing images (`.tgf`): none in any corpus. OLE:
  34 in 17 files.

## Run 1: `pictures`, queue 1 (DONE 2026-10-18: `docs/PICTURES_PROGRESS.md`, decisions P1–P8)

**Goal.** Pictures are drawn instead of grey boxes, as Ami Pro draws them.
Insert › Image… makes a picture frame at the caret (Ami Pro's default
picture frame), in a selected empty frame, or, with the caret in an empty
table cell, in that cell (T3). Graphics Scaling sizes, crops, and moves
the picture in its frame. Save As SAM writes new pictures, with a
rendering if Ami Pro needs one, so Ami Pro shows them as its own.

**Map.**
- File › Import Picture…'s handler and the filter it chooses by
  extension.
- Copy image (copied or linked).
- The frame it makes at the caret, in a frame, and in an empty cell
  (`1260:0509`, `14d0:05ec`; Import Picture is disabled in a cell with
  text, VM).
- Graphics Scaling's dialog and defaults (original size, fit in frame,
  percentage, maintain aspect ratio, rotation if any; cropping by
  dragging), and the `[isd]` numbers that store them.
- The rendering (`53 53`): which records it holds, whether Ami Pro draws
  from it or from the source, and whether Ami Pro rebuilds it when it is
  missing.

**VM fixtures.** BMP, WMF, TIFF, and PCX imported copied and linked; each
Graphics Scaling mode; a crop; a picture in an empty cell and in a
selected frame. Then the app's twins of each.

**Decoders to evaluate.**
- `image` (MIT/Apache) for BMP, TIFF, PNG, JPEG, and GIF.
- A WMF interpreter: look for a maintained crate. The format is
  published ([MS-WMF]), so an own interpreter drawing onto tiny-skia is
  the fallback.
- Ami Pro's rendering format, decoded from the decompile, for every kind
  without a decoder (drawings, charts, equations, OLE) as Ami Pro last
  drew it.
- `resvg` for SVG, if modern formats are taken.

**Decisions to expect.**
- **Which formats Insert › Image… takes.** Recommend the corpora's
  formats plus PNG, JPEG, and SVG, written to SAM in a form Ami Pro reads
  (BMP or WMF with a rendering), with the original kept in the native
  format.
- **Copy or link by default.** Recommend Copy, as Ami Pro's default.
- **Graphics Scaling in the rail** (Frame Properties › Picture) or a
  dialog. Recommend the rail.
- **What is drawn for kinds without a decoder.** Recommend Ami Pro's
  rendering, then the box.

**Better ways and logical additions to offer.**
- Paste an image from the clipboard as a picture frame.
- Drag an image file onto the page.
- Replace Picture, keeping the frame.
- Reset Scaling.
- Fit Frame to Picture.
- Crop by dragging with a modifier.
- Alt text (native only; Save As lists it).
- Sharp drawing at every zoom.

**Measures.** Pictures do not move text, so the layout comparison should
be identical. Round trip unchanged; a new picture's Save As lists
nothing. Decoding stays off the typing path: decode once per picture,
cache it, and draw scaled; time the first draw of README31's page 12 and
of the user file with the most pictures.

**Exit.** Both corpora draw every picture that has a decoder or a
rendering, with a count of what is left as boxes; Insert › Image… in all
three places; Graphics Scaling; twins open in Ami Pro; and the user's
physical check.

## Run 2: `drawing`, queue 9 (DONE 2026-10-25: `docs/DRAWING_PROGRESS.md`, decisions DW1–DW9)

**Goal.** Drawings (`.SDW`, the most common picture kind) are read as
objects and drawn from them. Draw mode edits them in their frame, as Ami
Pro does, and Ami Pro opens the app's drawings as its own.

**Map.**
- The SDW format from `amidraw.dll` and `drawman.dll`.
- The objects: line, polyline, polygon, rectangle, rounded rectangle,
  ellipse, arc, Bézier curve, freehand, and text, with line style, fill
  pattern, colour, rotation, flip, group, and layer.
- The grid and its snap, Draw settings, and import of a drawing.
- Saving a drawing as a graphic.
- How the rendering is rebuilt after an edit.
- App. B's symbols ship as `.SDW` files: proprietary art, never copied.

**Decisions to expect.**
- **Where Draw mode edits.** Recommend in place in the frame, as Ami Pro
  does, with the draw tools in the toolbar's place and the object's
  properties in the rail.
- **Whether to split the work.** If the format is large, split the run
  into showing drawings as objects, then Draw mode editing.

**Better ways and logical additions to offer.**
- Align and distribute.
- Smart guides.
- Arrowheads.
- Exact sizes and positions in the rail.
- SVG import and export.
- Copy as PNG or SVG.
- Every change one undo step on the document's history.

**Exit.** Corpus drawings drawn from their objects and equal to Ami Pro's
rendering (screenshots compared in the VM); Draw mode's gestures tested
by message and widget path; twins.

## Run 3: `charts`, queue 18

**Learned in the drawing run (2026-10-25).** A chart is a drawing stream
(`amipro_pictures::sdw`) whose look is ordinary objects with flag 4 and
whose data is one CHART record (type 15: counts, IEEE floats, bytes, and
three string lists; `DrawMan` `87f0` case 0x0f), never painted. Charts are
drawn from their objects since the drawing run; editing one means
rebuilding its objects from the data record by AmiChart's rules
(`amichart.dll`, not yet decompiled) and keeping the record byte for byte.
Draw mode's operations and Drawing Properties can serve the chart's look.

**Goal.** Charts are drawn from their data, and new ones are made from
data typed in or taken from a table, in Ami Pro's chart types, editable
with Drawing.

**Map.**
- `amichart.dll`: the chart types and options, the Charting Data dialog,
  defaults, and colours.
- How a chart is stored (a type-15 data record in the drawing stream,
  with the generated objects; see `OBJECTS_NOTES.md`) and how it becomes
  draw objects.

**Decisions to expect.**
- **Where the data is edited.** Recommend in the rail.
- **Data from a table in the document.** Recommend linking it so the
  chart updates; SAM stores the data as Ami Pro does.
- **Which chart types.** Recommend all of Ami Pro's.

**Better ways and logical additions to offer.** Live update from a table,
modern colour schemes (the colour panel), and axis and label formatting
in the rail.

## Run 4: `equations`, queue 19

**Goal.** Equations are drawn and edited with Ami Pro's templates,
operators, matrices, and TeX commands, and are stored so Ami Pro opens
them.

**Map.**
- `amieqn.dll`: the equation's stored form (see `OBJECTS_NOTES.md`; no corpus file has one)
  and the layout rules (sizes, limits, spacing).
- The icon palettes (ch. 30) and Equation preferences.

**Libraries to evaluate.** A maintained math layout or TeX-math parser in
Rust, and a math font with an OpenType MATH table under a free licence
(for example Latin Modern Math or STIX Two). Otherwise, own layout by
TeX's rules.

**Decisions to expect.** Recommend typed TeX as the main input (Ami Pro
takes TeX commands), with the palette in the rail and Ami Pro's look
first.

**Better ways and logical additions to offer.** LaTeX paste, equation
numbering, and inline equations in text.

## Run 5: `objects`, queue 22 (small)

**State (2026-11-07): built as run D** (`docs/OBJECTS_PROGRESS.md`; OLE objects drawn and kept, Paste Special, image processing, the Charting panel and Chart from Table, the equation editor; physical check pending). The text below is the original plan.

**Goal.**
- **Image processing (ch. 19):** brightness, contrast, sharpness, and
  halftones on grey-scale TIFF, and revert to the original.
- **OLE objects:** drawn from their stored presentation and kept byte for
  byte; Link Options shows their links.
- **Paste Special:** pastes clipboard formats as pictures.

**Map.** `amiimage.dll`; the OLE object's stored presentation in the SAM;
and Paste Special's formats.

**Decisions to expect.**
- Recommend non-destructive image adjustments in the rail.
- OLE objects shown and kept, but not editable (there is no OLE server
  here).

## Out of scope

Live OLE editing and DDE links (`docs/PHASE5_PLAYBOOK.md`, "Not
planned"), Ami Pro's bitmap editing, and copying Ami Pro's symbol
libraries.
