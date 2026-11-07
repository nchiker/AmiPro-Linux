# Phase 2 Unattended Runbook (2.3–2.7)

**Written:** 2026-10-11, after the Phase 2.2 model decision (`0cc201f`).
**Purpose:** let one unattended session, possibly long and possibly resumed
after context compaction, carry Phase 2 from 2.3a to the point where only
user feedback remains.

**Authority.** `AGENTS.md` governs how to work, and nothing here overrides it.
`PROJECT_PLAN.md` defines scope. The decision record
(`docs/research/DECISIONS.md`, 2026-10-11 Phase 2.2) and `ARCHITECTURE.md`
define the design. This runbook adds the order, the gates, the stop rules, and
practical guidance.

**Live state** is `docs/PHASE2_PROGRESS.md`. Keep it current: it is how a
resumed session knows where it is.

## 1. At the start of every session or loop iteration

1. Read `AGENTS.md`, `CLAUDE.md`, this runbook, and
   `docs/PHASE2_PROGRESS.md`. Then read `git status` and
   `git log --oneline -15`.
2. If the tag `phase2-run-start` does not exist, create it at HEAD with
   `git tag phase2-run-start`. It is the user's reference point for
   reviewing or reverting the run. Never move or delete it.
3. Resume at the first item in `docs/PHASE2_PROGRESS.md` that is not `DONE`.
   - If the working tree has uncommitted changes, they belong to an
     interrupted run. Do not discard them. Inspect them, run
     `tools/verify.sh`, and either finish the work or fix it.
   - Never reset, clean, or stash away work you did not write in this
     context without recording why in the progress file.
4. Before each design-heavy slice (2.3c, 2.3d, 2.4, 2.5b, 2.6), read the
   relevant sources: `docs/research/SAM_PAGE_FRAME_SPEC.md`,
   `docs/research/SAM_FORMAT_NOTES.md`, the 2.2 decision, `ARCHITECTURE.md`,
   and the code you will change.

## 2. Working rules for the unattended run

- **Keep going.** After a slice is done, start the next one at once. Do not
  end the turn to report progress while unblocked work remains. Progress
  goes in `docs/PHASE2_PROGRESS.md` and in commits, not in chat.
- **Small green commits.** Commit each coherent step on `master`, which is
  the project practice; there is no remote and nothing is pushed.
  - Run `tools/verify.sh` before every commit that touches code, or
    `tools/verify.sh --quick` when only the core changed. Never commit
    red, except as an explicitly labelled WIP that the progress file says is
    unverified.
  - Use the commit attribution trailer your harness specifies.
- **Slice done** means its "Done when" list in section 5 passes. Then:
  1. set the slice to `DONE` in the progress file, with commits, test
     counts, and measurements;
  2. commit that update;
  3. update `PROJECT_PLAN.md` checkboxes at the end of each numbered item
     (2.3, 2.4, …).
- **Timebox.** If one problem resists three distinct approaches, or a slice
  runs far beyond its expected size:
  1. record the blocker and the evidence in the progress file;
  2. take the smallest safe alternative that stays within the decision, or
     move to the next independent slice;
  3. come back later.

  A failing test is not a reason to stop.
- **Changing a decision.** D1–D8 may change only on measured or verified
  evidence: a failed performance gate, a VM observation, or a corpus
  counterexample. Record it as a new dated entry in
  `docs/research/DECISIONS.md` and update `ARCHITECTURE.md`. This is
  engineering, not a user decision.
- **Keep the default build stable.** The user's UI-2 physical review is
  pending. The default build must keep its current visible chrome and
  editing behaviour.
  - New page-surface UI goes behind a new Cargo feature, `page-surface`,
    in `app/cosmic`.
  - The visible default-build changes this runbook allows: imported
    documents show inline-object markers (U+FFFC) in the main text after
    2.4, and the status bar shows real page numbers after 2.5c.
  - List every visible change in the progress file.
- **Dependencies.** The core crates stay free of dependencies. An app
  dependency needs a licence check and a `DECISIONS.md` entry (Parley is the
  expected candidate, only if 2.5a selects it). Keep the libcosmic pin.
- **Reference material** (`AGENTS.md`):
  - Never commit proprietary bytes.
  - Synthetic SAM fixtures in Git are written by hand from the
    specification. They are not copies of Ami Pro output.
  - Tests that read private files take their paths from `AMIPRO_VM_LAB`,
    `AMIPRO_PERF_SAM`, or `AMIPRO_CORPUS`, and skip with a printed note when
    the path is absent.
