# Layout Comparison with Ami Pro (Phase 2.5d)

**Date:** 2026-10-12. **Code:** `crates/ami-formats/src/oracle.rs` (reads
Ami Pro's layout caches), `app/cosmic/src/layout_comparison.rs` (the
ignored report `layout_comparison_report`, run in release with
`AMIPRO_VM_LAB`, `AMIPRO_CORPUS`, and `AMIPRO_STYLES`, the style-sheet
path). Private inputs: the VM fixtures in `../AmiPro-Reference/vm-lab/OUT` (92,
then 102 with I1–I3 and F14–F20 on 2026-10-12),
the 160-file ISO corpus, and, from 2026-10-12, the user's 423-file corpus
(below); no original bytes, file names from the user's corpus, or text are
in Git.

## Oracles

Ami Pro writes what it laid out when it saves. The importer ignores these
caches (`SAM_PAGE_FRAME_SPEC.md` §8); the comparison reads them:

| Cache | Where | What it gives |
|---|---|---|
| Page table | `[pg]`: the page count, then one record per page | The page's last line: main-text paragraph (1-based), the line's start and length in Ami Pro's internal characters (a control takes several), a flag word (1 = last page), the layout's name |
| Paragraph cache | `<:#H,W>` at the start of a paragraph | The paragraph's height and nominal width in twips. The height excludes space above and below; the width is the column, not the indented measure |
| Anchored frames | `[frm]` fields 1 and 3–6 | Where Ami Pro last placed the frame (the page field is stale for anchored frames) |
| Frame columns | `[frmlay]` column edges | Absolute x of each interior column |

Ami Pro in the VM prints to the Microsoft XPS Document Writer (`[prn]`), so
its caches follow that printer's metrics.

## Fonts

Installed on the host: Liberation Serif, Sans, Mono, and Sans Narrow
(metric-compatible with Times New Roman, Arial, Courier New, and Arial
Narrow), the URW base 35 fonts (`fonts-urw-base35`: Nimbus, P052, C059, URW
Bookman, URW Gothic, Z003, metric-compatible with the PostScript core
fonts), Noto Serif and Sans, DejaVu, Noto Arabic and Hebrew. The COSMIC Text
shaper (`app/cosmic/src/page_layout.rs`) maps:

- Times New Roman, Times, Tms Rmn to Liberation Serif; Arial, Helv,
  Helvetica to Liberation Sans; Courier New and Courier to Liberation Mono;
- an installed family to itself;
- since 2026-10-12, an uninstalled face to an installed metric-compatible
  one: Palatino and Book Antiqua → P052; Century Schoolbook and
  NewCenturySchlbk → C059; Bookman, ITC Bookman, and Bookman Old Style → URW
  Bookman; AvantGarde and Century Gothic → URW Gothic; Arial Narrow and
  Helvetica-Narrow → Nimbus Sans Narrow; Zapf Chancery → Z003. These are
  fontconfig's groups, which COSMIC Text's font database does not apply,
  plus Book Antiqua and Century Gothic, which the comparison supports (the
  user's corpus, below);
- any other by kind from its name, as whole words (`Monotype Corsiva` is no
  longer monospaced; `Swiss742SWC` and `Frank Goth Cd` are sans).

`[fnt]` also stores the face's kind (`SAM_FORMAT_NOTES.md`, bits 14–15 of
value 4), which could replace the name heuristic in Phase 3.

## Line height

LH1 (a VM fixture made for this comparison, `SAM_FORMAT_NOTES.md`) gives Ami
Pro's single line spacing for Times New Roman and Arial at 8–72 pt. It is
the font's ascent, descent, and line gap (Windows' text height plus external
leading; 2,355 / 2,048 em for both families) with the printer's rounding.
The shaper now uses the same: each glyph's ascent includes its font's line
gap. Ours against Ami Pro's, twips:

| Size (pt) | 8 | 10 | 12 | 14 | 18 | 24 | 36 | 48 |
|---|---|---|---|---|---|---|---|---|
| Times New Roman, Ami Pro | 189 | 234 | 284 | 329 | 423 | 560 | 828 | 1,104 |
| Times New Roman, ours | 184 | 230 | 276 | 322 | 414 | 552 | 828 | 1,104 |
| Arial, Ami Pro | 185 | 228 | 277 | 323 | 415 | 551 | 830 | 1,104 |
| Arial, ours | 184 | 230 | 276 | 322 | 414 | 552 | 828 | 1,104 |

Courier New 10 and 12 pt: Ami Pro 211 and 257, ours 227 and 272 (+7%).
Times New Roman 12 bold italic: 279; Arial 10 bold: 237.

## Results

### VM fixtures (92 files; 102 since 2026-10-12)

| Measure | Result | Tolerance | Verdict |
|---|---|---|---|
| Line count of each cached paragraph, broken at Ami Pro's own width | 773 of 815 equal (95%), all within one line; 812 of 815 (100%) with the styles' indents taken off the width (below). With the 10 new fixtures: 778 of 820, and 817 of 820 with indents | ≥ 90% equal, all within one | Within |
| Paragraph height where line counts agree | single spacing 2.0% mean; other modes 4.6% | single spacing ≤ 3% | Within |
| Page count | 90 of 92 equal as imported; 91 of 92 with Ami Pro's paragraph spacing (100 of 102 with the new fixtures) | Equal unless explained | AP31NAV, README31 explained below |
| Page ends on the same page | 15 of 65 as imported; 27 of 65 with Ami Pro's paragraph spacing, and 26 of 26 outside AP31NAV | Same page once spacing is equal | Within, except AP31NAV (explained) |
| Text-frame column edges | 52 of 59 exact; 1–9 twips in the other 7 (L08's heavier line; F07, V05, V13, V20, and Z05's Flow with text frames, whose x follows the text before them) | ≤ 10 twips | Within |
| Anchored frame position, with Ami Pro's paragraph spacing | 16–24 twips (F06, F07, P01, TB1, V05, Z05, and since 2026-10-12 F16–F20, including Flow with text frames that hang below the baseline: one 8-twip line residual per Times New Roman line above); 64 (V20); 226–276 after about 30 lines (V12, V13) | The line-height residual times the lines above | Within |
| Anchored frame size | exact | exact | Within (after the table fix) |

