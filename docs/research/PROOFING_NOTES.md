# Proofing in Ami Pro 3.1: spelling, thesaurus, grammar, language

**Written 2026-10-26** for the Phase 5 queue's run 11 (`proofing`,
`docs/playbooks/PROOFING.md`), from the decompiled `AMIPRO.EXE` and
`AMIPROUI.DLL` (`docs/research/DECOMPILATION.md`; new `DecompAt.java` output
for `1320:12bc`, `1340:00a8`, `1340:0114`), Lotus's proofing modules from
the ISO's `FROMROM\LOTUSAPP\SPELL` (`LTSSPL1.DLL`, Houghton Mifflin
International CorrectSpell; `LTSTHES1.DLL`, Electronic Thesaurus;
`LTSGRM1.DLL`, CorrecText; all three plain NE DLLs that stock Ghidra
imports, `out/ltsspl1.c`, `out/ltsthes1.c`, `out/ltsgrm1.c`), the dialog
templates (`tools/ne_resources.py`), both corpora (counted only; no file
names), and Ami Pro 3.1 fixtures made in the XP VM on 2026-10-26 (below).
Findings are in our own words: routine addresses, rules, and formats, never
decompiler output. Lotus's word lists, thesaurus, and rule sets are
proprietary and are never copied; only their files' roles and formats are
recorded. Evidence: **V** VM fixture, **D** decompile, **D-** decompile
lossy there, **I** inferred, **G** the user's guide, **C** corpus.

## The VM fixtures (2026-10-26; `../AmiPro-Reference/vm-lab/OUT`)

The guest's `LOTUS.INI` points `[Spell Checker]`, `[Thesaurus]`, and
`[Grammar Checker]` at `D:\FROMROM\LOTUSAPP\SPELL` (the ISO), and the user
dictionary at `C:\WINDOWS\LOTUSAPP\SPELL` (`User Path`; `AMIPRO.INI
[AmiPro] userdictionary=`).

| Fixture | Made how | Found |
|---|---|---|
| PSP1 | Typed "Teh cat sat on the the mat. We recieve abc123 and Xyzzy and NASAA today." / "Another Teh line with teh word."; Tools › Spell Check | Defaults: Check from beginning off, Include other text streams on; Options: repeated words, numbers, initial caps on, user dictionary alternatives off. "Teh" offers The, Tech, Tee, Tea, Ted, Tel (**the alternatives take the word's capitals**: "teh" offers the, tech, tee…; "NASAA" offers NASA, NASSAU…); Replace and Replace All disabled until the box differs. Replace All "Teh" → "The": the later "Teh" was replaced silently, **"teh" was asked again** (Replace All is exact-case). "the the": a message box "The word immediately before this one was spelled exactly the same.", no alternatives; an empty Replace with deleted " the" (the space before too). "abc123" "No Alternative Found."; Add To Dictionary wrote `C:\WINDOWS\LOTUSAPP\SPELL\LTSUSER1.DIC` = `abc123\r\n`. Cancel, then Undo: only the last replacement came back (Ami Pro's one undo level) |
| PSP1 (thesaurus) | Caret in "sat", Tools › Thesaurus | "sat" selected; Meaning variations "posed"; Synonyms "posed, postured"; Meaning "1) verb  To assume a particular position, as for a portrait": **synonyms come inflected as the word** |
| PSP1 (thesaurus) | Caret in "Another" | Synonyms "Like, Second, Double, Identical, Duplicate" (**the word's capitals**); Replace put "Like" in place, selected, and **the dialog stayed open** |
| PGR1 | Tools › Grammar Check | Defaults: set Business Writing; Show readability statistics, Show explanations, Include other text streams on; Check from beginning, Check in draft mode off. Sets: Business Writing, All Rules, Legal Writing, Technical Writing, Fiction Writing, Academic Writing, Formal Writing, Casual Writing. On clean text the run ends with Readability Statistics: 20 words, 3 sentences, 2 paragraphs, 25 syllables, 0 three-syllable words, 6.7 words per sentence, 1.5 sentences per paragraph, 0% passive; Fog 2.7, Flesch-Kincaid 1.8, Flesch Reading Ease 94.3, its grade level 5.6. "This are a apple.": the sentence selected; Suggestions "1. Consider These instead of This. OR 2. Consider is instead of are." (the words in red), then the rule's explanation; Replacement options are the whole sentence rewritten; Replace put "This is a apple." in and **checked the same sentence again** ("Consider an instead of a.", with a paragraph of explanation). Cancel shows no statistics. Saved: `[GramStyle]` = `Business Writing` (the set the check used) |
| PLN1 | Language Options: Language for current document British | `[lang]` = 2. The list shows only languages whose lexicon is installed: American, Brazilian, British, British IZE, Canadian-French, French, Portuguese, Spanish (sorted by name) |
| PSP2 | A clean paragraph selected; Spell Check | No dialog with a selection: only the selection is checked, and the status bar says "Spell check complete."; no `<:s>` written |
| PSP3, PSP4B | A whole check (Skip at each stop); a new document "Clean words only." / "Another good line." checked | PSP4B: `<:s>Clean words only.` — **a paragraph checked clean is flagged `<:s>`**; the last paragraph of the text is not (nor in PSP3) |


## Spelling (decompile)

Sources: `out/amipro.c`, `out/amipro-cmds.c`, new `out/spell-cmds.c` (1320:12bc)
and `out/spell-cmds2.c` (1340:00a8, 1340:0114), both made with DecompAt.java
(proj2, -readOnly); `out/amiproui.c`; `out/ltsspl1.c`; string tables of
AMIPRO.EXE (unpacked) and AMIPROUI.DLL; DGROUP bytes of AMIPRO.EXE and
LTSSPL1.DLL; the sample .DIC files. Evidence levels: **D** = read in the
decompile (or the binary's data), **I** = inferred, needs the VM.

### 0. Architecture (who does what)

- Code lives in AMIPRO.EXE segments **1320** (engine driver), **1328** (start,
  frequent-word list), **1340** (user dictionary files), **1348** (text-stream
  walker), **1190** (menu entry, Language Options apply). D.
- Menu Tools > Spell Check = command 0x1c5 (`1190:0aae`). With no selection it
  posts 0x1cb, which shows the "Spell Check" options dialog (template 1099,
  `SPELLOPTSPROC` 1280:0000); OK (result 1) posts 0x1db (the correction dialog,
  template 1040, `SPELLPROC` 1288:0000), Edit Dictionary (result 2) posts 0x1c6.
  **With a selection, 0x1c5 posts 0x1db directly: the options dialog is skipped.**
  (`1190:0aec`) D.
- The correction dialog drives the engine by WM_COMMAND to the main window:
  0x1dc start (`1328:0000`), 0x1dd find next (`1320:0000`), 0x1de end
  (`1328:02cd` → `1320:12bc`), 0x1df Skip (`1320:0db9`), 0x1d8 Skip All
  (`1320:0dcb`), 0x1d7 Add To Dictionary (`1320:0e0e`), 0x1d9 Replace All
  (`1320:0e5c` → `1320:0c67`), 0x1da Replace (`1320:0e73`). Strings pass
  through AMIPROUI's word/string queue (AMIUIADDWORD/STRING,
  AMIUIREMOVEWORD/STRING = DAT_19c4/19d4/19e0/19dc). The mapping of buttons to
  0x1d7–0x1df is inferred from what each handler does (SPELLPROC's SendMessage
  arguments were lost by Ghidra). D/I.
- LTSSPL1.DLL (Houghton Mifflin International CorrectSpell, "May 1992") does
  the lexicon work. Ordinals: 2 OURINIT, 3 OURVERIFY, 4 OURCORRECT, 5 OURTERM,
  6 OURADDUSER, 7 SKIPALL, 8 REPLACEALL, 9 QUERYLANGUAGES, 10 OURREADUSERDICT,
  11 GETICBUFFSIZE, 12 CLEARPREVWORD. Ami Pro loads it from lotus.ini
  `[Spell Checker] Program Path` + `\Ltsspl1.dll`, falling back to the default
  search path; failure → "Cannot load the spelling program." (`1320:13b1`) D.
- One engine session per Spell Check run: OURINIT at start (`1320:13b1`),
  OURTERM at end (`1320:12bc`). D.

### 1. Breaking text into words

Token scanner (`1320:0541`, skip-blanks `1320:0472`), on Ami Pro's internal
paragraph bytes. D:
- A token is a run of bytes > 0x20 other than 0xA0. Ends at space, NBSP, tab,
  byte 10, and at 5-byte object codes whose type letter is `p` (page break /
  layout), `F` (footnote), `t` (table anchor), `A` (frame anchor), `X` (power
  field), `c` (column break), `C`. Other 5-byte codes, 2-byte codes,
  length-prefixed codes (type 4) and 0x1B attribute escapes are skipped
  **without** ending the token, so a bold change inside a word does not split
  it. Other single control bytes are skipped.
- **Punctuation, apostrophes, hyphens and digits stay in the token** handed to
  the speller. OURVERIFY itself returns the start offset and length of the real
  word within the token (Ami Pro stores them at state +4 / +2, `1320:0853`
  converts them to document positions, skipping internal codes with
  `1320:090b`), so leading/trailing punctuation trimming, apostrophe and
  hyphenated-compound rules are the HM engine's (its tokenizer `1018:1fa0`; not
  mapped further). The sample user dictionaries hold hyphenated words
  (`multi-colonnage`), so hyphenated compounds are looked up whole. D (that the
  engine decides), I (its exact rules).
