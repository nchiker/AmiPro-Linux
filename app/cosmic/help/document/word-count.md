---
id: word-count
title: Word count and statistics
area: document
keywords: words, characters, pages, statistics, count, size, selection
see_also: fields, spell-check
context: command:WordCount
---
Tools › Word Count (Ctrl+Shift+C) opens Document Properties in the rail at its Statistics section, showing pages, words, characters, and the size of the document. The figures follow your typing.

## Also in the status bar

The status bar shows the word count all the time. When part of the text is selected it shows the selected words against the total, for example "12 of 1,204 words".

## What is counted

Every text stream counts: the main text, headers and footers, frames, and table cells. Notes do not. A word ends at a space, a tab, a no-break space, or the end of a paragraph; characters include spaces and tabs but not paragraph ends. Fields and other inline objects are not counted as characters. A field that shows a count, such as NumWords, can put the number in the page itself; see [Field functions](field-functions).
