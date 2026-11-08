---
id: charts
title: Charts
area: objects
keywords: chart, graph, bar, pie, line, data, series, charting, amichart, plot
see_also: chart-types, chart-data, chart-options, chart-from-table, chart-output, chart-storage, tables
context: command:Charting, panel:Chart
---
A chart turns a small grid of numbers into a picture on the page. **Tools › Charting…** makes the chart and opens the **Chart** panel in the rail; the page redraws as you change anything.

## Starting a chart

What Charting… does depends on where you are:

- **A chart frame is selected:** the panel opens on that chart. Double-clicking a chart does the same, unless it was last edited in Draw mode (see [How charts are stored](chart-storage)).
- **An empty text frame, an empty table cell, or the main text:** a new chart is made. The clipboard is read once; if it holds tab-separated text with numbers, that is the data. Otherwise the chart starts with sample data (three categories, two series) and the panel says so.
- **A frame holding a drawing or picture:** the panel asks whether to replace it with a chart.
- **Anywhere else** (a frame holding text, a header): the status bar says what Charting needs.

A new chart gets a frame 3.2 by 2.7 inches, flowing with the text at the caret. Making it is one undo step; Undo closes the panel and removes the frame.

## The panel, top to bottom

[Type](chart-types), [Options](chart-options) (legend, grid, 3D), the colour set, Axis, and [Data](chart-data), then the buttons that take the chart out ([Chart output](chart-output)). Esc closes the panel and throws away text typed but not applied.

Charts are for numbers you type or paste. To chart a table, see [Chart from Table](chart-from-table). Charts in exported documents appear as pictures.
