# Synthetic SAM fixtures

Written by hand from `docs/research/SAM_PAGE_FRAME_SPEC.md` (draft 2) and
`docs/research/SAM_FORMAT_NOTES.md`. They are not copies of Ami Pro output and
contain no proprietary bytes; the "picture" is a short ASCII string.

`pages-and-frames.sam` holds, in Ami Pro's section order:

- two styles (`Body Text`, `Heading`);
- six page frames, back to front: a Where placed two-column text frame with a
  name and a macro; a group of two frames on page 2; a With para above frame
  (anchor 0) with top and bottom lines and a shadow; a Flow with text table
  (anchor 1) with a row and a column record, a covered cell, and a calculated
  cell whose formula follows its value; a Where placed picture copied into the
  file (sequence 8);
- the `Standard` layout with left pages, header text with a page-number
  field, a left header, footers, and a Repeat right/left frame, and an
  inserted layout `1`;
- `[elay]`, `[l1]`, and a `[pg]` cache;
- a body with a layout cache, bold, a hard return, a discretionary hyphen,
  both anchors, a font change kept as a legacy mark, a column break, a
  floating footer, a page break that switches to layout `1`, and an escaped
  literal `<` and `>`;
- the picture bytes, Ami Pro's rendering, the `[Embedded]` directory, and the
  trailer with the directory's offset.

The file uses CRLF line ends like Ami Pro. Its byte offsets matter: edit it
only with the offsets recomputed.
