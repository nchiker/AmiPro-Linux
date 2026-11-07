# Page Layout Setup Progress (live)

The live state for the page layout setup run
(`docs/LAYOUT_SETUP_RUNBOOK.md`). Update it after every slice and before
any long step, so that a resumed or compacted session can continue.

Status values: `TODO`, `IN PROGRESS`, `DONE`, `BLOCKED` (with the reason),
and `WAITING FOR USER` (only for a question that came up during the run;
the run's own decisions L1–L4 were taken before it started).

**Run start tag:** `layout-run-start` (create at HEAD when the run starts).

**Baseline before the run** (fill in at the start): round-trip audit
(user's corpus with `AMIPRO_STYLES`, ISO corpus, VM fixtures; read back
equal and "Save As would list nothing"), `layout_comparison_report` on the
user's corpus and the VM fixtures (page counts, line counts, page ends),
the page surface typing benchmark, and the test counts. At the end of
Phase 4 (2026-10-13): read back equal 416/416, 159/160, 136/136, Save As
listing nothing for the same; user's corpus 302 of 394 page counts, 14,400
of 14,871 line counts, 762 of 1,108 page ends; VM fixtures 136/136, 885 of
896, 76/76; core 236 + 2 ignored; app 175, 106, 182 with 10, 4, 10
ignored.

**Measured at the run's start** (`layout-run-start` = `af9d6a5`,
2026-10-13): read back equal and "Save As would list nothing": user's
corpus 417/417 (one more readable file than at Phase 4's end: 424 SAM
files, 7 not SAM or damaged), ISO 159/160, VM fixtures 136/136; read-back
warnings 0. `layout_comparison_report`: user's corpus page counts 302 of
394, line counts 14,880 of 15,356, page ends 762 of 1,108; VM fixtures
page counts 136/136, line counts 885 of 896, page ends 76/76; ISO corpus
page counts 65 of 160, line counts 22,773 of 24,971, page ends 492 of
1,307. Tests: core 236 + 2 ignored; app 175, 106, 182 with 10, 4, 10
ignored (`tools/verify.sh`, all PASS).

## The map (from PL.1)

Evidence and the full rules: `docs/research/PAGE_LAYOUT_NOTES.md`
(decompiled `AMIPROUI.DLL`, `AMIUICNT.DLL`, `AMIPRO.EXE`). Lengths are
typed in inches, two decimals read, truncated to twips; above 22 in is
invalid. "Fixture" is the twin (made in Ami Pro, then in the app).

| Ami Pro setting | Rule (decompile) | Model and command | Panel (§18) | Twin |
|---|---|---|---|---|
| Pages: All, Right, Left | All copies right onto left (0x200); Right/Left starts left as a copy | `PageLayout::left`, `ModifyLayout` | Pages radio | G15, PL05 |
| Mirror | only with Right/Left; margins exchanged, columns reversed, one-sided lines moved (`1248:110d`) | `PageLayout::mirror`, left computed, `ModifyLayout` | Mirror check box | G16, G17, PL06 |
| Paper: Letter, Legal, A4, A3, A5, B5 | stored as the boxes give them (A4 11,908 × 16,833); refused if margins and columns do not fit | `paper`, `PageGeometry::{width,height}` | Page › Size list | G11, G13, PL01 |
| Custom W × H | W ≥ L + R + n + (n−1)g, H ≥ T + B + 1, ≤ 22 in | the same, paper 7 | Page › width, height | G14, PL02 |
| Portrait, Landscape | swaps W and H (0x100); refused if it does not fit | `landscape` | Page › radio | G12, PL03 |
| Left, Right, Top, Bottom margins | ≥ 0; each column ≥ 1 twip; T + B < H (app; Ami Pro ≤) | `LayoutBox::margins` | Margins and columns | G01–G04 |
| Columns 1–8 | even widths (W − L − R − (n−1)g)/n; 1 column: no gutter, no balance | `columns`, edges derived | Margins and columns | G05, PL04 |
| Gutter | 240 default; max (W − L − R − n)/(n−1) | `gutter` | Margins and columns | PL04 |
| Column balance | only with 2+ columns | `balance` | Margins and columns | G06 |
| Page tabs (type, leader, position) | one list per box, ≤ 22 stops, stored after column 1 | `LayoutBox::tabs[0]` (engine: every column) | Tabs | C03-like, PL07 |
| Lines around page: sides, style, position | All or a side mask; 12 styles; Inside default | `PageGeometry::border` | Lines | G07, G08 |
| Line between columns, style | drawn with 2+ columns | `LayoutBox::line_between` | Lines | G09 |
| Line colour | one colour for both | `Border::colour`, `ColumnLine::colour` | Lines | G10 |
| Header: margins, columns, gutter, balance, tabs | within the band (band = top margin); band margins shrink to fit | header frame's `FrameStyle::interior`, `SetFrameStyle` | Header | PL08 |
| Header: Begin on second page | 0x800 | `header_from_second_page` | Header | G19 |
| Footer: the same | band = bottom margin; 0x1000 | footer frame, `footer_from_second_page` | Footer | G23, PL09 |
| Insert Page Layout: Insert | copy of the caret page's layout + `<:p</@>NAME>`; not where the paragraph already ends with one | `InsertLayout` | Insert Page Layout button | G21, PL10 |
| Remove, Revert | jump table: VM at PL.8 | `RemoveLayout`, `InsertBreak` | Remove button; Layout list | PL11, PL12 |
| Page break, column break | main text; centre with page break only | `InsertBreak` | Insert › Page Break, Column Break | G20, V22, V23 |
| Floating header/footer | Insert: header or footer, All/Right/Left; Remove where one is | `InsertFloatingHeaderFooter`, delete the object | Insert › Floating Header/Footer… | G22, V17, V18, PL13 |
| Page number: style 1 I i A a | stored as the character | **gap**: `PageNumberField` | Insert › Page Number… | G18, PL14 |
| Start on page, Start with number | 0 or 1–9999; blank before the start page | **gap** | the same | PL15 |
| Leading text | ≤ 30 characters | `leading_text` | the same | G18 |
| Units (values 3 and 6) | 1 in, 2 cm, 3 picas, 4 pt; the app writes 1 | kept | none (inches) | — |

