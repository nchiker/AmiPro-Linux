# Phase 2 Progress (live)

This is the live state for the unattended Phase 2 run (`docs/PHASE2_RUNBOOK.md`).
Update it after every slice, and before any long step, so that a resumed or
compacted session can continue.

Status values: `TODO`, `IN PROGRESS`, `DONE`, `BLOCKED` (with the reason),
and `WAITING FOR USER`.

**Run start tag:** `phase2-run-start`, the base for `tools/bench-ab.sh` and
for review.

## Slices

| Slice | Status | Commits | Evidence (tests, measurements) | Notes |
|---|---|---|---|---|
| 2.3a Identity and shared roots | DONE | `bbb39d6` | Core 48 pass + 1 ignored; app 71/76 pass (2 ignored); corpus unchanged; command+history 5.2 µs median per edit on README31; A/B: no end-to-end regression (see log) | One accepted sub-step flag (cold `plain_text`), see log |
| 2.3b Inline objects | DONE | `45aa4b4`, `4bdc057` | Core 63 + 1 ignored; app 73/79 (2 ignored); model, command, and widget-path tests for both editors; A/B: typing, deletion, replacement, and rich editor hold or improve | Undo/redo medians not comparable with 2.3a (no-op undo steps), see log |
| 2.3c Frames and layouts | DONE | `8b617ef` | Core 77 + 1 ignored; app 74/81 (2 ignored); invariant, command, and undo property tests; corpus unchanged | New documents get G00's Standard layout with empty header and footer frames |
| 2.3d Native v3 | DONE | `eb7eed7` | Core 80 + 1 ignored; app 74/81 (2 ignored); `verify.sh --release --corpus` PASS; corpus 160 round trips through v3 | Spec `docs/Design/NATIVE_FORMAT_V3.md`; the app saves v3 |
| 2.4 SAM import | DONE | `1343b63` | Core 92 + 1 ignored; `verify.sh --release --corpus` PASS; corpus 160/160, 160 v3 round trips, 0 unresolved anchors, 0 control leaks; private VM-fixture test PASS | D7 refined (page tables); counts changed as recorded in the log |
| 2.5a Line-breaking spike | DONE | `819be11` | `docs/research/LINE_BREAKING_SPIKE.md`; identical to COSMIC Text on 4,457 README31 lines and bidi; breaking 0.6–0.7 ms for 344 paragraphs | Keep the own breaker; Parley not needed |
| 2.5b Layout engine | DONE | `df0c48b` | Core 114 + 1 ignored; 19 rule tests citing fixtures; all 160 corpus files lay out (1,585 pages, fixed shaper) | Several placement details are inferred; 2.5d checks them |
| 2.5c Incremental layout | DONE | `8bb1216` | Core 117 + 1 ignored; app 77/84 (3 ignored); `verify.sh --release` PASS; incremental equals fresh layout after every change kind and 960 random edits; relayout after typing 0.09–0.10 ms median, 0.11 ms p95 (README31) | Status bar shows the real "Page X of Y"; A/B: +0.06–0.10 ms per edit, the cost of keeping pages current (see log) |
| 2.5d Comparison with the original | DONE | `412f9c7` | `docs/research/LAYOUT_COMPARISON.md`; core 118 + 1 ignored, app 77/84 (4 ignored), `verify.sh` PASS (release builds PASS), corpus PASS; fixtures: 94% line counts equal at Ami Pro's widths (100% within one), 90/92 page counts equal, page ends all on the same page with Ami Pro's spacing except AP31NAV | Three fixes (line gap, table frame size, inline shadow); the rest needs Phase 3 style mapping |
| 2.6 Page surface (automated parts) | DONE | `2c9a38e` | `verify.sh --release` PASS with `page-surface` in all features; widget-path tests (6), message-path tests (3), transform and editing unit tests (9); README31 typing 0.42–0.48 ms median, 0.44–0.49 ms p95 end to end; A/B vs `18c501c`: no regression | Physical check WAITING FOR USER (`UI_ACCEPTANCE.md` §10) |
| 2.6 Frame-interaction proposal | DONE (approved 2026-10-12) | `91195dd` | `docs/Design/UI_SPEC.md` §11 | |
| 2.6 Frame interaction | DONE (automated and original compatibility); physical PASS (2026-10-17, on created frames) | `0e5f3f7`, `9319efd`, `5694a8d` | Controller, inspector, widget-path (50/100/200%), and message-path tests; `verify.sh --release` PASS (core 128 + 1 ignored; app 78 and 121, 4 and 8 ignored); drag redraw 0.006 ms recorded, release 1.9 ms median (README31); A/B vs `e90ced1`: no regression | Checklist `UI_ACCEPTANCE.md` §10 "Frame interaction"; F14–F20 compared with Ami Pro (`5694a8d`) |
| 2.7 Physical and original acceptance | WAITING FOR USER | | `UI_ACCEPTANCE.md` §10; `docs/research/LAYOUT_COMPARISON.md` | Prepared |

## Baselines at the run start (`a55e0fe` plus tooling)

- Core tests 34/0. COSMIC tests 71 passed with 2 ignored (default), and 76
  passed with 2 ignored (all features).
- Corpus: 160/160 SAM imported, 160 native round trips, 63/63 STY, 28,748
  paragraphs, 11,147 character runs.
- Default editor, release, headless, `README31.SAM` at `8b78ecf`: typing
  0.58–0.62 ms median; undo and redo 0.43–0.67 ms median.
- Whole-`Document` snapshot clone: about 0.07 ms on `README31.SAM`.

## Visible changes in the default build

No new UI. The default editor now edits through commands (2.3b), which
changes three editing details; they are corrections, listed here so the
user's review can check them:

- An edit action that changes nothing (Backspace at the start of the
  document, Delete at its end) no longer marks the document modified or adds
  an undo step that does nothing.
- Enter splits a paragraph with its character formatting: the second half
  keeps its bold or italic runs (it lost them before). Enter at the start of
  a paragraph gives the new empty paragraph that paragraph's style (it took
  the previous paragraph's style before); Enter at the end continues the
  current style as before.
- A pasted or typed U+FFFC (object replacement character) becomes U+FFFD,
  so text can never invent an inline object.

