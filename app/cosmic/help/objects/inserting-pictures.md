---
id: inserting-pictures
title: Inserting pictures
area: objects
keywords: insert image, link to file, linked picture, embedded, paste picture, drop file, replace picture, clipboard image
see_also: pictures, picture-formats, picture-frame-actions, ole-objects
---
There are three ways to put a picture on a page: the Insert menu, the clipboard, and dragging a file onto the page.

## Insert › Image…

Choose **Insert › Image…** (Ctrl+Shift+I). The desktop's file chooser opens with a filter for pictures. Where the picture lands depends on what is selected:

- an **empty frame** selected: the picture goes into it;
- the caret in an **empty table cell**: the picture goes into the cell;
- otherwise a **new frame** is made at the caret, as large as the picture is at its own resolution, shrunk to fit the column and page. It flows with the text.

When a picture frame is selected, the same command replaces that picture and keeps the frame, its scaling and its placement. The command is unavailable in a cell that already holds text.

The whole insertion is one undo step, and the new frame is left selected.

## Copy or link

The file chooser has a **Link to file** box. Unticked (the default), the picture is copied into the document, so the document is complete on its own. Ticked, the document records only the file's path and draws the file as it is now. If the file later moves, the picture is looked for by name in the document's folder; failing that, Ami Pro's stored copy (if the file came from Ami Pro) or a box naming the path is shown.

Saving as an Ami Pro `.SAM` file always embeds a copy, because a Linux path means nothing to Ami Pro; the Save loss list says so.

## Paste and drop

- **Edit › Paste** puts an image from the clipboard on the page the same way as Insert › Image…. If the clipboard holds only a file address or path of a picture, the picture is read from it.
- **Edit › Paste Special…** lets you choose the format; see [Paste Special](paste-special).
- Dropping a picture file on the page inserts it.

If the file cannot be read, the status bar says so and nothing changes.
