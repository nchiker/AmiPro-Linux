//! Styles (`UI_SPEC.md` §14–16): Create Style, Style Properties in the rail
//! (each change one undo step), applying from the Styles panel and the style
//! field, rename, remove, Style Management, Save as a Style Sheet and Use
//! Another Style Sheet.

use amipro_document::{Alignment, Breaks, StyleId, StyleOrigin};

use crate::style_dialogs::{CreateAction, ManagementAction, UseSheetAction};
use crate::style_properties::{Field, FontAttribute, Section, StyleAction};

use super::*;

fn style_id(s: &Scenario, name: &str) -> StyleId {
    *s.document()
        .styles()
        .iter()
        .find(|(_, style)| style.name.eq_ignore_ascii_case(name))
        .unwrap_or_else(|| s.fail("style", &format!("no style named {name:?}")))
        .0
}

fn has_style(s: &Scenario, name: &str) -> bool {
    s.document()
        .styles()
        .values()
        .any(|style| style.name.eq_ignore_ascii_case(name))
}

/// The style of paragraph `index` of the main text, by name.
fn style_name_of(s: &Scenario, index: usize) -> String {
    let document = s.document();
    document.styles()[&document.main_flow().paragraphs[index].style]
        .name
        .clone()
}

/// A property change in Style Properties: the message, as one undo step.
fn property(s: &mut Scenario, what: &str, action: StyleAction) {
    s.edit(what, move |rt| {
        rt.send(Message::StyleProperties(action));
    });
}

/// A typed value in a Style Properties box, then Enter.
fn typed(s: &mut Scenario, what: &str, field: Field, text: &str) {
    let text = text.to_owned();
    s.edit(what, move |rt| {
        rt.send(Message::StyleProperties(StyleAction::Text(field, text)));
        rt.send(Message::StyleProperties(StyleAction::Submit(field)));
    });
}

fn properties_style(s: &Scenario) -> StyleId {
    s.app()
        .style_properties
        .as_ref()
        .unwrap_or_else(|| s.fail("panel", "Style Properties is not showing"))
        .style
}

/// The right button on a Styles panel row and an entry of its menu.
fn row_menu(s: &mut Scenario, what: &str, style: StyleId, action: StyleRowAction) {
    s.act(what, move |rt| {
        rt.send(Message::StyleRowContext(Some(style)));
        rt.send(Message::StyleRowMenu(action));
    });
    assert_eq!(
        s.app().rail.context_style,
        None,
        "{what}: the row is forgotten"
    );
}

/// Create Style from the caret paragraph's style, by name; one undo step.
fn create_style(s: &mut Scenario, name: &str) {
    let name = name.to_owned();
    s.edit(&format!("Create Style {name:?}"), move |rt| {
        rt.send(Message::Command(CommandId::CreateStyle));
        assert!(
            rt.app.create_style.is_some(),
            "the Create Style dialog opens"
        );
        assert!(cosmic::Application::dialog(&rt.app).is_some());
        rt.send(Message::CreateStyle(CreateAction::Name(name)));
        rt.send(Message::CreateStyle(CreateAction::Create));
        assert!(rt.app.create_style.is_none(), "the dialog closes");
    });
}

