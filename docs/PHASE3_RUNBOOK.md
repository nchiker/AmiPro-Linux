# Phase 3 Runbook (the rest of styles and style sheets)

**Written:** 2026-10-12, after character overrides and Ami Pro's Text menu
items (`217801f`). **Purpose:** let one session, possibly resumed after
context compaction, carry the rest of Phase 3 (`PROJECT_PLAN.md`) as far as
it can go before the user's decisions are needed.

**Authority.** `AGENTS.md` governs how to work; nothing here overrides it.
`PROJECT_PLAN.md` defines scope, `docs/research/DECISIONS.md` the decisions
so far. This runbook adds the order, the gates, and the stop rules.

**Live state** is `docs/PHASE3_PROGRESS.md`. Keep it current: it is how a
resumed session knows where it is.

## 1. At the start of every session

1. Read `AGENTS.md`, `CLAUDE.md`, `docs/SESSION_HANDOFF.md`, this runbook,
   and `docs/PHASE3_PROGRESS.md`; then `git status` and
   `git log --oneline -15`.
2. If the tag `phase3-run-start` does not exist, create it at HEAD. It is
   the user's reference point for review; never move or delete it.
3. Resume at the first slice in the progress file that is not `DONE` or
   `WAITING FOR USER`. Uncommitted changes belong to an interrupted run:
   inspect, verify, and finish them; never discard them.

## 2. Working rules

