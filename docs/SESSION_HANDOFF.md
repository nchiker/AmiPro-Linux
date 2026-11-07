# Session Handoff

**Updated:** 2026-11-07, after run D's exit.
`PROJECT_PLAN.md` is authoritative for sequencing. Earlier handoffs:
`docs/HANDOFF_ARCHIVE.md`.

## Run B finished and physically accepted 2026-11-06

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

## Run D finished 2026-11-07 (resume here)

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

## Run C finished 2026-11-06

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

## Start here

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
