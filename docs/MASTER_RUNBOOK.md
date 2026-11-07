# Master Documents Runbook

**Written:** 2026-11-03, for the Phase 5 queue's run 13
(`docs/PHASE5_PLAYBOOK.md`; `docs/playbooks/FIELDS_AND_REFERENCES.md`,
"Run 4: `master`").

**Purpose:** carry Ami Pro's master documents from start to finish as a
straight run: the file list, numbers that run on across the files, the TOC
and index over the set, printing, PDF, and export of the set, and Find
across the set. The run's decisions were asked before it began and
answered on 2026-11-03 (`docs/research/DECISIONS.md`, "Master documents:
the user's answers", MS1–MS11). The run does not stop to ask.

**Authority.**
- `AGENTS.md` governs how to work; nothing here overrides it.
- `PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
  decisions, and `docs/Design/UI_SPEC.md` §37 the interface.
- The evidence is in `docs/research/MASTER_NOTES.md`: the decompiled
  `AMIPRO.EXE` and `AMIPROUI.DLL`, the guide's chapter 26, and Ami Pro 3.1
  fixtures MD1–MD3.

**Live state** is in `docs/MASTER_PROGRESS.md`.

**Goal.**
- `[master]`'s file list and `[sequence]` are typed in the model, native
  v3, and SAM, written as Ami Pro writes them.
- A member's numbers (pages, footnotes, numbered styles, Seq) continue
  from its `[sequence]`, as Ami Pro shows them.
- The app works the set in memory while a master is open (MS3) and writes
  a member's `[sequence]` when it is saved.
- The Master Document panel (MS1, MS9), Treat as Single Document, Open
  Master (MS4).
- TOC and index over the set into new tabs, linked to their files (MS5).
- Print and PDF of the set (MS6); export of the set (MS10); Find across
  the set (MS11).
- Every change is one undo step. Ami Pro opens the app's files as its own
  (twins).

## 1. At the start of every session

1. Read the following, then run `git status` and
   `git log --oneline -15`:
   - `AGENTS.md`, `CLAUDE.md`, and `docs/SESSION_HANDOFF.md`
   - `PROJECT_PLAN.md`, this runbook, and `docs/MASTER_PROGRESS.md`
   - `docs/research/MASTER_NOTES.md`
   - `docs/Design/UI_SPEC.md` §37
2. The tag `master-run-start` marks the start. Never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, and finish them. Never
   discard them.

## 2. Decisions

- **MS1** A Master Document panel in the rail, opened by File › Master
  Document…; on a member, where its numbers start and Treat as Single
  Document.
- **MS2** The stored path, else the file of that name without case in the
  master's folder; new members stored as full paths.
- **MS3** Numbers live in the app while a master is open; member files are
  never overwritten; a member's `[sequence]` is written when it is saved.
- **MS4** A member opened alone shows its stored numbers and offers Open
  Master when a master in its folder lists it.
- **MS5** TOC and index over the set into new tabs; saved paths become the
  master's output files; entries link to their file and place.
- **MS6** The Print panel: the whole master document (default; Ami Pro's
  order) or this document; one PDF with a bookmark per file.
- **MS7** No book view.
- **MS8** Members are SAM and `.amipro` documents.
- **MS9** Checks for files whose numbering would be wrong in the set.
- **MS10** File › Export ▸ on a master: the whole set as one DOCX, ODT, or
  RTF, each file a section.
- **MS11** The find bar's In All Files of the Master; Replace All one undo
  step in each file it changes.
- **Choices made in the run** are MQ1, MQ2, … in the progress file and
  `DECISIONS.md`; each keeps Ami Pro's behaviour where Ami Pro has one.
- **Earlier decisions that apply:** rail inspectors; consistent
  conventions; TX1–TX8 (contents); PO1–PO6 (print); EX1–EX10 (export);
  F1–F4 and FD1–FD5 (find); SAM sources protected; Save As SAM lists what
  Ami Pro cannot hold.

## 3. Working rules

- **The core stays dependency-free.** `[sequence]` (`Sequence`), the
  typed file list, the carry rules (what a member passes on), and path
  resolution live in `amipro-document` and `amipro-editing`; the layout
  applies a sequence. The app supplies files, threads, and pages.
- **Ami Pro's bytes are the oracle.** Files Ami Pro wrote read back equal;
  MD2's and MD3's `[sequence]` sections are reproduced from their members;
  their members lay out with Ami Pro's numbers.
- **Every change is one `amipro_editing::Command`**, tested through the
  app's message path and the widget path.
- **The evidence comes first.** A rule not in the notes is looked up in
  the decompile, or made as a VM fixture, before it is built.
- **The pinned libcosmic's traps (`CLAUDE.md`) apply.** Tests click portal
  commands (Add…, Locate…, Print…, Export…) with `Runtime::click_unrun`,
  so no real dialog opens on the user's screen, and never touch the user's
  settings.
- **Small green commits.** Routine suites, Clippy, fmt, and corpus
  measures go through lower-cost subagents that report back.
- **Aggregate numbers only.** Never the corpora's file names or contents.
- **Text that moves is measured** with `target/master-baseline/baseline.sh`
  at the start, at the exit, and around any slice that touches the model,
  SAM, or layout.

## 4. Stop rules

None are planned.
- A problem gets 90 minutes. Then it is recorded as a blocker in the
  progress file, and the run moves on to the next independent slice.
- A question found during the run takes the choice that keeps Ami Pro's
  behaviour. It is recorded (MQ…) and goes into the handoff.
- If the run grows past one long session, hand off at a slice boundary
  with the progress file current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| MR.1 Setup | `MASTER_NOTES.md`, `UI_SPEC.md` §37, MS1–MS11, this runbook, the progress file, the baseline, the tag | Committed; `master-run-start` |
| MR.2 Model and SAM | `Sequence` (`[sequence]`: page base, footnotes, the reserved word, Seq and style counters) in the model, native v3, and SAM; `[master]`'s files typed with the byte count computed; `Command::SetMasterFiles`, `SetSequence`, `TreatAsSingle` | Round trip equal to the start; Save As lists nothing new |
| MR.3 Layout carry | Shown page numbers add the page base; the footnote numbers start after F; numbered styles and Seq fields continue from the carried counters | MD2 and MD3 members lay out with Ami Pro's numbers; layout comparison before and after |
| MR.4 The set in the core | `amipro_editing::master`: what a member passes on (`next_sequence`: last shown page, footnotes, style counters at the end, Seq maxima), path resolution (MS2), member checks (MS9) | MD2's and MD3's `[sequence]` reproduced from their members |
| MR.5 The set in the app | The set job (members read and laid out off the interface's thread, carried in order), live numbers in members' tabs, `[sequence]` written on save, the folder lookup (MS4) | Message-path tests; an edit in a member updates the later ones |
| MR.6 Master Document panel | File › Master Document…, the panel (files, Add…, Remove, ↑ ↓ drag, Open, Open All, Locate…, outputs, separators, checks), Treat as Single Document, Open Master | Message-path and widget-path tests; screenshots light and dark |
| MR.7 TOC and index over the set | Generate Contents and Generate Index on a master: every member with its shown pages, into new tabs; saved paths become `[master]`'s names; Ctrl+click opens the file at the heading | MD3's TOC and index reproduced |
| MR.8 Print and PDF of the set | The Print panel's whole set or this document; Ami Pro's order; one PDF with a bookmark per file | Tests with `click_unrun`; a set PDF's pages |
| MR.9 Export of the set | File › Export ▸ on a master: one DOCX, ODT, or RTF, a section per file | The exports read back (the import) with every member's text |
| MR.10 Find across the set | The find bar's In All Files of the Master; matches by file; Replace All one step per file | Tests |
| MR.11 Measures | Round trip and layout comparison against the start; release timings (the set job on the ISO's guide chapters, a keystroke in a member with the master open); the typing A/B | Numbers in the progress file |
| MR.12 Twins | The app makes a master and its members' `[sequence]` and writes SAM; Ami Pro opens them as the app shows them and works the set (TOC, index) to the same numbers | VM screenshots outside Git |
| MR.13 Exit | `tools/verify.sh --release --corpus` and the records (`UI_SPEC.md` §37 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue); the handoff and the push | Green; `origin/master` equals `HEAD` |
