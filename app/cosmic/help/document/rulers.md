---
id: rulers
title: Inserting and removing rulers
area: document
keywords: insert ruler, remove ruler, paragraph ruler, clear tabs, own ruler, go to ruler, ruler tabs, paragraph tabs
see_also: tabs, ruler-view, indents, go-to
context: command:InsertRuler, command:RemoveRuler
---
A paragraph's **ruler** is a set of tabs and indents that belong to that paragraph alone, ahead of those from its style and its page layout. The Format › Ruler submenu has two commands.

## Insert Ruler

Insert Ruler gives the caret's paragraph a ruler of its own. It starts as a copy of the stops in effect there (its style's, or the page layout's), so nothing moves until you change something. After that, changes made on the ruler, such as adding a tab or dragging an indent, are kept on that paragraph and not on its style. The ruler applies to the paragraphs selected.

Any change to a paragraph's tabs in the ruler creates its ruler automatically, so you need Insert Ruler only to make the ruler first.

## Remove Ruler

Remove Ruler takes the paragraph's own ruler away and returns it to the tabs of its style or layout. The ruler's right-click menu has the same command as **Remove Paragraph Ruler**, and **Clear Tabs**, which leaves the paragraph with a ruler that has no stops at all.

## The ruler with the keyboard

Edit › Go To Ruler puts the keyboard on the ruler at the caret paragraph's indent: Left and Right move it a sixteenth of an inch, Ctrl with them half an inch; Space adds a tab; Delete removes the tab there; 0, 1, 2, and 3 set All, First, Rest, and the right indent. Esc or a click in the text comes back. See [Go To](go-to).

Showing and hiding the ruler is in [The ruler](ruler-view). Tab kinds and leaders are in [Tab stops](tabs).
