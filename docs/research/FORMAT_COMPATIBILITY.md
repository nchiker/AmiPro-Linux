# Format Compatibility Register

Private filenames identify local evidence only; no fixture bytes are committed.

| Fixture | Evidence | Import | Edit | Export | Fidelity / limitation |
|---|---|---|---|---|---|
| 160-file private SAM corpus | Ami Pro/companion format-version-4 documents from verified media | 160/160 import; 28,748 main-text paragraphs; 11,152 runs; 774 frames, 170 layouts, 223 anchors resolved and 0 unresolved, 34 floating headers/footers, 16 page breaks, 72 embedded payloads; 0 control leaks (2026-10-11, Phase 2.4) | Text, styles, runs, inline objects, frames, and layouts survive native editing/undo | Native `.amipro` v3; 160/160 exact round trips | PARTIAL: pages and frames import (see "Phase 2.4 mapping"); 36,846 unmapped inline controls kept as legacy marks; never overwrites SAM |
| 63-file private STY corpus | Shipped Ami Pro format-version-4 style sheets | 63/63 bounded parses; `[tag]`/`[fnt]` mapped | Imported definitions can be applied in native model | Native document only | PARTIAL: original STY output disabled; detailed alignment/spacing semantics pending |
| `README31.SAM` (`301d655f…`) | Ami Pro 3.1 release document, format version 4 | Editable text, 8 embedded named styles, untagged Body Text, escapes, and paired bold/italic controls import; Body Text and escape display verified against Ami Pro 3.1 | Text/paragraph style/character-run editing supported | Separate native `.amipro` | PARTIAL/LOSSY: CP-1252 decoded; frames, unsupported controls and embedded bitmap omitted with report |
| VM lab fixtures (T01–T05, F01–F13, L01–L15, C01–C04, M01–M04, P01–P03, Z01–Z06, X01–X05, G00–G23, TB1, V01–V06) | Written by original Ami Pro 3.1 in the XP VM from typed input, or synthetic files it read back; private, `../AmiPro-Reference/vm-lab/` | Text and styles import; frames, layouts, tables, pictures reported | — | — | Format evidence for `SAM_PAGE_FRAME_SPEC.md`; not yet import tests for pages/frames (Phase 2.4) |
| `_DEFAULT.STY` (`dfade436…`) | Shipped default style sheet, format version 4 | 9 named styles and font records map | Definitions usable in native model | Original STY output disabled | PARTIAL: alignment, detailed spacing, numbering and layout pending |
| Synthetic native document | Independently authored | Loads v1, v2, and v3 | Whole model supported | Saves v3 atomically (`docs/Design/NATIVE_FORMAT_V3.md`) | TESTED for exact model round trip, malformed input, and generated documents; format is project-native, not SAM |
| `crates/ami-formats/tests/fixtures/pages-and-frames.sam` | Synthetic, hand-written from the specification | Every page and frame structure imports (test) | — | v3 round trip exact | TESTED |

Original SAM/STY overwrite remains disabled. A discovery parse is not a semantic compatibility claim.

## Phase 2.4 mapping (SAM import, read-only)

What the importer maps into the model, what it keeps as legacy data with
the object it belongs to (for a future writer), and what it drops as a
cache. The specification is `SAM_PAGE_FRAME_SPEC.md`; the import report's
`structure` counts and warnings say what happened in each file.

