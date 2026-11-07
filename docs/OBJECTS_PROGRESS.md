# Objects Progress (run D)

The live state of `docs/OBJECTS_RUNBOOK.md` (no start tag was made). **Run D is built and recorded (DO.1–DO.12, 2026-11-07); the physical
check is pending.** Decisions OD1–OD12 were answered 2026-11-06
(recommended options). Prepared by a read-only study
(`docs/research/OBJECTS_NOTES.md`).

**Run summary.** OLE objects: 34 of 34 corpus objects drawn from their
stored presentation (median agreement with Ami Pro's screen image 96.5 % over
8 compared), kept byte for byte; Frame Properties > Object, Convert Object to
Picture, Edit > Links... (read-only). Paste Special in the rail. Image
Processing (Brightness, Contrast, Edge Enhancement, Smoothing, Invert,
Halftone, Revert; state words equal to Ami Pro's 12 of 12; non-destructive).
Charts: the data record parses and writes back equal for 104 of 104 corpus
charts and 18 of 18 VM charts; all 12 types generated; written as Ami Pro
writes them (`.sdw` picture type); the Charting rail panel and Chart from
Table. Equations: Ami Pro's TeX dialect read and written byte equal (27 of
27 VM equations), laid out by TeX's rules fitted to them (mean error 1.9 % of
width, 0.5 % of height); the in-place card, the rail palette, LaTeX paste.
Twins opened in Ami Pro 3.1; typing cost with a chart and an equation 0.05
to 0.15 ms a key; corpora round trip and layout comparison identical to the
start. Gate counts: core 790 (+29 ignored), COSMIC 666 default, 128 text
editor, 673 all features (84, 8, 84 ignored).

