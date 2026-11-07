//! Page layout (`UI_SPEC.md` §18–19): Page Layout Properties (margins,
//! columns, paper, header and footer), page and column breaks, inserted page
//! layouts, floating headers and footers, page numbers, Go To Header and
//! Footer, and the ruler's tabs, indents, and columns.

use amipro_document::{Indents, TabKind};
use amipro_editing::layout_setup::{Band, MarginSide, Pages};
use cosmic::iced::mouse::Button;

use crate::page_dialogs::{FloatingAction, PageNumberAction};
use crate::page_layout_properties::{Field, LayoutAction, Section};

use super::*;

/// A layout setting changed in the rail, as one undo step.
fn layout(s: &mut Scenario, what: &str, action: LayoutAction) {
    s.edit(what, move |rt| {
        rt.send(Message::PageLayoutProperties(action));
    });
}

/// A typed value in a layout box, then Enter.
fn typed(s: &mut Scenario, what: &str, field: Field, text: &str) {
    let text = text.to_owned();
    s.edit(what, move |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Text(
            field, text,
        )));
        rt.send(Message::PageLayoutProperties(LayoutAction::Submit(field)));
    });
}

fn standard(s: &Scenario) -> amipro_document::PageLayout {
    s.document().document_layout().clone()
}

/// Paragraphs of text long enough to fill a few lines each.
fn type_paragraphs(s: &mut Scenario, count: usize) {
    for index in 0..count {
        if index > 0 {
            s.enter();
        }
        s.type_text(&format!(
            "Paragraph {} of the report: the quick brown fox jumps over the lazy dog, again and again, to make a few lines of text.",
            index + 1
        ));
    }
}

#[test]
fn the_page_is_set_up_in_the_rail_margins_paper_columns_and_lines_then_saved() {
    let mut s = Scenario::new_document("layout-rail");
    s.clean_folder();
    type_paragraphs(&mut s, 4);
    let main = s.main_flow();
    s.caret(main, 1, 5, 1, false);

    s.command_act(CommandId::PageLayout);
    assert!(
        s.app().rail_open(),
        "the rail opens on Page Layout Properties"
    );
    assert!(
        cosmic::Application::dialog(s.app()).is_none(),
        "no modal dialog"
    );
    let before = standard(&s);
    assert_eq!(
        s.app().page_layout_properties.as_ref().unwrap().layout,
        before.id,
        "the caret page's layout"
    );

    // A typed margin waits for Enter; a value Ami Pro refuses is refused in
    // place with the reason.
    let left = Field::Margin(MarginSide::Left);
    s.no_change("type a left margin without Enter", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Text(
            left,
            "1.5".into(),
        )));
    });
    let original_left = standard(&s).right.layout.margins.left;
    s.edit("Enter on 1.5", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Submit(left)));
    });
    assert_eq!(standard(&s).right.layout.margins.left, 2_160);
    assert_ne!(original_left, 2_160);
    assert_eq!(s.app().pages.layout().pages[0].columns[0].x, 2_160.0);
    s.no_change("a left margin with no room", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Text(
            left,
            "8".into(),
        )));
        rt.send(Message::PageLayoutProperties(LayoutAction::Submit(left)));
        let problem = rt
            .app
            .page_layout_properties
            .as_ref()
            .unwrap()
            .error
            .clone();
        assert!(problem.is_some_and(|text| text.contains("no room")));
    });
    assert_eq!(standard(&s).right.layout.margins.left, 2_160);
    typed(
        &mut s,
        "Top margin 0.75",
        Field::Margin(MarginSide::Top),
        "0.75",
    );
    assert_eq!(standard(&s).right.layout.margins.top, 1_080);

    // Paper: A4 portrait, then landscape.
    layout(&mut s, "Paper: A4", LayoutAction::Paper(2));
    assert_eq!(standard(&s).right.width, 11_908);
    layout(&mut s, "Landscape", LayoutAction::Landscape(true));
    assert_eq!(
        (standard(&s).right.width, standard(&s).right.height),
        (16_833, 11_908)
    );
    layout(&mut s, "Portrait again", LayoutAction::Landscape(false));
    assert_eq!(standard(&s).right.width, 11_908);

    // Columns: two columns lay the text out in two columns; one again.
    s.act("open Margins and columns", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Section(
            Section::Margins,
        )));
    });
    layout(&mut s, "Two columns", LayoutAction::Columns(None, 2));
    assert_eq!(s.app().pages.layout().pages[0].columns.len(), 2);
    typed(&mut s, "Gutter 0.5", Field::Gutter, "0.5");
    assert_eq!(standard(&s).right.layout.gutter, 720);
    layout(&mut s, "Balance columns", LayoutAction::Balance(None, true));
    layout(
        &mut s,
        "Line between columns",
        LayoutAction::ColumnLine(true),
    );
    assert!(standard(&s).right.layout.line_between.is_some());

    // Tabs of the page layout.
    s.act("open Tabs", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Section(
            Section::Tabs,
        )));
    });
    layout(&mut s, "Add a tab", LayoutAction::AddTab(None));
    assert!(!standard(&s).right.layout.tabs[0].is_empty());
    layout(&mut s, "Tab kind: Right", LayoutAction::TabKind(None, 0, 1));
    layout(&mut s, "Remove the tab", LayoutAction::RemoveTab(None, 0));

    // Lines around the page.
    s.act("open Lines", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Section(
            Section::Lines,
        )));
    });
    layout(
        &mut s,
        "Line on all sides",
        LayoutAction::LineSide(crate::page_layout_properties::LineSide::All, true),
    );
    layout(&mut s, "Columns: one", LayoutAction::Columns(None, 1));
    assert_eq!(s.app().pages.layout().pages[0].columns.len(), 1);

    // Right pages: the first change gives the layout left pages; Mirror
    // reflects them.
    s.act("Pages: Right", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Pages(
            Pages::Right,
        )));
    });
    s.edit("Right pages: left margin 2", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Text(
            left,
            "2".into(),
        )));
        rt.send(Message::PageLayoutProperties(LayoutAction::Submit(left)));
    });
    assert!(standard(&s).left.is_some(), "left pages exist now");
    layout(&mut s, "Mirror", LayoutAction::Mirror(true));
    assert_eq!(
        standard(&s).left.as_ref().unwrap().layout.margins.right,
        2_880
    );

    let wanted = standard(&s);
    let path = s.save_sam("PAGESET.SAM");
    let s = s.reopen(&path);
    let read = standard(&s);
    assert_eq!(read.right.width, wanted.right.width);
    assert_eq!(read.right.height, wanted.right.height);
    assert_eq!(read.right.layout.margins, wanted.right.layout.margins);
    assert_eq!(read.right.layout.columns, 1);
    assert!(read.left.is_some());
    assert_eq!(
        read.left.as_ref().unwrap().layout.margins,
        wanted.left.as_ref().unwrap().layout.margins
    );
}