#[test]
fn a_style_created_modified_in_the_rail_applied_renamed_and_saved_in_a_sam() {
    let mut s = Scenario::new_document("styles-create-modify");
    s.clean_folder();
    s.type_text("A title for the document");
    s.enter();
    s.type_text("First body paragraph of text.");
    s.enter();
    s.type_text("Second body paragraph of text.");
    s.enter();
    s.type_text("Closing line.");

    // Create Style based on Body Text; the paragraph keeps its style.
    let main = s.main_flow();
    s.caret(main, 1, 3, 1, false);
    let styles_before = s.document().styles().len();
    create_style(&mut s, "Callout");
    assert_eq!(s.document().styles().len(), styles_before + 1);
    assert_eq!(style_name_of(&s, 1), "Body Text", "kept, as in Ami Pro");
    let callout = style_id(&s, "Callout");
    assert_eq!(
        s.document().styles()[&callout].origin,
        StyleOrigin::Document
    );
    assert_eq!(
        s.document().styles()[&callout].font_size_points,
        s.document().styles()[&style_id(&s, "Body Text")].font_size_points,
        "based on Body Text"
    );

    // Apply it from the style field to the selected paragraph.
    s.edit("apply Callout to paragraph 1", move |rt| {
        rt.send(Message::ApplyStyle(callout));
    });
    assert_eq!(style_name_of(&s, 1), "Callout");
    assert_eq!(
        s.app().selected_style(),
        Some(callout),
        "the style field follows"
    );

    // Style Properties opens in the rail on the caret's style.
    s.command_act(CommandId::ModifyCurrentStyle);
    assert_eq!(properties_style(&s), callout);
    assert!(s.app().rail_open());
    assert!(
        cosmic::Application::dialog(s.app()).is_none(),
        "no modal dialog"
    );

    // A typed value waits for Enter; a value that does not read is refused
    // in place with the reason and changes nothing.
    s.no_change("type a size without Enter", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Text(
            Field::Size,
            "22".into(),
        )));
    });
    s.no_change("a size that does not read", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Text(
            Field::Size,
            "huge".into(),
        )));
        rt.send(Message::StyleProperties(StyleAction::Submit(Field::Size)));
        let problem = rt.app.style_properties.as_ref().unwrap().error.clone();
        assert!(problem.is_some(), "the reason is shown");
    });
    typed(&mut s, "Size 22", Field::Size, "22");
    assert_eq!(s.document().styles()[&callout].font_size_points, 22.0);
    property(
        &mut s,
        "Bold on",
        StyleAction::Attribute(FontAttribute::Bold, true),
    );
    property(
        &mut s,
        "Italic on",
        StyleAction::Attribute(FontAttribute::Italic, true),
    );
    property(&mut s, "Centre", StyleAction::Align(Alignment::Centre));
    s.act("open Alignment and indents", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Section(
            Section::Alignment,
        )));
    });
    typed(&mut s, "Indent All 0.50", Field::All, "0.5");
    typed(&mut s, "Right indent 0.25", Field::Right, "0.25");
    s.act("open Spacing", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Section(
            Section::Spacing,
        )));
    });
    typed(&mut s, "Space above 0.20", Field::Above, "0.2");
    typed(&mut s, "Space below 0.10", Field::Below, "0.1");
    property(&mut s, "Double line spacing", StyleAction::LineSpacing(2));
    s.act("open Breaks", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Section(
            Section::Breaks,
        )));
    });
    property(
        &mut s,
        "Page break before",
        StyleAction::Break(Breaks::PAGE_BEFORE, true),
    );
    property(&mut s, "Hyphenation on", StyleAction::Hyphenation(true));
    let definition = s.document().styles()[&callout].clone();
    assert_eq!(definition.alignment, Alignment::Centre);
    assert_eq!(definition.indents.all, 720);
    assert_eq!(definition.spacing.above, 288);
    assert!(definition.bold && definition.italic);
    assert!((definition.breaks.0 & Breaks::PAGE_BEFORE != 0));
    // The panel shows the document's definition, and the page break before
    // the paragraph made a second page.
    assert_eq!(s.app().style_properties.as_ref().unwrap().draft, definition);
    assert!(s.app().pages.layout().pages.len() >= 2, "the break shows");

    // Undo one step at a time and the panel shows the undone style.
    s.act("Undo (hyphenation on)", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(
        s.app().style_properties.as_ref().unwrap().draft,
        s.document().styles()[&callout],
        "the panel shows the undone style"
    );
    assert!(!s.document().styles()[&callout].hyphenation);
    s.act("Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert_eq!(s.document().styles()[&callout], definition);

    // The panel follows the caret to a paragraph in another style, and a
    // style picked in its list stays until the caret moves on.
    s.caret(main, 0, 0, 1, false);
    assert_eq!(properties_style(&s), style_id(&s, "Body Text"));
    s.act("pick Callout in the panel's list", |rt| {
        let index = crate::style_dialogs::listed_styles(rt.app.document.styles())
            .iter()
            .position(|(id, _)| *id == callout)
            .unwrap();
        rt.send(Message::StyleProperties(StyleAction::Style(index)));
    });
    assert_eq!(properties_style(&s), callout);
    s.caret(main, 2, 0, 1, false);
    assert_eq!(
        properties_style(&s),
        callout,
        "a picked style stays while the caret is in a paragraph of the same style as before"
    );
    s.caret(main, 1, 0, 1, false);
    assert_eq!(properties_style(&s), callout);
    s.caret(main, 3, 0, 1, false);
    assert_eq!(properties_style(&s), style_id(&s, "Body Text"));

    // Apply Callout to a two-paragraph selection: one step, both change.
    s.select_main((2, 2), (3, 4));
    s.edit("apply Callout from the Styles panel row", move |rt| {
        rt.send(Message::StyleRowContext(Some(callout)));
        rt.send(Message::StyleRowMenu(StyleRowAction::Apply));
    });
    assert_eq!(style_name_of(&s, 2), "Callout");
    assert_eq!(style_name_of(&s, 3), "Callout");

    // Rename it from the row's menu; a taken name is refused.
    row_menu(&mut s, "row menu: Rename", callout, StyleRowAction::Rename);
    assert!(s.app().rename_style.is_some());
    s.edit("Rename to Pull Quote", |rt| {
        rt.send(Message::RenameStyle(RenameAction::Name(
            "Pull Quote".into(),
        )));
        rt.send(Message::RenameStyle(RenameAction::Rename));
        assert!(rt.app.rename_style.is_none());
    });
    assert!(has_style(&s, "Pull Quote") && !has_style(&s, "Callout"));
    assert_eq!(
        style_name_of(&s, 1),
        "Pull Quote",
        "paragraphs follow the rename"
    );

    // Save as New Style… in the panel: a copy under a new name.
    s.act("Save as New Style…", |rt| {
        rt.send(Message::StyleProperties(StyleAction::SaveAs));
    });
    s.edit("Save as New Style: Quote Wide", |rt| {
        rt.send(Message::StyleProperties(StyleAction::SaveAsName(
            "Quote Wide".into(),
        )));
        rt.send(Message::StyleProperties(StyleAction::SaveAsConfirm));
        let shown = rt.app.style_properties.as_ref().unwrap().style;
        let made = rt
            .app
            .document
            .styles()
            .values()
            .find(|style| style.name == "Quote Wide")
            .unwrap()
            .id;
        assert_eq!(shown, made, "the new style shows");
    });
    let wide = style_id(&s, "Quote Wide");
    assert_eq!(s.document().styles()[&wide].indents.all, 720);

    // Remove the copy (unused): it is defined nowhere afterwards.
    row_menu_edit(&mut s, "row menu: Remove", wide, StyleRowAction::Remove);
    assert_eq!(s.document().styles()[&wide].origin, StyleOrigin::Missing);

    // Close the rail; the document keeps its styles. Save as SAM and reopen.
    s.act("close Style Properties", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Close));
    });
    // Remove left the unused copy as a style defined nowhere, which the
    // loss check lists (it cannot be stored).
    let path = s.save_sam_expecting("STYLED.SAM", &["1 style"]);
    let s = s.reopen(&path);
    let quote = style_id(&s, "Pull Quote");
    let reopened = &s.document().styles()[&quote];
    assert_eq!(reopened.font_size_points, 22.0);
    assert!(reopened.bold && reopened.italic);
    assert_eq!(reopened.alignment, Alignment::Centre);
    assert_eq!(reopened.indents.all, 720);
    assert_eq!(reopened.spacing.above, 288);
    assert!((reopened.breaks.0 & Breaks::PAGE_BEFORE != 0));
    assert_eq!(style_name_of(&s, 1), "Pull Quote");
    assert_eq!(style_name_of(&s, 3), "Pull Quote");
}

