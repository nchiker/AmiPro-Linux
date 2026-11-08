---
id: table-messages
title: Table messages and what to do
area: reference
keywords: error, formula error, cannot understand the formula, divide by zero, circular formula, REF, holds a formula, columns are 1 to 250, nothing to add
see_also: tables, table-formulas, table-insert, sorting-messages
---
These are the messages shown while you work in tables, and what each one asks of you.

## Inserting and sizes

- **Columns are 1 to 250. / Rows are 1 to 4000.** The Insert Table dialog stays closed until the numbers fit.
- **Type a column width such as 1.00, 2.5 cm, or 72 pt. / Type a row height of at least 0.01 in. / Type a length such as 1.50, 3 cm, or 72 pt.** A size field needs a number with an optional unit. Rows cannot be thinner than 0.01 inch.
- **A gutter is not less than 0. / A size is at least 0.01 in.** Raise the value.
- **Insert 1 to 250 at a time.** Use a smaller number, and repeat.
- **Empty: the columns fill the space.** A hint, not an error.

## Formulas

- **Cannot understand the formula in cell 'B7'.** The text is not a formula the program can read. The text stays in the box for correction: check brackets, cell names, and operators.
- The status bar reports a formula **dividing by zero**. The result is left blank. Fix the divisor.
- The status bar reports a **circular formula**. Cells refer to each other in a loop (this is the program's message "This field contains a circular formula."). The cell is blank; break the loop.
- A cell shows **REF** when a formula read a cell that is not a number. Fix that cell's text. There is no status message; the Formula section says so.
- **This cell holds a formula: use Table › Edit Formula… to change it.** Typing is refused in a formula cell.
- **Select one cell to edit its formula.** Edit Formula needs the caret in one cell.
- **Nothing to add: there is no cell before this one. / The cell holds no formula.** Quick Add needs a cell to the left or above; Remove formula needs a formula.

For sort messages see [Sort messages](sorting-messages).
