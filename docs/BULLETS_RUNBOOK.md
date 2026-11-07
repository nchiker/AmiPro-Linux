# Bullets and Numbering Runbook

**Written:** 2026-10-17, after the user's answers to the bullets and
numbering proposal (`docs/Design/UI_SPEC.md` §21; decisions B1–B5 in
`docs/research/DECISIONS.md`).
**Purpose:** let one session, possibly resumed after context compaction,
carry the plan section "Bullets and numbering commands"
(`PROJECT_PLAN.md`) from start to finish **as a straight run**: every
decision it needs was taken before it starts, so it does not stop in the
middle to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions so far, `docs/Design/UI_SPEC.md` §21 the approved interface
(with §13's character commands, §15's Style Properties, and §16's
right-click menus). This runbook adds the order, the gates, and the stop
rules.

**Live state** is `docs/BULLETS_PROGRESS.md`. Keep it current: it is how a
resumed session knows where it is.

**Goal.** Lists are made in the app as the user decided: Format › Bullets
(Ctrl+Shift+L) and Format › Numbering (Ctrl+Shift+N), toggles on the
toolbar and in the right-click menu, apply Ami Pro's list styles (made in
the document from `_DEFAULT.STY` when it has none) and return to Body
Text; Enter in an empty list paragraph ends the list; Insert › Bullet…
puts Ami Pro's bullets into the text; Format › Bullet and Number Style…
opens Style Properties at its Bullets and numbers section, which gains
Space for. Inserted bullets, Right align, superscript bullet text, and
the seven picture bullets are drawn as Ami Pro draws them. Everything is
written in both formats and opened by Ami Pro in the VM as Ami Pro's own
twin is. Then the user's physical check.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/BULLETS_PROGRESS.md`,
   `docs/research/BULLETS_NOTES.md`, the bullets part of
   `docs/research/SAM_FORMAT_NOTES.md` ("Bullets and numbers (`[spec]`)"),
   and `docs/Design/UI_SPEC.md` §12, §13, §15, §16, and §21; then
   `git status` and `git log --oneline -15`.
