---
id: pages-frames-layouts
title: Pages, frames, and page layouts
area: document
keywords: page, frame, page layout, margins, columns, header, footer, wrap, placement, group, ruler, page break
see_also: styles, tables, pictures, view-zoom
context: command:Frame, command:DrawFrame, command:FrameProperties, command:GraphicsScaling, command:BringToFront, command:SendToBack, command:GroupFrames, command:UngroupFrames, command:PageLayout, command:InsertPageLayout, command:RemovePageLayout, command:RevertPageLayout, command:InsertRuler, command:RemoveRuler, command:PageBreak, command:ColumnBreak, command:HeaderFooter, command:PageNumber, command:PageSetup, command:LineNumbering, command:GoToPage, command:GoToHeader, command:GoToFooter, command:GoToRuler, command:PreviousPage, command:NextPage, command:NextFrame, panel:Frame, panel:PageLayout
---
A document here is built the way Ami Pro builds it, which is not the way Word does. It is made of pages, page layouts, and frames.

## Pages and page layouts

Every page has exactly one **page layout**: its margins, columns, tab stops, header and footer, and page lines. The document's style sheet supplies the layouts, and Format › Page Layout Changes › Insert Page Layout starts a different one part-way through. Format › Page Layout… opens the layout's settings in the rail; right-clicking in a page's margin opens the same panel.

Format › Ruler › Insert Ruler changes tabs and indents from one point on, without starting a new layout. Insert › Page Break (Ctrl+Enter) and Column Break (Ctrl+Shift+Enter) end a page or a column early.

## Frames

A **frame** is a small document inside the document. It holds exactly one of: text, a table, or a picture (drawings, charts, and equations are pictures). Insert › Frame… makes one from a dialog; Insert › Draw Frame lets you draw it with the mouse.

Select a frame by clicking it. Frame Properties opens in the rail and sets:

- **Placement:** where placed, with the paragraph above, flowing with the text, or repeating on every page.
- **Wrap:** wrap around, no wrap around, or no wrap beside.
- Lines, shadow, colour, margins, and columns.

Frames can overlap. Format › Frame has Bring to Front and Send to Back, and Group and Ungroup join frames so they move together. Edit › Select Next Frame steps through them.

## Moving around

Edit › Go To… (Ctrl+G) and the Go To Header, Footer, and Ruler commands jump to a page or part of one. Header and footer text is edited in place on the page; Insert › Floating Header/Footer… places one at a chosen spot.
