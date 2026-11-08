---
id: import-export
title: Importing and exporting
area: files
keywords: import, export, docx, odt, rtf, markdown, text, word, libreoffice, import report, folder
see_also: saving-files, compatibility, printing-pdf
context: command:ExportDocx, command:ExportOdt, command:ExportRtf, command:ExportText, command:ExportMarkdown, command:ExportFolder, command:ImportFolder, panel:ExportFolder, panel:ImportFolder, panel:ImportReport
---
## Opening other formats

**File › Open…** reads DOCX, ODT (and flat ODT), RTF, Markdown, and plain text as well as Ami Pro files. The file opens as a new unsaved document. When something could not be carried over, the **Import Report** panel lists it in the rail: tracked changes, highlighting, hidden text, nested tables, and the like. Insert › File… puts a file's text into the current document.

## Exporting

File › Export has **DOCX**, **ODT**, **RTF**, **Text**, **Markdown**, and **PDF**. Before writing, the program shows what the format cannot hold, from the same loss check that Save As uses. The document keeps its own name and file; an export is a copy.

- Frames and layouts are reduced in other word processors' formats, as in Ami Pro. Tables, lists, footnotes, notes (as comments), bookmarks, and pictures are carried.
- Plain text keeps only the text.
- Markdown keeps headings, lists, quotes, code, emphasis, tables, and pictures beside the file.

## Whole folders

**Import Folder…** and **Export Folder…** convert many files at once, working in the rail and reporting each file.

For the detail of each format, see [Compatibility and fidelity](compatibility).
