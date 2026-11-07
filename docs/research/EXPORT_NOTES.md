# Export: Ami Pro's Filters, the Formats, and the Mapping

**Written:** 2026-10-20, for the export run (the Phase 5 queue's run 6,
`docs/playbooks/OUTPUT.md`, "Run 2"). Evidence: the decompiled `rtf.flt`
(stock Ghidra; output outside Git), strings and NE headers of the other
filters, Ami Pro 3.1 in the XP VM exporting VM fixtures (files in
`../AmiPro-Reference/vm-lab/EXP`, outside Git), the user's guide ch. 22,
LibreOffice 24.2 on the host, ECMA-376 (DOCX), and the RTF 1.9.1
specification. Routines are named by address; no decompiler output is
copied here.

## How Ami Pro exports

- **File › Save As** lists 33 file types under "List files of type": Ami
  Pro, Ami Pro Macro, ASCII, DCA/FFT, DCA/RFT, DisplayWrite, E-Mail, Enable,
  Exec MemoMaker, Manuscript, Microsoft Word, MultiMate, Navy DIF, Office
  Writer, PeachText, Professional Write, Q&A Write, **Rich Text Format**,
  Samna Word, SmartWare, Symphony Document, Wang (IWP), Windows Write,
  **Word for Windows 1.x, 2.0, 6.0**, WordPerfect 5.x and 6.0, and others.
  Choosing another type hides Keep format with document, Password protect,
  Ami Pro 1.2 format, and Document description (VM, EX1).
- **The document keeps its own name and file** after a Save As in another
  type: the title still shows `FC1.SAM` after writing `FC1W2.DOC` (VM).
  So Ami Pro's export writes a copy.
