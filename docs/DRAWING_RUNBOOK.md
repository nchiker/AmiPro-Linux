# Drawing Runbook

**Written:** 2026-10-25, the Phase 5 queue's run 9 (`docs/PHASE5_PLAYBOOK.md`,
`docs/playbooks/GRAPHICS.md`, "Run 2: `drawing`"). **Purpose:** carry
drawings from start to finish as a straight run. Its decisions were asked
before the run and answered on 2026-10-25 (`docs/research/DECISIONS.md`,
"Drawing: the user's answers", DW1–DW9); the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §33 the interface. Evidence:
`docs/research/DRAWING_NOTES.md` (the decompiled `AmiDraw.dll`,
`DrawMan.dll`, and `AMIPRO.EXE`; Ami Pro 3.1 fixtures DR1, DRBIG,
DRW0–DRW3), with `PICTURE_NOTES.md`.

**Live state** is `docs/DRAWING_PROGRESS.md`.

**Goal.** Drawings (and charts) are read as objects and drawn from them as
Ami Pro paints them, on screen, in PDF, and in exports; Draw mode edits a
drawing in place with Ami Pro's tools and the run's additions, the draw
bar in the toolbar's place, Drawing Properties in the rail, and a Draw
menu; every change is one undo step; drawings are written as Ami Pro
writes them, and Ami Pro opens the app's drawings as its own (twins).

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/DRAWING_PROGRESS.md`,
   `docs/research/DRAWING_NOTES.md`, and `docs/Design/UI_SPEC.md` §33;
   then `git status` and `git log --oneline -15`.
2. The tag `drawing-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **DW1** One straight run: showing drawings first, then Draw mode.
- **DW2** In place in the frame at the document's zoom; Drawing Properties
  in the rail (Line, Fill, Position and size, Text, Grid) in place of Line
  Style, Fill Pattern, Rotate, and Grid Settings; live, one undo step each.
