---
id: merge-messages
title: Merge and envelope messages
area: reference
keywords: error, no record to merge, choose a data file, no address found, field names, merge messages
see_also: merge, envelopes
---
These are the messages shown during a merge, and what each one asks of you.

- **Choose a data file first to see its fields. / Choose a data file in step 1 first.** The panel works from the data file. Pick or create one under Data file.
- **No record to merge.** The data file has no records, or the conditions exclude all of them. Open Edit Records to add records, or loosen the conditions under With conditions.
- **Type the field names first, separated by commas.** Making a new data file needs its field names, for example Title, First, Last.
- **Choose the field that names each PDF.** One PDF per record needs a field to name the files by.
- **No address found: type the recipient's.** The envelope found nothing to use as the address. Select the address in the document first, or type it in the panel.
- **Merge Envelopes takes the document's data file.** Choose a data file in File › Merge… first; the recipient is named as a field.
- **Viewing ended; nothing was printed.** You closed the preview before printing. The records to print are sent together at the end, so run the merge again.
- **Custom: the page is one label.** Not an error: the custom label size makes one label fill the page.
