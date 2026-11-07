# Commands run: evidence from Ami Pro 3.1

Evidence for run A (`commands`, CM1–CM12) in `docs/SPEEDUP_PLAN.md`. Read on
2026-11-04, read-only, from:

- **D**: the decompiled `AMIPRO.EXE` and `AMIPROUI.DLL` (routines given by
  segment:offset; no decompiler output is kept in Git). The command ids are
  those of `tools/ghidra/command_table.py`.
- **R**: the resource dumps of `tools/ne_resources.py` (dialog templates,
  string tables, the menu) of the two unpacked files. Ami Pro shows a
  command's hint in the status line from the string whose id equals the
  command id; those hints are quoted below.
- **I**: the install's `AMIPRO.INI` (the Wine-Test and VM copies; neither
  holds a `DefOptions`, `Display`, or `AutoSave` key, so the code's defaults
  apply).

Nothing here was run in the VM. Confidence: **high** (read from the code and
the templates), **medium** (read, one step of reasoning), **low** (a guess to
be settled in the VM).

## 0. How Ami Pro keeps its settings (D, high)

- File `AMIPRO.INI`, section `[AmiPro]`. Key names are strings of the
  executable's table (ids 1000–1500 of `AMIPRO.EXE`); the start-up reader is
  `1018:08c0`. Integers are written as **decimal text**.
- Word-sized option sets, with the defaults the code uses when the key is
  missing:

  | Key | Default | Holds |
  |---|---|---|
  | `DefOptions` | 1133 (0x46d) | default mode and window bits (§3), use working directory |
  | `DefOptions1` | 1538 (0x602) | User Setup bits (§1) and Load defaults bits |
  | `DefOptions2` | 330 (0x14a) | not read for these questions |
  | `Display` | 19590 (0x4c86) | View Preferences bits (§3) |
  | `CleanOpts` | 64 (0x40) | Clean Screen bits |
  | `viewlevel` | 91 | the Custom zoom percent (§4) |
  | `initview` | 2 | the view a document opens in (§4) |

- Other keys of this area: `AutoSave`, `recfiles`, `LastOpen<n>`, `backup`,
  `docpath`, `stypath`, `macrodir`, `iconpath`, `defstyle`, `UserName`,
  `UserInitials`, `NoteColor`, `AutoMacroLoad`, `AutoMacroExit`.

## 1. Tools › User Setup (command 0xa2)