#[test]
fn breaks_and_inserted_layouts_make_pages_and_a_layout_can_be_changed_and_removed() {
    let mut s = Scenario::new_document("layout-breaks");
    s.clean_folder();
    type_paragraphs(&mut s, 3);
    let main = s.main_flow();
    assert_eq!(s.app().page_count(), 1);

    // A page break in the middle of paragraph 1: two pages, the paragraph
    // not split.
    s.caret(main, 1, 9, 1, false);
    s.command(CommandId::PageBreak);
    assert_eq!(s.document().paragraphs().len(), 3);
    assert_eq!(s.app().page_count(), 2);

    // A column break at the start of the paragraph, once.
    s.caret(main, 2, 5, 2, false);
    s.command(CommandId::ColumnBreak);
    s.expect_status("already");
    s.no_change("a second Column Break in the same paragraph", |rt| {
        rt.send(Message::Command(CommandId::ColumnBreak));
    });
    s.clear_expected_status();

    // An inserted page layout from the next page: a copy named 1; the rail
    // follows the caret onto it.
    s.caret(main, 2, 0, 2, false);
    s.command_act(CommandId::PageLayout);
    let layouts = s.document().layouts().len();
    s.edit("Insert Page Layout", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Insert));
    });
    assert_eq!(s.document().layouts().len(), layouts + 1);
    let inserted = s.document().layouts().last().unwrap().clone();
    assert_eq!(inserted.name, "1");
    let page = s.app().caret_page_number();
    assert!(page >= 2, "the caret is on a later page: {page}");
    assert_eq!(
        s.app().page_layout_properties.as_ref().unwrap().layout,
        inserted.id,
        "the panel follows the caret onto the inserted layout"
    );

    // Changing the inserted layout leaves the first page's alone.
    let first_margin = s.app().pages.layout().pages[0].columns[0].x;
    typed(
        &mut s,
        "Left margin 2 on the inserted layout",
        Field::Margin(MarginSide::Left),
        "2",
    );
    assert_eq!(s.app().pages.layout().pages[0].columns[0].x, first_margin);
    let last = s.app().pages.layout().pages.len();
    assert_eq!(s.app().pages.layout().pages[last - 1].columns[0].x, 2_880.0);

    // Remove the inserted layout: break and layout go in one undo step.
    s.edit("Remove the inserted layout", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Remove));
    });
    assert_eq!(s.document().layouts().len(), layouts);

    s.caret_to_end();
    s.type_text(" Final words.");
    let path = s.save_sam("BREAKS.SAM");
    let s = s.reopen(&path);
    assert!(s.app().page_count() >= 2);
}

/// Typing without the Undo and Redo check (see
/// `typing_in_the_header_survives_undo_and_redo_with_the_caret`).
fn type_unchecked(s: &mut Scenario, text: &str) {
    let text = text.to_owned();
    s.act(&format!("type {text:?} (not undo-checked)"), move |rt| {
        rt.send(Message::Page(PageAction::Insert(text)));
    });
}

