# Rich Editor Surface Architecture

**Status:** Accepted incremental direction, 2026-10-09.  
**Related decisions:** `docs/research/ARCHITECTURE_DECISION.md` and `docs/research/WORD_PROCESSOR_ENGINE_REUSE_AUDIT.md`.

## Constraint confirmed against the pinned stack

The current Iced `text_editor` is a reliable native interaction widget and internally uses COSMIC Text. Its public highlighter format exposes only color and font identity. It cannot assign a different font size, line metric, paragraph spacing, or underline to a span, and its `Content` cannot be constructed from an externally prepared rich COSMIC Text buffer.

The existing AmiPro-Linux `rich_buffer` adapter proves that COSMIC Text itself carries the required per-span size, family, weight, style, underline, and per-paragraph line metrics. The missing layer is a libcosmic widget that uses that derived buffer for both rendering and interaction.

Consequently, making the current editor selectable, placing a formatted view behind it, or switching between two surfaces cannot satisfy WYSIWYG editing. Those approaches would give glyphs and cursor/selection different geometry.

## Required ownership boundary

The semantic `Document` remains the only document model. A custom page editor may own only derived and transient state:

- a COSMIC Text buffer rebuilt or incrementally refreshed from `Document`;
- caret, selection, preferred horizontal position, composition/preedit, and scroll state;
- shaped-line, hit-test, and page-fragment caches keyed by model revision and geometry;
- renderer caches that can be discarded without data loss.

All text and formatting mutations must enter the existing command/model pipeline. Undo/redo records semantic changes, not widget snapshots. Native serialization reads only the semantic model. This prevents a second editing pipeline from emerging.

## Incremental implementation gates

1. **Geometry adapter:** expose shaped runs, line metrics, hit-test positions, and decoration geometry from the existing rich buffer without changing the displayed editor. Add deterministic tests for Unicode byte/character boundaries and mixed metrics.
2. **Read/write widget prototype:** render one editable paragraph directly from the derived buffer and route insert/delete/motion/selection through model commands. It must pass IME, clipboard, pointer hit-testing, cursor motion, selection, and undo/redo tests before replacing any part of the working editor.
3. **Single-surface replacement:** replace the current Iced editor only when the custom surface preserves all baseline behavior while displaying variable size, underline, and paragraph spacing. Do not keep a fallback preview in the application UI.
4. **Paragraph and page fragmentation:** extend the same buffer/model mapping across paragraphs and page fragments. Cursor and selection positions remain document positions, never page-widget-local text copies.
5. **Accessibility and physical acceptance:** expose appropriate text/editing semantics, then verify scaling, IME, selection, undo/redo, safe open/save, and reopen on physical COSMIC.

## Prototype checkpoint (2026-10-09)

Gates 1 and the isolated implementation portion of gate 2 are complete. The
`rich_editor_prototype` module renders borrowed semantic spans with Iced's
renderer paragraph and uses that same paragraph for UTF-8 byte hits, caret
position, and bidi-aware selection rectangles. The widget owns only focus,
drag, preedit, and selection state. Typing, paste, cut, delete, and IME commit
emit actions that use the editing crate's atomic `Replace` command, preserving
character runs and one-step undo/redo.

The Cargo feature `rich-editor-prototype` now mounts this surface for all
documents so the expanded path can be tested on a physical COSMIC session.
Default builds continue to use the accepted Iced editor. This is an
experimental test seam, not a preview or a second document model.

The one-paragraph portion of gate 2 passed its focused physical checks. The
expanded multiparagraph portion is not accepted until physical testing covers
paragraph-boundary pointer placement/drag selection, horizontal and vertical
navigation, Enter/join, clipboard, undo/redo, style synchronization, and safe
save/reopen. Accessibility semantics and inline preedit drawing remain explicit
gaps; the surface must not replace the default editor before they are addressed.

The first physical gate run failed because the surface used `Length::Shrink`
and exposed only a one-line mouse/focus target inside the page. Consequently,
clicks on ordinary page whitespace never focused it and keyboard/IME events
were intentionally filtered. The corrected surface fills the page text area,
tracks window focus, requests IME state on redraw, and has a direct widget-event
test covering off-line page clicks, ordinary text events, Unicode IME commit,
and input-method activation. This correction still requires physical retest.

