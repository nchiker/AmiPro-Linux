# Known limitations

What Ami Pro for Linux does not do, does differently, or has not been
checked on a real machine. Last updated 2026-11-07. The evidence behind each
feature is in `FEATURE_PARITY.md`; this page is the user-facing summary.

## Not built, and not planned

- Revision marking, Revision Insertion, and Doc Compare. Files that carry
  revision data keep it untouched, but it is neither shown nor edited.
- Macros (Quick Record, Record, Playback, Edit) and Ami Pro's `.SMM` macro
  files. Deferred; no date is set. A document that names a macro on New
  opens without running it.
- Live OLE editing: OLE objects are drawn and kept, and Convert to Picture
  works, but Update, Unlink, Edit, Paste Link, and New Object are not
  offered.
- Printer Setup: printing goes through the desktop's print portal, which
  chooses the printer.
- Window › Tile and Cascade: tabs replace them.
- File Management, Notes/FX, mail, and DDE.
- Ami Pro's other import and export filters (WordPerfect, Word 1 and 2,
  WordStar, Write, spreadsheet filters beyond the data files listed below).

## Not built yet

- Window › New View (a second view of one document) and the Vertical Ruler.
- Draft mode does not draw frame markers.
- Help covers the main areas in a starter set of topics (F1, Help
  Contents, Search Help…); not every command has its own topic yet, and
  dialogs that are not rail panels have no F1 context.
- Some picture import filters: CGM, HPGL, DrawPerfect, Freelance, PIC,
  AutoCAD, EPS, WPG. BMP, WMF, EMF, PNG, JPEG, GIF, SVG, and Ami Pro's
  drawings are read; TIFF and PCX are converted when imported.
- Table: Paste and Cut into a formula cell are not refused yet; Sort does
  not give Ami Pro's "DDE" and "too large" refusals.
- Merge: Ami Pro's odd date parsing results, and dBASE memo fields.

## Layout differences from Ami Pro

- A word wider than the gap beside a frame goes below the frame; Ami Pro
  breaks the word there.
- A Wrap around frame beside the first lines of a column adds the space above
  it in Ami Pro even with "When not at break"; here it does not.
- Pagination follows Ami Pro's rules and was compared with the original on
  sample documents, but line breaks can differ by a word in places because
  the text is shaped with the system's fonts, not Ami Pro's.

## Charts and equations

- Charts: the data and the twelve chart types are read, written, and drawn;
  every chart in the test corpora writes back equal. The Pictures dialog,
  reordering colours, and a live link to a table are missing.
- Equations: Ami Pro's TeX dialect reads and writes the same bytes as Ami
  Pro for the 27 test equations. Layout follows TeX's rules fitted to Ami
  Pro's sizes (mean error about 2 %), so it will differ slightly. About 330
  of Ami Pro's 887 symbol names are known; an unknown control word is
  reported as an error.
- Image Processing: Brightness, Contrast, Edge Enhancement, Smoothing, Invert,
  Halftone, and Revert are available; the smoothing and edge filters are
  inferred, not copied from Ami Pro.

## Accessibility and input

- The page surface exposes no accessibility nodes for page text; a screen
  reader cannot read the document. This is blocked by the toolkit version in
  use.
- Input methods: preedit text is drawn at the caret and tested automatically,
  but not yet checked with a real input method.
- Ctrl+Tab is taken by the desktop; Next Document is Ctrl+PageDown.

## Formats

- Ami Pro files: SAM and STY are read and written (Save writes over a `.SAM`
  after a loss check and keeps `NAME.SAM.bak` the first time). Password
  protected SAM files open with their password and save protected.
  Imported files and STY files are never written over.
- Exchange: DOCX, ODT, RTF, Markdown, and plain text open, insert, and
  export; PDF exports (and prints) through the print portal. Data files
  for merge and Sort: Ami Pro's own, tables, DIF, fixed, comma, CSV, TSV,
  DBF, XLSX, and ODS.
- Word Pro (`.LWP`) files are not read. Frames, layouts, and some page
  controls are lost when exporting to other word processors, as in Ami Pro.
- Compatibility matrix: `docs/COMPATIBILITY.md`. Format register: `docs/research/FORMAT_COMPATIBILITY.md` and
  Help › Compatibility and Fidelity.

## Not yet checked on a real machine

Built and tested automatically, but waiting for a physical check: table data
(formulas, Quick Add, Sort), objects (OLE, Paste Special, charts, equations),
Markdown, the Documents folder, the update check against a later release,
crash recovery after a killed process, First line bold, and the status bar's
Face and Size lists.

## Release

Packaged as a `.deb` for amd64 (`docs/INSTALL.md`); other distributions build
from source. The repository is public under the MPL-2.0.