#[test]
fn headers_footers_floating_ones_and_page_numbers_go_in_and_show_on_every_page() {
    let mut s = Scenario::new_document("layout-header-footer");
    s.clean_folder();
    type_paragraphs(&mut s, 2);
    let main = s.main_flow();
    // Three pages.
    s.caret(main, 0, 20, 1, false);
    s.command(CommandId::PageBreak);
    s.caret(main, 1, 10, 2, false);
    s.command(CommandId::PageBreak);
    assert_eq!(s.app().page_count(), 3);

    // Go To Header puts the caret in the page's header; text typed there.
    s.caret(main, 0, 0, 1, false);
    s.command_act(CommandId::GoToHeader);
    let header = s.app().page_selection.flow();
    assert_ne!(header, main, "the caret is in the header");
    type_unchecked(&mut s, "The Report");
    s.act("Enter in the header", |rt| {
        rt.send(Message::Page(PageAction::ParagraphBreak));
    });
    type_unchecked(&mut s, "second header line");
    // A page number there: style i from page 2, with leading text.
    s.command_act(CommandId::PageNumber);
    assert!(cosmic::Application::dialog(s.app()).is_some());
    s.act("choose Page number style i", |rt| {
        rt.send(Message::PageNumber(PageNumberAction::Style(2)));
        rt.send(Message::PageNumber(PageNumberAction::LeadingText(
            "Page ".into(),
        )));
    });
    // (Undo and Redo of a page number are checked on their own below.)
    s.act("Page Number… OK", |rt| {
        rt.send(Message::PageNumber(PageNumberAction::Ok));
        assert!(rt.app.page_number_dialog.is_none());
    });
    let shown = |s: &Scenario, number: usize| -> Option<String> {
        s.app().pages.layout().pages[number - 1]
            .lines
            .iter()
            .filter(|line| line.flow == header)
            .flat_map(|line| line.words.iter())
            .find_map(|word| word.text.clone())
    };
    assert!(
        shown(&s, 1).is_some() && shown(&s, 2).is_some() && shown(&s, 3).is_some(),
        "the number shows in the header of each page: {:?} {:?} {:?}",
        shown(&s, 1),
        shown(&s, 2),
        shown(&s, 3)
    );
    assert_ne!(shown(&s, 1), shown(&s, 2), "numbers differ between pages");

    // The same through Go To Footer.
    s.command_act(CommandId::GoToFooter);
    let footer = s.app().page_selection.flow();
    assert_ne!(footer, header);
    type_unchecked(&mut s, "Footer text");

    // A floating footer for right pages at the caret in the main text; the
    // caret goes into it and typed text shows on later right pages.
    let main_end = s.document().flow(main).unwrap().paragraphs[1].text.len();
    s.caret(main, 1, main_end, 2, false);
    s.command_act(CommandId::HeaderFooter);
    assert!(cosmic::Application::dialog(s.app()).is_some());
    s.act("Floating footer, right pages", |rt| {
        rt.send(Message::FloatingHeaderFooter(FloatingAction::Kind(
            amipro_document::HeaderFooterKind::Footer,
        )));
        rt.send(Message::FloatingHeaderFooter(FloatingAction::Pages(
            amipro_document::PageSides::Right,
        )));
    });
    s.edit("Floating Header/Footer… OK", |rt| {
        rt.send(Message::FloatingHeaderFooter(FloatingAction::Ok));
        assert!(rt.app.floating_dialog.is_none());
    });
    // (After the step's Undo and Redo the caret is in the main text; go
    // into the floating footer by clicking its mark, as the user does:
    // the caret is moved to the mark's flow through the dialog's result.)
    let floating = s.document().flow(main).unwrap().paragraphs[1]
        .objects
        .iter()
        .find_map(|object| match object {
            amipro_document::InlineObject::FloatingHeaderFooter { flow, .. } => Some(*flow),
            _ => None,
        })
        .unwrap_or_else(|| s.fail("floating", "no floating footer mark in paragraph 1"));
    s.caret(floating, 0, 0, 2, false);
    type_unchecked(&mut s, "Floating foot");

    // Begin on second page: the first page loses its header.
    s.caret(main, 0, 0, 1, false);
    s.command_act(CommandId::PageLayout);
    s.act("open Header", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Section(
            Section::Header,
        )));
    });
    layout(
        &mut s,
        "Header from the second page",
        LayoutAction::FromSecondPage(Band::Header, true),
    );
    s.expect_status("second page");
    s.no_change("Go To Header on page 1", |rt| {
        rt.send(Message::Command(CommandId::GoToHeader));
        assert_eq!(rt.app.page_selection.flow(), rt.app.document.main_flow_id());
    });
    s.clear_expected_status();

    let path = s.save_sam("HEADERS.SAM");
    let mut s = s.reopen(&path);
    assert_eq!(s.app().page_count(), 3);
    let main = s.main_flow();
    s.caret(main, 1, 2, 2, false);
    s.command_act(CommandId::GoToHeader);
    let flow = s.app().page_selection.flow();
    let text = s.document().flow(flow).unwrap().plain_text();
    assert!(text.contains("The Report"), "{text:?}");
}

