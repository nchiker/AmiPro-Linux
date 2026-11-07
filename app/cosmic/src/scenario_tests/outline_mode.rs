//! Outline › Move Up and Move Down (run B, OB.2; Alt+Up, Alt+Down): a
//! heading moves with its section, a body paragraph alone, as one undo
//! step; the caret stays in the moved paragraph; in a table cell the same
//! keys move table rows; frames anchored in the moved text go with it.

use amipro_document::{InlineObject, StyleId};
use cosmic::iced::keyboard::{Key, Modifiers, key::Named};

use crate::style_properties::StyleAction;
use crate::table_commands::InsertTableAction;

use super::*;

/// The main text's paragraphs.
fn order(s: &Scenario) -> Vec<String> {
    s.document()
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect()
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|text| (*text).to_owned()).collect()
}

/// The caret's paragraph and byte (it must be a caret, in the main text).
fn caret_at(s: &Scenario) -> (usize, usize) {
    let selection = s.app().page_selection;
    assert!(selection.is_caret(), "a caret is expected: {selection:?}");
    assert_eq!(selection.focus.flow, s.main_flow());
    (selection.focus.paragraph, selection.focus.byte)
}

fn style_named(s: &Scenario, name: &str) -> StyleId {
    *s.document()
        .styles()
        .iter()
        .find(|(_, style)| style.name == name)
        .unwrap_or_else(|| s.fail("styles", &format!("no style {name:?}")))
        .0
}

/// A key as the window delivers it.
fn press(rt: &mut Runtime, key: Named, modifiers: Modifiers) {
    rt.app.page_keyboard += 1;
    rt.events(&[key_press(Key::Named(key), modifiers)]);
}

/// Outline › Move Up / Move Down from the menu, one undo step.
fn move_section(s: &mut Scenario, down: bool) {
    s.command(if down {
        CommandId::MoveDown
    } else {
        CommandId::MoveUp
    });
}

/// Gives Heading 1 and Heading 2 outline levels 1 and 2 (the new
/// document's styles have none), the way a user does: Bullet and Number
/// Style… in the Styles panel.
fn level_the_headings(s: &mut Scenario) {
    let main = s.main_flow();
    for (name, level, paragraph) in [("Heading 1", 1usize, 0usize), ("Heading 2", 2, 1)] {
        let id = style_named(s, name);
        s.caret(main, paragraph, 0, 1, false);
        s.edit(&format!("apply {name}"), move |rt| {
            rt.send(Message::ApplyStyle(id));
        });
        s.command_act(CommandId::BulletNumberStyle);
        assert_eq!(s.app().style_properties.as_ref().unwrap().style, id);
        s.edit(&format!("{name}: outline level {level}"), move |rt| {
            rt.send(Message::StyleProperties(StyleAction::OutlineLevel(level)));
        });
        assert_eq!(s.document().styles()[&id].bullets.level, level as u8);
    }
}

/// The document of the run's scenarios: two parts, each with subheads and
/// body text, in Heading 1 and Heading 2 with outline levels.
///
///  0 First title (level 1)
///  1 Subhead A (2)
///  2 Body A1
///  3 Body A2
///  4 Subhead B (2)
///  5 Body B
///  6 Second title (1)
///  7 Subhead C (2)
///  8 Body C
fn outline_document(name: &'static str) -> Scenario {
    let mut s = Scenario::new_document(name);
    s.clean_folder();
    let lines = [
        "First title",
        "Subhead A",
        "Body A1",
        "Body A2",
        "Subhead B",
        "Body B",
        "Second title",
        "Subhead C",
        "Body C",
    ];
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            s.enter();
        }
        s.type_text(line);
    }
    level_the_headings(&mut s);
    let main = s.main_flow();
    let one = style_named(&s, "Heading 1");
    let two = style_named(&s, "Heading 2");
    // (Level the headings already gave paragraphs 0 and 1 theirs.)
    for (paragraph, style) in [(4usize, two), (6, one), (7, two)] {
        s.caret(main, paragraph, 0, 1, false);
        s.edit(
            &format!("apply a heading style to {paragraph}"),
            move |rt| {
                rt.send(Message::ApplyStyle(style));
            },
        );
    }
    assert_eq!(order(&s), names(&lines));
    s
}

fn original() -> Vec<String> {
    names(&[
        "First title",
        "Subhead A",
        "Body A1",
        "Body A2",
        "Subhead B",
        "Body B",
        "Second title",
        "Subhead C",
        "Body C",
    ])
}

