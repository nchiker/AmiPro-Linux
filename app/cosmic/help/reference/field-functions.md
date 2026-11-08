---
id: field-functions
title: Field functions
area: reference
keywords: Seq, Now, IF, Mid$, StrCat$, FormatNum$, NumPages, NumWords, Set, Include, Query$, Decide, Defined, MergeField, power field list, language, macro
see_also: power-fields, fields, cross-references
---
The Power field page of the Fields panel lists the functions. Choosing one puts its name in the Field box with a syntax line; the panel shows the result.

## Numbers and dates

- **Seq name** numbers things in order (figures, tables); it can start at a value, step up or down, and use number styles such as 1, I, i, A, a.
- **Now**, **CreateDate**, **EditDate** give the current, creation, and last-saved date or time, in a chosen format (`%D…` for dates, `%T1` to `%T9` for times).
- **FormatNum$** formats a number with decimals and surrounding text.

## Document facts

NumPages, NumWords, NumChars, NumEdits, FileSize, TotalEditingTime, Description, and the user fields.

## Text

Left$, Right$, Mid$, Len, UCase$, LCase$, StrCat$, and StrField$ (the n-th part of a text).

## Logic and variables

- **IF** condition result [ELSEIF …] [ELSE …] ENDIF chooses between results.
- **Set name value** stores a variable; **Defined name** is 1 if a variable or bookmark exists; **Void** works something out and shows nothing.
- **Query$** asks for text and **Decide** asks yes or no when the field is updated.
- **Include "file"** brings in another document's text when updated.

## Marks and links

Index and TOC are marks with no visible result ([Marking entries](marking-entries)); PageRef and Bookmark show a bookmark's page or text ([Cross-references](cross-references)); MergeField and MergeRec belong to [Merge](merge).

## Not run

The list also shows fields that the program does not run, marked in the panel: running other programs (Exec), calling macros (Call), showing a message (Message), and sending codes to the printer (PrintEscape). They are kept in a file but give no result.
