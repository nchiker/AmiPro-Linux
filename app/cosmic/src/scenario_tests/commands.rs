//! Run A's commands (`UI_SPEC.md` §39): Help › Keyboard Shortcuts and
//! Compatibility and Fidelity (CM12), and the other commands as the run
//! adds them.

use crate::help_drawers::{HelpAction, HelpDrawer};

use super::*;

fn drawer(s: &Scenario) -> &HelpDrawer {
    s.app()
        .help_drawer
        .as_ref()
        .unwrap_or_else(|| s.fail("drawer", "no Help drawer shows"))
}

fn losses(s: &Scenario) -> Vec<String> {
    match drawer(s) {
        HelpDrawer::Compatibility {
            losses: Ok(losses), ..
        } => losses.clone(),
        other => s.fail("drawer", &format!("not a compatibility list: {other:?}")),
    }
}

#[test]
fn the_help_drawers_list_the_shortcuts_and_what_saving_as_sam_would_change() {
    let mut s = Scenario::new_document("help-drawers");
    s.clean_folder();
    s.type_text("Plain text Ami Pro stores exactly.");

    // Keyboard Shortcuts: the registry's list, filtered as typed.
    s.command_act(CommandId::KeyboardShortcuts);
    assert!(matches!(drawer(&s), HelpDrawer::Shortcuts { filter } if filter.is_empty()));
    s.act("find bold", |rt| {
        rt.send(Message::HelpDrawer(HelpAction::Filter("bold".into())));
    });
    assert!(matches!(drawer(&s), HelpDrawer::Shortcuts { filter } if filter == "bold"));
    let rows = crate::help_drawers::shortcut_rows("bold");
    assert!(
        rows.iter()
            .flat_map(|(_, rows)| rows)
            .any(|(label, key)| *label == "Bold" && key == "Ctrl+B")
    );

    // Compatibility and Fidelity replaces it: nothing would change.
    s.command_act(CommandId::Compatibility);
    assert_eq!(losses(&s), Vec::<String>::new());

    // Characters Windows-1252 has no code for are listed once checked again.
    s.edit("type characters Ami Pro cannot store", |rt| {
        rt.app.page_keyboard += 1;
        for character in [" ", "日", "本"] {
            rt.send(Message::Page(PageAction::Insert(character.into())));
        }
    });
    assert_eq!(
        losses(&s),
        Vec::<String>::new(),
        "the list waits for Check Again"
    );
    s.act("Check Again", |rt| {
        rt.send(Message::HelpDrawer(HelpAction::CheckAgain));
    });
    let listed = losses(&s);
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert!(listed[0].contains("characters"), "{listed:?}");

    // Undo takes the typing; checked again the list is empty; nothing was
    // ever written.
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    s.act("Check Again", |rt| {
        rt.send(Message::HelpDrawer(HelpAction::CheckAgain));
    });
    assert_eq!(losses(&s), Vec::<String>::new());
    assert!(std::fs::read_dir(s.folder()).map_or(true, |mut dir| dir.next().is_none()));

    s.act("close the drawer", |rt| {
        rt.send(Message::HelpDrawer(HelpAction::Close));
    });
    assert!(s.app().help_drawer.is_none());
    s.save_sam("HELP.SAM");
}

/// The File menu's labels, recent files included.
fn file_menu_labels(s: &Scenario) -> Vec<String> {
    let (_, slots) = crate::command::MENU_LAYOUTS
        .iter()
        .find(|(root, _)| *root == crate::command::MenuRoot::File)
        .unwrap();
    crate::menu_entries(s.app(), slots)
        .into_iter()
        .filter_map(|entry| match entry {
            crate::MenuEntry::Item { label, .. } => Some(label),
            _ => None,
        })
        .collect()
}

