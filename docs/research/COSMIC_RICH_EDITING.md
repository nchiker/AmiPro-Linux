# COSMIC Rich Editing Investigation

**Date:** 2026-10-09  
**Scope:** pinned libcosmic revision `dbc62f2d899cdbfbe43d170689ddc0290db663f4`

## Result

The Iced `text_editor` used by libcosmic is backed by COSMIC Text. It already provides shaped text input, cursor navigation, mouse hit testing, selection, clipboard behavior, wrapping, and IME integration. Its `highlight_with` interface can apply a different font and color to each byte range in each logical line. AmiPro-Linux now feeds paragraph-style and character-run font family, weight, and italic attributes from the existing `Document` directly into that editor. The former read-only styled preview and edit/preview switch were removed.

This is one editing surface and one semantic document model. The editor content remains an input/view adapter: every edit reconciles into `Document`, formatting commands mutate `Document`, and the next view derives all displayed attributes from `Document`. No second formatting model was introduced.

## Measured API boundary

The public editable highlighter format contains only `color` and `font`. Its COSMIC Text renderer builds an `AttrsList` from those values. It does not expose per-range metrics or decorations, although COSMIC Text 0.19 itself supports `Attrs::metrics` and text decorations.

Consequently, the unified editor currently displays:

- paragraph and character font-family categories (serif, sans-serif, monospace);
- paragraph and character bold and italic state;
- live changes from style application, direct formatting, undo, and redo.

It does not yet display variable point sizes, paragraph spacing, underline, page breaks, or frame geometry while editing. Those values remain preserved in the semantic model and native save format.

## Decision and next increment

Keep the supported COSMIC Text editor for the corrected Phase 1 workflow instead of implementing another text engine. The first custom-widget increment is implemented: `rich_buffer` derives a lower-level COSMIC Text buffer from `Document`, including point-size metrics, paragraph after-spacing, exact font-family requests, bold/italic, and underline. It is rebuilt after model changes and has attribute/paragraph-mapping tests, but is not yet the rendered input surface.

The next increment will render and edit through that buffer, preserve COSMIC Text hit testing/IME behavior, and translate edits back through the existing document/command interfaces. It must not own an independent document representation.

Phase 1 WYSIWYG acceptance remains open until variable paragraph metrics are visible and the physical COSMIC checklist passes.
