# SAM Page and Frame Format Specification

**Status:** draft 2, 2026-10-11 (Phase 2.1, plus the Phase 2.2 behaviour
checks V10–V23 and X06). Independently written from controlled fixtures made
in original Ami Pro 3.1, synthetic files read back by it, the private corpus,
and a decompiled third-party filter. The evidence
for every rule, with its level (VERIFIED, CORPUS, INFERRED, DECOMPILED), is in
`SAM_FORMAT_NOTES.md`; this document states the rules an implementation
follows. Rules marked *(inferred)* still need a fixture.

It covers the parts of a `.SAM` file that describe pages, page layouts,
headers and footers, frames, and their anchors in the main text. Paragraph
text, escapes, and styles are in `SAM_FORMAT_NOTES.md` ("Document body",
"Style records").

## 1. Conventions

- **Lines and sections.** The file is CRLF text in Windows-1252 up to the
  end of the body. A line `[name]` opens a section; its values are the
  following lines, indented by one tab per nesting level. Nested sections
  (`\t[frmlay]`) belong to the enclosing record. Values are decimal integers
  unless stated.
- **Units.** Lengths are twips (1/1440 inch). Positions are measured from
  the top-left corner of the physical page unless stated.
- **Colours** are Windows `COLORREF` integers `0x00BBGGRR` (black 0, white
  16777215, red 255).
- **Line styles** are 1-based indexes into Ami Pro's line-style list (1 =
  thinnest). The list's length is not yet known; preserve unknown indexes.
- **Line position** values: 1 middle, 2 inside, 3 outside, 4 close to
  inside, 5 close to outside.
- **Side masks:** 0 none, 1 all sides, otherwise 2 left | 4 right | 8 top |
  16 bottom.

## 2. Top-level order

```
[ver] … [tag]*          header sections and styles
[frm]*                  page-level frames, back to front
[lay]+                  page layouts, the first is the document's layout
[elay]
[l1]
[pg]                    page table (cache)
[edoc]                  main text, ended by a line ">" outside any control
<binary objects>        embedded files, only when pictures are copied in
[Embedded]              object directory
NNNNNNNN                byte offset of the "[Embedded]" line
```

The trailer is the zero-padded eight-digit byte offset at which `[Embedded]`
starts. It is present with or without objects.

## 3. Page layouts

### 3.1 `[lay]`

```
[lay]
    NAME                layout name ("Standard"; inserted layouts "1", "2", …)
    NUMBER              layout number, below
    [rght]  LAYOUT      right pages, or all pages
    [lft]   LAYOUT      left pages; only when bit 0x200 is clear
    [hlft]  FRAME       left-page header frame; only with [lft]
    [hrght] FRAME       header frame
    [flft]  FRAME       left-page footer frame; only with [lft]
    [frght] FRAME       footer frame
    [repfrm] FRAME*     repeating frames, each a [lyfrm] record
```

Layout number:

| Bits | Meaning |
|---|---|
| `0x7F` | Paper size: 1 Letter, 2 Legal, 4 A4, 7 Custom (3 A3, 5 A5, 6 B5 *(inferred)*) |
| `0x80` | Set in every style sheet's layout (641), clear in SAMs (513): the layout is the style sheet's *(inferred, SS2; writers keep it as read)* |
| `0x100` | Landscape (LAYOUT stores the rotated height and width) |
| `0x200` | One layout for all pages; when clear, `[lft]`/`[hlft]`/`[flft]` are present |
| `0x400` | Mirrored left and right pages |
| `0x800` | Header begins on the second page |
| `0x1000` | Footer begins on the second page |

Right pages are the odd pages. The paper size code is informational: the
height and width in LAYOUT are authoritative.

### 3.2 LAYOUT block

The same value list describes a page (`[rght]`, `[lft]`) and a frame's
interior (`[frmlay]`, §4.4):

| # | Page meaning | Frame meaning |
|---|---|---|
| 1 | Page height | Bottom edge of the frame (absolute) |
| 2 | Page width | Frame width |
| 3 | Unknown; preserve | same |
| 4 | Left margin | Left margin |
| 5 | Bottom margin | Bottom margin |
| 6 | Unknown; preserve | same |
| 7 | Top of the text area = top margin | Top of the text area (frame top + top margin) |
| 8 | Right margin | Right margin |
| 9 | Flags: 1 column balance, 2 line between columns, 4 lines around the page | Flags: 1 column balance *(inferred)*, 2 line between columns |
| 10 | Line between columns: style | same |
| 11 | Page line colour | Line between columns: colour *(inferred)* |
| 12 | Line around the page: style | Stale; preserve |
| 13 | Line around the page: side mask | Stale; preserve |
| 14 | Line around the page: position | Stale; preserve |
| 15 | Column count *n*, 1–8 | same |
| … | *n* columns | same |

