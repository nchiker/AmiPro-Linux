# Modal dialog review, 2026-11

The user's decision after the UI review (`UI_REVIEW_2026-11.md`, finding
19): the remaining modal dialogs stay modal (they are one-shot questions and
actions, unlike the rail's inspectors), but their format is reviewed and
made consistent. The conventions that came out of it are `UI_SPEC.md` §50,
"Modal dialogs".

## Method

- **Inventory.** Every dialog `AmiProApp::dialog` can return, which is also
  the list Esc handles (`dialog_keys`, formerly `dialog_cancel`) and the
  list the Esc test opens (`escape_dialogs`): about 30 dialogs. All are built on
  libcosmic's `widget::dialog` (title, body, controls, primary, secondary,
  and tertiary buttons; 570 px wide unless set; the theme's padding), so the
  frame was already shared. What differed was what each dialog put in it.

  | Area | Dialogs |
  |---|---|
  | Text | Font, Special Effects, Special Character, Fast Format (ask) |
  | Insert and page | Floating Header/Footer, Page Number (Insert/Change), Create Frame, Insert Table, Insert Bullet, Outline Numbering, Go To |
  | Styles | Create Style, Rename Style, Style Management, Use Another Style Sheet, Save as a Style Sheet |
  | Files | New, Password (set), Protected Document (open), Import Options, Revert to Saved |
  | Questions and problems | Save changes (closing), Delete frame, Save with losses (SAM), Export with losses, Replace existing file, Could not open, Could not save, Power Field (Query$, Decide) |
  | Settings | Preferences, Customize Toolbar |

- **Screens.** `rail_panel_tests::modal_dialog_screenshots` (ignored; writes
  `dialog-*-light.ppm` and `-dark.ppm` to `AMIPRO_SCREENSHOT_DIR`) renders 29
  of them (all but Export with losses, the twin of Save with losses, and the
  Power Field prompt, whose look is Rename Style's) at 1280 × 800 through the headless window harness, light and dark,
  before and after the fixes; the images were read one by one.
- **Keys and focus** were read from the code: Esc goes through one table
  (asserted against the view); Enter worked only inside a text field with an
  `on_submit`; only the password box took the keyboard on opening.

## Findings, ranked by user impact

Decision key: **Fixed** (this review), **Deferred** (with the reason),
**Kept** (deliberate).

| # | Finding | Impact | Decision |
|---|---|---|---|
| 1 | **Enter did nothing in most dialogs.** It worked only while a text field with a submit action held the keyboard; in Font, Special Effects, Insert Bullet, Fast Format, Floating Header/Footer, Outline Numbering, New, Use Another Style Sheet, Import Options, Preferences, Customize Toolbar, and every question, Enter was ignored, and with no field focused it reached nothing. | High: every dialog, keyboard users | **Fixed.** One table, `AmiProApp::dialog_keys`, gives each dialog its Esc message, its Enter message, and its first field. A plain Enter that no widget took sends `Message::DialogEnter`, which does what the dialog's accent button does while that button is enabled. Destructive dialogs have no Enter (finding 3). |
| 2 | **No dialog but Password put the keyboard in its field**, so Go To (Ctrl+G), Create Style, Rename Style, Save as a Style Sheet, Create Frame, Insert Table, and the Power Field question needed a click before typing. | High: the commonest keyboard path (Ctrl+G, type, Enter) | **Fixed.** `dialog_keys` names the field; `update` focuses it when the dialog opens (and the password box again after a wrong password, as before). A widget test types into each through the window with no click. |
| 3 | **Questions put their buttons in three different orders.** Delete frame and Revert: Cancel, then the red action on the right. Save with losses and Export with losses: the red action, then an accent Cancel on the right. Closing with changes: an accent Save on the far left, Cancel, a red Discard on the right. Replace existing file: an accent (not red) Replace. | High: a confirmation read the wrong way loses work | **Fixed.** One order: the action on the right, Cancel beside it, an alternative on the far left. A destructive action is red and never the Enter default. Closing asks "Save changes to NAME?" with Save (accent, Enter) on the right, Cancel, and Discard (red) on the left. Save Anyway, Export Anyway, and Replace are red on the right with Cancel beside them. |
| 4 | **A dialog's third button had three looks**: Create Frame's "Draw on Page" and Insert Table's "Layout…" were text links, the Power Field's Cancel a text link, Create Style's "Modify…" and Use Another Style Sheet's "Browse…" standard buttons. | Medium | **Fixed.** All are standard buttons (`dialog_frame::alternative`), except where the button was really a disclosure (finding 5). |
| 5 | **Insert Table's "Layout…"** showed more of the same dialog but read as a command (ellipsis), and turned into "Less". | Medium | **Fixed.** A folding "Layout" section heading in the dialog (`chrome::section_toggle`, the panels' look). |
| 6 | **Labels lined up at four widths** (Font 56 px, Insert Table 110, Create Frame 120, Page Number 170) and some controls had none: Font's colour button, Go To's item list, and Go To's page box (named only by the body text "Page number, 1 to N"). | Medium | **Fixed.** One label column (`dialog_frame::LABEL_WIDTH`, 170 px) and `dialog_frame::row` in Font, Go To, Create Frame, Page Number, and Insert Table's Layout. Font: Face, Size, Colour. Go To: Page (with "1 to N" beside the box), the page buttons under it, Item (the list with Previous and Next). |
| 7 | **Lengths without a unit.** Create Frame's four boxes and Insert Table's column width and row height took inches but showed no unit (Create Frame said "measured in inches" in its body); Font's size box showed none either. | Medium | **Fixed.** "in" after each length box (`dialog_frame::length`, §50's "in"), "pt" after Font's size box; the body's "in inches" went. |
| 8 | **Button labels in two cases**: Go To's "First page", "Last page", "Next frame" among Title Case buttons everywhere else ("Draw on Page", "Reset to Standard", "Choose Another Name…"). | Low–medium | **Fixed.** First Page, Last Page, Next Frame. |
| 9 | **Close-type verbs**: Preferences and Style Management say Close; Customize Toolbar said Done. | Low | **Fixed.** Close (the dialogs that apply each change at once). Draw mode's Done is a mode, not a dialog, and stays. |
| 10 | **Section headings in two sizes**: Floating Header/Footer's "Apply to" was a caption while Create Frame, Import Options, Use Another Style Sheet, and Style Management use headings. | Low–medium | **Fixed** (`dialog_frame::heading`). |
| 11 | **Titles that did not match their command**: the Power Field question was "Power field" (sentence case); File › Password… opened "Password Protect". | Low | **Fixed.** "Power Field"; "Password" (the opening dialog stays "Protected Document", UI review finding 17). Dialogs a bare-noun Insert command opens keep the verb Ami Pro's dialogs have (Insert Table, Insert Bullet, Insert Page Number, Create Frame). |
| 12 | **A refusal shown two ways**: the password dialog's "do not match" was body text; every other dialog shows a refusal as a caption under its field. | Low | **Fixed** (`dialog_frame::hint`). |
| 13 | **Revert to Saved's text was Ami Pro's own sentence** ("This will undo all the changes… Are you sure?"), against the program's-own-words rule (UI review finding 17). | Low | **Fixed.** "The changes made since the document was last saved are discarded, and it shows the saved file again." |
| 14 | **Width**: Customize Toolbar is 720 px (two lists side by side) and the New dialog's list and preview fill 570 px; every other dialog is libcosmic's 570 px. | Low | **Kept.** The wider one needs its width; one width for all would squeeze it or pad the rest. |
| 15 | **Labels above or beside**: a single full-width text box has its label above (Password, Create Style, Save as a Style Sheet, Page Number's Leading text); short boxes and lists have theirs beside (Font, Create Frame, Page Number, Insert Table, Go To). | Low | **Kept**, written down as the rule (§50). |
| 16 | **Tab order** follows the order controls are drawn (top to bottom, left to right) in every dialog, the toolkit's own; no dialog sets another. | — | **Kept** (consistent). |
| 17 | **Font's size is a list and a box** (Ami Pro's list and edit). | Low | **Kept.** The list gives the usual sizes, the box any size; both are Ami Pro's. |
| 18 | **Floating Header/Footer nests three radio groups** (Insert or Remove, header or footer, which pages) and says "Remove: there is no floating header or footer at the caret." in place of a disabled radio. | Low–medium | **Deferred.** A clearer split (Insert and Remove as two sections, or two commands) changes Ami Pro's dialog shape and its tests; worth doing with the user's view. |
| 19 | **Go To mixes three jobs** (a page, the first and last page and next frame, and the next item of a kind). | Low | **Deferred.** It is Ami Pro's Go To; the review's earlier ranking already suggests an in-place page field on the status bar for a later run. |
| 20 | **Number boxes accept any text and refuse on OK** (Create Frame, Insert Table, Page Number, Go To), with the reason under the box. | Low | **Kept.** Consistent across the four; a spin control would be new UI with no clear gain. |
| 21 | **Special Character has no accent button** (a click inserts and closes). | — | **Kept** (a palette); its Enter does nothing, its Esc cancels. |

