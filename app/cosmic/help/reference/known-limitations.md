---
id: known-limitations
title: Known limitations
area: reference
keywords: limitations, not built, missing, unsupported, macros, revision marking, OLE, differences
see_also: compatibility, import-export, charts, equations
context: 
---
This is what the program does not do, does differently, or has not yet been checked on a real desktop.

## Not built, and not planned

- Revision marking and Doc Compare. A file that carries revision data keeps it untouched, but it is neither shown nor edited.
- Macros. A document that names a macro opens without running it.
- Live editing of linked and embedded (OLE) objects. They are drawn and kept, and Convert to Picture works.
- Printer Setup: the desktop's print dialog chooses the printer.
- Window › Tile and Cascade: tabs replace them.
- Ami Pro's other import and export filters (WordPerfect, Word 1 and 2, WordStar).

## Not built yet

- A second view of one document, and the vertical ruler.
- Frame markers in Draft mode.
- Some picture formats: CGM, HPGL, DrawPerfect, Freelance, PIC, AutoCAD, EPS, and WPG. BMP, WMF, EMF, PNG, JPEG, GIF, SVG, TIFF, PCX, and Ami Pro drawings are read.

## Differences you may notice

- A word wider than the gap beside a frame goes below the frame; Ami Pro breaks it.
- Line breaks can differ by a word, because text is shaped with your system's fonts rather than Ami Pro's.
- The page surface gives screen readers no access to the page text yet, and input methods have not been checked on a real desktop.

The full list is in `docs/KNOWN_LIMITATIONS.md`. [Compatibility and fidelity](compatibility) says what each file format keeps.
