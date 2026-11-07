# Compatibility matrix

What Ami Pro for Linux opens, writes, and does, how far each item has been
checked, and where the evidence is. Written 2026-11-07 from the repository's
own records; it is the user-facing summary. A short list of gaps is in
`docs/KNOWN_LIMITATIONS.md`; the evidence per feature is in
`FEATURE_PARITY.md`; the long format register is
`docs/research/FORMAT_COMPATIBILITY.md`. Where a number is not recorded in the
repository, this page says "not measured".

Corpus files are private and are never named here; only counts recorded in
the project's progress files are quoted. They were taken at different dates,
so a count belongs to the run that recorded it, and the date is given.

## 1. Files the app opens

| Format | State | Evidence | Notes |
|---|---|---|---|
| Ami Pro SAM (3.1 and earlier format versions) | Built, checked | User's corpus: 417 of 417 documents read back equal after a write (`docs/COMMANDS_PROGRESS.md`, 2026-11-05); ISO corpus 159 of 160 (the other has a malformed layout the importer keeps raw); VM folder 417 of 417. Documents written by the app open in Ami Pro 3.1 as their originals do (VM "twins"), run by run: layouts, frames, tables, pictures, notes, fields, contents, master and merge files, passwords | Pages, frames, tables, pictures, notes, and fields are read into the model. What the model cannot hold is kept as raw data and written back (`FORMAT_COMPATIBILITY.md`, "Phase 2.4 mapping") |
| Ami Pro STY style sheets | Built, checked | User's 192 of 192 and the ISO corpus's 63 of 63 parse; written as sheets they read back with the same styles and layout (`PROJECT_STATUS.md`, 2026-10-17) | A STY is opened as a sheet and applied; it is never written over |
| Password-protected SAM | Built, checked | Ami Pro's protected files from the VM: 7 open, and protecting again gives Ami Pro's bytes exactly (`docs/COMMANDS_PROGRESS.md` CA.14) | Opens with its password (a wrong one leaves the box open for another try) and saves protected |
| Native `.amipro` (v3) | Built, tested | Exact model round trips: ISO corpus 160 of 160 (2026-10-11); the user's corpus 416 of 423 files, the other 7 being Word Pro, protected (before password support), or truncated files (`PROJECT_STATUS.md`); malformed-input tests (`docs/Design/NATIVE_FORMAT_V3.md`); v1 and v2 files load | The lossless format. It keeps everything the model holds, for example 1st line bold, image adjustments, and Convert Object results |
| DOCX | Built, tested | Read by the project's own zip and XML readers; documents from Word and LibreOffice opened in Ami Pro 3.1 after Save As SAM (`docs/IMPORT_PROGRESS.md` IM.15) | Open, Insert, Import Options |
| ODT (and flat ODT) | Built, tested | As DOCX (LibreOffice-made twin AIMO) | |
| RTF | Built, tested | As DOCX (Ami Pro's own RTF read back, twin AIM1) | |
| Plain text | Built, tested | `crates/exchange` reader | |
| Markdown (CommonMark, GitHub tables, strikethrough, footnotes) | Built, awaiting physical check | Scenario module `markdown`; decisions MD1-MD3 | Opened, inserted, exported, and saved as an exchange format |
| Data files for Merge and Sort: Ami Pro's own, tables, DIF, fixed, comma, CSV, TSV, DBF, XLSX, ODS | Built, tested | Merge run ME.3, ME.4 (`docs/MERGE_PROGRESS.md`); Ami Pro's merge over a file the app wrote equals the app's merge (twin AMT2) | Data sources for Merge and Sort, not documents. dBASE memo fields are not read |
| Pictures: DIB/BMP, WMF, EMF, PNG, JPEG, GIF, SVG, Ami Pro drawings (`.sdw`) | Built, tested | Pictures run (`docs/PICTURES_PROGRESS.md`); drawing run (`docs/DRAWING_PROGRESS.md`) | Stored in frames |
| Pictures: TIFF, PCX | Built, tested | Converted when imported (`docs/KNOWN_LIMITATIONS.md`) | A gray-scale 8-bit TIFF keeps its bytes for image processing |
| Pictures: CGM, HPGL, DrawPerfect, Freelance, PIC, AutoCAD, EPS, WPG | Not built | | Ami Pro's other picture filters |
| Word Pro (`.LWP`) | Not read | 5 Word Pro files in the user's corpus are refused with a message that says so (`PROJECT_STATUS.md`) | Not planned |
| WordPerfect, Word 1 and 2, WordStar, Write | Not built, not planned | | |

What the DOCX, ODT, and RTF readers list in the Import Report as not
carried: tracked changes (accepted), highlighting, character spacing, hidden
text, nested and floating tables (kept as text or in a frame), charts and
drawn shapes (their picture where the file has one), equations, form fields,
linked text boxes, right-to-left text. Comments become Ami Pro notes;
footnotes and endnotes are kept. Style names longer than Ami Pro's 13
characters are shortened on Save As SAM and listed.

## 2. Files the app writes

| Format | State | What is written and what is lost |
|---|---|---|
| SAM, as Ami Pro writes it | Built, checked | Save writes over a `.SAM` after the loss check and keeps `NAME.SAM.bak` the first time. Sections equal to Ami Pro's byte for byte at the end of Phase 4: 50,631 of 52,749 in the user's corpus and 13,833 of 13,835 in the VM folder (the rest differ by `[desc]` bookkeeping, the omitted `[pg]`, and some body controls). Imported files are never written over |
| SAM loss check (Save As lists these) | Built | Characters Windows-1252 lacks (written as `?`); style names over 13 characters (shortened); table line colours other than black; pictures Ami Pro cannot store (written as bitmaps of at most 300 dpi); picture descriptions; image adjustments on bitmaps other than a gray TIFF (baked in); pictures linked to Linux paths (copied in); languages Ami Pro has no number for; merge conditions; formula functions Ami Pro lacks (`@avg`, `@min`, `@max`, `@count`) |
| STY style sheets | Built, checked | Written from the model; an opened STY is not written over |
| Native `.amipro` | Built, tested | Lossless |
| DOCX | Built, checked | Styles, runs, sections, headers and footers, frames as DrawingML text boxes, tables with spans and borders, lists, fields, footnotes, notes as comments, bookmarks, pictures, Doc Info. Lost (listed before writing): protected text, 1st line bold, overstrike, no-hyphenation over part of a paragraph, index and contents entries, power fields with no Word counterpart (their results are written), frame lines on some sides only, frame groups, left pages unlike right pages, centred pages, text tightness, short paragraph rules, column balance, late page numbers, drawings and charts (written as pictures), frames with unknown content |
| ODT | Built, checked | As DOCX, with frames as `draw:frame`; also loses columns in frames and rounded frame corners |
| RTF | Built, checked | As DOCX, with frames as `\shp` with a positioned-paragraph result; also loses columns in frames and rounded frame corners |
| Plain text | Built, tested | The text; tables as tab-separated rows; frames' text after their anchor's paragraph. Headers and footers, pictures, and all formatting are lost |
| Markdown | Built, awaiting physical check | Headings, lists, quotes, code, bold, italic, strikethrough, tables, footnotes, pictures beside the file. Always lost: page layout; fonts, sizes, and colours; tab stops, indents, alignment, and spacing; other styles; table cells that span or hold several paragraphs |
| PDF (and PDF/A-2b on request) | Built, awaiting physical check | Written from the same layout as the screen with embedded fonts; PDF/A is validated as it is written |
| Print | Built, awaiting physical check | Through the desktop's print portal. The portal applies copies, collate, reverse, and odd or even pages; the app sends only the chosen page range |
| Charts | Built, awaiting physical check | Save Chart as Image (PNG or SVG); Copy as CSV |

Save As and Export show the loss list before writing, built from the same code
this table is read from (`crates/exchange/src/losses.rs`,
`amipro_formats::sam_losses`); a document that uses none of these things shows
an empty list. Notes, footnotes, and bookmarks are carried by DOCX, ODT, and
RTF and read back as Ami Pro's. Frames and layouts are lost or reduced in other
word processors' formats, as in Ami Pro.

## 3. Ami Pro feature areas

States: Built; Built, awaiting physical check (automated checks and, where
noted, the VM have passed; the user has not yet used it on the desktop);
Partial; Decided against; Deferred; Not planned.

| Area | State | Evidence | Notes |
|---|---|---|---|
| Pages, page layouts, columns, headers and footers | Built | Layout comparison against Ami Pro's caches and the VM; page-layout twins (`FEATURE_PARITY.md` PG-01, RF-01) | User's corpus page counts equal 322 of 394 and line counts 14,938 of 15,356 (2026-11-05, `docs/COMMANDS_PROGRESS.md`); the ISO corpus agrees less (70 of 160 page counts in the 2026-11-05 baseline). Line breaks can differ by a word because text is shaped with the system's fonts |
| Frames (text, picture, table; placement; wrap; groups; frames in cells) | Built | Frame twins 6 of 6 (2026-10-17); cell-frame twins | Open layout differences: a word wider than the gap beside a frame goes below it (FG3); a Wrap around frame beside a column's first lines (FG5). Draft mode draws no frame markers (CK10) |
| Styles, style sheets, ruler, bullets and numbering | Built | Physically accepted 2026-10-13 (ST-01, ST-02); ruler twins 12 of 12 | |
| Style attribute 1st line bold | Built, awaiting physical check | Bit 4096 of `[fnt]` read and written as Ami Pro does; compared with a VM picture in the live pass of 2026-11-07 | Not carried by DOCX, ODT, or RTF (listed) |
| Fonts | Partial | Faces uninstalled on this machine are replaced by metric-compatible substitutes where one exists (`PROJECT_STATUS.md`) | Pagination can differ where a face is missing |
| Tables (editing, cell formats, across pages) | Built | Table pages and table editing runs | |
| Table formulas, Quick Add, number formats, Sort, merge numbers | Built, awaiting physical check | Formula audit: every calculated cell recomputes to Ami Pro's stored result, user's corpus 261 of 261 and ISO 15 of 15; twins ATD1-ATD5 | Paste and Cut into a formula cell are not refused yet; Sort lacks Ami Pro's "DDE" and "too large" refusals |
| Pictures (frames, crop, scale, link, paste, drop) | Built | Pictures run; twins | Formats: section 1 |
| Drawing (Draw mode, `.sdw`) | Built | Drawing run; unedited drawings are written byte for byte; Ami Pro reads and writes back edited ones unchanged (ADR3S) | |
| Charts | Built, awaiting physical check | 104 of 104 corpus charts and 18 of 18 VM charts write back equal; twin CHT1 opens in Ami Pro; the twelve chart types are drawn | Missing: the Pictures dialog, reordering colours, a live link to a table. VM check CHV4 not run. Perspective and Expanded pie, Colour set, and Axis were not covered in the live pass |
| Equations | Built, awaiting physical check | 27 of 27 VM equations read and written byte equal; twin EQT2 opens in Ami Pro | Layout is TeX's rules fitted to Ami Pro's sizes (mean error about 2%). About 330 of Ami Pro's 887 symbol names are known; an unknown control word is reported |
| OLE objects | Partial, awaiting physical check | 34 of 34 corpus OLE objects drawn from their presentation; bytes never changed; Convert to Picture works | Update, Unlink, Edit, Paste Link, and New Object are not offered (the servers are Ami Pro's); decided against |
| Image processing | Built, awaiting physical check | State words equal to the VM's 12 of 12 | The smoothing and edge filters are inferred, not copied from Ami Pro |
| Paste Special | Built, awaiting physical check | Scenarios; live pass 2026-11-07 | The toolkit cannot list the clipboard's types |
| Fields (dates, Doc Info, page numbers, Seq, power fields) | Built | Fields run; twins | |
| Notes, footnotes, endnotes, bookmarks | Built | Notes run; twins ANT1, ABK1, AFN1-AFN3 | |
| Proofing (spelling, grammar, thesaurus, languages) | Built | Automated and VM passes, physical pass 2026-11-03 (PR-01); twin APF1 | Spelling uses the system's hunspell dictionaries |
| TOC, index, glossary | Built | Twins open in Ami Pro; glossary file written byte for byte (GLO0) | |
| Master documents | Built | Master run twins; Ami Pro works the app's member files to the same numbers | |
| Merge, labels, envelopes | Built | Twins AMD1, AML1, AMLB open in Ami Pro; Ami Pro's merge equals the app's | Ami Pro's odd date parsing (MC4) and dBASE memo fields remain open |
| Outline mode and outline numbering | Built | Twin OBT1; physically checked 2026-11-06 | |
| Line numbering | Built | Commands run (CM1-CM12) | |
| Passwords | Built | Twins; Ami Pro's 7 protected files re-protect byte for byte | |
| Find and Replace | Built | Physically checked 2026-10-20 | |
| Print and PDF | Built, awaiting physical check | Print-pdf run | Printer Setup is decided against: the desktop's portal chooses the printer |
| Page Setup, Status Bar toggle, Fast Format, inserted rulers | Built | Live pass on a private display, 2026-11-07 | |
| Macros (`.SMM`, Record, Playback) | Deferred | `FEATURE_PARITY.md` MA-01 | No date set. A document that names a macro opens without running it |
| Revision marking, Revision Insertion, Doc Compare | Not planned | `docs/KNOWN_LIMITATIONS.md` | Revision data in a file is kept untouched |
| Window › Tile and Cascade | Decided against | Tabs replace them | |
| Window › New View and the Vertical Ruler | Not built yet | CK11 | |
| File Management, Notes/FX, mail, DDE | Not planned | | |
| Help (F1) | Not built yet | | Keyboard Shortcuts, Compatibility and Fidelity, Check for Updates, and About exist |

## 4. Platform

- Target: Pop!_OS with the COSMIC desktop, amd64, built with libcosmic. Other
  distributions build from source. Other desktops: not measured.
- Package: a `.deb` (`tools/package-deb.sh`, `docs/INSTALL.md`). It installs
  the binary, the desktop entry, the MIME definitions for `.sam` and `.sty`
  (it does not take over as default handler), and the icon; it contains no Ami
  Pro files. The package was checked off the target machine (contents,
  libraries, lintian); the install on the user's machine is pending.
- Licence: MPL-2.0; the repository is public. Release v0.1.0 is published.
- Updates: Help › Check for Updates reads GitHub Releases, verifies the
  package against `SHA256SUMS`, and installs after confirmation. The "up to
  date" result was seen live; the "newer version" path needs a later release
  and has not been seen.
- Crash recovery: documents with unsaved changes are copied to the app's data
  folder every 1, 2 (default), 5, or 10 minutes, never over the user's file,
  and offered after a crash (File › Recover Documents). Passed in the live pass
  of 2026-11-07 (kill -9, two instances, never-saved documents).
  Password-protected documents are not copied.
- Accessibility: the page surface exposes no accessibility nodes for page
  text, so a screen reader cannot read the document. This is blocked by the
  pinned toolkit.
- Input methods: preedit text is drawn at the caret and tested automatically
  but has not been checked with a real input method.
- Ctrl+Tab is taken by the desktop; Next Document is Ctrl+PageDown.

## 5. What the evidence is

Three layers are reported separately and none stands in for another:

1. Model and controller tests, with no window.
2. The real toolkit's events and rendering, driven headless.
3. The user's physical use on COSMIC (the only evidence for anything that
   depends on the compositor, a portal, or an input method).

Sources for Ami Pro's behaviour, in order: its decompiled code, the Ami Pro 3.1
virtual machine (the behavioural oracle), the manual and help text, then the
user's corpus.

- VM oracle runs: documents made by the app ("twins") are opened in Ami Pro
  3.1 in the VM, saved back, and compared; the areas above name their twins.
  Fixtures written by Ami Pro were read into the app to establish formats.
- Corpus numbers: quoted above from the recorded runs (`PROJECT_STATUS.md`,
  the `*_PROGRESS.md` files), at the dates given; not re-measured for this
  page. The user's corpus has 423 SAM files, 192 STY files, and Word Pro
  files; the ISO corpus has 160 SAM and 63 STY files. Layout comparison
  against Ami Pro's own caches measures agreement, not correctness.
- Scenario tests: whole workflows run end to end with a check after each step
  (`app/cosmic/src/scenario_tests/`: 202 `#[test]` functions in 26 modules,
  counted 2026-11-07). They pass before any physical check.
- Test counts at 2026-11-07, run with `AMIPRO_STYLES` unset: core 802
  (+30 ignored); app 714 (normal build), 128 (text editor), 721 (all
  features), with 85, 9, and 85 ignored. The ignored tests are twin, corpus,
  benchmark, and census runs that need the private files or the VM
  (`docs/TEST_COMMANDS.md`).
- Dropped by the user (2026-11-07): fuzzing and large-document tests. They have
  not been run.

How a state is decided (`AGENTS.md`):

- Built: the feature exists and its automated checks pass.
- Built, awaiting physical check: as above, and the user's desktop check
  (layer 3) is outstanding. The ranked list is in `docs/SESSION_HANDOFF.md`.
- Physical results are recorded as PASS, PARTIAL PASS, FAIL, or PENDING with
  what was observed; a partial pass is never recorded as a pass, and
  unreported checks are not assumed passed.
- Partial: some of the area is built; the gaps are named.
- Decided against, Deferred, Not planned: stated with the record; Deferred
  means a later run, with no date.
- "Checked" in sections 1 and 2 means measured against Ami Pro (its files or
  the VM), not only against the app's own tests.
