//! A table cell's box: where Ami Pro draws its lines and fill (Table ›
//! Lines & Color; VM fixtures TL1, TL4). A cell's box runs from the middle
//! of the gutter before it to the middle of the gutter after it, and the
//! first row's and column's from the middle of the table's inset; lines
//! are drawn centred on the box's edges, and the fill covers the box.

use amipro_document::{CellLines, Table, rule_strokes};

use crate::engine::{Bounds, TablePart};

/// Where cell `index`'s lines and fill go, from the box the layout gave
/// it (`FrameBoxKind::Table`'s cells: from the middle of the gutter before
/// it, or the table's edge, to the middle of the gutter after it) in `part`
/// of the table: Ami Pro's first row and column start in the middle of the
/// table's inset instead (a part's first row too, as every part has the
/// inset).
pub fn cell_box(table: &Table, part: &TablePart, index: usize, laid_out: Bounds) -> Bounds {
    let columns = table.columns.len().max(1);
    let (row, column) = (index / columns, index % columns);
    let mut boxed = laid_out;
    if column == 0 {
        let half = table.spacing.left as f32 / 2.0;
        boxed.x += half;
        boxed.width -= half;
    }
    if row == 0 || row == part.rows.start {
        let half = table.spacing.top as f32 / 2.0;
        boxed.y += half;
        boxed.height -= half;
    }
    boxed
}

/// The strokes of a cell's lines on its box: (bounds, as filled rectangles)
/// for each side with a line, each line style's strokes centred on the
/// edge (`rule_strokes`: double and triple lines as several).
pub fn cell_line_strokes(cell_box: Bounds, lines: CellLines) -> Vec<Bounds> {
    let mut out = Vec::new();
    let mut side = |style: u8, horizontal: bool, at: f32| {
        let strokes = rule_strokes(style);
        let thickness: i32 = strokes.iter().sum();
        let mut offset = at - thickness as f32 / 2.0;
        for (index, part) in strokes.iter().enumerate() {
            if index % 2 == 0 {
                out.push(if horizontal {
                    Bounds {
                        x: cell_box.x,
                        y: offset,
                        width: cell_box.width,
                        height: *part as f32,
                    }
                } else {
                    Bounds {
                        x: offset,
                        y: cell_box.y,
                        width: *part as f32,
                        height: cell_box.height,
                    }
                });
            }
            offset += *part as f32;
        }
    };
    side(lines.top, true, cell_box.y);
    side(lines.bottom, true, cell_box.bottom());
    side(lines.left, false, cell_box.x);
    side(lines.right, false, cell_box.right());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use amipro_document::{CellMargins, CellSpan, TableRows, TableSpacing};

    fn table() -> Table {
        Table {
            columns: vec![4569, 4569],
            rows: vec![284, 284],
            cell_margins: CellMargins::default(),
            spacing: TableSpacing::default(),
            row_options: TableRows::default(),
            layout: Default::default(),
            read_rows: Default::default(),
            cells: Vec::new(),
            legacy: Default::default(),
        }
    }

    #[test]
    fn a_cells_box_starts_in_the_middle_of_the_inset() {
        // VM TL1 on paper: the table at 1,440 × 2,291, the left line at
        // 1,461, the top one at 2,312; the layout's box for the first cell
        // starts at the table's edge.
        let table = table();
        let part = TablePart {
            rows: 0..2,
            ..TablePart::default()
        };
        let laid_out = Bounds {
            x: 1_440.0,
            y: 2_291.0,
            width: 4_655.0,
            height: 370.0,
        };
        let boxed = cell_box(&table, &part, 0, laid_out);
        assert_eq!((boxed.x, boxed.y), (1_461.5, 2_312.5));
        assert_eq!(boxed.right(), 6_095.0);
        // Other cells keep the layout's box.
        assert_eq!(cell_box(&table, &part, 3, laid_out), laid_out);
    }

    #[test]
    fn lines_are_centred_on_the_box_edges() {
        let boxed = Bounds {
            x: 100.0,
            y: 200.0,
            width: 1_000.0,
            height: 300.0,
        };
        let strokes = cell_line_strokes(
            boxed,
            CellLines {
                top: 1,
                left: 8,
                ..CellLines::default()
            },
        );
        // Top: one 10-twip stroke centred on y = 200.
        assert_eq!(strokes[0].y, 195.0);
        assert_eq!(strokes[0].height, 10.0);
        // Left: a double line, two 20-twip strokes 20 apart, centred on 100.
        assert_eq!(strokes.len(), 3);
        assert_eq!((strokes[1].x, strokes[2].x), (70.0, 110.0));
        let _ = CellSpan::SINGLE;
    }
}
