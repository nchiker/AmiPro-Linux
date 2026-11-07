# Phase 2.2 Brief: Page, Layout, and Frame Model

**Status:** prepared 2026-10-11 for the Phase 2.2 model decision; **the
decision was taken the same day** and is recorded in
`docs/research/DECISIONS.md` (2026-10-11, "Phase 2.2 page, layout, and frame
model") and `ARCHITECTURE.md`. The research checkpoint (section 5) is in
`docs/research/PAGE_FRAME_REFERENCE_RESEARCH.md`, and the VM checks
(section 6) are in `docs/research/SAM_FORMAT_NOTES.md`. Where the decision
differs from this brief, the decision governs. The main differences:

- D6 uses root snapshots with `Arc` sharing instead of inverse commands.
- D1 adds `GroupId`, `ObjectId`, and persisted `ParagraphId`.
- D3 adds breaks with flags, legacy marks, and special characters as
  characters.
- D4 adds one `frame_stack` for fixed and anchored frames, with layout frames
  as a lower layer.
- D5 adds the page-count rule from V11/X06.

The rest of this brief is kept as written for the record.

Sources: `docs/research/SAM_PAGE_FRAME_SPEC.md` (format rules),
`docs/research/SAM_FORMAT_NOTES.md` (evidence), the Ami Pro 3.1 user's guide
(chapters 8, 11, 14–16), and the current crates.

## 1. What the current model has

- `Document { title, styles, paragraphs: Vec<Paragraph>, page: PageSetup }`:
  one paragraph flow and one page size with four margins.
- `Paragraph { text, style, runs }`; character runs are byte ranges.
- `amipro_editing::Cursor { paragraph, byte }` and typed `Command`s with
  undo, which the app does not use. The app keeps 100 whole-`Document`
  snapshots per tab instead.
- `amipro_layout::paginate` only estimates page counts.
- Native `.amipro` v2 stores exactly this.

Nothing in it can name a text stream other than the main text, attach an
object to a character, or describe more than one page geometry.

## 2. Requirements from the evidence

**R1. Several text streams.** A document holds many independent text flows,
each a list of styled paragraphs (Ami Pro calls them text streams; chapter
11 lists their search order):

1. the main text;
2. text frames placed on pages (Where placed);
3. footnotes (later);
4. anchored text frames and table cells;
5. floating headers and footers, which live inside the main text;
6. fixed headers and footers (per layout, right and left) and repeating
   frames.

Frame text uses the main text's paragraph syntax, styles, and runs (F12).

**R2. Page layouts are named, ordered objects.** Each has paper size and
orientation; a right-page geometry and an optional left-page geometry
(margins; 1–8 columns, each with its own tab list; gutters; column balance;
line between columns; lines around the page); a header frame and a footer
frame for right pages and optionally for left pages; begin-on-second-page
flags; mirroring; and repeating frames. The first layout is the document's;
others are inserted.

**R3. Layout changes live in the main text.** An inserted layout takes
effect from the page after a page-break mark that names it (G21). Plain page
breaks are the same mark without a name (G20). Pages themselves are computed,
not stored (`[pg]` is a cache).

**R4. Fixed frames belong to a page number** (F13), with an absolute
rectangle on that page. They stay on that page when text reflows (V01).

**R5. Anchored frames belong to a character.** The `<:A n>` mark sits in the
main text and moves with editing. With para above places the frame below the
mark's paragraph; Flow with text places it inline like a large character
(X02). Their stored rectangles are caches. Tables in the text are anchored
table frames with their own `<:t n>` mark (TB1).

**R6. Stacking and groups.** Page-level frames have one back-to-front order;
groups are contiguous runs of it (Z01–Z04). Anchored frames are never
grouped.

**R7. Frames are layout boxes too.** Every frame has an interior layout of
the same shape as a page (margins, columns with tabs), plus border sides,
style, position and colour, shadow extents and colour, rounded corners,
background, opacity, and wrap mode (L01–L15, C01–C04, M01–M04).

**R8. Content kinds.** Text frame (a flow), picture frame (a payload: copied
bytes or a linked path, plus scaling data still undecoded), table frame (a
grid of cell flows), and later drawings, charts, and equations.

**R9. Preservation.** Unknown SAM fields and sections stay with the object
they came from, so a future SAM writer can write them back (`AGENTS.md`).

**R10. Derived data is recomputed.** Pagination, anchored-frame rectangles,
frame column edges, sequence numbers, and paragraph size caches are outputs
of layout, not model state.

## 3. Decisions to make

### D1. Object identity

The model needs names that survive edits, reordering, and undo, for frames,
flows, layouts, and (for the layout cache) paragraphs. SAM's own numbers are
positional (field 13 is renumbered when z-order changes; `<:A n>` counts
anchored records), so they cannot be identities.

- **Option A (recommended):** opaque per-document IDs (`FrameId`, `FlowId`,
  `LayoutId`, later `ParagraphId`), allocated from a counter and never
  reused within a document. SAM numbers are mapped to IDs at import and
  regenerated at export.
- Option B: positional indexes everywhere. Cheap now, but every z-order
  change, frame deletion, or undo would renumber references.

### D2. Document positions and selection

- **Recommended:** a text position names its flow:
  `TextPosition { flow: FlowId, paragraph, byte }`. A text selection stays
  inside one flow, as in Ami Pro, where the insertion point is always in one
  text stream. Object selection is separate: one or more page-level frames,
  or one anchored frame.
- The current `Cursor { paragraph, byte }` becomes the main-flow case.

### D3. How anchors sit in the text

The anchor must move with every edit, survive undo, and delete its frame when
the mark is deleted.

- **Option A (recommended):** an object-replacement character (U+FFFC) in
  the paragraph text for each inline object (frame anchor, table anchor,
  page break, floating header/footer), plus a per-paragraph list of inline
  objects in text order. Byte-range runs, UTF-8 cursors, and shaping keep
  working; the k-th U+FFFC is the k-th entry. Editing code needs one rule:
  deleting the character deletes its object.
- Option B: paragraph content as `Vec<Inline>` (text runs and objects). More
  explicit, but it rewrites every editing command, run, and cursor.
- Option C: anchors as separate `(paragraph, byte)` references. Every edit
  would have to shift them, which is the bug class D3 is meant to avoid.

### D4. Document shape

Recommended outline (names illustrative):

```
Document
  styles
  flows: FlowId -> Flow { paragraphs }
  main: FlowId
  layouts: Vec<LayoutId>  (first is the document's)  + LayoutId -> PageLayout
  frames: FrameId -> Frame
  page_frames: Vec<FrameId>   back-to-front order of non-anchored, non-repeating frames
  groups: GroupId -> Vec<FrameId> (contiguous in page_frames)
  objects: ObjectId -> EmbeddedObject { bytes, extension }   picture payloads
  legacy: preserved document-level SAM sections

PageLayout { name, paper, right: PageGeometry, left: Option<PageGeometry>,
             mirror, header/footer frames (right, left), begins_second_page,
             repeating: Vec<FrameId>, legacy }
PageGeometry = LayoutBox { size, margins, columns[{left,right,tabs}], balance,
             between_line, border }
Frame { placement: Fixed{page, rect} | AnchoredBelowParagraph | AnchoredInline
                 | Repeating{all|right_left} | Header | Footer,
        size, interior: LayoutBox, border, shadow, corners, background,
        opaque, wrap, content: Text(FlowId) | Picture(..) | Table(..),
        name, macro, legacy }
```

The same `LayoutBox` type serves pages and frame interiors (R7). Anchored
frames are reached from their inline object; fixed frames from
`page_frames`; header, footer, and repeating frames from their layout.

### D5. Pages

Pages are a layout result: the main flow is broken into pages and columns
using the layout in force (the document's layout until a page break names
another). Fixed frames attach by page number, so the layout engine places
them once the page exists. If text shrinks below a fixed frame's page, the
decision must say what happens (Ami Pro behaviour to verify, section 6).

### D6. Undo

- **Recommended:** one undo history per document, of typed reversible
  commands that name their target (`FlowId`, `FrameId`, `LayoutId`): text
  edits in any flow, frame insert/delete/move/resize/property changes,
  z-order and grouping, layout insert/modify. Replace the app's whole-document
  snapshot history with it, which also retires the current dual undo systems.
- Alternative: keep snapshots and make them cheap with persistent (shared)
  data structures. Simpler to keep correct, but memory and diffing costs grow
  with frames and pictures, and the command history in `amipro-editing`
  stays dead.

### D7. Tables, pictures, and later objects

- Tables: `Table { columns, rows, cells: grid of FlowId, legacy }` inside a
  table frame. Cell formatting fields are Phase 5.
- Pictures: payload by reference to `objects` (copied) or a linked path,
  plus preserved `[isd]` values. The `.X3` rendering cache is not stored.
- Drawings, charts, equations: further `Frame` content kinds later.

### D8. Native format v3 and legacy data

Native v3 stores flows, layouts, frames, objects, and preserved legacy
records; it keeps reading v1/v2 (one main flow, one layout from `PageSetup`).
Embedded payloads need a binary-safe encoding (a side file or base64 lines;
decide with D8).

## 4. Effects on existing code

- `amipro-document`: new types and validation; `PageSetup` becomes the
  document layout's right-page geometry.
- `amipro-editing`: positions gain a flow; commands gain frame and layout
  variants; U+FFFC handling in delete and replace.
- `amipro-formats`: import layouts, frames, anchors, breaks, tables (as
  preserved records with cell text), pictures (payload + directory), and
  floating headers; drop `<:#…>` caches.
- `amipro-layout`: becomes the real pagination and frame-placement engine
  (Phase 2.5).
- COSMIC app: the default editor and the rich editor stay one-flow editors of
  the main flow until the page surface (2.6) replaces them.

## 5. Research checkpoint for the decision run

`AGENTS.md` requires a short look at maintained implementations before the
decision. Questions to answer, each with a concrete yes/no for reuse:

- **OpenDocument / LibreOffice Writer:** anchor types `page`, `paragraph`,
  `char`, `as-char`, and `frame`; page styles (Ami Pro's page layouts) with
  headers and footers; how Writer stores positions (node + content index) and
  anchors (a placeholder character for `as-char`). Ami Pro's three
  placements map closely onto `page`, `paragraph`, and `as-char`; With para
  above differs in placing the frame *below* the paragraph.
- **COSMIC Text / Iced:** one `Buffer` per flow and per column region;
  whether line layout can take per-line exclusion widths for wrap around
  frames, or whether the layout engine must break lines itself.
- **Scribus** (frames-first DTP) for frame stacking and groups only.
- Existing Rust document or rich-text crates are unlikely to fit; confirm
  briefly rather than survey.

## 6. VM checks to run before or during the decision

1. Where placed frame on page 1, then the text pushed onto page 2: **done
   (V01): the frame stays on page 1.** Still open: what happens when the
   frame's page disappears (text shrinks to fewer pages).
2. With para above frame whose paragraph moves to the next page.
3. Flow with text frame taller than the space left on the page.
4. Repeating frame together with header text: z-order between them.
5. Deleting an anchor mark: **done (V05, V06).** Deleting a selection that
   contains the mark deletes the frame after a confirmation, and one Undo
   restores both. This supports D3-A (the anchor is a character) and D6 (one
   command spans text and frame).
6. Group: move and delete a group; ungroup.
7. `<:h…>`, `<*X>` meanings, and `<:t n>` numbering with two tables.

**Results (2026-10-11, fixtures V10–V23 and X06):**

1. A fixed frame keeps its page, and Ami Pro adds empty pages so that page
   exists, also when it opens a file (V11, X06).
2. and 3. The paragraph or line moves to the next page with its anchored
   frame (V12, V13).
4. Page frames always paint above repeating frames, and header text wraps
   around both (V14).
5. Done earlier (V05, V06).
6. A group moves, deletes, and undoes as one unit. Ungrouping clears the
   group bits (V15, V16).
7. `<:h` is the floating footer, and its byte is a flag set (V17, V18).
   `<*X>` is byte X − 0x20 (V19). `<:t n>` shares `<:A n>`'s numbering
   (V20).

## 7. Recommended direction (to confirm)

Stable IDs (D1-A); flow-qualified positions (D2); U+FFFC inline objects
(D3-A); the document shape in D4 with one `LayoutBox` type for pages and
frame interiors; pages computed from the main flow and page-break objects
(D5); one typed command history replacing snapshots (D6); tables and
pictures as frame content with preserved legacy records (D7); native v3 with
v1/v2 reading (D8).
