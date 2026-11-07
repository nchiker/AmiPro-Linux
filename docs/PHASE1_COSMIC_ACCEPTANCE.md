# Phase 1 Physical COSMIC Acceptance Checklist

## Known-good baseline result — 2026-10-09

The implementation through commit `5b34720` passed the six physical COSMIC acceptance scenarios reported by the user, with no observed problems:

- [x] Graphical Open dialog and document opening.
- [x] Safe native Save As after legacy import, without modifying the source.
- [x] Subsequent Save to the selected native document.
- [x] Formatted editing in the unified editable surface.
- [x] Selection and undo behavior.
- [x] Reopen of the saved native document with content and formatting preserved.

This result establishes the known-good baseline for the next UI implementation sequence. It does not approve or validate the redesign proposed in `docs/Design/UI_SPEC.md` and `UI_MOCKUP.svg`, and it does not close Phase 1. Any approved revised interface will require a new physical pass.

## Feature-gated rich-editor prototype checklist

### Initial physical result — failed

The first physical COSMIC run launched and rendered imported SAM formatting,
but neither an imported document nor a new empty document accepted text. This
is a failed Phase 1 acceptance result. The custom surface had shrunk its input
bounds to one shaped line inside the page, so clicks elsewhere on the paper did
not acquire focus or request the Wayland input method. Automated coverage now
exercises a click near the bottom of an empty page followed by keyboard and IME
input; physical confirmation remains required.

### Follow-up physical result — partial, still failed

The next physical run confirmed that new documents and the one-paragraph rich
fixture accept input. Drag selection and copy/cut/paste/undo/redo worked in the
fixture, and a new document had correct caret placement and working
copy/paste/undo. The fixture caret was drawn above the mixed-size text. An
imported multiparagraph SAM used the intended fallback editor and had correct
caret placement, but editing was slow; cut worked while paste appeared to use
older clipboard content and undo did not work. These are failed acceptance
items, not a completed gate.

The caret now uses the shaped line's actual vertical box. Cursor and selection
actions in the fallback no longer reconstruct and reshape the whole semantic
document, Ctrl+Z/Ctrl+Shift+Z explicitly use document history, and Ctrl+C/X/V
continue through Iced's native clipboard bindings.

### Narrowed host retest after the follow-up fixes

**Result — passed on physical COSMIC:** all five checks passed. The caret aligned
with the text, imported-SAM navigation and selection showed no issue, cut/paste
used the current selection, undo/redo behaved correctly, and the new-document
regression check passed. This accepts the focused corrections in `d6477b0`; it
does not accept multiparagraph rich-surface editing or activate the prototype
as the production editor.

Launch from `app/cosmic` with the command below, then perform these checks:

1. Open `tests/fixtures/rich-editor-prototype.amipro`. Click at the start,
   middle, and end of each differently sized run. Confirm the caret spans and
   aligns with the containing text line rather than appearing above it.
2. Open the same disposable multiparagraph `.SAM` used in the failed run. Move
   the caret repeatedly and drag-select text; confirm these operations respond
   promptly without whole-document pauses.
3. Select a distinctive word in the SAM, press Ctrl+X, then Ctrl+V. Confirm the
   just-cut word—not content from the previous document—is pasted at the caret.
4. Press Ctrl+Z once and confirm the paste is undone; press Ctrl+Shift+Z and
   confirm it is restored. Repeat once with ordinary typing.
5. Choose New and briefly confirm typing, copy/paste, and undo remain working.

The reported result for items 1–5 was pass, with no issue observed for item 2.

### Corrected host retest

This bounded pass is required before the custom surface can replace any part
of the default editor. Launch it from `app/cosmic` with:

```sh
cargo run --offline --features rich-editor-prototype
```

1. Choose **New**, click well below the first text line but inside the white page, and type `Empty café 🙂`. Confirm a caret appears at the start and the complete text appears.
2. Press Left twice, type `X`, then test Backspace, Delete, Undo, and Redo. Confirm each operation occurs at the visible caret and one Undo reverses one edit.
3. Open `tests/fixtures/rich-editor-prototype.amipro` from the graphical chooser. It is an independently authored one-paragraph fixture containing mixed sizes, underline, bold, and italic. Click within the text, type, drag-select, replace, copy, cut, and paste; confirm formatting stays visible and the renderer updates after each edit.
4. Commit non-ASCII text through the desktop IME. Confirm composition is not inserted early, the commit appears once, and one Undo removes the whole commit.
5. Open a disposable original `.SAM`. Repeat typing and deletion in the rich surface and confirm it remains editable rather than ignoring input.
6. Save As to `.amipro`, make another edit and Save, then reopen. Confirm content and formatting survive and the original SAM remains unchanged.

