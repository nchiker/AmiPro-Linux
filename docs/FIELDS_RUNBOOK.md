# Fields Runbook

**Written:** 2026-10-20, the Phase 5 queue's run 4 (`docs/PHASE5_PLAYBOOK.md`,
`docs/playbooks/FIELDS_AND_REFERENCES.md`, "Run 1"). **Purpose:** carry
fields (dates and times, Doc Info, and power fields) from start to finish
as a straight run. Its decisions were asked before the run and answered on
2026-10-20 (`docs/research/DECISIONS.md`, "Fields: the user's answers",
FL1–FL7); the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §28 the interface. Evidence:
`docs/research/FIELDS_NOTES.md` (the decompiled `AMIPRO.EXE`,
`AMIPROUI.DLL`, and `AMIMACRO.DLL`; the user's guide ch. 5, 23, 32; VM
fixtures FD1–FD6).

**Live state** is `docs/FIELDS_PROGRESS.md`.

**Goal.** Dates, times, Doc Info fields, and power fields are typed parts
of the document that show their results as Ami Pro shows them, are
inserted and changed in a Fields panel in the rail, update by Ami Pro's
rule, and are written back as Ami Pro writes them. Document Properties in
the rail shows and edits Ami Pro's Doc Info, with live statistics. Ami
Pro's files show the same field text in the app as in Ami Pro, and the
app's files open in Ami Pro as its own.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/FIELDS_PROGRESS.md`,
   `docs/research/FIELDS_NOTES.md`, and `docs/Design/UI_SPEC.md` §28; then
   `git status` and `git log --oneline -15`.
