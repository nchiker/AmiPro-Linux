# Print and PDF Runbook

**Written:** 2026-10-20, the Phase 5 queue's run 5 (`docs/PHASE5_PLAYBOOK.md`,
`docs/playbooks/OUTPUT.md`, "Run 1"). **Purpose:** carry printing and PDF
from start to finish as a straight run. Its decisions were asked before
the run and answered on 2026-10-20 (`docs/research/DECISIONS.md`, "Print
and PDF: the user's answers", PO1–PO6); the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §29 the interface. Evidence:
`docs/research/PRINT_NOTES.md` (the decompiled `AMIPRO.EXE` and
`AMIPROUI.DLL`; the user's guide ch. 13; Ami Pro printing VM fixtures to a
PostScript file; the desktop's print portal and GTK's CUPS backend).

**Live state** is `docs/PRINT_PROGRESS.md`.

**Goal.** File › Export PDF… writes the document's pages as a PDF that
looks like the page surface (one drawing for both): fonts embedded and
subset, text selectable, pictures, frames, lines, shading, and colours,
each page at its layout's size and orientation, with bookmarks, Doc Info,
page labels, and an archival option. File › Print… prints through the
desktop's dialog. Ami Pro's print options are in a Print panel in the
rail, and the pages show as they print while it is open.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/PRINT_PROGRESS.md`,
   `docs/research/PRINT_NOTES.md`, and `docs/Design/UI_SPEC.md` §29; then
   `git status` and `git log --oneline -15`.
2. The tag `print-pdf-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **PO1** File › Print… (Ctrl+P) opens the desktop's print dialog through
  the XDG print portal (`ashpd`, already in the build); the app sends the
  chosen pages (the dialog's range: positions in the document) as a PDF
  made in the background; copies, collate, reverse, odd and even, pages
  per sheet, two-sided, and trays are the dialog's (CUPS applies them).
  File › Print Current Page… opens it with the caret's page as the range.
  Ami Pro's Crop marks, Without pictures, With doc description, On
  preprinted form, and Update fields are in a **Print panel in the rail**
  (File › Print Options…, 300 px), kept per document until it closes, for
  printing and PDF alike.
- **PO2** `krilla` 0.8 writes the PDF: fonts embedded and subset, text
  selectable; bitmaps at their own pixels; metafiles and SVG as vectors.
- **PO3** With notes comes with the notes run.
- **PO4** Bookmarks from outline levels; Doc Info as the PDF's properties
  and Ami Pro's page numbers as page labels; a PDF/A-2b option (off by
  default). Tagged PDF later.
- **PO5** While the Print panel is open, the pages show as they print; the
  document stays editable.
- **PO6** No bulk export: one document at a time.
- **Fixed whatever the answers** (Ami Pro's behaviour): options for the
  session only; crop marks put the page ½ in right and down, 7/16-in marks
  ending 1/16 in from each corner (in a PDF the paper is the page with
  ½ in all round, the trim box the page); On preprinted form leaves out
  frame and table lines, shading, fills, pictures, and protected text;
  Without pictures leaves picture frames empty; the cover sheet's labels
  and layout with four-digit years; Update fields runs Update All first.
- **Choices made in the run** are labelled PX1, PX2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** the page surface is the normal build
  (2026-10-12); rail inspectors over modal dialogs; consistent conventions
  (check marks on every on/off command); the text-editor build keeps
  Print and Export PDF as Future; fields' Update All (FL4).

## 3. Working rules

- **One drawing.** The PDF is drawn by the page surface's own `draw`
  through a renderer of the app's own (`pdf.rs`) that writes krilla
  pages; nothing that draws text, frames, tables, bullets, or rules is
  written twice. The surface's print mode leaves out what only the screen
  shows. Pictures are handed to the renderer as pictures.
- The evidence first: a rule not in `PRINT_NOTES.md` is mapped in the
  decompile or the VM before it is built, and recorded there.
- The pinned libcosmic's traps (`CLAUDE.md`): no zero-size children in a
  row or column; an operation-only task chains a repaint; never an id
  that changes at a fixed place; no `context_menu` inside a scrollable; a
  text field focused by a task hears no focus message.
- **Libraries:** `krilla` (MIT/Apache) in the app under the `page-surface`
  feature; `ashpd` as libcosmic has it; `hayro` (Apache/MIT) as a
  dev-dependency to render PDFs in tests. Core crates stay
  dependency-free.
- Small green commits: targeted tests, strict Clippy, rustfmt, a commit
  per slice. Routine full suites, Clippy, fmt, and corpus measures run
  through lower-cost subagents that report back (the user, 2026-10-20).
- Aggregate numbers only, never the corpora's file names.
- **Text that moves is measured.** Printing should move no text; the
  round-trip audit and `layout_comparison_report` run at the start and
  the exit, and before and after any slice that touches the layout or
  model crates; every changed row is explained.
- PDF files written by tests go to `target/` or a temporary directory,
  never into the repository.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour, is recorded (PX…), and goes into the handoff. If the run grows
past one long session, hand off at a slice boundary with the progress
file current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| PP.1 Setup | `PRINT_NOTES.md`, `UI_SPEC.md` §29, PO1–PO6, this runbook, the progress file, the baseline (`target/print-baseline/`), the tag | Committed; `print-pdf-run-start` |
| PP.2 The PDF renderer | `pdf.rs`: iced's renderer traits written into krilla pages: quads (fill, border, rounded corners), layers as clips, transformations, paragraphs as glyph runs with their text and embedded fonts (COSMIC Text's faces; colours; synthetic italic), plain text, images | Unit tests: a page with text and quads gives a PDF that `hayro` renders and whose text is there; fonts embedded |
| PP.3 Print mode | The page surface's print mode: one page at a time, 1 px per twip, no screen marks; pictures handed to the renderer as pictures (bitmaps at their own pixels, metafiles and SVG as vectors, unknown pictures nothing); `print::document_pdf` (each page at its layout's size and orientation) | Widget-path tests (a recording renderer: no screen marks); PDF pages rendered by `hayro` against the surface's tiny-skia drawing on fixtures |
| PP.4 Export PDF… | The command (Ctrl+Shift+E): the portal's Save dialog, the name beside the document, written to a new file and renamed; in the background with a status message | Message-path tests; README31 exported (pages, text) |
| PP.5 Ami Pro's options | Crop marks (paper geometry), Without pictures, On preprinted form (frame and table lines, shading, fills, pictures, protected text), Update fields (Update All first), With doc description (the cover sheet) | Tests per option against `PRINT_NOTES.md`; comparisons with the VM's PFC1C, PFC1D, PFC1F, PPT1F, PACLP |
| PP.6 PDF additions | Bookmarks from outline levels, Doc Info as properties, page labels from Ami Pro's page numbers, PDF/A-2b | Tests: the PDF's outline, metadata, labels; krilla's PDF/A validation |
| PP.7 Print… | `PreparePrint` and `Print` through `ashpd`: the document's paper and orientation as the dialog's set-up; the dialog's range read back (all, ranges); Print Current Page…; crop marks on the chosen paper; status messages; errors reported | Tests of the settings both ways and the pages chosen; the portal itself in the physical check |
| PP.8 The Print panel and preview | File › Print Options… opens the rail panel (`print_panel.rs`): What prints, PDF, the three buttons; the options per document for the session; the surface in print mode while the panel is open (PO5); registry and menus | Message-path and widget-path tests; screenshots light and dark |
| PP.9 Comparisons | Ami Pro's printed fixtures (PostScript → images) against the app's PDF of the same files; README31, MERCURY, and a sample of the user's corpus exported and compared page by page with the surface's drawing | Differences listed and explained in the progress file |
| PP.10 Timings | Exporting README31 and a long user file (release); the typing A/B against `print-pdf-run-start` | Release baselines; `tools/bench-ab.sh` |
| PP.11 Exit | `tools/verify.sh --release --corpus`, the measures, the records (`UI_SPEC.md` §29 notes, `FEATURE_PARITY.md` IO-03, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `REFERENCE_FEATURE_MATRIX.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue), the handoff, the push | Green; `origin/master` equals `HEAD` |