#[test]
fn the_ruler_sets_tabs_indents_and_columns_and_its_menus_act_on_what_was_pressed() {
    let mut s = Scenario::new_document("layout-ruler");
    s.clean_folder();
    s.type_text("First paragraph of the ruler base, with some words in it.");
    s.enter();
    s.type_text("Second one.");
    s.enter();
    s.type_text("Third one.");
    s.act("zoom 100%", |rt| {
        rt.send(Message::ZoomTo(100));
        rt.redraw();
        rt.redraw();
    });
    let main = s.main_flow();
    let tabs = |s: &Scenario| s.document().paragraphs()[0].format.tabs.clone();

    // The caret in the first paragraph, by a click in its text.
    let text = s.rt.on_page(1_500.0, 1_560.0);
    s.act("click in the first paragraph", |rt| {
        rt.click(Button::Left, text);
    });
    assert_eq!(s.app().page_selection.focus.paragraph, 0);

    // A click in the upper half where no stop is adds a left tab there.
    let at_ruler = s.rt.on_ruler(1_440.0 + 1_100.0, 10.0);
    s.edit("click the ruler: a left tab", |rt| {
        rt.click(Button::Left, at_ruler);
    });
    let stops = tabs(&s).expect("the paragraph has its own ruler now");
    let added = stops
        .iter()
        .find(|stop| stop.kind == TabKind::Left && (stop.position - 1_100).abs() <= 15)
        .unwrap_or_else(|| s.fail("ruler", &format!("no left tab near 1100: {stops:?}")))
        .position;

    // Drag it right: on Ami Pro's grid, one undo step.
    let from = s.rt.on_ruler(1_440.0 + added as f32, 10.0);
    // (A tab cannot be dragged past the next stop, at 1440; the position is
    // truncated to Ami Pro's grid of 1/16 in.)
    let to = s.rt.on_ruler(1_440.0 + added as f32 + 300.0, 10.0);
    s.edit("drag the tab right", |rt| {
        rt.drag(from, to);
    });
    let stops = tabs(&s).unwrap();
    let moved = stops
        .iter()
        .find(|stop| (1_200..1_440).contains(&stop.position) && stop.position != added)
        .unwrap_or_else(|| s.fail("ruler", &format!("the tab did not move: {stops:?}")))
        .position;
    assert!(!stops.iter().any(|stop| stop.position == added));
    assert_eq!(moved % 90, 0, "on Ami Pro's grid");

    // The kind button, then another tab: a right tab.
    let (workspace, _) = s.rt.workspace_view();
    let button = cosmic::iced::Point::new(
        workspace.x + 12.0,
        workspace.y - crate::ruler_view::HEIGHT + 18.0,
    );
    s.act("kind button: next kind", |rt| {
        rt.click(Button::Left, button);
    });
    assert_eq!(s.app().ruler_tab.0, TabKind::Right);
    let at_ruler = s.rt.on_ruler(1_440.0 + 6_300.0, 10.0);
    s.edit("click the ruler: a right tab", |rt| {
        rt.click(Button::Left, at_ruler);
    });
    assert!(
        tabs(&s)
            .unwrap()
            .iter()
            .any(|stop| stop.kind == TabKind::Right)
    );

    // The stop's right-click menu: make it a Decimal tab; then drag it
    // off the ruler to remove it.
    let tab = s.rt.on_ruler(1_440.0 + moved as f32, 10.0);
    s.act("right-click the tab", |rt| {
        rt.click(Button::Right, tab);
    });
    assert!(s.app().ruler_context.is_some());
    s.edit("tab menu: Decimal", |rt| {
        let messages = rt.click(Button::Left, menu_row(tab, 3, 0));
        assert!(
            messages.iter().any(|m| matches!(
                m,
                Message::RulerMenu(ruler_view::RulerMenu::TabKind(TabKind::Decimal))
            )),
            "{messages:?}"
        );
    });
    assert!(
        tabs(&s)
            .unwrap()
            .iter()
            .any(|stop| stop.kind == TabKind::Decimal)
    );
    let count = tabs(&s).unwrap().len();
    s.edit("drag the decimal tab off the ruler", |rt| {
        let below = cosmic::iced::Point::new(tab.x, tab.y + 60.0);
        rt.drag(tab, below);
    });
    assert_eq!(tabs(&s).unwrap().len(), count - 1);

    // The first-line indent dragged half an inch; the right indent too.
    let from = s.rt.on_ruler(1_440.0, 3.0);
    let to = s.rt.on_ruler(1_440.0 + 730.0, 3.0);
    s.edit("drag the first-line indent", |rt| {
        rt.drag(from, to);
    });
    let indents: Indents = s.document().paragraphs()[0].format.indents.unwrap();
    assert_eq!((indents.all, indents.first, indents.rest), (0, 720, 0));
    // Typing goes on in the paragraph.
    s.caret(main, 0, 5, 1, false);
    s.type_text("X");

    // Clear Tabs from a gap on the ruler.
    let gap = s.rt.on_ruler(1_440.0 + 4_000.0, 10.0);
    s.act("right-click between the stops", |rt| {
        rt.click(Button::Right, gap);
    });
    s.edit("ruler menu: Clear Tabs", |rt| {
        let messages = rt.click(Button::Left, menu_row(gap, 0, 0));
        assert!(
            messages
                .iter()
                .any(|m| matches!(m, Message::RulerMenu(ruler_view::RulerMenu::ClearTabs))),
            "{messages:?}"
        );
    });
    assert_eq!(tabs(&s), Some(Vec::new()));

    // Columns: two from the rail, then the gap dragged in the lower half.
    s.command_act(CommandId::PageLayout);
    s.act("open Margins and columns", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Section(
            Section::Margins,
        )));
    });
    layout(&mut s, "Two columns", LayoutAction::Columns(None, 2));
    s.act("redraw", |rt| {
        rt.redraw();
    });
    let from = s.rt.on_ruler(6_120.0, 28.0);
    let to = s.rt.on_ruler(6_120.0 + 1_100.0, 28.0);
    s.edit("drag the gap between the columns", |rt| {
        rt.drag(from, to);
    });
    let columns = standard(&s).right.layout.column_edges.clone();
    assert!(
        columns.is_some_and(|edges| edges.len() == 2),
        "uneven columns"
    );
    assert_eq!(
        s.app().ruler_guide,
        None,
        "the guide is gone after the drag"
    );
    s.edit("even columns again", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::EvenColumns(
            None,
        )));
    });
    assert!(standard(&s).right.layout.column_edges.is_none());

    // View › Ruler hides and shows it; the document is untouched.
    s.no_change("View › Ruler off", |rt| {
        rt.send(Message::Command(CommandId::Ruler));
    });
    assert!(!s.app().preferences.ruler_visible);
    s.no_change("View › Ruler on", |rt| {
        rt.send(Message::Command(CommandId::Ruler));
    });

    let path = s.save_sam("RULER.SAM");
    let s = s.reopen(&path);
    let indents = s.document().paragraphs()[0].format.indents;
    assert_eq!(indents.map(|i| i.first), Some(720));
    assert_eq!(s.document().paragraphs()[0].format.tabs, Some(Vec::new()));
    assert_eq!(standard(&s).right.layout.columns, 2);
}

