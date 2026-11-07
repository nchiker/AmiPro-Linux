//! Style commands (Phase 3.4): Ami Pro's Create Style, a style's definition
//! changed, renamed, or removed (Style Management), and Use Another Style
//! Sheet, by the rules of the user's guide (chapter 9) and VM fixture ST1
//! (`docs/research/SAM_FORMAT_NOTES.md`, "Styles and style sheets").

use std::collections::BTreeMap;

use amipro_document::{
    Document, PageLayout, ParagraphStyle, StyleId, StyleOrigin, StyleSheet, StyleSheetLink,
};

use crate::frames::LayoutSettings;
use crate::{Applied, ChangeReport, EditError};

const BODY_TEXT: &str = "Body Text";

fn changed(document: Document) -> Result<Applied, EditError> {
    document
        .validate()
        .map_err(|_| "the styles would not be valid")?;
    Ok(Applied {
        document,
        caret: None,
        report: ChangeReport {
            styles: true,
            ..ChangeReport::default()
        },
    })
}

/// The ID of the style named `name` (letter case aside).
fn named(styles: &BTreeMap<StyleId, ParagraphStyle>, name: &str) -> Option<StyleId> {
    styles
        .values()
        .find(|style| style.name.eq_ignore_ascii_case(name))
        .map(|style| style.id)
}

/// A usable style name, not taken by another style than `except`.
/// Why `name` cannot name a style (other than `except`), for dialogs to
/// say before a command is tried.
pub fn style_name_problem(
    styles: &BTreeMap<StyleId, ParagraphStyle>,
    name: &str,
    except: Option<StyleId>,
) -> Option<EditError> {
    check_name(styles, name, except).err()
}

fn check_name(
    styles: &BTreeMap<StyleId, ParagraphStyle>,
    name: &str,
    except: Option<StyleId>,
) -> Result<(), EditError> {
    if name.trim().is_empty() || name.trim() != name {
        return Err("a style needs a name without spaces around it");
    }
    if name.contains(['@', '\n', '\t']) {
        return Err("a style name cannot hold @, a tab, or a line break");
    }
    if named(styles, name).is_some_and(|id| Some(id) != except) {
        return Err("another style has that name");
    }
    Ok(())
}

/// The function key Ami Pro gives a style it creates (`10a8:0000`; VM
/// fixture BL4): the lowest of F2 to F12 that no style has, skipping F10
/// (the menu key), or 0 (none) when every one is taken.
pub(crate) fn next_function_key(styles: &BTreeMap<StyleId, ParagraphStyle>) -> u8 {
    (2..=12u8)
        .filter(|key| *key != 10)
        .find(|key| styles.values().all(|style| style.kept.function_key != *key))
        .unwrap_or(0)
}

fn next_style_id(styles: &BTreeMap<StyleId, ParagraphStyle>) -> StyleId {
    StyleId(styles.keys().next_back().map_or(1, |id| id.0 + 1))
}

/// `style` formatted as Body Text, as Ami Pro formats a style defined
/// nowhere: its ID and name stay.
fn as_body_text(style: &ParagraphStyle, body: Option<&ParagraphStyle>) -> ParagraphStyle {
    let Some(body) = body.filter(|body| body.id != style.id) else {
        return ParagraphStyle {
            origin: StyleOrigin::Missing,
            ..style.clone()
        };
    };
    ParagraphStyle {
        id: style.id,
        name: style.name.clone(),
        next: None,
        origin: StyleOrigin::Missing,
        ..body.clone()
    }
}

/// Missing styles follow Body Text whenever it changes.
fn refresh_missing(styles: &mut BTreeMap<StyleId, ParagraphStyle>) {
    let body = named(styles, BODY_TEXT).map(|id| styles[&id].clone());
    for style in styles.values_mut() {
        if style.origin == StyleOrigin::Missing {
            *style = as_body_text(style, body.as_ref());
        }
    }
}

