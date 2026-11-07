# Commands Progress (run A)

The live state of `docs/COMMANDS_RUNBOOK.md` (tag `commands-run-start`).
Resume at the first slice that is not `DONE`.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| CA.1 Setup | DONE | `8bc80fc` | `UI_SPEC.md` §39; CM1–CM12 and OM1–OM4 in `DECISIONS.md`; tag `commands-run-start`; the baseline below | Equal to the merge run's exit but for three VM files added since |
| CA.2 Help additions | DONE | `6813456` | `help_drawers.rs` tests; scenario `commands.rs`; screenshots light and dark (harness draws the context drawer) | Both in the context drawer beside About |
| CA.3 Preferences | DONE | `e76c71e` | settings round trip in a scratch configuration; scenario `commands.rs`; screenshots | General (undo levels, recent files, default sheet, the `.bak`), Folders, View, Toolbar; File menu recent files; CK1–CK3 |
| CA.4 File › New… | DONE | `e76c71e`, (this commit) | twin `new_documents_match_ami_pros`: nine shipped sheets' New documents equal to Ami Pro's (fix below); scenario `templates.rs` (memo with contents saved as SAM naming its sheet, reopened); screenshots; the user's 192 sheets listed in 0.3 s (release), 132 with contents, 40 with a macro | `[newmac]` named in the status bar, not run (CM2); toolbar item available |
| CA.5 Paragraph Properties | DONE | `5e9d6f9` | core `SetParagraphLineSpacing`, `RevertParagraphIndents` tests; scenario (each change one step, `<:I…>` and `<:S+360>` written, SAM round trip); screenshots | Indention, Spacing, Alignment per paragraph; Above, Below, breaks shown from the style (SAM keeps them only there); Increase/Decrease Indent (Ctrl+], Ctrl+[) ±0.5 in. |
| CA.6 Fast Format | DONE | `5e9d6f9` | `fast_format.rs` tests; widget test (the release sent only while on); scenario (bold, then a style, each selection one step, Esc, Cancel) | Ami Pro's question when nothing or a whole paragraph is selected; CK5, CK6 |
| CA.7 Style and Edit items | DONE | `3d414bb` | scenario (Protected Text and No Hyphenation checked at the caret and written `<+*>`/`<+.>`; Ctrl+Y and Enter; Revert to Saved asked, Cancel, Revert as one step, Undo); formats test `ended_attributes_in_mid_paragraph_read_back_as_the_style` | Define Style is Create Style (§14, already); CK7; the reader fix below |
| CA.8 Page items | DONE | `3d414bb` | scenario `page_layout.rs` (Insert/Remove Ruler, Insert/Remove/Revert Page Layout, each one step, SAM round trip) | Format › Page Layout Changes ▸ and Ruler ▸ over what the ruler and Page Layout Properties already did |
| CA.9 Status bar and characters | DONE | `0994391` | scenario (Face and 18 pt from the status bar, one step each; © inserted and remembered; SAM round trip); screenshots | The Face list holds the document's faces even when not installed; Special Character is Windows-1252 above ASCII with recent ones |
| CA.10 Zoom and Facing Pages | DONE | `0994391` | `page_view` tests (pairs, Full Page fit); widget test (a press on the right-hand page); scenario `views.rs` (presets checked, Full Page back to the view before, per-tab view); screenshots | Full Page Ctrl+D, Actual Size (Standard), Enlarged 200%, Custom (Preferences, 91%); Facing Pages and Continuous Pages; Single Page removed; CK8, CK9 |
| CA.11 Draft and Clean Screen | DONE | `e0ed7e7` | `view_modes` test; scenario `views.rs` (Draft at the window's width, following a resize, editing as before, nothing saved; Clean Screen on, Esc off); screenshots | Draft Ctrl+Shift+D; Show Clean Screen F11 (full screen, Return button, Esc); CK10. The typing benchmark in Draft is measured in CA.15 |
| CA.12 Show marks, views per tab, New View | PARTIAL | (this commit) | scenario `views.rs` (each mark checked; Tabs & Returns drawn and Margins in Colour off measured in the rendered window; the Preferences View page sets them); screenshots | View › Show ▸ Tabs & Returns (toolbar choice too), Column Guides, Margins in Colour, Pictures, with Power Fields, Field Shading, Notes, Bookmarks; zoom, Facing, and Draft per tab. **Open:** Window › New View and the Vertical Ruler (CK11) |
| CA.13 Line numbering (full) | DONE | (this commit) | VM fixtures LN1–LN20 (16 made, printed to PostScript; `COMMANDS_NOTES.md` §8.5); formats `sam_line_numbering.rs` (Ami Pro's 16 files read as set, Save As lists nothing); layout `line_numbers.rs` (7 rules); twin `line_numbers_match_ami_pros_prints` (11 of 13 equal page by page, LN9 and LN13 known: printer metrics); twins LNT1, LNT2 written by the app opened in Ami Pro with the dialog as set and the numbers printed as expected; scenario `page_layout.rs` (each change one step, `[lnopts]` written, reopened, Draft hides); screenshots light and dark | Page Layout Properties › Line numbering and Format › Line Numbering…; drawn on screen, print, and PDF through the page surface; CK12 |
| CA.14 Passwords (full) | DONE | (this commit) | VM fixtures PWPLN, PWA–PWEMP, PWNOP, PWSAV, PWSV2 (`COMMANDS_NOTES.md` §9.6); `protect.rs` tests; twin `ami_pros_protected_files_open_and_are_written_byte_for_byte` (7 files open, and re-protecting gives Ami Pro's bytes exactly); twins PWT1, PWT2 (a bitmap) written by the app opened in Ami Pro; scenario `files.rs` | File › Open asks; File › Password… sets or removes; saves write protected; CK13. **Open:** the corpus's protected file needs its password (the user) |
| CA.15 Measures and twin | DONE | (this commit) | Measures below (`target/commands-baseline/ca15/`); release typing A/B against `commands-run-start` and in Draft; twin CAT1 (indents, 1½ spacing, Protected Text, No Hyphenation, ©, Number text lines every fifth) written by the app opened in Ami Pro as made | Draft typing measured with `AMIPRO_PERF_DRAFT=1` on `page_surface_typing_performance_baseline` |
| CA.16 Exit | DONE | (this commit) | `tools/verify.sh --release --corpus` green: core 628 (+19 ignored), app 556, 124, 563; records (`FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, the playbook's row, `PROJECT_STATUS.md`, `PROJECT_PLAN.md`, `CLAUDE.md`), the handoff, the push; the live check before handoff (below) | |

## Choices made in the run (CK…)

- **CK1** No documents folder in Preferences (the pinned file chooser cannot start in one).
- **CK2** Recent files 0–9 (Ami Pro 0–5), default 4.
- **CK3** Undo levels 1–1000, default 100 (Ami Pro Off–4).
- **CK4** With contents counts real text, page frames, or header and footer text (not codes alone or empty styled paragraphs); off and disabled for a sheet without.
- **CK5** Fast Format's toolbar toggle uses the eyedropper icon (the COSMIC and Pop themes have no paintbrush).
- **CK6** Fast Format applies only the text attributes picked up (plain text picked up changes no text), as Ami Pro applies its picked-up set; with the style it also gives the paragraph's own indents, spacing, alignment, and ruler.
- **CK7** Select a Style (Ctrl+Y) opens the Styles panel with its filter focused, Enter applying the first style listed: the pinned libcosmic cannot open the toolbar's style list from a key.
- **CK8** Facing Pages pairs left even and right odd pages, so page 1 stands alone on the right (the decompile reading of Ami Pro's first spread was low confidence; the user's physical check passed it, 2026-11-05).
- **CK9** Custom Zoom runs 25–400 % (Ami Pro 10–400): the app's smallest zoom.
- **CK11** Window › New View (a second tab on one session) and View › Show ▸ Vertical Ruler are not built in this session: New View needs the session split into its document (text, history) and its views (caret, scroll, zoom, mode), a slice of its own. Tabs & Returns' toolbar icon is the theme's pilcrow-and-arrow (`format-text-direction-ltr-symbolic`).
- **CK12** Line numbers beside a Wrap around frame with text on both sides are each counted and stand in the left margin at their lines' heights, over one another, as Ami Pro prints them (LN9); not improved, so that numbered pages match Ami Pro's.
- **CK13** A document's password is set or removed with File › Password… (Ami Pro: Save As's Password protect box, which the portal's file chooser cannot hold) and stays until removed, so Save As keeps it (Ami Pro's Save As box starts unticked and drops it, PWNOP, PWSAV); a wrong password at Open leaves the box open for another try (Ami Pro allows one).
- **CK10** Draft lays the main text out at the window's width on Draft pages (one column, 0.25 in. margins) but still draws frames, headers, and footers where they stand, not as one-line markers (§39's markers are an open item); Ctrl+Shift+D.

## Baseline (CA.1)

Measured 2026-11-04 at `8bc80fc` with `target/commands-baseline/baseline.sh`
(outputs in `target/commands-baseline/start/`, outside Git):

- Round trip: user corpus read back equal 417 of 417, Save As lists
  nothing 417 of 417, bodies equal 203 of 417; ISO corpus 159 of 160, 159
  of 160, 28 of 160; VM folder 417 of 417, 417 of 417, 379 of 417.
- Layout comparison: user corpus page counts equal 81% (318 of 394), line
  counts 97% (14,938 of 15,356); ISO 44% (70 of 160), 91%; VM 94% (365 of
  390), 100% (2,408 of 2,420).
- Equal to the merge run's exit (`target/merge-baseline/exit/`), the VM
  folder having three files more (AMT1–AMT3, from the merge run's checks).

## Fixes found by the run

- **Protected text and no hyphenation in mid-paragraph did not read back
  equal** (found by CA.7's scenario): the reader took Ami Pro's off marks
  (`<-*>`, `<-.>`, and likewise strikethrough and the scripts) as an
  explicit "off" for the rest of the paragraph, so Save As listed
  character formatting. In a style without the attribute, off is now the
  style's own setting (`build.rs`, `ended_as_style`), but only where the
  off mark ends an on run; an off mark with nothing on before it stays, so
  that it is written back, and runs read stay apart. A first version that
  also joined runs lowered bodies equal to the original (user corpus 203 →
  191 of 417); the final one measures equal to the baseline on all six
  measures (`target/commands-baseline/ca7/`).

- **A new document from `_REPORT2.STY` did not look as in Ami Pro**
  (the user's physical check of CA.4, which had been tested only against
  itself). The document was right; Flow with text frames and narrow gaps
  beside frames were laid out wrongly (VM RPT2A, FA1–FA3, FG1–FG6;
  `SAM_PAGE_FRAME_SPEC.md` §5.1): the bottom shadow is now in the frame's
  line, a frame is no wider than its column with its right shadow, one too
  wide for its indented line moves left to end at the line's right edge,
  blanks opening a paragraph count as text (a space and a column-wide frame
  take two lines), text wraps around shadows, and a gap beside a frame
  takes a line whose first word fits (the 360-twip minimum was a guess).
  `engine.rs`, `breaking.rs`; tests `rules.rs`. New… was then compared
  with Ami Pro's New for nine shipped sheets with Run macro off: all nine
  documents equal (`new_documents_match_ami_pros`), and their layouts
  within font metrics but for `_FAX3`'s table, 72 twips higher (open).
  Measures (`target/commands-baseline/frames2/`): user corpus page counts
  equal 318 → 322 of 394, page ends on the same page 782 → 796 of 1108,
  table page ends 73 → 76 of 106, anchored frames closer in 5 files and
  further in none; ISO corpus unchanged; VM folder unchanged but for the
  17 files added. **Open:** a word wider than a gap beside a frame breaks
  there in Ami Pro (FG3) and goes below the frame here; a Wrap around frame
  beside a column's first lines adds the first paragraph's space above
  even with When not at break (FG5, FG6; to decompile).

- **Found by the live check before handoff (2026-11-05)**, the release app
  driven on a private Xwayland display (scratch settings, files opened from
  File's recent files, no portal dialogs): (1) with the page holding the
  keyboard, keys typed while a dialog was open went into the page behind
  it, for every dialog (the page surface now ignores keys and typed text
  while `dialog()` is open, `PageSurface::modal`; test
  `typing_under_a_dialog_does_not_reach_the_page`, which fails without the
  fix); (2) the password box did not take the keyboard when it opened (it
  is focused on opening and after a wrong password); (3) Insert › File of a
  protected SAM showed Import Options and failed later (it is refused at
  once, Ami Pro's 3148). Then live: Alt+Up/Alt+Down moved sections and
  table rows; File › Password…, Save, close, reopen from recent files (a
  wrong password refused, the right one opening); Ami Pro's PWA opened with
  its password; Line numbering set in the rail (text lines, other, fifth,
  Every 7 typed, Reset each page), drawn as in Ami Pro's prints, and saved
  as `152 / Body Text / 7`. Added tests: the PDF has the numbers
  (`line_numbers_are_drawn_in_the_pdf`), Revert to Saved, recent files,
  and Insert › File with a protected file (scenario `files.rs`), the
  password box's screenshots light and dark.

- **Physical check of CA.13, CA.14, and OB.2 (Move Up/Down): PASS** (the user, 2026-11-05: "all test passed").

## Measures (CA.15)

Measured 2026-11-05 at `3ba8bb1` with `target/commands-baseline/baseline.sh`
(`target/commands-baseline/ca15/`), against the start and the frame fix
(`frames2`):

- **User corpus:** read back equal 417 of 417, Save As lists nothing 417,
  bodies equal 203; page counts equal 322 of 394 (start 318), line counts
  14,938 of 15,356, page ends on the same page 796 of 1,108 (start 782),
  table page ends 76 of 106 (start 73). Equal to `frames2` on every line.
- **ISO corpus:** 159 of 160, 159, 28; page counts 70 of 160, lines 91%;
  unchanged since the start.
- **VM folder:** 453 of 453 readable read back equal, Save As lists
  nothing 453, bodies 414; the 7 files it cannot read are the protected
  fixtures PWA–PWLT (the audit has no password). All 19 new readable
  fixtures equal in page and line counts.
- **Typing (release, README31, 344 paragraphs, 14 pages, 1280×800, 40
  samples, interleaved A/B, 3 rounds; noisy host):** the page surface at
  HEAD against `commands-run-start`, typing medians 1.60/1.99/1.69 ms at
  start/middle/end (start tag 2.05/2.37/1.73); in Draft mode 1.75/2.04/1.93
  ms, p95 at most 2.32 ms, worst at most 2.39 ms (two runs): Draft costs
  nothing measurable.
