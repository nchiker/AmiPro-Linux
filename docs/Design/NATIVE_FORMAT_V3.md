# Native Format v3 (`AMIPRO-LINUX/3`)

**Status:** specification for Phase 2.3d, written 2026-10-11 before the
implementation (`crates/document/src/native.rs`). Decision D8 in
`docs/research/DECISIONS.md` (Phase 2.2) chose the shape: one strict,
line-oriented, tab-separated text file that stores everything in the model.

Version 3 replaces version 2 for saving. Versions 1 and 2 still load.

## 1. File

- UTF-8 text. Lines end with LF. The first line is exactly
  `AMIPRO-LINUX/3`.
- Every other line is one record: a record name, then fields separated by
  one tab. A record has a fixed number of fields unless this document says
  it has a list.
- The whole file is at most 64 MiB, embedded payloads included.
- The decoder is strict. It rejects an unknown record, a wrong field count,
  a malformed number, a bad escape, a record out of place, and anything the
  model's validation rejects (`Document::validate`).

### Escapes

Text fields (titles, names, paragraph text, legacy values, paths) escape
four characters: `\\` for a backslash, `\t` for a tab, `\n` for a line feed,
and `\r` for a carriage return. Any other character after a backslash is an
error.

### Values

- Integers are decimal, with a leading `-` when negative. IDs are positive.
- `-` stands for "none" where a field is optional. ID fields that may be
  empty use `0`.
- Booleans are `0` or `1`.
- Point sizes and spacing are decimal floats, written in Rust's shortest
  round-trip form.
- Lengths are integer twips. Colours are `COLORREF` integers (`0x00BBGGRR`).

## 2. Record order

The decoder accepts records only in this order of sections, and each
section's records only in the order shown:

```text
AMIPRO-LINUX/3
title      …                          once
ids        …                          once
stylesheet …  hyphenzone …           at most once each (Phase 3.4, 3.6)
docinfo    …  (docfield …)            at most once, then its fields (the fields run)
style      …  (sformat …) (sline) (snext) (stabs) (shanging) (shyphenation)
              (sbullets) (sorigin) (skept)
                                      zero or more, each with its attributes,
                                      rules, next style, tabs, hanging indent,
                                      bullets and numbers, origin
lineheight …                          zero or more (2026-10-12)
facekind   …                          zero or more (Phase 3.3)
doclegacy  …                          zero or more, in order
object     …  blob …                  zero or more payloads, each with its chunks
layout     …  (layout children)       one or more; the first is the document's
frame      …  (frame children)        zero or more
stack      …                          once
flow       …  (flow children)         one or more
```

Child records attach to the most recent record of their parent kind, as
listed below. A child that appears with no parent, or after a later
section has started, is an error.

## 3. Records

### Document

