---
id: undo-redo
title: Undo and Redo
area: document
keywords: undo, redo, ctrl+z, history, step, levels
see_also: revert-to-saved, preferences
context: command:Undo, command:Redo
---
**Edit › Undo** (Ctrl+Z) takes back your last change. **Edit › Redo** (Ctrl+Shift+Z) puts back what you undid.

## What one step is

A run of typing is one step, so one Undo removes the whole word or sentence you just typed. Other commands, such as applying a style, replacing all matches, or inserting a table, are each one step. Switching tabs does not mix histories: each document has its own.

Making a new change after an Undo discards the steps that could have been redone.

## How far back

Preferences › General sets the **Undo levels**, from 1 to 1000. Saving does not clear the history, so you can undo past a save.
