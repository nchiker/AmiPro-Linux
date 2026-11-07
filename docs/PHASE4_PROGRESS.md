# Phase 4 Progress (live)

The live state for Phase 4, the SAM writer (`docs/PHASE4_RUNBOOK.md`).
Update it after every slice and before any long step, so that a resumed or
compacted session can continue.

Status values: `TODO`, `IN PROGRESS`, `DONE`, `BLOCKED` (with the reason),
and `WAITING FOR USER` (only for a question that came up during the run;
the run's own decisions A–C were taken before it started).

**Run start tag:** `phase4-run-start` (created 2026-10-13 at `887ab86`).

**Baseline before the run** (2026-10-13, `36acf89`; layout unchanged since
`ef81012`): user's corpus 299 of 394 page counts, 14,400 of 14,871 line
counts, 744 of 1,108 page ends; ISO corpus 65 of 160, 22,779 of 24,971,
501 of 1,307; VM fixtures 135 of 135 page counts and 76 of 76 page ends;
ISO corpus audit gate PASS (160/160); core 215 + 1 ignored; app 172
(normal build), 103 (text editor), 179 (all features), with 9, 4, 9
ignored. Round-trip audit: not yet written (slice 4.2).

## Slices

| Slice | Status | Commits | Evidence | Notes |
|---|---|---|---|---|
| 4.1 The writer's map (decompile `11c8:1c7b`; the cache question in the VM) | DONE | `d4c0cf4` | `SAM_FORMAT_NOTES.md`, "Writing documents"; VM fixtures K1–K6 | Omit `[pg]`; write a paragraph's `<:#…>` only while it is as read; never compute caches. Importer gaps for 4.5: `[l1]` ignored; a sheet's Standard dropped when a SAM stores only inserted layouts |
| 4.2 Loss inventory, the round-trip audit, the loss check's core | DONE | `b5d37ac` | `FORMAT_COMPATIBILITY.md`, "Phase 4 inventory"; `examples/sam_round_trip.rs`; `roundtrip.rs` | Baseline 0 of 416 and 0 of 160 (the writer is a stub until 4.3) |
| 4.3 The document head and plain text | DONE | `30db71f` | Audit below; VM: T01–T05, ST1, EP1 written and opened, page area pixel-identical to the originals | Also writes the document's own layouts (Ami Pro hangs on a file with neither a sheet nor `[lay]`); byte 0xAD fixed (U+2010) |
| 4.4 The main text in full | DONE | `b06e959` | Main text reads back equal in every document without page frames (ISO 79 of 79, user's 128 of 128); VM: 24 fixtures (breaks, fields, floating headers and footers, Text menu formatting, protected text, bullets, Next style, hyphenation, tightness) opened at their start and end: 22 pixel-identical | G21: inserted layout lost (fixed in 4.5: `[elay]` per layout); O6: a run's underline drawn in one piece instead of two (explained) |
| 4.5 Page layouts | DONE | `fd8a64d` | VM: G00–G23 written and opened, all 48 screenshots (start and end) pixel-identical; audit below | `[l1]` typed (start layout, model and native v3); a sheet's Standard kept and written only once changed; inserted layouts renamed; `[elay]` per layout; the `[Embedded]` directory always written |
| 4.6 Frames | DONE | `46541c0`, 4.9's commit (fixes) | VM: 57 frame fixtures written, opened, and saved again (screens 56 of 57 identical, frame records 55 of 57); audit below | One module, `frames.rs`, for 4.6–4.8. A table's `[frmlay]` value 1 kept as read (4.9, VM) |
| 4.7 Tables | DONE | `46541c0`, 4.9's commit (fixes) | VM: TB1, TB2OUT, V20, V21 identical but for the caret; README31's and MERCURY's tables open (4.9) | Cell text written exactly when the record's eighth value says so (`1490:07c0`; Ami Pro refused README31 written otherwise) |
| 4.8 Pictures and embedded objects | DONE | `46541c0` | VM: P01–P03 identical; MERCURY's logo and drawings open (4.9) | Bytes and directory as Ami Pro writes them (`11c8:4155`, `11c8:4521`) |
| 4.9 Caches, as 4.1 decided | DONE | `b3081bb` | Audit: 416 of 416 user's read back equal *including* line heights; VM: K1, K2, K4, K5 written and opened, start and end pixel-identical; Ami Pro's re-saves have the same page count and page starts as its re-saves of the originals | Also three rules found in the VM (`SAM_FORMAT_NOTES.md`): cell text flag, a table's `[frmlay]` value 1, a note's closing line |
| 4.10 Save As SAM in the app (decisions A, B) | DONE | `1bcdece`, `575d9e1` | Test `save_as_sam_writes_new_files_only_and_lists_what_ami_pro_cannot_store`; screenshots `window-savesam-*`; `UI_SPEC.md` §17; VM: APP1 (new, typed) and APP2 (the page fixture, after Save Anyway) saved through the app's path open in Ami Pro, which saves their bodies, frames, and layouts back unchanged | The frame lines bit (0x10000) found here and fixed both ways |
| 4.11 With contents; edited documents | DONE | `bd53c98` | Sheets from SS2 equal Ami Pro's SS2W/SS2N, from F01 its F01W (but `[desc]`); VM: a new document from a sheet with contents starts with its text, header, and frame; edited README31 and MERCURY open in Ami Pro, which reads them as it reads the originals (MERCURY exactly; README31 but for frames in table cells, a model gap) | Tests `sam_after_edits.rs`; also Table Text for new tables, kept values the writer derives dropped at import, the sheet's 0x8/0x80 marks (correcting 3.6) |
| 4.12 Phase exit; the page layout setup proposal (decision C) | DONE | dd0b3a0 | `FORMAT_COMPATIBILITY.md` (inventory, final numbers), `FEATURE_PARITY.md` IO-01, IO-02, IO-04; `UI_SPEC.md` §18 from Ami Pro's Page menu and Modify Page Layout panels (VM screenshots); layout comparison unchanged | Waiting for the user: §18's four questions; the physical check of Save As SAM |

## Round-trip audit

(From 4.2: one row per slice, aggregate counts only.)

Run with `AMIPRO_STYLES=…/Charlie-Sample/STYLES cargo run --release -p
amipro-formats --example sam_round_trip -- DIR [--detail FILE]` (detail
files go to the scratchpad, never into Git). "Equal" is the model read
back from the written file against the model read from the original,
without IDs; sections are the written file's top-level sections against
the original's, `[pg]` and `[desc]`'s save time and count left out.

| After | User's corpus: documents read back equal | ISO corpus: documents read back equal | Sections equal to the original (user's; ISO) | Notes |
|---|---|---|---|---|
| 4.2 (writer stub) | 0 of 416 | 0 of 160 | 416 of 52,749; 160 of 8,443 (`[ver]` only) | Baseline |
| 4.3 | 51 of 416 (128 but for line heights; text and styles only: 9 of 9) | 5 of 160 (78 but for line heights; text and styles only: 14 of 14) | 40,021 of 52,749; 5,660 of 8,443 | Line heights return with the caches (4.9); the rest is anchors and frames (4.6), inserted layouts (4.5), tables (4.7), pictures (4.8) |
| 4.5 | 51 of 416 (128 but for line heights; main text 273, and 128 of 128 without page frames) | 5 of 160 (78; main text 88, and 79 of 79) | 41,118 of 52,749; 5,976 of 8,443 | Files equal but for `[desc]` and `[pg]`: user's 2; the VM's G fixtures 11 of 24 (the rest differ by paragraph caches, 4.9) |
| 4.6–4.8 | 123 of 416 (416 but for line heights; main text 416) | 11 of 160 (159; main text 160) | 50,354 of 52,749; ISO not recorded | The ISO file left is the malformed layout |
| 4.9–4.10 | **416 of 416** (line heights included) | 159 of 160 | 50,631 of 52,749; 8,205 of 8,443 | Read-back warnings the original did not raise: 0 in both corpora and the VM fixtures (136 of 136 equal); files equal but for `[desc]` and `[pg]`: user's 43 |

## Log

(Newest last: what was done, measured, decided, or found.)

- 2026-10-13: the run set up (`docs/PHASE4_RUNBOOK.md`) after the user's
  decisions A (new files only), B (warn, then let the user choose), and C
  (page layout setup after Phase 4; the run writes its proposal), in
  `docs/research/DECISIONS.md`. Both corpora's readable SAMs are all
  format version 4.
- 2026-10-13, 4.1: the document writer `11c8:1c7b` and its callees mapped
  (`SAM_FORMAT_NOTES.md`, "Writing documents"): header sections, styles in
  function-key order, page frames, layouts (only those changed from the
  sheet; inserted layouts renamed `1`, `2`, … as written), `[l1]`, `[pg]`,
  `[edoc]`, the body, the pictures' bytes, and the `[Embedded]` directory
  with its trailer (the offset of `[Embedded]`). The section-name pointer
  table at DGROUP 0xbe6 is decoded in full; an annotated decompile with the
  names is `ghidra-work/out/amipro-annotated.c` (outside Git). The corpora
  agree: 567 of 576 files in the writer's order (the other 9, with
  `[panosefonttable]`, come from a later writer); document styles in
  function-key order in 447 of 447.
