# Original Ami Pro VM Lab

**Authorized 2026-10-11:** the agent operates the `Windows XP` VirtualBox VM
directly, including snapshots (see `AGENTS.md`). This file records how. Keep
the VM, its disks, files written in it, and screenshots of private content
outside Git.

## The guest

- Windows XP Professional SP3, 1152×864, VirtualBox 7.2.14 host with Guest
  Additions 7.2.6. No network adapters; clipboard and drag-and-drop disabled.
- Auto-logs in as `Administrator`. `VBoxManage guestcontrol` fails to
  authenticate with the password from the unattended-install floppy (it was
  probably changed), so it is not used.
- Ami Pro 3.1 runs from the attached ISO (`D:` = `AMIPRO31`):
  `D:\FROMROM\AMIPRO\AMIPRO.EXE`. Style sheets are read from
  `D:\FROMROM\AMIPRO\STYLES` and documents default to
  `C:\LOTSUITE\AMIPRO\DOCS`. Files opened from `D:` are read-only.
- A user-made global shared folder `XPShare` (`~/XPShare`) also exists; leave
  it alone.

## Idle lock (2026-10-12)

After about 15 idle minutes the guest showed the Windows welcome screen and
asked for the Administrator password, which is not known (only auto-logon
works). Turn off computer › Restart on the welcome screen recovers: auto-logon
signs in again, and the transient share stays mounted. To stop it recurring,
the guest now uses the "Always On" power scheme with the monitor never
turned off (`powercfg /setactive "Always On"`, `powercfg /change "Always On"
/monitor-timeout-ac 0`) and no screen saver (`ScreenSaveActive` 0). After a
restart, start Ami Pro again from the guest `cmd`:
`start D:\FROMROM\AMIPRO\AMIPRO.EXE`.

## After an Ami Pro fault (2026-10-13)

A file Ami Pro faults on (a General Protection Fault in `AMIPRO.EXE`, after
an "An error has occurred in your application" box) leaves Ami Pro unable
to create the temporary files it draws frame contents from: every later
document shows "Cannot create temporary file for frame" and loses its
pictures, and the fault repeats, until the guest restarts. Killing
`ntvdm.exe` is not enough. Close the fault box, then restart the guest
(Win+R, `shutdown -r -t 0`), wait for auto-logon, open `cmd` (Win+R,
`cmd`), and start Ami Pro again. Once a restart ended in a VirtualBox
"guru meditation": power the VM off and start it headless, choose Start
Windows Normally on the recovery screen, and add the transient share
again (Session, below). Bisect a faulting file with variants of the
original, one difference at a time; a variant that faults costs a
restart, so test the likeliest ones first.

## Baseline snapshot

`Clean baseline (before AmiPro-Linux lab)`, taken 2026-10-11 with the VM
powered off and nothing changed since the user's install. Restore it whenever
the guest state is doubtful:

```sh
VBoxManage snapshot "Windows XP" restore "Clean baseline (before AmiPro-Linux lab)"
```

## Session

```sh
VBoxManage startvm "Windows XP" --type headless
VBoxManage guestproperty wait "Windows XP" /VirtualBox/GuestInfo/OS/LoggedInUsers --timeout 240000
# Repeat the wait until the value is 1 (it reports 0 first).

# Transient share: exists only while the VM runs, so no permanent host share.
VBoxManage sharedfolder add "Windows XP" --name amilab \
  --hostpath "$PWD/../AmiPro-Reference/vm-lab" --transient --automount --auto-mount-point L:
```

The guest sees the share as `L:`. Use `L:\IN` for files going to Ami Pro and
`L:\OUT` for files Ami Pro writes. **Ami Pro is a 16-bit program and needs 8.3
names:** it rejected `L:\from-guest` as "not a directory".

To finish, close Ami Pro without saving reference documents, then
`VBoxManage controlvm "Windows XP" acpipowerbutton`.

## Input and observation

