# Phase 1 SAM/STY Corpus Audit

**Date:** 2026-10-09
**Scope:** Read-only analysis of the verified Ami Pro 3.1 ISO. No reference bytes are stored in Git.

## Method

`tools/reference_inventory.py` extracted only `.SAM` and `.STY` files into a new private directory under `/tmp`, preserving ISO-relative paths and refusing to overwrite an existing directory. The Rust audit command then exercised the same bounded parser and semantic SAM importer used by the application:

```sh
cargo run -p amipro-formats --example corpus_audit -- /tmp/amipro-phase1-corpus
```

The parser stops at an explicit `[Embedded]` boundary or the first NUL byte. This avoids interpreting OLE, bitmap, audio, or other embedded binary payloads as section text.

## 2026-10-11 update

The importer was corrected against original Ami Pro 3.1 in the XP VM (rules
and evidence: `SAM_FORMAT_NOTES.md`). Untagged paragraphs are now Body Text,
escapes and character codes decode, and multi-line notes no longer end the
body early. That last defect had truncated 27 of the 160 documents. The audit
now also counts Body Text paragraphs and paragraphs whose imported text still
looks like control syntax.

| Measure | 2026-10-09 | 2026-10-11 |
|---|---:|---:|
| Imported logical paragraphs | 25,927 | 28,748 |
| Body Text paragraphs | not measured | 23,301 |
| Documents containing character runs | 123 | 145 |
| Imported bold/italic character runs | 10,174 | 11,151 |
| Distinct unimplemented inline control strings | 666 | 676 |
| Paragraphs with control-like text | not measured | 0 |

Imports, exact native round trips (160/160), STY parses (63/63), and the 30
unsupported section kinds are unchanged. The two paragraphs the leak check
first flagged quote literal `<<#>>` and `<"Version…">` text, which their raw
escapes confirm; the check now ignores `<` followed by ordinary text.

## Measured result (2026-10-09)

| Measure | Result |
|---|---:|
| SAM files | 160 |
| Successful SAM imports | 160 |
| Failed SAM imports | 0 |
| Exact imported-model → native → reopened-model round trips | 160 |
| Native round-trip failures | 0 |
| Imported logical paragraphs | 25,927 |
| STY files | 63 |
| Successful bounded STY parses | 63 |
| Failed STY parses | 0 |
| SAM files declaring an associated STY | 143 |
| Associated STY files resolved from this corpus | 0 |
| Distinct unsupported section kinds | 30 |
| Documents containing imported character runs | 123 |
| Imported bold/italic character runs | 10,174 |
| Distinct unimplemented inline control strings | 666 |

The 143 declarations name `SMARTEXT.STY` or `PRO21.STY`; neither file is present on the audited ISO. The other 17 SAM files carry embedded style definitions. Missing external styles are reported and named fallback styles are created; this is not style-fidelity verification.

Unsupported section kinds observed were: `book`, `data`, `docvars`, `e`, `elay`, `fldnames`, `flft`, `frght`, `frm`, `frmlay`, `frmmac`, `frmname`, `h`, `hlft`, `hrght`, `isd`, `l1`, `lay`, `lft`, `lyfrm`, `master`, `pg`, `recfile`, `repfrm`, `rght`, `sequence`, `tbl`, `tble`, `toc`, and `w`.

Paired `+!`/`-!` and `+"`/`-"` controls map to nested bold and italic character runs. This mapping is supported by repeated corpus context (emphasized headings, labels, titles, and nested combinations), but awaits original-application visual comparison. The most widespread remaining controls include indentation/position forms such as `:I720,0,0,0` and `:#480,8486`, plus field-like controls such as `/U`, `/T`, `/S`, and `/R`; their semantics remain unassigned without stronger evidence.

## Supported import subset

- Format-version and section discovery with explicit size/line bounds.
- Windows-1252 decoding after UTF-8 validation fails.
- `[edoc]` text extraction, physical-line joining, and paragraph separation.
- Named paragraph markers (`@style name@`).
- Embedded and supplied external `[tag]`/`[fnt]` records: style name, font family, point size, and observed low flag bits.
- Nested bold and italic character runs from paired inline controls, with UTF-8-safe byte ranges.
- Case-insensitive DOS-era STY filename resolution beside the SAM and in nearby `styles` directories.
- Safe conversion into native format v2 and exact paragraph-style/character-run round trip; native v1 files remain readable.

## Fidelity limits

Successful import means the supported subset was recovered without a parser failure. It does not mean visual or format compatibility. Character controls other than paired bold/italic, frames, tables, fields, embedded media, headers/footers, detailed paragraph geometry, and original-format export remain unsupported. Original SAM/STY overwrite is disabled.
