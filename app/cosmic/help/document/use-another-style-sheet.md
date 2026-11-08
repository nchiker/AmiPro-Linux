---
id: use-another-style-sheet
title: Use Another Style Sheet
area: document
keywords: change template, sty, style sheet, switch sheet, template, browse
see_also: styles, save-style-sheet, style-management
context: command:UseAnotherStyleSheet
---
Format › Styles › Use Another Style Sheet… replaces the document's styles with those of a different style sheet (`.STY`).

## Choosing the sheet

The dialog states the current sheet and lists the sheets found in the style-sheet folders. **List by description** shows each sheet by its description rather than its file name. **Browse…** picks a sheet from anywhere with the system file chooser, and its folder is remembered for next time. The folders are set in Preferences › Folders.

## What changes

- Styles stored in the document, marked with a bullet, keep their own definitions.
- Every other style takes the new sheet's style of the same name. If the new sheet has none, the paragraphs keep the name but format as Body Text, and the name shows in the error colour.
- The new sheet's other styles join the document's list.
- The page layout follows the sheet only where it was the old sheet's own; a layout you changed stays, and so do header and footer frames.

The change is one undo step. [Style Management](style-management) can revert or remove the styles that stayed behind, if you want the sheet's version.

A `.STY` file you open is never written over. To keep your styles as a sheet, use [Save as a Style Sheet](save-style-sheet).