The follow-up physical run confirmed typing, drag selection, clipboard, and
undo/redo in new and one-paragraph native documents. It exposed two narrower
issues: the caret used the paragraph's default font height instead of the
mixed-size shaped line box, and the multiparagraph fallback rebuilt and
reshaped the complete imported document even for cursor and selection actions.
The caret now derives its vertical bounds from adjacent shaped glyph geometry.
The fallback reconciles the semantic document only for editing actions,
explicitly maps Ctrl+Z/Ctrl+Shift+Z to document history, and delegates
copy/cut/paste to the existing Iced bindings. Physical confirmation remains
required. That focused physical retest subsequently passed all five checks:
caret alignment, imported-document responsiveness, current clipboard content,
undo/redo, and new-document regression behavior. This accepts the corrected
one-paragraph gate, not multiparagraph production activation.

The semantic editor provides atomic paragraph-range replacement, including
splitting, joining, multiline insertion, cross-paragraph replacement, retained
unaffected direct-format runs, and exact undo/redo. The expanded surface now
consumes this command through a derived flat renderer coordinate mapped to
semantic paragraph/UTF-8 cursors. Its newline spans and offsets are transient
view projections; it does not reconstruct or persist a widget-owned document.

### Multiparagraph pointer regression and correction

The initial automated interaction coverage did not exercise the geometry that
failed on a physical imported SAM. Its widget test used Iced's null renderer and
an empty one-paragraph document, so `hit_test` returned no shaped hit and the
fallback was always byte zero. Separate cross-paragraph tests invoked the
controller directly. They proved semantic range editing, but bypassed the
production widget's pointer-to-renderer-to-document mapping entirely.

The defective multiparagraph renderer flattened all paragraph spans into one
Iced paragraph separated by synthetic one-pixel newline spans. This made the
visual paragraph stacking and the semantic paragraph map disagree once wrapping,
mixed metrics, and paragraph spacing were present. A real shaped-widget
regression run against commit `402ccae` reproduced the error: a forward drag
whose intended third-paragraph endpoint was document byte 134 emitted byte 15.
The same test emits byte 134 after the correction.

The corrected surface shapes each semantic paragraph independently, stacks the
resulting bounds with its model-derived after-spacing, and uses the selected
paragraph's local COSMIC Text hit before converting the UTF-8 cursor to a flat
document offset. Caret, selection, underline, vertical navigation, and IME
cursor geometry consume the same composite paragraph list. The regression
drives the production `Widget::update` path with Iced's real COSMIC Text-backed
graphics paragraph implementation and verifies press, forward and reverse drag,
release/capture, wrapped lines, UTF-8, mixed size/bold/underline, and a translated
widget origin representing scroll offset. Controller/model tests remain a
separate verification layer.

A dedicated nested/headless Wayland test was considered. The host provides
`cosmic-comp` but no supported input-injection harness; compositor launch alone
would not add evidence beyond the real shaped-widget event test, while building
such a harness would be disproportionate for this correction. Physical COSMIC
testing therefore remains the third and required acceptance layer.

The corrected pointer path subsequently passed physical COSMIC testing with a
representative multiparagraph SAM. Drag selection was functional across the
previously failing paragraph boundaries. The user observed sluggish interaction;
that is a bounded performance follow-up, not a reason to alter the accepted
semantic-model/COSMIC Text architecture. The default editor remains active and
the custom surface remains behind `rich-editor-prototype`.

The bounded performance follow-up found that selection-only view updates were
eagerly reprojecting and reshaping every paragraph. The surface now retains
renderer-native shaped paragraphs in Iced widget-tree state and invalidates them
with an explicit document-session/layout generation plus geometry and renderer
constraints. Selection, caret, focus, and outer scrolling reuse the cache; text
or formatting edits, undo/redo, width/scale changes, and tab switches rebuild it.
The same-SAM warm selection-layout benchmark dropped from a 9.351-9.432 ms
complete pre-change pass to 0.039-0.044 us while cold shaping remained roughly
8.3-8.6 ms. Automated gates pass; physical drag, typing, and scrolling acceptance
is still required. See `../research/RICH_EDITOR_INCREMENTAL_LAYOUT.md`.

## Reuse policy

Use COSMIC Text shaping, bidi, font fallback, buffer editing primitives, hit testing, and glyph layout directly. Parley remains a narrowly scoped fallback only if a reproducible layout or hit-testing gap survives a focused COSMIC Text investigation. The engine reuse audit found no complete word-processor component with a better supported integration boundary.

## Rollback and safety

Each gate must leave the repository buildable and retain the current editor until its replacement clears equivalent automated and physical checks. Experimental renderer/cache state must not enter `.amipro` files. SAM/STY source protection and native atomic saves are independent of the editor widget and remain mandatory throughout.