- Token length: the scanner stops at 63 bytes; the rest of a longer run becomes
  the next token. The engine refuses words over 64 characters (code 8). D.
- **One-character tokens are never sent to the speller** (always accepted). An
  alphanumeric single character equal to the previous single-character token in
  the same paragraph is a repeated word ("a a"). D (`1320:00c2`).
- **Frequent-word list**: at start Ami Pro reads `<Data Path>\LTSFR<cc>1.DIC`
  (e.g. LTSFRAM1 American, LTSFRBR1 British, LTSFRFR1 French; up to 50 words,
  200 bytes, read with whitespace-delimited scanning) (`1328:02e1`). A token
  equal (case-sensitive byte compare) to a list word is accepted without calling
  the speller; two equal list words in a row in the same paragraph are a
  repeated word. This is a speed path for "the, and, of…" plus their
  repetition. D.
- Other tokens go to OURVERIFY. Its results as Ami Pro uses them: 4 = correct,
  2 = misspelled, 5 = repeated word (the engine remembers the previous word;
  CLEARPREVWORD resets it — Ami Pro calls it after one-character tokens,
  frequent-list hits, Replace and Replace All), 6 = in the Skip All list,
  0x3C = in the Replace All list (with the replacement copied out). D.
- Options applied to a flagged word (`1320:067e`), D:
  - "Check words with numbers" off → a token containing any ASCII digit 0–9 is
    skipped (`1320:0eac`) — note it tests the *token*, punctuation included.
  - "Check words with initial caps" off → a word whose first character is upper
    case and none of the rest are is skipped (`1320:0ed6`, Windows IsCharUpper).
    ALL-CAPS words are **not** skipped by this option. (D, with some decompiler
    noise in the loop; I for the exact test.)
  - "Check for repeated words" off → repeated-word hits are ignored.
- Repeated word: Ami Pro extends the flagged span backwards over the single
  space/tab/NBSP before the second word (if it fits in 64), so Replace with an
  empty box deletes " word"; Replace All is not offered for it. The dialog says
  "The word immediately before this one was spelled exactly the same." and lists
  no alternatives (`1320:067e`, `1320:09a5`, `1320:0c67`). D/I (the message id).
- **Paragraph "checked" flag = the SAM `<:s>` mark.** The scanner skips any
  paragraph whose flag bit 4 (byte +5) is set, and sets it when it reaches the
  end of a paragraph it started at the paragraph's beginning without stopping
  on any word (a Skip All hit also prevents it). Text edits clear it (many
  routines, e.g. the insert routine `1070:361c`). The body parser sets it on
  `<:s>` (`11b8:030a` case 0x73) and the writer writes it back. So a second
  Spell Check skips paragraphs unchanged since the last clean check, even "from
  beginning". D (`1320:00c2`). **Consequence for AmiPro-Linux:** the SAM writer
  keeps `<:s>` as a legacy mark; it must drop it from any paragraph edited since
  import, or Ami Pro will never spell-check that paragraph again.
- Where it starts (`1328:0131`, `1328:01e2`), D:
  - Selection: from the selection start to its end, in the selection's stream
    only (state flag 0x9d bit 2); ends "Spell check complete.".
  - No selection, "Check from beginning" on: main text, first paragraph.
  - Otherwise at the caret, backed up to the start of the word the caret is in.
  - No wrap-around to the text before the starting point was found (I).
- Streams (`1320:0000`, walker `1348:0fec`): at the end of the current stream,
  unless a selection is being checked or "Include other text streams" is off,
  the walker moves on to the next stream kind and announces it in the status
  line. Status strings, in resource order: Checking Main Document Text,
  Checking Footnotes, Checking Repeating Frames, Checking Floating
  Headers/Footers, Checking Floating Frames, Checking Fixed Frames (AMIPROUI
  0x854–0x859). The walker steps forward through six kinds (1 main; 2 and 4
  walk frame lists DAT_2348/234a, taking frames that hold text; 3 an
  `1350:0303` chain; 5 DAT_221f; 6 frames per page) and stops after the last;
  it does not go back to kinds before the start. D (mechanism), I (which kind
  is which string; whether notes are ever checked — no note stream was seen).
- Every 750 words the driver polls the dialog so Cancel interrupts. D.

### 2. Options storage

- AMIPRO.INI `[AmiPro]` (file name "AmiPro.ini" at DGROUP 0x85c, section
  "AmiPro" at 0x854), key **`SpellOpts`**, a decimal integer; missing → 0.
  Read once (`1128` provider case 0x107), cached in DAT_0d64, written back when
  changed after the options dialog (`1328:0032`). D. Bits (SPELLOPTSPROC,
  SPELLOPTOPTSPROC; engine tests in 1320/1328), D:
  - 1 = Check from beginning of document (on when set)
  - 2 = Include other text streams **off** (set when unchecked)
  - 4 = Check for repeated words **off**
  - 8 = Check words with numbers **off**
  - 16 = Check words with initial caps **off**
  - 32 = Include user dictionary alternatives **on**
  Default 0 = from caret, other streams included, all checks on, no user
  alternatives.
- With a selection, the dialog is skipped; "from beginning" and "other
  streams" are forced (checkbox shown unchecked/checked and disabled when the
  dialog would show) and the options come from the INI. D.
- "Include user dictionary alternatives": after the engine's alternatives,
  Ami Pro asks OURCORRECT in its second mode (0x37) for suggestions from the
  user dictionary and appends them, total capped at 10 (`1320:09a5`). D.

### 3. User dictionary

- Folder: AMIPRO.INI `[AmiPro] userdictionary=`; if empty, lotus.ini
  `[Spell Checker] Data Path`. File **`LTSUS<cc>1.DIC`** where `<cc>` is the
  language's code (AMIPRO.EXE strings 0x609+n), except American =
  **`LTSUSER1.DIC`**; British IZE uses LTSUSBR1; British Medical and British
  Medical IZE LTSUSBM1 (`1340:0462`). The language is the document's
  (DAT_21c8) or the INI `dictlang` (flag DAT_21b7 bit 0x20 decides; I which way
  round). D.
- Legacy: if the LTSUS file is missing, Ami Pro opens `<[AmiPro] dictionary= or
  ".">\VE<cc>AB51.LEX` (Ami Pro 2's user dictionary), converts each line from
  OEM to ANSI, and marks the dictionary dirty so it is rewritten as LTSUS at the
  end. If neither exists an empty LTSUS file is created (`1340:0000`,
  `1340:032a`). D.
- Format: plain text, opened "rb"; each line's first token (delimiters tab,
  LF, CR, space) is a word; lines beginning with Ctrl-Z (0x1A) are ignored.
  Written as `word\r\n` per entry. D (DGROUP 0xad0 "%s\r\n", 0x968 delimiters).
  Case is kept (the sample holds both `Multi-colonnage` and
  `multi-colonnage`). Order on rewrite is whatever the engine's
  OURREADUSERDICT enumerates; the sample LTSUSFR1 is alphabetical,
  case-insensitive with capitals first on ties (I).