Each column is `LEFT RIGHT T` followed by *T* tabs, each `TYPE POSITION`.
LEFT and RIGHT are absolute x positions. TYPE is 1 left, 2 centre, 3 right
*(inferred for 2 and 3)*, 4 decimal *(inferred)*. POSITION is relative to the
column's left edge. The gutter between columns is the next LEFT minus this
RIGHT (240 by default). Columns and gutters are stored, not derived, for
pages; for frames they are derived (§4.4).

### 3.3 Left pages and mirroring

When left pages differ, `[lft]` is a complete LAYOUT for even pages. With
mirroring (`0x400`) the writer stores the left page with the left and right
margins exchanged and columns recomputed. A reader uses `[lft]` as stored.

### 3.4 Headers and footers

`[hrght]`, `[frght]`, `[hlft]`, `[flft]` each hold one FRAME (§4) whose
`[lyfrm]` record covers the margin band: the header from the page top to the
top margin, the footer from the bottom margin to the page bottom, across the
full page width. Their own margins, columns, and tabs are in their
`[frmlay]` (defaults: header top 792 and bottom 72; footer top 72 and bottom
792; left and right 1440; a centre tab at the middle and a right tab at the
right margin). Their `[txt]` is the fixed header or footer text. Flag
`0x800` marks a header frame and `0x1000` a footer frame *(inferred)*.

### 3.5 Inserted layouts and breaks

Further `[lay]` sections are inserted layouts. The main text selects them
with page-break controls (§5.3). Each inserted layout is complete, including
its own header and footer frames.

## 4. Frames

A FRAME is a frame record followed by optional named subsections, its
interior layout, and its content:

```
[frm] or [lyfrm]
    FIELDS 1–13          (16 for anchored frames)
    [frmname] NAME       optional
    [frmmac]  MACRO      optional (Run macro)
    [frmlay]  LAYOUT     interior layout (§3.2)
    [txt] | [isd] | [tbl] content
```

### 4.1 Where frames are stored

| Kind | Stored as |
|---|---|
| Frame on a page (Where placed) | top-level `[frm]` |
| Anchored frame (With para above, Flow with text) | top-level `[frm]` plus a mark in the main text |
| Table in the main text | top-level `[frm]` with a table (§4.7) plus a `<:t n>` mark |
| Repeating frame (all pages, right/left) | `[lyfrm]` under `[repfrm]` in a layout |
| Fixed header or footer | `[lyfrm]` under `[hrght]` etc. |

Top-level `[frm]` sections are in z-order, back to front.

### 4.2 Frame record

