---
id: opening-files
title: Opening files
area: files
keywords: open, ctrl+o, recent files, file chooser, SAM, STY, docx, password, drag
see_also: import-export, saving-files, preferences-folders, password-protection
context: command:Open
---
**File › Open…** (Ctrl+O) shows the desktop's file chooser. Pick an Ami Pro document (`.SAM`), a style sheet (`.STY`), or a file from another program, and it opens in a new tab.

## What opens how

- **Ami Pro files** open as they are. If the document is protected, you are asked for its password first; see [Password protection](password-protection).
- **DOCX, ODT, RTF, Markdown, and text files** are converted as they open. The result is a new unsaved document, and the Import Report panel lists anything that could not be carried over. See [Importing and exporting](import-export).
- **Style sheets** open for editing, and are never written over; Save asks for another name.

## Starting folder and recent files

The chooser starts in the document's own folder, or otherwise in the Documents folder from Preferences › Folders. The bottom of the File menu lists your most recent files, numbered; choose one to open it. Preferences › General sets how many are kept (0 to 9).

**File › Import Folder…** converts a whole folder at once.