**Open at the exit.** Physical check (`UI_ACCEPTANCE.md`, Objects). Graphics
Scaling opening nothing in the rail once (2026-11-06) is unreproduced.
Equations: the fitted layout's worst cases (width 16.9 %, height 6.2 %, both
`\text{abc}` with a bold switch), and in Ami Pro's recomputation of the twin
`EQT2` widths -11 % to +14 % (`\sqrt` and `\left`...`\right` widest),
`\sum` +12 twips in height. Charts: the unconfirmed areas in notes 14 (the
expanded pie, 3D pies, perspective for column, line and area, legends of three
or more series, negative 3D stacked bars, small frame sizes). Ami Pro
regenerates an app-made chart 1 to 2 pixels different from ours (CHT1 twin).
Not built: Update, Unlink and Edit of OLE objects (OD7), Paste Link, New
Object, Ami Pro's Pictures dialog, colour reorder, Graphics Scaling theory.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| DO.1 Setup | PARTLY (2026-11-06; uncommitted) | | `target/objects-baseline/start/`; notes §11 | Baseline taken; plan counts corrected; UI_SPEC §42 drafted; decompile of 3 modules done and answered in notes §11 (equation module only half decompiled: 48 segments missing; chart generation geometry and Link Options procedures still to read). Was: Decompile `amichart.dll`, `amieqn.dll`, `amiimage.dll` and the EXE points in notes §9; `UI_SPEC.md` §42; baseline; tag; correct the plan's counts in `PHASE5_PLAYBOOK.md` and `GRAPHICS.md` |
| DO.2 VM fixtures | DONE (2026-11-06; uncommitted) | | `OBJECTS_NOTES.md` §12; files in `vm-lab/OUT/objects/` | CHV2 (18 charts), EQV1 (27 equations, sizes and stored form), IMV1 (state words, filters) done; CHV1 skipped (no generator yet); OLV1 impossible (the guest's OLE servers do not run). Open: frame-to-rectangle step, 3D pie and perspective, legend with 3 or more series, filter kernels, equation layout model |
| DO.3 OLE | BUILT, awaiting the user's physical check (2026-11-06; uncommitted) | | `amipro_pictures::ole`; `ole_objects_against_ami_pro` (34 of 34 drawn; 8 with a screen image compared, median agreement 96.5%); `scenario_tests/objects.rs`; UI_SPEC §42 notes | OLE 1.0 stream read, presentation metafile played: 34 of 34 drawn (31 WordArt, 2 123Worksheet, 1 ClipArt), the 8 compared with Ami Pro's screen image 89-100% (median 96.5%); Frame Properties › Object (read-only; Convert Object to Picture, one undo step, drops the object's data); Edit › Links… read-only rail list (OLE objects, DDE-flag frames); bytes never changed. Open: Links row scroll, Update/Unlink/Edit not offered (OD7) |
| DO.4 Paste Special | BUILT, awaiting the user's physical check (2026-11-07; uncommitted) | | `paste_special.rs`; `scenario_tests/objects.rs` (paste_special_*, widget click and double-click); UI_SPEC §42.2 notes | Edit › Paste Special… in the rail: the formats the clipboard offers (text, rich text, HTML, PNG/JPEG/GIF/BMP/TIFF/SVG, files) found one look each (the toolkit cannot list types; rows keep the list's order: OK-DO4-1), Paste or double-click pastes as text, formatted text or picture, one undo step; unreadable pictures and non-picture files refused with message 3092 text; empty first look retried after 150 ms; plain Paste unchanged; no Paste Link or New Object. Open: pasting a real browser or file-manager clipboard physically
| DO.5 Image processing | BUILT, awaiting the user's physical check (2026-11-07; uncommitted) | | `amipro_pictures::adjust` (6 pixel tests); `sam_image_adjust` (state words equal to IMV1A-L, 12 of 12); `scenario_tests/objects.rs` (gray TIFF, baked bitmap, not-a-bitmap, slider widget); UI_SPEC §42.3 notes; screenshots outside Git | Per OD8: ImageAdjust model, filters (lookup, smooth, edge, negate), non-destructive, drawn live (screen, PDF, print, export as PNG); rail Picture › Image (typed boxes with sliders, a drag = one undo step, Invert, Halftone, Revert), Tools › Image Processing…; gray 8-bit TIFF becomes `.tgf` with the blob in `[isd]` as Ami Pro writes it, other bitmaps baked by Save As SAM and listed. Baseline audits after DO.4/DO.5 (`target/objects-baseline/do5/`): round trip and layout comparison identical to the start for the user's corpus and the ISO; the VM folder (542 files, grown by DO.2's fixtures, the 12 IMV1 `.tgf` files among them) reads back equal 535 of 535. Open: kernel look against Ami Pro (OK-DO5-1/2), `.tgf` twin opened in Ami Pro, Tools command only opens the rail (no scroll to the section)
| DO.6 Chart model and data | DONE (2026-11-07; uncommitted) | | `amipro_pictures::chart` (11 unit tests); `crates/pictures/tests/chart_oracle.rs` (ignored census); `crates/editing/tests/charts.rs`; notes §13.1-13.2 | `Chart`: every word of the record kept; **104 of 104 readable corpus charts (6 distinct records) and 18 of 18 CHV2 charts parse and write back equal** (the other 3 of the 107 are in a password-protected file, a bodyless file, and Word Pro files the reader cannot open); data text parsed as `1018:0078` and written back (`parse_data`, `data_text`); icons, flip; native v3 keeps the record with the drawing. Open: where Automatic is stored (inferred by comparing), CHV4 not run (the decompile only)
| DO.7 Chart generation | DONE for the app slice; CHV1 open (2026-11-07; uncommitted) | | `amipro_pictures::chart_gen` (10 unit tests); `chart_oracle` (CHV2 and corpus tables); twin `CHT1.SAM` (`AMIPRO_TWIN_OUT=DIR`, `crates/editing/tests/charts.rs`); notes §13.3-13.4 | All 12 icons generate (flag 4, record last, deterministic, drawn by `sdw::paint`). Against CHV2: object counts by kind equal for 17 of 18, **347 of 380 objects within 3 twips over the 14 confirmed charts** (column, stacked column 3D, bar, line, area flat and 3D, 2206, 2207, pie, picture, stacked picture, small frame); unconfirmed: perspective, 3D and expanded pies, legends of 3+ series, negatives, the frame-to-rectangle rule (a line through 2 frames). Corpus: counts by kind equal for all 10 distinct drawings, coordinates do not (older label font and top margin). OK-DO7-1 to -4 in the notes. CHV1 done 2026-11-07 (notes §14): Ami Pro shows and regenerates the app's charts only with picture type `.sdw` (`.sct` gives "Cannot find a library"; the writer must change), the Charting dialog reads the data and options, the saved record equals ours in 30 of 38 charts; fixes for the next slice (rectangle = picture area, negative axis and order, perspective projection, axis step, legend scale, pie order) are in §14.2. DO.7b done 2026-11-07 (notes §14.6): the SAM writer writes `.sdw` as a chart frame's picture type (`.sct` stays the driver after the saved state and the in-memory marker), a test proves the frame's [isd] and [Embedded] entry equal Ami Pro's own (CHV2A); the nine corrections are in the generator: against the 39 regenerated charts, counts by kind equal 31 to 39 of 39 and objects within 3 twips 100 to 880 of 1249; the twin writer writes `.sdw` charts for the next sitting. App follow-up: call `Metrics::for_picture_area(frame width - 288, frame height - 288)` instead of `Metrics::for_frame`
| DO.8 Charting interface | BUILT, awaiting the user's physical check (2026-11-07; uncommitted) | | `chart_panel.rs`, `chart_commands.rs`; `scenario_tests/objects.rs` (`charts::`, 18 tests, `chart_screenshots`); `rail_panel_tests.rs`; UI_SPEC §42.4 notes; screenshots outside Git | Tools › Charting… and Chart from Table; the rail Chart panel (twelve own-drawn icons, variations, Legend, Grid, 3D, Depth, Perspective, colour set, Axis typed, Flip, Data grid and text, CSV copy, Save Chart as Image, Drawing…); one undo step per change; unreadable charts kept (OD9); double-click opens the panel or Draw mode. `chart_gen` gained a typed-axis override (`generate_with_axis`) and `Command::SetDrawing` accepts chart frames. Open: Pictures… (not offered), colour reorder, typing into a grid cell through the real window not tested (keys did not reach the box in the harness), OK-DO8-1 to -6 |
| DO.9 Equations | BUILT, awaiting the user's physical check and the VM twin (EQV1) (2026-11-07; uncommitted) | | `amipro_pictures::equation` (parser, writer, layout; 14 unit tests); `crates/pictures/tests/equation_oracle.rs` (ignored: the 27 VM equations, sizes and stored form); `crates/editing/tests/equations.rs` (twin `EQT2.SAM`, `AMIPRO_TWIN_OUT=DIR`); `equation_panel.rs`, `equation_commands.rs`; `scenario_tests/objects.rs` (`equations::`, 10 tests, `equation_screenshots`); notes §15; UI_SPEC §42.5 notes | Per OD2 (the stored form is known): Ami Pro's TeX dialect parsed and written as Ami Pro writes it (**27 of 27 VM texts normalise to Ami Pro's own text and read and write back byte equal**), laid out by TeX's rules with constants fitted to the 27 natural sizes (**mean error 1.9 % of width and 0.5 % of height; worst width 16.9 %, height 6.2 %, both `\text{abc}` with a bold switch**), drawn by the app (screen, PDF, exports as other app-drawn pictures); `Command::SetEquation` (one step: bytes, size in the scaling, baseline in the pan, frame fitted); Tools › Equations… and a double-click open the in-place card (a source field under the frame, redrawn as typed, errors 5100/5102/5105/5106 with their position, the last good drawing kept) and the rail palette (12 groups, face and size, Import/Save As Equation, Copy as LaTeX, Paste LaTeX); Tools › Paste LaTeX Equation (OD11); Esc or Enter leaves |
| DO.10 Extras | DONE (2026-11-07) | | DO.5, DO.8, DO.9 rows | Per OD11(a), all five built inside their slices: Chart from Table, Copy/Save Chart as Image and CSV, equation LaTeX paste (Tools › Paste LaTeX Equation), Convert Object to Picture, live image preview while dragging a slider |
| DO.11 Measures and twins | DONE (2026-11-07; uncommitted) | | notes §16; screens `vm-lab/screens/o_*.png` and Ami Pro's saved files in `vm-lab/DO11/` (outside Git) | Twins opened in Ami Pro 3.1: chart `CHT1` (opens, `.sdw` picture type accepted, Charting dialog reads the record, OK regenerates, bars 1 to 2 px different), equation `EQT2` (8 equations, no screen image: opens, repainted, Ami Pro's editor opens on them; its recomputed H and B within 1 twip but for `\sum` +12, widths -11 % to +14 %: sqrt and `\left`...`\right` the widest), adjusted `.tgf` image (opens, Image Processing mode), OLE round trip (WordArt invoice written by our writer looks identical to the original). Typing release A/B (3 interleaved rounds, 14 pages): chart + equation cost 0.05 to 0.15 ms a key (about 1.1 to 1.3 ms with, 1.0 to 1.3 without). Round trip and layout comparison identical to DO.1 (user 417, ISO 160, VM 544 of 544). No defect found |
| DO.12 Exit | DONE, records written (2026-11-07; uncommitted) | | `UI_SPEC.md` §42, `FEATURE_PARITY.md` (GR-01, GR-02, GR-03 to GR-07), `UI_ACCEPTANCE.md` (Objects), `UI_COMMAND_MATRIX.md` (no gaps), `FORMAT_COMPATIBILITY.md`, the playbook and `GRAPHICS.md`, `PROJECT_STATUS.md`, `PROJECT_PLAN.md`, `CLAUDE.md`, `SESSION_HANDOFF.md` | Docs only in this step; `tools/verify.sh --release --corpus` and the push are the committing session's to run and record |

## Choices made in the run (OK…)

- **OK-DO8-1** Charting on a drawing or picture frame asks in the panel before
  it replaces it (Replace with a Chart, Cancel); nothing is modal.
- **OK-DO8-2** A new chart's data is the clipboard's text only when it has a tab
  and numbers; otherwise the CHV2 sample, with a line saying so.
- **OK-DO8-3** A typed axis is kept in the record and used for later changes
  until Automatic is ticked; unticking alone changes nothing.
- **OK-DO8-4** A pie's 3D is stored with perspective depth (as Ami Pro's pies);
  Perspective is not offered for pies; switching to a type without 3D clears
  the depth.
- **OK-DO8-5** Charting with the caret in a table cell that holds text puts the
  chart in a With para above frame after the table; elsewhere a frame or flow
  that fits nothing gets a status message, not a new frame.
- **OK-DO8-6** Pending text that is refused when the user turns to something
  else is dropped with its reason in the panel (the change goes on); text
  applied with Enter or by leaving the box that is refused stays with its
  reason.

- **OK-DO4-1** Paste Special's rows keep the app's list order (text, rich
  text, HTML, images, files), not the clipboard source's, because the
  toolkit cannot list a clipboard's types. Ami Pro keeps the source's order.
- **OK-DO5-1** Smoothing: the pixel blended with a [1 2 1] x [1 2 1] / 16
  blur by the strength (Ami Pro's kernel not found; the VM showed about a
  one-pixel blur at 100). Needs a visual check.
- **OK-DO5-2** Edge enhancement: the pixel plus the strength times the
  four-neighbour Laplacian (one pixel of ringing with overshoot to
  saturation at 100, as the VM showed). Needs a visual check.
- **OK-DO5-3** Storage: a gray-scale 8-bit TIFF is kept in Ami Pro's own
  form (`.tgf`, the bytes unchanged, the state in `[isd]`, as the VM's IMV1
  files); every other bitmap keeps editable settings natively and Save As
  SAM bakes them into the 24-bit bitmap, listed in the loss list.
- **OK-DO5-4** The halftone mode is Fastest by default (as Ami Pro stores an
  untouched image) and is kept only where SAM can hold it (gray TIFFs).

## Baseline (DO.1)

Taken 2026-11-06 (`target/objects-baseline/start/`, release -j2). Round trip,
user corpus 417 read: bodies equal 203, files equal but for [desc]/[pg] 49,
sections 51211 of 53352, main text 417 of 417; ISO 160: bodies 28, 4 files,
sections 8204 of 8443; VM folder 516 read: bodies 469, 438 files, sections
47783 of 47810 (the folder grew by 63 files since run C). Layout
comparison, user: pages equal 82% (322 of 394), within one 94%, line counts
equal 97%; ISO pages 44% (70 of 160), lines 91%; user and ISO equal run C's.
Drawings against Ami Pro's screen images: user 105 compared, median
agreement 89.1% (27 at 95% or more, 51 at 90%); VM 10 compared, median 68.4%.

## Facts found in preparation (2026-11-06; detail in `OBJECTS_NOTES.md`)

- The plan's "charts in 4 files, equations in 4" came from `[chint]` and
  `[ehint]`, which are a table frame's per-cell hint section (decompile of
  the frame reader and the table writer), not charts or equations.
