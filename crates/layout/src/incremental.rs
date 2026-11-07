//! Incremental layout (D5): after an edit, lay out from the page before the
//! first changed paragraph and stop at the first page that starts as it did
//! before; the pages after it are reused. Shaped paragraphs are cached across
//! layouts. The result is always equal to a fresh [`layout_document`].
//!
//! [`layout_document`]: crate::layout_document

use std::sync::Arc;

use amipro_document::{
    CellContent, Document, FieldContext, FlowId, FrameContent, FrameId, InlineObject, Paragraph,
};

use crate::bullets::paragraph_numbers_carried;
use crate::engine::{
    Container, Convergence, DocumentLayout, Engine, Floating, MainStart, PageBox, PageStart,
    ShapeCache,
};
use crate::shaper::Shaper;

/// How the last layout was made.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LayoutKind {
    /// The whole document.
    #[default]
    Full,
    /// The main text from a restart page until it matched the previous
    /// layout (or to its end).
    Incremental,
    /// Only headers, footers, and frame interiors, over unchanged main text.
    Repaint,
    /// Nothing that affects layout changed.
    Unchanged,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LayoutStats {
    pub kind: LayoutKind,
    /// Pages whose main text was set again.
    pub pages_laid_out: usize,
    /// Pages taken from the previous layout.
    pub pages_reused: usize,
}

/// Keeps a document's layout up to date across edits.
#[derive(Default)]
pub struct IncrementalLayout {
    cache: ShapeCache,
    previous: Option<Previous>,
    layout: DocumentLayout,
    stats: LayoutStats,
    /// What dates and Doc Info entries show.
    context: FieldContext,
}

/// What the last layout leaves for the next one.
struct Previous {
    document: Document,
    /// The pages after the main phase, before headers, footers, and frame
    /// interiors.
    main_pages: Vec<Arc<PageBox>>,
    starts: Vec<Option<PageStart>>,
    floating: Vec<Floating>,
    run_starts: Vec<u32>,
    number_restarts: Vec<u32>,
}

impl IncrementalLayout {
    pub fn new() -> Self {
        Self::default()
    }

    /// Lays out `document`, reusing what it can from the previous call.
    pub fn layout(&mut self, document: &Document, shaper: &mut dyn Shaper) -> &DocumentLayout {
        match self.previous.take() {
            None => self.full(document, shaper),
            Some(previous) => self.update(document, shaper, previous),
        }
        self.layout.line_numbers = crate::line_numbers(document, &self.layout, shaper);
        &self.layout
    }

    /// Shows dates and Doc Info entries for `context` from the next layout:
    /// a document holding any is laid out whole again when it changes.
    pub fn set_field_context(&mut self, context: FieldContext) {
        if context == self.context {
            return;
        }
        if self
            .previous
            .as_ref()
            .is_some_and(|previous| previous.document.has_fields())
        {
            self.previous = None;
        }
        self.context = context;
    }

    pub fn field_context(&self) -> &FieldContext {
        &self.context
    }

    /// The last layout.
    pub fn current(&self) -> &DocumentLayout {
        &self.layout
    }

    pub fn stats(&self) -> LayoutStats {
        self.stats
    }

    pub fn cache(&self) -> &ShapeCache {
        &self.cache
    }

    /// Forgets everything, for example when fonts change.
    pub fn invalidate(&mut self) {
        let context = std::mem::take(&mut self.context);
        *self = Self {
            context,
            ..Self::default()
        };
    }

