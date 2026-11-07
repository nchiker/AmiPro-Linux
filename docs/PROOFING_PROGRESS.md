# Proofing Progress

Live state of `docs/PROOFING_RUNBOOK.md` (tag `proofing-run-start`).
Resume at the first slice that is not `DONE`.

| Slice | State | Commit | Evidence | Notes |
|---|---|---|---|---|
| PF.1 Setup | DONE | (the setup commit) | `PROOFING_NOTES.md`; `UI_SPEC.md` §35; SP1–SP8 in `DECISIONS.md`; the baseline below | Tag `proofing-run-start` |
| PF.2 Model and SAM | DONE | (this commit) | `amipro_document::language` (Ami Pro's 28 numbers, names, locales; `Language` with a locale Ami Pro has no number for), `Document::language`, `grammar_set`; native v3 `language`, `grammarset`; `take_proofing` at import, `[lang]` and `[GramStyle]` written from the model (SAM and sheets); `LossKind::Language`; `Command::SetLanguage`, `SetGrammarSet`; `<:s>` dropped from a paragraph whose text a command changes (`drop_checked_flags`); `tests/proofing_sam.rs`. Round trip: user's corpus 417 of 417, ISO 159 of 160, VM folder 341 of 341 (PSP2–PSP4B added); layout comparison unchanged (`target/proofing-baseline/pf2/`) | SK1: a locale without an Ami Pro number writes the first number of its language, else American |
| PF.3 The check in the core | DONE | (this commit) | `amipro_editing::proofing::spelling`: `Speller`, `SpellOptions` (Ami Pro's `SpellOpts` bits), `words` (tokens without the punctuation around them), `paragraph_problems` (one-letter words never, numbers, initial caps, repeats from the space before), `check_streams`, `next_problem` (from the caret's word, no wrap, a selection's end), `Ignored` (Skip All), `in_capitals_of`, `alternatives` (ten), `replace_all` (SP4: each occurrence's capitals, one step), `read_word_list`/`write_word_list` (Ami Pro's `LTSUSER1.DIC`); `tests/spelling.rs` (PSP1's walk, a selection, Skip All, Replace All) | SK2: web and e-mail addresses and tokens without a letter are no words |
| PF.4 The spelling engine | DONE | (this commit) | `spelling.rs`: `spellbook` 0.4.2 (MPL-2.0); dictionaries found in `~/.local/share/hunspell`, `/usr/share/hunspell`, `/usr/share/myspell` and the bundled en-US (`app/cosmic/dictionaries/`, SCOWL, with its notice), by locale and then by language; UTF-8 and Latin-1 files; loaded on a worker thread and kept; `UserDictionary` (Ami Pro's format, `$XDG_DATA_HOME/amipro-linux/dictionaries/LOCALE.dic`, written through a temporary file; none in tests); `SessionSpeller` (user words, user dictionary alternatives); tests with a small dictionary written for them | |
| PF.5 Spell Check | DONE | (this commit) | `proofing_panel.rs`, `proofing_commands.rs`: Tools › Spell Check… (F7), Thesaurus… (Shift+F7) and Language… open the Proofing panel (Spelling, Thesaurus, Grammar pages); the word selected and scrolled to, alternatives in its capitals, Replace (Enter, or the chosen alternative pressed again), Replace All (one step, the count), Skip, Skip All, Add to Dictionary, Delete Repeat, "Spell check complete."; a selection checked alone; Ami Pro's six options (the settings' `spell_options`, Ami Pro's bits); the language drop-down; the user dictionary's list, Add, Remove, Import…, Export…; tests `spell_check_finds_replaces_skips_and_adds_words_one_undo_step_each`, `spell_check_of_a_selection_checks_only_it_and_options_are_ami_pros`, `the_document_language_is_set_in_one_step_and_a_missing_dictionary_is_named`, `f7_opens_spell_check_and_the_panels_buttons_take_clicks` (window); screenshots light and dark (`proofing_screenshots`) | The status bar already counted a selection's words ("1 of 17 words") |
| PF.6 As you type | DONE | (this commit) | After every message a changed document starts a 350 ms pause (`sync_spelling`); its `Idle` checks every stream's paragraphs, each text checked once (`spell_cache`, by the text's hash), and the page surface draws a red wavy line under the words (`underlines`, never printed); Tools › Check Spelling as You Type (a check, saved); the right-click menu on an underlined word: its suggestions, Delete Repeated Word, Ignore All, Add to Dictionary, Spell Check…; tests `words_are_underlined_as_you_type_after_a_pause_and_not_when_off`, `the_right_click_menu_offers_an_underlined_words_suggestions` | Without any dictionary nothing is scheduled (the tests' window runs every task to its end); the typing A/B is measured in PF.10 |
| PF.7 Language | DONE | (this commit) | Tools › Language… opens the panel at its options; the language drop-down (every installed dictionary, the document's own first when it has none) sets the document's language in one undo step; Use for new documents (the settings' `default_language`, Ami Pro's `dictlang`), taken by File › New; hyphenation follows the language: `ShapeRequest::language`, `ShapeCache::use_language`, `hyphenation_language` (English for Ami Pro's 0), hypher's full set of patterns in the app; tests `the_document_language_is_set_in_one_step_and_a_missing_dictionary_is_named`, `new_documents_take_the_default_language`, `hyphenation_follows_the_documents_language`; round trip and layout comparison unchanged (`target/proofing-baseline/pf7/`: every corpus document is American or none) | |
| PF.8 Thesaurus | DONE | (this commit) | `amipro_editing::proofing::thesaurus`: the MyThes reader (`Index`, UTF-8 and Latin-1), Ami Pro's word at the caret (`word_at`, `1380:0000`), base forms (regular suffixes and irregular verbs and comparisons) with the synonyms re-inflected ("sat": posed, modeled), the word's capitals; the app finds the system's thesauri (`~/.local/share/mythes`, `/usr/share/mythes`, `/usr/share/myspell/dicts`), loads the index on a worker; the panel's Thesaurus page (word looked up, Replace with, Look Up, Meanings, Synonyms, Replace (a synonym pressed twice too), Previous, Look Up the Word at the Caret; the panel stays after Replace, the new word selected); the right-click Synonyms ▸ on any word; tests in the core and `the_thesaurus_looks_up_the_caret_word_and_replaces_it_in_one_step`, `the_right_click_menu_offers_synonyms_of_any_word` | SK3: MyThes has no definitions, so a meaning shows its part of speech and first synonyms where Ami Pro showed a definition |
| PF.9 Grammar | DONE | (this commit) | `harper-core` 2.13 (Apache-2.0; default features off, `concurrent`) in `grammar.rs`: Ami Pro's eight sets mapped onto Harper's categories (`checks`; spelling left to Spell Check), the app's own explanations, made once on a worker thread; Tools › Grammar Check… and Readability Statistics… (Tools menu); the panel's Grammar page: the set (`[GramStyle]` recorded at the start, one undo step when it changes), the problem selected on the page, the message, the explanation, the sentence rewritten as replacement options, Replace (the sentence checked again), Skip, Skip Rule, Next Sentence, the end with Ami Pro's Readability Statistics; options as `GrammarOpts`' bits (default 13); grammar underlines in blue (off by default); `amipro_editing::proofing::readability` (Ami Pro's formulas and grade table, the project's own counts; PGR1's figures reproduced); tests `grammar_check_finds_replaces_and_ends_with_ami_pros_statistics`, `grammar_is_english_only_and_underlines_when_asked`, `harper_finds_ami_pros_kind_of_problem_in_its_sets`; screenshots of the three pages | SK4: Harper's part-of-speech tagger brings `burn`'s CPU tensor backend (no GPU code built); the debug build's first grammar check takes seconds to make Harper's rules, a release one a fraction (PF.10). SK5: a problem selects its words (Ami Pro selected the sentence); the sentence shows in the options |
| PF.10 Measures | DONE | `cec8f1d` | Round trip: user's corpus 417 of 417, ISO 159 of 160, VM folder 341 of 341 (as at the start, with PSP2–PSP4B); layout comparison as at the start (`target/proofing-baseline/pf10/`). Release (`proofing_performance_baseline`, README31): the en-US dictionary 16 ms to load; a whole Spell Check 200 words in 6.1 s (about 30 ms a step, mostly suggestions); as you type 3.3 ms cold, 0.14 ms after a keystroke; a keystroke with underlines shown 1.27 ms; Harper made in 436 ms, about 10 ms a paragraph (3.5 s for README31's main text); the thesaurus index 23 ms, a lookup 0.5 ms; the user's corpus (417 documents, 75,724 paragraphs) checked for spelling and grammar without a panic in 105 s. Typing A/B against `proofing-run-start` (`tools/bench-ab.sh`, 3 rounds): README31 typing medians 1.00–1.28 ms against 0.91–1.45 ms | The benchmark found the walk going round once a check reached another stream (fixed in `cec8f1d` with a regression test); grammar underlines moved to a worker thread (3.5 s on the interface's thread before) |
| PF.11 Twins | DONE | (this commit) | `proofing_twins` (ignored): APF1 = PSP4B with "Teh " typed into its flagged paragraph, a word replaced by Spell Check, British, All Rules, by Save As SAM; in Ami Pro 3.1 Spell Check stops at "Teh" (the flag is gone), Language Options shows British for the document, Grammar Check offers All Rules | VM screenshots outside Git |
| PF.12 Exit | DONE | (the handoff commit) | `tools/verify.sh --release --corpus`: core 531 + 12 ignored; app 387, 116, 394 (61, 6, 61 ignored); strict Clippy, fmt, release builds, ISO corpus gate PASS. Records: `UI_SPEC.md` §35 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `REFERENCE_FEATURE_MATRIX.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue (run 11 DONE, run 12 NEXT), `SESSION_HANDOFF.md` | Physical check passed 2026-11-03; Harper kept (SK4) |

## Baseline (at `proofing-run-start`, `target/proofing-baseline/start/`)

Measured with `target/proofing-baseline/baseline.sh` (outside Git), with
the run's VM fixtures in the folder:

- Round trip: user's corpus 417 of 417 read back equal (Save As would list
  nothing 417), ISO 159 of 160, VM folder 338 of 338.
- Layout comparison: user's corpus page counts equal 318 of 394,
  paragraph line counts 14,938 of 15,356, page ends on the same page 782
  of 1,108; ISO 70 of 160, 22,789 of 24,971, 478 of 1,307; VM folder 337
  of 337, 2,197 of 2,209, 118 of 128.

## Choices made in the run (SK…, also in `DECISIONS.md`)

- **SK1** A language Ami Pro has no number for is written to `[lang]` as
  the first Ami Pro language of the same language (fr_BE → 3 French),
  else 1 American, so Ami Pro still checks the document; Save As lists it.
- **SK2** Web addresses, e-mail addresses, and tokens without a letter
  are not checked (Ami Pro's engine flagged them).
- **SK3** The thesaurus's meanings show their part of speech and first
  synonyms: MyThes thesauri have no definitions, which Ami Pro's Meaning
  box showed.
- **SK4** Harper (SP8) depends on `harper-brill`, whose part-of-speech
  tagger brings `burn` with its CPU (`ndarray`) backend; nothing for GPUs
  is built. Its cost is measured in PF.10.
- **SK5** A grammar problem selects its own words on the page; Ami Pro
  selected the whole sentence. The sentence shows, rewritten, in the
  replacement options.

## Log

### 2026-11-03, after the master documents run: three fixes from use

The user found spelling as you type underlining nothing on the desktop,
then a suggestion chosen from the right-click menu replacing nothing,
and the suggestions shown only in a maximised window.

- A stale pause (one arriving after newer typing) cleared the tab's
  checked root, so the next message scheduled another pause and none was
  ever current (the trace showed 1,748 pauses, no underline). It now does
  nothing. `AMIPRO_TRACE_SPELL=FILE` logs each step of the check. Test
  `a_stale_pause_schedules_no_other_and_the_current_one_underlines`.
- The right press published `ContextAt` before moving the caret, so the
  menu's word was the one at the old caret. It now comes after the move.
  Test `a_suggestion_chosen_by_right_click_replaces_the_word_under_the_pointer`,
  through the whole window.
- libcosmic opens a menu taller than the room below the pointer upwards,
  and its top (the suggestions) went above a window not maximised. On an
  underlined word with nothing selected the menu holds only the spelling
  entries (five suggestions at most). The user's recheck passed.

The harnesses missed the first two: `send` runs each task to its end
before the next message, so no pause is stale, and tests sent `ContextAt`
after placing the caret themselves.
