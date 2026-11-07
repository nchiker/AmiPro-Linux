# Tables across pages in Ami Pro 3.1: evidence (2026-10-18)

For the tables across pages run (`docs/TABLE_PAGES_RUNBOOK.md`, slice TP.1).
Decompiler output stays outside Git (`../AmiPro-Reference/ghidra-work/out/`);
these are findings in my own words. Evidence levels as in
`SAM_FORMAT_NOTES.md`: VERIFIED (decompile), VERIFIED (fixture), VERIFIED
(manual) for the 3.1 user's guide, chapter 16, CORPUS for both corpora
(aggregate counts only, no file names).

## What the user's guide says (VERIFIED (manual), ch. 16)

- **Rows span pages** (Modify Table Layout › Options): "In a page table,
  allows data in a cell to continue onto the next page without moving the
  entire row to the next page." Only available with Automatic row height.
- **Headings** (Table › Headings, a toggle on the caret's row): "If a table
  continues on multiple pages, you can create a heading for the table. Ami
  Pro automatically displays the heading as the first row on each page,
  above the rows that contain your data." "You can change the heading on
  subsequent pages by placing the insertion point in a different row and
  choosing Table/Headings. Ami Pro uses the new heading until you specify
  another one."
- **A page break in a table** (Page › Breaks › Insert page break with the
  caret in any cell of a row): "Ami Pro moves the current row to the next
  page and continues the table on that page." Remove page break works with
  the caret in the row immediately below the break.
- The other options of Modify Table Layout (Center table on page, Line
  around table, Honor protection, Disable mouse col/row sizing) belong to
  the table editing run.

## In the file

| Where | Value | Meaning | Evidence |
|---|---|---|---|
| `[tbl]` header value 7 | bit `0x1` | Automatic row height: a row grows to fit its text; without it a row keeps its stored height and its text is cut | VERIFIED (fixture) TP2E: rows of 282 under 284-twip lines stay 282 |
| `[tbl]` header value 7 | bit `0x100` | Rows span pages | VERIFIED (fixture) TP5: the dialog's check box sets 1 → 257 |
| `[h]` record value 4 (flags) | bit `0x10` | The row is a heading row | VERIFIED (fixture) TP2 (Table › Headings on row 0: 2 → 18); VERIFIED (decompile): the reader `1428:046b` sets the table's "has headings" bit (byte `0x14` bit 2) for any row with `0x10`, and `1468:0351`, `1468:044f`, `1468:05a8` test it |
| `[h]` record value 4 | bit `0x20` | A page break before the row | VERIFIED (fixture) TP6 (Insert page break with the caret in row 2: 2 → 34) |
| `[h]` record value 4 | bit `0x4` | The row's own gutter (value 3) counts | `CELL_FRAME_NOTES.md` |
| `[h]` record value 6 | 1–8 and more | Not read: the reader discards it (`1428:046b`) | VERIFIED (decompile) |
| `[pg]` record, third group, values 13 and 14 | first row, row count | Of the table whose anchor ends the page: the first row of its part on that page and how many rows the part holds (a repeated heading not counted; a row split by Rows span pages counted on both pages) | VERIFIED (fixture) TP1–TP6, TP2B–TP2E; CORPUS: 214 page records end in a table, 48 of them continue the previous page's table at the next row, one at the same row (a split row, Rows span pages) |
| The table frame's rectangle | — | The part of the table Ami Pro laid out last (usually the last page's; the first page's in TP2, saved with the caret there), with the frame's stale page field: a cache | VERIFIED (fixture) TP1, TP2 |
| A frame in a cell | page and rectangle | The page of its row's part and its place there | VERIFIED (fixture) TP7: page 2, the cell's text corner on page 2 |

Headings in the corpora (CORPUS): 49 heading rows in the ISO corpus (flags
18, 22), 1 in the user's (18); page breaks (`0x20`) in a handful of rows
(flags 34, 39, 62, 102, 110 and the like); Rows span pages in 14 of the
user's 389 tables (257, 385) and none of the ISO's; Automatic off in 63
of the user's tables and 4 of the ISO's.

## How a table continues (VERIFIED (fixture))

All fixtures are two-column tables of 4,569-twip columns, 86-twip gutters,
and 43-twip insets, on a letter page with one-inch margins (text area 1440
to 14400), after one-line Body Text paragraphs (284 twips). Written as
synthetic files into `IN`, opened in Ami Pro, and saved again (`…S`), or
changed in Ami Pro and saved.

- **Rows move whole** (TP1: 30 lines, a 40-row table): the table starts at
  9960 and page 1 holds rows 0–10 (43 + 11 × 370 = 4113, to 14073); row 11
  would end at 14443 and goes to page 2. A row fits when its text **and its
  gutter** fit: without the gutter it would end at 14357, inside the page.
  Page 2 holds rows 11–39 from the column's top, **with the top inset
  again**: 1440 + 43 + 29 × 370 = 12213, Ami Pro's stored rectangle.
- **A taller row moves whole** (TP3: a 16-line row): rows 0–5 on page 1,
  row 6 (4544) and the rest on page 2 (6523 = 43 + 4630 + 5 × 370).
- **A row taller than a page is cut to the page** (TP4: a 60-line cell):
  the row's height becomes 12,780, 45 lines, the most whole lines for
  which the inset, the row, and its gutter fit in the column (43 + 12,780
  + 86 = 12,909 of 12,960); lines 46–60 are not shown; the row stands alone
  on page 2 and the rest of the table on page 3.
- **With Rows span pages a row splits at a line** (TP5): the 16-line row
  shows lines 1–7 on page 1 (to 14297; an eighth line would end past 14314,
  the bottom less the gutter) and 8–16 on page 2, where its part is 2556
  (9 lines) tall. No grid line is drawn under the part on page 1 or above
  the part on page 2; the column lines run on. Each cell splits at its own
  lines (TP5B: an 18 pt cell beside a 12 pt one; the part on page 2 is as
  tall as the cell that needs most). A 60-line row splits over the pages
  it needs (TP4B: 37 lines on page 1, 23 on page 2). The row's stored
  height is then its last part's (2556; 6532 in TP4B), a cache.
- **The first row must fit below its paragraph** (TP10, TP11): when the
  first row does not fit after the anchor's paragraph, the table goes to
  the next page, and with it the paragraph that holds the anchor (TP11: the
  heading line "A heading that holds the table" moved to page 2, the table
  below it at 1724). The engine's With para above rule (V12) does this.
- **The next column first** (TP9: two columns): rows 0–26 in the first
  column, 27–39 at the top of the second (6300, 1440 to 6293).
- **Headings** (TP2, TP2B, TP2C, TP2D): on each page after the first, the
  heading rows are drawn first, then the rows that continue. The heading
  is the last run of consecutive heading rows before the page's first row
  (`1468:044f` walks the rows up to it; `1468:05a8` extends a run): rows 0
  and 1 together on page 2 of TP2B, row 40 alone on pages 3 and 4; row 5
  (not the first row) repeated from page 2 on in TP2C. A heading row in
  its own place is drawn there as any row, and a part that starts with a
  heading row still repeats the earlier heading above it (TP2F: headings
  0 and 11, page 2 starts at row 11 and shows row 0 above it). A page that repeats a heading
  holds one inset (43) less: with fixed rows of 368 (TP2D, TP2E), page 2
  holds 35 rows without a heading and the heading and 33 rows with one
  (43 + 43 + 34 × 368 = 12,598; 35 rows would need 12,966 of 12,960).
  The grid on screen has no gap between the heading and the next row
  (TP2C at Enlarged: the line under the repeated heading at 370 twips from
  the page's top, the next 370 below), so the extra inset is below the
  part; Ami Pro's stored rectangle includes it (TP2B's last part 1936 =
  86 + 5 × 370).
- **A page break in a row** (TP6, flag `0x20` on row 2): rows 0–1 on page
  1, then page 2 starts with row 2 (and holds 34 rows), page 3 the last 4.
- **Pictures in cells** (TP7): a picture imported into cell (20,1) on page
  2 is written with page 2 and its cell's text corner there (6138, 4813 =
  1440 + 43 + 9 × 370).
