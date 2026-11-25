---
id: fast-format
title: Fast Format and Format Painter
area: document
keywords: format painter, paintbrush, copy formatting, ctrl+t, paint format, copy style, copy format, paste format
see_also: clear-direct-formatting, select-style, text-formatting
context: command:FastFormat, command:FormatPainter, command:CopyFormat, command:PasteFormat
---
Fast Format copies the formatting of one place and paints it onto text you select afterwards. Both ways in are under Format › Fast Format ▸.

## The ways to paint

- **Fast Format** (Ctrl+T, Ami Pro's own; Ctrl+Shift+T also works): every selection you make with the mouse takes the format until you press Esc or choose it again.
- **Format Painter** (the paintbrush button, which Customize Toolbar offers, or Format › Fast Format ▸ Format Painter): the next selection you make with the mouse takes the format, and then the painter turns itself off. Press the button twice quickly, as a double-click, to keep it on.

The menu ticks the one that is armed.

While the painter is armed, the status bar names what it holds, for example "Format painter: Heading 1, Arial 12 bold - drag over text to apply, Esc to cancel", the pointer shows a plus, and a brush appears beside the message.

## Copy Format and Paste Format

Copy Format (Ctrl+Alt+C) keeps the format at the selection without arming the mouse. Paste Format (Ctrl+Alt+V) gives it to the current selection, or to the caret's paragraph for a style, as often as you like. Every time the painter picks up a format, it keeps it for Paste Format too. (Ctrl+Shift+C and Ctrl+Shift+V keep their other meanings, Word Count and Paste as Plain Text.)

## Using it

1. Put the caret in text that looks the way you want, or select some.
2. Choose the painter. The status bar says what it picked up.
3. Select other text with the mouse. A selection that takes the format is one undo step.
4. Esc ends it at any time.

## What is picked up

With a part of a paragraph selected, the text's font and attributes are picked up. With only the caret, or a whole paragraph selected, a question asks whether to take the text's font and attributes or the paragraph's style. The style is the first choice, and afterwards the question starts at your last answer, which the status bar repeats: "style only" or "text only". Protected Text and No Hyphenation are not copied.

A click that has nothing to apply, such as a text-only format on a caret, does not use up a one-time painter. Painting the style is the quick way to give several paragraphs the same style. To change a style for all its paragraphs, use [Modify Style](modify-style) instead.
