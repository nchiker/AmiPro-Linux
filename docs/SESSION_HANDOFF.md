# Session Handoff

**Updated:** 2026-11-22, after "open from outside" (files named on the command
line or handed to a running window). `PROJECT_PLAN.md` is authoritative for
sequencing. The previous handoff (P6.1-P6.3, the public release) is in
`docs/HANDOFF_ARCHIVE.md`; the open items in it still stand unless noted here.

## What was built (commits `90a5767`, `8292558`; pushed)

- `amipro-linux FILE...` opens files (relative or absolute; `file://` also).
  With a window running, the second launch hands the files over by libcosmic's
  single-instance D-Bus activation (`run_single_instance`; name
  `org.amipro_linux.AmiPro`, path `/org/amipro_linux/AmiPro`) and exits. The
  window gets `ActivateAction("open", absolute paths)` or `Open(file:// uris)`,
  both becoming `Message::OpenPaths` (`app/cosmic/src/launch.rs`,
  `AmiProApp::dbus_activation`, `open_named_paths`). Files open as File >
  Open does (new tabs; an open file is activated; other tabs' unsaved changes
  are untouched); an untouched first "Untitled" gives way to the first file.
- A file that cannot be opened (missing, unreadable, a folder, damaged) shows
  the `open_problem` dialog (title and reason, OK). The status bar says it too.
- Dependencies: libcosmic feature `single-instance`, crate `url`.
- Packaging needed no change: the desktop entry already had `Exec=amipro-linux
  %F` and the Ami Pro MIME types. `docs/INSTALL.md` has the section "Opening
  files from the command line and from other programs"; `COSMIC_SINGLE_INSTANCE=0`
  starts a separate window.
- Requested by the Stack session for its "Open in Ami Pro"; the Stack session
  was told (2026-11-22). Its command: `amipro-linux /full/path/to/file.sam`.

## Checked

- Automated: unit tests in `launch.rs`, scenarios in
  `scenario_tests/launch.rs` (named file replaces the untouched first tab; a
  file into a window with unsaved work; same file again; missing, folder and
  damaged files; several files). COSMIC default 750 pass (85 ignored), text
  editor 133 pass (9 ignored). Core workspace not touched.
- Layer 2/3: D-Bus handoff driven on a private Xwayland (second launch from
  another folder, `Open` with a percent-encoded `file://`, missing file
  dialog). Physical (2026-11-22, installed `.deb` 0.1.1+git90a5767): command
  line, second launch into the running window, file manager Open with: all
  worked.
- The scenario tests do not cover the D-Bus transport itself (libcosmic's).

## Open

- `cargo clippy -D warnings` fails in `page_surface.rs:3503` (duplicated
  `allow` attribute) and `find_bar.rs:346` (needless range loop), files this
  work did not touch; not compared against the earlier commit. Fix in a
  separate commit.
- Nothing is released or tagged. The installed package is a local build of
  `90a5767` (version 0.1.1+git90a5767); v0.1.1 on GitHub lacks this feature.
- Decided not to: add DOCX/ODT/RTF/Markdown to the desktop MimeType list (would
  make the app a default candidate), or set `DBusActivatable=true` (needs a
  D-Bus service file; the handoff already covers it).
- Flatpak (Stack's slice 6b) can start from `90a5767`. The `.deb` and a Flatpak
  share the D-Bus name, so run only one at a time (remove the `.deb` for a
  clean test).
- Counts for CLAUDE.md are still the 2026-11-07 ones; default is now 750.

## Physical checks outstanding (unchanged)

1. Run C: the Table data checklist in `docs/Design/UI_ACCEPTANCE.md`.
2. Run D: the Objects checklist.
3. Crash recovery: kill -9 with unsaved changes, restart, File > Recover Documents....
4. First line bold against the VM picture `flb_after.png`; the status bar's Face
   and Size lists after the SetFontParts fix.