    fn full(&mut self, document: &Document, shaper: &mut dyn Shaper) {
        let mut engine = Engine::new(document, shaper, &mut self.cache, &self.context);
        engine.main_phase();
        let main_pages = engine.pages.clone();
        let all = 0..engine.pages.len();
        engine.headers_and_footers(all.clone());
        engine.frame_interiors(all);
        self.stats = LayoutStats {
            kind: LayoutKind::Full,
            pages_laid_out: engine.pages.len(),
            pages_reused: 0,
        };
        self.layout = DocumentLayout {
            pages: std::mem::take(&mut engine.pages),
            line_numbers: Vec::new(),
        };
        self.previous = Some(Previous {
            document: document.clone(),
            main_pages,
            starts: std::mem::take(&mut engine.starts),
            floating: std::mem::take(&mut engine.floating),
            run_starts: std::mem::take(&mut engine.run_starts),
            number_restarts: std::mem::take(&mut engine.number_restarts),
        });
    }

    fn update(&mut self, document: &Document, shaper: &mut dyn Shaper, previous: Previous) {
        let changes = document.layout_changes(&previous.document);
        // Footnotes share each page with the text that refers to them, so a
        // document with any is laid out whole (the notes run, NK2).
        if changes.structure || has_footnotes(document) || has_footnotes(&previous.document) {
            return self.full(document, shaper);
        }
        let old = previous.document.paragraphs();
        let new = document.paragraphs();
        let common = old.len().min(new.len());
        let mut first_changed = (0..common)
            .find(|index| !same(&old[*index], &new[*index]))
            .unwrap_or(common);
        let main_changed = first_changed < common || old.len() != new.len();
        let other = if changes.other_flows {
            other_flows(document, &previous.document)
        } else {
            OtherFlows::Unchanged
        };
        if other == OtherFlows::Mixed || (main_changed && changes.other_flows) {
            return self.full(document, shaper);
        }
        // The tables whose cells changed, and the last paragraph anchoring
        // one: a page holding a part of one is never reused as it was.
        let mut changed_tables = Vec::new();
        let mut last_anchor = None;
        if !main_changed {
            match other {
                // An anchored table is as tall as its text, so text after it
                // moves when its cells change: lay out again from its anchor.
                OtherFlows::Tables {
                    first,
                    last,
                    tables,
                } => {
                    first_changed = first;
                    last_anchor = Some(last);
                    changed_tables = tables;
                }
                OtherFlows::Other => return self.repaint(document, shaper, previous),
                OtherFlows::Unchanged | OtherFlows::Mixed => {
                    self.stats = LayoutStats {
                        kind: LayoutKind::Unchanged,
                        pages_laid_out: 0,
                        pages_reused: self.layout.pages.len(),
                    };
                    self.previous = Some(Previous {
                        document: document.clone(),
                        ..previous
                    });
                    return;
                }
            }
        }
        // The last page whose main text starts before the change, on a page
        // that a restart can make again; otherwise the first page.
        let found = (0..previous.starts.len()).rev().find(|page| {
            previous.starts[*page]
                .as_ref()
                .is_some_and(|start| start.paragraph_index < first_changed && start.fresh)
        });
        let restart = found.unwrap_or(0);
        // The unchanged paragraphs at the end, with the same numbers when
        // some style numbers its paragraphs (an edit can renumber the rest).
        let numbered = |document: &Document| {
            document
                .styles()
                .values()
                .any(|style| style.bullets.numbered())
                .then(|| {
                    paragraph_numbers_carried(
                        document.paragraphs(),
                        document.styles(),
                        document.contents().sequence.as_ref(),
                    )
                })
        };
        let numbers = numbered(&previous.document).zip(numbered(document));
        // The paragraphs anchoring changed tables count as changed.
        let room = common - last_anchor.map_or(first_changed, |last| (last + 1).max(first_changed));
        let unchanged_tail = (0..room)
            .take_while(|offset| {
                let (old_index, new_index) = (old.len() - 1 - offset, new.len() - 1 - offset);
                same(&old[old_index], &new[new_index])
                    && numbers
                        .as_ref()
                        .is_none_or(|(before, after)| before[old_index] == after[new_index])
            })
            .count();
        let delta = new.len() as isize - old.len() as isize;
        let convergence = Convergence {
            starts: &previous.starts,
            floating: &previous.floating,
            run_starts: &previous.run_starts,
            number_restarts: &previous.number_restarts,
            unchanged_from: new.len() - unchanged_tail,
            delta,
            tables: &changed_tables,
        };
        let start = found.and_then(|page| previous.starts[page].clone());
        let mut engine = Engine::new(document, shaper, &mut self.cache, &self.context);
        engine.pages = previous.main_pages[..restart].to_vec();
        engine.page_layouts = vec![Vec::new(); restart];
        engine.starts = previous.starts[..restart].to_vec();
        engine.floating =
            previous.floating[..start.as_ref().map_or(0, |start| start.floating)].to_vec();
        engine.run_starts = previous
            .run_starts
            .iter()
            .copied()
            .filter(|number| (*number as usize) <= restart)
            .collect();
        // A restart is pushed by the break on the page before it: the one
        // for the first page laid out again comes from a page kept.
        engine.number_restarts = previous
            .number_restarts
            .iter()
            .copied()
            .filter(|number| (*number as usize) <= restart + 1)
            .collect();
        let main_start = match &start {
            Some(start) => {
                let page = engine.new_page(previous.main_pages[restart].layout);
                if engine.page_layouts[page] != start.frames {
                    drop(engine);
                    return self.full(document, shaper);
                }
                MainStart {
                    paragraph: start.paragraph_index,
                    word: start.word,
                    next_layout: start.next_layout,
                }
            }
            None => {
                engine.new_page(document.start_layout().id);
                MainStart::default()
            }
        };
        engine.main_text(restart, main_start, Some(&convergence));

        let end = match engine.converged {
            Some(converged) => {
                // The rest is as before: take it from the previous layout.
                for (main_page, page_start) in previous.main_pages[converged..]
                    .iter()
                    .zip(&previous.starts[converged..])
                {
                    engine.pages.push(shifted(main_page, delta));
                    engine.page_layouts.push(Vec::new());
                    engine.starts.push(page_start.clone().map(|mut page_start| {
                        page_start.paragraph_index =
                            (page_start.paragraph_index as isize + delta) as usize;
                        page_start
                    }));
                }
                engine.floating = previous.floating.clone();
                engine.run_starts = previous.run_starts.clone();
                engine.number_restarts = previous.number_restarts.clone();
                converged
            }
            None => usize::MAX,
        };
        engine.page_count_rule();
        let end = end.min(engine.pages.len());
        let main_pages = engine.pages.clone();
        engine.headers_and_footers(restart..end);
        engine.frame_interiors(restart..end);

        let mut pages = Vec::with_capacity(engine.pages.len());
        pages.extend(self.layout.pages[..restart].iter().cloned());
        pages.extend(engine.pages[restart..end].iter().cloned());
        pages.extend(
            self.layout.pages[end.min(self.layout.pages.len())..]
                .iter()
                .take(engine.pages.len() - end)
                .map(|page| shifted(page, delta)),
        );
        debug_assert_eq!(pages.len(), engine.pages.len());
        self.stats = LayoutStats {
            kind: LayoutKind::Incremental,
            pages_laid_out: end - restart,
            pages_reused: pages.len() - (end - restart),
        };
        self.layout = DocumentLayout {
            pages,
            line_numbers: Vec::new(),
        };
        self.previous = Some(Previous {
            document: document.clone(),
            main_pages,
            starts: std::mem::take(&mut engine.starts),
            floating: std::mem::take(&mut engine.floating),
            run_starts: std::mem::take(&mut engine.run_starts),
            number_restarts: std::mem::take(&mut engine.number_restarts),
        });
    }

