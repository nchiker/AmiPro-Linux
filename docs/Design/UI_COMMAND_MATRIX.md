# UI Command and Control Matrix

**Status:** Approved Revision 2 implementation reference, 2026-10-10. Gate A
is closed; UI-1 is the authorized active implementation slice.
**Normative for:** menu names/order, shortcut labels, placement, enabling, and
implemented-versus-future presentation.  
**Shortcut notation:** Linux/COSMIC (`Ctrl`, `Shift`, `Alt`). Shortcuts shown
here must be displayed in menus and tooltips and handled through the same
command path.

## Status and milestone legend

- **I** — implemented in the current application and routed from this surface.
- **K** — current editor/widget keyboard behavior exists, but the discoverable
  menu/command-state route is missing.
- **UI-1** — first approved UI implementation slice: command discovery,
  shortcuts, enabling, existing action wiring.
- **UI-2** — chrome/style/status hierarchy slice.
- **E-GATE** — requires the rich editor's performance, accessibility, and IME
  activation gates before it can become production behavior.
- **P2/P3/P5** — future project phase; item remains named and disabled in the
  menu until supported.
- **Hidden** — do not show in the action bar before implementation. Disabled
  menu presence is allowed because it communicates the stable command home.

Enabling is evaluated against the active document and current focus. Disabled
commands retain their labels and shortcut text but do not dispatch messages.
If a future feature has no useful present-day discovery value, its entire
submenu may be a single specifically named disabled item rather than a generic
“Not available yet”.

## File

