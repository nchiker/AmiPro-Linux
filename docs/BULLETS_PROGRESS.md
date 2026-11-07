# Bullets and Numbering Progress

Live state of `docs/BULLETS_RUNBOOK.md`. Resume at the first slice that
is not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| BN.1 The map | DONE | (setup commits) | `docs/research/BULLETS_NOTES.md`; VM BL1; `UI_SPEC.md` §21; B1–B5 | No list command in Ami Pro; Insert Bullet `0x344` → `<:B<*X>>`; Right align `1108:4107`; Space for is the First indent |
| BN.2 Inserted bullets in the model and the formats | DONE | (this commit) | `inserted_bullets_become_objects_bl1` (importer: the 17 codes, `<**>` as 0x0A; other forms stay marks), `inserted_bullets_are_written_as_ami_pro_writes_them_bl1` (body writer, read back equal), `inserted_bullets_round_trip_in_native_files_and_are_strict`, `inserted_bullets_go_into_any_flow_in_the_formatting_where_they_stand`, the random undo test with `InsertBullet`; round trip identical to the baseline line for line | `InlineObject::Bullet` and `BULLET_CODES` (Ami Pro's list order) in the model, allowed in any flow; native `inline bullet CODE`; zero-width in the engine until BN.3 (as the legacy mark was) |
| BN.3 Inserted bullets laid out and drawn | DONE | (this commit) | `inserted_bullets_take_the_size_where_they_stand_bl1` (engine: the style bullet's width in the font where it stands, the line that font's height), `inserted_bullets_draw_in_their_box_and_take_the_caret_as_one_character_bl1` (surface: drawn in its word box in its text's colour, the caret before and after it, clicks on either half, at a line's end), `inserted_bullets_are_one_character_and_copy_as_text_bl1`; ignored `sam_screenshot` (any SAM's first page); layout comparison identical row for row | `bullets::bullet_metrics` and `inline_bullet_width` shared by style and inserted bullets; `shape_marks` draws both (`fill_bullet_mark`); a copied bullet is its nearest Unicode character (plain-text clipboard) |
| BN.4 Right align, superscript, pictures | DONE | (this commit) | `right_aligned_numbers_end_a_space_before_the_text_1108_4107`, `superscript_bullet_text_is_set_smaller_but_not_asterisks` (engine), `picture_bullets_are_drawn_as_shapes_in_their_box_bl1`, `superscript_numbers_are_drawn_raised_and_right_aligned_bl2_bl3` (surface); VM fixtures BL2, BL3 and the app's screens of BL1–BL3 at 100 and 150 % compared with Ami Pro's; layout comparison identical (BL2 and BL3 added, equal) | `BulletBox` carries `right_align`, `space` (in the text's font), `superscript`; the surface draws superscript bullet text at 7/12 raised 5/12; `picture_marks` draws the seven pictures as the app's own shapes; every bullet is now marks (no glyph fallback) |
| BN.5 List styles in the model | DONE | (this commit) | `lists` tests: `styles_show_bullets_numbers_or_neither`, `the_sheets_list_styles_come_first_by_ami_pros_names`, `a_document_without_one_gets_ami_pros_default_list_styles` (one undo step takes the made style away), `toggling_turns_a_list_off_only_when_every_paragraph_shows_it`, `enter_in_an_empty_list_paragraph_ends_the_list`; `lists_and_inserted_bullets_made_in_the_app_write_as_ami_pro_writes_them` (SAM `[tag]`, `@Number List@`, `<:B<*9>>`, nothing in the loss check, native v3 exact); the random undo test with `SetList` | `amipro_editing::lists`: `ListKind`, `shown_list`, `paragraph_list`, `list_style` (Bullet 1, Bullet; Number List; only one that shows the kind), `default_list_style` (`_DEFAULT.STY`'s in Body Text's font), `toggle_list`, `ends_list`; `Command::SetList`; a taken name moves to Bullet, List Bullet (Numbered List, List Number) |
| BN.6 The commands in the app | DONE | (this commit) | `bullets_and_numbering_apply_ami_pros_list_styles_b1_b3` (through messages: a made Bullet 1, check marks, status, mixed selection, switching, off, undo step by step, Enter continuing and ending a list, the shortcuts, nothing with a frame selected); toolbar and right-click tests updated; `a_caret_after_a_selection_shows_its_own_paragraphs_style`; screenshots `window-bullets-light`, `-dark` (`bullets_screenshots`) | Format › Bullets (Ctrl+Shift+L) and Numbering (Ctrl+Shift+N) implemented with check marks; toolbar group after the alignments with the app's own icons (`app/cosmic/icons`, `chrome::app_icon_svg`, recoloured as symbolic); text right-click menu; Enter in an empty list paragraph ends the list; the Standard bar is about 935 px wide with the wide style field, which narrows below that, so it fits at the 900 px minimum without More |
| BN.7 Insert › Bullet… | DONE | (this commit) | `insert_bullet_offers_the_last_bullet_and_inserts_the_chosen_one_bl1` (dialog), `insert_bullet_puts_ami_pros_bullet_at_the_caret_or_over_the_selection_b4` (through messages: at the caret, over a selection, one undo step each, remembered, Cancel, Customize Toolbar), `the_last_bullet_is_saved_for_the_next_start` (scratch settings); screenshots `window-insertbullet-light`, `-dark` | Insert › Bullet… (`CommandId::InsertBullet`) opens a small dialog of the 17 bullets drawn by the page's own shapes (`BulletSwatch` over `page_surface::shape_marks`), with tooltips and a large example; `Preferences::last_bullet`; an icon of the app's own for Customize Toolbar; one list of names (`page_editing::BULLET_NAMES`) now also in Style Properties (the pictures named, not numbered) |
| BN.8 Style Properties | DONE | (this commit) | `bullet_and_number_style_opens_style_properties_at_its_bullets_b5` (the caret's style, only Bullets and numbers unfolded; Space for shows 0.25 for Number List and a change is one `ModifyStyle`); right-click menu test updated; screenshots `window-bulletstyle-light`, `-dark` | Format › Bullet and Number Style… (`CommandId::BulletNumberStyle`, also in the text right-click menu); Space for at the end of Bullets and numbers, the same box value as Alignment's First (`Field::First`), with a caption saying so |
| BN.9 Twins | DONE | (this commit) | `bullet_twins` (ignored; `AMIPRO_VM_LAB`): BL1 (20 bullets put back through Insert › Bullet…), BL2 (Numbering on Body Text paragraphs, the sheet's Number List), BL4 (Numbering on Ami Pro's base BL4B makes Number List; its whole `[tag]` equal to the one Ami Pro's Create Style made); in the VM Ami Pro shows ABL1, ABL2, ABL4 pixel for pixel as BL1, BL2, BL4; `function_keys_go_to_the_lowest_free_one_but_f10` | BL4 showed two values the made style had wrong: the function key (Ami Pro gives the next free one, `10a8:0000`, F2–F12 but F10; now also Create Style's) and `[spec]` value 4 (0; Ami Pro 2's Space for, `1058:0fac`) |
| BN.10 Both formats and performance | DONE | (this commit) | `lists_and_bullets_read_back_in_both_formats` (toggles, a made Bullet 1 and Number List, Enter ending a list, inserted bullets in the main text and a frame: native v3 exact, SAM read back equal, nothing in the loss check); `bullets_performance_baseline` (ignored, release); the round trip and the layout comparison on all three folders; typing A/B | Timings and measures below |
| BN.11 Exit | DONE | (this commit) | `UI_SPEC.md` §21 implementation notes; `FEATURE_PARITY.md` BL-01; `UI_COMMAND_MATRIX.md`; `UI_ACCEPTANCE.md` (Bullets and numbering); `DECISIONS.md`; `PROJECT_PLAN.md`; `PROJECT_STATUS.md`; `CLAUDE.md`; handoff; `tools/verify.sh --release --corpus` PASS | The user's physical check is next (`docs/SESSION_HANDOFF.md`) |

## The map (from BN.1)

What each slice builds, by `BULLETS_NOTES.md`:

- **BN.2.** `amipro_document::InlineObject::Bullet(char)` for the codes
  U+0008, U+000A–U+0019 (Ami Pro's list order: the ten shapes 0x10–0x19,
  then 0x08, 0x0A–0x0F); validation refuses others; native v3 `inline
  bullet CODE`; `amipro-formats` `text.rs` `control_meaning` `'B'`: the
  arguments are exactly one `<*X>` whose byte is one of the 17 codes
  (`<**>` there is 0x0A, not a hard return) → `PendingObject::Bullet`;
  `build.rs` resolves it; `body.rs` writes `<:B` + the code as body text
  writes a code + `>`; `roundtrip.rs` describes it. `Command::InsertBullet
  { at, code }` beside `InsertPageNumber`.
- **BN.3.** `amipro-layout` `engine.rs` words: a bullet object's width is
  `bullets::shape_width(code)` from the font's metrics at its byte
  (`format_at`), its ascent and descent the font's; `WordBox` carries the
  code for drawing; `page_surface.rs`: one `shape_marks` used by
  `bullet_marks` (style bullets) and inline bullets.
- **BN.4.** `bullets.rs`: Right align (`Bullets::flags` 16) moves the box
  right so it ends one space-width before All + First when narrower
  (`1108:4107`); superscript (attribute 256) raised as `1108:4063`
  raises superscript text (size: trace `1200:168b`); the pictures drawn by
  `shape_marks` as the app's own shapes.
- **BN.5.** `amipro-editing` `lists.rs`: `ListKind { Bullets, Numbering }`,
  `shown_list(&ParagraphStyle) -> Option<ListKind>` (a number code →
  Numbering; else a bullet code → Bullets), `list_style(&Document, kind) ->
  Option<StyleId>` (Bullet 1, Bullet; Number List; only one that shows the
  kind), `default_list_style(kind, &body_text)`, `Command::SetList`,
  `ends_list(&Document, TextPosition) -> Option<Command>`.
- **BN.6–BN.8.** `command.rs` (Bullets and Numbering implemented in the
  page surface, Ctrl+Shift+N, `BulletNumberStyle` new, `InsertBullet`
  new), `chrome.rs` (the Standard set and icons), `main.rs` (dispatch,
  check marks, the context menu, Enter), a small dialog in
  `page_dialogs.rs`, `style_properties.rs` (Space for, opening at a
  section).
- **BN.9–BN.11** as the runbook says.

## Baseline (at `bullets-run-start`)

Measured at `bullets-run-start`, 2026-10-17 (outputs kept in
`target/bn-baseline/`, outside Git), the same as at the end of the frame
creation run but for the new fixture BL1:

- Round trip (`sam_round_trip`, read back equal; with `AMIPRO_STYLES` for
  the user's corpus): user's corpus 417 of 417, ISO 159 of 160 (the
  malformed layout), VM folder 197 of 197; sections equal to the original
  51,218 of 53,352, 8,205 of 8,443, and 19,989 of 19,991.
- Layout comparison (`layout_comparison_report`): user's corpus page
  counts 303 of 394, paragraph line counts 14,880 of 15,356, page ends 761
  of 1,108; VM page counts 197 of 197, lines 992 of 1,003, page ends 86 of
  86; ISO page counts 65 of 160, lines 22,773 of 24,971, page ends 492 of
  1,307.

## Log

- 2026-10-17, BN.1 (before the run was set up): Ami Pro's menus and
  SmartIcons have no list command; Insert Bullet, Right align, and Space
  for mapped in the decompile; VM fixture BL1 (all 17 inserted bullets,
  their sizes, the dialog's list). The user's answers B1–B5 (with two
  questions of theirs answered first: both style and manual bullets
  exist; the work fits the side-panel model, hence B5).
- 2026-10-17, BN.2: inserted bullets are objects; the round-trip audit on
  all three folders is identical to the baseline in every line but the
  timings (user's 417/417, ISO 159/160, VM 197/197; sections 51,218,
  8,205, 19,989): the 20 inserted bullets of the corpora read as objects
  and are written back byte for byte. Core 284 tests (4 new); app 216,
  106, 223.
- 2026-10-17, BN.3: inserted bullets take their width and are drawn. The
  layout comparison is identical to the baseline row for row on all three
  folders: README31's inserted bullets are each followed by a tab, whose
  stop absorbs the bullet's width, and no other line with an inserted
  bullet wraps differently. The seven picture bullets draw nothing in the
  headless renderer (the arrowhead glyph's font is not loaded there),
  which BN.4 replaces with shapes. Core 285 tests; app 218, 106, 225.
- 2026-10-17, BN.4: VM fixtures BL2 (Number List's Right align: "9." and
  "10." end together) and BL3 (Superscript: small raised numbers; Right
  align's space in the reduced font, which the first implementation had
  in the full font). The decompile settles that Right align uses the
  paragraph's own indents when it has them (`1108:4107`'s caller) and that
  superscript bullet text takes the text font routine. Layout comparison
  identical on the user's and ISO corpora; VM folder 199 of 199 page
  counts with BL2 and BL3. Core 287 tests; app 220, 106, 227.
- 2026-10-17, BN.6: the first screenshot showed the style field saying
  "Mixed" with the caret in a Number List paragraph. The hidden text
  editor kept a selection made earlier, because the pinned iced
  `move_to` sets a selection only when given one and never clears it
  (`iced/graphics/src/text/editor.rs`); an older bug, visible after any
  selection followed by a click. The sync now clears it with the editor's
  own move first (regression test added). Core 293 tests; app 223, 106,
  230.
- 2026-10-17, BN.9: VM fixtures BL4B and BL4 (`_BASIC.STY`, Create Style
  Number List by hand). The first twin run found BL1 different only by an
  empty bold on and off (`<+!><-!>`) that the typing left in Ami Pro's
  file and no run holds (also why BL1's body is not byte-equal in the
  round-trip audit), so BL1's twin is compared with BL1 as read and
  written. The made Number List matched Ami Pro's after the function key
  and value 4 (above). Ami Pro opened the three twins and showed them
  pixel for pixel as its own. Core 294 tests; app 227, 106, 234.
- 2026-10-17, BN.10 timings (release, 1280 × 800, from the message to the
  next frame drawn, 20 samples): README31 (344 paragraphs, 14 pages)
  Numbering over three paragraphs 5.0 ms median (6.5 p95), its undo 4.7 ms,
  Insert › Bullet…'s Insert 1.2 ms; the user's document with the most
  paragraphs (883, 30 pages) 5.8, 5.7, and 5.0 ms. Typing
  (`page_surface_typing_performance_baseline`, README31, interleaved
  against `bullets-run-start`, three rounds): medians at the start 0.71,
  0.75, 0.71 (base) and 0.84, 0.86, 0.65 ms (now); middle 0.97–1.00 and
  0.97–0.98; end 0.80–0.84 and 0.80–0.90: unchanged within the host's
  noise. Final measures: round trip read back equal 417/417 (user's),
  159/160 (ISO), 201/201 (VM folder with BL2–BL4B); sections equal
  51,218, 8,205, and 20,316 of 20,318 (the same 2 as before); layout
  comparison identical row for row on the user's and ISO corpora (303/394
  page counts, 14,880/15,356 lines, 761/1,108 page ends; 65/160, 22,773/
  24,971, 492/1,307), VM folder 201/201 page counts, 992/1,003 lines,
  86/86 page ends (the four new fixtures equal).
