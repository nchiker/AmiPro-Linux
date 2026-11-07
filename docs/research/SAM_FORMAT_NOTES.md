# SAM Format Notes

**Started:** 2026-10-11. This is an independently written description of the
Ami Pro 3.1 `.SAM` format, built from observed behavior. It grows as Phase 2
fixtures are decoded. No original document bytes are stored in Git.

## Evidence levels

- **VERIFIED:** observed in Ami Pro 3.1 in the `Windows XP` VM. Either Ami Pro
  wrote the file from typed input, or Ami Pro opened a synthetic file and its
  display or status bar was read. Lab procedure: `VM_LAB.md`.
- **CORPUS:** consistent across the 160-file private corpus (and, from
  2026-10-12, the user's private 423-file corpus where stated), but not
  observed in Ami Pro.
- **INFERRED:** a reasonable reading that still needs a controlled fixture.
- **DECOMPILED:** read from decompiled reference code. Decompiler output
  stays outside Git (`../AmiPro-Reference/ghidra-work/`); only findings are
  recorded here.
- **VERIFIED (decompile):** read in Ami Pro 3.1's own code (`AMIPRO.EXE`,
  decompiled 2026-10-12; `DECOMPILATION.md`): the SAM reader and writer and
  the layout routines that use what they read. Stronger than DECOMPILED,
  which also covers the third-party filter. Where to find each routine is in
  "The reader in AMIPRO.EXE" below.

The VM fixtures are private (`../AmiPro-Reference/vm-lab/`):