#[test]
fn typing_in_the_header_survives_undo_and_redo_with_the_caret() {
    // Undo and Redo of text typed in the main text return the caret where
    // the step left it (documents.rs); the same must hold in a header, a
    // footer, and a floating footer.
    let mut s = Scenario::new_document("layout-header-undo");
    type_paragraphs(&mut s, 1);
    s.caret(s.main_flow(), 0, 0, 1, false);
    s.command_act(CommandId::GoToHeader);
    s.type_text("The Report");
}

#[test]
fn typing_in_the_footer_survives_undo_and_redo_with_the_caret() {
    let mut s = Scenario::new_document("layout-footer-undo");
    type_paragraphs(&mut s, 1);
    s.caret(s.main_flow(), 0, 0, 1, false);
    s.command_act(CommandId::GoToFooter);
    s.type_text("Footer text");
}

#[test]
fn a_page_number_in_the_main_text_is_redone_with_the_caret_after_it() {
    let mut s = Scenario::new_document("layout-page-number-redo");
    s.type_text("Page ");
    s.command_act(CommandId::PageNumber);
    s.edit("Page Number… OK", |rt| {
        rt.send(Message::PageNumber(PageNumberAction::Ok));
    });
    s.type_text(" of the book");
}

#[test]
fn a_page_number_in_the_header_is_redone_with_the_caret_after_it() {
    let mut s = Scenario::new_document("layout-page-number-header-redo");
    s.type_text("Body");
    s.caret(s.main_flow(), 0, 0, 1, false);
    s.command_act(CommandId::GoToHeader);
    type_unchecked(&mut s, "Page ");
    s.command_act(CommandId::PageNumber);
    s.edit("Page Number… OK", |rt| {
        rt.send(Message::PageNumber(PageNumberAction::Ok));
    });
}

/// File › Page Setup… (Ami Pro's name for Modify Page Layout) opens the same
/// Page Layout Properties panel as Format › Page Layout….
#[test]
fn file_page_setup_opens_the_page_layout_panel_like_format_page_layout() {
    let mut s = Scenario::new_document("layout-page-setup");
    type_paragraphs(&mut s, 2);
    let main = s.main_flow();
    s.caret(main, 1, 5, 1, false);
    assert!(s.app().command_enabled(CommandId::PageSetup));
    assert_eq!(
        crate::command::metadata(CommandId::PageSetup).label,
        "Page Setup…"
    );

    s.command_act(CommandId::PageSetup);
    assert!(s.app().rail_open());
    let setup = s
        .app()
        .page_layout_properties
        .as_ref()
        .map(|state| state.layout)
        .unwrap_or_else(|| s.fail("Page Setup", "Page Layout Properties did not open"));
    assert_eq!(setup, standard(&s).id);
    s.act("close the panel", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Close));
    });
    assert!(s.app().page_layout_properties.is_none());
    s.command_act(CommandId::PageLayout);
    assert_eq!(
        s.app()
            .page_layout_properties
            .as_ref()
            .map(|state| state.layout),
        Some(setup),
        "Format > Page Layout opens the same panel"
    );
    // Opened again from File, the panel draws.
    s.command_act(CommandId::PageSetup);
    s.act("redraw with the panel", |rt| {
        rt.redraw();
    });
}

