//! Ami Pro's bookmarks, as it stores them (decompile segment `1580`; VM
//! fixtures RNB1–RNB3; `docs/research/NOTES_BOOKMARKS_FOOTNOTES.md`,
//! "Bookmarks"), and the typed view over them (the notes run's NK1):
//!
//! - **text**: a pair of zero-width marks, `<:Zname>` where the bookmark's
//!   text starts and `<:Z~name>` where it ends, in any text;
//! - **cell range**: one mark `<:Zrows,columns;name>` at the start of the
//!   top-left cell's text, with no end mark;
//! - **one cell**: a `[book]` line `name table row column`, the table named
//!   by its frame's sequence number (field 13);
//! - **frame**: the frame's name (`[frmname]`).
//!
//! Names compare without regard to case; the marks stay where Ami Pro put
//! them, so they move with the text and go when it is deleted.

use crate::{Document, FlowId, FrameContent, FrameId, LegacyMark};

/// The kept section that holds one-cell bookmarks.
pub const BOOK_KEY: &str = "section [book]";

/// The most characters a name may have (the Bookmarks dialog reads 19).
pub const NAME_LIMIT: usize = 19;

/// A bookmark mark as it stands in text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookmarkMark {
    pub name: String,
    /// The end of a text bookmark (`<:Z~name>`).
    pub end: bool,
    /// A cell range's rows and columns (`<:Zrows,columns;name>`).
    pub cells: Option<(u16, u16)>,
}

impl BookmarkMark {
    /// Reads a mark Ami Pro wrote (`1580:0629`, `067f`, `11ed`).
    pub fn parse(raw: &str) -> Option<Self> {
        let body = raw.strip_prefix("<:Z")?.strip_suffix('>')?;
        if let Some(name) = body.strip_prefix('~') {
            return Some(Self {
                name: name.to_owned(),
                end: true,
                cells: None,
            });
        }
        // A comma before a semicolon: rows and columns before the name.
        if let Some((counts, name)) = body.split_once(';')
            && let Some((rows, columns)) = counts.split_once(',')
            && let (Ok(rows), Ok(columns)) = (rows.parse(), columns.parse())
        {
            return Some(Self {
                name: name.to_owned(),
                end: false,
                cells: Some((rows, columns)),
            });
        }
        Some(Self {
            name: body.to_owned(),
            end: false,
            cells: None,
        })
    }

    /// The mark as Ami Pro writes it (`1580:06ca`).
    pub fn raw(&self) -> String {
        match (self.end, self.cells) {
            (true, _) => format!("<:Z~{}>", self.name),
            (false, Some((rows, columns))) => format!("<:Z{rows},{columns};{}>", self.name),
            (false, None) => format!("<:Z{}>", self.name),
        }
    }

    /// A kept mark at `byte`.
    pub fn mark(&self, byte: usize) -> LegacyMark {
        LegacyMark {
            byte,
            raw: self.raw(),
        }
    }
}

/// A one-cell bookmark (`[book]`): the table's sequence number, the row and
/// column from 0.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CellBookmark {
    pub name: String,
    pub table: u32,
    pub row: u32,
    pub column: u32,
}

/// The document's one-cell bookmarks, in their order (`1580:145c`: a line
/// split at its first three spaces).
pub fn cell_bookmarks(document: &Document) -> Vec<CellBookmark> {
    let Some(raw) = document.legacy().get(BOOK_KEY) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.trim().splitn(4, ' ');
            let name = parts.next()?.to_owned();
            let number = |part: Option<&str>| part?.trim().parse().ok();
            Some(CellBookmark {
                name,
                table: number(parts.next())?,
                row: number(parts.next())?,
                column: number(parts.next())?,
            })
        })
        .filter(|bookmark| !bookmark.name.is_empty())
        .collect()
}

