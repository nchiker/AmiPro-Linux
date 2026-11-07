# Import Progress

Live state of `docs/IMPORT_RUNBOOK.md` (tag `import-run-start`). Resume at
the first slice that is not `DONE`.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| IM.1 Setup | DONE | (the setup commit) | `IMPORT_NOTES.md`; `UI_SPEC.md` §31; IM1–IM12 in `DECISIONS.md`; the baseline below | Tag `import-run-start` |
| IM.2 The readers' ground | DONE | `3253516` | `zip::tests`, `xml_tree::tests`, `import::tests`; every part of 4,142 sample files read or refused without a panic (`examples/import.rs --parts`) | IC1 |
| IM.3 Realise | DONE | `faf4b12`, `593ed93` | `tests/realise.rs`: built, realised, built again: 279 of 290 VM, 258 of 417 user, 142 of 160 ISO exchange documents equal (the rest what the exchange document cannot carry, listed below) | IC2–IC13 |
| IM.4 Plain text | DONE | `593ed93` | `read_text::tests` (VM IM2's and IM3's lines give IMT's and IMT3's paragraphs and styles) | IC14 |
| IM.5 RTF | DONE | `593ed93` | `read_rtf::tests` (VM IM1); RTF round trip 256 of 290 VM, 215 of 417 user, 121 of 160 ISO; 660 sample RTF and text files imported, valid, no panic | IC15; the RTF writer improved (style lists, frame names, seconds, page-break rows, thick lines) |
| IM.6 DOCX | DONE | `10af131` | `read_docx::tests`; DOCX round trip 269 of 290 VM, 224 of 417 user, 134 of 160 ISO; every sample DOCX imported or refused with its reason, valid, no panic | IC16–IC19 |
| IM.7 ODT | DONE | `97e3b7a` | `read_odt::tests`; ODT round trip 273 of 290 VM, 251 of 417 user, 136 of 160 ISO (realise alone: 279, 258, 142); 4,142 sample files: 4,099 imported and valid, 43 refused with their reason (damaged, password-protected, not a document), no panic, the slowest 1.3 s; the VM folder's ODT through LibreOffice before and after the writer's changes: the same page count for all 247 it converts | IC20–IC24; the ODT writer improved (IC24) |
| IM.8 EMF | DONE | `6952546` | `emf::tests`, the app's `enhanced_metafiles_are_drawn_and_converted_for_ami_pro`; the 153 distinct EMF pictures of the sample packages all played (`examples/emf_census.rs`, the slowest 6.6 ms) and drawn by the app's metafile renderer (the ignored `emf_renderings`; looked at: charts, logos, tables, equations, turned text, dashed and pattern-filled shapes as their programs draw them) | IC25 |
| IM.9 The starting layout | NOT NEEDED | | `[l1]` is read since Phase 4.5 (`fd8a64d`); `SAM_FORMAT_NOTES.md` corrected | |
| IM.10 Round trips | DONE | `6952546` | `tests/realise.rs` (`realise_round_trip_report`, `AMIPRO_REALISE_FORMAT`): each document built, written, read, realised, and built again, equal in what the format holds, VM / user / ISO: RTF 257, 215, 121; DOCX 269, 224, 134; ODT 273, 251, 136; plain text (written again equal) 289, 416, 160, the rest control characters the reader leaves out (undefined Windows ANSI codes, a DOS end-of-file mark); realise alone 279, 258, 142 of 290, 417, 160 | Settings compared since IM.7 |
| IM.11 The app | DONE | (the IM.11–IM.12 commit) | `page_surface_app_tests`: `other_programs_documents_open_through_import_options_in_a_protected_tab`, `a_word_document_the_app_exported_opens_with_its_text_and_frames` (DOCX and ODT exports read back: text and frames), `insert_file_puts_a_file_at_the_caret_as_one_undo_step` (main text and a frame's text), `paste_takes_rich_text_with_apply_styles_and_falls_back_to_text`, `a_dropped_document_opens_through_import_options`, `import_options_take_clicks_in_the_window` (widget path); `tests::both_builds_open_other_programs_documents` (the text-editor build too); `import::tests`; screenshots light and dark (`import_screenshots`) | IC26–IC28 |
| IM.12 The Import Report | DONE | (the IM.11–IM.12 commit) | `the_import_report_lists_what_changed_with_jumps_and_copy`, `the_import_reports_buttons_take_clicks_in_the_rail` (widget path); screenshots light and dark | IC29 |
| IM.13 Import Folder… | DONE | (the IM.13 commit) | `import_folder::tests` (subfolders kept, never over a file, a damaged file listed, the originals unchanged, `Import report.txt`); `import_folder_imports_a_folder_from_the_rail` (the panel, the folders kept, Import clicked in the rail, progress, a second run beside the first); screenshots light and dark | IC30 |
| IM.14 Comparisons | DONE | (the IM.14–IM.15 commit) | Robustness: 4,142 sample files, 4,099 imported, 43 refused with their reason, no panic, the slowest 1.2 s (`examples/import.rs`). LibreOffice against the app (90 LibreOffice- and Word-written samples, evenly spaced, `import_files`): the same page count for 82 of 89 (DOCX 36 of 40, ODT 28 of 29, RTF 18 of 20); pages with 95% of their words on the same page 32 of 83, 80% 53 (without one magazine layout of 16 pages: 30 and 49 of 68). Timings (release): README31 read and realised in 3.1 ms as DOCX, 2.6 ODT, 3.6 RTF, 1.1 text, laid out in 19 ms; the largest samples 11 ms and 6 ms. Typing A/B against `import-run-start`: the page surface 0.761–0.765 against 0.762–0.765 ms, the default editor 0.814 against 0.813–0.816 ms (no regression); the rich-editor prototype 0.38 against 0.43 ms in both rounds (below) | IC31–IC33 |
| IM.15 Twins | DONE | (the IM.14–IM.15 commit) | `import_twins` (Open, Import Options, Save As SAM, Save Anyway) wrote AIM1 (Ami Pro's IM1.RTF), AIMD (README31 through the app's DOCX), AIMO (a LibreOffice ODT: a text box and a table in a box), AIMW (a Word DOCX: pictures in cells), AIML (a Word DOCX: a numbered list, 1½ spacing, 3 pages), AIMN (a comment); Ami Pro 3.1 opened each (VM screenshots outside Git): AIM1, AIMD (14 pages, the same line breaks), AIML (3 pages, the same line breaks), AIMN (the note, its initials and text), and AIMO's table as the app shows them, after three fixes (IC31–IC33); AIMO's text beside two frames and AIMW's pictures differ (findings) | |
| IM.16 Exit | DONE | (the handoff commit) | `tools/verify.sh --release --corpus` PASS at `9b9c3a0` (core 461 + 10 ignored; app 333, 115, 340 with 50, 6, 50 ignored; release builds; ISO corpus gate); the measures equal the run's start; the records (`UI_SPEC.md` §31 notes, `FEATURE_PARITY.md` IO-06, `UI_ACCEPTANCE.md` "Import", `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `REFERENCE_FEATURE_MATRIX.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue and `OUTPUT.md`, the handoff) | Physical check passed 2026-10-25 |

## Baseline (at `import-run-start`, `target/import-baseline/`)

Measured with `target/import-baseline/baseline.sh` (outside Git; the
round-trip audit and the layout comparison over the user's corpus, the ISO
corpus, and the VM folder, each run with `AMIPRO_STYLES`). The VM folder
gained the run's Ami Pro fixtures (IMC, IMA, IMK, IMI, IMX, IMT, IMT3, NT1).

- Round trip: user's corpus 417 of 417 read back equal (Save As would list
  nothing 417 of 417), ISO 159 of 160, VM folder 290 of 290.
