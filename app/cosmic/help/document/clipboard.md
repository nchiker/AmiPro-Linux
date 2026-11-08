---
id: clipboard
title: Cut, copy, paste, and delete
area: document
keywords: clipboard, cut, copy, paste, plain text, delete, ctrl+x, ctrl+c, ctrl+v, rtf
see_also: paste-special, undo-redo, pictures
context: command:Cut, command:Copy, command:Paste, command:PastePlainText, command:Delete
---
**Cut** (Ctrl+X) and **Copy** (Ctrl+C) take the selection to the desktop clipboard; Cut also removes it. **Paste** (Ctrl+V) inserts the clipboard at the caret, replacing a selection. **Delete** removes the selection, or the character after the caret.

## What Paste brings

Paste looks at what the clipboard holds. Formatted text (RTF) from another program keeps its character formatting; otherwise plain text is pasted. If the clipboard holds only an image, or a path or address of an image file, a picture is inserted instead.

**Paste as Plain Text** (Ctrl+Shift+V) keeps just the characters and gives them the formatting at the caret.

If Paste finds nothing, the status bar says so; try again, since the desktop sometimes reports the clipboard a moment late.

To choose among the formats on the clipboard, use [Paste Special](paste-special). Each paste or cut is one undo step.