- `tools/vm/vmkey.py` sends keystrokes and takes screenshots, for example
  `tools/vm/vmkey.py combo:alt+f key:n wait:1.5 key:enter text:'Hello' shot:new`.
  Screenshots go to `../AmiPro-Reference/vm-lab/screens/`.
  `tools/vm/crop.sh` enlarges a region with ffmpeg.
- Num Lock must be on for `altcode:` (Alt+numpad) input:
  `VBoxManage controlvm "Windows XP" keyboardputscancode 45 c5`.
- `keyboardputstring` sent a typed backtick as an apostrophe. Use Alt codes
  for characters that matter.
- For shell work in the guest, open `cmd` (`combo:win+r text:cmd key:enter`)
  and redirect output to `L:\OUT\…` rather than reading it from screenshots.
- The caret is often invisible in screenshots. Read state from Ami Pro's
  status bar instead: paragraph style, typeface, point size, and page number,
  from left to right.
- `tools/vm/vmmouse.py` drives the mouse through the VirtualBox Python API
  (`vboxapi`), using screenshot pixel coordinates, for example
  `tools/vm/vmmouse.py click:370,370 shiftclick:540,430`. It is needed for
  multiple frame selection (Group) and for placing the caret. Prefer the
  keyboard where it works: menus and most dialog controls are keyboard
  accessible. In the New dialog, the style-sheet list responds to arrow and
  End keys but not to type-ahead.

## Pitfalls seen so far

- In Ami Pro dialogs, Alt+letter on a radio button only focuses it; press
  Space to select it. Mnemonics collide (Modify Frame Layout has two `L`
  and two `C` targets), so check a dialog screenshot before pressing OK, or
  move between radio buttons with the arrow keys.
- Ami Pro marks a file read-only while it is open, and Save As onto an
  existing name raises an overwrite prompt that swallows later keystrokes.
  Use a new name for every fixture, and confirm the file exists before
  closing the document.
- Keep one document open at a time. A failed Close leaves the old document
  active, and the next fixture's typing lands in it.
- Enter in a numeric dialog field (custom shadow depth) does not press OK,
  and a later Esc then cancels the whole dialog. Move focus away from the
  field before confirming, and check the saved file, not only the screenshot.
- Choosing an item in a drop-down list with Enter also closes the dialog
  with OK.
- In Modify Style, Alt+A opens Save As (New Style Name), not the Alignment
  panel: click the Modify radio buttons (Alignment at 338,283; Spacing at
  338,302) instead. Clicking into the Custom line-spacing box selects
  Custom; move focus by clicking the Above box instead.
- Spin-button arrows are not Tab stops. Colour palettes are: Left/Right and
  Home move the selected swatch.
- The first mouse click after leaving frame text with Esc is sometimes
  absorbed; check the caret before relying on it.
- The Tools menu does not open from the keyboard (Alt+T opens Text, Alt+O
  does nothing); click it with `vmmouse.py click:277,35`. Tables is its
  fourth item.
- Ctrl+Home did not move the caret after leaving frame text; click into the
  text instead. In the Page menu, Insert Page Layout is reached with
  `alt+p Down Right` and Page Numbering with four Downs from the top.
- Delete at an anchored frame's mark does nothing on its own; select text
  around it to delete the frame (Ami Pro then asks for confirmation).
- Exit and restart Ami Pro (`combo:alt+f key:x`, then
  `start D:\FROMROM\AMIPRO\AMIPRO.EXE` in the guest `cmd`) for a clean state.
- Go To (Ctrl+G) does nothing while the caret is in a header, footer, or
  floating header. Drag the vertical scroll thumb to the top with
  `vmmouse.py drag:1143,444,1143,100` and click into the text instead.
- Removing a page break or a floating header/footer works only with the
  caret at the mark (for a break at the end of a paragraph: Ctrl+Home, End).
  Elsewhere the dialog's Remove option is disabled.
