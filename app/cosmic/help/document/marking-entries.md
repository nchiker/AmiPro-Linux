---
id: marking-entries
title: Marking index and contents entries
area: document
keywords: index entry, mark text, primary, secondary, toc entry, see also, page number, level
see_also: toc-index, toc-index-messages, fields
context: command:MarkIndexEntry, command:MarkTocEntry
---
Marks tell the program which words to list in the index, or which text to list in the table of contents when its paragraph style is not a contents level.

## Index entries

Select the word or phrase, then choose Edit › Mark Text › Index Entry…. Give a **primary** entry (required) and, if wanted, a **secondary** one under it. Choose whether the entry shows a page number, and add text to follow the pages, such as a cross-reference to another entry. An empty primary is refused ("Must specify a primary.").

## Contents entries

Select the text and choose Edit › Mark Text › TOC Entry…. Pick a level from 1 to 9. The selected text becomes the entry. A paragraph that holds a contents mark is listed by its marks only, not by its style.

## Where the marks live

A mark is a field with no visible result. Show Power Fields (Alt+F9) displays it as code between angle brackets; see [Power fields](power-fields). The index or table is not changed until you generate it again from Tools › TOC, Index….
