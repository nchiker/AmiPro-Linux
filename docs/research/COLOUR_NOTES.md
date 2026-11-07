# Colour Selection Notes

Evidence for colour selection (`docs/Design/UI_SPEC.md` §23, decisions C1–C4
in `DECISIONS.md`, 2026-10-18). Decompiler output stays outside Git; these
are findings.

## Ami Pro's colour controls

- `AMIUICNT.DLL` registers three classes: COLORBAR (`SAMMYCOLORBARPROC`,
  `1010:002c` → `1028:0000`), COLORPALETTE (`1010:0057` → `1028:1462`), and
  COLORSELECTOR (`1010:0082` → `1028:22cc`). Dialogs place them as
  `samcolorbar` controls.
- Dialogs with colour bars (`AMIPROUI.DLL`): Font (1006, one), Modify
  Style (1028: the text's colour 25590 and the lines' 25591), Modify Page
  Layout (1034: lines, 25590), Modify Frame Layout (1036: Lines & shadows'
  Line 25590, Shadow 25593, Background 25591; Columns & tabs' line between
  columns 25592), Lines & Color for tables (3052), User Setup (1019),
  Revision Marking Options (2003, two), Edit SmartIcon (1091).
- **The bar** (VM, Font dialog, 2026-10-18): 15 colours, then ▼. Left to
  right (RGB): 255 0 0, 255 66 30, 255 129 0, 255 255 0, 0 255 0,
  0 255 178, 0 255 255, 0 128 255, 0 0 255, 97 63 255, 129 0 255,
  255 0 255, 255 0 128, 255 255 255, 0 0 0 (the app's `PALETTE`).
- **The palette** (▼): 8 rows of 15 cells, 13 hues in tints and shades and
  two grey columns, sampled from the screen (hex, row by row):

  ```text
  ffc0ce ffc0b6 ffc281 ffffc2 c2ff91 c1ffd5 c1ffff c2efff e0f1ff e0e0ff e1bfff ffc1fd ffc0e4 ffffff 8f8f8f
  ff82a0 ff8080 ffbf18 ffff80 80ff80 82ffca 80ffff a1e2ff c0e1ff bfbfff e29fff ff7fff ff82c2 f7f7f7 808080
  ff4070 ff8141 ff8100 ffff00 41ff32 42ffc7 00ffff 3fc0ff 819fe2 8080ff c082ff ff42f9 ff40a0 efefef 727272
  ff0000 ff421e e26200 bfbf00 00ff00 00ffb2 00e0e0 0080ff 4181ff 4141ff 8242ff ff00ff ff0080 e1e1e1 5f5f5f
  c20000 e12000 a13f00 a1a100 00c200 00c196 00c1c2 0062e1 0000ff 4100ff 8100ff a1009f c0007f d2d2d2 4f4f4f
  810000 a00000 824200 818100 008000 009f82 008280 004080 000080 2200a1 4100c2 800080 820040 c0c0c0 404040
  d0b1a1 e0a175 d2b06a c0c27c 82c168 81c097 7fc2bc 71b2cf 9f9fe0 c0a1e0 e29fde ef91eb e29fc8 b2b2b2 2f2f2f
  7f604f a16252 806210 82823f 3f621f 3c613e 37605e 104160 6260a1 624181 603181 602162 622152 a2a2a2 000000
  ```

  It ignores the keyboard; a click on ▼ or a cell closes it. Ami Pro has
  no way to mix a colour (not offered, C2).
- Files store a colour as a `COLORREF` (`0x00BBGGRR`, 24-bit), so any
  colour the app mixes is written and read back as it is.

## libcosmic's colour picker (pinned `dbc62f2`)

`cosmic::widget::color_picker`: `ColorPickerModel` (HSV state, `#RRGGBB`
or RGB entry, its own recent list, Reset/Save/Cancel when labelled) and its
builder. The saturation and value field and the hue slider send
`ActiveColor` while dragging and `ActionFinished` on release; the entry
sends `Input` and `ActionFinished` on Enter. Its `on_update` is a plain
`fn`, so the app keeps which colour the open panel is for. On
`ActionFinished` the model pushes the colour it had into its own recent
row; the app keeps its own Recent (C4) instead.
