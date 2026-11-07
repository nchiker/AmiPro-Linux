# Session Handoff

**Updated:** 2026-11-07, after P6.1-P6.3, the public release, Page Setup, the
Status Bar toggle and First line bold. `PROJECT_PLAN.md` is authoritative for
sequencing. The previous handoff (runs B, C, D) is in `docs/HANDOFF_ARCHIVE.md`.

## State

- Phase 5 queue complete (runs A-D built). Runs C (table data) and D
  (objects) await the user's physical check; A and B passed (2026-11-05,
  2026-11-06).
- Phase 6: P6.1 `.deb` (`tools/package-deb.sh`, `docs/INSTALL.md`), P6.2
  Help > Check for Updates... (`docs/RELEASING.md`, `tools/release.sh`;
  `UI_SPEC.md` section 43), P6.3 crash recovery (File > Recover Documents...;
  section 44).
- The repository is public (MPL-2.0 `LICENSE`, fresh one-commit history after
  an audit, 2026-11-07); release v0.1.0 is published.
- Also built: File > Page Setup... (opens Page Layout), View > Status Bar
  toggle (the dead Section Break and Move Document to New Window entries are
  gone), the status bar Face/Size fix (`SetFontParts`: it no longer resets other
  font parts), First line bold (a style attribute, bit 4096 of `[fnt]`;
  `docs/research/FIRST_LINE_BOLD_NOTES.md`).
- New scenario modules: `drawing`, `crash_recovery`, `update_check`.
- Counts (2026-11-07, `AMIPRO_STYLES` unset): core 802 (+30 ignored), COSMIC
  714 default, 128 text editor, 721 all features (85, 9, 85 ignored).
- Docs brought up to date 2026-11-07 (CLAUDE.md, PROJECT_PLAN, PROJECT_STATUS,
  FEATURE_PARITY, INSTALL, UI_COMMAND_MATRIX, playbooks, UI_SPEC notes); the
  user-facing list of gaps is `docs/KNOWN_LIMITATIONS.md` (keep it current).

## Physical checks outstanding (ranked for the user)

1. Run C: the Table data checklist in `docs/Design/UI_ACCEPTANCE.md`
   (formulas, Quick Add, number formats, Sort, merge numbers).
2. Run D: the Objects checklist (OLE section and Convert, Edit > Links...,
   Paste Special, Image Processing, Charting and Chart from Table, Equations).
3. Crash recovery: kill -9 the app with unsaved changes, restart, File >
   Recover Documents....
4. Update check against a later release (v0.1.0 is the only release, so the
   "newer version" path has not been seen on a real machine).
5. First line bold against the VM picture `flb_after.png`; the status bar's
   Face and Size lists after the SetFontParts fix.
6. Markdown (open, insert, export, Save As), the Documents folder
   (Preferences > Folders), Esc in dialogs, the rail across the 1200 px
   docking width.
7. IME preedit with a real input method.

## Open items

- Window > New View and the Vertical Ruler (CK11); Draft's frame markers
  (CK10); no packaged Help (F1).
- Layout: FG3 (a word wider than a gap beside a frame) and FG5 (a Wrap around
  frame beside a column's first lines); `_FAX3`'s table height (FG5/FG6).
- Accessibility nodes for page text: blocked by the pinned iced_winit.
- Graphics Scaling once opened nothing in the rail (2026-11-06, not
  reproduced); if it returns note the status-bar text and whether the item
  was greyed.
- Equations and charts: fitted layout, about 330 of 887 symbols, CHV4 not run,
  no Pictures dialog or colour reorder (`OBJECTS_PROGRESS.md`).
- Table: Paste and Cut into a formula cell are not refused; Sort's DDE and
  too-large refusals (TD.7). Merge: date parser (MC4), dBASE memo fields.
- Not planned or decided: revision marking, Doc Compare, macros (deferred),
  Printer Setup, live OLE editing, Paste Link, New Object, Tile/Cascade.

## Next steps

1. Phase 6 remaining: fuzzing of the readers, large-document tests, the
   compatibility matrix (then update `docs/KNOWN_LIMITATIONS.md`).
2. CK10 and CK11 (Draft frame markers; New View and the Vertical Ruler).
3. Help: mine Ami Pro's `.hlp` files (`amipro.hlp` first) as a read-only text
   dump for a behaviour oracle and a topic map for F1 Help; write the content
   in our own words.
4. Accessibility stays blocked until the toolkit forwards widget trees.

Gate note: run the test steps with `AMIPRO_STYLES` unset; set it only for
corpus, layout and benchmark runs. Push at the end of a session after the
handoff commit and confirm `origin/master` equals `HEAD`.

Prompt for the next session:

```text
Read AGENTS.md, CLAUDE.md, and docs/SESSION_HANDOFF.md, then do Phase 6's
remaining items (fuzzing, large-document tests, the compatibility matrix) as a
straight run, keeping docs/KNOWN_LIMITATIONS.md current. Every change is one
undo step; scenarios pass before any physical check. Use lower-cost subagents
for routine work and testing, keep decompiler output and my corpus file names
out of Git, commit small tested slices, and hand off with a short ranked list.
Tests must never open real desktop dialogs on my screen.
```
