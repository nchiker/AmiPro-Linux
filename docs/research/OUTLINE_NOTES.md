# Outline levels and outline numbering in Ami Pro 3.1: evidence (2026-10-17)

For the outline levels run (`docs/OUTLINE_RUNBOOK.md`, slice OL.1).
Decompiler output stays outside Git (`../AmiPro-Reference/ghidra-work/out/`);
these are findings. Evidence levels as in `SAM_FORMAT_NOTES.md`: VERIFIED
(decompile), VERIFIED (fixture), VERIFIED (manual) for the 3.1 user's
guide, chapter 11 ("Organizing a document using Outline mode and outline
numbering"). Bullets and numbers in general: `BULLETS_NOTES.md`.

## The model (VERIFIED (manual), (decompile))

- **Each level is a paragraph style** with an outline level 1–9 (`[spec]`
  value 2; 0 is None). Levels are counted by level, not by style: "Ami
  Pro increments the outline numbering based on the outline level,
  regardless of the paragraph style" (manual; `1200:1842` counts every
  paragraph at the level).
- **Style › Outline Styles…** (command `0x140`; dialog 2006, procedure
  `OUTLINESTYLEDLGPROC` `12c8:0000` in `AMIPROUI.DLL`): every style against
  levels 1–9 and None, Promote and Demote buttons that change a style's
  level, Quick Numbering, Number (none, 1, I, i, A, a, *, †), Separator
  (none, `.`, `:`, ` -`, `)`, `]`, `()`, `.)`, `(.)`), and three Reset
  options.
- **Outline mode** (View › Outline Mode) shows the outline with level
  icons; Outline › Promote and Demote (Alt+Left, Alt+Right) give a
  paragraph the style one level up or down. In Layout mode Alt+Left and
  Alt+Right do nothing (VM fixture OL1).

## Reset options (VERIFIED (decompile) `12c8:060c`, `1200:1842`)

`[spec]` value 8's bits (beside 16, Right align):

| Bit | Option | Counting at level L (`1200:1842`) |
|---|---|---|
| 2 | After higher level | restarts after a paragraph with a level smaller than L |
| 4 | After intervening style | restarts after any paragraph with another level or None |
| 8 | Cumulative numbering | each number code k shows level k's count (below) |

After higher level and After intervening style exclude each other in the
dialog. With neither, the count never restarts ("If you do not select a
Reset Option, Ami Pro does not restart the numbering"). The flags come
from the style at level L: the paragraph's own when it is at L, else the
first style with level L. Level 0 (None) counts a style's unbroken run of
paragraphs, as before (BN2).

**Cumulative numbering** (`1200:1384`, `1200:1489`): the number codes in a
style's text count, in order, levels 1, 2, 3…; without the bit every code
shows the style's own level. A full stop right after the last level's
code is not shown, so `1.1.1.` shows "1.1.1" (VERIFIED (fixture) OL2: 1,
1.1, 1.2, 2, 2.1).

**Today** this project counts every levelled style as if After higher
level were set, and shows every code at the style's own level: right for
the shipped outline sheets and for BN2 (all bit 2), wrong for the
levelled numbered styles with neither option (25 records in the user's
corpus, 7 in the ISO sheets), with After intervening style (6), and for
cumulative numbers (3, each with one code).

## Quick Numbering (VERIFIED (decompile) `12c8:06d6`, `12c8:0829`, `12c8:0755`, `12c8:0661`)

Applied to every style in the list:

- **I, A, 1, i, a**: levels 1–9 get `I.`, `A.`, `1.`, `i.`, `a.`, `1)`,
  `i)`, `a)`, `(1)` and After higher level; exactly the shipped
  `_OUTLIN1.STY`'s Outline 1–9.
- **1, 1.1, 1.1.1**: level n gets `1.` n times (`<*:>.<*:>.…`) and After
  higher level with Cumulative (VERIFIED (fixture) OL2).
- **Clear**: no number, no reset bits.
- **Styles without a level lose their numbers** with any of the three
  (OL2: Number List's `<*:>.` emptied; bullets and text kept): the number
  part is replaced (`12c8:08d4`) for every style.

## Promote and Demote in Outline mode (VERIFIED (fixture) OL1; manual)

`_DEFAULT.STY` (Title level 1, Subhead level 2):

- Demote on Subhead (2), with no style at level 3: Ami Pro makes
  **Outline3**, a copy of Subhead at level 3 (same reset bits), with the
  next free function key (12, after 2–9 and 11; `10a8:0000`) and its own
  name as Next style, and gives it to the paragraph.
- Promote on Body Text (None) after a Subhead paragraph: Subhead (level 2,
  the previous levelled paragraph's level). Demote on it: level 3, the
  existing Outline3.
- With several styles at a level, Ami Pro "chooses one" (manual).

## Fixtures

| Fixture | What | Shows |
|---|---|---|
| OL1 | `_DEFAULT.STY`: Title, Subhead, Body Text, Subhead; in Outline mode Demote the last, Promote then (undone) Demote the Body Text | Outline3 made; None promoted and demoted; Alt+arrows inert in Layout mode (`ol1-*`) |
| OL2 | `_DEFAULT.STY`: Title, Subhead ×2, Title, Subhead; Outline Styles › Quick Numbering 1, 1.1, 1.1.1 | Cumulative numbers, the hidden last full stop, After higher level, Number List's number cleared (`ol2-*`) |