- 4.1, caches (VM fixtures K1–K6): Ami Pro opens files without `[pg]` and
  `<:#…>` without complaint and lays them out with its printer (equal to
  the originals where their caches came from the same printer; page ends
  move where they did not). It trusts given caches whose width matches
  the column (doubled heights: 24 pages for 14) and ignores others, and
  rebuilds `[pg]` from the paragraph caches (identical to the re-saved
  original). Decision: omit `[pg]`; write each paragraph's cache only while
  the paragraph, its style, and its place are as read; never compute one.
  The decompile agrees: Ami Pro itself writes `<:#…>` only for paragraphs
  whose height is valid (`11b0:1405` → `11b0:0476`).
- 4.1, found for 4.5: `[l1]` names the layout the document starts in (27
  files start in an inserted layout, 15 with no page break); the importer
  ignores it. 7 linked files store only inserted layouts and take Standard
  from their sheet; the importer then drops Standard.
- 4.2: the loss inventory (`FORMAT_COMPATIBILITY.md`, "Phase 4
  inventory"): 32 items, each kept, derived, or omitted, with its slice.
  New: `amipro_formats::roundtrip` describes a document as ID-free facts
  (place, aspect, value) and compares two; `read_back` writes a SAM and
  reads it back with a sheet written from the same document;
  `sam_losses` groups the differences into the kinds the Save As dialog
  will list (characters Windows-1252 lacks, text, character and paragraph
  formatting, styles, layouts, frames, tables, pictures, settings).
  `sam_document` is a stub until 4.3. The importer no longer keeps the
  raw `[tag]` and `[elay]` text in the document's legacy data (styles and
  layouts hold them). Audit baseline above; ISO corpus gate PASS.
