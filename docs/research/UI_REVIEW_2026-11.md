# UI review, 2026-11

A senior UI/UX review of the whole interface at `ba7d4e0` (after View ›
Vertical Ruler, the Format Painter submenu, Save new documents as, and
drag autoscroll), followed by the fixes. The user's brief: fix
inconsistencies, and arrange areas that show too much information more
logically. Standing preferences respected: rail inspectors over modal
dialogs, edit in place, one convention for all siblings, improve on Ami Pro
where logical, Ami Pro's command names kept recognisable.

The conventions that came out of it are `UI_SPEC.md` §50; the menus are in
`UI_COMMAND_MATRIX.md`.

## Method

- **Inventory.** Every menu, submenu, and right-click menu was dumped from
  the registry (`MENU_LAYOUTS`, the `*_CONTEXT_MENU`s) with labels,
  shortcuts, and check behaviour (`menu_check`): 10 menus, 19 submenus, 5
  right-click menus, about 280 commands. The toolbar (`REVISION_2_TOOLBAR`,
  `action_icon_name`), the status bar, the tabs, both rulers, 28 rail
  panels (`RailPanel`), 4 Preferences pages, and about 30 modal dialogs.
- **Screens.** `rail_panel_tests::ui_review_screenshots` (ignored; writes to
  `AMIPRO_SCREENSHOT_DIR`) renders every rail panel, every Preferences page,
  and every dialog a command opens, light and dark, at 1280 × 800 through
  the headless window harness; the images were read panel by panel. Menus
  cannot be rendered headless (libcosmic panics in `menu_inner`), so their
  look stays a physical check; their content was audited from the dump.

## Findings, ranked by user impact

Decision key: **Fixed** (this review), **Deferred** (with the reason).

