---
id: outline-styles
title: Outline Numbering and outline levels
area: document
keywords: multi-level numbering, heading numbers, 1.1.1, legal numbering, roman numerals, outline level, outline styles, cumulative
see_also: list-levels, outline-mode, modify-style
context: command:OutlineNumbering
---
Format › Styles › Outline Numbering… numbers all the styles that have an outline level in one step. It is the style-sheet counterpart of [Outline mode](outline-mode), which shows and rearranges the same levels.

## Giving styles a level

In Style Properties › Bullets and numbers, set **Outline level** (None, or 1 to 9) for a style. A level counts every paragraph at that level, whatever its style. Two reset options say when the count restarts: **After higher level** or **After intervening style**. **Cumulative numbering** builds the text from each level above it, so a third-level item shows 1.2.3.

## The dialog

Choose one of four schemes, then Apply: plain numbers with letters and roman numerals below them, the same with roman capitals first, decimal numbering that grows a part per level (the level-three item reads 1.1.1), or No numbers to remove the numbering.

The dialog lists the styles with a level that it will number. Styles without a level are left alone, and each style keeps its own text before the number and its bullet attributes. Apply is off when no style has a level. It is one undo step.

Promote and Demote ([list levels](list-levels)) move paragraphs between the levels.