Gaps for PL.2 (notes, "What this means for the app"): the page-number
field; one tab list per box in the engine; Ami Pro's limits in the
commands; band margins after a margin change; Mirror; even columns after
an edit; one line colour.

## Slices

| Slice | Status | Commits | Evidence | Notes |
|---|---|---|---|---|
| PL.1 The map (evidence, no product code) | DONE | (this commit) | `docs/research/PAGE_LAYOUT_NOTES.md`; `AMIPROUI.DLL` and `AMIUICNT.DLL` decompiled (outside Git); `tools/ne_resources.py` | Remove and Revert's actions are behind an unresolved jump table: VM fixtures at PL.8 |
| PL.2 Model and command gaps | DONE | `6277575`, (this commit) | `amipro_editing::layout_setup` (8 tests from G11–G17 and the decompile); `InsertPageNumber`, `SetPageNumber`; the random undo/redo property test covers both and rule-driven `ModifyLayout`; round trip and layout comparison unchanged | `ModifyLayout` now carries the header and footer interiors (`BandBoxes`), so a margin change and its band shrink are one command |
| PL.3 The panel's frame | DONE | (this commit) | `page_layout_properties.rs` (2 unit tests); `page_layout_properties_set_the_page_up_live_one_undo_step_each` (through the app's messages); `presses_in_a_side_margin_offer_and_open_page_layout_properties` (the widget path); screenshots `window-pagelayout-*-light/-dark` (`page_layout_properties_screenshots`) | All sections' controls are in, through `layout_setup`; PL.4–PL.7 add what each needs beyond them (drawing, Go To Header/Footer) and their VM twins |
| PL.4 Page | DONE | (this commit) | Twins G11–G14 (Legal, Landscape, A4, Custom): layout records equal to Ami Pro's but for A4's known 3 twips; Ami Pro shows the app's twins pixel-identical to its own and re-saves them the same | Paper sizes as Ami Pro stores them; A4/A3/A5/B5 columns: Ami Pro's dialog sets them from its table's width and stores the box's (up to 5 twips apart); the app keeps them even with the margins |
| PL.5 Margins, columns, and tabs | DONE | (this commit) | Twins G01–G06, G15–G17 (margins, columns, balance, left pages, Mirror): records equal, screens identical, re-saved equal | `copy_frame` no longer copies a frame's sequence number, so left headers and footers number 3 and 4 as Ami Pro's do (G15) |
| PL.6 Lines | DONE | (this commit) | Twins G07–G10: records equal, screens identical; probes AP1–AP5 measured on Ami Pro's screen; `page_lines_sit_where_ami_pro_draws_them_ap1_to_ap5`; the app's AP3 window matches | Page lines and the line between columns are drawn now (`PageBox::strokes`); they did not exist before |
| PL.7 Header and footer | DONE | (this commit) | Twins G01, G02 (band rectangles follow the margins), G19, G23 (Begin on second page): equal; the app test covers Go To Header and Footer and the band section opening | Edit › Go To Header and Go To Footer; the band shrink is `layout_setup`'s (PL.2) |
| PL.8 Inserted page layouts and breaks | DONE | (this commit) | VM fixtures PB1, PB2, PB3, CB1, IL1, IR1, IV1 (made in Ami Pro); twins in `layout_twins.rs` equal in body and layouts (6 of 6); `breaks_and_inserted_page_layouts_go_where_ami_pro_puts_them` (the app's messages) | Revert is Insert Page Layout with an earlier layout chosen (§18); the caret's page now follows where it is drawn |
| PL.9 Floating headers and footers | DONE | (this commit) | VM fixtures FH1–FH3 (made in Ami Pro); twins equal (3 of 3); `floating_headers_insert_or_remove_where_ami_pro_does`; `floating_headers_and_page_numbers_go_in_through_their_dialogs`; screenshots `window-floating-*` | Insert › Floating Header/Footer… (the registry's Header or Footer placeholder) |
| PL.10 The page-number field | DONE | (this commit) | VM fixture PN1 (`<:Pi2,1,Page >` in the header, Ami Pro's dialog showing nothing on page 1); twin equal; dialog and app tests; screenshots `window-pagenumber-*` | Insert › Page Number…; at a field the dialog changes it (§18) |
| PL.11 Both formats, edited documents, and the VM | DONE | `268de1e`, (this commit) | `a_page_layout_set_up_in_the_app_reads_back_in_every_format`; SETUP in the VM (3 pages as ours; re-saved with the same records and body; new documents from SETUP.STY and SETUPC.STY); round trip and layout comparison unchanged; timings below | Found: Ami Pro balances columns at a page break and draws the line between balanced columns only as tall as they are; kept page-line values beside lines the model holds |
| PL.12 Exit | DONE | (this commit) | `UI_SPEC.md` §18 implementation notes; `FEATURE_PARITY.md` PG-01, RF-01; `UI_COMMAND_MATRIX.md`; `DECISIONS.md`; `PROJECT_PLAN.md`; `PROJECT_STATUS.md`; `CLAUDE.md`; handoff | Waiting for the user: the physical check (handoff, ranked list) |

## Log

(Newest last: what was done, measured, decided, or found.)

- 2026-10-13: the run set up (`docs/LAYOUT_SETUP_RUNBOOK.md`) after the
  user's answers to `UI_SPEC.md` §18 (L1–L4 in `DECISIONS.md`) and the
  direction to use the AMIPRO.EXE and DLL disassemblies as authoritative
  sources while conforming to the new interface. The 2026-10-13 VM
  screenshots of Ami Pro's Page menu and Modify Page Layout panels are in
  `../AmiPro-Reference/vm-lab/screens/pl_*.png` (outside Git).

- 2026-10-13, PL.1: the dialogs are in `AMIPROUI.DLL` (packed like
  `AMIPRO.EXE`), the number boxes in `AMIUICNT.DLL`; both decompiled.
  Found: `[rght]` values 3 and 6 are the dialog's units; a page box has one
  tab list (written after the first column); paper sizes are stored as the
  two-decimal boxes give them (A4 11,908 × 16,833, as G13); every limit;
  Mirror's rule; the band margins' shrink; and that the page-number field
  is a style character, a start page, and a start number (the model reads
  `10` as "style 10", and `<:Pi…>` not at all). Baseline measured (above).

- 2026-10-13, PL.2: the page-number field (style character, start page,
  start number; `6277575`) and one tab list per layout box in the engine:
  round trip 417/417, 159/160, 136/136 and every file's layout comparison
  row unchanged. Then `layout_setup.rs`, Ami Pro's rules over
  `LayoutSettings` (Pages, Mirror, limits, the band shrink, paper sizes as
  stored, lines with one colour, the bands), and `ModifyLayout` applying
  the bands' interiors and dropping kept unequal column edges. Choices
  where the model cannot hold what Ami Pro keeps (recorded for §18's
  notes): a line style or colour is set only while a line is on (Ami Pro
  keeps them with the line off); a top or bottom margin shrinks the band
  of the page kind edited (Ami Pro's dialog sets both kinds' band height
  from it); T + B must leave some height (Ami Pro allows none).

- 2026-10-13, PL.3: Format › Page Layout… (the File › Page Setup
  placeholder is gone, L2), Page Layout Properties in the rail at 300 px,
  replacing Style Properties when opened and replaced by it; the Layout
  list, Pages, Mirror, and the six folding sections (Page, Margins and
  columns, Tabs, Lines, Header, Footer) with their controls; a right press
  in a page's left or right margin puts Page Layout… first in the menu,
  and a double-click there opens the panel. Typed lengths read as Ami
  Pro's boxes read them (two decimals, truncated; 567 twips a
  centimetre). Finding for later: Style Properties' `parse_length` rounds
  and uses 1440/2.54 per centimetre, so a value like 8.27 in can differ by
  a twip from Ami Pro's; the same rule should go there (a consistency fix,
  not a user question). Found: page borders and the line between columns
  are not drawn by the engine or the surface (PL.6).

- 2026-10-13, PL.4–PL.7: `crates/editing/tests/layout_twins.rs` (ignored;
  `AMIPRO_VM_LAB`) makes G01–G23's page layout changes in G00 through
  `layout_setup`, as the panel makes them, writes them as SAM, and compares
  the layout records with Ami Pro's: 19 of 19 equal (A4's 3 twips noted).
  In the VM, Ami Pro opened each app-written twin (`AGnn.SAM`) and its own
  fixture: the screens are pixel-identical in all 19, and Ami Pro re-saves
  the twins with the same layout records (`RGnn.SAM`). Two findings fixed on
  the way: copied frames kept the original's sequence number (left
  headers and footers, inserted layouts' frames), and page lines were not
  drawn. Where Ami Pro draws page lines: probes AP1–AP5 (notes, "Lines").