#[test]
fn sections_move_up_and_down_with_their_text_as_single_undo_steps_and_are_kept() {
    let mut s = outline_document("outline-move-sections");
    let main = s.main_flow();
    let mut history = vec![original()];

    // 2. Move Up from the middle of "Subhead B", by the key: it goes over
    // Subhead A's whole section; the caret stays at the same byte.
    s.caret(main, 4, 4, 1, false);
    s.edit("Alt+Up in Subhead B", |rt| {
        press(rt, Named::ArrowUp, Modifiers::ALT)
    });
    let up = names(&[
        "First title",
        "Subhead B",
        "Body B",
        "Subhead A",
        "Body A1",
        "Body A2",
        "Second title",
        "Subhead C",
        "Body C",
    ]);
    assert_eq!(order(&s), up);
    assert_eq!(caret_at(&s), (1, 4), "the caret is still in Subhead B");
    history.push(up);

    // 3. Move Down: back below Subhead A's section; once more: at the end
    // of its parent's text it goes over the next higher heading alone and
    // becomes the first section under "Second title".
    // (Not `edit`: its Undo and Redo leave their own status.)
    s.act("Move Down", |rt| {
        rt.send(Message::Command(CommandId::MoveDown));
    });
    assert_eq!(s.app().status, "Moved the section down");
    assert_eq!(order(&s), original());
    assert_eq!(caret_at(&s), (4, 4));
    history.push(original());
    move_section(&mut s, true);
    let under = names(&[
        "First title",
        "Subhead A",
        "Body A1",
        "Body A2",
        "Second title",
        "Subhead B",
        "Body B",
        "Subhead C",
        "Body C",
    ]);
    assert_eq!(order(&s), under);
    assert_eq!(caret_at(&s), (5, 4), "the caret went with Subhead B");
    history.push(under);

    // 4. "Second title" Up: its whole section goes above the first title.
    s.caret(main, 4, 3, 1, false);
    move_section(&mut s, false);
    let swapped = names(&[
        "Second title",
        "Subhead B",
        "Body B",
        "Subhead C",
        "Body C",
        "First title",
        "Subhead A",
        "Body A1",
        "Body A2",
    ]);
    assert_eq!(order(&s), swapped);
    assert_eq!(caret_at(&s), (0, 3));
    history.push(swapped.clone());
    // At the start: refused, nothing changes, the status says so.
    s.no_change("Move Up at the start", |rt| {
        rt.send(Message::Command(CommandId::MoveUp));
    });
    assert_eq!(s.app().status, "The section is already at the start");
    assert_eq!(order(&s), swapped);
    assert_eq!(caret_at(&s), (0, 3));
    s.no_change("Alt+Up at the start", |rt| {
        press(rt, Named::ArrowUp, Modifiers::ALT)
    });
    assert_eq!(order(&s), swapped);
    // The last section Down is refused too: "First title" is the last
    // heading-1 section and is at the end of the flow.
    s.caret(main, 5, 0, 1, false);
    s.no_change("Move Down at the end", |rt| {
        rt.send(Message::Command(CommandId::MoveDown));
    });
    assert_eq!(s.app().status, "The section is already at the end");
    assert_eq!(order(&s), swapped);

    // 5. A body paragraph moves alone: "Body A1" Down swaps with the next
    // paragraph only.
    s.caret(main, 7, 3, 1, false);
    move_section(&mut s, true);
    let alone = names(&[
        "Second title",
        "Subhead B",
        "Body B",
        "Subhead C",
        "Body C",
        "First title",
        "Subhead A",
        "Body A2",
        "Body A1",
    ]);
    assert_eq!(order(&s), alone);
    assert_eq!(caret_at(&s), (8, 3), "the caret is in Body A1");
    history.push(alone);

    // 6. Undo takes the moves back one at a time; Redo brings one back.
    let steps = history.len() - 1;
    for back in (0..steps).rev() {
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        assert_eq!(order(&s), history[back], "after undoing to step {back}");
    }
    assert_eq!(order(&s), original());
    s.act("Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert_eq!(order(&s), history[1]);

    // 7. Save As SAM and reopen: the order reads back the same, and the
    // headings keep their levels (a move after reopening still works).
    let saved = order(&s);
    let path = s.save_sam("OUTMOVE.SAM");
    let mut s = s.reopen(&path);
    assert_eq!(order(&s), saved);
    let levels = amipro_editing::sections::levels(s.document(), s.main_flow());
    assert_eq!(levels, [1, 2, 0, 2, 0, 0, 1, 2, 0], "{levels:?}");
    let main = s.main_flow();
    s.caret(main, 1, 2, 1, false);
    move_section(&mut s, true);
    assert_eq!(
        order(&s)[1],
        "Subhead A",
        "Subhead B moved down over Subhead A"
    );
}

/// Types `text` in the cell at the caret (not undo-checked: cell typing
/// is checked by the table scenarios).
fn fill(s: &mut Scenario, text: &str) {
    let text = text.to_owned();
    s.act(&format!("type {text:?} in the cell"), move |rt| {
        rt.send(Message::Page(PageAction::Insert(text)));
    });
}

fn table_frame(s: &Scenario) -> FrameId {
    *s.document()
        .frame_stack()
        .iter()
        .rev()
        .find(|id| {
            matches!(
                s.document().frame(**id).unwrap().content,
                FrameContent::Table(_)
            )
        })
        .unwrap_or_else(|| s.fail("table", "the document has no table"))
}

fn column_of(s: &Scenario, rows: usize) -> Vec<String> {
    let frame = table_frame(s);
    (0..rows)
        .map(|row| flow_text(s.app(), cell(s.document(), frame, row, 0)))
        .collect()
}

#[test]
fn in_a_table_cell_alt_up_and_down_move_the_table_rows_not_the_paragraphs() {
    let mut s = Scenario::new_document("outline-move-table");
    s.clean_folder();
    for (index, line) in ["Part one", "Intro", "Part two", "Outro"]
        .iter()
        .enumerate()
    {
        if index > 0 {
            s.enter();
        }
        s.type_text(line);
    }
    level_the_headings(&mut s);
    let main = s.main_flow();
    // Part one is level 1; make Part two level 1 as well, Intro and Outro
    // body text.
    let one = style_named(&s, "Heading 1");
    s.caret(main, 2, 0, 1, false);
    s.edit("apply Heading 1 to Part two", move |rt| {
        rt.send(Message::ApplyStyle(one));
    });
    // The table goes at the start of "Outro" (Insert › Table…).
    s.caret(main, 3, 0, 1, false);
    s.command_act(CommandId::Table);
    s.act("a table of 2 columns and 3 rows", |rt| {
        rt.send(Message::InsertTable(InsertTableAction::Columns("2".into())));
        rt.send(Message::InsertTable(InsertTableAction::Rows("3".into())));
        rt.send(Message::InsertTable(InsertTableAction::Insert));
    });
    let frame = table_frame(&s);
    for row in 0..3 {
        let flow = cell(s.document(), frame, row, 0);
        let page = 1;
        s.caret(flow, 0, 0, page, false);
        fill(&mut s, &format!("row {}", row + 1));
    }
    assert_eq!(column_of(&s, 3), ["row 1", "row 2", "row 3"]);
    let paragraphs = order(&s);

    // The caret in row 2's cell: Alt+Up moves the row, not a section.
    let flow = cell(s.document(), frame, 1, 0);
    s.caret(flow, 0, 0, 1, false);
    s.act("Alt+Up in row 2", |rt| {
        press(rt, Named::ArrowUp, Modifiers::ALT)
    });
    assert_eq!(column_of(&s, 3), ["row 2", "row 1", "row 3"]);
    assert_eq!(order(&s), paragraphs, "the paragraphs did not move");
    s.act("Alt+Down in the moved row", |rt| {
        press(rt, Named::ArrowDown, Modifiers::ALT)
    });
    assert_eq!(column_of(&s, 3), ["row 1", "row 2", "row 3"]);
    assert_eq!(order(&s), paragraphs);

    // The menu command in a cell does the same as the key (the page takes
    // Alt+Up for rows; the command is the same one).
    s.act("Move Up from the menu in a cell", |rt| {
        rt.send(Message::Command(CommandId::MoveUp));
    });
    assert_eq!(order(&s), paragraphs, "no paragraph moved in the cell");

    // Outside the table the same key moves sections again.
    let before = order(&s);
    s.caret(main, 2, 0, 1, false);
    s.edit("Alt+Up outside the table moves Part two", |rt| {
        press(rt, Named::ArrowUp, Modifiers::ALT)
    });
    let after = order(&s);
    assert_ne!(after, before);
    assert_eq!(after[0], "Part two", "{after:?}");

    let path = s.save_sam("OUTTABLE.SAM");
    let s = s.reopen(&path);
    assert_eq!(order(&s), after);
}

#[test]
fn a_with_para_above_frame_goes_with_the_section_it_is_anchored_in() {
    use crate::frame_inspector::{Choice, PlacementKind};
    use crate::page_dialogs::{CreateFrameAction, GeometryField};
    let mut s = outline_document("outline-move-frame");
    let main = s.main_flow();

    // A frame just above "Body B"'s line, made With para above: its
    // anchor is the first line at or below its top.
    let top = {
        let page = s.app().page_box(1).unwrap();
        let line = page
            .lines
            .iter()
            .find(|line| {
                line.container == amipro_layout::Container::Main && line.paragraph_index == 5
            })
            .unwrap_or_else(|| s.fail("layout", "no line for Body B"));
        (line.top - 10.0) / 1440.0
    };
    let fields = [
        (GeometryField::Width, "1.50".to_owned()),
        (GeometryField::Height, "0.50".to_owned()),
        (GeometryField::Top, format!("{top:.3}")),
        (GeometryField::Left, "4.50".to_owned()),
    ];
    s.edit("Insert › Frame…", move |rt| {
        rt.send(Message::Command(CommandId::Frame));
        for (field, text) in fields {
            rt.send(Message::CreateFrame(CreateFrameAction::Input(field, text)));
        }
        rt.send(Message::CreateFrame(CreateFrameAction::Create));
    });
    let frame = *s.document().frame_stack().last().unwrap();
    s.act("select the frame", move |rt| {
        rt.send(Message::Frame(crate::frame_editing::FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        }));
    });
    s.edit("Placement: With para above", |rt| {
        rt.send(Message::InspectorChoice(Choice::Placement(
            PlacementKind::WithParaAbove,
        )));
    });
    let anchor = |s: &Scenario| {
        s.document().paragraphs().iter().position(|paragraph| {
            paragraph
                .objects
                .iter()
                .any(|object| matches!(object, InlineObject::FrameAnchor(f) if *f == frame))
        })
    };
    let at = anchor(&s).unwrap_or_else(|| s.fail("frame", "the frame has no anchor"));
    assert_eq!(
        s.document().paragraphs()[at].text.replace('\u{FFFC}', ""),
        "Body B",
        "the frame is anchored in Body B (paragraph {at})"
    );

    // Subhead B Up: the frame's anchor goes with its section.
    s.caret(main, 4, 2, 1, false);
    assert_eq!(s.app().frame_selection, None, "the caret leaves the frame");
    move_section(&mut s, false);
    let at = anchor(&s).unwrap_or_else(|| s.fail("frame", "the anchor is gone"));
    assert_eq!(at, 2, "Body B follows Subhead B (now paragraph 1)");
    assert_eq!(
        s.document().paragraphs()[at].text.replace('\u{FFFC}', ""),
        "Body B"
    );
    assert_eq!(s.document().paragraphs()[1].text, "Subhead B");
    assert!(s.document().frame(frame).is_some());

    // And Down again: back where it was.
    move_section(&mut s, true);
    assert_eq!(anchor(&s), Some(5));

    let path = s.save_sam("OUTFRAME.SAM");
    let s = s.reopen(&path);
    assert_eq!(s.document().frame_stack().len(), 1);
}

