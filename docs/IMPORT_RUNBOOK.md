# Import Runbook

**Written:** 2026-10-21, the Phase 5 queue's run 7 (`docs/PHASE5_PLAYBOOK.md`,
`docs/playbooks/OUTPUT.md`, "Run 3"). **Purpose:** carry import from start
to finish as a straight run. Its decisions were asked before the run and
answered on 2026-10-21 (`docs/research/DECISIONS.md`, "Import: the user's
answers", IM1–IM12); the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §31 the interface. Evidence:
`docs/research/IMPORT_NOTES.md` (Ami Pro's import from the VM and the
decompile; the readers; what other programs write; the sample documents)
and `docs/research/EXPORT_NOTES.md` (the exchange document and its
writers).

**Live state** is `docs/IMPORT_PROGRESS.md`.

**Goal.** File › Open reads Word Documents (.docx), OpenDocument Text
(.odt, .fodt), Rich Text (.rtf), and Plain Text (.txt) through the export's
exchange document run backwards, one reader per format, after Ami Pro's
Import Options (Convert, Apply, Keep, or Ignore styles; for text, Ami Pro's
ASCII options): text, styles, page layouts, headers and footers, frames,
tables, pictures (EMF too), lists, fields, comments as Ami Pro notes,
footnotes at the end, and Doc Info. Insert › File… puts a file at the
caret; a dropped document opens; Paste takes rich text. What the document
cannot hold is listed in an Import Report panel with jumps, never dropped
silently. Import Folder… converts a folder into `.amipro` documents.
Exported documents read back equal in what each format holds.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/IMPORT_PROGRESS.md`,
   `docs/research/IMPORT_NOTES.md`, and `docs/Design/UI_SPEC.md` §31; then
   `git status` and `git log --oneline -15`.
2. The tag `import-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **IM1** File › Open for DOCX, ODT/FODT, RTF, and text (a new tab, the
  source protected); Insert › File… at the caret (main text, frame text, a
  cell); a dropped document file opens.
- **IM2** Ami Pro's Import Options before each file with styles, the last
  choice preselected.
- **IM3** Insert › File… at the caret without a page break; the document's
  own styles keep their definitions (only new styles added); its page
  set-up untouched; one undo step.
- **IM4** Normal (Word) and the default paragraph style (LibreOffice)
  become Body Text; every other name kept; names over 13 characters
  shortened and listed only on Save As SAM.
- **IM5** Lists set on paragraphs take Ami Pro's list styles (Bullet 1,
  Number List, List 2…, Bullet 2…), the paragraph's own look kept; a
  number Ami Pro cannot count is text.
- **IM6** Comments as Ami Pro notes; footnotes and endnotes as numbered
  paragraphs at the document's end, their numbers raised in the text.
- **IM7** The document as it stands (tracked changes accepted); the nearest
  Ami Pro form for the rest; everything listed.
- **IM8** The Import Report: a rail panel with jumps and Copy, opened by
  itself when something is listed.
- **IM9** Text: Ami Pro's ASCII options in Import Options, Each line is a
  paragraph preselected, the characters detected.
- **IM10** Nothing through LibreOffice.
- **IM11** File › Import Folder…, a rail panel writing `.amipro`
  documents, never over a file, with `Import report.txt`.
- **IM12** Paste takes the clipboard's RTF with Apply styles.
- **Choices made in the run** are labelled IC1, IC2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one, the
  format's convention where it has none.
- **Earlier decisions that apply:** rail inspectors over modal dialogs (the
  Import Options dialog stays modal: it decides before the document
  exists, as in Ami Pro); consistent conventions; SAM sources protected
  (the first Save goes to Save As); new files written to a temporary file
  and renamed; EX1's one pipeline with a filter per format; the text-editor
  build opens the same formats (no Insert › File…, no report panel).

## 3. Working rules

- **One mapping, both ways.** Each reader produces an `ExchangeDocument`
  as the export's builder does; one step (`realise`) turns any exchange
  document into the model. A reader never builds the model; `realise`
  never looks at a format. What a reader resolves (style inheritance,
  themes, toggles, table styles, numbering) it resolves into the exchange
  document's flat terms.
- **The core stays dependency-free.** Zip, XML, and RTF readers are the
  project's own (`IMPORT_NOTES.md`, "The formats' readers"); inflate is
  passed in by the caller, as export passes deflate. The app inflates with
  `flate2`; core tests and examples may use a pure-Rust dev-dependency.
- **Hostile input.** Every reader takes any bytes: limits on sizes, entry
  counts, nesting depth, and inflated size; never a panic, never a fetch
  (no external entities, no linked files followed); the LibreOffice fuzz
  folders are the gate.
- **Nothing silent.** Every feature a reader or `realise` cannot carry
  becomes an import finding (kind, count, and a place where there is one);
  the report and `Import report.txt` show them.
- The evidence first: Ami Pro's behaviour is in `IMPORT_NOTES.md`; a
  format's rule comes from its specification and is checked on the sample
  documents and LibreOffice's rendering.
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
  through lower-cost subagents (or background scripts) that report back.
- Aggregate numbers only, never the corpora's file names. Sample documents
  stay in `../AmiPro-Reference/import-samples/`; pandoc's (GPL) are never
  copied anywhere.
