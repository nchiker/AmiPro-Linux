# Test commands

The specialised test, benchmark, census, and comparison commands, moved verbatim from CLAUDE.md's "Build and test" section on 2026-11-04 (SU1).

## Build and test

```sh
cargo test --workspace --offline                                            # core: 601 pass, 14 ignored
cargo clippy --workspace --all-targets -- -D warnings
cargo test --offline --manifest-path app/cosmic/Cargo.toml                  # normal build (page surface): 423 pass, 71 ignored
cargo test --offline --no-default-features --manifest-path app/cosmic/Cargo.toml   # text editor: 117 pass, 6 ignored
cargo test --offline --all-features --manifest-path app/cosmic/Cargo.toml   # 430 pass, 71 ignored
AMIPRO_STYLES=$PWD/../AmiPro-Reference/Charlie-Sample/STYLES cargo test --offline --release -p amipro-formats --test formula_corpus -- --ignored --nocapture   # the formula audit (TD.4): every calculated cell of the user's, ISO, and VM corpora recomputed against Ami Pro's stored result; aggregate counts; AMIPRO_FORMULA_DETAIL=1 lists the VM folder's differing cells
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test -p amipro-editing --test layout_twins -- --ignored   # the page layout twins
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test --offline --manifest-path app/cosmic/Cargo.toml ruler_twins -- --ignored   # the ruler twins (RU1–RU12)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test --offline --manifest-path app/cosmic/Cargo.toml frame_twins -- --ignored   # the frame creation twins (FC1–FC9; AMIPRO_TWIN_OUT writes AFC*.SAM)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test --offline --manifest-path app/cosmic/Cargo.toml bullet_twins -- --ignored   # the bullets twins (BL1, BL2, BL4; AMIPRO_TWIN_OUT writes ABL*.SAM)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test --offline --manifest-path app/cosmic/Cargo.toml outline_twins -- --ignored   # the outline twins (OL1, OL2, a nested list, nested bullets; AMIPRO_TWIN_OUT writes AOL*.SAM)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test --offline --manifest-path app/cosmic/Cargo.toml colour_twins -- --ignored   # the colour twin (ACS1: mixed colours everywhere; AMIPRO_TWIN_OUT writes ACS1.SAM)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test --offline --manifest-path app/cosmic/Cargo.toml cell_frame_twins -- --ignored   # the frames in cells twins (ATF2, ATF3, ATF7, ATF10, README31 with a page break; AMIPRO_TWIN_OUT writes ATF*.SAM and ARM31.SAM)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test --offline --manifest-path app/cosmic/Cargo.toml table_page_twins -- --ignored   # the tables across pages twins (ATP2, ATP5, ATP6 from TP1S and TP3S; AMIPRO_TWIN_OUT writes them)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT cargo test --offline --manifest-path app/cosmic/Cargo.toml picture_twins -- --ignored   # the pictures twins (AMIPRO_TWIN_OUT writes API1, API2, API5, API7, API8, APIRT, APNG, AJPG, APCX, ATIF, ACELL, AFRM through Save As SAM)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab cargo test --offline -p amipro-editing --test find_twins -- --ignored   # Find & Replace: Ami Pro's input files give Ami Pro's results (FR1A–FR6); with AMIPRO_CORPUS (and AMIPRO_STYLES) every paragraph found in its stream
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab cargo test --offline -p amipro-editing --test field_twins -- --ignored   # fields: 45 of Ami Pro's results reproduced (FD3, FD6, FD7); with AMIPRO_TWIN_OUT also writes the twin AFD1.SAM; with AMIPRO_CORPUS (and AMIPRO_STYLES) every document shows its codes and hides them back to itself
AMIPRO_PERF_SAM=FILE cargo test --offline --release --manifest-path app/cosmic/Cargo.toml field_performance_baseline -- --ignored --nocapture   # fields: a keystroke, Update All, its undo, Show Power Fields with 100 fields
AMIPRO_PDF_SAM=FILE AMIPRO_PDF_OUT=FILE.pdf [AMIPRO_PDF_OPTIONS=crop,cover,form,nopictures,pdfa] cargo test --offline --manifest-path app/cosmic/Cargo.toml print::tests::pdf_export -- --ignored --nocapture   # any SAM as the app exports it
AMIPRO_COMPARE_PDF=DIR [AMIPRO_STYLES=…] [AMIPRO_COMPARE_LIMIT=N] [AMIPRO_COMPARE_OUT=DIR] cargo test --offline --release --manifest-path app/cosmic/Cargo.toml pdf_against_screen -- --ignored --nocapture   # the PDF (hayro) against the page surface (tiny-skia), page by page
AMIPRO_CENSUS_DIR=DIR cargo test --offline --release --manifest-path app/cosmic/Cargo.toml vector_picture_census -- --ignored --nocapture   # metafiles and SVG the PDF draws as vectors (aggregate)
AMIPRO_PERF_SAM=FILE cargo test --offline --release --manifest-path app/cosmic/Cargo.toml pdf_performance_baseline -- --ignored --nocapture   # the PDF's timing
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT AMIPRO_TWIN_OUT=DIR cargo test --offline -p amipro-editing --test tables table_twins -- --ignored   # table editing: TE1 changed by each command family, written as ATE*.SAM (cells and headers against TE2–TE11)
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT AMIPRO_TWIN_OUT=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml drawing_twins -- --ignored   # drawings: ADR1 (every tool), ADR2 (DR2B edited), ADR3 (SVG and a picture imported) by Save As SAM
cargo run --release -p amipro-pictures --example drawing_census -- DIR...   # every drawing in SAM files and .sdw files read and written back (aggregate)
AMIPRO_CORPUS=DIR AMIPRO_STYLES=… [AMIPRO_COMPARE_OUT=DIR] cargo test --offline --release --manifest-path app/cosmic/Cargo.toml drawings_against_ami_pro -- --ignored --nocapture   # drawings painted against Ami Pro's own screen images
AMIPRO_CORPUS=DIR AMIPRO_STYLES=… [AMIPRO_COMPARE_OUT=DIR] cargo test --offline --release --manifest-path app/cosmic/Cargo.toml ole_objects_against_ami_pro -- --ignored --nocapture   # OLE objects drawn from their presentation metafile (every one must draw; the user's corpus: 34 of 34), those with Ami Pro's screen image compared as the drawings are (DO.3)
AMIPRO_SCREENSHOT_DIR=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml object_screenshots -- --ignored   # the Object section and the Links panel, light and dark (synthetic objects)
[AMIPRO_PERF_SAM=FILE] cargo test --offline --release --manifest-path app/cosmic/Cargo.toml drawing_performance_baseline -- --ignored --nocapture   # drawings painted; a move, a style, Undo in Draw mode
cargo test --offline --release --manifest-path app/cosmic/Cargo.toml table_command_performance_baseline -- --ignored --nocapture   # the table commands on a 100 × 5 table
AMIPRO_SCREENSHOT_DIR=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml table_screenshots -- --ignored   # the table interface, light and dark
AMIPRO_PERF_SAM=FILE [AMIPRO_CORPUS=DIR AMIPRO_STYLES=…] cargo test --offline --release --manifest-path app/cosmic/Cargo.toml proofing_performance_baseline -- --ignored --nocapture   # proofing: dictionary, a whole Spell Check, as you type, Harper, the thesaurus; with AMIPRO_CORPUS every SAM checked without a panic
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT AMIPRO_TWIN_OUT=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml proofing_twins -- --ignored   # the proofing twin APF1 (PSP4B edited, British, All Rules) by Save As SAM
AMIPRO_STYLES=$PWD/../AmiPro-Reference/vm-lab/STY AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab AMIPRO_TWIN_OUT=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml contents_twins -- --ignored   # TOC, index, glossary twins ATI1, ATI1X, ATI3, AGL1
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab cargo test --offline -p amipro-formats --test glossary_twin -- --ignored   # GLO0's Mark written byte for byte as Ami Pro wrote it
AMIPRO_TWIN_OUT=$PWD/../AmiPro-Reference/vm-lab/IN [AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT] cargo test --offline --manifest-path app/cosmic/Cargo.toml merge_twins -- --ignored --nocapture   # merge: AMD1, AML1, AMLB by Save As SAM; with AMIPRO_VM_LAB, Ami Pro's AMT1 against the app's merge
cargo test --offline --release --manifest-path app/cosmic/Cargo.toml merge_performance_baseline -- --ignored --nocapture   # a 1,000-record merge, the preview, the record editor
AMIPRO_VM_LAB=$PWD/../AmiPro-Reference/vm-lab/OUT AMIPRO_TWIN_OUT=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml master_twins -- --ignored   # master documents: the app's master over MD3's members and their [sequence] (AMS, AMA–AMC)
AMIPRO_PERF_MASTER=DIR cargo test --offline --release --manifest-path app/cosmic/Cargo.toml master_performance_baseline -- --ignored --nocapture   # a folder's SAM files as a set: reading, a keystroke, Find, the set's DOCX
AMIPRO_PERF_SAM=FILE cargo test --offline --release --manifest-path app/cosmic/Cargo.toml contents_performance_baseline -- --ignored --nocapture   # TOC, Update, Mark All, index
AMIPRO_PERF_SAM=FILE cargo test --offline --release --manifest-path app/cosmic/Cargo.toml find_performance_baseline -- --ignored --nocapture   # Find, a keystroke with the bar open, Replace All, its undo (AMIPRO_FIND, default "the")
cargo run --release -p amipro-pictures --example picture_census -- DIR   # what each kind of picture in a folder is drawn from (aggregate)
[AMIPRO_STYLES=…] cargo run --release -p amipro-exchange --example export -- OUT_DIR SAM_OR_DIR   # every SAM through the four filters with the test shaper; failures and lossless counts (aggregate)
AMIPRO_EXPORT_SAM=SAM_OR_DIR AMIPRO_EXPORT_OUT=DIR [AMIPRO_EXPORT_PDF=1] cargo test --offline --release --manifest-path app/cosmic/Cargo.toml export::tests::export_files -- --ignored --nocapture   # as the app exports (its fonts, pictures), with its PDF for comparing
soffice -env:UserInstallation=file://$PWD/target/lo-profile --headless --convert-to pdf --outdir DIR FILES   # LibreOffice's rendering of exports (a private profile)
AMIPRO_PERF_SAM=FILE cargo test --offline --release --manifest-path app/cosmic/Cargo.toml export_performance_baseline -- --ignored --nocapture   # each format's export time
cargo run --release -p amipro-exchange --example import -- DIR [--parts]   # every DOCX, ODT, FODT, RTF, and text file under DIR read and realised (aggregate counts; never a panic)
AMIPRO_STYLES=… AMIPRO_REALISE_DIR=DIR [AMIPRO_REALISE_FORMAT=docx|odt|rtf|text] cargo test --offline --release -p amipro-exchange --test realise realise_round_trip_report -- --ignored --nocapture   # each SAM built, written, read, realised, and built again (equal in what the format holds)
AMIPRO_IMPORT_IN=FILE_OR_DIR AMIPRO_IMPORT_OUT=DIR cargo test --offline --release --manifest-path app/cosmic/Cargo.toml import::tests::import_files -- --ignored --nocapture   # other programs' documents imported as File › Open does, with the app's PDF
AMIPRO_PERF_IMPORT=FILE cargo test --offline --release --manifest-path app/cosmic/Cargo.toml import_performance_baseline -- --ignored --nocapture   # reading, realising, and laying out a document
AMIPRO_IMPORT_TWINS='NAME=FILE;…' AMIPRO_TWIN_OUT=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml import_twins -- --ignored --nocapture   # imported documents written by Save As SAM for Ami Pro in the VM
AMIPRO_PERF_SAM=README31.SAM AMIPRO_PICTURE_SAM=FILE cargo test --offline --release --manifest-path app/cosmic/Cargo.toml picture_performance_baseline -- --ignored --nocapture   # picture timings
AMIPRO_NEW_TWINS='SHEET.STY=SAVED.SAM;…' cargo test --offline --manifest-path app/cosmic/Cargo.toml new_documents_match_ami_pros -- --ignored --nocapture   # File › New… with contents against Ami Pro's New (VM lab: STY/ and OUT/RPT2A, NRPT4, …; SAM_FORMAT_NOTES.md)
AMIPRO_SCREENSHOT_SAM=FILE AMIPRO_SCREENSHOT_DIR=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml sam_screenshot -- --ignored   # any SAM's first page as the app draws it (AMIPRO_SCREENSHOT_PAGE=N for another page, AMIPRO_SCREENSHOT_ZOOM)
cargo test --offline -p amipro-formats --test sam_image_adjust -- --nocapture   # Image Processing's `[isd]` state words against the VM's IMV1A..L files in ../AmiPro-Reference/vm-lab/OUT/objects (12 of 12 compared when present)
AMIPRO_SCREENSHOT_DIR=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml image_and_paste_special_screenshots -- --ignored   # the Image section (also in a tall window) and the Paste Special panel, light and dark, as PNG files
cargo run --offline --manifest-path app/cosmic/Cargo.toml                   # the normal build: the page editing surface
cargo clippy --offline --manifest-path app/cosmic/Cargo.toml --all-features --all-targets -- -D warnings
cargo run --offline --manifest-path app/cosmic/Cargo.toml --no-default-features   # the earlier text editor
cargo run --offline --manifest-path app/cosmic/Cargo.toml --no-default-features --features rich-editor-prototype
cargo run -p amipro-formats --example corpus_audit -- /tmp/amipro-phase1-corpus   # private extraction, outside Git
AMIPRO_STYLES=…/Charlie-Sample/STYLES cargo run --release -p amipro-formats --example sam_round_trip -- DIR [--detail FILE]   # SAM round-trip audit
# Release benchmarks (private SAM path via env var):
AMIPRO_PERF_SAM=$PWD/Reference/SAM/README31.SAM cargo test --offline --release --all-features \
  --manifest-path app/cosmic/Cargo.toml rich_editor_typing_performance_baseline -- --ignored --nocapture
AMIPRO_PERF_SAM=$PWD/Reference/SAM/README31.SAM cargo test --offline --release \
  --manifest-path app/cosmic/Cargo.toml default_editor_typing_performance_baseline -- --ignored --nocapture
```