| Source | Mapped | Kept as legacy data | Dropped (cache) |
|---|---|---|---|
| `[lay]` | name; paper, landscape, mirror, header/footer from second page; right and left pages | unknown number bits; unreadable `[rght]`/`[lft]`; unknown sections | — |
| `[rght]`/`[lft]` | size, margins, columns, gutter, per-column tabs, balance, line between columns, lines around the page | values 3 and 6; unknown tab kinds; extra values | — |
| `[hrght]` etc., `[repfrm]` | header, footer, and repeating frames in their slots | extra frames in a filled slot | — |
| `[frm]`/`[lyfrm]` | page, flags (placement, wrap, opacity, groups, content kind), rectangle, line sides, style, colour, and position, shadow, corners, background, `[frmname]`, `[frmmac]` | field 8; unknown flag bits; extra fields; unknown sections; unreadable records (whole) | sequence number (field 13), anchor index (14), Flow with text x offset (15); field 16 of table frames (stale; their size is the rectangle, since 2.5d) and field 16's height for With para above frames (stale; the rectangle's height, 2026-10-12) |
| `[frmlay]` | margins, columns, gutter, tabs, balance, line between | values 3 and 6 | bottom edge and width (derived), values 12–14 |
| `[txt]` | the frame's paragraphs | — | — |
| `[tbl]` | rows, columns, row heights, column widths, cells and their text; since 2026-10-19 header value 7's Automatic (1) and Rows span pages (`0x100`) and the `[h]` flags heading (`0x10`) and page break (`0x20`) (`Table::row_options`, `TABLE_PAGES_NOTES.md`) | header, `[h]` and `[w]` records, each cell's record, a calculated cell's formula, cells outside the grid | — |
| `[isd]` + `[Embedded]` | picture source (copied bytes or linked path), extension, Ami Pro's rendering | the `[isd]` values | offsets, trailer |
| `<:A n>`, `<:t n>` | frame anchors (one numbering over anchored records) | marks that name no record (none in the corpus) | — |
| `<:p…>`, `<:c>` | page breaks (centre, named layout), column breaks | — | — |
| `<:H…>`, `<:h…>` | floating headers and footers with their own flow | — | — |
| `<:P…>` | page-number field (any flow) | — | — |
| `<**>`, `<*7>` | hard return (U+2028), discretionary hyphen (U+00AD) | other `<*X>` bytes | — |
| `<:I…>`, `<:S+…>`, `<+@>`…`<+C>` (2026-10-12) | the paragraph's own indents, line spacing, and alignment (`ParagraphFormat`; O2, O3, O6, O7) | `<:S->` (no effect, O5); malformed ones | — |
| `<+X>`, `<-X>` for `!"#$)%&'+,-(` (2026-10-12) | bold, italic, underline, word and double underline, strikethrough, superscript, subscript, and the four caps as character runs (O1, O6; decompile) | protected text `<+*>`, no hyphenation `<+.>`, and marks Ami Pro ignores, as legacy marks | — |
| `<:fSIZE,KFACE,R,G,B>`, `<:f>` (2026-10-12) | the run's face, size, and colour; `<:f>` returns to the style's (O4, O5; decompile) | — | K, the face's pitch and family (Windows' substitution hint) |
| `[fnt]` values 3 and 4 (2026-10-12) | the style's colour, bold, italic, underlines, strikethrough, scripts, and caps | protected text (not kept) | the face's kind bits |
| other inline controls | — | legacy marks (zero-width, raw text) | `<:#…>` paragraph metrics |
| other top-level sections | — | document legacy data, in order | `[pg]` page table |


## Phase 4 inventory: what the SAM writer needs (2026-10-13, slice 4.2)

What the importer drops, derives, or keeps that a writer needs, and the
decision for each, from Ami Pro's writer (`SAM_FORMAT_NOTES.md`, "Writing
documents") and the cache fixtures K1–K6. "Keep" means the importer keeps
the value as read and the writer writes it back while the thing it belongs
to is unchanged; "derive" means the writer computes it; "omit" means it is
not written. Counts are over both corpora (576 readable SAMs).

