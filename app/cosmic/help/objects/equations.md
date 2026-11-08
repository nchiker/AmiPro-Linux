---
id: equations
title: Equations
area: objects
keywords: equation, math, tex, latex, formula, amieqn, symbols, equation editor
see_also: equation-source, equation-tex-reference, equation-palette, equation-errors, equation-files, charts
context: command:Equations, panel:Equation
---
An equation is a picture drawn from a short piece of text written in Ami Pro's dialect of TeX. You type the text in a small card on the page, and the picture updates as you type.

## Starting

**Tools › Equations…** (outside Draw mode) opens the editor:

- on an **equation frame** (also by double-click, or Enter on a selected one): edits that equation;
- on an **empty text frame, an empty table cell, or in the main text**: makes a new empty equation in a small frame that flows with the text.

The editor has two parts. The **card** sits under the frame on the page, with the TeX source and the caret in it. The **Equation panel** in the rail holds the [template palette](equation-palette) and the options. Esc or Enter in the card leaves the editor; the panel's X closes it too.

## While you type

Each edit that parses is applied at once. A text that does not parse is not applied: the frame keeps its last good picture and the card shows a message ([Equation messages](equation-errors)).

A template, a face or size change, an import, or a pasted LaTeX is one undo step; typing after a template joins its step. Undo of the step that made the equation removes it and closes the editor.

## Face and size

The panel's **Face and size** section picks one of four faces (the first is Ami Pro's `TimesNewRomanPS`) and a size from 1 to 400 points, both stored with the equation.

Layout follows TeX's rules, fitted to Ami Pro's sizes, so it may differ slightly from Ami Pro's drawing. Equations appear as pictures on the page, in PDF and in exported documents.
