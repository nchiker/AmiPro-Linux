//! The placeholder page estimate the status bar uses until the layout engine
//! replaces it (Phase 2.5c). It estimates lines from character counts.

use amipro_document::Document;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphBox {
    pub paragraph: usize,
    pub page: usize,
    pub bounds: Rect,
    pub estimated_lines: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub number: usize,
    pub bounds: Rect,
    pub paragraphs: Vec<ParagraphBox>,
}

pub fn paginate(document: &Document) -> Vec<Page> {
    let page_setup = document.page_setup();
    let [top, right, bottom, left] = page_setup.margin_points;
    let content_width = page_setup.width_points - left - right;
    let content_height = page_setup.height_points - top - bottom;
    let mut pages = vec![Page {
        number: 1,
        bounds: Rect {
            x: 0.0,
            y: 0.0,
            width: page_setup.width_points,
            height: page_setup.height_points,
        },
        paragraphs: vec![],
    }];
    let mut y = top;
    for (index, paragraph) in document.paragraphs().iter().enumerate() {
        let style = &document.styles()[&paragraph.style];
        let average_advance = style.font_size_points * 0.52;
        let chars_per_line = (content_width / average_advance).floor().max(1.0) as usize;
        let line_count = paragraph
            .text
            .chars()
            .count()
            .max(1)
            .div_ceil(chars_per_line);
        let height =
            line_count as f32 * style.font_size_points * 1.2 + style.spacing.below as f32 / 20.0;
        if y > top && y + height > top + content_height {
            let number = pages.len() + 1;
            pages.push(Page {
                number,
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: page_setup.width_points,
                    height: page_setup.height_points,
                },
                paragraphs: vec![],
            });
            y = top;
        }
        let page = pages.len() - 1;
        pages[page].paragraphs.push(ParagraphBox {
            paragraph: index,
            page,
            bounds: Rect {
                x: left,
                y,
                width: content_width,
                height,
            },
            estimated_lines: line_count,
        });
        y += height;
    }
    pages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_document_paginates_deterministically() {
        let document = Document::with_paragraphs((0..80).map(|_| {
            (
                "A paragraph with enough text to consume vertical space.",
                amipro_document::StyleId(1),
            )
        }));
        let first = paginate(&document);
        assert!(first.len() > 1);
        assert_eq!(first, paginate(&document));
        assert_eq!(first[1].number, 2);
    }
}