Selected rows (`lines` equal and within one of n; page ends on the same page
as imported, and with Ami Pro's paragraph spacing):

| Fixture | Pages Ami Pro / ours / spaced | Lines | Page ends | Spaced | Notes |
|---|---|---|---|---|---|
| README31 | 14 / 15 / 14 | 332 (336) of 336 | 4 of 13 | 13 of 13 | Exact line spacing (240) is not imported; with spacing every page end matches. 334 lines equal with indents |
| AP31NAV | 40 / 33 / 31 | 275 (312) of 312 | 2 of 39 | 1 of 39 | Body Text has a 2,088-twip right indent and keep rules (below). 312 of 312 lines equal with indents |
| MERCURY | 8 / 9 / 8 | 121 (126) of 127 | 4 of 7 | 7 of 7 | 126 lines equal with indents |
| LH1OUT | 3 / 3 / 3 | 32 (33) of 33 | 0 of 2 | 2 of 2 | |
| V12, V13 | 2 / 2 / 2 | 42 of 42 | 0 of 1 | 1 of 1 | The fixed 6 pt after each paragraph moves the break |
| G20, G21, F13, V01, V10, V11, V22, V23 | equal | — | all | all | Breaks, inserted layouts, fixed frames on later pages |
| TB1 | 1 / 1 / 1 | — | — | — | Was 1 / 2 before the table-frame fix |

### Corpus (160 files)

| Measure | Result |
|---|---|
| Page count | 29 equal (18%), 62 within one (39%), 33 within 10% (21%); with Ami Pro's paragraph spacing 35 equal (22%) |
| Line count at Ami Pro's width | 21,003 of 24,142 equal (87%), all within one line; 21,045 with the styles' indents |
| Paragraph height where line counts agree | single spacing 943 paragraphs, 2.4% mean; other spacing modes 619, 2.6%; styles from a missing style sheet 19,441, 21.8% |
| Page ends on the same page | 228 of 1,306 (17%); with spacing 245 (19%) |

(2026-10-12, after the font substitutes and the line-height estimate below:
Palatino, in 5,788 compared paragraphs, now uses P052; before, 28 page counts
and 20,474 of 24,104 line counts were equal, and 217 and 235 page ends.)

The corpus is not a layout oracle yet: 143 of its 160 documents take their
styles from `SMARTEXT.STY` or `PRO21.STY`, which are not on the ISO, so 93%
of the compared paragraphs are set in fallback styles. Where a document
defines its styles, line breaking and line heights agree as in the fixtures.
The corpus caches also come from other printers.

The With para above fixes (below) moved the corpus page-end counts from 202
and 219: MERCURY.SAM gained one page end as imported and three with spacing,
and eight documents whose styles come from a missing style sheet lost one
each. Those eight are already 4 to 7 pages shorter than Ami Pro, and a
table's anchor paragraph that now takes no height shortens them a little
more.

The table fixes (README31.SAM, below) then moved them to 217 and 235 and the
equal page counts from 26 to 28: ten documents changed, seven for the
better (CH2.SAM now has Ami Pro's 25 pages and 9 page ends on the same page
instead of 4; CH01 14 instead of 7; RNCH10 8 of 8 instead of 2), and three
whose fallback fonts make cell text taller than Ami Pro's for the worse
(AP3CH21, AP_APPXD, CH07).

### The user's corpus (423 files, 2026-10-12)

The user supplied their own old documents (private, outside Git): 423 SAM
files in client folders and 192 style sheets in a `STYLES` folder. Every
document's styles resolve: 120 name a style sheet, all found with
`AMIPRO_STYLES` set to that folder, and the rest carry their styles. 394
documents carry Ami Pro's page table, so this corpus is the layout oracle the
ISO corpus could not be. Its caches come from the printers the documents
were last saved with (`[prn]`), not the VM's XPS writer.

The corpus audit (`tools/corpus.sh` with `AMIPRO_CORPUS` and
`AMIPRO_STYLES`): 416 of 423 import, all 416 round-trip exactly through
native v3, 192 of 192 style sheets parse, 21,076 paragraphs, 3,085 frames,
587 anchors (none unresolved), 1,186 embedded payloads, no control leaks,
1,750 pages with the fixed shaper (slowest document 12 ms in release). The 7
that do not import are not Ami Pro documents or are damaged
(`SAM_FORMAT_NOTES.md`, "Files that are not ordinary SAM files"): 5 Lotus
Word Pro files named `.SAM`, 1 password-protected document, and 1 file cut
off at 2,048 bytes. The audit gate therefore fails on this corpus by design.
New here: 31 unsupported section kinds (including `master`, `toc`,
`docvars`, `fldnames`, `PanoseFontTable`), unknown frame flag bits
`0xc354228` (the ISO corpus has `0x8110200` of them), 36 anchored frames
without a mark (1 in the ISO corpus), and documents with up to 9 layouts.

The comparison (394 documents with a page table, 14,777 cached paragraphs):

| Measure | Result |
|---|---|
| Page count | 225 equal (57%), 355 within one (90%), 242 within 10% (61%); with Ami Pro's paragraph spacing 300 equal (76%) |
| Line count at Ami Pro's width | 13,398 equal (91%), 99% within one |
| Line count with the styles' indents | 14,290 equal (97%), all within one |
| Paragraph height where line counts agree | single spacing 12,362 paragraphs, 3.5% mean; other modes 749, 4.4%; styles neither the document nor its style sheet defines 287, 4.3% |
| Page ends on the same page | 548 of 1,097 (50%); with Ami Pro's spacing 674 (61%) |

By font (paragraphs compared, line counts equal at Ami Pro's width, mean
height difference): Times New Roman 3,920, 96%, 3.2%; Arial 3,680, 97%,
3.7%; Book Antiqua (P052) 4,739, 79%, 3.5%; Times 835, 100%, 1.1%;
Helvetica 430, 98%, 4.4%; Bodoni Book (no metric-compatible face, Liberation
Serif) 370, 70%; Arial Narrow (Nimbus Sans Narrow) 262, 98%, 6.2%; Century
Gothic (URW Gothic) 159, 100%, 3.0%; Palatino (P052) 144, 97%, 6.5%. Most of
Book Antiqua's mismatches are in training manuals whose body styles indent 2
inches; with indents they agree.

What the comparison changed:

- **Font substitutes** (shaper; Fonts, above). Measured on this corpus
  before and after: Book Antiqua 69% → 80% of line counts equal and 7.1% →
  3.8% height difference; Century Schoolbook 4.9% → 1.1% height; Century
  Gothic 6.8% → 3.0%; AvantGarde 8.7% → 4.7%; Palatino 10.7% → 6.7%; all line
  counts 87% → 91%, page ends with spacing 58% → 61%. Arial Narrow keeps 98%
  of line counts with Nimbus Sans Narrow (6.2% height) and with Liberation
  Sans Narrow (11.6%), so Nimbus comes first. The ISO corpus also improved
  (Palatino) and the VM fixtures did not change.
- **The comparison itself**: styles that only the style sheet defines now
  take their `[spc]` from it (they were counted as "missing"); a style's
  single-line height is the largest height, at least its font size, that
  divides all its cached heights (the smallest cached height was two or more
  lines in styles whose paragraphs all wrap or hold hard returns, which made
  Ami Pro's caches look short); and the report breaks each paragraph a second
  time with the style's indents (`SAM_FORMAT_NOTES.md`, `[algn]`).

Mapping indents (Phase 3) is therefore worth the largest gain in line
breaks: 812 of 815 fixture paragraphs and 97% of this corpus agree with them,
against 95% and 91% without. Line counts do not tell the first-line indent
from the other lines' one; a VM fixture must.

**Indents mapped (2026-10-12).** VM fixtures I1–I3 settled the reading, and
the model, importer, and engine now carry indents (`DECISIONS.md`,
"Paragraph indents"). The report's line count now uses the imported style's
indents (the separate count with the stored indents is gone; a check that
both agree finds no differences). After the change:

| Measure | VM fixtures (102) | User's corpus | ISO corpus |
|---|---|---|---|
| Line counts equal | 817 of 820 (was 778) | 14,290 of 14,777, 97% (was 91%) | 21,045 of 24,142, 87% (was 21,003) |
| Page counts equal, with spacing | 99% | 77% (was 76%) | 22% |
| Page ends on the same page, with spacing | 29 of 65 (was 27) | 649 of 1,097, 59% (was 674) | 247 of 1,306 (was 245) |

Where page ends got worse, line counts are now right and our lines are
taller than Ami Pro's: a training manual has 501 of 510 equal line counts
but 36 pages against 34. Line heights (printer metrics, substitutes) and
`[spc]` spacing are the next gains. (Corrected later: that manual's extra
pages were stale frame heights; see "With para above frame heights", below.)

**Spacing mapped (2026-10-12).** VM fixtures S1–S6 settled `[spc]`, and
the import now carries paragraph and line spacing (`DECISIONS.md`,
"Paragraph and line spacing"). The report no longer has a "with Ami Pro's
paragraph spacing" variant: as imported is now that. After the change:

| Measure | VM fixtures (108) | User's corpus | ISO corpus |
|---|---|---|---|
| Page counts equal | 107 of 108 | 300 of 394, 76% (as imported before: 57%) | 35 of 160 (was 29) |
| Page ends on the same page | 29 of 65 (README31 13 of 13, MERCURY 7 of 7, AP31NAV 3 of 39) | 646 of 1,097, 59% (was 46%) | 247 of 1,306 (was 229) |
| Line counts equal | 817 of 820 | 97% | 87% |

AP31NAV still needs `[brk]` rules (keep with next and the like); the user's
corpus now mostly differs by line height (single spacing 3.0% mean, other
spacing 5.5%) and break rules.

**Break rules mapped (2026-10-12).** VM fixtures B1–B8 settled `[brk]`
(`DECISIONS.md`, "Break rules"). As imported:

| Measure | VM fixtures (116) | User's corpus | ISO corpus |
|---|---|---|---|
| Page counts equal | 114 of 116 | 299 of 394, 76%; 93% within one | 36 of 160 |
| Page ends on the same page | 75 of 75 (AP31NAV 39 of 39, README31 13 of 13, MERCURY 7 of 7) | 642 of 1,097, 59% | 283 of 1,306 |

On the user's corpus some training manuals lose page ends (25 pages
against 24, 48 against 45) while others gain (an exercise book 20 of 20).
This was first put down to taller lines; the next section shows it was
mostly frame heights.

**With para above frame heights (2026-10-12).** The 48-page manual first
diverged on page 10, where its paragraph's last line ran onto a page of its
own. Page by page, our text there was 858 twips lower than Ami Pro's from
the first picture down: we took the picture's height from field 16 (4,335)
while its stored rectangle, and Ami Pro's page in the VM, give 3,477. A
second picture there was 2,008 twips too short the other way. Its Bodoni
Book body lines matched Ami Pro's line for line (230 twips), and its Arial
headings are shorter than Ami Pro's (368 against 384), so line height was
not the cause. With the rectangle's height (`DECISIONS.md`):

| Measure | VM fixtures (116) | User's corpus | ISO corpus |
|---|---|---|---|
| Page counts equal | 114 of 116 | 300 of 394 (was 299) | 36 of 160 |
| Page ends on the same page | 75 of 75 | 662 of 1,097, 60% (was 642) | 283 of 1,306 |

The 48-page manual now has 45 pages like Ami Pro and 40 of 44 page ends
(was 10); the 36-page one 35 against 34 and 27 of 33 (was 4). One report
went from 42 to 48 pages against Ami Pro's 41: its photo pages set each
caption's line spacing with `<:S+158>`, which is not imported (Ami Pro's
caches give those lines 158 twips; ours are 288), and the smaller field 16
heights had hidden it. `<:S+n>` appears about 1,000 times in 50 of the
user's files and 29 ISO files; per-paragraph overrides are next.