- 4.3: `sam_document` writes the header sections in Ami Pro 3.1's order
  (kept values, 3.1's defaults where an older file lacks one, unknown
  sections after the section they followed, `[desc]` with this save's time
  and one more save), the styles stored in the document (all of them
  without a style sheet) in function-key order and then in the order they
  were read (kept as "style order": 147 of 171 files are not
  alphabetical), the document's own page layouts with `sheet.rs`'s writer,
  `[l1]` as read, `[edoc]` with the main text, and `>`. Found and fixed on
  the way: a document whose style sheet is missing had default styles and a
  generated Table Text stored as its own, which the writer would have
  written over the real sheet's; generated styles copied Body Text's
  function key; byte 0xAD (`</m>`, Ami Pro's visible hyphen, 2,980 times
  in 35 files) was read as an invisible soft hyphen and would have been
  written as `<*7>` (`DECISIONS.md`). The comparison now merges adjacent
  runs of one format, ignores a style's `parent` (Ami Pro has none), and
  treats the sections 3.1 always writes as the writer's, not losses.
- 4.3, VM: a detached document written without `[lay]` made Ami Pro stop
  responding (ended with `taskkill /f /im ntvdm.exe` from the guest's
  `cmd`, reached by clicking its taskbar button). With layouts written,
  T01–T05, ST1, and EP1 written by the writer show page areas
  pixel-identical to the originals' in Ami Pro.
