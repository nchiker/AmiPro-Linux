# CLAUDE.md — AmiPro-Linux

`AGENTS.md` governs how to work (autonomy, proprietary material, the three verification layers, acceptance vocabulary, UI slice gates). Read it at session start. This file is current state only. History is in `docs/HISTORY.md`, specialised test commands in `docs/TEST_COMMANDS.md`, the current handoff in `docs/SESSION_HANDOFF.md` (older ones in `docs/HANDOFF_ARCHIVE.md`).

## Source-of-truth documents

- `SPEC.md` (requirements), `PROJECT_PLAN.md` (sequencing), `FEATURE_PARITY.md` (evidence per feature), `DECISIONS.md` (decision log), `PROJECT_STATUS.md`.
- `docs/PHASE5_PLAYBOOK.md`, `docs/playbooks/`: the run recipe and Ami Pro 3.1's inventory. `docs/SPEEDUP_PLAN.md`: runs A–D (SU1–SU4).
- `docs/Design/UI_SPEC.md` (the approved design; one section per area), `UI_MOCKUP.svg`, `UI_COMMAND_MATRIX.md`, `UI_ACCEPTANCE.md`.
- `docs/research/*`: evidence. Format specs: `SAM_PAGE_FRAME_SPEC.md`, `SAM_FORMAT_NOTES.md`, `PHASE1_FORMAT_AUDIT.md`; each run has its notes there.
- Each run: `docs/*_RUNBOOK.md` and `docs/*_PROGRESS.md`.

## State

