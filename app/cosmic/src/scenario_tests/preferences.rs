//! Preferences that shape the next document (`UI_SPEC.md` §46 and §39):
//! Save new documents as, followed to the saved file, and Open documents
//! at, followed through new, opened, and second tabs and a restart. The
//! settings are a scratch configuration. The Save As chooser itself is the
//! desktop's and is not opened: what it is given is checked, and its answer
//! sent.

use crate::SaveFormat;
use crate::page_view::Zoom;
use crate::preferences_dialog::{OpenZoom, PreferencesAction as A};

use super::*;

/// Tools › Preferences, the changes, and Close.
fn set(s: &mut Scenario, what: &str, actions: Vec<A>) {
    s.command_act(CommandId::Preferences);
    s.no_change(what, move |rt| {
        for action in actions {
            rt.send(Message::Preferences(action));
        }
    });
    s.no_change("Close Preferences", |rt| {
        rt.send(Message::Preferences(A::Close));
    });
}

/// What the Save As chooser would be given: the name, the format it starts
/// on, and that format's filter.
fn offered(s: &Scenario) -> (String, SaveFormat, String) {
    let (name, format) = s.app().save_offer();
    (name, format, format!("{:?}", crate::save_filter(format)))
}

fn answer_save_as(s: &mut Scenario, path: &Path, format: SaveFormat) {
    let path = path.to_path_buf();
    s.act("Save As answers", move |rt| {
        let session_id = rt.app.id;
        rt.send(Message::SaveDialogFinished {
            session_id,
            result: crate::FileDialogResult::Selected(path),
            format: Some(format),
        });
        if rt.app.export_pending.is_some() {
            rt.send(Message::ExportAnyway);
        }
    });
}

#[test]
fn save_new_documents_as_sets_what_save_as_offers_and_writes() {
    let mut s = Scenario::new_document("preferences-save-type");
    let folder = s.clean_folder();
    let (settings, _) = scratch_settings(&folder);
    s.app_mut().settings = Some(settings);
    s.type_text("A new document.");

    // The default: Ami Pro.
    let (name, format, filter) = offered(&s);
    assert_eq!((name.as_str(), format), ("untitled.SAM", SaveFormat::Sam));
    assert!(
        filter.contains("*.SAM") && !filter.contains("*.md"),
        "{filter}"
    );

    set(
        &mut s,
        "Markdown",
        vec![A::SaveFormat(SaveFormat::Markdown)],
    );
    let (name, format, filter) = offered(&s);
    assert_eq!(
        (name.as_str(), format),
        ("untitled.md", SaveFormat::Markdown)
    );
    assert!(filter.contains("*.md"), "{filter}");
    assert_eq!(
        scratch_settings(&folder).1.save_format,
        SaveFormat::Markdown
    );
    // The chooser's answer for what it offered: a Markdown copy is written
    // and the tab keeps no file (§46).
    let markdown = folder.join(&name);
    answer_save_as(&mut s, &markdown, format);
    let written = std::fs::read_to_string(&markdown).expect("the Markdown copy");
    assert!(written.contains("A new document."), "{written}");
    assert!(s.app().location.path().is_none());
    assert!(s.app().dirty, "a copy does not save the document");

    set(&mut s, "native", vec![A::SaveFormat(SaveFormat::Native)]);
    let (name, format, filter) = offered(&s);
    assert_eq!(
        (name.as_str(), format),
        ("untitled.amipro", SaveFormat::Native)
    );
    assert!(filter.contains("*.amipro"), "{filter}");
    let native = folder.join(&name);
    answer_save_as(&mut s, &native, format);
    assert_eq!(s.app().location.path(), Some(native.as_path()));
    assert!(!s.app().dirty && native.exists());

    // The next new document takes the preference; the saved one keeps its
    // own offer.
    set(
        &mut s,
        "Ami Pro again",
        vec![A::SaveFormat(SaveFormat::Sam)],
    );
    assert_eq!(offered(&s).1, SaveFormat::Native);
    s.command_act(CommandId::NewDocument);
    assert_eq!(offered(&s).0, "untitled.SAM");
    s.type_text("The second.");
    let sam = folder.join("untitled.SAM");
    answer_save_as(&mut s, &sam, SaveFormat::Sam);
    assert_eq!(s.app().location.path(), Some(sam.as_path()));
    assert!(!s.app().dirty);

    // After a restart.
    set(
        &mut s,
        "Markdown",
        vec![A::SaveFormat(SaveFormat::Markdown)],
    );
    let mut restarted = Scenario::new_document("preferences-save-type");
    restarted.app_mut().preferences = scratch_settings(&folder).1;
    assert_eq!(offered(&restarted).0, "untitled.md");
}