/// A row-menu entry that edits the document (one undo step).
fn row_menu_edit(s: &mut Scenario, what: &str, style: StyleId, action: StyleRowAction) {
    s.edit(what, move |rt| {
        rt.send(Message::StyleRowContext(Some(style)));
        rt.send(Message::StyleRowMenu(action));
    });
}

#[test]
fn styles_are_applied_from_the_style_field_the_panel_and_the_right_click_menu() {
    let mut s = Scenario::new_document("styles-apply");
    s.clean_folder();
    s.type_text("Chapter one");
    s.enter();
    s.type_text("Some text under it.");
    s.enter();
    s.type_text("More text under it.");
    let h1 = style_id(&s, "Heading 1");
    let h2 = style_id(&s, "Heading 2");
    let body = style_id(&s, "Body Text");

    // The style field follows the caret.
    let main = s.main_flow();
    s.caret(main, 0, 3, 1, false);
    s.act("the style field is read", |_| {});
    let first = s.app().selected_style();
    s.edit("apply Heading 1 to paragraph 0", move |rt| {
        rt.send(Message::ApplyStyle(h1));
    });
    assert_eq!(s.app().selected_style(), Some(h1));
    assert_ne!(first, Some(h1));
    s.caret(main, 1, 0, 1, false);
    assert_eq!(s.app().selected_style(), Some(body));

    // A selection over two paragraphs of different styles has no one style
    // in the field; applying takes all.
    s.select_main((0, 2), (1, 4));
    assert_eq!(s.app().selected_style(), None);
    s.edit("apply Heading 2 to both", move |rt| {
        rt.send(Message::ApplyStyle(h2));
    });
    assert_eq!(style_name_of(&s, 0), "Heading 2");
    assert_eq!(style_name_of(&s, 1), "Heading 2");
    assert_eq!(style_name_of(&s, 2), "Body Text");
    assert_eq!(s.app().selected_style(), Some(h2));

    // From the Styles panel: its filter narrows the rows (not a document
    // change), and a row's Apply does the same.
    s.no_change("filter the Styles panel", |rt| {
        rt.send(Message::StyleFilter("head".into()));
    });
    s.caret(main, 2, 0, 1, false);
    s.edit("panel row: Apply Heading 1", move |rt| {
        rt.send(Message::StyleRowContext(Some(h1)));
        rt.send(Message::StyleRowMenu(StyleRowAction::Apply));
    });
    s.no_change("clear the filter", |rt| {
        rt.send(Message::StyleFilter(String::new()));
    });

    // Typing continues in the style; Enter after a heading gives the
    // heading's next style (itself or what the style names).
    s.caret_to_end();
    s.type_text("!");
    let next = s.document().styles()[&h1].next;
    s.enter();
    let new_style = s.document().main_flow().paragraphs[3].style;
    assert_eq!(
        new_style,
        next.unwrap_or(h1),
        "Enter gives the next style (a heading's own Next style, or itself)"
    );

    // The right-click menu on the text: Modify Style… first.
    let text = s.rt.on_page(2200.0, 1500.0);
    s.act("right-click the text", |rt| {
        rt.click(cosmic::iced::mouse::Button::Right, text);
    });
    s.act("right-click menu: Modify Style…", |rt| {
        let messages = rt.click(cosmic::iced::mouse::Button::Left, menu_row(text, 0, 0));
        assert_eq!(commands(messages), [CommandId::ModifyCurrentStyle]);
    });
    assert!(s.app().style_properties.is_some());
    assert!(s.app().rail_open());

    let path = s.save_sam("APPLIED.SAM");
    let s = s.reopen(&path);
    assert_eq!(style_name_of(&s, 0), "Heading 2");
    assert_eq!(style_name_of(&s, 2), "Heading 1");
}

