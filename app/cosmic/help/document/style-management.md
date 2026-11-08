---
id: style-management
title: Style Management
area: document
keywords: revert style, remove style, move style, delete style, styles in document, missing style, rename style
see_also: styles, use-another-style-sheet, create-style, modify-style
context: command:StyleManagement
---
Format › Styles › Style Management… shows where each style comes from and lets you clean up.

## The lists

- **Styles in document:** styles you created or changed, stored in this document, marked with a bullet.
- **Styles in style sheet:** styles that still come unchanged from the document's sheet. The heading above shows the sheet's name, or says there is none.
- **Defined nowhere:** names the text uses that neither the document nor the sheet defines. They are shown in the error colour and format as Body Text.

## The buttons

- **<< Move <<** stores a sheet style in the document, so it can be changed for this document.
- **Revert** drops a document style's changes and goes back to the sheet's definition. It is off for a style that exists only in the document.
- **Remove** deletes a style from the document. Paragraphs that used it keep the name and format as Body Text until you apply another style.
- **Close** ends the dialog.

Each action is one undo step. Rename is in the Styles panel's row menu. To keep document styles for other documents, use [Save as a Style Sheet](save-style-sheet).
