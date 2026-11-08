---
id: table-rows-columns
title: Rows and columns
area: objects
keywords: insert row, insert column, delete row, delete column, delete table, distribute, equal width, resize, gutter
see_also: tables, table-select-cells, table-messages
context: command:InsertRowAbove, command:InsertRowBelow, command:InsertColumnLeft, command:InsertColumnRight, command:DeleteRow, command:DeleteColumn, command:DeleteTable, command:DistributeRows, command:DistributeColumns
---
Rows and columns can be added, removed and evened out at any time, and the table adjusts.

## Adding

The Table menu has Insert Row Above, Insert Row Below, Insert Column Left, and Insert Column Right. They add as many rows or columns as you have selected, or one when only the caret is in the table, at the table's default size, and copy the lines and shading of the neighbour. The table panel's *Insert* control can add 1 to 250 at a time.

A new column may not fit in the width of the table. The table keeps its width and the columns narrow in proportion, and the status bar says so.

Tab in the last cell also adds a row; see [Keys in cells](table-keys).

## Deleting

**Delete Row** and **Delete Column** remove the caret's or the selected ones. **Delete Table** removes the whole table, and so does deleting every row or column. There is no question first: Undo brings them back.

## Sizes

- Drag a grid line, or edit the width, height, and gutters in the panel's *Rows and columns* section. *Reset* removes an own size and returns to the table's default.
- **Distribute Rows Evenly** and **Distribute Columns Evenly** make the selected ones, or all of them, equal within the space they take now. Distribute Rows is unavailable when rows are automatic height.
- Automatic rows grow to fit their text.
