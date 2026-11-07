# Decision Log

## 2026-10-09 — Reference path

Use the task-specified repository-local `Reference/` directory for this session, overriding the general sibling-directory default. Ignore it entirely in Git.

## 2026-10-09 — Editing architecture

Adopt the hybrid semantic-model architecture in `ARCHITECTURE_DECISION.md`. The Iced text editor is a temporary vertical-slice input surface, not the document source of truth.

## 2026-10-09 — Legacy writes

Keep SAM/STY support read-only until all parsed and unknown structures have explicit preservation/loss behavior. Never overwrite original input during discovery.

## 2026-10-09 — Word-processor engine reuse audit

Retain the project-owned semantic model and native COSMIC Text/Parley-based architecture. LibreOffice Writer, AbiWord, Calligra Words, Qt `QTextDocument`, Typst, and ONLYOFFICE do not expose an editing/layout boundary that provides enough benefit to offset model duplication, toolkit/FFI coupling, licensing, or maintenance cost. The full matrix and reconsideration gates are in `WORD_PROCESSOR_ENGINE_REUSE_AUDIT.md`.

The source-level addendum for WordCraft and rdocx does not change this decision. WordCraft is the closest reusable design found, but its layout, hit testing, editing session, and rendering identities are coupled to its own young 0.3 model/stack. rdocx has a mature-looking, extensively tested batch layout boundary, but it accepts WordprocessingML semantics and does not expose an interactive editor contract. Monitor both as prior art; do not add a dependency or initiate a migration.

## 2026-10-09 — Rich editor surface boundary

The pinned Iced editor cannot display per-span metrics, underline, or paragraph spacing through its public API, even though its internal renderer uses COSMIC Text. Implement a model-backed custom surface incrementally as described in `../Design/RICH_EDITOR_ARCHITECTURE.md`; never overlay or toggle a second formatted view.

## 2026-10-09 — Rich editor shaped-layout cache

Retain complete shaped paragraphs in the Iced widget tree and key them by document-session identity, an explicit semantic layout generation, display scale, resolved bounds, and renderer text defaults. Selection, caret, focus, and outer scrolling reuse that derived geometry; text edits, formatting/style changes, undo/redo, width/scale changes, and tab identity invalidate it. This follows the persistent ownership and dirty-state patterns in the pinned COSMIC Text 0.19 and libcosmic/Iced sources while avoiding a second editable buffer. Defer per-paragraph deltas and Xi-style viewport wrapping until measurements show that cold edit-time shaping is the next bottleneck. Full evidence is in `RICH_EDITOR_INCREMENTAL_LAYOUT.md`.

## 2026-10-10 — Rich editor edit-time localization

Keep the accepted semantic `Document` and command/history boundary. In the
feature-gated surface, retain the hidden Iced content and synchronize it from
the semantic old/new text with one UTF-8-safe delta; do not rebuild it or the
unused `DocumentLayout` after each edit. On generation changes, compare cached
paragraph source/style inputs and reshape only changed paragraphs, with a
conservative full rebuild for paragraph-count, session, geometry, scale, or
renderer-font changes. This measured correction removes the actual redundant
work without adding a rope, new engine, custom GPU path, or dependency upgrade.
Deriving the fallback delta from the semantic result is required because Iced's
grapheme deletion and raw newline handling are not identical to the model's
Rust-character and CRLF/CR normalization semantics. Evidence and limitations
are in `RICH_EDITOR_TYPING_PERFORMANCE.md`; physical COSMIC acceptance remains
separate.

## 2026-10-11 — Default editor edit-time work

Remove the default build's per-session derived `DocumentLayout`. It was rebuilt
and reshaped after every edit, yet only a `debug_assert!` consumed it.

Use the semantic delta synchronizer (now `synchronize_editor_text`, shared with
the feature-gated rich editor) for undo/redo instead of rebuilding the visible
Iced content.

Release end-to-end measurements of `README31.SAM` show typing going from
52–55 ms to 0.58–0.62 ms median, and undo/redo from 79–86 ms to 0.43–0.67 ms.
An interleaved A/B shows no rich-editor regression.

No cache, rope, or dependency was added. Evidence and limitations are in
`DEFAULT_EDITOR_TYPING_PERFORMANCE.md`; physical COSMIC confirmation remains
separate.

## 2026-10-11 — Pages and frames move ahead of styles and UI-3

Ami Pro is a page- and frame-based, desktop-publishing-style word processor.
Each page has one page layout from the style sheet or an inserted layout, and
frames are mini-documents (text or picture) placed on the page with their own
layout, anchoring, and wrap (user direction; 3.1 user's guide chapters 8 and
14–16). The current model is a single paragraph flow with one `PageSetup`, and
both editors present one continuous flow.

Selection, positions, undo, pagination, hit testing, and SAM fidelity all depend
on how pages and frames are modeled, so the page, page-layout, and frame
foundation is now `PROJECT_PLAN.md` Phase 2. It follows UI-2 physical acceptance
and the SAM import-fidelity fix, and precedes the UI-3 Styles workflow, UI-4
workspace/zoom (now delivered with the page surface), style-sheet work, and
promotion of the one-flow rich editor. `AGENTS.md` no longer frames anchored
frames as an extension of flowing text.

The user also authorized direct operation of original Ami Pro in the
`Windows XP` VM, including snapshots, so fixtures can be authored and compared
without user-run steps.

## 2026-10-11 — Phase 2.1 page and frame format

Controlled fixtures in original Ami Pro 3.1 (XP VM), synthetic read-back
files, the corpus, and a decompiled third-party filter now describe pages,
page layouts, headers/footers, frames, anchors, tables, and pictures well
enough for the model decision. Decisions taken:

- **`<:t n>` is a table anchor, not a tab.** Ami Pro writes tabs as literal
  tab characters (T02) and `<:t0>` for a table in the text (TB1). The
  importer stops emitting a tab for it (regression test added). Corpus
  character runs 11,151 → 11,147; paragraphs unchanged.
- **Unknown fields are preserved, not chased further now.** Frame field 8,
  `[frmlay]` values 3 and 6, and some flag bits are not needed by Ami Pro's
  own conversion filter (`w4w33f.dll`, decompiled) and changed with no
  dialog setting tried. They stay with their records and are written back
  as read.
- **Caches are recomputed.** `[pg]`, `<:#…>`, anchored-frame rectangles,
  frame column edges, and frame sequence numbers are layout output.
- The specification is `docs/research/SAM_PAGE_FRAME_SPEC.md`. The Phase 2.2
  model decision is prepared in `docs/Design/PAGE_FRAME_MODEL_BRIEF.md` but
  **not taken**; it needs the Extra High effort run required by
  `docs/Design/UI_SPEC.md` section 9.


## 2026-10-11 — Phase 2.2 page, layout, and frame model

Decided with Extra High effort (`docs/Design/UI_SPEC.md` section 9), from
`docs/Design/PAGE_FRAME_MODEL_BRIEF.md`, after its research checkpoint
(`PAGE_FRAME_REFERENCE_RESEARCH.md`) and its remaining VM checks (fixtures
V10–V23 and X06, `SAM_FORMAT_NOTES.md`). The resulting architecture is in
`ARCHITECTURE.md`. Implementation is `PROJECT_PLAN.md` Phase 2.3–2.6.

**New evidence that shaped it:**

- A fixed frame keeps its page number. Ami Pro adds empty pages so that the
  page exists, both when text shrinks and when a file is opened (V11, X06).
- Anchored frames that do not fit move to the next page together with their
  paragraph or line (V12, V13).
- Page frames always paint above repeating (layout) frames, and z-order
  commands act within a layer. Header text wraps around frames (V14).
