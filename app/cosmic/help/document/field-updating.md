---
id: field-updating
title: Updating and locking fields
area: document
keywords: update, F9, ctrl+F9, update all, next field, previous field, lock field, refresh, prompt
see_also: fields, power-fields, fields-messages
context: command:UpdateFields, command:UpdateAllFields, command:NextField, command:PreviousField, command:LockField
---
A field shows the result it had when it was last worked out. Updating works it out again.

- **Update Fields** (F9) updates the fields in the selection, or the one at the caret.
- **Update All Fields** (Ctrl+F9) updates every field in the document, including headers, footers, and frames.
- **Lock Field** keeps a power field's result from changing when updates run. The menu item is checked while the field is locked, and a locked field says so in the status bar. Only power fields can be locked.
- **Next Field** and **Previous Field** select the whole next or previous field, moving into other text streams when needed.

Date, time, and document-information fields follow changes on their own. Fields that ask a question (a Query$ or Decide function) show a prompt when they are updated; if you cancel, the status bar says "The fields were not updated". A field set to *Update when opened* is brought up to date as the document opens.

All of these are in the Edit menu, under Power Fields in Ami Pro's arrangement. The status bar reports the outcome: "Updated 3 fields", "The fields are up to date", "There is no field here to update", or "The document has no fields".
