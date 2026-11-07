# Frame Creation Runbook (Insert › Frame…)

**Written:** 2026-10-17, after the user's answers to the frame creation
proposal (`docs/Design/UI_SPEC.md` §20; decisions F1–F4 in
`docs/research/DECISIONS.md`).
**Purpose:** let one session, possibly resumed after context compaction,
carry the plan section "Creating frames" (`PROJECT_PLAN.md`) from start to
finish **as a straight run**: every decision it needs was taken before it
starts, so it does not stop in the middle to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the decisions
so far, `docs/Design/UI_SPEC.md` §20 the approved interface (with §11's
frame interaction). This runbook adds the order, the gates, and the stop
rules.

**Live state** is `docs/FRAME_CREATION_PROGRESS.md`. Keep it current: it is
how a resumed session knows where it is.

**Goal.** Frames are created in the app as Ami Pro creates them: Insert ›
Frame… (the Create Frame dialog, remembered from the last frame) and Insert
› Draw Frame (drawing on the page, also from the toolbar) make Ami Pro's
default text frame, Where placed, on top of the page's Where placed
frames, selected, in one undo step; Make Default for New Frames sets the
default as Ami Pro's Make Default does; Where placed frames are drawn above
anchored ones; the result is written in both formats and opened by Ami Pro
in the VM as Ami Pro's own twin is. Then the user's physical check of
creating, moving, and resizing frames (`UI_ACCEPTANCE.md` §10).

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/FRAME_CREATION_PROGRESS.md`,
   `docs/research/FRAME_CREATION_NOTES.md`, and `docs/Design/UI_SPEC.md`
   §11, §12, §16, §18, and §20; then `git status` and
   `git log --oneline -15`.
2. If the tag `frame-run-start` does not exist, create it at HEAD. It is
   the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE`.
   Uncommitted changes belong to an interrupted run: inspect, verify, and
   finish them; never discard them.

## 2. Decisions already taken (do not ask again)

- **F1:** both ways: Insert › Frame… opens the small Create Frame dialog
  (Width, Height, Down from top, In from left, remembered from the last
  frame made, drawn ones included; Create, Draw on Page, Cancel), and
  Insert › Draw Frame goes straight to drawing on the page; Draw Frame is
  on the Standard toolbar.
- **F2:** Make Default for New Frames and Restore Ami Pro's Default in
  Frame Properties; the default (wrap, opaque, lines, line style, position,
  colour, background, shadow and its colour, corners, margins; never
  placement, size, columns, or tabs) is kept in the app's settings for
  every document.
- **F3:** Where placed frames are drawn and hit above anchored frames;
  Bring to Front and Send to Back work within each kind; the SAM writer
  lists Where placed frames before anchored ones.
- **F4:** no pictures in this run (Insert › Image… stays Future), and no
  frames made automatically for pictures or objects.
- **Earlier decisions that apply:** rail inspectors over modal dialogs,
  small dialogs for one-shot actions; a convention used on one item goes
  on all its siblings; §11's frame interaction (selection, handles, Esc,
  Enter, the rail switch); Save As SAM writes new files only (Phase 4
  decision A) with the loss check (B); one length rule for every panel
  (`lengths.rs`); the toolbar and its settings (§12).
- **Already authorized** (`AGENTS.md`): decompiling any binary as needed,
  operating Ami Pro in the XP VM (including snapshots), and writing
  `.SAM` files under decision A.

## 3. Working rules

The ruler run's rules apply unchanged (`docs/RULER_RUNBOOK.md` §3, which
points to `docs/LAYOUT_SETUP_RUNBOOK.md` §3): the disassemblies are
authoritative for what Ami Pro does (the decompiled `AMIPRO.EXE`,
`AMIPROUI.DLL`, and `AMIUICNT.DLL` in `../AmiPro-Reference/ghidra-work/out/`;
findings into `FRAME_CREATION_NOTES.md` as VERIFIED (decompile), the
implementation written independently); the approved interface decides how
it looks and works (deviations from Ami Pro recorded in §20's
implementation notes); the pinned libcosmic's traps (`CLAUDE.md`, "Watch
for this"); twin fixtures in the XP VM for each way of creating (name new
ones `FC…`; 8.3 names through `L:\IN` and `L:\OUT`); measure before and
after (the round-trip audit on the user's corpus with `AMIPRO_STYLES`, the
ISO corpus, and the VM folder, and `layout_comparison_report` on the same
three) for any change that can move text or change written files;
performance (from the release that creates a frame to the pages drawn, on
README31 and the user's largest document; typing unchanged, interleaved
A/B); small green commits (`tools/verify.sh`); native v3 records for any
model addition with round-trip and strictness tests, older files loading;
the three-approach timebox; push only at the end of the session.

