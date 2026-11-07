//! A paragraph's own tab ruler and indents, as Ami Pro's current ruler and
//! Page › Ruler set them (`docs/research/RULER_NOTES.md`): one tab list for
//! every selected paragraph (`1170:052d`), and indents changed one value at
//! a time, each paragraph keeping its other values (`12e8:01da`,
//! `12e8:056d`).

use amipro_document::{Document, FlowId, Indents, LayoutBox, TabStop, Twips};

use crate::{Applied, ChangeReport, EditError};

/// Indents to change: `None` keeps a paragraph's own value, or its
/// style's when it has none (Ami Pro's 0xFFFF).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IndentChange {
    pub all: Option<Twips>,
    pub first_line: Option<Twips>,
    pub rest: Option<Twips>,
    pub right: Option<Twips>,
}

impl IndentChange {
    /// `indents` with this change made.
    pub fn applied_to(self, indents: Indents) -> Indents {
        Indents {
            all: self.all.unwrap_or(indents.all),
            first: self.first_line.unwrap_or(indents.first),
            rest: self.rest.unwrap_or(indents.rest),
            right: self.right.unwrap_or(indents.right),
        }
    }

    fn values(self) -> [Option<Twips>; 4] {
        [self.all, self.first_line, self.rest, self.right]
    }
}

/// The largest length a ruler or box takes (22 in, `AMIUICNT.DLL`).
const MAX_LENGTH: Twips = 31_680;

fn checked_paragraphs(
    document: &Document,
    flow: FlowId,
    first: usize,
    last: usize,
) -> Result<(), EditError> {
    let flow = document.flow(flow).ok_or("flow does not exist")?;
    if first > last || last >= flow.len() {
        return Err("paragraph out of bounds");
    }
    Ok(())
}