- Layout comparison: user's corpus page counts equal 315 of 394, paragraph
  line counts 14,880 of 15,356, page ends on the same page 762; ISO 69 of
  160 and 22,773 of 24,971 (most of its styles from a missing STY); VM
  folder 289 of 289 and 1,859 of 1,871, page ends 112.

## VM fixtures

- `../AmiPro-Reference/vm-lab/IN/IMP`: IM1.RTF (styles, page set-up, direct
  formatting), IM2.TXT (line ends, tabs, blank lines), IM3.TXT (`@Style@`
  lines, PC-ASCII bytes).
- `OUT/IMC.SAM`, `IMA.SAM`, `IMK.SAM`, `IMI.SAM`: IM1.RTF opened with each
  Import Options choice; `IMX.SAM`: IM1.RTF inserted into a new document
  with Convert; `IMT.SAM`: IM2.TXT with Ami Pro's defaults; `IMT3.SAM`:
  IM3.TXT with CR/LF at paragraph ends only and Keep style names;
  `NT1.SAM`: an Ami Pro note.

## Choices made in the run (IC…; `DECISIONS.md`)

- **IC1** The readers' ground is the project's own, in the core: a zip
  reader through the central directory (inflate from the caller, as export
  passes deflate; the app's `flate2`, `miniz_oxide` for the core's tests
  and examples), an XML tree reader (namespaces, no entity expanded, nesting
  limited to 512), import options and findings. The formats' crates resolve
  nothing Ami Pro needs (`IMPORT_NOTES.md`).
