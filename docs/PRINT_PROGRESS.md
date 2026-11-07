# Print and PDF Progress

Live state of `docs/PRINT_RUNBOOK.md`. Resume at the first slice that is
not `DONE`.

| Slice | State | Commits | Evidence | Notes |
|---|---|---|---|---|
| PP.1 Setup | DONE | (setup commit) | `PRINT_NOTES.md`; `UI_SPEC.md` §29; PO1–PO6 in `DECISIONS.md`; the baseline below | Tag `print-pdf-run-start` |
| PP.2 The PDF renderer | DONE | `ce61d8b` | `pdf::tests`: layers in iced's order (a child layer after its parent's later primitives), a page with text and quads is a PDF with an embedded font and ToUnicode | `pdf.rs`: `PdfRenderer` (iced's `Renderer`, `text::Renderer`, `image::Renderer`), `PdfResources` (each COSMIC Text face and raster once), `PdfWriter`; quads as rounded paths with their borders as rings, layers as clips, paragraphs as glyph runs placed where COSMIC Text put each glyph, with their text (FAKE_ITALIC as a slant), images at their own pixels |
| PP.3 Print mode | DONE | `ce61d8b`, `e376738` | `print::tests`: every page at its layout's size (`hayro` reads the PDF); paper shows no screen marks, the same text, and on a preprinted form the fixture frame's lines go and its text stays; README31 exported: 14 pages, Liberation faces embedded, subset, Unicode-mapped, the text extracted in order | `PageSurface::printing(PrintMode)`; `print::document_pdf`, `draw_pages` (one page at a time at a pixel a twip); `pdf_pictures.rs`: metafiles as paths, images, text, clips, hatch and pattern fills as tiling patterns; SVG as paths; `Pictures::printed` registers stand-ins. Census (`vector_picture_census`, aggregate): 230 of 250 metafiles in the user's corpus, 13 of 13 ISO, 7 of 7 VM as vectors (the rest turned or unreadable, at 300 dpi). The page-by-page comparison with the screen is PP.9's |
| PP.4 Export PDF… | DONE | `65d17dd` | `export_pdf_reports_what_it_wrote` (message path); `an_export_writes_the_pdf_over_a_file_there`; `the_pdf_is_named_after_the_documents_file` | `ExportJob` on a thread of its own (`run_in_background`), `file_workflow::write_file_replacing` |
| PP.5 Ami Pro's options | DONE | `8d77b06` | `crop_marks_are_ami_pros_seven_sixteenths_ending_a_sixteenth_short`, `a_pdf_with_crop_marks_has_the_page_in_a_half_inch_border_and_a_trim_box`, `the_cover_sheet_lists_doc_info_as_ami_pro_does`, `printing_with_update_fields_updates_every_field_first` (message path); FC1 exported with each option beside Ami Pro's PFC1C, PFC1D: the same marks and cover lines | `PrintOptions` on the session; `CropMarks`, `crop_marks`, `CoverSheet`, `draw_cover`; protected text on a preprinted form is not in the PDF at all |
| PP.6 PDF additions | DONE | `1648228` | `bookmarks_nest_as_the_outline_levels_do`, `page_labels_are_ami_pros_page_numbers`, `an_archival_pdf_declares_pdf_a_2b`; README31, AP31NAV, MERCURY exported as PDF/A-2b | `PdfAdditions`; `bookmarks`, `nested`, `page_labels`, `metadata`, `serialize_settings` |
| PP.7 Print… | DONE | `dd1dc5f` | `the_dialogs_pages_are_read_from_its_ranges`, `the_paper_is_named_where_it_is_standard_and_turned_for_landscape`, `print_sends_the_pages_the_desktops_dialog_chose` (message path); the portal in the physical check | `print_portal.rs`: `prepare`, `send`, `chosen_pages`, `page_setup`; `PdfJob`, `in_background`; File › Print Current Page… |
| PP.8 The Print panel and preview | DONE | `369433d` | `the_print_panel_keeps_ami_pros_options_and_shows_the_pages_as_they_print` (message path), `options_and_additions_read_and_set_by_their_switches`; the preview's drawing in `paper_shows_no_screen_marks…`; `print_panel_screenshots` (ignored): `window-print-light`, `window-print-dark` | `print_panel.rs`; File › Print Options…; `PrintMode::preview` (white paper with its shadow and edge, the caret, selection, and handles kept); `Preferences::pdf_additions` (`pdf_bookmarks`, `pdf_properties`, `pdf_archival`); the buttons pinned below the options |
| PP.9 Comparisons | DONE | `5b130ea` | Ami Pro's printed fixtures (PFC1C, PFC1D, PFC1F, PPT1F, PACLF, PACLP, PPI4F) against the app's PDF of FC1, PT1OUT, ACELLS, PI4: the same crop marks, cover lines, and what a preprinted form and Without pictures leave out; `pdf_against_screen` (ignored): user corpus sample 126 of 126 pages within 5% non-coinciding ink (76 within 1%), ISO sample 175 of 175 (111), VM folder 318 of 348 (194), the VM's rest hairlines the screen draws a whole pixel wide; README31, AP31NAV, MERCURY exported and looked at page by page | Found and fixed: frame lines drawn one or two screen pixels whatever the scale (1/20 pt in the PDF), PX9; the preview drawing the PDF's vector stand-ins (blank on screen) |
| PP.10 Timings | DONE | `e10bdbc` | `pdf_performance_baseline` (release): README31's 14 pages 61.8 ms median (88.0 worst), AP31NAV's 40 pages 257 ms (287), one page 26–44 ms (the whole document is laid out for the surface first). Typing A/B against `print-pdf-run-start` (`tools/bench-ab.sh`, two rounds, `target/bench-ab/20261020-174217/`): the default build typing at the fixture's start 0.951 and 0.955 ms against 0.970–1.025, at its end 0.960 and 0.873 against 0.988–1.032, at its middle 0.990 against 1.026–1.055 in the second round (the first, 1.418, a noisy round on this host); the rich editor's sustained typing 0.631 and 0.479 against 0.607 and 0.733: no regression | Export and printing draw on a thread of their own |
| PP.11 Exit | DONE | (the exit commits) | `tools/verify.sh --release --corpus` PASS at `d8b9e44` (core 410 + 9; app 310, 111, 317 with 41, 4, 41 ignored; ISO corpus gate PASS), after three tests that assumed Print unimplemented or hatches raster were brought up to date; the round trip and layout comparison equal to the baseline in every summary (`target/print-exit/`) | `UI_SPEC.md` §29 notes; `FEATURE_PARITY.md` IO-03; `UI_ACCEPTANCE.md` "Print and PDF" and its sign-off row; `UI_COMMAND_MATRIX.md`; `FORMAT_COMPATIBILITY.md`; `REFERENCE_FEATURE_MATRIX.md`; `VM_LAB.md`; `DECISIONS.md` PX1–PX9; `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`; the playbook's queue (export NEXT); `docs/SESSION_HANDOFF.md` |

## Baseline (at `print-pdf-run-start`, `target/print-baseline/`)

Measured again at the start (equal to the fields exit's in every number):

- Round trip: user's corpus 417 of 417 read back equal (Save As would list
  nothing 417 of 417), ISO 159 of 160, VM folder 280 of 280.
- Layout comparison: user's corpus page counts 315 of 394, page ends 762 of
  1,108, page ends in tables 69 of 106, Automatic rows at Ami Pro's height
  2,639 of 2,777; ISO 69 of 160, 478 of 1,307; VM folder 279 of 279, 111 of
  121, 48 of 49, 665 of 666.
- Tests (the fields' codes change, 2026-10-20): core 410 + 9 ignored; app
  289, 111, 296 (36, 4, 36 ignored).

## VM set-up

A PostScript printer "PS File" (Apple LaserWriter 12/640 PS, 600 dpi, port
`FILE:`) was added to the XP guest for this run, after a live snapshot
"Before print-pdf run (2026-10-20)"; User Setup › Options › Print in
background is off. Ami Pro prints a fixture with File › Printer Setup (PS
File), File › Print, and a file name in `L:\OUT`; the host converts it with
`ps2pdf` and `pdftoppm` (`PRINT_NOTES.md`).

## Log
- 2026-10-20, PP.2–PP.4: the page surface draws a PDF through a renderer
  of the app's own, so nothing is drawn twice. PX1: a picture the app
  cannot draw (a labelled box on screen) prints nothing, as an empty
  frame. PX2: the grey cell grid is the screen's guide, as Ami Pro's dotted
  one, and does not print; table lines wait for Lines & Color (the
  table-editing run), so tables print without lines until then. PX3: the
  PDF is drawn at a pixel a twip (rules and text as fine as the layout's
  unit) and scaled to points. README31's export took 0.84 s in a debug
  build.
- 2026-10-20, PP.5: PX4: the cover sheet is placed 1 in from the paper's
  corner; Ami Pro's sits 1 in from the printer's printable corner (a few
  hundredths of an inch further in on the VM's printer). PX5: Update fields
  first stops the print when a field asks a question (Query$, Decide); the
  status bar says to answer it and print again. PX6: crop marks are
  quarter-point hairlines (the decompile does not show Ami Pro's pen).
- 2026-10-20, PP.6: krilla records one date, so the PDF is dated by the
  last save (or creation, or now); PDF/A requires one.
- 2026-10-20, PP.7: PX7: Print Current Page… is a command (and a Print
  panel button): the desktop's dialog cannot offer Current page through the
  portal, so the app opens it with the caret's page as its range. PX8: the
  job is named after the document's file (Ami Pro: "Ami Pro - FILE.SAM").
- 2026-10-20, PP.9: AP31NAV's "Card 1/40" box is hidden by the clip art's
  frame on screen and in the PDF alike (page frames paint above layout
  frames); whether Ami Pro shows it is a layout question for later, not a
  printing one.
- 2026-10-20, after the exit: the user's physical check: "printing works
  correctly" (PASS for printing through the desktop's dialog). A PDF opened
  in the user's viewer, Print Current Page…, and the Print panel were not
  reported separately.
- 2026-10-20: "pdf works correctly" (PASS for Export PDF). Print Current
  Page… and the Print panel's preview remain unreported.
- 2026-10-20: the Print panel "looks good, except for the arrangement of
  the buttons at the bottom of the screen"; "I didn't have any issues with
  print current page" (PASS for both). The headless render showed the
  three buttons as left-aligned pills of three widths: libcosmic's text
  buttons give `width(Fill)` to their label's row, not the button. Asked
  with three alternatives, the user chose equal widths, stacked, pinned
  under a divider; done with custom buttons (`print_panel::wide_button`),
  tested by clicks near each button's right edge
  (`the_print_panels_buttons_take_the_rails_width`, which fails without
  the change). Its first version clicked through the test runtime, which
  runs each command's task to completion, so the desktop's real print and
  file dialogs opened on the user's screen; the clicks are now delivered
  without running their commands (`Runtime::click_unrun`).