- **One stream, a filter per format.** Each text filter is a Windows DLL
  (`rtf.flt`, `wword20.flt`, `word.flt`, the Word for Word `w4w*.dll`)
  that exports `OPENEXPORTFILTER`, `PROCESSFILTERTOKEN`,
  `CLOSEEXPORTFILTER`, and `WANTSTYLE` for export, and `OPENIMPORTFILTER`,
  `GETFILTERTOKEN`, `CLOSEIMPORTFILTER` for import (`rtf.flt` `1000:43e0`,
  `451d`, `4572`, `94eb`). Ami Pro walks the document once and hands the
  filter a stream of tokens (text, attributes on and off, paragraph ends,
  styles, table rows and cells, header text, pictures); the filter pulls
  them through a callback (`PROCESSFILTERTOKEN` loops until the callback
  reports the end) and writes its own format. The same filter reads its
  format back into the same tokens. `AMIPRO.EXE` reaches the filters by
  ordinal through `GETPROCADDRESS`. Word for Word filters come in pairs:
  `w4w33f` (from Ami Pro to Word for Word's intermediate form) and
  `w4w44t` (to Word for Windows), so a format needs only its own half.
- **`rtf.flt`'s vocabulary** (its keyword table, read and written):
  styles (`stylesheet`, `sbasedon`, `snext`), fonts and colours, character
  and paragraph formatting, tabs and leaders (`tqr`, `tqc`, `tqdec`,
  `tldot`, `tlhyph`, `tlul`, `tlth`), tables (`trowd`, `cellx`, `trgaph`,
  `trleft`, `trrh`, `trqc`), sections (`sect`, `sectd`, `cols`, `sbkpage`,
  `titlepg`, `margmirror`, `paperw`, `paperh`), page numbers (`pgnstart`,
  `pgnucrm`, `pgnlcltr`, …), headers and footers, positioned paragraphs
  (`posx`, `posy`, `absw`, `phpg`, `pvpg`, `phmrg`, `pvmrg`, `phcol`),
  pictures (`pict`, `wmetafile`, `wbitmap`, `picscalex`, crop), OLE
  objects, fields (`field`, `fldinst`, `fldrslt`), footnotes, index and
  TOC entries (`xe`, `tc`), revisions, document information (`title`,
  `subject`, `nofpages`, `nofwords`, `nofchars`), and a private
  `lotusoutlinelevel`.

## What Ami Pro's RTF and Word exports keep (VM fixtures EX1–EX8)

Ami Pro exported VM fixtures to `L:\EXP` (`FC1`, `TB1`, `PI4`, `HDR1`,
`BL1`, `OL1` as RTF; `FC1` as Word for Windows 2.0):

| Feature | Ami Pro's RTF | Fixture |
|---|---|---|
| Paragraph styles | every style, with font, size, bold, italic, alignment, space above and below, left indent, keep with next, and `\lotusoutlinelevelN` | all |
| Character formatting | bold, italic, face and size runs | BL1 |
| Page set-up | left and right margins only (`\margl`, `\margr`); no paper size, no top and bottom margins | all |
| Header text | `{\header …}` with the band's paragraphs | HDR1 |
| Tables | `\trowd\trgaph43\cellx…` rows, cell text in Table Text | TB1 |
| Pictures | a bitmap written as a metafile (`\pict\wmetafile8`) that draws it, with its size goal, crop, and a hairline box | PI4 |
| **Frames** | **dropped** with their text, in RTF and in Word for Windows 2.0 | FC1, FC1W2 |
| Inserted bullets (`<:B…>`) | dropped | BL1 |
| Outline levels | the private `\lotusoutlinelevel`, which other programs ignore | OL1 |

Ami Pro reads RTF back through the same filter: File › Open with Rich Text
Format asks **Import Options: Convert styles, Apply styles, Keep style
names, or Ignore styles**, then opens the file as Untitled (its own TB1
export came back as a table with empty cells). So the app's RTF files can
be twin-checked by opening them in Ami Pro. The import options belong to
the import run.

**Conclusion.** Ami Pro's own exports keep the text, the styles, tables,
the header, and pictures, and drop frames. Frames are the heart of an Ami
Pro document, so the app's export goes further: the page layouts, frames,
and placements are written as the modern formats hold them.

## The formats

| | DOCX (ECMA-376, Office Open XML) | RTF 1.9.1 | Plain text |
|---|---|---|---|
| Container | a zip of XML parts | one text file | one text file |
| Read by | Word 2007+, LibreOffice, Google Docs, OnlyOffice, Apple Pages, WPS | Word (every version), LibreOffice, WordPad, Ami Pro itself, most others | everything |
| Styles | `styles.xml`: based on, next, outline level, the whole paragraph and character formatting | `\stylesheet` with the same | none |
| Page layouts | sections (`w:sectPr`): paper, orientation, margins, columns (unequal widths, a line between), header and footer distance, page borders, page-number format and start, first-page and even headers | sections (`\sect`, `\cols`, `\colsx`, `\linebetcol`, `\titlepg`, `\facingp`, `\pgbrdr…`, `\pgn…`) | none |
| Frames | DrawingML text boxes (`wps:wsp` with `wps:txbx`) and pictures (`pic:pic`), anchored (`wp:anchor`: to the page, margin, column, paragraph, or line, with square, top-and-bottom, or no wrap, front or behind, z-order) or inline (`wp:inline`); lines, fill, shadow, rounded corners, insets, text columns; older framed paragraphs (`w:framePr`) | shapes (`\shp`, `\shptxt`) with the same placements, plus a positioned-paragraph result (`\shprslt` with `\posx`, `\absw`) that older readers and Ami Pro read | contents in reading order |
| Repeating frames | a shape anchored in the header, positioned on the page (as watermarks are) | the same | — |
| Tables | `w:tbl` with column grid, joined cells (`gridSpan`, `vMerge`), row heights (exact or at least), cell margins, heading rows, rows that may not split | `\trowd`, `\clmgf`/`\clmrg`, `\clvmgf`/`\clvmrg`, `\trrh`, `\trhdr`, `\trkeep` | tab-separated |
| Lists | `numbering.xml` levels: formats (decimal, letters, roman), text with `%1.%2`, bullets with their font, restart rules | `\listtable`, `\listoverridetable` | numbers as text |
| Fields | `w:fldSimple` or `w:fldChar` fields with a cached result (PAGE, NUMPAGES, DATE, CREATEDATE, SAVEDATE, TIME, TITLE, KEYWORDS, COMMENTS, DOCPROPERTY, FILENAME, SEQ, EDITTIME, NUMWORDS) | `{\field{\*\fldinst …}{\fldrslt …}}` | results |
| Pictures | PNG, JPEG, GIF, BMP, TIFF, WMF, EMF, SVG (Office 2016+, with a PNG fallback) | `\pngblip`, `\jpegblip`, `\wmetafile8`, `\emfblip` | — |
| Properties | `docProps/core.xml` (title, subject, keywords, description, created, modified, revision), `app.xml` (pages, words, characters, total editing time), `custom.xml` (named fields) | `\info` (`\title`, `\keywords`, `\doccomm`, `\creatim`, `\revtim`, `\version`, `\edmins`, `\nofpages`, `\nofwords`, `\nofchars`, `\userprops`) | — |

**Also considered** (the user asked, 2026-10-20, which other formats to
weigh):
- **ODT** (OpenDocument Text, ISO/IEC 26300; LibreOffice's own format; the
  archival standard many public bodies require): a zip of XML parts like
  DOCX (`content.xml`, `styles.xml`, `meta.xml`, `manifest.xml`, the
  `mimetype` entry stored first). Its frames are `draw:frame` with a
  `draw:text-box` or `draw:image`, anchored to the page (`text:anchor-type
  ="page"` with a page number), a paragraph, or a character, with wrap,
  run-through (in front or behind), borders, shadow, padding, columns, and
  z-index: the closest match to Ami Pro's frames of any format. Lists,
  sections, page styles (one per page layout, with headers and footers,
  left pages), fields (`text:page-number`, `text:date`, `text:title`,
  `text:sequence`), and `meta.xml` properties map as in DOCX.
- **HTML** (a web page with its pictures): readable in any browser; holds
  styles (CSS), tables, lists, and pictures, but not the page design
  (frames become floats or blocks, pages vanish). Cheap to write; reading
  arbitrary HTML back is a large task.
- **Markdown, EPUB:** no page design; not proposed.
- **Lotus Word Pro (.lwp):** the user's corpus holds 82; a closed binary
  format; not writable.

## Libraries

- **DOCX writing:** `docx-rs` 0.4.22 (MIT) writes and reads DOCX and has
  text boxes, anchors, numbering, fields, comments, and properties, but
  only 5.5% of it is documented, and it brings its own zip, XML, and image
  stacks. **Not used:** the run needs an RTF writer of its own anyway (no
  maintained Rust crate writes RTF; `rtf-parser` only reads), both formats
  are written from one neutral form, and the XML is plain text a core
  crate writes without dependencies. The zip container is a small writer
  of the app's own (stored or deflated entries, CRC-32), with deflate from
  `flate2`, already in the app's build (through `png`).
- **Pictures:** the app's `image` crate encodes PNG (its `png` feature is
  already on); metafiles go in as WMF (Word and LibreOffice draw them);
  SVG goes in with a PNG fallback.