- Load: the whole file is fed word by word to OURADDUSER at start
  (`1320:13b1`). Add To Dictionary (0x1d7) adds the flagged word's trimmed span
  to the engine and marks the dictionary dirty; engine error 0x60 → "Cannot add
  any more words to the user dictionary. Free more hard disk space…". At the
  end (`1320:12bc`), if dirty, every engine entry is written to a temporary file
  (`1340:00a8`, `1340:0114`) which then replaces the dictionary
  (`1340:01e7`); a write failure deletes the temporary file. So added words
  reach disk only when the Spell Check run ends. No size limit in Ami Pro's code
  (the engine's limit not mapped). D/I (the rename step).
- Edit Dictionary (0x1c6, `1340:025c`): ends the Spell Check dialog and opens
  the user dictionary file **as an Ami Pro document**, through the ordinary
  open routine with ASCII import options (bit 0x10, or 8 when it is the legacy
  .LEX — I: ANSI vs OEM text). The user edits and saves it as text. The
  `AMISPELL.EXE` / `Samspell` strings are not referenced by any decompiled
  code: an Ami Pro 2 relic. D/I.

### 4. Languages

- Numbers 1–28 (AMIPROUI names at 0x815+n): 1 American, 2 British, 3 French,
  4 Canadian-French, 5 Italian, 6 Spanish, 7 German, 8 Dutch General,
  9 Norwegian, 10 Swedish, 11 Danish, 12 Portuguese, 13 Finnish, 14 American
  Medical, 15 Swiss German, 16 Greek, 17 Brazilian, 18 Australian, 19 Polish,
  20 Russian, 21 Catalan, 22 Nynorsk, 23 Dutch Preferred, 24 British Medical,
  25 Czech, 26 Russian Jo, 27 British IZE, 28 British Medical IZE. D.
- Two-letter codes (EXE strings 0x609+n): AM BR FR CA IT SP GE DU NO SW DA PO
  FI ME LE GR BP AU PL RU CT NY DP BM CZ RO BR BM (n = 1…28). D.
- Main lexicons: QUERYLANGUAGES lists language n as available when
  `<Data Path>\LOTUS<xx>1.DIC` exists, xx from LTSSPL1's table: EN for 1, 2,
  27; FR for 3, 4; IT; SP; GE for 7, 15; DU for 8, 23; NO for 9, 22; SW; DA;
  PO for 12, 17; FI; ME for 14, 24, 28; GR; AU; PL; RU for 20, 26; CT; CZ. It
  also accepts Ami Pro 2 lexicons `<[AmiPro] dictionary>\VE<cc>AA51.LEX` for
  1–20. So American and British share LOTUSEN1.DIC; the variant is the language
  number passed to OURINIT. D. `LTSFR??1` are the frequent-word lists (§1), not
  French; `LTSUS??1` user dictionaries.
- **`dictlang`** = AMIPRO.INI `[AmiPro] dictlang=`, "Default language for new
  documents", default 1. A new document takes it (several init paths, e.g.
  `1058` reader resets, line 19480 area). D.
- **`[lang]`** in a SAM = the document's language number (DAT_21c8), read by
  the document reader (`1058` section dispatch, DAT_0bf0) and written by the
  writer (`11c8:1c7b`). Spell check refuses to run when it is 0 or above 28
  (`1320:13b1` returns failure before any message: I what the user sees). So 0
  = no language. Changing it on load re-initialises hyphenation (`1310:0705`).
  D.
- No per-paragraph or per-style language: the style reader and body parser have
  no language field, and the speller is initialised once per run with the
  document's single language. D (by absence in the mapped readers).
- Language Options dialog (CHGLANGPROC 1188:0000; applied by command 0x1e1,
  `1190:0b30`): the path is lotus.ini `[Spell Checker] Data Path` (lower-cased;
  validated as a directory). If changed, it writes the same Data Path to
  `[Spell Checker]`, `[Grammar Checker]`, `[Hyphenation]` and `[Thesaurus]`. A
  changed default language is written to `dictlang`. A changed current-document
  language sets DAT_21c8 (written as `[lang]` at save) and reloads hyphenation.
  **Any of these changes marks the document modified** (DAT_21b5 |= 4, the
  flag the close prompt tests). D.

### 5. Replace, Replace All, Skip All

- Skip (0x1df): only re-establishes the selection; the run continues after the
  word. D.
- Skip All (0x1d8): SKIPALL adds the word (as typed, engine charset) to the
  run's list; at most 65 entries, then "Cannot add any more words to Skip All
  list. Use the Skip command instead." D.
- Replace (0x1da, `1320:0b12`): the flagged span is deleted and the text of
  "Replace with" (max 63 chars) inserted, **verbatim** — Ami Pro does not adapt
  its case. Without revision marking the caret is put at the start of the
  inserted text, so the replacement is itself checked next. With revision
  marking on (DAT_2a3a bit 0x40) the replacement's words are verified first; if
  all pass, checking resumes after it, else it is re-checked. If the deletion is
  refused (`1500:0000`, protected text I) nothing is replaced. D.