2. If the tag `bullets-run-start` does not exist, create it at HEAD. It is
   the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE`.
   Uncommitted changes belong to an interrupted run: inspect, verify, and
   finish them; never discard them.

## 2. Decisions already taken (do not ask again)

- **B1:** Format › Bullets and Format › Numbering apply Ami Pro's list
  styles: the document's Bullet 1, else Bullet, and its Number List. When
  the document has none, the command creates it in the document as
  `_DEFAULT.STY` defines it (Bullet 1: All, First, Rest 0.20 in, a small
  diamond; Number List: 0.25 in, `1.`, Right align), in the document's
  Body Text font, spacing, and lines, and applies it, as one undo step.
  Off (every selected paragraph already shows one) returns them to Body
  Text; the other kind switches.
- **B2:** both on the Standard toolbar as toggles after the alignment
  group, with icons of the app's own (the COSMIC theme has no list icons);
  Numbering gets Ctrl+Shift+N, Bullets keeps Ctrl+Shift+L.
- **B3:** Enter in an empty bullet or number paragraph ends the list: the
  paragraph returns to Body Text; elsewhere Enter keeps Ami Pro's Next
  style.
- **B4:** Insert › Bullet…: a small one-shot dialog with Ami Pro's 17
  bullets drawn as buttons in Ami Pro's order and an example in the
  caret's font, Insert and Cancel, remembering the last bullet (in the
  settings); the bullet replaces a selection, one undo step; offered in
  Customize Toolbar (not in the Standard set).
- **B5:** Format › Bullet and Number Style… (also in the text right-click
  menu) opens Style Properties at its Bullets and numbers section for the
  caret paragraph's style.
- **Within the run, without a question:** inserted bullets imported,
  drawn, edited as one character, and written back; Right align,
  superscript bullet text, and the picture bullets drawn as Ami Pro draws
  them (the pictures as shapes of the app's own, never Ami Pro's
  metafiles); Space for in Style Properties (the First indent); check
  marks on both toggles, following the caret (§13's "what you see").
- **Earlier decisions that apply:** rail inspectors over modal dialogs,
  small dialogs for one-shot actions; a convention used on one item goes
  on all its siblings; the toolbar and its settings (§12); Save As SAM
  writes new files only (Phase 4 decision A) with the loss check (B); one
  length rule for every panel (`lengths.rs`); a style created in a
  document is stored there (Phase 3.4).
- **Already authorized** (`AGENTS.md`): decompiling any binary as needed,
  operating Ami Pro in the XP VM (including snapshots), and writing
  `.SAM` files under decision A.

## 3. Working rules

The frame creation run's rules apply unchanged
(`docs/FRAME_CREATION_RUNBOOK.md` §3, which points to the ruler's and the
page layout setup's): the disassemblies are authoritative for what Ami Pro
does (the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL` in
`../AmiPro-Reference/ghidra-work/out/`; findings into `BULLETS_NOTES.md`
as VERIFIED (decompile), the implementation written independently); the
approved interface decides how it looks and works (deviations from Ami
Pro recorded in §21's implementation notes); the pinned libcosmic's traps
(`CLAUDE.md`, "Watch for this"); twin fixtures in the XP VM (name new ones
`BL…`; 8.3 names through `L:\IN` and `L:\OUT`); measure before and after
(the round-trip audit on the user's corpus with `AMIPRO_STYLES`, the ISO
corpus, and the VM folder, and `layout_comparison_report` on the same
three; the session's `measure.sh` in the scratchpad runs all six) for any
change that can move text or change written files; performance (a toggle
to the pages drawn on README31 and the user's largest document; typing
unchanged, interleaved A/B); small green commits (`tools/verify.sh`);
native v3 records for any model addition with round-trip and strictness
tests, older files loading; the three-approach timebox; push only at the
end of the session.

Bullets specifics:

- **Ami Pro's picture bullets are its artwork** (metafiles 2001–2007 in
  `AMIPRO.EXE`): look at them (the Insert Bullet dialog, `bl1-list`), never
  copy, trace, or embed them; draw shapes of the app's own.
- **The app's toolbar icons** are new SVG files written for the app, in
  the COSMIC symbolic style (one colour, recoloured by the theme).
- **Commit aggregate numbers only**, never the user's file names or text.

## 4. Stop rules

There is no planned stop. Stop only for what `AGENTS.md` lists (a missing
reference, a destructive action, a licence question, or a material
product decision these documents do not cover). If such a question comes
up, record it in the progress file as `WAITING FOR USER` for the slice it
affects, take the conservative choice that keeps the approved interface,
carry on with every slice that does not depend on it, and ask at the end,
batched and ranked, with a recommendation each. Physical checks are
batched at the end too; never ask the user to test what the tests, the
screenshots, or the VM can show.

## 5. Slices, in order

### BN.1 The map (evidence, no product code)

- The commands Ami Pro has (none for lists; Insert Bullet `0x344`,
  `1560:0122`; dialog 1116), inserted bullets (`<:B…>`, layout `10d0:3b66`,
  drawing `1200:0af0`), Right align (`1108:4107`), Space for (the First
  indent), the shipped list styles; VM fixture BL1. Output:
  `docs/research/BULLETS_NOTES.md`, the proposal (`UI_SPEC.md` §21), and
  the map at the top of the progress file. (Done before the run was set
  up.)

### BN.2 Inserted bullets in the model and the formats

- `InlineObject::Bullet` (a code among Ami Pro's 17, validated), native v3
  (`inline bullet`), the importer (`<:B<*X>>` with one of the 17 codes; any
  other form stays a legacy mark), the body writer (`<:B<*X>>`), the
  round-trip description; `Command::InsertBullet { at, code }`. Tests: BL1
  read, written byte for byte in its body, native round trip, strictness.
  Round-trip audit before and after (the inserted bullets of the corpora
  now objects; read back equal unchanged).

### BN.3 Inserted bullets laid out and drawn

- The engine gives an inserted bullet the width of the same bullet as a
  style bullet in the font where it stands and its line that font's
  height; the page surface draws it with the style bullets' shapes (one
  shape function for both); the caret, Delete, Backspace, selection, and
  copy treat it as one character. Layout comparison before and after
  (README31's lists move: explain them).

### BN.4 Right align, superscript, and the picture bullets

- Right align as `1108:4107` draws it; superscript bullet text as
  `1108:4063` raises it (trace the size first); the seven pictures as the
  app's own shapes. VM fixtures for each (BL2 Right align with one- and
  two-digit numbers, BL3 superscript, BL1's pictures at several sizes),
  compared with screenshots of the app. Layout comparison before and after
  (Right align and superscript move no text; say so with numbers).

### BN.5 List styles (B1, B3) in the model

- `amipro-editing` (a `lists` module, pure, tested): which kind a style
  shows; the document's list style by Ami Pro's names; Ami Pro's default
  Bullet 1 and Number List in a document's Body Text font;
  `Command::SetList { flow, first, last, kind }` (find or create, apply,
  or Body Text; one undo step); the rule that Enter in an empty list
  paragraph ends the list. Tests: a sheet with Bullet 1, one with only
  Bullet, one with neither (the style created, a document style, written
  as Ami Pro writes a created style).

### BN.6 The commands in the app (B1–B3)

- Format › Bullets and Numbering implemented as toggles with check marks
  following the caret; Ctrl+Shift+N; the toolbar buttons (new icons, after
  the alignment group in the Standard set, in Customize Toolbar); the text
  right-click menu; Enter in an empty list paragraph. Tests through
  messages (mixed selections, switching kinds, created styles, undo);
  screenshots light and dark.

### BN.7 Insert › Bullet… (B4)

- The dialog (the 17 bullets drawn, the example, Insert, Cancel, the last
  bullet remembered in the settings), Insert › Bullet… implemented, the
  Customize Toolbar item. Tests through messages; screenshots.

### BN.8 Style Properties (B5)

- Space for in the Bullets and numbers section (the First indent, the same
  value as Alignment's); Format › Bullet and Number Style… and its
  right-click entry open the panel at that section for the caret's style.
  Tests; screenshots.

### BN.9 Twins

- In the VM: BL1 made in the app (the 17 inserted bullets) and list
  styles applied and created in the app (twins of the same lists made in
  Ami Pro by applying Bullet 1 and Number List, and by Create Style on a
  sheet without them), written as SAM (an ignored `bullet_twins` test with
  `AMIPRO_VM_LAB`, like `frame_twins`): records equal to Ami Pro's; Ami Pro
  opens the app's twins as its own (VM screenshots).

### BN.10 Both formats and performance

- Documents with lists, created list styles, and inserted bullets written
  as `.amipro` and SAM, read back equal, nothing in the loss check; the
  round trip and the layout comparison on every corpus; a toggle's time to
  the pages drawn; typing A/B.

### BN.11 Exit

- §21's implementation notes, screenshots, `FEATURE_PARITY.md`,
  `UI_COMMAND_MATRIX.md`, `UI_ACCEPTANCE.md` (a bullets section),
  `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, and the handoff,
  ending with a short, ranked list for the user (the physical check, with
  exact steps).

## 6. Exit

Met when Bullets, Numbering, Insert › Bullet…, and Bullet and Number
Style… work as §21 describes under B1–B5; each change is one undo step;
inserted bullets, Right align, superscript, and the pictures are drawn as
Ami Pro draws them; documents with lists round-trip through both formats
with nothing in the loss check; and Ami Pro in the VM shows the app's
twins as its own. The user's physical check follows.
