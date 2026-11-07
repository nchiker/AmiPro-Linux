# Notes, Bookmarks, Footnotes, and Endnotes Runbook

**Written:** 2026-10-26, the Phase 5 queue's run 10
(`docs/PHASE5_PLAYBOOK.md`, `docs/playbooks/FIELDS_AND_REFERENCES.md`,
"Run 2: `notes`"). **Purpose:** carry notes, bookmarks, footnotes, and
endnotes from start to finish as a straight run. Its decisions were asked
before the run and answered on 2026-10-26 (`docs/research/DECISIONS.md`,
"Notes, bookmarks, footnotes, endnotes: the user's answers", NR1–NR10);
the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §34 the interface. Evidence:
`docs/research/NOTES_BOOKMARKS_FOOTNOTES.md` (the decompiled `AMIPRO.EXE`
and `AMIPROUI.DLL`; Ami Pro 3.1 fixtures RNN1–RNN3, RNB1–RNB3, RNF1–RNF6,
RNF71S–RNF74S), with `FIELDS_NOTES.md` (power fields) and
`SAM_FORMAT_NOTES.md` (text streams, `[pg]`).

**Live state** is `docs/NOTES_PROGRESS.md`.

**Goal.** Notes, bookmarks (text, cell ranges, one cell, frames),
footnotes (in the main text and in table cells), and endnotes are typed
in the model and native v3, read from and written to SAM as Ami Pro
writes them, laid out and drawn as Ami Pro lays them out (footnote areas,
separators, splits, continued messages, the referencing line moving,
endnotes, per-page numbering), edited on the page and in three rail
panels (Notes, Bookmarks, Footnotes), carried into print, PDF, and the
exports, and used by power fields and cross-references; every command is
one undo step; Ami Pro opens the app's files as its own (twins).

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/NOTES_PROGRESS.md`,
   `docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`, and
   `docs/Design/UI_SPEC.md` §34; then `git status` and
   `git log --oneline -15`.
2. The tag `notes-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **NR1** Notes: Ami Pro's marks in the text; a rail Notes panel (every
  note with initials, number, date, page, colour; edited in place;
  Previous/Next, Remove, Remove All, undoable; the user's initials and
  colour at the top); a click on a mark opens it.
- **NR2** Plain-text notes; a stored note's formatting kept until edited.
- **NR3** Print › With notes as Ami Pro; the PDF the same plus a text
  annotation per note; DOCX, ODT, RTF carry notes as comments.
- **NR4** A rail Bookmarks panel (Add or Go To, sorted list with kind and
  page, Rename, Remove); Edit › Go To's Next item gains Bookmark, Note,
  Footnote Mark, Footnote Text, with Previous.
- **NR5** View › Bookmarks, off by default, saved; never printed.
- **NR6** Insert › Cross-Reference… (PageRef or Bookmark power fields);
  PageRef, Bookmark, GetBookMark, Include file!bookmark work out; PDF
  named destinations and links; DOCX and ODT bookmarks and fields.
- **NR7** Ami Pro's name rules: no spaces, at most 19 characters, no
  leading digit, unique whatever the case; the reason shown.
- **NR8** Footnotes in place on the page, laid out as Ami Pro's; Insert ›
  Footnote (Ctrl+Alt+F); Esc back; double-click jumps.
- **NR9** Endnotes one switch for the document; Insert › Endnote leaves
  the menu.
- **NR10** A rail Footnotes panel (Tools › Footnote Options…), live, Ami
  Pro's dialog rules, Number style…, Previous/Next.
- **Choices made in the run** are labelled NK1, NK2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** rail inspectors over modal dialogs;
  consistent conventions (check marks on every on/off item); the colour
  button (§23); fields (FL1–FL7: the Fields panel, Show Power Fields,
  Update All before printing); print (PO1–PO6) and export (EX1–EX10)
  pipelines; import (IM1–IM12: comments as notes); SAM sources protected;
  Save As SAM lists what Ami Pro cannot hold.

## 3. Working rules

- **Each thing has one model.** A note is an inline object owning a flow
  (its paragraphs) and its fields (time, initials, colour, the main flag;
  the number is computed, the stored one kept only as read); a footnote
  is an inline object owning a flow (number computed); a table footnote
  is an inline object naming its row; a text bookmark is a pair of
  zero-width objects, a cell range one object in its cell, a one-cell
  bookmark a document record (table, row, column), a frame bookmark the
  frame's name; `FootnoteOptions` belong to the document. Floating headers
  are the pattern (an object owning a flow).
- **Ami Pro's bytes are the oracle.** Files Ami Pro wrote read back equal
  (the round trip), and their `[pg]` caches give each page end's main
  text and footnote position (RNF5, RNF7*): the layout comparison checks
  both.
- **Every command is one `amipro_editing::Command`**, applied to a new
  root; tested through the app's message path and the widget path (real
  clicks and keys through the `Runtime`).
- The evidence first: a rule not in the notes is looked up in the
  decompile or made as a VM fixture before it is built.