- Replace All (0x1d9, `1320:0c67`): verifies each word of the replacement; if
  all are correct (and it is not a repeated-word hit) REPLACEALL records
  word → replacement (max 65, then "Cannot add any more words to Replace All
  list…"), then replaces this occurrence as Replace does. Later occurrences are
  replaced **silently as the scan reaches them** (OURVERIFY 0x3C →
  `1320:1019` deletes the word span and inserts the stored text), i.e. only
  from here forward in the run, not earlier text. D.
- Both lists live in the engine and are freed by OURTERM: they last **one
  Spell Check run**, not the Ami Pro session, and are never saved. D.
- Formatting: the replacement is inserted at the deleted span's start with the
  ordinary insert routine (`1070:350d`), so it takes the formatting in effect
  there (I: the first character's).
- Case of alternatives: produced by the HM engine (list order is the engine's);
  Ami Pro converts them between ANSI and the engine's charset only
  (`1010:14e2`/`156b`). Whether the engine capitalises suggestions to match the
  flagged word: I.
- Undo: the spell code makes no undo record of its own and its edits go through
  the ordinary delete/insert routines; whether Edit › Undo after a check undoes
  the last replacement: I (VM).

### 6. Other details

- Alternatives (`1320:09a5`): repeated calls to OURCORRECT until it reports no
  more (0x41) or the 0x37 end, up to 10; an engine "known misspelling"
  correction (one or two words) comes first. Then optional user-dictionary
  alternatives (§2). None → the list is disabled and the label reads "No
  Alternative Found."; otherwise "Alternatives:". Replace and Replace All stay
  disabled until the user types in or picks from the list; a list click copies
  the alternative into "Replace with", a double-click replaces (SPELLPROC). D.
- While searching, the dialog shows the wait cursor; the found word is
  selected in the document (the driver selects the span, `1320:07dd`). D.
- End: when the walk has no more text the driver returns "complete": the
  dialog closes and the status line shows "Spell check complete." (SPELLPROC,
  string 0x85b). D.
- Cancel ends the run (0x1de): user dictionary written if dirty, OURTERM, the
  document redrawn. D.
- Engine load failure → "Cannot load the spelling program."; OURINIT failure
  → "Cannot find the spelling dictionary. Check the Language Path under
  Tools/Spell Check/Language Options." D.
- OURINIT receives a temporary file name (GetTempFileName prefix "DIC") and a
  buffer sized by GETICBUFFSIZE. D.

### Confirm in the VM

1. Second run skips paragraphs checked clean (`<:s>`); an edited paragraph is
   checked again; a fresh document saved after a clean check carries `<:s>`.
2. Initial caps option on "Smith" vs "McDonald" vs "NASA"; numbers option on
   "abc1" and "1st".
3. Repeated words across a paragraph break ("the" / "the" and "xylem" /
   "xylem"); "a a"; what Replace does with an empty box on a repeat.
4. Start mid-document without "from beginning": is the text above ever checked
   (wrap prompt)?
5. Stream order with frames, footnotes, headers, notes; are notes checked?
6. Case of suggestions for "Teh" / "TEH"; case of a Replace All replacement on
   "Teh" vs "teh".
7. Undo after Replace / Replace All.
8. `[lang]` 0 document: what Spell Check shows.
9. Which of `dictlang` or the document's language picks the user dictionary.
10. Order of LTSUSER1.DIC after Add To Dictionary.

## Thesaurus (decompile)

Written in my own words from `amipro.c`, `amiproui.c`, `ltsthes1.c` and the raw
binaries (outside Git). Evidence levels: **D** = read directly in decompiled
code; **D-** = decompile is lossy at that spot, behaviour inferred from
surrounding code / constants; **G** = Ami Pro 3 user's guide (AP3CH25.SAM);
**VM** = still to be confirmed in Ami Pro.

### 0. Where it lives and how it is dispatched

- Command 0x1C7 (Tools > Thesaurus..., guide: Alt+F1) is not in AMIPRO.EXE's
  DGROUP 0x46 command table because AMIPROUI.DLL runs it itself: AMIPROUI's
  DGROUP has a command-to-dialog table of 8-byte rows (command, dialog
  template, help/extra, flags); the row at DGROUP 0x4E0 is (0x1C7, dialog 1093,
  0, 0xFFFF). Dialog 1093 is the "Thesaurus" template; its procedure is
  `THESDLGPROC` at AMIPROUI 10a8:0000. (D)
- AMIPRO.EXE only supplies services through the dialog's callback (codes
  below), reached via `FUN_1128_...` (codes 0x47, 0x146) and `FUN_1130_0000`
  (codes 0x9E, 0x9F, 0xA0, 0xA1). Strings/words cross the boundary through the
  AMIUI word/string queue (`AMIUIADDWORD`, `AMIUIREMOVESTRING`, etc.). (D)
- Menu enablement: `FUN_1180_133d` enables 0x1C7 (with 0x1C3, 0x1C5) only when
  a global "busy/other mode" condition is off (`DAT_21b5 & 0x10` with
  `DAT_2467 != 1`, and `DAT_223d == 0`). (D; meaning of the flags not mapped)

Control IDs in template 1093: 25700 Word looked up (static), 25300 Replace
with (edit), 25350 Meaning variations (list), 25351 Synonyms (list), 25352
Meaning (list, 256x20 du, used as a multi-line text box), 25585 Replace
(default button), 25586 Lookup, 25587 Previous, 2 Cancel. (D)

### 1. Which word is looked up

Callback 0x9E (AMIPRO `FUN_1130_0000` case 0x9E) calls the shared
"word at caret" routine `FUN_1380_0000(select=1, kind=0, max=0x32, mode=1, buf)`
(AMIPRO 1380:0000). `kind` 0 is the thesaurus's word class. If a special mode
flag (`DAT_21b5 & 0x20`) is set the word is empty. (D)

Steps of `FUN_1380_0000` for the thesaurus (D):

1. No text caret (`DAT_222d == -1`) -> empty word.
2. **Selection present** -> start from the selection's start (only the first
   word of a selection is used; guide confirms: "If you select more than one
   word, Ami Pro looks up the first word only" (G)).
3. **No selection** -> `FUN_1380_04d5`: if the character at the caret is
   whitespace (char-class bit 8: space, tab, NBSP 0xA0, CR/LF...) or one of
   `. ! ? ; : } )`, or the caret is at the paragraph's last position, the
   caret steps back one character (skipping embedded control codes, via
   `FUN_11f0_0000`), unless that previous character is itself whitespace. So
   a caret just after a word ("word|", "word|.") looks up that word.
4. Ami Pro then uses its Ctrl+Left / Ctrl+Right word movement
   (`FUN_1220_022e` / `FUN_1220_00e8`) to find the start of the word holding
   the caret; if the caret is already at a word start it is kept; if the
   caret is on whitespace the caret position is kept.
5. From there it skips forward over every non-word unit (`FUN_1380_0579`)
   to the first letter. **Caret between words (on whitespace) -> the next
   word** (after step 3 failed to find a preceding letter). If no letter
   remains before the paragraph end, the word is empty.
6. Copy (`FUN_1380_0418`): letters are copied; non-letter units that are not
   whitespace are **skipped without ending the word**; the first whitespace
   after at least one letter ends it. Stops at the paragraph end, at 49
   characters (max 0x32 incl. NUL) or 64.
   - Word characters for kind 0 are **letters only** (AMIPRO's char-class
     table at DGROUP 0xF89, bits 1|2: A-Z, a-z and the Latin-1 letters
     0x80-0x9F/0xC0-0xFF that Windows-1252 maps to letters). Digits,
     apostrophes, hyphens and other punctuation are *not* word characters.
   - Consequence: `don't` -> `dont`, `John's` -> `Johns`, `well-known` ->
     `wellknown`, `abc123` -> `abc`, `end.` -> `end`. Embedded attribute and
     object codes (bytes 2, 3, 4, 0x1B) are skipped by their lengths. (D;
     the punctuation consequences are worth a VM check)
7. **The word is selected in the document**: the anchor goes to the first
   letter and the caret to just after the last copied letter (embedded
   punctuation inside the span is included in the selection), then the view
   is refreshed (`FUN_1088_012c`, `FUN_1068_2697`, `FUN_1068_2a0a`,
   `FUN_1228_15c6`). (D)

No case change is applied by Ami Pro; the word goes to the engine as typed.

Engine-side normalisation (LTSTHES1, D/D-):
- `OURTHESLOOKUP` (1008:0311) keeps the original string (DGROUP 0x203C) for
  case matching; if it equals the previous lookup exactly, the cached result
  count is returned without a new search.
- A key is built by `FUN_1060_0728`: leading/trailing punctuation removed,
  runs of spaces/newlines collapsed to one space (multi-word phrases are
  allowed), a hyphen at a line break dropped; flags note a trailing `.`,
  `-` or `'` (possessive / abbreviation hints). The text is mapped through a
  per-language code-page table (`FUN_1060_01de`); an accent-folding table
  (`AAAAAA c EEEE IIII OOOOO UUUU`) exists in the DLL's data.
- `FUN_1008_11a8` finds the core word inside surrounding punctuation and
  sets flags for apostrophes/hyphens/periods at either end; it reports the
  core's start and length to the caller (AMIPROUI does not visibly use
  them).
- **Inflections**: the engine is Houghton Mifflin's "RE" reference engine
  (exports `@REDB`, `@REENTLST`, `@REENTRY`, `@REETPAR1/2`, `@REIDXLST`,
  `@RESTRTAB`). The entry list is searched by the key and each meaning
  carries a part-of-speech code (meaning+8) and a 4-byte inflection code
  (meaning+0xE). The guide's "1,400,000 word thesaurus ... synonyms for
  40,000 root words" and the data tags `<WD>`, `<IWD>` (inflected word),
  `more `/`most ` strongly indicate inflected forms are mapped to root
  entries ("running" -> run's meanings). The DLL does **not** return the base
  form as text; "Word looked up" shows the user's word. (D- / VM)

### 2. The dialog

State block (LocalAlloc 0x1BF bytes) per dialog: current word (+0x6D),
working/replace text (+0x9F), last replacement (+0xD1) and the one before
(+0x3B), meaning count (+0x181, capped at **26**), current meaning (+0x183),
history list head (+0x17B), engine kind (+0x1A3: 1 = DLL), DLL handle and six
entry points, synonym cap (+0x1A5). A 4,680-byte buffer holds 26 meaning texts
of 180 bytes each. (D)

**Opening** (`FUN_10a8_0441`, WM_INITDIALOG) (D):
1. Load the engine (`FUN_10a8_0679`, section 3). Failure -> message box and
   the dialog closes.
2. Ask Ami Pro for the word (callback 0x9E). Empty -> "Click on word to look
   up and try again." (string 0x882 pushed at 10a8:0621) and the dialog
   closes.
3. Look it up (`FUN_10a8_0a8f`). Not found -> "%s is not in the thesaurus."
   and the dialog closes.
4. Found -> fill everything (as for Lookup below), select the first synonym,
   show the dialog.

**Lookup** (`FUN_10a8_0a8f`) replaces NBSP by spaces, shows the hourglass,
calls `OURTHESLOOKUP` (returns the number of meanings), then copies each
meaning's text from `OURTHESGETMEANING(i)` into the buffer. (D)

**Meaning text** built by the engine for meaning i (D):
`" " + (i+1)` for i < 9 (else just the number), then `") "` + part-of-speech
name (from the engine's string table, `FUN_1008_0f4b`) + two spaces, or
`")  "` when the meaning has no part of speech, then the definition; cut to
127 characters. E.g. `" 1) adj  large in size"` (shape; actual POS names
come from the data, VM).

**Meaning variations list** (`FUN_10a8_0dcc`): for each meaning with at
least one synonym, the meaning's first synonym, `OURTHESGETSYN(0, i)`. So the
list is one headword per sense, in the engine's order. (D)

**Meaning box** (`FUN_10a8_0b74` / `FUN_10a8_0c02`): Word looked up shows the
current word; the selected meaning's text is put into the Meaning list box,
word-wrapped by pixel width at spaces, continuation lines indented with 16
spaces. (D)

**Synonyms list** (`FUN_10a8_0f20`): `OURTHESGETSYN(k, meaning)` for
k < min(`OURTHESGETSYNS(meaning)`, ThesMaxSyns); then Replace with = the
first synonym of that meaning, and the first synonym is selected. (D)

Events (D):
- Selecting a different Meaning variation: Meaning box, Synonyms list, and
  Replace with (= that meaning's first synonym, i.e. the variation itself)
  are refilled. Guide step 4 agrees (G).
- Selecting a synonym: Replace with = that synonym.
- Typing in Replace with: kept as the working text (EN_CHANGE).
- **Lookup**: looks up the **Replace with text** (which a synonym click has
  set). Found -> the current word is pushed on the history, becomes the new
  word, all lists refill, meaning 1 and its first synonym selected. Not found
  -> "%s is not in the thesaurus.", the working text is reset to the current
  word and the current word is looked up again (lists unchanged in effect).
- **Previous**: does nothing when the history is empty; otherwise pops the
  last word and looks it up without pushing. The history is a linked list of
  50-byte nodes with **no fixed depth** (limited only by local heap); it is a
  back stack only (no forward). Freed on close.
- **Replace**: reads Replace with, remembers it (+0xD1, previous in +0x3B),
  sends it to Ami Pro (callback 0xA0). THESDLGPROC does not close the dialog
  itself after Replace (VM: whether it closes / whether further replaces work).
- **Cancel**/close: frees the buffer and history, calls `OURTHESTERM` and
  frees the DLL (or tells an external thesaurus program, 0x476), ends the
  dialog.
- Re-activation: WM_ACTIVATE sets a flag; the next command first calls
  callback 0xA1 and re-reads the word at the caret (0x9E) into the working
  text. This looks like support for the user moving in the document while
  the dialog is up (modeless use) (D-; VM).

**Replace in the document** (AMIPRO `FUN_1130_0000` case 0xA0) (D):
takes the string; if a text caret exists and a selection exists, deletes the
selection (`FUN_1500_0000`; or proceeds when typing-replaces-selection flag
`DAT_2a3a & 0x40` is set), inserts the string at the caret
(`FUN_1070_350d`, same insert path as typing) and redraws. So the
replacement takes the formatting at the insertion point, as typed text
does (attributes of the first character after deletion, VM). No case
adjustment is done on the Ami Pro side.

**Capitalisation** is matched by the engine (D): `OURTHESGETSYN` passes every
synonym through `FUN_1008_10ff`, which classifies the looked-up word
(`FUN_1060_00e4`): lower case (0), only the first letter capital (1), all
capitals (2), mixed (3). Patterns 1 and 2 are applied to the synonym
(`FUN_1060_0914`): `Big` -> `Large`, `BIG` -> `LARGE`; lower and mixed case
leave the synonym as stored. For two languages (codes 5 and 8, with a
per-language flag 0x80) capitals are further mapped through a table (likely
accents dropped on capitals, e.g. French). Since Replace with is filled from
GETSYN, the inserted word carries the pattern.

**Synonym qualifiers** (D): when a synonym entry has a second string, GETSYN
returns `synonym + " (" + qualifier + ")"`. That full text goes into the
lists and into Replace with (VM: what qualifiers look like and whether they
get inserted on Replace).

**Re-inflection** (D-): when the database's flag bit 2 is set, GETSYN looks
the synonym up in the index list (`@REIDXLST`) with the meaning's part of
speech and inflection code and returns the inflected form, then applies the
case pattern. So synonyms of "running" are expected to come back as -ing
forms. (VM)

No antonyms, related words or "see also" are exposed: the DLL exports only
meanings and synonyms, and the dialog has no other list. (D)

### 3. Options, INI keys, language

- `AMIPRO.INI [AmiPro] dictlang=` (default 1): the language number Ami Pro
  hands the thesaurus (callback 0x47 returns `DAT_21c8`; also set per
  document from Spell Check > Language Options, guide G). (D)
- `AMIPRO.INI [AmiPro] THES<n>=` (n = that language number): the thesaurus
  engine name; default `LTSTHES1.DLL` (string 0x881). If the lower-cased name
  contains `dll`, the DLL interface is used; otherwise Ami Pro talks to an
  external program with window class `AmiThes` (started through
  `lotus.ini [Thesaurus] Program Path`, private messages 0x46F lookup, 0x471
  init with a path atom, 0x473 get strings, 0x479 register, 0x476 done). (D)
- DLL interface: `LoadLibrary(<lotus.ini [Thesaurus] Program Path>\ltsthes1.dll)`
  (note: the literal `ltsthes1.dll`, not the THES<n> value), entry points by
  ordinal 1-6: OURTHESINIT, OURTHESLOOKUP, OURTHESGETMEANING, OURTHESGETSYNS,
  OURTHESGETSYN, OURTHESTERM. Empty path, load failure or a missing export
  -> "Cannot load the thesaurus program." (0x883). (D)
- `OURTHESINIT(language, data path)`, data path = `lotus.ini [Thesaurus]
  Data Path` (callback 0x146) with a trailing backslash. Init failure ->
  "Cannot find the thesaurus." (0x884). The data file is
  `<data path>lotus<cc>1.ths`, `<cc>` from a per-language two-letter table in
  LTSTHES1's data (index by language number: en, en, br, fr, fr, it, sp, ge,
  du, no, sw, da, ib, fi, en, ge, gr, po, au, pl, ru, ct, no, du, br, cz, ru,
  br, br) (D-; the strcat arguments are lost, the pieces "lotus" and "1.ths"
  are in the code). The ISO has `/FROMROM/LOTUSAPP/SPELL/LOTUSEN1.THS`
  (725,504 bytes) and `/CDNAV/XTRADIC/FR/LOTUSFR1.THS`,
  `/CDNAV/XTRADIC/SPAIN/LOTUSSP1.THS`. The VM's lotus.ini has both paths at
  `d:\fromrom\lotusapp\spell`.
- `AMIPRO.INI [AmiPro] ThesMaxSyns=` (default 25): the most synonyms shown
  per meaning. (D)
- Fixed limits: 26 meanings, word up to 49 characters from the document,
  meaning text 127 characters. (D)

### 4. Notes for a reimplementation

- Word pick-up rules (section 1) are specific: letters only, punctuation
  inside a word skipped rather than ending it, the word then selected.
- The data is proprietary (Houghton Mifflin); a reimplementation needs its
  own thesaurus data (e.g. a free one such as an OpenOffice/LibreOffice
  `th_en_US` MyThes file) and its own inflection handling to reproduce root
  lookup, re-inflection and case matching. Reading the user's own
  LOTUSEN1.THS would need the RE database format decoded (not done).
- The dialog never edits the document except through Replace (plus the
  selection made at open).

### 5. To confirm in the VM

1. Caret positions: inside a word, just after a word, before a period, in a
   run of spaces, at a paragraph end; a multi-word selection; `don't`,
   `John's`, `well-known`, a word with an embedded bold change. Does the
   word end up selected after opening and after Cancel?
2. `running`, `bigger`, `dogs`, `went`: is the root found, what does Word
   looked up show, are synonyms re-inflected?
3. `Big` / `BIG` / `bIg`: capitalisation of the lists and the inserted word.
4. The Meaning box text: the POS names and layout (`1) adj  ...`); any
   synonym with a `(...)` qualifier, and what Replace inserts for it.
5. Replace: does the dialog close? formatting of the inserted word (bold
   word replaced -> bold?), Edit > Undo after it, protected text.
6. Previous after several Lookups (depth), Lookup of a word not found.
7. Is the dialog modeless (can one click in the document while it is open,
   and does the next button press pick up the new word)?
8. Behaviour with dictlang changed (Language Options) and with
   `ThesMaxSyns=5` in AMIPRO.INI.

## Grammar (decompile)

Clean-room notes, written in my own words. Addresses are `segment:offset`
(AMIPRO.EXE selector = 0x1000 + 8(n−1); AMIPROUI.DLL and LTSGRM1.DLL by
their own segments). Evidence levels: **D** = read directly in decompiled
code or binary data; **I** = inferred from decompiled code where Ghidra
lost arguments or helper semantics; **V** = needs the VM.

### 0. Architecture (D)

Three modules cooperate:

| Module | Role |
|---|---|
| AMIPROUI.DLL seg `10b0` | the four dialogs: Grammar Check (template 1092, proc `GRAMMARDLGPROC` 10b0:0000 — despite the name it is the *start* dialog), Grammar and Style Options (3038, `GRAMMAROPTSDLGPROC` 10b0:03ce), Save Set Name (3040, `NEWRULEDLGBOX` 10b0:121c), Grammar Checker (3039, `GRAMMARSTARTDLGPROC` 10b0:134c — the modeless checker window), Readability Statistics (3053, `GRAMMARSTATSDLGPROC` 10b0:1c71) |
| AMIPRO.EXE seg `1330` | the session: loading the engine, walking the document, selecting the sentence, building the texts shown, applying replacements |
| LTSGRM1.DLL (Houghton Mifflin CorrecText, module name `GCS`) | the parser and rules, the per-sentence and document statistics, the readability formulas |

The UI and AMIPRO.EXE talk by `WM_COMMAND` (0x111) messages to the main
window plus the word/string queue (`AMIUIADDWORD`/`REMOVEWORD`/`ADDSTRING`/
`REMOVESTRING`) and the callback (`1128`/`1130`). Grammar command IDs:

| ID | Handler | Meaning |
|---|---|---|
| 0x1fb | 1330:0000 | Tools › Grammar Check… — posts 0x1fd (open the start dialog) |
| 0x1fc | (UI) | open Grammar and Style Options |
| 0x1fe | 1330:0021 → 1330:004b | start a session (options word + set name come from the queue) |
| 0x1ff | (UI) | Save Set Name dialog |
| 0x200 | 1330:04f2 | do one step (action code from the queue), find the next problem |
| 0x201 | 1330:01dd | end the session |
| 0x202 | 1330:080f | build the Suggestions text |
| 0x203 | 1330:0a94 | build the Replacement options list |
| 0x204 | 1330:04cf | is list item *n* a real replacement? |
| 0x205 | (UI) | show Readability Statistics with the block from callback 0x135 |
| callback 0x12e | 1128 | read `GrammarOpts` (default 0x0D) |
| callback 0x131 | 1128 | the set name to preselect (default "Business Writing") |
| callback 0x135 | 1330:24ea | compute and return the statistics block |

LTSGRM1.DLL exports used (by ordinal): 1 `GINIT`, 2 `GCS` (check from a
position; returns one sentence's result), 3 `GGETLEV3` (explanation text),
4 `GRELLEV3` (free it), 5 `GTERM`, 6 `GPROFILE` (enable/disable a rule),
9 `GCLRTOT`, 10 `GADDTOT` (add the last sentence to the document totals),
11 `GDOCSTAT` (compute readability). They are bound in 1330:0b4a.

### 1. Named grammar and style sets

### Where they live (D)

Sets are **not** in the GCS file. They are entries of section
**`[Grammar]` in AMIPRO.INI**, key = set name, value = three decimal
numbers `G,S,W`:

- the UI lists the section's keys in INI order (`AMIGETPROFILESTRING` with a
  null key; combo has no CBS_SORT) — 10b0:0000, 10b0:074a;
- 1330:0b4a reads `[Grammar] <set name>` (section name = AMIPRO.EXE string
  1389 "Grammar"; UI string 2267) and parses the three numbers;
- the Options dialog writes `G,S,W` back with `AMIWRITEPROFILESTRING`
  (10b0:0ed0 Save, 10b0:0fe3 Save As).

The installed AMIPRO.INI (VM copy `vm-lab/OUT/AMIPRO.INI`, and the Wine
prefix) holds the built-in list, in this order:

| Set | G | S | W |
|---|---|---|---|
| Business Writing | 1048575 | 1036174 | 2178 |
| All Rules | 1048575 | 1048575 | 1090 |
| Legal Writing | 1048567 | 827391 | 1154 |
| Technical Writing | 1048575 | 917495 | 4360 |
| Fiction Writing | 993271 | 1019523 | 545 |
| Academic Writing | 1048567 | 901119 | 1090 |
| Formal Writing | 1048575 | 1023999 | 546 |
| Casual Writing | 898551 | 823168 | 4356 |

AMIPRO.EXE also carries "Business Writing" as string 1014 (the default).

### G — grammar mask (D)

Bit *n* (n = 0…19) is the *n*-th checkbox of the Grammar rules page
(controls 25550…25569); a set bit = checked = on (10b0:08fe, 10b0:094a).

| bit | checkbox | engine rule |
|---|---|---|
| 0 | Noun phrase consistency | 200 |
| 1 | Mass vs. count; A vs. An | 201 |
| 2 | Verbal group consistency | 202 |
| 3 | Passive verb errors | 203 |
| 4 | Subject-Verb agreement | 204 |
| 5 | Agreement with Here or There | 205 |
| 6 | Pronoun errors | 206 |
| 7 | Commonly confused words | 207 |
| 8 | Word usage | 208 |
| 9 | Style parameters (the word order rules) | 209 |
| 10 | Repetitive expressions | 210 |
| 11 | Informal usage | 211 |
| 12 | Nonstandard expressions | 212 |
| 13 | Double negatives | 213 |
| 14 | Jargon words | 214 |
| 15 | Nonstandard modifiers | 215 |
| 16 | Clause errors | 216 |
| 17 | Format errors | 217 |
| 18 | Punctuation errors | 218 |
| 19 | Check quoted text | not a rule: a GINIT parameter (below) |

Rule 219 is always disabled (1330:0b4a calls GPROFILE off for it
unconditionally).

### S — style mask (D)

Bit *n* = *n*-th style checkbox (controls 25570…25578, 25597…25599,
25615…25622), engine rule 101 + *n*:

0 Wordy expressions, 1 Redundant expressions, 2 Inappropriate prepositions,
3 Informal expressions, 4 Clichés, 5 Weak modifiers, 6 Vague quantifiers,
7 Ungrammatical expressions, 8 Open vs. Closed spelling, 9 Multiple
negation, 10 Misspelled expressions, 11 Misused words, 12 Jargon
expressions, 13 Pretentious words, 14 Archaic expressions, 15 Misspelled
foreign expressions, 16 Overused phrases, 17 Stock phrases, 18 Possible
word confusions, 19 Homonyms.

### W — word order (D)

One 16-bit word; one bit per choice; no bit = default. UI 10b0:0b28 /
10b0:0bfa; engine values 1330:0b4a.

| Setting (combo) | choices (UI strings) | bit → engine value |
|---|---|---|
| Split infinitives (25976) | Do not flag / Flag all / Flag two or more intervening tokens / three / four (2421–2425) | 0x01→0, 0x02→1, none→2 (default "two or more"), 0x08→3, 0x10→4 |
| Consecutive nouns (25977) | Do not flag noun adjuncts / Flag three / four / five or more nouns in a row (2426–2429) | 0x20→0, 0x40→3, none (0x80 written)→4, 0x100→5 |
| Prepositional phrases (25978) | Do not flag / Flag three / four / five or more in a row (2430–2433) | 0x200→0, 0x400→3, none (0x800 written)→4, 0x1000→5 |

The UI writes the default choice as an explicit bit (0x04, 0x80, 0x800);
the engine treats "no bit" as the default, so both read the same.

### What the built-in sets turn off (D, decoded from the masks)

| Set | grammar off | style off | split / nouns / preps |
|---|---|---|---|
| Business Writing | — | Wordy, Clichés, Weak modifiers, Vague quantifiers, Jargon expressions, Pretentious words | flag all / 4+ / 4+ |
| All Rules | — | — | flag all / 3+ / 3+ |
| Legal Writing | Passive verb errors | Pretentious, Archaic, Overused, Stock phrases | flag all / 4+ / 3+ |
| Technical Writing | — | Informal expressions, Stock phrases | 3+ intervening / 5+ / 5+ |
| Fiction Writing | Passive, Informal usage, Nonstandard expressions, Jargon words, Nonstandard modifiers | Inappropriate prepositions, Informal expressions, Clichés, Weak modifiers, Vague quantifiers, Open vs. Closed, Jargon expressions, Pretentious, Archaic | none / none / none |
| Academic Writing | Passive verb errors | Archaic, Stock phrases | flag all / 3+ / 3+ |
| Formal Writing | — | Pretentious, Archaic | flag all / none / none |
| Casual Writing | Passive, Style parameters, Informal usage, Jargon words, Format errors | Wordy, Redundant, Inappropriate prepositions, Informal expressions, Clichés, Weak modifiers, Vague quantifiers, Jargon expressions, Pretentious, Archaic, Overused, Stock phrases | 2+ intervening / 5+ / 5+ |

"Check quoted text" is on in every built-in set.

### How a set reaches the engine (D)

1330:0b4a, each session: `GINIT` with a parameter block (split, nouns,
preps values above; byte at +20 = 1 when "Check quoted text" is *off*,
i.e. skip quoted text) and the GCS path; `GCLRTOT`; then `GPROFILE(…,
0x20 = off, rule, 0)` for every unchecked grammar or style rule, and for
219. An unknown set name reads as an empty value → all masks 0 → every rule
off (I; the UI shows such a set with every box clear).

### The GCS file (D)

Path = lotus.ini `[Grammar Checker] Data Path` + `\LOTUS` + language code +
`1.GCS`; the DLL from `[Grammar Checker] Program Path` + `Ltsgrm1.dll`
(falling back to Ami Pro's own directory). Language code from the document's
dictionary language (`DAT_1680_21c8`): 1 or 0x0e → "AM", 2, 0x12, 0x18,
0x1b, 0x1c → "BR" (strings 1545/1546); any other language: Grammar Check
is refused (string 3221 "Grammar Check is not available for the current
language dictionary"). The GCS is the engine's own rule and dictionary data
and holds no named sets; it was not extracted.

### `[GramStyle]` (D, except where marked)

- Read by the SAM/STY reader (1058:06cc) into a document-global string at
  DGROUP 0x26fc; written back escaped by the SAM writer and the style-sheet
  writer (11c8:14e0 and the document writer, both through 11c8:0bf5).
- At session start (1330:004b) the set chosen in the start dialog is
  compared with it; if different it is copied in and a document flag is set
  (`DAT_1680_21b5 |= 4`, I: "changed", so the next save writes it).
- Preselection (callback 0x131, 1128): reads `[Grammar]` with a key
  Ghidra lost, and if the value is empty uses "Business Writing" (I: the
  key is the document's `[GramStyle]`, so a document remembers its set,
  and a blank or unknown `[GramStyle]` preselects Business Writing) — **V**.
- So a blank `[GramStyle]` means "never grammar-checked / no set chosen";
  it is not a set.

### 2. The check flow

### Start dialog (Grammar Check, 1092) (D)

Combo "Use grammar and style set" (INI order); checkboxes; OK sends 0x1fe
with the options word then the set name; Options… opens the Options dialog
with the current name and reloads the combo, selecting the name it returns.
Enabling (I for the meaning of the two callback codes): callback 0x16
non-zero → "Check from beginning" unchecked and disabled, "Include other
text streams" disabled (and checked if callback 0x41 is zero); 0x16 zero
and 0x41 non-zero → both disabled. Probable meanings: 0x16 = a selection
exists, 0x41 = the caret is not in the main text — **V**.

### Session start (1330:004b) (D)

1. `GrammarOpts` written to AMIPRO.INI (section lost in the decompile; I:
   `[AmiPro]`).
2. Engine loaded and configured (above).
3. "Check in draft mode": if on and the view is not Draft, switch to Draft,
   remember it, and switch back to Layout at the end (1330:01dd).
4. Range: if there is a selection → from its start to its end (end
   paragraph and offset kept; "other streams" stops at the end). Else, if
   "Check from beginning" → from the start of the current text stream, no
   end; else from the caret, no end.

### The step (0x200, 1330:04f2) (D)

The checker window pushes an action code:

| code | button | what Ami Pro does |
|---|---|---|
| 8 | (first step) | check from the current position |
| 1 | Skip | if another problem of this sentence has not been shown (1330:1ba6), stop there (the window asks for the texts again); else go to the next sentence |
| 2 | Skip Rule | `GPROFILE` off for the current problem's rule — for the rest of this session only, the set is not changed — then go to the next sentence |
| 0 | Next Sentence | go to the next sentence |
| 4 | Replace (+ list index) | apply the replacement (1330:2081), then **re-check the same sentence** (no advance) |
| any, with a non-zero restart flag | Resume | restart from the caret (see below) |

"Go to the next sentence" = `GADDTOT` (the passed sentence joins the
statistics), advance the offset by the sentence length the engine
reported, and at the paragraph's end move to the next paragraph of the
stream (1070:443e).

Loop: if an end is set and this is its paragraph, stop when only spaces or
codes remain before the end (1330:264e). At the end of a stream: if
"Include other text streams" → the next stream (1348:0fec), else finish.
For each position: copy the paragraph's text (1330:02fe), let the window
pump messages and check for Cancel, call `GCS` on the text from the
offset to the paragraph's end. `GCS` reports one sentence: its length and
a list of problems. No problem → add to totals and continue; a problem →
select the sentence in the document (1330:0463: from the sentence's start
to start + length, clipped to the paragraph) and return to the window.

**Unit: the sentence, as the engine splits the paragraph text** (D).
Paragraphs are never joined; a sentence never crosses a paragraph.

Text given to the engine (1330:02fe, D): the paragraph's characters, with
every embedded code (bytes < 0x20, each counted at its code length) and
every revision-marked deletion (ESC V S- … ESC V E-) replaced by 0x01 bytes
of the same length, so offsets map 1:1 back to the document.

Stream order (1348:0fec, shared proofing code; I for names): six kinds
visited cyclically from the starting one: 1 main text, 2 and 4 two frame
chains (frames whose text is checkable), 3 a stream at `DAT_1680_2225`
(I: footnotes), 5 a linked list at `DAT_1680_221f` (I: notes), 6 a list
from `DAT_1680_2342` (I: tables); when the cycle returns to the starting
kind, the position goes to that stream's start. Exact order and whether a
check from the caret wraps to the start — **V**.

### The Grammar Checker window (3039, 10b0:134c) (D)

Modeless popup (stays open while the user edits). Controls: Suggestions
(25775, an Ami text control with highlight markers), Replacement options
list (25350), Skip (25585, default), Skip Rule (25588), Next Sentence
(25587), Replace (25586), Cancel.

After each step that found a problem:

- **Suggestions** (0x202, 1330:080f + 1330:17fd): "1. " + the problem's
  message. Linked problems (a ring through record +8) are joined as
  "… and " + the next message in lower case (the previous message's last
  character dropped). An alternative group (record +0x10) adds a line
  "OR" and "2. " + its message, and so on. Text in `[ ]` or `{ }` is shown
  highlighted (the brackets become highlight markers). If "Show
  explanations" is on, a rule of underscores (string 1428) and a blank line,
  then the rule's explanation from `GGETLEV3` (keyed by the rule and two
  explanation IDs of the record).
- **Replacement options** (0x203, 1330:1c17 + 1330:22ac): one item per
  alternative, "1. ", "2. ", …: the **whole sentence rebuilt from the
  engine's tokens** (≤ 55 tokens, single spaces, a space before the final
  punctuation removed), with the replacement text from the message's
  `[…]` spliced in highlighted; where a message has no `[…]`, the problem
  tokens themselves are highlighted.
- Selecting an item (0x204, 10b0:1825): Replace is enabled only when that
  item carries a real replacement; the list's label reads "Replacement
  options:" then, else "Sentence:" (strings 2500 / 2501). Double-click =
  Replace when enabled.
- The document shows the whole sentence selected (1330:0463).

When the window loses activation (the user clicks into the document):
Skip is relabelled **Resume** (string 2503) and Replace, Next Sentence,
Skip Rule and the list are disabled. On reactivation a restart flag is set;
Resume restarts checking from the current caret and the button reads Skip
again (strings 2502/2503).

**Replace** (1330:2081, D): for each linked problem of the chosen
alternative, delete the problem's token range (token offset and length
from the result's token table, shifted by earlier replacements in the same
sentence) and insert the bracketed text through Ami Pro's normal text
insert (1070:350d), which applies **revision marking** when it is on. The
pending end offset is adjusted by the length change. Undo: the same insert
path as typing; whether one Replace is one undo step — **V**.

Finish: when the step reports the end, the window marks the session
complete and closes; on close, if complete **and** "Show readability
statistics" is on, it takes the statistics (callback 0x135) before ending
the session (0x201: restores the view, frees the engine) and opens the
Readability Statistics dialog (0x205). Cancel ends without statistics.

### 3. Options stored (D)

AMIPRO.INI key **`GrammarOpts`** (AMIPRO.EXE string 1387), an integer
written at every session start; default when missing **13 (0x0D)**
(callback 0x12e):

| bit | checkbox (start dialog) |
|---|---|
| 0x01 | Show readability statistics |
| 0x02 | Check from beginning of document |
| 0x04 | Include other text streams |
| 0x08 | Show explanations |
| 0x10 | Check in draft mode |

Default 0x0D = statistics, other streams, explanations on; from beginning
and draft mode off.

Options dialog (10b0:03ce…): set combo; three pages by radio button;
each page's "Explanation:" box shows a help line for the focused control
(UI strings 2450–2469 grammar, 2470–2489 style, 2490–2492 word order).
OK saves the set if anything changed (no prompt); switching sets with
changes asks "%s has changed. Save changes?" (2434); Save writes the
current set; Save As… asks a name (≤ 24 characters, trailing spaces
trimmed, OK disabled when empty), asks "%s already exists. Do you want to
replace it?" (2435) for an existing name, writes it and selects it. There is
no Delete; Cancel discards.

### 4. Readability statistics

All figures come from the engine (D); Ami Pro only copies them
(1330:24ea) and the dialog formats them (10b0:1c71).

### Counts (LTSGRM1.DLL, D unless marked)

Per sentence (1100:0ca0, after `GCS`) and per word (1070:08d2), added to
document totals by `GADDTOT` (1040:0a0e), cleared by `GCLRTOT`
(1050:0f5a):

| Figure | how |
|---|---|
| Words | word tokens of the sentence (punctuation tokens excluded) |
| Sentences | one per `GCS` sentence |
| Paragraphs | 1 when the sentence starts a paragraph |
| Passive sentences | the parser's passive flag for the sentence |
| Syllables | per word, see below |
| 3-syllable words | words of ≥ 3 syllables, except proper nouns and words that reach 3 only by an -es/-ed ending (the root re-counted < 3) — Gunning's exceptions |
| readability words / sentences | a second pair used by the formulas: the same words, but sentences also counted at internal clause punctuation tokens (two engine strings, I: ";" and ":") that are not the last token |

Syllables (1070:08d2 + 11c8:0000): the word is lower-cased and split at
hyphens; each part counts at least 1. A one-letter part = 1. An all-capital
part (acronym) and ordinary words go through a rule-based English
syllabifier (11c8:0000: a chain of rewrite passes over prefixes/suffixes
and vowel clusters, then the remaining vowel letters a e i o u plus marked
vowel groups are counted). Parts made only of lower-case letters and
apostrophes, periods, hyphens, "s", "e", "d" are counted by vowel letters,
and when that gives < 2, by letters (abbreviations). Numbers are not words
for syllables. The exact rewrite rules are the engine's (about 5.7 KB of
code in segment 11c8) — reimplement with any good English syllable
counter; **V** against Ami Pro for a sample.

### Formulas (1100:01b2, GDOCSTAT; D: constants read from the DLL's data
segment, helper semantics I)