// ---- View › Outline Mode (OB.3) ---------------------------------------------

fn outline_checked(s: &Scenario) -> bool {
    crate::menu_check(s.app(), CommandId::OutlineMode)
        .unwrap_or_else(|| s.fail("menu", "Outline Mode has no check mark"))
}

/// The x of paragraph `index`'s first word in the laid-out main text.
fn first_word_x(s: &Scenario, index: usize) -> f32 {
    s.app()
        .pages
        .layout()
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .find(|line| {
            line.container == amipro_layout::Container::Main && line.paragraph_index == index
        })
        .and_then(|line| line.words.first())
        .unwrap_or_else(|| s.fail("layout", &format!("no line for paragraph {index}")))
        .x
}

/// The outline levels of the main text as the document has them now.
fn main_levels(s: &Scenario) -> Vec<u8> {
    amipro_editing::sections::levels(s.document(), s.main_flow())
}

/// The icons the mode must show, worked out from the levels alone: a
/// heading has a plus when the next paragraph is body text or a deeper
/// heading, a minus when it is not; body text a box.
fn expected_icons(levels: &[u8]) -> Vec<crate::outline_view::OutlineIcon> {
    use crate::outline_view::OutlineIcon::*;
    (0..levels.len())
        .map(|index| match levels[index] {
            0 => Body,
            level => match levels.get(index + 1) {
                Some(&next) if next == 0 || next > level => Plus,
                _ => Minus,
            },
        })
        .collect()
}

/// Every main paragraph's first word stands at the margin plus its indent.
fn assert_indents(s: &Scenario) {
    let levels = main_levels(s);
    for index in 0..levels.len() {
        let wanted =
            (crate::outline_view::MARGIN + crate::outline_view::indent(&levels, index)) as f32;
        let x = first_word_x(s, index);
        assert!(
            (x - wanted).abs() < 0.5,
            "paragraph {index} ({:?}, level {}) starts at {x}, not {wanted}",
            s.document().paragraphs()[index].text,
            levels[index]
        );
    }
}

/// The file's lines, `[desc]`'s save times (each save's own second)
/// blanked so that two saves compare by what they hold.
fn saved_lines(path: &Path) -> Vec<String> {
    let mut in_desc = false;
    String::from_utf8_lossy(&std::fs::read(path).unwrap())
        .lines()
        .map(|line| {
            if line.starts_with('[') {
                in_desc = line == "[desc]";
            }
            let value = line.trim();
            if in_desc && value.len() >= 9 && value.chars().all(|c| c.is_ascii_digit()) {
                "\t(time)".to_owned()
            } else {
                line.to_owned()
            }
        })
        .collect()
}

