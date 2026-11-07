# Interface Acceptance Plan

**Status:** Revision 2 approved implementation baseline, 2026-10-10. Gate A is
closed. UI-1 is physically accepted for the functionality tested, with
Ctrl+Tab and untested checklist cases still open. UI-2 is implemented and
automated-green at `96aeda4`; its physical COSMIC acceptance is pending (see
the UI-2 focused checklist in section 5).
**Rule:** Material UI work is accepted only when deterministic state tests,
actual widget/event tests, and physical COSMIC visual/interaction checks all
pass. A build, launch, or screenshot alone is not acceptance.

## 1. Result vocabulary

- **PASS:** Every required check in the gate passed with evidence.
- **PARTIAL PASS:** Named checks passed, but listed checks remain or failed.
- **FAIL:** A required behavior or visual criterion failed.
- **PENDING:** No adequate evidence yet.

The current integrated interface is **FAIL / NOT ACCEPTED** based on the
2026-10-09 physical report: incomplete Edit menu, invisible shortcut labels,
confusing toolbar, duplicated style controls, poor hierarchy, status-bar
problems, and substantial divergence from the desired composition. The primary
rich-editor typing defect is separately **PHYSICALLY RESOLVED**: typing in
`README31.SAM` now feels normal. Unreported regression checks remain pending.

## 2. Required evidence record

For every implementation slice record:

- commit and build profile;
- Pop!_OS/COSMIC version, theme, display scale, resolution, and input method;
- document fixture and source-protection checksum where proprietary/private;
- automated commands with pass/fail counts;
- physical checklist result and exact symptoms;
- light- and dark-theme screenshots at the states named below;
- deviations from `UI_SPEC.md` and disposition.

Screenshots of private documents must not enter Git unless independently
authored/sanitized. Store private acceptance images outside the repository and
record descriptions or redacted derivatives only where licensing permits.

## 3. Gate A — specification approval

- [x] `UI_SPEC.md`, `UI_COMMAND_MATRIX.md`, and `UI_REFERENCE_COMPARISON.md`
  have no conflicting ownership or menu definitions.
- [x] The four structural decisions in `UI_SPEC.md` section 10 are approved in
  principle: style ownership, compact hierarchy, modern shortcuts, and
  responsive remembered Styles-panel visibility.
- [x] The revised `UI_MOCKUP.svg` receives explicit visual approval for paper,
  toolbar grouping, context-rail behavior, DTP character, and realistic content.
- [x] Missing Gemini HTML and unverified Ami Pro details remain labeled as
  limits, not silently inferred.

Gate A passed by explicit user approval of Revision 2 commit `2037467` on
2026-10-10. This approves the specification, not the implemented interface.

## 4. Gate B — command discovery and keyboard behavior

### Deterministic/model/controller

- [x] One authoritative command registry supplies menu label, shortcut,
  enabled predicate, toolbar/overflow route, and dispatch.
- [x] New/Open/Close/Save/Save As/Quit preserve per-tab state and safe imported
  source behavior.
- [x] Undo/Redo enabled state follows only the active tab's history.
- [x] Cut/Copy/Delete enable only for appropriate selections; Paste responds to
  focus and compatible clipboard availability; Select All handles empty and
  nonempty documents.
- [x] Bold/Italic and style application create one undoable semantic command and
  cannot leak to another tab.
- [x] Every displayed shortcut invokes the same command as the menu; no
  duplicate binding silently wins.

### Actual menu/widget path

- [ ] Open all eight real menu roots by pointer and keyboard.
- [ ] Traverse items with arrows, activate with Enter/Space, close with Escape,
  and switch adjacent roots while the menu is open.
- [ ] Shortcut text is visibly right-aligned; disabled items remain legible and
  cannot dispatch.
- [ ] Edit visibly contains Undo, Redo, Cut, Copy, Paste, Paste as Plain Text,
  Delete, Select All, Find, Find Next/Previous, Replace, and Go to Page in the
  specified groups. Future commands are specifically named and disabled.
- [ ] Focus changes and tab switches update menu/toolbar enabled state in the
  displayed widget tree, not only a controller snapshot.
- [x] Non-Latin keyboard layout fallback is exercised where feasible, following
  pinned libcosmic `KeyBind` behavior.

Automated result at `85086b8`: **PASS**. Registry/menu ordering, shortcut
uniqueness and labels, enabled predicates, menu/keyboard dispatch, clipboard,
selection, Undo/Redo, tab isolation, imported-source protection, dirty
close/quit, default-editor bindings, and a focused rich-widget non-Latin event
path pass. The pinned toolkit supplies trailing shortcut rendering and disabled
entry behavior. Actual opened-menu rendering/navigation remains physical.

Physical report, 2026-10-10: **ACCEPTED FOR TESTED FUNCTIONALITY**. All menus
and editing commands exercised by the user worked without an observed defect;
menu appearance and shortcut visibility were substantially improved.
Ctrl+Shift+Tab switched documents. Ctrl+Tab did not reach document switching
and appeared to be intercepted before the application. The report did not
enumerate every checklist case, so unchecked cases remain pending.

### Physical COSMIC

- [ ] A keyboard-only user can discover and execute every currently implemented
  editing/file command without memorized shortcuts.
- [ ] Visible labels match actual shortcuts for Ctrl+N/O/W/S/Shift+S/Q,
  Ctrl+Z/Shift+Z/X/C/V/A, Ctrl+B/I, and Ctrl+Tab/Shift+Tab as implemented.
- [ ] Menus remain on-screen and readable at 100% and 200% scale and near screen
  edges; no submenu or shortcut column is clipped.

Gate B fails if Edit still exposes only Undo/Redo or any implemented shortcut
lacks a menu route.

### UI-1 focused physical COSMIC checklist

Do not mark these from automated evidence:

- [ ] Open File, Edit, View, Insert, Format, Tools, Window, and Help by pointer
  and keyboard; verify grouping, readable named future items, and no generic
  placeholder rows.
- [ ] Verify applicable shortcuts appear at the right edge of menu rows and
  remain readable in both light and dark themes.
- [ ] Invoke New, Open, Close, Save, Save As, Quit, Undo, Redo, Cut, Copy,
  Paste, Paste as Plain Text, Delete, Select All, Bold, Italic, next/previous
  document, Styles, and About through every applicable menu and shortcut route.
- [ ] Perform real selection Cut/Copy/Paste/Delete/Select All, then edit and
  verify Undo/Redo exactly once per action.
- [ ] Open multiple documents; switch from Window's checked document list and
  Ctrl+Tab/Ctrl+Shift+Tab, then safely close clean and dirty tabs.
- [ ] Open an imported SAM; confirm Save requires a protected native Save As
  destination and cannot overwrite the SAM, then verify subsequent native Save.
- [ ] Confirm disabled commands cannot activate and do not show misleading
  placeholder feedback.
- [ ] Traverse menus with arrows, Enter/Space, Escape, adjacent roots, focus
  return, and keyboard-only navigation at 100% and 200% scale.
- [ ] In `README31.SAM`, spot-check typing, drag selection, wheel scrolling,
  caret placement/visibility, clipboard history, and independent tabs. Retain
  any untested P0 regression as pending rather than inferring success.

## 5. Gate C — action bar, tabs, and styles workflow

### Deterministic/model/controller

- [ ] The Styles panel and one action quick-apply field derive uniform/mixed
  style from the same active selection.
- [ ] The status model contains no second style selector.
- [ ] Panel visibility/width preference is application-level; current style and
  list selection are document-level and never persisted as independent truth.
- [ ] Active, inactive, dirty, imported-source, long-name, and close-pending tab
  states are deterministic and isolated.
- [ ] Responsive priority produces stable full, first-contraction, and narrow
  action layouts without hiding Open, Save, Undo/Redo, or the style field before
  their specified thresholds.

### Actual widget/event path

- CP.1: every icon-only button of our own chrome now sets both `.name` and
  `.tooltip` (eight that had only a tooltip were fixed); the name reaches
  AT-SPI only once the toolkit delivers the tree (see the accessibility note
  under the editor gate).
- [ ] Every icon-only control exposes accessible name, tooltip, enabled state,
  keyboard focus, pointer hover/press, and selected/mixed state where relevant.
- [ ] Tooltips show command and shortcut and do not obscure the target
  indefinitely.
- [ ] Action bar groups have visible but restrained separation. Overflow
  contains only implemented commands and dispatches the same message path.
- [ ] At full desktop width the Document, History, Clipboard, Style, Character,
  and implemented Paragraph groups are visible in the specified order. The
  `More` control is labeled, common commands are not overflow-only, and the
  reserved Print slot is absent until printing actually works.
- [ ] Style list scrolls with many items, applies with keyboard and pointer, and
  never marks one style selected for a mixed paragraph selection.
- [ ] On narrow layout, Styles opens as a non-modal overlay/drawer and closes
  without moving the caret or losing the editor selection.
- [ ] Tabs use the actual native selected/focus state; close/new controls remain
  reachable when horizontally scrolled.

### Physical COSMIC

- [ ] Icons for New/Open/Save, Undo/Redo, and Bold/Italic are recognizable
  without text labels; tooltips remove ambiguity.
- [ ] Button hit boxes are at least 32×32 logical px and do not crowd the
  document at 1280×800.
- [ ] Moving through differently styled paragraphs updates the one quick-apply
  field and Styles row without flicker. Mixed selection reads `Mixed` and does
  not select a false panel row.
- [ ] Format > Styles opens/focuses the same panel; the status bar never offers
  a competing style control.
- [ ] Open four documents including a long filename and imported SAM; active,
  dirty, and source state remain understandable, and Ctrl+W prompts safely.

Gate C fails if a nonfunctional icon appears, style choice has more than the
two specified synchronized surfaces, or selected state depends on color alone.

### UI-2 automated result (`96aeda4`, 2026-10-11)

**AUTOMATED PASS / PHYSICAL PENDING.** Coverage:
- action-bar composition, responsive contraction, icon and tooltip derivation;
- status-bar ownership (no style selector);
- window-title transitions;
- native tab-model mirroring through the real `update` path: order, labels,
  dirty mark, activation, dirty-close prompt, clean close, and stale-entity
  rejection;
- tiny-skia layout of the real application view. At 1280×800 and 1920×1080
  the action bar is 40 px, tabs 44 px, and status 28 px; the document row takes
  all remaining height; menu, action bar, and tabs share x = 0. New Document
  follows a single tab and stays inside the window for widths 900–1920 px with
  16 open tabs.

The layout test reproduced a defect introduced with the action bar. Its
`responsive` wrapper defaulted to fill height and occupied 322 px of an
800 px window. Corrected in `96aeda4`.