/// Create Style: a new style with `based_on`'s settings, stored in the
/// document. Enter after it gives itself unless `based_on` named another.
pub(crate) fn create_style(
    document: &Document,
    name: &str,
    based_on: StyleId,
) -> Result<Applied, EditError> {
    let base = document
        .styles()
        .get(&based_on)
        .ok_or("the style to base it on does not exist")?;
    check_name(document.styles(), name, None)?;
    let id = next_style_id(document.styles());
    // A new style takes the next free function key, as in Ami Pro (BL4),
    // and names no unknown next style.
    let style = ParagraphStyle {
        id,
        name: name.to_owned(),
        next: base.next.filter(|next| *next != based_on),
        origin: StyleOrigin::Document,
        kept: amipro_document::StyleKept {
            function_key: next_function_key(document.styles()),
            next_name: None,
            ..base.kept.clone()
        },
        ..base.clone()
    };
    let mut document = document.clone();
    document.styles_mut().insert(id, style);
    changed(document)
}

/// A style's new definition; its ID stays, and a style changed here is
/// stored in the document (Ami Pro marks it in Style Management).
pub(crate) fn modify_style(
    document: &Document,
    style: &ParagraphStyle,
) -> Result<Applied, EditError> {
    if !document.styles().contains_key(&style.id) {
        return Err("the style does not exist");
    }
    check_name(document.styles(), &style.name, Some(style.id))?;
    if style
        .next
        .is_some_and(|next| !document.styles().contains_key(&next))
    {
        return Err("the next style does not exist");
    }
    let mut document = document.clone();
    let styles = document.styles_mut();
    styles.insert(
        style.id,
        ParagraphStyle {
            origin: StyleOrigin::Document,
            ..style.clone()
        },
    );
    refresh_missing(styles);
    changed(document)
}

pub(crate) fn rename_style(
    document: &Document,
    id: StyleId,
    name: &str,
) -> Result<Applied, EditError> {
    let style = document
        .styles()
        .get(&id)
        .ok_or("the style does not exist")?;
    if style.name.eq_ignore_ascii_case(BODY_TEXT) {
        return Err("Body Text keeps its name");
    }
    let style = ParagraphStyle {
        name: name.to_owned(),
        ..style.clone()
    };
    modify_style(document, &style)
}

/// Style Management's Remove (VM fixture ST1): the paragraphs keep the
/// style's name; a style created or changed in the document formats as
/// Body Text, or, given the style sheet's version of one the document
/// changed, takes that again. Body Text can only go back to the sheet's.
pub(crate) fn delete_style(
    document: &Document,
    id: StyleId,
    sheet_version: Option<&ParagraphStyle>,
) -> Result<Applied, EditError> {
    let style = document
        .styles()
        .get(&id)
        .ok_or("the style does not exist")?;
    let mut document = document.clone();
    let styles = document.styles_mut();
    match sheet_version {
        Some(sheet) => {
            if !sheet.name.eq_ignore_ascii_case(&style.name) {
                return Err("the style sheet's version has another name");
            }
            let next = sheet
                .next
                .filter(|next| styles.contains_key(next))
                .filter(|next| *next != id);
            styles.insert(
                id,
                ParagraphStyle {
                    id,
                    next,
                    origin: StyleOrigin::Sheet,
                    ..sheet.clone()
                },
            );
        }
        None => {
            if style.name.eq_ignore_ascii_case(BODY_TEXT) {
                return Err("Body Text cannot be removed");
            }
            if style.origin == StyleOrigin::Missing {
                return Err("the style is already defined nowhere");
            }
            let body = named(styles, BODY_TEXT).map(|body| styles[&body].clone());
            styles.insert(id, as_body_text(style, body.as_ref()));
        }
    }
    // Next styles that named a removed style give themselves.
    if sheet_version.is_none() {
        for other in styles.values_mut() {
            if other.next == Some(id) {
                other.next = None;
            }
        }
    }
    refresh_missing(styles);
    changed(document)
}