- In the Breaks dialog, `alt+v` only focuses the "Vertically center" check
  box; press Space and check a screenshot (once it took a second Space).
- Closing a changed document asks to save; `key:n` answers No.
- File › New turns Run macro back on whenever the chosen sheet has a macro,
  so clear it after choosing the sheet and check a screenshot before OK
  (`_PROPOS2`, `_NEWSLT1`, `_INVOICE`, `_FAX3`, `_TITLE2`, and others run
  macros that ask for information; one stored typed keystrokes as the
  personal information in `C:\WINDOWS\AMIPRO2.INI`, which was then deleted
  to restore the guest). The list's order is the folder's file names
  sorted: click the list, Home, then Down as many times as the sheet's
  position.
- A fixture edited from a SAM with pictures must keep `[Embedded]`'s byte
  offsets right: its directory gives each picture's absolute offsets and
  the trailer the directory's own. Text removed before them shifts them,
  and Ami Pro then reports "Cannot create temporary file for frame"
  (without a fault; no restart needed).
- Check that a fixture name is free in `IN` and `OUT` before writing it
  (2026-11-05: a generator wrote over older inputs named FS1–FS3, not
  recorded anywhere).

## Style sheets (2026-10-13)

- Ami Pro's style-sheet path (Tools › User Setup › Paths › Style sheet) is
  now `L:\STY` (`../AmiPro-Reference/vm-lab/STY`), which holds copies of
  the ISO's shipped sheets plus sheets written by Ami Pro or by this
  project. File › New and Use Another Style Sheet list that folder; Save
  as a Style Sheet writes there. The ISO path was
  `d:\fromrom\amipro\styles`; put it back if `L:` is not shared.
