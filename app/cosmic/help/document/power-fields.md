---
id: power-fields
title: Power fields
area: document
keywords: power field, field code, show codes, alt+f9, field shading, edit field, lock, update when opened, saved fields, auto run
see_also: fields, field-functions, field-updating, fields-messages
context: command:ShowPowerFields, command:EditField, command:FieldShading
---
A power field holds a small code that the program works out, and shows the result in the text. Dates, page counts, numbered sequences, and conditions are all power fields.

## Inserting one

Insert › Field… opens the Fields panel at its *Power field* page. Type a code in the Field box, or pick a function from the list. The panel shows a syntax line and the result as it would appear. Options offer formats for dates, times, and number styles. Tick **Lock** to freeze the result, and **Update when opened** to work it out each time the document is opened. **Save…** keeps a field's code under a name so it appears in the Saved list.

## Editing

Double-click a field, or put the caret in it and choose Edit › Edit Field…. The *This field* section shows its kind and what can change; a code is applied when you press Enter. The status bar names the field at the caret.

## Seeing the codes

**View › Show Power Fields** (Alt+F9) shows every field's code between angle brackets in red instead of its result; choose it again to go back. The codes are hidden again before saving. **Field Shading** tints the shown text of fields light grey on the screen only; it never prints.

Functions: [Field functions](field-functions). If a field misbehaves, see [Field messages](fields-messages).
