# Handoff archive

Older handoffs, moved verbatim from docs/SESSION_HANDOFF.md on 2026-11-04 (SU1).
Not loaded at session start.

## Handoff of 2026-11-07 (after run D's exit), moved 2026-11-07

**Updated:** 2026-11-07, after run D's exit.
`PROJECT_PLAN.md` is authoritative for sequencing. Earlier handoffs:
`docs/HANDOFF_ARCHIVE.md`.

### Run B finished and physically accepted 2026-11-06

Run B (`outline-mode`, OB.1–OB.8) is done and green: `tools/verify.sh` with
`AMIPRO_STYLES` unset (core 629 +20 ignored; app 581, text editor 128, all
features 588), release builds and the corpus audit pass. Measures: Layout
typing equal, Draft equal, Outline +0.1–0.15 ms a keystroke. The live check
on a private display passed except the navigator's arrow keys after a row
click (fixed: `PageSurface::arrows_elsewhere`, with a widget test). A
shrink pass took out about 20 lines (`7db7c8b`).

**Physical check: PASSED 2026-11-06 (the user: "all physical test passed"). It covered:** (1) View › Outline Mode: indents, icons,
typing, Alt+PageUp/PageDown, the bar, Promote/Demote, Alt+Up/Down; (2) the
Outline navigator: click, a real drag, then the arrow keys and Left/Right
after a click on a row; (3) View › Outline › Use Outline Styles; (4) Print
panel from Outline mode (it returns to Layout).

### Run D finished 2026-11-07 (resume here)

Run D (`objects`, `docs/OBJECTS_RUNBOOK.md`; record `docs/OBJECTS_PROGRESS.md`;
notes `docs/research/OBJECTS_NOTES.md`; `UI_SPEC.md` §42) is built; OD1-OD12
answered 2026-11-06 (recommended options). The exit records are written
(`FEATURE_PARITY.md` GR-01 to GR-07, `UI_ACCEPTANCE.md` Objects, the playbooks,
`FORMAT_COMPATIBILITY.md`). The gate, `tools/verify.sh --release --corpus`, the
handoff commit and the push are the committing session's: counts to expect are
core 790 (+29 ignored), COSMIC 666 default, 128 text editor, 673 all features
(84, 8, 84 ignored), with `AMIPRO_STYLES` unset.

**What was built.**
- *OLE (DO.3):* 34 of 34 corpus objects drawn from their stored presentation
  (`amipro_pictures::ole`), bytes never changed; Frame Properties › Object
  (read-only), Convert Object to Picture, Edit › Links… (read-only list). No
  live editing (OD7).
- *Paste Special (DO.4):* a rail list of the formats the clipboard offers;
  `paste_special.rs`.
- *Image processing (DO.5):* Brightness, Contrast, Edge Enhancement,
  Smoothing, Invert, Halftone, Revert; non-destructive; state words equal to
  Ami Pro's 12 of 12; a gray TIFF is stored as `.tgf`, other bitmaps baked by
  Save As SAM.
- *Charts (DO.6-DO.8):* the data record (104 of 104 corpus and 18 of 18 VM
  charts write back equal), the generator for all 12 types, written as Ami Pro
  writes them (`.sdw` picture type), the Charting rail panel (`chart_panel.rs`,
  `chart_commands.rs`) and Chart from Table.
- *Equations (DO.9):* Ami Pro's TeX dialect byte equal for 27 of 27 VM
  equations, layout fitted to them, the in-place card and rail palette
  (`equation_panel.rs`, `equation_commands.rs`), LaTeX paste. Shared rail
  helpers in `rail_panels.rs`.
- *Measures (DO.11):* typing with a chart and an equation costs 0.05 to
  0.15 ms a key; round trip and layout comparison identical to the start; twins
  CHT1, EQT2, the adjusted `.tgf` and an OLE round trip opened in Ami Pro.

**Open items.**
- Graphics Scaling opened nothing in the rail once (2026-11-06) and has not
  been reproduced (theory: a stale page after Insert › Image,
  `insert_picture_bytes`). If it returns: the status-bar text, whether the rail
  showed, whether the item was greyed.
- Equations: `\sqrt` and `\left`...`\right` widths are -11 % to +14 % against
  Ami Pro's own recomputation of the twin; the fit's worst cases are width
  16.9 % and height 6.2 % (both `\text{abc}` with a bold switch); `\sum` is 12
  twips off in height. Layout constants are fitted, not read (OK-DO9-1); about
  330 of Ami Pro's 887 symbol names (OK-DO9-7).
- Charts: unconfirmed areas in notes §14 (expanded and 3D pies, perspective
  for column, line and area, legends of three or more series, negative 3D
  stacked bars, small frames); Ami Pro regenerates an app-made chart 1 to 2 px
  different (CHT1); CHV4 not run; Pictures… not offered; no colour reorder.
- Image processing: the smoothing and edge kernels are inferred (OK-DO5-1, -2).
  (notes `TABLE_DATA_NOTES.md` §10.7 rate the case rule as medium).
- Not built by decision: OLE Update, Unlink and Edit, Paste Link, New Object.

**Physical check for you:** the Objects checklist in
`docs/Design/UI_ACCEPTANCE.md` (ranked): OLE Object section and Convert, Edit ›
Links…, Paste Special (a browser and a file-manager clipboard), the Image
Processing sliders, the Charting rail and Chart from Table, the Equations card,
palette and LaTeX paste; plus the Table data checklist still pending from run C.

**Next:** a closing pass (page-surface accessibility nodes, visible IME
preedit, UI-1/UI-2 leftovers), then Phase 6 packaging. Run E is not planned.

### Run C finished 2026-11-06

Run C (`table-data`, `docs/TABLE_DATA_RUNBOOK.md`, tag `table-data-start`;
record `docs/TABLE_DATA_PROGRESS.md`) is built and green; TN1–TN11 answered
2026-11-06. What it holds: table formulas (Ami Pro's grammar, engine in
`crates/document/src/formula.rs`, every corpus formula recomputes; Edit
Formula, Quick Add Row/Column, Recalculate Table; the rail's Formula
section; errors in the status bar, REF drawn), numbers shown through the
style's number format (screen and PDF; typed numbers rewritten as Ami Pro
does; the Styles rail's Table format section), Tools › Sort… (a rail panel;
Ami Pro's heap sort, tie order equal to the VM's), merge of table numbers as
Ami Pro merged them, the form recipe as a scenario, native extras `@avg`
`@min` `@max` `@count` (Save As lists them), View › Show › Table row/column
headings. Nine twins opened in Ami Pro as made; its recomputed results equal
the app's. Typing is unchanged (Layout/Draft within 0.02 ms); leaving a cell
in a table with 51 dependants costs about 3 ms.

Also done 2026-11-06 after run B: Esc cancels the topmost dialog; DF1 (a
Documents folder, Preferences › Folders, `file_dialogs.rs`); MD1–MD3
(Markdown opened, inserted, exported, saved: `read_markdown.rs`,
`markdown.rs`, pulldown-cmark; links read as `text (address)`); the rail
keeps its open or closed state across the 1200 px docking width.

**Physical check for you:** the Table data checklist in
`docs/Design/UI_ACCEPTANCE.md` (ranked), plus Preferences › Folders ›
Documents; open a `.md`, File › Export ▸ Markdown…, Save As › Markdown; Esc
in a few dialogs; the rail while un-maximizing the window.

**Not reproduced:** Graphics Scaling opened nothing in the rail once
(2026-11-06, the user's session; they could not reproduce it again, nor
could the live check). Theories left: a stale page after Insert › Image
(`insert_picture_bytes` takes the page from the caret, main.rs); an
in-between build. If it returns: the status-bar text, whether the rail
showed, and whether the item was greyed out.

