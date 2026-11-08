---
id: ole-objects
title: OLE objects
area: objects
keywords: ole, object linking and embedding, embedded object, linked object, dde, wordart, server, new object, convert to picture, convert object
see_also: links-panel, paste-special, pictures, known-limitations
---
Ami Pro documents can hold OLE objects: a chart from a spreadsheet, a piece of WordArt, a linked file. This program cannot run the programs that made them (there are no OLE servers on Linux), so it **keeps and draws** them instead of editing them.

## What you see

An OLE object sits in a picture frame and is drawn from the picture stored with it (a metafile). If there is no readable picture, Ami Pro's stored screen image or a labelled box is shown. Saving writes the object back unchanged.

Select the frame and the rail's **Object** section lists, read-only:

- **Class**, such as the name of the server program;
- **Kind**: embedded in the document, linked to a file, or a static picture;
- for a linked object, its **Topic** and **Item**;
- **Native data**: how large the object's own data is;
- **Presentation**: the stored picture and its size in inches.

Pressing Enter or double-clicking an OLE frame does not start a program. The status bar explains that this program keeps and draws the object.

## Convert Object to Picture

If the object has a readable stored picture, the Object section offers **Convert Object to Picture**. It turns the object into an ordinary metafile picture that you can scale, move and export like any other ([Pictures](pictures)). The object's own data is dropped. It is one undo step, so Undo brings the object back.

## Not offered

Update, Unlink, Edit, Paste Link and Insert › New Object… are not available. See [Known limitations](known-limitations). The list of what a document holds is [Links](links-panel).