- **IC2** `realise` is the export's builder run backwards, measured as a
  fixed point: a document built, realised, and built again gives the same
  exchange document.
- **IC3** Opening with Convert: the file's styles in the file's order; the
  new document's own styles the file does not use are left out (Body Text
  stays).
- **IC4** The file's first (default) style becomes Body Text; a file style
  named Body Text beside it is "Body Text (file)"; every other name is kept,
  a Normal that is not the default too.
- **IC5** What Ami Pro holds only in styles (a paragraph's own space above
  and below, keep with next, keep together, rules) comes in through a
  variant style named after its style ("Body Text 2"), reused for the same
  values; Apply styles leaves it out (as Ami Pro's Apply) and lists it.
- **IC6** Line spacing "at least" a height is single spacing with that
  printer line height for the font (EC5 backwards); multiples other than 1,
  1½, and 2 are custom heights (1.15 × the size per line); exact heights are
  custom.
- **IC7** Tables are Ami Pro's page tables (a frame alone in its line); a
  table at an offset is With para above, below the paragraph before; a box
  holding only a table is a table frame; tables in frames, headers, and
  cells are their text, a row a paragraph (listed).
- **IC8** Frames: placed from the page or margin, Where placed on the page
  their paragraph is laid out on (two layouts: without them, then with);
  placed from a paragraph with text beside them, Where placed at their
  paragraph's place on its page (Ami Pro's With para above frames have no
  text beside them); placed from a paragraph without, With para above; in
  the line, Flow with text; in headers, repeating. Shapes in frames' and
  headers' text are their text, listed; a shape at a cell's start is the
  cell's frame.
- **IC9** Comments are Ami Pro notes as VM NT1 shows them: yellow, the
  author's initials (from the name where the file has none), the date or
  now.
- **IC10** Footnotes are numbered 1, 2, 3 and endnotes i, ii, iii in the
  text (raised) and at the document's end, each number and a tab before its
  text.
- **IC11** A page number the file shows empty on the first page starts on
  page 2 (Ami Pro's numbering from a later page).
- **IC12** Header and footer text whose indents reach past the page's
  margins moves the band's own margins (Ami Pro's bands have margins of
  their own); its tabs move with them.
- **IC13** Dates are Ami Pro's date objects (Today, Created, Last revised),
  times its Time object or Doc Info times, other document fields Doc Info
  entries, Keywords a power field, index and contents entries Ami Pro's
  marks; fields with no counterpart keep their result (listed).
- **IC14** Plain text: a form feed is a page break; Ami Pro's leading space
  after a second blank line (CR/LF at lines) is not copied.
- **IC15** RTF: a style entry without `\s` is style 0 (Word writes Normal
  so); a paragraph without tab stops of its own has the default stops every
  `\deftab`; positioned paragraphs become one frame per run of the same
  frame; `\shprslt` and `\nonshppict` are skipped where the shape is read;
  text in another code page than Windows ANSI comes from its `\u` (Word and
  LibreOffice write both); a table without cell edges has inch-wide
  columns.
- **IC16** DOCX: the default paragraph style is style 0 (IC4), the others
  in the file's order; character styles toggle bold and italic as Word
  does; a text box holding only a picture is a picture frame; a rounded
  rectangle's adjustment is Ami Pro's corner rounding, an outer shadow its
  shadow; a picture's kind comes from its bytes, then its name; the empty
  paragraph Word keeps after a last table is left out.
- **IC17** Lists: each run of list paragraphs gets Ami Pro's list style
  for its kind and depth (Bullet 1, Number List, List 2…), each style made
  once through the editing commands (`SetList`, `ChangeLevel`).
- **IC18** A frame's opacity: a frame text runs through is opaque unless it
  lies behind the text; any other frame is opaque where it has a fill.
- **IC19** A picture lying inside its frame keeps its place as the frame's
  margins; a picture reaching past its frame is cut by it (Ami Pro's
  graphics scaling).
- **IC20** ODT: the body style is the file's Body Text (LibreOffice's
  Text body) where its paragraphs use it, else the default style Standard
  (IM4); a paragraph naming no style is in it.
- **IC21** ODT page styles: each master page change starts a section on a
  new page (ODF changes headers and footers only at a page break); ODF's
  page margins reach the header and footer, so the text's margin is the
  margin, the band, and its spacing, and the band's distance the margin.