| Item | Importer today | Decision | Slice |
|---|---|---|---|
| `[pg]` page table | dropped (cache) | omit: Ami Pro rebuilds it from the paragraph caches (K1Q, K5Q) | done (4.3) |
| `<:#H,W>` paragraph caches (42,081) | kept with each paragraph and a fingerprint (native `pcache`); heights calibrate line heights | keep with each paragraph and write while the paragraph, its style, and its column are as read; never compute one (K1W: Ami Pro trusts a wrong one) | done (4.9) |
| Header sections (`[prn]`, `[desc]`, …) | kept raw, in order; since the fields run `[desc]` and `[fldnames]` as the document's Doc Info (`DocumentInfo`), their places kept empty | write from the kept values in the writer's order; `[desc]` from the Doc Info with this save's time in Ami Pro's time base (the wall clock read as US Pacific time), the revisions one more than read, the session's minutes added, and the statistics counted for the save; sections 3.1 always writes added with Ami Pro's defaults where an older file lacks them | done (4.3; the fields run) |
| Unknown top-level sections (`[panosefonttable]`, 9 files) | kept raw, in order | write back after the section that preceded them | done (4.3) |
| `[tag]` and `[elay]` raw text | kept raw (until 4.2) | not kept: the styles (`writer.rs`) and the layouts hold them | 4.2 |
| Styles stored in the document | typed (origin, `StyleKept`) | write the document's styles in function-key order, then in the order read; all styles when the document has no style sheet | done (4.3) |
| `[l1]`, the layout the document starts in (27 files name an inserted layout) | kept raw, not used | type it: the first page's layout, laid out and written; measure layout | done (4.5) |
| A sheet's Standard layout when the SAM stores only inserted layouts (7 files) | dropped | keep the sheet's Standard first; measure layout | done (4.5) |
| Which layouts come from the sheet | one flag for all (`layout_from_sheet`) | the standard layout written when it is not the sheet's or has changed since read (a fingerprint); inserted layouts always | done (4.5) |
| Inserted layouts' names | kept as read | derive: Ami Pro renames them `1`, `2`, … as it writes them (`11c8:3ba0`), and the page breaks name them so | done (4.5) |
| Frame flags | typed; unknown bits kept (0x200 derived since 4.5: exactly the frames with text, 2,989 of 2,989) | derive 0x200 (text), 0x80000 (anchored), 0x4, 0x2000_0000, groups from the model; write kept bits | done (4.6) |
| Anchored frames' rectangle (fields 1, 3–6; 847 frames: 713 With para above, 134 Flow with text) | page kept; rectangle used for the size only | keep the read rectangle and write it while the importer's rules give the frame's size from it; otherwise keep its corner with the frame's size (Ami Pro places anchored frames from their anchor and uses only the size, X02) | done (4.6) |
| Field 7 line sides, flag 0x10000 (lines on) | typed with the bit, as the decompiled reader takes them (since 4.10; field 7 kept where it would differ) | derive the bit from the sides | done (4.10) |
| Fields 8, 13, extra fields | kept | keep; field 13 (sequence) assigned to new frames as Ami Pro does | done (4.6) |
| Field 14 anchor index | dropped | derive from the anchor order (anchored records without a mark included) | done (4.6) |
| Field 15 (Flow with text x offset), field 16 (anchored size; stale for tables and With para above heights) | partly typed, partly dropped | keep as read while the importer's rules give the model's values from it; derive otherwise | done (4.6) |
| `[frmlay]` values 1, 2, 7 | derived; value 1 kept as an offset where it is not the frame's bottom (353 of 406 tables) | derive, giving value 1's offset back | done (4.9) |
| `[frmlay]` values 3, 6, 9 bits, 10–14, column edges | kept | keep (`sheet.rs` writes them) | — |
| A frame's unknown parts (`[chint]`, `[ehint]` before `[frmname]`, 4 frames; `[mf]` in place of `[isd]`, 2) | kept, without their place among the parts | keep the parts' order too (`frame.parts`); a frame without a content part is written without one | done (4.6) |
| `[txt]` with no paragraph against one empty line | kept | keep | — |
| `[tbl]` header, `[h]` and `[w]` records, cell records, a cell's formula | kept where the writer would write them otherwise (since 4.11) | keep; a cell's eighth value derived from its text (`1490:07c0`); header value 7's bits 1 and `0x100` and the `[h]` flags `0x10` and `0x20` from the model (since 2026-10-19, a new row record in Ami Pro's shape; twins ATP2, ATP5, ATP6) | done (4.7, 4.9; row options 2026-10-19) |
| `[e]`, ending each group of a table's records | skipped | derive | done (4.7) |
| `[isd]` values | values 4–11 and the pan typed (`PictureScaling`, `Offset`); the record kept as a cache, and older versions' records (no pan) | written as read while they still say the typed fields; otherwise Ami Pro's record from the model with the kept driver state, the second block, value 3, and the unit (a metafile keeps the driver's record of its size), or an empty state for a new picture (PIR1, PIR3) | done (2026-10-18, the pictures run) |
| Pictures' bytes and Ami Pro's renderings | kept (object store) | write verbatim | done (4.8) |
| Pictures Ami Pro cannot store (PNG, JPEG, GIF, TIFF, PCX, SVG inserted in the app; P8) and links to Linux paths (P2) | kept as inserted (native) | converted to 24-bit bitmaps at most 300 dpi at their size on the page, their resolution keeping Ami Pro's natural size; Linux links copied in; both listed by Save As | done (2026-10-18, the pictures run) |
| A picture's description (P7) | kept (native `picturedescription`) | not stored; listed by Save As | done (2026-10-18, the pictures run) |
| Ami Pro's screen image beside a picture | kept, drawn for kinds no reader takes (P4) | written while the `[isd]` record is current; dropped with a rebuilt one (a stale cache), Ami Pro draws it again | done (2026-10-18, the pictures run) |
| `[Embedded]` offsets, lengths, trailer | dropped | derive from the bytes written | done (4.8) |
| Linked pictures (`[files]`, 28 files) | kept (path) | derive `[files]` from the linked pictures | done (4.8) |
| Inline controls the model does not map (notes, index entries, …) | kept as legacy marks | write verbatim | done (4.4) |
| `<:Dx>`, `<:rx>`, `<:kx>`, `<:b…>`, `<:d…>` (the fields run) | `InlineObject::Date`, `InlineObject::DocInfo` | the letter and the argument bytes escaped as Ami Pro's writer escapes them (a tab as it is) | done |
| `<:XK,V;CODE>` … `<:X~K,V;CODE>` (the fields run) | kept marks read by `PowerMark`; the result ordinary text; Seq (`<:X4,…>`) an `InlineObject::Seq`, its cached `<:X~4;N>` dropped | written as read; a field's start mark after the formatting that starts with it; Seq's mark and its number | done |
| `<:f…>`'s face kind byte | kept per face (document face kinds) | derive from the face kinds | done (4.4) |
| Physical line breaks, empty-paragraph forms | derived on reading | derive (`body.rs`, Ami Pro's 65- and 256-byte rules; a note's closing `>` ends its line, `11b0:0660`) | done (4.3, 4.9) |
| Frames in table cells (flag 0x20; 108 pictures in 23 files) | in their cells (`Placement::InCell`, `TableCell::frame`): a cell record's values 11 and 12 name the n-th frame of Ami Pro's list (`CELL_FRAME_NOTES.md`); one no cell names stays Where placed | written first, in Ami Pro's order, each cell numbering its frame; the frame at its cell's size, its page and corner as read (Ami Pro recomputes them on opening) | done (2026-10-18, the frames in table cells run) |
| No-op formatting controls Ami Pro wrote (`<-!>` with nothing on, `<:f>` repeated), the per-control face-kind byte, a run's control order | not kept | not written: the formatting reads back equal | — |

**What the writer cannot write exactly**, which the loss check
(`amipro_formats::sam_losses`, decision B) reports, by kind: it writes
the document, reads the result back with a sheet written from the same
document, and compares the two without their IDs
(`amipro_formats::roundtrip`). Some of it can never be written: characters
Windows-1252 has no code for (written as `?`, as Ami Pro's own writer
does), and formatting Ami Pro has no control for (normal caps over a
style's caps, an attribute switched off over a style that has it). The
rest shrinks as slices land; the round-trip audit
(`examples/sam_round_trip.rs`) measures it on both corpora.

**At the end of Phase 4 (2026-10-13, `bd53c98`).** The round-trip audit:
the user's corpus reads back equal in **416 of 416** documents (line
heights from the caches included), the ISO corpus in 159 of 160 (the
other has a malformed layout the importer keeps raw), the VM fixtures in
136 of 136; no read-back raises a warning its original did not. Sections
equal to the original's byte for byte: 50,631 of 52,749 (user's), 8,205
of 8,443 (ISO), 13,833 of 13,835 (VM); the rest differ by `[desc]`'s
bookkeeping, `[pg]` (omitted), and the body controls in the row above.
The largest corpus document (8.6 MB) writes in about 8 ms. In Ami Pro
3.1 (VM): 24 text fixtures, 48 layout screens, 57 frame fixtures, the
table and picture fixtures, README31, MERCURY, and two user documents
written by the writer open as their originals do (pixel-identical start
and end screens but for the caret), and Ami Pro saves them back with the
same page counts and page starts; documents made and edited in the app
open as made. What the loss check can still list for a document read
from a SAM is what the model cannot hold yet (frames in table cells), and
for any document, characters Windows-1252 lacks.