- The pinned libcosmic's traps (`CLAUDE.md`): no zero-size children in a
  row or column; an operation-only task chains a repaint; never an id that
  changes at a fixed place; no `context_menu` inside a scrollable; a text
  field focused by a task hears no focus message; **tests click portal
  commands with `Runtime::click_unrun`** so no real dialog opens on the
  user's screen; tests never touch the user's settings.
- Small green commits: targeted tests, strict Clippy, rustfmt, a commit
  per slice. Routine full suites, Clippy, fmt, and corpus measures run
  through lower-cost subagents (or background scripts) that report back.
- Aggregate numbers only, never the corpora's file names or contents;
  never Ami Pro's code, icons, or art.
- **Text that moves is measured.** The round-trip audit and
  `layout_comparison_report` (`target/notes-baseline/baseline.sh`) run at
  the start and the exit, and before and after any slice that touches the
  layout, model, or SAM crates; every changed row is explained.
  Footnotes move text at page ends: the largest layout change in this
  playbook.

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour, is recorded (NK…), and goes into the handoff. If the run grows
past one long session, hand off at a slice boundary with the progress file
current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| NB.1 Setup | `NOTES_BOOKMARKS_FOOTNOTES.md`, `UI_SPEC.md` §34, NR1–NR10, this runbook, the progress file, the baseline (`target/notes-baseline/`), the tag | Committed; `notes-run-start` |
| NB.2 Model and native | `InlineObject::Note` (fields, flow), `Footnote` (flow), `TableFootnote` (row), `BookmarkStart`/`BookmarkEnd`, `CellBookmark` (rows, columns); `Document::cell_bookmarks`, frame names as bookmarks, `FootnoteOptions`; validation; native v3 records | Unit tests; native round trip |
| NB.3 SAM in and out | The importer makes the typed objects (both `<:N` forms, `<:F`, `<:x`, `<:Z`, `<:Zr,c;`, `[book]`, `[fopts]`, the Footnote style); the writer writes them as Ami Pro does | The round trip equal to the start on all three corpora (now typed); Save As lists nothing new; RN* fixtures read back |
| NB.4 Layout | Footnote areas (one full-width area per page from the bottom margin, the 100-twip separator, lines after their reference, the referencing line moving when the separator and a line do not fit, splits with the continued messages and the repeated number, pending footnotes on pages of their own), endnotes after the text on a new page, numbering (start, reset per page, `*` and `†` per page, table footnotes within the table), note marks (no width) and initials text, footnote positions in `layout_comparison_report` from `[pg]` | Rule tests from RNF1–RNF7*; the layout comparison before and after, every changed row explained |
| NB.5 The page surface | Note marks, the footnote area and separator drawn; the caret and selections in footnote flows (in place; Up/Down between footnotes; a selection in one footnote), hit testing, Esc back, double-click jumps; View › Notes, Note Initials, Bookmarks drawing | Widget-path tests; screenshots light and dark |
| NB.6 Commands | `amipro_editing::references`: insert, edit, remove, Remove All notes; add, remove, rename, go to bookmarks (four kinds, Ami Pro's rules, no nesting); insert footnotes (main text; a cell adds the joined row), delete with the reference, `SetFootnoteOptions`, Make endnotes | Unit tests, undo by root; SAM after edits read back |
| NB.7 Notes | Insert › Note (Ctrl+Alt+M), the Notes panel, the user's initials and colour (settings), the delete confirmation, the status bar | Message-path and widget-path tests; screenshots |
| NB.8 Bookmarks | Edit › Bookmarks… and the panel, View › Bookmarks, Edit › Go To's Next item kinds with Previous | Tests as above |
| NB.9 Footnotes | Insert › Footnote, Tools › Footnote Options… and the Footnotes panel, Insert › Endnote removed, the reference's right-click items, the delete confirmation | Tests as above |
| NB.10 Fields and cross-references | `FieldHost` bookmarks: Bookmark, PageRef, GetBookMark, Include file!bookmark; Insert › Cross-Reference…; Find's footnote stream and word count by Ami Pro's rules | Field tests; Ami Pro's results where fixtures have them |
| NB.11 Output | Print Options › With notes (marks and end pages), the PDF (the same, annotations, named destinations, links), exports (notes as comments, footnotes and endnotes as the formats' own, bookmarks), import (comments to typed notes, footnotes to typed footnotes) | Tests; LibreOffice renders exports headless with a private profile |
| NB.12 Measures | The round trip and layout comparison against the start; release timings (a keystroke in a footnote, a page with footnotes relaid, the panels); the typing A/B | Numbers in the progress file |
| NB.13 Twins | The app makes notes, bookmarks of each kind, footnotes (options, a split, a table footnote, endnotes) and writes SAM; Ami Pro opens them as the app shows them | VM screenshots outside Git |
| NB.14 Exit | `tools/verify.sh --release --corpus`, the measures, the records (`UI_SPEC.md` §34 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue and `FIELDS_AND_REFERENCES.md`), the handoff, the push | Green; `origin/master` equals `HEAD` |
