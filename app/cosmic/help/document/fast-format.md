---
id: fast-format
title: Fast Format
area: document
keywords: format painter, paintbrush, copy formatting, ctrl+t, paint format, copy style
see_also: clear-direct-formatting, select-style, text-formatting
context: command:FastFormat
---
Fast Format copies the formatting of one place and paints it onto text you select afterwards. Use Format › Fast Format or Ctrl+T, or the paintbrush button in the action bar.

## Using it

1. Put the caret in text that looks the way you want, or select some.
2. Choose Fast Format. The command is checked and the status bar says the mode is on.
3. Select other text with the mouse. Each selection you finish takes the format at once.
4. Press Esc, or choose Fast Format again, to stop.

Every painted selection is its own undo step.

## What is picked up

With a part of a paragraph selected, the text's font and attributes are picked up. With only the caret, or a whole paragraph selected, a question asks whether to take the text's font and attributes or the paragraph's style; the style is the default. Protected Text and No Hyphenation are not copied.

Painting the style is the quick way to give several paragraphs the same style. To change a style for all its paragraphs, use [Modify Style](modify-style) instead.
