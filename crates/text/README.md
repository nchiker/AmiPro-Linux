# amipro-text

The plain text of an Ami Pro document (SAM or native), read from bytes, for search and indexing.

- `sniff(head)` tells the kind from the first bytes (never the file name).
- `extract_text(bytes, &Limits)` returns `Extracted::Text { title, text, truncated }`, `Protected` (password files are
  never read), or `Unreadable(TooLarge | NotAmiPro | Damaged)`. It does not panic on any input.
- Text is the main flow, then the other flows (frames, table cells, notes), one line per paragraph, cut at
  `max_chars` on a character boundary.
- `VERSION` changes when the text for the same input changes.

Depends only on `amipro-document` and `amipro-formats`. Tests use synthetic files only
(`cargo test -p amipro-text --offline`).
