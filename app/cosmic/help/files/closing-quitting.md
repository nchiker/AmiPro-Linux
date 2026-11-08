---
id: closing-quitting
title: Closing documents and quitting
area: files
keywords: close, exit, quit, ctrl+w, ctrl+q, unsaved changes, discard, tab
see_also: saving-files, crash-recovery
context: command:CloseDocument, command:Quit
---
**File › Close Document** (Ctrl+W) closes the document in the current tab. **File › Quit** (Ctrl+Q) closes the program.

## Unsaved changes

A document with changes you have not saved asks first. **Save** writes the file and then closes; **Discard** closes without saving; **Cancel** leaves everything as it was. If saving needs a new name, or the loss check has something to report, the close waits until the file is really written. Quit asks once for each changed document.

A document you have not touched closes at once.

## Recovery copies

Discarding changes also throws away the document's recovery copy. If the program ends some other way (a crash or power cut), the copies are offered at the next start; see [Crash recovery](crash-recovery).
