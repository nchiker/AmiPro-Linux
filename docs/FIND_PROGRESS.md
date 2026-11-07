# Find & Replace Progress

Live state of `docs/FIND_RUNBOOK.md`. Resume at the first slice that is not
`DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| FD.1 Setup | DONE | (setup commit) | The runbook; this file; the baseline below | Tag `find-run-start` |
| FD.2 The search | DONE | (this commit) | `crates/editing/tests/find.rs` (15 rule tests: FR1, FR1B, FR2, FR3, FR5/FR6/FR7's order, the guide's wildcard examples, tabs and returns, the capitals rule, a selection, paragraph styles, Replace All as one step) | `amipro_editing::find`: `Pattern` (`?`, `*` one or more word characters, `<?>`, `<*>`, `<»>`, `<¶>`, Ami Pro's marks » and ¶ for a tab and a return), `streams` and `search_streams` in Ami Pro's order, `find_all`, `replacement` (the n-th wildcard takes the n-th capture; `set_in_capitals_of`), `replace_all` (`Command::Several`, last match first) |
| FD.3 Exact attributes | DONE | (this commit) | `find_exact_attributes_finds_only_text_with_them`, `replace_exact_attributes_sets_exactly_them` (the guide's three examples) | `FindOptions::find_attributes` and `replace_attributes`; `shown_attributes`; a match counts only where every character shows the same five attributes |
| FD.4 Twins and the sweep | DONE | (FD.2's commit; this commit) | `find_twins.rs` `replace_all_gives_what_ami_pro_gave` (ignored, `AMIPRO_VM_LAB`): FR1A, FR1B, FR2, FR3, FR5A, FR6 from Ami Pro's own input files, every stream's text as Ami Pro saved it; `every_paragraph_is_found_in_its_stream` (ignored, `AMIPRO_CORPUS`): user's corpus 417 documents, 18,752 streams, 31,061 of 31,061 paragraphs found where they are; ISO 160, 6,699, 33,438 of 33,438 | Every flow is a stream but the covered cells of joined blocks (2,475 and 34), which show nothing |
| FD.5 The find bar | DONE | (this commit) | `find_bar.rs` tests (next and previous round from the caret, a selection fills Find or limits the search, recent searches, options in the settings and the attribute choices); `the_find_bar_finds_steps_replaces_and_replaces_all_as_one_step_each`, `the_find_bars_options_follow_ami_pro_and_are_kept` (message path); `the_find_bar_takes_the_keys_shows_under_the_ruler_and_keeps_the_scroll` (widget path: typed keys reach the Find field and not the page, Enter finds, the scroll is kept) | Edit › Find… (Ctrl+F), Replace… (Ctrl+H), Find Next (F3), Find Previous (Shift+F3) implemented on the page surface; `FindBar`, `FindAction`, `FindMenu`; Options ▾ and Recent ▾ as one-root menu bars; `Preferences::find_options`, `recent_searches`; the last bar's texts for the session |
| FD.6 On the page | DONE | (this commit) | `find_bar_screenshots` (ignored): `window-find-light`, `-dark`, `window-find-replace-light`, `-dark` | `page_surface::Highlights` drawn in a marker yellow under the selection; the current match is the selection alone; a match is selected on the page that shows it and scrolled into view |
| FD.7 Timings | DONE | (this commit) | `find_performance_baseline` (ignored, release; `AMIPRO_PERF_SAM`, `AMIPRO_FIND`) | README31 (344 paragraphs, "the" 585 matches): typing the search and Find Next 2.2–3.8 ms to the drawn frame; a keystroke in the page 0.9 ms without the bar, 2.5–3.9 ms with it open; Replace All 9.9–16.3 ms; its undo 5.9–9.3 ms. The user's longest file (883 paragraphs, 1,117 matches): 4.2–4.7 ms, 7.6–8.1 ms with the bar open, 29.3 ms, 7.5–8.7 ms. Ranges over runs on a busy host |
| FD.8 Exit | DONE | (the exit commit) | `tools/verify.sh --release --corpus` PASS (core 370 + 6; app 281, 108, 288 with 33, 4, 33 ignored; ISO corpus gate PASS); the measures at the exit equal the baseline row for row (`target/find-exit/`); the typing A/B against `find-run-start` (below) | §27's implementation notes; `FEATURE_PARITY.md` ED-02; `UI_ACCEPTANCE.md` "Find & Replace" and its sign-off row; `UI_COMMAND_MATRIX.md`; `FORMAT_COMPATIBILITY.md`; `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`; the playbook's queue (fields NEXT); `docs/SESSION_HANDOFF.md` |

## Baseline (at `find-run-start`)

Measured at `383e6e0` (the row cache), `target/rc-after/`:

- Round trip: user's corpus 417 of 417 read back equal, ISO 159 of 160, VM
  folder 272 of 272.
- Layout comparison: user's corpus page counts 315 of 394, page ends 762 of
  1,108; ISO 69 of 160, 478 of 1,307; VM folder 271 of 271, 111 of 121.
- Tests: core 352 + 4 ignored; app 274, 108, 281 (31, 4, 31 ignored).

## Log
- 2026-10-20, FD.2–FD.4: the search in the editing crate; Replace All is
  one `Command::Several`. Rules settled from the guide (ch. 20): Ami Pro
  shows a tab in a field as » and a return as ¶; a return in Find matches a
  paragraph's end (deleting it joins paragraphs) and, by choice FD1, a line
  break too; `*` takes one or more letters or digits and never none; each
  wildcard of the replacement takes what the wildcard in the same place
  among Find's matched (`*eet*` → `meet*`: "sheets" → "meetsh"); Beginning
  of document alone searches the main text from the top, with Include
  other text streams the whole document. FD2: Whole word only counts
  letters and digits as a word's. FD3: the capitals rule takes a found
  text of two or more letters, all capitals, as capitals; one capital
  letter capitalizes each word. Twins: Ami Pro's input files through the
  app's Replace All give Ami Pro's text in every stream (six of six).
- 2026-10-20, FD.4: the sweep's first run stopped at a flow outside the
  streams; all such flows are covered cells (joined blocks' inner cells,
  which show nothing), which the walk leaves out on purpose. Beginning of
  document with the other streams now searches the whole document from any
  stream (the guide), not only from the caret's.
- 2026-10-20, FD.5–FD.6: the bar on the page surface. Found while
  testing: reopening the bar with Ctrl+H rebuilt it and lost the current
  match, so Replace did nothing; the current match is now whichever match
  is selected (FD5). A field given the keyboard by a task hears nothing of
  it (libcosmic sends its focus message only for a click), so the bar
  records the field it focuses; Esc relies on it. The current match drawn
  over its yellow looked olive; it is drawn as the selection alone.
- 2026-10-20, FD.7: each text is folded to small letters once per search
  (it was folded at every comparison): Find a little faster, the keystroke
  with the bar open unchanged, so the search is not its main cost there.
  The bar finds every match again after each edit (1.6–3 ms on README31,
  about 7 ms on 883 paragraphs, under a frame): finding again only the
  paragraphs an edit changed is left for later (the handoff).
- 2026-10-20, FD.8: `tools/verify.sh --release --corpus` PASS; the measures
  at the exit (`target/find-exit/`) equal those after the row cache row
  for row. The typing A/B against `find-run-start`, run twice
  (`target/bench-ab/20261020-103739/`, `…-105801/`): 7 and then 8
  operations flagged, not the same ones; the default editor's flags did
  not repeat (0.05–0.07 ms once); the rich editor prototype 0.05–0.09 ms
  slower an edit in the second run only, as the pictures run found and
  bisected to code placement; nothing this run changed is on its path.
  Recorded as inconclusive, far below a frame, not chased.
- 2026-10-20, after the exit: the user's physical check passed. Their
  visual notes (the Find and Replace fields did not line up; the bar
  seemed to belong above the ruler): the bar is now a chrome row between
  the tabs and the ruler, styled as the toolbar (40-pixel rows, 32-pixel
  icon buttons, its gaps and dividers, fixed-width menu roots), its two
  rows on one grid so the fields start and end together; screenshots in
  both themes; the find, scroll, and layout tests pass.
- 2026-10-20, later: the user looked at the bar in its new place (above
  the ruler, its fields aligned): "visual find and replace passed".