    /// Headers, footers, and frame interiors again over the same main text.
    fn repaint(&mut self, document: &Document, shaper: &mut dyn Shaper, previous: Previous) {
        let mut engine = Engine::new(document, shaper, &mut self.cache, &self.context);
        engine.pages = previous.main_pages.clone();
        engine.page_layouts = vec![Vec::new(); engine.pages.len()];
        engine.floating = previous.floating.clone();
        engine.run_starts = previous.run_starts.clone();
        engine.number_restarts = previous.number_restarts.clone();
        let all = 0..engine.pages.len();
        engine.headers_and_footers(all.clone());
        engine.frame_interiors(all);
        self.stats = LayoutStats {
            kind: LayoutKind::Repaint,
            pages_laid_out: 0,
            pages_reused: engine.pages.len(),
        };
        self.layout = DocumentLayout {
            pages: std::mem::take(&mut engine.pages),
            line_numbers: Vec::new(),
        };
        self.previous = Some(Previous {
            document: document.clone(),
            ..previous
        });
    }
}

/// What changed outside the main text.
#[derive(Clone, Debug, Eq, PartialEq)]
enum OtherFlows {
    Unchanged,
    /// Only cells of tables anchored in the main text: the first and last
    /// paragraphs that anchor one of them, and the tables.
    Tables {
        first: usize,
        last: usize,
        tables: Vec<FrameId>,
    },
    /// Only other text (headers, footers, frames, fixed tables).
    Other,
    /// Both.
    Mixed,
}