**Rules from the decompile (2026-10-12).** The decompiled layout routines
(`SAM_FORMAT_NOTES.md`, "Layout rules from the decompile") changed four
things: paragraph alignment marks combine by OR; 1 1/2 and Double are
computed from each line's single height h (h + ⌊h/2⌋, 2h) instead of the
stored height; with Add in Always a paragraph's last line and its space
below must fit together; and a break after the last paragraph still starts
a page or column, which Ami Pro's page table records as an empty page.

| Measure | VM fixtures (121) | User's corpus |
|---|---|---|
| Page counts equal | 121 of 121 (was 119: B2 and B4 end in a page or column break after every paragraph) | 299 of 394 (unchanged) |
| Page ends on the same page | 75 of 75 | 679 of 1,097 (was 686) |

The loss is the space-below rule meeting our line heights, not a misreading
(the fit test, in Windows' upward twips, is "the line's bottom plus its
charged space below stays above the column bottom"). In the first file
that lost a page end, Arial 10 lines are 228 twips in Ami Pro and 230 in
ours: after 35 lines, Ami Pro's last line and its space below fit by 2
twips and ours miss by 71. Without the rule ours fit by luck. The other
six files were not examined one by one; every page-end change the rule
made was a loss, which fits our lines being taller than Ami Pro's in
those documents.

**Ami Pro's single line heights are the printer's.** The paragraph caches
of the user's corpus, single-spaced paragraphs only, grouped by the printer
each document names in `[prn]`: Times New Roman 12 pt is 273 twips on the
two fax drivers (225 paragraphs), 288 on a 600-dpi laser (151), 278 on a
generic PostScript driver (123), and 284 on most 300-dpi lasers and
PostScript printers (over 1,000), the same as the XPS driver in the VM
(LH1). Arial 10 pt is 228, 231, 237, or 240 by printer, Arial 12 pt 281,
284, or 288. Within one printer the value is the same in every document.
The decompile agrees: a line is its fonts' tallest ascent plus external
leading, plus the deepest descent, from the printer's font metrics. No
single font metric matches all documents, so line heights need a model of
the document's printer (Known deviations, 4; `DECISIONS.md`, 2026-10-12,
"Line heights follow the printer").

