# TOC, Index, and Glossary Runbook

**Written:** 2026-11-03, for the Phase 5 queue's run 12
(`docs/PHASE5_PLAYBOOK.md`; `docs/playbooks/FIELDS_AND_REFERENCES.md`,
"Run 3: `toc-index`").

**Purpose:** carry marking, generating, and updating a table of contents
and an index, and Ami Pro's glossary, from start to finish as a straight
run. The run's decisions were asked before it began and answered on
2026-11-03 (`docs/research/DECISIONS.md`, "TOC, index, glossary: the
user's answers", TX1–TX8). The run does not stop to ask.

**Authority.**
- `AGENTS.md` governs how to work; nothing here overrides it.
- `PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
  decisions, and `docs/Design/UI_SPEC.md` §36 the interface.
- The evidence is in `docs/research/TOC_INDEX_NOTES.md`: the decompiled
  `AMIPRO.EXE` and `AMIPROUI.DLL`, and Ami Pro 3.1 fixtures TI1, TI2, and
  GLO0.

**Live state** is in `docs/TOC_INDEX_PROGRESS.md`.

**Goal.**
- Tools › TOC, Index… and Edit › Mark Text ▸ Index Entry… and TOC Entry…
  work in one Contents & Index panel in the rail.
- Index and TOC marks are typed. `[toc]` and `[master]` are typed.
- A TOC is generated into the document's start, at the caret, or into a
  new document, and updated in place.
- An index is generated into a new tab or at the document's end.
- The glossary's commands and panel work on Ami Pro's data files.
- Every change is one undo step.
- Ami Pro opens the app's files as its own (twins).

## 1. At the start of every session

1. Read the following, then run `git status` and
   `git log --oneline -15`:
   - `AGENTS.md`, `CLAUDE.md`, and `docs/SESSION_HANDOFF.md`
   - `PROJECT_PLAN.md`, this runbook, and `docs/TOC_INDEX_PROGRESS.md`
   - `docs/research/TOC_INDEX_NOTES.md`
   - `docs/Design/UI_SPEC.md` §36
2. The tag `toc-index-run-start` marks the start. Never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, and finish them. Never
   discard them.

## 2. Decisions

- **TX1 One rail panel.** The Contents & Index panel has three pages:
  - Contents: levels, page number, right align, and the leader.
  - Index: separators, ranges, and where the index goes.
  - Entries: Mark Index Entry live, with the list of entries.
- **TX2 Where the TOC goes.** The start of this document (the default),
  at the caret, or a new document on `_TOC.STY`. Update replaces it in
  place.
- **TX3 Where the index goes.** A new untitled tab on `_INDEX.STY`; the
  source is neither saved nor closed. Optionally, the end of this
  document after a page break.
- **TX4 Mark All.** Whole words, Exact case on by default. It is one undo
  step and skips places that already hold the entry.
- **TX5 Page ranges.** An option, off by default, kept in the settings.
- **TX6 TOC links.** Ctrl+click on the page goes to the heading, and the
  PDF links to it. Nothing extra is stored.
- **TX7 Glossary.** Ami Pro's commands work as Ami Pro's. A Glossary
  panel in the rail adds Insert, Add from selection, Rename, and Delete.
  Complete as you type is an option, off by default.
- **TX8 Writing glossary data files.** The app writes them as Ami Pro
  does:
  - It appends records, then the index footer.
  - It writes through a temporary file and a rename.
  - It changes only a valid glossary file.
- **Choices made in the run** are labelled TQ1, TQ2, … in the progress
  file and `DECISIONS.md`. Each one is Ami Pro's behaviour where Ami Pro
  has one.
- **Earlier decisions that apply:**
  - Rail inspectors over modal dialogs.
  - Consistent conventions.
  - Power fields as typed marks (FL1–FL7, FX…).
  - Find's streams.
  - SAM sources protected.
  - Save As SAM lists what Ami Pro cannot hold.

## 3. Working rules

- **The core stays dependency-free.** These live in `amipro-document` and
  `amipro-editing`:
  - Marks, settings, the walk, entries, sorting (Ami Pro's sort table,
    written independently from its documented weights), page strings, the
    TOC table, and the index paragraphs.
  - The glossary file's format.
  The app supplies the pages, through the layout.
- **Ami Pro's bytes are the oracle.**
  - Files Ami Pro wrote must read back equal (the round trip).
  - `[toc]`, `[master]`, marks, the TOC page table, and glossary files are
    written as Ami Pro writes them.
  - TI1's and TI2's outputs must be reproduced from their inputs.
- **Every change is one `amipro_editing::Command`**, applied to a new
  root. It is tested through the app's message path and the widget path.
- **The evidence comes first.** A rule that is not in the notes is looked
  up in the decompile, or made as a VM fixture, before it is built.
- **The pinned libcosmic's traps (`CLAUDE.md`) apply.**
  - Tests click portal commands with `Runtime::click_unrun`, so no real
    dialog opens on the user's screen.
  - Tests never touch the user's settings or glossary (a scratch folder).
- **Small green commits.** Routine suites, Clippy, fmt, and corpus
  measures go through lower-cost subagents that report back.
- **Aggregate numbers only.** Never the corpora's file names or contents.
- **Text that moves is measured** with `target/toc-baseline/baseline.sh`:
  - At the start and at the exit.
  - Around any slice that touches the model, SAM, or layout. The TOC page
    break's numbering can move text.

## 4. Stop rules

None are planned.
- A problem gets 90 minutes. Then it is recorded as a blocker in the
  progress file, and the run moves on to the next independent slice.
- A question found during the run takes the choice that keeps Ami Pro's
  behaviour. It is recorded (TQ…) and goes into the handoff.
- If the run grows past one long session, hand off at a slice boundary
  with the progress file current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| TC.1 Setup | `TOC_INDEX_NOTES.md`, `UI_SPEC.md` §36, TX1–TX8, this runbook, the progress file, the baseline, the tag | Committed; `toc-index-run-start` |
| TC.2 Model and SAM | Index and TOC marks typed over the power-field code (parse, build, Ami Pro's form); `[toc]` (`TocSettings`) and `[master]` (`MasterSettings`) typed in the model, native v3, and SAM; the TOC page table recognised by its frame flag | Round trip equal to the start; Save As lists nothing new |
| TC.3 TOC in the core | `amipro_editing::contents`: TOC levels from outline levels, collected entries (main text, anchored text frames, TOC marks, numbers, attributes), page strings from the pages as shown, the two-column page table (leaders, separators), TOC1–9 and TOCPG1–9 made, `Command` to put or replace it (start, caret, new document) | TI1's and TI2's tables reproduced from their inputs |
| TC.4 TOC page numbering | The TOC's page break (flag 0x02) keeps later page numbers; confirmed with a VM fixture (a page-number footer) | Fixture and test; layout comparison before and after |
| TC.5 Index in the core | Ami Pro's sort, merge, pages, Other, separators, ranges option; the index document on `_INDEX.STY` or paragraphs at the end; marks' page values set | TI1IDX reproduced from TI1 |
| TC.6 Contents & Index panel | Rail panel (Contents, Index pages), Tools › TOC, Index…, Generate, Update, places, the settings; TOC links (Ctrl+click, PDF links) | Message-path and widget-path tests; screenshots light and dark |
| TC.7 Entries | The Entries page: Mark Index Entry live (Primary and Secondary lists, Page number, Other, example), Mark, Remove, Next/Prev Mark and Same, Mark All, the entry list; TOC Entry; Edit › Mark Text ▸ items | Tests; each one undo step |
| TC.8 Glossary core | The data file (merge format, delimiters, `[gloidx]`), records as SAM body text, append with the footer, Ctrl+K's word | Tests against GLO0's bytes |
| TC.9 Glossary in the app | Edit › Mark Text ▸ Glossary…, Edit › Insert ▸ Glossary Record… (Ctrl+K), Data File…, the Glossary panel, complete as you type | Tests; screenshots |
| TC.10 Measures | Round trip and layout comparison against the start; release timings (TOC and index of the longest corpus file, a Mark All); the typing A/B | Numbers in the progress file |
| TC.11 Twins | The app marks, generates, and writes SAM (a TOC at the start, an index file, a glossary file with a new record); Ami Pro opens them as the app shows them and regenerates the TOC in place | VM screenshots outside Git |
| TC.12 Exit | `tools/verify.sh --release --corpus` and the records (`UI_SPEC.md` §36 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue); then the handoff and the push | Green; `origin/master` equals `HEAD` |
