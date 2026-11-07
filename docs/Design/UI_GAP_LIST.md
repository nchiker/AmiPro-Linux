# UI Gap List — page surface against the approved mockup

**Started:** 2026-10-12, at `f1326fa`, for the UI pass the user moved ahead of
the rest of Phase 3 (`docs/research/DECISIONS.md`, 2026-10-12, "UI pass
before the rest of Phase 3"). It compares the `page-surface` build with
`UI_MOCKUP.svg` and `UI_SPEC.md` (Revision 2, approved 2026-10-10), region by
region. Each gap is closed by a change or accepted explicitly with the user
at the combined physical check that ends the pass.

**Screenshots.** The ignored test `window_screenshots`
(`app/cosmic/src/page_surface_app_tests.rs`) renders the whole window as
libcosmic's shell draws it: header bar, 7-pixel gutter (8 maximized), and the
theme's window background, in light and dark at 1280×800, maximized
1920×1080, and the 900×640 minimum, plus a selected frame:

```sh
AMIPRO_SCREENSHOT_DIR=/some/dir AMIPRO_PERF_SAM=$PWD/Reference/SAM/README31.SAM \
  cargo test --offline --release --all-features --manifest-path app/cosmic/Cargo.toml \
  window_screenshots -- --ignored --nocapture
```

It writes PPM files (convert with `ffmpeg -i x.ppm x.png`). The mockup renders
with `firefox --headless --screenshot mockup.png --window-size=1600,1100
file://…/UI_MOCKUP.svg`. Screenshots of shipped documents stay outside Git
(`../AmiPro-Reference/ui-screens/`). Headless rendering uses libcosmic's
embedded interface font and default themes, not the user's COSMIC settings,
so final colours and fonts are confirmed at the physical check.

**Severity:** **A** visibly wrong or contradicts an approved decision; **B**
missing an approved element; **C** polish.

## Window and workspace

| ID | Mockup and spec | Current (`f1326fa`) | Sev. | Plan |
|---|---|---|---|---|
| W1 | The canvas around the page is recessed: visibly darker than the chrome in both themes (§4 "Alignment rules", §5 "Page workspace") | The workspace is the window background, the same colour as the menu, action bar, and tabs; the page floats on chrome | A | Theme-derived canvas colour behind the pages and ruler |
| W2 | Paper edge: 1–2 px border with at least 3:1 contrast plus a restrained 3–6 px shadow | A faint top line and a right/bottom shadow; on a recessed canvas the edge needs a border all round | B | Border all round, shadow kept |
| W3 | A page caption outside the sheet (“Letter portrait · 100% · page 1”) is allowed once real pagination exists | None | C | Page number under or above each sheet; decide at the check |
| W4 | Optional nonprinting margin guides (dashed) | None | C | View › Margin Guides later, off by default |

## Menu bar and header

| ID | Mockup and spec | Current | Sev. | Plan |
|---|---|---|---|---|
| H1 | Title `Document name — AmiPro-Linux` | `README31 — AmiPro-Linux` (the tab and title drop the extension) | C | Keep; the spec does not require the extension |
| H3 | The title shows in the window's header bar | **None** (user, 2026-10-12: “I don't see a program title”): the app set only the compositor's window title; libcosmic's header bar draws a separate header title, never set. The first screenshots drew the title themselves and hid this | A | Set the header title with the window title; the harness draws what the app set |
| H2 | Eight menus, 30–34 px row, shortcuts right-aligned | Matches | — | — |
| H4 | Menu labels line up | View › Styles Panel was indented (user, 2026-10-12): the only entry with a check column; the Window menu mixed the same way | A | A menu with any checkable entry gives every entry the column |

## Action bar