- [ ] New empty document accepts keyboard and Unicode input after clicking anywhere inside the page text area.
- [ ] Apply Heading 1/Heading 2 and confirm the live editable glyph size changes; confirm the fixture's underline remains visible while editing.
- [ ] Click at the start, middle, and end of ASCII and non-ASCII text; confirm the caret lands at the expected character boundary.
- [ ] Drag left-to-right and right-to-left selections; type to replace each selection, then Undo and Redo once per replacement.
- [ ] Navigate with Left/Right/Home/End, with and without Shift; confirm caret and selection geometry follow the rendered text.
- [ ] Copy, cut, and paste formatted surrounding text; confirm the text changes and unaffected direct formatting remains intact.
- [ ] Commit non-ASCII text through the desktop IME; confirm one commit creates one undo step and no preedit text enters the document prematurely.
- [ ] Save As to `.amipro`, make a subsequent edit and Save, then reopen; confirm content and paragraph/direct formatting survive.
- [ ] Open an imported `.SAM`, confirm the feature-gated rich surface remains editable across paragraphs, and verify Save still requires a native Save As target without modifying the source.

Record exact symptoms for caret placement, drag selection, keyboard focus,
clipboard, IME, or rendering failures. This checklist does not accept
accessibility or production activation.

### Expanded multiparagraph gate

Automated coverage now verifies UTF-8 renderer/document coordinate mapping,
paragraph split and join, cross-paragraph selection replacement and clipboard
text, single-step undo/redo, vertical-key event routing, toolbar/status anchor
synchronization, retained formatting, native save/reopen, and unchanged legacy
source bytes. Builds and strict Clippy pass with and without the feature. The
following visual/compositor-dependent checks remain physical:

The original automated gap is now covered by a real shaped-widget regression.
The earlier test used the null renderer with an empty one-paragraph document,
whose hit test always fell back to byte zero, while cross-paragraph tests called
the controller directly. Against the defective flat-paragraph implementation at
`402ccae`, the new production-handler test mapped a drag intended for document
byte 134 to byte 15. The composite-paragraph correction maps it to byte 134 and
also covers reverse drag, release/capture, wrapping, mixed metrics and underline,
UTF-8, and a translated scroll origin. This is automated widget evidence, not a
physical acceptance result.

Verification layers for this correction:

1. **Model/controller:** deterministic paragraph split/join, UTF-8 cursor mapping,
   cross-paragraph selection/replacement/clipboard, formatting retention, and
   single-step undo/redo.
2. **Widget/integration:** the actual rich-editor `Widget::update` handler plus
   COSMIC Text-backed Iced paragraph shaping and renderer-to-document hit mapping,
   including press, drag in both directions, release, wrapping, mixed formatting,
   and scroll-coordinate translation.
3. **Physical COSMIC:** pending the focused checklist below; required before the
   defect is accepted as fixed or the feature gate is promoted.

### Mouse-selection correction physical result — passed

**Result reported 2026-10-09:** the feature-gated rich editor passed physical
COSMIC testing with a representative multiparagraph SAM. Forward and reverse
mouse selection worked across the previously failing paragraph boundaries,
including the focused replacement/undo/redo check requested for this gate. The
interaction was somewhat sluggish but functional. This accepts the
composite-paragraph pointer correction; it does not remove the feature gate or
promote the custom editor over the working default editor.

The sluggish interaction is tracked as a performance issue. A bounded baseline
will measure drag-event hit testing, paragraph projection/shaping, and the
selection-triggered layout/render path, then remove only obvious unnecessary
work. It is not evidence for an architecture redesign.

### Cached-layout performance physical result — partial acceptance