#[test]
fn a_style_sheet_is_saved_from_a_document_and_used_by_another_with_management() {
    let mut s = Scenario::new_document("styles-sheets");
    let folder = s.clean_folder();

    // The first document defines two styles and saves them as a sheet.
    s.type_text("Source document");
    create_style(&mut s, "Sheeted");
    let sheeted = style_id(&s, "Sheeted");
    s.command_act(CommandId::ModifyCurrentStyle);
    s.act("show Sheeted in the panel", move |rt| {
        let index = crate::style_dialogs::listed_styles(rt.app.document.styles())
            .iter()
            .position(|(id, _)| *id == sheeted)
            .unwrap();
        rt.send(Message::StyleProperties(StyleAction::Style(index)));
    });
    typed(&mut s, "Sheeted size 18", Field::Size, "18");
    property(
        &mut s,
        "Sheeted italic",
        StyleAction::Attribute(FontAttribute::Italic, true),
    );
    create_style(&mut s, "Plain Extra");

    s.act("Format › Styles › Save as a Style Sheet…", |rt| {
        rt.send(Message::Command(CommandId::SaveAsStyleSheet));
        assert!(
            rt.app.save_sheet.is_some(),
            "the dialog asks for a description"
        );
        assert!(cosmic::Application::dialog(&rt.app).is_some());
        rt.send(Message::SaveSheet(SaveSheetAction::Description(
            "Scenario sheet".into(),
        )));
        rt.send(Message::SaveSheet(SaveSheetAction::Contents(false)));
    });
    // Choose… opens the desktop's save dialog: never run.
    let sheet = folder.join("SCEN.STY");
    let asked = {
        let task =
            cosmic::Application::update(&mut s.rt.app, Message::SaveSheet(SaveSheetAction::Choose));
        cosmic::iced::runtime::task::into_stream(task).is_some()
    };
    assert!(asked, "Choose asks the desktop for a file");
    let session_id = s.app().id;
    s.act("the chooser names SCEN.STY", |rt| {
        rt.send(Message::SheetDialogFinished {
            session_id,
            description: "Scenario sheet".into(),
            contents: false,
            result: FileDialogResult::Selected(sheet.clone()),
        });
    });
    assert!(sheet.exists(), "{}", s.app().status);
    // An existing file is never overwritten, and a wrong extension is refused.
    let before = std::fs::read(&sheet).unwrap();
    s.expect_status("exist");
    s.no_change(
        "Save as a Style Sheet onto the same file, then close the problem",
        |rt| {
            rt.send(Message::Command(CommandId::SaveAsStyleSheet));
            rt.send(Message::SaveSheet(SaveSheetAction::Description(
                "Other".into(),
            )));
            let task = cosmic::Application::update(
                &mut rt.app,
                Message::SaveSheet(SaveSheetAction::Choose),
            );
            drop(cosmic::iced::runtime::task::into_stream(task));
            rt.send(Message::SheetDialogFinished {
                session_id,
                description: "Other".into(),
                contents: false,
                result: FileDialogResult::Selected(sheet.clone()),
            });
            let problem = rt.app.save_problem.clone().expect("a dialog says why");
            assert!(
                problem.title.contains("already exists"),
                "{}",
                problem.title
            );
            rt.send(Message::SaveProblemClose);
        },
    );
    assert_eq!(std::fs::read(&sheet).unwrap(), before, "never written over");
    s.clear_expected_status();
    s.app_mut().status = "Ready".into();

    // A second document uses the sheet.
    s.act("File › New", |rt| {
        rt.send(Message::Command(CommandId::NewDocument));
    });
    s.type_text("Second document");
    s.app_mut().preferences.style_folders = vec![folder.clone()];
    s.act("Use Another Style Sheet…", |rt| {
        rt.send(Message::Command(CommandId::UseAnotherStyleSheet));
    });
    let listed = s.app().use_sheet.clone().expect("the dialog lists sheets");
    let index = listed
        .sheets
        .iter()
        .position(|(path, description)| {
            path.ends_with("SCEN.STY") && description == "Scenario sheet"
        })
        .unwrap_or_else(|| s.fail("use sheet", &format!("not listed: {:?}", listed.sheets)));
    s.act("select the sheet", |rt| {
        rt.send(Message::UseStyleSheet(UseSheetAction::Select(index)));
    });
    s.edit("Use the sheet", |rt| {
        rt.send(Message::UseStyleSheet(UseSheetAction::Apply));
        assert!(rt.app.use_sheet.is_none());
    });
    assert_eq!(
        s.document()
            .style_sheet()
            .map(|link| link.name.to_lowercase()),
        Some("scen.sty".into())
    );
    assert!(
        has_style(&s, "Sheeted"),
        "the sheet's style is in the document"
    );
    let sheeted = style_id(&s, "Sheeted");
    assert_eq!(s.document().styles()[&sheeted].origin, StyleOrigin::Sheet);
    assert_eq!(s.document().styles()[&sheeted].font_size_points, 18.0);
    assert!(s.document().styles()[&sheeted].italic);
    s.edit("apply Sheeted", move |rt| {
        rt.send(Message::ApplyStyle(sheeted));
    });

    // Changing a sheet style in the document stores it there; Style
    // Management shows it, Revert takes the sheet's version again.
    s.command_act(CommandId::ModifyCurrentStyle);
    typed(&mut s, "Sheeted size 30", Field::Size, "30");
    assert_eq!(
        s.document().styles()[&sheeted].origin,
        StyleOrigin::Document
    );
    s.act("Format › Styles › Style Management…", |rt| {
        rt.send(Message::Command(CommandId::StyleManagement));
        assert!(rt.app.style_management.is_some());
        assert!(cosmic::Application::dialog(&rt.app).is_some());
    });
    assert!(s.app().style_management.as_ref().unwrap().sheet.is_some());
    s.act("select Sheeted", move |rt| {
        rt.send(Message::StyleManagement(ManagementAction::Select(sheeted)));
    });
    s.edit("Revert", |rt| {
        rt.send(Message::StyleManagement(ManagementAction::Revert));
    });
    assert_eq!(s.document().styles()[&sheeted].font_size_points, 18.0);
    assert_eq!(s.document().styles()[&sheeted].origin, StyleOrigin::Sheet);
    // Move: a sheet style stored in the document.
    s.edit("<< Move <<", |rt| {
        rt.send(Message::StyleManagement(ManagementAction::MoveToDocument));
    });
    assert_eq!(
        s.document().styles()[&sheeted].origin,
        StyleOrigin::Document
    );
    // Remove: the style is defined nowhere, its paragraphs format as Body Text.
    s.edit("Remove", |rt| {
        rt.send(Message::StyleManagement(ManagementAction::Remove));
    });
    // The sheet defines the name, so removing the document's copy leaves
    // the sheet's version (ST1).
    assert_eq!(s.document().styles()[&sheeted].origin, StyleOrigin::Sheet);
    s.act("Close Style Management", |rt| {
        rt.send(Message::StyleManagement(ManagementAction::Close));
    });
    assert!(s.app().style_management.is_none());

    // Use Another Style Sheet with a cancel: nothing changes.
    s.no_change("Use Another Style Sheet… Cancel", |rt| {
        rt.send(Message::Command(CommandId::UseAnotherStyleSheet));
        rt.send(Message::UseStyleSheet(UseSheetAction::Cancel));
        assert!(rt.app.use_sheet.is_none());
    });
    // A sheet chosen by Browse… (the chooser's finished message).
    let session_id = s.app().id;
    s.act("Browse… chose the sheet again", |rt| {
        rt.send(Message::SheetChosen {
            session_id,
            result: FileDialogResult::Selected(sheet.clone()),
        });
    });
    assert_eq!(
        s.document().style_sheet().map(|l| l.name.to_lowercase()),
        Some("scen.sty".into())
    );

    // (The minimal case is `a_document_using_another_style_sheet_saves_as_sam_with_no_loss_list`.)
    let path = s.save_sam("USEDSHEE.SAM");
    let s = s.reopen(&path);
    assert!(has_style(&s, "Sheeted"));
    assert_eq!(
        s.document().styles()[&style_id(&s, "Sheeted")].font_size_points,
        18.0
    );
}

