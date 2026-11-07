# Find & Replace Runbook

**Written:** 2026-10-20, the Phase 5 queue's run 3 (`docs/PHASE5_PLAYBOOK.md`,
`docs/playbooks/PROOFING.md`, "Run 1"). **Purpose:** carry Find & Replace
from start to finish as a straight run. Its decisions were asked before the
run and answered on 2026-10-19 (`docs/research/DECISIONS.md`, "The morning
report's answers": Find F1–F4, all as recommended); the run does not stop
to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §27 the interface. Evidence:
`docs/research/FIND_NOTES.md` (the decompiled `AMIPRO.EXE` and
`AMIPROUI.DLL`, the user's guide ch. 20, VM fixtures FR1–FR7).

**Live state** is `docs/FIND_PROGRESS.md`.

**Goal.** Edit › Find… (Ctrl+F) and Replace… (Ctrl+H) open a find bar over
the pages that finds text or a paragraph style through every text stream in
Ami Pro's order, with Ami Pro's options (Whole word only, Exact case, Exact
attributes, From the beginning, the other text streams), wildcards, codes,
and capitals rule; matches highlighted with a count; Replace, and Replace
All as one undo step; the options and recent searches remembered. The
app's Replace All gives what Ami Pro's gives on the same file.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/FIND_PROGRESS.md`,
   `docs/research/FIND_NOTES.md`, and `docs/Design/UI_SPEC.md` §27; then
   `git status` and `git log --oneline -15`.
2. The tag `find-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **Find F1** A find bar over the pages, under the ruler: Find, Replace,
  ▲ ▼, "3 of 12", Options ▾, Replace, Replace All; Ctrl+F, Ctrl+H, F3,
  Shift+F3, Enter, Shift+Enter, Esc.
- **Find F2** Ami Pro's defaults (Whole word only and the other text
  streams on), remembered between sessions.
- **Find F3** Ami Pro's capitals rule by default (the replacement follows
  the found text's capitals), with a switch for "as typed" (Ami Pro's
  Replace Exact case).
- **Find F4** Matches highlighted with a count, Replace All as one undo
  step, recent searches; no regular expressions yet.
- **Choices made in the run** are labelled FD1, FD2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** Save As SAM to new files only with the
  loss check (A, B); the page surface is the normal build (2026-10-12);
  the text-editor build keeps its commands (Find stays Future there).

## 3. Working rules

- The evidence first: a rule not in `FIND_NOTES.md` is mapped in the
  decompile or the VM before it is built, and recorded there.
- The pinned libcosmic's traps (`CLAUDE.md`): no zero-size children in a
  row or column (the find bar's slot is always present); an
  operation-only task (focus, scrolling) chains a repaint; never an id
  that changes at a fixed place; no `context_menu` inside a scrollable.
- Small green commits: targeted tests, strict Clippy, rustfmt, a commit
  per slice; native v3 and the SAM writer untouched (replacements are
  ordinary edits; nothing new to store).
- Aggregate numbers only, never the corpora's file names.
- **Text that moves is measured.** Find moves no text; Replace edits only
  what it is asked to. The round-trip audit and `layout_comparison_report`
  run at the start and at the exit and must be unchanged.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour, is recorded (FD…), and goes into the handoff.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| FD.1 Setup | This runbook, the progress file, the baseline, the tag | Committed; `find-run-start` |
| FD.2 The search | `amipro_editing::find`: Ami Pro's patterns (`?`, `*`, `<*>`, `<?>`, tab, return), Whole word only, case, the text streams in Ami Pro's order from the caret's stream, a selection, paragraph styles; each match's replacement (wildcards, Ami Pro's capitals rule); Replace All as one command | Rule tests from FR1–FR7 and the guide's examples |
| FD.3 Exact attributes | Find and replace Ami Pro's six attributes (Normal, Bold, Italic, Underline, Word underline, Small caps) | Rule tests |
| FD.4 Twins and the sweep | Ami Pro's input files from the VM run through the app's Replace All give Ami Pro's results (FR1A, FR1B, FR2, FR3, FR5A, FR6); every paragraph of both corpora is found in its stream | Ignored tests with `AMIPRO_VM_LAB`, `AMIPRO_CORPUS` |
| FD.5 The find bar | The commands on the page surface, the bar, its keys, the count, Replace, Replace All (one undo step, counts in the status bar), Options ▾, the settings, recent searches | Message-path and widget-path tests |
| FD.6 On the page | Matches highlighted on the pages in view, the current one selected and scrolled into view on its page | Widget-path tests; screenshots light and dark |
| FD.7 Timings | Time to the first match, Replace All, and its undo on README31 and the longest corpus file; the typing A/B against `find-run-start` | Release baselines; `tools/bench-ab.sh` |
| FD.8 Exit | `tools/verify.sh --release --corpus`, the round trip and layout comparison unchanged, the records, the handoff, the push | Green; `origin/master` equals `HEAD` |