- Phases 1–4 done: model, pages and frames, styles, the SAM writer.
- Phase 5 runs done, each physically checked: pictures, tables across pages, Find & Replace, fields, print and PDF, export, import, table editing, drawing, notes/bookmarks/footnotes, proofing, TOC/index/glossary, master documents, merge.
- Run A (commands, CM1–CM12) done (`docs/COMMANDS_PROGRESS.md`): line numbering (`[lnopts]`, `DocumentLayout::line_numbers`) and password-protected files (`amipro_formats::protect`; a session's `password` makes every SAM save protected). Run B (outline mode, OM1–OM4; `docs/OUTLINE_MODE_RUNBOOK.md`) done, physically checked 2026-11-06: Move Up/Down, the Outline mode view, Expand/Contract/Show levels and the bar, Use Outline Styles, the Outline navigator. DF1 (a Documents folder in Preferences › Folders; `file_dialogs.rs`) and MD1–MD3 (Markdown as an exchange format) are done. Run C (table data, TD.1–TD.13; `docs/TABLE_DATA_PROGRESS.md`): formulas (`crates/document/src/formula.rs`), Quick Add, number formats shown through the style, Tools › Sort…, merge numbers, forms scenario; awaiting the physical check. Run D (objects, DO.1–DO.12; `docs/OBJECTS_PROGRESS.md`) done, awaiting the physical check: OLE objects drawn and kept (Frame Properties › Object, Convert to Picture, Edit › Links…), Paste Special, Image Processing, charts (the Charting panel, Chart from Table), equations (card, palette, LaTeX paste). No run E is planned. No revision marking.
- P6.1 `.deb` (`tools/package-deb.sh`, `docs/INSTALL.md`). P6.2 Help › Check for Updates… (`updates.rs` logic and `Backend` trait, `updates_panel.rs`, `updates_commands.rs`; `UI_SPEC.md` §43; `docs/RELEASING.md`, `tools/release.sh`): GitHub Releases, SHA-256 checked, `pkexec apt-get install -y`; automated checks pass, physical check against a later release pending (v0.1.0 is published).
- P6.3 crash recovery (`recovery.rs`, `recovery_commands.rs`, `recovery_panel.rs`; `UI_SPEC.md` §44): timed copies of unsaved documents, File › Recover Documents…; automated checks pass, physical check (kill -9) pending.
- The repository is public (MPL-2.0 `LICENSE`, fresh one-commit history, 2026-11-07); release v0.1.0 is published. Also built since run D: File › Page Setup… (opens Page Layout), View › Status Bar toggle, First line bold (style attribute, bit 4096 of `[fnt]`), the status bar Face/Size fix; Fast Format, Face/Size lists, inserted rulers and Line Numbering are built. Phase 6 left: fuzzing, large-document tests, the compatibility matrix; `docs/KNOWN_LIMITATIONS.md` is the user-facing list and must be kept current.
- Save writes over a `.SAM` after the loss check, keeping `NAME.SAM.bak` the first time; imports and STY files are never written over. A run of typing undoes as one step (SU4: `TypingRun` in `main.rs`, `History::changes`).
- Help (P6.4, `UI_SPEC.md` §45): own Markdown topics in `app/cosmic/help/<area>/<topic>.md`, embedded by `build.rs`; `help.rs` (parse, search, context, history), `help_panel.rs` (rail panel, F1 contexts, the coverage test and its exemption list), `docs/HELP_AUTHORING.md` (original wording only). A new panel or menu command needs a topic context or a line in the exemption list.
- Every run adds its area's scenarios to `app/cosmic/src/scenario_tests/` (one module per area, one checker after every step); they pass before the user's physical check.

## Repository layout

No top-level `src/`. A dependency-free core Cargo workspace (edition 2024) plus a separately manifested COSMIC app.

| Path | Crate | Role |
|---|---|---|
| `crates/document` | `amipro-document` | The model: `Document`, typed IDs, flows, inline objects, page layouts, frames, tables, styles, fields, contents, merge settings, validation, native v3 |
| `crates/editing` | `amipro-editing` | `Command`s over `TextPosition`, `apply`, `History` (100 steps); find, tables, lists, outline, fields, references, proofing, contents, master, merge, drawing |
| `crates/ami-formats` | `amipro-formats` | SAM/STY reader and writer, loss check (`sam_losses`), style sheets, glossary; `oracle.rs` reads Ami Pro's caches for comparison only |
| `crates/layout` | `amipro-layout` | The layout engine: `Shaper` trait, line breaker, pages, frames, tables across pages, footnotes, numbering, hyphenation, master sets |
| `crates/pictures` | `amipro-pictures` | Readers of stored pictures (DIB, WMF, EMF, screen images), Ami Pro's drawings (`sdw`, `svg`), OLE objects (`ole`), image adjustment (`adjust`), charts (`chart`, `chart_gen`), equations (`equation`) |
| `crates/exchange` | `amipro-exchange` | Export and import: DOCX, ODT, RTF, Markdown, text, XLSX/ODS sheets; zip and XML readers; the neutral exchange document |
| `app/cosmic` | `amipro-cosmic` | libcosmic app (rev `dbc62f2`, cosmic-text 0.19); own `Cargo.lock`, excluded from the workspace |
| `tools/` | n/a | ISO extractor, `ghidra/`, `vm/`, `verify.sh`, `corpus.sh`, `bench-ab.sh`, `package-deb.sh`, `release.sh` (`experiments/` is not adopted) |
| `Reference/` | n/a | **Gitignored** private ISO and samples. Never commit or embed |

`app/cosmic/src/` by area (the normal build is the `page-surface` feature; `UI_SPEC.md` has a section per area):

- Shell: `main.rs` (app, sessions, update/view, most tests), `command.rs` (the command registry, menus, shortcuts), `chrome.rs`, `file_workflow.rs`, `lengths.rs`, `workspace_geometry.rs`.
- Page surface: `page_surface.rs`, `page_view.rs`, `page_editing.rs`, `page_commands.rs`, `page_preview.rs`, `view_modes.rs`, `page_layout.rs` (COSMIC Text `Shaper`, `SessionPages`), `page_dialogs.rs`, `page_layout_properties.rs`, `ruler.rs`, `ruler_view.rs`, `frame_editing.rs`, `frame_inspector.rs`.
- Styles and text: `style_dialogs.rs`, `style_properties.rs`, `character_dialogs.rs`, `colours.rs`, `colour_control.rs`.
- Objects: `chart_commands.rs`, `chart_panel.rs`, `equation_commands.rs`, `equation_panel.rs`, `paste_special.rs`, `links_panel.rs`.
- Commands and views: `fast_format.rs`, `paragraph_properties.rs`, `special_character.rs`, `outline_navigator.rs`, `outline_view.rs`, `preferences_dialog.rs`, `new_document_dialog.rs`, `password_dialog.rs`, `revert_saved.rs`, `sort_commands.rs`, `sort_panel.rs`, `help_drawers.rs`, `rail_panel_tests.rs`.
- Release: `updates*.rs`, `recovery*.rs`.
- Pictures and drawing: `pictures.rs`, `metafile_render.rs`, `picture_insert.rs`, `picture_inspector.rs`, `draw_*.rs`, `drawing_properties.rs`.
- Tables: `table_*.rs`.
- Objects: `equation_*.rs`, `chart_*.rs`, `paste_special.rs`, `rail_panels.rs`.
- Fields, notes, contents: `field_commands.rs`, `fields_panel.rs`, `document_properties.rs`, `clock.rs`, `reference_*.rs`, `references_panel.rs`, `contents_*.rs`, `glossary_*.rs`.
- Find and proofing: `find_bar.rs`, `find_set.rs`, `spelling.rs`, `grammar.rs`, `proofing_*.rs`.
- Help: `help.rs`, `help_panel.rs`, `help_drawers.rs`; topics in `app/cosmic/help/`.
- Output and exchange: `pdf.rs`, `pdf_pictures.rs`, `print.rs`, `print_portal.rs`, `print_panel.rs`, `export*.rs`, `import*.rs`.
- Master and merge: `master*.rs`, `merge*.rs`, `data_file.rs`, `envelope_panel.rs`.
- Tests, prototypes: `page_surface_app_tests.rs` (`Runtime` harness), `scenario_tests/`, `layout_comparison.rs`, `document_highlighter.rs`, `rich_*.rs` (not default).

Builds: normal (page surface), `--no-default-features` (text editor), `rich-editor-prototype`.

## Build and test

```sh
cargo test --workspace --offline                                              # core
cargo clippy --workspace --all-targets -- -D warnings
cargo test --offline --manifest-path app/cosmic/Cargo.toml                    # normal build
cargo test --offline --no-default-features --manifest-path app/cosmic/Cargo.toml   # text editor
cargo test --offline --all-features --manifest-path app/cosmic/Cargo.toml
cargo test --offline --manifest-path app/cosmic/Cargo.toml scenarios::        # the scenario suite (about 35 s in debug)
cargo clippy --offline --manifest-path app/cosmic/Cargo.toml --all-features --all-targets -- -D warnings
cargo run --offline --manifest-path app/cosmic/Cargo.toml                     # the app
```

- Last counts (2026-11-07): core 802 (+30 ignored); COSMIC 741 default, 128 text editor, 748 all features (85, 9, 85 ignored). Run the test steps with `AMIPRO_STYLES` unset (the style-sheet tests count sheets); set it for corpus and layout runs only.
- `tools/verify.sh [--quick] [--release] [--corpus]`: fmt, Clippy, tests (logs in `target/verify/`). `tools/corpus.sh`: corpus audit. `tools/bench-ab.sh BASE_REV [ROUNDS]`: release A/B.
- The ignored twin, benchmark, census, and comparison commands are in `docs/TEST_COMMANDS.md`.
- Benchmark caution: the host runs other workloads (governor `powersave`); compare with interleaved A/B runs, not old numbers.

## Watch for this

UI toolkit (the pinned iced and libcosmic):
- `responsive(...)` in a column defaults to `height: Fill`; give it an explicit `.height(...)` (it once took 322 px of 800).
- A task that only runs a widget operation (`scroll_to`, focus) is applied without a redraw. Scroll through `scroll_workspace` (it chains a no-op `Message::Repaint`); do the same for any new operation-only task. `AMIPRO_TRACE_SCROLL=FILE` logs scrolling.
- The tree diff keeps the id a scrollable (or any widget with id sets) first had at its place. Never give a widget at a fixed place an id that changes (per tab, per document). The page workspace uses one id, `WORKSPACE_SCROLL`; `update` restores each tab's offset (`follow_active_session`).
- Never put libcosmic's in-window `context_menu` inside a scrollable: it records the click in content coordinates and compares the window's pointer. The page menu wraps the workspace's scrollable; the Styles panel has one menu around its list (`RailState::context_style`).
- `Row`, `Column`, and `Stack` drop a child whose size hint is `Fixed(0.0)` (`Size::is_void`), shifting siblings and rebuilding them. Use empty slots that shrink (`space().height(Fill)`), never `width(0)`.
- Icon-button style ignores `selected` on an enabled button; toggles use `toggle_button_class`. Icons come from the COSMIC theme (Pop and hicolor fallback), not Adwaita: check a new name exists.
- libcosmic's text buttons give `width(...)` to their label's row, not the button; a full-width button is custom (`print_panel::wide_button`).
- libcosmic panics in `menu_inner` when the headless harness opens the menu bar, so a menu's look is checked physically. An opened in-window menu lays out its items only on the next event with the pointer on the click's pixel: the harness (`window_screenshots`, `render_window_after`) sends a move there, and `Runtime::click` ends with a redraw. The harness must draw what the app sets (the header title).
- A text field focused by a task gets no `on_focus` (only a click sends one); the find bar records what it focuses.
- A nested `with_layer` draws after its parent layer's primitives, so a frame's lines go in a layer after the picture's. COSMIC Text centres a line's text in a taller line; the page surface moves text in Flow with text frames onto the baseline (`text_shift`).
- A clipboard read sent as the window gains focus can find nothing (smithay-clipboard learns of the focus from its own keyboard events). Paste always reads the clipboard, the right button looks again, and an empty first look retries after 150 ms (`retry_clipboard`). Paste also looks for an image when the text is only an address or path (`picture_insert::link_text`). Paste on the page surface reads RTF first (`read_data`), then text.

Tests and harnesses:
- The test `Runtime` runs each message's task to completion. A click or `send` reaching Print…, Print Current Page…, Export PDF…, Open, Save As, or Insert › Image… opens the real portal dialog on the user's screen and blocks. Use `Runtime::click_unrun`, which returns the messages. Tests must never open real windows or dialogs.
- Because of that, timing between messages (a pause made stale by later typing, the order of a widget's messages) never shows in tests; the spelling fixes were found with `AMIPRO_TRACE_SPELL=FILE`. The as-you-type pause is not scheduled without a dictionary (default `Spelling`); tests using `test_spelling()` wait 350 ms a message; Harper takes seconds to make in debug (tests share `test_grammar()`).
- Headless scrolling tests follow the runtime's order: update, rebuild from the cache, then run the operations. The headless harnesses place the caret by position, so they miss click-placement defects; the live app was driven on a private rootful Xwayland (`Xwayland :7`, app with `DISPLAY=:7` and no `WAYLAND_DISPLAY`, python-xlib XTEST after `set_input_focus`), which reaches only that display.
- Tests use no settings or a scratch directory (glossary: `scratch_glossary`), never the user's configuration.
- Save As SAM asks before writing over a file, so a twin test or scenario removes the old file first (`Scenario::clean_folder`).
- Keys sent through the window reach the page only once it holds the keyboard: a click, or `app.page_keyboard += 1` in a test.
- The view asks for the field at the caret several times a frame; the session's `fields` (from the last change) answers it.

Layout and model:
- A document with a footnote in its main text skips the incremental layout (NK2).
- A table has one frame box per part, possibly two on one page (columns); look a cell up in the part whose box for it is not empty.
- A click in a table's cell puts the caret there (TK6), so a page table is never selected by a click.
- Text typed after a power field goes after it (MC11); hidden header text keeps the caret at the box's last visible line (`PageSurface::shown_end`).

Output:
- krilla fills a path black when neither fill nor stroke is set; never draw a path with both `None`.
- The GTK print portal applies copies, collate, reverse, and odd or even pages through CUPS but never the page range; the app sends only the chosen pages.
- LibreOffice ignores a picture crop past the picture (a larger frame is a box holding it), reads header ODF automatic styles only from `styles.xml`, and shrinks an RTF rounded rectangle with text. Ami Pro's RTF reader takes `\b0` for on (runs are `{\plain …}` with only what is on). Run LibreOffice headless with a private profile under `target/` (`-env:UserInstallation=file://…`).

## Ami Pro's document model

Page- and frame-based, not a Word clone.

- Each page has one page layout (margins, columns, tabs, header/footer format); inserted layouts change it mid-document.
- A frame is a mini-document with its own layout; it holds exactly one of text, a table, or a picture. Pictures and drawings must be in frames.
- Placement: Where placed, With para above, Flow with text, Repeat all pages. Where placed frames paint above anchored ones. Wrap: Wrap around, No wrap around, No wrap beside.
- Rules: `docs/research/SAM_PAGE_FRAME_SPEC.md`, `SAM_FORMAT_NOTES.md`. Model decisions D1–D8: `DECISIONS.md`, `ARCHITECTURE.md`.

## Reference resources on this machine (read-only; never in Git)

- `../AmiPro-Reference/`: `Binaries/` (the full Ami Pro 3.1 install set, 662 entries), `vm-lab/` (VM fixtures in `OUT/`, inputs in `IN/`), `ghidra-work/` (project and `out/*.c` decompiler output), `Charlie-Sample/` (the user's corpus), `Wine-Test/` (do not run reference binaries on the host).
- Decompiled output (`ghidra-work/out/*.c`, outside Git): `AMIPRO.EXE`, `AMIPROUI.DLL`, `AMIUICNT.DLL`, the drawing, chart, equation (half: 48 segments missing), image, proofing, `rtf` and `AMIXIMP` modules; `AMIMACRO.DLL` is not decompiled; method in `docs/research/DECOMPILATION.md`. Primary evidence; implement independently.
- Ghidra 12.1.3 is a Flatpak (`analyzeHeadless`; grant `--filesystem=` for outside paths). Stock Ghidra cannot import `AMIPRO.EXE`: `tools/ghidra/build-ne-unsigned-fix.sh`, `optloader-unpack.py`; menu commands `command_table.py`, `DecompAt.java`; resources `tools/ne_resources.py`.
- VirtualBox `"Windows XP"` has Ami Pro 3.1 and is the behavioural reference; the agent may operate it, including snapshots (baseline: `Clean baseline (before AmiPro-Linux lab)`). Procedure: `docs/research/VM_LAB.md`; helpers `tools/vm/`. Files move through `L:\IN` and `L:\OUT`; Ami Pro needs 8.3 names. Leave `"Windows11"` alone.
- Ami Pro's help files (`Binaries/*.hlp`: `amipro`, `helppack`, `amimacro`, `amichart`, `amidraw`, `amieqn`, `amiimage`, `amifm`) are not parsed yet; they are the planned oracle for unbuilt dialogs and the topic map for a packaged Help (rewrite in our own words, never copy).
- The user's corpus: `../AmiPro-Reference/Charlie-Sample/SAM` (423 SAM files) and `STYLES` (192 STY files), plus Word Pro `.LWP` files. Always set `AMIPRO_STYLES` to its `STYLES` folder. Never commit its content or its file names. The ISO corpus is extracted to `/tmp/amipro-phase1-corpus` by `tools/reference_inventory.py`.

## Notes for working here

- `Reference/` inside the repo is gitignored; read-only inspection only; never commit its contents or derived bytes.
- `AGENTS.md.backup` is an untracked user file. Do not modify or delete it.
- Backup: push to `origin` (`https://github.com/nchiker/AmiPro-Linux`, branch `master`) at the end of every session, after the handoff commit (`git push origin master`), and confirm `origin/master` equals `HEAD`. The repository is public (2026-11-07, user decision, after a fresh-history publication audit): never commit proprietary or corpus material, file names from the user's corpus, or personal data. Do not push mid-session unless asked.
- `../AmiPro-Reference` is never pushed; the user backs it up.

## Known open items

- The page surface has no accessibility nodes for page text, and visible IME preedit has not been checked physically.
- UI-1 checklist cases and the UI-2 tab tooltip and accessibility node remain open; Ctrl+Tab is intercepted before the app (Next Document is Ctrl+PageDown).
- Layout open (VM FG3, FG5): a word wider than a gap beside a frame goes below it (Ami Pro breaks it there); a Wrap around frame beside a column's first lines adds the space above in Ami Pro even with When not at break.
- Not implemented: Window › New View and View › Vertical Ruler (CK11), Draft's frame markers (CK10), a packaged Help (F1; mining Ami Pro's `.hlp` files is the plan). Not planned: revision marking and Doc Compare. Deferred: macros. Decided: Printer Setup (the print portal), no live OLE editing, Paste Link, or New Object, no Window › Tile/Cascade (tabs). Open from the merge run: Ami Pro's date parser (MC4), dBASE memo fields.