- **IC22** ODT breaks: a break after a paragraph is Ami Pro's break at its
  end; a break before is the break at the end of the one before (before
  the first paragraph a column break stays at its start).
- **IC23** ODT list levels: text with no number is Ami Pro's bullet text
  ("Note:"); a style's list is at its outline level; `continue-list` and
  `xml:id` keep a list's count across the paragraphs between.
- **IC24** The ODT writer improved on the way (each checked through
  LibreOffice): a break that ends or begins a paragraph is the paragraph's
  break after or before, not an empty paragraph; a table starting a
  section carries its master page (LibreOffice honours it; the empty
  paragraph before it is gone); a paragraph mark's look is written only
  for a paragraph without text (a paragraph style's text properties are
  all its text's, which set a paragraph's text in its mark's size); dates
  keep their pictures as data styles; a style's hyphenation is
  `fo:hyphenate`; list labels keep right alignment and their look; a
  bullet of more than one character is text; text boxes keep their
  rounded corners (`draw:corner-radius`).
- **IC25** Enhanced metafiles play into the Windows metafile's display list:
  logical units through the world transform and the mapping mode onto the
  reference device, the header's frame onto the unit square (its natural
  size the frame's, in hundredths of a millimetre); EMF+ comments skipped
  (files carry their EMF records too); a path's clip as its bounds; EMF is
  Insert › Image…'s too, a metafile in Frame Properties (not turned), a
  bitmap at 300 dpi for Save As SAM, and EMF in DOCX, ODT, and RTF
  (`\emfblip`).

- **IC26** Import Options live in the app, not the tab: Open has no tab
  until the file is read; Insert and Paste name their tab and caret.
- **IC27** Insert › File… goes in at the selection's start and leaves the
  selection (Ami Pro's Insert leaves the insertion point where it was);
  Paste replaces it.
- **IC28** Paste takes RTF without asking (Ami Pro's Paste always uses
  Apply styles), and no Import Report position jumps for inserted text
  (the realised document's places are not the inserted ones); its findings
  are still listed.
- **IC29** The Import Report's Go to walks a kind's places in turn and
  wraps; a place whose paragraph was deleted is skipped.
- **IC31** Save As SAM (and Save as a Style Sheet) shortens style names
  longer than Ami Pro's 13 characters (`1000:80cd`), each still unique
  ("Body Text (fi", "Body Text ( 2"), and lists them once ("2 style names
  longer than Ami Pro's 13 characters, shortened"): twin AIM1 showed Ami
  Pro finding no style for a longer name (IM4 built; Word's names such as
  "List Paragraph 2" are common).
- **IC32** Automatic rows take Ami Pro's measure: Ami Pro measures every
  Automatic row as it opens a file (VM RC1–RC3), so a file's minimum row
  height more than 0.1 in taller than the row's text becomes the text's
  height (twin AIMO); rows of empty cells keep their height, as Ami Pro
  keeps them; files the app wrote (their generator AmiPro-Linux, now also
  in RTF) keep their heights, which are Ami Pro's measure (the round trips
  stay equal).
- **IC33** A cell's picture gives its row at least the picture's height
  (the picture fills its cell, T2); such a row is not shrunk (twin AIMW:
  the pictures were written 0 high).
- **IC30** Import Folder… (File menu, after Open) takes the files named as
  a format read and then recognised by their contents (a `.docx` holding
  text is read as text); text files as each line a paragraph (IM9's
  default); its style option is its own, starting from Import Options'
  last choice.

## Log

- 2026-10-24, IM.14–IM.15. Comparisons and twins (above). Found on the
  way and fixed: Save As SAM wrote style names longer than Ami Pro's 13
  characters (IC31); Automatic rows kept a file's minimum height Ami Pro
  does not keep (IC32); a cell's picture was written 0 high (IC33). The
  round-trip audit and the layout comparison over the three corpora are
  equal to the run's start after them (`target/import-exit/measures/`),
  and the realise round trips equal IM.10's (`target/import-exit/after-flag/`).
  Not fixed, recorded for the layout:
  - **Text beside two frames** (AIMO): two Wrap around frames side by side
    with a 0.8 in gap between them; Ami Pro puts the empty paragraphs
    after their anchor in the gap but the next line of text below both
    frames; the app sets that text in the gap too. A first rule (no text
    between two frames) put the text 0.8 in lower than Ami Pro and was
    reverted; a fixture with two frames and text of known widths would
    settle it (the corpora have no such page: their measures did not move
    either way).
  - **Pictures placed inside a cell's frame** (AIMW, one of LibreOffice's
    positioning tests): a picture offset about −1,020 within its cell
    frame shows in the app and not in Ami Pro.
  - **The rich-editor prototype** (feature-gated) types 13% slower than at
    the run's start in both A/B rounds (0.43 against 0.38 ms); the default
    and page-surface builds are unchanged, and nothing the run changed is
    on its typing path (a code-generation difference, not looked into).
  - **LibreOffice and the app substitute fonts differently** (Calibri:
    Noto Sans there, Liberation Sans here, about 18% taller lines there),
    most of the words-per-page difference in the comparison.

- 2026-10-24, IM.11–IM.12 (resumed after the reboot from the working
  tree, which equalled `wip/import-im11`): File › Open's dialog lists All
  documents, then Ami Pro, AmiPro-Linux, and each format read; a file that
  is not SAM or `.amipro` and is one of the formats by its contents goes to
  Import Options (app-level state: the document does not exist yet), then
  is read and realised on a thread of its own (`import::in_background`)
  and opens in a new tab as a protected source (Save goes to Save As).
  Insert › File… (page surface; after Image… in the Insert menu) puts the
  file at the caret (the selection's start) of the tab it was chosen in,
  in any flow, as one `InsertDocument` step. Paste reads the clipboard's
  `text/rtf` (also `application/rtf`, `text/richtext`) first and imports
  it with Apply styles, replacing the selection; without RTF it pastes the
  text as before. A document dropped on the page opens through Import
  Options (pictures still go in). The Import Report panel (rail, 300 px)
  opens by itself when a file listed anything: the file and format, a
  sentence, each finding with Go to (cycling through its places, which
  follow paragraph identities after edits), Copy; the status bar's "N
  import notes" opens it again. Comments now carry a place (a marker
  before each note) in all three readers.

- 2026-10-21, IM.11 paused for a reboot. The slice's work so far is
  uncommitted in the working tree (and backed up on the branch
  `wip/import-im11`, one commit on top of master's):
  - Core, built and tested: `Command::InsertDocument` and
    `amipro_editing::insert_document` (a realised document put at a
    position as one command; its frames under new identities; its layouts
    and floating headers left out; three tests pass), `caret_format`
    made `pub(crate)`, and `RealiseOptions::into_text` (a file inserted
    into a frame's or a cell's text: tables and frames become their text).
  - App, not yet compiling: `app/cosmic/src/import.rs` (format by
    contents, Import Options' dialog, `ImportJob` read and realised off
    the interface's thread with `page_layout::with_private_shaper`,
    `Report`, the status message, the options kept as words; three tests),
    and in `main.rs` the session's `import_dialog` and `import_report`,
    the preferences `import_styles`, `import_line_ends`,
    `import_keep_names`, the messages `ImportOptions`, `Imported`, and
    `InsertFileDialogFinished`, File › Open's dialog offering the formats
    read, and `insert_file_task`.
  - To finish: `document_filters` (the Open dialog's filters from
    `import::PATTERNS`, with SAM and `.amipro` for File › Open),
    `use std::path::Path` for `insert_file_task`, the three messages in
    `update`, opening a non-SAM file through Import Options, the dialog's
    view, the tab (source protected), a dropped document, Insert › File…'s
    command, Paste of RTF, then the slice's tests and screenshots
    (runbook).
  - The XP VM was shut down (ACPI) with Ami Pro empty; add the transient
    share again after starting it (`VM_LAB.md`, Session).

- 2026-10-21, IM.6–IM.7: the DOCX and ODT readers. The round trip's
  comparison now covers the settings too. What ODF cannot hold, as the
  measure allows for it (`within_odt`): a header distance without a
  header, a single column's gap, a frame lowered below the baseline, lines
  of three strokes, a picture frame's corners, heading rows below the top,
  levels that do not count again after a higher one, a page break inside
  a paragraph, and Ami Pro's floating headers and footers in mid-page
  (each a new page style, so a new page: four ISO documents get a page
  more).

- 2026-10-21, IM.2–IM.5: the zip and XML readers, `realise`, the text and
  RTF readers. Found and fixed on the way: the model's span check divided
  by zero for a table without columns (`Table::validate_spans`); the RTF
  writer dropped style lists, frame names, seconds, page-break rows, and
  lines past 75 twips.
- 2026-10-21, IM.1: the map (VM fixtures, the decompile of `rtf.flt`,
  `AMIXIMP.DLL`, and the import engine `1318:0000`), the readers
  evaluated, the sample documents gathered, the proposal (`UI_SPEC.md`
  §31), the user's answers IM1–IM12, this file.
