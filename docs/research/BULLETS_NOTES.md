# Bullets and numbering in Ami Pro 3.1: evidence (2026-10-17)

For the bullets and numbering run (`docs/BULLETS_RUNBOOK.md`, slice BN.1).
Decompiler output stays outside Git (`../AmiPro-Reference/ghidra-work/out/`:
`amipro.c`, `amipro-cmds.c`, `amipro-annotated.c`, `amiproui.c`); these are
findings. Evidence levels as in `SAM_FORMAT_NOTES.md`: VERIFIED
(decompile), VERIFIED (fixture), VERIFIED (manual) for the 3.1 user's
guide (chapters 6, 7, 9, and 11). Counts from the corpora are aggregate;
the user's file names stay out of Git.

Style bullets and numbers (`[spec]`) were mapped in Phase 3.2e
(`SAM_FORMAT_NOTES.md`, "Bullets and numbers (`[spec]`)"); this file adds
what the run needs beyond them.

## What Ami Pro has (VERIFIED (decompile), (manual))

Ami Pro 3.1 has **no Bullets or Numbering command**: neither its menus
(`AMIPROUI.DLL` menus 1002 and 1003, `tools/ne_resources.py … menus`)
nor its SmartIcons (`AMIICNV.DLL` strings 203–322) toggle a list. It has
three things:

| Where | What | Evidence |
|---|---|---|
| Style › Modify Style › Bullets & Numbers | A style's bullet, number, text, attributes, Right align, and Space for (`[spec]`; every paragraph in the style shows them) | Manual ch. 7; Phase 3.2e |
| Edit › Insert › Bullet… (command `0x344`; SmartIcon 316 "Insert a bullet", not in the default set) | Puts one bullet into the text at the insertion point | Manual ch. 6; decompile below |
| Style › Outline Styles… (`0x140`) | Outline levels and outline numbering (Quick Numbering, Reset options, cumulative numbers) | Manual ch. 11; Phase 5 |

The manual's ways to make a list: apply a style the sheet already has
(**Bullet 1**, **Bullet**, **Number List**: "If you select one of these
style sheets when you create a document, all you need to do is assign the
paragraph style"), add a bullet or number to a style in Modify Style, or
insert bullets into the text, with a Tab after them, by hand.

**The shipped style sheets** (ISO, 63 sheets): 41 have a bullet style (Bullet
1 in 29, Bullet in 23, Bullet 2 in 16) and 34 a numbered style at level 0
(Number List in 32); 22 have neither. `_DEFAULT.STY` has Bullet, Bullet 1,
and Number List. **The user's sheets** (192): 137 have a bullet style
(Bullet 1 in 103, Bullet 2 in 71, Bullet in 53) and 111 a numbered one
(Number List in 104); 55 have neither. Every Number List checked counts in
arabic numerals with a full stop (`<*:>.`).

`_DEFAULT.STY`'s list styles (Body Text's font, spacing, and lines; each
its own Next style):

| Style | `[algn]` All, First, Rest | `[spec]` text | Flags |
|---|---|---|---|
| Bullet | 0, 288, 288 | small dot (`<*0>`) | |
| Bullet 1 | 288, 288, 288 | small diamond (`<*5>`) | |
| Number List | 360, 360, 360 | `1.` (`<*:>.`) | 16 Right align |

## Inserted bullets (`<:B…>`)

**The command** (`0x344`, handler `1560:0122`; dialog 1116 "Insert Bullet":
a three-column list box 25350 of the 17 bullets, an Example, OK, Cancel)
maps the chosen row to the same codes as style bullets (rows 1–10: 0x10–0x19;
rows 11–17: 0x08, 0x0A–0x0F) and inserts a five-byte text item
`{3, 'B', code, ' ', ' '}` at the caret (`1560:0374` kind 9), in the same
way as the date and other fields. VERIFIED (decompile).

