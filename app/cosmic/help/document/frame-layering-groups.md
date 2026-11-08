---
id: frame-layering-groups
title: Layering and grouping frames
area: document
keywords: bring to front, send to back, stacking order, overlap, group frames, ungroup, move together, z-order, arrange
see_also: frame-properties, frame-placement-wrap, draw-group-order
context: command:BringToFront, command:SendToBack, command:GroupFrames, command:UngroupFrames
---
Frames can overlap. Format › Frame has four commands for how they stack and travel; the right-click menu on a frame has them too.

## Bring to Front and Send to Back

Each moves the selected frame within its own kind. Frames that are Where placed are painted above frames tied to text, so Bring to Front on a text-tied frame lifts it over its fellows but never above a Where placed one. An opaque frame hides what is behind it. Each is one undo step.

## Group and Ungroup

Select several frames with Shift+click (they must be Where placed and on the same page), then choose **Group Frames**. A group moves, is selected, and is deleted as one, and it shows one outline with handles around all its members. A group cannot be resized. **Ungroup Frames** dissolves the group.

## Keyboard

Tab and Shift+Tab (or Edit › Select Next Frame) step through the page's frames in stacking order.

Drawings have their own grouping inside Draw mode: see [Group and stacking order](draw-group-order).