/// Use Another Style Sheet (user's guide, chapter 9; VM fixture ST1):
/// styles stored in the document keep their definitions; the others take
/// the new sheet's style of the same name, or, without one, format as Body
/// Text and keep their names; the sheet's other styles join the document.
/// The page's settings follow the sheet only where the layout was the old
/// sheet's; its header and footer frames stay.
pub(crate) fn use_style_sheet(
    document: &Document,
    sheet: &StyleSheet,
) -> Result<Applied, EditError> {
    if sheet.name.trim().is_empty() {
        return Err("a style sheet has a file name");
    }
    if sheet.styles.is_empty() {
        return Err("the style sheet has no styles");
    }
    let mut document = document.clone();
    let layout_from_sheet = document
        .style_sheet()
        .is_none_or(|link| link.layout_from_sheet);
    let styles = document.styles_mut();
    // The sheet's IDs mapped to the document's, by name.
    let mut ids: BTreeMap<StyleId, StyleId> = BTreeMap::new();
    for style in &sheet.styles {
        let id = match named(styles, &style.name) {
            Some(id) => id,
            None => {
                let id = next_style_id(styles);
                styles.insert(
                    id,
                    ParagraphStyle {
                        id,
                        origin: StyleOrigin::Sheet,
                        ..style.clone()
                    },
                );
                id
            }
        };
        ids.insert(style.id, id);
    }
    let by_name: BTreeMap<String, &ParagraphStyle> = sheet
        .styles
        .iter()
        .map(|style| (style.name.to_lowercase(), style))
        .collect();
    for style in styles.values_mut() {
        if style.origin == StyleOrigin::Document {
            continue;
        }
        match by_name.get(&style.name.to_lowercase()) {
            Some(from_sheet) => {
                *style = ParagraphStyle {
                    id: style.id,
                    next: from_sheet.next.and_then(|next| ids.get(&next).copied()),
                    origin: StyleOrigin::Sheet,
                    ..(*from_sheet).clone()
                };
            }
            None => style.origin = StyleOrigin::Missing,
        }
    }
    refresh_missing(styles);
    if layout_from_sheet && let Some(layout) = &sheet.layout {
        let target = document.document_layout().id;
        apply_page_settings(&mut document, target, layout)?;
        mark_as_sheets(&mut document, target, layout);
    }
    if layout_from_sheet {
        forget_sheet_layout(&mut document);
    }
    document.set_style_sheet(Some(StyleSheetLink {
        name: sheet.name.clone(),
        layout_from_sheet,
    }));
    changed(document)
}

/// After Save as a Style Sheet (SD1, SD2, SS2): a saved document is linked
/// to the sheet it wrote, which now holds all its styles and its page
/// layout; an untitled one (`name` none) is detached, its styles all stored
/// in it. Names defined nowhere stay so.
pub(crate) fn saved_as_style_sheet(
    document: &Document,
    name: Option<&str>,
) -> Result<Applied, EditError> {
    if name.is_some_and(|name| name.trim().is_empty()) {
        return Err("a style sheet has a file name");
    }
    let mut document = document.clone();
    let origin = if name.is_some() {
        StyleOrigin::Sheet
    } else {
        StyleOrigin::Document
    };
    for style in document.styles_mut().values_mut() {
        if style.origin != StyleOrigin::Missing {
            style.origin = origin;
        }
    }
    document.set_style_sheet(name.map(|name| StyleSheetLink {
        name: name.to_owned(),
        layout_from_sheet: true,
    }));
    forget_sheet_layout(&mut document);
    changed(document)
}

/// The layout is the style sheet's as it is now: what it was when read no
/// longer tells whether it changed (`SHEET_LAYOUT_KEY`), and the sheet is
/// here (`STYLE_SHEET_MISSING_KEY`).
fn forget_sheet_layout(document: &mut Document) {
    document.legacy_mut().values.retain(|value| {
        value.key != amipro_document::SHEET_LAYOUT_KEY
            && value.key != amipro_document::STYLE_SHEET_MISSING_KEY
    });
}

