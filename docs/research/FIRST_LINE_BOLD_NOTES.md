# First line bold (Modify Style › Font): research notes (2026-11-07)

Research, then implemented 2026-11-07 (section 6). Evidence: the decompile
(`ghidra-work/out/amipro.c`, `amiproui.c`), the dialog resources
(`tools/ne_resources.py Binaries/amiproui.dll dialogs`), a corpus scan, and VM
fixtures FLB0, FLB1, FLB2 (Windows XP, Ami Pro 3.1).

## Summary

- Ami Pro 3.1 calls it **"1st line bold"** (not "First line bold"). It is a
  check box on **Modify Style › Font** (not Alignment), beside "Initial caps"
  and "Dbl underline". Control id **25559 = 0x63D7**, mnemonic `&1`.
- It is stored in the style record's **`[fnt]` value 4, bit 4096 (0x1000)**,
  the bit `SAM_FORMAT_NOTES.md` and the model have so far called *protected
  text in a style*. That reading was wrong: the decompile shows this bit in a
  style is 1st line bold (below). **No style in either corpus has it set**, so
  the mistake never showed.
- Effect (VM, FLB1, FLB2): the **first visual line of each paragraph in the
  style** is set in bold, the line is broken using the bold widths, and the
  later lines are not bold. A hard line break (`<**>`) does not start a new
  "first line". Bold toggles typed inside that line change nothing visible.

## 1. Where it lives

Dialog template "Modify Style" (AMIPROUI.DLL, `caption "Modify Style"`
(26,16 310x204)), Font panel controls:

| id | label | note |
|---|---|---|
| 25551 / 0x63CF | `&Normal` | checked when no attribute is set |
| 25552 / 0x63D0 | `&Bold` | |
| 25553 / 0x63D1 | `&Italic` | |
| 25554 / 0x63D2 | `&Underline` | |
| 25555 / 0x63D3 | `&Word underline` | |
| 25556 / 0x63D4 | `&Dbl underline` | |
| 25557 / 0x63D5 | `A&ll caps` | |
| 25558 / 0x63D6 | `Initial &caps` | |
| **25559 / 0x63D7** | **`&1st line bold`** | at (226,109), third column, last row |

The Modify radio group (ids 25462–25468) is Font, Alignment, Spacing, Breaks,
Bullets & numbers, Lines, Table format; plus the Hyphenation check box (25550).
The user's guide's "Table format" and "Space for" are not on the Font panel.

Default: unchecked (the dialog loads `(attr_hi_byte & 0x80) != 0`
(`FUN_11c8_09e8`, `amiproui.c:51316`); a new style, `_DEFAULT.STY`'s styles and
all 2,600+ style records scanned have it clear).

Check-box behaviour (`FUN_11c8_026f` @ 11c8:026f, `amiproui.c:50806`):
- Bold (0x63D0) and 1st line bold (0x63D7) **exclude each other**: checking one
  unchecks the other. Underline and Word underline exclude each other; All caps
  and Initial caps likewise; any of them unchecks Normal; Normal unchecks all.
- `FUN_11c8_061f` (11c8:061f) enables Bold and 1st line bold **together**,
  only when the chosen face can be bold (otherwise both are disabled and
  cleared). Italic is enabled by the face's italic.
- `FUN_11c8_0917` (11c8:0917) builds the dialog's attribute word from the
  boxes (Bold 0x04, Italic 0x08, Underline 0x10, Word underline 0x20, Dbl
  underline 0x100, All caps 0x800, Initial caps 0x2000, **1st line bold
  0x8000**); `FUN_11c8_0349` handles Normal. This is Ami Pro's in-memory
  attribute word, not the file's.