- **The VM** (`docs/research/VM_LAB.md`) may be used whenever a question
  needs original behaviour.
  - Leave it in saved state at the end (`VBoxManage controlvm "Windows XP"
    savestate`). Keep the baseline snapshot. Leave `Windows11` alone.
  - Record new fixtures in `SAM_FORMAT_NOTES.md` with their evidence level.
- **Report each verification layer separately**: model, widget, and
  physical. Never mark a physical check passed.
- **`AGENTS.md.backup`** is the user's file. Do not touch it.

## 3. Stop conditions: when user feedback is required

Stop only when everything that remains is on this list, or for a stop reason
in `AGENTS.md`: a destructive action, a licensing problem, a missing
reference path, or a material product decision that the documents do not
cover.

1. **Physical COSMIC checks.** The page surface's interaction (2.6) and the
   2.7 acceptance. Before stopping, add a focused checklist for them to
   `docs/Design/UI_ACCEPTANCE.md`.
2. **The "separately approved milestone" for frame interaction**
   (`docs/Design/UI_SPEC.md`, "Future object selection" and "Context
   rail"). That means the Frame Properties inspector, frame selection
   handles, move and resize by pointer, and any frame toolbar or object
   action group. Do not implement them before approval.

   Before stopping, write the proposal as a new section of
   `docs/Design/UI_SPEC.md`, marked *proposed*. It covers the inspector's
   field groups (only model-backed fields), the selection and handle
   behaviour following Ami Pro's rules (VM evidence), keyboard access, the
   rail switching rules already approved, and acceptance gates. Add the two
   rail states to the mockup only if this is quick and clear; otherwise
   describe them.
3. **Making the page surface the default editor.** This needs physical
   acceptance plus the accessibility and visual-IME gates.

When stopping:

1. Fill in "Waiting for the user" in `docs/PHASE2_PROGRESS.md`. Give each
   request exactly, one command or checklist each.
2. Update `PROJECT_STATUS.md`, `PROJECT_PLAN.md`, `FEATURE_PARITY.md`,
   `docs/SESSION_HANDOFF.md`, and the status and next-step sections of
   `CLAUDE.md`.
3. Commit.
4. Leave the VM in saved state.
5. End with a short summary: what is done, the test and measurement
   evidence, and what is waiting.
6. If running under `/loop`, end the loop (`ScheduleWakeup` with `stop:
   true`).

## 4. Tools

- **`tools/verify.sh [--quick] [--release] [--corpus]`** is the gate.
  - The default runs fmt, strict Clippy, and tests for the core and for
    the app (default and all features).
  - Use `--release` before marking any numbered item done, and `--corpus`
    for 2.3a, 2.3d, and 2.4 onwards.
  - Logs are in `target/verify/`.
- **`tools/corpus.sh`** runs the private corpus audit. It re-extracts
  `/tmp/amipro-phase1-corpus` from the ISO if `/tmp` was cleared, and
  fails on any import or round-trip failure.
  - Baseline at `a55e0fe`: 160/160 SAM, 160 round trips, 63/63 STY,
    28,748 paragraphs, 11,147 character runs.
  - 2.3 must not change these counts. 2.4 changes them deliberately;
    record the new counts and why they changed.
- **`tools/bench-ab.sh BASE_REV [ROUNDS]`** is the interleaved release A/B
  test of both editors on `README31.SAM`. Results go to
  `target/bench-ab/<time>/summary.txt`.
  - Keep the benchmark test names `default_editor_typing_performance_baseline`
    and `rich_editor_typing_performance_baseline` stable, or the base and
    the working tree cannot be compared.
  - **Noise rule.** On this host (powersave governor, unrelated emulators),
    a regression counts only if B's median for an operation is worse than
    A's by more than max(10%, 0.05 ms) in at least two of three rounds, or
    its p95 is worse by more than 25% in at least two of three rounds.
- **The VM:** `tools/vm/*` and `docs/research/VM_LAB.md`.

## 5. Work queue

Each slice lists its goal, guidance, and "Done when". The guidance is a
starting point; the decision record governs.

### 2.3a Identity and shared roots

Goal: introduce the IDs, the flows map, and `Arc`-shared roots, and move the
app's history onto them. There is no user-visible change.

- `amipro-document`:
  - ID newtypes (`FlowId`, `FrameId`, `LayoutId`, `GroupId`, `ObjectId`,
    `ParagraphId`) and a per-document allocator.
  - `Flow { paragraphs }`, with the paragraphs shared through `Arc`, and
    `main_flow`.
  - Accessors for the main flow, so call sites migrate mechanically.
  - A minimal `PageLayout` (id, name, size, and margins in twips) replaces
    `PageSetup`. Keep a helper that returns the points the current UI
    needs.
  - Native v2 load assigns IDs in document order. Encoding v2 still works
    until 2.3d.
- `amipro-editing`:
  - A `Command` enum with no inverse payloads, and
    `apply(&Document, &Command) -> Result<Applied>`, which returns the new
    root and a change report (flows, paragraph IDs, and so on).
  - Port `Editor`'s operations (insert, delete, replace range, split, join,
    paragraph style, character format) and keep their tests as command
    tests. Retire the inverse history.
  - A reusable `History` type (roots plus selection, capped at 100 entries
    as today).
