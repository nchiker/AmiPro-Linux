# Export Progress

Live state of `docs/EXPORT_RUNBOOK.md` (tag `export-run-start`). Resume at
the first slice that is not `DONE`.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| EX.1 Setup | DONE | `eafc453` | `EXPORT_NOTES.md`; `UI_SPEC.md` §30; EX1–EX10 in `DECISIONS.md`; the baseline below | Tag `export-run-start` |
| EX.2 The exchange document | DONE | `69adc6c`, `1f9f74d` | `crates/exchange/tests/build.rs` (styles' names and order, runs over their styles, caps written as shown, a date field and its result, a live list showing Ami Pro's numbers, a DOCX of well-formed parts); `fields::tests`, `numbering` through the list test | `amipro-exchange`: the neutral `ExchangeDocument`, `build` (styles, runs, sections, bands, page numbers, fields, lists, frames placed from the layout, page tables, pictures through `PictureSink`), `losses` (`Feature`, per format) |
| EX.3 Text and the container | DONE | `69adc6c` | `zip::tests` (CRC-32, stored entries read back), `text` through the list test | `text.rs` (EC1), `zip.rs` (stored or deflated by the caller), `xml.rs` |
| EX.4 DOCX | DONE | `69adc6c`, `011e76c`, `1f9f74d` | Fixtures FC1, PI4, TB1, HDR1, BL1, OL1, G09, F08, ACELLS, README31 converted by LibreOffice and compared with the app's PDF; every DOCX of the corpus sweep well-formed | `docx.rs`: styles, numbering, settings, fonts, sections and headers, fields, tables, text boxes and pictures (DrawingML), properties |
| EX.5 ODT | DONE | `011e76c` | The same fixtures through LibreOffice; every ODT of the sweep well-formed | `odt.rs`: named styles, master pages, frames, tables, lists, fields, `meta.xml`; header styles in `styles.xml` |
| EX.6 RTF | DONE | `4e7170f`, `4da1556` | The fixtures through LibreOffice; twins in Ami Pro (below) | `rtf.rs`: full formatting per run (`{\plain …}`, only what is on), list table, sections, shapes with positioned-paragraph results |
| EX.7 Losses | DONE | `69adc6c`, `011e76c`, `f2ca77c`, `3ac8e32` | `an_export_that_loses_something_asks_first`, `plain_text_lists_what_it_leaves_out`; README31 exports to DOCX, ODT, and RTF without a list | `losses.rs`; EC13 |
| EX.8 The app | DONE | `f2ca77c`, `1447810`, `2389ca5` | `export_writes_each_format_and_offers_to_open_it`, `an_export_that_loses_something_asks_first`, the menu order test; `export_files` (ignored) | File › Export ▸; `export.rs`; `Pictures::exported` (EC3); the loss dialog; Open (OpenURI portal); the last folder (EC10) |
| EX.9 Table lines and shading | DONE | `f18b590` | `a_cells_lines_and_fill_come_from_its_record`, `cells::tests`; the app's PDF of TL1 and TL4 puts lines and fill where Ami Pro's printouts do, to the twip; measures equal (below) | `TableCell::lines`, `fill`; `amipro_layout::cell_box`, `cell_line_strokes`; drawn by the page surface (screen, paper, PDF) and exported (EC8) |
| EX.10 Export ▸ Folder… | DONE | `dd58899` | `export_folder_exports_every_document_with_a_report`, `export_folder::tests`; `export_screenshots` (ignored): the panel and the loss dialog, light and dark | `export_folder.rs` (EC14) |
| EX.11 Comparisons | DONE | `1f9f74d`, `4da1556` | Below | |
| EX.12 Timings | DONE | `2389ca5` | Below: exports in 1–6 ms; typing A/B no regression | |
| EX.13 Exit | DONE | (the exit commits) | `tools/verify.sh --release --corpus` PASS at `2c99cb1` (core 427 + 9; app 318, 111, 325 with 44, 4, 44 ignored; release builds; ISO corpus gate PASS); measures equal (`target/export-ex9/`) | The records; the handoff; the push |

## Baseline (at `export-run-start`, `target/export-baseline/`)

Measured at the start with `export-baseline.sh` (all three environment
variables set for each run, so the VM folder's comparison now finds its
style sheets through `AMIPRO_STYLES` too):

