# Outline Mode Runbook (run B)

**Written:** 2026-11-05, for the speed-up plan's run B (`docs/SPEEDUP_PLAN.md`;
`docs/PHASE5_PLAYBOOK.md`, row B; `docs/playbooks/VIEWS_AND_COMMANDS.md`,
run 2 `outline-mode`).

**Purpose:** View › Outline Mode as Ami Pro 3.1 has it, plus an Outline
navigator in the rail, as one straight run. The decisions were asked before
run A and answered on 2026-11-04 (`docs/research/DECISIONS.md`, OM1–OM4).
The run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work. `PROJECT_PLAN.md` defines
scope, `docs/research/DECISIONS.md` the decisions, and
`docs/Design/UI_SPEC.md` §40 the interface. Evidence:
`docs/research/OUTLINE_MODE_NOTES.md` (decompile and VM), with the levels
run's `docs/research/OUTLINE_NOTES.md`.

**Weight (OM4).** Light: spec, decisions, scenarios, one twin. Where a
rule is needed (what a section is, what the icons mean, Use Outline
Styles), look it up in the decompile first, briefly, and confirm in the VM.

**Live state** is in `docs/OUTLINE_MODE_PROGRESS.md`. The outline levels
run (2026-10-18) used `docs/OUTLINE_RUNBOOK.md`/`OUTLINE_PROGRESS.md` and
the tag `outline-run-start`; this run's tag is `outline-mode-start`.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`, this runbook,
   `docs/OUTLINE_MODE_PROGRESS.md`, `UI_SPEC.md` §40, and
   `OUTLINE_MODE_NOTES.md`; then `git status` and `git log --oneline -15`.
2. Never move the tag `outline-mode-start`.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **OM1** Ami Pro's Outline mode on the page surface (level icons, Expand
  and Contract, Show levels 1–n, Use Outline Styles, editing in place)
  plus an Outline navigator in the rail (the headings as a tree; a click
  goes to the heading, a drag moves a section, keys move in the tree).
- **OM2** Collapsing only in Outline mode and the navigator, per tab,
  never written; Layout mode always shows everything.
- **OM3** Move Up and Move Down (Alt+Up, Alt+Down) move the caret's heading
  with its section (a body paragraph alone) in Outline and Layout mode
  alike, also in Bullets and Numbering ▸; in a table they still move rows;
  frames anchored in the section move with it; one undo step. (Ami Pro
  moves an expanded heading alone and a contracted one with its hidden
  text, VM OM1; the decision's section move is kept: RB1.)
- **OM4** Light weight.
- **Choices made in the run** are RB1, RB2, … here, in the progress file,
  and in `DECISIONS.md`; each keeps Ami Pro's behaviour where it has one.

## 3. Working rules

As run A's (`docs/COMMANDS_RUNBOOK.md` §3): every document change is one
`amipro_editing::Command` and one undo step through the app's message
path, written as Ami Pro writes it; scenarios first (a module
`scenario_tests/outline_mode.rs`), written by a lower-cost agent from §40
and reviewed, passing before the physical check; tests never open real
dialogs or touch the user's settings; routine suites and measures through
lower-cost subagents; aggregate numbers only from the corpora.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on. A question found during the run takes
Ami Pro's behaviour, is recorded (RB…), and goes to the handoff. Hand off
at a slice boundary if the run outgrows a session.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| OB.1 Setup | This runbook, the progress file, `OUTLINE_MODE_NOTES.md` (decompile brief, VM fixtures OM1), the baseline (run A's exit measures), the tag | Committed; `outline-mode-start` |
| OB.2 Sections and Move Up/Down | The section rule (a heading and what follows until a heading of its level or higher; body text alone); `Command::MoveSection`; Alt+Up/Alt+Down in both modes and in Bullets and Numbering ▸ (rows in a table); anchored frames go along | Core tests; SAM round trip after moves; scenario |
| OB.3 Outline mode view | View › Outline Mode: the text at the window's width, indented by level, Ami Pro's icons (a heading with text under it, without; body text); editing in place | Widget tests (icons, indents, hit testing on an indented line); scenario; screenshots |
| OB.4 Expand, Contract, Show levels, the Outline bar | Per tab, never written (OM2): Contract hides a heading's section (the contracted icon), Expand shows it, Show levels n hides deeper headings and body text; the caret leaves hidden text; the Outline menu items and the bar shown in Outline mode (levels 1–9 and All, Promote, Demote, Move Up, Move Down, Expand, Contract) | Scenario (collapse kept per tab, nothing saved, Layout shows all); widget tests; screenshots |
| OB.5 Use Outline Styles | As the decompile says it works (OUTLINE_MODE_NOTES) | Tests; round trip if it is stored |
| OB.6 Outline navigator | The rail's tree of headings: click goes, drag moves a section (one step), keys move in the tree, collapse in step with Outline mode | Scenario; widget test for the drag; screenshots |
| OB.7 Measures and twin | Round trip and layout comparison against run A's exit; one twin (a document reorganised in Outline mode, opened in Ami Pro) | Numbers here |
| OB.8 Exit | `tools/verify.sh --release --corpus`; records; handoff; push | Green; `origin/master` equals `HEAD` |