W = readability words, S = readability sentences, Y = syllables,
C = 3-syllable words.

- **Flesch Reading Ease** = 206.835 − 0.846 × (100·Y/W) − 1.015 × (W/S),
  raised to 0 if negative (no upper clamp); 0 unless W, S, Y > 0.
- **Flesch Reading Ease grade level** (a lookup from the score, linear
  within bands):
  - score < 30 → 17
  - 30 ≤ score < 50 → from 16 (at 30) to 13 (at 50)
  - 50 ≤ score < 60 → from 13 to 10
  - 60 ≤ score < 70 → from 9 to 8
  - 70 ≤ score ≤ 100 → from 8 to 5
  - score > 100 → 0
- **Flesch-Kincaid** = 0.39 × (W/S) + 11.8 × (Y/W) − 15.59, raised to 0 if
  negative; 0 unless W, S, Y > 0.
- **Gunning's Fog Index** = 0.4 × (W/S + 100·C/W); 0 unless W, S > 0.

The engine also sets "reliable" flags (≥ 100 words for Flesch, ≥ 200 for
Flesch-Kincaid, ≥ 100 for Fog); Ami Pro does not copy or show them.

### The dialog (D for mapping, V for number formats)

| Field | source |
|---|---|
| Words, Sentences, Paragraphs, Syllables, 3-syllable words | the display totals, as integers |
| Words per sentence | words ÷ sentences (sentences 0 → 1) |
| Sentences per Paragraph | sentences ÷ paragraphs (0 → 1) |
| Passive sentences | passive ÷ sentences as a percentage, "0%" when none |
| Gunning's Fog, Flesch-Kincaid Score, Flesch Reading Ease Score, Flesch Reading Ease Grade Level | the four engine floats |