| Fixture | How it was made | What it shows |
|---|---|---|
| T01 | New document, `_DEFAULT.STY`; three typed paragraphs including `[`, `<`, `>`, `\` | Body Text writing, `[`/`<`/`>` escapes, a minimal page layout |
| T02 | New document, `_DEFAULT.STY`; Alt codes 128–255, a tab, ASCII punctuation, and a paragraph that starts with `@` | Character-code escapes, tab, leading `@` |
| T03 | New document, `_TOC.STY` (Body Text is the 11th style); two typed paragraphs | Body Text paragraphs are untagged even when Body Text is not first |
| T04, T05 | Synthetic SAM files opened in Ami Pro | Where `@@` is read as `@` |
| F01–F11 | New `_DEFAULT.STY` document with two main paragraphs and one text frame made with Frame › Create Frame (2.00 × 1.50 in, 2.00 in down, 1.50 in from the left, text "Frame text here."). Each variant changes one setting: position (F02), size (F03), No wrap around (F04), No wrap beside (F05), With para above (F06), Flow with text (F07), Repeat all pages (F08), Repeat right/left (F09), Transparent (F10), Square corners (F11) | Frame records, flags, anchoring, repeating frames |
| C09, C11 | As F01, with the Lines and Columns panels visited but nothing changed | Controls for unchanged output |
| L01–L15 | As F01, one Lines & shadows setting each: shadow None (L01), Deep (L02), bottom-left (L03); lines on Left (L05), Right (L06), Top + Bottom (L07); third line style (L08); line position Close to outside, Outside, Inside, Close to inside (L09–L12); red line, shadow, background (L13–L15). L04 (custom depth) could not be made | Line, shadow, and colour fields |
| C01–C04, M01–M04 | As F01 with 2 columns (C01), 3 columns (C02), a left tab at 0.50 in (C03), 2 columns + line between (C04); one frame margin set to 0.30 in: right, bottom, left, top (M01–M04) | Frame layout (`[frmlay]`) |
| P01–P03 | With para above + No wrap around (P01); a synthetic 64 × 48 BMP imported into the F01 frame with Copy image (P02) and linked (P03) | Placement encoding; picture frames and embedded data |
| Z01–Z05 | Two overlapping frames (Z01), the second sent to back (Z02), two and three frames grouped (Z03, Z04), two Flow with text frames (Z05) | z-order, groups, anchor numbering |
| X01–X05 | Synthetic edits opened in Ami Pro: F06 with F07's flag word (X01), F07 with its anchor moved into paragraph one (X02), Z05 with field 14 swapped (X03), Z05 with a Where placed frame inserted first (X04), F07 with field 16 changed (X05) | How Ami Pro reads anchored frames |
| G00–G23 | New `_DEFAULT.STY` document with the F01 main paragraphs. G00 unchanged; one page-layout change each: top, bottom, left, right margin 1.50 in (G01–G04); 2 columns (G05), + column balance (G06); lines around the page (G07), + Outside (G08), + red (G10); 2 columns + line between (G09); Legal, Landscape, A4, Custom 6 × 9 in (G11–G14); left pages with a 1.50 in left margin (G15); right pages mirrored (G16), + 1.50 in left margin (G17); fixed header text with a page number (G18); header and footer Begin on second page (G19, G23); page break (G20); inserted page layout on page 2 (G21); floating header (G22) | Page layouts, headers/footers, breaks |
| F12, F13, TB1 | F01 frame with two paragraphs, the second Body Single (F12); a frame created on page 2 (F13); a 2 × 2 table in the main text (TB1) | Frame text styles, frame page number, table frames |
| V01, V05, V06 | Behaviour checks: a Where placed frame on page 1 with a page break then inserted before all text (V01); deleting at an anchor mark (V05); selecting text containing an anchor mark, deleting, and undoing (V06, not saved) | Frames keep their page; anchors delete and restore their frames |
| V10, V11, X06 | A page break and a Where placed frame created on page 2 (V10); the break then removed so all text fits on page 1 (V11); V11 with frame field 1 changed to 3, opened in Ami Pro (X06, synthetic) | A fixed frame keeps its page, and Ami Pro adds empty pages so that page exists |
| V12, V13 | A 2.00 × 1.50 in With para above frame whose one-line paragraph is pushed toward the bottom of page 1 by 41 empty paragraphs (V12); the same frame switched to Flow with text (V13) | Anchored frames that do not fit move to the next page with their paragraph or line |
| V14 | Fixed header text; a Where placed frame and then a Repeat all pages frame, overlapping each other and the header text; Bring to Front on the repeating frame and Send to Back on the page frame | Page frames always paint above repeating frames; header text wraps around frames |
| V15, V16 | Two grouped frames dragged, deleted and restored with Undo, dragged again (V15), then ungrouped (V16) | A group moves and deletes as one action |
| V17, V18 | A floating footer for all pages (V17); a floating header for left pages (V18); the dialog's Remove option tried at and away from the mark | Floating header and footer flags |
| V19 | Ctrl+Enter, Ctrl+-, and Ctrl+Space typed inside one paragraph | Hard return, discretionary hyphen, no-break space |
| V20 | A table, a Flow with text frame, and a second table, created in that order | `<:t n>` and `<:A n>` share one numbering |
| V21 | Tools › Tables with a Where placed frame selected | Frame tables |
| V22, V23 | A column break (V22); a page break with "Vertically center text above page break" (V23) | Break forms |
| `README31.SAM`, `MERCURY.SAM` | Shipped documents opened in Ami Pro | Untagged paragraphs show Body Text; text after a multi-line note is displayed |
| I1–I3 (2026-10-12) | New `_DEFAULT.STY` document with one long paragraph; Body Text changed in Style › Modify Style › Alignment: All 0.50, First 1.00, Rest 0.25, From right 1.50 in (I1); then Both sides, Hanging indent, and Justify (I2); then Both sides off and Left (I3) | `[algn]` and the right indent |
| B1–B8 (2026-10-12) | The S fixtures' document; Body Text changed in Style › Modify Style › Breaks: Page break before (B1), after (B2), Column break before (B3), after (B4), Allow page/column break within cleared (B5), then Keep with next (B6), Keep with previous (B7), and Next style (B8) | `[brk]` |
| S1–S6 (2026-10-12) | New `_DEFAULT.STY` document with three paragraphs; Body Text changed in Style › Modify Style › Spacing: 1 1/2 (S1), Double (S2), Custom 0.25 in (S3), Custom 0.20 in with Above 0.10 and Below 0.20 in, When not at break (S4), Single with Add in Always (S5), Text Tightness Tight (S6) | `[spc]` |
| F14–F20 (2026-10-12) | F01's frame moved and resized with the mouse at 87.4 pixels per inch: dragged from inside by 88, 44 pixels (F14), then its bottom-right handle by 44, 22 (F15); a With para above frame dragged by 88, 44 (F16), its top handle down 44 (no change), then its left handle left 44 (F17); a Flow with text frame dragged by 88, 44 (F18), its bottom handle down 44 (F19), and a paragraph typed after it (F20) | How Ami Pro moves and resizes each placement (`UI_SPEC.md` §11 gate) |
| LH1 (2026-10-12) | Synthetic SAM (G00's header and Body Text block; 30 added styles in Times New Roman and Arial at 8–72 pt, Courier New 10 and 12, bold and italic variants; one short paragraph each and three long ones) opened and saved as `LH1OUT.SAM` | Single line heights from the `<:#h,w>` caches (`LAYOUT_COMPARISON.md`) |
| `AP31NAV.SAM` (2026-10-12) | Shipped document opened, ruler shown with the caret in Body Text | Body Text's right indent is the last value of its `[tag]` |
| O1–O4 (2026-10-12) | New `_DEFAULT.STY` documents, one setting per word or paragraph from the Text menu: Underline, Word Underline, the four Caps, and every Special Effect (O1); Alignment Center, Right, Justify, Left (O2); Indention All 0.25, First 0.50, Rest 0.75, From right 1.00 in, and Spacing 1 1/2, Double, Custom 0.30 in, Single (O3); Font: Arial 18, Courier New 8, size 14, face only, red (O4) | Character attributes and paragraph overrides in the body |
| O5–O7 (2026-10-12) | Synthetic SAM files (O4's header) opened in Ami Pro: nested and partial `<:f>`, `<:S->`, and marks in mid-paragraph (O5); two spacing, alignment, or indent marks in one paragraph, and nested underline (O6, O7) | How Ami Pro reads those controls |
| LS1 (2026-10-12) | Modify Style on four styles: Arial 10 at 1 1/2 and Double, Courier New 24 at both, Times New Roman 24 at both | The line heights Ami Pro computes |
| K1–K6 (2026-10-13) | `README31.SAM`, `MERCURY.SAM`, G21, a 30-page user document without frames, a 13-page user document with frames, tables, and pictures, and P02, each opened and saved again by Ami Pro as it was (`…R`), with `[pg]` and every `<:#…>` removed (`…S` → `…T`), and with only `[pg]` removed (K1, K2, K5: `…P` → `…Q`); K1 with every cache's height doubled and its width kept (K1W) or changed (K1X) | Whether Ami Pro needs the layout caches ("Writing documents") |
| RPT2A, N-twins, FA1–FA3, FG1–FG6 (2026-11-05) | File › New with contents and Run macro off from nine shipped sheets (`_REPORT2` as RPT2A; `_REPORT4`, `_PROPOS2`, `_NEWSLT1`, `_NEWSLT4`, `_INVOICE`, `_MEMO1`, `_FAX3`, `_TITLE2` as NRPT4, NPRP2, NNWS1, NNWS4, NINV, NMEM1, NFAX3, NTTL2), saved as SAM; synthetic SAM files on RPT2A's header opened and saved again (`…R`): Flow with text frames 1440 and 8296 twips wide at a paragraph's start, one after "Enter ", with the Deep shadow (FA1) and none (FA2), and 3800, 3500, 3300 (Deep), 3394 and 3500 (none) wide after a 720-twip first-line indent (FA3); a Where placed Wrap around frame leaving gaps of 20, 200, and 400 twips beside empty lines, "a", "Word", and "Abcdefghij" (FG1–FG3); NFAX3 without its first paragraphs' caches (FG5) and with its picture moved down the page (FG6) | A new document from a sheet is the sheet's text and frames as Ami Pro lays them out (`new_documents_match_ami_pros`); Flow with text frames' shadows, width, and place in an indented line; which gaps beside a frame take a line; a frame beside a column's first lines adds the space above (`SAM_PAGE_FRAME_SPEC.md` §5.1) |

## Files that are not ordinary SAM files

From the user's corpus (2026-10-12), 7 of its 423 `.SAM` files and 1 of its
192 style sheets:

| Case | What the file holds | Importer |
|---|---|---|
| Lotus Word Pro documents named `.SAM` (5 files) | A binary file that starts with `WordPro` (Ami Pro's successor's format) | Refused: "a Lotus Word Pro document, not an Ami Pro document" |
| A password-protected document (1) | An `[encrypt]` section, then binary data with NUL bytes; no other section is readable | Refused: "password-protected" (opening protected documents is a gap) |
| A truncated document (1) | Exactly 2,048 bytes, cut off inside a `[tag]`; no `[edoc]` | Refused: no body |
| A style sheet whose embedded drawing lost its NUL bytes in a copy (1) | Intact text up to the body's closing `>`, then binary data without NUL bytes and an `[Embedded]` directory whose trailer points past the end of the file, then a DOS end-of-file byte | Read: after `[edoc]`, a line that starts with `[` but is not a section header ends the sections, with a warning (body text writes `[` as `<[>`). Before `[edoc]` it is still an error |

Ami Pro reads style sheets from a directory of its own, not the document's
(`VM_LAB.md`: `…\AMIPRO\STYLES`). The importer looks for a document's `[sty]`
beside the document, in a `styles` directory there or one level up (any
letter case, as copies of DOS trees keep `STYLES`), then in the directories
it is given (`import_sam_path_with_style_directories`; `AMIPRO_STYLES` for the
audit and the comparison). In the user's corpus, 120 documents name a style
sheet, all found in its `STYLES` directory; the other 296 that import carry
their styles only.

## Document body (`[edoc]`)

| Rule | Evidence |
|---|---|
| A blank line ends a paragraph that has text. **A blank line where a paragraph starts is itself an empty Body Text paragraph**, and **a line that is only a style tag (`@Body Single@`) is a whole empty paragraph** in that style, with no blank line after it (Ami Pro writes an empty paragraph without a cache that way). The reader takes each line up to its line break, trimming nothing: a line holding only a tab is a paragraph. Before 2026-10-12 the importer joined a bare tag line to the next line and dropped blank lines at a paragraph's start, losing empty paragraphs in 93 of the user's files (and 146 bare tags in 38) and 11 ISO files, which shifted every later paragraph. | VERIFIED (decompile `1118:0e55`, `1118:0920`; VM fixture EP1, read and written back by Ami Pro as nine paragraphs; a user file whose two trailing blank lines Ami Pro laid out on a second page and wrote back with caches) |
| Ami Pro wraps long paragraphs onto several physical lines. A wrap can fall between two controls. Wrapped lines join with no separator, because the trailing space is kept on the earlier line. | VERIFIED (T02) |
| The body ends at a line containing only `>`, followed by a blank line and `[Embedded]` with an eight-digit number. | VERIFIED (T01–T03; decompile: exactly `>`) |
| The page table (`[pg]`) can be stale: one user file's table ended on page 1 at paragraph 33, while Ami Pro, opening it, laid out 35 paragraphs on 2 pages and rewrote the table. Page-end comparisons inherit this. | VERIFIED (VM, 2026-10-12) |
| Some controls span lines and contain blank lines and their own lone `>` line: notes (`<:N…`) and `<:h…`. The body-end and paragraph-break rules apply only outside controls. Ignoring this truncated 27 of 160 corpus documents. | VERIFIED (`MERCURY.SAM` displays through its final paragraph); CORPUS (only `<:N` and `<:h` span lines) |
| A paragraph may start with `@Style Name@`. | VERIFIED |
| **An untagged paragraph is Body Text.** Ami Pro writes Body Text paragraphs without a tag, wherever Body Text sits in the style sheet and whether or not it has a function key. The style sheet in use supplies Body Text's formatting: Times New Roman 12 in `_DEFAULT.STY`, Arial 12 in `_TOC.STY`. A tag applies to its own paragraph only; it does not carry over. | VERIFIED (T01, T03; status bar on untagged `README31.SAM` paragraphs) |
| Every one of the 63 shipped style sheets defines Body Text. In 3 of them it is not the first style. | CORPUS |
| A tab is stored as a literal tab character. | VERIFIED (T02) |
| `<+!>`…`<-!>` is bold and `<+">`…`<-">` is italic; they nest. | VERIFIED visually in `README31.SAM` ("Read This First" italic, "Note" bold) |

### Escapes and character codes

The body is Windows-1252. Bytes 0x00–0x7F that are not special appear as
themselves. The following forms were written by Ami Pro for typed characters
(T01, T02) and display correctly when read:

| Character | Written as | Notes |
|---|---|---|
| `[` | `<[>` | `]` is literal |
| `<` | `<<` | Also inside controls; it does not open a control |
| `>` | `<;>` | |
| bytes 0x80–0xBF | `</X>`, where X = byte − 0x40 (`@` … DEL) | `</`>` is 0xA0 no-break space; `</n>` is 0xAE ®; `</m>` is 0xAD, which Ami Pro shows as a hyphen and breaks after as after `-` (T02; `10d0:1f9e`), not the discretionary hyphen `<*7>` (read as U+2010 since 2026-10-13) |
| bytes 0xC0–0xFF | `<\X>`, where X = byte − 0x80 (`@` … DEL) | `<\i>` is 0xE9 é; `<\\>` is 0xDC Ü |
| `@` | `@` | Doubled only at the first text position, below |

**Leading `@` (VERIFIED, T02/T04/T05).** When a paragraph's first text
character is `@`, Ami Pro doubles it. When reading, `@@` becomes `@` only at
the first text position: after the optional tag and after any leading
controls, such as `<:#…>` or `<+!>`. Later in the paragraph `@@` stays `@@`. A
paragraph that starts with `@@` therefore has no tag. A single `@` with no
closing `@` is literal. Example from the corpus: `<:#284,10800>@@@("…")`
reads as `@@("…")`, the 1-2-3 `@@` function.

### Character attributes (O1, O6)

Ami Pro writes each Text menu attribute as an on and off pair around the
text, which is stored as typed (the caps attributes change only the
display). `<-X>` turns the attribute off however many `<+X>` came before:
`<+#>a<+#>b<-#>c<-#>` underlines only `ab` (O6).

| Attribute | On, off | Evidence |
|---|---|---|
| Bold, Italic | `<+!>` `<-!>`, `<+">` `<-">` | VERIFIED |
| Underline, Word underline, Double underline | `<+#>` `<-#>`, `<+$>` `<-$>`, `<+)>` `<-)>` | VERIFIED (O1) |
| Upper case, Lower case, Initial caps, Small caps | `<++>` `<-+>`, `<+,>` `<-,>`, `<+->` `<-->`, `<+(>` `<-(>` | VERIFIED (O1) |
| Superscript, Subscript, Strikethrough | `<+&>` `<-&>`, `<+'>` `<-'>`, `<+%>` `<-%>` | VERIFIED (O1) |
| Overstrike with character C | `<:O+C>` … `<:O-C>` | VERIFIED (O1; decompile `11b8:030a`, `10d0:2d0e`: a 3-byte attribute record, `+` sets the character in force and a new one replaces it, `-` clears it whatever its character; C may be an escape). Drawn over each character of the run, widths unchanged (O1, PT1) |
| No hyphenation (Edit › Mark Text) | `<+.>` `<-.>` | VERIFIED (decompile): the hyphenator never breaks text inside it |
| Protected text (Edit › Mark Text) | `<+*>` `<-*>` | VERIFIED (decompile): set and cleared over a selection with the protection commands; editing in PT1 (below) |

How the reader applies them (VERIFIED (decompile)):

- `<+X>` and `<-X>` become an attribute-change byte pair in memory (X, or X
  with bit 7 for off), so they nest and repeat freely.
- **Off returns to the style, not to plain:** `<-!>`, `<-">`, `<-#>`,
  `<-$>`, `<-)>` and the four caps give back the paragraph style's own
  setting (a bold style stays bold after `<-!>`). Strikethrough,
  superscript, subscript, no hyphenation, and protected text simply end.
- The caps attributes exclude each other: each one clears the other three
  first.
- Superscript and subscript are drawn at 7/12 of the size; small caps at
  7/10 (the font for the lower-case letters).
- Only `!"#$%&'()*+,-.` are attributes. `<+@>`…`<+C>` are the paragraph
  alignment (below); any other `<+X>` is kept but does nothing. `<+/>`, once
  listed here as unknown, occurs in neither corpus.
- The underlines are three separate bits, so they can combine; so can
  superscript and subscript (superscript wins when both are set).

How the display applies them (VERIFIED (decompile), 2026-10-12; the run's
font `10d0:0309`, the caps transform `14f0:01c4`, the drawing offset
`1108:4063`, the line's metrics `10d0:359b`):

- Bold, italic, underline and word underline, and strikethrough are the
  Windows font's own weight, italic, underline, and strike-out. Double
  underline is drawn separately.
- **Superscript and subscript** take 7/12 of the size (`size × 7 / 12`,
  truncated, in twips). A superscript's baseline is raised by 5/12 of the
  full size; a subscript's is lowered by half the smaller font's ascent.
  **Small caps** take 7/10 of the size for the lower-case letters (and the
  spaces) of the run, which are shown as capitals; capitals, digits, and
  punctuation keep the full size. A line holding such text is as tall as
  the full-size font would make it: the smaller font's ascent and descent
  are scaled back up by full size / smaller size.
- **Caps** change only the display: Upper case and Small caps show the
  text with Windows' `AnsiUpper`, Lower case with `AnsiLower`. Initial caps
  shows the first letter of each word upper case and every other letter
  lower case. A word is a run of letters by Ami Pro's Windows-1252
  character table (DGROUP 0xf89: ASCII and the accented letters; digits
  and punctuation end a word, so "3rd" shows as "3Rd"), and an apostrophe
  between two letters stays inside the word ("Don't"). The small-caps split
  reads the same table for lower case and spaces; for bytes above 0x7F it
  indexes it with a sign-extended byte, so accented lower-case letters may
  not shrink there (not checked in the VM).

**Protected text in editing** (PT1, 2026-10-12; the user's guide, chapter
5): Right at the start of protected text puts the caret after it, Left at
its end before it, and Shift+Right or Shift+Left selects it whole. A click
anywhere in it (near its start too) puts the caret after it. Delete and
Backspace remove one protected character. Text typed where an arrow left the
caret, at either edge, is not protected (saved as `ab<+*>cd<-*>Xef` and
`ghY<+*>ij<-*>kl`); a selection holding all of it is replaced by plain text.
Text typed after deleting into it is protected (`mn<+*>Zp<-*>qr`,
`<+*>protected words herB<-*>`): Ami Pro's caret then stands between the
attribute marks. Ours keeps text typed at either edge outside it, so this
last case differs. VERIFIED (PT1, PT1OUT).

Enter carries the attributes in force at the caret into the new paragraph
(EN1, typed in the VM and saved): `aa `, Ctrl+B, `bold`, Enter, `x` wrote
`aa <+!>bold<-!>` and `<+!>x` (x bold, never closed); Enter after plain
text started a plain paragraph; Home and Enter before `<+!>zz<-!>` wrote an
empty paragraph `<+!>` above it. VERIFIED (EN1).

### Paragraph overrides (O2, O3, O5–O7)

Text › Alignment, Indention, and Spacing set one paragraph over its style.
Each is a mark that applies to its whole paragraph wherever it stands in
it (O5, O6). In both corpora a paragraph almost never holds two different
marks of one kind (3 of about 28,000).

| Override | Written as | Evidence |
|---|---|---|
| Alignment | `<+@>` left, `<+A>` right, `<+B>` centre, `<+C>` justify (no off mark) | VERIFIED (O2) |
| Indention | `<:I ALL,FIRST,REST,RIGHT>` in twips; First and Rest add to All, as in styles | VERIFIED (O3 display) |
| Line spacing | `<:S+-1>` Single, `<:S+-2>` 1 1/2, `<:S+-3>` Double, `<:S+N>` Custom N twips | VERIFIED (O3) |

- Two spacing or indent marks: the later one applies (O6). VERIFIED
  (decompile): both write one per-paragraph record. `<:I` takes up to four
  numbers in order (All, First, Rest, Right) and leaves the rest as they
  were.
- **Two alignment marks combine** (VERIFIED (decompile), corrects the
  earlier "largest code wins" reading): the marks are OR'ed into a
  three-bit field, left 1, right 2, centre 3, justify 4. The result 1–4 is
  that alignment; 0 and 5–7 mean "the style's alignment". So left and right
  together make centre, centre with left or right stays centre (O6, O7),
  and justify with any other mark gives the style's alignment.
- `<:S->` changes nothing: `<:S+-3><:S->` is still double spaced (O5). It
  appears in pasted corpus text between `<:S+…>` marks. VERIFIED
  (decompile): only `<:S+` sets anything.
- `<:c>` (column break) is a paragraph flag: the column ends **after** the
  paragraph that holds it, wherever the mark stands in it (VERIFIED
  (decompile); V22 wrote it at the start of the paragraph whose end held the
  caret).
- Ami Pro writes no height for a 1 1/2 or Double override. The height
  matches the style setting's: 1.5 and 2 times the single line for Arial 10
  (345 and 460 for 230) and Courier New 24 (777 and 1,036 for 518). Times
  New Roman comes within 3% of that (12 pt: 432 and 561; 24 pt: 835 and
  1,123). VERIFIED (LS1).

### Font changes (O4, O5)

`<:fSIZE,KFACE,R,G,B>` … `<:f>` sets the typeface, the size in twentieths
of a point, and the colour (0–255 each) of the text up to `<:f>`. The
Font dialog writes every field: Arial 18 is `<:f360,BArial,0,0,0>` and red
Times New Roman 12 is `<:f240,2Times New Roman,255,0,0>` (O4). An empty
field keeps the style's: `<:f360,,>` changes only the size and
`<:f,BArial,>` only the face; `<:f,,>` changes nothing (O5). `<:f>`
returns to the style's font, not to an earlier `<:f…>`: in
`<:fA>big <:fB>small<:f> after<:f>` the word "after" is in the style's
font (O5).

K is the Windows pitch-and-family byte plus 0x20: `B` (0x22) Arial,
`Q` (0x31) Courier New, `2` (0x12) Times New Roman (O4). That is the family
in the high nibble (1 roman, 2 swiss, 3 modern) and the pitch in the low
two bits (1 fixed, 2 variable). Corpus faces also write `N`, `"`, and a
space. VERIFIED (decompile): the reader stores K − 0x20 as the font's
pitch and family, which Windows uses only to pick a substitute when the
face is not installed (as for the style's `[fnt]` kind bits, below).

More of the reader (VERIFIED (decompile)):

- The control stays in the text as a font change that lasts to the next
  `<:f…>` or `<:f>`; only one is in force at a time, so `<:f>` returns to
  the style's font (O5).
- SIZE is the character height in twips (the font's em, as a negative
  Windows height). FACE is at most 31 characters.
- R, G, B are read only when something follows the second comma; without
  them the colour is the style's. All three must read: a fourth field, a
  missing one, or a non-digit leaves the colour the style's. Each is taken
  modulo 256 (`<:f,BArial,256,0,767>` is blue). An empty number is 0.
- With no comma at all (`<:f240>`) nothing reads: the text takes the
  style's font, as after `<:f>`.
- K is the first character of a non-empty FACE field (`<:f,B,>` names no
  face). A face longer than 31 characters is cut there and ends the reading
  (no colour).
- A field with a character other than a digit where a number belongs ends
  the reading there: what was read before it still applies.

### Other controls

The remaining `<:…>` controls hold layout and formatting state, such as
`<:#480,9360>` (a cache) and `<:R…>` (a tab ruler) (see
`PHASE1_FORMAT_AUDIT.md`). They are reported and kept as legacy marks.

- `<:s>` sets a paragraph flag (bit 2 of the paragraph's flag byte, the
  byte `<:v+>`/`<:v->` and the cache also mark) and nothing else in the
  control parser (`11b8:030a`, VERIFIED (decompile)); what reads the flag
  was not traced. It is the commonest legacy mark in the user's corpus
  (839 of the compared paragraphs) and changes no font.

## Style records (`[tag]`)

| Field | Meaning | Evidence |
|---|---|---|
| value 1 | Style name | VERIFIED |
| value 2 | 2–9, 11, 12, or 0. Consistent with a function-key assignment: TOC1–TOC8 are 2–9, TOC9 is 11 (F10 skipped), Table Text is 12, and the rest are 0 | INFERRED (T03) |
| `[fnt]` value 1 | Typeface | VERIFIED |
| `[fnt]` value 2 | Size in twentieths of a point (240 = 12 pt) | VERIFIED (status bar) |
| `[fnt]` value 3 | The style's text colour, Windows RGB (`0x00BBGGRR`, a 32-bit value; 0 = black) | VERIFIED (decompile) |
| `[fnt]` value 4 | Flags: 1 bold, 2 italic, 4 underline, 8 word underline, 16 upper case, 32 small caps, 64 double underline, 128 strikethrough, 256 superscript, 512 subscript, 1024 lower case, 2048 initial caps, **4096 1st line bold** (Modify Style › Font; CORRECTED 2026-11-07: this note and the importer first read it as protected text in a style, which the decompile and VM fixtures FLB0 to FLB2 disproved, `FIRST_LINE_BOLD_NOTES.md`; protected text is only the run mark `<+*>`); `0x2000` is dropped when read | VERIFIED (decompile; VM FLB1; bold and italic also CORPUS) |
| `[fnt]` value 4, bits 14–15 | The face's kind: `0x4000` sans serif (Arial, Helv, Helvetica, Century Gothic, Franklin Gothic, AvantGarde, …), `0xC000` serif (Times New Roman, Tms Rmn, Palatino, Book Antiqua, Bodoni, Century Schoolbook, …), `0x8000` fixed pitch (Courier, Courier New, LetterGothic). Of 6,500 font records in both corpora, 2,348 are `0x4000` (22 families), 4,069 `0xC000` (25), 81 `0x8000` (3), and 2 have neither bit. A few disagree with the design (Gill Sans, Impact, and most News Gothic records are `0xC000`), so they are probably what Windows reported for the installed font. VERIFIED (decompile): they become the Windows pitch and family that picks a substitute **only when the face is not installed**: `0x4000` variable-pitch swiss (0x22), `0xC000` variable-pitch roman (0x12), `0x8000` fixed-pitch modern (0x31), neither fixed pitch of any family (0x01). Kept since Phase 3.3 as the document's face kinds, with `<:f…>`'s K − 0x20 (which also gives script 0x4_ and decorative 0x5_ families) | CORPUS (both corpora); VERIFIED (decompile) |

`[algn]`, `[spc]`, `[brk]`, `[line]`, `[spec]`, and `[nfmt]` follow inside each
`[tag]`. `[line]` is the style's paragraph rules (Modify Style › Lines),
ten values (`1058:0fac` for the order and sizes; VM fixtures NX1, NX2 with
the dialog for the meanings), all VERIFIED:

| Value | Meaning |
|---|---|
| 1 | Flags: 1 Line above, 2 Line below, 4 Length Text, 8 Length Margins (neither: Other); 16 also occurs (not traced, kept) |
| 2 | Length › Other in twips (2880 drew 2 in), kept when another length is chosen |
| 3, 5 | Bytes not traced (1 and 0 or 1), kept |
| 4 | Colour, a `COLORREF` (silver `12632256` is common) |
| 6, 7 | Line style of the rule above and below: 1–7 single lines of 10, 20, 40, 80, 120, 160, 240 twips; 8 double (20, gap 20, 20), 9 double heavy (40, 20, 40), 10 triple (20, 20, 40, 20, 20), 11 thick over thin (60, 20, 20), 12 thin over thick (20, 20, 60) (`1288:0000`, `1288:0800`) |
| 8, 9 | The height each rule takes, its thickness included: the dialog's Spacing is value 9 − thickness (300 with a 120-twip line shows 0.12 in) |
| 10 | The dialog's unit (1 inches; 2 seen) |

A rule above stands at the top of its space at the top of the paragraph's
first line, a rule below at the bottom of its last line, so the gap is next
to the text; the space is outside the line spacing (Double: 568 + 100 + 100)
and inside the paragraph's space above and below. Margins spans the column
whatever the indents; Text the first line's text (above) or the last
line's (below); Other its length from the text's start, ending at the
text's end on a right-aligned line, centred on a centred one. Ami Pro's
caches include the space (NX1OUT 484, 584, 684; NX2OUT 768 for two lines
plus 200). 385 style records in the user's corpus have a rule above or
below. The reader takes every value of a style record in a fixed order and
does not look at the sub-section names, except that `[nfmt]` and what follows
it are optional (a style without them gets number-format defaults, its own
name as the next style, no tabs, and a right indent of All with Both sides,
else 0); Text Tightness is read only from version 4 files. Readings from
Phase 2.5d and Phase 3, settled by the decompile where marked:

| Field | Meaning | Evidence |
|---|---|---|
| `[spc]` value 1 | Flags: the line spacing (1 Single, 2 1 1/2, 4 Double, 8 Custom) and the Add in choice (16 Always, 32 When not at break): S1 34, S2 36, S3 40, S5 17. VERIFIED (decompile): layout tests 1, then 2, then 4, and anything else is the custom height of value 2; only bit 32 matters for Add in, so a style with neither bit (326 style records in the corpora) adds its space as Always does | VERIFIED (S1–S5; decompile) |
| `[spc]` value 2 | The line height in twips. Custom: the value set (S3: 0.25 in, 360). 1 1/2 and Double: the height Ami Pro computed for its printer (12 pt Times New Roman: 432 and 561), and lays out (screen pitch 428 and 565 twips). Single: stale (`_DEFAULT.STY` stores 273; LH1 measures 284), the font decides. The extra height of a fixed line is split equally above and below the text (the first baseline moves by half). VERIFIED (decompile): layout reads this value only for Custom; 1 1/2 and Double are computed from each line's single height ("Line spacing" under "Layout rules" below), so the stored value is a dialog cache | VERIFIED (S1–S3, LH1; decompile) |
| `[spc]` values 3 and 6 | `1` in every fixture (README31 writes 4 for the first): bytes copied only to the Spacing dialog and the macro interface, never used by layout, so they are the dialog's display units (as `[algn]` value 2) | VERIFIED (decompile) |
| `[spc]` values 4 and 5 | Space above and below in twips (S4: 144, 288). Between two paragraphs both apply (about 0.30 in on screen). With When not at break, the space above is not added at the top of the page; with Always it is (S5: the first line 0.10 in lower) | VERIFIED (S4, S5) |
| `[spc]` value 7 | Text Tightness in percent: 100 Normal, 90 Tight (S6), 115 Loose. The user's guide says it "controls the width of a space"; measured, it adds (percent − 100)% of the font's average character width to every character, spaces included: 12 pt Times New Roman (average 96 twips, Windows' weighted lower-case average of 821/2,048 ems) is −9.6 twips a character Tight and +14.0 Loose at 200% (TT1), and TT2's five paragraphs break exactly where ours do. 180 style records in the user's corpus are not 100 (mostly 115 and 90) | VERIFIED (S6, TT1, TT2) |
| `[algn]` value 1 | Flags: 1 left, 2 right, 4 centre, 8 justified; 16 Both sides; 512 Hanging indent (I1: 1; I2: 536 = 512 + 16 + 8; I3: 513); 256 Hyphenation. VERIFIED (decompile): the reader clears 32, 64, and 128; layout takes right if 2 is set, else centre if 4, else justified if 8, else left; Both sides makes the right indent equal All whatever value the record ends with; Hanging indent adds a left tab stop where the following lines start (All + Rest), which is why I3 showed no change in text without tabs; Hyphenation (256) lets the hyphenator (`ltshyph1.dll`) split the last word of a line | VERIFIED (I1–I3; decompile) |
| `[algn]` values 2–5 | `1` (probably the unit, inches), then the indents in twips from the Modify Style dialog: All, First, Rest. First and Rest add to All: I1 (720, 1440, 360) starts the first line 1.5 in and the others 0.75 in from the margin, as the dialog's ruler shows. Both sides copies All into the right indent (I2). Hanging indent changes nothing on screen with First larger than Rest (I3). Every `[algn]` in both corpora has five values. With these indents and the right indent below, the cached line count matches for 817 of 820 fixture paragraphs (778 without) and 14,290 of 14,777 in the user's corpus (13,398 without) | VERIFIED (I1–I3 display); CORPUS (line counts) |
| Next style (`[brk]` 64) | Enter at the end of a paragraph gives the new paragraph the next style (the name after `[nfmt]`); Enter in the middle or at the start leaves both parts in the style, the empty paragraph Enter makes at the start included | VERIFIED (NX1, written back as `@NextHead@` paragraphs) |
| `[brk]` value | Bits: 1 page break before, 2 page break after, 4 allow page/column break within (Ami Pro's default), 8 keep with previous, 16 keep with next, 32 the style has its own tabs, 64 Next style on, 128 column break before, 256 column break after (B1–B8: 5, 6, 132, 260, 0, 16, 8, 68). Keep With is offered only when breaks within are not allowed. Corpus: 4 in most styles, 65 (Ctitle: Next style and page break before), 68 (Descript), 16 (Title), 48 (a-head), 36 (lists). With these rules laid out, AP31NAV's 39 page ends all match Ami Pro. VERIFIED (decompile) for 32: with it set, the tab list after `[nfmt]` (below) replaces the page or frame layout's tabs for the style's paragraphs (CORPUS: all 48 style records with 32 carry a tab list; 67 without it carry one that layout ignores); how the other bits act is under "Layout rules" below | VERIFIED (B1–B8; decompile) |
| Values after `[nfmt]` | The next style's name, then two numbers. AP31NAV's Body Text ends `Body Text 0 2088`, and Ami Pro's ruler shows a right indent of about 1.45 in in that style: the last value is the right indent in twips (I1: From right 1.50 in writes 2160; I2: Both sides writes All, 720). Most style records in the user's corpus have them (`[nfmt]`'s six values then these three); 1,062 records end after `[nfmt]`'s own six. VERIFIED (decompile): the first number is the count of the style's own tabs (at most 22 are kept), followed by that many type/position pairs, and the right indent comes after them | VERIFIED (AP31NAV ruler; decompile) |

Ami Pro's single line spacing in the VM, which prints to the Microsoft XPS
Document Writer (`[prn]`), is the font's ascent plus descent plus line gap,
rounded by the printer's hinting: Times New Roman 8, 12, 24, 48 pt give 189,
284, 560, 1,104 twips; Arial 8, 10, 12, 48 pt give 185, 228, 277, 1,104;
Courier New 10 and 12 give 211 and 257. Multi-line paragraphs are exact
multiples (LH1, VERIFIED).

### Writing style records (`11c8:26b7`, 2026-10-13)

Ami Pro's writer emits a `[tag]` record in the reader's order, every
section always present, ending with the next style's name, the tab count,
the type and position of each tab (one value per line), and the right
indent. Text is Windows-1252 with control characters (a tab aside) as
`<*X>`, bytes 0x80–0xBF as `</X>`, bytes from 0xC0 as `<\X>`, and `<`
doubled (`11c8:0bf5`); `[` is not escaped, so a bullet's text can start
with one (`[<*:>]`, 11 records). VERIFIED (decompile) and by writing every
style record of both corpora back from the model: the ISO corpus's 542
records come back byte for byte; the user's corpus's 1,396 of 1,912 do,
and the other 516 are older records that end after `[nfmt]`'s six values
(the 3.1 writer always adds the next style, tabs, and right indent).
Values the model does not use are kept (`ParagraphStyle::kept`): the
function key, the face-kind bits, the dialogs' units, `[spc]`'s Add in
bits and Single height, `[nfmt]`'s six values, a next style's name that
names no style (an outline style the sheet lacks, or none), and `[algn]`
bits 32, 64, and 128, which the reader clears but 443 older records carry.

### Style names

A style's name is escaped where Ami Pro writes it: in its `[tag]` record
as record text (`11c8:0bf5`: `<` doubled, other bytes as `<*X>`, `</X>`,
`<\X>`) and in a paragraph's tag as body text (`11b0:129e`, which also
writes `>` as `<;>` and `[` as `<[>`). Ami Pro decodes both: a sheet with
a style written `Caf<\i> <<1> [x]` in its record and `@Caf<\i> <<1<;>
<[>x]@` in a header paragraph lists the style as `Café <1> [x]` in Style
Management and shows the header paragraph in it (VERIFIED, AP4, written by
this project's writer, 2026-10-13). Neither corpus has a name with `<`,
`>`, or `[` (one garbled tag aside); the importer decodes names since
2026-10-13.

### Writing style sheets (`11c8:14e0`, 2026-10-13)

Style › Save as a Style Sheet (Alt+S, V) asks for a file name (8.3, no
path: the sheet goes into the style-sheet path, Tools › User Setup ›
Paths), a description, With contents (on by default), and Run macro. The
writer (`11c8:14e0`; the document writer is `11c8:1c7b`) emits, in order:

| Section | Values | Evidence |
|---|---|---|
| `[ver]` | 4 | SS1N |
| `[files]` | linked picture files; none without contents | `11c8:07dd` |
| `[charset]` | 82, `ANSI (Windows, IBM CP 1252)` | SS1N |
| `[revisions]` | 0, 1, or 2 (and a string when revision marking is on) | `11c8:14e0` |
| `[recfile]`, `[toc]`, `[docvars]`, `[book]` | only when the document has them | `11c8:14e0` |
| `[prn]`, `[port]` | the current printer and port | SS1N (the VM's XPS writer) |
| `[lang]` | the language | SS1N |
| `[fldnames]` | eight field names, `Field1` to `Field8` by default | SS1N, `11c8:342b`; absent from older sheets |
| `[desc]` | 21 values: the description, Doc Info Fields 1–4, nine numbers, the keywords, the annotation lock, Fields 5–8, a number (`FIELDS_NOTES.md`, "Doc Info"; corrected 2026-10-20, values 2–5 are not keywords) | SS1N, SS2N, SS2W |
| `[fopts]` | four numbers and two style names (empty when unset); older files have the numbers only | SS1N |
| `[newmac]` | the macro to run, when set | `11c8:14e0` |
| `[lnopts]`, `[docopts]`, `[GramStyle]` | as in a SAM | SS1N |
| `[tag]`… | every style, in function-key order, styles without a key last (all 640 corpus files) | SS1N, corpus |
| `[lay]`…`[elay]` | the layout named `Standard` (the document's own) only | `11c8:236e` |
| `[frm]`… | page frames, with contents only | `11c8:14e0` |
| `[edoc]` | with contents: the text, `>`, and the `[Embedded]` directory; without: nothing after it | SS1N, SS2N, SS2W |

`[desc]`'s nine numbers start with the save time in seconds since 1970
(local), 1, the previous save's time (on a new document, its creation),
and the minutes of editing since. Save as a Style Sheet in a new document
on `_DEFAULT.STY` writes `_DEFAULT.STY` back byte for byte apart from the
printer, the description, and these numbers (SS1N against the shipped
file).

**With contents off**, Ami Pro leaves out the text of every layout frame
whose flags have 0x8, and its picture (clearing 0x8000 and 0x6000_0000);
frames without 0x8 keep their text and pictures (`11c8:2a0f`).

**Marking and contents (decompile, 2026-10-13, Phase 4.11).** Before it
writes the standard layout, `11c8:14e0` sets 0x80 on the layout number and
0x8 on every layout frame, so a written sheet always has both; afterwards
it clears them again in memory on frames without text. `11c8:2a0f`'s
fourth argument decides whether a 0x8 frame keeps its text and picture:
the sheet writer passes With contents for layout frames (`11c8:236e`'s
third argument) and 1 for page frames, the document writer 0. Tables are
written whatever the marks (`1490:0000`). With contents, page frames
follow the layout, each with 0x8 set while it is written, then `[edoc]`,
the main text (`11b0:2623`), and the end through `11c8:3d76(1,1,0,…)`,
which stores the pictures of every page frame and layout frame (a SAM
leaves out those of 0x8 layout frames). `[files]` lists linked pictures
only with contents. VERIFIED: this project's sheets from SS2.SAM equal
Ami Pro's SS2W (with) and SS2N (without) but for `[desc]`'s save times,
and from F01.SAM equal Ami Pro's F01W but for `[desc]` and the caches Ami
Pro measured (F01.SAM had none); a new document made from a sheet written
with contents (WC1, from the app's page fixture) starts in Ami Pro with
its header, text, and frame. Ami Pro writes a description left at its
default "No Description" as "NAME.sty - no description". Every
corpus sheet's header and footer frames have 0x8, and almost no SAM's do
(10 of about 700); the layout number's 0x80 goes with it (641 in sheets,
513 in SAMs). Both look like "from the style sheet" marks: after Save as
a Style Sheet in an untitled document, the SAM saved next had neither, and
a sheet written from it would keep its header text (SS2N, SS2W, SS2).

**The document afterwards.** A saved document is linked to the sheet just
written: `[sty]` names it and none of its styles are stored in it any
more (SD1, SD2: each save as a sheet moves the link). An untitled document
is detached instead: `[sty]` empty, every style a document style (SS2;
Style Management lists them all under Styles in document).

**Body text** (frame text and contents) is written by the paragraph
writer `11b0:1405`: the tag (none for Body Text); the paragraph controls
written straight to the file in the order alignment, `<:v±>`, `<:c>`,
`<:s>`, `<:I…>`, `<:S…>`, `<:R…>` (or `<:R>` after a paragraph with a
ruler), `<:#…>`; then the text in physical lines. Plain characters (0x20
to 0x7F, with `<<`, `<;>`, `<[>`) are copied in stretches; a line ends
when it passes 256 bytes, or passes 65 bytes and ends in a space at the
end of a stretch or after any other piece (an escape, a tab, a control).
A second space after `. ` stays on the line. An empty paragraph is its
bare tag line, or an empty line; one with controls only is that line and
an empty line. The corpus agrees: of its wrapped body lines, 2,355 end in
a space between 66 and 256 bytes and the others are 257 bytes or more.

A frame's `[txt]` may hold no paragraph at all (1,289 of 2,989 frame texts
in both corpora, frames never typed in) or one empty line (105); the
writer keeps which.

AP5 (2026-10-13) is a sheet written by this project with a style edited
as Modify Style edits it: Ami Pro's Modify Style shows each value as set
(style tabs with a right tab at 3 in, All 0.50 and First 0.25 in,
Justify, Custom 0.25 in, Below 0.08 in, When not at break, Next style
Body Text, Number "1." with Text "Hi" and a bullet in bold, Line below in
style 3 with 0.10 in spacing, Arial 14 bold, Hyphenation). Switching from
a style whose face the guest lacks (Noto Serif) makes Ami Pro ask whether
to save that style's change: it substituted the face.

Checked in the VM (2026-10-13): Ami Pro 3.1 opens sheets written by this
project with File › New (List by description shows the written
description) and Use Another Style Sheet: AP2 (from GOODIES.SAM: its page
layout and its header text, a tagged, centred line with a font change),
AP3 (a new document of this project: three styles), AP4 (escaped names).

### Writing documents (`11c8:1c7b`, 2026-10-13)

File › Save and Save As go through `11b0:1cab`, which calls the document
writer `11c8:1c7b` for the sections up to `[edoc]`, then the main text's
paragraphs (`11b0:2623`, each by the paragraph writer `11b0:1405`), then
the end of the body and the embedded objects (`11c8:3d76`). All VERIFIED
(decompile); section names through the pointer table at DGROUP 0xbe6
(`DAT_1680_0be6` `[sty]` … `DAT_1680_0c94` `[fldnames]`; the format strings
`%s\r\n`, `\t%s\r\n`, `\t%u\r\n`, `\t%lu\r\n` follow it). Save As's "Keep
format with document" (save flag 2) empties `[sty]`, and makes the writer
store every style and every page layout. The corpora agree on the order: of
576 readable SAMs (107 distinct section sequences), 567 follow the table;
the other 9, the files with `[panosefonttable]`, also write `[recfile]`
before `[revisions]`, so a later writer made them. Document styles are in
function-key order in 447 of 447 files with `[tag]`s.

"Model source" says where a writer gets each item: **typed** (the model's
own data), **kept** (legacy data the importer keeps as read), **derived**
(computed when writing), **constant**, or **cache**.

| Section or part | Values (routine) | Written when | Model source |
|---|---|---|---|
| `[encrypt]` | a password-protected file's header (`1c7b`, save flag 0x40) | the user sets a password | never: protected files are refused on reading |
| `[ver]` | `4` | always | constant |
| `[sty]` | the style sheet's file name as Ami Pro lists it; empty with Keep format with document | always | typed (`StyleSheetLink`) |
| `[files]` | one path per linked picture (`11c8:07dd`, frames with flag 0x2000_0000 whose picture is linked) | always, often empty (28 of 577 files list paths; P03 `L:\IN\PIC.BMP`) | typed (`PictureSource::Linked`), in frame order |
| `[charset]` | `82`, `ANSI (Windows, IBM CP 1252)` (resource strings 0x57e, 0x57f; `11c8:1486`) | always (3.1; absent from 50 older files) | constant |
| `[revisions]` | revision marking: 0, 1, or 2 with an options string (`1c7b`) | always (3.1) | kept; 0 for a new document |
| `[recfile]` | a merge data file: two strings, six numbers | when the document has one (33 files) | kept |
| `[toc]` | table of contents options (`11c8:1bbc` → `1588:261d`) | when set (55) | kept |
| `[master]`, `[sequence]` | a master document's files (`1660:0000`, `1660:0748`) | when set (105, 59) | kept |
| `[docvars]` | document variables (`1630:1dd7`) | when set (3) | kept |
| `[book]` | bookmarks (`1580:1368`) | when set (34) | kept |
| `[prn]`, `[port]` | the printer and its port | always (`[port]` since 3.1) | kept; a new document has the default sheet's |
| `[lang]` | the language number | always | kept; 1 |
| `[fldnames]` | the eight Doc Info field names (`11c8:342b`) | always (3.1) | the document's Doc Info (since 2026-10-20); `Field1`…`Field8` |
| `[desc]` | 21 values (`11c8:342b`, read by `1058:2732`; `FIELDS_NOTES.md`, "Doc Info"): the description, Doc Info Fields 1–4 (not keywords: corrected 2026-10-20); then the save time and the creation time (seconds, Ami Pro's time base: the wall clock read as US Pacific time), the revisions, the minutes of editing, five statistics; the keywords, the annotation lock, Fields 5–8; the flags | always | the document's Doc Info (since 2026-10-20), written with this save's time in Ami Pro's time base, the revisions one more than read (the reader adds one), the session's minutes of editing added; a new document: created and saved now, 1 revision, 0 minutes |
| `[fopts]` | four numbers, then two style names (empty when unset) | always | kept; `0 1 0 0` |
| `[auto]` | two strings and a number | when set (no corpus file) | kept |
| `[lnopts]` | line numbering: a number, a style name, a number | always | kept; `2`, `Body Text`, `1` |
| `[docopts]` | the hyphenation zone and a number | always | typed (`hyphenation_zone`) and kept |
| `[GramStyle]` | the grammar checker's style | always (3.1) | kept; empty |
| `[ParaNum]` | `1` | when outline numbering is on (`DAT_1680_21b7` 0x400; 263 files) | kept |
| `[panosefonttable]` | not written by 3.1 | — | kept and written back in place (9 user files, between `[charset]` and `[recfile]`) |
| `[tag]` … | a style record (`11c8:26b7`) for each style stored in the document (style flag 1 clear), in function-key order; every style with Keep format with document | as listed | typed (`ParagraphStyle`, origin Document; every style but missing names when detached) |
| `[frm]` … | each page frame (`11c8:2a0f`), back to front, in Ami Pro's three lists (`11c8:3ba0`, `1248:0347`): frames in table cells (`DAT_1680_234c`), Where placed frames (`2348`), anchored frames (`234a`); `CELL_FRAME_NOTES.md` | when there are page frames | typed (`frame_stack`, placement, style, content) and kept (fields the model does not use) |
| `[lay]` … `[elay]` | each page layout (`11c8:236e`), **each ended by its own `[elay]`** (with one `[elay]` after several layouts, Ami Pro reads only the first: VM, G21 written so, 2026-10-13), whose style-sheet bit (layout flag 0x80) is clear, or every layout with Keep format with document: a layout taken from the sheet and left unchanged is not written (61 linked files have none; 7 store only inserted layouts and take Standard from the sheet). **Inserted layouts are renamed `1`, `2`, … in order as they are written** (`11c8:3ba0`: every name but the standard one's); 658 corpus layouts are Standard, the rest `1` to `9` | as listed | typed (`PageLayout`, frames) and kept |
| `[l1]` | the name of the layout the document starts in, `0` for Standard (`11c8:3ab2`; read by `1058:3156` with `1058:3194`, which finds it by name) | always | typed: the first page's layout (27 files start in an inserted layout, 15 of them with no page break at all; read since Phase 4.5, `fd8a64d`: `Document::start_layout`) |
| `[pg]` | the page table (`11c8:381e`) | unless an option suppresses it | **cache: omitted** (below) |
| `[edoc]` | then the main text, one paragraph at a time (`11b0:2623` → `11b0:1405`) | always | typed (main flow) |
| `>` | the body's end (`11c8:3d76`) | always | constant |
| embedded objects | for each picture frame (flag 0x2000_0000), in the frame order of `[files]`: its picture's bytes and then its rendering's, copied verbatim (`11c8:4155`) | when there are pictures | typed (`Picture`, object store) |
| `[Embedded]` | `\r\n[Embedded]\r\n` (written even when there are no pictures: every Ami Pro-written fixture ends so), then per picture `%u %s %ld %ld %ld %ld ` (the frame's sequence number, the extension or `.eps`, the picture's offset and length, the rendering's offset and length), the linked path if any, `\r\n` (`11c8:4521`); then the offset of the `[Embedded]` line as `%08ld\r\n` | when there are pictures (P02: `3 .bmp 5329 9270 14599 64646 ` and `00079247`) | derived (offsets and lengths of the bytes written) |

**Body.** Each paragraph is its tag, its paragraph controls, its text,
and an empty line (`11b0:1405`; "Writing style sheets" above has the line
rules). Item by item:

| Item | Written as | Model source |
|---|---|---|
| Style tag | `@Name@`, none for Body Text; `@@` for a leading `@` | typed |
| Alignment | `<+@>`, `<+A>`, `<+B>`, `<+C>` first | typed (`ParagraphFormat`) |
| `<:v±>`, `<:s>` | in that order, after alignment | kept (legacy marks at the start) |
| Indents, spacing | `<:I…>`, `<:S…>` | typed |
| Tab ruler | `<:R…>`, or `<:R>` after a paragraph with a ruler | typed, and kept (`<:R>`) |
| Paragraph cache | `<:#H,W>`: the paragraph's height and the column width it was laid out in (`11b0:0476`), written only when the paragraph's height is valid (paragraph flag 0x40 of byte 5; Ami Pro clears it when the paragraph must be laid out again, so it writes some paragraphs without one) | **cache** (below) |
| Characters | plain, `<<`, `<;>`, `<[>`, `<*X>`, `</X>`, `<\X>`, tabs; `<**>` hard return, `<*7>` discretionary hyphen | typed (text) |
| Attributes | `<+X>`/`<-X>` for bold `!`, italic `"`, underline `#`, word underline `$`, double underline `)`, caps `+ , - (`, strikethrough `%`, superscript `&`, subscript `'`, protected `*`, no hyphenation `.` | typed (runs) |
| Font, overstrike | `<:fSIZE,KFACE,R,G,B>`, `<:O±C>` | typed (runs) |
| Anchors | `<:A n>`, `<:t n>`, n the frame's place among anchored frames | derived (frame order) |
| Breaks | `<:p` flags [layout name]`>` page break, `<:c>` column break | typed (break objects; layout names as written) |
| Page number | `<:P…>` | typed |
| Floating header or footer | `<:H…>`/`<:h…>` with its own paragraphs and `>` | typed (flows) |
| Notes and other controls | `<:N…>` (notes; they span lines), `<:X…>` (power fields: kept marks read by `PowerMark`, but Seq an object; the fields run), `<:Z…>`, `<:T…>`, `<:M…>`, `<:e…>`, `<:F…>`, `<:V…>`, `<:x…>`, `<:>`, `<:H…>` text forms | kept (legacy marks, written verbatim); `<:D…>`, `<:b…>`, `<:r…>`, `<:k…>`, `<:d…>` are objects since the fields run (`FIELDS_NOTES.md`) |

**Nested sections.** Inside `[tag]`: `[fnt]`, `[algn]`, `[spc]`, `[brk]`,
`[line]`, `[spec]`, `[nfmt]` (`writer.rs`). Inside `[lay]`: `[rght]`,
`[lft]`, `[hrght]`, `[hlft]`, `[frght]`, `[flft]`, `[repfrm]`, each frame a
`[lyfrm]` (`sheet.rs`). Inside a frame: `[frmname]`, `[frmmac]`,
`[frmlay]`, then its content, `[txt]`, `[tbl]` (`[h]`, `[w]`, `[e]`,
`[data]`, `[tble]`), or `[isd]`; and, kept, `[chint]`, `[ehint]` (4
frames), and `[mf]` (2). Counts over both corpora: 4,045 style records,
3,741 frame layouts, 2,226 frame texts, 918 pictures, 595 tables.

**The caches (VM fixtures K1–K6).** Ami Pro opens every file with `[pg]`
and all `<:#…>` removed without complaint, and lays it out from scratch
with the current printer:

- Where the original's caches came from the VM's printer (`README31.SAM`,
  G21, P02), the file without caches is laid out as the original: saved
  again, its paragraph caches equal the original's (README31: 334 of 336;
  the other two are stale in the original, and Ami Pro rewrote one of
  them the same way when it saved the original again), with the same page
  count and page ends.
- Where they came from another printer (`MERCURY.SAM` and the two user
  documents), the file without caches is laid out with the VM's printer's
  metrics (lines of 279 and 372 twips for the original's 284 and 375), so
  page ends move: 31 pages for the original's 30, and the ends of pages 6
  to 12 in the other.
- **Ami Pro trusts the caches it is given.** Opening a file, it lays out
  only the pages it shows and keeps the cached heights, and the stored
  `[pg]` entries, for the rest (`…R`: page 1 changed, the others written
  back as read). With every height doubled and the widths kept (K1W),
  Ami Pro paginated README31 into 24 pages instead of 14; with the widths
  changed to one the column does not have (K1X), it ignored the caches
  and laid the file out exactly as without them. A cache counts only
  for a paragraph laid out in a column of its width.
- **`[pg]` is derived from the paragraph caches.** Without `[pg]` but with
  the caches (`…Q`), Ami Pro writes the page table it wrote for the
  original (K5: identical; K1: identical but for one page whose stored
  entry was stale).

So the writer **omits `[pg]`**, and **writes a paragraph's `<:#…>` only
while the paragraph, its style, and its place are as they were read**,
giving back the original's cache. Ami Pro then opens a written document
as it opens the original, and measures what was changed, as it does for
any paragraph whose height it has invalidated. The writer never computes
a cache: Ami Pro would trust a wrong one.

**Four things Ami Pro relies on (VM, 2026-10-13).** Written files Ami
Pro would not open, faulted on, or showed differently taught four rules
the round trip could not see:

- **A table cell's eighth value says whether text follows.** The cell
  writer (`1490:07c0`) writes nine values, a colour, and two more; the
  eighth is 1 when the cell has a text block or a formula, and only then
  does it write the cell's text (`1490:096b`; the formula follows as a
  second body). A cell never typed in has neither, so Ami Pro writes its
  record alone (README31's empty cells). Ami Pro refuses a file with text
  after a record whose eighth value is 0 ("This file cannot be opened";
  VERIFIED). Corpus: 21,588 records, value and text agree in every one
  (CORPUS). The writer writes text when the cell holds anything but one
  empty Table Text paragraph, and then sets the value to 1; the reader
  warns when the two disagree, and the round-trip audit counts such
  warnings.
- **A table frame's `[frmlay]` value 1 is not its bottom.** The reader
  stores value 1 as the frame layout's first word (`1058:1843`, which the
  frame reader calls when it meets `[frmlay]`), and `1148:0e1c` takes the frame's
  extent from it, measured from the frame's top. In text frames and
  pictures it is always the frame's bottom (2,269 of 2,269 frames in both
  corpora and the VM fixtures), but in 353 of 406 tables it differs, with
  no relation to the rectangle or the table's size: it looks left from
  when the table was created or last placed (where Ami Pro updates it was
  not found). Writing MERCURY.SAM's table with its bottom (7,707 for the
  original's 11,171) cost page 1 its logo picture ("Cannot create
  temporary file for frame. Frame contents will not be displayed.");
  VERIFIED with the original changed in that value alone, after a clean
  restart, while the original and other variants opened in the same
  session showed the logo. The importer
  keeps the value as its offset from the stored bottom
  (`frmlay.1`), and the writer adds it back, so an untouched table writes
  the value it was read with and a moved one keeps its room.
- **A note's closing `>` is a line of its own.** The writer of notes and
  floating headers and footers (`11b0:0660`, cases `N`, `H`, `h`) writes
  the opening, the control's own paragraphs, then `>` and ends the line;
  the paragraph's text goes on on the next line (MERCURY.SAM: `>`, then
  `. `). Ami Pro reads the control's paragraphs until a lone `>` line, so
  a written `>. ` made it read the rest of the document into the note and
  fault (GPF in `AMIPRO.EXE` after page 1; VERIFIED by bisecting: each
  other difference alone opened). This reader ends the control at the `>`
  wherever it is, so it now warns when a multi-line control's `>` has text
  after it (no original in either corpus or the VM fixtures does).
- **A frame's lines need the lines bit.** A frame made in the app with a
  border showed none in Ami Pro: the decompiled reader takes field 7's
  sides only with flag `0x10000` (table above). This reader took field 7
  alone, so 193 corpus frames and 3 VM fixtures with sides in field 7 but
  without the bit had a border here that Ami Pro does not draw. Now the
  reader applies the bit (and Ami Pro's 0-or-above-30 rule), keeps field
  7 as read where it would differ, and the writer sets the bit exactly
  when the frame has sides. VERIFIED: APP2 (the app's page fixture saved
  as SAM) shows its frame's border in Ami Pro, and Ami Pro saves its frame
  records back byte for byte.

**A picture frame in a table cell (VM, 2026-10-13).** README31's three
SmartIcon pictures are Where placed page frames (field 1 is page 12) with
flag `0x20` set (`0x20000060`), and Ami Pro draws them in the cells of the
table on that page. With a page break inserted before it (EDK1, Phase
4.11), Ami Pro moved them to page 13 with the table and put them back in
their cells, ignoring a move and resize made here; it saved them with
page 13 and the old rectangles. `0x20` is on 108 picture frames in 23
files of both corpora and on no other kind of frame. Traced on 2026-10-18
(`CELL_FRAME_NOTES.md`): Ami Pro keeps these frames in a list of their
own, written first; a cell record's values 11 and 12 say the cell holds a
frame and which (its place in that list); the frame covers the cell's
text area, its page and corner taken from the cell at layout. (`1238:0ca4`
tests a line's bit, not a frame's.)

### Bullets and numbers (`[spec]`, 2026-10-12)

A style's Modify Style › Bullets & numbers is `[spec]`, ten values (the
reader `1058:0fac`; widths `1200:1384`, `1200:159c`; drawing `1200:0af0`;
numbers `1200:1489`, `1308:0caa`; placement `1200:12de` from the line
builder `10d0:1b5f`; VM fixtures BN1, BN2, BN3 with the dialog):

| Value | Meaning | Evidence |
|---|---|---|
| 1 | 0 in every record seen; kept | CORPUS |
| 2 | The outline level, 0 for none | VERIFIED (BN2; decompile) |
| 3 | The text before each paragraph, escaped as body text; `<*X>` is code X − 0x20 (below). Empty (an indented blank line) for none | VERIFIED (BN1; decompile) |
| 4 | 360, 1008, 28948, 20, … Not the dialog's "Space for"; not traced; kept | VM (BN1) |
| 5, 6 | 1 and 1; kept | CORPUS |
| 7 | The text's attributes as `[fnt]` flags: 1 bold, 2 italic, 4 underline, 8 word underline, 16 upper case, 256 superscript | VERIFIED (BN1 dialog; decompile) |
| 8 | Flags: 16 Right align; 2 is set in outline styles (not traced) | VM (BN1) |
| 9, 10 | 0; kept | CORPUS |

The codes in the text: 0x10 a small dot, 0x11 a dot, 0x12 a small square,
0x13 a square, 0x14 an open square, 0x15 and 0x16 a small and a large
diamond, 0x17 and 0x18 a small and a large ring, 0x19 a check mark, 0x08 and
0x0A–0x0F Ami Pro's bitmaps (arrowheads, hands); 0x1A the number in arabic
numerals, 0x1B and 0x1C in upper and lower roman, 0x1D and 0x1E as upper
and lower letters (a to z, then aa, ab, … zz, then three letters), 0x1F as
that many asterisks, 0x07 as that many daggers. Any other character is
itself, in the paragraph's font with value 7's attributes. **0x1E is written
`<*>>`**: its code character is the closing bracket, so a reader that ends
an escape at the first `>` misreads it (18 of the user's files and 10 of
their style sheets have one). The drawn bullets are sized from the font's
ascent: a quarter (small dot), a third (small square), a half (dot, square,
small diamond and ring), three quarters (large diamond and ring), the whole
ascent (open square, bitmaps), and the average character width (check).

**Numbers** (BN2): a style without a level counts its unbroken run of
paragraphs, so any other paragraph, an empty one or one in another numbered
style included, starts it again at 1. A style with a level counts on across
paragraphs of other styles until a paragraph of a smaller level (a higher
one) restarts it; each levelled paragraph restarts the deeper levels.

**Placement** (`1200:12de`; BN1, BN3): the bullet or number stands at the
left indent (All), and the first line's text at the first-line indent
(All + First), or right after the bullet when it is wider than First. Of
the 4,264 compared bulleted paragraphs in the user's corpus only 23 have a
bullet wider than their first-line indent, so bullets barely change line
counts; 1,041 style records in 293 of the user's files have bullets.

The `<:s>` paragraph flag (839 compared paragraphs) changes neither the
bullet nor the numbering (BN3). Not modelled yet: Right align (value 8 bit
16; BN1 showed no difference with one-digit numbers), superscript bullet
text, and the bitmaps' pictures (drawn as an arrowhead).

**The dialog's parts** (VM fixture BN4, 2026-10-13): Number is an
editable box over the seven number codes (it offers 1, I, i, A, a, *, †;
"1." is the code and a typed full stop), Text is free text, and Bullet a
list of the bullet codes and pictures; all three combine. Ami Pro writes
them in the order number, Text, bullet: Number 1, Text "Hi", and the
first bullet are `<*:>Hi<*0>`, and the sample shows `1Hi•`.

**Empty fields.** Ami Pro writes an empty value as an indented blank line
(`[spec]`'s text, `[nfmt]`'s currency symbol). Until 3.2e the parser dropped
such lines, which shifted every later value of the style: 155 `[nfmt]`
records in the corpora lost their next style, tabs, and right indent that
way. A line holding only tabs and spaces is now a value; only a line with
nothing on it is not.

### Styles and style sheets (VM fixture ST1, 2026-10-12)

A SAM's own `[tag]` records are exactly the styles stored in the document:
those created there or changed there (Style Management's "Styles in
document", marked with a bullet in the Styles box). Every other style comes
from the style sheet `[sty]` names, which Ami Pro looks for in its style
directory. A SAM writes `[lay]` only when its page layout was changed there;
without it the page layout is the sheet's (user's guide, chapter 9).

ST1 is a document on `_DEFAULT.STY` with a created style (Mine), a changed
sheet style (Subhead), sheet styles (Title, Bullet), and a name defined
nowhere (Ghost). VERIFIED (ST1, ST1A, ST1B):

- Style Management lists Mine and Subhead under Styles in document (the
  function keys Ami Pro gives a `[tag]` written with 0 are the next free
  ones) and the sheet's under Styles in style sheet.
- **Remove** of Mine: its paragraph keeps `@Mine@` and formats as Body
  Text; the SAM saved after (ST1A) has no `[tag]` for it. Remove of Subhead
  (a changed sheet style) takes the sheet's Subhead again, and its `[tag]`
  goes too. A name defined nowhere shows red in the status bar.
- **Use Another Style Sheet** (`_basic.sty`, which has Body Text, Body
  Single, Header, and Footer): Title and Bullet, sheet styles the new sheet
  lacks, format as Body Text with their names kept and shown red; Subhead
  and Mine, stored in the document, keep their formats and `[tag]`s; `[sty]`
  becomes the new sheet's name as the dialog lists it (`_basic.sty`); every
  paragraph keeps its tag (ST1B). The guide adds that the page layout
  changes only when it was the sheet's.

## Page layouts (`[lay]`)

Readings from G00–G23 and TB1: a new `_DEFAULT.STY` document with the two
F01 main paragraphs and one change made in Page › Modify Page Layout, Page ›
Header/Footer, Page › Breaks, or Page › Insert Page Layout. G00 is the
unchanged control.

**Structure.** Layouts follow the top-level frames. Each layout is one
`[lay]` section with a name and a number, then its parts, in this order:

```
[lay]
    Standard            name
    513                 layout number (below)
    [rght]              right-page layout (all pages when the layout is shared)
    [lft]               left-page layout, only when left pages differ
    [hlft] [lyfrm]...   left header frame, only when left pages differ
    [hrght] [lyfrm]...  right header frame
    [flft] [lyfrm]...   left footer frame
    [frght] [lyfrm]...  right footer frame
    [repfrm] [lyfrm]... repeating frames (F08, F09)
[lay] ...               further layouts, from Insert Page Layout
[elay]
[l1]
[pg]                    page table (below)
```

The order is from G15–G17 (left-page parts) and F08 (`[repfrm]` after the
footer); a document with both is not yet a fixture.

**Layout number** (VERIFIED, one bit per fixture):

| Bits | Meaning | Evidence |
|---|---|---|
| low byte | Paper size: 1 Letter, 2 Legal, 4 A4, 7 Custom | G11, G13, G14 (3, 5, 6 presumably A3, A5, B5: INFERRED) |
| `0x100` | Landscape; the stored height and width are swapped | G12 |
| `0x200` | Same layout on all pages; cleared when left pages differ | G15, G16 |
| `0x400` | Mirror | G16 |
| `0x800` | Header begins on the second page | G19 |
| `0x1000` | Footer begins on the second page | G23 |

**`[rght]` / `[lft]` values.** The shape is the `[frmlay]` shape (see
Frames), read here for a whole page:

| # | G00 | Meaning | Evidence |
|---|---|---|---|
| 1, 2 | `15840 12240` | Page height and width (Letter 11 × 8.5 in; A4 16833 × 11908; Legal 20160 high; Custom 6 × 9 in as 12960 × 8640) | G11–G14 |
| 3 | `1` | The unit Modify Page Layout last showed the page size in: 1 inches, 2 cm, 3 picas, 4 points | VERIFIED (decompile, `PAGE_LAYOUT_NOTES.md`) |
| 4 | `1440` | Left margin | G03 |
| 5 | `1440` | Bottom margin | G02 |
| 6 | `1` | The unit the margins were last shown in, as value 3 | VERIFIED (decompile) |
| 7 | `1440` | Top of the text area (absolute; equals the top margin on a page) | G01 |
| 8 | `1440` | Right margin | G04 |
| 9 | `0` | Flags: 1 column balance, 2 line between columns, 4 lines around the page | G06, G09, G07 |
| 10 | `1` | Line between columns: style index (frames default to 2) | C04, G09 |
| 11 | `0` | Colour of the page lines (Windows RGB) | G10 |
| 12 | `1` | Line around the page: style index | INFERRED (default list item) |
| 13 | `0` | Sides with a line around the page, as frame field 7 (1 = All) | G07 |
| 14 | `2` | Line position, as frame field 9; the page default is Inside (2) | G08 |
| 15 | `1` | Number of columns | G05 |
| then | | Per column: left, right, number of tabs, then that many type/position pairs | G05 |

**One tab list per box** (VERIFIED (decompile), `PAGE_LAYOUT_NOTES.md`):
the writer writes the box's list after the first column and 0 after the
others, and the reader keeps the first list that is not empty for every
column. **Columns carry their own tabs** was the earlier reading. Two page
columns were written
`2 1440 6000 12 1 720 … 1 8640 6240 10800 0`: the first column kept the 12
default left tabs, the second has none (G05). The same per-column shape
explains the `0` between frame columns (C01, C02). Gutter = next left − this
right (240 twips by default). Columns did not narrow for a line between page
columns (G09), unlike frame columns.

**Left and right pages.** Choosing Pages › Left or Right and changing a
setting writes `[lft]`, `[hlft]`, `[flft]` alongside the right-page parts and
clears `0x200` (G15). With Mirror, the left page is written with its left and
right margins exchanged and its columns recomputed (G17: right page left 2160 /
right 1440, left page left 1440 / right 2160). Right pages are odd pages.

**Headers and footers are layout frames.** `[hrght]`/`[frght]` (and
`[hlft]`/`[flft]`) each hold one `[lyfrm]` frame record, its `[frmlay]`, and
`[txt]`:

- The header frame is the top-margin band (0,0 to page width × top margin),
  and the footer the bottom-margin band; both follow margin changes (G01,
  G02).
- Their own margins come from the Header and Footer panels: header top 0.55
  in (792 as the absolute text top) and bottom 0.05 in (72); footer the
  reverse.
- Flag words: header `0x2BC0`, footer `0x33C0`. Compared with the frame bits,
  `0x800` marks a header and `0x1000` a footer (INFERRED); `0x100` and
  `0x2000` are the repeating bits.
- Header text is ordinary paragraphs in `[txt]`. G18 wrote
  `Header text.<tab><:P10,0,Page >`: a tab to the default centre tab and a
  page-number field `<:P…>`: the style character (`1`, `I`, `i`, `A`, `a`),
  the start page and `,` the start number (0 when not set), and `,` the
  leading text (`Page `); PN1 wrote `<:Pi2,1,Page >` (VERIFIED (decompile)
  and PN1; `PAGE_LAYOUT_NOTES.md`, "Page numbering").
- Field 13 continues the frame sequence: Standard's header and footer are 1
  and 2, an inserted layout's are 3 and 4 (G21), and page frames follow.

**Floating headers and footers** are not layout frames. Page ›
Header/Footer › Floating Header/Footer inserts a multi-line control into the
main text at the caret (G22):

```
Second main paragraph.<:H<*->
Floating header.

>
```

It takes effect from the next page. `<:h` (lowercase) is the floating footer,
and the escaped byte after either form is a set of flags (VERIFIED, V17,
V18; see "Behaviour checks for the model decision" below).

**Page breaks and inserted layouts.** Both are `<:p…>` controls at the end
of a paragraph:

- Page › Breaks › Insert page break wrote `Second main paragraph.<:p<* >>`
  and started a new paragraph for the following text (G20).
- Page › Insert Page Layout › Insert wrote a second `[lay]` named `1` (a
  complete layout with its own header and footer frames) and
  `Page two text.<:p</@>1>` at the caret, plus an empty paragraph after it.
  The new layout applied from the following page: page 3's text area moved
  to the 2.00 in margin set for it (G21).
- Reading `<:p` + one byte + an optional layout name: the byte is `<* >`
  (0x00) for a plain break, `@` (0x40) for a break with "Vertically center
  text above page break" (V23), and `</@>` (0x80) when a layout name
  follows. So the byte is a set of flags: 0x40 centre the text above the
  break, 0x80 a layout name follows (VERIFIED (decompile): with 0x80 the
  reader reads the name up to `>` and looks the layout up by name; 0x40
  VERIFIED, V23). The corpus has 16 `<:p<* >>` and no other form.

**`<*X>` escape** (VERIFIED, V19). `<*X>` is byte X − 0x20 (0x00–0x1F), by
analogy with `</X>` (+0x40) and `<\X>` (+0x80), and confirmed by typed
special characters: Ctrl+Enter, the hard return (a new line inside the
paragraph), writes `<**>` (0x0A), and Ctrl+-, the discretionary hyphen,
writes `<*7>` (0x17). The corpus's 162 `<**>` in body text are hard returns.
`<*X>` also encodes the flag bytes of `<:p…>`, `<:H…>`, and `<:h…>`. The
remaining corpus bytes (`<*(>` 0x08, `<*0>` 0x10, `<*1>` 0x11, `<*2>` 0x12,
`<*5>` 0x15, `<*:>` 0x1A) are still unknown; the importer reports them.

**Page table (`[pg]`).** `[pg]` gives the page count and then one record per
page, for example
`2 0 27 576 0 0 0 65535 65535 Standard 65535 0 0 …` (G20). The first three
numbers locate the end of the page in the main text (paragraph number and
character offset), the fourth is a flag word (`1025` on the last page), and
the name is a layout. This is a pagination cache written at save time. A
reader should recompute pages from the text and its `<:p` breaks; a writer
must regenerate it. Its exact fields are not needed for import.

**Paragraph layout cache.** Ami Pro sometimes writes `<:#h,w>` at the start
of a paragraph: `<:#568,9360>` for a two-line Body Text paragraph and
`<:#284,9360>` for a one-line one, that is height and text width in twips
(284 per 12 pt line). It appeared and disappeared between saves of identical
content (F01 vs L01), so it is a cache too.

**Tables** (TB1: Tools › Tables, 2 × 2, typed `A1`, `B1`, `A2`, `B2`):

- A table in the main text is an anchored frame (flags `0x1800C4`: table
  `0x4`, anchored, wrap around, opaque, and `0x100000`) with a `[tbl]`
  section instead of `[txt]`.
- Its anchor in the text is `<:t0>`, not `<:A n>`. `<:t n>` is therefore a
  **table anchor, not a tab** (tabs are literal tab characters, T02). The
  corpus has `<:t0>`–`<:t25>`. The anchor sits at the start of the paragraph
  after the table (in TB1 that paragraph was empty), and n uses the same
  numbering as `<:A n>` (VERIFIED, V20, below).
- `[tbl]` starts with columns and rows (`2 2`), then sizes, then `[data]`
  with one record per cell: `row col 0 0 0 0 0 0 1 0 0 0 0`, the cell's
  paragraphs, and `>`. `[e]` and `[tble]` close it. Cell fields beyond row
  and column are Phase 5 work.

**Frames are fixed to pages.** Frame field 1 is the page number of a Where
placed frame: 2 for a frame created on page 2 (F13), 1–17 in the corpus. A
frame on page 1 stays on page 1 when a page break inserted before all the
text moves every paragraph to page 2 (V01, VERIFIED on screen and in the
file).

**Page breaks are inline marks.** A break inserted at the start of the first
paragraph was written there, `<:p<* >>Main text paragraph one, …` (V01): the
text after the mark starts the next page. G20's break at the end of a
paragraph is the same mark in another place.

**Deleting anchors** (VERIFIED, V05, V06). Delete at the anchor mark at the
end of the last paragraph did nothing. Deleting a selection that contains
the mark asks "There is a floating frame in the selection to be deleted.
Would you like to continue?"; Yes deletes the text and the frame together,
and one Undo restores both.

## Frames

Readings from F01–F11, L01–L15, C01–C04, M01–M04, P01–P03, Z01–Z05, and the
synthetic files X01–X05. A field is VERIFIED when changing one setting changed
only that field, or when Ami Pro displayed a synthetic edit as described.

**Where frames live.** Every frame uses the same record shape, but it is
stored in one of three places:

- Frames placed on a page, anchored or not, are top-level `[frm]` sections
  after the `[tag]` sections and before `[lay]`.
- Repeating frames (Repeat all pages, Repeat right/left) move into the page
  layout: `[lay]` gains `[repfrm]` containing a `[lyfrm]` record (F08, F09).
- Headers and footers are `[lyfrm]` records under `[hrght]`/`[frght]` (and
  presumably `[hlft]`/`[flft]`), as noted above.

**Order is z-order within a kind** (VERIFIED, Z01/Z02). Top-level `[frm]`
sections are written back to front: the first is the bottom of the stack.
Send to Back on the second of two frames swapped the two sections and changed
nothing else. Ami Pro keeps three lists and writes them in turn
(`11c8:3ba0`): frames in table cells (`0x20`), Where placed frames, then
anchored frames (both corpora: 'F', 'FA', 'A', 'CA', 'CFA' in every file
but one), and it draws anchored frames below Where placed ones whatever
order they were made in (VERIFIED (fixture) FC7G, FC8;
`FRAME_CREATION_NOTES.md`).

**Record fields** (`[frm]` or `[lyfrm]`, values in order):

| # | Example | Meaning | Evidence |
|---|---|---|---|
| 1 | `1` | Page number of a Where placed frame (2 for a frame made on page 2). Stale for anchored frames: V12's frame on page 2 kept 1 | VERIFIED (F13, V10, X06, V12); 1–17 in the corpus |
| 2 | `66240` | Flag word, below | VERIFIED per bit |
| 3–6 | `2160 2880 5040 5040` | Left, top, right, bottom in twips from the page's top-left corner | VERIFIED (F01–F03) |
| 7 | `1` | Sides with a line: 1 = All, otherwise a mask of 2 Left, 4 Right, 8 Top, 16 Bottom (Top and Bottom = 24); 0 = none | VERIFIED (L05–L07); 0 CORPUS |
| 8 | `1` | A byte of two independent options, bits 1 and 4 (1 in fixtures; 1, 3, or 4 in the corpus): the reader stores it and the frame-information macro routine reports each bit; their dialog names are not settled | VERIFIED (decompile), meaning open |
| 9 | `1` | Line position: 1 Middle, 2 Inside, 3 Outside, 4 Close to inside, 5 Close to outside | VERIFIED (L09–L12) |
| 10 | `1 0 0 100 100 18 0` | Seven numbers: line style (1-based index in the Style list), shadow extent on the left, top, right, and bottom in twips, rounded-corner percentage (0 = square), shadow colour | VERIFIED (L01–L03, L08, L14, F11) |
| 11 | `0` | Line colour, Windows RGB integer (`0x00BBGGRR`; red = 255) | VERIFIED (L13) |
| 12 | `16777215` | Background colour, same encoding (white) | VERIFIED (L15) |
| 13 | `3` | Sequence number in file order. The header and footer layout frames are 1 and 2, so page frames start at 3. Renumbered when the order changes; not a stable identity | VERIFIED (Z01/Z02) |
| 14 | `0` | Anchored frames only: the anchor index Ami Pro wrote, equal to the frame's position among anchored frames, table frames included. Ignored when reading (below) | VERIFIED (X03, V20) |
| 15 | `720` | Anchored frames only. With para above: the horizontal offset from the left margin, a user setting (F06: frame left 2160 − margin 1440). Flow with text: the x offset where layout put the frame, a cache (V13: 1790, just after the text "Anchor paragraph."). Not an offset for table frames: MERCURY.SAM writes 20000 for a table Ami Pro shows centred at its rectangle (left 3376 in a 1440–10800 column), and corpus tables at the margin write 0 to 19456; the table's offset is field 3 from the left of its column | INFERRED (F06, F07, Z05, V13); tables VERIFIED (MERCURY display) |
| 16 | `2880 2160` | Anchored frames only. With para above: the width (F17: the left handle 725 twips out wrote `3605 2160`) and a height that goes stale: in the user's corpus the width always equals the rectangle's, but the height differs in 149 of 309 records (text frames store 0, 200, or 360 with rectangles 6,000 to 9,500 tall), and in the VM two such pictures stand at the rectangle's height (3,477, not 4,335; 6,373, not 4,365). The size is the rectangle's. Flow with text: the width and the distance from the frame's top to the baseline, which equals the height only while the frame stands on the baseline: dragged down 725 twips, F18 wrote `2880 1435` and the text line moved up while the frame stayed; made taller from the bottom, F19 kept `1435` and its rectangle grew; the height is the rectangle's. Table frames write stale values here (`0 65535` in TB1; V20's first table repeats the next frame's `1440 720`): their size is the rectangle, fields 3–6 | VERIFIED (F17–F19; With para above heights: VM display of a corpus document, 2026-10-12); INFERRED (X05); tables VERIFIED (TB1, V20) |

The corpus confirms the record length: all 224 anchored records have 16
values and all 39 others have 13 (CORPUS).

**Shadow.** Shadow depth is written as extents on the sides the shadow falls
on: Normal is 100 twips (the dialog shows 0.07 in), Deep 172 (0.12 in), None
0 0 0 0. The direction moves the extents: bottom-right is `0 0 100 100`,
bottom-left `100 0 0 100` (VERIFIED, L01–L03). A Custom depth could not be
entered from the keyboard: Enter in the depth field does not confirm the
dialog, so a custom value is still unobserved.

**Derived columns.** A frame's column positions are computed, not chosen
(`1260:0827`). A square frame's text area is inside its margins. With rounded
corners Ami Pro insets it further (`1530:0892`, VERIFIED (decompile) and
(fixture) F01, F03, FC1–FC3): the inset is (the shorter side − 2 × the line
style's width) × corner% × 3 / 2000, truncated (0 when that side is shorter
than twice the line), the same across and down except at 100 %, where each
axis keeps its own; Ami Pro's integer stand-in for r(1 − cos 45°). Each edge
is then max(margin + inset, inset + max(the line's part, its width)), the
line's part by its position (`1530:0c22`: Middle line/2 + margin/2, Inside
the margin, Outside the line, Close to inside line/2 + ¾ margin, Close to
outside margin/4 + line/2), so a frame without margins keeps its line's
width. Line widths by style (`1288:0800`): 1 10, 2 20, 3 40, 4 80, 5 and 10
120, 6 160, 7 240, 8 60, 9, 11, and 12 100 twips. F01 (2.00 × 1.50 in, 18 %)
gives 57, F03 (3.00 × 1.00 in) 38, FC2 (2,472 × 2,142) 57 where r(1 − cos
45°) gives 56, and FC3 (50 × 33, no left or right margins) 10. A reader
should recompute columns from the frame and its margins; a writer must emit
them.

**`[frmlay]`**, the frame's own layout (the same shape is used by headers,
footers, and the page layout's `[rght]`):

| # | Example | Meaning | Evidence |
|---|---|---|---|
| 1 | `5040` | Bottom edge of the frame (absolute); page height for a page | VERIFIED (F02, F03) |
| 2 | `2880` | Width | VERIFIED (F03) |
| 3 | `1` | The unit the dialog last showed the size in (as `[rght]` value 3) | VERIFIED (decompile) |
| 4 | `144` | Left margin | VERIFIED (M03) |
| 5 | `144` | Bottom margin | VERIFIED (M02) |
| 6 | `1` | The unit the margins were last shown in | VERIFIED (decompile) |
| 7 | `3024` | Top of the text area (absolute): frame top + top margin | VERIFIED (M04, F02) |
| 8 | `144` | Right margin | VERIFIED (M01) |
| 9 | `0` | Line between columns: 0 none, otherwise the line style index (the default is 2) | VERIFIED (C04) |
| 10, 11 | `2 0` | Unknown; 10 is 2 for frames and 1 for headers, footers, and pages (a new page frame writes `2 0`, FC1, FC9; 2 in 2,411 corpus page frames, 1 in 68) | — |
| 12–14 | `48 42 1368` | Change between saves without a setting change in page frames (`48 42 1368`, `160 5 14472`, `9 53 13360`, `0 0 0`), so they look like stale memory. Headers and footers write `1 1 0` and the page `1 0 2`. A writer must keep them as read; a reader should ignore them | CORPUS + fixtures |
| 15 | `1` | Number of columns, 1–8 | VERIFIED (C01, C02) |
| then | `2361 4839` | Column left and right edges (absolute). Between two columns there is one extra value, `0` so far: `2 2361 3480 0 3720 4839` | VERIFIED (C01, C02) |
| then | `0` | Number of tabs, followed by that many type/position pairs | VERIFIED (C03) |

Two columns of the default frame are 2361–3480 and 3720–4839: the gutter is
240 twips, which the dialog shows as 0.17 in. Line between columns narrowed
each column by 2 twips beside the gutter (C04).

**Tabs.** A tab is a pair: type, then position relative to the column's
left edge (the Standard layout's defaults, `1 720` to `1 8640`, are every
half inch from the margin). Type 1 is left (VERIFIED, C03). The default header's
`2 4680` and `3 9360` land on the page centre and the right margin, so 2 is
centre and 3 is right. VERIFIED (decompile), for every tab list (layouts,
frames, styles, `<:R…>`): the low 14 bits of the type are 1 left, 2 centre,
3 right, 4 decimal (anything else is left); a right tab within 280 twips
of the column's right edge moves to that edge; the top two bits (`0x4000`,
`0x8000`, `0xC000`) choose the leader drawn in the tab's gap: hyphens,
dots, and a line (VERIFIED, TB2).

How tabs lay out (VERIFIED (decompile `10d0:1881`, `14f0:08fa`,
`10d0:49b8`, `10d0:3ef4`) and TB2, whose six lines our layout reproduces):

- The stops are the paragraph's own ruler (`<:R…>`, below) if it has one,
  else the style's own tabs while `[brk]` 32 is on, else the column's.
  With Hanging indent a left stop at All + Rest comes first and the stops
  up to it are dropped.
- A tab reaches the first stop past where it starts: left, the text starts
  there; right, it ends there; centre, it is centred on it; decimal, its
  first `.` stands there (a decimal stop without a point aligns right).
- A tab with no stop left on the line runs to the line's end, so the text
  after it starts the next line, and draws no leader. A column without
  stops therefore sends every tab to the line's end.
- Only the text after a line's last tab is justified.
- `<:R1,COUNT,TYPE,POS,…,>` is the paragraph's own ruler (`11b8:030a`): a
  flag that must not be 0, a count (22 kept), and that many type and
  position pairs; it sets the paragraph's ruler flag, which outranks the
  style's and the column's tabs. `<:R>` sets nothing.
- A document without `[lay]` takes its style sheet's page layout (INFERRED:
  every style sheet carries one, user's guide; such documents' caches need
  the sheet's tab stops; 22 of the user's files and 39 ISO files). A left tab set at 0.50 in
in the frame dialog was written as 653, not 720; the 67-twip difference is
unexplained.

**Flag word** (field 2). Default text frame: `0x102C0`.

| Bit | Meaning | Evidence |
|---|---|---|
| `0x4` | Table frame: has a `[tbl]` section instead of `[txt]` | CORPUS (214 of 263 corpus frames) |
| `0x40` | Opaque; cleared for Transparent | VERIFIED (F10) |
| `0x80` | Wrap around | VERIFIED (F04: cleared for No wrap around) |
| `0x20000` | No wrap beside (with `0x80` cleared) | VERIFIED (F05) |
| neither wrap bit | No wrap around: text flows behind or through | VERIFIED (F04) |
| `0x80000` | Anchored; with `0x80` it is Flow with text, without it With para above | VERIFIED (F06, F07, P01, X01) |
| `0x100` | Repeating frame | VERIFIED (F08) |
| `0x2000` | Repeat right/left (with `0x100`) | VERIFIED (F09) |
| `0x800000` | Member of a group | VERIFIED (Z03, Z04) |
| `0x1000000` | First frame of a group (in file order) | VERIFIED (Z03, Z04) |
| `0x2000000` | Last frame of a group | VERIFIED (Z03, Z04) |
| `0x20000000` | Picture frame: has `[isd]` instead of `[txt]` | VERIFIED (P02, P03) |
| `0x200` | Text frame: has `[txt]` (the reader reads text only with this bit) | VERIFIED (decompile) |
| `0x10000` | Lines on: field 7 holds the sides; without the bit the reader sets field 7 to 0 (and older files with 0 or above 30 get 1, All) | VERIFIED (decompile) |
| `0x100000` | A `[chint]` section may follow the frame layout (set on most corpus table frames) | VERIFIED (decompile); CORPUS |
| `0x200000` | The frame record is followed by three strings (a link: application, topic, item) | VERIFIED (decompile) |
| `0x1`, `0x40000`, `0x10000000` | Frame contents read from `[btmap]` (a bitmap), `[mf]` (a metafile), and, without `0x20000000`, a section named `[nw]`; `0x2` is a further content kind | VERIFIED (decompile); not in the corpora |

The reader also sets `0x40` (opaque) whenever the background colour (field
12) is not white, whatever the file says (VERIFIED (decompile)).

**Placement of anchored frames.** The dialog ties wrap to placement: With
para above disables Wrap around, and Flow with text forces it. So an anchored
frame with `0x80` is Flow with text and one without it is With para above.
With para above + No wrap around (P01) writes `0x90240`; Flow with text
writes `0x902C0`. VERIFIED: X01 is F06 with only the flag word changed to
F07's, and Ami Pro shows it as Flow with text.

**Groups** are contiguous runs in z-order. Three grouped frames wrote
`0x18102C0`, `0x8102C0`, `0x28102C0`: every member has `0x800000`, the first
also `0x1000000`, the last also `0x2000000` (Z04). The user's guide says
anchored frames cannot be part of a multiple selection, so they cannot be
grouped.

**Anchors in the text.** Every anchored frame has a `<:A n>` mark in the main
text. `n` counts anchored `[frm]` records in file order, from 0; field 14 and
non-anchored frames do not affect it:

- Z05 wrote `Second main paragraph.<:A1><:A0>` for two frames, the second
  record first in the text. Ami Pro shows record 2 at the first mark.
- X03 swaps the two field-14 values: no change on screen.
- X04 inserts a Where placed `[frm]` before both: no change on screen.

With para above puts the mark at the end of its paragraph (F06). For Flow
with text, the frame is laid out inline at the mark like a large character
standing on the baseline, which raises that line; the stored rectangle is
recomputed (VERIFIED, X02: the mark moved into the middle of the first
paragraph). Creating a frame and switching it to Flow with text put the mark
at the end of the text, not at the caret (Z05, Z06 trials); the user's guide
describes moving the frame next to a word first. Only the mark matters when
reading.

**Picture frames** (P02 Copy image on, P03 off; a 64 × 48 synthetic BMP of
9270 bytes):

- Flags gain `0x20000000`. A `[frmname]` section names the frame (`Frame3`,
  from field 13). `[frmname]` also appears on named text and table frames in
  the corpus, and `[frmmac]` on frames with Run macro.
- `[isd]` replaces `[txt]`. Its first values are `.X3`, the original file's
  extension (`.bmp`), and `.X3` when the image is copied or the original path
  (`L:\IN\PIC.BMP`) when it is linked, then about 60 numbers not yet decoded
  (scaling and cropping are the likely contents).
- The image bytes follow the body. Everything after the body's closing `>`
  line is binary, and `[Embedded]` near the end of the file is a directory:
  `3 .bmp 5329 9270 14599 64646` means frame 3's original `.bmp` is at byte
  offset 5329 with length 9270, and a second object is at offset 14599 with
  length 64646. The original BMP is stored byte for byte (VERIFIED). The
  second object starts `53 53` and looks like Ami Pro's own rendering of the
  picture, presumably the `.X3` form.
- A linked picture writes `3 .bmp 0 0 5360 64646 L:\IN\PIC.BMP`: no copy of
  the original, the rendering still embedded, and the path at the end.
- The line after the directory, an eight-digit number, is the byte offset of
  `[Embedded]` itself. This also holds for documents without pictures
  (F01: `00004934`).

**Tables are frames.** 214 of the 263 corpus frames have flag `0x4` and a
`[tbl]` section, most with `0x180044` (anchored). Tables in the main text are
therefore anchored table frames, which matches the user's guide listing
"page tables" with floating frames as one text stream (CORPUS; a table
fixture will confirm).

**Text streams.** The user's guide (chapter 11, Find & Replace) lists the
streams of a document in this order: main text; text in fixed (Where placed)
frames; footnotes; text in floating (anchored) frames and page tables;
floating headers and footers; fixed headers and footers and repeating frames.
Notes and Draw frames are not searched.

**No linked frames.** The Frame menu has Create Frame, Modify Frame Layout,
Graphics Scaling, Group, Bring to Front, and Send to Back, with no linking
command. The user's guide says text that does not fit a frame stays hidden
until the frame is enlarged (VERIFIED for the menu; overflow from the guide).

**Corpus readings from the 2.4 importer** (CORPUS, 2026-10-11; 160 files):

- 774 frames: 500 header and footer frames (250 each), 214 anchored tables
  (189 With para above, 25 Flow with text), 22 Where placed and 2 repeating
  text frames, and 36 pictures (18 Where placed, 9 With para above, 9
  repeating). 223 anchor marks resolve; none is left over. 150 documents have
  one layout and 10 have two.
- Table headers are `ROWS COLS ROWHEIGHT ? COLWIDTH ? ? ? ?`, with optional
  `[h]` row records and `[w]` column records, each closed by `[e]`, before
  `[data]` (SAM_PAGE_FRAME_SPEC.md §4.7). A cell record with 128 in its third
  field has no text (a covered cell); a calculated cell writes its formula as
  a second body after its value (MERCINV.SAM).
- One anchored frame record has no mark (AP3CH02.SAM); one inserted layout has
  left header and footer frames without a `[lft]` page (CH10.SAM); one layout
  repeats its left header and footer slots (APPENDIX.SAM).
- Unknown frame flag bits seen: `0x200`, `0x10000`, `0x100000` (known), and
  `0x8000000` on one picture frame (MERCURY.SAM).
- Every page's stored column edges match uniform columns derived from its
  margins and first gutter (within 2 twips).

**Open frame questions:** the names of field 8's two options (and field 1
for repeating frames); field
16's second value for Flow with text (V13 wrote 0); where a Flow with text
frame's vertical offset is stored; `[frmlay]` values 3, 6, 10, and 11; the
value between columns; the frame tab offset; custom shadow depth; the
`[isd]` numbers and the rendering format; line styles beyond the third; and
the `[chint]` section that `0x100000` allows.

## Frame gestures (F14–F20, 2026-10-12)

What Ami Pro 3.1 does when frames are moved and resized with the mouse,
compared with the page surface's commands (`UI_SPEC.md` §11):

| Gesture | Ami Pro | Evidence |
|---|---|---|
| Drag a Where placed frame from inside | The rectangle moves by the pointer's offset, to the twip (88, 44 pixels at 87.4 per inch: 1450, 725); no grid | VERIFIED (F14) |
| Drag its bottom-right handle | Only the right and bottom edges move (725, 363) | VERIFIED (F15) |
| Drag a With para above frame diagonally | It moves across only; field 15 (the offset) and the rectangle change by 1450 | VERIFIED (F16) |
| Drag its top handle | Nothing changes, although Ami Pro draws the handle | VERIFIED (F17 screenshots) |
| Drag its left handle | The left edge moves: the offset falls by 725 and the width grows by 725 | VERIFIED (F17) |
| Drag a Flow with text frame diagonally | It does not move across; dragged down it hangs below the baseline (the line's text moves up beside it), recorded in field 16 | VERIFIED (F18) |
| Drag its bottom handle | The frame grows downward and keeps its top's height above the baseline | VERIFIED (F19) |
| A line after a Flow with text frame that hangs below the baseline | Starts below the frame | VERIFIED (F20) |
| Make a Wrap around frame With para above (Modify Frame Layout) | Wrap around is disabled and No wrap beside is chosen; Flow with text disables every wrap option | VERIFIED (F16, F18 dialogs) |

Selected frames show eight 7-pixel handles in every placement; on a Where
placed frame the right and bottom middle handles are drawn hollow.

## Behaviour checks for the model decision (2026-10-11)

These are the remaining checks from `docs/Design/PAGE_FRAME_MODEL_BRIEF.md`
section 6, made in the XP VM before the Phase 2.2 decision. Screenshots are
`s22-*` in `../AmiPro-Reference/vm-lab/screens/` (outside Git).

**Fixed frames and the page count** (VERIFIED, V10, V11, X06):

- V10 wrote field 1 = 2 for a frame created on page 2.
- Removing the page break moved all text onto page 1. The frame stayed on
  page 2, which remained as an empty page holding only the frame, and
  `[pg]` still listed two pages (V11). (Page › Breaks › Remove page break
  is enabled only with the caret at the break.)
- X06 is V11 with field 1 changed to 3 and the `[pg]` cache left at two
  pages. Ami Pro showed the frame on page 3 and created empty pages 2 and 3.
- Rule: the page count is the larger of the pages the main text needs and
  the highest page number of a Where placed frame. `[pg]` is not trusted.

**Anchored frames at the end of a page** (VERIFIED, V12, V13):

- With 41 empty paragraphs before it, the one-line anchor paragraph would
  still fit on page 1 (about 0.7 in was left), but its 1.50 in With para
  above frame would not. Ami Pro moved the paragraph to the top of page 2,
  with the frame below it, and left the bottom of page 1 empty (V12).
- Switched to Flow with text, the frame stands inline after the paragraph's
  text and the whole line starts page 2 (V13).
- Not tested: a multi-line paragraph, where only the last line might move.
- The user's guide (chapter 15) adds: an anchored frame can be no wider than
  its column and no taller than the space left in the column or page, and
  Ami Pro resizes anchored frames when the column count changes. A With
  para above frame moves only horizontally, within the column; a Flow with
  text frame moves only vertically, by up to its own height.

**With para above frames in the text** (VERIFIED, MERCURY.SAM display and
stored rectangles, 2026-10-12):

- MERCURY's page 7 has seven 0.4-inch icons, With para above and No wrap
  around (`0x20080040`), with the text of the next paragraph beside each.
  Starting at the top margin and adding the cached paragraph heights
  (`<:#H,W>`) and the styles' `[spc]` space above and below places every
  icon at its stored top, to the twip, only if:
  - a No wrap around frame does not push the text after it down;
  - a paragraph whose only content is the mark (`<:s><:A3>`) takes no
    height, while its space above and below still apply;
  - a mark that ends a paragraph with text (`Lotus Application
    Manager<:A9>`) puts the frame below the text and before the space below;
  - a mark that starts a paragraph puts the frame at the paragraph's top,
    after its space above.
- The drawing on page 4 (No wrap beside, mark alone in its paragraph) lands
  at its stored top by the same rules, with the text after it below it.
- The sales table (mark alone in the paragraph after it) is shown 72 twips
  higher in the stored rectangle than the same rule gives, so a table may
  stand before that paragraph's space above; in the display the heading
  after it starts about 180 twips below the table, so the mark's paragraph
  takes no height there either.
- Paragraphs that contain a mark have no `<:#…>` cache.

**Table geometry, joined cells, and cell text** (2026-10-12):

- `[tbl]` header `ROWS COLS ROWHEIGHT V4 COLWIDTH V6 V7 V8 V9`: the row
  heights and column widths are text sizes; V4 is the vertical cell margin
  and V6 the horizontal one. A frame is its cells plus 43 twips: TB1
  (2 × (284 + 86) + 43 = 783, 2 × (4,569 + 86) + 43 = 9,353), V20 (1,893),
  README31.SAM (V4 = 0: 312 + 312 + 552 + 552 + 43 = 1,771). VERIFIED for
  those; CORPUS for 140 of 220 tables. The other 80 frames are smaller than
  their rows: tables that continue on later pages (CH2.SAM's 18-row table
  stores a 697-twip part at the top of a page).
- Joined cells: field 3 `384` (`0x180`) starts a block of field 4 rows ×
  field 5 columns; `128` (`0x80`) marks a covered cell, whose fields 4 and 5
  are its offset from the block's top-left cell. CORPUS: 27 blocks and 37
  covered cells in 220 tables, all consistent (only 1 × 2 and 1 × 4 blocks);
  VERIFIED that Ami Pro shows README31's three 1 × 2 blocks as single cells
  across the table.
- Cell text without a tag is Table Text: the user's guide (AP3CH16/18) says
  Ami Pro assigns Table Text to all cells, generated from Body Text with no
  space above or below. VERIFIED: MERCURY.SAM's cells, all untagged, show in
  Table Text's Arial while its Body Text is Times New Roman.

**Layers** (VERIFIED, V14). A Where placed frame created first and a Repeat
all pages frame created second overlap in the top margin. The page frame is
drawn on top. Bring to Front on the repeating frame and Send to Back on the
page frame change nothing. So top-level `[frm]` frames always paint above
layout frames (`[repfrm]`), and z-order commands act within each layer. The
fixed header text wrapped around both frames: wrap exclusions apply to
header and footer text as well as to the main text.

**Hidden overflow** (VERIFIED, V14). A frame too small for its text showed
none of it; the text was still saved in `[txt]`.

**Typing past a header's end** (VERIFIED, HDR1 and HDR2, 2026-10-18). In a
new `_DEFAULT.STY` document the header band (0.55 in to 0.95 in) holds two
lines of Body Text, which has no space below there. A third paragraph typed
after Enter is kept (`[hrght]`'s `[txt]` has `abc`, `def`, `ghi`, then
`ghiWWWWWW` after more typing) but not shown, and the caret stays at the
end of the last visible line however much is typed. The app's new
documents give Body Text 120 twips below (its own default since Phase 3),
so their band holds one line.

**Groups** (VERIFIED, V15, V16):

- Clicking one member selects the whole group. Dragging moves both
  rectangles by the same amount, and one Undo restores both positions.
- Del deletes both frames, and one Undo restores both.
- While frames are grouped, Modify Frame Layout, Bring to Front, and Send
  to Back are disabled, and Frame › Group shows a check. Choosing it again
  ungroups: the three group bits are cleared and the positions kept (V16).
- The user's guide adds that frames must be on the same page to be selected
  together or grouped, and that deleting one member deletes the group.

**Floating headers and footers** (VERIFIED, V17, V18, G22; CORPUS):

- `<:H` is a floating header and `<:h` a floating footer. The `<*X>` byte
  after the letter is a set of flags: 1 header, 2 footer, 4 left pages,
  8 right pages.
- All pages: header 0x0D (`<*->`, G22), footer 0x0E (`<*.>`, V17). Left
  pages, header: 0x05 (`<*%>`, V18).
- The corpus's 14 `<:h<**>` (0x0A, footer on right pages) contain a
  paragraph in a style named "right footer". Its 20 `<:h<*.>` include
  footers with no text.
- The dialog's Remove option is enabled only with the caret at an existing
  mark, and it deletes that mark. Removal is not stored; a footer with no
  text is an inserted, empty floating footer.
- The user's guide: floating headers and footers cannot be inserted while
  the insertion point is in a table, frame, or header/footer.
- While a floating footer was being edited in a one-page document, Ami Pro
  displayed a page 2, but `[pg]` kept one page.

**Special characters** (VERIFIED, V19): Ctrl+Enter (hard return) writes
`<**>`, Ctrl+- (discretionary hyphen) `<*7>`, and Ctrl+Space (no-break
space) `</`>`, the same 0xA0 that an Alt-code no-break space produced in
T02. See "`<*X>` escape" above.

**Anchor numbering** (VERIFIED, V20). A table, a Flow with text frame, and a
second table, created in that order, were written as `[frm]` records in the
reverse order: second table, frame, first table. The text was written as
`<:t2>Para two.<:A1>` … `<:t0>Para four.`. So `<:t n>` and `<:A n>` share
one numbering: n is the record's position among all anchored records (flag
`0x80000`), tables included, and field 14 equals n. Each table anchor is at
the start of the paragraph that follows the table.

**Frame tables** (VERIFIED, V21). Tools › Tables with a Where placed frame
selected turned that frame into a table frame: flags `0x100C4` (table bit
`0x4`, not anchored) and `[tbl]` instead of `[txt]`. A frame holds text, a
table, or a picture, never two of them.

**Breaks** (VERIFIED, V22, V23):

- The Breaks dialog offers Insert and Remove page break, Insert and Remove
  column break, and "Vertically center text above page break".
- A centred page break was written `<:p@>` (byte 0x40; see "Page breaks
  and inserted layouts" above).
- A column break inserted with the caret at the end of a paragraph was
  written `<:c>` at the *start* of that paragraph, which was not split. It
  had no visible effect in a one-column layout. The corpus has no `<:c>`;
  its effect in a multi-column layout is not yet observed.

## The reader in AMIPRO.EXE (decompile audit, 2026-10-12)

Where the routines are in the decompile (`DECOMPILATION.md`; Ghidra names
`FUN_SSSS_OOOO`, segment selector and offset). These are navigation aids;
the findings are in the tables above and in "Layout rules" below.

- **Section names** are strings in DGROUP (0x971–0xbd6, `[sty]` to
  `[fldnames]`), reached through a table of near pointers at DGROUP 0xbe6,
  one per name in the same order. The decompile shows each pointer as
  `DAT_1680_0bXX`/`0cXX`; the format strings the writer uses (`\t%u\r\n` and
  so on) follow the table.
- **Reading:** document open `1058:3753` (versions 3 and 4 only); the
  sections before `[edoc]`, a chain of name comparisons, `1058:06cc`; a style
  record `1058:0fac` with the `[fnt]` flag remap `1058:0f24`; a frame record
  `1058:1a7d`; body paragraphs `1118:0e55`, with escapes, attributes, and
  alignment marks in `1118:09d6` and the `<:…>` controls in `11b8:030a`.
- **Writing:** segment `11c8` (style record `11c8:26b7`, frame record
  `11c8:2a0f`); its field order matches the reader's.
- **Layout:** the per-line builder `10d0:0d82` (alignment, breaks, keeps),
  line spacing `10d0:1ecb`, indents `10d0:1e02`, tab stops `10d0:1881`, line
  breaking `10d0:0592` and `10d0:1f9e`, tab segments and their alignment
  `10d0:3ef4`, attributes `10d0:33c6`, a run's font `10d0:0309` with the
  `<:f…>` parser `14f0:0000`, hyphenation `1310:0000`, and the step that adds
  a line to a column `12f8:1321`.

## Layout rules from the decompile (2026-10-12)

All VERIFIED (decompile) unless marked.

**Single line height.** The tallest ascent plus external leading, plus the
deepest descent, over the fonts on the line, from the printer's font metrics
(the screen's when drawing). LH1's measurements fit this. Only characters
taken onto the line count (`10d0:359b` runs for each stretch of characters
the line builder consumes), so a format change after a paragraph's last
character changes no line. A paragraph without text is the exception
(`10d0:0592`): its line is the full height (height plus external leading)
and descent of the font in force after its controls, so `<:f200,,>` alone
in a 12 pt style is a 10 pt line (240 twips in the user's corpus, where the
style's lines are 276).

**Paragraph rules** (`10d0:0d82`; NX1, NX2): with `[line]` value 1 bit 1
(a rule above) the paragraph's first line is taller by value 8; with bit 2
(a rule below) its last line by value 9 (the table under "Style records"). Ami Pro's paragraph caches include them
(a 22 pt Arial bold chapter title with a 160-twip rule above caches 676, a
516 line plus 160).

**Line spacing** (`10d0:1ecb`), from the line's single height h:

- 1 1/2: h + ⌊h/2⌋, the baseline ⌊h/4⌋ lower.
- Double: 2h, the baseline ⌊h/2⌋ lower.
- Custom N: the height is N. If N ≥ h the baseline moves ⌊(N − h)/2⌋ lower;
  otherwise it moves ⌊(h − N)/2⌋ higher, but not above the top of the line.
- A paragraph's `<:S+…>` replaces the style's choice. `[spc]` value 2 is not
  used for 1 1/2 and Double. S1–S3's screen pitches (428 and 565 twips for
  12 pt Times New Roman, whose single line is 284) are within a screen pixel
  of 426 and 568.

**Space above and below** (`12f8:1321`, `10d0:0d82`):

- With "When not at break" (`[spc]` bit 32), the space above is skipped at
  the top of a column, and a paragraph's space below is counted only when
  another line follows it in the same column.
- Otherwise (Always, or neither bit), the space above is added at a column
  top too, and the space below belongs to the paragraph's last line: that
  line and its space below must fit in the column together.
- Frame text follows the same rule (VM fixtures FS4, FS5, frames of
  several interior heights): a 284-twip line shows from 320 twips of
  interior (not at 300), the same line with 288 twips below and Always
  from 650 (not at 600), and with When not at break from 340. The
  allowance of about 30 twips over the line itself is not explained (the
  frame's line, perhaps); it is not modelled.

**Breaks and keeps**, decided as each line is built (`10d0:0d82`):

- A line that is not its paragraph's last is kept with the next one unless
  the style allows breaks within (`[brk]` 4).
- After a paragraph's last line: a page break after (2) ends the page; a
  column break after (256) or a `<:c>` mark ends the column; keep with next
  (16) keeps the line with the next one. Otherwise the next paragraph's style
  decides: column break before (128) ends the column, page break before (1)
  ends the page, and keep with previous (8) keeps the two together.
- The line builder has no widow or orphan rule; none was seen in the
  pagination step either (not searched exhaustively).

**Alignment and justification** (`10d0:0d82`, `10d0:3ef4`): a line is split
into segments at its tabs. Each segment after a tab takes the tab's
alignment (decimal tabs align right at the decimal point); a left tab in a
justified paragraph stays justified; the text after the last tab takes the
paragraph's alignment. A justified segment spreads the extra width over its
spaces (the line breaker counts them). Which lines are left unjustified
(the paragraph's last, a line ending in a hard return) was not traced.

**Hyphenation** (`1310:0000`, `1310:02af`, `1310:03e0`; the line builder
`10d0:09b9`) runs only in styles with `[algn]` 256, VERIFIED (decompile):

- The line builder tries it only when the room left on the line before
  the word, in the paragraph font's average character widths, is at least
  the **hyphenation zone**: `[docopts]` value 1 (5 in all but one of the
  corpora's 832 records; Ami Pro's default 5).
- The word that does not fit is read up to 32 characters. A digit, a
  superscript digit or fraction (`²`, `³`, `¹`, `¼`, `½`, `¾`), or a
  no-break space keeps it whole; a discretionary hyphen in it is used
  first; letters inside No hyphenation text (`<+.>`) cannot take a break.
- The library (`ltshyph1.dll`, loaded through `AMIPRO.INI`, not in our
  install set) is asked only for words of five or more characters, for the
  last point that still fits, then earlier ones while the line is too long.

HY1 (2026-10-13; two hyphenated styles, left and justified, and a plain
one over long words, saved as HY1OUT): Ami Pro splits such words at line
ends ("specifi-cations", "implemen-tation", "In-ternational") and leaves
the plain style's whole. Since Phase 3.6 our layout hyphenates by these
rules with TeX's US English patterns (decision D2); HY1's eight paragraphs
have Ami Pro's line counts.

## Decompiled cross-check: the `w4w33f.dll` Ami Pro filter

`amipro.exe` is a self-loading NE whose code is not readable as stored, so
its section names do not appear as strings. The bundled conversion filter
`w4w33f.dll` (an ordinary 3-segment NE library) does contain them and
imports with stock Ghidra 12.1.3. Its frame reader (2026-10-11) is an
independent reading of the same records and agrees with the fixtures:

- `[frm]`: it reads the left, top (fields 3, 4), sides (7), and the line
  group (10), using the group's style and right/bottom shadow extents, and
  reads field 14 as the anchor index only when flag `0x80000` is set. It
  skips the right and bottom edges, fields 8 and 9, and fields 11–13.
- `[frmlay]`: it reads the bottom edge (stored as bottom − top, so value 1 is
  absolute), the width, the left and bottom margins, the text top (stored as
  text top − frame top, so value 7 is absolute), the right margin, and the
  flag value 9. It skips values 3, 6, and 10–14, then reads the column count
  and, per column, left, right, and a tab count followed by that many
  type/position pairs.

So this third-party reader needs nothing from frame field 8 or `[frmlay]`
values 3 and 6, and it confirms the per-column tab lists (DECOMPILED). Those
fields stay unknown; they must be preserved when writing.

