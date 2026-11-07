# Output and Exchange Playbook

Part of `docs/PHASE5_PLAYBOOK.md` (queue rows 5, 6, and 7). Follow its recipe
and standing rules; this file says what each run covers, what to map, and
what to expect.

**Why one playbook.** Printing, PDF, export, and import all move a
document between the app and the world outside it:
- **Print and PDF** draw the shared layout output (pages, frames,
  pictures, the display lists the page surface draws) to another target.
- **Export** writes the model in another format.
- **Import** reads that same format back into the model: one mapping
  used both ways (user decision, 2026-10-18: "whatever is chosen for
  export we should also make an import for").

Both runs need pictures drawn first (Graphics, `pictures`), so a printed
or exported page carries its pictures.

**State today.** Nothing is built. `IO-03` (Printing and PDF) is
UNVERIFIED. The registry has File › Print… (Ctrl+P) and Export PDF…
(Ctrl+Shift+E) as Future commands. The SAM writer and the native format
are done. `PROJECT_PLAN.md` Phase 5: "If possible add RTF export or
DOC/DOCX whichever is most feasible".

**Evidence to start from.**
- **Guide:** ch. 13 covers printing the document or the current page,
  printing in the background, the printer, landscape, envelopes (Merge
  and templates), printer fonts, and pictures. Ch. 22 covers import and
  export filters and their options.
- **Commands:** File › Print… (0x6f), Printer Setup… (0x70), Import
  Text… (0x98), and the Save As formats.
- **Modules:** `amiprint.exe`; the export filters `rtf.flt`, `w4w*.dll`,
  `wword10.flt`, `wword20.flt`, and `word.flt`.
- **Storage:** `[prn]` and `[port]` (the printer and its port; kept).
- **On this machine:** LibreOffice (`soffice`) is installed and can
  convert exported files to PDF headless for comparison. It is not a
  reference binary.

## Run 1: `print-pdf`, queue 5

**Goal.** File › Export PDF… writes the document's pages as a PDF that
looks like the page surface:
- fonts embedded and subset, the text selectable;
- pictures, frames, lines, shading, and colours drawn;
- the page sizes and orientation of each page layout.

File › Print… prints through the desktop, with Ami Pro's options where
they apply.

**Map.** Ami Pro's Print dialog and its options (copies, all or current
page or a range, reverse order, collate, with notes, without pictures,
crop marks, update fields, odd or even pages, and others as found), and
what each option does to the output.

**Libraries to evaluate.**
- `krilla` (MIT/Apache; a high-level PDF writer with font subsetting,
  used by Typst) or `pdf-writer` (low level).
- Printing through the XDG desktop print portal
  (`org.freedesktop.portal.Print`, which takes a PDF), for example with
  `ashpd` (MIT).

**Decisions to expect.**
- **Which PDF library.** Recommend `krilla`.
- **The print dialog.** Recommend the desktop's own (through the portal),
  preceded by a small sheet with Ami Pro's options the portal lacks (with
  notes, without pictures, update fields, crop marks).
- **Background printing.** Recommend always, as the portal does it.

**Better ways and logical additions to offer.**
- PDF bookmarks from the outline levels.
- PDF links for a TOC and cross references, once those runs exist.
- Document properties from Doc Info.
- A print preview from the page surface.

**Tests.** Page images of the PDF, rendered with a free renderer in tests
if one is available, against the page surface's own drawing; text
extraction equal to the laid-out text. Compare in the VM with Ami Pro
printing the same document to a file, and in the screenshots.

**Exit.** README31, MERCURY, and a sample of the user's corpus exported
and compared page by page; printing through the portal checked by the
user physically.

**Run 1 done (2026-10-20, `docs/PRINT_PROGRESS.md`).** What the later runs
take from it:
- The app's PDF of any document (`print::document_pdf`) and its page
  images (`hayro`, a dev-dependency; `print::tests::pdf_against_screen`)
  are ready for comparing an exported file after LibreOffice's conversion.