- **Checks:** LibreOffice 24.2 (`soffice --headless --convert-to pdf`, with
  a private profile in `target/` so the user's own LibreOffice is never
  touched) turns exported files into PDFs, which `hayro` (already a
  dev-dependency) renders beside the app's own PDF of the same document.
  LibreOffice is a check, not a reference binary.
- **ODT** needs nothing more: the same XML writing and zip writer (its
  `mimetype` entry stored, uncompressed, first).
- **For the import run:** `quick-xml` (MIT, already in the app's build) to
  read DOCX and ODT; an RTF reader of the app's own.

## The mapping (draft; decided in `UI_SPEC.md` §30 and DECISIONS EX…)

One neutral form (**the exchange document**, as Ami Pro's token stream):
the model is resolved once, Ami Pro's meaning worked out, and each format's
filter only spells it out. The import run reads each format back into the
same form.

- **Styles** keep their names. Body Text is the default paragraph style.
  Each style's font, size, attributes, colour, indents (left, right, first
  line, hanging), alignment, spacing above and below, line spacing (single;
  1½ and double as multiples; custom as exact), keep with next, keep
  together (Allow page break within off), page and column breaks before,
  Next style, tabs with leaders, rules above and below (paragraph
  borders), outline level, and bullets or numbers. A paragraph's own
  indents, line spacing, alignment, and tabs, and each character run, as
  direct formatting.
- **Character attributes:** bold, italic, single, word, and double
  underline, strikethrough, superscript, subscript, small caps, upper case
  (as caps formatting); lower case and initial caps written as shown (the
  formats have no such formatting); colour; overstrike as the character
  struck over; protected text and no hyphenation (no hyphenation as
  `w:noProof`/`suppressAutoHyphens` per paragraph where it covers one).
- **Page layouts** as sections: the document layout first, each inserted
  layout (`<:p…>` with a layout) a new section starting a page; paper,
  orientation, margins, columns (equal, or unequal widths), the gutter, a
  line between columns, lines around the page, mirror margins and left
  pages (even headers), Begin on second page (a blank first-page header),
  the header and footer bands' text and distances.
- **Page numbers** as PAGE fields with Ami Pro's style (1, I, i, A, a) and
  start; NumPages as NUMPAGES.
- **Breaks:** page and column breaks; a hard return as a line break; a
  discretionary hyphen as a soft hyphen.
- **Frames:** Where placed frames anchored to the page, in the first
  paragraph laid out on their page; With para above frames anchored below
  their paragraph; Flow with text frames inline in the text; repeating
  frames in the header, positioned on the page (right and left pages in
  the default and even headers); wrap around, no wrap beside, and no wrap
  around as square, top-and-bottom, and no wrap (in front, or behind for
  transparent frames); z-order from the frame stack; lines (style width,
  colour), shadow, rounded corners, fill or transparent, interior margins,
  columns; content: text (its paragraphs), a table, or a picture.
- **Tables:** the column grid from Ami Pro's widths and gutters, joined
  cells, row heights (Automatic as at least, else exact), cell margins,
  heading rows repeated, rows that may not span pages, page-break rows;
  pictures in cells inline in their cell; table lines and shading if the
  run maps them (EX question).
- **Lists:** each list style's bullets or numbers as a list with Ami Pro's
  levels; numbers restart as Ami Pro's reset options say; cumulative
  numbers as `%1.%2.%3`; Ami Pro's picture bullets as the nearest
  character; inserted bullets (`<:B…>`) as their character.
- **Fields:** dates and Doc Info entries as the formats' own fields with
  Ami Pro's format and current result; power fields with a counterpart
  (Seq, FileName, NumPages, Now, CreateDate, EditDate, Words) as that
  field, the rest as their result text.
- **Pictures:** bitmaps as PNG, JPEG and GIF as they are, metafiles as WMF,
  SVG with a PNG fallback, TIFF and PCX as PNG; drawings, charts, and OLE
  objects as Ami Pro's screen image; scaling, cropping, and rotation as
  the frame shows them; linked pictures embedded.
- **Doc Info:** the description, keywords, the four named fields (custom
  properties), created and saved times, revisions, editing minutes, and
  the statistics.
- **Kept only in the app's formats** (listed by the loss check): notes,
  footnotes, bookmarks, and other legacy controls until their runs; Ami
  Pro's printer line heights; frame groups; protected text; macros.

## Table lines and shading (the export run's EX5, 2026-10-20)

Ami Pro's Table › Lines & Color… (`TBLLINESPROC`, `AMIPROUI.DLL`
`13a8:0000`) offers Line position All, Left, Right, Top, Bottom, and
Outline, one of 12 line styles, and Fill color from the palette. **It has
no line colour: table lines are black.** The dialog hands Ami Pro the
checked positions (bit 1 All, 2 Outline, 4 Left, 8 Right, 16 Top, 32
Bottom, 64 Fill), the line style (1–12), and the fill colour
(`13a8:0303`); `14b0:067c` applies them to the selected cells and
`14b0:09c8` widens a gutter that is narrower than the line.

