# Phase 3 Progress (live)

The live state for the rest of Phase 3 (`docs/PHASE3_RUNBOOK.md`). Update it
after every slice and before any long step, so that a resumed or compacted
session can continue.

Status values: `TODO`, `IN PROGRESS`, `DONE`, `BLOCKED` (with the reason),
and `WAITING FOR USER`.

**Run start tag:** `phase3-run-start` (created by the first session of the
run).

**Baseline before the run** (2026-10-12, `217801f`): user's corpus 298 of 394
page counts, 14,396 of 14,867 line counts, 686 of 1,097 page ends; VM
fixtures 121 of 121 page counts and 75 of 75 page ends; core 164 + 1
ignored; app 143 (normal build), 150 (all features), 93 (text editor).

## Slices

| Slice | Status | Commits | Evidence | Notes |
|---|---|---|---|---|
| 3.1 Line heights for uncached fonts; paragraph-end font | DONE | `901fba7` | Tests; native round trip; ISO corpus gate PASS; user's corpus 416 round trips; comparison in `LAYOUT_COMPARISON.md` | Page ends unchanged (686); single-spaced heights 0.7% → 0.4%; found paragraph rules (3.2a) |
| 3.1b Empty paragraphs (importer fix found in 3.2a research) | DONE | `870c359` | Decompile `1118:0e55`; VM EP1, UPPX; tests; corpus gates | User's corpus page ends 686 → 742 of 1,108 |
| 3.2a Paragraph rules (`[line]`): model, native, layout, drawing | DONE | `a6df120` | VM NX1, NX2; tests; comparison; our NX1 rendering beside Ami Pro's | VM 76/76 page ends; user's corpus unchanged (742) |
| 3.2b Next style and Text Tightness | DONE | `f81ce7a` | VM NX1, TT1, TT2; tests; comparison; TT2 rendering beside Ami Pro's | User's corpus page ends 742 → 746 |
| 3.2c Tab stops, leaders, rulers, the style's own tabs, the hanging-indent stop | DONE | `9b22df0` | Decompile; VM TB2; tests; comparison; TB2 rendering beside Ami Pro's | User's corpus page ends 746 → 742 (bullets, 3.2e) |
| 3.2d Frame interiors' space-below rule | DONE | `f7b5a6d` | VM FS4, FS5; test | Same rule as pages; measures unchanged; a ~30-twip frame allowance unexplained |
| 3.2e Bullets & numbers (`[spec]`) | DONE | `d7deb91` | Decompile; VM BN1–BN3; tests; comparison; BN3 rendering beside Ami Pro's | Measures unchanged (bullets move 23 of 4,264 first lines); found `<*>>` and empty-field importer bugs |
| 3.3 Character leftovers: overstrike, protected text, no hyphenation, pitch-and-family byte | DONE | `47fcf12` | Decompile; VM O1, PT1 (PT1OUT); tests; comparison; overstrike beside Ami Pro's | Measures unchanged; Mark Text menu commands not added |
| 3.4 Style commands, Use Another Style Sheet, style tests | DONE | `842e5d3` | Guide ch. 9; VM ST1, ST1A, ST1B; tests (commands, save and reopen, kept values, refused sheets); gates | Menus wait for D1; sheet header/footer frames not applied |
| 3.5 Decision stop: D1 style editing design, D2 hyphenation dictionary, D3 STY writer | DONE | `70251a3` | D1: `UI_SPEC.md` §14 with VM screenshots; D2: `docs/research/HYPHENATION_OPTIONS.md`; questions in `SESSION_HANDOFF.md` | Asked together, ranked, with recommendations |
| 3.6 UI-3 and style editing; hyphenation; STY writer | DONE (physical check PASS, user, 2026-10-13) | `dcf444c`, `44f8e7f`, `038a778`, `72ef5ee`, `7921530`, `401b221`, `d92f189` | Hyphenation: decompile, VM HY1, comparison; STY: decompile `11c8:14e0`, VM SS1N/SS2N/SS2W/SD1/SD2, AP2–AP5 opened in Ami Pro, corpus read-back 255/255; dialogs: tests, screenshots light and dark, VM BN4 | D1 approved (§14), D2 and D3 yes (2026-10-13). With contents (the SAM writer) left for Phase 4 |

## Log

(Newest last: what was done, measured, decided, or found.)

- 2026-10-12, session 1. Tag `phase3-run-start` created at `e3c7b88`.
  Baseline confirmed (user's corpus 298/394, 14,396/14,867, 686/1,097; VM
  121/121, 75/75).
