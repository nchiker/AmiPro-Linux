# Objects Runbook (run D)

**Written:** 2026-11-06 (a draft, before the decisions are asked), for the
speed-up plan's run D (`docs/SPEEDUP_PLAN.md`; `docs/PHASE5_PLAYBOOK.md`,
row D; `docs/playbooks/GRAPHICS.md`, runs 3 to 5).

**Purpose:** the picture kinds Ami Pro 3.1 hands to its own modules, as one
straight run: OLE objects drawn from their stored presentation and kept
byte for byte (with an Object section and a read-only Link Options list);
Paste Special; Image Processing (Tools › Image Processing…); Charting
(Tools › Charting…: data, the chart types, options, colours, written as
Ami Pro stores a chart); and Equations (Tools › Equations…), as far as the
decisions OD1–OD12 (section 2) allow. The decisions are asked before the
run starts, together with run E's if it follows; the run does not stop to
ask.

**Authority.** `AGENTS.md` governs how to work. `PROJECT_PLAN.md` defines
scope, `docs/research/DECISIONS.md` the decisions, `docs/Design/UI_SPEC.md`
a new section for the run (§42, written in DO.1). Evidence:
`docs/research/OBJECTS_NOTES.md` (the inventory, the corrected corpus
counts, the storage, the open points, the fixtures), with
`DRAWING_NOTES.md`, `PICTURE_NOTES.md`, `SAM_FORMAT_NOTES.md`.

**Counts the run starts from (corrected 2026-11-06; the plan's "charts in 4
files, equations in 4" came from `[chint]`/`[ehint]`, which are table cell
hints).** User's corpus: charts 107 in 34 files (7 distinct data records),
OLE objects 34 in 17 files (every one with a presentation metafile),
equations 0, `.tgf` images 0, linked frames 3. ISO corpus: none of these.

**Weight (SU1).** Full: the run writes SAM structures the app has not
written before (a chart's data record and generated objects; possibly an
equation's and an image's bytes): decompile mapping of three modules, VM
fixtures one property at a time (only for what the decompile leaves open),
twins.