| Record | Fields |
|---|---|
| `title` | title |
| `ids` | next ID, main flow ID. The counter resumes at the stored next ID, which must be above every stored ID. |
| `style` | style ID, name, parent ID or `0`, font family, size (points), bold, italic, space below (points); then, only when a style has indents or spacing beyond space below, the indents in twips (all, first, rest, right) and the spacing: line spacing (`single`, `half`, `double`, or `custom`), its height in twips (`0` for single), space above in twips, `1` when the space above is added at page tops too, Text Tightness in percent, and the break rules as Ami Pro's `[brk]` bits (2026-10-12; records of 9 fields load with none of these, and of 13 or 18 fields, written briefly that day, with indents, or indents and spacing, only) |
| `sformat` | a character attribute of the most recent `style`, as described under "Character attributes" below (2026-10-12) |
| `sline` | the most recent style's paragraph rules: Ami Pro's ten `[line]` values in its order (flags, Other length, a kept byte, colour, a kept byte, line style above, line style below, space above, space below, unit; `docs/research/SAM_FORMAT_NOTES.md`, "Style records"), after the style's `sformat` records. At most one per style, written only when the rules are not the default (`8 0 1 0 1 1 1 10 10 1`); the colour is a `COLORREF` and the lengths are not negative. Files without it load with no rules (2026-10-12) |
| `stabs` | the most recent style's own tab list, which layout uses while its break rules have bit 32: stops as in `tabs` (`K:POSITION`, an optional leader suffix), after `snext`; at most one, with at least one stop (2026-10-12) |
| `stylesheet` | the style sheet the document uses (Ami Pro's `[sty]`): its file name as written (escaped, not empty), and `1` when the page layout is the sheet's (the SAM had no `[lay]`), so another style sheet replaces its page settings. At most one, after `ids`; files without it use none (Phase 3.4) |
| `skept` | the most recent style's Both sides (`[algn]` 16) and what Ami Pro's style record holds beyond the model (`StyleKept`): Both sides, function key, face-kind bits, the three dialog units, Add in bits, Single height, `[nfmt]`'s six values (escaped), a flag and the escaped next-style name that names no style, and the `[algn]` bits 32, 64, 128. Last of the style's records, written only when they differ from a new style's (Phase 3.6) |
| `docinfo` | Ami Pro's Doc Info (`[desc]`; `docs/research/FIELDS_NOTES.md`, "Doc Info"): the creation and last save times in Ami Pro's time base (empty when not yet), the revisions as held (one more than the file said), the editing minutes, the statistics (pages, words, characters, size in K, size with links), the flags (bit 0 Run frame macros), then the description, keywords, and annotation lock (escaped). Written only when the Doc Info is not empty; files without it load with none (2026-10-20) |
| `docfield` | one of the eight Doc Info fields, 1–8, its name (empty for FieldN) and value, escaped; only those named or filled, after `docinfo` (2026-10-20) |
| `hyphenzone` | the document's hyphenation zone in average character widths (Ami Pro's `[docopts]` value 1), written only when it is not 5; at most one, after `ids` (Phase 3.6) |
| `shyphenation` | the most recent style has Modify Style › Hyphenation on (`[algn]` 256); no fields; after `shanging`, before `sbullets` (Phase 3.6) |
| `sorigin` | where the most recent style's definition lives: `sheet` (from the style sheet) or `missing` (defined nowhere: its paragraphs format as Body Text); absent, it is stored in the document. Last of the style's records, at most one (Phase 3.4; `docs/research/SAM_FORMAT_NOTES.md`, "Styles and style sheets") |
| `shanging` | the most recent style has Hanging indent (a left tab stop at All + Rest); no fields; after `stabs` and before `sbullets` (2026-10-12) |
| `sbullets` | the most recent style's bullets and numbers (Ami Pro's `[spec]`; `docs/research/SAM_FORMAT_NOTES.md`, "Bullets and numbers"): the text before each paragraph (escaped; bullet and number codes are the characters U+0007 to U+001F as described there, U+000A written `\n`), the outline level (0–255), the attributes as `[fnt]` flags, the flags (`[spec]` value 8), then `[spec]` values 1, 4, 5, 6, 9, and 10 as written. After `shanging` and before `sorigin`, at most one, written only when they differ from Ami Pro's none (`0 0  0 1 1 0 0 0 0`, empty text). Files without it load with none (2026-10-12) |
| `snext` | the most recent style's next style ID (`[brk]` 64's Next style; the switch itself is in the style's break rules), after its `sline` record; at most one, naming an existing style other than itself (2026-10-12) |
| `lineheight` | face (lower case), size in twentieths of a point, bold, italic, single line height in twips: the printer's line height for that font, which layout uses before the font's own (2026-10-12, `docs/research/DECISIONS.md`, "Line heights follow the printer"). At most one per font; heights and sizes are positive. Files without these records load with none |
| `facekind` | face (lower case, escaped), its Windows pitch and family byte (0–255) as Ami Pro stored it (`[fnt]`'s kind bits, `<:f…>`'s K): what picks a substitute for a face that is not installed and whose name gives no kind. At most one per face. Files without these records load with none (Phase 3.3) |
| `doclegacy` | key, value: document-level legacy data, in order |

### Embedded payloads

| Record | Parent | Fields |
|---|---|---|
| `object` | — | object ID, byte length |
| `blob` | `object` | a chunk of standard base64 (RFC 4648, with `=` padding only in the last chunk) |

The chunks of one object, concatenated, decode to exactly its byte length.
The encoder writes chunks of 76 characters. It writes only payloads that a
picture refers to; unreferenced payloads are dropped at save.

### Layouts