- **A table in a frame** (TP8: a 2 × 10 table made in a 3 × 1 in frame):
  the frame keeps its size and shows the rows that fit; the rest is hidden
  (Ami Pro's text frames hide text that does not fit the same way). Frame
  tables never continue.

## Where it lives in AMIPRO.EXE (VERIFIED (decompile))

| Routine | Role |
|---|---|
| `1428:046b` | Reads a `[h]` or `[w]` record: index, size, gutter, flags, then for rows two more (value 6 discarded); a row with flag `0x10` sets the table's "has headings" bit; the row goes to `13d8:04ea` with its flags |
| `1468:0351` | Whether row n is a heading row (flag `0x10` in the row's record) |
| `1468:044f`, `1468:05a8` | The heading for a page: the last run of consecutive heading rows before a given row, returned as its first and last rows |
| `14e0:244f` | Moving the caret in a table (Up, Down, line ends) past a repeated heading |
| `11c8:381e` | The `[pg]` writer: each page record, the third group from the page record's words 10–26, the last two being the first row and the count of the page-end table |
| `12f8:1321`, `14f0:087a` | The pagination step, which builds the page's lines through `10d0:0d82`; the table part's own split was not traced to its routine (the fixtures above give the rule) |

## The app before this run

- An anchored table that does not fit in the rest of its column keeps its
  stored height (a cache of one page's part), and its later rows run over
  the text after it (`LAYOUT_COMPARISON.md`, item 7; `engine.rs`
  `anchored_height`).
- Rows always grow to their text (Automatic is not read), heading and
  page-break flags are kept as legacy data, and Rows span pages is not
  read.
- A frame table lays out all its rows; nothing clips them to the frame.

## Twins (TP.9, VM, 2026-10-19)

Made in the app (`table_page_twins`, ignored) from Ami Pro's own fixtures
and written as `ATP*.SAM` in `IN`; the app pages each as Ami Pro paged the
fixture made in Ami Pro, and its row flags and table header are Ami Pro's:

| Twin | Made from | The app's change | Opened and saved again by Ami Pro (`…S`) |
|---|---|---|---|
| ATP6 | TP1S | Insert › Page Break with the caret in row 2 (flag `0x20`) | Page table as TP6's: rows 0–1 on page 1, 2–35 on page 2, the last part 36–39 |
| ATP2 | TP1S | Row 0 a heading (flag `0x10`) | Page table as TP2's; the last part 1440 to 12,626, the repeated heading and its extra inset included |
| ATP5 | TP3S | Rows span pages (header value 7: 257) | Page table as TP5's: row 6 split, the last part 1440 to 5975 |


## The row cache (VM RC1–RC3, 2026-10-20)

The user decided on 2026-10-19 to trust Ami Pro's stored row height while
a row is as read, measured before merging. Three synthetic variants of
TP3S (an Automatic table of 12 rows; row 6 holds 16 lines, stored 4544)
were opened in Ami Pro and saved again without moving the caret or showing
the table's later page:

| Fixture | Changed | Ami Pro shows | Saved again (`…A`) | Shows |
|---|---|---|---|---|
| RC1 | Row 6 stored at 1136 (4 lines) | All 16 lines | Row 6 at 4544 | A height too small for the text is measured again |
| RC2 | Row 3 (one line) stored at 1136 | One line | Row 3 at 284 | A height too large is measured again: an Automatic row fits its text, the stored height is no minimum |
| RC3 | Rows 3 and 4 with empty cells, stored at 1136 and 0 | One empty line each | Both at 284 | An empty Table Text cell takes one line |

VERIFIED (fixture): Ami Pro measures every row of an Automatic table when
it opens a file and stores that measure, so in a file Ami Pro saved a
row's stored height is its measure of the row's text with its printer's
fonts. The stored height is a cache, as a paragraph's `<:#…>` is.

What follows for the app:

- **The row cache** (built 2026-10-20): while a row is as read
  (`Table::read_rows`, the row's fingerprint at import: its cells' text and
  styles, their columns' widths, the gutters inside joined cells, and
  Automatic), the row keeps the stored height when its text measures here
  at most half the row's gutter taller; the text then runs into the gutter,
  above the line between the rows. A taller measure is a stale cache, which
  Ami Pro would not keep either (RC1), and the row grows to its text as
  before.
- **Not built, for the table editing run:** an edited Automatic row should
  fit its text exactly, shrinking too (RC2), and an empty cell should take
  one line (RC3). Today a row never shrinks below its stored height, and an
  empty cell needs no height (TP.6: 63 rows of the user's corpus are stored
  at 0 in Automatic tables, 57 of them in tables whose header value 7 has
  bit 4, some with text in their cells, which RC3 does not explain).
- **The writer** writes the stored heights; Ami Pro measures the rows again
  on opening (RC1, RC2), so a row the app grew or edited needs no new
  height in the file.