- 3.1: the decompile settled both questions. An empty paragraph's line is
  the font in force after its controls (`10d0:0592`); lines with text count
  only their characters (`10d0:359b`). A mixed line is the tallest ascent
  plus leading over its fonts plus the deepest descent. Model:
  `Paragraph::end_format` (native `endformat`). The report's new split by
  fonts showed the real gap was 1,196 single-font paragraphs left
  uncalibrated by `<:s>` (a paragraph flag) and runs that change no font;
  calibration now samples them, plus mixed one-line paragraphs. Found that
  `[line]` paragraph rules add their space to the first and last lines
  (`10d0:0d82`); calibration subtracts it, and laying rules out is the new
  slice 3.2a. Page ends unchanged at 686 (explained in
  `LAYOUT_COMPARISON.md`); heights 0.7% → 0.4%.
- 3.1 editing: VM fixture EN1 (typed in Ami Pro) showed Enter carries the
  format at the caret into the new paragraph; the end format follows it.
  Page-surface typing A/B (README31, release, 4 interleaved rounds against
  `e3c7b88`): start/middle/end medians 0.59/0.72/0.71 ms base, 0.58/0.70/0.65
  ms after; no regression. Tests: core 169 + 1 ignored; app 144, 93, 151
  (9, 4, 9 ignored).
- 3.2a research (VM NX1, NX1OUT): `[line]` decoded with the Modify Style ›
  Lines dialog: flags 1 above, 2 below, 4 Length Text, 8 Margins (neither:
  Other, value 2 in twips, value 10 its unit); values 6 and 7 the line
  styles (1–7 single rules of 10–240 twips, 8–12 double and triple); values
  8 and 9 the space each rule takes, its thickness included (the dialog's
  Spacing is the gap); the rule above at the top of that space, the rule
  below at the bottom; colour value 4. Caches confirm the added height. Next
  style: Enter at the end gives the next style; in the middle or at the
  start both parts keep the style.
- 3.1b: while there, NX1OUT showed Ami Pro writing an empty paragraph as a
  bare `@Style@` line; the decompiled reader also makes a blank line at a
  paragraph's start an empty paragraph. The splitter was wrong on both;
  fixed (EP1). User's corpus page ends 686 → 742.
- 3.2a done: paragraph rules modelled, stored (`sline`), laid out, and
  drawn. NX1OUT line counts 4 → 7 of 7; NX2OUT heights all equal.
- Open (not in a slice yet): paragraphs that begin with hard returns
  (`<**>`) break into one more line than Ami Pro's caches imply in one
  manual (9 paragraphs; a style with a 2-inch indent). To examine with the
  decompiled line builder and a VM fixture.
- 3.2a typing A/B (4 rounds against `870c359`, load average 5): layout
  median at the middle 0.17 → 0.19 ms with one round reversed; no clear
  regression.
- 3.2b: Next style per NX1. Text Tightness: the user's guide calls it the
  width of a space, but TT1 measured every character changing by the
  same amount: (percent − 100)% of the average width. Laid out with COSMIC
  Text letter spacing; drawn character by character. TT2's breaks match.
- Found: the engine has no tab stops (tabs are glyph widths); the style's
  own tabs and the hanging-indent stop need them, so tab stops are 3.2c.
- 3.2c: tab layout from the decompile (stop list `10d0:1881`, next stop
  `14f0:08fa`, segments `10d0:49b8`, alignment `10d0:3ef4`) and VM fixture
  TB2 (leaders: 1 hyphens, 2 dots, 3 a line; no stop left: to the line's
  end; decimal without a point: right). Found on the way: paragraph tab
  rulers (`<:R…>`) and documents without `[lay]`, which take the style
  sheet's layout; both added. Bullets & numbers (`[spec]`) is the next gap.
- 3.2d: frame interiors follow the page's space-below rule (FS4, FS5 swept
  interior heights). Lines in frames need about 30 twips more than their
  height; not explained or modelled.
- 3.2e: `[spec]` decoded with the decompile and BN1–BN3 (`SAM_FORMAT_NOTES.md`,
  "Bullets and numbers"): codes, numbering by runs and outline levels, the
  bullet at the left indent and the text at the first-line indent unless
  the bullet is wider. `<:s>` does not affect bullets (BN3). The parser now
  keeps whitespace-only lines as empty values (155 `[nfmt]` records had
  shifted values), and `<*>>` decodes. Measures unchanged everywhere: only
  23 of 4,264 compared bulleted paragraphs have a bullet wider than their
  first-line indent, so the 3.2c guess that bullets explained the remaining
  tab differences was wrong. Typing A/B (README31, 4 rounds against
  `f7b5a6d`): start equal, middle +0.09 ms, end −0.08 ms; no clear
  regression.
