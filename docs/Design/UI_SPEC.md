# AmiPro-Linux Interface Specification

**Status:** Revision 2 approved implementation baseline, 2026-10-10. Gate A is
closed. Implementation proceeds only through separately accepted slices; UI-1
is authorized and later visual slices still require their own acceptance.
**Scope:** Application shell and editing workspace. This document does not claim
pagination, frames, tables, printing, or original-format write-back.  
**Companions:** [command matrix](UI_COMMAND_MATRIX.md),
[reference comparison](UI_REFERENCE_COMPARISON.md),
[acceptance gates](UI_ACCEPTANCE.md), and
[annotated layout](UI_MOCKUP.svg).

## 1. Product direction

AmiPro-Linux is a document-first COSMIC desktop-publishing word processor:
conventional menus for complete command discovery, one compact action bar for
frequent work, document tabs, an optional Styles panel, an accurate ruler, a
clearly bounded paper page, and a quiet status bar. Native COSMIC controls and
themes govern the chrome; visible paper, margins, typographic hierarchy, ruler
alignment, and contextual properties make the workspace feel publication-
oriented rather than like a generic text field.

The following decisions resolve the rejected 2026-10-09 interface:

1. Every implemented editing action has a discoverable menu item and any
   assigned shortcut is printed at the menu item's right edge.
2. The action bar uses recognizable symbolic icons, grouping, tooltips, and
   toggle state; it does not use a row of undifferentiated text buttons.
3. Named style **management and browsing** has one primary home: the Styles
   panel. The action bar has one synchronized quick-apply field. The status bar
   does not contain another style picker.
4. The document establishes the visual hierarchy: chrome is compact and
   contiguous; a high-contrast paper boundary sits in a recessed workspace;
   visible margins, page edge, ruler, scrollbar, and text share one geometry.
5. Unsupported commands remain visible but disabled in menus when their
   location matters for learnability. Unsupported toolbar controls are absent.

## 2. Provenance and decision authority

| Tag                     | Meaning                                                                                                        | How it may influence the product                                                                      |
| ----------------------- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| **CURRENT**             | Behavior observed in the current application or its tests                                                      | Evidence of implementation only; not accepted visual design                                           |
| **AMI**                 | Verified Ami Pro 3.1 artifact or controlled VM observation                                                     | Preserve recognizable workflow where it does not conflict with safety or COSMIC conventions           |
| **GEMINI-PNG**          | a conceptual image (removed from the repository before publication)                                            | Composition inspiration only; depicted controls are not evidence that behavior exists                 |
| **GEMINI-HTML-MISSING** | The requested interactive Gemini HTML was not found in the repository, `Reference/`, or `../AmiPro-Reference/` | No interaction, CSS dimension, or state may be inferred from it                                       |
| **COSMIC**              | Pinned libcosmic/Iced source at revision `dbc62f2`                                                             | Native component behavior, theme spacing, menu shortcuts, tab sizing, tooltips, active state, scaling |
| **PRODUCT**             | This approved Revision 2 baseline and root product requirements                                                | Governs modernized behavior when original evidence is absent or deliberately superseded               |

See `UI_REFERENCE_COMPARISON.md` for the evidence ledger and exact original-app
questions. Conflicts resolve in this order: safety/data integrity, approved
product behavior, native accessibility/platform behavior, verified original
workflow, then conceptual artwork.

## 3. Information architecture and ownership

```text
Native title/header
Menu bar                    complete command discovery
Action bar                  frequent actions + one current-style quick apply
Document tabs               document identity, dirty state, close/new
┌ Context rail ┐ ┌ Ruler aligned to page/text geometry ┐
│ Styles       │ │                                      │
│ normally;    │ │ Editable paper page(s) in workspace  │
│ future Frame │ │                                      │
│ Properties   │ │                                      │
└──────────────┘ └──────────────────────────────────────┘
Status bar                  page/section, words, mode, messages, view/zoom
```

### Command ownership

- **Menus:** canonical inventory, shortcuts, enabled/disabled state, and access
  to infrequent commands. A user must not need to know a shortcut first.
- **Action bar:** only frequent, already functional commands. It is a shortcut,
  never the only route to a command. At full desktop width, common document,
  history, clipboard, style, character-format, and implemented paragraph
  commands are visible rather than hidden behind an unexplained overflow.
- **Context menus:** local equivalents such as Cut/Copy/Paste; they do not own
  unique functionality.
- **Dialogs/panels:** persistent or multi-property work. They are opened from a
  menu; action-bar affordances may open the same object.

### Style ownership and workflow

- The **Styles panel** is the primary home for named paragraph styles: browse,
  search/filter (when implemented), apply, inspect, create, modify, rename,
  delete, and later organize linked `.STY` definitions.
- The **action-bar style field** is the sole always-visible current-style
  display and quick-apply control. It shows the caret paragraph's style,
  “Mixed” for a mixed paragraph selection, and “Unavailable” only when there is
  no editable context. Choosing a style applies it to every selected paragraph.
- **Format > Styles** opens the Styles panel and moves focus to it. It is a route
  to the same owner, not another selector.
- The **status bar never repeats the style name or a style dropdown.** It is
  reserved for page/document state and transient feedback.
- Direct-format buttons (Bold/Italic/Underline) describe character formatting,
  not paragraph style. On a mixed selection they show a mixed/indeterminate
  state when the toolkit supports it; they never rewrite the style definition.
- Panel, action field, caret, selection, undo/redo, and active tab share one
  model-derived state. There is no independently stored “selected style”.

This intentionally departs from the conceptual PNG, which shows an additional
clickable style control in the status bar, and from the current application,
which exposes style information in still more places.

## 4. Screen geometry

All values are logical pixels before display scaling. Use active COSMIC theme
tokens where available; the figures below are acceptance ranges, not hardcoded
physical pixels.

| Region            | Target       | Allowed behavior                                                                                     |
| ----------------- | ------------:| ---------------------------------------------------------------------------------------------------- |
| Menu row          | 30–34 high   | Root labels use native menu padding; no card around the whole row                                    |
| Action bar        | 40 high      | One row; 4 px inter-control gap, 8 px group gap; labeled responsive `More` only below required width |
| Action icon       | 16 symbolic  | 32×32 minimum control box; native selected/disabled/focus state                                      |
| Style quick apply | 168–220 wide | Ellipsize middle/end; never shrink below 144                                                         |
| Tabs              | 40–44 high   | Native target is 44; each tab 96–240 wide; horizontal overflow                                       |
| Ruler             | 26–30 high   | Page-aligned; scrolls horizontally with page, fixed vertically above it                              |
| Styles panel      | 216 default  | User resize 176–320; 8 px internal padding; remembers visibility/width                               |
| Workspace padding | 24 minimum   | 32 at ordinary desktop widths; may reduce to 16 in narrow windows                                    |
| Page gap          | 24           | Continuous vertical stack only when real pagination exists                                           |
| Status bar        | 26–30 high   | One line, 8 px inner gap, separators between field groups                                            |

The pinned COSMIC default spacing scale is 4/8/12/16/24/32 logical pixels;
components must consume theme tokens so compact/standard/spacious density and
user scaling remain valid. Typography uses the theme's interface type for
chrome. Document fonts are never used for menus or status fields. Body chrome
text is normally 14 logical px; secondary status text may be 13, but never
below the user's scaled readable size.

### Alignment rules

- Menu, action bar, and tabs share the same left edge and have no arbitrary
  12-pixel outer card margin.
- Group separators in the action bar are 1 logical px and span 20–24 px.
- The ruler's zero/text-origin/margin markers are derived from the same page
  transform used by rendering and hit testing.
- The visible page is centered in the workspace **after** subtracting the open
  panel. When the page exceeds available width, it anchors to the leading
  workspace padding and horizontal scrolling exposes both edges.
- Vertical and horizontal scrollbars belong to the workspace, not the page.
- Paper never visually merges with the workspace. Use a theme-aware 1–2 px
  page edge with at least 3:1 non-text contrast at the boundary plus a
  restrained 3–6 px shadow. The surrounding canvas is visibly darker/recessed
  in both themes; the paper remains a print-oriented neutral surface.

## 5. Region behavior

### Native title/header

Format: `Document name — AmiPro-Linux`, followed by the platform's modified
indicator if one is not already conveyed by the tab. Imported legacy documents
may add a concise “read-only source” subtitle/state, not a long title sentence.
Window controls remain native.

### Menu bar

Order is **File, Edit, View, Insert, Format, Tools, Window, Help**. Menus are
vertical, grouped with separators, keyboard navigable, and close on selection,
Escape, or outside activation. Shortcut labels are fed through libcosmic's
`KeyBind` map so they render aligned at the right edge; an empty map is a defect.
Unimplemented items are individually named and disabled. A generic “Not
available yet” item is not a sufficient menu inventory.

The complete inventory, status, enabling rules, and shortcuts are normative in
`UI_COMMAND_MATRIX.md`.

### Action bar

Left to right at full width:

1. **Document:** New, Open, Save. Print occupies the next slot only after the
   print/layout/portal command works; until then the icon is absent and
   File > Print remains visibly disabled.
2. **History:** Undo, Redo.
3. **Clipboard:** Cut, Copy, Paste. These move from existing editor key paths
   into the shared command registry in the first approved UI slice.
4. **Style:** the paragraph-style quick-apply field.
5. **Character:** Bold, Italic, Underline when each is supported by the active
   production editor.
6. **Paragraph:** Align Left, Center, Align Right, Justify, then Bullets only
   after their semantic commands exist.
7. **More:** a text-labeled chevron menu that appears only when a narrow
   window displaces action-bar groups, and holds only those buttons. It never
   owns a command that lacks a conventional menu route and never contains
   placeholders. (Revised 2026-10-12 at the user's request: a More menu at full
   width, holding Save As, Paste as Plain Text, Select All, Styles Panel, and
   About, duplicated the menus and was not understood.)

Save As stays in File because it is not a repeated editing action. Print's
reserved position beside Save is normative even though printing is not
implemented in this phase. The Styles-panel collapse control remains in its
header; View > Styles provides the discoverable menu route.
The dashed printer outline in the annotated mockup documents that future slot;
it is not a control to render in the application before Print is functional.

When the bar does not fit, the style field contracts to 144 px first; then
whole groups move to the labeled More menu from the end of the bar, so the
order is the priority, and the style field's group stays. The fit is measured
from the widgets' widths, not fixed window widths (revised 2026-10-12: fixed
thresholds near 1180, 1040, and 860 px showed More on a mid-size window with
room to spare). No group is split into unlabeled orphan icons.

Every icon-only button has:

- a standard symbolic icon from the configured icon theme;
- an accessible name matching the menu command;
- a tooltip in the form `Command (Shortcut)` when a shortcut exists;
- hover, pressed, focus-visible, disabled, and selected state supplied by the
  active theme;
- a selected state for toggles such as Bold and Italic, never color alone.

No icon is shown until its command works through the same command path and has
correct enabled state. Text glyphs such as plain `B` and `I` may be used only if
they have intentional typographic treatment, tooltips, accessible names, and
toggle state; theme symbolic icons are preferred when unambiguous.

### Document tabs

- One tab per editing session; opening an already-open canonical path activates
  its tab.
- Tab content is document icon, ellipsized filename, modified dot, and close
  action. The entire active tab uses native selected styling; a different text
  label/button style alone is insufficient.
- Tooltip shows the full path and source type. Keyboard focus is visible.
- Close invokes Save/Discard/Cancel for dirty content. Imported `.SAM` Save
  routes to native Save As and never overwrites the source.
- `Ctrl+Tab`/`Ctrl+Shift+Tab` cycle in tab order. `Ctrl+W` closes the active tab.
- Tabs scroll horizontally or expose an overflow list; close and new affordance
  remain reachable. A plus button follows the last tab and means New Document.
- Per-tab content, model/history, caret/selection, viewport, formatting state,
  dirty state, source provenance, and save target remain isolated.

### Styles panel

- Opens at the start edge in left-to-right locales and defaults **visible on a
  fresh desktop profile at widths ≥1200** to teach the style-centric workflow.
  User visibility and width are then remembered. At narrower widths it defaults
  closed and opens as a non-modal overlay/drawer so the page is not squeezed.
  Resizing the window across that width keeps the rail's state: open stays
  open (an overlay when narrow, docked when wide again) and closed stays
  closed (the user, 2026-11-06; it used to disappear when a maximized
  window was restored).
- Header contains `Styles`, optional search, and collapse button. The body is a
  virtualized/scrollable named-style list when needed.
- Each row shows the style name and a restrained type marker where paragraph
  versus future character styles must be distinguished. Current uniform style
  is selected; mixed selection has no falsely selected single row.
- Single activation selects the row; Enter, double activation, or an explicit
  Apply action applies it. For the initial minimal slice, single activation may
  apply if this is announced and consistent, but keyboard behavior must match.
- Management actions are disabled until style model semantics exist; do not
  show decorative plus/minus buttons that do nothing.
- The left side is one **context rail**, so Styles and future object properties
  never create two simultaneous columns. Text/caret selection shows the Styles
  panel. After the frame model exists, selecting a frame replaces the rail's
  contents with a distinct **Frame Properties** inspector using the same width;
  a clear `Back to Styles` action restores Styles without changing selection.
  Returning to text selection restores Styles automatically unless the user
  explicitly pinned the inspector. This is not a tab inside style definitions,
  and no empty inspector, frame button, or pin control ships before the frame
  feature exists. `UI_MOCKUP.svg` shows both mutually exclusive rail states.

### Ruler

- Initially read-only and accurate. It displays page/text extents, units, and
  paragraph markers only when backed by the model.
- Ruler marks maintain a minimum readable contrast without competing with the
  page. Major units are labeled; minor ticks are quiet.
- Future draggable margins, indents, and tabs provide hover/focus handles,
  keyboard adjustment, numeric alternatives, snapping feedback, and one
  undoable transaction per drag. They remain absent until implemented.

### Page workspace

- The workspace fills all space between tabs and status bar. It has no global
  padding around unrelated chrome.
- The canvas and paper are distinct surfaces. The complete visible page edge
  is drawn whenever the viewport permits; at zoomed/narrow sizes scrollbars
  reveal both edges. Whitespace inside the page communicates actual margins,
  while optional nonprinting guides may outline them without entering output.
  The margins are shaded, as Ami Pro's Layout mode shows them (View › View
  Preferences › Margins in color, on by default: RGB 223, 223, 223 outside
  the text area, the text area white; the gap between columns is not
  shaded, G05; header and footer text stands on the shade). The paper is
  white in both themes, and so is the shade's colour (user decision,
  2026-10-18: it shows where the header band ends and the text begins).
- The page uses a credible portrait paper ratio in the default document. A
  small page-gap/page-number treatment may appear outside the sheet only when
  real pagination supplies page identity; decorative fake pages are forbidden.
- The page uses actual `PageSetup` aspect ratio and margins. It is editable in
  place; there is no preview/edit split and no transparent editor overlay with
  different glyph geometry.
- Until shaped pagination exists, the UI must describe page count as estimated
  where relevant and must not paint decorative extra pages that imply fidelity.
- Zoom is display scale only. Rendering, hit testing, caret, selection, IME
  preedit, ruler, page bounds, and scroll extents consume the same transform.
- Vertical wheel/trackpad scroll is ordinary; Shift+wheel may scroll
  horizontally according to platform behavior. Scrollbars remain reachable.
- Long documents retain the currently edited/caret region when editing changes
  layout. Tab switching restores each document's viewport.

### Status bar

Left group: Ami Pro's page arrows and Page Status button (user's guide
chapter 1; user, 2026-10-12): previous page, `Page X of Y` as a button that
opens Edit › Go To (Ctrl+G: a page number, first or last page, or the next
frame), and next page; the arrows put the caret at the top of that page's
text and scroll the page's top into view, and are disabled at the ends.
Optional `Section N`.
Center-left: word count and selection count when available. Center: a flexible
transient message region with middle ellipsis. Right group: insert/overwrite
mode if supported, view mode, zoom out, slider, zoom percentage, zoom in.

While the caret is in text that does not fit its box (a header, footer,
frame, or cell: Ami Pro hides it, V14), the message region says so, and
where room comes from: "This text doesn't fit in the header, so it's
hidden; a larger top margin makes room" (2026-10-18, after the user's
report that typing seemed to stop after Enter in a header). Ami Pro says
nothing. The caret stays at the end of the box's last visible line, as Ami
Pro shows it (VM HDR1, HDR2), and Up, Home, and End return to that line.

The bar does **not** include a style picker, duplicated filename, permanent
“Modified” prose, or low-value diagnostics. Dirty state lives in the tab/title.
Success messages time out; errors persist until dismissed or superseded and
must also be exposed accessibly.

## 6. Themes, contrast, and accessibility

- Light and dark themes use semantic theme colors, never fixed black/white UI
  chrome. Paper may remain light in dark theme for print fidelity, with a
  user-facing dark-canvas/document option considered separately; selection,
  caret, and page boundary must remain legible either way.
- Text and essential icons target WCAG 2.2 AA contrast: 4.5:1 for ordinary
  text, 3:1 for large text and non-text state indicators. Disabled content may
  be lower contrast but remains recognizable.
- Focus order follows visual order: menus, action bar, tabs, panel, ruler
  controls if interactive, editor, status/view controls. The editor does not
  trap Tab unless editing a literal tab is an explicit command.
- Every command has an accessible name, role, state, and enabled state.
  Selection and caret updates reach the accessibility tree. Icon-only controls
  have tooltips but tooltips are not their accessible-name mechanism.
- Menus support keyboard opening, arrows, Enter/Space, Escape, mnemonics where
  libcosmic supports them, and shortcut invocation independent of menu state.
- Minimum pointer target is 32×32 logical px in dense chrome; 40×40 is preferred
  for isolated controls. Resizing and 200% scaling must not clip labels.
- IME preedit is drawn at the live caret and exposed to the input method; a
  committed composition is one undo step.

## 7. Screen states

### Empty document

One active `Untitled` tab, editable blank page, Body Text in the action style
field, zero words, Save disabled until content/format changes (Save As remains
available in File). The Styles panel follows the saved visibility rule.

### Imported SAM

Tab shows source filename and an imported-source indicator. A compact,
non-alarming information banner or first transient message states: “Imported
source is protected; Save creates an .amipro copy.” Unsupported-content count
links to the fidelity report when such UI exists. The page remains editable.

### Mixed selection

The realistic acceptance state contains actual multi-paragraph text, a heading,
body text with mixed bold/italic emphasis, a selected phrase, and a visible
caret endpoint. Style quick apply reads `Mixed`; direct-format toggles use
selected, unselected, or mixed state. Applying one value affects the selection
through a single undoable command. The Styles panel does not highlight a false
current row. Schematic gray placeholder bars are not sufficient for visual
approval.

### Narrow window

Styles panel is closed/overlay, low-priority action groups move to overflow,
tabs scroll, and page becomes horizontally scrollable after minimum readable
scale. Menus and status retain access to all commands.

### Future object selection

When a frame/object model is implemented and selected, a Frame Properties
inspector replaces the Styles contents inside the same context rail; it does
not squeeze the page with a second panel. The inspector groups geometry,
anchoring/wrap, border/fill, and future text-flow/link properties, with only
model-backed fields visible. An object-context action group may then appear.
Until that separately approved milestone, no frame toolbar, resize handles,
empty inspector, or simulated frame behavior is implemented.

## 8. Implementation slices after approval

1. **Command surface:** populate real key bindings, complete Edit and other menu
   inventories, wire existing Cut/Copy/Paste/Select All and formatting paths,
   implement state-dependent enabling, and add menu/widget tests.
2. **Chrome hierarchy:** replace text-button row with grouped action controls,
   remove the status style picker, adopt native tab treatment, normalize
   padding/heights, and add tooltips/accessibility names.
3. **Styles workflow:** make the panel the management owner, synchronize it with
   the one quick-apply field, add responsive overlay behavior, and verify mixed
   state/keyboard focus.
4. **Workspace/status/zoom:** align ruler/page/scroll geometry, simplify status,
   and add zoom only when the transform is shared by render and hit testing.

Each slice must preserve safe file operations and the accepted default editor;
screenshots and the corresponding `UI_ACCEPTANCE.md` gates are required before
moving to the next material visual slice.

**Sequencing change (user decision, 2026-10-11).** After slice 2's physical
acceptance, the page, page-layout, and frame foundation (`PROJECT_PLAN.md`
Phase 2) comes before slices 3 and 4. Slice 4's workspace geometry and zoom are
delivered with the page editing surface, because zoom must share one transform
with page rendering and hit testing. Slice 3 follows in Phase 3.

## 9. Explicitly future architecture work

Frames, tables, true pagination, interactive ruler mutations, print/PDF, and a
Frame Properties inspector are only reserved in the interface. Pages, page
layouts, and frames are the next foundation (Phase 2 since 2026-10-11) and
require a reference-driven model/layout decision first; no mockup control is
proof of an implementation. Linked flow between frames is not described in the
3.1 user's guide and remains unverified. The foundational frame/layout decision
should be run with **Extra High** reasoning effort because it affects stable
object identity, document positions, undo, hit testing, pagination, and legacy
SAM fidelity. The bounded research backlog is in
`UI_REFERENCE_COMPARISON.md`.

## 10. Approval record

**Approved by the user, 2026-10-10:** Revision 2 at commit `2037467`, including
the distinct paper surface and margins, complete toolbar grouping and reserved
Print location, mutually exclusive Styles/future Frame Properties context-rail
states, desktop-publishing character, realistic editing state, single-owner
Styles workflow, compact hierarchy, modern COSMIC shortcuts, and responsive
remembered Styles-panel visibility. This closes Gate A.

Approval authorizes implementation in the slices above; it is not physical
acceptance of any implementation. UI-1 is the active slice. UI-2 and later
visual implementation do not begin until UI-1 receives physical COSMIC
acceptance.

## 11. Frame interaction (Phase 2.6) — approved 2026-10-12

**Status: APPROVED by the user on 2026-10-12 as proposed, and implemented
on 2026-10-12 in the `page-surface` build (automated gates pass; physical
acceptance and the VM fixtures F14 onward are pending; "Implementation
notes" below).** The page surface (2.6,
`page-surface` feature) edits the text of every frame, header, footer, and
table cell, but frames themselves cannot yet be selected, moved, resized, or
inspected. This section proposes how; nothing here is built until the user
approves it (`docs/PHASE2_RUNBOOK.md` §3). The model and its commands already
exist (`crates/editing/src/frames.rs`: `CreateFrame`, `DeleteFrame`,
`MoveFrame`, `SetFramePage`, `ResizeFrame`, `SetFrameStyle`,
`ChangePlacement`, `BringToFront`, `SendToBack`, `Group`, `Ungroup`), with
Ami Pro's rules: With para above frames move only horizontally, Flow with
text frames only vertically, groups move and delete as one, and page frames
always sit above repeating frames (V14, V15, `SAM_FORMAT_NOTES.md`).

### Selection states

The surface has three states, after Ami Pro (VM: one click inside a frame
selects it; a drag from inside moves it and its group):