- The user declined bulk PDF export (PO6: one document at a time); ask
  again for DOCX only if the export run's reasons differ (the archive).
- Pictures already have vector forms (`pdf_pictures.rs`: metafiles and SVG
  as paths): an export format that holds vector drawings (DOCX's DrawingML
  or EMF) can reuse the walk.
- Tables print without lines until Lines & Color (the table-editing run).

## Run 2: `export`, queue 6

**Goal.** Save As writes a modern format with the document's text,
styles, page layouts, headers and footers, frames, tables, pictures,
lists, and fields, so that other word processors open the user's archive
of Ami Pro documents. The loss check lists what the format cannot hold.

**Feasibility (the run's first decision; this is the draft):**

| Format | Writing it | Fidelity for Ami Pro documents | Read by | Check |
|---|---|---|---|---|
| DOC (Word 97–2003, binary) | a large binary format ([MS-DOC]); not feasible to write well | — | Word, LibreOffice | — |
| RTF | plain text, documented (RTF 1.9.1); small | styles, tables, and pictures good; frames as positioned paragraphs or shapes, weaker | Word, LibreOffice, and Ami Pro itself (`rtf.flt`) | LibreOffice here; Ami Pro in the VM |
| DOCX | zip and XML (ECMA-376), documented; medium | styles, tables, headers and footers, footnotes, fields, and pictures good; frames as framed paragraphs (`w:framePr`) or anchored text boxes, good | Word, LibreOffice, Google Docs, and most others | LibreOffice here |

**Recommendation:** DOCX first. It is the most useful today, and it holds
Ami Pro's frames and page layouts better than RTF. It is checked here by
converting with LibreOffice and comparing page images with the app's PDF.
RTF second, only if the user wants files Ami Pro itself can read back.
ODT is a cheap addition once DOCX exists (the same mapping), if wanted.

**Map.** How Ami Pro's model maps onto the chosen format:
- styles and their attributes;
- page layouts as sections;
- repeating frames;
- placements and wraps;
- tables with joined cells and pictures in cells;
- lists and outline numbering;
- fields and footnotes.

Ami Pro's own RTF export is a reference for what it chose.

**Libraries to evaluate.** `docx-rs` (MIT), or the project's own XML
writer over the `zip` crate. Recommend the own writer if `docx-rs` cannot
express framed paragraphs and anchored text boxes.

**Decisions to expect.**
- **The format** (above).
- **Frames as framed paragraphs or text boxes.** Recommend text boxes for
  frames with their own layout, and framed paragraphs for simple ones,
  as LibreOffice reads them best.
- **The format is also the import format** (user, 2026-10-18), so
  choose it with reading in mind. Reading is the larger half: files from
  other programs use more of the format than the app writes.
- **Bulk conversion.** Whether a whole folder of SAM files can be
  exported at once.

**Better ways and logical additions to offer.** Bulk export of a folder
(the user's archive), an export report per file (the loss list), and
document properties from Doc Info.

**Exit.** Both corpora export without failure. A sample's page images are
compared with the app's PDF after LibreOffice's conversion, and the
differences are listed. Then the user's physical check, opening exported
files in the word processor they use.

**Run 2 done (2026-10-20, `docs/EXPORT_PROGRESS.md`).** What the import
run takes from it:
- The formats are DOCX, ODT, RTF, and plain text (EX1), all written from
  one neutral form, `amipro_exchange::ExchangeDocument`. The import run
  reads each format into that same form and adds the step back to the
  model (exchange document → `Document`): the mapping exists once, both
  ways.
- Round trips to test: every corpus document exported and read back must
  equal the original in what the format holds (EX's loss list says what it
  does not). `crates/exchange/examples/export.rs` and the app's
  `export_files` (ignored) export folders; `target/compare/` holds the
  LibreOffice comparison scripts' inputs (outside Git).
- Ami Pro's own reader shows what to expect of RTF from Ami Pro: its RTF
  export drops frames and writes `\lotusoutlinelevel`; its Import Options
  (Convert, Apply, Keep, Ignore styles) are the import's style choices.
- Readers: `quick-xml` (in the app's build) for DOCX and ODT; a zip
  reader with inflate is still to write (the exchange crate's
  `zip::read_stored` reads stored entries only, for tests); an RTF reader
  of the app's own.
- LibreOffice files from other programs use far more than the app writes
  (theme fonts, tblStyle, floating tables, `w:framePr`, section types): the
  playbook's list below stands.

## Run 3: `import`, queue 7

**Goal.** The format chosen in the export run is read into the model
with the same mapping, run backwards:
- text, styles, and page layouts (sections);
- headers and footers, frames (text boxes and framed paragraphs), and
  tables with joined cells and pictures in cells;
- pictures, lists and outline numbering, fields, and footnotes.

File › Open opens such a file as a new document. Insert › File… puts one
at the caret, as Ami Pro's File › Import Text… does. What the model
cannot hold is listed in an import report, never dropped silently. The
source is protected as SAM sources are: the first Save goes to Save As.

**Map.**
- Ami Pro's File › Import Text… (ch. 22): where the text goes, its
  options, how styles are matched or added, and what its filters keep of
  frames, tables, and pictures (the `w4w*.dll` and `.flt` filters, in the
  VM).
- How the chosen format's features map onto Ami Pro's model: the export
  run's mapping, plus what other programs write that the app never does
  (built-in style names, theme fonts, floating shapes, section types,
  and others as found).

**Libraries to evaluate.** The export run's choice. For DOCX: `docx-rs`
reads as well as writes (check how much); otherwise the project's own
reader over `zip` and `quick-xml`.

**Test documents.**
- Every corpus document exported and read back, which must be equal to
  the original in the round-trip facts that the format can hold.
- Documents written here in LibreOffice for each feature.
- Free sample documents with a licence that allows it.

Other people's private documents are never used.

**Decisions to expect.**
- **Open and Insert › File….** Recommend both: Open for a new document,
  Insert › File… at the caret.
- **Style names.** How the format's built-in styles map onto Ami Pro's
  (for example Heading 1 onto Ami Pro's heading styles, or kept by
  name). Recommend keeping names, with a mapping only for Normal and Body
  Text.
- **What Ami Pro cannot hold.** Features beyond the model (for example
  floating shapes): recommend keeping them in the native format and
  listing them on Save As SAM.

**Better ways and logical additions to offer.**
- Drag a file onto the window to open it.
- Bulk import of a folder.
- The import report as a panel with a jump to each item.

**Measures.** Export then import is equal on both corpora (the strongest
test, as the same mapping runs both ways). Imported LibreOffice
documents are laid out and compared with LibreOffice's own PDF of them.

**Exit.** The round-trip result on both corpora; LibreOffice-made
fixtures for every mapped feature; the user's physical check, opening
files from the word processor they use.

**Run 3 done (2026-10-24, `docs/IMPORT_PROGRESS.md`).** What later runs
take from it:
- The mapping exists once, both ways: `amipro_exchange::build` (model to
  exchange document) and `realise` (back), with a reader and a writer per
  format. A feature added to the model is added to both and measured by
  `tests/realise.rs` (`realise_round_trip_report`, `AMIPRO_REALISE_FORMAT`).
- `Settings::ami_pro_rows` tells `realise` whether a file's row heights
  are Ami Pro's measure (the app wrote it; its generator is AmiPro-Linux)
  or another program's minimums, which Ami Pro does not keep (IC32).
- Save As SAM shortens style names past Ami Pro's 13 characters (IC31);
  Word's names often are.
- The table-editing run (8) can take imported tables as its first test
  set: grids with joined cells, row heights, lines, and fills from Word
  and LibreOffice.
- Open for the layout: text beside two Wrap around frames with a gap
  between them (twin AIMO) needs a fixture; a picture placed inside a
  cell's frame (AIMW) shows in the app and not in Ami Pro.
