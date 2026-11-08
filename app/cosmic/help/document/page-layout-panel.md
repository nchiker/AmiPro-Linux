---
id: page-layout-panel
title: Page Layout Properties
area: document
keywords: page layout, modify page layout, page setup, margins panel, layout list, right pages, left pages, mirror, double-click margin
see_also: page-size-margins-columns, page-lines, headers-footers, insert-page-layout, pages-frames-layouts
context: command:PageLayout, panel:PageLayout
---
A page layout holds a page's size, margins, columns, tabs, lines, header, and footer. Format › Page Layout… opens its settings in the rail as **Page Layout Properties**. A double-click in a page's left or right margin does the same, and so does right-clicking there, where Page Layout… is the first item.

## How the panel works

- It shows the layout of the page holding the caret and follows the caret to other pages while open. The **Layout** list picks another of the document's layouts.
- Changes apply at once, and each one is a single undo step, so the pages themselves are the preview. A value that does not fit, such as margins wider than the paper, is refused in place with the reason.
- With the caret in a header or footer, the panel opens at that band's section.
- Sections fold: **Page**, **Margins and columns**, **Tabs**, **Lines**, **Header**, **Footer**, and **Line numbering**. Details are in [Size, margins, and columns](page-size-margins-columns), [Page lines](page-lines), [Headers and footers](headers-footers), and [Line numbering](line-numbering).
- The panel also has Insert Page Layout and Remove buttons; see [Inserting page layouts](insert-page-layout).

## Right and left pages

**Pages** chooses whether a change goes to All pages or only the Right or Left ones. The first time you change only Right or Left pages, the layout gets pages of its own on that side, and the two can then differ, for instance in header text or margins. **Mirror** is available once they differ: it mirrors the margins and columns on left pages. While Mirror is on, All is refused with the reason. Choosing All again while the left pages differ is a change of its own: it takes the left headers, footers, and their text along, and Undo brings them back.

The commands for the page layout live in the Format menu; the dialog for File › Page Setup is described in [Page Setup](page-setup).