- A group moves and deletes as one undoable action; its members are on one
  page (V15, V16, user's guide).
- `<:t n>` and `<:A n>` share one numbering (V20). A frame holds exactly one
  of text, a table, or a picture (V21).
- Floating headers and footers carry flags (header, footer, left, right).
  Removing one deletes its mark (V17, V18).
- Anchors, breaks, and floating headers occur only in the main text (user's
  guide; all 273 such marks in the corpus).

### D1. Object identity

Typed opaque IDs: `FlowId`, `FrameId`, `LayoutId`, `GroupId`, `ObjectId`
(embedded payloads), and `ParagraphId`. They come from one per-document
counter and are never reused within a document. Native v3 stores them, and
the counter resumes above the largest stored ID. Import assigns them in
document order, so test output is stable. `StyleId` is unchanged.

SAM's own numbers (field 13, `<:A n>`/`<:t n>`, layout names in breaks) are
mapped to IDs at import and regenerated at export.

Paragraph IDs key the layout caches and the convergence test of incremental
pagination. They are not used in positions or commands.

*Why:* SAM numbers are positional. Field 13 changes with z-order, and anchor
numbers change when anchored frames are added or deleted. Positions inside
frames and table cells must survive structural edits elsewhere. Incremental
layout needs to recognise a paragraph after insertions above it. Rejected:
positional indexes (brief option B).

### D2. Document positions and selection

- `TextPosition { flow, paragraph, byte }`, where `byte` is a UTF-8
  boundary within the paragraph. The current `Cursor` is the main-flow case.
- Both ends of a text selection are in one flow.
- `Selection` is either `Text { anchor, focus }` or `Frames(Vec<FrameId>)`.
  Several frames can be selected only if they are Where placed frames on one
  page. An anchored, repeating, or header/footer frame is selected alone
  (user's guide). Selecting any group member selects the whole group.
- A table-cell range is a later `Selection` variant (Phase 5).
- Headers, footers, and repeating frames appear on many pages. Which page
  the caret is shown on is view state, not part of the position.

*Why:* Ami Pro keeps the insertion point in one text stream. Addressing each
flow separately means an edit in one flow never shifts positions in another,
which Writer's single node array cannot guarantee.

### D3. Inline objects in text

Every object that occupies a place in text is one U+FFFC character in the
paragraph text plus an entry in the paragraph's `objects` list, in text
order: the k-th U+FFFC is the k-th entry. The kinds are:

- frame anchors, for both anchored placements, including table frames
  (`<:t>` and `<:A>` share one numbering);
- page and column breaks: kind, the centre flag, and an optional `LayoutId`;
- floating headers and footers: header or footer, all/right/left pages, and
  the `FlowId` they own;
- later, notes, footnotes, and fields. The page-number field (G18) comes
  first, because headers need it.

Editing rules:

- Deleting a range deletes its objects and everything they own (frames and
  flows) in the same command.
- The UI may ask for confirmation, as Ami Pro does. A single Delete next to
  an anchor may do nothing (V05). Both are command-layer policy.
- U+FFFC typed or pasted from outside is never accepted as text.

Some special characters are plain characters, not objects: hard return
U+2028, discretionary hyphen U+00AD, and no-break space U+00A0.

SAM inline controls that are not mapped yet are kept as zero-width *legacy
marks* on the paragraph (byte offset + raw text). They move like run
boundaries, are deleted with the text around them, and are never caret
stops.

*Why:* this is the representation Writer uses (a placeholder plus an
attribute). Byte-range runs, UTF-8 cursors, shaping, and the existing
range-replacement code keep working. The anchor moves with every edit by
construction, and V05/V06 reduce to one rule.

Rejected:

- `Vec<Inline>` paragraphs (option B), which would rewrite every command,
  run, and cursor.
- Separate anchor references (option C), which bring back the shifting bug
  class.

Legacy marks are not objects, because invisible caret stops would break
navigation. They only have to survive for a future SAM writer.

### D4. Document shape

- **Flows.** Every text stream is a `Flow` (a list of paragraphs) in one map,
  with `main_flow` naming the main text. Every other flow has exactly one
  owner: a text frame, a table cell, or a floating header/footer object. The
  owner is derived and validated, not stored.
- **Layouts.** `layouts` is ordered; the first is the document's layout. A
  `PageLayout` has a name, paper code and orientation, right-page geometry,
  optional left-page geometry, mirroring, begin-on-second-page flags, header
  and footer frames for each side, repeating frames back to front, and
  legacy data.
- **`LayoutBox`** serves both page geometry and frame interiors (R7). It holds
  margins, column count, gutter, per-column tabs, balance, and the line
  between columns. Column edges are derived; they are layout output.
- **`Frame`** has:
  - a placement: `Fixed { page, rect }`, `BelowParagraph { x_offset, size }`,
    `Inline { y_offset, size }`, `Repeating { all | right_left, rect }`,
    `Header { side }`, or `Footer { side }` (header and footer rectangles
    are derived from the layout's margins);
  - a wrap mode: around, through, or no wrap beside;
  - opacity, border, shadow, corners, background, and an interior
    `LayoutBox`;
  - an optional group, a name, a macro, and legacy data;
  - content: `Text(FlowId)`, `Table`, `Picture`, or `Legacy` (raw, for
    drawings, charts, and equations until they are implemented).
- **`frame_stack`** lists every top-level frame (fixed and anchored) back to
  front, as SAM writes them. Layout frames paint below the whole stack
  (V14).
- **Groups.** A group is a `GroupId` on its members: at least two Fixed
  frames, all on one page, contiguous in `frame_stack`.
- **`objects`** holds embedded payloads by `ObjectId`, as shared bytes.
  Unreferenced payloads are dropped at save, so undo never has to delete
  them.
- **Units.** Geometry is integer twips, exactly as SAM stores it. Font sizes
  stay in points.
- **Legacy data.** SAM data that is not mapped stays with its object: typed
  fields where the position is known, raw sections otherwise. Document-level
  sections are kept in order.
- **No nesting.** Frames never nest. Anchors and floating headers exist only
  in the main flow, and validation enforces it. "Frames in headers" are
  repeating frames placed in the margin (user's guide, chapter 14). Pictures
  in table cells are cell content (Phase 5).

*Why:* every requirement R1–R10 has exactly one home. Frames are layout
boxes (R7). The shape maps one to one onto the SAM structures, so a future
writer can regenerate them.

### D5. Pages

Pages are layout output and are never stored. The layout walks the main
flow:

- Page 1 uses the document layout. A break that names a layout switches to it
  from the next page. A plain page break keeps the layout, and a column break
  moves to the next column.
- The page count is the larger of the pages the main text needs and the
  highest Fixed page number (V11, X06). Trailing pages use the layout in
  force at the end of the text.
- Fixed frames are placed on their page.
- Anchored frames are placed from their mark. They are clamped to the column
  width and the remaining height, and move to the next column or page with
  their paragraph or line when they do not fit (V12, V13).
- Wrap exclusions apply to main text and to header and footer text (V14).
- Floating headers and footers apply from the page after their mark.
  Right pages are odd.
- Caches are recomputed (R10): `[pg]`, `<:#…>`, anchored rectangles, column
  edges, and sequence numbers.

Incremental layout:

- Shaping is cached per paragraph, keyed by paragraph ID and content, style,
  and width inputs.
- Pagination restarts at the first page whose inputs changed. It stops when
  a page starts at the same paragraph and line, with the same layout and
  floating state, and no later frame or exclusion has changed.
- Header and footer edits repaint pages but never repaginate the main text,
  because headers and footers live in the margins.

### D6. Undo

There is one history per document. Each entry is a complete document root
plus the selection.

- Roots share every unchanged part through `Arc`: flows, paragraphs, frames,
  layouts, and payloads.
- Every mutation goes through typed commands in `amipro-editing`. They
  implement the semantic rules (anchor deletion, group move and delete,
  layout insertion) and report what they touched.
- Undo and redo swap roots. There are no hand-written inverse commands.
- This replaces both current mechanisms: the app's whole-`Document`
  snapshots (100 steps; about 0.07 ms per clone on `README31.SAM`) and the
  unused inverse-command history in `amipro_editing::Editor`. The
  `Editor`'s UTF-8, run, split, and join operations become the text commands.
- Typing coalescing is a UI policy layered on top.

*Why:*

- Undo stays correct by construction across flows, frames, tables, and
  layouts, which is the risk that grows fastest in Phase 2: otherwise every
  new command needs a verified inverse.
- Cost is proportional to the objects touched, plus pointer copies.
- V06 and V15 follow directly: one Undo restores text together with its
  frame, or a whole group.
- The app's snapshot design is the one physically accepted, so behaviour
  stays the same while snapshots get cheap.
- The commands' reports, plus pointer comparison after undo or redo, give
  incremental layout its dirty set.

Rejected:

- Per-command inverses (the brief's recommendation): more code and more
  failure modes, with no gain once snapshots are cheap.
- Persistent-collection crates: no measured need.

### D7. Tables, pictures, and later objects

- **Table** is frame content: column and row specifications, plus row-major
  cells. Each cell has content (`Text(FlowId)` now, a picture later) and
  legacy data, and so does the table.
  - A page table is an anchored table frame with `Inline` placement;
    export writes `<:t n>` for it.
  - A frame table is a Fixed frame with table content (V21).
- **Picture** is frame content:
  - a source: `Embedded(ObjectId)` or `Linked(path)`;
  - the extension and the `[isd]` values, kept as legacy data;
  - Ami Pro's own rendering, kept as a legacy payload so that a future SAM
    writer can re-emit it. It is never displayed and is dropped when the
    picture changes.
- **Drawings, charts, and equations** stay as `Legacy` content until they
  are implemented.

### D8. Native format v3 and legacy data

Keep the strict, line-oriented, tab-separated text format.

- The header is `AMIPRO-LINUX/3`.
- Records cover IDs, styles, layouts and geometry, columns and tabs, frames,
  the stack, groups, flows, paragraphs, inline objects, runs, legacy marks,
  tables, pictures, and legacy records. Embedded bytes are stored as base64
  `blob` chunks.
- It stays one file, so saving remains an atomic temp-file-and-rename.
- v1 and v2 files still load: one main flow, the document layout converted
  from `PageSetup` to twips, and IDs assigned on load.
- Unknown records are rejected, documents are fully validated on load, and
  exact round trip is tested. The 64 MiB limit stays, and embedded payloads
  count against it.

*Why:* it keeps the dependency-free core, the proven strict decoder, and the
atomic save. A single file avoids keeping side files consistent. Base64's
33% overhead is acceptable for typical pictures. A container format can come
with a later version if sizes require it.

### Consequences

- Until the page surface (2.6), both editors stay one-flow editors of the
  main flow and must keep inline objects intact:
  - U+FFFC shows as an object marker;
  - deleting a range applies the object rule;
  - U+FFFC input is rejected.
- `PageSetup` becomes the document layout's right-page geometry.
  `replace_plain_text` is retired once both editors issue commands.
- **Open items (not blocking):**
  - the effect of `<:c>` in multi-column layouts;
  - a multi-line paragraph at a page end with a With para above frame;
  - the paint order of header text against repeating frames;
  - whether frame text wraps around other frames;
  - how Repeat right/left places the frame on left pages;
  - the remaining `<*X>` bytes.

## 2026-10-11 — Page-table placement and the 2.4 import (refines D7)

D7 said a page table is an anchored table frame with Inline (Flow with text)
placement. That came from one fixture, TB1, whose table record has the wrap
bit (`0x1800C4`). The corpus contradicts it: of the 214 anchored table frames
the 2.4 importer reads, 189 have no wrap bit (`0x180044` and similar) and only
25 have it.

Decision:

- A page table keeps the anchored placement its flags give, like any other
  anchored frame: With para above without `0x80`, Flow with text with it.
  Export writes `<:t n>` for every anchored frame whose content is a table.
- `<:t n>` sits at the start of the paragraph after the table, while a With
  para above text frame's `<:A n>` sits at the end of its paragraph (F06,
  TB1, V20). Both are the same position in the text stream. The layout engine
  (2.5b) therefore places a With para above frame after the text that comes
  before its anchor: below the anchor's paragraph when the anchor ends it,
  above that paragraph when the anchor starts it.

Other import decisions taken in 2.4, from corpus evidence:

- A frame record whose parts cannot be read keeps its place in the frame
  stack and in the anchor order, with its raw text as `Legacy` content. One
  corpus table (MERCINV.SAM) needed this before calculated cells were
  understood; dropping it would shift every later anchor.
- An anchored frame record with no mark in the text (one in the corpus,
  AP3CH02.SAM) becomes a Where placed frame on its stored page, with a legacy
  note.
- A layout with left-page header or footer frames but no `[lft]` page (one
  corpus layout, CH10.SAM) gets left pages copied from its right pages, with a
  legacy note, so its headers are kept.
- Header and footer frames beyond one per slot (APPENDIX.SAM repeats left
  slots) stay as raw layout data.


## 2026-10-11 — Line breaking over COSMIC Text words (Phase 2.5a)

Keep the project's own line breaker over COSMIC Text shaped words, as the
2.2 decision planned; do not adopt Parley.

Evidence (`LINE_BREAKING_SPIKE.md`): a 250-line spike broke lines over the
words of a `ShapeLine` with a width per line, U+FFFC inline boxes, U+2028
forced breaks, and U+00AD hyphenation. On `README31.SAM` (4,457 lines at
three widths) and on Hebrew and Arabic runs it reproduced COSMIC Text's own
wrapping exactly (break positions and glyph positions), and it broke all 344
paragraphs in 0.57–0.70 ms against 1.5 ms for COSMIC Text's layout. No case
failed, so Parley was not tried.

Consequences:

- `amipro-layout` breaks lines over a per-paragraph word list supplied by the
  shaper trait. The app implements the trait with COSMIC Text; core tests use
  a fixed-advance shaper.
- Shaping (about 50 µs per paragraph) dominates, so 2.5c caches shaped
  paragraphs and re-breaks them freely.
- Known limits: the engine needs full rule-L2 reordering over the reported
  levels; a ligature across a soft hyphen is lost, as in COSMIC Text.

## 2026-10-12 — Single line spacing (Phase 2.5d)

A line is as tall as its fonts' ascent, descent, and line gap (Windows' text
height plus external leading), with the gap above the text. Before, the
shaper left out the line gap.

Evidence (`LAYOUT_COMPARISON.md`): LH1, a VM fixture with Times New Roman and
Arial at 8–72 pt, gives Ami Pro's single spacing as 2,355 / 2,048 em plus
printer rounding. With the gap, Arial is within ±2 twips at every size and
Times New Roman within 1.5–3% (exact from 36 pt). Without it, 12 pt Times
New Roman lines were 266 twips against Ami Pro's 284.

Consequences: page counts in the status bar follow Ami Pro's line pitch for
single-spaced styles. Exact and other spacing modes, paragraph spacing, and
indents arrive with the style mapping in Phase 3. Ami Pro's printer hinting
(Times New Roman 4–9 twips taller at text sizes, Courier New 7% shorter) is
a known deviation, not emulated.


## 2026-10-12 — With para above frames in the text, and table offsets (refines D7)

The 2.5b engine reserved the height of every With para above frame, put a
frame anchored after text below the paragraph's space after, and gave a
paragraph that holds only an anchor a full line. The importer took a table
frame's horizontal offset from field 15. MERCURY.SAM, compared with Ami Pro
3.1 at the user's request, showed all four to be wrong.

Decision:

- A With para above frame set to No wrap around does not push the text after
  it down: that text starts at the frame's top and runs through it. No wrap
  beside frames and all table frames still take their height.
- A frame anchored after text stands below the text, before the paragraph's
  space after. One anchored at the start of a paragraph stands at its top,
  as before.
- A main-text paragraph whose text is only With para above anchors takes no
  height; its spacing still applies, and its line keeps a baseline so the
  caret shows.
- A table frame's offset is its stored rectangle's left edge, from the left
  of the column it starts in. Field 15 is not an offset for tables.

Evidence (`LAYOUT_COMPARISON.md`, "MERCURY.SAM against Ami Pro's display"):
MERCURY's stored rectangles for its seven icons and its drawing follow
exactly from its cached paragraph heights and `[spc]` spacing under these
rules, and not otherwise; Ami Pro's page 7 shows the text beside each icon,
and its page 3 shows the heading after the table about 180 twips below it.
MERCURY writes 20000 in field 15 for a table shown centred at its rectangle,
and corpus tables at the margin write up to 19,456. The F06 and V12 fixtures,
which the 2.5b rule came from, are No wrap beside, so their behaviour is
unchanged apart from the frame now standing before the space after.

Consequences: with Ami Pro's paragraph spacing MERCURY now has Ami Pro's 8
pages and all 7 page ends; table offsets in the corpus are exact. Text beside
an icon overlaps it until paragraph indents are imported (Phase 3). Where a
table sits relative to the following paragraph's space above is open until
space above is imported.

## 2026-10-12 — Table geometry, joined cells, and Table Text (refines D7 and D8)

README31.SAM, compared with Ami Pro 3.1 at the user's request, showed tables
whose rows grew far taller than Ami Pro's and ran over the text after them.
Three causes, each now handled:

- **Joined cells.** A table carries each cell's span: 1 × 1, rows × columns
  for the top-left cell of a joined block, or 0 × 0 for a covered cell. The
  importer reads them from cell field 3 (`0x100` with `0x80` starts a block
  of fields 4 × 5; `0x80` alone is covered); validation requires every block
  inside the grid and covering exactly the covered cells. Layout gives a
  joined cell the whole block, and a covered cell the block's bounds and no
  text of its own.
- **Cell margins.** A table carries a horizontal and a vertical cell margin
  (`[tbl]` header values 6 and 4). A cell is its column's text width and its
  row's text height plus these; the frame is the cells plus 43 twips. This
  replaces the engine's fixed 43-twip inset, which treated the stored column
  width as the whole cell and added 86 twips to every row of README31's
  table, whose vertical margin is 0.
- **Table Text.** Untagged cell text takes Table Text, created from Body
  Text when the file has none, as the user's guide says Ami Pro does. Until
  `[spc]` is mapped its space after is 0 (Ami Pro generates it with no space
  above or below); the stored row heights already hold Ami Pro's spacing.
- **Growth.** An anchored table's frame is as tall as its rows need when
  that fits in the rest of the column (a Flow with text table: in a column),
  so text after it never runs under it, and a cell edit lays the main text
  out again from the table's anchor. A table that does not fit continues on
  later pages in Ami Pro (80 of 220 corpus tables store one page's part);
  splitting tables is not built yet, so it keeps its stored height.

Native v3 gains the margins on `table` records and the span on `cell`
records; records without them (files written before) load with 86/86 and
1 × 1 (`NATIVE_FORMAT_V3.md`).

Evidence (`LAYOUT_COMPARISON.md`, "README31.SAM against Ami Pro's
display"): TB1 and V20 frames are 2 × (284 + 86) + 43 and 5 × (284 + 86) + 43
twips exactly, README31's 4 × rows + 0 + 43; 140 of 220 corpus tables follow
the rule exactly. All 27 joined blocks in the corpus are consistent. Growing
tables that need more than the rest of the column made the corpus worse
(CH2.SAM 23 → 49 pages against Ami Pro's 25) and is not done.

## 2026-10-12 — Frame interaction approved (user decision)

The user approved the frame-interaction proposal in `docs/Design/UI_SPEC.md`
§11 as written: selection states, handles, moving and resizing, keyboard
access, the Frame Properties inspector, rail switching, and its acceptance
gates. It is the next implementation work on the page surface (Phase 2.6).

## 2026-10-12 — Metric-compatible font substitutes and the style-sheet path

From running the user's 423-file corpus through the audit and the layout
comparison (`LAYOUT_COMPARISON.md`, "The user's corpus").

**Font substitutes.** COSMIC Text's font database does not apply
fontconfig's aliases, so every uninstalled face fell back to Liberation
Serif or Sans by a name heuristic. The shaper now asks for an installed
metric-compatible face first: fontconfig's groups for the PostScript core
fonts (P052 for Palatino, C059 for New Century Schoolbook, URW Bookman, URW
Gothic, Z003, Nimbus Sans Narrow, from `fonts-urw-base35`, plus Liberation
Sans Narrow), and two pairings the comparison supports beyond those groups:
Book Antiqua with P052 and Century Gothic with URW Gothic. Book Antiqua is
the corpus's most common face (4,739 compared paragraphs): its equal line
counts went from 69% to 80% and its line-height difference from 7.1% to 3.8%.
Arial Narrow takes Nimbus Sans Narrow before Liberation Sans Narrow (same
line counts, half the height difference). When none is installed, the name
heuristic applies, now on whole words ("Monotype Corsiva" was drawn
monospaced). Alternatives: bundling fonts (not needed: Pop!_OS installs
these), or fontconfig itself (a new dependency for a table of a dozen
names). The `[fnt]` kind bits (`SAM_FORMAT_NOTES.md`) could replace the name
heuristic when Phase 3 maps fonts.

**Style-sheet path.** Ami Pro reads style sheets from a directory of its own
(`…\AMIPRO\STYLES`, `VM_LAB.md`). The importer keeps looking beside the
document and in a `styles` directory there or one level up, now in any
letter case, and takes further directories
(`import_sam_path_with_style_directories`). The audit and the comparison read
them from `AMIPRO_STYLES`. The app has no setting for it yet: a user's
documents whose style sheets sit elsewhere open with fallback styles, as
before, and the status bar says so.

**Files that are not SAM.** Lotus Word Pro files and password-protected
documents are refused with messages that say what they are, and a style
sheet whose embedded payload lost its NUL bytes still gives its styles
(`SAM_FORMAT_NOTES.md`, "Files that are not ordinary SAM files").

## 2026-10-12 — Paragraph indents (Phase 3, first slice)

Ami Pro's Modify Style › Alignment indents, verified in the VM (I1–I3,
`SAM_FORMAT_NOTES.md`): `[algn]` values 3–5 are All, First, and Rest in
twips, First and Rest added to All; the style's last trailing value is the
right indent. The model gains `ParagraphStyle::indents` (`Indents`), native
v3 writes them on the `style` record only when any is set (older records
load with none), the importer reads them, and the engine narrows each
line's column by them: main text (before wrap slots), frame, header,
footer, and cell text, and table measuring. Indents wider than a column
leave 0.10 in (unverified edge).

Styles the importer generates do not take Body Text's indents: Table Text
(in 44 of 49 user style sheets where Body Text is indented, Table Text is
not) and fallbacks for styles neither the document nor its style sheet
defines. With Body Text's indents the ISO corpus, whose styles mostly come
from missing sheets, lost 186 equal line counts.

Measured (`LAYOUT_COMPARISON.md`): equal line counts 95% → 100% on the VM
fixtures (817 of 820), 91% → 97% on the user's corpus, 87% on the ISO
corpus (21,003 → 21,045). Page ends with Ami Pro's spacing: fixtures 27 → 29
of 65, ISO 245 → 247, but the user's corpus 674 → 649 of 1,097: with line
counts now right, our lines' extra height shows (a training manual: 501 of
510 line counts equal, 36 pages against Ami Pro's 34; Book Antiqua lines 3.5%
taller through P052), and spacing modes and break rules are not mapped yet.
(Corrected below, "With para above frame heights": that manual's extra pages
came from stale frame heights, not line height.) Equal
page counts there went from 76% to 77%. Line heights and `[spc]` are next.

Hanging indent (flag 512) and Both sides (16) need nothing more: Both sides
is written into the right indent, and Hanging indent changed nothing on
screen (I3).

## 2026-10-12 — Paragraph and line spacing (Phase 3, second slice)

Ami Pro's Modify Style › Spacing, verified in the VM (S1–S6,
`SAM_FORMAT_NOTES.md`): `[spc]` flags give the line spacing and the Add in
choice; 1 1/2, Double, and Custom lines take the stored height (Ami Pro
computed it for its printer and lays out with it), the extra split above
and below the text; space above and below add between paragraphs; the
space above is left out at the top of a page or column unless Add in is
Always. The model replaces `space_after_points` with `Spacing { line,
above, below, above_at_break, tightness }`; native v3 keeps the space-below
field and adds the rest after the indents; the importer reads `[spc]`
(a style without one has no spacing); the engine applies it in main text,
frame, header, footer, and cell text, and when measuring tables. The space
above is an offset of the line, not a move of the cursor, so a page start
replays it exactly (the incremental property test now runs with spacing).
A line holding a Flow with text frame keeps at least its own height (Ami
Pro's behaviour there is unverified).

Styles a file does not define get 6 pt below, as new documents have: on
the ISO corpus, whose style sheets are missing, that matches Ami Pro's caches
better than Body Text's spacing (247 page ends of 1,306 against 188).
Generated Table Text has no space above or below; a file's own Table Text
keeps its spacing.

Text Tightness is kept but not laid out yet.

Measured: the report's "with Ami Pro's paragraph spacing" variant is gone,
because the import now carries that spacing. As imported: VM fixtures 107 of
108 page counts and 29 of 65 page ends (README31 13 of 13, MERCURY 7 of 7);
the user's corpus 76% of page counts (was 57%) and 59% of page ends (was
46%); the ISO corpus 35 of 160 page counts and 247 page ends (was 29 and
229).

## 2026-10-12 — Break rules (Phase 3, third slice)

Ami Pro's Modify Style › Breaks, verified in the VM (B1–B8): `[brk]` bits
for page and column breaks before and after, breaks within allowed, keep
with previous and with next, and Next style (`SAM_FORMAT_NOTES.md`). The
model keeps the bits as written (`Breaks`), so Next style and the unknown 32
survive for a future writer; native v3 stores them after the spacing.

The engine:

- breaks before a paragraph (none at a column top) and after it (made
  before the next paragraph, and restored by every rollback);
- a paragraph that may not break within and already has lines in a column
  goes to the next column whole, through the same retry that With para
  above frames use (V12); one taller than a column still breaks;
- a paragraph kept with the next one (or the next kept with it) moves to
  the next column with it when the next one's first line lands there,
  taking the whole kept run, once, unless the run began its column;
- a layout never stops early (converges) at a paragraph kept with the one
  before, because that one may still move onto its page. The incremental
  property test found this: four more seeds run with break rules.

Next style is an editing rule (the style Enter gives the next paragraph),
not yet used.

Measured: VM fixtures 75 of 75 page ends (AP31NAV 39 of 39, was 3); ISO
corpus 283 page ends (was 247), 36 page counts; the user's corpus 642 page
ends (was 646) and 93% of page counts within one: some training manuals
ran longer than Ami Pro's (48 pages against 45). That was put down to taller
lines; it was stale frame heights (next entry).

## 2026-10-12 — With para above frame heights

A With para above frame's size is its stored rectangle (fields 3–6), as for
tables and Flow with text frames; field 16 keeps only the width. Evidence:

- in the user's corpus, field 16's width always equals the rectangle's,
  but its height differs in 149 of 309 With para above records, by up to
  thousands of twips (text frames storing 0, 200, or 360 with rectangles
  6,000 to 9,500 tall);
- in the VM, two pictures of a training manual whose field 16 and rectangle
  disagree (4,335 against 3,477, and 4,365 against 6,373) stand at the
  rectangle's height on Ami Pro's page;
- all VM-made fixtures have equal heights, so they are unaffected.

A rectangle made one twip tall (degenerate) keeps field 16's height.

Measured: the user's corpus 662 page ends of 1,097 (was 642) and 300 page
counts (was 299); the two manuals that ran three and two pages long now
match (45 of 45 pages, 40 of 44 page ends; 35 against 34, 27 of 33). VM
fixtures and the ISO corpus are unchanged (75 of 75, 283 of 1,306). One
report loses page ends (42 → 48 pages against 41): its photo pages set line
spacing per paragraph with `<:S+158>`, which is not imported yet, and the
smaller field 16 heights had hidden that. Per-paragraph overrides are next.

Our Bodoni Book lines in those manuals match Ami Pro's line for line, and
their Arial headings are shorter than Ami Pro's, not taller.

## 2026-10-12 — Paragraph overrides (Phase 3)

Ami Pro's Text menu formats one paragraph over its style. VM fixtures O1–O7
and LS1 decoded every Text menu control (`SAM_FORMAT_NOTES.md`, "Character
attributes", "Paragraph overrides", "Font changes"). The first slice maps
Indention and Spacing:

- `Paragraph` gains a `ParagraphFormat` (indents and line spacing, each
  optional). The engine puts it over the style when it shapes the
  paragraph, so every layout path (main text, frames, cells) sees one
  effective style, and the shape cache keys on it.
- A 1 1/2 or Double with height 0 has no stored height; it is 1.5 or 2
  times the line's single height (LS1). That covers the overrides and
  style records that store none. Over a style with a stored height of the
  same kind, the override keeps the stored height.
- The importer reads the marks anywhere in the paragraph, the later one
  winning (O6), and keeps `<:S->` as a legacy mark.
- Native v3 writes `pformat` records (one property each) between a
  paragraph and its runs, so alignment can follow without a format change.
- Splitting a paragraph gives both parts its format, as it gives them its
  style. Ami Pro's own behaviour on Enter is not checked yet.

Measured: the user's corpus 686 page ends of 1,097 (was 662); one report
went from 48 to 42 pages against Ami Pro's 41 (30 of 40 page ends). The ISO
corpus 287 page ends (was 283) and 38 page counts (was 36); its line count
share fell from 87% to 85% because about 800 paragraphs with Custom spacing
can now be compared, and 156 more agree. One file of the user's corpus
gained a page: its narrowed paragraph also sets 11 pt with `<:f>`, which is
not imported yet.

## 2026-10-12 — Paragraph alignment (Phase 3)

Styles take their alignment from `[algn]` value 1 (1 left, 2 right, 4
centre, 8 justified; I1–I3) and paragraphs from Text › Alignment's `<+@>`,
`<+A>`, `<+B>`, `<+C>` (O2), applied through `ParagraphFormat` like the
other overrides. With two marks in one paragraph the larger code applies:
it fits every pair tried in the VM (O6, O7), but it is inferred, and the
corpora have no such paragraph.

The engine moves right and centred lines by the free width (or half of
it), and gives a justified line's free width to the spaces between its
first and last words, each space's box covering its share. A paragraph's
last line and a line ended by a hard return stay left (O2 shows the first;
the second is assumed, to be checked in the decompile). Flow with text
frames take their x from the placed line. Lines are flagged `spaced` so
that the page surface draws their words one by one; its line cache now
also compares word positions, as it reused stale positions when a line
moved without its text changing.

Alignment does not move page breaks: the comparison is unchanged (user's
corpus 686 page ends, ISO 287, VM fixtures 75 of 75). Page-surface typing
on a justified 45-page manual shows no regression against `82d1834`.


## 2026-10-12 — Layout rules from the decompile

The decompile audit (`SAM_FORMAT_NOTES.md`, "The reader in AMIPRO.EXE" and
"Layout rules from the decompile") replaced inferences with Ami Pro's own
rules, and the importer and engine now follow them:

- **Alignment marks** OR together (left 1, right 2, centre 3, justify 4;
  anything else is the style's). This corrects "the larger code wins",
  which O6 and O7 could not tell apart. A style with several alignment bits
  is right, then centre, then justified. Both sides makes the right indent
  All.
- **`[spc]`**: only "When not at break" (32) keeps the space above off a
  column top, so a style with neither Add in bit adds it as Always does
  (`Spacing::above_at_break` is renamed `add_at_break`, since one bit
  governs both spaces). The spacing bits are tested 1, 2, 4 in turn.
- **1 1/2 and Double** are h + ⌊h/2⌋ and 2h from each line's single height.
  The height a style stores for them is the Spacing dialog's cache: kept in
  the model for a future writer, not laid out. The baseline moves by half
  the difference, rounded down, and never above the line's top.
- **Space below** with Always belongs to the paragraph's last line, which
  must fit with it. This is kept although it costs 7 page ends on the
  user's corpus (686 → 679): the loss comes from our lines being taller
  than the printer's in those documents (`LAYOUT_COMPARISON.md`, "Rules
  from the decompile"), and fitting by a compensating error would hide it.
  Frame interiors use it too (VM fixtures FS4, FS5; added 2026-10-12,
  Phase 3.2d; no change in the comparison's measures).
- **Column breaks** (`<:c>`) end the column after their paragraph, wherever
  they stand in it.
- **A break after the last paragraph** still starts a page or column; Ami
  Pro's page table lists the empty page (B2, B4, V22). The VM fixtures'
  page counts are now 121 of 121.

## 2026-10-12 — Line heights follow the printer (user decision: option 1)

Ami Pro lays lines out with the printer's font metrics, and documents
carry the printer they were last laid out for (`[prn]`). The user's corpus
shows one consistent single height per printer, face, and size, differing
between printers by up to 15 twips at 12 pt (`LAYOUT_COMPARISON.md`). Our
shaper uses one set of metrics for every document, which is now the main
cause of page-end differences.

Options:

1. **Calibrate from the document** (recommended): when a document is
   imported, read the single line heights Ami Pro cached in its paragraphs
   (`<:#h,w>`) per face, size, and weight, keep them with the document as
   its printer's metrics, and use them before the font's own; faces and
   sizes the document never cached fall back to today's metrics. This
   reverses the rule that the importer ignores the caches
   (`SAM_PAGE_FRAME_SPEC.md` §8), but only for metrics, not positions. It
   reproduces each document as its author saw it, and editing keeps using
   the same heights.
2. **Model printers**: derive heights from a printer resolution and Windows'
   rounding. The corpus values do not follow plain rounding at 300 or 600
   dpi, so this would be a table per printer, never complete.
3. **Leave as is**: one metric for all documents; page ends stay about 62%
   on the user's corpus.

The choice changes how imported documents paginate, so it waited for the
user. **Decided (user, 2026-10-12): option 1, calibrate from the
document's caches.** Implementation: below, "Calibrated line heights".

## 2026-10-12 — Calibrated line heights

Option 1 above, as implemented (`cc0b7fb`, `b18349d`):

- **Model:** `LineHeights`, a table from a font (face compared without
  letter case, size in twentieths of a point, bold, italic) to its single
  line height in twips, kept with the document. Native v3 stores it as
  `lineheight` records after the styles. Changing it is a structure change
  for relayout, and the shape cache forgets its entries when it changes.
- **Import:** a single-spaced paragraph in its style's font alone (no
  runs, inline objects, or legacy marks) whose cached height is one line
  (0.85 to 1.6 times the font size) votes for its font's height; each font
  takes its most frequent height, the smaller on a tie. Taller caches hold
  more lines and do not vote. The caches themselves are still dropped.
- **Layout:** a font with a height gets it for every glyph it sets, split
  between ascent and descent as the font splits its own; other fonts keep
  their metrics. Runs in a calibrated font use it too.
- **Not covered:** fonts that never appear alone in a one-line,
  single-spaced paragraph (bold runs inside body text, `<:f…>` faces once
  imported), and new documents, which have no table. A later printer
  setting could fill it.

Measured (release comparison report): user's corpus page ends 679 → 689
of 1,097 (686 before the space-below rule), page counts 299 → 298 of 394;
ISO corpus page ends 287 → 371 of 1,306 and page counts 38 → 45 of 160;
VM fixtures unchanged at 121 of 121 and 75 of 75. The single-spaced height
difference (3.1% → 0.7% on the user's corpus) is partly circular, since
the heights come from the same caches; page ends are the independent
check. Page-surface typing on README31 shows no regression in an
interleaved A/B against `071a8f7`.


## 2026-10-12 — UI pass before the rest of Phase 3 (user decision)

The user asked to skip the UI-2 physical checklist, to do the UI pass around
the page surface next together with the screenshot-against-mockup gap list,
and to move the UI ahead so that later work is not revisited. So:

- The UI-2 checklist (`UI_ACCEPTANCE.md` §5) is superseded; its remaining
  items move into one combined physical check at the end of the UI pass.
- The UI pass targets the `page-surface` build: it closes the gaps in
  `docs/Design/UI_GAP_LIST.md`, and includes UI-3 (the Styles workflow),
  because the context rail is rebuilt in it. Commands the approved action
  bar shows (Underline, paragraph alignment) are implemented for real, not
  drawn as placeholders.
- The rest of Phase 3 (character overrides, Text Tightness, Next style,
  style tabs) follows the UI pass, so that its drawing work lands on the
  final page surface.
- Whether the page surface becomes the default build is decided by the user
  at the combined check; its other gates (accessibility nodes, visual IME
  preedit, picture decoding) stay in `PROJECT_PLAN.md`.

## 2026-10-12 — The toolbar is editable (user requirement)

During the UI pass the user said the toolbar should ultimately be editable.
Ami Pro 3.1's SmartIcons (user's guide chapter 2) are: named sets,
add/move/remove/group icons, position and size, show/hide, custom macro
icons. The proposed design is `UI_SPEC.md` §12: a data-driven action bar
whose built-in Standard set is the approved Revision 2 order, a Tools ›
Customize Toolbar dialog, sets stored in the user's COSMIC configuration,
and View › Show Toolbar; top position only at first. It is built in the UI
pass, before Underline and the alignment group are added, so they land in
the data-driven bar. Two points are left for the user at the combined check:
sets at once or one editable set first, and whether left/right positions
matter.

**Answered (user, 2026-10-12):** one editable toolbar for now, top only. The
user also found More on a mid-size window with room to spare: the bar moved
groups at fixed window widths from the spec (1,040 and 860 px) although the
Standard bar needs 608 px. The bar is now data-driven (`chrome.rs`,
`ToolbarItem`, `STANDARD_TOOLBAR`) and moves groups to More only when the
measured widths do not fit (`the_action_bar_widths_match_the_real_widgets`
checks the measurement against the real widgets to 2 px).

## 2026-10-12 — The page surface is the normal build (user decision)

At the combined physical check the user found the editable toolbar,
Underline, the four alignments, and the toggles' selected states working
(“This all appears to work great”) and decided: “This should become the
normal build.” `default = ["page-surface"]` in `app/cosmic/Cargo.toml`.

- The text editor it replaces is kept, buildable and tested with
  `--no-default-features` (`tools/verify.sh` steps `app-clippy-text` and
  `app-test-text`), as AGENTS.md asks for the accepted editor; the
  rich-editor prototype alone is
  `--no-default-features --features rich-editor-prototype`.
- The page surface's remaining acceptance gates stay open and are now gaps
  of the normal build: accessibility nodes for the page text (screen
  readers do not see it), a physical check of visible IME preedit, and
  picture decoding (pictures draw as placeholders). They stay in
  `PROJECT_PLAN.md`.

## 2026-10-12 — Check marks for every on/off command (user decision)

Asked whether the alignment items should carry check marks as in Ami Pro's
Text menu, the user answered: “if we are using check marks on style sheet
to show it, we should use it on the other items.” View › Toolbar and View ›
Styles Panel already had them; Format › Bold, Italic, Underline, and the
four alignments now show theirs (`menu_check` in `main.rs`): Bold, Italic,
and Underline when the whole selection has them, an alignment when every
selected paragraph has it. The states follow the page selection in its own
flow (main, frame, header, footer, or cell text); before, the toolbar's
Bold and Italic states read only the main text.

## 2026-10-12 — Character attributes in the model (Phase 3, character overrides)

Ami Pro keeps a character's attributes as a bit word that the body's
`<+X>` and `<-X>` marks change (decompile: the attribute routine in the
layout segment, `SAM_FORMAT_NOTES.md`, "Character attributes"). The model
follows it:

- `CharacterFormat` gains word underline, double underline, strikethrough,
  superscript, subscript, caps, and colour, each optional like bold:
  `None` is the style's. The underlines are separate attributes, as in Ami
  Pro, where they are separate bits. The caps are one value
  (`Caps::Upper`, `Lower`, `Initial`, `Small`) because each excludes the
  others; switching any of them off returns to the style's caps.
- `Some(false)` is kept: Ami Pro's strikethrough, superscript, and
  subscript marks end the attribute instead of returning to the style, so
  `<-%>` in a strikethrough style is `strikethrough: Some(false)`.
- `ParagraphStyle` gains `CharacterAttributes`: the same attributes as
  plain values plus the style's colour (`[fnt]` flags and value 3, which
  the importer did not read). 150 style records in the user's corpus are
  upper case, 22 initial caps, 9 underlined, and 124 coloured.
- `CharacterFormat::attributes(style)` resolves a run over its style for
  layout and drawing.
- Native v3 stores them in `sformat` and `rformat` records after their
  `style` or `run`, one attribute each, as `pformat` does for paragraphs,
  so the `style` and `run` records keep their field counts and older files
  load unchanged (`NATIVE_FORMAT_V3.md`, "Character attributes").
- Not kept: the pitch-and-family byte of `<:f…>` (and of a style's
  `[fnt]`), which Windows uses only to substitute a face that is not
  installed; overstrike (`<:O…>`), protected text, and no hyphenation stay
  legacy marks. These belong to the preservation design a SAM writer
  needs.

## 2026-10-12 — Ami Pro's Text menu items, and states at the caret (user decisions)

Having no documents with the new attributes, the user asked for menu
commands to apply them, and noted that Ami Pro shows a word's attributes
when the caret is on it, not only over a selection.

- **Menu layout (user's choice of three):** Ami Pro's items in Format: Word
  Underline, Caps ▸, Special Effects…, Font…, Normal (`UI_SPEC.md` §13). The
  reserved "Character…" entry is gone; Clear Direct Formatting is now
  labelled Normal and implemented.
- **Ami Pro's behaviour, checked in the VM (fixture C1, a synthetic SAM
  with a bold word and a Title paragraph):** the Text menu checks Bold with
  the caret just before or after a bold word and not before the space ahead
  of it; Ctrl+B with nothing selected makes the next typing bold and a
  second Ctrl+B ends it; text typed at either end of a bold word is bold;
  Normal removed a word's bold and its paragraph's centring; and a bold,
  centred Title style showed neither Bold nor Center checked (Ami Pro
  checks only formatting applied with the Text menu).
- **Check semantics (user's choice of two):** keep "what you see": states
  include the style's formatting, and switching off over a style that has
  the attribute sets it off on the text. `Caps::Normal` lets a run switch a
  style's caps off.
- Implemented with `SetCharacterAttribute`, `SetFont`, and
  `ClearDirectFormatting` (`amipro-editing`), a typing format in the
  session applied to the next insertion in the same undo step, and runs
  that take text typed at either end.

## 2026-10-12 — The paragraph's end format (Phase 3.1)

The decompiled layout (`10d0:0592`) sets a paragraph without text in the
font in force after its format changes: the line is that font's height plus
external leading, with its descent. A line with text is measured by its
characters alone (`10d0:359b`, called only for characters taken onto the
line), so a format change after a paragraph's last character changes no
line. The importer kept such changes as legacy marks, and an empty
`<:f200,,>` paragraph was laid out in its style's 12 pt (276 twips where
Ami Pro's cache says 240; 75 such paragraphs in the user's corpus).

- **Model:** `Paragraph::end_format`, a `CharacterFormat` like a run's: the
  format in force at the paragraph's end, after its last character (the
  format of its paragraph mark). Empty is the style's.
- **Import:** every paragraph's end format is the format in force after
  its last control; the format changes of an empty paragraph are no longer
  legacy marks.
- **Layout:** a line with nothing measured (an empty paragraph) takes the
  end format's font over the style; lines with text are unchanged.
- **Native v3:** an `endformat` record after the runs, with the run's format
  fields and `rformat` records of its own (`NATIVE_FORMAT_V3.md`).
- **Editing** (VM fixture EN1, typed in Ami Pro and saved):
  - text typed into an empty paragraph takes its end format, as it lands
    after the paragraph's format changes; elsewhere the C1 rule holds
    (typing at either end of a run joins it);
  - a paragraph end that Enter inserts takes the format typing at the
    caret would take: after a bold word the new paragraph's typing is bold
    (Ami Pro wrote `<+!>x`), after plain text plain, and Enter before a
    bold word leaves a bold empty paragraph above it (`<+!>` alone); the
    text after the caret keeps the old paragraph end, and joining keeps the
    later paragraph's;
  - a character command over a range that covers a paragraph whole (an
    empty one included) changes its end format too, and Normal clears it;
  - the caret states and the Font dialog read an empty paragraph's end
    format.

Calibration (`DECISIONS.md`, "Calibrated line heights") was widened at the
same time, after measuring which paragraphs it missed: of the 1,196
single-spaced paragraphs of the user's corpus set in a font without a
height, 869 held a legacy mark that changes no font (839 `<:s>`, a
paragraph flag in the decompiled control parser `11b8:030a`) and about 170
a run that changes no font. So a sample is now any single-spaced paragraph
without objects or hard returns, measured by `Paragraph::fonts`: one font
votes as before, and a one-line paragraph that mixes known fonts with one
other, taller than the known ones, gives the other its height. A style's
paragraph rules (`[line]`) add their space to the first and last lines
(decompiled line builder `10d0:0d82`), so the importer subtracts it from
the sample; rules themselves are not laid out yet.

## 2026-10-12 — Paragraph rules (Phase 3.2a)

Found while calibrating line heights (3.1): a style's `[line]` record is
Ami Pro's Modify Style › Lines, rules above and below its paragraphs, and
Ami Pro adds their space to the first and last lines (decompile
`10d0:0d82`). 385 style records in the user's corpus have one. VM fixtures
NX1 and NX2 mapped the record to the dialog and showed the geometry
(`SAM_FORMAT_NOTES.md`, "Style records").

- **Model:** `ParagraphStyle::rules`, a `ParagraphRules`: each side on or
  off with its line style (1–12) and the space it takes, the length (Text,
  Margins, Other with its twips), the colour, and the three values whose
  meaning is not known, kept for a writer. `rule_strokes` is Ami Pro's
  table of strokes and gaps per line style.
- **Native v3:** an `sline` record after the style's `sformat` records,
  holding the ten `[line]` values as Ami Pro writes them, only when they
  differ from the default (`NATIVE_FORMAT_V3.md`).
- **Layout:** the space goes on the first line's ascent and the last line's
  descent, after the line spacing; each line carries the `RuleBox`es it
  draws, relative to the line, so they move with it.
- **Drawing:** the page surface draws each stroke on whole pixel rows, at
  least one pixel thick, in the rule's colour. Our rendering of NX1 matches
  Ami Pro's.
- Fallback styles for names no style sheet defines have no rules, like their
  indents and spacing.

Measured: VM fixtures 126 of 126 page counts and 76 of 76 page ends, with
NX1OUT's line counts 4 → 7 of 7 and every NX2OUT height equal to Ami Pro's
cache; user's corpus page ends 742 of 1,108 and page counts 297
(unchanged), single-spaced height difference 0.4% → 0.3%; ISO corpus 409 →
408 page ends.

## 2026-10-12 — Next style and Text Tightness (Phase 3.2)

- **Next style** (VM fixture NX1): `ParagraphStyle::next`, the style named
  after `[nfmt]` when it is another defined style (native `snext`); the
  `[brk]` 64 switch stays in `Breaks`. Enter at a paragraph's end gives the
  new, empty paragraph the next style, without the old paragraph's own
  formatting; in the middle or at the start both parts keep the style, as
  Ami Pro does.
- **Text Tightness** (VM fixtures TT1, TT2): every character, spaces
  included, gains (percent − 100)% of its font's average character width.
  The average is Windows' weighted lower-case average, computed from the
  installed face's advances (for Times New Roman it is exactly Windows'
  821/2,048, and metric-compatible substitutes share it). The shaper sets it
  as COSMIC Text letter spacing; the page surface, whose Iced spans have no
  letter spacing, draws a tightened line character by character at the
  measured places. TT2's five paragraphs break where Ami Pro breaks them.

## 2026-10-12 — Tab stops (Phase 3.2c)

The engine had no tab stops: a tab was as wide as the shaper's tab glyph.
The style's own tabs and Hanging indent's stop (runbook 3.2) need them, so
tab layout came first (`SAM_FORMAT_NOTES.md`, "Tabs"; VM fixture TB2):

- **Model:** `TabStop::leader` (hyphens, dots, a line); a style's own tab
  list and Hanging indent flag (`ParagraphStyle::tabs`, `hanging`); a
  paragraph's ruler as `ParagraphFormat::tabs`, which `apply` lays over the
  style as its own tabs. `ParagraphFormat` is no longer `Copy`. The default
  Letter layout has Ami Pro's default stops, every half inch to six inches,
  so new documents' tabs work.
- **Native v3:** stops write `K:POSITION` with `:h`, `:d`, or `:l` for a
  leader; `stabs` and `shanging` after a style; `pformat tabs` for a ruler.
- **Import:** leader bits no longer drop a stop into legacy data; style tab
  lists and the hanging flag are read; `<:R…>` rulers replace their legacy
  marks; a document without `[lay]` takes its style sheet's layouts.
- **Layout:** shapers make each tab a word of its own with no width;
  `tabs::size_tabs` sizes a line's tabs before it is broken, in the main
  text, frames, headers, and cells.
- **Page surface:** text runs end at tabs; a tab draws its leader across its
  gap and gives the caret a place at its end.

## 2026-10-12 — Bullets and numbers (Phase 3.2e)

Modify Style › Bullets & numbers (`[spec]`) was read into nothing, so no
bullet or number was drawn (`SAM_FORMAT_NOTES.md`, "Bullets and numbers";
decompile; VM fixtures BN1–BN3):

- **Model:** `ParagraphStyle::bullets` (`Bullets`): the text with Ami Pro's
  codes as characters U+0007–U+001F, the outline level, the attributes, the
  flags, and the six values not understood, kept as written. Numbers are
  not stored: layout counts them (`amipro_layout::paragraph_numbers`) as
  Ami Pro does, so an edit renumbers what follows; incremental layout
  reuses a tail of pages only when its numbers are unchanged.
- **Native v3:** `sbullets`, the last of a style's records.
- **Import:** the ten values; `<*>>` decoded; a line of only tabs and
  spaces is an empty value, so later values keep their places.
- **Layout:** the first line's `LineBox::bullet` holds the bullet's pieces
  (text, or a drawn shape) at the left indent; the text moves right only by
  what the first-line indent does not leave for it, in the main text,
  frames, headers, and cells.
- **Page surface:** number and bullet text in the paragraph's font with the
  bullet's bold, italic, and underline; dots, squares, rings, diamonds, and
  the check mark drawn; Ami Pro's bitmaps as an arrowhead glyph.
- **Not yet:** Right align, superscript bullet text, the bitmaps' pictures,
  and a command to choose bullets (style editing, after decision D1).

## 2026-10-12 — Overstrike, protected text, no hyphenation, face kinds (Phase 3.3)

The last character attributes the importer kept as legacy marks, and the
face byte it dropped (`SAM_FORMAT_NOTES.md`, "Character attributes";
decompile; VM fixtures O1, PT1):

- **Model:** `CharacterFormat` and `CharacterAttributes` gain `protected`,
  `no_hyphenation`, and `overstrike` (a character; no style has one). A
  style's `[fnt]` 4096 is protected text. `Document::face_kinds` keeps each
  face's Windows pitch and family byte, by name.
- **Native v3:** `protected`, `nohyphenation`, and `overstrike` attribute
  records; `facekind` document records after `lineheight`.
- **Import:** `<+*>`, `<+.>`, and `<:O±C>` become runs; face kinds come from
  `[fnt]`'s kind bits and each `<:f…>`'s K, the commonest byte that names
  a family.
- **Editing:** protected text as PT1 showed: the arrows step over it, Shift
  selects it whole, a click in it lands after it, Delete and Backspace
  remove it a character at a time, and text typed at its edges stays
  outside it. One difference: Ami Pro puts text typed right after deleting
  into protected text inside it; ours leaves it outside.
- **Layout:** text marked No hyphenation does not break at its
  discretionary hyphens.
- **Page surface and Special Effects:** overstrike draws its character over
  each character; the dialog has Ami Pro's Overstrike character box.
- **Substitutes:** a face that is not installed and whose name gives no
  kind takes its stored kind's substitute (swiss, fixed). Script faces keep
  the serif whatever their byte: Z003 for script made Monotype Corsiva's
  lines 32% taller than Ami Pro's caches (11% with the serif), and
  Shelley Allegro Script's byte is swiss, whose substitute cost a line.
  Measures unchanged.
- **Not yet:** Edit › Mark Text commands (Protected Text, No Hyphenation)
  are not in the menus; they can follow the style-editing decision.

## 2026-10-12 — Styles in the model: origins, commands, style sheets (Phase 3.4)

Ami Pro keeps two kinds of style (user's guide, chapter 9; VM fixture ST1):
those stored in the document (created or changed there; a SAM's own
`[tag]`s) and those of the attached style sheet. Removing a style and using
another style sheet both depend on the difference, so the model records it:

- **Model:** `ParagraphStyle::origin` (`StyleOrigin`: document, sheet, or
  missing) and `Document::style_sheet` (`StyleSheetLink`: the `[sty]` name
  and whether the page layout is the sheet's). A missing style keeps its
  name and ID, so its paragraphs keep their tag, and formats as Body Text,
  following Body Text when it changes. New documents' styles are stored in
  the document (Ami Pro's "None" sheet). `StyleSheet` carries a sheet's
  styles and layout for Use Another Style Sheet.
- **Native v3:** `stylesheet` after `ids`; `sorigin` last of a style's
  records, absent for document styles.
- **Import:** origins from where each style came from; generated Table Text
  is the document's; `read_style_sheet` reads an `.STY` alone (one without a
  body section too) and refuses a file without paragraph styles.
- **Commands:** `CreateStyle` (based on a style, stored in the document),
  `ModifyStyle`, `RenameStyle` (Body Text keeps its name; names stay unique
  without regard to case), `DeleteStyle` (Remove: the style becomes missing,
  or takes the sheet's version given one; Body Text only reverts), and
  `UseStyleSheet` (document styles stay; sheet and missing styles take the
  new sheet's of the same name or become missing; the sheet's others join;
  page settings follow the sheet only when the layout was the sheet's).
  All undo through the history like every command.
- **Not yet:** the menus and dialogs (decision D1), Style Management's Move
  and function keys, the sheet's header and footer frames on Use Another
  Style Sheet (only page size, margins, columns, and borders follow), and
  Save as a Style Sheet (decision D3).

## 2026-10-13 — Phase 3 decision stop: D1, D2, D3 (user decisions)

Asked together at slice 3.5 (`docs/SESSION_HANDOFF.md`); the user's answers:

- **D1, style editing design: approved** as `docs/Design/UI_SPEC.md` §14
  proposes: Ami Pro's Create Style, Modify Style (one dialog with its
  panels), Style Management, and Use Another Style Sheet dialogs, under
  Format › Styles ▸ (the section's recommendation; the eight menu roots
  stay), with the Styles panel as the management owner.
- **D2, hyphenation: yes**, as `docs/research/HYPHENATION_OPTIONS.md`
  recommends: the `hypher` crate (MIT OR Apache-2.0, US English patterns) in
  the app crate behind a hyphenator trait in `amipro-layout`, with Ami
  Pro's rules from the decompile, measured on the hyphenated styles.
- **D3, writing `.STY` files: yes**: Save as a Style Sheet (and Style
  Management's Move) write new files only and never overwrite an existing
  one; the written sheet is checked by opening it in Ami Pro in the VM.

## 2026-10-13 — Hyphenation (Phase 3.6, decision D2)

As decided (`HYPHENATION_OPTIONS.md`, option A):

- **Model and format:** `ParagraphStyle::hyphenation` (`[algn]` 256) and
  `Document::hyphenation_zone` (`[docopts]` value 1, default 5); native v3
  `shyphenation` and `hyphenzone`.
- **Rules** (`amipro_layout::hyphenation`, from the decompile): which words
  may be split and where (letters only, five or more, the first 32
  characters, no digits, no-break spaces, or discretionary hyphens, never
  next to No hyphenation text).
- **Points:** `Shaper::hyphenate`, by default none; the app's shaper uses
  `hypher` (MIT OR Apache-2.0) with TeX's US English patterns, `FixedShaper`
  a fixed rule for tests. The core crates stay dependency-free.
- **Breaking:** shapers split a word at its points into pieces that the
  breaker treats like discretionary hyphens, but takes only where the line
  had the zone left before the word (`next_line`'s `zone`), unless the word
  starts the line. The page surface shows the hyphen as for a soft hyphen.
- **Measured:** user's corpus line counts 14,389 → 14,400, page counts 298
  → 299, page ends 742 → 744 of 1,108; VM fixtures unchanged; ISO corpus
  one line fewer (21,315). HY1's eight paragraphs have Ami Pro's line
  counts. Ami Pro's own dictionary splits some words elsewhere ("In-
  ternational"); ours follows TeX's patterns.

## 2026-10-13 — Writing Ami Pro style records (Phase 3.6, decision D3, part 1)

Save as a Style Sheet writes Ami Pro files (D3: new files only, never
overwriting). Part 1 is the style record:

- **Writer:** `amipro_formats::style_record`, after the decompiled writer
  (`11c8:26b7`) and its text escaping (`11c8:0bf5`).
- **Kept values:** the model keeps what a record holds beyond its fields
  (`ParagraphStyle::kept`, `StyleKept`, native `skept`), and Both sides as a
  style field, so that a written record says what the original said.
- **Evidence:** the corpus audit writes every style record back. ISO corpus
  542 of 542 byte for byte; user's corpus 1,396 of 1,912, the rest older
  records that Ami Pro 3.1's writer would also extend with the next style,
  tabs, and right indent.
- **Found on the way:** bullet texts lost their trailing spaces at import,
  a bullet text starting with `[` was read as a section, and next-style
  names the sheet lacks were dropped. Fixed; layout measures unchanged.
- **Part 2, next:** the whole sheet: its header sections, the page layout
  (`[lay]` with header and footer frames and their text, the body writer a
  SAM writer needs too), `[edoc]`, the Save as a Style Sheet dialog, and the
  VM check that Ami Pro opens the written sheet.


## 2026-10-13 — Writing whole style sheets (Phase 3.6, decision D3, part 2)

Save as a Style Sheet writes a whole `.STY` as Ami Pro 3.1 does without
contents (`SAM_FORMAT_NOTES.md`, "Writing style sheets"):

- **Writer:** `amipro_formats::style_sheet` (header sections, styles,
  the `Standard` page layout with its header, footer, and repeating
  frames, `[elay]`, `[edoc]`), after the decompiled writer (`11c8:14e0`,
  `11c8:236e`, `11c8:24d6`, `11c8:2a0f`) and VM fixtures SS1N, SS2N, SS2W;
  `layout_record` writes one page layout; `body.rs` writes body text as
  `11b0:1405` does (frame text now; the main text when contents come).
- **Kept values:** the importer keeps what LAYOUT blocks and frame records
  hold beyond the model (`rght.3` … `rght.14`, `frmlay.9` … `frmlay.14`,
  column-edge offsets `*.edges`, a header or footer rectangle's offset from
  its margin band `frame.rect`, field 1, the sequence number
  `frame.field13`, a `[txt]` without paragraphs `txt`, a layout's frame
  order `lay.order`) as legacy data, which native v3 already stores. Bit
  0x8 of layout frames and 0x80 of the layout number are kept as read, not
  derived. Header sections come from the document's kept sections, else
  Ami Pro's values for a new document (the printer `_DEFAULT.STY` names).
- **Without contents** (Ami Pro's With contents off): the text of layout
  frames with 0x8 is left out, as Ami Pro does; pictures are left out of
  every layout frame (Ami Pro keeps those of frames without 0x8: 17 frames
  in the user's corpus, 4 in the ISO corpus). With contents (main text,
  page frames, tables, pictures, the `[Embedded]` directory) is not
  written yet.
- **Style names** are decoded at import (`Caf<\i>` is `Café`), as Ami Pro
  does (AP4), so the writers can escape them.
- **Evidence:** a sheet written from `_DEFAULT.STY` equals Ami Pro's SS1N
  byte for byte apart from the printer and `[desc]`'s bookkeeping; every
  corpus sheet written as a sheet reads back with the same styles and
  layout (ISO 63/63, user's 192/192); page layouts written back byte for
  byte: ISO 178 of 184, user's 442 of 588 (the rest: 89 older one-value
  line fields that 3.1 writes in full, 21 pictures, one layout with
  duplicate left headers, and frame text in which Ami Pro repeated or
  reordered controls); every layout frame's text reads back as the model
  had it (ISO 384/384, user's 786/786). Ami Pro opens the written sheets
  with File › New and Use Another Style Sheet (AP2–AP4). Layout measures
  unchanged.
- **Next:** the Save as a Style Sheet dialog (`UI_SPEC.md` §14) with the
  document's link afterwards as Ami Pro sets it (SD1, SD2, SS2).

## 2026-10-13 — The style editing interface (Phase 3.6, decision D1)

Built as `docs/Design/UI_SPEC.md` §14 approved it; the choices made on the
way (recorded there under "Implementation notes"):

- **Save as a Style Sheet** follows Ami Pro: a description, then the file
  (the portal; an 8.3 name suggested; `.STY` added; another extension or
  an existing file refused, D3), written without contents. Afterwards a
  saved document uses the sheet it wrote and every style becomes the
  sheet's; an untitled one has no sheet and every style its own (VM
  fixtures SD1, SD2, SS2): `Command::SavedAsStyleSheet`, one undo step.
  With contents and Run macro are not offered yet.
- **Style Management** offers << Move <<, Revert, and Remove. >> Move >>
  would rewrite the sheet file, which D3 rules out (new files only), so it
  is not offered. Function keys are left for later.
- **Use Another Style Sheet** lists the style-sheet folders: a preference
  (Ami Pro's style-sheet path) that Browse… extends and SAM import now
  searches; `AMIPRO_STYLES` adds folders.
- **Modify Style** is in the normal build only: its sample is a small
  document in the edited styles, laid out by the page engine and drawn by
  the page surface, and it reuses the Font dialog's palette. Edits stay in
  per-style drafts until OK, which applies them as one undo step. The
  Bullets & numbers panel writes Ami Pro's order (number, Text, bullet;
  VM fixture BN4). Ami Pro reads back every value it edits (AP5).
- **Styles panel**: + for Create Style, Ami Pro's bullet before document
  styles, names defined nowhere in the error colour with a tooltip, and a
  context menu per row.
- **Found:** style commands reported no change, so `History::execute`
  neither applied nor recorded them; `ChangeReport::styles` fixes it.
- **Observation for the user:** new documents' default styles used Noto
  Serif, which the XP guest lacks; Ami Pro substitutes it in a sheet
  written from a new document (it shows the face in red). Decided the same
  day (below).

## 2026-10-13 — New documents in Ami Pro's faces (user decision)

The user chose Ami Pro's faces for new documents: Body Text is Times New
Roman 12 and Heading 1 and 2 are Arial (bold, 20 and 16 pt), as
`_DEFAULT.STY` pairs them, with Ami Pro's face-kind bits (roman 0xC000,
swiss 0x4000) so that a style sheet written from a new document carries
the `[fnt]` values Ami Pro writes. The app shows them in Liberation Serif
and Liberation Sans. A style record without `[fnt]` (none in 6,500 corpus
records) now falls back to Times New Roman too.

Measured (`LAYOUT_COMPARISON.md`, "New documents in Ami Pro's faces"): the
ISO corpus, whose style sheets are mostly missing, gains 65 of 160 page
counts (was 49), 22,779 of 24,971 line counts (was 21,315), and 501 of
1,307 page ends (was 406); the user's corpus and the VM fixtures are
unchanged.

## 2026-10-13 — Style Properties in the rail and right-click menus (user)

The user liked Frame Properties because the document stays in view, asked
for style editing to work the same way, and answered `UI_SPEC.md` §15's
four questions "yes to all four": the rail panel replaces the Modify
Style dialog, follows the caret's style, widens to about 300 px, and
applies every change at once as one undo step. They also asked for
right-click menus and approved their contents (§16).

- **Built:** `style_properties.rs` replaced `modify_style.rs` (`30df4e7`);
  the right-click menus on the page (`98a956f`). Details and the choices
  made are in §15's implementation notes and §16.
- **Ami Pro's right button** opens a dialog, not a menu: Modify Style in
  the main text, Modify Page Layout in a margin, Modify Frame Layout on a
  frame (user's guide chapters 7, 8, 15). The menus keep that target
  first, so the Ami Pro habit still lands on the right thing.
- **Found:** Use Another Style Sheet's dialog panicked when a sheet index
  was out of range (an empty list); the selection is now checked.
- **Cost:** building the menu with each view adds about 0.05 ms per
  keystroke on README31 (median, three interleaved release rounds).

## 2026-10-13 — Phase 4 as a straight run; saving SAM files (user decisions)

After Phase 3's physical check passed, the user asked for Phase 4 (SAM
compatibility, starting with the SAM writer) to be set up as Phase 3 was,
but **as a straight run**: no stop for decisions in the middle. The
decisions it needs were asked first, ranked, with a recommendation each;
the user took every recommendation:

- **A, where SAM files are written: new files only.** File › Save As
  offers Ami Pro (`.SAM`) beside `.amipro` and never overwrites an existing
  file, the SAM the document came from included, as Save as a Style Sheet
  (Phase 3, D3). Save on a document whose location is a SAM keeps going to
  Save As. Writing over the source with a backup, as Ami Pro's own Save
  does, can be revisited once the writer has been used.
- **B, what cannot be written exactly: warn, then let the user choose.**
  Save As SAM lists what would be lost and offers Cancel or Save Anyway.
  It should be rare: the importer keeps what it does not map as legacy
  data with its object.
- **C, headers and footers setup: its own section right after Phase 4.**
  The user noticed the plan had no item for setting up headers and
  footers (Ami Pro's Page › Modify Page Layout and its floating headers
  and footers). Phase 2 imports, lays out, and edits their text only.
  Page layout setup becomes a plan section after Phase 4: a Page Layout
  Properties inspector in the context rail (the user's preference for
  rail inspectors), designed first in `UI_SPEC.md` for approval; the Phase
  4 run ends by writing that proposal.

The runbook is `docs/PHASE4_RUNBOOK.md`, its live state
`docs/PHASE4_PROGRESS.md`. Every readable SAM in both corpora is format
version 4 (417 in the user's corpus, 160 on the ISO; the others are Lotus
Word Pro files or encrypted, both refused with a message), so the writer
writes version 4 only.

## 2026-10-13 — The SAM writer's caches and Ami Pro's byte 0xAD (Phase 4.1–4.3)

- **Caches (4.1).** Ami Pro trusts the paragraph caches (`<:#H,W>`) it is
  given when their width matches the column, and rebuilds `[pg]` from them
  (VM fixtures K1–K6, `SAM_FORMAT_NOTES.md`, "Writing documents"). The
  writer therefore never writes `[pg]`, writes a paragraph's cache only
  while the paragraph, its style, and its column are as read (slice 4.9),
  and never computes one: a wrong cache would paginate wrongly in Ami Pro
  (K1W: 24 pages for 14).
- **Byte 0xAD is a visible hyphen (4.3).** Ami Pro shows the character it
  writes as `</m>` (byte 0xAD, Alt+0173) as a hyphen and allows a break
  after it as after `-` (line breaker `10d0:1f9e`); the discretionary
  hyphen is a different control, `<*7>` (V19). The importer read both as
  U+00AD, so the 2,980 `</m>` of 35 corpus files ("1-2-3") were invisible
  in our layout and would have been written back as `<*7>`; the written
  T02 showed the difference in Ami Pro. `</m>` is now U+2010 HYPHEN, which
  the writer writes as `</m>`, and layout breaks after it. Measured: the
  user's corpus and the VM fixtures unchanged; the ISO corpus 6 fewer
  paragraph line counts equal (22,773 of 24,971) and 3 fewer page ends
  (498 of 1,307), all in the online 1-2-3 manuals, whose style sheet is
  missing and whose layout is already far from Ami Pro's.
- **Styles of a missing style sheet (4.3).** A document that names a
  style sheet that is not available keeps its styles there: the default
  styles the importer creates for it, and a Table Text generated for its
  tables, now count as the sheet's (origin Sheet), so the writer does not
  store them in the document, where they would override the real sheet's
  in Ami Pro. Generated styles take no function key (Ami Pro gives a new
  style a free one).

## 2026-10-13 — Page layouts in the SAM writer (Phase 4.5)

- **The layout a document starts in is model data** (`Document::
  start_layout`, native v3 `startlayout`, `docs/Design/NATIVE_FORMAT_V3.md`).
  Ami Pro's `[l1]` names it; 27 corpus files start in an inserted layout,
  and Ami Pro's own page tables name that layout on their pages (one ISO
  manual: every one of its 20 pages in layout `1`, a 7 × 9 in page). The
  importer ignored `[l1]`; the layout engine now starts page 1 with it.
  Measured: user's corpus page counts 299 → 302 of 394 and page ends 744 →
  762 of 1,108; VM fixtures unchanged (136/136, 76/76); ISO corpus page
  ends 498 → 492, all in manuals whose style sheet is missing, whose page
  counts were already off (16 and 13 pages for Ami Pro's 20 and 16, now
  23 and 19 on the right page size).
- **A sheet's Standard layout stays the sheet's.** A linked SAM that stores
  only inserted layouts (7 files) takes Standard from its sheet; the
  importer put the inserted layout first and dropped Standard. Now the
  layout named Standard comes first, from the document or else the sheet,
  and the writer stores a sheet's Standard only once it has changed, as
  Ami Pro does (`11c8:3ba0`): what it was when read is kept as a
  fingerprint (`SHEET_LAYOUT_KEY`), which Use Another Style Sheet and Save
  as a Style Sheet drop.
- **Inserted layouts are renamed `1`, `2`, … as written** (`11c8:3ba0`),
  and page breaks and `[l1]` name them so.
- **Kept values are what differs from the writer's defaults.** The reader
  keeps a LAYOUT block's values 3, 6, and 10–14 and frame fields 1 and 8
  only when they are not what the writer writes without them, and the text
  flag 0x200 (exactly the frames that hold text, 2,989 of 2,989) is
  derived, not kept: a layout made in the app now reads back as it was.

## 2026-10-13 — Caches, Ami Pro's reading rules, and Save As SAM (Phase 4.9–4.10)

- **Paragraph caches are kept with a fingerprint (4.9, as 4.1 decided).**
  A paragraph keeps the `<:#H,W>` it was read with and a fingerprint of
  itself and its style then (`Paragraph::cache`, native v3 `pcache`); the
  writer writes the cache back only while both are unchanged, and never
  computes one (Ami Pro trusts given caches). Line heights calibrated from
  the caches now read back, so the user's corpus reads back equal in 416
  of 416 documents.
- **The VM is part of every writer slice's evidence, not only the round
  trip.** Our reader is more forgiving than Ami Pro's in places, so a
  written file can read back equal here and still fail in Ami Pro: four
  such rules were found by opening written files in the VM (a cell's text
  flag, a table frame's `[frmlay]` value 1, a note's closing line, a
  frame's lines bit; `SAM_FORMAT_NOTES.md`). Where Ami Pro's reader is
  stricter, ours now warns, and the round-trip audit counts read-back
  warnings the original did not raise.
- **Save As SAM (4.10, decisions A and B), implemented as one file type
  choice and one dialog** (`UI_SPEC.md` §17): the portal dialog offers
  `.amipro` and `.SAM` filters and a Format choice; a typed extension
  decides first. A `.SAM` is written only to a new file (temporary file,
  then a hard link that fails if the name exists), and a document saved
  so keeps its SAM as a source that Save never writes over. The loss check
  writes the document in memory, reads it back, and lists each kind of
  change once; Cancel is the suggested button.

## 2026-10-13 — With contents, and documents edited before writing (Phase 4.11)

- **Correction to 3.6: a style sheet marks every frame it writes.** 3.6
  kept 0x8 (layout frames) and 0x80 (the layout number) as read. The
  decompiled sheet writer sets both on everything it writes (`11c8:14e0`),
  so a sheet made from an app-made layout now has them too, and without
  contents its header and footer text is left out, as Ami Pro's own SS2N
  shows. The document writer still writes them as read. Tables in layout
  frames are now written without contents too, as `2a0f` does.
- **With contents** is a check box in Save as a Style Sheet, on each time
  (Ami Pro's default): the sheet carries the main text, the page frames,
  the header and footer text, and the pictures, as `11c8:14e0` writes them;
  this project's sheets equal Ami Pro's (SS2W, F01W) but for `[desc]`.
- **New tables use Table Text** (`Document::table_text_style`, made from
  Body Text with no space above or below and no indent when the first
  table is): Ami Pro puts all cell text in Table Text, and a cell written
  untagged reads back as Table Text.
- **The reader keeps only what the writer would not write the same.** After
  import, an anchored frame's stored rectangle and fields 15 and 16, a
  table's header and row and column records, and a cell's record are
  dropped when the writer, asked without them, gives the same values
  (`frames::drop_derived_values`); the round trip on both corpora is
  unchanged, and a frame or table made in the app reads back as made, so
  the loss check does not list it.
- **A style defined nowhere has no definition to compare.** Ami Pro formats
  such a name as Body Text (ST1) and nothing about it is written; the
  editor and the reader each give it a definition of their own, which the
  round-trip comparison and the loss check now leave out.

## 2026-10-13 — The user's check of Save As SAM: a loss list for a document that loses nothing

- **What the user saw.** Saving a document from their corpus as an Ami Pro
  document listed "2 page layouts, 1 document setting", though the
  round-trip audit reads that document back unchanged. The app had opened
  it without its style sheet (the style-sheet folders preference was empty,
  and the sheet sits two folders up), and the loss check then read the
  written copy back against a sheet it generated from the document: that
  sheet's Standard layout became a second layout the document never had.
  The written file itself was right (equal to the original but for
  `[desc]` and the order of two closing controls).
- **The loss check reads back as the document was read.** A document whose
  sheet was not found is marked (`STYLE_SHEET_MISSING_KEY`, removed by Use
  Another Style Sheet and Save as a Style Sheet) and read back without a
  sheet; Ami Pro uses its own copy of the sheet, which the written `[sty]`
  names. The generated stand-in sheet for the others is written with
  contents, so it carries the header and footer text the document takes
  from its sheet (4.11's marking left it out otherwise: 4 more user
  documents and one VM fixture would have listed a layout).
- **Sheets two folders up are found.** The importer also looks in a
  `styles` folder two levels above the document (documents filed in a
  client folder beside `STYLES`); no other corpus document changes which
  sheet it finds.
- **The audit measures what the user sees.** `sam_round_trip` now runs the
  app's own loss check on every document: Save As lists nothing for 416 of
  416 user documents (with and without the style-sheet folder), 136 of
  136 VM fixtures, and 159 of 160 ISO documents (the malformed layout).
  New example `loss_check` shows what Save As would list for one file and
  why.

## 2026-10-13 — A refused or failed save says so in a dialog

The user's recheck of Save As SAM passed; saving over the same `.SAM` was
refused as decided (A), but only the status bar said so, which the user
would not have seen without looking for it. Every refused or failed save
(Save, Save As, Save As SAM, Save as a Style Sheet) now opens a dialog
with the reason, Choose Another Name… (the file chooser again, with Save
as a Style Sheet's description and With contents kept), and Cancel; the
status bar keeps the message (`UI_SPEC.md` §17).

## 2026-10-13 — Writing over an existing SAM: not yet (user decision)

Asked why Save As refuses an existing `.SAM`, the user was given the
reasons (the originals may be the only copy, the writer was new, and the
project's sources are read-only) and three options: keep new files only;
allow writing over a SAM after a confirmation, keeping the original as a
backup the first time; or allow it without a backup, as Ami Pro's Save
does. The user chose to **keep new files only for now and revisit it**
("all of my files are backed up, but I do understand the logic"). The
recommendation on record for that revisit is the second option, with the
loss check still run first.

## 2026-10-13 — Page layout setup: the user's answers (`UI_SPEC.md` §18)

- **L1:** a rail panel, live, one undo step per change (yes).
- **L2:** the commands under Format, as everything else (no Page menu).
- **L3:** floating headers and footers and the page-number field as small
  one-shot dialogs, "as long as you think it matches the new interface":
  it does; the interface keeps dialogs for one-shot actions with nothing
  to watch (Go To, Create Style, Use Another Style Sheet, Save as a Style
  Sheet) and rail panels for what changes live.
- **L4:** the panel follows the caret's page while open (yes).

## 2026-10-13 — Page layout setup: choices made during the run

The run's decisions were taken before it (L1–L4). Where the model or the
approved interface differs from Ami Pro's dialogs, the run chose as below;
each is in `docs/Design/UI_SPEC.md` §18's implementation notes with its
reason, and none needed the user.

- **Evidence:** Ami Pro's dialogs are in `AMIPROUI.DLL` (packed like
  `AMIPRO.EXE`) and its number boxes in `AMIUICNT.DLL`; both decompiled
  (`docs/research/PAGE_LAYOUT_NOTES.md`). Twins made in Ami Pro decide what
  is written; Ami Pro's screen decides where page lines are drawn.
- **The page-number field** is a style character, a start page, and a
  start number (`<:Pi2,1,Page >`); the model's numeric "style" was a
  misreading. Old native v3 records still load.
- **One tab list per layout box**, used by every column, as Ami Pro's
  reader keeps it; the panel edits that list.
- **Lengths** in every rail panel are read as Ami Pro's boxes read them
  (two decimals, truncated, 567 twips a centimetre).
- **Kept as the model holds it, not as Ami Pro's dialog:** A3/A4/A5/B5
  columns even with the margins (Ami Pro's dialog leaves them up to 5 twips
  inside); line style, position, and colour only while a line is on; the
  band shrink on the page kind edited; top and bottom margins leaving some
  height.
- **Interface:** Revert is the From the next page list's earlier layouts;
  Page Number… changes a field at the caret as well as inserting one;
  choosing Right or Left is not a change until a setting changes; All
  while left pages differ is one undo step that takes the left headers and
  footers with it, as Ami Pro's OK does.
- **Engine:** page lines and the line between columns are drawn (Outside
  on the page edge, Inside on the margin, the others at a quarter, a half,
  three quarters; AP1–AP5); a page break balances the columns it ends, and
  the line between balanced columns is as tall as they are (SETUP).

## 2026-10-13 — The page layout setup: physical check passed

The user ran every step of the handoff's check (Page Layout Properties:
size, orientation, margins, columns, lines; Pages and Mirror; Begin on
second page; Go To Header; breaks; Floating Header/Footer…; Page
Number…; Insert Page Layout and Remove; the margin's right-click and
double-click): "I didn't find any issues. Nothing feels off - it all
looks amazing." The one question it raised (a page number's font) was a
defect, fixed the same day (`d9dde32`, VM fixture PF1).

## 2026-10-13 — The ruler: the user's answers (`UI_SPEC.md` §19)

The user: "I agree with your recommendations".

- **R1:** tabs set on the ruler go into the paragraph's own ruler, as in
  Ami Pro (the caret's or the selected paragraphs; Enter carries it on);
  the style's and the page layout's tabs stay in Style Properties and
  Page Layout Properties.
- **R2:** unequal columns are allowed (drag a gap or a column), with the
  model change §19 describes; imported unequal columns then lay out as
  Ami Pro shows them.
- **R3:** the ruler is always live, with a tab-type selector at its left
  end and right-click menus; no click-to-enter ruler mode.
- **R4:** the vertical ruler later, not in this run.


## 2026-10-17 — The ruler: choices made during the run

No question came up that R1–R4 and the earlier decisions do not cover;
these are the run's own choices, each from the evidence in
`RULER_NOTES.md` or, where there was none, the conservative one.

- **Unequal columns in the model:** each column's edges from the box's
  text area's left (`LayoutBox::column_edges`), kept only when widths or
  gaps differ by more than a twip, Ami Pro's own test for alike gutters
  (`1238:0000`). Near-even stored edges (a few twips off the margins) stay
  even columns with their exact edges kept as offsets for the writer, as
  before. Native v3 gains an optional `edges` record.
- **What changes do to them:** the ruler moves only the edges dragged
  (`1148:0d0c`); the panel's margin makes alike-gap columns even and
  otherwise moves only the outer edge (`1238:02d0`; two columns have one
  gap, so the panel evens them); a column count or gutter makes them even;
  Mirror reverses them. A page size or frame size change moves the last
  column's right edge with the right margin, or makes the columns even
  when they no longer fit: no evidence (Ami Pro's Page settings and frame
  resize were not traced for uneven columns).
- **Text › Normal keeps a paragraph's own ruler** (`12e0:132b`; the app's
  Normal had cleared it). **The SAM writer derives `<:R>`** after a ruler
  paragraph (`11b0:1405`); the importer drops the marks it derives,
  re-takes the fingerprint of a cache that held, and leaves them out of
  the unmapped-controls count, so the round-trip audit is unchanged.
- **Which paragraphs:** a selection that ends at the very start of a
  paragraph leaves that paragraph out (`12e8:01da`), except an empty one
  for tabs (`1170:052d`); the tabs in effect come from the caret's
  paragraph, or the one before in that case (`1148:1c21`).
- **Interface details** (within §19): markers are taken within 5 pixels
  (Ami Pro's 29 twips are about two pixels at 100%); a tab is removed when
  released 8 pixels below the ruler (onto the page) or above it; the kind
  button cycles the four kinds on a click and offers kinds and leaders on
  a right-click; the keyboard mode follows the user's guide's keys but
  leaves out Space+arrow (moving a tab is a drag); hover text goes to the
  status bar.
- **Not built:** in a table cell Ami Pro's ruler also drags the table's
  column widths (`13d0:0192`); the app's ruler shows only tabs and
  indents there, with table cell formatting still to come.
- **Found for later:** column balance moves finished lines between
  columns without re-breaking them, so with uneven columns and Column
  balance a line moved into a column of another width keeps its width
  (one user document).

## 2026-10-17 — The gutter follows the columns (the ruler's physical check)

The user's physical check found Save As listing "1 page layout" after
column drags on the ruler. Ami Pro stores no gutter; it works it out from
the columns (`1238:0000`), so a file can give back only that value.
Decided, from the decompile:

- **The model's gutter is what Ami Pro works out** wherever it cannot be
  anything else: 240 for one column, the first gap for uneven columns
  (`LayoutBox::settle_gutter`, called by every edge change and by Mirror).
- **A column count and Even Columns take Ami Pro's count gutter**
  (`1238:008a`): the gap the columns share, else 240. Even Columns used
  the box's own gutter when the gaps differed, a value Ami Pro has no
  equivalent for; the ruler's Columns stays 240 (`1148:1937`, RU11).
- **Columns a twip apart are even** when made on the ruler, as the
  importer reads them (`uneven_edges`); before, only exactly even ones
  were, so a near-even drag read back differently.
- **Frame Properties hides and refuses the gutter for one column**, as
  Page Layout Properties does.
- **The loss check compares the settled gutter**, so a document saved in
  the app's format before the fix lists nothing for it.

## 2026-10-17 — A tab under an indent marker (the ruler's physical check)

The user's physical check found that a tab's right-click menu would not
change its kind: after the first-line indent was dragged onto the stop at
0.5 in, a press there took the indent marker, whose menu is the ruler's
general one (its New Tabs ▸ kinds change only tabs added later). Ami Pro
tries tabs before the indent markers (`10d0:090c`). Decided:

- **A right-click in the upper half takes the nearest tab** within reach,
  wherever indent markers stand; indent markers have no menu.
- **A left press takes the tab first, as Ami Pro does**, except where the
  first line's triangle and the bar are drawn (the top 70% of the upper
  half), so that an indent marker standing on a tab can still be dragged
  (in Ami Pro only the part of the triangle left of the stop can).

## 2026-10-17 — The ruler: physical check passed

After the two fixes above, the user rechecked both ("both test passed")
and then ran the rest of the handoff's check: Edit › Go To Ruler's keys,
the ruler in a text frame and a header, and View › Ruler ("step 6 and 7
passed"). The ruler run is closed for its physical check.

## 2026-10-17 — The frame check waits for frame creation

The user: "lets wait until I can create frames to check moving and
resizeing". The physical check of frame interaction (`UI_ACCEPTANCE.md`
§10) waits until frames can be created in the app (Insert › Frame…, still
Future). Asked whether creating frames should come before frames in
table cells, the user: "yes, swap them so creating frames comes first".
Creating frames is next in `PROJECT_PLAN.md`, then frames in table cells.

## 2026-10-17 — When bullets and numbering come in

Bullets and numbers already work through paragraph styles, as Ami Pro's
do (Phase 3.2e, 3.6). Asked when the rest comes in, the user agreed to the
recommendation ("yes, add them to the plan that way"):

- **Format › Bullets and Format › Numbering:** a short run right after
  creating frames, with a short proposal first. Ami Pro keeps bullets only
  in styles, so the proposal asks how the commands use them. The bullet
  display gaps (right-aligned numbers, superscript bullet text, picture
  bullets) can join it.
- **Outline numbering and Outline mode** (the Outline Styles dialog,
  Quick Numbering, promote, demote, collapse; user's guide ch. 11): Phase 5.

## 2026-10-17 — Creating frames: the user's answers (`UI_SPEC.md` §20)

The user took the four recommendations (asked together, before the run):

- **F1:** frames are created both ways, as in Ami Pro: Insert › Frame…
  opens the small Create Frame dialog (Width, Height, Down from top, In
  from left, remembered from the last frame, with Draw on Page), and
  Insert › Draw Frame goes straight to drawing on the page; Draw Frame is
  added to the Standard toolbar, as in Ami Pro's default SmartIcons.
- **F2:** Frame Properties gets Make Default for New Frames and Restore
  Ami Pro's Default; the default (wrap, display, lines, colours, shadow,
  corners, margins; never placement, size, columns, or tabs) is kept in
  the app's settings for every document, as Ami Pro keeps it in
  AMIPRO.INI.
- **F3:** Where placed frames are drawn and hit above anchored frames, as
  Ami Pro draws them (FC8); Bring to Front and Send to Back work within
  each kind; the SAM writer lists frames in Ami Pro's order.
- **F4:** pictures (Insert › Image…, Ami Pro's Import Picture) later, in a
  run of their own with picture decoding; this run creates text frames.

## 2026-10-17 — Creating frames: choices made during the run

No question came up that F1–F4 and the earlier decisions do not cover;
these choices were made within them, by Ami Pro's rules
(`docs/FRAME_CREATION_PROGRESS.md`):

- **The page a dialog frame goes on** is the page at the workspace's
  centre: Ami Pro uses the page in view, not the caret's (FC9).
- **A click while drawing** is a release within 3 px of the press (Ami Pro
  compares twips; a pointer jitters).
- **Layering beyond F3:** each page frame is drawn in a layer of its own,
  so an opaque frame hides the text behind it (the renderer drew all
  quads before any text); rounded corners are drawn. Both were visible on
  the first screenshot of a frame created over another.
- **Ami Pro's text area for rounded frames** (`1530:0892`): one rule in
  the model for the writer, the importer, and the engine, which moved no
  page end and brought 15 files' frame columns nearer Ami Pro's.
- **A page frame's `[frmlay]` values 10 to 14** default to what Ami Pro
  writes for a new frame (`2 0 0 0 0`) rather than a header's; importer and
  writer agree, so documents read back as before.
- **A new text frame is "never typed in"** (a kept `txt none`, as the
  importer marks a `[txt]` without paragraphs), and the round-trip
  description ignores the mark once the frame has text.
- **Make Default's button** is labelled "Make Default", Ami Pro's label,
  under a "New frames" heading: the approved longer label did not fit the
  rail.

## 2026-10-17 — Bullets and numbering: the user's answers (`UI_SPEC.md` §21)

Ami Pro 3.1 has no Bullets or Numbering command: it keeps bullets and
numbers in paragraph styles, and has Edit › Insert › Bullet… for a bullet
typed into the text (`docs/research/BULLETS_NOTES.md`). The user asked
that the commands improve on Ami Pro's where Ami Pro's are lacking, asked
whether there should be style bullets "but also manual" (there are both:
style bullets and numbers, and manual bullets; Ami Pro's format has no
manual number), and asked whether the work fits the move of Ami Pro's
large dialogs into side panels (it does; B5 links the toggles to the
panel). Asked together, before the run:

- **B1:** Format › Bullets and Format › Numbering apply Ami Pro's list
  styles: the document's Bullet 1, else Bullet, and its Number List. When
  the document has none, the command creates it in the document as
  `_DEFAULT.STY` defines it, in the document's Body Text font, and applies
  it, as one undo step. Off (every selected paragraph already shows one)
  returns the paragraphs to Body Text; the other kind switches.
- **B2:** both on the Standard toolbar as toggles, after the alignment
  group, with icons of the app's own (the COSMIC theme has none);
  Numbering gets Ctrl+Shift+N (Bullets keeps Ctrl+Shift+L).
- **B3:** Enter in an empty bullet or number paragraph ends the list (the
  paragraph returns to Body Text); elsewhere Enter keeps Ami Pro's Next
  style.
- **B4:** Insert › Bullet… in this run: a small dialog with Ami Pro's 17
  bullets drawn as buttons and an example, remembering the last; also in
  Customize Toolbar, as Ami Pro's "Insert a bullet" SmartIcon.
- **B5:** Format › Bullet and Number Style… (also in the text right-click
  menu) opens Style Properties at its Bullets and numbers section for the
  caret paragraph's style.
- **Also in the run, without a question** (fidelity and the earlier
  conventions): inserted bullets (`<:B…>`) imported, drawn, edited as one
  character, and written back; Right align, superscript bullet text, and
  the seven picture bullets drawn as Ami Pro draws them (the pictures as
  the app's own shapes, never Ami Pro's metafiles); Space for in Style
  Properties (the First indent); check marks on both toggles.

## 2026-10-17 — Bullets and numbering: choices made during the run

No question came up that B1–B5 and the earlier decisions do not cover;
these choices were made within them, by Ami Pro's rules
(`docs/BULLETS_PROGRESS.md`):

- **A created style takes the next free function key**, in Create Style
  as in the list styles the toggles make: Ami Pro gives F2–F12 but F10
  (`10a8:0000`; VM fixture BL4). Until now Create Style gave none (0),
  which Ami Pro replaced with the next free key when it read the file.
- **A made list style writes `[spec]` value 4 as 0**, as Ami Pro 3.1
  writes a style it creates (BL4); `_DEFAULT.STY`'s 360 is Ami Pro 2's
  Space for, which the reader moves into First.
- **Right align and superscript** follow the decompile: the paragraph's own
  indents when it has them, the space measured in the bullet text's font
  (BL3), superscript never for asterisk and dagger numbers.
- **The picture bullets** are shapes the app draws (a pointing hand, a
  shadowed box, a box with a thick top, a checked box, a crossed box, two
  arrowheads), never Ami Pro's metafiles; every bullet is now drawn as
  shapes, in the dialog as on the page.
- **A copied inserted bullet** is its nearest Unicode character in the
  plain-text clipboard (the clipboard holds no objects).
- **The Standard toolbar** is about 935 px wide with the wide style field;
  below that the field narrows, so all its groups still show at the 900 px
  minimum window.
- **Found and fixed:** after a selection, a click left the hidden text
  editor's selection in place (the pinned iced `move_to` never clears
  one), so the style field read the old range.


## 2026-10-17 — Outline levels: the user's answers (`UI_SPEC.md` §22)

Asked how a numbered list moves between levels, the user agreed to a
short run ahead of frames in table cells, and answered (together, before
the run; `docs/research/OUTLINE_NOTES.md`):

- **O1:** Format › Promote and Demote with Alt+Left and Alt+Right only,
  in the normal page view; Tab stays a tab.
- **O2:** demoting an item of a list made with the Numbering button makes
  a multi-level list: 1. → A. → i. → a. → 1) → a) → i) → (1) → (a), each
  depth's style made as needed, deeper levels restarting under each item,
  with Ami Pro's limit that the top level restarts only after a heading
  with an outline level.
- **O3:** outline level, reset options, and cumulative numbers in Style
  Properties, and a small Format › Styles ▸ Outline Numbering… dialog for
  whole schemes (1/A/i, Ami Pro's I/A/1/i/a and 1/1.1/1.1.1, Clear).
- **O4:** Promote and Demote on the Standard toolbar (told: under about
  970 px the last group moves to More).
- **The Format menu** into submenus (Alignment ▸, Bullets and Numbering ▸,
  Frame ▸), after the user's remark "The format menu is getting very
  tall"; a separate Frame menu offered as the alternative.
- **Made within O2** (to keep Ami Pro's counting by level from mixing a
  list with headings): a nested list's depths use the outline levels below
  the deepest level the document's styles use; its styles are named
  "List 1"…"List 9".

## 2026-10-18 — New documents' Body Text spacing and shaded margins

After the user's report that typing seemed to stop in a header (fixed in
`f3f3ac1`; VM HDR1, HDR2), two questions, both answered yes:

- **New documents' Body Text has no space below**, as `_DEFAULT.STY`'s
  (`[spc]` 33 273 1 0 0 1 100: single, nothing above or below), so a new
  document's header band holds two lines as in Ami Pro. Until now it had
  6 pt below, the project's own choice since Phase 3. Headings keep theirs.
  Imported documents are unchanged: a document whose styles are all
  missing keeps the 6 pt stand-in the layout comparison was measured with.
- **The page margins are shaded, as Ami Pro's Layout mode shows them**
  (View › View Preferences › Margins in color, on in the VM: 223, 223, 223
  outside the text area; column gutters stay white, G05). The paper is
  white in both themes, so the shade is the same in both. It shows where
  the header band ends and the main text begins.

## 2026-10-18 — Lists accepted; bullets nest; colours (`UI_SPEC.md` §23)

The user, after the combined check: "All list items checked" (bullets and
numbering, and outline levels: PHYSICAL PASS), and:

- **Bullets nest** as numbered lists do (Alt+Right on a bulleted line
  gives a second-level bullet, further in), as recommended: styles
  "Bullet 2" … made as needed, the shapes going diamond, dot, small
  square.
- **The Frame menu stays folded** under Format › Frame ▸ ("Frames feel
  okay where they are now"), and the Format menu is short enough.
- **Colour selection** is to be "more like modern color selectors. Easier
  to choose wide variety of colors, but maintain your current pallet so
  you don't have to keep finding the colors you are using", including "a
  number of color options around frames". Answers:
  - **C1:** one colour button in every place a colour is chosen, opening
    a colour panel in place.
  - **C3:** a colour applies at once in the rail; dragging in the picker
    shows it live and leaves one undo step; the Font dialog applies with
    OK.
  - **C2 and C4:** the user chose "Colours in this document" and "Per
    kind", then, asked which combination was meant, deferred: "What do
    you think would be most useful for someone". Made: Ami Pro's 15
    colours (a new document has none in use), **In this document** (every
    kind together, most used first, so a heading's colour is offered for
    a frame line), **Recent** (one list for the app, kept in the
    settings), then More colours (libcosmic's picker with `#RRGGBB`).
  - Every frame colour uses it, as Ami Pro's Modify Frame Layout does:
    line, shadow, background, and the line between columns (its own
    colour, which the app did not have).

## 2026-10-18 — Frames in table cells: the user's answers (`UI_SPEC.md` §24)

Before the frames in table cells run (`docs/research/CELL_FRAME_NOTES.md`:
the decompile's three frame lists, the cell's values 11 and 12, Ami Pro's
table grid; VM fixtures TF1–TF15), the user answered, together:

- **T1:** a picture in a cell behaves as in Ami Pro, with undo: a click
  puts the caret in the cell and outlines the cell; no handles, no drag,
  no Frame Properties, and Format › Frame ▸ and Select Next Frame leave it
  out; Delete removes the picture as one undo step (Ami Pro cannot undo
  it); Backspace and typing do nothing, and the status bar says Delete
  removes it.
- **T2:** every cell's picture is laid out at its cell's size and written
  so, including the ten in two of the user's files that Ami Pro stored
  with no height and shows as nothing (Ami Pro itself fits a picture to
  its row when the row next changes, TF1YT).
- **T3:** pictures go into cells with Insert › Image… in the pictures run
  (the caret in an empty cell, as Ami Pro's Import Picture); this run adds
  no way to make one, and Insert › Frame… and Draw Frame keep making page
  frames, as Ami Pro's Create Frame does.
- **Made from the evidence, not asked:** tables are laid out on Ami Pro's
  grid (the table's insets, each row's and column's gutter, the first
  column 43 in whatever the gutter), which moves cell text in the 161 of
  603 corpus tables whose gutters are not 86; the frames are kept in the
  frame stack at its front in Ami Pro's list order, so Save As writes them
  first and numbers them from it.

## 2026-10-18 — Phase 5 playbook: improvements on any item; import with export

The user asked for a playbook for Phase 5, grouped as makes sense
(`docs/PHASE5_PLAYBOOK.md`, seven playbooks in `docs/playbooks/`), and
added two rules while it was written:

- **Improvements on any item:** "any item can [be] improved upon if there
  is a better way to do it or logical functions to be added". Every run
  looks for a better way and for logical additions to each Ami Pro item,
  not only where Ami Pro is lacking (bullets, 2026-10-17), and offers them
  as decisions with a recommendation in its up-front batch; Ami Pro's own
  behaviour is never dropped silently, and what SAM cannot hold stays in
  the native format and is listed by Save As.
- **Import with export:** "Put importing other word processors as a to-do
  item. Logically whatever is chosen for export we should also make an
  import for". Import is the queue's run 7 (`import`, Output and exchange
  playbook), right after export, reading the format the export run chooses
  (DOCX recommended there) with the same mapping run backwards; Ami Pro's
  File › Import Text… is its reference for Insert › File…. It is no longer
  in the playbook's "Not planned" list.
- **Made by the agent, not asked:** the queue's order is a recommendation
  (P0 items and what the user's files show most first, foundations before
  their dependants); the user may reorder it at any time.


## 2026-10-18 — Pictures: the user's answers (`UI_SPEC.md` §25)

Before the pictures run (`docs/research/PICTURE_NOTES.md`: Import
Picture `10d8:126c`, Graphics Scaling `1168:0000` and `FRAMESCALEPROC`,
the picture's rectangle `1168:2dd2`, the `[isd]` record `11c8:2e29`, the
drivers `AMIPAINT.DLL` and `AMIWMF.DLL`, Ami Pro's rendering
`1168:2016`; VM fixtures PI1–PI9, PIR1–PIR5), the user answered P1–P4 in
one batch and P5–P8 in the next:

- **P1 Formats:** Insert › Image… takes BMP and WMF (Ami Pro's own) plus
  PNG, JPEG, GIF, TIFF, PCX, and SVG; the native format keeps each file
  as inserted.
- **P2 Copy or link:** copied into the document by default, **with a Link
  option** (the user's choice over the recommendation, always copy): the
  file chooser's "Link to file"; the native format keeps the path and
  draws the file as it is now; Save As SAM embeds a copy, since Ami Pro
  cannot follow a Linux path, and lists it. Linked pictures in old files
  are found by their path, then by file name in the document's folder
  (any case), else drawn from Ami Pro's stored screen image, else a box
  naming the path.
- **P3 A new picture's frame** (caret in text, no frame selected): the
  picture's size at its own resolution, reduced to fit the column and the
  page, in place of Ami Pro's remembered Create Frame size; Ami Pro's
  frame style, Flow with text, and Fit in frame with aspect kept stay.
- **P4 Drawings, charts, OLE objects** until their runs: Ami Pro's stored
  screen image where the file has one, else a labelled box.
- **P5 Graphics Scaling** in Frame Properties › Picture (the rail), with
  Rotate as Ami Pro's; Format › Frame ▸ Graphics Scaling… opens it there.
- **P6 Moving a picture in its frame** as Ami Pro (double-click or Enter,
  drag, arrows 0.05 in, Ctrl+arrows 0.5 in, Esc), plus Shift and a corner
  handle sizing frame and picture together (Custom).
- **P7 Additions:** all four groups: paste an image from the clipboard and
  drop a picture file on the page; Replace Picture… and Save Picture As…;
  Fit Frame to Picture and Reset; a description (alt text, native only,
  listed by Save As).
- **P8 Ami Pro files:** pictures in formats Ami Pro cannot store are
  written as Windows bitmaps, reduced when above 300 dpi at their size on
  the page, and Save As lists each conversion; the native format keeps
  the originals.
- **Made from the evidence, not asked:** pictures Ami Pro stored are
  written back as read while unchanged (the `[isd]` values and the
  driver's state); a new or changed picture is written as Ami Pro's record
  with no driver state and no screen image, which Ami Pro rebuilds (PIR1,
  PIR3). Decoders that need libraries live in the app, as the shaper's
  hyphenation does, so the core crates stay dependency-free: `image`
  (PNG, JPEG, GIF, TIFF), `image-extras` (PCX), `resvg` (SVG), and
  `tiny-skia` (drawing); the Windows bitmap and metafile readers and Ami
  Pro's screen image are the app's own, dependency-free, in a new core
  crate `amipro-pictures`, so that print and export can share them. The
  metafile reader is the app's own because the two maintained crates drew
  69 and 63 of the corpora's 72 distinct metafiles (`PICTURE_NOTES.md`,
  "WMF libraries").


## 2026-10-18 — Tables across pages: taken overnight by the rulebook; to confirm (`UI_SPEC.md` §26)

The overnight rulebook (`docs/SESSION_HANDOFF.md`) takes the recommended
choice for every decision of the tables across pages run and records it
here to confirm. Evidence: `docs/research/TABLE_PAGES_NOTES.md` (VM
fixtures TP1–TP11, the user's guide ch. 16, the decompiled row flags and
page table). All five keep Ami Pro's behaviour and its file format, and
each is cheap to change.

- **TP1 Insert › Page Break in a table cell** (taken overnight by the
  rulebook; to confirm): sets Ami Pro's page break on the caret's row
  (`[h]` flag `0x20`: the row starts the next page), one undo step; the
  item is checked while the caret is in a row with a break, and choosing
  it there removes the break. Before, the app refused a page break in a
  cell. Alternative: a Breaks dialog with Ami Pro's Insert and Remove.
- **TP2 Headings, Rows span pages, Automatic** (taken overnight by the
  rulebook; to confirm): read from files, kept in the model and native
  v3, laid out, and written back; their commands (Table › Headings,
  Modify Table Layout) wait for the table editing run (queue 8), where
  the table interface is decided.
- **TP3 A row taller than a page** without Rows span pages (taken
  overnight by the rulebook; to confirm): cut to the page as Ami Pro does
  (TP4), its lines past the page hidden. Alternative: split it anyway in
  the app only.
- **TP4 The caret in a repeated heading** (taken overnight by the
  rulebook; to confirm): it goes in and edits the heading row, as in Ami
  Pro (TP2).
- **TP5 A table in a frame** (taken overnight by the rulebook; to
  confirm): shows the rows its frame holds and hides the rest, as Ami Pro
  does (TP8); frame tables never continue.

**Confirmed by the user on 2026-10-19** (all five as taken; below), and
the physical check passed the same day.


## 2026-10-19 — The morning report's answers (user decisions)

The user answered the overnight run's morning report
(`docs/SESSION_HANDOFF.md`) with "all of this sounds good": every
recommendation is taken.

- **TP1–TP5 confirmed** as taken overnight (the entry above).
- **Find & Replace, run 3** (`UI_SPEC.md` §27, `FIND_NOTES.md`):
  - **F1** a find bar over the pages: Find, Replace, ▲ ▼, "3 of 12",
    Options ▾, Replace, Replace All; Ctrl+F, Ctrl+H, F3, Shift+F3, Esc.
  - **F2** Ami Pro's defaults (Whole word only and the other text streams
    on), remembered between sessions.
  - **F3** Ami Pro's capitals rule by default (the replacement follows
    the found text's capitals: "Status bar" → "Window Pane"), with a
    switch for "as typed".
  - **F4** matches highlighted with a count, Replace All as one undo step
    (Ami Pro cannot undo it, FR4), and recent searches; no regular
    expressions yet.
- **Text on both sides of a Wrap around frame: merge** the branch
  `overnight-wip-strips` (VM WB1–WB3; measured on the branch: the user's
  page counts 312 → 315 of 394, page ends 777 → 762 of 1,108, the loss in
  one long manual already 16 pages longer than Ami Pro's). Merged on
  2026-10-19 (`f6ee32f`) when the user said so explicitly.
- **Rows measured taller than Ami Pro's: trust Ami Pro's stored row
  height while a row's cells are as read** (a cache, like the paragraph
  caches the writer keeps), built and measured before it is merged.

## 2026-10-20 — The row cache, as built

The user decided on 2026-10-19 to trust Ami Pro's stored row height while a
row is as read, measured before merging. VM fixtures RC1–RC3 then showed
that Ami Pro measures every Automatic row when it opens a file (growing and
shrinking it), so a stored height is Ami Pro's own measure, and a height
the text does not fit is stale (`TABLE_PAGES_NOTES.md`, "The row cache").
Built accordingly: a row as read (`Table::read_rows`) keeps its stored
height while its text measures here at most half the row's gutter taller,
the text running into the gutter above the line under the row; a stale
height, or a row edited since, is measured as before. Measured and merged
(`docs/TABLE_PAGES_PROGRESS.md`, "The row cache"): no measure worse; the
ISO corpus gains 4 page ends and a page count, the user's corpus 42 rows.
Parked for the table editing run: edited rows fitting their text exactly
(RC2) and empty cells taking a line (RC3).

## 2026-10-20 — Find & Replace: choices made in the run (FD1–FD5)

The run's decisions were the user's (Find F1–F4, 2026-10-19). These
smaller choices were made while building it, each Ami Pro's behaviour
where it has one (`docs/FIND_PROGRESS.md`; `UI_SPEC.md` §27,
implementation notes):

- **FD1** A return in Find (Ami Pro's ¶, typed with Insert Return) matches
  a paragraph's end and also a line break (Ami Pro's Ctrl+Enter in text);
  in Replace it makes a paragraph break.
- **FD2** Whole word only counts letters and digits as a word's: a match
  needs neither right before nor right after it (FR1B).
- **FD3** The capitals rule: a found text of two letters or more, all
  capitals, gives the replacement in capitals; a first capital letter
  capitalizes each word of the replacement; otherwise small letters (FR1,
  FR1B). A single capital letter capitalizes.
- **FD4** Opening the bar on a selection of fewer than 64 characters in one
  paragraph fills Find with it (the proposal); a longer selection, or one
  over paragraphs, limits the search to it (Ami Pro's rule), shown as
  Options ▾ › In the Selection, which can be switched off.
- **FD5** The current match is whichever match is selected, however it
  came to be (Find Next, Undo, a click and drag); Replace replaces it.

## 2026-10-20 — The find bar above the ruler (user, after the physical check)

The user's physical check of Find & Replace passed. Their visual notes:
the Find and Replace fields did not line up, and the bar seemed to belong
above the ruler; "improve it visually to match the rest of the
application". Done: the bar is a chrome row like the toolbar's between the
tabs and the ruler (it was under the ruler, `UI_SPEC.md` §27's proposal),
styled as the toolbar is (40-pixel rows, 32-pixel icon buttons, its gaps,
dividers, and fixed-width menu roots), its two rows on one grid so the
fields start and end together.


## 2026-10-20 — Fields: the user's answers (`UI_SPEC.md` §28, FL1–FL7)

Asked before the fields run (the Phase 5 queue's run 4), with the evidence
in `docs/research/FIELDS_NOTES.md` (the decompiled `AMIPRO.EXE`,
`AMIPROUI.DLL`, and `AMIMACRO.DLL`; the user's guide ch. 5, 23, 32; VM
FD1–FD6). The user's answers:

- **FL1 Power fields worked out: the document-only set, and also Include
  and the prompts** (not the recommendation, which left those two out).
  The app works out the counts and Doc Info fields, Now, Seq, Set and
  names, IF, Defined, Void, User, arithmetic and comparisons, `{…}` in
  text, the `%D` `%T` `%N` formats, Ami Pro's text functions (Left$,
  Right$, Mid$, Len, LCase$, UCase$, StrCat$, StrField$, FormatNum$),
  **Include** (another Ami Pro or text file's main text, or a bookmark's
  in it, when the field updates), and **Query$ and Decide** (a prompt when
  the field updates). Merge fields show `<NAME>` and MergeRec 0 outside a
  merge, as Ami Pro (the merge run); Bookmark, PageRef, and GetBookMark
  keep their result until bookmarks (the notes run); Index and TOC stay
  marks (the TOC run). Never run, the last result kept: Message, Exec,
  Call, Button's action, PrintEscape.
- **FL2 Inserting and changing fields: a Fields panel in the rail** (not
  the recommended dialog): it inserts fields and edits the field at the
  caret live, like Style Properties.
- **FL3 Doc Info: Document Properties in the rail** (as recommended),
  live, statistics counted as you type and written at save.
- **FL4 Updating: Ami Pro's rule** (as recommended): dates, times, and Doc
  Info fields as their lines are laid out; power fields when inserted, by
  Update and Update All, and Auto run ones when the document opens; Update
  All before printing and PDF later (an option, on).
- **FL5 Numeric dates and times: Ami Pro on US Windows** (as recommended):
  03/02/05, 12-hour time with AM and PM, English names.
- **FL6 Field Shading on** (as recommended): light grey on screen only;
  View › Show Power Fields as Ami Pro.
- **FL7 Ami Pro's typing rule without its delete question** (as
  recommended): a result is ordinary text until the next update; deleting
  a field asks nothing (one undo step).

Fixed whatever the answers (Ami Pro's behaviour, not choices): stored
times in Ami Pro's time base (the wall clock read as US Pacific time);
editing minutes added at each save; `[desc]` values 2–5 as Doc Info
Fields 1–4; Ami Pro's "126" year and its Seq-and-Lock fault not copied.

## 2026-10-20 — Fields: choices made in the run (FX1–FX13)

The run's decisions were the user's (FL1–FL7). These choices were made
while building it, each Ami Pro's behaviour where it has one
(`docs/FIELDS_PROGRESS.md`; `UI_SPEC.md` §28, implementation notes):

- **FX1** Save as a Style Sheet writes the revisions as the open document
  holds them (one more than its file said), as Ami Pro's writer does.
- **FX2** `[desc]` value 14 (the size with the style sheet and linked
  files, shown nowhere) is written as the file's own size.
- **FX3** The Doc Info entries Number of Words, Characters, and Size show
  the figures as of the last save; Number of Pages follows the layout.
- **FX4** A power field's marks stay kept marks, read on demand; only Seq
  is an object.
- **FX5** A mark left without its partner is written as it is (Ami Pro's
  reader drops it).
- **FX6** A name the document has as a bookmark keeps its field's result;
  a name nothing defines shows `<NAME>` with Ami Pro's warning.
- **FX7** An unknown function is a macro's and keeps its result.
- **FX8** Include's paragraphs go in as lines of one paragraph.
- **FX9** Update All walks the streams in Find's order.
- **FX10** An updated result keeps the formatting at its start.
- **FX11** While codes show, the pages show a copy and take no edits.
  Changed the same day after the run (below): the codes are typed over.
- **FX12** A Seq's code changes in This field, not its flags.
- **FX13** A field's prompts are asked when it is inserted.

## 2026-10-20 — Fields: codes typed over, and Edit › Power Fields ▸

After the fields run the user agreed that the page should take edits
while Show Power Fields shows the codes, as Ami Pro's does (replacing
FX11), and asked: "shouldn't there be menu items to show and update
powerfields?" (`UI_SPEC.md` §28, implementation notes, "Typing codes"):

- **Codes in the text.** Show Power Fields is an undo step that sets each
  field's result aside in the document (never saved) and puts its code in
  red in its place, as Ami Pro keeps each result in the field's record
  (`1638:0d0b`). Off reads each code back: unchanged, the result returns
  exactly; changed, the marks are rewritten and the old result stays
  until an update; emptied, the field goes. Update and insertions read
  the codes back first and show them again; Update All and saves leave
  them off, as Ami Pro's do.
- **Menus.** Edit › Power Fields ▸, as Ami Pro's: Update Fields (F9),
  Update All Fields (Ctrl+F9), Next Field, Previous Field, Edit Field…,
  Lock Field; Show Power Fields stays in View, where Ami Pro has it; Ami
  Pro's Power Fields ▸ Insert… is Insert › Field….

## 2026-10-20 — Print and PDF: the user's answers (`UI_SPEC.md` §29, PO1–PO6)

Asked before the print and PDF run (the Phase 5 queue's run 5), with the
evidence in `docs/research/PRINT_NOTES.md` (the decompiled `AMIPRO.EXE`
and `AMIPROUI.DLL`; the user's guide ch. 13; Ami Pro printing VM fixtures
to a PostScript file; the desktop's print portal, GTK's CUPS backend). The
user's answers:

- **PO1 Printing: the desktop's print dialog, Ami Pro's options in the
  rail** (as recommended). File › Print… (Ctrl+P) opens COSMIC's print
  dialog through the XDG print portal (`ashpd`); the app sends the chosen
  pages as a PDF made in the background. File › Print Current Page… opens
  it with the caret's page as the range. Ami Pro's Crop marks, Without
  pictures, With doc description, On preprinted form, and Update fields
  are in a Print panel in the rail (File › Print Options…), kept per
  document until it closes, for printing and PDF alike. Copies, collate,
  reverse, odd and even, and trays are the desktop dialog's.
- **PO2 PDF: `krilla`, vectors kept** (as recommended). Fonts embedded and
  subset, text selectable; bitmaps at their own pixels; metafiles and SVG
  as vectors.
- **PO3 With notes: with the notes run** (as recommended), when notes are
  in the model.
- **PO4 PDF additions: bookmarks, properties and page labels, PDF/A-2b**
  (as recommended; answered in text after the multiple choice did not
  register): bookmarks from outline levels; Doc Info as the PDF's
  properties and Ami Pro's page numbers as page labels; a PDF/A-2b option,
  off by default. Tagged PDF later.
- **PO5 Preview: live while the Print panel is open** (as recommended):
  the pages show as they will print, the document still editable.
- **PO6 Many files at once: not needed** (not the recommendation, which
  was File › Export Folder as PDF…). Export is one document at a time.

Fixed whatever the answers (Ami Pro's behaviour, not choices): print
options for the session only, nothing written to the file; From–To counts
positions in the document; crop marks put the page ½ in right and down
with 7/16-in marks ending 1/16 in from each corner; On preprinted form
leaves out frame and table lines, shading, fills, pictures, and protected
text; Without pictures leaves picture frames empty; the cover sheet's
labels and layout, with four-digit years (Ami Pro's "126" not copied).
Choices made while building are labelled PX1, PX2, … in
`docs/PRINT_PROGRESS.md` and here.

## 2026-10-20 — Print and PDF: choices made in the run (PX1–PX9)

The run's decisions were the user's (PO1–PO6). These choices were made
while building it, each Ami Pro's behaviour where it has one
(`docs/PRINT_PROGRESS.md`; `UI_SPEC.md` §29, implementation notes):

- **PX1** A picture the app cannot draw (a labelled box on screen) prints
  nothing: an empty frame, as Without pictures prints it.
- **PX2** The grey cell grid is the screen's guide, as Ami Pro's dotted
  one, and does not print; tables print without lines until Lines & Color
  (the table-editing run).
- **PX3** The PDF is drawn at a pixel a twip and scaled to points, so rules,
  rounding, and text are as fine as the layout's unit.
- **PX4** The cover sheet stands 1 in from the paper's corner; Ami Pro's
  from its printer's printable corner (a little further in).
- **PX5** Update fields first stops the print at a field's question
  (Query$, Decide); the status bar says to answer it and print again.
- **PX6** Crop marks are quarter-point hairlines (the decompile does not
  show Ami Pro's pen).
- **PX7** Print Current Page… opens the desktop's dialog with the caret's
  page as its range, as the portal's dialog cannot offer Current page.
- **PX8** The print job is named after the document's file (Ami Pro: "Ami
  Pro - FILE.SAM").
- **PX9** On paper and in the Print panel's preview a frame's line is its
  style's width (`line_style_width`), inside, across, or outside the edge
  as its position says (the close-to positions as inside and outside); on
  screen lines stay one or two pixels as before.

## 2026-10-20 — Export: the user's answers (`UI_SPEC.md` §30, EX1–EX10)

Asked before the export run (the Phase 5 queue's run 6), with the evidence
in `docs/research/EXPORT_NOTES.md` (the decompiled `rtf.flt`; Ami Pro 3.1
exporting VM fixtures through its RTF and Word for Windows 2.0 filters,
which drop frames with their text; the formats' specifications;
LibreOffice 24.2 on the host). While the questions were prepared the user
wrote: "if both formats are easy to implement, you can do both - or if you
see a standard export process like Ami Pro used with filters for each file
type we can do that", and asked which other formats to weigh (ODT and HTML
were added to the choices; Markdown, EPUB, and Lotus Word Pro were not).
After asking how Ami Pro handled frames (it drops them on export; its RTF
reader knows Word's positioned paragraphs), the user answered:

- **EX1 Formats: DOCX, ODT, RTF, and plain text** (as recommended), through
  one export pipeline with a filter per format, as Ami Pro's filters work:
  the document is resolved once into a neutral form and each filter writes
  it. The import run (queue 7) adds each filter's reader. HTML is not
  built.
- **EX2 Commands: File › Export ▸** (as recommended): PDF… (Ctrl+Shift+E),
  Word Document (.docx)…, OpenDocument Text (.odt)…, Rich Text (.rtf)…,
  Plain Text (.txt)…, and Folder…. Each writes a copy; the tab keeps its
  own file and name, as Ami Pro's Save As in another type does.
- **EX3 Frames: floating boxes where Ami Pro places them** (as
  recommended): text frames as floating text boxes (ODT frames), picture
  frames as floating pictures, with their placements, wraps, layering,
  lines, shadow, corners, fill, margins, and columns; RTF shapes carry a
  positioned-paragraph fallback.
- **EX4 The word processor: LibreOffice Writer** (as recommended); exports
  are checked against it here, and the physical check uses it.
- **EX5 Table lines and shading: mapped and drawn in this run** (as
  recommended): the cell fields mapped from the decompile and the VM, then
  drawn on screen, on paper, in PDFs, and in every export. The Lines &
  Color dialog stays in the table-editing run.
- **EX6 Both one document at a time and a whole folder** (the user: "Can we
  do both?"): the Export ▸ items for the document, and Export ▸ Folder… (a
  rail panel: From with subfolders, To, the format; originals only read;
  never over an existing file; an `Export report.txt`; in the background
  with Stop). PDF is not among the folder's formats (PO6).
- **EX7 Losses: listed before writing, with Cancel and Export Anyway** (as
  recommended), as Save As SAM; nothing shown when nothing is lost.
- **EX8 Lists and fields: live where the format has them** (as
  recommended), showing Ami Pro's current results; the rest as text.
- **EX9 Style names: Ami Pro's** (as recommended); Body Text the default
  paragraph style; outline levels carried.
- **EX10 Additions: all three** (as recommended): Doc Info as the file's
  properties; an Open button after an export (the desktop's program for
  the type); the last format and folder remembered.

AGENTS.md's "no premature DOCX implementation" is met: the user scheduled
export in `PROJECT_PLAN.md` Phase 5 and the queue. Choices made while
building are labelled EC1, EC2, … in `docs/EXPORT_PROGRESS.md` and here.

## 2026-10-20 — Export: choices made in the run (EC1–EC15)

The run's decisions were the user's (EX1–EX10). These choices were made
while building it, each Ami Pro's behaviour where it has one
(`docs/EXPORT_PROGRESS.md`, "Choices made in the run"; `UI_SPEC.md` §30,
implementation notes):

- **EC1** Plain text: UTF-8 with line feeds; headers and footers left out,
  as Ami Pro's ASCII export leaves them; frames' text and tables after the
  paragraph anchoring them, cells separated by tabs.
- **EC2** Document times written as UTC from the wall clock Ami Pro shows.
- **EC3** Pictures: metafiles as WMF, PNG and JPEG as they are, SVG with a
  PNG, the rest as PNG at 300 dpi; drawings, charts, and OLE objects as
  Ami Pro's screen image; linked pictures embedded.
- **EC4** Font names no system resolves now (Tms Rmn, Helv,
  TimesNewRomanPS, Arial MT, Courier, Book Antiqua, Helvetica-Narrow)
  written by today's name for the same face, so Word and LibreOffice find
  the face or its metric clone.
- **EC5** Single, 1½, and double spacing at least Ami Pro's printer line
  height for the style's font, where the document has it.
- **EC6** Lists: a new instance wherever the target's counting would show
  another number than Ami Pro's; numbers no list can show as text.
- **EC7** Frames placed as the app's layout places them (Where placed on
  their page; repeating in the headers; With para above at their offset);
  With para above tables and lone Flow with text tables in the text; a
  picture frame a picture when the picture fills it, else a box holding
  it.
- **EC8** Table lines black, centred on the cell box (gutter middle to
  gutter middle), the fill over the box; drawn on screen and paper, not on
  a preprinted form.
- **EC9** Export ▸'s items keep "Export" in their labels.
- **EC10** The save dialog starts in the last export's folder.
- **EC11** RTF frames as shapes with a positioned-paragraph result, text
  frames as text boxes (square corners), runs as `{\plain …}` with only
  what is on (Ami Pro's reader).
- **EC12** Index and contents entries as the formats' entry fields.
- **EC13** The loss list counts only what shows (no hyphenation only where
  the style hyphenates; `<:s>` no loss); plain text lists frames,
  pictures, and tables, not formatting.
- **EC14** Export Folder never writes over a file; `Export report.txt`.
- **EC15** Header and footer distances from the bands' interior margins.

## 2026-10-21 — Import: the user's answers (`UI_SPEC.md` §31, IM1–IM12)

Asked before the import run (the Phase 5 queue's run 7), with the evidence
in `docs/research/IMPORT_NOTES.md` (Ami Pro 3.1 opening and inserting
fixtures written for the run, IM1.RTF, IM2.TXT, IM3.TXT; a note, NT1; the
user's guide ch. 22; the decompiled `AMIPRO.EXE`, `AMIPROUI.DLL`,
`rtf.flt`, and `AMIXIMP.DLL`; the formats' readers evaluated; free sample
documents). The user answered:

- **IM1 Commands: File › Open, Insert › File…, and a dropped document**
  (as recommended): Open reads Word Documents (.docx), OpenDocument Text
  (.odt and flat .fodt), Rich Text (.rtf), and Plain Text (.txt) into a new
  tab whose source is protected (Save goes to Save As); Insert › File…
  (Ami Pro's Open › Insert) puts a file at the caret, in the main text, a
  frame's text, or a table cell; a document file dropped on the window
  opens (pictures still insert).
- **IM2 Styles: Ami Pro's Import Options asked each time, the last choice
  preselected** (as recommended): Convert styles, Apply styles, Keep style
  names, Ignore styles, with Ami Pro's meanings (VM IMC, IMA, IMK, IMI).
  Ami Pro itself preselects Convert every time.
- **IM3 Insert › File…: at the caret, the document's styles win** (as
  recommended): no page break; styles the document already has keep its
  definitions, only new styles are added; the document's page set-up is
  untouched; one undo step. Ami Pro starts the inserted text on a new page
  in the file's page set-up and lets Convert redefine the document's
  styles (VM IMX); that is not copied.
- **IM4 Names: Normal becomes Body Text** (as recommended): Word's Normal
  and LibreOffice's default paragraph style become Body Text; every other
  name is kept; names longer than Ami Pro's 13 characters are kept in the
  app and shortened (and listed) only on Save As SAM.
- **IM5 Lists: Ami Pro's list styles** (as recommended): paragraphs
  numbered or bulleted directly take the list styles the app's Bullets and
  Numbering commands use (Bullet 1, Number List, List 2…, Bullet 2…), each
  keeping its own look as formatting; a number Ami Pro's counting cannot
  show is kept as text.
- **IM6 Comments as Ami Pro notes; footnotes and endnotes at the end**
  (as recommended): comments become notes stored as Ami Pro stores them
  (NT1); footnotes and endnotes become numbered paragraphs at the
  document's end with their numbers raised in the text; all listed.
- **IM7 The document as it stands, the nearest Ami Pro form** (as
  recommended): tracked changes accepted; links as their text, character
  styles as formatting, hidden text left out, a nested table as its text
  in the cell, a floating table in a frame, charts and shapes as their
  stored picture; everything listed.
- **IM8 The Import Report: a rail panel with jumps** (as recommended),
  opened by itself only when something is listed, with Copy; the status
  bar names the count.
- **IM9 Plain text: Ami Pro's ASCII options with modern defaults** (as
  recommended): in the Import Options dialog, Each line is a paragraph
  preselected, Join lines and Keep style names offered, the characters
  detected (Unicode, Windows ANSI, PC-ASCII, 7-bit ASCII) and changeable.
- **IM10 Other formats through LibreOffice: no** (the user's choice; the
  recommendation was yes): only the four formats the app reads itself.
- **IM11 Import Folder…: yes, writing `.amipro`** (the user's choice; the
  recommendation was `.SAM`): a rail panel as Export Folder (From with
  subfolders, To, the style option), each file written as an AmiPro-Linux
  document to the same relative place, never over a file, with an
  `Import report.txt`.
- **IM12 Paste: rich text from other programs** (as recommended): Paste
  takes the clipboard's RTF through the same reader, as Ami Pro pastes RTF
  with Apply styles (its look kept as formatting in the caret's style,
  tables and pictures with it); Paste as Plain Text stays.

Choices made while building are labelled IC1, IC2, … in
`docs/IMPORT_PROGRESS.md` and here.

## 2026-10-24 — Import: choices made in the run (IC1–IC33)

The run's decisions were the user's (IM1–IM12). These choices were made
while building it, each Ami Pro's behaviour where it has one, the format's
convention where it has none (`docs/IMPORT_PROGRESS.md`, "Choices made in
the run"; `UI_SPEC.md` §31, implementation notes):

- **IC1** The readers' ground is the project's own, in the core: a zip
  reader through the central directory (inflate from the caller, as export
  passes deflate; the app's `flate2`, `miniz_oxide` for the core's tests
  and examples), an XML tree reader (namespaces, no entity expanded, nesting
  limited to 512), import options and findings. The formats' crates resolve
  nothing Ami Pro needs (`IMPORT_NOTES.md`).
- **IC2** `realise` is the export's builder run backwards, measured as a
  fixed point: a document built, realised, and built again gives the same
  exchange document.
- **IC3** Opening with Convert: the file's styles in the file's order; the
  new document's own styles the file does not use are left out (Body Text
  stays).
- **IC4** The file's first (default) style becomes Body Text; a file style
  named Body Text beside it is "Body Text (file)"; every other name is kept,
  a Normal that is not the default too.
- **IC5** What Ami Pro holds only in styles (a paragraph's own space above
  and below, keep with next, keep together, rules) comes in through a
  variant style named after its style ("Body Text 2"), reused for the same
  values; Apply styles leaves it out (as Ami Pro's Apply) and lists it.
- **IC6** Line spacing "at least" a height is single spacing with that
  printer line height for the font (EC5 backwards); multiples other than 1,
  1½, and 2 are custom heights (1.15 × the size per line); exact heights are
  custom.
- **IC7** Tables are Ami Pro's page tables (a frame alone in its line); a
  table at an offset is With para above, below the paragraph before; a box
  holding only a table is a table frame; tables in frames, headers, and
  cells are their text, a row a paragraph (listed).
- **IC8** Frames: placed from the page or margin, Where placed on the page
  their paragraph is laid out on (two layouts: without them, then with);
  placed from a paragraph with text beside them, Where placed at their
  paragraph's place on its page (Ami Pro's With para above frames have no
  text beside them); placed from a paragraph without, With para above; in
  the line, Flow with text; in headers, repeating. Shapes in frames' and
  headers' text are their text, listed; a shape at a cell's start is the
  cell's frame.
- **IC9** Comments are Ami Pro notes as VM NT1 shows them: yellow, the
  author's initials (from the name where the file has none), the date or
  now.
- **IC10** Footnotes are numbered 1, 2, 3 and endnotes i, ii, iii in the
  text (raised) and at the document's end, each number and a tab before its
  text.
- **IC11** A page number the file shows empty on the first page starts on
  page 2 (Ami Pro's numbering from a later page).
- **IC12** Header and footer text whose indents reach past the page's
  margins moves the band's own margins (Ami Pro's bands have margins of
  their own); its tabs move with them.
- **IC13** Dates are Ami Pro's date objects (Today, Created, Last revised),
  times its Time object or Doc Info times, other document fields Doc Info
  entries, Keywords a power field, index and contents entries Ami Pro's
  marks; fields with no counterpart keep their result (listed).
- **IC14** Plain text: a form feed is a page break; Ami Pro's leading space
  after a second blank line (CR/LF at lines) is not copied.
- **IC15** RTF: a style entry without `\s` is style 0 (Word writes Normal
  so); a paragraph without tab stops of its own has the default stops every
  `\deftab`; positioned paragraphs become one frame per run of the same
  frame; `\shprslt` and `\nonshppict` are skipped where the shape is read;
  text in another code page than Windows ANSI comes from its `\u` (Word and
  LibreOffice write both); a table without cell edges has inch-wide
  columns.
- **IC16** DOCX: the default paragraph style is style 0 (IC4), the others
  in the file's order; character styles toggle bold and italic as Word
  does; a text box holding only a picture is a picture frame; a rounded
  rectangle's adjustment is Ami Pro's corner rounding, an outer shadow its
  shadow; a picture's kind comes from its bytes, then its name; the empty
  paragraph Word keeps after a last table is left out.
- **IC17** Lists: each run of list paragraphs gets Ami Pro's list style
  for its kind and depth (Bullet 1, Number List, List 2…), each style made
  once through the editing commands (`SetList`, `ChangeLevel`).
- **IC18** A frame's opacity: a frame text runs through is opaque unless it
  lies behind the text; any other frame is opaque where it has a fill.
- **IC19** A picture lying inside its frame keeps its place as the frame's
  margins; a picture reaching past its frame is cut by it (Ami Pro's
  graphics scaling).
- **IC20** ODT: the body style is the file's Body Text (LibreOffice's
  Text body) where its paragraphs use it, else the default style Standard
  (IM4); a paragraph naming no style is in it.
- **IC21** ODT page styles: each master page change starts a section on a
  new page (ODF changes headers and footers only at a page break); ODF's
  page margins reach the header and footer, so the text's margin is the
  margin, the band, and its spacing, and the band's distance the margin.
- **IC22** ODT breaks: a break after a paragraph is Ami Pro's break at its
  end; a break before is the break at the end of the one before (before
  the first paragraph a column break stays at its start).
- **IC23** ODT list levels: text with no number is Ami Pro's bullet text
  ("Note:"); a style's list is at its outline level; `continue-list` and
  `xml:id` keep a list's count across the paragraphs between.
- **IC24** The ODT writer improved on the way (each checked through
  LibreOffice): a break that ends or begins a paragraph is the paragraph's
  break after or before, not an empty paragraph; a table starting a
  section carries its master page (LibreOffice honours it; the empty
  paragraph before it is gone); a paragraph mark's look is written only
  for a paragraph without text (a paragraph style's text properties are
  all its text's, which set a paragraph's text in its mark's size); dates
  keep their pictures as data styles; a style's hyphenation is
  `fo:hyphenate`; list labels keep right alignment and their look; a
  bullet of more than one character is text; text boxes keep their
  rounded corners (`draw:corner-radius`).
- **IC25** Enhanced metafiles play into the Windows metafile's display list:
  logical units through the world transform and the mapping mode onto the
  reference device, the header's frame onto the unit square (its natural
  size the frame's, in hundredths of a millimetre); EMF+ comments skipped
  (files carry their EMF records too); a path's clip as its bounds; EMF is
  Insert › Image…'s too, a metafile in Frame Properties (not turned), a
  bitmap at 300 dpi for Save As SAM, and EMF in DOCX, ODT, and RTF
  (`\emfblip`).

- **IC26** Import Options live in the app, not the tab: Open has no tab
  until the file is read; Insert and Paste name their tab and caret.
- **IC27** Insert › File… goes in at the selection's start and leaves the
  selection (Ami Pro's Insert leaves the insertion point where it was);
  Paste replaces it.
- **IC28** Paste takes RTF without asking (Ami Pro's Paste always uses
  Apply styles), and no Import Report position jumps for inserted text
  (the realised document's places are not the inserted ones); its findings
  are still listed.
- **IC29** The Import Report's Go to walks a kind's places in turn and
  wraps; a place whose paragraph was deleted is skipped.
- **IC30** Import Folder… (File menu, after Open) takes the files named as
  a format read and then recognised by their contents (a `.docx` holding
  text is read as text); text files as each line a paragraph (IM9's
  default); its style option is its own, starting from Import Options'
  last choice.
- **IC31** Save As SAM (and Save as a Style Sheet) shortens style names
  longer than Ami Pro's 13 characters (`1000:80cd`), each still unique
  ("Body Text (fi", "Body Text ( 2"), and lists them once ("2 style names
  longer than Ami Pro's 13 characters, shortened"): twin AIM1 showed Ami
  Pro finding no style for a longer name (IM4 built; Word's names such as
  "List Paragraph 2" are common).
- **IC32** Automatic rows take Ami Pro's measure: Ami Pro measures every
  Automatic row as it opens a file (VM RC1–RC3), so a file's minimum row
  height more than 0.1 in taller than the row's text becomes the text's
  height (twin AIMO); rows of empty cells keep their height, as Ami Pro
  keeps them; files the app wrote (their generator AmiPro-Linux, now also
  in RTF) keep their heights, which are Ami Pro's measure (the round trips
  stay equal).
- **IC33** A cell's picture gives its row at least the picture's height
  (the picture fills its cell, T2); such a row is not shrunk (twin AIMW:
  the pictures were written 0 high).

## 2026-10-24 — Table editing: the user's answers (`UI_SPEC.md` §32, TD1–TD11)

Asked before the table editing run (the Phase 5 queue's run 8), with the
evidence in `docs/research/TABLE_EDIT_NOTES.md` (the decompiled Table
commands and Ami Pro 3.1 fixtures TE1–TE20). Answers, 2026-10-24:

- **TD1 — Where the commands live:** a Table menu root after Format,
  always present, its items greyed outside a table (Ami Pro shows it only
  in a table); the same row, column, and cell items on the right-click
  menu in a table; the rail panel (TD2). As recommended.
- **TD2 — The rail instead of Ami Pro's dialogs:** Table Properties in the
  rail (Table, Rows and columns, Cells), live, each change one undo step,
  in place of Modify Table Layout, Column/Row Size, Insert Column/Row, and
  Lines & Color; Insert Row Above/Below and Column Left/Right as direct
  commands (as many as are selected; the rail takes a count). Only Insert
  Table keeps a small dialog. As recommended.
- **TD3 — Selecting cells:** Ami Pro's gestures (a drag across cells,
  Shift+click, Shift+Ctrl+arrows, the strips above a column and left of a
  row, Table › Select ▸), and Shift+arrows that cross a cell's edge turn
  into a cell selection. As recommended.
- **TD4 — A column with no room:** the table keeps its width and the
  columns narrow in proportion to make room (Ami Pro refuses); the status
  bar says so. As recommended.
- **TD5 — Tab in the last cell** adds a row at the table's defaults (Ami
  Pro leaves the table); Ctrl+Down or Esc leaves it. As recommended.
- **TD6 — Grid lines and the ruler:** dragging a grid line as Ami Pro (that
  column or row changes, the table's size follows), **without** the
  recommended Shift variant; the ruler shows the table's columns in a
  table and its markers drag the same way.
- **TD7 — Additions:** Distribute Rows/Columns Evenly, Convert Text to
  Table and back, **and table styles** (presets of Ami Pro's lines and
  shading applied in one step), more than recommended.
- **TD8 — Line colour:** **added**, kept in the native format only (Ami
  Pro's lines are black; Save As SAM writes black and lists the loss).
  The recommendation was black only.
- **TD9 — Clipboard and drag and drop:** all in this run (Delete empties
  cells; Copy, Cut, Paste of cell ranges as Ami Pro; drag and drop of rows,
  columns, and blocks; Alt+Up/Down move rows). As recommended.
- **TD10 — Questions:** none; every command is one undo step and acts at
  once. As recommended.
- **TD11 — One straight run.** As recommended.

## 2026-10-24 — Table editing: choices made in the run (TK1–TK11)

Taken during the table editing run by its stop rules (Ami Pro's behaviour
where it has one), recorded in `docs/TABLE_EDIT_PROGRESS.md`:

- **TK1** A cell's fill is F3 `0x200` with F10, black too; F6 (Ami Pro's
  palette index) is recomputed from F10 (`1490:0d0a`). VM TF0X (`0x200`
  with F10 0 is black; a corpus letter's heading row is black with white
  text, which the app had drawn unfilled) and TF0Y (F6 and F10 without
  the bit: nothing). 1,557 cells of the user's corpus are black fills.
- **TK2** The writer keeps the `[h]`/`[w]` records Ami Pro wrote as read:
  flag 1 (inside a joined block) only in records it writes itself (Ami
  Pro's reader sets it, `1458:041b`), flag 2 added only for a size the
  model makes its own. Own sizes are read from flag 2 alone; an Automatic
  table has none (Ami Pro flags every row it measured).
- **TK3** A table made in the text carries Ami Pro's `0x100000` as a kept
  frame flag (TE1's `0x1800C4`): With para above tables lack it (56 of 586
  anchored tables in the corpora), so it is not derived from placement.
- **TK4** The row cache's fingerprints are taken at the end of the import,
  after the derived ruler marks are dropped: rows whose cells held such
  marks never counted as read (one row of READMEBP's picture table).
- **TK5** Center table on page centres a page table in its column, by
  whole twips, whatever the paragraph's alignment (`1478:128a`).
- **TK6** A click in a table's cell puts the caret there, as Ami Pro does
  (TE1–TE19); before, a first click selected the table's frame. A table in
  the text is never selected by a click; a frame table is, on its edge
  outside the cells (and through Go To and Select Next Frame).
- **TK7** A cell selection is a page selection whose ends are in two cells
  of one table, a rectangle (`14a0:0000`). Typing with cells selected types
  in the focus's cell; Delete and Backspace empty the cells (one undo step,
  TD10). Home, Home and End, End are not built (the row's first and last
  cell are a Ctrl+arrow away).
- **TK8** Insert › Table… stays in the Insert menu only (the registry gives
  each command one menu); the Table menu starts with the row and column
  commands. The Insert Table dialog remembers the last size (Ami Pro offers
  5 × 5 each time). Table › Table Style ▸ and the rail's Style row apply
  Grid (style 1 everywhere), Outline (style 2 around), Header Row (row 0
  shaded E1E1E1, a heading, style 2 under it), Banded Rows (every second
  row F7F7F7, a grid), and Plain.
- **TK9** A grid line moves as far as the pointer does from where it was
  pressed (Ami Pro's outline follows the pointer); its hit zone is the
  gutter with two pixels' slack. On the ruler a table's columns show from
  their cells, and only a column's right edge is taken (left edges follow
  from the gutters); the lower half's menu offers Distribute Columns
  Evenly and Table Properties….
- **TK10** Copied cells are kept beside the clipboard's text (tabs between
  cells, a line per row, as Ami Pro's `1240:2636`); a paste of that text in
  a table pastes the cells with their look. Text with tabs or several
  lines pasted in a table fills cells from the caret's (Ami Pro asks before
  putting several lines in one cell; TD10: no question). A selection that
  does not match the copied block is refused in the status bar, as Ami
  Pro's "Shade range does not match copied data range." A dropped block
  that runs past the table is cut at its edge without asking (Ami Pro
  asks); a move empties the source first, then fills the target.
- **TK11** A table in the text stands on a line of its own however narrow
  (found by the twin ATEC, which the app drew beside "Above the table."):
  its anchor's word takes the column's width, so the text before it ends
  its line and the text after it starts the next. Measures: the round
  trip and the corpora's summaries unchanged; in the VM folder TE3's and
  TE16's narrow tables now stand where Ami Pro put them (284 → 8 twips
  down, 1,579 → 0 across), page ends in a table with the same rows 68 → 70
  of 71.

## 2026-10-25 — Drawing: the user's answers (`UI_SPEC.md` §33, DW1–DW9)

Asked before the drawing run (the Phase 5 queue's run 9), with the
evidence in `docs/research/DRAWING_NOTES.md` (the decompiled `AmiDraw.dll`,
`DrawMan.dll`, and `AMIPRO.EXE`; Ami Pro 3.1 fixtures DR1, DRBIG,
DRW0–DRW3). Answers, 2026-10-25:

- **DW1 — One straight run:** drawings drawn from their objects first
  (screen, PDF, export), then Draw mode. As recommended.
- **DW2 — In place, with the rail:** Draw mode edits inside the frame on
  the page at the document's zoom; Drawing Properties in the rail (Line,
  Fill, Position and size, Text, Grid), live, each change one undo step,
  in place of Line Style, Fill Pattern, Rotate, and Grid Settings. As
  recommended.
- **DW3 — The draw bar replaces the toolbar** while in Draw mode: Ami
  Pro's ten tools and thirteen commands, Undo and Redo, the font face,
  size, bold, italic, and underline for text objects, and Done; shown even
  with View › Toolbar off; the toolbar returns on leaving. The proposal had
  a row above the ruler; the user asked why not replace the toolbar (no
  page jump, the whole width), and chose that.
- **DW4 — A Draw menu always present** after Table, its items disabled
  outside Draw mode; the selection's items also on the right-click menu
  in Draw mode. As recommended.
- **DW5 — The full set of additions:** arrow-key nudge (Shift ×10), Ctrl+D
  duplicate, Tab through objects; exact position, size, and angle in the
  rail with Rotate Left/Right 90°; Align and Distribute; Bring Forward and
  Send Backward; smart guides; any colour (§23) and any width to 12¾ pt;
  true curves. Stored in SAM as Ami Pro's objects. As recommended.
- **DW6 — Files and clipboard:** Import Drawing… takes `.sdw` objects, an
  SVG's shapes as objects, or an image as a bitmap object; Save Drawing
  As… writes `.sdw`, SVG, or PNG (optionally the selection); Copy gives
  the app's objects, SVG, and PNG; Paste takes them, SVG, an image, or
  text (a text object per paragraph). As recommended.
- **DW7 — Charts drawn now, edited later:** charts are drawn from their
  objects and kept byte for byte; opening a chart is the charts run's. As
  recommended.
- **DW8 — The frame keeps its size** on leaving Draw mode (Ami Pro's rule,
  DR1); Fit Frame to Picture sizes it on request. As recommended.
- **DW9 — Leaving:** Esc, Done, or a click outside the frame (which then
  acts there). As recommended.

## 2026-10-25 — Drawing: choices made in the run (DK1–DK13)

Made during the drawing run (`docs/DRAWING_PROGRESS.md`), each Ami Pro's
behaviour where it has one, recorded here for the handoff:

- **DK1** A new rounded rectangle's corners: 450 twips (Ami Pro's 30 screen
  pixels at 100 %), at most a third of each side (VM DR2B).
- **DK2** Several objects selected: one set of handles on their joint
  rectangle, sizing them together (Ami Pro shows each object's handles).
- **DK3** A look chosen with objects selected applies to them and becomes
  the look of new objects (Ami Pro's Line Style and Fill Pattern OK).
- **DK4** Esc ends a gesture, point editing, or typing first, else leaves
  Draw mode (Ami Pro: Esc leaves).
- **DK5** Arrow keys nudge 72 twips (a grid step with Snap To), Shift ten
  times, Alt one screen pixel (Ami Pro has no keyboard drawing).
- **DK6** A text object's box is its text's extent in its font.
- **DK7** Paste of the app's own objects puts them 144 twips down and right;
  other pasted or imported things go to the drawing's origin (Ami Pro's).
- **DK8** Save Drawing As… saves the selection when there is one (Ami Pro's
  Selected objects only check box).
- **DK9** Typing in a text object is one undo step while keys follow within
  a second.
- **DK10** Undo is the document's history, every change one step (Ami Pro's
  Draw mode has one toggling level).
- **DK11** The clipboard type of the app's objects is
  `application/x-amipro-drawing` (a `.sdw` stream), with SVG and PNG.
- **DK12** Insert › Drawing…'s frame is the remembered Create Frame size,
  2 × 2 in before any frame was made (Ami Pro: 1 × 1 in), narrowed to the
  column.
- **DK13** Exports carry drawings and charts as SVG with a PNG fallback.

## 2026-10-26 — Notes, bookmarks, footnotes, endnotes: the user's answers (NR1–NR10)

Asked before the run (the Phase 5 queue's run 10; `docs/Design/UI_SPEC.md`
§34; evidence `docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`). All as
recommended:

- **NR1 — Notes in a rail Notes panel:** marks drawn as Ami Pro's
  coloured boxes; a click on a mark opens the panel at that note; every
  note listed (initials, number, date, page, colour), edited in place;
  Previous/Next, Remove, Remove All, all undoable; the user's initials and
  colour at the top.
- **NR2 — Plain-text notes**, as Ami Pro's note window; a stored note's
  formatting is kept until it is edited.
- **NR3 — Notes out everywhere:** Print Options › With notes as Ami Pro
  (marks and end pages grouped by page); the PDF the same plus a PDF text
  annotation per note; DOCX, ODT, and RTF exports carry notes as comments.
- **NR4 — A rail Bookmarks panel:** Edit › Bookmarks… opens it (name with
  Add or Go To, a sorted list with kind and page, Rename, Remove); Edit ›
  Go To gains Next/Previous Bookmark, Note, Footnote Mark, Footnote Text;
  each change one undo step.
- **NR5 — View › Bookmarks**, off by default and saved: grey brackets at
  text bookmarks, grey outlines on bookmarked cells and frames; never
  printed.
- **NR6 — Cross-references:** Insert › Cross-Reference… inserts a PageRef
  or Bookmark power field; PageRef, Bookmark, GetBookMark, and Include
  file!bookmark work out; the PDF gets named destinations and links,
  DOCX and ODT bookmarks and fields.
- **NR7 — Ami Pro's name rules:** no spaces, at most 19 characters, not
  starting with a digit, unique whatever the case; a refused name says why.
- **NR8 — Footnotes edited in place on the page**, laid out as Ami Pro
  lays them out; Insert › Footnote (Ctrl+Alt+F), Esc back to the
  reference, double-click to jump between reference and footnote.
- **NR9 — Endnotes are one switch for the document**, as Ami Pro (and
  SAM): Insert › Endnote leaves the menu; Make endnotes moves every
  footnote, one undo step.
- **NR10 — A rail Footnotes panel** for Ami Pro's Footnote Options (Tools
  › Footnote Options…), live, with Ami Pro's dialog rules, Number style…,
  and Previous/Next footnote.

## 2026-10-26 — Notes, bookmarks, footnotes, endnotes: choices made in the run (NK1–NK10)

Made during the notes run (`docs/NOTES_PROGRESS.md`), each Ami Pro's
behaviour where it has one, recorded for the handoff:

- **NK1** Bookmarks stay in the forms Ami Pro stores (zero-width marks
  `<:Z…>`, `[book]` lines, frame names); `amipro_document::bookmarks`
  gives the typed view and the commands change those forms. Notes and
  footnotes are typed objects owning a flow, as floating headers.
- **NK2** A document whose main text has a footnote is laid out whole on
  every edit (a 43-page one: 18 ms a key in release); others keep the
  incremental layout.
- **NK3** Notes outside the main text are numbered within their own text,
  0 in headers, footers, and footnotes; renumbered on every insertion and
  removal.
- **NK4** View › Note Initials (Ami Pro's `[LS1]` in the text) is not built:
  it changes line widths; the Notes panel shows each note's initials and
  number.
- **NK5** (withdrawn: Edit › Go To gained Next item as NR4 asked.)
- **NK6** A bookmark of several paragraphs shows them in one field
  result, apart by spaces.
- **NK7** krilla 0.8.2 has no text annotations: the PDF carries notes as
  Ami Pro's printed note pages (With notes), not as sticky notes.
- **NK8** With notes does not draw `[LS1]` in the printed text (it would
  move the lines); the note pages name each note's page.
- **NK9** A table footnote is numbered after the table's others, as
  inserted.
- **NK10** A hard return that opens a paragraph makes no empty line (the
  corpus caches: every such short paragraph one line high in Ami Pro; Ami
  Pro's display of a user document); found while measuring footnotes, it
  moved the user's corpus page counts from 315 to 318 of 394.

## 2026-10-26 — Proofing: the user's answers (SP1–SP8)

Asked before the run (the Phase 5 queue's run 11; `docs/Design/UI_SPEC.md`
§35; evidence `docs/research/PROOFING_NOTES.md`). All as recommended but
SP1:

- **SP1 — Dictionaries: the system's Hunspell dictionaries, plus en-US
  bundled** (the user's choice): spelling reads the Hunspell dictionaries
  installed on the system (`/usr/share/hunspell`, `~/.local/share/hunspell`)
  with `spellbook` (MPL-2.0, pure Rust), and the app carries a copy of the
  free en-US dictionary (SCOWL, permissive licence, its notice shipped with
  it) so English always works; a missing language names the package to
  install.
- **SP2 — Spelling as you type, on by default and saved:** red wavy
  underlines on unknown and repeated words, never printed; checked
  paragraph by paragraph after a pause, off the typing path; the
  right-click menu on one gives its suggestions, Ignore All, Add to
  Dictionary, Spell Check….
- **SP3 — One Proofing panel in the rail** with Spelling, Thesaurus, and
  Grammar pages, Ami Pro's buttons and options, the document live, the
  current word or sentence highlighted on the page; F7 Spell Check,
  Shift+F7 Thesaurus.
- **SP4 — Replacing improved:** each Replace one undo step; Replace All
  replaces every occurrence in the check's range at once, keeping each
  occurrence's capitals, one undo step with a count; Skip All for the
  session in that document, shared with spelling as you type.
- **SP5 — One user dictionary per user and language**, in the app's data
  folder, in Ami Pro's format, saved as words are added; listed in the
  panel (add, remove), Import… (Ami Pro's `LTSUSER1.DIC` or any word list),
  Export….
- **SP6 — One language per document, any installed:** `[lang]` with Ami
  Pro's numbers mapped to locales; chosen in the panel and Tools ›
  Language… (one undo step); the default for new documents in the
  settings; a language without an Ami Pro number kept in the native format
  and listed by Save As SAM; hyphenation follows the language where
  patterns exist.
- **SP7 — Thesaurus from the system's MyThes thesauri** (LibreOffice's),
  read by the project's own reader; the panel's Thesaurus page as Ami Pro's
  dialog (meanings with part of speech, synonyms in the word's capitals and
  inflection, Look Up, Back, Replace, staying open), and a right-click
  Synonyms ▸ submenu.
- **SP8 — Grammar with Harper** (`harper-core`, Apache-2.0, offline,
  English only) in the panel's Grammar page as Ami Pro's Grammar Checker;
  Ami Pro's eight rule sets and rule names mapped onto Harper's rules, kept
  in `[GramStyle]`; readability statistics by Ami Pro's formulas, also from
  Tools › Readability Statistics…; blue grammar underlines as you type
  offered, off by default.

## 2026-10-26 — Proofing: choices made in the run (SK1–SK5)

The run's decisions were the user's (SP1–SP8). These choices were made
within them, by Ami Pro's rules where it has one (`docs/PROOFING_PROGRESS.md`):

- **SK1** A language Ami Pro has no number for is written to `[lang]` as
  the first Ami Pro language of the same language (fr_BE → 3 French), else
  1 American, so Ami Pro still checks the document; Save As lists it.
- **SK2** Web addresses, e-mail addresses, and tokens without a letter are
  not checked (Ami Pro's engine flagged them).
- **SK3** The thesaurus's meanings show their part of speech and first
  synonyms: MyThes thesauri have no definitions, which Ami Pro's Meaning
  box showed.
- **SK4** Harper depends on `harper-brill`, whose part-of-speech tagger
  brings `burn` with its CPU (`ndarray`) backend; nothing for GPUs is
  built. Harper takes about 10 ms a paragraph, so grammar underlines are
  worked out on a worker thread; a grammar check walks paragraph by
  paragraph.
- **SK5** A grammar problem selects its own words on the page; Ami Pro
  selected the whole sentence. The sentence shows, rewritten, in the
  replacement options.

## 2026-11-03 — TOC, index, glossary: the user's answers (TX1–TX8)

These were asked before the run (the Phase 5 queue's run 12;
`docs/Design/UI_SPEC.md` §36; the evidence is in
`docs/research/TOC_INDEX_NOTES.md`). The user took the recommendation
every time.

- **TX1 One rail panel.** A Contents & Index panel in the rail replaces
  Ami Pro's TOC, Index, TOC Options, Mark Index Entry, and TOC Entry
  dialogs. It has three pages:
  - Contents: the levels by style, with page number, right align, and the
    leader or separator.
  - Index: alphabetical separators, ranges, and where the index goes.
  - Entries: Mark Index Entry's fields, live, and a list of every entry.
  Tools › TOC, Index… and Edit › Mark Text ▸ Index Entry… and TOC Entry…
  open it.
- **TX2 Where the TOC goes.** There are three places:
  - The start of this document (the default; Ami Pro's blank output file).
  - At the caret (an improvement; Ami Pro updates it where it stands).
  - A new document on `_TOC.STY`, opened in a tab.
  Update replaces a TOC in place.
- **TX3 Where the index goes.** It goes into a new untitled tab on
  `_INDEX.STY`, and the source is neither saved nor closed. Optionally it
  goes at the end of this document after a page break, in Primary,
  Secondary, and Separator.
- **TX4 Mark All.** It marks every occurrence of the entry's text, as
  whole words, with Exact case (on by default). It is one undo step and
  skips places that already hold the same entry.
- **TX5 Page ranges.** Ami Pro's form (every page) is the default. The
  option "Join pages into ranges" gives "3–5" and is kept in the settings.
- **TX6 TOC links.** A TOC row links to its heading: Ctrl+click on the
  page, and a link in the PDF. Nothing is stored in the SAM.
- **TX7 Glossary.** Ami Pro's commands work as Ami Pro's: Mark Text ▸
  Glossary…, Insert ▸ Glossary Record… and Ctrl+K, and Data File…. A
  Glossary panel in the rail adds:
  - The records with a preview; a double-click inserts one.
  - Add from selection, Rename, and Delete.
  - An option, Complete glossary names as you type (off by default; Tab
    takes the suggestion).
- **TX8 Writing glossary data files.** The app changes a glossary data
  file as Ami Pro does:
  - Records are appended, then the index footer.
  - The file is written to a temporary file and renamed.
  - Only a file that reads as a glossary data file is changed.
  Reading a glossary never changes the file.

## 2026-11-03 — Proofing: Harper kept; the physical check passed

- **SK4 confirmed.** Grammar keeps Harper (`harper-core` with its
  part-of-speech tagger and `burn`'s CPU backend), rather than dropping to
  readability statistics only (the user: "keep Harper").
- The proofing run's physical check passed (the user: "proofing check
  passes").

## 2026-11-03 — Master documents: the user's answers (MS1–MS11)

Asked before the master documents run (the Phase 5 queue's run 13),
with the evidence in `docs/research/MASTER_NOTES.md` and the proposal in
`docs/Design/UI_SPEC.md` §37. MS1–MS9 as recommended; MS10 and MS11 took
the addition.

- **MS1 A Master Document panel in the rail.** File › Master Document…
  opens it on any document: the files in order with their pages in the
  set, Add… (before the selected file, as Ami Pro's Include), Remove, ↑,
  ↓, drag, Open, Open All, the TOC and index output files, the separators
  option, and the Generate buttons. On a member it says where its numbers
  start, with Treat as Single Document (Ami Pro's Yes). Each change is one
  undo step.
- **MS2 Finding the files.** The stored path first; when it does not
  exist, the file of that name, without case, in the master's folder. New
  members are stored as full paths, as Ami Pro stores them. A file not
  found is shown missing, with Locate….
- **MS3 Numbers live in the app.** While a master is open the app works
  the set in memory: each member shows its page numbers, footnote numbers,
  list numbers, and Seq values in the set, and later members update after
  an edit. Member files are never overwritten; a member's `[sequence]` is
  written when it is saved. (Ami Pro recomputes them itself when it works
  the set.)
- **MS4 A member opened alone** shows the numbers stored in its
  `[sequence]`, as in Ami Pro. The app looks in its folder for a master
  that lists it and offers Open Master; opening it switches to the live
  numbers.
- **MS5 TOC and index over the set** go into new untitled tabs on
  `_TOC.STY` and `_INDEX.STY` (as TX2, TX3). When saved, their paths
  become the master's TOC and index files. Entries link to their file and
  place: Ctrl+click opens the file at the heading.
- **MS6 Print and PDF.** On a master the Print panel offers The whole
  master document (Ami Pro's order: TOC file, the files, index file, all
  pages) or This document only; the whole set is the default. Export PDF
  makes one PDF with a bookmark per file.
- **MS7 No book view in this run.** The panel, a tab per file with live
  numbers, and the set's PDF cover it.
- **MS8 Members** are SAM and `.amipro` documents. Save As SAM of the
  master lists `.amipro` members as files Ami Pro cannot open.
- **MS9 Checks.** The panel lists each file that restarts page numbers,
  has a page-number start or leading text, or sets a footnote starting
  number (the guide's warning), with Go to. Nothing is changed for the
  user.
- **MS10 Export of the set (the addition).** File › Export ▸ on a master
  offers the whole set as one DOCX, ODT, or RTF document, each file a
  section with its numbers in the set.
- **MS11 Find across the set (the addition).** The find bar gains In All
  Files of the Master: matches listed by file, a match opening its file
  there; Replace All is one undo step in each file it changes. Spell Check
  stays per file.
- **Choices made in the run** are MQ1, MQ2, … (the progress file and
  here). Each keeps Ami Pro's behaviour where Ami Pro has one.

## 2026-11-03 — Master documents: choices made in the run (MQ1–MQ6)

- **MQ1 Treat as Single Document holds for the tab.** A tab where it was
  chosen stays out of an open master's set until it closes; otherwise the
  master would number it again at once. Ami Pro's Yes holds until the set
  is worked again.
- **MQ2 No dragging of rows.** The panel moves files with ↑ and ↓, and
  Add… inserts before the selected file as Ami Pro's Include does; the
  pinned libcosmic has no list drag. Left for the handoff.
- **MQ3 Seq fields** continue from the carried values when fields are
  updated (F9, Update All, Auto run), as Ami Pro's run updates each file's
  fields first; the set's live numbers do not rewrite Seq results by
  themselves.
- **MQ4 One style per name in the set's export.** Files that define a
  style of the same name alike share it; where a later file defines it
  otherwise, its paragraphs take the first file's definition, as Word and
  LibreOffice do when a document is inserted into another, and the loss
  list says so ("styles defined otherwise in a later file"). Members of a
  master normally share a style sheet.
- **MQ5 What the set's export holds.** The same parts as printing the set
  (MS6): the TOC file when named, the files in order, the index file, each
  a section on a new page whose page numbers start where the file's start
  in the set. The Print panel's This document only exports the master
  alone, as it prints it. Plain Text takes the set too. A member exported
  alone writes the numbers it shows (its `[sequence]`'s page base and
  footnotes).
- **MQ6 Find across the set.** Options ▾ › In All Files of the Master,
  with Files of the Master listing each file and its matches (the bar
  keeps its rows). Started in the master, the master's own text comes
  first, then its files. Find Next past a file's last match opens the next
  file with matches in its tab; Replace All opens each file it changes and
  makes one undo step there, leaving the files unsaved (MS3).

## 2026-11-03 — Merge, labels, envelopes: the user's answers (MG1–MG11)

Asked before the merge run (the Phase 5 queue's run 14), with the
evidence in `docs/research/MERGE_NOTES.md`. The proposal goes in
`docs/Design/UI_SPEC.md` §38 at the run's setup. MG1, MG3–MG10 as
recommended; MG2 took spreadsheets too; MG11 took File › New's sheets into
this run.

- **MG1 A Merge panel in the rail**, non-modal, with the document live:
  the data file (Open, Edit records…), its fields (a click inserts one at
  the caret as Ami Pro's `MergeField NAME` power field), a record browser
  (◀ 3 of 120 ▶) for the preview, Conditions, Labels, and Merge to: New
  Document, PDF, Printer…, View & Print. Ami Pro's three steps become its
  sections; File › Merge… and Edit › Insert › Merge Field… open it.
- **MG2 Data sources:** all of Ami Pro's kinds (its delimited data
  document, a document holding only a table, DIF, fixed-width and comma
  files through description files), CSV/TSV, dBASE `.DBF` read natively
  (the user's own merges use it; the second `[recfile]` string `*dBase`),
  **and XLSX and ODS spreadsheets** (the first sheet, the first row as
  field names).
- **MG3 A record editor:** Edit records… opens the data file as a grid,
  one row a record (sort by a column, find, add and delete rows, add,
  rename, and reorder fields), saved in Ami Pro's delimited format so
  Ami Pro reads it; New Data File… asks only for the field names;
  delimiters `~` and `|` by default, changeable. External files are read
  only, as in Ami Pro.
- **MG4 Output:** Ami Pro's Merge & print, Merge, view & print (Print and
  View Next, Skip and View Next, Print All, Cancel), and Merge & save as
  (one file, a page break between records), plus Merge to New Document
  (every record in a new tab, frames and headers included) and Merge to
  PDF (one PDF, or one per record named from a field).
- **MG5 Preview:** fields show as `<NAME>`, shaded as fields; Preview in
  the panel shows the chosen record's values in place, laid out, stepping
  with ◀ ▶. Display only: the file keeps the fields.
- **MG6 Conditions:** Ami Pro's operators and rules exactly (`=` `<` `>`
  `!=` `<=` `>=`; And binding tighter than Or; numbers, then dates, then
  text), any number of rows, a live count ("42 of 120 records"), the
  preview stepping through matching records only. Kept for the session
  and in the native file; SAM cannot hold them.
- **MG7 Labels:** a sheet list of the app's own from Avery's published
  sizes (every part Ami Pro listed), plus Custom; set up as Ami Pro does
  (the page one label, across, down, and the first label's indent in
  `[recfile]` with flag 0x10), the preview showing a whole sheet, Print
  each label N times, and a start position on a partly used sheet.
- **MG8 Envelopes, native, in this run:** an Envelope panel in the rail
  with Ami Pro's sizes plus DL, C5, and C6, return addresses saved in the
  settings, the recipient from the selection or the first address, to
  Print, PDF, or a new document on Ami Pro's envelope sheet, and Merge for
  envelopes from the data file; feed position and offsets.
- **MG9 Empty fields:** Ami Pro's rule (a paragraph holding only empty
  fields takes no space), plus a doubled space collapsed and a leading
  comma dropped where an empty field left them, in merged output only.
- **MG10 Merge power fields:** MergeField, MergeRec, NextRec, Skip, and
  Defined, during a merge and in the preview, by Ami Pro's rules.
- **MG11 File › New's merge sheets in this run:** `_LABEL.STY`,
  `_MERGLET.STY`, and `_ENVELOP.STY` (and `_AMIENV.STY`) on File › New
  open the Merge panel's Labels section, a merge letter with the panel,
  and the Envelope panel in place of their macros, ahead of the templates
  run (queue 15), which keeps the rest of File › New.

## 2026-11-03 — Merge run: choices made in the run (MC1–MC4)

- **MC1** In the field language, a text ordered against a number (`<`,
  `>`, `<=`, `>=`) holds for none: Ami Pro skipped no record by `if
  MergeField AMOUNT < 100 Skip endif` (VM MGM7, MGS4). The fields run's 45
  reproduced results are unchanged.
- **MC2** A paragraph holding only empty merge fields (or `NextRec`,
  `Skip`) is left out of merged output; Ami Pro writes it with `<:S+0>` and
  shows it with no height, which looks the same (MGM1, MGM6).
- **MC3** An edit or Undo while the Merge panel's preview shows a record
  turns the preview off and is not made: the record's values are display
  only (MG5).
- **MC4** Date conditions compare as dates for every typed form; Ami Pro's
  own parser gives unexplained results for `3/1/94`, `1/1/94`, and
  `03/01/94` (MGM4, MGMA, MGN6), an open point for the handoff.
- dBASE dates show as M/D/YYYY so conditions read them as dates, and memo
  fields are empty (no `.DBT` reading), recorded in `merge::data`.

## 2026-11-03 — Merge run: choices made in the run (MC5–MC13)

- **MC5** The label sheet list holds Avery's laser and inkjet sheets
  (letter and A4) by their published sizes; continuous forms are left
  out.
- **MC6** The record editor's data document follows the 2026-10-13 rule:
  an existing `.SAM` is never written over, so Save of an edited SAM data
  file goes to Save As; open merge documents follow the data tab's
  records live. Writing over a SAM data file is put to the user.
  (Answered 2026-11-04: written over with a backup the first time, below.)
- **MC7** View & Print sends the records chosen to print together at the
  end, through the desktop's print dialog once.
- **MC8** Labels merged to New Document are whole sheets (each label a
  Where placed frame); As labels dims View & Print and Ami Pro File….
- **MC9** Merge to Printer…'s page range counts across the merged output.
- **MC10** An envelope's page is its size; on larger paper from the print
  dialog it goes at the feed's place along the top edge.
- **MC11** Text typed right after a power field goes after it, as Ami
  Pro's caret after a field it puts in (the fields run's rule that it
  goes in was its own; Update then deleted it). Shown codes keep their
  own rule.
- **MC12** One PDF per record never writes over a file (" (2)" added).
- **MC13** Fields the data file lacks are named in the Merge panel and
  merge empty with Ami Pro's message 3098; nothing asks to continue.

## 2026-11-04 — Writing over SAM files, with a backup the first time (user decision)

The merge run's handoff asked MC6 (Save of an edited SAM data file).
The user answered for SAM files generally: "if we feel confident of
preserving SAM files now, we can write over them and backup the first
time". The revisit of 2026-10-13 is closed with that recommendation:

- **The evidence for confidence**, measured at the merge run's exit
  (`sam_round_trip`): the user's corpus reads back equal in 417 of 417
  documents and the loss check lists nothing for any; the ISO corpus in
  159 of 160, the one that differs being listed by the loss check (a
  malformed layout); Ami Pro opens written documents as their originals
  (Phase 4 and every run's twins since). The written file is the app's
  writing of the document, not the original's bytes (caches such as
  `[pg]` are dropped and sections come in Ami Pro's writer's order), which
  is what the backup covers.
- **Save writes over an Ami Pro document.** A document opened from a
  `.SAM` or saved as one is at that file (`DocumentLocation::Sam`); Save
  runs the loss check (B: a list, then Cancel or Save Anyway) and writes
  over the file (`file_workflow::replace_sam_document`). The record
  editor's SAM data files are the same (MC6).
- **The backup.** The first time a file is written over, its original is
  copied beside it as `NAME.SAM.bak` before anything is written (a failed
  copy writes nothing); an existing `.bak` is never replaced, so it stays
  the first original whatever the later saves. The status bar names it.
- **Save As onto an existing `.SAM`** asks first ("Replace existing
  file?", naming the backup), as for `.amipro`, then writes over it the
  same way.
- **Safety kept.** The new file is written to a temporary file beside it
  with the original's permissions and renamed into place, so a failed
  write leaves the file as it was; a link is followed to its file.
  Imports from other formats (DOCX, ODT, RTF, text; their Save goes to
  Save As) and style sheets (D3, new files only) are unchanged.

## 2026-11-04 — The merge after first use: tables for data, three steps (user decisions MR1, MR2)

The user's first use of the merge run's interface: "overly complicated,
small and cramped and the entry area is counter intuitive - Adding to the
grid and then typing doesn't put the values in the grid. This should look
more like the table functions in Ami Pro - as far as entering into the
grid." Asked with a recommendation each, the user took both:

- **MR1** Data files are entered as Ami Pro tables (the table data file,
  kind 4, which Ami Pro merges from: VM MGT1), edited on the page with the
  table editing of `UI_SPEC.md` §32; the record editor's grid is removed.
  New Data File… writes a table file; Ami Pro's delimited data files are
  shown as the table and saved back in their own form. Table › Sort
  Ascending and Sort Descending are added for records.
- **MR2** The Merge panel becomes Ami Pro's three steps, one open at a
  time, wider, with full-width buttons; conditions, labels, and one PDF
  per record fold under step 3's Options.

Details in `UI_SPEC.md` §38, "Rework after first use".

## 2026-11-04 — Typing in the merge preview, and fields as units (after the second use)

- **Context:** the user's address block came out as "Hickory NC
  28602DayCharlie": with "Preview a record" checked, clicks put the caret
  in the record's values, and when the preview went off the caret kept
  that place in the letter, inside its fields. Two empty fields were left
  after Zip, which show nothing on the page.
- **Decision (agent's, within the user's "edit in place"):** the caret
  maps back to the letter when the preview goes off; typing while the
  preview shows turns it off and is made at that place (this replaces
  MC3's refusal, which swallowed the keystroke); a merge field goes in
  outside any other field; Backspace after a power field and Delete
  before one remove it whole, empty fields first, and a selection
  holding fields removes their marks too. Ami Pro's own files may still
  hold empty fields (merge output, `MergeRec`), so they are read and
  written as before.
- **Also:** the user asked for scenario tests of every part of the
  application, run by a lower-cost agent and reviewed before each
  physical check (`docs/SESSION_HANDOFF.md`, item 1).

## 2026-11-04 — Speeding up the rest (SU1–SU4, the user's answers)

- **Context:** the user's order of 2026-11-04, item 3: progress had
  slowed; `docs/SPEEDUP_PLAN.md` proposed how to finish faster, after the
  scenario suite found 14 defects the unit tests missed.
- **SU1, the plan adopted in full:**
  - Start-up reading cut to about 40 KB: `CLAUDE.md` becomes current state
    only, at most 15 KB, with its history in `docs/HISTORY.md`.
    `docs/SESSION_HANDOFF.md` keeps only the current handoff, with the
    rest in `docs/HANDOFF_ARCHIVE.md`.
  - The scenario suite is the gate before every physical check.
  - The nine remaining runs become five (A commands = templates, the
    remaining commands, and views; B outline mode; C table data; D
    objects; E revisions, but see SU3).
  - Two recipe weights: light for A and B, full for C and D.
  - Less paperwork at each exit.
  - The decisions are asked for two runs at once.
  - Optionally, disk space is freed for one worktree.
- **SU2, run D's scope is full editors** (not the recommendation):
  - charts, equations, and image processing get editors as Ami Pro had
    them;
  - OLE objects are drawn from their renderings and kept as before (live
    OLE editing stays not planned, since Windows' OLE has no
    counterpart).
  
  Expect D to take several sessions.
- **SU3, revision marking and Doc Compare are not planned** (not the
  recommendation): no corpus file has revision marking on. Files keep
  their `[revisions]` section and `<:V…>` marks untouched. Run E leaves
  the queue.
- **SU4, typing undoes as one step per run of typing:** typed text joins
  one undo step until the caret moves, a command runs, or there is a
  pause, as frame nudges already join. It is done first in the next
  session, with its scenarios.

## 2026-11-04 — Typing undoes as one step per run (SU4, as built)

- A run of keys of one kind (typing, Backspace, or Delete) on the page
  surface joins the last undo step (`History::amend`) while the caret
  stays where the run left it, the history has not changed since
  (`History::changes`, which grows at the step limit too), no formatting
  was chosen at the caret, and the pause between keys is under 2 s
  (`TYPING_PAUSE`). A caret move, a command (`Message::Command`), Undo or
  Redo, Enter, Paste, or another kind of key ends it.
- Only plain text edits make a run: a whole power field taken by
  Backspace or Delete, Enter ending a list, a picture's cell, and typing
  over fields stay steps of their own.
- Undo of a run puts the caret where the run began. Scenarios:
  `scenario_tests/documents.rs`, the two "run of typing" scenarios.

## 2026-11-04 — Runs A (`commands`) and B (`outline-mode`): the user's answers (CM1–CM12, OM1–OM4)

All sixteen answered as recommended, in one round for both runs (SU1).

- **CM1, File › New:** File › New… opens a dialog listing the style sheets
  from the style-sheet folders, by description or name, recently used
  first, With contents (on by default), and a first-page preview drawn by
  the app; also on the toolbar. Ctrl+N still makes a blank `_DEFAULT.STY`
  document at once.
- **CM2, Run macro:** a style sheet that runs a macro on File › New makes
  the document from its contents, the status bar names the macro that is
  not run, and the macro name is kept as read so Ami Pro still runs it.
- **CM3, Indention… and Spacing…:** Format › Paragraph… opens Paragraph
  Properties in the rail beside Style Properties: Ami Pro's Indention and
  Spacing as sections, plus alignment and breaks, live, each change one
  undo step, Revert to style per section; Text › Indention… and
  Spacing… open it at their section.
- **CM4, Fast Format:** Ami Pro's behaviour (Ctrl+T picks up the format;
  each selection then takes it until turned off; text format or paragraph
  style) plus a paintbrush toggle on the toolbar.
- **CM5, password-protected files:** map the `[encrypt]` scheme; open them
  after asking for the password, and let Save As set or remove a password,
  if the scheme can be written; otherwise open only and say so.
- **CM6, Preferences… (Ctrl+,):** one dialog with pages (General: default
  style sheet for New, recent files, undo levels, the `.bak` on saving
  over a SAM; Folders; View; Toolbar), app-wide, in the COSMIC
  configuration.
- **CM7, Line Numbering:** Ami Pro's options as a section of Page Layout
  Properties, the numbers drawn on screen and on paper as Ami Pro places
  them, written to `[lnopts]`.
- **CM8, paging:** continuous pages stay; View › Facing Pages (left and
  right pages side by side, paired as Ami Pro pairs them) and Ami Pro's
  zoom presets Full Page (Ctrl+D), Standard, Enlarged, and Custom (its
  percentage in Preferences), checked in the View menu.
- **CM9, Draft mode and Clean Screen:** Draft mode is one column of text
  at the window's width (no pages, frames as one-line markers, the styles'
  fonts kept; a new shortcut, as Ctrl+M is Normal); Show Clean Screen
  hides menus, toolbar, ruler, rail, and status bar, with a small Return
  button and Esc, merged with the registry's Full Screen.
- **CM10, View Preferences:** each mark a checked item in View › Show ▸
  (Tabs & Returns also a toolbar toggle), all of them on the Preferences
  dialog's View page, app-wide; the view (mode, zoom, marks) remembered
  per tab.
- **CM11, New Window:** Window › New View opens a second tab on the same
  document (its own caret, scroll, zoom, and mode; one history and dirty
  flag; titled NAME:2). Tile and Cascade are left out.
- **CM12, additions:** Insert › Special Character… (Ami Pro's ANSI table
  with recent characters), Help › Keyboard Shortcuts (from the registry),
  Help › Compatibility and Fidelity (what Save As SAM would list for the
  document), and the status bar's Face and Point Size buttons.
- **OM1, Outline mode:** Ami Pro's Outline mode on the page surface plus
  an Outline navigator in the rail (the headings as a tree, a click goes to
  the heading, a drag moves a section, keyboard moves in the tree).
- **OM2, collapsing:** in Outline mode and the navigator only, kept per
  tab, never written to files; Layout mode always shows everything.
- **OM3, Move Up and Move Down (Alt+Up, Alt+Down):** the caret's heading
  with its section (a body paragraph alone) in Outline and Layout mode
  alike, also in Bullets and Numbering ▸; in a table they still move rows;
  frames anchored in the section move with it; one undo step.
- **OM4, weight:** both runs light (spec, decisions, scenarios, one twin),
  but full (decompile mapping, VM fixtures, twins) for the password scheme
  (CM5) and line numbering (CM7), which write SAM structures the app has
  not written before.

### Run A: choices made in the run (CK…)

Each keeps Ami Pro's behaviour where it has one; evidence in
`docs/research/COMMANDS_NOTES.md`.

- **CK1, no documents folder (replaced by DF1):** Preferences › Folders holds the
  style-sheet folders only. The pinned libcosmic file chooser cannot be
  told where to start, so a documents folder would do nothing; the
  desktop's chooser remembers its own last folder.
- **CK2, recent files:** the File menu lists 0–9 (Ami Pro 0–5, `recfiles`),
  default 4 as Ami Pro's; files opened or saved go to the top.
- **CK3, undo levels:** 1–1000, default 100, the app's limit so far (Ami
  Pro offers Off to 4, default 1); a lower number drops the oldest steps of
  open documents at once.
- **CK4, With contents:** a sheet has contents when its body holds real
  text, it has page frames, or its header or footer holds text (codes
  alone or empty styled paragraphs are not contents); With contents is off
  and disabled for a sheet without.
- **CK5, Fast Format's icon:** the toolbar toggle is the eyedropper
  (`color-select-symbolic`); the COSMIC and Pop themes have no paintbrush.
- **CK6, what Fast Format applies:** the text attributes picked up (a plain
  pick-up changes no text, as Ami Pro applies its picked-up set); with the
  style, the paragraph's own indents, spacing, alignment, and ruler too.
  Each mouse selection while on is one undo step; Esc or the command ends.
- **CK7, Select a Style (Ctrl+Y):** opens the Styles panel with its filter
  focused; Enter applies the first style listed. The pinned libcosmic cannot
  open the toolbar's style list from a key; Define Style is Create Style
  (§14).
- **CK8, Facing Pages:** left pages even, right pages odd, so page 1 stands
  alone on the right; Ami Pro's first spread is unsettled in the decompile
  (low confidence).
- **CK9, Custom Zoom:** 25–400 % (Ami Pro 10–400), the app's smallest zoom;
  default 91 as Ami Pro's `viewlevel`.
- **CK10, Draft mode:** the main text laid out at the window's width (pages
  of that width, one column, 0.25 in. margins; nothing saved), Ctrl+Shift+D;
  frames, headers, and footers are still drawn where they stand. §39's
  one-line frame markers remain open.
- **CK11, New View and the Vertical Ruler:** not built in this session.
  New View needs the session split into its document (text, history) and
  its views (caret, scroll, zoom, mode), a slice of its own; Tabs &
  Returns' toolbar icon is the theme's pilcrow with an arrow.
- **CK12, line numbers beside frames:** each line on either side of a Wrap
  around frame is counted and numbered in the left margin at its height,
  over one another, as Ami Pro prints them (VM LN9).
- **CK13, passwords:** set or removed with File › Password… and kept until
  removed, so Save As keeps them (Ami Pro's Save As box, unticked by
  default, drops them; the portal's chooser cannot hold a check box); a
  wrong password at Open lets the user try again.

## 2026-11-05 — Documents folder and Markdown: the user's answers (DF1, MD1–MD3)

Asked when the user requested both (after run A's exit); built **after run
B** (the user's choice of timing).

- **DF1, documents folder:** Preferences › Folders gains a Documents
  folder, as the style-sheet folders (Ami Pro's User Setup › Paths). File ›
  Open, Save As, Insert › File, Import, and Export choosers start there,
  and an untitled document's first Save As suggests it; the chooser can
  still go anywhere. The pinned libcosmic wrapper cannot set a starting
  folder, so the app calls the desktop portal's current folder directly.
  Replaces CK1.
- **MD1, Markdown scope:** an exchange format like DOCX, ODT, and RTF:
  File › Open and Insert › File read `.md` (Import Options as for the
  others); Export ▸ and Save As write it, and Save As lists what Markdown
  cannot hold (frames, page layout, fonts, colours).
- **MD2, flavour:** CommonMark with GitHub's tables, strikethrough, task
  lists, and footnotes (pulldown-cmark, MIT); headings by outline level
  (`#`–`######`), lists, bold, italic, code, links, and pictures written
  beside the file.
- **MD3, timing:** after run B (outline mode).

## 2026-11-06 — Run C, table data: the user's answers (TN1–TN11)

Asked before the run (`docs/TABLE_DATA_RUNBOOK.md`, "Decisions to ask"):
TN1–TN4 put to the user, who took each recommended option; the user's
standing direction is to continue unless input is truly needed, so
TN5–TN11 take their recommended options.

- **TN1:** formulas are edited in the rail (Table Properties › Formula);
  Table › Edit Formula… and a double-click move focus there.
- **TN2:** numbers show through the cell style's number format, as Ami
  Pro; the file keeps Ami Pro's stored text.
- **TN3:** Tools › Sort… is built, as a rail Sort panel.
- **TN4:** Ami Pro's functions in SAM files, plus `@avg`, `@min`, `@max`,
  `@count` as native-only extras that Save As SAM lists.
- **TN5–TN11:** recalculation on leaving a cell or applying a command, none
  on open; number format in the style's Table format section; forms as a
  scenario only; errors in the cell and messages in the status bar; cell
  address headings in View Preferences, off by default; merge of numbers as
  Ami Pro once the display exists; one straight run.

## 2026-11-06 — Run D, objects: the user's answers (OD1–OD12)

Asked before the run (`docs/OBJECTS_RUNBOOK.md`, "Decisions to ask"). The
research corrected the plan's counts (charts in 34 corpus files and OLE
objects in 17; equations and image-processing images in none; the earlier
"4 and 4" were table-cell hints), and the user was asked again.
OD1–OD4 put to the user, who took each recommended option; OD5–OD12 take
their recommended options.

- **OD1:** full editors in value order (SU2 kept).
- **OD2:** an equation editor in Ami Pro's form once found, else typed TeX
  stored as a metafile picture.
- **OD3:** charts stored as Ami Pro does (data record plus generated
  objects).
- **OD4:** chart data from a rail table, clipboard text, and Chart from
  Table.
- **OD5–OD12:** all chart types; rail editing (equations in place); OLE
  drawn and kept with an Object section and read-only Link Options;
  non-destructive image adjustments; unreadable objects kept byte for byte;
  Paste Special lists formats; the extras set; one straight run.

