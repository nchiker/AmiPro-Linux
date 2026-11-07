# Pictures Runbook

**Written:** 2026-10-18, after the user's answers to the pictures proposal
(`docs/Design/UI_SPEC.md` §25; decisions P1–P8 in
`docs/research/DECISIONS.md`). The Phase 5 queue's run 1
(`docs/PHASE5_PLAYBOOK.md`, `docs/playbooks/GRAPHICS.md`).
**Purpose:** let one session, possibly resumed after context compaction,
carry the pictures run from start to finish **as a straight run**: every
decision it needs was taken before it starts, so it does not stop in the
middle to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions so far, `docs/Design/UI_SPEC.md` §25 the approved interface (with
§11's frame interaction, §20's frame creation, and §24's frames in table
cells). This runbook adds the order, the gates, and the stop rules.

**Live state** is `docs/PICTURES_PROGRESS.md`. Keep it current: it is how
a resumed session knows where it is.

**Goal.** Pictures are drawn instead of grey boxes, as Ami Pro draws them:
bitmaps and metafiles from their bytes, other kinds from Ami Pro's stored
screen image or as a labelled box. Insert › Image… puts a picture into a
selected empty frame, the caret's empty table cell, or a new frame at the
caret the picture's size, copied or linked. Frame Properties › Picture
sizes, rotates, and moves it; picture mode moves it on the page. Pictures
paste from the clipboard and drop from the file manager. Save As SAM
writes new pictures as Ami Pro's own records, converting what Ami Pro
cannot store, and Ami Pro opens the app's files with the pictures where
the app put them. Then the user's physical check.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/PICTURES_PROGRESS.md`,
   `docs/research/PICTURE_NOTES.md`, and `docs/Design/UI_SPEC.md` §11, §20,
   §24, and §25; then `git status` and `git log --oneline -15`.