/// Format › Ruler ▸ and Page Layout Changes ▸ (`UI_SPEC.md` §39, CA.8): Ami
/// Pro's Page › Ruler and Insert Page Layout commands from the menu, each
/// one undo step, saved as SAM.
#[test]
fn ruler_and_page_layout_changes_from_the_menu() {
    let mut s = Scenario::new_document("ruler-layout-menu");
    s.clean_folder();
    s.type_text("First page text.");
    s.enter();
    s.type_text("More text.");
    s.select_main((0, 2), (0, 2));
    s.command(CommandId::InsertRuler);
    assert!(s.document().main_flow().paragraphs[0].format.tabs.is_some());
    assert!(s.document().main_flow().paragraphs[1].format.tabs.is_none());
    s.no_change("Insert Ruler again", |rt| {
        rt.send(Message::Command(CommandId::InsertRuler));
    });
    s.command(CommandId::RemoveRuler);
    assert!(s.document().main_flow().paragraphs[0].format.tabs.is_none());

    // Insert Page Layout: a copy from the next page, shown in the rail.
    let layouts = s.document().layouts().len();
    s.command(CommandId::InsertPageLayout);
    assert_eq!(s.document().layouts().len(), layouts + 1);
    let new = s.document().layouts().last().unwrap().id;
    assert_eq!(
        s.app()
            .page_layout_properties
            .as_ref()
            .map(|state| state.layout),
        Some(new)
    );
    // The caret went on to the new page; on the first page Remove and
    // Revert have nothing to do.
    s.select_main((0, 2), (0, 2));
    s.no_change("Remove on the first page", |rt| {
        rt.send(Message::Command(CommandId::RemovePageLayout));
    });
    s.no_change("Revert on the first page", |rt| {
        rt.send(Message::Command(CommandId::RevertPageLayout));
    });
    s.no_change("Remove on the first page again", |rt| {
        rt.send(Message::Command(CommandId::RemovePageLayout));
    });
    // On the new layout's page, Remove takes it away again.
    s.caret_to_end();
    s.command(CommandId::RemovePageLayout);
    assert_eq!(s.document().layouts().len(), layouts);

    // Inserted again, then Revert from its page: the standard layout
    // comes back from the next page.
    s.select_main((0, 2), (0, 2));
    s.command(CommandId::InsertPageLayout);
    s.caret_to_end();
    s.command(CommandId::RevertPageLayout);
    s.save_sam("LAYOUTS.SAM");
}

/// The tab stops the page ruler shows at the caret now.
fn ruler_tabs(s: &Scenario) -> Vec<amipro_document::TabStop> {
    crate::ruler::Ruler::at(s.document(), s.app().pages.layout(), s.app().page_selection)
        .unwrap_or_else(|| s.fail("ruler", "no ruler at the caret"))
        .tabs
}

