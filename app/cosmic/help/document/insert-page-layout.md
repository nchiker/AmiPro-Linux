---
id: insert-page-layout
title: Inserting and removing page layouts
area: document
keywords: new layout, change margins mid-document, landscape page, section, revert to standard page layout, remove page layout, columns part-way
see_also: page-layout-panel, headers-footers, page-column-breaks
context: command:InsertPageLayout, command:RemovePageLayout, command:RevertPageLayout
---
Every page has one layout, but a document can switch layouts part-way: a landscape page for a wide table, or two columns after a one-column opening. The Format › Page Layout Changes submenu has the three commands.

## Insert Page Layout

Insert Page Layout starts a layout **from the next page**. It puts a page break at the caret that names the new layout. At the very end of the text, an empty paragraph is added after the break so the caret has somewhere to go. The new layout begins as a copy of the one on the caret's page, named 1, 2, and so on. Page Layout Properties is already open and follows the caret onto it, so you can change the copy straight away.

The panel's own **Insert Page Layout** button has a list, **From the next page**: choose a copy of the layout shown, or any layout the document already has.

## Revert to Standard Page Layout

This inserts a break that returns to the document's standard layout. It is the same as inserting a layout and choosing the existing one in that list.

## Remove Page Layout

Remove takes out the page break before the caret that switches to the layout shown. If no other break uses that layout, the layout is removed too, all in one undo step. The document's own standard layout cannot be removed.

You can also delete the break mark by hand, as with any other object. See [Page and column breaks](page-column-breaks).
