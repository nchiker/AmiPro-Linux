---
id: drawing
title: Drawing
area: objects
keywords: draw, drawing, draw mode, shapes, line, rectangle, ellipse, polygon, sdw, draw bar, amidraw
see_also: draw-tools, drawing-properties, draw-shape-editing, draw-files, draw-in-frames
context: command:Drawing, command:DrawDone, command:DrawSelectAll, panel:Drawing
---
Draw mode lets you build line art directly on the page, inside a frame. The drawing is made of objects (lines, shapes, text) that stay editable, and it prints and exports as sharp vector graphics at any zoom.

## Starting

- **Insert › Drawing…** makes a frame at the caret and opens Draw mode in it. If an empty frame or an empty table cell is selected, that is used instead.
- Double-click an existing drawing, or select its frame and press Enter, to edit it.

While Draw mode is on, the page around the frame is dimmed, the toolbar is replaced by the **draw bar**, the **Draw** menu is enabled, and the rail shows [Drawing Properties](drawing-properties).

## Working

1. Pick a tool from the draw bar and drag on the page ([Draw tools](draw-tools)).
2. Select objects with the arrow tool and change their look in the rail ([Line and fill](draw-line-fill)).
3. Move, turn, flip, group and stack them ([Editing shapes](draw-shape-editing), [Group and order](draw-group-order), [Align and grid](draw-align-grid)).

Every change is one undo step on the document's history, and Undo and Redo are on the draw bar. **Draw › Select All Objects** selects everything in the drawing.

## Leaving

Press Esc, choose **Draw › Done Drawing** (or the draw bar's Done), or click outside the frame. Leaving is not an undo step. The frame keeps the size it has; use Fit Frame to Picture (see [Picture frame actions](picture-frame-actions)) to shrink it around the art.

Drawings from Ami Pro files (`.sdw`) are read and written back without changing what you did not touch; see [Drawing files](draw-files).