- 4.3, layout measured before and after the 0xAD change: VM fixtures 136
  of 136 page counts and 76 of 76 page ends (136 fixture files now), and
  the user's corpus (299 of 394, 14,400 of 14,871, 744 of 1,108)
  unchanged; ISO corpus 22,779 → 22,773 line counts and 501 → 498 page
  ends, all in the 1-2-3 manuals with a missing style sheet. The cache
  experiment files moved from `vm-lab/OUT` to `vm-lab/K` so they stay out
  of the fixture comparison.
- 4.3, for 4.5: a page layout made in the app (no kept values) reads back
  with the values the writer filled in kept as legacy data (`rght.3`,
  `frmlay.10`…, `frame.field8`, field 13), and the derived text flag 0x200
  as an unknown frame flag. The reader should keep only values that differ
  from what the writer writes without them, and the comparison should
  leave out frame sequence numbers (field 13), which are bookkeeping.
- 4.4: the body writer (`body.rs`, from Phase 3.6) already writes every
  main-text construct; the audit's new measure, the main text alone, reads
  back equal in every document without page frames (79 of 79 ISO, 128 of
  128 user's); the others differ only at anchors, which frames bring
  (4.6). VM: G18–G23, V17–V19, V22, V23, O1–O7, PT1OUT, BN4, NX1OUT,
  NX2OUT, HY1OUT, TT2OUT written and opened in Ami Pro, screenshots at the
  start and after Ctrl+End compared with the originals': 22 of 24
  pixel-identical. G21's third page had Standard's margins: the writer
  closed all layouts with one `[elay]`, and Ami Pro reads only the first
  layout then (fixed in 4.5). O6 repeats `<+#>` inside an underlined run;
  Ami Pro draws the two runs' underlines with a one-pixel gap between
  them, and the writer writes the same formatting as one run, so the
  line is unbroken: the formatting is equal.
- 4.5: page layouts. The model gains the layout a document starts in
  (`Document::start_layout`, Ami Pro's `[l1]`; native v3 `startlayout`;
  the engine starts page 1 with it), and the importer reads `[l1]`, puts
  the layout named Standard first, and takes a sheet's Standard when the
  SAM stores only inserted layouts. The writer stores the standard layout
  unless it is the sheet's and unchanged (a fingerprint of what it was,
  dropped by Use Another Style Sheet and Save as a Style Sheet), every
  inserted layout renamed `1`, `2`, … with breaks and `[l1]` naming them so,
  each layout with its own `[elay]` (one `[elay]` for all made Ami Pro
  lose layout `1` of G21), and the file's end as Ami Pro writes it, `>`,
  an empty line, and an `[Embedded]` directory with its trailer even when
  empty. The reader now keeps a layout's and a frame's values only where
  they differ from what the writer writes without them, and treats the
  text flag 0x200 as derived, so a layout made in the app reads back as it
  was. Importer-made left pages are written as read.
- 4.5, layout measured (`DECISIONS.md`): user's corpus page counts 299 →
  302, page ends 744 → 762 of 1,108; VM fixtures unchanged; ISO corpus
  page ends 498 → 492 in manuals with a missing style sheet, now on the
  page size their own page tables name.
- 4.5, VM: G00–G23 written by the writer and opened in Ami Pro: the start
  and the end of each pixel-identical to the original's (48 of 48).
- 4.5, left for the loss check: one ISO layout whose extra header frames
  the importer keeps raw (a malformed file), and the 10 SAM layout frames
  with 0x8, whose text Ami Pro takes from the sheet (empty in the SAM, so
  empty in our layout too).
