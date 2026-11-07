# Outline Levels Runbook

**Written:** 2026-10-17, after the user's answers to the outline levels
proposal (`docs/Design/UI_SPEC.md` §22; decisions O1–O4 in
`docs/research/DECISIONS.md`).
**Purpose:** let one session, possibly resumed after context compaction,
carry the outline levels run from start to finish **as a straight run**:
every decision it needs was taken before it starts, so it does not stop
in the middle to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions so far, `docs/Design/UI_SPEC.md` §22 the approved interface (with
§15's Style Properties and §21's bullets and numbering). This runbook adds
the order, the gates, and the stop rules.

**Live state** is `docs/OUTLINE_PROGRESS.md`. Keep it current: it is how a
resumed session knows where it is.

**Goal.** A paragraph moves between outline levels with Format › Promote
and Demote (Alt+Left, Alt+Right, and toolbar buttons); a numbered list
nests as 1. → A. → i.; outline levels, reset options, and cumulative
numbers are set in Style Properties and whole schemes in Format › Styles ▸
Outline Numbering…; numbers are counted as Ami Pro counts them; the Format
menu is folded into submenus; everything is written in both formats and
opened by Ami Pro in the VM as Ami Pro's own twin is. Then the user's
physical check.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/OUTLINE_PROGRESS.md`,
   `docs/research/OUTLINE_NOTES.md`, `docs/research/BULLETS_NOTES.md`, and
   `docs/Design/UI_SPEC.md` §12, §15, §16, §21, and §22; then `git status`
   and `git log --oneline -15`.
2. If the tag `outline-run-start` does not exist, create it at HEAD. It is
   the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE`.
   Uncommitted changes belong to an interrupted run: inspect, verify, and
   finish them; never discard them.

## 2. Decisions already taken (do not ask again)

- **O1:** Format › Promote (Alt+Left) and Demote (Alt+Right) in the normal
  page view, on the caret's or the selected paragraphs; Tab stays a tab;
  with a frame selected Alt+arrows keep resizing it.
- **O2:** demoting an item of a list made with the Numbering button makes
  a multi-level list: depth 1 `1.`, 2 `A.`, 3 `i.`, 4 `a.`, 5 `1)`, 6
  `a)`, 7 `i)`, 8 `(1)`, 9 `(a)`; each depth's style ("List 1"…) made in
  the document as needed, a further 0.25 in per depth, Right align, After
  higher level; the list's depths take the outline levels below the
  deepest level the document's styles use; the whole unbroken run of the
  list's paragraphs moves into the levels when one item is demoted. Other
  paragraphs promote and demote by Ami Pro's rule (OL1): the style one
  level up or down (one already used in the document first), None
  promoted to the previous levelled paragraph's level and demoted one
  below it, Outline# made (a copy at the level, next free function key)
  when none exists; level 1 does not promote and 9 does not demote.
- **O3:** Style Properties' Bullets and numbers gains Outline level, the
  Reset options (exclusive pair, and Cumulative numbering, which builds
  the text from each level's number and separator); Format › Styles ▸
  Outline Numbering… applies 1/A/i, I/A/1/i/a, 1/1.1/1.1.1, or Clear to
  the document's levelled styles only (not, as Ami Pro's buttons do, to
  styles without a level).
- **O4:** Promote and Demote on the Standard toolbar after Numbering, with
  the theme's `format-indent-less-symbolic` and `format-indent-more-symbolic`.
- **The Format menu** in submenus: Alignment ▸, Bullets and Numbering ▸
  (with Promote and Demote), Frame ▸.
- **Earlier decisions that apply:** B1–B5 (§21); rail inspectors over
  modal dialogs, small dialogs for one-shot actions; a convention used on
  one item goes on all its siblings; Save As SAM to new files only with
  the loss check; a created style takes the next free function key.
- **Already authorized** (`AGENTS.md`): decompiling any binary as needed,
  operating Ami Pro in the XP VM (including snapshots), writing `.SAM`
  files under Phase 4's decision A.

## 3. Working rules