#[test]
fn create_style_based_on_selected_text_and_on_a_row_then_modify_from_the_dialog() {
    let mut s = Scenario::new_document("styles-create-from-text");
    s.clean_folder();
    s.type_text("Text with its own indent");
    s.enter();
    s.type_text("Plain text");
    let main = s.main_flow();
    s.caret(main, 0, 2, 1, false);
    // Give the paragraph an indent of its own through the ruler's route:
    // the paragraph's own indents (First line).
    s.command_act(CommandId::ModifyCurrentStyle);
    s.act("close the panel", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Close));
    });

    // Selected text: the caret paragraph's style with its own formatting.
    s.command(CommandId::Center);
    s.edit("Create Style from the selected text", |rt| {
        rt.send(Message::Command(CommandId::CreateStyle));
        rt.send(Message::CreateStyle(CreateAction::Name(
            "Centred Copy".into(),
        )));
        rt.send(Message::CreateStyle(CreateAction::SelectedText(true)));
        rt.send(Message::CreateStyle(CreateAction::Create));
    });
    let copy = style_id(&s, "Centred Copy");
    assert_eq!(
        s.document().styles()[&copy].alignment,
        Alignment::Centre,
        "the paragraph's own alignment joined the style"
    );

    // Create Style Based on This… from a row, then Modify opens the rail on
    // the new style (Create Style's Modify…).
    row_menu(
        &mut s,
        "row menu: Create Style Based on This…",
        copy,
        StyleRowAction::CreateBasedOn,
    );
    assert_eq!(s.app().create_style.as_ref().unwrap().base(), Some(copy));
    s.edit("Create and Modify…", |rt| {
        rt.send(Message::CreateStyle(CreateAction::Name(
            "Centred Child".into(),
        )));
        rt.send(Message::CreateStyle(CreateAction::Modify));
        let shown = rt.app.style_properties.as_ref().unwrap().style;
        assert_eq!(
            rt.app.document.styles()[&shown].name,
            "Centred Child",
            "Modify opens the rail on the new style"
        );
    });
    let child = style_id(&s, "Centred Child");
    assert_eq!(s.document().styles()[&child].alignment, Alignment::Centre);
    typed(&mut s, "Child size 14", Field::Size, "14");

    s.caret(main, 1, 0, 1, false);
    s.edit("apply Centred Child", move |rt| {
        rt.send(Message::ApplyStyle(child));
    });
    let path = s.save_sam("CHILD.SAM");
    let s = s.reopen(&path);
    assert_eq!(style_name_of(&s, 1), "Centred Child");
}

