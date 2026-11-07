# Phase 5 Playbook

**Written:** 2026-10-18, after the frames in table cells run, at the
user's request: "a playbook for Tables (in text and in frames), graphics,
drawing, charts, equations, references, indexes/TOC, fields, mail merge,
templates, language tools and other verified features, built on the Phase
2 model; group these how it makes logical sense".

**Purpose.** Turn Phase 5 into a queue of straight runs, grouped by what
they build on, so that every session can set up the next run from one
prompt (at the end of this file) and run it through without stopping.

**Authority.** `AGENTS.md` governs how to work. `PROJECT_PLAN.md` defines
scope, and the user may reorder the queue at any time. This playbook adds
the grouping, the order, and the recipe. Once a run's runbook is written,
that runbook governs the run.

## The playbooks

The features fall into seven groups. Each group shares one foundation, so
its runs build on one another. Each playbook lists its runs, the evidence
to map, the likely decisions with a draft recommendation, and the exit
criteria.

| Playbook | Covers | Shared foundation |
|---|---|---|
| [Graphics](playbooks/GRAPHICS.md) | pictures (Insert › Image…, decoding, Graphics Scaling), drawing (Draw mode), charts, equations, image processing, OLE objects | picture frames (`[isd]`, `[Embedded]`, Ami Pro's rendering); one vector and bitmap drawing path shared by the screen, PDF, and export |
| [Tables](playbooks/TABLES.md) | tables across pages, cell fidelity, creating and editing tables in text and in frames, lines and colour, formulas, Quick Add, Sort, forms | the table grid (`TableSpacing`, spans, cells as flows) and its layout |
| [Fields and references](playbooks/FIELDS_AND_REFERENCES.md) | date and time, Doc Info, power fields, bookmarks, notes, footnotes and endnotes, TOC, index, glossary, master documents, revision marking, Doc Compare | marks and fields as typed inline objects, shown as text computed at layout, and generation that walks the document's text streams in Ami Pro's order |
| [Proofing](playbooks/PROOFING.md) | Find & Replace, spell check (with as you type), thesaurus, grammar check, language | walking every text stream and changing it with one undo step |
| [Output and exchange](playbooks/OUTPUT.md) | print, PDF, export to DOCX or RTF and import from the same format | the shared layout output (pages, frames, display lists) and the model |
| [Merge and templates](playbooks/MERGE_AND_TEMPLATES.md) | mail merge (data files, conditions, envelopes, labels), Print Envelope, templates (File › New from style sheets with contents) | new documents made from a style sheet or a data file, through fields |
| [Views and commands](playbooks/VIEWS_AND_COMMANDS.md) | Outline mode, Draft mode, Facing Pages, View Preferences, and the remaining verified commands (Indention…, Spacing…, Fast Format, inserted rulers, Line Numbering, Revert to Saved, password-protected files, preferences) | the existing model; no new storage |

## The queue

This order is a recommendation. It puts the P0 items (tables, Find &
Replace) and what is visible in the user's files ahead of the rest, and it
builds each foundation before the runs that use it: fields come before
merge and TOC, print comes before envelopes and labels, and drawing comes
before charts. Each run is about one session. Small runs (marked *) can be
pulled forward between large ones.

| # | Run (tag `NAME-run-start`) | Playbook | Builds on | Why here | State |
|---|---|---|---|---|---|
| 1 | `pictures`: Insert › Image…, decoding, Graphics Scaling, pictures into empty cells (F4, T3) | Graphics | frames, cells | every picture in the files is a grey box today: about 1,000 pictures in both corpora (965 / 36) | DONE 2026-10-18 (tag `pictures-run-start`; `docs/PICTURES_PROGRESS.md`) |
| 2 | `table-pages`: tables across pages, cell text height | Tables | the grid (cells run) | P0 layout fidelity: about 80 of 220 corpus tables continue on later pages (re-measure in the run) | DONE 2026-10-19, overnight (tag `table-pages-run-start`; `docs/TABLE_PAGES_PROGRESS.md`; TP1–TP5 confirmed 2026-10-19) |
| 3 | `find`*: Find & Replace | Proofing | text streams | P0 (`ED-02`), expected of any editor | DONE 2026-10-20 (tag `find-run-start`; `docs/FIND_PROGRESS.md`; F1–F4 approved 2026-10-19; physical check passed 2026-10-20) |
| 4 | `fields`: date and time, Doc Info, power fields, Show Power Fields | Fields and references | inline objects | `<:D…>` date fields in 43 user files showed nothing | DONE 2026-10-20 (tag `fields-run-start`; `docs/FIELDS_PROGRESS.md`; FL1–FL7 answered 2026-10-20; physical check passed 2026-10-20) |
| 5 | `print-pdf`: print and PDF | Output | layout, pictures | getting documents out (`IO-03`) | DONE 2026-10-20 (tag `print-pdf-run-start`; `docs/PRINT_PROGRESS.md`; PO1–PO6 answered 2026-10-20; the physical check passed 2026-10-20) |
| 6 | `export`: DOCX, ODT, RTF, and text through one pipeline, a filter each (EX1) | Output | model, pictures | the user's archive in a modern format (user request) | DONE 2026-10-20 (tag `export-run-start`; `docs/EXPORT_PROGRESS.md`; EX1–EX10 answered 2026-10-20; table lines and shading drawn and exported too; physical check passed 2026-10-24) |
| 7 | `import`: the export's formats read in (DOCX, ODT, RTF, text; Open, and Insert › File… as Ami Pro's Import Text, whose Import Options are Convert, Apply, Keep, or Ignore styles) | Output | run 6 | user decision, 2026-10-18: whatever is chosen for export is also imported | DONE 2026-10-24 (tag `import-run-start`; `docs/IMPORT_PROGRESS.md`; IM1–IM12 answered 2026-10-21; Import Folder…, the Import Report, Paste of RTF, EMF pictures too; physical check passed 2026-10-25) |
| 8 | `table-editing`: create and edit tables, lines and colour | Tables | run 2 | P0 (`TB-01`): no table command yet | DONE 2026-10-24 (tag `table-edit-run-start`; `docs/TABLE_EDIT_PROGRESS.md`; TD1–TD11 answered 2026-10-24; physical check passed 2026-10-25) |
| 9 | `drawing`: drawings as objects, Draw mode | Graphics | run 1 | the most common picture kind: 380 drawings in 107 user files | DONE 2026-10-25 (tag `drawing-run-start`; `docs/DRAWING_PROGRESS.md`; DW1–DW9 answered 2026-10-25; physical check passed 2026-10-26) |
| 10 | `notes`: notes, bookmarks, footnotes, endnotes | Fields and references | run 4 | bookmarks in 33 user files, notes in 7; footnotes need a page area | DONE 2026-10-26 (tag `notes-run-start`; `docs/NOTES_PROGRESS.md`; NR1–NR10 answered 2026-10-26; physical check passed 2026-10-26) |
| 11 | `proofing`: spelling, thesaurus, grammar, language | Proofing | run 3 | `PR-01` | DONE 2026-10-26 (tag `proofing-run-start`; `docs/PROOFING_PROGRESS.md`; SP1–SP8 answered 2026-10-26; physical check passed 2026-11-03) |
| 12 | `toc-index`: TOC, index, glossary | Fields and references | runs 4, 10 | TOC options in 53 user files; 295 index entries in 11 files of both corpora | DONE 2026-11-03 (tag `toc-index-run-start`; `docs/TOC_INDEX_PROGRESS.md`; TX1–TX8 answered 2026-11-03; physical check passed 2026-11-03) |
| 13 | `master`: master documents | Fields and references | run 12 | master documents in 50 user and 56 ISO files (the user's guide is one) | DONE 2026-11-03 (tag `master-run-start`; `docs/MASTER_PROGRESS.md`; MS1–MS11 answered 2026-11-03; MQ1–MQ6; physical check passed 2026-11-03) |
| 14 | `merge`: merge, envelopes, labels | Merge and templates | runs 4, 5 | merge data files named in 33 files of both corpora | DONE 2026-11-03 for its automated parts (tag `merge-run-start`; `docs/MERGE_PROGRESS.md`; MG1–MG11 answered 2026-11-03; MC1–MC13; physical check passed 2026-11-04, after the rework) |
| A | `commands` (light; SU1): templates (15: File › New from style sheets, With contents, Run macro), the remaining commands (16: text, style, page, file), and views (21: Draft mode, Facing Pages, View Preferences, Clean Screen) | Merge and templates; Views and commands | STY writer | 255 style sheets in both corpora; small, basic gaps | DONE 2026-11-05 (`docs/COMMANDS_PROGRESS.md`; New View and the Vertical Ruler open, CK11) |
| B | `outline-mode` (light; 17): Outline mode | Views and commands | outline levels run | PROJECT_PLAN Phase 5 item | IN PROGRESS from 2026-11-05: OB.1–OB.2 done (`docs/OUTLINE_MODE_RUNBOOK.md`, `docs/OUTLINE_MODE_PROGRESS.md`) |
| C | `table-data` (full; 20): formulas, Quick Add, forms; numbers in table cells merged as numbers (MERGE_NOTES §13) | Tables | run 8 | | DONE 2026-11-06 (tag `table-data-start`; `docs/TABLE_DATA_PROGRESS.md`; TN1–TN11 answered 2026-11-06; physical check pending) |
| D | `objects` (full; SU2: full editors): charts (18), equations (19), image processing and Paste Special (22); OLE objects drawn from their renderings and kept | Graphics | runs 1, 8, 9 | charts 107 in 34 user files (7 distinct data records), OLE objects 34 in 17 files, equations 0, `.tgf` images 0 (corrected 2026-11-06; the earlier "4 and 4" were table-cell hints) | DONE 2026-11-07 for its automated parts (`docs/OBJECTS_PROGRESS.md`; OD1-OD12 answered 2026-11-06; physical check pending) |

Phase 5's exit (`PROJECT_PLAN.md`) is met when each verified feature has
evidence and passing acceptance checks: one row per feature in
`FEATURE_PARITY.md`, PASS or listed as not planned below.

## What Ami Pro 3.1 has (the verified inventory)

Sources: the menus in `AMIPROUI.DLL` (`tools/ne_resources.py … menus`:
the main menus 1002 and 1003, the Table menu 1057, the Outline menu 1058;
the Tools menu's `ISD PlaceHolder` (0x1a9) is where the Drawing, Charting,
Equations, and Image Processing modules add their own items); the 3.1
user's guide on the ISO (`AP3CH01`–`AP3CH32.SAM`); the install's modules
(`../AmiPro-Reference/Binaries`); and the two corpora, counted on
2026-10-18 (user's corpus / ISO; the files' names stay out of Git).

| Feature | Ami Pro (menu, guide chapter) | Modules to decompile | In the files | Playbook |
|---|---|---|---|---|
| Pictures | File › Import Picture… (0x8d), Frame › Graphics Scaling… (0x193); ch. 15, 19 | `AMIPRO.EXE`; the graphics filters (`pcx.flt`, `tiff.flt`, `cgm.flt`, `eps.flt`, `hpgl.flt`, `wpg.flt`, `dxf.flt`, `pic.flt`, …), `amiwmf.dll` | `[isd]` in 200 / 7 files; by source: drawings 380 / 5, BMP 283 / 18, WMF 239 / 13, OLE 34 / 0, TIFF 5 / 0, CGM 1 / 0, `.G0x` 23 / 0 (pictures) | Graphics |
| Drawing | Tools › Drawing (module); ch. 17, App. B | `amidraw.dll`, `drawman.dll` | `.SDW` drawings in 107 / 3 files | Graphics |
| Charts | Tools › Charting (module); ch. 18 | `amichart.dll`, `g4g*.dll` (to identify) | 107 charts (driver `.sct`) in 34 / 0 files; `[chint]` is a table-cell hint, not a chart. Built in run D (edited in the Charting panel) | Graphics |
| Equations | Tools › Equations (module); ch. 30 | `amieqn.dll` | none in either corpus (`[ehint]` only closes `[chint]`). Built in run D | Graphics |
| Image processing | Tools › Image Processing (module); ch. 19 | `amiimage.dll` | 5 TIFF pictures. Built in run D | Graphics |
| OLE and links | Edit › Paste Link, Paste Special…, Link Options…, Insert › New Object…; ch. 27 | `AMIPRO.EXE` | 34 OLE objects in 17 user files. Drawn and kept in run D | Graphics |
| Tables | Tools › Tables… (0x1c9); the Table menu (Modify Table Layout…, Lines & Color…, Insert and Delete Column/Row…, Delete Entire Table, Column/Row Size…, Select Column, Row, Entire Table, Connect Cells, Headings, Leaders ▸, Protect Cells, Edit Formula…, Quick Add ▸); ch. 16 | `AMIPRO.EXE` (grid `1428:0000`, `046b`; cells `1490:07c0`, `1428:07ce`) | 389 tables in 172 user files; 214 in 72 ISO files | Tables |
| Sort | Tools › Sort… (0x1c8); ch. 21 | `AMIPRO.EXE` | — | Tables |
| Date and time | Edit › Insert › Date/Time… (0x9e); ch. 5 | `AMIPRO.EXE` | `<:D…>` in 43 / 1 files | Fields and references |
| Doc Info | File › Doc Info… (0x6d), Edit › Insert › Doc Info Field… (0xb7); ch. 23 | `AMIPRO.EXE`, `AMIPROUI.DLL` | `[desc]` in every file; `[docvars]` 2 / 1 | Fields and references |
| Power fields | Edit › Power Fields ▸ (0x238, 0x23b, 0x10b, 0x239, 0x23a), View › Show Power Fields (0x10a); ch. 32 | `AMIPRO.EXE` | `<:X…>` 75 in 4 / 220 in 7 files (all index entries); other fields to find | Fields and references |
| Bookmarks | Edit › Bookmarks… (0x1d2); ch. 20 | `AMIPRO.EXE` (`1580:1368`) | `[book]` 33 / 1; `<:Z…>` 7 / 1 files | Fields and references |
| Notes | Edit › Insert › Note (0x11a); ch. 24 | `AMIPRO.EXE` | `<:N…>` 7 / 1 files | Fields and references |
| Footnotes, endnotes | Tools › Footnotes… (0xa0); ch. 25 | `AMIPRO.EXE` | `<:F…>` in 2 user files | Fields and references |
| TOC, index | Edit › Mark Text › TOC Entry… (0x343), Index Entry… (0xb4); Tools › TOC, Index… (0x1d1); ch. 25 | `AMIPRO.EXE` (`[toc]`: `11c8:1bbc`, `1588:261d`) | `[toc]` 53 / 3 files | Fields and references |
| Glossary | Edit › Insert › Glossary Record… (0x1d3, Ctrl+K), Mark Text › Glossary… (0xc7); ch. 25 | `AMIPRO.EXE` | — | Fields and references |
| Master documents | File › Master Document… (0x92), Save As Master (0x82); ch. 26 | `AMIPRO.EXE` (`1660:0000`, `0748`) | `[master]` 50 / 56, `[sequence]` 59 ISO files | Fields and references |
| Revision marking, Doc Compare | Tools › Revision Marking… (0x1cc), Doc Compare… (0x1f4), Mark Text › Revision Insertion (0xbb); ch. 24 | `AMIPRO.EXE` | `[revisions]` 0 in every file; `<:V…>` 1 ISO file | Fields and references |
| Find & Replace | Edit › Find & Replace… (0x1c3, Ctrl+F); ch. 20 | `AMIPRO.EXE` | — | Proofing |
| Spelling, grammar, thesaurus | Tools › Spell Check… (0x1c5), Grammar Check… (0x1fb), Thesaurus… (0x1c7); ch. 12, 25 | the proofing modules (to identify; their dictionaries are proprietary and never copied) | `[lang]` in every file; `[GramStyle]` set in 20 / 4 | Proofing |
| Print | File › Print… (0x6f), Printer Setup… (0x70); ch. 13 | `amiprint.exe`, `AMIPRO.EXE` | `[prn]`, `[port]` in every file | Output |
| Import and export filters | File › Import Text… (0x98), Save As formats; ch. 22 (Ami Pro imports WordPerfect, Word for Windows, Q&A Write, RTF, and text through filters) | `rtf.flt`, `w4w*.dll`, `wword*.flt` | — | Output |
| Merge | File › Merge… (0x30d), Edit › Insert › Merge Field… (0xb6); ch. 28 | `AMIPRO.EXE` | `[recfile]` 27 / 6 files; `<:M…>` 16 in 1 file | Merge and templates |
| Envelopes | File › Print Envelope… (0xed); ch. 13 | `AMIPRO.EXE` | — | Merge and templates |
| Templates | File › New… (0x65): style sheets, With contents, Run macro; ch. 4, 9 | `AMIPRO.EXE`, `AMIPROUI.DLL` | 192 user and 63 ISO style sheets | Merge and templates |
| Outline mode | View › Outline Mode (0xda); the Outline menu (Move Up/Down, Expand, Contract, Use Outline Styles); ch. 11 | `AMIPRO.EXE` | outline numbering in 263 files | Views and commands |
| Views | View › Full Page, Custom, Standard, Enlarged, Facing Pages, Layout/Draft Mode, Clean Screen, View Preferences…; ch. 10 | `AMIPRO.EXE`, `AMIPROUI.DLL` | — | Views and commands |
| Remaining commands | Text › Indention…, Spacing…, Fast Format; Style › Define Style, Select a Style; Page › Ruler ▸ Insert/Remove, Line Numbering…; File › Revert to Saved; password protection; Tools › User Setup…; Window › New Window, Tile, Cascade | `AMIPRO.EXE`, `AMIPROUI.DLL` | inserted rulers `<:R…>` in 37 / 4 files; `[lnopts]` in every file | Views and commands |

The meanings of the body controls in the table above (`<:D`, `<:X`,
`<:Z`, `<:V`, and the others the parser `11b8:030a` takes: `b`, `k`, `r`,
`T`, `e`, `d`, and `<:>`) are each run's first mapping step. The guesses
here (dates, index entries, bookmarks) come from the corpora's samples,
not from the decompile.

## Not planned (and why)

- **Revision marking and Doc Compare** (ch. 24; user decision SU3,
  2026-11-04): no file in either corpus has revision marking on. Files
  keep their `[revisions]` section and `<:V…>` marks untouched.

- **Macros** (ch. 31; `MA-01`, P2): deferred until separately scheduled
  (`PROJECT_PLAN.md`, Phase 6). Commands stay addressable through the
  registry for later automation. Style sheets that run a macro on File ›
  New are handled in the templates run (a decision there).
- **Mail, DDE links, Notes/FX** (ch. 27): Windows 3.1 services with no
  counterpart here. Files keep their data and write it back.
- **Live OLE editing**: an OLE object is drawn from its stored rendering
  and kept byte for byte (the objects run).
- **File Management** (ch. 29): the desktop's file manager does this.
- **SmartIcons sets**: the editable toolbar covers them.
- **The WordPerfect SwitchKit** (App. D).

## The recipe for every run

These are the steps of the ruler, frame creation, bullets, outline,
colour, and frames in table cells runs, written once.

1. **Start.** Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this playbook, and the run's playbook; then
   `git status` and `git log --oneline -15`. Take the queue's NEXT row
   unless the user names another run.
2. **Map in the decompile first** (`docs/research/DECOMPILATION.md`;
   `tools/ghidra/command_table.py` for menu handlers, `DecompAt.java` for
   addresses Ghidra missed; decompile any other module the run needs, as
   authorized). Find how Ami Pro stores, lays out, draws, and edits the
   feature, and the rules of its dialogs. Record the findings in your own
   words in a new `docs/research/NAME_NOTES.md`: routines by address,
   never decompiler output.
3. **Confirm in the XP VM** with fixtures made one property at a time
   (`docs/research/VM_LAB.md`, `tools/vm/`), saved in
   `../AmiPro-Reference/vm-lab/OUT`, with screenshots outside Git.
4. **Choose libraries** where the run needs one
   (`docs/REFERENCE_DRIVEN_DEVELOPMENT.md`): maintained, suitably licensed,
   offline, and pure Rust where possible. Record the reuse decision and add
   or update the run's row in `docs/research/REFERENCE_FEATURE_MATRIX.md`.
5. **Look for a better way** (standing rule below). For each Ami Pro
   item, ask whether a better way exists and which functions logically
   belong with it. Offer each improvement as a decision with a
   recommendation; never drop Ami Pro's own behaviour silently.
6. **Write the proposal** in a new section of `docs/Design/UI_SPEC.md`
   when the interface changes. Include a mockup in words or ASCII, how it
   fits the rail (`UI_SPEC.md` §15, §18), and what SAM can and cannot
   store.
7. **Ask every decision in one go**, before any implementation:
   AskUserQuestion with the recommendation first in each, several calls in
   a row if there are more than four. Spell out the options concretely.
   Record the answers in `docs/research/DECISIONS.md`, with a decision
   prefix not used before.
8. **Write the runbook and progress file** (`docs/NAME_RUNBOOK.md`,
   `docs/NAME_PROGRESS.md`; slices, gates, stop rules, the decisions).
   Make the setup commit and tag `NAME-run-start`; never move the tag.
9. **Measure the baseline**: the round-trip audit (`sam_round_trip`) on
   the user's corpus (with `AMIPRO_STYLES`), the ISO corpus, and the VM
   folder; and `layout_comparison_report` (`AMIPRO_CORPUS`,
   `AMIPRO_STYLES`, `AMIPRO_VM_LAB`). Measure again before and after any
   change that can move text, and explain every row that changes. Outputs
   go in `target/`, outside Git.
10. **Run it straight through** in small slices, each tested and
    committed: model and native format, import and the SAM writer (Save As
    lists nothing new), layout, the page surface, and the commands.
    Message-path and widget-path tests; screenshots in light and dark;
    release baselines and an interleaved A/B (`tools/bench-ab.sh`) for any
    path the run touches.
11. **Twins in the VM.** The app writes the run's files, and Ami Pro must
    open them as its own.
12. **Exit.** Run `tools/verify.sh --release --corpus`. Update the
    progress file, `UI_SPEC.md` (implementation notes), `FEATURE_PARITY.md`,
    `UI_ACCEPTANCE.md` (checklist and sign-off row),
    `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`,
    `PROJECT_STATUS.md`, `CLAUDE.md`, and this playbook's queue (DONE, the
    tag, the next row). Write `docs/SESSION_HANDOFF.md` with a short
    ranked list and a physical check of a few minutes. Push to `origin
    master` and confirm that `origin/master` equals `HEAD`.

