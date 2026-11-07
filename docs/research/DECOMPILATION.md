# Decompiling Ami Pro 3.1 (AMIPRO.EXE)

AGENTS.md authorizes decompiling the reference executables. Decompiler
output stays outside Git (`../AmiPro-Reference/ghidra-work/out/`), and the
implementation is written independently from findings documented here and
in `SAM_FORMAT_NOTES.md`. Use the decompile first for algorithms (line
heights, line breaking, justification, break and keep rules) and the VM for
behaviour that is quicker to observe.

## Why stock Ghidra saw 15 functions

`AMIPRO.EXE` is a self-loading NE file (flags `0x0B0A`, 209 segments) linked
with SLR Systems' OPTLINK and its OPTLOADER ("OPTLOADER - Copyright (C)
1993 SLR Systems", segment 1 offset 0x17f). Every segment but the first is
compressed, and the loader in segment 1 unpacks and relocates each segment
when Windows asks for it. Ghidra therefore disassembled compressed bytes.
Only segment 1 carries standard NE relocations.

## The format (2026-10-12, from the loader's disassembly)

- **Segment 1** starts with the self-loading table (`A0`). Its first 0x1e7
  bytes are plain: the table, the decompressor at 0x31, and the boot code.
  The rest (length at offset 0x2e, 1,677 bytes) is compressed and unpacks
  in place at 0x1e7; back-references may reach into the plain part.
- **Other segments**: a word with the number of relocations, a compressed
  stream, then the relocations. The unpacked size is the segment's minimum
  allocation or one byte less.
- **Compression**: LZ with 16-bit control words read most significant bit
  first. The next word is fetched as soon as the 16th bit is used, before
  the token it announces. A 1 bit is a literal byte. A 0 bit starts a match:
  a prefix code gives the length (2 to 19, or a length byte; 0x81 is a
  no-op and anything above ends the stream), a second prefix code gives
  the high byte of the distance (0, 1, 2–3, 4–7, 8–15, 16–31, 32–47,
  48–63, 64–127), and a byte gives the low byte. Distance is that value
  plus 1. A two-byte match uses a one-byte distance.
- **Relocations**: groups of `TYPE COUNT`. Type 0xF0 is a list of
  `SEGMENT HEAD` (three bytes): selector fix-up chains. Otherwise bits 0–1
  give the width (low byte, selector, far pointer, offset), bit 2 makes it
  additive, and bits 3–4 the kind: internal (a segment byte, 0xFF for entry
  ordinals, then `OFFSET TARGET` pairs), import by ordinal or by name (a
  module word, then `OFFSET TARGET` pairs), or OS fix-up (a type word, then
  offsets). Non-additive fix-ups chain through their targets as in standard
  NE.

## Procedure

```sh
tools/ghidra/optloader-unpack.py ../AmiPro-Reference/Binaries/amipro.exe \
  ../AmiPro-Reference/ghidra-work/amipro-unpacked.exe
tools/ghidra/build-ne-unsigned-fix.sh ../AmiPro-Reference/ghidra-work/settings
W=$PWD/../AmiPro-Reference/ghidra-work
flatpak run --filesystem=$W --env=JAVA_TOOL_OPTIONS=-Dapplication.settingsdir=$W/settings \
  --command=/app/lib/ghidra/support/analyzeHeadless org.ghidra_sre.Ghidra \
  $W/proj2 amipro -import $W/amipro-unpacked.exe -loader NeUnsignedLoader \
  -scriptPath $W/scripts -postScript DumpDecomp.java $W/out/amipro.c
```

The rebuilt file keeps the original bytes and appends the unpacked
segments (512-byte units) with 5,201 standard relocation records. Import,
analysis, and the dump take about three minutes. Ghidra finds 3,953
functions (the unpacked code has about 4,047 `push bp; mov bp,sp`
prologues) and decompiles all of them, with imported Windows calls by name
(`GETTEXTEXTENT`, `TEXTOUT`, `CREATEFONTINDIRECT`).

