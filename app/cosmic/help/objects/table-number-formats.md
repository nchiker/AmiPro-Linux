---
id: table-number-formats
title: Numbers and number formats in cells
area: objects
keywords: number format, currency, percent, fixed decimal, decimal places, thousands separator, negative, parentheses, numeric cell
see_also: table-formulas, styles
---
A cell whose text is a number is treated as numeric: it is right-aligned and shown in the format of its paragraph style once you leave the cell. While the caret is in it, only the plain number shows. A space, a letter, a comma, a % sign, or a currency symbol makes the cell text instead.

## Setting the format

The format belongs to the paragraph style, normally Table Text, so every cell using the style changes together. In the Styles panel open the style's **Table format** section:

- **Cell format:** General, Fixed Decimal, Currency, or Percent.
- **Decimal places** and the **decimal symbol**.
- **Thousands separator** on or off, and which character.
- **Negative numbers:** leading minus, trailing minus, red, or parentheses. Leading and trailing minus and parentheses exclude one another.
- **Currency symbol** and whether it goes before or after.

A sample line shows the result. The change is one undo step. The cell's own text in the file stays the plain number.

See [Styles](styles) for editing styles, and [Formulas and Quick Add](table-formulas) for calculated cells.
