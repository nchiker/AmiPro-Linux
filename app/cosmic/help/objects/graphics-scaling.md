---
id: graphics-scaling
title: Graphics scaling
area: objects
keywords: scale, size, fit in frame, original size, percentage, custom, aspect ratio, rotate picture, stretch
context: command:GraphicsScaling
see_also: pictures, picture-position-crop, picture-frame-actions
---
Scaling decides how large a picture is drawn inside its frame. Choose **Format › Frame › Graphics Scaling…**, or open Frame Properties and look at the **Picture** section; both show the same controls beside the page.

## Sizing

The Sizing list has four choices; a note under it says what the current one means.

- **Original size**: the size the picture was made at.
- **Fit in frame**: as large as the frame allows. With *Keep aspect ratio* on, the picture keeps its shape; with it off, it is stretched to fill the frame.
- **Percentage**: a percentage of the original, so 50 is half size.
- **Custom**: the width and height typed below.

Typing a percentage chooses Percentage; typing a width or height chooses Custom. With *Keep aspect ratio* ticked, changing one of width and height changes the other. The percentage box always shows how large the picture currently is.

## Rotation

**Rotate (degrees)** turns bitmaps and drawings. It is not shown for metafiles, equations, EPS and OLE pictures.

## Values are typed where shown

Press Enter or leave the box to apply it. Each change is one undo step. A value out of range is refused with a short reason under the box.

Sizing changes the picture, not the frame. To make the frame match, use **Fit Frame to Picture** ([Picture frame actions](picture-frame-actions)).