#[test]
fn create_style_refuses_a_taken_name_without_a_failure_message() {
    // `UI_SPEC.md` §14: a taken name (letter case aside) disables Create
    // and says so. Enter in the name box is wired to Create, so it must
    // wait too, as the Create button does.
    let mut s = Scenario::new_document("styles-create-taken-name");
    s.type_text("Text");
    s.no_change(
        "Create Style with a taken name, Enter in the name box",
        |rt| {
            rt.send(Message::Command(CommandId::CreateStyle));
            rt.send(Message::CreateStyle(CreateAction::Name("HEADING 1".into())));
            assert!(cosmic::Application::dialog(&rt.app).is_some());
            rt.send(Message::CreateStyle(CreateAction::Create));
            assert!(rt.app.create_style.is_some(), "Create waits for a name");
        },
    );
    s.no_change("Cancel", |rt| {
        rt.send(Message::CreateStyle(CreateAction::Cancel));
        assert!(rt.app.create_style.is_none());
    });
}

#[test]
fn create_style_refuses_an_empty_name_without_a_failure_message() {
    let mut s = Scenario::new_document("styles-create-empty-name");
    s.type_text("Text");
    s.no_change("Create Style with no name, Enter in the name box", |rt| {
        rt.send(Message::Command(CommandId::CreateStyle));
        rt.send(Message::CreateStyle(CreateAction::Create));
        assert!(rt.app.create_style.is_some());
    });
    s.no_change("Cancel", |rt| {
        rt.send(Message::CreateStyle(CreateAction::Cancel));
    });
}

