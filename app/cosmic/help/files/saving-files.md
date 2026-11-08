---
id: saving-files
title: Saving and Ami Pro files
area: files
keywords: save, save as, SAM, backup, bak, loss check, password, revert, native, amipro
see_also: import-export, crash-recovery, compatibility
context: command:Save, command:SaveAs
---
**Save** (Ctrl+S) writes the document in the format it came from. **Save As…** (Ctrl+Shift+S) lets you choose: an Ami Pro document (`.SAM`), or the program's own lossless `.amipro` format.

## Writing Ami Pro documents

Ami Pro's format cannot hold everything this program can. Before saving as SAM the program runs a **loss check** and lists anything that would change, such as characters outside Windows-1252, style names longer than 13 characters, or a picture format Ami Pro cannot store. You can cancel and save as `.amipro` instead, which loses nothing.

When Save writes over an existing `.SAM`, the first time it keeps the old file as `NAME.SAM.bak`. A file you imported, or a `.STY` you opened, is never written over: Save asks for a new name.

## Password

[**File › Password…**](password-protection) protects the document the way Ami Pro does; the file then asks for the password to open. A protected document is saved protected, and is not copied by crash recovery.

## Revert

[**Revert to Saved…**](revert-to-saved) drops your changes and reloads the file, after asking.

[Compatibility and fidelity](compatibility) lists what each format keeps.
