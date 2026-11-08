---
id: sorting
title: Sorting
area: document
keywords: sort, sort ascending, sort descending, order, alphabetical, numeric, field, delimiter, records, rows
see_also: tables, sorting-messages, table-formulas
context: command:Sort, command:SortAscending, command:SortDescending, panel:Sort
---
Sorting puts selected paragraphs, or the rows of a table, in order.

## The Sort panel

Tools › Sort… opens the panel in the rail. The first line says what will be sorted: the selected paragraphs, the whole text when nothing is selected, the selected table rows, or the whole table.

- **Field** gives the part of each record to sort by, counting from 1. For a table the fields are its columns. Level 1 needs a field; levels 2 and 3 break ties and are skipped when empty.
- Choose **Alphanumeric** or **Numeric** for each level, and which word of the field to use (first, second, or all).
- **Order** is Ascending or Descending.
- **Delimiter** separates the fields in paragraphs, a tab or a character you choose. In a table the delimiter is not used because whole rows move.
- **Paragraphs in record** groups several paragraphs as one record.

Press **Sort**. It is one undo step and the sorted paragraphs stay selected; in a table the caret goes to the first sorted row. The settings are remembered for the tab.

## Quick sorts in a table

Table › Sort Ascending and Descending sort the rows below the heading rows by the column the caret is in, in one click.

If the sort will not run or needs a decision, see [Sort messages](sorting-messages).