#[test]
fn rename_style_refuses_a_taken_name_without_a_failure_message() {
    // `UI_SPEC.md` §14: the Rename dialog refuses a taken name; Enter in the
    // name box is wired to Rename as the button is, so it must wait too.
    let mut s = Scenario::new_document("styles-rename-taken-name");
    s.type_text("Text");
    create_style(&mut s, "Alpha");
    let alpha = style_id(&s, "Alpha");
    row_menu(&mut s, "row menu: Rename", alpha, StyleRowAction::Rename);
    s.no_change("Rename to a taken name, Enter in the name box", |rt| {
        rt.send(Message::RenameStyle(RenameAction::Name("heading 2".into())));
        rt.send(Message::RenameStyle(RenameAction::Rename));
        assert!(rt.app.rename_style.is_some(), "a taken name is refused");
    });
    s.no_change("Cancel", |rt| {
        rt.send(Message::RenameStyle(RenameAction::Cancel));
    });
}

#[test]
fn a_document_using_another_style_sheet_saves_as_sam_with_no_loss_list() {
    let mut s = Scenario::new_document("styles-sheet-then-sam");
    let folder = s.clean_folder();
    s.type_text("Source");
    create_style(&mut s, "Sheeted");
    let session_id = s.app().id;
    let sheet = folder.join("MINI.STY");
    s.act("Save as a Style Sheet", |rt| {
        rt.send(Message::Command(CommandId::SaveAsStyleSheet));
        rt.send(Message::SaveSheet(SaveSheetAction::Description(
            "Mini".into(),
        )));
        let task =
            cosmic::Application::update(&mut rt.app, Message::SaveSheet(SaveSheetAction::Choose));
        drop(cosmic::iced::runtime::task::into_stream(task));
        rt.send(Message::SheetDialogFinished {
            session_id,
            description: "Mini".into(),
            contents: false,
            result: FileDialogResult::Selected(sheet.clone()),
        });
    });
    assert!(sheet.exists());
    s.act("File › New", |rt| {
        rt.send(Message::Command(CommandId::NewDocument));
    });
    s.type_text("Second document");
    let session_id = s.app().id;
    s.edit("Use Another Style Sheet: MINI.STY", |rt| {
        rt.send(Message::SheetChosen {
            session_id,
            result: FileDialogResult::Selected(sheet.clone()),
        });
    });
    assert!(has_style(&s, "Sheeted"));
    s.save_sam("MINI.SAM");
}

/// Paragraph 0's lines of the page layout: the byte range and the width of
/// each, for the first page.
fn first_paragraph_lines(
    layout: &amipro_layout::DocumentLayout,
) -> Vec<(std::ops::Range<usize>, f32)> {
    layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| {
            line.container == amipro_layout::Container::Main && line.paragraph_index == 0
        })
        .map(|line| (line.bytes.clone(), line.width))
        .collect()
}

/// The weight of the run drawn for the text holding `word` on page 1, as
/// the print and PDF paths draw it.
fn drawn_weight(s: &Scenario, word: &str) -> u16 {
    let layout = s.app().pages.layout().clone();
    let runs = crate::print::drawn_weights(s.document(), &layout, 0);
    runs.iter()
        .find(|(text, _)| text.contains(word))
        .unwrap_or_else(|| s.fail("paint", &format!("no run draws {word:?}: {runs:?}")))
        .1
}

