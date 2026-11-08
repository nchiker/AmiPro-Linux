---
id: cross-references
title: Cross-references
area: document
keywords: cross reference, page reference, PageRef, Bookmark field, refer to page, link
see_also: bookmarks, fields, field-functions
context: command:CrossReference
---
A cross-reference is a field that shows the page number or the text of a bookmark. It is how a sentence can say "see page 12" and stay right when pages move.

## Inserting one

1. Make the bookmark first ([Bookmarks](bookmarks)).
2. Put the caret where the reference goes.
3. Choose Insert › Cross-Reference… and pick the bookmark and whether to show its page or its text.

The result is an ordinary field, using the PageRef or Bookmark function. It is worked out with the other fields, and **Update Fields** (F9) refreshes it; see [Updating fields](field-updating).

## Elsewhere

In a PDF each bookmark is a named destination and each cross-reference becomes a link to it. DOCX and ODT exports carry the bookmarks and the references as their own fields.
