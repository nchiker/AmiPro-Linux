# Import: Ami Pro's Import, the Formats, and the Readers

**Written:** 2026-10-21, for the import run (the Phase 5 queue's run 7,
`docs/playbooks/OUTPUT.md`, "Run 3"). Evidence: Ami Pro 3.1 in the XP VM
opening and inserting fixtures written for this run (`../AmiPro-Reference/
vm-lab/IN/IMP`, results in `OUT/IM*.SAM` and `OUT/NT1.SAM`, outside Git);
the user's guide ch. 22; the decompiled `AMIPRO.EXE`, `AMIPROUI.DLL`, and
`rtf.flt` (output outside Git; routines named by address, nothing copied);
free sample documents (below); ECMA-376, ODF 1.3, and RTF 1.9.1. The export
side is `EXPORT_NOTES.md`.

## How Ami Pro imports (VM, guide)

- **No Import command.** File › Open has OK, Cancel, **Insert**, Close
  current file (checked), List files of type, and Preview; Insert is
  disabled while no document is open. OK opens the file as an **Untitled**
  document; an ASCII file keeps its name and is saved back as ASCII by
  Save (guide). Insert puts the file at the insertion point, in the main
  text, a selected text frame, or a table cell (a text file goes into the
  current cell), and leaves the insertion point where it was (guide; VM
  IMX).
- **Import Options** (VM, IM1.RTF): Paragraph Styles: Convert styles
  (preselected), Apply styles, Keep style names, Ignore styles. On IM1
  (Normal, heading 1, Body Text redefined as Arial 14 with 6 pt after, My
  Style, Title; one paragraph Body Text set right by direct formatting;
  one paragraph with no style, which RTF reads as style 0, Normal):

  | Option | Styles of the result | Paragraphs |
  |---|---|---|
  | Convert (IMC) | the file's styles added with the file's definitions; same-named sheet styles **redefined** (Body Text Arial 14, 6 pt after; Title centred) | keep their styles; direct formatting kept |
  | Apply (IMA) | the sheet's only, unchanged | every paragraph Body Text, its style's look as direct formatting (alignment marks, `<:I…>`, `<:S…>`, `<:f…>` face, size, and colour, attribute marks) |
  | Keep style names (IMK) | the sheet's only, unchanged | tagged with the file's names; Title takes the sheet's Title, the names the sheet lacks are missing styles; direct formatting kept |
  | Ignore (IMI) | the sheet's only, unchanged | every paragraph Body Text; only direct formatting kept |

  Title came in at 12.8 pt where the file said 14 (a quirk of the filter,
  not copied).
- **Page set-up** (all four): the RTF's paper and margins become an
  inserted page layout named `1` and the document starts in it (`[l1]` 1);
  Standard stays the style sheet's. The results were saved with Keep format
  with document (styles in the file, `[sty]` empty).
- **Insert** (IMX: a new `_DEFAULT.STY` document, a typed paragraph and an
  empty one, then Open › Insert IM1.RTF with Convert): the inserted
  paragraphs follow the caret's paragraph, which gets a page break into the
  file's page set-up (`<:p…>` with inserted layout `1`); an empty Body Text
  paragraph follows them; Convert redefined the document's own Body Text
  and Title, so the typed paragraph changed too.
- **ASCII Options** (a button in Open while ASCII is the type): ASCII File
  Options: CR/LF at lines (preselected) or CR/LF at paragraph ends only;
  Keep style names (off); ASCII File Type: 7 bit ASCII, 8 bit PC-ASCII
  (preselected), 8 bit ANSI. No dialog appears unless the button is used.
  - CR/LF at lines (IMT, IM2.TXT): a single line end inside a paragraph
    became a space, a blank line ended the paragraph, a line's leading tab
    was dropped, a line starting with spaces joined the line before, and a
    second blank line gave no empty paragraph but a leading space.
  - CR/LF at paragraph ends only with Keep style names (IMT3, IM3.TXT):
    each line a paragraph, a blank line an empty paragraph, a leading tab
    kept, `@Title@` and `@Subhead@` at a line's start applied those styles.
  - PC-ASCII decodes code page 437: byte 0x82 came in as é; 0xE9 (Θ) as T,
    the nearest letter Windows ANSI has.
  - The document links to `_DEFAULT.STY`.