Gate scripts (2026-10-11):

- `tools/verify.sh [--quick] [--release] [--corpus]` runs fmt, strict Clippy, and tests for the core and the app (default and all features). Logs go to `target/verify/`.
- `tools/corpus.sh` runs the corpus audit, re-extracting the corpus from the ISO if `/tmp` was cleared.
- `tools/bench-ab.sh BASE_REV [ROUNDS]` is the interleaved release A/B of both editor baselines against the working tree.

`tools/verify.sh --release --corpus` last passed on 2026-11-03 (the end of the merge run): core 599 + 14 ignored, COSMIC 422 (default), 116 (text editor), and 429 (all features) with 71, 6, and 71 ignored, release builds, ISO corpus audit gate PASS; the round trip and the layout comparison equal to the run's first session but for Ami Pro's merge of the twin in the VM folder (`target/merge-baseline/exit/`).

Benchmark caution: this host often runs unrelated emulator workloads, and the CPU governor is `powersave`. Compare builds with interleaved A/B runs, not against earlier numbers.

## Chart oracle (run D, DO.6–DO.7)

The chart data records of the corpus and the VM's CHV2 charts read, write
back equal, and regenerate (object counts by kind; coordinates within the
tolerances in `docs/research/OBJECTS_NOTES.md` §13). Ignored; skips when the
corpus is absent; `CHART_DIFF=n` lists chart n's objects:

