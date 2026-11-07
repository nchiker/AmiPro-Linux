//! The plain-text filter (Ami Pro's ASCII): the main text in order, each
//! paragraph a line with its number or bullet, a frame's text and a table
//! after the paragraph that anchors it, cells separated by tabs. UTF-8 with
//! line feeds (EC1). Headers and footers are left out, as Ami Pro's ASCII
//! export leaves them.

use crate::{Block, ExchangeDocument, Inline, Paragraph, ShapeContent, Table};

/// The document as plain text.
pub fn write(document: &ExchangeDocument) -> Vec<u8> {
    let mut out = String::new();
    for section in &document.sections {
        blocks(&section.blocks, &mut out);
    }
    out.into_bytes()
}

fn blocks(blocks: &[Block], out: &mut String) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => paragraph_text(paragraph, out),
            Block::Table(table) => table_text(table, out),
        }
    }
}

fn paragraph_text(paragraph: &Paragraph, out: &mut String) {
    if let Some(number) = &paragraph.number_text {
        out.push_str(number);
        out.push('\t');
    }
    let mut frames = Vec::new();
    for inline in &paragraph.inlines {
        inline_text(inline, out, &mut frames);
    }
    out.push('\n');
    for frame in frames {
        match frame {
            ShapeContent::Text { blocks: inner, .. } => {
                out.push('\n');
                blocks(&inner, out);
                out.push('\n');
            }
            ShapeContent::Picture(_) | ShapeContent::Empty => {}
        }
    }
}

fn inline_text(inline: &Inline, out: &mut String, frames: &mut Vec<ShapeContent>) {
    match inline {
        Inline::Text { text, .. } => {
            out.extend(text.chars().filter(|character| *character != '\u{AD}'));
        }
        Inline::Tab { .. } => out.push('\t'),
        Inline::LineBreak | Inline::PageBreak | Inline::ColumnBreak => out.push('\n'),
        Inline::Field(field) => out.push_str(&field.result),
        Inline::Symbol { character, .. } => out.push(*character),
        Inline::Shape(shape) => frames.push(shape.content.clone()),
        Inline::Note(_) | Inline::Footnote(_) | Inline::Marker(_) | Inline::Bookmark { .. } => {}
    }
}

/// A row a line, each cell's paragraphs joined by spaces.
fn table_text(table: &Table, out: &mut String) {
    for row in &table.rows {
        let cells: Vec<String> = row
            .cells
            .iter()
            .map(|cell| {
                let mut text = String::new();
                for block in &cell.blocks {
                    let mut part = String::new();
                    match block {
                        Block::Paragraph(paragraph) => {
                            let mut frames = Vec::new();
                            for inline in &paragraph.inlines {
                                inline_text(inline, &mut part, &mut frames);
                            }
                        }
                        Block::Table(inner) => table_text(inner, &mut part),
                    }
                    let part = part.replace(['\n', '\t'], " ");
                    let part = part.trim();
                    if !part.is_empty() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(part);
                    }
                }
                text
            })
            .collect();
        out.push_str(&cells.join("\t"));
        out.push('\n');
    }
}