**Calibrated line heights (2026-10-12, user decision).** Each imported
document now takes its fonts' single line heights from its own one-line
paragraph caches (`DECISIONS.md`, "Calibrated line heights"):

| Measure | VM fixtures (121) | User's corpus | ISO corpus |
|---|---|---|---|
| Page counts equal | 121 of 121 | 298 of 394 (was 299) | 45 of 160 (was 38); within one 52% (was 41%) |
| Page ends on the same page | 75 of 75 | 689 of 1,097 (was 679) | 371 of 1,306 (was 287) |
| Single-spaced height difference | 0.2% (was 1.9%) | 0.7% (was 3.1%) | 0.4% (was 2.3%) |

The height differences are partly circular (the heights come from the same
caches); page ends are the independent check. On the user's corpus the net
gain is 10 page ends: 41 gained and 29 lost across 15 files. Most of both
come from two long manuals, where one early divergence shifts every later
page (+30 in one, −19 in the other). In the second, our text on page 10
stands about 90 twips lower than Ami Pro's around a table, so a heading no
longer fits before a chapter title that starts a new page; the heading's
own line height matches the cache. The ISO corpus gains most because 93% of
its paragraphs use fallback styles (missing style sheets), and the caches
recover their real line heights: that group's height difference fell from
21.1% to 10.6%.

**Character overrides (2026-10-12).** The importer now maps `<:f…>` font
changes and the `<+X>` attributes, styles carry their `[fnt]` colour and
attributes, and layout measures caps, scripts, and small caps
(`DECISIONS.md`, "Character attributes in the model"). Measured on the
user's corpus before and after each slice (VM fixtures unchanged at 121 of
121 page counts and 75 of 75 page ends):

| Measure | Before | Font changes and attributes imported | Caps and scripts laid out |
|---|---|---|---|
| Page counts equal | 298 of 394 | 299 | 298 |
| Line counts equal at Ami Pro's width | 14,387 of 14,867 | 14,396 | 14,396 |
| Page ends on the same page | 689 of 1,097 | 687 | 686 |

The net change hides two opposite effects:

- Font changes changed 22 files. Page ends moved in seven, and all but one
  gained (a 20-page manual from 11 to 18 of 19, three short documents from
  0 to all 3);
  together they gained 18. Paragraphs set in 10 pt inside a 12 pt style
  now wrap and stand as Ami Pro's caches say (240 twips a line, as cached,
  where they had wrapped at 12 pt widths).
- The one loss is a 41-page photo report, from 31 to 11. Its fifth page's
  text now has Ami Pro's cached heights line for line. Its page count
  became equal (41, was 42), but from that page on every page end is one
  page early. The earlier match was two errors cancelling out: the text was
  too tall, while the page's large frames and an inserted-layout page
  break left too little room. This is frame and break placement, not
  character formatting; it is left for the frame work.
