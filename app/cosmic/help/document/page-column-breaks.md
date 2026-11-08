---
id: page-column-breaks
title: Page and column breaks
area: document
keywords: page break, column break, new page, ctrl+enter, ctrl+shift+enter, force break, break before
see_also: modify-style, insert-page-layout, page-size-margins-columns
context: command:PageBreak, command:ColumnBreak
---
A break ends a page or a column early. Insert › Page Break (Ctrl+Enter) starts a new page at the caret; Insert › Column Break (Ctrl+Shift+Enter) starts the next column.

## Behaviour

- A page break goes in at the caret without splitting the paragraph. The text after it begins the next page.
- A column break goes at the start of the caret's paragraph, once, so repeating the command does not stack breaks.
- A break is a mark in the text. Delete it as you would any character.

## Breaks as part of a style

If every paragraph of a kind should start on a new page, such as chapter titles, do not insert a break each time: set **Page break before paragraph** in the style's Breaks section ([Modify Style](modify-style)). The same section has Allow break within and Keep with previous or next paragraph, to stop a paragraph being split or left alone at the foot of a page.

A page break can also name a different layout: see [Inserting page layouts](insert-page-layout).
