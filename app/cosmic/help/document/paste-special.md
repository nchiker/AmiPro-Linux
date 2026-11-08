---
id: paste-special
title: Paste Special
area: document
keywords: paste special, clipboard formats, html, rtf, bitmap, png, files
see_also: clipboard, pictures
context: command:PasteSpecial, panel:PasteSpecial
---
**Edit › Paste Special…** opens a panel in the rail that lists the formats the clipboard offers, so you can choose one rather than letting Paste pick.

The program tests the clipboard for text, Rich Text, HTML, several image formats, and file lists, and lists the ones present. The first row is selected. Press **Paste**, or double-click a row, to insert it at the caret in one undo step.

- Text and HTML arrive as text; HTML loses its markup.
- Rich Text arrives with its formatting.
- An image, or the first picture file of a file list, becomes a picture.

If a row cannot be used, the status bar says why and the panel stays open. Linking and embedding objects are not offered: see [Links](links-panel).
