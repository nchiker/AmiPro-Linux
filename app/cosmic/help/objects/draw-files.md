---
id: draw-files
title: Drawing files and the clipboard
area: objects
keywords: import drawing, save drawing as, sdw, svg, png, clipboard, copy objects, symbols, export drawing
see_also: drawing, draw-in-frames, pictures
context: command:ImportDrawing, command:SaveDrawingAs
---
A drawing can be brought in from a file or saved to one, separately from the document that holds it.

## Import Drawing…

**Draw › Import Drawing…** adds the contents of a file to the drawing you are editing: an Ami Pro drawing (`.sdw`, including symbol drawings), an SVG (its shapes become polygons, polylines, curves and text), or a picture (a bitmap object).

## Save Drawing As…

**Draw › Save Drawing As…** writes the drawing as an Ami Pro drawing (`.sdw`), an SVG, or a PNG. If objects are selected, only those are written; the status bar says which happened.

## Clipboard

Copy and Cut in Draw mode put the selected objects on the clipboard in the program's own form, as SVG and as a PNG, so other programs can take them. Paste accepts the same, or an SVG, an image, or text.

## Ami Pro compatibility

Saving as a `.SAM` writes objects the way Ami Pro does (points, pens, fills, endings, text, groups, bitmaps), without Ami Pro's cached screen image or driver state, which it rebuilds on opening. Curves are true curves here; Ami Pro approximates them with line pieces, so they may look slightly different there. Line widths are whole twips up to 255.
