# Merge and Templates Playbook

Part of `docs/PHASE5_PLAYBOOK.md` (queue rows 14 and 15). Follow its
recipe and standing rules; this file says what each run covers, what to
map, and what to expect.

**Why one playbook.** Both runs produce new documents. A template is a
style sheet with contents that File › New starts a document from. A merge
fills a document's merge fields from each record of a data file. Envelopes
and labels are merges onto special page layouts. All of them need fields
(Fields and references, `fields`). Merging to the printer and printing an
envelope need printing (Output, `print-pdf`).

**State today.**
- **Style sheets:** read and written with or without contents (Save as a
  Style Sheet). Use Another Style Sheet and the style-sheet folders (a
  preference) exist.
- **File › New:** makes a new document whose Body Text follows
  `_DEFAULT.STY`.
- **Merge:** built in run 14 (2026-11-03, `docs/MERGE_PROGRESS.md`):
  data files of every kind, the merge, the Merge panel, every output, the
  record editor, labels, envelopes, Start from in place of File › New's
  merge sheets. `MM-01` is IMPLEMENTED for merge; templates remain.

**Evidence to start from.**
- **Guide:**
  - ch. 4: a new document from a style sheet, and "Run macro" with
    personal, default, and optional information;
  - ch. 9: style sheets and their contents;
  - ch. 13: printing envelopes and their options and styles;
  - ch. 28: merge.
    - Data files: making them, delimiters, viewing, editing, and sorting
      records, and changing fields.
    - The merge document.
    - Merging to the printer, to the screen, or to a file.
    - Conditions.
    - Envelopes, Avery and custom labels, several records on a page.
    - A table or an external delimited file as the data file.
- **Commands:** File › New… (0x65), File › Merge… (0x30d), File › Print
  Envelope… (0xed), Edit › Insert › Merge Field… (0xb6).
- **Storage:**
  - `[recfile]`: the merge data file, as two strings and six numbers
    (named in 27 user and 6 ISO files).
  - `[fldnames]`: eight field names, `Field1`…`Field8` by default.
    Whether these are merge or Doc Info names is to map.
  - `<:M name>` … `<:M>`: a merge field (16 in one user file).
  - The style sheets' contents and descriptions: 192 user and 63 ISO
    style sheets.

## Run 1: `merge`, queue 14

**Goal.** File › Merge… as Ami Pro has it:
- choose or make a data file, with its delimiters and field names;
- insert merge fields;
- merge to new documents, to the printer, or to a file, with conditions;
- envelopes and labels, several records on a page;
- a table, or an external delimited file, as the data file.

File › Print Envelope… prints one envelope from the selected address.

**Map.**
- The data file's format (an Ami Pro document with field and record
  delimiters) and `[recfile]`'s values.
- The Merge dialog's steps, the conditions' syntax, and how merge fields
  print when there is no data.
- The label definitions (sizes and layouts).
- The envelope dialog, its styles, and its page layout.

**VM fixtures.** A data file, a merge document, and a conditional merge,
each merged in Ami Pro to a file and compared with the app's result.

**Decisions to expect.**
- **The merge workflow.** Recommend a Merge panel in the rail: the data
  file, its fields to insert, a record browser that previews the
  document with each record live, and Merge to New Document, PDF, or
  Printer.
- **Data sources.** Recommend CSV and the user's address book (if the
  desktop offers one) beside Ami Pro's data files and tables.
- **Labels.** Recommend Ami Pro's label sheets, plus a custom size.

**Better ways and logical additions to offer.**
- Preview with a record, by moving through the records.
- Merging to one PDF per record.
- Skipping empty lines when a field is empty.
- Filtering records in the panel.

**Exit.** Merges equal to Ami Pro's merged output (text and page breaks);
twins of a data file and a merge document; the user's physical check.

## Run 2: `templates`, queue 15 (small)

**Goal.** File › New… lists the style sheets in the style-sheet folders,
by file name or by description, with a preview. With contents starts the
document from the sheet's contents, and the document links to the sheet
as Ami Pro's do.

**Map.** The New dialog (`AMIPROUI.DLL`): its list, the descriptions'
source, the preview, With contents, and Run macro. Also which shipped
sheets run a macro, and what that macro asks for (personal information,
the date, addresses).

**Decisions to expect.**
- **Where the chooser lives.** Recommend a template chooser in the New
  tab's place, with previews drawn by the page surface.
- **Sheets that run a macro.** Macros are deferred (`MA-01`). Recommend
  opening them with contents and no macro, and saying so; or replacing
  the common macro prompts (name, address, date) with fill-in fields from
  the fields run (an improvement), for the shipped sheets the corpus uses.
- **Saving a template.** Whether the user's own template is just Save as
  a Style Sheet with contents (recommend yes, as Ami Pro).

**Better ways and logical additions to offer.** Recent templates first, a
template made from the current document in one step, and a preview at
true page size.

**Exit.** Every corpus style sheet listed and previewed without failure;
a document made from a sheet with contents equal to Ami Pro's (VM); the
user's physical check.