| # | Meaning |
|---|---|
| 1 | Page number of a Where placed frame (1-based). Stale for anchored frames; preserve for other kinds |
| 2 | Flags, §4.3 |
| 3–6 | Left, top, right, bottom, absolute page twips |
| 7 | Line sides mask |
| 8 | Unknown; preserve |
| 9 | Line position |
| 10 | `STYLE SHL SHT SHR SHB CORNER SHCOLOR`: line style; shadow extents left, top, right, bottom (Normal 100, Deep 172, none 0); rounded-corner percentage (0 = square); shadow colour |
| 11 | Line colour |
| 12 | Background colour |
| 13 | Sequence number: position in the file's frame order, counting header and footer frames first. Not an identity; regenerate on write |
| 14 | Anchored only: anchor index as written (equals the frame's position among anchored frames, tables included). Readers use the position, not this value |
| 15 | Anchored only. With para above: horizontal offset from the left margin (a setting). Flow with text: the laid-out x offset (a cache). Not an offset for table frames (20000 for a centred table in MERCURY.SAM; up to 19456 for tables at the margin), whose offset is field 3 from the left of its column |
| 16 | Anchored only: `WIDTH HEIGHT` as written. The size is fields 3–6: field 16's height goes stale for With para above frames and is the top-to-baseline distance for Flow with text; both values are stale for table frames |

### 4.3 Flags (field 2)

| Bit | Meaning |
|---|---|
| `0x4` | Table frame (`[tbl]` content) |
| `0x8` | Set on every style sheet's layout frames, almost never in SAMs: the frame is the sheet's; Save as a Style Sheet without contents leaves out its text and picture (decompile `11c8:2a0f`, SS2N) |
| `0x40` | Opaque (clear: transparent) |
| `0x80` | Wrap around |
| `0x20000` | No wrap beside |
| `0x80000` | Anchored |
| `0x100` | Repeating |
| `0x2000` | Repeat right/left only (with `0x100`) |
| `0x800000` | Group member |
| `0x1000000` | First member of a group |
| `0x2000000` | Last member of a group |
| `0x20000000` | Picture frame (`[isd]` content) |
| `0x200` | The frame holds text (`[txt]` follows; decompile `11c8:2a0f`) |
| `0x400` | Never written (the writer clears it) |
| `0x10000`, `0x100000` | Unknown; preserve (`0x100000`: tables, `[chint]`) |

Wrap mode: `0x80` set means Wrap around; `0x20000` set (with `0x80` clear)
means No wrap beside; neither means No wrap around (text runs through).

Placement: not anchored and not repeating means Where placed. Anchored with
`0x80` means Flow with text; anchored without `0x80` means With para above.
Repeating with `0x2000` means Repeat right/left, otherwise Repeat all pages.

Groups are contiguous runs of top-level frames in z-order, from the frame
with `0x1000000` to the frame with `0x2000000`. Anchored frames are never
grouped, and the members of a group are on one page.

Content: a frame holds exactly one of text (`[txt]`), a table (`[tbl]`,
flag `0x4`), or a picture (`[isd]`, flag `0x20000000`). A table in a Where
placed frame is a *frame table*; a table in the main text is an anchored
table frame (§5.2).

Layers: top-level `[frm]` frames always paint above layout frames
(`[repfrm]` repeating frames). Bring to Front and Send to Back reorder
frames within their own layer only. Wrap exclusions of any frame apply to
the main text and to header and footer text.

### 4.4 Interior layout and derived columns

`[frmlay]` is a LAYOUT (§3.2). For a frame, the margins (default 144) are
authoritative and the column edges are derived: the text area is the frame
rectangle minus its margins, further inset by r(1 − cos 45°) on each side
when corners are rounded, where r = CORNER% × min(width, height) / 2, and by
the line width for heavier line styles. A reader recomputes columns from the
margins and the stored gutters; a writer stores them.

### 4.5 Text frames

`[txt]` holds the frame's own paragraphs in main-text syntax, styles
included, ended by a lone `>` line. An empty frame writes just `>`.

### 4.6 Picture frames

`[isd]` holds the picture description: `.X3`, the source extension (for
example `.bmp`), then `.X3` when the picture was copied into the document or
the source path when it is linked, then about sixty numbers not yet decoded
(scaling and cropping are likely). `[frmname]` is present (`FrameN`).

The picture bytes are in the object area after the body (§6).

### 4.7 Table frames

```
[tbl]
    ROWS COLS ROWHEIGHT V4 COLWIDTH V6 V7 V8 V9     defaults (CORPUS)
    [h]                                             optional, closed by [e]
        ROW HEIGHT …                                one per row that differs
    [e]
    [w]                                             optional, closed by [e]
        COL WIDTH …                                 one per column that differs
    [e]
    [data]
        ROW COL F3 … F12                            a cell record
        paragraphs, then >                          its text (absent for a covered cell)
        paragraphs, then >                          further bodies: a formula (optional)
        …
    [e]
[tble]
```

The first header value is the row count and the second the column count
(212 corpus tables: the column width is the text width divided by the second
value). The defaults apply to rows and columns without a record. A cell
record with `128` in its third field writes no text: it is covered by a
joined neighbour. A calculated cell's displayed value is followed by a second
body holding its formula (MERCINV.SAM: `<+A>d4*e4`; `<+A>` is the
right alignment of both bodies, as in any paragraph, `SAM_FORMAT_NOTES.md`
O2). Cells may be omitted;
missing cells are empty. Cells are text flows. The meaning of the remaining
header values and cell fields is Phase 5 work; they are preserved
*(CORPUS)*.

Geometry (*VERIFIED*: TB1, V20, README31.SAM; *CORPUS*: 140 of 220 tables
exactly): ROWHEIGHT and the `[h]` heights are each row's text height, and
COLWIDTH and the `[w]` widths each column's text width. V4 is the rows'
gutter and V6 the columns' (86 by default; Column/Row Size's and Modify
Table Layout's Gutter size), V7 the table's flags (1: Automatic row
height), V8 and V9 the table's top and left insets (43; absent in older
files, read as 43). A `[h]` or `[w]` record is index, size, gutter, flags
(and two more for rows); its gutter counts only with flag bit 4, else the
header's (`1428:046b`; READMEBP.SAM). Column c's text starts at V9 plus
each earlier column's width and gutter, row r's at V8 plus each earlier
row's height and gutter, so the frame is V9 plus every column and gutter
wide, V8 plus every row and gutter tall (*VERIFIED*: TF14, TF14X;
`CELL_FRAME_NOTES.md`; with the default 86 and 43 this is "a cell is its
column's width plus 86, the frame all cells plus 43"). In the other 80
tables the frame is smaller than the rows: the table continues on later
pages and the rectangle holds one page's part.

A cell record's values 11 and 12 are 1 and the place of the cell's frame
in Ami Pro's list of frames in cells when the cell holds a picture (a
drawing, chart, or equation), 0 and 0 otherwise (`CELL_FRAME_NOTES.md`).