pub(crate) fn set_paragraph_tabs(
    document: &Document,
    flow_id: FlowId,
    first: usize,
    last: usize,
    tabs: Option<&[TabStop]>,
) -> Result<Applied, EditError> {
    checked_paragraphs(document, flow_id, first, last)?;
    let tabs = match tabs {
        Some(tabs) => {
            if tabs.len() > LayoutBox::MAX_TABS {
                return Err("a ruler holds at most 22 tabs");
            }
            if tabs
                .iter()
                .any(|stop| !(0..=MAX_LENGTH).contains(&stop.position))
            {
                return Err("tab position out of range");
            }
            let mut tabs = tabs.to_vec();
            tabs.sort_by_key(|stop| stop.position);
            Some(tabs)
        }
        None => None,
    };
    let flow = document.flow(flow_id).ok_or("flow does not exist")?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for index in first..=last {
        if flow.paragraphs[index].format.tabs == tabs {
            continue;
        }
        let paragraph = next
            .flow_mut(flow_id)
            .and_then(|flow| flow.paragraph_mut(index))
            .ok_or("paragraph out of bounds")?;
        paragraph.format.tabs.clone_from(&tabs);
        report.changed.push(paragraph.id);
    }
    if !report.changed.is_empty() {
        report.flows.insert(flow_id);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

pub(crate) fn set_paragraph_indents(
    document: &Document,
    flow_id: FlowId,
    first: usize,
    last: usize,
    change: IndentChange,
) -> Result<Applied, EditError> {
    checked_paragraphs(document, flow_id, first, last)?;
    if change.values().iter().all(Option::is_none) {
        return Err("indent change is empty");
    }
    if change
        .values()
        .iter()
        .flatten()
        .any(|value| !(0..=MAX_LENGTH).contains(value))
    {
        return Err("indent out of range");
    }
    let flow = document.flow(flow_id).ok_or("flow does not exist")?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for index in first..=last {
        let source = &flow.paragraphs[index];
        let current = source.format.indents.unwrap_or_else(|| {
            document
                .styles()
                .get(&source.style)
                .map(|style| style.indents)
                .unwrap_or_default()
        });
        let indents = Some(change.applied_to(current));
        if source.format.indents == indents {
            continue;
        }
        let paragraph = next
            .flow_mut(flow_id)
            .and_then(|flow| flow.paragraph_mut(index))
            .ok_or("paragraph out of bounds")?;
        paragraph.format.indents = indents;
        report.changed.push(paragraph.id);
    }
    if !report.changed.is_empty() {
        report.flows.insert(flow_id);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

/// Changes one part of paragraphs `first..=last`'s own format, each
/// paragraph whose part `change` makes different.
fn change_paragraph_format(
    document: &Document,
    flow_id: FlowId,
    first: usize,
    last: usize,
    change: impl Fn(&mut amipro_document::ParagraphFormat) -> bool,
) -> Result<Applied, EditError> {
    checked_paragraphs(document, flow_id, first, last)?;
    let mut next = document.clone();
    let mut report = ChangeReport::default();
    for index in first..=last {
        let paragraph = next
            .flow_mut(flow_id)
            .and_then(|flow| flow.paragraph_mut(index))
            .ok_or("paragraph out of bounds")?;
        let mut format = paragraph.format.clone();
        if change(&mut format) && format != paragraph.format {
            paragraph.format = format;
            report.changed.push(paragraph.id);
        }
    }
    if !report.changed.is_empty() {
        report.flows.insert(flow_id);
    }
    Ok(Applied {
        document: next,
        caret: None,
        report,
    })
}

pub(crate) fn revert_paragraph_indents(
    document: &Document,
    flow_id: FlowId,
    first: usize,
    last: usize,
) -> Result<Applied, EditError> {
    change_paragraph_format(document, flow_id, first, last, |format| {
        format.indents.take().is_some()
    })
}

pub(crate) fn set_paragraph_line_spacing(
    document: &Document,
    flow_id: FlowId,
    first: usize,
    last: usize,
    spacing: Option<amipro_document::LineSpacing>,
) -> Result<Applied, EditError> {
    // 1 1/2 and Double keep 0 for "the height Ami Pro computes".
    if let Some(amipro_document::LineSpacing::Custom(height)) = spacing
        && !(1..=MAX_LENGTH).contains(&height)
    {
        return Err("line spacing out of range");
    }
    change_paragraph_format(document, flow_id, first, last, |format| {
        format.line_spacing = spacing;
        true
    })
}

#[cfg(test)]
mod tests {
    use amipro_document::{Indents, ParagraphFormat, TabKind, TabLeader, TabStop};

    use crate::{Command, History, IndentChange, TextPosition, apply};

    fn three_paragraphs() -> amipro_document::Document {
        let document = amipro_document::Document::default();
        let flow = document.main_flow_id();
        let at = TextPosition {
            flow,
            paragraph: 0,
            byte: 0,
        };
        apply(
            &document,
            &Command::ReplaceText {
                start: at,
                end: at,
                text: "One\nTwo\nThree".into(),
            },
        )
        .unwrap()
        .document
    }

    #[test]
    fn line_spacing_and_indents_are_set_and_reverted_per_paragraph_as_one_step() {
        use amipro_document::LineSpacing;
        let mut document = three_paragraphs();
        let flow = document.main_flow_id();
        let mut history = History::new();
        history
            .execute(
                &mut document,
                (),
                &Command::SetParagraphLineSpacing {
                    flow,
                    first: 0,
                    last: 1,
                    spacing: Some(LineSpacing::Double(0)),
                },
            )
            .unwrap();
        let spacing: Vec<_> = document
            .main_flow()
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.format.line_spacing)
            .collect();
        assert_eq!(
            spacing,
            [
                Some(LineSpacing::Double(0)),
                Some(LineSpacing::Double(0)),
                None
            ]
        );
        // Out of range, or no change, is refused or changes nothing.
        assert!(
            apply(
                &document,
                &Command::SetParagraphLineSpacing {
                    flow,
                    first: 0,
                    last: 0,
                    spacing: Some(LineSpacing::Custom(0)),
                },
            )
            .is_err()
        );
        let same = apply(
            &document,
            &Command::SetParagraphLineSpacing {
                flow,
                first: 0,
                last: 1,
                spacing: Some(LineSpacing::Double(0)),
            },
        )
        .unwrap();
        assert!(same.report.changed.is_empty());
        history
            .execute(
                &mut document,
                (),
                &Command::SetParagraphIndents {
                    flow,
                    first: 1,
                    last: 2,
                    indents: IndentChange {
                        all: Some(720),
                        ..IndentChange::default()
                    },
                },
            )
            .unwrap();
        history
            .execute(
                &mut document,
                (),
                &Command::RevertParagraphIndents {
                    flow,
                    first: 0,
                    last: 2,
                },
            )
            .unwrap();
        assert!(
            document
                .main_flow()
                .paragraphs
                .iter()
                .all(|paragraph| paragraph.format.indents.is_none())
        );
        // Revert to style for the spacing; then everything undoes in steps.
        history
            .execute(
                &mut document,
                (),
                &Command::SetParagraphLineSpacing {
                    flow,
                    first: 0,
                    last: 2,
                    spacing: None,
                },
            )
            .unwrap();
        assert!(
            document
                .main_flow()
                .paragraphs
                .iter()
                .all(|paragraph| paragraph.format == ParagraphFormat::default())
        );
        assert_eq!(history.undo_len(), 4);
        while history.undo(&mut document, ()).is_some() {}
        assert_eq!(document, three_paragraphs_like(&document));
    }

    /// The three paragraphs with every format removed, IDs as in `document`.
    fn three_paragraphs_like(document: &amipro_document::Document) -> amipro_document::Document {
        let mut plain = document.clone();
        let flow = plain.main_flow_id();
        for index in 0..3 {
            plain
                .flow_mut(flow)
                .unwrap()
                .paragraph_mut(index)
                .unwrap()
                .format = ParagraphFormat::default();
        }
        plain
    }

    fn left(position: i32) -> TabStop {
        TabStop::new(TabKind::Left, position)
    }

    #[test]
    fn one_tab_list_becomes_every_selected_paragraphs_ruler_ru1() {
        let document = three_paragraphs();
        let flow = document.main_flow_id();
        // Out of order on purpose: a ruler is kept by position.
        let tabs = vec![
            left(3228),
            TabStop {
                kind: TabKind::Right,
                position: 720,
                leader: TabLeader::Dots,
            },
        ];
        let applied = apply(
            &document,
            &Command::SetParagraphTabs {
                flow,
                first: 0,
                last: 1,
                tabs: Some(tabs),
            },
        )
        .unwrap();
        let paragraphs = applied.document.paragraphs();
        let positions: Vec<_> = paragraphs[0]
            .format
            .tabs
            .as_ref()
            .unwrap()
            .iter()
            .map(|stop| stop.position)
            .collect();
        assert_eq!(positions, [720, 3228]);
        assert_eq!(paragraphs[1].format.tabs, paragraphs[0].format.tabs);
        assert_eq!(paragraphs[2].format.tabs, None, "outside the range");
        assert_eq!(applied.report.changed.len(), 2);
        // Removing the ruler returns them to the style's or the box's stops.
        let removed = apply(
            &applied.document,
            &Command::SetParagraphTabs {
                flow,
                first: 0,
                last: 2,
                tabs: None,
            },
        )
        .unwrap();
        assert!(
            removed
                .document
                .paragraphs()
                .iter()
                .all(|paragraph| paragraph.format.tabs.is_none())
        );
        assert_eq!(removed.report.changed.len(), 2, "only the two with rulers");
        // An empty ruler (Clear Tabs) is a ruler with no stops.
        let cleared = apply(
            &document,
            &Command::SetParagraphTabs {
                flow,
                first: 2,
                last: 2,
                tabs: Some(Vec::new()),
            },
        )
        .unwrap();
        assert_eq!(
            cleared.document.paragraphs()[2].format.tabs,
            Some(Vec::new())
        );
    }

    #[test]
    fn a_ruler_holds_at_most_22_tabs_at_valid_positions() {
        let document = three_paragraphs();
        let flow = document.main_flow_id();
        let set = |tabs: Vec<TabStop>| {
            apply(
                &document,
                &Command::SetParagraphTabs {
                    flow,
                    first: 0,
                    last: 0,
                    tabs: Some(tabs),
                },
            )
        };
        assert!(set((1..=22).map(|stop| left(stop * 360)).collect()).is_ok());
        assert!(set((1..=23).map(|stop| left(stop * 360)).collect()).is_err());
        assert!(set(vec![left(-1)]).is_err());
        assert!(
            apply(
                &document,
                &Command::SetParagraphTabs {
                    flow,
                    first: 1,
                    last: 3,
                    tabs: None,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn one_indent_changes_and_each_paragraph_keeps_its_others_ru5_to_ru8() {
        let mut document = three_paragraphs();
        let flow = document.main_flow_id();
        // The second paragraph has its own indents; the others take the
        // style's (Body Text: none).
        document.paragraph_mut(1).format = ParagraphFormat {
            indents: Some(Indents {
                all: 360,
                first: 0,
                rest: 180,
                right: 90,
            }),
            ..ParagraphFormat::default()
        };
        let mut history = History::<()>::new();
        let before = document.clone();
        history
            .execute(
                &mut document,
                (),
                &Command::SetParagraphIndents {
                    flow,
                    first: 0,
                    last: 2,
                    indents: IndentChange {
                        first_line: Some(720),
                        ..IndentChange::default()
                    },
                },
            )
            .unwrap();
        let indents: Vec<_> = document
            .paragraphs()
            .iter()
            .map(|paragraph| paragraph.format.indents.unwrap())
            .collect();
        let only_first = Indents {
            first: 720,
            ..Indents::default()
        };
        assert_eq!(indents[0], only_first, "RU5: <:I0,720,0,0>");
        assert_eq!(
            indents[1],
            Indents {
                all: 360,
                first: 720,
                rest: 180,
                right: 90
            }
        );
        assert_eq!(indents[2], only_first);
        history.undo(&mut document, ()).unwrap();
        assert_eq!(document, before, "one undo step");
        // Nothing to change, or a negative value, is refused.
        let change = |indents: IndentChange| {
            apply(
                &document,
                &Command::SetParagraphIndents {
                    flow,
                    first: 0,
                    last: 0,
                    indents,
                },
            )
        };
        assert!(change(IndentChange::default()).is_err());
        assert!(
            change(IndentChange {
                right: Some(-10),
                ..IndentChange::default()
            })
            .is_err()
        );
    }

    #[test]
    fn text_normal_keeps_the_paragraphs_own_ruler() {
        let mut document = three_paragraphs();
        let flow = document.main_flow_id();
        let tabs = Some(vec![left(1440)]);
        document.paragraph_mut(0).format = ParagraphFormat {
            indents: Some(Indents {
                all: 720,
                ..Indents::default()
            }),
            tabs: tabs.clone(),
            ..ParagraphFormat::default()
        };
        let at = TextPosition {
            flow,
            paragraph: 0,
            byte: 1,
        };
        let cleared = apply(
            &document,
            &Command::ClearDirectFormatting { start: at, end: at },
        )
        .unwrap()
        .document;
        let format = &cleared.paragraphs()[0].format;
        assert_eq!(format.indents, None);
        assert_eq!(format.tabs, tabs);
    }
}