#[test]
fn a_1st_line_bold_style_sets_each_paragraphs_first_line_bold_through_edits_and_a_saved_file() {
    let mut s = Scenario::new_document("styles-first-line-bold");
    s.clean_folder();
    let words = "Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi \
                 and then some more words follow on the later lines of this paragraph until the very last Omega.";
    s.type_text(words);
    s.enter();
    s.type_text("Short second paragraph.");
    let main = s.main_flow();
    s.caret(main, 0, 0, 1, false);

    // A style of its own, applied to the first paragraph.
    create_style(&mut s, "Lead");
    let lead = style_id(&s, "Lead");
    s.edit("apply Lead to paragraph 0", move |rt| {
        rt.send(Message::ApplyStyle(lead));
    });
    let regular_lines = first_paragraph_lines(s.app().pages.layout());
    assert!(regular_lines.len() >= 3, "{regular_lines:?}");
    s.command_act(CommandId::ModifyCurrentStyle);
    assert_eq!(properties_style(&s), lead);

    // Bold first, then 1st line bold excludes it and the reverse.
    property(
        &mut s,
        "Bold on",
        StyleAction::Attribute(FontAttribute::Bold, true),
    );
    property(
        &mut s,
        "1st line bold on",
        StyleAction::Attribute(FontAttribute::FirstLineBold, true),
    );
    let style = s.document().styles()[&lead].clone();
    assert!(style.first_line_bold && !style.bold, "exclusive with Bold");
    assert_eq!(
        s.app().style_properties.as_ref().unwrap().draft,
        style,
        "the panel shows it"
    );

    // The first line is set bold and broken with the bold widths: it is the
    // first line of the same style set bold throughout; the later lines are
    // regular.
    let lines = first_paragraph_lines(s.app().pages.layout());
    let mut all_bold = s.document().clone();
    {
        let style = all_bold.styles_mut().get_mut(&lead).unwrap();
        style.first_line_bold = false;
        style.bold = true;
    }
    let mut pages = crate::page_layout::SessionPages::default();
    pages.update(&all_bold);
    let bold_lines = first_paragraph_lines(pages.layout());
    assert_eq!(
        lines[0], bold_lines[0],
        "the first line is broken as bold text"
    );
    assert!(
        lines[0].1 > regular_lines[0].1 * 0.9 && lines[0].0.end <= regular_lines[0].0.end,
        "bold is no narrower: {lines:?} against {regular_lines:?}"
    );
    assert!(lines.len() >= 3);
    assert!(
        drawn_weight(&s, "Alpha") >= 700,
        "the first line is drawn bold"
    );
    assert!(drawn_weight(&s, "Omega") < 600, "the last line is not");
    assert!(
        drawn_weight(&s, "Short") < 600,
        "another style's paragraph is not"
    );

    // Edit the text so the wrapping changes: the first line is still bold
    // and broken as bold text, the new text's lines are regular.
    s.caret(main, 0, 0, 1, false);
    s.type_text("Prelude with a few additional words in front of it, ");
    let lines = first_paragraph_lines(s.app().pages.layout());
    let mut all_bold = s.document().clone();
    {
        let style = all_bold.styles_mut().get_mut(&lead).unwrap();
        style.first_line_bold = false;
        style.bold = true;
    }
    let mut pages = crate::page_layout::SessionPages::default();
    pages.update(&all_bold);
    assert_eq!(
        lines[0],
        first_paragraph_lines(pages.layout())[0],
        "after typing"
    );
    assert!(drawn_weight(&s, "Prelude") >= 700);
    assert!(drawn_weight(&s, "Omega") < 600);

    // Save as SAM (no loss) and reopen: the style and the look are back.
    s.act("close Style Properties", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Close));
    });
    let path = s.save_sam("FLBOLD.SAM");
    let mut s = s.reopen(&path);
    let lead = style_id(&s, "Lead");
    assert!(s.document().styles()[&lead].first_line_bold);
    assert!(!s.document().styles()[&lead].bold);
    assert!(!s.document().styles()[&lead].attributes.protected);
    assert!(drawn_weight(&s, "Prelude") >= 700);
    assert!(drawn_weight(&s, "Omega") < 600);

    // Unchecking it (one undo step) sets the paragraph regular again.
    s.command_act(CommandId::ModifyCurrentStyle);
    property(
        &mut s,
        "1st line bold off",
        StyleAction::Attribute(FontAttribute::FirstLineBold, false),
    );
    assert!(drawn_weight(&s, "Prelude") < 600);
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(s.document().styles()[&lead].first_line_bold);
    assert!(drawn_weight(&s, "Prelude") >= 700);
}
