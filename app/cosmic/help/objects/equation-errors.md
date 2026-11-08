---
id: equation-errors
title: Equation messages
area: objects
keywords: error parsing tex, message 5106, 5100, 5102, 5105, too complex, too big, matrix 10 x 10, unknown command
see_also: equations, equation-tex-reference, equation-source
---
When the card's text cannot be drawn, a line under the card says why. The frame keeps the last picture that worked, so nothing is lost; fix the text and the picture updates.

The message has the form: the problem, the message number in brackets, then the line and column where it was found.

| Number | Meaning |
|---|---|
| 5106 | The text is not valid TeX. The first words say what was wrong: an argument is missing, a `}` is missing or unmatched, a `$` is missing, `{` expected, `\of` expected after `\root`, `\right` expected after `\left`, a misplaced delimiter or command, or an unknown command followed by its name |
| 5100 | The equation is nested too deeply or has too many parts |
| 5102 | A matrix is larger than 10 by 10 |
| 5105 | The equation is too big to draw |

An **unknown command** usually means a symbol name the program does not have, or a typing slip. Check [TeX commands accepted](equation-tex-reference).

If an equation from an Ami Pro file is not made of the two text lines Ami Pro writes, the panel says it cannot read it; it is kept as it is and drawn from its stored picture.