| ID | Mockup and spec | Current | Sev. | Plan |
|---|---|---|---|---|
| A1 | Character group Bold, Italic, **Underline** | Bold and Italic only: Underline is not a command yet | B | Implement Format › Underline (Ctrl+U) on the page surface; show it in the group |
| A2 | **Paragraph group**: Align Left, Center, Align Right, Justify, with selected state | Absent: alignment is imported and laid out but has no command | B | Implement the four alignment commands (paragraph format, one undo step, selected state from the caret paragraph); show the group; responsive order per §5 |
| A3 | Open and Save icons recognizable without labels | Open and Save draw as up and down arrows in the default icon theme (upload/download) | A | Folder-open icon for Open and a save icon that reads as saving; check both themes |
| A4 | Toggle buttons show selected state as a filled button (B highlighted in the mockup) | Bold and Italic use the button's selected state; plain at rest | C | Confirm at the check |
| A5 | Style field 168–220 wide, middle/end ellipsis | Matches (`Body Text` dropdown) | — | — |
| A7 | The toolbar is editable (user requirement, 2026-10-12; Ami Pro's SmartIcons; `UI_SPEC.md` §12) | A fixed set of groups | B | Data-driven bar with the Standard set, generic overflow, Tools › Customize Toolbar…, sets in the user's configuration, View › Show Toolbar |
| A6 | `More` only below the required width (§4) | Always shown, holding Save As, Paste as Plain Text, Select All, Styles Panel, and About (user, 2026-10-12: “I don't understand the More option”) | A | Show it only when groups are displaced, holding only them |

## Document tabs

| ID | Mockup and spec | Current | Sev. | Plan |
|---|---|---|---|---|
| T1 | Tab: icon, name, modified dot, close; `+` after the last tab | Matches with native libcosmic tabs | — | — |
| T2 | Tooltip with the full path and source type | Not offered by the pinned tab bar | C | Recorded deviation (`UI_ACCEPTANCE.md` §5) |

## Context rail (Styles and Frame Properties)

| ID | Mockup and spec | Current | Sev. | Plan |
|---|---|---|---|---|
| R1 | The rail is a distinct panel surface, 216 px by default (176–320), separated from the workspace | 200 px, no surface or edge: the list sits on the window background | A | Panel surface with a 1 px edge on the workspace side; 216 px |
| R2 | Header: `Styles`, a context label, and a close button | A bare `Styles` heading; closing only through View or More | B | Header row with title and close (`StylesPanel`) |
| R3 | Filter field (search) | None | B | Filter by name, live; Escape clears |
| R4 | Rows: full-width list rows with a ¶ marker, the current style highlighted as a row, no false selection for a mixed selection | Pill-shaped buttons (suggested and standard), one per style; the active one filled | A | List rows: ¶ marker, name, row highlight for the uniform current style, none when mixed |
| R5 | Visible by default at widths ≥ 1200 on a fresh profile; visibility and width remembered | Closed by default; not remembered | B | Open by default at ≥ 1200; remember visibility per profile |
| R6 | Below 1200 the rail opens as a non-modal overlay so the page is not squeezed | At 900 px the open rail squeezes the page | B | Overlay over the workspace below 1200 |
| R7 | Frame Properties uses the same width, with readable groups | Group headings render at title size (“Size and position” larger than the document text); the Placement and Text wrap dropdowns collapse to a few letters per line; the scroll bar overlaps the fields | A | Section headings at heading size, dropdowns full width, scroll bar outside the content |

## Ruler

| ID | Mockup and spec | Current | Sev. | Plan |
|---|---|---|---|---|
| U1 | A ruler strip with its own light surface and edge, spanning the page | Numbers and ticks drawn on the window background; a grey bar marks the text area | B | Ruler surface over the page width; text area as before |
| U2 | Indent markers for the caret paragraph (first line and left at the start, right at the end), read-only, from the model | None | B | Draw the caret paragraph's first-line, left, and right indents (model-backed; not draggable yet) |

## Status bar

| ID | Mockup and spec | Current | Sev. | Plan |
|---|---|---|---|---|
| S1 | `Page X of Y`, then the word count (and selection count) | `Page X of Y`, message | B | Word count of the document; selection count when text is selected |
| S2 | Optional `Section N` | None (no sections in the model) | C | Not applicable |
| S3 | Zoom out, slider, percentage, zoom in at the right | Matches | — | — |
| S4 | Ami Pro's page arrows and Page Status button: previous page, the page number (opens Go To), next page (user, 2026-10-12: “a fast way to move by pages”) | `Page X of Y` as text | B | Arrows and a page-number button; Edit › Go To (Ctrl+G) with page, first, last, next frame |

## Themes

| ID | Mockup and spec | Current | Sev. | Plan |
|---|---|---|---|---|
| D1 | Dark theme: chrome and canvas use theme colours, paper stays light, contrast at the paper edge ≥ 3:1 | Paper light; canvas the same as chrome (W1); ruler grey bar low contrast | A | Covered by W1, W2, U1; checked in both themes |

## Default build

| ID | Item | Plan |
|---|---|---|
| B1 | The page surface is behind the `page-surface` feature; the default build still uses the older text editor | At the combined physical check, the user decides whether the page surface becomes the default. Its remaining gates before that: accessibility nodes, visual IME preedit, picture decoding (`PROJECT_PLAN.md`) |

## Progress

Each closed gap names its commit here.

- **W1, W2** (canvas and paper edge): the pages lie on a recessed canvas,
  the window background darkened (×0.8 light, ×0.6 dark), so the paper's
  1-pixel edge and shadow read in both themes. D1 is closed with them except
  the ruler's contrast (U1).
- **R1–R4, R6, R7** (context rail): one 216-pixel surface for Styles and
  Frame Properties (the theme's primary container with an edge); a header
  with the close button; a live filter; list rows with ¶ markers and the
  current style selected (none when mixed); on windows narrower than 1,200
  pixels the rail starts closed and opens over the workspace, which keeps its
  width; the inspector's sections at heading size, its lists full width, and
  its scroll bar beside the fields. Tests:
  `the_rail_docks_at_its_width_and_overlays_narrow_windows`,
  `the_styles_filter_keeps_matching_names_in_order`.
- **R5** (default visibility): open by default on desktop-width windows;
  remembering visibility and width between runs is still open.
- **A3** (Open and Save icons): a folder and a floppy disk
  (`folder-open-symbolic`, `media-floppy-symbolic`) instead of the up and
  down arrows.
- **S1** (word count): `7,171 words`, or `4 of 7,171 words` for a selection,
  after the page number; counted per paragraph and recounted only for
  changed paragraphs. Interleaved A/B of page-surface typing on README31
  against `d6885dc`: about +0.04 ms median at the first position (0.44–0.52
  against 0.49–0.58 ms), the other positions level.
- **U1, U2** (ruler): a paper strip over the page width with shaded margins,
  quarter-inch ticks, numbered inches, and the caret paragraph's first-line,
  left, and right indent markers (read-only). Test:
  `the_ruler_marks_the_caret_paragraphs_indents`.
- **A6** (More): shown only when a narrow window displaces groups, holding
  only them.
- **H3** (title): the header bar shows `Document — AmiPro-Linux`; the
  screenshot harness now draws the header title the app set. Test:
  `window_title_tracks_new_switch_and_close_transitions`.
- **A6, revised** (More on a mid-size window, user report 2026-10-12): the
  bar is data-driven (`ToolbarItem`, `STANDARD_TOOLBAR`), and groups move to
  More only when the measured widths do not fit, never at fixed window
  widths; the Standard bar needs 608 px, so More cannot appear at the
  900-pixel minimum. Tests: `the_action_bar_widths_match_the_real_widgets`
  (prediction within 2 px of the real widgets),
  `the_standard_bar_fits_any_supported_window_without_more`,
  `narrow_bars_narrow_the_style_field_then_move_whole_groups_from_the_end`.
- **S4** (page arrows and Go To): `▲ Page X of Y ▼` in the status bar; the
  arrows move the caret to the top of the previous or next page's text and
  scroll there, disabled at the ends; the page number opens Edit › Go To
  (Ctrl+G, as in Ami Pro): a page number refused in place when out of
  range, First page, Last page, and Next frame. Select Next Frame keeps its
  Edit menu item without a shortcut. Test:
  `the_status_bar_page_arrows_and_go_to_move_by_pages`. To confirm at the
  check: the disabled arrow's look.
- **H4** (menu alignment): View and Window give every entry the check
  column, so labels line up; the other menus have none. Test:
  `menus_with_a_check_line_up_every_label`. The open menu is not in the
  screenshot harness (an overlay); to confirm at the check.
- **S4, fixed after the user's test** (the arrows moved the caret but the
  view stayed): the scroll reached the page workspace (shown headlessly),
  but the pinned runtime (libcosmic `dbc62f2`, iced winit `run_action`)
  applies a widget operation without requesting a redraw and skips the
  frame when no message or event is pending. Every workspace scroll (page
  arrows and Go To, the caret kept in view while typing, and tab switches)
  now goes through `scroll_workspace`, which follows the scroll with a
  no-op `Message::Repaint`. Test:
  `the_page_arrows_scroll_the_view_and_ask_for_a_repaint`. The physical
  result needs the user's recheck. The disabled arrow's look is accepted by
  the user (2026-10-12).
- **S4, physical recheck PASS** (user, 2026-10-12): the arrows now move the
  view. The `AMIPRO_TRACE_SCROLL` log of that run shows each move scrolling
  to its page's top (1,088, 2,168, 3,248 … one page plus gap apart), a
  repaint after each, and the workspace reporting the new offset before the
  next move. The run before it still showed the old behaviour; which build
  it used is not known.
- **S4, that PASS was premature** (user, 2026-10-12, debug build with the
  trace): in a document opened into a new tab the arrows moved the caret
  but not the view until the window was resized. The trace shows the new
  tab's viewport as 0 × 0 and every scroll missing. Cause: each tab's
  workspace scrollable had its own id (`document-scroll-<session>`), and
  the pinned libcosmic's tree diff (`iced/core/src/widget/tree.rs`,
  `Tree::diff`) gives a scrollable the id it first had at its place in the
  tree (`Scrollable::id` is an id set, which the diff copies back from the
  old tree), so a tab opened later kept the first tab's id and its scrolls
  matched nothing; the release run worked because its document was the
  first tab. Tab switches restored no offset for the same reason. Fixed:
  one workspace id for every tab (`WORKSPACE_SCROLL`); `update` follows a
  tab change by restoring that tab's offset and giving it the workspace's
  size (`follow_active_session`). Tests, through the whole window in the
  runtime's order (update, rebuild from the cache, then the operations):
  `a_tab_opened_later_scrolls_with_the_page_arrows`,
  `each_tab_shows_its_own_scroll_offset`.
- **S4, physical PASS** (user, 2026-10-12, debug build, `c0c8185`): “it
  works now”: the arrows and Go To move the view in a document opened into
  a new tab, with no resize.
- **A7** (editable toolbar, one toolbar at the top, as the user decided):
  Tools › Customize Toolbar… (Available and Toolbar lists; Add, Remove,
  Move Up, Move Down, Add Separator, Reset to Standard, Done), View ›
  Toolbar to hide or show the bar, and the toolbar saved in the user's COSMIC
  configuration (`UI_SPEC.md` §12, "Implemented"). Customizable commands
  gained icons from the COSMIC theme. Tests:
  `customize_toolbar_adds_after_the_selection_moves_removes_and_resets`,
  `a_saved_toolbar_reads_back_without_what_this_build_cannot_show`,
  `customize_toolbar_offers_the_style_field_and_implemented_commands_with_icons`,
  `view_toolbar_hides_the_bar_and_is_checked_in_the_view_menu`,
  `customize_toolbar_changes_the_bar_and_the_next_start_keeps_it` (a scratch
  configuration directory). Screenshots `window-customize-light` and `-dark`;
  the harness now draws an open dialog as `view_main` does (a modal
  popover). Physical check pending.
- **A1, A2** (Underline and the Paragraph group): Format › Underline
  (Ctrl+U) toggles the page selection's underline; Align Left, Center,
  Align Right, and Justify (Ctrl+L, E, R, J) set the selected paragraphs'
  alignment over their styles' through a new editing command,
  `SetParagraphAlignment` (one undo step; `None` returns a paragraph to its
  style's). Both are page-surface commands (the default build's editor
  cannot draw them) and join the Standard set there: the bar needs about
  800 px and still fits the 900 px minimum window without More. The
  approved order is `REVISION_2_TOOLBAR`; `standard_toolbar()` leaves out
  what the build cannot run. Found on the way: the pinned libcosmic's icon
  style ignores `selected` on an enabled icon button, so Bold and Italic
  never showed their state; toggles now use the theme's selected background
  and accent icon (`toggle_button_class`). Tests:
  `underline_toggles_on_the_page_selection_in_one_undo_step`,
  `the_alignment_commands_align_the_selected_paragraphs_and_show_which`
  (centred, right, and justified lines measured in the layout),
  `the_standard_bar_has_underline_and_the_paragraph_group_with_their_states`,
  `the_standard_set_is_the_approved_order_less_what_the_build_cannot_run`,
  and the editing crate's
  `paragraph_alignment_is_set_over_the_style_and_cleared_in_one_undo_step`.
  Physical check pending. The menu items carry no check marks (as Bold and
  Italic); Ami Pro's Text menu checks the current alignment: to decide at
  the check.
- **Combined physical check, PASS** (user, 2026-10-12, after `c4aec53`):
  “This all appears to work great”: Customize Toolbar and View › Toolbar
  (kept after a restart), Underline, the alignments, and the toggles'
  selected states. Decisions: the page surface is the normal build (B1),
  and every on/off command gets a check mark like View › Styles Panel:
  Format › Bold, Italic, Underline, and the four alignments now show
  theirs (`menu_check`; test
  `the_format_menu_checks_bold_italic_underline_and_the_alignment`). Their
  states, and the toolbar's, now follow the page selection in its own flow
  (frame, header, footer, or cell text too).
- **R5, visibility** remembered: the docked rail's visibility is saved with
  the toolbar (`styles_visible`); a fresh profile still opens it. Its width
  is fixed (216), so there is no width to remember yet.

