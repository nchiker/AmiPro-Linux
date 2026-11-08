---
id: picture-formats
title: Picture formats
area: objects
keywords: bmp, wmf, emf, png, jpeg, jpg, gif, tiff, tif, pcx, svg, metafile, sdw, import filters, eps, cgm
see_also: pictures, inserting-pictures, known-limitations, compatibility
---
Which picture files this program can read, and what it writes back.

## Read

| Kind | Files |
|---|---|
| Bitmaps | BMP, PNG, JPEG, GIF, TIFF, PCX |
| Metafiles | WMF, EMF |
| Vector | SVG |
| Ami Pro | drawings (`.sdw`), charts, equations, and stored screen images |

Insert › Image… offers all of the first three rows. TIFF and PCX are converted when imported.

## Not read

CGM, HPGL, DrawPerfect, Freelance, PIC, AutoCAD, EPS and WPG pictures have no reader yet. See [Known limitations](known-limitations).

## Saving

In the program's own file format a picture is kept exactly as it was inserted. Saving as an Ami Pro `.SAM` file keeps bitmaps and metafiles as they are and turns the other formats into Windows bitmaps (reduced if they would print above 300 dpi at their size on the page). The Save As report lists each conversion.

Pictures in exported documents (DOCX, ODT, RTF, PDF) are carried as pictures; charts and equations are exported as pictures too. See [Import and export](import-export).