## Verification

- Model and controller: `enter_never_confirms_a_destructive_dialog` (each of
  the six dialogs without an Enter: Enter leaves it open and changes
  nothing), `every_other_dialog_names_its_enter_and_esc_and_enter_differs_from_esc_in_a_question`,
  `enter_acts_as_the_accent_button_and_waits_while_it_is_disabled` (Font,
  Special Effects, Insert Bullet, Go To, Create Style before and after a
  name, Preferences, and no dialog), the existing
  `escape_cancels_each_dialog_and_applies_nothing`.
- Keys through the subscription: `only_a_plain_enter_no_widget_took_goes_to_the_dialog`
  (the extracted `window_event_message`).
- Widget layer: `a_modal_dialog_keeps_enter_from_the_page_and_its_accent_button_takes_it`
  (the real window with the page holding the keyboard: the page gets no
  Enter under Insert Bullet, the widgets leave the event uncaptured, the
  subscription sends it to the dialog, the bullet goes in) and
  `a_dialog_with_a_value_to_type_opens_with_the_keyboard_in_it` (typing with
  no click lands in Go To, Create Style, Create Frame, and Save as a Style
  Sheet; it fails with the focus code turned off).
- Screens: `modal_dialog_screenshots` re-run after the fixes, light and dark.
- Physical: pending with the user (below).

## What the user should check on the desktop (ranked)

1. **Ctrl+G, type a page number, Enter**: no click needed. Then Format ›
   Font…, change the size, Enter applies.
2. **Close a changed document** (Ctrl+W): "Save changes to …?" with Save on
   the right; Enter saves; Discard is red on the left.
3. **Delete a frame** (select it, Delete) and **Revert to Saved**: Enter does
   nothing; the red button on the right does it.
4. **Insert › Table…**: the Layout section opens in place; lengths show "in".
5. **Insert › Frame…**, **Format › Font…**, **Edit › Go To ▸ Go To…**: the
   labels line up in one column; does the 170 px label column look too wide
   in Font?
