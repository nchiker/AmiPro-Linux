# Fields and References Playbook

Part of `docs/PHASE5_PLAYBOOK.md` (queue rows 4, 10, 12, 13, 23). Follow
its recipe and standing rules; this file says what each run covers, what
to map, and what to expect.

**Why one playbook.** Each feature here does one of two things:
- It marks text and later shows something computed from it: a date, a
  document's information, a field's result, a footnote's number, a TOC or
  an index, a revision.
- It walks the document's text streams to generate or check something.

They share three foundations:
- Typed inline objects for marks and fields (U+FFFC, like anchors and
  page numbers), with their shown text computed at layout and their stored
  form written back as Ami Pro writes it.
- One walk over the streams in Ami Pro's order (`SAM_FORMAT_NOTES.md`,
  "Text streams"), shared with Find & Replace.
- Generation into a document.

**State today.** Every control here is kept as a zero-width legacy mark
and written back verbatim (`FORMAT_COMPATIBILITY.md`): nothing is shown
where Ami Pro shows a date, a field's result, a note mark, or a footnote.
The page-number field (`<:P…>`) and floating headers are done. The
registry has Insert › Field…, Footnote (Alt+Ctrl+F), Endnote (Alt+Ctrl+D),
and File › Document Properties as Future commands.

**Evidence to start from.**
- **Guide:** ch. 5 (inserting the date or time), ch. 20 (bookmarks), ch.
  23 (Doc Info), ch. 24 (revision marking, Doc Compare, notes), ch. 25
  (footnotes, TOC, index, glossary; the thesaurus is in Proofing), ch. 26
  (master documents), ch. 32 (power fields).
- **Commands:**
  - Edit › Insert ▸ Date/Time… (0x9e), Note (0x11a), Merge Field…
    (0xb6), Doc Info Field… (0xb7), Glossary Record… (0x1d3, Ctrl+K);
  - Edit › Power Fields ▸ Insert… (0x238), Update (0x23b), Update All
    (0x10b), Next Field (0x239), Prev Field (0x23a); View › Show Power
    Fields (0x10a);
  - Edit › Mark Text ▸ Index Entry… (0xb4), TOC Entry… (0x343),
    Glossary… (0xc7), Revision Insertion (0xbb); Edit › Bookmarks…
    (0x1d2);
  - Tools › Footnotes… (0xa0), Revision Marking… (0x1cc), Doc Compare…
    (0x1f4), TOC, Index… (0x1d1);
  - File › Doc Info… (0x6d), Master Document… (0x92), and Save As
    Master (0x82, menu 1003).
- **Decompile already found:**
  - the control parser `11b8:030a`, which takes `D`, `b`, `k`, and `r`
    together, then `T`, `X` and `Z` (named, looked up by
    `1628:0000`/`0115`), `V` and `f`, `e`, `d`, `M`, `N`, `F`;
  - `[toc]` (`11c8:1bbc` → `1588:261d`), `[book]` (`1580:1368`),
    `[master]` and `[sequence]` (`1660:0000`, `0748`), `[docvars]`
    (`1630:1dd7`);
  - `[desc]`'s 21 values (`11c8:342b`, read by `1058:2732`).
- **In the files** (user's corpus / ISO):

  | Mark or section | Files | Samples (shapes only) |
  |---|---|---|
  | `<:D…>` (one letter after it) | 43 / 1 | alone at the top of letters: likely the date |
  | `<:T>` | 48 / 0 | no arguments; to map |
  | `<:r…>` | 10 / 0 | one letter; to map |
  | `<:X…>` | 4 / 7 (75 / 220 marks) | `<:X8,-32768;Index "…" # "…">`: index entries |
  | `<:Z name>` … `<:Z~name>` | 7 / 1 | paired and named: likely bookmarks |
  | `<:N…>` | 7 / 1 | a number, initials, a colour, then the note's text over lines |
  | `<:F…>` | 2 / 0 (6 marks) | the footnote's own paragraph in the Footnote style |
  | `<:M name>` … `<:M>` | 1 / 0 (16) | merge fields (Merge and templates) |
  | `<:V…>` | 0 / 1 | to map |
  | `<:>` | 4 / 25 | to map |
  | `[toc]` | 53 / 3 | |
  | `[book]` | 33 / 1 | |
  | `[master]`, `[sequence]` | 50 / 56, — / 59 | the ISO's user's guide is a master document |
  | `[docvars]` | 2 / 1 | |
  | `[revisions]` | never non-zero | |

## Run 1: `fields`, queue 4

**Goal.** Dates, times, Doc Info fields, and power fields are typed
objects that show their result as Ami Pro shows it. View › Show Power
Fields shows their codes. Edit › Power Fields ▸ inserts, updates, and
moves between them. Insert › Date/Time… and Doc Info Field… insert them,
and File › Doc Info… shows and edits the document's information. All are
written back as Ami Pro writes them.

**Map.**
- Each control's meaning and arguments (`D`, `b`, `k`, `r`, `T`, `X`,
  `Z`, `V`, `e`, `d`, `<:>`).
- The date and time formats.
- The power-field language: the predefined fields, `%D`, `%T`, and `%N`
  formats, Set and Seq variables (`[docvars]`), If, Include, and others
  as found.
- When fields update (on opening, on printing, or by command), and how a
  field is locked.
- The Doc Info dialog and `[desc]`'s fields: the description, the
  keywords, the statistics, and the eight user-defined Doc Info fields.

**Decisions to expect.**
- **Which power fields to evaluate.** Recommend every predefined field
  that needs no macro; the rest show their last result and are kept.
- **Doc Info.** Recommend a Document Properties panel in the rail.
- **Whether dates update when opened** or only on command, as Ami Pro.

