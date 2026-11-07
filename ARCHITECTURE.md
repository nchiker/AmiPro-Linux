# Ami Pro for Linux — Architecture

**Status:** Phase 0 stack decision accepted 2026-10-09. Phase 2.2 page,
layout, and frame model decided 2026-10-11 (`docs/research/DECISIONS.md`).
The model, its commands, its undo, and native v3 are implemented (Phase 2.3,
2026-10-11); SAM import of pages and frames, the layout engine, and the page
surface (Phase 2.4–2.6) are not yet. **Stack:** Rust stable + libcosmic (Iced), a
native Wayland-first application, a project-owned semantic model, and
reusable shaping primitives.

## Principles
1. The semantic document model is independent of the GUI, legacy file formats, and page rendering.
2. All edits are typed commands. Each one is applied as a transaction to an immutable document root, and undo restores the previous root. Objects have stable identifiers.
3. Layout is deterministic and produces a reusable display list for screen, PDF, and print.
4. Legacy formats are adapters, not the internal representation. Unknown content is preserved with the object it came from.
5. Reuse well-maintained, suitably licensed libraries for generic problems. Do not build a text shaper or spelling engine from scratch.

## Cargo workspace
```text
ami-pro-linux/
  app/cosmic/             # libcosmic shell and page/editor widget (separate manifest)
  crates/document/       # document model: flows, paragraphs, inline objects, layouts, frames, tables, pictures, styles, native format
  crates/editing/        # positions, selection, typed commands, history
  crates/layout/         # line breaking, pagination, frame placement, display list (shaper behind a trait)
  crates/rendering/      # future shared display-list adapters for PDF/print
  crates/ami-formats/    # defensive SAM/STY readers (writers later) and diagnostics
  crates/interchange/    # future ODT/DOCX adapters
  tests/                 # nonproprietary synthetic fixtures and integration tests
  docs/research/         # evidence and architecture decisions
```
Only `document`, `editing`, `layout`, and `ami-formats` exist so far. The COSMIC app has its own manifest, so the dependency-free core stays testable without UI dependencies. Add the remaining crates only together with a working vertical feature.

## Document model (Phase 2.2 decision)

Ami Pro is page- and frame-based. Its document has several text streams,
named page layouts, and frames that are mini-documents placed on pages. The
model represents all of them from the start:

```text
Document
  title, styles (shared by every flow)
  flows        FlowId -> Flow { paragraphs }        every text stream
  main_flow    FlowId
  layouts      [PageLayout]                          first = the document's layout
  frames       FrameId -> Frame
  frame_stack  [FrameId]                             top-level frames (fixed and anchored), back to front
  objects      ObjectId -> EmbeddedObject            shared picture payloads
  next_id, legacy (document-level SAM sections, in order)

Paragraph   { id: ParagraphId, style, text, runs (byte ranges),
              objects: [InlineObject]  (one per U+FFFC in text, in order),
              legacy marks (zero-width, byte offsets) }
InlineObject = FrameAnchor(FrameId) | Break { page | column, centre, layout: Option<LayoutId> }
             | FloatingHeaderFooter { header | footer, all | right | left, flow: FlowId }
             | later: Note, Footnote, Field (page number first)
PageLayout  { id, name, paper, landscape, right: LayoutBox + size, left: Option<…>, mirror,
              header/footer frames per side, begin-on-second-page flags,
              repeating: [FrameId] back to front, legacy }
LayoutBox   { margins, column count, gutter, per-column tabs, balance, line between }  (pages and frame interiors)
Frame       { id, placement, wrap, opaque, border, shadow, corners, background,
              interior: LayoutBox, group: Option<GroupId>, content, name, macro, legacy }
Placement   = Fixed { page, rect } | BelowParagraph { x_offset, size } | Inline { y_offset, size }
            | Repeating { all | right_left, rect } | Header { side } | Footer { side }
Content     = Text(FlowId) | Table { columns, rows, cells: [Text(FlowId) | …] } | Picture { Embedded(ObjectId) | Linked(path) } | Legacy(raw)
```

Rules the model enforces (validation):

- IDs (`FlowId`, `FrameId`, `LayoutId`, `GroupId`, `ObjectId`,
  `ParagraphId`) are unique, come from one per-document counter, and are
  never reused within a document.
- Every flow other than the main flow has exactly one owner: a text frame,
  a table cell, or a floating header/footer object.
- Every frame is reachable exactly once:
  - fixed and anchored frames through `frame_stack`;
  - repeating frames through one layout's list;
  - header and footer frames through one layout slot.
- Each anchored frame is referenced by exactly one `FrameAnchor`.
- Frame anchors, breaks, and floating headers occur only in the main flow,
  so frames never nest.
- The U+FFFC count in a paragraph equals its object count. U+FFFC has no
  other use.
- A group has at least two Fixed frames, all on one page and contiguous in
  `frame_stack`.