## The row cache (2026-10-20)

Nothing new in SAM files: Ami Pro's `[h]` row heights are read as before
and written as stored. The model marks each Automatic row whose stored
height is Ami Pro's measure as read (`Table::read_rows`, the row's
fingerprint at import), and native v3 keeps the marks (`readrow`,
`docs/Design/NATIVE_FORMAT_V3.md`). Ami Pro measures every Automatic row
when it opens a file (VM RC1–RC3), so a row the app grew or shrank needs
no new height in the file.

## Find & Replace (2026-10-20)

Nothing to store: replacements are ordinary edits, and a Replace All that
joins and splits paragraphs and sets attributes writes to SAM with nothing
in the loss check (`sam_after_edits.rs`,
`a_replace_all_writes_and_reads_back_with_nothing_lost`). The options and
recent searches are the app's settings, not the document's.


## Print and PDF (2026-10-20)

Nothing new in SAM files. `[prn]` and `[port]` (the document's printer and
port, which Ami Pro lays the document out with) are read and written as
before; the desktop's print dialog chooses the printer, so the app never
changes them. Ami Pro keeps its print options for the session only, and
so does the app (`PrintOptions` on the session); the PDF's additions are
the app's settings. The PDF is an output only: a PDF/A-2b, when asked, is
validated by krilla as it is written.

