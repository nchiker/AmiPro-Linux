# Export Runbook

**Written:** 2026-10-20, the Phase 5 queue's run 6 (`docs/PHASE5_PLAYBOOK.md`,
`docs/playbooks/OUTPUT.md`, "Run 2"). **Purpose:** carry export from start
to finish as a straight run. Its decisions were asked before the run and
answered on 2026-10-20 (`docs/research/DECISIONS.md`, "Export: the user's
answers", EX1–EX10); the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §30 the interface. Evidence:
`docs/research/EXPORT_NOTES.md` (Ami Pro's filters from the decompiled
`rtf.flt`; Ami Pro 3.1 exporting VM fixtures; the formats; LibreOffice).

**Live state** is `docs/EXPORT_PROGRESS.md`.

**Goal.** File › Export ▸ writes the document as a Word Document (.docx),
OpenDocument Text (.odt), Rich Text (.rtf), or Plain Text (.txt), from one
neutral form with a filter per format, as Ami Pro's filters work: text,
styles with Ami Pro's names, page layouts as sections, headers and
footers, frames as floating boxes where Ami Pro places them, tables with
their lines and shading, pictures, live lists and fields, and Doc Info as
properties. What a format cannot hold is listed before writing. Export ▸
Folder… exports a whole folder of SAM files with a report. Table lines and
shading are drawn on screen, on paper, and in PDFs too.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/EXPORT_PROGRESS.md`,
   `docs/research/EXPORT_NOTES.md`, and `docs/Design/UI_SPEC.md` §30; then
   `git status` and `git log --oneline -15`.
2. The tag `export-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **EX1** DOCX, ODT, RTF, and plain text, through one pipeline: the model
  resolved once into the **exchange document** (`amipro-exchange`), each
  format a filter writing it. No HTML.
- **EX2** File › Export ▸ PDF… (Ctrl+Shift+E), Word Document (.docx)…,
  OpenDocument Text (.odt)…, Rich Text (.rtf)…, Plain Text (.txt)…,
  Folder…; each a copy (the tab keeps its file and name).
- **EX3** Frames as floating boxes where Ami Pro places them (DrawingML
  text boxes and pictures; ODF frames; RTF shapes with a positioned-
  paragraph result).
- **EX4** LibreOffice Writer is the target; exports are converted with it
  and compared with the app's PDF.
- **EX5** Table lines and shading mapped (decompile, VM) and drawn on
  screen, on paper, in PDFs, and in exports; Lines & Color stays in run 8.
- **EX6** Both one document and a whole folder (Export ▸ Folder…, a rail
  panel, an `Export report.txt`, originals only read, never over a file).
- **EX7** Losses listed before writing, Cancel or Export Anyway.
- **EX8** Live lists and fields where the format has them, Ami Pro's
  current results shown; the rest as text.
- **EX9** Ami Pro's style names; Body Text the default; outline levels.
- **EX10** Doc Info as properties; an Open button after an export; the
  last format and folder remembered.
- **Choices made in the run** are labelled EC1, EC2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** rail inspectors over modal dialogs;
  consistent conventions; Save As SAM's loss dialog (§17); new files
  written to a temporary file and renamed; PO6 (no bulk PDF); the
  text-editor build keeps export commands Future.

## 3. Working rules

- **One resolution, a filter per format.** Ami Pro's meaning (styles over
  runs, list numbers, field results, frame placements on pages, picture
  scaling) is worked out once in the exchange builder; a filter never
  looks at the Ami Pro model. The import run will read each format into
  the same exchange document.
- **The core stays dependency-free.** `amipro-exchange` depends only on the
  workspace's crates; it writes XML and RTF as text and zips with stored
  entries or a compressor the caller passes. The app compresses with
  `flate2` and converts pictures with `image`.
- **Pictures** are converted by the caller (`PictureSink`): the core asks
  for each picture in the forms a format accepts; the app answers with
  PNG, the original JPEG, GIF, or WMF, or Ami Pro's screen image.
- The evidence first: a rule not in `EXPORT_NOTES.md` (or the table-line
  notes) is mapped in the decompile or the VM before it is built, and
  recorded there.
- The pinned libcosmic's traps (`CLAUDE.md`): no zero-size children in a
  row or column; an operation-only task chains a repaint; never an id that
  changes at a fixed place; no `context_menu` inside a scrollable; a text
  field focused by a task hears no focus message; **tests click portal
  commands with `Runtime::click_unrun`** so no real dialog opens on the
  user's screen.
