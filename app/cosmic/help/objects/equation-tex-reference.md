---
id: equation-tex-reference
title: TeX commands accepted
area: objects
keywords: tex commands, control words, frac, sqrt, root, matrix, left right, sum, int, overline, hat, vec, overbrace, binom, limits, bf, text, symbols, greek
see_also: equation-source, equations, equation-palette, equation-errors
---
The commands the equation editor understands. A command not listed here (and not a symbol name the program knows) is reported as an error.

## Structures

| Write | Gives |
|---|---|
| `\frac{a}{b}` | fraction; `\tfrac` text size, `\dfrac` display size |
| `a \over b`, `a \atop b`, `n \choose k` | the same with the operator between the parts |
| `\binom{n}{k}` | binomial; `\tbinom`, `\dbinom` for sizes |
| `\sqrt{x}` | square root |
| `\root{n} \of{x}` | root with an index |
| `x^{a}`, `x_{a}`, `x'` | superscript, subscript, prime |
| `\left( ... \right)` | brackets that grow; the delimiter may be `( ) [ ] \{ \} \langle \rangle \lfloor \rfloor \lceil \rceil \vert \Vert` and others |
| `\overset{a}\to{b}`, `\underset{a}\to{b}` | text a above or below b |

## Big operators

`\sum \prod \coprod \int \iint \iiint \iiiint \oint \bigcap \bigcup \bigvee \bigwedge \bigoplus \bigotimes \bigodot \biguplus \bigsqcup`. Give limits with `_` and `^`: `\sum_{i=1}^{n}`. Add `\limits` or `\nolimits` straight after the operator to put the limits above and below or beside it. A `t` or `d` before the name, as in `\dsum`, picks text or display style.

## Functions

Written as `\name`: `\arccos \arcsin \arctan \arg \cos \cosh \cot \coth \csc \deg \det \dim \exp \gcd \hom \inf \ker \lg \lim \liminf \limsup \ln \log \max \min \sec \sin \sinh \sup \tan \tanh \Pr`. `\lim`, `\max`, `\min`, `\sup`, `\inf`, `\det`, `\gcd` and `\Pr` put subscripts underneath. `\func{name}` sets any other word as a function name.

## Matrices and stacks

```
\matrix a & b \\ c & d \endmatrix
```

Cells are separated by `&` and rows by `\\`. At most 10 by 10. `\format \l & \c & \r \\` before the cells sets left, centre or right alignment for each column. `\Sb ... \endSb` stacks lines below an operator and `\Sp ... \endSp` above it, with rows separated by `\\`.

## Marks over and under

- Accents on one character: `\hat \check \tilde \acute \grave \dot \ddot \breve \bar \vec`.
- Over or under a whole group: `\overline \underline \overbrace \underbrace \widehat \widetilde \overleftarrow \overrightarrow \underleftarrow \underrightarrow \overleftrightarrow \underleftrightarrow`.

## Fonts and text

Switch inside braces, as `{\BF x}`: `\BF` bold, `\IT` italic, `\RM` roman, `\SL` slanted, `\SMC` small caps, `\TT` typewriter, `\CAL` calligraphic, `\FRAK` fraktur, `\BBB` blackboard bold, `\NEG` negative. `\text{...}` writes plain text.

## Spacing

`\,` thin, `\!` negative thin, `\:` medium, `\;` thick, `\ ` space, `\thinspace \negthinspace \medspace \thickspace \enspace \quad \qquad \space`.

## Symbols

Greek letters (`\alpha`, `\Omega`), operators (`\pm \times \cdot \cup \cap`), relations (`\leq \neq \approx \in \subset`), arrows (`\rightarrow \Rightarrow \mapsto`), and ordinary symbols (`\infty \partial \nabla \forall \exists \ldots \cdots`) are control words. The program knows a few hundred of Ami Pro's 887 symbol names; the palette's Greek, Operators, Relations and Arrows groups insert ones that work.

## Read and ignored

`\label`, `\ref`, `\pageref`, `\cite` and `\glossary` and their argument are accepted and dropped.