- Geometry is integer twips (SAM's unit). Font sizes are points.

Pages are not stored. They are layout output.

Unmapped SAM data stays with its object:

- typed fields where the position is known (frame field 8, unknown flag
  bits, `[frmlay]` values 3 and 6, …);
- raw sections otherwise;
- legacy marks for unmapped inline controls.

## Positions, selection, and editing

- `TextPosition { flow, paragraph, byte }` (UTF-8 boundary). A text
  selection stays in one flow.
- A frame selection is several Where placed frames on one page, or one
  other frame. Selecting a group member selects its group.
- For headers, footers, and repeating frames, which page the caret is shown
  on is view state.
- `amipro-editing` holds the typed commands: text, inline objects, frames,
  z-order, groups, and layouts.
- Commands implement Ami Pro's semantic rules:
  - deleting a range deletes the objects in it, the frames they anchor, and
    their flows;
  - a group moves and deletes as one action;
  - U+FFFC from input is rejected.
- Each command returns a new document root and a report of what it touched.

**Undo.** One history per document of document roots, each with its
selection. Roots share unchanged flows, paragraphs, frames, layouts, and
payloads through `Arc`, so an entry costs the touched objects plus pointer
copies. Undo and redo swap roots. This replaces the app's earlier
whole-document snapshots and the unused inverse-command history.

## Layout engine

`amipro-layout` stays dependency-free. It owns:

- line breaking;
- pagination: layouts selected by break objects, page count, columns, and
  balance;
- frame placement;
- wrap exclusions, which apply to main text and to header and footer text;
- headers and footers: right/left pages, mirroring, begin on the second
  page, and floating overrides;
- frame interiors;
- a display list whose fragments carry flow, paragraph, and byte ranges.
  Rendering, hit testing, and later PDF use that list.

Shaping is behind a trait:

- The app implements it with COSMIC Text, so layout and drawing share one
  font stack.
- Core tests use a deterministic fixed-advance shaper.
- The engine breaks lines itself from shaped words. That gives each line its
  own width (wrap around frames) and puts inline frames in lines as boxes.
  COSMIC Text's `Buffer` offers only one width per paragraph.
- Parley 0.12 offers this natively. It stays the measured fallback if the
  2.5 spike fails a concrete fixture
  (`docs/research/PAGE_FRAME_REFERENCE_RESEARCH.md`).

Incremental layout:

- Shaping is cached per paragraph, keyed by paragraph ID plus its inputs.
- Pagination restarts at the first page whose inputs changed and stops when
  a page starts at the same paragraph, line, layout, and floating state.
- Edits to headers and footers repaint pages without repaginating.

Ami Pro placement rules come from the VM evidence in
`docs/research/SAM_FORMAT_NOTES.md`:

- Fixed frames stay on their page number, and pages are added to hold them.
- Anchored frames are clamped to their column and move with their
  paragraph or line to the next page when they do not fit.
- Page frames paint above layout frames.

## UI and editing surfaces

libcosmic provides the application shell, menus, panels, and dialogs.

Until the page surface (Phase 2.6) replaces them, two editors edit only the
main flow:

- The default editor is an Iced `text_editor` with a model-driven
  highlighter (family, bold, italic). It synchronizes through
  `synchronize_editor_text`.
- The feature-gated rich editor is a custom widget over COSMIC Text with
  real per-span metrics, underline, and spacing.

Both must keep inline objects intact once they exist (2.3). The page surface
renders the display list. It hit-tests into any flow or frame and edits
through the same commands. The UI-4 workspace and zoom share one transform
for rendering and hit testing. See `docs/research/COSMIC_RICH_EDITING.md` and
`docs/Design/RICH_EDITOR_ARCHITECTURE.md` for the editor evidence.

## Native format
The native format is line-oriented, tab-separated text with escapes and a strict decoder that rejects unknown records.

- v3 (current, `docs/Design/NATIVE_FORMAT_V3.md`) stores IDs, flows, paragraphs with inline objects and legacy marks, runs, layouts, frames, the stack, groups, tables, pictures, legacy records, and base64 `blob` chunks for embedded bytes, all in one file. Saving stays an atomic temp-file-and-rename.
- v1 and v2 (one paragraph flow and `PageSetup`) keep loading into the main flow and the document layout; the app no longer writes them.

## Compatibility architecture
`SAM/STY bytes -> bounded parser -> semantic model (+ legacy data per object) -> edit/layout -> writer + loss report`.

- Caches (`[pg]`, `<:#…>`, anchored rectangles, frame column edges, sequence numbers) are recomputed, not imported.
- SAM's positional numbers (frame sequence, `<:A n>`/`<:t n>`, layout names in breaks) are mapped to IDs on import and regenerated on export.
- Use explicit version detection, and refuse unsafe write-back while round-trip fidelity is not established.
- The format is specified in `docs/research/SAM_PAGE_FRAME_SPEC.md`.

## Reference isolation
Read references from `AMIPRO_REFERENCE_DIR` or `../AmiPro-Reference`. Only file metadata and independently written findings enter Git. Do not embed original resources in snapshots or tests. Keep the SHA-256 manifest private, or store hashes only.

## Decision record
- **Phase 0:** a custom semantic document and pagination model on maintained shaping and editing primitives (`docs/research/ARCHITECTURE_DECISION.md`).
- **Phase 1:** the engine reuse audit kept that choice (`docs/research/WORD_PROCESSOR_ENGINE_REUSE_AUDIT.md`).
- **Phase 2.2:** the page, layout, and frame model above, decisions D1–D8 (`docs/research/DECISIONS.md`, 2026-10-11, based on `docs/Design/PAGE_FRAME_MODEL_BRIEF.md`).

## Known risks
- Remaining format unknowns, all preserved rather than interpreted: frame field 8, `[isd]` numbers, table cell fields, the effect of `<:c>`.
- Line breaking with exclusions and inline boxes over COSMIC Text shaped words. Bidi, and ligatures at breaks, are the likely failure points; Parley is the fallback.
- The cost of `Arc` snapshots in documents with thousands of table-cell flows. Measure it in 2.3 against the current snapshot cost.
- Selection and hit testing across pages and frames (2.6).
- Behavior of original macros.
- Iced/libcosmic toolkit evolution.

Mitigate these with representative fixtures, small vertical slices, pinned dependency versions, and regression tests.
