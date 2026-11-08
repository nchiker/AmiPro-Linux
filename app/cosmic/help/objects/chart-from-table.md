---
id: chart-from-table
title: Chart from Table
area: objects
keywords: table to chart, selected cells, table data, chart table, link
see_also: charts, tables, chart-data
context: command:ChartFromTable
---
**Tools › Chart from Table** makes a chart from the table the caret is in. Select a block of cells first to chart only those; with just the caret in a cell, the whole table is used. Charting… itself does the same when two or more cells are selected, and the Chart panel has a **Chart from Table** button while the caret is in a table.

## How cells become data

- A first row of words gives the category labels; a first cell of words in a row gives that series its name.
- Numbers in the first row are data, not labels. Write a year as a label with a leading underscore, as in `_2020` ([Chart data](chart-data)).
- An empty cell among numbers is zero.

## Where the chart goes

The chart is a new frame placed after the table, anchored with the paragraph above it and not wrapping text around itself. The data is a copy: changing the table later does not change the chart. There is no live link; re-run Chart from Table to refresh, or edit the chart's own grid. See [Tables](tables) and [Known limitations](known-limitations).

One undo step makes the whole chart.
