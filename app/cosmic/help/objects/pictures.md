---
id: pictures
title: Pictures
area: objects
keywords: picture, image, insert image, graphics scaling, crop, frame, brightness, contrast, paste special, link
see_also: pages-frames-layouts, drawing, charts
context: command:Image, command:InsertFile, command:ImageProcessing, command:PasteSpecial, command:Links, panel:PasteSpecial, panel:Links
---
A picture always lives in a [frame](pages-frames-layouts).

## Putting one in

- **Insert › Image…** (Ctrl+Shift+I) opens a picture file. With an empty frame selected, the picture goes into it; with the caret in text, a new frame is made at the caret.
- You can also paste an image, or drop a file on the page.
- **Insert › File…** inserts the text of a document instead.

BMP, WMF, EMF, PNG, JPEG, GIF, SVG, TIFF, and PCX pictures, and Ami Pro drawings, are read.

## Sizing and position

**Format › Frame › Graphics Scaling…** chooses original size, fit in frame, a percentage, or custom, and can keep the aspect ratio. Double-click a picture frame to move the picture within it, which crops it; Esc returns.

## Adjusting

**Tools › Image Processing…** changes brightness and contrast, smooths, sharpens edges, inverts, or halftones a bitmap. Revert undoes the changes.

## Linked and embedded objects

**Edit › Links…** lists linked pictures and embedded objects. **Edit › Paste Special…** pastes the clipboard as a particular type. Embedded OLE objects from Ami Pro files are drawn and kept but not edited; see [Known limitations](known-limitations).
