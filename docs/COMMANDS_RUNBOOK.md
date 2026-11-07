# Commands Runbook (run A)

**Written:** 2026-11-04, for the speed-up plan's run A (`docs/SPEEDUP_PLAN.md`;
`docs/PHASE5_PLAYBOOK.md`, row A; `docs/playbooks/MERGE_AND_TEMPLATES.md`
(templates) and `docs/playbooks/VIEWS_AND_COMMANDS.md` (runs 1 and 3)).

**Purpose:** Ami Pro's remaining verified commands, templates, and views as
one straight run. The decisions were asked before the run, together with
run B's, and answered on 2026-11-04 (`docs/research/DECISIONS.md`, "Runs
A and B: the user's answers", CM1–CM12, OM4). The run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work. `PROJECT_PLAN.md` defines
scope, `docs/research/DECISIONS.md` the decisions, and
`docs/Design/UI_SPEC.md` §39 the interface.

**Weight (OM4).** Light: spec, decisions, scenarios, one twin. Full
(decompile mapping in `docs/research/COMMANDS_NOTES.md`, VM fixtures one
property at a time, twins) for line numbering (CM7) and password-protected
files (CM5), which write SAM structures the app has not written before.
Where a light item still needs a rule (a dialog's limits, Fast Format's
options, Draft mode's drawing), look it up in the decompiled
`AMIPRO.EXE`/`AMIPROUI.DLL` first, briefly, and note it.

**Live state** is in `docs/COMMANDS_PROGRESS.md`.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`, this runbook,
   `docs/COMMANDS_PROGRESS.md`, and `UI_SPEC.md` §39; then `git status`
   and `git log --oneline -15`.
2. The tag `commands-run-start` marks the start. Never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **CM1** File › New… dialog (style sheets by description or name, recent
  first, With contents, a preview); Ctrl+N stays blank.
- **CM2** Run macro: noted in the status bar, kept as read, not run.
- **CM3** Paragraph Properties in the rail (Indention, Spacing, alignment,
  breaks; Revert to style); Indention…/Spacing… open its sections.
- **CM4** Fast Format as Ami Pro's, plus a toolbar paintbrush toggle.
- **CM5** Password-protected files opened and saved (open only if the
  scheme cannot be written).
- **CM6** One Preferences dialog (General, Folders, View, Toolbar).
- **CM7** Line Numbering in Page Layout Properties, drawn, printed,
  written to `[lnopts]`.
- **CM8** Continuous pages + Facing Pages; zoom presets Full Page (Ctrl+D),
  Standard, Enlarged, Custom.
- **CM9** Draft mode at the window's width; Show Clean Screen (= Full
  Screen).
- **CM10** View › Show ▸ marks as checks (Tabs & Returns on the toolbar
  too), the Preferences View page; the view kept per tab.
- **CM11** Window › New View: a second tab on one session.
- **CM12** Special Character…, Keyboard Shortcuts, Compatibility and
  Fidelity, the status bar's Face and Point Size buttons.
- **Choices made in the run** are CK1, CK2, … here, in the progress file,
  and in `DECISIONS.md`; each keeps Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** rail inspectors; consistent
  conventions (check marks on every on/off command); SU4 (typing runs);
  SAM sources written over with a `.bak`; Save As lists what Ami Pro
  cannot hold.

## 3. Working rules

- Every document change is one `amipro_editing::Command` and one undo
  step, tested through the app's message path; written as Ami Pro writes
  it (round trip unchanged unless the slice is meant to change it).
- Scenarios first for each slice's user task, in
  `app/cosmic/src/scenario_tests/` (a module `commands.rs`, `views.rs`, and
  `templates.rs`), written by a lower-cost agent from §39 and reviewed;
  they pass before the physical check.
- Tests never open real dialogs (`Runtime::click_unrun`) and never touch
  the user's settings.
- Routine suites, Clippy, fmt, corpus measures, and decompile sweeps go
  through lower-cost subagents that report back (the user, 2026-11-04).
- Aggregate numbers only; never the corpora's file names or contents.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on. A question found during the run takes
Ami Pro's behaviour, is recorded (CK…), and goes to the handoff. Hand off
at a slice boundary if the run outgrows a session.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| CA.1 Setup | `UI_SPEC.md` §39, this runbook, the progress file, the decisions, the baseline (round trip, layout comparison), the tag | Committed; `commands-run-start` |
| CA.2 Help additions | Help › Keyboard Shortcuts (from the registry), Compatibility and Fidelity (the loss list without saving) | Tests; screenshots |
| CA.3 Preferences | Preferences… (Ctrl+,): General, Folders (the style-sheet folders moved in), View, Toolbar | Settings round trip in a scratch directory |
| CA.4 File › New… | The dialog, the preview, With contents, Run macro noted (CM1, CM2); a toolbar item | Scenarios: a memo from a sheet with contents, saved as SAM and reopened |
| CA.5 Paragraph Properties | The rail panel, `SetParagraphFormat` (or the existing indents command widened), Indention…, Spacing…, Revert to style | Round trip of `<:I…>`/`<:S…>`; scenarios |
| CA.6 Fast Format | Ctrl+T, the paintbrush toggle, Ami Pro's options | Scenarios (each application one step) |
| CA.7 Style and Edit items | Define Style, Select a Style (Ctrl+Y), Mark Text ▸ Protected Text and No Hyphenation, Revert to Saved | Scenarios |
| CA.8 Page items | Ruler ▸ Insert/Remove, Insert Page Layout ▸ Insert/Remove/Revert where missing | Round trip; scenarios |
| CA.9 Status bar and characters | Face and Point Size buttons, Insert › Special Character… | Scenarios |
| CA.10 Zoom and Facing Pages | Presets (Ctrl+D), Facing Pages | Widget tests (pages placed in pairs); screenshots |
| CA.11 Draft and Clean Screen | Draft mode, Show Clean Screen | Widget tests; typing benchmark in Draft |
| CA.12 Show marks, views per tab, New View | View › Show ▸, the toolbar toggle, per-tab view, Window › New View | Tests: two tabs on one session stay in step, Undo shared |
| CA.13 Line numbering (full) | Map `[lnopts]` and the drawing; VM fixtures; the model, native, SAM; Page Layout Properties section; screen, PDF | Ami Pro's fixtures read back equal; the twin opens as made |
| CA.14 Passwords (full) | Map `[encrypt]`; VM fixtures; open (asking) and Save As with Password… | The corpus's protected file opens; Ami Pro opens the app's protected twin |
| CA.15 Measures and twin | Round trip and layout comparison against CA.1; release typing A/B; one twin of a document using the run's commands | Numbers here |
| CA.16 Exit | `tools/verify.sh --release --corpus`; the run's records (progress, §39 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, the playbook's row, one line in `PROJECT_STATUS.md` and `PROJECT_PLAN.md`, `CLAUDE.md` if its state changes); the handoff; the push | Green; `origin/master` equals `HEAD` |
