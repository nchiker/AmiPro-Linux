# Page layout setup in Ami Pro 3.1 (PL.1 notes)

**Written:** 2026-10-13, slice PL.1 of the page layout setup run
(`docs/LAYOUT_SETUP_RUNBOOK.md`). Findings only; no decompiler output.

**Sources and evidence levels.**

- VERIFIED (decompile): read in the decompiled code. Ami Pro's dialogs and
  their procedures are not in `AMIPRO.EXE` but in **`AMIPROUI.DLL`** (118
  dialog templates, 843 strings), which is packed with OPTLOADER like
  `AMIPRO.EXE` and unpacks with `tools/ghidra/optloader-unpack.py`
  (142 segments, 1,009 functions; `DECOMPILATION.md`). Its custom controls
  (the number boxes, unit buttons, colour bar, line-style lists) are in
  **`AMIUICNT.DLL`**, which is not packed. The application side (reading,
  writing, applying, the page-number text) is in `AMIPRO.EXE`.
- VERIFIED (fixture): seen in a VM fixture (G00–G23, V17–V23;
  `SAM_FORMAT_NOTES.md`, "Page layouts").
- Dialog resources (control IDs and labels) are read with
  `tools/ne_resources.py`.

## Where things are

| What | Where |
|---|---|
| Modify Page Layout dialog | `AMIPROUI.DLL` dialog 1034; `MODLAYOUTPROC` `1248:0000`; panels: Margins & columns, Header, Footer `1240:0000` (fill `1240:0555`, read back `1240:0812`), Page settings `1258:0000` (fill `1258:0858`, read back `1258:0729`), Lines `1250:0000` (fill `1250:02ce`, read back `1250:04ba`); shared helpers `1238` (gutter `1238:0000`, column count `1238:008a`, column edges after a margin `1238:02d0`, limits `1238:0573`, `1238:076f`); Pages and Mirror `1248:0a9d`, `1248:110d`; OK `1248:09f4`, `1248:0703` |
| Page Numbering dialog | dialog 1024; `PAGENUMPROC` `1140:0000` |
| Headers & Footers dialog | dialog 3037; `HEADERFOOTERGOTODLGPROC` `13c0:01fc` |
| Floating Header/Footer dialog | dialog 1124; `FLOATHEADFOOTPROC` `13c0:0000` |
| Breaks dialog | dialog 1044; `PGBRKPROC` `1150:0000` |
| Number boxes and units | `AMIUICNT.DLL`: `SAMMYSUBCLASSPROC` `1008:00dd`, text to twips `1060:0000`, twips to text `1060:0261`, range check `1030:0a72` |
| The dialog's data from and to `AMIPRO.EXE` | callback `1130:0000` (codes `0x6f`–`0x77`: one layout record each) |
| Layout box reader, writer | `1058:1843`; `11c8:236e`, `11c8:24d6` |
| Page-number field | parsed `11b8:030a` (`P`), split `11f8:023f`, text `1308:0026`, number format `1308:0249` (roman `1308:0b7f`, letters `1308:0caa`, the bullets' own) |
| Insert Page Layout enablement | `1180:24e0` (commands Insert 375, Remove 373, Revert 374) |

## The layout record

The dialog edits six records of 0x94 bytes: the right (or only) page, its
header, its footer, and the same three for left pages. Each has the shape
of a SAM LAYOUT block (`SAM_FORMAT_NOTES.md`, `[rght]` and `[frmlay]`
tables), field for field (`1058:1843` reads them in this order):

| SAM value | Record | Meaning |
|---|---|---|
| 1 | +0 | Page height (header and footer: the band's height, below) |
| 2 | +2 | Page width |
| 3 | +8 byte | **The unit the page size is shown in** (Page settings' unit button) |
| 4 | +4 | Left margin |
| 5 | +6 | Bottom margin |
| 6 | +9 byte | **The unit the margins are shown in** (Margins' unit button) |
| 7 | +0xa | Top margin (a page's; a band's is from the band's top) |
| 8 | +0xc | Right margin |
| 9 | +0xe byte | 1 Column balance, 2 Line between columns, 4 Lines around the page |
| 10 | +0xf byte | Line between columns: style (1-based in a 12-entry list) |
| 11 | +0x10 long | One colour for the page lines and the line between columns |
| 12 | +0x14 byte | Lines around the page: style (1-based, 12 entries) |
| 13 | +0x15 byte | Sides: 1 All, else 2 Left, 4 Right, 8 Top, 16 Bottom |
| 14 | +0x16 | Position: 1 Middle, 2 Inside, 3 Outside, 4 Close to inside, 5 Close to outside |
| 15 | +0x18 | Columns, 1–8 (more is a read error) |
| then | +0x1a | Column edges, left and right, absolute |
| then | +0x3a/+0x3c | **One tab list**, at most 22 stops |

Values 3 and 6 were "unknown" in the format notes: they are the units the
dialog last showed, 1 = inches, 2 = centimetres, 3 = picas, 4 = points
(`AMIUICNT.DLL` `1060:0000`; its unit names are strings 4–7). The app
writes 1, as Ami Pro's new documents do.

**Tabs: one list per box.** The writer writes the box's list after the
first column and 0 after every other column (`11c8:24d6`); the reader keeps
the first non-empty list it meets, at most 22 stops (`1058:1843`). G05's
`… 12 1 720 … 1 8640 6240 10800 0` is that shape, not one list per column.
The app's model keeps a list per column (`LayoutBox::tabs`) and the engine
gives each column its own (`engine.rs`), so a second column has no stops
where Ami Pro gives it the first column's (PL.2).

The layout number's bits (`SAM_FORMAT_NOTES.md`) are the dialog's word at
+6: the low seven bits the paper (1 Letter, 2 Legal, 3 A3, 4 A4, 5 A5,
6 B5, 7 Custom; VERIFIED (decompile) now for 3, 5, and 6), 0x100
Landscape, 0x200 All pages alike, 0x400 Mirror, 0x800 header and 0x1000
footer begin on the second page.

## Typed values and units (`AMIUICNT.DLL`)

- A length is typed in the box's unit with at most two decimals read
  (`1.505` reads as 1.50; other characters make it invalid), and converted
  to twips **by truncation**: whole units × unit + (hundredths × unit) / 100
  (1440 per inch, 567 per centimetre, 240 per pica, 20 per point).
- Shown with two decimals, rounded half up.
- Above 31,680 twips (22 in) a value is invalid.
- Each box has a minimum and a maximum (below). A value outside them, or
  one that does not read, is refused when the box loses focus: a beep, and
  the focus stays (`1008:00dd`, `1030:0a72`). No message box.
- **Values pass through the boxes.** Choosing a paper size puts its twips
  in the boxes and OK reads them back, so a size is stored as its two-decimal
  inches truncated: A4 is shown 8.27 × 11.69 and stored 11,908 × 16,833
  (G13; the dialog's own table says 11,905 × 16,838). Margins likewise.
  The app parses typed inches the same way (two decimals, truncated), and
  stores Ami Pro's stored sizes for the paper list.

## Pages: All, Right, Left, and Mirror (`1248:0a9d`, `1248:110d`)

- **All** (0x200 set): the right page, header, and footer are copied onto
  the left ones; one setting for every page.
- **Right** or **Left** while All was on: 0x200 is cleared and the left
  records start as copies of the right ones; then each change goes to the
  page kind chosen. Right pages are odd.
- **Mirror** (0x400) is available only with Right or Left, and while it is
  on All is not. With Mirror, every change to the page kind being edited is
  copied to the other kind mirrored: left and right margins exchanged,
  columns reversed (the first column of one is the last of the other, gaps
  kept), and a line on only the left or only the right side moved to the
  other side. Header and footer records are mirrored the same way.

## Margins and columns (`1240`, `1238`)

- Margins: Left, Right, Top, Bottom; minimum 0.
- **Columns 1–8.** A count change spreads the text width evenly:
  w = (W − L − R − (n − 1)·g) / n (integer division), the first column at
  the left margin, each next one a gutter after the previous one. One
  column hides the gutter and clears and disables Column balance.
- **Gutter**: 240 twips (0.17 in) by default and for one column; shown only
  when every gutter is the same (within one twip); a change respaces the
  columns evenly. Maximum (W − L − R − n)/(n − 1).
- **Limits** (each box's maximum): every column at least one twip wide:
  L ≤ W − R − n − (n − 1)·g, R likewise; T ≤ H − B and B ≤ H − T. A column
  count whose columns would not fit is refused (`1238:076f`).
- **A margin change** recomputes the columns: evenly when the gutters are
  alike (`1238:02d0`), otherwise keeping each column's width.
- **Top or bottom margin and the bands**: a page's top margin is its
  header's height (record +0 of the header record), its bottom margin its
  footer's. When the band becomes shorter than its own top and bottom
  margins, the band's bottom margin shrinks first, then its top margin,
  never below 0 (`1240:09ee`).

## Page settings (`1258`)

| Paper | Code | Dialog's twips (portrait W × H) | Stored after the boxes |
|---|---|---|---|
| Letter | 1 | 12,240 × 15,840 | the same |
| Legal | 2 | 12,240 × 20,160 | the same (G11) |
| A4 | 4 | 11,905 × 16,838 | 11,908 × 16,833 (G13) |
| A3 | 3 | 16,838 × 23,810 | 16,833 × 23,803 |
| A5 | 5 | 8,391 × 11,905 | 8,395 × 11,908 |
| B5 | 6 | 9,978 × 14,173 | 9,979 × 14,169 |
| Custom | 7 | typed | typed |

- **Landscape** (0x100) swaps width and height (G12). Choosing a paper or
  an orientation that the margins and columns do not fit is refused
  (`1258:0a76`, `1258:0b64`).
- **Custom**: width at least L + R + n + (n − 1)·g, height at least
  T + B + 1, both at most 31,680 (22 in). Typing in a custom box selects
  Custom.
- The page size also sets the width of the header and footer records (and
  their columns) on both page kinds (`1258:0335`).

## Lines (`1250`)

- Around page: All, Left, Right, Top, Bottom (All clears the four; a side
  clears All; none checked means no lines). Style: a list of 12 line
  styles. Position: Inside (the default), Close to inside, Middle, Close to
  outside, Outside.
- Line between columns: on or off, with its own style from the same list.
  It is drawn only with two or more columns.
- One colour (the colour bar) for both.
- **Where Ami Pro draws them** (VERIFIED on its screen, probes AP1–AP5,
  2026-10-13: written by the app with two-inch margins, a 4 pt line, two
  columns with a one-inch gutter, and opened in Ami Pro): **Outside** puts
  the line's outer edge on the page's edge; **Inside** its inner edge on
  the margin; **Close to outside**, **Middle**, and **Close to inside**
  centre it a quarter, a half, and three quarters of the margin in from
  the edge, each side by its own margin (the callback that computes it,
  code `0x79` from `AMIPROUI.DLL`'s preview, goes through a dispatcher the
  decompile does not resolve). The line between columns is centred in each
  gutter and runs the columns' full height, text or not. Neither moves the
  text (G09).

## Header and footer (`1240` with the band's record)

- The same panel as Margins & columns, on the band's record: its own tabs,
  Left, Right, Top, Bottom margins **within the band**, columns, gutter,
  Column balance, and **Begin on second page** (header 0x800, footer
  0x1000).
- The band is the page's top (bottom) margin tall; its own margins are
  limited by it (top + bottom ≤ the band). New documents: header top 0.55
  in, bottom 0.05 in; footer the reverse (G00).
- Message 3013 (`AMIPROUI.DLL` strings): a header set to begin on the
  second page cannot be edited from the first page.

## Insert Page Layout (`1180:24e0`; VM for the actions)

- **Insert** (375) is enabled unless the caret's paragraph already ends
  with a page break that names a layout. It inserts a copy of the caret
  page's layout and a page break naming it (G21: `<:p</@>1>` and a layout
  named `1`), then opens Modify Page Layout on it (string 375).
- **Remove** (373): "Remove an inserted page layout from the current
  document"; enabled when the caret's page has an inserted layout, or when
  the document has more than one layout. **Revert** (374): "Revert to the
  standard page layout"; enabled when the caret's page layout is not the
  standard one. Their actions go through a jump table the decompile does
  not resolve: VM fixtures at PL.8.

## Floating headers and footers (`13c0:0000`)

- Insert (default) or Remove. Insert: Floating header (default) or footer,
  for All pages (default), Right pages, or Left pages. It takes effect from
  the next page (G22).
- Remove is enabled only where a floating header or footer is at the
  caret; then a check box for each kind present there.
- The stored flag byte (1 header, 2 footer, 4 left pages, 8 right pages)
  is VERIFIED (fixture) V17, V18.

## Page numbering (`1140:0000`, `1308:0026`)

- **Style**: 1, I, i, A, a (strings 2353–2357). Stored as that character.
- **Start on page** (check box, 1–9999) and **Start with number** (check
  box, 1–9999); each 0 when its box is clear.
- **Leading text**: up to 30 characters.
- **Stored** as `<:P` + style character + start page + `,` + start number
  + `,` + leading text + `>` (`11b8:030a`, `11f8:023f`). G18's
  `<:P10,0,Page >` is style `1`, start page 0, start number 0, text
  `Page `. The corpora have `<:P10,0,…>` (34), `<:P11,1,…>` (34),
  `<:P12,1,…>` (43), `<:Pi1,1,…>` (5), `<:PI0,0,…>` (1), and others.
- **The number shown on page p** (`1308:0026`): nothing before the start
  page P; with a start number N, p − P + N, or p + N − 1 when P is 0;
  otherwise p. Then leading text + the number in the style: 1 decimal,
  i and I roman, a and A letters (a … z, aa …), the bullets' formats.
- **Formatting** VERIFIED (fixture) PF1 (2026-10-13, from G18): the field
  selects as one unit and takes the text formatting where it stands, like
  a character. Text › Font at 24 pt red made the number 24 pt red and
  the header line taller, written as
  `<:f480,2Times New Roman,255,0,0><:P10,0,Page ><:f>`. The app measured
  and drew the field in its paragraph style's font until then; it now
  takes the run's face, size, colour, and lines (`Shaper::measure_text`).
- The app's model reads the first number after `P` as a numeric style
  (`PageNumberField { style: u16, start, leading_text }`), so `10` is
  "style 10", a roman or letter style does not read (11 corpus fields are
  kept as unmapped marks and show nothing), and the engine ignores the
  style and the start page (PL.2).

## Breaks (`1150:0000`)

- Insert page break, Remove page break, Insert column break, Remove column
  break, and Vertically center text above page break (with Insert page
  break only). Each is enabled by what is at the caret: a page or column
  break can be inserted in the main text, removed where one ends the
  caret's paragraph.

## What this means for the app (gaps for PL.2 onward)

1. **Page-number field** (PL.2, PL.10): model the style character, start
   page, and start number; read `<:Pi…>` and `<:PI…>`; write them back
   byte for byte; draw the number by `1308:0026`.
2. **Tabs per box** (PL.2): the engine should give every column the box's
   one list, the first non-empty one as Ami Pro's reader keeps; the panel
   edits that one list and stores it on the first column.
3. **Validation** (PL.2): margins, columns, gutter, paper, orientation,
   and custom sizes by the limits above; the model's own check refuses
   T + B ≥ H (Ami Pro allows T + B = H and then cannot place text, message
   3007); the app keeps the stricter rule.
4. **Bands** (PL.2, PL.7): a top or bottom margin change shrinks the
   band's own margins as `1240:09ee` does; a page size change resizes the
   bands.
5. **Mirror** (PL.2, PL.5): compute the other page kind as `1248:110d`
   does.
6. **Columns after an edit** (PL.2): a change to margins, columns, or
   gutter makes the columns even again; kept unequal edges are dropped.
7. **One colour** for page lines and the line between columns (PL.6).
8. **Paper sizes** as Ami Pro stores them (PL.4).

## The current ruler (VM RU1, RU2, 2026-10-13)

For the ruler proposal (`docs/Design/UI_SPEC.md` §19). Both fixtures start
from PF1 (G18 with a 24 pt page number), caret in its second paragraph.

- View › Show Ruler shows the caret column's tabs (the Standard layout's
  12 left stops), the left indent at the margin, the right indent at the
  right margin, and the caret's x as a dotted line. The first click on
  the ruler only enters ruler mode: the menus grey out and a row appears
  (four tab types, a leader button, Cols 1–8, a unit button, Clear Tabs,
  and a position readout from the paper's left edge).
- **RU1**: a second click at 3.24 in on the top half added a left tab;
  the paragraph was written `<:R1,13,1,720,…,1,3228,…,1,8640,>`: its own
  ruler, the column's 12 stops copied and the new one at 3,228 twips from
  the margin (not on a ruler mark: mouse clicks do not snap; the guide's
  snapping is for keyboard moves). The next paragraph has none.
- A click on the bottom half selected the left margin (readout 1.00 in).
- **RU2**: Cols 2 from the ruler made the Standard layout two equal
  columns; dragging in the bottom half from the gap (4.25 in) to 5 in
  moved it: `[lay]` Standard now has columns `1440 7050` and `7290 10800`
  (5,610 and 3,510 twips wide, the 240-twip gutter kept). The change is to
  the page layout itself, not an inserted one.
- Not yet found in `AMIPRO.EXE`: the ruler's own code (hit testing, the
  drags). "Ami Tab Ruler Format" is registered as a clipboard format.
