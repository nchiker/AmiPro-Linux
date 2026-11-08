---
id: frame-placement-wrap
title: Frame placement and text wrap
area: document
keywords: where placed, with para above, flow with text, repeat all pages, repeat right left, wrap around, no wrap beside, opaque, transparent, anchor, header logo
see_also: frame-properties, frame-layering-groups, pages-frames-layouts
---
Frame Properties › Type decides how a frame sits on the page and how text behaves near it.

## Placement

- **Where placed:** fixed on the page at the position you give. Move it by dragging, or by the From left and From top boxes. It stays on its page.
- **With para above:** tied to a paragraph and shown below that paragraph's text, moving with it from page to page. By hand it moves only sideways.
- **Flow with text:** sits in the line of text at the frame's mark, on the baseline, and makes the line taller; by hand it moves only up and down.
- **Repeat all pages:** appears on every page of the page layout, a good place for a logo or a watermark. **Repeat right/left** limits it to one side of the spread. Changes to a repeating frame apply to every page that uses the layout.

Switching placement is one undo step. Frames that are Where placed are painted above frames tied to text.

## Wrap

- **Wrap around:** text runs around the frame on its free sides.
- **No wrap around:** text runs through the frame's area as if it were not there.
- **No wrap beside:** the text after the frame starts below it, leaving the sides empty.

A With para above frame cannot wrap around, because it sits below its paragraph; asking for Wrap around there gives No wrap beside.

## Opaque

**Opaque** hides what is behind the frame, other frames' text and the main text alike. Without it the frame is transparent and what lies under shows through.

See [Layering and grouping](frame-layering-groups) for what is on top when frames overlap.
