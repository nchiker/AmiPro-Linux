//! Run A's views (`UI_SPEC.md` §39, CM8–CM11): the zoom presets, Facing
//! Pages, and the view kept per tab.

use crate::page_view::Zoom;

use super::*;

fn checked(s: &Scenario, command: CommandId) -> bool {
    crate::menu_check(s.app(), command).unwrap_or_else(|| s.fail("menu", "no check mark"))
}

#[test]
fn zoom_presets_and_facing_pages_are_checked_and_kept_per_tab() {
    let text: Vec<String> = (0..150)
        .map(|paragraph| format!("Paragraph {paragraph} with enough words to fill pages."))
        .collect();
    let mut s = Scenario::new(
        "views-zoom",
        amipro_document::Document::from_plain_text(&text.join("\n")),
    );
    s.clean_folder();
    assert!(s.app().pages.layout().pages.len() >= 3);

    // Full Page (Ctrl+D), then again: back to the view before it.
    s.command_act(CommandId::Enlarged);
    assert!(checked(&s, CommandId::Enlarged));
    assert_eq!(s.app().zoom, Zoom::Percent(200));
    s.command_act(CommandId::FullPage);
    assert!(checked(&s, CommandId::FullPage) && !checked(&s, CommandId::Enlarged));
    let view = s.app().sessions[0].page_view();
    let viewport = s.app().sessions[0].shown_viewport();
    assert!(view.pages[0].height <= viewport.height);
    s.command_act(CommandId::FullPage);
    assert_eq!(s.app().zoom, Zoom::Percent(200));
    s.command_act(CommandId::ActualSize);
    assert!(checked(&s, CommandId::ActualSize));
    s.command_act(CommandId::CustomZoom);
    assert_eq!(s.app().zoom, Zoom::Percent(91));
    assert!(checked(&s, CommandId::CustomZoom));

    // Facing Pages: page 1 alone on the right, then pairs.
    assert!(checked(&s, CommandId::ContinuousPages));
    s.command_act(CommandId::TwoPages);
    assert!(checked(&s, CommandId::TwoPages) && !checked(&s, CommandId::ContinuousPages));
    let view = s.app().sessions[0].page_view();
    let centre = view.size.width / 2.0;
    assert!(view.pages[0].x > centre);
    assert!(view.pages[1].x + view.pages[1].width < centre);
    assert!((view.pages[1].y - view.pages[2].y).abs() < 0.5);
    // Typing still works and is one step in facing pages.
    s.caret_to_end();
    s.type_text("The end.");

    // Another tab keeps its own view.
    s.command_act(CommandId::NewDocument);
    assert!(!s.app().facing);
    assert_eq!(s.app().zoom, Zoom::default());
    s.act("back to the first tab", |rt| {
        rt.send(Message::ActivateTab(0));
    });
    assert!(s.app().facing);
    assert_eq!(s.app().zoom, Zoom::Percent(91));
    s.command_act(CommandId::ContinuousPages);
    assert!(!s.app().facing);
    s.save_sam("VIEWS.SAM");
}

#[test]
fn draft_mode_sets_the_text_at_the_window_width_and_clean_screen_hides_the_chrome() {
    let mut s = Scenario::new_document("views-draft");
    s.clean_folder();
    s.type_text("Draft text that wraps at the width of the window rather than the page.");
    let letter = s.app().pages.layout().pages[0].width;
    s.command_act(CommandId::DraftMode);
    assert!(checked(&s, CommandId::DraftMode));
    let width = s.app().draft.expect("draft is on");
    assert!((s.app().pages.layout().pages[0].width - width as f32).abs() < 1.0);
    assert!(s.app().pages.layout().pages[0].width != letter);
    // The document itself keeps its page layout; editing works as before.
    assert_eq!(s.document().document_layout().right.width, 12_240);
    s.enter();
    s.type_text("Typed in Draft.");
    // A wider window lays the draft out wider.
    let wider = cosmic::iced::Size::new(1_600.0, 900.0);
    s.act("the window widens", move |rt| {
        rt.send(Message::Page(PageAction::Viewport(wider)));
    });
    assert!(s.app().draft.unwrap() > width);
    s.command_act(CommandId::DraftMode);
    assert!(s.app().draft.is_none());
    assert!((s.app().pages.layout().pages[0].width - letter).abs() < 1.0);

    // Show Clean Screen: on with F11's command, off with Esc.
    s.command_act(CommandId::FullScreen);
    assert!(s.app().clean_screen && checked(&s, CommandId::FullScreen));
    s.act("Esc", |rt| {
        rt.send(Message::Find(crate::find_bar::FindAction::Escape));
    });
    assert!(!s.app().clean_screen);
    s.save_sam("DRAFT.SAM");
}

/// The window's pixels: how many are near `colour`.
fn pixels_near(rgb: &[u8], colour: [u8; 3]) -> usize {
    rgb.chunks(3)
        .filter(|pixel| {
            pixel
                .iter()
                .zip(colour)
                .all(|(value, wanted)| value.abs_diff(wanted) <= 6)
        })
        .count()
}

#[test]
fn view_show_marks_are_checked_drawn_and_kept_in_the_preferences_page() {
    let mut s = Scenario::new_document("views-marks");
    s.clean_folder();
    s.type_text("One\tTwo\tThree");
    s.enter();
    s.type_text("A second paragraph.");
    let mark = [89, 128, 191];
    let theme = cosmic::Theme::light();
    let before = pixels_near(&render_window(s.app(), (1_280, 800), &theme, false), mark);
    assert!(!checked(&s, CommandId::ShowTabsReturns));
    s.command_act(CommandId::ShowTabsReturns);
    assert!(checked(&s, CommandId::ShowTabsReturns));
    let after = pixels_near(&render_window(s.app(), (1_280, 800), &theme, false), mark);
    assert!(after > before + 10, "marks drawn: {before} → {after}");

    // Margins in Colour off: no margin shade left on the page.
    let shade = [223, 223, 223];
    let shaded = pixels_near(&render_window(s.app(), (1_280, 800), &theme, false), shade);
    s.command_act(CommandId::MarginsInColour);
    assert!(!checked(&s, CommandId::MarginsInColour));
    let white = pixels_near(&render_window(s.app(), (1_280, 800), &theme, false), shade);
    assert!(white + 20_000 < shaded, "margins: {shaded} → {white}");
    s.command_act(CommandId::ColumnGuides);
    s.command_act(CommandId::ShowPictures);
    assert!(checked(&s, CommandId::ColumnGuides) && !checked(&s, CommandId::ShowPictures));
    // The Preferences View page shows and sets the same settings.
    s.command_act(CommandId::Preferences);
    s.act("Tabs & Returns off on the View page", |rt| {
        rt.send(Message::Preferences(
            crate::preferences_dialog::PreferencesAction::View(
                crate::preferences_dialog::ViewSetting::TabsReturns,
                false,
            ),
        ));
        rt.send(Message::Preferences(
            crate::preferences_dialog::PreferencesAction::Close,
        ));
    });
    assert!(!checked(&s, CommandId::ShowTabsReturns));
    s.save_sam("MARKS.SAM");
}