- **LibreOffice** runs headless only, with a private profile under
  `target/` (`-env:UserInstallation=file://…`), never the user's own.
- Small green commits: targeted tests, strict Clippy, rustfmt, a commit
  per slice. Routine full suites, Clippy, fmt, and corpus measures run
  through lower-cost subagents that report back.
- Aggregate numbers only, never the corpora's file names. Exported corpus
  files go to `target/` only.
- **Text that moves is measured.** The round-trip audit and
  `layout_comparison_report` run at the start and the exit, and before and
  after any slice that touches the layout, model, or SAM crates (table
  lines: EX.9); every changed row is explained.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour (or the format's convention where Ami Pro has none), is recorded
(EC…), and goes into the handoff. If the run grows past one long session,
hand off at a slice boundary with the progress file current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| EX.1 Setup | `EXPORT_NOTES.md`, `UI_SPEC.md` §30, EX1–EX10, this runbook, the progress file, the baseline (`target/export-baseline/`), the tag | Committed; `export-run-start` |
| EX.2 The exchange document | `crates/exchange`: the neutral form (properties, fonts, colours, styles, lists, sections with page set-up and headers, blocks, paragraphs, inlines, fields, shapes, tables, pictures) and the builder from the model (styles over runs, paragraph formats, breaks, page layouts as sections, header and footer bands, page numbers, dates and Doc Info, list numbering, frames placed on pages from the layout, tables, pictures through `PictureSink`) | Unit tests per mapping on synthetic documents; README31 and MERCURY build without loss of text (every character of every flow accounted for) |
| EX.3 Text and the container | The plain-text filter (main text, then frames in reading order, tables tab-separated, numbers as shown); the zip writer (stored, deflated by a callback; CRC-32); XML helpers | Tests: text equal to the shown text; zips read back by a reader of the test's own and by LibreOffice |
| EX.4 DOCX | `docx.rs`: package parts, styles, fonts, paragraphs and runs, sections, headers and footers, breaks, tables (grid, spans, heights, margins, heading rows), lists (`numbering.xml`), fields, frames (text boxes and pictures, anchors, wraps, lines, shadow, corners), pictures, properties | Unit tests on the XML; LibreOffice converts every fixture; page counts and text against the app's |
| EX.5 ODT | `odt.rs`: the same through ODF (page styles per layout, frames, lists, fields, `meta.xml`) | As EX.4 |
| EX.6 RTF | `rtf.rs`: the same through RTF 1.9 (`\shp` with `\shprslt` positioned paragraphs) | As EX.4; Ami Pro opens the app's RTF (twins) |
| EX.7 Losses | What each format cannot hold (`ExportLoss`), from the builder and each filter | Tests per loss; README31, MERCURY, AP31NAV listed |
| EX.8 The app | File › Export ▸ submenu and its commands; the Save dialog per format; the background job; pictures converted (`image`: PNG; WMF kept); deflate; the loss dialog (§17's); the status message with Open (the OpenURI portal); the last format and folder in the settings | Message-path tests (`click_unrun` for portal commands); README31 exported in all four formats |
| EX.9 Table lines and shading | Map the cell line and shading fields (decompile, VM fixtures), the model (`TableCell` lines and shading), import and SAM writer (Save As lists nothing new), native format, layout strokes, the page surface and PDF drawing, the exports | Baseline before and after (round trip, layout comparison: no text moves); VM fixtures drawn as Ami Pro draws them; screenshots light and dark |
| EX.10 Export ▸ Folder… | The rail panel (`export_folder.rs`): From, subfolders, To, format, Export and Stop; background export with style-sheet search; `Export report.txt`; never over a file | Message-path and widget-path tests; a folder of fixtures exported; screenshots light and dark |
| EX.11 Comparisons | Both corpora and the VM folder exported in all four formats without failure; a sample converted by LibreOffice and compared page by page with the app's PDF; Ami Pro opening the app's RTF; differences listed and explained | The numbers in the progress file |
| EX.12 Timings | README31 and AP31NAV exported per format (release); the typing A/B against `export-run-start` (EX.9 touches drawing) | Release baselines; `tools/bench-ab.sh` |
| EX.13 Exit | `tools/verify.sh --release --corpus`, the measures, the records (`UI_SPEC.md` §30 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `REFERENCE_FEATURE_MATRIX.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue), the handoff, the push | Green; `origin/master` equals `HEAD` |
