//! Outline › Move Up and Move Down (Alt+Up, Alt+Down; `UI_SPEC.md` §40,
//! decision OM3): the caret's heading moves with its section, a body
//! paragraph alone, as one undo step, in Outline and Layout mode alike.
//!
//! A paragraph's level is its style's outline level (`Bullets::level`, 0
//! for body text). A heading's **section** is the heading and every
//! paragraph after it up to the next heading of its level or a higher one
//! (a smaller number), or the end of the flow; body text (level 0) is a
//! section of one paragraph. Frames anchored in the moved paragraphs go
//! with them, as their anchors are in the text.
//!
//! What a section moves over:
//! - **up**, a heading at level L: the previous section at level L when
//!   one comes before the next higher heading; otherwise the last piece of
//!   that higher heading's text before it (a body paragraph, or a deeper
//!   heading's section); right under the higher heading, the higher
//!   heading itself, so the section leaves it;
//! - **down**: the next section at level L, or, at the end of its parent's
//!   text, the next higher heading alone, so the section becomes that
//!   heading's first;
//! - body text moves over one paragraph, as Ami Pro moves it.
//!
//! Ami Pro moves an expanded heading alone and a contracted one with its
//! hidden text (VM OM1); the decision moves the section always (RB1).

use std::ops::Range;
use std::sync::Arc;

use amipro_document::{Document, FlowId, Paragraph};

use crate::{Applied, ChangeReport, EditError, TextPosition};

/// The outline level of each paragraph of `flow` (0 for body text).
pub fn levels(document: &Document, flow: FlowId) -> Vec<u8> {
    let Some(flow) = document.flow(flow) else {
        return Vec::new();
    };
    flow.paragraphs
        .iter()
        .map(|paragraph| {
            document
                .styles()
                .get(&paragraph.style)
                .map_or(0, |style| style.bullets.level)
        })
        .collect()
}

/// The section of paragraph `index`: the heading and what follows up to
/// the next heading of its level or higher; a body paragraph alone.
pub fn section(levels: &[u8], index: usize) -> Range<usize> {
    let level = levels[index];
    if level == 0 {
        return index..index + 1;
    }
    let end = levels[index + 1..]
        .iter()
        .position(|&next| next != 0 && next <= level)
        .map_or(levels.len(), |offset| index + 1 + offset);
    index..end
}

/// The pieces of `range` at the level under a heading at `level`: body
/// paragraphs one by one, deeper headings with their sections.
fn pieces(levels: &[u8], range: Range<usize>, level: u8) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut at = range.start;
    while at < range.end {
        let piece = if levels[at] == 0 || levels[at] <= level {
            at..at + 1
        } else {
            let own = section(levels, at);
            at..own.end.min(range.end)
        };
        at = piece.end;
        out.push(piece);
    }
    out
}

/// What the section at `index` moves over, up or down; `None` at the
/// flow's start or end.
pub fn neighbour(levels: &[u8], index: usize, down: bool) -> Option<Range<usize>> {
    let own = section(levels, index);
    let level = levels[index];
    if level == 0 {
        return if down {
            (own.end < levels.len()).then(|| own.end..own.end + 1)
        } else {
            index.checked_sub(1).map(|above| above..index)
        };
    }
    if down {
        let next = own.end;
        if next >= levels.len() {
            return None;
        }
        return Some(if levels[next] == level {
            section(levels, next)
        } else {
            next..next + 1
        });
    }
    if index == 0 {
        return None;
    }
    // The nearest heading above at this level or higher.
    let heading = levels[..index]
        .iter()
        .rposition(|&above| above != 0 && above <= level);
    match heading {
        Some(heading) if levels[heading] == level => Some(heading..index),
        Some(parent) => {
            let lead = pieces(levels, parent + 1..index, level);
            Some(lead.last().cloned().unwrap_or(parent..parent + 1))
        }
        None => pieces(levels, 0..index, level).last().cloned(),
    }
}

/// The result of a move: the paragraphs in `range` changed, the caret at
/// the start of `caret_paragraph`.
fn changed_range(
    document: Document,
    flow: FlowId,
    range: std::ops::Range<usize>,
    caret_paragraph: usize,
) -> Applied {
    let changed = document.flow(flow).map_or_else(Vec::new, |f| {
        f.paragraphs[range].iter().map(|p| p.id).collect()
    });
    let mut report = ChangeReport {
        changed: changed.into_iter().collect(),
        ..ChangeReport::default()
    };
    report.flows.insert(flow);
    Applied {
        caret: Some(TextPosition {
            flow,
            paragraph: caret_paragraph,
            byte: 0,
        }),
        document,
        report,
    }
}