| # | Finding | Impact | Decision |
|---|---|---|---|
| 1 | **Rail panel headers differed.** Nine panels (Style, Page Layout, Table, Document Properties, Fields, Print, Import Folder, Export Folder, Import Report) had "‹ Styles" (back to the list) and an X that hid the rail; seventeen had only the X, so the only way back to the Styles list was to close the rail and open it again; and the X of Paragraph Properties and Proofing kept the rail open, the opposite of every other X. Each panel built its own header (two accessible-name styles, one with a misleading tooltip). | High: every panel, every day | **Fixed.** One shared header (`chrome::panel_header`), the design §15 and §18 approved: title, "‹ Styles", X. "‹ Styles" returns to the list with the rail open (`Message::RailBack` for panels without their own back action); the X closes the panel and the rail everywhere. A widget test clicks both on all 25 panels a command opens. Frame Properties keeps its own header (it follows the selection). |
| 2 | **Preferences › General was one flat list of nine unrelated settings** (undo, zoom, recent files, style sheet, save format, backup, updates, recovery, interval), and the "Save new documents as" label wrapped under its buttons (overlapping text, screenshot). The recovery interval stayed visible, disabled, with copies off. | High: the brief's "too much information" | **Fixed.** Sections: New documents, Saving, Editing, Updates; the format buttons on their own row under the label; the interval shown only while copies are on; Custom zoom moved to View. |
| 3 | **Preferences had a Toolbar page whose only switch duplicated the View page's "Toolbar" switch**; the View page had no Status bar switch although View › Status Bar is one of its siblings; window parts and page marks were mixed in one list, in neither menu's order. | High | **Fixed.** Toolbar page removed; View page: Window (Toolbar with Customize Toolbar… beside it, Ruler, Vertical ruler, Status bar), Zoom (Custom zoom), Show on the page (View › Show's order). |
| 4 | **Fast Format (Ctrl+T) and Format Painter (Keep On) (Ctrl+Shift+T) were the same action** (`arm_painter(KeepOn)` both) under two names in two places of the Format menu, and Fast Format was ticked for the one-shot too, so two items could be ticked at once. | High: same concept, two labels (brief) | **Fixed.** Format › Fast Format ▸ holds Fast Format (Ctrl+T; Ctrl+Shift+T kept as a second shortcut), Format Painter (one use), Copy Format, Paste Format. Each tick means one mode. A saved toolbar's Keep On button loads as Fast Format. |
| 5 | **The Edit menu was 31 rows**, seven of them navigation (Go To…, Go To Header/Footer/Ruler, Previous/Next Page, Select Next Frame) between Replace and Bookmarks. | Medium–high: menu length | **Fixed.** Edit › Go To ▸ (Go To… first, Ctrl+G unchanged), divided into pages/frames and header/footer/ruler. Edit is 25 rows. |
| 6 | **Tools mixed proofing, objects, and settings in one 12-row block**, with Word Count (a proofing figure) after a divider among Footnote Options and TOC. | Medium | **Fixed.** Groups: proofing (with Word Count), Sort…, objects, notes and contents, settings. |
| 7 | **File menu grouping**: Import Folder… sat with Open while Export Folder… was inside Export ▸; Recover Documents… sat among the save commands; Export ▸ and five print commands shared one group. | Medium | **Fixed.** Recover Documents… after Open…; Import Folder… just before Export ▸; printing its own group. |
| 8 | **Panel titles did not match their commands or each other**: "Recover documents" (sentence case among Title Case), "Outline" for Outline Navigator, "Chart" for Charting…, "Equation" for Equations…, "Updates" for Check for Updates…, "Contents & Index" for TOC, Index…. | Medium | **Fixed.** Title = the command without its ellipsis, Title Case: Recover Documents, Outline Navigator, Charting, Equations, Check for Updates, TOC and Index. Panels several commands open keep their names (Proofing, Fields, Bookmarks). |
| 9 | **Two looks for folding sections**: Proofing's "▸ Options" and "▸ User dictionary" were small text links; every other panel uses an arrow icon and a heading. | Medium | **Fixed** (`chrome::section_toggle`). |
| 10 | **A panel's main action had no consistent weight**: Print…, Sort, Import, Export, and Fields' Insert were accent buttons; Generate Contents/Index, Start Spell Check, Start Grammar Check, and Paste Special's Paste were plain. | Medium | **Fixed.** All are full-width accent buttons (Paste stays compact; it sits under the format list). |
| 11 | **The same thing named differently in a panel and a menu**: Table Properties' "Header"/"Banded" vs Table › Table Style ▸ "Header Row"/"Banded Rows"; Glossary's "Data File…"/"New…" vs Merge's "Open…"/"New Data File…"; the Print panel's "Doc Info" vs File › Document Properties…; the New… dialog titled "New Document", the name of another command (Ctrl+N). | Medium | **Fixed.** Header Row, Banded Rows; Open Data File…, New Data File…; "Document Properties"; the dialog is titled New. |
| 12 | **Previous Document showed Ctrl+Shift+Tab while Next Document showed Ctrl+PageDown** (Ctrl+Tab is taken by COSMIC): an unmatched pair. | Medium–low | **Fixed.** Ctrl+PageUp is shown; Ctrl+Shift+Tab still works (`SHORTCUT_ALIASES`), and the Keyboard Shortcuts list shows "Ctrl+PageUp or Ctrl+Shift+Tab". |
| 13 | **Spelling and punctuation**: "Center" (Format, Paragraph, Style Properties) vs "Centre" (Draw › Align ▸, the ruler's tab tooltip, the envelope's feed); "Line and Fill…" beside "Extract Line & Fill" in one menu; "Tabs & Returns"; "Snap To" (an unfinished phrase). | Low–medium | **Fixed.** Center throughout (Ami Pro's command name); "and" in labels; Snap to Grid. |
| 14 | **Custom Zoom gave no hint of its percentage**, while its sibling reads "Enlarged (200%)". | Low | **Fixed.** "Custom Zoom (91%)" from the preference. |
| 15 | **Format › Frame ▸ had no dividers** while the frame's right-click menu, with the same items, has three. | Low | **Fixed.** Same grouping. |
| 16 | **Units**: "in." in Page Layout and Style/Paragraph Properties, "in" in Footnotes, Drawing, Envelope, the status bar, and the rulers. | Low | **Fixed.** "in" everywhere. |
| 17 | **Dialog labels with trailing colons** (Password's "Enter password:", Create Style's "Style:", Special Effects' "Overstrike character:"), and a shouted "WARNING -" line copied in spirit from Ami Pro's dialog; the opening password dialog was titled "Password Protect". | Low | **Fixed.** No colons; "Password", "Confirm password"; the warning in the program's own words; the opening dialog is "Protected Document". |
| 18 | **Esc closes only some panels** (Equations, Charting, Recover Documents, Help, Check for Updates, Sort) and not the others. | Medium | **Deferred.** The Esc subscription sees every Esc, including the one that ends Fast Format, Draw mode, a frame selection, or the find bar; making it close any panel would close Style Properties when the user only meant to end a mode. Needs a focus rule (Esc closes the panel only when the panel holds the keyboard), which the toolkit's focus reporting does not give reliably (`CLAUDE.md`: a focused field gets no `on_focus` from a task). |
| 19 | **Many modal dialogs remain** (Font, Special Effects, Create Frame, Insert Table, Insert Bullet, Page Number, Floating Header/Footer, Outline Numbering, Go To, Create Style, Style Management, Use Another Style Sheet, Save as a Style Sheet, Special Character, Password, New…, Preferences, Customize Toolbar) against the rail-first preference. | Medium | **Deferred.** Each is a design decision for the user (§15 deliberately kept one-shot actions as dialogs). Candidates ranked for a later run: Font and Special Effects (already mirrored by Style Properties' Font section) → a "Text" section of Paragraph Properties; Insert Bullet and Special Character → one rail palette; Go To → the status bar's page button as an in-place field. |
| 20 | **Promote and Demote use the indent icons** (`format-indent-less/more`), the same icons as Decrease/Increase Indent; both pairs can be on the toolbar at once, and the standard toolbar's "indent" buttons are really outline levels. | Medium | **Fixed** (2026-11-25, the user's decision). Original icons `outline-promote-symbolic` and `outline-demote-symbolic` (elbow arrows from a level line); the Standard toolbar keeps its items and order. `outline_and_indent_commands_have_four_distinct_icons_that_exist`. |
| 21 | **View › Show Power Fields and Show Clean Screen** keep "Show" while their checked siblings are nouns (Notes, Pictures). | Low | **Deferred.** Both are Ami Pro's exact names, and "Power Fields" alone would collide with Edit › Power Fields ▸ in the Keyboard Shortcuts list. |
| 22 | **Two Draw menu pairs share labels with Format › Frame ▸** (Bring to Front, Send to Back) for different objects. | Low | **Kept.** Ami Pro names both; each menu is about its own objects, and the Draw ones are disabled outside Draw mode. |
| 23 | **Dialog height changes between Preferences pages**, so the tabs jump. | Low | **Deferred.** libcosmic's dialog sizes to its content; a fixed height needs a scrollable body, to be checked on the desktop first. |
| 24 | **Stale status messages**: a panel's hint ("Choose a format to paste", "Equation: type TeX in the card…") can stay in the status bar after another panel replaces it. | Low | **Deferred.** Seen only when panels are switched programmatically in the screenshot run; the X path was not shown to leave them. Worth a scenario check. |
| 25 | **The Table and Draw menus are always present and mostly disabled** outside a table or Draw mode. | Low | **Kept.** User decisions TD1 and DW4 (Ami Pro shows them). |

## Checked and consistent

- Every toggle in the View, Show ▸, Outline ▸, Format, Table, Draw, and Tools
  menus carries a check mark, and a menu with one aligns every row (§13).
- New items fit: View › Vertical Ruler (checked, beside Ruler, a switch on
  Preferences › View, saved), Preferences › General › Save new documents as
  (now in New documents), drag autoscroll (no UI of its own), the Format
  Painter commands (now in Fast Format ▸).
- Shortcuts are shown in every menu and tooltip from the one registry, and
  are unique (tests); the menu notation "Ctrl + N" and the tooltip and Help
  notation "Ctrl+N" are libcosmic's and the app's respectively (kept).
- Light and dark: every panel and dialog renders with readable contrast in
  both; the rulers stay paper-coloured in dark mode by design (§48).

## Verification

- Widget layer: `every_panel_header_has_styles_back_and_an_x_that_closes_the_rail`
  clicks "‹ Styles" and the X through the real widgets on all 25 panels a
  command opens (it found the Sort, Paragraph, and Proofing differences).
  The command registry tests pin the new menu order, the aliases
  (`shortcut_aliases_are_unique_and_never_shadow_a_shown_shortcut`,
  Ctrl+PageUp and Ctrl+Shift+Tab), and the Format Painter scenario drives
  Ctrl+Shift+T through `command_for_shortcut`.
- Screens: `ui_review_screenshots` re-run after the fixes (Preferences'
  three pages, the panels' headers, Proofing's sections), light and dark.
- Physical: pending with the user (below).

## What the user should check on the desktop (ranked)

1. **The panel header.** Open a few panels (Modify Style, Sort, Notes,
   Help): "‹ Styles" goes back to the Styles list, the X closes the rail.
   Does one pair everywhere feel right, or should the X only go back?
2. **Preferences** (Ctrl+,): the three pages in sections, the page area of
   one height (scrolling on View), Customize Toolbar… beside the Toolbar
   switch, Status bar on View, recovery interval shown only while copies
   are on.
3. **Format › Fast Format ▸**: Ctrl+T and Ctrl+Shift+T both keep it on; the
   paintbrush button once; the tick on the right item.
4. **Edit › Go To ▸** and the regrouped File and Tools menus: do the groups
   read well at a glance (menus cannot be rendered headless).
5. **Ctrl+PageUp / Ctrl+PageDown** between tabs (and Ctrl+Shift+Tab still).
6. View › Custom Zoom shows its percentage.