| Command | Shortcut | Action bar | Enabled when / behavior | Current | Milestone |
|---|---|---|---|---|---|
| New Document | Ctrl+N | Icon, always visible | Application active; creates an isolated untitled tab | UI-1 I | UI-1 |
| Open… | Ctrl+O | Icon, always visible | Application active; XDG chooser (All documents, then Ami Pro, AmiPro-Linux, Word, OpenDocument, Rich Text, and text); an already-open path activates its tab; other programs' documents through Import Options into a protected tab (§31) | UI-1 I; import both builds (2026-10-24) | UI-1, §31 (IM1, IM2) |
| Import Folder… | — | Hidden | Always: the Import Folder panel in the rail (From, To, the style option, Import, Stop, the report) | Yes, page surface (2026-10-24) | §31 (IM11) |
| Close Document | Ctrl+W | Tab close only | Active tab exists; dirty tab opens Save/Discard/Cancel | UI-1 I | UI-1 |
| Save | Ctrl+S | Icon, always visible | Dirty active document or no native target; imported SAM routes to protected native Save As | UI-1 I | UI-1 |
| Save As… | Ctrl+Shift+S | Menu only | Active document exists; only project-native target accepted today | UI-1 I | UI-1 |
| Revert to Saved… | — | Hidden | Native saved target exists and differs from memory; confirmation required | No | P2 |
| Export PDF… (File › Export ▸) | Ctrl+Shift+E | Hidden; a button in the Print panel | A document is open: the desktop's save dialog, the pages as a PDF made in the background | Yes, page surface (2026-10-20) | §29 (PO1, PO2) |
| Export Word Document (.docx)… (File › Export ▸) | — | Hidden; offered in Customize Toolbar | A document is open: the desktop's save dialog, the export made in the background, the loss dialog when something is lost, Open in the status bar | Yes, page surface (2026-10-20) | §30 (EX1, EX2, EX7, EX10) |
| Export OpenDocument Text (.odt)… (File › Export ▸) | — | Hidden | As above | Yes, page surface (2026-10-20) | §30 |
| Export Rich Text (.rtf)… (File › Export ▸) | — | Hidden | As above | Yes, page surface (2026-10-20) | §30 |
| Export Plain Text (.txt)… (File › Export ▸) | — | Hidden | As above | Yes, page surface (2026-10-20) | §30 |
| Export Markdown (.md)… (File › Export ▸) | — | Hidden | As above; the loss dialog always opens (Markdown never holds the page layout, fonts, or colours) and names files that would be replaced; the pictures are written beside the .md | Yes, page surface (2026-11-06) | §30 (MD1, MD2) |
| Export Folder… (File › Export ▸) | — | Hidden | Always: the Export Folder panel in the rail (From, To, format, Export, Stop, the report) | Yes, page surface (2026-10-20) | §30 (EX6) |
| Print… | Ctrl+P | A button in the Print panel; offered in Customize Toolbar (its printer icon), not in the standard set | A document is open: the desktop's print dialog (portal), the chosen pages sent as a PDF | Yes, page surface (2026-10-20) | §29 (PO1) |
| Print Current Page… | — | Hidden; a button in the Print panel | A document is open: the print dialog with the caret's page as its range | Yes, page surface (2026-10-20) | §29 (PX7) |
| Page Setup… | — | Hidden | A document is open: opens the Page Layout Properties panel, the same handler as Format › Page Layout… (Ami Pro's File › Page Setup is its Modify Page Layout) | Yes, page surface (2026-11-07) | §18 |
| Print Options… | — | Hidden | A document is open: the Print panel in the rail (Ami Pro's options; the PDF's additions); the pages show as they print | Yes, page surface (2026-10-20) | §29 (PO1, PO5) |
| Print Envelope… | — | Hidden | A document is open: the Envelope panel in the rail (Ami Pro's sizes and Custom, the recipient from the selection or the first address, saved return addresses, feed and offsets, Print…, PDF…, New Document, Merge Envelopes) | Yes, page surface (2026-11-03) | §38 (MG8) |
| Merge… | — | Hidden | A document is open: the Merge panel in the rail (data file: Open…, New Data File…, Edit records…; fields; preview; conditions; labels; Merge to: New Document, PDF…, Printer…, View & Print, Ami Pro File…; Start from); each change one undo step | Yes, page surface (2026-11-03) | §38 (MG1–MG11) |
| Master Document… | — | Hidden | A document is open: the Master Document panel in the rail (files, Add…, Remove, ↑ ↓, Open, Open All, Locate…, checks, outputs, Generate Contents and Index); on a member, where its numbers start, Treat as Single Document, Open Master; each list change one undo step | Yes, page surface (2026-11-03) | §37 (MS1–MS11) |
| Document Properties… | Alt+Enter | Hidden | A document is open: Document Properties in the rail (Ami Pro's Doc Info), each change one undo step | Yes, page surface (2026-10-20) | §28 (FL3) |
| Quit | Ctrl+Q | Menu only | Application active; resolves every dirty tab safely | UI-1 I | UI-1 |

File grouping: New/Open; Close; Save/Save As/Revert; Export/Print (Page Setup… sits before Print
and opens the same panel as Format › Page Layout…; decisions L2, 2026-10-13, and 2026-11-07);
Properties; Quit. Recent Documents may be added between Open and Close only
after privacy, missing-path, and duplicate-session behavior is defined.

## Edit

| Command | Shortcut | Action bar | Enabled when / behavior | Current | Milestone |
|---|---|---|---|---|---|
| Undo *action* | Ctrl+Z | Icon, always visible | Active document undo stack nonempty; label names the action when available | UI-1 I; generic label today | UI-1 state/label |
| Redo *action* | Ctrl+Shift+Z | Icon, always visible | Active document redo stack nonempty | UI-1 I | UI-1 state/label |
| Cut | Ctrl+X | Full-width Clipboard group; moves with group only on narrow widths | Editable selection nonempty | UI-1 I | UI-1 |
| Copy | Ctrl+C | Full-width Clipboard group; moves with group only on narrow widths | Selection nonempty, including read-only contexts | UI-1 I | UI-1 |
| Paste | Ctrl+V | Full-width Clipboard group; moves with group only on narrow widths | Editor focused and compatible clipboard content available; plain text first; on the page surface an image without text pastes as a picture where Insert › Image… puts one (§25, P7), and rich text (RTF) pastes first with Apply styles (§31, IM12) | UI-1 I; availability refreshed on focus (text, then an image) | UI-1 |
| Paste as Plain Text | Ctrl+Shift+V | Menu only | Editor focused and clipboard contains text | UI-1 I; current clipboard is text-only | UI-1 |
| Delete | Delete | Menu only | Editable selection nonempty, a selected frame, or the caret in a cell holding a picture (removes the picture, one undo step; `UI_SPEC.md` §24); otherwise key retains forward-delete behavior | UI-1 I | UI-1 |
| Select All | Ctrl+A | Menu only | Active document contains selectable content | UI-1 I | UI-1 |
| Find… | Ctrl+F | Menu only | A document is open: the find bar under the ruler, the Find field given the keyboard with its text selected; a short selection fills Find, a longer one limits the search (FD4) | Yes, page surface (2026-10-20) | §27, F1 |
| Find Next | F3 | Menu only; the bar's ▼ and Enter | The next match after the caret in the streams searched, round to the first; opens the bar when it is closed | Yes, page surface (2026-10-20) | §27 |
| Find Previous | Shift+F3 | Menu only; the bar's ▲ and Shift+Enter | The match before the caret, round to the last | Yes, page surface (2026-10-20) | §27 |
| Replace… | Ctrl+H | Menu only | The find bar with its Replace row: Replace (the selected match, one undo step, then the next) and Replace All (one undo step, the count in the status bar) | Yes, page surface (2026-10-20) | §27, F3, F4 |
| Go To… | Ctrl+G | Status bar page number | A document is open (page surface) | Yes, page surface (2026-10-12) | Dialog: page number, first or last page, next frame |
| Previous Page | — | Status bar ▲ | The caret's page has a page before it | Yes, page surface (2026-10-12) | Caret to the top of that page's text |
| Next Page | — | Status bar ▼ | The caret's page has a page after it | Yes, page surface (2026-10-12) | As Previous Page |
| Go To Header | — | Menu only | A document is open; the caret to the start of the page's fixed header, or a status-bar reason (Begin on second page) | Yes, page surface (2026-10-13) | `UI_SPEC.md` §18 |
| Go To Footer | — | Menu only | As Go To Header, for the footer | Yes, page surface (2026-10-13) | §18 |
| Next Field | — | Edit › Power Fields ▸ | Selects the next field, into the other text streams, round to the first (Ami Pro's Next Field) | Yes, page surface (2026-10-20) | §28 |
| Previous Field | — | Edit › Power Fields ▸ | The same backwards | Yes, page surface (2026-10-20) | §28 |
| Go To Ruler | — | Menu only | The ruler takes the keyboard (Ami Pro's Edit › Go To › Ruler): arrows, Ctrl+arrows, Space, Del, 0–3, `[` `]`, Esc | Yes, page surface (2026-10-17) | §19 |

Edit grouping: Undo/Redo; clipboard/delete; selection; find/replace/go-to. Cut,
Copy, Paste, and Select All are mandatory menu entries even though native editor
bindings already work. This directly closes the memorized-shortcut defect.

## View

| Command | Shortcut | Action bar | Enabled when / behavior | Current | Milestone |
|---|---|---|---|---|---|
| Toolbar | — | Hidden | Always; checked; shows or hides the action bar; kept in the user's COSMIC configuration (`UI_SPEC.md` §12) | I | UI pass, 2026-10-12 |
| Styles Panel | — | Customizable | Always; checked state mirrors panel visibility; the docked state is kept in the user's configuration | I | UI-2 ownership/responsive behavior |
| Ruler | — | Hidden | Active document exists; checked while the ruler shows; saved with the toolbar's visibility | Yes, page surface (2026-10-17): the interactive ruler | §19 |
| Show Power Fields | Alt+F9 | Menu only | Checked; the codes in red in place of the results, typed over as text and read back when off (since 2026-10-20 after the run); one undo step; off before a save | Yes, page surface (2026-10-20) | §28 (FL6) |
| Field Shading | — | Menu only | Checked, on by default (settings); fields' shown text light grey on screen | Yes, page surface (2026-10-20) | §28 (FL6) |
| Status Bar | — | Hidden | Active document exists; checked while the bar shows; saved in the preferences (default shown); hidden, the bar still appears while the message is a failure, so no error is concealed | Yes, page surface (2026-11-07) | UI-2 optional toggle |
| Zoom In | Ctrl++ | Right status control | Active document exists and below maximum zoom | No | UI-2/shared transform |
| Zoom Out | Ctrl+- | Right status control | Active document exists and above minimum zoom | No | UI-2/shared transform |
| Actual Size | Ctrl+0 | Zoom menu/status | Active document exists | No | UI-2/shared transform |
| Fit Page Width | — | Zoom menu/status | Active document exists | Current fit is automatic, not a mode | UI-2 |
| Single Page | — | View-mode group | Active document; selected default; real pagination required for final semantics | Approximate one page | P3 |
| Continuous Pages | — | View-mode group | Shaped pagination exists | No | P3 |
| Two Pages | — | View-mode group | Shaped pagination exists and viewport permits | No | P3 |
| Full Screen | F11 | Menu only | Always; native full-screen behavior | No | P2 |

View grouping: panels/chrome; zoom; page view; full screen. No zoom percentage
appears in the action bar; the status bar owns view and zoom.

## Insert

| Command | Shortcut | Action bar | Enabled when / behavior | Current | Milestone |
|---|---|---|---|---|---|
| Page Break | Ctrl+Enter | Hidden | Caret in the main text; the break at the caret without splitting the paragraph, an empty paragraph after it at the end of the text (VM PB1, PB3). Caret in a table cell: the row starts the next page (Ami Pro's page break in a table, VM TP6), checked while the row has one, chosen again to take it away (§26, TP1) | Yes, page surface (2026-10-13; in a cell 2026-10-19) | §18, §26 |
| Column Break | Ctrl+Shift+Enter | Hidden | Caret in the main text; the mark at the start of the caret's paragraph, once (VM CB1) | Yes, page surface (2026-10-13) | §18 |
| Frame… | — | Hidden | A document; the small Create Frame dialog (Width, Height, Down from top, In from left, from the last frame made; Create on the page in view, Draw on Page, Cancel; a frame off the paper refused in place); Ami Pro's default frame, selected, one undo step (VM FC1, FC5, FC9) | Yes, page surface (2026-10-17) | §20 (F1) |
| Draw Frame | — | Standard (last) | A document; the next press on a page draws a frame (crosshair, dashed outline kept on the paper); a click, Esc, or a press outside every page ends drawing (VM FC2–FC4, FC10, FC11) | Yes, page surface (2026-10-17) | §20 (F1) |
| Table… | Ctrl+F12 | Hidden | The caret in the main text outside a table, or one empty text frame selected: Insert Table's dialog, a table at the caret or in the frame, one undo step | Yes, page surface (2026-10-24) | §32 (TD1, TD2) |
| Table menu: Insert Row Above/Below, Insert Column Left/Right, Delete Row/Column/Table, Select ▸ Column/Row/Entire Table, Connect Cells (Disconnect Cells in a joined cell), Headings ✓, Leaders ▸ ✓, Protect Cells ✓, Distribute Rows/Columns Evenly, Table Style ▸, Edit Formula… (focuses the rail's Formula box), Quick Add ▸ Row/Column, Recalculate Table (since 2026-11-06, TD.7: one cell, Honor protection, Row needs a cell to the left and Column one above, Recalculate a formula in the table; no shortcuts, Ami Pro has none), Sort Ascending, Sort Descending (since 2026-11-04, MR1: the rows below the headings by the caret's column), Convert Text to Table, Convert Table to Text, Table Properties… | — | Hidden; the row, column, and cell items in the right-click menu in a table | The caret or a cell selection in a table (each with Ami Pro's own rule: Select Column needs two rows, Headings a page table, Distribute Rows no Automatic); Convert Text to Table: paragraphs of the main text selected | Yes, page surface (2026-10-24) | §32 (TD1–TD10) |
| Drawing… | — | Hidden | Where Insert › Image… can put a picture (a new frame at the caret, an empty frame, an empty cell), or a drawing selected (which opens): a new drawing and Draw mode | Yes, page surface (2026-10-25) | §33 (DW1–DW3) |
| Draw menu: Line and Fill…, Extract/Apply Line & Fill, Rotate Left/Right 90°, Rotate by Step, Flip ▸ Horizontal/Vertical, Group, Ungroup, Bring to Front, Send to Back, Bring Forward, Send Backward, Align ▸ (six), Distribute ▸ Across/Down, Duplicate, Snap To ✓, Show Grid ✓, Select All Objects ✓, Import Drawing…, Save Drawing As…, Done Drawing | — | The draw bar (in the toolbar's place in Draw mode); the selection's items in the right-click menu in Draw mode | Draw mode open; each with Ami Pro's rule (Extract one object, Group two, Align two, Distribute three, Flip a flippable object) | Yes, page surface (2026-10-25) | §33 (DW2–DW6) |
| Image… | Ctrl+Shift+I | Hidden | A caret in the main text, an empty table cell, or one empty text frame or picture frame selected; the portal's chooser (BMP, WMF, PNG, JPEG, GIF, TIFF, PCX, SVG) with Link to file; the picture into the empty frame or cell, a new Flow with text frame at the caret the picture's size, or replacing the selected picture; one undo step (VM PI1–PI9) | Yes, page surface (2026-10-18) | §25 (P1–P3) |
| File… (Insert) | — | Hidden | A document is open: the portal's chooser for the formats read, Import Options, the file at the caret (or the selection's start) in any flow, one undo step | Yes, page surface (2026-10-24) | §31 (IM1, IM3) |
| Drawing… | — | Hidden | Drawing/object model exists | No | P5 |
| Floating Header/Footer… | — | Hidden | Caret in the main text; a small dialog: insert one for all, right, or left pages, or remove the one at the caret (VM FH1–FH3) | Yes, page surface (2026-10-13) | §18 (L3) |
| Page Number… | — | Hidden | A caret (any text); a small dialog: style, Start on page, Start with number, leading text; inserts Ami Pro's field or changes the one at the caret (VM PN1) | Yes, page surface (2026-10-13) | §18 (L3) |
| Footnote | Ctrl+Alt+F | Hidden | The caret in the main text or a table's cell: the reference there and the caret in the new footnote at the page's foot (in a cell, Ami Pro's joined row at the table's end); one undo step | Yes, page surface (2026-10-26) | §34 (NR8) |
| Note | Ctrl+Alt+M | Hidden | The caret in any text: a note there with your initials and colour, opened in the Notes panel; one undo step | Yes, page surface (2026-10-26) | §34 (NR1) |
| Cross-Reference… | — | Hidden; the Bookmarks panel | The Bookmarks panel, where Its page or Its text inserts a PageRef or Bookmark field | Yes, page surface (2026-10-26) | §34 (NR6) |
| Bookmarks… (Edit) | Ctrl+Shift+F5 | Hidden | The Bookmarks panel: Add, Go To, Rename, Remove, Show bookmarks; each change one undo step | Yes, page surface (2026-10-26) | §34 (NR4, NR7) |
| Notes… (Edit) | — | Hidden | The Notes panel | Yes, page surface (2026-10-26) | §34 (NR1) |
| Links… (Edit) | — | Hidden | The Links panel: the document's OLE objects and linked frames, each with kind, source, and status; read-only (no Update, Unlink, or Edit: the servers are Ami Pro's); a row selects its frame, whose Frame Properties show the Object section; an empty list says so | Yes, page surface (2026-11-06) | §42.1 (OD7) |
| Paste Special… (Edit, after Paste; since 2026-11-07, DO.4) | — | Hidden | The Paste Special panel in the rail: the formats the clipboard offers (Text, Rich Text Format, HTML, PNG, JPEG, GIF, Bitmap, TIFF, SVG images, Files) in the list's order, the first chosen; Paste (or a double-click on a row) pastes the chosen one as text, formatted text, or a picture, one undo step, and closes the panel; a format the app cannot hold shows Ami Pro's message 3092 ("Cannot accept an object that Ami Pro can not display") and the panel stays; an empty clipboard is looked at twice, then says so. Plain Paste unchanged; Paste Link and New Object are not offered | A document on the page surface | Yes, page surface (2026-11-07) | §42.2 (OD10) |
| Image Processing… (Tools, after Sort…; since 2026-11-07, DO.5) | — | Hidden | Enabled for one selected bitmap frame a reader takes; opens the rail, whose Frame Properties › Picture has an Image section: Brightness, Contrast, Edge enhancement, Smoothing (typed 0 to 100 beside a slider; a drag is one undo step), Invert, Halftone (a print setting), Revert; non-destructive; a gray-scale TIFF keeps them in Ami Pro's own `.tgf` form, any other bitmap has them baked in by Save As SAM (listed) | One selected bitmap picture frame | Yes, page surface (2026-11-07) | §42.3 (OD8) |
| Charting… (Tools, after Image Processing…; since 2026-11-07, DO.8) | — | Hidden | The Chart panel in the rail: for a chart frame its panel; for a frame holding a drawing or picture a question (Replace with a Chart, Cancel); for an empty text frame, an empty table cell, or the main text a new chart from the clipboard's tab-separated text (else sample data); two or more selected table cells chart those cells. Type (twelve icons, variations), Legend, Grid, 3D with Depth and Perspective, colour set, Axis (Automatic, Inc, Min, Max; not for pies), Flip Data, Data grid and Data text, Copy Data as CSV, Save Chart as Image…, Drawing…; every change one undo step; Esc closes; double-click on a chart opens it (Draw mode when last edited with Drawing) | A document on the page surface, outside Draw mode | Yes, page surface (2026-11-07) | §42.4 (OD3 to OD6) |
| Chart from Table (Tools, after Charting…; since 2026-11-07, DO.8) | — | Hidden | The caret in a table or a cell selection: the cells (the whole table for a caret) become the data of a new chart in a With para above frame after the table, opened in the Chart panel; the data is a copy; also a button in the panel | The caret or the cell selection in a table | Yes, page surface (2026-11-07) | §42.4 (OD4, OD11) |
| Equations… (Tools, after Chart from Table; since 2026-11-07, DO.9) | — | Hidden | Equation mode in place: an equation frame (a selected one, or a double-click or Enter on it) opens the source card under the frame and the Equation panel in the rail; on an empty text frame, an empty table cell, or the main text a new empty equation starts (Ami Pro's 220 by 300 twip box). The card holds the TeX source, the equation is redrawn as it is typed, a text that does not parse shows its message and position and the last good drawing stays; Enter or Esc leaves. The panel: the template palette (fractions, radicals, scripts, sums and integrals with limits, brackets, matrices, over and under, functions, Greek, operators, relations, arrows), face and size, Import Equation…, Save As Equation…, Copy as LaTeX, Paste LaTeX. A run of typing is one undo step with the template that began it; each template, face or size change is one step | A document on the page surface, outside Draw mode | Yes, page surface (2026-11-07) | §42.5 (OD2, OD6) |
| Paste LaTeX Equation (Tools, after Equations…; since 2026-11-07, DO.9) | — | Hidden | The clipboard's text when it begins with `$`, `\[`, `\(` or `\begin{equation}` becomes an equation (in the one being edited, else a new one) in one undo step; any other text leaves the status bar saying so. Edit › Paste never does this by itself (OD11) | A document on the page surface, outside Draw mode | Yes, page surface (2026-11-07) | §42.5 (OD11) |
| Footnote Options… (Tools) | — | Hidden | The Footnotes panel: Ami Pro's Footnote Options, live, each change one undo step | Yes, page surface (2026-10-26) | §34 (NR10) |
| Notes ✓, Bookmarks ✓ (View) | — | Hidden | Note marks shown (on by default) and bookmarks bracketed (off), saved | Yes, page surface (2026-10-26) | §34 (NR1, NR5) |
| Endnote | — | Removed 2026-10-26 (NR9: endnotes are Footnote Options' Make endnotes) | — | — | §34 |
| Bullet… | — | Offered by Customize Toolbar (the app's own icon) | A caret or selection in any text; a small dialog of Ami Pro's 17 bullets drawn (Ami Pro's Edit › Insert › Bullet…); the bullet replaces the selection or goes in at the caret, one undo step; the last one offered next (settings) | I with `page-surface` (2026-10-17) | §21, B4 |
| Date/Time… | — | Hidden | A document is open: the Fields panel at Date and time (Ami Pro's Edit › Insert › Date/Time…) | Yes, page surface (2026-10-20) | §28 (FL2) |
| Field… | — | Hidden | A document is open: the Fields panel at Power field | Yes, page surface (2026-10-20) | §28 (FL2) |
| Merge Field… | — | Hidden | A document is open: the Merge panel at its fields; a click inserts `MergeField NAME` at the caret, one undo step | Yes, page surface (2026-11-03) | §38 (MG1, MG10) |
| Edit Field… | — | Edit › Power Fields ▸; a field's right-click menu | The caret is in a field: the Fields panel's This field | Yes, page surface (2026-10-20) | §28 |
| Special Character… | — | Hidden | Character chooser/insert path exists; do not intercept the platform's Ctrl+Shift+U Unicode entry | No | P2 |

Insert grouping: breaks; objects; page regions/notes; fields/characters. The
listed shortcuts are modern product proposals, not verified Ami Pro shortcuts;
the Unicode-entry conflict must be resolved before assigning Ctrl+Shift+U.

## Format

| Command | Shortcut | Action bar | Enabled when / behavior | Current | Milestone |
|---|---|---|---|---|---|
| Styles | — | Quick-apply field + panel | Active document exists; menu opens/focuses the single owner panel | Style apply I | UI-2 |
| Modify Current Style… | — | Hidden | One uniform style is selected and style editing semantics exist | No | P2 |
| 1st line bold (Style Properties › Font) | — | Check box after Caps | Exclusive with Bold; the first line of each paragraph in the style is bold, later lines regular; live, one undo step | I with `page-surface` (2026-11-07) | FIRST_LINE_BOLD_NOTES.md |
| Bold | Ctrl+B | Toggle icon | Page surface: a selection, or a caret, where it formats the text typed next (VM C1, `UI_SPEC.md` §13); checked when the whole selection, or a character beside the caret, looks bold. Text editor: a selection | I (caret since 2026-10-12) | UI-1 then E-GATE |
| Italic | Ctrl+I | Toggle icon | Same rule as Bold, check mark included | I | UI-1 then E-GATE |
| Underline | Ctrl+U | Toggle icon (Standard set with the page surface) | As Bold on the page surface, one undo step | I with `page-surface`; future in the default build, whose editor cannot draw it | UI pass (gap A1) |
| Word Underline | — (Ami Pro's Ctrl+W is Close Document) | Menu only | As Underline; words underlined, the spaces between them not | I with `page-surface` (2026-10-12) | §13 |
| Caps ▸ Upper Case, Lower Case, Initial Caps, Small Caps | — | Menu only | As Bold; each replaces the others; a checked one removes it | I with `page-surface` (2026-10-12) | §13 |
| Special Effects… | — | Menu only | Selection or caret; Superscript, Subscript, Double underline, Strikethrough, Overstrike character (since Phase 3.3); OK switches the changed boxes, one undo step | I with `page-surface` (2026-10-12) | §13 |
| Font… | — | Menu only | Selection or caret; face, size, colour, Revert to style; OK writes all three | I with `page-surface` (2026-10-12) | §13 |
| Normal | Ctrl+M (Ami Pro's Ctrl+N is New) | Offered by Customize Toolbar | Selection or caret; character and paragraph formatting back to the styles (VM C1) | I with `page-surface` (2026-10-12) | §13 |
| Paragraph… | — | Hidden | Selection maps to paragraphs and paragraph-property model exists | No | P2/P3 |
| Align Left | Ctrl+L | Paragraph group, selected when the selected paragraphs share it | The selected paragraphs or the caret's, over their styles' (`SetParagraphAlignment`, one undo step); none while frames are selected; menu check when every selected paragraph has it | I with `page-surface` (2026-10-12) | UI pass (gap A2) |
| Center | Ctrl+E | Paragraph group | As Align Left | I with `page-surface` | UI pass (gap A2) |
| Align Right | Ctrl+R | Paragraph group | As Align Left | I with `page-surface` | UI pass (gap A2) |
| Justify | Ctrl+J | Paragraph group | As Align Left | I with `page-surface` | UI pass (gap A2) |
| Bullets | Ctrl+Shift+L | Toggle icon (the app's own), list group of the Standard set | The selected paragraphs or the caret's take the document's Bullet 1 (or Bullet), made from `_DEFAULT.STY` when it has none, or Body Text when every one shows bullets (`SetList`, one undo step); none while frames are selected; menu check when every selected paragraph shows bullets; also in the text right-click menu | I with `page-surface` (2026-10-17) | §21, B1–B2 |
| Numbering | Ctrl+Shift+N | Toggle icon (the app's own), list group | As Bullets with Number List | I with `page-surface` (2026-10-17) | §21, B1–B2 |
| Promote | Alt+Left (a selected frame's Alt+Left still resizes it) | Icon (`format-indent-less-symbolic`), list group | The selected paragraphs or the caret's go one level up: List n to List n−1 (List 1 stays), a levelled style to the style at the level above, None to the previous levelled paragraph's level; Outline# made when no style is at the level (`ChangeLevel`, one undo step) | I with `page-surface` (2026-10-18) | §22, O1, O2 |
| Demote | Alt+Right | Icon (`format-indent-more-symbolic`), list group | One level down: an item of a Number List run nests it (the run List 1, the items List 2), List n to List n+1, a levelled style to the level below, None to one below the previous levelled paragraph | I with `page-surface` (2026-10-18) | §22, O1, O2 |
| Bullet and Number Style… | — | Menu and text right-click menu | Style Properties for the caret paragraph's style at Bullets and numbers | I with `page-surface` (2026-10-17) | §21, B5 |
| Outline Numbering… (Format › Styles ▸) | — | Menu only | A dialog of four schemes for the document's levelled styles (1. A. i. a.; Ami Pro's I. A. 1. i. a.; Ami Pro's 1, 1.1, 1.1.1; none), Apply as one undo step; disabled Apply without levelled styles | I with `page-surface` (2026-10-18) | §22, O3 |
| Increase Indent | Ctrl+] | Overflow after implementation | Paragraph indentation supported | No | P2/P3 |
| Decrease Indent | Ctrl+[ | Overflow after implementation | Paragraph indentation supported | No | P2/P3 |
| Frame Properties… | — | Hidden; future inspector | One frame selected and frame model exists | No | P3 |
| Graphics Scaling… | — | Hidden; Format › Frame ▸ and the frame's right-click menu | One picture frame selected; opens Frame Properties, whose Picture section is first (Ami Pro's Frame › Graphics Scaling…) | Yes, page surface (2026-10-18) | §25 (P5) |
| Page Layout… | — | Hidden; margin double-click and right-click | A document is open; Page Layout Properties in the rail, following the caret's page; each change one undo step | Yes, page surface (2026-10-13) | `UI_SPEC.md` §18 (L1, L2, L4) |

Format grouping: styles; character; paragraph; lists/indents; contextual object
properties. Since 2026-10-18 (§22, the user: "The format menu is getting
very tall") the page surface's Format menu folds Alignment ▸ (the four
alignments), Bullets and Numbering ▸ (Bullets, Numbering, Promote, Demote,
Bullet and Number Style…; also in the text right-click menu), and Frame ▸
(Frame Properties, Bring to Front, Send to Back, Group, Ungroup), as Ami
Pro's Text menu nests Alignment ▸ and Caps ▸. The modern **Format** menu consolidates the original application's
evidenced Text and Style paths; this is an intentional modernization pending
the focused VM comparison.

## Tools

| Command | Shortcut | Action bar | Enabled when / behavior | Current | Milestone |
|---|---|---|---|---|---|
| Spell Check… | F7 | Menu only | A document is open: the Proofing panel's Spelling page, checking from the caret's word (or the selection alone; Ami Pro's options), each replacement one undo step, Replace All in one (SP4) | Yes, page surface (2026-10-26) | §35 (SP3, SP4) |
| Check Spelling as You Type | — | Menu only | Checked while on (default on, saved): red wavy underlines after a pause; the right-click menu's suggestions | Yes, page surface (2026-10-26) | §35 (SP2) |
| Thesaurus… | Shift+F7 | Menu only and the right-click Synonyms ▸ | The word at the caret (Ami Pro's rule), its meanings and synonyms in its capitals and inflection; Replace one undo step, the panel stays | Yes, page surface (2026-10-26) | §35 (SP7) |
| Grammar Check… | — | Menu only | English documents: the Grammar page with Ami Pro's sets (`[GramStyle]`), Replace, Skip, Skip Rule, Next Sentence, statistics at the end | Yes, page surface (2026-10-26) | §35 (SP8) |
| Readability Statistics… | — | Menu only | Ami Pro's figures for the document, or the selection | Yes, page surface (2026-10-26) | §35 (SP8) |
| Language… | — | Menu only | The Proofing panel at its options: the document's language (one undo step), Use for new documents | Yes, page surface (2026-10-26) | §35 (SP6) |
| Sort… (Tools, after Language…; since 2026-11-06, TD.8) | — | Menu only | The Sort panel in the rail: three levels (field, Alphanumeric or Numeric, Word 1st, 2nd, All), Ascending or Descending, delimiter (text only), paragraphs or rows in a record, for the selected paragraphs, all text, or the table's (selected) rows; refusals and questions in the panel; one undo step; settings kept per tab. Table › Sort Ascending and Descending stay | A document on the page surface | Yes, page surface (2026-11-06) | §41 (TN3) |
| TOC, Index… | — | Menu only | A document is open: the Contents & Index panel (levels, Generate Contents and Update, Generate Index); each change one undo step | Yes, page surface (2026-11-03) | §36 (TX1–TX6) |
| Mark Text ▸ Index Entry… | — | Menu only | The Contents & Index panel's Entries page: the selection as the Primary, or the mark at the caret; Mark, Mark All, Remove, Next and Previous (same) | Yes, page surface (2026-11-03) | §36 (TX1, TX4) |
| Mark Text ▸ TOC Entry… | — | Menu only | The Entries page's TOC entry: the caret style's level, the selection's text; one undo step | Yes, page surface (2026-11-03) | §36 |
| Mark Text ▸ Glossary… | — | Menu only | The Glossary panel: the selection added to the data file as a named record (Ami Pro's rules) | Yes, page surface (2026-11-03) | §36 (TX7, TX8) |
| Glossary Record… (Insert) | Ctrl+K | Menu only | The letters and digits at the caret replaced by the record they name, inline, one undo step; else the Glossary panel's list | Yes, page surface (2026-11-03) | §36 (TX7) |
| Word Count | Ctrl+Shift+C | Menu only | A document is open: Document Properties at Statistics | Yes, page surface (2026-10-20) | §28 |
| Update Fields | F9 | Edit › Power Fields ▸ and a field's right-click menu | The fields in the selection or the one at the caret (Ami Pro's Power Fields ▸ Update; with codes shown, the edited code read back first), one undo step | Yes, page surface (2026-10-20) | §28 (FL4) |
| Update All Fields | Ctrl+F9 | Edit › Power Fields ▸ | Every field but locked and mark kinds, Seq renumbered (Ami Pro's Update All; codes switched off), one undo step | Yes, page surface (2026-10-20) | §28 (FL4) |
| Lock Field | — | Edit › Power Fields ▸ and a field's right-click menu | The caret is in a power field; checked while locked | Yes, page surface (2026-10-20) | §28 |
| Customize Toolbar… | — | Menu only | Always; edits the one toolbar (add, remove, move, separators, Reset to Standard), shows the bar, saves each change (`UI_SPEC.md` §12) | I | UI pass, 2026-10-12 |
| Preferences… | Ctrl+, | Menu only | Always; application settings dialog exists | No | P2 |

Tools grouping: language tools; document calculation; application preferences.
Macro commands are deliberately absent from the main approval-candidate
inventory until the product-priority-P2 macro-compatibility scope is separately
approved.

## Window

| Command | Shortcut | Action bar | Enabled when / behavior | Current | Milestone |
|---|---|---|---|---|---|
| Next Document | Ctrl+Tab | Tabs | More than one tab | UI-1 I | UI-1 label/enabling |
| Previous Document | Ctrl+Shift+Tab | Tabs | More than one tab | UI-1 I | UI-1 label/enabling |
| *Open document list* | mnemonic index only | Tabs/overflow | Dynamic checked list; selecting activates tab; show when at least one tab exists | UI-1 I | UI-1 |

The original release document evidences Tile Windows and Cascade Windows, but
the modern tabbed design does not expose them without multi-window support.
They are recorded in the comparison rather than presented as inert commands.

## Help

| Command | Shortcut | Action bar | Enabled when / behavior | Current | Milestone |
|---|---|---|---|---|---|
| AmiPro-Linux Help | F1 | Hidden | Packaged/current help exists | No | P2 |
| Keyboard Shortcuts | — | Hidden | Generated from the authoritative command registry | No | UI-1/P2 |
| Compatibility and Fidelity | — | Hidden | Report UI exists; explains SAM/STY protection and unsupported content | No | P2 |
| About AmiPro-Linux | — | Hidden | Always; native COSMIC About dialog | UI-1 I | UI-1 |
| Check for Updates… (Help) | — | Hidden | Always; opens the Updates panel in the rail and checks GitHub Releases of the public repository; the panel shows the installed version, the state, the release notes' first 20 lines, Check again, Download and Install (download, SHA-256 check, then a confirmation showing `pkexec apt-get install -y <file>`), and Restart; without `pkexec` or `apt-get` it shows the command instead; Esc closes | Yes, page surface (2026-11-07, P6.2) | §43 |
| Recover Documents… (File, after Revert to Saved…) | — | Hidden | Always; opens the Recover documents panel in the rail (also at start when recovery copies of an earlier crashed session exist): each copy with its original path, time and size, Recover (opens it as an unsaved tab "name (recovered)" whose original is a Save As source, never overwritten silently), Discard, Discard all; Esc closes and keeps the copies | Yes, page surface (2026-11-07, P6.3) | §44 |

The original QuickStart Tutorial is confirmed only as original behavior; it is
not shown in the modern menu until an independently authored tutorial exists.

## Action-bar responsive order

| Group | Full width, left to right | Responsive behavior |
|---|---|---|
| Document | New, Open, Save, then Print once implemented | New/future Print move only at the narrowest width; Open and Save remain |
| History | Undo, Redo | Remain; disabled states retain stable positions |
| Clipboard | Cut, Copy, Paste | Move together below approximately 860 logical px; never split |
| Style | Quick apply, 168–220 wide | Shrink to 144; remain visible |
| Character | Bold, Italic, accepted Underline | Move together below approximately 1040 |
| Paragraph | Implemented alignment, then list controls | Move first below approximately 1180 |
| Lists (page surface) | Bullets, Numbering, Promote, Demote | With Draw Frame after it, moves to More below about 971 px (the compact style field; about 1,000 with the wide one), because More needs room of its own (§22) |
| More | Labeled `More` with chevron | Appears only when a narrow window displaces action-bar groups, and contains only those buttons (user, 2026-10-12: at full width it duplicated menu commands and confused) |

At representative full desktop width, all implemented common groups are
visible. `More` uses the same labels, state, shortcuts, and dispatch as the main
menus, never contains placeholders, and never becomes the sole discoverable
route to a command. The future Print location is specified without showing a
nonfunctional toolbar icon in the shipped interface.

## State rules that require automated coverage

1. Menu shortcut text and actual bindings are generated from one command
   registry; a displayed binding cannot be inert.
2. Undo/Redo, Cut/Copy/Delete, Save, tab cycling, and formatting enable/disable
   immediately when active tab, selection, history, dirty state, or focus
   changes.
3. Toolbar, menu, context menu, and shortcut dispatch the same command and
   produce one undo transaction.
4. Switching tabs cannot leak enabled state, style, selection, history, or
   source-save behavior.
5. Disabled future commands never send the generic `Unavailable` action and
   toolbar never displays them.
6. Every action-bar icon has an accessible name and tooltip; every toggle has a
   programmatic selected/mixed state.