## What has been read from it

The decompile audit of 2026-10-12 mapped the SAM reader and writer and the
layout routines that use what they read: where each routine is, and the
findings marked VERIFIED (decompile), are in `SAM_FORMAT_NOTES.md` ("The
reader in AMIPRO.EXE" and "Layout rules from the decompile"). The section
names are reached through a table of near pointers at DGROUP 0xbe6, which
the decompile shows as `DAT_1680_0bXX` and `DAT_1680_0cXX`.

## The dialogs: AMIPROUI.DLL and AMIUICNT.DLL (2026-10-13)

Ami Pro's dialog templates, strings, and dialog procedures are not in
`AMIPRO.EXE` but in `AMIPROUI.DLL`, which is packed the same way (flags
`0x8B09`, 142 segments) and unpacks with the same tool; its exported
procedures keep their names (`MODLAYOUTPROC`, `PAGENUMPROC`,
`FLOATHEADFOOTPROC`, `MODFRAMEPROC`, `MODTAGPROC`, …), and it talks to
`AMIPRO.EXE` through a word queue (`AMIUIADDWORD`/`AMIUIREMOVEWORD`) and a
callback whose codes `AMIPRO.EXE` answers in `1130:0000`. The custom
controls (number boxes with units, unit buttons, colour bars, line-style
lists) are in `AMIUICNT.DLL`, which is not packed. `tools/ne_resources.py`
lists a module's dialog templates (control IDs, classes, labels) and
string tables, which tie a procedure's code to its fields.

```sh
tools/ghidra/optloader-unpack.py ../AmiPro-Reference/Binaries/amiproui.dll \
  ../AmiPro-Reference/ghidra-work/amiproui-unpacked.dll
# then analyzeHeadless as above, with a new project (proj3), the unpacked
# file, and out/amiproui.c; AMIUICNT.DLL imports as it is (proj4,
# out/amiuicnt.c). Each takes about two minutes.
tools/ne_resources.py ../AmiPro-Reference/Binaries/amiproui.dll dialogs "Page Layout"
```

The page layout findings are in `PAGE_LAYOUT_NOTES.md`.

## Menu commands and the handlers Ghidra misses (2026-10-17)

`AMIPRO.EXE` runs a menu command through a table in DGROUP at 0x46:
entries of (command ID, offset, segment), the segment filled by a
relocation, ended by command 0 (dispatcher `1010:0b07`; `WM_COMMAND`
reaches it from `AMIEXEMAINWNDPROC`). About 320 handlers are reached only
through this table, so Ghidra's analysis does not find them, or folds
them into the function before. The command IDs are in `AMIPROUI.DLL`'s
menu resources (type 4). `AMIPRO.EXE` does not import `AMIPROUI.DLL`: it
loads it and takes its exports by ordinal (`1028:08b0`), so the word
queue's `AMIUIREMOVEWORD` (ordinal 8) is called through `DAT_1680_19e0`.

```sh
W=$PWD/../AmiPro-Reference/ghidra-work
tools/ghidra/command_table.py $W/amipro-unpacked.exe 32a fd > $W/cmd-addrs.txt   # IDs in hex; none = all
cp tools/ghidra/DecompAt.java $W/scripts/
flatpak run --filesystem=$W --env=JAVA_TOOL_OPTIONS=-Dapplication.settingsdir=$W/settings \
  --command=/app/lib/ghidra/support/analyzeHeadless org.ghidra_sre.Ghidra \
  $W/proj2 amipro -process amipro-unpacked.exe -readOnly -noanalysis \
  -scriptPath $W/scripts -postScript DecompAt.java $W/cmd-addrs.txt $W/out/amipro-cmds.c
```

`DecompAt.java` creates a function at each listed address (splitting a
function that swallowed it) and decompiles it; `-readOnly` leaves the
project as it was. Any other missed address (a call to `func_0x…`) goes
through the same script. The ruler's findings are in `RULER_NOTES.md`.