**Result reported 2026-10-09: PARTIAL PASS / NOT CLOSED.** The shaped-layout
cache improved selection and scrolling. Multiparagraph mouse selection works,
ordinary scrolling is improved, mouse-wheel scrolling works, and typing in a
new empty document is responsive. These results preserve and accept the
selection/scrolling improvements themselves.

The performance gate remains failed for large-document typing: inserting text
into an existing paragraph of the imported approximately 18-page
`README31.SAM` is extremely slow. The earlier isolated projection, shaping,
hit-test, and warm-cache microbenchmarks did not predict this physical latency;
end-to-end realistic editing instrumentation and a further correction are
required. Automated results must not be treated as physical responsiveness
evidence.

The same run also rejected the current interface design. The Edit menu exposes
only Undo and Redo; other editing commands depend on memorized shortcuts; menu
items do not show shortcuts; the toolbar is confusing and poorly grouped;
style information appears in roughly four places; the status bar has spacing
and readability problems; and no sufficiently detailed approved visual
specification governs the implementation. These are design-gate failures, not
authorization to implement an unreviewed redesign.

### Typing recovery implementation — automated pass, physical pending

Commit `bf0eaa5` traces the physical failure to two hidden full-document layout
passes on every rich edit: rebuilding Iced's fallback `Content` and refreshing
an unused derived `DocumentLayout`. The correction retains and incrementally
synchronizes the fallback text, skips that unused layout in prototype builds,
and reshapes only changed paragraphs while document geometry and style inputs
remain stable. Conservative full invalidation remains for paragraph-count,
session, width, scale, and renderer-font changes.

On the same private `README31.SAM` fixture, the final release benchmark measured
sustained typing at 0.285 ms median / 0.300 ms p95 / 0.478 ms worst and isolated
insertion at 0.294 / 0.432 / 0.491 ms. The removed fallback and derived-layout
rebuilds measured 17.558 ms and 84.695 ms median respectively. On `AP31NAV.SAM`,
sustained typing measured 0.144 / 0.253 / 0.263 ms. These headless CPU timings
exclude event-loop scheduling, GPU rasterization, compositor presentation, and
visible-pixel latency; they do not close the physical gate.

Automated verification passes default/all-feature COSMIC tests, strict Clippy,
both builds, core tests, UTF-8/newline/history regressions, realistic release
benchmarks, and the complete 160-SAM/63-STY corpus. The accepted default editor
and `rich-editor-prototype` feature gate are unchanged.

**Physical update, 2026-10-10: PRIMARY DEFECT RESOLVED.** Typing in
`README31.SAM` now feels normal. This physically closes the reported
large-import typing-responsiveness defect. The user did not report results for
the remaining drag-selection, scrolling, menu/toolbar Undo/Redo, caret
visibility, or independent-tab regression checks below; they remain pending
and are not inferred as passed. Do not reopen P0 unless one of those checks
identifies a regression.

If the pending regression checklist is completed, use the feature-gated editor
with a new document plus `README31.SAM` and `AP31NAV.SAM` where available.
Exercise insertion, deletion, selection replacement, Undo/Redo, caret
visibility, scrolling, and edits near the beginning, middle, and end. Record
any stale text, formatting, selection, or caret geometry. These checks do not
reopen the resolved P0 issue unless a regression is actually found.

1. Open a representative multiparagraph SAM and click at the start/end of
   adjacent paragraphs, including non-ASCII text. Confirm the caret follows the
   clicked glyph and does not jump to the preceding paragraph.
2. Drag a selection forward and backward across at least three paragraphs.
   Confirm every selected line is highlighted, then type once and verify one
   Undo/Redo restores/reapplies the complete replacement.
3. Press Enter in the middle of styled text. Confirm a new paragraph appears at
   the caret with formatting outside the split preserved. Backspace at its
   start and Delete at the previous paragraph's end; confirm both joins work.
4. Navigate across paragraph boundaries with Left/Right/Home/End and Up/Down,
   repeating with Shift. Confirm caret/selection geometry and the toolbar/status
   paragraph style remain synchronized.
5. Cut and paste a selection spanning paragraphs, then save as `.amipro`, edit,
   Save, and reopen. Confirm content and unaffected formatting survive and the
   SAM source remains unchanged.
6. Confirm mixed font sizes, underline, and paragraph spacing remain visible
   while performing the operations above and that interaction remains
   responsive on the representative SAM.

