# Outline mode in Ami Pro 3.1: evidence for run B (2026-11-05)

For `docs/OUTLINE_MODE_RUNBOOK.md` (OB.2 to OB.5). Read-only study of the
decompiled `AMIPRO.EXE` and `AMIPROUI.DLL` (routines by segment:offset; no
decompiler output is kept in Git), their menu, accelerator, bitmap and
string resources (`tools/ne_resources.py`), `_OUTLINE.STY` and
`MACTOKEN.SAM` of the install. A few routines the dump had missed were
decompiled with `tools/ghidra/DecompAt.java` (`1070:182e`, `14e0:19f8`
and the like). Levels of evidence as in `COMMANDS_NOTES.md`: **D** decompile,
**R** resources, **I** inference; confidence high, medium, low. Nothing here
was run in the VM; the last section lists what is left for it. The levels
themselves (a style's outline level 1 to 9, Promote and Demote choosing
styles, Outline Styles…) are in `OUTLINE_NOTES.md`.

## 0. The pieces (D, R; high)

- **Mode word** (`DAT 2a39`, the document's view state): 0x01 Layout, 0x10
  Draft, 0x20 Outline (so Outline is 0x30), 0x80 a table's cell-selection
  mode. View › Outline Mode (0xda, `1110:0b7f`) enters Draft (0x10) first and
  then 0x20; choosing it while in Outline does nothing (no toggle). Layout
  (0xd2) and Draft (0xd3) leave it through `1110:0000` and `1110:090a`.
- **Outline menu** (menu 1058, "&Outline", string 2197): in the menu bar only
  in Outline mode (AMIPROUI `1028` message 0x24 adds it, 0 removes it).
  Commands: 0x2ef Promote (`1608:0000`), 0x2f0 Demote (`1608:0062`), 0x2f1
  Move Up (`1070:17d8`), 0x2f2 Move Down (`1070:1803`), 0x2f3 Expand
  (`1608:00c4`), 0x2f4 Contract (`1608:0111`), 0x2f8 Use Outline Styles
  (`15c8:1c0a`). Three more commands have no menu item: **0x2f5 Show
  levels n** (`1608:1773`; macro `OutlineLevels` takes one number), **0x2f6
  Expand one level** (`1608:17b3`) and **0x2f7 Contract one level**
  (`1608:191e`). 0x2f9 (`1608:1a4a`) is a wrapper that sends the same
  as 0x2f5 from a message. Macro names (`MACTOKEN.SAM`): OutlineMode,
  OutlineStyle (0x140, the Outline Styles… dialog), OutlineLevels,
  MoveParagraphUp, MoveParagraphDown, MoveLeftOrPromote (0x2ef),
  MoveRightOrDemote (0x2f0). Hint strings exist only for 0xda ("Display a
  collapsible and expandable view of the current document.") and 0x140.
- **Accelerators** (AMIPROUI table 1004; Alt, virtual key): Alt+Left 0x2ef,
  Alt+Right 0x2f0, Alt+Up 0x2f1, Alt+Down 0x2f2, Alt+PageUp 0x2f4 (Contract),
  Alt+PageDown 0x2f3 (Expand). They are global, not tied to the mode.
- **The button bar** (window class `OutlineWnd`, `AMIOUTLINEWNDPROC`
  `1050:0000`; created and destroyed by `1050:0909` on message 0x21, sent on
  entering and leaving Outline mode). Buttons, left to right: **1 to 9 and
  an "All" button**, each sending 0x2f5 with 1 to 9 or 10; a gap; four
  buttons Promote, Demote, Move Up, Move Down (0x2ef, 0x2f0, 0x2f1, 0x2f2);
  a gap; four buttons 0x2f6, 0x2f7, 0x2f3, 0x2f4 (`1050:0285`, the dispatch
  at `1050:02c3` to `1050:0393`). The button of the current level is drawn
  pressed (All for 10). Faces (AMIPROUI bitmaps 1155 to 1166): left and right
  arrows with a dot, an arrow up and down with a bar, a small plus, a small
  minus, a big plus, a bold minus (the order of the last four buttons is my
  reading of the faces: I, low).

## 1. The display

**The draw path (D `1108:0166` via `1108:0000`; medium).** Outline mode paints
through the Draft routine (bit 0x10 stays set), with the outline flag on:
the lines of the current flow (the main text) only, with no page frame,
margins, headers, footers, footnotes. Frames, and so their pictures and
tables, are not drawn in the main flow's lines (the draw loop walks one flow's
line list); how a table's rows appear is not shown by the code (open).
Pages and page breaks do not exist (a continuous list). Fonts are the
paragraphs' styles' (`1098:0333`, see §4 for the one exception).

**Indent per level (D `1108:0f06`; high for the rule, medium for the unit).**
- A levelled paragraph (style level L from 1 to 9) starts at (L - 1) times
  the **indent step**; level 1 is at the left edge.
- **Body text** (level 0) takes a display level N: its x is (N - 1) times the
  step, minus **half a step** when that is not 0. N is the nearest preceding
  paragraph's: the heading's level plus 1 under a heading, the previous body
  paragraph's N under body text (`1070:44bc`, kept in the low nibble of the
  paragraph record's byte 0xf; `1070:2c8c` sets it for a new paragraph). So
  body text sits **half a step to the right of the heading above it**.
  Body text before any heading: not settled (open).
- The paragraph's own indents and tabs are replaced by this in the lines'
  first line; how wrapped lines are placed is not shown (open).
- **The step** is 480 (`DAT 2a3f`, set to 0x1e0 at start): (window width / 3
  x 2) / 9 when that is less than 480 (`1610:0084`). The unit is the
  window's logical unit; 480 matches 1/3 inch in twips (I, medium).
  A mouse drag turns an x position into a level by dividing by the step
  with rounding to the nearest (`1608:0d19`; at most 10).

**The icons (D `1108:0f06`, `1108:12f7`, `1108:13be`, `1110:07c2`; high).**
Drawn only when View Preferences › **Outline buttons** (0x4000) is on and
the line is a paragraph's first line, left of the text start by 4/3 of the
icon's size. One icon per paragraph; `12f7` looks at the **next paragraph
(in storage order, hidden or not)**:

| Icon (bitmap id, mid size; AMIPRO.EXE) | When |
|---|---|
| **Plus, grey** (1202) | a heading whose next paragraph is deeper or body text and **is shown** (a heading with subordinates, expanded) |
| **Plus, coloured (cyan)** (1210) | a heading whose next paragraph is deeper or body text and is **hidden** (contracted) |
| **Minus** (1203) | a heading whose next paragraph is the same style or a levelled paragraph of the same or a higher level, or the last paragraph (no subordinates) |
| **Small box** (1200) | body text (level 0), also at the end |
| tiny dot (1217) | used when the icon would be far left of the window edge (the overflow case in `13be`) |

Sizes 10 x 10 (ids 1200 to 1203, 1210, 1217), 8 high (1204 to 1209, small
screens, 4 to 6 pixel dots), 15 x 15 (1211 to 1215, large screens): the set
is picked by a screen metric below 0x17, below 0x28, or above. Hit-testing
`1048:158e` (`1088:14ba`) needs the same option.

**Hidden state and collapse marking.** A paragraph is hidden when (`1608:154c`;
high): its flag 0x20 is set; or its flag 0x10 is clear and (its level is
greater than the **shown level N** (`DAT 2468`, 1 to 10, 10 = all), or it is
body text and N < 10). Flag 0x10 means "forced shown", 0x20 "forced hidden"
(byte 5 of the paragraph record). A contracted section is just paragraphs
that are not drawn, with the heading's coloured plus; no ellipsis or other
mark (high).

## 2. A section; Move Up and Move Down

**Section (D `1608:015e`, `1608:12a7`; high).** For the heading at the caret
(base level B from 1 to 9) the section is the paragraphs after it until the
first **levelled paragraph with level <= B**; body text (level 0) and
deeper headings are in it. Level 0 paragraphs have **no section** (Expand and
Contract do nothing on them; a click on a body paragraph's icon selects it
alone). Clicking an icon (`1608:12a7`) selects the paragraph and its
section as the selection (through visible paragraphs). The commands act
only when the selection lies in one paragraph (a caret or an in-paragraph
selection) and not in a table's cell mode.

**Move Up / Move Down (D `1070:17d8`, `1070:1803` push the key code 0x26 or
0x28 to `1070:182e`; `1070:190c` does the move; high for the rule).**
- No mode check: the handlers work in Layout and Draft as well (the
  accelerators are global). Only the table cell-selection mode diverts them to
  moving a table row or column (`14e0:19f8`, `14e0:1ae5`, also for Alt+Left and
  Alt+Right, `14e0:190a`, `14e0:17df`: "MoveLeftOrPromote" and the like).
- The unit is the **selected paragraphs** (a caret is one paragraph). In
  Outline mode the block's end is extended over the hidden paragraphs that
  follow it (`1608:145f`: the paragraphs up to the next shown one), so a
  **contracted heading moves with its hidden text** and an **expanded
  heading moves alone**. (This agrees with the VM check OM1 in the runbook.)
- **Move Down**: the block goes after the **next shown paragraph** (`1070:443e`),
  itself extended over its hidden tail. **Move Up**: before the **previous
  shown paragraph** (`1070:40e4`). At the document's end or start nothing
  happens (the neighbour is "none"); no message.
- The paragraphs are copied one by one (text, style, flags 0x10 and 0x20
  kept, the paragraph record's other flags `1070:2b84`) and the old ones
  removed (`1070:2e03`); the selection or caret follows the moved
  paragraphs. With revision marking on (a state bit at `DAT 2a3a & 0x40`) it
  writes insertion and deletion codes; this run has no revision marking.
- What happens to frames anchored in the paragraphs or to a table among them
  is not shown: only the paragraph records move (open). A table cell's
  paragraphs are in other flows, so they are never part of the move.

## 3. Expand, Contract, Show levels

**Expand / Contract (D `1608:00c4`, `1608:0111`, `1608:015e(level, all,
expand, start)`; high).** Both are "all levels" (the second argument is 1) on
the section of the caret's heading: every paragraph of it, whatever its level,
gets the flags below. The heading itself is not changed.

| Paragraph | Contract (Alt+PageUp) | Expand (Alt+PageDown) |
|---|---|---|
| levelled, level <= N | set 0x20 (hidden) | clear 0x20 |
| levelled, level > N | clear 0x10 (hidden by default) | set 0x10 (forced shown) |
| body text | set 0x20, clear 0x10 | set 0x10, clear 0x20 |

So **Expand shows the whole section down to body text whatever N is**; Contract
hides it all. They do not remove one level at a time: that is 0x2f6 and 0x2f7.

**One level (D `1608:17b3`, `1608:191e`; medium).** They run the same routine
with "all" off, which touches only the paragraphs at **one level**. **0x2f6
(Expand one level)** takes the smallest level among the section's hidden
levelled paragraphs and shows those (with only hidden body text left, shows it);
**0x2f7 (Contract one level)** takes the deepest level of the section's
levelled paragraphs and hides those, but at the first body text paragraph it
stops and contracts the **body text** instead. Reached from the button bar
only (no menu, no accelerator).

**Show levels n (0x2f5, `1608:1773`, `1608:15cf`; high).** n from 1 to 9, or
10 for all (0 and anything above 10 count as 10). It sets N (per document,
10 on opening), **clears every paragraph's 0x10 and 0x20 flags** of the
flow (`1608:09ea`, `1608:0ae4`), and moves the caret to a shown paragraph
(`1608:1623`: the previous shown one, else the next). So it also **hides
body text for n < 10**, and every earlier Expand or Contract is forgotten.
Entering Outline mode clears the flags the same way (`1110:0000` calls
`1608:09ea`); N survives (high for the clear, medium for N surviving).

**Stored in the file?** No sign of it: N is a view variable, the flags a
byte of the in-memory paragraph record that the save code does not carry (I,
medium; check in the VM by Save and reopen). Collapsed state is lost on
re-entering Outline mode in any case.

**Mouse and keys.** A double click on a paragraph's icon toggles it
(`1608:0cc5`): Expand when the next paragraph is hidden, else Contract
(all levels). A mouse down on an icon starts a drag (`1608:0d19`,
`1608:0fa5`, `1608:1388`): vertically it moves the section (through
`1070:190c`), horizontally it changes the level (through `1608:056e` with
the step of `DAT 2a3f`); a body paragraph's level is shown as 10.

## 4. Use Outline Styles (0x2f8; D `15c8:1c0a`, `1608:0b4b`, `1608:0c5d`, `1098:0333`; high)

- A **global setting, not a document's**: bit 0x80 of `DefOptions` (`DAT
  2c53`, the word read from `AMIPRO.INI` key DefOptions, default 0x46d, which
  has the bit off). The menu item is a check item (state routine at `1180:07d7`
  area). Nothing goes into a SAM. It is written back with the INI at exit
  (`DefOptions` is written back with its other bits; medium). It is applied
  to every open document at once.
- **Turning it on** (in Outline mode, or when entering Outline mode with it
  on) loads the style sheet **`_OUTLINE.STY`** (string 1332, found along the
  style path; "Outline - used in Outline mode") and keeps, per level 0 to 9,
  the style that has that level: a table of ten style records (`DAT 2346`).
  If the file cannot be read, nothing is loaded; a file of the wrong kind
  gives the box "Error reading from disk." (string 3010, `1608:0b4b`). Turning
  it off frees the table.
- **Effect: display only.** In Outline mode `1098:0333(1, …)` returns, for
  each paragraph, the **table's style for its level** instead of its own style;
  it is used for lines, icons' hidden test, section checks and level reads.
  Editing calls use the paragraph's own style (`0333(0, …)`): the paragraph
  keeps its style in the document, and typing and Enter use it. A level with
  no style in the table gets a new one made as a copy of the level 0 (else 1,
  else default) style with that level (`1098:0333`).
- The shipped `_OUTLINE.STY` (read): Body Text Arial 12; **Level 1 Times New
  Roman 18, bold; Level 2 Times 14; Level 3 Times 12; Levels 4 to 9 Arial
  12** (sizes in twentieths of a point, 360, 280, 240); Header and Footer.
  Level 1 has `[spec]` value 8 = 16 (Right-align numbering bit); no numbering
  otherwise. The styles of the document are not changed and not saved.
- It does **not** change what Promote and Demote do: they look for a style
  with the wanted level in the document's own list (`1608:03b7`).

## 5. Promote and Demote, as Outline mode does them (D `1608:056e`, `1608:03b7`)

Adds to `OUTLINE_NOTES.md`. The range is the caret's paragraph or the
selected paragraphs, extended over a contracted heading's hidden tail
(`1608:145f`), so **hidden subordinates are promoted and demoted with it**.
- **Levelled paragraph**: Demote is level + 1, Promote level - 1; **level 1
  cannot be promoted** (nothing happens); **demoting level 9 gives level 0**
  (`if 9 < new level then 0`: the style found for level 0; medium-low).
- **Body text** as the first paragraph of the range: Promote gives the
  display level of the text minus 1 (the heading's level above it), Demote
  the display level itself (the heading's level + 1); with a result of 0
  nothing happens. Body text after a levelled first paragraph is left alone
  (medium-low).
- The style for the new level is the first style of the document's list
  with that level; none: a copy of the paragraph's style named `Outline<n>`
  is made (level set, next free function key; `1608:03b7`; matches OL1).

## 6. Typing in Outline mode (D `1070:2462`, `1070:283e`; medium)

- Enter at a paragraph's **end** inserts a new paragraph. Its style is the
  current paragraph's **own style** (not the outline table's), or, when the
  style's flag 0x40 (at +0x40) says it has a different following style,
  the style **named by its Next style field** (+0x42) if that exists.
  So Enter at a heading's end gives what the heading style's "Next style"
  says (in `_DEFAULT.STY` for Subhead and Title: Body Text), the same as in
  Layout. Enter in the middle splits the paragraph; both halves keep the style.
- In Outline mode the insert point is after `1608:145f` of the caret
  paragraph: **after its hidden tail** when it is contracted; directly after
  it otherwise (so before its expanded children).
- Text cannot be typed into a hidden paragraph (the caret is always on a
  shown one, `1608:1623`).

## 7. Limits and messages

- Level 1 to 9 and None; N from 1 to 10 (`1608:1773` clamps 0 and >10 to 10).
  Ten slots in the outline style table.
- Strings: 218 (View › Outline Mode hint), 320 (Outline Styles hint), 1331
  "Outline" (the base of `Outline<n>`), 1332 "_OUTLINE.STY", 3010 "Error
  reading from disk." (only from `1608:0b4b`), 2197 "&Outline". "1 Level" to
  "4 Levels" (2224 to 2227) are Undo levels, not outline.
- No message appears for Promote at level 1, Move at either end, Expand or
  Contract on body text, or an unavailable level.

## Open points for the VM

1. **Indent**: the step in pixels or twips at 100 % zoom (is 480 twips = 1/3
   inch right?); how wrapped lines of a heading and of body text are placed;
   body text at the document's start before any heading; whether the step
   shrinks in a narrow window as the code says.
2. **Icons**: which icon shows for each case (grey plus, cyan plus, minus,
   box), the last paragraph, a heading followed by a heading of the same
   level (minus), a heading followed by a deeper heading that is hidden by
   Show levels; where exactly the icon sits (vertical centre of the first
   line?); the button-bar faces and which of the last four does what.
3. **What is drawn**: do frames, pictures, tables (their rows), page and
   column breaks, footnotes, headers and the "System font in draft mode" box
   show in Outline mode; fonts and sizes with Use Outline Styles off and on
   (Level 1 18-point bold Times, and so on).
4. **Move Up and Down**: an expanded heading with a visible child (does it
   move past one paragraph?); a contracted heading with a contracted next
   sibling; with a selection across several paragraphs; with a frame
   anchored in the moved paragraph; a table in the main text; Move at the
   first and last paragraph; Move in Layout and Draft mode (the code gates
   nothing, the menu does not exist there); undo (one step?).
5. **Expand and Contract**: Alt+PageDown on a heading under Show levels 2;
   0x2f6 and 0x2f7 from the bar (the order and the body text rule, §3); Expand
   or Contract with a selection of several paragraphs (the code does nothing);
   double click on an icon; whether Expand on a hidden deeper level shows
   everything.
6. **State**: is Show levels n kept when leaving and entering Outline mode,
   when switching windows; is anything stored in the SAM (Save, reopen, in
   Outline mode: collapsed?); does Layout show everything.
7. **Use Outline Styles**: check mark; whether the setting survives a restart
   (INI `DefOptions` bit 0x80); the look of the levels; what it does when
   `_OUTLINE.STY` is missing from the style folder; whether Promote and
   Demote are changed.
8. **Enter** at the end of a heading in Outline mode with Subhead (Next style
   Body Text) and with a style whose Next style is itself; at a contracted
   heading's end (after the hidden text?); in the middle of a heading.
9. **Promote and Demote**: demote at level 9 (to None?), body text in a
   mixed selection, Promote at level 1, a contracted heading's hidden text
   following it.

## VM fixture OM1 (2026-11-05, run A's session; screenshots `om1`–`om11`, outside Git)

`_DEFAULT.STY`, typed: Title "First title", Subhead A, two Body Text, Subhead
B, Body Text, Title "Second title", Subhead C, Body Text; View › Outline
Mode. Confirms the decompile where it overlaps (VM, high):

- No pages: the text at the window's width; Title and Subhead in their own
  fonts; each level one step further in, body text half a step right of
  its heading (about 28 and 14 screen pixels at 100%).
- Icons: a plus on a heading with text under it, a bold plus on a
  contracted one, a minus on a heading with nothing under it, a small box
  on body text.
- The Outline menu (Promote Alt+Left, Demote Alt+Right, Move Up Alt+Up,
  Move Down Alt+Down, Expand Alt+PageDown, Contract Alt+PageUp, Use
  Outline Styles) and the Outline bar (1–9, All, Promote, Demote, Move Up,
  Move Down, two pairs of plus and minus) appear in Outline mode.
- Contract on the first Title hid its whole section; Expand showed it
  all again.
- Move Down on an expanded Subhead A moved that paragraph alone over the
  next paragraph (twice: past both bodies); Move Up on a contracted
  Subhead B moved it with its hidden body.
- Show 1 left the two Titles; Show 2 left Titles and Subheads (no body
  text); All showed everything.