- 4.6–4.8, code (one module, `frames.rs`, since anchors number frames and
  tables together and a picture frame's flags need its content): the
  frame writer (`11c8:2a0f`) is shared by page frames (`[frm]`) and layout
  frames (`[lyfrm]`, refactored out of `sheet.rs`, whose style-sheet output
  is unchanged byte for byte). Page frames are written back to front with
  their group bits, flags derived from placement, look, and content, the
  kept unknown bits, fields 1–13, and for anchored frames field 14 (the
  anchor number), 15, and 16; the parts in Ami Pro's order (`[frmmac]`,
  `[frmname]`, `[frmlay]`, content) or the order read when the frame had
  parts the model does not know. The importer now keeps anchored frames'
  stored rectangle and fields 15 and 16 (Ami Pro places an anchored frame
  from its anchor and uses only the size, X02), which the writer gives
  back while the importer's own rules give the model's values from them,
  and the order of unknown parts. Anchored records without a mark stay
  anchored and keep their place in the anchor numbering. Frames numbered
  0 (older files) get the next free number, as Ami Pro gives them
  (`11c8:07dd`).
- Tables (`1490:0000`): the header (its length as read), `[h]` and `[w]`
  records with the model's sizes (records beyond the grid kept), each
  group closed by `[e]`, cell records with the join bits from the model,
  cell text with Table Text untagged and Body Text tagged (Ami Pro switches
  the paragraph writer's default style for cells), covered cells' own text
  when they had some, formulas after their cells, and cells the file left
  out still left out while empty. The importer generates Table Text before
  resolving paragraph names, so a cell tagged Table Text and an untagged
  one mean the same.
- Pictures (`11c8:4155`, `11c8:4521`): `[isd]` as read; after `>`, each
  picture's bytes and then its rendering's, page frames first, then each
  written layout's frames (not a style sheet's layout frames); the
  `[Embedded]` directory, one line per picture with the frame's number,
  extension, offsets and lengths, and a linked picture's path; pictures
  whose bytes the file never had keep their `[isd]` and flag.
- Round trip after 4.6–4.8: user's corpus **416 of 416 read back equal
  but for line heights** (123 exactly; main text 416 of 416); ISO 159 of
  160 (11 exactly; the other is the malformed layout). The slowest corpus
  document to write (8.6 MB, mostly pictures) takes about 10 ms (release;
  read-back time recorded at 4.9).
- 4.6–4.8, layout measured: unchanged in total (user's corpus 302 of 394,
  14,400 of 14,871, 762 of 1,108; VM 136/136, 885 of 896 lines, 76/76; ISO
  65 of 160, 22,773, 492). In two ISO manuals an anchored table's size is
  now nearer Ami Pro's (difference 4,222 → 0 and 4,320 → 2,160 twips): their
  cells name Table Text without defining it, and now get the generated one
  (no spacing) instead of a fallback with 6 pt below.
- 4.6, VM: F01–F20, L01–L15, C01–C04, C09, C11, M01–M04, P01, Z01–Z05,
  V10–V16 (57 fixtures) written by the writer, opened in Ami Pro, and saved
  again by it. Screens: 56 of 57 pixel-identical to the originals' (F17: the
  caret's three pixels). Ami Pro's re-saved frame records equal the
  originals' in every field but the sequence number for 55 of 57. In Z05
  the batch was written before caches were (4.9) and Ami Pro, measuring the
  first paragraph, renumbered the two Flow with text anchors; written with
  caches, Z05 is byte-identical to the original but for `[desc]` and
  `[pg]`, and Ami Pro re-saves both identically. In V12 the original's page
  table keeps the frame where it stood before its paragraph moved to page
  2 (top 1,440), and Ami Pro reuses that stale entry; without `[pg]` it
  lays page 2 out afresh and puts the frame below its paragraph's line
  (1,724), as V12's own rule says.
- 4.7, VM: TB1, V20, V21, TB2OUT written and opened: identical to the
  originals but for the caret's two rows in TB1 and V20; P02 and P03
  (pictures) identical.