| Record | Parent | Fields |
|---|---|---|
| `layout` | — | layout ID, name, paper code, landscape, mirror, header from second page, footer from second page |
| `page` | `layout` | `right` or `left`, width, height. `right` comes first; `left` is optional. |
| `box` | `page` or `frame` | top, right, bottom, left margins; columns (1–8); gutter; balance; line-between style or `-`; line-between colour or `-` |
| `edges` | `box` | uneven columns (2026-10-17, the ruler run): each column's left and right edge in twips from the box's text area's left, two numbers per column. Optional, at most once, after `box` and before its `tabs`; absent while the columns are even. Edges must go left to right without overlapping and end within the text area. |
| `tabs` | `box` | column index, then a list of tab stops, each `KIND:POSITION` with KIND `L`, `C`, `R`, or `D`, and from 2026-10-12 an optional `:h`, `:d`, or `:l` for a leader of hyphens, dots, or a line. One record per column, in column order. |
| `pageborder` | `page` | border (below) |
| `slots` | `layout` | header right, header left, footer right, footer left: frame IDs or `0` |
| `repeating` | `layout` | a list of frame IDs, back to front (may be empty) |
| `layoutlegacy` | `layout` | key, value |
| `startlayout` | — | the ID of the layout the first page uses when it is not the document's own (Ami Pro's `[l1]`: a layout inserted at the start). At most one, after every layout; files without it start with the document's layout (Phase 4.5) |

A border is four fields: sides as a subset of `LRTB` (`-` for none), line
style, colour, position (1 middle, 2 inside, 3 outside, 4 close to inside,
5 close to outside).

Each `page` is followed by its `box`, its optional `edges`, the box's
`tabs`, and then an optional `pageborder`. `slots`, `repeating`, and `layoutlegacy` follow the pages.

### Frames

| Record | Parent | Fields |
|---|---|---|
| `frame` | — | frame ID, group ID or `0` |
| `placement` | `frame` | one of the forms below |
| `look` | `frame` | wrap (`around`, `none`, `beside`), opaque, corners (percent), background colour |
| `border` | `frame` | border |
| `shadow` | `frame` | left, top, right, bottom extents, colour |
| `box`, `edges`, `tabs` | `frame` | the frame's interior layout, as for pages |
| `name` | `frame` | frame name (optional) |
| `macro` | `frame` | macro name (optional) |
| `content` | `frame` | one of the forms below |
| `framelegacy` | `frame` | key, value |

Placement forms:

| Form | Fields after the kind |
|---|---|
| `fixed` | page, left, top, right, bottom |
| `below` | x offset, width, height |
| `inline` | y offset, width, height |
| `repeating` | `all` or `rightleft`, left, top, right, bottom |
| `header` | `right` or `left` |
| `footer` | `right` or `left` |

Content forms and their children:

| Form | Fields after the kind | Children |
|---|---|---|
| `text` | flow ID | — |
| `table` | column count, row count, horizontal cell margin, vertical cell margin | `column` (text width) × columns, then `row` (least text height) × rows, then `tablerows` (Automatic `0`/`1`, Rows span pages `0`/`1`, the heading rows and the rows with a page break before them, each a comma-separated list of row indices or `-`; written only when not a new table's `1 0 - -`; since 2026-10-18, the tables across pages run, `UI_SPEC.md` §26), then `readrow` (row index, the row's fingerprint as 16 hexadecimal digits) for each row whose stored height is Ami Pro's measure as read, in row order (since 2026-10-20, the row cache; `docs/research/TABLE_PAGES_NOTES.md`), then `cell` (flow ID, rows spanned, columns spanned) × columns × rows in row-major order; `celllegacy` (key, value) after a `cell`; `tablelegacy` (key, value) after the cells |
| `picture` | `embedded` with an object ID, or `linked` with a path; extension; rendering object ID or `0` | `picturescaling` (sizing `original`, `fit`, `percentage`, or `custom`; keep aspect `0`/`1`; percent; width; height; rotation in degrees; pan x; pan y, downward; since 2026-10-18, the pictures run: a file without it takes these from the kept `isd` value, `UI_SPEC.md` §25), then `picturedescription` (text) when there is one, then `picturelegacy` (key, value) |
| `legacy` | — | `contentlegacy` (key, value) |

