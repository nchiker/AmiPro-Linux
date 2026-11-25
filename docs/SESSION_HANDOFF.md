# Session Handoff

**Updated:** 2026-11-25, after the UI work of 2026-11-25 (save type, drag
selection, vertical ruler, Format Painter, the UI and modal dialog reviews,
Preferences, opening zoom), its scenarios, and two follow-up fixes.
`PROJECT_PLAN.md` is authoritative for sequencing. The previous handoff (open
from outside) is in `docs/HANDOFF_ARCHIVE.md`.

## What was built (`39ef6c6`..`c70c6fd`)

- Preferences > General > Save new documents as (`UI_SPEC.md` section 46).
- Selecting past the window's edge auto-scrolls; a drag stays in the flow it
  started in, so a selection spans pages (section 47).
- View > Vertical Ruler with draggable top and bottom margins (section 48).
- Format Painter, Copy Format and Paste Format (Ctrl+Alt+C, Ctrl+Alt+V),
  Format > Fast Format submenu (section 49).
- UI review and modal dialog review (section 50): one rail panel header, menus
  regrouped, Preferences in three pages (Folders restyled), one key table for
  dialogs (Esc, Enter, first focus), Promote/Demote icons.
- Preferences > View > Open documents at; Custom zoom (91%) by default, taken
  by every new tab.
- `2229639`: during drag auto-scroll a redraw still asks for the input method
  (it did not).
- `d7f8bd2`: at fractional zoom the recorded scroll offset is the drawn (whole
  pixel) one (was 992.96 against 993).
- `c70c6fd`: scenarios `selecting`, `format_painter`, `vertical_ruler`,
  `preferences`, `dialogs`. They found one bug, fixed: Go To opened with the
  current page in its box and the cursor after it (Ctrl+G, 3, Enter asked for
  page 13). A dialog's first field is now selected when it opens.
- `tools/test-changed.sh` runs only the tests a change can affect.

## Checked (2026-11-25, code at `c70c6fd`, `AMIPRO_STYLES` unset)

- COSMIC default 784 pass (88 ignored); text editor 137 (9 ignored); all
  features 791 (88 ignored); core 802 (30 ignored).
- App Clippy (`--all-features --all-targets -D warnings`) and `cargo fmt
  --check` (core and app) pass.
- Core `cargo clippy --workspace --all-targets -- -D warnings` FAILS: two
  `needless_range_loop` errors in `crates/ami-formats/src/info.rs` (lines 291
  and 308), a file this work did not touch (rustc 1.100.0). Not fixed.
- Corpus audit, release build and VM comparison were not run.
- Layer 3: nothing newly accepted. The scenarios are headless widget and
  message tests. (The cross-page drag defect had been found on a private
  Xwayland before `1d66957`.)

## Physical checks: PENDING (ranked)

The installed package (version 0.1.2) predates all of this: build and install
with `tools/package-deb.sh`, or run `cargo run --offline --release
--manifest-path app/cosmic/Cargo.toml`.

1. Drag across pages: select from page 1 into page 2 over the footer and
   header; hold below and above the window (speed, stopping on release);
   Shift+click; copy and paste elsewhere.
2. Menus: Format > Fast Format, Edit > Go To, the File, View and Tools groups.
3. Preferences: the three pages, nothing overlapping at 900 x 640; Folders.
4. Format Painter: the toolbar button once and twice, the pointer and status
   line; Ctrl+Alt+C and Ctrl+Alt+V (does the desktop take them?); Esc.
5. View > Vertical Ruler: light and dark, drag both margins, restart.
6. Save As for each "Save new documents as" choice: the name, type and filter
   the desktop's chooser shows (only what it is given is tested).
7. Dialogs: Ctrl+G, a number, Enter; Ctrl+W on a changed document (Enter
   saves, Esc cancels); Enter does nothing in Delete Frame, Revert, Replace.
8. Promote and Demote icons beside the indent icons, light and dark.
9. Zoom: documents open at 91% (is that the right size?); change Open
   documents at; each tab keeps its zoom.

Still outstanding from before: the run C (table data) and run D (objects)
checklists in `docs/Design/UI_ACCEPTANCE.md`; crash recovery after `kill -9`;
First line bold against the VM picture; the status bar's Face and Size lists;
visible IME preedit with a real input method.

## Open

- The core Clippy failure above: fix in its own commit.
- Not pushed (this session was told not to): `origin/master` is at `4b944fa`.
  Push at the end of the session and confirm `origin/master` equals `HEAD`.
- Nothing is released or tagged for this work.