Joined cells (*CORPUS*: 27 blocks and 37 covered cells in 220 tables, all
consistent; README31.SAM displayed): a cell record whose third field has
`0x100` (with `0x80`; `384`) is the top-left cell of a block of field 4 rows
by field 5 columns; a covered cell has `0x80` alone (`128`) and fields 4 and
5 give its offset from the top-left cell. Ami Pro shows the block as one
cell.

Untagged cell text is Table Text: "Ami Pro automatically assigns a Table Text
paragraph style to all table cells", generated from Body Text with no space
above or below and no indent (user's guide; MERCURY.SAM's cells show Table
Text's Arial, not Body Text's Times New Roman).

## 5. Main-text controls for pages and frames

### 5.1 Frame anchors

`<:A n>` anchors the *n*-th anchored `[frm]` (0-based, counting only
records with flag `0x80000`, in file order, table frames included). Anchors
occur only in the main text.

With para above writes the mark at the end of its paragraph and places the
frame below that paragraph's text, before its space below; a mark at the
start of a paragraph places the frame at the paragraph's top, and a paragraph
that holds only marks takes no height (its spacing still applies). Text runs
through a No wrap around frame, so the next paragraph starts beside it; No
wrap beside and table frames push the text after them below them
(MERCURY.SAM: seven icons and a drawing, `SAM_FORMAT_NOTES.md`). Flow with
text places the frame inline at the
mark, sitting on the baseline and enlarging the line; its right shadow
takes space after it (Z05: two frames with the Normal shadow stand 100 twips
apart). The frame stands at its line's top and its bottom shadow is in the
line, so the next line starts below the shadow (VM FA1: a 408-twip frame
with the Deep shadow makes a 580-twip line; FA2: 408 without). A frame that
fits keeps the line's indent; one too wide for its line moves left until it
ends, with its right shadow, at the line's right edge, but not past the
column's left (FA3: 3800, 3500, and 3300 twips with the Deep shadow in a
3394-twip line after a 720-twip first-line indent stand at 142, 442, and 642
twips from the column's left; 3394 without a shadow keeps the indent). A
frame after text that does not fit goes to the next line like a word, and
blanks that open a paragraph count as its text (a space and a column-wide
frame take two lines: `_PROPOS2.STY`'s New document). The stored rectangle
of an anchored frame is a cache. An anchored frame is no wider than its
column with its right shadow (FA1, FA2: 8296 twips in a 4114-twip column
is 3942 wide with the Deep shadow, 4114 without) and no taller than the
space left in it.

Text wraps around a frame's shadow as well as the frame. A part of the
column beside a Wrap around frame takes a line when the line's first
character fits; an empty line fits in any part (VM FG1–FG3: gaps of 20,
200, and 400 twips beside a Where placed frame hold empty lines in all
three, "a" from 200, and "Word" only in 400, broken over two lines; the
20-twip gap beside `_REPORT2.STY`'s Deep-shadowed frame holds its empty
first line). The app takes a part when the line's first word fits, so a
word wider than such a part goes below the frame instead of breaking
(FG3, open). A Wrap around frame beside the first lines of a column makes
Ami Pro add the first paragraph's space above even with When not at break
(FG5: `_FAX3.STY`'s document, its table 72 twips lower than FG6, the same
with the picture moved down the page); the rule is not yet decompiled or
modelled (open). When the frame does not fit
below its paragraph, or its line does not fit, the paragraph or line moves
to the next page with it.

Deleting a text range that contains an anchor deletes its frame (Ami Pro
asks first), and undo restores both.

### 5.2 Table anchors

`<:t n>` anchors the table frame that is the *n*-th anchored `[frm]`: the
same numbering as `<:A n>`, shared by both marks. It is written at the start
of the paragraph that follows the table. It is not a tab.

### 5.3 Page breaks

`<:p` + one flag byte + optional layout name + `>`, anywhere in a main-text
paragraph; the text after it starts the next page (it was written at the end
of a paragraph in G20 and at the start of one in V01). The byte is written
escaped when needed: `<:p<* >>` (0x00) is a plain page break, `<:p@>` (0x40)
a break that vertically centres the text above it, and `<:p</@>NAME>` (0x80)
breaks the page and switches to layout NAME from the next page. So 0x40
means centre and 0x80 means a layout name follows *(flag reading inferred
from these three forms)*.

A column break is `<:c>`, written at the start of the paragraph that holds
the caret when it is inserted. Its effect in a multi-column layout is not
yet observed; preserve it.

### 5.4 Floating headers and footers

`<:H` (header) or `<:h` (footer) + a flag byte written as `<*X>`, a line
break, the paragraphs, and a closing `>` line form one multi-line control
in the main text. Flags: 1 header, 2 footer, 4 left pages, 8 right pages
(all pages = both). It applies from the next page. A floating header or
footer with no text is valid and blanks it. Removing one deletes its mark;
nothing else is stored. They occur only in the main text.

### 5.5 Fields and caches

- `<:P` S PAGE `,` NUMBER `,` TEXT `>` is a page-number field: S the style
  character (`1`, `I`, `i`, `A`, `a`), PAGE the start page and NUMBER the
  start number (0 when not set), TEXT the leading text (decompile
  `11f8:023f`; VM fixtures G18, PN1; `PAGE_LAYOUT_NOTES.md`).
- `<:#H,W>` at the start of a paragraph caches its height and width. Ignore
  on read, except that one-line heights calibrate the document's printer
  line heights (2026-10-12, `DECISIONS.md`, "Calibrated line heights"); it
  may be omitted on write.
- `<*X>` is a one-character escape for byte X − 0x20. In text, 0x0A
  (`<**>`) is a hard return (a line break inside the paragraph) and 0x17
  (`<*7>`) a discretionary hyphen. A no-break space is `</`>` (0xA0).

## 6. Embedded objects

After the body's closing `>` line, copied pictures are stored as raw bytes.
The `[Embedded]` section lists them, one line per picture frame:

```
SEQ EXT OFFSET LENGTH ROFFSET RLENGTH [PATH]
```

SEQ is the frame's sequence number (field 13), EXT the source extension,
OFFSET/LENGTH the original file's bytes (0 0 when linked), ROFFSET/RLENGTH
Ami Pro's own rendering of the picture (the `.X3` form, beginning `53 53`),
and PATH the linked file. The original bytes are stored unchanged. The line
after the directory is the byte offset of `[Embedded]`.

A reader must not decode the body past the closing `>` line as text.

## 7. Page table

The page count is the larger of the pages the main text needs and the
highest page number of a Where placed frame: Ami Pro adds empty pages so a
fixed frame's page exists, when text shrinks and when a file is opened.

`[pg]` is the page count followed by one record per page: the end of the
page in the main text (paragraph, character offset), a flag word, and the
name of a layout, among other values. It is written at save time. A reader
recomputes pagination; a writer regenerates the table.

## 8. Reader and writer rules

- Import every structure above into the model, or report it. Unknown fields
  and sections are retained with their record so they are not silently lost
  (`AGENTS.md`).
- Caches (`[pg]`, `<:#…>`, anchored rectangles, frame columns, `[frmlay]`
  values 12–14 of frames, field 13) are recomputed on read. The one
  exception is metrics: `<:#…>` one-line heights give each font's printer
  line height, which no reader can recompute.
- Values a writer cannot derive (fields 8, `[frmlay]` 3 and 6, flag bits
  `0x200`, `0x10000`, `0x100000`, `[isd]` numbers) are written back as read.
- No SAM writer exists yet; this section constrains the future one.

## 9. Open questions

Frame field 8; flag `0x8000000` (seen on one corpus picture frame);
field 16's second value and the vertical offset of Flow with text frames; `[frmlay]` values 3 and 6; the `0x200`, `0x10000`, and
`0x100000` flags; tab types 2–4 in frames; the 653-for-720 tab offset in a
frame; custom shadow depth; the line-style list; the remaining `<*X>` bytes
in text (0x08, 0x10–0x12, 0x15, 0x1A); the effect of `<:c>` in multi-column
layouts; a multi-line paragraph with an anchored frame at a page end; the
`[isd]` numbers and `.X3` rendering format; `[tbl]` cell fields; `[pg]`
fields; pictures in table cells. Frames in headers are repeating frames
placed in the margin (user's guide, chapter 14), not frames inside header
text.
