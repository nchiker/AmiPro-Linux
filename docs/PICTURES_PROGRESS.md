# Pictures Progress

Live state of `docs/PICTURES_RUNBOOK.md`. Resume at the first slice that
is not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| PI.1 The map | DONE | `511a9b4` | `docs/research/PICTURE_NOTES.md`; VM PI1–PI9, PIR1–PIR5, T1; `UI_SPEC.md` §25; P1–P8 | Import Picture `10d8:126c`, `1260:0509`; Graphics Scaling `1168:0000`, `FRAMESCALEPROC`; the rectangle `1168:2dd2`; `[isd]` `11c8:2e29`; `AMIPAINT.DLL`, `AMIWMF.DLL`; the screen image `1168:2016`/`26e7` |
| PI.2 The picture model and native format | DONE | `b7f0ca9` | `pictures_are_sized_as_ami_pro_sizes_them_pi1_pi5_pi6_pi7` (`1168:2dd2` against PI1's 816 × 544, PI5, PI6, PI7; 22 in the cap), `kept_isd_values_give_the_scaling_and_pan_pi8` (936, 144; charts by their driver), `native_files_without_picture_scaling_read_it_from_the_kept_values`, the native round trip with every field; `a_picture_goes_into_an_empty_frame`, `a_picture_goes_into_an_empty_cell` (T3; TF12's refusal; deleted with its table), `replace_scale_move_and_describe_a_picture`; the random undo test with the five picture commands | `PictureScaling` (sizing, aspect, percent, size, rotation), `Offset` pan, description, `PictureFormat`; `Picture::kept_scaling` reads `[isd]` values 4–11 and the pan, `kept_is_current` says whether a writer may write them back; native `picturescaling` and `picturedescription` (`NATIVE_FORMAT_V3.md`); `InsertPicture` (an empty frame or cell), `ReplacePicture`, `SetPictureScaling`, `MovePictureInFrame`, `SetPictureDescription` (`crates/editing/src/pictures.rs`); `FrameDefaults::cell_frame_style` (`14d0:0000`). **Found and fixed:** Insert Layout copied a repeating table's cells but not their pictures, leaving one picture in two cells (the random test, once pictures could go into cells) |
| PI.3 Import, the SAM writer, and the loss check | DONE | `650ec4a` | `tests/sam_pictures.rs`: `a_stored_picture_is_written_as_read_until_it_changes_pi8` (as read; moved: Ami Pro's record from the model with the driver's state as read and no screen image; read back equal), `a_new_picture_is_written_as_ami_pro_writes_one_pir1` (PIR1's record exactly), `save_as_names_descriptions_conversions_and_linux_links`, `an_older_versions_record_is_written_as_read`; the round trip on all three folders writes every file byte for byte as at `pictures-run-start` (417/417, 159/160, 235/235; sections 51,216, 8,205, 24,101) | The importer types values 4–11 and the pan (out-of-range values made the nearest Ami Pro writes, with a warning); `write_isd` rebuilds a record whose kept values are stale, keeping the parts the model does not type (`kept_isd_parts`; a metafile keeps the driver's record of its size), and drops Ami Pro's screen image with it; the loss check's facts type the scaling (the stored size only for Custom), pan, and description, and count the screen image only with a current record; `LossKind::PictureDescriptions`, `ConvertedPictures` (by the kept driver: a linked TIFF Ami Pro converts on loading is a bitmap to it), `LinkedPictures` (`ami_pro_path`) |
| PI.4 The readers | DONE | `9ab8ed7` | `crates/pictures` (9 tests: every bitmap depth, RLE8 and RLE4, `AmiPaint`'s natural sizes for PI1 and a 72 dpi bitmap, the 21 in and 32,767 rules, refusals and every truncation, the 24-bit writer read back; the screen image's three depths and refusals; the metafile window on the unit square, bitmaps, text in CP-1252, `PatBlt`, arcs, pies, ellipses, the placeable size, every truncation); example `picture_census` | New core crate `amipro-pictures`, no dependencies: `dib` (BMP and packed DIBs to `Raster`; `Header::natural_size` as `AMIPAINT.DLL`; `encode_bmp24`), `wmf` (`play` to a `Drawing` of `Op`s in the window's unit square: paths, images, `PatBlt` fills, text, clips), `screen` (Ami Pro's screen image). Census: every stored bitmap and metafile reads (user's corpus 281 + 230, ISO 18 + 13, VM 41 + 7); Ami Pro's screen image for 138 drawings, charts, and OLE objects and 7 linked pictures in the user's corpus, 3 drawings in the ISO; boxes 286 / 2 / 3 (drawings, charts, and OLE objects without one, 8-bit screen images, linked files not found) |
| PI.5 Drawing on the page surface | DONE | `808afcd` | `pictures_are_drawn_in_their_frames_clipped_to_the_picture_area` (the window, by colour: a huge, moved picture kept inside the picture area, its margins showing; a 2 × 1 bitmap fitted at 2 × 1); `metafile_ops_draw_where_the_window_puts_them`; `resolutions_come_from_png_and_jpeg_headers`; `linked_pictures_are_found_by_name_in_the_documents_folder`; screen images' 5-5-5 and 5-6-5 (core); screenshots against Ami Pro's (VM PI1, PI2, PI5, PI7, PI8: place, size, crop, pan; README31's page 12; three corpus pages: a letterhead metafile with text, drawings from Ami Pro's screen images) | `pictures.rs` (the session's `Pictures`: decoded once, keyed by the bytes' allocation or the linked file and its time; `natural_size`, `shown`: picture, screen image, or box), `metafile_render.rs` (the display list with tiny-skia: paths, hatches, patterns, images, `PatBlt`, text through COSMIC Text with the shaper's font substitutes, clips; one-pixel lines one screen pixel), SVG through `resvg`; the page surface draws a picture frame as any frame with the picture between its background and its lines, clipped to the picture area in a layer of its own, the lines in a layer above (iced draws a layer after its parent's primitives). **Found and fixed:** text in a line a Flow with text frame makes taller was drawn centred, not on the baseline (COSMIC Text centres a line's text; PI1–PI8 against Ami Pro); 13 of 191 16-bit screen images are 5-5-5, told by their top bit (the corpora: none of them sets it; 176 of the other 178 hold 5-6-5 white) |
| PI.6 Insert › Image… | DONE | `9495af2` | `insert_image_puts_a_picture_in_a_new_frame_at_the_caret` (messages: a Flow with text frame at the caret whose picture area is the picture's 1,800 × 1,200, Fit in frame, selected, one undo step, undone; Link to file keeps the path; with the picture frame selected it replaces the picture; a file no reader takes refused), `insert_image_puts_a_picture_in_an_empty_cell` (the cell's frame in Ami Pro's cell style, not selected; disabled in a cell holding a picture), `a_new_pictures_frame_is_the_pictures_size_within_the_column`, `insert_image_takes_the_decided_formats` | Insert › Image… (`Ctrl+Shift+I`), the portal's chooser filtered to P1's formats with a Link to file choice (`file_chooser::Choice::boolean`); `picture_insert.rs`: `target` (Replace, an empty frame, an empty cell, a new frame at the caret in the main text; refused elsewhere, with the reason in the status bar), `new_frame_size` (P3: the picture's own size inside Ami Pro's default frame, reduced to the caret's column); `EnableRule::PictureTarget`. Not tested by the widget path: the portal dialog itself (the physical check) |
| PI.7 Frame Properties › Picture | DONE | `9aa5919` | `the_picture_section_sizes_moves_and_resets_the_picture` (messages: Graphics Scaling… opens the rail; a typed width makes it Custom 1,440 × 720 with the 2 × 1 aspect kept; moved 0.05 in down; a 400° rotation refused with the reason; Reset and Fit Frame to Picture one step each, undone; the window lays out); `picture_inspector` tests (PI1's numbers in the fields; Ami Pro's limits; Reset; Fit Frame to Picture); screenshots `window-picture-rail-light`, `-dark`; rotation against Ami Pro's (VM PIRT30, PIRT90) | `picture_inspector.rs` (`view`, `text_command`, `choice_command`), the inspector's `Field::Picture…` and `Choice::Picture…` routed with the picture's natural size (`DocumentSession::picture_natural`); the section first in Frame Properties for a picture frame: the kind and natural size, the link, Sizing, Percentage (when chosen), Width and Height (typing makes it Custom; aspect kept sets the other), Keep aspect ratio, Rotate (not for metafiles), Moved right and down, Reset, Replace Picture… (Insert › Image…'s chooser), Fit Frame to Picture, Save Picture As…, Description; Format › Frame ▸ Graphics Scaling… and the frame's right-click menu (`EnableRule::PictureSelected`). The picture area is the engine's frame interior. **Mapped in the VM:** Rotate turns the picture clockwise as seen and stretches the turned picture's box onto its rectangle (`PICTURE_NOTES.md`, Rotation); drawn so |
| PI.8 Picture mode | DONE | `617d0da` | `picture_mode_moves_the_picture_through_the_widget` (widget: the hand pointer; a drag in the frame sends one `Pan` with the pointer's offset; Right 72 twips, Ctrl+Down 720; Esc leaves; Shift and the bottom-right handle send `Resize` with the picture), `picture_mode_moves_the_picture_in_its_frame` (messages: Enter starts it with the note; a drag's move one step; three arrow presses one more; Esc leaves with the frame selected; Shift and a corner handle double the fitted picture with the frame, Custom 5,184 × 2,592, one step) | `DragKind::Pan` with the picture following the drag (`draw_picture`'s preview offset); `picture_key_action` (`PICTURE_STEP` 72, `PICTURE_LARGE_STEP` 720); grey handles; `FrameAction::Pan`, `LeavePicture`, `Resize { with_picture }`; the session's `picture_mode`, entered by Enter or a double-click on a picture frame (they opened Frame Properties before), left by Esc, Select, or Deselect; `picture_scaled_with_frame` |
| PI.9 Paste and drop | DONE | `e2ff3df` | `pasted_and_dropped_pictures_go_in_as_insert_image_puts_them` (messages: an image alone enables Paste; a pasted PNG goes in at the caret as a PNG picture, one step; a dropped file's URI with a space goes in at the caret the drop moved; a text file refused), `a_drop_moves_the_caret_to_where_it_was_dropped` (widget: the drag as the pointer, the drop event moves the caret there), `drops_and_the_clipboard_give_pictures` | Paste with no text reads the clipboard's image (`ClipboardImage`: PNG, JPEG, GIF, BMP, TIFF, SVG, PNG first) and inserts it as Insert › Image… does (`insert_picture_bytes`, shared with the chooser); the focus probe also looks for an image (`clipboard_has_image`); the page surface sits in libcosmic's `dnd_destination` for `text/uri-list` with the drag forwarded as the pointer, moves the caret to the drop point on the drop, and the file follows (`Message::Dropped`, `dropped_pictures`). Not reachable headless: the compositor's drag and the portal's clipboard (the physical check) |
| PI.10 Save As SAM and twins | DONE | `4729431` | `save_as_sam_converts_pictures_ami_pro_cannot_hold` (a 2,400 × 1,600 JPEG and a Linux link: the loss check names both; the file holds a 24-bit bitmap at most 300 dpi at the frame's size whose resolution keeps Ami Pro's natural size within 2 twips, the link copied in; read back, both bitmaps); `picture_twins` (ignored; `AMIPRO_TWIN_OUT` writes API1, API2, API5, API7, API8, APIRT, APNG, AJPG, APCX, ATIF, ACELL, AFRM through the app's Save As path) | `pictures::for_ami_pro` before `write_sam`: other formats to bitmaps (`sam_bitmap`: decoded, reduced to 300 dpi at the drawn size, the resolution from the natural size), Linux links copied in, Ami Pro's kept records and screen images dropped; `natural_of` reads raster files' headers only (a large photo is not decoded to be inserted). **In the VM:** all twelve open in Ami Pro with their pictures where the app put them (`twin-*` screenshots): fitted, the placeable metafile, Original size, Custom 1.00 × 0.25 in, moved 936 and 144, turned 90°, the converted PNG, JPEG, PCX, and TIFF, TF7's cell, an empty frame; Ami Pro's re-saves of API8 and ACELL (API8S, ACELLS) keep the scaling and pan, filling in the driver's 48 words (and refitting the cell's Fit in frame picture to its row). **Found and fixed:** a new picture frame's picture stored the frame's size, not its picture area (API1: 2,170 × 1,570 for 1,800 × 1,200) |
| PI.11 Both formats and performance | DONE | `132af51` | `pictures_read_back_in_both_formats` (a PNG picture Custom, turned, moved, and described, and a linked bitmap: native exact; Ami Pro's file keeps scaling, rotation, and pan, the PNG a bitmap, the link a copy, each listed, the description listed); `picture_performance_baseline` (ignored, release); the final measures (log) | Timings in the log; typing A/B against `pictures-run-start` in the log |
| PI.12 Exit | DONE | (the handoff commit) | `tools/verify.sh --release --corpus` PASS at `132af51` (core 330 + 4; app 267, 106, 274 with 28, 4, 28 ignored) | §25's implementation notes; `FEATURE_PARITY.md` GR-01; `UI_COMMAND_MATRIX.md` (Image…, Graphics Scaling…, Paste); `UI_ACCEPTANCE.md` "Pictures" and its sign-off row; `FORMAT_COMPATIBILITY.md`; `REFERENCE_FEATURE_MATRIX.md`; `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue and `GRAPHICS.md`, `docs/SESSION_HANDOFF.md` |

## Baseline (at `pictures-run-start`)

Measured 2026-10-18 (`target/pi-baseline/`, outside Git) at
`pictures-run-start`; the VM folder now holds the run's fixtures (PI*,
PIR*), 235 files.

- Round trip (read back equal; Save As lists nothing): user's corpus 417
  of 417, ISO 159 of 160, VM folder 235 of 235; sections equal to the
  original 51,216 of 53,352, 8,205 of 8,443, 24,101 of 24,111.
- Layout comparison: user's corpus page counts 302 of 394, paragraph line
  counts 14,880 of 15,356, page ends 760 of 1,108; ISO 65 of 160, 22,773
  of 24,971, 492 of 1,307; VM folder 234 of 234 page counts, 1,020 of
  1,031 line counts, page ends 89 of 89.
- `[isd]` records over the three folders (1,053): every number written as
  an unsigned word (values 6–9 and the pan; −544 as 64992); rotation 0 in
  all; the unit 1 (1,010) or 3 (12); 31 older records end after the pan,
  without the second state block's count; that block is empty in all but
  one (16 words).

## Log

- 2026-10-18, PI.1: the decompile (`AMIPRO.EXE`, `AMIPROUI.DLL`,
  `AMIPAINT.DLL`, `AMIWMF.DLL`) and the VM (PI1–PI9, PIR1–PIR5, T1); the
  corpora's 973 stored pictures counted by kind; two WMF crates tried on
  the 72 distinct metafiles; the user's answers P1–P4 (P2: copy, with a
  Link option) and P5–P8, all the recommendations otherwise.
- 2026-10-18, PI.2: the dialog's limits from `FRAMESCALEPROC` (Rotate
  0–359; Percentage 1 to what keeps the picture within 31,680; Width and
  Height 1–31,680) and `1168:2dd2`'s arithmetic (each axis truncated; an
  axis below 0 or past 31,680 is 31,680); the `[isd]` survey over the
  three folders (Custom pictures always `0 0 W −H`; the other modes keep
  stale offsets in 133 records, which only the kept record holds).
- 2026-10-18, PI.3 (`target/pi-writer/`, `target/pi-cmp/`): a first
  version listed the two linked TIFFs as conversions (their driver is
  AmiPaint, so Ami Pro reads them) and rebuilt the 16 older-version
  records of three files (`.G0x` metafiles), whose state ends otherwise
  and holds no pan; both fixed, and every written file compared with the
  baseline build's (a scratch worktree at the tag): no file differs in
  any folder.
- 2026-10-18, PI.4: the bitmap driver's size rule read in
  `AMIPAINT.DLL` (pixels a metre, or 96 dpi lowered to keep each side at
  most 21 in; sides halved past 32,767); the metafile driver's state words
  2 and 3 its extent (PI2: 2,880 × 1,440; all 269 corpus metafile records
  carry one); the screen image's header confirmed in all 499 corpus
  images (four planes 202, 16 bits 196, 8 bits 66, 32 bits 35); the
  metafile records and their values counted over the 73 distinct corpus
  metafiles (every raster operation a copy; bitmaps uncompressed 4- and
  8-bit RGB; one file of 5,787 `PatBlt` fills; three windows y-up).
- 2026-10-18, PI.5: app dependencies `image` 0.25 (bmp, gif, jpeg,
  png, tiff), `image-extras` 0.1.1 (pcx), `resvg` 0.45 (text, system
  fonts), `tiny-skia` 0.11, fetched once (`tiff`, `fax`, `pcx`,
  `image-extras`, `gif` 0.14 new in the lock; all MIT, Apache-2.0, Zlib,
  or WTFPL). Vector pictures are drawn at twice the logical size; rotation
  is drawn about the picture's centre until PI.7 maps Ami Pro's.
- 2026-10-18, PI.6: the disk filled during the build (222 MB free); the
  app's incremental cache, the old A/B and comparison builds under
  `target/` (24 GB, all regenerable) were removed. Insert › Image… with
  the caret in a header, footer, or frame's text is refused with a
  message (a Flow with text frame anchors in the main text only); what
  Ami Pro does there is not mapped (handoff).
- 2026-10-18, PI.11 (`target/pi-final/`): the round trip and the layout
  comparison equal the baseline row for row on both corpora (417/417,
  159/160; sections 51,216 and 8,205; page counts 302 of 394 and 65 of 160,
  line counts 14,880 and 22,773, page ends 760 and 492); the VM folder
  holds four more files Ami Pro wrote this run (PIRT30, PIRT90, API8S,
  ACELLS): 239/239 read back equal, each one page as in Ami Pro, line
  counts 1,026 of 1,037. The pictures run moves no text. Release timings
  (`picture_performance_baseline`): README31's page 12 drawn first 32 ms
  and again 0.57 ms, its three pictures decoded anew on the warm page 0.49
  ms; the user file with the most pictures, page of five, 76 ms first,
  0.62 ms again, 0.66 ms decoded anew; the corpus's heaviest metafile
  (5,787 fills) 7.2 ms decoded anew, 0.55 ms again; typing beside a
  picture 0.87 ms; a picture mode step 0.91 ms (the first draws are the
  pages' text, shaped once).
- 2026-10-18, PI.11, typing A/B against `pictures-run-start`
  (`tools/bench-ab.sh`, five interleaved rounds, README31,
  `target/bench-ab/20261018-181308`): the default editor is unchanged
  (typing 0.64–0.92 ms a keystroke on both sides, the medians crossing
  from round to round; each p95 flag is one round). The rich editor
  prototype (feature-gated, not the normal build) is 0.05–0.07 ms slower
  in every edit (insert 0.32–0.37 ms to 0.39–0.44 ms, redo 0.20–0.24 ms
  to 0.25–0.27 ms). Bisected to PI.2 (`b7f0ca9`); the later slices add
  nothing. At PI.2 and the commit before it, each step of the insert
  (the command, the cursor, the document's clone, history, the hidden
  text, the pages, the caret) times alike, and the real call takes 0.32
  ms in both once an unrelated test is appended to `main.rs`, cold or
  after the benchmark's warm-up. PI.2 adds no work to an edit; the
  difference is code placement in that build, as in the outline levels
  run.
- 2026-10-18, after the run, the user's physical check: Insert › Image…
  and picture mode pass; paste and drop did nothing; Percentage was not
  understood. **Paste:** Chromium's Copy Image puts the image's address
  on the clipboard as text beside the image (its own comment: text is
  left out only on the Mac), and Paste took the text first. Paste whose
  text is only a web address or a file path now looks for an image, then
  for copied picture files (`text/uri-list`, which COSMIC Files copies
  with the paths as text), and pastes the text only when there is
  neither. **Drop:** read through libcosmic, smithay-clipboard, and COSMIC
  Files' source: Files offers `text/uri-list` with Copy or Move, which
  the page's target took; the whole drop now runs through the window in a
  test (`a_file_dropped_on_the_page_goes_in_through_the_window`: the id
  the runtime registers, a motion, the drop, the caret's move rebuilding
  the view, then the data under the drop's id), and it passes, so the
  failure on the desktop is not found by reading. A first guess (a new
  drop-target id at every view) was wrong: the tree diff keeps the id.
  Changed anyway: the drop asks for a copy (libcosmic prefers a move), an
  image a browser drags is taken before files, and
  `AMIPRO_TRACE_DROP=FILE` writes what paste and drop receive with
  libcosmic's drop and clipboard traces, for the recheck. **Percentage:**
  a line under Sizing says what each sizing does, and the percentage is
  always shown (in the other sizings, how large the picture is now).
- 2026-10-18, the user's recheck with `AMIPRO_TRACE_DROP`: Copy Image
  pasted and a file dragged from Files went in (the trace: Enter, Copy
  chosen, motions, Drop, the `text/uri-list` data, the picture); a file
  copied in Files pasted through Edit › Paste (Files offers it as
  `image/png`), but the right-click menu's Paste was greyed out. The
  trace also showed the browser's Copy Image put **no** text on the
  clipboard, so the address beside the image (the first explanation
  above) was not what failed in the first check. **Cause:** Paste's state
  comes from a look at the clipboard when the window gains focus, and
  smithay-clipboard's worker learns of the focus from its own copy of the
  keyboard's enter, which it can handle after the read (its request
  channel is dispatched first), so the read finds nothing and Paste is
  disabled, and a disabled Paste also dropped Ctrl+V without a word.
  **Fixed:** Paste always reads the clipboard (the status bar says when
  there is nothing to paste); the right button looks at the clipboard
  before its menu opens; a first look that finds nothing is tried once
  more 150 ms later (a sleeping thread, not the executor); the trace
  records each look. Tests `paste_looks_at_the_clipboard_again`,
  `the_text_editor_looks_at_the_clipboard_again`.
- 2026-10-18, the user's second recheck: right-click Paste after a copy
  in Files, Percentage 50, README31 page 12, a logo at 200 %, and Save As
  Ami Pro listing the converted picture all pass: **the physical check
  passes**. The list said "1 picture … written as Windows bitmaps"; one
  picture now reads "a Windows bitmap" (and one link "a copy of the
  file", several "copies of the files").