- 4.9: paragraph caches. A paragraph keeps the `<:#H,W>` it was read with
  (`Paragraph::cache`: height, width, and a fingerprint of the paragraph
  and its style when read; native v3 `pcache`, strict), and the writer
  writes it back only while the fingerprint still matches (`Body`), right
  after the paragraph's ruler controls as Ami Pro does. Edits that change
  a paragraph or its style drop it; a split keeps it on the first part
  only while that part is unchanged. Round trip: user's corpus 416 of 416
  read back equal **including line heights** (they are calibrated from
  the caches), ISO 159 of 160. The body writer also got faster (the run
  cursor, plain pieces without allocation): the largest corpus document
  (8.6 MB, mostly pictures) writes in about 8 ms and reads back in about
  12.5 ms (release). Boundaries between two equal runs are written as a
  close and a reopen (`<-!><+!>`), as Ami Pro wrote them; multi-line
  controls are written with CR LF.
- 4.9, VM, three rules the round trip could not see (`SAM_FORMAT_NOTES.md`,
  "Three things Ami Pro relies on"), found by opening the written K1
  (README31), K2 (MERCURY), K4, and K5 in Ami Pro, then settled in the
  decompile:
  - README31 written was refused ("This file cannot be opened"): empty
    table cells got a text block though their record's eighth value is 0.
    The cell writer `1490:07c0` writes text exactly when that value is 1
    (text or a formula); 21,588 corpus records agree. The writer now sets
    it from the cell's content; the reader warns on a mismatch, and the
    audit counts read-back warnings the original did not raise (0).
  - MERCURY written lost its logo: its table's `[frmlay]` value 1 was
    written as the frame's bottom. Ami Pro reads it as the frame layout's
    first word and takes the frame's extent from it (`1058:1843`,
    `1148:0e1c`); in 353 of 406 tables it is not the bottom. Kept as its
    offset from the stored bottom.
  - MERCURY written then faulted after page 1 (GPF in `AMIPRO.EXE`, three
    times; one guest restart ended in a VirtualBox guru meditation and a
    power cycle): text after a note went on the note's `>` line. Bisected
    with the original changed one difference at a time; the note writer
    `11b0:0660` always ends the line after `>`. Fixed; the reader warns
    when a multi-line control's `>` has text after it (none of the
    originals does).
  - The user asked mid-run that the decompile be the first source for
    such findings too (it now is, `SAM_FORMAT_NOTES.md` cites it).
- 4.10: Save As SAM in the app (`UI_SPEC.md` §17; `DECISIONS.md`): a
  Format choice and two file type filters in the portal dialog, a typed
  extension deciding first; `.SAM` written to new files only (a hard link
  from a temporary file, which fails if the name exists; an existing file
  is refused in the status bar); the loss check before writing, with a
  dialog listing each kind of change and Cancel suggested; the close and
  quit paths wait for the file. Tests through the message path, and the
  dialog in the screenshot harness (light and dark).
- 4.10, VM: an ignored test (`sam_documents_for_ami_pro`,
  `AMIPRO_VM_IN`) builds two documents through the app's messages and
  saves them through Save As SAM: APP1, a new document typed in with
  centring, bold, italic, underline, and accented characters; APP2, the
  page fixture (header, a frame, a size run, 180 characters Windows-1252
  lacks, so through the dialog's Save Anyway). Both open in Ami Pro as
  made; APP2's frame showed no border, which found the lines bit (fixed,
  `1bcdece`). Ami Pro saves both back with the same bodies, frames, and
  layouts, adding only its caches and page table (6 pages, as the app
  shows).
- Found for a later layout slice (not Phase 4): Ami Pro flows text into
  both sides of a Wrap around frame, including a narrow strip (APP2: a
  0.6 in strip right of the frame takes one or two words per line), where
  the engine uses only the widest part (`Engine::slot`). The corpora's
  frames mostly sit at a margin, so the layout comparison barely sees it.
- 4.11, With contents (decompile `11c8:14e0`, `11c8:2a0f`, `11c8:3d76`):
  Save as a Style Sheet's dialog has the With contents check box, on each
  time as in Ami Pro; with it the sheet carries the main text, the page
  frames, header and footer text, and pictures. The sheet writer now marks
  every frame it writes as the sheet's (0x8, and 0x80 on the layout), as
  Ami Pro does, which corrects 3.6 (`DECISIONS.md`). Compared with sheets
  Ami Pro wrote in the VM from the same documents: SS2 with and without
  contents, and F01 with them, equal but for `[desc]` (and the caches Ami
  Pro measured for F01, which had none). A new document made in Ami Pro
  from a sheet written with contents (WC1) starts with its header, text,
  and frame. New example `write_sheet`.