The bullets run's rules apply unchanged (`docs/BULLETS_RUNBOOK.md` §3):
the disassemblies first, findings into `OUTLINE_NOTES.md`; the approved
interface; the pinned libcosmic's traps (`CLAUDE.md`); twin fixtures in the
VM named `OL…`; measure before and after (the session's `measure.sh`, or
the round-trip audit and `layout_comparison_report` on the user's corpus
with `AMIPRO_STYLES`, the ISO corpus, and the VM folder) for any change
that can move text or change written files; timings of the new commands
and typing A/B; small green commits (`tools/verify.sh`); native v3 records
for any model addition; aggregate numbers only, never the user's file
names; push only at the end of the session.

## 4. Stop rules

There is no planned stop. Stop only for what `AGENTS.md` lists. Anything
new goes in the progress file as `WAITING FOR USER` with the conservative
choice taken, and is asked at the end, batched and ranked. Known already:
whether bullets should nest (Bullet 1 → Bullet 2) as numbered lists do.

## 5. Slices, in order

### OL.1 The map (evidence, no product code)

Done before the run was set up: `OUTLINE_NOTES.md` (reset bits
`12c8:060c`, counting `1200:1842`, cumulative codes `1200:1384`, Quick
Numbering `12c8:06d6`/`0829`/`0755`, Outline mode's Promote and Demote and
Outline# from OL1, cumulative display from OL2), §22, O1–O4.

### OL.2 Counting as Ami Pro counts

`amipro-layout` `bullets.rs`: per-level counters with the three reset
rules, the flags of the style at each level, cumulative codes counting
levels 1, 2, 3… and the hidden full stop after the last level's code.
Tests from BN2 (unchanged numbers), OL2 (1, 1.1, 1.2, 2, 2.1), and a
style with neither option and one with After intervening style. Layout
comparison before and after (number widths can move text: explain).

### OL.3 Promote and Demote in the model

`amipro-editing` `outline.rs`: the style one level up or down for each
paragraph by O2's two rules (lists and Ami Pro's), Outline# and List n
made as needed, one `Command::ChangeLevel` per gesture; tests from OL1
and nested lists (restarts under each item, the top restarting after a
heading); written as Ami Pro writes made styles.

### OL.4 The commands and the Format menu

Format › Promote and Demote (Alt+Left, Alt+Right; frames keep their
Alt+arrows), toolbar buttons, the Format menu's submenus (Alignment ▸,
Bullets and Numbering ▸, Frame ▸) with check marks kept; tests through
messages and the window; screenshots light and dark.

### OL.5 Style Properties

Outline level, the Reset options, Cumulative numbering in Bullets and
numbers (each change one `ModifyStyle`); tests; screenshots.

### OL.6 Outline Numbering…

The small dialog under Format › Styles ▸ with the four choices, one undo
step, levelled styles only; tests; screenshots.

### OL.7 Twins

In the VM: OL1 (Demote and Promote making Outline3) and OL2 (1, 1.1,
1.1.1) made in the app (an ignored `outline_twins` test), written as Ami
Pro wrote them (body and the changed `[tag]` records); Ami Pro shows the
app's twins as its own; a nested list made in the app opened in Ami Pro
and its numbers compared.

### OL.8 Both formats and performance

Documents with levels, nested lists, and cumulative numbers read back
from `.amipro` and SAM with nothing in the loss check; timings of Promote,
Demote, and a scheme; typing A/B; the final measures.

### OL.9 Exit

§22's implementation notes, `FEATURE_PARITY.md`, `UI_COMMAND_MATRIX.md`,
`UI_ACCEPTANCE.md` (an outline section), `PROJECT_PLAN.md`,
`PROJECT_STATUS.md`, `CLAUDE.md`, the handoff with a short ranked list (the
physical check with exact steps, the bullets-nesting question).

## 6. Exit

Met when Promote, Demote, nested lists, the Style Properties fields, and
Outline Numbering… work as §22 describes under O1–O4, numbers count as Ami
Pro's do, the Format menu is folded, documents round-trip through both
formats with nothing in the loss check, and Ami Pro shows the app's twins
as its own. The user's physical check follows.