- 3.3: PT1 (typed and saved in the VM) gave protected text's editing rules:
  arrows step over it, Shift selects it whole, clicks land after it, Delete
  and Backspace remove it, typing at its edges stays outside (except right
  after deleting into it, a recorded difference). Overstrike: the decompile
  shows one character in force, replaced by a new one and cleared by any
  `<:O-…>`; drawn over each character. Face kinds: kept per face; as a
  substitute rule only where the name tells nothing and the face is not a
  script (script bytes and Z003 made two documents worse). Measures
  unchanged (user's corpus 742/1,108; VM 76/76 with PT1OUT added, 129 page
  counts).
- 3.3 typing A/B (3 interleaved rounds against `d7deb91`, README31): start
  equal; middle +0.02 ms and end +0.04 ms, all in the update phase; layout,
  view, and draw equal. Not isolated (face-kind registration is a few hash
  lookups); totals 0.51–0.68 ms.
- 3.4: ST1 (made for the purpose, opened on `_DEFAULT.STY`) settled the
  rules: a SAM's `[tag]`s are its document styles; Remove leaves the tag,
  formatted as Body Text; Use Another Style Sheet keeps document styles,
  makes sheet styles the new sheet lacks format as Body Text with their
  names, and changes `[sty]`. Model: style origins and the sheet link;
  commands and `read_style_sheet`. Measures unchanged (VM 131/131 page
  counts with ST1A and ST1B, 76/76 page ends; user's corpus 742/1,108).
- 3.5: the D1 proposal (`UI_SPEC.md` §14, after Ami Pro's Style menu,
  Create Style, Modify Style, Style Management, and Use Another Style Sheet
  dialogs, screenshots outside Git) and the D2 note (hyphenation options;
  recommended: the `hypher` crate in the app behind a layout trait). Asked
  with D3 in `SESSION_HANDOFF.md`. Session ended here (2026-10-13).
- 2026-10-13: the user approved `UI_SPEC.md` §14 (D1) and said yes to D2
  and D3 (`DECISIONS.md`). 3.6 starts. Asked whether 3.3's typing
  slowdown could be other processes: re-measured on a quieter machine
  (below).
- Typing A/B again on a quieter machine (VM saved, load 1.6 at the start;
  4 interleaved rounds, `d7deb91` against `70251a3`): start equal (medians
  0.60/0.60 ms), middle faster after (0.82 → 0.75), end equal (0.69/0.69);
  only the end's update phase is about 0.02 ms higher in 3 of 4 rounds. The
  same build varies by up to 0.5 ms between rounds at one position, so the
  earlier +0.02–0.04 ms (measured with Ami Pro running in the VM) is within
  noise: no regression found.
- 3.6 hyphenation (D2): Ami Pro's rules from the decompile (the zone in
  average characters from `[docopts]`, 32 characters, five letters, no
  digits, discretionary hyphens first, No hyphenation), `hypher` with TeX's
  US English patterns in the app behind `Shaper::hyphenate`. User's corpus
  line counts 14,389 → 14,400, page ends 742 → 744; VM fixtures 132/132
  with HY1OUT (Ami Pro's line counts for its eight paragraphs), 76/76 page
  ends. Tests: core 203 + 1 ignored; app 152, 95, 159.
- 3.6 STY writer part 1 (D3): style records written as Ami Pro's writer
  does (`11c8:26b7`); every value kept. Corpus audit: ISO 542/542 records
  byte for byte, user's 1,396/1,912 (the rest are older short records).
  Importer fixes on the way (bullet trailing spaces, bracketed bullet text,
  unknown next-style names); measures unchanged. Part 2 (the page layout
  and frame writer, the whole sheet, the dialog, the VM check) is next.

- 3.6c, the whole sheet (D3 part 2): Ami Pro's sheet writer is
  `11c8:14e0` (the SAM writer `11c8:1c7b`); VM fixtures SS1N (new document
  on `_DEFAULT.STY`, no contents), SS2N/SS2W (header text, a created
  style, without and with contents), SS2/SD1OUT/SD2OUT (the documents
  saved after). Findings (`SAM_FORMAT_NOTES.md`, "Writing style sheets"):
  3.1 writes `[fldnames]` and a 21-value `[desc]`; without contents the
  text and pictures of layout frames with 0x8 are left out; styles are in
  function-key order (640/640 files); a saved document is linked to the
  new sheet, an untitled one detached. Written: `style_sheet`,
  `layout_record`, and a body writer (`11b0:1405`: line breaks at 65/256
  bytes, the paragraph controls' order). The importer keeps the LAYOUT and
  frame values the model lacks, decodes style names (AP4), and keeps
  0x80 out of the paper code. Measures: a sheet from `_DEFAULT.STY` equals
  SS1N but for the printer and `[desc]` bookkeeping; corpus sheets read
  back 63/63 and 192/192; layouts byte for byte 178/184 and 442/588
  (rest explained in `DECISIONS.md`); layout frame text 384/384 and
  786/786. Layout comparison unchanged (user's corpus 299/394, 14,400/
  14,871, 744/1,108; VM 135/135 page counts with the three new SAMs,
  76/76). Ami Pro opened AP2, AP3, AP4 (File › New) and applied AP2 (Use
  Another Style Sheet). Tests: core 213 + 1 ignored; app unchanged (152,
  95, 159). Not yet: With contents (main text, page frames, tables,
  pictures), pictures in layout frames.
- 3.6d, Save as a Style Sheet in the app: Format › Styles ▸ (Ami Pro's
  Style menu order; unbuilt items disabled), the dialog (description, then
  the portal's file chooser), a writer that creates new files only
  (`write_new_file`: the existence check and the creation are one step),
  and the document linked to the new sheet or detached when untitled
  (`Command::SavedAsStyleSheet`, one undo step). Found on the way: style
  commands reported no change, so the history neither applied nor
  recorded them (no UI used them yet); `ChangeReport::styles` fixes it.
  Screenshots light and dark. Tests: core 215 + 1 ignored; app 156, 99,
  163 (9, 4, 9 ignored).
- 3.6e, Create Style, Style Management, and Use Another Style Sheet
  (`UI_SPEC.md` §14), with the style-sheet folders as a preference (SAM
  import now searches them; Browse… adds a folder). Commands exist since
  3.4; the dialogs apply each change as one undo step. >> Move >> is not
  offered (it would rewrite the sheet; D3: new files only). Found: a new
  document's styles are stored in the document, so Use Another Style
  Sheet keeps them (Ami Pro's rule for document styles, ST1B); documents
  made from a sheet change as Ami Pro's do. Screenshots light and dark.
  Tests: app 159, 102, 166 (9, 4, 9 ignored); core 215 + 1 ignored.
- 3.6f, Modify Style (`UI_SPEC.md` §14): one dialog with the style list,
  the six panels and Hyphenation, and a sample laid out and drawn by the
  page engine and surface in the edited styles; drafts per style applied
  on OK as one undo step; Save As; Create Style's Modify…. VM fixture BN4
  (made for it) gave the bullet text's order: number, Text, bullet. Ami
  Pro's Alignment, Spacing, and Breaks panels captured for the layout
  (screenshots outside Git). Tests: app 165, 102, 172 (9, 4, 9 ignored).
- 3.6g, the Styles panel as the management owner (UI-3, `UI_SPEC.md`
  §14): + for Create Style…, Ami Pro's bullet before document styles, names
  defined nowhere in the error colour with a tooltip, and a context menu
  per row (Apply, Modify Style…, Create Style Based on This…, Rename…,
  Remove). Tests: app 166, 103, 173 (9, 4, 9 ignored).
- Phase 3 exit (2026-10-13): `PROJECT_PLAN.md` Phase 3 items checked,
  `FEATURE_PARITY.md` ST-01, ST-02, IO-02 updated, final measures above
  (unchanged since hyphenation: user's corpus 299/394, 14,400/14,871,
  744/1,108; ISO 49/160, 21,315/24,971, 406/1,307; VM 135/135, 76/76).
  Still open (not in a slice): the findings listed after 3.6's first
  slices (hard-return paragraphs in one manual, frame text's ~30-twip
  allowance, the report's line-height inference for justified paragraphs,
  typing right after deleting into protected text, the sheet's header and
  footer frames on Use Another Style Sheet, Ami Pro's own hyphenation
  dictionary), With contents, pictures in layout frames when writing a
  sheet, and new documents' Noto default faces, which Ami Pro lacks.
- 2026-10-13, after the exit: new documents use Ami Pro's faces (user
  decision): Times New Roman and Arial with Ami Pro's face-kind bits. ISO
  corpus page ends 406 → 501 of 1,307 (its sheets are mostly missing, so
  the importer starts from these styles); user's corpus and VM fixtures
  unchanged.