- Round trip: user's corpus 417 of 417 read back equal (Save As would list
  nothing 417 of 417), ISO 159 of 160, VM folder 280 of 280: equal to the
  print and PDF exit.
- Layout comparison: user's corpus and ISO summaries equal to the print
  and PDF exit. VM folder: equal but for the paragraphs whose style sheet
  is now found (styles from a missing STY 22 → 17 paragraphs, single
  spacing 1,464 → 1,469), an effect of `AMIPRO_STYLES` being set, not of
  any change.

After EX.9 (`target/export-ex9/`): every summary of both corpora equal to
the baseline; the VM folder gained TL1 and TL4, both read back equal
(282 of 282) and laid out as Ami Pro cached them.

## VM fixtures

- `../AmiPro-Reference/vm-lab/EXP`: Ami Pro 3.1 exported FC1, TB1, PI4,
  HDR1, BL1, OL1 as Rich Text Format and FC1 as Word for Windows 2.0
  (`EXPORT_NOTES.md`): frames dropped, the rest as listed there. Ami Pro
  opened its own TB1 RTF after Import Options.
- `OUT/TL1.SAM` (Table › Lines & Color › All, style 1) and `OUT/TL4.SAM`
  (Fill color yellow), printed to `TL1.PS` and `TL4P.PS` on the PostScript
  printer: the cell record's line word and fill, and where Ami Pro draws
  them (`EXPORT_NOTES.md`, "Table lines and shading").
- Twins in `IN/` (the app's RTF opened in Ami Pro through Import Options ›
  Convert styles): AFC1, ATL1 (before the fix: all text bold, italic, and
  capitals, as Ami Pro's reader takes `\b0` for on), ATL1B and AF01 after
  it: the text, styles, the table and its lines as the app has them; a
  frame's positioned-paragraph result comes in as a paragraph before the
  main text, its text kept; AF08's header frame text comes in as header
  text.

## Comparisons (EX.11)

- **No failures:** every SAM of the user's corpus (417 read), the ISO
  corpus (160), and the VM folder (282) exported in all four formats
  (`crates/exchange/examples/export.rs`); every DOCX and ODT package of
  the sweep (1,714) well-formed XML. Samples through the app's own export
  (`export_files`: its fonts and picture conversion): 40 user and 30 ISO
  documents, none failed.
- **LibreOffice against the app's PDF** (DOCX converted by LibreOffice
  24.2, words per page compared with `pdftotext`): the same page count for
  36 of 40 user and 21 of 30 ISO documents; pages with 95% of their words
  on the same page 103 of 173 and 97 of 231, 80% 136 and 153. Before EC4,
  EC5, and the flowed tables: 29 and 15 documents, 97 and 57 pages.
- **What still differs** (looked at page by page): LibreOffice hyphenates
  a few words the app does not; list paragraphs a little looser; headers
  grow to show text Ami Pro hides; a column separator only as tall as the
  text; rounded corners square on frames holding text in ODT and RTF;
  shadows on RTF shapes.
- **Fixture by fixture** (FC1, PI4, TB1, HDR1, BL1, OL1, G09, F08, ACELLS,
  TL1, TL4, README31): DOCX, ODT, and RTF through LibreOffice show the
  frames, pictures, tables, lines and fills, headers, bullets, outline
  numbers, columns, and README31's 14 pages as the app draws them, but for
  the differences above.

## Timings (EX.12, release)

`export_performance_baseline`, the export's background work with pictures
converted, median of five: README31's 14 pages DOCX 4.1 ms, ODT 5.2, RTF
2.3, text 1.8; AP31NAV's 40 pages 3.1, 3.2, 2.9, 0.9 ms. The typing A/B
against `export-run-start` (the table lines are drawn by the page surface;
`tools/bench-ab.sh`, two rounds, `target/bench-ab/20261020-233342/`): no
regression. The default build's typing in an empty document 0.477 and
0.588 ms against 0.451 and 0.485, at the fixture's end 0.783 and 0.756
against 0.738 and 0.735, at its middle 0.965 against 0.873 and 0.908 in
the second round (the first, 2.126 with every stage slower at once, ran
while LibreOffice converted files on this host); the rich editor's
sustained typing 0.431 and 0.393 against 0.404 and 0.394.

## Choices made in the run (EC1–EC15; `DECISIONS.md`)