The averages and indices are formatted by 10b0:1f99 (fixed-point via
`fcvt`, padded to three integer places); decimals — **V**.

Shown only at the end of a completed check with the option on; it covers
the sentences passed in that session (from the start position to the end),
not the whole document unless checked from the beginning.

### 5. Other points for a faithful reimplementation

- One session at a time (the session block is a single global); the
  checker is modeless and the user may edit while it is open; Resume
  continues from the caret.
- A Replace re-checks the same sentence; Skip walks the remaining problems
  of the sentence; Skip Rule and Next Sentence move on.
- Statistics count each sentence once, when it is passed (`GADDTOT` at the
  advance), so re-checks after Replace are not double-counted.
- Revision-marked deleted text is invisible to the checker; replacements
  are revision-marked when marking is on.
- Grammar Check is limited to US and UK English documents.
- Rule explanations and messages are the engine's data (GCS) and
  proprietary; a reimplementation needs its own rules, messages and
  explanations. The 40 rule names, the help lines (UI strings 2450–2492),
  the eight set names and their masks are the interface's data and define
  what the options must offer.
- Error codes of `GINIT`/`GTERM` map to "Cannot load the grammar program."
  (3207), "not available for the current language dictionary" (3221) and
  out of memory (3000).

## What the VM settled (2026-10-26)