- Laying out caps and scripts changed two files by one page end each. One
  paragraph's line sat at 9,264 of 9,360 twips; an upper-case word now
  pushes it to two lines. It has no cache to check, and the decompiled line
  builder applies the caps transform before measuring, so this is
  substitute-font width precision. In the other, empty paragraphs that
  hold only a font change are 276 twips high (the style's 12 pt) where Ami
  Pro gives 240 (the 10 pt font in force). Such a change applies to no
  text, so no run can hold it. It is kept as a legacy mark (75 such
  paragraphs in the user's corpus). Laying it out needs a paragraph-end
  font in the model, a follow-up.

The page surface draws what is measured. Our rendering of VM fixture O1 (every Text
menu attribute) beside Ami Pro's shows the same underline, word underline
(spaces left out), double underline, strikethrough, caps, small caps, and
the raised superscript and lowered subscript. Overstrike (`<:O…>`) is
still a legacy mark and is not drawn. O4's faces, sizes, and red text match
the file.

**Line heights for uncalibrated fonts and the end format (Phase 3.1,
2026-10-12).** The report now splits the single-spaced height difference
by the fonts a paragraph's characters are set in (`Paragraph::fonts`), and
measures each line as the engine does (its characters' fonts only, the
empty line's font when it has none; it had folded the style's font into
every line). Before the slice, on the user's corpus: 10,219 paragraphs in
one calibrated font differed by 0.4% on average, 1,196 in one font without
a height by 3.9%, and 225 that mix fonts by 0.7%. Most of the 1,196 were
left out of calibration only by a legacy mark that changes no font (839
`<:s>`) or a run that changes none. With the end format and the wider
calibration (`DECISIONS.md`, "The paragraph's end format"):

| Measure | Before | After |
|---|---|---|
| Page counts equal (user's corpus) | 298 of 394 | 298 (one file gained, one lost) |
| Line counts equal | 14,396 of 14,867 | 14,396 |
| Page ends on the same page | 686 of 1,097 | 686 |
| Single-spaced height difference | 11,640 paragraphs, 0.7% | 0.4% |
| One calibrated font | 10,219, 0.4% | 11,281, 0.4% |
| One font without a height | 1,196, 3.9% | 134, 7.4% |
| Mixed fonts, all calibrated / with others | 0 / 225, 0.7% | 40, 0.1% / 185, 0.6% |
| VM fixtures | 121 of 121 pages, 75 of 75 page ends | unchanged; single-spaced 0.0% → 0.3% |

Page ends did not move: the files with the most newly calibrated
paragraphs already had all their page ends (four of them with 3 to 9 page
ends each), or differ for frame reasons. The one file that gained a page
(1 against Ami Pro's 1, now 2) has every paragraph height equal to Ami
Pro's cache; one paragraph breaks into three lines where Ami Pro has two
(substitute widths), which a shorter uncalibrated heading had hidden. The
VM fixtures' 0.3% is AP31NAV's ruled style: calibration no longer counts
the rule's space as the font's, and rules are not laid out yet (next
slice). The 134 single-font paragraphs still without a height are mostly
multi-line paragraphs of fonts that never have a one-line paragraph.

**Empty paragraphs (2026-10-12).** Researching paragraph rules, a VM
fixture showed Ami Pro writing an empty paragraph as a bare `@Style@` line
with no blank line after it, which the importer joined to the next
paragraph. The decompiled body reader (`1118:0e55`) settled the rules
(`SAM_FORMAT_NOTES.md`, "Document body"): a bare tag line is an empty
paragraph, and so is a blank line where a paragraph starts, which the
importer had dropped. Fixing the paragraph splitter (measured at
`901fba7` and after):

| Measure | User's corpus before | After | ISO corpus before | After |
|---|---|---|---|---|
| Page ends on the same page | 686 of 1,097 | 742 of 1,108 | 409 of 1,306 | 409 of 1,307 |
| Page counts equal | 298 of 394 | 297 | 50 of 160 | 49 |
| Line counts equal | 14,396 of 14,867 | 14,398 of 14,869 | 21,300 of 24,969 | 21,301 of 24,970 |

The page ends gain because Ami Pro's page table numbers paragraphs as its
reader does, and every lost empty paragraph had shifted the later ones; 11
page ends that pointed past our last paragraph are now compared. Of the
page counts, two files now match and four one-page files run onto a second
page because of trailing empty paragraphs. Opening one of those in Ami Pro
showed that Ami Pro lays them out on a second page too and rewrites its page
table (2 pages, paragraph 35); the stored table was stale. VM fixtures
unchanged (124 of 124 page counts, 75 of 75 page ends, with the new
fixtures EN1, NX1OUT, and EP1OUT).

**Paragraph rules (Phase 3.2a, 2026-10-12).** Rules above and below
(`[line]`) are imported, laid out, and drawn (`DECISIONS.md`, "Paragraph
rules"). The report subtracts the rules' space from Ami Pro's caches before
finding a style's line height and line count, and adds it to ours.

| Measure | Before | After |
|---|---|---|
| VM fixtures, page counts / page ends | 124 of 124 / 75 of 75 | 126 of 126 / 76 of 76 (with NX2 and its rewrite) |
| VM single-spaced height difference | 0.3% | 0.1% |
| User's corpus page ends / page counts | 742 of 1,108 / 297 | unchanged |
| User's single-spaced height difference | 0.4% | 0.3% |
| ISO corpus page ends | 409 of 1,307 | 408 |

Two files changed, both by one: a manual already two pages longer than
Ami Pro's gains a third (its chapter titles have a 160-twip rule above,
which Ami Pro's caches include), and an ISO chapter loses one page end.
Both diverge earlier for other reasons; in the manual, nine paragraphs that
begin with hard returns break into one more line than Ami Pro's caches
imply (an open question, logged in `docs/PHASE3_PROGRESS.md`).

**Text Tightness (Phase 3.2, 2026-10-12).** With Tight and Loose laid
out (`DECISIONS.md`, "Next style and Text Tightness"), the user's corpus
gains 4 page ends (742 → 746 of 1,108; one file 7 → 11 of 12) and one line
count changes (14,395 → 14,394); VM and ISO results are unchanged. VM
fixture TT2 (Tight, Normal, and Loose in Times New Roman and Arial) breaks
all five paragraphs where Ami Pro does; the report cannot show it, since
with one multi-line paragraph per style it cannot tell a style's single
line height (it reads 1,420 twips as four lines of 355).

**Tab stops (Phase 3.2c, 2026-10-12).** Tabs now reach their stops
(`DECISIONS.md`, "Tab stops"); before, a tab was a glyph's width. The report
sizes tabs as the engine does, with the document layout's first column's
stops and the paragraph's ruler.

| Measure | Before | After |
|---|---|---|
| User's corpus page counts / page ends / line counts | 297 / 746 of 1,108 / 14,394 | 298 / 742 / 14,389 |
| ISO corpus page ends / line counts | 408 of 1,307 / 21,302 | 406 / 21,316 |
| VM fixtures | 126 / 76 of 76 | unchanged (128 with TT2OUT and TB2OUT) |

The first attempt lost 16 page ends; the losses showed two missing pieces,
now in: paragraphs' own rulers (`<:R…>`), which outrank the style's tabs,
and documents without a page layout, which take their style sheet's (their
tabs had gone to the line's end). What still differs is mostly paragraphs
whose style has a bullet or a number (Modify Style › Bullets & numbers,
`[spec]`), which is not modelled: in Ami Pro the bullet gives the tab after
it a place where our columns have no stop. 1,041 style records in 293 of
the user's files have one (slice 3.2e). TB2, drawn by our page surface,
matches Ami Pro's screen: stops, alignments, leaders, and the wrap after the
last stop.

**Bullets and numbers (Phase 3.2e, 2026-10-12).** Bullets and numbers are
laid out and drawn (`DECISIONS.md`, "Bullets and numbers";
`SAM_FORMAT_NOTES.md`, "Bullets and numbers"). The report moves a first
line's text as the engine does.

| Measure | Before | After |
|---|---|---|
| User's corpus page counts / page ends / line counts | 298 / 742 of 1,108 / 14,389 | unchanged |
| ISO corpus page ends / line counts | 406 of 1,307 / 21,316 | unchanged |
| VM fixtures | 128 / 76 of 76 | unchanged |

The guess above was wrong: a bullet stands at the left indent and the text
keeps its first-line indent unless the bullet is wider, which only 23 of the
4,264 compared bulleted paragraphs have, so bullets move no page end. The
first run lost a line in one file, which found an importer bug: `<*>>`
(lower-case letters) was not decoded, since its code character is the
closing bracket. The remaining differences after 3.2c are not bullets.
BN3, drawn by our page surface, matches Ami Pro's screen: numbers counting
their runs, dots, and an outline level's letters at the left indent with
the text at the first-line indent.

**Character leftovers and face kinds (Phase 3.3, 2026-10-12).** Protected
text, no hyphenation, and overstrike change no layout measure. Face kinds
(the pitch and family byte Ami Pro stores per face) were measured as a
substitute rule: used for every missing face whose name has no sans, mono,
or Courier word, they cost a page count and two lines, in two documents set
in script faces (Monotype Corsiva's heights went from 11% to 32% off with
Z003, and Shelley Allegro Script's byte says swiss). Used only where the
name tells nothing and is not a script face's, every measure is unchanged
(user's corpus 742 of 1,108 page ends, 14,389 line counts; VM fixtures 76
of 76; ISO corpus unchanged); CloisterBlack BT and Haettenschweiler, with no
compared paragraphs, now take the sans their bytes give.

**Hyphenation (Phase 3.6, 2026-10-13).** Hyphenated styles (`[algn]` 256)
split the word that does not fit, by Ami Pro's rules and TeX's US English
patterns (`DECISIONS.md`, "Hyphenation").

| Measure | Before | After |
|---|---|---|
| User's corpus page counts / page ends / line counts | 298 / 742 of 1,108 / 14,389 | 299 / 744 / 14,400 |
| ISO corpus page ends / line counts | 406 of 1,307 / 21,316 | 406 / 21,315 |
| VM fixtures | 131 / 76 of 76 | unchanged (132 with HY1OUT) |

Eight files gained 13 equal line counts and two lost one each, where TeX's
patterns and Ami Pro's dictionary split a word differently. HY1OUT's three
justified paragraphs show as differing only because the report reads their
5-line heights as four lines (its known limit with one multi-line
paragraph per style); the screen shows five, as ours.

### README31.SAM against Ami Pro's display (2026-10-12)

README31's Arial text breaks as Ami Pro's does (332 of 336 paragraphs with
the same line count); pages 1 to 4 end at the same paragraphs, and the rest
run one page late (15 pages against 14) through the spacing that Phase 3
maps (exact 240-twip lines; 72 twips below list items against our 120).
Bold looked missing from several headings in Ami Pro's Full Page view, but
at 100% they are bold, as imported. Three table defects were found and fixed
(`DECISIONS.md`, "Table geometry, joined cells, and Table Text"):

- **Joined cells** were laid out as single cells, so the merge-field table's
  three joined rows squeezed into one column and grew three to four times
  taller. Spans are now in the model, imported (all 27 corpus blocks are
  consistent), laid out, and kept in native v3.
- **Cell geometry**: the engine took the stored column width as the whole
  cell and added 86 twips to every row. The header's cell margins (86 and,
  for README31, 0) now do that, which gives TB1's, V20's, and README31's
  stored frame sizes exactly (140 of 220 corpus tables).
- **Table Text**: untagged cell text was Body Text (Times New Roman in
  MERCURY, Arial 10 for Arial 9 in README31); it is now Table Text, which the
  user's guide says Ami Pro assigns to all cells.
- A table whose text still needs more room grows to fit when it fits in the
  rest of its column, so text after it never runs under it.

After the fixes both documents' tables keep their stored sizes, sit where
Ami Pro shows them, and no longer cover the text after them. Number formats,
alignment within cells, shading, and colour are still Phase 3 and 5 work.

### MERCURY.SAM against Ami Pro's display (2026-10-12)

The user asked for the shipped demo `MERCURY.SAM` to be compared in the XP VM
with the page surface. Its styles are all in the file, so it is a fair test.
Pages 1 to 3 ended at the same paragraphs; the rest differed through the
Phase 3 gaps (indents, spacing, bullets, colour, hyphenation, footer fields
and alignment, table cell formats and joined cells, pictures) and two layout
defects:

- **Table position.** The importer read field 15 as the table's offset;
  MERCURY writes 20000 for a table Ami Pro shows centred at its stored
  rectangle, and corpus tables at the margin write values up to 19,456. The
  offset now comes from the rectangle (largest |dx| in the corpus: 30INTRO
  2,443 → 0, CH7 1,661 → 0, CH2 1,651 → 0; MERCURY 1,936 → 0).
- **With para above frames.** The engine reserved every such frame's height,
  so the text after MERCURY's icons started below them instead of beside
  them. MERCURY's stored rectangles give the rule exactly: starting each page
  at the top margin and adding the cached paragraph heights and `[spc]`
  spacing places all seven icons on page 7 and the drawing on page 4 at
  their stored tops, to the twip, when (1) a No wrap around frame does not
  push the text after it down, (2) a paragraph that holds only anchors takes
  no height, and (3) a frame anchored after text stands below the text,
  before the paragraph's space below. Tables keep taking their height; on
  Ami Pro's page 3 the heading after the table starts about 180 twips below
  it, which is the spacing alone.

After the fixes MERCURY's anchored frames are within 316 twips vertically
(was 9,987) and exactly in place horizontally; with Ami Pro's paragraph
spacing it has 8 pages like Ami Pro and all 7 page ends on the same page
(was 9 pages and 4). Until indents are imported, the text that runs beside
an icon overlaps it, because Ami Pro's paragraph indent is what moves the
text clear. F06 and P01's frames, which end their paragraph, moved from 216
to 96 twips off as imported.

### New documents in Ami Pro's faces (2026-10-13)

New documents' styles moved from Noto Serif to Times New Roman (Body
Text) and Arial (the headings), as Ami Pro's `_DEFAULT.STY` sets them
(user decision). The importer starts from these styles when a SAM names a
style sheet that cannot be found and stores no `[tag]` of its own, which
is most of the ISO corpus (143 of 160 files name `SMARTEXT.STY` or
`PRO21.STY`, absent from the ISO). Measured before and after:

| Set | Page counts | Line counts | Page ends |
|---|---|---|---|
| ISO corpus | 49 → **65** of 160 | 21,315 → **22,779** of 24,971 | 406 → **501** of 1,307 |
| User's corpus | 299 of 394 (unchanged) | 14,400 of 14,871 (unchanged) | 744 of 1,108 (unchanged) |
| VM fixtures | 136 of 136 (BN4 added) | 885 of 896 (unchanged) | 76 of 76 (unchanged) |

The user's corpus does not move because every style sheet it names
resolves. The ISO gain says Ami Pro, missing a sheet, sets text much as
Times New Roman 12 does (Liberation Serif here, metric-compatible).

## Known deviations

1. **Paragraph spacing is not imported** (Phase 3). The importer gives every
   style 6 pt after; Ami Pro's `[spc]` has space above and below (0 in
   `_DEFAULT.STY` Body Text, 72 and 72 in AP31NAV). This alone explains the
   page ends of V12, V13, and LH1OUT and most anchored-frame offsets.
2. **Exact line spacing is not imported** (Phase 3). `[spc]` mode 4 sets
   lines to a stored height (README31: 240 for Arial 10, where single spacing
   is 230). README31's anchored tables are 2,102 twips lower in Ami Pro after
   about 200 lines with spacing applied.
3. **Indents and break rules are not imported** (Phase 3). AP31NAV's Body
   Text has a right indent of 2,088 twips (VERIFIED with Ami Pro's ruler) and
   its titles carry `[brk]` flags; the narrower measure and the keep rules
   give Ami Pro 40 pages against our 31–33. The paragraph cache's width is the
   column (7,921) even there, so the plain line-count check does not see
   indents; the report's second count takes them off the width (all 312 of
   AP31NAV's paragraphs then agree).
4. **Printer hinting.** Ami Pro's Times New Roman lines are 4–9 twips taller
   at 8–24 pt (1.5–3%) and Courier New lines 7% shorter; bold and italic
   variants differ by up to 3%. Over a full page of 12 pt Times New Roman
   (about 45 lines) this moves a page break by up to one line. Since
   2026-10-12 this is known to depend on the printer each document names
   (above, "Ami Pro's single line heights are the printer's"). Since
   then each imported document's fonts take the heights its own caches
   give; fonts never cached alone in a one-line paragraph (bold runs in
   body text, for one) still use the font's metrics.
5. **Character widths** agree to about 0.3% (8 twips over 23 characters
   before a Flow with text frame in F07).
6. **Stale caches.** Picture and table frames store column edges that do not
   match their margins (README31's pictures 600 twips off; TB1's table 144
   inside a frame with 0 margins); anchored frames store a stale page; table
   frames store a stale field 16, and a field 15 that is not an offset.
7. **Tables across pages** (since 2026-10-18, the tables across pages
   run): tables continue on later columns and pages as Ami Pro continues
   them (`TABLE_PAGES_NOTES.md`). What still differs is upstream of the
   split: cells wrapping into one more line than Ami Pro's, multi-line
   cells in faces without a calibrated printer height (5–7 twips a line
   taller), text before a table running longer, and a few stale stored
   rows (below, "Tables across pages").

## Tables across pages (2026-10-18/19)

Ami Pro's page table names, for every page that ends in a table, the first
row of the table's part on that page and how many rows it holds
(`TABLE_PAGES_NOTES.md`); its stored row heights (`[h]`) are each row's
height as Ami Pro measured it. The report compares both (`tables:` in each
summary; `AMIPRO_COMPARE_DETAIL_TABLES=*` prints every differing row and
page end).

| Folder | Page counts | Page ends | Page-end tables with Ami Pro's rows | Rows at Ami Pro's height |
|---|---|---|---|---|
| VM folder | 247 → 256 of 256 | 99 of 113 → 105 of 114 | 41 of 42 | 534 of 535 |
| User's corpus | 302 → 312 of 394 | 760 → 777 of 1,108 | 69 of 106 | 2,597 of 2,777 |
| ISO corpus | 65 → 68 of 160 | 492 → 474 of 1,307 | 18 of 106 | 666 of 2,056 |

- Every VM fixture of the run (TP1–TP11) pages as Ami Pro pages it; the
  one page-end table that differs (BPS, Ami Pro's re-save of an ISO file)
  has a cell that wraps into one more line here.
- The user's corpus gains 17 page ends overall. Its 37 page-end tables that
  differ trace to the rows before them: a cell wrapping into an extra line,
  multi-line cells in Times New Roman 11 pt or Book Antiqua 12 pt (faces no
  one-line main-text paragraph calibrates; the substitutes' lines are 5–7
  twips taller), or text before the table running longer.
- The ISO corpus loses 18 page ends, all in files whose style sheets
  (smartext.sty, pro21.sty) are not on the ISO: their cells' styles are
  fallbacks, so rows measure 20, 40, or 120 twips taller per row, where the
  table used to keep Ami Pro's stored height.
- An empty cell needs no height: Ami Pro keeps a row of empty cells at its
  stored height, even 0 (15 such rows in the user's corpus).

**Text on both sides of a Wrap around frame** (merged 2026-10-19, user
decision; `TABLE_PAGES_PROGRESS.md`, fill-in): the text fills the part of
the column left of the frame down its band, then the part right of it,
then goes on below (VM WB1–WB3). Measured (`target/tp-wb/`): the VM folder
unchanged (268 of 268 page counts with the WB fixtures); the user's corpus
page counts 312 → 315 of 394, page ends 777 → 762 of 1,108 (one file
5 → 10 of 10; one long manual already 16 pages longer than Ami Pro's
34 → 14 of 51); the ISO corpus unchanged (68 of 160, 474 of 1,307); round
trip unchanged.

**The row cache** (2026-10-20, user decision; `TABLE_PAGES_PROGRESS.md`,
"The row cache"; VM RC1–RC3): an Automatic row as read keeps Ami Pro's
stored height (its own measure) while its text measures here at most half
its gutter taller. Measured (`target/rc-baseline/` → `target/rc-after/`):
the user's corpus rows at Ami Pro's height 2,597 → 2,639 of 2,777 (42 by
the cache; the summary line now says how many), page counts 315 and page
ends 762 of 1,108 unchanged; the ISO corpus rows 666 → 1,225 of 2,056 (559
by the cache), page counts 68 → 69, page ends 474 → 478; the VM folder
unchanged; round trip unchanged. Of the user's 140 rows still off, most
hold a line more here (line breaks, not heights) or a stale height.

## Changes made from the comparison

- **Line height** (shaper): ascent plus descent plus the font's line gap,
  from LH1 (`DECISIONS.md`, "Single line spacing").
- **Table frame size** (importer): the stored rectangle, not field 16, which
  Ami Pro leaves stale for tables (`0 65535` in TB1). TB1 now has one page,
  like Ami Pro, and README31 has 15 pages instead of 17. The corpus audit's
  fixed-shaper layout went from 1,585 to 1,465 pages.
- **Flow with text frames** (engine): the right shadow takes space after the
  frame (Z05: two frames with the Normal shadow stand 100 twips apart; was
  92 twips off).
- **Table frame position** (importer): the stored rectangle's left edge,
  from the left of its column, not field 15 (MERCURY.SAM, above).
- **With para above frames** (engine): No wrap around frames other than
  tables let the text after them run beside them; a paragraph holding only
  anchors takes no height; a frame after text stands before the space after
  (MERCURY.SAM, above; `DECISIONS.md`, 2026-10-12).
- **Tables** (model, importer, engine): joined cells, cell margins, Table
  Text for cell text, and tables that grow to fit their text within a column
  (README31.SAM, above).
- **Font substitutes** (shaper): installed metric-compatible faces for
  uninstalled ones, and the kind read from whole words of the name (the
  user's corpus, above; `DECISIONS.md`, "Metric-compatible font
  substitutes").
- **Calibrated line heights** (model, importer, shaper): each document's
  printer line heights from its one-line paragraph caches (above).
- **Character overrides** (model, importer, layout, page surface): `<:f…>`
  faces, sizes, and colours, the `<+X>` attributes, the styles' colour and
  attributes, caps, and script and small-caps sizes (above).
- **Tab stops** (model, importer, engine, page surface): tabs reach their
  stops with leaders; rulers; the style sheet's layout for documents
  without one (above).
- **Text Tightness** (shaper, page surface): each character gains its
  share of the average width (above).
- **Hyphenation** (model, importer, layout, shaper): words split at line
  ends in hyphenated styles by Ami Pro's rules, with TeX's patterns (below).
- **Face kinds** (importer, shaper): a missing face whose name gives no
  kind takes the substitute of its stored pitch and family (below).
- **Bullets and numbers** (model, importer, engine, page surface): `[spec]`
  bullets, numbers counted as Ami Pro counts them, placed at the left
  indent; empty style fields keep their places (above).
- **Paragraph rules** (model, importer, engine, page surface): `[line]`
  rules above and below, their space and drawing (above).
- **Empty paragraphs** (importer): bare style-tag lines and blank lines
  where a paragraph starts are empty paragraphs, as Ami Pro's reader has
  them (above).
- **End format and wider calibration** (model, importer, engine): an empty
  paragraph's line in the font in force at its end; fonts calibrated from
  paragraphs with marks or runs that change no font and from mixed lines
  (above).
- **Rules from the decompile** (importer, engine): alignment marks by OR,
  1 1/2 and Double from the line, the last line's space below, column
  breaks after their paragraph, and a final break's empty page (above;
  `DECISIONS.md`, "Layout rules from the decompile").

## How to run

```sh
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT AMIPRO_CORPUS=/tmp/amipro-phase1-corpus \
  cargo test --offline --release --manifest-path app/cosmic/Cargo.toml \
  layout_comparison_report -- --ignored --nocapture
# The user's corpus, with its style sheets:
AMIPRO_CORPUS=$PWD/../AmiPro-Reference/Charlie-Sample \
  AMIPRO_STYLES=$PWD/../AmiPro-Reference/Charlie-Sample/STYLES cargo test … layout_comparison_report -- --ignored --nocapture
# One file in detail (oracle values next to our pages, frames, and lines):
AMIPRO_COMPARE_DETAIL=V20.SAM AMIPRO_VM_LAB=… cargo test … layout_comparison_report -- --ignored --nocapture
# Its tables only, or every file's differing table rows and page ends:
AMIPRO_COMPARE_DETAIL_TABLES=1 AMIPRO_COMPARE_DETAIL=V20.SAM …
AMIPRO_COMPARE_DETAIL_TABLES='*' AMIPRO_CORPUS=… cargo test … layout_comparison_report -- --ignored --nocapture
```