- **Notes** (NT1, for comments): Edit › Insert › Note writes
  `<:N` + time (Ami Pro's time base), the author's initials, the note's
  colour (`COLORREF`, yellow 65535), and two fields of 1, a line end, the
  note's paragraphs separated by blank lines, and a lone `>`. The app keeps
  notes as legacy marks and writes them back as read, so a note built in
  this form is saved as Ami Pro saves its own.
- **Word for Windows 6.0 and WordPerfect 6.0** (guide): opened through a
  converter that writes `NAME.SAM` beside the file (asking before
  overwriting) and opens that.

## Ami Pro's import code (decompile)

- **Open and Insert** (`AMIPROUI.DLL` template 1031, `OPENDLGPROC`
  `10a0:0000`; Insert is control 25585, ASCII Options… 25586, shown for
  ASCII only). The types come from `AMIPRO.INI` `[Filters]` (`name=dll,
  kind, extension`; kind T text, X data, F converted by a macro).
  `AMIPRO.EXE` takes the file at `11c0:0175`; filter imports run through
  `10d8:07a0` and the import engine `1318:0000`. File › Import Text…
  (0x98) exists only in the HP NewWave menu (1003), as the same dialog
  inserting at the caret.
- **Opening** makes an Untitled document on the default style sheet
  (`AMIPRO.INI` `defstyle`, `_DEFAULT.STY`; `10d8:07a0` → `1040:0212`) and
  imports at its start. **Insert** (`10d8:09bb`) imports into the current
  document: into a selected text frame (`1318:7caa`; a data file makes a
  table there), into the cells from the caret's (`1318:804a`), or at the
  caret; new paragraphs take the caret paragraph's style (`1318:0704`,
  `1318:2f30`).
- **The import engine** (`1318:0000`) loads the filter, opens it with an
  exchange block (`1318:705e`: the ASCII options, the exchange level 2, an
  option string that skips the dialog, the clipboard's memory when
  pasting), and loops on `GETFILTERTOKEN`, dispatching each token
  (`1318:1d94`): main text with embedded codes (`1318:21b2`: paragraph end,
  page and column breaks, line spacing, attribute words, dates, tabs,
  alignment, tab rulers, cell moves, controls with data (page numbers, font
  changes, picture frames, column widths, joined cells, cell lines, row
  heights), footnote references, paragraph styles by name, table start and
  end, indents, floating headers and footers), header and footer text (into
  the layout's bands), footnote text, **style definitions** (`1318:774b`),
  and **page layouts** (`1318:78d8`: at the document's start the standard
  layout, elsewhere an inserted layout). Errors (`1318:73a5`): "File is not
  the correct type for the import filter.", "Cannot open file for
  import/export.", "Unable to import all of %s." (what arrived is kept);
  Cancel is silent. **Nothing tells the user what was dropped.**
- **Import Options** is the filter's own dialog (`rtf.flt` dialog 100,
  procedure `1000:94eb`), **Convert preselected every time, never
  remembered**; pasted RTF always takes Apply without asking. Convert
  sends every style before the text (`1000:42f3`); a name the document has
  is **redefined** (`10a8:00e2`), a new one starts as a copy of Body Text,
  takes what the RTF sets, and gets the next free function key. Apply sends
  no names and every style's look as formatting (`1000:43b8`); Keep sends
  names only (`1000:10ca`); Ignore neither. **Style names are cut to 13
  characters** (`1000:80cd`, `1318:21b2`). A paragraph without `\s` is
  style 0, named Normal. `\sbasedon` and `\snext` are ignored.