- **DW3** The draw bar replaces the toolbar in Draw mode (tools, commands,
  Undo, Redo, the text objects' font, Done), shown even with the toolbar
  hidden.
- **DW4** A Draw menu after Table, always present, disabled outside Draw
  mode; the selection's items on the right-click menu in Draw mode.
- **DW5** Nudge (arrows, Shift ×10), Ctrl+D, Tab through objects, exact
  position, size, and angle, Rotate Left/Right 90°, Align, Distribute,
  Bring Forward, Send Backward, smart guides, any colour and width to
  12¾ pt, true curves.
- **DW6** Import Drawing… (`.sdw`, SVG as objects, an image as a bitmap
  object); Save Drawing As… (`.sdw`, SVG, PNG; the selection optionally);
  the clipboard (the app's objects, SVG, PNG out; those, images, text in).
- **DW7** Charts drawn from their objects and kept; not opened.
- **DW8** The frame keeps its size on leaving.
- **DW9** Esc, Done, or a click outside leaves (the click then acts).
- **Choices made in the run** are labelled DK1, DK2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** rail inspectors over modal dialogs;
  consistent conventions (check marks on every on/off item); the colour
  button and panel (§23); pictures (P1–P8: Graphics Scaling, picture mode,
  Save As SAM's conversions); frames in cells (T1–T3); SAM sources
  protected; Save As SAM lists what Ami Pro cannot hold.

## 3. Working rules

- **The drawing stays a picture.** A drawing is the picture's bytes (a
  `.sdw` stream) in the document; `amipro-pictures::sdw` reads them into
  objects, writes objects back as Ami Pro writes them, paints them into
  the display list the metafile path already draws (screen, PDF, export),
  and gives the natural size. Draw mode's edits are commands that replace
  the bytes (and drop the stale screen image), one undo step each, so
  Undo, native v3, and Save As need nothing new.
- **Ami Pro's bytes are kept** until the drawing is edited: an unedited
  drawing writes back byte for byte with its rendering and driver state;
  an edited one is written fresh, version 0x0102, no driver state, no
  rendering (DRW0).
- **Ami Pro's renderings are the oracle.** 142 of the user's drawings
  carry Ami Pro's screen image; the app's painting at the same size is
  compared with it pixel by pixel (aggregate numbers only).
- **Every command is one `amipro_editing::Command`**, applied to a new
  root; tested through the app's message path and the widget path (real
  clicks and keys through the `Runtime`).
- The evidence first: a rule not in the notes is looked up in the
  decompile or made as a VM fixture before it is built.
- The pinned libcosmic's traps (`CLAUDE.md`): no zero-size children in a
  row or column; an operation-only task chains a repaint; never an id that
  changes at a fixed place; no `context_menu` inside a scrollable; a text
  field focused by a task hears no focus message; **tests click portal
  commands with `Runtime::click_unrun`** so no real dialog opens on the
  user's screen; tests never touch the user's settings.
- Small green commits: targeted tests, strict Clippy, rustfmt, a commit
  per slice. Routine full suites, Clippy, fmt, and corpus measures run
  through lower-cost subagents (or background scripts) that report back.
- Aggregate numbers only, never the corpora's file names; never Ami Pro's
  code, icons, or symbol art (the app's own icons, `app/cosmic/icons/`).
- **Text that moves is measured.** The round-trip audit and
  `layout_comparison_report` (`target/drawing-baseline/baseline.sh`) run
  at the start and the exit, and before and after any slice that touches
  the layout, model, or SAM crates; every changed row is explained.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour, is recorded (DK…), and goes into the handoff. If the run grows
past one long session, hand off at a slice boundary with the progress file
current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| DR.1 Setup | `DRAWING_NOTES.md`, `UI_SPEC.md` §33, DW1–DW9, this runbook, the progress file, the baseline (`target/drawing-baseline/`), the tag | Committed; `drawing-run-start` |
| DR.2 The format | `amipro_pictures::sdw`: the objects (`DrawObject`: lines, polylines, polygons, rectangles, rounded rectangles, ellipses, arcs, text, groups, bitmaps, chart data, unknown kept), the reader (bounded), the writer, the natural size | Every corpus drawing parses; unedited objects write back byte for byte but for the runtime links; the census example |
| DR.3 Painting | `sdw::paint` into the display list: fills and patterns, hatches, pens (one pixel when styled), endings, curves, text spread to its box and turned, bitmaps, groups; charts too | Unit tests; the corpus compared with Ami Pro's renderings (pixel agreement, aggregate) |
| DR.4 Showing | The app's pictures draw drawings and charts from their objects on screen, in PDF (vectors), and in exports (SVG with a PNG); the natural size for Original size and Fit Frame to Picture; Save As SAM as before | Screenshots of corpus drawings; the PDF census; export tests |
| DR.5 Commands | `amipro_editing::drawing`: `Command::EditDrawing` (the frame, its new objects; the rendering dropped), Insert › Drawing's frame (`Command` for Ami Pro's frame at the caret, an empty frame, an empty cell); the object operations (add, delete, move, resize, flip, rotate, group, order, align, distribute, styles, points, text) on the object list | Unit tests, undo by root; SAM after edits read back |
| DR.6 Draw mode | The session's Draw mode state (frame, tool, selection, defaults, grid), entering (Insert › Drawing…, double-click, Enter) and leaving (Esc, Done, a click outside); the draw bar in the toolbar's place; the Draw menu and right-click items | Message-path and widget-path tests; screenshots light and dark |
| DR.7 Tools | Line, polyline, polygon (freehand), rectangle, rounded rectangle, ellipse, arc, text (typed in place): Ami Pro's gestures, Shift, snap, the tool staying | Widget-path tests per tool |
| DR.8 Selection and editing | Click, Shift+click, Ctrl+click, marquee, handles, move (Shift copies), resize, flips, point editing (rotation, centre, points added and deleted), Hand; nudge, duplicate, Tab, Delete; smart guides | Widget-path tests |
| DR.9 Drawing Properties | The rail panel: Line, Fill, Position and size, Text, Grid; the colour panel; defaults when nothing is selected | Message-path and widget-path tests; screenshots |
| DR.10 Files and clipboard | Import Drawing… (sdw, SVG, images), Save Drawing As… (sdw, SVG, PNG), Copy, Cut, Paste | Tests (portal commands with `click_unrun`) |
| DR.11 Measures | The round trip and layout comparison against the start; timings (release) of painting the largest drawings, a drag in Draw mode, a property change and its undo; the typing A/B | Numbers in the progress file |
| DR.12 Twins | The app makes and edits drawings (each tool and command family) and writes SAM; Ami Pro opens them as the app shows them | VM screenshots outside Git |
| DR.13 Exit | `tools/verify.sh --release --corpus`, the measures, the records (`UI_SPEC.md` §33 notes, `FEATURE_PARITY.md` GR-01/GR-02, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue and `GRAPHICS.md`), the handoff, the push | Green; `origin/master` equals `HEAD` |