- 2026-10-13, PL.8: in Ami Pro (VM), a page break goes at the caret
  without splitting the paragraph (PB1, PB2), and at the end of the text an
  empty paragraph follows (PB3, as G20 and G21); a column break goes at
  the start of the caret's paragraph and acts after it (CB1, as V22);
  Insert Page Layout puts a break naming a copy (`1`) at the caret and
  opens Modify Page Layout on it (IL1); Remove on the inserted layout's
  page takes the break and the layout in one step, the caret where the
  break was (IR1); Revert puts a break naming Standard at the caret (IV1).
  Built: Insert › Page Break and Column Break (Ctrl+Enter,
  Ctrl+Shift+Enter, as approved), and the panel's From the next page list
  (a copy of the shown layout, or an earlier one again), Insert Page
  Layout, and Remove. Found and fixed: the status bar's page for a caret
  right after a page break was the earlier page while the caret was drawn
  on the next (`caret_page` now follows `line_of`).

- 2026-10-13, PL.9–PL.10: in Ami Pro, a floating header or footer goes
  at the caret and the caret goes into it on the next page (FH1, FH2);
  with the caret at a mark the dialog opens on Remove with that kind
  checked (FH3); Page Numbering wrote `<:Pi2,1,Page >` into the header
  (PN1), confirming the field's reading, and its sample was empty on
  page 1 (the start page rule). Built: `page_dialogs.rs` (Floating
  Header/Footer… and Page Number…, small one-shot dialogs, L3), each one
  undo step; the page-number dialog also changes a field at the caret.
  Twins: 10 of 10 equal (breaks, layouts, page number, floating headers).

