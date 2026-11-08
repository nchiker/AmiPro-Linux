---
id: equation-files
title: Equation files and LaTeX
area: objects
keywords: import equation, save as equation, tex file, paste latex, copy as latex, clipboard, dollar, environment
see_also: equations, equation-tex-reference, equation-errors
context: command:PasteLatexEquation
---
Equations can be read from TeX files, saved to them, and pasted from LaTeX text.

## Import Equation… and Save As Equation…

In the Equation panel's **Files and LaTeX** section:

- **Import Equation…** reads a TeX file into the equation. Constructs such as fractions or scripts become math and plain letters around them stay text.
- **Save As Equation…** writes the equation as Ami Pro's TeX text, in lines of about 70 to 79 columns.

Both use the desktop's file dialogs.

## LaTeX on the clipboard

- **Copy as LaTeX** copies the source in standard LaTeX: `\root{n} \of{x}` becomes `\sqrt[n]{x}` and `\matrix` becomes a matrix environment.
- **Paste LaTeX** in the panel, or **Tools › Paste LaTeX Equation**, reads LaTeX from the clipboard and builds the equivalent. It accepts text wrapped in `$...$`, `$$...$$`, `\[...\]`, `\(...\)` or an equation environment.

Edit › Paste never turns text into an equation by itself.

LaTeX that uses commands outside the program's list ([TeX commands](equation-tex-reference)) is reported as an error in the card.