```sh
AMIPRO_STYLES=../AmiPro-Reference/Charlie-Sample/STYLES cargo test --offline -p amipro-pictures --test chart_oracle -- --ignored --nocapture
```

The twin for the VM (CHV1: the app's own 3D clustered column, data 12, 23, 5, 14):

```sh
AMIPRO_TWIN_OUT=DIR cargo test --offline -p amipro-editing --test charts
```

## Equation oracle (run D, DO.9)

The 27 equations of the VM's EQV1 fixtures (`../AmiPro-Reference/vm-lab/OUT/objects/EQV1A.SAM`, `EQV1B.SAM`): the stored form reads and writes back byte equal, the text is Ami Pro's normal form, and the natural sizes are compared with the layout's (the table, with the mean and worst error in percent, is printed). Ignored; skips when the folder is absent (`AMIPRO_REFERENCE_DIR` overrides `../AmiPro-Reference`):

```sh
cargo test --offline -p amipro-pictures --test equation_oracle -- --ignored --nocapture --test-threads=1
```

The twin for the VM (EQV1: does Ami Pro open and repaint what the app wrote; eight equations in `EQT2.SAM`):

```sh
AMIPRO_TWIN_OUT=DIR cargo test --offline -p amipro-editing --test equations
```

The Equation editor's screenshots (light and dark, the card with a sample and with an error; not committed):

```sh
AMIPRO_SCREENSHOT_DIR=DIR cargo test --offline --manifest-path app/cosmic/Cargo.toml equation_screenshots -- --ignored
```


## Debian package (P6.1)

```sh
cd app/cosmic && cargo build --release -j1 && cd ../..   # one cargo command at a time on this host
tools/package-deb.sh                                      # dist/amipro-linux_<version>_amd64.deb
dpkg-deb --info dist/*.deb; dpkg-deb --contents dist/*.deb
lintian dist/*.deb                                        # if installed
dpkg-deb -x dist/*.deb "$SCRATCH/x" && ldd "$SCRATCH/x/usr/bin/amipro-linux" | grep -c 'not found'   # 0
```

The binary has no `--help`/`--version` (it opens a window), so the headless check is `ldd`.