/// Classifies the flows other than the main text that differ from `earlier`
/// (the flow IDs are the same; `layout_changes` checked that).
fn other_flows(document: &Document, earlier: &Document) -> OtherFlows {
    let main = document.main_flow_id();
    let changed: Vec<FlowId> = document
        .flows()
        .filter(|(id, flow)| {
            *id != main
                && earlier
                    .flow(*id)
                    .is_none_or(|before| !std::ptr::eq(*flow, before) && *flow != before)
        })
        .map(|(id, _)| id)
        .collect();
    let mut anchors: Option<(usize, usize)> = None;
    let mut tables = Vec::new();
    let mut other = false;
    for flow in changed {
        let table = document.frames().find(|frame| {
            frame.placement.is_anchored()
                && matches!(&frame.content, FrameContent::Table(table) if table.cells.iter().any(|cell| matches!(cell.content, CellContent::Text(cell_flow) if cell_flow == flow)))
        });
        let paragraph = table.and_then(|frame| {
            document.paragraphs().iter().position(|paragraph| {
                paragraph.objects.iter().any(
                    |object| matches!(object, InlineObject::FrameAnchor(id) if *id == frame.id),
                )
            })
        });
        match (paragraph, table) {
            (Some(index), Some(table)) => {
                anchors = Some(anchors.map_or((index, index), |(first, last)| {
                    (first.min(index), last.max(index))
                }));
                if !tables.contains(&table.id) {
                    tables.push(table.id);
                }
            }
            _ => other = true,
        }
    }
    match (anchors, other) {
        (None, false) => OtherFlows::Unchanged,
        (None, true) => OtherFlows::Other,
        (Some((first, last)), false) => OtherFlows::Tables {
            first,
            last,
            tables,
        },
        (Some(_), true) => OtherFlows::Mixed,
    }
}

fn same(left: &Arc<Paragraph>, right: &Arc<Paragraph>) -> bool {
    Arc::ptr_eq(left, right) || left == right
}

/// A reused page with its main-text paragraph indexes moved by `delta`.
fn shifted(page: &Arc<PageBox>, delta: isize) -> Arc<PageBox> {
    let mut page = Arc::clone(page);
    if delta != 0 {
        for line in &mut Arc::make_mut(&mut page).lines {
            if line.container == Container::Main {
                line.paragraph_index = (line.paragraph_index as isize + delta) as usize;
            }
        }
    }
    page
}

/// Whether the main text refers to a footnote.
fn has_footnotes(document: &Document) -> bool {
    document.paragraphs().iter().any(|paragraph| {
        paragraph
            .objects
            .iter()
            .any(|object| matches!(object, InlineObject::Footnote { .. }))
    })
}