**Storage** (the cell record `ROW COL F3 … F12`; VERIFIED (decompile)
`13e8:1232`, VM TL1, TL4; CORPUS 24,566 cells):

- F7 is the cell's line word: four line styles, left in bits 0–3, right
  4–7, top 8–11, bottom 12–15 (All with style 1 is `0x1111`, 4369; the
  corpora's `0x1100` is top and bottom, `0x1000` bottom only). On a joined
  block the top and bottom styles apply along its first and last rows and
  left and right along its outer columns.
- F6 is the fill colour's place in the palette (0 for none) and F10 the
  colour as a `COLORREF` (TL4: 9 and 65535, yellow); the corpora's fills
  are greys (`0xF7F7F7`, `0xEFEFEF`). F6 and F10 are set in exactly the
  same 1,570 cells. F3's bit `0x200` comes with a fill in TL4 but is set
  in 1,567 other cells without one: its meaning is open (kept).
- Applying lines adds `[h]` records with flag 2 for the rows (kept).

**Where they are drawn** (VM TL1 and TL4 printed to PostScript, measured
at 600 dpi): each line is centred on the cell's box, which runs from the
middle of the gutter before the cell to the middle of the gutter after it;
before the first column and row, from the middle of the table's inset. On
TL1 (the table at 1,440 × 2,291 twips, 43-twip insets, 86-twip gutters)
the vertical lines are centred at 1,461, 6,095, and 10,750, the
horizontal ones at 2,312, 2,661, and 3,030; TL4's fill covers 1,462 to
10,747 across, the boxes exactly. A style's strokes are the paragraph
rules' (`rule_strokes`: 1 is 10 twips, 8 a double line).

The app reads them from the kept record (`TableCell::lines`, `fill`), so
no file or model changes; the page surface draws them (and so printing and
the PDF), and every export writes them. The app's PDF of TL1 and TL4 puts
the lines and the fill where Ami Pro's printouts do, to the twip.

## The app's exports in Ami Pro and LibreOffice (2026-10-20)

- **Ami Pro 3.1 opens the app's RTF** (File › Open, Rich Text Format,
  Import Options › Convert styles): the text, styles, character
  formatting, tables, and their lines. Its reader takes `\b0`, `\i0`, and
  `\caps0` for **on**, so the RTF writer states only what is on, each run
  a `{\plain …}` group (twins ATL1 before, ATL1B and AF01 after). It reads
  a frame's positioned-paragraph result (`\posx`, `\absw`) as a paragraph
  before the main text, keeping the frame's text but not its place, and
  a header shape's text as header text (AF08). It cannot show PNG
  pictures (`\pngblip` is newer than its reader).
- **LibreOffice 24.2** opens all three formats. Found while comparing
  (each fixed in the writers): it does not take a picture crop that
  reaches past the picture (a box holding the picture instead); it reads
  a header's automatic styles only from `styles.xml`; it shrinks an RTF
  rounded rectangle holding text to its text (a text box instead); it
  falls back to a wide sans for font names fontconfig does not know
  (today's names instead); its single lines are a little tighter than Ami
  Pro's printer lines (at least those lines instead).