## Export (2026-10-20)

The export run (`docs/EXPORT_PROGRESS.md`, `EXPORT_NOTES.md`) writes the
model in other programs' formats through one neutral exchange document
(`amipro-exchange`), as Ami Pro's filters did. Nothing new is stored in
SAM or the native format. What maps where:

| Ami Pro | DOCX | ODT | RTF | Text |
|---|---|---|---|---|
| Paragraph styles (names kept, Body Text the default) | `w:style`, `w:basedOn`, `w:next`, `w:outlineLvl` | named paragraph styles, `default-outline-level` | `\stylesheet` | — |
| Runs, paragraph formats | `w:rPr`, `w:pPr` over the style | automatic styles | full formatting per run (`{\plain …}`) | lower case and initial caps as shown |
| Page layouts | sections (`w:sectPr`) | master pages and page layouts | `\sect` | — |
| Header and footer bands, floating ones | header and footer parts (default, even, first) | `style:header`, `-left`, `-first` | `\header`, `\headerl`, `\headerr`, `\headerf` | left out |
| Frames | DrawingML text boxes and pictures, anchored | `draw:frame` | `\shp` with a positioned-paragraph result | after their anchor's paragraph |
| Tables (page tables and With para above in the text) | `w:tbl` with spans, heights, heading rows | `table:table` | `\trowd` rows | tab-separated rows |
| Cell lines and fill (Lines & Color, read from the kept record) | `w:tcBorders`, `w:shd` | `fo:border-*`, background | `\clbrdr*`, `\clcbpat` | — |
| Bullets and numbers | `numbering.xml` instances | `text:list` continuing its instance | `\listtable`, `\listoverridetable` | as shown |
| Dates, Doc Info entries, page numbers, power fields with a counterpart, Seq, index and contents entries | Word fields with Ami Pro's result | ODF fields | `\field` | results |
| Pictures | PNG, JPEG, WMF, SVG with PNG | the same, `Pictures/` | `\pngblip`, `\jpegblip`, `\wmetafile8`, `\dibitmap0` | — |
| Doc Info | `docProps/core.xml`, `app.xml`, `custom.xml` | `meta.xml` | `\info` | — |

Since the notes run (2026-10-26): footnotes and endnotes (DOCX
`footnotes.xml`/`endnotes.xml` with `footnotePr`, ODT `text:note` with
`notes-configuration`, RTF `\footnote`/`\ftnalt`), notes as comments (DOCX
`comments.xml`, ODT `office:annotation`, RTF `\annotation` with
`\atndate`), and bookmarks (`w:bookmarkStart`, `text:bookmark-start`,
`\bkmkstart`) are written, and read back as Ami Pro's notes, footnotes,
and bookmarks.

Not written (listed before export): protected text as such, overstrike,
frame groups, left pages unlike mirrored
right pages, text tightness, short paragraph rules, column balance; in ODT
and RTF square corners on frames holding text.