- **What `rtf.flt` reads** (295 keywords, `1000:9289`; dispatch
  `1000:0498`): styles, fonts, colours, character formatting (hidden text
  becomes protected text), indents, alignment, line spacing, tabs and
  leaders, sections and page set-up (paper, margins, columns, a line
  between them; defaults Letter and 1-inch margins, `1000:34f2`), page
  numbers, headers and footers (at a section's start into the bands,
  elsewhere as floating ones), tables (a pre-scan to the end of the
  enclosing group counts rows and cells, `1000:2d1c`; widths from the first
  row; cell lines), pictures (`\wmetafile`, `\wbitmap`), OLE objects,
  fields (PAGE, IMPORT as a linked picture, SYMBOL; any other field its
  result as text), dates, footnotes (renumbered; endnotes become
  footnotes), and the outline level. **Not read:** `\line`, `\-`, `\_`,
  `\u` (its fallback character instead), `\ansicpg` and `\fcharset`,
  space above and below, keep with next, and borders as direct formatting,
  joined cells, row heights, heading rows, cell fill, positioned
  paragraphs and shapes (their text comes in as text), PNG, JPEG, EMF, and
  DIB pictures, bookmarks, index and contents entries (their text comes
  in), lists (numbers as text), document information, revisions, and
  annotations.
- **ASCII** (`15e0:0000` for Open, `ascii.flt` for Insert): refused unless
  80% of the first 2 KB is printable; paragraph ends at every line end
  (`15e0:02c7`) or lines joined with spaces, paragraphs at blank lines, a
  word hyphenated at a line's end joined (`15e0:04f2`); Keep style names
  reads `@Name@` or `<Name>` at a paragraph's start (`15e0:0827`); 7-bit,
  OEM, and ANSI conversions (`15e0:0a35`); the choices are remembered
  (`AMIPRO.INI` `DefOptions2`, `1318:7339`).
- **Paste** of RTF from the clipboard runs `rtf.flt` in paste mode
  (`1578:09bf`), with Apply styles.

## Findings for the app outside import

- `SAM_FORMAT_NOTES.md` said the importer ignores `[l1]` (the layout a
  document starts in); it has read it since Phase 4.5 (`fd8a64d`,
  `Document::start_layout`), so Ami Pro's imports (IMC…) open in their
  layout. The note is corrected; the runbook's IM.9 is not needed.

## The formats' readers (libraries)

Evaluated 2026-10-21 (crates.io, the crates' sources, a probe over the
sample sets):

