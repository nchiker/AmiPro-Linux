# Merge Runbook

**Written:** 2026-11-03, for the Phase 5 queue's run 14
(`docs/PHASE5_PLAYBOOK.md`; `docs/playbooks/MERGE_AND_TEMPLATES.md`, "Run
1: `merge`").

**Purpose:** carry Ami Pro's merge from start to finish as a straight run:
data files of every kind, merge fields, conditions, the preview, every
output, the record editor, labels, envelopes, and File › New's merge
sheets. The decisions were asked before the run and answered on
2026-11-03 (`docs/research/DECISIONS.md`, "Merge, labels, envelopes: the
user's answers", MG1–MG11). The run does not stop to ask.

**Authority.**
- `AGENTS.md` governs how to work; nothing here overrides it.
- `PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
  decisions, and `docs/Design/UI_SPEC.md` §38 the interface.
- The evidence is in `docs/research/MERGE_NOTES.md`: the decompiled
  `AMIPRO.EXE` and `AMIPROUI.DLL`, the guide, the shipped macros' text,
  and Ami Pro 3.1 fixtures MGM1–MGM8.

**Live state** is in `docs/MERGE_PROGRESS.md`.

**Goal.**
- `[recfile]` typed in the model, native v3, and SAM, written as Ami Pro
  writes it; conditions in native v3.
- Every data kind read in the core (Ami Pro's data document, a table
  document, DIF, fixed-width, comma, CSV/TSV, DBF; XLSX and ODS in
  `amipro-exchange`).
- The merge in the core: MergeField, MergeRec, NextRec, Skip, Defined by
  Ami Pro's rules; conditions; empty fields (MG9); Ami Pro's merged file.
- The Merge panel (MG1, MG5, MG6), every output (MG4), the record editor
  (MG3), labels (MG7), envelopes (MG8), the merge sheets (MG11).
- Every change is one undo step. Ami Pro reads the app's files as its own
  (twins).

## 1. At the start of every session

1. Read the following, then run `git status` and
   `git log --oneline -15`:
   - `AGENTS.md`, `CLAUDE.md`, and `docs/SESSION_HANDOFF.md`
   - `PROJECT_PLAN.md`, this runbook, and `docs/MERGE_PROGRESS.md`
   - `docs/research/MERGE_NOTES.md`
   - `docs/Design/UI_SPEC.md` §38
2. The tag `merge-run-start` marks the start. Never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, and finish them. Never
   discard them.

## 2. Decisions

- **MG1** A Merge panel in the rail (data file, fields, record browser,
  conditions, labels, Merge to); File › Merge… and Edit › Insert › Merge
  Field… open it.
- **MG2** Ami Pro's data kinds, CSV/TSV, DBF read natively, XLSX and ODS
  (first sheet, first row as names).
- **MG3** A record editor grid; saved in Ami Pro's delimited form; New
  Data File… asks for field names; external files read only.
- **MG4** Merge & print, Merge, view & print, Merge & save as, plus New
  Document and PDF (one, or one per record named from a field).
- **MG5** Preview with a record, display only.
- **MG6** Ami Pro's conditions exactly, any number of rows, a live count;
  native only.
- **MG7** Labels from the app's own sheet list (Avery's published sizes)
  plus Custom; set up as Ami Pro does; N copies; a start position.
- **MG8** A native Envelope panel: Ami Pro's sizes, saved return
  addresses, the recipient found, Print, PDF, New Document, merged
  envelopes, feed and offsets.
- **MG9** Ami Pro's empty-field rule plus tidied spaces and commas, in
  merged output only.
- **MG10** MergeField, MergeRec, NextRec, Skip, Defined by Ami Pro's
  rules, in merges and the preview.
- **MG11** `_MERGLET.STY`, `_LABEL.STY`, `_ENVELOP.STY`, `_AMIENV.STY`
  open the panels in place of their macros.
- **Choices made in the run** are MC1, MC2, … in the progress file and
  `DECISIONS.md`; each keeps Ami Pro's behaviour where Ami Pro has one.
- **Earlier decisions that apply:** rail inspectors; consistent
  conventions; FL1–FL7 (fields); PO1–PO6 (print); MS2 (a missing file
  looked up by name); SAM sources protected; Save As SAM lists what Ami
  Pro cannot hold.

## 3. Working rules

- **The core stays dependency-free.** `[recfile]`, the data readers
  (but XLSX and ODS, which go in `amipro-exchange` over its own zip and
  XML readers, inflated by the caller), conditions, and the merge live in
  `amipro-document` and `amipro-editing`. The app supplies files,
  threads, pages, and the print portal.
- **Ami Pro's bytes are the oracle.** Files Ami Pro wrote read back equal;
  MGM1–MGM8's merged text is reproduced from their inputs.
- **Every change is one `amipro_editing::Command`**, tested through the
  app's message path and the widget path.
- **The evidence comes first.** A rule not in the notes is looked up in
  the decompile, or made as a VM fixture, before it is built.
- **The pinned libcosmic's traps (`CLAUDE.md`) apply.** Tests click portal
  commands (Open…, PDF…, Printer…, Ami Pro File…) with
  `Runtime::click_unrun`, so no real dialog opens on the user's screen,
  and never touch the user's settings.
- **Small green commits.** Routine suites, Clippy, fmt, and corpus
  measures go through lower-cost subagents that report back.
- **Aggregate numbers only.** Never the corpora's file names or contents.
- **Text that moves is measured** with `target/merge-baseline/baseline.sh`
  at the start, at the exit, and around any slice that touches the model,
  SAM, or layout.

## 4. Stop rules

None are planned.
- A problem gets 90 minutes. Then it is recorded as a blocker in the
  progress file, and the run moves on to the next independent slice.
- A question found during the run takes the choice that keeps Ami Pro's
  behaviour. It is recorded (MC…) and goes into the handoff.
- If the run grows past one long session, hand off at a slice boundary
  with the progress file current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| ME.1 Setup | VM fixtures MGM1–MGM8, the libraries, `UI_SPEC.md` §38, this runbook, the progress file, the baseline, the tag | Committed; `merge-run-start` |
| ME.2 Model and SAM | `RecordFile` (`[recfile]`) typed in the model, native v3, and SAM; merge conditions in native v3; `Command::SetRecordFile`, `SetMergeConditions` | Round trip equal to the start; Save As lists conditions only |
| ME.3 Data in the core | `amipro_editing::merge_data`: `DataTable`, field keys, Ami Pro's data document, a table document, description files (DIF, FIXED, COMMA), CSV/TSV, DBF; the delimited writer | MGD1 and the shipped data file read; synthetic files of each kind; written data read back |
| ME.4 Spreadsheets | `amipro_exchange::sheets`: XLSX and ODS first sheets | Synthetic files of both; limits |
| ME.5 The merge in the core | `amipro_editing::merge`: the record in the field language (`FieldHost`), MergeField, MergeRec, NextRec, Skip, Defined; conditions (`Condition`, Ami Pro's numbers and dates); `merge_record`, `merge_all`, empty fields (MG9); Ami Pro's merged file | MGM1–MGM7 reproduced from their inputs |
| ME.6 The Merge panel | File › Merge…, Edit › Insert › Merge Field…, the panel: data file (Open…, a job off the interface's thread), fields, the preview (display only), conditions with the live count | Message-path and widget-path tests; screenshots light and dark |
| ME.7 Outputs | New Document, PDF (one, per record), Printer…, View & Print, Ami Pro File… | Tests with `click_unrun`; the PDF's pages |
| ME.8 The record editor | Edit records… (the grid in the data tab), New Data File…; each edit one step | Tests; Ami Pro reads the written data file |
| ME.9 Labels | The sheet list, Merge as labels, `[recfile]`'s label values, sheet placement, copies, start label | Positions as Ami Pro's formula; tests |
| ME.10 Envelopes | File › Print Envelope…, the Envelope panel, sizes, return addresses, outputs, merged envelopes | Tests; screenshots |
| ME.11 Merge sheets | MG11's sheets open the panels | Tests |
| ME.12 Measures | Round trip and layout comparison against the start; release timings (a 1,000-record merge to a new document and a PDF; a keystroke with the preview on); the typing A/B | Numbers in the progress file |
| ME.13 Twins | The app's data file, merge letter, and label document by Save As SAM; Ami Pro merges them to the app's results | VM screenshots outside Git |
| ME.14 Exit | `tools/verify.sh --release --corpus` and the records (`UI_SPEC.md` §38 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue); the handoff and the push | Green; `origin/master` equals `HEAD` |
