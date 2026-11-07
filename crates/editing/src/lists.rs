//! Bullets and numbering as the app's Format › Bullets and Format ›
//! Numbering make them (`docs/Design/UI_SPEC.md` §21, decisions B1 and B3).
//! Ami Pro has no such commands and keeps bullets and numbers only in
//! paragraph styles (`docs/research/BULLETS_NOTES.md`), so they apply Ami
//! Pro's list styles: the document's Bullet 1 or Bullet, and its Number
//! List, made in the document as `_DEFAULT.STY` defines them when it has
//! none.

use amipro_document::{
    Bullets, Document, FlowId, Indents, ParagraphStyle, StyleId, StyleOrigin, is_bullet_code,
};

use crate::{Applied, Command, EditError, TextPosition};

/// The two kinds of list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListKind {
    Bullets,
    Numbering,
}

impl ListKind {
    /// The names Ami Pro's style sheets give the style, in the order the
    /// commands look for them (`_DEFAULT.STY`; the shipped and the user's
    /// sheets: Bullet 1 in 29 and 103, Bullet in 23 and 53, Number List in
    /// 32 and 104).
    pub(crate) fn names(self) -> &'static [&'static str] {
        match self {
            Self::Bullets => &["Bullet 1", "Bullet"],
            Self::Numbering => &["Number List"],
        }
    }

    /// Names for a style made when the document has none, in order of
    /// preference; the first not taken is used.
    fn new_names(self) -> &'static [&'static str] {
        match self {
            Self::Bullets => &["Bullet 1", "Bullet", "List Bullet"],
            Self::Numbering => &["Number List", "Numbered List", "List Number"],
        }
    }
}

/// The list a style's paragraphs show: Numbering when its bullets text
/// counts, else Bullets when it draws one of Ami Pro's bullets; leading
/// text alone is neither.
pub fn shown_list(style: &ParagraphStyle) -> Option<ListKind> {
    if style.bullets.numbered() {
        Some(ListKind::Numbering)
    } else if style.bullets.text.chars().any(is_bullet_code) {
        Some(ListKind::Bullets)
    } else {
        None
    }
}

/// The list the paragraph at `flow`, `paragraph` shows, through its style.
pub fn paragraph_list(document: &Document, flow: FlowId, paragraph: usize) -> Option<ListKind> {
    let paragraph = document.flow(flow)?.paragraphs.get(paragraph)?;
    shown_list(document.styles().get(&paragraph.style)?)
}

/// The document's style for `kind`, by Ami Pro's names (letter case aside),
/// if one of them shows that kind of list.
pub fn list_style(document: &Document, kind: ListKind) -> Option<StyleId> {
    kind.names().iter().find_map(|name| {
        document
            .styles()
            .values()
            .find(|style| style.name.eq_ignore_ascii_case(name) && shown_list(style) == Some(kind))
            .map(|style| style.id)
    })
}

/// Ami Pro's list style as `_DEFAULT.STY` defines it, in `body`'s font,
/// spacing, lines, and right indent (Body Text's): Bullet 1 has All,
/// First, and Rest 0.20 in and a small diamond; Number List 0.25 in and
/// `1.` with Right align. `[spec]` value 4 is 0, as Ami Pro 3.1 writes it
/// for a style it creates (VM fixture BL4; the sheet's 360 is Ami Pro 2's
/// Space for, which the reader moves into First, `1058:0fac`). Stored in
/// the document; Enter after it gives itself. `set_list` gives it the next
/// free function key.
pub fn default_list_style(kind: ListKind, body: &ParagraphStyle) -> ParagraphStyle {
    let (indent, bullets) = match kind {
        ListKind::Bullets => (
            288,
            Bullets {
                text: "\u{15}".into(),
                ..Bullets::default()
            },
        ),
        ListKind::Numbering => (
            360,
            Bullets {
                text: "\u{1A}.".into(),
                flags: 16,
                ..Bullets::default()
            },
        ),
    };
    ParagraphStyle {
        name: kind.new_names()[0].into(),
        parent: None,
        indents: Indents {
            all: indent,
            first: indent,
            rest: indent,
            right: body.indents.right,
        },
        next: None,
        hanging: false,
        bullets,
        origin: StyleOrigin::Document,
        kept: amipro_document::StyleKept {
            function_key: 0,
            next_name: None,
            ..body.kept.clone()
        },
        ..body.clone()
    }
}

