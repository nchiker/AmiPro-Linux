---
id: toc-index
title: Table of contents, index, and glossary
area: document
keywords: toc, table of contents, index, glossary, mark entry, alphabetical, include
see_also: outline-mode, master-documents, notes-footnotes
context: command:ContentsIndex, command:MarkIndexEntry, command:MarkTocEntry, command:MarkGlossary, command:InsertGlossary, panel:Contents, panel:Glossary
---
Tools › TOC, Index… builds a table of contents and an index from your document, and opens the Contents and Index panel in the rail.

## Table of contents

Tick **Table of contents**, then use TOC Options to say which paragraph styles go at which level, from 1 to 9. The first choice comes from the styles' outline levels. To list a heading that has no style for it, select the text and use **Edit › Mark Text › TOC Entry…**.

## Index

Mark each entry with **Edit › Mark Text › Index Entry…**: a primary entry, an optional secondary one, and whether it shows a page number. The index is built from the marks, with alphabetical separators if you ask for them.

The result is made into its own document, so you can review and print it. Generate again after the text has changed.

## Glossary

A glossary holds pieces of text you use again. **Edit › Mark Text › Glossary…** stores the selection under a name; **Insert › Glossary Record…** (Ctrl+K) puts it back in.

For a book made of several files, [Master documents](master-documents) build one table of contents and index across them.
