# Frame Creation Progress

Live state of `docs/FRAME_CREATION_RUNBOOK.md`. Resume at the first slice
that is not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| FC.1 The map | DONE | `34a6dbb` | `docs/research/FRAME_CREATION_NOTES.md`; VM FC1–FC11; `tools/ne_resources.py … menus` | Create Frame `0x19e` → dialog `ADDFRAMEDLG`, OK `0x19d`, Manual `0x199`; the default frame (`1018:0eeb`, AMIPRO.INI `FrDefs`), Make Default `0x19f`, the remembered frame (`frmcrt`); Where placed frames above anchored ones (FC8) |
| FC.2 Default frame and creation rules | DONE | (this commit) | `crates/editing/src/frame_creation.rs` (6 tests: FC1, FC3, FC6N, `FrDefs` numbers, the dialog's start and limits with FC2 and FC5, drawn rectangles with FC2 and FC4) | `FrameDefaults` (`AMI_PRO`, `from_style`, `frame_style`, `to_numbers`/`from_numbers` in `FrDefs` order, so the app keeps it in its settings), `FrameGeometry` (`FIRST`, `starting`, `limits`, `remembered`), `drawn_rect`; the app shows inches throughout, so the default's unit is always 1 |
| FC.3 Layering (F3) | DONE | (this commit) | `anchored_frames_are_drawn_below_where_placed_ones_fc8` (engine), `z_order_moves_a_frame_within_its_kind_fc8`, `a_where_placed_frame_made_after_an_anchored_one_is_written_first_fc7` (SAM after edits), the random undo test, the group validation test; round trip and layout comparison identical but for one file (log) | The engine draws anchored frames first (`page_frame_z`), so the surface's hit test and Tab order follow; `restack` moves a frame within its kind and keeps the other kind's places; the writer, its picture directory, `[files]`, and the round-trip description use Ami Pro's order (`page_frames_in_file_order`); a group is adjacent among the Where placed frames |
| FC.4 The commands and the dialog | DONE | (this commit) | `page_dialogs` tests `create_frame_starts_from_the_last_frame_and_makes_fc1`, `create_frame_refuses_a_frame_off_the_paper_fc5`; app test `insert_frame_makes_ami_pros_frame_on_the_page_in_view_fc1_fc9` (through messages: page in view, Ami Pro's frame, selected, caret kept, remembered, one undo step, Draw on Page); toolbar tests; screenshots `window-createframe-light`, `-dark` (`frame_creation_screenshots`) | Insert › Frame… (the dialog: Width, Height, Down from top, In from left; Create, Draw on Page, Cancel) and Insert › Draw Frame (icon `insert-rectangle-symbolic`, last on the Standard toolbar); the page in view is the page at the workspace's centre; `Preferences` keeps the last frame and the default frame (`last_frame`, `frame_defaults`: Ami Pro's `frmcrt` and `FrDefs` numbers) |
| FC.5 Drawing on the page | DONE | (this commit) | Window tests `draw_frame_draws_ami_pros_frame_through_the_window_fc2_fc4` (FC2's drag from bottom right to top left to within a pixel, Ami Pro's frame, selected, caret kept, one undo step; FC4's click and FC11's Esc end drawing) and `a_frame_drawn_on_a_scrolled_zoomed_page_stays_on_its_paper_over_other_frames` (50%, scrolled to page 2, past the right edge) | The page surface's `drawing_frame`: a crosshair over the pages; a press on a page (frames there included) starts a dashed outline kept on that paper; the release sends `PageAction::FrameDrawn`; a drag under 3 px (a click), Esc, or a press outside every page ends drawing without a frame |
| FC.6 Make Default (F2) | DONE | (this commit) | `make_default_for_new_frames_keeps_what_ami_pro_keeps_fc6n` (through messages), `the_last_frame_and_the_frame_default_are_saved_for_the_next_start` (a scratch settings directory); screenshots `window-newframe-light`, `-dark` | Frame Properties ends with New frames: Make Default (Ami Pro's label: "Make Default for New Frames" does not fit the 216 px rail) and Restore Ami Pro's Default (disabled while the default is Ami Pro's), with captions saying what is kept |
| FC.7 Twins | DONE | `adb3ece`, (this commit) | `frame_twins` (ignored; `AMIPRO_VM_LAB`): FC1, FC2, FC3, FC6N, FC8, FC9 equal to Ami Pro's frame records and bodies; in the VM Ami Pro shows each `AFC*` twin as its own (7 of 7 screens identical in the document area, FC9 on both pages); `text_insets_are_ami_pros_f01_fc1_fc2_fc3` | Three writer rules found: a page frame's `[frmlay]` defaults (`PAGE_FRAME_DEFAULTS`, importer and writer), a new frame's empty `[txt]` (kept `txt none` until typed in; the round-trip description drops it once typed), and Ami Pro's corner inset with the line width (`FrameStyle::text_insets`, `1530:0892`, used by the writer, the importer, and the engine) |
| FC.8 Both formats and performance | DONE | (this commit) | `created_frames_read_back_in_both_formats` (frames from the dialog, drawing, and Make Default, one typed in: native v3 exact, SAM read back equal, nothing in the loss check, empty frames written as `[txt]` without paragraphs); `frame_creation_performance_baseline` (ignored, release); round trip and layout comparison as after FC.7 (nothing since moves text or changes files); typing A/B | Timings below |
| FC.9 Exit | DONE | (this commit) | `UI_SPEC.md` §20 implementation notes; `FEATURE_PARITY.md` FR-01; `UI_COMMAND_MATRIX.md`; `UI_ACCEPTANCE.md` §10 (Creating frames); `DECISIONS.md`; `PROJECT_PLAN.md`; `PROJECT_STATUS.md`; `CLAUDE.md`; handoff; `tools/verify.sh --release --corpus` PASS | The user's physical check of creating, moving, and resizing frames is next (`docs/SESSION_HANDOFF.md`) |

## The map (from FC.1)

What each slice builds, by `FRAME_CREATION_NOTES.md`:

- **FC.2.** In `amipro-editing` (a new `frame_creation` module):
  `FrameDefaults` (the 17 values: sides, line style, position, unit, line
  colour, background, opaque, wrap, lines on, margins left, bottom, top,
  right, shadow extents and colour, corners) with `AMI_PRO` built in
  (`1018:0eeb`); `FrameDefaults::new_frame_style(rect)` (`1260:0827`: one
  column, no tabs, margins dropped when left + bottom ≥ width, or top +
  bottom ≥ the frame's bottom edge); `FrameDefaults::from_style`
  (`12b0:0465`: keeps wrap, opaque, lines on, no placement); the dialog's
  `CreateFrameGeometry` (`1128:0000`: remembered or 2880 × 2880 at 6480,
  4680; top past the page → page height − 72, left → page width − 144;
  width and height clamped to the paper) with each box's limits
  (`1360:0382`) and the remembered size (at least 360, `1260:01cf`).
- **FC.3.** `amipro-layout` draws page frames by (kind, stack index):
  anchored first, Where placed above; the page surface's frame hit test,
  Tab order, and handles use the same order; `BringToFront` and
  `SendToBack` move within the frame's kind; `write_page_frames` writes
  Where placed frames, then anchored ones (`11c8:3ba0`); anchor numbers
  unchanged (they count anchored frames only).
- **FC.4.** `CommandId::Frame` implemented (Insert › Frame…), new
  `CommandId::DrawFrame` (Insert › Draw Frame) with an icon and a place in
  the Standard toolbar after the paragraph group; `create_frame_dialog.rs`
  (or in `page_dialogs.rs`): the four lengths, limits, Create, Draw on
  Page, Cancel; `Preferences` gains the last frame (`frmcrt`) and the
  default frame (for FC.6); the page in view.
- **FC.5.** Page surface drawing mode: a `FrameDraw` state like the
  frame-move drag (start page, corners, dashed outline), crosshair,
  status-bar text, release → `Command::CreateFrame` → select the new frame
  (frame interaction's selection), the remembered frame updated.
- **FC.6.** Frame Properties: Make Default for New Frames, Restore Ami
  Pro's Default; `Preferences::frame_defaults` saved like the toolbar.
- **FC.7–FC.9** as the runbook says.

## Baseline (at `frame-run-start`)

Measured at `frame-run-start`, 2026-10-17 (outputs kept in
`target/fc-baseline/`, outside Git):

- Round trip (`sam_round_trip`, read back equal; with `AMIPRO_STYLES`):
  user's corpus 417 of 417, ISO 159 of 160 (the malformed layout), VM
  folder 194 of 194 (the FC fixtures included); Save As would list
  nothing in the same counts.
- Layout comparison (`layout_comparison_report`): user's corpus page
  counts 303 of 394, paragraph line counts 14,880 of 15,356, page ends 761
  of 1,108; VM page counts 194 of 194, lines 989 of 1,000, page ends 85 of
  85; ISO page counts 65 of 160, lines 22,773 of 24,971, page ends 492 of
  1,307.

## Log

- 2026-10-17, FC.1 (before the run was set up): the handler and dialog
  mapped in the decompile (through the command table; `ne_resources.py`
  now lists menus), confirmed by fixtures FC1–FC11 in the VM. The VM's
  default frame was changed for FC6 and put back by Make Default on a
  frame with the built-in settings. Found on the way: Ami Pro draws Where
  placed frames above anchored ones (FC8), which the app does not (now
  decision F3); a With para above frame narrows beside a Wrap around frame
  (FC7; not in this run). The user took the four recommendations (F1–F4).
- 2026-10-17, FC.3: corpus scan of `[frm]` order by kind: frames in table
  cells (`0x20`), Where placed, anchored ('F', 'FA', 'A', 'CA', 'CFA') in
  every file of both corpora and the VM folder but one of the user's
  ('CAF', 10 cell frames, 36 anchored, then 4 Where placed), so the writer
  writes the non-anchored frames, then the anchored ones. The random undo
  test then found that moving a group within its kind could leave an
  anchored frame between its members in the stack, which validation
  refused; groups are runs in Ami Pro's Where placed list, so validation
  now looks at the Where placed frames only. Measured: round trip read
  back equal unchanged everywhere (417/417, 159/160, 194/194); the user's
  corpus's sections equal to the original 51,234 → 51,218 of 53,352, all
  in the 'CAF' file, now written in Ami Pro 3.1's order (the audit without
  that file is identical before and after); layout comparison identical
  on all three folders (drawing order moves no text).
- 2026-10-17, after FC.6: the screenshot of a new frame over the fixture's
  frame showed two drawing gaps that every created frame meets. (1) Frames
  were drawn square: the surface ignored the rounded-corner percentage,
  and Ami Pro's default frame has 18 %; the shadow, fill, and (with all
  four lines) the border are now rounded quads, radius corner% × half the
  shorter side (Ami Pro draws an ellipse's quarter at 100 % on unequal
  sides; a circle's is drawn). (2) An opaque frame did not hide the text
  behind it, main text or another frame's: within one renderer layer every
  quad is drawn before any text. Each page frame now has a layer of its
  own, with the paper edge and the ruler guide above them and the frame
  selection, drawing outline, and caret above everything; pages without
  page frames draw as before. Test `an_opaque_frame_hides_the_text_behind_it`
  (fails without the layers). Layout cannot move (drawing only); the
  typing timing is measured in FC.8.
- 2026-10-17, FC.7 (session paused here: the user had to move the
  laptop): bases FCB and FC9B saved in the VM (each fixture with its frame
  deleted in Ami Pro); `frame_twins` drives the app from Ami Pro's files.
  First run: FC8 the same; the others differ in three ways, to fix next:
  (1) a new frame's `[frmlay]` values 10 and 12–14 are written as a
  header's (`1`, `1 1 0`) where Ami Pro writes `2` and `0 0 0` (the
  writer's defaults for frames without kept values); (2) a new frame's
  empty text is written as an empty paragraph where Ami Pro writes a
  `[txt]` with none (mark new text frames as never typed in, as the
  importer marks `[txt]` without paragraphs); (3) FC1's body: the app
  writes paragraph caches (`<:#568,9360>`) that FCB's original did not
  have (FCB was written without caches; check why the app adds them for
  paragraphs Ami Pro wrote without, decompile `11b0:1405` and 4.9's cache
  rule). Then FC2, FC3, FC6N, and FC9 again (FC6N also orders sequence
  numbers 4, 5, 7, 6: check field 13), and Ami Pro opening the `AFC*`
  twins (`fcb-open` is Ami Pro's own FC1 screen to compare with). The VM
  was saved (`VBoxManage controlvm "Windows XP" savestate`) with Ami Pro
  open and no document changed.
- 2026-10-17, FC.7 (resumed): the three differences fixed. (1) Page frames
  have their own `[frmlay]` defaults (`2 0 0 0 0`, the commonest in both
  corpora); importer and writer agree, so the round trip is unchanged;
  native files saved before this keep page frames' values 10 to 14 that
  equalled a header's only implicitly, and a SAM written from them now
  says `2 … 0 0 0` there (stale values Ami Pro ignores). (2) A new text
  frame keeps `txt none` (as the importer marks a `[txt]` without
  paragraphs); the round-trip description leaves it out once the text is
  typed in (an imported empty frame typed into would otherwise have been a
  false loss). (3) FC1's body differed only by paragraph caches that Ami
  Pro wrote into FCB when it saved the opened FC1; the twins compare
  bodies without caches. FC2 and FC3 then showed Ami Pro's corner inset:
  57 where r(1 − cos 45°) gives 56, and 10 for FC3's tiny frame; decompiled
  (`1530:0892`, `1530:0c22`, `1288:0800`) and now one rule in the model
  used by the writer, the importer, and the engine. Measured: round trip
  as after FC.3 (user's 417/417, ISO 159/160, VM 196/196 with FCB and
  FC9B); layout comparison summaries identical on all three folders; text
  frames' column edges closer to Ami Pro's stored ones in 15 files and
  further in none (all exact: VM 53 → 59, ISO 6 → 7, user's 101 → 104).
  In the VM Ami Pro opened the six twins and showed them pixel for pixel
  as its own files.
- 2026-10-17, FC.8 timings (release, 1280 × 800, from the message to the
  next frame drawn, 20 samples): README31 (344 paragraphs, 14 pages) the
  dialog's Create 3.8 ms median (4.6 p95), a drawing's release 3.8 ms
  (4.0), undo 3.6 ms; the user's document with the most paragraphs (883
  paragraphs, 30 pages) 8.4 ms, 8.5 ms, and 8.3 ms. Typing
  (`page_surface_typing_performance_baseline`, README31, against
  `frame-run-start`): a first interleaved session (three rounds) measured
  +0.12 ms at the document's start (0.59–0.64 against 0.73–0.76 ms) and
  +0.02 to +0.05 elsewhere; a second session of twelve interleaved runs,
  with a third build without Draw Frame on the toolbar, did not reproduce
  it: medians at the start 0.69 (start of the run), 0.72 (now), 0.69 (no
  toolbar item); in the middle 0.87, 0.83, 0.90; at the end 0.72, 0.74,
  0.77. Read as host drift (powersave governor, other workloads); typing
  stays near 0.7 ms.

## Exit (2026-10-17)

Every slice DONE. `tools/verify.sh --release --corpus` PASS at `27daf73`:
core 279 + 4 ignored; app 213, 106, 220 with 15, 4, 15 ignored; ISO corpus
gate. Waiting for the user: the physical check of creating, moving, and
resizing frames (`docs/Design/UI_ACCEPTANCE.md` §10). No question came up
that F1–F4 did not cover (choices in `DECISIONS.md`, 2026-10-17).

## After the run (the user's first use, 2026-10-17)

- "Restore Ami Pro's defaults – text in the side panel is cut off": the
  label did not fit the 216 px rail in the user's interface font (it just
  fitted in the tests' font). The New frames buttons are now rail buttons
  that wrap their labels (`rail_button`; libcosmic's standard button keeps
  one line at a fixed height), and the label is "Restore Default". Test
  `rail_buttons_wrap_their_labels_rather_than_cut_them_off`.
- Asked how to turn off rounded corners: Frame Properties › Rounded
  corners (%) 0 (Ami Pro's Square corners).
- "I set it for 2 columns in the frame and line between columns – I don't
  see a line": frames never drew their line between columns (only page
  layouts did, since PL.6). In Ami Pro the line is centred in each gutter
  and runs the frame's text area, text or not (VM C04, checked on its
  screen). The engine now gives a text frame's box its strokes (the page
  lines' pattern code, `between_column_strokes`) and the surface draws them
  with the frame. Tests `a_frames_line_between_columns_runs_down_its_gutter_c04`
  (engine) and `a_frames_line_between_columns_is_drawn_c04` (pixels; fails
  without the drawing); `c04_screenshot` shows the app's C04 beside Ami
  Pro's. Drawing only: no text moves.
- After both fixes, the user: "the line shows now, the label looks good.
  I'm able to draw frames, edit and manually create frames." **Physical
  check of creating frames: PASS** (drawing, the Create Frame dialog, Frame
  Properties on new frames). Moving and resizing were still to check.
- Then: "moving and resizing work, grouping frames, delete frames, undo,
  bring to front and back, make default carries to the next frame."
  **The physical check of creating, moving, and resizing frames: PASS**
  (2026-10-17), and with it frame interaction's own check, pending since
  2026-10-12 (`UI_ACCEPTANCE.md` §10 records what was not separately
  reported).