/// `SetList`: paragraphs `first..=last` of `flow` take the document's style
/// for `list`, made first when it has none, or Body Text for `None`.
pub(crate) fn set_list(
    document: &Document,
    flow: FlowId,
    first: usize,
    last: usize,
    list: Option<ListKind>,
) -> Result<Applied, EditError> {
    let Some(kind) = list else {
        return crate::set_paragraph_style(document, flow, first, last, document.body_text_style());
    };
    if let Some(style) = list_style(document, kind) {
        return crate::set_paragraph_style(document, flow, first, last, style);
    }
    let body = document
        .styles()
        .get(&document.body_text_style())
        .ok_or("the document has no styles")?;
    let name = kind
        .new_names()
        .iter()
        .find(|name| {
            !document
                .styles()
                .values()
                .any(|style| style.name.eq_ignore_ascii_case(name))
        })
        .ok_or("every list style name is taken")?;
    let id = StyleId(
        document
            .styles()
            .keys()
            .next_back()
            .map_or(1, |id| id.0 + 1),
    );
    let mut style = ParagraphStyle {
        id,
        name: (*name).into(),
        ..default_list_style(kind, body)
    };
    style.kept.function_key = crate::styles::next_function_key(document.styles());
    let mut with_style = document.clone();
    with_style.styles_mut().insert(id, style);
    with_style
        .validate()
        .map_err(|_| "the styles would not be valid")?;
    let mut applied = crate::set_paragraph_style(&with_style, flow, first, last, id)?;
    applied.report.styles = true;
    Ok(applied)
}

/// The command Format › Bullets or Numbering gives for paragraphs
/// `first..=last` of `flow`: off (Body Text) when every one already shows
/// `kind`, else on.
pub fn toggle_list(
    document: &Document,
    flow: FlowId,
    first: usize,
    last: usize,
    kind: ListKind,
) -> Command {
    let all = (first..=last).all(|index| paragraph_list(document, flow, index) == Some(kind));
    Command::SetList {
        flow,
        first,
        last,
        list: (!all).then_some(kind),
    }
}

