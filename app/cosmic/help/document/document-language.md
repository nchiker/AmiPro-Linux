---
id: document-language
title: Document language
area: document
keywords: language, dictionary, locale, english, hyphenation, default language, british, american
see_also: spell-check, grammar-check, proofing-messages
context: command:Language
---
Each document has one language. It decides which dictionary the spell checker uses, which thesaurus is offered, and which hyphenation patterns the layout follows.

## Choosing it

Tools › Language… opens the Proofing panel at its language list. The list shows every language that has a spelling dictionary installed on the system, and always includes American English, which the program carries itself. Choosing one is a single undo step.

**Use for new documents** keeps the choice as the language of documents you create later.

## What depends on it

- **Spelling** uses the Hunspell dictionary for the language.
- **Thesaurus** needs a MyThes file for it.
- **Grammar Check** works for English only.
- **Hyphenation** follows the language wherever patterns exist; at present that means English. Other languages are not hyphenated.

The language is kept in Ami Pro files as a number. A language Ami Pro had no number for is saved in the program's own format; saving as an Ami Pro document writes the nearest one and lists the difference in the loss check.

If the list lacks a language you need, see [Proofing messages and what to do](proofing-messages).
