# Interface Reference Comparison and Evidence Ledger

**Status:** Approved Revision 2 implementation reference, 2026-10-10. Gate A
is closed; evidence limits and unresolved original-app comparisons remain.
**Purpose:** Separate verified evidence from design inspiration and product
decisions. This document does not elevate inferred Ami Pro behavior to fact.

## Available sources and limits

### Original Ami Pro 3.1

The private audit verifies `AMIPRO.EXE` version 3.1.0.0 and identifies the
format-v4 `README31.SAM` as the Ami Pro 3.1 release document. Read-only string
inspection of that document provides the following UI evidence:

- Ami Pro used **SmartIcons**. Descriptions included Open, Save, Undo, Cut,
  Copy, Print, Drawing, Tile Windows, and Cascade Windows.
- SmartIcon sets could be ordered, modified, and deleted; bubble help/icon
  descriptions could be shown or hidden.
- Direct font formatting could be applied through SmartIcons, the status bar,
  or the **Text** menu.
- The **Style** menu included Modify Style and Save as a Style Sheet workflows.
- A note refers to modifying tabs in the current ruler and carrying an inserted
  ruler with a copied paragraph.
- Footer, Footnote Text, Frame, Header, and Ruler were available only in
  **Layout mode**.
- The release document names **View > Show/Hide Clean Screen** and **Help >
  QuickStart Tutorial**.
- It documents original spacing-character shortcuts and at least one historical
  command shortcut that conflicts with modern expectations; original shortcut
  parity therefore cannot be assumed.

These facts establish a compact command/icon workflow, style emphasis, and a
layout-mode ruler. They do **not** establish exact menu order, complete command
inventory, default SmartIcon layout, color, pixel spacing, enabled-state rules,
or the behavior of all status fields. No original screenshot or complete user
manual is present in the repository. Proprietary artwork and resources remain
outside Git.

Evidence locations:

- `docs/research/REFERENCE_AUDIT.md`
- private read-only `Reference/SAM/README31.SAM` / verified ISO counterpart
- `docs/research/REFERENCE_FEATURE_MATRIX.md`

### Gemini concept

The concept image `amipro-linux-ui-reference.png` (1536×1024; removed from the repository before publication) showed a
useful document-centered composition: traditional menu bar, compact icon row,
tabs, left Styles panel, horizontal ruler, centered page, and status/zoom row.
It also depicts behavior that is not established, including rich pagination,
multiple view modes, alignment/list/table/image actions, and three concurrent
style selectors.

The supplied **interactive Gemini COSMIC HTML could not be located** in the Git
worktree, ignored `Reference/`, default `../AmiPro-Reference/`, or via
`AMIPRO_REFERENCE_DIR` (unset). Consequently its interaction states, markup,
CSS measurements, responsiveness, accessibility, and control behavior were
not reviewed. The PNG is treated as conceptual artwork only. If the HTML is
later supplied, review it as inspiration—not production code or Ami Pro
evidence—and update this ledger before borrowing a behavior.

### Current AmiPro-Linux

Source inspection through UI-1 commit `85086b8` and current acceptance records
shows:

- One registry defines the complete eight-root command inventory, labels,
  shortcuts, enable rules, dispatch categories, toolbar routes, and
  implemented/future status.
- Pinned libcosmic `KeyBind` values generate trailing shortcut labels and the
  same values drive default/rich editor bindings with non-Latin physical-key
  fallback. Disabled future entries have no dispatch route.
- File commands, Undo/Redo, clipboard/delete/select-all commands, Bold/Italic,
  Styles routes, tab cycling and the checked open-document list, and native
  About are connected to real application/editor state. Imported Save still
  routes through protected native Save As.
- The action row uses text buttons without the icon/tooltip/group hierarchy in
  the concept.
- Style selection appears in the action row, Styles panel, status picker, and
  model-derived formatting state.
- Tabs retain independent document state, the page workspace scrolls, and the
  ruler/page geometry is model-derived.
- The 2026-10-09 physical result predates UI-1 and explicitly rejected menu
  completeness/shortcut visibility as well as the still-unmodified toolbar,
  duplicated style controls, hierarchy, and status spacing/readability. UI-1
  menu/shortcut behavior is automated-green but awaits focused physical COSMIC
  acceptance; the remaining visual concerns belong to later slices.

Automated implementation is evidence of capability, not visual acceptance.

### Pinned COSMIC/libcosmic/Iced

