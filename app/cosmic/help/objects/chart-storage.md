---
id: chart-storage
title: How charts are stored
area: objects
keywords: chart storage, type 15, drawing objects, edit chart with draw, unreadable chart, sam chart, generated objects
see_also: charts, drawing, draw-in-frames
---
A chart is a drawing in a picture frame plus a record of its data, type and settings. Every control in the Chart panel regenerates the drawing from that record.

## Ami Pro files

Saving as `.SAM` writes the data record and the drawing objects, the way Ami Pro does, and Ami Pro can open the result. Charts read from Ami Pro files are written back unchanged unless you edit them.

## Editing with Draw

**Drawing…** in the panel closes it and opens [Draw mode](drawing) on the chart's frame, where each part of the chart is an object you can move, recolour or add text to. Afterwards:

- Double-clicking the chart opens Draw mode again, until Charting changes it.
- Using the Chart panel again regenerates the drawing, which replaces edits made to generated parts; the panel warns about this.

## Unreadable charts

If a chart's data record is not in a layout the program knows, the panel says so and offers only **Replace with a New Chart** and **Drawing…**. The record is kept as it is and saved back unchanged.
