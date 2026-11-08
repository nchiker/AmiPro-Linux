---
id: table-formulas
title: Formulas and Quick Add
area: objects
keywords: formula, sum, quick add, total, calculate, recalculate, cell address, avg, min, max, count, circular, @sum
see_also: tables, table-number-formats, table-messages, sorting
context: command:EditFormula, command:QuickAddRow, command:QuickAddColumn, command:RecalculateTable
---
A cell can hold a formula that works out a number from other cells.

## Entering a formula

Choose Table › Edit Formula…, or double-click a formula cell. The Formula section of the Table panel opens with its box focused. Type the formula and press Enter or leave the box; Esc puts the old text back. Edit Formula needs the caret in one cell (not a selection of several).

A formula may start with `=` or `@`. It can use `+ - * / %`, parentheses, cell names such as `B2` or `$B$2`, and `@sum(B2..B5)` (a colon also works), in any letter case. The program also accepts `@avg`, `@min`, `@max`, and `@count`; Ami Pro cannot calculate those, so saving as an Ami Pro document lists them in the loss check and keeps the result as text.

Turn on View › Show › Table Row/Column Headings to see the letters and numbers of the cells.

## Quick Add

**Table › Quick Add › Row** writes a sum into the caret cell of the run of cells to its left; **Column** sums the run above. The run stops at the nearest cell that holds plain text (a label), or a joined cell, so a label such as "Total" is not added; with no such cell it reaches the table's edge. It needs a cell to the left or above.

## Keeping results current

When you leave a cell you edited, or run a table command, every formula that depends on it is worked out again inside the same undo step. **Recalculate Table** works out all of them. Opening a file keeps the stored results.

A formula cell is right-aligned like a number and drawn with a thin box while the caret is in it. You cannot type into it; change it with Edit Formula…, or take it out with *Remove formula* or Delete on the selected cell.

Messages: [Table messages and what to do](table-messages).
