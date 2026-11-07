# Private Reference Audit

**Audit date:** 2026-10-09  
**Method:** Read-only ISO-9660 directory parsing, hashing, file-type identification, and string/resource inspection. No reference executable was run.

## Source media

The supplied image is `Reference/lotus-ami-pro-3.1.iso` (253,321,216 bytes), SHA-256 `87c47f1ff1134a682ea8bd9ae957c4bf652c9f34b0af5b090a9dd44d4d37cd2a`. Its ISO-9660 volume label is `AMIPRO31`; its primary descriptor identifies `ELEKTROSON BV 1994` and a 1994-11-30 creation timestamp.

This is not merely a standalone Ami Pro installer. Directory and installer evidence identify it as a Windows Lotus SmartSuite CD distribution containing 1-2-3, Approach, Freelance Graphics, Organizer, shared tools, and Ami Pro. Ami Pro must therefore be evaluated independently from the suite bundle.

## Version and platform

`/FROMROM/AMIPRO/AMIPRO.EXE` is a 16-bit New Executable (NE) for Microsoft Windows 3.x. Its Windows version resource contains:

- `FileDescription`: `Lotus Ami Pro 3.1`
- `FileVersion`: `3.1.0.0`
- `ProductName`: `Ami Pro 3.1`
- `ProductVersion`: `3.1.0.0`
- `OriginalFilename`: `AMIPRO.EXE`

Its SHA-256 is `555506d1558d61579d5c6fee8bf5fa9d960aa05a20a5d171240ac2e0ea73cbbd`. This directly verifies the target as Lotus Ami Pro 3.1 for 16-bit Windows 3.x; it is stronger evidence than the ISO filename or label alone.

`INSTALCD.EXE` is NE for Windows 3.0 and `/FROMROM/INSTALL.EXE` is NE for Windows 3.10. Representative DLLs are also 16-bit NE. `AMIPRO.HLP` is Microsoft Windows 3.0 help format. The installer uses `INSTALL.DSK` archives plus LSO/LSS installer scripts listed by `INSTALL.LST`.

## Inventory

The image contains 4,459 files in 89 directories (246,635,975 aggregate logical file bytes). Counts cover the entire SmartSuite image, so not every item belongs to Ami Pro.

| Type | Whole image | Ami Pro tree |
|---|---:|---:|
| EXE | 93 | 5 |
| DLL | 335 | 66 |
| HLP | 39 | 8 |
| SAM | 160 | 14 |
| STY | 63 | 63 |
| SMM macro | 131 | 130 |
| QST tutorial/template resource | 40 | 40 |
| FLT import/export filter | 84 | 49 |
| BMP | 1,831 | 97 |

The dedicated `/FROMROM/AMIPRO` tree contains 656 files totaling 19,532,112 bytes. Other SAM files are online manuals, suite samples, and Organizer integration data.

Important groups include:

- Core behavior/resources: `AMIPRO.EXE`, `AMIPROUI.DLL`, `AMIUICNT.DLL`, icon tables, initialization files.
- Help: `AMIPRO.HLP`, `AMIMACRO.HLP`, drawing/equation/image/file-manager help.
- Documents: `DOCS/README31.SAM`, `AP31NAV.SAM`, Mercury mail-merge examples, and `CDNAV/DOCONLIN/AMIPRO/STLGUIDE.SAM`.
- Styles/templates: 63 STY files including `_DEFAULT.STY`, `_ARTICLE.STY`, letters, reports, outlines, newsletters, labels, indexes, TOC, calendars, and mail-merge styles.
- Format/filter clues: RTF, Word, WordPerfect, TIFF, PCX, EPS, Windows Write, Lotus Write, spreadsheet/database, and graphic filters.
- Behavior clues: named SMM macros and QST tutorial resources cover frame operations, styles, tables, mail merge, indexes, TOC, printing, equations, drawing, spellcheck, conversion, and navigation. These names are evidence of shipped resources, not yet behavioral confirmation.

## SAM/STY evidence

Representative files are section-oriented text formats rather than opaque containers. Both `README31.SAM` and `_DEFAULT.STY` begin with `[ver]` followed by `4`, then sections such as `[sty]`, `[files]`, `[charset]`, `[tag]`, `[fnt]`, `[algn]`, `[spc]`, `[brk]`, and `[nfmt]`. `README31.SAM` describes itself as the Ami Pro Release 3.1 readme and declares Windows CP-1252.

The initial read-only discovery parser successfully processed private copies extracted to `/tmp`:

- `README31.SAM`: 126 sections, 8 style tags, format version 4; 48 section instances currently unknown.
- `_DEFAULT.STY`: 97 sections, 9 style tags, format version 4; 13 section instances currently unknown.

The parser retains unknown sections, decodes invalid UTF-8 input deterministically as Windows CP-1252, and always reports original overwrite as unsafe. Semantic section mapping remains incomplete.

## Research priorities and boundaries

The HLP files, style guide, tutorials, diverse STY corpus, sample SAM documents, UI resource DLLs, filters, and macro names are valuable for reconstructing documented behavior and file structures. Proprietary artwork, fonts, dictionaries, binaries, and resource data must not enter this repository or derived tests.

The checked-in `tools/reference_inventory.py` reads ISO-9660 extents without mounting or executing the image. `Reference/` is ignored by Git. Only hashes, counts, filenames, and independently written findings are recorded here.