**Live state** is in `docs/OBJECTS_PROGRESS.md`. The tag is
`objects-start`.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`, this runbook,
   `docs/OBJECTS_PROGRESS.md`, `docs/research/OBJECTS_NOTES.md`, and the
   run's `UI_SPEC.md` section; then `git status` and
   `git log --oneline -15`.
2. Never move the tag `objects-start`.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions already taken

Answered by the user 2026-11-06: OD1–OD4 asked, each the recommended
option (the user's standing rule is to continue unless input is truly
needed, so OD5–OD12 take their recommended option too). The full text of
each option is in "Decisions to ask" at the end of this file and in
`docs/research/DECISIONS.md`.

- **OD1:** keep SU2: full editors for charts, equations, and image
  processing, in value order (OLE drawn and Paste Special, charts, image
  adjustments, equations), each slice leaving the tree green. The corpus
  counts (charts in 34 files, OLE in 17, equations and `.tgf` in none)
  were put to the user first.
- **OD2:** an equation editor that stores Ami Pro's form once the decompile
  finds it, with the fallback gate of typed TeX drawn by the app and stored
  as a metafile picture.
- **OD3:** charts stored as Ami Pro does (a data record plus generated
  drawing objects); CHV1 decides whether Ami Pro accepts what the app
  generates.
- **OD4:** chart data from a rail table, clipboard text, and a Chart from
  Table command.
- **OD5–OD12 (recommended options taken):** all 12 chart types and every
  option; editing in the rail for chart, image, and OLE, equations in place
  with the palette in the rail; OLE drawn, kept, with an Object section and
  read-only Link Options; non-destructive image adjustments for any
  bitmap, baked into the bitmap for SAM; objects the app cannot read kept
  byte for byte and drawn from the best stored rendering, never refused;
  Paste Special lists the clipboard's formats while New Object and Paste
  Link are not offered; the extras set (Chart from Table, chart as image
  and data as CSV, LaTeX paste, Convert Object to Picture, live slider
  preview); one straight run with hand-offs at slice boundaries.

Standing decisions that bear on the run: **SU2**, P4 (kinds without a
decoder show Ami Pro's stored rendering, else a labelled box), DW7 (charts
drawn now, edited later), rail inspectors over modal dialogs, edit in
place, improve on Ami Pro where SAM can still store the result, keep
unknown structures byte for byte, consistent conventions, SU4. Choices
made during the run are OK1, OK2, … in the progress file and
`DECISIONS.md`, each keeping Ami Pro's behaviour where it has one.

## 3. Working rules

As run C's (`docs/TABLE_DATA_RUNBOOK.md` §3) and the drawing run's:
- Every document change is one `amipro_editing::Command` and one undo step
  through the app's message path (chart data and options, equation edits,
  image adjustments, Convert to Picture), written as Ami Pro writes it
  (round trip unchanged unless the slice is meant to change it).
- **Unknown structures are never dropped or rebuilt without evidence.** A
  chart, equation, image, or OLE object the app cannot parse is kept byte
  for byte, drawn from the best stored rendering, and its Save As SAM is
  exactly what was read.
- Evidence first: a rule not in the notes is looked up in the decompile
  (briefly) or made a VM fixture before it is built.
- Scenarios first, in `app/cosmic/src/scenario_tests/objects.rs` (one
  module for the run, one checker after every step), written by a
  lower-cost agent from the run's `UI_SPEC.md` section and reviewed; they
  pass before the physical check.
- Tests never open real dialogs (`Runtime::click_unrun`) and never touch
  the user's settings. The Paste Special tests use an injected clipboard.
- **What is drawn moves nothing in the text:** the round-trip audit and
  `layout_comparison_report` run at the start and the exit and after DO.3;
  every changed row is explained (OLE drawing changes pixels only).
- Routine suites, Clippy, fmt, corpus measures, and decompile sweeps go
  through lower-cost subagents that report back.
- Aggregate numbers only; never the corpora's file names or contents. The
  decompiler's output stays outside Git. Ami Pro's artwork (the sixteen
  chart pictures, palettes, icons, fonts, the symbol libraries) is never
  copied; the app draws its own.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on. A question found during the run takes
Ami Pro's behaviour, is recorded (OK…), and goes to the handoff. VM
sessions are batched (DO.2, one or two sittings), the VM is shut down
afterwards. The run is long (SU2: several sessions): hand off at any slice
boundary, and keep the tree green at each.

**Fallback gates (not stops).** DO.1's decompile decides two things, and
the run takes the recorded fallback without asking:
- If the equation storage and layout cannot be established by the end of
  DO.2, DO.9 builds the fallback of OD2(b): equations typed as TeX,
  drawn by the app's own layout, stored for Ami Pro as a metafile picture
  (static in Ami Pro), with the TeX kept in the native file.
- If Ami Pro does not accept chart objects the app generated (CHV1), DO.5
  writes the chart as Ami Pro writes it with the app's objects anyway and
  records the difference; Save As SAM lists "charts edited in the app show
  as drawings in Ami Pro" if Ami Pro then discards the data.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| DO.1 Setup | `UI_SPEC.md` §42 (draft), this runbook and the progress file with the answers, corrections of the plan's counts in `PHASE5_PLAYBOOK.md` (rows D, charts, equations) and `GRAPHICS.md`, the baseline (round-trip audit, layout comparison, drawing-against-screen-image), the tag; the decompile pass (lower-cost agents, outside Git): `amichart.dll` (record layout, types, generation, options, `AmiChart.ini`), `amieqn.dll` (`.tex` payload, state words, parser, layout), `amiimage.dll` (`.tgf`, the six filters, state), and in `AMIPRO.EXE` the ISD double-click dispatch, `Link Options` rows, Paste Special's ranking, Unlink; notes sections 2.3, 3.2, 5.2 and 9 answered | Committed; `objects-start`; notes updated; open list for DO.2 |
| DO.2 VM fixtures | Only what DO.1 leaves open (notes §7): CHV1, CHV2, EQV1, IMV1, OLV1 (reduced by the decompile follow-ups, notes 11.6), in one or two sittings | Results in the notes; the VM shut down |
| DO.3 OLE | `amipro-pictures` `ole.rs`: the OLE 1.0 stream reader (header, class, native data, presentation, trailing bytes kept), the presentation metafile played by the WMF reader, size from the METAFILEPICT extents; `Content::Metafile` for `PictureFormat::Ole`; PDF and export paths; the picture inspector's Object section (class, native size, presentation, link strings); Edit › Link Options… as a read-only rail list of linked frames (Application, Topic, Item; Update and Edit disabled with the reason); frames with `0x200000` strings round trip | Unit tests on the 34 objects' structure (aggregate); user's corpus **34 of 34** drawn, the 13 with a screen image compared with it (agreement as in run 9); round trip unchanged; layout comparison unchanged |
| DO.4 Paste Special | Edit › Paste Special… as a list of the clipboard's formats (text, rich text, HTML, image formats, files), pasting as picture or text; plain Paste unchanged; an object the app cannot hold refused with Ami Pro's message 3092 in the status bar | Message-path and widget tests with an injected clipboard; scenario |
| DO.5 Image processing | Per OD8: the adjustment model (brightness, contrast, edge enhancement, smoothing, invert, halftone mode as the decompile defines them), applied non-destructively to bitmap pictures, kept in the native file and baked into the bitmap for SAM; the rail Picture › Image section (sliders typed or dragged, Apply live, Revert), Tools › Image Processing…; double-click is unchanged; if OD8(b), the `.tgf` reader and writer and AmiImage's state words | Pixel tests of each filter on synthetic images (the decompile's formulas); undo one step per Apply; round trip; twin opened in Ami Pro (IMV1) |
| DO.6 Chart model and data | The chart data record read and written (`Chart` in `amipro-pictures`: series, points, labels, values, type, variation, options, colours, flip), kept byte for byte when untouched; the seven distinct corpus records parse and write back equal; Charting Data text (tabs, spaces, underscores, currency) parsed as Ami Pro parses it; the rail Chart section shows the data as a table | Unit tests; all 107 records parse and write back equal; fixtures CHV4 |
| DO.7 Chart generation | The chart layout: the 12 types, variations, legend, grid, 3D depth and perspective, axis automatic/inc/min/max, colour sets, labels, flip; objects flag 4 with the record, as Ami Pro writes them; the app's own pictures for the picture types | Generated objects for the seven corpus charts compared with Ami Pro's stored ones (ink agreement in the run-9 measure and the object count by kind); twin charts opened in Ami Pro (CHV1) |
| DO.8 Charting interface | Tools › Charting… (selected empty frame, chart frame, empty cell, or a new frame from Create Frame settings, using the clipboard text or asking), the rail Chart panel (type icons, variation, options, colour set, axis, Flip data, Data grid, Pictures, Drawing…), Chart from Table (selected cells), double-click opens the panel (or Draw mode if last edited with Drawing, as Ami Pro), each change one undo step, live preview on the page | Message-path and widget tests; scenarios; screenshots light and dark |
| DO.9 Equations | Per OD2: the model, the TeX parser and layout (own, by TeX's rules, with an OFL math font), the stored form as DO.1/DO.2 found it (or the fallback), Equation mode in place with the template palette in the rail, Tools › Equations…, Save As Equation… and Import Equation… (TeX files), preferences; error messages 5100–5108 | Unit tests of parse and layout; round trip of the stored form; twin opened in Ami Pro (EQV1) |
| DO.10 Extras (OD11) | The extra functions as decided, native only and listed by Save As | Tests |
| DO.11 Measures and twins | Round trip and layout comparison against DO.1; typing in a document with a chart and an equation (release A/B); twins (a chart, an OLE round trip, an equation, an adjusted image) opened in Ami Pro | Numbers here; VM screenshots outside Git |
| DO.12 Exit | `tools/verify.sh --release --corpus`; the records (progress, `UI_SPEC.md` §42, `FEATURE_PARITY.md` GR-01/GR-02 rows and new rows, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, the playbook's row and `GRAPHICS.md`, one line in `PROJECT_STATUS.md` and `PROJECT_PLAN.md`, `CLAUDE.md` if its state changes, the Not implemented list: chart editing); the handoff; the push | Green; `origin/master` equals `HEAD` |

Order note: DO.3 and DO.4 are small and visible in the user's files, so
they come first; charts (DO.6 to DO.8) before equations (DO.9) because the
corpus has charts and no equations. Slices may be handed off between
sessions in this order.

## 6. Scenarios (`scenario_tests/objects.rs`; one checker after every step)

1. Open a document with OLE objects: each draws its presentation; the
   inspector shows class and sizes; Save As SAM equal; nothing marked
   modified.
2. The linked 1-2-3 frame: Link Options… lists Application, Topic, Item;
   Update is disabled with the reason; Save As SAM keeps the strings.
3. Paste Special: text on the clipboard offers its formats; paste as
   picture makes a frame; paste as text pastes text; one undo step each.
4. A bitmap in a frame: brightness, contrast, edge enhancement, smoothing,
   invert, each one undo step, Revert restores; Save As SAM writes the
   adjusted bitmap; reopen shows it.
5. Charting from selected table cells: a new frame, the chart drawn with
   the right values; change a value in the rail's data table: redrawn;
   change the type through all twelve; Undo each.
6. Charting from the clipboard text with labels, underscores, and a
   currency value; Flip data; 3D with depth and perspective; Grid and
   Legend; axis Min/Max.
7. Open a corpus chart: its data table is shown, edit one value, Save As
   SAM, reopen: record and objects agree; Ami Pro's untouched charts write
   back byte for byte.
8. A chart last edited with Drawing: double-click enters Draw mode; after
   a Charting change, double-click opens the panel.
9. An equation: type `\frac{a}{b}`, a sum with limits, a matrix, a radical;
   the palette inserts the same; Save As Equation and Import Equation
   round trip; Undo.
10. An object the app cannot parse (a damaged chart record, an unknown
    driver): kept byte for byte, drawn from the screen image or a labelled
    box, never refused, Save As SAM equal.
11. Export (DOCX, ODT, RTF, PDF) of a document with a chart, an equation,
    and an OLE object: pictures present, the losses listed.

## 7. VM fixtures (only if DO.1 leaves them open)

CHV1, CHV2, EQV1, IMV1, OLV1 (notes §7, reduced by 11.6). Written to `IN`, opened
and saved in Ami Pro; files and screenshots outside Git; the VM shut down
after each sitting; memory is tight.

## Decisions to ask (before the run; recommendation first)

Ranked by how much each changes the build.

1. **OD1 Scope of the run.** The corpus counts are lower than the plan
   said (equations 0 and `.tgf` 0 in all files; charts 107 in 34 files
   but 7 distinct; OLE 34 in 17 files).
   (a) *Recommended:* keep SU2 (full editors for charts, equations, and
   image processing; OLE drawn and kept), built in value order: OLE and
   Paste Special first, then image processing, then charts, then
   equations, each slice leaving the tree green so that a later slice can
   be cut. (b) The plan's first scope: draw and keep everything, chart data
   edited as a table, no equation or image editors (about a third of the
   work). (c) Charts and OLE only; equations and image processing marked
   not planned.
2. **OD2 Equations: how far.** Nothing is known of the stored form, and no
   file has one.
   (a) *Recommended:* an editor with Ami Pro's templates, palette, TeX
   Command, Save As and Import of TeX, stored as Ami Pro stores it once
   DO.1/DO.2 find the form (the own layout; the look near Ami Pro's); if
   they cannot find it, fall back to (b). (b) Typed TeX with a template
   palette, drawn by the app, stored for Ami Pro as a picture it can show
   (static there); the TeX kept in the native file. (c) Draw only what a
   file holds (nothing in the corpora) and leave editing out.
3. **OD3 How a chart is stored.**
   (a) *Recommended:* as Ami Pro stores it: the data record plus the
   objects the app generates (flag 4), so Ami Pro shows them and its
   Charting dialog still opens the data (CHV1 proves it). (b) A native chart
   object the app draws from data; Save As SAM writes a plain drawing with
   no data record. (c) Charts stay pictures.
4. **OD4 Where the data comes from.**
   (a) *Recommended:* a data table in the rail (cells typed in place), the
   clipboard's text (Ami Pro's rule), and Chart from Table (the selected
   cells copied in, as Ami Pro's clipboard route). (b) Also a live link to
   a table that updates the chart (native only; SAM keeps the snapshot,
   Save As lists it). (c) Typed text only, as Ami Pro's Charting Data box.
5. **OD5 Chart types and options.**
   (a) *Recommended:* all twelve of Ami Pro's types and every option
   (legend, grid, 3D depth, perspective, colour set, axis, flip). (b) Plus
   native-only additions (scatter, donut, data labels, axis titles) that
   Save As SAM stores as plain drawings. (c) The common types only
   (column, bar, line, pie).
6. **OD6 Where editing happens.**
   (a) *Recommended:* rail panels: Chart (type, options, data table),
   Picture › Image (adjustments), Object section for OLE; equations edited
   in place on the page (Equation mode, as Draw mode is) with the palette
   and options in the rail. (b) Everything in the rail including
   equations. (c) Ami Pro's modal dialogs.
7. **OD7 OLE objects.**
   (a) *Recommended:* drawn from the stored presentation (34 of 34), kept
   byte for byte, an Object section (class, sizes, link strings), a
   read-only Link Options list, no editing and no New Object… item (there
   are no servers). (b) (a) plus **Convert to Picture** (replace the
   object with its metafile as an ordinary picture, one undo step; SAM can
   store it as a metafile). (c) Draw only, no panel.
8. **OD8 Image processing.**
   (a) *Recommended:* non-destructive adjustments for any bitmap picture
   (brightness, contrast, edge enhancement, smoothing, invert; the halftone
   modes as a print setting), in the rail; SAM stores the adjusted bitmap,
   the native file keeps the original for Revert. (b) As Ami Pro: gray-scale
   TIFFs as `.tgf` through AmiImage's own storage (needs IMV1 and the
   decompile; no corpus file has one). (c) Leave out.
9. **OD9 Objects the app cannot edit or read.**
   (a) *Recommended:* kept byte for byte; drawn from the best stored
   picture (presentation, then Ami Pro's screen image, then a labelled box
   naming the kind); never refused on open; Save As SAM unchanged; editing
   commands disabled with the reason in the status bar. (b) The same, with
   a one-click replacement by the stored picture.
10. **OD10 Paste Special and New Object.**
    (a) *Recommended:* Paste Special… lists the clipboard's formats and
    pastes as picture or text; Insert › New Object… and Paste Link are not
    offered (no servers, DDE not planned). (b) Paste Special only for
    pictures. (c) Leave Paste Special out.
11. **OD11 Extras.** Choose any: (a) *Recommended set:* Chart from Table;
    Copy Chart as Image and Save Chart Data as CSV; equation LaTeX paste;
    Convert Object to Picture (if OD7(b)); live image preview while
    dragging a slider. (b) None. (c) Add chart labels on the page (native
    only).
12. **OD12 Pace.** (a) *Recommended:* one straight run, no stop, VM
    sittings batched in DO.2, hand-offs at slice boundaries. (b) Stop after
    DO.5 (OLE, Paste Special, image processing) for a physical look before
    charts. (c) Stop before equations.