/// Page › Ruler ▸ Insert Ruler and Remove Ruler (`COMMANDS_NOTES.md` §7)
/// as a user does them: a paragraph takes its own ruler, a tab set on it
/// shows on the page ruler only there, the selection's paragraphs all take
/// one, Remove gives the style's back, each one undo step, and the own rulers
/// survive Save As SAM and a fresh window.
#[test]
fn inserted_rulers_belong_to_their_paragraphs_show_on_the_ruler_and_are_saved() {
    let mut s = Scenario::new_document("layout-own-rulers");
    s.clean_folder();
    for index in 0..3 {
        if index > 0 {
            s.enter();
        }
        s.type_text(&format!("Paragraph {index} of the ruler test."));
    }
    s.act("zoom 100%", |rt| {
        rt.send(Message::ZoomTo(100));
        rt.redraw();
        rt.redraw();
    });
    let own =
        |s: &Scenario, paragraph: usize| s.document().paragraphs()[paragraph].format.tabs.clone();
    assert!(own(&s, 0).is_none() && own(&s, 1).is_none() && own(&s, 2).is_none());
    assert!(s.app().command_enabled(CommandId::InsertRuler));

    // The first paragraph takes its own ruler (a click in its text first).
    let text = s.rt.on_page(1_500.0, 1_560.0);
    s.act("click in the first paragraph", |rt| {
        rt.click(Button::Left, text);
    });
    assert_eq!(s.app().page_selection.focus.paragraph, 0);
    s.command(CommandId::InsertRuler);
    assert!(own(&s, 0).is_some() && own(&s, 1).is_none());

    // A tab clicked on the ruler goes to that paragraph alone.
    let at_ruler = s.rt.on_ruler(1_440.0 + 1_300.0, 10.0);
    s.edit("click the ruler: a tab", |rt| {
        rt.click(Button::Left, at_ruler);
    });
    let stops = own(&s, 0).expect("own ruler");
    // (The stops in effect, Ami Pro's every-half-inch ones, became its own.)
    let near = |stops: &[amipro_document::TabStop], at: i32| {
        stops
            .iter()
            .any(|stop| (stop.position - at).abs() <= 15 && stop.position % 720 != 0)
    };
    assert!(near(&stops, 1_300), "{stops:?}");
    assert_eq!(ruler_tabs(&s), stops, "the page ruler shows it");
    s.select_main((1, 3), (1, 3));
    assert!(
        !near(&ruler_tabs(&s), 1_300),
        "the second paragraph's ruler is the style's"
    );
    s.select_main((0, 3), (0, 3));
    assert_eq!(ruler_tabs(&s), stops);

    // Two paragraphs selected: both take their own ruler in one step.
    s.select_main((1, 2), (2, 4));
    s.command(CommandId::InsertRuler);
    assert!(own(&s, 1).is_some() && own(&s, 2).is_some());
    // The ruler on one of them gets its own tab, which the other lacks.
    s.select_main((2, 3), (2, 3));
    let second = s.rt.on_ruler(1_440.0 + 2_600.0, 10.0);
    s.edit("click the ruler on the third paragraph", |rt| {
        rt.click(Button::Left, second);
    });
    assert!(near(&own(&s, 2).unwrap(), 2_600));
    assert!(!near(&own(&s, 1).unwrap(), 2_600));
    assert!(!near(&own(&s, 2).unwrap(), 1_300));
    assert_eq!(own(&s, 0).unwrap(), stops);

    // Inserting again changes nothing.
    s.no_change("Insert Ruler again on the third paragraph", |rt| {
        rt.send(Message::Command(CommandId::InsertRuler));
    });

    // Remove Ruler on the middle paragraph; none to remove on the next press.
    s.select_main((1, 3), (1, 3));
    s.command(CommandId::RemoveRuler);
    assert!(own(&s, 1).is_none());
    s.no_change("Remove Ruler where there is none", |rt| {
        rt.send(Message::Command(CommandId::RemoveRuler));
    });

    // Undo takes the removal back, step by step.
    s.act("Undo the removal", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert!(own(&s, 1).is_some());
    s.act("Redo the removal", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert!(own(&s, 1).is_none());

    // Saved: the tab survives a fresh window; the removed one stays removed.
    let path = s.save_sam("OWNRULERS.SAM");
    let fresh = s.reopen(&path);
    let own_fresh = |paragraph: usize| fresh.document().paragraphs()[paragraph].format.tabs.clone();
    assert_eq!(own_fresh(0), own(&s, 0), "the first paragraph's tab");
    assert_eq!(own_fresh(2), own(&s, 2), "the third paragraph's tab");
    assert!(own_fresh(1).is_none(), "the removed ruler stays removed");
}

/// Format › Line Numbering… (`UI_SPEC.md`, CA.13): the numbers in the left
/// margin of every page, each setting one undo step, the numbers kept by
/// SAM's `[lnopts]` section, and none in Draft mode.
#[test]
fn line_numbering_is_set_in_the_rail_drawn_in_the_margin_saved_and_hidden_in_draft() {
    use amipro_document::{LineNumbering, LineNumberingEvery, LineNumberingMode};
    use amipro_layout::Container;

    fn numbers(s: &Scenario) -> Vec<Vec<String>> {
        s.app()
            .pages
            .layout()
            .line_numbers
            .iter()
            .map(|page| page.iter().map(|number| number.text.clone()).collect())
            .collect()
    }
    fn first(page: &[String]) -> u32 {
        page.first()
            .unwrap_or_else(|| panic!("the page has numbers"))
            .parse()
            .unwrap()
    }

    let mut s = Scenario::new_document("line-numbers");
    let folder = s.clean_folder();
    type_paragraphs(&mut s, 30);
    assert!(
        s.app().pages.layout().pages.len() >= 2,
        "the text fills more than one page"
    );

    // 2. The rail opens on the section; numbering is off by default.
    s.command_act(CommandId::LineNumbering);
    assert!(
        s.app().rail_open(),
        "the rail opens on Page Layout Properties"
    );
    assert!(
        cosmic::Application::dialog(s.app()).is_none(),
        "no modal dialog"
    );
    assert!(
        s.app()
            .page_layout_properties
            .as_ref()
            .unwrap()
            .open
            .contains(&Section::LineNumbering),
        "the Line numbering section is open"
    );
    assert!(s.app().pages.layout().line_numbers.is_empty());
    assert_eq!(
        s.document().line_numbering().mode(),
        LineNumberingMode::None
    );

    // 3. Number text lines.
    layout(&mut s, "Number text lines", LayoutAction::NumberingMode(2));
    assert_eq!(
        s.document().line_numbering().mode(),
        LineNumberingMode::TextLines
    );
    let all = numbers(&s);
    assert_eq!(all.len(), s.app().pages.layout().pages.len());
    assert_eq!(all[0][0], "1");
    assert_eq!(all[0][1], "2");
    assert_eq!(
        all[0].iter().map(String::as_str).collect::<Vec<_>>(),
        (1..=all[0].len())
            .map(|n| n.to_string())
            .collect::<Vec<_>>(),
        "page 1 counts 1, 2, 3, …"
    );
    assert_eq!(
        first(&all[1]),
        all[0].len() as u32 + 1,
        "page 2 goes on from page 1"
    );
    {
        let layout = s.app().pages.layout();
        for (index, page) in layout.pages.iter().enumerate() {
            let margin = page.columns[0].x;
            let baselines: Vec<f32> = page
                .lines
                .iter()
                .filter(|line| line.container == Container::Main)
                .map(|line| line.baseline)
                .collect();
            assert!(!layout.line_numbers[index].is_empty());
            for number in &layout.line_numbers[index] {
                assert!(
                    number.x < margin,
                    "a number at {} is left of the margin {margin}",
                    number.x
                );
                assert!(
                    baselines
                        .iter()
                        .any(|baseline| (baseline - number.baseline).abs() < 0.5),
                    "number {:?} on page {} stands on a text line's baseline",
                    number.text,
                    index + 1
                );
            }
        }
    }

    // 4. Every fifth line.
    layout(
        &mut s,
        "Every fifth line",
        LayoutAction::NumberingEvery(LineNumberingEvery::FifthLine),
    );
    let all = numbers(&s);
    assert!(all.iter().flatten().count() > 0);
    for text in all.iter().flatten() {
        assert_eq!(
            text.parse::<u32>().unwrap() % 5,
            0,
            "{text} is not a multiple of 5"
        );
    }
    assert_eq!(all[0][0], "5");

    // 5. Every 7 lines, typed in the N box; 0 is refused.
    typed(&mut s, "Every 7 lines", Field::LineInterval, "7");
    let numbering = s.document().line_numbering().clone();
    assert_eq!(
        numbering.flags,
        LineNumbering::TEXT_LINES | LineNumbering::EVERY_N
    );
    assert_eq!(numbering.interval, 7);
    let all = numbers(&s);
    assert_eq!(all[0][0], "7");
    assert_eq!(all[0][1], "14");
    for text in all.iter().flatten() {
        assert_eq!(
            text.parse::<u32>().unwrap() % 7,
            0,
            "{text} is not a multiple of 7"
        );
    }
    s.no_change("Every 0 lines is refused", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Text(
            Field::LineInterval,
            "0".into(),
        )));
        rt.send(Message::PageLayoutProperties(LayoutAction::Submit(
            Field::LineInterval,
        )));
        assert!(
            rt.app
                .page_layout_properties
                .as_ref()
                .unwrap()
                .error
                .is_some(),
            "the panel says why"
        );
    });
    assert_eq!(s.document().line_numbering(), &numbering);
    s.no_change("Every 100 lines is refused", |rt| {
        rt.send(Message::PageLayoutProperties(LayoutAction::Text(
            Field::LineInterval,
            "100".into(),
        )));
        rt.send(Message::PageLayoutProperties(LayoutAction::Submit(
            Field::LineInterval,
        )));
        assert!(
            rt.app
                .page_layout_properties
                .as_ref()
                .unwrap()
                .error
                .is_some()
        );
    });
    assert_eq!(s.document().line_numbering(), &numbering);

    // 6. Reset each page: page 2 starts again.
    let before_reset = first(&numbers(&s)[1]);
    layout(
        &mut s,
        "Reset each page",
        LayoutAction::NumberingReset(true),
    );
    assert!(s.document().line_numbering().reset_each_page());
    let all = numbers(&s);
    assert_eq!(
        all[1][0], "7",
        "page 2 counts from 1 again, so its first shown is 7"
    );
    assert_ne!(before_reset, 7, "the count had carried over before");

    // 7. Number all lines: the last page has slots below its text.
    layout(&mut s, "Number all lines", LayoutAction::NumberingMode(1));
    assert_eq!(
        s.document().line_numbering().mode(),
        LineNumberingMode::AllLines
    );
    layout(
        &mut s,
        "Every line",
        LayoutAction::NumberingEvery(LineNumberingEvery::Line),
    );
    let all = numbers(&s);
    {
        let layout = s.app().pages.layout();
        let last = layout.pages.len() - 1;
        let text_lines = layout.pages[last]
            .lines
            .iter()
            .filter(|line| line.container == Container::Main)
            .count();
        assert!(
            all[last].len() > text_lines,
            "the last page numbers {} slots for {text_lines} text lines",
            all[last].len()
        );
        let last_text = layout.pages[last]
            .lines
            .iter()
            .filter(|line| line.container == Container::Main)
            .map(|line| line.baseline)
            .fold(0.0, f32::max);
        assert!(
            layout.line_numbers[last]
                .iter()
                .any(|number| number.baseline > last_text + 1.0),
            "numbers stand past the last text line"
        );
    }
    // Back to every 7th, the state the file is checked in.
    typed(&mut s, "Every 7 lines again", Field::LineInterval, "7");
    let saved = s.document().line_numbering().clone();
    assert_eq!(saved.flags, 0x01 | 0x20 | 0x10);
    assert_eq!(saved.interval, 7);

    // 8. Undo and Redo walk back through the settings.
    s.act("Undo to the Every N setting", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(
        s.document().line_numbering().every(),
        LineNumberingEvery::Line
    );
    s.act("Undo to the text lines setting", |rt| {
        rt.send(Message::Command(CommandId::Undo));
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(
        s.document().line_numbering().flags,
        LineNumbering::TEXT_LINES | LineNumbering::EVERY_N | LineNumbering::RESET_EACH_PAGE
    );
    assert!(s.document().line_numbering().reset_each_page());
    s.act("Redo the three steps", |rt| {
        for _ in 0..3 {
            rt.send(Message::Command(CommandId::Redo));
        }
    });
    assert_eq!(s.document().line_numbering(), &saved);

    // 9. Save As SAM and open again.
    let path = s.save_sam("NUMBERS.SAM");
    let bytes = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("[lnopts]\r\n\t49\r\n\tBody Text\r\n\t7\r\n"),
        "the file holds the section: {:?}",
        text.find("[lnopts]")
            .map(|at| &text[at..(at + 40).min(text.len())])
    );
    let mut s = s.reopen(&path);
    assert_eq!(s.document().line_numbering(), &saved);
    assert!(!s.app().pages.layout().line_numbers.is_empty());
    assert_eq!(
        s.app().pages.layout().line_numbers.len(),
        s.app().pages.layout().pages.len()
    );
    drop(folder);

    // 10. Draft mode shows none; leaving it brings them back.
    s.command_act(CommandId::DraftMode);
    assert!(s.app().draft.is_some());
    assert!(s.app().pages.layout().line_numbers.is_empty());
    assert_eq!(
        s.document().line_numbering(),
        &saved,
        "the document keeps its settings"
    );
    s.command_act(CommandId::DraftMode);
    assert!(s.app().draft.is_none());
    assert!(!s.app().pages.layout().line_numbers.is_empty());
}
