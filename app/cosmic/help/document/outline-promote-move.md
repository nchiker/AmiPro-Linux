---
id: outline-promote-move
title: Promote, demote, and move headings
area: document
keywords: promote, demote, move up, move down, alt+left, alt+right, alt+up, alt+down, change level, reorder section, nested list
see_also: outline-mode, outline-styles, styles, table-keys
context: command:Promote, command:Demote, command:MoveUp, command:MoveDown
---
Promote, Demote, Move Up and Move Down change an outline heading's level or position together with the text under it.

## Promote and Demote

**Promote** (Alt+Left) and **Demote** (Alt+Right) change the level of the paragraph the caret is in, or of the selected paragraphs. They give the paragraph the style that stands one level up or down. If the document has no style at that level, the program makes a copy of the paragraph's style at that level. The status bar names the style given, or says the paragraph is already at the top or lowest level. One undo step.

They are in Format › Bullets and Numbering, on the toolbar, and in the text menu, and they work in the normal view as well as in Outline mode. Tab stays a tab.

Demoting an item of a numbered list turns the list into a multi-level one: the list is numbered 1, A, i, and so on by depth, and the styles List 1, List 2, … are made as needed. Bulleted lines nest to deeper bullet styles the same way.

With a frame selected, Alt+arrows resize the frame instead.

## Move Up and Move Down

**Move Up** (Alt+Up) and **Move Down** (Alt+Down) move the caret's heading together with its section, which runs to the next heading of the same level or higher, past its neighbour. A paragraph of body text moves alone. Frames anchored in the section go with it. The caret stays in the moved text. One undo step.

In a table cell these keys move table rows instead ([Keys inside table cells](table-keys)).