/// The marks a style sheet's layout carries (0x80 on its number) and its
/// header and footer frames (0x8), as Save as a Style Sheet writes them
/// (`11c8:14e0`), on the document's layout `target` taken from `layout`:
/// the document is then what reopening its file gives, with its layout read
/// from the sheet, and Save As lists no lost page layout.
fn mark_as_sheets(document: &mut Document, target: amipro_document::LayoutId, layout: &PageLayout) {
    const NUMBER: &str = "lay.number.unknown";
    const FLAGS: &str = "frame.flags.unknown";
    const LAYOUT_FRAME: u32 = 0x8;
    let Some(number) = layout.legacy.get(NUMBER).map(str::to_owned) else {
        return;
    };
    let Some(own) = document.layout_mut(target) else {
        return;
    };
    own.legacy.values.retain(|value| value.key != NUMBER);
    own.legacy.push(NUMBER, number);
    let frames: Vec<_> = [own.header, own.footer]
        .into_iter()
        .flat_map(|sides| [sides.right, sides.left])
        .flatten()
        .collect();
    for id in frames {
        let Some(frame) = document.frame_mut(id) else {
            continue;
        };
        let flags = frame
            .legacy
            .get(FLAGS)
            .and_then(|value| u32::from_str_radix(value.trim_start_matches("0x"), 16).ok())
            .unwrap_or(0)
            | LAYOUT_FRAME;
        frame.legacy.values.retain(|value| value.key != FLAGS);
        frame.legacy.push(FLAGS, format!("{flags:#x}"));
    }
}

/// The sheet layout's page settings over the document's layout `target`.
fn apply_page_settings(
    document: &mut Document,
    target: amipro_document::LayoutId,
    layout: &PageLayout,
) -> Result<(), EditError> {
    let current = document
        .layouts()
        .iter()
        .find(|candidate| candidate.id == target)
        .ok_or("the document has no page layout")?;
    let settings = LayoutSettings {
        name: current.name.clone(),
        paper: layout.paper,
        landscape: layout.landscape,
        right: layout.right.clone(),
        left: layout.left.clone(),
        mirror: layout.mirror,
        header_from_second_page: layout.header_from_second_page,
        footer_from_second_page: layout.footer_from_second_page,
        header: crate::BandBoxes::default(),
        footer: crate::BandBoxes::default(),
    };
    *document = crate::frames::modify_layout(document, target, &settings)?.document;
    Ok(())
}

#[cfg(test)]
mod tests {
    use amipro_document::{StyleSheet, default_styles};

    use super::*;
    use crate::{Command, apply};

    /// ST1: a document on a style sheet with sheet styles (Body Text,
    /// Title, Bullet), a style it changed (Subhead), one it created (Mine),
    /// and one defined nowhere (Ghost), a paragraph in each.
    fn st1() -> Document {
        let body = ParagraphStyle {
            origin: StyleOrigin::Sheet,
            ..default_styles()[&StyleId(1)].clone()
        };
        let style = |id: u64, name: &str, size: f32, origin| ParagraphStyle {
            id: StyleId(id),
            name: name.into(),
            font_size_points: size,
            origin,
            ..body.clone()
        };
        let mut styles = BTreeMap::from([(StyleId(1), body.clone())]);
        for style in [
            style(2, "Title", 18.0, StyleOrigin::Sheet),
            style(3, "Subhead", 18.0, StyleOrigin::Document),
            style(4, "Mine", 12.0, StyleOrigin::Document),
            style(5, "Bullet", 12.0, StyleOrigin::Sheet),
            style(6, "Ghost", 11.0, StyleOrigin::Missing),
        ] {
            styles.insert(style.id, style);
        }
        styles.get_mut(&StyleId(4)).unwrap().font_family = "Courier New".into();
        let mut document = Document::empty("ST1", styles);
        for id in 1..=6 {
            document.push_paragraph("para", StyleId(id));
        }
        document.set_style_sheet(Some(StyleSheetLink {
            name: "_DEFAULT.STY".into(),
            layout_from_sheet: true,
        }));
        document.validate().unwrap();
        document
    }