- Save as a Style Sheet: `combo:alt+s key:v`. Its File name takes an 8.3
  name only (12 characters, no path: "Filename contains invalid
  characters"). With contents is on by default; `alt+w` focuses it and it
  takes two Spaces to clear (check a screenshot). Tools menu: click
  (277,35), User Setup (310,321), Paths… (716,296).
- Style Management is not reachable by its mnemonic from the keyboard:
  open the Style menu and click (215,167).
- In the New dialog, clear List by description (588,408) to see file
  names; click into the list, then End and Up to reach a sheet.

## Seeing more than one page

View › Facing Pages (`combo:alt+v`, four Downs, Enter) shows two pages side
by side, which is the quickest way to check where text and frames land at a
page end. It opens a small "Facing Pages" box; Esc returns to the normal
view. The 91% custom view shows about 87 pixels per inch, with the page top
at y = 87 when scrolled to the top.

## Keyboard routes in Ami Pro

- Create a frame with exact geometry: Frame › Create Frame (`alt+r c`), then
  `alt+w` width, `alt+h` height, `alt+d` down from top, `alt+i` in from left,
  in inches, and Enter. The new frame stays selected.
- Type into the selected frame: Enter, type, then Esc twice to return to the
  main text.
- Modify the selected frame: Frame › Modify Frame Layout (`alt+r m`).
- Select a frame later: Ctrl+G, then `alt+n space` (Next item, with Frame
  highlighted in the list) and Enter. Ctrl+H repeats the last Go To, which
  may be a page jump rather than a frame.
- Frame order: Frame › Bring to Front (`alt+r f`), Send to Back
  (`alt+r b`), Group (`alt+r g`, needs several frames selected with
  Shift+click).
- Import a picture into the selected frame: File › Import Picture
  (`alt+f i`), `alt+t End Up` for Windows Bitmap, `alt+n` and the path;
  `alt+c` toggles Copy image.
- Select a frame with the mouse: one click inside it (fixed or anchored)
  selects it; a drag from inside moves it (and its whole group).
- Placement in Modify Frame Layout: `alt+p space` With para above,
  `alt+f space` Flow with text, `alt+r space` Repeat all pages.
- Fixed header text: click in the top margin (about y = 135 at the top of
  page 1) and type.
- Breaks: Page › Breaks (`alt+p b`). Enter inserts a page break; `alt+c
  space` selects Insert column break.
- Floating header/footer: Page › Header/Footer (`alt+p h`), then `alt+f`
  for the Floating Header/Footer dialog. Choose Floating footer, Right
  pages, or Left pages by clicking their radio buttons (504,284; 584,323;
  584,342) and press Enter; the caret then sits in the floating header or
  footer on the next page.
- Tables: click Tools (277,35), then Tables (290,107), and press Enter for
  the default 5 × 5 (or type the column count, Tab, the row count). With a
  frame selected, the table goes into that frame.
- Special characters: `combo:ctrl+enter` hard return, `combo:ctrl+minus`
  discretionary hyphen, `combo:ctrl+space` no-break space.

## Fixture method

`tools/vm/frame-fixture.sh NAME "ACTIONS"` makes one F01-style frame
fixture. Because dialog mnemonics collide, `--open NAME "ACTIONS"` stops
after the dialog screenshot so it can be checked, and `--finish NAME`
confirms and saves.

Make one change at a time and save each variant under `L:\OUT` with a short
name. To test reading, write a synthetic SAM into `L:\IN` with a header copied
from an Ami Pro-written file, open it, and read the display. Record findings
in `SAM_FORMAT_NOTES.md` with their evidence level.

## Printing to a file (2026-10-20, the print and PDF run)

- The guest has a PostScript printer **"PS File"** (Apple LaserWriter
  12/640 PS, 600 dpi) on port `FILE:`, added from the guest's own driver
  cache with `rundll32 printui.dll,PrintUIEntry /if /b "PS File" /f
  %windir%\inf\ntprint.inf /r "FILE:" /m "Apple LaserWriter 12/640 PS"`
  after a live snapshot "Before print-pdf run (2026-10-20)". The Microsoft
  XPS Document Writer stays the default printer, so fixtures keep their
  `[prn]`.
- To print a fixture: File › Printer Setup (Alt+F, T), Down to "PS File on
  FILE:", Enter; Ctrl+P, Enter; type `L:\OUT\NAME.PS` in Windows' Print to
  File box, Enter. Print › Options… is Alt+O; its check boxes toggle with
  Alt+letter (M crop marks, P without pictures, W doc description, F
  preprinted form, U update fields). Changing the printer marks the
  document changed: close it with No.
- On the host: `ps2pdf FILE.PS FILE.pdf`, then `pdftoppm` and `pdftotext`.
  Coordinates in the PostScript are 600 dpi from the printable corner (75
  and 92 dots in from the paper's edge on this printer).
- User Setup › Options › Print in background is **off**: AmiPrint failed
  with "Couldn't open spool file".
- Pitfall: `PRN.TXT` (and any `PRN.*`) is the DOS printer device, so `cmd`
  output redirected there never reaches the share; use another name.

## Exports and table lines (2026-10-20, the export run)

- **Save As in another type:** File › Save As (`alt+f a`), `alt+t` and
  `alt+down` open List files of type; a letter jumps to the type (`r` is
  Rich Text Format, `w` four times Word for Windows 2.0), `alt+up` closes
  the list keeping it (Enter would press OK), then `alt+n` and the name.
  The document keeps its own name.
- **Open RTF:** File › Open, the same type list, the name, Enter; Import
  Options asks Convert, Apply, Keep, or Ignore styles: Enter takes
  Convert. The document opens as Untitled.
- **Table › Lines & Color:** click into a cell, then the Table menu
  (replaces nothing; it appears at about (322,35) with the caret in a
  table): Select Entire Table (360,210), then Lines & Color (360,73). In
  the dialog All is (422,274), Fill color's box (409,402), the palette's
  yellow (560,402), OK (714,259). Fixtures TL1 (All) and TL4 (yellow
  fill).
- **The printer is per document:** a document opened afterwards prints to
  its own `[prn]` (XPS, a zip) until File › Printer Setup chooses PS File
  again.
