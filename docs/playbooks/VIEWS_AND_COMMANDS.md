# Views and Commands Playbook

Part of `docs/PHASE5_PLAYBOOK.md` (queue rows 16, 17, 21). Follow its
recipe and standing rules; this file says what each run covers, what to
map, and what to expect.

**Why one playbook.** These runs add no new storage. Each is an interface
over a model that already exists (outline levels, styles, paragraph
formats, rulers, page layouts, the session). The items are small and
mostly independent, so each run is short and can be pulled forward
between large runs.

**State today.**
- **Outline levels are done:** Promote and Demote, outline numbering,
  and Outline Numbering….
- **Already built:** zoom, View › Ruler, the toolbar, the Styles panel,
  Style Properties, and Page Layout Properties in the rail.
- **Future in the registry:** Revert to Saved…, Replace…, Status Bar, Two
  Pages, Single Page, Continuous Pages, Special Character…, Paragraph…,
  Preferences…, Move Document to New Window, Keyboard Shortcuts, and
  Compatibility and Fidelity.
- **In `CLAUDE.md`'s gaps:** Fast Format, the status bar's Face and Point
  Size buttons, editing commands for indents and spacing other than
  through styles, the Edit › Mark Text commands, Line Numbering and
  inserted rulers, and opening password-protected SAM files.

**Evidence to start from.**
- **Guide:** ch. 3 (User Setup defaults), ch. 4 (password protection,
  Fast Format), ch. 6–7 (Indention, Spacing, Fast Format), ch. 10 (views
  and modes), ch. 11 (Outline mode), ch. 20 (Line Numbering).
- **Commands:**
  - View: Full Page (0xc9, Ctrl+D), Custom (0xca), Standard (0xcb),
    Enlarged (0xcc), Facing Pages (0xd4), Layout Mode (0xd2), Outline
    Mode (0xda), Draft Mode (0xd3, Ctrl+M), Show Clean Screen (0xe9),
    Show Styles Box (0x133), View Preferences… (0xce);
  - the Outline menu (1058): Promote, Demote, Move Up (Alt+Up), Move Down
    (Alt+Down), Expand (Alt+PageDown), Contract (Alt+PageUp), Use Outline
    Styles;
  - Text: Indention… (0x109), Spacing… (0xfc), Fast Format (0xf6,
    Ctrl+T);
  - Style: Define Style (0x144), Select a Style (0x132, Ctrl+Y);
  - Page: Ruler ▸ Insert (0x173) and Remove (0x174), Line Numbering…
    (0x167), Insert Page Layout ▸ Insert, Remove, Revert (0x177, 0x175,
    0x176);
  - File: Revert to Saved (0x69);
  - Tools: User Setup… (0xa2);
  - Window: New Window (0x20d), Tile (0x20e), Cascade (0x20f).
- **Storage:** `[lnopts]` (line numbering, in every file), `<:R…>` (a
  paragraph's own ruler: 37 user and 4 ISO files, already typed), and
  `[encrypt]` (password-protected files, refused today; their scheme is
  to map, and their count in the corpus to take).

## Run 1: `commands`, queue 16 (small)

**Goal.** The remaining verified commands of the Text, Style, Page, File,
and Edit menus, each as Ami Pro has it, one undo step each:
- **Text:** Indention… and Spacing… for the selected paragraphs, and Fast
  Format.
- **Style:** Define Style (from the caret's paragraph) and Select a Style
  (Ctrl+Y).
- **Page:**
  - Ruler ▸ Insert and Remove (naming the paragraph's own ruler, which
    the ruler run made);
  - Line Numbering…;
  - Insert Page Layout ▸ Insert, Remove, and Revert, where not yet a
    command.
- **Edit:** Mark Text ▸ Protected Text and No Hyphenation.
- **File:** Revert to Saved, and opening (and, if the scheme allows,
  saving) password-protected files.
- **Tools:** User Setup's settings that apply (backups, timed save, undo
  levels, recent files) as Preferences….
- **Status bar:** the Face and Point Size buttons.

**Map.** Each dialog and its limits (`AMIPROUI.DLL`), Fast Format's
behaviour (copy and apply, the paragraph style or the text format), Line
Numbering's options and drawing, and the password scheme.

**Decisions to expect.**
- **Indention and Spacing.** Recommend a Paragraph Properties panel in
  the rail (the registry's Format › Paragraph…) for the paragraph's own
  indents, spacing, alignment, and breaks, beside Style Properties.
- **Fast Format.** Recommend Ami Pro's behaviour, also as a toolbar
  toggle.
- **Password-protected files.** Recommend open and save, if the scheme
  is known.
- **Preferences.** Recommend a Preferences page for the settings that
  apply (including the style-sheet folders, today in their own place).

**Better ways and logical additions to offer.**
- Insert › Special Character… (Ami Pro had an ANSI table, App. C), with
  recent characters.
- Help › Keyboard Shortcuts, generated from the registry.
- Help › Compatibility and Fidelity, from the loss check.
- Timed save with crash recovery (`PROJECT_PLAN.md` Phase 6) instead of
  Ami Pro's timed save.

## Run 2: `outline-mode`, queue 17

**Goal.** View › Outline Mode as Ami Pro has it:
- level icons;
- Move Up and Move Down, which move a heading with its section;
- Expand and Contract, and showing levels 1 to n;
- Use Outline Styles;
- typing and editing in the outline.

The `PROJECT_PLAN.md` Phase 5 item, after the outline levels run.

**Map.** Outline mode's display (indents, icons, collapsed marks), what
counts as a section, how Move Up and Down treat frames and tables, and
the Outline toolbar.

**Decisions to expect.** Recommend Ami Pro's Outline mode on the page
surface, plus an outline navigator in the rail (an improvement): the
headings as a tree, a click going to the heading, and dragging to move a
section.

**Better ways and logical additions to offer.** The navigator, collapsed
sections in Layout mode too (native only, if SAM cannot store them), and
keyboard moves in the navigator.

## Run 3: `views`, queue 21 (small)

**Goal.**
- **Zoom presets:** Full Page, Custom, Standard, and Enlarged (zoom
  exists; the presets are Ami Pro's).
- **Pages side by side:** Facing Pages (the registry's Two Pages).
- **Draft mode:** the text without the page layout, as Ami Pro draws it.
- **Show Clean Screen.**
- **View Preferences:** shown marks (tabs and returns, column guides,
  margins in colour, notes, the vertical ruler), as Ami Pro has them.
- **Window:** a second window on the same document (New Window).

**Map.** Each view's drawing rules and View Preferences' options
(`AMIPROUI.DLL`).

**Decisions to expect.**
- **Paging.** Recommend continuous pages (the current workspace) plus
  Facing Pages.
- **Draft mode.** Recommend a column of text at the window's width,
  for writing.
- **New Window.** Recommend a second view of one document with one
  history.

**Better ways and logical additions to offer.** Show Formatting Marks as
a toolbar toggle, a focus mode, and the view per tab remembered.