- **Alternatives' capitals** (spelling): the engine gives them in the
  flagged word's capitals ("Teh" → The…, "teh" → the…, "NASAA" → NASA…). V
- **Replace All is exact-case**: a later "Teh" was replaced silently,
  "teh" was asked again. V
- **Undo** after a check restores only the last replacement (Ami Pro's
  single undo level); the spell and grammar code make no undo record of
  their own. V
- **Repeated words**: a message box first; an empty Replace with deletes
  the word and the space before it. V
- **`<:s>`**: written on a paragraph a check passed clean (PSP4B), not on
  the last paragraph of the text, not after a selection check (PSP2). V
- **The user dictionary** for American English is
  `C:\WINDOWS\LOTUSAPP\SPELL\LTSUSER1.DIC`, `word\r\n` per word. V
- **`[lang]`**: British is 2 (PLN1); the Language Options lists show only
  installed languages, sorted by name. V
- **Thesaurus**: inflected synonyms ("sat" → posed, postured), the word's
  capitals ("Another" → Like, Second…), the dialog stays open after
  Replace with the new word selected. V
- **Grammar**: the defaults (Business Writing; statistics, explanations,
  other streams on), the eight sets, `[GramStyle]` written with the set
  used, Replace re-checking the sentence, statistics only at a completed
  check. V
- **Readability formulas**: PGR1's 20 words, 3 sentences, 25 syllables, 0
  three-syllable words give Fog 0.4 × 6.67 = 2.7, Flesch-Kincaid 0.39 ×
  6.67 + 11.8 × 1.25 − 15.59 = 1.8, Reading Ease 206.835 − 84.6 × 1.25 −
  1.015 × 6.67 = 94.3, and the grade table's 70–100 band 8 − 3 × (94.3 −
  70) / 30 = 5.6, as Ami Pro shows. V + D