/// What Enter does instead of a new paragraph at `at` (decision B3): in an
/// empty paragraph that shows a list, it ends the list (Body Text).
pub fn ends_list(document: &Document, at: TextPosition) -> Option<Command> {
    let paragraph = document.flow(at.flow)?.paragraphs.get(at.paragraph)?;
    (paragraph.text.is_empty() && paragraph_list(document, at.flow, at.paragraph).is_some())
        .then_some(Command::SetList {
            flow: at.flow,
            first: at.paragraph,
            last: at.paragraph,
            list: None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{History, apply};
    use amipro_document::default_styles;

    fn styled(names: &[(&str, &str)]) -> Document {
        let mut styles = default_styles();
        let body = styles
            .values()
            .find(|style| style.name == "Body Text")
            .unwrap()
            .clone();
        for (index, (name, text)) in names.iter().enumerate() {
            let id = StyleId(100 + index as u64);
            styles.insert(
                id,
                ParagraphStyle {
                    id,
                    name: (*name).into(),
                    bullets: Bullets {
                        text: (*text).into(),
                        ..Bullets::default()
                    },
                    origin: StyleOrigin::Sheet,
                    ..body.clone()
                },
            );
        }
        let mut document = Document::empty("Lists", styles);
        let body = document.body_text_style();
        for text in ["one", "two", "three"] {
            document.push_paragraph(text, body);
        }
        document
    }

    fn style_of(document: &Document, index: usize) -> &ParagraphStyle {
        &document.styles()[&document.paragraphs()[index].style]
    }

    #[test]
    fn styles_show_bullets_numbers_or_neither() {
        let document = styled(&[
            ("Bullet 1", "\u{15}"),
            ("Number List", "\u{1A}."),
            ("Note", "Note:"),
            ("Both", "\u{1A}.\u{11}"),
        ]);
        let shown = |name: &str| {
            shown_list(
                document
                    .styles()
                    .values()
                    .find(|style| style.name == name)
                    .unwrap(),
            )
        };
        assert_eq!(shown("Bullet 1"), Some(ListKind::Bullets));
        assert_eq!(shown("Number List"), Some(ListKind::Numbering));
        assert_eq!(shown("Note"), None);
        assert_eq!(shown("Both"), Some(ListKind::Numbering));
        assert_eq!(shown("Body Text"), None);
    }

    #[test]
    fn the_sheets_list_styles_come_first_by_ami_pros_names() {
        // Bullet 1 before Bullet; a "Bullet 1" without a bullet is passed
        // over.
        let document = styled(&[("Bullet", "\u{10}"), ("Bullet 1", "\u{15}")]);
        let bullet_1 = list_style(&document, ListKind::Bullets).unwrap();
        assert_eq!(document.styles()[&bullet_1].name, "Bullet 1");
        let only_bullet = styled(&[("Bullet", "\u{10}"), ("bullet 1", "")]);
        let bullet = list_style(&only_bullet, ListKind::Bullets).unwrap();
        assert_eq!(only_bullet.styles()[&bullet].name, "Bullet");
        assert_eq!(list_style(&only_bullet, ListKind::Numbering), None);
        // Applied as it is; nothing is made.
        let applied = apply(
            &document,
            &Command::SetList {
                flow: document.main_flow_id(),
                first: 0,
                last: 1,
                list: Some(ListKind::Bullets),
            },
        )
        .unwrap();
        assert_eq!(applied.document.styles().len(), document.styles().len());
        assert!(!applied.report.styles);
        assert_eq!(style_of(&applied.document, 0).name, "Bullet 1");
        assert_eq!(style_of(&applied.document, 1).name, "Bullet 1");
        assert_eq!(style_of(&applied.document, 2).name, "Body Text");
    }

    #[test]
    fn a_document_without_one_gets_ami_pros_default_list_styles() {
        let mut document = styled(&[]);
        let body = document.body_text_style();
        document.styles_mut().get_mut(&body).unwrap().font_family = "Arial".into();
        let mut history = History::new();
        let flow = document.main_flow_id();
        history
            .execute(
                &mut document,
                (),
                &Command::SetList {
                    flow,
                    first: 0,
                    last: 2,
                    list: Some(ListKind::Numbering),
                },
            )
            .unwrap();
        let made = style_of(&document, 0).clone();
        assert_eq!(made.name, "Number List");
        assert_eq!(made.origin, StyleOrigin::Document);
        assert_eq!(made.font_family, "Arial", "Body Text's font");
        assert_eq!(made.bullets.text, "\u{1A}.");
        assert_eq!(made.bullets.flags, 16, "Right align");
        assert_eq!(made.bullets.kept, Bullets::default().kept, "value 4 is 0");
        assert_eq!(
            (made.indents.all, made.indents.first, made.indents.rest),
            (360, 360, 360)
        );
        // The next free function key, as Ami Pro's Create Style gives one.
        let key = made.kept.function_key;
        assert!((2..=12).contains(&key) && key != 10, "{key}");
        assert!(
            document
                .styles()
                .values()
                .filter(|style| style.id != made.id)
                .all(|style| style.kept.function_key != key)
        );
        assert_eq!(made.next, None);
        assert_eq!(style_of(&document, 2).id, made.id);
        // One undo step takes the style away again.
        history.undo(&mut document, ());
        assert!(
            document
                .styles()
                .values()
                .all(|style| style.name != "Number List")
        );
        assert_eq!(style_of(&document, 0).name, "Body Text");
        // Bullet 1: a small diamond at 0.20 in.
        history
            .execute(
                &mut document,
                (),
                &Command::SetList {
                    flow,
                    first: 1,
                    last: 1,
                    list: Some(ListKind::Bullets),
                },
            )
            .unwrap();
        let bullet = style_of(&document, 1);
        assert_eq!(bullet.name, "Bullet 1");
        assert_eq!(bullet.bullets.text, "\u{15}");
        assert_eq!(bullet.indents.first, 288);
        // A taken name without the list moves to the next.
        let taken = styled(&[("Bullet 1", ""), ("Bullet", "Note:")]);
        let applied = apply(
            &taken,
            &Command::SetList {
                flow: taken.main_flow_id(),
                first: 0,
                last: 0,
                list: Some(ListKind::Bullets),
            },
        )
        .unwrap();
        assert_eq!(style_of(&applied.document, 0).name, "List Bullet");
        assert!(applied.report.styles);
    }

    #[test]
    fn toggling_turns_a_list_off_only_when_every_paragraph_shows_it() {
        let document = styled(&[("Bullet 1", "\u{15}"), ("Number List", "\u{1A}.")]);
        let flow = document.main_flow_id();
        let bullets = list_style(&document, ListKind::Bullets).unwrap();
        let numbers = list_style(&document, ListKind::Numbering).unwrap();
        let mut document = document;
        document.paragraph_mut(0).style = bullets;
        document.paragraph_mut(1).style = numbers;
        // A mixed selection turns bullets on for every paragraph.
        assert_eq!(
            toggle_list(&document, flow, 0, 2, ListKind::Bullets),
            Command::SetList {
                flow,
                first: 0,
                last: 2,
                list: Some(ListKind::Bullets)
            }
        );
        // All bulleted: off, to Body Text.
        let off = toggle_list(&document, flow, 0, 0, ListKind::Bullets);
        assert_eq!(
            off,
            Command::SetList {
                flow,
                first: 0,
                last: 0,
                list: None
            }
        );
        let applied = apply(&document, &off).unwrap().document;
        assert_eq!(style_of(&applied, 0).name, "Body Text");
        // A numbered paragraph given Bullets switches.
        let switch = toggle_list(&document, flow, 1, 1, ListKind::Bullets);
        let applied = apply(&document, &switch).unwrap().document;
        assert_eq!(style_of(&applied, 1).id, bullets);
    }

    #[test]
    fn enter_in_an_empty_list_paragraph_ends_the_list() {
        let mut document = styled(&[("Bullet 1", "\u{15}")]);
        let flow = document.main_flow_id();
        let bullets = list_style(&document, ListKind::Bullets).unwrap();
        document.paragraph_mut(0).style = bullets;
        document.paragraph_mut(1).style = bullets;
        document.paragraph_mut(1).text.clear();
        let at = |paragraph| TextPosition {
            flow,
            paragraph,
            byte: 0,
        };
        assert_eq!(ends_list(&document, at(0)), None, "a paragraph with text");
        assert_eq!(ends_list(&document, at(2)), None, "not a list");
        assert_eq!(
            ends_list(&document, at(1)),
            Some(Command::SetList {
                flow,
                first: 1,
                last: 1,
                list: None
            })
        );
    }
}