#[test]
fn outline_mode_shows_levels_and_icons_edits_in_step_is_kept_per_tab_and_never_saved() {
    use crate::outline_view::{MARGIN, OutlineIcon::*};
    let mut s = outline_document("outline-mode-view");
    let main = s.main_flow();

    // A last heading with nothing under it (a minus).
    let two = style_named(&s, "Heading 2");
    s.caret_to_end();
    s.enter();
    s.type_text("Subhead D");
    s.edit("apply Heading 2 to the last paragraph", move |rt| {
        rt.send(Message::ApplyStyle(two));
    });
    let lines_before = order(&s);
    assert_eq!(lines_before.len(), 10);
    assert_eq!(main_levels(&s), [1, 2, 0, 0, 2, 0, 1, 2, 0, 2]);

    // Heading 1 is centred, as many a document's titles are.
    let one = style_named(&s, "Heading 1");
    s.caret(main, 0, 0, 1, false);
    s.command_act(CommandId::BulletNumberStyle);
    assert_eq!(s.app().style_properties.as_ref().unwrap().style, one);
    s.edit("Heading 1: centred", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Align(
            amipro_document::Alignment::Centre,
        )));
    });
    assert_eq!(
        s.document().styles()[&one].alignment,
        amipro_document::Alignment::Centre
    );
    let layout_x = first_word_x(&s, 0);
    assert!(
        layout_x > MARGIN as f32 + 100.0,
        "centred in Layout mode: {layout_x}"
    );

    // 1. View › Outline Mode: the document is not changed.
    assert!(!outline_checked(&s));
    s.no_change("View › Outline Mode", |rt| {
        rt.send(Message::Command(CommandId::OutlineMode));
    });
    assert!(s.app().outline.is_some(), "Outline mode is on");
    assert!(s.app().draft.is_none());
    assert!(outline_checked(&s));
    assert_eq!(s.app().status, "Outline mode: the text by outline level");
    assert_eq!(
        s.app().outline_icons,
        [Plus, Plus, Body, Body, Plus, Body, Plus, Plus, Body, Minus]
    );
    assert_eq!(s.app().outline_icons, expected_icons(&main_levels(&s)));

    // 2. The lines: each paragraph's first word at the margin plus its
    // indent; the headings left aligned whatever their style says.
    assert_indents(&s);
    assert_eq!(first_word_x(&s, 0), MARGIN as f32, "First title: level 1");
    assert_eq!(
        first_word_x(&s, 1),
        (MARGIN + 480) as f32,
        "Subhead A: level 2"
    );
    assert_eq!(first_word_x(&s, 2), (MARGIN + 720) as f32, "Body A1");
    assert_ne!(first_word_x(&s, 0), layout_x, "left aligned, not centred");
    // The document's own style keeps its alignment: the view's is a copy.
    assert_eq!(
        s.document().styles()[&one].alignment,
        amipro_document::Alignment::Centre
    );

    // 3. Typing in a body paragraph, in the middle: one step; the icons and
    // the indent do not change.
    let icons = s.app().outline_icons.clone();
    let indent = first_word_x(&s, 2);
    s.caret(main, 2, 4, 1, false);
    s.type_text("XY");
    assert_eq!(s.document().paragraphs()[2].text, "BodyXY A1");
    assert_eq!(s.app().outline_icons, icons);
    assert_eq!(first_word_x(&s, 2), indent);
    assert_indents(&s);

    // 4. Alt+Left on that paragraph: it takes a heading's style; its icon
    // and indent follow its new level; Undo restores both.
    let before_levels = main_levels(&s);
    let before_icons = s.app().outline_icons.clone();
    s.edit("Alt+Left (Promote) in Outline mode", |rt| {
        rt.app.page_keyboard += 1;
        rt.events(&[key_press(Key::Named(Named::ArrowLeft), Modifiers::ALT)]);
    });
    let levels = main_levels(&s);
    assert!(levels[2] > 0, "the paragraph became a heading: {levels:?}");
    assert_ne!(levels, before_levels);
    assert_ne!(s.app().outline_icons[2], Body, "its icon is a heading's");
    assert_eq!(s.app().outline_icons, expected_icons(&levels));
    assert_indents(&s);
    let promoted_x = first_word_x(&s, 2);
    assert_ne!(promoted_x, indent, "the indent changed with the level");
    s.act("Undo the promotion", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(main_levels(&s), before_levels);
    assert_eq!(s.app().outline_icons, before_icons);
    assert_eq!(first_word_x(&s, 2), indent);
    s.act("Redo the promotion", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert_eq!(first_word_x(&s, 2), promoted_x);
    // Take it back (a Demote), so the rest works on the first outline.
    s.act("Undo the promotion again", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(main_levels(&s), before_levels);

    // 5. Move Up on a heading in Outline mode: its section goes over the
    // one above, as in Layout mode.
    let moved_from = order(&s);
    s.caret(main, 4, 3, 1, false);
    s.edit("Alt+Up on Subhead B in Outline mode", |rt| {
        press(rt, Named::ArrowUp, Modifiers::ALT)
    });
    let after_move = order(&s);
    assert_eq!(
        after_move,
        names(&[
            "First title",
            "Subhead B",
            "Body B",
            "Subhead A",
            "BodyXY A1",
            "Body A2",
            "Second title",
            "Subhead C",
            "Body C",
            "Subhead D",
        ])
    );
    assert_ne!(after_move, moved_from);
    assert_eq!(s.app().outline_icons, expected_icons(&main_levels(&s)));
    assert_indents(&s);

    // 6. Draft Mode turns Outline off; Outline Mode turns Draft off.
    s.command_act(CommandId::DraftMode);
    assert!(s.app().outline.is_none() && s.app().draft.is_some());
    assert!(!outline_checked(&s));
    s.command_act(CommandId::OutlineMode);
    assert!(s.app().outline.is_some() && s.app().draft.is_none());
    assert!(outline_checked(&s));
    assert_indents(&s);

    // 7. A second tab starts in Layout mode; the first is still in Outline
    // mode when it is back.
    s.command_act(CommandId::NewDocument);
    assert!(s.app().outline.is_none() && s.app().draft.is_none());
    assert!(!outline_checked(&s));
    s.act("back to the first tab", |rt| {
        rt.send(Message::ActivateTab(0));
    });
    assert!(
        s.app().outline.is_some(),
        "the first tab is still in Outline mode"
    );
    assert!(outline_checked(&s));
    assert_eq!(s.document().paragraphs().len(), 10);
    assert_indents(&s);

    // 8. Saved in Outline mode and in Layout mode, the files are the same:
    // no indent or alignment of the view is written.
    let saved = order(&s);
    let in_outline = s.save_sam("OUTVIEW1.SAM");
    s.command_act(CommandId::OutlineMode);
    assert!(s.app().outline.is_none());
    let in_layout = s.save_sam("OUTVIEW2.SAM");
    let (a, b) = (saved_lines(&in_outline), saved_lines(&in_layout));
    let differing: Vec<_> = a
        .iter()
        .zip(&b)
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(line, (x, y))| format!("line {line}: {x:?} vs {y:?}"))
        .collect();
    assert_eq!(a.len(), b.len(), "the files differ in length");
    assert!(
        differing.is_empty(),
        "the mode wrote something: {differing:#?}"
    );
    // (The document read back from each is the same.)
    let read = |path: &Path| {
        amipro_formats::import_sam_path_with_password(path, &[], None)
            .unwrap()
            .document
    };
    let (outline_doc, layout_doc) = (read(&in_outline), read(&in_layout));
    for (id, style) in layout_doc.styles() {
        let other = &outline_doc.styles()[id];
        assert_eq!(style.alignment, other.alignment, "{}", style.name);
        assert_eq!(style.spacing, other.spacing, "{}", style.name);
        assert_eq!(style.indents, other.indents, "{}", style.name);
    }
    for (x, y) in layout_doc.paragraphs().iter().zip(outline_doc.paragraphs()) {
        assert_eq!(x.format, y.format, "{:?}", x.text);
    }

    // 9. The reopened file is in Layout mode.
    let s = s.reopen(&in_outline);
    assert!(s.app().outline.is_none() && s.app().draft.is_none());
    assert!(!outline_checked(&s));
    assert_eq!(order(&s), saved);
}

// ---- Expand, Contract and Show levels (OB.4) --------------------------------

/// The laid-out line of main paragraph `index`: its page, top, height, and
/// whether it holds any words (a paragraph has one line in these scenarios).
fn line_of(s: &Scenario, index: usize) -> (usize, f32, f32, bool) {
    let lines: Vec<_> = s
        .app()
        .pages
        .layout()
        .pages
        .iter()
        .enumerate()
        .flat_map(|(page, content)| content.lines.iter().map(move |line| (page, line)))
        .filter(|(_, line)| {
            line.container == amipro_layout::Container::Main && line.paragraph_index == index
        })
        .collect();
    assert_eq!(lines.len(), 1, "paragraph {index} has one line");
    let (page, line) = lines[0];
    (page, line.top, line.height, !line.words.is_empty())
}

/// Each main paragraph is hidden exactly as `hidden` says, in the flags, in
/// the layout (an empty line under 2 twips high), and in the icons (a
/// heading above a hidden paragraph shows contracted).
fn assert_hidden(s: &Scenario, hidden: &[bool]) {
    use crate::outline_view::OutlineIcon::*;
    assert_eq!(s.app().outline_hidden, hidden, "the hidden flags");
    for (index, &is_hidden) in hidden.iter().enumerate() {
        let (_, _, height, words) = line_of(s, index);
        if is_hidden {
            assert!(
                height < 2.0 && !words,
                "paragraph {index} is hidden: {height}"
            );
        } else {
            assert!(
                height > 100.0 && words,
                "paragraph {index} is shown: {height}"
            );
        }
    }
    let levels = main_levels(s);
    let icons = s.app().outline_icons.clone();
    for (index, icon) in icons.iter().enumerate() {
        if levels[index] > 0 && hidden.get(index + 1).copied().unwrap_or(false) {
            assert_eq!(*icon, Contracted, "heading {index} is contracted");
        } else {
            assert_ne!(*icon, Contracted, "paragraph {index} is not contracted");
        }
    }
}

/// The caret, which must be a shown paragraph's.
fn assert_caret_shown(s: &Scenario) -> (usize, usize) {
    let (paragraph, byte) = caret_at(s);
    assert!(
        !s.app().outline_hidden[paragraph],
        "the caret is in hidden paragraph {paragraph}"
    );
    (paragraph, byte)
}

#[test]
fn contract_expand_and_show_levels_hide_sections_move_the_caret_and_are_never_saved() {
    use crate::outline_view::OutlineIcon::*;
    let mut s = outline_document("outline-contract");
    let main = s.main_flow();
    let none = [false; 9];
    s.command_act(CommandId::OutlineMode);
    assert!(s.app().outline.is_some());
    assert_hidden(&s, &none);

    // 1. Contract on the first title, by Alt+PageUp: not an edit, no undo
    // step; its whole section (to the next title) is hidden, the title's
    // icon is contracted, and the second title's line moves up.
    s.caret(main, 0, 0, 1, false);
    let undo = s.app().history.undo_len();
    let second_before = line_of(&s, 6);
    let first_before = line_of(&s, 0);
    s.no_change("Alt+PageUp (Contract) on the first title", |rt| {
        press(rt, Named::PageUp, Modifiers::ALT)
    });
    assert_eq!(s.app().history.undo_len(), undo, "Contract is not an edit");
    assert_eq!(s.app().status, "Contracted the heading's section");
    let mut hidden = [false; 9];
    hidden[1..=5].fill(true);
    assert_hidden(&s, &hidden);
    assert_eq!(s.app().outline_icons[0], Contracted);
    assert_eq!(s.app().outline_icons[6], Plus, "the second title is open");
    let second_after = line_of(&s, 6);
    assert_eq!(second_after.0, second_before.0);
    assert!(
        second_after.1 < second_before.1 - 500.0,
        "the second title moved up: {} to {}",
        second_before.1,
        second_after.1
    );
    assert_eq!(line_of(&s, 0).1, first_before.1, "the first title stays");
    assert_eq!(assert_caret_shown(&s), (0, 0));

    // 2. Alt+PageDown (Expand): everything is shown again, the icon a plus.
    s.no_change("Alt+PageDown (Expand) on the first title", |rt| {
        press(rt, Named::PageDown, Modifiers::ALT)
    });
    assert_eq!(s.app().history.undo_len(), undo);
    assert_eq!(s.app().status, "Expanded the heading's section");
    assert_hidden(&s, &none);
    assert_eq!(s.app().outline_icons[0], Plus);
    assert_eq!(
        line_of(&s, 6).1,
        second_before.1,
        "the second title is back"
    );

    // 3. The caret never stays in hidden text. Subhead A is contracted; a
    // click (the caret helper) in its hidden Body A1 lands in the next
    // shown paragraph, Subhead B, as the caret was moving forward.
    s.caret(main, 1, 0, 1, false);
    s.no_change("Contract Subhead A", |rt| {
        rt.send(Message::Command(CommandId::OutlineContract));
    });
    let mut hidden = [false; 9];
    hidden[2..=3].fill(true);
    assert_hidden(&s, &hidden);
    assert_eq!(s.app().outline_icons[1], Contracted);
    assert_eq!(assert_caret_shown(&s), (1, 0));
    s.caret(main, 2, 3, 1, false);
    assert_eq!(
        assert_caret_shown(&s),
        (4, 0),
        "the caret moved on to Subhead B"
    );
    // On body text, Contract and Expand do nothing.
    s.caret(main, 5, 0, 1, false);
    let status = s.app().status.clone();
    s.no_change("Contract on Body B", |rt| {
        rt.send(Message::Command(CommandId::OutlineContract));
    });
    s.no_change("Expand on Body B", |rt| {
        rt.send(Message::Command(CommandId::OutlineExpand));
    });
    assert_hidden(&s, &hidden);
    assert_eq!(s.app().status, status, "nothing happened to say");

    // 4. Show 1 Level: only the titles show (and the marks of 3 go); the
    // caret that was in Body B goes back to the title above it.
    s.no_change("Show 1 Level", |rt| {
        rt.send(Message::Command(CommandId::ShowLevel1));
    });
    let mut level_one = [true; 9];
    level_one[0] = false;
    level_one[6] = false;
    assert_hidden(&s, &level_one);
    assert!(s.app().outline_state.shown.is_empty() && s.app().outline_state.hidden.is_empty());
    assert!(crate::menu_check(s.app(), CommandId::ShowLevel1).unwrap());
    assert!(!crate::menu_check(s.app(), CommandId::ShowAllLevels).unwrap());
    assert!(!crate::menu_check(s.app(), CommandId::ShowLevel2).unwrap());
    assert_eq!(s.app().outline_icons[0], Contracted);
    assert_eq!(s.app().outline_icons[6], Contracted);
    assert_eq!(assert_caret_shown(&s).0, 0, "back to the first title");
    // Expand on the first title: its whole section shows, body text too,
    // while the second title's stays hidden.
    s.caret(main, 0, 0, 1, false);
    s.no_change("Expand on the first title", |rt| {
        rt.send(Message::Command(CommandId::OutlineExpand));
    });
    let mut expanded = [false; 9];
    expanded[7..=8].fill(true);
    assert_hidden(&s, &expanded);
    assert_eq!(s.app().outline_icons[0], Plus);
    assert_eq!(s.app().outline_icons[6], Contracted);
    // Show All Levels clears the marks: everything shows.
    s.no_change("Show All Levels", |rt| {
        rt.send(Message::Command(CommandId::ShowAllLevels));
    });
    assert_hidden(&s, &none);
    assert!(s.app().outline_state.shown.is_empty() && s.app().outline_state.hidden.is_empty());
    assert!(crate::menu_check(s.app(), CommandId::ShowAllLevels).unwrap());
    assert!(!crate::menu_check(s.app(), CommandId::ShowLevel1).unwrap());

    // 5. Show 2 Levels: the headings show, body text is hidden; the arrow
    // keys never put the caret in hidden text.
    s.no_change("Show 2 Levels", |rt| {
        rt.send(Message::Command(CommandId::ShowLevel2));
    });
    let two = [false, false, true, true, false, true, false, false, true];
    assert_hidden(&s, &two);
    assert!(crate::menu_check(s.app(), CommandId::ShowLevel2).unwrap());
    s.caret(main, 1, 0, 1, false);
    let mut seen = vec![1];
    for _ in 0..8 {
        s.no_change("Down arrow", |rt| {
            press(rt, Named::ArrowDown, Modifiers::empty())
        });
        let (paragraph, _) = assert_caret_shown(&s);
        seen.push(paragraph);
    }
    assert_eq!(
        &seen[..4],
        [1, 4, 6, 7],
        "Down skips the hidden text: {seen:?}"
    );
    assert!(seen.windows(2).all(|pair| pair[0] <= pair[1]), "{seen:?}");
    assert_eq!(
        *seen.last().unwrap(),
        7,
        "stays on the last shown: {seen:?}"
    );
    let mut seen = Vec::new();
    for _ in 0..8 {
        s.no_change("Up arrow", |rt| {
            press(rt, Named::ArrowUp, Modifiers::empty())
        });
        seen.push(assert_caret_shown(&s).0);
    }
    assert_eq!(&seen[..3], [6, 4, 1], "Up skips the hidden text: {seen:?}");
    assert_eq!(*seen.last().unwrap(), 0, "{seen:?}");

    // 6. Typing in a shown heading while others are hidden: one undo step;
    // the hidden paragraphs keep their text.
    let hidden_text: Vec<String> = order(&s)
        .into_iter()
        .zip(two)
        .filter(|(_, hidden)| *hidden)
        .map(|(text, _)| text)
        .collect();
    s.caret(main, 4, 9, 1, false);
    let undo = s.app().history.undo_len();
    s.type_text("ab");
    assert_eq!(s.app().history.undo_len(), undo + 1, "one undo step");
    assert_eq!(s.document().paragraphs()[4].text, "Subhead Bab");
    assert_hidden(&s, &two);
    let after: Vec<String> = order(&s)
        .into_iter()
        .zip(two)
        .filter(|(_, hidden)| *hidden)
        .map(|(text, _)| text)
        .collect();
    assert_eq!(after, hidden_text, "hidden text is untouched");
    assert_eq!(assert_caret_shown(&s), (4, 11));
    s.act("Undo the typing", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(s.document().paragraphs()[4].text, "Subhead B");
    assert_eq!(s.app().history.undo_len(), undo, "the typing was one step");
    s.act("Redo the typing", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert_eq!(s.document().paragraphs()[4].text, "Subhead Bab");
    assert_hidden(&s, &two);

    // 7. Move Down on a contracted title: the section goes under the second
    // title's, hidden paragraphs with it, still hidden (the marks follow
    // the paragraphs).
    s.command_act(CommandId::ShowAllLevels);
    assert_hidden(&s, &none);
    let before_move = order(&s);
    s.caret(main, 0, 0, 1, false);
    s.no_change("Contract the first title", |rt| {
        rt.send(Message::Command(CommandId::OutlineContract));
    });
    s.command(CommandId::MoveDown);
    let moved = order(&s);
    let expected: Vec<String> = before_move[6..]
        .iter()
        .chain(&before_move[..6])
        .cloned()
        .collect();
    assert_eq!(moved, expected, "the first title's section moved down");
    let mut hidden = [false; 9];
    hidden[4..=8].fill(true);
    assert_hidden(&s, &hidden);
    assert_eq!(s.app().outline_icons[3], Contracted, "First title");
    assert_eq!(assert_caret_shown(&s), (3, 0), "the caret went with it");

    // 8. Layout mode shows everything; Outline mode again has the
    // contraction, kept with the tab.
    s.no_change("View › Outline Mode off", |rt| {
        rt.send(Message::Command(CommandId::OutlineMode));
    });
    assert!(s.app().outline.is_none());
    for index in 0..9 {
        let (_, _, height, words) = line_of(&s, index);
        assert!(
            height > 100.0 && words,
            "Layout mode line {index}: {height}"
        );
    }
    assert!(!s.app().outline_state.hidden.is_empty(), "the state lives");
    s.no_change("View › Outline Mode on again", |rt| {
        rt.send(Message::Command(CommandId::OutlineMode));
    });
    assert!(s.app().outline.is_some());
    assert_hidden(&s, &hidden);
    assert_eq!(s.app().outline_icons[3], Contracted);

    // 9. Save As SAM with the contraction: nothing hidden is lost, and the
    // file is the one Layout mode writes; a reopened file shows all.
    let saved = order(&s);
    let in_outline = s.save_sam("OUTHIDE1.SAM");
    s.command_act(CommandId::OutlineMode);
    let in_layout = s.save_sam("OUTHIDE2.SAM");
    assert_eq!(saved_lines(&in_outline), saved_lines(&in_layout));
    let read = amipro_formats::import_sam_path_with_password(&in_outline, &[], None)
        .unwrap()
        .document;
    let texts: Vec<String> = read.paragraphs().iter().map(|p| p.text.clone()).collect();
    assert_eq!(texts, saved, "every paragraph is in the file");
    let t = s.reopen(&in_outline);
    assert!(t.app().outline.is_none());
    assert!(t.app().outline_state.hidden.is_empty());
    for index in 0..9 {
        let (_, _, height, words) = line_of(&t, index);
        assert!(height > 100.0 && words, "reopened line {index}: {height}");
    }

    // 10. Outside Outline mode, Contract does nothing and says why.
    s.caret(main, 3, 0, 1, false);
    let marks = s.app().outline_state.clone();
    s.no_change("Contract in Layout mode", |rt| {
        rt.send(Message::Command(CommandId::OutlineContract));
    });
    assert!(
        s.app().status.contains("work in Outline mode"),
        "{:?}",
        s.app().status
    );
    assert_eq!(s.app().outline_state, marks, "no mark was made");
    s.no_change("Show 1 Level in Layout mode", |rt| {
        rt.send(Message::Command(CommandId::ShowLevel1));
    });
    assert!(s.app().status.contains("works in Outline mode"));
    assert_eq!(s.app().outline_state, marks);
    for index in 0..9 {
        let (_, _, height, _) = line_of(&s, index);
        assert!(height > 100.0, "Layout mode line {index}");
    }
}

// ---- The Outline navigator (OB.6) ---------------------------------------------

/// The navigator's listed headings: paragraph, level, text.
fn listed(s: &Scenario) -> Vec<(usize, u8, String)> {
    crate::outline_navigator::entries(s.document(), &s.app().outline_state)
        .into_iter()
        .map(|entry| (entry.paragraph, entry.level, entry.text))
        .collect()
}

/// The heading the navigator draws highlighted for the caret now.
fn current_heading(s: &Scenario) -> Option<usize> {
    let entries = crate::outline_navigator::entries(s.document(), &s.app().outline_state);
    crate::outline_navigator::current(&entries, caret_at(s).0)
}

fn navigator_collapsed(s: &Scenario, paragraph: usize) -> bool {
    crate::outline_navigator::entries(s.document(), &s.app().outline_state)
        .iter()
        .find(|entry| entry.paragraph == paragraph)
        .unwrap_or_else(|| s.fail("navigator", &format!("no entry for paragraph {paragraph}")))
        .collapsed
}

fn nav_action(rt: &mut Runtime, action: crate::outline_navigator::NavigatorAction) {
    rt.send(Message::OutlineNavigator(action));
}

/// A click on a navigator row: a press and a release on the same heading.
fn nav_click(s: &mut Scenario, paragraph: usize) {
    use crate::outline_navigator::{NavigatorAction as N, Target};
    s.no_change(
        &format!("click heading {paragraph} in the navigator"),
        |rt| {
            nav_action(rt, N::Press(paragraph));
            nav_action(rt, N::Release(Target::Before(paragraph)));
        },
    );
}

fn nav_key(s: &mut Scenario, key: Named) {
    s.no_change(&format!("navigator key {key:?}"), move |rt| {
        rt.send(Message::NavigatorKey(key));
    });
}

/// A drag in the navigator: press the heading, pass over the target, release.
fn nav_drag_steps(rt: &mut Runtime, from: usize, target: crate::outline_navigator::Target) {
    use crate::outline_navigator::NavigatorAction as N;
    nav_action(rt, N::Press(from));
    nav_action(rt, N::Over(target));
    nav_action(rt, N::Release(target));
}

#[test]
fn the_outline_navigator_lists_the_headings_goes_to_them_moves_sections_by_drag_and_contracts() {
    use crate::outline_navigator::{NavigatorAction as N, Target};
    let mut s = outline_document("outline-navigator");
    let main = s.main_flow();
    let command_checked = |s: &Scenario| {
        crate::menu_check(s.app(), CommandId::OutlineNavigator)
            .unwrap_or_else(|| s.fail("menu", "Outline Navigator has no check mark"))
    };

    // 1. View › Outline ▸ Outline Navigator in Layout mode: the document is
    // not changed; the rail shows the navigator; the command is checked; it
    // lists exactly the headings with their levels.
    assert!(s.app().outline.is_none(), "Layout mode");
    assert!(!command_checked(&s));
    assert!(s.app().outline_navigator.is_none());
    s.no_change("View › Outline ▸ Outline Navigator", |rt| {
        rt.send(Message::Command(CommandId::OutlineNavigator));
    });
    assert!(s.app().rail_open(), "the rail is open");
    assert!(s.app().outline_navigator.is_some(), "the navigator is open");
    assert!(command_checked(&s), "the command is checked while open");
    let text = |paragraph: usize, level: u8, text: &str| (paragraph, level, text.to_owned());
    let all = vec![
        text(0, 1, "First title"),
        text(1, 2, "Subhead A"),
        text(4, 2, "Subhead B"),
        text(6, 1, "Second title"),
        text(7, 2, "Subhead C"),
    ];
    assert_eq!(listed(&s), all);
    assert_eq!(
        crate::outline_navigator::entries(s.document(), &s.app().outline_state)
            .iter()
            .map(|entry| (entry.has_children, entry.collapsed))
            .collect::<Vec<_>>(),
        [
            (true, false),
            (true, false),
            (true, false),
            (true, false),
            (true, false)
        ]
    );

    // 2. A click on the second title: the caret goes to its start; it is the
    // highlighted heading; the document is untouched (and no undo step).
    let undo = s.app().history.undo_len();
    nav_click(&mut s, 6);
    assert_eq!(caret_at(&s), (6, 0));
    assert_eq!(current_heading(&s), Some(6));
    assert_eq!(s.app().history.undo_len(), undo, "a click is not an edit");
    assert_eq!(
        s.app().outline_navigator.as_ref().unwrap().pressed,
        None,
        "the press is over"
    );
    assert!(
        s.app().outline_navigator.as_ref().unwrap().keys,
        "the navigator holds the keys after a click"
    );

    // 3. Down goes to the next heading, Up back; Down at the last stays.
    nav_key(&mut s, Named::ArrowDown);
    assert_eq!(caret_at(&s), (7, 0), "Subhead C");
    assert_eq!(current_heading(&s), Some(7));
    nav_key(&mut s, Named::ArrowUp);
    assert_eq!(caret_at(&s), (6, 0), "back to the second title");
    nav_key(&mut s, Named::ArrowDown);
    nav_key(&mut s, Named::ArrowDown);
    assert_eq!(caret_at(&s), (7, 0), "Down at the last heading stays");
    nav_key(&mut s, Named::ArrowUp);
    nav_key(&mut s, Named::ArrowUp);
    nav_key(&mut s, Named::ArrowUp);
    assert_eq!(caret_at(&s), (1, 0), "Up through the list to Subhead A");
    nav_key(&mut s, Named::ArrowUp);
    nav_key(&mut s, Named::ArrowUp);
    assert_eq!(caret_at(&s), (0, 0), "Up at the first heading stays");
    assert_eq!(order(&s), original());

    // 4. Drag Subhead B onto Subhead A: its section goes before Subhead A's,
    // one undo step, the caret at Subhead B; Undo restores the order.
    let undo = s.app().history.undo_len();
    let mut status = String::new();
    s.edit("drag Subhead B onto Subhead A", |rt| {
        nav_drag_steps(rt, 4, Target::Before(1));
        status = rt.app.status.clone();
    });
    // (Read inside the step: the checker's Undo and Redo leave their own.)
    assert_eq!(status, "Moved the section");
    let dragged = names(&[
        "First title",
        "Subhead B",
        "Body B",
        "Subhead A",
        "Body A1",
        "Body A2",
        "Second title",
        "Subhead C",
        "Body C",
    ]);
    assert_eq!(order(&s), dragged);
    assert_eq!(caret_at(&s), (1, 0), "the caret is at Subhead B");
    assert_eq!(s.app().history.undo_len(), undo + 1, "one undo step");
    assert_eq!(
        listed(&s).iter().map(|e| e.2.as_str()).collect::<Vec<_>>(),
        [
            "First title",
            "Subhead B",
            "Subhead A",
            "Second title",
            "Subhead C"
        ]
    );
    assert_eq!(current_heading(&s), Some(1));
    s.act("Undo the drag", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(order(&s), original(), "one Undo restores the order");
    assert_eq!(s.app().history.undo_len(), undo);
    s.act("Redo the drag", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert_eq!(order(&s), dragged);

    // 5. Drag the first title onto the end area: its whole section goes to
    // the end, one step; the caret goes with it.
    let undo = s.app().history.undo_len();
    s.edit("drag the first title onto the end", |rt| {
        nav_drag_steps(rt, 0, Target::End);
    });
    let at_end = names(&[
        "Second title",
        "Subhead C",
        "Body C",
        "First title",
        "Subhead B",
        "Body B",
        "Subhead A",
        "Body A1",
        "Body A2",
    ]);
    assert_eq!(order(&s), at_end);
    assert_eq!(s.app().history.undo_len(), undo + 1, "one undo step");
    assert_eq!(caret_at(&s), (3, 0), "the caret is at the moved title");
    assert_eq!(current_heading(&s), Some(3));

    // 6. A drop inside the dragged section (a title onto its own subhead),
    // or right where it is (the end, for the last section): nothing changes.
    s.no_change("drag a title onto its own subhead", |rt| {
        nav_drag_steps(rt, 3, Target::Before(4));
    });
    assert_eq!(s.app().status, "The section is already there");
    assert_eq!(order(&s), at_end);
    s.no_change("drag the last section onto the end", |rt| {
        nav_drag_steps(rt, 3, Target::End);
    });
    assert_eq!(s.app().status, "The section is already there");
    assert_eq!(order(&s), at_end);
    assert_eq!(caret_at(&s), (3, 0));
    assert_eq!(
        s.app().outline_navigator.as_ref().unwrap().pressed,
        None,
        "the drag is over"
    );

    // 7. The chevron on the second title (now paragraph 0): its subheads
    // leave the list and it shows as collapsed; Outline mode hides the
    // same paragraphs; the chevron again shows them.
    assert_eq!(
        listed(&s).iter().map(|e| e.0).collect::<Vec<_>>(),
        [0, 1, 3, 4, 6]
    );
    s.no_change("Toggle the second title", |rt| {
        nav_action(rt, N::Toggle(0));
    });
    assert_eq!(
        listed(&s).iter().map(|e| e.0).collect::<Vec<_>>(),
        [0, 3, 4, 6],
        "Subhead C is not listed"
    );
    assert!(navigator_collapsed(&s, 0));
    assert!(!navigator_collapsed(&s, 3));
    s.no_change("View › Outline Mode on", |rt| {
        rt.send(Message::Command(CommandId::OutlineMode));
    });
    assert!(s.app().outline.is_some());
    assert!(
        s.app().outline_navigator.is_some(),
        "the navigator works in Outline mode"
    );
    let mut hidden = [false; 9];
    hidden[1..=2].fill(true);
    assert_hidden(&s, &hidden);
    s.no_change("Toggle the second title again", |rt| {
        nav_action(rt, N::Toggle(0));
    });
    assert_hidden(&s, &[false; 9]);
    assert!(!navigator_collapsed(&s, 0));
    assert_eq!(
        listed(&s).iter().map(|e| e.0).collect::<Vec<_>>(),
        [0, 1, 3, 4, 6]
    );
    // A contraction made in Outline mode shows in the tree too.
    s.caret(main, 3, 0, 1, false);
    s.no_change("Contract the first title in Outline mode", |rt| {
        rt.send(Message::Command(CommandId::OutlineContract));
    });
    assert!(navigator_collapsed(&s, 3));
    assert_eq!(
        listed(&s).iter().map(|e| e.0).collect::<Vec<_>>(),
        [0, 1, 3]
    );
    s.no_change("Expand the first title in Outline mode", |rt| {
        rt.send(Message::Command(CommandId::OutlineExpand));
    });
    assert_hidden(&s, &[false; 9]);
    s.no_change("View › Outline Mode off", |rt| {
        rt.send(Message::Command(CommandId::OutlineMode));
    });
    assert!(s.app().outline.is_none());

    // 8. With the navigator holding the keys: Left contracts the current
    // heading, Right expands it; after a click on the page the arrows do
    // nothing.
    nav_click(&mut s, 3);
    assert_eq!(caret_at(&s), (3, 0));
    nav_key(&mut s, Named::ArrowLeft);
    assert!(navigator_collapsed(&s, 3), "Left contracted the heading");
    assert_eq!(
        listed(&s).iter().map(|e| e.0).collect::<Vec<_>>(),
        [0, 1, 3]
    );
    assert_eq!(caret_at(&s), (3, 0));
    nav_key(&mut s, Named::ArrowLeft);
    assert!(
        navigator_collapsed(&s, 3),
        "Left on a contracted one: nothing"
    );
    nav_key(&mut s, Named::ArrowRight);
    assert!(!navigator_collapsed(&s, 3), "Right expanded it");
    nav_key(&mut s, Named::ArrowRight);
    assert!(!navigator_collapsed(&s, 3));
    assert_eq!(
        listed(&s).iter().map(|e| e.0).collect::<Vec<_>>(),
        [0, 1, 3, 4, 6]
    );
    s.caret(main, 0, 0, 1, false);
    assert!(
        !s.app().outline_navigator.as_ref().unwrap().keys,
        "a click on the page took the keys"
    );
    nav_key(&mut s, Named::ArrowDown);
    nav_key(&mut s, Named::ArrowLeft);
    assert_eq!(caret_at(&s), (0, 0), "the arrows do nothing now");
    assert!(!navigator_collapsed(&s, 0));
    assert_eq!(order(&s), at_end);
    // A press on a row gives the keys back.
    nav_click(&mut s, 0);
    nav_key(&mut s, Named::ArrowDown);
    assert_eq!(caret_at(&s), (1, 0));

    // 9. Close: the navigator and the rail go; the command is unchecked.
    // The command opens it again, and a second use closes it.
    s.no_change("Close the navigator", |rt| nav_action(rt, N::Close));
    assert!(s.app().outline_navigator.is_none());
    assert!(!s.app().rail_open());
    assert!(!command_checked(&s));
    nav_key(&mut s, Named::ArrowDown);
    assert_eq!(caret_at(&s), (1, 0), "a closed navigator takes no keys");
    s.no_change("Outline Navigator again", |rt| {
        rt.send(Message::Command(CommandId::OutlineNavigator));
    });
    assert!(s.app().rail_open() && s.app().outline_navigator.is_some());
    assert!(command_checked(&s));
    s.no_change("Outline Navigator once more", |rt| {
        rt.send(Message::Command(CommandId::OutlineNavigator));
    });
    assert!(s.app().outline_navigator.is_none() && !s.app().rail_open());
    assert!(!command_checked(&s));

    // The moved order is kept in the file.
    let path = s.save_sam("OUTNAV.SAM");
    let s = s.reopen(&path);
    assert_eq!(order(&s), at_end);
    assert_eq!(
        amipro_editing::sections::levels(s.document(), s.main_flow()),
        [1, 2, 0, 1, 2, 0, 2, 0, 0]
    );
}