**Better ways and logical additions to offer.**
- A field picker with a live preview of the result.
- Fields shown shaded so that they can be told from text (a View
  option).
- Click a field to edit it in place.
- Update All before printing and export.

**Measures.** Fields change line widths, so the layout comparison will
move: each changed row should come closer to Ami Pro's cache.

**Done (2026-10-20):** `docs/FIELDS_PROGRESS.md`. Learned for the later
runs: bookmark names already keep their fields' results (`bookmark_names`,
FX6), so the notes run makes Bookmark, PageRef, and GetBookMark work out;
merge fields show `<NAME>` until the merge run gives `FieldHost` the
record; Include of a bookmark in another file waits for bookmarks; Update
All's walk (`fields::update_fields`) is where the TOC and index runs add
their generation; Print Options › Update fields belongs to the print run.

## Run 2: `notes`, queue 10

**Goal.** Notes are shown and edited. Bookmarks are added, removed, and
reached by Go To. Footnotes and endnotes are inserted, numbered, and laid
out at the page bottom or the document's end, with Ami Pro's options.

**Map.**
- `<:N…>`'s fields, the note window, and how notes print.
- Bookmarks (`[book]`, `<:Z…>`).
- Footnotes:
  - Tools › Footnotes…'s options: the starting number, numbering reset
    each page, the end of the document, the separator line, and its
    length;
  - the Footnote style;
  - footnotes in tables and frames;
  - how Ami Pro lays out the footnote area: a footnote too long for its
    page continues on the next one, and the reference moves with its
    line.

**Decisions to expect.**
- **Notes.** Recommend notes shown as margin comments in the app (an
  improvement), stored as Ami Pro's notes.
- **Footnote options.** Recommend in the rail.
- **Go To.** Bookmarks in the existing Go To dialog, with Ami Pro's Next
  and Previous.

**Better ways and logical additions to offer.**
- A notes list in the rail.
- Jumping between a footnote and its reference.
- Converting footnotes to endnotes and back.

**Measures.** Footnotes move text at page ends. This is the largest
layout change in this playbook, so measure it carefully.

**Done (2026-10-26):** `docs/NOTES_PROGRESS.md`. Learned for the later
runs: Find's streams have footnotes third (`StreamKind::Footnote`), notes
never; bookmarks are `amipro_document::bookmarks` over Ami Pro's stored
forms, with `bookmark_text` and the layout's pages (`bookmark_pages`) for
fields; a document with footnotes lays out whole on each edit (NK2: an
incremental footnote layout is a later improvement); the PDF has named
destinations for bookmarks, so the TOC run can link its entries; the
`[pg]` page cache's second part gives each page end's footnote position
(paragraph, offset, length, footnote number), not yet compared.

## Run 3: `toc-index`, queue 12

**Goal.** TOC levels by style and Mark Text › TOC Entry, index entries
(primary and secondary, see also), Tools › TOC, Index… generating both as
Ami Pro does, and glossary records (data files, Ctrl+K, Mark Text ›
Glossary).

**Map.**
- The TOC, Index dialog and `[toc]`'s options.
- Where Ami Pro puts the output (an output file or the current document;
  `_TOC.STY` and `_INDEX.STY`), page numbers, leaders, and levels.
- The index entry's form (`<:X8,…;Index "…" # "…">`), sorting, and
  grouping.
- Glossary data files and their records.

**VM fixtures.** Generate a TOC and an index for the ISO's user's guide
chapters (220 index entries) and compare the app's output with Ami Pro's
text and page numbers.

**Decisions to expect.**
- **Where the output goes.** Recommend as Ami Pro, plus an Update command
  that regenerates in place.
- **Glossary.** Recommend completing a glossary name as you type, as an
  option.

**Better ways and logical additions to offer.** A TOC whose entries jump
to their headings, an index entry list in the rail, and marking every
occurrence of a word.

**Done (2026-11-03):** `docs/TOC_INDEX_PROGRESS.md`. Learned for the later
runs: the TOC's break (flag 0x02) makes page numbers start again after it
(`PageBox::shown`), and Ami Pro computes a master document's page offset the
same way (`1588:06e4`); `[master]` is typed (`MasterSettings`) with its file
list kept as read, for the master run; glossary files are merge data files,
read by `amipro_formats::glossary`, which the merge run can extend; numeric
table cells (`<+A>5.00`, shown through the style's `[nfmt]`) are left for
the table-data run (TQ2).

## Run 4: `master`, queue 13

**Goal.** File › Master Document… lists a document's files.
- Page numbers, footnote numbers, and outline numbers run on across the
  files.
- The TOC and index cover all of them.
- Printing and PDF take the whole set.

**Map.** `[master]` and `[sequence]`, the dialog's options, how numbering
carries over, and how Ami Pro handles a missing or changed file.

**Decisions to expect.**
- Recommend a combined, read-only view of the set (an improvement), with
  each file opened for editing.
- How a master document opens from a file that names it.

**Measures.** The ISO's user's guide set is the real test: its chapters
are a master document.

## Run 5: `revisions`, queue 23

**Goal.** Tools › Revision Marking… marks insertions and deletions with
Ami Pro's options, and reviews them (accept or cancel, one at a time or
all). Mark Text › Revision Insertion marks text as inserted. Tools › Doc
Compare… compares a document with another file into revision marks.

**Map.** `[revisions]` and its options string, the marks in the text
(`<:V…>`?), the review dialog, and Doc Compare's matching.

**Decisions to expect.** Recommend reviewing in the rail (Next, Accept,
Reject), with author and date if SAM can store them.

**Better ways and logical additions to offer.** Show or hide the marks,
and a change list in the rail.
