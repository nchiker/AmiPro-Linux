# Printing in Ami Pro 3.1: evidence (2026-10-20)

Mapped for the Phase 5 queue's run 5 (`print-pdf`,
`docs/playbooks/OUTPUT.md`): File › Print…, its options, and what each
does to the printed page. Decompiler output stays outside Git; these are
findings in my own words, routines by address. Evidence levels as in
`SAM_FORMAT_NOTES.md`: VERIFIED (decompile), VERIFIED (fixture) for the XP
VM, VERIFIED (manual) for the 3.1 user's guide (ch. 13), and INFERRED.

**VM set-up for this run.** The guest had only the Microsoft XPS Document
Writer. A PostScript printer, "PS File" (driver Apple LaserWriter 12/640
PS, 600 dpi, port `FILE:`), was added from the guest's own driver cache
(`printui.dll,PrintUIEntry /if … /r "FILE:"`; live snapshot "Before
print-pdf run (2026-10-20)" taken first). Ami Pro prints to it through
File › Printer Setup (the document's printer) and File › Print…; Windows
asks for the output file (`L:\OUT\P*.PS`), and the host turns it into
PDF and page images with Ghostscript's `ps2pdf` and poppler's
`pdftoppm`/`pdftotext` (outside Git). User Setup › Options › Print in
background had to be turned off: AmiPrint failed ("Couldn't open spool
file"). Fixtures printed (in `../AmiPro-Reference/vm-lab/OUT`): FC1 (a
frame with lines, shadow, rounded corners) plain, with Crop marks, With
doc description, and On preprinted form (PFC1, PFC1C, PFC1D, PFC1F);
PT1OUT (protected text) plain and on preprinted form (PPT1, PPT1F); ACELLS
(a picture in a table cell) plain, on preprinted form, and Without
pictures (PACL, PACLF, PACLP); PI4 (a TIFF picture frame) and TF1 plain
and on preprinted form; F13 and ATP2S plain and on preprinted form.

## The commands and dialogs

| Ami Pro | Command | Dialog (`AMIPROUI.DLL`) | Notes |
|---|---|---|---|
| File › Print… (Ctrl+P) | `0x6f` → `1238:0000` | 1039, `PRINTPROC` `1160:0000`; OK checks From ≤ To (`1160:0435`), else beeps and focuses From | Number of copies (1–9999); Page range: All, Current page, From–To (1–9999); including Even pages, Odd pages, Both; "Printer: NAME on PORT"; Options…, Setup… |
| Print › Options… | `0x7e` (`1238:0a7a`) on OK | 1083, `PRINTOPTPROC` `1168:0000` | Reverse order, Collate, Crop marks, Without pictures, With notes, With doc description, On preprinted form, Update fields; Bin Options (First page, Rest: the driver's bins, dimmed when it lists none) |
| File › Printer Setup… | `0x70` → `1060:0000` | 1017, `CHGPRTPROC` `1180:0000` | The document's printer from the installed ones; Setup… runs the driver's own dialog. Changing it reformats the document and resets the bins |
| (no dialog) | `0x75` (`1238:0149`) | — | Print at once with the settings; used by master documents |
| /P on the command line | `0x147` (`1238:0085`) | — | Prints without the dialog, then closes Ami Pro |
| Print Orientation | `0x77` passes its result | 1041, `WRONGPAPERPROC` `1178:0000` | Below |
| Now Printing | Cancel is `0x78` | 1014, `DLGPRINTABORT` | "Ami Pro is printing your document." and the file name; the main window is disabled meanwhile |

VERIFIED (decompile); every dialog's contents VERIFIED (fixture,
screenshots `pp-print`, `pp-opts-*`).

## The settings

- Kept in DGROUP: copies `2323`, From `2325`, To `2327`, the flag word
  `2329`, and the first-page and other-pages bins `232b`/`232c` (`0xff`:
  none). VERIFIED (decompile).
- The flag word: `0x0001` All, `0x0002` Reverse order, `0x0004` Collate,
  `0x0010` Crop marks, `0x0020` With doc description, `0x0040` On
  preprinted form, `0x0080` Update fields, `0x0100` With notes, `0x0200`
  Even, `0x0400` Odd (both: `0x0600`), `0x0800` Without pictures,
  `0x1000` Current page. VERIFIED (decompile: `PRINTPROC`,
  `PRINTOPTPROC`, `1160:0435`).
- Defaults (`1050:0178`, a document's set-up): 1 copy, From 1, To 9999,
  All, Both, no option, no bin. **Per document, for the session:**
  nothing is written to AMIPRO.INI or the file. VERIFIED (decompile);
  VERIFIED (manual): "Ami Pro continues to use the options you specify
  until you either change them, close the document, or exit the
  application."
- Current page is dimmed in Draft and Outline modes (callback `0x1c`:
  `2a39 & 0x10`, `1038:084c`); choosing it sets From and To to the
  current page; All sets 1 and 9999. VERIFIED (decompile).

## The print loop (`1238:0149`)

- Update fields first runs `0x10b` (Edit › Power Fields › Update All).
  Then the job (`1238:1506`), the description page, the pages
  (`1238:057a`, one page `1238:074b`), and the end (`1238:1f74`).
  VERIFIED (decompile).
- Windows 3.0 escapes: STARTDOC with "Ami Pro - FILE.SAM" (the
  PostScript's `%%Title` in every fixture), SETABORTPROC, NEXTBAND and
  BANDINFO on banding devices, NEWFRAME, ENDDOC. VERIFIED (decompile,
  fixture).
- **Coordinates from the paper's corner**: the printing offset
  (GETPRINTINGOFFSET) is subtracted, so a 1-in margin is 1 in from the
  paper's edge whatever the printer's unprintable border. VERIFIED
  (fixture: text at 525 dots from the printable origin plus a 75-dot
  offset, 600 dots = 1 in).
- **Which pages**: From, To, Even, and Odd count the page's position in
  the document, not its printed number. Reverse steps down from To (or
  from the last page with All). Collate repeats the whole range per
  copy; otherwise the driver makes the copies when it can, else Ami Pro
  sends each page N times. VERIFIED (decompile).
- **Each page layout's paper and orientation** go to the driver
  (`1238:25ba`): orientation from the layout; the paper matched to a
  standard size within 1/8 in, else a custom size. With ResetDC (Windows
  3.1) the device is reset at every change of layout, so mixed sizes and
  orientations print as laid out. The Print Orientation dialog appears
  only without ResetDC, when the first page is more than 1 in larger than
  the paper (Continue, Cancel, Configure Printer…). VERIFIED (decompile);
  VERIFIED (manual).
- **Background printing** (User Setup › Options › Print in background,
  on by default): Ami Pro writes the pages to a temporary metafile and
  hands it to AMIPRINT.EXE, which prints while Ami Pro goes on; without
  AMIPRINT.EXE it offers to print in the foreground. VERIFIED
  (decompile); VERIFIED (manual).
- **No print to file** in Ami Pro: only through a Windows port such as
  `FILE:` (this run's set-up). VERIFIED (dialogs; fixture).

## What each option does

- **Reverse order, Collate, copies**: the order and repetition above.
- **Crop marks** (`1238:1506`, `074b`, `1008`): the page is not scaled;
  it moves **1/2 in right and 1/2 in down** on the paper. Each corner gets
  two thin lines on the extensions of the page's edges, in the half inch
  outside the page, **7/16 in long, ending 1/16 in short of the corner**;
  for the size check the paper counts 1 in smaller. On paper the size of
  the page only the top-left mark lands on the paper. VERIFIED
  (decompile); VERIFIED (fixture PFC1C: every mark of the page moved by
  300 dots at 600 dpi; a horizontal line at 1/2 in from the top from the
  paper's edge to 263 dots, a vertical one at 1/2 in from the left to 262
  dots; nothing else added); VERIFIED (manual). The pen's width is not
  readable in the decompile; the fixture's stroke is under 1 pt.
- **With doc description** (`1238:12af` → `12a0:04e0`): one extra sheet
  before the document, printed once whatever the copies; none when the
  document has no description record. Fixed-pitch lines (the printer's
  default font: Courier 10 pt on the PostScript printer), 1 in from the
  left and 1 in down (from the printable corner here), double-spaced:
  "Filename:", "Path:", "Style Sheet:" (or "None"), "Date Created:",
  "Time Created:", "Date Last Rev:", "Total Editing Time:", "Total
  Revisions:", the eight Doc Info fields by their names, each label
  padded to 20 characters before its value; then "Doc Desc:" and the
  description word-wrapped from after the label to 1 in from the right;
  then, one line lower, "Imported files:" and the linked pictures' files.
  VERIFIED (decompile; fixture PFC1D, whose dates print with Ami Pro's
  year "126" for 2026, a fault the fields run already declined to copy).
- **With notes** (`1158:1ea4`, `16ce`, `1a32`, `1c1b`, `200d`,
  `1238:057a`): each note mark prints as "[" initials and note number
  "]" where it is; after the last page a temporary page break starts the
  notes pages: "Page N:" when the page changes, then the note's
  paragraphs, the first prefixed "[XXn]:"; only notes on printed pages.
  VERIFIED (decompile); VERIFIED (manual). Not tried in the VM (notes are
  the notes run's).
- **On preprinted form**: frame lines, shadows, and fills are not drawn (a
  text frame's background is white), nor frame and page column lines,
  page border lines, table lines and shading, pictures, and text with
  the protected attribute; everything else is, where it was. VERIFIED
  (decompile: the shared test `1288:1471`, `1530:0000`, `1510:05bb`,
  `1178:08a4`, `13f0:0e42`, `1108:2f2f`); VERIFIED (fixtures: FC1's frame
  gone, text in place; PT1OUT's five protected runs gone, the other
  letters where they were; the pictures of ACELLS and PI4 gone);
  VERIFIED (manual) for protected text. Paragraph rules: no check found
  (they print).
- **Without pictures** (`1530:0000`): the picture is not drawn; its
  frame's lines, shadow, and fill are. VERIFIED (decompile; fixture
  PACLP); VERIFIED (manual): "Frames containing pictures print as empty
  frames."
- **Update fields**: Update All before the job. VERIFIED (decompile).
- **Bins**: the first page from one tray, the rest from another, when the
  driver has trays. VERIFIED (decompile).

## Screen and paper

- Nothing in a document is marked "do not print": no frame, style, or
  text property says so. VERIFIED (decompile).
- What only the screen shows is drawn only when there is no print device
  (`2a49 == 0` in `1108:02a3`, `1530:0000`): the margin shading, column
  guides, selection, the selected frame's handles. The paper's background
  is always white. VERIFIED (decompile; fixtures: no such marks in any
  printed file).
- `[prn]` and `[port]` hold the document's printer name and port only (no
  driver, no device settings); read by `1058:06cc`, written by
  `11c8:1c7b` and `11c8:14e0`. Ami Pro lays the document out with that
  printer's metrics (the calibrated line heights come from them).
  VERIFIED (decompile).

## The desktop side (this machine, 2026-10-20)

- **Printing goes through the XDG print portal**
  (`org.freedesktop.portal.Print`, version 2): COSMIC's own portal has no
  Print interface, so `xdg-desktop-portal-gtk` 1.15.1 answers it with
  GTK's print dialog (printer, copies, collate, reverse, odd or even,
  pages per sheet, scale, paper, two-sided, quality, "Print to File").
  `ashpd` 0.12.3 (MIT), already in the build through libcosmic, has its
  client (`desktop::print`). VERIFIED (busctl introspection; the portal's
  source).
- **PreparePrint** shows the dialog and returns the settings, the page
  set-up, and a token; **Print** takes a file (PDF) and the token. The
  dialog declares page set, copies, collate, reverse, scale, and pages per
  sheet as the application's to apply ("manual capabilities"), but when
  the job is a file, GTK's CUPS backend passes copies, collate, reverse,
  odd/even, and pages per sheet to CUPS as job options, which CUPS
  applies (`cups_printer_prepare_for_print`, GTK 3.24.41). **The page
  range is not passed to CUPS**: the application must send only the
  chosen pages. The dialog's "Current page" stays dimmed, as the portal
  never tells it the current page. VERIFIED (source, GTK 3.24.41 and
  xdg-desktop-portal-gtk 1.15.1); the printed result is the physical
  check's.
- CUPS 2.4.7 with cups-filters 2 is installed; three printers are set up.

## Libraries (2026-10-20)

| Need | Choice | Licence | Why |
|---|---|---|---|
| Writing PDF | `krilla` 0.8.2 (Typst's PDF writer, over `pdf-writer` 0.15) | MIT or Apache-2.0 | Fonts embedded and subset (`subsetter`), text drawn as glyph runs with their text (selectable, searchable), paths, clips, images, outlines (bookmarks), metadata, page labels, PDF/A and PDF/UA validation; maintained (June 2026); pure Rust; its `skrifa` and `tiny-skia-path` are already in the build at other versions |
| Printing | the XDG print portal through `ashpd` 0.12.3 | MIT | Already in the build; the desktop's own dialog and printers |
| Checking PDFs in tests | `hayro` 0.8.0 (a PDF renderer by krilla's author) as a dev-dependency | Apache-2.0 or MIT | Renders the app's PDF to images in tests, to compare with the page surface's own drawing; pure Rust |
| Lower level (not chosen) | `pdf-writer` alone, `printpdf` 0.12 (MIT) | | Font subsetting and text extraction would be the app's own work |

**One drawing path.** The page surface's widget draws through iced's
renderer traits (`renderer::Renderer`, `text::Renderer`,
`image::Renderer`), and iced's paragraphs expose their shaped COSMIC Text
buffer (`Paragraph::buffer`), whose glyphs carry font ids, glyph ids,
and positions; the fonts' bytes come from COSMIC Text's font database. A
renderer of the app's own that writes each call into a krilla page draws
the PDF with the same code that draws the screen: no second drawing of
text, frames, tables, bullets, rules, or pictures. Pictures are the
exception worth making: the surface hands the renderer a picture already
rasterized at the screen's size, so the PDF renderer is given the
picture itself (its bitmap at its own pixels, or the metafile's and
SVG's drawing as vectors).