From 2.4, imported documents show (both allowed by runbook section 6 and
2.4's own scope):

- U+FFFC markers where they have frame anchors, page and column breaks,
  floating headers and footers, and page-number fields;
- hard returns as U+2028 and discretionary hyphens as U+00AD, which were
  dropped before (the text editor shows U+2028 as a line break inside the
  paragraph).

From 2.5d, single-spaced lines include the font's line gap, as Ami Pro's do,
so page counts in the status bar grow slightly (12 pt Times New Roman: 276
twips per line instead of 266); imported table frames have their real size.

From 2.5c, the status bar shows `Page X of Y`: X is the caret's page and Y
the laid-out page count (COSMIC Text shaping, Liberation fonts for the
Windows core fonts), in place of `Page 1 of N · estimate`. Both builds.

In the `page-surface` build only (2.6): the page surface replaces the text
editor; Zoom In, Zoom Out, Actual Size, and Fit Page Width are implemented;
the status bar gains zoom controls. The default build shows none of this.

## Decisions changed during the run

- 2026-10-11, D7 refined (`docs/research/DECISIONS.md`, "Page-table
  placement and the 2.4 import"): page tables keep the anchored placement
  their flags give (189 of 214 corpus page tables are With para above, not
  Flow with text as D7 assumed from TB1); a With para above frame is placed
  after the text before its anchor, so `<:t n>` at a paragraph start puts the
  table above that paragraph. Also recorded there: unreadable frame records
  keep their place, an anchored record without a mark becomes Where placed,
  a layout with left frames but no `[lft]` gets left pages from its right
  pages, and extra slot frames stay raw.
- 2026-10-12, after the run, D7 refined again (`DECISIONS.md`, "With para
  above frames in the text, and table offsets"): from the user's MERCURY.SAM
  comparison, text runs beside a No wrap around With para above frame (only
  No wrap beside and table frames take their height), a frame after text
  stands before the space after, a paragraph holding only anchors takes no
  height, and a table's offset comes from its stored rectangle.
- 2026-10-12, after the run, D7 and D8 refined (`DECISIONS.md`, "Table
  geometry, joined cells, and Table Text"): from the user's README31.SAM
  comparison, tables carry cell margins and joined cells (in the model and
  native v3, old records loading with defaults), untagged cell text is Table
  Text, and an anchored table grows to fit its text within a column.

## Blockers and open questions

- `experiments/betteroffice-layout-poc` (excluded from the workspace and the
  gates, not adopted) still uses the pre-2.3a `Document` fields and no longer
  builds against the current core. It is left as a historical proof of
  concept.

## Waiting for the user

The run stopped here (runbook §3): everything left needs the user.

**Answered on 2026-10-12:** the frame-interaction proposal is approved (item
2, implemented the same day: see the run log); the user's short page-surface check (item 1) passed scrolling, zoom, and
clicks into frames and cells, and found the caret not moving at the space
bar, fixed and rechecked ("typing seems good", `ce4e47c`;
`UI_ACCEPTANCE.md` §10, "User's short physical check"). The rest of item 1
and items 3–5 remain open.

1. **Physical COSMIC check of the page surface.** Run
   `cargo run --offline --manifest-path app/cosmic/Cargo.toml --features page-surface`
   and work through `docs/Design/UI_ACCEPTANCE.md` §10 (Gates D, F, and G),
   with a new document, `README31.SAM`, `AP31NAV.SAM`, and `MERCURY.SAM`
   (its text overlaps the icons on page 8 until indents are imported in
   Phase 3; that is expected). Report PASS,
   PARTIAL PASS, or FAIL per item, with screenshots of anything wrong. The
   known gaps listed there (picture placeholders, no accessibility nodes yet,
   action-bar state in frame text, no double-click selection, no frame
   selection) are to be confirmed, not failed.
2. **Approve, change, or reject the frame-interaction proposal** in
   `docs/Design/UI_SPEC.md` §11 (selection states, handles, moving and
   resizing, keyboard access, the Frame Properties inspector, rail switching,
   and its acceptance gates). Nothing in it is built until you approve.
3. **Physical check of the default build's visible changes from this run**,
   as part of the UI-2 review below: the status bar's `Page X of Y` (the
   caret's page of the real layout); U+FFFC markers, hard returns, and
   discretionary hyphens in imported documents; and the three editing
   corrections listed under "Visible changes in the default build".
4. **Decide later whether the page surface becomes the default editor.** It
   should not before items 1 and 2 pass and Gate F's accessibility work is
   done; the runbook leaves that switch to you.
5. **Review the comparison with Ami Pro** (`docs/research/LAYOUT_COMPARISON.md`,
   including the MERCURY.SAM section added after the run):
   whether its tolerances and known deviations are acceptable for Phase 2.7,
   given that paragraph spacing, exact line spacing, indents, and break rules
   arrive with the style mapping in Phase 3.

Already pending before the run:

- UI-2 physical COSMIC review (`docs/Design/UI_ACCEPTANCE.md` section 5),
  including default-editor responsiveness.

## Run log

Newest last. One entry per slice or notable event: date, what happened,
commits.

- 2026-10-11: run prepared. Added the runbook, this file, and
  `tools/verify.sh`, `tools/corpus.sh`, and `tools/bench-ab.sh`.
- 2026-10-11: 2.3a done (`bbb39d6`).
  - Model: typed IDs (`FlowId`, `FrameId`, `LayoutId`, `GroupId`,
    `ObjectId`, `ParagraphId`) from one counter; a flows map with the main
    flow; `Arc`-shared styles, flows, paragraphs, and layouts; a minimal
    `PageLayout` in twips ("Standard", Letter, 1-inch margins) replacing
    `PageSetup`, which remains as the points view (`page_setup()`). v1/v2
    load assigns IDs in document order. `eq_ignoring_ids` compares content
    for v2 reloads, which cannot keep IDs.
  - Editing: `Command` (`ReplaceText`, `SetParagraphStyle`,
    `FormatCharacters`), `apply(&Document, &Command) -> Applied` with a
    `ChangeReport`, and `History<S>` (roots plus selection, 100 steps, an ID
    floor so undone IDs are never reissued). `Editor` is a thin main-flow
    wrapper; the inverse history is gone. Typing inside a paragraph now
    extends the run it lands in for both editors (the rich prototype used to
    leave inserted text unformatted; the default editor already extended).
  - App: one `History` per session; dirty tracking compares roots (pointer
    fast path). The rich prototype maps actions to commands executed by the
    session history. The default editor still reconciles through
    `replace_plain_text`, which now keeps unchanged paragraphs shared.
  - Gates: `tools/verify.sh --release --corpus` PASS (core 47 + 1 ignored at
    the run, 48 after a later test; app 71 and 76 with 2 ignored each;
    release builds; corpus 160/160 imported, 160 round trips, 63/63 STY,
    28,748 paragraphs, 11,147 runs, 2 control-like paragraphs, both literal
    text as before).
  - Per-edit command plus history on README31.SAM (344 paragraphs, release,
    2,000 insertions): median 5.2 µs, p95 8.1 µs, max 27 µs; undo and redo
    about 30 ns. Gate: at most 70 µs (the old whole-document clone).
  - `tools/bench-ab.sh phase2-run-start 3`, run twice
    (`target/bench-ab/20261011-224458`, `…-224756`), checked with the new
    `tools/bench-compare.py`. Default editor: no regression in the second
    run (the first run's empty-document redo p95 flag did not repeat). Rich
    editor end to end improved: insert, delete, replacement, and sustained
    typing 0.32–0.36 → 0.24 ms median; undo and redo 0.18 → 0.15 ms;
    document clone 0.065 → 0.001 ms.
  - One flag in both runs, accepted: the rich benchmark's "plain-text
    rebuild" sub-step went from 0.011 to 0.033 ms median. It is measured
    cold, right after an 80 ms layout rebuild; warm it is 3.3 µs before and
    after (scratch micro-benchmark). The cause is one more pointer per
    paragraph, the `Arc` sharing that D6 decided. It is not an editor
    operation, and every end-to-end operation held or improved.
- 2026-10-11: 2.3b done (`45aa4b4`, performance follow-up `4bdc057`).
  - Model: `Paragraph.objects` (one per U+FFFC, in order) and
    `Paragraph.marks` (zero-width `LegacyMark { byte, raw }`).
    `InlineObject::Break { kind, centre, layout }` and
    `FloatingHeaderFooter { kind, pages, flow }`, which owns its flow.
    Validation: placeholder count, objects only in the main flow, every
    other flow owned exactly once, column breaks neither centre nor switch,
    named layouts exist, marks sorted, non-empty, and on boundaries. Native
    v2 refuses documents it cannot store. U+2028, U+00AD, and U+00A0 are
    ordinary text (and the COSMIC text stack's line iterator splits only on
    CR and LF, so the default editor treats U+2028 as text too).
  - Commands: range replacement deletes objects in the range and the flows
    they own, carries objects through split and join, moves marks like run
    boundaries (strictly inside a deleted range: deleted; at either end: after
    the inserted text), and turns U+FFFC from input into U+FFFD. New:
    `DeleteCharacter` (a single Backspace or Delete; breaks and floating
    headers delete like characters), `InsertBreak`, and
    `InsertFloatingHeaderFooter`. The report lists added and removed flows.
  - Default editor: `replace_plain_text` is retired. Each text change becomes
    one `ReplaceText` from `plain_text_edit`, which uses the caret to place
    ambiguous edits, executed through the session history; the editor is
    brought back to the model when they differ. Rich prototype: single
    Backspace and Delete issue `DeleteCharacter`.
  - Tests: 7 model tests, 13 command tests, a default-editor message-path
    test, a prototype app-path test, and a prototype surface test with real
    key and IME events, all over multi-paragraph UTF-8 content with a break
    and a floating header.
  - Gate: `tools/verify.sh` PASS (core 63 + 1 ignored; app 73 and 79, 2
    ignored each).
  - Performance. The first A/B (`target/bench-ab/20261011-225906`) showed default-editor typing up by about
    0.1 ms at the end of README31. A probe found the new per-keystroke work
    scanned the document byte by byte twice (caret offset 69 µs, diff
    56 µs). `4bdc057` made both run at memory speed (`changed_span`,
    `memchr` line starts) and also sped up the editor resync that undo uses.
    Headless update only, last paragraph of README31: typing 63 µs (2.3a:
    76 µs), backspace 50 µs (71), replacement 47 µs (69).
  - A/B after the fix against `154e6dc`, three runs
    (`target/bench-ab/20261011-230722`, `…-230901`, `…-230953`; the first
    was disturbed by host load, with even the empty document doubling in two
    rounds). In the two clean runs, typing, deletion, replacement, and every
    rich-editor operation show no regression. Two kinds of flag remain:
    - Undo and redo at the fixture start and end (B about 0.51 ms against A
      0.33 ms median). Not comparable: 2.3a recorded an undo step for every
      edit action, even one that changed nothing, and in this benchmark's
      sequence 28 of A's 40 undo samples restore an identical root (counted
      with a probe), so A's median is a no-op. Per real undo step, the update
      costs about 102 µs in B against about 175 µs in A (probe, same
      position). Accepted; future runs against a 2.3b-or-later base compare
      like with like.
    - The rich "plain-text rebuild" p95, within 0.003 ms (noise).
- 2026-10-11: 2.3c done (`8b617ef`).
  - Model (new modules `geometry.rs`, `frame.rs`): `Frame { placement,
    style: FrameStyle, group, content, legacy }`; `Placement` (Fixed,
    BelowParagraph, Inline, Repeating, Header, Footer); `FrameStyle` (wrap,
    opacity, `Border`, `Shadow`, corners, background, interior `LayoutBox`,
    name, macro); `FrameContent` (text flow, `Table` of cell flows, `Picture`
    embedded or linked with an optional Ami Pro rendering, legacy); the frame
    stack; embedded objects (`Arc<[u8]>`); document-level `LegacyData`; and
    the full `PageLayout` (paper, landscape, right and optional left
    `PageGeometry` with columns, per-column tabs, balance, line between, and
    page lines; mirror; begin-on-second-page flags; header and footer frames
    per side; repeating frames). Inline objects add `FrameAnchor` and the
    `PageNumber` field, which may occur in any flow.
  - New documents (and v1/v2 loads and SAM imports, for now) get the
    Standard Letter layout with empty header and footer frames: G00's
    Header/Footer panel margins (0.55 in toward the edge, 0.05 in toward the
    text, 1 in at the sides) and its centre and right tabs.
  - Validation covers every `ARCHITECTURE.md` invariant, each with a
    failing-case test: unique IDs below the counter; one owner per non-main
    flow (text frames, table cells, floating headers); every frame reachable
    exactly once and in its own layer; one anchor per anchored frame and
    none for others; anchors, breaks, and floating headers only in the main
    flow; placeholder counts; groups (two or more Where placed frames, one
    page, adjacent in z-order); geometry (pages, rectangles, sizes, margins,
    1–8 columns, one tab list per column, corners, shadow); left-page parts
    only with left pages; table cell counts; picture payloads; and the
    placement–wrap tie (Flow with text wraps around, With para above does
    not).
  - Commands: `CreateFrame`, `DeleteFrame` (whole group; an anchored frame
    goes with its anchor), `MoveFrame` and `SetFramePage` (group as one),
    `ResizeFrame`, `SetFrameStyle`, `ChangePlacement` (anchor added or
    removed without deleting the frame; frames move between the stack and a
    layout's repeating list), `BringToFront`/`SendToBack` (within the layer,
    groups as a block), `Group`/`Ungroup`, `InsertLayout` (fresh-ID copy,
    named "1", "2", …, plus a page break naming it), `ModifyLayout` (left
    pages get copies of the right header and footer; removing them deletes
    the left frames), and `RemoveLayout` (refused while a break names it).
    Range deletion over an anchor deletes the frame and its flows; a single
    Delete or Backspace onto a frame anchor does nothing (V05), in the
    command and in both editors (the default editor maps a single delete of
    a placeholder to `DeleteCharacter` and restores the editor).
  - Undo property test: 40 seeded sequences of 80 random commands over
    text, objects, frames, groups, and layouts; every root validates;
    undo-all restores the original root exactly and redo-all the final one;
    the test asserts that all 14 command kinds were exercised.
  - Importer: styles are resolved before the document is built, so imported
    header and footer text starts in Body Text (33 corpus files lacked Body
    Text until the fallback was added, which broke their v2 round trip
    during development).
  - Native v2 still saves documents whose only frames are the default
    header and footer frames; anything more is refused until v3.
  - Gates: `tools/verify.sh` PASS (core 77 + 1 ignored; app 74 and 81, 2
    ignored each); `tools/corpus.sh` PASS with counts unchanged (160/160,
    160 round trips, 63/63 STY, 28,748 paragraphs, 11,147 runs).
- 2026-10-11: 2.3d done.
  - `docs/Design/NATIVE_FORMAT_V3.md` written first: record grammar,
    escapes (`\\`, `\t`, `\n`, `\r`), section order, how each child record
    attaches to its parent, the rebuild and validation steps, and a worked
    example that a test checks byte for byte against the encoder.
  - `crates/document/src/native.rs`: encoder and strict decoder for
    `AMIPRO-LINUX/3` (IDs and counter, styles, document legacy data,
    base64 `blob` chunks of 76 characters, layouts with pages, boxes, tabs,
    page lines, slots, and repeating lists, frames with placement, look,
    border, shadow, box, tabs, name, macro, content, and legacy data, the
    stack, flows, paragraphs, runs, inline objects, and marks). Payloads no
    picture refers to are dropped at save (`drop_unreferenced_objects`).
    v1 and v2 still load; the app now writes v3 (`NATIVE_MAGIC`).
  - Tests: exact round trip of a document with every model feature (and
    deterministic re-encoding); dropping unreferenced payloads; the
    specification example; 13 kinds of malformed input rejected (unknown
    record, dangling frame and flow IDs, broken base64, wrong payload
    length, wrong table shape, a frame stacked twice, a missing tab list, a
    counter below the IDs, a bad escape, a record out of order, a malformed
    boolean, a mark past its paragraph) and truncated inputs that never
    panic; v1/v2 loading and saving again as v3; the undo property test now
    also round-trips every generated document through v3; the app's
    save-and-reopen test is exact again.
  - Gate: `tools/verify.sh --release --corpus` PASS (core 80 + 1 ignored;
    app 74 and 81 with 2 ignored each; release builds; corpus 160/160 with
    160 exact round trips through v3, counts unchanged).
- 2026-10-11: 2.4 done (`1343b63`).
  - New `crates/ami-formats` modules: `structure.rs` (tab-nested sections
    around unindented text bodies; frames, layouts, tables, pictures, and the
    `[Embedded]` directory with trailer and bounds checks), `text.rs`
    (paragraph parsing with anchors through one numbering, breaks, floating
    headers and footers, page-number fields, `<**>` and `<*7>`, dropped
    `<:#…>` caches, and every other control as a legacy mark), and
    `build.rs` (mapping to layouts, frames, groups, tables, pictures, and
    inline objects; unknown fields and sections stay with their object). The
    import report gains `structure` counts.
  - Corpus (`tools/corpus.sh`, which now also fails on unresolved anchors and
    control leaks): 160/160 imported, 160 exact v3 round trips, 63/63 STY;
    28,748 main-text paragraphs (unchanged); 11,152 runs (was 11,147: five
    runs whose only content was a control that was dropped now cover the
    hard return or object that replaces it); 774 frames (500 header and
    footer, 214 anchored tables of which 189 With para above and 25 Flow with
    text, 22 Where placed and 2 repeating text frames, 36 pictures); 170
    layouts (10 documents have two); 223 anchors resolved, 0 unresolved; 1
    anchored record without a mark (AP3CH02.SAM, placed on its page); 34
    floating headers and footers; 16 page breaks; 1 page-number field; 72
    embedded payloads; 36,846 legacy marks kept; 26,376 caches dropped; no
    page with non-uniform stored columns; unknown frame flag bits `0x200`,
    `0x10000`, `0x100000`, and the new `0x8000000`; 0 control leaks. The two
    "control-like" paragraphs are literal `<<` text (checked in the source).
  - Corpus findings fed back into the specification and notes: the `[tbl]`
    header is rows, columns, and default sizes, with `[h]`/`[w]` records;
    covered cells have no text; calculated cells write their formula as a
    second body.
  - Tests: parser and reader unit tests; a hand-written synthetic fixture
    (`tests/fixtures/pages-and-frames.sam`) covering every structure; a
    truncation and garbling test that must never panic; and the private
    VM-fixture test (`AMIPRO_VM_LAB`): F01's rectangle, G21's two layouts
    and the break naming layout 1, V20's shared anchor numbering, P02's
    9,270 picture bytes and 64,646-byte rendering, V17's footer flags, TB1's
    cells, G15's left pages, Z04's group, and v3 round trips of all eight.
  - Gates: `tools/verify.sh --release --corpus` PASS (core 92 + 1 ignored;
    app 74 and 81 with 2 ignored each). A/B against `eb7eed7`
    (`target/bench-ab/20261011-235258`): no median regression; one p95 flag
    (default editor, backspace at the fixture start, round 2 only above the
    threshold) is noise under the rule.
- 2026-10-11: 2.5a done. `experiments/page-layout-spike` breaks lines over
  COSMIC Text `ShapeLine` words with a width per line, U+FFFC boxes, U+2028
  forced breaks, and U+00AD hyphens. It matched COSMIC Text's own wrapping
  exactly on README31 (337 paragraphs, 4,457 lines at 624, 300, and 120 px)
  and on Hebrew and Arabic runs, kept ligatures within words, and broke all
  344 paragraphs in 0.57–0.70 ms (COSMIC Text layout 1.5 ms; shaping 14–17
  ms once). No case failed, so Parley was not tried. Report:
  `docs/research/LINE_BREAKING_SPIKE.md`; decision: `DECISIONS.md`, "Line
  breaking over COSMIC Text words".
- 2026-10-11/12: 2.5b done (`df0c48b`).
  - `amipro-layout`: `Shaper` trait (words in logical order ending at break
    opportunities, classified for objects, forced breaks, and soft hyphens),
    `FixedShaper` (120-twip characters at 12 pt), `next_line` (the 2.5a
    breaker, one line at a time, so each line takes the width its position
    allows), rule-L2 `visual_order`, `ShapeCache` (per paragraph ID, reused
    while the paragraph is the same shared value or has the same text and
    runs in the same style), and `layout_document`, which returns pages with
    columns, frame boxes (layer, bounds, interior, table cells), and lines
    (container, flow, paragraph ID and index, byte range, words in visual
    order with x, page-number field text).
  - Rules and the fixtures their tests cite: page breaks (G20, V01),
    inserted layouts from the next page (G21), centring breaks (V23), the
    page count including the highest Where placed page (V11, X06), columns
    and column breaks (G05), balance (G06), left pages and mirroring (G15,
    G16), headers per side, from the second page, and floating overrides
    (G15, G19, G22, V17), page-number fields (G18), repeating frames below
    page frames in stack order (F08, F09, Z01, V14), With para above frames
    (F06, V12, and D7 as refined), Flow with text frames (X02, V13), wrap
    around, through, and beside (F01, F04, F05, V14), header text wrapping
    around page frames (V14), frame interiors with derived columns and the
    corner inset (F01: 57 twips; F03; C01), a table grid (TB1), and display
    list identities.
  - Inferred, to check in 2.5d or with new fixtures: which pages column
    balance applies to (here the last page of a layout's run); Repeat
    right/left mirroring on left pages; text beside a wrapping frame takes
    the wider side, and a slot narrower than 0.25 in is skipped; With para
    above frames take vertical space in the column; the 43-twip cell inset;
    the header from the second page of each layout's run.
  - The corpus audit also lays out every document with the fixed shaper:
    1,585 pages in 0.85 s (debug build), most 57 (CH2.SAM).
- 2026-10-12: 2.5c done (`8bb1216`).
  - Engine: each page records where its main text starts (paragraph index
    and ID, word and byte, layout and pending layout, cursor, the page
    frames already on it, whether the paragraph keeps its place under V12,
    and the floating-header and layout-run counts). `IncrementalLayout`
    compares the new document with the previous one (`Document::
    layout_changes`, pointer-first): styles, layouts, frames, the frame
    stack, embedded objects, or the set of flows changed → full layout;
    only other flows → headers, footers, and frame interiors again over
    the kept main-phase pages; main text changed → restart at the last
    page that starts before the first changed paragraph on a fresh page
    (else page 1), and stop at the first page whose start equals the old
    one with only unchanged paragraphs after it. The old tail is spliced
    in, with main-text paragraph indexes shifted by the change in count.
    The V12 rollback now also clears a page start it made, and snapshots
    only count lines from the cursor's page on.
  - Tests (`crates/layout/tests/incremental.rs`): every result equals a
    fresh `layout_document` after typing, split, join, multi-paragraph
    delete, a 300-word insertion and its removal, direct formatting,
    paragraph styles, edits at the start, in balanced columns, and at the
    end, a style definition, margins, page size, header text, undo and
    redo through `History`, and 8 seeds × 120 random edits, undos, and
    redos over a rich and a crowded document (5,191 V12 retries, 676
    convergences, 50 of them inside a paragraph).
  - App: `page_layout.rs` shapes with COSMIC Text at twips (Times New
    Roman, Arial, Courier New → Liberation Serif, Sans, Mono; installed
    families as named; otherwise by kind), ignoring fallback glyphs for
    U+2028, U+00AD, and U+FFFC. Each session lays out on open and after
    every semantic change; the font system's version invalidates the
    cache. `chrome::status_bar_model` shows the caret's page.
  - Benchmark (`page_layout_performance_baseline`, ignored, release; two
    runs, host under load):
    | Document | Paragraphs | Pages | Full layout (cold) | Relayout after typing | Shaping the edited paragraph | After Enter |
    |---|---|---|---|---|---|---|
    | README31 | 344 | 17 | 19.5–24 ms | 0.093–0.098 ms median, 0.105–0.110 p95 | 0.037–0.039 ms (97 bytes) | 0.25–0.28 ms median, 0.29 p95 |
    | AP31NAV | 315 | 32 | 6.2–6.7 ms | 0.075–0.078 ms median, 0.085 p95 | 0.043–0.046 ms | 0.21–0.23 ms median, 0.40–0.43 p95 |
    | MERCURY | 153 | 11 | 7.0–7.5 ms | 0.150–0.159 ms median, 0.164–0.168 p95 | 0.091–0.096 ms (255 bytes) | 0.13–0.14 ms median |
    Target (≤ 1 ms median, ≤ 4 ms p95 on README31) met. Shaping the
    edited paragraph is the largest part of a typing relayout; after
    Enter, shifting the paragraph indexes of every reused page (a copy
    of each) dominates.
  - Gates: `tools/verify.sh --release` PASS (core 117 + 1 ignored; app 77
    and 84 with 3 ignored each). A/B against `141a000`
    (`target/bench-ab/20261012-003014`): the default editor holds except
    fixture-middle backspace and undo (+0.05–0.07 ms median), and the
    rich-editor controller's edits rise from 0.23 to 0.30–0.34 ms median
    (undo/redo 0.12 → 0.20 ms). Accepted: this is the incremental page
    layout that every edit now runs, measured above; the view no longer
    runs the old `paginate` estimate per frame.
- 2026-10-12: 2.5d done (`412f9c7`).
  - Oracles (`crates/ami-formats/src/oracle.rs`, comparison only): `[pg]`
    records are the page's last line (1-based paragraph, line start and
    length in Ami Pro's internal characters, flags, layout name);
    `<:#h,w>` gives paragraph height (without paragraph spacing) and the
    column width; frame fields 1 and 3–6 and `[frmlay]` column edges; each
    style's `[spc]`.
  - Report (`app/cosmic/src/layout_comparison.rs`, ignored, release):
    line counts at Ami Pro's widths, line heights, page counts, page ends,
    anchored frames, and text-frame columns, as imported and with Ami Pro's
    paragraph spacing applied (`AMIPRO_COMPARE_DETAIL` prints one file).
  - VM (operated and left saved): LH1 (new fixture: 30 styles, Times New
    Roman and Arial 8–72 pt, Courier New, bold and italic) gave Ami Pro's
    single line heights; AP31NAV opened with the ruler showed Body Text's
    2,088-twip right indent (the last value of its `[tag]`). The VM prints
    to the Microsoft XPS Document Writer.
  - Fixes: the shaper adds each font's line gap (`DECISIONS.md`, "Single
    line spacing"); table frames take their size from the rectangle
    (field 16 is stale for tables; TB1 1 page as in Ami Pro, README31 15
    pages instead of 17, corpus fixed-shaper layout 1,465 pages instead of
    1,585); a Flow with text frame's right shadow takes space after it
    (Z05, new rule test). The synthetic fixture's table now writes
    `0 65535` like Ami Pro, with its embedded offsets recomputed.
  - Results, fixtures: line counts 766 of 815 equal, all within one; line
    heights 2.0% mean for single spacing; page counts 90 of 92 equal (91
    with spacing); page ends 15 of 65 on the same page as imported, 27 of
    65 with spacing (26 of 26 outside AP31NAV); text-frame columns 52 of 59
    exact, the rest within 9 twips; anchored frames within the per-line
    residual (16–24 twips on short fixtures with spacing).
  - Results, corpus: line counts 85% equal, all within one; page counts
    16% equal. 18,937 of 20,474 compared paragraphs use styles from the
    missing `SMARTEXT.STY`/`PRO21.STY` (23% height difference); single
    spacing 2.4%, other modes 4.3%.
  - Known deviations for Phase 3: paragraph spacing, exact line spacing,
    indents (including the `[tag]` right indent), and `[brk]` rules are not
    imported; printer hinting (Times New Roman 1.5–3% taller at text sizes,
    Courier New 7% shorter) is not emulated.
- 2026-10-12: 2.6 automated parts done (`2c9a38e`); the run stops here.
  - `page_view.rs`: one transform (pixels per twip, page rectangles, content
    size) for drawing, hits, the caret, selection, IME, the ruler, and the
    scroll extents. Pages stack with 24-pixel gaps, centred when they fit and
    anchored to the padding when not; Fit width stops at 100% and 25%.
  - `page_editing.rs`: a selection in one flow with its page instance;
    logical moves; selected text; Insert, Backspace, Delete (single deletes
    are `DeleteCharacter`, V05), and Enter as commands in the caret's flow.
  - `page_surface.rs`: the widget. Paper, layout frames with their text,
    main text, then page frames in stack order with their text (V14); frame
    fill, lines, and shadow; table grids; labelled picture placeholders (no
    BMP or `.X3` decoder among the app's dependencies; recorded as a gap);
    header and footer frames never filled (README31 stores a black opaque
    background). Each line is Iced paragraphs (one per run between objects,
    page-number fields as their own) at the engine's x; the same paragraphs
    give hits and carets. Lines are cached by paragraph value; pages the
    layout reused keep their caches. Hits choose the front-most text frame,
    a header or footer band (with its page instance), a table cell, or the
    main text column. Up, Down, Page Up, Page Down, Home, End, Ctrl+Home, and
    Ctrl+End use the layout; Esc returns to the main text at the frame's
    anchor or the page's first line. A ruler widget uses the same transform.
  - App: a runtime flag selects the page surface in feature builds, so the
    default editor's and the prototype's tests are unchanged in every
    build. Commands (Cut, Copy, Paste, Select All, Delete, Undo, Redo, Bold,
    Italic, styles) act on the page selection in any flow; the hidden text
    editor follows main-text selections for the action bar and history; the
    caret scrolls into view. Zoom commands are implemented only with the
    feature (registry `page_surface_view`).
  - Screenshots (tiny-skia, ignored test `page_surface_screenshots`) were
    reviewed: they showed the black README31 header band, a paper edge
    hidden by full-width frames, and clipped status-bar zoom buttons, all
    fixed before the commit.
  - Gates: `tools/verify.sh --release` PASS (core 118 + 1 ignored; app 77
    default and 102 all features, with 4 and 6 ignored); page-surface-only
    tests 95 pass. A/B against `18c501c` (`target/bench-ab/20261012-012548`,
    2 rounds): no regression in the default or prototype editors.
  - Benchmark (`page_surface_typing_performance_baseline`, release,
    headless, README31, 15 pages, 1280×800): typing 0.415/0.455/0.479 ms
    median at start/middle/end (p95 0.44–0.49), of which update 0.12–0.18,
    view 0.14, layout 0.11, draw 0.02; Backspace the same; Undo 0.41–0.51 ms
    median.
  - The checklist (`UI_ACCEPTANCE.md` §10) and the frame-interaction
    proposal (`UI_SPEC.md` §11) are committed. The VM is saved.
- 2026-10-12, after the run, at the user's request: MERCURY.SAM compared in
  the XP VM with the page surface (side-by-side images outside Git in
  `../AmiPro-Reference/vm-lab/screens/mercury-compare/`). Two defects fixed
  (commit "Place tables and With para above frames as Ami Pro does"):
  - Importer: a table frame's offset comes from its stored rectangle, not
    field 15 (MERCURY writes 20000 for a centred table). Corpus table |dx|
    went to 0 where the column is known (30INTRO 2,443 → 0, CH7 1,661 → 0).
  - Engine: No wrap around With para above frames no longer push the text
    after them; frames after text stand before the space after; a paragraph
    holding only anchors takes no height (the page surface keeps its caret
    visible). MERCURY's stored icon positions follow exactly from these
    rules. With Ami Pro's spacing MERCURY now has 8 pages and 7 of 7 page
    ends (was 9 and 4); its anchored |dy| went from 9,987 to 316 twips.
  - Tests: an importer test for table offsets, a rule test
    (`text_runs_beside_with_para_above_frames_without_wrap_mercury`), a page
    surface test for the caret and clicks on an anchor-only paragraph; the
    F06/V12 rule test and the incremental tests now use No wrap beside, the
    fixtures' real mode. All fail without the fix where they should.
  - Corpus page ends: 198 of 1,306 as imported (was 202), 218 with spacing
    (was 219); the losses are in documents with missing style sheets.
  - Gates: `tools/verify.sh --release` PASS (core 120 + 1 ignored; app 77
    default and 103 all features, 4 and 6 ignored); `tools/corpus.sh` PASS.
    A/B against `82f7552` (`target/bench-ab/20261012-105608`, 3 rounds): no
    regression; default-editor typing and undo medians within 0.01 ms of the
    base on README31, rich editor equal. (A first 2-round run during heavy
    emulator load was inconclusive in both directions.)
- 2026-10-12, after the run, at the user's request: README31.SAM compared in
  the XP VM (images in `../AmiPro-Reference/vm-lab/screens/readme31-compare/`).
  Line breaks agree (332 of 336 paragraphs); page ends drift from page 5 by
  spacing (Phase 3). Three table gaps fixed, at the user's request (commit
  "Lay tables out with joined cells, cell margins, and Table Text"):
  - Joined cells: `CellSpan` on each cell (model, validation, native v3,
    importer from cell field 3, engine, page-surface hit testing).
  - Cell margins: `CellMargins` on each table from header values 6 and 4;
    the engine's fixed 43-twip inset is gone. TB1, V20, and README31 tables
    get Ami Pro's stored sizes exactly.
  - Table Text for untagged cell text (user's guide), with no space after
    until `[spc]` is mapped.
  - An anchored table grows to fit its text when it fits in the rest of the
    column; a cell edit relays the main text from the table's anchor
    (incremental). Growing tables that do not fit made the corpus worse
    (CH2.SAM 49 pages against Ami Pro's 25) because Ami Pro continues them on
    later pages; splitting tables is a recorded gap.
  - Tests: model validation and native round trip with old records, an
    importer test, a rule test, an incremental test, and a page-surface click
    test; the TB1 rule test now uses cell margins.
  - Comparison: VM fixtures unchanged; corpus page ends 217 of 1,306 (was
    198), 235 with spacing (was 218); equal page counts 28 (was 26).
  - Gates: `tools/verify.sh --release` PASS (core 124 + 1 ignored; app 77
    default and 104 all features, 4 and 6 ignored);
    `tools/corpus.sh` PASS (fixed-shaper corpus layout 1,475 pages, was
    1,465). A/B against `c48f3ea` (`target/bench-ab/20261012-114827`, 3
    rounds, a busy host): no regression; README31 typing and undo medians
    equal or lower, empty-document typing within the rounds' spread.
- 2026-10-12, after the run: the user approved the frame-interaction
  proposal (`DECISIONS.md`) and reported a short physical check. Fixed (commit
  "Move the caret past trailing spaces on the page surface"):
  - The caret stood still at the space bar: spaces that end a line hang
    without word boxes, and the page surface placed the caret from word
    boxes only. They now get caret positions and selection width (test
    `the_caret_moves_past_trailing_spaces`, which fails without the fix).
  - Measuring MERCURY showed that the table work had doubled typing time in
    its middle (0.69 → 1.30 ms median): each relayout measured the table
    twice. The engine now measures each table once per layout; interleaved
    against `c48f3ea`, middle 0.66 against 0.65 ms and end 0.50 against 0.49
    ms median.
  - The user supplied a new private corpus (not personal or confidential):
    `../AmiPro-Reference/Charlie-Sample/SAM` (423 SAM files in 24 folders; 417
    are format version 4, Ami Pro 3.x) and `.../Charlie-Sample/STYLES` (192
    STY files), plus 82 Lotus Word Pro `.LWP` files and a few other formats.
    Every SAM's styles resolve (302 embedded only, 121 from the STYLES
    folder) and 396 carry Ami Pro's `[pg]` layout cache, so this corpus can
    serve as the layout oracle the ISO corpus could not. Not yet run through
    the audit or the comparison.
  - Gates: `tools/verify.sh --release` PASS (core 124 + 1 ignored; app 77
    default and 105 all features, 4 and 6 ignored); `tools/corpus.sh` PASS.
- 2026-10-12, new session: the user's corpus run through the audit and the
  layout comparison (handoff step 1; `LAYOUT_COMPARISON.md`, "The user's
  corpus"; commits `1f1dc98`, `d388081`, `f1fb9d9`).
  - Audit (`AMIPRO_CORPUS` and `AMIPRO_STYLES` set to its folders): 416 of
    423 import and round-trip exactly through v3; 192 of 192 style sheets
    parse; 120 of 120 named style sheets resolve; 0 unresolved anchors, 0
    control leaks. The 7 that fail are 5 Lotus Word Pro files named `.SAM`,
    1 password-protected document, and 1 truncated file, now refused with
    messages that say so; the gate fails on this corpus by design.
  - Importer: a `STYLES` directory in any letter case and an explicit
    style-sheet path; a style sheet whose payload lost its NUL bytes still
    parses.
  - Shaper: installed metric-compatible faces (P052, C059, URW, Nimbus Sans
    Narrow) for uninstalled ones; Book Antiqua 69% → 80% of line counts.
  - Comparison: 57% of page counts equal (76% with Ami Pro's spacing), 91%
    of line counts, 61% of page ends with spacing. With the styles' `[algn]`
    and right indents, 97% of line counts agree here and 812 of 815 in the
    VM fixtures: indents are Phase 3's largest gain.
  - Gates: `tools/verify.sh --release --corpus` PASS (core 127 + 1 ignored;
    app 78 default and 106 all features, 4 and 6 ignored; ISO corpus
    unchanged).
- 2026-10-12, new session: frame interaction implemented (`UI_SPEC.md` §11,
  "Implementation notes"; commits `0e5f3f7`, `9319efd`).
  - Editing: `History::amend` puts a command into the last undo step, for
    gestures that issue several commands and for arrow presses within a
    second of each other.
  - `frame_editing.rs`: the frame selection (one frame, a group, or Where
    placed frames on one page), handles per placement, move and resize
    commands with Ami Pro's constraints and minimum size, mirrored Repeat
    right/left frames, hits, Tab order, and text entry.
  - `page_surface.rs`: press, drag, release, Esc, keys, outline, handles,
    dashed preview, and cursors; the caret and IME are off while frames are
    selected.
  - `frame_inspector.rs` and the rail: Frame Properties in place of Styles
    while one frame is selected, fields in Modify Frame Layout order, each
    applied as one command on Enter or focus loss; refusals shown in place.
  - Registry: Select Next Frame (Ctrl+G with the page surface; Go to Page,
    still future, keeps it in the default build), Bring to Front, Send to
    Back, Group Frames, Ungroup Frames, and Frame Properties; disabled in
    the default build.
  - Measurements (release, headless, README31, 15 pages): a drag's pointer
    move with the redraw recorded 0.006 ms median, 0.016 ms worst (a
    picture frame on page 12; the fixture's text frame 0.010 ms); a move's
    release (command, relayout, view, layout, draw) 1.87 ms median, 2.07
    p95. GPU rasterisation is not measured; the physical check covers it.
  - A/B against `e90ced1`: `tools/bench-ab.sh` 3 rounds
    (`target/bench-ab/20261012-130330`): one p95 flag (default editor,
    typing at the fixture end) from round 3, in which every B value rose
    about 40% including the empty document; noise. The page-surface typing
    baseline, interleaved 4 rounds twice: the second, quieter run is within
    0.02 ms everywhere (start, middle, end; typing, Backspace, Undo). Both
    builds measure 0.52–0.68 ms median, above the 0.42–0.48 ms recorded at
    `2c9a38e`; that rise predates this work (A equals B).
  - Visible in the default build: five more disabled menu items (Edit ›
    Select Next Frame; Format › Bring to Front, Send to Back, Group Frames,
    Ungroup Frames).
  - Gates: `tools/verify.sh --release` PASS (core 128 + 1 ignored; app 78
    default and 121 all features, 4 and 8 ignored).
- 2026-10-12: VM fixtures (the VM operated; left saved).
  - I1–I3: Body Text's indents in Modify Style › Alignment. `[algn]` value 1
    is flags (1 left, 2 right, 4 centre, 8 justified, 16 Both sides, 512
    Hanging indent); values 3–5 are All, First, and Rest in twips, First and
    Rest added to All (the first line at 1.5 in and the others at 0.75 in for
    720, 1440, 360); the right indent is the tag's last value; Both sides
    copies All into it. VERIFIED; Phase 3 can map indents.
  - F14–F20: frame gestures (`SAM_FORMAT_NOTES.md`, "Frame gestures").
    Where placed and With para above moves and resizes give Ami Pro's stored
    rectangles, offsets, and sizes to the twip. Flow with text frames did
    not: field 16 holds the top-to-baseline distance, not the height, and a
    frame dragged down hangs below the baseline. Fixed in `5694a8d`
    (importer, engine, `ResizeFrame`, and the With para above default wrap,
    No wrap beside as Ami Pro chooses); tests: importer (synthetic and
    private fixtures), rule `f18_f20`, editing.
  - Gates: `tools/verify.sh --release --corpus` PASS (core 130 + 1 ignored;
    app 78 and 121, 4 and 8 ignored). Comparison: fixtures 100 of 102 page
    counts, 778 of 820 line counts (817 with indents); F16–F20 anchored
    within 16–24 twips with spacing.
- 2026-10-12: Phase 3 begun (styles), measured against the user's corpus.
  - Indents (`7076f36`): VM fixtures I1–I3; `ParagraphStyle::indents`, native
    v3 (optional fields), importer, engine. Equal line counts: fixtures 817 of
    820, user's corpus 97%, ISO 87%. Generated styles take Body Text's font
    but not its indents (Table Text, 44 of 49 user style sheets).
  - Spacing (`60e8e2c`): VM fixtures S1–S6 decoded `[spc]` (flags for the
    line spacing and Add in; fixed line heights split above and below; space
    above left out at page tops unless Always; Text Tightness). The import
    now carries it, and the report's "with spacing" variant is gone. As
    imported: user's corpus 76% of page counts and 59% of page ends (were
    57% and 46%); README31 13 of 13 and MERCURY 7 of 7 page ends; ISO 35
    page counts and 247 page ends. Undefined styles get 6 pt below (better on
    the ISO corpus than Body Text's spacing).
  - The VM locked at the welcome screen after idling (password unknown);
    recovered by a guest restart and set to never blank (`VM_LAB.md`).
  - Gates: `tools/verify.sh --release --corpus` PASS (core 135 + 1 ignored;
    app 78 and 121, 4 and 8 ignored).
  - A/B against `5694a8d` (page-surface typing, 4 interleaved rounds, quiet
    host): start and end of README31 within 0.005 ms; the middle +0.07 ms
    (0.463 → 0.530 ms median). Cause, found with a temporary probe: in the
    new layout the benchmarks' paragraph (172) is the first on page 8, so
    relayout restarts at page 7 (an edit could move it back) and sets two
    pages; typing in paragraphs 100 or 250 sets the same pages as before.
    Within the 1 ms budget; not an engine regression.
  - Break rules (`2031b8e`): VM fixtures B1–B8 decoded `[brk]` (page and
    column breaks before and after, breaks within, keep with previous and
    next, Next style); the engine applies them, and the incremental property
    test found that a layout must not converge at a kept paragraph. VM
    fixtures 75 of 75 page ends (AP31NAV 39 of 39, was 3).
  - With para above frame heights (`6095d8e`): field 16's height goes stale;
    the rectangle is what Ami Pro lays out (a manual opened in the VM).
    Two manuals that ran long now match. The earlier explanation (taller
    lines) was wrong and is corrected in the records.
  - Paragraph overrides (`0daf07b`): VM fixtures O1–O7 and LS1 decoded every
    Text menu control; `<:I…>` indents and `<:S…>` line spacing are imported
    and laid out (`ParagraphFormat`). User's corpus 686 page ends (was 662).
  - `AMIPRO.EXE` decompiled (`82d1834`, user's suggestion): OPTLOADER
    unpacking, 3,953 functions (`docs/research/DECOMPILATION.md`).
  - Alignment (`003d345`): `[algn]` and `<+@ABC>`; right, centre, and
    justified lines; the page surface draws justified words one by one.
    No page-end change (as expected); page-surface typing A/B on a justified
    manual: no regression.
  - Gates at `003d345`: `tools/verify.sh --release --corpus` PASS (core 145
    + 1 ignored; app 78 and 121, 4 and 8 ignored).
  - Next: the decompile audit, then character overrides
    (`docs/SESSION_HANDOFF.md`, "Next steps").