- The panel's sample (`FUN_11c8_1b40`/`amiproui.c:50196`, `54378`) draws the
  sample text in bold when the box is checked (VM screenshot: "Times New
  Roman" appears bold in the preview with Bold unchecked).

## 2. Storage

In memory the style's flag word (record +8) uses 0x04 bold, 0x08 italic, 0x10
underline, 0x20 word underline, 0x40/0x80 super/subscript, 0x100 dbl underline,
0x200 strikethrough, 0x400/0x1 the face-kind bits, 0x800 caps, 0x1000 lower,
0x2000 initial caps, 0x4000 small caps, 0x8000 (protected text on a run; 1st
line bold on a style). `FUN_11c8_2631` (11c8:2631, `amipro.c:93745`) writes
the file's `[fnt]` value 4, `FUN_1058_0f24` (1058:0f24, `amipro.c:20279`; the
same code at 1318:4833) reads it back:

| in memory | file `[fnt]` value 4 |
|---|---|
| 0x8000 | **0x1000 (4096)** |
| 0x04 bold | 0x01 |
| 0x2000 | 0x800 |
| (others as in `SAM_FORMAT_NOTES.md`) | |

So 1st line bold is the **same bit as protected text** in the word (a run's
protection is the run attribute `<+*>`; a style's bit means 1st line bold).
`FUN_10d0_1aa6` (10d0:1aa6, `amipro.c:43448`) shows how the style's word is
used for text: `attr = style_flags & 0xfbfe` (drops the two face-kind bits),
and when the style's 0x8000 is set the bit is **cleared from the attribute
word** and **bit 0x80 of byte +6 of the line's attribute record** is set
instead. A style therefore never makes text protected.

In a SAM, a style stored in the document is a `[tag]` record exactly like an
STY's; nothing else records it (paragraphs of that style are written with no
extra mark; no per-paragraph override exists: Text menu has no such command).

VM byte diff (FLB0 = three Body Text paragraphs; FLB1 = the same after
Modify Style › Font › 1st line bold on Body Text, OK; both Save As in Ami
Pro; the Body Text record is in the document because Keep format with document
is on):

```
[tag] Body Text / 2 / [fnt]  Times New Roman / 240 / 0 / 49152   FLB0
                                                          53248   FLB1  (+4096 = 0x1000)
```

Other differences between FLB0 and FLB1 are the document-information stamp
(`[desc]` time values), the document's edit counter (`1`→`2`) and a window-geometry line; nothing else. The two files are the same
size only because 49152 and 53248 are both five digits. `Save as a Style
Sheet` (FLB1.STY, with contents) writes the same `[fnt]` value, 53248, in its
Body Text record. FLB2.SAM adds paragraphs that show text marks are
unchanged by the style: `<+">…<-">` (italic) and `<+!>…<-!>` (bold) inside
the first line, and a hard line break written `<**>`.

Corpus (read-only scan of `[tag]` `[fnt]` value 4; no names recorded):

| set | files | style records | with 4096 |
|---|---|---|---|
| user's STYLES | 192 | 1,913 | **0** |
| user's SAM files | 395 with records | 3,915 | **0** |
| ISO corpus (`/tmp/amipro-phase1-corpus`) | 116 | 702 | **0** |

Bit 13 (0x2000, dropped by the reader) and bit 12 are never set. Bits seen
in `[fnt]` value 4 across all three sets: 1 bold, 2 italic, 16 upper case,
2048 initial caps, and the face-kind bits 0x4000/0x8000 (underline 4 only
in the STYLES and ISO sets). So there is no sample of the
feature in either corpus; Ami Pro 3.1's own fixture is the only evidence of
how a real file looks, and the format is exactly one bit.

## 3. What it does

Font construction `FUN_10d0_0309` (10d0:0309, `amipro.c:42079`, used for
measuring and painting) takes the line's attribute record `param_5`:

```
weight = 700 if (rec.word0 & 4) != 0 || (rec.byte6 & 0x80) != 0 else 400
```

so the style's bit forces the bold weight of every font made from that
record. The record is the line's *starting attribute record* inside the line
structure (`line + 0x2a`, byte +6 = line + 0x30):