## Standing rules

- **Improve where there is a better way** (user, 2026-10-18): "any item
  can [be] improved upon if there is a better way to do it or logical
  functions to be added". Every run looks for improvements and logical
  additions to each Ami Pro item, not only where Ami Pro is lacking
  (bullets, 2026-10-17). It offers them as decisions with a
  recommendation and maps them onto what SAM can store. What SAM cannot
  hold stays in the native format, and Save As lists it.
- **Fidelity where it matters.** Files Ami Pro wrote read back equal and
  lay out as Ami Pro lays them out; files the app writes open in Ami Pro
  as its own.
- **Interface conventions.** Non-modal rail inspectors over modal dialogs,
  with the document live. A convention used on one item goes on all its
  siblings (check marks, toggles).
- **Light user testing.** Ask the user only for desktop feel and
  decisions, ranked; verify Ami Pro fidelity yourself in the VM.
- **Private material.** Decompiler output, the corpora's files and file
  names, VM screenshots, and anything under `Reference/` never go into
  Git. Never copy Ami Pro's code, resources, artwork, dictionaries, or
  thesaurus. Run reference binaries only in the VM. Leave
  `AGENTS.md.backup` alone.
- **No stops mid-run.** Every decision is asked before the run starts. A
  question found later goes to the handoff, with the run's best choice
  made and recorded.

## Prompt for each Phase 5 session

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run as a straight
run by the playbook's recipe and its own playbook in docs/playbooks/: map
the feature in the decompile first and confirm it in the XP VM with
fixtures; choose maintained, suitably licensed libraries where the run
needs one; look for a better way and for logical functions to add to each
item; write the proposal in docs/Design/UI_SPEC.md if the interface
changes; ask me every decision the run needs in one go, with a
recommendation each, and record my answers in docs/research/DECISIONS.md;
then write the runbook and progress file, tag the start, and run it
straight through. Measure with the round-trip audit and
layout_comparison_report before and after any change that can move text,
keep decompiler output and my corpus file names out of Git, commit small
tested slices, update the playbook's queue, and hand off with a short
ranked list.
```

To run something other than the NEXT row, add "The run is NAME." to the
prompt.

## Keeping this playbook current

At each run's exit:
- Mark its row DONE with the date and the tag, and mark the next row
  NEXT.
- Add anything learned that changes a later run to that run's entry in
  its playbook.
- Don't reorder the queue without the user. A dependency found during a
  run goes into the handoff as a recommendation.