#[test]
fn open_documents_at_is_followed_by_new_opened_and_second_tabs_on_screen() {
    let mut s = Scenario::new_document("preferences-zoom");
    let folder = s.clean_folder();
    let (settings, _) = scratch_settings(&folder);
    s.app_mut().settings = Some(settings);
    let scale = |s: &mut Scenario| s.rt.workspace_view().1.scale;
    s.command_act(CommandId::ActualSize);
    let actual = scale(&mut s);
    s.type_text("A file to open again.");
    let saved = s.save_sam("ZOOM.SAM");
    let copy = folder.join("ZOOMTWO.SAM");
    std::fs::copy(&saved, &copy).unwrap();

    set(
        &mut s,
        "Custom zoom 75%",
        vec![A::OpenZoom(OpenZoom::Custom), A::CustomZoom("75".into())],
    );
    assert_eq!(
        s.app().zoom,
        Zoom::Percent(100),
        "the open tab keeps its own"
    );

    // A new document is drawn at 75%.
    s.command_act(CommandId::NewDocument);
    assert_eq!(s.app().zoom, Zoom::Percent(75));
    assert!((scale(&mut s) - actual * 0.75).abs() < 1e-6);
    s.type_text("Typed at 75%.");

    // So is an opened one.
    s.open(&copy);
    assert_eq!(s.app().sessions.len(), 3);
    assert_eq!(s.app().zoom, Zoom::Percent(75));
    assert!((scale(&mut s) - actual * 0.75).abs() < 1e-6);

    // A zoom chosen for one tab stays with it, on screen too.
    s.command_act(CommandId::Enlarged);
    assert!((scale(&mut s) - actual * 2.0).abs() < 1e-6);
    for (tab, zoom, times) in [(1, 75, 0.75), (0, 100, 1.0), (2, 200, 2.0)] {
        s.act(&format!("tab {tab}"), move |rt| {
            rt.send(Message::ActivateTab(tab));
        });
        assert_eq!(s.app().zoom, Zoom::Percent(zoom), "tab {tab}");
        assert!((scale(&mut s) - actual * times).abs() < 1e-6, "tab {tab}");
    }

    set(&mut s, "100%", vec![A::OpenZoom(OpenZoom::ActualSize)]);
    s.command_act(CommandId::NewDocument);
    assert!((scale(&mut s) - actual).abs() < 1e-6);
    set(&mut s, "Full page", vec![A::OpenZoom(OpenZoom::FullPage)]);
    s.command_act(CommandId::NewDocument);
    assert_eq!(s.app().zoom, Zoom::FullPage);

    // After a restart the first tab opens as chosen.
    let preferences = scratch_settings(&folder).1;
    assert_eq!(preferences.open_zoom, OpenZoom::FullPage);
    assert_eq!(preferences.custom_zoom, 75);
    let mut restarted = Scenario::new_document("preferences-zoom");
    restarted.app_mut().preferences = preferences;
    restarted.act("the first update", |rt| {
        for session in &mut rt.app.sessions {
            session.zoom_opened = false;
        }
        rt.send(Message::Repaint);
    });
    assert_eq!(restarted.app().zoom, Zoom::FullPage);
}
