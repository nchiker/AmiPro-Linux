# Proofing Playbook

Part of `docs/PHASE5_PLAYBOOK.md` (queue rows 3 and 11). Follow its recipe
and standing rules; this file says what each run covers, what to map, and
what to expect.

**Why one playbook.** Find & Replace, spell check, grammar check, and the
thesaurus share three things:
- They walk every text stream of a document in Ami Pro's order: main
  text, Where placed frames, footnotes, anchored frames and page tables,
  floating headers and footers, then fixed headers, footers, and
  repeating frames; notes and Draw frames are not searched
  (`SAM_FORMAT_NOTES.md`, "Text streams").
- They show one match at a time on the page.
- They change text through commands, with one undo step per change and
  one for a Replace All.

Find & Replace is built first, and the language tools reuse its walk and
its review interface.

**Superseded (2026-11-07):** find and replace and proofing are built (runs 3 and 11). **State then.** None of these is built. `ED-02` (Find and replace, P0)
and `PR-01` (spellcheck, language tools) are UNVERIFIED. The registry
has Find…, Find Next, Find Previous, Spelling…, Check Spelling as You
Type, Thesaurus…, and Language… as Future commands. Hyphenation already
uses outside language data (`hypher`, TeX's patterns): the precedent for
dictionaries.

**Evidence to start from.**
- **Guide:** ch. 20 covers Find & Replace (text, paragraph styles,
  deleting text, tabs, and returns, wildcards and literals). Ch. 12 covers
  Spell Check (repeated words, the user dictionary, Language Options) and
  Grammar Check (rule sets). Ch. 25 covers the Thesaurus.
- **Commands:** Edit › Find & Replace… (0x1c3, Ctrl+F), Tools › Spell
  Check… (0x1c5), Grammar Check… (0x1fb), Thesaurus… (0x1c7).
- **Modules:** the proofing engines and their dictionaries (to identify
  in `../AmiPro-Reference/Binaries`). Their word lists are proprietary:
  map only the behaviour and the files' formats, never copy them.
- **Storage:** `[lang]` (the language number, in every file) and
  `[GramStyle]` (the grammar rule set; set in 20 user and 4 ISO files).
  The user dictionary's file and format are to map.

## Run 1: `find`, queue 3 (small)

**Goal.** Edit › Find & Replace… as Ami Pro has it:
- **What it finds:** text, or a paragraph style.
- **Options:** whole word, exact case, and including the other text
  streams.
- **Where and which way:** from the caret or the beginning, forward or
  backward.
- **Replacing:** Find, Replace, Replace All, and Replace and Find.
- **Patterns:** Ami Pro's wildcards and literals, and its codes for tabs
  and returns.

**Map.** The dialog (`AMIPROUI.DLL`), the order it walks the streams in,
the wildcard rules and codes, what Replace keeps of the found text's
formatting, and where the caret and view go after each step.

**Decisions to expect.**
- **The form.** Recommend a find bar over the pages, non-modal, with Ami
  Pro's options on it. Ctrl+F opens it; Enter and Shift+Enter find next
  and previous; the document stays live.
- **Highlighting.** Recommend highlighting every match on the visible
  pages.
- **Replace All.** Recommend one undo step, with the number replaced in
  the status bar.

**Better ways and logical additions to offer.**
- Live match count.
- Find in the selection.
- Find and replace character formatting.
- Regular expressions as an option beside Ami Pro's wildcards.
- History of recent searches.

**Measures.** Find moves no text. Replace is measured on README31 and the
longest corpus file: time to the first match, Replace All, and its undo.

**Exit.** Message-path and widget-path tests of every option; a corpus
sweep that finds every paragraph's text in each stream; the user's
physical check.

**Done 2026-10-20** (`docs/FIND_PROGRESS.md`). For run 2: the walk is
`amipro_editing::find::streams` (Ami Pro's order; covered cells left out)
and `search_streams`; matches drawn on the page through
`page_surface::Highlights` (the find bar's yellow; a wavy underline would
be a second style there); the find bar's one-root menus and its focus
handling (`find_bar.rs`, `UI_SPEC.md` §27 notes) are the pattern for a
spelling bar if the run chooses one.

## Run 2: `proofing`, queue 11

**Goal.**
- Tools › Spell Check… with Ami Pro's options (repeated words, the user
  dictionary, Language Options), plus spelling as you type.
- Tools › Thesaurus….
- Tools › Grammar Check… with rule sets.
- The document's and a paragraph's language (`[lang]`).

**Map.** Each dialog and its options, the user dictionary's format (to
read the user's own if they have one), how `[lang]` and a style's
language are stored, and what `[GramStyle]` holds.

**Libraries to evaluate** (licences to confirm in the run):
- **Spelling:** `spellbook`, a pure-Rust Hunspell-compatible checker
  (MPL-2.0), reading Hunspell dictionaries.
- **Thesaurus:** a MyThes-format thesaurus.
- **Grammar:** Harper (`harper-core`, Apache-2.0, offline), or none.

**Decisions to expect.**
- **Which dictionaries.** Recommend the Hunspell dictionaries installed
  on the system (`/usr/share/hunspell`), so none are bundled, with a
  prompt when none is installed.
- **Spelling as you type.** Recommend on, with a wavy underline and the
  suggestions in the right-click menu; Ami Pro's batch Spell Check stays.
- **Grammar.** Recommend Harper, if its English-only scope is acceptable,
  with Ami Pro's rule sets mapped onto its options where they match.
- **The user dictionary.** Recommend one per user, in the settings, with
  an import of Ami Pro's.

**Better ways and logical additions to offer.**
- Ignore All for the session.
- Add to dictionary from the right-click menu.
- A word count of the selection (the status bar has the document's).
- Readability statistics if the grammar checker gives them, which Ami
  Pro's grammar checker showed.

**Measures.** Checking as you type stays off the typing path: check after
an idle pause, by paragraph, and keep its cost out of the typing
benchmark (A/B). Time a full Spell Check of the longest corpus file.

**Exit.** Tests with a small free dictionary fixture in the repository;
the corpus's text checked without a crash; the user's physical check.

**Done 2026-10-26** (`docs/PROOFING_PROGRESS.md`; tag `proofing-run-start`).
For later runs: the core's `amipro_editing::proofing` (spelling's walk,
the thesaurus reader, readability) is engine-free; the app's `spelling.rs`
finds Hunspell dictionaries and MyThes thesauri, `grammar.rs` wraps
Harper. A check walks from the stream it is in (`next_problem`); grammar
work goes to a worker thread. The TOC and index run can reuse the walk.