2. If the tag `pictures-run-start` does not exist, create it at HEAD. It
   is the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE`.
   Uncommitted changes belong to an interrupted run: inspect, verify, and
   finish them; never discard them.

## 2. Decisions already taken (do not ask again)

- **P1:** Insert › Image… takes BMP, WMF, PNG, JPEG, GIF, TIFF, PCX, and
  SVG; the native format keeps each file as inserted.
- **P2:** copied by default, with a "Link to file" choice in the file
  chooser. A linked picture is drawn from its file (its path, then its
  name in the document's folder in any case), else Ami Pro's screen
  image, else a box naming the path. Save As SAM embeds a copy of a link
  Ami Pro cannot follow (a Linux path) and lists it; links read from Ami
  Pro files are written back as read.
- **P3:** a new picture's frame (caret in text, no frame selected) is the
  picture's size at its own resolution, reduced to fit the column and the
  page; Flow with text, Ami Pro's default frame style, Fit in frame with
  the aspect ratio kept, selected, one undo step.
- **P4:** drawings, charts, and OLE objects are drawn from Ami Pro's
  screen image where the file has one, else a labelled box ("Drawing",
  "Chart", "Equation", "OLE object").
- **P5:** Frame Properties › Picture in the rail (Sizing: Fit in frame,
  Original size, Percentage, Custom; Width, Height, Keep aspect ratio;
  Rotate for bitmaps and drawings; Moved in frame; Reset); Format › Frame
  ▸ Graphics Scaling… opens it there.
- **P6:** picture mode as Ami Pro's: double-click a picture frame or Enter
  on it; grey handles, a hand pointer, a status note; a drag moves the
  picture (one undo step), arrows 0.05 in, Ctrl+arrows 0.5 in (a run of
  presses one step); Esc or a click elsewhere returns. Shift and a corner
  handle size the frame and the picture together (Custom).
- **P7:** paste an image from the clipboard as a picture; drop a picture
  file on the page; Replace Picture…; Save Picture As…; Fit Frame to
  Picture; Reset; a description (native only, listed by Save As).
- **P8:** Save As SAM writes pictures Ami Pro cannot store as Windows
  bitmaps, reduced when above 300 dpi at their size on the page, and lists
  each conversion.
- **From the evidence:** pictures Ami Pro stored are written back as read
  while unchanged; a new or changed picture is written with no driver
  state and no screen image (PIR1, PIR3). The bitmap and metafile readers
  and Ami Pro's screen image are the app's own, dependency-free, in a new
  core crate `amipro-pictures`; `image`, `image-extras`, `resvg`, and
  `tiny-skia` are used in the app only.
- **Earlier decisions that apply:** Save As SAM to new files only with the
  loss check (A, B); rail inspectors over modal dialogs; a convention used
  on one item goes on all its siblings; frame interaction (§11), creation
  (§20, Ami Pro's default frame and Make Default), and frames in cells
  (§24, T1–T3: Insert › Image… in an empty cell puts the picture there).
- **Already authorized** (`AGENTS.md`): decompiling any binary as needed,
  operating Ami Pro in the XP VM (including snapshots), writing `.SAM`
  files under Phase 4's decision A.

## 3. Working rules

The outline run's rules apply unchanged (`docs/OUTLINE_RUNBOOK.md` §3):
the disassemblies first, findings into `PICTURE_NOTES.md`; the approved
interface; the pinned libcosmic's traps (`CLAUDE.md`); twin fixtures in
the VM named `API…`; measure before and after (the round-trip audit on the
user's corpus with `AMIPRO_STYLES`, the ISO corpus, and the VM folder, and
`layout_comparison_report` on both corpora and the VM folder) for any
change that can move text or change written files, and explain every
difference; timings of the new behaviour and typing A/B; small green
commits (`tools/verify.sh`); native v3 records for any model addition;
aggregate numbers only, never the user's file names; push only at the end
of the session.

Also for this run:
- **Decoding stays off the typing path.** Decode on first draw, cache per
  picture (and per pixel size for vector pictures), never in `update` for
  a keystroke; a picture that fails to decode is a labelled box, never a
  panic. Bounds on every size read from a file (the 64 MiB object limit,
  dimensions, record counts).
- **Pictures do not move text**: the layout comparison must stay
  identical row for row, except where a command changes a frame.

## 4. Stop rules

There is no planned stop. Stop only for what `AGENTS.md` lists. Anything
new goes in the progress file as `WAITING FOR USER` with the conservative
choice taken, and is asked at the end, batched and ranked.

## 5. Slices, in order

### PI.1 The map (evidence, no product code)

Done before the run was set up: `PICTURE_NOTES.md` (Import Picture
`10d8:126c` and the frame it makes `1260:0509`; Graphics Scaling
`1168:0000`, `FRAMESCALEPROC`; the rectangle every draw `1168:2dd2`; the
`[isd]` record `11c8:2e29`; the drivers `AMIPAINT.DLL`, `AMIWMF.DLL`; the
screen image `1168:2016`/`26e7`), VM PI1–PI9, PIR1–PIR5, T1, the WMF
library spike, §25, P1–P8.

### PI.2 The picture model and native format

`amipro-document`: `Picture` gains typed fields read from `[isd]` values
4–11 and the pan: the sizing (`PictureSizing`: Original, Fit in frame,
Percentage, Custom), Keep aspect ratio, the percentage, the custom size,
rotation, the pan, and a description; `PictureFormat` from the extension
(the stored kinds and P1's); `Picture::rect(natural, area)` as `1168:2dd2`
(the modes, the 12-twip floor, the 31,680 reset) and the pan. The `[isd]`
values stay in `legacy` as a cache written back only while they still say
what the typed fields say. Validation (percent 1–999, sizes 1–31,680,
rotation 0–359). Native v3: the `content picture` record carries the
typed fields (older files read Ami Pro's defaults from the kept values).
`amipro-editing`: `SetPictureSizing`, `MovePictureInFrame`,
`SetPictureDescription`, `ReplacePicture`, `InsertPicture` (into a frame,
a cell, or a new frame at a position); undo tests, the random undo test.

### PI.3 Import, the SAM writer, and the loss check

The importer parses values 4–11 and the pan into the typed fields; the
writer writes the kept values when they still match, else Ami Pro's
record from the model (values 1–11, no driver state, the type, the pan,
0), as PIR1 and PIR3. Round trip on all three folders before and after
(read back equal, sections equal, unchanged); the loss check lists a
description, a format Ami Pro cannot store (P8), and a Linux link (P2),
each in words.

### PI.4 The readers (`amipro-pictures`, dependency-free)

A new core crate: Windows bitmaps (`BITMAPINFOHEADER` and core headers;
1, 4, 8, 16, 24, 32 bits; RLE4, RLE8; bit fields; top-down) to RGBA and a
writer of 24-bit bitmaps (for P8); natural sizes as `AmiPaint`'s rule; the
metafile reader (placeable header or the driver's extent) to a display
list of the records the corpora use (window and viewport, pens, brushes,
fonts, regions as clips where met, polygons, polylines, rectangles,
ellipses, arcs, text, `StretchDIBits`, `DibStretchBlt`, `PatBlt`,
palettes, save and restore); Ami Pro's screen image (`SS`: 4 planes, 16
and 32 bits; 8 bits a box). Bounds checks and fuzz-style tests on
truncated input. An example `picture_census` counts, over a folder, the
pictures each reader takes (aggregate only).

### PI.5 Drawing on the page surface

The app: `pictures.rs` decodes (the core readers, `image`,
`image-extras` for PCX, `resvg`), rasterizes display lists with
`tiny-skia` at the drawn size (sharp at every zoom), and caches per
session by object and size; the page surface draws the picture through
the renderer's raster images at `Picture::rect`, moved by the pan,
clipped to the frame's inner area, rotated as Ami Pro's (mapped in the VM
first: PIR rotation fixtures); linked pictures found as P2; P4's screen
image or labelled box. Screenshots light and dark (PI1, PI2, PI5–PI8,
README31's page 12, a corpus page with metafiles and drawings); the count
of what draws over both corpora.

### PI.6 Insert › Image…

The portal's file chooser filtered to P1's formats with "Link to file";
the picture into a selected empty frame, the caret's empty cell (T3), or a
new Flow with text frame at the caret, the picture's size (P3) reduced to
the column and page, Ami Pro's default frame style; one undo step;
disabled in a cell with text; with a picture frame selected the command
offers Replace Picture…. `Ctrl+Shift+I`. Message-path and widget-path
tests; a file that does not decode is refused with a message.

### PI.7 Frame Properties › Picture

The rail section (§25's mockup): the format and natural size; Sizing,
Width and Height (typing selects Custom), Keep aspect ratio; Rotate
(bitmaps and drawings); Moved in frame with Reset; Replace Picture…, Fit
Frame to Picture, Save Picture As…; Description. Each change one command
and one undo step; Format › Frame ▸ Graphics Scaling… opens it. Tests by
message and widget path; screenshots.

### PI.8 Picture mode

Double-click or Enter on a picture frame; grey handles and the hand
pointer; drag (one undo step through `History::amend`), arrows and
Ctrl+arrows (a run one step), Esc or a click elsewhere; Shift and a
corner handle size frame and picture together (Custom). Tests by message
and widget path.

### PI.9 Paste and drop

Edit › Paste with an image on the clipboard (and no text) inserts a
picture as Insert › Image… does (copied, PNG kept); a picture file dropped
on the page inserts it at the drop point. Tests through the runtime
harness where it can deliver them; otherwise the exact physical check.

### PI.10 Save As SAM and twins

P8's conversion before the writer (other formats to 24-bit bitmaps,
reduced above 300 dpi at their size on the page; links to Linux paths
embedded), each listed; twins in the VM made by an ignored
`picture_twins` test (`AMIPRO_TWIN_OUT` writes `API*.SAM`): PI1, PI2, PI5,
PI7, PI8 rebuilt in the app, a PNG and a JPEG converted, a picture in an
empty cell and in a selected frame; Ami Pro opens each with the picture
where the app drew it (screenshots compared), and its re-save keeps it.

### PI.11 Both formats and performance

Pictures read back from `.amipro` and SAM with nothing unexpected in the
loss check; timings to the drawn frame: README31's page 12 first draw and
redraw, the user file with the most pictures, typing beside a picture,
a pan step; typing A/B against `pictures-run-start`; the final measures.

### PI.12 Exit

§25's implementation notes, `FEATURE_PARITY.md` (GR-01 and the new rows),
`UI_COMMAND_MATRIX.md`, `UI_ACCEPTANCE.md` (a section for pictures and its
sign-off row), `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`,
`PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue, the handoff with a
short ranked list (the physical check with exact steps); `tools/verify.sh
--release --corpus`; push.

## 6. Exit

Met when both corpora draw every picture with a reader or a screen image
(the rest counted as boxes), Insert › Image… works in all three places
with copy and link, Frame Properties › Picture and picture mode size and
move pictures with undo, paste and drop insert pictures, documents
round-trip through both formats with only the listed losses, and Ami Pro
shows the app's twins as its own. The user's physical check follows.