Hint (R): "Specify save, notes, speed, load, default path, and typographic
options." The command runs `14f8:0e55`. Its dialog is **AMIPROUI.DLL dialog
1019** (`DEFAULTSPROC`, `10f8:0000`); the buttons OK, Cancel, **Options…**
(dialog 1131), **Paths…** (dialog 1033), **Load…** (dialog 1008, "Load
Defaults"). Initial values are read in `10f8:0215`, written back in
`10f8:0653`.

Controls of dialog 1019 and where they land (D, high unless stated):

| Control (id) | Stored in | Notes |
|---|---|---|
| Auto backup (25550) | `DefOptions1` bit 0x80 | see "Backup" below |
| Auto timed save (25551) + minutes (25300) | `DefOptions1` bit 0x100; key `AutoSave` | minutes field takes 0 to 99 (two digits, step 1) |
| Undo levels (combo 25977) | `DefOptions1` bits 0x01–0x10 | the five choices, strings 2223–2227: **Off, 1 Level, 2 Levels, 3 Levels, 4 Levels** |
| Recent files (25303) | key `recfiles` | 0 to 5; code default 4 |
| Disable warning messages (25556) | `DefOptions1` bit 0x800 | |
| Disable one-line help (25557) | `DefOptions1` bit 0x20 | |
| Disable drag & drop (25559) | `DefOptions1` bit 0x40 | |
| Name, Initials (25301, 25302) | keys `UserName`, `UserInitials` | 25558 "Display initials in text" |
| Note color (25590) | key `NoteColor` | a colour bar |
| Run macros: Program load / exit (25554/25555 + combos) | `AutoMacroLoad` / `AutoMacroExit` | value `1,NAME` on, `0,NAME` off (the install has `automacroload=1,_autorun.smm!zrunmacs`) |

- **Undo levels.** The level is the one-hot bit of `DefOptions1`: 0x01 =
  Off, 0x02 = 1 level, 0x04 = 2, 0x08 = 3, 0x10 = 4 (`1018:08c0` turns the
  bit into the level 0 to 4). **The default word 0x602 holds 0x02, so the default is 1
  level.** The Edit › Undo history is therefore at most 4 deep in Ami Pro
  (`History` keeps 100 here).
- **Recent files.** The File menu lists up to `recfiles` entries, read from
  `LastOpen1`, `LastOpen2`, … (string 1409) in `[AmiPro]`; most recent
  first. Maximum 5 (the dialog's limit), default 4 when the key is absent
  (code default in `1018:08c0`; the VM copy of `AMIPRO.INI` shows five
  `LastOpen` lines, so a larger number was in force there).
- **Auto timed save.** Active only when `DefOptions1` bit 0x100 is set and
  the `AutoSave` key is a number; a blank key clears the bit. A window timer
  ticks every 60 seconds (`1520:0000`); when the count reaches `AutoSave`
  minutes (0 is treated as 1) and the document is modified and has a name
  and is not busy in a dialog, the timer posts **command 0x67, the normal
  File › Save** (`1520:009e`). Unnamed documents are skipped. So it saves
  over the document itself, not into a recovery file. (medium: the exact
  "busy" tests are the program's state words)
- **Backup.** `DefOptions1` bit 0x80 (Auto backup). Before a save writes
  over a file, `1190:06a0` copies the existing file into the **Backup
  folder**: key `backup` (Paths dialog, "Backup:"), default the root of the
  current drive. The copy keeps the file's own 8.3 name (name and extension,
  at most 12 characters); there is no `.BAK` string in either binary. The
  same routine also finds and treats the document's glossary files
  (`NAME.G*`, string at DGROUP 0x8aa). Failure messages: "There is not
  enough room on disk for backup of %s." and "Unable to complete backup for
  file %s." (strings 3020, 3021). (medium; whether the copy is renamed is
  not shown, check in the VM: save twice with Auto backup on.)
- **Options… (dialog 1131, "User Setup Options")**, all in `DefOptions1`
  or `DefOptions`: Hyphenation hot-zone (characters, 25300), Widow/Orphan
  control, Pair kerning, Hyphenate last word in paragraph, Hyphenate last
  word in column/page, Print in background, Flow in background, Graphic
  display speed options (combo). (medium: the mapping of each box to its
  bit was not traced)
- **Paths… (dialog 1033, "Default Paths")**: Document (`docpath`), Style
  sheet (`stypath`), Backup (`backup`), Macro (`macrodir`), SmartIcons
  (`iconpath`), and a check box **Use working directory** (probably
  `DefOptions` bit 0x10). The install defaults are `stypath=…\styles`,
  `docpath=…\docs`, `macrodir=…\macros`, `iconpath=…\icons` (I).
- **Default style sheet for File › New.** Key **`defstyle`**, default
  `_DEFAULT.STY`, looked up in `stypath` (`1018:08c0`, strings 1291–1293).
  It is changed in **Load… (dialog 1008, "Load Defaults")**, which has the
  style-sheet list ("List by description") and is saved by `12a8:02ea`
  (command 0xd9: writes `defstyle`, `initview` and the `DefOptions` bits). The File › New dialog (dialog 1055) pre-selects the
  list and offers: With contents, Run macro, Close current file, List by
  description, Preview. Load Defaults also sets: View Custom or Standard,
  Clean screen, Styles box, Maximized, and Mode Layout, Outline or Draft
  (the mode a document opens in).

## 2. Text › Fast Format (command 0xf6, Ctrl+T)

Hint (R): "Extract formatting information from text and apply it to other
text." Evidence: D `12e0:1cd4` (the command), `12e0:1d6c` (pick up),
`12e0:1fa9` (apply), `AMIPROUI.DLL FASTFORMATDLGBOX` (dialog 3041).

- **A toggle mode.** Choosing it while off picks up and turns the mode on;
  the menu item shows checked (global bit `0x0040 & 2`) and the mouse
  pointer changes to a brush pointer (cursor 0x3ee, or 0x3ef when a style
  name was picked up). Choosing it **again turns it off** and restores the
  arrow (`12e0:1d4e`). One more call of the same toggle exists at
  `amipro.c` 28705 (another command cancels the mode; not identified).
  Applying does not turn it off: each later selection made with the mouse
  is formatted (`1048:067e` calls `12e0:1fa9` after a mouse selection), repeatedly. (high for the
  toggle, medium for "another command cancels")
- **What it picks up (high).** From the caret or selection start: the
  paragraph's **style name** and the paragraph's own format words (the
  record at `0x2750`–`0x274e`, flags masked with 0x7bfc), always; and, when
  text attributes are wanted, the **text attributes at that position**
  (bold, italic, underline, word underline, case, super/subscript,
  strike-through… and the font, size and colour in the same call).
  The mask 0x7bfc **excludes** "No hyphenation" (0x0002), "Protected"
  (0x8000) and attribute bit 0x0001 and 0x0400, so those are not copied.
- **The options dialog (dialog 3041, caption "Fast Format").** It appears
  only when the pick-up is ambiguous: no text is selected ("You do not have
  any text selected.", string 2444) or the whole paragraph is selected
  ("You have selected the whole paragraph.", string 2445). It asks "What
  formats do you want to extract?" with two radio buttons: **Only text font
  and attributes** and **Only paragraph style**. The code checks the
  second button as initial focus and default (control 25451, `1318:0000`).
  With a partial selection the dialog is skipped and both are taken.
  Cancel ends the command without turning the mode on.
- **Applying (medium).** `12e0:1fa9` runs through twelve attribute bits
  (4, 8, 0x10, 0x20, 0x40, 0x80, 0x100, 0x200, 0x800, 0x1000, 0x2000,
  0x4000) of the picked-up set and applies each to the new selection, then
  applies the style and paragraph format record when one was picked up. The
  whole apply is one undo step (it runs inside the common edit wrapper).

## 3. View › View Preferences (command 0xce)

Hint (R): "Specify how the screen should appear while you work in a
document." Dialog **1018 "View Preferences"** (`10f0:0089` fills it,
`10f0:0209` reads it). The stored word is `Display`. Each box, its control
id, its bit and its default (D/R, high):

| Box | Control | `Display` bit | Default |
|---|---|---|---|
| Column guides | 25550 | 0x0001 | off |
| Margins in color | 25551 | 0x0002 | on |
| Pictures | 25552 | 0x0004 | on |
| Tabs & returns | 25558 | 0x0008 | off |
| Marks | 25553 | 0x0040 | off |
| Table gridlines | 25555 | 0x0080 | on |
| Table row/column headings | 25556 | 0x0100 | off |
| Vertical ruler | 25554 | 0x0200 | off |
| Horizontal scroll bar | 25559 | 0x0400 | on |
| Notes | 25561 | 0x0800 | on |
| Display as printed | 25562 | 0x1000, **inverted** (clear = on) | on |
| Outline buttons | 25563 | 0x4000 | on |
| System font in draft mode | 25560 | 0x8000 | off |

- Bit 0x0020 is kept as it was; it is not in the dialog (probably Show
  Power Fields, which has its own View menu command 0x10a, and Hide Ruler,
  0xcf).
- Beside the boxes: **Clean Screen Options…** (dialog 3042, bits in
  `CleanOpts`, default 0x40): Title bar, Menu, SmartIcons, Status bar,
  Vertical scroll bar, Horizontal scroll bar, Return icon (checked = kept
  shown; the default keeps only the Return icon; choosing no title bar
  forces no menu). And **Custom view: N %** (25301), limited to **10 to 400**,
  stored in `viewlevel`, default **91**.

## 4. View modes

Commands (R, hints): Full Page 0xc9 ("Display the current document in full
page view."), Custom 0xca, Standard 0xcb ("Windows standard view"),
Enlarged 0xcc ("enlarged 200%"), Facing Pages 0xd4 ("Display two facing
pages side by side."), Layout 0xd2 ("with all pictures, tables, headers,
footers, and footnotes"), Draft 0xd3 ("without page breaks, headers,
footers, and footnotes"), Outline 0xda. The menu text of Custom is "Custom
%d%%". Evidence: D `1080:0060`, `1080:01cd`, `1080:0236`, `1080:0291`,
`1050:034a` (initial view).

- **Zoom is held as 144000 divided by the percent** (the state word is a
  scale in 1/1440 inch units): **Standard 100 % (value 1440), Enlarged
  200 % (720), Custom = the `viewlevel` percent (value 144000/percent,
  default 91 %, range 10 to 400 from the dialog)**. Full Page computes the
  scale that fits the whole page into the window (the smaller of the
  height and width ratios, 20 units of margin kept), capped at 0x7fff; a
  fall-back of 0x630 (about 91 %) is used on an invalid view. (high)
- **Full Page is a toggle.** Choosing Full Page while in it returns to the
  view used before (kept in `0x2a3d`); the other three set the scale and
  clear the Full Page state. (high)
- **Opening view** (`initview`, read at start-up; a document opens in it
  when no other is given): 3 = Standard (the value used when the key is
  anything but 2 or 3), 4 = Enlarged, 5 = Facing, 6 = Full Page, anything
  else (the code default 2) = Custom at `viewlevel` %. So the factory view
  is Custom 91 %. (high)
- **Facing Pages** is a **temporary two-window mode**: command 0xd4 creates
  a second page window beside the first and runs a small modal loop (a
  dialog 1140 "Facing Pages" with only Cancel); the page-up/page-down
  commands move the spread, and leaving restores the single page and the
  earlier zoom (`1100:004a`). The left window shows page N−1 and the right
  page N (N kept in `0x297a`). Starting from an odd page n puts n on the
  right; from an even page n puts n on the left. **Stepping is by 2 pages
  (3, 5, 7…), left even and right odd.** Starting from page 1 the code
  forces the first spread to pages 1 and 2 (N=2), and the next step goes
  to 3, giving (2,3); a branch that paints a blank left window when N=1
  exists, but is overwritten on entry. So whether page 1 stands alone on the
  right is **not settled** (low): verify in the VM (open a 3-page document,
  Facing Pages, step). A conventional pairing (page 1 alone on the right,
  then 2|3, 4|5) is the safe choice.
- **Draft mode** (bit 0x10 of the mode word; Outline is 0x30; Layout is
  0x01). It is the text of the main flow only: the draw routine (`1108:0166`)
  walks the line list of the flow and the notes, with none of the page
  frame, margins, column guides, frames, headers/footers or footnotes that
  the Layout draw (`1108:02a3`) paints. Whether tables and pictures are
  drawn in it is not shown (medium-low: the hint says "without page breaks,
  headers, footers, and footnotes" and does not name them). Text is
  re-wrapped at the window width at the chosen zoom; the box "System font
  in draft mode" uses the Windows system font instead of the document's
  fonts. Factory default mode: Layout (`DefOptions` bit 1 set).

## 5. Special characters, Protected Text, No Hyphenation

- **Insert › Special Character… does not exist in Ami Pro 3.1.** The full
  menu dump (R, menu 1003) has Edit › Insert: Bullet…, Date/Time…, Note,
  Merge Field…, Doc Info Field…, Glossary Record…, New OLE Object…, Power
  Fields; Edit › Paste Special… (0x24c) is OLE; Text › Special Effects…
  (0x102) is a text-enhancement dialog ("Apply special text enhancements
  to selected text."). Symbols were typed with the Windows Alt+number codes
  or inserted as a bullet. A character picker here is an addition. (high)
- **Edit › Mark Text** holds Index Entry… (0xb4), TOC Entry… (0x343),
  Glossary… (0xc7), Revision Insertion (0xbb), **Protected Text (0xbc)**
  and **No Hyphenation (0xbd)**.
- **They are text attributes** like Bold: `12e0:15fa` takes the command id
  and toggles one bit of the attribute word at the selection (0x0002 =
  No Hyphenation, 0x8000 = Protected Text; Bold 0x04, Italic 0x08,
  Underline 0x10, Word Underline 0x20). They apply to the selection, or
  switch the typing attribute at the caret (on if the state at the caret is
  off, off if on). Hints: "Protect text from being easily accessed with the
  arrow keys or the mouse." and "Prevent selected text from being
  hyphenated." (high)
- **SAM tags** (already in `SAM_FORMAT_NOTES.md`): `<+*>`…`<-*>` Protected,
  `<+.>`…`<-.>` No hyphenation. They are not picked up by Fast Format (§2).

## 6. File › Revert to Saved (command 0x69)

- **It exists** in the File menu ("&Revert to Saved", between Save As Master
  and Import Text). Hint: "Return to the last saved version of the current
  document." (R, high)
- **It asks first.** `1040:121d` shows a confirmation with string 3036:
  "This will undo all the changes you have made since you last saved. Are
  you sure?" (OK/Cancel); on OK it first sends the close-modeless message
  when a modeless box is up, then reloads the file from disk
  (`1040:0d36`, or `1040:1155` when the document is an unmodified
  file-based one: both replace the open document in its window). Cancel does
  nothing. (high for the question, medium for the details of the reload;
  whether the item is dimmed for an unmodified or unnamed document is not
  traced, assume yes)

## 7. Page › Ruler and Page › Insert Page Layout

Hints (R): 0x173 "Insert a ruler into the current document."; 0x174 "Remove
an inserted ruler from the current document."; 0x177 "Insert a page layout
into the current document and specify the layout settings."; 0x175 "Remove
an inserted page layout from the current document."; 0x176 "Revert to the
standard page layout in the current document." Also 0x167 Line Numbering…
("Number the lines of the pages in the current document.", handler
`15c0:0000`) and 0x160 Modify Page Layout… ("Specify margins, columns, page
size, and orientation for the current document.").

- **Ruler › Insert (0x173), `1170:0245`.** Puts a ruler (the paragraph's
  own tab ruler, the `<:R…>` code) at the caret's paragraph, or each
  paragraph of the selection. A new ruler starts from the tabs of the
  paragraph's current ruler (an existing inserted ruler, else the page
  layout's); the ruler on screen then edits it.
- **Ruler › Remove (0x174).** Same handler with a different mode: removes
  the inserted ruler from the paragraph(s), so the paragraph falls back to
  its page layout's tabs.
- **Insert Page Layout › Insert (0x177), `12c8:0000` mode 1.** Adds a layout
  change at the caret's page (a copy of the layout in force, named from
  string 1172 "New"), then **opens Modify Page Layout (0x160) at once**; if
  that is cancelled while still flagged new the insertion is dropped. From
  the caret on, the pages use the new layout.
- **Insert Page Layout › Remove (0x175), mode 2.** Removes the inserted
  layout that governs the caret's page (the one that starts at or before
  it); nothing changes when the page uses only the standard layout. The
  following pages return to the layout before it.
- **Insert Page Layout › Revert (0x176), mode 3.** Inserts a change that
  returns the pages from the caret on to the document's standard
  (first) layout: it finds the layout record named like the standard layout
  in the layout list and re-applies it at the caret. The item is only
  meaningful after an inserted layout. (medium)

## Decisions this suggests

- Undo levels: offer Off or 1 to 4 as Ami Pro does (default 1), but keep the 100-step history as an improvement; a Preferences page with "Undo levels" is cosmetic.
- Recent files: 0 to 5 entries, default 4, most recent first, in a settings file; one `LastOpen`-like list.
- Auto backup: copy the old file to a Backup folder (default beside the file as `NAME.SAM.bak`, already the Save rule) before the save; the name change `.bak` is our choice.
- Timed save: minutes 0 to 99 (0 = 1), as the normal Save of a named, modified document (the plan's crash-recovery file is an improvement, not Ami Pro).
- Default style sheet for New: one setting (`defstyle`, default `_DEFAULT.STY`), chosen from the style-sheet list; New offers the same list with "List by description" and Preview.
- Fast Format: a toggle mode with a brush pointer; first pick-up copies the style and paragraph format and the text attributes; ask "Only text font and attributes / Only paragraph style" only when nothing, or the whole paragraph, is selected; stays on until chosen again; do not copy Protected or No hyphenation.
- View Preferences: the 13 boxes and defaults of the table in §3 (Display as printed, Margins in color, Pictures, Table gridlines, Notes, Outline buttons and the horizontal scroll bar on by default); Custom view 10 to 400, default 91.
- Zoom presets: Standard 100 %, Enlarged 200 %, Custom N % (10–400, default 91), Full Page fits the whole page with a small margin; Full Page toggles back to the previous view.
- Facing Pages: pair even-left, odd-right; show page 1 alone on the right (conventional) unless the VM check shows otherwise; keep it a view of the continuous workspace, not a modal loop.
- Draft mode: main-flow text only at window width, no frames, headers, footers, footnotes or page frame; "System font in draft mode" as a View Preferences option.
- Special Character… was not in Ami Pro 3.1: add it as an improvement (recent characters), not as a parity item.
- Protected Text and No Hyphenation: plain toggled text attributes (`<+*>` and `<+.>`) on the selection or at the caret, same machinery as Bold.
- Revert to Saved: exists; always asks "This will undo all the changes you have made since you last saved. Are you sure?"; reload from disk as one undoable replace (our improvement) or clear history.
- Ruler Insert/Remove: add or drop the paragraph's own ruler (`<:R…>`), starting from the current tabs; Page Layout Insert opens Modify Page Layout at once and cancelling drops it; Remove drops the governing inserted layout; Revert re-applies the standard layout from the caret on.

## 8. Page › Line Numbering (command 0x167; CM7)

Hint (R, string 359): "Number the lines of the pages in the current document." Read
2026-11-04. Nothing here was run in the VM.

**Where the code is (D).** The command handler `15c0:0000` of `AMIPRO.EXE` is
only a shell: it opens the dialog (template 1130 of `AMIPROUI.DLL`, procedure
`LINENUMPROC` `1368:0000`, its OK routine `1368:0295`), compares the answer
with the document's current values, and on a change marks the document
modified, then repaginates (see "Effect of OK"). The drawing routine is
`1178:0000`, the per-page line counter `1068:353a`, the reader (inside the
SAM reader) `1058:06cc`, the writer `11c8:14e0` and `11c8:1c7b`.

### 8.1 The dialog (R, template 1130, "Line Numbering", 196x96 dialog units)

| Control (resource id) | Type | Text / content |
|---|---|---|
| 25975 | combo box, drop-down list, top left, no caption | three entries from strings 2229-2231: "None", "Number all lines", "Number text lines" |
| 101 | group | "Options" |
| 25450 | radio | "&Every line" |
| 25451 | radio | "Every &other line" |
| 25452 | radio | "Every &fifth line" |
| 25453 + 25300 | radio + edit + static | "Every " [number box] "lines" |
| 25551 | check box | "&Reset each page" |
| 25700 + 25350 | static + list box | "&Based on:" + the document's styles |
| 1, 2 | buttons | OK, Cancel |

Only the list and its caption react to the combo (below); the radios, the
check box and the number box stay enabled whatever the combo says, so a user
can set them while "None" is chosen and the values are kept. (D, high)

Behaviour (D, high unless said):

- **Combo.** Index 0 "None" = numbering off, index 1 "Number all lines",
  index 2 "Number text lines". Choosing "Number text lines" disables the
  list and its "Based on" caption; the other two leave them enabled.
- **Radios.** Exactly one is on. On opening the dialog picks "Every line" if
  its bit is set, else "Every other", else "Every fifth", else the "Every N"
  radio (so a value with none of the three bits, such as `0`, opens on
  "Every N"). Typing in the number box selects the "Every N" radio (medium: the
  call's argument order was read, not run).
- **Number box.** Whole number 1 to 99; the box is checked as the user types
  (0, empty and above 99 are marked invalid and OK is refused). It always
  shows the stored third value, also when another radio is on.
- **Reset each page.** Starts at 1 on every page when on; off = the count
  carries over from the page before.
- **Based on.** Lists the document's paragraph styles; the stored style name
  is selected on opening. Style names are held in a 14 byte field (13
  characters and the end mark; the same limit as the importer's cut of style
  names, `docs/research/IMPORT_NOTES.md`).
- **OK/Cancel.** Cancel changes nothing. OK returns three things: a flag
  byte, the chosen style's name (empty selection = the list gives none and
  the dialog refuses to close), and the number box's value.

**Defaults when the dialog opens on a new document (D, high):** combo "None",
"Every line", Reset each page off, Based on "Body Text" (string 1285),
number box 1.

### 8.2 `[lnopts]` in the SAM (D for the code, I for the corpus)

Three lines, each written as the section's usual `<tab>value` line, always
present in every document and style sheet Ami Pro writes (it sits after
the macro/auto sections and before `[docopts]`):

1. **Flag byte, decimal** (0 to 255). Bits (set by `1368:0295`):

   | Bit | Value | Meaning |
   |---|---|---|
   | 0 | 0x01 | combo = "Number all lines" |
   | 1 | 0x02 | "Every line" |
   | 2 | 0x04 | "Every other line" |
   | 3 | 0x08 | "Every fifth line" |
   | 4 | 0x10 | "Reset each page" |
   | 5 | 0x20 | "Every N lines" (N is line 3) |
   | 6 | 0x40 | never written: in memory it means "no `[lnopts]` read yet" |
   | 7 | 0x80 | combo = "Number text lines" |

   The dialog writes exactly one of 0x02/0x04/0x08/0x20, plus 0x10 if reset,
   plus 0x01 or 0x80 or neither. **Numbering is on iff bit 0 or bit 7 is
   set** (the drawing and the page counter test `& 0x81`). Examples:
   `2` = off (the default), `3` = all lines, every line, `0x82` (130) = text
   lines, every line; `0xa0` (160) = text lines, every N.
2. **Style name** ("Based on"), written with the usual `<…>` escapes.
3. **N, decimal** (the number box). Ami Pro writes it always; the reader
   treats a missing third line (the next line starts with `[`) as 1.

**Default written when off (I, high):** `2`, `Body Text`, `1`. A new
document gets this by default; a file lacking the section gets it too (the
reader's "not read" marker 0x40 is replaced by it). Turning numbering off
again keeps whatever radios, reset and N were set, so a file can carry
e.g. `8`/`Body Text`/`1` or `0x92` with numbering off or on.

**Belongs to the document, not the page layout (D, high).** The three
values are document-wide globals (one set per open document), saved in
their own section next to `[docopts]`, not in any `[layout]` or inserted
layout. Style sheets carry the same section but only copy it as part of the
file header; the values of the document win. Ami Pro's internal clipboard
and save stream also holds them as a record (code 0x12: flag byte, count if
bit 5, style name if bit 0), irrelevant to SAM.

**Corpora (aggregate counts):**

| Corpus | Files read | Default `2 / Body Text / 1` | Default but no third line | Non-default | Numbering on |
|---|---|---|---|---|---|
| User's SAM corpus | 423 | 373 | 47 | 1 (value 3: all lines, every line) | 1 |
| ISO corpus (160 SAM + 63 STY) | 223 | 194 | 2 | 27 (`0 / Body Text / 0`, all in one sample folder; numbering off, "Every N" radio with N = 0) | 0 |
| VM outputs (`vm-lab/OUT`) | 417 | 417 | 0 | 0 | 0 |

One further file of the user's corpus has no `[lnopts]` at all, and one is
password protected (§9). So only **1 of 423** user files and **0 of 640**
others carry numbering that is on. The 27 `0 / … / 0` files have bytes
that Ami Pro's own dialog cannot produce (N = 0 is refused), so they were
not saved by the dialog (medium); the reader accepts them.

### 8.3 What is drawn, and where (D, medium; points marked "open" need the VM)

- **When.** Only when bit 0 or bit 7 is set, in the page painter (`1108:02a3`)
  after the page's text blocks, on screen and in the pass that records a
  page for the printer, so the numbers print (D, medium-high). Whether the
  Draft view shows them is open.
- **Text of the number.** Decimal, then one space (the format is a long
  integer plus a space), in the font of the "Based on" style when the combo is
  "Number all lines" and of the style "Body Text" when it is "Number text
  lines" (the stored name is not consulted; also the fall back when the
  stored style no longer exists). Right-aligned in a box that reaches from
  the page's left edge to the left margin, so the numbers stand in the left
  margin close to the text (medium; the print path uses a different call
  whose alignment was not confirmed - open). Not user-settable: the dialog
  has no margin or position choice, no starting number, no separator.
- **Which numbers.** With count `k` of the line: every line = all; every
  other = odd `k` (1, 3, 5, …); every fifth = multiples of 5; every N =
  multiples of N (so N = 3 prints 3, 6, 9, not 1). The count is a 32-bit
  integer. The test uses the count after counting the line, starting at 1.
- **Number all lines (bit 0).** Ignores the text. From the top margin to the
  bottom margin of the page it steps the line pitch of the "Based on" style
  (its line height with its spacing setting) and numbers every slot, empty
  or not, like numbered legal paper. Unless "Reset each page", page 1
  starts at 1 and each later page starts at the previous pages' slot count
  plus 1, each page contributing the whole number of pitches in its own text
  height (so odd/even layouts count differently).
- **Number text lines (bit 7).** Numbers only lines of the page's text blocks of
  kind 1 (the flow text; the type byte is the same one the layout uses for
  text), skipping a line when a predicate (`1178:060e`, probably "line of a
  table row or similar structure") says so or its flags have bit 4 or bit 6
  set. The number sits at the line's own height. The layout keeps a
  running 32-bit count per page (`1068:353a`) so the next page begins where
  this one ended, unless "Reset each page". In this mode the line-building
  routine (`10d0:0d82`) also sets a mark on the lines of kind-1 blocks
  (open: which lines exactly).
- **Which lines count (open).** Not settled from the code: blank lines
  (probably counted, they are lines), lines in tables, in frames of text,
  headers and footers, footnotes, and lines of an inserted layout with other
  margins. The loops visit the page's block chain, so header, footer and
  footnote text, which are not in it, probably are not numbered (low).
- **Effect of OK.** If the document is in a page view, a change of the
  "text lines" bit (bit 7) repaginates (and recounts, unless Reset); any other
  change only repaints. In every case a changed value marks the document
  modified.

### 8.4 Fixtures that would settle the open points

One property at a time, from the same three-page body (about 120 short
paragraphs of Body Text, a page break by flow), screen shot after, and a
print to file or PDF of LN2 and LN3 for the placement.

| Fixture | Setting | Settles |
|---|---|---|
| LN1.SAM | numbering off, default dialog saved | the written bytes of an untouched dialog (expected `2 / Body Text / 1`) |
| LN2.SAM | Number all lines, Every line | the value 3; slot count per page; number position and font |
| LN3.SAM | Number text lines, Every line | the value 130; font (Body Text); height; position |
| LN4.SAM | text lines, Every other line | value 132; odd numbers only |
| LN5.SAM | text lines, Every fifth line | value 136 |
| LN6.SAM | text lines, Every 7 lines | value 160 and N = 7; 7, 14, … |
| LN7.SAM | text lines, Reset each page | value 146; restart on each page |
| LN8.SAM | all lines, Reset each page | value 19; page 2 start when not reset (LN2 against LN8) |
| LN9.SAM | text lines; blank paragraphs and an empty last line | whether blank lines count |
| LN10.SAM | text lines; a table spanning a page | table rows counted or not |
| LN11.SAM | text lines; a text frame, a picture frame | frames counted or not |
| LN12.SAM | text lines; header, footer, a footnote | headers, footers, footnotes counted or not |
| LN13.SAM | all lines, Based on a 24-point style | the pitch and the font follow the style |
| LN14.SAM | text lines, Based on the same 24-point style | the style is ignored in text mode |
| LN15.SAM | text lines; facing pages | number side on left and right pages |
| LN16.SAM | text lines; an inserted layout with other margins mid-document | continuity, position at the layout's margin |
| LN17.SAM | None after setting Every fifth + Reset | the values kept when off (expected 24, not 2) |
| LN18.SAM | N typed 100, 0 and 99 | the limit (dialog behaviour; 99 saved as 99) |
| LN19.SAM | Based on a 13-character style name | the name is kept whole |
| LN20.SAM | text lines; two columns | numbering per column or per page |

20 fixtures.

### 8.5 What the VM showed (2026-11-05; fixtures LN1–LN10, LN13–LN15, LN17, LN18, LN20)

Made in Ami Pro 3.1 from a synthetic three-page body (`IN/LNB.SAM`: 110 Body
Text paragraphs, every tenth three lines long) and one content page made
in Ami Pro (LN9, LN10: two empty paragraphs, a 2 × 3 page table with a
three-line cell, a Wrap around text frame with text on both sides, header,
footer, a footnote). Each saved under `OUT/`, printed to `OUT/NAME.PS` on
the PS File printer and read with `pdftotext -bbox` (VM, high).

| Fixture | Dialog | `[lnopts]` written | Seen |
|---|---|---|---|
| LN1 | untouched, OK | `2 / Body Text / 1` | nothing drawn |
| LN2 | Number all lines | `3` | 45 slots a page, 1–45, 46–90, 91–135; page 3 numbered to its bottom margin past its 42 lines |
| LN3 | Number text lines | `130` | 1–45, 46–90, 91–132: lines only |
| LN4 | text, Every other | `132` | odd counts only (1, 3, …; page 2 from 47) |
| LN5 | text, Every fifth | `136` | 5, 10, … |
| LN6 | text, Every 7 | `160 / … / 7` | 7, 14, …; typing in the box selects the radio |
| LN7 | text, Reset | `146` | each page from 1 |
| LN8 | all, Reset | `19` | each page 1–45 |
| LN17 | None after Every fifth + Reset | `24` | values kept while off |
| LN18 | Every N: 100 and 0 refused (OK does nothing, no message); 99 taken | `176 / … / 99` | |
| LN9 | text lines on the content page | `130` | numbered: text lines of the main text only. **Not counted:** empty paragraphs, table cells (also a cell's three lines), frame text, header, footer, footnote. Lines on both sides of the frame each count, in flow order (left side 13–18, right side 19–24), all numbered in the left margin, the right side's numbers drawn over the left side's at the same heights |
| LN10 | all lines on the content page | `3` | 45 slots at Body Text's pitch whatever the page holds (table rows, the frame, blank space) |
| LN13 | all lines, Based on Title (Arial 18 bold, 144/72 twips above/below) | `3 / Title / 1` | 30 slots at the style's line pitch (21.35 pt), spacing above and below not added; numbers in Arial bold 18 |
| LN14 | text lines, Based on Title kept | `130 / Title / 1` | numbers in Body Text (Times 12): the stored style is ignored |
| LN15 | text lines, left pages with a 2 in. left margin | `130` | page 2's numbers end at its own left margin (2 in.) |
| LN20 | text lines, two columns, a Title line first | `130` | only the first column's lines are numbered and counted (page 2 starts at 45, page 1's first column having 44 lines); the number beside the Title line is Times 12 on the Title line's baseline |

- **Position.** The number and one space end at the left margin of the
  page's own layout: right edge of the digits one space width (3 pt in
  Times 12) before the margin. Text-lines numbers sit on their line's
  baseline (0.5 pt lower in the print, ignored); all-lines slots start at
  the top margin.
- **Draft view** shows no numbers (Layout mode only, as far as seen).
- Not made: LN11, LN12 (covered by LN9), LN16 (inserted layout: the rule
  of LN15, the page's own margin, is assumed), LN19 (the 13-character
  name: the dialog's field, as the importer's style-name cut).

## 9. Password-protected files (CM5)

Read 2026-11-04. Nothing here was run in the VM. Strings (R): "Password
Protect" dialogs 1042 and 1043 of `AMIPROUI.DLL`, messages 3147, 3148, 3179,
3199, 3224.

**Where the code is (D).** Save As routine `11b0:1cab` (it asks, verifies,
keeps the password in two global copies); the writer `11c8:1c7b`; the open
paths `1058:017f` and `1058:3753`; the first-line probes `1388:0211` and
`15a0:0000`; the coder in segment `1668` (`1668:0000` read a line,
`1668:0206` write bytes, `1668:032a` the model update, `1668:03bf` finish).
The password dialog procedure is `PASSWORDDLGPROC` `1190:0e5d`.

### 9.1 The user's side (R, D)

- **File › Save As** has a check box "&Password protect" (25551, template
  1021; hidden when a file type other than an Ami Pro document is chosen, in
  the same branch that hides the "Ami Pro 1.2 format" box; medium). With it
  on, OK opens dialog 1042 "Password Protect" (static "&Enter password:",
  one edit, OK/Cancel), then dialog 1043 "Password Protect" ("&Verify
  password:", the same edit, and the warning "WARNING - the contents of
  protected files cannot be recovered if the password is lost"). The two
  strings are compared byte for byte; a difference gives message 3179
  "Invalid Password Verification.  Please reenter password and reverify."
  and the Save As is not done. Cancel in either dialog cancels the save.
- **The edit** (25300, style `ES_PASSWORD`, no `ES_UPPERCASE`, no OEM
  conversion) shows the typed characters hidden. Its text limit is **14
  characters** (a 16 byte buffer). The password is **case-sensitive**: it is
  never upper-cased (the file name is, by a separate call), compared with
  an ordinary byte compare, and used as typed. An empty password is not
  refused by the dialog (medium: no length check was seen; an empty password
  makes the model priming a no-op) - open.
- **File › Open** of a file whose first line is `[encrypt]` asks for the
  password with dialog 1042 only (no verify). Cancel abandons the open
  silently. A wrong password gives message 3147 "Invalid Password.
  Encrypted file will not open with incorrect password." and the file is
  closed; there is one attempt (no retry loop). The same prompt-and-check
  runs in both open routines (`1058:017f` the normal Open, `1058:3753` the
  variant that falls back to another reader on failure).
- **Other commands** that probe a file's first line (insert-file style
  readers, glossary files, master and merge sources) refuse `[encrypt]` with
  3148 "Cannot do that command to an encrypted file.  De-Encrypt and try
  again." (the repo's glossary reader already uses this text) or 3199 "File
  is password protected.  Remove password to open file." (call site not
  found; medium that it is the same family). Either way Ami Pro does not
  ask for a password there; the user opens and re-saves without one.
- **Save of a protected document.** The password given at open is kept in
  two copies (the "current" one and the one the file was opened with). On
  Save As, if the document was opened protected and the box was not ticked:
  equal copies mean the file is written protected again with the same
  password; different copies give 3224 "Internal password failed to verify.
  Please reenter password and reverify." and the box is forced on (medium:
  whether the box starts ticked for a protected document, and so how a
  user removes protection, is open - fixtures PWSAV, PWNOP). Failed saves
  restore the old password.

### 9.2 The file (D, high for the layout; medium for exact constants)

```
[encrypt] CR LF            plain text, 11 bytes
<coded stream> ... end of file
```

The whole rest of the file is one coded stream; nothing after the first
line is readable text. The stream **decodes to the normal SAM text, with
one extra first line**: the writer writes (through the same coder) a
check line - the tab-led line `<tab>PASSWORD<space>`, then the decimal
character code of each password byte run together (for "AB": `AB 6566`;
a byte above 127 is a signed value, so negative: `é` = -23) - and then the
usual `[ver]`, `4`, `[sty]` ... sections. The line writer's usual
escapes apply to the check line (`<` doubled, control and high bytes as
`<…>`), lines end CR LF. What is scrambled is therefore **everything
except the `[encrypt]` line, including `[ver]`**; there is no plain header
or size field.

**The coder (D).** It is a splay-tree (self-adjusting, adaptive prefix)
coder of the kind D. W. Jones described in 1988 for data compression, not a
cipher with a key stream: it also **compresses**. The model:

- 257 symbols: the byte values 0 to 255 and an end symbol 256. A complete
  binary tree over heap-numbered nodes: node `n` has the children `2n`
  (code bit 0) and `2n+1` (bit 1), parent `n/2`, the root is node 1; the
  leaf of symbol `s` is node `s + 259`. Tables: left and right children for
  nodes 0 to 258, parent for nodes 0 to 517, filled that way at the start.
- After every symbol, coded or decoded, the model **splays** the symbol's
  leaf: going up from the leaf two levels at a time, each step swaps the
  grandparent's child on the far side of the path with the leaf's own
  parent branch (the usual semi-rotation that halves the path), until the
  parent is the root. Frequent symbols therefore get short codes.
- Bits are packed **most significant bit first**; a symbol's bits run from
  the root to its leaf. Decoding reads a bit at a time, walks left or right,
  and at a leaf outputs the symbol (the byte is the leaf number minus 3,
  low byte) and starts again at the root. The decompiled leaf test is
  "node number above 257", which makes node 258 an extra leaf that decodes
  as byte 0xFF without being a real symbol (medium: replicate as read, the
  VM fixtures decide).
- A line read stops at a decoded LF (byte 0x0A) or the end symbol (the end
  of the file's data); the reader then strips the line end like a plain
  line.

**Key derivation (D, high).** There is none beyond priming: before the first
symbol both sides splay the model once for each byte of the password, in
order (each byte as a signed value). This is the whole key. No salt, no
iteration, no checksum, no per-block restart: one stream from the first coded
bit. Two different passwords give two different trees, and so different
codes for the same text; a wrong password decodes garbage from the first
bit.

**Check.** The first decoded line must equal `PASSWORD<space>codes` (the
form written now) **or** the bare password (accepted for older files), by
a case-sensitive compare, and the next line must be `[ver]`; otherwise 3147.
This is the only password check, and a wrong password almost certainly fails
it.

**Writing.** After the last line the writer codes the end symbol, then keeps
coding space characters until the output position advances (the last partial
byte is flushed); a decoder stops at the end symbol and ignores the padding.
(High that padding exists, medium on its exact length.)

**Internal flags (D).** Bit 0x80 of the document flag word means "lines go
through the coder", 0x100 "the model needs setting up on the next call" (the
setup and password priming run lazily on the first line), 0x200 "the file was
opened protected and verified". Bit 0x180 is set by the open and the save
paths.

### 9.3 Can it be reproduced? (assessment)

- **Reading: yes, with the right password.** The scheme is a compression
  coder primed by the password, fully described by the tables above; nothing
  is hidden or machine-bound. Confidence **medium-high**: the family and the
  layout are certain; two details need the VM (the leaf boundary at node
  258, and the sign handling of bytes above 127). A scratch implementation of
  this reading round-trips its own output but could not be checked against
  the real protected file in the user's corpus because its password is
  unknown (a handful of trivial guesses decoded to noise). The check line
  makes a wrong password detectable at once, so no whole-file test is needed.
- **Writing: yes, in principle, same confidence, slightly lower.** The coder
  is symmetrical (encode walks leaf to root and emits the bits root first,
  then splays). Ami Pro reading the result is the test: the fixtures below
  must read back in Ami Pro, and Ami Pro's own files must decode to the
  plain SAM text.
- **Not a strong scheme.** Anyone with a few known bytes of text can mount a
  dictionary attack cheaply (the check line is a known plaintext), and the
  limit of 14 characters is small. Offering "write protected" is a parity
  item; if the product adds it, read it as obfuscation, not security. The
  app already refuses files that start with `[encrypt]`
  (`crates/ami-formats`, `ImportError::PasswordProtected`).

### 9.4 Corpora (aggregate counts)

| Corpus | Files read | Start with `[encrypt]` |
|---|---|---|
| User's SAM corpus | 423 | **1** (4366 bytes; 36 zero bytes in the coded part, as expected of coded data) |
| ISO corpus (SAM + STY) | 223 | 0 |
| VM outputs | 417 | 0 |

### 9.5 Fixtures that would settle it

All with the same body (one paragraph of Body Text, "Hello"), saved with
File › Save As, Password protect ticked, unless said. The text stays the
same so any difference is the password.

| Fixture | Setting | Settles |
|---|---|---|
| PWPLN.SAM | no password | the plain bytes to compare the decoded files against |
| PWA.SAM | password `A` | the decoder: the check line `A 65`, then the plain text; first coded bytes |
| PWB.SAM | password `B` | different priming gives different bytes |
| PWAB.SAM | password `AB` | the order of the priming (against `BA`) and two-byte codes |
| PWBA.SAM | password `BA` | the order of the priming |
| PWLC.SAM | password `a` | case sensitivity (the check line `a 97`) |
| PWA2.SAM | `A` saved a second time | determinism of the bytes (any date field aside) |
| PW14.SAM | a 14-character password | the full limit; the check line length |
| PW15.SAM | 15 characters typed | the edit stops at 14 |
| PWSP.SAM | `A B` (a space) | spaces in passwords and in the check line |
| PWLT.SAM | `<A` | the line writer's escape in the check line |
| PWHI.SAM | a password with `é` (Alt+0233) | signed bytes above 127 in priming and the check line |
| PWEMP.SAM | an empty password (OK on an empty edit) | whether it is accepted and what is written |
| PWBIG.SAM | `A`, a document of 5000 characters | long streams, tree adaptation, the end padding |
| PWSAV.SAM | open PWA.SAM, type nothing, File › Save | the file stays protected with the same password |
| PWNOP.SAM | open PWA.SAM, Save As with the box unticked | whether protection is kept or removed |
| PWCHG.SAM | open PWA.SAM, Save As with password `B` | changing the password |
| (behaviour) | PWA.SAM opened with `a`, with Cancel | messages 3147 and silent cancel; one attempt |
| (behaviour) | Verify typed differently | message 3179 and no file |
| (behaviour) | Insert › File, Merge data and the glossary on PWA.SAM | 3148 and 3199 and the dialogs |

17 file fixtures and 3 behaviour checks (20 in all).

### 9.6 What the VM showed (2026-11-05; fixtures PWPLN, PWA, PWAB, PWBA, PWLT, PW14, PWHI, PWEMP, PWNOP, PWSAV, IN/PWSV2)

A one-paragraph document ("Hello") saved by File › Save As with Password
protect ticked; decoded on the host with a scratch decoder of §9.2's model
(VM and the bytes, high).

- **The coder is as mapped**, with internal nodes 1–257 and leaves from
  258 (node 258 the stray 0xFF leaf), symbol `s` at leaf `s + 259`, end
  symbol 256, bits most significant first. PWA decodes to `\tA 65`, then
  PWPLN's text byte for byte (only `[desc]`'s save time differs), and a
  re-encoding of that text gives PWA's bytes exactly.
- **What is coded:** from after `[encrypt]` CR LF to just before the body's
  closing `>` line. Then the end symbol, then the codes of spaces cut at
  the next byte boundary: 1 to 8 bits, a whole byte when the end symbol
  ends on one (PWA 7, PWAB 8, PW14 2). **The rest is plain:** `>`, the
  pictures' bytes, an empty line, and `[Embedded]` with its offsets
  absolute in the protected file (PWA's trailer 2661 is where `[Embedded]`
  stands in it).
- **Priming** splays each password byte unsigned (PWHI: é primes as 233;
  -23 decodes noise). The check line is the password written with the
  SAM escapes, a space, then each byte's decimal code unsigned: `\tAB
  6566`, `\tBA 6665`, `\t<<A 6065`, `\t<\i> 233`, `\tABCDEFGHIJKLMN
  65…78`; an empty password is accepted and gives `\t ` (PWEMP). Case
  matters (PWA with `a` decodes noise).
- **Open:** a protected file asks "Enter password:" only; a wrong one gives
  "Invalid Password.  Encrypted file will not open with incorrect
  password." and nothing opens (one attempt).
- **Saving:** File › Save of a file opened protected writes it protected
  with the same password (PWSV2). Save As starts with Password protect
  unticked even for a protected document, and saving unticked writes a
  plain file, whether the protection was set in this session (PWNOP) or
  the file was opened protected (PWSAV).