#[test]
fn preferences_change_the_undo_levels_the_backup_and_the_recent_files() {
    use crate::preferences_dialog::{Page, PreferencesAction as A, ViewSetting};
    let mut s = Scenario::new_document("preferences");
    let folder = s.clean_folder();
    s.command_act(CommandId::Preferences);
    s.act("two undo levels, no backup", |rt| {
        for action in [A::UndoLevels("2".into()), A::KeepBackup(false)] {
            rt.send(Message::Preferences(action));
        }
    });
    s.act("the ruler off on the View page", |rt| {
        rt.send(Message::Preferences(A::Page(Page::View)));
        rt.send(Message::Preferences(A::View(ViewSetting::Ruler, false)));
    });
    assert!(!s.app().preferences.ruler_visible);
    s.act("close", |rt| {
        rt.send(Message::Preferences(A::Close));
    });
    assert!(s.app().preferences_dialog.is_none());

    // Three steps, of which two are kept.
    s.type_text("First");
    s.enter();
    s.type_text("Second");
    assert_eq!(s.app().history.undo_len(), 2);

    // Saved as SAM, then saved over: no backup is made.
    let path = s.save_sam("PREFS.SAM");
    s.type_text(" again");
    s.save();
    assert!(path.exists());
    assert!(!folder.join("PREFS.SAM.bak").exists());

    // The file is the File menu's first recent file; choosing it shows it.
    let labels = file_menu_labels(&s);
    assert!(
        labels.iter().any(|label| label == "1 PREFS.SAM"),
        "{labels:?}"
    );
    s.act("File › 1 PREFS.SAM", |rt| {
        rt.send(Message::OpenRecent(0));
    });
    assert!(
        s.app().status.starts_with("Activated"),
        "{}",
        s.app().status
    );

    // Recent files off: the menu lists none.
    s.command_act(CommandId::Preferences);
    s.act("no recent files", |rt| {
        rt.send(Message::Preferences(A::RecentFiles("0".into())));
        rt.send(Message::Preferences(A::Close));
    });
    assert!(
        !file_menu_labels(&s)
            .iter()
            .any(|label| label.starts_with("1 "))
    );
}

#[test]
fn paragraph_properties_set_indents_spacing_and_alignment_each_one_step_and_save_as_sam() {
    use crate::paragraph_properties::{Field, ParagraphAction as P, Section};
    use amipro_document::{Alignment, LineSpacing};
    let mut s = Scenario::new_document("paragraph-properties");
    s.clean_folder();
    s.type_text("A first paragraph of text.");
    s.enter();
    s.type_text("A second paragraph, indented and spaced.");
    s.enter();
    s.type_text("A third paragraph, the same.");
    s.select_main((1, 2), (2, 3));
    s.command_act(CommandId::ParagraphProperties);
    let panel = |s: &Scenario| {
        s.app()
            .paragraph_properties
            .clone()
            .unwrap_or_else(|| s.fail("panel", "Paragraph Properties is not showing"))
    };
    assert_eq!(panel(&s).shown.paragraphs, 2);
    assert!(!panel(&s).shown.own_indents);

    let typed = |s: &mut Scenario, what: &str, field: Field, text: &str| {
        let text = text.to_owned();
        s.edit(what, move |rt| {
            rt.send(Message::ParagraphProperties(P::Text(field, text)));
            rt.send(Message::ParagraphProperties(P::Submit(field)));
        });
    };
    typed(&mut s, "All 0.5 in.", Field::All, "0.5");
    typed(&mut s, "First 0.25 in.", Field::First, "0.25");
    let format =
        |s: &Scenario, index: usize| s.document().main_flow().paragraphs[index].format.clone();
    for index in [1, 2] {
        let indents = format(&s, index).indents.expect("own indents");
        assert_eq!((indents.all, indents.first), (720, 360));
    }
    assert_eq!(format(&s, 0).indents, None);
    assert!(panel(&s).shown.own_indents);

    s.act("open Spacing", |rt| {
        rt.send(Message::ParagraphProperties(P::Section(Section::Spacing)));
    });
    s.edit("Double", |rt| {
        rt.send(Message::ParagraphProperties(P::LineSpacing(2)));
    });
    assert_eq!(format(&s, 2).line_spacing, Some(LineSpacing::Double(0)));
    s.edit("Custom", |rt| {
        rt.send(Message::ParagraphProperties(P::LineSpacing(3)));
    });
    typed(&mut s, "Custom 0.25 in.", Field::Custom, "0.25");
    assert_eq!(format(&s, 1).line_spacing, Some(LineSpacing::Custom(360)));
    s.act("open Alignment", |rt| {
        rt.send(Message::ParagraphProperties(P::Section(Section::Alignment)));
    });
    s.edit("Center", |rt| {
        rt.send(Message::ParagraphProperties(P::Align(Alignment::Centre)));
    });
    assert_eq!(format(&s, 1).alignment, Some(Alignment::Centre));

    // A typed text that is not a length changes nothing and is restored.
    s.no_change("All: not a length", |rt| {
        rt.send(Message::ParagraphProperties(P::Text(
            Field::All,
            "wide".into(),
        )));
        rt.send(Message::ParagraphProperties(P::Submit(Field::All)));
    });
    assert_eq!(panel(&s).texts[0], "0.50");

    // Increase Indent adds half an inch; Revert to style takes it back.
    s.command(CommandId::IncreaseIndent);
    assert_eq!(format(&s, 1).indents.unwrap().all, 1440);
    s.command(CommandId::DecreaseIndent);
    assert_eq!(format(&s, 2).indents.unwrap().all, 720);
    s.edit("Revert alignment", |rt| {
        rt.send(Message::ParagraphProperties(P::Revert(Section::Alignment)));
    });
    assert_eq!(format(&s, 1).alignment, None);

    // Saved as SAM and reopened, the paragraphs keep their own format.
    let path = s.save_sam("PARA.SAM");
    let bytes = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
    assert!(bytes.contains("<:I"), "indents written");
    assert!(bytes.contains("<:S+360>"), "custom spacing written");

    s.edit("Revert indents", |rt| {
        rt.send(Message::ParagraphProperties(P::Revert(Section::Indention)));
    });
    s.edit("Revert spacing", |rt| {
        rt.send(Message::ParagraphProperties(P::Revert(Section::Spacing)));
    });
    assert!(format(&s, 1).is_empty() && format(&s, 2).is_empty());
    s.act("close", |rt| {
        rt.send(Message::ParagraphProperties(P::Close));
    });
    assert!(s.app().paragraph_properties.is_none());
}

