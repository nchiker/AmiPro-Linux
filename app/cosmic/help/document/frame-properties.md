---
id: frame-properties
title: Frame Properties
area: document
keywords: modify frame layout, frame inspector, frame settings, size, position, name, make default, restore default, select frame, move frame, resize frame
see_also: frame-placement-wrap, frame-lines-shadow, frame-text-columns, frame-layering-groups
context: command:FrameProperties, command:NextFrame, panel:Frame
---
Select a frame and the rail shows **Frame Properties**; Format › Frame › Frame Properties… opens it, and so does right-clicking a frame. Every field applies on Enter or when you leave it, as one undo step, and the page shows the result at once. A value that does not read is refused in place with the reason.

## Selecting, moving, and resizing

Click a frame to select it. Eight square handles appear. Drag from inside to move; a dashed outline follows and the text reflows on release. Drag a handle to resize; Shift on a corner keeps the proportions. Esc cancels a drag.

With the frame selected, the arrow keys move it by a point (Shift: a tenth of an inch) and Alt+arrows move its right or bottom edge. Tab and Shift+Tab, or Edit › Select Next Frame, select another frame on the page. Enter edits the text inside. Esc returns to the main text. Shift+click adds more frames to the selection.

## The sections

- **Size and position:** Page, then where it sits (From left and From top, or the offset from the margin for some placements), Width and Height, in inches.
- **Type:** [placement, wrap, and Opaque](frame-placement-wrap).
- **Lines and shadows:** see [Frame lines, shadow, and colour](frame-lines-shadow).
- **Columns and margins:** see [Columns and margins in a frame](frame-text-columns).
- **Name:** a name for the frame. Ami Pro's Run macro is shown as read-only text, since macros are not supported.
- **New frames:** **Make Default** keeps this frame's settings for frames you create next; **Restore Default** goes back to the original ones.

For a repeating frame the panel says the change applies on every page of the layout.
