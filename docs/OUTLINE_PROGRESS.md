# Outline Levels Progress

Live state of `docs/OUTLINE_RUNBOOK.md`. Resume at the first slice that
is not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| OL.1 The map | DONE | (setup commits) | `docs/research/OUTLINE_NOTES.md`; VM OL1, OL2; `UI_SPEC.md` §22; O1–O4 | Reset bits 2/4/8 (`12c8:060c`); counting `1200:1842`; cumulative codes and the hidden last full stop; Quick Numbering; Outline# from Promote/Demote |
| OL.2 Counting as Ami Pro counts | DONE | (this commit) | `reset_options_restart_a_level_as_ami_pro_does_1200_1842` (After higher level, After intervening style, neither; a level counts every paragraph at it, whatever the style), `cumulative_numbers_count_each_level_and_hide_the_last_full_stop_ol2` (1, 1.1, 1.2, 2, 2.1), BN2's numbers unchanged; layout comparison identical row for row | `ParagraphNumber { own, levels }`; `bullet_box` takes it; only shown numbers are kept (0 for unnumbered styles, levels only for cumulative ones), so incremental relayout's tail check is as before |
| OL.3 Promote and Demote in the model | DONE | (this commit) | `headings_and_none_move_by_ami_pros_rule_ol1` (Outline3 made from Subhead; None promoted to the previous level and demoted below it; level 1 stays), `a_numbered_list_nests_as_one_a_i_o2` (the run to List 1, the demoted to List 2 at levels 3 and 4 below Subhead, A., i., back, one undo step), `nested_lists_and_outline_levels_made_in_the_app_write_as_ami_pro_writes_them` (SAM `[tag]` levels and flags 18, read back equal, nothing lost, native exact); the random undo test with `ChangeLevel` | `amipro_editing::outline`: `LevelChange`, `list_depth`, `Command::ChangeLevel`; made styles take the next free function key; a used style at a level is preferred |
| OL.4 The commands and the Format menu | DONE | (this commit) | `promote_and_demote_nest_a_numbered_list_as_one_a_i_o1_o2` (through messages: List 1/List 2, the numbers drawn 1., A., B., 2., i., one undo step, the top not promoting, the shortcuts), `alt_arrows_promote_and_demote_text_and_resize_a_selected_frame` (widget path), the frame-key test, the folded menus in the order, check-mark, and right-click tests; `the_standard_bar_fits_wide_windows_and_moves_its_last_groups_when_narrow`; screenshots `window-bullets-*` (a nested list) | Format › Promote (Alt+Left) and Demote (Alt+Right), on the toolbar with the theme's indent icons; Format › Alignment ▸, Bullets and Numbering ▸ (also the text right-click menu's), Frame ▸; a selected frame's keys win over a shortcut on the same key. The menu bar cannot be opened in the headless harness (libcosmic panics in `menu_inner`), so the folded menu's look is for the physical check |
| OL.5 Style Properties | DONE | (this commit) | `outline_level_reset_options_and_cumulative_numbers_set_the_style_ol2` (level, the exclusive reset pair, Cumulative building "I.1." from the level above and back, the cumulative text read back as parts); screenshots `window-bulletstyle-*` | Bullets and numbers gains Outline level (None, 1–9) and, for a level, Reset options: After higher level, After intervening style, Cumulative numbering (1.1.1); `BulletParts::prefix` holds the earlier levels' numbers |
| OL.6 Outline Numbering… | DONE | (this commit) | `numbering_schemes_number_levelled_styles_as_ami_pros_quick_numbering` (I, A, 1 by level like `_OUTLIN1.STY`; 1, 1.1, 1.1.1 cumulative; 1. A. i.; Clear; Text and bullet kept; no-level styles untouched), `outline_numbering_applies_a_scheme_to_the_levelled_styles_ol2` (through messages: 1, 1.1, 1.2, 2, 2.1 drawn, then II. and A., undo a scheme at a time, Apply refused without levelled styles); screenshots `window-outlinenumbering-*` | Format › Styles ▸ Outline Numbering… (`CommandId::OutlineNumbering`, after Modify Style as Ami Pro's Outline Styles…): four choices, the levelled styles listed, Apply as one undo step of `ModifyStyle`s |
| OL.7 Twins | DONE | (this commit) | `outline_twins` (ignored, `AMIPRO_VM_LAB`): OL1 (Demote on OL1B's Sub two, then Body one: Outline3 made with function key 12) and OL2 (Outline Numbering's 1, 1.1, 1.1.1 on OL2B) the same as Ami Pro's files, body and the changed `[tag]` records; the app's AOL1, AOL2, AOL3 opened in Ami Pro (`ol7-*`): Outline3 on Body one, 1 / 1.1 / 1.2 / 2 / 2.1, and the nested list 1. A. B. 2. as the app draws it | OL2's Number List is left out of the comparison: Ami Pro's button also clears numbers from styles without a level, the app's Outline Numbering does not (§22) |
| OL.8 Both formats and performance | DONE | `c68129b` | `nested_lists_and_outline_levels_made_in_the_app_write_as_ami_pro_writes_them` (cumulative numbers with After intervening style, value 8 = 28, read back equal, nothing lost, native exact), `outline_levels_and_cumulative_numbers_read_back_in_both_formats` (through messages: List 1–3 numbered 1, 1.1, 1.1.1, 2, 3 by Outline Numbering; native exact, SAM read back equal, nothing in the loss check, the copy drawn the same); `outline_performance_baseline` (ignored, release) | README31 to the drawn frame: Demote 2.8 ms, Promote 4.7 ms, Outline Numbering 20.6 ms and its undo 12.8 ms (every page relaid). Typing A/B: see the log. Final measures unchanged |
| OL.9 Exit | DONE | (the handoff commit) | `tools/verify.sh --release --corpus` PASS at `c68129b` (core 299 + 4; app 234, 106, 241 with 20, 4, 20 ignored) | §22's implementation notes; `FEATURE_PARITY.md` BL-02; `UI_COMMAND_MATRIX.md`; `UI_ACCEPTANCE.md` "Outline levels" and the sign-off rows; `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md` |

## Waiting for the user

- Nothing. Answered 2026-10-18: bullets nest (Bullet 2, Bullet 3…, the
  bullet going diamond, dot, small square), built after the run
  (`a_bulleted_list_nests_as_diamond_dot_square`,
  `promote_and_demote_nest_a_bulleted_list_as_diamond_dot_square`,
  `nested_bullets_made_in_the_app_write_as_ami_pro_writes_them`; VM AOL4
  from `outline_twins`, shown by Ami Pro as the app draws it).

## Baseline (at `outline-run-start`)

Measured at `outline-run-start`, 2026-10-17 (`target/ol-baseline/`, outside
Git): as at the end of the bullets run, with OL1 and OL2 added to the VM
folder.

- Round trip (read back equal): user's corpus 417 of 417, ISO 159 of 160,
  VM folder 203 of 203.
- Layout comparison: user's corpus page counts 303 of 394, lines 14,880 of
  15,356, page ends 761 of 1,108; ISO 65 of 160, 22,773 of 24,971, 492 of
  1,307; VM folder 202 of 202 page counts, page ends 86 of 86.

## Log

- 2026-10-17, OL.1 (before the run was set up): the Outline Styles dialog
  and the counting routine in the decompile; Outline mode's Promote and
  Demote in the VM (OL1); Quick Numbering's cumulative scheme (OL2). The
  user's answers O1–O4, and the Format menu folded at the user's remark.
- 2026-10-17, OL.2: levels counted by level and the style's reset
  options, cumulative codes; the layout comparison is identical to the
  baseline row for row on all three folders (number widths moved no
  compared line). Core 298 tests (with OL.3's two, uncommitted then).
- 2026-10-17, OL.4: the toolbar with Promote and Demote needs about 1,000
  px with the wide style field and 971 with the compact one; below 971
  the last two groups (the lists and Draw Frame) move to More, because
  More needs room of its own (the user had been told only Draw Frame
  would; the handoff corrects it). Core 299 tests; app 230, 106, 237.
- 2026-10-18, OL.7: the twins match Ami Pro's files, and Ami Pro shows
  the app's three files as its own; a nested list made in the app (List 1
  and List 2 below Title) numbers 1. A. B. 2. in both.
- 2026-10-18, OL.8: Promote, Demote, and Outline Numbering timed on
  README31 (above). Typing, interleaved release A/B of
  `page_surface_typing_performance_baseline` against `outline-run-start`
  (three rounds, README31's start, middle, and end): 0.65–0.66, 0.83–0.86,
  0.71–0.73 ms (base) and 0.68–0.75, 0.88–0.96, 0.79–0.84 ms (now), slower
  in every round; the same with Promote and Demote taken off the toolbar
  (two rounds); OL.2 against the end of the run, equal within 0.01 ms in
  update, so the step arrives with OL.2, whose only change on the typing
  path is the type of a paragraph's number, which README31 never computes
  (no numbered style): code placement in the release build, not work.
  0.02–0.06 ms a keystroke is far below a frame; left as is.
- 2026-10-18, final measures (`target/ol-final/`, outside Git): round trip
  417/417, 159/160, and 206/206 (the VM folder's three new bases OL1B,
  OL2B, OL3B); layout comparison identical to the baseline on both
  corpora, the VM folder's new bases one page each and equal.