#[test]
fn fast_format_picks_up_bold_or_a_style_and_each_mouse_selection_takes_it() {
    use crate::fast_format::FastAction as F;
    let mut s = Scenario::new_document("fast-format");
    s.clean_folder();
    s.type_text("Bold words here.");
    s.enter();
    s.type_text("Plain words here.");
    s.enter();
    s.type_text("A heading.");
    s.select_main((0, 0), (0, 4));
    s.command(CommandId::Bold);

    // A partial selection picks up at once: the text's bold and the style.
    s.select_main((0, 1), (0, 3));
    s.command_act(CommandId::FastFormat);
    assert!(
        s.app()
            .fast_format
            .as_ref()
            .is_some_and(|format| format.text.is_some())
    );
    assert!(s.app().fast_format_ask.is_none());
    s.select_main((1, 0), (1, 5));
    s.edit("release the mouse over Plain", |rt| {
        rt.send(Message::Page(PageAction::PointerReleased));
    });
    let bold = |s: &Scenario, paragraph: usize, byte: usize| {
        s.document().main_flow().paragraphs[paragraph]
            .format_at(byte)
            .bold
    };
    assert_eq!(bold(&s, 1, 2), Some(true));
    assert_eq!(bold(&s, 1, 7), None);
    // Another selection takes it again; Ctrl+T ends the mode.
    s.select_main((1, 6), (1, 11));
    s.edit("release the mouse over words", |rt| {
        rt.send(Message::Page(PageAction::PointerReleased));
    });
    assert_eq!(bold(&s, 1, 7), Some(true));
    s.command_act(CommandId::FastFormat);
    assert!(s.app().fast_format.is_none());
    s.select_main((1, 12), (1, 16));
    s.no_change("a release with the mode off", |rt| {
        rt.send(Message::Page(PageAction::PointerReleased));
    });

    // Heading 1 on the third paragraph; a caret there asks, as Ami Pro.
    let heading = *s
        .document()
        .styles()
        .iter()
        .find(|(_, style)| style.name == "Heading 1")
        .expect("Heading 1")
        .0;
    s.select_main((2, 1), (2, 1));
    s.edit("Heading 1", move |rt| {
        rt.send(Message::ApplyStyle(heading));
    });
    s.command_act(CommandId::FastFormat);
    assert_eq!(
        s.app().fast_format_ask,
        Some(crate::fast_format::FastFormatAsk { style: true })
    );
    s.act("Only paragraph style, OK", |rt| {
        rt.send(Message::FastFormat(F::Choose(true)));
        rt.send(Message::FastFormat(F::Ok));
    });
    assert!(
        s.app()
            .fast_format
            .as_ref()
            .is_some_and(|format| format.paragraph.is_some())
    );
    s.select_main((0, 3), (0, 3));
    s.edit("click in the first paragraph", |rt| {
        rt.send(Message::Page(PageAction::PointerReleased));
    });
    assert_eq!(s.document().main_flow().paragraphs[0].style, heading);
    // Esc ends it.
    s.act("Esc", |rt| {
        rt.send(Message::Page(PageAction::Escape));
    });
    assert!(s.app().fast_format.is_none());

    // Cancel in the question leaves the mode off.
    s.command_act(CommandId::FastFormat);
    s.act("Cancel", |rt| {
        rt.send(Message::FastFormat(F::Cancel));
    });
    assert!(s.app().fast_format.is_none() && s.app().fast_format_ask.is_none());
    s.save_sam("FAST.SAM");
}