- **Decompiled code is the primary source** (user direction): the
  decompiled `AMIPRO.EXE` (`../AmiPro-Reference/ghidra-work/out/amipro.c`;
  routine map in `docs/research/SAM_FORMAT_NOTES.md`, "The reader in
  AMIPRO.EXE") comes first for algorithms, format meanings, and behaviour;
  the XP VM (`docs/research/VM_LAB.md`) confirms what the code shows, or
  answers what it cannot (what the screen shows, dialogs).
- **Decompile whatever else the work needs**, without asking (`AGENTS.md`
  authorizes it as broadly as needed): other binaries in
  `../AmiPro-Reference/Binaries/` (for example `ltshyph1.dll`, the
  hyphenator, for 3.6; `amiproui.dll`, `amidlg.dll`, or `amifont.dll`
  for dialogs and fonts; the filters), or a fresh, deeper Ghidra pass over
  parts of `AMIPRO.EXE`. Methods, including the OPTLOADER unpacking and
  the NE loader fix, are in `docs/research/DECOMPILATION.md`; record each
  new binary's method and routine map there. Decompiler projects and
  output stay outside Git (`../AmiPro-Reference/ghidra-work/`); findings
  go into the format notes as VERIFIED (decompile); the implementation is
  written independently, never transliterated.
- **Measure layout changes.** Before and after every slice that can move
  text, run `layout_comparison_report` on the user's corpus
  (`AMIPRO_CORPUS=../AmiPro-Reference/Charlie-Sample`,
  `AMIPRO_STYLES=../AmiPro-Reference/Charlie-Sample/STYLES`) and on the VM
  fixtures (`AMIPRO_VM_LAB=../AmiPro-Reference/vm-lab/OUT`). Keep the
  output in the scratchpad; commit aggregate numbers only, never the
  user's file names or text. Explain any loss before moving on
  (`LAYOUT_COMPARISON.md`, "Character overrides", shows the expected
  depth).
- **Small green commits** on `master`: fmt, strict Clippy (core and the
  app's normal, `--all-features`, and `--no-default-features` builds), and
  all suites before each commit; `tools/corpus.sh` after importer changes.
  The VM fixtures must stay at 121 of 121 page counts and 75 of 75 page
  ends unless a change is explained.
- **Performance:** for changes on the typing path, an interleaved release
  A/B of `page_surface_typing_performance_baseline` (README31) against the
  slice's base, as recorded in `PROJECT_STATUS.md`.
- **Native format:** every model addition gets native v3 records,
  documented in `docs/Design/NATIVE_FORMAT_V3.md`, with round-trip and
  strictness tests; older files keep loading.
- **Timebox:** a problem that resists three distinct approaches is recorded
  in the progress file with its evidence, and the next independent slice
  starts.
- **Hand off at logical stopping points** (user preference): at the decision
  stop below, or when a session has run long, update the progress file,
  `docs/SESSION_HANDOFF.md`, `PROJECT_STATUS.md`, `PROJECT_PLAN.md`,
  `FEATURE_PARITY.md`, and `CLAUDE.md`, and end with a short, ranked list
  for the user: decisions first, then the shortest physical checks. Do not
  ask the user to test what the comparison, the VM, or the tests can show.

## 3. Stop rules

Stop only for what `AGENTS.md` lists, and for these, **batched into one
stop** after every slice that does not need them is done:

- **D1. Style editing design.** Create Style and Modify Style (and the
  Styles panel as the management owner, UI-3) are major interface work:
  `AGENTS.md` requires a user-approved visual specification first. Write a
  proposal (a new `UI_SPEC.md` section, with Ami Pro's dialogs from VM
  screenshots and the panel's states) and ask for approval.
- **D2. Hyphenation dictionary.** Hyphenation (`[algn]` 256) needs patterns
  and a dependency; the core crates are dependency-free. Write a short
  options note (candidate crates and pattern sets, licences, where the
  dependency would live, size) and ask.
- **D3. Writing Ami Pro files.** Writing a new `.STY` (Save as a Style
  Sheet) is original-format output; only proceed on the user's go-ahead,
  and never overwrite an existing file.

Ask the three as one short, ranked set of questions with a recommendation
each, then continue with whatever the answers unblock.

## 4. Slices, in order

### 3.1 Line heights for fonts never cached alone, and the paragraph's end font

- Calibrate the printer line height of fonts that Ami Pro's caches never
  show alone (bold runs in body text, `<:f…>` faces): a one-line paragraph
  whose line mixes a calibrated font with another caches the taller, which
  gives the other font once the first is known. Check how Ami Pro computes
  a mixed line in the decompile first.
- An empty paragraph holding only a format change takes that font's line
  height (Ami Pro caches 240 for a `<:f200,,>` line in a 12 pt style; we
  give the style's). The model needs the format in force at a paragraph's
  end; the importer keeps those controls as legacy marks today (75 such
  paragraphs in the user's corpus). Decide where it lives (paragraph-end
  format), store it in native v3, import it, lay it out.
- Done when: tests, native round trip, corpus gate, comparison before and
  after recorded in `LAYOUT_COMPARISON.md`.

### 3.2 Style rules from the decompile audit

- The style's own tabs (`[brk]` 32: the tab list after `[nfmt]` replaces
  the layout's tabs for the style's paragraphs) and Hanging indent's tab
  stop (`[algn]` 512) (`SAM_FORMAT_NOTES.md`, "Style records").
- Text Tightness (`[spc]` value 7: 90, 100, 115): how Ami Pro applies it
  (decompile), then layout.
- Next style (`[brk]` 64 and the style named after `[nfmt]`): Enter at the
  end of such a paragraph gives the new paragraph the next style; check
  Ami Pro in the VM (Enter in the middle, at the start).
- Whether frame interiors use the space-below rule (`SESSION_HANDOFF.md`).
- Done when: each rule has evidence, tests, and a measured comparison.

### 3.3 Character leftovers

- Overstrike (`<:O+C>` … `<:O-C>`): model, native v3, import, drawing (the
  character drawn over each character, in the text's font), and Special
  Effects' Overstrike character box.
- Protected text (`<+*>`) and no hyphenation (`<+.>`) as run attributes
  (model, native, import) instead of legacy marks; what protected text does
  to editing in Ami Pro (VM), and do the same.
- The face's pitch-and-family byte (`<:f…>` K and the style's `[fnt]` kind
  bits): keep it, and use it to pick a substitute for a face that is not
  installed (measure).
- Done when: tests, corpus gate, comparison, and a screenshot of overstrike
  beside Ami Pro's (VM fixture O1 has one).

### 3.4 Styles in the model: commands, style sheets, and their tests

- Typed, undoable commands to create (based on an existing style, as Ami
  Pro's Create Style), modify, rename, and delete styles; what happens to a
  deleted style's paragraphs in Ami Pro (VM).
- Use Another Style Sheet: apply an `.STY`'s styles and page layout to the
  document (Ami Pro's rules for styles the document has and the sheet does
  not: VM).
- Tests: save and reopen, unknown records kept, malformed style sheets
  refused with a message.
- Save as a Style Sheet (STY writer) waits for D3.
- Done when: commands and tests are in; menu entries only after D1.

### 3.5 Decision stop (D1–D3)

Prepare the D1 proposal and the D2 note, ask D1–D3 together, and hand off
if the answers are not immediate. Mark the dependent slices `WAITING FOR
USER` in the progress file.

### 3.6 After the decisions

- UI-3: the Styles panel as management owner, synchronised with the
  toolbar's style field, mixed state, keyboard focus (`UI_SPEC.md` §8
  slice 3; `UI_ACCEPTANCE.md` Gate C), and the approved Create and Modify
  Style dialogs. Screenshots in light and dark; physical check WAITING FOR
  USER.
- Hyphenation as decided (D2), measured.
- The STY writer as decided (D3): the decompiled writer's field order
  (`11c8:26b7`), a VM check that Ami Pro opens the written sheet, and no
  overwriting.

## 5. Phase exit

`PROJECT_PLAN.md`: "Stable styles and explicit fidelity reports." Update
its Phase 3 checkboxes, `FEATURE_PARITY.md` (ST-01, ST-02, IO-02), and the
handoff, with the comparison's final numbers.