- App:
  - Use `History`. Dirty tracking compares roots.
  - Keep `replace_plain_text` for the default editor's main flow for now.
  - The rich prototype issues commands.
- Add an ignored release micro-benchmark of per-edit command plus history
  cost on `AMIPRO_PERF_SAM`.

Done when:

- `tools/verify.sh --release --corpus` passes, with corpus counts
  unchanged.
- `tools/bench-ab.sh phase2-run-start 3` shows no regression under the
  noise rule. Record the summary.
- The per-edit command plus history cost on `README31.SAM` is at most
  0.07 ms median (today's whole-`Document` clone).
- Undo and redo stay exact in the existing app tests.

### 2.3b Inline objects

Goal: U+FFFC objects in paragraphs, legacy marks, and special characters,
kept intact by both one-flow editors.

- Model:
  - `Paragraph.objects` in text order, with validation (the number of
    U+FFFC equals the number of objects; U+FFFC has no other use).
  - `InlineObject::Break { kind, centre, layout }` and
    `FloatingHeaderFooter { kind, pages, flow }`, which owns a flow.
    `FrameAnchor` arrives in 2.3c.
  - Legacy marks are byte offsets plus raw text, adjusted like run
    boundaries.
  - U+2028, U+00AD, and U+00A0 are ordinary text.
- Commands:
  - Range deletion removes the objects in the range and everything they
    own (and, from 2.3c, the frames they anchor).
  - Split and join carry objects with their characters.
  - Input and paste replace U+FFFC with U+FFFD.
  - A single Delete or Backspace onto an anchor follows the V05 policy:
    for frame anchors (2.3c) it does nothing. Breaks and floating headers
    delete like characters.
- Editors:
  - The default editor's plain-text reconciliation and delta
    synchronization map the removal of a U+FFFC to the object rule and
    never invent objects.
  - The rich prototype does the same.
  - Widget-path tests per `AGENTS.md`: delete a range over objects, Undo,
    paste text containing U+FFFC, and type next to objects. Use
    multi-paragraph UTF-8 content.

Done when:

- `tools/verify.sh` passes.
- The model, command, and widget-path tests above exist and pass.
- `tools/bench-ab.sh` shows no regression against the commit before 2.3b.

### 2.3c Frames and layouts

Goal: the full D4 shape and its commands.

- `Frame` with:
  - placements (`Fixed`, `BelowParagraph`, `Inline`, `Repeating`,
    `Header`, `Footer`);
  - wrap, opacity, border, shadow, corners, background, interior
    `LayoutBox`, group, name, macro, and legacy data;
  - content `Text(FlowId)`, `Table` (columns, rows, and cells with
    `Text(FlowId)`), `Picture` (`Embedded(ObjectId)`, `Linked`, or legacy
    values), or `Legacy`.
- `frame_stack`, groups, the `objects` table, and the full `PageLayout`:
  - right and optional left geometry, mirroring, and begin-on-second-page;
  - header and footer frames per side;
  - repeating frames;
  - legacy data.
- `InlineObject::FrameAnchor` and the page-number field.
- Every invariant listed in `ARCHITECTURE.md` gets validation, and a
  failing-case test for each.
- Commands:
  - create, delete, move, resize, and set properties of frames;
  - change placement (converting to anchored inserts an anchor at a given
    position; converting to Fixed removes it);
  - bring to front and send to back, within the layer;
  - group and ungroup (Fixed frames on one page), and group move and
    delete;
  - insert a layout (a copy with fresh IDs) and modify a layout;
  - insert and remove breaks and floating headers/footers.
- A property test: random command sequences, then undo everything, must
  give back the original root exactly, and redo must give back the final
  one.

Done when:

- `tools/verify.sh` passes.
- The invariant tests, the command tests, and the undo property test pass.
- New documents get the default layout, with empty header and footer
  frames, as Ami Pro's G00 does.

### 2.3d Native v3

Goal: save and load everything in the model.

- First write `docs/Design/NATIVE_FORMAT_V3.md`: the record grammar, the
  escapes, the order of records, and how each record attaches to its
  context. Then implement it.
- `AMIPRO-LINUX/3` holds IDs and the counter, styles, layouts, frames, the
  stack, groups, flows, paragraphs, objects, runs, legacy marks, tables,
  pictures, legacy records, and base64 `blob` chunks.
- Decoding is strict, with full validation on load. v1 and v2 still load.
- The app saves v3.
- Tests:
  - exact round trip of every model feature;
  - a property test over generated documents;
  - rejection of malformed input (unknown records, dangling IDs, broken
    base64, wrong counts);
  - v1 and v2 compatibility;
  - the app's save and reopen.

Done when:

- `tools/verify.sh --release --corpus` passes, with the corpus round-trip
  check now going through v3.
- The tests above exist and pass.

### 2.4 SAM import (read-only)

Goal: import pages, layouts, frames, marks, tables, and pictures as
`docs/research/SAM_PAGE_FRAME_SPEC.md` (draft 2) specifies.

- Layouts: `[lay]`, `[rght]`/`[lft]`, header/footer frames, `[repfrm]`.
- Frames: `[frm]` flags and fields; `[frmlay]` margins, columns, and tabs;
  `[txt]`, `[tbl]` (grid plus cell paragraphs; other cell fields kept as
  legacy data), and `[isd]`.
- Embedded objects: the `[Embedded]` directory, offsets, the trailer, and
  bounds checks.
- Marks:
  - `<:A n>` and `<:t n>` through the shared index of anchored records;
  - `<:p…>` flags and layout names, and `<:c>`;
  - `<:H`/`<:h` flags, with their paragraphs as an owned flow;
  - `<**>` as U+2028 and `<*7>` as U+00AD;
  - `<:P…>` as a page-number field.
- Unmapped controls become legacy marks. Unknown fields stay with their
  object. `[pg]` and `<:#…>` are dropped as caches.
- Every new parse path keeps the existing limits and bounds checks. Add a
  fuzz-style test over truncated and garbled inputs; it must return errors,
  never panic.
- The import report and `docs/research/FORMAT_COMPATIBILITY.md` say what is
  mapped, what is kept as legacy data, and what is dropped as a cache.
- The corpus audit gains these metrics: frames by kind and placement;
  anchors resolved and unresolved; unknown flag bits; layouts per
  document; page columns that differ from the uniform derived edges; and
  the v3 round trip.
- Fixtures:
  - Hand-written synthetic fixtures in
    `crates/ami-formats/tests/fixtures/`.
  - A private test over the VM fixtures (`AMIPRO_VM_LAB`) that asserts
    decoded structure: for example, F01's rectangle, G21's two layouts
    and the break that names layout 1, V20's anchors resolving to the
    right frames, P02's 9,270 picture bytes, V17's footer flags.
- If a corpus document contradicts the specification, settle it with a VM
  fixture where possible and update the specification.

Done when:

- `tools/verify.sh --release --corpus` passes, with all 160 imported, 0
  unresolved anchors, 0 control-like leaks, and 160 v3 round trips. Record
  the new counts.
- The private VM-fixture test passes.

### 2.5a Line-breaking spike (bounded: about one slice)

- In `experiments/page-layout-spike` (isolated, like the BetterOffice
  proof of concept), break lines over COSMIC Text `ShapeLine` words with:
  - a different width per line;
  - inline boxes for U+FFFC;
  - U+2028 as a forced break, and U+00AD as a break point that shows a
    hyphen when used.

  Run it on `README31.SAM` text and on synthetic bidi and ligature cases.
- Compare with Parley 0.12 on the same cases only if a case fails.
- Record the result and the choice in
  `docs/research/LINE_BREAKING_SPIKE.md`, and add a `DECISIONS.md` entry.

Done when: the document and the decision entry exist and are committed.

### 2.5b Layout engine

- `amipro-layout` defines a shaper trait. Core tests use a deterministic
  fixed-advance shaper. The COSMIC Text implementation lives in the app.
- The engine covers:
  - pages, columns, balance, breaks, and layout switching;
  - the page-count rule (V11, X06);
  - Fixed, anchored, and repeating placement: anchored frames are clamped
    and move with their paragraph or line (V12, V13);
  - wrap exclusions for main and header/footer text (V14);
  - headers and footers: right/left, mirror, begin on second page, and
    floating overrides;
  - frame interiors, with derived columns and corner insets
    (`SAM_FORMAT_NOTES.md`);
  - a basic table grid and picture boxes.
- Output is a display list whose fragments carry flow, paragraph ID, and
  byte range, plus frame boxes with their layer and z-order.

Done when:

- `tools/verify.sh` passes.
- Deterministic tests cover each rule above, citing the VM fixture each
  rule comes from.

### 2.5c Incremental layout

- A per-paragraph shaping cache, and re-pagination with convergence (D5).
  Header and footer edits repaint only.
- Add an ignored release benchmark: full layout, and relayout after typing,
  on `README31.SAM`, `AP31NAV.SAM`, and `MERCURY.SAM`.
- Target: relayout after typing on `README31.SAM` at most 1 ms median and
  4 ms p95, headless. If it is missed, record the numbers and the
  bottleneck, and continue.
- The status bar shows the real "Page X of Y" in both builds. This is an
  allowed visible change.

Done when:

- `tools/verify.sh --release` passes.
- The benchmarks are recorded in the progress file.
- Stale-cache tests pass: edit, format, style change, resize, undo, and
  redo.

### 2.5d Comparison with the original

- Use Ami Pro's own caches as oracles in fixtures and corpus files:
  - `[pg]` page ends (paragraph and offset per page);
  - `<:#h,w>` paragraph heights and widths;
  - stored rectangles of anchored frames;
  - stored frame column edges.
- Compare against our layout for the VM fixtures, and measure page-count
  agreement over the corpus.
- Note which fonts were used. Times New Roman and Arial need
  metric-compatible substitutes (Liberation or Tinos/Arimo); check which
  are installed.
- Make new VM fixtures when a disagreement needs explaining.
- Record per-fixture results, tolerances, and known deviations in
  `docs/research/LAYOUT_COMPARISON.md`.

Done when: the document exists, and every fixture is within tolerance or
has a recorded explanation.

### 2.6 Page editing surface (automated parts; `page-surface` feature)

- Render the display list: pages, text, frame borders, backgrounds and
  shadows, and pictures.
  - Decode pictures if an existing app dependency supports the format;
    otherwise draw a labelled placeholder and record the gap.
  - Layers and z-order follow D4 and V14.
- Use one view transform for rendering, hit testing, caret, selection, IME,
  ruler, and scroll extents.
- Zoom follows the approved status-bar design (`UI_SPEC.md`, "Status bar"
  and "Page workspace", UI-4).
- Hit testing and caret go into any flow:
  - main text across pages and columns;
  - text frames;
  - headers and footers, with the page instance as view state;
  - basic table cells.
- Editing through commands: typing, deletion, Enter and join, selection,
  clipboard, Undo and Redo, and IME commit and preedit, reusing the rich
  prototype's techniques.
- Navigation: arrows across lines, columns, and pages; Home and End;
  PageUp and PageDown. Esc from frame text returns to the main text.
- Out of scope until approved: frame handles, pointer move and resize, the
  Frame Properties inspector, and a frame toolbar (section 3, item 2).
- Tests through the real widget path (`AMIPRO_VM_LAB`-free synthetic
  documents with frames, multiple pages, UTF-8, and mixed sizes):
  - press, drag, and release;
  - caret across pages and into and out of frames;
  - scrolling and zoom transforms;
  - stale-layout after edits.
- Add an end-to-end typing benchmark for the page surface. Target: median
  at most 1 ms and p95 at most 4 ms on `README31.SAM`, headless.
- Add the page-surface checklist to `UI_ACCEPTANCE.md` (Gates D, F, and G
  as they apply). Write the frame-interaction proposal (section 3).

Done when:

- `tools/verify.sh --release` passes, with the feature included in
  `--all-features`.
- The tests and the benchmark are recorded.
- The checklist and the proposal are committed.

Then stop (section 3).

### 2.7 Physical and original-compatibility acceptance

This needs the user. Prepare it only: the checklist (from 2.6) and the
comparison materials (from 2.5d).

## 6. Expected visible changes in the default build

The only allowed ones:

- U+FFFC markers where imported documents have anchors, breaks, or floating
  headers (from 2.4);
- a real page number and count in the status bar (from 2.5c).

Anything else visible belongs behind `page-surface`.