(Run C's "next" was run D, below.)
**Gate note:** run the test steps with `AMIPRO_STYLES` unset (the
style-sheet tests count sheets); set it only for corpus, layout and
benchmark runs.

### Start here

Run A (`commands`) is finished (`docs/COMMANDS_PROGRESS.md`). This session
built CA.13 and CA.14 at full weight and closed the run:

- **CA.13 Line numbering:** mapped from the decompile and 16 VM fixtures
  printed to PostScript (`COMMANDS_NOTES.md` §8.5). The document's
  `LineNumbering` (`[lnopts]`, kept as read when unchanged), one
  `SetLineNumbering` step, numbers worked out after every layout
  (`crates/layout/src/line_numbers.rs`) and drawn by the page surface (so
  screen, print, and PDF), Page Layout Properties › Line numbering and
  Format › Line Numbering…; Draft shows none. 11 of 13 printed fixtures
  equal page by page (two differ by printer metrics); Ami Pro opens the
  app's numbered documents as made.
- **CA.14 Passwords:** Ami Pro's splay-tree coder reproduced
  (`crates/ami-formats/src/protect.rs`, §9.6): its 7 protected fixtures
  open and re-protect to their exact bytes; Ami Pro opens the app's
  protected files (one with a picture). File › Open asks for the
  password; File › Password… sets or removes it; every save writes the
  file protected while it is set (CK13).
- **CA.15:** corpora unchanged against the frame fix; typing level with
  the run's start; Draft costs nothing (`AMIPRO_PERF_DRAFT`); twin CAT1.
- **CA.16:** gate `tools/verify.sh --release --corpus` (result in the
  progress file's commit), records, this handoff, the push.

Run B (`outline-mode`, `docs/OUTLINE_MODE_RUNBOOK.md`, tag
`outline-mode-start`) is set up: the decompile brief and VM OM1 are in
`docs/research/OUTLINE_MODE_NOTES.md`; **OB.2 Move Up/Down is done**
(`crates/editing/src/sections.rs`, Alt+Up/Alt+Down and Bullets and
Numbering ▸, rows in a table, scenarios `outline_mode.rs`). **Next: OB.3**,
the Outline mode view (no pages, indents per level, Ami Pro's four icons,
the Outline menu and bar), then OB.4–OB.8.

Counts: core 628 (+19 ignored); app 556 default, 124 text editor, 563 all
features; `tools/verify.sh --release --corpus` green.

### Found and fixed by the live check before this handoff

Typing while a dialog was open went into the page behind it (every dialog;
fixed and tested), the password box did not take the keyboard, and Insert ›
File of a protected file failed late; details in `COMMANDS_PROGRESS.md`.

### Physical check: PASSED 2026-11-05 (the user: "all test passed")

Each item below passed its automated tests, Ami Pro's fixtures, and a run
of the release app on a private display (screenshots of each step looked
at); what is left is how it feels on your desktop.

1. **Line numbering:** open a document of a few pages, Format › Line
   Numbering…, choose Number text lines: numbers in the left margin beside
   each text line, counting on across pages; then Every fifth, Reset each
   page, and Number all lines (numbers to the bottom of the last page);
   File › Export PDF… shows them too.
2. **Passwords:** File › Password… (Enter and Verify), File › Save As,
   close it, open it again from File's recent files: it asks; a wrong
   password says so and lets you try again. Then the protected file in
   your own corpus: open it with its password (you know it; it is the one
   file the audits cannot read). File › Password… › Remove Password, Save:
   it opens without one.
3. **Move Up and Move Down:** in a document whose headings have outline
   levels (any of Ami Pro's outline sheets, or a document from
   `_DEFAULT.STY` with Title and Subhead; the app's own new documents'
   Heading styles have none yet), put the caret in a heading and press
   Alt+Up or Alt+Down: the heading moves with the text under it; in a
   table they move rows.

### Queued after run B (the user, 2026-11-05)

A Documents folder in Preferences › Folders where every file chooser
starts (DF1, replacing CK1), and Markdown as an exchange format, opened,
inserted, exported, and saved (MD1–MD3, CommonMark + GFM). Set it up as a
small straight run once OB.8 is done.

### Open points

- Outline mode (run B), after the live check: the ruler is now hidden in
  Outline mode (it was offset; `ruler_shown`). Left as they are: Ctrl+Shift+D
  toggles Draft and Layout, as Ami Pro's Draft does; Enter at a heading's end
  keeps the heading when its style has no Next style (the app's default
  headings have none; Ami Pro's sheets set them); once Return opened
  Document Properties (not reproduced; probably a modifier left down by the
  test driver). Escape now cancels the topmost dialog (`Message::Escape`,
  `dialog_cancel` in `main.rs`, in `dialog()`'s order; a debug assertion and
  `escape_cancels_each_dialog_and_applies_nothing` catch a dialog added
  without one); the key-to-message mapping in the subscription is only
  checked physically. RB1–RB3 in `OUTLINE_MODE_PROGRESS.md`.

- The default document's Heading styles have no outline level (Move Up/Down
  treats them as body text until given one; Ami Pro's `_DEFAULT` gives
  Title and Subhead levels). To check in OB.3.
- RB1: Move Up/Down moves a section whether expanded or not (OM3); Ami Pro
  moves an expanded heading alone.
- From earlier: New View and the Vertical Ruler (CK11), Draft's frame
  markers (CK10); `_FAX3`'s table 72 twips high (FG5/FG6), word breaking in
  a narrow gap (FG3); Demote then Promote of a Number List item; one click
  on the first spelling alternative; Ami Pro's date parser (MC4); dBASE
  memo fields; old sample bytes in Git history (see the archive).

Prompt for the next session:

```text
Read AGENTS.md, CLAUDE.md, and docs/SESSION_HANDOFF.md, then do
DF1 and MD1–MD3 (docs/PROJECT_PLAN.md, DECISIONS.md) as a small straight
run; run B is finished. Every change is one undo
step, written as Ami Pro writes it; scenarios pass before any physical
check. Use lower-cost subagents for routine work and testing, keep
decompiler output and my corpus file names out of Git, commit small
tested slices, and hand off with a short ranked list. Tests must never
open real desktop dialogs on my screen.
```

## Handoff of 2026-11-05 (run A, first session), moved 2026-11-05

**Updated:** 2026-11-05, after run A's first session (CA.1–CA.12) and the New… frame fix.
`PROJECT_PLAN.md` is authoritative for sequencing. Earlier handoffs:
`docs/HANDOFF_ARCHIVE.md`.

## Start here (2026-11-04, run A, first session)

The user passed the typing-undo check and said "start run A". Run A
(`docs/COMMANDS_RUNBOOK.md`) went straight through CA.1–CA.12; the state of
each slice, the evidence, and the run's choices CK1–CK11 are in
`docs/COMMANDS_PROGRESS.md` (also `DECISIONS.md`, "Run A: choices made in
the run"; `UI_SPEC.md` §39's implementation notes).

**The next session:** resume run A at CA.13 (line numbering, full weight),
then CA.14 (passwords, full), CA.15 (measures, typing A/B in Draft, one
twin), CA.16 (exit: `tools/verify.sh --release --corpus`, records, push).
The decompile mapping for CA.13 and CA.14 is in
`docs/research/COMMANDS_NOTES.md` §8–§9 (written at the end of this
session; check it before making the VM fixtures it lists). Then run B
(outline mode): write its runbook from OM1–OM4 and `UI_SPEC.md` §40.

### Fix after the user's check (2026-11-05)

The user's File › New… from `_REPORT2.STY` did not match the VM. CA.4 had
been tested only against itself. The documents were right, but Flow with
text frames and narrow gaps beside frames were laid out wrongly; fixed from
VM fixtures RPT2A, FA1–FA3, FG1–FG6, and New… is now compared with Ami Pro's
for nine shipped sheets (`COMMANDS_PROGRESS.md`, "Fixes found by the run").
Gate green (`tools/verify.sh --release`: core 609; app 547, 123, 554). Open:
`_FAX3`'s table 72 twips high (the space above beside a frame, FG5/FG6, to
decompile), and word breaking in a narrow gap (FG3). In the VM lab, a
fixture generator wrote over older inputs `IN/FS1–FS3.SAM` (not recorded
anywhere; the user's backup has them if needed).

### Built this session

- CA.1: baseline in `target/commands-baseline/start/` (equal to the merge
  run's exit).
- CA.2: Help › Keyboard Shortcuts (from the registry, filtered) and
  Compatibility and Fidelity (the loss check without saving), in the
  context drawer.
- CA.3: Tools › Preferences… (Ctrl+,): General (undo levels, recent files
  in the File menu, the sheet New… selects, custom zoom, the `.bak`),
  Folders (style sheets), View (every View toggle and mark), Toolbar.
- CA.4: File › New… from a style sheet (by description or name, recent
  first, With contents, a first-page preview, the sheet's macro named and
  not run); a toolbar choice.
- CA.5: Format › Paragraph… in the rail (indents, line spacing, alignment,
  Revert to style; the style's spacing and breaks shown); Increase and
  Decrease Indent (Ctrl+], Ctrl+[).
- CA.6: Format › Fast Format (Ctrl+T, toolbar toggle) as Ami Pro's.
- CA.7: Edit › Mark Text ▸ Protected Text and No Hyphenation; Select a
  Style (Ctrl+Y: the Styles filter, Enter applies); File › Revert to
  Saved… (asks; one undo step).
- CA.8: Format › Page Layout Changes ▸ and Ruler ▸ as menu commands.
- CA.9: the status bar's Face and Point Size lists; Insert › Special
  Character….
- CA.10: View › Full Page (Ctrl+D), Actual Size, Enlarged, Custom Zoom, Fit
  Page Width; Continuous Pages and Facing Pages, per tab.
- CA.11: View › Draft Mode (Ctrl+Shift+D) and Show Clean Screen (F11).
- CA.12 (partial): View › Show ▸ Tabs & Returns, Column Guides, Margins in
  Colour, Pictures. **Open:** Window › New View, Vertical Ruler (CK11).

**A fix found by the run:** protected text, no hyphenation, strikethrough,
and the scripts set in mid-paragraph read back unequal from SAM (the
reader took the off mark as an explicit "off"); fixed in the importer
(`build.rs`, `ended_as_style`) without changing any corpus measure (a first
version did, and was narrowed; `COMMANDS_PROGRESS.md`).

**Gate:** `tools/verify.sh --release` green at `3a6c2e0`: core 607 (+14
ignored); app 547 default, 123 text editor, 554 all features; fmt, Clippy,
and release builds clean.

### Physical check (about 10 minutes, ranked)

1. ~~File › New… with `_REPORT2`~~: passed 2026-11-05, after the frame fix
   (the user: "appears to be fixed").
2. ~~Format › Paragraph…, Ctrl+T Fast Format, the status bar's Face and
   Size lists, View › Facing Pages (page 1 alone on the right, CK8), Full
   Page, Draft Mode, Show Clean Screen, Show ▸ Tabs & Returns~~: all passed
   2026-11-05.

### Open points

- Earlier commits (since `1f1dc98`, 2026-10-12) hold a few bytes taken from
  two real files as test samples (a Word Pro header and the protected
  document's first coded bytes); the test now uses synthetic bytes
  (`afa87ae`), but the old bytes stay in Git history unless the user
  chooses to rewrite it.
- Draft draws frames and headers where they stand, not as one-line
  markers (CK10). New View and the Vertical Ruler not built (CK11).
- From earlier: Demote then Promote of a Number List item ends in List 1;
  one click on the preselected first alternative in the Spelling and
  Grammar pages replaces at once; Ami Pro's date parser (MC4); dBASE memo
  fields.

Prompt for the next session:

```text
Read AGENTS.md, CLAUDE.md, and docs/SESSION_HANDOFF.md, then continue run
A (`commands`) from docs/COMMANDS_RUNBOOK.md at the first slice of
docs/COMMANDS_PROGRESS.md that is not DONE (CA.13, line numbering, full
weight: decompile mapping in COMMANDS_NOTES.md §8, VM fixtures one
property at a time, twins), then CA.14–CA.16, then set up and run run B.
Every change is one undo step, written as Ami Pro writes it; scenarios
pass before any physical check. Use lower-cost subagents for routine work
and testing, keep decompiler output and my corpus file names out of Git,
commit small tested slices, and hand off with a short ranked list. Tests
must never open real desktop dialogs on my screen.
```


## The SU1/SU4 session (2026-11-04)
**Updated:** 2026-11-04, after the start-up cut, typing undo, and run A's
set-up. `PROJECT_PLAN.md` is authoritative for sequencing. Earlier
handoffs: `docs/HANDOFF_ARCHIVE.md`.

## Start here (2026-11-04, the SU1/SU4 session)

The user's order was: (1) cut the start-up reading, (2) typing undo as one
step per run, (3) set up run A, asking run B's decisions in the same
round. All three are done.

**The next session:** run A (`commands`) straight through, from
`docs/COMMANDS_RUNBOOK.md`; resume at the first slice of
`docs/COMMANDS_PROGRESS.md` that is not `DONE` (CA.1's baseline and tag
first, if not yet done). Then run B (`outline-mode`), whose decisions are
answered (OM1–OM4, `UI_SPEC.md` §40): write its runbook and go.

### 1. The start-up cut (SU1, done)

- `CLAUDE.md` is current state only (15,356 bytes): layout, building and
  testing, every "watch for this" note in a line or two. Its old text is
  verbatim in `docs/HISTORY.md`; the specialised test commands verbatim in
  `docs/TEST_COMMANDS.md`.
- This file is the current handoff only; older ones are verbatim in
  `docs/HANDOFF_ARCHIVE.md` (the scenario suite session first).
- `PROJECT_STATUS.md` and `PROJECT_PLAN.md` take one line per run from now
  on.
- Done by a sonnet agent, reviewed here; the verbatim moves checked with
  `diff`.

### 2. Typing undoes as one step per run (SU4, done)

- On the page surface a run of keys of one kind (typing, Backspace, or
  Delete) is one undo step while the caret stays at the run's end, the
  history has not changed (`History::changes`, new in the core: it grows at
  the 100-step limit too), no formatting was chosen at the caret, and keys
  come within 2 s (`TYPING_PAUSE`). A caret move, a command, Undo or Redo,
  Enter, Paste, or a whole power field taken by Backspace ends it
  (`TypingRun`, `continues_typing_run`, `update_page_from` in `main.rs`).
- Scenarios first, failing before the change: `scenario_tests/documents.rs`,
  `a_run_of_typing_undoes_and_redoes_at_once` and
  `a_caret_move_a_command_enter_or_a_pause_ends_a_run_of_typing` (keys
  through the window; `app.page_keyboard += 1` gives the page the
  keyboard, as a click does).
- Core test `history_changes_grow_at_the_limit_too`.

### 3. Runs A and B set up (decisions all as recommended)

- CM1–CM12 (run A) and OM1–OM4 (run B) in `docs/research/DECISIONS.md`;
  `UI_SPEC.md` §39 (run A) and §40 (run B); `docs/COMMANDS_RUNBOOK.md` and
  `docs/COMMANDS_PROGRESS.md`; the playbook's rows A and B.
- Weight (OM4): light, but full (decompile, VM fixtures, twins) for line
  numbering (CM7) and password-protected files (CM5).

### Physical check (a few minutes, any time)

- Type a sentence, Undo: it goes at once. Type, click elsewhere, type,
  Undo: only the second part goes.

### Smaller open points

- Demote then Promote of a Number List item ends in List 1, not Number
  List (it looks the same); one click on the preselected first
  alternative in the Spelling and Grammar pages replaces at once.

### Open points (the merge run)

- Ami Pro's own date parser gives odd results for some typed dates (MC4).
- dBASE memo fields are not read.

Prompt for the next session:

```text
Read AGENTS.md, CLAUDE.md, and docs/SESSION_HANDOFF.md, then run A
(`commands`) straight through from docs/COMMANDS_RUNBOOK.md, resuming at
the first slice of docs/COMMANDS_PROGRESS.md that is not DONE. Light
weight, but full (decompile, VM fixtures, twins) for line numbering and
password-protected files. Every change is one undo step, written as Ami
Pro writes it; scenarios for each slice in app/cosmic/src/scenario_tests/
pass before any physical check. Use lower-cost subagents for routine work
and testing, keep decompiler output and my corpus file names out of Git,
commit small tested slices, and hand off with a short ranked list. Tests
must never open real desktop dialogs on my screen.
```

## The scenario suite session (2026-11-04)

The user's order of 2026-11-04 was: (1) a full scenario suite, (2) fixes
from it, and (3) a plan to speed up the rest. All three are done. The
plan, `docs/SPEEDUP_PLAN.md`, was adopted in full (`DECISIONS.md`
SU1–SU4): run D gets full editors (SU2), and revision marking is not
planned (SU3).

**The next session, in order:**

1. **Cut the start-up reading (SU1).** A lower-cost agent does the
   editing and the main session reviews it.
   - `CLAUDE.md` becomes current state only, at most 15 KB: the layout,
     building and testing, and the "watch for this" notes. Its record
     lines and per-run histories move to `docs/HISTORY.md`.
   - This file keeps only the current handoff, at most 10 KB. Everything
     from "### 2. The merge recheck" down moves to
     `docs/HANDOFF_ARCHIVE.md`.
   - `PROJECT_STATUS.md` and `PROJECT_PLAN.md` get one line per run from
     now on.
   - Nothing is lost: what moves is moved, not rewritten.
2. **SU4: typing undoes as one step per run of typing.** The run ends
   when the caret moves, a command runs, or there is a pause. Write the
   scenarios first: `scenario_tests/documents.rs`, typing key by key
   through `key_press`, then Undo.
3. **Run A, `commands`** (templates, the remaining commands, and views;
   light weight), with run B's decisions asked in the same round (SU1).
4. ~~The user's merge recheck~~: passed (2026-11-04).

### 1. The scenario suite (done 2026-11-04)

`app/cosmic/src/scenario_tests/` (a child of `page_surface_app_tests`, the
`page-surface` build): 95 scenarios in 16 modules, one per area, each a
user's whole task through the window's message path, with one checker
after every step (`mod.rs`, `Scenario`):

- `edit` steps must change the document; Undo must return exactly the
  document before (compared as the round trip's facts) and Redo the one
  after, with the caret where the step left it. `act` and `no_change`
  steps for the rest.
- After every step: `Document::validate`, the pages lay out, the window's
  view builds and redraws, the hidden editor holds the main text, no power
  field shows an empty result or starts inside another (unless allowed),
  the status bar reads no failure (unless expected), no save problem.
- `save_sam` ends a scenario: no loss list (or exactly the expected one;
  an unexpected one prints the differing facts), the file reads back
  equal, and a fresh window reopens it to the same text.
- Run: `cargo test --offline --manifest-path app/cosmic/Cargo.toml
  scenarios::` (about 35 s in debug). Scratch files in
  `app/cosmic/target/scenarios/`.

Written by three sonnet agents from `UI_ACCEPTANCE.md` and `UI_SPEC.md`,
reviewed here. **Rule from now on** (memory `scenario-tests-before-handoff`):
each run adds its area's scenarios before the user's physical check.

### What the suite found, all fixed (2026-11-04)

1. **Undo and Redo lost the caret outside the main text** (headers,
   footers, frames, cells came back at their start; Redo of Insert ›
   Frame… left the frame unselected): an undo step keeps the page
   selection and the frame selection (`HistoryCaret`, `main.rs`).
2. **Lines, fill, leader, and protection on empty table cells were never
   written to the SAM** (a grid drawn before typing lost its rules; data
   files' empty cells too): `write_table` writes an empty cell with its own
   format.
3. **False loss lists:** a master document or a letter with a data file
   ("1 document setting": the empty place a taken section leaves), a
   generated TOC ("1 table": its page column not marked as its own width),
   a picture in an empty cell or an empty cell with lines ("1 table": a
   kept cell record counted on a cell written anyway), a document after
   Use Another Style Sheet ("1 page layout": the sheet's marks, 0x80 and
   0x8, now carried as reopening gives them).
4. **Merge:** the hidden editor did not follow the preview (now synced at
   each swap); "Edit failed: the merge settings are unchanged" when New
   Data File… replaced the file the letter already names (the same
   settings are no change); typing right after New Data File… did not
   reach the first cell (the page takes the keyboard:
   `PageSurface::keyboard`, `AmiProApp::page_keyboard`).
5. **Enter in Create Style's and Rename Style's name box** ran past the
   disabled button ("Edit failed" in the status bar): it waits now.
6. **A power field with an empty result** was silently not inserted while
   the status said "Power field inserted": the status now says it was not
   inserted and why (an empty field would be invisible).
7. **Replace All of an italic word** left the replacement roman: the
   replacement takes the found text's formatting (Exact attributes still
   sets its own); the find twins (Ami Pro's FR results) still pass.

### Open questions for the user (from the suite)

- **Undo of typing is per key** on the page surface (typing "Ann" takes
  three Undos). Ami Pro and modern editors undo a typing run at once.
  Recommended: one step per run of typing (until a caret move, a command,
  or a pause), as frame nudges already join within a second.
- Smaller, noted only: Demote then Promote of a Number List item ends in
  List 1, not Number List (it looks the same); one click on the
  preselected first alternative in the Spelling and Grammar pages replaces
  at once.

### Not covered by the suite yet

Real file choosers and portals (only their finished messages), visual
layout of dialogs, spelling as you type (the 350 ms pause), the Query$ and
Decide prompts, Include, picture mode by mouse, the ruler's keyboard mode,
right-click menus beyond the page text's. The agents' lists are in this
session's record (each module's top comment says what it covers).

### 2. The merge recheck (the user): PASSED 2026-11-04

File › Merge… again with the address block: the preview, clicks in it,
fields, Enter; Save As SAM; Backspace at the end of a line takes the
field before it whole (the user's saved `letter.sam` has two empty
fields after Zip: Backspace there removes them). Then
`docs/Design/UI_ACCEPTANCE.md`, "Merge, labels, envelopes", and saving
over a SAM copy.

### 3. Run 15 (`templates`), with the prompt below, once the user says so.

### Fixed on 2026-11-04, the merge's second use

- **The preview's caret:** when the preview went off, the caret kept its
  place in the record's values, so the next field or Enter went inside a
  field of the letter (`<La` `st>`; fields after Zip). The caret now maps
  back (`amipro_editing::merge::source_position`: by paragraph id, text
  between fields to itself, a value to after its field); typing in the
  preview turns it off and is made there (MC3 changed); a field is never
  inserted inside another; a field the data file has gives no "is not a
  valid field name".
- **Whole fields:** Backspace after a field and Delete before one take it
  whole, empty fields (which show nothing) first; a selection holding
  fields takes their marks (`Command::DeleteFields`, `field_beside`).
- **New Data File… over an existing file** (after the chooser's Replace)
  writes over it with `NAME.SAM.bak`, refreshes a tab showing it, and says
  why when it cannot; the panel's messages show at its top and in the
  status bar (`3eb5910`).
- Tests: `an_address_block_typed_around_the_preview_stays_whole`,
  `backspace_and_delete_take_whole_fields_and_leave_none_invisible`,
  `a_new_data_file_writes_over_a_confirmed_file_with_a_backup`,
  `a_place_in_the_preview_maps_back_to_the_letter`.

### Done on 2026-11-04

- **Save writes over SAM files** after the loss check, the original kept
  as `NAME.SAM.bak` the first time and never replaced (`8e94c17`).
- **MR1:** data files are entered as Ami Pro tables on the page (Ami Pro
  merges from them, VM MGT1); the grid is gone. New Data File… writes a
  table file; Ami Pro's delimited files show as the same table and save
  back in their form; Table › Sort Ascending, Sort Descending.
- **MR2:** the Merge panel is Ami Pro's three steps, one open at a time,
  wider, full-width buttons; conditions, labels, and one PDF per record
  under step 3's Options.
- **VM:** the table data file opens in Ami Pro as made, and Ami Pro's
  merge over it equals the app's. Found: Ami Pro merges a table cell that
  reads as a number as a number (`75.50` as `75.5`), an open point for the
  table-data run (`MERGE_NOTES.md` §13).
- `tools/verify.sh --release` PASS (core 601; app 423 / 117 / 430);
  release: a keystroke in a 1,000-record table 17 ms.

### What was done in the run's second session (2026-11-03)

- **ME.6–ME.11:** the Merge panel's rest (fields the data file lacks named
  and merged empty, as Ami Pro's MGM8), every output (New Document, PDF one
  or one per record named from a field, Printer…, View & Print with edits
  carrying, Ami Pro File…), the record editor (a grid in the data file's
  tab, each change one undo step, Ami Pro's delimited form), labels (26
  Avery sheets, Ami Pro's set-up, start label, copies, whole sheets),
  File › Print Envelope… (the Envelope panel: sizes, the recipient found,
  saved return addresses, feed and offsets, Print, PDF, New Document,
  Merge Envelopes; on `_AMIENV.STY` when found), Start from (Letter,
  Labels, Envelope).
- **ME.13 twins pass in Ami Pro:** Ami Pro opens the app's data file,
  merge letter, and label document; its own merge of the app's letter
  equals the app's paragraph for paragraph; the label document shows 3 ×
  10 at 0.50 and 0.19 in.
- **ME.12, ME.14:** the round trip and layout comparison unchanged (but
  Ami Pro's merge of the twin, now in the VM folder); release timings: a
  1,000-record merge to a new document 177 ms, to one PDF 3.1 s in the
  background, a preview step 1.9 ms; `tools/verify.sh --release --corpus`
  PASS (core 599, app 422 / 116 / 429).
- The full suite caught one more defect: Undo in the record editor or
  while viewing records panicked (the hidden editor's text was not kept
  in step); fixed and tested.

### Found on the way (fixed)

- **Text typed right after a field went inside it**, so a second field
  clicked in after a typed space nested, and Update would delete the
  typed text. Text typed right after a field now goes after it (MC11).
- A NumPages field's marks were rewritten at stale places when its result
  changed length (the order of changes in Update).
- The Labels unit: Ami Pro's 1 is inches (the app first wrote 0).

### Choices made in the run (also `DECISIONS.md`)

MC1–MC4 (first session); MC5 the sheet list (Avery laser and inkjet
sheets, no continuous forms); MC6 (above); MC7 View & Print prints the
kept records together at the end; MC8 labels to New Document are whole
sheets; MC9 the print dialog's pages count across the merged output; MC10
an envelope on larger paper goes at the feed's place; MC11 (above); MC12
one PDF per record never over a file; MC13 unknown fields named and
merged empty, nothing asks.

### Open points

- Ami Pro's own date parser gives odd results for some typed dates (MC4).
- dBASE memo fields are not read.

Prompt for the next session:

```text
Read AGENTS.md, CLAUDE.md, and docs/SESSION_HANDOFF.md, and do the
handoff's next-session order: cut the start-up documents as SU1 says
(delegated, reviewed), make typing undo as one step per run (SU4) with
scenarios first, then set up run A (`commands`) as a light straight run,
asking A's and B's decisions in one round, ranked with a recommendation
each. Every command is one undo step, written as Ami Pro writes it; add
the run's scenarios to app/cosmic/src/scenario_tests/ and have them pass
before any physical check. Keep decompiler output and my corpus file
names out of Git, commit small tested slices, use lower-cost subagents for
routine work, and hand off with a short ranked list. Tests must never
open real desktop dialogs on my screen.
```

## Run 14 (`merge`), first session (2026-11-03)

Set up and ran to ME.7 in part: VM fixtures, `UI_SPEC.md` §38, the
runbook, the baseline; ME.2–ME.5 in the core; the Merge panel's first
parts and three outputs (`docs/MERGE_PROGRESS.md`).

## After the master run (2026-11-03): fixes from use

- **Spelling as you type** underlined nothing on the desktop: a stale
  pause cleared the tab's checked root and scheduled another pause
  forever. A right-click suggestion replaced nothing: the right press
  built the menu before moving the caret. The suggestions showed only in
  a maximised window: libcosmic opens a tall menu upwards past the
  window's top, so the spelling menu now holds only the spelling entries.
  All fixed (`6c27b7d`; `docs/PROOFING_PROGRESS.md`, end), with
  `AMIPRO_TRACE_SPELL=FILE`; the user's recheck passed.
- **Watch for this:** the test `Runtime` and `send` run each task to its
  end before the next message, so timing bugs between messages (stale
  pauses, the order of a widget's published messages) do not show there.
  A trace switch to a file is the fastest way to see the desktop's order.
- The master documents and the TOC, index, glossary physical checks
  passed (2026-11-03).

## The master documents run (2026-11-03, second session: the exit)

### What I needed from you (all done 2026-11-03)

Both physical checks passed, and run 14's decisions were answered.

### What was done this session

- MR.9 The set exports as one DOCX, ODT, RTF, or text, a section per file
  starting at its page number in the set (`amipro_exchange::set`); a member
  exported alone writes the numbers it shows.
- MR.10 Find's Options ▾ › In All Files of the Master: counts by file,
  Find Next and Previous go on into the next file's tab, Replace All is one
  undo step in each file it changes.
- MR.12 **Twins pass in Ami Pro:** the app made a master over MD3's members
  and wrote their `[sequence]` exactly as Ami Pro had; Ami Pro opened a
  member at page 3 and footnote 3, and worked the app's master to MD3's
  TOC (pages 1–6) and index, line for line.
- MR.11, MR.13 Measures and the exit (below).

### Found on the way

- **A defect the twin found, fixed:** a new master saved into its files'
  folder kept them marked missing (the set never looked again when its
  folder changed). Regression test added.

### Choices made in the run (also `DECISIONS.md`)

- MQ1 Treat as Single holds for the tab. MQ2 Rows move with ↑ ↓, no drag.
  MQ3 Seq fields take the carried values on update.
- **MQ4** The set's export keeps one style per name, the first file's;
  the loss list says when a later file differs.
- **MQ5** The set's export holds what printing it does (TOC file, files,
  index file); This document only exports the master alone.
- **MQ6** Find across the set lists files in Options ▾ › Files of the
  Master; Replace All leaves the files it changes open and unsaved.

### Measures

- Round trip and layout comparison equal to the run's start on both
  corpora and the VM folder.
- Release, the ISO's 32 guide chapters as one set (483 pages): read and
  numbered in 0.5 s off the interface's thread; a keystroke in a chapter
  with the master open 1.9 ms (1.3 ms alone); the set as one DOCX 91 ms.

### Next steps (ranked)

1. Run 14, `merge` (merge, envelopes, labels).
2. Open from this run: dragging rows in the panel (MQ2, needs a list drag
   in libcosmic).

## The master documents run (2026-11-03, first session)

### What I need from you (ranked)

1. **Nothing to decide.** MS1–MS11 are recorded (`DECISIONS.md`); the
   run's own choices are MQ1–MQ3 (below).
2. **Start the next session** with the prompt below; it resumes at MR.9.
3. **The TOC, index, glossary physical check** (from the last run) is
   still open; the master run's own check comes at its exit.

### What was done (commits since `master-run-start`)

- Setup: `MASTER_NOTES.md` (decompile and VM fixtures MD1–MD3),
  `UI_SPEC.md` §37, MS1–MS11.
- MR.2 `[master]`'s file list and `[sequence]` typed (model, native v3,
  SAM); every corpus file reads back equal.
- MR.3–MR.4 A member's numbers continue from `[sequence]`: pages,
  footnotes, numbered styles, Seq fields. **Ami Pro's own `[sequence]`
  sections are reproduced:** the members of MD2 and MD3 lay out with Ami
  Pro's pages 1–6 and pass on exactly what Ami Pro wrote into the next
  file (`master_twin`).
- MR.5–MR.6 File › Master Document… opens the panel; the set is read off
  the interface's thread and numbered live (an edit in one file moves the
  next file's numbers at once; nothing is written to the files); Treat as
  Single Document; Open Master for a member opened alone.
- MR.7 The set's TOC and index in new tabs, as Ami Pro's MD3 (rows 1, 3,
  5; "Common, 2, 4, 6"); Ctrl+click on a row opens the file there.
- MR.8 Print and Export PDF of the whole set (or this document only), one
  PDF with a bookmark per file.

### Found on the way

- **The ISO's user's guide is a master's members, not a master:** its 59
  chapters carry `[sequence]` (pages and list counters from the authors'
  set), but no chapter lists files. The app now shows their page numbers
  continued, as Ami Pro does.
- **Ami Pro's master run re-saves every member** (TOC, index, and print
  alike), and with the master in its own list it hits "currently
  protected". The app never writes the members (MS3).
- **Leading text or a start page in a member's page numbers** shifts the
  set (MD1); the panel's checks list such files (MS9).

### Choices made in the run

- **MQ1** Treat as Single holds for the tab while it is open.
- **MQ2** Rows move with ↑ and ↓; no drag.
- **MQ3** Seq fields take the carried values when fields are updated.

### Next steps (ranked)

1. **MR.9 Export of the set** (MS10): one DOCX, ODT, or RTF, a section
   per file.
2. **MR.10 Find across the set** (MS11).
3. **MR.12 Twins** (the app's master and members opened and worked in Ami
   Pro), **MR.11 measures**, **MR.13 exit** (records, verify, push).

(The first session's prompt resumed at MR.9; the run is finished.)

## The TOC, index, and glossary run (2026-11-03)

### What I need from you (ranked)

1. **The physical check** (about five minutes; `docs/Design/UI_ACCEPTANCE.md`,
   "TOC, index, glossary"):
   - **TOC:** in a document of yours with headings, Tools › TOC, Index…;
     Generate Contents; Ctrl+click a row; Undo.
   - **Index:** select a word, Edit › Mark Text ▸ Index Entry…, Mark;
     Mark All on another word; Generate Index (a new tab).
   - **Glossary:** Edit › Mark Text ▸ Glossary…, New… a data file, Add a
     selected sentence as "sy"; type "sy" and press Ctrl+K.
   - **Then:** save as SAM and reopen. Tell me how the panel and Ctrl+K
     feel.
2. **Start the next run** (master documents) with the prompt below.

### What was done (commits since `toc-index-run-start`)

- `b452105` `[toc]`, `[master]`, and the index and TOC marks typed (model,
  native v3, SAM).
- `28e9d2e`, `92f40e8` The TOC and the index in the core:
  - Ami Pro's two-column page table (TOC1–9, TOCPG1–9, leaders), Update in
    place.
  - The TOC's page break, after which page numbers start again (flag 0x02).
  - The index by Ami Pro's sort, merging, pages, Other, and separators;
    ranges as an option.
- `803b8c3` The Contents & Index panel (levels, places, Mark Index Entry
  live, Mark All, TOC Entry), Edit › Mark Text ▸, TOC links (Ctrl+click and
  the PDF).
- `52b65bb`, `e7bbd11` Glossary data files read and written as Ami Pro's;
  Mark Text ▸ Glossary…, Ctrl+K, the Glossary panel, completion as you
  type.
- `c06ba2f` The twins and two fixes they found (below).

### Found on the way

- **Ami Pro's own output reproduced.** From their inputs the app makes
  Ami Pro's TOC tables, the same index, and the same glossary file bytes.
  Ami Pro regenerates the app's TOC in place, and inserts the app's
  glossary records.
- **Two fixes from the twins.**
  - Without stored levels, Ami Pro rebuilt the app's TOC from its marks
    alone. A TOC now stores its levels as `[toc]` (TQ5).
  - Editing a later page lost the page numbering that restarts after a TOC
    page. Fixed, with a regression test.
- **Numeric table cells (TQ2; for the table-data run).** Ami Pro shows a
  cell stored as `5.00` as `5`, through the style's number format; the app
  shows `5.00`. About 550 two-decimal cells in your files are affected.
- **An index in a new tab lacks `_INDEX.STY`'s "Index" title frame (TQ6).**
  This is Use Another Style Sheet's open gap.

### Measures

- Round trip and layout comparison as at the start on both corpora, with no
  row changed. The VM folder agrees with Ami Pro on all 348 page counts.
- Release: README31's TOC in 6.8 ms and its index in 1.1 ms. On your largest
  document, Mark All of "the" (1,187 places) takes 6.0 ms.
- Typing A/B equal to the start.
- `tools/verify.sh --release --corpus`: core 551 + 13 ignored; app 396, 116,
  403 (65, 6, 65 ignored); strict Clippy, fmt, release builds, ISO corpus
  gate PASS.

### Next steps (ranked, 2026-11-03)

1. **Master documents** (queue run 13). `[master]` is typed with its file
   list kept as read, ready for that run.
2. **From this run, small:**
   - The sheet's layout frames in an output document (TQ6).
   - The index's marks given their pages (TQ3), if Ami Pro's re-save ever
     needs them.
3. **For the table-data run:** numeric cells (TQ2).

### Prompt for the next session

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run, master
documents (run 13), as a straight run by the playbook's recipe and
docs/playbooks/FIELDS_AND_REFERENCES.md: map the feature in the decompile
first (AMIPRO.EXE and AMIPROUI.DLL: File › Master Document…, its Options,
[master] and [sequence], page, footnote, and outline numbering across the
files, the TOC and index over them, printing the set) and confirm it in the
XP VM with fixtures (the ISO's user's guide is a master document); look for
a better way and for logical functions to add to each item; write the
proposal in docs/Design/UI_SPEC.md (§37); ask me every decision the run
needs in one go, with a recommendation each, and record my answers in
docs/research/DECISIONS.md; then write the runbook and progress file, tag
the start (master-run-start), and run it straight through. Every command is
one undo step, tested through the message path and the widget path, and
written as Ami Pro writes it (twins opened in Ami Pro). Measure with the
round-trip audit and layout_comparison_report before and after any change
that can move text, keep decompiler output and my corpus file names out of
Git, commit small tested slices, update the playbook's queue, and hand off
with a short ranked list. Use lower-cost subagents for routine work and have
them report the results back to you. Tests must never open real desktop
dialogs on my screen (see CLAUDE.md, the Runtime's click_unrun).
```

## The proofing run (2026-10-26)

### What I need from you (ranked)

1. ~~The physical check~~: **passed 2026-11-03** (about five minutes; `docs/Design/UI_ACCEPTANCE.md`,
   "Proofing"): type a sentence with "recieve" and "the the"; wavy red
   lines appear after a moment; right-click each (a suggestion, Delete
   Repeated Word). F7 in a document of yours: Skip, Replace, Replace All,
   Add to Dictionary, the options. Shift+F7 on a word: a meaning, a
   synonym, Replace; right-click another word: Synonyms ▸. Tools › Grammar
   Check… on "This are a apple.": Replace, Skip, the statistics at the end.
   Language British; save as SAM and reopen. Tell me how the underlines
   and the panel feel.
2. ~~Harper's weight~~ (SK4; **kept**, the user, 2026-11-03): grammar brings `harper-brill`, whose
   part-of-speech tagger pulls in `burn` (a machine-learning framework,
   CPU only): Harper's dependency tree is 207 crates (some already in the
   build), and the lock file lists 291 more packages than at the run's
   start. Say if you would rather drop grammar to readability statistics
   only.
3. **Start the next run** (TOC, index, glossary) with the prompt below.

### What was done (commits since `proofing-run-start`)

- `090f7ef` The document's language (`[lang]`, Ami Pro's 28 numbers or
  any locale) and grammar set (`[GramStyle]`) typed; `<:s>`, Ami Pro's
  "checked" flag, dropped from paragraphs the app edits (so Ami Pro checks
  them again); the spell check's core.
- `4f5f3e3` The spelling engine (`spellbook`, the system's Hunspell
  dictionaries and a bundled en-US), Spell Check in the Proofing panel,
  spelling as you type with the right-click suggestions.
- `88521f2` The language drop-down and default, hyphenation by language;
  the thesaurus (MyThes, inflected synonyms, Synonyms ▸).
- `d39298f` Grammar (Harper behind Ami Pro's eight sets) and readability
  statistics by Ami Pro's formulas.
- `cec8f1d`, `cd121cd` A check went round once it reached another stream
  (found by the benchmark, fixed with a regression test); grammar
  underlines on a worker thread.

### Found on the way

- **`<:s>` is Ami Pro's "spell-checked" mark** (11,799 in 249 of your
  files). Ami Pro skips such paragraphs in later checks; the app kept the
  mark through edits, so Ami Pro would never have checked edited text
  again. Fixed (twin APF1 shows Ami Pro stopping in the edited paragraph).
- Ami Pro's Replace All is exact-case and forward only, and Undo restores
  one replacement: the app's Replace All (SP4) is broader by your choice.

### Measures

- Round trip and layout comparison as at the start on all three corpora.
- Typing A/B equal to the start (README31 1.00–1.28 against 0.91–1.45 ms).
- Release: as you type 3.3 ms cold for README31, 0.14 ms after a
  keystroke; a Spell Check step about 30 ms; Harper about 10 ms a
  paragraph (on a worker thread); your corpus checked without a panic.
- `tools/verify.sh --release --corpus`: core 531 + 12 ignored; app 387, 116, 394 (61, 6, 61 ignored); strict Clippy, fmt, release builds, ISO corpus gate PASS.

### Next steps (ranked, 2026-10-26)

1. **TOC, index, glossary** (queue run 12).
2. From this run, small: suggestions for a Spell Check step on a worker
   thread (30 ms a step now); grammar per paragraph cached across
   sessions; a language per paragraph (native only) if wanted.
3. Still open from earlier runs: incremental layout with footnotes (NK2);
   Note Initials (NK4); one set of handles for several drawing objects
   (DK2); Home and End in a table (TK7).

### Prompt for the next session

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run, TOC, index,
and glossary (run 12), as a straight run by the playbook's recipe and
docs/playbooks/FIELDS_AND_REFERENCES.md: map the feature in the decompile
first (AMIPRO.EXE and AMIPROUI.DLL: Tools › TOC, Index…, Edit › Mark Text,
Insert › Glossary Record…, the [toc] section and the <:X> index entries)
and confirm it in the XP VM with fixtures; look for a better way and for
logical functions to add to each item; write the proposal in
docs/Design/UI_SPEC.md (§36); ask me every decision the run needs in one
go, with a recommendation each, and record my answers in
docs/research/DECISIONS.md; then write the runbook and progress file, tag
the start (toc-index-run-start), and run it straight through. Every
command is one undo step, tested through the message path and the widget
path, and written as Ami Pro writes it (twins opened in Ami Pro). Measure
with the round-trip audit and layout_comparison_report before and after
any change that can move text, keep decompiler output and my corpus file
names out of Git, commit small tested slices, update the playbook's queue,
and hand off with a short ranked list. Use lower-cost subagents for
routine test runs and have them report the results back to you. Tests
must never open real desktop dialogs on my screen (see CLAUDE.md, the
Runtime's click_unrun).
```

## The notes, bookmarks, footnotes, and endnotes run (2026-10-26)

### What I need from you (ranked)

1. ~~The physical check~~: **passed 2026-10-26** ("all items pass"; about five minutes; `docs/Design/UI_ACCEPTANCE.md`,
   "Notes, bookmarks, footnotes, endnotes"): in a document of yours,
   Insert › Note (Ctrl+Alt+M) and type in the Notes panel; double-click the
   mark; Insert › Footnote (Ctrl+Alt+F), type it on the page, double-click
   between the reference and the footnote; Tools › Footnote Options… (try
   Make endnotes); select a word and add a bookmark (Edit › Bookmarks…),
   Go To it, Insert › Cross-Reference…; save as SAM and reopen. Tell me how
   editing a footnote in place feels.
2. **Start the next run** (proofing) with the prompt below.

### What was done (commits since `notes-run-start`)

- `5d69aa0` Notes, footnotes, table footnotes, Footnote Options, and
  bookmarks in the model, native v3, and SAM; footnotes laid out as Ami
  Pro lays them out (one area per page, the separator, splits with the
  continued messages, the referencing line moving to the next page,
  endnotes).
- `358c39f` Note marks, footnote areas, and references drawn; footnotes
  edited in place.
- `2de8ed9` The commands (each one undo step).
- `cd7014a` The Notes, Bookmarks, and Footnotes panels in the rail; Insert ›
  Note and Footnote; View › Notes and Bookmarks.
- `bb19b37` Edit › Go To steps through bookmarks, notes, and footnotes.
- `feac44e` Bookmark fields and cross-references; footnotes in Find's
  streams; word count without notes.
- `963e424` DOCX, ODT, RTF out and in; With notes in print and PDF;
  bookmarks as the PDF's named destinations.
- `d58c954` Insert › Footnote in a table's cell, as Ami Pro's joined row.
- `4ac20d0`, `f6be8a3` Layout findings, note marks at Ami Pro's size,
  screenshots.

### Found on the way

- **A hard return that opens a paragraph makes no empty line**, and a
  footnote's last paragraph keeps its space below (NK10; found through the
  one corpus document with footnotes and Ami Pro's caches): 48 of your
  files now break closer to Ami Pro's pages, 7 one paragraph further.
- **Ami Pro numbers notes and table footnotes within their own text**: a
  note in a header, footer, or footnote is number 0 (NK3); a table
  footnote follows the table's others as inserted (NK9).
- **krilla has no text annotations**, so the PDF carries notes as Ami
  Pro's printed note pages (NK7).

### Measures

- **Round trip:** equal to the run's start on all three corpora (user 417
  of 417, ISO 159 of 160, VM folder 335 of 335).
- **Layout comparison against the start:** your corpus page counts 315 →
  318 of 394, paragraph line counts 14,880 → 14,938, page ends 762 → 782
  of 1,108; ISO 69 → 70; VM folder 333 → 334 of 334.
- **Twins:** ANT1, ABK1, AFN1 (identical to Ami Pro's RNF5), AFN2, AFN3
  open in Ami Pro 3.1 as made.
- **Timings (release):** typing A/B equal to the start (0.84–0.97 against
  0.85–0.95 ms); a keystroke in a document with footnotes 1.23 ms; a
  43-page document with footnotes relaid whole in 18 ms (NK2).
- `tools/verify.sh --release --corpus`: core 511 + 12 ignored; app 369, 115, 376 (58, 6, 58 ignored); strict Clippy, fmt, release builds, ISO corpus gate PASS.

### Next steps (ranked, 2026-10-26)

1. **Proofing** (queue run 11).
2. From this run, small: incremental layout for documents with footnotes
   (NK2); View › Note Initials (`[LS1]`, NK4); cross-reference links in
   the PDF; renumbering cross-references when bookmarks move.
3. Still open from earlier runs: Graphics Scaling's Rotate of a drawing;
   one set of handles for several objects (DK2); Home and End in a table
   (TK7); text beside two Wrap around frames with a gap.

### Prompt for the next session

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run, proofing:
spelling, thesaurus, grammar, and language (run 11), as a straight run by
the playbook's recipe and docs/playbooks/PROOFING.md: map the feature in
the decompile first (AMIPRO.EXE, AMIPROUI.DLL, and the speller and
thesaurus modules) and confirm it in the XP VM with fixtures; look for a
better way and for logical functions to add to each item; write the
proposal in docs/Design/UI_SPEC.md (§35); ask me every decision the run
needs in one go, with a recommendation each, and record my answers in
docs/research/DECISIONS.md; then write the runbook and progress file, tag
the start (proofing-run-start), and run it straight through. Every command
is one undo step, tested through the message path and the widget path,
and written as Ami Pro writes it (twins opened in Ami Pro). Measure with
the round-trip audit and layout_comparison_report before and after any
change that can move text, keep decompiler output and my corpus file names
out of Git, commit small tested slices, update the playbook's queue, and
hand off with a short ranked list. Use lower-cost subagents for routine
test runs and have them report the results back to you. Tests must never
open real desktop dialogs on my screen (see CLAUDE.md, the Runtime's
click_unrun).
```

## The drawing run (2026-10-25)

### What I need from you (ranked)

1. ~~The drawing physical check~~: **passed 2026-10-26** (about five minutes;
   `docs/Design/UI_ACCEPTANCE.md`, "Drawings and Draw mode"): open one of
   your documents with drawings; Insert › Drawing…, draw with a few tools,
   select, move, resize, group, edit points (double-click an object),
   type a text object; Copy an object into another program and paste an
   image or SVG in; Esc, save as SAM, reopen. Tell me how Draw mode feels.
2. **Start the next run** (notes, bookmarks, footnotes, endnotes) with the
   prompt below.

### What was done (commits since `drawing-run-start`)

- `a63cf29` The map (`docs/research/DRAWING_NOTES.md`: the decompiled
  `AmiDraw.dll`, `DrawMan.dll`, `AMIPRO.EXE`; VM DR1, DRBIG, DRW0–DRW3),
  the proposal (`UI_SPEC.md` §33), your answers DW1–DW9, the runbook.
- `ebc681e` `amipro_pictures::sdw`: Ami Pro's drawing stream read and
  written (all 487 drawings in the corpora, the VM folder, and the shipped
  symbols, byte for byte but for runtime links).
- `9a523e8` Drawings and charts drawn from their objects on screen, in PDF,
  and in exports; turned and spread text in both renderers.
- `73e3e85` `Command::SetDrawing` and Draw mode's operations; shapes built
  as Ami Pro builds them (VM DR2B).
- `5b71be7` Draw mode: tools, gestures, selection, handles, point editing,
  text objects, keys, smart guides; the draw bar in the toolbar's place;
  the Draw menu; Drawing Properties in the rail.
- `0d34eb5` SVG and PNG out, SVG and pictures in, Import Drawing…, Save
  Drawing As…, Copy, Cut, Paste; exports carry drawings as SVG.
- `c1cbf57` Twins (ADR1–ADR3) and release timings.

### Found on the way

- **238 of your 380 drawings showed as grey boxes** (no screen image
  kept); all 371 drawings and charts now draw from their objects. Against
  Ami Pro's own screen images the median ink agreement is 89 % within two
  pixels; what differs is Ami Pro's 16-colour dithering, frame
  backgrounds, a few old screen images the earlier decoder garbles, and
  small offsets.
- **Ami Pro needs no driver state for a drawing:** a hand-written stream
  with none and no screen image opens, and Ami Pro writes it back
  unchanged with a fresh state (DRW0), so edited drawings are written
  that way.
- **The drawing format is twips, y down, shapes as points** (ellipses and
  rounded corners as Béziers); text keeps an angle and is spread to its
  box. Two text attribute bits no file uses draw at a wrong size in Ami
  Pro.

### Measures

- **Round trip and layout comparison:** equal to the run's start on all
  three corpora (the VM folder gained the run's five fixtures, all equal).
- **Twins:** ADR1 (every tool and look), ADR2 (Ami Pro's DR2B edited), ADR3
  (an SVG and a picture imported) open in Ami Pro 3.1 as the app shows
  them; Ami Pro edits ADR3 in its Draw mode and saves the app's stream
  back byte for byte but for runtime handles.
- **Timings (release):** a corpus drawing read, painted, and drawn at
  100 % 3.8 ms median (max 26.5 ms); a move in Draw mode on 1,100 objects
  1.3 ms end to end; a line width 0.6 ms.
- **The typing A/B** (`tools/bench-ab.sh drawing-run-start 2`): README31
  typing medians 0.87–0.96 ms at the start against 0.93–1.20 ms now; round
  1 favours the start, round 2 overlaps it: within this host's spread.
- `tools/verify.sh --release --corpus`: core 489 + 11 ignored; app 360,
  115, 367 (56, 6, 56 ignored); strict Clippy, release builds, ISO corpus
  gate PASS; app fmt failed only on the tests appended last, fixed in the
  handoff commit.
- **After the run (your report):** the grid did not show. It drew one-pixel
  half-transparent dots, too faint to see; they are two pixels and solid
  blue now (`drawing_grid_screenshot`).

### Next steps (ranked, 2026-10-25)

1. **Notes, bookmarks, footnotes, endnotes** (queue run 10).
2. From this run, small: Graphics Scaling's Rotate of a drawing turns the
   objects in Ami Pro (no file has one); several objects show one set of
   handles (DK2); smart guides only while moving.
3. Still open from earlier runs: Home, Home and End, End in a table (TK7);
   text beside two Wrap around frames with a gap; a picture placed inside a
   cell's frame.

### Prompt for the next session

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run, notes,
bookmarks, footnotes, and endnotes (run 10), as a straight run by the
playbook's recipe and docs/playbooks/FIELDS_AND_REFERENCES.md: map the
feature in the decompile first (AMIPRO.EXE, AMIPROUI.DLL; how notes,
bookmarks, and footnotes are stored, laid out, and edited) and confirm it
in the XP VM with fixtures; look for a better way and for logical
functions to add to each item; write the proposal in
docs/Design/UI_SPEC.md (§34); ask me every decision the run needs in one
go, with a recommendation each, and record my answers in
docs/research/DECISIONS.md; then write the runbook and progress file, tag
the start (notes-run-start), and run it straight through. Every command
is one undo step, tested through the message path and the widget path,
and written as Ami Pro writes it (twins opened in Ami Pro). Measure with
the round-trip audit and layout_comparison_report before and after any
change that can move text, keep decompiler output and my corpus file names
out of Git, commit small tested slices, update the playbook's queue, and
hand off with a short ranked list. Use lower-cost subagents for routine
test runs and have them report the results back to you. Tests must never
open real desktop dialogs on my screen (see CLAUDE.md, the Runtime's
click_unrun).
```

## The table editing run (2026-10-24)

### What I need from you (ranked)

1. ~~The table editing physical check~~: **passed 2026-10-25** ("tables
   pass"), after one fix from first use: a right-click in a cell of a
   table selected as a frame showed only Frame Properties (`a46224d`; the
   right press now goes to the cell, and the menu starts with Table
   Properties…).
2. ~~The import's physical check~~: **accepted 2026-10-25**.
3. **Start the next run, drawing** (queue run 9), with the prompt below.

### What was done (commits since `table-edit-run-start`)

- `d0bb02e` The map (every Table command in the decompile, Ami Pro
  fixtures TE1–TE20), the proposal (`UI_SPEC.md` §32), your answers
  TD1–TD11, the runbook.
- `d7beef4` The model (`TableLayout`, `CellFormat`), native v3, SAM in and
  out as Ami Pro writes them, and `amipro_editing::tables` (every command,
  one undo step each).
- `9c42e6a` Layout: leaders, line colours, Center table on page, edited
  rows fitting their text; the row cache taken after the import's last
  pass.
- `96151d4` Cell selections and the table keys (Tab adds a row from the
  last cell; a click in a cell puts the caret there, as Ami Pro's).
- `aed8acd` The Table menu, Insert › Table…'s dialog, Table Properties in
  the rail.
- `6cab317` Grid lines and the ruler's table columns.
- `a40731a` Copy, Cut, Paste, and drag and drop of cells.
- `1cb3933` Twins in Ami Pro, a table always on a line of its own, the
  records.
- `a46224d` After the run: a right-click in a table's cell offers the
  table commands even with the table selected as a frame.

### Found on the way

- **Fills:** a cell's fill is F3 `0x200` with its colour, black too. The
  app had read F6 instead, so 1,557 cells of your corpus with black fills
  (headings in white text) showed unfilled; they now show black as in Ami
  Pro (TK1, VM TF0X, TF0Y).
- **A click in a table** selected the table's frame first; it now puts the
  caret in the cell, as Ami Pro does (TK6).
- **A narrow table** after text stood beside it; Ami Pro always puts it on
  a line of its own (TK11; two VM fixtures now where Ami Pro put them).
- **Center table on page** was read but not laid out; 27 of 38 changed
  table positions in the corpora now match Ami Pro exactly (TK5).

### Measures

- **The round-trip audit** equal to the run's start on all three corpora
  (user's 417 of 417, ISO 159 of 160, VM 313 of 313).
- **The layout comparison:** every summary equal to the start on both
  corpora; per file only centred tables' places moved (27 of 38 changed
  values now exact, 7 off by 3 twips, 3 worse), and in the VM folder two
  narrow tables now stand where Ami Pro put them (`target/table-edit-baseline/`).
- **Twins:** twelve, opened in Ami Pro 3.1 as made; ATE2–ATE11's cells and
  headers equal what Ami Pro wrote for the same changes.
- **Timings (release):** on a 100 × 5 table each command 2.4–3.2 ms
  median end to end with its undo 2.3–2.6 ms; typing in a cell 2.4 ms; Tab
  1.2 ms. README31 typing 1.1–1.3 ms median; the typing A/B against the
  run's start within the host's noise (two rounds disagree in direction).
- `tools/verify.sh --release --corpus` **PASS** at 1cb3933: core 476 + 11
  ignored; app 342, 115, 349 (52, 6, 52 ignored); release builds; ISO
  corpus gate PASS.

### Next steps (ranked, 2026-10-24)

1. **Drawing** (queue run 9, `docs/playbooks/GRAPHICS.md`): drawings are
   the most common picture kind in your corpus.
2. From this run, small and optional: Home, Home and End, End in a table
   (Ami Pro's row ends, TK7); a typed tab in a cell (Ami Pro swallows it);
   the Insert Table grid's squares draw as round buttons (libcosmic).
3. Still open from the import run: text beside two Wrap around frames with
   a gap; a picture placed inside a cell's frame.

### Prompt for the next session

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run, drawing (run
9), as a straight run by the playbook's recipe and docs/playbooks/GRAPHICS.md:
map the feature in the decompile first (AMIPRO.EXE, amidraw.dll,
drawman.dll; how drawings are stored, drawn, and edited) and confirm it in
the XP VM with fixtures; build on the picture frames, metafile drawing,
and Ami Pro's screen images that already exist; look for a better way and
for logical functions to add to each item; write the proposal in
docs/Design/UI_SPEC.md (§33); ask me every decision the run needs in one
go, with a recommendation each, and record my answers in
docs/research/DECISIONS.md; then write the runbook and progress file, tag
the start (drawing-run-start), and run it straight through. Every command
is one undo step, tested through the message path and the widget path,
and written as Ami Pro writes it (twins opened in Ami Pro). Measure with
the round-trip audit and layout_comparison_report before and after any
change that can move text, keep decompiler output and my corpus file names
out of Git, commit small tested slices, update the playbook's queue, and
hand off with a short ranked list. Use lower-cost subagents for routine
test runs and have them report the results back to you. Tests must never
open real desktop dialogs on my screen (see CLAUDE.md, the Runtime's
click_unrun).
```

## Earlier: the table-editing run's setup notes (2026-10-24)

### State

- **Export's physical check passed** (2026-10-24, in LibreOffice Writer:
  "Export works fine"). **Import's physical check is pending**
  (`docs/Design/UI_ACCEPTANCE.md`, "Import"); it can be done any time and
  does not block the table run.
- Master is clean and pushed; the last full gate (`tools/verify.sh
  --release --corpus`) passed at `9b9c3a0`. No branches besides `master`.
- The XP VM is powered off; add the transient share again when starting
  it (`docs/research/VM_LAB.md`, Session).

### What the table run starts from

- **Model, files, and layout already exist** for tables: `Table` (columns,
  rows, `CellMargins`, `TableSpacing` gutters, `TableRows`: Automatic,
  Rows span pages, headings, page-break rows; `read_rows`, the row cache),
  `TableCell` (span, frame, kept record with Lines & Color: `lines`,
  `fill`), the SAM reader and writer, native v3, the engine's grid and
  parts (`TableGrid`, `TablePart`), drawing of lines and fills on screen
  and paper, export and import of tables. **What is missing is every
  command**: no table can be made, and no row, column, size, span, line,
  or heading changed (`FEATURE_PARITY.md` TB-01; `CLAUDE.md`, "Feature
  gaps").
- `Command::SetTableRows` (run 2's) and Insert › Page Break in a cell are
  the only table commands; Headings and Modify Table Layout's Automatic
  and Rows span pages need only their commands and interface.
- Evidence so far: `docs/research/CELL_FRAME_NOTES.md` (the grid, cell
  records' values, frames in cells, the decompiled `1428:0000`/`046b`,
  `1490:07c0`, `14d0:0000`, `1488:0e10`/`14f0`), `TABLE_PAGES_NOTES.md`
  (headings, Rows span pages, page breaks, the row cache RC1–RC3),
  `EXPORT_NOTES.md` (Lines & Color: `TBLLINESPROC`, `13e8:1232`, VM TL1,
  TL4). Still to map: each Table menu command's handler and dialog in
  `AMIPRO.EXE` and `AMIPROUI.DLL` (`tools/ghidra/command_table.py` lists
  the menu command handlers), the fields each writes, and Ami Pro's
  limits.
- **A ready test set:** imported Word and LibreOffice tables (joined
  cells, row heights, lines, fills) through `import_files` and
  `import_twins` (`CLAUDE.md`, Build and test), besides both corpora's 400+
  tables.
- **Open findings to keep in mind** (from the import twins,
  `docs/IMPORT_PROGRESS.md`, log): a picture offset inside a cell's frame
  shows in the app and not in Ami Pro (AIMW); text beside two Wrap around
  frames with a gap needs a fixture (AIMO, layout, not tables).

### Decisions to expect (from the playbook; ask them all up front)

- Where the commands live: a Table Properties panel in the rail (table,
  row, column, cell) and Ami Pro's Table menu shown while the caret is in
  a table (recommended).
- The cell selection gestures (rectangular selection, Select Row,
  Column, Entire Table).
- Lines & Color in the rail with the colour panel (recommended).
- Whether to split the run into structure, then formatting.
- Additions to offer: Insert Row Above/Below and Column Left/Right, even
  distribution, table styles from Ami Pro's lines and shading, text to
  table and back, Repeat Headings by default for long tables.

### Prompt for the table editing session (used 2026-10-24)

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run, table editing
(run 8), as a straight run by the playbook's recipe and
docs/playbooks/TABLES.md ("Run 2: table-editing"): map every Table
command in the decompile first (AMIPRO.EXE's menu command table,
AMIPROUI.DLL's dialogs, the cell fields each command writes, Ami Pro's
limits) and confirm it in the XP VM with fixtures; build on the table
model, files, layout, and drawing that already exist (see the handoff's
"What the table run starts from"); look for a better way and for logical
functions to add to each item; write the proposal in
docs/Design/UI_SPEC.md (§32); ask me every decision the run needs in one
go, with a recommendation each, and record my answers in
docs/research/DECISIONS.md; then write the runbook and progress file,
tag the start (table-edit-run-start), and run it straight through. Every
command is one undo step, tested through the message path and the widget
path, and written as Ami Pro writes it (twins opened in Ami Pro). Measure
with the round-trip audit and layout_comparison_report before and after
any change that can move text, keep decompiler output and my corpus file
names out of Git, commit small tested slices, update the playbook's
queue, and hand off with a short ranked list. Use lower-cost subagents
for routine test runs and have them report the results back to you.
Tests must never open real desktop dialogs on my screen (see CLAUDE.md,
the Runtime's click_unrun).
```

## Earlier: the import run (2026-10-21 to 2026-10-24)

### What I need from you (ranked)

1. **The import's physical check** (about five minutes;
   `docs/Design/UI_ACCEPTANCE.md`, "Import"):
   - File › Open on a Word or LibreOffice document of your own: Import
     Options asks how to take the styles; it opens in a new tab, and Save
     goes to Save As.
   - The Import Report in the rail when something is different: Go to,
     Copy, ×, and "N import notes" in the status bar.
   - Insert › File… in the middle of a document; Undo takes it out.
   - Copy formatted paragraphs with a table in LibreOffice Writer and
     Paste here.
   - Drop a .docx or .odt on the page.
   - File › Import Folder… on a small folder into an empty one.
2. ~~The export's physical check in LibreOffice Writer~~: passed
   2026-10-24 ("Export works fine").
3. **Start the next run, table editing** (queue run 8), with the prompt
   above.

### What was done (commits since `import-run-start`)

- `7c34194` The map (VM fixtures IM1–IM3 opened with each Import Options
  choice, NT1; the decompiled `rtf.flt`, `AMIXIMP.DLL`, and the import
  engine), the proposal (`UI_SPEC.md` §31), your answers IM1–IM12, the
  runbook.
- `3253516` to `97e3b7a` In `amipro-exchange`: zip and XML readers of the
  project's own, `realise` (any exchange document into the model), and the
  text, RTF, DOCX, and ODT readers; the RTF and ODT writers improved on
  the way.
- `6952546` Enhanced metafiles (Word's drawings) drawn, printed,
  exported, and converted for Save As SAM; the round trips measured.
- `90f1a99` **The app**: File › Open for the four formats (by contents;
  Import Options; read off the interface's thread; a protected tab; the
  text-editor build too), Insert › File…, Paste of RTF with Apply styles,
  a dropped document opening, and **the Import Report** in the rail.
- `90e6d15` **File › Import Folder…** (a rail panel; `.amipro` documents;
  `Import report.txt`).
- `9b9c3a0` **Comparisons and twins**, and three fixes they found: Save As
  SAM wrote style names longer than Ami Pro's 13 characters, which Ami Pro
  then cannot find (now shortened and listed; Word's names often are);
  tables kept other programs' minimum row heights, which Ami Pro shrinks
  to the text; a cell's picture had no height.

### Measures

- **Round trips:** the app's exports of both corpora and the VM folder
  read back equal in what each format holds (VM / user / ISO): DOCX 269 /
  224 / 134, ODT 273 / 251 / 136, RTF 257 / 215 / 121 of 290 / 417 / 160.
- **The round-trip audit and the layout comparison** are equal to the
  run's start on all three corpora (`target/import-exit/measures/`).
- **Robustness:** 4,142 sample documents from other programs' test
  folders: 4,099 imported, 43 refused with their reason (damaged,
  password-protected, not a document), none panicked, the slowest 1.2 s.
- **LibreOffice against the app** (89 samples): the same page count for 82;
  most of the per-page differences come from fonts substituted
  differently (Calibri: Noto Sans there, Liberation Sans here).
- **Twins:** Ami Pro 3.1 opens the app's SAM of imported documents as the
  app shows them: Ami Pro's own RTF, README31 through DOCX (14 pages, the
  same line breaks), a Word list document (3 pages, the same line breaks),
  a comment (Ami Pro's note), and a LibreOffice table in a box.
- **Timings (release):** README31 read and realised in 2.6–3.6 ms (DOCX,
  ODT, RTF), laid out in 19 ms. Typing A/B: the normal build unchanged.
- `tools/verify.sh --release --corpus` **passed** at `9b9c3a0`: core 461 + 10 ignored; app 333, 115, 340 (50, 6, 50 ignored); release builds; ISO corpus gate PASS.

### Next steps (ranked, 2026-10-24)

1. The physical checks (above).
2. **Table editing** (queue run 8, `docs/playbooks/TABLES.md`): imported
   Word and LibreOffice tables make a ready test set.
3. For the layout, from the twins: text beside two Wrap around frames with
   a gap between them (Ami Pro puts empty paragraphs in the gap but the
   next text below both frames; the app sets text in the gap) needs a
   fixture; a picture placed inside a cell's frame shows here and not in
   Ami Pro.
4. The rich-editor prototype (feature-gated) types 13% slower than at the
   run's start (0.43 against 0.38 ms; nothing the run changed is on its
   path); the normal build is unchanged.

## Earlier: the export run (2026-10-20)

### What was done (commits since `export-run-start`)

- `eafc453`: the map, the proposal (`UI_SPEC.md` §30), your answers
  EX1–EX10, the runbook. Ami Pro's own RTF and Word exports (VM) keep the
  text, styles, tables, header, and pictures and drop frames with their
  text; its filters take one token stream, which the run copied: one
  exchange document, a filter per format.
- `69adc6c`, `011e76c`, `4e7170f`: `crates/exchange` and its DOCX, ODT, RTF,
  and text filters: Ami Pro's style names, sections for page layouts,
  headers and footers, frames as floating text boxes and pictures where
  the app's layout puts them, tables, live lists showing Ami Pro's numbers,
  fields with Ami Pro's results, index entries, Doc Info as properties.
- `f2ca77c`, `1447810`: File › Export ▸ in the app (the four formats, the
  loss dialog, Open in the status bar, the last folder remembered).
- `f18b590`: **table lines and shading** (Ami Pro's Lines & Color),
  mapped from the decompile and measured on Ami Pro's printouts, now drawn
  on screen and paper and exported. Tables printed without lines until
  now.
- `dd58899`: File › Export ▸ Export Folder… (a rail panel, a report).
- `1f9f74d`: from comparing LibreOffice's renderings of your documents
  with the app's: today's names for fonts like Tms Rmn and Helv, Ami
  Pro's printer line heights, and long tables kept in the text.
- `4da1556`: Ami Pro itself opens the app's RTF (its reader took `\b0`
  for bold on, so everything came out bold italic capitals until this fix).
- `2389ca5`: timings.

### Measures

- **Round trip and layout comparison:** unchanged for both corpora at the
  start (`target/export-baseline/`) and after the table lines
  (`target/export-ex9/`); the VM folder gained TL1 and TL4, read back
  equal.
- **Corpus export:** every SAM of your corpus (417), the ISO corpus (160),
  and the VM folder (282) exported in all four formats without failure;
  every DOCX and ODT package well-formed.
- **LibreOffice against the app's PDF** (70 sampled documents, DOCX): the
  same page count for 36 of 40 of yours and 21 of 30 ISO documents; pages
  with 95% of their words on the same page 103 of 173 and 97 of 231.
- **Timings (release):** README31 as DOCX 4.1 ms, ODT 5.2, RTF 2.3, text
  1.8.
- **Typing A/B against `export-run-start`:** no regression (two rounds;
  one first-round sample taken while LibreOffice ran on this host was not
  reproduced; `target/bench-ab/20261020-233342/`).
- **`tools/verify.sh --release --corpus`:** **passed** at `2c99cb1`: core 427 + 9
  ignored; app 318, 111, 325 (44, 4, 44 ignored); release builds; ISO
  corpus gate PASS.

### Next steps (ranked, 2026-10-20)

1. **Import** (queue run 7, `docs/playbooks/OUTPUT.md`): the same
   exchange document, read from each format; export then import must give
   back the original in what the format holds.
2. **Smaller gaps from the export:**
   - Ami Pro 3.1's RTF reader shows no PNG pictures; a WMF version of each
     picture (`\nonshppict`) would let it show them.
   - Ami Pro's RTF reader takes frames as paragraphs (their text kept).
   - LibreOffice hyphenates a few more words than the app; headers grow
     where Ami Pro hides what does not fit; rounded corners are square on
     frames holding text in ODT and RTF.
   - Notes, footnotes, and bookmarks are listed as losses until their runs
     (the notes run, 10).
3. **Table lines' dialog** (Lines & Color) belongs to the table-editing run
   (8); the lines are read and drawn already.

## Earlier: the print and PDF run (2026-10-20)

Its physical check passed (2026-10-20: "printing works correctly", "pdf
works correctly", the Print panel "looks good", no issues with Print
Current Page…); the Print panel's buttons span the rail since (your
choice). Its record is `docs/PRINT_PROGRESS.md`.

## Earlier: the fields run (2026-10-20)

Its physical check passed (2026-10-20: "Powerfields work").

### What it asked of you (ranked)

1. **Start the next run, print and PDF** (the queue's run 5,
   `docs/PHASE5_PLAYBOOK.md`), with the playbook's session prompt. The
   fields' physical check passed (2026-10-20: "Powerfields work").

### What was done (commits since `2feef48`)

- `b262e85` Your look at the find bar above the ruler: passed.
- `0883de4`, `2f159fc` The map (the decompiled `AMIPRO.EXE`,
  `AMIPROUI.DLL`, and, decompiled for this run, `AMIMACRO.DLL`; the guide
  ch. 5, 23, 32; VM fixtures FD1–FD6), the proposal (`UI_SPEC.md` §28),
  your answers FL1–FL7, the runbook; tag `fields-run-start`.
- `e5d2520`, `afddd7b` **Dates on the page**: System date and time, Date
  of last revision, Date created in Ami Pro's 21 formats (FD4 on a 2005
  date), the time with Ami Pro's leading space; Doc Info entries.
- `b895ba4`, `986627d` **Doc Info** in the model (values 2–5 of `[desc]`
  are fields 1–4, not keywords as our notes said); **saves in Ami Pro's
  time base**: Ami Pro reads stored times as US Pacific time, so the
  app's save times were 4–8 hours off in Ami Pro until now; editing
  minutes added; statistics counted as Ami Pro counts them.
- `0beae64`, `6f8d74b`, `e2c6cda` **Power fields**: Ami Pro's marks kept,
  results as text; the language (FD7 settled the numbers: 3 / 2 is 1.50,
  10 / 3 is 3.3333333333333330); 45 of Ami Pro's own results reproduced;
  Update, Update All, Auto run, Lock, Seq, Include, Query$ and Decide.
- `642ce00`, and after it: **the interface**: the Fields panel and
  Document Properties in the rail, Show Power Fields, Field Shading, the
  commands (F9, Ctrl+F9, Alt+F9, Alt+Enter), the right-click menu on a
  field.
- **Twin AFD1**: the app's file opens in Ami Pro showing every field, the
  times, the renamed field, and the counts as the app does; Ami Pro's
  Update and Update All agree (one flag it sets on NumPages was added).
- `145304e`, `27dbae8` Timings; the caret's field looked up from the
  fields of the last change (the first A/B showed 0.1 ms more an edit).
- After the run, at your word: **codes typed over** as in Ami Pro (Show
  Power Fields sets each result aside in the document and shows its code
  in red in the text; off reads the codes back; Update reads them back
  first; Update All and saves switch them off; every document of the
  three corpora shows and hides back to itself), and **Edit › Power
  Fields ▸** (Update Fields, Update All Fields, Next Field, Previous
  Field, Edit Field…, Lock Field; Show Power Fields stays in View, as in
  Ami Pro).

### Measures

Round trip and layout comparison: unchanged in every summary of both
corpora; the VM folder gained this run's fixtures, all read back equal
and their cached lines at Ami Pro's (`target/fields-exit/`: user's corpus 417/417 read back equal, page counts 315 of 394, page ends 762 of 1,108; ISO 159/160, 69 of 160, 478 of 1,307; VM folder 280/280, 279 of 279 page counts, 111 of 121 page ends).

Timings (release, to the drawn frame, README31 with 100 fields): a
keystroke 1.36 ms median, Update All 22.9 ms, its undo 11.7 ms, Show
Power Fields on and off 39.5 ms. Without fields the fields' work after a
change is 24 µs. Typing A/B against `fields-run-start` after `27dbae8`:
the two builds overlap within this host's noise (the default build at
most 0.05–0.09 ms at the fixture's start; the rich editor no slower).

`tools/verify.sh --release --corpus` **passed**: core 404 + 8 ignored; app 289, 111, 296 (36, 4, 36 ignored); release builds; ISO corpus gate PASS.

### Next steps (ranked, 2026-10-20)

1. The fields' physical check (above).
2. **Print and PDF** (queue run 5, `docs/playbooks/OUTPUT.md`): Print
   Options › Update fields belongs there (Update All before printing).
3. For the notes run: bookmarks make Bookmark, PageRef, and GetBookMark
   work out (they keep their results now); for the merge run, merge
   fields get their records through `FieldHost`.
4. The find bar's keystroke cost, if it is ever felt.

## Earlier: the Find & Replace run (2026-10-20)

The find bar's physical check passed, and your look at its new place
above the ruler passed the same day. Its measures and timings are in
`docs/FIND_PROGRESS.md`.

## Answers received (2026-10-19)

The user took every recommendation below ("all of this sounds good";
`docs/research/DECISIONS.md`, "The morning report's answers"): TP1–TP5
confirmed; F1–F4 as recommended (`UI_SPEC.md` §27 approved); merge
`overnight-wip-strips`; build the row cache, measured before merging.
Since then: the strips branch is merged (`f6ee32f`, at the user's word),
and **the tables across pages physical check passed** (2026-10-19; the
checklist's page 3 heading corrected to R40 first). **Next:** the row
cache, then Find & Replace (the prompt below).

`tools/verify.sh --release --corpus` **passed** on the merge `f6ee32f`:
core 350 + 4 ignored (one more, the strips rule test); app 274, 108, 281
(31, 4, 31 ignored); release builds; ISO corpus audit gate.

## Morning report (2026-10-19)

### What I need from you (ranked)

1. **Confirm the five decisions I took overnight** (TP1–TP5,
   `docs/research/DECISIONS.md`; each Ami Pro's behaviour, cheap to
   change):
   - **TP1** Insert › Page Break with the caret in a table cell starts
     that row on the next page (Ami Pro's page break in a table), one undo
     step, checked while the row has one; choosing it again takes it
     away. Alternative: a Breaks dialog with Insert and Remove.
   - **TP2** Headings, Rows span pages, and Automatic are laid out, kept,
     and written; their commands (Table › Headings, Modify Table Layout)
     come with the table editing run.
   - **TP3** A row taller than a page is cut to the page (Ami Pro's),
     unless Rows span pages.
   - **TP4** The caret goes into a repeated heading on a later page and
     edits the heading row (Ami Pro's).
   - **TP5** A table in a frame shows the rows its frame holds (Ami
     Pro's).
2. **Find & Replace, run 3** (`docs/Design/UI_SPEC.md` §27,
   `docs/research/FIND_NOTES.md`), four questions:
   - **F1 The form.** Recommend a find bar over the pages (Find, Replace,
     ▲ ▼, "3 of 12", Options ▾, Replace, Replace All; Ctrl+F, Ctrl+H, F3,
     Shift+F3, Esc). Alternatives: a panel in the rail; Ami Pro's four
     dialogs.
   - **F2 Defaults.** Recommend Ami Pro's (Whole word only and the other
     text streams on), remembered between sessions.
   - **F3 Capitals when replacing.** Recommend Ami Pro's rule by default
     (the replacement follows the found text's capitals: "Status bar" →
     "Window Pane"), with a switch for "as typed".
   - **F4 Improvements.** Recommend matches highlighted with a count,
     Replace All as one undo step (Ami Pro cannot undo it, FR4), and recent
     searches; not regular expressions yet.
3. **Text on both sides of a frame** (branch `overnight-wip-strips`,
   local, not pushed). Ami Pro fills the part left of a Wrap around frame
   down its band, then the part right of it, then goes on below (VM
   WB1–WB3 and page 4 of one of your manuals). Built and measured: page
   counts 312 → 315 and one file's page ends 5 → 10, but one long manual
   already 16 pages longer than Ami Pro's loses 20 page ends, so the
   total falls 777 → 762 and the rulebook kept it off master. Recommend
   merging (`git merge overnight-wip-strips`): the rule is Ami Pro's, and
   the loss is in a file whose pagination is already far off.
4. **Rows measured taller than Ami Pro's.** 180 of your 2,777 rows still
   differ from Ami Pro's stored heights, most of them taller: multi-line cells in
   faces your main text never calibrates (Times New Roman 11 pt, Book
   Antiqua 12 pt: the substitutes' lines run 5–7 twips taller). Recommend
   trusting Ami Pro's stored row height while a row's cells are as read
   (a cache, like the paragraph caches the writer keeps), measured before
   it is merged. Alternative: leave the measurement as it is.
5. **A physical check of about five minutes** (below).

### What was done (commits since `table-pages-run-start`)

- `5d4e67e` The map (`docs/research/TABLE_PAGES_NOTES.md`: the user's
  guide ch. 16, the decompiled row records and page table, VM TP1–TP11),
  §26, TP1–TP5, the runbook, the baseline.
- `0800591` Automatic, Rows span pages, heading rows, and page-break rows
  in the model, native v3, the importer, and the SAM writer.
- `26004bb` Tables continue across columns and pages in parts: whole
  rows with each part's inset, headings repeated with one more inset,
  page breaks on rows, Rows span pages at lines, rows cut to a page,
  Automatic, frame tables; page tables on a line of their own split too;
  a cell edit never reuses a page holding its table.
- `e892e98` Empty cells need no height; the comparison prints differing
  rows.
- `8b357e6` The page surface: parts drawn (no grid line at a split),
  clicks and the caret in every part and in repeated headings; Insert ›
  Page Break in a cell.
- `24cd585` The comparison against Ami Pro's page table.
- `8d12c02` Twins ATP2, ATP5, ATP6: Ami Pro pages them as its own.
- `fd817a5` Both formats; timings; the typing A/B (no regression).
- `e92438a` The exit records.
- `06debca` Fill-in: With para above frames narrow beside Wrap around
  frames (VM FC7, FC7E, FC7G; both corpora unchanged, the four FC7
  fixtures now at Ami Pro's place and size).
- `43636e8` Find & Replace prepared (not built).
- `c666f39` Fill-in recorded: column balance with uneven columns parked
  (6 layouts in both corpora, all within 90 twips of even); strips on the
  branch.

### Measures (before → after, `target/tp-baseline/` → `target/tp-fc7/`)

| | User's corpus | ISO corpus | VM folder |
|---|---|---|---|
| Round trip, read back equal | 417 → 417 of 417 | 159 → 159 of 160 | 256 of 256 → 266 of 266 (new fixtures) |
| Page counts equal | 302 → 312 of 394 | 65 → 68 of 160 | 247 of 255 → 265 of 265 |
| Page ends on Ami Pro's page | 760 → 777 of 1,108 | 492 → 474 of 1,307 | 99 of 113 → 108 of 118 |
| Page-end tables with Ami Pro's rows | 69 of 106 | 18 of 106 | 45 of 46 |
| Rows at Ami Pro's stored height | 2,597 of 2,777 | 666 of 2,056 | 629 of 630 |

The ISO corpus's 18 lost page ends are all in files whose style sheets
(smartext.sty, pro21.sty) are not on the ISO, so their cells' styles are
fallbacks; the table used to keep Ami Pro's stored height there.
`docs/research/LAYOUT_COMPARISON.md`, "Tables across pages", explains the
rest. Timings (release, to the drawn frame): typing in a continued table
1.1–1.3 ms, a page break in a row 1.0–2.8 ms; the interleaved typing A/B
against the run's start: no regression. Tests: core 349 + 4 ignored; app
274, 108, 281 (31, 4, 31 ignored); `tools/verify.sh --release --corpus`
PASS; every VM twin test passes.

### Blockers, branches, and notes

- No blocker. One local branch: `overnight-wip-strips` (above).
- Not chased: the headless screenshot harness stays on the last page
  whatever the caret or Next Page does (the table screenshots show page 3
  of 3; enough for light and dark).
- Found for later: a document whose caches hold no one-line paragraph
  keeps the font's own line height (276 against Ami Pro's 284 in WB1S);
  multi-line caches with a clear line count could calibrate it.

### Physical check (about five minutes)

`docs/Design/UI_ACCEPTANCE.md`, "Tables across pages": open
`../AmiPro-Reference/vm-lab/OUT/TP2BS.SAM` (headings on pages 2–4), type in
a cell on page 3 and in the repeated heading on page 2 (Ctrl+Z after
each), Insert › Page Break in row 20 and again, open `TP5.SAM` (a row split
between pages without a line at the split), and the dark theme.

## Next steps (ranked, 2026-10-19)

1. Your answers to F1–F4, then **Find & Replace** (run 3) as a straight
   run from the prepared map and proposal.
2. The tables across pages physical check, and TP1–TP5 confirmed.
3. The strips branch and the row cache (questions 3 and 4 above).
4. Fields (run 4): `<:D…>` date marks in 43 of your files.

### Prompt for the next session (after the tables across pages run, used 2026-10-20)

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. My answers to the morning report's questions are
below. Record them in docs/research/DECISIONS.md (confirming or changing
TP1-TP5), act on the strips branch and the row cache as I answered, then
set up Find & Replace (the queue's NEXT run) as a straight run from
docs/research/FIND_NOTES.md and UI_SPEC.md section 27 with my answers to
F1-F4: write the runbook and progress file, tag the start, and run it
through by the playbook's recipe, measuring, committing small tested
slices, and handing off with a short ranked list.
Answers (2026-10-19, already recorded in DECISIONS.md): TP1-TP5 confirmed;
F1-F4 as recommended; overnight-wip-strips merged (f6ee32f); the tables
across pages physical check passed; build the row cache and measure it
before merging.
```

## Overnight rulebook (2026-10-18)

You asked for a rulebook to run overnight without you. A session started
with the prompt at the end of this section follows it. Start Claude Code
in auto mode, so that it does not wait at a permission prompt.

### What to run, in order

1. **`table-pages`**, the queue's run 2 (`docs/playbooks/TABLES.md`, "Run
   1"), by the playbook's recipe (`docs/PHASE5_PLAYBOOK.md`, "The recipe
   for every run"), with the changes below. Its playbook expects no
   decisions beyond Ami Pro's behaviour, which is why it suits a night.
   Tag `table-pages-run-start`; runbook `docs/TABLE_PAGES_RUNBOOK.md`,
   progress `docs/TABLE_PAGES_PROGRESS.md`; decision prefix `TP`.
2. **If it finishes with time left**, fill-in work that needs no decision,
   one at a time, each measured and committed: anchored frames beside
   Wrap around frames; column balance with unequal columns
   (`docs/RULER_PROGRESS.md`); then the open findings in
   `docs/PHASE4_PROGRESS.md`'s log.
3. **Then prepare, but do not build, run 3 (`find`, Find & Replace)**:
   map it in the decompile, make its VM fixtures, write the `UI_SPEC.md`
   proposal, and write its questions, each with a recommendation, under
   "What I need from you". Its interface is your decision, so it waits
   for the morning.

### Decisions while you sleep

- Never wait on a question. Recipe step 7 (ask every decision in one go)
  becomes: take the recommendation, record it in
  `docs/research/DECISIONS.md` as **"taken overnight by the rulebook;
  to confirm"**, and list it in the morning report. Prefer the choice
  that keeps Ami Pro's behaviour and its file format, and build it so
  that another answer is cheap to switch to.
- A **material** decision is not taken: anything Ami Pro's files cannot
  hold, a change to the approved interface (`UI_SPEC.md`), a new
  dependency or licence, or anything hard to undo. Park it in the morning
  report with a recommendation and continue with other slices.
- A failing test or an unclear rule is not a decision: investigate the
  decompile and the VM, fix, and retry (AGENTS.md).

### Allowed

Everything AGENTS.md and CLAUDE.md already allow: decompiling, the XP VM
(Ami Pro, fixtures, snapshots; restore `Clean baseline (before
AmiPro-Linux lab)` when the guest is in doubt), reading both corpora,
builds, tests, benchmarks, and small commits to `master`.

### Not allowed

- Destructive Git: no `reset --hard`, force push, rebase or amend of
  pushed commits, moving or deleting tags, or deleting branches.
- Changing the repository's visibility; pushing before the end.
- Committing decompiler output, corpus files or their names, VM
  screenshots, or anything under `Reference/` or `../AmiPro-Reference/`.
- Touching `AGENTS.md.backup` or the `Windows11` VM; running reference
  binaries on the host.
- New dependencies or downloads (the run needs none); deleting anything
  but the regenerable caches named below.

### Timeboxes and stop rules

- **One problem gets 90 minutes.** After that, record it as a blocker in
  the progress file, and if `master` is not green, move the unfinished
  work to a branch (`git switch -c overnight-wip-SLICE`, commit, `git
  switch master`; nothing is discarded). Then continue with the next
  independent slice.
- **Text that moves is measured.** Before and after any change that can
  move text: `layout_comparison_report` on both corpora and the VM
  folder, and the round-trip audit (CLAUDE.md, "Build and test"). Every
  row that changes is explained. A change that makes page ends or row
  heights worse overall, and cannot be fixed within its timebox, is not
  merged.
- **Each slice:** targeted tests, strict Clippy, a commit. **Before the
  report:** `tools/verify.sh --release --corpus`, the twins in the VM,
  and an interleaved typing A/B (`tools/bench-ab.sh
  table-pages-run-start`) if the run touched typing's path.
- **Disk:** check `df -h /` before long builds; under 10 GB free, remove
  only `app/cosmic/target/debug/incremental`, `target/bench-ab`, and
  `target/bisect`.
- **Stop for good** only when the work above is done, or for something
  that truly needs you (AGENTS.md: a missing reference, a destructive
  action, a licence, a material decision). Write the report even then.

### The morning report

At the top of this file, replacing "What I need from you": the commits
and what each did; measures before and after (round trip, page counts,
page ends, row heights against Ami Pro's); the decisions taken overnight,
to confirm; parked questions with recommendations; blockers and WIP
branches; and a physical check of a few minutes. Then update the records
as the recipe's exit step lists, commit, push with
`git -c credential.helper= -c credential.helper='!gh auth git-credential' push origin master`
(a plain `git push` could not find the credentials on 2026-10-18), and
confirm with `git ls-remote origin master` that it equals `HEAD`.

### Prompt for the overnight session

```text
Overnight run. Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md
(especially "Overnight rulebook"), PROJECT_PLAN.md, docs/PHASE5_PLAYBOOK.md,
and docs/playbooks/TABLES.md. I am asleep: follow the overnight rulebook
exactly. Run table-pages by the playbook's recipe, taking the recommended
choice for any decision and recording it as taken overnight, never
waiting on a question. Then do the fill-in work and prepare run 3 as the
rulebook says. Measure before and after anything that moves text, keep
private material and corpus file names out of Git, commit small tested
slices, and finish with the morning report, the records, and a push.
```

## The pictures run (2026-10-18): what was done

- **PI.1** (setup `511a9b4`): the decompile and the VM mapped Ami Pro's
  pictures (`docs/research/PICTURE_NOTES.md`); `UI_SPEC.md` §25; P1–P8.
- **PI.2** (`b7f0ca9`): scaling (Original, Fit in frame, Percentage,
  Custom; aspect; percent; size; rotation), the pan, and a description in
  the model and native v3; five commands. Found on the way: Insert Layout
  left a repeating table's cell pictures behind.
- **PI.3** (`650ec4a`): Ami Pro's `[isd]` written back as read while
  current, else rebuilt keeping the driver's state; the loss check names
  descriptions, conversions, and Linux links. Your corpus, the ISO's, and
  the VM folder are written byte for byte as before the run.
- **PI.4** (`9ab8ed7`): the core crate `amipro-pictures` (Windows bitmaps,
  metafiles as a display list, Ami Pro's screen image): every stored
  bitmap and metafile in both corpora reads.
- **PI.5** (`808afcd`): pictures drawn in their frames, clipped, under
  their lines; metafiles sharp at every zoom; PNG, JPEG, GIF, TIFF, PCX,
  SVG; drawings, charts, and OLE objects from Ami Pro's screen image (138
  of your files' 424 such pictures) or as boxes. Fixed on the way: text
  beside a Flow with text frame was centred in its line, now on the
  baseline; some 16-bit screen images are 5-5-5.
- **PI.6** (`9495af2`): Insert › Image… (Ctrl+Shift+I), copy or link.
- **PI.7** (`9aa5919`): Frame Properties › Picture; Format › Frame ▸
  Graphics Scaling…; rotation mapped in the VM (clockwise, the turned
  picture stretched onto its rectangle).
- **PI.8** (`617d0da`): picture mode.
- **PI.9** (`e2ff3df`): paste and drop.
- **PI.10** (`4729431`): Save As SAM converts other formats to bitmaps
  (300 dpi at their size) and copies Linux links in; twelve twins open in
  Ami Pro with their pictures where the app put them.
- **PI.11**: both formats read back; timings (release, to the drawn
  frame): a page's pictures decoded 0.5–0.7 ms on a warm page, the
  heaviest metafile (5,787 fills) 7.2 ms once per size, 0.55 ms redrawn;
  typing beside a picture 0.87 ms; a picture mode step 0.91 ms.
  Typing A/B against the run's start: the default editor unchanged; the
  rich editor prototype 0.05–0.07 ms slower an edit, bisected to PI.2,
  where no step does more work (code placement; progress log).
- **Measures:** round trip unchanged; layout comparison unchanged (the
  run moves no text). See the progress file's log.
- **Housekeeping:** the disk filled during PI.6 (222 MB free); I removed
  only regenerable build caches (the app's incremental cache, old A/B and
  comparison builds; 24 GB). About 20 GB are free now.

## Next steps after the pictures run (history)

1. **Tables across pages** (the queue's run 2, `docs/playbooks/TABLES.md`),
   with cell text measured taller than Ami Pro's rows (one of your tables
   by 75 twips).
2. Find & Replace (run 3), then fields (run 4).
3. Open findings: what Insert › Image… does with the caret in a header,
   footer, or frame's text in Ami Pro (the app refuses with a message);
   eight-bit screen images (the writing screen's palette); the picture
   area's exact clamp (the frame's interior is used); anchored frames
   beside Wrap around frames; column balance with unequal columns.
4. Drawings drawn from their objects and Draw mode (run 9): 380 drawings
   in 107 of your files, 168 shown as boxes today.

### Prompt for the next session (after the pictures run, history)

The same prompt serves every Phase 5 session; the playbook's queue says
which run is next (tables across pages).

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run as a straight
run by the playbook's recipe and its own playbook in docs/playbooks/: map
the feature in the decompile first and confirm it in the XP VM with
fixtures; choose maintained, suitably licensed libraries where the run
needs one; look for a better way and for logical functions to add to each
item; write the proposal in docs/Design/UI_SPEC.md if the interface
changes; ask me every decision the run needs in one go, with a
recommendation each, and record my answers in docs/research/DECISIONS.md;
then write the runbook and progress file, tag the start, and run it
straight through. Measure with the round-trip audit and
layout_comparison_report before and after any change that can move text,
keep decompiler output and my corpus file names out of Git, commit small
tested slices, update the playbook's queue, and hand off with a short
ranked list.
```

## Before the pictures run (history)


**Updated:** 2026-10-18, after the frames in table cells run's physical
check passed (the run: `docs/CELL_FRAMES_PROGRESS.md`, from tag
`cells-run-start` to the handoff commit). `PROJECT_PLAN.md` is
authoritative for sequencing.

You asked to set up frames in table cells as a straight run; you answered
T1–T3 with the three recommendations, and the run went straight through.

## What I needed from you after the cells run (history; answered 2026-10-18)

1. ~~One physical check~~ **PASS (2026-10-18)**: README31's page 12 with
   the pictures in their cells, a click putting the caret in the outlined
   cell with the status note, Delete and Ctrl+Z, and a page break taking
   the table and its pictures to page 13. You: "that all works".
2. To start the next run, paste "Prompt for the next session" below. Phase
   5 now runs from `docs/PHASE5_PLAYBOOK.md` (written 2026-10-18 at your
   request): seven playbooks in `docs/playbooks/` and a queue of 23
   straight runs, pictures first; importing the format chosen for export
   follows the export run. Reorder the queue whenever you like.
   Nothing else to decide.

## The frames in table cells run (2026-10-18): what was done

- **CF.1** (setup commit `49bc725`): the decompile maps it all: Ami Pro
  keeps frames in cells in a list of its own (`1248:0347`), written first
  (`11c8:3ba0`); a cell record's values 11 and 12 say the cell holds a
  frame and which (its place in that list, `1490:07c0`, `1428:07ce`);
  putting a picture in a cell (`14d0:0000`); the caret entering a picture's
  cell and its outline (`1488:0e10`, `14f0`). (`1238:0ca4` turned out to
  test a line's bit, not a frame's.) VM fixtures TF1–TF15 and variants;
  `docs/research/CELL_FRAME_NOTES.md`; `UI_SPEC.md` §24; your T1–T3.
- **CF.2** (`09d49e4`): tables on Ami Pro's grid. The VM showed Ami Pro
  puts the first column's text 43 twips in whatever the gutter, then each
  column's width and whole gutter, with insets and per-row and per-column
  gutters stored in the file; the app had split each gutter in half. A
  table's natural height now equals Ami Pro's stored one in 159 of 389 of
  your tables (58 before). Fixed on the way: a row or column record the
  writer had to add had two values, which Ami Pro's reader rejects.
- **CF.3–CF.4** (`4caffdc`): pictures in cells in the model, native v3,
  the importer, and the SAM writer (Ami Pro's order and numbers; the ten
  pictures in two of your files that Ami Pro never sized are written at
  their cells' size, T2).
- **CF.5** (`ec92c33`): laid out over their cells, right above their
  tables, following them to other pages and growing with their rows: 76 of
  the 94 in your files exactly where Ami Pro's stored rectangles put them
  in their tables (the rest: the ten never sized, rows on a later page,
  and a few rows whose text measures taller here).
- **CF.6** (`070db7f`): on the page, as T1: a click puts the caret in the
  cell, outlined as Ami Pro outlines it; Delete and Edit › Delete remove
  the picture as one undo step (Ami Pro cannot undo it); typing, Enter,
  and Backspace do nothing there, and the status bar says so.
- **CF.7** (`b5fab55`): twins: the app's versions of TF2, TF3, TF7, TF10,
  and README31 with a page break open in Ami Pro with the pictures in
  their cells; Ami Pro's own re-save of the TF3 twin is its TF3.
- **CF.8** (`53a7430`): both formats read back with nothing in the loss
  check; README31 to the drawn frame: typing beside a picture 0.64 ms,
  Delete 1.9 ms, a page break before the table 0.9 ms.
- **Measures:** read back equal unchanged (your 417/417, ISO 159/160, VM
  224/224). Layout comparison: page counts 303 → 302 and page ends 761 →
  760 of 1,108 in your corpus, explained in the progress log: the grid is
  right (natural heights now equal Ami Pro's), and two files lost page ends
  because the old, wrong heights had fallen back on Ami Pro's stored
  height, hiding that cell text measures taller here; one file gained
  four. Typing: 0.02–0.04 ms slower in the update phase in interleaved A/B,
  likely code placement, far below a frame.
- **Found and fixed besides:** your files' pictures in cells had been Wrap
  around page frames at stale places, pushing their own tables aside.
- **Lab:** fixtures TF1–TF15, TF14X/TF14XS, TF1Y/TF1YS/TF1YT, TF1Z/TF1ZS,
  BPS, ATF3S in `../AmiPro-Reference/vm-lab/OUT`; twins `ATF*.SAM`,
  `ARM31.SAM` in `IN`; screenshots `tf*`, `bp-*`, `atf*`, `arm31-*`. The
  VM is saved with Ami Pro open and no document changed.

## Next steps after the cells run (history)

1. ~~Your physical check above~~ (passed 2026-10-18).
2. **Pictures (F4)**, the Phase 5 queue's first run
   (`docs/playbooks/GRAPHICS.md`): Insert › Image… with picture decoding;
   the grey boxes become pictures, and Insert › Image… with the caret in
   an empty cell puts the picture in the cell (T3).
3. Cell text measured taller than Ami Pro's rows (one of your tables by 75
   twips; READMEBP's long cells): the grid is right now, so this is the
   next table fidelity step.
4. Tables that continue on later pages (80 of 220 corpus tables; six
   pictures in cells sit in such rows).
5. Open findings from earlier runs: anchored frames beside Wrap around
   frames; column balance with unequal columns; text on both sides of a
   Wrap around frame.
6. Table editing (rows, columns, sizes, joining cells, moving pictures
   between cells), with table formatting, in Phase 5.

### Prompt for the next session

The same prompt serves every Phase 5 session; the playbook's queue says
which run is next (pictures, then tables across pages, then Find &
Replace, …). The pictures run's specifics (File › Import Picture, the
filters, Copy image, Graphics Scaling, the frame at the caret and in an
empty cell, `1260:0509`, `14d0:05ec`, `[isd]`, `[Embedded]`, Ami Pro's
rendering) are in `docs/playbooks/GRAPHICS.md`.

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md, and
docs/PHASE5_PLAYBOOK.md. Then set up the queue's NEXT run as a straight
run by the playbook's recipe and its own playbook in docs/playbooks/: map
the feature in the decompile first and confirm it in the XP VM with
fixtures; choose maintained, suitably licensed libraries where the run
needs one; look for a better way and for logical functions to add to each
item; write the proposal in docs/Design/UI_SPEC.md if the interface
changes; ask me every decision the run needs in one go, with a
recommendation each, and record my answers in docs/research/DECISIONS.md;
then write the runbook and progress file, tag the start, and run it
straight through. Measure with the round-trip audit and
layout_comparison_report before and after any change that can move text,
keep decompiler output and my corpus file names out of Git, commit small
tested slices, update the playbook's queue, and hand off with a short
ranked list.
```

## The colour selection run (2026-10-18): what was done

- **CS.1:** Ami Pro's colour controls mapped (`AMIUICNT.DLL`'s colour bar
  and palette, the dialogs that use them, Modify Frame Layout's four; VM:
  the 15 colours and the ▼ grid of 120; `docs/research/COLOUR_NOTES.md`);
  `UI_SPEC.md` §23; your answers C1–C4 (C2 and C4 as you left to me: Ami
  Pro's 15, In this document with every kind together, one Recent list).
- **CS.2–CS.5** (`8d964a1`): `colours.rs` and `colour_control.rs`; every
  colour through one button and panel: the Font dialog, Style Properties'
  text and rules, Page Layout Properties' lines, and Frame Properties'
  line, shadow, background, and line between columns (its own colour
  now); a picker drag is one undo step. Found and fixed: a frame's line
  between columns in a style other than Ami Pro's frame default made Save
  As list the frame as a loss.
- **CS.6** (`ecdae67`): twin ACS1, made in the app with mixed colours
  everywhere; Ami Pro draws each colour exactly (sampled pixels).
- **CS.7:** mixed colours read back from SAM and native; Recent kept in
  the settings; round trip and layout comparison unchanged (417/417,
  159/160, 208/208; identical row for row); typing A/B in
  `docs/COLOUR_PROGRESS.md`.

## Next steps

1. ~~Your physical check of the colour buttons~~ (passed 2026-10-18).
2. Frames in table cells (flag 0x20; `PROJECT_PLAN.md`), the next run;
   its prompt is "Prompt for the next session" under the outline levels
   run's next steps, below (still unused).
3. Not built in this run: table lines and shading (Ami Pro's Table › Lines
   & Color, with table formatting), and a style of its own for a frame's
   line between columns (Ami Pro's Columns panel has one; the app copies
   the frame line's).

## After the run: typing that seemed to stop (2026-10-18)

You reported that in a new document, after Numbering, a line, and Enter,
typing stopped. The click near the top of the page had put the caret in
the **header** band, which holds one line of the app's Body Text. Enter
made a second header line that does not fit, which is hidden as Ami Pro
hides it (V14), and the caret was drawn nowhere, so typing looked dead
(it went into the hidden line). Ami Pro (VM HDR1, HDR2) keeps the caret
at the end of the band's last visible line. Fixed: the caret now does the
same; Up, Home, and End return to that line; the ruler keeps the header's
band; the status bar says "This text doesn't fit in the header, so it's
hidden; a larger top margin makes room"; and the style field and Styles
panel now follow the caret into headers, footers, frames, and cells
(they showed the main text's style). Numbering was not involved.

Two questions it raised, both answered yes and done:

- **New documents' Body Text has no space below**, as Ami Pro's
  `_DEFAULT.STY` (it had 6 pt, the project's own choice since Phase 3), so
  a new document's header band holds two lines, as in Ami Pro. Imported
  documents are unchanged (round trip and layout comparison identical
  before and after on all three folders).
- **The page margins are shaded**, as Ami Pro's Layout mode shows them
  (grey 223, the text area white, the gap between columns white), in both
  themes, so the header band is visible.

**Physical check: PASS (2026-10-18).** You saw grey margins and a white
text area, typed two lines in the top margin, and saw the third hidden
with the caret at the end of the second and the status bar saying why;
you also numbered a list and promoted and demoted it with the toolbar's
buttons. Still open from the list below: Alt+Right and Alt+Left, Enter
ending a list, Bullets and Insert › Bullet…, Outline Numbering…, the
icons in dark, and whether the Format menu is short enough.

**Updated:** 2026-10-18, at the end of the outline levels run
(`docs/OUTLINE_PROGRESS.md`, from tag `outline-run-start` to the handoff
commit), which followed the bullets and numbering run of 2026-10-17.
`PROJECT_PLAN.md` is authoritative for sequencing.

You asked how a numbered list moves between levels (1, 2, 3, then A, B,
C, then i, ii, iii), and said to set it up as the next run; you answered
O1–O4 before bed and remarked that the Format menu was getting tall. The
run went straight through overnight.

## What I needed from you after the outline levels run (history; all answered 2026-10-18)

1. **One physical check of lists and levels** (about eight minutes; it
   covers the bullets run's check, still open, and this run's;
   `docs/Design/UI_ACCEPTANCE.md`, "Bullets and numbering" and "Outline
   levels"). In the normal build (`cargo run --offline --manifest-path
   app/cosmic/Cargo.toml`), in a new document:
   - Type five short lines, select the first four, and press the
     Numbering button (or Ctrl+Shift+N): 1. to 4., lined up on the right.
   - Caret in the second line, Alt+Right: it becomes "A." under "1.", a
     step further in, and the rest renumber (2., 3.); Alt+Right again:
     "i."; Alt+Left twice brings it back; Ctrl+Z steps back. The new
     toolbar buttons beside Numbering do the same. Does it feel right, and
     are the indents comfortable?
   - Enter at the end of a numbered line gives the next number; Enter on
     the empty line ends the list. Bullets (Ctrl+Shift+L) on a few lines:
     diamonds; Insert › Bullet… puts one bullet in the text.
   - Open Format: Alignment ▸, Bullets and Numbering ▸, and Frame ▸ are
     now submenus. Is the menu short enough? (The menu bar cannot be
     opened in the headless harness.)
   - Format › Styles ▸ Outline Numbering…: pick "1  1.1  1.1.1" and Apply;
     the list shows 1, 1.1, 1.1.1; Ctrl+Z undoes it in one step.
   - The four list icons in light and dark.
2. **One question:** should bullets nest as numbered lists now do (Alt+Right
   on a bulleted line gives a second-level bullet, a different shape,
   further in)? I recommend **yes**, in a short slice: styles "Bullet 2" …
   made as needed, the shapes going diamond, dot, small square. Today a
   bulleted line moves by Ami Pro's outline rule, as in Ami Pro's Outline
   mode: Alt+Right can give it a heading's style (Title, Subhead) or an
   Outline# copy, which is surprising in a list.
3. **One offer:** Ami Pro has a top-level Frame menu (File Edit View Text
   Style Page Frame Tools Window Help). Format › Frame ▸ could become a
   top-level Frame menu with Insert's Frame… and Draw Frame beside the
   frame commands. I recommend keeping the fold unless the submenu feels
   slow in use.
4. **A correction:** I told you only Draw Frame would move to More when
   the window is narrow. In fact, below about 970 px the list buttons and
   Draw Frame move together, because More needs room of its own.

## The outline levels run (2026-10-17/18): what was done

- **OL.1** (`806999f`): Ami Pro's Outline Styles dialog (2006), its Reset
  options (`12c8:060c`: bits 2, 4, 8), the counting routine (`1200:1842`:
  every paragraph at a level counts, whatever its style), cumulative codes
  and the hidden last full stop, Quick Numbering, and Outline mode's
  Promote and Demote making Outline3 (VM OL1, OL2;
  `docs/research/OUTLINE_NOTES.md`); `UI_SPEC.md` §22; your answers O1–O4.
- **OL.2** (`53718be`): numbers counted as Ami Pro counts them; the layout
  comparison identical row for row on all three folders.
- **OL.3** (`6bd099c`): `amipro_editing::outline` and `ChangeLevel`: a
  numbered list nests as List 1, List 2 … (1. › A. › i.) at levels below
  the document's headings, so Ami Pro counts them the same; other
  paragraphs by Ami Pro's rule, making Outline# with the next free
  function key.
- **OL.4** (`9fb4bb6`): Format › Promote (Alt+Left) and Demote (Alt+Right),
  on the toolbar and the right-click menu (a selected frame's Alt+arrows
  still resize it); the Format menu folded.
- **OL.5** (`18dbe79`, `35f16a0`): Style Properties' Outline level, Reset
  options, and Cumulative numbering.
- **OL.6** (`89e51db`): Format › Styles ▸ Outline Numbering…: 1. A. i.,
  Ami Pro's I. A. 1., Ami Pro's 1, 1.1, 1.1.1, or none, in one step.
- **OL.7** (`3806adc`): twins OL1 (Outline3 with F12) and OL2 (1, 1.1,
  1.1.1) written as Ami Pro wrote them; Ami Pro opens the app's three
  files as its own, and numbers a nested list made in the app 1. A. B. 2.,
  as the app does.
- **OL.8** (`c68129b`): levels, nested lists, and cumulative numbers read
  back from `.amipro` and SAM with nothing in the loss check; Demote 2.8
  ms and Promote 4.7 ms to the drawn frame on README31, Outline Numbering
  20.6 ms (every page relaid); typing 0.02–0.06 ms (3–8 %) slower than at the run's start (0.68–0.88 ms a keystroke, interleaved A/B); bisected to OL.2, whose only typing-path change is the type of a number README31 never counts, so it is code placement rather than work, far below a frame.
- **Measures:** round trip unchanged (user's 417/417, ISO 159/160, VM 206/206 with the three new bases); the layout comparison identical on both corpora, the VM folder's new bases one page each and equal.
- **Lab:** fixtures OL1, OL2 and bases OL1B, OL2B, OL3B, the twins `AOL*`
  in `../AmiPro-Reference/vm-lab/` (`IN/OL`); screenshots `ol*`, `ol7-*`.
  The VM is saved with Ami Pro open and no document changed.

## Next steps (after the outline levels run, history)

1. Your answers to 2 and 3 above (each a short slice if yes).
2. **Frames in table cells** (`SAM_FORMAT_NOTES.md`, "A picture frame in a
   table cell"): flag `0x20`, Ami Pro's first frame list (`11c8:3ba0`),
   `1238:0ca4`, `13b0:1f29`; model the frame's cell, lay it out in the
   cell, move it with its table.
3. Open findings (`FRAME_CREATION_PROGRESS.md`, `RULER_PROGRESS.md`,
   `PHASE4_PROGRESS.md` logs): anchored frames beside Wrap around frames;
   column balance with unequal columns; text on both sides of a Wrap
   around frame; tables that continue on later pages.
4. Pictures (F4): Insert › Image… with picture decoding, in a run of its
   own. Phase 5 as planned (Outline mode there).

### Prompt for the next session

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, and PROJECT_PLAN.md.
Then set up frames in table cells as a straight run, as the outline
levels run was set up: map how Ami Pro keeps a frame in a table cell in
the decompile first (flag 0x20, its frame list 11c8:3ba0, 1238:0ca4,
13b0:1f29; how the frame moves with its table and cell, and what editing
does to it) and confirm in the XP VM with fixtures; write the proposal in
docs/Design/UI_SPEC.md if the interface changes; ask me every decision the
run needs in one go, with a recommendation each, and record my answers in
docs/research/DECISIONS.md; then write the runbook and progress file, tag
the start, and run it straight through. Measure with the round-trip audit
and layout_comparison_report before and after any change that can move
text, keep decompiler output and my corpus file names out of Git, commit
small tested slices, and hand off with a short ranked list.
```

## The bullets and numbering run (2026-10-17): what was done

- **BN.1** (`0a96af9`, `dd36cae`): Ami Pro 3.1 has no Bullets or Numbering
  command; lists come from styles (Bullet 1, Bullet, Number List) and Edit
  › Insert › Bullet… (`0x344` → `<:B<*X>>`); Right align (`1108:4107`),
  superscript (`1200:168b`), and Space for (the First indent) mapped;
  `docs/research/BULLETS_NOTES.md`.
- **BN.2–BN.4** (`fe8c372`, `e1ab9eb`, `3e8f23a`): inserted bullets in the
  model (`InlineObject::Bullet`), native v3, the importer, and the writer
  (20 in the corpora, written back byte for byte), laid out and drawn as
  style bullets in the font where they stand, one character to the caret;
  Right align (BL2), superscript (BL3), and the seven picture bullets as
  the app's own shapes.
- **BN.5–BN.8** (`6238553`, `e297be4`, `2f79caf`, `f5a8e61`):
  `amipro_editing::lists` and `SetList`; Format › Bullets and Numbering
  (toolbar toggles with the app's own icons, right-click menu, check
  marks), Enter ending a list; Insert › Bullet… (the 17 bullets drawn);
  Format › Bullet and Number Style… and Space for.
- **BN.9** (`1c68d80`): twins BL1, BL2, BL4 written as Ami Pro writes them;
  Ami Pro shows each as its own, pixel for pixel. BL4 (Create Style by
  hand) showed a made style takes the next free function key (now also
  Create Style's rule) and writes `[spec]` value 4 as 0.
- **BN.10** (`0a80901`): both formats read back with nothing in the loss
  check; Numbering 5.0 ms to the drawn frame on README31 (5.8 ms on 30
  pages), Insert Bullet 1.2 ms (5.0); typing unchanged within noise.
- **Measures:** round trip unchanged (user's 417/417, ISO 159/160, VM
  201/201); layout comparison identical row for row on both corpora.
- **Found and fixed:** a click after a selection left the hidden text
  editor's selection, so the style field said "Mixed" (the pinned iced
  `move_to` never clears a selection).
- **Lab:** fixtures BL1–BL4 and BL4B, the twins `ABL*` in
  `../AmiPro-Reference/vm-lab/` (`IN/BL`); screenshots `bl*`, `abl*`. The
  VM is saved with Ami Pro open and no document changed.

## Next steps (after the bullets run, history)

1. **Frames in table cells** (`SAM_FORMAT_NOTES.md`, "A picture frame in a
   table cell"): flag `0x20`, Ami Pro's first frame list (`11c8:3ba0`),
   `1238:0ca4`, `13b0:1f29`; model the frame's cell, lay it out in the
   cell, move it with its table.
2. Open findings (`FRAME_CREATION_PROGRESS.md`, `RULER_PROGRESS.md`,
   `PHASE4_PROGRESS.md` logs): anchored frames beside Wrap around frames;
   column balance with unequal columns; text on both sides of a Wrap
   around frame; tables that continue on later pages.
3. Pictures (F4): Insert › Image… with picture decoding, in a run of its
   own. Phase 5 as planned (outline numbering and Outline mode there).

## The frame creation run (2026-10-17): what was done

- **FC.1** (`34a6dbb`): Create Frame is `0x19e`; its dialog (`ADDFRAMEDLG`)
  creates through `0x19d`, Manual and the frame SmartIcon draw through
  `0x199`; the new frame takes the default frame (AMIPRO.INI `FrDefs`,
  built in `1018:0eeb`), which Make Default (`0x19f`) replaces; the dialog
  remembers the last frame (`frmcrt`). VM fixtures FC1–FC11;
  `docs/research/FRAME_CREATION_NOTES.md`; `ne_resources.py` lists menus.
- **FC.2** (`20080fc`): `amipro_editing::frame_creation` (`FrameDefaults`,
  `FrameGeometry`, `drawn_rect`).
- **FC.3** (`4058038`, `8c1ba08`): Where placed frames drawn and hit above
  anchored ones (FC8); Bring to Front and Send to Back within a kind; the
  SAM writer in Ami Pro's list order.
- **FC.4–FC.6** (`282e538`, `6126d1a`, `8580b0e`): Insert › Frame…, Insert
  › Draw Frame (toolbar), drawing on the page, Make Default and Restore
  Ami Pro's Default; then (`3fc8da8`) opaque frames hide what is behind
  them (a renderer layer per page frame) and rounded corners are drawn.
- **FC.7** (`adb3ece`, `e828413`): twins FC1, FC2, FC3, FC6N, FC8, FC9 write
  Ami Pro's records (after three fixes: a page frame's `[frmlay]` defaults,
  a new frame's empty `[txt]`, and Ami Pro's integer corner inset with the
  line width, `1530:0892`, now one rule for writer, importer, and engine);
  Ami Pro shows the six as its own, pixel for pixel (VM).
- **FC.8** (`27daf73`): created frames read back in both formats with
  nothing in the loss check; creating a frame 3.8 ms to the drawn frame on
  README31, 8.4 ms on a 30-page document; typing unchanged within noise.
- **Measures:** round trip read back equal 417/417 (user's), 159/160 (ISO),
  196/196 (VM folder); the user's corpus's sections equal to the original
  51,234 → 51,218, all in one file that another writer had stored in a
  different frame order, now written in Ami Pro 3.1's; layout comparison
  summaries unchanged on all three folders; text frames' column edges
  nearer Ami Pro's stored ones in 15 files, further in none.
- **Found for later:** a With para above frame narrows beside a Wrap
  around frame (FC7); Ami Pro's keyboard way of drawing; the frames Ami Pro
  makes itself for pictures and objects (with pictures, F4).
- **Lab:** fixtures FC1–FC11, bases FCB and FC9B, the twins `AFC*` in
  `../AmiPro-Reference/vm-lab/` (`IN/FC`); screenshots `fc*`, `tw-*`. The
  VM's default frame was changed for FC6 and put back. The VM is saved
  with Ami Pro open and no document changed.

## Next steps (after the frame creation run, history)

1. **Format › Bullets and Format › Numbering** (user decision 2026-10-17):
   a short proposal with its questions up front (Ami Pro keeps bullets only
   in styles), then a short run; the bullet display gaps can join it.
2. **Frames in table cells** (`SAM_FORMAT_NOTES.md`, "A picture frame in a
   table cell"): flag `0x20`, Ami Pro's first frame list (`11c8:3ba0`).
3. Open findings (`FRAME_CREATION_PROGRESS.md`, `RULER_PROGRESS.md`,
   `PHASE4_PROGRESS.md` logs): anchored frames beside Wrap around frames;
   column balance with unequal columns; text on both sides of a Wrap
   around frame; tables that continue on later pages.
4. Pictures (F4): Insert › Image… with picture decoding, in a run of its
   own. Phase 5 as planned.

### Prompt for the next session (after the frame creation run, already used)

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, and PROJECT_PLAN.md.
Then set up Format › Bullets and Format › Numbering as a short straight
run, as creating frames was set up: map Ami Pro's Bullets and Numbering
commands in the decompile first (AMIPRO.EXE through the menu-command
table, AMIPROUI.DLL's dialogs) and confirm in the XP VM; write the
proposal in docs/Design/UI_SPEC.md; ask me every decision the run needs in
one go, with a recommendation each, and record my answers in
docs/research/DECISIONS.md; then write the runbook and progress file, tag
the start, and run it straight through. Measure with the round-trip audit
and layout_comparison_report before and after any change that can move
text, keep decompiler output and my corpus file names out of Git, commit
small tested slices, and hand off with a short ranked list.
```

## The ruler run (2026-10-17): what was done

- **RU.1** (`2ab6a26`, `7dce90d`): the ruler control is `AMIPROUI.DLL`'s
  `TABRACKPROC`; its gestures reach `AMIPRO.EXE` as command `0x32a`, whose
  handler (`1148:06e5`) was found through the EXE's menu-command table in
  DGROUP (about 320 handlers there were invisible to Ghidra's analysis;
  `tools/ghidra/command_table.py` and `DecompAt.java` now list and
  decompile them). Every rule the run needed is in
  `docs/research/RULER_NOTES.md`; VM fixtures RU1–RU12.
- **RU.2** (`e4f5d8a`): `SetParagraphTabs` (one list for every selected
  paragraph) and `SetParagraphIndents` (one value, each paragraph keeping
  its others); Text › Normal keeps a paragraph's ruler; the SAM writer
  writes `<:R>` after a ruler paragraph, as Ami Pro's does.
- **RU.3** (`b035de6`, `4326f05`): unequal columns in the model
  (`LayoutBox::column_edges`), native v3 (`edges`), the importer and both
  writers, the engine, Ami Pro's dialog rules in `layout_setup`, and Even
  Columns in Page Layout Properties and Frame Properties.
- **RU.4–RU.8** (`ca81d60` to `327ad6b`): the ruler's model, the
  interactive ruler, its right-click menus, frames and bands and cells,
  Edit › Go To Ruler with Ami Pro's keys, View › Ruler (saved).
- **RU.9** (`17613ab`): 12 of 12 twins written as Ami Pro writes them; in
  the VM Ami Pro shows each twin as its own (22 of 22 screens identical).
- **RU.10** (`932eaa6`): both formats; timings: an indent release 0.63 ms
  and a margin release 41 ms to the drawn frame on README31 (0.47 and
  87 ms on the user's 68-page document); typing +0.03 ms at a document's
  start (the ruler's widget layout), unchanged elsewhere.
- **Measures:** round-trip audit identical to the baseline (user's corpus
  417/417, ISO 159/160, VM folder 181/181); layout comparison: user's
  corpus page counts 302 → 303 of 394 and page ends 762 → 761 of 1,108
  (both rows one two-column layout now laid out at Ami Pro's unequal
  widths, confirmed by Ami Pro's caches; the lost page end is an older
  empty-paragraph height difference that even columns had cancelled), VM
  folder and ISO corpus unchanged. Tests: core 268 + 4 ignored; app 201,
  106, 208 with 13, 4, 13 ignored; `tools/verify.sh --release --corpus`
  PASS.
- **Found for later:** column balance moves finished lines between columns
  without re-breaking them (only matters with unequal columns and Column
  balance); in a table cell Ami Pro's ruler also drags the table's column
  widths (not built, with table cell formatting).
- **Lab:** VM fixtures RU1–RU12 and the twins `ARU1`–`ARU12` in
  `../AmiPro-Reference/vm-lab/` (`IN/RU`); screenshots `ru9-*`. The VM is
  left running with Ami Pro open and no document changed.

## Next steps (after the ruler run, history)

1. **Creating frames** (user decision 2026-10-17: before frames in table
   cells): Insert › Frame… is still Future. A proposal in `UI_SPEC.md`
   from the user's guide (ch. 15) and the decompile, with its questions
   asked up front, then a straight run, twinned in the VM. The user's
   check of creating, moving, and resizing frames follows.
2. **Format › Bullets and Format › Numbering** (user decision 2026-10-17:
   after creating frames): a short proposal with its questions up front
   (Ami Pro keeps bullets only in styles), then a short run; the bullet
   display gaps can join it. Outline numbering and Outline mode are in
   Phase 5.
3. **Frames in table cells** (`SAM_FORMAT_NOTES.md`, "A picture frame in a
   table cell"): trace flag `0x20` in the decompile (`1238:0ca4`,
   `13b0:1f29`), model the frame's cell, lay it out in the cell.
4. Open findings (`PHASE4_PROGRESS.md` log, `PHASE3_PROGRESS.md` end,
   `RULER_PROGRESS.md` log): column balance with unequal columns; text on
   both sides of a Wrap around frame; tables that continue on later pages;
   hard-return paragraphs in one manual; frame text's ~30-twip allowance;
   Ami Pro's own hyphenation dictionary; whether style-driven page breaks
   balance columns too.
5. Phase 5 (with the user's new items: RTF or DOC/DOCX export, whichever
   is most feasible; outline numbering and Outline mode).

### Prompt for the next session (after the ruler run, already used)

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, and PROJECT_PLAN.md.
Then set up creating frames (Insert › Frame…) as a straight run, as the
ruler was set up:

1. Map how Ami Pro creates frames, decompile first: the Create Frame
   command's handler in AMIPRO.EXE (through the menu-command table,
   tools/ghidra/command_table.py and DecompAt.java) and its dialog in
   AMIPROUI.DLL: drawing a frame with the mouse, the dialog's sizes and
   position, defaults (placement, wrap, lines, margins, columns, text or
   picture), limits, where the new frame goes in the stack, and what the
   caret and selection do next. Confirm in the XP VM with fixtures, and
   write the findings to docs/research/.
2. Write the proposal in docs/Design/UI_SPEC.md, following the app's
   conventions (Frame Properties in the rail for a frame's settings,
   small dialogs only for one-shot actions).
3. Ask me every decision the run needs in one go, with a recommendation
   for each; record my answers in docs/research/DECISIONS.md.
4. Write the runbook and progress file, tag the start, and run it straight
   through without stopping: anything new goes in the progress file as
   WAITING FOR USER while the rest carries on.

Throughout: build twins in the VM and check that Ami Pro opens the app's
files as its own; measure with the round-trip audit (my corpus with
AMIPRO_STYLES set, and the ISO corpus) and layout_comparison_report before
and after any change that can move text; keep decompiler output and my
corpus file names out of Git; commit small, tested slices; keep the
progress file current; and at the end hand off with a short, ranked list of
what you need from me, including the physical check of creating, moving,
and resizing frames.
```

## After the page layout setup run (2026-10-13)

- The user: "Resave of a SAM file worked": Save As SAM's second check
  passed (Phase 4 is closed for its physical checks).
- The user asked how to change the font of a page number in a header. In
  Ami Pro the field takes the text formatting where it stands (VM fixture
  PF1: select it, Text › Font). The app drew it in the paragraph style's
  font, so Format › Font did nothing visible on it. Fixed (`d9dde32`); the
  layout comparison is unchanged (PF1 added and equal).
- The user asked for Ami Pro's ruler: "in ami pro you can see and control
  tabs and column from the ruler". Proposed in `docs/Design/UI_SPEC.md`
  §19 from the user's guide and VM fixtures RU1 (a tab clicked onto the
  ruler becomes the paragraph's own ruler, a copy of the stops in effect
  plus the new one, unsnapped) and RU2 (dragging the gap between two
  columns makes them unequal in the page layout itself). Nothing built.

## Page layout setup run (2026-10-13): what was done

- **PL.1** (`226850f`): Ami Pro's dialogs are in `AMIPROUI.DLL` (packed
  like `AMIPRO.EXE`) and its number boxes in `AMIUICNT.DLL`; both
  decompiled (outside Git). The map of every setting, unit, limit, and
  default is `docs/research/PAGE_LAYOUT_NOTES.md`; `tools/ne_resources.py`
  reads an NE module's dialogs and strings.
- **PL.2** (`6277575`, `c43ba41`): the page-number field is a style
  character, a start page, and a start number (the model read `<:P10,…>`
  as "style 10" and could not read `<:Pi…>`); one tab list per layout box;
  `amipro_editing::layout_setup` (Ami Pro's rules); `InsertPageNumber`,
  `SetPageNumber`.
- **PL.3** (`ac4be74`): Page Layout Properties in the rail, Format › Page
  Layout…, the margin right-click and double-click.
- **PL.4–PL.7** (`dff1066`): page, margins, columns, tabs, lines, header
  and footer; 19 of 19 G-fixture twins write Ami Pro's layout records, and
  Ami Pro shows and re-saves the app's twins identically; page lines drawn
  where Ami Pro draws them (probes AP1–AP5); Edit › Go To Header and Go To
  Footer.
- **PL.8** (`5e19952`): Insert › Page Break and Column Break, Insert Page
  Layout, Remove, and Revert, after VM fixtures PB1–PB3, CB1, IL1, IR1,
  IV1.
- **PL.9–PL.10** (`716190f`): Insert › Floating Header/Footer… and Page
  Number… (small dialogs, L3), after FH1–FH3 and PN1; 10 of 10 edit twins.
- **PL.11** (`268de1e`, `d0a38c5`): a document set up through every part
  reads back in native v3, SAM (nothing in the loss check), and style
  sheets; Ami Pro lays it out as the app does and re-saves it unchanged.
  Timings: 47 ms (README31) and 123 ms (68 pages) from a layout change to
  the drawn frame; typing unchanged (interleaved A/B).
- **After PL.11** (`6b46fd1`): one length rule for Style, Frame, and Page
  Layout Properties (Ami Pro's boxes: two decimals, truncated).
- **Found and fixed on the way:** page lines and the line between columns
  were never drawn; Ami Pro balances columns at a page break; copied frames
  kept their sequence numbers; the importer kept page-line values beside
  the lines the model holds; the status bar's page after a page break.
- **Measures:** round trip unchanged (user's 417/417, ISO 159/160, VM
  fixtures 166/166, nothing in the loss check); layout comparison unchanged
  on every file of both corpora (user's 302/394 page counts, 762/1,108
  page ends; VM 167/167, 84/84). Tests: core 253 + 4 ignored; app 185,
  106, 192 with 11, 4, 11 ignored; strict Clippy and fmt clean.
- **Lab:** 24 new VM fixtures (PB1–PB3, CB1, IL1, IR1, IV1, FH1–FH3, PN1,
  RG01–RG23 re-saves, RSETUP) and probes AP1–AP5 and SETUP in
  `../AmiPro-Reference/vm-lab/`; the VM is left running with Ami Pro open
  and no document changed.

## Next steps (earlier, history)

1. **The ruler** (`docs/Design/UI_SPEC.md` §19, approved R1–R4): a
   straight run under way, `docs/RULER_RUNBOOK.md`, live state
   `docs/RULER_PROGRESS.md` (tag `ruler-run-start`); RU.1's findings so
   far are `docs/research/RULER_NOTES.md`. Likely slices:
   find the ruler's code in `AMIPRO.EXE` (not yet located; the document
   window is `SAMMYTEXTPROC` `1048:0000`, the clipboard format "Ami Tab
   Ruler Format" is registered there) for hit testing, the tab copy rule,
   the indent drag, and the column drags; commands for a paragraph's tabs
   and indents; unequal columns (if approved) through the model, engine,
   native format, and SAM writer (the kept `.edges` offsets become real);
   the interactive ruler and Edit › Go To › Ruler; twins in the VM; the
   round trip and the layout comparison before and after.
2. **Frames in table cells** (`SAM_FORMAT_NOTES.md`, "A picture frame in a
   table cell"): trace flag `0x20` in the decompile (`1238:0ca4`,
   `13b0:1f29`), model the frame's cell, lay it out in the cell.
3. Open findings (`PHASE4_PROGRESS.md` log, `PHASE3_PROGRESS.md` end):
   text on both sides of a Wrap around frame; tables that continue on later
   pages; hard-return paragraphs in one manual; frame text's ~30-twip
   allowance; Ami Pro's own hyphenation dictionary; whether style-driven
   page breaks balance columns too (only the inline break was observed).
4. Phase 5 (with the user's new item: RTF or DOC/DOCX export, whichever is
   most feasible).

### Prompt for the next session (earlier, already used)

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, and PROJECT_PLAN.md.
Then resume the ruler run straight through (docs/RULER_RUNBOOK.md,
docs/RULER_PROGRESS.md): finish RU.1 from the open questions at the end of
docs/research/RULER_NOTES.md, then RU.2 onward.
```

## Phase 4 run (2026-10-13): what was done

- **4.1–4.5** (`d4c0cf4` to `fd8a64d`): the decompiled document writer
  (`11c8:1c7b`) mapped; the cache question settled in the VM (omit `[pg]`,
  keep paragraph caches while valid, never compute one); the loss
  inventory, the round-trip audit (`examples/sam_round_trip.rs`), and the
  loss check (`amipro_formats::sam_losses`); the head, styles, main text,
  and page layouts (`[l1]` now typed).
- **4.6–4.8** (`46541c0`): frames, tables, pictures, and the `[Embedded]`
  directory (`crates/ami-formats/src/frames.rs`).
- **4.9** (`b3081bb`): paragraph caches kept with a fingerprint (native
  `pcache`). Opening written files in Ami Pro found three rules the round
  trip could not see, each then settled in the decompile: a cell's eighth
  value says whether text follows (Ami Pro refused README31), a table's
  `[frmlay]` value 1 is not its bottom (MERCURY lost its logo), and a
  note's closing `>` ends its line (MERCURY faulted). The reader now warns
  where Ami Pro's is stricter, and the audit counts such warnings.
- **4.10** (`1bcdece`, `575d9e1`): Save As SAM in the app (`UI_SPEC.md`
  §17), and the frame lines bit `0x10000` (an app-made frame lost its
  border in Ami Pro; 193 corpus frames had a border here Ami Pro does not
  draw).
- **4.11** (`bd53c98`): With contents in Save as a Style Sheet (the sheet
  writer now marks every frame as the sheet's, as the decompile shows,
  correcting 3.6); sheets equal Ami Pro's own (SS2W, SS2N, F01W) but for
  `[desc]`. Edited documents write and read back as edited; new tables
  use Table Text; the reader drops kept values the writer derives.
- **4.12**: final numbers (`FORMAT_COMPATIBILITY.md`, `FEATURE_PARITY.md`
  IO-01, IO-02, IO-04) and the page layout proposal (`UI_SPEC.md` §18).
- **Measures:** read back equal 416/416 (user's corpus, line heights
  included), 159/160 (ISO; one malformed layout), 136/136 (VM fixtures);
  layout unchanged by the run's last slices (user's 302/394 page counts,
  14,400/14,871 line counts, 762/1,108 page ends; VM 136/136, 76/76).
- **In Ami Pro (VM):** text, layout, frame, table, and picture fixtures,
  README31, MERCURY, and two of the user's documents written by the writer
  open as their originals do (start and end screens pixel-identical but
  for the caret), and Ami Pro saves them back with the same pages;
  documents made in the app (APP1, APP2) and edited ones (EDK1, EDK2) open
  as made.
- **Found for later:** frames in table cells (flag `0x20` on Where placed
  pictures, 98 frames in 23 files) are Where placed frames in the model,
  but Ami Pro moves them with their table; text on both sides of a Wrap
  around frame (the engine uses the wider side); styles stored without a
  function key get the next free ones in Ami Pro.
- **Lab:** one VirtualBox guru meditation during a guest restart (after
  Ami Pro faults); recovered by a power cycle and the transient share
  re-added. Scratch scripts for re-saving and rebooting were outside Git.
- The user's mid-run direction, "the decompile source should always be
  your primary source", is in memory and was applied to every finding
  after it.
- Tests: core 235 + 2 ignored; app 175 (normal), 106 (text editor), 182
  (all features) with 10, 4, 10 ignored; strict Clippy and fmt clean; ISO
  corpus gate PASS.

## Phase 3 run (2026-10-12/13): what was done

Each slice measured with `layout_comparison_report` on the user's corpus,
the VM fixtures, and the ISO corpus (`LAYOUT_COMPARISON.md`):

- **3.1** (`901fba7`): an empty paragraph's line in the font at its end
  (`Paragraph::end_format`), wider line-height calibration. **3.1b**
  (`870c359`): empty paragraphs read as Ami Pro's reader does; user's corpus
  page ends 686 → 742 of 1,108.
- **3.2a–d** (`a6df120`, `f81ce7a`, `9b22df0`, `f7b5a6d`): paragraph rules
  (`[line]`), Next style, Text Tightness, tab stops with leaders, rulers,
  the style's own tabs and the hanging-indent stop, frame interiors' space
  below. **3.2e** (`d7deb91`): bullets and numbers (`[spec]`), drawn and
  numbered as Ami Pro does; found two importer bugs (`<*>>`, empty fields).
- **3.3** (`47fcf12`): overstrike (drawn, and in Special Effects), protected
  text (Ami Pro's caret rules from VM fixture PT1), no hyphenation, and each
  face's pitch-and-family byte for substitutes.
- **3.4** (`842e5d3`): style origins (document, sheet, missing), the style
  sheet link, `CreateStyle`/`ModifyStyle`/`RenameStyle`/`DeleteStyle`/
  `UseStyleSheet` commands, and `read_style_sheet`, by VM fixture ST1. No
  menus yet (decision D1).
- **3.5** (prepared): the D1 proposal is `UI_SPEC.md` §14; the D2 note is
  `docs/research/HYPHENATION_OPTIONS.md`; D3 is the go-ahead for writing
  `.STY` files. The questions are at the end of this file.
- Final measures: user's corpus 298 of 394 page counts, 14,389 of 14,871
  line counts, **742 of 1,108 page ends** (686 of 1,097 at the run's start);
  VM fixtures 131 of 131 page counts and 76 of 76 page ends; ISO corpus 406
  of 1,307 page ends.
- Tests (`842e5d3`): core 200 + 1 ignored; app 151 (normal build), 94
  (text editor), 158 (all features) with 9, 4, and 9 ignored; strict Clippy
  and fmt clean; ISO corpus gate PASS; user's corpus 416/416 round trips.

## Where things stand

- **Phase 2.3–2.5 are complete and 2.6's automated parts are done** (base tag
  `phase2-run-start`, last implementation commit `2c9a38e`):
  - 2.3: typed IDs, a flows map, `Arc`-shared roots behind one command
    history, U+FFFC inline objects and legacy marks, the page-layout and frame
    model with commands, and native v3.
  - 2.4: SAM import of layouts, headers, footers, frames, tables, pictures,
    anchors, breaks, and floating headers (160/160 corpus files).
  - 2.5: `amipro-layout` (the own line breaker over COSMIC Text words, pages,
    columns, frames, headers, interiors, tables), incremental relayout
    (0.09–0.10 ms after typing on README31), the real `Page X of Y`, and the
    comparison with Ami Pro (`docs/research/LAYOUT_COMPARISON.md`).
  - 2.6: the page editing surface behind `page-surface`, the normal build
    since 2026-10-12 (`cargo run --offline --manifest-path app/cosmic/Cargo.toml`).
  - After the run (user request): MERCURY.SAM compared with Ami Pro in the
    VM; table offsets and With para above frames fixed
    (`LAYOUT_COMPARISON.md`, `DECISIONS.md` 2026-10-12). README31.SAM
    compared next; joined cells, cell margins, Table Text, and table growth
    fixed. Tables that continue on later pages are not split yet.
  - This session: the user's corpus through the audit and the comparison
    (`LAYOUT_COMPARISON.md`, "The user's corpus"; importer messages for Word
    Pro and protected files, the style-sheet path, metric-compatible font
    substitutes, and evidence that `[algn]` indents explain most remaining
    line differences), then frame interaction (`UI_SPEC.md` §11): selection,
    handles, moving, resizing, keys, arranging, and the Frame Properties
    inspector in the rail (`0e5f3f7`, `9319efd`).
  - Next session (2026-10-12, `5694a8d` to `003d345`): Flow with text frame
    offsets (F14–F20); Phase 3 indents, spacing, break rules, and alignment
    from styles and from Text menu overrides (VM fixtures I, S, B, O, LS1);
    With para above frames sized by their rectangle; and `AMIPRO.EXE`
    unpacked and fully decompiled (`docs/research/DECOMPILATION.md`). The
    user's corpus now has 686 of 1,097 page ends on Ami Pro's page (was
    642 at the start of the session), 76% of page counts equal and 94%
    within one; VM fixtures 75 of 75 page ends.
  - Decompile audit session (2026-10-12): the SAM reader, writer, and
    layout routines mapped in the decompiled `AMIPRO.EXE` (pointer table at
    DGROUP 0xbe6; routine addresses in `SAM_FORMAT_NOTES.md`, "The reader
    in AMIPRO.EXE"); the format notes' INFERRED and unknown items settled
    as VERIFIED (decompile) or narrowed; the rules imported and laid out
    (`DECISIONS.md`, "Layout rules from the decompile"). VM fixtures 121 of
    121 page counts (was 119); user's corpus 679 page ends (was 686), a
    loss explained by line heights (below).
  - Line heights (user decision: option 1): each imported document keeps
    its printer's single line heights, calibrated from its one-line
    paragraph caches, in the model and native v3, and the shaper uses them
    (`DECISIONS.md`, "Calibrated line heights"). Page ends: user's corpus
    689 of 1,097 (679 before), ISO corpus 371 of 1,306 (287 before).
- **UI pass (user decision 2026-10-12, ahead of the rest of Phase 3), DONE:** the
  UI-2 checklist is skipped; the pass works on the page-surface build against
  `docs/Design/UI_GAP_LIST.md` and ends in one combined physical check.
  Done: whole-window screenshot harness (`window_screenshots`), recessed
  canvas and paper edge, the rebuilt context rail (216 px surface, header
  with close, filter, list rows, narrow-window overlay, readable Frame
  Properties), Open/Save icons, More only when narrow, the header-bar title
  (it was missing: only the compositor title was set), word count, and the
  ruler strip with indent markers. Then, from the user's own testing:
  More only when groups do not fit (measured, data-driven bar,
  `82d594f`), Ami Pro's page arrows and Go To in the status bar
  (`42a6133`; physical PASS after `c0c8185`, which gave the workspace
  one scrollable id: a per-tab id never reached a tab opened later), and
  aligned menu labels (`d210fa0`). The user asked for an
  editable toolbar and answered its questions: one editable toolbar, top
  only (`UI_SPEC.md` §12); built the same day (Customize Toolbar…, View ›
  Toolbar, saved in the user's COSMIC configuration, `03a2fc1`), then
  Underline and the four alignments with visible toggle states
  (`c4aec53`). **The combined physical check passed** (“This all appears to
  work great”), and the user decided: the page surface is the normal build,
  and every on/off command gets a check mark (Format › Bold, Italic,
  Underline, the alignments; `83636d1`). Gap list: `docs/Design/UI_GAP_LIST.md`;
  decisions: `DECISIONS.md`, 2026-10-12.
- **Character overrides, DONE (2026-10-12, this session)**, in four
  tested commits:
  - `d672f88` model and native v3: `CharacterFormat` gains word and double
    underline, strikethrough, superscript, subscript, caps, and colour;
    `ParagraphStyle` gains `CharacterAttributes`; `sformat`/`rformat`
    records (`NATIVE_FORMAT_V3.md`, which now also documents `pformat`).
  - `09bf508` import, after the decompiled control parser (`1118:09d6`),
    attribute routine (`10d0:33c6`), and font parser (`14f0:0000`): `<+X>`
    marks (off returns to the style for bold, italic, the underlines, and
    caps; strikethrough and the scripts end), `<:f…>` (each replaces the
    last; empty fields are the style's), and the styles' `[fnt]` colour and
    flags. `f83cf71` keeps the format changes of a paragraph without text as
    legacy marks (no run can hold them).
  - `755da7d` layout and drawing: `amipro-layout::display` gives the text
    as shown (caps, byte for byte) and display spans (scripts 7/12, small
    caps 7/10 for lower case and spaces, full-size line metrics); both
    shapers and the page surface use it; the surface draws colour, the
    three underlines, strikethrough, and raised or lowered scripts.
    `a55dd5b` puts rules on whole pixel rows (found comparing O1 with Ami
    Pro).
  - Measured on the user's corpus (`LAYOUT_COMPARISON.md`, "Character
    overrides"): page ends 689 → 686 of 1,097. Font changes gained 18 page
    ends in six files and lost 20 in one photo report, whose earlier match
    was two errors cancelling (its text heights now equal the caches). VM
    fixtures unchanged (121/121, 75/75). New findings in
    `SAM_FORMAT_NOTES.md` ("How the display applies them"); decision in
    `DECISIONS.md`, "Character attributes in the model".
- **Ami Pro's Text menu items and caret states, DONE (2026-10-12, same
  session, user request; `a9e0e06` to `153b855`):** Format › Word
  Underline, Caps ▸, Special Effects…, Font…, and Normal (user's choice of
  Ami Pro's items); check marks and toolbar states follow the caret and a
  character command at a caret formats the next typing, as VM fixture C1
  showed Ami Pro doing; text typed at either end of a run joins it. The
  user kept "what you see" check semantics (Ami Pro checks only Text menu
  formatting, not the style's). `UI_SPEC.md` §13; `DECISIONS.md`, "Ami
  Pro's Text menu items". **Physical check: PASS** (user: "This looks
  great"; the Caps submenu "works and lines up").
- **The page surface is the normal build** (user decision, 2026-10-12,
  after the combined check passed): plain `cargo run` runs it; the text
  editor is `--no-default-features`. Still open for it: accessibility
  nodes, a visible-IME-preedit check, picture decoding.
- **Waiting for the user** (not urgent): the comparison's tolerances
  (frame interaction's own checklist passed on 2026-10-17).
- **Tests (2026-10-12, `153b855`):** core 164 + 1 ignored; app 143 (normal
  build), 93 (text editor, `--no-default-features`), 150 (all features)
  with 9, 4, and 9 ignored; strict Clippy and fmt clean; ISO corpus gate
  PASS (13,817 runs, at `e67b6b5`). `tools/verify.sh --release --corpus`
  last passed at `003d345`.

## Reference environment

- The agent operates the `Windows XP` VM directly (user authorization). Read
  `docs/research/VM_LAB.md` first. It is left in **saved state**; resume with
  `VBoxManage startvm "Windows XP" --type headless`. If `L:` is missing in the
  guest, re-add the transient share as VM_LAB.md shows.
- Baseline snapshot: `Clean baseline (before AmiPro-Linux lab)`.
- Tools: `tools/vm/vmkey.py` (keys, Alt codes, screenshots),
  `tools/vm/vmmouse.py` (mouse via the VirtualBox API; needed for Group, the
  Tools menu, and placing the caret), `crop.sh`, and `frame-fixture.sh`
  (`--open`/`--finish` to check a dialog before saving). Fixtures and
  screenshots are in `../AmiPro-Reference/vm-lab/` (outside Git).
- The online manual as SAM is a `/tmp` extraction. If
  `/tmp/amipro-phase1-corpus` is gone, rebuild it with
  `tools/reference_inventory.py`; the corpus audit needs it.
- Ghidra: `AMIPRO.EXE` is decompiled in full (3,953 functions) at
  `../AmiPro-Reference/ghidra-work/out/amipro.c`, project `proj2`; the
  method (OPTLOADER unpacking with `tools/ghidra/optloader-unpack.py`, then
  the NE loader fix) is in `docs/research/DECOMPILATION.md`. Functions are
  unnamed (`FUN_SSSS_OOOO`); DGROUP is segment 209 (`1680`). The SAM
  section names are reached through a pointer table at DGROUP 0xbe6
  (`DAT_1680_0bXX`); the reader, writer, and layout routines are listed in
  `SAM_FORMAT_NOTES.md`, "The reader in AMIPRO.EXE". The `w4w33f.dll`
  filter's decompile is beside it.

## Next steps (earlier, history)

1. **Page layout setup** (`PROJECT_PLAN.md`), once the user answers
   `UI_SPEC.md` §18's questions: the Page Layout Properties panel, Insert ›
   Floating Header/Footer… and Page Number…, inserted layouts; each change
   one `ModifyLayout` command; written by both writers; physical check.
2. **Frames in table cells** (`SAM_FORMAT_NOTES.md`, "A picture frame in a
   table cell"): trace flag `0x20` in the decompile (`1238:0ca4`,
   `13b0:1f29`), model the frame's cell, lay it out in the cell.
3. Open findings (`PHASE4_PROGRESS.md` log, `PHASE3_PROGRESS.md` end):
   text on both sides of a Wrap around frame; tables that continue on later
   pages; hard-return paragraphs in one manual; frame text's ~30-twip
   allowance; Ami Pro's own hyphenation dictionary.

Outside these, still open: accessibility nodes for the page text, a
physical check of visible IME preedit, picture decoding,
password-protected documents (frame interaction's own checklist passed
on 2026-10-17).

### Prompt for the next session (earlier, already used)

```text
Read AGENTS.md, CLAUDE.md, docs/SESSION_HANDOFF.md, PROJECT_PLAN.md,
docs/LAYOUT_SETUP_RUNBOOK.md, and docs/LAYOUT_SETUP_PROGRESS.md. Run the
page layout setup straight through from the first slice that is not DONE:
its decisions are taken (L1–L4, UI_SPEC.md §18), so do not stop to ask;
record anything new as WAITING FOR USER and carry on. Treat the decompiled
AMIPRO.EXE and the Ami Pro DLLs as the authoritative source for what Ami
Pro does (meanings, units, limits, defaults, validation, how settings are
written), confirmed in the XP VM with twin fixtures; build the interface
to the approved design and the app's existing conventions, not to Ami
Pro's dialogs. Measure with the round-trip audit (my corpus with
AMIPRO_STYLES set, and the ISO corpus) and layout_comparison_report before
and after any change that can move text. Keep decompiler output and my
corpus file names out of Git. Commit small, tested slices, keep the
progress file current, and at the end hand off with a short, ranked list
of what you need from me.
```

## UI-2 checkpoints (history)

- Native title (`2fa7a31`): the active session supplies
  `Document Name — AmiPro-Linux` through libcosmic's native window-title API.
  Compositor title visibility is a pending physical check.
- Grouped action bar (`8fecd2e`): native symbolic New/Open/Save, Undo/Redo,
  Cut/Copy/Paste, and Bold/Italic share UI-1 dispatch. There is one
  synchronized style dropdown. A labeled More menu holds lower-priority or
  responsively displaced implemented commands.
- Status bar (`e96f167`): one compact 28 px line. It holds an estimated-page
  field, a middle-ellipsized message, and an optional imported-source
  protection state. There is no style control and no permanent Modified prose.
- Document-switch fallback (`f80c178`): Next Document is Ctrl+PageDown because
  the physical Ctrl+Tab press never reached the application. Ctrl+Shift+Tab
  remains Previous.
- Native tabs and chrome geometry (`96aeda4`):
  - libcosmic `tab_bar` gives a document icon, filename, modified dot, close
    action, native selected/focus styling, and overflow arrows.
  - A New Document button follows the last tab and stays reachable when tabs
    overflow (checked by sweeping widths 900–1920 px).
  - Sessions remain authoritative; `sync_tab_model` reconciles the tab model
    after every message.
  - Fixed a layout defect from `8fecd2e`. The action bar's `responsive` wrapper
    defaulted to fill height, so it took 322 px of an 800 px window. A real
    tiny-skia layout test of the whole view reproduced it before the fix.
  - Removed the 12 px outer frame and inter-row spacing. Action groups use
    4 px internal gaps.

## Verification (2026-10-11, `96aeda4`)

- Core: `cargo fmt --all -- --check` passed. `cargo test --workspace` passed
  29/0. Strict Clippy passed.
- COSMIC: fmt passed. Default tests passed 70/0 with 1 ignored; all-feature
  tests passed 75/0 with 2 ignored. Strict Clippy passed for default and
  all-features. `--locked --release` builds passed for both.
- Corpus audit is unchanged: 160/160 SAM import and native round-trip, 63/63
  STY parse, 30 unsupported section kinds, 666 control forms.
- Release rich-editor baselines on `README31.SAM` (median/p95): insert
  0.275/0.382 ms, sustained typing 0.341/0.570 ms. There is no regression from
  `bf0eaa5`. Full figures are in `PROJECT_STATUS.md`.

## Deviations to review with the user

- The user reported on 2026-10-11 that the UI is closer to the approved mockup
  than at the start but still not in line with it. It is not the final
  appearance; a screenshot-versus-mockup gap list is required before visual
  acceptance.

- Per-tab tooltips (full path, source type): the pinned libcosmic
  `segmented_button` has no per-item tooltip. The filename shows the extension.
  The status bar shows "Imported source protected" for SAM sources.

- The pinned tab bar exposes no accessibility node. Icon buttons in the action
  bar and New Document do have accessible names.

- The status bar shows the caret's page of the real layout since 2.5c (the
  hard-coded "Page 1" and the estimate are gone).

- Styles panel width/behavior, workspace padding/page edge, and zoom belong to
  UI-3 and UI-4.

## Default editor performance (`8b78ecf`)

The dead per-edit whole-document `DocumentLayout` rebuild and the full-content
undo/redo rebuild were removed. On `README31.SAM`, release end-to-end headless
typing went from 52–55 ms to 0.58–0.62 ms median, and undo/redo from 79–86 ms
to 0.43–0.67 ms. Evidence is in
`docs/research/DEFAULT_EDITOR_TYPING_PERFORMANCE.md`. Physical confirmation is
folded into UI-2 checklist item 12.

## Findings for later slices

- SAM importer, fixed 2026-10-11 after verification in original Ami Pro
  (`docs/research/SAM_FORMAT_NOTES.md`): untagged paragraphs are Body Text,
  escapes and character codes decode, and multi-line notes no longer truncate
  documents (27 of 160 corpus files were cut short). Physical COSMIC check of
  `README31.SAM` is pending.

## Code map and constraints

- `app/cosmic/src/main.rs`: window title, action/tab/status views, session
  state, tab-model reconciliation, command dispatch, and UI tests (including
  real tiny-skia layout tests of the whole view).
- `app/cosmic/src/command.rs`: authoritative command metadata and shortcuts.
- `app/cosmic/src/chrome.rs`: pure action-bar and status-bar composition.
- `docs/Design/UI_SPEC.md` and `UI_MOCKUP.svg`: approved visual authority.
- Preserve the default editor, the `rich-editor-prototype` gate, P0 incremental
  layout, the importer/file formats, and the untracked `AGENTS.md.backup`.

## Repository state

- Expected untracked user file: `AGENTS.md.backup` (do not modify or delete).
- Commits `7076f36` and `2031b8e` put four file names from the user's
  corpus into `DECISIONS.md` and `LAYOUT_COMPARISON.md`; `6095d8e` removed
  them from the current text, but they remain in Git history. History was
  not rewritten; that is the user's call.
- No known implementation blocker. Everything left in Phase 2 needs the
  user (`docs/PHASE2_PROGRESS.md`, "Waiting for the user"). The VM is
  saved (Ami Pro open, no document changed); its transient share `L:` is
  gone after a restore and is re-added as `docs/research/VM_LAB.md` shows.