    fn letter_wide(document: &Document) -> PageLayout {
        let mut layout = document.document_layout().clone();
        layout.right.layout.margins.left = 360;
        layout
    }

    fn run(document: &Document, command: Command) -> Document {
        apply(document, &command).unwrap().document
    }

    fn style<'a>(document: &'a Document, name: &str) -> &'a ParagraphStyle {
        document
            .styles()
            .values()
            .find(|style| style.name == name)
            .unwrap()
    }

    #[test]
    fn function_keys_go_to_the_lowest_free_one_but_f10() {
        let mut styles = st1().styles().clone();
        let keys = |styles: &BTreeMap<StyleId, ParagraphStyle>| next_function_key(styles);
        for (index, style) in styles.values_mut().enumerate() {
            style.kept.function_key = [2, 3, 4, 5, 6, 7][index % 6];
        }
        assert_eq!(keys(&styles), 8);
        let mut taken = styles.values().next().unwrap().clone();
        for key in [8, 9, 11, 12] {
            taken.id = StyleId(100 + u64::from(key));
            taken.kept.function_key = key;
            styles.insert(taken.id, taken.clone());
        }
        assert_eq!(keys(&styles), 0, "F10 is never given");
    }

    #[test]
    fn create_modify_and_rename_keep_names_unique_and_mark_the_document() {
        let document = st1();
        let created = run(
            &document,
            Command::CreateStyle {
                name: "Quote".into(),
                based_on: StyleId(2),
            },
        );
        let quote = style(&created, "Quote");
        assert_eq!(quote.font_size_points, 18.0);
        assert_eq!(quote.origin, StyleOrigin::Document);
        assert_eq!(quote.id, StyleId(7));
        // The next free function key, as Ami Pro's Create Style gives one
        // (`10a8:0000`; BL4).
        let taken: Vec<u8> = document
            .styles()
            .values()
            .map(|style| style.kept.function_key)
            .collect();
        assert!(!taken.contains(&quote.kept.function_key));
        assert_eq!(
            quote.kept.function_key,
            (2..=12)
                .find(|key| *key != 10 && !taken.contains(key))
                .unwrap()
        );
        assert_eq!(document.styles().len(), 6, "the old root is unchanged");
        for (name, based_on) in [("title", 1), ("", 1), (" Pad", 1), ("A@B", 1), ("New", 99)] {
            assert!(
                apply(
                    &document,
                    &Command::CreateStyle {
                        name: name.into(),
                        based_on: StyleId(based_on),
                    },
                )
                .is_err(),
                "{name:?}"
            );
        }
        // A sheet style changed here is stored in the document, and the
        // unknown values it carries stay.
        let mut title = style(&document, "Title").clone();
        title.bold = true;
        title.bullets.kept = [9, 8, 7, 6, 5, 4];
        let modified = run(
            &document,
            Command::ModifyStyle {
                style: title.clone(),
            },
        );
        assert_eq!(style(&modified, "Title").origin, StyleOrigin::Document);
        assert_eq!(style(&modified, "Title").bullets.kept, [9, 8, 7, 6, 5, 4]);
        let renamed = run(
            &modified,
            Command::RenameStyle {
                style: StyleId(2),
                name: "Big Title".into(),
            },
        );
        assert_eq!(renamed.styles()[&StyleId(2)].name, "Big Title");
        for name in ["Mine", "MINE"] {
            assert!(
                apply(
                    &renamed,
                    &Command::RenameStyle {
                        style: StyleId(2),
                        name: name.into(),
                    },
                )
                .is_err()
            );
        }
        assert!(
            apply(
                &renamed,
                &Command::RenameStyle {
                    style: StyleId(1),
                    name: "Body".into(),
                },
            )
            .is_err(),
            "Body Text keeps its name"
        );
        // Changing Body Text changes the styles defined nowhere.
        let mut body = style(&document, "Body Text").clone();
        body.font_size_points = 14.0;
        let bigger = run(&document, Command::ModifyStyle { style: body });
        assert_eq!(style(&bigger, "Ghost").font_size_points, 14.0);
    }

    #[test]
    fn removing_a_style_leaves_its_name_formatted_as_body_text_st1a() {
        let mut document = st1();
        document.styles_mut().get_mut(&StyleId(2)).unwrap().next = Some(StyleId(4));
        let removed = run(
            &document,
            Command::DeleteStyle {
                style: StyleId(4),
                sheet_version: None,
            },
        );
        let mine = style(&removed, "Mine");
        assert_eq!(mine.origin, StyleOrigin::Missing);
        assert_eq!(mine.font_family, style(&removed, "Body Text").font_family);
        assert_eq!(removed.paragraphs()[3].style, StyleId(4), "the tag stays");
        assert_eq!(style(&removed, "Title").next, None);
        // Subhead, changed in the document, goes back to the sheet's.
        let sheet_subhead = ParagraphStyle {
            id: StyleId(40),
            name: "Subhead".into(),
            font_size_points: 13.0,
            italic: true,
            ..style(&document, "Body Text").clone()
        };
        let reverted = run(
            &document,
            Command::DeleteStyle {
                style: StyleId(3),
                sheet_version: Some(sheet_subhead),
            },
        );
        let subhead = &reverted.styles()[&StyleId(3)];
        assert_eq!(
            (subhead.origin, subhead.font_size_points, subhead.italic),
            (StyleOrigin::Sheet, 13.0, true)
        );
        for (id, sheet) in [(1, None), (6, None), (99, None)] {
            assert!(
                apply(
                    &document,
                    &Command::DeleteStyle {
                        style: StyleId(id),
                        sheet_version: sheet,
                    },
                )
                .is_err(),
                "{id}"
            );
        }
    }

    #[test]
    fn style_commands_are_undo_steps() {
        // A style command reports its change, so that the history records
        // and applies it.
        let mut document = st1();
        let before = document.clone();
        let mut history: crate::History<()> = crate::History::new();
        history
            .execute(
                &mut document,
                (),
                &Command::CreateStyle {
                    name: "New".into(),
                    based_on: StyleId(1),
                },
            )
            .unwrap();
        assert!(style_named(&document, "New"));
        assert!(history.can_undo());
        history.undo(&mut document, ()).unwrap();
        assert_eq!(document, before);
    }

    fn style_named(document: &Document, name: &str) -> bool {
        document.styles().values().any(|style| style.name == name)
    }

    #[test]
    fn saving_a_style_sheet_links_a_saved_document_and_detaches_an_untitled_one() {
        let document = st1();
        let linked = run(
            &document,
            Command::SavedAsStyleSheet {
                name: Some("SD1N.STY".into()),
            },
        );
        assert_eq!(
            linked.style_sheet(),
            Some(&StyleSheetLink {
                name: "SD1N.STY".into(),
                layout_from_sheet: true,
            })
        );
        assert_eq!(style(&linked, "Mine").origin, StyleOrigin::Sheet);
        assert_eq!(style(&linked, "Subhead").origin, StyleOrigin::Sheet);
        assert_eq!(style(&linked, "Ghost").origin, StyleOrigin::Missing);
        let detached = run(&document, Command::SavedAsStyleSheet { name: None });
        assert_eq!(detached.style_sheet(), None);
        assert_eq!(style(&detached, "Title").origin, StyleOrigin::Document);
        assert_eq!(style(&detached, "Ghost").origin, StyleOrigin::Missing);
        assert!(
            apply(
                &document,
                &Command::SavedAsStyleSheet {
                    name: Some(" ".into())
                }
            )
            .is_err()
        );
    }

    #[test]
    fn another_style_sheet_keeps_document_styles_and_formats_the_rest_st1b() {
        let mut document = st1();
        document.legacy_mut().push("section [xyz]", "kept");
        let body = ParagraphStyle {
            id: StyleId(1),
            name: "Body Text".into(),
            font_family: "Arial".into(),
            font_size_points: 10.0,
            origin: StyleOrigin::Sheet,
            ..default_styles()[&StyleId(1)].clone()
        };
        let mut letter = document.document_layout().clone();
        letter.right.layout.margins.left = 2_000;
        let sheet = StyleSheet {
            name: "_basic.sty".into(),
            styles: vec![
                body.clone(),
                ParagraphStyle {
                    id: StyleId(2),
                    name: "Body Single".into(),
                    next: Some(StyleId(1)),
                    ..body.clone()
                },
            ],
            layout: Some(letter),
        };
        let used = run(&document, Command::UseStyleSheet { sheet });
        let font = |name| {
            let style = style(&used, name);
            (style.font_family.as_str(), style.origin)
        };
        assert_eq!(font("Body Text"), ("Arial", StyleOrigin::Sheet));
        assert_eq!(font("Title"), ("Arial", StyleOrigin::Missing));
        assert_eq!(font("Bullet"), ("Arial", StyleOrigin::Missing));
        assert_eq!(font("Ghost"), ("Arial", StyleOrigin::Missing));
        assert_eq!(font("Mine"), ("Courier New", StyleOrigin::Document));
        assert_eq!(style(&used, "Subhead").font_size_points, 18.0);
        // The sheet's other style joins, its next style mapped by name.
        let single = style(&used, "Body Single");
        assert_eq!(single.origin, StyleOrigin::Sheet);
        assert_eq!(single.next, Some(StyleId(1)));
        assert_eq!(used.paragraphs()[1].style, StyleId(2), "tags stay");
        assert_eq!(used.style_sheet().unwrap().name, "_basic.sty");
        assert_eq!(used.document_layout().right.layout.margins.left, 2_000);
        assert_eq!(used.legacy(), document.legacy());
        // A layout stored in the document stays.
        document.set_style_sheet(Some(StyleSheetLink {
            name: "_DEFAULT.STY".into(),
            layout_from_sheet: false,
        }));
        let sheet = StyleSheet {
            name: "_basic.sty".into(),
            styles: vec![body],
            layout: Some(letter_wide(&document)),
        };
        let kept = run(&document, Command::UseStyleSheet { sheet });
        assert_eq!(
            kept.document_layout().right.layout.margins,
            document.document_layout().right.layout.margins
        );
        // A sheet without styles is refused.
        let empty = StyleSheet {
            name: "_x.sty".into(),
            styles: Vec::new(),
            layout: None,
        };
        assert!(apply(&document, &Command::UseStyleSheet { sheet: empty }).is_err());
    }

    #[test]
    fn style_commands_survive_saving_and_reopening() {
        let document = st1();
        let edited = [
            Command::CreateStyle {
                name: "Quote".into(),
                based_on: StyleId(2),
            },
            Command::DeleteStyle {
                style: StyleId(4),
                sheet_version: None,
            },
            Command::RenameStyle {
                style: StyleId(5),
                name: "Point".into(),
            },
        ]
        .into_iter()
        .fold(document, |document, command| run(&document, command));
        let reopened = Document::decode_native(&edited.encode_native().unwrap()).unwrap();
        assert_eq!(reopened, edited);
        assert_eq!(style(&reopened, "Mine").origin, StyleOrigin::Missing);
        assert_eq!(style(&reopened, "Point").origin, StyleOrigin::Document);
    }
}