/// Moves the section of paragraph `paragraph` of `flow` up or down, the
/// caret staying at its start.
pub(crate) fn move_section(
    document: &Document,
    flow: FlowId,
    paragraph: usize,
    down: bool,
) -> Result<Applied, EditError> {
    let levels = levels(document, flow);
    if paragraph >= levels.len() {
        return Err("no such paragraph");
    }
    let own = section(&levels, paragraph);
    let Some(other) = neighbour(&levels, paragraph, down) else {
        return Err(if down {
            "the section is already at the end"
        } else {
            "the section is already at the start"
        });
    };
    let mut next = document.clone();
    let paragraphs = &mut next.flow_mut(flow).ok_or("no such flow")?.paragraphs;
    let (first, second) = if down {
        (own.clone(), other)
    } else {
        (other, own.clone())
    };
    // The two adjacent ranges change places.
    let moved: Vec<Arc<Paragraph>> = paragraphs[second.clone()]
        .iter()
        .chain(&paragraphs[first.clone()])
        .cloned()
        .collect();
    let start = first.start;
    paragraphs.splice(first.start..second.end, moved);
    let caret_paragraph = if down {
        start + (second.end - second.start)
    } else {
        start
    };
    Ok(changed_range(
        next,
        flow,
        first.start..second.end,
        caret_paragraph,
    ))
}

/// The Outline navigator's drop (OM1): the section of paragraph
/// `paragraph` moved to stand before paragraph `to` (the flow's length: at
/// the end), as one undo step; a drop inside the section itself, or right
/// where it is, changes nothing and is refused.
pub(crate) fn move_section_to(
    document: &Document,
    flow: FlowId,
    paragraph: usize,
    to: usize,
) -> Result<Applied, EditError> {
    let levels = levels(document, flow);
    if paragraph >= levels.len() || to > levels.len() {
        return Err("no such paragraph");
    }
    let own = section(&levels, paragraph);
    if (own.start..=own.end).contains(&to) {
        return Err("the section is already there");
    }
    let mut next = document.clone();
    let paragraphs = &mut next.flow_mut(flow).ok_or("no such flow")?.paragraphs;
    let moved: Vec<Arc<Paragraph>> = paragraphs.drain(own.clone()).collect();
    let at = if to > own.end { to - own.len() } else { to };
    let count = moved.len();
    paragraphs.splice(at..at, moved);
    Ok(changed_range(
        next,
        flow,
        at.min(own.start)..(at + count).max(own.end),
        at,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Title 1, Subhead 2, body 0.
    const OUTLINE: [u8; 10] = [1, 2, 0, 0, 2, 0, 1, 2, 0, 3];

    #[test]
    fn a_section_runs_to_the_next_heading_of_its_level_or_higher() {
        assert_eq!(section(&OUTLINE, 0), 0..6);
        assert_eq!(section(&OUTLINE, 1), 1..4);
        assert_eq!(section(&OUTLINE, 4), 4..6);
        assert_eq!(section(&OUTLINE, 2), 2..3);
        assert_eq!(section(&OUTLINE, 7), 7..10);
        assert_eq!(section(&OUTLINE, 9), 9..10);
    }

    #[test]
    fn sections_move_over_their_siblings_out_of_and_into_parents() {
        // Subhead B up over Subhead A; Subhead A down over B.
        assert_eq!(neighbour(&OUTLINE, 4, false), Some(1..4));
        assert_eq!(neighbour(&OUTLINE, 1, true), Some(4..6));
        // Subhead B down: the next Title alone (B becomes its first).
        assert_eq!(neighbour(&OUTLINE, 4, true), Some(6..7));
        // Subhead C (first under the second Title) up: the Title alone.
        assert_eq!(neighbour(&OUTLINE, 7, false), Some(6..7));
        // The second Title up over the first's whole section.
        assert_eq!(neighbour(&OUTLINE, 6, false), Some(0..6));
        assert_eq!(neighbour(&OUTLINE, 6, true), None);
        assert_eq!(neighbour(&OUTLINE, 0, false), None);
        // Body text moves one paragraph at a time.
        assert_eq!(neighbour(&OUTLINE, 3, false), Some(2..3));
        assert_eq!(neighbour(&OUTLINE, 5, true), Some(6..7));
        // A level-3 heading at the end goes up over the body before it.
        assert_eq!(neighbour(&OUTLINE, 9, false), Some(8..9));
        // Under a parent with lead text: over the last piece of it.
        let lead = [1, 0, 3, 0, 2];
        assert_eq!(neighbour(&lead, 4, false), Some(2..4));
        // Before any heading: over the paragraph above.
        assert_eq!(neighbour(&[0, 0, 1], 2, false), Some(1..2));
    }
}