1. **Text:** the caret is in a flow (today's only state).
2. **Frame selected:** one frame, or one group, is selected. No caret.
3. **Frame text:** the caret is in a selected text frame's text.

Transitions:

| From           | Action                                                           | To                                             |
| -------------- | ---------------------------------------------------------------- | ---------------------------------------------- |
| Text           | Click inside a page frame (Where placed, anchored, or repeating) | Frame selected                                 |
| Frame selected | Double-click inside a text frame, or Enter                       | Frame text (caret at the hit, or at the start) |
| Frame selected | Click inside a table frame                                       | Text in the clicked cell                       |
| Frame text     | Esc                                                              | Frame selected                                 |
| Frame selected | Esc, or a click in the main text                                 | Text (at the anchor for anchored frames)       |
| Frame selected | Shift+click another Where placed frame on the same page          | Several selected (Group becomes available)     |

Header and footer bands are not frames to the user: clicks there always enter
their text (as now); their margins belong to the page layout (Page › Modify
Page Layout, a later slice). Picture frames have no text state.

### Handles, moving, and resizing

- A selected frame shows eight square handles (7 logical px, the accent
  colour with a 1 px contrasting outline) on its corners and edge midpoints,
  half outside the frame, plus a 1 px accent outline. A group shows one
  outline and handles around all its members. Handles are drawn above every
  frame and stay 7 px at every zoom; hit testing uses a 12 px target and the
  shared page transform.
- Pointer cursors: move inside the frame, the matching resize cursor over a
  handle.
- **Move:** drag from inside a selected frame. A dashed outline follows the
  pointer; text reflows when the button is released, as one `MoveFrame`
  command (one undo step). With para above frames follow only horizontally
  and Flow with text frames only vertically; the outline shows the constraint.
  Esc during the drag cancels it. A Where placed frame stays on its page;
  moving it to another page is the inspector's Page field (`SetFramePage`).
- **Resize:** drag a handle; corner handles change two edges, Shift keeps the
  aspect ratio. The size cannot go below the frame's interior margins plus
  0.10 in. One `ResizeFrame` command on release. Groups are not resized.
- No snapping grid in the first version: exact values go through the
  inspector.

### Keyboard access

With a frame selected:

| Key            | Action                                                                                                |
| -------------- | ----------------------------------------------------------------------------------------------------- |
| Arrows         | Move by 1 pt (20 twips); Shift+arrows by 0.10 in. Repeated presses within a second form one undo step |
| Alt+arrows     | Move the right or bottom edge (resize) by the same steps                                              |
| Tab, Shift+Tab | Select the next or previous frame on the page, in stack order                                         |
| Enter          | Frame text (text frames) or the first cell (tables)                                                   |
| Delete         | Delete the frame or group after confirmation (`DeleteFrame`)                                          |
| Esc            | Back to the main text                                                                                 |
| Frame menu     | Bring to Front, Send to Back, Group, Ungroup, Modify Frame Layout (opens the inspector)               |

Ctrl+G (Go To › Next frame, as in Ami Pro) selects frames from the text state.
Every frame command is in the registry with its shortcut, so the menus, the
keyboard, and the action bar use one path.

### Frame Properties inspector

When a frame is selected, the context rail shows **Frame Properties** in place
of the Styles list (§5 "Styles panel" and §7 "Future object selection"). Only
fields the model holds appear, in Ami Pro's Modify Frame Layout order:

1. **Size & Position:** Page (Where placed), X and Y from the page's top-left
   corner, or the offset from the left margin (With para above); Width and
   Height; units from the ruler.
2. **Type:** placement (Where placed, With para above, Flow with text, Repeat
   all pages, Repeat right/left; `ChangePlacement`), text wrap (Wrap around,
   No wrap around, No wrap beside), Opaque or Transparent.
3. **Lines & Shadows:** sides (left, right, top, bottom), line style, line
   position (Middle, Inside, Outside, Close to inside, Close to outside), line
   colour; shadow (None, Normal, Deep, Custom extents) and its colour;
   background colour; rounded corners (percent).
4. **Columns & Margins:** columns 1–8, gutter, balance, line between columns;
   interior margins; tabs per column.
5. **Name** (and the Run macro, read-only until macros exist).

There is no Text flow or Link group: Ami Pro 3.1 has no linked frames. Each
field applies on Enter or when focus leaves it, as one command and one undo
step (`SetFrameStyle`, `ResizeFrame`, `MoveFrame`, `ChangePlacement`).
Invalid values are refused in place with the reason and keep the old value.
For repeating frames the inspector says that the change applies to every page
of the layout.

### Rail switching

- Selecting a frame switches an open rail to Frame Properties; deselecting
  returns it to Styles. The rail keeps its width and its open or closed state;
  Format › Frame Properties (or Enter on a selected frame when the rail is
  closed) opens it.
- In frame text, the rail shows Styles (styles apply to the frame's
  paragraphs, as now).
- The rail never shows both panels and never squeezes the page with a second
  panel.

### Implementation notes (2026-10-12)

Where the build refines the proposal, within its intent:

- **Handles that cannot resize are not drawn.** A With para above frame has
  no handles on its top edge (its top follows the paragraph) and a Flow with
  text frame none on its left edge (its left follows the text). Ami Pro draws
  them but they do nothing (F17). A group or
  several picked frames show eight handles around all of them, as proposed,
  but those do not resize; a press on them inside the outline moves.
- **"Frame menu" commands live in Format**, after Frame Properties: Bring to
  Front, Send to Back, Group Frames, Ungroup Frames. Select Next Frame is in
  Edit, after Go To. The approved menu roots stay eight. Ctrl+G is Go To,
  as in Ami Pro, whose dialog holds Next frame (since 2026-10-12; before the
  dialog existed Ctrl+G selected the next frame).
- **A press on an unselected frame selects it, and dragging from that press
  moves it** (Ami Pro). Shift+click never starts a drag.
- **Text frames and table cells are frame text**: Esc there selects the
  frame. Typing does nothing while frames are selected; Backspace deletes
  like Delete, after confirmation.
- **Undo.** A gesture is one step even when it issues several commands (a
  move of several picked frames; an anchored frame's move and resize), and
  arrow presses within a second of each other join one step
  (`History::amend`).
- **Colours.** Outline, handles, and the dashed drag outline use the same
  fixed blue as the text selection, with white handle borders; following the
  COSMIC accent colour is a visual-review item.
- **Inspector fields** (`frame_inspector.rs`): lengths show in inches and
  accept `in`, `"`, `cm`, `mm`, or `pt`; colours are `#RRGGBB` text fields
  (no colour picker yet); the line style is its number, 1–8 (Ami Pro's list
  is not drawn yet); the shadow is None, Normal (0.07 in), Deep (0.12 in), or
  Custom with a depth, plus the side it falls on; tabs per column are shown
  as a count, not edited; the macro is read-only. A With para above frame
  shows its offset from the left margin and a Flow with text frame its
  offset from the line, with the other position read-only. Changing the
  placement to With para above or Flow with text anchors the frame at the
  start of the first paragraph on its page at or below its top. With a group
  or several frames selected, the inspector asks for one frame.
- **Original compatibility (F14–F20, `SAM_FORMAT_NOTES.md`, "Frame
  gestures").** Moving and resizing Where placed and With para above frames
  give Ami Pro's stored rectangles, offsets, and sizes to the twip. Dragging
  a Flow with text frame down makes it hang below the baseline, as in Ami
  Pro; the engine ignored that offset until `5694a8d`.
- **Enter on a picture frame** (no text) opens Frame Properties, which
  reconciles "Enter: Frame text" with "Enter on a selected frame opens the
  inspector" in the two tables above.

### Acceptance gates (proposed)

- **Model and controller:** selection-state transitions; each gesture and
  key produces exactly one command with the placement's constraints; Esc
  cancels; undo restores the rectangle in one step; group selection.
- **Widget path:** handles hit at 50%, 100%, and 200%, scrolled and not;
  press, drag, and release move and resize; keyboard moves and resizes; Tab
  order; inspector fields round-trip through commands; no stale layout after
  each gesture.
- **Performance:** a drag redraws at least 60 times a second on
  `README31.SAM` with frames (the outline only; reflow on release).
- **Original compatibility:** new VM fixtures (F14 onward) that move and
  resize each placement kind in Ami Pro, compared with the same gestures here
  by stored rectangle.
- **Physical COSMIC:** handles visible and reachable in light and dark
  themes; cursors; drag and resize feel; inspector keyboard navigation and
  accessible names; the rail switch.

## 12. Editable toolbar (SmartIcons) — user requirement 2026-10-12, proposed design

The user asked that the toolbar ultimately be editable. Ami Pro 3.1's
SmartIcons are (user's guide, chapter 2, "Using SmartIcons"): named icon
sets, one shown at a time and chosen from Tools › SmartIcons or a status-bar
button; icons added, moved, removed, and grouped, then saved to the set or a
new set; sets deleted; position left, right, top, bottom, or floating, and
icon size; descriptions on hover; View › Show/Hide SmartIcons; custom icons
that run macros.

Proposed for AmiPro-Linux, built in the UI pass so that later commands land
in it without rework:

- **Data-driven action bar.** The bar is an ordered list of items: a command
  (any implemented command with an icon), the style field, or a group
  separator. The approved Revision 2 order (§5 "Action bar") is the built-in
  **Standard** set and cannot be deleted; the responsive rule becomes
  generic: items that do not fit move to `More` from the end of the list, so
  the user's order is the priority.
- **Tools › Customize Toolbar…** (the SmartIcons dialog): the available
  commands (implemented commands with icons, with their names and
  shortcuts) beside the current set's items; Add, Remove, Move up/down (and
  drag where the toolkit allows), Add separator; Save, Save as new set, Delete
  set, Reset Standard. Every change is previewed in the bar.
- **Sets** are chosen from the dialog and from View › Toolbar Set; the last
  set used comes back on the next start. Sets and the choice are stored in
  the user's COSMIC configuration, not in documents.
- **Show/hide:** View › Show Toolbar (no Ctrl+Q, which is Quit on COSMIC).
- **Descriptions:** tooltips always (`Command (Shortcut)`); no off switch.
- **Position and size:** top only at first, as COSMIC apps do; left, right,
  and icon size are later options; floating palettes are not planned (no
  native floating toolbars in COSMIC). Custom macro icons wait for macros.

**Decided by the user (2026-10-12):** one editable toolbar for now (no named
sets), at the top only. The dialog therefore edits one toolbar with Reset to
Standard; sets, other positions, and sizes are not planned until asked for.

**Implemented (2026-10-12):**

- **Tools › Customize Toolbar…** is a dialog with two lists: *Available*
  (the style list, then every implemented command with an icon, in menu
  order, without what is already on the bar) and *Toolbar* (the bar's items
  with separators). Add puts the chosen item after the selected one (or at
  the end); Remove, Move Up, Move Down, and Add Separator act on the
  selected item; Reset to Standard restores the Revision 2 set; Done closes.
  Each change shows in the bar at once and is saved. Opening the dialog
  shows a hidden bar. Drag and drop is not offered (no list reordering in the
  pinned libcosmic).
- **View › Toolbar** (a check item) shows or hides the bar.
- **Saved** in the user's COSMIC configuration
  (`~/.config/cosmic/org.amipro_linux.AmiPro/v1/`): `toolbar` (item names:
  the command's registry name, `Style`, `Separator`), `toolbar_visible`, and
  `styles_visible` (the docked rail, gap R5). A name this build does not
  know or cannot show is skipped when read, so an edit in the default build
  drops page-surface commands from the saved bar.
- **Icons** come from the COSMIC icon theme, which falls back to Pop and
  hicolor, not Adwaita: every command icon must exist there (checked on
  2026-10-12; Styles Panel uses `view-list-symbolic`).

## 13. Character formatting (Ami Pro's Text menu items) — user decisions 2026-10-12

The user wanted to apply the character attributes the importer reads, and
chose Ami Pro's own items over one Character… dialog. Format holds, after
Bold, Italic, and Underline: **Word Underline**, **Caps ▸** (Upper Case,
Lower Case, Initial Caps, Small Caps), **Special Effects…**, **Font…**, and
**Normal** (Ctrl+M). Ami Pro's Ctrl+W and Ctrl+N stay Close Document and New
here, so Word Underline has no shortcut. The Caps submenu's label keeps the
check column's space, so it lines up with the checked entries.

- **Special Effects…** has Ami Pro's check boxes: Superscript, Subscript,
  Double underline, Strikethrough, and Overstrike character with a
  one-character box (since Phase 3.3; `/` when the text has none; typing a
  character checks the box). Each box starts as the text shows the effect;
  OK switches only the boxes that changed, or a changed overstrike
  character, as one undo step.
- **Font…** has the face (the installed families and the document's style
  faces), the size (Ami Pro's list plus a typed size, 1–999 points), Ami
  Pro's 15 colour swatches (sampled from its dialog; a colour the text has
  outside them gets its own swatch), Revert to style, and a preview on
  paper white. OK writes face, size, and colour together, as Ami Pro's dialog
  writes all three; Revert to style returns them to the style.
- **Normal** returns the selected text's character formatting and its
  paragraphs' alignment, indents, and spacing to their styles (VM fixture
  C1: Text › Normal removed both a bold word and its paragraph's centring).

**Check marks and toolbar states follow the caret** (user, 2026-10-12, from
Ami Pro). With nothing selected, an attribute shows when the character on
either side of the caret has it (C1: Bold is checked just before and just
after a bold word, and not before the space ahead of it). **Choosing an
attribute with nothing selected formats the text typed next** (C1: Ctrl+B,
typing, Ctrl+B again types bold and then plain); the choice lapses when the
caret moves. Typed text joins a run at either end (C1).

**What you see is checked** (user decision): a bold heading shows Bold
checked, and choosing Bold there un-bolds it. Ami Pro checks only formatting
applied with the Text menu (C1: a bold, centred Title style shows neither
Bold nor Center checked), and its off returns to the style; the user kept
the current rule.

## 14. Style editing (Phase 3.6) — approved 2026-10-13 (decision D1)

The model and commands exist since Phase 3.4 (`DECISIONS.md`, "Styles in
the model"): Create Style, Modify Style, Rename, Remove, and Use Another
Style Sheet, each one undo step, with each style's origin (stored in the
document, from the style sheet, or defined nowhere). This section proposes
their interface. It follows Ami Pro's dialogs, as the user chose for the
Text menu items (§13). The reference screenshots are Ami Pro 3.1's, outside
Git in `../AmiPro-Reference/vm-lab/screens/`: `stylemenu.png` (the Style
menu), `createstyle.png`, `modstyle.png` (Modify Style, Font), `modlines.png`
and `modlinesA.png` (Lines), `modbn.png` and `modbn2.png` (Bullets &
numbers), `st1mgmt.png` (Style Management), `st1use2-crop.png` (Use
Another Style Sheet).

### Where the commands live

- **Recommended:** Format gets a **Styles ▸** submenu with Ami Pro's Style
  menu items in its order: Create Style…, Modify Style…, Use Another Style
  Sheet…, Save as a Style Sheet… (after decision D3), and Style
  Management…. The eight menu roots stay (UI-1). Ami Pro's Ctrl+A (Modify
  Style) is Select All here, so Modify Style has no shortcut at first.
- *Alternative:* a ninth root menu, **Style**, exactly as Ami Pro's.
- The Styles panel stays the management owner (§3): its header gets a
  **+** button (Create Style…), and each row a context menu: Apply, Modify
  Style…, Create Style Based on This…, Rename…, and Remove (Revert to Style
  Sheet for a sheet style the document changed).

### Create Style

Ami Pro's dialog (`createstyle.png`): **New style** (name), **Based on**
either a **Style** (the list, document styles marked) or **Selected text**
(the caret paragraph's style with its direct paragraph formatting), and
**Create**, **Modify** (create, then open Modify Style on it), **Cancel**.
A taken name (letter case aside) disables Create and says so.

### Modify Style

Ami Pro's single dialog (`modstyle.png`): on the left the **Style** list and
a **Modify** list of panels; on the right the panel and a sample set in the
style; OK, Cancel, and Save As (a new style from these settings). Save
(write the change into the style sheet) waits for decision D3. Panels:

| Panel             | Fields (all exist in the model)                                                                                                        |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Font              | face, size (list and points), colour swatches (the Font dialog's), Bold, Italic, Underline, Word underline, Double underline, caps, 1st line bold (exclusive with Bold; the first line of each paragraph is set bold and broken with bold widths) |
| Alignment         | Left, Center, Right, Justify; indents All, First, Rest, From right, Both sides; Hanging indent; the style's own tabs (a ruler)         |
| Spacing           | line spacing Single, 1½, Double, Custom; space above and below; Add in Always or When not at break; Text Tightness                     |
| Breaks            | page break before and after, allow breaks within, keep with previous and next, column break before and after, Next style with its list |
| Bullets & numbers | Ami Pro's bullet and number choices, text before, attributes, Right align (`modbn.png`)                                                |
| Lines             | rule above and below, line style, spacing, length Text, Margins, or Other, colour (`modlines.png`)                                     |
| Hyphenation       | a check box, enabled once decision D2 is made                                                                                          |

Ami Pro's Table format panel is built (§41, TD.5; it was not offered when this section was written). **1st
line bold** (2026-11-07; Ami Pro's Modify Style › Font check box beside
Initial caps) is a check box after the Caps list: checking it unchecks Bold
and the reverse, one undo step, live in the document (the page is the sample).
Unlike Ami Pro the Normal choice of Caps does not clear it, and it is not
disabled for a face without a bold (the app draws a synthesised bold). The sample uses the page surface's
shaping, on paper white.

### Style Management and Use Another Style Sheet

- **Style Management…** keeps Ami Pro's two lists (`st1mgmt.png`): Styles
  in document and Styles in style sheet, plus a third group for names
  defined nowhere, shown in the error colour as Ami Pro shows them red.
  Remove and Revert work now; Move (document to sheet) waits for D3;
  function-key assignment is left for later.
- **Use Another Style Sheet…** lists the style sheets in the style-sheet
  folders by description, as Ami Pro's "List by description" does
  (`st1use2-crop.png`), with Browse… through the portal. The folders become
  a setting (today `AMIPRO_STYLES` is the only way to name one).
- In the Styles panel, a document style shows Ami Pro's bullet before its
  name and a style defined nowhere shows its name in the error colour, with
  a tooltip saying it formats as Body Text.

### Implementation notes (2026-10-13)

- **Format › Styles ▸** holds Ami Pro's Style menu items in its order:
  Create Style…, Modify Style…, Use Another Style Sheet…, Save as a Style
  Sheet…, Style Management…, and Select a Style (Ami Pro's Ctrl+Y; it opens
  the Styles panel, as Format › Styles did). Items not built yet are
  disabled.
- **Save as a Style Sheet…** (both builds): a dialog with the description
  (prefilled with the document's own), then the portal's save dialog for
  the file, suggesting an 8.3 name Ami Pro can list (`QUARTERL.STY`), in
  the style-sheet folder when `AMIPRO_STYLES` names one. `.STY` is added to
  a name without an extension; another extension, or an existing file, is
  refused with a message and nothing is written (D3). A With contents
  check box, on each time the dialog opens as in Ami Pro (Phase 4.11),
  puts the main text, the page frames, the header and footer text, and the
  pictures into the sheet, so a new document made from it starts with them;
  off, the sheet holds the styles and the page layout alone. Run macro is
  not offered. Afterwards the document uses the new sheet,
  or, untitled, has none, as in Ami Pro (VM fixtures SD1, SD2, SS2), as one
  undo step. Screenshots: `window-savesheet-light`, `-dark` (harness).
- **Create Style…**: the name (a taken name, letter case aside, or one
  with `@`, a tab, or spaces around it is refused, the reason shown and
  Create disabled), Based on a listed style (the caret paragraph's at
  first; document styles marked •) or Selected text (the caret
  paragraph's style with its own paragraph formatting). The paragraph
  keeps its style, as in Ami Pro (SS2). Ami Pro's Modify button comes with
  Modify Style.
- **Style Management…**: Styles in document, Styles in style sheet, and
  the names defined nowhere in the error colour; << Move << (a sheet style
  stored in the document), Revert (a changed sheet style takes the sheet
  file's version again; the file is found as the importer finds it), and
  Remove (a document style's paragraphs format as Body Text; a changed
  sheet style takes the sheet's version, ST1). >> Move >> would change the
  sheet file, which D3 rules out, so it is not offered; function keys are
  left for later.
- **Use Another Style Sheet…**: the `.sty` files in the style-sheet folders
  (and a `styles` folder beside the document or one level up), listed by
  description or by file name, OK, Cancel, and Browse… (the portal's file
  chooser). A sheet chosen with Browse… adds its folder to the style-sheet
  folders, a preference saved with the toolbar's (Ami Pro's style-sheet
  path); `AMIPRO_STYLES` adds more. SAM import searches the same folders.
  Screenshots: `window-createstyle-*`, `window-management-*`,
  `window-usesheet-*`.
- **Modify Style…** (normal build; its sample needs the page surface): the
  Style list (the caret paragraph's style first), the Modify panels Font,
  Alignment, Spacing, Breaks, Bullets & numbers, Lines, and Hyphenation,
  and a sample: the paragraph above and below in Body Text and two in the
  edited style, laid out by the page engine and drawn by the page surface
  in the edited styles. Edits stay in drafts per style until OK, which
  applies every changed style as one undo step; Cancel drops them; Save
  As… makes a new style from the draft (the original is kept) and goes on
  editing it. Create Style's Modify… creates the style and opens Modify
  Style on it. Lengths are inches (also `cm`, `mm`, `pt`); a value that
  does not read keeps its box and says why. Bullets & numbers composes the
  text before the paragraph as Ami Pro does: the number with what is typed
  before and after it, then Text, then the bullet (VM fixture BN4); a
  text in another order is kept unless the panel is used. Ami Pro's
  Space for is not offered (not in the model; the Table format panel is built, §41);
  1st line bold is on the Font panel. Screenshots: `window-modify-*`.
- **Styles panel** (both builds): the header's **+** opens Create Style…;
  a style stored in the document shows Ami Pro's bullet (• Name), a name
  defined nowhere shows in the error colour with a tooltip ("Defined
  nowhere: formatted as Body Text"); each row's context menu (secondary
  click) has Apply, Modify Style… (normal build), Create Style Based on
  This…, Rename… (a dialog; a taken name is refused), and Remove (a
  document style; a changed sheet style takes the sheet's version again,
  as Style Management's Remove does, ST1). Each change is one undo step.
  Since 2026-10-13 the list has one such menu, around its scroller, for
  the row the right button pressed (a menu per row, inside the scrolled
  list, took no clicks once the list was scrolled, as on the page, §16);
  its entries act on that row when chosen, and it forgets the row when it
  closes.

### Questions for D1

1. Approve this section (Ami Pro's dialogs as above)?
2. Format › Styles ▸ (recommended) or a Style root menu?
3. Any panel or field to drop or add?

## 15. Style Properties in the rail — approved 2026-10-13, built

The user likes the Frame Properties inspector (§11) because the document
stays in view while a frame changes, and asked whether style editing could
work the same way. This section proposes moving Modify Style (§14) from its
modal dialog into the context rail.

### Proposal

```
┌ Style Properties ─────── ‹ Styles  × ┐
│ Style  [ Heading 1             ▾ ]   │   follows the caret's paragraph
│ ▾ Font                               │
│   Face [ Arial               ▾ ]     │
│   Size [ 20 ] pt   ■■■■■■■■■■■■■     │
│   [x] Bold  [ ] Italic  [ ] Underline│
│ ▸ Alignment and indents              │   groups fold, like Frame
│ ▸ Tabs                               │   Properties' sections
│ ▸ Spacing                            │
│ ▸ Breaks and next style              │
│ ▸ Bullets and numbers                │
│ ▸ Lines                              │
│   [ ] Hyphenation                    │
│ [ Save as New Style… ]               │
└──────────────────────────────────────┘
```

- **Opening:** Format › Styles › Modify Style…, the Styles panel's row menu,
  Create Style's Modify…, or Enter on a Styles row. The rail switches from
  the Styles list to Style Properties, as it switches to Frame Properties;
  ‹ Styles returns.
- **Which style:** the caret paragraph's, following the caret while the
  panel is open; the Style list at the top picks another, which then stays
  until the caret moves to a paragraph in another style.
- **Applying:** as Frame Properties: a check box or list applies at once; a
  typed value applies on Enter or when focus leaves its box; each change is
  one command and one undo step (`ModifyStyle`), and the document lays out
  again at once, so the document is the sample (the dialog's sample goes).
  A value that does not read is refused in place with the reason.
- **Fields:** the dialog's panels (§14) as folding groups, top to bottom,
  with the same fields and rules (inches; Ami Pro's bullet order).
- **Width:** the rail widens from 216 to about 300 px while Style
  Properties shows (the tab rows and bullet controls need it); on narrow
  windows it overlays the page as now.
- **What stays a dialog:** Create Style, Style Management, Use Another Style
  Sheet, and Save as a Style Sheet: one-shot actions with nothing to watch.

### Answers (user, 2026-10-13: "yes to all four")

1. The panel replaces the Modify Style dialog.
2. It follows the caret's style while open.
3. The rail widens to about 300 px while it shows.
4. Changes are live, each one undo step.

### Implementation notes (2026-10-13)

- `style_properties.rs` (normal build) replaced `modify_style.rs`; the
  dialog's sample went with it. The rail shows, in order of priority,
  Frame Properties (a frame selected), Style Properties, or Styles.
- **Opening:** Format › Styles ▸ Modify Style…, the Styles panel's row
  menu, Create Style's Modify…, and the page's right-click menu (§16).
  The rail opens if it was closed. Enter on a Styles row is not wired yet
  (the rows take no keyboard focus).
- **Header:** ‹ Styles returns to the list; × closes the rail, as on the
  other rail panels, and Style Properties with it. Closing the rail any
  other way (View › Styles Panel) also ends it.
- **Which style:** the panel keeps the caret paragraph's style as it last
  saw it; when the caret reaches a paragraph in another style it shows
  that one. A style picked from the Style list stays until then. While a
  frame is selected there is no caret, and Frame Properties shows instead.
- **Applying:** each check box, list, or radio choice is one
  `ModifyStyle` command at once; a typed value applies on Enter or when its
  box loses focus (`on_unfocus`), and a value that does not read stays in
  its box with the reason under the Style list. After every message the
  panel takes the style's definition from the document, so Undo and Redo
  show at once and typed text is dropped only when the style changed.
- **Sections:** Font (open at first), Alignment and indents, Tabs,
  Spacing, Breaks and next style, Bullets and numbers, and Lines fold
  independently; Hyphenation and Save as New Style… stay at the bottom.
  The fields and rules are §14's (inches; Ami Pro's bullet order; Keep
  With only when breaks within are off; at most 22 tabs).
- **Save as New Style…** asks for a name in the panel, creates the style
  from the shown definition (Create Style, then the definition, as one
  undo step), and shows the new style; the caret's style is unchanged.

## 16. Right-click menus on the page — requested 2026-10-13, built

The user asked for right-click menus (2026-10-13) and approved the
contents with §15. Ami Pro 3.1's right button opens a dialog directly
rather than a menu (user's guide: Modify Style in the main text, chapter
7; Modify Page Layout in a margin, chapter 8; Modify Frame Layout on a
frame, chapter 15), so each menu puts that target first.

```
On text                              On a selected frame
┌──────────────────────────────┐     ┌──────────────────────┐
│ Modify Style "Body Text"…    │     │ Frame Properties…    │
│ ──────────────────────────── │     │ ──────────────────── │
│ Cut                   Ctrl+X │     │ Bring to Front       │
│ Copy                  Ctrl+C │     │ Send to Back         │
│ Paste                 Ctrl+V │     │ ──────────────────── │
│ ──────────────────────────── │     │ Group Frames         │
│ Font…                        │     │ Ungroup Frames       │
│ Special Effects…             │     │ ──────────────────── │
│ ──────────────────────────── │     │ Delete        Delete │
│ Paragraph Style            ▸ │     └──────────────────────┘
└──────────────────────────────┘
```

- **What the press selects:** the right press reaches the page before the
  menu opens on the release. On a frame outside the selection it selects
  that frame; elsewhere it moves the caret to the pointer, unless the
  pointer is in the selected text or on a selected frame, so that the menu
  acts on what was clicked. A right press never starts a drag.
- **Text menu:** Modify Style names the caret paragraph's style and opens
  Style Properties (§15). Paragraph Style ▸ lists the styles as the Styles
  panel does, checks the paragraph's style, and applies the chosen one.
  Entries follow the commands' enabled state (Cut and Copy need a
  selection) and show their shortcuts.
- **Frame menu:** the frame commands as Format has them; Group and Ungroup
  are enabled only when they apply; Delete asks first, as the key does.
- **Margins:** a right press in a margin moves the caret to the nearest
  text and shows the text menu; Modify Page Layout joins when Page Setup
  is built.
- **Placement:** libcosmic draws the menu inside the window; it opens to
  the left of the pointer when the click is right of the window's centre
  (its rule for in-window menus), and long lists scroll. The menu wraps
  the pages' scrollable, not the pages inside it: inside, it got the
  pointer in scrolled coordinates, and once the pages were scrolled it
  neither laid out at the pointer nor took clicks (user, 2026-10-13: Modify
  Style, Cut, Copy, and Paste did nothing).
- **Cost:** the menu is built with each view; on README31 typing costs
  about 0.05 ms more per keystroke at the median (three interleaved
  release rounds, 2026-10-13).

## 17. Save As an Ami Pro document — decided 2026-10-13 (Phase 4, decisions A and B), built

Implementation notes only: a file type choice in File › Save As and one
dialog; no new screens.

- **Choosing the format.** The Save As portal dialog has two file type
  filters, "AmiPro-Linux documents (.amipro)" and "Ami Pro 3.1 documents
  (.SAM)", and a Format choice with the same two entries, which portals
  that support choices return. A typed `.sam` or `.amipro` decides
  first; otherwise the Format choice, with its extension added (`.SAM`);
  without either, the document is saved as `.amipro`, as before. `.sty`
  is refused: style sheets have Format › Styles › Save as a Style Sheet.
- **Writing over an Ami Pro document (since 2026-11-04; A until then).**
  A document opened from a `.SAM` or saved as one is at that file: Save
  writes over it, after the loss check (B), and the first time the file is
  written over its original is kept beside it as `NAME.SAM.bak` (copied
  before anything is written; an existing `.bak` is never replaced, so it
  stays the first original). The status bar says "Saved Ami Pro document
  PATH (N bytes); the original is kept as NAME.SAM.bak" when it made one.
  Save As onto an existing `.SAM` asks first, as for `.amipro` ("Replace
  existing file?", naming the backup it will keep). The new file is
  written beside its name with the original's permissions and renamed
  into place, so a failed write leaves the file as it was; a link is
  followed to its file. A new `.SAM` is written beside its name and linked
  into place, which fails rather than replace a file that appeared
  meanwhile. Imports from other formats and style sheets stay protected:
  their Save goes to Save As.
- **Refused and failed saves.** The status bar alone was easy to miss
  (user, 2026-10-13), so every refused or failed save opens a dialog with
  the reason, **Choose Another Name…** (suggested; the file chooser again),
  and **Cancel**: a `.sty` name, a write error, a file chooser that could
  not open, and the same refusals in Save as a Style Sheet (whose Choose
  Another Name… keeps the description and With contents). Screenshots
  `window-saveproblem-light`, `-dark`.
- **What Ami Pro cannot store (B).** Before writing, the document is
  written in memory, read back, and compared (`amipro_formats::sam_losses`).
  When nothing differs the file is written at once and the status bar says
  "Saved as Ami Pro document PATH (N bytes)". Otherwise a dialog lists
  each kind of change once:

```
┌──────────────────────────────────────────────────────┐
│ Save REPORT.SAM as an Ami Pro document?              │
│                                                      │
│ Ami Pro 3.1 cannot store everything in this document │
│ exactly. Saving as an Ami Pro document changes:      │
│ • 182 characters that Ami Pro cannot store, written  │
│   as ?                                               │
│                                                      │
│                         [Save Anyway]  [ Cancel ]    │
└──────────────────────────────────────────────────────┘
```

  Cancel is the suggested (default) button and writes nothing; Save Anyway
  writes the file. Closing a tab or quitting with unsaved changes follows
  the same path: the tab closes only after the file is written, and Cancel
  keeps it open.

- **Tests and screenshots:** `save_as_sam_writes_new_files_only_and_lists_what_ami_pro_cannot_store`
  (`main.rs`; both formats, refusal of an existing file, both answers, the
  close path, files read back) and `window-savesam-light`/`-dark` in
  `window_screenshots`.

## 18. Page Layout Properties in the rail — approved 2026-10-13 (L1–L4), built

Decision C put page layout setup (headers, footers, the page) after Phase 4
in its own plan section; the Phase 4 run writes this proposal and builds
none of it. The model, the commands (`InsertLayout`, `ModifyLayout`,
`RemoveLayout`, `InsertBreak` with a layout, `InsertFloatingHeaderFooter`),
the layout engine, native v3, and the SAM reader and writer already hold
everything below except the page-number field's settings dialog; only the
interface is missing.

### What Ami Pro 3.1 has (VM, 2026-10-13)

Page menu: Header/Footer…, Insert Page Layout ▸ (Insert, Remove, Revert),
Modify Page Layout…, Ruler ▸, Page Numbering…, Line Numbering…, Breaks….
Modify Page Layout is one dialog with five panels chosen on the left
(Modify: Margins & columns, Page settings, Lines, Header, Footer) and a
Pages group (All, Right, Left; Mirror when left pages differ):

- **Margins & columns:** a tab ruler (tab type, leader, position, unit,
  Set Tab, Clear Tabs), Left, Right, Top, Bottom margins, 1–8 columns, the
  gutter width, Column balance, and a page preview.
- **Page settings:** Letter, Legal, A4, A3, A5, B5, or Custom width × height;
  Portrait or Landscape.
- **Lines:** lines around the page (All, Left, Right, Top, Bottom) in a
  style, their position (Inside, Close to inside, Middle, Close to outside,
  Outside), a line between columns in a style, and a colour.
- **Header, Footer:** the band's own tab ruler, margins within the band,
  columns, gutter, Column balance, and Begin on second page.
- **Header/Footer…** chooses the fixed header or footer to type in, and
  opens Floating Header/Footer… (insert a floating header or footer for
  All, Right, or Left pages at the caret, or remove one there).
- **Page Numbering…** inserts a page-number field: style (1, I, i, A, a),
  Start on page, Start with number, and leading text.

### Proposal

```
┌ Page Layout Properties ── ‹ Styles  × ┐
│ Layout [ Standard            ▾ ]      │   the caret page's layout
│ Pages  (•) All  ( ) Right  ( ) Left   │
│        [ ] Mirror                     │
│ ▾ Page                                │
│   Size [ Letter ▾ ]  8.50 × 11.00 in  │
│   (•) Portrait   ( ) Landscape        │
│ ▸ Margins and columns                 │   margins, columns, gutter,
│ ▸ Tabs                                │   balance; the page's tabs
│ ▸ Lines                               │
│ ▸ Header                              │   band margins, columns, tabs,
│ ▸ Footer                              │   Begin on second page
│ [ Insert Page Layout ] [ Remove ]     │
└───────────────────────────────────────┘
```

- **Opening:** Format › Page Layout… (Ami Pro's Modify Page Layout), the
  page's right-click menu in a margin (§16 reserved it), and a double-click
  in the margin. The rail switches to it as it does to Style Properties;
  ‹ Styles returns, × closes.
- **Which layout:** the layout of the page holding the caret, following
  the caret while the panel is open; the Layout list picks another. Pages
  chooses whether a change goes to all pages or only right or left ones (a
  first Right or Left change gives the layout its own left pages, as Ami
  Pro does); Mirror appears when they differ.
- **Applying:** live, as Style Properties: each change is one `ModifyLayout`
  command and undo step, and the pages lay out again at once, so the
  document is the preview (the dialog's preview goes). A value that does
  not fit (margins wider than the page) is refused in place with the
  reason.
- **Rail priority:** Frame Properties (a frame selected), then the panel
  last opened of Page Layout Properties and Style Properties, then Styles.
  With the caret in a header or footer, the panel opens that band's
  section.
- **Insert Page Layout** adds a copy of the caret page's layout from the
  next page on (a page break naming it, `InsertBreak`), named `1`, `2`, …
  as Ami Pro's writer names them; **Remove** takes out an inserted layout
  no page break names (the break first, then the layout, one undo step).
  Ami Pro's Revert (a break back to the previous layout) is an Insert
  Page Layout that picks an existing layout in the Layout list.
- **What stays a dialog** (one-shot, nothing to watch): Insert › Floating
  Header/Footer… (insert or remove at the caret; All, Right, or Left
  pages) and Insert › Page Number… (style, start page and number, leading
  text), the page-number field going in at the caret, usually in a header
  or footer. Edit › Go To Header and Go To Footer put the caret in the
  fixed header or footer (Ami Pro's Header/Footer…); a click there already
  does.
- **Width:** about 300 px, as Style Properties.
- **Not proposed:** Line Numbering (not in the model), the Ruler submenu's
  inserted rulers (the paragraph's own ruler is imported and written; an
  editor for it belongs with Text › Indents and spacing), and a Page menu:
  the commands go to Format and Insert as above unless the user wants Ami
  Pro's menu back (question 2).

### Questions for the user (ranked)

1. Page layout editing as a rail panel, live and one undo step per change,
   like Style Properties, rather than Ami Pro's modal dialog? - yes rail panel
2. Commands under Format and Insert as above, or a Page menu as in Ami Pro
   (Header/Footer, Insert Page Layout, Modify Page Layout, Page Numbering,
   Breaks)? - lets do under format to match everything else
3. Floating headers and footers and the page-number field as small
   dialogs (one-shot), as proposed? - As long as you think it matches the new interface
4. The panel follows the caret's page while open, as Style Properties
   follows the caret's style?- yes

### Answers (user, 2026-10-13)

Recorded in `DECISIONS.md` as L1–L4: a rail panel, live, one undo step per
change; the commands under Format (Format › Page Layout…), with Insert ›
Floating Header/Footer… and Insert › Page Number…; those two as small
one-shot dialogs, which matches the interface's rule (dialogs for one-shot
actions, rail panels for what changes live); and the panel follows the
caret's page.

### Implementation notes (2026-10-13, the page layout setup run)

Built as approved (L1–L4), from the decompiled `AMIPROUI.DLL` and
`AMIUICNT.DLL` (`docs/research/PAGE_LAYOUT_NOTES.md`) and twins made in Ami
Pro in the VM (`docs/LAYOUT_SETUP_PROGRESS.md`). Modules:
`page_layout_properties.rs` (the panel), `page_dialogs.rs` (the two
dialogs), `lengths.rs`; the rules are `amipro_editing::layout_setup`.

- **Opening:** Format › Page Layout… and File › Page Setup… (Ami Pro's
  name for the same command; one handler, 2026-11-07), a double-click in a page's left or right margin, and the
  right-click menu there, which puts Page Layout… first, above the text
  menu (§16). The panel and Style Properties replace each other in the
  rail; Frame Properties still comes first while a frame is selected.
- **Layout list and Pages:** the list names the document's layouts; the
  panel follows the caret's page (L4) and, with the caret in a header or
  footer, opens that section. Pages shows All while every page is alike and
  Right otherwise; choosing Right or Left changes nothing until a setting
  changes (Ami Pro copies the right pages to the left at once; the result
  is the same and nothing appears in the undo list for a mere choice).
  Choosing All while left pages differ is a change of its own (one undo
  step), which takes the left headers and footers and their text with
  them, as Ami Pro's OK does; Undo brings them back.
- **Mirror** is available with Right or Left, as in Ami Pro; while it is
  on, All is refused with the reason (Ami Pro greys it).
- **Page:** the Size list holds Ami Pro's six sizes and Custom; Width and
  Height are always shown and typing one makes the size Custom, as typing
  in Ami Pro's custom boxes does. Sizes are stored as Ami Pro stores them
  after its two-decimal boxes (A4 11,908 × 16,833 twips). Ami Pro's dialog
  also leaves A3, A4, A5, and B5 columns up to 5 twips inside the margins
  (it computes them from its table's sizes); the app keeps the columns
  even with the margins.
- **Margins and columns:** Columns is a list (Ami Pro: eight radio
  buttons); the gutter shows only with two or more columns and Column
  balance is unavailable with one, as in Ami Pro. Limits are Ami Pro's
  (each column at least a twip wide; 22 inches at most) except that the
  top and bottom margins must leave some height (Ami Pro accepts none and
  then cannot place text).
- **Tabs:** one list for the whole layout, edited as Style Properties edits
  a style's (kind, position, leader, Add tab, remove); Ami Pro's ruler is
  not drawn in the panel.
- **Lines:** Around page (All, Left, Right, Top, Bottom), the style list of
  12 (shared with Modify Style's lines), the position list, Line between
  columns with its own style, and one colour for both, as in Ami Pro. The
  style, position, and colour controls show only while a line is on: the
  model keeps them with the line (Ami Pro keeps them with the line off).
  The page now draws them where Ami Pro does (probes AP1–AP5).
- **Header, Footer:** the band's margins, columns, gutter, balance, tabs,
  and Begin on second page. A smaller top or bottom margin shrinks the
  band's own margins (bottom first), on the page kind changed (Ami Pro's
  dialog sets both kinds' band height from the edited one). Edit › Go To
  Header and Go To Footer put the caret in the page's fixed header or
  footer, or say why there is none (Begin on second page).
- **Insert Page Layout and Remove:** From the next page chooses between a
  copy of the layout shown and each existing layout again, so Ami Pro's
  Revert (Standard again) is one of the choices, as proposed; Insert Page
  Layout puts the page break at the caret (with an empty paragraph after
  it at the end of the text, as Ami Pro adds one). Remove takes the break
  before the caret that switches to the shown layout and, when no other
  break names it, the layout, in one undo step. Ami Pro also opens Modify
  Page Layout on a new copy; here the panel is already open and follows
  the caret onto it.
- **Insert › Page Break, Column Break** (Ctrl+Enter, Ctrl+Shift+Enter, as
  approved earlier): a page break at the caret without splitting the
  paragraph; a column break at the start of the caret's paragraph, once.
  Removing either is deleting its mark, as for any object.
- **Insert › Floating Header/Footer… and Page Number…** are small dialogs
  (L3). The floating dialog opens on Remove when one is at the caret, as
  Ami Pro's does; Insert puts the caret in the new one on the next page.
  Page Number… inserts Ami Pro's field (style, Start on page, Start with
  number, leading text) or, at a field, changes it (Ami Pro always inserts
  a new one); its sample says what the caret's page will show.
- **Lengths** in all three panels (Style, Frame, Page Layout Properties)
  read as Ami Pro's boxes read them: two decimals, truncated, 567 twips a
  centimetre (`lengths.rs`).
- **Not built when written** (Line Numbering and the Ruler submenu were built in run A, §39): Ami Pro's units
  button (inches only, as Style Properties).
- **Screenshots** (`page_layout_properties_screenshots`, light and dark):
  `window-pagelayout-page`, `-margins`, `-tabs`, `-lines`, `-header`,
  `-footer`, `window-floating`, `window-pagenumber`.

## 19. The ruler: tabs, indents, margins, and columns — approved 2026-10-13 (R1–R4), built

The user, 2026-10-13: "in ami pro you can see and control tabs and column
from the ruler". Before the ruler run the page surface's ruler showed the
paper, the shaded margins, inch numbers, and the caret paragraph's indent
markers, and took no input. Built in the ruler run (2026-10-17); see the
implementation notes at the end.

### What Ami Pro 3.1 does (user's guide ch. 8; VM RU1, RU2, 2026-10-13)

- **Showing.** View › Show Ruler shows the *current ruler* above the
  document in Layout mode: the tabs, indents, and margins in effect at the
  caret, for the caret's column only, and the caret's x as a dotted line.
  In a header, footer, text frame, or table cell it shows that box. A
  vertical ruler for the top and bottom margins is a View Preferences
  option.
- **Ruler mode.** A click on the ruler (or Edit › Go To › Ruler) enters it:
  the menus grey out and a row appears under the ruler with the four tab
  types (left, right, centre, decimal), a leader button, Cols (1–8), a
  unit button, Clear Tabs, and the pointer's position from the paper's
  left edge. A click in the text leaves it.
- **Top half: tabs and indents.** A click adds a tab of the chosen type
  where clicked, unsnapped (RU1: 3228 twips from the margin); a tab drags
  along the ruler (not past its neighbours) and off it to delete; the
  indent markers (First, Rest, both, Right) drag. Keyboard moves snap to
  the nearest ruler mark.
- **Where tab changes go.** Into the paragraph's own ruler, a copy of the
  stops in effect plus the change, on the caret's paragraph or the
  selected ones (RU1 wrote `<:R1,13,…>`: the page's 12 stops and the new
  one). It stays with the paragraph through cut and paste, Enter carries
  it on, and Page › Ruler › Remove takes it off. **Indent changes** are
  paragraph formatting (`<:I…>`; Text › Indention › Revert to style
  undoes them).
- **Bottom half: margins and columns** of the box's layout (page layout,
  band, or frame), not the paragraph's: drag a margin arrow; drag in the
  gap between two columns to move it (both widths change, the gap stays:
  RU2's 240-twip gutter between columns of 5,610 and 3,510 twips, written
  as the page layout's edges `1440 7050` and `7290 10800`); drag inside a
  column to move it (the gaps on each side change, its width stays). Cols
  changes the column count, which makes the columns equal again.

### What the model has

A paragraph's own ruler (`ParagraphFormat::tabs`, read and written as
`<:R…>`), paragraph indents (`<:I…>`), and a layout box's margins,
columns, gutter, and tabs (`ModifyLayout`). No command sets a paragraph's
tabs or indents yet. **Columns are always equal**: edges read from a file
that differ are kept only to be written back, the engine lays out equal
columns, and any layout change drops them.

### Proposal

```
 [⇥ Left ▾] ▶·····⇥·····⇥·····⇥·····⇥······◀ ▒▒▒ ▶·····⇥·····◀
            ▕1    ·    ·    ·    2    ·   ▏▒▒▒▕    3    ·  ▏
            └ tabs and indents of the caret's column (top half)
            └ margins, every column, and the gaps (bottom half)
```

- **Always live, no ruler mode.** View › Ruler shows or hides it (a check
  item, saved like View › Toolbar); while shown it takes the pointer at
  once. Hovering a marker shows its name and position; dragging shows a
  guide line down the page and the position, and the change applies on
  release as **one command and one undo step**.
- **Tab type** from a small selector at the ruler's left end (Left,
  Right, Centre, Decimal; Leader ▸ None, Dots, Dashes, Line), as Ami
  Pro's row did. A click in the top half adds that tab where clicked
  (unsnapped, RU1); a tab drags between its neighbours and off the ruler
  to delete. Right-click on a tab: its type and leader, Remove; on the top
  half: Clear Tabs, Remove Paragraph Ruler (back to the style's or the
  column's stops).
- **Tab changes** give the caret's paragraph, or the selected ones, their
  own ruler as Ami Pro does (a copy of the stops in effect plus the
  change; Enter carries it on). **Indent markers** drag First, Rest, both
  (the bar under them), and Right, as paragraph formatting on the same
  paragraphs.
- **Bottom half** shows the margins and every column of the caret's box
  (Ami Pro shows only the caret's column), and edits that box's layout:
  drag a margin; drag a gap to move it; drag a column to move it between
  its gaps. Right-click: Columns ▸ 1–8, Even Columns, Page Layout… (or
  Frame Properties in a frame). A table cell shows its tabs and indents
  only, as in Ami Pro.
- **Keyboard:** Edit › Go To gets Ruler, as in Ami Pro: arrows move a
  ruler cursor (Ctrl for larger steps, snapping to marks), Space adds a
  tab, Delete removes it, 0 1 2 3 set the indents, `[` `]` the margins,
  Esc returns to the text.
- **Unequal columns** (question 2) need a model change: a layout box keeps
  its column widths when they differ; the engine lays them out; native
  and SAM keep them as Ami Pro's edges; Page Layout Properties and Frame
  Properties show "unequal" with an Even Columns button. Imported unequal
  columns then lay out as Ami Pro shows them.
- **Not proposed:** the vertical ruler (question 4), the text-editor
  build (no pages), and Ami Pro's unit button (inches, as the panels).

### Questions for the user (ranked)

1. Tabs set on the ruler go into the paragraph's own ruler, as in Ami Pro
   (the caret's or selected paragraphs, carried on by Enter), rather than
   into the style's or the page layout's tabs (those stay in Style
   Properties and Page Layout Properties)? Recommended: yes.
2. Allow unequal columns (drag a gap or a column), which needs the model
   change above, rather than only margins and the column count? Recommended:
   yes, as Ami Pro does, and imported documents then lay out right.
3. The ruler always live, with a tab-type selector at its left end and
   right-click menus, rather than Ami Pro's click-to-enter ruler mode with
   a tool row? Recommended: always live.
4. Add the vertical ruler for the top and bottom margins now, or later?
   Recommended: later.

### Answers (user, 2026-10-13: "I agree with your recommendations")

Recorded in `DECISIONS.md` as R1–R4: tabs into the paragraph's own ruler;
unequal columns, with the model change; always live, with the tab-type
selector and right-click menus; the vertical ruler later.

### Implementation notes (2026-10-17, the ruler run)

By Ami Pro's rules from the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`
(`docs/research/RULER_NOTES.md`) and the user's guide, twinned in the VM
(RU1–RU12: the app's twins are written as Ami Pro writes them and Ami Pro
shows them as its own).

- **The ruler** (`ruler_view.rs`, model `ruler.rs`) is 36 px: tabs and
  indents in the upper half, margins and columns in the lower half, every
  column of the caret's box white and the margins and gaps grey, quarter
  inch ticks with the inch numbers beside them. The caret column's tabs
  show as Ami Pro's glyphs (left, right, centre, decimal; a leader as dots
  before it); First is the upper triangle, Rest the lower one, All the bar
  between them, Right the triangle at the right. Column edges (the margins
  included) are handles in the lower half.
- **Hovering** a marker puts its name and position (from the paper's left
  edge) in the status bar; a **drag** draws a guide line down the page
  and on the ruler, snaps by truncation to 1/16 in (`10d0:1a40`), stops at
  Ami Pro's limits (a tab between its neighbours and within the indents,
  First and Rest not left of All, columns at least a twip, an edge between
  its neighbours), and applies on release as one command and one undo step;
  Esc abandons it. A **click** in the upper half where no marker is adds a
  tab of the kind the button at the left end shows, where clicked and not
  snapped (RU1); a tab dragged down onto the page (or above the ruler) is
  removed (RU4). A press on the ruler leaves the caret and its keyboard
  focus in the text.
- **Tabs** go into the selected paragraphs' own ruler, one list for all of
  them: a copy of the stops in effect at the caret (its own ruler, else its
  style's tabs, else the box's) with the change (`1148:1329`, `1170:052d`).
  A selection that ends at the start of a paragraph leaves it out (except an
  empty one, for tabs). **Indents** change one value on each selected
  paragraph, which keeps its other three (`12e8:056d`).
- **Columns:** a margin drag moves only the outer edge (RU10), an edge drag
  only that edge (RU9), a gap drag the two edges around it (RU2), a column
  drag both of its edges; Columns ▸ n spreads them evenly with a 240-twip
  gutter whatever the gaps were (RU11, RU12). In Page Layout Properties a
  margin makes columns whose gaps are alike even again and otherwise moves
  only the outer edge, as Ami Pro's dialog does (`1238:02d0`); a column
  count or gutter makes them even (a count, and Even Columns, with the gap
  they share, else 240, as `1238:008a`); Mirror reverses them; the gutter
  shows only while the gaps are alike, and not for one column, and
  "Unequal columns — Even Columns" shows while a box keeps uneven ones
  (Frame Properties likewise).
- **The kind button** at the left end: a click gives the next kind (Left,
  Right, Centre, Decimal); its right-click menu chooses the kind and the
  leader.
- **Right-click menus** (one menu around the ruler, outside the scrollable,
  built for what the right press hit): on a tab, its kind, leader, and
  Remove Tab; in the upper half, Clear Tabs (a ruler without stops),
  Remove Paragraph Ruler, and the new tabs' kind and leader; in the lower
  half, Columns ▸ 1–8, Even Columns, and Page Layout… (Frame Properties… in
  a frame).
- **Keyboard:** Edit › Go To Ruler (Ami Pro's Edit › Go To › Ruler) gives
  the ruler the keyboard, its line at the caret paragraph's indent:
  Left/Right move it 1/16 in, Ctrl+Left/Right half an inch; Space adds a
  tab there, Del removes the one there, 0 1 2 3 set All, First, Rest,
  Right, `[` and `]` the caret column's left and right edges; Esc or a
  click in the text returns (user's guide, ch. 8). Ami Pro's Space+arrow
  to move a tab is left out (drag it).
- **Boxes:** in a frame the ruler edits the frame's interior (Frame
  Properties' columns and margins), in a header or footer the band through
  the page layout, in a table cell or a floating header only tabs and
  indents (Ami Pro's ruler also drags table column widths in a cell; not
  built, with table cell formatting).
- **View › Ruler** shows or hides it, saved with the toolbar's visibility;
  hidden it leaves a one-pixel slot so that the pages keep their scroll.
- **Text › Normal** keeps a paragraph's own ruler, as Ami Pro's does; the
  SAM writer writes `<:R>` after a paragraph with a ruler, as Ami Pro's does.
- **Not built:** the vertical ruler (R4), Ami Pro's unit button (inches
  throughout), a tab type change by Space+arrow.


## 20. Creating frames — approved 2026-10-17 (F1–F4), built

(Superseded: Insert › Frame… is built, as this section proposed.) Insert › Frame… was Future when written: frames can be selected, moved, resized,
and changed (§11), but only frames read from a file exist. This section
proposes how frames are created, by Ami Pro's rules from the decompiled
`AMIPRO.EXE` and `AMIPROUI.DLL` and the VM fixtures FC1–FC11
(`docs/research/FRAME_CREATION_NOTES.md`). Approved as proposed
(answers below); built in the frame creation run (`docs/FRAME_CREATION_RUNBOOK.md`).

### What Ami Pro 3.1 does (user's guide ch. 15; FC1–FC11)

- **Two ways.** Frame › Create Frame… opens a small dialog: Width, Height,
  Down from top, In from left (from the paper's edges), a unit button, OK,
  Cancel, and Manual. OK makes the frame; Manual closes the dialog and
  turns the pointer into a frame for drawing. The frame SmartIcon (in the
  default set) goes straight to drawing.
- **The dialog remembers** the last frame made, drawn ones included, across
  sessions (2.00 × 2.00 in, centred on a letter page, the first time; at
  least 0.25 in each way). Each box's limit keeps the frame on the paper;
  a value past it is refused in place.
- **Drawing:** a press on the page and a drag draw a dotted outline, kept
  on the paper; the release makes the frame, in either drag direction, to
  the twip. A click makes nothing and ends drawing, as do Esc and a press
  outside the paper (which then acts as an ordinary click). One frame per
  drawing. No minimum size.
- **The new frame** is an empty text frame, Where placed, on the page in
  view (the dialog) or the page drawn on, with the **default frame's**
  settings: opaque, Wrap around, a thin black line on all sides (Middle),
  a Normal shadow falling right and down, 18 % rounded corners, 0.10 in
  margins, white, one column, no tabs. Margins that do not fit a tiny frame
  are dropped. It goes **on top** of the page's Where placed frames, is
  **selected** (handles; the caret stays where it was, so Esc returns to
  it), and is one undo step. Text typed in it starts as Body Text.
- **Make Default** (Modify Frame Layout) makes the selected frame's wrap,
  display, lines, colours, shadow, corners, and margins the default for
  every later frame, in every document and session; never its placement,
  size, columns, or tabs.
- **Layering:** Where placed frames are drawn above anchored frames,
  whatever order they were made in (FC7G, FC8); the file lists the Where
  placed frames first. The app draws page frames in one stack order, so
  imported anchored frames are drawn over Where placed ones today.

### What the app has

The model and command already exist (`CreateFrame`, new frames at the
front of the stack with one empty Body Text paragraph), and the SAM writer
writes app-made frames as Ami Pro does (Phase 4, APP1 and APP2). Missing:
the commands, the dialog, drawing on the page, Ami Pro's default frame
(`FrameStyle::default()` has no shadow and square corners), Make Default,
and the layering.

### Proposal

```
 Insert › Frame…                       Insert › Draw Frame (toolbar ▭)
 ┌ Create Frame ─────────────────┐     pointer: crosshair over the pages
 │ Width   [2.00 in]  Height [2.00 in]  status: "Drag on a page to draw a
 │ Down from top [4.50 in]           │          frame. Esc cancels."
 │ In from left  [3.25 in]           │     ┌╌╌╌╌╌╌╌╌╌┐  dashed outline,
 │      [Draw on Page] [Cancel] [Create]   ╎         ╎  kept on the paper
 └───────────────────────────────┘     └╌╌╌╌╌╌╌╌╌┘  release → frame
```

- **Insert › Frame…** opens a small one-shot dialog, as Ami Pro's: Width,
  Height, Down from top, In from left, with the last frame's values (kept in
  the app's settings, like the toolbar), lengths typed as in the rail panels
  (`lengths.rs`: inches, or `cm`, `mm`, `pt`, `"`), each refused in place
  with its reason when the frame would leave the paper. **Create** makes the
  frame on the page in view (the page at the workspace's centre); **Draw on
  Page** closes the dialog and starts drawing; Cancel.
- **Insert › Draw Frame** starts drawing directly (Ami Pro's SmartIcon),
  also from the toolbar (question 1). While drawing, the pointer is a
  crosshair over the pages and the status bar says how to draw; a press on
  a page and a drag show the same dashed outline as a frame move, its
  moving corner kept on that page's paper; the release creates the frame.
  A release within 3 px of the press, Esc, or a press outside every page
  ends drawing without a frame (a press outside then acts as usual).
- **The new frame** is Ami Pro's: an empty text frame, Where placed, with
  the default frame's settings (Ami Pro's built-in default until Make
  Default changes it), one column, no tabs, margins that do not fit dropped
  as Ami Pro drops them; on top of the page's Where placed frames; **one
  `CreateFrame` command and one undo step**; then **selected** (handles, and
  the rail shows Frame Properties if it is open), with the caret left where
  it was: Esc returns to it, Enter or a double-click puts the caret in the
  frame. The dialog then offers the new frame's size and position.
- **Make Default for New Frames** (question 2) at the end of Frame
  Properties: the frame's wrap, opaque or transparent, lines, line style,
  position, and colour, background, shadow and its colour, corners, and
  margins become the default, kept in the app's settings; never its
  placement, size, columns, or tabs, as in Ami Pro. **Restore Ami Pro's
  Default** beside it.
- **Layering** (question 3): Where placed frames are drawn and hit above
  anchored frames, as in Ami Pro; Bring to Front and Send to Back move a
  frame within its kind; the SAM writer lists Where placed frames before
  anchored ones, as Ami Pro's does.
- **Twins:** FC1 (the dialog), FC2 (a drawn frame over another), FC3 (a
  tiny frame), and FC6N (after Make Default) made in the app, written as
  Ami Pro wrote them and opened in Ami Pro as its own; FC8's layering drawn
  as Ami Pro draws it.
- **Not proposed:** Ami Pro's keyboard way of drawing (arrows and
  Space; the dialog is the keyboard way), pictures (question 4), the frames
  Ami Pro makes itself when a picture or object arrives with no frame
  selected (they come with pictures), and the text-editor build (no pages).

### Questions for the user (ranked)

1. Create frames both ways, as Ami Pro: Insert › Frame… (the dialog, with
   Draw on Page) and Insert › Draw Frame, with Draw Frame also on the
   Standard toolbar, as Ami Pro's default SmartIcons have it? Recommended:
   yes, both, and on the toolbar.
2. Add Make Default for New Frames (and Restore Ami Pro's Default) to Frame
   Properties, kept in the app's settings for every document, as Ami Pro's
   Make Default? Recommended: yes.
3. Draw Where placed frames above anchored ones, as Ami Pro does (FC8),
   with Bring to Front and Send to Back working within each kind?
   Recommended: yes; it changes how a few imported documents with
   overlapping frames look, to match Ami Pro.
4. Pictures (Insert › Image…, Ami Pro's File › Import Picture) in this run,
   or later with picture decoding? Recommended: later; it needs picture
   decoding and Ami Pro's picture records, a run of its own.

### Answers (user, 2026-10-17: the four recommendations)

Recorded in `DECISIONS.md` as F1–F4: both ways, with Draw Frame on the
Standard toolbar; Make Default for New Frames and Restore Ami Pro's
Default, kept in the app's settings; Where placed frames above anchored
ones, with Bring to Front and Send to Back within each kind; pictures
later, with picture decoding.

### Implementation notes (2026-10-17, the frame creation run)

By Ami Pro's rules from the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`
(`docs/research/FRAME_CREATION_NOTES.md`), twinned in the VM (FC1–FC9: the
app's twins write Ami Pro's frame records and Ami Pro shows them as its
own).

- **Insert › Frame…** (`page_dialogs.rs`): Width, Height, Down from top,
  In from left, in inches (lengths as the rail panels read them), from the
  last frame made, drawn ones included (kept in the settings), or 2.00 ×
  2.00 at 4.50 and 3.25 the first time; each value is refused in place
  with its reason when the frame would leave the paper (Ami Pro keeps the
  focus in the box without a message). **Create** makes the frame on the
  page at the workspace's centre (Ami Pro's current page, FC9), **Draw on
  Page** starts drawing, Cancel. The remembered size is at least 0.25 in,
  as Ami Pro's.
- **Insert › Draw Frame** (and the toolbar's square, last in the Standard
  set): the next press on a page, a frame there included, starts the
  frame; a crosshair over the pages; the dashed outline of a frame move,
  its moving corner kept on that page's paper; the release makes the
  frame. A release within 3 px of the press (Ami Pro: at the same point),
  Esc, or a press outside every page ends drawing without one; the press
  outside then acts as usual.
- **The new frame** is Ami Pro's: an empty text frame (written as a
  `[txt]` without paragraphs until typed in), Where placed, with the
  default frame's settings, one column, no tabs, margins that do not fit
  dropped by Ami Pro's rule, on top of the page's Where placed frames,
  `CreateFrame` as one undo step; then selected, with the caret where it
  was.
- **Make Default** (Ami Pro's label; "Make Default for New Frames" does not
  fit the 216 px rail) and **Restore Default** end Frame Properties under
  "New frames", with what is kept spelled out, as rail buttons that wrap
  their labels rather than cut them off (the user saw "Restore Ami Pro's
  Default" cut off in their interface font, 2026-10-17); the
  default is kept in the settings as Ami Pro's 17 `FrDefs` numbers (the
  unit is always inches: the app shows inches throughout).
- **Layering:** anchored frames are drawn and hit below Where placed ones;
  Bring to Front and Send to Back move a frame within its kind, the other
  kind keeping its places; the SAM writer lists frames in Ami Pro's order
  (cell frames and Where placed, then anchored); a group is a run among
  the Where placed frames.
- **Drawing (found on the way):** each page frame is drawn in a renderer
  layer of its own, so an opaque frame hides the text behind it, main text
  and other frames' text alike (a layer draws all its quads before its
  text); frames with rounded corners are drawn rounded (a circle's quarter
  where Ami Pro draws an ellipse's at 100 % on unequal sides).
- **Frame text areas** follow Ami Pro's integer corner inset and line width
  (`FrameStyle::text_insets`, `1530:0892`), in the writer, the importer,
  and the engine.
- **Not built:** pictures (F4), the frames Ami Pro makes itself for a
  picture or object, Ami Pro's keyboard way of drawing (the dialog is the
  keyboard way).


## 21. Bullets and numbering — approved 2026-10-17 (B1–B5)

(Superseded: both commands are built.) Format › Bullets (Ctrl+Shift+L) and Format › Numbering were Future when written. Bullets
and numbers already work as Ami Pro's do, through paragraph styles
(Phase 3.2e and 3.6: imported, counted, drawn, edited in Style Properties,
written back). This section proposes the two commands, Ami Pro's Insert
Bullet, and the display gaps, from the decompiled `AMIPRO.EXE` and
`AMIPROUI.DLL`, the user's guide, and VM fixture BL1
(`docs/research/BULLETS_NOTES.md`). The user asked (2026-10-17) that the
commands improve on Ami Pro's where Ami Pro's are lacking.

### What Ami Pro 3.1 does (user's guide ch. 6, 7, 11; BL1)

- **No Bullets or Numbering command.** A list is made by applying a style
  the sheet has (Bullet 1, Bullet, Number List: in `_DEFAULT.STY`, 32 of
  the 63 shipped sheets, and 104 of the user's 192 for Number List), by
  adding a bullet or number to a style in Modify Style › Bullets &
  Numbers, or by inserting bullets into the text by hand.
- **Edit › Insert › Bullet…** puts one bullet into the text at the caret:
  a small dialog lists Ami Pro's 17 bullets (ten shapes, seven pictures)
  with an example. The bullet is one character to the caret and Delete,
  takes the size of the text where it stands, and is written `<:B<*X>>`.
- **Numbers** count each unbroken run of paragraphs in the style (a level
  counts on across other styles); nothing restarts or continues a list by
  hand. Outline numbering (Style › Outline Styles) is Phase 5.
- **Right align**, on in every shipped Number List, ends the number one
  space before the first line's text, so "9." and "10." line up.
  **Space for** is the style's First indent under another name.

### What the app has

Style bullets and numbers are imported, counted, drawn, and written;
Style Properties edits a style's bullet, number, text, attributes, and
Right align. Missing: the two commands; inserted bullets (`<:B…>` is kept
as an invisible legacy mark, so README31's bullets are not drawn);
Right align (numbers are drawn at the left indent), superscript bullet
text, and Ami Pro's seven picture bullets (all drawn as one arrowhead);
Space for.

### Proposal

```
 Toolbar: … [≡ ≡ ≡ ≡] | [•≡] [1≡] | [▭]       Insert › Bullet…
                         Bullets Numbering      ┌ Insert Bullet ─────────────┐
 Format                                          │ [·][•][▪][■][□][◆][◆]      │
   …                                             │ [◦][○][✓][☛][❏][▀][☑]      │
 ✓ Bullets          Ctrl+Shift+L                 │ [☒][➢][➣]   Example:  •    │
   Numbering        Ctrl+Shift+N                 │              [Cancel] [Insert]│
                                                 └────────────────────────────┘
```

- **Format › Bullets and Format › Numbering** (question 1), toggles with
  check marks, also in the text right-click menu and on the toolbar
  (question 2). Checked when the caret paragraph's style shows a bullet
  (Bullets) or a number (Numbering). **On:** every selected paragraph takes
  the document's bullet style (Bullet 1, else Bullet) or number style
  (Number List), the names Ami Pro's sheets use. When the document has
  none, the command first creates it in the document as `_DEFAULT.STY`
  defines it, in the document's Body Text font (a document style, as
  Create Style makes one: shown with Ami Pro's • in the Styles panel), and
  applies it; one undo step for both. **Off** (every selected paragraph
  already shows one): they return to Body Text. A numbered paragraph
  given Bullets switches to the bullet style, and the other way round.
  The file stays plain Ami Pro: the paragraphs carry their style's name,
  as any styled paragraph does, and Ami Pro opens them as its own.
- **Enter in an empty list paragraph** (question 3) ends the list: the
  paragraph returns to Body Text instead of a new empty bullet, as modern
  word processors do. Elsewhere Enter keeps Ami Pro's Next style.
- **Insert › Bullet…** (question 4), Ami Pro's Edit › Insert › Bullet…:
  a small one-shot dialog with the 17 bullets drawn as buttons in Ami
  Pro's order, an example in the caret's font, and Insert and Cancel; it
  remembers the last bullet; the bullet goes in at the caret (replacing a
  selection) as one undo step. Offered in Customize Toolbar, as Ami Pro's
  "Insert a bullet" SmartIcon (not in the Standard set).
- **Inserted bullets** become an inline object (model, native v3, the
  importer, both writers): drawn as the bullet in the font where it
  stands, one character to the arrows, Delete, Backspace, selection, and
  copy; written back as `<:B<*X>>`.
- **The display gaps**: Right align as Ami Pro draws it; superscript
  bullet text raised as Ami Pro raises it; the seven picture bullets drawn
  as shapes of the app's own that read as Ami Pro's (a pointing hand, a
  box with a shadow, a box with a thick top, a checked box, a crossed box,
  two arrowheads); Ami Pro's pictures are not copied.
- **Style Properties › Bullets and numbers** gains **Space for**, the First
  indent under Ami Pro's name (the same value as in Alignment), so the room
  for a bullet is set where the bullet is chosen.
- **Twins:** BL1 (inserted bullets) and list styles applied and created in
  the app, written as Ami Pro writes them and opened in Ami Pro as its own;
  Right align, superscript, and the pictures compared with Ami Pro's
  screens.
- **Not proposed:** restarting or continuing numbering by hand (Ami Pro
  stores no such thing per paragraph; any other paragraph restarts a
  list); a bullet picker on the toolbar button (a style's bullet changes in
  Style Properties, for every paragraph in it); Tab and Shift+Tab to change
  a list's level, outline numbering, and Outline Styles (Phase 5); the
  text-editor build (it does not draw bullets).

### Questions for the user (ranked)

1. What should Bullets and Numbering do? Recommended: apply Ami Pro's list
   styles (Bullet 1 or Bullet, Number List), created in the document from
   `_DEFAULT.STY` when the sheet has none; off returns to Body Text. The
   alternatives: a list version of each paragraph's own style ("Note" →
   "Note Bullet", off back to "Note"), or bullets inserted into each
   paragraph's text with a tab (bullets only; numbering would still need a
   style).
2. Put Bullets and Numbering on the Standard toolbar, with icons of the
   app's own (the COSMIC theme has none), and give Numbering Ctrl+Shift+N?
   Recommended: yes.
3. Should Enter in an empty bullet or number paragraph end the list?
   Recommended: yes, as modern word processors do; Ami Pro makes another
   empty bullet.
4. Insert › Bullet… in this run? Recommended: yes, as Ami Pro's small
   dialog with the bullets drawn; the alternatives are a submenu (the seven
   pictures cannot be drawn in a menu) or only drawing imported bullets.

### Answers (user, 2026-10-17)

The user first asked whether there should be style bullets and numbers
"but also manual": there are both, style bullets and numbers and manual
bullets (Insert › Bullet…); Ami Pro's format has no manual number and no
bullet on a paragraph apart from its style, so the question was only what
the toggles do. The user also asked whether this fits the move of Ami
Pro's large dialogs into side panels: it does (Modify Style's Bullets &
Numbers is already Style Properties' section; the toggles need no dialog;
Insert › Bullet… is a small one-shot dialog), and the user took the
proposed link to the panel. Recorded in `DECISIONS.md` as B1–B5:

- **B1:** the toggles apply Ami Pro's list styles (Bullet 1, else Bullet;
  Number List), created in the document from `_DEFAULT.STY` in the
  document's Body Text font when it has none; off returns to Body Text.
- **B2:** Bullets and Numbering on the Standard toolbar as toggles with
  the app's own icons, after the alignment group; Numbering Ctrl+Shift+N.
- **B3:** Enter in an empty bullet or number paragraph ends the list.
- **B4:** Insert › Bullet… in this run, as a small dialog with the 17
  bullets drawn; also in Customize Toolbar.
- **B5:** Format › Bullet and Number Style… (also in the text right-click
  menu) opens Style Properties at its Bullets and numbers section for the
  caret paragraph's style.

### Implementation notes (2026-10-17, the bullets and numbering run)

By Ami Pro's rules from the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`
(`docs/research/BULLETS_NOTES.md`), twinned in the VM (BL1, BL2, BL4: the
app's twins write Ami Pro's files and Ami Pro shows them as its own, pixel
for pixel).

- **Format › Bullets (Ctrl+Shift+L) and Format › Numbering
  (Ctrl+Shift+N)**: toggles with check marks, on the Standard toolbar after
  the alignments (icons of the app's own, recoloured as the theme's
  symbolic icons) and in the text right-click menu. Checked when every
  selected paragraph's style shows that kind (a number code: Numbering;
  else one of Ami Pro's bullets: Bullets). One `SetList` command:
  `amipro_editing::lists`; the status bar names the style applied. A frame
  selection disables them.
- **The list styles**: Bullet 1, else Bullet, for bullets; Number List for
  numbers; only a style that shows the kind counts. A made style is
  `_DEFAULT.STY`'s in the document's Body Text font, spacing, and lines,
  under the first free name of Bullet 1, Bullet, List Bullet (Number List,
  Numbered List, List Number), with the next free function key and
  `[spec]` value 4 as 0, as Ami Pro's Create Style makes a style (BL4);
  Create Style itself now gives the next free function key too.
- **Enter** in an empty paragraph that shows a list turns it into Body
  Text instead of making a new paragraph.
- **Insert › Bullet…**: a small dialog of the 17 bullets in rows of six,
  each drawn by the page's own shapes, with its name as a tooltip, and a
  large example with the name; Insert puts the bullet over the selection
  or at the caret in one undo step; the last bullet is kept in the
  settings; Customize Toolbar offers it.
- **Format › Bullet and Number Style…** (also in the text right-click
  menu): Style Properties for the caret paragraph's style with only
  Bullets and numbers unfolded. **Space for** ends that section: the same
  value as Alignment's First, as in Ami Pro's dialog.
- **Inserted bullets** are `InlineObject::Bullet`: native v3 `inline
  bullet CODE`, `<:B<*X>>` read and written, laid out as the style bullet
  with that code in the font where it stands, drawn in that text's colour;
  the caret and clicks take one as one character; copied, it is its
  nearest Unicode character (the clipboard is plain text).
- **Drawing**: Right align ends the bullet text one space (in its font)
  before the first line's text (BL2); superscript bullet text is 7/12 the
  size, raised 5/12 (BL3); the seven pictures are the app's own shapes;
  every bullet is drawn as shapes, so the headless renderer and the
  dialog show the pictures too.
- **Also fixed**: after a selection, a click left the hidden text editor's
  selection in place, so the style field read the old range ("Mixed").
- **Not built**, as proposed: restarting or continuing numbering by hand,
  a bullet picker on the toolbar button, Tab and Shift+Tab levels,
  outline numbering (Phase 5).

## 22. Outline levels and multi-level numbering — approved 2026-10-17 (O1–O4)

The user asked how a numbered list moves between levels (1, 2, 3, then
A, B, C, then i, ii, iii). The app counts outline levels but has no
command to change a paragraph's level, no way to set a style's level, no
Quick Numbering, and no "1.1.1" numbers. By Ami Pro's rules from the
decompiled `AMIPRO.EXE` and `AMIPROUI.DLL` and VM fixtures OL1 and OL2
(`docs/research/OUTLINE_NOTES.md`). Outline mode itself stays in Phase 5.

### What Ami Pro 3.1 does

Each level is a style with an outline level 1–9, counted by level whatever
the style; Style › Outline Styles… sets levels, numbers, separators, and
three Reset options (After higher level, After intervening style,
Cumulative numbering), with Quick Numbering's two schemes; Outline mode's
Promote and Demote (Alt+Left, Alt+Right) give a paragraph the style one
level up or down, making "Outline#" (a copy at that level) when none
exists.

### Proposal and answers (user, 2026-10-17, asked together)

- **O1, the keys:** Format › Promote (Alt+Left) and Demote (Alt+Right) in
  the normal page view, on the caret's or the selected paragraphs; Tab
  stays a tab, as in Ami Pro. With a frame selected Alt+arrows keep
  resizing it (§11).
- **O2, nested numbered lists:** demoting an item of a list made with the
  Numbering button turns that list into a multi-level outline: depth 1
  counts 1., depth 2 A., depth 3 i., then a., 1), a), i), (1), (a); each
  depth's style ("List 1", "List 2", …) is made in the document as
  needed, indented a further 0.25 in per depth, with Right align and After
  higher level, so deeper levels restart under each item. Because Ami Pro
  counts by level whatever the style, the list's depths take the outline
  levels below the deepest level the document's styles already use (in
  `_DEFAULT.STY`, Title 1 and Subhead 2, so a list starts at level 3):
  the top of a list then restarts after a heading with an outline level,
  Ami Pro's limit, and headings' counts are untouched. Other paragraphs
  (headings, levelled styles, None) promote and demote by Ami Pro's rule,
  making Outline# when no style exists at the level.
- **O3, where levels are set:** Style Properties' Bullets and numbers
  gains Outline level (None, 1–9), the Reset options (After higher level
  or After intervening style, and Cumulative numbering, which builds the
  "1.1.1" text from each level's number and separator), and a small
  Format › Styles ▸ Outline Numbering… dialog applies a whole scheme in
  one click: 1. A. i. (O2's), Ami Pro's I. A. 1. i. a., Ami Pro's 1, 1.1,
  1.1.1, or Clear, to the document's levelled styles. (Ami Pro's buttons
  also clear the numbers of styles without a level, Number List's
  included, OL2; the app leaves those styles alone.)
- **O4, the toolbar:** Promote and Demote on the Standard toolbar beside
  Bullets and Numbering (the theme's indent-less and indent-more icons).
  The bar then needs about 1,000 px; in narrower windows its last group
  (Draw Frame) moves to More.
- **The counting rules** follow `1200:1842`: After higher level, After
  intervening style, or neither (no restart), cumulative codes counting
  levels 1, 2, 3…, and the hidden full stop after the last level's code.
- **The Format menu** (the user: "The format menu is getting very tall"):
  folded into submenus as Ami Pro's Text menu nests Alignment ▸ and Caps ▸:
  Alignment ▸ (Left, Center, Right, Justify), Bullets and Numbering ▸
  (Bullets, Numbering, Promote, Demote, Bullet and Number Style…), and
  Frame ▸ (Frame Properties, Bring to Front, Send to Back, Group, Ungroup);
  shortcuts, check marks, and toolbar buttons unchanged.
- **Not proposed:** Outline mode (collapse, expand, level icons, moving
  sections; Phase 5), restarting a list's top level by hand (Ami Pro
  stores none).
- **Bullets nest** (the user, 2026-10-18, after the run): a bulleted
  line without an outline level moves one depth, "Bullet 1" (or the
  sheet's Bullet, or another bullet style) → "Bullet 2" → … "Bullet 9",
  each made as needed from its neighbour 0.25 in further in, the bullet
  going small diamond, dot, small square and round again; the top does not
  promote. Bullets count nothing, so these styles take no outline level.
  Ami Pro shows the app's nested bullets as its own (VM, AOL4).

### Implementation notes (2026-10-18, the outline levels run)

By Ami Pro's rules from the decompiled `AMIPRO.EXE` (`1200:1842`,
`1200:1384`, `1200:1489`) and `AMIPROUI.DLL` (`12c8:060c`, Quick
Numbering's `12c8:06d6`, `12c8:0829`), twinned in the VM (OL1, OL2: the
app writes Ami Pro's bodies and changed `[tag]` records; Ami Pro shows the
app's files, and a nested list made in the app, as its own).

- **Counting** (`amipro_layout::paragraph_numbers`, `ParagraphNumber {
  own, levels }`): a level counts every paragraph at that level whatever
  its style, and restarts after a higher level (bit 2), after any other
  level or None (bit 4), or never, by the flags of the paragraph's style
  at its level, else of the first style at that level. Cumulative codes
  (bit 8) show levels 1, 2, 3…'s counts; a full stop right after the last
  level's code is not drawn. Level 0 counts a style's unbroken run, as
  before. Ami Pro's sheets and every compared page are drawn as before.
- **Format › Bullets and Numbering ▸ Promote (Alt+Left) and Demote
  (Alt+Right)**, also on the Standard toolbar after Bullets and Numbering
  (the theme's `format-indent-less-symbolic` and
  `format-indent-more-symbolic`) and in the text right-click menu's
  Bullets and Numbering ▸: one `ChangeLevel` command over the caret's or
  the selected paragraphs, one undo step; the status bar names the style
  and level given, or says the paragraph is already at the top or lowest
  level. A selected frame's Alt+arrows still resize it, in the widget and
  through the shortcut.
- **Nested lists** (`amipro_editing::outline`): Demote on an item of a run
  of a numbered list style without a level (Number List) turns the run
  into List 1 and the demoted items into List 2; List n demotes to List
  n+1 and promotes to List n−1; List 1 does not promote. List n is made
  as needed from the list above it: 1. A. i. a. 1) a) i) (1) (a) by depth,
  0.25 in further in, Right align and After higher level, outline level n
  plus the deepest level of the document's other styles (List 1 is level
  3 under `_DEFAULT.STY`'s Title and Subhead). Other paragraphs move by
  Ami Pro's rule, making Outline# (a copy of the style at that level, with
  the next free function key; Outline3 has F12 in OL1) when no style is at
  the level, preferring a used style when several are. Bullets move by
  that rule too (the open question).
- **Style Properties › Bullets and numbers** gains Outline level (None,
  1–9) and, with a level, the Reset options After higher level and After
  intervening style (exclusive) and Cumulative numbering, which builds the
  text from the number of each level above (`I.1.` under `I.`) and takes
  it off again.
- **Format › Styles ▸ Outline Numbering…** (after Modify Style, where Ami
  Pro has Outline Styles…): a small dialog of four schemes (1. A. i. a.;
  Ami Pro's I. A. 1. i. a.; Ami Pro's 1, 1.1, 1.1.1; No numbers), the
  levelled styles it will number, and Apply (disabled without levelled
  styles) as one undo step of `ModifyStyle`s. Each style keeps its text
  before the number and its bullet attributes; styles without a level are
  left alone.
- **The Format menu** is folded as proposed; the text right-click menu
  uses the same Bullets and Numbering ▸.
- **The toolbar, corrected:** with Promote and Demote the Standard bar
  needs about 1,000 px with the wide style field and 971 px with the
  compact one; below 971 px its last two groups (the lists and Draw Frame)
  move to More, because More needs room of its own (O4 said only Draw
  Frame would).
- **Timings** (README31, release, update to drawn frame): Demote 2.8 ms,
  Promote 4.7 ms, Outline Numbering's Apply 20.6 ms and its undo 12.8 ms
  (every page relaid, as any change to used styles does).
- **Not built**, as proposed: Outline mode, restarting a list's top by
  hand, nesting bullets (waiting for the user).

## 23. Choosing colours — approved 2026-10-18 (C1–C4)

The user, 2026-10-18: colour selection should be "more like modern color
selectors. Easier to choose wide variety of colors, but maintain your
current pallet so you don't have to keep finding the colors you are
using."

### Where colours are chosen today

| Place | Colour | Control |
|---|---|---|
| Text › Font… dialog | text | Ami Pro's 15 colours in a row, plus the text's colour when it is not one of them |
| Style Properties › Font | the style's text | the same row |
| Style Properties › Lines | paragraph rules | the same row |
| Page Layout Properties › Lines | the page's lines and the line between columns (one colour, as Ami Pro's Lines panel) | the same row |
| Frame Properties | line, shadow, background | `#RRGGBB` text fields (Ami Pro's Modify Frame Layout › Lines & shadows has a colour bar for each: dialog 1036, controls 25590, 25593, 25591) |
| Frame Properties › Columns | line between columns | none: it takes the frame's line colour when turned on (Ami Pro's Columns & tabs panel has its own colour bar, 25592) |
| (built since: Table Properties, `table_properties.rs`) Table › Lines & Color | table lines and shading | Ami Pro's dialog 3052; waits for table formatting |

### Ami Pro (VM, 2026-10-18; `AMIUICNT.DLL`'s COLORBAR, COLORPALETTE)

The colour bar is the 15 colours with a ▼ at its end; ▼ drops a grid of 120
fixed colours (13 hues in 8 tints and shades, two grey columns). There is
no way to mix a colour. Files store any colour (`COLORREF`, 24-bit RGB),
so a mixed colour is written and read back as it is.

### Design (C1–C4, `DECISIONS.md` 2026-10-18)

One colour control, used in every place above:

- **A colour button** showing the current colour (and its name or
  `#RRGGBB`), the same size everywhere, which opens a **colour panel**
  in place (below it in the rail, in the dialog), non-modal, closed by
  choosing or by the button again:
  - **Ami Pro's 15 colours**, the row they are now, so a new document
    has quick colours.
  - **In this document**: the colours the document already uses, every
    kind together (text, styles, rules, frames, lines), most used first,
    so a heading's colour is offered for a frame line (C2).
  - **Recent**: the last colours chosen anywhere in the app, one list
    (C4), kept in the settings across sessions, newest first, colours
    already shown above left out.
  - **More colours**: libcosmic's colour picker (a saturation and value
    field, a hue slider, `#RRGGBB` or RGB entry), as COSMIC's own
    settings use it. Ami Pro's 120-colour grid is not offered (C2).
- A colour chosen in the rail applies at once as one undo step; dragging
  in the picker shows the colour live and leaves one step when it ends
  (as a frame drag does). In the Font dialog it applies with OK.
- All four frame colours use it, as in Ami Pro's Modify Frame Layout:
  Frame Properties' three `#RRGGBB` fields (line, shadow, background)
  become colour buttons (the hex is typed in the picker), and the frame's
  line between columns gets a colour button of its own (the user,
  2026-10-18: "there are a number of color options around frames").

Mockup (the rail, the panel open):

```
Colour  [■ Red        ▾]
┌──────────────────────────────┐
│ Ami Pro  ■■■■■■■■■■■■■■■     │
│ Recent   ■■■■■               │
│ In use   ■■■                 │
│ More colours ▸               │
│  ┌────────────────┐  #RRGGBB │
│  │  sat × value   │  [______]│
│  └────────────────┘          │
│  hue ═══════●══════          │
└──────────────────────────────┘
```

### Implementation notes (2026-10-18, the colour selection run)

- `colours.rs` (the model) and `colour_control.rs` (the button and panel).
  The panel's rows wrap in the rail: Ami Pro's 15 take two rows at 300 px.
  In this document weighs each style's colour by the paragraphs in it and
  leaves out what does not show (a transparent frame's background, no
  shadow, lines that are off); it is worked out when a panel opens and
  after each colour applies, not per frame. Recent keeps two rows' worth
  and shows one, leaving out colours in the rows above.
- libcosmic's picker sends its messages through a plain `fn`, so the tab
  keeps which colour the open panel is for (`Panel::target`); after each
  finished choice the panel takes a fresh picker, because the picker keeps
  its own recent row, which would repeat the panel's.
- A drag (C3): the tab's `colour_drag` holds the undo depth the drag's
  first change left; while it matches, the style, page layout, and frame
  paths amend that step (`continues_colour_drag`). One undo restores the
  colour from before the drag.
- Frame Properties shows its four colours as buttons; the line between
  columns has its own (`Field::ColumnLineColour`). The page's lines keep
  one colour for the border and the line between columns, as Ami Pro's
  Lines panel and its file do (`[lay]` value 11).
- A panel closes when its host closes (the Font dialog, either rail
  panel, or Frame Properties on one frame).
- **Found and fixed:** a frame's line between columns in a style other
  than Ami Pro's frame default made Save As list the frame: the importer
  kept `[frmlay]` value 10 as a raw value though the writer writes it while
  the line is on.
- Ami Pro opens the app's mixed colours and draws each exactly (VM ACS1:
  text, a rule, a frame's four colours, the page's lines).

## 24. Frames in table cells — approved 2026-10-18 (T1–T3)

A table cell holds text or one picture (a drawing, chart, or equation)
that fills it (user's guide, chapter 16). README31.SAM's SmartIcon table
is one: three pictures in the cells of a table on page 12. Both corpora
have 108 such pictures in 23 files. Evidence: `docs/research/CELL_FRAME_NOTES.md`
(decompile and VM fixtures TF1–TF15).

### Ami Pro

- The picture's frame covers the cell's text area and goes wherever the
  cell goes: when the table moves on the page or to another page, and
  when its row grows (TF2, TF10, TF3; a joined cell's frame spans it,
  TF13).
- A click on it puts the caret in the cell and outlines the cell (43-twip
  inverted bands); no frame is selected, and Modify Frame Layout, Group,
  and Bring to Front/Send to Back are disabled (TF5).
- Del deletes the picture, with no question and no undo; Backspace and
  typing do nothing (TF7, TF12). Import Picture (also Drawing, Charting,
  Equations, Paste) with the caret in an empty cell puts the picture in
  the cell; it is disabled in a cell with text. Create Frame always makes
  a page frame (TF15).

### The app today

These pictures are imported as Where placed frames at their stored places.
A table moved by editing leaves them behind; the main text wraps around
them; they can be dragged, resized, grouped, restacked, and deleted like
page frames, and restacking or deleting one breaks the cell numbers Ami
Pro reads them by. Cell text is also placed by a simpler rule than Ami
Pro's (half the gutter on each side of each cell), which only agrees with
it at the default gutter: 161 of the 603 corpus tables have other gutters.

### Design

- **The frame belongs to its cell.** It is laid out over the cell's text
  area wherever the table is laid out (any page, rows that grow with their
  text, joined cells), drawn above the table's cells and below Where
  placed frames, and the main text does not wrap around it (the table
  already holds its place). Deleting the table deletes its pictures; Undo
  restores both.
- **Tables are laid out on Ami Pro's grid:** the table's insets (43 and
  43 in nearly every table) and each row's and column's gutter, the first
  column 43 in whatever the gutter (`CELL_FRAME_NOTES.md`, "Where the frame
  goes"). Text moves in tables with gutters other than 86; nothing changes
  in the others.
- **Clicking and editing (T1).** A click on a cell's picture puts the
  caret in the cell, as Ami Pro does, and the cell is outlined as Ami Pro
  outlines it (a 43-twip band around the text area, in the accent colour);
  no handles; a drag does nothing to it. The rail shows Styles as for any
  cell, and Format › Frame ▸ and Select Next Frame leave it out. **Delete**
  removes the picture as one undo step (Ami Pro cannot undo it). Backspace
  does nothing, as in Ami Pro; typing does nothing, and the status bar
  says "This cell holds a picture; Delete removes it."
- **Pictures Ami Pro never sized (T2).** Ten pictures in two of the
  user's files were stored with no height (never laid out); Ami Pro shows
  nothing for them until their row changes, and opens any stored size it
  finds (TF1Y, TF1Z). The app lays every cell's picture out at its cell's
  size, and writes it so.
- **Putting a picture in a cell (T3)** comes with pictures (Insert ›
  Image…, a later run): with the caret in an empty cell the picture goes
  into the cell, as Ami Pro's Import Picture does. Insert › Frame… and Draw
  Frame keep making page frames, as Ami Pro's Create Frame does.
- **Saving.** Save As SAM writes these frames first, in Ami Pro's order
  (the newest first), each cell naming its frame by that order, with the
  frame's size from its cell; the page and corner are written as read
  while the table is unchanged (Ami Pro takes them from the cell when it
  opens the file, TF1Y). The native format records which cell holds which
  frame. Nothing about them is in the loss check.

Mockup (the caret in cell (0,1), which holds a picture):

```
┌──────────────┬──────────────┬──────────────┐
│ One          │▓▓▓▓▓▓▓▓▓▓▓▓▓▓│              │
│ Two          │▓┌──────────┐▓│              │
│ Three        │▓│ picture  │▓│              │
│              │▓└──────────┘▓│              │
├──────────────┼──────────────┼──────────────┤
│              │              │              │
└──────────────┴──────────────┴──────────────┘
Status: This cell holds a picture; Delete removes it.
```

### Questions (answered 2026-10-18 in `DECISIONS.md`: the three recommendations)

- **T1:** clicking and editing as above (recommended), or select the
  picture like a frame (handles and Frame Properties for its look, kept in
  its cell).
- **T2:** lay never-sized pictures out at their cell's size and write them
  so (recommended), or keep Ami Pro's stored size (they stay invisible).
- **T3:** pictures go into cells with Insert › Image… in the pictures run
  (recommended), or now: Draw Frame or Insert › Frame… in an empty cell
  puts an empty frame in it, which Ami Pro does not do.

### Implementation notes (2026-10-18, the frames in table cells run)

- **Tables on Ami Pro's grid** (CF.2): `TableSpacing` on `Table` holds the
  insets (`[tbl]` header values 8 and 9) and each row's and column's own
  gutter (`[h]`/`[w]` value 3 with flag 4); the engine's `TableGrid` puts
  column c's text at the left inset plus each earlier width and gutter, and
  a cell's box to the middle of the gutters beside it (the first from the
  table's edge). Identical to the old geometry at 86 and 43; a table's
  natural height now equals Ami Pro's stored one in 159 of 389 user tables
  (58 before).
- **The model** (CF.3): `Placement::InCell`, `TableCell::frame`; the frame
  stays in the frame stack (Ami Pro keeps it with the page frames, in a
  list of its own), named by exactly one uncovered cell, never grouped or a
  table. Deleting a table deletes its pictures; a frame in a cell is not
  moved, resized, restacked, or placed elsewhere (`IN_CELL`).
- **Files** (CF.4): the importer links a cell whose record's values 11 and
  12 name the n-th `0x20` frame in file order; one no cell names stays
  Where placed, first in the file. The writer writes Ami Pro's list first
  (the newest first), each cell numbering its frame from it, the frame at
  its cell's size (T2), its page and corner as read while its size holds
  (Ami Pro recomputes both on opening, TF1Y). A cell whose picture is
  deleted keeps its record (`… 0 0`, as TF7). The round-trip facts treat a
  cell frame's stored rectangle as a cache and a cell record as written.
- **Layout** (CF.5): the table's layout returns a box for each cell's frame
  over the cell's text area, put right after the table's box; rows grown
  by text grow the picture (TF3); no text wraps around it.
- **The page** (CF.6): frames in cells are not selectable; a press passes
  through to the cell; `page_editing::edit_command` turns Delete in a
  picture's cell into `DeleteFrame` and ignores typing, Enter, and
  Backspace there (paste goes the same way); Edit › Delete is enabled
  there; the outline is `cell_outline` (four 43-twip bands in the accent
  colour); the status note is `cell_frame_note`.
- **Not built when written; pictures are built since (§25) and go into cells:** putting a picture into a cell (T3: Insert › Image…, the
  pictures run), moving pictures between cells (Cut, Copy, Paste, and Drag
  & Drop of cells, with table editing), pictures drawn (placeholders until
  picture decoding), rows of a table that continues on later pages (tables
  are not split yet). The app writes the heights of rows grown by typing
  as read; Ami Pro grows them and fits their pictures on opening (ATF3).

## 25. Pictures — approved 2026-10-18 (P1–P8)

Every picture is a grey "Picture" box today, and Insert › Image… is
Future. This section proposes how pictures are drawn, inserted, sized,
and moved in their frames, by Ami Pro's rules from the decompiled
`AMIPRO.EXE`, `AMIPROUI.DLL`, `AMIPAINT.DLL`, and `AMIWMF.DLL` and the VM
fixtures PI1–PI9, PIR1–PIR5 (`docs/research/PICTURE_NOTES.md`).

### What Ami Pro 3.1 does (guide ch. 15, 19; PI1–PI9)

- **File › Import Picture…** lists 13 file types and has **Copy image**
  (on). With an empty frame selected the picture goes into it; with the
  caret in an empty table cell, into the cell; with the caret in text, into
  a new **Flow with text** frame at the caret, of the remembered Create
  Frame size (1.00 × 1.00 in at first, whatever the picture's size), in
  the default frame's style. It is disabled with a picture frame selected
  (no replace: delete the picture first) and in a cell with text.
- **Converted at import:** PCX and TIFF become Windows bitmaps, CGM and
  HPGL metafiles, DrawPerfect, Freelance, and 1-2-3 PIC drawings. A
  document holds only bitmaps, metafiles, drawings, charts, equations, and
  OLE objects.
- **Fit in frame, aspect ratio kept** after import. **Frame › Graphics
  Scaling…** chooses Original size, Fit in frame, Percentage, or Custom
  (W × H), Maintain aspect ratio, and Rotate (bitmaps and drawings). The
  picture is drawn from the frame's top-left inner corner, stretched to
  its size, and clipped to the frame; Fit in frame follows the frame when
  it is resized.
- **Moving a picture in its frame** (cropping): double-click the frame (or
  Enter), then drag, or arrows (0.05 in) and Ctrl+arrows (0.5 in); Esc
  returns. Shift and a corner handle size the frame and the picture
  together (Custom).
- Ami Pro stores its own low-resolution screen image of each picture
  beside it, a cache it rebuilds when missing.

### What the app has

Picture frames import, round-trip byte for byte, place, wrap, and follow
table cells. No decoder, no command, and Enter on a picture frame opens
Frame Properties.

### Proposal

**Drawing.** A picture frame is drawn as any frame (background, lines,
shadow, corners) with its picture inside, at the size its scaling mode
gives, moved by its pan, clipped to the frame's inner area:
- **Bitmaps** (281 + 18 in the corpora) decoded once and cached;
  **metafiles** (230 + 13) by the app's own metafile interpreter, redrawn
  sharp at each zoom; inserted PNG, JPEG, GIF, TIFF, PCX, and SVG pictures
  likewise (SVG redrawn sharp); since the import run also enhanced
  metafiles (EMF, Word's and LibreOffice's drawings), played by the app's
  own reader into the same display list as metafiles, inserted or
  imported, written as bitmaps by Save As SAM and as EMF by export.
- **Kinds the app cannot draw yet** (drawings, charts, OLE objects, until
  their runs): Ami Pro's stored screen image, else a labelled box (P4).
- A **linked picture** is found by its file name in the document's folder
  (any case), else the stored rendering, else a box with its path.
- Decoding stays off the typing path: decode on first draw, cache per
  picture and zoom, draw scaled while a redraw is pending.

**Insert › Image…** (Ami Pro's File › Import Picture; `Ctrl+Shift+I`,
free) opens the desktop's file chooser filtered to pictures
(P1). The picture goes where Ami Pro puts it: into a selected
empty frame; into the caret's empty table cell (T3); otherwise into a new
Flow with text frame at the caret (the picture's size, P3), Fit in frame
with the aspect ratio kept, the frame selected, one undo step. Copied into
the document, or linked when the chooser's **Link to file** is checked
(P2): the native format keeps the path and draws the file as it is now;
Save As SAM embeds a copy (Ami Pro cannot follow a Linux path) and lists
it. Disabled in a cell with text; with a picture frame selected it is
Replace Picture… (below).

**Frame Properties › Picture** (the rail; Format › Frame ▸ Graphics
Scaling… opens it there; P5):

```
 ▾ Picture
   Windows Bitmap · 120 × 80 px · 1.25 × 0.83 in at 96 dpi
   Sizing    (•) Fit in frame   ( ) Original size
             ( ) Percentage [ 50 %]   ( ) Custom
   Width  [0.57 in]   Height [0.38 in]   [x] Keep aspect ratio
   Rotate [  0°]                      (bitmaps and drawings)
   Moved in frame  →[0.00 in] ↓[0.00 in]           [Reset]
   [Replace Picture…]  [Fit Frame to Picture]  [Save Picture As…]
   Description [________________________]   (not kept in .SAM)
```

Each change is one command and one undo step (`SetPictureScaling`,
`MovePictureInFrame`); typing W or H selects Custom, as Ami Pro's dialog;
limits as Ami Pro's (0.01 to 22 in, 1–999 %).

**Moving a picture in its frame** (P6): double-click a picture
frame or press Enter on it: the handles turn grey, the pointer a hand, and
the status bar says "Drag the picture or use the arrow keys; Esc
returns". A drag moves the picture (one undo step), arrows 0.05 in,
Ctrl+arrows 0.5 in (a run of presses one step); Esc or a click elsewhere
returns. Shift and a corner handle size the frame and the picture
together, as Ami Pro's.

**Additions** (P7): paste an image from the clipboard as a
picture (in place of Ami Pro's Paste of a bitmap), drop a picture file on
the page, Replace Picture… (keeps the frame, scaling, and placement),
Fit Frame to Picture, Reset (scaling and position), Save Picture As…, and
a description (alt text).

**Ami Pro files.** Save As writes pictures as Ami Pro's records with no
driver state and no screen image, which Ami Pro rebuilds (PIR1, PIR3).
Bitmaps and metafiles are written as they are; other formats as Windows
bitmaps (P8). Twins: PI1, PI2, PI5–PI8, a picture in a frame and
in a cell, made in the app and opened in Ami Pro as its own.

**Not proposed:** Ami Pro's other import filters (CGM, HPGL, DrawPerfect,
Freelance, PIC, AutoCAD, EPS), drawings, charts, equations, OLE editing,
Image Processing (their own runs), and the text-editor build.

### Decisions (2026-10-18; `docs/research/DECISIONS.md`)

1. **P1 Formats:** BMP and WMF, plus PNG, JPEG, GIF, TIFF, PCX, and SVG;
   the native format keeps each file as inserted.
2. **P2 Copy or link:** copied by default, with a **Link to file** choice
   in the file chooser (not the recommendation, which was always copy).
   Linked pictures, old or new, are drawn from their file when it is
   found (the path, then the name in the document's folder in any case),
   else from Ami Pro's stored screen image, else a box naming the path.
3. **P3 A new picture's frame:** the picture's size at its own
   resolution, reduced to fit the column and the page.
4. **P4 Drawings, charts, OLE objects:** Ami Pro's stored screen image
   where there is one, else a labelled box, until their runs.
5. **P5 Graphics Scaling** in Frame Properties › Picture (the rail), with
   Rotate as Ami Pro's.
6. **P6 Moving a picture in its frame** as Ami Pro, plus Shift and a
   corner handle sizing frame and picture together.
7. **P7 Additions:** all: paste and drop, Replace Picture… and Save
   Picture As…, Fit Frame to Picture and Reset, a description.
8. **P8 Ami Pro files:** other formats written as Windows bitmaps,
   reduced above 300 dpi at their size on the page, each conversion listed
   by Save As.

### Implementation notes (2026-10-18, the pictures run)

- **Model** (PI.2): `Picture` carries `PictureScaling` (sizing, Keep aspect
  ratio, percent, size, rotation), a pan (`Offset`, y downward), and a
  description; `PictureScaling::drawn_size` is Ami Pro's `1168:2dd2`. The
  `[isd]` values stay as a cache written back only while they still say
  the typed fields (`Picture::kept_is_current`). Commands: `InsertPicture`
  (an empty frame or cell), `ReplacePicture`, `SetPictureScaling`,
  `MovePictureInFrame`, `SetPictureDescription`; a new picture frame made
  by `CreateFrame` stores its picture area.
- **Files** (PI.3, PI.10): the SAM writer rebuilds a stale `[isd]` with the
  kept driver state (a plain metafile's only record of its size) and
  without Ami Pro's screen image; Save As converts other formats to 24-bit
  bitmaps at most 300 dpi at their size on the page, their resolution
  keeping Ami Pro's natural size, and copies in Linux links
  (`pictures::for_ami_pro`); the loss check names descriptions,
  conversions, and links. Native v3 `picturescaling`, `picturedescription`.
- **Drawing** (PI.4, PI.5): the core crate `amipro-pictures` reads Windows
  bitmaps, plays metafiles into a display list on the window's unit
  square, and reads Ami Pro's screen image (16-bit images as 5-5-5 when no
  pixel sets the top bit); the app decodes PNG, JPEG, GIF, TIFF (`image`),
  PCX (`image-extras`), and SVG (`resvg`), draws metafiles and SVG with
  tiny-skia at twice the shown size, and caches per session by the bytes'
  allocation or the linked file and its time. A picture frame is drawn as
  any frame, the picture between its background and its lines in a layer
  clipped to the frame's interior. Rotation turns the picture clockwise
  into its box, stretched onto its rectangle (PIRT30, PIRT90).
- **Insert › Image…** (PI.6, `Ctrl+Shift+I`): the portal's chooser with
  Link to file; `picture_insert::target` decides where (Replace with a
  picture frame selected; an empty text frame; an empty cell; else a new
  Flow with text frame at the caret in the main text, refused elsewhere
  with the reason); P3's frame is the picture's size inside Ami Pro's
  default frame, reduced to the caret's column.
- **Frame Properties › Picture** (PI.7): first in the rail for a picture
  frame; the mockup's fields, Reset, Replace Picture… (the chooser), Fit
  Frame to Picture, Save Picture As…, Description; Format › Frame ▸
  Graphics Scaling… and the frame's right-click menu open it. After the
  user's check (2026-10-18): a line under Sizing says what the sizing
  does, and **Percentage of its size** is always shown (typing one
  chooses Percentage; in the other sizings it says how large the picture
  is now), where Ami Pro shows it only for Percentage.
- **Picture mode** (PI.8): Enter or a double-click on a picture frame; a
  drag (the picture follows it), arrows 72 twips, Ctrl+arrows 720 (a run
  one step); grey handles, a hand; Esc returns. Shift and a corner handle
  size the picture with its frame (Custom).
- **Paste and drop** (PI.9): Paste with an image and no text inserts it
  (PNG kept); an image alone enables Paste; the page sits in libcosmic's
  `dnd_destination`, the drag forwarded as the pointer, and the drop
  moves the caret there before the file arrives. After the user's check
  (2026-10-18; paste and drop did nothing): Paste whose text is only a
  web address or a file path (`picture_insert::link_text`; Chromium's
  Copy Image puts the address beside the image) looks for an image, then
  copied picture files (`text/uri-list`, as COSMIC Files copies), before
  pasting the text; a drop takes an image a browser drags before files;
  the drop asks for a copy (libcosmic's default prefers a move).
  `AMIPRO_TRACE_DROP=FILE` writes what paste and drop receive, with
  libcosmic's drop and clipboard traces. The recheck's trace found the
  real cause: Paste's state came from a look at the clipboard as the
  window gained focus, which can run before the clipboard's worker knows
  of the focus and find nothing, and a disabled Paste dropped Ctrl+V.
  Paste now always reads the clipboard (the status bar says when there
  is nothing); the right button looks again before its menu opens, and a
  first look that finds nothing is tried once more 150 ms later.
- **Found and fixed:** text beside a Flow with text frame was drawn
  centred in its tall line (COSMIC Text centres), now on the baseline;
  Insert Layout copied a repeating table's cells without their pictures.
- **Not built:** Ami Pro's other import filters (CGM, HPGL, DrawPerfect,
  Freelance, PIC, AutoCAD, EPS). (Drawings, charts, equations, and OLE
  objects are built since: §33, §42.) eight-bit screen
  images (the writing screen's palette is not kept), the picture area's
  exact clamp (`1530:0892` mode 2; the frame's interior is used), Insert ›
  Image… with the caret in a header, footer, or frame's text.

## 26. Tables across pages — taken overnight 2026-10-18 (TP1–TP5), confirmed by the user 2026-10-19

A table that does not fit in the rest of its column keeps one page's part
today, and its later rows run over the text after it. This section says
how tables continue across pages and columns, by Ami Pro's rules from the
VM fixtures TP1–TP11 and the decompiled `AMIPRO.EXE`
(`docs/research/TABLE_PAGES_NOTES.md`). It adds no control: the run is
layout, plus Insert › Page Break in a table cell. Its decisions were taken
overnight by the rulebook (`docs/SESSION_HANDOFF.md`), each Ami Pro's
behaviour and cheap to change.

### What Ami Pro 3.1 does

- A page table continues on the next column or page with **whole rows**: a
  row moves on when its text and its gutter do not fit; each part starts
  with the table's top inset. With **Rows span pages** (Modify Table
  Layout, with Automatic) a row splits between two lines instead, each
  cell at its own lines, with no grid line at the split.
- A row taller than a page is cut to the page without Rows span pages.
- **Headings** rows are drawn again at the top of every later part (the
  last heading before the part's rows), and the caret can go into them;
  such a part holds one inset less.
- **A page break in a table**: Page › Breaks › Insert page break with the
  caret in a cell moves its row to the next page; Remove page break with
  the caret in that row takes it away.
- When even the first row does not fit below its paragraph, the table goes
  to the next page with that paragraph.
- Without **Automatic** row height, rows keep their size and their text is
  cut. A table in a frame shows the rows that fit the frame.

### In the app

- **Layout** as above, for anchored tables in the main text: each part a
  frame box on its page, cells of other parts left out, a heading's cells
  repeated, pictures in cells with their row's part, text after the table
  after its last part. Rows span pages, Automatic, and heading and
  page-break rows are read from files, kept in the model and native v3,
  and written back.
- **Editing:** the caret goes into cells on every page, the repeated
  heading included (it edits the heading row); a cell's text grows its row
  and the table repaginates, as typing in main text does.
- **Insert › Page Break with the caret in a table cell** (TP1): sets the
  row's page break, one undo step, the item then checked while the caret
  is in that row; choosing it again there removes the break. Outside a
  table it inserts a break as before.
- **Not in this run** (TP2): Table › Headings and Modify Table Layout's
  Rows span pages and Automatic, which belong to the table editing run
  (queue 8) and its decision on where table commands live.
- A row cut to the page (TP3) and rows hidden in a frame table (TP5) are
  Ami Pro's; with the caret in hidden text the status bar says so, as for
  header bands.

### Decisions (taken overnight 2026-10-18; `docs/research/DECISIONS.md`)

- **TP1** Insert › Page Break in a cell toggles the row's page break
  (checked while set). Alternative: Ami Pro's Breaks dialog with Insert
  and Remove.
- **TP2** Headings and Rows span pages are laid out and kept; their
  commands wait for the table editing run.
- **TP3** A row taller than a page without Rows span pages is cut to the
  page, as Ami Pro does.
- **TP4** The caret enters a repeated heading and edits the heading row,
  as in Ami Pro (TP2's page 2).
- **TP5** A table in a frame hides the rows the frame cannot hold, as Ami
  Pro does; frame tables never continue.

### Implementation notes (2026-10-19, the tables across pages run)

- **Model** (TP.2): `Table::row_options` (`TableRows`: Automatic, Rows span
  pages, heading rows, page-break rows; `heading_before`, `1468:044f`);
  native v3 `tablerows`; the SAM writer patches `[tbl]` value 7 bits `0x1`
  and `0x100` and `[h]` flags `0x10` and `0x20`, a new row record in Ami
  Pro's shape. `Command::SetTableRows` sets them as one undo step.
- **Layout** (TP.4, TP.5): `place_table` lays an anchored table out in
  parts, one frame box each (`FrameBoxKind::Table { part }`, `TablePart`:
  rows, heading, heights, the text shown before a continued row, a cut
  last row); whole rows that fit with their gutter, the top inset in each
  part, the heading and one more inset after the first, page breaks on
  rows, Rows span pages at lines, a row cut to an empty column, rows kept
  together over joined cells. Ami Pro's page tables are Flow with text
  frames on a line of their own (`lone_table`), so a line holding only a
  table takes the same path; its empty line stays where the table began.
  Rows keep their size without Automatic; a frame table shows the rows its
  frame holds; an empty cell needs no height. A cell edit never reuses a
  page that holds part of its table (`Convergence::tables`).
- **Page surface** (TP.7): each part draws its cells, those of other parts
  have no box, and a split row has no grid line at the split
  (`cell_edges`); clicks and the ruler find the part that shows a cell;
  the caret goes into a repeated heading on the page clicked (TP4); cell
  lines are not page copies for selections. Insert › Page Break in a cell
  is `page_editing::row_break_command`, checked by `row_has_break` (TP1).
- **Measures** (TP.8): `docs/research/LAYOUT_COMPARISON.md`, "Tables across
  pages"; twins ATP2, ATP5, ATP6 (`TABLE_PAGES_NOTES.md`).
- **Not built when written** (TP2): Table › Headings (built since, `CommandId::TableHeadings`), Modify Table Layout's Automatic
  and Rows span pages (not checked in this pass).

## 27. Find & Replace — proposed 2026-10-19; APPROVED by the user 2026-10-19 as recommended (F1–F4); built 2026-10-20

**Status: approved as proposed** (`DECISIONS.md`, 2026-10-19, "The
morning report's answers"): the find bar (F1), Ami Pro's defaults
remembered (F2), Ami Pro's capitals rule with a switch (F3), and all three
improvements without regular expressions (F4). Built in the queue's run 3
(`docs/FIND_PROGRESS.md`; implementation notes at the end of this
section).

Prepared overnight by the rulebook. Edit › Find…, Find Next,
Find Previous, and Replace… are registered (Ctrl+F, F3, Shift+F3, Ctrl+H)
and disabled. This section proposes how they work, from Ami Pro's four
dialogs and their rules (`docs/research/FIND_NOTES.md`: the decompiled
`AMIPRO.EXE` and `AMIPROUI.DLL`, the user's guide ch. 20, VM fixtures
FR1–FR7).

### What Ami Pro 3.1 does

- One dialog for both (Ctrl+F): Find, Replace with, then Find (the next
  match, with a small modeless dialog to step: Replace & Find Next, Find
  Next, Replace Remaining, Cancel) or Replace All. Options… and
  Attributes… open two more dialogs.
- Options: Whole word only (on), Exact case for finding and for replacing,
  Exact attributes for each, Beginning of document, Include other text
  streams (on), Find backwards, and Text or Style.
- Searches the text streams in Ami Pro's order (main text, Where placed
  frames, footnotes, anchored frames and page tables, floating headers and
  footers, headers, footers, and repeating frames), a selection alone if
  there is one.
- Without Replace Exact case the replacement takes the found text's
  capitals (all capitals, capitalized words, or lower case).
- `?` and `*` wildcards; Ctrl+Tab and Ctrl+Enter in a field for a tab and
  a return; Style replaces one paragraph style with another.
- Replace All cannot be undone; the status bar gives the counts.

### Proposal

```
 ┌ Find  [ status bar              ]  ▲ ▼   3 of 12      Options ▾   × ┐
 │ Replace [ window pane           ]  Replace   Replace All           │
 └────────────────────────────────────────────────────────────────────┘
   (a bar over the pages, under the ruler; the document stays live)
```

- **A find bar** over the pages (F1), opened by Edit › Find… (Ctrl+F) with
  the Find field focused, or Edit › Replace… (Ctrl+H) with the Replace row
  shown; a selected word fills Find. Enter and F3 find the next match,
  Shift+Enter and Shift+F3 the previous; Esc closes the bar and leaves the
  last match selected.
- **Matches**: every match on the visible pages highlighted, the current
  one selected and scrolled into view, its page instance kept for headers
  and repeated headings; "3 of 12" counts them in the streams searched
  (F4).
- **Options ▾** (a menu on the bar, checked items as everywhere): Whole
  word only, Match case, Keep the found text's capitals (Ami Pro's Replace
  Exact case off; F3), Attributes ▸ (Ami Pro's six, find and replace),
  From the beginning, Include headers, footers, and frames (Ami Pro's
  other text streams), Paragraph styles (Ami Pro's Style type: the fields
  become style pickers). Defaults Ami Pro's; remembered in the settings
  (F2).
- **Replace** replaces the current match and moves to the next; **Replace
  All** replaces every match in the streams searched as one undo step,
  the counts in the status bar (F4; Ami Pro cannot undo it).
- **Wildcards and codes** as Ami Pro's: `?`, `*`, `<*>`, `<?>`, a tab and a
  return typed with Ctrl+Tab and Ctrl+Enter in a field.
- **SAM**: nothing to store; replacements are ordinary edits.

### Questions (ranked; F5 optional)

- **F1 The form.** Recommend the find bar over the pages (the playbook's
  recommendation; non-modal like the rail inspectors, but searching needs
  the full width of the page in view). Alternatives: a Find & Replace
  panel in the context rail; Ami Pro's dialogs as they are.
- **F2 Defaults.** Recommend Ami Pro's (Whole word only and other text
  streams on), remembered between sessions. Alternative: modern defaults
  (whole words off), or Ami Pro's kept only for the session as Ami Pro
  does.
- **F3 Capitals when replacing.** Recommend Ami Pro's rule by default
  (the replacement follows the found text's capitals), with "Keep the
  found text's capitals" to turn it off. Alternative: the replacement as
  typed by default.
- **F4 Improvements.** Recommend all three: matches highlighted with a
  count, Replace All as one undo step, recent searches in the Find field's
  menu. Offered, not recommended now: regular expressions as an option
  beside the wildcards.
- **F5 (optional) Spelling later.** The bar's match walk is the base for
  spell check (queue 11): no question now.

### Implementation notes (2026-10-20, the Find & Replace run)

- **The search** (FD.2–FD.4): `amipro_editing::find`. `Pattern` reads Ami
  Pro's syntax: `?` one character, `*` one or more letters or digits
  (never none), `<?>` `<*>` `<»>` `<¶>` the characters themselves, and Ami
  Pro's marks » and ¶ for a tab and a return (the guide, ch. 20). `streams`
  lists every text stream in Ami Pro's order (main text; Where placed
  frames; With para above and Flow with text frames and page tables, by
  their anchors, a table's cells row by row; floating headers and footers;
  headers; footers; repeating frames); covered cells of joined blocks show
  nothing and are left out. `search_streams` takes the caret's stream and
  those after it, every stream with Beginning of document, the caret's
  alone without Include other text streams. `find_all` honours Whole word
  only (letters and digits make a word, FD2), Exact case, Exact
  attributes, a selection, and paragraph styles. `replacement` fills each
  wildcard of Replace with the one in the same place among Find's
  captures and applies Ami Pro's capitals rule (FD3); `replace_all` is one
  `Command::Several`, the last match first.
- **The bar** (FD.5): `find_bar.rs` holds the state per tab (`FindBar`:
  the texts, the Replace row, the stream the search started in, a
  selection it is limited to, the matches and the current one, found again
  only when the text, the options, the scope, or the document's generation
  change) and the menus' actions; `main.rs` the behaviour (`open_find`,
  `find_step`, `find_replace_one`, `find_replace_all`, `update_find`) and
  the view (`find_bar_view`). **Since 2026-10-20 (the user's physical
  check):** a chrome row like the toolbar's between the tabs and the ruler,
  not under the ruler: 40-pixel rows, 32-pixel icon buttons, the toolbar's
  gaps and dividers, Options ▾ and Recent ▾ as fixed-width menu roots like
  More ▾; both rows on one grid (the toggle's column, 64-pixel labels, the
  controls after each field in a column as wide in both), so the Find and
  Replace fields start and end together. Its slot stays when closed so the
  document row keeps its place in the tree and the pages their scroll. Options ▾ and Recent ▾ are one-root menu
  bars like the toolbar's More ▾. The options and recent searches are in
  the settings (`find_options`, `recent_searches`); the last bar's texts
  open the next, in any tab, for the session.
- **Keys:** Ctrl+F and Ctrl+H give a field the keyboard with its text
  selected (a focus task chained with a repaint); typed keys go to the
  field, which comes before the pages in the tree, so the page surface
  sees them taken. Enter finds the next, Shift+Enter (the app tracks Shift
  from modifier events) the previous; Enter in Replace replaces; F3 and
  Shift+F3 work anywhere; Esc closes the bar while one of its fields has
  the keyboard (a field focused by a task hears nothing of it, so the bar
  records the field it focuses).
- **On the page** (FD.6): the matches are drawn under the selection in a
  marker yellow wherever their lines show (`page_surface::Highlights`, by
  flow and paragraph); the current match is the selection, drawn alone; a
  match is selected on the page that shows it (the page in view for a
  header) and scrolled into view; the count is "3 of 12" in the bar and the
  status bar.
- **Choices made in the run** (`DECISIONS.md`, 2026-10-20): FD1 a return in
  Find matches a line break too; FD2 Whole word only counts letters and
  digits; FD3 the capitals rule's thresholds; FD4 a selection of 64
  characters or more, or over paragraphs, limits the search (Ami Pro's
  rule), a shorter one fills Find; FD5 the current match is whichever match
  is selected, however it came to be.
- **SAM:** nothing new to store; replacements are ordinary edits. The text
  editor (`--no-default-features`) keeps Find as Future.


## 28. Fields — proposed 2026-10-20; APPROVED by the user 2026-10-20 (FL1–FL7)

**Status: approved** (`DECISIONS.md`, 2026-10-20, "Fields: the user's
answers"): as recommended but two: FL1 also works out Include and the
Query$ and Decide prompts, and FL2 is a **Fields panel in the rail**
instead of the Insert Field dialog. The proposal below is as approved
(the dialog it first proposed is replaced by the panel). Prepared for the queue's
run 4 (`docs/playbooks/FIELDS_AND_REFERENCES.md`) from the decompiled
`AMIPRO.EXE`, `AMIPROUI.DLL`, and `AMIMACRO.DLL`, the user's guide ch. 5,
23, and 32, and VM fixtures FD1–FD6 (`docs/research/FIELDS_NOTES.md`).
(Superseded: Insert › Field… and File › Document Properties… are built; the others were not checked in this pass.) Registered as Future when written: Insert › Field…, File › Document
Properties… (Alt+Enter), Tools › Update Fields (F9), Tools › Word Count.

### What Ami Pro 3.1 does

- **Edit › Insert › Date/Time…**: Today's date (plain text), System date,
  System time, Date of last revision, Date created, in 21 formats shown as
  samples (10/20/26, October 20, 2026, Tuesday, October 20, 2026, 20.
  October 2026, …). The four others are fields: System date and time are
  worked out again whenever their line is laid out.
- **Edit › Insert › Doc Info Field…** (a dialog that stays open): the file
  name, path, style sheet, description, dates and times created and last
  revised, revisions, editing time, pages, words, characters, size, and
  the eight user fields, by their names.
- **File › Doc Info…**: description (119 characters), keywords, the dates,
  revisions, and editing time, statistics dimmed until Update, Lock for
  annotations, Lock revision marking on, Run frame macros; Other Fields…
  (the eight fields) and their Rename Fields…. It shows 2026 as "126".
- **Power fields** (Edit › Power Fields ▸ Insert…, Update, Update All,
  Next Field, Prev Field; View › Show Power Fields): a code typed in a
  dialog (a field list, formats, a syntax line, Save… for named fields,
  Lock, Auto run), worked out by Ami Pro's macro language: counts, Doc
  Info, dates and times, Seq numbering, Set variables, IF, arithmetic,
  text functions, bookmarks, merge fields, Include, prompts, and macros.
  The result is ordinary text between hidden marks; it changes on
  insertion, Update, Update All, Auto run when opened, merging, and
  printing with Update fields. Show Power Fields shows every code in red
  between `<` and `>`.

### Proposal

- **Inserting and changing fields (FL2): the Fields panel in the rail**,
  like Style Properties: opened by Insert › Field… (at Power field), by
  Insert › Date/Time… (at Date and time; Ami Pro's Edit › Insert ›
  Date/Time…), and by a double-click on a field or Edit Field… on its
  right-click menu (at This field). The document stays live: an insertion
  is one undo step, and so is each change to the field at the caret.

```
 Fields                                                ×
 ▾ Insert   ( Date and time | Document info | Power field )
   Date and time
     Insert  [ System date (updates)              ▾ ]
             Today's date (as text), System date, System time,
             Date last saved, Date created
     Format  [ October 20, 2026                    ▾ ]  21 samples
     Shows   October 20, 2026                     [ Insert ]
   Document info
     [ Filename        FD1.SAM          ]  the 22 entries with
     [ Path            L:\OUT           ]  their values now
     [ Date created    20 October 2026  ]  (dates in Format above)
     …                                           [ Insert ]
   Power field
     Field   [ Seq Figure %N2                            ]
     Fields ▾ (Ami Pro's list)  Options ▾ (formats)  Saved ▾  Save…
     Syntax  Seq SequenceName[=e][-e][+e]: numbers in order
     Shows   III
     [ ] Lock   [ ] Update when opened              [ Insert ]
 ▾ This field          (while the caret is in a field)
     System date      Format [ October 20, 2026      ▾ ]
     or: Field [ NumPages            ]  Shows 3  [ Update ]
         [ ] Lock   [ ] Update when opened
```

  The Power field page has Ami Pro's field list (the fields the app does
  not run are marked so, FL1), its options (date and time formats, number
  styles), a syntax line, Lock, and Update when opened (Ami Pro's Auto
  run); Save… keeps a named field in the settings (Ami Pro's AMIPRO.INI
  `[Fields]`), listed under Saved ▾. This field shows the kind of field at
  the caret and what can change: a date's format, a Doc Info entry, or a
  power field's code (applied on Enter), Lock, and Update when opened. The
  status bar names the field at the caret ("Power field: NumPages"); its
  right-click menu has Edit Field…, Update Field (F9), and Lock Field
  (checked).
- **Updating (FL4):** Update Fields (F9): the fields in the
  selection, or the one at the caret; Update All Fields (Ctrl+F9; both in
  Edit › Power Fields ▸ since 2026-10-20);
  Edit › Next Field and Previous Field select the whole field, into the
  other text streams. Dates, times, and Doc Info fields follow on their
  own, as in Ami Pro.
- **Seeing fields (FL6):** View › Show Power Fields (Alt+F9) shows the
  codes in red between `<` `>` as Ami Pro, and is off again before a save;
  View › Field Shading (on) shades every field's shown text light grey on
  screen, never in print.
- **Document Properties in the rail (FL3):** File › Document Properties…
  (Alt+Enter; Ami Pro's Doc Info), like Page Layout Properties:

```
 Document Properties                                   ×
 ▾ Description
   [ Quarterly report for the board                  ]   48 / 119
   Keywords [ budget, 1994                           ]
 ▾ Fields
   Client    [ Alpha                     ]    (the name edits in place)
   Field2    [ Beta                      ]
   …          (eight)
 ▾ Statistics                                   counted as you type
   Pages 3    Words 1,204    Characters 6,882    Size 14 K
 ▾ History
   Created     Tuesday, October 20, 2026   11:23 AM
   Last saved  Tuesday, October 20, 2026   11:26 AM
   Revisions 9    Editing time 2 h 14 min
 ▾ File
   FD1.SAM   in  L:\OUT   Style sheet  _DEFAULT.STY
   Linked files  (pictures and links the document reads)
   Run frame macros  ☑  (kept; this app runs no macros)
   Locked for annotations by …  (shown when Ami Pro locked it)
```

  Each change is one undo step; the statistics count as Ami Pro counts
  (every text stream but notes; words end at spaces, tabs, no-break
  spaces, and paragraph ends) and are written at each save, so Ami Pro
  and Number of Words fields see current figures. Tools › Word Count
  opens it at Statistics.
- **What the app works out (FL1):** the recommended set below, and
  Include (another file's main text, or a bookmark's in it, read when the
  field updates) and Query$ and Decide (a small prompt when the field
  updates, as Ami Pro's).
- **Fixed whatever the answers:** the stored times are written in Ami
  Pro's time base (the wall clock read as US Pacific time), so Ami Pro
  shows the app's save and creation times correctly (today the app writes
  real UTC, 4–8 hours off); editing minutes are added at each save as Ami
  Pro adds them; Ami Pro's "126" for 2026 and its Seq-and-Lock fault
  (MMMMMMMMCXCVII) are not copied; `[desc]` values 2–5 are the fields 1–4
  (the notes called them keywords).
- **SAM:** everything is stored as Ami Pro stores it: `<:D…>`, `<:b…>`,
  `<:r…>`, `<:k…>`, `<:d…>`, `<:X…>` with their results, `[desc]`,
  `[fldnames]`, `[docvars]`. Saved fields and the view choices are in the
  settings.

### Questions (ranked)

- **FL1 Which power fields the app works out.** Recommend everything that
  needs only the document: the counts and Doc Info fields (NumPages,
  NumWords, NumChars, FileSize, NumEdits, TotalEditingTime, CreateDate,
  EditDate, Description), Now, Seq, Set and names, IF, Defined, Void,
  User, arithmetic and comparisons, `{…}` in text, the `%D` `%T` `%N`
  formats, and Ami Pro's text functions (Left$, Right$, Mid$, Len, LCase$,
  UCase$, StrCat$, StrField$, FormatNum$). Merge fields show `<NAME>` and
  MergeRec 0, as Ami Pro outside a merge (the merge run); Bookmark,
  PageRef, and GetBookMark keep their result until bookmarks (the notes
  run); Index and TOC stay marks (the TOC run). Kept with their last
  result and never run: Include, Query$, Decide, Message, Exec, Call,
  Button's action, PrintEscape. Alternatives: also Include (another file's
  text) and Query$ and Decide (a prompt when updated); or only the counts
  and dates. **The user chose the recommended set and also Include and
  Query$ and Decide** (a prompt when the field updates).
- **FL2 Inserting and changing fields.** Recommended an Insert Field
  dialog (one-shot, so a dialog), with changes through the same dialog.
  Alternatives: Ami Pro's three dialogs as they are; or a Fields panel in
  the rail that inserts and edits the field at the caret live. **The user
  chose the Fields panel** (above).
- **FL3 Doc Info.** Recommend Document Properties in the rail, live, as
  above. Alternative: Ami Pro's Doc Info, Other Fields, and Rename Fields
  dialogs.
- **FL4 When fields update.** Recommend Ami Pro's rule: dates, times, and
  Doc Info fields as their lines are laid out; power fields when
  inserted, by Update and Update All, and Update when opened ones as the
  document opens; later, Update All before printing and PDF (an option,
  on). Alternative: power fields also kept current after every edit
  (counts, Seq, names), locked ones excepted.
- **FL5 Numeric dates and times.** Recommend Ami Pro on US Windows as in
  the reference VM: 03/02/05, 12-hour time with AM and PM, English names.
  Alternatives: the desktop's region settings for the date order,
  separator, and 12- or 24-hour time; or Ami Pro's choices (order,
  separator, leading zeros, 24-hour) as preferences.
- **FL6 Telling fields from text.** Recommend Field Shading on (light grey
  on screen only) with Show Power Fields as Ami Pro. Alternatives: Ami
  Pro's look, no shading; or shading only around the field at the caret.
- **FL7 Editing a result, deleting a field.** Recommend Ami Pro's rule
  for typing (a result is ordinary text until the next update) without
  its question on deleting a field (one undo step brings it back).
  Alternatives: exactly Ami Pro, asking before deleting a selection with a
  field; or results protected (a field is one unit the caret steps over,
  deleted whole).

Not proposed now: typing a code on the page with Show Power Fields on
(Ami Pro allows it; here codes change in the Fields panel; built after the
run, see the implementation notes), merge and bookmarks (their runs), and
running macros (not planned).

### Implementation notes (2026-10-20, the fields run)

- **The model and files** (FL.2, FL.4, FL.6): `InlineObject::Date`
  (System date, Date of last revision, Date created, System time),
  `InlineObject::DocInfo`, and `InlineObject::Seq` are objects; other power
  fields stay kept marks (`<:X…>`, read by `PowerMark`), their results
  ordinary text between them; `DocumentInfo` holds `[desc]` and
  `[fldnames]`; the stored times are in Ami Pro's time base (the wall
  clock read as US Pacific time by its runtime's old summer-time rule),
  and the app saves in it, adding the session's minutes and the statistics
  counted as Ami Pro counts them (the file's own size in K).
- **On the page** (FL.3, FL.9): dates and Doc Info entries show their text
  in the formatting where they stand, as one piece; the clock to the
  minute and the file come from the session (`FieldContext`); Field
  Shading fills each field's shown text light grey (objects whole); View ›
  Show Power Fields shows each code in red between `<` `>` in the text
  itself, to be edited as Ami Pro allows (`fields::swap_codes`; FX11 as
  changed 2026-10-20, below), and goes off before a save; the status bar
  names the field at
  the caret; on a field the right-click menu adds Edit Field…, Update
  Fields, and Lock Field.
- **The language and updating** (FL.7, FL.8): `field_language` works out a
  code as Ami Pro's macro interpreter does in field mode (FD3, FD6, FD7;
  45 of Ami Pro's results reproduced); `fields::update_fields` takes the
  caret's field, a selection's, all (Update All: locked and mark kinds
  left, Seq renumbered by stream and name), or the Auto run ones as a
  document opens; Query$ and Decide ask in a dialog and the update runs
  again with each answer; Include reads an Ami Pro document's main text or
  a text file's lines from the document's folder, its paragraphs as lines
  of one paragraph (FX8); an update that reads NumPages sets Ami Pro's
  after-pagination flag (AFD1).
- **The panels** (FL.10, FL.11): the Fields panel (`fields_panel.rs`):
  Insert › Date and time (Ami Pro's 5 choices; 21 formats as samples),
  Document info (the 22 entries with their values), Power field (Ami Pro's
  41 fields, 8 marked not run; options; a syntax line; Lock; Update when
  opened; saved fields in the settings), a live Shows line, and This field
  (a date's format, a power field's code, Lock, Update when opened,
  Update); Document Properties (`document_properties.rs`): Description,
  Keywords, Fields (names and values), Statistics (counted as the document
  changes), History (created, last saved, revisions, editing time), File
  (name, folder, style sheet, Run frame macros, an annotation lock); each
  change one `SetDocumentInfo` step. The Insert pages are a list
  (buttons did not fit the 300-pixel rail).
- **Commands** (FL.12): Insert › Date/Time…, Field…; Edit › Power Fields ▸
  (Ami Pro's submenu, since 2026-10-20 after the run, the user's question:
  "shouldn't there be menu items to show and update powerfields?") Update
  Fields (F9), Update All Fields (Ctrl+F9), Next Field, Previous Field,
  Edit Field…, Lock Field (checked); Tools › Word Count (Document
  Properties at Statistics); View › Show Power Fields (Alt+F9), Field
  Shading (both checked); File › Document Properties… (Alt+Enter). Ami
  Pro's Power Fields ▸ Insert… is Insert › Field….
- **Twin** (FL.13): AFD1, written by the app, opens in Ami Pro showing
  every field as the app does; its Doc Info shows the description,
  keywords, renamed field, the creation and save times as set, the
  revisions, and the minutes; Ami Pro's Update counts the same words,
  characters, and size, and its Update All gives every field the app's
  result.
- **Typing codes** (2026-10-20 after the run; the user agreed that the
  page should take edits while codes show, as Ami Pro's does): View › Show
  Power Fields is one undo step, `Command::ShowPowerFields` (off,
  `HidePowerFields`). On, each outermost power field's result (its text,
  formatting, objects, and inner marks; a Seq's object) is set aside in
  the document (`Document::shown_codes`, never saved) and its code shown
  in red in the format of the result's first character, between the
  field's marks (a Seq and a field without a result get an end mark while
  shown). The codes are ordinary text: typed over, deleted, formatted.
  Off, each code is read back between its first `<` and last `>`: an
  unchanged one gives its result back exactly (runs joined where they were
  one; every document of the three corpora shows and hides back to
  itself), a changed one rewrites the field's marks (its kind from the
  code; a Seq stays an object) and keeps its old result until an update,
  an emptied one deletes the field, and text typed outside the brackets
  stays outside the field, out of red. Update and an insertion with codes
  shown read the codes back first, work on the result, and show the codes
  again in the same step (`fields::codes_hidden`, `CodesHidden::step`),
  as Ami Pro's Update reads the edited code; Update All leaves them off,
  as Ami Pro's switches them off; a save switches them off first, as an
  undo step. The caret and selection move with the swapped text (a
  selected result becomes the selected code). Fields whose result spans
  paragraphs or holds a frame's anchor keep their result.
- **Not built when written** (recorded; bookmarks §34 and merge §38 are built since; macros are deferred):
  Include of a bookmark in another file.

## 29. Print and PDF — proposed 2026-10-20; APPROVED by the user 2026-10-20 (PO1–PO6)

**Status: approved** (`DECISIONS.md`, 2026-10-20, "Print and PDF: the
user's answers"): as recommended but PO6: there is no bulk export (File ›
Export Folder as PDF… is not built). The proposal below is as approved. Prepared for the Phase 5 queue's
run 5 (`docs/playbooks/OUTPUT.md`) from the decompiled `AMIPRO.EXE` and
`AMIPROUI.DLL`, the user's guide ch. 13, and Ami Pro printing VM fixtures
to a PostScript file (`docs/research/PRINT_NOTES.md`). Registered today as
Future: File › Export PDF… (Ctrl+Shift+E) and File › Print… (Ctrl+P).

### What Ami Pro 3.1 does

- **File › Print…** (Ctrl+P): Number of copies; Page range All, Current
  page, or From–To (positions in the document, not printed numbers);
  including Even pages, Odd pages, or Both; the printer and port; Options…
  and Setup…. The choices last for the document until it closes.
- **Print Options**: Reverse order, Collate, Crop marks (the page ½ in in
  from the paper's corner, 7/16-in marks at its corners), Without pictures
  (picture frames print empty), With notes (note marks as "[XX1]" and the
  notes on pages after the document), With doc description (a cover sheet
  listing Doc Info in a fixed-pitch font), On preprinted form (no frame or
  table lines, shading, or fills, no pictures, no protected text), Update
  fields (Update All first), and the first page's and other pages' trays.
- **File › Printer Setup…**: the document's printer (`[prn]`, `[port]`),
  whose metrics lay the document out. Each page layout's paper size and
  orientation go to the printer page by page.
- **Print in background** (User Setup): AmiPrint prints while Ami Pro goes
  on. There is no PDF and no print to file (only a Windows `FILE:` port).

### Proposal

- **One drawing for screen, PDF, and paper.** The page surface draws its
  pages through a PDF renderer of the app's own: the same text, frames,
  tables, bullets, rules, and pictures as on screen, without what only the
  screen shows (margin shading, field shading, find highlights, selection,
  caret, frame handles and outlines, cell outlines). Fonts are embedded and
  subset, so the text is selectable and searchable; each page has its page
  layout's size and orientation; colours as in the document.
- **File › Print…** (Ctrl+P) opens **the desktop's print dialog** at once
  (COSMIC asks GTK's: printer, pages, copies, collate, reverse, odd or even
  pages, pages per sheet, two-sided, paper, quality, Print to File), set to
  the document's paper and orientation. Print sends the chosen pages as a
  PDF made in the background; the status bar says "Printing 12 pages on
  PRINTER". Its Pages range counts positions in the document, as Ami Pro's
  From–To does.
- **File › Print Current Page…** (new): the same dialog with the caret's
  page as the range (Ami Pro's Current page, which the desktop's dialog
  cannot offer itself).
- **File › Export PDF…** (Ctrl+Shift+E): the desktop's Save dialog, the
  document's name with `.pdf`, beside the document; all pages, written to a
  new file and renamed into place as saves are.
- **Ami Pro's options in a Print panel in the rail** (File › Print
  Options…, also from the Print commands' status message), 300 px like
  Document Properties; the options last for the document until it closes,
  as Ami Pro's do, and apply to printing and to PDF alike. While the panel
  is open, the pages show as they will print.

```
 Print                                                   ×
 ▾ What prints        (this document, until it closes)
   ☐ Crop marks           the page ½ in in, marks at its corners
   ☐ Without pictures     picture frames print empty
   ☐ With doc description a cover sheet listing Doc Info
   ☐ On preprinted form   no frame or table lines, shading,
                          pictures, or protected text
   ☐ Update fields first  Update All before printing
 ▾ PDF
   ☑ Bookmarks from outline levels
   ☑ Doc Info as the PDF's properties; Ami Pro's page numbers
   ☐ PDF/A-2b, for archiving
   The pages show as they print while this panel is open.
 [ Print… ]   [ Print Current Page… ]   [ Export PDF… ]
```

  Copies, collate, reverse, odd and even, and trays are the desktop
  dialog's (it passes them to CUPS); Ami Pro's dialog is not rebuilt, so
  no choice has two homes. With notes waits for notes in the model (the
  notes run).
- **Crop marks**: on paper as Ami Pro prints them (the page ½ in right and
  down, the paper the desktop dialog chose). In a PDF the paper is the page
  with ½ in all round, the trim box the page, and the four corners' marks
  in that border.
- **With doc description**: Ami Pro's cover sheet (its labels, fixed-pitch
  10 pt, 1 in from the paper's corner, double-spaced, the description
  wrapped), with four-digit years.
- **Pictures in a PDF**: bitmaps at their own pixels; metafiles, SVG, and
  Ami Pro's drawings' screen images as vectors where they are vectors; a
  picture the app cannot draw (a labelled box on screen) prints nothing,
  as an empty frame.
- **Not built (decided)**: Printer Setup (the desktop's dialog chooses the printer;
  `[prn]` and `[port]` are kept as read), Ami Pro's Print Orientation
  dialog (each page goes out at its own size), AmiPrint (printing is always
  in the background), the /P switch.
- **SAM:** nothing new is stored: Ami Pro keeps print options for the
  session only, and so does the app. The PDF choices are in the settings.
- **The text-editor build** keeps Print and Export PDF as Future (it has
  no pages).

### Questions (ranked)

- **PO1 Printing.** Recommend the desktop's print dialog through the
  portal, as above, with Ami Pro's options in the rail. Alternatives: Ami
  Pro's Print and Print Options dialogs rebuilt in the app, sending to
  CUPS directly (no printer properties, two-sided, or quality); or a sheet
  with Ami Pro's options before the desktop's dialog at every print.
- **PO2 PDF and pictures.** Recommend `krilla` (Typst's PDF writer;
  MIT/Apache; fonts subset and embedded, text selectable, PDF/A), with
  bitmaps at their own pixels and vectors as vectors. Alternatives: krilla
  with every picture as the screen draws it at 300 dpi (simpler; a
  metafile's text not selectable); or `pdf-writer` alone (more of the
  app's own code: font subsetting, text).
- **PO3 With notes.** Recommend building it in the notes run, when notes
  are in the model. Alternative: now, from the kept note marks.
- **PO4 PDF additions.** Recommend bookmarks from outline levels, Doc Info
  as the PDF's properties, Ami Pro's page numbers as page labels, and a
  PDF/A-2b option; tagged PDF (structure for screen readers) later.
- **PO5 The preview.** Recommend the pages showing as they print while the
  Print panel is open. Alternatives: a View › Print Preview mode; none.
- **PO6 Many files at once.** Recommend File › Export Folder as PDF…: a
  folder of SAM files to PDFs beside them or in another folder, with a
  report of what each lost or could not draw (the user's archive), and the
  same from the command line. Alternatives: one bulk export later, with
  the DOCX export; none. **The user chose none:** one document at a time.

The user took the recommendation for PO1–PO5 (PO4 answered in text: all
three additions).

### Implementation notes (2026-10-20, the print-pdf run)

- **One drawing.** `pdf.rs`'s `PdfRenderer` implements iced's renderer
  traits and records what the page surface draws as iced records it
  (layers in order; in each, quads, then images, then text), then writes
  it into a krilla page: quads as rounded paths with their borders as
  rings, layers as clips, paragraphs as glyph runs placed where COSMIC Text
  put each glyph, with their text (synthetic italic as a slant), images at
  their own pixels. `print.rs` lays the surface out once at a pixel a twip
  and draws each page into its own PDF page (`document_pdf`, `draw_pages`),
  on a thread of its own (`PdfJob`, `in_background`).
- **Print mode** (`PageSurface::printing`, `PrintMode`): for paper, no
  shadow, margin shading, field shading, find highlights, selection,
  caret, handles, guides, table grid (the screen's guide), or labelled
  boxes for pictures the app cannot draw (PX1, PX2); Without pictures and
  On preprinted form as Ami Pro (protected text not in the PDF at all).
  Frame lines are their style's width on paper and in the preview, placed
  by their position (PX9); on screen they stay one or two pixels.
- **Pictures** (`pdf_pictures.rs`): the picture cache registers a stand-in
  for each unturned metafile or SVG picture (`Pictures::printed`); the PDF
  draws metafiles as paths, images, text, clips, and hatch and pattern
  fills (tiling patterns), and SVG as paths. 230 of 250 metafiles in the
  user's corpus, all 13 and 7 in the ISO and VM folders, print as vectors;
  turned and unreadable ones at 300 dpi as the screen draws them.
- **Options** (`PrintOptions` on the session, `print_panel.rs`): Crop marks
  (`CropMarks::AroundPage` in a PDF, `OnPaper` printing; quarter-point
  hairlines, PX6), With doc description (`CoverSheet`, Liberation Mono 10
  pt from the paper's corner, PX4), On preprinted form, Without pictures,
  Update fields first (a field's question stops the print, PX5). The PDF's
  additions in the settings (`pdf_bookmarks`, `pdf_properties`,
  `pdf_archival`).
- **Printing** (`print_portal.rs`): `PreparePrint` with the first page's
  paper (by its PWG name within 1/8 in, else custom) and, for Print Current
  Page…, the caret's page as the range (PX7); the dialog's range read back
  (0-based) and applied by the app; the PDF sent with `Print` and the
  dialog's token through a temporary file; the job named after the file
  (PX8).
- **Measured:** README31's 14 pages in 62 ms, AP31NAV's 40 in 257 ms
  (release); the PDF against the page surface page by page at 100 dpi
  (`pdf_against_screen`): every page of a 60-file user sample and a 60-file
  ISO sample within 5% of non-coinciding ink, the VM folder's differences
  in hairlines the screen draws a whole pixel wide.
- **Not built** (recorded): With notes (the notes run); table lines (Lines
  & Color, the table-editing run); turned metafiles as vectors; double and
  triple frame line styles drawn as one line; Printer Setup; tagged PDF.
- **After the physical check** (2026-10-20): the Print panel's three
  buttons span the rail, stacked under a divider with their labels
  centred, Print… first (the user's choice). libcosmic's text buttons give
  a set width to their label's row, not the button, so they are custom
  buttons (`print_panel::wide_button`).

## 30. Export — proposed 2026-10-20; APPROVED by the user 2026-10-20 (EX1–EX10)

**Status: approved** (`DECISIONS.md`, 2026-10-20, "Export: the user's
answers"): as recommended, with both one document at a time and
Export ▸ Folder… (EX6), DOCX, ODT, RTF, and plain text (EX1), and table
lines and shading mapped and drawn (EX5). Prepared for the Phase 5 queue's run 6 (`docs/playbooks/OUTPUT.md`,
"Run 2"), from Ami Pro's filters (the decompiled `rtf.flt`), Ami Pro 3.1
exporting VM fixtures, and the formats' specifications
(`docs/research/EXPORT_NOTES.md`). Registered today: File › Export PDF…
(Ctrl+Shift+E); nothing else exports.

### What Ami Pro 3.1 does

- **File › Save As** has "List files of type" with 33 formats (ASCII, Rich
  Text Format, Word for Windows 1.x, 2.0, and 6.0, WordPerfect 5.x and 6.0,
  and others). The document keeps its own name and file: another type is
  written as a copy.
- Each format is a **filter** (a DLL) fed one stream of tokens by Ami Pro;
  the same filter reads its format back (File › Open with that type, after
  Import Options: Convert, Apply, Keep, or Ignore styles).
- **What its exports keep:** the text, styles (and a private outline
  level), bold, italic, faces and sizes, left and right margins, the header
  text, tables, and pictures (as metafiles). **Frames are dropped with their
  text**, in RTF and Word alike; so are inserted bullets.

### Proposal

- **One export pipeline, a filter per format** (Ami Pro's design, the
  user's suggestion): the document is resolved once into a neutral form
  (styles, sections, paragraphs, frames as shapes, tables, lists, fields,
  pictures, properties) and each format's filter writes it; the import run
  adds each filter's reader. Proposed formats: **Word Document (.docx)**,
  **OpenDocument Text (.odt)** (ISO/IEC 26300, the archival standard and
  LibreOffice's own format, whose frames are closest to Ami Pro's),
  **Rich Text (.rtf)** (every word processor, and Ami Pro itself), and
  **Plain Text (.txt)** (Ami Pro's ASCII). Considered: a web page (.html,
  readable in any browser; offered as an option), Markdown and EPUB (not
  proposed: they hold no page design), Lotus Word Pro (a closed format).
- **File › Export ▸** (a submenu in place of Export PDF…):

```
 File
   …
   Export                ▸ │ PDF…                       Ctrl+Shift+E
   Print…         Ctrl+P   │ Word Document (.docx)…
   Print Current Page…     │ OpenDocument Text (.odt)…
   Print Options…          │ Rich Text (.rtf)…
   …                       │ Plain Text (.txt)…
                           │ ─────────────
                           │ Folder…
```

  Each item opens the desktop's Save dialog with the document's name and
  that extension, beside the document; the file is written as a copy (the
  tab keeps its own file and name, as in Ami Pro), to a new file renamed
  into place, in the background, and the status bar says "Exported
  NAME.docx (14 pages)" with an **Open** button that opens it in the
  desktop's program for that type.
- **What the format cannot hold** is listed before writing, as Save As SAM
  does (`UI_SPEC.md` §17): "Word Document cannot hold: protected text (3
  places); 2 notes …" with Cancel and Export Anyway. Nothing is listed
  when nothing is lost.
- **Frames go where Ami Pro puts them**: text frames as floating text
  boxes (ODT: frames, as Ami Pro's), picture frames as floating pictures, with their lines, shadow,
  rounded corners, fill, margins, and columns; Where placed frames on
  their page, With para above below their paragraph, Flow with text in the
  line, repeating frames on every page (through the header), and Ami Pro's
  three wraps and its layering. RTF writes the same shapes with a
  positioned-paragraph fallback that older readers, and Ami Pro itself,
  read as frames.
- **Pages:** each page layout a section (paper, orientation, margins,
  columns of equal or unequal width with the line between, lines around
  the page, mirrored and left pages, Begin on second page), header and
  footer text, page numbers in Ami Pro's style.
- **Styles keep their names**, with every attribute; Body Text is the
  default paragraph style; outline levels go with them, so Word's and
  LibreOffice's navigators show the outline.
- **Lists stay lists** (renumbering when edited in the other program),
  with Ami Pro's levels, formats, and reset rules; **fields stay fields**
  where the format has the same one (page number, page count, dates, Doc
  Info entries, file name, Seq), showing Ami Pro's current result; others
  as their result.
- **Pictures** as the format holds them best: metafiles as metafiles
  (vectors), bitmaps as PNG, JPEG and GIF as they are, SVG with a PNG
  fallback; drawings, charts, and OLE objects as Ami Pro's screen image;
  each scaled, cropped, and turned as the frame shows it.
- **Doc Info** becomes the file's properties: description, keywords, the
  four named fields, created and saved times, revisions, editing time, and
  the counts.
- **File › Export ▸ Folder…** (if chosen): a panel in the rail (300 px):

```
 Export Folder                                            ×
 From   ~/Archive/SAM                       [ Choose… ]
        ☑ Include subfolders
 To     ~/Archive/DOCX                      [ Choose… ]
 As     ◉ Word Document  ○ OpenDocument  ○ Rich Text
        ○ Plain Text
 Style sheets: the document's folder and the style-sheet folders
                                            [ Export ]
 Exporting 37 of 423 …                       [ Stop ]
```

  The originals are only read. Each document is written to the same
  relative place under To, never over an existing file (a number is added),
  and an `Export report.txt` there lists each file with what its format
  could not hold and any file that could not be read.
- **SAM:** nothing new is stored. The export choices (last format, folder)
  are in the settings.
- **The text-editor build:** the export commands stay Future (no pages for
  the placements); the pipeline itself needs no page surface.

### Questions (ranked; DECISIONS EX…)

- **EX1 Formats.** Recommend DOCX, ODT, RTF, and plain text through one
  pipeline (each one more reader in the import run). Alternatives: DOCX,
  RTF, and plain text; those four plus a web page (.html); DOCX only.
- **EX2 The commands.** Recommend File › Export ▸ with one item per format,
  a copy each time. Alternatives: one File › Export… with the format in the
  desktop dialog's type list; the formats in Save As's type list.
- **EX3 Frames.** Recommend floating text boxes and pictures where Ami Pro
  places them. Alternatives: Word's older framed paragraphs (no lines,
  shadows, or corners); frames' contents in the text at their anchor.
- **EX4 The word processor the files are for.** Recommend LibreOffice
  Writer (installed here, so every export is checked against it
  automatically). Alternatives: Microsoft Word, Google Docs, several.
- **EX5 Table lines and shading.** Ami Pro stores each cell's lines and
  shading, which the app keeps but neither draws nor prints yet. Recommend
  mapping them in this run (decompile and VM) and drawing them on screen,
  on paper, in PDFs, and in exports. Alternative: wait for the
  table-editing run (exported tables have no lines until then).
- **EX6 A whole folder at once.** Recommend File › Export ▸ Folder… for the
  archive. Alternative: one document at a time, as for PDF.
- **EX7 What cannot be written.** Recommend the list before writing, with
  Cancel and Export Anyway, as Save As SAM. Alternatives: write, then list
  in the status bar with Details; never list.
- **EX8 Lists and fields.** Recommend live lists and fields where the
  format has them. Alternative: every number, bullet, and field result as
  plain text exactly as Ami Pro shows it.
- **EX9 Style names.** Recommend Ami Pro's names, Body Text the default.
  Alternative: Word's names for the common styles (Body Text as Normal,
  levelled styles as Heading 1–9).
- **EX10 Additions.** Recommend Doc Info as the file's properties, the Open
  button after an export, and the last format and folder remembered.
  Alternatives: properties only; none.

### Implementation notes (the export run, 2026-10-20)

Built as approved; choices EC1–EC15 (`DECISIONS.md`;
`docs/EXPORT_PROGRESS.md`).

- **One pipeline** (`crates/exchange`, `amipro-exchange`, without
  dependencies): `build` resolves the model, the session's pages, and the
  fields' context into an `ExchangeDocument`; `docx.rs`, `odt.rs`,
  `rtf.rs`, and `text.rs` write it; `losses.rs` names what each format
  cannot hold. Zips are deflated by the app (`flate2`); pictures are
  converted by the app (`Pictures::exported`, EC3).
- **File › Export ▸** (`command.rs`, `EXPORT_MENU`): Export PDF…
  (Ctrl+Shift+E), Export Word Document (.docx)…, Export OpenDocument Text
  (.odt)…, Export Rich Text (.rtf)…, Export Plain Text (.txt)…, and Export
  Folder… (EC9). Each document export opens the desktop's save dialog in
  the last export's folder (EC10), makes the export on a thread of its own
  (`export::ExportJob`), and writes it when nothing is lost, else shows
  the loss dialog (Cancel, Export Anyway). The status bar then says
  "Exported NAME.docx (Word Document)" with **Open** (the OpenURI portal).
  The text-editor build keeps them Future.
- **Markdown** (MD1, MD2; 2026-11-06): the fifth exchange format.
  File › Export ▸ Export Markdown (.md)… follows Plain Text in the menu,
  no shortcut. It writes CommonMark with GitHub's tables, strikethrough,
  and footnotes through `amipro_exchange::markdown`; the pictures are
  written beside the file as `NAME-1.png`, `NAME-2.png`, and so on, and
  linked relatively. The loss dialog **always** opens: besides what the
  document uses that Markdown lacks, it lists `always_lost(Markdown)` (page
  layout, fonts and colours, tab stops and indents, styles other than
  headings, lists, quotes, and code, spanned or bordered table cells), and
  "These files already exist and will be replaced: …" when the .md or a
  picture file is there (Cancel writes nothing). **File › Save As** has a
  third Format choice, Markdown (.md), and a `.md` filter; a typed `.md`,
  `.markdown`, or `.mdown` is Markdown whatever the choice. It is an
  export: the tab keeps its own file and its changed mark, and a close in
  progress is not taken as saved. **Open and Insert › File…** read `.md`,
  `.markdown`, and `.mdown` (found by extension only) through Import
  Options, whose style choice applies (headings become Heading 1–6); the
  status says "Opened NAME.md (Markdown)"; links, pictures, and HTML are
  listed by the Import Report; the tab is an import, so Save goes to Save
  As. Export Folder… does not offer Markdown (its report would list the
  fixed losses for every file). Scenarios: `scenario_tests/markdown.rs`.
- **Export Folder…** (`export_folder.rs`): the rail panel as proposed (300
  px), From with Include subfolders, To, the four formats, Export or Stop,
  "Exporting 37 of 423…", the result, and Open the report. The last
  folders and format are in the settings (`export_folder`, `export_from`,
  `export_format`).
- **Table lines and shading** (EX5): read from each cell's kept record
  (`TableCell::lines`, `fill`), placed by `amipro_layout::cell_box` and
  `cell_line_strokes`, drawn by the page surface on screen (at least a
  pixel) and on paper, so the Print panel's preview, printing, and the PDF
  have them; exported as cell borders and shading. The grey cell grid
  stays the screen's guide.
- **What the formats show** (LibreOffice 24.2, fixtures and corpus
  samples; Ami Pro 3.1 for RTF): `docs/research/EXPORT_NOTES.md` and the
  progress file. Known differences: LibreOffice hyphenates a few more
  words; headers grow to show what Ami Pro hides; rounded corners are
  square on frames holding text in ODT and RTF; Ami Pro's RTF reader
  takes frames as paragraphs and shows no PNG pictures.
- **Measured:** README31's 14 pages as DOCX in 4.1 ms (release, pictures
  converted).

## 31. Import — proposed 2026-10-21; APPROVED by the user 2026-10-21 (IM1–IM12)

**Status: approved** (`DECISIONS.md`, 2026-10-21, "Import: the user's
answers"): as recommended, except that nothing is opened through
LibreOffice (IM10) and Import Folder… writes `.amipro` documents (IM11).
Prepared for the Phase 5 queue's run 7 (`docs/playbooks/OUTPUT.md`,
"Run 3"), from Ami Pro 3.1 importing VM fixtures (IM1.RTF, IM2.TXT,
IM3.TXT, NT1), the user's guide ch. 22, the decompiled `rtf.flt` and
`AMIPRO.EXE` (`docs/research/IMPORT_NOTES.md`), and the export run's
exchange document. Registered today: File › Open reads SAM and `.amipro`;
nothing else imports.

### What Ami Pro 3.1 does

- **File › Open** has OK, Cancel, **Insert**, Close current file, List files
  of type, and Preview. OK opens the file as an **Untitled** document (an
  ASCII file keeps its name and saves back as ASCII). **Insert** puts the
  file at the insertion point, in the main text, a selected text frame, or
  a table cell, and leaves the insertion point where it was. There is no
  separate Import command.
- **Import Options** (RTF, Word, and other formats with styles), before the
  file opens, Convert styles preselected:
  - **Convert styles:** the file's styles become the document's, as the
    file defines them; a style of the same name is redefined (on Insert too,
    so the document's own Body Text changes).
  - **Apply styles:** every paragraph Body Text, each style's look applied
    as formatting.
  - **Keep style names:** the paragraphs keep the file's style names, but
    the document's style sheet defines them; names it lacks are missing
    styles until a sheet with them is used.
  - **Ignore styles:** every paragraph Body Text with only the file's own
    formatting.
- **ASCII Options** (a button in Open when ASCII is the type): CR/LF at
  lines (lines joined into paragraphs, a blank line ending one; the
  default) or CR/LF at paragraph ends only (each line a paragraph); Keep
  style names (`@Style@` at a line's start applies that style); 7-bit
  ASCII, 8-bit PC-ASCII (code page 437; the default), or 8-bit ANSI.
- **Page set-up:** the file's paper and margins become a page layout the
  document starts in (an inserted layout named 1; Standard stays the
  style sheet's). Insert starts the inserted text on a new page in that
  layout.
- **What its filters keep** (`rtf.flt`): styles (names cut to 13
  characters), formatting, page set-up, headers and footers, tables (no
  joined cells, row heights, or fill), Windows metafiles and bitmaps, page
  numbers, and footnotes; frames, shapes, lists, most fields, bookmarks,
  and document information are not read (`IMPORT_NOTES.md`). Nothing tells
  the user what was dropped.
- **Paste** of RTF from the clipboard goes through the same filter with
  Apply styles.

### Proposal

- **File › Open** also opens Word Documents (.docx), OpenDocument Text
  (.odt, and LibreOffice's flat .fodt), Rich Text (.rtf), and Plain Text
  (.txt), recognised by their contents as well as their names. The desktop's
  Open dialog lists "All documents" first, then each format.
- **Import Options** as a small dialog before the file opens (as in Ami
  Pro), the last choice preselected:

```
 Import Options — Quarterly report.docx                     ×
 Paragraph styles
   ◉ Convert styles     the file's styles, as the file defines them
   ○ Apply styles       Body Text, each style's look kept as formatting
   ○ Keep style names   the names, defined by this document's styles
   ○ Ignore styles      Body Text with the file's own formatting only
                                               [ Cancel ]  [ Open ]
```

  For a text file the same dialog shows Ami Pro's ASCII options:

```
 Import Options — notes.txt                                 ×
 Line ends
   ◉ Each line is a paragraph       (CR/LF at paragraph ends only)
   ○ Join lines into paragraphs     (CR/LF at lines; a blank line ends one)
   ☐ Keep style names               (@Style@ at a line's start)
 Characters   Unicode (UTF-8), detected                       [▾]
              (Unicode, Windows ANSI, PC-ASCII, 7-bit ASCII)
                                               [ Cancel ]  [ Open ]
```

- **The opened file** is a new tab named after the file, its source
  protected: Save goes to Save As, which writes an `.amipro` or `.SAM`
  document and never the original.
- **The page set-up** comes with it: each of the file's sections becomes a
  page layout (paper, orientation, margins, columns, a line between them,
  lines around the page, headers and footers, left and right pages, a
  different first page as Begin on second page); the first is the
  document's Standard layout.
- **Insert › File…** (Ami Pro's Open › Insert) puts a file at the caret, in
  the main text, a frame's text, or a table cell, as one undo step, after
  the same Import Options.
- **What the document cannot hold** is listed in an **Import Report** (if
  chosen), never dropped silently: a panel in the rail (300 px) that opens
  by itself when anything was changed or left out, each item with a jump to
  its place:

```
 Import Report — Quarterly report.docx                    ×
 Opened as an Ami Pro document. 6 things are different:
 ▸ 2 comments kept as Ami Pro notes                  Go to
 ▸ 3 links kept as their text                         Go to
 ▸ A chart kept as its picture                        Go to
 ▸ Tracked changes accepted (4 insertions, 1 deletion)
 ▸ Character style "Emphasis" applied as formatting (12 places)
                                                 [ Copy ]
```

- **Frames:** text boxes and pictures become Ami Pro frames placed as the
  file places them: on a page (Where placed), below a paragraph (With para
  above), or in the line (Flow with text), with their wrap, lines, fill,
  shadow, corners, margins, and columns; shapes in headers repeat on every
  page. Word's framed paragraphs and RTF's positioned paragraphs become
  frames too. Shapes Ami Pro cannot hold (lines, arrows, WordArt, groups,
  charts) are kept as their picture where the file stores one.
- **Tables** keep their grid, joined cells, row heights, margins, heading
  rows, lines, and shading; a floating table becomes a frame holding it.
- **Lists** keep their numbers and bullets as Ami Pro's list styles (IM5),
  **fields** become Ami Pro's where it has the same (page number, page
  count, dates, Doc Info entries, file name, Seq, index and contents
  entries), others their result; the file's properties become Doc Info.
- **Pictures:** PNG, JPEG, GIF, BMP, TIFF, WMF, SVG as the app draws them
  already, and **EMF** (Word's usual drawings) drawn like WMF.
- **Additions** (if chosen): drop a document on the window to open it;
  other formats (.doc, .wpd, .lwp, …) through LibreOffice when it is
  installed (not chosen, IM10); **File › Import Folder…**, a rail panel as
  Export Folder (chosen, writing `.amipro`, IM11):

```
 Import Folder                                            ×
 From   ~/Word files                        [ Choose… ]
        ☑ Include subfolders
 To     ~/Ami Pro                           [ Choose… ]
 As     AmiPro-Linux documents (.amipro)
 Styles ◉ Convert  ○ Apply  ○ Keep names  ○ Ignore
                                            [ Import ]
 Importing 12 of 40 …                        [ Stop ]
```

  The originals are only read; each document is written to the same
  relative place under To, never over a file, with an `Import report.txt`.
- **SAM:** nothing new is stored. Comments become Ami Pro notes (`<:N…>`,
  written as Ami Pro writes them). Save As SAM lists what Ami Pro 3.1
  cannot hold, as for any document.
- **The text-editor build:** File › Open reads the same formats (the
  reading needs no page surface); Insert › File… and the Import Report stay
  in the page-surface build.

### Questions (ranked; DECISIONS IM…)

- **IM1 The commands.** Recommend File › Open for the four formats, Insert ›
  File… at the caret, and a document dropped on the window opening it.
  Alternatives: Open only; Open and Insert › File… without the drop.
- **IM2 Styles.** Recommend Ami Pro's Import Options (Convert, Apply, Keep,
  Ignore) asked each time, the last choice preselected. Alternatives:
  Convert always, without asking; ask only on Insert › File….
- **IM3 Insert › File….** Recommend the file's text at the caret, keeping
  the document's own definitions of styles it already has (only new styles
  added) and its page set-up. Alternative: Ami Pro's way (the inserted text
  on a new page in the file's page set-up, Convert redefining the
  document's styles of the same name).
- **IM4 Word's style names.** Recommend Word's and LibreOffice's default
  paragraph style (Normal, Default Paragraph Style) as Body Text, every
  other name kept. Alternatives: every name kept, Normal a style of its own
  (Ami Pro's way); Word's common styles renamed to Ami Pro's (Title,
  Subhead, Bullet 1, Number List).
- **IM5 Lists set on paragraphs** (Word and LibreOffice; Ami Pro holds
  bullets and numbers in styles). Recommend Ami Pro's list styles, as the
  app's Bullets and Numbering commands make them (Bullet 1, Number List,
  List 2…), each paragraph's own look kept as its formatting. Alternatives:
  a new style per style and list; numbers and bullets as text.
- **IM6 Comments, footnotes, endnotes** (the app shows none until the notes
  run, 10). Recommend comments as Ami Pro notes, and footnotes and endnotes
  as numbered paragraphs at the document's end with their numbers raised in
  the text. Alternatives: all listed and left out; footnotes in brackets
  where they are referenced.
- **IM7 Tracked changes and what Ami Pro cannot hold.** Recommend the
  document as it stands (changes accepted), the nearest Ami Pro form for
  the rest (links as their text, character styles as formatting, a nested
  table as its text, a floating table in a frame), all listed. Alternative:
  refuse files with tracked changes until they are accepted elsewhere.
- **IM8 The Import Report.** Recommend the rail panel with jumps, opened by
  itself when anything is listed. Alternatives: a dialog after opening; a
  status-bar message only.
- **IM9 Plain text.** Recommend Ami Pro's ASCII options in the Import
  Options dialog with Each line is a paragraph and Unicode (detected)
  preselected. Alternatives: Ami Pro's defaults (joined lines, PC-ASCII);
  no options (each line a paragraph, the encoding detected).
- **IM10 Other formats through LibreOffice.** Recommend File › Open also
  listing Word 97–2003 (.doc), WordPerfect (.wpd), Lotus Word Pro (.lwp),
  and others when LibreOffice is installed, converted by it in the
  background (headless, a private profile) and then imported. Alternative:
  none.
- **IM11 Import Folder….** Recommend it (as Export Folder), writing Ami Pro
  3.1 documents. Alternatives: writing `.amipro`; none.
- **IM12 Paste from other programs.** Recommend Paste taking the
  clipboard's rich text (RTF, offered by LibreOffice, Word, and WordPad)
  through the same reader, as Ami Pro pastes RTF: its look kept as
  formatting (Apply styles), tables and pictures with it; Paste as Plain
  Text stays. Alternative: plain text, as today.

### Implementation notes (the import run, 2026-10-21 to 2026-10-24)

Built as approved; choices IC1–IC33 (`DECISIONS.md`;
`docs/IMPORT_PROGRESS.md`).

- **One mapping, both ways** (`crates/exchange`): the readers `read_docx`,
  `read_odt` (and flat `.fodt`), `read_rtf`, and `read_text` each produce
  the export's `ExchangeDocument`, on the project's own zip and XML
  readers (inflate passed in by the app, `flate2`); `realise` turns any
  exchange document into the model, with the findings (`ImportFinding`
  kinds, counts, and markers for their places). Enhanced metafiles are
  played like Windows metafiles (`amipro-pictures` `emf`).
- **File › Open** lists All documents, then Ami Pro documents,
  AmiPro-Linux documents, and each format read; a file is known by its
  contents first. Import Options is the dialog as proposed (the style
  choice, or for text the line ends, Keep style names, and the characters,
  Detected first), the last choice kept in the settings
  (`import_styles`, `import_line_ends`, `import_keep_names`). The file is
  read and realised on a thread of its own (`import::ImportJob`, a private
  COSMIC Text shaper for the layouts `realise` needs) and opens in a new
  tab as a protected source: Save goes to Save As. The text-editor build
  opens the same formats through the same dialog.
- **Insert › File…** (after Image…) puts the file at the caret, or the
  selection's start, of the tab it was chosen in, in the main text, a
  frame's text, or a cell (tables and frames become text there), as one
  `InsertDocument` undo step; the document's own styles keep their
  definitions; its page set-up is untouched (IM3).
- **Paste** reads the clipboard's RTF (`text/rtf`, `application/rtf`,
  `text/richtext`) first and pastes it with Apply styles, replacing the
  selection; without RTF it pastes text as before (IM12). A document
  dropped on the page opens through Import Options; pictures still go in
  where they are dropped.
- **The Import Report** (`import_report.rs`, 300 px in the rail) opens by
  itself when an import lists anything: the file and format, "Opened as an
  Ami Pro document. 6 things are different:", each kind with its count and
  Go to (its places in turn), and Copy. The status bar's "N import notes"
  opens it again; ‹ Styles and × as the other panels.
- **File › Import Folder…** (after Open; `import_folder.rs`): the rail
  panel as proposed, writing `.amipro` documents at the same relative
  places under To, never over a file, with `Import report.txt`; the last
  folders kept (`import_from`, `import_to`).
- **Save As SAM** of an imported document lists, besides what Ami Pro
  cannot hold, style names longer than Ami Pro's 13 characters, which it
  shortens (IC31). Ami Pro 3.1 opens the app's writing of imported
  documents as the app shows them (twins AIM1, AIMD, AIML, AIMN, and
  AIMO's table; `docs/IMPORT_PROGRESS.md`, IM.15).
- **Measured:** README31 read and realised in 2.6–3.6 ms (release) from
  DOCX, ODT, or RTF, and laid out in 19 ms.

## 32. Making and editing tables — proposed 2026-10-24; APPROVED by the user 2026-10-24 (TD1–TD11)

**Status: approved** (`DECISIONS.md`, 2026-10-24, "Table editing: the
user's answers"): as recommended, except that dragging a grid line has no
Shift variant (TD6), the additions include table styles (TD7), and table
lines get a colour kept in the native format only (TD8). Prepared for the Phase 5 queue's run 8
(`docs/playbooks/TABLES.md`, "Run 2"), from the decompiled `AMIPRO.EXE`
and `AMIPROUI.DLL` and Ami Pro 3.1 fixtures TE1–TE20
(`docs/research/TABLE_EDIT_NOTES.md`). Today tables are read, laid out,
drawn, printed, exported, imported, and written back, their text is
editable, and Insert › Page Break in a cell sets a row's break; nothing
else about a table can be changed, and Insert › Table… is a Future
command.

### What Ami Pro 3.1 does

- **Tools › Tables…** asks for columns and rows (5 × 5; up to 250 × 4000)
  and makes a table at the caret, its anchor inside the caret's paragraph,
  the columns filling the text column; with an empty text frame selected
  the frame becomes the table's (a frame table). Layout… sets the new
  table's sizes and options first.
- **The Table menu** appears after Tools while the caret is in a table:
  Modify Table Layout…, Lines & Color…; Insert Column/Row…, Delete
  Column/Row…, Delete Entire Table, Column/Row Size…; Select Column, Row,
  Entire Table; Connect Cells (Disconnect Cells in a joined cell), Headings,
  Leaders ▸, Protect Cells; Edit Formula…, Quick Add ▸ (the table-data
  run).
- **Modify Table Layout** sets the defaults (column width and gutter, row
  height or Automatic, row gutter) and the options (Center table on page,
  Line around table with a line style, Honor protection, Rows span pages,
  Disable mouse col/row sizing). **Column/Row Size** gives the selected
  columns and rows their own width, height, and gutters. A column's or
  row's grid line can be dragged: that column or row changes, the table
  gets wider or narrower.
- **Insert Column/Row** adds columns or rows Before or After, at the
  defaults; the table grows and **refuses a column that does not fit**
  ("Not enough room to insert a column."). **Delete Column/Row** removes
  the caret's (or the selected) rows or columns; the table narrows.
- **Cells:** a drag from one cell into another selects a rectangle of whole
  cells (also Shift+click, Shift+Ctrl+arrows, the strips above a column and
  left of a row). Connect Cells joins it (the hidden cells keep their
  text); Protect Cells, with Honor protection, keeps the caret out;
  Leaders fill the rest of a cell with dots, dashes, or a line; Lines &
  Color sets each cell's sides (12 black styles) and fill. Delete empties
  the selected cells, Copy, Cut, and Paste work on cell ranges, and drag
  and drop moves rows, columns, or blocks.
- **Keys:** Tab and Shift+Tab move between cells; Tab in the last cell
  leaves the table; Ctrl+arrows move a cell at a time.
- **No undo** for any of it: Undo is greyed after Insert Row or Protect,
  and deleting rows or columns warns that it cannot be undone.

### Proposal

**Making a table.** Insert › Table… (Ctrl+F12; Ami Pro's Tools › Tables…,
also on the toolbar as Ami Pro's grid icon) opens a small dialog: columns
and rows as number fields beside a grid to sweep over, and Layout… (the
new table's sizes and options, the rail section below in a dialog). The
table goes at the caret as Ami Pro puts it, or into a selected empty text
frame. The caret lands in its first cell. One undo step.

```
 Insert Table                                          ×
 ┌─┬─┬─┬─┬─┬─┬─┬─┐   Columns [  3 ]    (1–250)
 ├─┼─┼─┼─┼─┼─┼─┼─┤   Rows    [  3 ]    (1–4000)
 ├─┼─┼─┼─┼─┼─┼─┼─┤
 └─┴─┴─┴─┴─┴─┴─┴─┘   3 × 3
 [ Layout… ]                          [ Cancel ] [ Insert ]
```

**The Table menu (TD1)** holds Ami Pro's items and the additions below:

```
 Table
   Insert Table…                Ctrl+F12
   ──────────
   Insert Row Above
   Insert Row Below
   Insert Column Left
   Insert Column Right
   Delete Row
   Delete Column
   Delete Table
   ──────────
   Select Cell / Row / Column / Table ▸
   ──────────
   Connect Cells                 (Disconnect Cells in a joined cell)
 ✓ Headings
   Leaders ▸        ✓ None  ...  - - -  ___
 ✓ Protect Cells
   Distribute Rows Evenly
   Distribute Columns Evenly
   ──────────
   Convert Text to Table / Convert Table to Text
   ──────────
   Table Properties…             (the rail)
```

Every item is checked or unchecked as its state is, Leaders too (Ami Pro
has no check marks there). The same items for rows, columns, and cells are
on the right-click menu in a table.

**Table Properties in the rail (TD2)**, as Frame Properties and Page
Layout Properties: folding sections, each change one undo step, following
the caret's table and its cell selection:

```
┌ Table Properties ──────────────── × ┐
│ ▾ Table                             │  Modify Table Layout
│   Style [Grid][Outline][Header][Banded][Plain] │  TD7
│   Default column  [2.10] in  gutter [0.06] │
│   Default row  [x] Automatic  [0.20] in    │
│                 gutter [0.06]              │
│   [ ] Rows span pages                      │
│   [ ] Center on page                       │
│   [ ] Line around  [ ───── ▾ ]             │
│   [ ] Honor protection                     │
│   [ ] Disable mouse sizing                 │
│ ▾ Rows and columns   (row 2, column 3)     │  Column/Row Size
│   Width  [1.50] in   gutter [0.06]  [Reset]│  "Mixed" when they differ
│   Height [auto ]     gutter [0.10]  [Reset]│
│   [ Distribute rows ] [ Distribute columns]│
│   Insert [1] [Rows ▾] [Above] [Below]      │  Insert Column/Row…
│ ▾ Cells   (B2:C3)                          │
│   Lines  [■ All][□ Outline][◧][◨][⬒][⬓]    │  Lines & Color
│          [ ───── ▾ ] [■ ▾]  [ Remove lines ]│  style, colour (TD8)
│   Fill   [■ ▾]  (the colour panel, §23)    │
│   Leader [None ▾]   [ ] Protected          │
│   [ Connect cells ]                        │
└────────────────────────────────────────────┘
```

- Values come from the caret's cell or the selected cells; a field the
  selection disagrees on shows "Mixed" and leaves those cells as they are
  until it is set (Ami Pro's grey check boxes). Reset returns a row or
  column to the table's default (removes its own size).
- Lines: All, Outline, and each side are toggles with Ami Pro's three
  states (on, off, mixed); the style list shows Ami Pro's 12 line styles
  drawn; Remove lines clears every side of the selected cells. A line
  wider than its gutter widens the gutter, as Ami Pro does.
- Fill uses the colour button and panel of §23 (Ami Pro's 15, the
  document's colours, Recent, the picker).
- Opening: Table › Table Properties…, a double-click on a grid line, the
  right-click menu's Table Properties…; the rail returns to Styles when the
  caret leaves the table.

**Selecting cells (TD3).** Ami Pro's gestures: a drag from one cell into
another, Shift+click, Shift+Ctrl+arrows, a press on the strip above a
column or left of a row (the pointer becomes an arrow there), and Table ›
Select ▸. Shift+arrows that cross a cell's edge also turn the text
selection into a cell selection. The cells are drawn selected whole
(gutters included), with the accent colour as text selections are.

**Rows and columns (TD4, TD6).**
- Insert Row Above/Below and Column Left/Right add as many rows or columns
  as are selected (one with a caret), at the table's defaults, copying the
  lines and fill of the row or column beside them (Ami Pro's rule).
- A column that does not fit: Ami Pro refuses it; here the table keeps its
  width and its columns narrow in proportion to make room (TD4), and the
  status bar says so.
- Delete Row/Column removes the caret's or the selected ones; deleting
  them all deletes the table. No question: Undo brings them back (TD10).
- **Grid lines** can be dragged (unless Disable mouse sizing is on): a
  column line changes that column and moves the rest, as Ami Pro does
  (TD6). Row lines likewise, when Automatic is off.
- **The ruler** shows the table's columns while the caret is in a table,
  and its column markers drag as the grid lines do (TD6).

**Keys (TD5).** Tab and Shift+Tab move between cells (skipping hidden and,
with Honor protection, protected cells); **Tab in the last cell adds a
row** (TD5), and Ctrl+Down there leaves the table; Ctrl+arrows move a cell
at a time; Home, Home and End, End go to the row's first and last cell;
Esc leaves the table. Alt+Up and Alt+Down move the caret's row (or the
selected rows) up or down, Ami Pro's keyboard move.

**Clipboard and drag and drop (TD9).** Delete empties the selected cells
(rows and columns stay); Copy and Cut take the cells (and a plain-text
version with tabs and returns); Paste fills cells from the caret's,
repeating a copied block over a larger selection as Ami Pro does, adding
rows to a page table that needs them; cells pasted outside a table are
tabbed text, a whole copied table a new table; tabbed text pasted into a
table fills its cells. A selection of whole rows or columns, or a block,
drags to a new place (Ctrl copies).

**Additions (TD7).** Distribute Rows Evenly and Distribute Columns Evenly
(the selected ones, or the table's: own sizes made equal within their
total); Convert Text to Table (the selected paragraphs, a column per tab)
and back (cells to paragraphs with tabs); **table styles**: Table ›
Table Style ▸ and a row of previews in the rail's Table section (Grid,
Outline, Header row, Banded rows, Plain), each one undo step applying Ami
Pro's lines and fills to every cell (a header row also a heading row).
Everything here is stored in Ami Pro's terms (sizes, rows, text, lines,
fills), so Save As SAM lists nothing new.

**What SAM stores.** Every command writes what Ami Pro writes: the
header's defaults and value 7, `[h]`/`[w]` records with their flags,
cell records (joins with the hidden cells' text, leaders in F3, protection
in F9, lines in F7, fill in F3 `0x200`, F6, F10), the table frame (Line
around), the anchor in the text, Table Text made from Body Text. Line
**Line colour (TD8):** the Lines row of the rail has a colour button
(§23) for the selected cells' lines; black is Ami Pro's and the default.
Another colour is kept in the native format only; Save As SAM writes the
lines black and lists "table line colours" in the loss check.

### Questions for the user

TD1 where the commands live; TD2 the rail panel in place of Ami Pro's
dialogs; TD3 the selection gestures; TD4 a column with no room; TD5 Tab in
the last cell; TD6 dragging grid lines and the ruler; TD7 additions; TD8
line colour; TD9 the clipboard and drag and drop in this run; TD10
questions before deleting; TD11 one run or two.

### Implementation notes (2026-10-24, the table editing run)

Built as approved (`docs/TABLE_EDIT_PROGRESS.md`, the run's choices
TK1–TK11 in `DECISIONS.md`):

- **Model:** `Table::layout` (`TableLayout`: the default row height and
  column width, Center, Honor protection, Disable mouse sizing, own sizes)
  and `TableCell::format` (`CellFormat`: lines, line colour, fill, leader,
  protection), in native v3 (`tablelayout`, `cellformat`) and SAM as Ami
  Pro writes them. A cell's fill is F3 `0x200` with F10, black too (TK1).
- **Commands** (`amipro_editing::tables`): `Command::InsertTable`,
  `Command::EditTable` (`TableEdit`: rows and columns, sizes, Reset,
  Distribute, Connect and Disconnect, Headings, cell formats, Clear,
  Move and copy of rows and columns, table styles, Paste of cells),
  `Command::TextToTable`, `Command::TableToText`; each one undo step.
- **Interface:** the Table menu (after Format; Insert › Table… stays in
  Insert, TK8), its right-click items in a table, Insert Table's dialog
  (a grid of sizes, Layout…), Table Properties in the rail (`table_properties.rs`),
  cell selections (`table_editing.rs`), the table keys, grid lines and the
  ruler's table columns (TD6), Copy, Cut, Paste, Delete, and drag and drop
  of cells (`table_commands.rs`).
- **Layout:** leaders drawn by the text's alignment, line colours, Center
  table on page, edited Automatic rows fitting their text, a table always
  on a line of its own (TK11).
- **Not built:** Home, Home and End, End (TK7); a literal tab in a cell
  (Ami Pro swallows typed tabs in tables).

## 33. Drawings and Draw mode — proposed 2026-10-25; APPROVED by the user 2026-10-25 (DW1–DW9)

**Status: approved** (`DECISIONS.md`, 2026-10-25, "Drawing: the user's
answers"): as recommended, except that the draw bar replaces the toolbar
in Draw mode (DW3). Prepared for the Phase 5 queue's run 9, `docs/playbooks/GRAPHICS.md`
"Run 2"), from the decompiled `AmiDraw.dll`, `DrawMan.dll`, and `AMIPRO.EXE`
and Ami Pro 3.1 fixtures DR1, DRBIG, DRW0–DRW3
(`docs/research/DRAWING_NOTES.md`). Today a drawing is a picture the app
cannot read: it shows Ami Pro's screen image where the file has one (142 of
the user's 380 drawings) and a labelled box otherwise; Insert › Drawing… is
a Future command.

### What Ami Pro 3.1 does

- **Drawings are objects** in a `.sdw` stream (lines, polylines, polygons,
  rectangles, rounded rectangles, ellipses, arcs, text, groups, bitmaps),
  in twips, one to one with the page; the frame shows them by its picture's
  scaling, like any picture.
- **Tools › Drawing** makes a frame at the caret (or uses the selected
  empty frame) and enters **Draw mode** in place: an icon bar of ten tools
  and thirteen commands across the document window, a Draw menu after
  Tools, Import Drawing… and Save As Drawing… in File, and Undo, Cut, Copy,
  Paste taken over. A double-click on a drawing opens it; Esc or a click
  outside leaves.
- **Line Style**, **Fill Pattern**, **Rotate**, and **Grid Settings** are
  dialogs; Extract and Apply Line & Fill copy one object's look to others;
  Flip, Group, Bring to Front, Send to Back, Select All, Snap To, Show Grid.
- Mouse only: no keyboard drawing, no exact sizes, no alignment. One level
  of undo, toggling.

### Proposal

**Showing drawings.** Every drawing (and every chart, whose look is
ordinary objects) is drawn from its objects on screen, in PDF (as vectors),
and in exports, exactly as Ami Pro paints them (fills, hatches, endings,
one-pixel styled lines, text spread to its box), sharp at every zoom. Ami
Pro's screen image is no longer needed.

**Making a drawing.** Insert › Drawing… (Ami Pro's Tools › Drawing; also a
toolbar button) makes Ami Pro's frame at the caret (the remembered Create
Frame size, Flow with text, the default frame style), or uses a selected
empty frame or an empty table cell, and opens Draw mode. A double-click on
a drawing (or Enter on a selected drawing frame) opens it; Esc or a click
outside closes it.

**Draw mode, in place (DW2).** The frame stays on the page at the
document's zoom; the page around it dims slightly. A **draw bar replaces
the toolbar** while Draw mode is open (DW3: no page jump, the whole width;
shown even with View › Toolbar off), with Undo, Redo, and the text
objects' font face, size, bold, italic, and underline besides Ami Pro's
tools and commands:

```
 ┌───────────────────────────────────────────────────────────────────────┐
 │ ↶ ↷ │ ↖ ✋ ╲ 〰 ⬠ ▭ ▢ ◯ ◜ A │ ⊞ ⧉ ⬆ ⬇ ⟳ ⇋ ⇵ │ ⌗ ⧄ │ ⎘ ⎗ │ ─ ▾ ▦ ▾ │ Arial▾ 12▾ B I U │ Done │
 └───────────────────────────────────────────────────────────────────────┘
  undo   tools (Ami Pro's ten)  Select All, Group, Front, Back, grid, extract, line,  text objects' font   leave
                                Rotate, Flip H, Flip V          snap  apply    fill
```

**Drawing Properties in the rail (DW2)**, replacing Line Style, Fill
Pattern, Rotate, and Grid Settings with folding sections, live, each change
one undo step, for the selection (or the defaults for new objects when
nothing is selected):

```
┌ Drawing Properties ─────────────── × ┐
│ ▾ Line                               │  Line Style
│   [ ─── 1 pt ▾ ]  [■ ▾]  (colour §23)│  ½–12 pt, dotted, none
│   Dash [solid ▾]                     │  Ami Pro's five dashes
│   Start [ none ▾ ]  End [ ◀ ▾ ]      │  the nine endings, drawn
│ ▾ Fill                               │  Fill Pattern
│   [□][▒][▓][■][≡][‖][╲][╱][#][╳]     │  none, dots, solid, hatches
│   Colour [■ ▾]  Background [□ ▾]     │
│ ▾ Position and size        (DW5)     │
│   X [1.25] Y [0.40] in               │
│   W [2.00] H [1.10] in  [x] keep ratio│
│   Angle [ 0 ]°  [⟲ 90] [⟳ 90]        │  Rotate
│   [Align ▾] [Distribute ▾]           │
│ ▾ Text                               │  the text objects' font
│   [Arial ▾] [12 ▾] [B][I][U]         │
│ ▾ Grid                               │  Grid Settings
│   [x] Show  [x] Snap  [ ¼ in ▾ ]     │
│   [Dots ▾]                           │
└──────────────────────────────────────┘
```

**A Draw menu (DW4)**, always present after Table, enabled in Draw mode:
Line and Fill (the rail), Extract / Apply Line & Fill, Rotate Left /
Right / by…, Flip ▸, Group / Ungroup, Bring to Front, Send to Back, Bring
Forward, Send Backward, Align ▸, Distribute ▸, Snap To ✓, Show Grid ✓,
Select All, Import Drawing…, Save Drawing As…. The same items for the
selection are on the right-click menu in Draw mode.

**Gestures: Ami Pro's** (DRAWING_NOTES.md, "Draw mode"): each tool's drag
and clicks, Shift's constraints, freehand, the tool staying, Shift+click,
Ctrl+click, the marquee, handles, flips past a side and by double-click on
a handle, point editing by double-click (rotation arrows, the centre of
rotation, points added and deleted), the Hand pan, text objects typed in
place. **Additions (DW5):** arrow keys nudge (Shift ×10, Alt one pixel),
Ctrl+D duplicates, Delete, Ctrl+A, Tab cycles objects, Esc steps out;
smart guides (edges and centres of other objects); every colour (§23) and
any width up to 12¾ pt; true curves rather than Ami Pro's 16-piece ones.

**Undo.** Every change is one step on the document's history (gesture,
property, command); leaving Draw mode is not a step. Ami Pro had one
toggling level.

**Files and clipboard (DW6).** Draw › Import Drawing… adds a `.sdw`
drawing's objects, an SVG's shapes (as polygons, polylines, curves, and
text), or a picture (as a bitmap object); Save Drawing As… writes `.sdw`,
SVG, or PNG (the selection only, optionally). Copy puts the selected
objects on the clipboard as the app's own objects, SVG, and PNG; Paste
takes them back, or SVG, an image, or text (a text object per paragraph,
Ami Pro's rule).

**What SAM stores.** Everything above: objects as Ami Pro writes them
(points, pens, fills, endings, text, groups, bitmaps), version 0x0102, no
driver state and no rendering (Ami Pro rebuilds both; DRW0). Colours are
RGB, widths whole twips up to 255. Nothing new for the loss list.

**Charts (DW7)** are drawn from their objects and kept byte for byte;
opening one is the charts run's. **Leaving (DW8, DW9):** Esc, Done, or a
click outside the frame (which then acts there); the frame keeps its size
(Ami Pro's rule), and Fit Frame to Picture sizes it on request.

### Questions for the user

DW1 scope; DW2 where Draw mode edits; DW3 the draw bar's place; DW4 the
Draw menu; DW5 additions; DW6 files and clipboard; DW7 charts; DW8 the
frame when leaving; DW9 a click outside. Answered 2026-10-25
(`DECISIONS.md`).

### Implementation notes (2026-10-25, the drawing run)

Built as approved (`docs/DRAWING_PROGRESS.md`; the run's choices DK1–DK13
in `DECISIONS.md`):

- **The format:** `amipro_pictures::sdw` reads and writes Ami Pro's drawing
  stream (all 487 drawings in both corpora, the VM folder, and the shipped
  symbols, byte for byte but for runtime links), builds shapes as Ami Pro
  builds them (DR2B), moves, sizes, flips, and turns objects, gives the
  natural size, and paints into the metafile display list (fills, dot
  patterns, hatches, one-pixel styled lines, endings, true curves, text
  spread to its box and turned, bitmaps). `amipro_pictures::svg` writes a
  display list as SVG.
- **Showing:** every drawing and chart is drawn from its objects on screen,
  in PDF (as vectors), and in exports (SVG with a PNG); turned text is
  drawn on screen and in PDF.
- **Commands:** `Command::SetDrawing` (one step; the stale screen image and
  driver state dropped, as Ami Pro rebuilds both, DRW0) and
  `amipro_editing::drawing`'s operations.
- **Interface:** Insert › Drawing… (Ami Pro's frame at the caret, an empty
  frame, an empty cell); Draw mode by Enter or a double-click on a drawing
  (`draw_mode.rs`: tools, gestures, selection, handles, point editing,
  text, keys, smart guides, the overlay); the draw bar in the toolbar's
  place (`draw_commands.rs`; the app's own icons); the Draw menu after
  Table, always present, and its right-click items; Drawing Properties in
  the rail (`drawing_properties.rs`); Import Drawing…, Save Drawing As…,
  Copy, Cut, Paste (`draw_files.rs`). Charts are drawn, not opened (DW7).
- **Not built:** Graphics Scaling's Rotate of a drawing still turns the
  picture (no file has one turned); charts' editing (the charts run).

## 34. Notes, bookmarks, footnotes, and endnotes — proposed 2026-10-26; APPROVED by the user 2026-10-26 (NR1–NR10)

**Status: approved** (`DECISIONS.md`, 2026-10-26, NR1–NR10, all as recommended), for the Phase 5 queue's run 10
(`docs/playbooks/FIELDS_AND_REFERENCES.md`, "Run 2"), from the decompiled
`AMIPRO.EXE` and `AMIPROUI.DLL` and Ami Pro 3.1 fixtures RNN1–RNN3,
RNB1–RNB3, RNF1–RNF6, RNF71S–RNF74S
(`docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`). Today all three are kept
marks written back verbatim: nothing shows where Ami Pro shows a note mark
or a footnote, a document's footnotes are invisible, and Insert ›
(Superseded: Insert › Footnote and Endnote are built.)

### What Ami Pro 3.1 does

- **Notes** (Edit › Insert › Note): a coloured box with a black outline at
  a position in any text (main, frame, table, header, footer, footnote),
  taking no width, shown when View Preferences › Notes is on. Each note
  keeps its author's initials, colour (User Setup), time, and number (1, 2,
  3 in the main text; separately in frames and tables; 0 in headers,
  footers, and footnotes). A double-click opens a small floating window in
  the note's colour holding plain text (the formatting of a stored note is
  lost when it is edited), titled with the initials, number, date, and
  time; its control menu has Remove This Note and Remove All Notes (not
  undoable). Display initials in text shows `[LS1]` after each mark. Print
  › With notes prints the `[LS1]` marks and the notes on pages at the end,
  grouped by page. Find & Replace and word count skip notes.
- **Bookmarks** (Edit › Bookmarks…, a modal dialog: a name, a sorted list,
  Go To, Add, Remove): a selection or the caret in any text (a pair of
  invisible marks), a block of cells, one cell, or a frame (its name). Names
  are case-insensitive, without spaces, up to 19 characters (the guide says
  17 and no leading digit); bookmarks cannot nest. Go To selects the text,
  cells, or frame; Edit › Go To's Next item › Bookmark moves to the next
  text bookmark. Power fields use them (Bookmark, PageRef, GetBookMark,
  Include file!bookmark).
- **Footnotes** (Tools › Footnotes…: Insert footnote or Edit footnote /
  Return to Text, Options…): the reference at the caret in the main text or
  a table cell; the text in the Footnote style (made from Body Text the
  first time), edited in place in the area at the page's foot, under a
  separator line; only the first paragraph shows the number, which is the
  Footnote style's bullet number (1, I, i, A, a, `*`, `†`, with its text and
  attributes, superscript by default). A footnote follows the line that
  refers to it; when the separator and one line no longer fit, that line
  moves to the next page (RNF72); a long footnote splits across pages, with
  optional "Continued on" and "Continued from" messages and the number
  repeated (RNF5). In a table, the footnote goes in a joined row added at
  the table's end, numbered within the table. Footnote Options…: Make
  endnotes (the whole document's footnotes on pages after the text, without
  a separator), Reset number on each page, Starting number, the separator's
  length (margins or custom) and indent, the two messages and their
  alignment, Repeat footnote number. Deleting a reference asks first, then
  removes the footnote.

### Proposal

**Notes: marks in the text, the notes in the rail (NR1).** The mark is
drawn as Ami Pro's (the note's colour, a black outline, no width), with
View › Notes (on by default, saved) and View › Note Initials (Ami Pro's
Display initials in text). A click on a mark opens the **Notes panel** in
the rail at that note; Insert › Note (Ctrl+Alt+M) inserts one at the caret
in any text and opens it there, focused:

```
┌ Notes ─────────────────────────── × ┐
│ You: [LS ] [■ ▾]  (initials, colour) │  Ami Pro's User Setup
│ ‹ Previous   3 notes   Next ›        │
│ ┌─────────────────────────────────┐ │
│ │■ LS1  26 Oct 2026, 09:54  p. 1  │ │  number, date, page; the colour
│ │ First note text.                │ │  plain text, edited in place
│ │                    [Remove]     │ │
│ ├─────────────────────────────────┤ │
│ │■ LS2  26 Oct 2026, 09:55  p. 1  │ │  click: the caret to its mark
│ │ Second note.                    │ │
│ └─────────────────────────────────┘ │
│ [Remove All Notes]                   │
└──────────────────────────────────────┘
```

Each note is plain text (NR2), as Ami Pro's window gives it; a stored
note's formatting is kept until it is edited. Typing in a note is one undo
step while keys follow (as text typed on the page), Remove and Remove All
are undoable, and a note's colour can be changed (§23's colour button).
The note's time is set when its text changes; dates show four-digit years.
Deleting text that holds a note asks first (Ami Pro's message), and the
deletion is one undo step. Notes are numbered as Ami Pro numbers them, and
Edit › Go To's Next item gains Note.

**Notes out (NR3).** Print Options gains With notes (Ami Pro's: the
`[LS1]` marks in the text and the notes on pages at the end, grouped by
page); the PDF does the same and also carries each note as a PDF text
annotation at its mark; DOCX, ODT, and RTF exports carry notes as comments
(and import already reads comments as notes).

**Bookmarks: a Bookmarks panel in the rail (NR4).** Edit › Bookmarks…
(Ctrl+Shift+F5) opens it, filled with the selected frame's name, or the
selected text when it makes a valid name:

```
┌ Bookmarks ──────────────────────── × ┐
│ Name [ totalsales          ] [Add]   │  Add, or Go To when it exists
│ ┌──────────────────────────────────┐ │
│ │ ¶ intro          text   p. 1     │ │  sorted; double-click: Go To
│ │ ▦ prices         cells  p. 2     │ │
│ │ ▭ logo           frame  p. 1     │ │
│ └──────────────────────────────────┘ │
│ [Go To] [Rename] [Remove]            │
│ [x] Show bookmarks on the page        │  View › Bookmarks (NR5)
│ [Insert Cross-Reference…]             │  (NR6)
└──────────────────────────────────────┘
```

Add works on a selection or the caret in any text, a block of cells, a
cell, or one selected frame, as Ami Pro's; Go To selects the text, cells,
or frame and scrolls to it; Remove leaves the text; Rename is an addition
(SAM stores the new name). Names follow Ami Pro's rules (NR7): no spaces,
up to 19 characters, not starting with a digit, unique whatever the case;
a refused name says why under the field. No nesting (Ami Pro's message).
Edit › Go To's Next item gains Bookmark, with Next and Previous. Each Add,
Remove, and Rename is one undo step (Ami Pro undoes none).

**Seeing bookmarks (NR5).** View › Bookmarks (off by default, saved)
draws thin grey brackets at a text bookmark's ends (a single I-beam for an
empty one) and a grey outline around bookmarked cells and frames; never
printed.

**Cross-references (NR6), an addition.** Insert › Cross-Reference… (also
in the panel) inserts a power field for a bookmark's page (`PageRef
name`) or its text (`Bookmark name`), worked out with the other fields
(F9, Update All, before printing). The fields run kept these fields'
results until bookmarks existed; they now work out, and so do
GetBookMark and Include file!bookmark. In the PDF, each bookmark is a
named destination and each cross-reference a link to it; DOCX and ODT
exports write bookmarks and the fields as their own.

**Footnotes: in place on the page (NR8).** Insert › Footnote (Ctrl+Alt+F)
puts the reference at the caret in the main text or a table cell and the
caret in the new footnote, in the area at the page's foot (in a table, the
joined row at its end, as Ami Pro's); a click in a footnote edits it, and
Esc, or Return to Text, goes back to the reference. A double-click on a
reference goes to its footnote and on a footnote's number back to its
reference; Edit › Go To's Next item gains Footnote Mark and Footnote
Text; Up and Down move between footnotes, and a selection stays in one
footnote. Everything is laid out as Ami Pro lays it out: one full-width
area per page growing up from the bottom margin, the separator, lines
following their reference, the referencing line moving to the next page
when the separator and a line do not fit, long footnotes split with the
messages, pending footnotes on pages of their own, endnotes after the text
on a new page. Deleting a reference asks first, and removes the footnote
in the same undo step. The status bar names the footnote ("Footnote 2").

**Endnotes (NR9).** As in Ami Pro, endnotes are one choice for the whole
document (SAM has nothing else): Insert › Endnote leaves the menu, and the
Footnotes panel's Make endnotes moves every footnote to the end (and
back), one undo step.

**The Footnotes panel (NR10).** Tools › Footnote Options… (and the
reference's right-click menu) opens it in the rail, live, each change one
undo step:

```
┌ Footnotes ──────────────────────── × ┐
│ [ ] Make endnotes                     │
│ [ ] Reset number on each page         │
│ Starting number [ 1 ]                 │
│ ▾ Separator line                      │
│   (•) Margins  ( ) Custom [2.00] in   │
│   Indent from left [0.00] in          │
│ ▾ Continued messages                  │
│   [x] On   [Continued on next page ] (•)Left ( )Right │
│   [x] From [Continued from previous] ( )Left (•)Right │
│   [ ] Repeat footnote number          │
│ Number style… (Style Properties at the│
│   Footnote style's Bullets and numbers)│
│ ‹ Previous  4 footnotes  Next ›       │
└──────────────────────────────────────┘
```

with Ami Pro's dialog rules (Make endnotes disables the rest, Reset
disables Repeat, a message's alignment only with its message on).

**What SAM stores.** Everything above but the additions' settings: notes
(`<:N` with five fields and the note's paragraphs), text and cell-range
bookmarks (`<:Z…>`), one-cell bookmarks (`[book]`), frame bookmarks
(`[frmname]`), footnotes (`<:F`, the Footnote style), table footnotes
(`<:x`, the row), and `[fopts]`, all as Ami Pro writes them. Cross-
references are power fields, which SAM stores. View › Notes, Note
Initials, Bookmarks, and the user's initials and colour are settings, not
the document's.

### Questions for the user

NR1 where notes are read and edited; NR2 note text; NR3 notes in print,
PDF, and exports; NR4 the bookmarks interface; NR5 seeing bookmarks; NR6
cross-references; NR7 bookmark names; NR8 where footnotes are edited; NR9
endnotes; NR10 footnote options. Answered 2026-10-26, all as recommended
(`DECISIONS.md`).

### Implementation notes (2026-10-26, the notes run)

Built as approved (`docs/NOTES_PROGRESS.md`; the run's choices NK1–NK10 in
`DECISIONS.md`):

- **The model:** `InlineObject::Note` (`NoteInfo`: time, initials, colour,
  number; its text a flow), `Footnote` (a flow), `TableFootnote` (number,
  row); `FootnoteOptions` on the document; bookmarks in Ami Pro's stored
  forms with `amipro_document::bookmarks` as their typed view (NK1).
- **Layout:** `amipro_layout` engine's `footnotes` module (the area, the
  100-twip separator, splits and messages, endnotes, numbers); a
  paragraph's opening hard return makes no empty line (NK10).
- **The page:** note marks (5h/9 × 5h/6 in the note's colour), the
  separator, superscript references, View › Notes and View › Bookmarks;
  footnotes edited in place, Esc back, double-click jumps.
- **Commands:** Insert › Note (Ctrl+Alt+M), Footnote (Ctrl+Alt+F; in a
  table's cell Ami Pro's joined row), Cross-Reference…; Edit › Bookmarks…
  (Ctrl+Shift+F5), Notes…; Tools › Footnote Options…; Edit › Go To's Next
  item (Bookmark, Note, Footnote mark, Footnote text) with Previous and
  Next; Insert › Endnote left the menu (NR9).
- **The rail:** `references_panel.rs` (Notes, Bookmarks, Footnotes) and
  `reference_commands.rs`; typing in a note joins its keys into one step.
- **Out:** Print Options › With notes (note pages under their pages'
  numbers); the PDF's named destinations for bookmarks; DOCX, ODT, RTF
  footnotes, endnotes, comments, bookmarks, read back by the importers as
  typed objects. Not built: `[LS1]` in the text (NK4, NK8), PDF sticky
  notes (NK7, krilla has none), cross-reference links in the PDF.

## 35. Proofing: spelling, thesaurus, grammar, language — proposed 2026-10-26; APPROVED by the user 2026-10-26 (SP1–SP8)

**Status: approved** (`DECISIONS.md`, 2026-10-26, SP1–SP8, all as recommended but SP1: en-US is also bundled) for the Phase 5 queue's run 11
(`docs/playbooks/PROOFING.md`, "Run 2"), from the decompiled `AMIPRO.EXE`,
`AMIPROUI.DLL`, and Lotus's proofing modules (`LTSSPL1.DLL`,
`LTSTHES1.DLL`, `LTSGRM1.DLL`), and Ami Pro 3.1 fixtures PSP1–PSP4B, PGR1,
PLN1 (`docs/research/PROOFING_NOTES.md`). (Superseded: Spelling…, Spelling as You Type, Thesaurus…, and Language… are built.) When written they were Future commands;
`[lang]` and `[GramStyle]` are kept and written back as read.

### What Ami Pro 3.1 does

- **Spell Check** (Tools › Spell Check…): a small dialog first (Check
  from beginning of document, Include other text streams, Language
  Options…, Edit Dictionary…, Options…: repeated words, words with
  numbers, words with initial caps, user dictionary alternatives; the
  options kept in `AMIPRO.INI`), then a correction dialog over the page:
  the word (selected on the page), Replace with, up to ten Alternatives in
  the word's capitals, Replace, Replace All, Skip, Skip All, Add To
  Dictionary, Cancel, and the stream being checked ("Checking Main
  Document Text"). With a selection, only the selection is checked and the
  first dialog is skipped. It walks the text streams in Ami Pro's order
  from the caret (or the beginning) to the end, never wrapping. A repeated
  word ("the the") is announced by a message box; an empty Replace with
  deletes it and the space before it. Replace inserts the box's text as
  typed; Replace All replaces the same word in the same capitals silently
  as the check reaches it later; Skip All and Replace All last one check.
  Ends with "Spell check complete." in the status bar. Edit can undo only
  the last replacement. A paragraph checked clean is flagged in the file
  (`<:s>`) and skipped by later checks until it is edited.
- **The user dictionary** is a plain text file, one word per line
  (`LTSUSER1.DIC` for American English), read at the start of a check and
  written at its end; Edit Dictionary opens it as a document.
- **Languages**: one per document (`[lang]`, a number 1–28: 1 American, 2
  British, 3 French, 27 British IZE, …; 0 none), chosen in Language
  Options with the default for new documents; no language per paragraph or
  style. Only languages whose lexicon is installed are listed.
- **Thesaurus** (Tools › Thesaurus…): the word at the caret (selected),
  its meanings (part of speech and a definition), synonyms for each, in
  the word's capitals and inflection ("sat" gives "posed"); Replace with,
  Lookup (a synonym, kept in a history), Previous, Replace (the dialog
  stays open).
- **Grammar Check** (Tools › Grammar Check…): a rule set (Business
  Writing by default; All Rules, Legal, Technical, Fiction, Academic,
  Formal, Casual Writing), Show readability statistics, Show explanations,
  Check from beginning, Include other text streams, Check in draft mode;
  Options… edits a set's 20 grammar rules, 20 style rules, and three word
  order rules. Each problem selects its sentence and shows suggestions
  (the words in red), an explanation, and the sentence rewritten as
  replacement options; Skip, Skip Rule, Next Sentence, Replace (which
  checks the rewritten sentence again), Cancel. At the end, Readability
  Statistics: words, sentences, paragraphs, syllables, three-syllable
  words, averages, passive sentences, Gunning's Fog Index, Flesch-Kincaid,
  Flesch Reading Ease and its grade. The set used is saved in the document
  (`[GramStyle]`).

### Proposal

**One Proofing panel in the rail (SP3).** Tools › Spell Check… (F7),
Thesaurus… (Shift+F7), and Grammar Check… open it at their page; the
document stays live, the current word or sentence is selected and
highlighted on the page as the find bar's matches are, and the panel
follows edits made meanwhile:

```
┌ Proofing ─────────────────────────── × ┐
│ [Spelling] [Thesaurus] [Grammar]        │  pages (a segmented button)
│ Not in dictionary: recieve              │
│ Replace with [ receive            ]     │
│ ┌─────────────────────────────────────┐ │
│ │ receive                             │ │  double-click or Enter: Replace
│ │ received                            │ │  (the word's capitals)
│ │ receiver                            │ │
│ └─────────────────────────────────────┘ │
│ [Replace]  [Replace All]                │
│ [Skip]     [Skip All]                   │
│ [Add to Dictionary]                     │
│ Main text · 2 of 7 found                │
│ ▸ Options (from the beginning, other    │
│   text streams, repeated words, words   │
│   with numbers, initial caps, user      │
│   dictionary alternatives)              │
│ ▸ Language  [English (US)        ▾]     │
│ ▸ User dictionary  (12 words) [Edit…]   │
└─────────────────────────────────────────┘
```

Ami Pro's options and their defaults; with a selection, only the
selection is checked; repeated words show "Repeated word" with Delete
Repeat (an empty Replace with, as Ami Pro); the end says "Spell check
complete." in the status bar and the panel shows the count found.

**Replacing (SP4).** Each Replace is one undo step. Replace All replaces
every occurrence of the word in the check's range at once, keeping each
occurrence's capitals (Teh → The, teh → the), as one undo step with the
count; Skip All ignores the word for the rest of the session in this
document, shared with spelling as you type. (Ami Pro: forward only, exact
capitals, one check, no undo.)

**Spelling as you type (SP2).** Tools › Check Spelling as You Type (on by
default, saved): unknown words get a red wavy underline (repeated words
too), never printed; checked after a short pause, paragraph by paragraph,
off the typing path. The right-click menu on an underlined word starts
with its suggestions, then Ignore All, Add to Dictionary, and Spell
Check….

**Dictionaries (SP1).** Spelling uses the Hunspell dictionaries installed
on the system (`/usr/share/hunspell`, also the user's
`~/.local/share/hunspell`), read with `spellbook` (MPL-2.0, pure Rust);
the free en-US dictionary (SCOWL) is also bundled, so English always
works (SP1, the user's choice). A language without a dictionary says so
in the panel, with the package to install.

**The user dictionary (SP5).** One per user and language, in the app's
data folder, in Ami Pro's format (one word per line), written as words
are added. The panel's User dictionary section lists it (add, remove),
with Import… (Ami Pro's `LTSUSER1.DIC` or any word list) and Export….

**Language (SP6).** The document's language, stored in `[lang]` (Ami
Pro's numbers mapped to locales: 1 en-US, 2 en-GB, 27 en-GB with -ize, 3
fr-FR, 4 fr-CA, 6 es, 7 de-DE, 12 pt-PT, 17 pt-BR, 18 en-AU, …), chosen in
the panel and in Tools › Language… (one undo step), from the languages
with an installed dictionary; the default for new documents in the
settings. A language Ami Pro has no number for is kept in the native
format; Save As SAM writes the nearest and lists it. Hyphenation follows
the document's language where `hypher` has its patterns (today English
only).

**Thesaurus (SP7).** The panel's Thesaurus page as Ami Pro's dialog: the
word at the caret (selected), its meanings with the part of speech,
the synonyms of the chosen meaning in the word's capitals and inflection,
Replace with, Look Up, Back (the history), Replace (the panel stays).
Data: the MyThes thesauri installed on the system (LibreOffice's,
`/usr/share/mythes`), read by the project's own reader. The right-click
menu gains Synonyms ▸ with the first meaning's synonyms.

**Grammar (SP8).** The panel's Grammar page as Ami Pro's Grammar
Checker: the rule set (Ami Pro's eight names, kept in `[GramStyle]`),
the sentence selected, the problem with the words in colour and an
explanation, replacement options, Skip, Skip Rule, Next Sentence,
Replace; the options as Ami Pro's. The checker is Harper (`harper-core`,
Apache-2.0, offline, English only), its rules grouped under Ami Pro's
rule names where they match; a language other than English says grammar
is not available. Readability statistics by Ami Pro's formulas (its
figures for PGR1 reproduced) at the end of a check and from Tools ›
Readability Statistics…. Grammar underlines as you type (blue) are
offered, off by default.

**Also.** The status bar's word count counts the selection when there is
one ("12 of 1,204 words"). Edit Dictionary opens the panel's list.

**What SAM stores.** `[lang]` (Ami Pro's numbers), `[GramStyle]` (the
set's name), and `<:s>` (the checked flag: the app drops it from any
paragraph it changes, so Ami Pro checks that paragraph again). The user
dictionary, Skip All, and the settings are not the document's.

### Questions for the user

SP1 dictionaries; SP2 spelling as you type; SP3 the panel; SP4 replacing;
SP5 the user dictionary; SP6 language; SP7 thesaurus; SP8 grammar.
Answered 2026-10-26: all as recommended but SP1, where en-US is also
bundled (`DECISIONS.md`).

### Implementation notes (2026-10-26, the proofing run)

- **The panel** (`proofing_panel.rs`, `proofing_commands.rs`): one rail
  panel with three pages (a segmented row); Tools › Spell Check… (F7),
  Thesaurus… (Shift+F7), Grammar Check…, Readability Statistics…, and
  Language… open it at their page; closing another rail panel and
  opening this one follow the Notes panels' rules. Every change to the
  document goes through `execute_frame_commands`: one undo step each
  (Replace, Replace All, Delete Repeat, a synonym, a grammar
  replacement, the language, the grammar set).
- **Spelling** (`amipro_editing::proofing::spelling`; `spelling.rs`): the
  walk starts at the beginning of the caret's word (or the first stream,
  or the selection's start) and goes through the streams in Ami Pro's
  order without wrapping, from the stream it is in; the word is selected
  and scrolled to; alternatives in the word's capitals, ten at most;
  Replace with is the word (empty for a repeat: Delete Repeat); a chosen
  alternative pressed again replaces; Replace All replaces each
  occurrence in its own capitals (SP4). The options are Ami Pro's
  `SpellOpts` bits in the settings. Dictionaries: `~/.local/share/hunspell`,
  `/usr/share/hunspell`, `/usr/share/myspell`, then the bundled en-US
  (`app/cosmic/dictionaries/`), by locale then by language; read on a
  worker thread and kept. User dictionaries:
  `$XDG_DATA_HOME/amipro-linux/dictionaries/LOCALE.dic` in Ami Pro's
  format.
- **As you type**: after every message, a changed document starts a
  350 ms pause (`sync_spelling`); its `Idle` checks every paragraph whose
  text is new (`spell_cache`, by the text's hash) and the page surface
  draws a wavy line under the words (`underlines`; red for spelling,
  blue for grammar; never printed). The right-click menu, at the right
  press: an underlined word's suggestions, Delete Repeated Word, Ignore
  All, Add to Dictionary, Spell Check…; any word's Synonyms ▸.
- **Language**: the drop-down lists every installed dictionary (the
  document's own first when it has none); Use for new documents keeps the
  locale in the settings (`default_language`); hyphenation takes the
  language's patterns (`ShapeRequest::language`; English for Ami Pro's
  0).
- **Thesaurus** (`amipro_editing::proofing::thesaurus`): MyThes files from
  `~/.local/share/mythes`, `/usr/share/mythes`, `/usr/share/myspell/dicts`;
  the index loaded on a worker thread; the word at the caret by Ami Pro's
  rule (`word_at`); base forms and re-inflected synonyms (regular rules
  and irregular verbs); meanings show their part of speech and first
  synonyms (SK3).
- **Grammar** (`grammar.rs`): Harper's curated rules, made once on a
  worker thread, per dialect (en_GB British, en_AU Australian, en_CA
  Canadian); Ami Pro's sets keep Harper's categories as `checks` maps
  them; Harper's spelling lint is never shown. A problem selects its words
  (SK5); Skip Rule skips the category and message for the check; Next
  Sentence goes to the sentence's end; Replace checks the sentence again.
  Readability statistics (`amipro_editing::proofing::readability`) at a
  completed check when the option is on, and from Readability
  Statistics… for the document or the selection.
- **SAM**: `[lang]` and `[GramStyle]` from the model; `<:s>` dropped from
  a paragraph whose text a command changes (`drop_checked_flags`).

## 36. Table of contents, index, and glossary — proposed 2026-11-03; APPROVED by the user 2026-11-03 (TX1–TX8)

**Status: approved** (`DECISIONS.md`, 2026-11-03, TX1–TX8, all as recommended), for the Phase 5 queue's run 12
(`docs/playbooks/FIELDS_AND_REFERENCES.md`, "Run 3"). The evidence is the
decompiled `AMIPRO.EXE` and `AMIPROUI.DLL` and Ami Pro 3.1 fixtures TI1,
TI2, and GLO0 (`docs/research/TOC_INDEX_NOTES.md`). Today the app keeps
index and TOC marks as power fields with nothing to make or edit them,
keeps `[toc]` and `[master]` as read, and has no glossary.

### What Ami Pro 3.1 does

**Marks**
- An index entry is a power field with no result,
  `<:X8,V;Index "primary" # "secondary" "other">`, at a point:
  - `#` means the entry has a page number.
  - Other is text written after the pages (See, See also).
  - Generating the index sets V to the mark's page.
- A TOC entry is `<:X11,V;TOC n "text">`, with level n from 1 to 9.
- Edit › Mark Text ▸ Index Entry… is a dialog that stays open. It has:
  - Primary and Secondary combo boxes, listing the document's entries.
  - Page number, and Other with its text.
  - An example of the result.
  - Mark, Remove, Next Mark, Prev. Mark, Next Same, and Prev. Same.
- Mark Text ▸ TOC Entry… asks for a level (the caret style's TOC level)
  and the text (the selection, up to 240 characters).

**Tools › TOC, Index…**
- It has two check boxes, Table of contents and Index, plus Include
  alphabetical separators and the two output files. The file names and
  the separators setting are kept in `[master]`.
- **TOC Options…** puts each paragraph style in a column from 1 to 9 or
  None. The first state comes from the styles' outline levels.
  - Promote and Demote move a style between columns. Only one style is
    allowed per level.
  - Each level has its own Page number and Right align settings, and a
    Leader (none, dots, hyphens, line) or, without Right align, a
    Separator (none, comma). These are kept in `[toc]`.

**Generating a TOC**
- It takes the main text, and the text of With para above and Flow with
  text frames, in document order.
  - A paragraph in a level's style is an entry, unless it holds TOC marks;
    then only the marks are entries.
  - An entry takes the paragraph's number, its text, and its bold and
    italic.
- It becomes a two-column page table, one row per entry:
  - The entry in TOCn.
  - The page in TOCPGn, with the leader as the cells' leader.
  - Without Right align, the page follows the entry with the separator.
  - Without a page number, the page cell holds a space.
- With no output file, the table goes at the start of the current
  document, followed by a page break, and later page numbers do not count
  the TOC page.
  - TOC1–TOC9 and TOCPG1–TOCPG9 are made in the document. TOCn takes the
    font of the level n+1 style.
  - A second run replaces the rows in place.
- With an output file, it is a new document on `_TOC.STY`.

**Generating an index**
- It always goes into a new document on `_INDEX.STY`, in the styles
  Primary, Secondary, and Separator. Ami Pro saves the source first and
  closes it.
- Entries are sorted by Ami Pro's sort table: case is folded, accents go
  to their base letter, and capitals come first when entries are
  otherwise equal.
- Equal entries are merged. The pages follow ", ", ascending, with no
  repeats and no ranges, and the Other text comes after them.
- A separator is the upper-cased first letter. Each digit gets its own
  separator.

**Glossary**
- A glossary data file is a SAM document of records, `name~text|`, which
  is merge's format. Ami Pro keeps a hidden index after the text.
- Mark Text ▸ Glossary… adds the selection as a named record to the data
  file (the Data File… button chooses an existing file, kept in
  `AMIPRO.INI`). Names are up to 27 characters, with no duplicates.
- Edit › Insert ▸ Glossary Record… (Ctrl+K):
  - When the word before the caret is a record's name, it is replaced by
    the record's text, formatting included.
  - Otherwise a list of names opens, and Insert puts the record at the
    caret.

### Proposal

**One Contents & Index panel in the rail (TX1),** opened by Tools › TOC,
Index… and Edit › Mark Text ▸ Index Entry… or TOC Entry…. It has three
pages:

```
┌ Contents & Index ─────────────── × ┐
│ [Contents] [Index] [Entries]        │
│ ── Contents ──────────────────────  │
│ Levels                              │
│  1 Title          ▸ ◂   ☑ Page ☑ →  │  one style per level; ◂ ▸ promote,
│  2 Subhead        ▸ ◂   ☑ Page ☑ →  │  demote; Page number, Right align
│  3 —  [Add style ▾]                 │  Leader: [ … ▾] for the level
│ Leader [ ..... ▾]                   │
│ Place: (•) Start of this document   │  Ami Pro's blank output file
│        ( ) At the caret             │  (TX2)
│        ( ) New document             │  Ami Pro's output file (_TOC.STY)
│ [ Generate Contents ]  [ Update ]   │  Update replaces it in place
│ ── Index ─────────────────────────  │
│ ☑ Alphabetical separators           │
│ ☐ Join pages into ranges (3–5)      │  TX5, off: Ami Pro's form
│ Place: (•) New document             │  Ami Pro's (_INDEX.STY)
│        ( ) End of this document     │  TX3
│ [ Generate Index ]                  │
│ ── Entries ───────────────────────  │
│ Primary  [Printer          ▾]       │  Mark Index Entry's fields, live
│ Secondary[setup            ▾]       │
│ ☑ Page number  ☐ Other [        ]   │
│ Printer, 4 / setup, 4  (example)    │
│ [Mark] [Mark All] [Remove]          │  Mark All: every occurrence (TX4)
│ ‹ Prev  Next ›  ‹ Prev same  Next › │
│ 20 entries                          │
│  Apple · green  p. 3                │  every entry; click: go to it
│  Printer        p. 1, 4             │
│  …                                  │
└─────────────────────────────────────┘
```

- Each Mark, Remove, Mark All, Generate, and Update is one undo step. In
  Ami Pro, generation cannot be undone.
- The source is not saved or closed. A new document opens in a tab of its
  own, untitled, until Save As.
- Page numbers are those the page surface shows: the page-number object's
  style and prefix, and the layout's restarts. A TOC at the start of the
  document keeps later pages numbered as Ami Pro does.
- TOC rows link to their headings. Ctrl+click on the page and the PDF's
  links go to the heading, and the PDF bookmarks stay as they are
  (TX6).
- Show Power Fields (§28) shows the marks as their codes, as now. With
  Field Shading on, a mark is drawn as a thin grey bar at its place.

**Glossary (TX7)**
- Ami Pro's commands work as Ami Pro's:
  - Edit › Mark Text ▸ Glossary… adds a record.
  - Edit › Insert ▸ Glossary Record… (Ctrl+K) expands the word at the
    caret, or opens the list.
  - Data File… chooses the data file, kept in the settings.
- A Glossary panel in the rail lists the records with a preview. A
  double-click inserts one; there are also Add from selection, Rename, and
  Delete.
- An option, Complete glossary names as you type (off by default), shows
  the record a typed name would expand to, and Tab takes it.
- The app writes the data file as Ami Pro does:
  - Records are appended in Ami Pro's form, then the index footer.
  - The file is written to a temporary file and renamed.
  - Only a file that reads as a glossary is ever changed.
- A new data file is made from the default style sheet with `~|` and
  `id~text|`, as Ami Pro's Mark makes it.

### What SAM stores

- The marks, `[toc]`, and `[master]` are stored as Ami Pro writes them:
  - Index marks get their pages when an index is generated.
  - The TOC table is a page table whose frame flag Ami Pro uses to find it
    again (`0x4000000`).
  - A TOC at the caret is stored the same way; Ami Pro updates it where it
    stands.
- **Not stored, kept in the app's settings:**
  - Join pages into ranges.
  - Generated output's place.
  - The choice of a new document or the end of this one.
- An index at the end of the document is ordinary text in Primary,
  Secondary, and Separator. Ami Pro shows it but does not update it.

### Implementation notes (2026-11-03, the TOC, index, and glossary run)

**Model and files**
- `amipro_document::contents` holds:
  - `TocSettings`: `[toc]` kept place by place, with `levels` and
    `from_levels`.
  - `MasterSettings` (`[master]`) and `ContentsSettings` on the document.
  - `IndexEntry` and `TocMark` over the kept power-field marks.
  - `is_toc_table`/`mark_toc_table`: frame flag `0x4000000`.
- `InlineObject::Break::restart` is the TOC break's flag 0x02. The layout
  counts page numbers from 1 after it (`number_restarts`, `PageBox::shown`).

**Commands** (`amipro_editing::contents`)
- `ContentsHost` gives the pages as their page-number field shows them, and
  the list numbers.
- `collect_toc`, `collect_index`, `build_index`, Ami Pro's sort
  (`index_order`).
- `Command::PutToc` (`TocPlace` Start, At, Output), `PutIndex` (Output,
  End), `PutMarks`, and `SetContents`.
- A TOC in the document stores its levels as `[toc]` in the same step
  (TQ5).

**The panels**
- `contents_panel.rs`, `contents_commands.rs`:
  - The Contents, Index, and Entries pages.
  - `LayoutHost`; output documents on `_TOC.STY` or `_INDEX.STY` in a new
    tab.
  - Ctrl+click on a TOC row (`toc_link_target`).
  - The PDF's links (`print::toc_links`, `PageExtras`).
- `glossary_panel.rs`, `glossary_commands.rs`:
  - The data file in the settings (`glossary_file`), Data File… and New…
    through the portal.
  - Ctrl+K's word (`word_at`), inline insertion (`inline_insert`: the
    record's paragraphs joined to the text around the caret, TQ4).
  - Complete as you type (`complete_glossary`: the status bar names the
    record, Tab takes it).
- `amipro_formats::glossary` reads, appends, renames, deletes, and makes
  data files as Ami Pro's.

**Settings:** `index_ranges`, `glossary_file`, `complete_glossary`.

**Deviations**
- Generating does not save or close the source.
- The index's marks keep their value words (TQ3).
- Output documents lack the sheet's layout frames (TQ6).
- Numeric cells show as stored (TQ2).

## 37. Master documents — proposed 2026-11-03; APPROVED by the user 2026-11-03 (MS1–MS11)

**Status: approved** (`DECISIONS.md`, 2026-11-03, MS1–MS11; MS10 and MS11 took the addition), for the Phase 5 queue's run 13
(`docs/playbooks/FIELDS_AND_REFERENCES.md`, "Run 4"). The evidence is the
decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`, the guide's chapter 26, and
Ami Pro 3.1 fixtures MD1–MD3 (`docs/research/MASTER_NOTES.md`). Today the
app keeps `[master]` typed but its file list as read, keeps `[sequence]`
verbatim, and has no command for either.

### What Ami Pro 3.1 does

- **A master document** is any document whose `[master]` lists files, in
  order, by full lower-cased path. File › Master Document… edits the list
  (Include before the selected file, Remove) and, through Options…, the
  TOC and index output files and Include alphabetical separators.
- **Numbers run on only when the set is worked.** Tools › TOC, Index… and
  File › Print on the master open each member in turn, give it the
  carried numbers, work it, and save it with them in `[sequence]`:
  - the page base (the pages before it), so its page numbers continue;
  - the footnotes before it, so its footnotes continue;
  - each numbered style's counter and each Seq field's highest value.
- A member opened alone shows the numbers it was last saved with. Editing
  an earlier member changes nothing until the set is worked again.
- File › Master Document… on a member only asks "Should it be considered a
  single document?"; Yes drops its `[sequence]` (and any `[master]`).
- **TOC and index over the set** go into the output files (a blank TOC
  name is an error for a master), on `_TOC.STY` and `_INDEX.STY`, with
  the shown pages.
- **Printing the set** prints the TOC output file, the members in order,
  then the index output file, each through its own printer and all its
  pages.
- A member that cannot be opened stops the run with Ami Pro's Open
  message; the TOC and index hold what was scanned.

### Proposal

**One Master Document panel in the rail (MS1),** opened by File › Master
Document… on any document:

```
┌ Master Document ─────────────────── × ┐
│ Files (in order)                       │
│  1 Introduction     intro.sam   p 1–4  │  name, file, its pages in the set
│  2 Installing       instal.sam  p 5–12 │
│  3 ⚠ Printing       print.sam   missing│  missing: Locate…
│ [Add…] [Remove] [↑] [↓] [Open] [Open All]
│                                        │
│ ⚠ install.sam restarts page numbers    │  checks Ami Pro's guide warns about
│                                        │
│ Contents and index                     │
│  TOC file   [toc.sam        ] [Choose…]│  [master]'s names; TOC Options in
│  Index file [index.sam      ] [Choose…]│  the Contents & Index panel (§36)
│  ☑ Alphabetical separators             │
│ [ Generate Contents ] [ Generate Index ]│
│ 3 files · 22 pages · 14 footnotes      │
└────────────────────────────────────────┘
```

- Add… (the portal's file dialog, several at once) inserts before the
  selected file, or at the end, as Ami Pro's Include; ↑ ↓ and drag move a
  file. Each change is one undo step on the master document.
- Open opens a file in a tab (or goes to its tab); Open All opens all.
- The list shows each file's pages in the set, worked out by laying the
  files out in the background as they are read (MS3), and marks missing
  files with Locate….
- **On a member** the panel says "Part of a master document: pages start
  at 5, footnotes at 3", with Treat as Single Document (Ami Pro's Yes; one
  undo step) and, when the master is open, Go to Master.

**Numbers live in the app (MS3).** The app works the set itself whenever a
master is open (no files written): a member opened from the master, or
open while its master is open, shows its numbers in the set at once, and
an edit to an earlier member updates the later ones. A member opened
alone shows the numbers it was saved with, as in Ami Pro. Its numbers are
written as Ami Pro writes them whenever it is saved.

**Contents and index over the set (MS5).** Generate Contents and Generate
Index on a master take every file in order, with the shown pages, into new
tabs on `_TOC.STY` and `_INDEX.STY` (Ami Pro's output files, named by
`[master]` when saved). TOC rows and index entries link to their file and
place (Ctrl+click opens the file at the heading).

**Print and PDF of the set (MS6).** On a master, File › Print… and Export
PDF… offer The whole master document (Ami Pro's: TOC file, files, index
file) or This document only, in the Print panel; the default is the whole
set. The PDF is one file with a bookmark per file.

**Logical additions (MS2, MS4, MS9–MS11).**
- A stored path that does not exist is looked up by name in the master's
  folder (MS2); a member opened alone offers Open Master when a master in
  its folder lists it (MS4).
- Checks for files whose numbering would be wrong in the set (MS9).
- File › Export ▸ on a master offers the whole set as one DOCX, ODT, or RTF
  document, each file a section (MS10).
- The find bar's In All Files of the Master (MS11): matches listed by
  file; Replace All one undo step in each file it changes.

### What SAM stores

- `[master]`: the files as paths, the output names, the separators flag,
  written as Ami Pro writes them (the byte count computed).
- `[sequence]` in a member: page base, footnotes, Seq and style counters,
  in Ami Pro's order.
- **Not stored:** the panel's state; the choice of whole set or this
  document for printing (the settings).

### Implementation notes (2026-11-03, the master documents run, first session)

**Model and files**
- `amipro_document::contents`: `MasterSettings::files` typed (Ami Pro's
  form only, the byte count computed, `is_master`), `Sequence` and
  `Counter` (`[sequence]`), `ContentsSettings::sequence`; native v3
  `sequence`; read by `take_contents`, written by the SAM writer.
- The layout adds the page base to every shown number, starts footnotes
  after the carried count, and seeds numbered styles
  (`paragraph_numbers_carried`, `end_counters`); Seq fields continue from
  the carried values (`fields::carried_seq`, MQ3).
- `amipro_layout::master::next_sequence` (and `_rebased`) is what a file
  passes on; `amipro_editing::master` has the list commands, Treat as
  Single, path lookup (MS2), and the checks (MS9).

**The app**
- `master.rs`: `MasterSet`, `Member`, `resolve`, `read_member` (off the
  interface's thread), `Carrier`, `master_listing` (MS4).
- `master_commands.rs`: `sync_master_sets` after every message (MS3; a
  member's set numbers go into its document and its saved copy outside the
  history, so they do not make it modified, and are written when it is
  saved), the panel's actions, `set_documents`, the set's TOC and index
  (`generate_set_contents`, `generate_set_index`, `follow_set_link`,
  `record_master_output`), `print_set`.
- `master_panel.rs`: the panel. `print_panel.rs`: the Master document
  choice. `print::set_pdf` and `PdfJob::set`: the set's PDF.

- **Export of the set (MS10, second session):** `amipro_exchange::set`
  joins each file's exchange document (built with its pages and pictures)
  into one, a section per file starting where its page numbers start in
  the set; `export.rs`'s `ExportJob::set` takes `print_set`'s parts (TOC
  file, files, index file; MQ5). A member exported alone writes the
  numbers it shows.
- **Find across the set (MS11):** `find_set.rs`; Options ▾ › In All Files
  of the Master and Files of the Master (MQ6).

**Deviations**
- Rows move with ↑ and ↓, not by dragging (MQ2).
- Treat as Single holds for the tab (MQ1).
- The set's export writes one style per name, the first file's (MQ4).

## 38. Merge, labels, and envelopes — proposed 2026-11-03; APPROVED by the user 2026-11-03 (MG1–MG11)

**Status: approved** (`DECISIONS.md`, 2026-11-03, MG1–MG11; MG2 took
spreadsheets too, MG11 took File › New's merge sheets into this run), for
the Phase 5 queue's run 14 (`docs/playbooks/MERGE_AND_TEMPLATES.md`, "Run
1"). The evidence is the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL`, the
guide's chapters 13, 16, 28, and 32, the shipped macros' text, and Ami Pro
3.1 fixtures MGM1–MGM8 (`docs/research/MERGE_NOTES.md`). Today the app
keeps `[recfile]` as read, shows `MergeField` fields as `<NAME>`, and has
no merge command.

### What Ami Pro 3.1 does

- **File › Merge…** is three steps in a welcome box: the data file (make,
  select, or edit it; editing runs the `AMIMERGE.SMM` macro's index card),
  the merge document (Edit › Insert › Merge Field…, a modeless box listing
  the data file's fields), and the Merge dialog: Merge & print; Merge,
  view & print (Print and View Next, Skip and View Next, Print All,
  Cancel); Merge & save as (one file, a page break between records, main
  text only); With conditions (three rows of field, operator, value, And
  or Or); As labels (across, down, the first label's indent, Print each
  label N times).
- **The data file** is an Ami Pro document whose first paragraph gives the
  field and record delimiters (`~|` in new files) and whose next record
  gives the field names; or a document holding only a table (first row
  names); or a DIF, fixed-width, or comma file through a description
  file; or a database through an import filter.
- **The document stores** the data file and description file, the last
  mode, As labels, and the label values in `[recfile]`; conditions are not
  stored.
- **File › Print Envelope…** is the `AMIENV.SMM` macro: the selected or
  first address, the sizes Monarch (7½ × 3⅞ in), 6¾ (6½ × 3⅝), 9 (8⅞ ×
  3⅞), 10 (9½ × 4⅛), 11 (10⅜ × 4½), 12 (11 × 4¾), Custom, DL (220 × 110
  mm), C5 (229 × 162), C6 (162 × 114), C65 (230 × 110), Italian (230 ×
  110); a return address of up to five lines, saved by name; per printer
  the bin, orientation, feed position, and offsets; `_AMIENV.STY`'s
  `Recipient` style.

### Proposal

**One Merge panel in the rail (MG1),** opened by File › Merge… and Edit ›
Insert › Merge Field…; Ami Pro's three steps are its sections:

```
┌ Merge ───────────────────────────── × ┐
│ Data file                              │
│  clients.sam · 120 records · 8 fields  │  Ami Pro data, table, DIF, fixed,
│ [Open…] [New Data File…] [Edit records…]  comma, CSV/TSV, DBF, XLSX, ODS
│                                        │
│ Fields                     (click: insert at the caret)
│  TITLE  FIRST  LAST  COMPANY  CITY …   │  MergeField NAME power fields
│ [MergeRec] [NextRec] [Skip if…]        │
│                                        │
│ Preview   ☑ Show record  ◀  3 of 120 ▶ │  the record's values in place
│                                        │
│ Conditions                       42 of 120 match
│  If [CITY  ▾] [=  ▾] [Boston    ]  And │  any number of rows
│  If [AMOUNT▾] [>  ▾] [100       ]  [−] │
│ [+ Condition]                          │
│                                        │
│ Labels    ☐ Merge as labels            │
│  Sheet [Avery 5160 Address  ▾] 3 × 10  │  the sheet drawn small; a click
│  ▦▦▦ start at label [1]  copies [1]    │  picks the start label
│                                        │
│ Merge to                               │
│ [New Document] [PDF…] [Printer…]       │
│ [View & Print] [Ami Pro File…]         │  Ami Pro's Merge & save as
│  ☐ One PDF per record, named by [LAST▾]│
└────────────────────────────────────────┘
```

- **The data file (MG2).** Open… (the portal's file dialog, with
  `click_unrun` in tests) reads any of Ami Pro's kinds, CSV and TSV, dBASE
  `.DBF` natively, and the first sheet of XLSX and ODS, off the
  interface's thread. Choosing a file is one undo step (`[recfile]`). A
  file that is not found is looked up by name in the document's folder
  (as MS2). Fields match without regard to case, spaces as `_`.
- **Fields.** A click inserts `MergeField NAME` at the caret as Ami Pro's
  power field (one undo step); MergeRec, NextRec, and Skip if… insert
  those fields (MG10).
- **Preview (MG5).** Off: fields show `<NAME>` with field shading. On: the
  chosen record's values show in place, laid out; ◀ ▶ step through the
  matching records. Display only: the document and its file keep the
  fields, and the history does not change.
- **Conditions (MG6).** Ami Pro's operators and rules exactly, any number
  of rows, a live count, kept in the session and in the native file (SAM
  cannot hold them). Each change is one undo step.
- **Merge to (MG4).**
  - *New Document*: every matching record in one new tab, a page break
    between records, frames and fields included.
  - *PDF…*: one PDF of every record's pages, or one per record named from
    a field.
  - *Printer…*: the print portal once, then every record's pages.
  - *View & Print*: the panel steps record by record with Print and View
    Next, Skip and View Next, Print All, and Cancel; edits made while
    viewing carry to later records, as in Ami Pro (one document).
  - *Ami Pro File…*: Ami Pro's Merge & save as, a SAM of the main text, a
    page break between records.
- **Empty fields (MG9).** In merged output a paragraph holding only empty
  fields takes no space (Ami Pro's rule); a doubled space and a leading
  comma left by an empty field are tidied. The preview shows the same.

**The record editor (MG3).** Edit records… shows the data file's tab as a
grid in place of its pages (View › Show as Document returns):

```
│ [+ Record] [− Record] [+ Field] [Rename] [◀ Move ▶]  Find [      ] │
│ TITLE ▲ │ FIRST   │ LAST    │ COMPANY    │ CITY     │ …           │
│ Mr.     │ Terry   │ Allen   │ Vega Corp  │ Boston   │             │
│ Ms.     │ Pam     │ Beck    │            │ Seattle  │             │
```

- A click on a heading sorts by it; each edit (a cell, a record, a field,
  a sort) is one undo step on the data document, which is saved in Ami
  Pro's delimited form so Ami Pro reads it. New Data File… asks only for
  the field names (delimiters `~` and `|`, changeable). External files
  are read only (Ami Pro's rule).

**Labels (MG7).** Merge as labels turns the document into one label's
page: the sheet list (the app's own, from Avery's published sizes, every
part Ami Pro listed, plus Custom) sets the page layout (top and left
margins 0, the gaps as Ami Pro's custom rule) and `[recfile]`'s across,
down, and first label's indent with flag 0x10, in one undo step. Output
places each label on the sheet as Ami Pro prints them (across, then down),
from the start label, each label N times.

**Envelopes (MG8).** File › Print Envelope… opens an Envelope panel:

```
┌ Envelope ────────────────────────── × ┐
│ Size [Size 10 (9½ × 4⅛ in)        ▾]   │  Ami Pro's sizes, Custom
│ Recipient                              │  the selection, else the first
│  [Ms. Pam Beck                    ]    │  address on page 1
│  [41 West St.                     ]    │
│ Return address [Home ▾] [Save] [Delete]│  saved in the settings
│  ☑ Print return address                │
│ Feed [Left|Centre|Right] Offsets ↓ →   │
│ [Print…] [PDF…] [New Document]         │
│ [Merge Envelopes]  (with a data file)  │
└────────────────────────────────────────┘
```

The envelope is a document on `_AMIENV.STY` (the `Recipient` style; the
app's own equivalent when the sheet is not found), its page the
envelope's size, landscape.

**File › New's merge sheets (MG11).** A new document from `_MERGLET.STY`,
`_LABEL.STY`, `_ENVELOP.STY`, or `_AMIENV.STY` (with contents) opens the
Merge panel, the Merge panel at Labels, or the Envelope panel in place of
the sheet's macro. Until the templates run (queue 15) gives File › New its
chooser, the Merge panel offers these as Start from: Letter, Labels,
Envelope.

### What SAM stores

- `[recfile]`: the data file's path, the description file (`*dBase` and
  the like for external files), the flags (last mode, As labels 0x10),
  the unit, across, down, and the first label's indent, written as Ami
  Pro writes them (values kept as read when As labels is off).
- Merge fields as `<:X12,-32768;MergeField NAME>` with the result
  `<NAME>`; MergeRec, NextRec, Skip as kind 3 fields.
- The data file in Ami Pro's delimited form.
- **Not stored:** conditions (native only, listed by Save As), the
  preview record, label copies and start position, envelope options (the
  settings).

### Implementation notes (2026-11-03, the merge run)

**Model and files**
- `amipro_document::merge`: `RecordFile` (`[recfile]`'s eight values,
  written as Ami Pro writes them, label values kept as read without As
  labels), `Condition`, `Operator`, `Join`, `MergeSettings`; native v3
  `recfile`, `mergecondition`, `withconditions`; Save As SAM lists the
  conditions (`LossKind::MergeConditions`).
- `amipro_editing::merge`: `data` (every data kind, the delimited writer,
  the record editor's changes and `set_data_command`, `new_data_document`),
  `conditions` (Ami Pro's numbers and dates, text with case, And before
  Or), the merge (`merge_one`, `merge_all`, `joined`, `MergeInputs::tidy`
  and `in_place`, `drop_empty_field_paragraphs`, `tidy_empty_fields`,
  `merge_field_names`, `unknown_fields`), `labels` (`SHEETS`,
  `LabelSheet::of`, `place`, `label_run`, `set_up_labels`, `labels_off`,
  `label_sheets`), `envelopes` (`SIZES`, `first_address`, `fill`,
  `envelope_document`, `envelope_on`). `amipro_exchange::sheets`: XLSX and
  ODS first sheets.
- In the field language a merge field the data file lacks gives "" and
  Ami Pro's message during a merge (MGM8). Text typed right after a field
  goes after it (MC11).

**The app**
- `merge.rs` (the data file found and read, every kind), `merge_panel.rs`
  (the panel: data file, fields, preview, conditions, labels with the
  sheet drawn small, Merge to, View & Print's buttons, Start from),
  `merge_commands.rs` (the actions, the outputs, View & Print in place,
  New Data File…, labels, Start from; print jobs answered in
  `print_merged`).
- `data_grid.rs` (since MR1 `data_file.rs`, below): the record editor in place of the pages
  (`AmiProApp::sync_data_grids` after every message follows Undo and
  feeds open merge documents' panels).
- `envelope_panel.rs`: File › Print Envelope… (`CommandId::PrintEnvelope`,
  File menu after Print Options…), its options and return addresses in
  the settings (`envelope`, `envelope_return`, `return_addresses`).
- `print.rs`: `SetPart::pages`, `PdfPart::pages`, `PdfJob::labels`,
  `labels_pdf` (labels on sheets, `PdfWriter::sheet`), `PdfOptions::placed`
  (an envelope on larger paper).
- Deviations from the proposal: labels to New Document are whole sheets
  (MC8); View & Print prints at the end (MC7); the record editor's Save
  writes over a SAM data file with a backup the first time, as every SAM
  since 2026-11-04 (MC6).

### Rework after first use (2026-11-04, the user's MR1 and MR2)

The user's first use found the record editor and the panel "overly
complicated, small and cramped", and the entry area counter-intuitive:
after + Record, typing did not put values in the grid (a click chose a
cell, and its value was typed in a box above the grid). Entering data
should work as Ami Pro's tables do. Asked with a recommendation each, the
user took both:

- **MR1: data files are Ami Pro tables.** A document holding one table,
  its first row the field names and each later row a record, is a data
  file Ami Pro merges from (kind 4, VM MGT1). The grid goes; the data
  file's tab shows its pages with that table, edited with the table
  editing of §32:
  - click a cell and type; Tab and Shift+Tab move between cells, Tab in
    the last cell adds a record; Ctrl+arrows, Alt+Up/Down;
  - the Table menu and the right-click menu add and delete records (rows)
    and fields (columns); a field's name is typed in its heading cell;
  - Table › Sort Ascending and Sort Descending sort the rows below the
    heading rows by the caret's column (new; Ami Pro's Tools › Sort is
    not built);
  - each change one undo step; open merge documents follow the records
    as typed.
  - **New Data File…** asks for the field names and a file, and writes a
    new SAM holding the table: the names in a shaded heading row (Headings,
    so it repeats on each page) and one empty record, the caret in it;
    with more than five fields the page is landscape.
  - **Edit Records** opens the data file's tab. A table file is shown as
    it is. Ami Pro's delimited data file (`~|`) is shown as the same table
    and Save writes it back in its own delimited form; a value holding a
    delimiter stops the save with a dialog naming its record and field.
- **MR2: the panel is Ami Pro's three steps** (MRGWELCOME's), 340 px wide,
  one step open at a time, each step's title a button with its summary:
  1. **Data file** (the file, its kind, records and fields): Open…, New
     Data File… (the names box in place), Edit Records, as full-width
     buttons.
  2. **Merge document**: Start from (Letter, Labels, Envelope); the fields
     as buttons that insert at the caret; MergeRec, NextRec, Skip if…;
     the fields the data file lacks; Preview with ◀ N of M ▶.
  3. **Merge**: how many records merge, then New Document, PDF…,
     Printer…, View & Print, Ami Pro File… as full-width buttons, and
     **Options ▸** folding With conditions (the rows), Merge as labels
     (the sheet, its start label, copies), and One PDF per record. While
     viewing, step 3 holds View & Print's buttons.

  The step opened first: 1 without a data file, 2 when the document has
  no merge field yet, 3 otherwise; choosing or making a data file opens 2.

Implementation (2026-11-04):

- `merge::data::table_data_document` (the table, its shaded heading row,
  landscape past `PORTRAIT_FIELDS`), `data_document`, `delimiter_clash`;
  `TableEdit::SortRows` (`tables::sort_order`: numbers by value, other
  text without regard to case, the heading rows first), Table › Sort
  Ascending and Sort Descending.
- `data_file.rs` replaces `data_grid.rs`: `DataForm` (the delimiters Save
  writes back in, the table as last read), `edit_records`,
  `show_records_as_table` (a delimited file becomes the table in a fresh
  tab state, the caret in the first record), `data_for_sam` (Save's
  delimited document, or the clash as a dialog), `sync_data_forms` (open
  merge panels follow; the panel's count of merge fields, made when the
  document changes).
- `merge_panel.rs`: `Step`, `Shown::step`, `MergeAction::Step` and
  `Options`, `step_title`, `step_body`, `options`; 340 px.
- VM: AMD1 as a table opens as made, and Ami Pro's merge over it (AMT2)
  is the app's. Found: Ami Pro takes a table cell that reads as a number
  as a number (`75.50` merges as `75.5`; `MERGE_NOTES.md` §13), an open
  point for the table-data run.

## 39. Commands, templates, and views (run A) — proposed and APPROVED by the user 2026-11-04 (CM1–CM12)

Run A (`docs/COMMANDS_RUNBOOK.md`) adds Ami Pro's remaining verified
commands over structures the app already reads and writes, plus line
numbering and password-protected files, which are new in SAM. Every
change to a document is one undo step; every app-wide setting is in the
COSMIC configuration (never touched by tests).

**File › New… (CM1, CM2).** A dialog over the window:

```
┌ New Document ─────────────────────────────────────────────┐
│ List by: (•) Description  ( ) File name   [Folder ▾]      │
│ ┌───────────────────────────┐ ┌───────────────────────┐   │
│ │ Recent                    │ │                       │   │
│ │  Default (_DEFAULT.STY)   │ │   first page, drawn   │   │
│ │  Memo 1 (_MEMO1.STY)      │ │   by the app from the │   │
│ │ All style sheets          │ │   sheet's contents    │   │
│ │  …                        │ │                       │   │
│ └───────────────────────────┘ └───────────────────────┘   │
│ [x] With contents   Runs macro NAME: not run (CM2)        │
│                                    [Cancel]  [Create]     │
└───────────────────────────────────────────────────────────┘
```

The style sheets come from the style-sheet folders (Preferences ›
Folders); double-click or Enter creates. Ctrl+N stays the blank
`_DEFAULT.STY` document. The merge sheets keep their Start from route
(MG11). SAM: the new document names its sheet in `[sty]`, as Ami Pro's;
a macro name is kept as read.

**Paragraph Properties (CM3).** Format › Paragraph… opens a rail panel at
300 px (like Style Properties, §15) for the caret's or the selected
paragraphs: Indention (All, First, Rest, Right; hanging; from the style
shown greyed), Spacing (line spacing, Above, Below, Add in), Alignment,
Breaks (Ami Pro's style break rules, `[brk]`, per paragraph where SAM holds them); Revert to style per
section; each field change one undo step. Text › Indention… and
Spacing… open it at their section. SAM: `<:I…>`, `<:S…>`, `<+@ABC>` as
the importer already reads them.

**Fast Format (CM4).** Ctrl+T or the toolbar's paintbrush (a checked
toggle while on) picks up the selection's text format or paragraph
style, by Ami Pro's options (where Ami Pro offers them is mapped in the
run; CK…); every
selection made while on takes it, each one undo step; Esc or the toggle
turns it off; the status bar says it is on.

**Style.** Define Style is Create Style from the caret's paragraph
(§14); Select a Style (Ctrl+Y) focuses the toolbar's style field with
its list open.

**Page.** Ruler ▸ Insert (the caret's paragraph takes its own ruler, the
ruler run's `SetParagraphTabs`) and Remove; Insert Page Layout ▸ Insert,
Remove, Revert where not yet commands. **Line Numbering (CM7)** is a
section of Page Layout Properties (§18) with Ami Pro's options, drawn in
the margin on screen, in print and PDF, written to `[lnopts]`.

**Edit.** Mark Text ▸ Protected Text and No Hyphenation (toggles, checked
at the caret, as the Text menu's character commands).

**File.** Revert to Saved (asks first; undoable as one step).
**Password-protected files (CM5):** File › Open asks for the password;
Save As has Password… (set or remove). Mapped and twinned in the VM.

**Preferences… (CM6, CM10).** One dialog (Ctrl+,) with pages: General
(default style sheet for New, recent files, undo levels, the `.bak` on
saving over a SAM, Custom zoom), Folders (style sheets, documents), View
(every mark of View › Show ▸), Toolbar (the existing Customize Toolbar).

**Views (CM8–CM11).** View › Full Page (Ctrl+D), Standard, Enlarged,
Custom as checked presets beside Zoom In/Out; Facing Pages (pairs of left
and right pages, as Ami Pro pairs them); Draft Mode (one column at the
window's width, frames as one-line markers, the styles' fonts; a free
shortcut, CK…); Show Clean Screen (all chrome hidden, a small Return
button, Esc); View › Show ▸ Tabs & Returns (also a toolbar toggle),
Column Guides, Margins in Colour, Pictures, Notes, Vertical Ruler, Power
Fields; the view kept per tab. Window › New View opens a second tab on
the same session (own caret, scroll, zoom, mode; one history, titled
NAME:2).

**Additions (CM12).** Insert › Special Character… (Ami Pro's ANSI table,
App. C, with recent characters at the top); Help › Keyboard Shortcuts
(generated from the registry); Help › Compatibility and Fidelity (the
loss check's list for this document, without saving); the status bar's
Face and Point Size buttons (a click opens the list; the choice applies
to the selection or the caret, one step).

**Implementation notes** are added here as the run builds each item.

**DO.4 Paste Special (built 2026-11-07).** `paste_special.rs`. The rail panel
lists what the clipboard offers: the toolkit cannot list a clipboard's types,
so each is looked for in turn (text, then Rich Text Format, HTML, PNG, JPEG,
GIF, Bitmap, TIFF and SVG images, Files) and the rows keep that order (Ami
Pro keeps the source's own order; 3.1 has no ranking, no "As:" text, and no
display as icon). The first row is chosen; Paste, or a double-click on a row,
pastes it: Text and HTML as text (HTML as its text: tags and script/style
dropped, entities read), Rich Text as formatted text (the Paste import with
Apply styles), an image or the first picture file as a picture where
Insert › Image… puts one. One undo step; the panel closes. A row the app
cannot hold (image bytes no reader takes, files that are not pictures, empty
text) shows Ami Pro's message 3092 in the status bar and the panel stays. An
empty first look is repeated once after 150 ms (the clipboard can find
nothing as the window gains focus) before the panel says the clipboard holds
nothing it can paste. Plain Paste, Paste as Plain Text and the toolbar are
unchanged. OK candidate OK-DO4-1: the rows' order is the app's, not the
source's.

**DO.5 Image Processing (built 2026-11-07).** `amipro_pictures::adjust` holds
the filters and `amipro_document::ImageAdjust` the settings (defaults 50, 50,
0, 0, off; halftone Fastest as Ami Pro's untouched image stores it). Order:
Lookup, Smooth, Edge, Negate. The lookup curve is the decompile's polyline
(checked against the VM's end points: 50/50 identity, brightness 100 white
and 0 black, contrast 0 flat 124 at brightness 49, contrast 100 a threshold
at 130, brightness 30 contrast 70 a ramp ending at 205). Smoothing blends
the pixel with a [1 2 1] x [1 2 1] / 16 blur by the strength; edge
enhancement adds the strength times the four-neighbour Laplacian (one pixel
of ringing to saturation at 100); invert is 255 - x; alpha is kept.
Adjustments are drawn live on the page, in the PDF and print, and exported
as a PNG of what the page shows (a picture with adjustments is never handed
over as the original JPEG or PNG). Frame Properties › Picture › Image: the
four values typed where shown beside a slider (a drag is one undo step, the
page follows it), Invert, Halftone, Revert; Tools › Image Processing…
opens the rail for the selected bitmap. Storage (OK-DO5-3): a gray-scale
8-bit TIFF takes Ami Pro's form on its first adjustment (extension `.tgf`,
the TIFF's bytes unchanged, the state blob in the frame's `[isd]` words,
written as the VM's IMV1 files hold it: the twelve fixtures' words are
equal); every other bitmap keeps editable settings in the native file and
Save As SAM bakes them into the 24-bit bitmap it writes and lists them
("pictures with image adjustments"). OK candidates: OK-DO5-1 the smoothing
kernel and OK-DO5-2 the edge kernel (Ami Pro's were not found; the VM showed
only about a one-pixel blur and one-pixel ringing); both need a visual
check against Ami Pro on a wider test image.

*DO.3, OLE objects (built 2026-11-06).* An OLE frame is drawn from the
metafile of its stored presentation (`amipro_pictures::ole`: the OLE 1.0
stream reader; `pictures::ole_drawing`): 34 of 34 corpus objects draw (13 drew
from a screen image before, 21 were boxes); the natural size is the
presentation's width and height (hundredths of a millimetre, the height
negative in the files), the frame's scaling then applies as for any picture;
in Draft (pictures off), on the PDF (as paths) and in print the object goes
the way every picture goes. An object whose presentation cannot be read keeps
Ami Pro's screen image, else a labelled box (P4, OD9). Bytes are never
changed: the stream is written back as read, trailing bytes included.

*Object section.* First in Frame Properties for an OLE frame (above Picture,
which loses Rotate for these): Class (friendly label and the class name),
Kind (embedded, linked, static), Native data size, Presentation (metafile and
its size in inches, or "None readable"), Extra data (bytes kept after the
presentation), the caption "Editing needs the program that made it.", and
Convert Object to Picture with its note ("The object's own data is dropped;
Undo brings it back."). Convert is one `ReplacePicture`: a placeable metafile
at the presentation's size, the frame's scaling and place kept; refused for
an object without a readable presentation. A non-OLE stream shows "not
readable" and no Convert.

*Edit › Links….* A rail list of OLE objects (kind "OLE object" or "OLE
link", source, status) and of frames Ami Pro marks as DDE links (flag
`0x200000`: application, topic, item); no Update, Unlink, Edit, or Create (no
servers); an empty document says "This document holds no links or OLE
objects."; a row goes to the frame's page and selects it, so Frame Properties
(Object) show; closing the selection brings the list back.


- CA.2 (2026-11-04): Keyboard Shortcuts and Compatibility and Fidelity
  open in the context drawer beside About (`help_drawers.rs`). The
  shortcuts list every implemented command with a shortcut by menu, with
  a filter; Compatibility runs `sam_losses` on demand, with Check Again.
- CA.3: Preferences is a dialog (`preferences_dialog.rs`): General (undo
  levels 1–1000, recent files 0–9, the sheet New… selects first, the
  `.bak`), Folders (Documents; style-sheet folders, `AMIPRO_STYLES`'s
  listed), View (the View menu's toggles), Toolbar (shown; Customize
  Toolbar…). File › 1 NAME… lists recent files above Quit.
- DF1: Folders › Documents is the first row: the folder, or "Not set — the
  desktop's own default", with Choose… (the desktop's folder chooser) and
  Clear (disabled while unset). Kept as `documents_folder`; a folder that
  no longer exists counts as unset when used, without erasing the setting.
  Choosers with no better start open there: File › Open, Save As of an
  untitled document, Insert › File, Insert › Image, Save Picture As,
  Export ▸ and Export PDF (after the last export folder), the Export and
  Import Folder choosers, Add to Master Document, Locate File, merge data
  files and Merge to, Draw import/save, the envelope PDF. A document with a
  file keeps its own folder; the style-sheet choosers keep the style-sheet
  folder; the glossary and user-dictionary choosers keep the desktop's
  start (their files are the app's own). `file_dialogs.rs` sends the
  portal's current folder (the pinned libcosmic open dialog does not).
- CA.4: File › New… (`new_document_dialog.rs`) under New Document, also a
  toolbar choice; the preview is a miniature of the laid-out first page
  (`page_preview.rs`); clicking a selected row creates.
- CA.5: Paragraph Properties (`paragraph_properties.rs`) in the rail;
  boxes apply on Enter or leaving them; Above, Below, and the breaks are
  shown from the style with Modify Style…; Increase and Decrease Indent
  step All by 0.5 in.
- CA.6: Fast Format (`fast_format.rs`): Format › Fast Format (Ctrl+T),
  checked while on, and a toolbar choice; the page sends
  `PageAction::PointerReleased` after a press in text only while it is on.
- CA.7: Edit › Mark Text ▸ Protected Text and No Hyphenation (checked at
  the caret); Select a Style is Ctrl+Y (CK7); File › Revert to Saved…
  (enabled when the saved document changed) asks with Ami Pro's question
  and is one undo step (`revert_saved.rs`).
- CA.8: Format › Page Layout Changes ▸ (Insert, Remove, Revert to Standard)
  and Format › Ruler ▸ (Insert, Remove), `page_commands.rs`.
- CA.9: the status bar's Face and Point Size are drop-down lists beside
  the zoom (the caret's or the selection's font); Insert › Special
  Character… (`special_character.rs`) inserts on a click.
- CA.10: View › Full Page (Ctrl+D; again returns to the view before),
  Actual Size, Enlarged (200%), Custom Zoom, Fit Page Width as checked
  presets; Continuous Pages and Facing Pages (`PageView::arranged`), kept
  per tab.
- CA.11: View › Draft Mode (Ctrl+Shift+D, `view_modes.rs`): a display copy
  of the document with Draft pages, relaid out when the window's width
  changes; frames still drawn (CK10). View › Show Clean Screen (F11): the
  menu, toolbar, tabs, and status bar become empty slots, the window goes
  full screen, Return or Esc ends it.
- CA.12: View › Show ▸ (Tabs & Returns, Column Guides, Margins in Colour,
  Pictures, then Power Fields, Field Shading, Notes, Bookmarks), checked;
  the same settings on Preferences › View. Marks are drawn with quads on
  screen only. Window › New View and the Vertical Ruler are open (CK11).
- CA.13: Line numbering is a section of Page Layout Properties ("For the
  whole document": the list None, Number all lines, Number text lines;
  Every line, other, fifth, or N with the count typed in place; Reset each
  page; Based on, shown as Body Text for text lines), each change one
  `SetLineNumbering` step; Format › Line Numbering… opens it there. The
  layout works the numbers out after each layout (`line_numbers.rs`) and
  the page surface draws them, so screen, print, and PDF agree; Draft
  shows none.
- CA.14: File › Password… (Enter and Verify, Remove Password when set;
  14 characters, Ami Pro's messages) and the password box at File › Open
  (`password_dialog.rs`); every save as an Ami Pro document writes the
  file protected while the document has a password (CK13).

## 40. Outline mode and the outline navigator (run B) — APPROVED by the user 2026-11-04 (OM1–OM4)

Run B (`outline-mode`) is set up after run A. View › Outline Mode shows
the text by outline level with Ami Pro's level icons, Expand and
Contract, Show levels 1–n, Use Outline Styles, and editing in place; an
Outline navigator in the rail shows the headings as a tree (a click goes,
a drag moves a section, keys move in it) (OM1). Collapsing applies only in
Outline mode and the navigator, per tab, never written (OM2). Move Up and
Move Down (Alt+Up, Alt+Down) move the caret's heading with its section in
both modes, outside tables (in a table they move rows), frames anchored
in the section with it, one undo step (OM3). Light weight (OM4).

**Implementation notes** are added here as the run builds each item.

- OB.2 (2026-11-05): Move Up and Move Down (Alt+Up, Alt+Down; Format ›
  Bullets and Numbering ▸) move the caret's section (`sections.rs`: a
  heading to the next heading of its level or higher; body text alone)
  over its neighbour as one step, the caret staying in the moved text; in
  a table cell the keys move rows (§32). RB1.
- OB.3 (2026-11-05): View › Outline Mode (a checked toggle beside Draft
  Mode, per tab, exclusive with it; `outline_view.rs`): a display copy on
  window-wide pages (720-twip left margin for the icons), every style left
  aligned without space above or below, headings (L − 1) × 480 twips in,
  body text 240 twips right of the heading above; a grey plus on a heading
  with text under it, a minus on one without, a small box on body text.
  The document is never changed by the mode. RB2.
- OB.4: View › Outline ▸ Expand (Alt+PageDown), Contract (Alt+PageUp),
  Show 1–9 Levels and Show All Levels (checked), and the Outline bar in
  the toolbar's place in Outline mode (1–9, All, Promote, Demote, Move Up,
  Move Down, Expand, Contract, Layout Mode). Contract hides a heading's
  whole section (a black, heavier plus); Show n hides deeper headings and
  body text and clears the marks; Expand shows a section down to body
  text. RB3.
- OB.5: View › Outline ▸ Use Outline Styles (checked; a preference, saved
  as Ami Pro keeps it in its INI, for every open document): in Outline mode
  each level shows in `_OUTLINE.STY`'s font (level 1 Times 18 bold, 2
  Times 14, 3 Times 12, 4–9 and body text Arial 12); the paragraphs keep
  their styles.
- OB.6: View › Outline ▸ Outline Navigator (checked while open) shows the
  headings in the rail as a tree by level, in Layout and Outline mode: the
  current heading highlighted; a click goes to a heading; a drag onto
  another heading moves the section before it (a line shows where), onto
  the area below the tree to the end, one step (`MoveSectionTo`); the
  chevrons contract and expand in step with Outline mode; after a click on
  the tree, Up and Down go between headings, Left contracts, Right expands.

## 41. Table data: formulas, number formats, Quick Add, Sort (run C) — interface built 2026-11-06 (TD.7); decisions TN1–TN11 answered 2026-11-06 (recommended options)

Run C (`table-data`, `docs/TABLE_DATA_RUNBOOK.md`; evidence
`docs/research/TABLE_DATA_NOTES.md`). Numbers in table cells as Ami Pro 3.1
has them: formulas in cells, Quick Add, the paragraph style's number
format, cell addresses, errors, and Tools › Sort. Nothing here is a modal
dialog (rail inspectors, edit in place); Ami Pro's Edit Formula and Sort
dialogs become rail sections. The draft is checked against the physical
build at the run's exit.

**Menus.**

```
Table                                   Tools
  ...                                     Sort…               (the rail's Sort panel)
  Edit Formula…       (focus: rail Formula)
  Quick Add ▸ Row       Alt+…  none (Ami Pro has none)
              Column
  ...
View ▸ View Preferences… ▸ [ ] Table row/column headings
```

- **Edit Formula…** is enabled when the caret is in one table cell (not a
  cell selection of several, not a cell Honor protection keeps out). It
  opens Table Properties if needed, opens the Formula section (TN1), and
  moves focus to its box with the cell's formula text selected. A
  double-click on a formula cell does the same (`PageAction::EditFormula`).
- **Quick Add ▸ Row / Column** is enabled in a single cell that has a cell
  to its left (Row) or above it (Column). It writes the formula into the
  caret cell (replacing what was there, one undo step) and leaves the caret
  in the cell.
- **Recalculate Table** (added; Ami Pro recalculates only on edits) works
  every formula of the table out again as one undo step; enabled when the
  table has a formula. No shortcuts for any of the three (Ami Pro has none).
  Page surface only.
- **Tools › Sort…** opens the Sort panel in the rail, for the selection
  (paragraphs) or the table's selected rows (TN3). Table › Sort Ascending
  and Descending stay as one-click sorts of the caret's column.

**Table Properties › Formula (TN1)**, a new folding section after Cells,
following the caret cell:

```
│ ▾ Formula   (cell C4)                      │
│   [ =D4*E4                             ]   │  typed where shown
│   Result  3,250.00                         │  as the style shows it
│   [ Quick Add row ] [ Quick Add column ]   │
│   [ Remove formula ]                       │
│   status line: "Cannot understand the formula." / "Divides by zero." /
│                "Circular formula." / "Refers to a cell that is not a number (REF)."│
```

- The text box takes the formula as typed; Enter or leaving the box applies
  it as one undo step (`SetCellFormula`); Esc restores. Ami Pro's syntax is
  accepted: an optional leading `=` or `@`, `+ - * / %`, parentheses, cell
  names (`B2`, `$B$2`), `@sum(B2..B5)` or `@sum(B2:B5)`, any letter case.
  The text is kept as typed in the file (the cell's second body).
- The result is the cell's own text; the section shows it as the style
  displays it. A cell with a formula is right-aligned like a number unless
  the paragraph is aligned (Ami Pro).
- Native-only extras (TN4: `@avg`, `@min`, `@max`, `@count`) are accepted;
  Save As SAM lists them as "formulas Ami Pro cannot calculate" and writes
  the text and the result.
- **Remove formula** and Del on a selected formula cell take the formula
  out; the cell is empty and plain (VM TDF1), no question.
- A formula cell is drawn with a thin accent-coloured box round it while
  the caret is in it (Ami Pro).
- **Typing in a formula cell is refused** (TNK-TD7a, this run's choice):
  printable keys, Enter, Backspace, and Delete (a caret or selected
  characters) inside a cell that holds a formula do nothing, and the status
  bar says "This cell holds a formula: use Table › Edit Formula… to change
  it". A cell selection's Del still takes the formula out.
- Errors (TN8): the message in the status bar, no modal box, and the Formula
  section shows the cell's message under the box. Texts (notes 11.3): "Cannot
  understand the formula in cell 'B7'." (3125, built with the cell's name;
  also shown when Enter is refused, the typed text staying in the box for
  correction), "There is a formula that tries to divide by zero." (3126),
  "This field contains a circular formula." (3127). The status bar shows the
  caret cell's message after a formula is set, a cell is left, or a table
  command runs, and forgets it once the cause is fixed. A divide by zero or
  a loop is drawn blank (stored empty, TDF1); a formula that read a cell
  that is not a number (REF) is drawn as the word REF (the layout shapes it
  as a number-style word over no bytes; `Engine::note_refs`), with no
  status message, and the Formula section's line says it. The rest of the
  table calculates.
- Esc in the box restores the cell's formula; Enter or leaving the box with
  an unchanged text does nothing.

**Recalculation (TN5).** When a cell is left or a command is applied (type,
paste, cut, Del, Insert/Delete Row/Column, sort, Undo, Redo), every
dependant is recalculated inside the same undo step. Typing in a cell does
not recalculate per keystroke: when the caret leaves a cell it edited,
`DocumentSession::settle_numbers` rewrites the number (TD.5) and then
sends `RecalculateTable { changed: [cell] }`, both amended into the typing
run's step (`recalculate_cell`); a recalculation that changes nothing is
ignored. Opening a file keeps stored results (VM TDF6).

**Number format in the Styles rail (TN6)**: the style's properties get a
**Table format** section (as Modify Style › Table format):

```
│ ▾ Table format                            │
│   Cell format [General ▾]  (General, Fixed Decimal, Currency, Percent)
│   Decimal places [2]   Decimal symbol [.]  │
│   [x] Thousands separator [,]              │
│   Negative  [x] Leading minus [ ] Trailing minus
│             [ ] Red   [ ] Parentheses      │
│   Currency  Symbol [$]  (•) Leading ( ) Trailing  │
│   Sample: -1,234.50                        │
```

- Edited in place, applied to every table cell that uses the style (usually
  Table Text), one undo step; Revert as for other style properties.
- Leading minus and Trailing minus (and Parentheses) are exclusive, as in
  Ami Pro. Currency options apply to the Currency format. The Sample shows
  the format on a number.
- Table Properties › Cells shows "Number format: Table Text — Edit style".

**Numbers in cells (TN2).** A cell whose text is a number (digits, a sign,
the decimal symbol) is numeric: right-aligned and shown through the style's
format when the caret leaves it; while the caret is in the cell only the
number shows. A space, letter, comma, `%`, or currency sign makes it text.
The file keeps the plain text Ami Pro stores.

**Cell addresses (TN9).** View › Show ▸ and View Preferences › View: "Table
row/column headings" (`CommandId::TableAddresses`, a check item, off by
default, kept in the preferences as `table_addresses`): letters above the
columns and numbers left of the rows of every page table, small and in the
view-marks colour, drawn outside the cells on screen only (a covered cell
gets none). The status bar shows the caret cell's address ("Cell B3") while
in a table.

**Sort panel (TN3)**, in the rail:

```
┌ Sort ───────────────────────────── × ┐
│ Level 1  Field [1 ]  (•) Alphanumeric ( ) Numeric │
│                       Word (•) 1st ( ) 2nd ( ) All │
│ Level 2  Field [  ]  ...                           │
│ Level 3  Field [  ]  ...                           │
│ Order  (•) Ascending ( ) Descending                │
│ Delimiter (•) Tab ( ) Character [ ]   (paragraphs only)│
│ Paragraphs in record [1]                           │
│ [ Sort ]                                           │
└────────────────────────────────────────────────────┘
```

Fields are numbered from 1 (a table's columns). In a table the Delimiter
group gives way to a line saying whole rows move, and the record counts
rows; with the caret in a table (or a single cell selected) the whole
table sorts, below its heading rows, and with several rows selected those
rows. Refusals and questions as
Ami Pro's, in the panel's status line with Continue and Cancel: a formula
cell in the range (refused), connected cells (continue to un-connect), a
page or column break in the text (continue across it), a table with
footnotes (refused). The sort is one undo step (Ami Pro cannot undo it).

**Sort panel, as built (TD.8).** Tools › Sort… (after Language…) opens
the panel (`sort_panel.rs`, `sort_commands.rs`, `CommandId::Sort`, no
shortcut); Esc or the close button closes it. The first line follows the
selection: "Sorting 12 paragraphs", "Nothing is selected: sorting all 12
paragraphs of the text" (Ami Pro asks "sort the entire stream?"; the panel
does not ask, the line says it and Sort goes ahead), "Sorting table rows
2–9", "Sorting the whole table (12 rows)", or why nothing can be sorted
(Sort is then off). Level 1 needs a field; levels 2 and 3 are left out
when their field is empty. Word applies to Numeric too (Ami Pro's dialog
greys it, but the VM keyed `x 12` by `x`). The settings are kept for the
tab (`sort_form`) and come back when the panel is opened again. Sort runs
`Command::SortParagraphs` or `TableEdit::SortRecords` as one undo step; the
sorted paragraphs stay selected, in a table the caret goes to the first
sorted row. Refusals show under Sort: a formula cell in the sorted rows
(3113), a table with footnotes (3193), "Already in that order."; the
questions (3112 a page or column break in the text, 3184 connected cells,
which Continue disconnects) show with Continue and Cancel. Not built: the
DDE escape question (3186; the model has no DDE escapes) and the
too-large selection refusal (3215). Ami Pro's heap sort is reproduced, so
equal keys end in Ami Pro's order (`sort.rs`).

**Forms (TN7)** have no new interface; the recipe of Ami Pro's guide
(chapter 16) is walked as a scenario.

**Implementation notes** are added here as the run builds each item.

## 42. Objects: OLE, Paste Special, image adjustments, Charting, Equations (run D) — built 2026-11-07 (DO.3–DO.10), physical check pending; decisions OD1–OD12 answered 2026-11-06 (recommended options)

Run D (`objects`; `docs/OBJECTS_RUNBOOK.md`). All editing is non-modal: rail
inspectors beside the live page, values typed in place (edit in place), each
change one `Command` and one undo step, written as Ami Pro writes it. An
object the app cannot read is kept byte for byte, drawn from the best stored
rendering, never refused (OD9).

**42.1 OLE Object section (OD7).** Selecting an OLE frame shows an Object
section in the picture inspector: Class (for example the server's name),
Native data size, Presentation (metafile, size in inches), and for a linked
frame Application, Topic, Item as read-only text. No Edit or New Object
(there are no servers); the status bar says so when Enter or double-click is
used ("Ami Pro starts the server here; this app keeps and draws the object").
Convert to Picture (OD7b) in the section replaces the object by its
presentation metafile as an ordinary picture, one undo step. Edit › Links…
(as built; the draft called it Link Options…) opens a read-only rail list of
OLE objects and linked frames (kind, source, status); Update, Unlink and Edit
are not offered.

**42.2 Paste Special (OD10).** Edit › Paste Special… opens a rail list of the
clipboard's formats (text, rich text, HTML, image formats, files), the best
selected. Paste as Picture makes a frame, Paste as Text pastes text; one undo
step; plain Paste unchanged. A format the app cannot hold shows Ami Pro's
message 3092 in the status bar. Paste Link and Insert › New Object… are not
offered.

**42.3 Image adjustments (OD8).** Tools › Image Processing… (and the Picture
section of the rail) for any bitmap: Brightness, Contrast, Edge Enhancement
and Smoothing as typed boxes with sliders, Invert as a check, Posterize
threshold, Halftone mode as a print setting; Apply is live (OD11 live
preview), Revert restores the original. Adjustments are non-destructive in
the native file; Save As SAM bakes them into the bitmap. Double-click is
unchanged.

**42.4 Charting panel (OD3–OD5).** Tools › Charting… on a selected empty
frame, chart frame or empty table cell (a new frame from Create Frame when
none). A Chart rail section: the twelve type icons (column, stacked column,
bar, stacked bar, line, area, line and picture, pie, expanded pie, picture,
stacked picture), variation, Legend, Grid, 3D with Depth and Perspective,
colour set (sixteen swatches, drag to reorder), Axis (Automatic, Inc, Min,
Max), Flip data, a Data grid whose cells are typed in place (labels in the
first row and column), Pictures… and Drawing…. Data from the clipboard
follows Ami Pro's parsing (tabs or spaces, `_` for a space, currency and
commas accepted). Double-click opens the panel, or Draw mode when the chart
was last edited with Drawing. Chart from Table (OD11) uses the selected
cells. Copy Chart as Image and Save Chart Data as CSV are in the section's
menu. Charts are stored as the type-15 record plus generated objects.

**42.5 Equation editor (OD2, OD6).** Tools › Equations… opens the in-place card (the source under the frame;
see the implementation notes below, which govern); the rail holds the template palette
(fraction, radical, scripts, operators with limits, brackets, matrix,
function, label, over/under), Math/Text mode, Greek and Symbol keyboards,
and Preferences. Save As Equation… and Import Equation… read and write TeX
files; pasting LaTeX inserts the equivalent structure (OD11). The stored form was established (EQV1), so the metafile fallback was not
needed.

**42.6 Extras (OD11).** Chart from Table, Copy Chart as Image, Save Chart Data
as CSV, equation LaTeX paste, Convert Object to Picture, live preview while a
slider is dragged. Items Ami Pro cannot store are native-only and listed by
Save As.

**42.5 implementation notes (DO.9, built 2026-11-07; `equation_panel.rs`,
`equation_commands.rs`, `amipro_pictures::equation`).** Tools › Equations…
(Tools, after Chart from Table; page surface, outside Draw mode) and a
double-click or Enter on an equation frame open the editor; an empty text
frame, an empty table cell, or the main text gets a new empty equation (a
Flow with text frame with no line, shadow or margins, 220 by 300 twips: Ami
Pro's empty box). The editor is two parts that share one state: the **card**
(a popover over the page, under the frame it edits, so it scrolls with the
page) holds the TeX source in a multi-line text field with the caret in it;
the **rail panel** holds the template palette and the options. Esc or Enter
in the card leaves (what was typed is on the frame); the panel's X closes
too. Typed where shown: no separate value boxes except the size (points).

- *Source and drawing.* The source is Ami Pro's dialect without the outer
  `$` (no `$` in it: one math run). Every edit that parses is applied at
  once (`Command::SetEquation`: the two stored lines, the natural size in the
  picture's scaling, the baseline in its pan, the frame fitted to the size)
  and the page redraws; a text that does not parse is not applied: under the
  card "Error parsing TeX: an argument is missing (message 5106) at line 1,
  column 134", the frame keeps the last good drawing. Messages: 5100 too
  complex, 5102 matrix over 10 by 10, 5105 too big, 5106 parse error.
- *Palette.* Twelve groups of buttons (38 by 32 px, a tooltip each): Fractions
  (fraction, binomial, text and display size), Radicals (square root, root
  with index), Scripts, Sums, integrals, products (with and without limits
  above and below), Brackets, Matrices (2 by 2, 3 by 3, bracketed, determinant,
  column), Over and under (bars, hat, vector, tilde, dot, braces, text over),
  Functions, Greek, Operators, Relations, Arrows. A button puts its template
  at the caret (or over the selection) with `⬚` where something is typed, and
  the first `⬚` is selected, so typing fills it. Face (four, the first is Ami
  Pro's `TimesNewRomanPS`) and Size (1 to 400 points) are stored in the
  equation's settings line.
- *Undo.* A template, a face or size change, an import, a pasted LaTeX is
  one undo step; typing after a template joins its step. Undo and Redo
  reload the card from the frame; Undo of the step that made the equation
  takes the frame and closes the editor.
- *Files and LaTeX.* Import Equation… reads a TeX file as Ami Pro's import
  does (a construct is math, plain letters around it stay text) and Save As
  Equation… writes Ami Pro's text (broken at the last space before column 79
  once a line reaches 70); both through the portal's dialogs. Copy as LaTeX
  copies the source (`\root{n} \of{x}` as `\sqrt[n]{x}`, `\matrix` as the
  matrix environment); Paste LaTeX and Tools › Paste LaTeX Equation read the
  clipboard's `$..$`, `$$..$$`, `\[..\]`, `\(..\)` or equation environment;
  Edit › Paste never makes an equation by itself.
- *Output.* The page, the PDF (vector paths and text), the printer and the
  image fallback of the exports draw the equation from its text like any
  app-drawn picture. Save As SAM stores Ami Pro's form (two lines in
  `[Embedded]`, `.tex`, the `[isd]` words with the width, the negative
  height and the negative baseline): no loss entry. An equation whose text
  is not two lines is kept as it is and the panel says so (OD9).
- *Open (physical or VM).* The look of the card and the palette in the
  COSMIC themes; Ami Pro opening an app-made equation and its repaint beside
  ours (EQV1 twin `EQT2.SAM`); the frame record's trailing `W B` words for a
  new equation frame; a visible mark on the frame while it is edited.

**42.4 implementation notes (DO.8, built 2026-11-07; `chart_panel.rs`,
`chart_commands.rs`).**

- *Entry.* Tools › Charting… (Tools, after Image Processing…) reads the
  place as Insert › Image… does: a chart frame opens its panel; a frame
  holding a drawing or a picture opens the panel with the question "This
  frame holds a drawing. Charting replaces it with a chart." (Replace with a
  Chart, Cancel; an empty drawing asks too); an empty text frame, an empty table cell, or the main text
  gets a new chart: the clipboard is read once, and tab-separated text with
  numbers is the data, else the sample (CHV2's: 3 categories, 2 series) with
  the line "The clipboard held no tab-separated data, so the chart starts
  with sample data." The panel is open (and says it is looking at the
  clipboard) until the answer arrives. A new frame is Flow with text at the
  caret, 3.2 by 2.7 inches within the column (Ami Pro's chart frame). The
  caret in a table cell that holds text puts a new With para above frame
  after the table. Anywhere else (a frame holding text, a header) the status
  bar says what Charting needs. One undo step; Undo closes the panel with the
  frame.
- *Chart from Table* (Tools, after Charting…; also a button in the panel
  while the caret is in a table, and what Charting… does with two or more
  selected cells): the selected cells, or the whole table when only the caret
  is in it, become the data by Ami Pro's label-versus-number rule (a first
  row of words makes the categories, a first cell of words in a row its
  series; an empty cell among numbers is 0; numbers in the first row are
  data, as in Ami Pro: `_2020` makes one a label). The chart goes in a With
  para above frame (No wrap around, default size) anchored after the table's
  anchor; the data is a copy.
- *The panel* (rail, 340 pixels, one panel at a time): Type (the twelve
  icons as the app's own drawings, two rows of six in Ami Pro's order, the
  name below, two radios for the family's variations: Clustered or Stacked,
  Lines or Areas, one marker per series or per category, Whole or Expanded);
  Options (Legend, Grid, 3D, Perspective, Depth typed with a slider; 3D and
  Depth for columns, bars, areas, and pies, Perspective only with 3D and not
  for pies, as the dialog's enabling rules; a pie's 3D is stored with
  perspective depth as Ami Pro's pies are; Depth 1 to 100); Colour set (the
  sixteen entries as swatches, one selected, its colour button below with
  the app's colour panel); Axis (not for pies: Automatic, Inc, Min, Max;
  Automatic shows the computed values greyed; unticking it makes them
  editable; Enter or leaving a box applies; a wrong box says why in place:
  Inc above 0, Max above Min, at most 100 steps); Data (the grid of
  series by categories, typed in place, applied on Enter or when the box
  loses the focus; Add and Remove Series and Category; Flip Data; the Data
  text, which takes Ami Pro's parser, with its refusals (no numbers, more
  than 5,000 values) shown under the box and Apply Data); Copy Data as CSV;
  Save Chart as Image… (the portal's save dialog, PNG or SVG, the drawing
  renderer Save Drawing As uses); Chart from Table; Drawing….
- *Changes.* Every control regenerates the chart's drawing and sets it as
  one undo step; the page shows it at once. Text boxes apply on Enter or
  when they lose the focus; anything else the user then does applies pending
  text first, in the same step, or drops a refused text with its reason in
  the panel; Esc closes the panel and drops what was typed and not applied.
  A Depth slider drag is one step; so is a colour dragged in the picker.
  An Undo or Redo is followed by the panel.
- *Pictures…* is not offered: the app draws the markers and pictures of the
  picture types itself (the panel says so); Ami Pro's picture list
  (`AmiChart.ini`) is not read.
- *Drawing…* opens Draw mode on the chart's frame (the panel closes). The
  frame is remembered: a double-click opens Draw mode on it until Charting
  changes it again (the panel then says that the next change replaces the
  drawing edits); any other double-click on a chart opens the panel.
- *Unreadable chart* (a chart frame whose data record is not in the known
  layout): "This chart's data record is not in a form the app can edit; it is
  kept as it is." The panel offers Replace with a New Chart and Drawing…;
  Save As SAM writes the bytes read.
- *A caution* line says when the chart is of a kind drawn from the notes and
  not compared with Ami Pro's own drawing (`chart_gen::confidence`).
- *Not done:* Ami Pro's Pictures dialog, drag to reorder the colour set,
  a live link from a table (OD4b), a data grid larger than 30 by 10 shown
  (the Data text holds all).

**Status (2026-11-07).** Built: 42.1 to 42.6. Not built: Update, Unlink and Edit of OLE objects, Paste Link, New Object, Ami Pro's Pictures dialog for charts, colour-set reorder by drag, a live link from a table to a chart. The notes above are the as-built record; `OBJECTS_PROGRESS.md` has the choices OK-DO4 to OK-DO9.

## 43. Help › Check for Updates… (P6.2) — decisions taken 2026-11-07 (the user's recommended defaults)

*Where.* Updates are the GitHub Releases of the public repository
(`nchiker/AmiPro-Linux`); no token is used. Help › Check for Updates… (before
About) opens one rail panel, "Updates", under the single-panel rule
(`rail_panels.rs`: `RailPanel::Updates`); Esc and the X close it. The state
(`Status`) belongs to the program, the panel to the tab that opened it.

*When.* Only when asked, plus an optional quiet check at start: Preferences ›
General › "Check for updates when the program starts (once a day at most)",
off by default. The quiet check says nothing unless a newer version exists, and
then it opens the panel.

*What it shows.* The installed version, then by state:

- *Checking…* (Check again greyed).
- *Up to date:* "You have the latest version (x.y.z)." and Check again. A
  build's `+git…` metadata never counts: `0.1.0+git1a2b3c4` is current when
  `0.1.0` is the latest; a pre-release is older than its release.
- *Newer:* "Version X is available.", "What's new in X" with the release notes'
  first 20 lines, Download and Install, Check again.
- *Downloading* with a moving bar; then, verified, "downloaded and its checksum
  matches", the exact command (`pkexec apt-get install -y <file>`) and Install /
  Not now (the confirmation). Install shows "Installing… (enter your password
  when asked)"; polkit asks once. Then "Version X is installed." and Restart:
  the new version starts and this window closes the usual way (unsaved
  documents are asked about first).
- *No pkexec or apt-get:* the package is still downloaded and verified, and the
  panel shows the command to run in a terminal.
- *Failed:* the reason (no network, GitHub's rate limit, no release yet, a
  release without a package or without `SHA256SUMS`, checksum mismatch, the
  password declined) and Check again.

*Safety.* HTTPS only (curl with `--proto =https`, timeouts, a size cap; each
redirect is followed by hand and only to github.com and githubusercontent.com
hosts). The package must be named `amipro-linux_<tag>_amd64.deb` and come from
the repository's release downloads. The release's `SHA256SUMS` must list it and
match, and its size must be the one the release states; otherwise the file is
deleted and nothing runs. The download lives in a new 0700 directory under
`$XDG_RUNTIME_DIR`, removed after the install or when declined. The program runs
nothing but the install line (and starts itself again for Restart).

*Not done:* a byte-by-byte progress bar (the bar moves, the size is stated),
signature verification beyond the checksum (the checksum and the package come
from the same release), delta updates, other platforms.

## 44. Crash recovery: File › Recover Documents… (P6.3) — decision taken 2026-11-07 (the user's)

*Rule.* Autosave never writes the user's own file. Open documents with unsaved
changes are copied, periodically, to a hidden recovery folder of the program's
own; after a crash (power loss, `kill -9`) the next start offers them.

*Where the copies live.* `$XDG_DATA_HOME/amipro-linux/recovery/<pid>-<start>/`
(default `~/.local/share/...`), one 0700 folder per running instance (its
session) holding a `lock` (the instance's pid and process start time), and for
each document `<id>.amipro-recovery` (the lossless native v3 encoding, so
nothing is lost even for an import or an Ami Pro document) and `<id>.meta`
(title, original path, kind, time, size). Files are written whole beside their
name and renamed into place. Logic and tests: `app/cosmic/src/recovery.rs`
(the clock and the process probe are a trait, faked in tests).

*When a copy is written.* Every N minutes (Preferences › General › "Keep
recovery copies while editing", on by default, and "Write a recovery copy
every" 1, 2, 5, or 10 min; default 2) each document that has unsaved changes
and differs from its last copy is written; the encoding and the write run on a
thread. A document with a password (File › Password…) is never copied, because
the copy would hold it unprotected. A copy over 64 MB, or a session folder over
512 MB, is refused with a status line.

*When a copy goes.* When the document is saved (or undone back to the saved
state), when its tab is closed with Discard, and at a clean exit (the whole
session folder). Nothing else removes a live instance's copies.

*Two instances.* A session whose process still runs (same pid and start time)
is never offered, so two windows do not take each other's files. A session
whose process is gone is stale: its copies are offered at the next start.
Stale sessions with no copy are tidied, and stale sessions last written more
than 30 days ago are deleted at start.

*The panel.* File › Recover Documents… (after Revert to Saved…) opens the rail
panel "Recover documents" (`RailPanel::Recovery`, single-panel rule), and it
opens by itself at start when copies exist. For each copy: the file name (or
the title of a never-saved document), its original path or "Never saved", the
time of the last copy and its size, and the buttons Recover and Discard; below,
Discard all (with two or more). Esc and the X close the panel and keep the
copies. A copy that cannot be read (a damaged note or a document the reader
refuses) shows a line saying so and is kept on disk.

*Recover.* Opens the copy as a new tab named "name (recovered)", marked
changed, whose location is the original file as a read-only source: Save goes
to Save As (which proposes the original folder and name), so the original is
never overwritten silently; saving over it goes through the usual overwrite
confirmation, the loss check, and the `.bak` copy. The recovered tab keeps its
recovery copy until the document has a copy of its own (the next autosave), or
until it is saved or closed. The suffix goes when the document is saved.

*Not done:* recovering a document's password, undo history, or scroll position;
a recovery of the Merge or Master set windows (their documents are ordinary
tabs).
