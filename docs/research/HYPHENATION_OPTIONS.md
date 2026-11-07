# Hyphenation: options for decision D2 (2026-10-13)

Phase 3 runbook, stop rule D2: hyphenation needs word patterns and a
dependency, and the core crates have none. This note gives the options.

## What Ami Pro does

From the decompile (`SAM_FORMAT_NOTES.md`, "Paragraph overrides" and "Style
records"):

- Only styles with Modify Style › Hyphenation (`[algn]` bit 256) are
  hyphenated. The word that does not fit at a line's end (at most 32
  characters) goes to the hyphenation library, `ltshyph1.dll`, which is not
  in the install set we have (its dictionary neither).
- A discretionary hyphen (Ctrl+-) in the word is used first; text marked
  No hyphenation (`<+.>`) is never split.
- A document setting from `[docopts]` (default 5) limits it, apparently a
  hyphenation zone in average character widths (inferred).

Our layout breaks only at discretionary hyphens today.

## How much it matters

- The user's corpus: 537 of 5,809 style records turn Hyphenation on, in 103
  files (308 of 3,896 records in 67 documents; the rest in style sheets).
- The ISO corpus: 40 of 702 records, in 14 files.
- Paragraphs in those styles break later than Ami Pro's wherever a word
  would have been split; the comparison report can count them once a
  hyphenator exists.

## Options

| Option | What | Licence | Size and maintenance | Fidelity |
|---|---|---|---|---|
| A. `hypher` crate (recommended) | Knuth–Liang hyphenation with TeX's hyph-utf8 patterns, compiled in; a feature per language (`english`) | MIT OR Apache-2.0; the en-US patterns carry a permissive notice | No dependencies; crate 862 KB with every language, English alone a small part; 0.1.8 (2026-09), maintained for Typst, about 3.6 M downloads | TeX's US English patterns, not Ami Pro's dictionary: words may split at other points |
| B. `hyphenation` crate | Knuth–Liang, same patterns, dictionaries built or embedded (`embed_en-us`) | Apache-2.0/MIT | Crate 3.4 MB; last release 0.8.4 (2021) | As A |
| C. The system's LibreOffice dictionaries (`hyph_en_US.dic`) through libhyphen | Patterns read at run time from the distribution's packages | libhyphen LGPL/MPL; dictionaries vary | A C library or a new reader; depends on installed packages | As A, varies by system |
| D. Ami Pro's own hyphenator | Decompile `ltshyph1.dll` for its rules | Its dictionary is Lotus data: cannot be shipped | We do not have the DLL; it would have to be found in the VM's install | Closest, but only the rules are usable, not the word list |
| E. None | Keep breaking only at discretionary hyphens | — | — | Hyphenated styles stay looser than Ami Pro's |

## Where it would live

A `Hyphenator` trait in `amipro-layout` (like `Shaper`: no dependency in
the core), with Ami Pro's rules around it (32 characters, discretionary
hyphens first, No hyphenation, the zone), and the implementation in the app
crate, which already has dependencies. Tests use a fixed hyphenator, as
they use `FixedShaper`.

## Recommendation

Option A, US English only, with Ami Pro's rules from the decompile, and
measured with the comparison report on the hyphenated styles. Option D's
rules (the zone) can be read from the decompile of `AMIPRO.EXE` without the
DLL.
