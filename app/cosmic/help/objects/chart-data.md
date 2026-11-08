---
id: chart-data
title: Chart data
area: objects
keywords: data grid, series, categories, labels, flip data, numbers, clipboard, tab separated, underscore, apply data, 5000
see_also: charts, chart-from-table, chart-output
---
The **Data** section holds the numbers. It has two forms of the same data.

## The grid

Rows are series, columns are categories. The first row holds the category labels and the first column the series names. Click a cell and type; the value applies when you press Enter or leave the cell. The grid shows the first 30 by 10 cells; the text box below holds everything.

- **Add Series** and **Add Category** append; the **Remove** buttons delete the last one.
- **Flip Data** swaps series and categories (and their labels).

## The data text

The text box takes the data as text, with tabs or runs of spaces between values, then **Apply Data**.

- A word in the first row (before the numbers) is a category label; a word at the start of a row is a series name.
- A label that is itself a number, such as a year, needs an underscore in front: `_2020`.
- An underscore between words in a label shows as a space: `North_East`.
- Numbers may be negative, have a decimal point or an exponent. Currency signs and thousands commas are accepted and ignored.
- An empty cell among numbers counts as zero.

If there are no numbers, or more than 5,000 values, Apply Data is refused with the reason under the box.

## Pasted data

When the chart is made, the clipboard is read once. Tab-separated text from a spreadsheet or a table is taken as it is. You can also paste into the text box.

The grid edits this chart's own copy; the source of pasted data is never changed.
