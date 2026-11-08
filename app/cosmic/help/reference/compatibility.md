---
id: compatibility
title: Compatibility and fidelity
area: reference
keywords: compatibility, fidelity, formats, loss check, sam, sty, docx, odt, rtf, word pro
see_also: saving-files, import-export, known-limitations
context: command:Compatibility
---
**Help › Compatibility and Fidelity** shows what saving the current document as an Ami Pro file would change, without saving. Press **Check again** after editing.

## What is opened

- Ami Pro documents (`.SAM`) and style sheets (`.STY`), including password-protected documents.
- The program's own `.amipro` format, which keeps everything.
- DOCX, ODT, RTF, Markdown, and plain text.
- Pictures: BMP, WMF, EMF, PNG, JPEG, GIF, SVG, TIFF, PCX, and Ami Pro drawings.
- Word Pro (`.LWP`) files are not read.

## What is written

Ami Pro documents are written the way Ami Pro writes them, and documents saved by the program open in Ami Pro 3.1. Anything its format cannot hold is listed first. Other formats list what they lose in the same way; frames and page layouts are reduced in other word processors.

The project's matrix of formats and features, with the evidence for each, is `docs/COMPATIBILITY.md`. [Known limitations](known-limitations) lists what is missing.