## Import (2026-10-24)

The import run (`docs/IMPORT_PROGRESS.md`, `IMPORT_NOTES.md`) reads other
programs' formats into the same exchange document, one reader per format,
and makes the model from it in one step (`realise`). Nothing new is stored
in SAM or the native format: comments become Ami Pro notes (`<:N…>` as Ami
Pro writes them), footnotes and endnotes numbered paragraphs at the end.
What maps where:

| DOCX | ODT | RTF | Ami Pro |
|---|---|---|---|
| `w:style` (resolved through `docDefaults`, `basedOn`, toggles, themes) | named and automatic styles | `\stylesheet` | paragraph styles by Import Options; Normal or the default style Body Text (IM4) |
| `w:rPr`, character styles | text properties, character styles | run formatting | character runs (character styles applied as formatting) |
| a paragraph's own spacing, keep, borders | paragraph properties | `\sb`, `\sa`, `\keepn`, `\brdr*` | a variant style ("Body Text 2", IC5) or left out (Apply) |
| `w:sectPr` | master pages | `\sect` | page layouts (the first Standard) |
| header and footer parts | `style:header`, `-left`, `-first` | `\header*` | header and footer bands (a first page's: Begin on second page) |
| DrawingML and VML shapes, framed paragraphs | `draw:frame` | `\shp`, positioned paragraphs | frames: Where placed, With para above, or Flow with text (IC8) |
| `w:tbl` | `table:table` | `\trowd` | page tables (Automatic rows at Ami Pro's measure, IC32), frame tables, cells' pictures (IC33) |
| `numbering.xml` | `text:list` | `\listtable`, `\pn` | Ami Pro's list styles (IM5) |
| fields | fields | `\field` | dates, Doc Info, page numbers, Seq, index and contents marks; others their result |
| pictures (PNG, JPEG, GIF, BMP, TIFF, WMF, EMF, SVG) | the same | `\pngblip`, `\jpegblip`, `\wmetafile`, `\emfblip`, `\dibitmap` | picture frames (EMF drawn as metafiles) |
| `docProps` | `meta.xml` | `\info` | Doc Info |

Save As SAM of an imported document writes style names longer than Ami
Pro's 13 characters shortened, and lists them (IC31). Listed in the Import
Report, not carried: tracked changes (accepted), highlighting, character
spacing, hidden text, nested and floating tables (kept as text or in a
frame), charts and drawn shapes (their picture where the file has one),
equations, form fields, linked text boxes, right-to-left text.

## Table editing (2026-10-24)

The table editing run (`docs/TABLE_EDIT_PROGRESS.md`,
`TABLE_EDIT_NOTES.md`) gives the model what Ami Pro's table commands
write, read and written as Ami Pro writes them:

| Ami Pro | Model | Native v3 |
|---|---|---|
| `[tbl]` values 3 and 5 (default row height, column width) | `TableLayout::row_height`, `column_width` | `tablelayout` |
| value 7 `0x4`, `0x80`, `0x400` (Center, Honor protection, Disable mouse sizing) | `TableLayout::center`, `honor_protection`, `fixed_sizes` | `tablelayout` |
| `[h]`/`[w]` flag 2 (own size; none for an Automatic table's rows) | `TableLayout::own_heights`, `own_widths` | `tablelayout` |
| cell F7 (line word), F3 `0x200` with F10 (fill; F6 its palette index, recomputed) | `CellFormat::lines`, `fill` | `cellformat` |
| cell F3 `0xC000` (leader), F9 (protected) | `CellFormat::leader`, `protected` | `cellformat` |
| — (Ami Pro's lines are black) | `CellFormat::line_colour` (TD8; Save As writes black and lists it) | `cellformat` |
| frame `0x10000` with its line and margins on a page table | Line around table: the frame's border | (frame records) |
| frame `0x100000` (a table made in the text) | kept flag | (frame legacy) |

Records Ami Pro wrote keep their values; flag 1 of `[h]`/`[w]` (inside a
joined block) is written only in records the writer makes, and a row
record it makes counts the cells the row starts, as Ami Pro's.

## Drawings (2026-10-25)

The drawing run (`docs/DRAWING_PROGRESS.md`, `DRAWING_NOTES.md`) reads a
drawing's objects from the picture's bytes and writes them back:

| Ami Pro | Model | Native v3 |
|---|---|---|
| a `.sdw` (or `.sct`) picture's `[Embedded]` bytes: the object stream | `amipro_pictures::sdw::Drawing` read from the picture's object (the bytes stay the picture's) | the picture's object, base64 |
| `[isd]` driver state (384 words) | kept while the drawing is unedited; dropped (count 0) when Draw mode changes it, as Ami Pro rebuilds it (DRW0) | `isd` legacy |
| Ami Pro's screen image | kept while unedited; dropped when changed | the rendering object |

Unedited drawings are written byte for byte; edited ones as version 0x0102
streams with runtime links and handles 0, which Ami Pro reads and writes
back unchanged (ADR3S). Nothing is new for Save As's loss list.

## Notes, bookmarks, footnotes, endnotes (2026-10-26)

The notes run (`docs/NOTES_PROGRESS.md`, `NOTES_BOOKMARKS_FOOTNOTES.md`):

| SAM | Model | Native v3 | Written |
|---|---|---|---|
| `<:NTIME,INITIALS,COLOUR,NUMBER,MAIN` + paragraphs + `>` (and the short `<:NTIME`) | `InlineObject::Note` (`NoteInfo`, its flow) | `inline note` | as read; MAIN worked out; a cell's note with the cell's untagged style |
| `<:F` + paragraphs + `>` | `InlineObject::Footnote` (its flow) | `inline footnote` | as Ami Pro |
| `<:xN,ROW>` | `InlineObject::TableFootnote` | `inline tablefootnote` | as Ami Pro |
| `<:Zname>`, `<:Z~name>`, `<:Zrows,cols;name>` | kept marks, typed by `bookmarks` | `mark` | as read |
| `[book]` `name table row column` | the kept section, typed by `bookmarks` | `doclegacy` | tab-indented lines |
| `[fopts]` (six values; older files four) | `FootnoteOptions` | `footnotes` | Ami Pro 3.1's six values |

## Proofing (2026-10-26)

`docs/research/PROOFING_NOTES.md`; `UI_SPEC.md` §35.

| Stored form | Ami Pro | The app |
|---|---|---|
| `[lang]` | The document's language, 1–28 (1 American, 2 British, 27 British IZE…; 0 none); `dictlang` gives new documents theirs | `Document::language` (`Language`: Ami Pro's number, or a locale it has none for, kept in native v3 and written as the nearest number, which Save As lists: `LossKind::Language`); read at import, written from the model in documents and sheets |
| `[GramStyle]` | The grammar and style set the last check used (blank: none) | `Document::grammar_set`, set by Grammar Check (one undo step), written from the model |
| `<:s>` | A paragraph a spell check passed clean (not the text's last, not a selection's); skipped by later checks until edited (VM PSP4B) | Kept as a paragraph mark while the paragraph's text is as read; a command that changes the text drops it, so Ami Pro checks the paragraph again (twin APF1) |
| `LTSUSER1.DIC` (the user dictionary) | Plain text, a word per line, CR LF | The app's user dictionaries in the same format; Import… reads Ami Pro's |

## TOC, index, glossary (2026-11-03)

`docs/research/TOC_INDEX_NOTES.md`; `UI_SPEC.md` §36.

| Stored form | Ami Pro | The app |
|---|---|---|
| `[toc]` | The TOC levels: the first three styles, their separators, a flags word (Page number, Right align, two bits a level), then a name, separator, and flags for each further level | `ContentsSettings::toc` (`TocSettings`, kept place by place with older files' extra bits); read at import, written from the model in documents and sheets; native `toc` |
| `[master]` | TOC, Index's output files and flags (1: alphabetical separators), then the master document's files | `ContentsSettings::master` (`MasterSettings`; the file list typed since the master run, the byte count computed); native `master` |
| `[sequence]` | a master's member: page base, footnotes before, a reserved 0, Seq and numbered-style counters | `ContentsSettings::sequence` (`Sequence`, `Counter`); native `sequence`; written by Save As SAM with the set's numbers (MS3; Ami Pro opens the app's members and works the set to the same numbers, MR.12) |
| `<:X8,V;Index "p" # "s" "o">` | An index entry: primary, page number, secondary, Other; V the page after generation | `IndexEntry` over the kept power-field mark; Mark writes Ami Pro's code; V left as read (TQ3) |
| `<:X11,V;TOC n "text">` | A TOC entry | `TocMark` over the kept mark |
| The TOC's page table | Frame flag `0x4000000` (Ami Pro finds it again and replaces its rows); TOC1–9 and TOCPG1–9 styles; leaders as cell leaders | `is_toc_table`/`mark_toc_table` (kept among the frame's unknown flags); `Command::PutToc` |
| `<:p</B>Standard>` | The break after a TOC at the start: flag 0x02 (page numbers start again after it), 0x80 with the layout's name | `InlineObject::Break::restart`, read and written; native `restart`; the layout's `PageBox::shown` |
| `[recfile]` | The data file, the description file (`*dBase` for a filter), flags (1 print, 2 view, 0x10 As labels), the Labels unit (1 in., 2 cm., 3 picas, 4 pt.), across, down, the first label's indent down and right in twips | `MergeSettings::record_file` (`RecordFile`; label values kept as read without As labels); native `recfile`; written in Ami Pro's place (the merge run; twins AML1, AMLB open in Ami Pro) |
| Merge conditions | Not stored | `MergeSettings::conditions`, `with_conditions`; native `mergecondition`, `withconditions`; listed by Save As SAM (`LossKind::MergeConditions`) |
| `<:X12,-32768;MergeField NAME>` | A merge field, its result `<NAME>` outside a merge; older files `<:MNAME>` | Kept as power-field marks; worked by the merge and the preview (MG10) |
| A merge data document | `~|` (or one field delimiter), the names record, then the records | `merge::data::read_ami_data`; the record editor writes `data_paragraphs` (Ami Pro opens the app's AMD1 and merges from it) |
| A glossary data file | A SAM document: `~|` (field and record delimiters), `id~text|`, `name~text|` records of SAM body text, then a hidden `[gloidx]` index Ami Pro adds even when only choosing the file | `amipro_formats::glossary` reads without writing; Add, Rename, and Delete write the records and the index as Ami Pro does (GLO0 byte for byte), through a temporary file and a rename |

## Objects: OLE, charts, equations, image processing (2026-11-07)

`docs/research/OBJECTS_NOTES.md`; `UI_SPEC.md` §42; `docs/OBJECTS_PROGRESS.md`.

| Stored form | Ami Pro | The app |
|---|---|---|
| An OLE 1.0 embedded stream in `[Embedded]` | Class (WordArt, 123Worksheet, ClipArt Gallery in the corpus), native data, then a `METAFILEPICT` presentation; 12 of 34 have trailing bytes | `amipro_pictures::ole` reads it and plays the presentation (34 of 34 drawn); the bytes are never changed; Convert Object to Picture replaces the object by the metafile (native only, one undo step; Save As SAM then writes a plain picture) |
| A chart's picture type and data record | Picture type `.sdw` (`.sct` gives "Cannot find a library" in Ami Pro) with the type-15 data record after the drawing's objects and the `[isd]` state words | `amipro_pictures::chart::Chart` keeps every word of the record (104 of 104 corpus and 18 of 18 VM charts write back equal); the data text is Ami Pro's `1018:0078` form; objects generated by `chart_gen` and drawn by `sdw::paint`; the SAM writer writes `.sdw` for a chart frame (the `.sct` driver name stays after the state words); native v3 keeps the record with the drawing |
| An equation: two lines in `[Embedded]`, `.tex` | The TeX text and the settings line (face, size); the `[isd]` words hold the width, the negative height and the negative baseline | `amipro_pictures::equation` reads and writes them byte equal (27 of 27 VM equations); the app draws the equation from its text; an equation whose text is not two lines is kept as it is |
| A gray-scale 8-bit TIFF as `.tgf` | The TIFF bytes unchanged with the image-processing state in `[isd]` (Brightness, Contrast, Edge Enhancement, Smoothing, Invert, Halftone) | `ImageAdjust`: the state words are equal to the VM's 12 of 12; other bitmaps keep editable settings natively and Save As SAM bakes them into the 24-bit bitmap, listed in the loss list |

Not carried: Link Options changes, Paste Link and New Object (the servers are
Ami Pro's); a chart's Pictures dialog entries (`AmiChart.ini` is not read).