- **Zip:** `zip` 9.0.5 (MIT; with only `deflate-flate2-zlib-rs` it adds one
  crate to the app, but needs Rust 1.88 against the project's 1.85),
  `rc-zip`, `piz`. **Chosen: a reader of the project's own** in the core
  (about 150 lines: the central directory, stored and deflated entries,
  limits against zip bombs), inflate passed in by the caller as export
  passes deflate in; the app inflates with `flate2`. It must read the
  central directory: 607 of 694 sample ODT and 209 of 2,359 DOCX packages
  use data descriptors (sizes after the data), which the export's
  stored-entry test reader cannot read.
- **XML:** `quick-xml` 0.41 (in the app's lock only as a build-time
  dependency), `roxmltree` 0.20 (linked through `usvg`; strict, and it
  overflowed an 8 MiB stack on a sample with tables nested 5,000 deep),
  `xml-rs`. Over 41,491 XML parts of 3,118 sample packages the first two
  agree but for files that are not XML. **Chosen: a small pull reader of
  the project's own** in the core: namespaces through a scope stack, the
  predefined entities and character references, CDATA, comments and
  processing instructions skipped, DOCTYPE skipped (no entity is ever
  fetched), an explicit element stack with a depth limit, lenient on an
  undeclared prefix.
- **RTF:** `rtf-parser` 0.4.3 reads no tables, pictures, fields, lists, or
  headers. **Chosen: the project's own reader** in the core, beside its RTF
  writer.
- **DOCX, ODT:** `docx-rs` and `docx-rust` read the XML into structs but
  resolve nothing (style inheritance, themes, toggles, table-style
  conditions); no ODT reader crate exists. Not used.

## What other programs write (beyond the app's own exports)

From the sample sets (test suites, so rare features are over-represented)
and the specifications:

- **DOCX:** `docDefaults` and `basedOn` chains (resolved into each style);
  bold, italic, caps and the other toggles flip against the style's value;
  theme fonts and colours (`asciiTheme`, `themeColor`, in 75% of sample
  packages) resolved from `theme1.xml`; linked and character styles
  (flattened into runs: Ami Pro has none); numbering through styles,
  `lvlOverride`, and `startOverride`; a section's properties in the last
  paragraph's `pPr`, header and footer references inherited from the
  section before, section types; table styles with conditional rows and
  columns (resolved into each cell's lines and shading), `tblLook`, auto
  and percentage widths (resolved to a grid); `mc:AlternateContent` (one
  branch only), VML (`w:pict`, older text boxes), `wp:anchor` positions
  relative to page, margin, column, paragraph, or line; `w:framePr`
  (framed paragraphs, closest to Ami Pro's frames); simple and complex
  fields, nested; hyperlinks, bookmarks, content controls (`w:sdt`, their
  content kept); tracked changes (`w:ins`, `w:del`, `w:moveFrom`, property
  changes); comments; footnotes and endnotes (their separator notes
  skipped); `w:sym`; hidden text (`w:vanish`); EMF pictures.
- **ODT:** styles in `styles.xml` and automatic styles in `content.xml`;
  `_20_` in style names for spaces; a paragraph style's
  `style:master-page-name` as the page set-up's change; `text:s`,
  `text:tab`, `text:line-break`; nested `text:list` with two label-position
  modes and `text:continue-numbering`; `table:covered-table-cell` and
  `number-columns-repeated`/`number-rows-repeated`; frames anchored to the
  page, paragraph, character, or as a character; deleted text kept in
  `text:tracked-changes`, not in the body; `office:annotation`; flat `.fodt`
  (the same XML in one file); unknown namespaces (`loext`) ignored.
- **RTF:** a byte-level tokenizer (`\bin` carries raw bytes); `\ansicpg`
  and each font's `\fcharset` decide how `\'hh` decodes; `\uN` with `\ucN`
  skip counts; section properties after `\sectd`; `\cellx` gives the
  cells' right edges; `\clmgf`/`\clmrg` and `\clvmgf`/`\clvmrg` merges;
  nested tables (`\itap`, `\nesttableprops`); `\listtable` and
  `\listoverridetable` (and the old `\pn`), `\listtext` a copy of the number
  to skip; `\shp` with `\shprslt` and `\nonshppict` fallbacks (one read,
  the other skipped); ignorable destinations `\*`; `\loch`, `\hich`,
  `\dbch` runs.

## Test documents (outside Git)

`../AmiPro-Reference/import-samples/` (252 MB), each set with its licence
kept: LibreOffice core's `sw/qa` data (MPL-2.0: 1,714 DOCX, 1,371 of them
written by Word; 562 ODT; 345 FODT; 589 RTF), Apache POI (Apache-2.0), Apache
Tika (Apache-2.0), python-docx (MIT), mammoth (BSD-2-Clause), docx4j
(Apache-2.0), the ODF Toolkit (Apache-2.0; ODF 1.0 and 1.1 from Symphony,
StarOffice, OpenOffice.org), pandoc (GPL-2.0-or-later; tested locally only,
never copied into the repository), and four LibreOffice fixtures made for
the run. LibreOffice's fuzzing folders (`sw/qa/core/data/*/fail`, `pass`)
are the robustness gate: every file read or refused, none panicking.
No sample comes from Google Docs; WordPad appears only as about eight
rich-edit RTF files.
