# Tables Across Pages Runbook

**Written:** 2026-10-18, overnight, by the rulebook in
`docs/SESSION_HANDOFF.md` ("Overnight rulebook"). The Phase 5 queue's run 2
(`docs/PHASE5_PLAYBOOK.md`, `docs/playbooks/TABLES.md`, "Run 1").
**Purpose:** carry the tables across pages run from start to finish as a
straight run. Its decisions (TP1–TP5) were taken overnight with the
recommended choice and are recorded in `docs/research/DECISIONS.md` to
confirm; the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §26 the interface (with §11 and §24).
The rulebook adds its limits: 90 minutes per problem, measured before and
after anything that moves text, no destructive Git, no new dependencies,
push only at the end.

**Live state** is `docs/TABLE_PAGES_PROGRESS.md`.

**Goal.** Tables continue across columns and pages as Ami Pro continues
them: whole rows, or rows split between lines with Rows span pages; each
part with its top inset; Headings rows repeated at the top of later parts;
page breaks in rows; rows that keep their size without Automatic; rows cut
to a page; frame tables showing the rows that fit. Cell text measured as
Ami Pro measures it, so rows are as tall as Ami Pro's stored ones. Insert ›
Page Break in a cell sets a row's break. Ami Pro opens the app's files
with the tables where the app put them.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/TABLE_PAGES_PROGRESS.md`,
   `docs/research/TABLE_PAGES_NOTES.md`, and `docs/Design/UI_SPEC.md` §24
   and §26; then `git status` and `git log --oneline -15`.
2. The tag `table-pages-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions (taken overnight; to confirm)

- **TP1** Insert › Page Break in a cell toggles the row's page break
  (checked while set), one undo step.
- **TP2** Headings, Rows span pages, and Automatic are laid out, kept, and
  written; their commands wait for the table editing run.
- **TP3** A row taller than a page is cut to the page without Rows span
  pages.
- **TP4** The caret enters a repeated heading and edits the heading row.
- **TP5** A frame table hides the rows its frame cannot hold.
- **Earlier decisions that apply:** Save As SAM to new files only with the
  loss check (A, B); frames in cells (T1–T3); pictures (P1–P8).

## 3. Working rules

The outline run's rules apply (`docs/OUTLINE_RUNBOOK.md` §3): the evidence
first, findings into `TABLE_PAGES_NOTES.md`; the pinned libcosmic's traps
(`CLAUDE.md`); twin fixtures in the VM named `ATP…`; small green commits;
native v3 records for any model addition; aggregate numbers only, never
the corpora's file names.

**This run moves text.** Before and after every slice that can move text:
the round-trip audit (user's corpus with `AMIPRO_STYLES`, the ISO corpus,
the VM folder) and `layout_comparison_report` on both corpora and the VM
folder, into `target/tp-*` (outside Git). Every changed row is explained
in the progress log. A change that makes page ends or row heights worse
overall and cannot be fixed within its timebox is not merged (it goes to
a branch `overnight-wip-SLICE`).

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice.
Material decisions (anything SAM cannot hold, a change to the approved
interface, a dependency, anything hard to undo) are parked in the morning
report with a recommendation.

## 5. Slices, in order

| Slice | What | Gate |
|---|---|---|
| TP.1 | The map: the guide, the decompile, VM fixtures TP1–TP11; §26; TP1–TP5; this runbook; the baseline | Notes, fixtures, setup commit, tag |
| TP.2 | Model and native v3: `Table` gains Automatic, Rows span pages, heading rows, and page-break rows; validation; native records | Round trip of every new field; malformed input rejected |
| TP.3 | Import and the SAM writer: header value 7 bits `0x1` and `0x100`, `[h]` flags `0x10` and `0x20` into the model and back | The three folders read back equal and written as before; a changed flag written as Ami Pro writes it |
| TP.4 | Layout of rows: Automatic off keeps a row's size; a row at most a column tall; a frame table's rows clipped to its frame | Rule tests from TP2E, TP4, TP8; the comparison measured and explained |
| TP.5 | Tables across columns and pages: parts of whole rows with their inset, the first row's fit (V12), headings repeated with their extra inset, page-break rows, Rows span pages at lines, pictures in cells with their row, incremental layout | Rule tests from TP1–TP11; incremental equals full layout; the comparison measured and explained |
| TP.6 | Cell text measured as Ami Pro measures it (Table Text's spacing, the cell's line height, the margins) | Rows against Ami Pro's stored heights, measured and explained |
| TP.7 | The page surface: parts drawn (no grid line at a split), hit testing and the caret in every part and in repeated headings, Insert › Page Break in a cell (TP1) | Message-path and widget-path tests; screenshots light and dark |
| TP.8 | The comparison: each page-end table's first row and row count against Ami Pro's page table, row heights, both corpora | Numbers in the progress log and `LAYOUT_COMPARISON.md` |
| TP.9 | Twins in the VM (`ATP…`): Ami Pro opens the app's files with their tables where the app put them | Twins recorded in the notes |
| TP.10 | Both formats and performance: typing in a long table, a page break in a row, release baselines, the typing A/B | Timings in the progress log |
| TP.11 | Exit: `tools/verify.sh --release --corpus`, the records, the handoff | As the recipe's exit step |
