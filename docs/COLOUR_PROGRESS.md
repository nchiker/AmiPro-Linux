# Colour Selection Progress

Live state of `docs/COLOUR_RUNBOOK.md`. Resume at the first slice that is
not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| CS.1 The map | DONE | (setup commit) | `docs/research/COLOUR_NOTES.md`; VM (the Font dialog's bar and ▼ palette); `UI_SPEC.md` §23; C1–C4 | |
| CS.2 The colour model | DONE | `8d964a1` | `colours.rs` tests: conversions and names, `a_document_offers_its_colours_most_used_first` (styles weighed by paragraphs, runs, a frame's four colours, hidden ones left out), `recent_keeps_the_newest_first_without_repeats`; `recent_colours_are_saved_for_the_next_start` | Recent kept as `recent_colours` in the settings, two rows at most, one shown |
| CS.3 The colour button and panel | DONE | `8d964a1` | `colour_panels_offer_ami_pro_in_use_recent_and_the_picker` (the rows, a swatch one step, a three-step drag one step and undone at once, typed `#336699` on Enter, Recent's order, closing with the host); screenshots `window-colour-*` light and dark | `colour_control.rs`: `Target`, `Panel` (a fresh picker after each finished choice, so its own recent row never repeats the panel's), `ColourAction`, `Context`; the tab's `colour_drag` makes a drag's later changes amend its first step (`continues_colour_drag` in the three execution paths) |
| CS.4 Style Properties, Page Layout Properties, the Font dialog | DONE | `8d964a1` | the same test (text and rule colours, the Font dialog's OK); the existing Style Properties, Page Layout, and Font dialog tests unchanged | The old swatch rows and `character_dialogs::swatch` removed; the Font dialog's button disabled with Revert to style |
| CS.5 Frame Properties | DONE | `8d964a1` | `frame_colours_are_chosen_with_colour_buttons_all_four` (each one undo step; read back from SAM and native; nothing about the frame in the loss check); `a_frames_line_between_columns_reads_back_in_any_style_and_colour` | `Field::ColumnLineColour`; the `#RRGGBB` fields replaced. **Found and fixed:** a frame's line between columns in a style other than 2 made the loss check list the frame (the importer kept `[frmlay]` value 10 though the writer writes it) |
| CS.6 Twins | DONE | (this commit) | `colour_twins` (ignored, `AMIPRO_VM_LAB`): on OL3B, One's text `#336699` typed in the Font dialog's picker, Title's rule `#CC6600`, a frame's line `#996633`, shadow `#808080`, background `#FFF0C0`, and line between columns `#008000`, the page's lines `#663399`; nothing in the loss check; `ACS1.SAM` opened in Ami Pro (VM, `acs1`): every colour drawn exactly as written (sampled pixels), and as the app draws it | |
| CS.7 Both formats and performance | DONE | (the handoff commit) | Frame colours, a text colour, and a rule read back from SAM and native (`frame_colours_are_chosen_with_colour_buttons_all_four`, `colour_twins`); Recent across a restart (`recent_colours_are_saved_for_the_next_start`); `colour_panel_performance_baseline` (ignored, release); typing A/B (log) | README31: the document's colours 0.014 ms; opening a panel 0.03 ms; choosing a colour 16.2 ms (the style's paragraphs relaid, as any style change) |
| CS.8 Exit | DONE | (the handoff commit) | `tools/verify.sh --release --corpus` (log) | §23's implementation notes; `FEATURE_PARITY.md` UI-03; `UI_ACCEPTANCE.md` "Choosing colours" and its sign-off row; `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md` |

## Waiting for the user

Nothing. The user's physical check passed (2026-10-18): "colour buttons
all work", and nested bullets "look good".

## Baseline (at `colour-run-start`)

Measured 2026-10-18 (`target/bt-spacing/`, outside Git, at `510a937`,
nothing since moves text): round trip 417/417, 159/160, 208/208; layout
comparison user's corpus 303/394 page counts, 14,880 lines, 761 of 1,108
page ends; ISO 65/160, 22,773, 492 of 1,307; VM folder 207/207, 86 of 86.

## Log

- 2026-10-18, CS.5: a frame's line between columns turned on in the app
  made the loss check list the frame; bisected to `[frmlay]` value 10 kept
  as a raw value by the importer while the line is on (the writer writes
  it); kept now only while the line is off, and value 11 only while
  neither the line nor a border is on. Round trip and layout comparison
  identical to the baseline row for row afterwards (`target/cs-after/`).
- 2026-10-18, CS.7: typing, interleaved release A/B of
  `page_surface_typing_performance_baseline` against `colour-run-start`
  (three rounds, README31's start, middle, and end; in the one tree,
  checking out `app/` and `crates/` in turn, as the disk has no room for a
  second build): 0.811/0.914/0.948, 0.740/0.866/0.734, 0.714/0.857/0.742
  ms (base) and 0.676/0.850/0.737, 0.754/0.854/0.749, 0.836/0.958/0.766
  ms (now): faster in round 1, equal in round 2, slower in round 3, so no
  consistent difference (the typing path changed only by a check on
  colour drags in the style and frame paths).