- Real counts, user's corpus: charts 107 (driver `.sct`, 108 pictures) in
  34 files, only 7 distinct data records (188, 238, 263 bytes); OLE
  objects 34 in 17 files; equations 0; `.tgf` images 0; frames with link
  strings 3 (a 1-2-3 range); `.G0x` pictures of an earlier version (their
  bytes are not in the file) 16 in 3 files. ISO corpus and VM folder:
  no OLE, equation, or `.tgf`; the VM folder has 24 chart pictures in 8
  files (run 9's twins).
- Every one of the 34 OLE objects is an OLE 1.0 embedded stream
  (classes WordArt 31, 123Worksheet 2, ClipArt Gallery 1) that ends with a
  `METAFILEPICT` presentation holding a playable metafile: all 34 can be
  drawn, against 13 today. 12 have trailing bytes after the presentation
  (unknown, kept).
- `AmiOLE.dll` is not in the install set; the client is in `AMIPRO.EXE`.
- Charting dialog (R): 12 type icons, Legend, Grid, 3D with depth,
  Perspective, colour set, Axis Automatic/Inc/Min/Max, Flip data, Data…,
  Pictures…; the data is tab/space text with labels in the first row.
- Image Processing (R, H): Brightness, Contrast, Edge Enhancement,
  Smoothing, Invert; Revert; Halftone modes; for gray-scale TIFFs only.
- SU2 (2026-11-04) already chose full editors for charts, equations, and
  image processing.

### DO.9 (Equations)

- **OK-DO9-1** Layout constants are fitted to the 27 VM equations, not read
  from Ami Pro (the layout code is in segments the decompile did not name
  and its `Measure` methods are virtual calls): italic `x`, `y`, `z` and the
  `\frac` side space and gaps are the free constants; two samples disagree
  with any single fraction side space (`\frac ab` +6.7 %, `\frac xx` -6.6 %).
- **OK-DO9-2** The glyph widths are the Times family's (no font file is
  bundled or needed; Liberation Serif and Tinos have the same advances). The
  drawing names `Times New Roman` and the system's Times-compatible face
  draws it; the big operators, Greek and symbols use the system's fallback.
- **OK-DO9-3** The source field is a card under the frame (a popover over the
  page), not a field inside the frame's rectangle: it keeps the equation
  visible while typed. The equation is applied on every edit (layout takes
  microseconds), so there is no debounce; a text that does not parse is not
  applied.
- **OK-DO9-4** The editor's source is Ami Pro's dialect without the outer `$`
  (a text with no `$` is one math run); Import Equation uses Ami Pro's own
  rule (letters outside constructs are text: `x_{i}^{2}` is `x$_i^2$`).
- **OK-DO9-5** A run of typing joins the step of the template that began it;
  each template, face or size change, import or LaTeX paste is its own step.
- **OK-DO9-6** An equation frame is made with no line, no shadow, no margins
  and a transparent background (Ami Pro's `[frm]` has side mask 0 and no
  shadow); the new frame is Flow with text, as charts are.
- **OK-DO9-7** Unknown control words are an error (5106) with their position;
  Ami Pro's 887-name table is not complete here (about 330 symbols).