The records of a frame come in the order of the first table. `name` and
`macro` appear only when set.

A cell spans 1 × 1 normally, more when it is the top-left cell of joined
cells, and 0 × 0 when joined cells cover it. Cell margins and spans were
added on 2026-10-12; a `table` record without the margins (86 and 86) or a
`cell` record without the span (1 × 1), as files written before then have,
loads with those defaults.

### Frame stack

| Record | Fields |
|---|---|
| `stack` | a list of frame IDs: every page frame, back to front (may be empty) |

### Flows

| Record | Parent | Fields |
|---|---|---|
| `flow` | — | flow ID |
| `paragraph` | `flow` | paragraph ID, style ID, text |
| `pformat` | `paragraph` | one property set on the paragraph over its style: `indents` with all, first, rest, right in twips; `line` with the line spacing and its height, as in `style`; `align` with `left`, `right`, `centre`, or `justify`; or `tabs` with the paragraph's own tab ruler, zero or more stops as in `tabs` (2026-10-12) |
| `run` | `paragraph` | start, end, family present, family, size or `-`, bold, italic, underline (each `-`, `0`, or `1`) |
| `rformat` | `run` or `endformat` | a character attribute of the run or of the end format, as described under "Character attributes" below (2026-10-12) |
| `endformat` | `paragraph` | the paragraph's end format, the character format in force after its last character, which sets an empty paragraph's line: the `run` record's fields from family present on (family present, family, size or `-`, bold, italic, underline), followed by its own `rformat` records. At most one, written only when the format is not empty; files without it load with none (2026-10-12) |
| `inline` | `paragraph` | one of the forms below |
| `mark` | `paragraph` | byte offset, raw text |
| `pcache` | `paragraph` | Ami Pro's layout cache for the paragraph (`<:#H,W>`): height and column width in twips, and the 16-digit hexadecimal fingerprint of the paragraph and its style when it was read (`Paragraph::content_fingerprint`); the SAM writer gives the cache back only while the fingerprint is the same. At most one, the paragraph's last record (Phase 4.9) |

Paragraph formats, runs (each followed by its `rformat` records), the end
format (followed by its own), inline objects, and marks of a paragraph come
in that order; runs, objects, and marks each in text order. The encoder writes each `pformat` property at
most once. The k-th `inline` record of a paragraph is the object of its k-th U+FFFC.

### Character attributes