Recorded deviations:
- Per-tab tooltips are unavailable in the pinned libcosmic tab bar.
- The tab bar exposes no accessibility node. (Same toolkit block as the page surface; libcosmic's segmented button has no tooltip or node support. CP.1.)
- Status "Page 1" is not caret-derived.
- Styles panel, workspace, and zoom items remain UI-3/UI-4 scope.

### User visual observation (2026-10-11)

The user reports that the presented UI is closer to the approved mockup
(`UI_MOCKUP.svg`) than at the start of the project, but still not in line with
it. **The current appearance is not final.** Passing the UI-2 checklist below
accepts UI-2's functional behavior only; it is not visual acceptance against
the mockup. Before any gate is visually accepted, produce a side-by-side gap
list (application screenshots against the mockup, per screen region) and close
or explicitly accept each gap with the user.

### UI-2 focused physical COSMIC checklist

**Superseded (user decision, 2026-10-12):** this checklist is not run. The
UI pass on the page surface (`UI_GAP_LIST.md`) ends with one combined
physical check that covers its remaining items.

Run the default editor in release mode:
`cargo run --offline --release --manifest-path app/cosmic/Cargo.toml`.
Use 1280×800 or a similar window plus maximized, in both light and dark themes.
Record PASS/FAIL per item with symptoms. Screenshots of private documents stay
outside Git.

1. **Title:** the header reads `Untitled — AmiPro-Linux`, then
   `README31.SAM — AmiPro-Linux` after Open, and follows tab switches.
2. **Frame and rows:**
   - No margin frames the whole application.
   - Menu, action bar, and tabs start at the same left edge.
   - The action bar is one compact row with no large blank band under it.
   - The page area extends down to the status bar.
3. **Action bar groups:** New/Open/Save | Undo/Redo | Cut/Copy/Paste | style
   field | Bold/Italic | `More ▾`. Icons are recognizable. Hover tooltips read
   like `Save (Ctrl+S)`.
4. **Action bar state:**
   - Every icon works.
   - Undo is disabled on a fresh document.
   - Cut/Copy are disabled with no selection.
   - Bold/Italic show selected state on bold/italic text.
5. **Responsive:** narrowing the window moves Bold/Italic into `More` below
   about 1040 px, and Cut/Copy/Paste below about 860 px. `More` items work.
6. **Style field:** it follows the caret paragraph's style, reads `Mixed` for
   mixed selections, and applies a chosen style once (Undo reverts it).
7. **Native tabs:**
   - The active tab has native selected styling.
   - Each tab shows a document icon, filename, and close button.
   - Unsaved tabs show `•`.
   - Keyboard Tab focus reaches the tabs visibly.
8. **Tab overflow:** open at least four documents (include `README31.SAM` and a
   long `.amipro` name), then keep adding. Long names ellipsize. Scroll arrows
   appear. `+` stays visible after the tabs and creates a document.
9. **Tab close:**
   - × on a clean tab closes it.
   - × or Ctrl+W on a dirty tab offers Save/Discard/Cancel.
   - Closing the last tab leaves a new Untitled document.
10. **Switching:**
    - Ctrl+PageDown goes to the next tab and Ctrl+Shift+Tab to the previous.
    - Window menu checked entries match the active tab.
    - Report whether Ctrl+Tab now works (known pending).
11. **Status bar:**
    - One line showing `Page X of Y`, then the message, then
      `Imported source protected` for a SAM. Since Phase 2.5c, X is the
      caret's page and Y the laid-out page count; both follow typing,
      Enter, undo, and redo.
    - There is no style control and no permanent "Modified" text.
12. **Regression spot check in `README31.SAM`:**
    - Typing, drag selection, wheel scrolling, Undo/Redo, and clipboard work.
    - Save requires Save As to `.amipro` and leaves the SAM untouched.
    - Typing and Undo/Redo should feel immediate in the default build since
      `8b78ecf` (0.6 ms median headless, previously 52–86 ms). Report any
      sluggishness.

## 6. Gate D — workspace, ruler, status, and zoom

### Deterministic geometry

- [ ] Page, text area, margins, ruler zero/ticks, editor origin, hit testing,
  selection, caret, scroll extents, and zoom all use the same transform.
- [ ] Geometry cases cover Letter and A4 portrait, a landscape page, nondefault
  margins, 80/100/125/200% display scale, panel open/closed, and narrow windows.
- [ ] At minimum scale, horizontal scroll exposes both page edges and vertical
  scroll reaches the complete page/document extent.
- [ ] Zoom changes view scale but not semantic font sizes, document pagination
  inputs (unless the final display algorithm intentionally reflows by view
  width), or saved model values.

### Actual widget/event path

- [ ] Pointer hit tests and drag selection remain correct after panel toggle,
  resize, zoom, vertical/horizontal scrolling, and tab restore.
- [ ] Ruler stays aligned with page/text geometry during both-axis scrolling.
- [ ] Wheel/trackpad scrolling reaches the expected workspace; controls do not
  steal ordinary editor scrolling.
- [ ] Status fields neither overlap nor push zoom off-screen. Long transient
  messages ellipsize and expose full accessible text.
- [ ] View/zoom controls have visible focus and selected state and update the
  displayed editor, not only a percentage label.

### Physical COSMIC

- [ ] At 1280×800 and 1920×1080, the page remains the dominant region, chrome is
  compact, and no bottom/right content is clipped.
- [ ] Resize continuously from wide to narrow. Panel response, action overflow,
  tab overflow, page centering, and scrollbars transition without jumps or
  unreachable content.
- [ ] Paper proportions/margins are credible and ruler ticks remain aligned.
- [ ] The entire visible paper edge is distinguishable from the recessed
  workspace in both themes; whitespace reads as document margins rather than
  generic container padding.
- [ ] The status bar reads clearly, has consistent separators/spacing, and does
  not repeat filename, Modified prose, or style.

Gate D cannot pass shaped pagination or real page counts by appearance; those
remain future until backed by the layout engine.

## 7. Gate E — light/dark visual matrix

Capture all rows at both 100% and one high-DPI scale where practical.

| State | 1280×800 light | 1280×800 dark | 1920×1080 light | 1920×1080 dark |
|---|---|---|---|---|
| Empty new document, Styles open | [ ] | [ ] | [ ] | [ ] |
| Representative imported document, panel closed | [ ] | [ ] | [ ] | [ ] |
| Mixed selection + active formatting | [ ] | [ ] | [ ] | [ ] |
| Four tabs including dirty and long filename | [ ] | [ ] | [ ] | [ ] |
| One open menu showing shortcuts and disabled future items | [ ] | [ ] | [ ] | [ ] |
| Narrow/responsive state with overflow | [ ] | [ ] | [ ] | [ ] |
| Future context-rail illustration: Styles versus Frame Properties | design-only | design-only | design-only | design-only |

For every image verify:

- [ ] Interface type size/weight is consistent; document formatting is not
  mistaken for chrome typography.
- [ ] Active tab, focused control, toggled formatting, mixed state, disabled
  item, selection, caret, and page edge remain distinguishable.
- [ ] Ordinary text contrast is at least 4.5:1 and essential non-text state is
  at least 3:1, measured where colors are custom rather than assumed from the
  native theme.
- [ ] Page/workspace contrast and the 1–2 px page edge remain legible without a
  heavy card shadow.
- [ ] The representative editing state uses real heading/body text, mixed
  formatting, selection highlight, and caret rather than placeholder bars.
- [ ] No fixed light-only color makes dark theme unreadable.
- [ ] Long labels ellipsize intentionally; icons and baselines align; no row has
  accidental 12-pixel framing around the full application.

A reviewer compares these images with `UI_MOCKUP.svg` and records deviations.
Pixel identity is not required; hierarchy, ownership, dimensions, and state are.

## 8. Gate F — editor and document-integrity regression

Use nonempty multiparagraph content with UTF-8, mixed character sizes/styles,
underline, paragraph spacing, vertical and horizontal scrolling, and realistic
COSMIC Text shaping.

- [ ] Model/controller tests cover insertion/deletion/replacement, navigation,
  selection, clipboard, style/direct formatting, undo/redo, and tab isolation.
- [ ] The real production widget event path covers press, movement/drag,
  release, focus/capture, hit testing, selection geometry, coordinate
  transforms, keyboard navigation, clipboard, undo/redo, and IME.
- [ ] Safe Open/Save As/Save/reopen preserves the supported model and leaves
  imported SAM/STY source bytes unchanged.
- [ ] Accessibility exposes document text, caret/selection, editable state,
  command names, and status/error changes.
- [ ] IME preedit appears at the visible caret and commit creates one undo step.
  CP.1 (2026-11-07): AUTOMATED PASS, PHYSICAL PENDING. The surface now draws
  the composing text at the caret on the paper, underlined (thick under the
  input method's selection), `page_surface::draw_preedit`; a commit is one
  `Insert` and one undo step (`an_input_method_commit_is_one_undo_step`,
  `ime_commit_inserts_and_navigation_keys_stay_in_their_flow`). Physical:
  compose with a real IME (fcitx5/ibus Japanese or Chinese) at the caret in
  body text, a frame and a cell; the candidate window should stand beside the
  caret; commit, Undo.
  Accessibility is BLOCKED BY THE TOOLKIT, not implemented: in the pinned
  iced (libcosmic `dbc62f2`) `iced_winit` creates the accesskit adapter with
  only a root `Window` node and never calls `UserInterface::a11y_nodes` or
  `Adapter::update_if_active` (`a11y_enabled` in `iced/winit/src/lib.rs` is
  written and never read), so no widget's `a11y_nodes` reaches AT-SPI and
  Orca sees a window with no content; libcosmic's own text input has no
  `a11y_nodes` either. A node on the page surface would be dead code. Needs
  a toolkit that delivers the tree (a libcosmic update or a patched
  `iced_winit`), then a Document/multiline text node with a bounded value, the
  selection, and the editable state, on the page window only.

The rich surface stays feature-gated until this gate and its separate physical
activation checklist pass. Passing the shell design does not activate it.

## 9. Gate G — representative performance

The shell cannot be accepted if it makes already severe editing latency worse.
Measure a new empty document, approximately 18-page `README31.SAM`, and a
300+-paragraph case such as `AP31NAV.SAM` where available.

- [ ] Record hardware, COSMIC version, release/debug mode, document size,
  repetitions, p50/p95/p99 key-event-to-visible-update latency, and known
  measurement limitations.
- [ ] Exercise sustained insertion, deletion, selection replacement, Undo/Redo,
  drag selection, scrolling, and edits near start/middle/end.
- [ ] Break down mutation, projection, shaping, invalidation, widget update, and
  render where measurable.
- [ ] Verify no stale text, formatting, caret, selection, ruler, zoom, or tab
  state after actual edits and geometry changes.
- [ ] Obtain focused physical COSMIC confirmation that existing-paragraph
  typing is acceptable in both substantial imports.

Default editor (`8b78ecf`, 2026-10-11): **AUTOMATED PASS / PHYSICAL PENDING**.
The release end-to-end headless baseline measures `README31.SAM` typing going
from 52–55 ms to 0.58–0.62 ms median, and undo/redo from 79–86 ms to
0.43–0.67 ms. See `docs/research/DEFAULT_EDITOR_TYPING_PERFORMANCE.md`.

Rich editor current result: **PRIMARY DEFECT PHYSICALLY RESOLVED / REGRESSIONS PENDING**.
Cached selection/scrolling and mouse-wheel behavior had already improved; the
user now reports that typing in `README31.SAM` feels normal. This closes the P0
typing-responsiveness defect. Drag, scrolling, menu/toolbar history, caret, and
multi-tab regression items not exercised in that report remain pending and are
not inferred as passed. Do not reopen P0 unless a pending check fails.

## 10. Page surface checklist (Phase 2.6, `page-surface` feature)

Run it with
`cargo run --offline --manifest-path app/cosmic/Cargo.toml --features page-surface`.
In that build the page surface replaces the text editor. **Since 2026-10-12
it is the normal build** (user decision after the combined check; the text
editor stays available with `--no-default-features`), and plain `cargo run`
runs it. The surface draws the layout
engine's pages (`docs/research/LAYOUT_COMPARISON.md` describes how closely
they follow Ami Pro) and edits every flow through commands.

**Automated result (`2c9a38e`, 2026-10-12): AUTOMATED PASS / PHYSICAL PENDING.**
The frame steps wait until frames can be created in the app (user,
2026-10-17).

- Transform (`page_view.rs`): pages stack with 24-pixel gaps, centre when they
  fit and anchor to the 32-pixel padding when they do not; points round-trip at
  25–400%; Fit width never exceeds 100% or falls below 25%; only pages in the
  viewport are drawn.
- Widget path (`page_surface.rs`, the real widget with tiny-skia shaping):
  press, drag, and release select across lines; the caret is drawn where the
  click landed; ArrowDown crosses from page 1 to page 2; clicks enter a text
  frame, the header on page 2 (as the page-2 instance), and a table's second
  cell; a hit lands on the same text at 50%, 100%, 200%, and Fit width, scrolled
  or not; one edit reshapes at most four cached lines and every cached line
  matches the document after it; IME preedit is kept and a commit inserts
  once; Home, End, and Page Down move within the flow; Ctrl+Z goes to the
  command registry; the ruler's ticks and margins follow the transform and the
  horizontal scroll.
- Message path (`page_surface_app_tests.rs`): typing in frame text, in the
  header on page 2 (every page's header changes), and in the main text; Enter
  and Backspace join; Undo and Redo across flows with a valid caret; Select
  All, Bold, Cut, Paste, and a paragraph style in frame text; Esc back to the
  main text; Copy in the main text with the hidden editor sharing the
  selection; zoom commands, the slider, and the whole window laid out and
  drawn with the page surface.
- Performance (release, headless, `README31.SAM`, 15 pages, 1280×800, one
  message then view, layout, and draw): typing 0.42–0.48 ms median and
  0.44–0.49 ms p95 at the start, middle, and end; Backspace the same; Undo
  0.41–0.51 ms median. Target 1 ms median, 4 ms p95.

### User's short physical check (2026-10-12)

The user typed in `MERCURY.SAM` and a new document, scrolled, zoomed, and
clicked into frames and cells:

- **FAIL, fixed:** typing seemed slow because the caret did not move when
  the space bar was pressed, in `MERCURY.SAM` and in a blank document. Spaces
  that end a line had no caret position; they now do (test
  `the_caret_moves_past_trailing_spaces`). MERCURY typing measures 0.46,
  0.66, and 0.50 ms median at its start, middle, and end. **Recheck PASS**
  (`ce4e47c`, user: "typing seems good").
- **PASS:** scrolling and zoom ("work great").
- **PASS:** clicking into text frames and table cells. Picture frames take no
  click yet (frame selection is §11 of `UI_SPEC.md`, approved 2026-10-12).
- Not yet checked: the rest of Gates D, F, and G below.

### Gate D — workspace, ruler, status, and zoom (physical)

- [ ] Pages stack vertically with a gap, centred; the paper edge and its shadow
  are visible against the darker workspace in light and dark themes.
- [ ] The status-bar zoom controls (−, slider, percentage, +), Ctrl++, Ctrl+−,
  Ctrl+0, and View › Fit Page Width change the page size, and the percentage
  matches what is shown. Text stays sharp at 50%, 100%, and 200%.
- [ ] At 200% horizontal scrolling reaches both page edges. The ruler scrolls
  horizontally with the page, stays fixed vertically, and its ticks and text
  margins line up with the page.
- [ ] `Page X of Y` follows the caret, including in a header clicked on page N.
  The zoom controls are not clipped at 1280×800 or 1920×1080.

### Gate F — editing and document integrity (physical)

- [ ] A click places the caret in the main text on any page and in a header or
  footer (on the clicked page). Since frame interaction (below), a click in a
  frame selects it; a double-click (or Enter) puts the caret in a text frame,
  and a click on a selected table puts it in the cell under the pointer.
- [ ] Drag selects across lines and paragraphs; Shift+click extends.
- [ ] Typing, Backspace, Delete, Enter, Backspace at a paragraph start, and Tab
  work; UTF-8 text (é, ü, 東京) and an IME (preedit at the caret, one undo step
  per commit).
- [ ] Arrows cross lines, columns, and pages; Home, End, Ctrl+Home, Ctrl+End,
  Page Up, and Page Down work; Esc in frame, header, or cell text returns to
  the main text.
- [ ] Cut, Copy, Paste, Select All, Delete, Bold, Italic, and a style from the
  Styles panel work in frame text and in the main text; Undo and Redo restore
  each flow.
- [ ] Header text edited on page 2 changes on every page.
- [ ] `README31.SAM` and `AP31NAV.SAM` open with text, frames, headers,
  footers, pictures, and tables on the expected pages; Save As `.amipro` and
  reopen keep the edits; the SAM files are unchanged.
- [ ] (Superseded 2026-11-07: pictures and the Frame Properties inspector are built; see the Pictures and Frame sections and `UI_SPEC.md` §25.) Known gaps, to confirm rather than fail: pictures are labelled
  placeholders (no BMP or `.X3` decoder among the app's dependencies); the page
  surface has no accessibility nodes yet; with the caret in frame text the
  action bar's Bold, Italic, and style state shows the last main-text
  selection; there is no double-click word selection; the Frame Properties
  inspector is not built yet (below); paragraph alignment, indents, spacing,
  and tabs follow the import's current style mapping (Phase 3).

### Gate G — performance (physical)

- [ ] Typing at the start, middle, and end of `README31.SAM` and in
  `AP31NAV.SAM` feels immediate at 100% and 200%.
- [ ] Scrolling through every page is smooth; a zoom change is quick after the
  first layout at that zoom.

### Frame interaction (`UI_SPEC.md` §11, approved 2026-10-12)

**Automated result (2026-10-12): AUTOMATED PASS / PHYSICAL PASS
(2026-10-17).**

Physical (2026-10-17, the user's desktop, on frames made with the frame
creation run): "moving and resizing work, grouping frames, delete frames,
undo, bring to front and back", and earlier the same day changes in Frame
Properties (rounded corners, columns, the line between columns). Not
separately reported, and covered by the automated tests above: Esc during a drag, Tab between frames, Ungroup, a refused value in the rail, the outline in both themes.

- Model and controller (`frame_editing.rs`): Shift+click gathers Where
  placed frames on one page and drops a picked one; a group moves once even
  when several members are picked; With para above frames move only
  horizontally and Flow with text frames only vertically; Repeat right/left
  frames on left pages move and resize back to their stored rectangle;
  resizing keeps the interior margins plus 0.10 in and, with Shift on a
  corner, the aspect ratio; resizing an anchored frame from its free edge is
  a move and a resize in one undo step (`History::amend`); hits take the
  front-most frame and never a header or footer; Tab order is the stack.
- Widget path (`page_surface.rs`): at 50%, 100%, and 200%, scrolled and not,
  the first click selects, a drag from inside moves by the pointer's offset,
  and the bottom-right handle (hit within its 12-pixel target, with the
  diagonal resize cursor) resizes; 7 pixels past the corner is text again;
  Esc cancels a drag; arrows, Shift+arrows, Alt+arrows, Tab, Shift+Tab,
  Enter, Delete, and Esc become frame actions and typing does nothing;
  Ctrl+Z still reaches the registry; a double-click enters the frame's text
  at the pointer; a table's first click selects it and a click on the
  selected table enters the cell under the pointer.
- Message path (`page_surface_app_tests.rs`): a drag's move is one undo step
  and the frame stays selected through Undo and Redo; three arrow presses
  are one step, and a press more than a second later is another; Alt+Left
  stops at the minimum width; a handle's resize; Enter and Esc between the
  frame and its text; Delete (the key and Edit › Delete) asks first, Cancel
  keeps the frame, Delete removes it and Undo restores it; a click in the
  main text leaves the selection; Ctrl+G (Select Next Frame) and Tab cycle
  the page's frames; Group and Ungroup are enabled only when they apply;
  the group moves as one; Send to Back and Bring to Front restack.
- Inspector (`frame_inspector.rs`, message path): lengths with units and
  colours parse, and nonsense is refused; the fields show the frame; each
  typed value (position, size, page, columns with one tab list each, name,
  margins that must leave 0.10 in) and each choice (shadow depth and side,
  lines, placement with an anchor, Repeat all pages) is one command; With
  para above frames offer only the wraps they allow. Through the app:
  Format › Frame Properties opens the rail, which shows the inspector while a
  frame is selected and Styles after; a typed width applies on submit as one
  undo step and a second submit does nothing; a refused value keeps the old
  one with its reason; a choice applies at once and undoes in one step; a
  frame that changes placement stays selected where it is drawn; drafts
  never apply to another frame; the whole window lays out and draws with
  the inspector open.
- Performance (release, headless, `README31.SAM`): see `PHASE2_PROGRESS.md`
  (frame interaction entry).

Physical checklist (short; `cargo run --offline --manifest-path
app/cosmic/Cargo.toml --features page-surface`):

- [x] Click a frame in `MERCURY.SAM` or a new document's frame: the outline
  and handles show in light and dark themes; the pointer changes over the
  handles and inside the frame.
- [x] Drag it, drag a handle, and press Esc during a drag: the dashed outline
  follows smoothly; text reflows on release; Ctrl+Z undoes each gesture in
  one step.
- [x] Arrows and Alt+arrows move and resize; Tab moves between frames; Enter
  or a double-click types into the frame; Esc returns to the frame, then to
  the text; Delete asks before deleting.
- [x] Shift+click two frames, then Format › Group Frames; they move together;
  Format › Ungroup Frames, Bring to Front, and Send to Back.
- [x] With the Styles rail open (View › Styles Panel), selecting a frame shows
  Frame Properties and leaving it shows Styles again; type a width or a
  margin and press Enter or Tab away; try a bad value (it is refused with a
  reason); change the shadow, lines, and placement; Ctrl+Z undoes each
  change; the rail's fields are reachable with Tab and read sensibly with a
  screen reader if you use one.

### Creating frames (`UI_SPEC.md` §20, approved 2026-10-17, F1–F4)

**Automated result (2026-10-17): AUTOMATED PASS and VM PASS / PHYSICAL
PASS (2026-10-17)** (`docs/FRAME_CREATION_PROGRESS.md`).

Physical (2026-10-17, the user's desktop): creating passed. The user drew
frames, made them through Insert › Frame…, and changed them in Frame
Properties ("I'm able to draw frames, edit and manually create frames").
First use found two defects, both fixed and rechecked: the Restore Default
label was cut off in the rail (`6cb15fc`), and a frame's line between
columns was not drawn (`9e31005`). Then the rest: "moving and resizing
work, grouping frames, delete frames, undo, bring to front and back, make
default carries to the next frame". Not separately reported: Esc after
creating, and Restore Default (both covered by message-path tests).

- Rules (`amipro_editing::frame_creation`): Ami Pro's default frame and
  Make Default (`FrDefs` numbers), the dialog's starting values and
  limits, the remembered size, drawn rectangles (FC1–FC6N).
- Dialog (message path): starts from the last frame made or Ami Pro's
  first values, refuses a frame off the paper with the reason (FC5),
  creates on the page in view with the caret left where it was (FC9),
  remembers the frame, one undo step, Draw on Page starts drawing.
- Drawing (window path): FC2's drag from bottom right to top left to
  within a pixel at 100 %, a click or Esc ends drawing (FC4, FC11), a drag
  past the paper's edge stops there at 50 % scrolled to page 2; the new
  frame is selected and drawn above the page's other frames.
- Make Default and Restore Ami Pro's Default (message path; settings in a
  scratch directory): FC6/FC6N.
- Drawing: Where placed frames above anchored ones (FC8); an opaque frame
  hides the text behind it (pixels); rounded corners drawn.
- Twins: six fixtures' frames made again in the app write Ami Pro's
  records, and Ami Pro shows the six as its own (VM, pixel for pixel).
- Timings (release): creating a frame 3.8 ms to the drawn frame on
  README31, 8.4 ms on a 30-page document; typing unchanged within noise.

Physical checklist (creating, then moving and resizing, the list above):

- [x] Insert › Frame…: the dialog shows 2.00, 2.00, 4.50, 3.25 the first
  time; Create: a frame with a thin line, a shadow, and rounded corners,
  selected, on the page you are looking at. Open it again: it offers the
  frame you just made. Type 9 in Width and press Create: it says why not.
- [x] Draw Frame (the square at the toolbar's right end, or Insert › Draw
  Frame): the pointer is a crosshair over the pages; drag a frame, in any
  direction, also over another frame (the new one covers it); a click
  makes nothing; Esc cancels.
- [x] Ctrl+Z removes a new frame in one step; Esc after creating returns to
  the caret where it was; Enter or a double-click types in the frame.
- [x] Frame Properties › New frames › Make Default on a frame you changed
  (say Transparent, no shadow), then make another frame: it takes those
  settings but not the placement or columns; Restore Ami Pro's Default.
- [x] Then the frame interaction list above (moving, resizing, keys,
  grouping, the rail) on frames you made.

### Bullets and numbering (`UI_SPEC.md` §21, approved 2026-10-17, B1–B5)

**Automated result (2026-10-17): AUTOMATED PASS and VM PASS / PHYSICAL
PASS** (`docs/BULLETS_PROGRESS.md`; the user, 2026-10-18: "All list items
checked").

- Rules (`amipro_editing::lists`): which list a style shows, the
  document's list style by Ami Pro's names, a made style as
  `_DEFAULT.STY`'s in Body Text's font with the next free function key,
  the toggle (off only when every paragraph shows the kind), Enter ending
  a list; written as Ami Pro writes a style it creates (BL4).
- Commands (message path): a made Bullet 1, check marks and the toolbar's
  state, the status bar, a mixed selection, switching kinds, off, undo
  step by step, Enter, the shortcuts, nothing with a frame selected.
- Insert › Bullet… (message path; settings in a scratch directory): at the
  caret and over a selection, one undo step each, remembered, Cancel.
- Bullet and Number Style…: the caret's style at Bullets and numbers;
  Space for is First.
- Drawing: inserted bullets in their text's size and colour, the caret and
  clicks around them; Right align (BL2), superscript (BL3), the seven
  pictures as the app's own shapes; screenshots light and dark.
- Twins: BL1, BL2, BL4 made again in the app write Ami Pro's files, and
  Ami Pro shows them as its own (VM, pixel for pixel).
- Formats: lists, made styles, and inserted bullets read back equal from
  `.amipro` and SAM, nothing in the loss check.

Physical checklist:

- [ ] Select a few paragraphs and press the Bullets button (or
  Ctrl+Shift+L): small diamonds, the text indented, the button stays
  pressed and Format › Bullets and Numbering ▸ Bullets has a check (the
  submenu since the outline levels run). The Styles list now has
  "• Bullet 1" if the document's style sheet had none.
- [ ] Press Numbering (Ctrl+Shift+N) on the same paragraphs: 1., 2., 3.,
  lined up on the right; press it again: back to Body Text. Ctrl+Z
  steps back one change at a time.
- [ ] At the end of a numbered paragraph press Enter: the next number;
  Enter again on that empty line: the list ends there.
- [ ] Insert › Bullet…: pick a bullet (try the checked box), Insert: it
  goes in at the caret in the text's size; open it again: it offers the
  same one. Arrows and Backspace treat it as one character.
- [ ] Right-click in a list paragraph › Bullet and Number Style…: the
  side panel opens at Bullets and numbers; change the number to "a" and
  the list follows.
- [ ] The toolbar's two list icons look right in light and dark.

### Outline levels (`UI_SPEC.md` §22, approved 2026-10-17, O1–O4)

**Automated result (2026-10-18): AUTOMATED PASS and VM PASS / PHYSICAL
PASS** (`docs/OUTLINE_PROGRESS.md`). Physical, 2026-10-18 (user): a
numbered list promoted and demoted with the toolbar's buttons
"successfully", then "All list items checked".

- Counting (`amipro_layout`): After higher level, After intervening
  style, and no restart; a level counts every paragraph at it whatever
  the style; cumulative 1, 1.1, 1.2, 2, 2.1 (OL2); BN2 and every compared
  page unchanged.
- Rules (`amipro_editing::outline`): Outline3 made from Subhead and None
  promoted and demoted (OL1); a numbered list nested as 1. › A. › i. as
  List 1, List 2 below the document's levels, back again, one undo step;
  written as Ami Pro writes levels (SAM `[tag]`s) and read back equal.
- Commands (message path and widget): Promote and Demote by menu,
  toolbar, and Alt+arrows, the numbers drawn, the top not promoting, a
  selected frame resizing on Alt+arrows instead; the folded Format menu's
  order, check marks, and right-click menu.
- Style Properties: the level, the exclusive reset pair, Cumulative
  building and removing the earlier levels' numbers.
- Outline Numbering…: each scheme by level, Text and bullets kept,
  styles without a level untouched, one undo step a scheme, Apply refused
  without levelled styles; screenshots light and dark.
- Twins: OL1 and OL2 made in the app write Ami Pro's bodies and `[tag]`s;
  Ami Pro shows the app's AOL1–AOL3 as its own, a nested list included
  (VM).
- Formats: levels, nested lists, and cumulative numbers read back from
  `.amipro` and SAM, nothing in the loss check.

Physical checklist (only what the headless harness cannot show):

- [ ] Number four paragraphs (Ctrl+Shift+N), put the caret in the second
  and press Alt+Right: it becomes "A." under "1."; Alt+Right again: "i.";
  Alt+Left twice brings it back. Does it feel right, and are the
  indents comfortable?
- [ ] Open Format: Alignment ▸, Bullets and Numbering ▸, and Frame ▸ open
  as submenus, and the menu is short enough (the menu bar cannot be
  opened headless).
- [ ] The toolbar's Promote and Demote icons look right in light and
  dark; in a window narrower than about 970 px the list buttons and Draw
  Frame move to More together.
- [ ] Format › Styles ▸ Outline Numbering… on a document with headings
  (for example a new document with Title and Subhead paragraphs): pick "1
  1.1 1.1.1" and Apply; Ctrl+Z undoes it in one step.

### Style Properties and right-click menus (2026-10-13, `UI_SPEC.md` §15–16)

Automated: the panel's choices apply at once and typed values on submit,
a bad value is refused with its reason, the panel follows the caret and a
picked style stays until the caret moves to another style, Undo shows in
the panel, Save as New Style shows the new style, ‹ Styles and × behave
(`style_properties.rs`, `page_surface_app_tests.rs`); the right press
selects a frame, moves the caret, or keeps the selection (`page_surface.rs`);
both menus' entries and enabled states, and the style list applies a style
(message path); screenshots of the panel's sections and both menus in
light and dark (`window_screenshots`). Since the user's first check
(2026-10-13): clicks on the menu's rows through the whole window on a
scrolled page (Copy, Cut, Paste, and Modify Style with the rail closed),
a Styles panel row's menu in a scrolled list, and the pages keep their
scroll when the rail opens, closes, docks, or floats
(`page_surface_app_tests.rs`, `Runtime`).

Physical checklist (normal build):

- [x] Format › Styles ▸ Modify Style…: the rail widens and shows the caret
  paragraph's style; change the size (Enter), Bold, an indent (Tab away),
  and the spacing; the page changes at once; Ctrl+Z undoes each change.
  PASS (user, 2026-10-13: size, Bold, indent, Ctrl+Z).
- [x] Click into a paragraph in another style: the panel follows. PASS
  (user, 2026-10-13). Picking a style from its list that then stays while
  you click within the same style was not asked physically (automated).
- [ ] ‹ Styles returns to the list; × closes the rail. Not asked
  physically; covered by automated tests, no further check needed.
- [x] Right-click text: Modify Style "…" opens the panel; Paragraph Style ▸
  applies a style; Cut and Copy work on a selection right-clicked inside it.
  First check (user, 2026-10-13): FAIL with the rail closed: Modify Style,
  Cut, Copy, and Paste did nothing. Fixed (`b05da29`); recheck on a
  scrolled page with the Styles panel off (Copy, Cut, Paste, then Modify
  Style, the page staying put): PASS (user, 2026-10-13, "That all
  works").
- [x] Right-click a frame: it is selected and the frame menu shows; Delete
  asks first. PASS (user, 2026-10-13).
- [x] Format › Styles ▸ Create Style…, Style Management…, Use Another Style
  Sheet… (Browse… once), and Save as a Style Sheet… (a new name). PASS
  (user, 2026-10-13: "all worked").

### Choosing colours (`UI_SPEC.md` §23, approved 2026-10-18, C1–C4)

**Automated result (2026-10-18): AUTOMATED PASS and VM PASS / PHYSICAL
PASS** (`docs/COLOUR_PROGRESS.md`; the user, 2026-10-18: "colour buttons
all work").

- Model (`colours.rs`): conversions and names; the document's colours,
  most used first, hidden ones left out; Recent's order, repeats, and
  limits; Recent kept in the settings across a restart.
- Messages: the panel's rows; a swatch one undo step; a picker drag of
  three steps one undo step, undone at once; a typed `#336699` on Enter;
  the rule colour; the panel closing with Style Properties, the Font
  dialog, and Frame Properties; the Font dialog applying with OK; all four
  frame colours, read back from SAM and native with nothing about the
  frame in the loss check; screenshots light and dark.
- Twins: ACS1 made in the app (text, a rule, a frame's four colours, the
  page's lines in mixed colours); Ami Pro draws each colour exactly (VM).

Physical checklist (only what the headless harness cannot show):

- [ ] Style Properties › Font › Colour: the button opens the panel; a
  swatch colours the text at once and closes it; Ctrl+Z undoes it.
- [ ] More colours: drag in the colour field and along the hue slider;
  the text follows as you drag, and one Ctrl+Z undoes the whole drag.
  Type a colour in the box (`#336699`) and press Enter.
- [ ] The colours you picked appear under Recent, and the document's
  colours under In this document, in another panel too (a frame's
  Background) and after restarting the app.
- [ ] Frame Properties: line, shadow, background, and (with two columns
  and Line between columns) the line between columns each have a colour
  button.
- [ ] The panel in light and dark, and in the narrower Frame Properties
  rail.

### Frames in table cells (`UI_SPEC.md` §24, approved 2026-10-18, T1–T3)

**Result (2026-10-18): AUTOMATED PASS and VM PASS / PHYSICAL PASS**
(`docs/CELL_FRAMES_PROGRESS.md`; the user: "that all works").

- Model and files: Ami Pro's table grid (insets, own gutters); a picture
  held by exactly one cell; Ami Pro's list order and cell numbers written
  and read; never-sized pictures written at their cell's size (T2);
  deleting a picture or its table, one undo step each; native v3.
- Layout: the picture over its cell's text area, right above its table,
  following the table to other pages and the row as it grows; incremental
  layout equal to a full layout.
- Widget and messages: a click on the picture puts the caret in its cell
  (no frame selection); typing, Enter, and Backspace there change nothing;
  Delete and Edit › Delete remove the picture, one undo step; Select Next
  Frame passes it by; the status note; screenshots light and dark
  (`window-cellpicture-*`) and README31's page 12.
- Twins: ATF2, ATF3, ATF7, ATF10 and README31 with a page break, made in
  the app; Ami Pro opens each with the pictures in their cells (VM).

Physical checklist (only what the headless harness cannot show):

- [x] Open README31.SAM (`Reference/SAM`), go to page 12 (Ctrl+G): the
  SmartIcon table's three pictures sit in their cells (grey "Picture"
  boxes until pictures are drawn).
- [x] Click one: the caret goes into the cell, which is outlined in the
  accent colour; no handles; the status bar says "This cell holds a
  picture; Delete removes it". Typing and Backspace do nothing.
- [x] Press Delete: the picture goes and the cell is empty; Ctrl+Z brings
  it back.
- [x] Click at the end of the line just above the table and press
  Ctrl+Enter (Insert › Page Break): the table and its pictures move to
  page 13 together; Ctrl+Z.

### Pictures (`UI_SPEC.md` §25, approved 2026-10-18, P1–P8)

**Result (2026-10-18): AUTOMATED PASS, VM PASS, and PHYSICAL PASS**
(`docs/PICTURES_PROGRESS.md`). The user's first check: Insert › Image…
and picture mode PASS; paste and drop did nothing (FAIL, fixed: see
`UI_SPEC.md` §25's notes); Percentage was not understood (a note under
Sizing and the percentage always shown since). The recheck: Copy Image
pasted and a dropped file went in (PASS); a file copied in Files pasted
through Edit › Paste, but the right-click menu's Paste was greyed out
(fixed). The second recheck: right-click Paste, Percentage 50, README31
page 12, a logo at 200 %, and Save As naming the converted picture all
PASS.

- Model and files: scaling, pan, rotation, and a description in the model
  and native v3; Ami Pro's `[isd]` written back as read while current, or
  rebuilt with the driver's state kept; the three folders written byte for
  byte as before the run; Save As SAM converts other formats to bitmaps
  (300 dpi at their size) and copies Linux links in, listing both.
- Drawing: every stored bitmap and metafile in both corpora reads;
  drawn in their frames, clipped, under their lines, metafiles at the
  shown size; Ami Pro's screen image for drawings, charts, and OLE
  objects, else a labelled box; screenshots against Ami Pro's (PI1, PI2,
  PI5, PI7, PI8, PIRT30, PIRT90) and the light and dark rail.
- Widget and messages: Insert › Image… into a new frame, an empty frame,
  an empty cell, or replacing; Frame Properties › Picture; picture mode
  (drag, arrows, Esc; Shift and a corner handle); paste; a drop moving the
  caret.
- Twins: twelve made in the app open in Ami Pro with their pictures where
  the app put them; Ami Pro's re-saves keep scaling and pan.
- Timings (release, to the drawn frame): a page's pictures decoded on a
  warm page 0.5–0.7 ms; the heaviest corpus metafile 7.2 ms once per size,
  0.55 ms redrawn; typing beside a picture 0.87 ms; a picture mode step
  0.91 ms.

Physical checklist (only what the headless harness cannot show; about
eight minutes):

- [x] Open README31.SAM (`Reference/SAM`), Ctrl+G to page 12: the three
  SmartIcon pictures are drawn in their table's cells.
- [x] Open one of your documents with a letterhead or logo: the picture
  is drawn; zoom to 200 %: a metafile stays sharp.
- [x] In a new document type a line, then Insert › Image… (Ctrl+Shift+I)
  and choose a photo or PNG: it goes in at the caret in a frame its own
  size, selected; Ctrl+Z removes it. Again with **Link to file** checked:
  the status bar says it is linked.
- [x] With the picture selected, Frame Properties shows Picture first:
  choose Original size; type 50 in **Percentage of its size** (the
  picture becomes half its size, the frame stays); type a Width (Sizing
  becomes Custom); Rotate 90; then Reset and Fit Frame to Picture;
  Ctrl+Z after each. The line under Sizing says what each choice does.
- [x] Double-click the picture: the handles turn grey and the pointer is a
  hand; drag the picture inside its frame, press the arrows (Ctrl for
  larger steps), then Esc.
- [x] Copy an image in another program (a browser's Copy Image, or a
  picture file in Files) and press Ctrl+V in the text: it pastes as a
  picture. (FAIL 2026-10-18, fixed; Copy Image PASS on the recheck; a
  file copied in Files: right-click Paste greyed out, fixed; PASS.)
- [x] Drag a picture file from Files, or an image from a browser, onto a
  line of text: it goes in there. (FAIL 2026-10-18, fixed; PASS on the
  recheck with `AMIPRO_TRACE_DROP`.)
- [x] File › Save As, Ami Pro document: the list names the converted
  picture (and the linked one); Save Anyway.

### Tables across pages (`UI_SPEC.md` §26, TP1–TP5 taken overnight 2026-10-18, confirmed 2026-10-19)

**Result (2026-10-19): AUTOMATED PASS, VM PASS, and PHYSICAL PASS**
(`docs/TABLE_PAGES_PROGRESS.md`; the user's physical check passed
2026-10-19).

- Layout: 14 rule tests from the VM fixtures TP1–TP11 (whole rows, Rows
  span pages, rows cut to a page, headings repeated with one more inset,
  page breaks on rows, columns, the first row's fit, Automatic, frame
  tables, pictures in cells, incremental equal to fresh); the corpora and
  the VM folder measured (`LAYOUT_COMPARISON.md`, "Tables across pages").
- Widget and messages: clicks in a continued table and in its repeated
  heading; Insert › Page Break in a cell sets and clears the row's break,
  one undo step, checked there; both formats read back.
- Twins: ATP2, ATP5, ATP6 page in Ami Pro as its own fixtures.
- Timings (release, to the drawn frame): typing in a continued table
  1.1–1.3 ms; a page break in a row 1.0–2.8 ms, its undo the same.

Physical checklist (about five minutes; the files are the run's own
synthetic fixtures in `../AmiPro-Reference/vm-lab/OUT`):

- [x] Open `TP2BS.SAM`: the table runs over pages 1–4; page 2 starts with
  the heading rows R0 and R1, pages 3 and 4 with R40 (the last heading
  before each page's first row; rows 0–10, 11–42, 43–75, 76 on, as in
  Ami Pro's page table).
- [x] Click a cell on page 3 and type: the letters go into that cell and
  the table moves on as needed; Ctrl+Z.
- [x] Click R0C0 at the top of page 2 and type a letter: it shows in the
  heading on every page; Ctrl+Z.
- [x] Click a cell in row 20 and choose Insert › Page Break (Ctrl+Enter):
  row 20 starts the next page, and the Insert menu shows Page Break
  checked there; Ctrl+Enter again takes the break away.
- [x] Open `TP5.SAM`: row 6's tall cell starts at the bottom of page 1 and
  goes on at the top of page 2, with no grid line where it splits.
- [x] The same in the dark theme.

### Find & Replace (`UI_SPEC.md` §27, approved 2026-10-19, F1–F4)

**Result (2026-10-20): AUTOMATED PASS, VM PASS, and PHYSICAL PASS**
(`docs/FIND_PROGRESS.md`; the user's check 2026-10-20). The user's visual
notes (the fields did not line up; the bar seemed to belong above the
ruler) were acted on the same day: the bar is now a toolbar-style row
between the tabs and the ruler, its fields on one grid. The user looked at
the bar in its new place the same day: passed.

- The search: 17 rule tests from VM fixtures FR1–FR7 and the user's guide,
  ch. 20 (the capitals rule, Whole word only, Exact case, wildcards and
  their captures, tabs and returns, the streams in Ami Pro's order, a
  selection, paragraph styles, Exact attributes, Replace All as one step).
- Twins: Ami Pro's own input files through the app's Replace All give the
  text Ami Pro saved in every stream (FR1A, FR1B, FR2, FR3, FR5A, FR6).
- Sweep: every paragraph of both corpora is found where it is, in its
  stream (31,061 and 33,438).
- Widget and messages: the bar's keys (typing goes to the field, not the
  page; Enter, Shift+Enter, Esc), Find Next and Previous round the ends,
  Replace and Replace All one undo step each, the options and recent
  searches kept, the pages' scroll kept when the bar opens and closes;
  screenshots in both themes (`find_bar_screenshots`).

Physical checklist (about five minutes):

- [x] Open `README31.SAM` and press Ctrl+F: the bar shows under the ruler
  with the Find field ready; type `the`: the matches turn yellow on the
  page and the bar says how many.
- [x] Press Enter a few times, then Shift+Enter: the selection moves from
  match to match and the page follows; the bar and the status bar say
  "n of N".
- [x] Options ▾: switch Whole Word Only off and Match Case on: the count
  changes at once; check that the menu shows the check marks.
- [x] Ctrl+H, type a replacement, press Replace twice, then Replace All:
  the status bar gives the count; one Ctrl+Z takes back the Replace All,
  and each earlier Replace takes one more.
- [x] Recent ▾ lists `the`; choose it.
- [x] Esc with the cursor in the Find field closes the bar and leaves the
  last match selected; the page keeps its place.
- [x] The same in the dark theme.

### Fields (`UI_SPEC.md` §28, approved 2026-10-20, FL1–FL7)

**Result (2026-10-20): AUTOMATED PASS, VM PASS, PHYSICAL PASS** (the user, 2026-10-20: "Powerfields work")
(`docs/FIELDS_PROGRESS.md`).

- Rules: the 21 date formats and the time (FD1, FD4, FD5), Ami Pro's time
  base (FD2, FD5), the Doc Info entries (FD2), the counts (FD2), the
  power-field language (FD3, FD6, FD7; 45 of Ami Pro's results), Seq
  (FD6), updating, the panels' insertions and changes.
- Twin AFD1: Ami Pro shows the app's file as the app does; its Doc Info,
  Update, and Update All agree with the app.
- Messages: the Fields panel inserts and changes fields, one undo step
  each; Update All; Next Field; Lock Field; Show Power Fields; Document
  Properties' edits; screenshots of both panels in both themes.

Physical checklist (about five minutes):

- [ ] Open a letter of yours with a date at the top (one with `<:D…>`):
  the date shows, shaded light grey; View › Field Shading turns the grey
  off and on.
- [ ] Insert › Date/Time…: the Fields panel opens; choose System date and
  a format, Insert; the date appears at the caret. Ctrl+Z takes it back.
- [ ] Insert › Field…: type `NumPages`, Insert; then `Seq Figure` twice:
  1 and 2. Put the caret in a field: the status bar names it, This field
  shows it; tick Lock and press F9: it stays.
- [ ] Edit › Power Fields ▸ holds Update Fields, Update All Fields, Next
  Field, Previous Field, Edit Field…, and Lock Field.
- [ ] View › Show Power Fields (Alt+F9): the codes in red in the text.
  Change `NumPages` to `2+5` by typing, press F9 (the code stays), then
  Alt+F9 again: the field shows 7. Ctrl+Z steps back through each.
- [ ] File › Document Properties… (Alt+Enter): type a description, press
  Enter; the statistics count as you type elsewhere; Ctrl+Z takes the
  description back.
- [ ] Save As SAM and open the file in Ami Pro if you like: the fields
  and Doc Info as the app showed them.
- [ ] The same in the dark theme.

### Print and PDF (`UI_SPEC.md` §29, approved 2026-10-20, PO1–PO6)

**Result (2026-10-20): AUTOMATED PASS, VM PASS (the options against Ami
Pro's printed fixtures), PHYSICAL PASS** (the user, 2026-10-20:
"printing works correctly", "pdf works correctly", then: the Print panel
"looks good, except for the arrangement of the buttons at the bottom of
the screen", "I didn't have any issues with print current page"). The
panel's three buttons were pills of three widths, left-aligned; they now
span the rail, stacked under a divider (the user's choice, 2026-10-20;
`the_print_panels_buttons_take_the_rails_width`; `docs/PRINT_PROGRESS.md`).

- Rules: crop marks' geometry (VM PFC1C), the cover sheet's lines (PFC1D),
  preprinted form and Without pictures (PFC1F, PPT1F, PACLF, PACLP), Update
  fields first, page ranges read from the dialog, the paper named for it.
- The PDF: every page at its layout's size, fonts embedded with their
  text, vector pictures (230 of 250 metafiles in the user's corpus), the
  page surface's drawing reproduced (`pdf_against_screen`), bookmarks,
  page labels, PDF/A-2b.
- Messages: Export PDF…, Print…, Print Current Page…, the Print panel's
  options and additions; screenshots of the panel in both themes.

Physical checklist (about five minutes):

- [x] File › Export PDF… (Ctrl+Shift+E) on one of your documents with a
  picture: the save dialog offers its name with `.pdf` beside it; open the
  PDF in your viewer: the pages look as in the app, the text can be
  selected and searched, the picture is sharp when zoomed, and the
  sidebar lists its headings if it has outline levels.
- [x] File › Print… (Ctrl+P): COSMIC's print dialog opens on your paper;
  print pages 2–3 (or choose Print to File): only those pages come out.
- [x] File › Print Current Page…: the dialog opens with the caret's page as
  its range.
- [x] File › Print Options…: the Print panel in the rail; the margins'
  shading goes while it is open. Tick On preprinted form: frame lines and
  pictures disappear from the pages; untick it. Tick Crop marks and
  Export PDF: the page sits in a half-inch border with marks at its
  corners.
- [ ] Close the panel: the margins are shaded again. The same in the dark
  theme.

### Export (`UI_SPEC.md` §30, approved 2026-10-20, EX1–EX10)

**Result (2026-10-20): AUTOMATED PASS, VM PASS (Ami Pro 3.1 opens the
app's RTF: twins ATL1B, AF01), LibreOffice comparison PASS for the
recorded differences, PHYSICAL PASS (2026-10-24: "Export works fine")** (`docs/EXPORT_PROGRESS.md`).

- Rules: the exchange document (styles' names and order, runs over their
  styles, caps as shown, fields and their results, live lists showing Ami
  Pro's numbers), zips and XML well-formed (1,714 corpus packages), table
  lines and fills where Ami Pro's printouts put them (VM TL1, TL4).
- Messages: Export ▸'s four formats written as copies with Open; the loss
  dialog both ways; Export Folder… with its report and progress;
  screenshots of the panel and the loss dialog in both themes.
- LibreOffice 24.2 against the app's PDF on 70 sampled documents: the
  same page count for 36 of 40 user and 21 of 30 ISO documents.

Physical checklist (about five minutes, in LibreOffice Writer, EX4):

- [x] File › Export ▸ Export Word Document (.docx)… on one of your
  documents with frames and a table: the save dialog offers its name with
  `.docx`; click **Open** in the status bar: LibreOffice shows it with its
  frames where they were, the table with its lines, its pictures, and its
  headings in the Navigator.
- [x] The same document as OpenDocument Text and as Rich Text: both open
  in LibreOffice looking the same.
- [x] A document with protected text or notes: the export first lists what
  Word Document cannot hold; Cancel writes nothing, Export Anyway writes
  it.
- [x] File › Export ▸ Export Folder…: choose a small folder of Ami Pro
  documents and an empty folder; Export: the panel counts them, then
  Open the report lists each file; the documents are in the empty folder
  in the same subfolders, and nothing in the first folder changed.
- [x] Tables with lines (Lines & Color in Ami Pro) show their lines in the
  app, in the Print panel's pages, and in a printed page or PDF.

### Import (`UI_SPEC.md` §31, approved 2026-10-21, IM1–IM12)

**Result (2026-10-24): AUTOMATED PASS, VM PASS (Ami Pro 3.1 opens the
app's SAM of imported documents as the app shows them: twins AIM1, AIMD,
AIML, AIMN, and AIMO's table; differences recorded for text beside two
frames and a picture placed inside a cell's frame), PHYSICAL PASS (2026-10-25)**
(`docs/IMPORT_PROGRESS.md`).

- Rules: each reader against Ami Pro's own fixtures (IM1–IM3, NT1) and
  the formats' specifications; the app's DOCX, ODT, and RTF of both
  corpora and the VM folder read back equal in what each format holds;
  4,099 of 4,142 sample files imported, 43 refused with their reason, none
  panicked.
- Messages and widgets: Open through Import Options (both builds), Insert
  › File… in the main text and a frame's text as one undo step, Paste of
  RTF, a dropped document, the Import Report (Go to, Copy, the status
  bar's count), Import Folder…; the dialog's radio and Open and the
  panels' buttons clicked through the window; screenshots in both themes.
- LibreOffice 24.2 against the app on 89 samples: the same page count for
  82.

Physical checklist (about five minutes):

- [ ] File › Open on a Word document of your own (or a LibreOffice one):
  the Open dialog lists All documents first; Import Options asks how to
  take the styles, the last choice preselected; Open: the document opens
  in a new tab, and Save goes to Save As.
- [ ] The Import Report opens in the rail when something is different:
  Go to shows each place; Copy; × closes it, and "N import notes" in the
  status bar opens it again.
- [ ] Insert › File… with the caret in the middle of a document: the
  file's text goes in there; Undo takes it out in one step.
- [ ] Copy a few formatted paragraphs (with a table) in LibreOffice
  Writer and Paste here: the formatting and table come with them.
- [ ] Drop a .docx or .odt file from the file manager onto the page: it
  opens through Import Options.
- [ ] File › Import Folder… on a small folder of Word or LibreOffice
  documents into an empty folder: the panel counts them, Open the report
  lists each; the `.amipro` documents open in the app.

### Table editing (`UI_SPEC.md` §32, approved 2026-10-24, TD1–TD11)

**Result (2026-10-24): AUTOMATED PASS, VM PASS (twins ATE1, ATE2, ATE3,
ATE5, ATE8, ATE10, ATE11, ATEL, ATES, ATEM, ATEC, ATET open in Ami Pro 3.1
as the app made them; the cells and headers of ATE2–ATE11 equal what Ami
Pro wrote for the same changes), PHYSICAL PASS (2026-10-25)** after one
fix from first use (a right-click in a cell of a table selected as a frame
showed only Frame Properties, `a46224d`)
(`docs/TABLE_EDIT_PROGRESS.md`).

- Rules: every command against Ami Pro's records for the same change
  (VM TE1–TE20, `TABLE_EDIT_NOTES.md`), one undo step each.
- Messages and widgets: the Table menu's commands and checks, the
  right-click items, Insert Table's dialog clicked through the window,
  Table Properties' buttons, cell selections by drag and strips, Tab,
  Ctrl+arrows, grid lines dragged, the ruler's table columns, Copy, Cut,
  Paste, and drag and drop of cells; screenshots in both themes.

Physical checklist (about five minutes):

- [ ] Insert › Table… (Ctrl+F12): sweep the grid to 4 × 3, Insert; the
  table stands below the line, the caret in its first cell. Type, Tab
  through the cells; Tab in the last cell adds a row.
- [ ] Drag across a few cells: they show selected; Table › Connect Cells,
  then the menu says Disconnect Cells; Undo.
- [ ] Drag a column's grid line (and a column marker on the ruler): only
  that column changes.
- [ ] Table › Table Properties…: change a width, turn on lines (All), a
  fill colour and a line colour, a leader, Center table on page.
- [ ] Copy two cells, click in another cell, Paste; drag a selected row
  onto another row.
- [ ] Right-click in the table: the row and column commands come first.

### Drawings and Draw mode (`UI_SPEC.md` §33, approved 2026-10-25, DW1–DW9)

**Result (2026-10-25): AUTOMATED PASS, VM PASS (twins ADR1–ADR3 open in
Ami Pro 3.1 as the app shows them; Ami Pro edits ADR3 in its own Draw mode
and saves the app's stream back unchanged), PHYSICAL PASS (2026-10-26)**
(`docs/DRAWING_PROGRESS.md`).

- Rules: the stream read and written for every corpus drawing; painting
  against Ami Pro's own screen images (median 89 % ink agreement within
  two pixels) and fixtures DRW1–DRW3; Ami Pro's constructions (DR2B).
- Messages and widgets: drawing, selecting, Delete, Undo, Esc, Enter, and
  a click outside through the window; Drawing Properties, the Draw menu,
  the clipboard, Import Drawing and Save Drawing As through messages;
  screenshots in both themes.

Physical checklist (about five minutes):

- [ ] Open a document of yours with drawings: they show as in Ami Pro.
- [ ] Insert › Drawing…: the draw bar replaces the toolbar and Drawing
  Properties opens. Draw a rectangle, an ellipse (with Shift: a circle), a
  line with an arrow (Drawing Properties › Line › End), a polygon (clicks,
  double-click), and a text object (type, Enter for another).
- [ ] Click an object, drag it, drag a handle; Shift+click a second, Draw ›
  Align ▸ Top; Group; Undo.
- [ ] Double-click an object: drag a point, a corner arrow to turn it.
- [ ] Copy an object and paste it into another program (an image); paste
  an image or SVG from a browser into the drawing.
- [ ] Esc or Done; save as SAM and reopen.

### Notes, bookmarks, footnotes, endnotes (`UI_SPEC.md` §34, approved 2026-10-26, NR1–NR10)

**Result (2026-10-26): AUTOMATED PASS, VM PASS (twins ANT1, ABK1,
AFN1–AFN3 open in Ami Pro 3.1 as the app made them; AFN1 is Ami Pro's own
RNF5), PHYSICAL PASS (2026-10-26, the user: "all items pass")** (`docs/NOTES_PROGRESS.md`).

- Rules: notes, footnotes, table footnotes, bookmarks, and Footnote
  Options read and written as Ami Pro writes them; footnote layout against
  RNF1–RNF7*; exports read back through every format.
- Messages and widgets: a click into a footnote, typing, Esc, and a
  double-click on its reference through the window; a double-click on a
  note mark opens the Notes panel; the panels, Go To, and the commands
  through messages; screenshots in both themes.

Physical checklist (about five minutes):

- [ ] Put the caret in a sentence; Insert › Note (Ctrl+Alt+M): the Notes
  panel opens; type a note; a coloured mark shows in the text. Double-click
  the mark later: the note opens again.
- [ ] Select a word; Edit › Bookmarks… (Ctrl+Shift+F5): type a name, Add;
  move away and Go To; tick Show bookmarks on the page.
- [ ] Insert › Footnote (Ctrl+Alt+F) at a sentence's end: type the
  footnote at the page's foot; Esc returns; Tools › Footnote Options…: try
  Make endnotes and a custom separator.
- [ ] Save as SAM and reopen; Export Word Document and open it in
  LibreOffice: the note is a comment, the footnote a footnote.

### Merge, labels, envelopes (`UI_SPEC.md` §38, approved 2026-11-03, MG1–MG11)

**Result (2026-11-03): AUTOMATED PASS, VM PASS (MGM1–MGM7, MGS6, MGS7
reproduced from their inputs; twins AMD1, AML1, AMLB: Ami Pro opens the
app's data file, merge letter, and label document, and merges AML1 to the
app's merge paragraph for paragraph), PHYSICAL PASS (2026-11-04, the
user's recheck after the rework and the scenario suite's fixes; the first
check had failed the record editor and the panel's layout)**
(`docs/MERGE_PROGRESS.md`).

- Rules: `[recfile]`, every data kind, conditions, MergeField, MergeRec,
  NextRec, Skip, Defined, empty fields, unknown fields, label set-up and
  sheet places, envelopes.
- Messages and widgets: the Merge panel's and the record editor's buttons
  through the window (`click_unrun`; print and file dialogs never opened),
  each change one undo step with Undo, the outputs, View & Print,
  screenshots of the panels and the record editor in both themes.

Physical checklist (about ten minutes):

- [ ] File › Merge…, step 1, New Data File…: three names; the data file's
  table opens with the caret in its first record: type two records (Tab
  between values; Tab in the last cell adds a record); Table › Sort
  Ascending; Undo once. (Reworked 2026-11-04, MR1; the steps are MR2.)
- [ ] Back in the letter, step 2: click two fields with a space typed
  between; Preview and ◀ ▶ show each record. Step 3 opens the outputs;
  Options ▸ holds the conditions, labels, and one PDF per record.
- [ ] Merge to New Document, then PDF…; View & Print: edit the first
  record's text, Print and View Next, then Cancel (the print dialog
  opens once).
- [ ] Merge as labels: pick a sheet, click a start label, Merge to New
  Document: the labels sit on their sheet.
- [ ] Select an address in a letter, File › Print Envelope…: the
  recipient is filled; save a return address; New Document and PDF….

First use (2026-11-04, the user): **FAIL** for the record editor and the
workflow: "overly complicated, small and cramped", and the entry area is
counter-intuitive: after + Record, typing does not put values in the grid.
Entering data should work as Ami Pro's tables do. Reworked the same day
(MR1: data files are Ami Pro tables; MR2: the panel is Ami Pro's three
steps; `UI_SPEC.md` §38, "Rework after first use"); AUTOMATED PASS, VM
PASS (AMD1 as a table, AMT2), PHYSICAL PENDING (the checklist above).

### Saving over SAM files (user decision 2026-11-04)

**Result (2026-11-04): AUTOMATED PASS, PHYSICAL PENDING.**

- [ ] Open a copy of a SAM, edit it, Save: no file chooser; the status bar
  names `NAME.SAM.bak`, which is the original. Save again: the `.bak` is
  unchanged.

### Master documents (`UI_SPEC.md` §37, approved 2026-11-03, MS1–MS11)

**Result (2026-11-03): AUTOMATED PASS, VM PASS (MD2's and MD3's
`[sequence]` reproduced from their members; twins AMS, AMA–AMC: Ami Pro
shows a member's numbers in the set and works the app's master to MD3's
TOC and index), PHYSICAL PASS (2026-11-03, the user's check)** (`docs/MASTER_PROGRESS.md`).

- Rules: `[master]`, `[sequence]`, the carry (pages, footnotes, numbered
  styles, Seq), path lookup by name, the checks.
- Messages and widgets: the panel's buttons through the window
  (`click_unrun` for Add… and Locate…), live numbers after an edit and its
  Undo, the set's TOC and index, print, PDF, and export, the find bar's
  Find Next by a click; screenshots of the panel in both themes.

Physical checklist (about five minutes):

- [ ] Put three of your documents in a folder. In a new document, File ›
  Master Document…, Add… the three; the panel shows each file's pages in
  the set.
- [ ] Open the second file: its page numbers continue from the first's. Add
  a page break in the first: the second's numbers move at once.
- [ ] Generate Contents and Generate Index in the panel; Ctrl+click a TOC
  row opens its file there.
- [ ] Export PDF… and File › Export ▸ Word Document on the master: one
  file with all three.
- [ ] Find a word with Options ▾ › In All Files of the Master; Find Next
  goes on into the next file.

### TOC, index, glossary (`UI_SPEC.md` §36, approved 2026-11-03, TX1–TX8)

**Result (2026-11-03): AUTOMATED PASS, VM PASS (Ami Pro's TI1T2, TI2TOC,
TI3T, and TI1IDX reproduced from their inputs; twins ATI1, ATI1X, ATI3,
AGL1: Ami Pro shows them as its own, regenerates ATI1's TOC in place, and
inserts AGL1's records), PHYSICAL PASS (2026-11-03, the user's check)** (`docs/TOC_INDEX_PROGRESS.md`).

- Rules: `[toc]`, `[master]`, the marks, the TOC's page table and break,
  the index's order, merging, pages, Other, separators, and the glossary
  file's bytes as Ami Pro writes them.
- Messages and widgets: the panel's buttons through the window, Ctrl+click
  on a TOC row, Mark All, Ctrl+K and Tab completion, Undo after each;
  screenshots of the panels in both themes.

Physical checklist (about five minutes):

- [ ] In a document of yours with headings, Tools › TOC, Index…: the
  levels list its heading styles; Generate Contents puts the TOC on a page
  of its own at the start; Ctrl+click a row goes to its heading; Undo
  takes it out in one step.
- [ ] Select a word, Edit › Mark Text ▸ Index Entry…, Mark; Mark All on
  another word; Generate Index: a new tab with the index.
- [ ] Edit › Mark Text ▸ Glossary…: New… a data file, select a sentence,
  Add it as "sy"; type "sy" and press Ctrl+K.
- [ ] Save as SAM and reopen. Tell me how the panel and Ctrl+K feel.

### Proofing (`UI_SPEC.md` §35, approved 2026-10-26, SP1–SP8)

**Result (2026-10-26): AUTOMATED PASS, VM PASS (twin APF1: Ami Pro
checks the paragraph the app edited, shows British and All Rules),
PHYSICAL PASS** (`docs/PROOFING_PROGRESS.md`; the user, 2026-11-03: "proofing check passes").

- Rules: Ami Pro's words, options, walk, repeats, capitals, Replace All,
  the user dictionary's format; the thesaurus's word at the caret, base
  forms, and inflections; readability by Ami Pro's formulas (PGR1);
  `[lang]`, `[GramStyle]`, and `<:s>` as Ami Pro writes them.
- Messages and widgets: F7 and Shift+F7, the panel's buttons through the
  window, the right-click entries, the idle check; screenshots of the
  three pages in both themes.

Physical checklist (about five minutes):

- [ ] Type a sentence with a misspelling ("recieve") and a repeated word
  ("the the"): red wavy lines appear after a moment. Right-click the
  misspelling: choose a suggestion; right-click the repeat: Delete
  Repeated Word.
- [ ] Tools › Spell Check… (F7) in a document of yours: step through with
  Skip, Replace, Replace All, Add to Dictionary; try the options and the
  user dictionary section (Add, Remove).
- [ ] Put the caret in a word; Tools › Thesaurus… (Shift+F7): choose a
  meaning and a synonym, Replace. Right-click another word: Synonyms ▸.
- [ ] Tools › Grammar Check… on a paragraph with "This are a apple.":
  Replace, Skip; at the end the readability statistics.
- [ ] Change the language in the panel to British; save as SAM and
  reopen. Tell me how the underlines and the panel feel to use.

### Table data (`UI_SPEC.md` §41, TD.1–TD.12)

**Result (2026-11-06): AUTOMATED PASS, VM PASS (engine and storage; twins
ATD1–ATD5 opened in Ami Pro as made), PHYSICAL PENDING** (`docs/TABLE_DATA_PROGRESS.md`).

Physical checklist, ranked (about ten minutes):

- [ ] Table › Edit Formula… and the rail's Formula section: type `@sum(B1..B3)`
  in a cell and confirm; the total shows; double-click a formula cell to edit it.
- [ ] Quick Add ▸ Row and Column on a table with a heading row and a blank
  cell: the written formula appears; edit it; Del removes it.
- [ ] Change a price: every dependant changes in the same step; Undo returns all.
- [ ] Number display: Currency, a negative, red, and parentheses through the
  style; a typed `10` shows `10.00` on leaving the cell.
- [ ] Styles rail, Table format section: change the number format; every cell
  using the style follows; Revert.
- [ ] Tools › Sort…: sort a paragraph list and a table's rows by a key; Undo.
- [ ] Errors in the status bar: divide by zero and a loop say so; no crash; the
  rest of the table still calculates. REF shows as the word.
- [ ] View › Show › Table row/column headings: they show and hide.
- [ ] Paste or Cut into a formula cell: refused with "This cell holds a formula: use Table › Edit Formula… to change it" (built in the TD.7 follow-up); Copy still works.
- [ ] Open a table with Ami Pro's formulas: results and formulas are unchanged.

### Updates (`UI_SPEC.md` §43, P6.2)

**Result (2026-11-07): AUTOMATED PASS** (the panel's state machine, the checksum refusal, the missing-`pkexec` command and the start-up gating through the window's message path with a scripted backend), **PHYSICAL PENDING**. The first real check needs a published release (`docs/RELEASING.md`); until then the panel says "There is no published release yet".

Physical checklist, ranked (about ten minutes; needs a test release, e.g. a version above the installed one made with `tools/release.sh`):

- [ ] Help › Check for Updates… opens the rail's Updates panel at once and checks; with no release, or the installed version published, it says so ("no published release yet" / "You have the latest version"); Check again repeats it.
- [ ] With a newer release: the panel shows the version and the notes' first lines; Download and Install shows a moving bar, then "downloaded and its checksum matches" with the exact `pkexec apt-get install -y ...` line and Install / Not now. Not now removes the download.
- [ ] Install asks for your password once (polkit); on success the panel says the version is installed; Restart opens the new version and this window closes, asking about unsaved documents first. Declining the password says "not authorised" and installs nothing.
- [ ] A release whose `SHA256SUMS` does not match its package (make one by editing the list): the panel refuses it with "checksum does not match" and nothing is installed.
- [ ] Preferences › General: "Check for updates when the program starts" is off by default; switched on, the next start opens the panel only if a newer version exists, and not again for a day.
- [ ] Esc closes the panel; opening another rail panel closes it; the Help menu lists the command before About.

### Crash recovery (`UI_SPEC.md` §44, P6.3)

**Result (2026-11-07): AUTOMATED PASS** (the store with a fake clock and process table: live and gone sessions, whole-file writes, caps, 30-day cleanup, damaged notes and copies; and scenarios through the window's message path: edit, autosave tick, crash by dropping the window, a new window that lists and recovers, the original file byte-for-byte untouched, Save asking for a place, saved/undone/discarded documents leaving no copy, a clean exit leaving none, two windows not offering each other's, recovery off, a password document not copied), **PHYSICAL PENDING**.

Physical checklist, ranked (about ten minutes; use a scratch copy of a document):

- [ ] Open a saved document, type a few words, wait over 2 minutes (or set Preferences › General to 1 min), then `kill -9 $(pidof amipro-cosmic)`. Start the program: the Recover documents panel opens by itself, listing the document with its path, the time and the size; Recover opens a tab "name (recovered)" with your words, marked changed.
- [ ] Ctrl+S on the recovered tab opens Save As, not a silent save; the original file on disk is unchanged until you choose to replace it (then the usual overwrite question, loss list and `.bak` apply). After saving, "(recovered)" is gone and the copy is deleted (`ls ~/.local/share/amipro-linux/recovery/`).
- [ ] Two instances: start a second window while the first has unsaved changes and has autosaved; the second does not offer the first's copies. Kill the first (`kill -9`); File › Recover Documents… in the second now lists them.
- [ ] A never-saved document is listed as "Never saved" and recovers as an unsaved tab; Discard and Discard all delete the copies; Esc closes the panel and keeps them (they are listed again at the next start).
- [ ] A normal quit (File › Quit, answering the unsaved-changes question) leaves no files in the recovery folder; closing a changed tab with Discard removes its copy.
- [ ] Preferences › General: "Keep recovery copies while editing" is on by default; with it off nothing is written; the interval buttons (1, 2, 5, 10 min) take effect at once. Typing stays smooth during an autosave of a large document (try an imported 18-page file).
- [ ] The File menu lists Recover Documents… after Revert to Saved…; the panel looks right in light and dark themes.

### Objects (`UI_SPEC.md` §42, DO.3-DO.11)

**Result (2026-11-07): AUTOMATED PASS, VM PASS (OLE drawn 34 of 34; image state words 12 of 12; chart records 104 of 104 corpus and 18 of 18 VM; equations 27 of 27 byte equal; twins CHT1, EQT2, the adjusted `.tgf` and an OLE round trip opened in Ami Pro), PHYSICAL PENDING** (`docs/OBJECTS_PROGRESS.md`).

Physical checklist, ranked (about fifteen minutes):

- [ ] Open a file with a WordArt or 123Worksheet object: it is drawn. Select it: Frame Properties › Object shows Class, data size and presentation; Convert Object to Picture makes it an ordinary picture; Undo brings the object back.
- [ ] Edit › Links…: the rail lists the OLE objects and linked frames; a row selects its frame; nothing offers Update or Edit.
- [ ] Copy text, a picture from a browser, and a file in the file manager in turn; Edit › Paste Special… lists what each offers; Paste (or double-click a row) pastes as text, formatted text or picture; Undo is one step.
- [ ] Select a bitmap frame; Tools › Image Processing…: drag Brightness, Contrast, Edge Enhancement and Smoothing (the picture follows the drag; one undo step per drag), type a value, tick Invert, Revert. Save As SAM and reopen: the look is kept.
- [ ] Tools › Charting… in an empty frame: a chart appears; change the type icons, Legend, Grid, 3D, Depth, Perspective, colour set and Axis; edit the data grid and the Data text; Flip Data. Each change is one undo step. Double-click the chart reopens the panel.
- [ ] In a table with numbers, Tools › Chart from Table: a chart in a frame after the table. Save Chart as Image and Copy Data as CSV work.
- [ ] Tools › Equations… in an empty frame: type `\frac{a}{b}` in the card and the frame redraws as typed; an error shows its message and position; the rail palette puts templates at the caret and selects the first box to fill; Esc leaves.
- [ ] Copy a LaTeX formula (`$x^2$`) from another program; Tools › Paste LaTeX Equation makes an equation; Copy as LaTeX copies it back.
- [ ] Save As SAM, close and reopen: charts, equations and adjusted images are as left; open the file in the page view and the PDF export to see them drawn.
- [ ] Tell me how the equation and chart look beside Ami Pro's (the widths of `\sqrt` and `\left`...`\right` are the known weak spots) and how the rail panels feel.

## 11. Physical sign-off summary

| Gate | Result | Evidence/date | Remaining issue |
|---|---|---|---|
| A — Specification approval | PASS | Revision 2 commit `2037467` explicitly approved 2026-10-10 | Closed; implementation acceptance remains separate |
| B — Commands/keyboard | PHYSICAL ACCEPTANCE FOR TESTED FUNCTIONALITY | `85086b8`; user report 2026-10-10 | Ctrl+Tab conflict and untested checklist cases remain open |
| C — Action/tabs/styles | AUTOMATED PASS FOR UI-2 / PHYSICAL PENDING | `96aeda4`, 2026-10-11 automated; 2026-10-09 physical FAIL predates UI-2; 2026-10-11 user: closer to the mockup but not in line with it; 2026-10-12 user: skip the UI-2 checklist, do the UI pass on the page surface first | The combined physical check after the UI pass (`UI_GAP_LIST.md`); per-tab tooltip/a11y deviations |
| D — Workspace/status/zoom | PARTIAL PASS | Current geometry automation; physical UI rejected | Status/visual hierarchy; zoom future |
| E — Light/dark visual matrix | PENDING | 2026-10-11 user: current UI is not the final appearance | Required screenshots/review against `UI_MOCKUP.svg` |
| F — Editor/integrity | PARTIAL PASS | Existing automated + prior physical evidence | Accessibility, visual IME, rich-surface activation |
| G — Performance | PRIMARY PASS / REGRESSIONS PENDING | `bf0eaa5`; `README31.SAM` typing feels normal physically | Unreported drag/scroll/history/caret/tab checks |
| Page surface (D, F, G; §10) | AUTOMATED PASS / PHYSICAL PENDING | `2c9a38e`, 2026-10-12; widget-path and message-path tests; README31 typing 0.42–0.48 ms median | The §10 physical checklist; accessibility; picture decoding |
| Style interface (§10, `UI_SPEC.md` §13–16; Phase 3) | AUTOMATED PASS / PHYSICAL PASS | 2026-10-13: Style Properties' live edits and undo, following the caret, both right-click menus (after the scroll fix `b05da29`), and the Format › Styles ▸ dialogs (user: "That all works", "all worked") | ‹ Styles and × not asked physically (automated) |
| Frame interaction (§10, `UI_SPEC.md` §11) | AUTOMATED PASS; ORIGINAL COMPATIBILITY PASS (F14–F20, after `5694a8d`) / PHYSICAL PASS | 2026-10-12; controller, inspector, widget-path, and message-path tests; F14–F20 compared by stored rectangle; the user's desktop 2026-10-17 | Not separately reported: Esc during a drag, Tab between frames, Ungroup, a refused value in the rail, the outline in both themes; Repeat right/left mirroring on left pages (inferred, no fixture); accessibility of the page surface |
| Creating frames (§10, `UI_SPEC.md` §20) | AUTOMATED PASS; VM PASS (twins FC1, FC2, FC3, FC6N, FC8, FC9) / PHYSICAL PASS | 2026-10-17; the user drew frames, used Insert › Frame…, changed, moved, resized, grouped, restacked, deleted, and undid them, and Make Default carried, after `6cb15fc` and `9e31005` | None for this slice; pictures later (F4) |
| Bullets and numbering (§10, `UI_SPEC.md` §21) | AUTOMATED PASS; VM PASS (twins BL1, BL2, BL4) / PHYSICAL PASS | 2026-10-17, `docs/BULLETS_PROGRESS.md`; the user, 2026-10-18: "All list items checked" | None |
| Outline levels (§10, `UI_SPEC.md` §22) | AUTOMATED PASS; VM PASS (twins OL1, OL2, a nested list) / PHYSICAL PASS | 2026-10-18, `docs/OUTLINE_PROGRESS.md`; the user: "All list items checked", the Format menu "short enough now"; bullets to nest (`DECISIONS.md`) | None |
| Text that does not fit its box; shaded margins (§5, §8; 2026-10-18) | AUTOMATED PASS; VM PASS (HDR1, HDR2) / PHYSICAL PASS | `f3f3ac1`, `510a937`, `cfb4ac1`; the user, 2026-10-18: margins grey and the text area white; two lines typed in the top margin; the third hidden with the caret at the end of the second and the status bar saying why | None |
| Choosing colours (§10, `UI_SPEC.md` §23) | AUTOMATED PASS; VM PASS (twin ACS1) / PHYSICAL PASS | 2026-10-18, `docs/COLOUR_PROGRESS.md`; the user: "colour buttons all work", and nested bullets "look good" | None |
| Frames in table cells (§10, `UI_SPEC.md` §24) | AUTOMATED PASS; VM PASS (twins ATF2, ATF3, ATF7, ATF10, README31 with a page break) / PHYSICAL PASS | 2026-10-18, `docs/CELL_FRAMES_PROGRESS.md`; the user: "that all works" | None for this slice; pictures drawn, and put into empty cells, with pictures (F4) |
| Tables across pages (§10, `UI_SPEC.md` §26) | AUTOMATED PASS; VM PASS (twins ATP2, ATP5, ATP6); **PHYSICAL PASS** | 2026-10-19, `docs/TABLE_PAGES_PROGRESS.md`; the user's check 2026-10-19 | None; decisions TP1–TP5 confirmed 2026-10-19 |
| Fields (§10, `UI_SPEC.md` §28) | AUTOMATED PASS; VM PASS (twin AFD1); PHYSICAL PASS | 2026-10-20, `docs/FIELDS_PROGRESS.md`; the user, 2026-10-20: "Powerfields work" | None |
| Print and PDF (§10, `UI_SPEC.md` §29) | AUTOMATED PASS; VM PASS (the options against Ami Pro's printed fixtures) / PHYSICAL PASS | 2026-10-20, `docs/PRINT_PROGRESS.md`; the user, 2026-10-20: "printing works correctly", "pdf works correctly", the Print panel "looks good", no issues with Print Current Page… | The panel's buttons span the rail since the check (the user's choice) |
| Find & Replace (§10, `UI_SPEC.md` §27) | AUTOMATED PASS; VM PASS (twins FR1A, FR1B, FR2, FR3, FR5A, FR6) / PHYSICAL PASS | 2026-10-20, `docs/FIND_PROGRESS.md`; the user's check 2026-10-20, and their look at the bar's new place above the ruler the same day ("visual find and replace passed") | None |
| Export (§10, `UI_SPEC.md` §30) | AUTOMATED PASS; VM PASS (twins ATL1B, AF01: Ami Pro opens the app's RTF); LibreOffice comparison recorded / PHYSICAL PASS | 2026-10-20; physical 2026-10-24, `docs/EXPORT_PROGRESS.md` | — |
| Table editing (§10, `UI_SPEC.md` §32) | AUTOMATED PASS; VM PASS (twelve twins ATE*) / PHYSICAL PASS | 2026-10-24, `docs/TABLE_EDIT_PROGRESS.md`; the user, 2026-10-25: "tables pass", after the right-click fix `a46224d` | None |
| Import (§10, `UI_SPEC.md` §31) | AUTOMATED PASS; VM PASS (twins AIM1, AIMD, AIML, AIMN, AIMO's table) / PHYSICAL PASS | 2026-10-24, `docs/IMPORT_PROGRESS.md`; the user, 2026-10-25: "import physical check is accepted" | None |
| Drawings and Draw mode (§10, `UI_SPEC.md` §33) | AUTOMATED PASS; VM PASS (twins ADR1–ADR3, re-save ADR3S) / PHYSICAL PASS (2026-10-26) | 2026-10-25, `docs/DRAWING_PROGRESS.md` | None |
| Notes, bookmarks, footnotes, endnotes (§10, `UI_SPEC.md` §34) | AUTOMATED PASS; VM PASS (twins ANT1, ABK1, AFN1–AFN3) / PHYSICAL PASS | 2026-10-26, `docs/NOTES_PROGRESS.md`; the user, 2026-10-26: "all items pass" | None |
| Proofing (§10, `UI_SPEC.md` §35) | AUTOMATED PASS; VM PASS (twin APF1) / PHYSICAL PASS | 2026-10-26, `docs/PROOFING_PROGRESS.md`; the user, 2026-11-03: "proofing check passes" | None |
| TOC, index, glossary (§10, `UI_SPEC.md` §36) | AUTOMATED PASS; VM PASS (TI1T2, TI2TOC, TI3T, TI1IDX reproduced; twins ATI1, ATI1X, ATI3, AGL1) / PHYSICAL PASS | 2026-11-03, `docs/TOC_INDEX_PROGRESS.md` | None |
| Master documents (§10, `UI_SPEC.md` §37) | AUTOMATED PASS; VM PASS (MD2, MD3 reproduced; twins AMS, AMA–AMC) / PHYSICAL PASS | 2026-11-03, `docs/MASTER_PROGRESS.md` | None |
| Run A commands and views (§10, `UI_SPEC.md` §39) | AUTOMATED PASS; VM PASS (New… for nine sheets; line numbering LN1–LN20 and twins LNT1, LNT2; passwords PWA–PWEMP byte for byte and twins PWT1, PWT2; twin CAT1) / PHYSICAL PASS (CA.1–CA.12, then line numbering, passwords, and Move Up/Down: the user, 2026-11-05, "all test passed") | 2026-11-05, `docs/COMMANDS_PROGRESS.md` | New View, Vertical Ruler, Draft's frame markers not built (CK10, CK11) |
| Table data (§10, `UI_SPEC.md` §41) | AUTOMATED PASS; VM PASS (engine and storage; twins ATD1–ATD5) / PHYSICAL PENDING | 2026-11-06, `docs/TABLE_DATA_PROGRESS.md` | Physical checklist above |
| Updates (`UI_SPEC.md` §43, P6.2) | AUTOMATED PASS / PHYSICAL PENDING | 2026-11-07 | Physical checklist above; needs a published release |
| Crash recovery (`UI_SPEC.md` §44, P6.3) | AUTOMATED PASS / PHYSICAL PENDING | 2026-11-07 | Physical checklist above |
| Objects (§10, `UI_SPEC.md` §42) | AUTOMATED PASS; VM PASS (twins CHT1, EQT2, the adjusted `.tgf`, an OLE round trip) / PHYSICAL PENDING | 2026-11-07, `docs/OBJECTS_PROGRESS.md` | Physical checklist above |
| Pictures (§10, `UI_SPEC.md` §25) | AUTOMATED PASS; VM PASS (twelve twins: API1, API2, API5, API7, API8, APIRT, APNG, AJPG, APCX, ATIF, ACELL, AFRM; re-saves API8S, ACELLS) / PHYSICAL PASS (paste and drop after two fixes) | 2026-10-18, `docs/PICTURES_PROGRESS.md` | None |

Overall interface acceptance remains **FAIL / NOT ACCEPTED** until A–F required
for the implemented slice pass. Pending G regressions must remain green, but
the primary typing-responsiveness defect is closed.

## 1st line bold (2026-11-07): physical item

Automated (model, STY/SAM twins of VM FLB0-FLB2, layout, scenario, print
and PDF draw): green. PENDING physical check, in the app with FLB1.SAM
(`../AmiPro-Reference/vm-lab/OUT`, with `AMIPRO_STYLES` unset): the first
line of each Body Text paragraph is bold and breaks where `screens/flb_after.png`
breaks it ("quickly," ends the first paragraph's first line, "and" wraps),
the later lines are regular, the one-line paragraph is wholly bold; Format ›
Styles › Modify Style shows the 1st line bold box after Caps, checking it
unchecks Bold and the reverse, each change is live and one Undo step.
