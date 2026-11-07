# Find & Replace in Ami Pro 3.1: evidence (2026-10-19)

Prepared overnight for the Phase 5 queue's run 3 (`find`,
`docs/playbooks/PROOFING.md`), not built: its interface is the user's
decision (`docs/SESSION_HANDOFF.md`, "What I need from you"). Decompiler
output stays outside Git; these are findings in my own words. Evidence
levels as in `SAM_FORMAT_NOTES.md`: VERIFIED (decompile), VERIFIED
(fixture), VERIFIED (manual) for the 3.1 user's guide, chapter 20, and
INFERRED.

## The commands and dialogs (VERIFIED (decompile))

- Edit › Find & Replace… is command `0x1c3` (Ctrl+F). Its handler
  (`1368:0000`) posts command `0x1e0` to the main window (`0x1e8` in one
  case), whose handler (`1368:0057`) opens the dialog; the search state
  (options, strings, the place reached) is one structure (`DAT_1680_28cc`).
- `AMIPROUI.DLL` holds four dialogs:
  - **1016 "Find & Replace"**: Find, Replace with; Find (default), Cancel,
    Replace All, Options…, Attributes….
  - **1121 "Find & Replace Options"**: Find options Whole word only, Exact
    case, Exact attributes; Replace options Exact case, Exact attributes;
    Range & direction Beginning of document, Include other text streams,
    Find backwards; Find & replace type Text or Style.
  - **1007 "Find & Replace Attributes"**: Find and Replace attributes, each
    Normal, Bold, Italic, Underline, Word underline, Small caps.
  - **1012 "Find & Replace"**, modeless, while stepping: "Finding:" and the
    text, "Replacing with:" and the text, a line naming the text stream
    being searched, and Replace & Find Next, Find Next (default), Replace
    Remaining, Cancel.
- The stepping commands are `0x1f0`–`0x1f3` in segment `1368`
  (`1368:057d`, `06a7`, `0c90`, `018f`); which is which button is INFERRED
  from their shapes (a find loop; a replace then find; closing with the
  totals; replacing the rest).

## The rules (VERIFIED (manual) ch. 20 unless marked)

- **Defaults** (VERIFIED (fixture), FR1): Whole word only and Include
  other text streams on; the rest off; type Text. The options are kept for
  the session, across documents (FR3).
- **Text streams**, in priority order: main text; Where placed frames;
  footnotes; With para above and Flow with text frames and page tables;
  floating headers and footers; fixed headers, footers, and repeating
  frames. Notes and Draw frames are not searched. With the caret in the
  main text, Include other text streams also searches the lower streams,
  from the beginning of the document; with the caret in another stream,
  that stream and the lower ones are searched and Beginning of document is
  off and dimmed. VERIFIED (fixture) FR5, FR6: Replace All from the main
  text with the option on changed the main text, a page table's cell, and
  the header; with it off, only the main text (the cell is another
  stream).
- **A selection** limits the search to it.
- **Whole word only** finds text with no letters around it (FR1B: "the"
  found, "theater", "then", "thesis" not).
- **Case** (VERIFIED (fixture) FR1, FR1B, FR2):
  - Find Exact case off: any capitals match.
  - Replace Exact case off: the replacement is set in the found text's
    capitals, whatever was typed: all capitals stay all capitals ("STATUS
    BAR" → "WINDOW PANE"), a capital first letter capitalizes each word of
    the replacement ("Status bar" → "Window Pane", "The" → "An" for "aN"),
    otherwise lower case ("sTatus bar" → "window pane").
  - Replace Exact case on: the replacement exactly as typed (FR2).
- **Attributes**: considered only with Exact attributes on (find and
  replace each); the Attributes dialog is dimmed for a style search.
- **Wildcards**: `?` one character, `*` one or more (never none); a `*` in
  the replacement takes the characters its `*` in Find matched (FR3:
  `phon*` → `telephon*` turned "phones" into "telephones" and, with whole
  word off, "telephone" into "teletelephone": `*` stopped at the word's
  end, INFERRED from that); `<*>` and `<?>` are the characters themselves.
- **Special characters**: Ctrl+Tab in a field is a tab, Ctrl+Enter a
  return; a character Ami Pro shows as a mark is written in `<< >>`.
- **Style** type: Find names a style, Replace with another; Replace changes
  those paragraphs' style. A function key in the field chooses Style.
- **Find** selects the next match and opens the stepping dialog (FR7: the
  match in a page table's cell first, then the header's, the stream line
  saying which stream it searches); the dialog closes when nothing is
  left. **Replace All** replaces every match without stopping.
- **The end**: the status bar reports how many were found and replaced
  (not caught on screen in FR7: it may be brief).
- **Undo**: none. After Replace All, Edit › Undo is dimmed (VERIFIED
  (fixture) FR4, without saving in between).

## Fixtures (`../AmiPro-Reference/vm-lab`, 2026-10-19)

| Fixture | What | Shows |
|---|---|---|
| FR1, FR1A | Five capitalizations of "status bar"; Replace All with "window pane", defaults | Case taken from the found text |
| FR1B | "the" among "theater then The thesis THE"; Replace All with "aN" | Whole word; the typed case dropped |
| FR2 | FR1 with Replace Exact case | The replacement as typed |
| FR3 | `phon*` → `telephon*`, whole word off | Wildcards |
| FR5, FR5A | "alpha" in the main text, a page table's cell, and the header; Replace All | Other text streams searched |
| FR6 | FR5 with Include other text streams off | Only the main text |
| FR7 | Find in FR6 | The stepping dialog, stream by stream |

Screenshots `fr-*`, `fr1*`–`fr7*` in `screens/`.

## The app today

Edit › Find & Replace… is registered as Future and disabled (`ED-02`);
nothing searches. The editing layer has no multi-stream walk; each flow's
text is plain `String` with runs, so a search over flows in Ami Pro's
order and one `Command` per replacement (or one grouped command for
Replace All) fit the existing model.
