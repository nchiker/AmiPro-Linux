# Outline Mode Progress (run B)

The live state of `docs/OUTLINE_MODE_RUNBOOK.md` (tag `outline-mode-start`).
Resume at the first slice that is not `DONE`.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| OB.1 Setup | DONE | (tag `outline-mode-start`) | Runbook, this file, `OUTLINE_MODE_NOTES.md` (decompile brief; VM OM1); baseline = run A's CA.15 measures | |
| OB.2 Sections and Move Up/Down | DONE | (next commit) | `sections.rs` tests; editing `tests/sections.rs` (one undo step, SAM round trip); scenarios `outline_mode.rs` (Alt+Up/Down by key, caret kept, refused at the ends, body text alone, Undo/Redo, Save As and reopen; in a table cell rows move; a With para above frame goes with its section) | Format › Bullets and Numbering ▸ Move Up, Move Down; RB1. The default sheet's headings have no outline level (the scenario gives them levels) |
| OB.3 Outline mode view | DONE | (this commit) | `outline_view.rs` tests (indents by level, icons, the copy keeps the text and leaves the document as it is); widget test `outline_mode_lines_take_clicks_and_typing_where_they_show`; scenario `outline_mode.rs`; screenshots light and dark; typing in Outline mode on README31 1.5–1.9 ms median (release; Layout 1.4–1.6) | View › Outline Mode, per tab, exclusive with Draft; RB2 |
| OB.4 Expand, Contract, Show levels, the Outline bar | DONE | (this commit) | `outline_view.rs` tests (Contract, Expand, Show levels as `1608:154c`; hidden paragraphs keep their places); scenario `outline_mode.rs` (keys, the contracted icon, hidden lines under 2 twips, the caret leaving hidden text both ways, arrows over hidden text, typing, Move Down of a contracted section, Layout shows all, state kept per tab, nothing saved); screenshots (bar, contracted, Show 2) | View › Outline ▸; the Outline bar in the toolbar's place; RB3 |
| OB.5 Use Outline Styles | DONE | (this commit) | units (each level's `_OUTLINE.STY` font, display only); app test `use_outline_styles_changes_only_what_outline_mode_shows` | A preference (Ami Pro's INI bit), for every open document; the page surface draws from the outline copy in Outline mode |
| OB.6 Outline navigator | DONE | (this commit) | editing `tests/sections.rs` (`MoveSectionTo`, one step, refused inside itself); `outline_navigator.rs` test (the tree lists what Outline mode shows; the current heading); scenario `outline_mode.rs` (click goes, drag moves one step, the end area, Toggle in step with Outline mode, keys); screenshots | View › Outline ▸ Outline Navigator; in both modes |
| OB.7 Measures and twin | DONE | (this commit) | Twin OBT1 opens in Ami Pro as made. Measures (interleaved A/B vs `outline-mode-start`, README31, release, 3 rounds): Layout typing +0.04 ms (noise), Draft equal to Layout, Outline +0.10–0.15 ms (1.2 ms total); corpus gate PASS (160 SAM, 0 failed), CA.15 round-trip and layout comparisons identical. Live check on a private Xwayland (release app, screenshots looked at): items 1–6, 8, 9 pass; item 6 found the navigator's arrow keys going to the page after a row click | Fixed: the page leaves plain arrows alone while the navigator holds the keys (`PageSurface::arrows_elsewhere`; test `the_page_leaves_plain_arrows_to_the_navigator_that_holds_them`). Not run: Save in Outline mode (the untitled Save opens the portal; covered by the scenario) |
| OB.8 Exit | DONE | (this commit) | `tools/verify.sh` green without `AMIPRO_STYLES` (core 629 +20 ignored; app 569, text editor 124, all features 576); `--release` builds and `--corpus` audit pass (run with the variable set) | Run the test steps with `AMIPRO_STYLES` unset: the style-sheet tests count sheets and fail against the corpus folder |

## Choices made in the run (RB…)

- **RB1** Move Up and Move Down move a heading with its section whether it
  is expanded or not (OM3); Ami Pro moves an expanded heading alone and a
  contracted one with its hidden text (VM OM1).

- **RB2** Outline mode lays the text out on window-wide pages as Draft does, so a document's pages still show with the gaps between them; Ami Pro shows one continuous column (a page break would make one page per break in a continuous copy). Frames, headers, and footers show where they stand, as in Draft (CK10); Ami Pro's Outline mode draws the main text only.

- **RB3** Expand, Contract, and Show levels are kept per tab while it lives, also across Layout mode (OM2); Ami Pro clears them each time Outline mode is entered (`1608:1773`). A hidden paragraph keeps its place in the display copy as an empty line one twip high, so every position stays the document's; the caret never stays in one (it goes on the way it was moving).

Physical check by the user, 2026-11-06: all passed.

## From the live check (OB.7)

- Fixed: the navigator's arrow keys (`arrows_elsewhere`); the ruler offset (hidden in Outline mode, `ruler_shown`).
- By design: Ctrl+Shift+D toggles Draft and Layout; Enter at a heading's end keeps the heading when the style has no Next style.
- Not reproduced: Return once opening Document Properties.
- Not outline, done after the run: Escape cancels the topmost dialog (29 dialogs; tests in `page_surface_app_tests.rs`).

## Baseline (OB.1)

Run A's exit measures (`docs/COMMANDS_PROGRESS.md`, CA.15).
