---
id: table-insert
title: Inserting a table
area: objects
keywords: insert table, new table, columns, rows, text to table, table to text, convert, grid size, Ctrl+F12
see_also: tables, table-rows-columns, table-messages
context: command:TextToTable, command:TableToText
---
A table can be inserted from the menu, or made from existing text, and turned back into text.

## Making a table

Choose Insert › Table… (Ctrl+F12). Sweep the grid with the pointer or type the number of **columns** (1 to 250) and **rows** (1 to 4000), then press Insert. The dialog's *Layout…* button sets the new table's sizes and options before it is made.

The table goes at the caret, with its columns filling the text column, and the caret lands in its first cell. It is one undo step. If an empty text frame is selected, the table takes the frame's place instead.

## From text and back

- **Convert Text to Table** (Table menu) turns the selected paragraphs of the main text into rows, one column for each tab in them.
- **Convert Table to Text** turns the table's cells into paragraphs separated by tabs.

Tables that have been made from a data file, or imported from a spreadsheet, work like any other table.

If a number is rejected, see [Table messages and what to do](table-messages).