/// Replaces the document's one-cell bookmarks; none removes `[book]`.
pub fn set_cell_bookmarks(document: &mut Document, bookmarks: &[CellBookmark]) {
    let value = bookmarks
        .iter()
        .map(|bookmark| {
            // As the importer keeps a section's lines: tab-indented.
            format!(
                "\t{} {} {} {}",
                bookmark.name, bookmark.table, bookmark.row, bookmark.column
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let legacy = document.legacy_mut();
    match legacy.values.iter_mut().find(|kept| kept.key == BOOK_KEY) {
        Some(kept) if value.is_empty() => kept.value.clear(),
        Some(kept) => kept.value = value,
        None if value.is_empty() => {}
        None => legacy.push(BOOK_KEY, value),
    }
}

/// A frame's sequence number (field 13), which `[book]` names its table by.
pub fn frame_sequence(document: &Document, frame: FrameId) -> Option<u32> {
    document
        .frame(frame)?
        .legacy
        .get("frame.field13")?
        .trim()
        .parse()
        .ok()
        .filter(|number| *number > 0)
}

/// The table frame with sequence number `table`.
pub fn table_with_sequence(document: &Document, table: u32) -> Option<FrameId> {
    document
        .frames()
        .find(|frame| {
            matches!(frame.content, FrameContent::Table(_))
                && frame_sequence(document, frame.id) == Some(table)
        })
        .map(|frame| frame.id)
}

/// A place in a flow.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MarkPlace {
    pub flow: FlowId,
    pub paragraph: usize,
    pub byte: usize,
}

/// Where a bookmark is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookmarkTarget {
    /// The text between the marks (the same place for a position); `end` is
    /// `None` when the end mark is missing.
    Text {
        start: MarkPlace,
        end: Option<MarkPlace>,
    },
    /// A block of cells from the mark's cell; `table` is `None` when the
    /// mark is not in a table's cell.
    Cells {
        mark: MarkPlace,
        table: Option<(FrameId, usize)>,
        rows: u16,
        columns: u16,
    },
    /// One cell; `table` is `None` when no table has the sequence number
    /// (Ami Pro removes such a bookmark when it is gone to).
    Cell {
        table: Option<FrameId>,
        row: u32,
        column: u32,
    },
    Frame(FrameId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bookmark {
    pub name: String,
    pub target: BookmarkTarget,
}

impl Bookmark {
    /// "text", "cells", "cell", or "frame".
    pub fn kind(&self) -> &'static str {
        match self.target {
            BookmarkTarget::Text { .. } => "text",
            BookmarkTarget::Cells { .. } => "cells",
            BookmarkTarget::Cell { .. } => "cell",
            BookmarkTarget::Frame(_) => "frame",
        }
    }
}

/// The table and cell index whose text is `flow`, if any.
pub fn cell_of_flow(document: &Document, flow: FlowId) -> Option<(FrameId, usize)> {
    document.frames().find_map(|frame| match &frame.content {
        FrameContent::Table(table) => table
            .cells
            .iter()
            .position(|cell| match cell.content {
                crate::CellContent::Text(cell_flow) => cell_flow == flow,
            })
            .map(|index| (frame.id, index)),
        _ => None,
    })
}

/// Every bookmark in the document, sorted by name without regard to case
/// (the Bookmarks dialog's sorted list).
pub fn bookmarks(document: &Document) -> Vec<Bookmark> {
    let mut found: Vec<Bookmark> = Vec::new();
    let mut ends: Vec<(String, MarkPlace)> = Vec::new();
    for (flow, contents) in document.flows() {
        for (paragraph, text) in contents.paragraphs.iter().enumerate() {
            for mark in &text.marks {
                let Some(parsed) = BookmarkMark::parse(&mark.raw) else {
                    continue;
                };
                let place = MarkPlace {
                    flow,
                    paragraph,
                    byte: mark.byte,
                };
                if parsed.end {
                    ends.push((parsed.name.to_lowercase(), place));
                } else if let Some((rows, columns)) = parsed.cells {
                    found.push(Bookmark {
                        name: parsed.name,
                        target: BookmarkTarget::Cells {
                            mark: place,
                            table: cell_of_flow(document, flow),
                            rows,
                            columns,
                        },
                    });
                } else {
                    found.push(Bookmark {
                        name: parsed.name,
                        target: BookmarkTarget::Text {
                            start: place,
                            end: None,
                        },
                    });
                }
            }
        }
    }
    for bookmark in &mut found {
        if let BookmarkTarget::Text { start, end } = &mut bookmark.target {
            let name = bookmark.name.to_lowercase();
            *end = ends
                .iter()
                .find(|(other, place)| *other == name && place.flow == start.flow)
                .map(|(_, place)| *place);
        }
    }
    for cell in cell_bookmarks(document) {
        found.push(Bookmark {
            target: BookmarkTarget::Cell {
                table: table_with_sequence(document, cell.table),
                row: cell.row,
                column: cell.column,
            },
            name: cell.name,
        });
    }
    for frame in document.frames() {
        if let Some(name) = frame.style.name.as_ref().filter(|name| !name.is_empty()) {
            found.push(Bookmark {
                name: name.clone(),
                target: BookmarkTarget::Frame(frame.id),
            });
        }
    }
    found.sort_by_key(|bookmark| bookmark.name.to_lowercase());
    found
}

/// The bookmark named `name`, whatever its case.
pub fn bookmark_named(document: &Document, name: &str) -> Option<Bookmark> {
    let name = name.to_lowercase();
    bookmarks(document)
        .into_iter()
        .find(|bookmark| bookmark.name.to_lowercase() == name)
}

/// A bookmark's text as paragraphs (what the Bookmark and GetBookMark
/// fields and Include of a bookmark show): the text between its marks, its
/// cells' text row by row (cells apart by tabs), or its frame's text.
pub fn bookmark_text(document: &Document, bookmark: &Bookmark) -> Vec<String> {
    let clean = |text: &str| text.replace(crate::OBJECT_REPLACEMENT_CHARACTER, "");
    let flow_text = |flow: FlowId| -> Vec<String> {
        document.flow(flow).map_or_else(Vec::new, |flow| {
            flow.paragraphs
                .iter()
                .map(|paragraph| clean(&paragraph.text))
                .collect()
        })
    };
    match &bookmark.target {
        BookmarkTarget::Text { start, end } => {
            let Some(end) = end else {
                return Vec::new();
            };
            let Some(flow) = document.flow(start.flow) else {
                return Vec::new();
            };
            (start.paragraph..=end.paragraph)
                .filter_map(|index| {
                    let text = &flow.paragraphs.get(index)?.text;
                    let from = if index == start.paragraph {
                        start.byte
                    } else {
                        0
                    };
                    let to = if index == end.paragraph {
                        end.byte
                    } else {
                        text.len()
                    };
                    text.get(from.min(to)..to).map(clean)
                })
                .collect()
        }
        BookmarkTarget::Cells {
            table: Some((frame, cell)),
            rows,
            columns,
            ..
        } => {
            let Some(FrameContent::Table(table)) =
                document.frame(*frame).map(|frame| &frame.content)
            else {
                return Vec::new();
            };
            let width = table.columns.len().max(1);
            let (row, column) = (cell / width, cell % width);
            (row..row + usize::from(*rows))
                .map(|row| {
                    (column..(column + usize::from(*columns)).min(width))
                        .filter_map(|column| table.cells.get(row * width + column))
                        .map(|cell| match cell.content {
                            crate::CellContent::Text(flow) => flow_text(flow).join(" "),
                        })
                        .collect::<Vec<_>>()
                        .join("\t")
                })
                .collect()
        }
        BookmarkTarget::Cell {
            table: Some(frame),
            row,
            column,
        } => {
            let Some(FrameContent::Table(table)) =
                document.frame(*frame).map(|frame| &frame.content)
            else {
                return Vec::new();
            };
            let index = *row as usize * table.columns.len() + *column as usize;
            table
                .cells
                .get(index)
                .map_or_else(Vec::new, |cell| match cell.content {
                    crate::CellContent::Text(flow) => flow_text(flow),
                })
        }
        BookmarkTarget::Frame(frame) => match document.frame(*frame).map(|frame| &frame.content) {
            Some(FrameContent::Text(flow)) => flow_text(*flow),
            _ => Vec::new(),
        },
        BookmarkTarget::Cells { .. } | BookmarkTarget::Cell { .. } => Vec::new(),
    }
}

/// Why a name cannot be a bookmark's (NR7: Ami Pro's rules).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NameProblem {
    Empty,
    Space,
    TooLong,
    StartsWithDigit,
    Taken,
}

impl NameProblem {
    pub fn message(self) -> &'static str {
        match self {
            Self::Empty => "Type a name for the bookmark.",
            Self::Space => "A bookmark's name cannot hold a space.",
            Self::TooLong => "A bookmark's name has at most 19 characters.",
            Self::StartsWithDigit => "A bookmark's name cannot start with a digit.",
            Self::Taken => "There is a bookmark with this name.",
        }
    }
}