Frame creation specifics:

- **The VM's default frame is shared state.** A fixture that uses Make
  Default must put the built-in default back afterwards (Make Default on a
  frame with Ami Pro's built-in settings, as FC.1 did), so later fixtures
  start from Ami Pro's default.
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

### FC.1 The map (evidence, no product code)

- The Create Frame handler (`0x19e`), drawing (`0x199`, `0x19d`), the
  dialog (`ADDFRAMEDLG`), the new frame's settings, the default frame and
  Make Default (`0x19f`), the stack, the page, what is selected next;
  VM fixtures FC1–FC11. Output: `docs/research/FRAME_CREATION_NOTES.md`,
  the proposal (`UI_SPEC.md` §20), and the map at the top of the progress
  file. (Done before the run was set up.)

### FC.2 Ami Pro's default frame and the creation rules

- `amipro-editing` (pure, tested): the default frame's 17 values with Ami
  Pro's built-in ones; a new frame's style from a default and a rectangle
  (one column, no tabs, margins that do not fit dropped by Ami Pro's rule);
  the default taken from a frame (Make Default, keeping only what Ami Pro
  keeps); the dialog's starting values (remembered or built in, clamped to
  the page), limits, and the remembered size (at least 360 twips). Tests
  against FC1, FC3, FC6N's records.

### FC.3 Layering (F3)

- The engine draws anchored frames below Where placed ones; hit testing
  and Tab order follow the drawing order; Bring to Front and Send to Back
  within each kind; the SAM writer lists Where placed frames first. Check
  the corpora for files Ami Pro wrote in another order; round-trip audit
  before and after (identical expected); a test from FC8.

### FC.4 The commands and the dialog

- Insert › Frame… implemented; Insert › Draw Frame (new command, icon from
  the COSMIC theme, on the Standard toolbar, offered by Customize
  Toolbar); the Create Frame dialog (`UI_SPEC.md` §20): lengths as the
  rail's, limits refused in place, Create on the page in view, Draw on
  Page; the last frame's size and position in the settings. Tests through
  messages; screenshots light and dark.

### FC.5 Drawing on the page

- Drawing mode on the page surface: crosshair, status message, press,
  drag (the dashed outline, kept on the paper), release to one
  `CreateFrame` and one undo step; a click, Esc, or a press outside every
  page ends it; the new frame selected and the rail switched; Esc returns
  to the caret. Tests through pointer events in the window (scrolled and
  zoomed); screenshots.

### FC.6 Make Default (F2)

- Make Default for New Frames and Restore Ami Pro's Default at the end of
  Frame Properties; the default in the settings (tests with no settings or
  a scratch directory, never the real one); new frames take it.

### FC.7 Twins

- FC1, FC2, FC3, FC6N, and FC8 made in the app and written as SAM
  (an ignored `frame_twins` test with `AMIPRO_VM_LAB`, like
  `ruler_twins`): records equal to Ami Pro's; Ami Pro opens the app's
  twins as its own (VM screenshots).

### FC.8 Both formats and performance

- Documents with created frames written as `.amipro` and SAM, read back
  equal, nothing in the loss check; the round trip and the layout
  comparison on every corpus; the release-to-pages time; typing A/B.

### FC.9 Exit

- §20's implementation notes, screenshots, `FEATURE_PARITY.md`,
  `UI_COMMAND_MATRIX.md`, `UI_ACCEPTANCE.md` §10 (creating added),
  `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, and the handoff,
  ending with a short, ranked list for the user (the physical check of
  creating, moving, and resizing frames, with exact steps).

## 6. Exit

Met when frames are created both ways as §20 describes, by the rules FC.1
maps; each creation is one undo step; Make Default works as Ami Pro's;
Where placed frames are drawn above anchored ones; documents with created
frames round-trip through both formats with nothing in the loss check; and
Ami Pro in the VM shows the app's twins as its own. The user's physical
check follows.