- `FUN_10d0_0d82` (10d0:0d82, the line-start routine, `amipro.c:42692`):
  `if (line.flags & 0x10) FUN_10d0_1aa6(line + 0x2a, style)` runs when the
  line begins a paragraph (flag 0x10); for a line that continues a paragraph
  the record is copied from the previous line and
  `line[0x30] &= 0x7f` and `line.flags &= 0xef` clear the bit. So **only the
  paragraph's first line carries it**, a paragraph continued after a page
  break is not bold at its top.
- Because the weight comes from the record, **bold toggles in the run
  attributes cannot turn it off** on that line, and italic and the other
  attributes combine with it (FLB2: `beta` is bold italic; a `<+!>…<-!>`
  pair in line 1 leaves the line bold).
- Line breaking uses the same fonts (the break routine measures through the
  same record), so the first line breaks where the **bold** text fits: FLB0's
  first line ended `…quickly, and` and in FLB1 it ends `…quickly,` with
  `and` wrapping, in both paragraphs 1 and 3.
- A hard line break (`<**>`, Ctrl+Enter) ends the first line but the next
  line is **not** bold (FLB2, last paragraph: "First segment" bold, "Second
  segment after a hard line break…" regular). First line means the first
  *laid-out line of the paragraph*, not up to the first hard break, not N
  characters or words. A one-line paragraph ("Short second paragraph.") is
  wholly bold.
- Interaction with Bold in the style: the dialog never stores both; a file
  with both bits behaves as bold (`rec.word0 & 4`) on every line.
- Other first-line state is not affected (indents, bullets, rules).

Open (VM not yet run): lines in a narrower slot beside a frame as a
paragraph's first line (expected the same: it is the line flag, not the
position), a paragraph whose first line is in a table cell, headers/footers,
and the first line of a paragraph continuing from a previous page.

## 4. Fixtures

In `../AmiPro-Reference/vm-lab/` (outside Git):

| file | content |
|---|---|
| `OUT/FLB0.SAM` | New document on `_DEFAULT.STY`, three Body Text paragraphs (long, one-line, long); Body Text unchanged |
| `OUT/FLB1.SAM` | FLB0 after Modify Style › Font › 1st line bold (Body Text `[fnt]` 53248) |
| `STY/FLB1.STY` | Save as a Style Sheet of the same, with contents (`[fnt]` 53248) |
| `OUT/FLB2.SAM` | FLB1 plus an italic word in a first line, a bold toggle in a first line, a paragraph with a hard line break |
| `screens/flb_sa.png`, `flb_after.png`, `flb_e.png` | screen captures of the off state, on state, and FLB2 |

Method: `docs/research/VM_LAB.md` (Modify Style by Ctrl+A, then clicking the
check box at (707,372) and OK at (391,426); Ami Pro exited with Alt+F X, the VM
powered off with `acpipowerbutton`). VM left powered off.

## 5. Implementation plan

**Model (`crates/document`)**
- Add `first_line_bold: bool` to `ParagraphStyle` (default false; native v3
  record `firstlinebold` written only when true, absent reads false).
- Stop treating a style's 4096 as protected: `CharacterAttributes.protected`
  stays a run attribute; a style's `attributes.protected` is always false
  (remove the comment "`[fnt]` 4096 in a style", `DECISIONS.md` 2026-10-12
  entry gets a correction line). `StyleAction` and `style_properties` drafts
  get `FirstLineBold(bool)`.

**Reader/writer (`crates/ami-formats`)**
- `lib.rs` `style_attributes`: do not set `protected` from 4096; set
  `first_line_bold: flags & 4096 != 0` in the `StyleSpec` (and `build.rs`'s
  `ParagraphStyle` construction).
- `writer.rs` `font_flags`: replace `(attributes.protected, 4096)` by
  `(style.first_line_bold, 4096)`; the run writer is untouched (`<+*>`).
