# Drawing Progress

Live state of `docs/DRAWING_RUNBOOK.md` (tag `drawing-run-start`).
Resume at the first slice that is not `DONE`.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| DR.1 Setup | DONE | (the setup commit) | `DRAWING_NOTES.md`; `UI_SPEC.md` §33; DW1–DW9 in `DECISIONS.md`; the baseline below | Tag `drawing-run-start` |
| DR.2 The format | DONE | `ebc681e` | `amipro_pictures::sdw` (read, write, natural size); `drawing_census`: all 487 drawings (both corpora, the VM folder, the shipped symbols) read and write back byte for byte but for runtime links | |
| DR.3 Painting | DONE | `9a523e8` | `sdw::paint`; unit tests (fills, hatches, endings, text); `drawings_against_ami_pro` (ignored): 105 drawings with Ami Pro's screen image, median ink agreement 89 % within two pixels | Differences: Ami Pro's 16-colour dithering, frame backgrounds, a few garbled old screen images, small offsets |
| DR.4 Showing | DONE | `9a523e8`, `0d34eb5` | `picture_census`: user's corpus 371 of 371 drawings and charts drawn by a reader (before: 0; 142 from Ami Pro's screen image, the rest boxes), ISO 5 of 5; screenshots of a corpus document; PDF vectors; exports as SVG with a PNG | Turned text drawn on screen and in PDF |
| DR.5 Commands | DONE | `73e3e85` | `amipro_editing::drawing`, `Command::SetDrawing`; `tests/drawing.rs` (operations; a drawing made and edited written to SAM with no driver state and read back) | DK1 |
| DR.6 Draw mode | DONE | `5b71be7` | `draw_mode.rs`, `draw_commands.rs`; the page surface's Draw mode hooks; the Draw menu (always present) and right-click items; the draw bar in the toolbar's place; `draw_mode_draws_selects_deletes_and_leaves_through_the_window`; screenshots light and dark (`drawing_screenshots`, ignored) | DK4, DK12 |
| DR.7 Tools | DONE | `5b71be7` | `draw_mode` tests: each shape tool with Shift and snap, the 40-twip click, polylines and polygons by clicks, text typed in place | DK6, DK9 |
| DR.8 Selection and editing | DONE | `5b71be7` | `draw_mode` tests: click, Shift+click, marquee, move, Shift-copy, resize and flip, point editing (move, add, delete, turn), keys, smart guides | DK2, DK5 |
| DR.9 Drawing Properties | DONE | `5b71be7` | `drawing_properties.rs`; `drawing_properties_edit_the_selection_and_the_new_look` (choices, the colour panel, a typed size with proportions, Rotate Left, the grid) | DK3 |
| DR.10 Files and clipboard | DONE | `0d34eb5` | `amipro_pictures::svg`; `draw_files.rs`; `drawings_copy_paste_import_and_save` (copy, cut, paste of objects, SVG, text; Import Drawing of an SVG; Save Drawing As SVG and SDW) | DK7, DK8, DK11, DK13 |
| DR.11 Measures | DONE | `c1cbf57` | Round trip and layout summaries equal to the start (`target/drawing-baseline/dr10/`; the VM folder's five new fixtures add 5 pages, all equal); release: a corpus drawing read, painted, drawn 3.8 ms median (max 26.5); a move on 1,100 objects 1.3 ms end to end; a line width 0.6 ms; the typing A/B below | |
| DR.12 Twins | DONE | `c1cbf57` | `drawing_twins` (ignored): ADR1–ADR3 opened in Ami Pro 3.1 as the app shows them; Ami Pro edits ADR3 in its Draw mode and saves the app's stream back unchanged (ADR3S) | |
| DR.13 Exit | DONE | (the handoff commit) | `tools/verify.sh --release --corpus`: core 489 + 11 ignored; app 360, 115, 367 (56, 6, 56 ignored); Clippy, release builds, ISO corpus gate PASS (app fmt fixed after); the records | Physical check passed (2026-10-26); the grid made visible after the user's first look |

## Baseline (at `drawing-run-start`, `target/drawing-baseline/start/`)

Measured with `target/drawing-baseline/baseline.sh` (outside Git; the
round-trip audit and the layout comparison over the user's corpus, the ISO
corpus, and the VM folder, each with `AMIPRO_STYLES`), equal to the table
editing run's exit.

- Round trip: user's corpus 417 of 417 read back equal (Save As would
  list nothing 417), ISO 159 of 160, VM folder 313 of 313 (before the
  run's fixtures DR1, DRBIG, DRW0S).
- Layout comparison: user's corpus page counts equal 315 of 394,
  paragraph line counts 14,880 of 15,356, page ends on the same page 762
  of 1,108; ISO 69 of 160, 22,773 of 24,971, 478 of 1,307; VM folder 312
  of 312, 1,860 of 1,872, 112 of 122.

## Choices made in the run (DK1–DK13, also in `DECISIONS.md`)

- **DK1** A new rounded rectangle's corners: 450 twips (Ami Pro's 30 screen
  pixels at 100 %), at most a third of each side (DR2B).
- **DK2** Several objects selected: one set of handles on their joint
  rectangle, which sizes them together (Ami Pro shows each object's).
- **DK3** A look chosen with objects selected applies to them and becomes
  the look of new objects (Ami Pro's Line Style and Fill Pattern OK).
- **DK4** Esc ends a gesture, point editing, or typing first, else leaves
  Draw mode (Ami Pro: Esc leaves).
- **DK5** Arrow keys nudge 72 twips (a grid step with Snap To), Shift ten
  times, Alt one screen pixel.
- **DK6** A text object's box is its text's extent in its font (Ami Pro
  measures with its own font layer).
- **DK7** Paste of the app's own objects puts them 144 twips down and right
  (a copy shows); other pasted or imported things go to the origin.
- **DK8** Save Drawing As… saves the selection when there is one (Ami Pro's
  Selected objects only).
- **DK9** Typing in a text object is one step while keys follow within a
  second (as the frame keys).
- **DK10** Undo is the document's history, every change one step (Ami Pro's
  Draw mode has one toggling level).
- **DK11** The clipboard type of the app's objects is
  `application/x-amipro-drawing` (a `.sdw` stream), with SVG and PNG.
- **DK12** Insert › Drawing…'s frame is the remembered Create Frame size,
  2 × 2 in before any (Ami Pro: 1 × 1 in), narrowed to the column.
- **DK13** Exports carry drawings and charts as SVG with a PNG fallback.

## Log

- 2026-10-25: VM fixtures DR1, DRBIG, DRW0–DRW3 (setup), DR2/DR2B (Ami Pro's
  rounded rectangle, ellipse, and arc constructions), ADR1–ADR3 (twins),
  ADR3S (Ami Pro's re-save).
- 2026-10-25, after the run: the user could not see the grid; its dots were
  one pixel at half opacity. Now two pixels, solid blue (`drawing_grid_screenshot`).