/// Checks `name` as a new bookmark's: Ami Pro's rules (no space, at most
/// 19 characters, no leading digit) and not the name of another bookmark
/// in any case. `name` is trimmed first, as Ami Pro trims it.
pub fn check_name(document: &Document, name: &str) -> Result<String, NameProblem> {
    let name = name.trim();
    if name.is_empty() {
        return Err(NameProblem::Empty);
    }
    if name.chars().any(char::is_whitespace) {
        return Err(NameProblem::Space);
    }
    if name.chars().count() > NAME_LIMIT {
        return Err(NameProblem::TooLong);
    }
    if name.starts_with(|character: char| character.is_ascii_digit()) {
        return Err(NameProblem::StartsWithDigit);
    }
    if bookmark_named(document, name).is_some() {
        return Err(NameProblem::Taken);
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StyleId;

    #[test]
    fn marks_read_and_write_as_ami_pro_rnb1_rnb3() {
        for raw in ["<:Zbmone>", "<:Z~bmone>", "<:Z2,3;cellbm>"] {
            assert_eq!(BookmarkMark::parse(raw).unwrap().raw(), raw);
        }
        let cells = BookmarkMark::parse("<:Z2,3;cellbm>").unwrap();
        assert_eq!((cells.name.as_str(), cells.cells), ("cellbm", Some((2, 3))));
        assert!(BookmarkMark::parse("<:Z~x>").unwrap().end);
        assert_eq!(BookmarkMark::parse("<:X8,1;Index>"), None);
    }

    #[test]
    fn text_bookmarks_pair_their_marks_and_sort_by_name() {
        let mut document = Document::with_paragraphs([("Alpha beta gamma.", StyleId(1))]);
        document.paragraph_mut(0).marks = vec![
            BookmarkMark::parse("<:Zbmone>").unwrap().mark(6),
            BookmarkMark::parse("<:Z~BMONE>").unwrap().mark(11),
            BookmarkMark::parse("<:ZAlpha>").unwrap().mark(0),
            BookmarkMark::parse("<:Z~Alpha>").unwrap().mark(0),
        ];
        let found = bookmarks(&document);
        assert_eq!(
            found.iter().map(|b| b.name.as_str()).collect::<Vec<_>>(),
            ["Alpha", "bmone"]
        );
        let BookmarkTarget::Text { start, end } = &found[1].target else {
            panic!("a text bookmark");
        };
        assert_eq!((start.byte, end.map(|end| end.byte)), (6, Some(11)));
        assert_eq!(check_name(&document, " BMone "), Err(NameProblem::Taken));
        assert_eq!(check_name(&document, "two words"), Err(NameProblem::Space));
        assert_eq!(
            check_name(&document, "1abc"),
            Err(NameProblem::StartsWithDigit)
        );
        assert_eq!(
            check_name(&document, &"x".repeat(20)),
            Err(NameProblem::TooLong)
        );
        assert_eq!(check_name(&document, " fine_name "), Ok("fine_name".into()));
    }

    #[test]
    fn one_cell_bookmarks_are_book_lines() {
        let mut document = Document::with_paragraphs([("x", StyleId(1))]);
        let wanted = vec![CellBookmark {
            name: "incell".into(),
            table: 3,
            row: 2,
            column: 0,
        }];
        set_cell_bookmarks(&mut document, &wanted);
        assert_eq!(document.legacy().get(BOOK_KEY), Some("\tincell 3 2 0"));
        assert_eq!(cell_bookmarks(&document), wanted);
        let found = bookmarks(&document);
        assert_eq!(found[0].kind(), "cell");
        set_cell_bookmarks(&mut document, &[]);
        assert!(cell_bookmarks(&document).is_empty());
    }
}