2. The tag `fields-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **FL1** The app works out every power field that needs only the
  document (counts and Doc Info, Now, Seq, Set and names, IF, Defined,
  Void, User, arithmetic, comparisons, `{…}`, the `%D` `%T` `%N` formats,
  Ami Pro's text functions), **and Include and the Query$ and Decide
  prompts**. Merge fields show `<NAME>` and MergeRec 0; Bookmark, PageRef,
  and GetBookMark keep their result; Index and TOC stay marks; Message,
  Exec, Call, Button's action, and PrintEscape are never run.
- **FL2** A **Fields panel in the rail** inserts fields (Date and time,
  Document info, Power field) and changes the field at the caret live,
  each change one undo step; opened by Insert › Field…, Insert ›
  Date/Time…, a double-click on a field, and Edit Field….
- **FL3** **Document Properties in the rail** (File › Document
  Properties…, Alt+Enter): Ami Pro's Doc Info, live; statistics counted as
  you type and written at each save.
- **FL4** Ami Pro's updating rule: dates, times, and Doc Info fields as
  their lines are laid out; power fields when inserted, by Update (F9) and
  Update All (Ctrl+F9), and Auto run ones as the document opens.
- **FL5** Numeric dates and times as Ami Pro on US Windows (the reference
  VM): month/day/year, `/`, two-digit parts, 12-hour time with AM and PM.
- **FL6** View › Field Shading on (light grey, on screen only); View › Show
  Power Fields as Ami Pro (codes in red between `<` `>`).
- **FL7** A result is ordinary text until the next update; deleting a
  field asks nothing.
- **Fixed whatever the answers:** stored times in Ami Pro's time base (the
  wall clock read as US Pacific time, by the old US daylight rule of Ami
  Pro's runtime); editing minutes added at each save; `[desc]` values 2–5
  are Doc Info Fields 1–4; Ami Pro's "126" year and its Seq-and-Lock fault
  are not copied.
- **Choices made in the run** are labelled FX1, FX2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** Save As SAM to new files only with the
  loss check (A, B); the page surface is the normal build (2026-10-12);
  rail inspectors over modal dialogs; consistent conventions (check marks
  on every on/off command); the text-editor build keeps its commands (the
  field commands stay Future there).

## 3. Working rules

- The evidence first: a rule not in `FIELDS_NOTES.md` is mapped in the
  decompile or the VM before it is built, and recorded there.
- The pinned libcosmic's traps (`CLAUDE.md`): no zero-size children in a
  row or column; an operation-only task (focus, scrolling) chains a
  repaint; never an id that changes at a fixed place; no `context_menu`
  inside a scrollable; a text field focused by a task hears no focus
  message.
- Core crates stay dependency-free: the date formatter, Ami Pro's time
  base, and the field language are written here. The app takes the local
  wall clock from `jiff` (already in the build through libcosmic).
- Small green commits: targeted tests, strict Clippy, rustfmt, a commit
  per slice. Routine full suites, Clippy, fmt, and corpus measures run
  through lower-cost subagents that report back (the user, 2026-10-20).
- Native format: fields and Doc Info are new records of native v3, added
  as the bullets run added `inline bullet` (no version bump); every v3 file
  reads as before.
- Aggregate numbers only, never the corpora's file names.
- **Text that moves is measured.** Dates and Doc Info fields show text
  where nothing showed before, so lines move in about 45 files. The
  round-trip audit and `layout_comparison_report` run before and after
  that slice; every changed row is explained and should come closer to
  Ami Pro's cache (its caches were made with the dates shown).

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour, is recorded (FX…), and goes into the handoff. If the run grows
past one long session, hand off at a slice boundary with the progress
file current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| FL.1 Setup | This runbook, the progress file, the baseline (the Find & Replace exit's measures: no layout code changed since), the tag | Committed; `fields-run-start` |
| FL.2 Dates in the model | Ami Pro's date and time formatter (21 letters, the time, US settings), its time base both ways, `InlineObject` kinds for System date and time, Date last revised, Date created, Doc Info fields; SAM import and writer (`<:D` `<:b` `<:r` `<:k` `<:d`, byte for byte), native v3 records | Rule tests from FD1, FD2, FD4, FD5; round trip unchanged |
| FL.3 Dates on the page | Layout shows their text in the font where they stand, as one piece; the clock given by the app; layout comparison measured and explained | Layout tests; measures before and after |
| FL.4 Doc Info | `DocumentInfo` from `[desc]` and `[fldnames]` (values 2–5 as fields 1–4), written as Ami Pro writes it: Ami Pro's time base, revisions, editing minutes, statistics at save; the Doc Info fields' shown text | Tests from FD2, FD2B, FD5; round trip unchanged but the save's own values |
| FL.5 Statistics | Ami Pro's counting (every stream but notes; its word and character rules), pages from the layout, size at save | Rule tests; FD2's 6 and 23 |
| FL.6 Power fields in the model | Field records and marks (pairs by code, Seq's cache control, unpaired marks dropped on write, `<:M…>` as Ami Pro reads it), results as ordinary text, native v3 records, SAM writer | Round trip of every `<:X` in both corpora and the FD files |
| FL.7 The field language | Parser and evaluator: expressions, IF, names and Set variables, Defined, Void, User, `{…}`, the formats, text functions, Seq numbering by stream, counts and Doc Info, Now, Include (files beside the document, `%P`, `%W`), Query$ and Decide (a prompt the app answers), merge and bookmark fields as Ami Pro outside a merge | Twins: FD3 and FD6's stored results reproduced at their save times |
| FL.8 Updating | Commands: insert a field (evaluated at once), Update (the selection or the caret's field), Update All (Ami Pro's order, locked and mark kinds skipped, NumPages again after layout), Auto run on open, Lock and Auto run flags, each one undo step | Command tests |
| FL.9 On the page | Results editable as text (FL7), the caret and selection over marks, Seq numbers drawn, Field Shading, Show Power Fields (codes in red between `<` `>`, off before a save), the status bar's field name, the right-click menu, double-click | Widget-path and message-path tests; screenshots light and dark |
| FL.10 The Fields panel | The rail panel: Insert (Date and time, Document info, Power field with Ami Pro's list, options, syntax, saved fields), This field (live changes), the prompts' dialog | Message-path and widget-path tests; screenshots |
| FL.11 Document Properties | The rail panel: description, keywords, fields and their names, statistics, history, file; Tools › Word Count opens it | Tests; screenshots |
| FL.12 Commands | Insert › Field…, Insert › Date/Time…, Tools › Update Fields (F9), Update All Fields (Ctrl+F9), Edit › Next Field, Previous Field, View › Show Power Fields (Alt+F9), View › Field Shading, File › Document Properties… (Alt+Enter), Tools › Word Count; check marks; the registry's counts | Registry tests |
| FL.13 Twins | The app writes every field kind and Doc Info; Ami Pro opens them and shows the same text, Doc Info, and times | VM screenshots and Ami Pro's re-saves |
| FL.14 Timings | Typing in a document with fields, Update All, opening with Auto run; the typing A/B against `fields-run-start` | Release baselines; `tools/bench-ab.sh` |
| FL.15 Exit | `tools/verify.sh --release --corpus`, the measures explained, the records (`UI_SPEC.md` §28 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `SAM_FORMAT_NOTES.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue), the handoff, the push | Green; `origin/master` equals `HEAD` |
