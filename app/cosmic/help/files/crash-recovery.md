---
id: crash-recovery
title: Crash recovery
area: files
keywords: recover, autosave, crash, copies, lost work, recovery
see_also: saving-files, getting-started
context: command:RecoverDocuments, panel:Recovery
---
While you edit, the program keeps recovery copies of documents with unsaved changes, in a hidden folder of its own. It never writes over your file. If the program ends without saving (a crash, a power cut), the next start opens **Recover documents** in the rail.

## Recovering

Each copy shows its file name (or "Never saved"), where it came from, and when it was last copied. **Recover** opens it as a new tab named "name (recovered)", marked changed. Save goes to Save As, which proposes the original folder, so the original file is not overwritten by accident. **Discard** deletes a copy; **Discard all** deletes them all. Esc closes the panel and keeps the copies. You can open the panel yourself with File › Recover Documents….

## Settings

Preferences › General turns the copies on or off (on by default) and sets the interval: 1, 2 (the default), 5, or 10 minutes.

## Good to know

- Copies are removed when you save, close without saving, or quit normally.
- A document protected with a password is never copied, because the copy would not be protected.
- Two windows of the program do not offer each other's copies.