The application lockfile pins libcosmic/Iced to commit
[`dbc62f2`](https://github.com/pop-os/libcosmic/tree/dbc62f2d899cdbfbe43d170689ddc0290db663f4).
The checked-out primary source establishes reusable conventions:

- `src/widget/menu/menu_tree.rs` derives a displayed shortcut from a `KeyBind`
  map and renders it at the trailing edge; it ranks competing bindings
  deterministically. It also supplies individually disabled menu entries.
- `src/widget/menu/key_bind.rs` represents Ctrl/Alt/Shift/Super combinations
  and includes physical-key fallback for non-Latin layouts.
- `src/widget/button/icon.rs` defaults symbolic icons to 16 logical px, uses
  theme padding, supports tooltips, accessibility names/descriptions when the
  feature is active, and exposes a selected state.
- `src/widget/tab_bar.rs` defines native horizontal tabs with 44-pixel height,
  76–250-pixel width, and theme spacing.
- `src/widget/mod.rs` supplies themed tooltip presentation.
- `cosmic-theme/src/model/spacing.rs` supplies semantic density-aware spacing;
  standard values are 4/8/12/16/24/32/48/64/128 logical px.

These are toolkit capabilities, not an instruction to reproduce example-app
layouts. The approval candidate uses the native state/spacing machinery and
narrows tab widths for the document use case only where usability tests support
it.

## Comparison and decision table

| Concern | Verified Ami Pro | Gemini PNG | Current app | Proposed modern decision |
|---|---|---|---|---|
| Menu organization | Text, Style, View, Help and Window-related commands evidenced; complete order unknown | File/Edit/View/Insert/Format/Tools/Window/Help | UI-1 now supplies all eight roots and complete named inventory; physical review pending | Keep eight roots; complete named inventory; modern Format consolidates text/style commands |
| Shortcut discovery | Keyboard documentation existed; exact set incomplete and sometimes conflicts with modern norms | Shortcut labels not visible in open state | UI-1 uses one pinned-libcosmic key map for display/matching/dispatch; physical review pending | Right-aligned labels from one key registry; modern COSMIC defaults unless a compatibility mode is later justified |
| Frequent actions | Configurable SmartIcons with bubble help; Open, Save, Undo, Cut, Copy, and Print descriptions evidenced | One icon row | Text-button row | Full-width Document, History, Clipboard, Style, Character, and implemented Paragraph groups; labeled More only for responsive displacement; future Print slot after Save |
| Styles | Style menu, style sheets, and formatting via status bar evidenced | Panel + action style dropdown + status dropdown | Panel + two dropdowns + other state | Panel owns browse/manage; one action quick apply; no status style selector |
| Multiple documents | Tile/Cascade Windows evidenced | Tabs | Tabs with isolated state | Native document tabs; do not mimic MDI tiling without multi-window support |
| Ruler | Layout-mode ruler and inserted rulers evidenced | Always-visible horizontal ruler | Read-only model-driven ruler | Accurate read-only ruler now; future interactions only with undoable model support |
| Page surface | Layout mode evidenced; exact appearance unverified | Centered paper page | Responsive paper card with estimated pagination | Clearly edged portrait paper on recessed canvas; visible real margins; realistic text/selection state; shared render/hit/ruler transform without fake pagination |
| Status bar | Formatting through status bar evidenced; fields unknown | Page/section/words/style/language/mode/view/zoom | Page estimate + style dropdown + long message | Page/section/count/message/view/zoom only; style moves out |
| Side panels | No verified equivalent | Styles panel and illustrative Frame Properties | Collapsible fixed-width Styles list | One responsive context rail: Styles normally, future distinct Frame Properties state on frame selection; never two cluttering columns |
| Themes | Windows 3.x original, not a modern theme source | Light only | Native theme primitives, visual acceptance pending | Semantic COSMIC light/dark colors; screenshot/contrast gates in both |
| Unsupported features | Original provided many mature features | Many illustrative live-looking icons | Generic disabled menu rows; toolbar mostly working | Named disabled menu locations; no decorative toolbar controls |

## Intentional departures

1. **Tabs replace MDI tile/cascade as the primary document switcher.** This is
   a modern COSMIC product decision; Window retains a dynamic document list.
2. **Format replaces separate original Text and Style command homes.** Named
   styles remain prominent through the Styles panel and first Format item.
3. **Modern shortcuts win by default.** For example, Ctrl+A is Select All even
   if a historical context used it differently. A future “Ami Pro keymap” is
   possible only after the original set is documented and conflicts are tested.
4. **Toolbar customization is deferred.** Original SmartIcon customization is
   real, but a stable command registry and complete full-width grouping come
   first. Responsive `More` is labeled and never the only route to a common
   command.
5. **Status formatting controls are not copied.** They would recreate the
   duplication rejected by the user; the action style field and Styles panel
   provide clearer ownership.
6. **Layout mode is not invented yet.** The current interface presents an
   editable page. A future layout/object mode requires real behavioral and
   model need, not just an original label.

## Exact Windows XP VM questions

Run these against Ami Pro 3.1.0.0 with a disposable new document and
`README31.SAM`. Capture one screenshot per numbered item, note menu labels
verbatim, and do not modify the only copy of a source document.

1. **Menu inventory:** With ordinary text selected, open each top-level menu
   from left to right. What are the exact root names/order, item labels,
   separators, visible shortcut labels, checked items, and disabled items?
   Repeat in Layout mode with a frame selected and record only changes.
2. **Edit discovery:** Without using shortcuts, where are Undo/Redo,
   Cut/Copy/Paste, Delete, Select All, Find, and Replace? Does the Undo/Redo
   label name the action, and what conditions disable each item?
3. **Ctrl+A conflict:** In body text, in the Style workflow, and with a frame
   selected, what does Ctrl+A do? Record whether behavior is focus/context
   dependent and where Ami Pro documents the binding.
4. **Default SmartIcons:** On a clean/default profile, which icon sets/rows are
   visible, in what order, and what exact bubble-help text appears for each
   visible icon? Which icons show persistent active/toggle state?
5. **SmartIcon customization:** Where are show/hide bubble help and icon-set
   modification invoked? Are changes global, per document, or per mode?
6. **Style workflow:** With the caret in two differently styled paragraphs and
   then a mixed multi-paragraph selection, where is the current style shown?
   Which surface applies a style, which modifies its definition, and does a
   status-bar field open a chooser or dialog?
7. **Direct formatting:** Apply bold/italic/font through SmartIcons, status bar,
   and Text menu. Do all three show synchronized state for a caret, uniform
   selection, and mixed selection? How is “mixed” represented?
8. **Ruler/layout mode:** Enter and leave Layout mode. Which ruler(s), margin,
   indent, and tab markers appear; what units are used; and which markers are
   draggable? Does the ruler scroll with the page or remain viewport-fixed?
9. **Page and scroll behavior:** At 100% and Fit Width, resize the window and
   scroll a multipage document. How are paper edges, page gaps, margins,
   horizontal overflow, caret visibility, and zoom represented?
10. **Status bar:** Transcribe every field in normal text, mixed selection,
    Layout mode, and a selected frame. Identify clickable/editable fields and
    record tooltips or dialogs they open.
11. **Window workflow:** Open three documents. How are active documents named
    in Window; how do Next Window, Tile, and Cascade behave; and are dirty/source
    states shown anywhere in the document/window chrome?
12. **Clean Screen:** Invoke View > Show/Hide Clean Screen. Exactly which menu,
    SmartIcon, ruler, status, and window elements hide, and how is the UI
    restored by keyboard?

Answers should update this ledger and relevant acceptance cases. They should
not automatically reverse the intentional modern departures above.

## Bounded future architecture research backlog

Do not implement these during UI work. Before foundational page/frame/layout
changes, run one decision task at **Extra High reasoning effort** covering:

1. Stable semantic IDs and positions for pages, frames, tables, and linked text
   flow without making rendered fragments authoritative.
2. Anchor coordinate systems, wrap modes, z-order, overflow/link ordering,
   cycle prevention, and cross-frame selection in verified Ami Pro behavior.
3. Incremental shaped paragraph invalidation after actual edits, including
   following y-offset repair and viewport prioritization, measured against the
   18-page and 300+-paragraph workloads.
4. A single display/layout mapping for page rendering, ruler markers, caret,
   selection, IME, print/PDF, and accessibility.
5. Reuse cost for COSMIC Text, BetterOffice concepts, LibreOffice/Calligra or
   other maintained license-compatible sources, without creating a second
   document model.

Required output is a narrow decision, behavior tests, integration cost, and
documented limitations. Custom GPU rendering, a speculative engine replacement,
and interface-only frame controls remain out of scope.