## Revised-interface checklist

Run this checklist on Pop!_OS COSMIC with a disposable copy of a representative `.SAM` document and one existing disposable `.amipro` document. Keep the original legacy source outside the repository and record its checksum before starting.

- [ ] Launch the current debug or release build on COSMIC; confirm the document canvas is editable and there is no edit/preview switch.
- [ ] Open each menu heading. Confirm a conventional vertical menu appears, implemented File/Edit/View/Window commands work, and unavailable commands are visibly disabled rather than acting like placeholders.
- [ ] Create two new documents and open one `.SAM` plus one `.amipro`. Confirm each has one readable tab, the active tab is obvious, the tab strip scrolls when crowded, and opening the same path again activates its existing tab rather than creating a duplicate.
- [ ] In different tabs, set different text, caret/selection, paragraph style, undo history, and scroll position. Switch with clicks and Ctrl+Tab/Ctrl+Shift+Tab; confirm every state is restored independently.
- [ ] Make one tab dirty and confirm its marker appears. Close it and exercise Cancel, then Save (including legacy-source Save As), and finally Discard on disposable changes; confirm the chosen operation affects only that tab and closing the last tab leaves a fresh document.
- [ ] Confirm the compact toolbar contains no inert alignment/zoom placeholders and remains usable without crowding out the document.
- [ ] At approximately 1280×800 and 1920×1080, resize the window in both directions; confirm the workspace fills the remaining area, the paper stays centered, and no bottom or right-edge content is clipped.
- [ ] Narrow the window until the minimum page scale is reached; confirm horizontal scrolling exposes both paper edges and vertical scrolling reaches the bottom of the page.
- [ ] Confirm the displayed paper proportions and margins are credible, and the ruler remains horizontally aligned with the paper while resizing and scrolling.
- [ ] Confirm the toolbar style selector and status bar initially show the caret paragraph's style. Move between differently styled paragraphs and confirm both update together.
- [ ] Select paragraphs with different styles; confirm the toolbar/status show a mixed state. Choose a style and confirm all selected paragraphs, the sidebar highlight, toolbar, and status agree.
- [ ] Toggle the Styles panel; confirm it opens without clipping the document, scrolls when necessary, applies a style, and restores the full-width workspace when hidden.
- [ ] Choose **Open**; confirm the graphical system file browser opens, supports directory navigation, and filters for `.SAM` and `.amipro` documents.
- [ ] Cancel Open; confirm the current document and its text are unchanged.
- [ ] Open the disposable `.SAM`; confirm its text appears in the editable canvas and its visible bold/italic/font-family formatting remains present while typing and moving the cursor.
- [ ] Select text, apply Bold and Italic, apply a paragraph style, then Undo and Redo; confirm selection/cursor behavior and visible formatting update in the same canvas.
- [ ] Choose **Save**; confirm it opens graphical Save As instead of writing the `.SAM` source, proposes a `.amipro` filename, and permits directory navigation.
- [ ] Cancel Save As; confirm no new file is created and the legacy source checksum is unchanged.
- [ ] Invoke Save again and choose an existing `.amipro`; confirm replacement requires confirmation. Cancel that confirmation and verify the existing file is byte-for-byte unchanged.
- [ ] Save to a new `.amipro` target; confirm the file is created and the legacy `.SAM` checksum remains unchanged.
- [ ] Make another edit and choose **Save**; confirm only the selected `.amipro` changes and no chooser appears.
- [ ] Choose **Save As**, select a different native filename, and confirm subsequent Save updates only that new target.
- [ ] Attempt to use a `.SAM` or `.STY` filename as a Save As target; confirm the app refuses it and neither legacy file changes.
- [ ] Reopen the saved `.amipro`; confirm text, paragraph style assignments, and direct bold/italic formatting are preserved.
- [ ] Confirm a large enough document still permits cursor navigation, selection, scrolling, and editing without switching surfaces.

Record the build commit, COSMIC version, pass/fail result for each item, and screenshots or exact symptoms for any failure. Variable font sizes, paragraph spacing, underline display, shaped pagination, and frames are known pending custom-widget acceptance items and must not be marked passed by inference. Page presentation, ruler alignment, and responsive scrolling are implemented but remain unaccepted until the checks above pass physically.
