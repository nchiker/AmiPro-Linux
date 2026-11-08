---
id: equation-source
title: Writing equation source
area: objects
keywords: tex source, dollar sign, math mode, text mode, braces, superscript, subscript, prime, spaces, groups
see_also: equations, equation-tex-reference, equation-errors
---
The card holds plain text. A few rules cover most of it.

- **Math is the default.** Whatever you type in the card is one run of math, so no `$` is needed. If a `$` does appear, `$` signs switch between math and plain text.
- **Braces group.** `{a+b}` is one unit. Most commands take one braced argument or two: `\frac{1}{2}`.
- **Scripts.** `x^2` raises, `x_i` lowers, and both together as `x_i^2`. An apostrophe, as in `f'`, is a prime. For more than one character use braces: `x^{n+1}`.
- **Spaces** in the source do not appear in the picture. Use the spacing commands such as `\,` and `\quad` ([TeX reference](equation-tex-reference)).
- **Words** such as `sin` should be written as `\sin`, which sets them upright with the right spacing. `\text{some words}` writes plain text in the equation.
- **Symbols** are control words: `\alpha`, `\leq`, `\rightarrow`.

Example, typed in the card:

```
x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}
```

The palette builds structures like this for you: each button puts its template at the caret with `⬚` where you type, and the first `⬚` is selected.
