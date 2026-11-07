# Colour Selection Runbook

**Written:** 2026-10-18, after the user's answers to the colour selection
proposal (`docs/Design/UI_SPEC.md` §23; decisions C1–C4 in
`docs/research/DECISIONS.md`).
**Purpose:** carry the colour selection run from start to finish **as a
straight run**: every decision it needs was taken before it starts.

**Authority.** `AGENTS.md` governs how to work; `PROJECT_PLAN.md` scope,
`DECISIONS.md` the decisions, `UI_SPEC.md` §23 the interface (with §11's
Frame Properties, §13's Font dialog, §15's Style Properties, and §18's Page
Layout Properties). This runbook adds the order, gates, and stop rules.

**Live state** is `docs/COLOUR_PROGRESS.md`.

**Goal.** Every place a colour is chosen (the Font dialog; Style
Properties' text and rules; Page Layout Properties' lines; Frame
Properties' line, shadow, background, and line between columns) uses one
colour button that opens a colour panel in place: Ami Pro's 15 colours, In
this document, Recent, and More colours (libcosmic's picker); a colour
applies at once in the rail, a drag in the picker live with one undo step,
the Font dialog with OK; colours mixed in the app are written in both
formats and shown by Ami Pro (VM). Then the user's physical check.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`, this runbook,
   `docs/COLOUR_PROGRESS.md`, `docs/research/COLOUR_NOTES.md`, and
   `UI_SPEC.md` §11, §13, §15, §18, §23; then `git status` and
   `git log --oneline -15`.
2. If the tag `colour-run-start` does not exist, create it at HEAD; never
   move or delete it.
3. Resume at the first slice that is not `DONE`; finish uncommitted work,
   never discard it.

## 2. Decisions already taken (do not ask again)

- **C1:** one colour button (the colour and its name, or `#RRGGBB`) in
  every place; it opens a colour panel in place, non-modal; one panel open
  at a time.
- **C2/C4:** the panel's rows: Ami Pro's 15 colours; In this document (every
  kind together, most used first); Recent (one list for the app, kept in
  the settings, newest first, colours shown above left out); More colours
  (libcosmic's picker, `#RRGGBB` or RGB). No Ami Pro 120-colour grid.
- **C3:** in the rail a colour applies at once as one undo step; a drag in
  the picker shows the colour live and leaves one step when it ends; the
  Font dialog applies with OK.
- **All four frame colours**, as Ami Pro's Modify Frame Layout: line,
  shadow, background, and the line between columns, which gets a colour of
  its own.
- **Earlier decisions that apply:** rail inspectors over modal dialogs; a
  convention used on one item goes on all its siblings; Save As SAM to new
  files only with the loss check; settings in the user's COSMIC
  configuration (tests use none or a scratch directory).

## 3. Working rules

As the outline levels run (`docs/OUTLINE_RUNBOOK.md` §3): evidence first
(findings into `COLOUR_NOTES.md`); the pinned libcosmic's traps
(`CLAUDE.md`: no in-window menu inside a scrollable, `Fixed(0.0)` children
dropped, operation-only tasks need a repaint); twin fixtures named `CS…`;
the round-trip audit and the layout comparison before and after (colours
should move nothing); small green commits (`tools/verify.sh`); aggregate
numbers only; push only at the end.

## 4. Stop rules

There is no planned stop. Anything new goes in the progress file as
`WAITING FOR USER` with the conservative choice taken, asked at the end.

## 5. Slices, in order

### CS.1 The map

Done before the run: `COLOUR_NOTES.md`, §23, C1–C4.

### CS.2 The colour model

A shared module: `COLORREF` ↔ display colour and `#RRGGBB`; the colours a
document uses, counted, most used first; Recent (push, de-duplicate, cap,
leave out shown colours) kept in the settings. Pure tests.

### CS.3 The colour button and panel

The widget and its state (which colour the open panel is for, the picker's
model), messages, one panel at a time; tests through messages;
screenshots light and dark.

### CS.4 Style Properties, Page Layout Properties, and the Font dialog

The text and rule colours, the page's lines, and the Font dialog's colour
through the control; live drags as one undo step (`History::amend`); the
Font dialog applies with OK; tests.

### CS.5 Frame Properties

Line, shadow, and background as colour buttons (the `#RRGGBB` fields go);
the line between columns gets its own colour; written in both formats;
tests.

### CS.6 Twins

In the VM: files made in the app with mixed colours (text, a rule, a
frame's four colours, the page's lines) opened in Ami Pro and shown as the
app draws them (an ignored `colour_twins` test writes `ACS…`).

### CS.7 Both formats and performance

Mixed colours read back from `.amipro` and SAM with nothing in the loss
check; Recent survives a restart; the panel's cost (In this document is
computed only while a panel is open); typing A/B; final measures.

### CS.8 Exit

§23's implementation notes, `FEATURE_PARITY.md`, `UI_COMMAND_MATRIX.md` if
commands change, `UI_ACCEPTANCE.md` (a colour section), `PROJECT_PLAN.md`,
`PROJECT_STATUS.md`, `CLAUDE.md`, the handoff with a short ranked list.

## 6. Exit

Met when every colour in the app is chosen through the control as §23
describes, documents round-trip with mixed colours, Ami Pro shows them,
and the measures are unchanged. The user's physical check follows.
