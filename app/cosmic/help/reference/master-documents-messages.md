---
id: master-documents-messages
title: Master document messages
area: reference
keywords: error, master document, file missing, locate, no toc entries, treated as a single document
see_also: master-documents, toc-index-messages
---
These are the messages shown while you build and use a master document, and what each one asks of you.

- **No files yet: Add… the documents of the book in their order.** A master with no members does nothing. Use Add….
- **Only Ami Pro and AmiPro-Linux documents other than this one can be files of a master document.** A member must be one of those formats and cannot be the master itself. Convert other files first ([Importing and exporting](import-export)).
- **That file is missing: Locate… it first.** The member was moved or deleted. Use Locate… to point at it, or remove it from the list.
- **Treated as a single document: its numbers start from 1.** Shown after you turn a member back into a stand-alone document.
- **No TOC entries in the files / No index entries in the files.** None of the members has any. Give a style a level in TOC, Index… (the master's levels apply to every file), or mark index entries.
- **A member cannot be opened.** The run stops and names the file. Fix or remove it and run again.