- **EC1** Plain text is UTF-8 with line feeds; headers and footers are
  left out (as Ami Pro's ASCII leaves them); a frame's text and a table go
  after the paragraph that anchors them, cells separated by tabs.
- **EC2** Document times are written as UTC (W3C's `Z`) from the wall
  clock Ami Pro shows.
- **EC3** Pictures: metafiles as WMF (a placeable header added where
  missing), PNG and JPEG as they are, SVG with a PNG, the rest as PNG at
  300 dpi of their natural size; drawings, charts, and OLE objects as Ami
  Pro's screen image over the picture area; linked pictures embedded.
- **EC4** Font names of Ami Pro's era that no system resolves now are
  written by today's name for the same face (Tms Rmn, TimesNewRomanPS →
  Times New Roman; Helv, Arial MT → Arial; Courier → Courier New; Book
  Antiqua → Palatino Linotype; Helvetica-Narrow → Arial Narrow).
- **EC5** Where the document has Ami Pro's printer line height for a
  style's font, single, 1½, and double spacing are at least that line.
- **EC6** Lists: a new list instance wherever Word's or LibreOffice's
  counting would show another number than Ami Pro; numbers no list can
  show (two formats in one place, asterisks, daggers) as text.
- **EC7** Frames: Where placed frames in the first paragraph laid out on
  their page, at page positions (frames past the text on pages of their
  own); repeating frames in the headers (the first page's too, and left
  pages mirrored for Repeat right/left); With para above text frames at
  the layout's offset from their paragraph; With para above tables and
  tables alone in a Flow with text line in the text, as the layout
  continues them, a lone table taking its line's alignment; a picture
  frame is a picture when the picture fills it, else a box with the
  frame's look holding the picture at its place.
- **EC8** Table lines are black (Ami Pro's dialog has no line colour),
  centred on the cell's box from gutter middle to gutter middle, the fill
  over the box; drawn on screen and paper, left out on a preprinted form.
- **EC9** The submenu's items keep "Export" in their labels (as Format ›
  Alignment ▸ Align Left), so they stand alone in Customize Toolbar.
- **EC10** The save dialog starts in the folder the last export went to,
  else beside the document.
- **EC11** RTF: frames are shapes with a positioned-paragraph result; a
  text frame is a text box (LibreOffice shrinks a rounded rectangle
  holding text), so its corners are square; runs are `{\plain …}` groups
  with only what is on.
- **EC12** Index and contents entries (power fields 8 and 11) are the
  formats' index and contents entry fields; power fields with a
  counterpart are fields; the rest are their results.
- **EC13** The loss list counts only what shows: no hyphenation over part
  of a paragraph only where the style hyphenates; Ami Pro's `<:s>`
  paragraph flag is no loss; notes, footnotes, and bookmarks are listed
  until their runs; plain text lists frames, pictures, and tables (not
  formatting, which choosing it leaves out).
- **EC14** Export Folder never writes over a file (a number is added),
  writes `Export report.txt` (numbered too), finds style sheets in the
  style-sheet folders, and lays out one document at a time with the
  shared font system.
- **EC15** Header and footer distances come from the band frames'
  interior margins, their text's indents shifted by the band's margins
  against the page's; Word and LibreOffice grow a header to show all its
  lines where Ami Pro hides what does not fit.

## Log

- 2026-10-20, EX.1: the map (decompiled `rtf.flt`, the VM exports), the
  proposal (`UI_SPEC.md` §30), the user's answers EX1–EX10, this runbook.
- 2026-10-20, EX.2–EX.6: the exchange crate and its four filters; the
  fixtures through LibreOffice. Found and fixed: LibreOffice does not take
  a crop that reaches past a picture (pictures smaller than their frame
  were stretched); ODT header frames lost their style (a header's
  automatic styles belong in `styles.xml`); inline ODT frames hung below
  the baseline; RTF header shapes need `\shpfhdr1`; LibreOffice shrinks a
  rounded rectangle holding text.
- 2026-10-20, EX.8: the text-editor build did not compile (the export
  commands' arms needed the page surface's gate; found by `verify.sh`).
- 2026-10-20, EX.9: Lines & Color mapped (TBLLINESPROC `13a8:0000`, the
  cell word `13e8:1232`) and measured on Ami Pro's printouts.
- 2026-10-20, EX.11: the corpus samples through LibreOffice found EC4, EC5,
  and the flowed tables; Ami Pro's RTF reader found EC11's `\plain` runs.

- 2026-10-24: **the physical check passed** (the user, in LibreOffice Writer: "Export works fine").