#[test]
fn mark_text_select_a_style_and_revert_to_saved() {
    let mut s = Scenario::new_document("mark-revert");
    s.clean_folder();
    s.type_text("Keep this word safe.");
    s.enter();
    s.type_text("A heading to be.");

    // Mark Text ▸ Protected Text and No Hyphenation: checked at the caret.
    s.select_main((0, 5), (0, 9));
    s.command(CommandId::ProtectedText);
    s.command(CommandId::NoHyphenation);
    let format = s.document().main_flow().paragraphs[0].format_at(6).clone();
    assert_eq!(
        (format.protected, format.no_hyphenation),
        (Some(true), Some(true))
    );
    let check = |s: &Scenario, command| {
        crate::menu_check(s.app(), command).unwrap_or_else(|| s.fail("menu", "no check"))
    };
    assert!(check(&s, CommandId::ProtectedText));
    s.select_main((0, 1), (0, 1));
    assert!(!check(&s, CommandId::ProtectedText));

    // Select a Style (Ctrl+Y): the Styles panel's filter; Enter applies the
    // first style listed.
    s.select_main((1, 2), (1, 2));
    s.command_act(CommandId::Styles);
    assert!(s.app().rail_open());
    s.act("type Heading 2", |rt| {
        rt.send(Message::StyleFilter("heading 2".into()));
    });
    s.edit("Enter", |rt| {
        rt.send(Message::StyleFilterSubmit);
    });
    let document = s.document();
    assert_eq!(
        document.styles()[&document.main_flow().paragraphs[1].style].name,
        "Heading 2"
    );

    // Saved, then changed: Revert to Saved asks; Cancel keeps the change.
    let path = s.save_sam("MARK.SAM");
    let bytes = String::from_utf8_lossy(&std::fs::read(&path).unwrap()).into_owned();
    assert!(bytes.contains("<+*>") && bytes.contains("<+.>"));
    assert!(!s.app().command_enabled(CommandId::RevertToSaved));
    s.caret_to_end();
    s.type_text(" Unsaved.");
    assert!(s.app().command_enabled(CommandId::RevertToSaved));
    s.command_act(CommandId::RevertToSaved);
    assert!(s.app().revert_pending);
    s.act("Cancel", |rt| {
        rt.send(Message::RevertToSaved(false));
    });
    assert!(s.main_text().ends_with("Unsaved."));
    // Revert: the saved text, unchanged since; Undo brings the change back.
    s.command_act(CommandId::RevertToSaved);
    s.edit("Revert", |rt| {
        rt.send(Message::RevertToSaved(true));
    });
    assert_eq!(s.main_text(), "Keep this word safe.\nA heading to be.");
    assert!(!s.app().dirty);
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(s.main_text().ends_with("Unsaved."));
    assert!(s.app().dirty);
}

#[test]
fn the_status_bar_sets_face_and_size_and_special_characters_are_inserted() {
    use crate::special_character::CharacterAction;
    let mut s = Scenario::new_document("status-font");
    s.clean_folder();
    s.type_text("Some words to format.");
    s.select_main((0, 5), (0, 10));
    // The Face list holds the selection's face; choosing another applies it.
    let families = s.app().font_families.clone();
    assert!(!families.is_empty());
    let current = s.app().sessions[s.app().active_session].selection_font().0;
    assert!(
        families
            .iter()
            .any(|family| family.eq_ignore_ascii_case(&current))
    );
    let other = families
        .iter()
        .position(|family| !family.eq_ignore_ascii_case(&current))
        .expect("another face");
    let other_name = families[other].clone();
    s.edit("Face from the status bar", move |rt| {
        rt.send(Message::StatusFace(other));
    });
    let run = s.document().main_flow().paragraphs[0].format_at(6).clone();
    assert_eq!(run.font_family.as_deref(), Some(other_name.as_str()));
    let eighteen = crate::character_dialogs::SIZES
        .iter()
        .position(|size| *size == "18")
        .unwrap();
    s.edit("18 points from the status bar", move |rt| {
        rt.send(Message::StatusSize(eighteen));
    });
    assert_eq!(
        s.document().main_flow().paragraphs[0]
            .format_at(6)
            .font_size_points,
        Some(18.0)
    );

    // Special Character… inserts at the caret and remembers it.
    s.caret_to_end();
    s.command_act(CommandId::SpecialCharacter);
    assert!(s.app().special_character);
    s.edit("insert ©", |rt| {
        rt.send(Message::SpecialCharacter(CharacterAction::Insert('©')));
    });
    assert!(s.main_text().ends_with("format.©"));
    assert!(!s.app().special_character);
    assert_eq!(s.app().preferences.recent_characters, ['©']);
    s.command_act(CommandId::SpecialCharacter);
    s.act("Cancel", |rt| {
        rt.send(Message::SpecialCharacter(CharacterAction::Cancel));
    });
    assert!(!s.app().special_character);
    s.save_sam("FONTS.SAM");
}
