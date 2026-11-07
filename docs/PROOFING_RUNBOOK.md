# Proofing Runbook

**Written:** 2026-10-26, the Phase 5 queue's run 11
(`docs/PHASE5_PLAYBOOK.md`, `docs/playbooks/PROOFING.md`, "Run 2:
`proofing`"). **Purpose:** carry spelling, spelling as you type, the user
dictionary, the document's language, the thesaurus, grammar, and
readability statistics from start to finish as a straight run. Its
decisions were asked before the run and answered on 2026-10-26
(`docs/research/DECISIONS.md`, "Proofing: the user's answers", SP1–SP8);
the run does not stop to ask.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the
decisions, `docs/Design/UI_SPEC.md` §35 the interface. Evidence:
`docs/research/PROOFING_NOTES.md` (the decompiled `AMIPRO.EXE`,
`AMIPROUI.DLL`, `LTSSPL1.DLL`, `LTSTHES1.DLL`, `LTSGRM1.DLL`; Ami Pro 3.1
fixtures PSP1–PSP4B, PGR1, PLN1), with `FIND_NOTES.md` (text streams).

**Live state** is `docs/PROOFING_PROGRESS.md`.

**Goal.** Tools › Spell Check… (F7), Thesaurus… (Shift+F7), Grammar
Check…, Readability Statistics…, Language…, and Check Spelling as You Type
work as Ami Pro's commands with the approved improvements, in one Proofing
panel in the rail; unknown words are underlined as you type; the user
dictionary is the user's, in Ami Pro's format; the document's language is
`[lang]`, the grammar set `[GramStyle]`, and paragraphs the app changes
lose Ami Pro's checked flag `<:s>`; every change is one undo step; Ami
Pro opens the app's files as its own (twins).

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`,
   `PROJECT_PLAN.md`, this runbook, `docs/PROOFING_PROGRESS.md`,
   `docs/research/PROOFING_NOTES.md`, and `docs/Design/UI_SPEC.md` §35;
   then `git status` and `git log --oneline -15`.
2. The tag `proofing-run-start` marks the start; never move it.
3. Resume at the first slice that is not `DONE`. Uncommitted changes
   belong to an interrupted run: inspect, verify, finish; never discard.

## 2. Decisions

- **SP1** The system's Hunspell dictionaries through `spellbook`, plus a
  bundled free en-US dictionary (SCOWL, its notice shipped); a missing
  language names its package.
- **SP2** Spelling as you type on by default (saved): red wavy underlines,
  never printed, checked by paragraph after a pause off the typing path;
  right-click suggestions, Ignore All, Add to Dictionary, Spell Check….
- **SP3** One Proofing panel in the rail: Spelling, Thesaurus, Grammar
  pages; the document live; the current word or sentence highlighted; F7,
  Shift+F7.
- **SP4** Each Replace one undo step; Replace All every occurrence in the
  range at once, keeping each one's capitals, one step with a count; Skip
  All for the session in that document, shared with as you type.
- **SP5** One user dictionary per user and language, Ami Pro's format, in
  the app's data folder; listed in the panel; Import… and Export….
- **SP6** One language per document in `[lang]`, any installed; Tools ›
  Language… one undo step; default for new documents in the settings;
  languages without an Ami Pro number native only, listed by Save As SAM;
  hyphenation follows the language where patterns exist.
- **SP7** System MyThes thesauri through the project's own reader; the
  Thesaurus page as Ami Pro's dialog; right-click Synonyms ▸.
- **SP8** Harper for grammar, Ami Pro's eight sets mapped, `[GramStyle]`;
  readability by Ami Pro's formulas, also from Tools › Readability
  Statistics…; blue underlines as you type offered, off by default.
- **Choices made in the run** are labelled SK1, SK2, … in the progress
  file and `DECISIONS.md`, each Ami Pro's behaviour where it has one.
- **Earlier decisions that apply:** rail inspectors over modal dialogs;
  consistent conventions (check marks on every on/off item); the find
  bar's streams and highlights (F1–F4); SAM sources protected; Save As
  SAM lists what Ami Pro cannot hold.

## 3. Working rules

- **The core stays dependency-free.** Words, the check's walk, options,
  capitals, the user dictionary's format, the language table, the MyThes
  reader, inflection, and readability live in `amipro-editing` behind
  traits (`Speller`, as `Shaper` and hyphenation are); `spellbook` and
  `harper-core` live in the app.
- **Ami Pro's bytes are the oracle.** Files Ami Pro wrote read back equal
  (the round trip); `[lang]`, `[GramStyle]`, and `<:s>` as Ami Pro writes
  them.
- **Every change is one `amipro_editing::Command`**, applied to a new
  root; tested through the app's message path and the widget path.
- **Checking stays off the typing path**: as-you-type checks run after a
  pause, by paragraph, cached by the paragraph's text; the typing
  benchmark's A/B is equal to the start.
- The evidence first: a rule not in the notes is looked up in the
  decompile or made as a VM fixture before it is built.
- The pinned libcosmic's traps (`CLAUDE.md`); **tests click portal
  commands with `Runtime::click_unrun`** so no real dialog opens on the
  user's screen; tests never touch the user's settings or user
  dictionary (a scratch folder).
- Small green commits; routine suites, Clippy, fmt, and corpus measures
  through lower-cost subagents or background scripts that report back.
- Aggregate numbers only, never the corpora's file names or contents;
  never Lotus's lexicons, thesaurus, or rule data.
- **Text that moves is measured** (`target/proofing-baseline/baseline.sh`)
  at the start, the exit, and around any slice touching the model, SAM, or
  layout (hyphenation by language can move text).

## 4. Stop rules

None planned. A problem gets 90 minutes, then it is a blocker in the
progress file and the run moves on to the next independent slice. A
question found during the run takes the choice that keeps Ami Pro's
behaviour, is recorded (SK…), and goes into the handoff. If the run grows
past one long session, hand off at a slice boundary with the progress file
current.

## 5. Slices

| Slice | What | Gate |
|---|---|---|
| PF.1 Setup | `PROOFING_NOTES.md`, `UI_SPEC.md` §35, SP1–SP8, this runbook, the progress file, the baseline, the tag | Committed; `proofing-run-start` |
| PF.2 Model and SAM | The document's language (Ami Pro's number, a locale for others) and grammar set typed, native v3, SAM; `Command::SetLanguage`; `<:s>` dropped from a paragraph whose text changes | Round trip equal to the start; Save As lists nothing new; a changed paragraph loses `<:s>` |
| PF.3 The check in the core | `amipro_editing::proofing`: words (Ami Pro's tokens), options, the walk through `find::streams` from a position or a selection, repeated words, Skip All and ignore sets, capitals of a replacement, Replace All as `Command::Several`, the user dictionary's format, the language table | Unit tests with a fixture speller; the corpus walked without a panic |
| PF.4 The spelling engine | `spellbook` in the app; dictionaries found (system, user, the bundled en-US), loaded off the interface's thread; the user dictionary stored and imported | Tests with a small dictionary fixture in the repository |
| PF.5 Spell Check | The Proofing panel's Spelling page, Tools › Spell Check… (F7), options, Replace, Replace All, Skip, Skip All, Add to Dictionary, Delete Repeat, highlights, the language and user dictionary sections | Message-path and widget-path tests; screenshots light and dark |
| PF.6 As you type | The idle check by paragraph, wavy underlines (never printed), the right-click entries, Tools › Check Spelling as You Type | Tests; the typing A/B equal to the start |
| PF.7 Language | Tools › Language…, the default for new documents, Save As SAM's loss line, hyphenation by language | Tests; layout comparison before and after |
| PF.8 Thesaurus | The MyThes reader, English inflection and capitals, the Thesaurus page, right-click Synonyms ▸ | Tests with a small thesaurus fixture |
| PF.9 Grammar | Harper, Ami Pro's sets mapped, the Grammar page, `[GramStyle]`, readability statistics (Tools › Readability Statistics…), the selection's word count, grammar underlines (off by default) | Tests; PGR1's statistics reproduced |
| PF.10 Measures | Round trip and layout comparison against the start; release timings (a full Spell Check of the longest corpus file, a paragraph's idle check, a grammar check); the typing A/B | Numbers in the progress file |
| PF.11 Twins | The app checks and changes a document (language British, `[GramStyle]` set, a replaced word) and writes SAM; Ami Pro opens it as the app shows it | VM screenshots outside Git |
| PF.12 Exit | `tools/verify.sh --release --corpus`, the records (`UI_SPEC.md` §35 notes, `FEATURE_PARITY.md`, `UI_ACCEPTANCE.md`, `UI_COMMAND_MATRIX.md`, `FORMAT_COMPATIBILITY.md`, `PROJECT_PLAN.md`, `PROJECT_STATUS.md`, `CLAUDE.md`, the playbook's queue and `PROOFING.md`), the handoff, the push | Green; `origin/master` equals `HEAD` |