**In the file** it is `<:B<*X>>`, the code written as body text writes a
code (`<*X>` is X − 0x20): `<*0>`–`<*9>` the ten shapes, `<*(>` and
`<**>`–`<*/>` the seven pictures. Inside `<:B…>`, `<**>` is bullet 0x0A,
not a hard return. VERIFIED (fixture BL1: all 17 inserted and saved; a bold
one is `<+!><:B<*1>>`, one after a 24-point font change
`<:f480,…><:B<*1>>`).

**Layout and drawing:** the line builder gives it the width of the same
bullet as a style bullet (`1200:159c`, from the font's metrics where it
stands) and the line its font's height (`10d0:3b66`, the `'B'` case); the
painter draws it with the style bullets' routine (`1200:0af0`), at the
text's position (`10d0:3df5`). It counts as one character (`10d0:3d84`
returns 5, the item's length), so the arrows step over it and Delete or
Backspace removes it (manual: "Placing the insertion point immediately
before the inserted bullet and pressing DEL"). VERIFIED (decompile);
VERIFIED (fixture BL1): a bullet after a 24-point change is drawn at 24
points; bold does not change a drawn shape.

**Corpora:** 8 inserted bullets in one of the user's documents; 12 in two
ISO documents (README31 among them; its "what's new" lists are inserted
bullets followed by a tab). **This project keeps them as zero-width legacy
marks today**, so they are not drawn and take no width.

## The display gaps (from Phase 3.2e's "not yet")

**Right align** (`[spec]` value 8, bit 16; the painter `1108:4107`).
VERIFIED (decompile): with the bit set, the bullet text (numbers, text, and
bullets as the line builder measures them, `1200:1384`) is moved right so
that it ends one space-width (the font's space) before the first line's
text, All + First (the style's First indent, `+0x30`), when it is narrower
than First less that space; a wider text stays at All. Without the bit it
starts at All. Numbers therefore line up on their right edge ("9." and
"10."). The text's own first-line position does not change (`1200:12de`
pushes it right only by what a wider bullet needs, either way), so Right
align moves no text. Every shipped Number List has the bit; in the
corpora, 125 of the user's documents and 2 ISO documents store a
right-aligned numbered style, and 40 of the user's sheets and 39 ISO
sheets have one. This project draws them at All.

VERIFIED (fixture BL2): ten Number List paragraphs in `_DEFAULT.STY`;
"1." to "9." end where "10." and "11." end, one space before the text,
and "10." and "11." (with their space as wide as the 0.25 in First) stand
at the left indent.

**Superscript** (attribute 256): the bullets' font is chosen by
`1200:168b`, which takes the attributes of the text where the paragraph
starts, adds the bullets' own, and selects the font through the same
routine as text runs (`10d0:0000`), so superscript bullet text is set at
superscript size (7/12) and raised as superscript text is (`1108:4063`,
5/12 of the size); for a text starting with the asterisk or dagger number
code (0x1F, 0x07) the script bits are cleared. Right align's space is
measured in that font. VERIFIED (decompile); VERIFIED (fixture BL3:
Number List with Superscript, stored as attributes 256 with flags 16: the
numbers small and raised, ending a small space before the text). Used by
8 of the user's documents and 3 sheets.

**Pictures** (codes 0x08, 0x0A–0x0F): placeable Windows metafiles 2001–2007
in `AMIPRO.EXE` (resource type 2000), drawn by `1200:16fc` into a square of
the font's ascent (`1200:0af0`). Seen in the Insert Bullet dialog (BL1): a
pointing hand (0x08), a box with a shadow (0x0A), a box with a thick top
(0x0B), a box with a check (0x0C), a box with a cross (0x0D), and two 3-D
arrowheads pointing right, lit from above (0x0E) and from below (0x0F).
This project draws an arrowhead glyph for all seven. The metafiles are
Ami Pro's artwork and are not copied: the run draws its own shapes that
read the same. In 37 of the user's documents and 13 sheets.

## Space for (VERIFIED (decompile))

Modify Style › Bullets & Numbers' **Space for** box (25305) writes the same
field as the Alignment panel's **First** box (25302): offset `0x309` of the
dialog's style copy (All `0x307`, First `0x309`, Rest `0x30b`, Right
`0x30e`; `11f8:0000`; the Alignment panel's fill `11d0:0824`), which the style writer stores as
`[algn]` value 4 (`11c8:26b7`). So Space for is the first-line indent
under another name, as the manual says ("The first line of those
paragraphs is indented by the amount you specified in the Space for text
box"); the model already has it (`Indents::first`). `[spec]` value 4 is
something else (Phase 3.2e).

## A style Ami Pro creates (VM fixture BL4; VERIFIED (decompile))

BL4 is `_BASIC.STY` (no list style) with Style › Create Style "Number
List" based on Body Text, Modify: All, First, Rest 0.25 in, Number "1"
with "." typed in Text (the number box is a list, not an edit box; the
manual's "type a period in the text box" means Text, which Ami Pro writes
after the number: `<*:>.`). Ticking Number turned Right align on by
itself. Its `[tag]` record:

- **The function key is the next free one** (6, after Body Text, Body
  Single, Header, and Footer's 2–5): Create Style gives the new style
  `10a8:0000`'s key, the lowest of F2–F12 no style has, never F10 (the
  menu key), or none when all are taken (`15c8:04fb`).
- **`[spec]` value 4 is 0.** It is Ami Pro 2's Space for: the style reader
  (`1058:0fac`) moves it into First for an old record (value 6 is 0) and
  clears it, and for an old record with a bullet text and no First sets
  First to 0.25 in. `_DEFAULT.STY`'s 360 for Number List and Bullet is a
  leftover of that.
- Everything else is Body Text's: `[fnt]`, `[spc]`, `[brk]` 4, `[line]`,
  `[nfmt]`, and the style's own name after `[nfmt]` with no tabs and no
  right indent.

## The style record in memory (VERIFIED (decompile), `11c8:26b7`)

For later reference: `[spec]` value 1 is the byte at `+0x65`, the level
`+0x66`, value 4 `+0x67` (16 bits), value 5 `+0x69`, value 6 `+0x6a`,
the attributes `+0x6b` (stored in another bit order, converted by
`11c8:2631`), the flags `+0x6d`, the text from `+0x6e`; `[algn]` is the
flags `+0x2c`, the unit `+0x34`, All `+0x2e`, First `+0x30`, Rest `+0x32`.

## Fixtures

| Fixture | What | Shows |
|---|---|---|
| BL1 | A `_DEFAULT.STY` document: each of the 17 bullets inserted into its own paragraph with Edit › Insert › Bullet…, one in bold, one after a change to 24 points, one picture at 24 points | The `<:B…>` forms; inserted bullets take the size where they stand; the dialog's list (`bl1-list`) |
| BL2 | Eleven Number List paragraphs (F6) in a `_DEFAULT.STY` document | Right align: the numbers end one space before the text; "10." stays at the left indent (`bl2-doc`) |
| BL3 | Three Number List paragraphs with Modify Style › Bullets & numbers › Superscript; the panel showed Space for 0.25 (the First indent) | Superscript numbers small and raised; Right align's space in the reduced font (`bl3-doc`, `bl3-bn`) |
| BL4B, BL4 | `_BASIC.STY` with three Body Text paragraphs (BL4B); then Create Style Number List as above, applied with its key, F6 (BL4) | A created style's function key and `[spec]` value 4 (`bl4-*`) |
| ABL1, ABL2, ABL4 | The app's twins (`bullet_twins`): BL1's bullets put back with Insert › Bullet…, BL2's paragraphs as Body Text given Numbering, BL4B given Numbering | Written as Ami Pro wrote BL1, BL2, BL4 (body and the made `[tag]`); Ami Pro shows each as its own, pixel for pixel (`abl*-open`, `bl*-open`) |