- 4.11, edited documents (`crates/editing/tests/sam_after_edits.rs`): a
  document made and edited through commands (text in the main text, a
  header, a frame, and a table cell; a style made, modified, and applied;
  bold; a frame moved and resized; a page break) writes and reads back as
  it was, with no loss listed; edits drop exactly the caches they make
  stale. Found and fixed on the way: new tables' cells were Body Text
  (now Table Text, made with the first table, as Ami Pro does); the
  reader kept an app-made frame's and table's derived values, which the
  loss check would have listed (now dropped when the writer gives the
  same without them; the corpora's round trip is unchanged); a style
  defined nowhere was compared by definition (now not: Ami Pro formats it
  as Body Text and nothing is written).
- 4.11, VM: README31 and MERCURY edited through commands (an ignored test
  writes `EDK1`, `EDK2`) open in Ami Pro; saved again by it and read here
  (new example `compare_sam`), MERCURY differs from its original's re-save
  only in `[desc]`, and README31 also in three pictures that sit in a
  table's cells: Ami Pro moved them with the table to the page the page
  break pushed it to, and ignored a move made here (flag `0x20`; 98 such
  frames in 23 files). Frames in table cells are a model gap for a later
  phase (`SAM_FORMAT_NOTES.md`). APP1's re-save differs only in the
  function keys Ami Pro gives styles written without one (2, 3, 4), as
  recorded at ST1.
- 4.12: the phase exit. Layout comparison at the end of the run, unchanged
  from 4.5–4.8: user's corpus 302 of 394 page counts, 14,400 of 14,871
  line counts, 762 of 1,108 page ends; VM fixtures 136/136, 885 of 896,
  76/76; ISO corpus 65 of 160, 22,773 of 24,971, 492 of 1,307. Final round
  trip in `FORMAT_COMPATIBILITY.md`. The page layout setup proposal is
  `UI_SPEC.md` §18 (Ami Pro's Page menu and Modify Page Layout panels were
  captured in the VM: Margins & columns, Page settings, Lines, Header,
  Footer, Header/Footer…, Floating Header/Footer…, Insert Page Layout ▸,
  Page Numbering…).

## Waiting for the user

1. ~~`UI_SPEC.md` §18's four questions~~ (answered 2026-10-13, L1–L4).
2. ~~The physical check of Save As SAM~~: the recheck passed (2026-10-13)
   after the fix below; it also found that a refused save was reported
   only in the status bar (now a dialog, below). With contents: not yet
   reported. A second check after the page layout run passed too (the
   user, 2026-10-13: "Resave of a SAM file worked").

## After the run: the user's first check (2026-10-13)

- Saving a document from the user's corpus as SAM listed "2 page layouts,
  1 document setting": the app had opened it without its style sheet, and
  the loss check read the written copy back against a sheet generated from
  the document, whose Standard layout became a layout the document never
  had. The written file was right. Fixed (`DECISIONS.md`): a document
  whose sheet was not found is read back without one; the stand-in sheet
  for the others is written with contents (else four more documents would
  list a layout); the importer finds a `styles` folder two levels up. The
  audit now runs the app's own loss check: nothing listed for 416 of 416
  user documents (with and without the style folder), 136 of 136 VM
  fixtures, 159 of 160 ISO documents (the malformed layout). The audit
  had measured read-back against the real sheet file, not the check the
  user sees; it measures both now.
- The user's recheck passed: the same document now saves as SAM with no
  dialog. Saving over the same `.SAM` was refused as decision A says, but
  only in the status bar, which the user would not have seen without
  looking for it. Every refused or failed save now opens a dialog with the
  reason, Choose Another Name…, and Cancel (`UI_SPEC.md` §17,
  `DECISIONS.md`); tests through the message path, screenshots in light
  and dark.
