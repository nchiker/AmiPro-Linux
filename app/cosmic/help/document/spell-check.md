---
id: spell-check
title: Spell check
area: document
keywords: spelling, misspelled, dictionary, replace all, skip all, add to dictionary, repeated word, F7, hunspell, user dictionary
see_also: spell-as-you-type, document-language, proofing-messages, thesaurus
context: command:Spelling, panel:Proofing
---
Spell Check finds words that are not in the dictionary and lets you fix them one at a time. It runs in the Proofing panel in the rail, so the document stays open and editable beside it.

## Running a check

Choose Tools › Spell Check… (F7). The check starts at the word the caret is in and goes forward through the main text and the other text streams, such as headers and footers. If text is selected, only the selection is checked. Each unknown word is selected on the page and shown in the panel with a list of suggestions.

For each word you can:

- **Replace** it with the suggestion you picked, or with text typed in the Replace with box.
- **Replace All** to change every occurrence in the checked range at once. Each one keeps its own capitals, so a wrong "teh" becomes "the" and "Teh" becomes "The". This is one undo step.
- **Skip** or **Skip All** to leave it alone. Skip All lasts for the session in this document and also hides the underline while you type.
- **Add to Dictionary** to keep the word in your own word list.

A word written twice in a row is shown as a repeated word; the button deletes the second copy.

## Options and dictionaries

The panel's Options section turns on or off the checks for repeated words, words with numbers, and words that start with a capital. The language is chosen in the panel too; see [Document language](document-language). Your own word list is shown in the User dictionary section, where you can add and remove words, import a list, or export yours.

When the check reaches the end it says "Spell check complete." in the status bar.

If something goes wrong, see [Proofing messages and what to do](proofing-messages).
