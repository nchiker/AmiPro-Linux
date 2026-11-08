---
id: protected-text-hyphenation
title: Protected text and No Hyphenation
area: document
keywords: protect text, preprinted form, hyphenation, hyphen, mark text, no hyphens, form fill
see_also: text-formatting, printing-pdf
context: command:ProtectedText, command:NoHyphenation
---
Edit › Mark Text has two toggles that behave like character formatting: Protected Text and No Hyphenation. Each is checked when the text at the caret has it.

## No Hyphenation

Selected text with No Hyphenation is never broken at a line end with a hyphen, even where its style has hyphenation on. Use it for names, codes, and words that must stay whole. Without a selection it applies to the text typed next.

## Protected Text

Protected text is kept out of easy reach: a click anywhere inside it puts the caret after it, so it is not edited by accident. It is meant for fixed text on a form. When you print or make a PDF with **On preprinted form** (in the Print panel), protected text is left off the paper, so only the fill-in text prints onto a form that already carries the fixed text. See [Printing and PDF](printing-pdf).

## Notes

Neither mark is copied by [Fast Format](fast-format). Both are saved in Ami Pro files. Other formats may not hold protected text; the loss check before saving lists it.