- Round-trip: unchanged for the corpus (no record has the bit); add a unit
  test from the FLB1 `[fnt]` line (synthetic: 53248) and one with a style that
  writes back byte for byte. `sam_losses` needs no new case; `exchange`'s
  `Feature::ProtectedText` for styles becomes a `Feature::FirstLineBold`
  (reported as "not carried": DOCX/ODT/RTF/Markdown have no equivalent; the
  alternative of bolding the first line's runs is not faithful because the
  break is layout-dependent).

**Layout/paint (`crates/layout`, `app/cosmic/src/page_layout.rs`)**
- The paragraph's first line (engine `first_line`, `engine.rs` around line
  2096) is set in bold weight. `ShapeRequest` gets the style flag, and
  `ShapedParagraph` carries a second set of word widths (and ascent/descent)
  shaped bold, for the words that can fall on the first line; the breaker
  fills the first line with the bold widths and later lines with the regular
  widths. Simplest correct form: shape the whole paragraph twice (bold
  override and normal) only when `style.first_line_bold`, and let the line
  builder take `words_bold[i]` until the first line ends (a forced break
  ends it too), then `words[i]`. A word that straddles (hyphenated or
  wrapped later) is re-measured regular on its new line.
- Paint: spans on the first line use bold weight regardless of the run's
  `bold`; italic and the rest as set. A paragraph continued onto a new page
  does not repeat it (only the first line box of the paragraph).
- Incremental layout (`incremental.rs`): the paragraph's cache key includes
  the style flag, and the first line's breaks shifting must invalidate the
  paragraph only (no neighbouring effects).

**Style Properties (`app/cosmic/src/style_properties.rs`)**
- Font panel: add a "1st line bold" check box after Initial caps, using the
  toggle convention of its siblings; Bold and 1st line bold exclusive, enabled
  together, Normal unchecked as in Ami Pro (`FUN_11c8_026f`); the sample shows
  the first line bold; disabled when the face has no bold. Shown in
  `UI_SPEC.md` where it now says "First line bold ... not offered".

**Tests**
- Document/native round trip with the flag; STY/SAM read of a `[fnt]` value
  53248 sets it and 49152 does not; writer emits 53248 from the model and
  FLB1's Body Text record is reproduced.
- Layout (FixedShaper with a wider bold): a three-line paragraph breaks its
  first line earlier than FLB0 and the other lines are not bold; hard line
  break: second segment not bold; one-line paragraph is wholly bold; a
  paragraph continued across a page does not bold its continuation; bold
  toggles inside the first line change nothing.
- Scenario module in `app/cosmic/src/scenario_tests/` (style panel: tick the
  box, OK, check the page text's weight on line 1 only; untick; Undo).
- Physical check: compare against `flb_after.png` (VM) line for line.

**Open questions**
1. Frame/table cell/header cases (above), one VM run.
2. Does Fast Format carry it? (Not a text attribute; assumed it follows the
   style.)
3. Printing: the same font record is used for printing; a PostScript print of
   FLB1 would confirm, not expected to differ.

## 6. Implemented (2026-11-07)

As planned in section 5, with these details. Model `ParagraphStyle::first_line_bold`
(native `sfirstbold`); `[fnt]` 4096 read and written as 1st line bold, never
as a style's protected text; `Feature::FirstLineBold` (not carried by any
export format). Layout: `shape_with_first_line_bold` shapes a bold twin of
the words (`ShapedParagraph::first_line`); the engine breaks and sets the
first line (`start == 0`) with them in the main flow, frames and cells,
header/footer flows and footnotes, then continues with the regular words;
the shape cache and incremental layout key on the style, which now includes
the flag. The page surface draws that line bold (`first_line_bold` in
`page_surface.rs`); print and PDF draw through the same surface. Tests:
`crates/layout/tests/first_line_bold.rs`,
`crates/ami-formats/tests/first_line_bold.rs` (FLB records byte for byte with
`AMIPRO_VM_LAB`), `scenario_tests/styles.rs`, the layout comparison.
Open: Fast Format does not carry it (it copies character formats and
re-applies a style, which carries the flag with the style; not checked in
Ami Pro), frames/cells/headers/PostScript remain VM questions.