- 2026-10-13, PL.11 timings (release, `page_layout_change_performance_baseline`,
  1280 × 800 window, update, view, layout, and draw of the next frame): a
  left margin typed in Page Layout Properties takes 47 ms median (54 ms
  p95) on README31 (344 paragraphs, 14 pages) and 123 ms median (136 ms
  p95) on the user's largest document (798 paragraphs, 68 pages); undo
  the same. The command's update is 1.8 and 7 ms; the rest is the page
  surface preparing the relaid pages. Typing (page surface baseline,
  README31, interleaved A/B against `layout-run-start`, three rounds):
  start 0.62–0.65 ms against 0.60–0.70, middle 0.78–0.87 against
  0.95–0.97, end 0.66–0.67 against 0.69–0.72 (median): unchanged.

- 2026-10-13, after the run (the user's question: how to change a page
  number's font in a header): in Ami Pro the field takes the text
  formatting where it stands (PF1); the app drew it in the paragraph
  style's font. Fixed: the engine measures the field in its run's format
  and the page surface draws it so. Layout comparison: no row changed
  (PF1 added and equal). The user's recheck of Save As SAM passed.

## Waiting for the user

Nothing. The physical check passed (2026-10-13: "I didn't find any
issues. Nothing feels off - it all looks amazing").

## Waiting for the user (at the run's end, kept for the record)

- The physical check of Page Layout Properties and the two dialogs
  (`docs/SESSION_HANDOFF.md`, item 1, with steps). No question came up
  during the run that needed the user.