`sformat` and `rformat` records carry the character attributes beyond the
`style` and `run` records' own fields: Ami Pro's underline variants,
strikethrough, superscript, subscript, caps, and colour
(`docs/research/SAM_FORMAT_NOTES.md`, "Character attributes" and "Font
changes"). Each record is an attribute name and a value, and each attribute
appears at most once per style or run:

| Name | Value | Notes |
|---|---|---|
| `underline` | `0` or `1` | styles only; a run's underline is the last field of its `run` record |
| `wordunderline` | `0` or `1` | words underlined, the spaces between them not |
| `doubleunderline` | `0` or `1` | |
| `strikethrough` | `0` or `1` | |
| `superscript` | `0` or `1` | 7/12 of the size, raised; wins over subscript |
| `subscript` | `0` or `1` | 7/12 of the size, lowered |
| `protected` | `0` or `1` | protected text (`<+*>`, `[fnt]` 4096; Phase 3.3) |
| `nohyphenation` | `0` or `1` | never split at a line's end (`<+.>`; Phase 3.3) |
| `caps` | `upper`, `lower`, `initial`, `small`, or `normal` | the four exclude one another; `normal` (runs, 2026-10-12) shows the letters as typed over a style that sets caps |
| `colour` | a `COLORREF` integer, at most 16777215 | |
| `overstrike` | one character, escaped | drawn over each character (`<:O+C>`; Phase 3.3) |

The encoder writes them in the order of this table. For a style it writes
only the attributes that are set (a colour other than black); a style
without `sformat` records has none and is black. For a run it writes every
attribute the run sets, `0` included: `0` switches the attribute off over a
style that has it, as Ami Pro's `<-%>` does for strikethrough, while an
absent record means the style's own setting. Files written before these
records existed load with no attributes.

Inline forms:

| Form | Fields after the kind |
|---|---|
| `break` | `page` or `column`, centre, layout ID or `0` |
| `floating` | `header` or `footer`, `all`, `right`, or `left`, flow ID |
| `anchor` | frame ID |
| `pagenumber` | style character, start page (0 for none), start number (0 for none), leading text |
| `date` | a date or the time (Ami Pro's `<:D…>`, `<:r…>`, `<:k…>`, `<:b…>`): `today`, `saved`, `created`, or `time`, then the byte after the control's letter as a decimal number (the format letter; for the time, Ami Pro's ignored list position); since 2026-10-20 |
| `docinfo` | a Doc Info entry (`<:d…>`): Ami Pro's code (`docs/research/FIELDS_NOTES.md`, "Insert Doc Info Field") and the date format byte, as decimal numbers; since 2026-10-20 |
| `bullet` | the bullet's code as a decimal number, one of Ami Pro's 17 (16–25 the shapes, 8 and 10–15 the pictures; `docs/research/BULLETS_NOTES.md`); since 2026-10-17 |

A `pagenumber` record with three fields after the kind (written before
2026-10-13) holds the style character and the start page as one number,
then the start number: `10 0 Page ` is style `1`, start page 0, start
number 0 (`docs/research/PAGE_LAYOUT_NOTES.md`, "Page numbering"); it
still loads.

Flows are written in ID order. Every flow, the main flow included, has at
least one paragraph.

## 4. How the decoder rebuilds the model

1. Read every record, check its fields and its place in the order above,
   and collect styles, payloads, layouts, frames, the stack, and flows.
2. Build the document from them with the stored IDs and counter. Nothing is
   renumbered.
3. Run the model's full validation: ID uniqueness and counter, ownership of
   every flow, reachability of every frame, anchors, groups, geometry, and
   object counts. A file that fails is rejected as a whole.

Saving writes the document to a temporary file and renames it over the
target, as before (`Document::save_native`).

## 5. Versions 1 and 2

Version 1 (`AMIPRO-LINUX/1`) and version 2 (`AMIPRO-LINUX/2`) store one
paragraph flow, styles, and the page in points (v2 adds runs). They load
into the main flow, with the document layout's right-page size and margins
converted to twips and Ami Pro's default header and footer frames. IDs are
assigned on load in document order. The application no longer writes them.

## 6. Example

A new document with one paragraph "Hello", whose header holds a page-number
field. Its styles carry Ami Pro's face-kind bits (roman for Times New
Roman, swiss for Arial), so each has a `skept` record:

```text
AMIPRO-LINUX/3
title	Untitled
ids	10	1
style	1	Body Text	0	Times New Roman	12	0	0	0
skept	0	0	49152	1	1	1	16	0	272	1	2	.	,	$	0		0
style	2	Heading 1	1	Arial	20	1	0	10
skept	0	0	16384	1	1	1	16	0	272	1	2	.	,	$	0		0
style	3	Heading 2	1	Arial	16	1	0	8
skept	0	0	16384	1	1	1	16	0	272	1	2	.	,	$	0		0
layout	2	Standard	1	0	0	0	0
page	right	12240	15840
box	1440	1440	1440	1440	1	240	0	-	-
tabs	0	L:720	L:1440	L:2160	L:2880	L:3600	L:4320	L:5040	L:5760	L:6480	L:7200	L:7920	L:8640
slots	3	0	6	0
repeating
frame	3	0
placement	header	right
look	around	1	0	16777215
border	-	1	0	1
shadow	0	0	0	0	0
box	792	1440	72	1440	1	240	0	-	-
tabs	0	C:4680	R:9360
content	text	4
frame	6	0
placement	footer	right
look	around	1	0	16777215
border	-	1	0	1
shadow	0	0	0	0	0
box	72	1440	792	1440	1	240	0	-	-
tabs	0	C:4680	R:9360
content	text	7
stack
flow	1
paragraph	9	1	Hello
flow	4
paragraph	5	1	￼
inline	pagenumber	1	0	0	Page 
flow	7
paragraph	8	1	
```

The header paragraph's text is the field's U+FFFC, written as the
character itself; the field's leading text "Page " (with its trailing
space) is the last field of the `inline` record.