- **Text that moves is measured.** The round-trip audit and
  `layout_comparison_report` (`target/import-baseline/baseline.sh`) run at
  the start and the exit, and before and after any slice that touches the
  layout, model, or SAM crates (IM.9 `[l1]`); every changed row is
  explained.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour (or the format's convention where Ami Pro has none), is recorded
(IC…), and goes into the handoff. If the run grows past one long session,
hand off at a slice boundary with the progress file current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| IM.1 Setup | `IMPORT_NOTES.md`, `UI_SPEC.md` §31, IM1–IM12, this runbook, the progress file, the baseline (`target/import-baseline/`), the tag | Committed; `import-run-start` |
| IM.2 The readers' ground | In `amipro-exchange`: the zip reader (central directory, stored and deflated entries through an `Inflate` callback, zip64 refused, limits), the XML pull reader (namespaces, entities, CDATA, depth limit, lenient on undeclared prefixes), import findings (`ImportFinding`), the options (`ImportOptions`: the style option, the text options) | Unit tests; every sample package's parts read or refused without a panic |
| IM.3 Realise | `realise`: exchange document → `Document`: styles by the option (IM2, IM4), paragraphs and runs, paragraph formatting (what Ami Pro cannot hold as a paragraph's own formatting through variant styles), sections as page layouts, headers and footers, page numbers, fields, lists (IM5), tables, frames by their anchors, pictures, notes and footnotes (IM6), properties as Doc Info; the findings with places | `realise(build(document))` equal to the document in the round-trip facts the exchange document holds, on the VM fixtures and README31 |
| IM.4 Plain text | `text` reader: line ends (each line, or joined with blank lines ending paragraphs, a word hyphenated at a line's end joined), Keep style names (`@Name@`), characters (BOM, UTF-8, UTF-16, Windows ANSI, PC-ASCII, 7-bit) | Ami Pro's fixtures IM2 and IM3 give IMT's and IMT3's paragraphs; the app's text exports read back |
| IM.5 RTF | `rtf` reader: tokenizer (bytes, `\bin`, `\'hh` by code page, `\u` and `\uc`), destinations, styles, fonts and colours, character and paragraph formatting, sections, headers and footers, tables (merges, nesting flattened), lists (`\listtable` and `\pn`), fields, pictures, shapes and positioned paragraphs as frames, footnotes, annotations, `\info` | Ami Pro's own RTF exports (`vm-lab/EXP`) and the app's read back; the sample RTF files read or refused without a panic |
| IM.6 DOCX | `docx` reader: package and relationships, styles (defaults, inheritance, toggles, linked and character styles), theme fonts and colours, numbering, sections, headers and footers, fields, tables (grid, spans, merges, table styles), drawings (inline, anchored, text boxes, `mc:AlternateContent`, VML), framed paragraphs, footnotes, comments, tracked changes, content controls, properties | The app's exports read back; the sample DOCX files read or refused without a panic |
| IM.7 ODT | `odt` reader (and flat `.fodt`): styles and automatic styles, master pages, lists, frames, tables (covered cells, repeats), fields, notes, annotations, tracked changes, `meta.xml` | As IM.6 |
| IM.8 EMF | `amipro-pictures` `emf`: Enhanced Metafiles played to a `Drawing` as WMF is; `PictureFormat::Emf` in the model and native format; drawn on screen and in the PDF; Save As SAM converts it; exported as EMF | Sample EMFs drawn; screenshots; the PDF census |
| IM.9 The starting layout | The SAM importer reads `[l1]` (the layout a document starts in), written back by the SAM writer | Measures before and after; Ami Pro's imports (IMC…) start in their layout |
| IM.10 Round trips | Every document of both corpora and the VM folder exported to each format and read back: the exchange documents equal in what the format holds, and `realise` equal in the facts | The numbers in the progress file |
| IM.11 The app | File › Open (filters, detection by content), the Import Options dialog, the import off the interface's thread, the tab (source protected), a dropped document, Insert › File… (one undo step, any flow), Paste of RTF | Message-path tests (`click_unrun`); widget-path tests; screenshots light and dark |
| IM.12 The Import Report | The rail panel (IM8): findings by kind with counts, jumps, Copy; the status bar's count | Message-path and widget-path tests; screenshots light and dark |
| IM.13 Import Folder… | The rail panel (IM11): From, subfolders, To, the style option, Import and Stop; `.amipro` documents, never over a file; `Import report.txt` | Tests; a folder of samples imported; screenshots |
| IM.14 Comparisons | Sample documents (LibreOffice- and Word-written) imported and laid out against LibreOffice's PDFs of them (pages, words per page); the robustness gate; timings (release); the typing A/B if drawing changed (EMF) | The numbers in the progress file |
| IM.15 Twins | The app imports fixtures (RTF, DOCX, ODT) and writes them as SAM; Ami Pro opens them as the app shows them | VM screenshots outside Git |
| IM.16 Exit | `tools/verify.sh --release --corpus`, the measures, the records (`UI_SPEC.md` §31 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `REFERENCE_FEATURE_MATRIX.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue), the handoff, the push | Green; `origin/master` equals `HEAD` |
