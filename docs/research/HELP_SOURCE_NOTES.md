# Ami Pro 3.1 help files as a source of facts (H1)

Status: tool built and run 2026-11-07. The `.hlp` files in `../AmiPro-Reference/Binaries/` are a **read-only
source of facts** for writing our own help. Their text is proprietary: it is never committed, quoted
(beyond a topic title) or embedded. All dump output and all fact sheets live outside Git under
`../AmiPro-Reference/help-dump/` (one folder per file, plus `FACTS.md`).

## Tools

- `tools/hlp_dump.py FILE.hlp [OUTDIR]`: standalone WinHelp 3.x reader (Python 3 stdlib only, opens the
  input read-only). Default output `../AmiPro-Reference/help-dump/<name>/`: one `NNNN_slug.txt` per topic
  (title, offset, context hashes, keywords, text, links with resolved target topic), `index.tsv`
  (number, topic offset, hash, title, chars, links, ok/FAIL, file), `_system.txt` (|SYSTEM data, internal
  file list, warnings).
- `tools/test_hlp_dump.py`: unit tests on a synthetic mini HLP built in the test (LZ77, compressed numbers,
  phrase expansion, whole-file dump). Run `python3 -I tools/test_hlp_dump.py`.

## What the reader handles (all eight files are WinHelp 3.1, minor version 21, topic blocks LZ77 compressed)

- Header and the internal directory (B+ tree, string keys); generic B+ tree leaf walk for the other trees.
- `|SYSTEM` (record list), `|Phrases` (uncompressed offset table followed by an LZ77 body; header is
  count, 0x0100, 4-byte expanded size), phrase expansion of text and titles.
- `|TOPIC`: 4096-byte blocks with 12-byte headers, LZ77 per block, topic links chained by position
  (positions step across blocks as `block << 14 | offset`; the decoded stream is the concatenation of the
  block bodies). Record types 0x02 (topic header), 0x20 (paragraphs), 0x23 (tables).
- Paragraph records: formatting commands live in LinkData1, text strings in LinkData2, one string taken per
  command (including the final 0xFF); hot spots (jump/popup by context hash, macro), pictures (skipped,
  marked `[picture]`), tabs, tables (cells joined by ` | `).
- `|TTLBTREE` (titles, keyed by cumulative text offset), `|CONTEXT` (hash -> topic offset), `|KWBTREE`/`|KWDATA`
  (keywords), `|CTXOMAP` (entry count only).
- Not handled: WinHelp 3.0 (minor <= 16) topic records, `|bmN` picture extraction, `|FONT`, `|KWMAP`, Hall
  compression (`|PhrIndex`; none of these files use it), window/browse-sequence interpretation. Context
  names are not recovered (only hashes; names appear in a few macro strings).

## Results per file

| File | Topics | Extracted ok / failed | Chars of text | Links (unresolved) | Keywords | Sample titles |
|---|---|---|---|---|---|---|
| amipro.hlp | 820 | 820 / 0 | 888182 | 3336 (84) | 1731 | Ami Pro Contents; Using Password Protect; Understanding Frames; Modifying Margins and Columns |
| amimacro.hlp | 757 | 757 / 0 | 764207 | 5060 (62) | 1181 | About; ActivateApp; ApplyFormat; MergeMacro |
| amichart.hlp | 26 | 26 / 0 | 22829 | 81 (2) | 73 | Charting Contents; Understanding Chart Types; Using Drawing to Modify a Chart |
| amidraw.hlp | 32 | 32 / 0 | 28947 | 170 (0) | 152 | Drawing Contents; Sizing an Object; Using Layered Objects; Using a Grid |
| amieqn.hlp | 53 | 53 / 0 | 58603 | 281 (10) | 276 | Equations Contents; Inserting an Operator; Editing an Equation; Saving an Equation as a TeX File |
| amifm.hlp | 10 | 10 / 0 | 9683 | 92 (0) | 55 | File Management Contents; Renaming a File; Exiting File Management |
| amiimage.hlp | 6 | 6 / 0 | 8512 | 30 (0) | 46 | Understanding Gray Scale Images; Using Preferences; Reverting to the Original TIFF File |
| helppack.hlp | 34 | 34 / 0 | 27303 | 33 (0) | 0 | Contents of the Working Together Bonus Pack; Lotus Application Manager |

"Failed" means a record raised an error while decoding; there were none. Counts exclude the end-of-chain
sentinel record. About 46 topics in amipro.hlp have no title; they are picture-only or tiny popup topics.

## Caveats

- Unresolved links (about 150 in total) point to context hashes that have no `|CONTEXT` entry; they are
  most likely dead links in the shipped files (several have a label equal to an existing topic title).
- 18 (amimacro) and 12 (amipro) topics have a "browse next" field that differs from the next topic offset;
  harmless (browse sequences need not follow storage order). Topic titles from the header record agree with
  `|TTLBTREE`, which supports the offset alignment.
- The 4 bytes after the paragraph header numbers and the picture record's trailing pad byte are skipped by
  observation, not by a published spec; text and links of every record in all eight files parse to a clean
  end, which is the check used.
- The help describes Ami Pro 3.1 for Windows; where our app deliberately differs (see DECISIONS.md) the
  help is a statement of the original's behaviour, not a requirement.
