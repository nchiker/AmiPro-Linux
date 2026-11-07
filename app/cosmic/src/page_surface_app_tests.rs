//! The page surface through the application's real message path: editing in
//! every flow, history, clipboard commands, Esc, formatting, zoom, and the
//! whole window's view (Phase 2.6).

use std::time::{Duration, Instant};

use amipro_document::{Caps, CharacterRun, Document, FlowId, FrameContent, FrameId};
use amipro_editing::TextPosition;
use cosmic::iced::advanced::widget::Tree;
use cosmic::iced::advanced::{Renderer as _, layout};

use super::*;
use crate::page_surface::page_fixture;

pub(crate) fn page_app(document: Document) -> AmiProApp {
    let mut app = AmiProApp {
        core: cosmic::Core::default(),
        shift: false,
        control: false,
        last_find: None,
        spelling: crate::spelling::Spelling::default(),
        field_variables: std::collections::BTreeMap::new(),
        sessions: vec![DocumentSession::new(
            1,
            document,
            DocumentLocation::Untitled,
            "Ready".into(),
        )],
        tab_model: segmented_button::SingleSelectModel::default(),
        active_session: 0,
        next_session_id: 2,
        pending_close: None,
        import_dialog: None,
        close_after_save: None,
        styles_visible: false,
        rail: RailState::default(),
        workspace: WorkspaceState::default(),
        preferences: Preferences::default(),
        settings: None,
        updates: crate::updates_panel::UpdatesState::default(),
        update_backend: std::sync::Arc::new(crate::updates::OfflineBackend),
        toolbar_editor: None,
        preferences_dialog: None,
        new_document_dialog: None,
        revert_pending: false,
        password_dialog: None,
        clean_screen: false,
        #[cfg(feature = "page-surface")]
        special_character: false,
        #[cfg(feature = "page-surface")]
        font_families: Vec::new(),
        #[cfg(feature = "page-surface")]
        fast_format_ask: None,
        about: About::default(),
        show_about: false,
        help_drawer: None,
        quit_requested: false,
        clipboard_has_text: None,
        clipboard_has_image: None,
        #[cfg(feature = "page-surface")]
        cell_clipboard: None,
        draw_memory: None,
        draw_clipboard: None,
        page_keyboard: 0,
        page_surface: true,
    };
    app.sync_tab_model();
    send(
        &mut app,
        Message::Page(PageAction::Viewport(cosmic::iced::Size::new(
            1_000.0, 700.0,
        ))),
    );
    app
}

pub(crate) fn send(app: &mut AmiProApp, message: Message) {
    drop(cosmic::Application::update(app, message));
}

fn page(app: &mut AmiProApp, action: PageAction) {
    send(app, Message::Page(action));
}

fn flow_of(document: &Document, frame: FrameId) -> FlowId {
    let FrameContent::Text(flow) = document.frame(frame).unwrap().content else {
        panic!("a text frame")
    };
    flow
}

fn text_frame(document: &Document) -> FrameId {
    *document
        .frame_stack()
        .iter()
        .find(|id| matches!(document.frame(**id).unwrap().content, FrameContent::Text(_)))
        .unwrap()
}

pub(crate) fn at(flow: FlowId, paragraph: usize, byte: usize) -> TextPosition {
    TextPosition {
        flow,
        paragraph,
        byte,
    }
}

pub(crate) fn move_to(app: &mut AmiProApp, position: TextPosition, page_number: u32, extend: bool) {
    page(
        app,
        PageAction::MoveTo {
            position,
            page: page_number,
            extend,
        },
    );
}

fn flow_text(app: &AmiProApp, flow: FlowId) -> String {
    app.document.flow(flow).unwrap().plain_text()
}

fn assert_editor_in_sync(app: &AmiProApp) {
    let mut text = app.editor.text();
    if text.ends_with('\n') && !app.document.plain_text().ends_with('\n') {
        text.pop();
    }
    assert_eq!(
        text,
        app.document.plain_text(),
        "the hidden editor follows the model"
    );
}

#[test]
fn typing_in_main_frame_and_header_text_goes_through_history() {
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    let frame = text_frame(&app.document);
    let frame_flow = flow_of(&app.document, frame);
    let header = app.document.document_layout().header.right.unwrap();
    let header_flow = flow_of(&app.document, header);

    // Frame text.
    move_to(&mut app, at(frame_flow, 0, 0), 1, false);
    page(&mut app, PageAction::Insert("ä".into()));
    assert_eq!(flow_text(&app, frame_flow), "äFrame é text");
    assert_eq!(app.page_selection.focus, at(frame_flow, 0, 2));
    assert!(app.dirty);

    // The header on page 2: every page's header shows the change.
    move_to(&mut app, at(header_flow, 0, 6), 2, false);
    page(&mut app, PageAction::Insert("!".into()));
    assert_eq!(flow_text(&app, header_flow), "Header! ü");
    assert_eq!(
        app.page_selection.page, 2,
        "the caret stays on the clicked instance"
    );
    let headers_on_pages = app
        .pages
        .layout()
        .pages
        .iter()
        .filter(|page| {
            page.lines
                .iter()
                .any(|line| line.flow == header_flow && line.bytes.end == "Header! ü".len())
        })
        .count();
    assert_eq!(headers_on_pages, app.pages.layout().pages.len());

    // Main text: Enter, then Backspace joins again.
    let paragraphs = app.document.paragraphs().len();
    move_to(&mut app, at(main, 3, 9), 1, false);
    page(&mut app, PageAction::ParagraphBreak);
    assert_eq!(app.document.paragraphs().len(), paragraphs + 1);
    assert_eq!(app.page_selection.focus, at(main, 4, 0));
    assert_editor_in_sync(&app);
    page(&mut app, PageAction::Backspace);
    assert_eq!(app.document.paragraphs().len(), paragraphs);
    assert_eq!(app.page_selection.focus, at(main, 3, 9));
    assert_editor_in_sync(&app);

    // Undo the join, the split, the header, and the frame edit; redo one.
    for _ in 0..4 {
        send(&mut app, Message::Command(CommandId::Undo));
    }
    assert_eq!(flow_text(&app, frame_flow), "Frame é text");
    assert_eq!(flow_text(&app, header_flow), "Header ü");
    assert_eq!(app.document.paragraphs().len(), paragraphs);
    assert_eq!(
        page_editing::valid_selection(&app.document, app.page_selection),
        app.page_selection
    );
    send(&mut app, Message::Command(CommandId::Redo));
    assert_eq!(flow_text(&app, frame_flow), "äFrame é text");
    assert_editor_in_sync(&app);
}

/// T1 (`UI_SPEC.md` §24) through the application's messages: in a cell
/// holding a picture, typing, Enter, and Backspace do nothing and the status
/// bar says why; Delete (the key, and Edit › Delete) removes the picture as
/// one undo step; Select Next Frame passes it by.
#[test]
fn a_cells_picture_takes_delete_and_ignores_typing() {
    let (document, table, picture, flows) = crate::page_surface::cell_picture_fixture();
    let mut app = page_app(document.clone());
    move_to(&mut app, at(flows[1], 0, 0), 1, false);
    assert_eq!(app.page_selection.focus.flow, flows[1]);
    for action in [
        PageAction::Insert("typed".into()),
        PageAction::ParagraphBreak,
        PageAction::Backspace,
    ] {
        page(&mut app, action.clone());
        assert_eq!(app.document, document, "{action:?} does nothing there");
    }
    assert_eq!(
        crate::page_surface::cell_frame_note(&app.document, app.page_selection),
        Some("This cell holds a picture; Delete removes it")
    );
    assert!(app.command_enabled(CommandId::Delete));

    page(&mut app, PageAction::Delete);
    assert!(app.document.frame(picture).is_none());
    assert_eq!(app.document.cell_holding(picture), None);
    assert!(app.document.frame(table).is_some());
    assert_eq!(
        app.page_selection.focus.flow, flows[1],
        "the caret stays in the cell"
    );
    assert_eq!(
        crate::page_surface::cell_frame_note(&app.document, app.page_selection),
        None
    );
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document, document, "one undo step");

    // Edit › Delete does the same.
    move_to(&mut app, at(flows[1], 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Delete));
    assert!(app.document.frame(picture).is_none());
    send(&mut app, Message::Command(CommandId::Undo));

    // Select Next Frame takes the table, never the picture in it.
    for _ in 0..3 {
        send(&mut app, Message::Command(CommandId::NextFrame));
        assert!(
            app.frame_selection
                .as_ref()
                .is_none_or(|selection| !selection.frames.contains(&picture)),
            "{:?}",
            app.frame_selection
        );
    }
}

#[test]
fn clipboard_select_all_escape_and_formatting_follow_the_page_selection() {
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    let frame = text_frame(&app.document);
    let frame_flow = flow_of(&app.document, frame);

    move_to(&mut app, at(frame_flow, 0, 3), 1, false);
    send(&mut app, Message::Command(CommandId::SelectAll));
    assert!(app.has_selection());
    assert_eq!(app.selected_text().as_deref(), Some("Frame é text"));

    // Bold applies to the frame's text, not the main text.
    send(&mut app, Message::Command(CommandId::Bold));
    let paragraph = &app.document.flow(frame_flow).unwrap().paragraphs[0];
    assert!(
        paragraph
            .runs
            .iter()
            .any(|run| run.format.bold == Some(true))
    );
    assert!(
        app.document
            .paragraphs()
            .iter()
            .all(|paragraph| paragraph.runs.iter().all(|run| run.format.bold.is_none()))
    );

    // Cut, then paste back as plain text.
    send(&mut app, Message::Command(CommandId::Cut));
    assert_eq!(flow_text(&app, frame_flow), "");
    app.insert_clipboard_text("Pasted".into());
    assert_eq!(flow_text(&app, frame_flow), "Pasted");

    // A paragraph style applies to the frame's paragraph.
    let style = *app.document.styles().keys().last().unwrap();
    send(&mut app, Message::ApplyStyle(style));
    assert_eq!(
        app.document.flow(frame_flow).unwrap().paragraphs[0].style,
        style
    );

    // Esc in frame text selects the frame (`UI_SPEC.md` §11); Esc again
    // (the surface's Deselect) leaves it for the main text on its page.
    page(&mut app, PageAction::Escape);
    assert_eq!(
        app.frame_selection,
        Some(crate::frame_editing::FrameSelection::one(frame, 1))
    );
    assert!(
        !app.has_selection(),
        "no text selection with a frame selected"
    );
    send(
        &mut app,
        Message::Frame(crate::frame_editing::FrameAction::Deselect),
    );
    assert_eq!(app.frame_selection, None);
    assert_eq!(app.page_selection.flow(), main);
    assert_eq!(app.page_selection.page, 1);

    // Copy in the main text.
    move_to(&mut app, at(main, 0, 0), 1, false);
    move_to(&mut app, at(main, 0, 9), 1, true);
    assert_eq!(app.selected_text().as_deref(), Some("Paragraph"));
    assert_eq!(
        app.editor.selection().as_deref(),
        Some("Paragraph"),
        "the hidden editor shares a main-text selection"
    );
}

#[test]
fn zoom_commands_share_one_transform_and_render_through_the_application_view() {
    let mut app = page_app(page_fixture());
    assert!(app.command_enabled(CommandId::ZoomIn));
    let fit = app.page_view().percent();
    assert_eq!(
        fit, 100,
        "Fit width stops at actual size in a wide workspace"
    );
    send(&mut app, Message::Command(CommandId::ZoomIn));
    assert_eq!(app.zoom, Zoom::Percent(125));
    send(&mut app, Message::Command(CommandId::ZoomOut));
    send(&mut app, Message::Command(CommandId::ZoomOut));
    assert_eq!(app.zoom, Zoom::Percent(90));
    send(&mut app, Message::Command(CommandId::ActualSize));
    assert_eq!(app.page_view().percent(), 100);
    send(&mut app, Message::ZoomTo(200));
    assert_eq!(app.page_view().percent(), 200);
    send(&mut app, Message::Command(CommandId::FitPageWidth));
    assert_eq!(app.zoom, Zoom::FitWidth);

    // The whole window lays out and draws with the page surface.
    let renderer_window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut renderer = cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
        cosmic::iced::Font::default(),
        cosmic::iced::Pixels(14.0),
    ));
    let mut root = cosmic::Application::view(&app);
    let mut tree = Tree::new(&root);
    let node = root.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(cosmic::iced::Size::ZERO, renderer_window),
    );
    assert_eq!(node.size(), renderer_window);
    let viewport = cosmic::iced::Rectangle::with_size(renderer_window);
    renderer.reset(viewport);
    root.as_widget().draw(
        &tree,
        &mut renderer,
        &cosmic::Theme::default(),
        &cosmic::iced::advanced::renderer::Style::default(),
        cosmic::iced::advanced::Layout::new(&node),
        cosmic::iced::mouse::Cursor::Unavailable,
        &viewport,
    );
}

/// Headless frame: view, retained-tree layout, and tiny-skia draw recording,
/// as the default editor's baseline measures.
struct FrameHarness {
    renderer: cosmic::Renderer,
    theme: cosmic::Theme,
    tree: Option<Tree>,
    window: cosmic::iced::Size,
}

impl FrameHarness {
    fn new(window: cosmic::iced::Size) -> Self {
        Self {
            renderer: cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
                cosmic::iced::Font::default(),
                cosmic::iced::Pixels(14.0),
            )),
            theme: cosmic::Theme::default(),
            tree: None,
            window,
        }
    }

    fn frame(&mut self, app: &AmiProApp) -> (Duration, Duration, Duration) {
        let started = Instant::now();
        let mut root = cosmic::Application::view(app);
        match &mut self.tree {
            Some(tree) => tree.diff(root.as_widget_mut()),
            None => self.tree = Some(Tree::new(&root)),
        }
        let tree = self.tree.as_mut().unwrap();
        let viewed = Instant::now();
        let node = root.as_widget_mut().layout(
            tree,
            &self.renderer,
            &layout::Limits::new(cosmic::iced::Size::ZERO, self.window),
        );
        let laid_out = Instant::now();
        let viewport = cosmic::iced::Rectangle::with_size(self.window);
        self.renderer.reset(viewport);
        root.as_widget().draw(
            tree,
            &mut self.renderer,
            &self.theme,
            &cosmic::iced::advanced::renderer::Style::default(),
            cosmic::iced::advanced::Layout::new(&node),
            cosmic::iced::mouse::Cursor::Unavailable,
            &viewport,
        );
        let drawn = Instant::now();
        (viewed - started, laid_out - viewed, drawn - laid_out)
    }
}

fn summary(samples: &mut [Duration]) -> String {
    samples.sort_unstable();
    let at = |percent: usize| samples[(samples.len() - 1) * percent / 100].as_secs_f64() * 1_000.0;
    format!(
        "median {:.3} ms, p95 {:.3} ms, worst {:.3} ms",
        at(50),
        at(95),
        samples.last().unwrap().as_secs_f64() * 1_000.0
    )
}

/// Release-mode end-to-end typing baseline for the page surface on
/// `AMIPRO_PERF_SAM` (README31), or the test fixture: one message, then the
/// frame the user would see (view, layout, draw).
#[test]
#[ignore = "manual release-mode page-surface typing performance baseline"]
fn page_surface_typing_performance_baseline() {
    const SAMPLES: usize = 40;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let paragraphs = document.paragraphs().len();
    eprintln!(
        "page surface baseline: {paragraphs} paragraphs, {SAMPLES} samples per operation, {}x{} window",
        window.width, window.height
    );
    for (name, paragraph) in [
        ("start", 0),
        ("middle", paragraphs / 2),
        ("end", paragraphs - 1),
    ] {
        let mut app = page_app(document.clone());
        let main = app.document.main_flow_id();
        // `AMIPRO_PERF_DRAFT=1`: the same in View › Draft Mode (CA.15).
        if std::env::var_os("AMIPRO_PERF_DRAFT").is_some() {
            send(&mut app, Message::Command(CommandId::DraftMode));
            assert!(app.draft.is_some(), "Draft mode on");
        }
        // `AMIPRO_PERF_OUTLINE=1`: the same in View › Outline Mode (OB.3).
        if std::env::var_os("AMIPRO_PERF_OUTLINE").is_some() {
            send(&mut app, Message::Command(CommandId::OutlineMode));
            assert!(app.outline.is_some(), "Outline mode on");
        }
        let mut harness = FrameHarness::new(window);
        harness.frame(&app);
        let text = &app.document.paragraphs()[paragraph].text;
        let byte = text
            .char_indices()
            .nth(text.chars().count() / 2)
            .map_or(text.len(), |(byte, _)| byte);
        move_to(&mut app, at(main, paragraph, byte), 1, false);
        harness.frame(&app);
        eprintln!(
            "{name} (paragraph {paragraph}, {} pages):",
            app.pages.page_count()
        );
        for (label, action) in [
            ("typing", PageAction::Insert("e".into())),
            ("backspace", PageAction::Backspace),
        ] {
            let (mut total, mut update, mut view, mut layout, mut draw) =
                (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
            for _ in 0..SAMPLES {
                let started = Instant::now();
                page(&mut app, action.clone());
                let updated = started.elapsed();
                let (viewed, laid_out, drawn) = harness.frame(&app);
                total.push(started.elapsed());
                update.push(updated);
                view.push(viewed);
                layout.push(laid_out);
                draw.push(drawn);
            }
            eprintln!(
                "  {label}: total {}; update {}; view {}; layout {}; draw {}",
                summary(&mut total),
                summary(&mut update),
                summary(&mut view),
                summary(&mut layout),
                summary(&mut draw)
            );
        }
        let mut undo = Vec::new();
        for _ in 0..SAMPLES {
            let started = Instant::now();
            send(&mut app, Message::Command(CommandId::Undo));
            harness.frame(&app);
            undo.push(started.elapsed());
        }
        eprintln!("  undo: total {}", summary(&mut undo));
    }
}

/// Writes screenshots of the window with the page surface, rendered by
/// tiny-skia, for review: `AMIPRO_SCREENSHOT_DIR` receives PPM files of the
/// test fixture and of `AMIPRO_PERF_SAM` when it is set.
#[test]
#[ignore = "writes screenshots of the page surface for review"]
fn page_surface_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut documents = vec![("fixture", page_fixture())];
    if let Some(path) = std::env::var_os("AMIPRO_PERF_SAM") {
        documents.push((
            "perf",
            amipro_formats::import_sam_path(path).unwrap().document,
        ));
    }
    for (name, document) in documents {
        for (suffix, zoom) in [("fit", Zoom::FitWidth), ("200", Zoom::Percent(200))] {
            let mut app = page_app(document.clone());
            app.zoom = zoom;
            // A selection on the first lines shows the highlight.
            let main = app.document.main_flow_id();
            move_to(&mut app, at(main, 0, 0), 1, false);
            move_to(&mut app, at(main, 0, 9), 1, true);
            let window = cosmic::iced::Size::new(1_280.0, 800.0);
            let mut renderer = cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
                cosmic::iced::Font::default(),
                cosmic::iced::Pixels(14.0),
            ));
            let mut root = cosmic::Application::view(&app);
            let mut tree = Tree::new(&root);
            let node = root.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(cosmic::iced::Size::ZERO, window),
            );
            let bounds = cosmic::iced::Rectangle::with_size(window);
            renderer.reset(bounds);
            root.as_widget().draw(
                &tree,
                &mut renderer,
                &cosmic::Theme::default(),
                &cosmic::iced::advanced::renderer::Style::default(),
                cosmic::iced::advanced::Layout::new(&node),
                cosmic::iced::mouse::Cursor::Unavailable,
                &bounds,
            );
            let cosmic::Renderer::Secondary(tiny) = &mut renderer else {
                unreachable!("built as tiny-skia")
            };
            let viewport = cosmic::iced::advanced::graphics::Viewport::with_physical_size(
                cosmic::iced::Size::new(1_280, 800),
                1.0,
            );
            let rgba = iced_tiny_skia::window::compositor::screenshot(
                tiny,
                &viewport,
                cosmic::iced::Color::from_rgb(0.5, 0.5, 0.5),
            );
            let mut ppm = b"P6\n1280 800\n255\n".to_vec();
            for pixel in rgba.as_chunks::<4>().0 {
                ppm.extend_from_slice(&pixel[..3]);
            }
            let path = directory.join(format!("page-surface-{name}-{suffix}.ppm"));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    }
}

fn fixed_rect(app: &AmiProApp, frame: FrameId) -> amipro_document::Rect {
    let amipro_document::Placement::Fixed { rect, .. } =
        app.document.frame(frame).unwrap().placement
    else {
        panic!("a Where placed frame")
    };
    rect
}

pub(crate) fn frame_action(app: &mut AmiProApp, action: crate::frame_editing::FrameAction) {
    send(app, Message::Frame(action));
}

#[test]
fn frame_gestures_are_one_undo_step_each_and_keys_within_a_second_join() {
    use crate::frame_editing::{FrameAction, FrameSelection};
    let mut app = page_app(page_fixture());
    let frame = text_frame(&app.document);
    let frame_flow = flow_of(&app.document, frame);
    let original = app.document.clone();
    let rect = fixed_rect(&app, frame);

    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    assert_eq!(app.frame_selection, Some(FrameSelection::one(frame, 1)));
    assert!(app.command_enabled(CommandId::BringToFront));
    assert!(app.command_enabled(CommandId::Delete));
    assert!(!app.command_enabled(CommandId::Cut), "no text selection");
    assert!(!app.command_enabled(CommandId::GroupFrames));

    // A drag's release: one MoveFrame, one undo step.
    let steps = app.history.undo_len();
    frame_action(
        &mut app,
        FrameAction::Move {
            dx: 100,
            dy: -50,
            key: false,
        },
    );
    assert_eq!(fixed_rect(&app, frame), rect.offset(100, -50).unwrap());
    assert_eq!(app.history.undo_len(), steps + 1);
    assert!(app.dirty);
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document, original);
    assert_eq!(
        app.frame_selection,
        Some(FrameSelection::one(frame, 1)),
        "the frame stays selected"
    );
    send(&mut app, Message::Command(CommandId::Redo));
    assert_eq!(fixed_rect(&app, frame), rect.offset(100, -50).unwrap());

    // Three arrow presses in a row: one step.
    let steps = app.history.undo_len();
    for _ in 0..3 {
        frame_action(
            &mut app,
            FrameAction::Move {
                dx: 20,
                dy: 0,
                key: true,
            },
        );
    }
    assert_eq!(fixed_rect(&app, frame), rect.offset(160, -50).unwrap());
    assert_eq!(app.history.undo_len(), steps + 1);
    // After more than a second, the next press is a new step.
    app.sessions[0].frame_keys = Some((
        Instant::now() - Duration::from_secs(2),
        app.history.undo_len(),
    ));
    frame_action(
        &mut app,
        FrameAction::Move {
            dx: 0,
            dy: 20,
            key: true,
        },
    );
    assert_eq!(app.history.undo_len(), steps + 2);
    send(&mut app, Message::Command(CommandId::Undo));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(fixed_rect(&app, frame), rect.offset(100, -50).unwrap());

    // Alt+Left shrinks the right edge, never below the margins plus 0.10 in.
    frame_action(&mut app, FrameAction::ResizeBy { dx: -20_000, dy: 0 });
    assert_eq!(fixed_rect(&app, frame).width(), 144 + 144 + 144);
    // A handle's drag: the rectangle as given.
    let from = fixed_rect(&app, frame);
    let to = amipro_document::Rect::new(from.left - 200, from.top, from.right, from.bottom + 300);
    frame_action(
        &mut app,
        FrameAction::Resize {
            from,
            to,
            with_picture: false,
        },
    );
    assert_eq!(fixed_rect(&app, frame), to);

    // Enter puts the caret in its text; Esc selects the frame again.
    frame_action(&mut app, FrameAction::EnterText { at: None });
    assert_eq!(app.frame_selection, None);
    assert_eq!(app.page_selection.focus, at(frame_flow, 0, 0));
    page(&mut app, PageAction::Escape);
    assert_eq!(app.frame_selection, Some(FrameSelection::one(frame, 1)));

    // Delete asks first; Cancel keeps the frame, Delete removes it.
    frame_action(&mut app, FrameAction::Delete);
    assert!(app.pending_frame_delete);
    assert!(cosmic::Application::dialog(&app).is_some());
    send(&mut app, Message::CancelFrameDelete);
    assert!(app.document.frame(frame).is_some());
    send(&mut app, Message::Command(CommandId::Delete));
    assert!(app.pending_frame_delete, "Edit › Delete asks too");
    send(&mut app, Message::ConfirmFrameDelete);
    assert!(app.document.frame(frame).is_none());
    assert_eq!(app.frame_selection, None);
    assert_eq!(app.page_selection.flow(), app.document.main_flow_id());
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(app.document.frame(frame).is_some());
    assert_editor_in_sync(&app);

    // A click in the main text leaves the frame selection.
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    assert_eq!(app.frame_selection, None);
}

#[test]
fn frames_group_restack_and_cycle_through_commands() {
    use crate::frame_editing::{FrameAction, FrameSelection};
    let mut document = page_fixture();
    let second = document.create_text_frame(
        amipro_document::Placement::Fixed {
            page: 1,
            rect: amipro_document::Rect::new(1_440, 10_080, 4_320, 11_520),
        },
        amipro_document::FrameStyle::default(),
    );
    document.frame_stack_mut().push(second);
    document.validate().unwrap();
    let mut app = page_app(document);
    let first = text_frame(&app.document);

    // Ctrl+G is Go To, as in Ami Pro; its Next frame selects the first
    // frame on the caret's page, and Select Next Frame the next.
    assert_eq!(
        command::command_for_shortcut(
            cosmic::iced::keyboard::Modifiers::CTRL,
            &cosmic::iced::keyboard::Key::Character("g".into()),
            None
        ),
        Some(CommandId::GoToPage)
    );
    send(&mut app, Message::Command(CommandId::GoToPage));
    assert!(app.go_to.is_some());
    send(&mut app, Message::GoTo(GoToAction::NextFrame));
    assert!(app.go_to.is_none(), "the dialog closes");
    assert_eq!(app.frame_selection, Some(FrameSelection::one(first, 1)));
    frame_action(&mut app, FrameAction::Cycle { backwards: false });
    assert_eq!(app.frame_selection, Some(FrameSelection::one(second, 1)));

    // Shift+click gathers both; Group makes them one group, in one step.
    frame_action(
        &mut app,
        FrameAction::Select {
            frame: first,
            page: 1,
            extend: true,
        },
    );
    assert!(app.command_enabled(CommandId::GroupFrames));
    send(&mut app, Message::Command(CommandId::GroupFrames));
    let group = app.document.frame(first).unwrap().group;
    assert!(group.is_some());
    assert_eq!(app.document.frame(second).unwrap().group, group);
    assert!(app.command_enabled(CommandId::UngroupFrames));
    assert!(!app.command_enabled(CommandId::GroupFrames));

    // The group moves as one.
    let (a, b) = (fixed_rect(&app, first), fixed_rect(&app, second));
    frame_action(
        &mut app,
        FrameAction::Move {
            dx: 40,
            dy: 0,
            key: false,
        },
    );
    assert_eq!(fixed_rect(&app, first), a.offset(40, 0).unwrap());
    assert_eq!(fixed_rect(&app, second), b.offset(40, 0).unwrap());

    send(&mut app, Message::Command(CommandId::UngroupFrames));
    assert_eq!(app.document.frame(first).unwrap().group, None);

    // Send to Back puts the second frame behind the first.
    frame_action(
        &mut app,
        FrameAction::Select {
            frame: second,
            page: 1,
            extend: false,
        },
    );
    send(&mut app, Message::Command(CommandId::SendToBack));
    let stack = app.document.frame_stack();
    let position = |frame| stack.iter().position(|id| *id == frame).unwrap();
    assert!(position(second) < position(first));
    send(&mut app, Message::Command(CommandId::BringToFront));
    let stack = app.document.frame_stack();
    let position = |frame| stack.iter().position(|id| *id == frame).unwrap();
    assert!(position(second) > position(first));
}

/// Release-mode baseline for a frame move's release on `AMIPRO_PERF_SAM`
/// (README31) or the fixture: the command, the relayout, and the next frame
/// the user sees (view, layout, draw).
#[test]
#[ignore = "manual release-mode frame move performance baseline"]
fn frame_move_release_performance_baseline() {
    use crate::frame_editing::FrameAction;
    const SAMPLES: usize = 40;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    let (page, frame) = app
        .pages
        .layout()
        .pages
        .iter()
        .find_map(|page| {
            page.frames
                .iter()
                .find(|frame_box| crate::frame_editing::selectable(&app.document, frame_box.frame))
                .map(|frame_box| (page.number, frame_box.frame))
        })
        .expect("a document with a page frame");
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page,
            extend: false,
        },
    );
    harness.frame(&app);
    let (mut total, mut update) = (Vec::new(), Vec::new());
    for step in 0..SAMPLES {
        let dx = if step % 2 == 0 { 144 } else { -144 };
        let started = Instant::now();
        frame_action(
            &mut app,
            FrameAction::Move {
                dx,
                dy: 0,
                key: false,
            },
        );
        update.push(started.elapsed());
        harness.frame(&app);
        total.push(started.elapsed());
    }
    eprintln!(
        "frame move release on page {page} ({} pages): total {}; update {}",
        app.pages.page_count(),
        summary(&mut total),
        summary(&mut update)
    );
}

/// Release-mode baseline for a page layout change (PL.11) on
/// `AMIPRO_PERF_SAM` or the fixture: a left margin typed in Page Layout
/// Properties, alternating between two values, the relayout of every page,
/// and the next frame the user sees (view, layout, draw); then undo.
#[test]
#[ignore = "manual release-mode page layout change performance baseline"]
fn page_layout_change_performance_baseline() {
    use crate::page_layout_properties::{Field, LayoutAction};
    use amipro_editing::layout_setup::MarginSide;
    const SAMPLES: usize = 20;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    send(&mut app, Message::Command(CommandId::PageLayout));
    harness.frame(&app);
    let left = Field::Margin(MarginSide::Left);
    let (mut total, mut update) = (Vec::new(), Vec::new());
    for step in 0..SAMPLES {
        let value = if step % 2 == 0 { "1.10" } else { "1.00" };
        send(
            &mut app,
            Message::PageLayoutProperties(LayoutAction::Text(left, value.into())),
        );
        let started = Instant::now();
        send(
            &mut app,
            Message::PageLayoutProperties(LayoutAction::Submit(left)),
        );
        update.push(started.elapsed());
        harness.frame(&app);
        total.push(started.elapsed());
    }
    let mut undo = Vec::new();
    for _ in 0..SAMPLES {
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undo.push(started.elapsed());
    }
    eprintln!(
        "page layout change ({} paragraphs, {} pages): total {}; update {}; undo {}",
        app.document.paragraphs().len(),
        app.pages.page_count(),
        summary(&mut total),
        summary(&mut update),
        summary(&mut undo)
    );
}

/// Release-mode baseline for the ruler (RU.10) on `AMIPRO_PERF_SAM` or the
/// fixture: a release of the first-line indent on the caret's paragraph,
/// and of the page's left margin, alternating between two places, each
/// timed from the message to the next frame the user sees (view, layout,
/// draw); then undo.
#[test]
#[ignore = "manual release-mode ruler performance baseline"]
fn ruler_release_performance_baseline() {
    use crate::ruler::Marker;
    use crate::ruler_view::RulerEvent;
    const SAMPLES: usize = 20;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    harness.frame(&app);
    let ruler = crate::ruler::Ruler::at(&app.document, app.pages.layout(), app.page_selection)
        .expect("a ruler at the caret");
    let (left, _) = ruler.column();
    let margin = Marker::Edge {
        column: 0,
        right: false,
    };
    let margin_at = ruler.position(margin);
    let mut timed = |marker: Marker, places: [f32; 2]| {
        let (mut total, mut update) = (Vec::new(), Vec::new());
        for step in 0..SAMPLES {
            let started = Instant::now();
            send(
                &mut app,
                Message::Ruler(RulerEvent::Release(marker, places[step % 2], false)),
            );
            update.push(started.elapsed());
            harness.frame(&app);
            total.push(started.elapsed());
        }
        (total, update)
    };
    let (mut indent, mut indent_update) = timed(Marker::First, [left + 720.0, left]);
    let (mut page, mut page_update) = timed(margin, [margin_at + 180.0, margin_at]);
    let mut undo = Vec::new();
    for _ in 0..SAMPLES {
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undo.push(started.elapsed());
    }
    eprintln!(
        "ruler release ({} paragraphs, {} pages): indent total {}, update {}; left margin total {}, update {}; undo {}",
        app.document.paragraphs().len(),
        app.pages.page_count(),
        summary(&mut indent),
        summary(&mut indent_update),
        summary(&mut page),
        summary(&mut page_update),
        summary(&mut undo)
    );
}

/// Lays out and draws the whole window, returning the root layout's size.
pub(crate) fn draw_window(app: &AmiProApp) -> cosmic::iced::Size {
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut renderer = cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
        cosmic::iced::Font::default(),
        cosmic::iced::Pixels(14.0),
    ));
    let mut root = cosmic::Application::view(app);
    let mut tree = Tree::new(&root);
    let node = root.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(cosmic::iced::Size::ZERO, window),
    );
    let viewport = cosmic::iced::Rectangle::with_size(window);
    renderer.reset(viewport);
    root.as_widget().draw(
        &tree,
        &mut renderer,
        &cosmic::Theme::default(),
        &cosmic::iced::advanced::renderer::Style::default(),
        cosmic::iced::advanced::Layout::new(&node),
        cosmic::iced::mouse::Cursor::Unavailable,
        &viewport,
    );
    node.size()
}

#[test]
fn the_inspector_takes_the_rail_and_applies_each_field_as_one_step() {
    use crate::frame_editing::{FrameAction, FrameSelection};
    use crate::frame_inspector::{Choice, Field, PlacementKind};
    let mut app = page_app(page_fixture());
    let frame = text_frame(&app.document);
    let rect = fixed_rect(&app, frame);
    assert!(!app.command_enabled(CommandId::FrameProperties));
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    assert!(app.frame_inspector_view().is_some());
    // Format › Frame Properties opens the rail, which shows the inspector.
    assert!(app.command_enabled(CommandId::FrameProperties));
    send(&mut app, Message::Command(CommandId::FrameProperties));
    assert!(app.styles_visible);
    assert_eq!(draw_window(&app), cosmic::iced::Size::new(1_280.0, 800.0));

    // A typed width applies on Enter (or when focus leaves), as one step.
    let steps = app.history.undo_len();
    send(
        &mut app,
        Message::InspectorInput(Field::Width, "3.5 in".into()),
    );
    assert_eq!(
        fixed_rect(&app, frame),
        rect,
        "typing alone changes nothing"
    );
    send(&mut app, Message::InspectorSubmit(Field::Width));
    assert_eq!(fixed_rect(&app, frame).width(), 5_040);
    assert_eq!(app.history.undo_len(), steps + 1);
    assert!(app.inspector_drafts.is_empty());
    // A second submit (focus leaving after Enter) does nothing more.
    send(&mut app, Message::InspectorSubmit(Field::Width));
    assert_eq!(app.history.undo_len(), steps + 1);

    // A refused value keeps the old one and says why.
    send(
        &mut app,
        Message::InspectorInput(Field::Corners, "250".into()),
    );
    send(&mut app, Message::InspectorSubmit(Field::Corners));
    assert_eq!(app.document.frame(frame).unwrap().style.corners, 0);
    assert!(
        app.inspector_error
            .as_ref()
            .is_some_and(|(field, reason)| *field == Field::Corners && reason.contains("0 to 100"))
    );
    assert_eq!(app.history.undo_len(), steps + 1);
    assert_eq!(draw_window(&app), cosmic::iced::Size::new(1_280.0, 800.0));

    // A choice applies at once; Undo restores it in one step.
    send(&mut app, Message::InspectorChoice(Choice::Opaque(false)));
    assert!(!app.document.frame(frame).unwrap().style.opaque);
    assert_eq!(app.inspector_error, None);
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(app.document.frame(frame).unwrap().style.opaque);

    // A new placement keeps the frame selected where it is now drawn.
    send(
        &mut app,
        Message::InspectorChoice(Choice::Placement(PlacementKind::WithParaAbove)),
    );
    assert!(matches!(
        app.document.frame(frame).unwrap().placement,
        amipro_document::Placement::BelowParagraph { .. }
    ));
    let selection = app.frame_selection.clone().unwrap();
    assert!(
        app.page_box(selection.page)
            .unwrap()
            .frames
            .iter()
            .any(|frame_box| frame_box.frame == frame)
    );
    assert_eq!(draw_window(&app), cosmic::iced::Size::new(1_280.0, 800.0));

    // Leaving the frame returns the rail to Styles; it stays open.
    frame_action(&mut app, FrameAction::Deselect);
    assert!(app.frame_inspector_view().is_none());
    assert!(app.styles_visible);

    // Drafts belong to the frame they were typed for.
    let mut document = page_fixture();
    let other = document.create_text_frame(
        amipro_document::Placement::Fixed {
            page: 1,
            rect: amipro_document::Rect::new(1_440, 10_080, 4_320, 11_520),
        },
        amipro_document::FrameStyle::default(),
    );
    document.frame_stack_mut().push(other);
    let mut app = page_app(document);
    let first = text_frame(&app.document);
    frame_action(
        &mut app,
        FrameAction::Select {
            frame: first,
            page: 1,
            extend: false,
        },
    );
    send(
        &mut app,
        Message::InspectorInput(Field::Name, "Draft".into()),
    );
    app.sessions[0].frame_selection = Some(FrameSelection::one(other, 1));
    send(&mut app, Message::InspectorSubmit(Field::Name));
    assert_eq!(app.document.frame(other).unwrap().style.name, None);
    assert_eq!(app.document.frame(first).unwrap().style.name, None);
}

/// The application view inside the shell libcosmic draws around it: the
/// header bar with the title and window controls, a 7-pixel gutter (8 when
/// maximized), and the theme's window background (`cosmic::app`'s
/// `view_main` with its default content container).
fn window_shell<'a>(app: &'a AmiProApp, maximized: bool) -> Element<'a, Message> {
    // The title the app gave libcosmic's header bar, as the shell draws it.
    let header = widget::header_bar()
        .title(app.core.window.header_title.clone())
        .on_close(Message::RefreshClipboard)
        .on_maximize(Message::RefreshClipboard)
        .on_minimize(Message::RefreshClipboard);
    let gutter = if maximized { 8 } else { 7 };
    // As `view_main` on a narrow window: the context drawer (About, the
    // Help drawers) over the content.
    let main = match cosmic::Application::context_drawer(app) {
        Some(context) => cosmic::widget::context_drawer(
            context.title,
            context.actions,
            context.header,
            context.footer,
            context.on_close,
            cosmic::Application::view(app),
            context.content,
            360.0,
        )
        .into(),
        None => cosmic::Application::view(app),
    };
    let content = widget::container(main)
        .padding([0, gutter, 0, gutter])
        .width(Length::Fill)
        .height(Length::Fill);
    let window = widget::container(widget::column::with_children(vec![
        header.into(),
        content.into(),
    ]))
    .class(cosmic::theme::Container::WindowBackground)
    .width(Length::Fill)
    .height(Length::Fill);
    // As `view_main`: an open dialog is a modal popover over the window.
    let mut popover = widget::popover(window).modal(true);
    if let Some(dialog) = cosmic::Application::dialog(app) {
        popover = popover.popup(dialog);
    }
    popover.into()
}

/// Renders the whole window with tiny-skia and returns its RGB pixels.
fn render_window(
    app: &AmiProApp,
    size: (u32, u32),
    theme: &cosmic::Theme,
    maximized: bool,
) -> Vec<u8> {
    render_window_after(app, size, theme, maximized, &[])
}

/// [`render_window`] after `events` reach the window's widgets, each with
/// the pointer at its point (a right-button release opens a right-click
/// menu); the messages they publish are dropped.
fn render_window_after(
    app: &AmiProApp,
    size: (u32, u32),
    theme: &cosmic::Theme,
    maximized: bool,
    events: &[(cosmic::iced::Event, cosmic::iced::Point)],
) -> Vec<u8> {
    let window = cosmic::iced::Size::new(size.0 as f32, size.1 as f32);
    let mut renderer = cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
        cosmic::iced::Font::default(),
        cosmic::iced::Pixels(14.0),
    ));
    let mut root = window_shell(app, maximized);
    let mut tree = Tree::new(&root);
    let node = root.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(cosmic::iced::Size::ZERO, window),
    );
    let bounds = cosmic::iced::Rectangle::with_size(window);
    for (event, point) in events {
        let mut messages = Vec::new();
        let mut shell = cosmic::iced::advanced::Shell::new(&mut messages);
        root.as_widget_mut().update(
            &mut tree,
            event,
            cosmic::iced::advanced::Layout::new(&node),
            cosmic::iced::mouse::Cursor::Available(*point),
            &renderer,
            &mut cosmic::iced::advanced::clipboard::Null,
            &mut shell,
            &bounds,
        );
    }
    // The runtime then sends an opened menu the next event, on which it
    // lays out its items.
    if let Some((_, point)) = events.last()
        && let Some(mut overlay) = root.as_widget_mut().overlay(
            &mut tree,
            cosmic::iced::advanced::Layout::new(&node),
            &renderer,
            &bounds,
            cosmic::iced::Vector::ZERO,
        )
    {
        let overlay_node = overlay.as_overlay_mut().layout(&renderer, window);
        let mut messages = Vec::new();
        let mut shell = cosmic::iced::advanced::Shell::new(&mut messages);
        overlay.as_overlay_mut().update(
            &cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::CursorMoved {
                position: *point,
            }),
            cosmic::iced::advanced::Layout::new(&overlay_node),
            cosmic::iced::mouse::Cursor::Available(*point),
            &renderer,
            &mut cosmic::iced::advanced::clipboard::Null,
            &mut shell,
        );
    }
    renderer.reset(bounds);
    let text_color: cosmic::iced::Color = theme.cosmic().on_bg_color().into();
    let style = cosmic::iced::advanced::renderer::Style {
        text_color,
        icon_color: text_color,
        ..cosmic::iced::advanced::renderer::Style::default()
    };
    root.as_widget().draw(
        &tree,
        &mut renderer,
        theme,
        &style,
        cosmic::iced::advanced::Layout::new(&node),
        cosmic::iced::mouse::Cursor::Unavailable,
        &bounds,
    );
    // Dialogs (an open popover) draw as an overlay above the window.
    if let Some(mut overlay) = root.as_widget_mut().overlay(
        &mut tree,
        cosmic::iced::advanced::Layout::new(&node),
        &renderer,
        &bounds,
        cosmic::iced::Vector::ZERO,
    ) {
        let overlay_node = overlay.as_overlay_mut().layout(&renderer, window);
        renderer.with_layer(bounds, |renderer| {
            overlay.as_overlay().draw(
                renderer,
                theme,
                &style,
                cosmic::iced::advanced::Layout::new(&overlay_node),
                cosmic::iced::mouse::Cursor::Unavailable,
            );
        });
    }
    let cosmic::Renderer::Secondary(tiny) = &mut renderer else {
        unreachable!("built as tiny-skia")
    };
    let viewport = cosmic::iced::advanced::graphics::Viewport::with_physical_size(
        cosmic::iced::Size::new(size.0, size.1),
        1.0,
    );
    let background: cosmic::iced::Color = theme.cosmic().bg_color().into();
    let rgba = iced_tiny_skia::window::compositor::screenshot(tiny, &viewport, background);
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|pixel| pixel[..3].to_vec())
        .collect()
}

/// Screenshots of a cell holding a picture with the caret in it (§24, T1),
/// light and dark, and, with `AMIPRO_PERF_SAM` set to README31.SAM, its
/// SmartIcon table on page 12. Writes PPM files to `AMIPRO_SCREENSHOT_DIR`.
/// A SAM's frame records as flags and sizes, its table cell records
/// (values only), and its body without paragraph caches: what a twin must
/// match. A frame's page and corner are caches Ami Pro recomputes from the
/// cell when it opens the file (TF1Y), so they are compared by opening the
/// twins in Ami Pro.
fn cell_twin_facts(bytes: &[u8]) -> (Vec<String>, Vec<String>, String) {
    let text =
        String::from_utf8_lossy(bytes.split(|byte| *byte == 0).next().unwrap_or(&[])).into_owned();
    let lines: Vec<&str> = text.split("\r\n").collect();
    let frames = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| **line == "[frm]")
        .map(|(index, _)| {
            let value = |offset: usize| lines[index + offset].trim().parse::<i64>().unwrap_or(0);
            format!(
                "flags {:#x} {} x {}",
                value(2),
                value(5) - value(3),
                value(6) - value(4)
            )
        })
        .collect();
    let cells = lines
        .iter()
        .filter(|line| line.starts_with("\t\t\t "))
        .map(|line| line.trim().to_owned())
        .collect();
    // The main text as read (Ami Pro writes an empty paragraph with a cache
    // as a line of its own and one without as a blank line).
    let body = amipro_formats::import_sam(bytes, None, "Twin")
        .map(|imported| {
            imported
                .document
                .paragraphs()
                .iter()
                .map(|paragraph| paragraph.text.clone())
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    (frames, cells, body)
}

/// CF.7: frames in cells edited in the app as in Ami Pro's fixtures, written
/// as Ami Pro wrote them (VM TF1–TF11; `CELL_FRAME_NOTES.md`).
/// `AMIPRO_TWIN_OUT` receives the twins (`ATF*.SAM`) for Ami Pro to open.
#[test]
#[ignore = "needs the VM fixtures (AMIPRO_VM_LAB)"]
fn cell_frame_twins() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let out = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from);
    let open = |name: &str| {
        page_app(
            amipro_formats::import_sam_path(lab.join(format!("{name}.SAM")))
                .unwrap()
                .document,
        )
    };
    let cell_flow = |app: &AmiProApp, cell: usize| {
        app.document
            .frames()
            .find_map(|frame| match &frame.content {
                FrameContent::Table(table) => {
                    let amipro_document::CellContent::Text(flow) = table.cells[cell].content;
                    Some(flow)
                }
                _ => None,
            })
            .unwrap()
    };
    let mut cases: Vec<(&str, AmiProApp)> = Vec::new();
    // TF2: two lines above the table.
    let mut tf2 = open("TF1");
    let main = tf2.document.main_flow_id();
    move_to(&mut tf2, at(main, 0, 0), 1, false);
    page(&mut tf2, PageAction::ParagraphBreak);
    page(&mut tf2, PageAction::ParagraphBreak);
    cases.push(("TF2", tf2));
    // TF3: three lines in the cell beside the picture.
    let mut tf3 = open("TF2");
    let flow = cell_flow(&tf3, 0);
    move_to(&mut tf3, at(flow, 0, 0), 1, false);
    for (index, line) in ["One", "Two", "Three"].into_iter().enumerate() {
        if index > 0 {
            page(&mut tf3, PageAction::ParagraphBreak);
        }
        page(&mut tf3, PageAction::Insert(line.into()));
    }
    cases.push(("TF3", tf3));
    // TF7: Delete in the picture's cell.
    let mut tf7 = open("TF3");
    let flow = cell_flow(&tf7, 1);
    move_to(&mut tf7, at(flow, 0, 0), 1, false);
    page(&mut tf7, PageAction::Delete);
    cases.push(("TF7", tf7));
    // TF10: a page break after the line above the table.
    let mut tf10 = open("TF11");
    let main = tf10.document.main_flow_id();
    let before = tf10
        .document
        .paragraphs()
        .iter()
        .position(|paragraph| paragraph.text.starts_with("Before"))
        .unwrap();
    let end = tf10.document.paragraphs()[before].text.len();
    move_to(&mut tf10, at(main, before, end), 1, false);
    send(&mut tf10, Message::Command(CommandId::PageBreak));
    cases.push(("TF10", tf10));

    let options = amipro_formats::SamOptions::at(0);
    // README31 with a page break before its SmartIcon table (as EDK1, Phase
    // 4.11): its pictures go to the next page with the table, in their
    // cells, here and in Ami Pro.
    let mut readme = open("README31");
    let main = readme.document.main_flow_id();
    let anchor = readme
        .document
        .paragraphs()
        .iter()
        .enumerate()
        .filter(|(_, paragraph)| {
            paragraph.objects.iter().any(|object| {
                object.anchored_frame().is_some_and(|frame| {
                    readme
                        .document
                        .frame(frame)
                        .is_some_and(|frame| !frame.owned_frames().is_empty())
                })
            })
        })
        .map(|(index, _)| index)
        .next()
        .unwrap();
    let end = readme.document.paragraphs()[anchor - 1].text.len();
    move_to(&mut readme, at(main, anchor - 1, end), 1, false);
    let pages_of = |app: &AmiProApp| {
        let layout = app.pages.layout();
        let page_of = |frame: FrameId| {
            layout
                .pages
                .iter()
                .find(|page| page.frames.iter().any(|frame_box| frame_box.frame == frame))
                .map(|page| page.number)
        };
        app.document
            .frames()
            .filter(|frame| !frame.owned_frames().is_empty())
            .flat_map(|table| {
                let table_page = page_of(table.id);
                table
                    .owned_frames()
                    .into_iter()
                    .map(move |inner| (table_page, inner))
            })
            .map(|(table_page, inner)| (table_page, page_of(inner)))
            .collect::<Vec<_>>()
    };
    let before = pages_of(&readme);
    send(&mut readme, Message::Command(CommandId::PageBreak));
    let after = pages_of(&readme);
    assert_eq!(after.len(), 3);
    for (table_page, picture_page) in &after {
        assert_eq!(table_page, picture_page, "a picture on its table's page");
    }
    assert_eq!(
        after[0].0.unwrap(),
        before[0].0.unwrap() + 1,
        "{before:?} {after:?}"
    );
    if let Some(out) = &out {
        std::fs::write(
            out.join("ARM31.SAM"),
            amipro_formats::sam_document(&readme.document, &options),
        )
        .unwrap();
    }

    let mut different = Vec::new();
    for (name, app) in &cases {
        let written = amipro_formats::sam_document(&app.document, &options);
        if let Some(out) = &out {
            std::fs::write(out.join(format!("A{name}.SAM")), &written).unwrap();
        }
        let theirs = std::fs::read(lab.join(format!("{name}.SAM"))).unwrap();
        let (ours, theirs) = (cell_twin_facts(&written), cell_twin_facts(&theirs));
        // Row heights and the table's and pictures' heights after typing in
        // a cell are Ami Pro's layout: it grows the row and fits the picture
        // to it when it opens the file (ATF3, saved again as ATF3S, is TF3).
        // A row record at the default height says what the default does
        // (TF10's row 1).
        let settled = |facts: &(Vec<String>, Vec<String>, String)| {
            let cells: Vec<String> = facts
                .1
                .iter()
                .filter(|record| record.split(' ').count() == 12)
                .cloned()
                .collect();
            let frames: Vec<String> = facts
                .0
                .iter()
                .map(|frame| frame.split(' ').take(2).collect::<Vec<_>>().join(" "))
                .collect();
            (frames, cells, facts.2.clone())
        };
        if ours == theirs {
            eprintln!("{name}: the same");
        } else if settled(&ours) == settled(&theirs) {
            eprintln!(
                "{name}: the same but for Ami Pro's row and frame heights\nours: {ours:#?}\nAmi Pro's: {theirs:#?}"
            );
        } else {
            eprintln!("{name}: differs\nours: {ours:#?}\nAmi Pro's: {theirs:#?}");
            different.push(*name);
        }
    }
    assert!(different.is_empty(), "different: {different:?}");
}

#[test]
#[ignore = "writes screenshots of pictures in table cells"]
fn cell_picture_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let write = |runtime: &mut Runtime, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let rgb = runtime.screenshot(&theme);
        let size = runtime.window;
        let mut ppm = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    let (document, _, _, flows) = crate::page_surface::cell_picture_fixture();
    let mut app = page_app(document);
    send(&mut app, Message::ZoomTo(150));
    let mut runtime = Runtime::new(app);
    runtime.send(Message::Page(PageAction::MoveTo {
        position: at(flows[1], 0, 0),
        page: 1,
        extend: false,
    }));
    runtime.redraw();
    write(&mut runtime, "window-cellpicture-light", false);
    write(&mut runtime, "window-cellpicture-dark", true);

    if let Some(path) = std::env::var_os("AMIPRO_PERF_SAM") {
        let document = amipro_formats::import_sam_path(path).unwrap().document;
        let holder = document.frames().find_map(|frame| match &frame.content {
            FrameContent::Table(table) => table.cells.iter().find_map(|cell| {
                cell.frame.map(|_| {
                    let amipro_document::CellContent::Text(flow) = cell.content;
                    flow
                })
            }),
            _ => None,
        });
        let mut app = page_app(document);
        let page = app.pages.layout().pages.iter().find(|page| {
            page.frames.iter().any(|frame_box| {
                app.document
                    .frame(frame_box.frame)
                    .is_some_and(|frame| frame.placement.is_in_cell())
            })
        });
        let number = page.map_or(1, |page| page.number);
        if let Some(flow) = holder {
            move_to(&mut app, at(flow, 0, 0), number, false);
        }
        let mut runtime = Runtime::new(app);
        runtime.send(Message::Command(CommandId::GoToPage));
        runtime.send(Message::GoTo(GoToAction::Input(number.to_string())));
        runtime.send(Message::GoTo(GoToAction::Go));
        runtime.redraw();
        write(&mut runtime, "window-cellpicture-readme31", false);
    }
}

/// Whole-window screenshots for the UI gap list (`docs/Design/UI_GAP_LIST.md`):
/// light and dark, 1280×800, maximized 1920×1080, and the 900×640 minimum,
/// with the Styles panel open, a heading caret, and a selected phrase; plus
/// a selected frame with its inspector. Writes PPM files to
/// `AMIPRO_SCREENSHOT_DIR`; `AMIPRO_PERF_SAM` supplies the document.
#[test]
#[ignore = "writes screenshots for the UI gap list"]
fn window_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    send(&mut app, Message::Command(CommandId::StylesPanel));
    let main = app.document.main_flow_id();
    // As `init` does.
    drop(app.update_title());
    move_to(&mut app, at(main, 1, 5), 1, false);
    move_to(&mut app, at(main, 1, 30), 1, true);
    let write = |app: &AmiProApp, name: &str, size: (u32, u32), dark: bool, maximized| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let rgb = render_window(app, size, &theme, maximized);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    write(&app, "window-1280-light", (1_280, 800), false, false);
    write(&app, "window-1280-dark", (1_280, 800), true, false);
    write(&app, "window-1920-light", (1_920, 1_080), false, true);
    // Narrow: the rail starts closed, then opens over the workspace.
    let mut narrow = page_app(app.document.clone());
    drop(narrow.update_title());
    send(&mut narrow, Message::WindowResized(900.0));
    write(&narrow, "window-900-light", (900, 640), false, false);
    send(&mut narrow, Message::Command(CommandId::StylesPanel));
    write(&narrow, "window-900-overlay", (900, 640), false, false);
    // A selected frame puts Frame Properties in the rail.
    let mut framed = page_app(page_fixture());
    drop(framed.update_title());
    send(&mut framed, Message::Command(CommandId::StylesPanel));
    let frame = text_frame(&framed.document);
    frame_action(
        &mut framed,
        crate::frame_editing::FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    write(&framed, "window-frame-light", (1_280, 800), false, false);
    // The right-click menus (§16), opened by a right-button release.
    let right = |app: &AmiProApp, name: &str, at: cosmic::iced::Point, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let release = cosmic::iced::Event::Mouse(cosmic::iced::mouse::Event::ButtonReleased(
            cosmic::iced::mouse::Button::Right,
        ));
        let rgb = render_window_after(app, (1_280, 800), &theme, false, &[(release, at)]);
        let mut ppm = b"P6\n1280 800\n255\n".to_vec();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    let mut text = page_app(page_fixture());
    drop(text.update_title());
    move_to(&mut text, at(main, 2, 5), 1, false);
    move_to(&mut text, at(main, 2, 30), 1, true);
    right(
        &text,
        "window-context-text-light",
        cosmic::iced::Point::new(640.0, 420.0),
        false,
    );
    right(
        &text,
        "window-context-text-dark",
        cosmic::iced::Point::new(640.0, 420.0),
        true,
    );
    right(
        &framed,
        "window-context-frame-light",
        cosmic::iced::Point::new(640.0, 420.0),
        false,
    );
    // Tools › Customize Toolbar… with Save As chosen and added after Save.
    send(&mut app, Message::Command(CommandId::CustomizeToolbar));
    let save_as = ToolbarItem::Command(CommandId::SaveAs);
    for edit in [
        ToolbarEdit::SelectChoice(save_as),
        ToolbarEdit::SelectItem(2),
        ToolbarEdit::Add,
    ] {
        send(&mut app, Message::Toolbar(edit));
    }
    write(&app, "window-customize-light", (1_280, 800), false, false);
    write(&app, "window-customize-dark", (1_280, 800), true, false);
    // Help › Keyboard Shortcuts and Compatibility and Fidelity (CM12).
    let mut help = page_app(app.document.clone());
    drop(help.update_title());
    send(&mut help, Message::Command(CommandId::KeyboardShortcuts));
    write(&help, "window-shortcuts-light", (1_280, 800), false, false);
    write(&help, "window-shortcuts-dark", (1_280, 800), true, false);
    send(&mut help, Message::Command(CommandId::Compatibility));
    write(
        &help,
        "window-compatibility-light",
        (1_280, 800),
        false,
        false,
    );
    write(
        &help,
        "window-compatibility-dark",
        (1_280, 800),
        true,
        false,
    );
    // Tools › Preferences… (CM6), its General and View pages.
    send(&mut help, Message::Command(CommandId::Preferences));
    write(
        &help,
        "window-preferences-light",
        (1_280, 800),
        false,
        false,
    );
    send(
        &mut help,
        Message::Preferences(crate::preferences_dialog::PreferencesAction::Page(
            crate::preferences_dialog::Page::View,
        )),
    );
    write(
        &help,
        "window-preferences-view-dark",
        (1_280, 800),
        true,
        false,
    );
    // File › New… (CM1) over three synthetic sheets.
    let sheets = directory.join("new-sheets");
    std::fs::create_dir_all(&sheets).unwrap();
    for (name, text, description) in [
        (
            "MEMO.STY",
            "MEMORANDUM\nTo:\nFrom:\nSubject:\n\nThe memo's text goes here, over a few lines of the page.",
            "Memo with lines",
        ),
        ("LETTER.STY", "Dear\n\nYours sincerely,", "Letter"),
        ("PLAIN.STY", "", "Plain pages"),
    ] {
        let bytes = amipro_formats::style_sheet(
            &Document::from_plain_text(text),
            &amipro_formats::SheetOptions {
                description: description.into(),
                saved_at: 0,
                contents: true,
            },
        );
        std::fs::write(sheets.join(name), bytes).unwrap();
    }
    help.preferences.style_folders = vec![sheets.clone()];
    send(
        &mut help,
        Message::Preferences(crate::preferences_dialog::PreferencesAction::Close),
    );
    send(&mut help, Message::Command(CommandId::NewFromStyleSheet));
    write(&help, "window-new-light", (1_280, 800), false, false);
    write(&help, "window-new-dark", (1_280, 800), true, false);
    // Format › Paragraph… (CM3) with its Spacing section open.
    let mut paragraph = page_app(app.document.clone());
    drop(paragraph.update_title());
    move_to(&mut paragraph, at(main, 1, 5), 1, false);
    send(
        &mut paragraph,
        Message::Command(CommandId::ParagraphProperties),
    );
    send(
        &mut paragraph,
        Message::ParagraphProperties(crate::paragraph_properties::ParagraphAction::Section(
            crate::paragraph_properties::Section::Spacing,
        )),
    );
    write(
        &paragraph,
        "window-paragraph-light",
        (1_280, 800),
        false,
        false,
    );
    write(
        &paragraph,
        "window-paragraph-dark",
        (1_280, 800),
        true,
        false,
    );
    // Insert › Special Character… (CM12) with two recent characters.
    let mut characters = page_app(app.document.clone());
    drop(characters.update_title());
    characters.preferences.recent_characters = vec!['©', 'é'];
    send(
        &mut characters,
        Message::Command(CommandId::SpecialCharacter),
    );
    write(
        &characters,
        "window-special-light",
        (1_280, 800),
        false,
        false,
    );
    write(
        &characters,
        "window-special-dark",
        (1_280, 800),
        true,
        false,
    );
    // View › Facing Pages at Full Page (CM8).
    let mut facing = page_app(app.document.clone());
    drop(facing.update_title());
    send(&mut facing, Message::Command(CommandId::TwoPages));
    send(&mut facing, Message::Command(CommandId::FullPage));
    write(&facing, "window-facing-light", (1_280, 800), false, false);
    write(&facing, "window-facing-dark", (1_280, 800), true, false);
    send(&mut facing, Message::Command(CommandId::FullPage));
    send(&mut facing, Message::Command(CommandId::CustomZoom));
    write(
        &facing,
        "window-facing-custom-light",
        (1_280, 800),
        false,
        false,
    );
    // View › Draft Mode and Show Clean Screen (CM9).
    let mut draft = page_app(app.document.clone());
    drop(draft.update_title());
    send(&mut draft, Message::Command(CommandId::DraftMode));
    write(&draft, "window-draft-light", (1_280, 800), false, false);
    send(&mut draft, Message::Command(CommandId::DraftMode));
    send(&mut draft, Message::Command(CommandId::FullScreen));
    write(&draft, "window-clean-dark", (1_280, 800), true, false);
    // View › Show ▸ Tabs & Returns and Column Guides (CM10).
    let mut marks = page_app(app.document.clone());
    drop(marks.update_title());
    send(&mut marks, Message::Command(CommandId::ShowTabsReturns));
    send(&mut marks, Message::Command(CommandId::ColumnGuides));
    write(&marks, "window-marks-light", (1_280, 800), false, false);
    write(&marks, "window-marks-dark", (1_280, 800), true, false);
    // Format › Special Effects… and Format › Font… over the selection.
    let mut dialogs = page_app(app.document.clone());
    drop(dialogs.update_title());
    move_to(&mut dialogs, at(main, 1, 5), 1, false);
    move_to(&mut dialogs, at(main, 1, 30), 1, true);
    send(&mut dialogs, Message::Command(CommandId::SpecialEffects));
    send(
        &mut dialogs,
        Message::SpecialEffects(crate::character_dialogs::EffectsAction::Toggle(0, true)),
    );
    write(&dialogs, "window-effects-light", (1_280, 800), false, false);
    send(
        &mut dialogs,
        Message::SpecialEffects(crate::character_dialogs::EffectsAction::Cancel),
    );
    send(&mut dialogs, Message::Command(CommandId::Font));
    send(
        &mut dialogs,
        Message::FontDialog(crate::character_dialogs::FontAction::Colour(
            crate::character_dialogs::PALETTE[0].0,
        )),
    );
    write(&dialogs, "window-font-light", (1_280, 800), false, false);
    write(&dialogs, "window-font-dark", (1_280, 800), true, false);
    // Format › Styles › Save as a Style Sheet… (`UI_SPEC.md` §14).
    let mut sheet = page_app(app.document.clone());
    drop(sheet.update_title());
    send(&mut sheet, Message::Command(CommandId::SaveAsStyleSheet));
    send(
        &mut sheet,
        Message::SaveSheet(crate::SaveSheetAction::Description(
            "Quarterly reports".into(),
        )),
    );
    write(&sheet, "window-savesheet-light", (1_280, 800), false, false);
    write(&sheet, "window-savesheet-dark", (1_280, 800), true, false);
    send(
        &mut sheet,
        Message::SaveSheet(crate::SaveSheetAction::Cancel),
    );
    // File › Save As… as an Ami Pro document, with what it cannot store
    // listed first (decision B; `UI_SPEC.md` §17).
    let mut lossy = sheet.document.clone();
    lossy
        .main_flow_mut()
        .paragraph_mut(0)
        .unwrap()
        .text
        .push_str(" \u{2192} \u{03a9}");
    let losses = amipro_formats::sam_losses(&lossy, &amipro_formats::SamOptions::at(0))
        .unwrap()
        .iter()
        .map(amipro_formats::Loss::message)
        .collect();
    sheet.sam_save = Some(crate::PendingSamSave {
        path: "/home/user/Documents/REPORT.SAM".into(),
        losses,
        saved_at: 0,
        replace: false,
    });
    write(&sheet, "window-savesam-light", (1_280, 800), false, false);
    write(&sheet, "window-savesam-dark", (1_280, 800), true, false);
    send(&mut sheet, Message::CancelSamSave);
    // A refused save is shown in a dialog, not only in the status bar
    // (user, 2026-10-13).
    sheet.save_problem = Some(crate::SaveProblem::save_as(
        &crate::file_workflow::SaveSafetyError::StyleSheetTarget,
    ));
    write(
        &sheet,
        "window-saveproblem-light",
        (1_280, 800),
        false,
        false,
    );
    write(&sheet, "window-saveproblem-dark", (1_280, 800), true, false);
    send(&mut sheet, Message::SaveProblemClose);
    // Create Style, Style Management, and Use Another Style Sheet (§14).
    send(&mut sheet, Message::Command(CommandId::CreateStyle));
    send(
        &mut sheet,
        Message::CreateStyle(crate::style_dialogs::CreateAction::Name("Heading 1".into())),
    );
    write(
        &sheet,
        "window-createstyle-light",
        (1_280, 800),
        false,
        false,
    );
    write(&sheet, "window-createstyle-dark", (1_280, 800), true, false);
    send(
        &mut sheet,
        Message::CreateStyle(crate::style_dialogs::CreateAction::Cancel),
    );
    let mut styles = sheet.document.styles().clone();
    let ids: Vec<_> = styles.keys().copied().collect();
    for (index, id) in ids.iter().enumerate() {
        styles.get_mut(id).unwrap().origin = if index == 0 {
            amipro_document::StyleOrigin::Document
        } else {
            amipro_document::StyleOrigin::Sheet
        };
    }
    let mut ghost = styles[&ids[0]].clone();
    ghost.id = amipro_document::StyleId(ids.last().unwrap().0 + 1);
    ghost.name = "Ghost".into();
    ghost.origin = amipro_document::StyleOrigin::Missing;
    styles.insert(ghost.id, ghost);
    *sheet.document.styles_mut() = styles;
    sheet
        .document
        .set_style_sheet(Some(amipro_document::StyleSheetLink {
            name: "_DEFAULT.STY".into(),
            layout_from_sheet: true,
        }));
    send(&mut sheet, Message::Command(CommandId::StyleManagement));
    send(
        &mut sheet,
        Message::StyleManagement(crate::style_dialogs::ManagementAction::Select(ids[1])),
    );
    write(
        &sheet,
        "window-management-light",
        (1_280, 800),
        false,
        false,
    );
    write(&sheet, "window-management-dark", (1_280, 800), true, false);
    send(
        &mut sheet,
        Message::StyleManagement(crate::style_dialogs::ManagementAction::Close),
    );
    if let Some(folder) = std::env::var_os("AMIPRO_SCREENSHOT_STYLES") {
        sheet.preferences.style_folders = vec![std::path::PathBuf::from(folder)];
    }
    send(
        &mut sheet,
        Message::Command(CommandId::UseAnotherStyleSheet),
    );
    send(
        &mut sheet,
        Message::UseStyleSheet(crate::style_dialogs::UseSheetAction::Select(0)),
    );
    write(&sheet, "window-usesheet-light", (1_280, 800), false, false);
    write(&sheet, "window-usesheet-dark", (1_280, 800), true, false);
    // Style Properties in the rail, each section (§15).
    let mut modify = page_app(app.document.clone());
    drop(modify.update_title());
    move_to(&mut modify, at(main, 1, 2), 1, false);
    send(&mut modify, Message::Command(CommandId::ModifyCurrentStyle));
    write(
        &modify,
        "window-properties-font-light",
        (1_280, 800),
        false,
        false,
    );
    write(
        &modify,
        "window-properties-font-dark",
        (1_280, 800),
        true,
        false,
    );
    use crate::style_properties::{Section, StyleAction};
    send(
        &mut modify,
        Message::StyleProperties(StyleAction::Section(Section::Font)),
    );
    for (section, name) in [
        (Section::Alignment, "alignment"),
        (Section::Tabs, "tabs"),
        (Section::Spacing, "spacing"),
        (Section::Breaks, "breaks"),
        (Section::Bullets, "bullets"),
        (Section::Lines, "lines"),
    ] {
        send(
            &mut modify,
            Message::StyleProperties(StyleAction::Section(section)),
        );
        match section {
            Section::Tabs => send(&mut modify, Message::StyleProperties(StyleAction::AddTab)),
            Section::Bullets => send(
                &mut modify,
                Message::StyleProperties(StyleAction::Number(true)),
            ),
            Section::Lines => send(
                &mut modify,
                Message::StyleProperties(StyleAction::Rule(false, true)),
            ),
            _ => {}
        }
        write(
            &modify,
            &format!("window-properties-{name}-light"),
            (1_280, 800),
            false,
            false,
        );
        send(
            &mut modify,
            Message::StyleProperties(StyleAction::Section(section)),
        );
    }
}

#[test]
fn the_status_bar_page_arrows_and_go_to_move_by_pages() {
    // Ami Pro's page arrows and Page Status button (user's guide chapter 1)
    // and Go To (chapter 20); user, 2026-10-12.
    let mut app = page_app(page_fixture());
    let count = app.page_count();
    assert!(count >= 3, "{count}");
    assert_eq!(app.caret_page_number(), 1);
    assert!(!app.command_enabled(CommandId::PreviousPage));
    assert!(app.command_enabled(CommandId::NextPage));

    send(&mut app, Message::Command(CommandId::NextPage));
    assert_eq!(app.caret_page_number(), 2);
    let (top, _) = page_surface::page_top(&app.document, app.pages.layout(), 2, true).unwrap();
    assert_eq!(app.page_selection.focus, top, "the top of page 2's text");
    assert!(app.page_selection.is_caret());
    assert!(app.command_enabled(CommandId::PreviousPage));
    send(&mut app, Message::Command(CommandId::PreviousPage));
    assert_eq!(app.caret_page_number(), 1);

    // Go To starts at the caret's page; a page out of range is refused in
    // place.
    send(&mut app, Message::Command(CommandId::GoToPage));
    assert_eq!(app.go_to.as_ref().unwrap().page, "1");
    assert!(cosmic::Application::dialog(&app).is_some());
    send(&mut app, Message::GoTo(GoToAction::Input("99".into())));
    send(&mut app, Message::GoTo(GoToAction::Go));
    assert!(app.go_to.as_ref().unwrap().error.is_some());
    assert_eq!(app.caret_page_number(), 1);
    send(&mut app, Message::GoTo(GoToAction::Input("3".into())));
    send(&mut app, Message::GoTo(GoToAction::Go));
    assert!(app.go_to.is_none());
    assert_eq!(app.caret_page_number(), 3);

    // First and last page; Cancel changes nothing.
    send(&mut app, Message::Command(CommandId::GoToPage));
    send(&mut app, Message::GoTo(GoToAction::Last));
    let last = app.caret_page_number();
    assert_eq!(last, count);
    assert!(!app.command_enabled(CommandId::NextPage));
    send(&mut app, Message::Command(CommandId::GoToPage));
    send(&mut app, Message::GoTo(GoToAction::First));
    assert_eq!(app.caret_page_number(), 1);
    send(&mut app, Message::Command(CommandId::GoToPage));
    send(&mut app, Message::GoTo(GoToAction::Cancel));
    assert!(app.go_to.is_none());
    assert_eq!(app.caret_page_number(), 1);

    // Moving is not editing.
    assert!(!app.history.can_undo());
    assert!(!app.dirty);
    // The status bar with the arrows renders.
    draw_window(&app);
}

/// Each scrollable's id and vertical scroll.
struct ScrollProbe(Vec<(Option<cosmic::iced::widget::Id>, f32)>);

impl<T> cosmic::iced::advanced::widget::Operation<T> for ScrollProbe {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn cosmic::iced::advanced::widget::Operation<T>),
    ) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&cosmic::iced::widget::Id>,
        _bounds: cosmic::iced::Rectangle,
        _content: cosmic::iced::Rectangle,
        translation: cosmic::iced::Vector,
        _state: &mut dyn cosmic::iced::advanced::widget::operation::Scrollable,
    ) {
        self.0.push((id.cloned(), translation.y));
    }
}

/// The whole window (`view_main`: header bar, content, popovers) driven in
/// the pinned runtime's order (iced/winit `run_instance` and `update`): the
/// update's immediate actions are collected, the interface is rebuilt from
/// the previous one's cache, the widget operations then run on it, and the
/// app's own messages go round again.
struct Runtime {
    app: AmiProApp,
    cache: Option<cosmic::iced::runtime::user_interface::Cache>,
    renderer: cosmic::Renderer,
    /// The system clipboard as the runtime's clipboard actions leave it.
    clipboard: Option<String>,
    /// What the clipboard offers besides text: its bytes and type.
    clipboard_data: Option<(Vec<u8>, String)>,
    /// Further types the clipboard offers (Paste Special lists them all).
    clipboard_more: Vec<(Vec<u8>, String)>,
    window: cosmic::iced::Size,
}

impl Runtime {
    const WINDOW: cosmic::iced::Size = cosmic::iced::Size::new(1_280.0, 800.0);

    fn new(app: AmiProApp) -> Self {
        let mut runtime = Self {
            app,
            cache: Some(cosmic::iced::runtime::user_interface::Cache::default()),
            renderer: cosmic::Renderer::Secondary(iced_tiny_skia::Renderer::new(
                cosmic::iced::Font::default(),
                cosmic::iced::Pixels(14.0),
            )),
            clipboard: None,
            clipboard_data: None,
            clipboard_more: Vec::new(),
            window: Self::WINDOW,
        };
        runtime.operate(&mut []);
        runtime
    }

    /// Builds the interface from the cache and runs `operations` on it.
    fn operate(&mut self, operations: &mut [Box<dyn cosmic::iced::advanced::widget::Operation>]) {
        let mut ui = cosmic::iced::runtime::user_interface::UserInterface::build(
            cosmic::ApplicationExt::view_main(&self.app),
            self.window,
            self.cache.take().expect("the interface's cache"),
            &mut self.renderer,
        );
        for operation in operations {
            ui.operate(&self.renderer, operation.as_mut());
        }
        self.cache = Some(ui.into_cache());
    }

    /// Delivers `message`; returns the kinds of action it produced.
    fn send(&mut self, message: Message) -> Vec<&'static str> {
        let mut kinds = Vec::new();
        let mut messages = vec![message];
        while !messages.is_empty() {
            let mut operations = Vec::new();
            for message in std::mem::take(&mut messages) {
                let task = cosmic::Application::update(&mut self.app, message);
                let Some(mut stream) = cosmic::iced::runtime::task::into_stream(task) else {
                    continue;
                };
                // One action at a time: a clipboard read waits for its
                // answer.
                while let Some(action) = cosmic::iced::futures::executor::block_on(
                    cosmic::iced::futures::StreamExt::next(&mut stream),
                ) {
                    use cosmic::iced::runtime::clipboard::Action as Clipboard;
                    match action {
                        cosmic::iced::runtime::Action::Widget(operation) => {
                            kinds.push("scroll");
                            operations.push(operation);
                        }
                        cosmic::iced::runtime::Action::Output(cosmic::Action::App(message)) => {
                            kinds.push(match message {
                                Message::Repaint => "repaint",
                                _ => "message",
                            });
                            messages.push(message);
                        }
                        cosmic::iced::runtime::Action::Clipboard(Clipboard::Write {
                            contents,
                            ..
                        }) => {
                            kinds.push("clipboard");
                            self.clipboard = Some(contents);
                        }
                        cosmic::iced::runtime::Action::Clipboard(Clipboard::Read {
                            channel,
                            ..
                        }) => {
                            kinds.push("clipboard");
                            drop(channel.send(self.clipboard.clone()));
                        }
                        cosmic::iced::runtime::Action::Clipboard(Clipboard::ReadData(
                            types,
                            channel,
                            _,
                        )) => {
                            kinds.push("clipboard");
                            drop(
                                channel.send(
                                    self.clipboard_data
                                        .iter()
                                        .chain(self.clipboard_more.iter())
                                        .find(|(_, mime)| types.contains(mime))
                                        .cloned(),
                                ),
                            );
                        }
                        _ => {}
                    }
                }
            }
            self.operate(&mut operations);
        }
        kinds
    }

    /// Delivers `events` to the window one at a time, each with the pointer
    /// at its point, as the runtime does: the messages they publish go
    /// through [`Self::send`] before the next event. Returns those
    /// messages.
    fn events(&mut self, events: &[(cosmic::iced::Event, cosmic::iced::Point)]) -> Vec<Message> {
        self.deliver(events, true)
    }

    /// Delivers `events` as [`Self::events`] does, running their messages
    /// only when `run` is set: Print… and Export PDF… run tasks that open
    /// the desktop's own dialogs through its portals.
    fn deliver(
        &mut self,
        events: &[(cosmic::iced::Event, cosmic::iced::Point)],
        run: bool,
    ) -> Vec<Message> {
        let mut published = Vec::new();
        for (event, point) in events {
            let mut ui = cosmic::iced::runtime::user_interface::UserInterface::build(
                cosmic::ApplicationExt::view_main(&self.app),
                self.window,
                self.cache.take().expect("the interface's cache"),
                &mut self.renderer,
            );
            let mut messages = Vec::new();
            ui.update(
                std::slice::from_ref(event),
                cosmic::iced::mouse::Cursor::Available(*point),
                &mut self.renderer,
                &mut cosmic::iced::advanced::clipboard::Null,
                &mut messages,
            );
            self.cache = Some(ui.into_cache());
            for message in messages {
                if let cosmic::Action::App(message) = message {
                    published.push(message.clone());
                    if run {
                        self.send(message);
                    }
                }
            }
        }
        published
    }

    /// A click of `button` at `point`: the pointer moves there, then the
    /// button goes down and up. Returns the messages it published.
    fn click(
        &mut self,
        button: cosmic::iced::mouse::Button,
        point: cosmic::iced::Point,
    ) -> Vec<Message> {
        self.click_running(button, point, true)
    }

    /// A click whose messages are only returned, not run (see
    /// [`Self::deliver`]).
    fn click_unrun(
        &mut self,
        button: cosmic::iced::mouse::Button,
        point: cosmic::iced::Point,
    ) -> Vec<Message> {
        self.click_running(button, point, false)
    }

    fn click_running(
        &mut self,
        button: cosmic::iced::mouse::Button,
        point: cosmic::iced::Point,
        run: bool,
    ) -> Vec<Message> {
        use cosmic::iced::mouse::Event as Mouse;
        self.deliver(
            &[
                (
                    cosmic::iced::Event::Mouse(Mouse::CursorMoved { position: point }),
                    point,
                ),
                (
                    cosmic::iced::Event::Mouse(Mouse::ButtonPressed(button)),
                    point,
                ),
                (
                    cosmic::iced::Event::Mouse(Mouse::ButtonReleased(button)),
                    point,
                ),
                // The redraw that follows, with the pointer still there: an
                // opened menu lays out its items on it.
                (
                    cosmic::iced::Event::Window(cosmic::iced::window::Event::RedrawRequested(
                        Instant::now(),
                    )),
                    point,
                ),
            ],
            run,
        )
    }

    /// A double-click at `point`: both clicks delivered in one update, as
    /// one frame, so that a busy test machine never parts them.
    fn double_click(&mut self, point: cosmic::iced::Point) -> Vec<Message> {
        use cosmic::iced::mouse::{Button, Event as Mouse};
        self.events(&[(
            cosmic::iced::Event::Mouse(Mouse::CursorMoved { position: point }),
            point,
        )]);
        let press = cosmic::iced::Event::Mouse(Mouse::ButtonPressed(Button::Left));
        let release = cosmic::iced::Event::Mouse(Mouse::ButtonReleased(Button::Left));
        let mut ui = cosmic::iced::runtime::user_interface::UserInterface::build(
            cosmic::ApplicationExt::view_main(&self.app),
            self.window,
            self.cache.take().expect("the interface's cache"),
            &mut self.renderer,
        );
        let mut messages = Vec::new();
        ui.update(
            &[press.clone(), release.clone(), press, release],
            cosmic::iced::mouse::Cursor::Available(point),
            &mut self.renderer,
            &mut cosmic::iced::advanced::clipboard::Null,
            &mut messages,
        );
        self.cache = Some(ui.into_cache());
        let mut published = Vec::new();
        for message in messages {
            if let cosmic::Action::App(message) = message {
                published.push(message.clone());
                self.send(message);
            }
        }
        published
    }

    /// A redraw, as the runtime sends after a repaint, with the pointer at
    /// the window's origin.
    fn redraw(&mut self) -> Vec<Message> {
        self.events(&[(
            cosmic::iced::Event::Window(cosmic::iced::window::Event::RedrawRequested(
                Instant::now(),
            )),
            cosmic::iced::Point::ORIGIN,
        )])
    }

    /// The page workspace's vertical scroll as the screen shows it.
    fn workspace_scroll(&mut self) -> f32 {
        let mut ui = cosmic::iced::runtime::user_interface::UserInterface::build(
            cosmic::ApplicationExt::view_main(&self.app),
            self.window,
            self.cache.take().expect("the interface's cache"),
            &mut self.renderer,
        );
        let mut found = ScrollProbe(Vec::new());
        ui.operate(&self.renderer, &mut found);
        self.cache = Some(ui.into_cache());
        let workspace = Some(workspace_scroll_id());
        let scrolls: Vec<_> = found.0.iter().filter(|(id, _)| *id == workspace).collect();
        assert_eq!(scrolls.len(), 1, "one page workspace in {:?}", found.0);
        scrolls[0].1
    }

    /// The window as the runtime shows it now (its scroll included), drawn
    /// by tiny-skia: RGB bytes.
    fn screenshot(&mut self, theme: &cosmic::Theme) -> Vec<u8> {
        let mut ui = cosmic::iced::runtime::user_interface::UserInterface::build(
            cosmic::ApplicationExt::view_main(&self.app),
            self.window,
            self.cache.take().expect("the interface's cache"),
            &mut self.renderer,
        );
        let bounds = cosmic::iced::Rectangle::with_size(self.window);
        self.renderer.reset(bounds);
        let text_color: cosmic::iced::Color = theme.cosmic().on_bg_color().into();
        let style = cosmic::iced::advanced::renderer::Style {
            text_color,
            icon_color: text_color,
            ..cosmic::iced::advanced::renderer::Style::default()
        };
        ui.draw(
            &mut self.renderer,
            theme,
            &style,
            cosmic::iced::mouse::Cursor::Unavailable,
        );
        self.cache = Some(ui.into_cache());
        let cosmic::Renderer::Secondary(tiny) = &mut self.renderer else {
            unreachable!("built as tiny-skia")
        };
        let viewport = cosmic::iced::advanced::graphics::Viewport::with_physical_size(
            cosmic::iced::Size::new(self.window.width as u32, self.window.height as u32),
            1.0,
        );
        let background: cosmic::iced::Color = theme.cosmic().bg_color().into();
        let rgba = iced_tiny_skia::window::compositor::screenshot(tiny, &viewport, background);
        rgba.as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| pixel[..3].to_vec())
            .collect()
    }

    /// Where the page arrows put page `page` (from 1) of the active tab.
    fn page_scroll(&self, page: usize) -> f32 {
        (self.app.page_view().pages[page - 1].y - 24.0).max(0.0)
    }
}

#[test]
fn the_page_arrows_scroll_the_view_and_ask_for_a_repaint() {
    // User, 2026-10-12: the arrows moved the caret (the style field
    // changed) but not the view. The pinned runtime does not redraw after a
    // widget operation, so the scroll also carries a message.
    let mut runtime = Runtime::new(page_app(page_fixture()));
    assert_eq!(
        runtime.send(Message::Command(CommandId::NextPage)),
        ["scroll", "repaint"]
    );
    let target = runtime.page_scroll(2);
    let scrolled = runtime.workspace_scroll();
    assert!(
        (scrolled - target).abs() < 0.5,
        "scrolled to {scrolled} for page 2 at {target}"
    );
    runtime.send(Message::Command(CommandId::PreviousPage));
    assert_eq!(runtime.workspace_scroll(), runtime.page_scroll(1));
}

#[test]
fn a_tab_opened_later_scrolls_with_the_page_arrows() {
    // User, 2026-10-12, debug build: the arrows moved the caret of a
    // document opened into a new tab, but the view only followed after a
    // resize. Each tab's scrollable had its own id, and libcosmic's tree
    // diff kept the first tab's id at that place in the tree, so the new
    // tab's scrolls matched nothing; the new tab also had no viewport size
    // until an event reached its page surface.
    let mut runtime = Runtime::new(page_app(Document::default()));
    let path = std::env::temp_dir().join(format!("amipro-later-tab-{}.amipro", std::process::id()));
    page_fixture().save_native(&path).unwrap();
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        path.clone(),
    )));
    std::fs::remove_file(&path).unwrap();
    assert_eq!(runtime.app.sessions.len(), 2);
    assert_eq!(runtime.app.active_session, 1);
    assert_eq!(runtime.app.viewport, runtime.app.sessions[0].viewport);

    runtime.send(Message::Command(CommandId::NextPage));
    let target = runtime.page_scroll(2);
    assert!(target > 0.0);
    let scrolled = runtime.workspace_scroll();
    assert!(
        (scrolled - target).abs() < 0.5,
        "scrolled to {scrolled} for page 2 at {target}"
    );
}

#[test]
fn each_tab_shows_its_own_scroll_offset() {
    // The tabs share the workspace's scrollable; switching tabs restores
    // each one's own offset (a new tab starts at the top).
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.send(Message::Command(CommandId::NextPage));
    // The scrollable reports the offset back as it would on screen.
    let first = runtime.page_scroll(2);
    runtime.app.sessions[0].scroll_offset.y = first;
    let path =
        std::env::temp_dir().join(format!("amipro-tab-offsets-{}.amipro", std::process::id()));
    page_fixture().save_native(&path).unwrap();
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        path.clone(),
    )));
    std::fs::remove_file(&path).unwrap();
    assert_eq!(runtime.workspace_scroll(), 0.0);

    runtime.send(Message::ActivateTab(0));
    assert!((runtime.workspace_scroll() - first).abs() < 0.5);
    runtime.send(Message::Command(CommandId::NextDocument));
    assert_eq!(runtime.workspace_scroll(), 0.0);
}

/// The main text's lines of paragraph `paragraph` on page 1.
fn main_lines(app: &AmiProApp, paragraph: usize) -> Vec<amipro_layout::LineBox> {
    app.pages.layout().pages[0]
        .lines
        .iter()
        .filter(|line| {
            line.container == amipro_layout::Container::Main && line.paragraph_index == paragraph
        })
        .cloned()
        .collect()
}

#[test]
fn underline_toggles_on_the_page_selection_in_one_undo_step() {
    // Gap A1: Format › Underline (Ctrl+U) on the page surface.
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    assert_eq!(
        command::metadata(CommandId::Underline)
            .shortcut
            .map(|shortcut| shortcut.key_bind().to_string()),
        Some("Ctrl + U".to_owned())
    );
    // At a caret it formats the text typed next (VM fixture C1).
    move_to(&mut app, at(main, 0, 0), 1, false);
    assert!(app.command_enabled(CommandId::Underline));
    move_to(&mut app, at(main, 0, 9), 1, true);
    assert!(app.command_enabled(CommandId::Underline));
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Underline));
    let underlined = |app: &AmiProApp| {
        app.document.paragraphs()[0]
            .runs
            .iter()
            .any(|run| run.start == 0 && run.end == 9 && run.format.underline == Some(true))
    };
    assert!(underlined(&app));
    assert_eq!(app.history.undo_len(), undo + 1);
    // The selection stays, and the toggle shows it.
    assert_eq!(
        app.page_selection.ordered(),
        (at(main, 0, 0), at(main, 0, 9))
    );
    assert_eq!(
        selected_character_toggle_state(
            &app.document,
            app.editor.cursor(),
            CharacterAttribute::Underline
        ),
        ToggleState::On
    );
    // Again: off.
    send(&mut app, Message::Command(CommandId::Underline));
    assert!(!underlined(&app));
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(underlined(&app));
}

#[test]
fn the_alignment_commands_align_the_selected_paragraphs_and_show_which() {
    // Gap A2: Align Left, Center, Align Right, and Justify on the page
    // surface, over the paragraphs' styles, one undo step each, with the
    // Paragraph group's selected state from the selection.
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    for (command, key) in [
        (CommandId::AlignLeft, "L"),
        (CommandId::Center, "E"),
        (CommandId::AlignRight, "R"),
        (CommandId::Justify, "J"),
    ] {
        assert_eq!(
            command::metadata(command)
                .shortcut
                .map(|shortcut| shortcut.key_bind().to_string()),
            Some(format!("Ctrl + {key}"))
        );
        assert!(app.command_enabled(command));
    }
    move_to(&mut app, at(main, 0, 3), 1, false);
    assert_eq!(app.selected_alignment(), Some(Alignment::Left));
    let slot = |line: &amipro_layout::LineBox| (line.x, line.x + line.slot_width);
    let first_word = |line: &amipro_layout::LineBox| line.words.first().unwrap().x;
    let last_word = |line: &amipro_layout::LineBox| {
        let word = line.words.last().unwrap();
        word.x + word.width
    };

    // Center: the caret's paragraph only.
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Center));
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(
        app.document.paragraphs()[0].format.alignment,
        Some(Alignment::Centre)
    );
    assert_eq!(app.document.paragraphs()[1].format.alignment, None);
    assert_eq!(app.selected_alignment(), Some(Alignment::Centre));
    let last = main_lines(&app, 0).pop().unwrap();
    let (left, right) = slot(&last);
    assert!(
        ((first_word(&last) - left) - (right - last_word(&last))).abs() < 2.0,
        "centred: {last:?}"
    );

    // Align Right over a selection of two paragraphs.
    move_to(&mut app, at(main, 1, 4), 1, true);
    assert_eq!(app.selected_alignment(), None, "mixed");
    send(&mut app, Message::Command(CommandId::AlignRight));
    assert_eq!(app.selected_alignment(), Some(Alignment::Right));
    assert_eq!(
        app.page_selection.ordered(),
        (at(main, 0, 3), at(main, 1, 4))
    );
    for paragraph in [0, 1] {
        let last = main_lines(&app, paragraph).pop().unwrap();
        assert!(
            (last_word(&last) - slot(&last).1).abs() < 1.0,
            "right: {last:?}"
        );
    }

    // Justify: full lines are spaced out; the last line is not.
    send(&mut app, Message::Command(CommandId::Justify));
    let lines = main_lines(&app, 0);
    assert!(lines.len() > 1);
    assert!(lines[0].spaced);
    assert!((last_word(&lines[0]) - slot(&lines[0]).1).abs() < 1.0);
    assert!(!lines.last().unwrap().spaced);

    // Align Left, then undo back through each step.
    send(&mut app, Message::Command(CommandId::AlignLeft));
    assert_eq!(app.selected_alignment(), Some(Alignment::Left));
    for expected in [Alignment::Justify, Alignment::Right, Alignment::Centre] {
        send(&mut app, Message::Command(CommandId::Undo));
        assert_eq!(
            app.document.paragraphs()[0].format.alignment,
            Some(expected)
        );
    }
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document.paragraphs()[0].format.alignment, None);

    // With a frame selected there is no paragraph to align.
    let frame = text_frame(&app.document);
    frame_action(
        &mut app,
        crate::frame_editing::FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Center));
    assert_eq!(app.history.undo_len(), undo);
    assert_eq!(app.selected_alignment(), None);
}

#[test]
fn the_standard_bar_has_underline_and_the_paragraph_group_with_their_states() {
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    let groups = chrome::toolbar_groups(&app.preferences.toolbar);
    // Draw Frame ends the bar, as Ami Pro's frame SmartIcon (F1).
    assert_eq!(
        groups.last().unwrap(),
        &[ToolbarItem::Command(CommandId::DrawFrame)]
    );
    // Bullets, Numbering, Promote, and Demote before it (B2, O4).
    assert_eq!(
        groups[groups.len() - 2],
        &[
            ToolbarItem::Command(CommandId::Bullets),
            ToolbarItem::Command(CommandId::Numbering),
            ToolbarItem::Command(CommandId::Promote),
            ToolbarItem::Command(CommandId::Demote),
        ]
    );
    assert_eq!(
        groups[groups.len() - 3],
        &[
            ToolbarItem::Command(CommandId::AlignLeft),
            ToolbarItem::Command(CommandId::Center),
            ToolbarItem::Command(CommandId::AlignRight),
            ToolbarItem::Command(CommandId::Justify),
        ]
    );
    assert!(groups[groups.len() - 4].contains(&ToolbarItem::Command(CommandId::Underline)));
    move_to(&mut app, at(main, 0, 3), 1, false);
    send(&mut app, Message::Command(CommandId::Center));
    let state = ActionBarState {
        enabled_commands: HashMap::new(),
        style_names: Vec::new(),
        style_ids: Vec::new(),
        selected_style_index: None,
        bold: ToggleState::Off,
        italic: ToggleState::Off,
        underline: ToggleState::Off,
        alignment: app.selected_alignment(),
        list: app.selected_list(),
        fast_format: false,
        tabs_returns: false,
        styles_visible: false,
        toolbar: app.preferences.toolbar.clone(),
    };
    assert_eq!(state.toggle_state(CommandId::Center), ToggleState::On);
    assert_eq!(state.toggle_state(CommandId::AlignLeft), ToggleState::Off);
    // The whole window still lays out and draws with the wider bar.
    draw_window(&app);
}

#[test]
fn character_states_follow_the_caret_and_choices_apply_to_typing_c1() {
    // VM fixture C1: Ami Pro checks Bold with the caret just before or just
    // after a bold word, not before the space ahead of it; Ctrl+B with
    // nothing selected makes the next typing bold, and Ctrl+B again ends it.
    let mut document = Document::from_plain_text("plain bold plain");
    document.paragraph_mut(0).runs.push(CharacterRun {
        start: 6,
        end: 10,
        format: CharacterFormat {
            bold: Some(true),
            ..CharacterFormat::default()
        },
    });
    let mut app = page_app(document);
    let main = app.document.main_flow_id();
    let bold_at = |app: &mut AmiProApp, byte| {
        move_to(app, at(main, 0, byte), 1, false);
        app.character_state(CharacterAttribute::Bold)
    };
    assert_eq!(bold_at(&mut app, 5), ToggleState::Off);
    assert_eq!(bold_at(&mut app, 6), ToggleState::On);
    assert_eq!(bold_at(&mut app, 8), ToggleState::On);
    assert_eq!(bold_at(&mut app, 10), ToggleState::On);
    assert_eq!(bold_at(&mut app, 11), ToggleState::Off);
    // The menu's check follows the caret too (the toolbar reads the same
    // state in `view`).
    move_to(&mut app, at(main, 0, 8), 1, false);
    assert_eq!(menu_check(&app, CommandId::Bold), Some(true));

    // Typing at the end with Bold chosen, then Bold again.
    move_to(&mut app, at(main, 0, 16), 1, false);
    let depth = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Bold));
    assert_eq!(
        app.document.paragraphs()[0].runs.len(),
        1,
        "nothing changed yet"
    );
    assert_eq!(
        app.character_state(CharacterAttribute::Bold),
        ToggleState::On
    );
    assert_eq!(menu_check(&app, CommandId::Bold), Some(true));
    page(&mut app, PageAction::Insert("XY".into()));
    send(&mut app, Message::Command(CommandId::Bold));
    assert_eq!(
        app.character_state(CharacterAttribute::Bold),
        ToggleState::Off
    );
    page(&mut app, PageAction::Insert("z".into()));
    let paragraph = &app.document.paragraphs()[0];
    assert_eq!(paragraph.text, "plain bold plainXYz");
    let bold: Vec<_> = paragraph
        .runs
        .iter()
        .filter(|run| run.format.bold == Some(true))
        .map(|run| &paragraph.text[run.start..run.end])
        .collect();
    assert_eq!(bold, ["bold", "XY"]);
    // Each typed and formatted insertion is one undo step.
    assert_eq!(app.history.undo_len(), depth + 2);
    send(&mut app, Message::Command(CommandId::Undo));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document.paragraphs()[0].text, "plain bold plain");
    assert_eq!(app.document.paragraphs()[0].runs.len(), 1);

    // A choice lapses when the caret moves.
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Italic));
    assert_eq!(
        app.character_state(CharacterAttribute::Italic),
        ToggleState::On
    );
    move_to(&mut app, at(main, 0, 3), 1, false);
    assert_eq!(
        app.character_state(CharacterAttribute::Italic),
        ToggleState::Off
    );
    page(&mut app, PageAction::Insert("Q".into()));
    assert!(
        app.document.paragraphs()[0]
            .runs
            .iter()
            .all(|run| run.format.italic.is_none())
    );
}

#[test]
fn an_empty_paragraph_shows_and_types_in_its_end_format() {
    // An empty paragraph holding Ami Pro's `<:f200,,><+!>` is set in 10 pt
    // bold (its end format): Bold is checked there, the Font dialog starts
    // at 10 pt, and typing there is 10 pt bold, with Italic chosen on top.
    let mut document = Document::from_plain_text("text\n\nmore");
    document.paragraph_mut(1).end_format = CharacterFormat {
        font_size_points: Some(10.0),
        bold: Some(true),
        ..CharacterFormat::default()
    };
    let mut app = page_app(document);
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 1, 0), 1, false);
    assert_eq!(
        app.character_state(CharacterAttribute::Bold),
        ToggleState::On
    );
    assert_eq!(app.selection_font().1, 10.0);
    send(&mut app, Message::Command(CommandId::Italic));
    page(&mut app, PageAction::Insert("ab".into()));
    let paragraph = &app.document.paragraphs()[1];
    assert_eq!(paragraph.text, "ab");
    assert_eq!(
        paragraph.runs,
        [CharacterRun {
            start: 0,
            end: 2,
            format: CharacterFormat {
                font_size_points: Some(10.0),
                bold: Some(true),
                italic: Some(true),
                ..CharacterFormat::default()
            },
        }]
    );
    // Elsewhere the caret follows the characters, as before.
    move_to(&mut app, at(main, 2, 0), 1, false);
    assert_eq!(
        app.character_state(CharacterAttribute::Bold),
        ToggleState::Off
    );
}

#[test]
fn ami_pros_text_menu_items_format_text_through_the_menu_and_dialogs() {
    use crate::character_dialogs::{EffectsAction, FontAction, PALETTE};
    // User decision, 2026-10-12: Ami Pro's Text menu items in Format.
    let mut app = page_app(Document::from_plain_text("one two three four"));
    let main = app.document.main_flow_id();
    let (_, layout) = MENU_LAYOUTS
        .iter()
        .find(|(root, _)| *root == command::MenuRoot::Format)
        .unwrap();
    let entries = menu_entries(&app, layout);
    let labels: Vec<&str> = entries
        .iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item { label, .. } | MenuEntry::Folder { label, .. } => Some(label.as_str()),
            MenuEntry::Divider => None,
        })
        .collect();
    let text_items = [
        "Bold",
        "Italic",
        "Underline",
        "Word Underline",
        "Caps",
        "Special Effects…",
        "Font…",
        "Normal",
    ];
    assert!(
        labels
            .windows(text_items.len())
            .any(|window| window == text_items),
        "{labels:?}"
    );
    let caps = entries
        .iter()
        .find_map(|entry| match entry {
            MenuEntry::Folder { label, entries, .. } if label == "Caps" => Some(entries),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        caps.iter()
            .map(|entry| match entry {
                MenuEntry::Item { label, checked, .. } => (label.as_str(), checked.is_some()),
                _ => ("", false),
            })
            .collect::<Vec<_>>(),
        [
            ("Upper Case", true),
            ("Lower Case", true),
            ("Initial Caps", true),
            ("Small Caps", true)
        ]
    );
    let select = |app: &mut AmiProApp, start, end| {
        move_to(app, at(main, 0, start), 1, false);
        move_to(app, at(main, 0, end), 1, true);
    };
    let format_of = |app: &AmiProApp, byte: usize| {
        app.document.paragraphs()[0]
            .runs
            .iter()
            .find(|run| run.start <= byte && byte < run.end)
            .map(|run| run.format.clone())
            .unwrap_or_default()
    };

    // Word Underline, checked over the word and at a caret in it.
    select(&mut app, 4, 7);
    send(&mut app, Message::Command(CommandId::WordUnderline));
    assert_eq!(format_of(&app, 5).word_underline, Some(true));
    move_to(&mut app, at(main, 0, 5), 1, false);
    assert_eq!(menu_check(&app, CommandId::WordUnderline), Some(true));

    // Each caps replaces the others; choosing a checked one removes it.
    select(&mut app, 8, 13);
    send(&mut app, Message::Command(CommandId::UpperCase));
    assert_eq!(format_of(&app, 9).caps, Some(Caps::Upper));
    send(&mut app, Message::Command(CommandId::SmallCaps));
    assert_eq!(format_of(&app, 9).caps, Some(Caps::Small));
    assert_eq!(menu_check(&app, CommandId::UpperCase), Some(false));
    assert_eq!(menu_check(&app, CommandId::SmallCaps), Some(true));
    send(&mut app, Message::Command(CommandId::SmallCaps));
    assert_eq!(format_of(&app, 9).caps, None);

    // Special Effects: the boxes start as the text shows them; OK switches
    // the changed ones in one undo step; Cancel changes nothing.
    select(&mut app, 14, 18);
    let depth = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::SpecialEffects));
    assert_eq!(
        app.special_effects.as_ref().map(|effects| effects.chosen),
        Some([false; 4])
    );
    send(
        &mut app,
        Message::SpecialEffects(EffectsAction::Toggle(0, true)),
    );
    send(
        &mut app,
        Message::SpecialEffects(EffectsAction::Toggle(3, true)),
    );
    send(&mut app, Message::SpecialEffects(EffectsAction::Apply));
    assert!(app.special_effects.is_none());
    let four = format_of(&app, 15);
    assert_eq!(
        (four.superscript, four.strikethrough),
        (Some(true), Some(true))
    );
    assert_eq!(app.history.undo_len(), depth + 1);
    send(&mut app, Message::Command(CommandId::SpecialEffects));
    assert_eq!(
        app.special_effects.as_ref().map(|effects| effects.chosen),
        Some([true, false, false, true])
    );
    send(&mut app, Message::SpecialEffects(EffectsAction::Cancel));
    assert_eq!(app.history.undo_len(), depth + 1);

    // Font: starts with the text's face and size; a bad size is refused
    // with the dialog left open; OK writes face, size, and colour.
    select(&mut app, 0, 3);
    send(&mut app, Message::Command(CommandId::Font));
    let dialog = app.font_dialog.as_ref().unwrap();
    assert_eq!(dialog.families[dialog.family], "Times New Roman");
    assert_eq!(dialog.size, "12");
    send(&mut app, Message::FontDialog(FontAction::Size("0".into())));
    send(&mut app, Message::FontDialog(FontAction::Apply));
    assert!(app.font_dialog.as_ref().unwrap().error.is_some());
    send(&mut app, Message::FontDialog(FontAction::Size("18".into())));
    send(
        &mut app,
        Message::FontDialog(FontAction::Colour(PALETTE[0].0)),
    );
    send(&mut app, Message::FontDialog(FontAction::Apply));
    assert!(app.font_dialog.is_none());
    let one = format_of(&app, 1);
    assert_eq!(
        (one.font_family.as_deref(), one.font_size_points, one.colour),
        (Some("Times New Roman"), Some(18.0), Some(0x0000_00FF))
    );
    // Revert to style.
    send(&mut app, Message::Command(CommandId::Font));
    assert_eq!(app.font_dialog.as_ref().unwrap().size, "18");
    send(&mut app, Message::FontDialog(FontAction::Revert(true)));
    send(&mut app, Message::FontDialog(FontAction::Apply));
    assert_eq!(format_of(&app, 1), CharacterFormat::default());

    // A font chosen at the caret applies to the text typed there.
    move_to(&mut app, at(main, 0, 18), 1, false);
    send(&mut app, Message::Command(CommandId::Font));
    send(&mut app, Message::FontDialog(FontAction::SizePreset(11)));
    send(&mut app, Message::FontDialog(FontAction::Apply));
    page(&mut app, PageAction::Insert("!".into()));
    assert_eq!(format_of(&app, 18).font_size_points, Some(24.0));

    // Normal returns the whole selection to its style (VM fixture C1).
    select(&mut app, 0, 19);
    send(&mut app, Message::Command(CommandId::Center));
    send(&mut app, Message::Command(CommandId::ClearDirectFormatting));
    assert!(app.document.paragraphs()[0].runs.is_empty());
    assert_eq!(app.document.paragraphs()[0].format.alignment, None);
}

#[test]
fn the_format_menu_checks_bold_italic_underline_and_the_alignment() {
    // User, 2026-10-12: check marks as for the Styles panel, on the other
    // items with an on or off state. They follow the page selection in its
    // own flow, here a frame's text.
    let mut app = page_app(page_fixture());
    let frame_flow = flow_of(&app.document, text_frame(&app.document));
    let format_checks = |app: &AmiProApp| -> Vec<(CommandId, bool)> {
        let (_, layout) = MENU_LAYOUTS
            .iter()
            .find(|(root, _)| *root == command::MenuRoot::Format)
            .unwrap();
        // Submenus included (Alignment ▸ holds the alignments).
        fn checked(entries: Vec<MenuEntry>, found: &mut Vec<(CommandId, bool)>) {
            for entry in entries {
                match entry {
                    MenuEntry::Item {
                        command: MenuCommand::Command(command),
                        checked: Some(true),
                        ..
                    } => found.push((command, true)),
                    MenuEntry::Folder { entries, .. } => checked(entries, found),
                    _ => {}
                }
            }
        }
        let mut found = Vec::new();
        checked(menu_entries(app, layout), &mut found);
        found
    };
    move_to(&mut app, at(frame_flow, 0, 0), 1, false);
    assert_eq!(format_checks(&app), [(CommandId::AlignLeft, true)]);
    move_to(&mut app, at(frame_flow, 0, 5), 1, true);
    send(&mut app, Message::Command(CommandId::Bold));
    send(&mut app, Message::Command(CommandId::Underline));
    send(&mut app, Message::Command(CommandId::Justify));
    assert_eq!(
        app.character_state(CharacterAttribute::Bold),
        ToggleState::On
    );
    assert_eq!(
        app.character_state(CharacterAttribute::Italic),
        ToggleState::Off
    );
    assert_eq!(
        format_checks(&app),
        [
            (CommandId::Bold, true),
            (CommandId::Underline, true),
            (CommandId::Justify, true)
        ]
    );
    // Every Format entry has the check column, so the labels line up.
    let (_, layout) = MENU_LAYOUTS
        .iter()
        .find(|(root, _)| *root == command::MenuRoot::Format)
        .unwrap();
    assert!(menu_entries(&app, layout).iter().all(|entry| matches!(
        entry,
        MenuEntry::Divider
            | MenuEntry::Item {
                checked: Some(_),
                ..
            }
            | MenuEntry::Folder { indented: true, .. }
    )));
    // Part bold: neither checked nor mixed in the menu.
    move_to(&mut app, at(frame_flow, 0, 0), 1, false);
    move_to(&mut app, at(frame_flow, 0, 9), 1, true);
    assert_eq!(
        app.character_state(CharacterAttribute::Bold),
        ToggleState::Mixed
    );
    assert!(!format_checks(&app).contains(&(CommandId::Bold, true)));
}

#[test]
fn the_right_click_menus_offer_the_text_or_frame_commands() {
    // `UI_SPEC.md` §16 (user, 2026-10-13); Ami Pro's right-click target
    // comes first.
    use crate::frame_editing::FrameAction;
    let labels = |entries: &[MenuEntry]| -> Vec<String> {
        entries
            .iter()
            .map(|entry| match entry {
                MenuEntry::Item { label, enabled, .. } => {
                    format!("{label}{}", if *enabled { "" } else { " (off)" })
                }
                MenuEntry::Folder { label, .. } => format!("{label} ▸"),
                MenuEntry::Divider => "—".into(),
            })
            .collect()
    };
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 1, 2), 1, false);
    let menu = app.page_context_menu();
    assert_eq!(
        labels(&menu),
        [
            "Modify Style “Body Text”…",
            "—",
            "Cut (off)",
            "Copy (off)",
            "Paste",
            "—",
            "Font…",
            "Special Effects…",
            "—",
            "Bullets and Numbering ▸",
            "—",
            "Paragraph Style ▸",
        ]
    );
    // The style list checks the paragraph's style and applies one.
    let Some(MenuEntry::Folder { entries, .. }) = menu.last() else {
        panic!("the style list");
    };
    let heading = app
        .document
        .styles()
        .values()
        .find(|style| style.name == "Heading 1")
        .unwrap()
        .id;
    let mut command = None;
    for entry in entries {
        let MenuEntry::Item {
            command: MenuCommand::ApplyStyle(style),
            checked,
            ..
        } = entry
        else {
            panic!("styles only: {entry:?}");
        };
        assert_eq!(*checked, Some(*style == app.document.paragraphs()[1].style));
        if *style == heading {
            command = Some(MenuCommand::ApplyStyle(*style));
        }
    }
    send(&mut app, command.unwrap().message());
    assert_eq!(app.document.paragraphs()[1].style, heading);
    // With a selection, Cut and Copy are on.
    move_to(&mut app, at(main, 1, 8), 1, true);
    assert!(labels(&app.page_context_menu()).contains(&"Cut".to_owned()));
    // On a frame: the frame menu.
    let frame = text_frame(&app.document);
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    assert_eq!(
        labels(&app.page_context_menu()),
        [
            "Frame Properties…",
            "Graphics Scaling… (off)",
            "—",
            "Bring to Front",
            "Send to Back",
            "—",
            "Group Frames (off)",
            "Ungroup Frames (off)",
            "—",
            "Delete",
        ]
    );
}

/// Where row `row` (from 0) of a right-click menu opened at `click` lies,
/// with `dividers` dividers above it: the pinned libcosmic opens the menu
/// right of and below a click left of the window's centre, its rows 36
/// pixels apart and its dividers 1 pixel.
fn menu_row(click: cosmic::iced::Point, row: usize, dividers: usize) -> cosmic::iced::Point {
    cosmic::iced::Point::new(
        click.x + 100.0,
        click.y + 18.0 + 36.0 * row as f32 + dividers as f32,
    )
}

/// The commands among `messages`.
fn commands(messages: Vec<Message>) -> Vec<CommandId> {
    messages
        .into_iter()
        .filter_map(|message| match message {
            Message::Command(command) => Some(command),
            _ => None,
        })
        .collect()
}

/// Scrolls the workspace to the second page; the redraw that follows tells
/// the session where the pages are.
fn scrolled_runtime() -> Runtime {
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.send(Message::Command(CommandId::NextPage));
    runtime.redraw();
    let scroll = runtime.workspace_scroll();
    assert!(scroll > 0.0);
    assert_eq!(runtime.app.scroll_offset.y, scroll);
    runtime
}

#[test]
fn the_right_click_menu_runs_its_items_on_a_scrolled_page() {
    // User, 2026-10-13: with the Styles panel off, Modify Style from the
    // right-click menu did nothing, and Cut, Copy, and Paste did not work.
    // The menu sat inside the scrolled pages, where libcosmic's in-window
    // menu gets the pointer in scrolled coordinates: once the pages were
    // scrolled it neither laid out at the pointer nor took clicks. (With the
    // panel on, the pages had just jumped to the top, where it worked.)
    use cosmic::iced::mouse::Button;
    let mut runtime = scrolled_runtime();
    assert!(!runtime.app.rail_open());
    let text = cosmic::iced::Point::new(350.0, 330.0);
    runtime.click(Button::Left, text);
    let scroll = runtime.workspace_scroll();
    assert!(scroll > 0.0, "still scrolled");
    let caret = runtime.app.page_selection.focus;
    let page = runtime.app.page_selection.page;

    // Copy and Cut take the selection under the pointer.
    let length = runtime.app.document.flow(caret.flow).unwrap().paragraphs[caret.paragraph]
        .text
        .len();
    move_to(
        &mut runtime.app,
        at(caret.flow, caret.paragraph, 0),
        page,
        false,
    );
    move_to(
        &mut runtime.app,
        at(caret.flow, caret.paragraph, length),
        page,
        true,
    );
    let selected = runtime.app.selected_text().unwrap();
    runtime.click(Button::Right, text);
    assert_eq!(runtime.app.selected_text().as_ref(), Some(&selected));
    assert_eq!(
        commands(runtime.click(Button::Left, menu_row(text, 2, 1))),
        [CommandId::Copy]
    );
    assert_eq!(runtime.clipboard.as_ref(), Some(&selected));
    runtime.click(Button::Right, text);
    assert_eq!(
        commands(runtime.click(Button::Left, menu_row(text, 1, 1))),
        [CommandId::Cut]
    );
    assert!(!flow_text(&runtime.app, caret.flow).contains(&selected));
    // Paste puts it back where the right button put the caret.
    runtime.clipboard = Some(selected.clone());
    runtime.click(Button::Right, text);
    assert_eq!(
        commands(runtime.click(Button::Left, menu_row(text, 3, 1))),
        [CommandId::Paste]
    );
    assert_eq!(
        flow_text(&runtime.app, caret.flow)
            .matches(&selected)
            .count(),
        1
    );

    // Modify Style opens Style Properties in the closed rail, and the
    // pages stay where they were.
    runtime.click(Button::Right, text);
    assert_eq!(
        commands(runtime.click(Button::Left, menu_row(text, 0, 0))),
        [CommandId::ModifyCurrentStyle]
    );
    assert!(runtime.app.rail_open());
    assert!(runtime.app.style_properties.is_some());
    assert_eq!(runtime.workspace_scroll(), scroll);
}

#[test]
fn opening_and_closing_the_rail_keeps_the_page_scroll() {
    // The tree diff rebuilt the pages when the rail opened or closed beside
    // them, and they went back to the top.
    let mut runtime = scrolled_runtime();
    let scroll = runtime.workspace_scroll();
    let resize = |runtime: &mut Runtime, width: f32| {
        runtime.window.width = width;
        runtime.send(Message::WindowResized(width));
    };
    // The window's first report of its width.
    resize(&mut runtime, 1_280.0);
    // Docked beside the pages: open, closed, and open again.
    for open in [true, false, true] {
        runtime.send(Message::Command(CommandId::StylesPanel));
        assert_eq!(runtime.app.rail_open(), open);
        assert_eq!(runtime.workspace_scroll(), scroll);
    }
    // A narrow window keeps the open rail, over the pages (the user,
    // 2026-11-06: it used to go away when the window was un-maximized).
    resize(&mut runtime, 900.0);
    assert!(runtime.app.rail_overlays() && runtime.app.rail_open());
    assert_eq!(runtime.workspace_scroll(), scroll);
    // Closed there, it stays closed when the window widens and narrows.
    runtime.send(Message::Command(CommandId::StylesPanel));
    assert!(!runtime.app.rail_open());
    resize(&mut runtime, 1_280.0);
    assert!(!runtime.app.rail_overlays() && !runtime.app.rail_open());
    resize(&mut runtime, 900.0);
    assert!(!runtime.app.rail_open());
    runtime.send(Message::Command(CommandId::StylesPanel));
    assert!(runtime.app.rail_overlays() && runtime.app.rail_open());
    assert_eq!(runtime.workspace_scroll(), scroll);
    // Wide again, the rail docks.
    resize(&mut runtime, 1_280.0);
    assert!(!runtime.app.rail_overlays() && runtime.app.rail_open());
    assert_eq!(runtime.workspace_scroll(), scroll);
}

#[test]
fn the_styles_panel_row_menu_acts_on_its_row_when_the_list_is_scrolled() {
    // The page menu's defect (user, 2026-10-13) in the Styles panel: each
    // row had its own menu inside the scrolled list, so once the list was
    // scrolled no row's menu took clicks. One menu around the list now acts
    // on the row the right button pressed.
    use cosmic::iced::mouse::{Button, Event as Mouse, ScrollDelta};
    let mut document = page_fixture();
    let body = document
        .styles()
        .values()
        .find(|style| style.name == "Body Text")
        .unwrap()
        .clone();
    let next = document.styles().keys().map(|id| id.0).max().unwrap() + 1;
    for n in 0..30 {
        let id = StyleId(next + n);
        let name = format!("Style {n:02}");
        document.styles_mut().insert(
            id,
            ParagraphStyle {
                id,
                name,
                ..body.clone()
            },
        );
    }
    let mut runtime = Runtime::new(page_app(document));
    runtime.send(Message::Command(CommandId::StylesPanel));
    let row = cosmic::iced::Point::new(100.0, 400.0);
    // A left click applies the row's style: that tells which row it is.
    let under_pointer = |runtime: &mut Runtime| {
        runtime.click(Button::Left, row);
        runtime.app.caret_paragraph().unwrap().style
    };
    let first = under_pointer(&mut runtime);
    runtime.events(&[
        (
            cosmic::iced::Event::Mouse(Mouse::CursorMoved { position: row }),
            row,
        ),
        (
            cosmic::iced::Event::Mouse(Mouse::WheelScrolled {
                delta: ScrollDelta::Lines { x: 0.0, y: -3.0 },
            }),
            row,
        ),
    ]);
    runtime.redraw();
    let scrolled = under_pointer(&mut runtime);
    assert_ne!(scrolled, first, "the list scrolled");

    runtime.click(Button::Right, row);
    assert_eq!(runtime.app.rail.context_style, Some(scrolled));
    runtime.click(Button::Left, menu_row(row, 1, 0));
    assert_eq!(
        runtime
            .app
            .style_properties
            .as_ref()
            .map(|state| state.style),
        Some(scrolled),
        "Modify Style… opens the row's style"
    );
    assert_eq!(
        runtime.app.rail.context_style, None,
        "a chosen entry forgets"
    );
    // Dismissed by a click elsewhere, the menu forgets the row too.
    runtime.send(Message::StyleProperties(
        crate::style_properties::StyleAction::Back,
    ));
    runtime.click(Button::Right, row);
    assert!(runtime.app.rail.context_style.is_some());
    runtime.click(Button::Left, cosmic::iced::Point::new(700.0, 400.0));
    assert_eq!(
        runtime.app.rail.context_style, None,
        "the closed menu forgets"
    );
}

#[test]
fn style_properties_edit_the_style_live_in_the_rail_one_undo_step_each() {
    // `UI_SPEC.md` §15 (approved 2026-10-13).
    use crate::style_properties::{Field, Section, StyleAction};
    let properties = |app: &AmiProApp| app.style_properties.clone().unwrap();
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 1, 2), 1, false);
    let style = app.document.paragraphs()[1].style;
    let before = app.document.styles()[&style].clone();
    send(&mut app, Message::Command(CommandId::ModifyCurrentStyle));
    assert_eq!(properties(&app).style, style, "the caret paragraph's style");
    assert!(app.rail_open(), "the rail opens on Style Properties");
    assert!(
        cosmic::Application::dialog(&app).is_none(),
        "no modal dialog"
    );
    // Typing waits for Enter (or focus leaving the box); then the style
    // changes in the document at once.
    send(
        &mut app,
        Message::StyleProperties(StyleAction::Text(Field::Size, "20".into())),
    );
    assert_eq!(app.document.styles()[&style], before);
    send(
        &mut app,
        Message::StyleProperties(StyleAction::Submit(Field::Size)),
    );
    assert_eq!(app.document.styles()[&style].font_size_points, 20.0);
    send(
        &mut app,
        Message::StyleProperties(StyleAction::Section(Section::Spacing)),
    );
    send(
        &mut app,
        Message::StyleProperties(StyleAction::Text(Field::Above, "0.25".into())),
    );
    send(
        &mut app,
        Message::StyleProperties(StyleAction::Submit(Field::Above)),
    );
    // A check box applies at once.
    send(
        &mut app,
        Message::StyleProperties(StyleAction::Attribute(
            crate::style_properties::FontAttribute::Italic,
            true,
        )),
    );
    let after = app.document.styles()[&style].clone();
    assert_eq!(after.spacing.above, 360);
    assert!(after.italic);
    assert_eq!(after.origin, amipro_document::StyleOrigin::Document);
    // Each change is one undo step, and the panel shows the undone style.
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(!app.document.styles()[&style].italic);
    assert_eq!(app.document.styles()[&style].spacing.above, 360);
    send(&mut app, Message::Command(CommandId::Undo));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document.styles()[&style], before);
    assert_eq!(properties(&app).draft, before);
    // The panel follows the caret to a paragraph in another style.
    let heading = app
        .document
        .styles()
        .values()
        .find(|style| style.name == "Heading 1")
        .unwrap()
        .id;
    move_to(&mut app, at(main, 3, 0), 1, false);
    send(&mut app, Message::ApplyStyle(heading));
    assert_eq!(properties(&app).style, heading);
    move_to(&mut app, at(main, 1, 0), 1, false);
    assert_eq!(properties(&app).style, style);
    // Save as New Style makes a style from the shown one and shows it.
    send(&mut app, Message::StyleProperties(StyleAction::SaveAs));
    send(
        &mut app,
        Message::StyleProperties(StyleAction::SaveAsName("Small print".into())),
    );
    send(
        &mut app,
        Message::StyleProperties(StyleAction::SaveAsConfirm),
    );
    let made = app
        .document
        .styles()
        .values()
        .find(|style| style.name == "Small print")
        .unwrap()
        .clone();
    assert_eq!(made.font_size_points, before.font_size_points);
    assert_eq!(properties(&app).style, made.id);
    // ‹ Styles returns to the list; × closes the rail.
    send(&mut app, Message::StyleProperties(StyleAction::Back));
    assert!(app.style_properties.is_none());
    assert!(app.rail_open());
    send(&mut app, Message::Command(CommandId::ModifyCurrentStyle));
    send(&mut app, Message::StyleProperties(StyleAction::Close));
    assert!(app.style_properties.is_none());
    assert!(!app.rail_open());
    // Create Style's Modify… creates the style and shows it.
    send(&mut app, Message::Command(CommandId::CreateStyle));
    send(
        &mut app,
        Message::CreateStyle(crate::style_dialogs::CreateAction::Name(
            "Then modify".into(),
        )),
    );
    send(
        &mut app,
        Message::CreateStyle(crate::style_dialogs::CreateAction::Modify),
    );
    let made = app
        .document
        .styles()
        .values()
        .find(|style| style.name == "Then modify")
        .unwrap()
        .id;
    assert_eq!(properties(&app).style, made);
    assert!(app.rail_open());
}

/// Documents made in the app and saved through File › Save As as Ami Pro
/// documents, for opening in Ami Pro (`docs/research/VM_LAB.md`): a new
/// document typed in (APP1.SAM), and the page fixture with its frames,
/// header, and characters Ami Pro lacks, saved after the loss dialog's
/// Save Anyway (APP2.SAM). Writes into `AMIPRO_VM_IN`, which must exist.
#[test]
#[ignore = "writes Ami Pro documents for the VM"]
fn sam_documents_for_ami_pro() {
    let Some(directory) = std::env::var_os("AMIPRO_VM_IN") else {
        eprintln!("AMIPRO_VM_IN is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let save = |app: &mut AmiProApp, name: &str| {
        let session_id = app.id;
        send(
            app,
            Message::SaveDialogFinished {
                session_id,
                result: crate::FileDialogResult::Selected(directory.join(name)),
                format: Some(crate::SaveFormat::Sam),
            },
        );
    };

    let mut app = page_app(Document::default());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    page(
        &mut app,
        PageAction::Insert("A document made in AmiPro-Linux".into()),
    );
    send(&mut app, Message::Command(CommandId::Center));
    page(&mut app, PageAction::ParagraphBreak);
    send(&mut app, Message::Command(CommandId::AlignLeft));
    page(&mut app, PageAction::Insert("Typed text with a ".into()));
    send(&mut app, Message::Command(CommandId::Bold));
    page(&mut app, PageAction::Insert("bold".into()));
    send(&mut app, Message::Command(CommandId::Bold));
    page(&mut app, PageAction::Insert(" word, an ".into()));
    send(&mut app, Message::Command(CommandId::Italic));
    page(&mut app, PageAction::Insert("italic".into()));
    send(&mut app, Message::Command(CommandId::Italic));
    page(&mut app, PageAction::Insert(" one, and an ".into()));
    send(&mut app, Message::Command(CommandId::Underline));
    page(&mut app, PageAction::Insert("underlined".into()));
    send(&mut app, Message::Command(CommandId::Underline));
    page(
        &mut app,
        PageAction::Insert(" one: café, naïve, 25 °C, £5.".into()),
    );
    page(&mut app, PageAction::ParagraphBreak);
    page(&mut app, PageAction::Insert("A second paragraph.".into()));
    save(&mut app, "APP1");
    assert_eq!(app.sam_save, None, "nothing to lose: {:?}", app.sam_save);
    assert!(directory.join("APP1.SAM").exists(), "{}", app.status);

    let mut app = page_app(page_fixture());
    save(&mut app, "APP2");
    assert!(
        app.sam_save.is_some(),
        "the fixture has characters Ami Pro lacks"
    );
    eprintln!("APP2 losses: {:?}", app.sam_save.as_ref().unwrap().losses);
    send(&mut app, Message::SaveSamAnyway);
    assert!(directory.join("APP2.SAM").exists(), "{}", app.status);
}

#[test]
fn page_layout_properties_set_the_page_up_live_one_undo_step_each() {
    // `UI_SPEC.md` §18 (approved 2026-10-13, L1–L4).
    use crate::page_layout_properties::{Field, LayoutAction, Section};
    use amipro_editing::layout_setup::{MarginSide, Pages};
    let layout_send = |app: &mut AmiProApp, action: LayoutAction| {
        send(app, Message::PageLayoutProperties(action));
    };
    let properties = |app: &AmiProApp| app.page_layout_properties.clone().unwrap();
    let mut app = page_app(page_fixture());
    let before = app.document.document_layout().clone();
    send(&mut app, Message::Command(CommandId::PageLayout));
    assert!(app.rail_open(), "the rail opens on Page Layout Properties");
    assert!(
        cosmic::Application::dialog(&app).is_none(),
        "no modal dialog"
    );
    assert_eq!(
        properties(&app).layout,
        before.id,
        "the caret page's layout"
    );
    assert_eq!(properties(&app).pages, Pages::All);
    // A typed margin waits for Enter (or focus leaving the box), then the
    // pages lay out again at once.
    let left = Field::Margin(MarginSide::Left);
    layout_send(&mut app, LayoutAction::Text(left, "1.5".into()));
    assert_eq!(
        app.document.document_layout().right.layout.margins.left,
        1_440
    );
    layout_send(&mut app, LayoutAction::Submit(left));
    assert_eq!(
        app.document.document_layout().right.layout.margins.left,
        2_160
    );
    assert_eq!(app.pages.layout().pages[0].columns[0].x, 2_160.0);
    // A value Ami Pro would refuse is refused in place, with the reason.
    layout_send(&mut app, LayoutAction::Text(left, "8".into()));
    layout_send(&mut app, LayoutAction::Submit(left));
    assert_eq!(
        app.document.document_layout().right.layout.margins.left,
        2_160
    );
    assert!(properties(&app).error.unwrap().contains("no room"));
    assert_eq!(properties(&app).text(left), "8");
    // Lists, radio buttons, and check boxes apply at once.
    layout_send(&mut app, LayoutAction::Paper(2));
    assert_eq!(app.document.document_layout().right.width, 11_908);
    layout_send(&mut app, LayoutAction::Landscape(true));
    assert_eq!(
        (
            app.document.document_layout().right.width,
            app.document.document_layout().right.height
        ),
        (16_833, 11_908)
    );
    layout_send(&mut app, LayoutAction::Section(Section::Margins));
    layout_send(&mut app, LayoutAction::Columns(None, 2));
    assert_eq!(app.pages.layout().pages[0].columns.len(), 2);
    // Each change is one undo step, and the panel shows the undone layout.
    for _ in 0..4 {
        send(&mut app, Message::Command(CommandId::Undo));
    }
    assert_eq!(*app.document.document_layout(), before);
    assert_eq!(properties(&app).text(left), "1.00");
    // Right pages: the first change gives the layout left pages; Mirror
    // reflects them; All waits until Mirror is off.
    layout_send(&mut app, LayoutAction::Pages(Pages::Right));
    assert!(
        app.document.document_layout().left.is_none(),
        "not until a change"
    );
    layout_send(&mut app, LayoutAction::Text(left, "2".into()));
    layout_send(&mut app, LayoutAction::Submit(left));
    let layout = app.document.document_layout().clone();
    assert_eq!(layout.right.layout.margins.left, 2_880);
    assert_eq!(layout.left.as_ref().unwrap().layout.margins.left, 1_440);
    layout_send(&mut app, LayoutAction::Mirror(true));
    let left_pages = app.document.document_layout().left.clone().unwrap();
    assert_eq!(left_pages.layout.margins.right, 2_880, "mirrored");
    assert_eq!(
        app.pages.layout().pages[1].columns[0].x,
        1_440.0,
        "page 2 is a left page"
    );
    layout_send(&mut app, LayoutAction::Pages(Pages::All));
    assert!(app.document.document_layout().left.is_some());
    assert!(properties(&app).error.unwrap().contains("Mirror"));
    // ‹ Styles returns to the list; × closes the rail; Style Properties
    // and Page Layout Properties replace each other.
    layout_send(&mut app, LayoutAction::Back);
    assert!(app.page_layout_properties.is_none());
    assert!(app.rail_open());
    send(&mut app, Message::Command(CommandId::PageLayout));
    send(&mut app, Message::Command(CommandId::ModifyCurrentStyle));
    assert!(app.page_layout_properties.is_none());
    assert!(app.style_properties.is_some());
    send(&mut app, Message::Command(CommandId::PageLayout));
    assert!(app.style_properties.is_none());
    layout_send(&mut app, LayoutAction::Close);
    assert!(app.page_layout_properties.is_none());
    assert!(!app.rail_open());
    // In a margin the right-click menu offers Page Layout… first, and a
    // double-click there opens it.
    page(&mut app, PageAction::ContextAt { margin: true });
    let menu = app.page_context_menu();
    assert!(matches!(
        menu.first(),
        Some(MenuEntry::Item { label, .. }) if label == "Page Layout…"
    ));
    page(&mut app, PageAction::ContextAt { margin: false });
    assert!(matches!(
        app.page_context_menu().first(),
        Some(MenuEntry::Item { label, .. }) if label.starts_with("Modify Style")
    ));
    page(&mut app, PageAction::OpenPageLayout);
    assert!(app.page_layout_properties.is_some() && app.rail_open());
    // Edit › Go To Header puts the caret in the page's header, and the
    // panel opens the Header section; Begin on second page leaves the
    // first page without one.
    send(&mut app, Message::Command(CommandId::GoToHeader));
    let header = app.document.document_layout().header.right.unwrap();
    assert_eq!(app.page_selection.flow(), flow_of(&app.document, header));
    assert!(properties(&app).open.contains(&Section::Header));
    send(&mut app, Message::Command(CommandId::GoToFooter));
    let footer = app.document.document_layout().footer.right.unwrap();
    assert_eq!(app.page_selection.flow(), flow_of(&app.document, footer));
    assert!(properties(&app).open.contains(&Section::Footer));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    layout_send(
        &mut app,
        LayoutAction::FromSecondPage(amipro_editing::layout_setup::Band::Header, true),
    );
    send(&mut app, Message::Command(CommandId::GoToHeader));
    assert_eq!(app.page_selection.flow(), app.document.main_flow_id());
    assert!(app.status.contains("second page"), "{}", app.status);
}

/// Page Layout Properties in the rail, each section, light and dark
/// (`UI_SPEC.md` §18), into `AMIPRO_SCREENSHOT_DIR` as for
/// `window_screenshots`.
#[test]
fn page_layout_properties_screenshots() {
    use crate::page_layout_properties::{LayoutAction, Section};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    send(&mut app, Message::Command(CommandId::PageLayout));
    let write = |app: &AmiProApp, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    write(&app, "window-pagelayout-page-light", false);
    write(&app, "window-pagelayout-page-dark", true);
    send(
        &mut app,
        Message::PageLayoutProperties(LayoutAction::Section(Section::Page)),
    );
    for (section, name) in [
        (Section::Margins, "margins"),
        (Section::Tabs, "tabs"),
        (Section::Lines, "lines"),
        (Section::Header, "header"),
        (Section::Footer, "footer"),
    ] {
        send(
            &mut app,
            Message::PageLayoutProperties(LayoutAction::Section(section)),
        );
        if section == Section::Lines {
            send(
                &mut app,
                Message::PageLayoutProperties(LayoutAction::LineSide(
                    crate::page_layout_properties::LineSide::All,
                    true,
                )),
            );
        }
        write(&app, &format!("window-pagelayout-{name}-light"), false);
        write(&app, &format!("window-pagelayout-{name}-dark"), true);
        send(
            &mut app,
            Message::PageLayoutProperties(LayoutAction::Section(section)),
        );
    }
    // The one-shot dialogs (L3).
    send(&mut app, Message::Command(CommandId::HeaderFooter));
    write(&app, "window-floating-light", false);
    write(&app, "window-floating-dark", true);
    send(
        &mut app,
        Message::FloatingHeaderFooter(crate::page_dialogs::FloatingAction::Cancel),
    );
    send(&mut app, Message::Command(CommandId::PageNumber));
    write(&app, "window-pagenumber-light", false);
    write(&app, "window-pagenumber-dark", true);
}

#[test]
fn breaks_and_inserted_page_layouts_go_where_ami_pro_puts_them() {
    // VM fixtures PB1, PB3, CB1, IL1, IR1, IV1 (PL.8).
    use crate::page_layout_properties::LayoutAction;
    use amipro_document::{BreakKind, InlineObject};
    let mut app = page_app(Document::from_plain_text(
        "Main text paragraph one.\nSecond main paragraph.",
    ));
    let main = app.document.main_flow_id();
    let objects = |app: &AmiProApp, paragraph: usize| {
        app.document.flow(main).unwrap().paragraphs[paragraph]
            .objects
            .clone()
    };
    // A page break at the caret, the paragraph not split (PB1).
    move_to(&mut app, at(main, 0, 9), 1, false);
    send(&mut app, Message::Command(CommandId::PageBreak));
    assert_eq!(app.document.paragraphs().len(), 2);
    assert_eq!(&app.document.paragraphs()[0].text[9..12], "\u{FFFC}");
    assert_eq!(app.page_count(), 2);
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.page_count(), 1);
    // At the end of the text an empty paragraph follows for the next page,
    // in the same undo step (PB3).
    let end = app.document.paragraphs()[1].text.len();
    move_to(&mut app, at(main, 1, end), 1, false);
    send(&mut app, Message::Command(CommandId::PageBreak));
    assert_eq!(app.document.paragraphs().len(), 3);
    assert_eq!(app.page_selection.ordered().0, at(main, 2, 0));
    assert_eq!(app.caret_page_number(), 2);
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document.paragraphs().len(), 2);
    // A column break at the start of the caret's paragraph (CB1), once.
    move_to(&mut app, at(main, 0, 9), 1, false);
    send(&mut app, Message::Command(CommandId::ColumnBreak));
    assert!(matches!(
        objects(&app, 0)[..],
        [InlineObject::Break {
            kind: BreakKind::Column,
            ..
        }]
    ));
    assert!(app.document.paragraphs()[0].text.starts_with('\u{FFFC}'));
    send(&mut app, Message::Command(CommandId::ColumnBreak));
    assert!(app.status.contains("already"));
    send(&mut app, Message::Command(CommandId::Undo));
    // Insert Page Layout: a copy named 1 from the next page (IL1); the
    // panel follows the caret onto it.
    move_to(&mut app, at(main, 0, 9), 1, false);
    send(&mut app, Message::Command(CommandId::PageLayout));
    send(
        &mut app,
        Message::PageLayoutProperties(LayoutAction::Insert),
    );
    assert_eq!(app.document.layouts().len(), 2);
    let inserted = app.document.layouts()[1].clone();
    assert_eq!(inserted.name, "1");
    assert!(matches!(
        objects(&app, 0)[..],
        [InlineObject::Break { layout: Some(id), .. }] if id == inserted.id
    ));
    assert_eq!(app.caret_page_number(), 2);
    assert_eq!(
        app.page_layout_properties.as_ref().unwrap().layout,
        inserted.id
    );
    // An earlier layout again from a later page (Ami Pro's Revert, IV1).
    let end = app.document.paragraphs()[1].text.len();
    move_to(&mut app, at(main, 1, end), 2, false);
    send(
        &mut app,
        Message::PageLayoutProperties(LayoutAction::InsertChoice(1)),
    );
    send(
        &mut app,
        Message::PageLayoutProperties(LayoutAction::Insert),
    );
    let standard = app.document.document_layout().id;
    assert!(matches!(
        objects(&app, 1)[..],
        [InlineObject::Break { layout: Some(id), .. }] if id == standard
    ));
    assert_eq!(app.document.paragraphs().len(), 3);
    assert_eq!(app.page_count(), 3);
    send(&mut app, Message::Command(CommandId::Undo));
    // Remove on the inserted layout's page: the break and the layout, one
    // undo step, the caret where the break was (IR1).
    move_to(&mut app, at(main, 1, 0), 2, false);
    assert_eq!(
        app.page_layout_properties.as_ref().unwrap().layout,
        inserted.id
    );
    send(
        &mut app,
        Message::PageLayoutProperties(LayoutAction::Remove),
    );
    assert_eq!(app.document.layouts().len(), 1);
    assert!(objects(&app, 0).is_empty());
    assert_eq!(app.page_selection.ordered().0, at(main, 0, 9));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document.layouts().len(), 2);
    assert_eq!(objects(&app, 0).len(), 1);
}

#[test]
fn floating_headers_and_page_numbers_go_in_through_their_dialogs() {
    // VM fixtures FH1–FH3 and Page Numbering (PL.9, PL.10).
    use crate::page_dialogs::{FloatingAction, PageNumberAction};
    use amipro_document::{HeaderFooterKind, InlineObject, PageSides};
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    // A floating footer for right pages at the caret; the caret then goes
    // into it on the next page (FH1, FH2).
    move_to(&mut app, at(main, 3, 4), 1, false);
    send(&mut app, Message::Command(CommandId::HeaderFooter));
    assert!(cosmic::Application::dialog(&app).is_some());
    send(
        &mut app,
        Message::FloatingHeaderFooter(FloatingAction::Kind(HeaderFooterKind::Footer)),
    );
    send(
        &mut app,
        Message::FloatingHeaderFooter(FloatingAction::Pages(PageSides::Right)),
    );
    send(&mut app, Message::FloatingHeaderFooter(FloatingAction::Ok));
    assert!(app.floating_dialog.is_none());
    let InlineObject::FloatingHeaderFooter { kind, pages, flow } =
        app.document.paragraphs()[3].object_at(4).unwrap().clone()
    else {
        panic!("a floating footer");
    };
    assert_eq!((kind, pages), (HeaderFooterKind::Footer, PageSides::Right));
    assert_eq!(app.page_selection.flow(), flow);
    // Typed there, it shows on later right pages.
    page(&mut app, PageAction::Insert("Foot".into()));
    let shown = |app: &AmiProApp| {
        app.pages
            .layout()
            .pages
            .iter()
            .filter(|page| {
                page.lines
                    .iter()
                    .any(|line| line.container == amipro_layout::Container::Floating(flow))
            })
            .map(|page| page.number)
            .collect::<Vec<_>>()
    };
    assert!(
        shown(&app).iter().all(|page| page % 2 == 1 && *page > 1),
        "{:?}",
        shown(&app)
    );
    assert!(!shown(&app).is_empty());
    // At the mark the dialog removes it, in one undo step (FH3).
    move_to(&mut app, at(main, 3, 7), 1, false);
    send(&mut app, Message::Command(CommandId::HeaderFooter));
    assert!(app.floating_dialog.as_ref().unwrap().remove);
    send(&mut app, Message::FloatingHeaderFooter(FloatingAction::Ok));
    assert!(app.document.paragraphs()[3].objects.is_empty());
    assert!(app.document.flow(flow).is_none(), "its text goes with it");
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(app.document.flow(flow).is_some());
    // A page number in the header: style i from page 2, shown by the
    // engine as Ami Pro numbers; at the field the dialog changes it.
    send(&mut app, Message::Command(CommandId::GoToHeader));
    let header_flow = app.page_selection.flow();
    send(&mut app, Message::Command(CommandId::PageNumber));
    send(&mut app, Message::PageNumber(PageNumberAction::Style(2)));
    send(
        &mut app,
        Message::PageNumber(PageNumberAction::StartPage("2".into())),
    );
    send(
        &mut app,
        Message::PageNumber(PageNumberAction::StartNumber("1".into())),
    );
    send(&mut app, Message::PageNumber(PageNumberAction::Ok));
    assert!(app.page_number_dialog.is_none());
    let field_text = |app: &AmiProApp, number: u32| {
        app.pages.layout().pages[number as usize - 1]
            .lines
            .iter()
            .filter(|line| line.flow == header_flow)
            .flat_map(|line| line.words.iter())
            .find_map(|word| word.text.clone())
    };
    assert_eq!(field_text(&app, 1).as_deref(), Some(""));
    assert_eq!(field_text(&app, 2).as_deref(), Some("i"));
    assert_eq!(field_text(&app, 3).as_deref(), Some("ii"));
    page(&mut app, PageAction::MoveLeft { extend: false });
    send(&mut app, Message::Command(CommandId::PageNumber));
    assert!(app.page_number_dialog.as_ref().unwrap().existing.is_some());
    send(&mut app, Message::PageNumber(PageNumberAction::Style(3)));
    send(&mut app, Message::PageNumber(PageNumberAction::Ok));
    assert_eq!(field_text(&app, 3).as_deref(), Some("B"));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(field_text(&app, 3).as_deref(), Some("ii"));
}

#[test]
fn a_page_number_in_the_header_takes_a_font_like_text_pf1() {
    // The user's question (2026-10-13): how to change a header page
    // number's font. As in Ami Pro (VM fixture PF1): select the number and
    // use Format › Font…; it is drawn and measured in that font.
    use crate::character_dialogs::FontAction;
    use crate::page_dialogs::PageNumberAction;
    let mut app = page_app(page_fixture());
    send(&mut app, Message::Command(CommandId::GoToHeader));
    let header = app.page_selection.flow();
    send(&mut app, Message::Command(CommandId::PageNumber));
    send(
        &mut app,
        Message::PageNumber(PageNumberAction::LeadingText("Page ".into())),
    );
    send(&mut app, Message::PageNumber(PageNumberAction::Ok));
    let field_word = |app: &AmiProApp| {
        app.pages.layout().pages[0]
            .lines
            .iter()
            .filter(|line| line.flow == header)
            .flat_map(|line| line.words.iter())
            .find(|word| word.text.is_some())
            .cloned()
            .unwrap()
    };
    let before = field_word(&app);
    // The caret is after the field: select it with Shift+Left.
    page(&mut app, PageAction::MoveLeft { extend: true });
    send(&mut app, Message::Command(CommandId::Font));
    send(&mut app, Message::FontDialog(FontAction::Size("24".into())));
    send(&mut app, Message::FontDialog(FontAction::Apply));
    let paragraph = &app.document.flow(header).unwrap().paragraphs[0];
    let object = paragraph.object_offsets().next().unwrap();
    assert_eq!(paragraph.format_at(object).font_size_points, Some(24.0));
    let after = field_word(&app);
    assert_eq!(after.text, before.text);
    assert!(
        after.width > before.width * 1.8,
        "{} → {}",
        before.width,
        after.width
    );
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(field_word(&app).width, before.width);
}

/// The workspace scrollable's bounds in the window.
struct WorkspaceBounds(Option<cosmic::iced::Rectangle>);

impl<T> cosmic::iced::advanced::widget::Operation<T> for WorkspaceBounds {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn cosmic::iced::advanced::widget::Operation<T>),
    ) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        id: Option<&cosmic::iced::widget::Id>,
        bounds: cosmic::iced::Rectangle,
        _content: cosmic::iced::Rectangle,
        _translation: cosmic::iced::Vector,
        _state: &mut dyn cosmic::iced::advanced::widget::operation::Scrollable,
    ) {
        if id == Some(&workspace_scroll_id()) {
            self.0 = Some(bounds);
        }
    }
}

impl Runtime {
    /// The page workspace's bounds in the window, and the page view it
    /// builds there: its width less the scroll bar.
    fn workspace_view(&mut self) -> (cosmic::iced::Rectangle, PageView) {
        let mut ui = cosmic::iced::runtime::user_interface::UserInterface::build(
            cosmic::ApplicationExt::view_main(&self.app),
            self.window,
            self.cache.take().expect("the interface's cache"),
            &mut self.renderer,
        );
        let mut found = WorkspaceBounds(None);
        ui.operate(&self.renderer, &mut found);
        self.cache = Some(ui.into_cache());
        let bounds = found.0.expect("the page workspace");
        let session = &self.app.sessions[self.app.active_session];
        let view = PageView::new(
            session.pages.layout(),
            session.zoom,
            cosmic::iced::Size::new((bounds.width - PAGE_SCROLLBAR).max(1.0), bounds.height),
        );
        (bounds, view)
    }

    /// Where page twips `x` is on the ruler above the workspace, `down`
    /// pixels from its top.
    fn on_ruler(&mut self, x: f32, down: f32) -> cosmic::iced::Point {
        let (bounds, view) = self.workspace_view();
        let page = view.pages[0];
        cosmic::iced::Point::new(
            bounds.x + page.x - self.app.scroll_offset.x + x * view.scale,
            bounds.y - crate::ruler_view::HEIGHT + down,
        )
    }

    /// Where page twips (`x`, `y`) on the first page are in the window.
    fn on_page(&mut self, x: f32, y: f32) -> cosmic::iced::Point {
        let (bounds, view) = self.workspace_view();
        let page = view.pages[0];
        cosmic::iced::Point::new(
            bounds.x + page.x - self.app.scroll_offset.x + x * view.scale,
            bounds.y + page.y - self.app.scroll_offset.y + y * view.scale,
        )
    }

    /// A press at `from`, a move to `to`, and the release there.
    fn drag(&mut self, from: cosmic::iced::Point, to: cosmic::iced::Point) -> Vec<Message> {
        use cosmic::iced::mouse::{Button, Event as Mouse};
        self.events(&[
            (
                cosmic::iced::Event::Mouse(Mouse::CursorMoved { position: from }),
                from,
            ),
            (
                cosmic::iced::Event::Mouse(Mouse::ButtonPressed(Button::Left)),
                from,
            ),
            (
                cosmic::iced::Event::Mouse(Mouse::CursorMoved { position: to }),
                to,
            ),
            (
                cosmic::iced::Event::Mouse(Mouse::ButtonReleased(Button::Left)),
                to,
            ),
        ])
    }
}

#[test]
fn the_ruler_sets_tabs_and_indents_through_the_window_one_undo_step_each() {
    use amipro_document::{TabKind, TabStop};
    use cosmic::iced::mouse::Button;
    // RUB: four Body Text paragraphs on the Standard layout, at 100%.
    let mut app = page_app(Document::from_plain_text(
        "First paragraph of the ruler base.\nSecond one.\nThird one.\nFourth one.",
    ));
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    // The caret in the first paragraph, by a click in its text.
    let text = runtime.on_page(1_500.0, 1_560.0);
    runtime.click(Button::Left, text);
    assert_eq!(runtime.app.page_selection.focus.paragraph, 0);
    let tabs = |runtime: &Runtime| runtime.app.document.paragraphs()[0].format.tabs.clone();
    // RU3: the stop at 2.5 in on the ruler (2160 from the margin) dragged
    // right: on the grid at 2700, one undo step, the guide gone after.
    let from = runtime.on_ruler(3_600.0, 10.0);
    let to = runtime.on_ruler(3_600.0 + 570.0, 10.0);
    let published = runtime.drag(from, to);
    assert!(
        published
            .iter()
            .any(|message| matches!(message, Message::Ruler(ruler_view::RulerEvent::Drag(..)))),
        "{published:?}"
    );
    let stops = tabs(&runtime).expect("a paragraph ruler");
    assert!(
        stops.contains(&TabStop::new(TabKind::Left, 2_700)),
        "{stops:?}"
    );
    assert!(!stops.iter().any(|stop| stop.position == 2_160));
    assert_eq!(runtime.app.ruler_guide, None);
    assert_eq!(runtime.app.history.undo_len(), 1);
    // The caret keeps the keyboard: typing goes on in the paragraph.
    let key = cosmic::iced::keyboard::Key::Character("x".into());
    runtime.events(&[(
        cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: cosmic::iced::keyboard::key::Physical::Unidentified(
                cosmic::iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: cosmic::iced::keyboard::Location::Standard,
            modifiers: cosmic::iced::keyboard::Modifiers::default(),
            text: Some("x".into()),
            repeat: false,
        }),
        cosmic::iced::Point::ORIGIN,
    )]);
    assert!(
        runtime.app.document.paragraphs()[0].text.contains('x'),
        "{:?}",
        runtime.app.document.paragraphs()[0].text
    );
    // The kind button, then a click in the upper half: a right tab where
    // clicked, not snapped (RU1).
    let (workspace, _) = runtime.workspace_view();
    let button = cosmic::iced::Point::new(
        workspace.x + 12.0,
        workspace.y - crate::ruler_view::HEIGHT + 18.0,
    );
    runtime.click(Button::Left, button);
    assert_eq!(runtime.app.ruler_tab.0, TabKind::Right);
    let at = runtime.on_ruler(1_440.0 + 6_300.0, 10.0);
    runtime.click(Button::Left, at);
    let stops = tabs(&runtime).unwrap();
    let added = stops
        .iter()
        .find(|stop| stop.kind == TabKind::Right)
        .expect("a right tab");
    assert!((added.position - 6_300).abs() <= 15, "{added:?}");
    // RU4: a stop dragged down off the ruler is removed.
    let count = stops.len();
    let from = runtime.on_ruler(1_440.0 + 720.0, 10.0);
    let below = cosmic::iced::Point::new(from.x, from.y + 60.0);
    runtime.drag(from, below);
    assert_eq!(tabs(&runtime).unwrap().len(), count - 1);
    // RU5: the first-line indent dragged half an inch.
    let from = runtime.on_ruler(1_440.0, 3.0);
    let to = runtime.on_ruler(1_440.0 + 730.0, 3.0);
    runtime.drag(from, to);
    let indents = runtime.app.document.paragraphs()[0].format.indents.unwrap();
    assert_eq!((indents.all, indents.first, indents.rest), (0, 720, 0));
    // Each gesture was one undo step.
    assert_eq!(runtime.app.history.undo_len(), 5);
    // View › Ruler hides and shows it; the pages keep their place.
    runtime.send(Message::Command(CommandId::Ruler));
    assert!(!runtime.app.preferences.ruler_visible);
    draw_window(&runtime.app);
    runtime.send(Message::Command(CommandId::Ruler));
    assert!(runtime.app.preferences.ruler_visible);
}

#[test]
fn the_rulers_lower_half_moves_columns_with_a_guide_down_the_page_ru2() {
    use cosmic::iced::mouse::{Button, Event as Mouse};
    let mut document = Document::from_plain_text("One\nTwo\nThree");
    let layout = document.document_layout().id;
    let mut settings = amipro_editing::LayoutSettings::of(&document, layout).unwrap();
    settings
        .set_ruler_columns(None, amipro_editing::layout_setup::Pages::All, 2)
        .unwrap();
    document = amipro_editing::apply(
        &document,
        &amipro_editing::Command::ModifyLayout { layout, settings },
    )
    .unwrap()
    .document;
    let mut app = page_app(document);
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    // The gap between 6000 and 6240, taken and moved: the guide follows.
    let from = runtime.on_ruler(6_120.0, 28.0);
    let to = runtime.on_ruler(6_120.0 + 1_100.0, 28.0);
    runtime.events(&[
        (
            cosmic::iced::Event::Mouse(Mouse::CursorMoved { position: from }),
            from,
        ),
        (
            cosmic::iced::Event::Mouse(Mouse::ButtonPressed(Button::Left)),
            from,
        ),
        (
            cosmic::iced::Event::Mouse(Mouse::CursorMoved { position: to }),
            to,
        ),
    ]);
    assert_eq!(runtime.app.ruler_guide, Some((0, 7_290.0)));
    assert!(
        runtime.app.status.contains("Space between columns"),
        "{}",
        runtime.app.status
    );
    draw_window(&runtime.app);
    runtime.events(&[(
        cosmic::iced::Event::Mouse(Mouse::ButtonReleased(Button::Left)),
        to,
    )]);
    assert_eq!(runtime.app.ruler_guide, None);
    let page = &runtime.app.document.document_layout().right.layout;
    let edges: Vec<_> = page
        .column_edges
        .as_ref()
        .expect("uneven")
        .iter()
        .map(|edges| (edges.left + 1_440, edges.right + 1_440))
        .collect();
    assert_eq!(edges, [(1_440, 7_050), (7_290, 10_800)]);
    assert_eq!(runtime.app.history.undo_len(), 1);
    // The pages follow at once: the second column starts at 7290.
    let columns = &runtime.app.pages.layout().pages[0].columns;
    assert_eq!((columns[1].x, columns[1].width), (7_290.0, 3_510.0));
}

/// The ruler (`UI_SPEC.md` §19) light and dark: a paragraph with its own
/// ruler (each kind of tab, one with a leader) and indents, in uneven
/// columns, and the guide a drag draws down the page; into
/// `AMIPRO_SCREENSHOT_DIR` as for `window_screenshots`.
#[test]
fn ruler_screenshots() {
    use amipro_document::{Indents, TabKind, TabLeader, TabStop};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut document = page_fixture();
    document.document_layout_mut().right.layout.set_edges(
        vec![
            amipro_document::ColumnEdges {
                left: 0,
                right: 5_610,
            },
            amipro_document::ColumnEdges {
                left: 5_850,
                right: 9_360,
            },
        ],
        9_360,
    );
    let format = &mut document.paragraph_mut(0).format;
    format.tabs = Some(vec![
        TabStop::new(TabKind::Left, 720),
        TabStop::new(TabKind::Centre, 1_800),
        TabStop {
            kind: TabKind::Right,
            position: 3_240,
            leader: TabLeader::Dots,
        },
        TabStop::new(TabKind::Decimal, 4_320),
    ]);
    format.indents = Some(Indents {
        all: 360,
        first: 360,
        rest: 0,
        right: 360,
    });
    let mut app = page_app(document);
    drop(app.update_title());
    send(&mut app, Message::ZoomTo(100));
    let write = |app: &AmiProApp, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    write(&app, "window-ruler-light", false);
    write(&app, "window-ruler-dark", true);
    app.ruler_guide = Some((0, 7_290.0));
    write(&app, "window-ruler-guide-light", false);
}

#[test]
fn the_rulers_right_click_menus_act_on_what_was_pressed() {
    use amipro_document::TabKind;
    use cosmic::iced::mouse::Button;
    let mut app = page_app(Document::from_plain_text(
        "First paragraph of the ruler base.\nSecond one.",
    ));
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let text = runtime.on_page(1_500.0, 1_560.0);
    runtime.click(Button::Left, text);
    let ruler_message = |messages: Vec<Message>| {
        messages
            .into_iter()
            .filter_map(|message| match message {
                Message::RulerMenu(action) => Some(action),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    // On the stop at 1.5 in on the ruler (720 from the margin): Right Tab.
    let tab = runtime.on_ruler(2_160.0, 10.0);
    runtime.click(Button::Right, tab);
    assert_eq!(
        runtime.app.ruler_context,
        Some((Some(crate::ruler::Marker::Tab(0)), ruler_view::Part::Upper))
    );
    assert_eq!(
        ruler_message(runtime.click(Button::Left, menu_row(tab, 1, 0))),
        [ruler_view::RulerMenu::TabKind(TabKind::Right)]
    );
    let stops = runtime.app.document.paragraphs()[0]
        .format
        .tabs
        .clone()
        .unwrap();
    assert_eq!((stops[0].kind, stops[0].position), (TabKind::Right, 720));
    // Remove Tab, after the kinds, the leaders, and two dividers.
    runtime.click(Button::Right, tab);
    assert_eq!(
        ruler_message(runtime.click(Button::Left, menu_row(tab, 8, 2))),
        [ruler_view::RulerMenu::RemoveTab]
    );
    assert_eq!(
        runtime.app.document.paragraphs()[0]
            .format
            .tabs
            .as_ref()
            .unwrap()
            .len(),
        11
    );
    // Between the stops: Clear Tabs leaves a ruler without stops.
    let empty = runtime.on_ruler(2_160.0 + 360.0, 10.0);
    runtime.click(Button::Right, empty);
    assert_eq!(
        ruler_message(runtime.click(Button::Left, menu_row(empty, 0, 0))),
        [ruler_view::RulerMenu::ClearTabs]
    );
    assert_eq!(
        runtime.app.document.paragraphs()[0].format.tabs,
        Some(Vec::new())
    );
    assert_eq!(runtime.app.history.undo_len(), 3);
    // The kind button's menu: Decimal tabs from now on.
    let (workspace, _) = runtime.workspace_view();
    let button =
        cosmic::iced::Point::new(workspace.x + 12.0, workspace.y - ruler_view::HEIGHT + 18.0);
    runtime.click(Button::Right, button);
    runtime.click(Button::Left, menu_row(button, 3, 0));
    assert_eq!(runtime.app.ruler_tab.0, TabKind::Decimal);
    // The lower half: Page Layout… opens Page Layout Properties.
    let lower = runtime.on_ruler(4_000.0, 28.0);
    runtime.click(Button::Right, lower);
    assert_eq!(
        ruler_message(runtime.click(Button::Left, menu_row(lower, 2, 1))),
        [ruler_view::RulerMenu::BoxSettings]
    );
    assert!(runtime.app.page_layout_properties.is_some());
}

/// A key press as the window delivers it.
fn key_press(
    key: cosmic::iced::keyboard::Key,
    modifiers: cosmic::iced::keyboard::Modifiers,
) -> (cosmic::iced::Event, cosmic::iced::Point) {
    let text = match &key {
        cosmic::iced::keyboard::Key::Character(text) => Some(text.clone()),
        _ => None,
    };
    (
        cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: cosmic::iced::keyboard::key::Physical::Unidentified(
                cosmic::iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: cosmic::iced::keyboard::Location::Standard,
            modifiers,
            text,
            repeat: false,
        }),
        cosmic::iced::Point::ORIGIN,
    )
}

#[test]
fn typing_past_the_end_of_a_header_goes_on_and_says_it_is_hidden() {
    // User, 2026-10-18: in a new document, Numbering, "this is a test",
    // Enter, and typing seemed to stop. The click had put the caret in the
    // header band, which then held one line of Body Text; the second line
    // was hidden, as Ami Pro hides it (V14), and the caret went with it.
    // The band holds two since new documents' Body Text has no space below
    // (as `_DEFAULT.STY`), so here the third line is hidden.
    use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
    use cosmic::iced::mouse::Button;
    let mut app = page_app(Document::default());
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let band = runtime.on_page(1_500.0, 900.0);
    runtime.click(Button::Left, band);
    let header = runtime.app.document.document_layout().header.right.unwrap();
    let header_flow = flow_of(&runtime.app.document, header);
    assert_eq!(runtime.app.page_selection.focus.flow, header_flow);
    let plain = Modifiers::default();
    let type_text = |runtime: &mut Runtime, text: &str| {
        for character in text.chars() {
            runtime.events(&[key_press(
                Key::Character(character.to_string().into()),
                plain,
            )]);
        }
    };
    runtime.events(&[key_press(
        Key::Character("n".into()),
        Modifiers::CTRL | Modifiers::SHIFT,
    )]);
    type_text(&mut runtime, "this is a test");
    let note = |runtime: &Runtime| {
        page_surface::hidden_text_note(
            &runtime.app.document,
            runtime.app.pages.layout(),
            runtime.app.page_selection,
        )
    };
    assert_eq!(note(&runtime), None);
    runtime.events(&[key_press(Key::Named(Named::Enter), plain)]);
    type_text(&mut runtime, "more");
    assert_eq!(note(&runtime), None, "two lines fit, as in Ami Pro (HDR1)");
    runtime.events(&[key_press(Key::Named(Named::Enter), plain)]);
    type_text(&mut runtime, "again");
    let paragraphs = &runtime.app.document.flow(header_flow).unwrap().paragraphs;
    let texts: Vec<_> = paragraphs
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .collect();
    assert_eq!(
        texts,
        ["this is a test", "more", "again"],
        "the typing went on"
    );
    assert!(note(&runtime).is_some_and(|note| note.contains("header")));
    // The style field and the Styles panel follow the caret into the
    // header.
    let number_list = paragraphs[2].style;
    assert_eq!(
        runtime.app.document.styles()[&number_list].name,
        "Number List"
    );
    assert_eq!(runtime.app.selected_style(), Some(number_list));
    draw_window(&runtime.app);
}

#[test]
fn edit_go_to_ruler_gives_the_ruler_the_keyboard_as_ami_pro_does() {
    use amipro_document::{TabKind, TabStop};
    use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
    use cosmic::iced::mouse::Button;
    let mut app = page_app(Document::from_plain_text(
        "First paragraph of the ruler base.\nSecond one.",
    ));
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let text = runtime.on_page(1_500.0, 1_560.0);
    runtime.click(Button::Left, text);
    runtime.send(Message::Command(CommandId::GoToRuler));
    assert_eq!(runtime.app.ruler_cursor, Some(1_440.0));
    let plain = Modifiers::default();
    let character = |text: &str| key_press(Key::Character(text.into()), plain);
    let right = key_press(Key::Named(Named::ArrowRight), plain);
    // Four steps of 1/16 in, then Space: a tab at 0.25 in.
    runtime.events(&[right.clone(), right.clone(), right.clone(), right.clone()]);
    assert_eq!(runtime.app.ruler_cursor, Some(1_800.0));
    runtime.events(&[character(" ")]);
    let tabs = runtime.app.document.paragraphs()[0]
        .format
        .tabs
        .clone()
        .unwrap();
    assert!(tabs.contains(&TabStop::new(TabKind::Left, 360)), "{tabs:?}");
    assert_eq!(
        runtime.app.document.paragraphs()[0].text,
        "First paragraph of the ruler base.",
        "the space went to the ruler, not the text"
    );
    // Del removes the tab at the line.
    runtime.events(&[key_press(Key::Named(Named::Delete), plain)]);
    let tabs = runtime.app.document.paragraphs()[0]
        .format
        .tabs
        .clone()
        .unwrap();
    assert!(!tabs.iter().any(|stop| stop.position == 360));
    assert_eq!(
        runtime.app.document.paragraphs()[0].text,
        "First paragraph of the ruler base."
    );
    // Ctrl+Right steps half an inch; 1 sets the first-line indent there.
    runtime.events(&[key_press(Key::Named(Named::ArrowRight), Modifiers::CTRL)]);
    assert_eq!(runtime.app.ruler_cursor, Some(2_520.0));
    runtime.events(&[character("1")]);
    let indents = runtime.app.document.paragraphs()[0].format.indents.unwrap();
    assert_eq!((indents.all, indents.first), (0, 1_080));
    // ] sets the right margin at the line.
    for _ in 0..11 {
        runtime.events(&[key_press(Key::Named(Named::ArrowRight), Modifiers::CTRL)]);
    }
    assert_eq!(runtime.app.ruler_cursor, Some(10_440.0));
    runtime.events(&[character("]")]);
    assert_eq!(
        runtime
            .app
            .document
            .document_layout()
            .right
            .layout
            .margins
            .right,
        12_240 - 10_440
    );
    // Esc returns the keyboard to the text.
    runtime.events(&[key_press(Key::Named(Named::Escape), plain)]);
    assert_eq!(runtime.app.ruler_cursor, None);
    runtime.events(&[character("x")]);
    assert!(runtime.app.document.paragraphs()[0].text.contains('x'));
}

/// A tab under an indent marker (the first line dragged onto the stop at
/// 0.5 in, as step 3 of the physical check leaves it): its right-click
/// menu is still the tab's, and Centre changes it; a press on its foot
/// drags the tab, one on the triangle the indent. (Found in the user's
/// physical check: the menu was the ruler's general one.)
#[test]
fn a_tab_under_an_indent_marker_keeps_its_menu_and_its_drag() {
    use amipro_document::{Indents, TabKind};
    use cosmic::iced::mouse::Button;
    let mut document = Document::from_plain_text("First paragraph of the ruler base.\nSecond one.");
    document.paragraph_mut(0).format.indents = Some(Indents {
        all: 0,
        first: 720,
        rest: 0,
        right: 0,
    });
    let mut app = page_app(document);
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let text = runtime.on_page(1_500.0, 1_560.0);
    runtime.click(Button::Left, text);
    // The stop's foot, below the triangle.
    let tab = runtime.on_ruler(2_160.0, 15.0);
    runtime.click(Button::Right, tab);
    assert_eq!(
        runtime.app.ruler_context,
        Some((Some(crate::ruler::Marker::Tab(0)), ruler_view::Part::Upper))
    );
    // Centre Tab, the third kind.
    runtime.click(Button::Left, menu_row(tab, 2, 0));
    let stops = |runtime: &Runtime| {
        runtime.app.document.paragraphs()[0]
            .format
            .tabs
            .clone()
            .unwrap()
    };
    assert_eq!(
        (stops(&runtime)[0].kind, stops(&runtime)[0].position),
        (TabKind::Centre, 720)
    );
    // A drag from the foot moves the tab; from the triangle, the indent.
    let to = runtime.on_ruler(2_160.0 + 360.0, 15.0);
    runtime.drag(tab, to);
    assert_eq!(stops(&runtime)[0].position, 1_080);
    let triangle = runtime.on_ruler(2_160.0, 4.0);
    let to = runtime.on_ruler(2_160.0 + 720.0, 4.0);
    runtime.drag(triangle, to);
    assert_eq!(
        runtime.app.document.paragraphs()[0]
            .format
            .indents
            .unwrap()
            .first,
        1_440
    );
}

#[test]
fn insert_frame_makes_ami_pros_frame_on_the_page_in_view_fc1_fc9() {
    use crate::page_dialogs::{CreateFrameAction, GeometryField};
    use amipro_document::{Placement, Rect};
    use amipro_editing::frame_creation::{FrameDefaults, FrameGeometry};
    let mut app = page_app(Document::from_plain_text(&["Line."; 120].join("\n")));
    assert!(app.page_count() >= 2);
    let caret = app.page_selection;
    // Page 2 in view, the caret on page 1 (FC9).
    let page_two = app.page_view().pages[1];
    app.sessions[0].scroll_offset.y = page_two.y;
    send(&mut app, Message::Command(CommandId::Frame));
    let dialog = app.create_frame_dialog.clone().expect("the dialog");
    assert_eq!(dialog.page, 2);
    assert_eq!(
        (
            dialog.width.as_str(),
            dialog.top.as_str(),
            dialog.left.as_str()
        ),
        ("2.00", "4.50", "3.25"),
        "Ami Pro's first values"
    );
    // FC1: 2.50 × 1.00 in at 3.00 in down, 1.25 in in.
    for (field, text) in [
        (GeometryField::Width, "2.50"),
        (GeometryField::Height, "1.00"),
        (GeometryField::Top, "3.00"),
        (GeometryField::Left, "1.25"),
    ] {
        send(
            &mut app,
            Message::CreateFrame(CreateFrameAction::Input(field, text.into())),
        );
    }
    send(&mut app, Message::CreateFrame(CreateFrameAction::Create));
    assert!(app.create_frame_dialog.is_none());
    let frame_id = *app.document.frame_stack().last().unwrap();
    let rect = Rect::new(1_800, 4_320, 5_400, 5_760);
    let frame = app.document.frame(frame_id).unwrap();
    assert_eq!(frame.placement, Placement::Fixed { page: 2, rect });
    assert_eq!(frame.style, FrameDefaults::AMI_PRO.frame_style(rect));
    assert!(matches!(frame.content, FrameContent::Text(_)));
    // Selected, the caret where it was; the frame laid out on page 2.
    assert_eq!(
        app.frame_selection,
        Some(frame_editing::FrameSelection::one(frame_id, 2))
    );
    assert_eq!(app.page_selection, caret);
    assert!(
        app.pages.layout().pages[1]
            .frames
            .iter()
            .any(|frame_box| frame_box.frame == frame_id)
    );
    assert_eq!(
        app.preferences.last_frame,
        Some(FrameGeometry {
            width: 3_600,
            height: 1_440,
            top: 4_320,
            left: 1_800
        })
    );
    // The dialog offers it next time.
    send(&mut app, Message::Command(CommandId::Frame));
    assert_eq!(app.create_frame_dialog.as_ref().unwrap().width, "2.50");
    send(&mut app, Message::CreateFrame(CreateFrameAction::Cancel));
    assert!(app.create_frame_dialog.is_none());
    // One undo step.
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(app.document.frame_stack().is_empty());
    // Draw on Page starts drawing; so does Insert › Draw Frame.
    send(&mut app, Message::Command(CommandId::Frame));
    send(&mut app, Message::CreateFrame(CreateFrameAction::Draw));
    assert!(app.create_frame_dialog.is_none());
    assert!(app.drawing_frame);
}

/// Screenshots of creating frames (`UI_SPEC.md` §20), light and dark, when
/// `AMIPRO_SCREENSHOT_DIR` is set.
#[test]
fn frame_creation_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    let write = |app: &AmiProApp, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    send(&mut app, Message::Command(CommandId::Frame));
    write(&app, "window-createframe-light", false);
    write(&app, "window-createframe-dark", true);
    // The new frame, selected, with Frame Properties down to its New frames
    // section (a tall window shows the whole rail).
    send(
        &mut app,
        Message::CreateFrame(crate::page_dialogs::CreateFrameAction::Create),
    );
    send(&mut app, Message::Command(CommandId::StylesPanel));
    let tall = |app: &AmiProApp, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 2_300);
        let rgb = render_window(app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    tall(&app, "window-newframe-light", false);
    tall(&app, "window-newframe-dark", true);
}

#[test]
fn draw_frame_draws_ami_pros_frame_through_the_window_fc2_fc4() {
    use amipro_document::Placement;
    use amipro_editing::frame_creation::FrameDefaults;
    use cosmic::iced::mouse::Button;
    let escape = || {
        let key = cosmic::iced::keyboard::Key::Named(cosmic::iced::keyboard::key::Named::Escape);
        cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: cosmic::iced::keyboard::key::Physical::Unidentified(
                cosmic::iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: cosmic::iced::keyboard::Location::Standard,
            modifiers: cosmic::iced::keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        })
    };
    let fixed = |app: &AmiProApp, frame| match app.document.frame(frame).unwrap().placement {
        Placement::Fixed { page, rect } => (page, rect),
        other => panic!("{other:?}"),
    };
    let mut app = page_app(Document::from_plain_text(
        "Main text paragraph one, long enough to wrap around a frame.\nSecond main paragraph.",
    ));
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let text = runtime.on_page(1_600.0, 1_560.0);
    runtime.click(Button::Left, text);
    let caret = runtime.app.page_selection;

    // FC2: dragged from bottom right to top left, to the twip of the drag
    // (a pixel is 15 twips at 100%).
    runtime.send(Message::Command(CommandId::DrawFrame));
    assert!(runtime.app.drawing_frame);
    let from = runtime.on_page(6_656.0, 7_218.0);
    let to = runtime.on_page(4_184.0, 5_076.0);
    runtime.drag(from, to);
    assert!(!runtime.app.drawing_frame);
    let frame = *runtime.app.document.frame_stack().last().unwrap();
    let (page, rect) = fixed(&runtime.app, frame);
    assert_eq!(page, 1);
    for (got, want) in [
        (rect.left, 4_184),
        (rect.top, 5_076),
        (rect.right, 6_656),
        (rect.bottom, 7_218),
    ] {
        assert!((got - want).abs() <= 15, "{rect:?}");
    }
    assert_eq!(
        runtime.app.document.frame(frame).unwrap().style,
        FrameDefaults::AMI_PRO.frame_style(rect)
    );
    assert_eq!(
        runtime.app.frame_selection,
        Some(frame_editing::FrameSelection::one(frame, 1))
    );
    assert_eq!(runtime.app.page_selection, caret, "the caret stays");
    assert_eq!(runtime.app.history.undo_len(), 1, "one undo step");

    // FC4: a click draws nothing and ends drawing; so does Esc (FC11).
    runtime.send(Message::Command(CommandId::DrawFrame));
    let empty = runtime.on_page(9_000.0, 3_000.0);
    runtime.click(Button::Left, empty);
    assert!(!runtime.app.drawing_frame);
    assert_eq!(runtime.app.document.frame_stack().len(), 1);
    runtime.send(Message::Command(CommandId::DrawFrame));
    runtime.events(&[(escape(), cosmic::iced::Point::ORIGIN)]);
    assert!(!runtime.app.drawing_frame);
}

#[test]
fn a_frame_drawn_on_a_scrolled_zoomed_page_stays_on_its_paper_over_other_frames() {
    use amipro_document::Placement;
    // Six pages at 50%; the view scrolled to page 2 by the page arrow.
    let mut app = page_app(page_fixture());
    send(&mut app, Message::ZoomTo(50));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    runtime.send(Message::Command(CommandId::NextPage));
    runtime.redraw();
    let scrolled = runtime.workspace_scroll();
    assert!(scrolled > 0.0);
    let on_page_two = |runtime: &mut Runtime, x: f32, y: f32| {
        let (bounds, view) = runtime.workspace_view();
        let page = view.pages[1];
        cosmic::iced::Point::new(
            bounds.x + page.x - runtime.app.scroll_offset.x + x * view.scale,
            bounds.y + page.y - scrolled + y * view.scale,
        )
    };
    let frames_before = runtime.app.document.frame_stack().to_vec();
    runtime.send(Message::Command(CommandId::DrawFrame));
    // Past the paper's right edge: the corner stops there.
    let from = on_page_two(&mut runtime, 5_000.0, 6_000.0);
    let past = on_page_two(&mut runtime, 14_000.0, 6_500.0);
    runtime.drag(from, past);
    let stack = runtime.app.document.frame_stack().to_vec();
    assert_eq!(stack.len(), frames_before.len() + 1);
    let drawn = *stack.last().unwrap();
    let Placement::Fixed { page, rect } = runtime.app.document.frame(drawn).unwrap().placement
    else {
        panic!("a Where placed frame");
    };
    assert_eq!(page, 2);
    assert_eq!(rect.right, 12_240, "the paper's right edge");
    // A pixel is 30 twips at 50%.
    assert!(
        (rect.left - 5_000).abs() <= 30 && (rect.bottom - 6_500).abs() <= 30,
        "{rect:?}"
    );
    // Drawn above the page's other Where placed frames.
    let order: Vec<_> = runtime.app.pages.layout().pages[1]
        .frames
        .iter()
        .map(|frame_box| frame_box.frame)
        .collect();
    assert_eq!(order.last(), Some(&drawn));
}

#[test]
fn make_default_for_new_frames_keeps_what_ami_pro_keeps_fc6n() {
    use crate::page_dialogs::CreateFrameAction;
    use amipro_document::{Margins, Placement, Shadow, Sides, Wrap};
    use amipro_editing::frame_creation::FrameDefaults;
    let mut app = page_app(Document::from_plain_text("One.\nTwo."));
    send(&mut app, Message::Command(CommandId::Frame));
    send(&mut app, Message::CreateFrame(CreateFrameAction::Create));
    let frame = *app.document.frame_stack().last().unwrap();
    // FC6: No wrap around, Transparent, square, a top line of style 3, no
    // shadow, 0.20 in margins, four columns.
    let mut style = app.document.frame(frame).unwrap().style.clone();
    style.wrap = Wrap::NoWrapAround;
    style.opaque = false;
    style.corners = 0;
    style.border.sides = Sides {
        top: true,
        ..Sides::NONE
    };
    style.border.style = 3;
    style.shadow = Shadow::default();
    style.interior.margins = Margins::uniform(288);
    style.interior.columns = 4;
    style.interior.tabs = vec![Vec::new(); 4];
    assert!(
        app.execute_page_command(&amipro_editing::Command::SetFrameStyle {
            frame,
            style: style.clone()
        })
    );
    send(&mut app, Message::NewFrameDefault(NewFrameDefault::Make));
    assert_eq!(
        app.preferences.frame_defaults,
        FrameDefaults::from_style(&style)
    );
    // FC6N: the next frame takes all of it but the columns, Where placed.
    send(&mut app, Message::Command(CommandId::Frame));
    send(&mut app, Message::CreateFrame(CreateFrameAction::Create));
    let next = *app.document.frame_stack().last().unwrap();
    assert_ne!(next, frame);
    let made = app.document.frame(next).unwrap();
    assert!(matches!(made.placement, Placement::Fixed { .. }));
    assert_eq!(made.style.wrap, Wrap::NoWrapAround);
    assert!(!made.style.opaque);
    assert_eq!(made.style.border.sides, style.border.sides);
    assert_eq!(made.style.border.style, 3);
    assert_eq!(made.style.shadow, Shadow::default());
    assert_eq!(made.style.corners, 0);
    assert_eq!(made.style.interior.margins, Margins::uniform(288));
    assert_eq!(made.style.interior.columns, 1);
    // Restore Ami Pro's Default.
    send(&mut app, Message::NewFrameDefault(NewFrameDefault::Restore));
    assert_eq!(app.preferences.frame_defaults, FrameDefaults::AMI_PRO);
}

/// An opaque frame hides the text behind it, main text and other frames'
/// text alike, as in Ami Pro (user's guide ch. 15); a transparent one shows
/// it. The renderer draws a layer's quads before its text, so each page
/// frame has a layer of its own.
#[test]
fn an_opaque_frame_hides_the_text_behind_it() {
    use amipro_document::{Placement, Rect, Wrap};
    use amipro_editing::frame_creation::FrameDefaults;
    let mut app = page_app(Document::from_plain_text(
        &["Text behind the frame, line after line of it, all the way across the page."; 30]
            .join("\n"),
    ));
    send(&mut app, Message::ZoomTo(100));
    let rect = Rect::new(2_880, 2_880, 7_200, 5_760);
    let mut style = FrameDefaults::AMI_PRO.frame_style(rect);
    style.wrap = Wrap::NoWrapAround;
    style.shadow = amipro_document::Shadow::default();
    style.corners = 0;
    assert!(
        app.execute_page_command(&amipro_editing::Command::CreateFrame {
            placement: Placement::Fixed { page: 1, rect },
            style: style.clone(),
            content: amipro_editing::NewFrameContent::Text,
            anchor: None,
            layout: None,
        })
    );
    let frame = *app.document.frame_stack().last().unwrap();
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let (bounds, view) = runtime.workspace_view();
    let page = view.pages[0];
    let inside = |x: f32, y: f32| {
        (
            (bounds.x + page.x + x * view.scale) as usize,
            (bounds.y + page.y + y * view.scale) as usize,
        )
    };
    let (left, top) = inside(3_100.0, 3_100.0);
    let (right, bottom) = inside(7_000.0, 5_500.0);
    let dark = |app: &AmiProApp| {
        let rgb = render_window(app, (1_280, 800), &cosmic::Theme::light(), false);
        (top..bottom)
            .flat_map(|y| (left..right).map(move |x| (x, y)))
            .filter(|(x, y)| {
                let at = (y * 1_280 + x) * 3;
                rgb[at..at + 3].iter().all(|channel| *channel < 128)
            })
            .count()
    };
    assert_eq!(dark(&runtime.app), 0, "the opaque frame hides the text");
    style.opaque = false;
    assert!(
        runtime
            .app
            .execute_page_command(&amipro_editing::Command::SetFrameStyle { frame, style })
    );
    runtime.redraw();
    assert!(dark(&runtime.app) > 100, "a transparent frame shows it");
}

/// Each top-level `[frm]` record of an Ami Pro document, with what it
/// holds (its `[frmlay]`, `[txt]`, …) but `[frmlay]` values 12 to 14, which
/// are stale memory (`SAM_FORMAT_NOTES.md`: FC1 wrote `0 0 0`, FC2 in the
/// same session `114 100 120`), and the body without paragraph caches
/// (`<:#…>`, which Ami Pro writes once it has laid a paragraph out: FCB has
/// them, FC1 not): what the frame twins compare.
fn frames_and_body(bytes: &[u8]) -> (Vec<String>, String) {
    let text =
        String::from_utf8_lossy(bytes.split(|byte| *byte == 0).next().unwrap_or(&[])).into_owned();
    let mut frames = Vec::new();
    let mut current: Option<String> = None;
    let mut body = String::new();
    let mut in_body = false;
    // The place of a line in the current `[frmlay]`, from 1.
    let mut layout_value: Option<usize> = None;
    for line in text.split("\r\n") {
        if in_body {
            let mut rest = line;
            while let Some(start) = rest.find("<:#") {
                body.push_str(&rest[..start]);
                rest = rest[start..]
                    .find('>')
                    .map_or("", |end| &rest[start + end + 1..]);
            }
            body.push_str(rest);
            body.push('\n');
            if line == ">" {
                in_body = false;
            }
            continue;
        }
        if line.starts_with('[') {
            if let Some(frame) = current.take() {
                frames.push(frame);
            }
            if line == "[frm]" {
                current = Some(String::new());
            } else if line == "[edoc]" {
                in_body = true;
            }
            continue;
        }
        if let Some(frame) = current.as_mut() {
            if line == "\t[frmlay]" {
                layout_value = Some(0);
            } else if let Some(index) = layout_value.as_mut() {
                *index += 1;
                if (12..=14).contains(index) {
                    frame.push_str("\t\t(stale)\n");
                    continue;
                }
                if !line.starts_with("\t\t") {
                    layout_value = None;
                }
            }
            frame.push_str(line);
            frame.push('\n');
        }
    }
    (frames, body)
}

/// The frame creation twins (`docs/FRAME_CREATION_PROGRESS.md`, FC.7): each
/// VM fixture's frame made again in the app, from Ami Pro's document
/// without it, and written as an Ami Pro document: its frame records and
/// body equal to Ami Pro's. With `AMIPRO_TWIN_OUT`, the twins are written
/// there as `AFCn.SAM` for Ami Pro to open.
#[test]
#[ignore = "needs the private VM fixtures (AMIPRO_VM_LAB)"]
fn frame_twins() {
    use crate::page_dialogs::{CreateFrameAction, GeometryField};
    use amipro_document::{Placement, Rect};
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let out = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from);
    let open = |name: &str| {
        page_app(
            amipro_formats::import_sam_path(lab.join(format!("{name}.SAM")))
                .unwrap()
                .document,
        )
    };
    let dialog = |app: &mut AmiProApp, values: [&str; 4]| {
        send(app, Message::Command(CommandId::Frame));
        for (field, text) in [
            GeometryField::Width,
            GeometryField::Height,
            GeometryField::Top,
            GeometryField::Left,
        ]
        .into_iter()
        .zip(values)
        {
            send(
                app,
                Message::CreateFrame(CreateFrameAction::Input(field, text.into())),
            );
        }
        send(app, Message::CreateFrame(CreateFrameAction::Create));
        assert!(app.create_frame_dialog.is_none(), "refused: {values:?}");
    };
    let mut cases: Vec<(&str, AmiProApp)> = Vec::new();
    // FC1: Create Frame, 2.50 × 1.00 in at 3.00 in down, 1.25 in in.
    let mut fc1 = open("FCB");
    dialog(&mut fc1, ["2.50", "1.00", "3.00", "1.25"]);
    cases.push(("FC1", fc1));
    // FC2 and FC3: frames drawn over FC1 and FC2, at Ami Pro's rectangles.
    let mut fc2 = open("FC1");
    fc2.create_frame(1, Rect::new(4_184, 5_076, 6_656, 7_218));
    cases.push(("FC2", fc2));
    let mut fc3 = open("FC2");
    fc3.create_frame(1, Rect::new(8_303, 8_701, 8_353, 8_734));
    cases.push(("FC3", fc3));
    // FC6N: Make Default on FC6's changed (anchored) frame, then a frame.
    let mut fc6n = open("FC6");
    let changed = fc6n
        .document
        .frame_stack()
        .iter()
        .copied()
        .find(|id| {
            fc6n.document
                .frame(*id)
                .is_some_and(|frame| matches!(frame.placement, Placement::BelowParagraph { .. }))
        })
        .expect("FC6's changed frame");
    fc6n.sessions[0].frame_selection = Some(frame_editing::FrameSelection::one(changed, 1));
    send(&mut fc6n, Message::NewFrameDefault(NewFrameDefault::Make));
    dialog(&mut fc6n, ["2.00", "1.50", "7.50", "1.00"]);
    cases.push(("FC6N", fc6n));
    // FC8 as read: the Where placed frame written before the anchored one.
    cases.push(("FC8", open("FC8")));
    // FC9: page 2 in view, the caret on page 1.
    let mut fc9 = open("FC9B");
    let page_two = fc9.page_view().pages[1];
    fc9.sessions[0].scroll_offset.y = page_two.y;
    dialog(&mut fc9, ["2.00", "1.50", "2.00", "1.50"]);
    cases.push(("FC9", fc9));

    let options = amipro_formats::SamOptions::at(0);
    // README31 with a page break before its SmartIcon table (as EDK1, Phase
    // 4.11): its pictures go to the next page with the table, in their
    // cells, here and in Ami Pro.
    let mut readme = open("README31");
    let main = readme.document.main_flow_id();
    let anchor = readme
        .document
        .paragraphs()
        .iter()
        .enumerate()
        .filter(|(_, paragraph)| {
            paragraph.objects.iter().any(|object| {
                object.anchored_frame().is_some_and(|frame| {
                    readme
                        .document
                        .frame(frame)
                        .is_some_and(|frame| !frame.owned_frames().is_empty())
                })
            })
        })
        .map(|(index, _)| index)
        .next()
        .unwrap();
    let end = readme.document.paragraphs()[anchor - 1].text.len();
    move_to(&mut readme, at(main, anchor - 1, end), 1, false);
    let pages_of = |app: &AmiProApp| {
        let layout = app.pages.layout();
        let page_of = |frame: FrameId| {
            layout
                .pages
                .iter()
                .find(|page| page.frames.iter().any(|frame_box| frame_box.frame == frame))
                .map(|page| page.number)
        };
        app.document
            .frames()
            .filter(|frame| !frame.owned_frames().is_empty())
            .flat_map(|table| {
                let table_page = page_of(table.id);
                table
                    .owned_frames()
                    .into_iter()
                    .map(move |inner| (table_page, inner))
            })
            .map(|(table_page, inner)| (table_page, page_of(inner)))
            .collect::<Vec<_>>()
    };
    let before = pages_of(&readme);
    send(&mut readme, Message::Command(CommandId::PageBreak));
    let after = pages_of(&readme);
    assert_eq!(after.len(), 3);
    for (table_page, picture_page) in &after {
        assert_eq!(table_page, picture_page, "a picture on its table's page");
    }
    assert_eq!(
        after[0].0.unwrap(),
        before[0].0.unwrap() + 1,
        "{before:?} {after:?}"
    );
    if let Some(out) = &out {
        std::fs::write(
            out.join("ARM31.SAM"),
            amipro_formats::sam_document(&readme.document, &options),
        )
        .unwrap();
    }

    let mut different = Vec::new();
    for (name, app) in &cases {
        let written = amipro_formats::sam_document(&app.document, &options);
        if let Some(out) = &out {
            std::fs::write(out.join(format!("A{name}.SAM")), &written).unwrap();
        }
        let ours = frames_and_body(&written);
        let theirs = frames_and_body(&std::fs::read(lab.join(format!("{name}.SAM"))).unwrap());
        if ours == theirs {
            eprintln!("{name}: the same");
            continue;
        }
        if ours.1 != theirs.1 {
            eprintln!(
                "{name}: body differs\nours:\n{}\nAmi Pro's:\n{}",
                ours.1, theirs.1
            );
        }
        if ours.0 != theirs.0 {
            eprintln!(
                "{name}: frames differ\nours:\n{}\nAmi Pro's:\n{}",
                ours.0.join("--\n"),
                theirs.0.join("--\n")
            );
        }
        different.push(*name);
    }
    assert!(different.is_empty(), "differ: {different:?}");
}

/// Frames made in the app, by the dialog, by drawing, and after Make
/// Default, one typed in, read back as made from `.amipro` and from an Ami
/// Pro document, with nothing in Save As's loss check (FC.8).
#[test]
fn created_frames_read_back_in_both_formats() {
    use crate::page_dialogs::CreateFrameAction;
    use amipro_document::Rect;
    use amipro_formats::roundtrip::{differences, is_writer_addition};
    let mut app = page_app(Document::from_plain_text(
        "Main text paragraph one, long enough to wrap around a frame.\nSecond main paragraph.",
    ));
    send(&mut app, Message::Command(CommandId::Frame));
    send(&mut app, Message::CreateFrame(CreateFrameAction::Create));
    let first = *app.document.frame_stack().last().unwrap();
    app.create_frame(1, Rect::new(4_184, 5_076, 6_656, 7_218));
    app.create_frame(1, Rect::new(8_303, 8_701, 8_353, 8_734));
    // A changed frame made the default, then a frame with it.
    let mut style = app.document.frame(first).unwrap().style.clone();
    style.opaque = false;
    style.corners = 0;
    style.interior.margins = amipro_document::Margins::uniform(288);
    assert!(
        app.execute_page_command(&amipro_editing::Command::SetFrameStyle {
            frame: first,
            style
        })
    );
    app.sessions[0].frame_selection = Some(frame_editing::FrameSelection::one(first, 1));
    send(&mut app, Message::NewFrameDefault(NewFrameDefault::Make));
    app.create_frame(1, Rect::new(1_440, 10_800, 4_320, 12_960));
    // Text typed in the drawn frame.
    let drawn = app.document.frame_stack()[1];
    let flow = flow_of(&app.document, drawn);
    assert!(
        app.execute_page_command(&amipro_editing::Command::ReplaceText {
            start: at(flow, 0, 0),
            end: at(flow, 0, 0),
            text: "Frame text here.".into(),
        })
    );
    let document = app.document.clone();
    assert_eq!(document.frame_stack().len(), 4);

    let encoded = document.encode_native().unwrap();
    assert_eq!(Document::decode_native(&encoded).unwrap(), document);

    let options = amipro_formats::SamOptions::at(0);
    let written = amipro_formats::sam_document(&document, &options);
    let copy = amipro_formats::import_sam(&written, None, "Created").unwrap();
    let found: Vec<_> = differences(&document, &copy.document)
        .into_iter()
        .filter(|difference| {
            !is_writer_addition(difference)
                && !matches!(difference.aspect.as_str(), "face kinds" | "line heights")
        })
        .collect();
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(amipro_formats::sam_losses(&document, &options).unwrap(), []);
    // The empty frames write a `[txt]` without paragraphs; the typed one its
    // text.
    let text = String::from_utf8_lossy(&written);
    assert_eq!(text.matches("\t[txt]\r\n>\r\n").count(), 3, "{text}");
    assert!(text.contains("Frame text here."));
}

/// Release-mode timings for creating a frame (`UI_SPEC.md` §20): the
/// dialog's Create and a drawing's release, each timed from the message to
/// the next frame the user sees (view, layout, draw), then undone.
/// `AMIPRO_PERF_SAM` chooses the document.
#[test]
#[ignore = "manual release-mode frame creation performance baseline"]
fn frame_creation_performance_baseline() {
    use crate::page_dialogs::CreateFrameAction;
    use amipro_document::Rect;
    const SAMPLES: usize = 20;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    harness.frame(&app);
    let (mut dialog, mut drawn, mut undo) = (Vec::new(), Vec::new(), Vec::new());
    for step in 0..SAMPLES {
        send(&mut app, Message::Command(CommandId::Frame));
        harness.frame(&app);
        let started = Instant::now();
        send(&mut app, Message::CreateFrame(CreateFrameAction::Create));
        harness.frame(&app);
        dialog.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undo.push(started.elapsed());
        let shift = (step % 2) as i32 * 360;
        let started = Instant::now();
        send(
            &mut app,
            Message::Page(PageAction::FrameDrawn(Some((
                1,
                Rect::new(2_000 + shift, 3_000, 5_000 + shift, 5_000),
            )))),
        );
        harness.frame(&app);
        drawn.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
    }
    eprintln!(
        "frame creation ({} paragraphs, {} pages): dialog Create {}; drawing release {}; undo {}",
        app.document.paragraphs().len(),
        app.pages.page_count(),
        summary(&mut dialog),
        summary(&mut drawn),
        summary(&mut undo)
    );
}

/// The user set two columns and Line between columns on a frame and saw no
/// line (2026-10-17): the line runs down the gutter, the text area's
/// height, as Ami Pro draws it (VM fixture C04).
#[test]
fn a_frames_line_between_columns_is_drawn_c04() {
    use amipro_document::{ColumnLine, Placement, Rect};
    use amipro_editing::frame_creation::FrameDefaults;
    let mut app = page_app(Document::from_plain_text("One.\nTwo."));
    send(&mut app, Message::ZoomTo(100));
    let rect = Rect::new(2_160, 2_880, 6_480, 5_760);
    let mut style = FrameDefaults::AMI_PRO.frame_style(rect);
    style.interior.columns = 2;
    style.interior.tabs = vec![Vec::new(), Vec::new()];
    style.interior.line_between = Some(ColumnLine {
        style: 2,
        colour: 0,
    });
    assert!(
        app.execute_page_command(&amipro_editing::Command::CreateFrame {
            placement: Placement::Fixed { page: 1, rect },
            style,
            content: amipro_editing::NewFrameContent::Text,
            anchor: None,
            layout: None,
        })
    );
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let (bounds, view) = runtime.workspace_view();
    let page = view.pages[0];
    let frame_box = runtime.app.pages.layout().pages[0]
        .frames
        .last()
        .unwrap()
        .clone();
    let centre = frame_box.interior.x + frame_box.interior.width / 2.0;
    let x = (bounds.x + page.x + centre * view.scale).round() as usize;
    let top = (bounds.y + page.y + (frame_box.interior.y + 200.0) * view.scale) as usize;
    let bottom = (bounds.y + page.y + (frame_box.interior.bottom() - 200.0) * view.scale) as usize;
    let rgb = render_window(&runtime.app, (1_280, 800), &cosmic::Theme::light(), false);
    let dark_rows = (top..bottom)
        .filter(|y| {
            // Within a few pixels of where the page view puts it.
            (x.saturating_sub(4)..=x + 4).any(|x| {
                let at = (y * 1_280 + x) * 3;
                rgb[at..at + 3].iter().all(|channel| *channel < 100)
            })
        })
        .count();
    assert_eq!(dark_rows, bottom - top, "the line down the gutter");
}

/// A screenshot of VM fixture C04 (a frame with two columns and a line
/// between them) as the app draws it, when `AMIPRO_VM_LAB` and
/// `AMIPRO_SCREENSHOT_DIR` are set.
#[test]
fn c04_screenshot() {
    let (Some(lab), Some(directory)) = (
        std::env::var_os("AMIPRO_VM_LAB"),
        std::env::var_os("AMIPRO_SCREENSHOT_DIR"),
    ) else {
        eprintln!("AMIPRO_VM_LAB or AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let document = amipro_formats::import_sam_path(std::path::Path::new(&lab).join("C04.SAM"))
        .unwrap()
        .document;
    let mut app = page_app(document);
    send(&mut app, Message::ZoomTo(100));
    let size = (1_280, 800);
    let rgb = render_window(&app, size, &cosmic::Theme::light(), false);
    let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
    ppm.extend(rgb);
    std::fs::write(std::path::Path::new(&directory).join("window-c04.ppm"), ppm).unwrap();
}

/// A screenshot of the top of any SAM file as the app draws it, for
/// comparing with Ami Pro's screens: `AMIPRO_SCREENSHOT_SAM` (the file),
/// `AMIPRO_SCREENSHOT_ZOOM` (percent, default 100), and
/// `AMIPRO_SCREENSHOT_DIR` (receives `window-sam.ppm`).
#[test]
#[ignore = "writes a screenshot of a private file for review"]
fn sam_screenshot() {
    let (Some(path), Some(directory)) = (
        std::env::var_os("AMIPRO_SCREENSHOT_SAM"),
        std::env::var_os("AMIPRO_SCREENSHOT_DIR"),
    ) else {
        eprintln!("AMIPRO_SCREENSHOT_SAM or AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let zoom = std::env::var("AMIPRO_SCREENSHOT_ZOOM")
        .ok()
        .and_then(|zoom| zoom.parse().ok())
        .unwrap_or(100);
    let document = amipro_formats::import_sam_path(&path).unwrap().document;
    let mut app = page_app(document);
    // Linked pictures are looked for beside the file, as in the app.
    app.location = DocumentLocation::LegacySource(std::path::PathBuf::from(&path));
    send(&mut app, Message::ZoomTo(zoom));
    let page = std::env::var("AMIPRO_SCREENSHOT_PAGE").ok();
    let rgb = match page {
        Some(page) => {
            // `AMIPRO_SCREENSHOT_PAGE`: that page, through Go To.
            let mut runtime = Runtime::new(app);
            runtime.send(Message::Command(CommandId::GoToPage));
            runtime.send(Message::GoTo(GoToAction::Input(page)));
            runtime.send(Message::GoTo(GoToAction::Go));
            runtime.redraw();
            runtime.screenshot(&cosmic::Theme::light())
        }
        None => render_window(&app, (1_280, 800), &cosmic::Theme::light(), false),
    };
    let size = (1_280, 800);
    let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
    ppm.extend(rgb);
    std::fs::write(std::path::Path::new(&directory).join("window-sam.ppm"), ppm).unwrap();
}

#[test]
fn bullets_and_numbering_apply_ami_pros_list_styles_b1_b3() {
    use amipro_editing::lists::ListKind;
    // `UI_SPEC.md` §21: the fixture's styles have no list style, so the
    // first Bullets makes Bullet 1 in the document (one undo step).
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    let style_name = |app: &AmiProApp, index: usize| {
        app.document.styles()[&app.document.paragraphs()[index].style]
            .name
            .clone()
    };
    move_to(&mut app, at(main, 0, 3), 1, false);
    assert_eq!(menu_check(&app, CommandId::Bullets), Some(false));
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Bullets));
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(style_name(&app, 0), "Bullet 1");
    assert_eq!(app.selected_list(), Some(ListKind::Bullets));
    assert_eq!(menu_check(&app, CommandId::Bullets), Some(true));
    assert_eq!(menu_check(&app, CommandId::Numbering), Some(false));
    assert_eq!(app.status, "Bullets: Bullet 1");
    assert_eq!(app.page_selection.focus, at(main, 0, 3), "the caret stays");
    // A selection over a bulleted and a plain paragraph: both bulleted.
    move_to(&mut app, at(main, 1, 2), 1, true);
    send(&mut app, Message::Command(CommandId::Bullets));
    assert_eq!(style_name(&app, 1), "Bullet 1");
    assert!(!app.page_selection.is_caret(), "the selection stays");
    // Numbering switches them, making Number List; Bullets again on a
    // numbered pair makes them bulleted; then off to Body Text.
    send(&mut app, Message::Command(CommandId::Numbering));
    assert_eq!(
        (style_name(&app, 0), style_name(&app, 1)),
        ("Number List".into(), "Number List".into())
    );
    send(&mut app, Message::Command(CommandId::Numbering));
    assert_eq!(style_name(&app, 0), "Body Text");
    assert_eq!(app.status, "Numbering off");
    // Undo goes back a step at a time; the made style goes with its step.
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(style_name(&app, 1), "Number List");
    send(&mut app, Message::Command(CommandId::Undo));
    send(&mut app, Message::Command(CommandId::Undo));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(style_name(&app, 0), "Body Text");
    assert!(
        app.document
            .styles()
            .values()
            .all(|style| style.name != "Bullet 1")
    );
    // Enter at the end of a list paragraph continues the list; Enter in
    // the empty one ends it (B3), without a new paragraph.
    move_to(&mut app, at(main, 2, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Bullets));
    let end = app.document.paragraphs()[2].text.len();
    move_to(&mut app, at(main, 2, end), 1, false);
    page(&mut app, PageAction::ParagraphBreak);
    assert_eq!(style_name(&app, 3), "Bullet 1");
    assert!(app.document.paragraphs()[3].text.is_empty());
    let count = app.document.paragraphs().len();
    page(&mut app, PageAction::ParagraphBreak);
    assert_eq!(app.document.paragraphs().len(), count);
    assert_eq!(style_name(&app, 3), "Body Text");
    assert_eq!(app.page_selection.focus, at(main, 3, 0));
    // The shortcuts: Ctrl+Shift+L and Ctrl+Shift+N.
    let shift_ctrl =
        cosmic::iced::keyboard::Modifiers::CTRL | cosmic::iced::keyboard::Modifiers::SHIFT;
    for (key, command) in [("l", CommandId::Bullets), ("n", CommandId::Numbering)] {
        assert_eq!(
            command::command_for_shortcut(
                shift_ctrl,
                &cosmic::iced::keyboard::Key::Character(key.into()),
                None,
            ),
            Some(command)
        );
    }
    // With a frame selected the commands change nothing.
    let frame = text_frame(&app.document);
    send(
        &mut app,
        Message::Frame(crate::frame_editing::FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        }),
    );
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Numbering));
    assert_eq!(app.history.undo_len(), undo);
}

/// Screenshots of bullets and numbering (`UI_SPEC.md` §21), light and
/// dark, when `AMIPRO_SCREENSHOT_DIR` is set: a numbered list with the
/// caret in it (Numbering on in the toolbar).
#[test]
fn bullets_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    move_to(&mut app, at(main, 2, 4), 1, true);
    send(&mut app, Message::Command(CommandId::Numbering));
    // Paragraph 1 one level down: A. under 1. (§22).
    move_to(&mut app, at(main, 1, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Demote));
    move_to(&mut app, at(main, 3, 0), 1, false);
    move_to(&mut app, at(main, 4, 4), 1, true);
    send(&mut app, Message::Command(CommandId::Bullets));
    move_to(&mut app, at(main, 1, 4), 1, false);
    let write = |app: &AmiProApp, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    write(&app, "window-bullets-light", false);
    write(&app, "window-bullets-dark", true);
    // Insert › Bullet… with the pointing hand chosen.
    send(&mut app, Message::Command(CommandId::InsertBullet));
    send(
        &mut app,
        Message::InsertBullet(page_dialogs::BulletAction::Choose(10)),
    );
    write(&app, "window-insertbullet-light", false);
    write(&app, "window-insertbullet-dark", true);
    // Format › Bullet and Number Style…: Style Properties at its Bullets
    // and numbers section for the numbered paragraph's style.
    send(
        &mut app,
        Message::InsertBullet(page_dialogs::BulletAction::Cancel),
    );
    send(&mut app, Message::Command(CommandId::BulletNumberStyle));
    write(&app, "window-bulletstyle-light", false);
    write(&app, "window-bulletstyle-dark", true);
    // Format › Styles ▸ Outline Numbering… (§22).
    send(&mut app, Message::Command(CommandId::OutlineNumbering));
    write(&app, "window-outlinenumbering-light", false);
    write(&app, "window-outlinenumbering-dark", true);
}

#[test]
fn a_caret_after_a_selection_shows_its_own_paragraphs_style() {
    // Found on the bullets screenshot: after a selection over two styles,
    // a click left the hidden editor's selection in place, so the style
    // field said "Mixed".
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    move_to(&mut app, at(main, 2, 4), 1, true);
    send(&mut app, Message::Command(CommandId::Numbering));
    move_to(&mut app, at(main, 3, 0), 1, false);
    move_to(&mut app, at(main, 4, 4), 1, true);
    send(&mut app, Message::Command(CommandId::Bullets));
    move_to(&mut app, at(main, 1, 4), 1, false);
    assert_eq!(app.editor.cursor().selection, None);
    let style = selected_paragraph_style(&app.document, app.editor.cursor()).unwrap();
    assert_eq!(app.document.styles()[&style].name, "Number List");
}

#[test]
fn insert_bullet_puts_ami_pros_bullet_at_the_caret_or_over_the_selection_b4() {
    use page_dialogs::BulletAction;
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 9), 1, false);
    send(&mut app, Message::Command(CommandId::InsertBullet));
    assert_eq!(app.bullet_dialog.as_ref().unwrap().code(), '\u{10}');
    assert!(cosmic::Application::dialog(&app).is_some());
    send(&mut app, Message::InsertBullet(BulletAction::Choose(1)));
    let undo = app.history.undo_len();
    send(&mut app, Message::InsertBullet(BulletAction::Insert));
    assert!(app.bullet_dialog.is_none());
    assert_eq!(app.history.undo_len(), undo + 1);
    let paragraph = &app.document.paragraphs()[0];
    assert_eq!(&paragraph.text[9..12], "\u{FFFC}");
    assert_eq!(
        paragraph.objects,
        [amipro_document::InlineObject::Bullet('\u{11}')]
    );
    assert_eq!(
        app.page_selection.focus,
        at(main, 0, 12),
        "the caret after it"
    );
    assert_eq!(app.preferences.last_bullet, Some('\u{11}'));
    // Next time it offers the dot; over a selection it replaces it, in
    // one undo step.
    move_to(&mut app, at(main, 1, 0), 1, false);
    move_to(&mut app, at(main, 1, 9), 1, true);
    send(&mut app, Message::Command(CommandId::InsertBullet));
    assert_eq!(app.bullet_dialog.as_ref().unwrap().code(), '\u{11}');
    send(&mut app, Message::InsertBullet(BulletAction::Choose(15)));
    let undo = app.history.undo_len();
    send(&mut app, Message::InsertBullet(BulletAction::Insert));
    assert_eq!(app.history.undo_len(), undo + 1);
    assert!(
        app.document.paragraphs()[1]
            .text
            .starts_with("\u{FFFC} 1 café")
    );
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(app.document.paragraphs()[1].text.starts_with("Paragraph 1"));
    // Cancel changes nothing; Customize Toolbar offers the command.
    send(&mut app, Message::Command(CommandId::InsertBullet));
    let undo = app.history.undo_len();
    send(&mut app, Message::InsertBullet(BulletAction::Cancel));
    assert!(app.bullet_dialog.is_none());
    assert_eq!(app.history.undo_len(), undo);
    assert!(chrome::toolbar_choices().contains(&ToolbarItem::Command(CommandId::InsertBullet)));
    assert!(!chrome::standard_toolbar().contains(&ToolbarItem::Command(CommandId::InsertBullet)));
}

#[test]
fn bullet_and_number_style_opens_style_properties_at_its_bullets_b5() {
    use crate::style_properties::{Field, Section, StyleAction};
    // B5: the caret paragraph's style in Style Properties, at Bullets and
    // numbers; Space for there is the style's First indent.
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    send(&mut app, Message::Command(CommandId::Numbering));
    send(&mut app, Message::Command(CommandId::BulletNumberStyle));
    assert!(app.rail_open());
    let state = app.style_properties.as_ref().unwrap();
    let style = app.document.paragraphs()[0].style;
    assert_eq!(state.style, style);
    assert_eq!(app.document.styles()[&style].name, "Number List");
    assert_eq!(
        state.open,
        std::collections::BTreeSet::from([Section::Bullets])
    );
    assert_eq!(state.text(Field::First), "0.25");
    let undo = app.history.undo_len();
    send(
        &mut app,
        Message::StyleProperties(StyleAction::Text(Field::First, "0.5".into())),
    );
    send(
        &mut app,
        Message::StyleProperties(StyleAction::Submit(Field::First)),
    );
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(app.document.styles()[&style].indents.first, 720);
}

/// One style's `[tag]` record in an Ami Pro document, if it has one.
fn tag_record(written: &[u8], name: &str) -> Option<String> {
    let text = String::from_utf8_lossy(written).replace('\r', "");
    text.split("[tag]\n")
        .skip(1)
        .find(|record| record.starts_with(&format!("\t{name}\n")))
        .map(|record| {
            // The record ends where the next top-level section starts.
            record
                .lines()
                .take_while(|line| line.starts_with('\t'))
                .collect::<Vec<_>>()
                .join("\n")
        })
}

/// The bullets and numbering twins (`docs/BULLETS_PROGRESS.md`, BN.9): VM
/// fixtures made again in the app and written as Ami Pro documents, their
/// bodies (and a made style's record) equal to Ami Pro's. BL1's 17 bullets
/// through Insert › Bullet…; BL2's Number List through Numbering on its
/// paragraphs as Body Text; BL4's Number List made by Numbering on Ami
/// Pro's base BL4B (`_BASIC.STY`, which has none), against the one Ami
/// Pro's Create Style made. With `AMIPRO_TWIN_OUT`, written there as
/// `ABL1.SAM`… for Ami Pro to open.
#[test]
#[ignore = "needs the private VM fixtures (AMIPRO_VM_LAB)"]
fn bullet_twins() {
    use amipro_document::InlineObject;
    use page_dialogs::BulletAction;
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let sheets = vec![lab.join("../STY")];
    let out = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from);
    let open = |name: &str| {
        page_app(
            amipro_formats::import_sam_path_with_style_directories(
                lab.join(format!("{name}.SAM")),
                &sheets,
            )
            .unwrap()
            .document,
        )
    };
    let mut cases: Vec<(&str, AmiProApp, Option<&str>)> = Vec::new();

    // BL1: every inserted bullet taken out, then put back with Insert ›
    // Bullet… where it stood. Compared with BL1 as read and written: Ami
    // Pro's file also holds an empty bold on and off (`<+!><-!>`) left by
    // the typing, which no run holds.
    let mut bl1 = open("BL1");
    let bl1_as_read =
        amipro_formats::sam_document(&bl1.document, &amipro_formats::SamOptions::at(0));
    let main = bl1.document.main_flow_id();
    let mut bullets = Vec::new();
    for (index, paragraph) in bl1.document.paragraphs().iter().enumerate() {
        for (offset, object) in paragraph.object_offsets().zip(&paragraph.objects) {
            if let InlineObject::Bullet(code) = object {
                bullets.push((index, offset, *code));
            }
        }
    }
    assert_eq!(bullets.len(), 20);
    for (index, offset, _) in bullets.iter().rev() {
        assert!(
            bl1.execute_page_command(&amipro_editing::Command::ReplaceText {
                start: at(main, *index, *offset),
                end: at(main, *index, offset + '\u{FFFC}'.len_utf8()),
                text: String::new(),
            })
        );
    }
    // In order, each going back where it stood once those before it are
    // back.
    for (index, offset, code) in &bullets {
        move_to(&mut bl1, at(main, *index, *offset), 1, false);
        send(&mut bl1, Message::Command(CommandId::InsertBullet));
        let choice = page_editing::BULLET_NAMES
            .iter()
            .position(|(name_code, _)| name_code == code)
            .unwrap();
        send(
            &mut bl1,
            Message::InsertBullet(BulletAction::Choose(choice)),
        );
        send(&mut bl1, Message::InsertBullet(BulletAction::Insert));
    }
    cases.push(("BL1", bl1, None));

    // BL2: the paragraphs as Body Text, then Numbering over all of them.
    let mut bl2 = open("BL2");
    let main = bl2.document.main_flow_id();
    let count = bl2.document.paragraphs().len();
    let body = bl2.document.body_text_style();
    assert!(
        bl2.execute_page_command(&amipro_editing::Command::SetParagraphStyle {
            flow: main,
            first: 0,
            last: count - 1,
            style: body,
        })
    );
    move_to(&mut bl2, at(main, 0, 0), 1, false);
    let end = bl2.document.paragraphs()[count - 1].text.len();
    move_to(&mut bl2, at(main, count - 1, end), 1, true);
    send(&mut bl2, Message::Command(CommandId::Numbering));
    cases.push(("BL2", bl2, None));

    // BL4: Numbering on Ami Pro's base without a list style.
    let mut bl4 = open("BL4B");
    let main = bl4.document.main_flow_id();
    let count = bl4.document.paragraphs().len();
    move_to(&mut bl4, at(main, 0, 0), 1, false);
    let end = bl4.document.paragraphs()[count - 1].text.len();
    move_to(&mut bl4, at(main, count - 1, end), 1, true);
    send(&mut bl4, Message::Command(CommandId::Numbering));
    cases.push(("BL4", bl4, Some("Number List")));

    let options = amipro_formats::SamOptions::at(0);
    let mut different = Vec::new();
    for (name, app, tag) in &cases {
        let written = amipro_formats::sam_document(&app.document, &options);
        if let Some(out) = &out {
            std::fs::write(out.join(format!("A{name}.SAM")), &written).unwrap();
        }
        let theirs = if *name == "BL1" {
            bl1_as_read.clone()
        } else {
            std::fs::read(lab.join(format!("{name}.SAM"))).unwrap()
        };
        let (ours_body, theirs_body) = (frames_and_body(&written).1, frames_and_body(&theirs).1);
        let mut same = true;
        if ours_body != theirs_body {
            eprintln!("{name}: body differs\nours:\n{ours_body}\nAmi Pro's:\n{theirs_body}");
            same = false;
        }
        if let Some(tag) = tag {
            let (ours, theirs) = (tag_record(&written, tag), tag_record(&theirs, tag));
            if ours != theirs {
                eprintln!("{name}: {tag} differs\nours:\n{ours:?}\nAmi Pro's:\n{theirs:?}");
                same = false;
            }
        }
        if same {
            eprintln!("{name}: the same");
        } else {
            different.push(*name);
        }
    }
    assert!(different.is_empty(), "differ: {different:?}");
}

/// Lists made with the toggles (made styles included), inserted bullets in
/// the main text and a frame, and a list ended with Enter, read back as
/// made from `.amipro` and from an Ami Pro document, with nothing in Save
/// As's loss check (BN.10).
#[test]
fn lists_and_bullets_read_back_in_both_formats() {
    use amipro_document::Rect;
    use amipro_formats::roundtrip::{differences, is_writer_addition};
    use page_dialogs::BulletAction;
    let text: Vec<String> = (0..8)
        .map(|index| format!("Paragraph {index} with a few words in it."))
        .collect();
    let mut app = page_app(Document::from_plain_text(&text.join("\n")));
    app.create_frame(1, Rect::new(1_440, 10_800, 4_320, 12_960));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    move_to(&mut app, at(main, 2, 3), 1, true);
    send(&mut app, Message::Command(CommandId::Numbering));
    move_to(&mut app, at(main, 3, 2), 1, false);
    send(&mut app, Message::Command(CommandId::Bullets));
    let end = app.document.paragraphs()[3].text.len();
    move_to(&mut app, at(main, 3, end), 1, false);
    page(&mut app, PageAction::ParagraphBreak);
    page(&mut app, PageAction::ParagraphBreak);
    for (paragraph, choice) in [(5, 13), (6, 16)] {
        move_to(&mut app, at(main, paragraph, 4), 1, false);
        send(&mut app, Message::Command(CommandId::InsertBullet));
        send(
            &mut app,
            Message::InsertBullet(BulletAction::Choose(choice)),
        );
        send(&mut app, Message::InsertBullet(BulletAction::Insert));
    }
    let frame = text_frame(&app.document);
    let flow = flow_of(&app.document, frame);
    app.sessions[0].frame_selection = None;
    move_to(&mut app, at(flow, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::InsertBullet));
    send(&mut app, Message::InsertBullet(BulletAction::Choose(0)));
    send(&mut app, Message::InsertBullet(BulletAction::Insert));
    assert_eq!(
        app.document.flow(flow).unwrap().paragraphs[0].objects,
        [amipro_document::InlineObject::Bullet('\u{10}')]
    );
    let document = app.document.clone();
    let names: Vec<_> = document.paragraphs()[..5]
        .iter()
        .map(|paragraph| document.styles()[&paragraph.style].name.clone())
        .collect();
    assert_eq!(
        names,
        [
            "Number List",
            "Number List",
            "Number List",
            "Bullet 1",
            "Body Text"
        ]
    );

    let encoded = document.encode_native().unwrap();
    assert_eq!(Document::decode_native(&encoded).unwrap(), document);

    let options = amipro_formats::SamOptions::at(0);
    let written = amipro_formats::sam_document(&document, &options);
    let copy = amipro_formats::import_sam(&written, None, "Lists").unwrap();
    let found: Vec<_> = differences(&document, &copy.document)
        .into_iter()
        .filter(|difference| {
            !is_writer_addition(difference)
                && !matches!(difference.aspect.as_str(), "face kinds" | "line heights")
        })
        .collect();
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(amipro_formats::sam_losses(&document, &options).unwrap(), []);
    let text = String::from_utf8_lossy(&written);
    assert!(text.contains("<:B<*,>>"), "the checked box");
    assert!(text.contains("<:B<*0>>"), "the frame's small dot");
    assert!(
        text.contains("\r\n\tBullet 1\r\n"),
        "the made Bullet 1 stored"
    );
}

/// Release-mode timings for the list commands (`UI_SPEC.md` §21): Numbering
/// over three paragraphs on the first page, its undo, and Insert › Bullet…'s
/// Insert, each from the message to the next frame the user sees (view,
/// layout, draw). `AMIPRO_PERF_SAM` chooses the document.
#[test]
#[ignore = "manual release-mode bullets performance baseline"]
fn bullets_performance_baseline() {
    use page_dialogs::BulletAction;
    const SAMPLES: usize = 20;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    let main = app.document.main_flow_id();
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    harness.frame(&app);
    let (mut toggle, mut undo, mut bullet) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..SAMPLES {
        move_to(&mut app, at(main, 2, 0), 1, false);
        move_to(&mut app, at(main, 4, 0), 1, true);
        harness.frame(&app);
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Numbering));
        harness.frame(&app);
        toggle.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undo.push(started.elapsed());
        move_to(&mut app, at(main, 2, 0), 1, false);
        send(&mut app, Message::Command(CommandId::InsertBullet));
        harness.frame(&app);
        let started = Instant::now();
        send(&mut app, Message::InsertBullet(BulletAction::Insert));
        harness.frame(&app);
        bullet.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
    }
    eprintln!(
        "bullets ({} paragraphs, {} pages): Numbering {}; undo {}; Insert Bullet {}",
        app.document.paragraphs().len(),
        app.pages.page_count(),
        summary(&mut toggle),
        summary(&mut undo),
        summary(&mut bullet)
    );
}

/// The bullet or number text drawn before paragraph `index`'s first line.
fn drawn_number(app: &AmiProApp, index: usize) -> Option<String> {
    let paragraph = app.document.paragraphs()[index].id;
    app.pages
        .layout()
        .pages
        .iter()
        .flat_map(|page| page.lines.iter())
        .find(|line| line.paragraph == paragraph && line.bullet.is_some())
        .map(|line| {
            line.bullet
                .as_ref()
                .unwrap()
                .pieces
                .iter()
                .map(|piece| match piece {
                    amipro_layout::BulletPiece::Text { text, .. } => text.clone(),
                    amipro_layout::BulletPiece::Shape { .. } => String::new(),
                })
                .collect()
        })
}

#[test]
fn promote_and_demote_nest_a_bulleted_list_as_diamond_dot_square() {
    // The user, 2026-10-18: bullets nest as numbered lists do.
    let mut app = page_app(Document::from_plain_text("One\nTwo\nThree\nAfter"));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    move_to(&mut app, at(main, 2, 1), 1, true);
    send(&mut app, Message::Command(CommandId::Bullets));
    let drawn_shape = |app: &AmiProApp, index: usize| {
        let paragraph = app.document.paragraphs()[index].id;
        app.pages
            .layout()
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .find(|line| line.paragraph == paragraph && line.bullet.is_some())
            .and_then(|line| {
                line.bullet
                    .as_ref()
                    .unwrap()
                    .pieces
                    .iter()
                    .find_map(|piece| match piece {
                        amipro_layout::BulletPiece::Shape { code, .. } => Some(*code),
                        amipro_layout::BulletPiece::Text { .. } => None,
                    })
            })
    };
    let style_name = |app: &AmiProApp, index: usize| {
        app.document.styles()[&app.document.paragraphs()[index].style]
            .name
            .clone()
    };
    move_to(&mut app, at(main, 1, 0), 1, false);
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Demote));
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(style_name(&app, 1), "Bullet 2");
    assert_eq!(drawn_shape(&app, 0), Some('\u{15}'), "a small diamond");
    assert_eq!(drawn_shape(&app, 1), Some('\u{11}'), "a dot");
    send(&mut app, Message::Command(CommandId::Demote));
    assert_eq!(drawn_shape(&app, 1), Some('\u{12}'), "a small square");
    // Further in at each depth.
    let left = |app: &AmiProApp, index: usize| {
        let paragraph = app.document.paragraphs()[index].id;
        app.pages
            .layout()
            .pages
            .iter()
            .flat_map(|page| page.lines.iter())
            .find(|line| line.paragraph == paragraph)
            .unwrap()
            .bullet
            .as_ref()
            .unwrap()
            .x
    };
    assert!(left(&app, 1) > left(&app, 0) + 700.0);
    send(&mut app, Message::Command(CommandId::Promote));
    send(&mut app, Message::Command(CommandId::Promote));
    assert_eq!(style_name(&app, 1), "Bullet 1");
    send(&mut app, Message::Command(CommandId::Promote));
    assert_eq!(style_name(&app, 1), "Bullet 1", "the top does not promote");
    send(&mut app, Message::Command(CommandId::Undo));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(style_name(&app, 1), "Bullet 3");
}

#[test]
fn promote_and_demote_nest_a_numbered_list_as_one_a_i_o1_o2() {
    use amipro_document::{Bullets, ParagraphStyle, StyleId};
    // A document whose sheet gives Title level 1 and Subhead level 2, as
    // `_DEFAULT.STY` does; a Title, then four items.
    let mut document = Document::from_plain_text("Heading\nOne\nTwo\nThree\nFour");
    let body = document.paragraphs()[0].style;
    for (id, name, level) in [(60u64, "Title", 1u8), (61, "Subhead", 2)] {
        let style = ParagraphStyle {
            id: StyleId(id),
            name: name.into(),
            bullets: Bullets {
                level,
                ..Bullets::default()
            },
            ..document.styles()[&body].clone()
        };
        document.styles_mut().insert(style.id, style);
    }
    document.paragraph_mut(0).style = StyleId(60);
    let mut app = page_app(document);
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 1, 0), 1, false);
    move_to(&mut app, at(main, 4, 2), 1, true);
    send(&mut app, Message::Command(CommandId::Numbering));
    // Demote Two and Three (Alt+Right): A. and B. under 1.; Four is 2.
    move_to(&mut app, at(main, 2, 0), 1, false);
    move_to(&mut app, at(main, 3, 1), 1, true);
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Demote));
    assert_eq!(app.history.undo_len(), undo + 1);
    let names: Vec<String> = app
        .document
        .paragraphs()
        .iter()
        .map(|paragraph| app.document.styles()[&paragraph.style].name.clone())
        .collect();
    assert_eq!(names, ["Title", "List 1", "List 2", "List 2", "List 1"]);
    let shown: Vec<Option<String>> = (0..5).map(|index| drawn_number(&app, index)).collect();
    assert_eq!(
        shown,
        [
            None,
            Some("1.".into()),
            Some("A.".into()),
            Some("B.".into()),
            Some("2.".into())
        ]
    );
    assert_eq!(app.status, "List 2, level 4");
    // Three one level further: i.; then promoted back.
    move_to(&mut app, at(main, 3, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Demote));
    assert_eq!(drawn_number(&app, 3).as_deref(), Some("i."));
    send(&mut app, Message::Command(CommandId::Promote));
    assert_eq!(drawn_number(&app, 3).as_deref(), Some("B."));
    // The top of the list does not promote; the status says so.
    move_to(&mut app, at(main, 1, 0), 1, false);
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::Promote));
    assert_eq!(app.history.undo_len(), undo);
    assert_eq!(app.status, "Already at the top level");
    // Alt+Left and Alt+Right are the shortcuts.
    let alt = cosmic::iced::keyboard::Modifiers::ALT;
    for (named, command) in [
        (
            cosmic::iced::keyboard::key::Named::ArrowLeft,
            CommandId::Promote,
        ),
        (
            cosmic::iced::keyboard::key::Named::ArrowRight,
            CommandId::Demote,
        ),
    ] {
        assert_eq!(
            command::command_for_shortcut(alt, &cosmic::iced::keyboard::Key::Named(named), None),
            Some(command)
        );
    }
}

#[test]
fn outline_numbering_applies_a_scheme_to_the_levelled_styles_ol2() {
    use amipro_document::{Bullets, ParagraphStyle, StyleId};
    use page_dialogs::{NumberingScheme, OutlineNumberingAction};
    // OL2: Title (1), Subhead ×2, Title, Subhead; Ami Pro's 1, 1.1, 1.1.1.
    let mut document = Document::from_plain_text("T1\nS1\nS2\nT2\nS3\nBody");
    let body = document.paragraphs()[0].style;
    for (id, name, level) in [(60u64, "Title", 1u8), (61, "Subhead", 2)] {
        let style = ParagraphStyle {
            id: StyleId(id),
            name: name.into(),
            bullets: Bullets {
                level,
                ..Bullets::default()
            },
            ..document.styles()[&body].clone()
        };
        document.styles_mut().insert(style.id, style);
    }
    for (index, id) in [60, 61, 61, 60, 61].into_iter().enumerate() {
        document.paragraph_mut(index).style = StyleId(id);
    }
    let mut app = page_app(document.clone());
    send(&mut app, Message::Command(CommandId::OutlineNumbering));
    assert_eq!(
        app.outline_numbering.as_ref().unwrap().styles,
        ["Title (1)", "Subhead (2)"]
    );
    send(
        &mut app,
        Message::OutlineNumbering(OutlineNumberingAction::Choose(NumberingScheme::Legal)),
    );
    let undo = app.history.undo_len();
    send(
        &mut app,
        Message::OutlineNumbering(OutlineNumberingAction::Apply),
    );
    assert!(app.outline_numbering.is_none());
    assert_eq!(app.history.undo_len(), undo + 1);
    let shown: Vec<Option<String>> = (0..6).map(|index| drawn_number(&app, index)).collect();
    assert_eq!(
        shown,
        [
            Some("1".into()),
            Some("1.1".into()),
            Some("1.2".into()),
            Some("2".into()),
            Some("2.1".into()),
            None
        ]
    );
    // Ami Pro's I, A, 1: I. and A.
    send(&mut app, Message::Command(CommandId::OutlineNumbering));
    send(
        &mut app,
        Message::OutlineNumbering(OutlineNumberingAction::Choose(NumberingScheme::Outline)),
    );
    send(
        &mut app,
        Message::OutlineNumbering(OutlineNumberingAction::Apply),
    );
    assert_eq!(drawn_number(&app, 3).as_deref(), Some("II."));
    assert_eq!(drawn_number(&app, 4).as_deref(), Some("A."));
    // Undo steps back a scheme at a time.
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(drawn_number(&app, 4).as_deref(), Some("2.1"));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(drawn_number(&app, 4), None);
    // With no levelled style, Apply does nothing.
    let mut plain = page_app(Document::from_plain_text("One"));
    send(&mut plain, Message::Command(CommandId::OutlineNumbering));
    send(
        &mut plain,
        Message::OutlineNumbering(OutlineNumberingAction::Apply),
    );
    assert!(plain.outline_numbering.is_some(), "stays open");
}

/// The outline levels twins (`docs/OUTLINE_PROGRESS.md`, OL.7): OL1's
/// Outline3 made again by Demote on Ami Pro's base OL1B, and OL2's 1, 1.1,
/// 1.1.1 by Outline Numbering on OL2B, written as Ami Pro wrote them (body
/// and the changed `[tag]` records; OL2's Number List, whose number Ami
/// Pro's button also cleared, is left out). A nested list made from OL3B is
/// written too, for Ami Pro to show. With `AMIPRO_TWIN_OUT`, written there
/// as `AOL1.SAM`, `AOL2.SAM`, `AOL3.SAM`.
#[test]
#[ignore = "needs the private VM fixtures (AMIPRO_VM_LAB)"]
fn outline_twins() {
    use amipro_editing::outline::LevelChange;
    use page_dialogs::{NumberingScheme, OutlineNumberingAction};
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let sheets = vec![lab.join("../STY")];
    let out = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from);
    let open = |name: &str| {
        page_app(
            amipro_formats::import_sam_path_with_style_directories(
                lab.join(format!("{name}.SAM")),
                &sheets,
            )
            .unwrap()
            .document,
        )
    };
    let level = |app: &mut AmiProApp, first: usize, last: usize, change: LevelChange| {
        let main = app.document.main_flow_id();
        move_to(app, at(main, first, 0), 1, false);
        move_to(app, at(main, last, 1), 1, true);
        send(
            app,
            Message::Command(match change {
                LevelChange::Promote => CommandId::Promote,
                LevelChange::Demote => CommandId::Demote,
            }),
        );
    };
    let mut cases: Vec<(&str, AmiProApp, Vec<&str>)> = Vec::new();
    // OL1: Demote Sub two (Outline3 made), then Body one (to Outline3).
    let mut ol1 = open("OL1B");
    level(&mut ol1, 3, 3, LevelChange::Demote);
    level(&mut ol1, 2, 2, LevelChange::Demote);
    cases.push(("OL1", ol1, vec!["Outline3"]));
    // OL2: Ami Pro's 1, 1.1, 1.1.1.
    let mut ol2 = open("OL2B");
    send(&mut ol2, Message::Command(CommandId::OutlineNumbering));
    send(
        &mut ol2,
        Message::OutlineNumbering(OutlineNumberingAction::Choose(NumberingScheme::Legal)),
    );
    send(
        &mut ol2,
        Message::OutlineNumbering(OutlineNumberingAction::Apply),
    );
    cases.push(("OL2", ol2, vec!["Title", "Subhead"]));
    let options = amipro_formats::SamOptions::at(0);
    let mut different = Vec::new();
    for (name, app, tags) in &cases {
        let written = amipro_formats::sam_document(&app.document, &options);
        if let Some(out) = &out {
            std::fs::write(out.join(format!("A{name}.SAM")), &written).unwrap();
        }
        let theirs = std::fs::read(lab.join(format!("{name}.SAM"))).unwrap();
        let (ours_body, theirs_body) = (frames_and_body(&written).1, frames_and_body(&theirs).1);
        let mut same = true;
        if ours_body != theirs_body {
            eprintln!("{name}: body differs\nours:\n{ours_body}\nAmi Pro's:\n{theirs_body}");
            same = false;
        }
        for tag in tags {
            let (ours, theirs) = (tag_record(&written, tag), tag_record(&theirs, tag));
            if ours != theirs {
                eprintln!("{name}: {tag} differs\nours:\n{ours:?}\nAmi Pro's:\n{theirs:?}");
                same = false;
            }
        }
        if same {
            eprintln!("{name}: the same");
        } else {
            different.push(*name);
        }
    }
    // A nested list for Ami Pro to show: Two and Three under One.
    let mut ol3 = open("OL3B");
    level(&mut ol3, 2, 3, LevelChange::Demote);
    if let Some(out) = &out {
        std::fs::write(
            out.join("AOL3.SAM"),
            amipro_formats::sam_document(&ol3.document, &options),
        )
        .unwrap();
    }
    let shown: Vec<Option<String>> = (0..5).map(|index| drawn_number(&ol3, index)).collect();
    eprintln!("AOL3 as the app draws it: {shown:?}");
    // A nested bulleted list (the user, 2026-10-18): the four items in the
    // sheet's Bullet 1, Two demoted once and Three twice.
    let mut ol4 = open("OL3B");
    let main = ol4.document.main_flow_id();
    move_to(&mut ol4, at(main, 1, 0), 1, false);
    move_to(&mut ol4, at(main, 4, 1), 1, true);
    send(&mut ol4, Message::Command(CommandId::Bullets));
    level(&mut ol4, 2, 3, LevelChange::Demote);
    level(&mut ol4, 3, 3, LevelChange::Demote);
    let names: Vec<_> = ol4.document.paragraphs()[1..]
        .iter()
        .map(|paragraph| ol4.document.styles()[&paragraph.style].name.clone())
        .collect();
    assert_eq!(names, ["Bullet 1", "Bullet 2", "Bullet 3", "Bullet 1"]);
    if let Some(out) = &out {
        std::fs::write(
            out.join("AOL4.SAM"),
            amipro_formats::sam_document(&ol4.document, &options),
        )
        .unwrap();
    }
    assert!(different.is_empty(), "differ: {different:?}");
}

#[test]
fn outline_levels_and_cumulative_numbers_read_back_in_both_formats() {
    use amipro_formats::roundtrip::{differences, is_writer_addition};
    use page_dialogs::{NumberingScheme, OutlineNumberingAction};
    // A numbered list nested two deep by Demote, then numbered 1, 1.1,
    // 1.1.1 by Outline Numbering (OL.8).
    let mut app = page_app(Document::from_plain_text(
        "One\nTwo\nThree\nFour\nFive\nAfter the list",
    ));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    move_to(&mut app, at(main, 4, 1), 1, true);
    send(&mut app, Message::Command(CommandId::Numbering));
    move_to(&mut app, at(main, 1, 0), 1, false);
    move_to(&mut app, at(main, 2, 1), 1, true);
    send(&mut app, Message::Command(CommandId::Demote));
    move_to(&mut app, at(main, 2, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Demote));
    send(&mut app, Message::Command(CommandId::OutlineNumbering));
    send(
        &mut app,
        Message::OutlineNumbering(OutlineNumberingAction::Choose(NumberingScheme::Legal)),
    );
    send(
        &mut app,
        Message::OutlineNumbering(OutlineNumberingAction::Apply),
    );
    let shown: Vec<_> = (0..6).map(|index| drawn_number(&app, index)).collect();
    assert_eq!(
        shown,
        [
            Some("1".to_owned()),
            Some("1.1".to_owned()),
            Some("1.1.1".to_owned()),
            Some("2".to_owned()),
            Some("3".to_owned()),
            None
        ]
    );
    let document = app.document.clone();
    let names: Vec<_> = document.paragraphs()[..3]
        .iter()
        .map(|paragraph| document.styles()[&paragraph.style].name.clone())
        .collect();
    assert_eq!(names, ["List 1", "List 2", "List 3"]);

    let encoded = document.encode_native().unwrap();
    assert_eq!(Document::decode_native(&encoded).unwrap(), document);
    let options = amipro_formats::SamOptions::at(0);
    let written = amipro_formats::sam_document(&document, &options);
    let copy = amipro_formats::import_sam(&written, None, "Levels").unwrap();
    let found: Vec<_> = differences(&document, &copy.document)
        .into_iter()
        .filter(|difference| {
            !is_writer_addition(difference)
                && !matches!(difference.aspect.as_str(), "face kinds" | "line heights")
        })
        .collect();
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(amipro_formats::sam_losses(&document, &options).unwrap(), []);
    // The copy numbers as the original does.
    let reopened = page_app(copy.document);
    let again: Vec<_> = (0..6).map(|index| drawn_number(&reopened, index)).collect();
    assert_eq!(again, shown);
}

/// CF.8: a picture in a cell, and its cell after Delete, read back from
/// `.amipro` exactly and from SAM with nothing in the loss check; the copy
/// lays the picture out in its cell.
#[test]
fn cell_pictures_read_back_in_both_formats() {
    use amipro_formats::roundtrip::{differences, is_writer_addition};
    // TF1's shape (`CELL_FRAME_NOTES.md`): a linked picture in the second
    // cell of a table under a line.
    let picture_frame = "[frm]\r\n\t1\r\n\t536871136\r\n\t4586\r\n\t1767\r\n\t7603\r\n\t2051\r\n\t1\r\n\t1\r\n\t1\r\n\t0 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t4\r\n\
        \t[frmlay]\r\n\t\t2051\r\n\t\t3017\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t1767\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t0\r\n\t\t0\r\n\t\t120\r\n\t\t1\r\n\t\t4730\r\n\t\t7459\r\n\t\t0\r\n\
        \t[isd]\r\n\t\t.X4\r\n\t\t.bmp\r\n\t\tL:\\IN\\PIC.BMP\r\n\t\t18\r\n";
    let table_frame = "[frm]\r\n\t1\r\n\t1573060\r\n\t1440\r\n\t1724\r\n\t10792\r\n\t2507\r\n\t1\r\n\t1\r\n\t1\r\n\t1 0 0 0 0 0 0\r\n\t0\r\n\t16777215\r\n\t3\r\n\t0\r\n\t2560\r\n\t256 0\r\n\
        \t[frmlay]\r\n\t\t2292\r\n\t\t9352\r\n\t\t1\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t1724\r\n\t\t0\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t0\r\n\t\t0\r\n\t\t0\r\n\t\t1\r\n\t\t1584\r\n\t\t10656\r\n\t\t0\r\n\
        \t[tbl]\r\n\t\t 2 3 284 86 3017 86 1 43 43\r\n\t\t[data]\r\n\t\t\t 0 1 0 0 0 0 0 0 0 0 1 0\r\n\t\t[e]\r\n\t[tble]\r\n";
    let sam = format!(
        "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tTimes New Roman\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
         {picture_frame}{table_frame}[edoc]\r\nBefore the table.\r\n\r\n<:t0>\r\n\r\n>\r\n"
    );
    let document = amipro_formats::import_sam(sam.as_bytes(), None, "TF1")
        .unwrap()
        .document;
    let picture = document.frame_stack()[0];
    let (table, cell) = document.cell_holding(picture).unwrap();
    let FrameContent::Table(content) = &document.frame(table).unwrap().content else {
        unreachable!()
    };
    let amipro_document::CellContent::Text(holder) = content.cells[cell].content;
    let mut deleted = page_app(document.clone());
    move_to(&mut deleted, at(holder, 0, 0), 1, false);
    page(&mut deleted, PageAction::Delete);
    assert!(deleted.document.frame(picture).is_none());
    let options = amipro_formats::SamOptions::at(0);
    for (name, document) in [("picture", document), ("deleted", deleted.document.clone())] {
        let encoded = document.encode_native().unwrap();
        assert_eq!(
            Document::decode_native(&encoded).unwrap(),
            document,
            "{name}"
        );
        let written = amipro_formats::sam_document(&document, &options);
        let copy = amipro_formats::import_sam(&written, None, "Cells").unwrap();
        let found: Vec<_> = differences(&document, &copy.document)
            .into_iter()
            .filter(|difference| {
                !is_writer_addition(difference)
                    && !matches!(difference.aspect.as_str(), "face kinds" | "line heights")
            })
            .collect();
        assert!(found.is_empty(), "{name}: {found:?}");
        assert_eq!(
            amipro_formats::sam_losses(&document, &options).unwrap(),
            [],
            "{name}"
        );
        // The copy's picture is in its cell, right above its table.
        let reopened = page_app(copy.document);
        let frames = &reopened.pages.layout().pages[0].frames;
        let holders: Vec<_> = reopened
            .document
            .frames()
            .filter_map(|frame| reopened.document.cell_holding(frame.id))
            .collect();
        if name == "picture" {
            assert_eq!(holders.len(), 1);
            let at = frames
                .iter()
                .position(|frame_box| frame_box.frame == holders[0].0)
                .unwrap();
            assert!(
                reopened
                    .document
                    .frame(frames[at + 1].frame)
                    .is_some_and(|frame| frame.placement.is_in_cell())
            );
        } else {
            assert!(holders.is_empty());
        }
    }
}

/// CF.8: typing beside a picture in a cell, Delete on a picture, and a page
/// break before the table, each from the message to the drawn frame.
/// `AMIPRO_PERF_SAM` chooses the document (README31.SAM's SmartIcon table).
#[test]
#[ignore = "manual release-mode cell frame performance baseline"]
fn cell_frame_performance_baseline() {
    const SAMPLES: usize = 20;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(
        || crate::page_surface::cell_picture_fixture().0,
        |path| amipro_formats::import_sam_path(path).unwrap().document,
    );
    let mut app = page_app(document);
    let main = app.document.main_flow_id();
    let (table, beside, holder) = app
        .document
        .frames()
        .find_map(|frame| match &frame.content {
            FrameContent::Table(table) => {
                let flow = |cell: &amipro_document::TableCell| {
                    let amipro_document::CellContent::Text(flow) = cell.content;
                    flow
                };
                let holder = table.cells.iter().find(|cell| cell.frame.is_some())?;
                let beside = table
                    .cells
                    .iter()
                    .find(|cell| cell.frame.is_none() && !cell.span.is_covered())?;
                Some((frame.id, flow(beside), flow(holder)))
            }
            _ => None,
        })
        .expect("a table with a picture in a cell");
    let anchor = app
        .document
        .paragraphs()
        .iter()
        .position(|paragraph| {
            paragraph
                .objects
                .iter()
                .any(|object| object.anchored_frame() == Some(table))
        })
        .unwrap_or(1);
    let page_of = |app: &AmiProApp| {
        app.pages
            .layout()
            .pages
            .iter()
            .find(|page| page.frames.iter().any(|frame_box| frame_box.frame == table))
            .map_or(1, |page| page.number)
    };
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    harness.frame(&app);
    let (mut typing, mut delete, mut undo_delete, mut breaking, mut undo_break) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for _ in 0..SAMPLES {
        let page_number = page_of(&app);
        let end = app.document.flow(beside).unwrap().paragraphs[0].text.len();
        move_to(&mut app, at(beside, 0, end), page_number, false);
        harness.frame(&app);
        let started = Instant::now();
        page(&mut app, PageAction::Insert("e".into()));
        harness.frame(&app);
        typing.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
        move_to(&mut app, at(holder, 0, 0), page_number, false);
        harness.frame(&app);
        let started = Instant::now();
        page(&mut app, PageAction::Delete);
        harness.frame(&app);
        delete.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undo_delete.push(started.elapsed());
        let before = anchor.saturating_sub(1);
        let end = app.document.paragraphs()[before].text.len();
        move_to(&mut app, at(main, before, end), page_number, false);
        harness.frame(&app);
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::PageBreak));
        harness.frame(&app);
        breaking.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undo_break.push(started.elapsed());
    }
    eprintln!(
        "cells ({} paragraphs, {} pages): typing beside a picture {}; Delete on a picture {}, undo {}; a page break before the table {}, undo {}",
        app.document.paragraphs().len(),
        app.pages.page_count(),
        summary(&mut typing),
        summary(&mut delete),
        summary(&mut undo_delete),
        summary(&mut breaking),
        summary(&mut undo_break)
    );
}

/// Promote, Demote, and Outline Numbering's Apply, each from the message to
/// the drawn frame (update, layout, draw). `AMIPRO_PERF_SAM` chooses the
/// document.
#[test]
#[ignore = "manual release-mode outline performance baseline"]
fn outline_performance_baseline() {
    use page_dialogs::{NumberingScheme, OutlineNumberingAction};
    const SAMPLES: usize = 20;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    let main = app.document.main_flow_id();
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    harness.frame(&app);
    let (mut demote, mut promote, mut scheme, mut undo) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for _ in 0..SAMPLES {
        move_to(&mut app, at(main, 3, 0), 1, false);
        move_to(&mut app, at(main, 5, 0), 1, true);
        harness.frame(&app);
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Demote));
        harness.frame(&app);
        demote.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Promote));
        harness.frame(&app);
        promote.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        send(&mut app, Message::Command(CommandId::OutlineNumbering));
        send(
            &mut app,
            Message::OutlineNumbering(OutlineNumberingAction::Choose(NumberingScheme::Legal)),
        );
        harness.frame(&app);
        let started = Instant::now();
        send(
            &mut app,
            Message::OutlineNumbering(OutlineNumberingAction::Apply),
        );
        harness.frame(&app);
        scheme.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undo.push(started.elapsed());
    }
    eprintln!(
        "outline ({} paragraphs, {} pages): Demote {}; Promote {}; Outline Numbering {}; undo {}",
        app.document.paragraphs().len(),
        app.pages.page_count(),
        summary(&mut demote),
        summary(&mut promote),
        summary(&mut scheme),
        summary(&mut undo)
    );
}

#[test]
fn colour_panels_offer_ami_pro_in_use_recent_and_the_picker() {
    // `UI_SPEC.md` §23, C1–C4.
    use crate::colour_control::{ColourAction, Target};
    use cosmic::cosmic_theme::palette::{FromColor, Hsv, Srgb};
    use cosmic::widget::color_picker::ColorPickerUpdate;
    let colour = |app: &mut AmiProApp, action: ColourAction| send(app, Message::Colour(action));
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 1, 2), 1, false);
    let style = app.document.paragraphs()[1].style;
    send(&mut app, Message::Command(CommandId::ModifyCurrentStyle));
    let style_colour = |app: &AmiProApp| app.document.styles()[&style].attributes.colour;
    let start = style_colour(&app);
    // The button opens the panel with the document's colours.
    colour(&mut app, ColourAction::Toggle(Target::StyleText, start));
    let panel = app.colour_panel.as_ref().unwrap();
    assert_eq!(panel.target, Target::StyleText);
    assert!(panel.in_use.contains(&start));
    assert!(!panel.more);
    // A swatch applies at once as one undo step, closes the panel, and is
    // Recent's newest.
    let undo = app.history.undo_len();
    colour(&mut app, ColourAction::Choose(0x0000_00FF));
    assert_eq!(style_colour(&app), 0x0000_00FF);
    assert_eq!(app.history.undo_len(), undo + 1);
    assert!(app.colour_panel.is_none());
    assert_eq!(app.preferences.recent_colours.first(), Some(&0x0000_00FF));
    // A drag in the picker shows each colour live and is one undo step.
    colour(
        &mut app,
        ColourAction::Toggle(Target::StyleText, 0x0000_00FF),
    );
    colour(&mut app, ColourAction::More);
    let hsv = |red, green, blue| Hsv::from_color(Srgb::new(red, green, blue));
    let undo = app.history.undo_len();
    for (red, green, blue) in [(0.2, 0.4, 0.6), (0.3, 0.5, 0.7), (0.4, 0.6, 0.8)] {
        let shade = hsv(red, green, blue);
        colour(
            &mut app,
            ColourAction::Picker(ColorPickerUpdate::ActiveColor(shade)),
        );
        assert_eq!(style_colour(&app), crate::colours::from_hsv(shade));
        assert_eq!(app.history.undo_len(), undo + 1, "one step for the drag");
    }
    let dragged = crate::colours::from_hsv(hsv(0.4, 0.6, 0.8));
    colour(
        &mut app,
        ColourAction::Picker(ColorPickerUpdate::ActionFinished),
    );
    assert_eq!(style_colour(&app), dragged);
    assert_eq!(app.history.undo_len(), undo + 1);
    assert!(
        app.colour_panel.as_ref().unwrap().more,
        "the picker stays open"
    );
    assert_eq!(app.preferences.recent_colours.first(), Some(&dragged));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(
        style_colour(&app),
        0x0000_00FF,
        "one undo takes the drag back"
    );
    // A typed colour applies on Enter.
    colour(
        &mut app,
        ColourAction::Picker(ColorPickerUpdate::Input("#336699".into())),
    );
    colour(
        &mut app,
        ColourAction::Picker(ColorPickerUpdate::ActionFinished),
    );
    assert_eq!(style_colour(&app), 0x0099_6633);
    // Recent leaves out what the rows above show and keeps the newest
    // first.
    assert_eq!(
        &app.preferences.recent_colours[..3],
        [0x0099_6633, dragged, 0x0000_00FF]
    );
    // The rules' colour through its own button.
    colour(&mut app, ColourAction::Toggle(Target::StyleRule, 0));
    colour(&mut app, ColourAction::Choose(0x0000_FF00));
    assert_eq!(app.document.styles()[&style].rules.colour, 0x0000_FF00);
    // The panel closes with Style Properties.
    colour(&mut app, ColourAction::Toggle(Target::StyleText, 0));
    send(
        &mut app,
        Message::StyleProperties(crate::style_properties::StyleAction::Close),
    );
    assert!(app.colour_panel.is_none());
    // The Font dialog: the panel sets the dialog's colour; OK applies it.
    move_to(&mut app, at(main, 0, 0), 1, false);
    move_to(&mut app, at(main, 0, 4), 1, true);
    send(&mut app, Message::Command(CommandId::Font));
    colour(&mut app, ColourAction::Toggle(Target::FontText, 0));
    colour(&mut app, ColourAction::Choose(0x00FF_0000));
    assert_eq!(app.font_dialog.as_ref().unwrap().colour, 0x00FF_0000);
    assert_eq!(
        app.history.undo_len(),
        undo + 2,
        "nothing applied before OK"
    );
    send(
        &mut app,
        Message::FontDialog(crate::character_dialogs::FontAction::Apply),
    );
    assert!(app.font_dialog.is_none() && app.colour_panel.is_none());
    let run = app.document.paragraphs()[0].runs[0].format.colour;
    assert_eq!(run, Some(0x00FF_0000));
    draw_window(&app);
}

/// Screenshots of a colour panel (`UI_SPEC.md` §23), light and dark, when
/// `AMIPRO_SCREENSHOT_DIR` is set: Style Properties' text colour with the
/// panel and More colours open, after a few colours were chosen.
#[test]
fn colour_screenshots() {
    use crate::colour_control::{ColourAction, Target};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    send(&mut app, Message::WindowResized(1_280.0));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 1, 2), 1, false);
    send(&mut app, Message::Command(CommandId::ModifyCurrentStyle));
    for colour in [0x0099_6633, 0x0033_66CC, 0x0000_00FF] {
        send(
            &mut app,
            Message::Colour(ColourAction::Toggle(Target::StyleText, 0)),
        );
        send(&mut app, Message::Colour(ColourAction::Choose(colour)));
    }
    send(
        &mut app,
        Message::Colour(ColourAction::Toggle(Target::StyleText, 0x0000_00FF)),
    );
    send(&mut app, Message::Colour(ColourAction::More));
    let write = |app: &AmiProApp, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 900);
        let rgb = render_window(app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    write(&app, "window-colour-light", false);
    write(&app, "window-colour-dark", true);
}

#[test]
fn frame_colours_are_chosen_with_colour_buttons_all_four() {
    // Ami Pro's Modify Frame Layout has four colour bars: line, shadow,
    // background, and the line between columns (`UI_SPEC.md` §23).
    use crate::colour_control::{ColourAction, Target};
    use crate::frame_editing::FrameAction;
    use crate::frame_inspector::{Choice, Field, NORMAL_SHADOW};
    let mut app = page_app(page_fixture());
    let frame = text_frame(&app.document);
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    let style = |app: &AmiProApp| app.document.frame(frame).unwrap().style.clone();
    let choose = |app: &mut AmiProApp, target: Target, colour| {
        send(app, Message::Colour(ColourAction::Toggle(target, 0)));
        send(app, Message::Colour(ColourAction::Choose(colour)));
    };
    let undo = app.history.undo_len();
    choose(&mut app, Target::FrameLine, 0x0000_00FF);
    assert_eq!(style(&app).border.colour, 0x0000_00FF);
    assert_eq!(app.history.undo_len(), undo + 1);
    choose(&mut app, Target::FrameBackground, 0x00C0_FFFF);
    assert_eq!(style(&app).background, 0x00C0_FFFF);
    send(
        &mut app,
        Message::InspectorChoice(Choice::ShadowDepth(Some(NORMAL_SHADOW))),
    );
    choose(&mut app, Target::FrameShadow, 0x0080_8080);
    assert_eq!(style(&app).shadow.colour, 0x0080_8080);
    // Two columns with a line between them: its own colour, the frame's
    // line kept.
    send(
        &mut app,
        Message::InspectorInput(Field::Columns, "2".into()),
    );
    send(&mut app, Message::InspectorSubmit(Field::Columns));
    send(
        &mut app,
        Message::InspectorChoice(Choice::LineBetween(true)),
    );
    choose(&mut app, Target::FrameColumnLine, 0x0000_8000);
    let chosen = style(&app);
    assert_eq!(chosen.interior.line_between.unwrap().colour, 0x0000_8000);
    assert_eq!(chosen.border.colour, 0x0000_00FF);
    // The four colours read back from SAM and the native format.
    let options = amipro_formats::SamOptions::at(0);
    let written = amipro_formats::sam_document(&app.document, &options);
    let copy = amipro_formats::import_sam(&written, None, "Colours")
        .unwrap()
        .document;
    let read = copy
        .frames()
        .find(|candidate| candidate.style.background == 0x00C0_FFFF)
        .expect("the frame read back")
        .style
        .clone();
    assert_eq!(
        (
            read.border.colour,
            read.shadow.colour,
            read.interior.line_between.map(|line| line.colour)
        ),
        (0x0000_00FF, 0x0080_8080, Some(0x0000_8000))
    );
    // Nothing about the frame in the loss check (the fixture's Chinese text
    // is the only loss: SAM is Windows-1252).
    let losses = amipro_formats::sam_losses(&app.document, &options).unwrap();
    assert!(
        losses
            .iter()
            .all(|loss| loss.kind == amipro_formats::roundtrip::LossKind::Characters),
        "{losses:?}"
    );
    let native = Document::decode_native(&app.document.encode_native().unwrap()).unwrap();
    assert_eq!(native.frame(frame).unwrap().style, chosen);
    // The panel closes when the frame is no longer selected.
    send(
        &mut app,
        Message::Colour(ColourAction::Toggle(Target::FrameLine, 0)),
    );
    frame_action(&mut app, FrameAction::Deselect);
    assert!(app.colour_panel.is_none());
}

/// Colours mixed in the app for Ami Pro to show (`UI_SPEC.md` §23, CS.6):
/// on the VM's OL3B (`AMIPRO_VM_LAB`), One's text `#336699` typed in the
/// Font dialog's picker, Title's rule below in `#CC6600`, a frame with a
/// `#996633` line, a grey shadow, a `#FFF0C0` background, and a `#008000`
/// line between its two columns, and the page's lines in `#663399`; written
/// as `ACS1.SAM` to `AMIPRO_TWIN_OUT`.
#[test]
#[ignore = "needs the VM lab's files"]
fn colour_twins() {
    use crate::colour_control::{ColourAction, Target};
    use crate::frame_editing::FrameAction;
    use crate::frame_inspector::{Choice, Field, NORMAL_SHADOW};
    use crate::page_layout_properties::{LayoutAction, LineSide};
    use cosmic::widget::color_picker::ColorPickerUpdate;
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let mut app = page_app(
        amipro_formats::import_sam_path_with_style_directories(
            lab.join("OL3B.SAM"),
            &[lab.join("../STY")],
        )
        .unwrap()
        .document,
    );
    let main = app.document.main_flow_id();
    let colour = |app: &mut AmiProApp, action: ColourAction| send(app, Message::Colour(action));
    let choose = |app: &mut AmiProApp, target: Target, value| {
        colour(app, ColourAction::Toggle(target, 0));
        colour(app, ColourAction::Choose(value));
    };
    // One's text, typed in the Font dialog's picker.
    move_to(&mut app, at(main, 1, 0), 1, false);
    move_to(&mut app, at(main, 1, 3), 1, true);
    send(&mut app, Message::Command(CommandId::Font));
    colour(&mut app, ColourAction::Toggle(Target::FontText, 0));
    colour(&mut app, ColourAction::More);
    colour(
        &mut app,
        ColourAction::Picker(ColorPickerUpdate::Input("#336699".into())),
    );
    colour(
        &mut app,
        ColourAction::Picker(ColorPickerUpdate::ActionFinished),
    );
    send(
        &mut app,
        Message::FontDialog(crate::character_dialogs::FontAction::Apply),
    );
    // Title's rule below.
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::ModifyCurrentStyle));
    send(
        &mut app,
        Message::StyleProperties(crate::style_properties::StyleAction::Rule(false, true)),
    );
    choose(&mut app, Target::StyleRule, 0x0000_66CC);
    send(
        &mut app,
        Message::StyleProperties(crate::style_properties::StyleAction::Close),
    );
    // A frame's four colours.
    app.create_frame(1, amipro_document::Rect::new(2_880, 7_200, 7_200, 9_360));
    let frame = *app.document.frame_stack().last().unwrap();
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    choose(&mut app, Target::FrameLine, 0x0033_6699);
    choose(&mut app, Target::FrameBackground, 0x00C0_F0FF);
    send(
        &mut app,
        Message::InspectorChoice(Choice::ShadowDepth(Some(NORMAL_SHADOW))),
    );
    choose(&mut app, Target::FrameShadow, 0x0080_8080);
    send(
        &mut app,
        Message::InspectorInput(Field::Columns, "2".into()),
    );
    send(&mut app, Message::InspectorSubmit(Field::Columns));
    send(
        &mut app,
        Message::InspectorChoice(Choice::LineBetween(true)),
    );
    choose(&mut app, Target::FrameColumnLine, 0x0000_8000);
    frame_action(&mut app, FrameAction::Deselect);
    // The page's lines.
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::PageLayout));
    send(
        &mut app,
        Message::PageLayoutProperties(LayoutAction::LineSide(LineSide::All, true)),
    );
    choose(&mut app, Target::PageLines, 0x0099_3366);
    let document = &app.document;
    assert_eq!(
        document.paragraphs()[1].runs[0].format.colour,
        Some(0x0099_6633)
    );
    let options = amipro_formats::SamOptions::at(0);
    assert_eq!(amipro_formats::sam_losses(document, &options).unwrap(), []);
    if let Some(out) = std::env::var_os("AMIPRO_TWIN_OUT") {
        let path = std::path::PathBuf::from(out).join("ACS1.SAM");
        std::fs::write(&path, amipro_formats::sam_document(document, &options)).unwrap();
        eprintln!("wrote {}", path.display());
    }
}

/// What a colour panel costs on a long document (`AMIPRO_PERF_SAM`,
/// README31): opening one (the document's colours worked out) and choosing
/// a colour (applied, then worked out again), release build.
#[test]
#[ignore = "release-mode performance baseline"]
fn colour_panel_performance_baseline() {
    use crate::colour_control::{ColourAction, Target};
    let Some(path) = std::env::var_os("AMIPRO_PERF_SAM") else {
        eprintln!("AMIPRO_PERF_SAM is not set; skipping");
        return;
    };
    let document = amipro_formats::import_sam_path(path).unwrap().document;
    let start = Instant::now();
    let colours = crate::colours::document_colours(&document);
    eprintln!(
        "document colours: {:.3} ms ({} paragraphs, {} colours)",
        start.elapsed().as_secs_f64() * 1_000.0,
        document.paragraphs().len(),
        colours.len()
    );
    let mut app = page_app(document);
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 3, 0), 1, false);
    send(&mut app, Message::Command(CommandId::ModifyCurrentStyle));
    let mut open = Vec::new();
    let mut choose = Vec::new();
    for round in 0..20u32 {
        let start = Instant::now();
        send(
            &mut app,
            Message::Colour(ColourAction::Toggle(Target::StyleText, 0)),
        );
        open.push(start.elapsed());
        let start = Instant::now();
        send(
            &mut app,
            Message::Colour(ColourAction::Choose(0x0010_2030 + round)),
        );
        choose.push(start.elapsed());
    }
    let median = |times: &mut Vec<Duration>| {
        times.sort();
        times[times.len() / 2].as_secs_f64() * 1_000.0
    };
    eprintln!(
        "open a panel: median {:.3} ms; choose a colour: median {:.3} ms",
        median(&mut open),
        median(&mut choose)
    );
}

/// A document with one Where placed picture frame on page 1: opaque, cyan,
/// without lines or shadow, holding a bitmap of `width` × `height` magenta
/// pixels sized by `scaling` and moved by `pan` (`UI_SPEC.md` §25).
pub(crate) fn magenta_picture_document(
    width: u32,
    height: u32,
    scaling: amipro_document::PictureScaling,
    pan: amipro_document::Offset,
) -> Document {
    let mut document = Document::from_plain_text("Text before the picture.");
    let raster = amipro_pictures::Raster::filled(width, height, [255, 0, 255, 255]).unwrap();
    let bytes = amipro_pictures::dib::encode_bmp24(&raster, (0, 0));
    let object = document.add_object(std::sync::Arc::from(bytes));
    let mut picture = amipro_document::Picture::new(
        amipro_document::PictureSource::Embedded(object),
        ".bmp",
        scaling.size,
    );
    picture.scaling = scaling;
    picture.pan = pan;
    let frame = document.allocate_frame_id();
    let mut style = amipro_document::FrameStyle {
        background: 0x00FF_FF00,
        ..amipro_document::FrameStyle::default()
    };
    style.border.sides = amipro_document::Sides::NONE;
    document.insert_frame(amipro_document::Frame {
        id: frame,
        placement: amipro_document::Placement::Fixed {
            page: 1,
            rect: amipro_document::Rect::new(2_880, 2_880, 5_760, 5_760),
        },
        style,
        group: None,
        content: FrameContent::Picture(picture),
        legacy: amipro_document::LegacyData::default(),
    });
    document.frame_stack_mut().push(frame);
    document.validate().unwrap();
    document
}

/// The bounding box (left, top, right, bottom) of the screenshot's pixels
/// of exactly `colour`, if any.
fn colour_box(rgb: &[u8], width: usize, colour: [u8; 3]) -> Option<(usize, usize, usize, usize)> {
    let mut found: Option<(usize, usize, usize, usize)> = None;
    for (index, pixel) in rgb.as_chunks::<3>().0.iter().enumerate() {
        if *pixel == colour {
            let (x, y) = (index % width, index / width);
            found = Some(match found {
                None => (x, y, x, y),
                Some((l, t, r, b)) => (l.min(x), t.min(y), r.max(x), b.max(y)),
            });
        }
    }
    found
}

/// `UI_SPEC.md` §25 through the real window: a picture is drawn in its
/// frame's picture area at the size its scaling gives, kept to the area
/// whatever its size and pan (PI5, PI8), and keeps its aspect ratio when
/// fitted (PI1).
#[test]
fn pictures_are_drawn_in_their_frames_clipped_to_the_picture_area() {
    use amipro_document::{Offset, PictureScaling, PictureSizing, Size};
    let shot = |document: Document| {
        let mut app = page_app(document);
        send(&mut app, Message::ZoomTo(100));
        let mut runtime = Runtime::new(app);
        runtime.redraw();
        runtime.screenshot(&cosmic::Theme::light())
    };
    let width = 1_280;
    // Far larger than the frame and moved up and left: it fills the
    // picture area and nothing outside it.
    let huge = PictureScaling {
        sizing: PictureSizing::Custom,
        keep_aspect: false,
        percent: 100,
        size: Size {
            width: 31_680,
            height: 31_680,
        },
        rotation: 0,
    };
    let rgb = shot(magenta_picture_document(
        2,
        2,
        huge,
        Offset {
            x: -1_000,
            y: -1_000,
        },
    ));
    let frame = colour_box(&rgb, width, [0, 255, 255]).expect("the frame's background");
    let picture = colour_box(&rgb, width, [255, 0, 255]).expect("the picture");
    assert!(
        picture.0 > frame.0 && picture.1 > frame.1 && picture.2 < frame.2 && picture.3 < frame.3,
        "the picture {picture:?} stays inside the frame {frame:?}, in its margins"
    );
    // The 0.10 in margins are about 10 pixels at 100 %.
    assert!(
        (picture.0 - frame.0).abs_diff(10) <= 2,
        "{picture:?} {frame:?}"
    );

    // A 2 × 1 bitmap fitted with its aspect kept in a square area is half
    // as high as wide.
    let rgb = shot(magenta_picture_document(
        20,
        10,
        PictureScaling::imported(Size {
            width: 1,
            height: 1,
        }),
        Offset::default(),
    ));
    let (left, top, right, bottom) = colour_box(&rgb, width, [255, 0, 255]).expect("the picture");
    let (wide, high) = (right - left + 1, bottom - top + 1);
    assert!(wide.abs_diff(2 * high) <= 2, "{wide} × {high}");
}

/// A 120 × 80 bitmap without a resolution (VM PIC1's shape) in a temporary
/// folder: 1,800 × 1,200 twips to Ami Pro.
fn picture_file(name: &str) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("amipro-insert-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let raster = amipro_pictures::Raster::filled(120, 80, [255, 0, 255, 255]).unwrap();
    let path = folder.join(name);
    std::fs::write(&path, amipro_pictures::dib::encode_bmp24(&raster, (0, 0))).unwrap();
    path
}

fn chosen(app: &mut AmiProApp, path: &std::path::Path, link: bool) {
    let session_id = app.id;
    send(
        app,
        Message::ImageChosen {
            session_id,
            result: FileDialogResult::Selected(path.to_owned()),
            link,
        },
    );
}

/// Insert › Image… with the caret in text (`UI_SPEC.md` §25, P1–P3): a
/// Flow with text frame at the caret the picture's size inside Ami Pro's
/// default frame, Fit in frame with the aspect ratio kept, selected, one
/// undo step; with Link to file, the path is kept instead of the bytes.
#[test]
fn insert_image_puts_a_picture_in_a_new_frame_at_the_caret() {
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    assert!(app.command_enabled(CommandId::Image));
    let undo = app.history.undo_len();
    let path = picture_file("pic1.bmp");
    chosen(&mut app, &path, false);
    assert_eq!(app.history.undo_len(), undo + 1, "one undo step");
    let selected = app
        .frame_selection
        .as_ref()
        .expect("the frame is selected")
        .frames[0];
    let frame = app.document.frame(selected).unwrap();
    let amipro_document::Placement::Inline { size, y_offset } = frame.placement else {
        panic!("{:?}", frame.placement);
    };
    assert_eq!(y_offset, 0);
    let FrameContent::Picture(picture) = &frame.content else {
        panic!("a picture");
    };
    assert!(matches!(
        picture.source,
        amipro_document::PictureSource::Embedded(_)
    ));
    assert_eq!(
        picture.scaling.sizing,
        amipro_document::PictureSizing::FitInFrame
    );
    let insets = frame.style.text_insets(size.width, size.height);
    assert_eq!(
        (
            size.width - insets.left - insets.right,
            size.height - insets.top - insets.bottom
        ),
        (1_800, 1_200),
        "the picture area is the picture's own size (P3)"
    );
    // The anchor is at the caret.
    assert!(
        app.document.paragraphs()[0]
            .objects
            .contains(&amipro_document::InlineObject::FrameAnchor(selected))
    );
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(app.document.frame(selected).is_none());

    // Linked (P2).
    send(
        &mut app,
        Message::Page(PageAction::MoveTo {
            position: at(main, 0, 3),
            page: 1,
            extend: false,
        }),
    );
    chosen(&mut app, &path, true);
    let selected = app.frame_selection.as_ref().unwrap().frames[0];
    let FrameContent::Picture(picture) = &app.document.frame(selected).unwrap().content else {
        panic!("a picture");
    };
    assert_eq!(
        picture.source,
        amipro_document::PictureSource::Linked(path.to_string_lossy().into_owned())
    );
    // With the picture frame selected, Insert › Image… replaces its
    // picture (Ami Pro has no replace; PI9).
    assert!(app.command_enabled(CommandId::Image));
    chosen(&mut app, &path, false);
    let FrameContent::Picture(picture) = &app.document.frame(selected).unwrap().content else {
        panic!("a picture");
    };
    assert!(matches!(
        picture.source,
        amipro_document::PictureSource::Embedded(_)
    ));
    // A file no reader takes is refused.
    let text = path.with_file_name("not-a-picture.png");
    std::fs::write(&text, b"plain text").unwrap();
    let before = app.history.undo_len();
    chosen(&mut app, &text, false);
    assert_eq!(app.history.undo_len(), before);
    assert_eq!(app.status, "Insert Image could not read this picture");
}

/// Insert › Image… into an empty cell (`14d0:05ec`, T3) as a frame of Ami
/// Pro's cell style; refused in a cell holding a picture or text (TF12).
#[test]
fn insert_image_puts_a_picture_in_an_empty_cell() {
    let (document, table, _, flows) = crate::page_surface::cell_picture_fixture();
    let mut app = page_app(document);
    move_to(&mut app, at(flows[1], 0, 0), 1, false);
    assert!(
        !app.command_enabled(CommandId::Image),
        "the cell holds a picture"
    );
    move_to(&mut app, at(flows[0], 0, 0), 1, false);
    assert!(app.command_enabled(CommandId::Image));
    chosen(&mut app, &picture_file("cell.bmp"), false);
    let FrameContent::Table(grid) = &app.document.frame(table).unwrap().content else {
        panic!("a table");
    };
    let frame = grid.cells[0].frame.expect("the cell holds the picture");
    let frame = app.document.frame(frame).unwrap();
    assert_eq!(frame.placement, amipro_document::Placement::InCell);
    assert!(frame.style.border.sides == amipro_document::Sides::NONE);
    assert!(
        app.frame_selection.is_none(),
        "a cell's picture is not selected (T1)"
    );
}

/// Frame Properties › Picture (`UI_SPEC.md` §25, P5, P7) through the app's
/// messages: Format › Frame ▸ Graphics Scaling… opens it for a picture
/// frame; a typed width makes the picture Custom with its aspect kept, one
/// undo step; moving, Reset, and Fit Frame to Picture are one step each;
/// the window still lays out.
#[test]
fn the_picture_section_sizes_moves_and_resets_the_picture() {
    use crate::frame_editing::FrameAction;
    use crate::frame_inspector::{Choice, Field};
    use amipro_document::{Offset, PictureScaling, PictureSizing, Size};
    let document = magenta_picture_document(
        20,
        10,
        PictureScaling::imported(Size {
            width: 1,
            height: 1,
        }),
        Offset::default(),
    );
    let frame = document.frame_stack()[0];
    let mut app = page_app(document);
    let picture = |app: &AmiProApp| match &app.document.frame(frame).unwrap().content {
        FrameContent::Picture(picture) => picture.clone(),
        _ => panic!("a picture"),
    };
    assert!(!app.command_enabled(CommandId::GraphicsScaling));
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    assert!(app.command_enabled(CommandId::GraphicsScaling));
    send(&mut app, Message::Command(CommandId::GraphicsScaling));
    assert!(app.styles_visible, "the rail shows Frame Properties");
    assert_eq!(draw_window(&app), cosmic::iced::Size::new(1_280.0, 800.0));

    let steps = app.history.undo_len();
    send(
        &mut app,
        Message::InspectorInput(Field::PictureWidth, "1".into()),
    );
    send(&mut app, Message::InspectorSubmit(Field::PictureWidth));
    let scaling = picture(&app).scaling;
    assert_eq!(scaling.sizing, PictureSizing::Custom);
    assert_eq!(
        scaling.size,
        Size {
            width: 1_440,
            height: 720
        },
        "the 2 × 1 aspect kept"
    );
    assert_eq!(app.history.undo_len(), steps + 1);

    send(
        &mut app,
        Message::InspectorInput(Field::PictureDown, "0.05".into()),
    );
    send(&mut app, Message::InspectorSubmit(Field::PictureDown));
    assert_eq!(picture(&app).pan, Offset { x: 0, y: 72 });
    assert_eq!(app.history.undo_len(), steps + 2);

    // A refused rotation keeps the picture and says why.
    send(
        &mut app,
        Message::InspectorInput(Field::PictureRotation, "400".into()),
    );
    send(&mut app, Message::InspectorSubmit(Field::PictureRotation));
    assert_eq!(picture(&app).scaling.rotation, 0);
    assert!(app.inspector_error.is_some());

    // Reset: Fit in frame, aspect kept, not moved, in one step.
    send(&mut app, Message::InspectorChoice(Choice::PictureReset));
    let reset = picture(&app);
    assert_eq!(reset.scaling.sizing, PictureSizing::FitInFrame);
    assert_eq!(reset.pan, Offset::default());
    assert_eq!(app.history.undo_len(), steps + 3);

    // Fit Frame to Picture: the frame takes the picture's size and its
    // margins, in one step.
    send(
        &mut app,
        Message::InspectorChoice(Choice::PictureSizing(PictureSizing::Original)),
    );
    send(&mut app, Message::InspectorChoice(Choice::PictureFitFrame));
    let amipro_document::Placement::Fixed { rect, .. } =
        app.document.frame(frame).unwrap().placement
    else {
        panic!("Where placed");
    };
    // 20 × 10 pixels at 96 dpi are 300 × 150 twips; 0.10 in margins.
    assert_eq!((rect.width(), rect.height()), (300 + 288, 150 + 288));
    assert_eq!(app.history.undo_len(), steps + 5);
    send(&mut app, Message::Command(CommandId::Undo));
    let amipro_document::Placement::Fixed { rect, .. } =
        app.document.frame(frame).unwrap().placement
    else {
        panic!("Where placed");
    };
    assert_eq!(rect.width(), 2_880);
    assert_eq!(draw_window(&app), cosmic::iced::Size::new(1_280.0, 800.0));
}

/// Screenshots of Frame Properties › Picture (`UI_SPEC.md` §25), light and
/// dark, for review: `AMIPRO_SCREENSHOT_DIR` receives
/// `window-picture-rail-light.ppm` and `-dark.ppm`.
#[test]
#[ignore = "writes screenshots of the picture section"]
fn picture_rail_screenshots() {
    use crate::frame_editing::FrameAction;
    use amipro_document::{Offset, PictureScaling, Size};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let document = magenta_picture_document(
        20,
        10,
        PictureScaling::imported(Size {
            width: 1,
            height: 1,
        }),
        Offset::default(),
    );
    let frame = document.frame_stack()[0];
    let mut app = page_app(document);
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    send(&mut app, Message::Command(CommandId::GraphicsScaling));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    for (name, theme) in [
        ("light", cosmic::Theme::light()),
        ("dark", cosmic::Theme::dark()),
    ] {
        let rgb = runtime.screenshot(&theme);
        let size = runtime.window;
        let mut ppm = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
        ppm.extend(rgb);
        let path = std::path::Path::new(&directory).join(format!("window-picture-rail-{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
    }
}

/// Picture mode through the app's messages (`UI_SPEC.md` §25, P6): Enter on
/// a selected picture frame starts it with its status note; a drag's move
/// is one undo step, a run of arrow presses one more; Esc leaves it with the
/// frame still selected; Shift and a corner handle size the picture with
/// the frame (Custom), one step.
#[test]
fn picture_mode_moves_the_picture_in_its_frame() {
    use crate::frame_editing::FrameAction;
    use amipro_document::{Offset, PictureScaling, PictureSizing, Size};
    let document = magenta_picture_document(
        20,
        10,
        PictureScaling::imported(Size {
            width: 1,
            height: 1,
        }),
        Offset::default(),
    );
    let frame = document.frame_stack()[0];
    let mut app = page_app(document);
    let picture = |app: &AmiProApp| match &app.document.frame(frame).unwrap().content {
        FrameContent::Picture(picture) => picture.clone(),
        _ => panic!("a picture"),
    };
    frame_action(
        &mut app,
        FrameAction::Select {
            frame,
            page: 1,
            extend: false,
        },
    );
    frame_action(&mut app, FrameAction::EnterText { at: None });
    assert_eq!(app.picture_mode, Some(frame));
    assert!(app.status.starts_with("Drag the picture"));
    let steps = app.history.undo_len();
    frame_action(
        &mut app,
        FrameAction::Pan {
            dx: 300,
            dy: -100,
            key: false,
        },
    );
    assert_eq!(picture(&app).pan, Offset { x: 300, y: -100 });
    assert_eq!(app.history.undo_len(), steps + 1);
    for _ in 0..3 {
        frame_action(
            &mut app,
            FrameAction::Pan {
                dx: 72,
                dy: 0,
                key: true,
            },
        );
    }
    assert_eq!(picture(&app).pan, Offset { x: 516, y: -100 });
    assert_eq!(
        app.history.undo_len(),
        steps + 2,
        "a run of arrow presses is one step"
    );
    frame_action(&mut app, FrameAction::LeavePicture);
    assert_eq!(app.picture_mode, None);
    assert!(app.frame_selection.is_some(), "the frame stays selected");

    // Shift and a corner handle: the frame doubles, so does the picture.
    let amipro_document::Placement::Fixed { rect: from, .. } =
        app.document.frame(frame).unwrap().placement
    else {
        panic!("Where placed");
    };
    let to = amipro_document::Rect::new(
        from.left,
        from.top,
        from.left + 2 * from.width(),
        from.top + 2 * from.height(),
    );
    frame_action(
        &mut app,
        FrameAction::Resize {
            from,
            to,
            with_picture: true,
        },
    );
    let after = picture(&app).scaling;
    assert_eq!(after.sizing, PictureSizing::Custom);
    assert_eq!(app.history.undo_len(), steps + 3);
    // The fitted picture (the 2 × 1 bitmap across the 2,592-twip area) is
    // twice as large.
    assert_eq!(
        after.size,
        Size {
            width: 5_184,
            height: 2_592
        }
    );
}

/// Paste and drop (`UI_SPEC.md` §25, P7): an image on the clipboard pastes
/// as a picture at the caret, kept as PNG, and enables Paste without text;
/// a picture file dropped on the page goes in at the caret the drop moved.
#[test]
fn pasted_and_dropped_pictures_go_in_as_insert_image_puts_them() {
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    // A PNG of 30 × 20 pixels.
    let mut png = Vec::new();
    image::RgbaImage::from_pixel(30, 20, image::Rgba([0, 128, 255, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    app.clipboard_has_text = Some(false);
    send(
        &mut app,
        Message::ClipboardImageAvailability {
            has_image: true,
            retry: false,
        },
    );
    assert!(
        app.command_enabled(CommandId::Paste),
        "an image alone enables Paste"
    );
    let session_id = app.id;
    let steps = app.history.undo_len();
    send(
        &mut app,
        Message::ClipboardImage {
            session_id,
            image: Some(picture_insert::ClipboardImage {
                bytes: png.clone(),
                mime: "image/png".into(),
            }),
            text: None,
        },
    );
    assert_eq!(app.history.undo_len(), steps + 1);
    let pasted = app.frame_selection.as_ref().expect("selected").frames[0];
    let FrameContent::Picture(picture) = &app.document.frame(pasted).unwrap().content else {
        panic!("a picture");
    };
    assert_eq!(picture.extension, ".png");
    assert_eq!(picture.format(), amipro_document::PictureFormat::Png);

    // A browser's Copy Image also leaves the image's address as text
    // (user, 2026-10-18): Paste looks for the image before pasting it.
    move_to(&mut app, at(main, 1, 0), 1, false);
    let text = app.document.paragraphs()[1].text.clone();
    let address = "https://example.com/images/logo.png".to_owned();
    let task = cosmic::Application::update(
        &mut app,
        Message::ClipboardRead {
            command: CommandId::Paste,
            session_id,
            contents: Some(address.clone()),
        },
    );
    assert!(
        cosmic::iced::runtime::task::into_stream(task).is_some(),
        "the image is read"
    );
    assert_eq!(app.document.paragraphs()[1].text, text, "the address waits");
    let pictures = |app: &AmiProApp| {
        app.document
            .frames()
            .filter(|frame| matches!(frame.content, FrameContent::Picture(_)))
            .count()
    };
    let before = pictures(&app);
    send(
        &mut app,
        Message::ClipboardImage {
            session_id,
            image: Some(picture_insert::ClipboardImage {
                bytes: png.clone(),
                mime: "image/png".into(),
            }),
            text: Some(address.clone()),
        },
    );
    assert_eq!(pictures(&app), before + 1, "the image, not its address");
    assert!(!app.document.paragraphs()[1].text.contains(&address));
    // Without an image the address is pasted as text.
    app.frame_selection = None;
    move_to(&mut app, at(main, 1, 0), 1, false);
    send(
        &mut app,
        Message::ClipboardImage {
            session_id,
            image: None,
            text: Some(address.clone()),
        },
    );
    assert!(app.document.paragraphs()[1].text.starts_with(&address));
    // A picture file copied in a file manager pastes as a drop puts it.
    let copied = picture_file("copied.bmp");
    let before = pictures(&app);
    send(
        &mut app,
        Message::ClipboardImage {
            session_id,
            image: Some(picture_insert::ClipboardImage {
                bytes: format!("file://{}\r\n", copied.display()).into_bytes(),
                mime: "text/uri-list".into(),
            }),
            text: Some(copied.display().to_string()),
        },
    );
    assert_eq!(pictures(&app), before + 1, "the copied file");

    // A drop: the file's URI, the caret already moved by the drop.
    send(
        &mut app,
        Message::Page(PageAction::MoveTo {
            position: at(main, 0, 0),
            page: 1,
            extend: false,
        }),
    );
    let path = picture_file("dropped pic.bmp");
    let uri = format!("file://{}\r\n", path.display()).replace(' ', "%20");
    send(
        &mut app,
        Message::Dropped {
            session_id,
            data: uri.into_bytes(),
            mime: "text/uri-list".into(),
        },
    );
    let dropped = app.frame_selection.as_ref().expect("selected").frames[0];
    assert_ne!(dropped, pasted);
    assert!(
        app.document.paragraphs()[0].objects[0]
            == amipro_document::InlineObject::FrameAnchor(dropped),
        "anchored where the caret was"
    );
    // Something that is not a picture is refused.
    let before = app.history.undo_len();
    send(
        &mut app,
        Message::Dropped {
            session_id,
            data: b"file:///tmp/notes.txt\r\n".to_vec(),
            mime: "text/uri-list".into(),
        },
    );
    assert_eq!(app.history.undo_len(), before);
    // An image a browser drags comes as the image.
    send(
        &mut app,
        Message::Dropped {
            session_id,
            data: png,
            mime: "image/png".into(),
        },
    );
    assert_eq!(app.history.undo_len(), before + 1);
}

/// Paste's state after a copy in another program (user, 2026-10-18: the
/// right-click menu's Paste was greyed out while Edit › Paste worked): the
/// right button looks at the clipboard before its menu opens, a first look
/// that finds nothing is tried once more a moment later, and Paste reads
/// the clipboard even when its state says it is empty.
#[test]
fn paste_looks_at_the_clipboard_again() {
    use cosmic::iced::futures::StreamExt as _;
    let mut app = page_app(page_fixture());
    let update = |app: &mut AmiProApp, message| {
        cosmic::iced::runtime::task::into_stream(cosmic::Application::update(app, message))
    };
    assert!(
        update(
            &mut app,
            Message::Page(PageAction::ContextAt { margin: false })
        )
        .is_some(),
        "the right button looks"
    );
    let mut retry = update(
        &mut app,
        Message::ClipboardImageAvailability {
            has_image: false,
            retry: true,
        },
    )
    .expect("a second look");
    let action = cosmic::iced::futures::executor::block_on(retry.next());
    assert!(matches!(
        action,
        Some(cosmic::iced::runtime::Action::Output(cosmic::Action::App(
            Message::RetryClipboard
        )))
    ));
    assert!(
        update(
            &mut app,
            Message::ClipboardImageAvailability {
                has_image: false,
                retry: false,
            },
        )
        .is_none(),
        "only one more"
    );
    assert!(update(&mut app, Message::RetryClipboard).is_some());
    app.clipboard_has_text = Some(false);
    assert!(!app.command_enabled(CommandId::Paste), "shown as empty");
    assert!(
        update(&mut app, Message::Command(CommandId::Paste)).is_some(),
        "Paste reads the clipboard all the same"
    );
}

/// A picture file dropped on the page through the whole window, as the
/// runtime delivers a drop (P7): the drop moves the caret, which rebuilds
/// the view before the file's data arrives with the id the drop had, so
/// the drop target must keep its id across views (the tree diff keeps
/// it). It takes a browser's image before files, and asks for a copy,
/// never a move.
#[test]
fn a_file_dropped_on_the_page_goes_in_through_the_window() {
    use cosmic::iced::clipboard::dnd::{DndAction, DndEvent, OfferEvent};
    let mut runtime = Runtime::new(page_app(page_fixture()));
    let target = |runtime: &mut Runtime| {
        let ui = cosmic::iced::runtime::user_interface::UserInterface::build(
            cosmic::ApplicationExt::view_main(&runtime.app),
            runtime.window,
            runtime.cache.take().expect("the interface's cache"),
            &mut runtime.renderer,
        );
        let rectangles = ui.dnd_rectangles(0, &runtime.renderer).into_rectangles();
        runtime.cache = Some(ui.into_cache());
        rectangles
            .into_iter()
            .find(|rectangle| {
                rectangle
                    .mime_types
                    .iter()
                    .any(|mime| mime == "text/uri-list")
            })
            .expect("the page's drop target")
    };
    let registered = target(&mut runtime);
    assert_eq!(registered.mime_types[0], "image/png");
    assert_eq!(registered.actions, DndAction::Copy);
    assert_eq!(registered.preferred, DndAction::Copy);
    let id = Some(registered.id);
    let point = runtime.on_page(2_880.0, 2_000.0);
    let area = &registered.rectangle;
    let (x, y) = (f64::from(point.x), f64::from(point.y));
    assert!(area.x <= x && x <= area.x + area.width && area.y <= y && y <= area.y + area.height);
    let pictures = |app: &AmiProApp| {
        app.document
            .frames()
            .filter(|frame| matches!(frame.content, FrameContent::Picture(_)))
            .count()
    };
    let before = pictures(&runtime.app);
    let path = picture_file("dropped through the window.bmp");
    let uri = format!("file://{}\r\n", path.display()).replace(' ', "%20");
    let offer = |event| cosmic::iced::Event::Dnd(DndEvent::Offer(id, event));
    let published = runtime.events(&[
        (
            offer(OfferEvent::Motion {
                x: f64::from(point.x),
                y: f64::from(point.y),
            }),
            point,
        ),
        (offer(OfferEvent::Drop), point),
    ]);
    assert!(
        published
            .iter()
            .any(|message| matches!(message, Message::Page(PageAction::MoveTo { .. }))),
        "the drop moves the caret: {published:?}"
    );
    assert_eq!(
        target(&mut runtime).id,
        registered.id,
        "one id across views"
    );
    let published = runtime.events(&[(
        offer(OfferEvent::Data {
            data: uri.into_bytes(),
            mime_type: "text/uri-list".into(),
        }),
        point,
    )]);
    assert!(
        published
            .iter()
            .any(|message| matches!(message, Message::Dropped { .. })),
        "{published:?}"
    );
    assert_eq!(pictures(&runtime.app), before + 1);
}

/// Save As SAM with pictures Ami Pro cannot hold (P8, P2): the loss check
/// names the conversion and the link; the file holds a 24-bit bitmap of at
/// most 300 dpi at the picture's size on the page, with Ami Pro's natural
/// size kept by its resolution, and the linked file copied in.
#[test]
fn save_as_sam_converts_pictures_ami_pro_cannot_hold() {
    use amipro_document::{FrameContent, PictureFormat, PictureSource};
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    // A 2,400 × 1,600 photo without a resolution: 21 × 14 in (Ami Pro's
    // largest at 96 dpi), shown in the column, so reduced.
    let folder = std::env::temp_dir().join(format!("amipro-sam-pictures-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut jpeg = Vec::new();
    image::RgbImage::from_pixel(2_400, 1_600, image::Rgb([200, 30, 30]))
        .write_to(
            &mut std::io::Cursor::new(&mut jpeg),
            image::ImageFormat::Jpeg,
        )
        .unwrap();
    let photo = folder.join("photo.jpg");
    std::fs::write(&photo, &jpeg).unwrap();
    chosen(&mut app, &photo, false);
    let photo_frame = app.frame_selection.as_ref().unwrap().frames[0];
    send(
        &mut app,
        Message::Page(PageAction::MoveTo {
            position: at(main, 1, 0),
            page: 1,
            extend: false,
        }),
    );
    chosen(&mut app, &picture_file("linked.bmp"), true);
    let linked_frame = app.frame_selection.as_ref().unwrap().frames[0];

    let options = amipro_formats::SamOptions::at(0);
    let kinds: Vec<_> = amipro_formats::sam_losses(&app.document, &options)
        .unwrap()
        .into_iter()
        .map(|loss| loss.kind)
        .collect();
    assert!(
        kinds.contains(&amipro_formats::LossKind::ConvertedPictures),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&amipro_formats::LossKind::LinkedPictures),
        "{kinds:?}"
    );

    let written = crate::pictures::for_ami_pro(&app.document, None);
    let picture = |document: &Document, frame| match &document.frame(frame).unwrap().content {
        FrameContent::Picture(picture) => picture.clone(),
        _ => panic!("a picture"),
    };
    let converted = picture(&written, photo_frame);
    assert_eq!(converted.format(), PictureFormat::Bitmap);
    let PictureSource::Embedded(object) = converted.source else {
        panic!("embedded");
    };
    let bytes = &written.objects()[&object].bytes;
    let header = amipro_pictures::dib::bmp_header(bytes).unwrap();
    let natural = crate::pictures::natural_of(PictureFormat::Jpeg, &jpeg).unwrap();
    let kept = header.natural_size();
    assert!(
        (kept.width - natural.width).abs() <= 2 && (kept.height - natural.height).abs() <= 2,
        "{kept:?} {natural:?}"
    );
    let frame_width = app
        .document
        .frame(photo_frame)
        .unwrap()
        .placement
        .size()
        .unwrap()
        .width;
    assert!(
        i64::from(header.width) <= i64::from(frame_width) * 300 / 1_440 + 1,
        "{} pixels for {frame_width} twips",
        header.width
    );
    assert!(matches!(
        picture(&written, linked_frame).source,
        PictureSource::Embedded(_)
    ));
    // Written and read back, the copy keeps its pictures and lists nothing
    // more (a converted picture is what Ami Pro reads).
    assert!(
        amipro_formats::sam_losses(&written, &options)
            .unwrap()
            .iter()
            .all(|loss| !matches!(
                loss.kind,
                amipro_formats::LossKind::Pictures
                    | amipro_formats::LossKind::ConvertedPictures
                    | amipro_formats::LossKind::LinkedPictures
            )),
        "nothing more to list for pictures (the fixture's text has characters Windows-1252 \
         lacks)"
    );
    let path = folder.join("CONVERT.SAM");
    let _ = std::fs::remove_file(&path);
    assert!(app.write_sam(&path, 0, false));
    let back = amipro_formats::import_sam_path(&path).unwrap().document;
    assert_eq!(
        back.frames()
            .filter(|frame| matches!(&frame.content,
                FrameContent::Picture(picture) if picture.format() == PictureFormat::Bitmap))
            .count(),
        2
    );
    std::fs::remove_dir_all(&folder).unwrap();
}

/// Twins for the VM (`docs/PICTURES_RUNBOOK.md`, PI.10): Ami Pro's picture
/// fixtures rebuilt in the app and written as Save As SAM writes them, to
/// `AMIPRO_TWIN_OUT`: API1 (PI1: a bitmap at the caret), API2 (PI2: a
/// placeable metafile), API5 (Original size), API7 (Custom 1.00 × 0.25 in,
/// aspect off), API8 (API7 moved 936 right, 144 down), APIRT (turned 90°),
/// APNG and AJPG (converted, P8), APCX and ATIF (Ami Pro's own import
/// filters' formats, converted), ACELL (TF7's empty cell, T3), AFRM (an
/// empty frame selected).
#[test]
#[ignore = "writes the pictures run's twins for the VM"]
fn picture_twins() {
    use crate::frame_editing::FrameAction;
    use crate::frame_inspector::{Choice, Field};
    use amipro_document::PictureSizing;
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let inputs = lab.parent().unwrap().join("IN");
    let out = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from);
    let open = |name: &str| {
        page_app(
            amipro_formats::import_sam_path(lab.join(format!("{name}.SAM")))
                .unwrap()
                .document,
        )
    };
    // PI1 without its picture: its two lines, the caret after line one.
    let base = || {
        let mut app = open("PI1");
        let frame = app.document.frame_stack()[0];
        let commands = [amipro_editing::Command::DeleteFrame { frame }];
        app.sessions[0]
            .execute_frame_commands(&commands, false)
            .unwrap();
        let main = app.document.main_flow_id();
        let end = app.document.paragraphs()[0].text.len();
        move_to(&mut app, at(main, 0, end), 1, false);
        app
    };
    let with = |name: &str| {
        let mut app = base();
        chosen(&mut app, &inputs.join(name), false);
        app
    };
    let scale = |app: &mut AmiProApp, edits: &[(Field, &str)], choices: &[Choice]| {
        for choice in choices {
            send(app, Message::InspectorChoice(*choice));
        }
        for (field, text) in edits {
            send(app, Message::InspectorInput(*field, (*text).into()));
            send(app, Message::InspectorSubmit(*field));
        }
    };
    let mut cases: Vec<(&str, AmiProApp)> = Vec::new();
    cases.push(("API1", with("PIC1.BMP")));
    cases.push(("API2", with("PIC3.WMF")));
    let mut api5 = with("PIC1.BMP");
    scale(
        &mut api5,
        &[],
        &[Choice::PictureSizing(PictureSizing::Original)],
    );
    cases.push(("API5", api5));
    let mut api7 = with("PIC1.BMP");
    scale(
        &mut api7,
        &[(Field::PictureWidth, "1"), (Field::PictureHeight, "0.25")],
        &[Choice::PictureAspect(false)],
    );
    let mut api8 = with("PIC1.BMP");
    scale(
        &mut api8,
        &[(Field::PictureWidth, "1"), (Field::PictureHeight, "0.25")],
        &[Choice::PictureAspect(false)],
    );
    frame_action(&mut api8, FrameAction::EnterText { at: None });
    frame_action(
        &mut api8,
        FrameAction::Pan {
            dx: 936,
            dy: 144,
            key: false,
        },
    );
    cases.push(("API7", api7));
    cases.push(("API8", api8));
    let mut rotated = with("PIC1.BMP");
    scale(&mut rotated, &[(Field::PictureRotation, "90")], &[]);
    cases.push(("APIRT", rotated));
    // PNG and JPEG of PIC1's picture, converted to bitmaps (P8).
    let pic1 = image::load_from_memory(&std::fs::read(inputs.join("PIC1.BMP")).unwrap()).unwrap();
    let scratch = std::env::temp_dir().join(format!("amipro-twins-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).unwrap();
    for (name, file, format) in [
        ("APNG", "pic1.png", image::ImageFormat::Png),
        ("AJPG", "pic1.jpg", image::ImageFormat::Jpeg),
    ] {
        let path = scratch.join(file);
        pic1.to_rgb8().save_with_format(&path, format).unwrap();
        let mut app = base();
        chosen(&mut app, &path, false);
        cases.push((name, app));
    }
    cases.push(("APCX", with("PIC4.PCX")));
    cases.push(("ATIF", with("PIC5.TIF")));
    // TF7's empty cell (0, 1).
    let mut cell = open("TF7");
    let flow = cell
        .document
        .frames()
        .find_map(|frame| match &frame.content {
            FrameContent::Table(table) => {
                let amipro_document::CellContent::Text(flow) = table.cells[1].content;
                Some(flow)
            }
            _ => None,
        })
        .unwrap();
    move_to(&mut cell, at(flow, 0, 0), 1, false);
    chosen(&mut cell, &inputs.join("PIC1.BMP"), false);
    cases.push(("ACELL", cell));
    // An empty frame made with Insert › Frame…'s rules, then selected.
    let mut framed = base();
    framed.create_frame(1, amipro_document::Rect::new(2_880, 4_320, 5_760, 6_480));
    chosen(&mut framed, &inputs.join("PIC1.BMP"), false);
    cases.push(("AFRM", framed));

    for (name, app) in &mut cases {
        let pictures = app
            .document
            .frames()
            .filter(|frame| matches!(frame.content, FrameContent::Picture(_)))
            .count();
        assert!(pictures >= 1, "{name}: no picture");
        if let Some(out) = &out {
            let path = out.join(format!("{name}.SAM"));
            let _ = std::fs::remove_file(&path);
            assert!(app.write_sam(&path, 0, false), "{name}: {}", app.status);
            let back = amipro_formats::import_sam_path(&path).unwrap().document;
            assert_eq!(
                back.frames()
                    .filter(|frame| matches!(&frame.content,
                        FrameContent::Picture(picture) if picture.ami_pro_stores()))
                    .count(),
                pictures,
                "{name}: every picture as Ami Pro holds it"
            );
            eprintln!("wrote {}", path.display());
        }
    }
    std::fs::remove_dir_all(&scratch).unwrap();
}

/// Release timings for pictures (`docs/PICTURES_RUNBOOK.md`, PI.11), to
/// the drawn frame: the first draw of a page of pictures (decoding them)
/// and a redraw (cached), in README31 (`AMIPRO_PERF_SAM`, its page 12) and
/// in another file (`AMIPRO_PICTURE_SAM`, its page with the most
/// pictures); typing beside a picture; a step in picture mode.
#[test]
#[ignore = "release timings of pictures"]
fn picture_performance_baseline() {
    use crate::frame_editing::FrameAction;
    const SAMPLES: usize = 10;
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let page_with_most = |app: &AmiProApp| {
        app.pages
            .layout()
            .pages
            .iter()
            .max_by_key(|page| {
                page.frames
                    .iter()
                    .filter(|frame_box| frame_box.kind == amipro_layout::FrameBoxKind::Picture)
                    .count()
            })
            .map_or(1, |page| page.number)
    };
    for variable in ["AMIPRO_PERF_SAM", "AMIPRO_PICTURE_SAM"] {
        let Some(path) = std::env::var_os(variable) else {
            continue;
        };
        let (mut first, mut again, mut decoding) = (Vec::new(), Vec::new(), Vec::new());
        let mut pictures = 0;
        for _ in 0..SAMPLES {
            let document = amipro_formats::import_sam_path(&path).unwrap().document;
            let mut app = page_app(document);
            app.location = DocumentLocation::LegacySource(std::path::PathBuf::from(&path));
            let number = if variable == "AMIPRO_PERF_SAM" {
                12
            } else {
                page_with_most(&app)
            };
            pictures = app
                .pages
                .layout()
                .pages
                .iter()
                .find(|page| page.number == number)
                .map_or(0, |page| {
                    page.frames
                        .iter()
                        .filter(|frame_box| frame_box.kind == amipro_layout::FrameBoxKind::Picture)
                        .count()
                });
            send(&mut app, Message::Command(CommandId::GoToPage));
            send(
                &mut app,
                Message::GoTo(GoToAction::Input(number.to_string())),
            );
            send(&mut app, Message::GoTo(GoToAction::Go));
            let mut harness = FrameHarness::new(window);
            let started = Instant::now();
            harness.frame(&app);
            first.push(started.elapsed());
            let started = Instant::now();
            harness.frame(&app);
            again.push(started.elapsed());
            // The pictures alone: the page warm, its pictures decoded anew.
            app.sessions[0].pictures = crate::pictures::Pictures::default();
            let started = Instant::now();
            harness.frame(&app);
            decoding.push(started.elapsed());
        }
        eprintln!(
            "{variable}: a page of {pictures} pictures drawn first {}, again {}; with the page \
             warm and the pictures decoded anew {}",
            summary(&mut first),
            summary(&mut again),
            summary(&mut decoding)
        );
    }

    // Typing beside an inserted picture, and a step in picture mode.
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    chosen(&mut app, &picture_file("perf.bmp"), false);
    let frame = app.frame_selection.as_ref().unwrap().frames[0];
    let mut harness = FrameHarness::new(window);
    let (mut typing, mut step) = (Vec::new(), Vec::new());
    for _ in 0..SAMPLES {
        move_to(&mut app, at(main, 0, 1), 1, false);
        harness.frame(&app);
        let started = Instant::now();
        page(&mut app, PageAction::Insert("e".into()));
        harness.frame(&app);
        typing.push(started.elapsed());
        frame_action(
            &mut app,
            FrameAction::Select {
                frame,
                page: 1,
                extend: false,
            },
        );
        frame_action(&mut app, FrameAction::EnterText { at: None });
        harness.frame(&app);
        let started = Instant::now();
        frame_action(
            &mut app,
            FrameAction::Pan {
                dx: 72,
                dy: 0,
                key: true,
            },
        );
        harness.frame(&app);
        step.push(started.elapsed());
        frame_action(&mut app, FrameAction::Deselect);
    }
    eprintln!(
        "typing beside a picture {}; a picture mode step {}",
        summary(&mut typing),
        summary(&mut step)
    );
}

/// Pictures read back from both formats (`docs/PICTURES_RUNBOOK.md`,
/// PI.11): a PNG picture scaled Custom, turned, moved, and described, and a
/// linked bitmap, made in the app; the native file keeps everything; the
/// Ami Pro file keeps the scaling, rotation, and pan, the PNG becomes a
/// bitmap and the link a copy (listed), and the description is listed.
#[test]
fn pictures_read_back_in_both_formats() {
    use crate::frame_inspector::{Choice, Field};
    use amipro_document::{FrameContent, PictureFormat, PictureSource};
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    let folder = std::env::temp_dir().join(format!("amipro-both-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let png = folder.join("both.png");
    image::RgbaImage::from_pixel(60, 40, image::Rgba([10, 200, 90, 255]))
        .save(&png)
        .unwrap();
    chosen(&mut app, &png, false);
    let made = app.frame_selection.as_ref().unwrap().frames[0];
    send(
        &mut app,
        Message::InspectorChoice(Choice::PictureAspect(false)),
    );
    for (field, text) in [
        (Field::PictureWidth, "0.9"),
        (Field::PictureHeight, "0.3"),
        (Field::PictureRotation, "30"),
        (Field::PictureRight, "0.05"),
        (Field::PictureDown, "-0.1"),
        (Field::PictureDescription, "A green square"),
    ] {
        send(&mut app, Message::InspectorInput(field, text.into()));
        send(&mut app, Message::InspectorSubmit(field));
    }
    move_to(&mut app, at(main, 1, 0), 1, false);
    chosen(&mut app, &picture_file("both-linked.bmp"), true);
    let linked = app.frame_selection.as_ref().unwrap().frames[0];
    let picture = |document: &Document, frame| match &document.frame(frame).unwrap().content {
        FrameContent::Picture(picture) => picture.clone(),
        _ => panic!("a picture"),
    };
    let original = picture(&app.document, made);
    assert_eq!(original.scaling.rotation, 30);
    assert_eq!(original.pan, amipro_document::Offset { x: 72, y: -144 });

    // Native: everything.
    let native = Document::decode_native(&app.document.encode_native().unwrap()).unwrap();
    assert_eq!(picture(&native, made), original);
    assert_eq!(picture(&native, linked), picture(&app.document, linked));

    // Ami Pro: scaling, rotation, pan; a bitmap and a copy; no description.
    let kinds: Vec<_> =
        amipro_formats::sam_losses(&app.document, &amipro_formats::SamOptions::at(0))
            .unwrap()
            .into_iter()
            .map(|loss| loss.kind)
            .collect();
    for kind in [
        amipro_formats::LossKind::PictureDescriptions,
        amipro_formats::LossKind::ConvertedPictures,
        amipro_formats::LossKind::LinkedPictures,
    ] {
        assert!(kinds.contains(&kind), "{kind:?} in {kinds:?}");
    }
    let path = folder.join("BOTH.SAM");
    let _ = std::fs::remove_file(&path);
    assert!(app.write_sam(&path, 0, false));
    let back = amipro_formats::import_sam_path(&path).unwrap().document;
    let mut read: Vec<_> = back
        .frames()
        .filter_map(|frame| match &frame.content {
            FrameContent::Picture(picture) => Some(picture.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(read.len(), 2);
    read.sort_by_key(|picture| picture.scaling.rotation);
    let turned = &read[1];
    assert_eq!(turned.format(), PictureFormat::Bitmap);
    assert_eq!(turned.scaling, original.scaling);
    assert_eq!(turned.pan, original.pan);
    assert!(turned.description.is_empty());
    assert!(
        read.iter()
            .all(|picture| matches!(picture.source, PictureSource::Embedded(_)))
    );
    std::fs::remove_dir_all(&folder).unwrap();
}

/// The rows of `table`'s parts on each page.
fn table_rows_by_page(app: &AmiProApp, table: FrameId) -> Vec<(u32, std::ops::Range<usize>)> {
    app.pages
        .layout()
        .pages
        .iter()
        .flat_map(|page| {
            page.frames
                .iter()
                .filter(|frame_box| frame_box.frame == table)
                .filter_map(move |frame_box| match &frame_box.kind {
                    amipro_layout::FrameBoxKind::Table { part, .. } => {
                        Some((page.number, part.rows.clone()))
                    }
                    _ => None,
                })
        })
        .collect()
}

/// §26, TP1 (VM TP6): Insert › Page Break with the caret in a table cell
/// starts the caret's row on the next page, one undo step, and is checked
/// there; chosen again it takes the break away. In the main text it
/// inserts a break as before and shows no check.
#[test]
fn a_page_break_in_a_table_cell_starts_its_row_on_the_next_page() {
    let (document, table, flows) = crate::page_surface::long_table_fixture();
    let mut app = page_app(document);
    let before = table_rows_by_page(&app, table);
    assert_eq!(before[0], (1, 0..before[0].1.end));
    let row = 5;
    move_to(&mut app, at(flows[row * 2], 0, 0), 1, false);
    assert_eq!(menu_check(&app, CommandId::PageBreak), Some(false));
    send(&mut app, Message::Command(CommandId::PageBreak));
    let after = table_rows_by_page(&app, table);
    assert_eq!(after[0], (1, 0..row), "{after:?}");
    assert_eq!(after[1].0, 2);
    assert_eq!(after[1].1.start, row);
    assert_eq!(menu_check(&app, CommandId::PageBreak), Some(true));
    assert_eq!(app.status, "The row starts the next page");

    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(table_rows_by_page(&app, table), before, "one undo step");
    send(&mut app, Message::Command(CommandId::Redo));
    assert_eq!(table_rows_by_page(&app, table), after);

    // Chosen again in that row, the break goes.
    move_to(&mut app, at(flows[row * 2 + 1], 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::PageBreak));
    assert_eq!(table_rows_by_page(&app, table), before);
    assert_eq!(menu_check(&app, CommandId::PageBreak), Some(false));
    assert_eq!(app.status, "The row no longer starts a page");

    // The main text: no check; a break as before.
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 3, 0), 1, false);
    assert_eq!(menu_check(&app, CommandId::PageBreak), None);
    let paragraphs = app.document.paragraphs().len();
    send(&mut app, Message::Command(CommandId::PageBreak));
    assert!(
        app.document.paragraphs()[3]
            .objects
            .iter()
            .any(|object| matches!(object, amipro_document::InlineObject::Break { .. })),
        "a page break in the main text"
    );
    assert_eq!(app.document.paragraphs().len(), paragraphs);
}

/// §26: a table continued on page 2 with its repeated heading, the caret in
/// a cell there, at 100 %: `window-tablepages-light` and `-dark`, and with a
/// row split by Rows span pages (`window-tablepages-split-light`). Writes
/// PPM files to `AMIPRO_SCREENSHOT_DIR`.
#[test]
#[ignore = "writes screenshots of a table across pages"]
fn table_pages_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let write = |runtime: &mut Runtime, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let rgb = runtime.screenshot(&theme);
        let size = runtime.window;
        let mut ppm = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    let show = |document: Document, flow: FlowId, name: &str, both: bool| {
        let mut app = page_app(document);
        move_to(&mut app, at(flow, 0, 0), 1, false);
        let mut runtime = Runtime::new(app);
        runtime.send(Message::Command(CommandId::NextPage));
        runtime.redraw();
        write(&mut runtime, &format!("{name}-light"), false);
        if both {
            write(&mut runtime, &format!("{name}-dark"), true);
        }
    };
    let (document, _, flows) = crate::page_surface::long_table_fixture();
    show(document, flows[0], "window-tablepages", true);

    // Rows span pages with a tall row at page 1's end.
    let (mut document, table, flows) = crate::page_surface::long_table_fixture();
    let FrameContent::Table(content) = &mut document.frame_mut(table).unwrap().content else {
        unreachable!()
    };
    content.row_options.span_pages = true;
    let last = table_rows_by_page(&page_app(document.clone()), table)[0]
        .1
        .end
        - 1;
    let tall = flows[last * 2];
    let command = amipro_editing::Command::ReplaceText {
        start: at(tall, 0, 0),
        end: at(tall, 0, 5),
        text: (1..=12)
            .map(|line| format!("Tall line {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
    };
    let document = amipro_editing::apply(&document, &command).unwrap().document;
    show(document, tall, "window-tablepages-split", false);
}

/// Twins of the tables across pages run (§26; VM TP2, TP5, TP6): the app
/// makes Ami Pro's changes to Ami Pro's own files (a page break on row 2
/// through Insert › Page Break in its cell, row 0 a heading, Rows span
/// pages), writes them as `ATP*.SAM` to `AMIPRO_TWIN_OUT`, and pages them
/// as Ami Pro paged its own: the rows of each page-end table equal those of
/// Ami Pro's page table in the fixture made in Ami Pro, and the written row
/// flags and table header equal Ami Pro's.
#[test]
#[ignore = "needs the VM fixtures (AMIPRO_VM_LAB)"]
fn table_page_twins() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let out = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from);
    let open = |name: &str| {
        page_app(
            amipro_formats::import_sam_path(lab.join(format!("{name}.SAM")))
                .unwrap()
                .document,
        )
    };
    let table_of = |app: &AmiProApp| {
        app.document
            .frames()
            .find(|frame| matches!(frame.content, FrameContent::Table(_)))
            .map(|frame| frame.id)
            .unwrap()
    };
    let cell_flow = |app: &AmiProApp, cell: usize| {
        let FrameContent::Table(table) = &app.document.frame(table_of(app)).unwrap().content else {
            unreachable!()
        };
        let amipro_document::CellContent::Text(flow) = table.cells[cell].content;
        flow
    };
    let set_rows = |app: &mut AmiProApp, change: &dyn Fn(&mut amipro_document::TableRows)| {
        let table = table_of(app);
        let FrameContent::Table(content) = &app.document.frame(table).unwrap().content else {
            unreachable!()
        };
        let mut rows = content.row_options.clone();
        change(&mut rows);
        assert!(
            app.execute_page_command(&amipro_editing::Command::SetTableRows { frame: table, rows })
        );
    };
    let mut cases: Vec<(&str, &str, AmiProApp)> = Vec::new();
    // TP6: Insert › Page Break with the caret in row 2.
    let mut tp6 = open("TP1S");
    let flow = cell_flow(&tp6, 4);
    move_to(&mut tp6, at(flow, 0, 0), 1, false);
    send(&mut tp6, Message::Command(CommandId::PageBreak));
    cases.push(("ATP6", "TP6", tp6));
    // TP2: row 0 a heading.
    let mut tp2 = open("TP1S");
    set_rows(&mut tp2, &|rows| {
        rows.headings.insert(0);
    });
    cases.push(("ATP2", "TP2", tp2));
    // TP5: Rows span pages.
    let mut tp5 = open("TP3S");
    set_rows(&mut tp5, &|rows| rows.span_pages = true);
    cases.push(("ATP5", "TP5", tp5));

    let options = amipro_formats::SamOptions::at(0);
    let row_flags = |sam: &str| -> (String, Vec<String>) {
        let start = sam.find("[tbl]").unwrap();
        let header = sam[start..].lines().nth(1).unwrap().trim().to_owned();
        let flags = sam[start..]
            .lines()
            .skip_while(|line| line.trim() != "[h]")
            .skip(1)
            .take_while(|line| line.trim() != "[e]")
            .map(|line| {
                line.split_whitespace()
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        (header, flags)
    };
    let mut different = Vec::new();
    for (name, fixture, app) in &cases {
        let written = amipro_formats::sam_document(&app.document, &options);
        if let Some(out) = &out {
            std::fs::write(out.join(format!("{name}.SAM")), &written).unwrap();
        }
        let theirs = std::fs::read(lab.join(format!("{fixture}.SAM"))).unwrap();
        let oracle = amipro_formats::oracle::read_layout_oracle(&theirs);
        let expected: Vec<(u32, (usize, usize))> = oracle
            .pages
            .iter()
            .enumerate()
            .filter_map(|(index, end)| {
                end.table_rows
                    .map(|(first, count)| (index as u32 + 1, (first as usize, count as usize)))
            })
            .collect();
        let table = table_of(app);
        let ours: Vec<(u32, (usize, usize))> = table_rows_by_page(app, table)
            .into_iter()
            .map(|(page, rows)| (page, (rows.start, rows.len())))
            .filter(|(page, _)| expected.iter().any(|(theirs, _)| theirs == page))
            .collect();
        let (ours_head, ours_flags) = row_flags(&String::from_utf8_lossy(&written));
        let (their_head, their_flags) = row_flags(&String::from_utf8_lossy(&theirs));
        // Only the flags each change sets; sizes are Ami Pro's measure.
        let flag = |records: &[String]| -> Vec<String> {
            records
                .iter()
                .map(|record| record.split(' ').nth(3).unwrap_or("").to_owned())
                .collect()
        };
        let header_flags = |header: &str| header.split_whitespace().nth(6).map(str::to_owned);
        eprintln!(
            "{name}: pages {ours:?} (Ami Pro {expected:?}); header {ours_head} (Ami Pro {their_head})"
        );
        if ours != expected
            || header_flags(&ours_head) != header_flags(&their_head)
            || flag(&ours_flags) != flag(&their_flags)
        {
            eprintln!("{name}: differs\nours: {ours_flags:?}\nAmi Pro's: {their_flags:?}");
            different.push(*name);
        }
    }
    assert!(different.is_empty(), "different: {different:?}");
}

/// §26: a long table's row options in both formats: native exactly, Save
/// As SAM read back with the same options and the same parts, and nothing
/// in the loss check.
#[test]
fn table_row_options_read_back_in_both_formats() {
    let (mut document, table, _) = crate::page_surface::long_table_fixture();
    let FrameContent::Table(content) = &mut document.frame_mut(table).unwrap().content else {
        unreachable!()
    };
    content.row_options.page_breaks.insert(20);
    content.row_options.span_pages = true;
    let app = page_app(document.clone());
    let parts = table_rows_by_page(&app, table);
    assert!(parts.iter().any(|(_, rows)| rows.start == 20), "{parts:?}");

    let native = Document::decode_native(&document.encode_native().unwrap()).unwrap();
    assert_eq!(native, document);

    let options = amipro_formats::SamOptions::at(0);
    let written = amipro_formats::sam_document(&document, &options);
    let copy = amipro_formats::import_sam(&written, None, "Table")
        .unwrap()
        .document;
    let copied = copy
        .frames()
        .find(|frame| matches!(frame.content, FrameContent::Table(_)))
        .unwrap();
    let FrameContent::Table(back) = &copied.content else {
        unreachable!()
    };
    let FrameContent::Table(original) = &document.frame(table).unwrap().content else {
        unreachable!()
    };
    assert_eq!(back.row_options, original.row_options);
    // The fixture's new-document styles aside, nothing about the table.
    let losses = amipro_formats::sam_losses(&document, &options).unwrap();
    assert!(
        losses.iter().all(|loss| !matches!(
            loss.kind,
            amipro_formats::LossKind::Tables | amipro_formats::LossKind::Frames
        )),
        "{losses:?}"
    );
    let copy_app = page_app(copy.clone());
    assert_eq!(table_rows_by_page(&copy_app, copied.id), parts);
}

/// Release timings of tables across pages (§26), to the drawn frame:
/// typing in a cell of the table's second part, Insert › Page Break in a
/// row and its undo. `AMIPRO_PERF_SAM` may name a document with a long
/// table; the long-table fixture otherwise.
#[test]
#[ignore = "release-mode performance baseline"]
fn table_pages_performance_baseline() {
    const SAMPLES: usize = 20;
    let (fixture, _, _) = crate::page_surface::long_table_fixture();
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or(fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    // The table with the most rows, and a cell in its second part.
    let table = app
        .document
        .frames()
        .filter_map(|frame| match &frame.content {
            FrameContent::Table(content) if frame.placement.is_anchored() => {
                Some((frame.id, content.rows.len()))
            }
            _ => None,
        })
        .max_by_key(|(_, rows)| *rows)
        .map(|(id, _)| id)
        .expect("an anchored table");
    let parts = table_rows_by_page(&app, table);
    let (page_number, rows) = parts.get(1).or(parts.first()).cloned().unwrap();
    let row = rows.start + rows.len() / 2;
    let FrameContent::Table(content) = &app.document.frame(table).unwrap().content else {
        unreachable!()
    };
    let columns = content.columns.len();
    let amipro_document::CellContent::Text(flow) = content.cells[row * columns].content;
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    harness.frame(&app);
    let (mut typing, mut breaking, mut undo_break) = (Vec::new(), Vec::new(), Vec::new());
    for _ in 0..SAMPLES {
        let end = app.document.flow(flow).unwrap().paragraphs[0].text.len();
        move_to(&mut app, at(flow, 0, end), page_number, false);
        harness.frame(&app);
        let started = Instant::now();
        page(&mut app, PageAction::Insert("e".into()));
        harness.frame(&app);
        typing.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
        move_to(&mut app, at(flow, 0, 0), page_number, false);
        harness.frame(&app);
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::PageBreak));
        harness.frame(&app);
        breaking.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undo_break.push(started.elapsed());
    }
    eprintln!(
        "table {table:?} row {row} on page {page_number} ({} parts)",
        parts.len()
    );
    eprintln!("typing in a continued table: {}", summary(&mut typing));
    eprintln!("page break in a row: {}", summary(&mut breaking));
    eprintln!("undo the page break: {}", summary(&mut undo_break));
}

/// FR1's three paragraphs (`docs/research/FIND_NOTES.md`).
fn find_fixture() -> Document {
    Document::from_plain_text(
        "status bar. Status Bar. STATUS BAR. Status bar. sTatus bar.\n\
         the theater then The thesis THE the.\n\
         phone phones phoned phoning telephone.",
    )
}

fn find(app: &mut AmiProApp, action: crate::find_bar::FindAction) {
    send(app, Message::Find(action));
}

fn find_bar(app: &AmiProApp) -> &crate::find_bar::FindBar {
    app.find.as_ref().expect("the find bar")
}

#[test]
fn the_find_bar_finds_steps_replaces_and_replaces_all_as_one_step_each() {
    use crate::find_bar::{Field, FindAction};
    let mut app = page_app(find_fixture());
    let main = app.document.main_flow_id();
    send(&mut app, Message::Command(CommandId::Find));
    assert!(!find_bar(&app).replacing);
    find(&mut app, FindAction::Text(Field::Find, "status bar".into()));
    assert_eq!(find_bar(&app).count_label(), "5 matches");
    // Typing in the bar leaves the page alone; Find Next selects the first
    // match from the caret, then the next; Find Previous goes back.
    assert!(app.page_selection.is_caret());
    send(&mut app, Message::Command(CommandId::FindNext));
    assert_eq!(
        app.page_selection.ordered(),
        (at(main, 0, 0), at(main, 0, 10))
    );
    assert_eq!(find_bar(&app).count_label(), "1 of 5");
    send(&mut app, Message::Command(CommandId::FindNext));
    assert_eq!(app.page_selection.ordered().0, at(main, 0, 12));
    send(&mut app, Message::Command(CommandId::FindPrevious));
    assert_eq!(app.page_selection.ordered().0, at(main, 0, 0));
    // Enter finds the next; Shift+Enter the previous (round to the last).
    find(&mut app, FindAction::Submit(Field::Find));
    assert_eq!(find_bar(&app).count_label(), "2 of 5");
    send(&mut app, Message::Modifiers(keyboard::Modifiers::SHIFT));
    find(&mut app, FindAction::Submit(Field::Find));
    find(&mut app, FindAction::Submit(Field::Find));
    assert_eq!(find_bar(&app).count_label(), "5 of 5");
    send(&mut app, Message::Modifiers(keyboard::Modifiers::empty()));
    find(&mut app, FindAction::Submit(Field::Find));
    assert_eq!(find_bar(&app).count_label(), "1 of 5");

    // Replace: Edit › Replace… shows the row; Replace changes the selected
    // match in the found text's capitals, one undo step, and goes on.
    send(&mut app, Message::Command(CommandId::Replace));
    assert!(find_bar(&app).replacing);
    find(
        &mut app,
        FindAction::Text(Field::Replace, "window pane".into()),
    );
    find(&mut app, FindAction::ReplaceOne);
    let first = |app: &AmiProApp| app.document.paragraphs()[0].text.clone();
    assert!(
        first(&app).starts_with("window pane. Status Bar."),
        "{}",
        first(&app)
    );
    assert_eq!(find_bar(&app).count_label(), "1 of 4");
    assert_eq!(app.page_selection.ordered().0, at(main, 0, 13));
    assert_editor_in_sync(&app);
    // Replace All: the other four, one step; Undo takes them all back.
    find(&mut app, FindAction::ReplaceAll);
    assert_eq!(
        first(&app),
        "window pane. Window Pane. WINDOW PANE. Window Pane. window pane."
    );
    assert_eq!(app.status, "Replaced 4 matches");
    assert_eq!(find_bar(&app).count_label(), "No matches");
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(first(&app).starts_with("window pane. Status Bar."));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(first(&app), find_fixture().paragraphs()[0].text);
    // Undo gives back the selection: the first match, current again.
    assert_eq!(find_bar(&app).count_label(), "1 of 5");
    assert_editor_in_sync(&app);
    // The searches are remembered, newest first.
    assert_eq!(app.preferences.recent_searches, ["status bar"]);
}

#[test]
fn the_find_bars_options_follow_ami_pro_and_are_kept() {
    use crate::find_bar::{AttributeChoice, Field, FindAction, FindMenu, Toggle};
    let mut app = page_app(find_fixture());
    send(&mut app, Message::Command(CommandId::Find));
    find(&mut app, FindAction::Text(Field::Find, "the".into()));
    // Whole word only, Ami Pro's default: three of the seven.
    assert_eq!(find_bar(&app).matches.len(), 4);
    find(
        &mut app,
        FindAction::Menu(FindMenu::Toggle(Toggle::WholeWord)),
    );
    assert!(!app.preferences.find_options.whole_word);
    assert_eq!(find_bar(&app).matches.len(), 7);
    find(
        &mut app,
        FindAction::Menu(FindMenu::Toggle(Toggle::MatchCase)),
    );
    assert_eq!(find_bar(&app).matches.len(), 5);
    // Find attributes: Italic finds nothing here; Any ends Exact attributes.
    find(
        &mut app,
        FindAction::Menu(FindMenu::FindAttribute(AttributeChoice::Italic)),
    );
    assert_eq!(find_bar(&app).count_label(), "No matches");
    find(
        &mut app,
        FindAction::Menu(FindMenu::FindAttribute(AttributeChoice::Any)),
    );
    assert_eq!(find_bar(&app).matches.len(), 5);
    // Ami Pro's marks for a tab and a return go into the last field.
    find(&mut app, FindAction::Focus(Field::Find, true));
    find(&mut app, FindAction::Menu(FindMenu::InsertReturn));
    assert_eq!(find_bar(&app).find, "the¶");
    // Paragraph styles: Find lists the styles; every Body Text paragraph.
    find(&mut app, FindAction::Menu(FindMenu::Toggle(Toggle::Styles)));
    let names = find_style_names(&app.document);
    let body = names.iter().position(|name| name == "Body Text").unwrap();
    find(&mut app, FindAction::Style(Field::Find, body));
    assert_eq!(find_bar(&app).find, "Body Text");
    // The main text's three, and the header's and footer's empty lines.
    assert_eq!(find_bar(&app).matches.len(), 5);
    // Esc closes the bar only from its fields; × closes it.
    find(&mut app, FindAction::Focus(Field::Find, false));
    find(&mut app, FindAction::Escape);
    assert!(app.find.is_some());
    find(&mut app, FindAction::Focus(Field::Replace, true));
    find(&mut app, FindAction::Escape);
    assert!(app.find.is_none());
    // Opened again, it keeps its text; a selection across paragraphs
    // limits the search to it (Ami Pro's rule).
    send(&mut app, Message::Command(CommandId::Find));
    assert_eq!(find_bar(&app).find, "Body Text");
    find(&mut app, FindAction::Close);
    let main = app.document.main_flow_id();
    app.sessions[0].page_selection = PageSelection {
        anchor: at(main, 0, 5),
        focus: at(main, 1, 5),
        page: 1,
    };
    send(&mut app, Message::Command(CommandId::Find));
    assert!(find_bar(&app).within.is_some());
    find(&mut app, FindAction::Menu(FindMenu::Toggle(Toggle::Styles)));
    // Match case and not whole words, as set above: "the" once in the
    // selection's part of the second paragraph.
    find(&mut app, FindAction::Text(Field::Find, "the".into()));
    assert_eq!(find_bar(&app).matches.len(), 1);
    find(
        &mut app,
        FindAction::Menu(FindMenu::Toggle(Toggle::InSelection)),
    );
    assert_eq!(find_bar(&app).matches.len(), 5);
    // The options are kept in the settings' form.
    let kept = crate::find_bar::options_from_numbers(&crate::find_bar::options_to_numbers(
        &app.preferences.find_options,
    ));
    assert_eq!(kept.as_ref(), Some(&app.preferences.find_options));
}

#[test]
fn the_find_bar_takes_the_keys_and_keeps_the_pages_scroll() {
    use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
    let mut runtime = scrolled_runtime();
    let scroll = runtime.workspace_scroll();
    let before = runtime.app.document.clone();
    // Ctrl+F gives the Find field the keyboard: typed keys go there, not
    // into the page.
    runtime.send(Message::Command(CommandId::Find));
    assert_eq!(runtime.workspace_scroll(), scroll);
    let typed: Vec<_> = "café"
        .chars()
        .map(|c| key_press(Key::Character(c.to_string().into()), Modifiers::empty()))
        .collect();
    let messages = runtime.events(&typed);
    assert!(
        messages.iter().any(|message| matches!(
            message,
            Message::Find(crate::find_bar::FindAction::Text(..))
        )),
        "{messages:?}"
    );
    assert_eq!(runtime.app.find.as_ref().unwrap().find, "café");
    assert_eq!(
        runtime.app.find.as_ref().unwrap().focused,
        Some(crate::find_bar::Field::Find)
    );
    assert_eq!(runtime.app.document, before);
    // Enter in the field finds the next match and shows it.
    runtime.events(&[key_press(Key::Named(Named::Enter), Modifiers::empty())]);
    // From the caret, on page 2.
    let bar = runtime.app.find.as_ref().unwrap();
    let current = bar.current.expect("a current match");
    assert_eq!(
        bar.count_label(),
        format!("{} of {}", current + 1, bar.matches.len())
    );
    assert!(current > 0);
    let (start, end) = runtime.app.page_selection.ordered();
    let paragraph = &runtime.app.document.paragraphs()[start.paragraph];
    assert_eq!(&paragraph.text[start.byte..end.byte], "café");
    // Closing it keeps the pages where they are.
    runtime.send(Message::Find(crate::find_bar::FindAction::Close));
    assert!(runtime.app.find.is_none());
    let shown = runtime.app.scroll_offset.y;
    assert_eq!(runtime.workspace_scroll(), shown);
}

/// The find bar with its matches on the page, in both themes
/// (`UI_SPEC.md` §27): `AMIPRO_SCREENSHOT_DIR` receives
/// `window-find-*.ppm`, as `window_screenshots`.
#[test]
#[ignore = "writes screenshots (AMIPRO_SCREENSHOT_DIR)"]
fn find_bar_screenshots() {
    use crate::find_bar::{Field, FindAction};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    send(&mut app, Message::Command(CommandId::Replace));
    find(
        &mut app,
        FindAction::Text(Field::Find, "second line".into()),
    );
    find(
        &mut app,
        FindAction::Text(Field::Replace, "next line".into()),
    );
    send(&mut app, Message::Command(CommandId::FindNext));
    send(&mut app, Message::Command(CommandId::FindNext));
    let write = |app: &AmiProApp, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    write(&app, "window-find-replace-light", false);
    write(&app, "window-find-replace-dark", true);
    find(&mut app, FindAction::ShowReplace);
    write(&app, "window-find-light", false);
    write(&app, "window-find-dark", true);
}

/// Find & Replace's timings (FD.7), release, to the drawn frame: a search
/// typed into the bar and Find Next, a keystroke in the page while the bar
/// shows its matches, Replace All, and its undo. `AMIPRO_PERF_SAM` names
/// the document (README31 and the longest corpus file), `AMIPRO_FIND` the
/// text (default "the").
#[test]
#[ignore = "release-mode performance baseline"]
fn find_performance_baseline() {
    use crate::find_bar::{Field, FindAction};
    const SAMPLES: usize = 10;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let text = std::env::var("AMIPRO_FIND").unwrap_or_else(|_| "the".into());
    let mut app = page_app(document);
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    harness.frame(&app);
    send(&mut app, Message::Command(CommandId::Replace));
    find(&mut app, FindAction::Text(Field::Replace, "THE".into()));
    let main = app.document.main_flow_id();
    let (mut finding, mut typing, mut replacing, mut undoing) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut count = 0;
    for _ in 0..SAMPLES {
        find(&mut app, FindAction::Text(Field::Find, String::new()));
        move_to(&mut app, at(main, 0, 0), 1, false);
        harness.frame(&app);
        let started = Instant::now();
        find(&mut app, FindAction::Text(Field::Find, text.clone()));
        send(&mut app, Message::Command(CommandId::FindNext));
        harness.frame(&app);
        finding.push(started.elapsed());
        count = find_bar(&app).matches.len();
        // A keystroke in the page finds the matches again.
        move_to(&mut app, at(main, 0, 0), 1, false);
        harness.frame(&app);
        let started = Instant::now();
        page(&mut app, PageAction::Insert("x".into()));
        harness.frame(&app);
        typing.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        let started = Instant::now();
        find(&mut app, FindAction::ReplaceAll);
        harness.frame(&app);
        replacing.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undoing.push(started.elapsed());
    }
    // The same keystroke without the bar, for comparison.
    find(&mut app, FindAction::Close);
    let mut plain = Vec::new();
    for _ in 0..SAMPLES {
        move_to(&mut app, at(main, 0, 0), 1, false);
        harness.frame(&app);
        let started = Instant::now();
        page(&mut app, PageAction::Insert("x".into()));
        harness.frame(&app);
        plain.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
    }
    eprintln!(
        "{} paragraphs, \"{text}\" {count} matches",
        app.document.paragraphs().len()
    );
    eprintln!("a keystroke without the bar: {}", summary(&mut plain));
    eprintln!("type the search and Find Next: {}", summary(&mut finding));
    eprintln!("a keystroke with the bar open: {}", summary(&mut typing));
    eprintln!("Replace All: {}", summary(&mut replacing));
    eprintln!("undo Replace All: {}", summary(&mut undoing));
}

fn fields(app: &mut AmiProApp, action: crate::fields_panel::FieldsAction) {
    send(app, Message::FieldsPanel(action));
}

#[test]
fn fields_are_inserted_updated_and_shown_through_the_fields_panel() {
    use crate::fields_panel::{FieldsAction, Page};
    let mut app = page_app(Document::from_plain_text("Pages: ."));
    let main = app.document.main_flow_id();
    app.page_selection = PageSelection::caret(at(main, 0, 7), 1);
    // Insert › Field… opens the panel at Power field; a code inserted is
    // worked out at once between its marks, one undo step.
    send(&mut app, Message::Command(CommandId::Field));
    assert!(
        app.fields_panel
            .as_ref()
            .is_some_and(|panel| panel.page == Page::Power)
    );
    fields(&mut app, FieldsAction::Code("NumPages".into()));
    let undo = app.history.undo_len();
    fields(&mut app, FieldsAction::Insert);
    assert_eq!(app.document.paragraphs()[0].text, "Pages: 1.");
    assert_eq!(app.history.undo_len(), undo + 1);
    // The status bar names the field at the caret; This field follows it.
    app.page_selection = PageSelection::caret(at(main, 0, 8), 1);
    send(&mut app, Message::Repaint);
    assert!(app.field_at_caret().is_some());
    // Edit › Power Fields ▸ Update All Fields; Next Field selects the
    // whole field.
    send(&mut app, Message::Command(CommandId::UpdateAllFields));
    app.page_selection = PageSelection::caret(at(main, 0, 0), 1);
    send(&mut app, Message::Command(CommandId::NextField));
    assert_eq!(
        app.page_selection.ordered(),
        (at(main, 0, 7), at(main, 0, 8))
    );
    // Lock Field, checked; Update leaves a locked field.
    send(&mut app, Message::Command(CommandId::LockField));
    assert_eq!(menu_check(&app, CommandId::LockField), Some(true));
    // Shading covers the result; View › Show Power Fields shows the code
    // in red in the text, to be typed over, one undo step.
    let shaded = app
        .field_shading
        .get(&(main, 0))
        .cloned()
        .unwrap_or_default();
    assert_eq!(shaded.len(), 1);
    assert_eq!((shaded[0].start, shaded[0].end), (7, 8));
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::ShowPowerFields));
    assert_eq!(app.document.paragraphs()[0].text, "Pages: <NumPages>.");
    assert_eq!(menu_check(&app, CommandId::ShowPowerFields), Some(true));
    assert_eq!(app.history.undo_len(), undo + 1);
    // The selected result became the selected code.
    assert_eq!(
        app.page_selection.ordered(),
        (at(main, 0, 7), at(main, 0, 17))
    );
    // Typed over: the code becomes a sum; Update reads it back and works
    // it out, the code still showing (the lock taken off first).
    send(&mut app, Message::Command(CommandId::LockField));
    app.page_selection = PageSelection {
        anchor: at(main, 0, 8),
        focus: at(main, 0, 16),
        page: 1,
    };
    page(&mut app, PageAction::Insert("2+5".into()));
    assert_eq!(app.document.paragraphs()[0].text, "Pages: <2+5>.");
    send(&mut app, Message::Command(CommandId::UpdateFields));
    assert_eq!(app.document.paragraphs()[0].text, "Pages: <2+5>.");
    // Off: the new result.
    send(&mut app, Message::Command(CommandId::ShowPowerFields));
    assert!(!app.codes_shown());
    assert_eq!(app.document.paragraphs()[0].text, "Pages: 7.");
    // Update All with codes showing switches them off, as Ami Pro's does.
    send(&mut app, Message::Command(CommandId::ShowPowerFields));
    send(&mut app, Message::Command(CommandId::UpdateAllFields));
    assert!(!app.codes_shown());
    assert_eq!(app.document.paragraphs()[0].text, "Pages: 7.");
    // A save switches them off first, as Ami Pro's does: the file holds
    // the results.
    send(&mut app, Message::Command(CommandId::ShowPowerFields));
    let folder = std::env::temp_dir().join(format!("amipro-codes-save-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("CODES.SAM");
    assert!(app.write_sam(&path, 0, false));
    assert!(!app.codes_shown());
    let back = amipro_formats::import_sam_path(&path).unwrap().document;
    assert_eq!(back.paragraphs()[0].text, "Pages: 7.");
    std::fs::remove_dir_all(&folder).unwrap();
    // Insert › Date/Time…: System date in format b, as an object.
    send(&mut app, Message::Command(CommandId::DateTime));
    fields(&mut app, FieldsAction::DateKind(1));
    fields(&mut app, FieldsAction::DateFormat(1));
    app.page_selection = PageSelection::caret(at(main, 0, 0), 1);
    fields(&mut app, FieldsAction::Insert);
    assert!(matches!(
        app.document.paragraphs()[0].objects[0],
        amipro_document::InlineObject::Date(_)
    ));
}

#[test]
fn document_properties_edit_doc_info_one_step_at_a_time() {
    use crate::document_properties::{PropertiesAction, Text};
    let mut app = page_app(Document::from_plain_text("One two three"));
    send(&mut app, Message::Command(CommandId::DocumentProperties));
    assert!(app.document_properties.is_some());
    let undo = app.history.undo_len();
    send(
        &mut app,
        Message::DocumentProperties(PropertiesAction::Text(
            Text::Description,
            "Quarterly".into(),
        )),
    );
    send(
        &mut app,
        Message::DocumentProperties(PropertiesAction::Commit),
    );
    assert_eq!(app.document.info().description, "Quarterly");
    assert_eq!(app.history.undo_len(), undo + 1);
    assert!(app.dirty);
    // Tools › Word Count opens it at the statistics; the Fields panel and
    // it never show together.
    send(&mut app, Message::Command(CommandId::Field));
    assert!(app.document_properties.is_none());
    send(&mut app, Message::Command(CommandId::WordCount));
    assert!(app.fields_panel.is_none() && app.document_properties.is_some());
}

/// The Fields panel and Document Properties in both themes (FL.10, FL.11):
/// `AMIPRO_SCREENSHOT_DIR` receives `window-fields-*` and
/// `window-document-properties-*`.
#[test]
#[ignore = "writes screenshots"]
fn field_panel_screenshots() {
    use crate::document_properties::{PropertiesAction, Section};
    use crate::fields_panel::FieldsAction;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(Document::from_plain_text(
        "Pages: .\nA quarterly report for the board.",
    ));
    drop(app.update_title());
    let main = app.document.main_flow_id();
    app.page_selection = PageSelection::caret(at(main, 0, 7), 1);
    send(&mut app, Message::Command(CommandId::Field));
    fields(&mut app, FieldsAction::Pick(25));
    fields(&mut app, FieldsAction::Insert);
    app.page_selection = PageSelection::caret(at(main, 0, 8), 1);
    send(&mut app, Message::Command(CommandId::EditField));
    fields(&mut app, FieldsAction::Pick(3));
    fields(&mut app, FieldsAction::Option(1));
    let write = |app: &AmiProApp, name: &str, dark: bool| {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    };
    write(&app, "window-fields-light", false);
    write(&app, "window-fields-dark", true);
    send(&mut app, Message::Command(CommandId::DocumentProperties));
    for section in [Section::Fields, Section::Statistics, Section::History] {
        send(
            &mut app,
            Message::DocumentProperties(PropertiesAction::Section(section)),
        );
    }
    write(&app, "window-document-properties-light", false);
    write(&app, "window-document-properties-dark", true);
}

/// The fields' timings (FL.14), release, to the drawn frame: on the
/// document `AMIPRO_PERF_SAM` names with a power field and a date put in
/// each of its first 50 paragraphs, a keystroke in the page, Update All,
/// its undo, and Show Power Fields on and off.
#[test]
#[ignore = "release-mode performance baseline"]
fn field_performance_baseline() {
    use crate::fields_panel::FieldsAction;
    const SAMPLES: usize = 10;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    let main = app.document.main_flow_id();
    // Fields at the start of the first paragraphs: NumWords (which counts
    // the whole document) and System date.
    let count = app.document.paragraphs().len().min(50);
    send(&mut app, Message::Command(CommandId::Field));
    for paragraph in (0..count).rev() {
        app.page_selection = PageSelection::caret(at(main, paragraph, 0), 1);
        fields(
            &mut app,
            FieldsAction::Page(crate::fields_panel::Page::Power),
        );
        fields(&mut app, FieldsAction::Code("NumWords".into()));
        fields(&mut app, FieldsAction::Insert);
        app.page_selection = PageSelection::caret(at(main, paragraph, 0), 1);
        fields(
            &mut app,
            FieldsAction::Page(crate::fields_panel::Page::DateTime),
        );
        fields(&mut app, FieldsAction::DateKind(1));
        fields(&mut app, FieldsAction::Insert);
    }
    send(&mut app, Message::FieldsPanel(FieldsAction::Close));
    harness.frame(&app);
    let (mut typing, mut updating, mut undoing, mut showing) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for _ in 0..SAMPLES {
        move_to(&mut app, at(main, 0, 0), 1, false);
        harness.frame(&app);
        let started = Instant::now();
        page(&mut app, PageAction::Insert("x".into()));
        harness.frame(&app);
        typing.push(started.elapsed());
        // Update All works every field out again (the words changed).
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::UpdateAllFields));
        harness.frame(&app);
        updating.push(started.elapsed());
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        undoing.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
        harness.frame(&app);
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::ShowPowerFields));
        harness.frame(&app);
        send(&mut app, Message::Command(CommandId::ShowPowerFields));
        harness.frame(&app);
        showing.push(started.elapsed());
    }
    eprintln!(
        "{} paragraphs, {} fields",
        app.document.paragraphs().len(),
        2 * count
    );
    eprintln!("a keystroke: {}", summary(&mut typing));
    eprintln!("Update All: {}", summary(&mut updating));
    eprintln!("its undo: {}", summary(&mut undoing));
    eprintln!("Show Power Fields on and off: {}", summary(&mut showing));
}

/// How long the fields' work after each change takes alone (FL.14): the
/// shading walk and the context with the clock.
#[test]
#[ignore = "release-mode performance baseline"]
fn field_refresh_cost() {
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    let started = Instant::now();
    for _ in 0..1000 {
        app.sessions[0].refresh_field_views();
    }
    eprintln!("refresh_field_views: {:?} each", started.elapsed() / 1000);
    let started = Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(app.sessions[0].field_context());
    }
    eprintln!("field_context: {:?} each", started.elapsed() / 1000);
}

#[test]
fn export_pdf_reports_what_it_wrote() {
    // `UI_SPEC.md` §29: File › Export PDF… (Ctrl+Shift+E) with a document
    // open; the export runs off the interface's thread and the status bar
    // says what it wrote.
    let mut app = page_app(page_fixture());
    assert!(app.command_enabled(CommandId::ExportPdf));
    let session_id = app.id;
    let path = std::env::temp_dir().join("Fixture.pdf");
    send(
        &mut app,
        Message::ExportPdfChosen {
            session_id,
            result: FileDialogResult::Selected(path.clone()),
        },
    );
    assert_eq!(app.status, "Exporting PDF…");
    send(
        &mut app,
        Message::PdfExported {
            session_id,
            result: Ok(crate::print::Exported {
                path: path.clone(),
                pages: 3,
            }),
        },
    );
    assert_eq!(app.status, "Exported 3 pages to Fixture.pdf");
    send(
        &mut app,
        Message::PdfExported {
            session_id,
            result: Err("No space left on device".into()),
        },
    );
    assert_eq!(app.status, "Export PDF failed: No space left on device");
    send(
        &mut app,
        Message::ExportPdfChosen {
            session_id,
            result: FileDialogResult::Cancelled,
        },
    );
    assert_eq!(app.status, "Export PDF failed: No space left on device");
}

/// The export the app makes for `format` (what its background job runs).
fn prepared_export(
    app: &AmiProApp,
    format: amipro_exchange::Format,
    path: &std::path::Path,
) -> crate::export::Prepared {
    crate::export::ExportJob {
        document: app.document.clone(),
        layout: app.pages.layout().clone(),
        folder: None,
        fields: app.field_context(),
        title: "Fixture".into(),
        format,
        path: path.to_path_buf(),
        set: Vec::new(),
    }
    .run()
    .expect("the export is made")
}

#[test]
fn export_writes_each_format_and_offers_to_open_it() {
    // `UI_SPEC.md` §30 (EX1, EX2, EX10): File › Export ▸ for each format;
    // the file is made off the interface's thread, written when nothing is
    // lost, and the status bar says so with Open.
    use amipro_exchange::Format;
    let folder = std::env::temp_dir().join(format!("amipro-formats-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    for (format, command) in [
        (Format::Docx, CommandId::ExportDocx),
        (Format::Odt, CommandId::ExportOdt),
        (Format::Rtf, CommandId::ExportRtf),
        (Format::Text, CommandId::ExportText),
        (Format::Markdown, CommandId::ExportMarkdown),
    ] {
        let mut app = page_app(page_fixture());
        assert!(app.command_enabled(command));
        let session_id = app.id;
        // The dialog's name without an extension gets the format's.
        let chosen = folder.join("Fixture");
        send(
            &mut app,
            Message::ExportChosen {
                session_id,
                format,
                result: FileDialogResult::Selected(chosen),
            },
        );
        assert_eq!(app.status, format!("Exporting {}…", format.name()));
        let path = folder.join(format!("Fixture.{}", format.extension()));
        let prepared = prepared_export(&app, format, &path);
        // Plain text cannot hold the fixture's frames (EC13): it asks; so
        // does Markdown, which never holds the page layout (MD1).
        let asks = matches!(format, Format::Text | Format::Markdown);
        assert_eq!(!prepared.losses.is_empty(), asks, "{:?}", prepared.losses);
        send(
            &mut app,
            Message::ExportPrepared {
                session_id,
                result: Ok(prepared),
            },
        );
        if asks {
            assert!(app.export_pending.is_some());
            send(&mut app, Message::ExportAnyway);
        }
        assert_eq!(
            app.status,
            format!(
                "Exported Fixture.{} ({})",
                format.extension(),
                format.name()
            )
        );
        assert_eq!(app.exported.as_deref(), Some(path.as_path()));
        let written = std::fs::read(&path).unwrap();
        match format {
            Format::Docx | Format::Odt => assert!(written.starts_with(b"PK")),
            Format::Rtf => assert!(written.starts_with(b"{\\rtf1")),
            Format::Text | Format::Markdown => assert!(!written.is_empty()),
        }
        // The next export starts in this folder.
        assert_eq!(
            app.preferences.export_folder.as_deref(),
            Some(folder.as_path())
        );
    }
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn export_markdown_lists_what_it_never_holds_and_what_it_would_replace() {
    // MD1: Export ▸ Export Markdown… (the real command, its chooser not
    // run) and Save As Markdown both end in the loss list, which names the
    // files that exist already; nothing is written until it is answered.
    use amipro_exchange::Format;
    let folder = std::env::temp_dir().join(format!("amipro-markdown-app-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    let mut app = page_app(page_fixture());
    assert!(app.command_enabled(CommandId::ExportMarkdown));
    let task = cosmic::Application::update(&mut app, Message::Command(CommandId::ExportMarkdown));
    assert!(
        cosmic::iced::runtime::task::into_stream(task).is_some(),
        "the command asks for the chooser"
    );
    let path = folder.join("Fixture.md");
    std::fs::write(&path, b"old").unwrap();
    let session_id = app.id;
    send(
        &mut app,
        Message::SaveDialogFinished {
            session_id,
            result: FileDialogResult::Selected(folder.join("Fixture")),
            format: Some(crate::SaveFormat::Markdown),
        },
    );
    let prepared = prepared_export(&app, Format::Markdown, &path);
    for line in amipro_exchange::losses::always_lost(Format::Markdown) {
        assert!(prepared.losses.iter().any(|loss| loss == line), "{line}");
    }
    assert_eq!(prepared.replaces, ["Fixture.md"]);
    send(
        &mut app,
        Message::ExportPrepared {
            session_id,
            result: Ok(prepared),
        },
    );
    assert!(app.export_pending.is_some());
    assert_eq!(std::fs::read(&path).unwrap(), b"old");
    send(&mut app, Message::ExportAnyway);
    let text = std::fs::read_to_string(&path).unwrap();
    assert_ne!(text, "old");
    assert_eq!(app.status, "Exported Fixture.md (Markdown)");
    let _ = std::fs::remove_dir_all(folder);
}

#[test]
fn an_export_that_loses_something_asks_first() {
    // EX7: the list before writing, with Cancel and Export Anyway, as Save
    // As SAM.
    use amipro_exchange::Format;
    let mut app = page_app(page_fixture());
    let session_id = app.id;
    let path = std::env::temp_dir().join(format!("amipro-export-asks-{}.txt", std::process::id()));
    let mut prepared = prepared_export(&app, Format::Text, &path);
    prepared.losses = vec!["pictures (2 places)".into()];
    send(
        &mut app,
        Message::ExportPrepared {
            session_id,
            result: Ok(prepared.clone()),
        },
    );
    assert!(app.export_pending.is_some());
    assert!(!path.exists());
    send(&mut app, Message::CancelExport);
    assert!(app.export_pending.is_none());
    assert!(!path.exists());
    assert_eq!(app.status, "Export cancelled; nothing was written");
    send(
        &mut app,
        Message::ExportPrepared {
            session_id,
            result: Ok(prepared),
        },
    );
    send(&mut app, Message::ExportAnyway);
    assert!(path.exists());
    assert!(app.exported.is_some());
    let _ = std::fs::remove_file(path);
    send(
        &mut app,
        Message::ExportPrepared {
            session_id,
            result: Err("No space left on device".into()),
        },
    );
    assert_eq!(app.status, "Export failed: No space left on device");
}

#[test]
fn export_folder_exports_every_document_with_a_report() {
    // `UI_SPEC.md` §30 (EX6): File › Export ▸ Folder… in the rail; the
    // documents keep their places under To, and a report lists them.
    use crate::export_folder::{FolderAction, FolderJob, Progress};
    use amipro_exchange::Format;
    let root = std::env::temp_dir().join(format!("amipro-folder-app-{}", std::process::id()));
    let from = root.join("Archive");
    let to = root.join("Word");
    std::fs::create_dir_all(from.join("Client")).unwrap();
    let sam = amipro_formats::sam_document(
        &crate::page_surface::page_fixture(),
        &amipro_formats::SamOptions::at(0),
    );
    std::fs::write(from.join("First.SAM"), &sam).unwrap();
    std::fs::write(from.join("Client/Second.sam"), &sam).unwrap();
    std::fs::write(from.join("Client/Broken.SAM"), b"WordPro not a SAM").unwrap();

    let mut app = page_app(page_fixture());
    assert!(app.command_enabled(CommandId::ExportFolder));
    send(&mut app, Message::Command(CommandId::ExportFolder));
    assert!(app.export_folder.is_some());
    assert!(!app.export_folder.as_ref().unwrap().ready());
    for (to_folder, folder) in [(false, from.clone()), (true, to.clone())] {
        send(
            &mut app,
            Message::ExportFolder(FolderAction::Chosen {
                to: to_folder,
                folder: Some(folder),
            }),
        );
    }
    send(
        &mut app,
        Message::ExportFolder(FolderAction::Format(Format::Odt)),
    );
    let panel = app.export_folder.as_ref().unwrap();
    assert!(panel.ready());
    assert_eq!(panel.format, Format::Odt);
    // Remembered for next time (EX10).
    assert_eq!(app.preferences.export_from.as_deref(), Some(from.as_path()));
    assert_eq!(app.preferences.export_folder.as_deref(), Some(to.as_path()));
    assert_eq!(app.preferences.export_format, "odt");

    // The job itself, as its thread runs it.
    let reports = std::cell::RefCell::new(Vec::new());
    let finished = FolderJob {
        from: from.clone(),
        subfolders: true,
        to: to.clone(),
        format: Format::Odt,
        styles: Vec::new(),
        stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }
    .run(&|progress| reports.borrow_mut().push(format!("{progress:?}")))
    .unwrap();
    assert_eq!((finished.exported, finished.not_read), (2, 1));
    assert!(to.join("First.odt").exists());
    assert!(to.join("Client/Second.odt").exists());
    let report = std::fs::read_to_string(finished.report.as_ref().unwrap()).unwrap();
    assert!(report.contains("First.SAM → First.odt"), "{report}");
    assert!(report.contains("Broken.SAM: not exported"), "{report}");
    assert!(reports.borrow()[0].contains("Started { total: 3 }"));

    // Its reports reach the panel and the status bar.
    let session_id = app.id;
    send(&mut app, Message::ExportFolder(FolderAction::Export));
    assert!(app.export_folder.as_ref().unwrap().running.is_some());
    send(
        &mut app,
        Message::ExportFolderProgress {
            session_id,
            progress: Progress::Done { done: 1 },
        },
    );
    assert_eq!(
        app.export_folder
            .as_ref()
            .unwrap()
            .running
            .as_ref()
            .unwrap()
            .done,
        1
    );
    send(&mut app, Message::ExportFolder(FolderAction::Stop));
    let stop = app
        .export_folder
        .as_ref()
        .unwrap()
        .running
        .as_ref()
        .unwrap()
        .stop
        .clone();
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed));
    send(
        &mut app,
        Message::ExportFolderProgress {
            session_id,
            progress: Progress::Finished(Ok(finished)),
        },
    );
    assert!(app.export_folder.as_ref().unwrap().running.is_none());
    assert_eq!(app.status, "Exported 2 files; 1 could not be read");
    send(&mut app, Message::ExportFolder(FolderAction::Close));
    assert!(app.export_folder.is_none());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn printing_with_update_fields_updates_every_field_first() {
    // Ami Pro's Print Options › Update fields: Update All before the job
    // (`PRINT_NOTES.md`, `1238:0000`).
    use crate::fields_panel::{FieldsAction, Page};
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    send(&mut app, Message::Command(CommandId::Field));
    app.page_selection = PageSelection::caret(at(main, 0, 0), 1);
    fields(&mut app, FieldsAction::Page(Page::Power));
    fields(&mut app, FieldsAction::Code("NumWords".into()));
    fields(&mut app, FieldsAction::Insert);
    send(&mut app, Message::FieldsPanel(FieldsAction::Close));
    let counted = app.document.paragraphs()[0].text.clone();
    // More words: the field's result is out of date.
    move_to(&mut app, at(main, 1, 0), 1, false);
    page(&mut app, PageAction::Insert("one two three ".into()));
    assert_eq!(app.document.paragraphs()[0].text, counted);
    app.print_options.update_fields = true;
    let session_id = app.id;
    send(
        &mut app,
        Message::ExportPdfChosen {
            session_id,
            result: FileDialogResult::Selected(std::env::temp_dir().join("Updated.pdf")),
        },
    );
    assert_ne!(app.document.paragraphs()[0].text, counted, "updated first");
    assert_eq!(app.status, "Exporting PDF…");
}

#[test]
fn print_sends_the_pages_the_desktops_dialog_chose() {
    // `UI_SPEC.md` §29 (PO1): File › Print… opens the desktop's dialog; its
    // choice prints in the background and the status bar follows it.
    let mut app = page_app(page_fixture());
    assert!(app.command_enabled(CommandId::Print));
    assert!(app.command_enabled(CommandId::PrintCurrentPage));
    let session_id = app.id;
    let status = app.status.clone();
    // Cancel in the dialog: nothing happens.
    send(
        &mut app,
        Message::PrintPrepared {
            session_id,
            result: Ok(None),
        },
    );
    assert_eq!(app.status, status);
    send(
        &mut app,
        Message::PrintPrepared {
            session_id,
            result: Ok(Some(crate::print_portal::Prepared {
                pages: vec![0, 1],
                paper: Some((12_240.0, 15_840.0)),
                token: 7,
            })),
        },
    );
    assert_eq!(app.status, "Printing 2 pages…");
    send(
        &mut app,
        Message::Printed {
            session_id,
            result: Ok(2),
        },
    );
    assert_eq!(app.status, "Sent 2 pages to the printer");
    send(
        &mut app,
        Message::PrintPrepared {
            session_id,
            result: Ok(Some(crate::print_portal::Prepared {
                pages: Vec::new(),
                paper: None,
                token: 8,
            })),
        },
    );
    assert_eq!(app.status, "No pages to print");
    send(
        &mut app,
        Message::PrintPrepared {
            session_id,
            result: Err("the desktop's print service: not found".into()),
        },
    );
    assert_eq!(
        app.status,
        "Print failed: the desktop's print service: not found"
    );
}

#[test]
fn the_print_panel_keeps_ami_pros_options_and_shows_the_pages_as_they_print() {
    // `UI_SPEC.md` §29 (PO1, PO4, PO5): File › Print Options… opens the
    // Print panel in the rail; its options are the document's, its PDF
    // additions the settings'; other panels close it and it closes them.
    use crate::print_panel::{Addition, PrintAction, PrintOption};
    let mut app = page_app(page_fixture());
    send(&mut app, Message::Command(CommandId::DocumentProperties));
    assert!(app.document_properties.is_some());
    send(&mut app, Message::Command(CommandId::PrintOptions));
    assert!(app.print_panel.is_some());
    assert!(app.document_properties.is_none());
    assert!(app.rail_open());
    send(
        &mut app,
        Message::PrintPanel(PrintAction::Option(PrintOption::PreprintedForm, true)),
    );
    assert!(app.print_options.preprinted_form);
    send(
        &mut app,
        Message::PrintPanel(PrintAction::Addition(Addition::Archival, true)),
    );
    assert!(app.preferences.pdf_additions.archival);
    // The options go to the PDF.
    let options = app.prepared_print(None).unwrap();
    assert!(options.mode.preprinted_form);
    assert!(options.additions.archival);
    // A second document has its own options.
    send(&mut app, Message::Command(CommandId::NewDocument));
    assert!(!app.print_options.preprinted_form);
    send(&mut app, Message::Command(CommandId::CloseDocument));
    // Back to Styles closes the panel; the options stay for the document.
    send(&mut app, Message::PrintPanel(PrintAction::Back));
    assert!(app.print_panel.is_none());
    assert!(app.print_options.preprinted_form);
    send(&mut app, Message::Command(CommandId::PrintOptions));
    send(&mut app, Message::Command(CommandId::Field));
    assert!(app.print_panel.is_none(), "the Fields panel took the rail");
}

#[test]
fn the_print_panels_buttons_take_the_rails_width() {
    // User, 2026-10-20: the buttons at the bottom were left-aligned pills
    // of three widths; libcosmic's text buttons give a width to their
    // label's row, not the button. Each now spans the rail, so a click near
    // its right edge presses it. The commands are not run: they open the
    // desktop's print and file dialogs.
    use cosmic::iced::mouse::Button;
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.send(Message::Command(CommandId::PrintOptions));
    // The 28 px status bar, the panel's 8 px padding, 32 px buttons 6 px
    // apart, from the bottom.
    let bottom = Runtime::WINDOW.height - 28.0 - 8.0 - 16.0;
    for (row, command) in [
        CommandId::ExportPdf,
        CommandId::PrintCurrentPage,
        CommandId::Print,
    ]
    .into_iter()
    .enumerate()
    {
        let y = bottom - 38.0 * row as f32;
        for x in [24.0, 290.0] {
            assert_eq!(
                commands(runtime.click_unrun(Button::Left, cosmic::iced::Point::new(x, y))),
                [command],
                "{command:?} at x = {x}"
            );
        }
    }
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Print panel beside a page shown as it
/// prints (preprinted form), in both themes.
#[test]
#[ignore = "writes screenshots"]
fn print_panel_screenshots() {
    use crate::print_panel::{PrintAction, PrintOption};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    send(&mut app, Message::Command(CommandId::PrintOptions));
    send(
        &mut app,
        Message::PrintPanel(PrintAction::Option(PrintOption::CropMarks, true)),
    );
    for (dark, name) in [(false, "window-print-light"), (true, "window-print-dark")] {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(&app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    }
}

/// `UI_SPEC.md` §30: the Export Folder panel (a run finished) and the
/// export's loss dialog, in both themes.
#[test]
#[ignore = "writes screenshots"]
fn export_screenshots() {
    use crate::export_folder::{FolderAction, Progress};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    send(&mut app, Message::Command(CommandId::ExportFolder));
    for (to, folder) in [
        (false, "/home/user/Archive/SAM"),
        (true, "/home/user/Archive/Word"),
    ] {
        send(
            &mut app,
            Message::ExportFolder(FolderAction::Chosen {
                to,
                folder: Some(folder.into()),
            }),
        );
    }
    let session_id = app.id;
    if let Some(panel) = app.export_folder.as_mut() {
        panel.running = Some(crate::export_folder::Running {
            done: 0,
            total: 0,
            stop: Default::default(),
        });
    }
    send(
        &mut app,
        Message::ExportFolderProgress {
            session_id,
            progress: Progress::Finished(Ok(crate::export_folder::Finished {
                exported: 412,
                with_losses: 17,
                not_read: 7,
                stopped: false,
                report: Some("/home/user/Archive/Word/Export report.txt".into()),
            })),
        },
    );
    let mut shots = vec![("window-export-folder", None)];
    let path = std::env::temp_dir().join("Fixture.docx");
    let mut prepared = prepared_export(&app, amipro_exchange::Format::Docx, &path);
    prepared.losses = vec![
        "protected text, written as ordinary text (3 places)".into(),
        "frame groups, written as separate frames (1 place)".into(),
    ];
    shots.push(("window-export-losses", Some(prepared)));
    for (name, pending) in shots {
        app.export_pending = pending;
        for (dark, theme_name) in [(false, "light"), (true, "dark")] {
            let theme = if dark {
                cosmic::Theme::dark()
            } else {
                cosmic::Theme::light()
            };
            let size = (1_280, 800);
            let rgb = render_window(&app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!("{name}-{theme_name}.ppm"));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    }
}

/// Files for the import tests (`UI_SPEC.md` §31) in a folder of their own.
fn import_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!("amipro-app-import-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
}

const IMPORTED_RTF: &[u8] = br"{\rtf1\ansi{\fonttbl{\f0 Arial;}}{\stylesheet{\s0 Normal;}{\s1 Heading 1;}}{\s1\b First heading\par}{\s0 Body with \i italic\i0  text\par}{\s0 Second body\par}}";

#[test]
fn other_programs_documents_open_through_import_options_in_a_protected_tab() {
    // IM1, IM2: File › Open's file goes to Import Options first (the last
    // choice preselected), is read off the interface's thread, and opens
    // in a new tab whose source stays protected (Save goes to Save As).
    use crate::import::OptionsAction;
    use amipro_exchange::import::{LineEnds, StyleOption};
    let path = import_file("Report.rtf", IMPORTED_RTF);
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.app.preferences.import_styles = "keep".into();
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        path.clone(),
    )));
    let dialog = runtime.app.import_dialog.as_ref().expect("Import Options");
    assert_eq!(dialog.options.styles, StyleOption::Keep);
    assert_eq!(runtime.app.sessions.len(), 1);
    runtime.send(Message::ImportOptions(OptionsAction::Styles(
        StyleOption::Convert,
    )));
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    assert!(runtime.app.import_dialog.is_none());
    assert_eq!(runtime.app.sessions.len(), 2);
    assert_eq!(runtime.app.active_session, 1);
    assert_eq!(runtime.app.preferences.import_styles, "convert");
    let app = &runtime.app;
    assert_eq!(app.location, DocumentLocation::LegacySource(path.clone()));
    assert_eq!(app.location.save_request(), SaveRequest::ChooseDestination);
    assert!(
        app.status.starts_with("Opened Report.rtf (Rich Text)"),
        "{}",
        app.status
    );
    let paragraphs = &app.document.main_flow().paragraphs;
    assert_eq!(paragraphs[0].text, "First heading");
    assert_eq!(
        app.document.styles()[&paragraphs[0].style].name,
        "Heading 1"
    );
    assert_eq!(
        app.document.styles()[&paragraphs[1].style].name,
        "Body Text"
    );
    assert!(!app.pages.layout().pages.is_empty());
    // Opening it again shows its tab.
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        path.clone(),
    )));
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    assert_eq!(runtime.app.sessions.len(), 2);

    // A text file shows Ami Pro's ASCII options; Join lines makes one
    // paragraph of lines a blank line ends.
    let text = import_file("notes.txt", b"one\r\ntwo\r\n\r\nthree\r\n");
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        text,
    )));
    assert_eq!(
        runtime
            .app
            .import_dialog
            .as_ref()
            .unwrap()
            .options
            .text
            .line_ends,
        LineEnds::Paragraphs
    );
    runtime.send(Message::ImportOptions(OptionsAction::LineEnds(
        LineEnds::Joined,
    )));
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    let texts: Vec<&str> = runtime
        .app
        .document
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .collect();
    assert_eq!(texts, ["one two", "three"]);
    assert_eq!(runtime.app.preferences.import_line_ends, "joined");

    // Cancel opens nothing; a file of no format read says so.
    let sessions = runtime.app.sessions.len();
    let docx = import_file(
        "Copy.docx",
        &prepared_export(&runtime.app, amipro_exchange::Format::Docx, &path).bytes,
    );
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        docx,
    )));
    runtime.send(Message::ImportOptions(OptionsAction::Cancel));
    assert_eq!(runtime.app.sessions.len(), sessions);
    assert_eq!(runtime.app.status, "Open cancelled");
    let damaged = import_file("Damaged.docx", b"PK\x03\x04 not a package");
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        damaged,
    )));
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    assert_eq!(runtime.app.sessions.len(), sessions);
    assert!(
        runtime.app.status.starts_with("Import failed: "),
        "{}",
        runtime.app.status
    );
}

#[test]
fn a_word_document_the_app_exported_opens_with_its_text_and_frames() {
    // IM1 through the app's own export: the fixture's text, styles, and
    // frames come back.
    use crate::import::OptionsAction;
    let app = page_app(page_fixture());
    let path = std::env::temp_dir().join(format!("amipro-app-import-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    for (format, name) in [
        (amipro_exchange::Format::Docx, "Fixture.docx"),
        (amipro_exchange::Format::Odt, "Fixture.odt"),
    ] {
        let file = path.join(name);
        std::fs::write(&file, prepared_export(&app, format, &file).bytes).unwrap();
        let mut runtime = Runtime::new(page_app(page_fixture()));
        runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            file,
        )));
        runtime.send(Message::ImportOptions(OptionsAction::Open));
        let opened = &runtime.app;
        assert_eq!(opened.sessions.len(), 2, "{name}: {}", opened.status);
        let original: Vec<&str> = app
            .document
            .main_flow()
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.text.trim_matches('\u{FFFC}'))
            .filter(|text| !text.is_empty())
            .collect();
        let read: Vec<&str> = opened
            .document
            .main_flow()
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.text.trim_matches('\u{FFFC}'))
            .filter(|text| !text.is_empty())
            .collect();
        assert_eq!(read, original, "{name}");
        assert_eq!(
            opened.document.frames().count(),
            app.document.frames().count(),
            "{name}"
        );
    }
}

#[test]
fn insert_file_puts_a_file_at_the_caret_as_one_undo_step() {
    // IM3: Insert › File… at the caret in the main text and in a frame's
    // text, after Import Options; the document's own styles keep their
    // definitions; Undo takes it out again. The command opens the desktop's
    // file chooser, so its result is sent as the chooser would send it.
    use crate::import::OptionsAction;
    let path = import_file("Insert.rtf", IMPORTED_RTF);
    let mut runtime = Runtime::new(page_app(page_fixture()));
    assert!(runtime.app.command_enabled(CommandId::InsertFile));
    let main = runtime.app.document.main_flow_id();
    move_to(&mut runtime.app, at(main, 1, 0), 1, false);
    let before = runtime.app.document.clone();
    let body = before
        .styles()
        .values()
        .find(|style| style.name == "Body Text")
        .cloned();
    let session_id = runtime.app.id;
    runtime.send(Message::InsertFileDialogFinished {
        session_id,
        result: FileDialogResult::Selected(path.clone()),
    });
    let dialog = runtime.app.import_dialog.as_ref().expect("Import Options");
    assert!(matches!(
        dialog.purpose,
        crate::import::Purpose::Insert { paste: false, .. }
    ));
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    let app = &runtime.app;
    assert_eq!(app.sessions.len(), 1);
    assert!(
        app.status.starts_with("Inserted Insert.rtf"),
        "{}",
        app.status
    );
    let paragraphs = &app.document.main_flow().paragraphs;
    assert_eq!(paragraphs.len(), before.main_flow().paragraphs.len() + 3);
    assert_eq!(paragraphs[0].text, before.main_flow().paragraphs[0].text);
    assert_eq!(paragraphs[1].text, "First heading");
    assert_eq!(paragraphs[4].text, before.main_flow().paragraphs[1].text);
    assert_eq!(
        app.document
            .styles()
            .values()
            .find(|style| style.name == "Body Text")
            .cloned(),
        body
    );
    assert!(app.dirty);
    send(&mut runtime.app, Message::Command(CommandId::Undo));
    assert_eq!(runtime.app.document.plain_text(), before.plain_text());

    // Into a frame's text.
    let frame = text_frame(&runtime.app.document);
    let flow = flow_of(&runtime.app.document, frame);
    move_to(&mut runtime.app, at(flow, 0, 0), 1, false);
    runtime.send(Message::InsertFileDialogFinished {
        session_id,
        result: FileDialogResult::Selected(path),
    });
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    assert!(flow_text(&runtime.app, flow).contains("First heading"));
    assert_eq!(
        runtime.app.document.main_flow().paragraphs.len(),
        before.main_flow().paragraphs.len()
    );
}

#[test]
fn paste_takes_rich_text_with_apply_styles_and_falls_back_to_text() {
    // IM12: Paste reads the clipboard's RTF first, as Ami Pro pastes it
    // (Apply styles: Body Text with each style's look); without RTF the
    // clipboard's text pastes as before.
    let mut runtime = Runtime::new(page_app(page_fixture()));
    let main = runtime.app.document.main_flow_id();
    move_to(&mut runtime.app, at(main, 2, 0), 1, false);
    let before = runtime.app.document.clone();
    runtime.clipboard = Some("plain".into());
    runtime.clipboard_data = Some((IMPORTED_RTF.to_vec(), "text/rtf".into()));
    runtime.send(Message::Command(CommandId::Paste));
    let app = &runtime.app;
    assert_eq!(app.status, "Pasted", "{}", app.status);
    let paragraphs = &app.document.main_flow().paragraphs;
    assert_eq!(paragraphs.len(), before.main_flow().paragraphs.len() + 3);
    assert_eq!(paragraphs[2].text, "First heading");
    assert_eq!(
        app.document.styles()[&paragraphs[2].style].name,
        "Body Text"
    );
    assert_eq!(
        app.document.styles().len(),
        before.styles().len(),
        "Apply styles adds no styles"
    );
    send(&mut runtime.app, Message::Command(CommandId::Undo));
    assert_eq!(runtime.app.document.plain_text(), before.plain_text());

    runtime.clipboard_data = None;
    runtime.send(Message::Command(CommandId::Paste));
    assert!(flow_text(&runtime.app, main).contains("plain"));
}

#[test]
fn a_dropped_document_opens_through_import_options() {
    // IM1: a document dropped on the page opens (pictures still go in).
    use crate::import::OptionsAction;
    let path = import_file("Dropped Letter.rtf", IMPORTED_RTF);
    let mut runtime = Runtime::new(page_app(page_fixture()));
    let session_id = runtime.app.id;
    let uri = format!("file://{}\r\n", path.display()).replace(' ', "%20");
    runtime.send(Message::Dropped {
        session_id,
        data: uri.into_bytes(),
        mime: "text/uri-list".into(),
    });
    assert!(runtime.app.import_dialog.is_some());
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    assert_eq!(runtime.app.sessions.len(), 2);
    assert_eq!(runtime.app.location, DocumentLocation::LegacySource(path));
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: Import Options for a Word Document and for
/// a text file, in both themes.
#[test]
#[ignore = "writes screenshots"]
fn import_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    for (file, name) in [
        (
            import_file("Quarterly report.rtf", IMPORTED_RTF),
            "import-options",
        ),
        (import_file("notes.txt", b"text\r\n"), "import-text-options"),
    ] {
        let mut app = page_app(page_fixture());
        drop(app.update_title());
        send(
            &mut app,
            Message::OpenDialogFinished(FileDialogResult::Selected(file)),
        );
        for (dark, theme_name) in [(false, "light"), (true, "dark")] {
            let theme = if dark {
                cosmic::Theme::dark()
            } else {
                cosmic::Theme::light()
            };
            let size = (1_280, 800);
            let rgb = render_window(&app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!("window-{name}-{theme_name}.ppm"));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    }
    // The Import Report beside the opened document.
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        import_file("Linked.rtf", LINKED_RTF),
    )));
    runtime.send(Message::ImportOptions(crate::import::OptionsAction::Open));
    drop(runtime.app.update_title());
    for (dark, theme_name) in [(false, "light"), (true, "dark")] {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(&runtime.app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("window-import-report-{theme_name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    }
}

#[test]
fn import_options_take_clicks_in_the_window() {
    // The dialog's controls through the window's widgets: the first point
    // down a column that publishes each choice, then Open reads the file.
    use crate::import::OptionsAction;
    use amipro_exchange::import::StyleOption;
    use cosmic::iced::mouse::Button;
    let path = import_file("Clicked.rtf", IMPORTED_RTF);
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        path,
    )));
    let mut find = |x: f32, wanted: &dyn Fn(&Message) -> bool| {
        (200..600).step_by(4).find_map(|y| {
            let point = cosmic::iced::Point::new(x, y as f32);
            runtime
                .click_unrun(Button::Left, point)
                .iter()
                .any(wanted)
                .then_some(point)
        })
    };
    let apply = find(390.0, &|message| {
        matches!(
            message,
            Message::ImportOptions(OptionsAction::Styles(StyleOption::Apply))
        )
    })
    .expect("Apply styles in the dialog");
    let open = find(866.0, &|message| {
        matches!(message, Message::ImportOptions(OptionsAction::Open))
    })
    .expect("Open in the dialog");
    assert!(open.y > apply.y);
    runtime.click(Button::Left, apply);
    assert_eq!(
        runtime.app.import_dialog.as_ref().unwrap().options.styles,
        StyleOption::Apply
    );
    runtime.click(Button::Left, open);
    assert_eq!(runtime.app.sessions.len(), 2, "{}", runtime.app.status);
    let paragraphs = &runtime.app.document.main_flow().paragraphs;
    assert_eq!(
        runtime.app.document.styles()[&paragraphs[0].style].name,
        "Body Text"
    );
}

/// Two links on two paragraphs and a comment: three findings of two kinds.
const LINKED_RTF: &[u8] = br#"{\rtf1\ansi{\stylesheet{\s0 Normal;}}{\s0 See {\field{\*\fldinst HYPERLINK "https://example.org"}{\fldrslt the site}} first\par}{\s0 Middle\par}{\s0 And {\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt another}} here{\*\atnid CD}{\*\atnauthor Chris Day}\chatn{\*\annotation A remark}\par}}"#;

#[test]
fn the_import_report_lists_what_changed_with_jumps_and_copy() {
    // IM8: the panel opens by itself when an import lists anything; each
    // kind's Go to walks its places; Copy puts the report on the clipboard;
    // the status bar's count opens it again.
    use crate::import::OptionsAction;
    use crate::import_report::ReportAction;
    use amipro_exchange::import::FindingKind;
    let path = import_file("Linked.rtf", LINKED_RTF);
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        path,
    )));
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    let app = &runtime.app;
    assert_eq!(app.sessions.len(), 2);
    let report = app.import_report.as_ref().expect("a report");
    let kinds: Vec<&FindingKind> = report
        .findings
        .iter()
        .map(|finding| &finding.kind)
        .collect();
    assert!(kinds.contains(&&FindingKind::Links), "{kinds:?}");
    assert!(kinds.contains(&&FindingKind::Comments), "{kinds:?}");
    assert!(app.import_report_shown && app.rail_open());
    assert!(
        app.status.contains("things are different (Import Report)"),
        "{}",
        app.status
    );
    assert!(crate::import_report::summary(report).starts_with("Opened as an Ami Pro document."));

    // Go to: the links' two places in turn, then the first again.
    let links = report
        .findings
        .iter()
        .position(|finding| finding.kind == FindingKind::Links)
        .unwrap();
    let mut seen = Vec::new();
    for _ in 0..3 {
        runtime.send(Message::ImportReport(ReportAction::GoTo(links)));
        let caret = runtime.app.page_selection.focus;
        let paragraph = &runtime.app.document.flow(caret.flow).unwrap().paragraphs[caret.paragraph];
        seen.push(paragraph.text.clone());
    }
    assert!(seen[0].starts_with("See the site"), "{seen:?}");
    assert!(seen[1].starts_with("And another"), "{seen:?}");
    assert_eq!(seen[2], seen[0]);
    assert_eq!(runtime.app.status, "1 of 2");

    // Copy.
    runtime.send(Message::ImportReport(ReportAction::Copy));
    let copied = runtime.clipboard.clone().unwrap();
    assert!(
        copied.starts_with("Import report: Linked.rtf (Rich Text)"),
        "{copied}"
    );
    assert!(copied.contains("kept as Ami Pro notes"), "{copied}");

    // Back shows Styles; the status bar's count shows the report again;
    // Close closes the rail.
    runtime.send(Message::ImportReport(ReportAction::Back));
    assert!(!runtime.app.import_report_shown);
    runtime.send(Message::ImportReport(ReportAction::Show));
    assert!(runtime.app.import_report_shown);
    runtime.send(Message::ImportReport(ReportAction::Close));
    assert!(!runtime.app.import_report_shown && !runtime.app.rail_open());

    // A file read with nothing different has no report.
    let plain = import_file("Plain.rtf", IMPORTED_RTF);
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        plain,
    )));
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    assert!(runtime.app.import_report.is_none());
}

#[test]
fn the_import_reports_buttons_take_clicks_in_the_rail() {
    // The panel's Go to and the status bar's count through the window.
    use crate::import::OptionsAction;
    use crate::import_report::ReportAction;
    use cosmic::iced::mouse::Button;
    let path = import_file("Linked clicks.rtf", LINKED_RTF);
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
        path,
    )));
    runtime.send(Message::ImportOptions(OptionsAction::Open));
    assert!(runtime.app.import_report_shown);
    let mut found = Vec::new();
    for y in (100..760).step_by(4) {
        for x in [250.0, 280.0] {
            for message in runtime.click_unrun(Button::Left, cosmic::iced::Point::new(x, y as f32))
            {
                if let Message::ImportReport(action) = message
                    && !found.contains(&action)
                {
                    found.push(action);
                }
            }
        }
    }
    assert!(found.contains(&ReportAction::GoTo(0)), "{found:?}");
    assert!(found.contains(&ReportAction::Copy), "{found:?}");
    runtime.send(Message::ImportReport(ReportAction::Back));
    let status_y = Runtime::WINDOW.height - 14.0;
    let shows = (300..900).step_by(8).any(|x| {
        runtime
            .click_unrun(Button::Left, cosmic::iced::Point::new(x as f32, status_y))
            .iter()
            .any(|message| matches!(message, Message::ImportReport(ReportAction::Show)))
    });
    assert!(shows, "the status bar's count opens the report");
}

#[test]
fn import_folder_imports_a_folder_from_the_rail() {
    // IM11: File › Import Folder… in the rail; the folders come from the
    // desktop's chooser (sent here as it answers), the last ones are kept,
    // Import runs on its own thread with its progress in the panel, and the
    // report can be opened. Choose… is clicked unrun: it opens a portal.
    use crate::import_folder::FolderAction;
    use amipro_exchange::import::StyleOption;
    use cosmic::iced::mouse::Button;
    let root =
        std::env::temp_dir().join(format!("amipro-app-import-folder-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (from, to) = (root.join("Word files"), root.join("Ami Pro"));
    std::fs::create_dir_all(from.join("2019")).unwrap();
    std::fs::write(from.join("Letter.rtf"), IMPORTED_RTF).unwrap();
    std::fs::write(from.join("2019").join("Linked.rtf"), LINKED_RTF).unwrap();
    let mut runtime = Runtime::new(page_app(page_fixture()));
    assert!(runtime.app.command_enabled(CommandId::ImportFolder));
    runtime.send(Message::Command(CommandId::ImportFolder));
    assert!(runtime.app.import_folder.is_some() && runtime.app.rail_open());
    // Import waits for both folders.
    let import_button = |runtime: &mut Runtime| {
        (100..780).step_by(4).find_map(|y| {
            let point = cosmic::iced::Point::new(150.0, y as f32);
            runtime
                .click_unrun(Button::Left, point)
                .iter()
                .any(|message| matches!(message, Message::ImportFolder(FolderAction::Import)))
                .then_some(point)
        })
    };
    assert!(import_button(&mut runtime).is_none());
    let chooses = (100..500).step_by(4).any(|y| {
        runtime
            .click_unrun(Button::Left, cosmic::iced::Point::new(40.0, y as f32))
            .iter()
            .any(|message| matches!(message, Message::ImportFolder(FolderAction::ChooseFrom)))
    });
    assert!(chooses, "Choose… for From");
    runtime.send(Message::ImportFolder(FolderAction::Chosen {
        to: false,
        folder: Some(from.clone()),
    }));
    runtime.send(Message::ImportFolder(FolderAction::Chosen {
        to: true,
        folder: Some(to.clone()),
    }));
    runtime.send(Message::ImportFolder(FolderAction::Styles(
        StyleOption::Apply,
    )));
    assert_eq!(
        runtime.app.preferences.import_from.as_deref(),
        Some(from.as_path())
    );
    assert_eq!(
        runtime.app.preferences.import_to.as_deref(),
        Some(to.as_path())
    );
    let point = import_button(&mut runtime).expect("Import, now ready");
    runtime.click(Button::Left, point);
    let panel = runtime.app.import_folder.as_ref().unwrap();
    assert!(panel.running.is_none());
    let finished = panel.finished.clone().expect("finished");
    assert_eq!(finished.imported, 2, "{finished:?}");
    assert_eq!(finished.with_findings, 1);
    assert_eq!(
        runtime.app.status,
        "Imported 2 files, 1 with something different"
    );
    assert!(to.join("Letter.amipro").exists());
    assert!(to.join("2019").join("Linked.amipro").exists());
    let report = std::fs::read_to_string(to.join("Import report.txt")).unwrap();
    assert!(report.contains("kept as Ami Pro notes"), "{report}");
    // A second run never writes over the first.
    runtime.send(Message::ImportFolder(FolderAction::Import));
    assert!(to.join("Letter (2).amipro").exists());
    assert!(to.join("Import report (2).txt").exists());
    // Another panel takes the rail; Close closes it.
    runtime.send(Message::Command(CommandId::PrintOptions));
    assert!(runtime.app.import_folder.is_none());
    runtime.send(Message::Command(CommandId::ImportFolder));
    runtime.send(Message::ImportFolder(FolderAction::Close));
    assert!(runtime.app.import_folder.is_none() && !runtime.app.rail_open());
    let _ = std::fs::remove_dir_all(root);
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Import Folder panel after a run, in
/// both themes.
#[test]
#[ignore = "writes screenshots"]
fn import_folder_screenshots() {
    use crate::import_folder::FolderAction;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let root =
        std::env::temp_dir().join(format!("amipro-shots-import-folder-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (from, to) = (root.join("Word files"), root.join("Ami Pro"));
    std::fs::create_dir_all(&from).unwrap();
    std::fs::write(from.join("Letter.rtf"), IMPORTED_RTF).unwrap();
    let mut runtime = Runtime::new(page_app(page_fixture()));
    runtime.send(Message::Command(CommandId::ImportFolder));
    for (to_folder, folder) in [(false, &from), (true, &to)] {
        runtime.send(Message::ImportFolder(FolderAction::Chosen {
            to: to_folder,
            folder: Some(folder.clone()),
        }));
    }
    runtime.send(Message::ImportFolder(FolderAction::Import));
    drop(runtime.app.update_title());
    for (dark, theme_name) in [(false, "light"), (true, "dark")] {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(&runtime.app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("window-import-folder-{theme_name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    }
    let _ = std::fs::remove_dir_all(root);
}

/// IM.15: other programs' documents opened as File › Open opens them and
/// written by Save As SAM (Save Anyway after the loss list), for Ami Pro
/// in the VM. `AMIPRO_IMPORT_TWINS` lists `NAME=FILE` pairs separated by
/// `;`, `AMIPRO_TWIN_OUT` is the folder for `NAME.SAM`.
#[test]
#[ignore = "writes Ami Pro documents for the VM"]
fn import_twins() {
    use crate::import::OptionsAction;
    let (Some(twins), Some(out)) = (
        std::env::var_os("AMIPRO_IMPORT_TWINS"),
        std::env::var_os("AMIPRO_TWIN_OUT"),
    ) else {
        eprintln!("AMIPRO_IMPORT_TWINS or AMIPRO_TWIN_OUT is not set; skipping");
        return;
    };
    let out = std::path::PathBuf::from(out);
    for pair in twins
        .to_string_lossy()
        .split(';')
        .filter(|pair| !pair.is_empty())
    {
        let (name, file) = pair.split_once('=').expect("NAME=FILE");
        let mut runtime = Runtime::new(page_app(Document::default()));
        runtime.send(Message::OpenDialogFinished(FileDialogResult::Selected(
            file.into(),
        )));
        runtime.send(Message::ImportOptions(OptionsAction::Open));
        assert_eq!(
            runtime.app.sessions.len(),
            2,
            "{name}: {}",
            runtime.app.status
        );
        if let Some(report) = &runtime.app.import_report {
            eprintln!(
                "{name} import: {}",
                report.findings.report().replace('\n', "; ")
            );
        }
        let session_id = runtime.app.id;
        let target = out.join(format!("{name}.SAM"));
        let _ = std::fs::remove_file(&target);
        runtime.send(Message::SaveDialogFinished {
            session_id,
            result: crate::FileDialogResult::Selected(out.join(name)),
            format: Some(crate::SaveFormat::Sam),
        });
        if let Some(pending) = &runtime.app.sam_save {
            eprintln!("{name} Save As lists: {:?}", pending.losses);
            runtime.send(Message::SaveSamAnyway);
        }
        assert!(target.exists(), "{name}: {}", runtime.app.status);
        eprintln!("{name}: {} pages", runtime.app.pages.layout().pages.len());
    }
}

// ----- Tables (`UI_SPEC.md` §32) -----

/// "Above" and a 3 × 3 table made by Insert Table, A1…C3 typed in it.
pub(crate) fn table_app() -> (AmiProApp, FrameId) {
    use amipro_editing::tables::{NewTable, TableTarget};
    let document = Document::from_plain_text("Above");
    let start = at(document.main_flow_id(), 0, 5);
    let mut document = amipro_editing::apply(
        &document,
        &amipro_editing::Command::InsertTable {
            target: TableTarget::Text(start),
            table: NewTable::fitting(3, 3, 9_360, 284),
        },
    )
    .unwrap()
    .document;
    let frame = *document.frame_stack().last().unwrap();
    for row in 0..3 {
        for column in 0..3 {
            let flow = cell(&document, frame, row, column);
            document = amipro_editing::apply(
                &document,
                &amipro_editing::Command::ReplaceText {
                    start: at(flow, 0, 0),
                    end: at(flow, 0, 0),
                    text: format!("{}{}", ['A', 'B', 'C'][column], row + 1),
                },
            )
            .unwrap()
            .document;
        }
    }
    let mut app = page_app(document);
    send(&mut app, Message::ZoomTo(100));
    (app, frame)
}

pub(crate) fn cell(document: &Document, frame: FrameId, row: usize, column: usize) -> FlowId {
    let FrameContent::Table(table) = &document.frame(frame).unwrap().content else {
        panic!("a table");
    };
    let amipro_document::CellContent::Text(flow) =
        table.cells[row * table.columns.len() + column].content;
    flow
}

fn caret_cell(app: &AmiProApp) -> Option<(usize, usize)> {
    crate::table_editing::caret_cell(&app.document, app.page_selection)
        .map(|(_, row, column)| (row, column))
}

fn table_rows(app: &AmiProApp, frame: FrameId) -> usize {
    match &app.document.frame(frame).unwrap().content {
        FrameContent::Table(table) => table.rows.len(),
        _ => 0,
    }
}

/// Tab and Shift+Tab move between cells; Tab in the last cell adds a row
/// as one undo step (TD5); Ctrl+arrows and Alt+Down; the column strip;
/// Delete empties selected cells, one undo step.
#[test]
fn table_keys_and_cell_selections_through_messages() {
    let (mut app, frame) = table_app();
    let target = at(cell(&app.document, frame, 0, 0), 0, 0);
    move_to(&mut app, target, 1, false);
    page(&mut app, PageAction::CellStep { forward: true });
    assert_eq!(caret_cell(&app), Some((0, 1)));
    page(&mut app, PageAction::CellStep { forward: false });
    assert_eq!(caret_cell(&app), Some((0, 0)));
    let target = at(cell(&app.document, frame, 2, 2), 0, 0);
    move_to(&mut app, target, 1, false);
    let undo = app.history.undo_len();
    page(&mut app, PageAction::CellStep { forward: true });
    assert_eq!(table_rows(&app, frame), 4);
    assert_eq!(caret_cell(&app), Some((3, 0)));
    assert_eq!(app.history.undo_len(), undo + 1);
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(table_rows(&app, frame), 3);

    // Ctrl+Right, Ctrl+Down; Shift selects cells.
    let target = at(cell(&app.document, frame, 0, 0), 0, 0);
    move_to(&mut app, target, 1, false);
    page(
        &mut app,
        PageAction::CellMove {
            down: 0,
            right: 1,
            extend: false,
        },
    );
    page(
        &mut app,
        PageAction::CellMove {
            down: 1,
            right: 0,
            extend: true,
        },
    );
    let selected = crate::table_editing::cell_selection(&app.document, app.page_selection).unwrap();
    assert_eq!((selected.anchor, selected.focus), ((0, 1), (1, 1)));

    // Alt+Down: row 0 after row 1, the caret with it.
    let target = at(cell(&app.document, frame, 0, 0), 0, 0);
    move_to(&mut app, target, 1, false);
    page(&mut app, PageAction::MoveRows { down: true });
    assert_eq!(flow_text(&app, cell(&app.document, frame, 1, 0)), "A1");
    assert_eq!(caret_cell(&app), Some((1, 0)));
    send(&mut app, Message::Command(CommandId::Undo));

    // The strip above column 1, then Delete: its cells empty, the rows kept.
    page(
        &mut app,
        PageAction::SelectCells {
            frame,
            anchor: (0, 1),
            focus: (2, 1),
        },
    );
    let undo = app.history.undo_len();
    page(&mut app, PageAction::Delete);
    for row in 0..3 {
        assert_eq!(flow_text(&app, cell(&app.document, frame, row, 1)), "");
        assert_eq!(
            flow_text(&app, cell(&app.document, frame, row, 0)),
            format!("A{}", row + 1)
        );
    }
    assert_eq!(app.history.undo_len(), undo + 1);
    // Typing with cells selected types in the focus cell.
    page(
        &mut app,
        PageAction::SelectCells {
            frame,
            anchor: (0, 0),
            focus: (1, 2),
        },
    );
    page(&mut app, PageAction::Insert("x".into()));
    assert_eq!(flow_text(&app, cell(&app.document, frame, 1, 2)), "xC2");
    assert_eq!(flow_text(&app, cell(&app.document, frame, 0, 0)), "A1");
}

fn table_key(
    runtime: &mut Runtime,
    key: cosmic::iced::keyboard::Key,
    modifiers: cosmic::iced::keyboard::Modifiers,
) {
    runtime.events(&[(
        cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: cosmic::iced::keyboard::key::Physical::Unidentified(
                cosmic::iced::keyboard::key::NativeCode::Unidentified,
            ),
            location: cosmic::iced::keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        }),
        cosmic::iced::Point::ORIGIN,
    )]);
    // The keys come up again.
    runtime.events(&[(
        cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::ModifiersChanged(
            cosmic::iced::keyboard::Modifiers::default(),
        )),
        cosmic::iced::Point::ORIGIN,
    )]);
}

/// The same through the window: a click in a cell, Tab, a drag across
/// cells (a cell selection), Delete, and a click on the strip above a
/// column.
#[test]
fn table_keys_and_cell_selections_through_the_window() {
    use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
    use cosmic::iced::mouse::Button;
    let (app, frame) = table_app();
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let cells = {
        let session = &runtime.app.sessions[runtime.app.active_session];
        session.pages.layout().pages[0]
            .frames
            .iter()
            .find_map(|frame_box| match &frame_box.kind {
                amipro_layout::FrameBoxKind::Table { cells, .. } if frame_box.frame == frame => {
                    Some(cells.clone())
                }
                _ => None,
            })
            .unwrap()
    };
    let centre = |runtime: &mut Runtime, index: usize| {
        let cell = cells[index];
        runtime.on_page(cell.x + 200.0, cell.y + cell.height / 2.0)
    };
    let a1 = centre(&mut runtime, 0);
    runtime.click(Button::Left, a1);
    assert_eq!(caret_cell(&runtime.app), Some((0, 0)));
    table_key(&mut runtime, Key::Named(Named::Tab), Modifiers::default());
    assert_eq!(caret_cell(&runtime.app), Some((0, 1)));
    table_key(&mut runtime, Key::Named(Named::Tab), Modifiers::SHIFT);
    assert_eq!(caret_cell(&runtime.app), Some((0, 0)));

    // A drag from B1 into C2 selects four cells; Delete empties them.
    let (b1, c2) = (centre(&mut runtime, 1), centre(&mut runtime, 5));
    runtime.drag(b1, c2);
    let selected =
        crate::table_editing::cell_selection(&runtime.app.document, runtime.app.page_selection)
            .expect("a cell selection");
    assert_eq!(selected.range().rows, 0..2);
    assert_eq!(selected.range().columns, 1..3);
    table_key(
        &mut runtime,
        Key::Named(Named::Delete),
        Modifiers::default(),
    );
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 1, 2)),
        ""
    );
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 2, 2)),
        "C3"
    );

    // The strip just above column 0 selects the column.
    let strip = runtime.on_page(cells[0].x + 200.0, cells[0].y - 20.0);
    runtime.click(Button::Left, strip);
    let column =
        crate::table_editing::cell_selection(&runtime.app.document, runtime.app.page_selection)
            .expect("the column");
    assert_eq!((column.range().rows, column.range().columns), (0..3, 0..1));
    // Ctrl+Down from a cell: the cell below.
    runtime.click(Button::Left, a1);
    table_key(&mut runtime, Key::Named(Named::ArrowDown), Modifiers::CTRL);
    assert_eq!(caret_cell(&runtime.app), Some((1, 0)));
}

/// Insert › Table…'s dialog and the Table menu's commands, each one undo
/// step, with their check marks and Connect's label (`UI_SPEC.md` §32).
#[test]
fn insert_table_and_the_table_menu_through_messages() {
    use crate::table_commands::InsertTableAction;
    let mut app = page_app(Document::from_plain_text("Above"));
    send(&mut app, Message::ZoomTo(100));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 5), 1, false);
    assert!(app.command_enabled(CommandId::Table));
    assert!(
        !app.command_enabled(CommandId::InsertRowAbove),
        "not in a table"
    );
    send(&mut app, Message::Command(CommandId::Table));
    assert!(app.insert_table_dialog.is_some());
    send(
        &mut app,
        Message::InsertTable(InsertTableAction::Pick(3, 2)),
    );
    let undo = app.history.undo_len();
    send(&mut app, Message::InsertTable(InsertTableAction::Insert));
    assert!(app.insert_table_dialog.is_none());
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(app.preferences.insert_table, Some((3, 2)));
    let frame = *app.document.frame_stack().last().unwrap();
    assert_eq!(table_rows(&app, frame), 2);
    assert_eq!(caret_cell(&app), Some((0, 0)));
    assert!(!app.command_enabled(CommandId::Table), "not inside a table");
    let columns = |app: &AmiProApp| match &app.document.frame(frame).unwrap().content {
        FrameContent::Table(table) => table.columns.len(),
        _ => 0,
    };
    let run = |app: &mut AmiProApp, command| {
        let undo = app.history.undo_len();
        send(app, Message::Command(command));
        assert_eq!(
            app.history.undo_len(),
            undo + 1,
            "{command:?}: one undo step"
        );
    };
    run(&mut app, CommandId::InsertRowBelow);
    assert_eq!(table_rows(&app, frame), 3);
    // TD4: a column with no room narrows the others.
    run(&mut app, CommandId::InsertColumnRight);
    assert_eq!(columns(&app), 4);
    let width = match &app.document.frame(frame).unwrap().content {
        FrameContent::Table(table) => amipro_editing::tables::grid_size(table).width,
        _ => 0,
    };
    assert!(width <= 9_360, "{width}");
    // Connect a selection, then the item reads Disconnect Cells.
    page(
        &mut app,
        PageAction::SelectCells {
            frame,
            anchor: (0, 0),
            focus: (1, 1),
        },
    );
    assert_eq!(app.table_menu_label(CommandId::ConnectCells), None);
    run(&mut app, CommandId::ConnectCells);
    let target = at(cell(&app.document, frame, 0, 0), 0, 0);
    move_to(&mut app, target, 1, false);
    assert_eq!(
        app.table_menu_label(CommandId::ConnectCells),
        Some("Disconnect Cells")
    );
    run(&mut app, CommandId::ConnectCells);
    // Leaders, Protect Cells, and Headings show their state as checks.
    assert_eq!(menu_check(&app, CommandId::LeaderNone), Some(true));
    run(&mut app, CommandId::LeaderDots);
    assert_eq!(menu_check(&app, CommandId::LeaderDots), Some(true));
    assert_eq!(menu_check(&app, CommandId::LeaderNone), Some(false));
    run(&mut app, CommandId::ProtectCells);
    assert_eq!(menu_check(&app, CommandId::ProtectCells), Some(true));
    run(&mut app, CommandId::TableHeadings);
    assert_eq!(menu_check(&app, CommandId::TableHeadings), Some(true));
    run(&mut app, CommandId::TableStyleBandedRows);
    run(&mut app, CommandId::DistributeColumns);
    // The right-click menu in a table starts with its commands.
    let menu = app.page_context_menu();
    let Some(MenuEntry::Item { label, .. }) = menu.first() else {
        panic!("a first item");
    };
    assert_eq!(label, "Table Properties…");
    // Select ▸ Column, then Delete Column: the column's cells go.
    send(&mut app, Message::Command(CommandId::SelectColumn));
    assert!(crate::table_editing::cell_selection(&app.document, app.page_selection).is_some());
    run(&mut app, CommandId::DeleteColumn);
    assert_eq!(columns(&app), 3);
    run(&mut app, CommandId::DeleteRow);
    assert_eq!(table_rows(&app, frame), 2);
    // Table to text and back.
    run(&mut app, CommandId::TableToText);
    assert!(app.document.frame(frame).is_none());
    let paragraphs = app.document.paragraphs().len();
    assert!(paragraphs >= 2, "{paragraphs}");
    move_to(&mut app, at(main, 1, 0), 1, false);
    let end = app.document.paragraphs()[paragraphs - 1].text.len();
    move_to(&mut app, at(main, paragraphs - 1, end), 1, true);
    assert!(app.command_enabled(CommandId::TextToTable));
    run(&mut app, CommandId::TextToTable);
    let made = *app.document.frame_stack().last().unwrap();
    assert!(matches!(
        app.document.frame(made).unwrap().content,
        FrameContent::Table(_)
    ));
    let target = at(cell(&app.document, made, 0, 0), 0, 0);
    move_to(&mut app, target, 1, false);
    run(&mut app, CommandId::DeleteTable);
    assert!(app.document.frame(made).is_none());
}

/// Table Properties in the rail: each change one undo step, the colour
/// buttons, and its buttons taking clicks in the window (TD2, TD8).
#[test]
fn table_properties_edit_the_table_live_in_the_rail() {
    use crate::table_properties::{Field, Side, TableAction};
    use cosmic::iced::mouse::Button;
    let (mut app, frame) = table_app();
    let target = at(cell(&app.document, frame, 0, 0), 0, 0);
    move_to(&mut app, target, 1, false);
    send(&mut app, Message::Command(CommandId::TableProperties));
    assert!(app.table_properties.is_some());
    assert!(app.rail_open());
    let rail = |app: &mut AmiProApp, action: TableAction| {
        let undo = app.history.undo_len();
        send(app, Message::TableProperties(action.clone()));
        assert_eq!(
            app.history.undo_len(),
            undo + 1,
            "{action:?}: one undo step"
        );
    };
    let table = |app: &AmiProApp| match &app.document.frame(frame).unwrap().content {
        FrameContent::Table(table) => table.clone(),
        _ => unreachable!(),
    };
    send(
        &mut app,
        Message::TableProperties(TableAction::Text(Field::Width, "1.5".into())),
    );
    rail(&mut app, TableAction::Submit(Field::Width));
    assert_eq!(table(&app).columns[0], 2_160);
    rail(&mut app, TableAction::Lines(Side::All));
    assert_eq!(table(&app).cells[0].format.lines.word(), 0x1111);
    rail(&mut app, TableAction::Center(true));
    assert!(table(&app).layout.center);
    rail(&mut app, TableAction::LineAround(true));
    assert!(app.document.frame(frame).unwrap().style.border.sides != amipro_document::Sides::NONE);
    rail(&mut app, TableAction::Automatic(false));
    assert!(!table(&app).row_options.automatic);
    // The fill and the lines' colour through their colour panels.
    send(
        &mut app,
        Message::Colour(crate::colour_control::ColourAction::Toggle(
            crate::colour_control::Target::TableFill,
            amipro_document::WHITE,
        )),
    );
    send(
        &mut app,
        Message::Colour(crate::colour_control::ColourAction::Choose(0x00_FFFF)),
    );
    assert_eq!(table(&app).cells[0].format.fill, Some(0x00_FFFF));
    send(
        &mut app,
        Message::Colour(crate::colour_control::ColourAction::Toggle(
            crate::colour_control::Target::TableLines,
            0,
        )),
    );
    send(
        &mut app,
        Message::Colour(crate::colour_control::ColourAction::Choose(0x00_00FF)),
    );
    assert_eq!(table(&app).cells[0].format.line_colour, 0x00_00FF);
    // The caret out of the table closes the panel.
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    assert!(app.table_properties.is_none());

    // Through the window: the panel's buttons take clicks.
    move_to(&mut app, target, 1, false);
    send(&mut app, Message::Command(CommandId::TableProperties));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let mut found = Vec::new();
    let scan = |runtime: &mut Runtime, found: &mut Vec<TableAction>| {
        for y in (90..780).step_by(6) {
            for x in (16..320).step_by(24) {
                for message in
                    runtime.click_unrun(Button::Left, cosmic::iced::Point::new(x as f32, y as f32))
                {
                    if let Message::TableProperties(action) = message
                        && !found.contains(&action)
                    {
                        found.push(action);
                    }
                }
            }
        }
    };
    scan(&mut runtime, &mut found);
    assert!(found.contains(&TableAction::Style(
        amipro_editing::tables::TableStyle::Grid
    )));
    // The Table section folded, the others show.
    runtime.send(Message::TableProperties(TableAction::Section(
        crate::table_properties::Section::Table,
    )));
    runtime.redraw();
    scan(&mut runtime, &mut found);
    for wanted in [
        TableAction::Style(amipro_editing::tables::TableStyle::Grid),
        TableAction::Lines(Side::Outline),
        TableAction::RemoveLines,
        TableAction::DistributeColumns,
        TableAction::Insert {
            rows: true,
            after: true,
        },
    ] {
        assert!(found.contains(&wanted), "{wanted:?} in {found:?}");
    }
}

/// Insert › Table…'s dialog through the window: a square of its grid picks
/// the size, Insert makes the table (one undo step).
#[test]
fn the_insert_table_dialog_takes_clicks_in_the_window() {
    use crate::table_commands::InsertTableAction;
    use cosmic::iced::mouse::Button;
    let mut app = page_app(Document::from_plain_text("Above"));
    send(&mut app, Message::ZoomTo(100));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 5), 1, false);
    send(&mut app, Message::Command(CommandId::Table));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let mut found: Vec<(InsertTableAction, cosmic::iced::Point)> = Vec::new();
    for y in (120..700).step_by(12) {
        for x in (300..1000).step_by(12) {
            let point = cosmic::iced::Point::new(x as f32, y as f32);
            for message in runtime.click_unrun(Button::Left, point) {
                if let Message::InsertTable(action) = message
                    && !found.iter().any(|(seen, _)| *seen == action)
                {
                    found.push((action, point));
                }
            }
        }
    }
    let point = |wanted: InsertTableAction| {
        found
            .iter()
            .find(|(action, _)| *action == wanted)
            .map(|(_, point)| *point)
            .unwrap_or_else(|| panic!("{wanted:?} in {found:?}"))
    };
    let pick = point(InsertTableAction::Pick(4, 3));
    let insert = point(InsertTableAction::Insert);
    runtime.click(Button::Left, pick);
    runtime.click(Button::Left, insert);
    assert!(runtime.app.insert_table_dialog.is_none());
    let frame = *runtime.app.document.frame_stack().last().unwrap();
    let FrameContent::Table(table) = &runtime.app.document.frame(frame).unwrap().content else {
        panic!("a table");
    };
    assert_eq!((table.columns.len(), table.rows.len()), (4, 3));
    assert_eq!(runtime.app.history.undo_len(), 1);
}

/// Screenshots of the table interface in light and dark (`UI_SPEC.md` §32):
/// a table with a cell selection, Table Properties' sections, the Insert
/// Table dialog with Layout…, and the Table menu's right-click items.
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn table_screenshots() {
    use crate::table_properties::{Section, Side, TableAction};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let (mut app, frame) = table_app();
    drop(app.update_title());
    let write = |app: &AmiProApp, name: &str| {
        for (dark, theme) in [
            (false, cosmic::Theme::light()),
            (true, cosmic::Theme::dark()),
        ] {
            let size = (1_280, 800);
            let rgb = render_window(app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!(
                "window-{name}-{}.ppm",
                if dark { "dark" } else { "light" }
            ));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    };
    let target = at(cell(&app.document, frame, 0, 0), 0, 0);
    move_to(&mut app, target, 1, false);
    send(
        &mut app,
        Message::TableProperties(TableAction::Lines(Side::All)),
    );
    send(&mut app, Message::Command(CommandId::TableProperties));
    send(
        &mut app,
        Message::TableProperties(TableAction::Lines(Side::All)),
    );
    page(
        &mut app,
        PageAction::SelectCells {
            frame,
            anchor: (1, 1),
            focus: (2, 2),
        },
    );
    write(&app, "table-properties-table");
    send(
        &mut app,
        Message::TableProperties(TableAction::Section(Section::Table)),
    );
    write(&app, "table-properties-cells");
    send(&mut app, Message::TableProperties(TableAction::Close));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Table));
    send(
        &mut app,
        Message::InsertTable(crate::table_commands::InsertTableAction::ToggleLayout),
    );
    write(&app, "insert-table");
}

/// A table's column line dragged through the window (TD6, `1498:2d2a`):
/// that column's width changes, the others keep theirs, one undo step;
/// Disable mouse sizing turns it off.
#[test]
fn a_tables_column_line_drags_through_the_window() {
    let (app, frame) = table_app();
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let cells = {
        let session = &runtime.app.sessions[runtime.app.active_session];
        session.pages.layout().pages[0]
            .frames
            .iter()
            .find_map(|frame_box| match &frame_box.kind {
                amipro_layout::FrameBoxKind::Table { cells, .. } if frame_box.frame == frame => {
                    Some(cells.clone())
                }
                _ => None,
            })
            .unwrap()
    };
    let columns = |runtime: &Runtime| match &runtime.app.document.frame(frame).unwrap().content {
        FrameContent::Table(table) => table.columns.clone(),
        _ => unreachable!(),
    };
    let before = columns(&runtime);
    let line_x = cells[0].x + cells[0].width + 43.0;
    let middle_y = cells[0].y + cells[0].height / 2.0;
    let from = runtime.on_page(line_x, middle_y);
    let to = runtime.on_page(line_x - 600.0, middle_y);
    let published = runtime.drag(from, to);
    assert!(
        published.iter().any(|message| matches!(
            message,
            Message::Page(PageAction::ResizeTrack {
                column: true,
                index: 0,
                ..
            })
        )),
        "{published:?}"
    );
    let after = columns(&runtime);
    assert!(
        (before[0] - after[0] - 600).abs() <= 60,
        "{before:?} {after:?}"
    );
    assert_eq!(after[1..], before[1..]);
    assert_eq!(runtime.app.history.undo_len(), 1);
    // Disable mouse sizing: the line is not taken.
    send(
        &mut runtime.app,
        Message::TableProperties(crate::table_properties::TableAction::FixedSizes(true)),
    );
    let target = at(cell(&runtime.app.document, frame, 0, 0), 0, 0);
    move_to(&mut runtime.app, target, 1, false);
    send(
        &mut runtime.app,
        Message::Command(CommandId::TableProperties),
    );
    send(
        &mut runtime.app,
        Message::TableProperties(crate::table_properties::TableAction::FixedSizes(true)),
    );
    runtime.redraw();
    let line_x = cells[0].x + (after[0] as f32) + 43.0;
    let from = runtime.on_page(line_x, middle_y);
    let to = runtime.on_page(line_x + 600.0, middle_y);
    let published = runtime.drag(from, to);
    assert!(
        !published
            .iter()
            .any(|message| matches!(message, Message::Page(PageAction::ResizeTrack { .. }))),
        "{published:?}"
    );
}

/// Copy, Cut, and Paste of cells through the clipboard (`1240:2008`,
/// `13b0:4065`; TD9): the clipboard gets the cells' text, a paste in a
/// table pastes the cells, tabbed text from elsewhere fills cells, and a
/// whole table copied pastes in the text as a new table; Cut empties the
/// cells. One undo step each.
#[test]
fn cells_copy_cut_and_paste_through_the_clipboard() {
    let (app, frame) = table_app();
    let mut runtime = Runtime::new(app);
    let select = |runtime: &mut Runtime, anchor, focus| {
        runtime.send(Message::Page(PageAction::SelectCells {
            frame,
            anchor,
            focus,
        }));
    };
    select(&mut runtime, (0, 0), (0, 1));
    runtime.send(Message::Command(CommandId::Copy));
    assert_eq!(runtime.clipboard.as_deref(), Some("A1\tB1"));
    let target = at(cell(&runtime.app.document, frame, 2, 1), 0, 0);
    move_to(&mut runtime.app, target, 1, false);
    let undo = runtime.app.history.undo_len();
    runtime.send(Message::Command(CommandId::Paste));
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 2, 1)),
        "A1"
    );
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 2, 2)),
        "B1"
    );
    assert_eq!(runtime.app.history.undo_len(), undo + 1);
    // Tabbed text from elsewhere fills cells from the caret's.
    runtime.clipboard = Some("x\ty\nz".into());
    let target = at(cell(&runtime.app.document, frame, 0, 0), 0, 0);
    move_to(&mut runtime.app, target, 1, false);
    runtime.send(Message::Command(CommandId::Paste));
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 0, 1)),
        "y"
    );
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 1, 0)),
        "z"
    );
    // Cut empties the cells and keeps them for a paste.
    select(&mut runtime, (1, 1), (1, 2));
    runtime.send(Message::Command(CommandId::Cut));
    assert_eq!(runtime.clipboard.as_deref(), Some("B2\tC2"));
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 1, 1)),
        ""
    );
    // The whole table, pasted in the text: a new table with its text.
    select(&mut runtime, (0, 0), (2, 2));
    runtime.send(Message::Command(CommandId::Copy));
    let main = runtime.app.document.main_flow_id();
    move_to(&mut runtime.app, at(main, 0, 0), 1, false);
    let tables = runtime.app.document.frame_stack().len();
    let undo = runtime.app.history.undo_len();
    runtime.send(Message::Command(CommandId::Paste));
    assert_eq!(runtime.app.document.frame_stack().len(), tables + 1);
    let pasted = *runtime.app.document.frame_stack().last().unwrap();
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, pasted, 0, 1)),
        "y"
    );
    assert_eq!(runtime.app.history.undo_len(), undo + 1, "one step");
}

/// Selected cells dragged through the window (TD9): a row moves past the
/// row it is dropped on, a block of cells moves over the cells there (one
/// undo step each); with Ctrl a block is copied.
#[test]
fn selected_cells_drag_and_drop_through_the_window() {
    let (app, frame) = table_app();
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let cells = {
        let session = &runtime.app.sessions[runtime.app.active_session];
        session.pages.layout().pages[0]
            .frames
            .iter()
            .find_map(|frame_box| match &frame_box.kind {
                amipro_layout::FrameBoxKind::Table { cells, .. } if frame_box.frame == frame => {
                    Some(cells.clone())
                }
                _ => None,
            })
            .unwrap()
    };
    let centre = |runtime: &mut Runtime, index: usize| {
        let cell = cells[index];
        runtime.on_page(cell.x + 150.0, cell.y + cell.height / 2.0)
    };
    // Row 0 selected, dragged onto row 2: it goes after row 2.
    runtime.send(Message::Page(PageAction::SelectCells {
        frame,
        anchor: (0, 0),
        focus: (0, 2),
    }));
    let (from, to) = (centre(&mut runtime, 1), centre(&mut runtime, 7));
    let undo = runtime.app.history.undo_len();
    let published = runtime.drag(from, to);
    assert!(
        published
            .iter()
            .any(|message| matches!(message, Message::Page(PageAction::DropCells { .. }))),
        "{published:?}"
    );
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 2, 0)),
        "A1"
    );
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 0, 0)),
        "A2"
    );
    assert_eq!(runtime.app.history.undo_len(), undo + 1);
    send(&mut runtime.app, Message::Command(CommandId::Undo));
    // A block, A1 alone with B1, moved onto B3: B3, C3 take them, A1 and B1
    // are emptied.
    runtime.send(Message::Page(PageAction::SelectCells {
        frame,
        anchor: (0, 0),
        focus: (0, 1),
    }));
    runtime.redraw();
    let (from, to) = (centre(&mut runtime, 0), centre(&mut runtime, 7));
    runtime.drag(from, to);
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 2, 1)),
        "A1"
    );
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 2, 2)),
        "B1"
    );
    assert_eq!(
        flow_text(&runtime.app, cell(&runtime.app.document, frame, 0, 0)),
        ""
    );
}

/// Release-mode timings of the table commands (`UI_SPEC.md` §32): on a
/// 100-row, 5-column table after a page of text, each command end to end
/// (update, view, layout, draw), with its undo; and typing in a cell.
#[test]
#[ignore = "release-mode performance baseline"]
fn table_command_performance_baseline() {
    use amipro_editing::tables::{NewTable, TableTarget};
    const SAMPLES: usize = 20;
    let text: Vec<String> = (0..40)
        .map(|line| format!("Line {line} of the text before the table."))
        .collect();
    let document = Document::from_plain_text(&text.join("\n"));
    let at_end = at(document.main_flow_id(), 39, 0);
    let mut document = amipro_editing::apply(
        &document,
        &amipro_editing::Command::InsertTable {
            target: TableTarget::Text(at_end),
            table: NewTable::fitting(5, 100, 9_360, 284),
        },
    )
    .unwrap()
    .document;
    let frame = *document.frame_stack().last().unwrap();
    for row in 0..100 {
        let flow = cell(&document, frame, row, 0);
        document = amipro_editing::apply(
            &document,
            &amipro_editing::Command::ReplaceText {
                start: at(flow, 0, 0),
                end: at(flow, 0, 0),
                text: format!("Row {row}"),
            },
        )
        .unwrap()
        .document;
    }
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut app = page_app(document);
    let mut harness = FrameHarness::new(window);
    let target = at(cell(&app.document, frame, 50, 2), 0, 0);
    move_to(&mut app, target, 1, false);
    harness.frame(&app);
    eprintln!("table commands: 100 × 5 table, {SAMPLES} samples, release");
    for command in [
        CommandId::InsertRowBelow,
        CommandId::InsertColumnRight,
        CommandId::LeaderDots,
        CommandId::TableStyleGrid,
        CommandId::DistributeColumns,
    ] {
        let mut run = Vec::new();
        let mut undo = Vec::new();
        for _ in 0..SAMPLES {
            let started = Instant::now();
            send(&mut app, Message::Command(command));
            harness.frame(&app);
            run.push(started.elapsed());
            let started = Instant::now();
            send(&mut app, Message::Command(CommandId::Undo));
            harness.frame(&app);
            undo.push(started.elapsed());
        }
        eprintln!(
            "  {command:?}: {}; undo {}",
            summary(&mut run),
            summary(&mut undo)
        );
    }
    let mut typing = Vec::new();
    for _ in 0..SAMPLES {
        let started = Instant::now();
        page(&mut app, PageAction::Insert("e".into()));
        harness.frame(&app);
        typing.push(started.elapsed());
    }
    eprintln!("  typing in a cell: {}", summary(&mut typing));
    let mut tab = Vec::new();
    for _ in 0..SAMPLES {
        let started = Instant::now();
        page(&mut app, PageAction::CellStep { forward: true });
        harness.frame(&app);
        tab.push(started.elapsed());
    }
    eprintln!("  Tab to the next cell: {}", summary(&mut tab));
}

/// The right button in a table's cell (user, 2026-10-24: with the table
/// selected as a frame, a right-click in its first cell showed only Frame
/// Properties): the caret goes to the cell, the frame is let go, and the
/// menu starts with Table Properties….
#[test]
fn a_right_click_in_a_table_cell_offers_the_table_commands() {
    use cosmic::iced::mouse::Button;
    let (mut app, frame) = table_app();
    app.sessions[0].frame_selection = Some(FrameSelection::one(frame, 1));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let cells = {
        let session = &runtime.app.sessions[runtime.app.active_session];
        session.pages.layout().pages[0]
            .frames
            .iter()
            .find_map(|frame_box| match &frame_box.kind {
                amipro_layout::FrameBoxKind::Table { cells, .. } if frame_box.frame == frame => {
                    Some(cells.clone())
                }
                _ => None,
            })
            .unwrap()
    };
    let first = runtime.on_page(cells[0].x + 150.0, cells[0].y + cells[0].height / 2.0);
    runtime.click(Button::Right, first);
    assert!(runtime.app.frame_selection.is_none(), "the frame let go");
    assert_eq!(caret_cell(&runtime.app), Some((0, 0)));
    let menu = runtime.app.page_context_menu();
    let Some(MenuEntry::Item { label, .. }) = menu.first() else {
        panic!("a first item");
    };
    assert_eq!(label, "Table Properties…");
}

/// Draw mode through the window (`UI_SPEC.md` §33): Insert › Drawing…
/// makes Ami Pro's frame at the caret and opens Draw mode with the draw bar
/// in the toolbar's place; a drag with the Rectangle tool draws one, one
/// undo step; a click selects it, Delete removes it, Undo brings it back;
/// Esc leaves with the frame selected; Enter opens it again, and
/// a click outside the frame leaves and acts there (DW9).
#[test]
fn draw_mode_draws_selects_deletes_and_leaves_through_the_window() {
    use crate::draw_mode::Tool;
    use amipro_pictures::sdw::Shape;
    use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
    use cosmic::iced::mouse::Button;
    let mut app = page_app(Document::from_plain_text("One.\nTwo."));
    send(&mut app, Message::ZoomTo(100));
    assert!(app.command_enabled(CommandId::Drawing));
    assert!(!app.command_enabled(CommandId::DrawGroup));
    send(&mut app, Message::Command(CommandId::Drawing));
    let frame = app.draw.as_ref().expect("Draw mode").frame;
    let FrameContent::Picture(picture) = &app.document.frame(frame).unwrap().content else {
        panic!("a picture frame");
    };
    assert_eq!(picture.extension, ".sdw");
    assert_eq!(
        picture.scaling.sizing,
        amipro_document::PictureSizing::Original
    );
    assert!(app.command_enabled(CommandId::DrawDone));
    let objects = |app: &AmiProApp| {
        amipro_editing::drawing::drawing_of(&app.document, frame)
            .unwrap()
            .objects
    };
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let (placement, _) = runtime.app.sessions[0].draw_placement().expect("placed");
    let at = |runtime: &mut Runtime, x: f32, y: f32| {
        let (px, py) = placement.to_page((f64::from(x), f64::from(y)));
        runtime.on_page(px, py)
    };
    send(&mut runtime.app, Message::DrawTool(Tool::Rectangle));
    let steps = runtime.app.history.undo_len();
    let (from, to) = (
        at(&mut runtime, 200.0, 200.0),
        at(&mut runtime, 1_200.0, 800.0),
    );
    let published = runtime.drag(from, to);
    assert!(
        published.iter().any(|message| matches!(
            message,
            Message::Draw(crate::draw_mode::Event::Release { .. })
        )),
        "{published:?}"
    );
    let drawn = objects(&runtime.app);
    assert_eq!(drawn.len(), 1);
    assert!(matches!(drawn[0].shape, Shape::Rectangle { .. }));
    let rect = drawn[0].rect;
    assert!(
        (rect.width() - 1_000).abs() <= 20 && (rect.height() - 600).abs() <= 20,
        "{rect:?}"
    );
    assert_eq!(runtime.app.history.undo_len(), steps + 1);
    // The selection tool: a click on its line selects it; Delete removes
    // it; Undo brings it back.
    send(&mut runtime.app, Message::DrawTool(Tool::Select));
    let edge = at(&mut runtime, f32::from(rect.left), 500.0);
    runtime.click(Button::Left, edge);
    assert_eq!(runtime.app.draw.as_ref().unwrap().selection, [0]);
    runtime.events(&[key_press(Key::Named(Named::Delete), Modifiers::empty())]);
    assert!(objects(&runtime.app).is_empty());
    runtime.send(Message::Command(CommandId::Undo));
    assert_eq!(objects(&runtime.app).len(), 1);
    // Esc leaves with the frame selected.
    let published = runtime.events(&[key_press(Key::Named(Named::Escape), Modifiers::empty())]);
    assert!(runtime.app.draw.is_none(), "{published:?}");
    assert!(runtime.app.frame_selection.is_some());
    // Enter on the selected drawing opens Draw mode again (a double-click
    // does too; its timing is the runtime's).
    runtime.events(&[key_press(Key::Named(Named::Enter), Modifiers::empty())]);
    assert!(runtime.app.draw.is_some(), "Enter opens the drawing");
    // A click in the text below leaves it.
    let below = runtime.on_page(6_000.0, 4_500.0);
    let published = runtime.click(Button::Left, below);
    assert!(runtime.app.draw.is_none(), "{published:?}");
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: Draw mode in light and dark (`UI_SPEC.md`
/// §33): the draw bar in the toolbar's place, a drawing with a few shapes,
/// one selected, the grid shown.
#[test]
#[ignore = "needs AMIPRO_SCREENSHOT_DIR"]
fn drawing_screenshots() {
    use crate::draw_mode::{Event, Keys, Tool};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        return;
    };
    let mut app = page_app(Document::from_plain_text(
        "A drawing below.\nText after the drawing.\nMore text.",
    ));
    send(&mut app, Message::ZoomTo(100));
    send(&mut app, Message::Command(CommandId::Drawing));
    let drag = |app: &mut AmiProApp, tool: Tool, a: (f64, f64), b: (f64, f64)| {
        send(app, Message::DrawTool(tool));
        let keys = Keys::default();
        send(
            app,
            Message::Draw(Event::Press {
                at: a,
                keys,
                double: false,
            }),
        );
        send(app, Message::Draw(Event::Drag { at: b, keys }));
        send(app, Message::Draw(Event::Release { at: b, keys }));
    };
    send(
        &mut app,
        Message::DrawStyle(amipro_editing::drawing::StyleChange {
            foreground: Some(amipro_pictures::sdw::Colour::rgb(80, 140, 230)),
            ..Default::default()
        }),
    );
    drag(&mut app, Tool::Ellipse, (200.0, 200.0), (1_400.0, 1_000.0));
    send(
        &mut app,
        Message::DrawStyle(amipro_editing::drawing::StyleChange {
            foreground: Some(amipro_pictures::sdw::Colour::rgb(240, 200, 60)),
            pattern: Some(9),
            width: Some(40),
            ..Default::default()
        }),
    );
    drag(
        &mut app,
        Tool::RoundedRectangle,
        (1_000.0, 700.0),
        (2_200.0, 1_500.0),
    );
    drag(&mut app, Tool::Line, (300.0, 1_700.0), (2_400.0, 1_800.0));
    send(&mut app, Message::Command(CommandId::DrawGrid));
    send(&mut app, Message::DrawTool(Tool::Select));
    send(
        &mut app,
        Message::Draw(Event::Key(crate::draw_mode::Key::Tab, Keys::default())),
    );
    send(
        &mut app,
        Message::Draw(Event::Key(crate::draw_mode::Key::Tab, Keys::default())),
    );
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    for (name, theme) in [
        ("light", cosmic::Theme::light()),
        ("dark", cosmic::Theme::dark()),
    ] {
        let rgb = runtime.screenshot(&theme);
        let size = runtime.window;
        let mut ppm = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
        ppm.extend(rgb);
        let path = std::path::Path::new(&directory).join(format!("window-drawing-{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
    }
}

/// Drawing Properties through the app's messages (`UI_SPEC.md` §33, DW2,
/// DW5): with nothing selected a choice sets the look of new objects; with
/// an object selected it changes it, one undo step each; a colour from the
/// colour panel; a typed width places it, keeping proportions on request;
/// Rotate Left turns it; the Draw menu's commands are on as the selection
/// allows.
#[test]
fn drawing_properties_edit_the_selection_and_the_new_look() {
    use crate::colour_control::{ColourAction, Target};
    use crate::draw_mode::{Event, Keys, Tool};
    use crate::drawing_properties::{Field, PropertyAction};
    use amipro_pictures::sdw::{Colour, Fill};
    let mut app = page_app(Document::from_plain_text("One."));
    send(&mut app, Message::Command(CommandId::Drawing));
    let frame = app.draw.as_ref().unwrap().frame;
    assert!(app.rail_open(), "Drawing Properties opens in the rail");
    send(&mut app, Message::DrawProperties(PropertyAction::Width(60)));
    send(
        &mut app,
        Message::DrawProperties(PropertyAction::Pattern(Fill::SOLID)),
    );
    assert_eq!(app.draw.as_ref().unwrap().pen.width, 60);
    let keys = Keys::default();
    send(&mut app, Message::DrawTool(Tool::Rectangle));
    send(
        &mut app,
        Message::Draw(Event::Press {
            at: (100.0, 100.0),
            keys,
            double: false,
        }),
    );
    send(
        &mut app,
        Message::Draw(Event::Release {
            at: (1_540.0, 820.0),
            keys,
        }),
    );
    let objects = |app: &AmiProApp| {
        amipro_editing::drawing::drawing_of(&app.document, frame)
            .unwrap()
            .objects
    };
    let object = objects(&app)[0].clone();
    assert_eq!(object.pen().unwrap().width, 60);
    assert_eq!(object.fill().unwrap().pattern, Fill::SOLID);
    assert!(!app.command_enabled(CommandId::DrawGroup));
    assert!(app.command_enabled(CommandId::DrawRotateLeft));
    let steps = app.history.undo_len();
    // A colour through the colour button and its panel.
    send(
        &mut app,
        Message::Colour(ColourAction::Toggle(Target::DrawFill, 0)),
    );
    send(&mut app, Message::Colour(ColourAction::Choose(0x0000FF)));
    assert_eq!(
        objects(&app)[0].fill().unwrap().foreground,
        Colour::rgb(255, 0, 0)
    );
    assert_eq!(app.history.undo_len(), steps + 1);
    // A typed width, keeping proportions.
    send(
        &mut app,
        Message::DrawProperties(PropertyAction::KeepRatio(true)),
    );
    send(
        &mut app,
        Message::DrawProperties(PropertyAction::Text(Field::Width, "2".into())),
    );
    send(
        &mut app,
        Message::DrawProperties(PropertyAction::Submit(Field::Width)),
    );
    let rect = objects(&app)[0].rect;
    assert_eq!((rect.width(), rect.height()), (2_880, 1_440));
    assert_eq!(app.history.undo_len(), steps + 2);
    // Rotate Left: a quarter turn.
    send(&mut app, Message::Command(CommandId::DrawRotateLeft));
    let turned = objects(&app)[0].rect;
    assert_eq!((turned.width(), turned.height()), (1_440, 2_880));
    // Grid choices are the mode's, not the document's.
    send(
        &mut app,
        Message::DrawProperties(PropertyAction::ShowGrid(true)),
    );
    assert!(app.draw.as_ref().unwrap().grid.show);
    assert_eq!(app.history.undo_len(), steps + 3);
    send(&mut app, Message::Command(CommandId::DrawDone));
    assert!(app.draw.is_none());
}

/// Draw mode's clipboard and files (`UI_SPEC.md` §33, DW6): Copy and
/// Paste of objects (a copy a little down and right, selected), Cut, SVG
/// and text from the clipboard, Import Drawing… of an SVG file, and Save
/// Drawing As… as SVG and as an Ami Pro drawing (the file messages sent
/// directly: no dialog opens).
#[test]
fn drawings_copy_paste_import_and_save() {
    use crate::draw_mode::{Event, Keys, Tool};
    use amipro_pictures::sdw::Shape;
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("One.")));
    runtime.send(Message::Command(CommandId::Drawing));
    let frame = runtime.app.draw.as_ref().unwrap().frame;
    let objects = |runtime: &Runtime| {
        amipro_editing::drawing::drawing_of(&runtime.app.document, frame)
            .unwrap()
            .objects
    };
    let keys = Keys::default();
    runtime.send(Message::DrawTool(Tool::Ellipse));
    runtime.send(Message::Draw(Event::Press {
        at: (100.0, 100.0),
        keys,
        double: false,
    }));
    runtime.send(Message::Draw(Event::Release {
        at: (900.0, 600.0),
        keys,
    }));
    assert_eq!(objects(&runtime).len(), 1);
    runtime.send(Message::Command(CommandId::Copy));
    runtime.send(Message::Command(CommandId::Paste));
    let pasted = objects(&runtime);
    assert_eq!(pasted.len(), 2);
    assert_eq!(pasted[1].rect.left, pasted[0].rect.left + 144);
    assert_eq!(runtime.app.draw.as_ref().unwrap().selection, [1]);
    runtime.send(Message::Command(CommandId::Cut));
    assert_eq!(objects(&runtime).len(), 1);
    // SVG, then text, from the clipboard.
    runtime.clipboard_data = Some((
        b"<svg xmlns='http://www.w3.org/2000/svg' width='20' height='10'><rect width='20' height='10' fill='#00ff00'/></svg>".to_vec(),
        "image/svg+xml".into(),
    ));
    runtime.send(Message::Command(CommandId::Paste));
    let after = objects(&runtime);
    assert_eq!(after.len(), 2);
    assert!(matches!(after[1].shape, Shape::Polygon { .. }));
    assert_eq!(after[1].rect.width(), 300);
    runtime.clipboard_data = Some((b"Hello\nWorld".to_vec(), "text/plain;charset=utf-8".into()));
    runtime.send(Message::Command(CommandId::Paste));
    let texts = objects(&runtime)
        .iter()
        .filter(|object| matches!(object.shape, Shape::Text(_)))
        .count();
    assert_eq!(texts, 2);
    // Import Drawing… of an SVG file, Save Drawing As… as SVG and SDW.
    let folder = std::env::temp_dir().join(format!("amipro-draw-files-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("shape.svg");
    std::fs::write(
        &file,
        "<svg xmlns='http://www.w3.org/2000/svg' width='40' height='40'><circle cx='20' cy='20' r='18' stroke='black' fill='none'/></svg>",
    )
    .unwrap();
    let session_id = runtime.app.id;
    let before = objects(&runtime).len();
    runtime.send(Message::DrawImportChosen {
        session_id,
        result: FileDialogResult::Selected(file),
    });
    assert_eq!(objects(&runtime).len(), before + 1);
    // Nothing selected: the whole drawing is saved.
    if let Some(mode) = runtime.app.draw.as_mut() {
        mode.selection.clear();
    }
    for name in ["out.svg", "out.sdw"] {
        runtime.send(Message::DrawSaveChosen {
            session_id,
            result: FileDialogResult::Selected(folder.join(name)),
        });
    }
    let svg = std::fs::read_to_string(folder.join("out.svg")).unwrap();
    assert!(svg.starts_with("<?xml") && svg.contains("<svg"), "{svg}");
    let sdw = std::fs::read(folder.join("out.sdw")).unwrap();
    assert_eq!(
        amipro_pictures::sdw::read(&sdw).unwrap().objects,
        objects(&runtime)
    );
    std::fs::remove_dir_all(&folder).unwrap();
}

/// Twins for the VM (`docs/DRAWING_RUNBOOK.md`, DR.12), written by Save As
/// SAM to `AMIPRO_TWIN_OUT`: ADR1 a drawing made with every tool and look
/// (a hatched rectangle, a solid ellipse, a line with endings, a polygon,
/// a polyline, an arc, a rounded rectangle, text turned 30°, a group);
/// ADR2 Ami Pro's DR2B (`AMIPRO_VM_LAB`) edited (the ellipse moved and
/// filled, text added); ADR3 an SVG's shapes and a picture imported.
#[test]
#[ignore = "writes the drawing run's twins for the VM"]
fn drawing_twins() {
    use crate::draw_mode::{Event, Key, Keys, Tool};
    use amipro_editing::drawing::StyleChange;
    use amipro_pictures::sdw::Colour;
    let Some(out) = std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from) else {
        eprintln!("AMIPRO_TWIN_OUT is not set; skipping");
        return;
    };
    let keys = Keys::default();
    let drag = |app: &mut AmiProApp, tool: Tool, a: (f64, f64), b: (f64, f64)| {
        send(app, Message::DrawTool(tool));
        send(
            app,
            Message::Draw(Event::Press {
                at: a,
                keys,
                double: false,
            }),
        );
        send(app, Message::Draw(Event::Drag { at: b, keys }));
        send(app, Message::Draw(Event::Release { at: b, keys }));
    };
    let click = |app: &mut AmiProApp, at: (f64, f64), double: bool| {
        send(app, Message::Draw(Event::Press { at, keys, double }));
        send(app, Message::Draw(Event::Release { at, keys }));
    };
    let style = |app: &mut AmiProApp, change: StyleChange| send(app, Message::DrawStyle(change));
    // ADR1. Each look is set with nothing selected (the look of new
    // objects); a polygon and a polyline take a click a point and a
    // double-click at the last.
    let deselect = |app: &mut AmiProApp| {
        if let Some(mode) = app.draw.as_mut() {
            mode.selection.clear();
        }
    };
    let mut app = page_app(Document::from_plain_text(
        "Drawing twin one.\nAfter the drawing.",
    ));
    send(&mut app, Message::Command(CommandId::Drawing));
    style(
        &mut app,
        StyleChange {
            pattern: Some(9),
            foreground: Some(Colour::rgb(255, 0, 0)),
            background: Some(Colour::rgb(255, 255, 0)),
            ..StyleChange::default()
        },
    );
    drag(&mut app, Tool::Rectangle, (100.0, 100.0), (1_300.0, 800.0));
    deselect(&mut app);
    style(
        &mut app,
        StyleChange {
            pattern: Some(5),
            foreground: Some(Colour::rgb(0, 0, 255)),
            ..StyleChange::default()
        },
    );
    drag(&mut app, Tool::Ellipse, (1_500.0, 100.0), (2_700.0, 900.0));
    deselect(&mut app);
    style(
        &mut app,
        StyleChange {
            width: Some(60),
            start: Some(1),
            end: Some(9),
            ..StyleChange::default()
        },
    );
    drag(&mut app, Tool::Line, (100.0, 1_100.0), (2_700.0, 1_100.0));
    deselect(&mut app);
    style(
        &mut app,
        StyleChange {
            width: Some(20),
            start: Some(0),
            end: Some(0),
            pattern: Some(3),
            foreground: Some(Colour::rgb(0, 128, 0)),
            background: Some(Colour::rgb(255, 255, 255)),
            ..StyleChange::default()
        },
    );
    send(&mut app, Message::DrawTool(Tool::Polygon));
    for at in [(100.0, 1_400.0), (900.0, 1_300.0), (700.0, 2_000.0)] {
        click(&mut app, at, false);
    }
    click(&mut app, (700.0, 2_000.0), true);
    deselect(&mut app);
    send(&mut app, Message::DrawTool(Tool::Polyline));
    for at in [(1_100.0, 1_400.0), (1_500.0, 2_000.0), (1_900.0, 1_400.0)] {
        click(&mut app, at, false);
    }
    click(&mut app, (1_900.0, 1_400.0), true);
    drag(&mut app, Tool::Arc, (2_100.0, 1_300.0), (2_700.0, 2_000.0));
    deselect(&mut app);
    style(
        &mut app,
        StyleChange {
            pattern: Some(0),
            ..StyleChange::default()
        },
    );
    drag(
        &mut app,
        Tool::RoundedRectangle,
        (100.0, 2_200.0),
        (1_200.0, 2_750.0),
    );
    send(&mut app, Message::DrawTool(Tool::Text));
    click(&mut app, (1_400.0, 2_300.0), false);
    send(&mut app, Message::Draw(Event::Text("Twin".into())));
    send(&mut app, Message::DrawTool(Tool::Select));
    let count =
        amipro_editing::drawing::drawing_of(&app.document, app.draw.as_ref().unwrap().frame)
            .unwrap()
            .objects
            .len();
    assert_eq!(count, 8);
    // The text turned 30° clockwise.
    if let Some(mode) = app.draw.as_mut() {
        mode.selection = vec![count - 1];
        mode.rotate_step = 300;
    }
    send(&mut app, Message::Command(CommandId::DrawRotate));
    // The rectangle and the ellipse grouped.
    if let Some(mode) = app.draw.as_mut() {
        mode.selection = vec![0, 1];
    }
    send(&mut app, Message::Command(CommandId::DrawGroup));
    send(&mut app, Message::Command(CommandId::DrawDone));
    std::fs::create_dir_all(&out).unwrap();
    let _ = std::fs::remove_file(out.join("ADR1.SAM"));
    assert!(app.write_sam(&out.join("ADR1.SAM"), 0, false));
    // ADR2.
    if let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") {
        let lab = std::path::PathBuf::from(lab);
        let mut app = page_app(
            amipro_formats::import_sam_path(lab.join("DR2B.SAM"))
                .unwrap()
                .document,
        );
        let frame = app.document.frame_stack()[0];
        frame_action(
            &mut app,
            crate::frame_editing::FrameAction::Select {
                frame,
                page: 1,
                extend: false,
            },
        );
        frame_action(
            &mut app,
            crate::frame_editing::FrameAction::EnterText { at: None },
        );
        assert!(app.draw.is_some());
        if let Some(mode) = app.draw.as_mut() {
            mode.selection = vec![1];
        }
        send(
            &mut app,
            Message::Draw(Event::Key(
                Key::Down,
                Keys {
                    shift: true,
                    ..keys
                },
            )),
        );
        style(
            &mut app,
            StyleChange {
                pattern: Some(5),
                foreground: Some(Colour::rgb(255, 0, 0)),
                ..StyleChange::default()
            },
        );
        send(&mut app, Message::DrawTool(Tool::Text));
        click(&mut app, (300.0, 2_500.0), false);
        send(
            &mut app,
            Message::Draw(Event::Text("Edited in the app".into())),
        );
        send(&mut app, Message::Command(CommandId::DrawDone));
        let _ = std::fs::remove_file(out.join("ADR2.SAM"));
        assert!(app.write_sam(&out.join("ADR2.SAM"), 0, false));
    }
    // ADR3.
    let mut app = page_app(Document::from_plain_text("Imported shapes."));
    send(&mut app, Message::Command(CommandId::Drawing));
    let folder = std::env::temp_dir().join(format!("amipro-twins-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let svg = folder.join("shapes.svg");
    std::fs::write(
        &svg,
        "<svg xmlns='http://www.w3.org/2000/svg' width='120' height='80'>\
         <rect x='5' y='5' width='50' height='30' fill='#ffa500' stroke='#000' stroke-width='2'/>\
         <circle cx='90' cy='50' r='25' fill='#4080ff'/></svg>",
    )
    .unwrap();
    let png = folder.join("dots.png");
    let mut image = image::RgbaImage::new(40, 20);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = image::Rgba(if (x / 5 + y / 5) % 2 == 0 {
            [200, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        });
    }
    image.save(&png).unwrap();
    let session_id = app.id;
    for file in [svg, png] {
        send(
            &mut app,
            Message::DrawImportChosen {
                session_id,
                result: FileDialogResult::Selected(file),
            },
        );
        send(
            &mut app,
            Message::Draw(Event::Key(
                Key::Right,
                Keys {
                    shift: true,
                    ..keys
                },
            )),
        );
    }
    send(&mut app, Message::Command(CommandId::DrawDone));
    let _ = std::fs::remove_file(out.join("ADR3.SAM"));
    assert!(app.write_sam(&out.join("ADR3.SAM"), 0, false));
    std::fs::remove_dir_all(&folder).unwrap();
}

/// Release timings for drawings (`docs/DRAWING_RUNBOOK.md`, DR.11): every
/// drawing and chart in `AMIPRO_PERF_SAM` (if given) read, painted, and
/// drawn at 100 %; and on a drawing of 1,000 polygons and 100 text objects,
/// a move in Draw mode (press, drag, release: one step, the pages laid out
/// again), a property change, and Undo, each end to end through the app's
/// update. Medians and maxima of 20.
#[test]
#[ignore = "release timing; AMIPRO_PERF_SAM optional"]
fn drawing_performance_baseline() {
    use crate::draw_mode::{Event, Keys};
    use amipro_pictures::sdw::{self, Colour, Fill, Object, Pen, Point, Rect};
    use std::time::Instant;
    let report = |name: &str, mut times: Vec<f64>| {
        times.sort_by(f64::total_cmp);
        println!(
            "{name}: median {:.2} ms, max {:.2} ms ({} samples)",
            times[times.len() / 2],
            times[times.len() - 1],
            times.len()
        );
    };
    if let Some(path) = std::env::var_os("AMIPRO_PERF_SAM") {
        let document = amipro_formats::import_sam_path(&path).unwrap().document;
        let mut times = Vec::new();
        for frame in document.frames() {
            let FrameContent::Picture(picture) = &frame.content else {
                continue;
            };
            let amipro_document::PictureSource::Embedded(object) = &picture.source else {
                continue;
            };
            let Some(bytes) = document
                .objects()
                .get(object)
                .map(|object| object.bytes.clone())
            else {
                continue;
            };
            if sdw::read(&bytes).is_none() {
                continue;
            }
            let started = Instant::now();
            let drawing = sdw::read(&bytes).unwrap();
            let size = drawing.natural_size();
            let painted = sdw::paint(&drawing);
            let _ = crate::metafile_render::render(
                &painted,
                (size.width as f32 / 15.0).ceil() as u32,
                (size.height as f32 / 15.0).ceil() as u32,
                1.0,
            );
            times.push(started.elapsed().as_secs_f64() * 1_000.0);
        }
        if !times.is_empty() {
            report("a corpus drawing read, painted, and drawn at 100 %", times);
        }
    }
    let mut objects = Vec::new();
    for index in 0..1_000i16 {
        let (x, y) = ((index % 40) * 70, (index / 40) * 110);
        objects.push(Object::polygon(
            vec![
                Point::new(x, y),
                Point::new(x + 60, y),
                Point::new(x + 30, y + 90),
            ],
            Pen::default(),
            Fill::solid(Colour::rgb(200, 30, 30)),
        ));
    }
    for index in 0..100i16 {
        objects.push(Object::text(sdw::Text {
            origin: Point::new((index % 10) * 280, 2_800 + (index / 10) * 200),
            size: Point::new(250, 180),
            face: "Arial".into(),
            pitch_and_family: 0x22,
            height: -160,
            colour: Colour::BLACK,
            attributes: 0,
            attributes2: 0,
            angle: 0,
            text: "Label".into(),
            kept: sdw::TextKept::default(),
        }));
    }
    let drawing = sdw::Drawing {
        objects,
        ..sdw::Drawing::new()
    };
    let mut document = Document::from_plain_text("A large drawing.");
    let main = document.main_flow_id();
    document = amipro_editing::apply(
        &document,
        &amipro_editing::Command::CreateFrame {
            placement: amipro_document::Placement::Inline {
                y_offset: 0,
                size: amipro_document::Size {
                    width: 2_900,
                    height: 4_900,
                },
            },
            style: amipro_document::FrameStyle::default(),
            content: amipro_editing::NewFrameContent::EmbeddedPicture {
                bytes: sdw::write(&drawing).into(),
                extension: ".sdw".into(),
            },
            anchor: Some(TextPosition {
                flow: main,
                paragraph: 0,
                byte: 0,
            }),
            layout: None,
        },
    )
    .unwrap()
    .document;
    let frame = document.frame_stack()[0];
    let mut app = page_app(document);
    app.enter_draw_mode(frame);
    let keys = Keys::default();
    let (mut moves, mut styles, mut undos) = (Vec::new(), Vec::new(), Vec::new());
    for sample in 0..20 {
        let at = (30.0 + f64::from(sample % 2) * 10.0, 40.0);
        if let Some(mode) = app.draw.as_mut() {
            mode.selection = vec![0];
        }
        let started = Instant::now();
        send(
            &mut app,
            Message::Draw(Event::Press {
                at,
                keys,
                double: false,
            }),
        );
        send(
            &mut app,
            Message::Draw(Event::Drag {
                at: (at.0 + 200.0, at.1),
                keys,
            }),
        );
        send(
            &mut app,
            Message::Draw(Event::Release {
                at: (at.0 + 200.0, at.1),
                keys,
            }),
        );
        moves.push(started.elapsed().as_secs_f64() * 1_000.0);
        let started = Instant::now();
        send(
            &mut app,
            Message::DrawStyle(amipro_editing::drawing::StyleChange {
                width: Some(if sample % 2 == 0 { 40 } else { 20 }),
                ..Default::default()
            }),
        );
        styles.push(started.elapsed().as_secs_f64() * 1_000.0);
        let started = Instant::now();
        send(&mut app, Message::Command(CommandId::Undo));
        undos.push(started.elapsed().as_secs_f64() * 1_000.0);
    }
    report("a move in Draw mode (1,100 objects), end to end", moves);
    report("a line width on the selection", styles);
    report("Undo", undos);
    let _ = Rect::default();
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: a new drawing with the grid turned on from
/// the draw bar, as the user turns it on.
#[test]
#[ignore = "needs AMIPRO_SCREENSHOT_DIR"]
fn drawing_grid_screenshot() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        return;
    };
    let mut app = page_app(Document::from_plain_text("A drawing below."));
    send(&mut app, Message::Command(CommandId::Drawing));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    runtime.send(Message::Command(CommandId::DrawGrid));
    runtime.redraw();
    let rgb = runtime.screenshot(&cosmic::Theme::light());
    let size = runtime.window;
    let mut ppm = format!("P6\n{} {}\n255\n", size.width, size.height).into_bytes();
    ppm.extend(rgb);
    std::fs::write(
        std::path::Path::new(&directory).join("window-grid.ppm"),
        ppm,
    )
    .unwrap();
}

/// A document of `count` one-line paragraphs with a footnote ("Footnote
/// text.") on the first (the notes run).
fn footnote_document(count: usize) -> (Document, FlowId) {
    let text: Vec<String> = (0..count)
        .map(|index| format!("Paragraph {index}."))
        .collect();
    let mut document = Document::from_plain_text(&text.join("\n"));
    let style = document.body_text_style();
    let flow = document.create_flow(style);
    document.flow_mut(flow).unwrap().paragraphs[0] =
        std::sync::Arc::new(document.new_paragraph("Footnote text.", style));
    let paragraph = document.paragraph_mut(0);
    paragraph.text.push('\u{FFFC}');
    paragraph
        .objects
        .push(amipro_document::InlineObject::Footnote { flow });
    (document, flow)
}

#[test]
fn a_click_in_the_footnote_area_edits_the_footnote_in_place() {
    use cosmic::iced::keyboard::{Key, Modifiers};
    use cosmic::iced::mouse::Button;
    let (document, flow) = footnote_document(4);
    let mut app = page_app(document);
    send(&mut app, Message::ZoomTo(50));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let footnote_line = runtime.app.pages.layout().pages[0]
        .lines
        .iter()
        .find(|line| line.container == amipro_layout::Container::Footnote(flow))
        .cloned()
        .expect("the footnote is laid out");
    let point = runtime.on_page(
        footnote_line.x + footnote_line.width + 200.0,
        footnote_line.top + footnote_line.height / 2.0,
    );
    runtime.click(Button::Left, point);
    assert_eq!(runtime.app.page_selection.focus.flow, flow);
    runtime.events(&[key_press(Key::Character("!".into()), Modifiers::default())]);
    assert_eq!(flow_text(&runtime.app, flow), "Footnote text.!");
    send(&mut runtime.app, Message::Command(CommandId::Undo));
    assert_eq!(flow_text(&runtime.app, flow), "Footnote text.");
    // Esc returns to the text after the reference (Ami Pro's Esc).
    use cosmic::iced::keyboard::key::Named;
    runtime.events(&[key_press(Key::Named(Named::Escape), Modifiers::default())]);
    let main = runtime.app.document.main_flow_id();
    let reference = "Paragraph 0.".len();
    assert_eq!(
        (
            runtime.app.page_selection.focus.flow,
            runtime.app.page_selection.focus.byte
        ),
        (main, reference + 3)
    );
    // A double-click on the reference goes to the footnote's text.
    let line = runtime.app.pages.layout().pages[0]
        .lines
        .iter()
        .find(|line| line.container == amipro_layout::Container::Main)
        .cloned()
        .unwrap();
    let word = line
        .words
        .iter()
        .find(|word| word.object.is_some())
        .unwrap();
    let on_reference = runtime.on_page(word.x + word.width / 2.0, line.top + line.height / 2.0);
    runtime.double_click(on_reference);
    assert_eq!(runtime.app.page_selection.focus.flow, flow);
    assert_eq!(runtime.app.page_selection.focus.byte, 0);
}

#[test]
fn notes_are_inserted_typed_and_removed_in_the_notes_panel() {
    use crate::references_panel::{Kind, ReferenceAction};
    let mut app = page_app(Document::from_plain_text("One two three."));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    app.preferences.note_initials = "LS".into();
    send(&mut app, Message::Command(CommandId::InsertNote));
    assert_eq!(
        app.references.as_ref().map(|panel| panel.kind),
        Some(Kind::Notes)
    );
    let notes = amipro_editing::references::notes(&app.document);
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].1.initials.as_deref(), Some("LS"));
    assert_eq!(notes[0].1.number, Some(1));
    // Typing in the note's editor: each key a change, joined into one step.
    let undo_before = app.history.undo_len();
    for character in "Hi".chars() {
        send(
            &mut app,
            Message::References(ReferenceAction::NoteEdit(
                cosmic::iced::widget::text_editor::Action::Edit(
                    cosmic::iced::widget::text_editor::Edit::Insert(character),
                ),
            )),
        );
    }
    let notes = amipro_editing::references::notes(&app.document);
    assert_eq!(
        amipro_editing::references::note_text(&app.document, notes[0].2),
        "Hi"
    );
    assert_eq!(app.history.undo_len(), undo_before + 1);
    // Removed, and back with Undo.
    send(
        &mut app,
        Message::References(ReferenceAction::RemoveNote(notes[0].0)),
    );
    assert!(amipro_editing::references::notes(&app.document).is_empty());
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(amipro_editing::references::notes(&app.document).len(), 1);
    // View › Notes is a check, on by default.
    assert!(app.preferences.show_notes);
    send(&mut app, Message::Command(CommandId::ShowNotes));
    assert!(!app.preferences.show_notes);
}

#[test]
fn bookmarks_are_added_gone_to_renamed_and_removed_in_their_panel() {
    use crate::references_panel::ReferenceAction;
    let mut app = page_app(Document::from_plain_text("Alpha beta gamma.\nSecond."));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 6), 1, false);
    move_to(&mut app, at(main, 0, 11), 1, true);
    send(&mut app, Message::Command(CommandId::Bookmarks));
    send(
        &mut app,
        Message::References(ReferenceAction::Name("two words".into())),
    );
    send(&mut app, Message::References(ReferenceAction::Add));
    assert!(app.references.as_ref().unwrap().problem.is_some());
    send(
        &mut app,
        Message::References(ReferenceAction::Name("bmone".into())),
    );
    send(&mut app, Message::References(ReferenceAction::Add));
    let found = amipro_document::bookmarks::bookmarks(&app.document);
    assert_eq!(found.len(), 1);
    // Go To selects the bookmarked text from anywhere.
    move_to(&mut app, at(main, 1, 0), 1, false);
    send(
        &mut app,
        Message::References(ReferenceAction::GoTo("BMONE".into())),
    );
    assert_eq!(
        (
            app.page_selection.anchor.byte,
            app.page_selection.focus.byte
        ),
        (6, 11)
    );
    send(
        &mut app,
        Message::References(ReferenceAction::Select("bmone".into())),
    );
    send(
        &mut app,
        Message::References(ReferenceAction::Name("second".into())),
    );
    send(&mut app, Message::References(ReferenceAction::Rename));
    assert_eq!(
        amipro_document::bookmarks::bookmarks(&app.document)[0].name,
        "second"
    );
    send(&mut app, Message::References(ReferenceAction::Remove));
    assert!(amipro_document::bookmarks::bookmarks(&app.document).is_empty());
    assert_eq!(app.document.paragraphs()[0].text, "Alpha beta gamma.");
}

#[test]
fn footnotes_are_inserted_and_their_options_set_in_the_footnotes_panel() {
    use crate::references_panel::{Kind, ReferenceAction};
    use amipro_document::FootnoteOptions;
    let mut app = page_app(Document::from_plain_text("Text with a footnote.\nMore."));
    let main = app.document.main_flow_id();
    move_to(
        &mut app,
        at(main, 0, "Text with a footnote.".len()),
        1,
        false,
    );
    send(&mut app, Message::Command(CommandId::Footnote));
    let flow = app.page_selection.focus.flow;
    assert_ne!(flow, main, "the caret is in the footnote");
    page(&mut app, PageAction::Insert("Its text.".into()));
    assert_eq!(flow_text(&app, flow), "Its text.");
    assert!(
        app.pages.layout().pages[0]
            .lines
            .iter()
            .any(|line| line.container == amipro_layout::Container::Footnote(flow))
    );
    send(&mut app, Message::Command(CommandId::FootnoteOptions));
    assert_eq!(
        app.references.as_ref().map(|panel| panel.kind),
        Some(Kind::Footnotes)
    );
    send(
        &mut app,
        Message::References(ReferenceAction::Flag(FootnoteOptions::ENDNOTES, true)),
    );
    assert!(app.document.footnote_options().endnotes());
    // Endnotes: the footnote follows the text on a page of its own.
    assert_eq!(app.pages.layout().pages.len(), 2);
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(!app.document.footnote_options().endnotes());
    send(
        &mut app,
        Message::References(ReferenceAction::Start("3".into())),
    );
    assert_eq!(app.document.footnote_options().start, 3);
}

#[test]
fn a_double_click_on_a_note_mark_opens_the_note_in_the_rail() {
    let mut app = page_app(Document::from_plain_text("One two three."));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    send(&mut app, Message::Command(CommandId::InsertNote));
    send(
        &mut app,
        Message::References(crate::references_panel::ReferenceAction::Close),
    );
    assert!(app.references.is_none());
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let line = runtime.app.pages.layout().pages[0]
        .lines
        .iter()
        .find(|line| line.container == amipro_layout::Container::Main)
        .cloned()
        .unwrap();
    let mark = line
        .words
        .iter()
        .find(|word| word.object.is_some())
        .unwrap();
    let point = runtime.on_page(mark.x, line.top + line.height / 2.0);
    runtime.double_click(point);
    let panel = runtime.app.references.as_ref().expect("the Notes panel");
    assert!(panel.editing.is_some());
    // The panel is drawn in the window.
    runtime.redraw();
    let shot = runtime.screenshot(&cosmic::Theme::light());
    assert!(!shot.is_empty());
}

#[test]
fn go_to_steps_through_bookmarks_notes_and_footnotes() {
    let (document, flow) = footnote_document(3);
    let mut app = page_app(document);
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 2, 0), 1, false);
    send(&mut app, Message::Command(CommandId::GoToPage));
    send(&mut app, Message::GoTo(GoToAction::Item(2)));
    send(&mut app, Message::GoTo(GoToAction::Step(true)));
    // Past the last, Next wraps to the first: the reference in paragraph 0.
    assert_eq!(
        (
            app.page_selection.focus.flow,
            app.page_selection.focus.paragraph
        ),
        (main, 0)
    );
    send(&mut app, Message::Command(CommandId::GoToPage));
    send(&mut app, Message::GoTo(GoToAction::Item(3)));
    send(&mut app, Message::GoTo(GoToAction::Step(true)));
    assert_eq!(app.page_selection.focus.flow, flow);
    assert!(app.go_to.is_none());
}

#[test]
fn cross_references_show_a_bookmarks_page_and_text() {
    use crate::references_panel::ReferenceAction;
    let lines: Vec<String> = (0..90).map(|index| format!("Line {index}.")).collect();
    let mut app = page_app(Document::from_plain_text(&lines.join("\n")));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 80, 0), 1, false);
    move_to(&mut app, at(main, 80, "Line 80.".len()), 1, true);
    send(&mut app, Message::Command(CommandId::Bookmarks));
    send(
        &mut app,
        Message::References(ReferenceAction::Name("target".into())),
    );
    send(&mut app, Message::References(ReferenceAction::Add));
    let page = app.pages.layout().page_of(80, 0).unwrap();
    assert!(page > 1);
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::CrossReference));
    send(
        &mut app,
        Message::References(ReferenceAction::Select("target".into())),
    );
    send(
        &mut app,
        Message::References(ReferenceAction::CrossReference { page: true }),
    );
    assert!(
        app.document.paragraphs()[0]
            .text
            .starts_with(&page.to_string()),
        "{:?}",
        app.document.paragraphs()[0].text
    );
    move_to(&mut app, at(main, 1, 0), 1, false);
    send(
        &mut app,
        Message::References(ReferenceAction::CrossReference { page: false }),
    );
    assert!(app.document.paragraphs()[1].text.starts_with("Line 80."));
}

/// The Notes, Bookmarks, and Footnotes panels with a document that has all
/// three, light and dark (`AMIPRO_SCREENSHOT_DIR`; the notes run).
#[test]
#[ignore]
fn references_screenshots() {
    use crate::references_panel::ReferenceAction;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let (document, _) = footnote_document(30);
    let mut app = page_app(document);
    drop(app.update_title());
    let main = app.document.main_flow_id();
    app.preferences.note_initials = "LS".into();
    move_to(&mut app, at(main, 1, 4), 1, false);
    send(&mut app, Message::Command(CommandId::InsertNote));
    for character in "Check this paragraph.".chars() {
        send(
            &mut app,
            Message::References(ReferenceAction::NoteEdit(
                cosmic::iced::widget::text_editor::Action::Edit(
                    cosmic::iced::widget::text_editor::Edit::Insert(character),
                ),
            )),
        );
    }
    move_to(&mut app, at(main, 2, 0), 1, false);
    move_to(&mut app, at(main, 2, 9), 1, true);
    send(&mut app, Message::Command(CommandId::Bookmarks));
    send(
        &mut app,
        Message::References(ReferenceAction::Name("intro".into())),
    );
    send(&mut app, Message::References(ReferenceAction::Add));
    app.preferences.show_bookmarks = true;
    let write = |app: &AmiProApp, name: &str| {
        for (dark, theme) in [
            (false, cosmic::Theme::light()),
            (true, cosmic::Theme::dark()),
        ] {
            let size = (1_280, 800);
            let rgb = render_window(app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!(
                "window-{name}-{}.ppm",
                if dark { "dark" } else { "light" }
            ));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    };
    send(&mut app, Message::Command(CommandId::Notes));
    write(&app, "notes-panel");
    send(&mut app, Message::Command(CommandId::Bookmarks));
    write(&app, "bookmarks-panel");
    send(&mut app, Message::Command(CommandId::FootnoteOptions));
    write(&app, "footnotes-panel");
}

// The proofing run (`UI_SPEC.md` §35): Spell Check in the Proofing panel.

fn proofing_app(text: &str) -> AmiProApp {
    let mut app = page_app(Document::from_plain_text(text));
    app.spelling = crate::spelling::tests::test_spelling();
    app
}

fn proofing(app: &mut AmiProApp, action: crate::proofing_panel::ProofingAction) {
    send(app, Message::Proofing(action));
}

fn spell_word(app: &AmiProApp) -> Option<String> {
    app.proofing
        .as_ref()?
        .spelling
        .as_ref()?
        .problem
        .as_ref()
        .map(|problem| problem.word.clone())
}

#[test]
fn spell_check_finds_replaces_skips_and_adds_words_one_undo_step_each() {
    use crate::proofing_panel::{ProofingAction as P, SpellState};
    let mut app =
        proofing_app("Teh cat sat on the the mat. We recieve Xyzzy today.\nAnother Teh line.");
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Spelling));
    // The word is selected on the page, with alternatives in its capitals.
    assert_eq!(spell_word(&app).as_deref(), Some("Teh"));
    assert_eq!(
        (
            app.page_selection.anchor.byte,
            app.page_selection.focus.byte
        ),
        (0, 3)
    );
    let check = app.proofing.as_ref().unwrap().spelling.as_ref().unwrap();
    assert_eq!(check.alternatives.first().map(String::as_str), Some("The"));
    // Choosing an alternative twice replaces (Ami Pro's double-click).
    let undo = app.history.undo_len();
    proofing(&mut app, P::Choose(0));
    proofing(&mut app, P::Choose(0));
    assert!(app.document.paragraphs()[0].text.starts_with("The cat"));
    assert_eq!(app.history.undo_len(), undo + 1);
    // The repeat next: its empty Replace with deletes it and the space.
    assert_eq!(spell_word(&app).as_deref(), Some("the"));
    proofing(&mut app, P::Replace);
    assert!(
        app.document.paragraphs()[0]
            .text
            .starts_with("The cat sat on the mat.")
    );
    // Replace All for "recieve": typed, every occurrence, one step.
    assert_eq!(spell_word(&app).as_deref(), Some("recieve"));
    proofing(&mut app, P::ReplaceWith("receive".into()));
    proofing(&mut app, P::ReplaceAll);
    assert!(
        app.document.paragraphs()[0]
            .text
            .contains("We receive Xyzzy")
    );
    // Add to Dictionary: Xyzzy is known from now on (in memory in tests).
    assert_eq!(spell_word(&app).as_deref(), Some("Xyzzy"));
    proofing(&mut app, P::AddToDictionary);
    assert!(app.spelling.user["en_US"].words.contains("Xyzzy"));
    // Skip All: the second paragraph's Teh is passed for the session.
    assert_eq!(spell_word(&app).as_deref(), Some("Teh"));
    proofing(&mut app, P::SkipAll);
    let check = app.proofing.as_ref().unwrap().spelling.as_ref().unwrap();
    assert_eq!(check.state, SpellState::Complete);
    assert_eq!(app.status, "Spell check complete.");
    assert!(app.spell_ignored.contains("Teh"));
    // Undo takes back the last change only.
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(
        app.document.paragraphs()[0]
            .text
            .contains("We recieve Xyzzy")
    );
}

#[test]
fn spell_check_of_a_selection_checks_only_it_and_options_are_ami_pros() {
    use crate::proofing_panel::{ProofingAction as P, SpellOption, SpellState};
    let mut app = proofing_app("teh cat sat. Xyzzy mat.");
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 4), 1, false);
    move_to(&mut app, at(main, 0, 12), 1, true);
    send(&mut app, Message::Command(CommandId::Spelling));
    // PSP2: only the selection; nothing wrong there.
    let check = app.proofing.as_ref().unwrap().spelling.as_ref().unwrap();
    assert_eq!(check.state, SpellState::Complete);
    // Without Check words with initial caps, Xyzzy is left alone.
    proofing(&mut app, P::SpellOption(SpellOption::InitialCaps, false));
    assert_eq!(app.preferences.spell_options, 16);
    move_to(&mut app, at(main, 0, 0), 1, false);
    proofing(&mut app, P::Start);
    assert_eq!(spell_word(&app).as_deref(), Some("teh"));
    proofing(&mut app, P::Skip);
    let check = app.proofing.as_ref().unwrap().spelling.as_ref().unwrap();
    assert_eq!(check.state, SpellState::Complete);
}

#[test]
fn the_document_language_is_set_in_one_step_and_a_missing_dictionary_is_named() {
    use crate::proofing_panel::{ProofingAction as P, SpellState};
    let mut app = proofing_app("Teh cat.");
    send(&mut app, Message::Command(CommandId::Language));
    let panel = app.proofing.as_ref().unwrap();
    assert!(panel.show_options);
    // Only en_US is installed here; British has no dictionary of its own
    // and falls back to the language's (en_US) for checking.
    let undo = app.history.undo_len();
    let _ = app.set_document_language(amipro_document::Language::ami_pro(3));
    assert_eq!(app.document.language().number, 3);
    assert_eq!(app.history.undo_len(), undo + 1);
    proofing(&mut app, P::Start);
    let check = app.proofing.as_ref().unwrap().spelling.as_ref().unwrap();
    assert_eq!(
        check.state,
        SpellState::NoDictionary("hunspell-fr-fr".into())
    );
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document.language().number, 1);
}

#[test]
fn f7_opens_spell_check_and_the_panels_buttons_take_clicks() {
    use crate::proofing_panel::ProofingAction as P;
    use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
    let mut runtime = Runtime::new(proofing_app("Teh cat sat."));
    runtime.redraw();
    // F7 and Shift+F7 are Ami Pro-era Spell Check and Thesaurus; the
    // subscription turns the key into the command.
    assert_eq!(
        crate::command::command_for_shortcut(Modifiers::empty(), &Key::Named(Named::F7), None),
        Some(CommandId::Spelling)
    );
    assert_eq!(
        crate::command::command_for_shortcut(Modifiers::SHIFT, &Key::Named(Named::F7), None),
        Some(CommandId::Thesaurus)
    );
    runtime.send(Message::Command(CommandId::Spelling));
    assert_eq!(
        spell_word(&runtime.app).as_deref(),
        Some("Teh"),
        "{} {:?}",
        runtime.app.status,
        runtime
            .app
            .proofing
            .as_ref()
            .map(|panel| panel.spelling.clone())
    );
    runtime.redraw();
    // Down the rail, a click on each row: Skip is among them.
    let mut found = Vec::new();
    let mut y = 120.0;
    while y < Runtime::WINDOW.height - 40.0 {
        for x in [80.0, 220.0] {
            for message in runtime.click_unrun(
                cosmic::iced::mouse::Button::Left,
                cosmic::iced::Point::new(x, y),
            ) {
                if let Message::Proofing(action) = message {
                    found.push(format!("{action:?}"));
                }
            }
        }
        y += 6.0;
    }
    assert!(found.iter().any(|action| action == "Skip"), "{found:?}");
    assert!(
        found.iter().any(|action| action == "Choose(0)"),
        "{found:?}"
    );
    runtime.send(Message::Proofing(P::Skip));
    assert_eq!(runtime.app.status, "Spell check complete.");
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Proofing panel at a misspelled word,
/// with its options and user dictionary open, in both themes.
#[test]
#[ignore = "writes screenshots"]
fn proofing_screenshots() {
    use crate::proofing_panel::ProofingAction as P;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = proofing_app(
        "We recieve teh letters today.\nThe cat sat on the the mat and the line was long.",
    );
    drop(app.update_title());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Spelling));
    proofing(&mut app, P::ToggleOptions);
    proofing(&mut app, P::UserWord("Xyzzy".into()));
    proofing(&mut app, P::AddUserWord);
    proofing(&mut app, P::ToggleDictionary);
    let shots = |app: &AmiProApp, prefix: &str| {
        for (dark, name) in [(false, "light"), (true, "dark")] {
            let theme = if dark {
                cosmic::Theme::dark()
            } else {
                cosmic::Theme::light()
            };
            let size = (1_280, 800);
            let rgb = render_window(app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!("window-{prefix}-{name}.ppm"));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    };
    // The Thesaurus page on "big" and the Grammar page on its first
    // problem, with grammar underlines on.
    let mut thesaurus = proofing_app("We receive big letters today.");
    test_thesaurus(&mut thesaurus, "shots");
    drop(thesaurus.update_title());
    let main_flow = thesaurus.document.main_flow_id();
    move_to(&mut thesaurus, at(main_flow, 0, 12), 1, false);
    send(&mut thesaurus, Message::Command(CommandId::Thesaurus));
    shots(&thesaurus, "thesaurus");
    let mut grammar = proofing_app(
        "This is a apple. It was decided by the committee that the plan was in close proximity.",
    );
    grammar.spelling.grammar = Some(test_grammar());
    grammar.preferences.grammar_as_you_type = true;
    drop(grammar.update_title());
    let main_flow = grammar.document.main_flow_id();
    move_to(&mut grammar, at(main_flow, 0, 0), 1, false);
    send(&mut grammar, Message::Command(CommandId::GrammarCheck));
    drop(grammar.check_as_you_type());
    shots(&grammar, "grammar");
    for (dark, name) in [
        (false, "window-proofing-light"),
        (true, "window-proofing-dark"),
    ] {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(&app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    }
}

#[test]
fn a_stale_pause_schedules_no_other_and_the_current_one_underlines() {
    // On the desktop the pauses of earlier keystrokes arrive after later
    // keystrokes; a stale one used to clear the tab's checked root, so the
    // next message scheduled another pause and none was ever current.
    use crate::proofing_panel::ProofingAction as P;
    let mut app = proofing_app("Teh cat.");
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 8), 1, false);
    send(&mut app, Message::Page(PageAction::Insert(" Xyzzy".into())));
    let (session, first) = (app.id, app.spell_generation);
    send(&mut app, Message::Page(PageAction::Insert(" teh".into())));
    let current = app.spell_generation;
    assert!(current > first);
    proofing(
        &mut app,
        P::Idle {
            session,
            generation: first,
        },
    );
    send(&mut app, Message::Repaint);
    assert_eq!(
        app.spell_generation, current,
        "a stale pause scheduled another"
    );
    proofing(
        &mut app,
        P::Idle {
            session,
            generation: current,
        },
    );
    assert_eq!(app.spell_marks.get(&(main, 0)).map(Vec::len), Some(3));
}

#[test]
fn words_are_underlined_as_you_type_after_a_pause_and_not_when_off() {
    use crate::proofing_panel::ProofingAction as P;
    let mut app = proofing_app("Teh cat sat on the the mat.");
    let main = app.document.main_flow_id();
    // A change schedules the check; its pause arrives as Idle.
    move_to(&mut app, at(main, 0, 27), 1, false);
    send(&mut app, Message::Page(PageAction::Insert(" Xyzzy".into())));
    assert!(app.spell_root.is_some());
    let (session, generation) = (app.id, app.spell_generation);
    proofing(
        &mut app,
        P::Idle {
            session,
            generation,
        },
    );
    let marks = app.spell_marks.get(&(main, 0)).cloned().unwrap_or_default();
    let text = app.document.paragraphs()[0].text.clone();
    let marked: Vec<&str> = marks.iter().map(|range| &text[range.clone()]).collect();
    assert_eq!(marked, ["Teh", " the", "Xyzzy"]);
    // An older pause does nothing after a newer change.
    send(&mut app, Message::Page(PageAction::Insert("s".into())));
    proofing(
        &mut app,
        P::Idle {
            session,
            generation,
        },
    );
    assert_eq!(app.spell_marks.get(&(main, 0)).map(Vec::len), Some(3));
    // Ignore All (Skip All) and Add to Dictionary take effect at once.
    app.spell_ignored.add("Teh");
    drop(app.check_as_you_type());
    // The repeat and "Xyzzys" stay.
    assert_eq!(app.spell_marks.get(&(main, 0)).map(Vec::len), Some(2));
    // Off: no underlines.
    send(&mut app, Message::Command(CommandId::SpellAsYouType));
    assert!(!app.preferences.spell_as_you_type);
    assert!(app.spell_marks.is_empty());
}

#[test]
fn the_right_click_menu_offers_an_underlined_words_suggestions() {
    use crate::proofing_panel::{ContextEntry, ProofingAction as P};
    let mut app = proofing_app("We sat on teh mat.");
    let main = app.document.main_flow_id();
    drop(app.check_as_you_type());
    // The right press puts the caret in the word, then the menu opens.
    move_to(&mut app, at(main, 0, 11), 1, false);
    page(&mut app, PageAction::ContextAt { margin: false });
    let labels: Vec<String> = app
        .text_context_menu()
        .into_iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item { label, .. } => Some(label),
            _ => None,
        })
        .collect();
    assert_eq!(labels.first().map(String::as_str), Some("the"));
    assert!(labels.contains(&"Ignore All".to_string()));
    assert!(labels.contains(&"Add to Dictionary".to_string()));
    // Only the spelling entries, so that the menu fits a window that is not
    // maximised (user, 2026-11-03).
    assert!(!labels.contains(&"Cut".to_string()));
    let undo = app.history.undo_len();
    proofing(&mut app, P::Context(ContextEntry::Suggestion(0)));
    assert_eq!(app.document.paragraphs()[0].text, "We sat on the mat.");
    assert_eq!(app.history.undo_len(), undo + 1);
    // Off an underlined word, no spelling entries (synonyms only, when a
    // thesaurus is installed).
    move_to(&mut app, at(main, 0, 1), 1, false);
    page(&mut app, PageAction::ContextAt { margin: false });
    assert!(app.context_word.as_ref().is_none_or(|word| !word.flagged));
    let labels: Vec<String> = app
        .text_context_menu()
        .into_iter()
        .filter_map(|entry| match entry {
            MenuEntry::Item { label, .. } => Some(label),
            _ => None,
        })
        .collect();
    assert!(!labels.contains(&"Ignore All".to_string()));
}

#[test]
fn new_documents_take_the_default_language() {
    use crate::proofing_panel::ProofingAction as P;
    let mut app = proofing_app("Colour.");
    let _ = app.set_document_language(amipro_document::Language::ami_pro(2));
    send(&mut app, Message::Command(CommandId::Language));
    proofing(&mut app, P::DefaultLanguage);
    assert_eq!(app.preferences.default_language, "en_GB");
    send(&mut app, Message::Command(CommandId::NewDocument));
    assert_eq!(app.document.language().number, 2);
    // Back to American: no setting kept.
    let _ = app.set_document_language(amipro_document::Language::ami_pro(1));
    send(&mut app, Message::Command(CommandId::Language));
    proofing(&mut app, P::DefaultLanguage);
    assert_eq!(app.preferences.default_language, "");
}

/// A small MyThes thesaurus written for the tests, loaded as en-US.
fn test_thesaurus(app: &mut AmiProApp, name: &str) {
    let folder = std::env::temp_dir().join(format!("amipro-thes-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let mut data = String::from("UTF-8\n");
    let mut index = String::from("UTF-8\n3\n");
    for (word, entry) in [
        (
            "big",
            "big|2\n(adj)|large|great (similar term)\n(adj)|important\n",
        ),
        ("large", "large|1\n(adj)|big|great\n"),
        ("sit", "sit|1\n(verb)|pose|model\n"),
    ] {
        index.push_str(&format!("{word}|{}\n", data.len()));
        data.push_str(entry);
    }
    std::fs::write(folder.join("th_en_US_v2.idx"), index).unwrap();
    std::fs::write(folder.join("th_en_US_v2.dat"), data).unwrap();
    let thesaurus = crate::spelling::Thesaurus::load(
        "en_US",
        &folder.join("th_en_US_v2.idx"),
        &folder.join("th_en_US_v2.dat"),
    )
    .unwrap();
    app.spelling.thesaurus_folders = vec![folder];
    app.spelling
        .thesauri
        .insert("en_US".into(), std::sync::Arc::new(thesaurus));
}

#[test]
fn the_thesaurus_looks_up_the_caret_word_and_replaces_it_in_one_step() {
    use crate::proofing_panel::{Page, ProofingAction as P};
    let mut app = proofing_app("A Big house where we sat.");
    test_thesaurus(&mut app, "look-up");
    let main = app.document.main_flow_id();
    // The caret just after the word: Ami Pro looks up the word before it.
    move_to(&mut app, at(main, 0, 5), 1, false);
    send(&mut app, Message::Command(CommandId::Thesaurus));
    let panel = app.proofing.as_ref().unwrap();
    assert_eq!(panel.page, Page::Thesaurus);
    let lookup = &panel.thesaurus;
    assert_eq!(lookup.looked_up, "Big");
    assert_eq!(
        (
            app.page_selection.anchor.byte,
            app.page_selection.focus.byte
        ),
        (2, 5)
    );
    // The word's capitals (PSP1), the first meaning's first synonym ready.
    let entry = lookup.entry.as_ref().unwrap();
    assert_eq!(entry.meanings[0].synonyms, ["Large", "Great"]);
    assert_eq!(lookup.replace_with, "Large");
    // Another meaning, a synonym pressed twice: replaced, one step; the
    // panel stays with the new word selected.
    proofing(&mut app, P::Meaning(1));
    assert_eq!(
        app.proofing.as_ref().unwrap().thesaurus.replace_with,
        "Important"
    );
    let undo = app.history.undo_len();
    proofing(&mut app, P::Synonym(0));
    proofing(&mut app, P::Synonym(0));
    assert_eq!(
        app.document.paragraphs()[0].text,
        "A Important house where we sat."
    );
    assert_eq!(app.history.undo_len(), undo + 1);
    assert_eq!(
        app.proofing.as_ref().unwrap().thesaurus.looked_up,
        "Important"
    );
    // Look Up and Previous; an unknown word says so.
    proofing(&mut app, P::ThesaurusReplaceWith("large".into()));
    proofing(&mut app, P::LookUp);
    assert_eq!(app.proofing.as_ref().unwrap().thesaurus.looked_up, "large");
    proofing(&mut app, P::ThesaurusReplaceWith("big".into()));
    proofing(&mut app, P::LookUp);
    proofing(&mut app, P::ThesaurusReplaceWith("xyzzy".into()));
    proofing(&mut app, P::LookUp);
    assert_eq!(
        app.proofing.as_ref().unwrap().thesaurus.message.as_deref(),
        Some("xyzzy is not in the thesaurus.")
    );
    proofing(&mut app, P::Previous);
    assert_eq!(app.proofing.as_ref().unwrap().thesaurus.looked_up, "large");
    // An inflected word: "sat" finds sit's verbs, in the past.
    move_to(&mut app, at(main, 0, 30), 1, false);
    proofing(&mut app, P::LookUpCaret);
    let lookup = &app.proofing.as_ref().unwrap().thesaurus;
    assert_eq!(lookup.looked_up, "sat");
    assert_eq!(
        lookup.entry.as_ref().unwrap().meanings[0].synonyms,
        ["posed", "modeled"]
    );
}

#[test]
fn the_right_click_menu_offers_synonyms_of_any_word() {
    use crate::proofing_panel::{ContextEntry, ProofingAction as P};
    let mut app = proofing_app("A big house.");
    test_thesaurus(&mut app, "menu");
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 3), 1, false);
    page(&mut app, PageAction::ContextAt { margin: false });
    let synonyms = app
        .text_context_menu()
        .into_iter()
        .find_map(|entry| match entry {
            MenuEntry::Folder { label, entries, .. } if label == "Synonyms" => Some(entries.len()),
            _ => None,
        });
    assert_eq!(synonyms, Some(2));
    proofing(&mut app, P::Context(ContextEntry::Synonym(1)));
    assert_eq!(app.document.paragraphs()[0].text, "A great house.");
}

/// Harper's checker, made once for the tests (it takes seconds unoptimized).
fn test_grammar() -> std::sync::Arc<crate::grammar::Grammar> {
    static GRAMMAR: std::sync::OnceLock<std::sync::Arc<crate::grammar::Grammar>> =
        std::sync::OnceLock::new();
    GRAMMAR.get_or_init(crate::grammar::load).clone()
}

#[test]
fn grammar_check_finds_replaces_and_ends_with_ami_pros_statistics() {
    use crate::proofing_panel::{GrammarState, ProofingAction as P};
    let mut app = proofing_app("This is a apple. The cat sat on the mat.");
    app.spelling.grammar = Some(test_grammar());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    assert_eq!(app.document.grammar_set(), "");
    send(&mut app, Message::Command(CommandId::GrammarCheck));
    // Ami Pro records the set the check uses (PGR1).
    assert_eq!(app.document.grammar_set(), "Business Writing");
    let check = app.proofing.as_ref().unwrap().grammar.as_ref().unwrap();
    assert_eq!(check.state, GrammarState::Checking);
    let found = check.found.as_ref().unwrap();
    assert!(
        found.problem.replacements.contains(&"an".to_string()),
        "{found:?}"
    );
    let chosen = found
        .problem
        .replacements
        .iter()
        .position(|replacement| replacement == "an")
        .unwrap();
    let undo = app.history.undo_len();
    proofing(&mut app, P::ChooseReplacement(chosen));
    proofing(&mut app, P::GrammarReplace);
    assert!(
        app.document.paragraphs()[0]
            .text
            .starts_with("This is an apple.")
    );
    assert_eq!(app.history.undo_len(), undo + 1);
    // Skip what remains to the end: the statistics come (Ami Pro's option,
    // on by default).
    for _ in 0..10 {
        let done = app
            .proofing
            .as_ref()
            .unwrap()
            .grammar
            .as_ref()
            .is_some_and(|check| check.state == GrammarState::Complete);
        if done {
            break;
        }
        proofing(&mut app, P::GrammarSkip);
    }
    let check = app.proofing.as_ref().unwrap().grammar.as_ref().unwrap();
    assert_eq!(check.state, GrammarState::Complete);
    let figures = check.statistics.unwrap();
    assert_eq!(
        (figures.words, figures.sentences, figures.paragraphs),
        (10, 2, 1)
    );
    // Tools › Readability Statistics… alone: the document's figures at a
    // caret (a selection's own figures otherwise).
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Command(CommandId::ReadabilityStatistics));
    assert_eq!(app.proofing.as_ref().unwrap().statistics.unwrap().words, 10);
}

#[test]
fn grammar_is_english_only_and_underlines_when_asked() {
    use crate::proofing_panel::{GrammarState, ProofingAction as P};
    let mut app = proofing_app("This is a apple.");
    app.spelling.grammar = Some(test_grammar());
    let main = app.document.main_flow_id();
    // Off by default; on, a blue underline under the problem.
    assert!(!app.preferences.grammar_as_you_type);
    send(&mut app, Message::Command(CommandId::GrammarCheck));
    proofing(&mut app, P::GrammarAsYouType(true));
    assert!(app.grammar_marks.contains_key(&(main, 0)));
    proofing(&mut app, P::GrammarAsYouType(false));
    assert!(app.grammar_marks.is_empty());
    // A French document: no grammar.
    let _ = app.set_document_language(amipro_document::Language::ami_pro(3));
    proofing(&mut app, P::StartGrammar);
    let check = app.proofing.as_ref().unwrap().grammar.as_ref().unwrap();
    assert_eq!(check.state, GrammarState::NotEnglish);
}

/// Release timings of proofing (`AMIPRO_PERF_SAM=FILE`; the bundled en-US
/// dictionary, the system's en-US thesaurus when installed): loading, a
/// whole Spell Check, the as-you-type check cold and after a keystroke, a
/// keystroke with the underlines shown, Harper made and a grammar check of
/// the whole document, a thesaurus lookup. With `AMIPRO_CORPUS=DIR`, every
/// SAM's text spell-checked and grammar-checked without a panic (counts
/// only).
#[test]
#[ignore = "release benchmark; AMIPRO_PERF_SAM, AMIPRO_CORPUS"]
fn proofing_performance_baseline() {
    use crate::proofing_panel::ProofingAction as P;
    use amipro_editing::proofing::spelling as sp;
    let median = |times: &mut Vec<std::time::Duration>| {
        times.sort();
        times[times.len() / 2].as_secs_f64() * 1_000.0
    };
    let started = Instant::now();
    let available = crate::spelling::available();
    let dictionary = crate::spelling::dictionary_for(&available, "en_US")
        .unwrap()
        .clone();
    let (_, loaded) = crate::spelling::load(dictionary.clone());
    let loaded = loaded.unwrap();
    eprintln!(
        "dictionary {:?}: loaded in {:.1} ms",
        dictionary.source,
        started.elapsed().as_secs_f64() * 1_000.0
    );
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    app.spelling = crate::spelling::Spelling::new(vec![dictionary.clone()], None);
    app.spelling.loaded("en_US".into(), Ok(loaded.clone()));
    let main = app.document.main_flow_id();
    // A whole Spell Check, every word to the end (Skip at each).
    move_to(&mut app, at(main, 0, 0), 1, false);
    let started = Instant::now();
    send(&mut app, Message::Command(CommandId::Spelling));
    let mut found = 0;
    let mut slowest = (std::time::Duration::ZERO, String::new());
    while app
        .proofing
        .as_ref()
        .and_then(|panel| panel.spelling.as_ref())
        .is_some_and(|check| {
            check.problem.is_some() && check.state == crate::proofing_panel::SpellState::Checking
        })
    {
        found += 1;
        let step = Instant::now();
        proofing(&mut app, P::Skip);
        if step.elapsed() > slowest.0 {
            slowest = (step.elapsed(), spell_word(&app).unwrap_or_default());
        }
        if found % 25 == 0 {
            eprintln!(
                "  {found} words, {:.1} s so far; slowest step {:?} before {:?}",
                started.elapsed().as_secs_f64(),
                slowest.0,
                slowest.1
            );
        }
    }
    eprintln!(
        "spell check: {found} words found in {:.1} ms ({} paragraphs)",
        started.elapsed().as_secs_f64() * 1_000.0,
        app.document.paragraphs().len()
    );
    // As you type: cold, then after a keystroke (one paragraph new).
    let mut cold = Vec::new();
    let mut warm = Vec::new();
    let mut typing = Vec::new();
    let window = cosmic::iced::Size::new(1_280.0, 800.0);
    let mut harness = FrameHarness::new(window);
    for _ in 0..10 {
        let started = Instant::now();
        drop(app.check_as_you_type());
        cold.push(started.elapsed());
        move_to(&mut app, at(main, 0, 0), 1, false);
        harness.frame(&app);
        let started = Instant::now();
        page(&mut app, PageAction::Insert("x".into()));
        harness.frame(&app);
        typing.push(started.elapsed());
        let (session, generation) = (app.id, app.spell_generation);
        let started = Instant::now();
        proofing(
            &mut app,
            P::Idle {
                session,
                generation,
            },
        );
        warm.push(started.elapsed());
        send(&mut app, Message::Command(CommandId::Undo));
    }
    let marks: usize = app.spell_marks.values().map(Vec::len).sum();
    eprintln!(
        "as you type: {marks} underlined; cold {:.2} ms, after a keystroke {:.3} ms; a keystroke with underlines shown {:.3} ms (medians)",
        median(&mut cold),
        median(&mut warm),
        median(&mut typing)
    );
    // Grammar.
    let started = Instant::now();
    let grammar = crate::grammar::load();
    eprintln!(
        "Harper made in {:.1} ms",
        started.elapsed().as_secs_f64() * 1_000.0
    );
    let started = Instant::now();
    let mut problems = 0;
    for paragraph in app.document.paragraphs().iter() {
        problems += grammar
            .check(&paragraph.text, "All Rules", harper_core::Dialect::American)
            .len();
    }
    eprintln!(
        "grammar: {problems} problems in the main text in {:.1} ms",
        started.elapsed().as_secs_f64() * 1_000.0
    );
    // The thesaurus.
    if let Some((idx, dat)) =
        crate::spelling::thesaurus_files(&crate::spelling::thesaurus_folders(), "en_US")
    {
        let started = Instant::now();
        let thesaurus = crate::spelling::Thesaurus::load("en_US", &idx, &dat).unwrap();
        let loaded = started.elapsed().as_secs_f64() * 1_000.0;
        let mut lookups = Vec::new();
        for word in ["sat", "big", "running", "Another", "happiest"] {
            let started = Instant::now();
            let entry = thesaurus.look_up(word);
            lookups.push(started.elapsed());
            let first: Vec<String> = entry
                .map(|entry| {
                    entry
                        .meanings
                        .into_iter()
                        .take(2)
                        .flat_map(|m| m.synonyms.into_iter().take(3))
                        .collect()
                })
                .unwrap_or_default();
            eprintln!("thesaurus: {word} → {first:?}");
        }
        eprintln!(
            "thesaurus: index in {loaded:.1} ms, lookups {:.2} ms (median)",
            median(&mut lookups)
        );
    }
    // The corpus.
    if let Some(folder) = std::env::var_os("AMIPRO_CORPUS") {
        let styles: Vec<std::path::PathBuf> = std::env::var_os("AMIPRO_STYLES")
            .map(|styles| std::env::split_paths(&styles).collect())
            .unwrap_or_default();
        let mut files = Vec::new();
        let mut stack = vec![std::path::PathBuf::from(folder)];
        while let Some(folder) = stack.pop() {
            for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
                {
                    files.push(path);
                }
            }
        }
        let speller = crate::spelling::SessionSpeller {
            dictionary: &loaded,
            user: &crate::spelling::UserDictionary::default(),
            user_alternatives: false,
        };
        let (mut documents, mut paragraphs, mut flagged, mut grammar_found) = (0, 0, 0, 0);
        let started = Instant::now();
        for path in files {
            let Ok(imported) =
                amipro_formats::import_sam_path_with_style_directories(&path, &styles)
            else {
                continue;
            };
            documents += 1;
            let document = imported.document;
            for stream in amipro_editing::find::streams(&document) {
                for paragraph in &document.flow(stream.flow).unwrap().paragraphs {
                    paragraphs += 1;
                    flagged += sp::paragraph_problems(
                        &paragraph.text,
                        &speller,
                        &sp::SpellOptions::default(),
                        &sp::Ignored::default(),
                    )
                    .len();
                    grammar_found += grammar
                        .check(
                            &paragraph.text,
                            "Business Writing",
                            harper_core::Dialect::American,
                        )
                        .len();
                }
            }
        }
        eprintln!(
            "corpus: {documents} documents, {paragraphs} paragraphs, {flagged} words flagged, {grammar_found} grammar problems, in {:.1} s",
            started.elapsed().as_secs_f64()
        );
    }
}

/// The proofing twin (`AMIPRO_VM_LAB=…/vm-lab/OUT`, `AMIPRO_TWIN_OUT=DIR`):
/// PSP4B, whose first paragraph Ami Pro flagged checked (`<:s>`), with
/// "Teh " typed into it, a misspelling replaced by Spell Check, the
/// language British, and the grammar set All Rules, written by Save As SAM
/// as APF1.SAM. In Ami Pro, Spell Check stops at "Teh" (the flag is gone),
/// Language Options shows British, and Grammar Check offers All Rules.
#[test]
#[ignore = "writes a twin for Ami Pro"]
fn proofing_twins() {
    use crate::proofing_panel::ProofingAction as P;
    let (Some(lab), Some(out)) = (
        std::env::var_os("AMIPRO_VM_LAB").map(std::path::PathBuf::from),
        std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from),
    ) else {
        eprintln!("AMIPRO_VM_LAB or AMIPRO_TWIN_OUT is not set; skipping");
        return;
    };
    let document = amipro_formats::import_sam_path(lab.join("PSP4B.SAM"))
        .unwrap()
        .document;
    let mut app = proofing_app("");
    app.sessions[0] = DocumentSession::new(1, document, DocumentLocation::Untitled, "Twin".into());
    app.spelling = crate::spelling::tests::test_spelling();
    let main = app.document.main_flow_id();
    // "Teh " typed at the start of the flagged paragraph.
    move_to(&mut app, at(main, 0, 0), 1, false);
    page(&mut app, PageAction::Insert("Teh ".into()));
    // A misspelling in the second paragraph, replaced through Spell Check.
    move_to(&mut app, at(main, 1, 0), 1, false);
    page(&mut app, PageAction::Insert("Recieve ".into()));
    move_to(&mut app, at(main, 1, 0), 1, false);
    send(&mut app, Message::Command(CommandId::Spelling));
    assert_eq!(spell_word(&app).as_deref(), Some("Recieve"));
    proofing(&mut app, P::ReplaceWith("Receive".into()));
    proofing(&mut app, P::Replace);
    let _ = app.set_document_language(amipro_document::Language::ami_pro(2));
    let _ = app.execute_frame_commands(
        &[amipro_editing::Command::SetGrammarSet("All Rules".into())],
        false,
    );
    let _ = std::fs::remove_file(out.join("APF1.SAM"));
    assert!(app.write_sam(&out.join("APF1.SAM"), 0, false));
    let written = std::fs::read(out.join("APF1.SAM")).unwrap();
    let text = String::from_utf8_lossy(&written);
    assert!(
        text.contains("[edoc]\r\nTeh Clean words only."),
        "the flag is gone"
    );
    assert!(text.contains("[lang]\r\n\t2\r\n"));
    assert!(text.contains("[GramStyle]\r\n\tAll Rules\r\n"));
}

#[test]
fn grammar_underlines_are_worked_out_off_the_interfaces_thread() {
    use crate::proofing_panel::ProofingAction as P;
    let mut app = proofing_app("Fine words here.\nThis is a apple.");
    app.spelling.grammar = Some(test_grammar());
    let main = app.document.main_flow_id();
    let mut runtime = Runtime::new(app);
    // Turning them on returns the worker's task; when it is done, the
    // problem's paragraph is underlined.
    runtime.send(Message::Proofing(P::GrammarAsYouType(true)));
    assert!(runtime.app.grammar_marks.contains_key(&(main, 1)));
    assert!(!runtime.app.grammar_marks.contains_key(&(main, 0)));
}

/// A document with Ami Pro's TOC levels (Title, Subhead) and its headings
/// over two pages (`UI_SPEC.md` §36).
fn contents_document() -> Document {
    let edoc = [
        "@Title@Getting Started",
        "The printer must be set up first.",
        "@Subhead@Installing",
        "Install the printer driver.<:p<* >>",
        "@Title@Advanced",
        "Printer fonts are many.",
    ]
    .map(|paragraph| format!("{paragraph}\r\n\r\n"))
    .concat();
    let bytes = format!(
        "[ver]\r\n\t4\r\n[toc]\r\n\tTitle\r\n\tSubhead\r\n\t\r\n\t.\r\n\t.\r\n\t\r\n\t15\r\n[edoc]\r\n{edoc}>\r\n\r\n"
    );
    amipro_formats::import_sam(bytes.as_bytes(), None, "Contents")
        .unwrap()
        .document
}

#[test]
fn a_toc_is_generated_updated_and_undone_from_the_contents_panel() {
    use crate::contents_panel::{ContentsAction, Page};
    let mut app = page_app(contents_document());
    send(&mut app, Message::Command(CommandId::ContentsIndex));
    assert_eq!(
        app.contents.as_ref().map(|panel| panel.page),
        Some(Page::Contents)
    );
    let undo = app.history.undo_len();
    send(&mut app, Message::Contents(ContentsAction::GenerateToc));
    let table = amipro_editing::contents::toc_table(&app.document).expect("a TOC");
    assert_eq!(app.history.undo_len(), undo + 1, "one undo step");
    assert!(
        app.document.contents().toc.is_some(),
        "the levels kept as [toc]"
    );
    let shown: Vec<(u32, u32)> = app
        .pages
        .layout()
        .pages
        .iter()
        .map(|page| (page.number, page.shown))
        .collect();
    assert_eq!(
        shown,
        [(1, 1), (2, 1), (3, 2)],
        "pages after the TOC count from 1"
    );
    let FrameContent::Table(toc) = &app.document.frame(table).unwrap().content else {
        panic!("a table");
    };
    assert_eq!(toc.rows.len(), 3, "Getting Started, Installing, Advanced");
    // Page 2's heading after the TOC's own page counts as page 2 again.
    let cell = |row: usize, column: usize| {
        let amipro_document::CellContent::Text(flow) = toc.cells[row * 2 + column].content;
        app.document.flow(flow).unwrap().paragraphs[0].text.clone()
    };
    assert_eq!(
        (cell(0, 0), cell(0, 1)),
        ("Getting Started".into(), "1".into())
    );
    assert_eq!((cell(2, 0), cell(2, 1)), ("Advanced".into(), "2".into()));
    // Update replaces it in place, one step more.
    send(&mut app, Message::Contents(ContentsAction::GenerateToc));
    assert_eq!(
        app.document
            .frames()
            .filter(|frame| amipro_document::contents::is_toc_table(frame))
            .count(),
        1
    );
    assert_eq!(app.history.undo_len(), undo + 2);
    send(&mut app, Message::Command(CommandId::Undo));
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(amipro_editing::contents::toc_table(&app.document).is_none());
    // A level's leader changes [toc] in one step.
    send(
        &mut app,
        Message::Contents(ContentsAction::LevelRightAlign(1, false)),
    );
    let levels = amipro_editing::contents::toc_levels(&app.document);
    assert!(!levels[1].right_align);
    send(&mut app, Message::Command(CommandId::Undo));
    assert!(amipro_editing::contents::toc_levels(&app.document)[1].right_align);
}

#[test]
fn index_entries_are_marked_and_the_index_opens_in_a_new_tab() {
    use crate::contents_panel::{ContentsAction, IndexChoice, Page};
    let mut app = page_app(contents_document());
    let main = app.document.main_flow_id();
    // "printer" selected in the second paragraph.
    move_to(&mut app, at(main, 1, 4), 1, false);
    move_to(&mut app, at(main, 1, 11), 1, true);
    send(&mut app, Message::Command(CommandId::MarkIndexEntry));
    let panel = app.contents.as_ref().unwrap();
    assert_eq!(
        (panel.page, panel.primary.as_str()),
        (Page::Entries, "printer")
    );
    send(
        &mut app,
        Message::Contents(ContentsAction::Primary("Printer".into())),
    );
    send(&mut app, Message::Contents(ContentsAction::Mark));
    let marks = |app: &AmiProApp| {
        app.document
            .paragraphs()
            .iter()
            .flat_map(|paragraph| paragraph.marks.iter().map(|mark| mark.raw.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(marks(&app), ["<:X8,-32768;Index \"Printer\" #>"]);
    // Mark All, case folded: "printer" in paragraph 3 and "Printer" in 5,
    // not the one marked already; one undo step.
    send(
        &mut app,
        Message::Contents(ContentsAction::Primary("Printer".into())),
    );
    send(
        &mut app,
        Message::Contents(ContentsAction::ExactCase(false)),
    );
    let undo = app.history.undo_len();
    send(&mut app, Message::Contents(ContentsAction::MarkAll));
    assert_eq!(marks(&app).len(), 3);
    assert_eq!(app.history.undo_len(), undo + 1);
    // Next Mark shows the first entry; Remove takes it out.
    move_to(&mut app, at(main, 0, 0), 1, false);
    send(&mut app, Message::Contents(ContentsAction::NextMark));
    assert!(app.contents.as_ref().unwrap().current.is_some());
    send(&mut app, Message::Contents(ContentsAction::Remove));
    assert_eq!(marks(&app).len(), 2);
    // Generate Index: a new tab on its own, the source left as it was.
    let sessions = app.sessions.len();
    send(
        &mut app,
        Message::Contents(ContentsAction::IndexPlace(IndexChoice::NewDocument)),
    );
    send(&mut app, Message::Contents(ContentsAction::GenerateIndex));
    assert_eq!(app.sessions.len(), sessions + 1);
    let lines: Vec<String> = app
        .document
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect();
    assert_eq!(lines, ["Printer, 1, 2"]);
}

#[test]
fn a_toc_entry_is_marked_at_the_caret_in_one_step() {
    use crate::contents_panel::ContentsAction;
    let mut app = page_app(contents_document());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 1, 3), 1, false);
    send(&mut app, Message::Command(CommandId::MarkTocEntry));
    send(
        &mut app,
        Message::Contents(ContentsAction::TocLevel("2".into())),
    );
    send(
        &mut app,
        Message::Contents(ContentsAction::TocText("Setting up".into())),
    );
    send(&mut app, Message::Contents(ContentsAction::MarkToc));
    assert_eq!(
        app.document.paragraphs()[1].marks[0].raw,
        "<:X11,-32768;TOC 2 \"Setting up\">"
    );
}

#[test]
fn the_contents_panels_buttons_take_clicks() {
    use crate::contents_panel::{ContentsAction, Page};
    let mut runtime = Runtime::new(page_app(contents_document()));
    runtime.redraw();
    runtime.send(Message::Command(CommandId::ContentsIndex));
    runtime.redraw();
    let found = |runtime: &mut Runtime| {
        let mut found = Vec::new();
        let mut y = 100.0;
        while y < Runtime::WINDOW.height - 40.0 {
            for x in [60.0, 150.0, 240.0] {
                for message in runtime.click_unrun(
                    cosmic::iced::mouse::Button::Left,
                    cosmic::iced::Point::new(x, y),
                ) {
                    if let Message::Contents(action) = message {
                        found.push(format!("{action:?}"));
                    }
                }
            }
            y += 6.0;
        }
        found
    };
    let actions = found(&mut runtime);
    assert!(
        actions.iter().any(|action| action == "GenerateToc"),
        "{actions:?}"
    );
    assert!(
        actions.iter().any(|action| action == "Page(Entries)"),
        "{actions:?}"
    );
    // A click on Generate does what the message does.
    runtime.send(Message::Contents(ContentsAction::GenerateToc));
    assert!(amipro_editing::contents::toc_table(&runtime.app.document).is_some());
    runtime.send(Message::Contents(ContentsAction::Page(Page::Entries)));
    runtime.redraw();
    let actions = found(&mut runtime);
    assert!(
        actions.iter().any(|action| action == "NextMark"
            || action.starts_with("TocText")
            || action == "MarkToc"
            || action.starts_with("PageNumber")),
        "{actions:?}"
    );
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Contents & Index panel's three pages
/// with a generated TOC, in both themes.
#[test]
#[ignore = "writes screenshots"]
fn contents_screenshots() {
    use crate::contents_panel::{ContentsAction, Page};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(contents_document());
    drop(app.update_title());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 1, 4), 1, false);
    move_to(&mut app, at(main, 1, 11), 1, true);
    send(&mut app, Message::Command(CommandId::MarkIndexEntry));
    send(&mut app, Message::Contents(ContentsAction::Mark));
    send(
        &mut app,
        Message::Contents(ContentsAction::Page(Page::Contents)),
    );
    send(&mut app, Message::Contents(ContentsAction::GenerateToc));
    for page in [Page::Contents, Page::Index, Page::Entries] {
        send(&mut app, Message::Contents(ContentsAction::Page(page)));
        for (dark, name) in [(false, "light"), (true, "dark")] {
            let theme = if dark {
                cosmic::Theme::dark()
            } else {
                cosmic::Theme::light()
            };
            let size = (1_280, 800);
            let rgb = render_window(&app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path =
                directory.join(format!("window-contents-{page:?}-{name}.ppm").to_lowercase());
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    }
}

#[test]
fn ctrl_click_on_a_toc_row_goes_to_its_heading() {
    use crate::contents_panel::ContentsAction;
    let mut app = page_app(contents_document());
    send(&mut app, Message::Command(CommandId::ContentsIndex));
    send(&mut app, Message::Contents(ContentsAction::GenerateToc));
    let table = amipro_editing::contents::toc_table(&app.document).unwrap();
    let FrameContent::Table(toc) = &app.document.frame(table).unwrap().content else {
        panic!("a table");
    };
    // Row 2 is "Advanced", the fifth paragraph after the TOC's own.
    let amipro_document::CellContent::Text(cell) = toc.cells[4].content;
    send(
        &mut app,
        Message::Modifiers(cosmic::iced::keyboard::Modifiers::CTRL),
    );
    page(
        &mut app,
        PageAction::MoveTo {
            position: at(cell, 0, 2),
            page: 1,
            extend: false,
        },
    );
    let caret = app.page_selection.focus;
    assert_eq!(caret.flow, app.document.main_flow_id());
    assert_eq!(app.document.paragraphs()[caret.paragraph].text, "Advanced");
    // Without Ctrl a click stays in the cell.
    send(
        &mut app,
        Message::Modifiers(cosmic::iced::keyboard::Modifiers::empty()),
    );
    page(
        &mut app,
        PageAction::MoveTo {
            position: at(cell, 0, 2),
            page: 1,
            extend: false,
        },
    );
    assert_eq!(app.page_selection.focus.flow, cell);
}

#[test]
fn the_pdf_links_each_toc_row_to_its_heading() {
    use crate::contents_panel::ContentsAction;
    let mut app = page_app(contents_document());
    send(&mut app, Message::Command(CommandId::ContentsIndex));
    send(&mut app, Message::Contents(ContentsAction::GenerateToc));
    let layout = app.pages.layout();
    let pages: Vec<usize> = (0..layout.pages.len()).collect();
    let links = crate::print::toc_links(&app.document, layout, &pages, 0, 0.0);
    // The TOC's three rows on its own page, nothing elsewhere.
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[&0].len(), 3);
    // The PDF has them.
    let pdf = crate::print::document_pdf(
        &app.document,
        layout,
        None,
        &crate::print::PdfOptions::default(),
    )
    .unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.matches("/Link").count() >= 3, "link annotations");
}

/// A glossary data file of its own under `target/`, for one test.
fn scratch_glossary(name: &str) -> std::path::PathBuf {
    let folder = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/glossary-tests")
        .join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("GLOSSARY.SAM");
    let _ = std::fs::remove_file(&path);
    std::fs::write(
        &path,
        amipro_formats::glossary::new_glossary(b"[ver]\r\n\t4\r\n[edoc]\r\n>\r\n"),
    )
    .unwrap();
    path
}

#[test]
fn a_glossary_record_is_marked_expanded_with_ctrl_k_and_undone_in_one_step() {
    use crate::glossary_panel::GlossaryAction;
    let path = scratch_glossary("ctrl-k");
    let mut app = page_app(Document::from_plain_text("Sincerely yours, Ami.\nSay sy"));
    app.preferences.glossary_file = path.display().to_string();
    let main = app.document.main_flow_id();
    // Mark Text ▸ Glossary…: the selection as the record "sy".
    move_to(&mut app, at(main, 0, 0), 1, false);
    move_to(&mut app, at(main, 0, 21), 1, true);
    send(&mut app, Message::Command(CommandId::MarkGlossary));
    send(
        &mut app,
        Message::Glossary(GlossaryAction::Name("sy".into())),
    );
    send(&mut app, Message::Glossary(GlossaryAction::Add));
    let written = std::fs::read(&path).unwrap();
    let glossary = amipro_formats::glossary::read_glossary(&written).unwrap();
    assert_eq!(glossary.names(), ["sy"]);
    assert_eq!(
        app.glossary.as_ref().unwrap().records[0].preview,
        "Sincerely yours, Ami."
    );
    // A name in use is refused with Ami Pro's message.
    send(
        &mut app,
        Message::Glossary(GlossaryAction::Name("SY".into())),
    );
    send(&mut app, Message::Glossary(GlossaryAction::Add));
    assert_eq!(
        app.glossary.as_ref().unwrap().message.as_deref(),
        Some("Glossary name already in use.")
    );
    // Ctrl+K after "sy": the word becomes the record, one undo step.
    move_to(&mut app, at(main, 1, 6), 1, false);
    let undo = app.history.undo_len();
    send(&mut app, Message::Command(CommandId::InsertGlossary));
    let texts: Vec<String> = app
        .document
        .paragraphs()
        .iter()
        .map(|p| p.text.clone())
        .collect();
    assert_eq!(
        app.document.paragraphs()[1].text,
        "Say Sincerely yours, Ami.",
        "{texts:?} {}",
        app.status
    );
    assert_eq!(app.history.undo_len(), undo + 1);
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document.paragraphs()[1].text, "Say sy");
    // A word that names no record opens the panel's list.
    app.glossary = None;
    move_to(&mut app, at(main, 1, 3), 1, false);
    send(&mut app, Message::Command(CommandId::InsertGlossary));
    assert!(app.glossary.is_some());
    // Rename and Delete change the data file.
    send(
        &mut app,
        Message::Glossary(GlossaryAction::Select("sy".into())),
    );
    send(
        &mut app,
        Message::Glossary(GlossaryAction::RenameText("yours".into())),
    );
    send(&mut app, Message::Glossary(GlossaryAction::Rename));
    let names = |path: &std::path::Path| {
        amipro_formats::glossary::read_glossary(&std::fs::read(path).unwrap())
            .unwrap()
            .names()
    };
    assert_eq!(names(&path), ["yours"]);
    send(&mut app, Message::Glossary(GlossaryAction::Delete));
    assert!(names(&path).is_empty());
}

#[test]
fn glossary_names_complete_as_you_type_when_asked() {
    use crate::glossary_panel::GlossaryAction;
    let path = scratch_glossary("complete");
    let bytes = amipro_formats::glossary::append_record(
        &std::fs::read(&path).unwrap(),
        "addr",
        b"1 Main Street",
    )
    .unwrap();
    std::fs::write(&path, bytes).unwrap();
    let mut app = page_app(Document::from_plain_text(""));
    app.preferences.glossary_file = path.display().to_string();
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    page(&mut app, PageAction::Insert("addr".into()));
    // Off by default: Tab is a tab.
    page(&mut app, PageAction::Insert("\t".into()));
    assert_eq!(app.document.paragraphs()[0].text, "addr\t");
    send(&mut app, Message::Command(CommandId::Undo));
    send(&mut app, Message::Glossary(GlossaryAction::Complete(true)));
    move_to(&mut app, at(main, 0, 4), 1, false);
    page(&mut app, PageAction::ParagraphBreak);
    page(&mut app, PageAction::Insert("addr".into()));
    assert!(app.status.contains("addr"), "{}", app.status);
    page(&mut app, PageAction::Insert("\t".into()));
    assert_eq!(app.document.paragraphs()[1].text, "1 Main Street");
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Glossary panel with two records, one
/// selected, in both themes.
#[test]
#[ignore = "writes screenshots"]
fn glossary_screenshots() {
    use crate::glossary_panel::GlossaryAction;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let path = scratch_glossary("shots");
    let mut bytes = std::fs::read(&path).unwrap();
    for (name, text) in [
        ("addr", &b"1 Main Street, Springfield"[..]),
        ("sy", &b"Sincerely yours, <+!>Ami<-!>."[..]),
    ] {
        bytes = amipro_formats::glossary::append_record(&bytes, name, text).unwrap();
    }
    std::fs::write(&path, bytes).unwrap();
    let mut app = page_app(Document::from_plain_text("Dear reader,\nsy"));
    app.preferences.glossary_file = path.display().to_string();
    drop(app.update_title());
    send(&mut app, Message::Command(CommandId::MarkGlossary));
    send(
        &mut app,
        Message::Glossary(GlossaryAction::Select("sy".into())),
    );
    for (dark, name) in [(false, "light"), (true, "dark")] {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(&app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("window-glossary-{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    }
}

/// `AMIPRO_VM_LAB=…/vm-lab AMIPRO_TWIN_OUT=DIR`: the run's twins, made by
/// the panel's commands and written by Save As SAM, for Ami Pro 3.1 to
/// open (`docs/TOC_INDEX_PROGRESS.md`, TC.11):
/// - ATI1: TI1A with the TOC generated at its start and one more index
///   entry marked by Mark All;
/// - ATI1X: TI1A's index, made in a new tab;
/// - ATI3: TI3 with the TOC at the caret (its third paragraph);
/// - AGL1.SAM: a glossary data file the app made, with two records.
#[test]
#[ignore = "writes VM twins"]
fn contents_twins() {
    use crate::contents_panel::{ContentsAction, IndexChoice, TocChoice};
    use crate::glossary_panel::GlossaryAction;
    let (Some(lab), Some(out)) = (
        std::env::var_os("AMIPRO_VM_LAB").map(std::path::PathBuf::from),
        std::env::var_os("AMIPRO_TWIN_OUT").map(std::path::PathBuf::from),
    ) else {
        eprintln!("AMIPRO_VM_LAB or AMIPRO_TWIN_OUT is not set; skipping");
        return;
    };
    let read = |name: &str| {
        amipro_formats::import_sam_path_with_style_directories(
            lab.join("IN").join(name),
            &[lab.join("STY")],
        )
        .unwrap()
        .document
    };
    let write = |app: &mut AmiProApp, name: &str| {
        let path = out.join(name);
        let _ = std::fs::remove_file(&path);
        assert!(app.write_sam(&path, 0, false), "{name}: {}", app.status);
    };
    // ATI1 and ATI1X.
    let mut app = page_app(read("TI1A.SAM"));
    send(&mut app, Message::Command(CommandId::ContentsIndex));
    send(&mut app, Message::Contents(ContentsAction::GenerateToc));
    assert!(amipro_editing::contents::toc_table(&app.document).is_some());
    send(&mut app, Message::Command(CommandId::MarkIndexEntry));
    send(
        &mut app,
        Message::Contents(ContentsAction::Primary("disk".into())),
    );
    send(&mut app, Message::Contents(ContentsAction::MarkAll));
    write(&mut app, "ATI1.SAM");
    let written =
        String::from_utf8_lossy(&std::fs::read(out.join("ATI1.SAM")).unwrap()).into_owned();
    assert!(
        written.contains("[toc]\r\n\tTitle\r\n\tSubhead\r\n"),
        "the levels written"
    );
    send(
        &mut app,
        Message::Contents(ContentsAction::Separators(true)),
    );
    send(
        &mut app,
        Message::Contents(ContentsAction::IndexPlace(IndexChoice::NewDocument)),
    );
    let shown: Vec<(u32, u32)> = app
        .pages
        .layout()
        .pages
        .iter()
        .map(|page| (page.number, page.shown))
        .collect();
    assert_eq!(shown[1], (2, 1), "the TOC's page not counted: {shown:?}");
    send(&mut app, Message::Contents(ContentsAction::GenerateIndex));
    write(&mut app, "ATI1X.SAM");
    // ATI3: the TOC at the caret.
    let mut app = page_app(read("TI3.SAM"));
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 2, 0), 1, false);
    send(&mut app, Message::Command(CommandId::ContentsIndex));
    send(
        &mut app,
        Message::Contents(ContentsAction::TocPlace(TocChoice::Caret)),
    );
    send(&mut app, Message::Contents(ContentsAction::GenerateToc));
    write(&mut app, "ATI3.SAM");
    // AGL1: a new glossary data file and two records marked into it.
    let glossary = out.join("AGL1.SAM");
    let _ = std::fs::remove_file(&glossary);
    let mut app = page_app(Document::from_plain_text(
        "Sincerely yours, Ami.\nThe printer",
    ));
    send(&mut app, Message::Command(CommandId::MarkGlossary));
    app.glossary_file_chosen(Some(glossary.clone()), true);
    let main = app.document.main_flow_id();
    for (paragraph, length, name) in [(0, 21, "sy"), (1, 11, "pr")] {
        move_to(&mut app, at(main, paragraph, 0), 1, false);
        move_to(&mut app, at(main, paragraph, length), 1, true);
        send(
            &mut app,
            Message::Glossary(GlossaryAction::Name(name.into())),
        );
        send(&mut app, Message::Glossary(GlossaryAction::Add));
    }
    let records = amipro_formats::glossary::read_glossary(&std::fs::read(&glossary).unwrap())
        .unwrap()
        .names();
    assert_eq!(records, ["pr", "sy"]);
}

/// `AMIPRO_PERF_SAM=FILE` (release): collecting and generating the TOC
/// and the index, and Mark All, on a document.
#[test]
#[ignore = "release performance baseline"]
fn contents_performance_baseline() {
    use crate::contents_panel::ContentsAction;
    let document = std::env::var_os("AMIPRO_PERF_SAM").map_or_else(page_fixture, |path| {
        amipro_formats::import_sam_path(path).unwrap().document
    });
    let mut app = page_app(document);
    send(&mut app, Message::Command(CommandId::ContentsIndex));
    let time = |app: &mut AmiProApp, action: ContentsAction| {
        let started = Instant::now();
        send(app, Message::Contents(action));
        started.elapsed().as_secs_f64() * 1_000.0
    };
    let toc = time(&mut app, ContentsAction::GenerateToc);
    let update = time(&mut app, ContentsAction::GenerateToc);
    send(
        &mut app,
        Message::Contents(ContentsAction::Primary("the".into())),
    );
    send(
        &mut app,
        Message::Contents(ContentsAction::ExactCase(false)),
    );
    let mark_all = time(&mut app, ContentsAction::MarkAll);
    let marks = app
        .document
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.marks.len())
        .sum::<usize>();
    let index = time(&mut app, ContentsAction::GenerateIndex);
    eprintln!(
        "TOC {toc:.1} ms, update {update:.1} ms; Mark All \"the\" {mark_all:.1} ms ({marks} marks); index {index:.1} ms; status: {}",
        app.status
    );
}

// ------------------------------------------------------- master documents

/// A scratch folder for one master documents test, emptied first.
fn master_scratch(name: &str) -> std::path::PathBuf {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/master-tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// A member of two pages with two footnotes and a numbered list ending it,
/// as VM fixture MD3's members.
fn master_member(folder: &std::path::Path, name: &str) -> std::path::PathBuf {
    let edoc = [
        format!("@Title@{name} chapter one"),
        format!("Text of {name}.<:F\r\n@Footnote@First of {name}.\r\n\r\n>"),
        "The end of page one.<:p<* >>".to_string(),
        format!(
            "More text of {name}.<:F\r\n@Footnote@Second of {name}.\r\n\r\n><:X8,-32768;Index \"Common\" #>"
        ),
    ]
    .map(|paragraph| format!("{paragraph}\r\n\r\n"))
    .concat();
    let bytes = format!("[ver]\r\n\t4\r\n[edoc]\r\n{edoc}>\r\n\r\n");
    let path = folder.join(format!("{name}.SAM"));
    std::fs::write(&path, bytes).unwrap();
    path
}

fn master_session(app: &AmiProApp, name: &str) -> usize {
    app.sessions
        .iter()
        .position(|session| {
            session
                .file_path()
                .is_some_and(|path| path.file_name().is_some_and(|file| file == name))
        })
        .unwrap()
}

#[test]
fn a_master_documents_files_carry_their_numbers_live_in_one_undo_step_each() {
    use crate::master_panel::MasterAction;
    let folder = master_scratch("carry");
    let paths: Vec<_> = ["MA", "MB", "MC"]
        .iter()
        .map(|name| master_member(&folder, name))
        .collect();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("The master.")));
    runtime.send(Message::Command(CommandId::MasterDocument));
    assert!(runtime.app.master_panel.is_some());
    let depth = runtime.app.history.undo_len();
    runtime.send(Message::Master(MasterAction::Added(paths.clone())));
    assert_eq!(runtime.app.history.undo_len(), depth + 1, "one undo step");
    let files = runtime.app.master_files_for_tests();
    assert_eq!(files.len(), 3);
    assert_eq!(files[0], paths[0].to_string_lossy());
    // The files are read off the interface's thread, then carried.
    runtime.send(Message::Repaint);
    let set = runtime.app.master_set.as_ref().unwrap();
    let pages: Vec<_> = set.members.iter().map(|member| member.pages).collect();
    assert_eq!(pages, [Some((1, 2)), Some((3, 4)), Some((5, 6))]);
    assert_eq!(set.total_footnotes(), 6);
    // MB opened from the panel continues from MA, live.
    runtime.send(Message::Master(MasterAction::Open(1)));
    let mb = master_session(&runtime.app, "MB.SAM");
    let sequence = runtime.app.sessions[mb]
        .document
        .contents()
        .sequence
        .clone()
        .unwrap();
    assert_eq!((sequence.page_base, sequence.footnotes), (2, 2));
    let shown: Vec<u32> = runtime.app.sessions[mb]
        .pages
        .layout()
        .pages
        .iter()
        .map(|page| page.shown)
        .collect();
    assert_eq!(shown, [3, 4]);
    assert!(
        !runtime.app.sessions[mb].dirty,
        "the set's numbers alone change nothing"
    );
    // A page added to MA, open in a tab, moves MB's numbers at once (MS3).
    runtime.app.active_session = 0;
    runtime.send(Message::Master(MasterAction::Open(0)));
    let ma = master_session(&runtime.app, "MA.SAM");
    runtime.app.active_session = ma;
    let end = TextPosition {
        flow: runtime.app.document.main_flow_id(),
        paragraph: 0,
        byte: runtime.app.document.paragraphs()[0].text.len(),
    };
    assert!(
        runtime
            .app
            .execute_document_command(&amipro_editing::Command::InsertBreak {
                at: end,
                kind: amipro_document::BreakKind::Page,
                centre: false,
                layout: None,
            })
    );
    runtime.send(Message::Repaint);
    let mb = master_session(&runtime.app, "MB.SAM");
    let sequence = runtime.app.sessions[mb]
        .document
        .contents()
        .sequence
        .clone()
        .unwrap();
    assert_eq!(sequence.page_base, 3);
    // Undo in the master removes the files; the members return to their own
    // numbers.
    let master = runtime
        .app
        .sessions
        .iter()
        .position(|session| session.master_set.is_some())
        .unwrap();
    runtime.app.active_session = master;
    runtime.send(Message::Command(CommandId::Undo));
    assert!(runtime.app.master_files_for_tests().is_empty());
    let mb = master_session(&runtime.app, "MB.SAM");
    assert!(
        runtime.app.sessions[mb]
            .document
            .contents()
            .sequence
            .is_none()
    );
}

#[test]
fn a_member_is_treated_as_a_single_document_and_finds_its_master() {
    use crate::master_panel::MasterAction;
    let folder = master_scratch("member");
    master_member(&folder, "MA");
    let path = master_member(&folder, "MB");
    // A master in the same folder lists MB by a DOS path (MS2, MS4).
    std::fs::write(
        folder.join("MS.SAM"),
        "[ver]\r\n\t4\r\n[master]\r\n\t\r\n\t\r\n\t0\r\n\t30\r\n\t2\r\n\tc:\\book\\ma.sam\r\n\tc:\\book\\mb.sam\r\n[edoc]\r\nSource.\r\n\r\n>\r\n\r\n",
    )
    .unwrap();
    // MB as Ami Pro's run left it: pages after 2, footnotes after 2.
    let mut bytes = std::fs::read_to_string(&path).unwrap();
    bytes = bytes.replace(
        "[edoc]",
        "[sequence]\r\n\t2\r\n\t2\r\n\t0\r\n\t0\r\n\t0\r\n[edoc]",
    );
    std::fs::write(&path, bytes).unwrap();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("x")));
    runtime.app.open_document_path(path.clone());
    runtime.send(Message::Command(CommandId::MasterDocument));
    assert_eq!(
        runtime
            .app
            .master_offer
            .clone()
            .flatten()
            .and_then(|path| path.file_name().map(|name| name.to_os_string())),
        Some("MS.SAM".into())
    );
    let shown: Vec<u32> = runtime
        .app
        .pages
        .layout()
        .pages
        .iter()
        .map(|page| page.shown)
        .collect();
    assert_eq!(shown, [3, 4], "the stored numbers");
    // Open Master: MB in its folder, found by name, takes the set's numbers.
    runtime.send(Message::Master(MasterAction::OpenMaster));
    runtime.send(Message::Repaint);
    let master = master_session(&runtime.app, "MS.SAM");
    let set = runtime.app.sessions[master].master_set.as_ref().unwrap();
    assert!(
        set.members.iter().all(|member| member.path.is_some()),
        "found by name"
    );
    let mb = master_session(&runtime.app, "MB.SAM");
    assert!(runtime.app.sessions[mb].live_sequence.is_some());
    // Treat as Single Document: one undo step; its numbers start from 1.
    runtime.app.active_session = mb;
    let depth = runtime.app.history.undo_len();
    runtime.send(Message::Master(MasterAction::TreatAsSingle));
    assert_eq!(runtime.app.history.undo_len(), depth + 1);
    assert!(runtime.app.document.contents().sequence.is_none());
}

#[test]
fn the_master_panels_buttons_take_clicks() {
    let folder = master_scratch("clicks");
    let paths = vec![master_member(&folder, "MA"), master_member(&folder, "MB")];
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("The master.")));
    runtime.redraw();
    runtime.send(Message::Command(CommandId::MasterDocument));
    runtime.send(Message::Master(crate::master_panel::MasterAction::Added(
        paths,
    )));
    runtime.send(Message::Master(crate::master_panel::MasterAction::Select(
        0,
    )));
    runtime.redraw();
    let mut found = Vec::new();
    let mut y = 100.0;
    while y < Runtime::WINDOW.height - 40.0 {
        for x in [40.0, 90.0, 140.0, 200.0, 250.0] {
            for message in runtime.click_unrun(
                cosmic::iced::mouse::Button::Left,
                cosmic::iced::Point::new(x, y),
            ) {
                if let Message::Master(action) = message {
                    found.push(format!("{action:?}"));
                }
            }
        }
        y += 6.0;
    }
    for wanted in ["Add", "Remove", "Down", "OpenAll", "GenerateContents"] {
        assert!(
            found.iter().any(|action| action == wanted),
            "{wanted}: {found:?}"
        );
    }
}

#[test]
fn a_new_master_saved_beside_its_files_finds_them_by_name_ms2() {
    // A new master listing DOS paths finds nothing while it has no folder;
    // saved into its files' folder, it finds them by name there (MS2).
    let folder = master_scratch("saved");
    master_member(&folder, "MA");
    let mut master = Document::from_plain_text("The master.");
    let files = vec!["l:\\in\\md\\ma.sam".to_string()];
    master = amipro_editing::apply(&master, &amipro_editing::master::set_files(&master, files))
        .unwrap()
        .document;
    let mut runtime = Runtime::new(page_app(master));
    runtime.send(Message::Command(CommandId::MasterDocument));
    runtime.send(Message::Repaint);
    let found = |runtime: &Runtime| {
        runtime.app.master_set.as_ref().unwrap().members[0]
            .path
            .is_some()
    };
    assert!(!found(&runtime));
    assert!(runtime.app.write_sam(&folder.join("MS.SAM"), 0, false));
    runtime.send(Message::Repaint);
    assert!(found(&runtime), "found by name once saved beside it");
    let pages = runtime.app.master_set.as_ref().unwrap().members[0].pages;
    assert_eq!(pages, Some((1, 2)));
}

#[test]
fn the_find_bar_searches_all_files_of_the_master_ms11() {
    // MS11: In All Files of the Master counts the matches file by file,
    // Find Next goes on into the next file's tab past this one's last,
    // Previous back, and Replace All changes each file in one undo step
    // there.
    use crate::find_bar::{FindAction, FindMenu, Toggle};
    use crate::master_panel::MasterAction;
    let folder = master_scratch("find");
    let paths: Vec<_> = ["MA", "MB"]
        .iter()
        .map(|name| master_member(&folder, name))
        .collect();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("The master.")));
    runtime.send(Message::Command(CommandId::MasterDocument));
    runtime.send(Message::Master(MasterAction::Added(paths)));
    runtime.send(Message::Repaint);
    runtime.send(Message::Command(CommandId::Find));
    runtime.send(Message::Find(FindAction::Text(
        crate::find_bar::Field::Find,
        "text of".into(),
    )));
    assert!(runtime.app.find.as_ref().unwrap().matches.is_empty());
    runtime.send(Message::Find(FindAction::Menu(FindMenu::Toggle(
        Toggle::AllFiles,
    ))));
    let bar = runtime.app.find.as_ref().unwrap();
    let counts: Vec<_> = bar
        .files
        .iter()
        .map(|file| (file.name.as_str(), file.count))
        .collect();
    assert_eq!(
        counts,
        [("MA.SAM", 2), ("MB.SAM", 2)],
        "Text of, More text of"
    );
    assert_eq!(bar.count_label(), "4 in 2 files");

    // The widget path: the bar's Find Next takes the click, though the
    // master itself has no match.
    runtime.redraw();
    let mut next = None;
    // The bar's row (Find Next sits near x 992, y 164 in the test window).
    'scan: for y in (148..184).step_by(4) {
        for x in (960..1_030).step_by(6) {
            for message in runtime.click_unrun(
                cosmic::iced::mouse::Button::Left,
                cosmic::iced::Point::new(x as f32, y as f32),
            ) {
                if matches!(message, Message::Find(FindAction::Next)) {
                    next = Some(message);
                    break 'scan;
                }
            }
        }
    }
    runtime.send(next.expect("Find Next takes a click"));
    let shown = |runtime: &Runtime| {
        let session = &runtime.app.sessions[runtime.app.active_session];
        (
            session
                .file_path()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            session.status.clone(),
        )
    };
    assert_eq!(
        shown(&runtime),
        ("MA.SAM".into(), "1 of 2 in MA.SAM".into())
    );
    runtime.send(Message::Find(FindAction::Next));
    assert_eq!(shown(&runtime).1, "2 of 2 in MA.SAM");
    runtime.send(Message::Find(FindAction::Next));
    assert_eq!(
        shown(&runtime),
        ("MB.SAM".into(), "1 of 2 in MB.SAM".into())
    );
    runtime.send(Message::Find(FindAction::Previous));
    assert_eq!(
        shown(&runtime),
        ("MA.SAM".into(), "2 of 2 in MA.SAM".into())
    );
    // Round the set from the last file to the first.
    runtime.send(Message::Find(FindAction::Next));
    runtime.send(Message::Find(FindAction::Next));
    runtime.send(Message::Find(FindAction::Next));
    assert_eq!(
        shown(&runtime),
        ("MA.SAM".into(), "1 of 2 in MA.SAM".into())
    );

    // Replace All from the master's tab: each file in one step.
    let master = runtime
        .app
        .sessions
        .iter()
        .position(|session| session.file_path().is_none())
        .unwrap();
    runtime.app.active_session = master;
    runtime.send(Message::Command(CommandId::Replace));
    runtime.send(Message::Find(FindAction::Text(
        crate::find_bar::Field::Find,
        "text of".into(),
    )));
    if !runtime.app.find.as_ref().unwrap().all_files {
        runtime.send(Message::Find(FindAction::Menu(FindMenu::Toggle(
            Toggle::AllFiles,
        ))));
    }
    runtime.send(Message::Find(FindAction::Text(
        crate::find_bar::Field::Replace,
        "words of".into(),
    )));
    runtime.send(Message::Find(FindAction::ReplaceAll));
    assert_eq!(runtime.app.active_session, master);
    assert_eq!(runtime.app.status, "Replaced 4 matches in 2 files");
    assert_eq!(
        runtime.app.find.as_ref().unwrap().count_label(),
        "No matches"
    );
    let text = |runtime: &Runtime, name: &str| {
        runtime.app.sessions[master_session(&runtime.app, name)]
            .document
            .main_flow()
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.text.clone())
            .collect::<Vec<_>>()
            .join("|")
    };
    assert!(text(&runtime, "MA.SAM").contains("words of MA."));
    assert!(!text(&runtime, "MB.SAM").contains("More text of MB."));
    // One undo step in MB returns all of MB's, and leaves MA's.
    runtime.app.active_session = master_session(&runtime.app, "MB.SAM");
    runtime.send(Message::Command(CommandId::Undo));
    assert!(text(&runtime, "MB.SAM").contains("Text of MB."));
    assert!(text(&runtime, "MB.SAM").contains("More text of MB."));
    assert!(text(&runtime, "MA.SAM").contains("words of MA."));
}

/// MR.12's twins (`AMIPRO_VM_LAB` = vm-lab/OUT, `AMIPRO_TWIN_OUT`): VM
/// fixture MD3's three members without their `[sequence]` become the
/// files of a master the app makes (MD3's TOC levels, `l:\\in\\amd\\…`
/// paths, output files in `L:\\IN\\AMD`), found by name in its folder
/// (MS2); the app numbers the set, and Save As SAM writes the master
/// (`AMS.SAM`) and each member with its `[sequence]` (`AMA.SAM`, `AMB.SAM`,
/// `AMC.SAM`) into `AMIPRO_TWIN_OUT`, for Ami Pro to open from
/// `L:\\IN\\AMD`. The members' `[sequence]` equal what Ami Pro wrote
/// into MD3's; the set's TOC and index are MD3's.
#[test]
#[ignore = "needs AMIPRO_VM_LAB and AMIPRO_TWIN_OUT"]
fn master_twins() {
    use crate::master_panel::MasterAction;
    let (Some(lab), Some(out)) = (
        std::env::var_os("AMIPRO_VM_LAB"),
        std::env::var_os("AMIPRO_TWIN_OUT"),
    ) else {
        eprintln!("AMIPRO_VM_LAB or AMIPRO_TWIN_OUT is not set; skipping");
        return;
    };
    let md3 = std::path::PathBuf::from(lab).join("MD3");
    let out = std::path::PathBuf::from(out);
    let src = out.join("src");
    let _ = std::fs::remove_dir_all(&src);
    std::fs::create_dir_all(&src).unwrap();
    for name in ["AMS.SAM", "AMA.SAM", "AMB.SAM", "AMC.SAM"] {
        let _ = std::fs::remove_file(out.join(name));
    }
    // A section of a SAM file's lines, by name.
    let section = |bytes: &[u8], name: &str| -> Vec<String> {
        let text = String::from_utf8_lossy(bytes).replace('\r', "");
        let mut lines = text.lines();
        let header = format!("[{name}]");
        lines.by_ref().find(|line| line.trim() == header);
        lines
            .take_while(|line| !line.starts_with('['))
            .map(|line| line.trim().to_owned())
            .collect()
    };
    let without_sequence = |bytes: &[u8]| -> Vec<u8> {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let start = text.find("[sequence]").unwrap();
        let end = start + text[start..].find("[prn]").unwrap();
        format!("{}{}", &text[..start], &text[end..]).into_bytes()
    };
    for (from, to) in [
        ("MA.SAM", "AMA.SAM"),
        ("MB.SAM", "AMB.SAM"),
        ("MC.SAM", "AMC.SAM"),
    ] {
        let bytes = std::fs::read(md3.join(from)).unwrap();
        let bytes = if from == "MA.SAM" {
            bytes
        } else {
            without_sequence(&bytes)
        };
        std::fs::write(src.join(to), bytes).unwrap();
    }
    // The master: MD3's TOC levels, the app's file list and outputs.
    let mut master = amipro_formats::import_sam_path(md3.join("MS.SAM"))
        .unwrap()
        .document;
    let mut contents = master.contents().clone();
    contents.master = None;
    master.set_contents(contents);
    let files: Vec<String> = ["ama", "amb", "amc"]
        .iter()
        .map(|name| format!("l:\\in\\amd\\{name}.sam"))
        .collect();
    master = amipro_editing::apply(&master, &amipro_editing::master::set_files(&master, files))
        .unwrap()
        .document;
    let outputs = amipro_editing::master::set_outputs(
        &master,
        "L:\\IN\\AMD\\AMTOC.SAM",
        "L:\\IN\\AMD\\AMIDX.SAM",
        true,
    );
    master = amipro_editing::apply(&master, &outputs).unwrap().document;
    let mut app = page_app(master);
    assert!(
        app.write_sam(&src.join("AMS.SAM"), 0, false),
        "{}",
        app.status
    );
    let mut runtime = Runtime::new(app);
    runtime.app.open_document_path(src.join("AMS.SAM"));
    runtime.send(Message::Command(CommandId::MasterDocument));
    runtime.send(Message::Repaint);
    let master_tab = runtime.app.active_session;
    let set = runtime.app.master_set.as_ref().unwrap();
    assert!(
        set.members.iter().all(|member| member.path.is_some()),
        "MS2: the files found by name in the master's folder"
    );
    for (index, name) in ["AMA.SAM", "AMB.SAM", "AMC.SAM"].iter().enumerate() {
        runtime.app.active_session = master_tab;
        runtime.send(Message::Master(MasterAction::Open(index)));
        runtime.send(Message::Repaint);
        let path = out.join(name);
        assert!(
            runtime.app.write_sam(&path, 0, false),
            "{name}: {}",
            runtime.app.status
        );
        let ours = section(&std::fs::read(&path).unwrap(), "sequence");
        let theirs = section(&std::fs::read(md3.join(&name[1..])).unwrap(), "sequence");
        assert_eq!(
            ours, theirs,
            "{name}: the [sequence] Ami Pro wrote into MD3's"
        );
        eprintln!("{name}: [sequence] {ours:?}");
    }
    runtime.app.active_session = master_tab;
    assert!(runtime.app.write_sam(&out.join("AMS.SAM"), 0, false));
    eprintln!(
        "[master] {:?}",
        section(&std::fs::read(out.join("AMS.SAM")).unwrap(), "master")
    );
}

/// Release timings of a master document (`AMIPRO_PERF_MASTER`: a folder
/// whose SAM files, in name order, are the set; `AMIPRO_STYLES`): the set
/// read, laid out, and numbered; a keystroke in a file in the middle with
/// the master open, and with the file alone; Find's counts over the set;
/// the set's DOCX. Median and worst.
#[test]
#[ignore = "release timing; needs AMIPRO_PERF_MASTER"]
fn master_performance_baseline() {
    use crate::find_bar::{FindAction, FindMenu, Toggle};
    use crate::master_panel::MasterAction;
    let Some(folder) = std::env::var_os("AMIPRO_PERF_MASTER") else {
        eprintln!("AMIPRO_PERF_MASTER is not set; skipping");
        return;
    };
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(folder)
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
        })
        .collect();
    paths.sort();
    let report = |name: &str, mut times: Vec<Duration>| {
        times.sort();
        eprintln!(
            "{name}: median {:.2} ms, worst {:.2} ms ({} runs)",
            times[times.len() / 2].as_secs_f64() * 1_000.0,
            times[times.len() - 1].as_secs_f64() * 1_000.0,
            times.len()
        );
    };
    let middle = paths.len() / 2;
    let mut reading = Vec::new();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("The master.")));
    runtime.send(Message::Command(CommandId::MasterDocument));
    for round in 0..3 {
        if round > 0 {
            runtime.send(Message::Master(MasterAction::Select(0)));
            for _ in 0..paths.len() {
                runtime.send(Message::Master(MasterAction::Remove));
                runtime.send(Message::Master(MasterAction::Select(0)));
            }
            runtime.send(Message::Master(MasterAction::Select(0)));
        }
        let started = Instant::now();
        runtime.send(Message::Master(MasterAction::Added(paths.clone())));
        runtime.send(Message::Repaint);
        reading.push(started.elapsed());
    }
    let pages: u32 = runtime
        .app
        .master_set
        .as_ref()
        .map(|set| {
            set.members
                .iter()
                .filter_map(|member| member.pages)
                .map(|(first, last)| last + 1 - first)
                .sum()
        })
        .unwrap_or(0);
    eprintln!("{} files, {pages} pages in the set", paths.len());
    report("the set read, laid out, and numbered", reading);
    let master = runtime.app.active_session;
    runtime.send(Message::Master(MasterAction::Open(middle)));
    let keystrokes = |runtime: &mut Runtime| {
        let mut times = Vec::new();
        for _ in 0..21 {
            let started = Instant::now();
            runtime.send(Message::Page(PageAction::Insert("x".into())));
            times.push(started.elapsed());
        }
        times
    };
    report(
        "a keystroke in a file with the master open",
        keystrokes(&mut runtime),
    );
    runtime.app.active_session = master;
    let mut finding = Vec::new();
    runtime.send(Message::Command(CommandId::Find));
    runtime.send(Message::Find(FindAction::Menu(FindMenu::Toggle(
        Toggle::AllFiles,
    ))));
    for word in ["the", "and", "of", "to", "a", "in", "is"] {
        let started = Instant::now();
        runtime.send(Message::Find(FindAction::Text(
            crate::find_bar::Field::Find,
            word.into(),
        )));
        finding.push(started.elapsed());
    }
    report("Find's counts over the set (a new word)", finding);
    let mut exporting = Vec::new();
    for _ in 0..3 {
        let set = runtime.app.print_set();
        let started = Instant::now();
        let prepared = crate::export::ExportJob {
            document: runtime.app.document.clone(),
            layout: runtime.app.pages.layout().clone(),
            folder: None,
            fields: runtime.app.field_context(),
            title: "Master".into(),
            format: amipro_exchange::Format::Docx,
            path: std::path::PathBuf::from("unused"),
            set,
        }
        .run()
        .unwrap();
        exporting.push(started.elapsed());
        assert!(prepared.bytes.starts_with(b"PK"));
    }
    report("the set as one DOCX (built and written)", exporting);
    // The same file alone.
    let mut alone = Runtime::new(page_app(
        amipro_formats::import_sam_path(&paths[middle])
            .unwrap()
            .document,
    ));
    report("a keystroke in the file alone", keystrokes(&mut alone));
}

#[test]
fn the_toc_and_index_of_a_master_document_take_every_file_with_its_pages_md3() {
    use crate::master_panel::MasterAction;
    let folder = master_scratch("contents");
    let paths: Vec<_> = ["MA", "MB", "MC"]
        .iter()
        .map(|name| master_member(&folder, name))
        .collect();
    // The master's TOC levels apply to every file (Ami Pro's run).
    let master = amipro_formats::import_sam(
        b"[ver]\r\n\t4\r\n[toc]\r\n\tTitle\r\n\t\r\n\t\r\n\t.\r\n\t\r\n\t\r\n\t3\r\n[edoc]\r\nThe master.\r\n\r\n>\r\n\r\n",
        None,
        "Master",
    )
    .unwrap()
    .document;
    let mut runtime = Runtime::new(page_app(master));
    runtime.send(Message::Command(CommandId::MasterDocument));
    runtime.send(Message::Master(MasterAction::Added(paths)));
    runtime.send(Message::Repaint);
    runtime.send(Message::Master(MasterAction::GenerateContents));
    let table = runtime
        .app
        .document
        .frames()
        .find_map(|frame| match &frame.content {
            amipro_document::FrameContent::Table(table) => Some(table.clone()),
            _ => None,
        })
        .expect("a TOC tab");
    let cell_text = |index: usize| {
        let amipro_document::CellContent::Text(flow) = table.cells[index].content;
        runtime.app.document.flow(flow).unwrap().paragraphs[0]
            .text
            .clone()
    };
    let rows: Vec<(String, String)> = (0..3)
        .map(|row| (cell_text(2 * row), cell_text(2 * row + 1)))
        .collect();
    assert_eq!(
        rows,
        [
            ("MA chapter one".to_string(), "1".to_string()),
            ("MB chapter one".to_string(), "3".to_string()),
            ("MC chapter one".to_string(), "5".to_string())
        ]
    );
    assert_eq!(runtime.app.set_links.len(), 3);
    // The index: the marks of every file with their pages in the set.
    let master = runtime
        .app
        .sessions
        .iter()
        .position(|session| session.master_set.is_some())
        .unwrap();
    runtime.app.active_session = master;
    runtime.send(Message::Master(MasterAction::GenerateIndex));
    let text: Vec<String> = runtime
        .app
        .document
        .paragraphs()
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect();
    assert!(
        text.iter().any(|line| line == "Common, 2, 4, 6"),
        "{text:?}"
    );
}

#[test]
fn a_master_document_prints_its_whole_set_or_itself_ms6() {
    use crate::master_panel::MasterAction;
    let folder = master_scratch("print");
    let paths: Vec<_> = ["MA", "MB"]
        .iter()
        .map(|name| master_member(&folder, name))
        .collect();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("The master.")));
    runtime.send(Message::Command(CommandId::MasterDocument));
    runtime.send(Message::Master(MasterAction::Added(paths)));
    runtime.send(Message::Repaint);
    let job = runtime.app.pdf_job(crate::print::PdfOptions::default());
    assert_eq!(job.set.len(), 2, "the files, no TOC or index file named");
    assert_eq!(job.pages(), 4);
    let shown: Vec<u32> = job.set[1]
        .layout
        .pages
        .iter()
        .map(|page| page.shown)
        .collect();
    assert_eq!(shown, [3, 4], "the second file's pages in the set");
    let pdf = job.render().unwrap();
    assert!(pdf.starts_with(b"%PDF"));
    // This document only.
    runtime.send(Message::PrintPanel(
        crate::print_panel::PrintAction::WholeSet(false),
    ));
    assert!(
        runtime
            .app
            .pdf_job(crate::print::PdfOptions::default())
            .set
            .is_empty()
    );
}

#[test]
fn a_master_document_exports_its_whole_set_as_one_document_ms10() {
    // MS10: File › Export ▸ on a master writes every file in order, each a
    // section starting where its numbers start in the set; This document
    // only (the Print panel's choice) exports the master alone.
    use crate::master_panel::MasterAction;
    use amipro_exchange::Format;
    let folder = master_scratch("export");
    let paths: Vec<_> = ["MA", "MB"]
        .iter()
        .map(|name| master_member(&folder, name))
        .collect();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("The master.")));
    runtime.send(Message::Command(CommandId::MasterDocument));
    runtime.send(Message::Master(MasterAction::Added(paths)));
    runtime.send(Message::Repaint);
    let inflate = |data: &[u8], size: usize| -> Option<Vec<u8>> {
        use std::io::Read as _;
        let mut out = Vec::with_capacity(size);
        flate2::read::DeflateDecoder::new(data)
            .take(size as u64 + 1)
            .read_to_end(&mut out)
            .ok()?;
        Some(out)
    };
    for format in [Format::Docx, Format::Odt, Format::Rtf, Format::Text] {
        let path = folder.join(format!("Set.{}", format.extension()));
        let session_id = runtime.app.id;
        runtime.send(Message::ExportChosen {
            session_id,
            format,
            result: FileDialogResult::Selected(path.clone()),
        });
        if runtime.app.export_pending.is_some() {
            runtime.send(Message::ExportAnyway);
        }
        assert_eq!(runtime.app.exported.as_deref(), Some(path.as_path()));
        let bytes = std::fs::read(&path).unwrap();
        let read = amipro_exchange::import::read(
            &bytes,
            Some(format.extension()),
            &amipro_exchange::import::ImportOptions::default(),
            &inflate,
        )
        .unwrap()
        .document;
        let mut text = String::new();
        for section in &read.sections {
            for block in &section.blocks {
                if let amipro_exchange::Block::Paragraph(paragraph) = block {
                    for inline in &paragraph.inlines {
                        if let amipro_exchange::Inline::Text { text: part, .. } = inline {
                            text.push_str(part);
                        }
                    }
                    text.push('\n');
                }
            }
        }
        let at = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{format:?}: {needle} in {text:?}"))
        };
        assert!(at("Text of MA.") < at("Text of MB."), "{format:?}");
        assert!(
            !text.contains("The master."),
            "{format:?}: the master's own text"
        );
        if format != Format::Text {
            let starts: Vec<_> = read
                .sections
                .iter()
                .filter_map(|section| section.page_numbers.and_then(|numbers| numbers.start))
                .collect();
            assert_eq!(starts.last(), Some(&3), "{format:?}: MB's pages start at 3");
        }
    }
    // This document only.
    runtime.send(Message::PrintPanel(
        crate::print_panel::PrintAction::WholeSet(false),
    ));
    let path = folder.join("Master.txt");
    let session_id = runtime.app.id;
    runtime.send(Message::ExportChosen {
        session_id,
        format: Format::Text,
        result: FileDialogResult::Selected(path.clone()),
    });
    if runtime.app.export_pending.is_some() {
        runtime.send(Message::ExportAnyway);
    }
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("The master.") && !text.contains("Text of MA."));
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Master Document panel on a master
/// with three files and on one of its files, in both themes.
#[test]
#[ignore = "writes screenshots"]
fn master_screenshots() {
    use crate::master_panel::MasterAction;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let folder = master_scratch("screenshots");
    let paths: Vec<_> = ["MA", "MB", "MC"]
        .iter()
        .map(|name| master_member(&folder, name))
        .collect();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("The master.")));
    drop(runtime.app.update_title());
    runtime.send(Message::Command(CommandId::MasterDocument));
    runtime.send(Message::Master(MasterAction::Added(paths)));
    runtime.send(Message::Master(MasterAction::Select(1)));
    runtime.send(Message::Repaint);
    let mut shots = vec![("master", ())];
    shots.push(("member", ()));
    for (which, ()) in shots {
        if which == "member" {
            runtime.send(Message::Master(MasterAction::Open(1)));
            runtime.send(Message::Command(CommandId::MasterDocument));
        }
        for (dark, name) in [(false, "light"), (true, "dark")] {
            let theme = if dark {
                cosmic::Theme::dark()
            } else {
                cosmic::Theme::light()
            };
            let size = (1_280, 800);
            let rgb = render_window(&runtime.app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!("window-master-{which}-{name}.ppm"));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    }
}

/// User, 2026-11-03: a suggestion chosen from the right-click menu
/// replaced nothing. The right press asked for the menu's word before it
/// moved the caret, so the word was the one at the old caret. Through the
/// whole window, as the desktop delivers it: a right press on the word, its
/// menu, and a click on the suggestion.
#[test]
fn a_suggestion_chosen_by_right_click_replaces_the_word_under_the_pointer() {
    use cosmic::iced::mouse::Button;
    let mut runtime = Runtime::new(proofing_app("We sat on teh mat."));
    drop(runtime.app.check_as_you_type());
    runtime.redraw();
    let main = runtime.app.document.main_flow_id();
    let marks = runtime
        .app
        .spell_marks
        .get(&(main, 0))
        .cloned()
        .unwrap_or_default();
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0], 10..13);
    // The caret is at the start, in "We", away from the word.
    let point = cosmic::iced::Point::new(395.0, 331.0);
    runtime.click(Button::Right, point);
    let word = runtime.app.context_word.as_ref().expect("the menu's word");
    assert_eq!((word.word.as_str(), word.flagged), ("teh", true));
    assert_eq!(word.suggestions.first().map(String::as_str), Some("the"));
    let undo = runtime.app.history.undo_len();
    runtime.click(Button::Left, menu_row(point, 0, 0));
    assert_eq!(
        runtime.app.document.paragraphs()[0].text,
        "We sat on the mat."
    );
    assert_eq!(runtime.app.history.undo_len(), undo + 1);
}

fn merge_scratch(name: &str) -> std::path::PathBuf {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/merge-tests")
        .join(name);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

/// A letter whose `[recfile]` names a CSV data file of two records.
fn merge_letter(folder: &std::path::Path) -> Document {
    let data = folder.join("clients.csv");
    std::fs::write(&data, "NAME,CITY\nAnn,Boston\nBob,Dallas\n").unwrap();
    let document = Document::from_plain_text("Dear \nYours.");
    let settings = amipro_document::merge::MergeSettings {
        record_file: Some(amipro_document::merge::RecordFile::new(
            data.to_string_lossy(),
        )),
        ..Default::default()
    };
    amipro_editing::apply(
        &document,
        &amipro_editing::merge::set_merge_command(settings),
    )
    .unwrap()
    .document
}

fn first_paragraph(app: &AmiProApp) -> String {
    app.document.main_flow().paragraphs[0].text.clone()
}

#[test]
fn the_merge_panel_inserts_fields_previews_records_and_merges_mg1_mg5_mg6() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("panel");
    let mut runtime = Runtime::new(page_app(merge_letter(&folder)));
    runtime.redraw();
    runtime.send(Message::Command(CommandId::Merge));
    let panel = runtime.app.merge_panel.as_ref().expect("the panel opens");
    assert_eq!(
        panel.data.as_ref().unwrap().table.records.len(),
        2,
        "read at once"
    );
    assert_eq!(panel.keys, ["NAME", "CITY"]);
    // A field goes in at the caret as Ami Pro's power field, one step.
    runtime.send(Message::Page(PageAction::MoveTo {
        position: amipro_editing::TextPosition {
            flow: runtime.app.document.main_flow_id(),
            paragraph: 0,
            byte: 5,
        },
        page: 1,
        extend: false,
    }));
    let steps = runtime.app.history.undo_len();
    runtime.send(Message::Merge(A::Insert("NAME".into())));
    assert_eq!(runtime.app.history.undo_len(), steps + 1);
    assert_eq!(first_paragraph(&runtime.app), "Dear <NAME>");
    let written = String::from_utf8_lossy(&amipro_formats::sam_document(
        &runtime.app.document,
        &amipro_formats::SamOptions::at(1_793_000_000),
    ))
    .into_owned();
    assert!(written.contains("<:X12,"), "{written}");
    assert!(
        written.contains(";MergeField NAME><<NAME<;><:X~12,"),
        "{written}"
    );
    // The preview shows each record's values, changing nothing.
    let steps = runtime.app.history.undo_len();
    runtime.send(Message::Merge(A::Preview(true)));
    assert_eq!(first_paragraph(&runtime.app), "Dear Ann");
    runtime.send(Message::Merge(A::Next));
    assert_eq!(first_paragraph(&runtime.app), "Dear Bob");
    assert_eq!(runtime.app.history.undo_len(), steps);
    // An edit turns the preview off and is made in the letter, at the
    // caret's place there (after the field; MC3 changed after the user's
    // second use), one undo step.
    runtime.send(Message::Page(PageAction::Insert("x".into())));
    assert!(runtime.app.merge_preview.is_none());
    assert_eq!(first_paragraph(&runtime.app), "Dear <NAME>x");
    assert_eq!(runtime.app.history.undo_len(), steps + 1);
    runtime.send(Message::Command(CommandId::Undo));
    assert_eq!(first_paragraph(&runtime.app), "Dear <NAME>");
    // A condition: one step to add, one to give it a value; the count.
    runtime.send(Message::Merge(A::AddCondition));
    runtime.send(Message::Merge(A::ConditionValue(0, "Bob".into())));
    runtime.send(Message::Merge(A::CommitValue(0)));
    assert_eq!(runtime.app.history.undo_len(), steps + 2);
    assert_eq!(runtime.app.merge_records(), [1]);
    // Merge to New Document: the chosen record in a new tab.
    let sessions = runtime.app.sessions.len();
    runtime.send(Message::Merge(A::ToNewDocument));
    assert_eq!(runtime.app.sessions.len(), sessions + 1);
    assert_eq!(first_paragraph(&runtime.app), "Dear Bob");
    // Back in the letter, Undo takes the condition's value away.
    runtime.app.active_session = 0;
    runtime.send(Message::Command(CommandId::Undo));
    assert_eq!(
        runtime.app.merge_records(),
        Vec::<usize>::new(),
        "Name = \"\" matches none"
    );
    // Saved as SAM, the conditions are listed and the record file kept.
    let losses = amipro_formats::sam_losses(
        &runtime.app.document,
        &amipro_formats::SamOptions::at(1_793_000_000),
    )
    .unwrap();
    assert!(
        losses
            .iter()
            .any(|loss| loss.kind == amipro_formats::LossKind::MergeConditions),
        "{losses:?}"
    );
}

#[test]
fn merged_to_an_ami_pro_file_every_record_follows_a_page_break_mg4() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("file");
    let mut runtime = Runtime::new(page_app(merge_letter(&folder)));
    runtime.send(Message::Command(CommandId::Merge));
    runtime.send(Message::Page(PageAction::MoveTo {
        position: amipro_editing::TextPosition {
            flow: runtime.app.document.main_flow_id(),
            paragraph: 0,
            byte: 5,
        },
        page: 1,
        extend: false,
    }));
    runtime.send(Message::Merge(A::Insert("NAME".into())));
    let target = folder.join("MERGED.SAM");
    runtime.send(Message::Merge(A::ToAmiProFileChosen(target.clone())));
    let merged = amipro_formats::import_sam_path(&target).unwrap().document;
    let texts: Vec<String> = merged
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect();
    assert_eq!(texts[0], "Dear Ann");
    assert!(texts.iter().any(|text| text == "Dear Bob"), "{texts:?}");
    // Never over a file (D3).
    runtime.send(Message::Merge(A::ToAmiProFileChosen(target)));
    assert!(
        runtime
            .app
            .merge_panel
            .as_ref()
            .and_then(|panel| panel.message.clone())
            .is_some_and(|message| !message.starts_with("Merged")),
    );
}

#[test]
fn the_merge_panels_buttons_take_clicks() {
    let folder = merge_scratch("clicks");
    let mut runtime = Runtime::new(page_app(merge_letter(&folder)));
    runtime.redraw();
    runtime.send(Message::Command(CommandId::Merge));
    runtime.send(Message::Merge(
        crate::merge_panel::MergeAction::AddCondition,
    ));
    // A condition one record meets, so that Merge to is ready.
    runtime.send(Message::Merge(
        crate::merge_panel::MergeAction::ConditionValue(0, "Ann".into()),
    ));
    runtime.send(Message::Merge(
        crate::merge_panel::MergeAction::CommitValue(0),
    ));
    runtime.redraw();
    let mut found = Vec::new();
    // From the bottom up: a dropdown clicked opens its list over what is
    // below it.
    let sweep = |runtime: &mut Runtime, found: &mut Vec<String>| {
        let mut y = Runtime::WINDOW.height - 40.0;
        while y > 100.0 {
            // Every button is wider than 20 and taller than 8 pixels.
            for x in (1..17).map(|step| step as f32 * 20.0) {
                for message in runtime.click_unrun(
                    cosmic::iced::mouse::Button::Left,
                    cosmic::iced::Point::new(x, y),
                ) {
                    if let Message::Merge(action) = message {
                        found.push(format!("{action:?}"));
                    }
                }
            }
            y -= 8.0;
        }
    };
    use crate::merge_panel::{MergeAction as A, Step};
    use cosmic::iced::mouse::{Event as Mouse, ScrollDelta};
    // Each step open in turn (MR2), Options open in the third; the rest
    // of a step scrolls into view.
    runtime.send(Message::Merge(A::Options(true)));
    for step in [Step::DataFile, Step::MergeDocument, Step::Merge] {
        runtime.send(Message::Merge(A::Step(step)));
        runtime.redraw();
        sweep(&mut runtime, &mut found);
        let rail = cosmic::iced::Point::new(150.0, 500.0);
        runtime.events(&[(
            cosmic::iced::Event::Mouse(Mouse::WheelScrolled {
                delta: ScrollDelta::Lines { x: 0.0, y: -200.0 },
            }),
            rail,
        )]);
        runtime.redraw();
        sweep(&mut runtime, &mut found);
        runtime.events(&[(
            cosmic::iced::Event::Mouse(Mouse::WheelScrolled {
                delta: ScrollDelta::Lines { x: 0.0, y: 200.0 },
            }),
            rail,
        )]);
    }
    for wanted in [
        "Open",
        "NewDataFile",
        "Insert(\"NAME\")",
        "InsertMergeRec",
        "InsertNextRec",
        "InsertSkip",
        "Preview(true)",
        "WithConditions(false)",
        "ConditionJoin(0, Or)",
        "RemoveCondition(0)",
        "AddCondition",
        "Labels(true)",
        "ToNewDocument",
        "ToPdf",
        "PdfEach(true)",
        "ToPrinter",
        "ViewAndPrint",
        "ToAmiProFile",
        "StartFrom(Letter)",
        "StartFrom(Envelope)",
        "Step(DataFile)",
        "Step(MergeDocument)",
        "Step(Merge)",
        "Options(false)",
    ] {
        assert!(
            found.iter().any(|action| action == wanted),
            "{wanted}: {found:?}"
        );
    }
}

/// Runs `message` through the app without running the task it returns:
/// for steps that would open the desktop's print or file dialogs.
fn update_unrun(runtime: &mut Runtime, message: Message) {
    drop(cosmic::Application::update(&mut runtime.app, message));
}

/// The merge letter with a `NAME` field at "Dear ".
fn merge_letter_with_field(folder: &std::path::Path) -> Runtime {
    let mut runtime = Runtime::new(page_app(merge_letter(folder)));
    runtime.redraw();
    runtime.send(Message::Command(CommandId::Merge));
    runtime.send(Message::Page(PageAction::MoveTo {
        position: amipro_editing::TextPosition {
            flow: runtime.app.document.main_flow_id(),
            paragraph: 0,
            byte: 5,
        },
        page: 1,
        extend: false,
    }));
    runtime.send(Message::Merge(crate::merge_panel::MergeAction::Insert(
        "NAME".into(),
    )));
    runtime
}

fn pdf_pages(path: &std::path::Path) -> usize {
    let bytes = std::fs::read(path).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    text.matches("/Type /Page\n").count()
        + text.matches("/Type /Page/").count()
        + text.matches("/Type /Page ").count()
        + text.matches("/Type/Page/").count()
        + text.matches("/Type/Page\n").count()
        + text.matches("/Type/Page ").count()
        + text.matches("/Type /Page>").count()
        + text.matches("/Type/Page>").count()
}

#[test]
fn fields_the_data_file_lacks_are_named_and_merge_empty_mgm8() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("unknown");
    let mut runtime = merge_letter_with_field(&folder);
    runtime.send(Message::Merge(A::Insert("TITLE".into())));
    let data = runtime
        .app
        .merge_panel
        .as_ref()
        .and_then(|panel| panel.data.clone())
        .unwrap();
    assert_eq!(
        amipro_editing::merge::unknown_fields(runtime.app.merge_document(), &data.table),
        ["TITLE"]
    );
    runtime.send(Message::Merge(A::ToNewDocument));
    assert_eq!(
        first_paragraph(&runtime.app),
        "Dear Ann",
        "{:?} {:?}",
        runtime
            .app
            .merge_panel
            .as_ref()
            .and_then(|panel| panel.message.clone()),
        runtime.app.status
    );
}

/// The data file's table (MR1): its caret cell as (row, column).
fn data_cell(app: &AmiProApp) -> Option<(usize, usize)> {
    crate::table_editing::caret_cell(&app.document, app.page_selection)
        .map(|(_, row, column)| (row, column))
}

fn type_in(runtime: &mut Runtime, text: &str) {
    runtime.send(Message::Page(PageAction::Insert(text.into())));
}

/// Types `records` into a new data file's table from its first record's
/// first cell, Tab between values (a new record from the last cell).
fn type_records(runtime: &mut Runtime, records: &[&[&str]]) {
    let count: usize = records.iter().map(|record| record.len()).sum();
    let mut typed = 0;
    for record in records {
        for value in *record {
            if !value.is_empty() {
                type_in(runtime, value);
            }
            typed += 1;
            if typed < count {
                runtime.send(Message::Page(PageAction::CellStep { forward: true }));
            }
        }
    }
}

/// MR1: New Data File… writes a table data file; its tab shows the table
/// with the caret in the first record; typing goes in the cell; Tab moves
/// on and adds a record from the last cell; Sort; each change one undo
/// step; the letter follows the records live; Save writes over the file.
#[test]
fn data_files_are_entered_as_ami_pro_tables_mr1() {
    use crate::merge_panel::MergeAction as A;
    use amipro_editing::merge::data::{DataTable, read_table_data};
    let folder = merge_scratch("records");
    let mut runtime = merge_letter_with_field(&folder);
    let letter = runtime.app.active_session;
    runtime.send(Message::Merge(A::NewDataFile));
    runtime.send(Message::Merge(A::NewDataNames("Name, City".into())));
    let path = folder.join("NEWDATA.SAM");
    runtime.send(Message::Merge(A::NewDataFileChosen(path.clone())));
    assert!(path.is_file(), "written as a new file");
    // Written as Ami Pro's table data file: the names and one empty record.
    let read = |path: &std::path::Path| {
        let document = amipro_formats::import_sam(&std::fs::read(path).unwrap(), None, "Data")
            .unwrap()
            .document;
        read_table_data(&document).unwrap()
    };
    assert_eq!(
        read(&path),
        DataTable {
            names: vec!["Name".into(), "City".into()],
            records: vec![vec![String::new(), String::new()]],
        }
    );
    // The data file's tab, its pages, the caret in the first record.
    let data_tab = runtime.app.active_session;
    assert_ne!(data_tab, letter);
    assert!(runtime.app.data_form.is_some());
    assert_eq!(data_cell(&runtime.app), Some((1, 0)));
    // Typing goes in the cell; Tab moves on; Tab in the last cell adds a
    // record; each one undo step.
    let steps = runtime.app.history.undo_len();
    type_in(&mut runtime, "Cy");
    runtime.send(Message::Page(PageAction::CellStep { forward: true }));
    assert_eq!(data_cell(&runtime.app), Some((1, 1)));
    type_in(&mut runtime, "Dallas");
    runtime.send(Message::Page(PageAction::CellStep { forward: true }));
    assert_eq!(data_cell(&runtime.app), Some((2, 0)));
    type_in(&mut runtime, "Al");
    assert_eq!(
        runtime.app.data_form.as_ref().unwrap().table.records,
        [vec!["Cy", "Dallas"], vec!["Al", ""]]
    );
    assert!(runtime.app.history.undo_len() > steps);
    // Sort Ascending by the caret's column: the heading row stays first.
    runtime.send(Message::Command(CommandId::SortAscending));
    let table = &runtime.app.data_form.as_ref().unwrap().table;
    assert_eq!(table.names, ["Name", "City"]);
    assert_eq!(table.records[0][0], "Al");
    // Undo goes back a step, and the records follow.
    runtime.send(Message::Command(CommandId::Undo));
    assert_eq!(
        runtime.app.data_form.as_ref().unwrap().table.records[0][0],
        "Cy"
    );
    // The letter's panel follows the open data file's records live.
    let panel = runtime.app.sessions[letter].merge_panel.as_ref().unwrap();
    assert_eq!(panel.data.as_ref().unwrap().table.records.len(), 2);
    // Save writes over the data file, keeping the original the first time
    // (MC6), and it reads back as the table shows.
    let original = std::fs::read(&path).unwrap();
    assert_eq!(
        runtime.app.location,
        crate::DocumentLocation::Sam(path.clone())
    );
    runtime.send(Message::Command(CommandId::Save));
    assert!(!runtime.app.dirty, "{}", runtime.app.status);
    assert_eq!(
        std::fs::read(folder.join("NEWDATA.SAM.bak")).unwrap(),
        original
    );
    assert_eq!(read(&path), runtime.app.data_form.as_ref().unwrap().table);
    // Merging the letter takes the records.
    runtime.app.active_session = letter;
    runtime.send(Message::Merge(A::ToNewDocument));
    assert_eq!(first_paragraph(&runtime.app), "Dear Cy");
}

/// New Data File… onto an existing file the chooser confirmed (the user's
/// first use, 2026-11-04: nothing seemed to happen) writes over it, keeping
/// the original as NAME.SAM.bak, and opens the new table, also when a tab
/// already shows the old file; a tab with unsaved changes is refused, in
/// the panel and the status bar.
#[test]
fn a_new_data_file_writes_over_a_confirmed_file_with_a_backup() {
    use crate::merge_panel::MergeAction as A;
    use amipro_editing::merge::data::{Delimiters, new_data_document};
    let folder = merge_scratch("replace-data");
    let path = folder.join("OLDDATA.SAM");
    let old = new_data_document(&["Old".into()], Delimiters::default());
    std::fs::write(
        &path,
        amipro_formats::sam_document(&old, &amipro_formats::SamOptions::at(0)),
    )
    .unwrap();
    let original = std::fs::read(&path).unwrap();
    let mut runtime = merge_letter_with_field(&folder);
    let letter = runtime.app.active_session;
    // The old file open in a tab with unsaved changes: refused.
    runtime.app.edit_records(&path);
    let old_tab = runtime.app.active_session;
    runtime.app.sessions[old_tab].dirty = true;
    runtime.app.active_session = letter;
    runtime.send(Message::Merge(A::NewDataFile));
    runtime.send(Message::Merge(A::NewDataNames("Name, City".into())));
    runtime.send(Message::Merge(A::NewDataFileChosen(path.clone())));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(
        runtime.app.status.contains("unsaved changes"),
        "{}",
        runtime.app.status
    );
    let panel = runtime.app.sessions[letter].merge_panel.as_ref().unwrap();
    assert!(panel.message.as_ref().unwrap().contains("unsaved changes"));
    // Clean, it is written over and its tab shows the new table.
    runtime.app.sessions[old_tab].dirty = false;
    runtime.send(Message::Merge(A::NewDataFileChosen(path.clone())));
    assert_eq!(
        std::fs::read(folder.join("OLDDATA.SAM.bak")).unwrap(),
        original
    );
    assert_eq!(runtime.app.active_session, old_tab);
    assert_eq!(runtime.app.sessions.len(), 2);
    let form = runtime.app.data_form.as_ref().unwrap();
    assert_eq!(form.table.names, ["Name", "City"]);
    assert_eq!(form.delimiters, None);
    assert_eq!(data_cell(&runtime.app), Some((1, 0)));
    assert!(
        runtime.app.status.contains("OLDDATA.SAM.bak"),
        "{}",
        runtime.app.status
    );
    type_in(&mut runtime, "Ann");
    assert_eq!(
        runtime.app.data_form.as_ref().unwrap().table.records,
        [vec!["Ann", ""]]
    );
    let panel = runtime.app.sessions[letter].merge_panel.as_ref().unwrap();
    assert!(panel.message.is_none());
    assert_eq!(panel.data.as_ref().unwrap().table.names, ["Name", "City"]);
    // The merge document itself is never the data file.
    runtime.app.active_session = letter;
    let letter_path = folder.join("LETTER.SAM");
    runtime.app.sessions[letter].location = crate::DocumentLocation::Sam(letter_path.clone());
    runtime.send(Message::Merge(A::NewDataFile));
    runtime.send(Message::Merge(A::NewDataNames("Name".into())));
    runtime.send(Message::Merge(A::NewDataFileChosen(letter_path.clone())));
    assert!(!letter_path.exists());
    assert!(
        runtime.app.status.contains("merge document"),
        "{}",
        runtime.app.status
    );
}

/// MR1: Ami Pro's delimited data file is shown as the same table and Save
/// writes it back in its own form; a value holding a delimiter stops the
/// save with a dialog.
#[test]
fn delimited_data_files_are_edited_as_tables_and_saved_in_their_form_mr1() {
    use amipro_editing::merge::data::{Delimiters, new_data_document, read_ami_data};
    let folder = merge_scratch("delimited");
    let path = folder.join("CLIENTS.SAM");
    let mut document = new_data_document(&["Name".into(), "City".into()], Delimiters::default());
    let (mut table, delimiters) = read_ami_data(&document).unwrap();
    table.insert_record(0);
    table.set_value(0, 0, "Ann");
    table.set_value(0, 1, "Boston");
    document = amipro_editing::merge::data::data_document(&table, delimiters);
    std::fs::write(
        &path,
        amipro_formats::sam_document(&document, &amipro_formats::SamOptions::at(0)),
    )
    .unwrap();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("")));
    runtime.app.edit_records(&path);
    let form = runtime.app.data_form.as_ref().unwrap();
    assert_eq!(form.delimiters, Some(delimiters));
    assert_eq!(form.table, table);
    assert_eq!(data_cell(&runtime.app), Some((1, 0)));
    // Typed over in the cell, saved back as Ami Pro's delimited file.
    runtime.send(Message::Page(PageAction::Insert("Dr. ".into())));
    runtime.send(Message::Command(CommandId::Save));
    let saved = amipro_formats::import_sam(&std::fs::read(&path).unwrap(), None, "Data")
        .unwrap()
        .document;
    let (back, kept) = read_ami_data(&saved).unwrap();
    assert_eq!(kept, delimiters);
    assert_eq!(back.records[0], ["Dr. Ann", "Boston"]);
    assert!(folder.join("CLIENTS.SAM.bak").is_file());
    // A value holding the field delimiter cannot be written that way.
    runtime.send(Message::Page(PageAction::Insert("~".into())));
    let before = std::fs::read(&path).unwrap();
    runtime.send(Message::Command(CommandId::Save));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let problem = runtime.app.save_problem.clone().expect("a dialog says why");
    assert!(problem.body.contains("Record 1's Name"), "{}", problem.body);
}

#[test]
fn merged_labels_go_on_sheets_from_the_start_label_mg7() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("labels");
    let mut runtime = merge_letter_with_field(&folder);
    let steps = runtime.app.history.undo_len();
    runtime.send(Message::Merge(A::Labels(true)));
    assert_eq!(runtime.app.history.undo_len(), steps + 1, "one step");
    let sheet = amipro_editing::merge::labels::LabelSheet::of(&runtime.app.document).unwrap();
    assert_eq!(sheet, amipro_editing::merge::labels::SHEETS[0]);
    let page = &runtime.app.document.layouts()[0].right;
    assert_eq!((page.width, page.height), (3_960, 1_440));
    // As labels dims Ami Pro's save choice; View & Print too.
    // Two records, three copies each, from label 29: two sheets.
    runtime.send(Message::Merge(A::LabelCopies("3".into())));
    runtime.send(Message::Merge(A::LabelStart(29)));
    let job = runtime.app.merged_job_for_tests().unwrap();
    let (sheet, run) = job.labels.clone().unwrap();
    assert_eq!(run.len(), 28 + 6);
    assert_eq!(job.pages(), 2);
    let path = folder.join("labels.pdf");
    runtime.send(Message::Merge(A::ToPdfChosen(path.clone())));
    assert_eq!(pdf_pages(&path), 2, "two sheets");
    assert_eq!(sheet.per_sheet(), 30);
    // New Document: the whole sheets, each label at its place (MC8).
    let sessions = runtime.app.sessions.len();
    runtime.send(Message::Merge(A::ToNewDocument));
    assert_eq!(runtime.app.sessions.len(), sessions + 1);
    let labels = runtime
        .app
        .document
        .frames()
        .filter(|frame| matches!(frame.placement, amipro_document::Placement::Fixed { .. }))
        .count();
    assert_eq!(labels, 6);
    assert_eq!(runtime.app.pages.layout().pages.len(), 2);
    runtime.app.active_session = 0;
    // Another sheet from the list, one step; off again, one step.
    runtime.send(Message::Merge(A::LabelSheet(1)));
    assert_eq!(runtime.app.history.undo_len(), steps + 2);
    runtime.send(Message::Merge(A::Labels(false)));
    assert!(amipro_editing::merge::labels::LabelSheet::of(&runtime.app.document).is_none());
}

#[test]
fn view_and_print_shows_each_record_in_place_and_carries_edits_mg4() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("viewing");
    let mut runtime = merge_letter_with_field(&folder);
    runtime.send(Message::Merge(A::ViewAndPrint));
    assert_eq!(first_paragraph(&runtime.app), "Dear Ann");
    // An edit while viewing is made (Ami Pro's one document).
    let main = runtime.app.document.main_flow_id();
    runtime.send(Message::Page(PageAction::MoveTo {
        position: amipro_editing::TextPosition {
            flow: main,
            paragraph: 1,
            byte: 0,
        },
        page: 1,
        extend: false,
    }));
    runtime.send(Message::Page(PageAction::Insert("Kind ".into())));
    assert_eq!(
        runtime.app.document.main_flow().paragraphs[1].text,
        "Kind Yours."
    );
    // Undo and Redo while viewing (the hidden editor kept in step).
    runtime.send(Message::Command(CommandId::Undo));
    assert_eq!(
        runtime.app.document.main_flow().paragraphs[1].text,
        "Yours."
    );
    runtime.send(Message::Command(CommandId::Redo));
    assert_eq!(
        runtime.app.document.main_flow().paragraphs[1].text,
        "Kind Yours."
    );
    runtime.send(Message::Merge(A::PrintAndViewNext));
    assert_eq!(first_paragraph(&runtime.app), "Dear Bob");
    assert_eq!(
        runtime.app.document.main_flow().paragraphs[1].text,
        "Kind Yours."
    );
    // The last record: printing it ends the viewing and asks the printer
    // (not run here: it opens the desktop's dialog).
    update_unrun(&mut runtime, Message::Merge(A::PrintAndViewNext));
    assert!(runtime.app.merge_viewing.is_none());
    assert_eq!(runtime.app.viewing_job.as_ref().map(Vec::len), Some(2));
    assert_eq!(first_paragraph(&runtime.app), "Dear <NAME>", "fields again");
    let task = runtime.app.print_merged(&crate::print_portal::Prepared {
        pages: vec![0, 1],
        paper: None,
        token: 0,
    });
    assert!(task.is_some(), "the kept records go to the printer");
    drop(task);
    assert!(runtime.app.viewing_job.is_none());
}

#[test]
fn one_pdf_per_record_is_named_from_a_field_mg4() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("pdfs");
    let mut runtime = merge_letter_with_field(&folder);
    runtime.send(Message::Merge(A::PdfEach(true)));
    runtime.send(Message::Merge(A::PdfNameField(1)));
    let out = folder.join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("Boston.pdf"), b"older").unwrap();
    runtime.send(Message::Merge(A::ToPdfFolderChosen(out.clone())));
    assert!(out.join("Boston (2).pdf").is_file(), "never over a file");
    assert!(out.join("Dallas.pdf").is_file());
    assert_eq!(std::fs::read(out.join("Boston.pdf")).unwrap(), b"older");
    assert_eq!(crate::merge_commands::pdf_file_name("A/B: C", 3), "A_B_ C");
    assert_eq!(crate::merge_commands::pdf_file_name("  ", 3), "Record 3");
}

#[test]
fn merge_to_printer_asks_once_and_prints_the_chosen_pages_mg4() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("printer");
    let mut runtime = merge_letter_with_field(&folder);
    update_unrun(&mut runtime, Message::Merge(A::ToPrinter));
    assert!(runtime.app.merge_panel.as_ref().unwrap().printing);
    let task = runtime.app.print_merged(&crate::print_portal::Prepared {
        pages: vec![1],
        paper: None,
        token: 0,
    });
    assert!(task.is_some());
    drop(task);
    assert!(!runtime.app.merge_panel.as_ref().unwrap().printing);
}

#[test]
fn print_envelope_finds_the_address_and_makes_the_envelope_mg8() {
    use crate::envelope_panel::EnvelopeAction as E;
    let document = Document::from_plain_text(
        "November 3, 2026\n\nMs. Pam Beck\n41 West St.\nSeattle, WA 98101\n\nDear Ms. Beck:",
    );
    let mut runtime = Runtime::new(page_app(document));
    runtime.redraw();
    runtime.send(Message::Command(CommandId::PrintEnvelope));
    let panel = runtime
        .app
        .envelope_panel
        .as_ref()
        .expect("the panel opens");
    assert_eq!(
        panel.recipient,
        ["Ms. Pam Beck", "41 West St.", "Seattle, WA 98101"]
    );
    runtime.send(Message::Envelope(E::ReturnLine(0, "Terry Allen".into())));
    runtime.send(Message::Envelope(E::ReturnLine(1, "1 Main St.".into())));
    runtime.send(Message::Envelope(E::ReturnName("Home".into())));
    runtime.send(Message::Envelope(E::SaveReturn));
    assert_eq!(
        runtime.app.preferences.return_addresses,
        [("Home".to_string(), "Terry Allen\n1 Main St.".to_string())]
    );
    runtime.send(Message::Envelope(E::Size(6)));
    let sessions = runtime.app.sessions.len();
    runtime.send(Message::Envelope(E::NewDocument));
    assert_eq!(runtime.app.sessions.len(), sessions + 1);
    let page = &runtime.app.document.layouts()[0].right;
    assert_eq!((page.width, page.height), (12_472, 6_236), "DL");
    assert_eq!(first_paragraph(&runtime.app), "Terry Allen");
    runtime.app.active_session = 0;
    let path = merge_scratch("envelope").join("Envelope.pdf");
    runtime.send(Message::Envelope(E::PdfChosen(path.clone())));
    assert_eq!(pdf_pages(&path), 1);
    // Printed on letter paper fed at the centre: the page placed there.
    runtime.send(Message::Envelope(E::Feed(
        crate::envelope_panel::Feed::Centre,
    )));
    update_unrun(&mut runtime, Message::Envelope(E::Print));
    let task = runtime.app.print_envelope(&crate::print_portal::Prepared {
        pages: vec![0],
        paper: Some((12_240.0, 15_840.0)),
        token: 0,
    });
    assert!(task.is_some());
    drop(task);
}

#[test]
fn merged_envelopes_take_the_documents_data_file_mg8() {
    use crate::envelope_panel::EnvelopeAction as E;
    let folder = merge_scratch("envelopes");
    let mut runtime = merge_letter_with_field(&folder);
    runtime.send(Message::Command(CommandId::PrintEnvelope));
    runtime.send(Message::Envelope(E::Recipient(0, "<NAME>".into())));
    runtime.send(Message::Envelope(E::Recipient(1, "<CITY>".into())));
    runtime.send(Message::Envelope(E::Recipient(2, String::new())));
    runtime.send(Message::Envelope(E::MergeEnvelopes));
    let frames: Vec<String> = runtime
        .app
        .document
        .frames()
        .filter(|frame| frame.style.name.as_deref() == Some("Recipient"))
        .filter_map(|frame| match frame.content {
            amipro_document::FrameContent::Text(flow) => runtime.app.document.flow(flow),
            _ => None,
        })
        .map(|flow| flow.paragraphs[0].text.clone())
        .collect();
    assert_eq!(frames, ["Ann", "Bob"]);
}

#[test]
fn start_from_opens_the_merge_sheets_documents_mg11() {
    use crate::merge_panel::{MergeAction as A, StartFrom};
    let folder = merge_scratch("start");
    let mut runtime = Runtime::new(page_app(merge_letter(&folder)));
    runtime.send(Message::Command(CommandId::Merge));
    let sessions = runtime.app.sessions.len();
    runtime.send(Message::Merge(A::StartFrom(StartFrom::Labels)));
    assert_eq!(runtime.app.sessions.len(), sessions + 1);
    assert!(amipro_editing::merge::labels::LabelSheet::of(&runtime.app.document).is_some());
    assert!(
        runtime
            .app
            .merge_panel
            .as_ref()
            .is_some_and(|panel| panel.shown)
    );
    runtime.send(Message::Merge(A::StartFrom(StartFrom::Envelope)));
    assert!(runtime.app.envelope_panel.is_some());
}

/// The Merge panel (with a condition, labels on), the record editor, and
/// the Envelope panel, light and dark, to look at.
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn merge_screenshots() {
    use crate::merge_panel::MergeAction as A;
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let folder = merge_scratch("shots");
    let mut runtime = merge_letter_with_field(&folder);
    runtime.send(Message::Merge(A::Insert("TITLE".into())));
    runtime.send(Message::Merge(A::AddCondition));
    runtime.send(Message::Merge(A::Preview(true)));
    let shoot = |name: &'static str, runtime: &mut Runtime| {
        drop(runtime.app.update_title());
        for (dark, theme_name) in [(false, "light"), (true, "dark")] {
            let theme = if dark {
                cosmic::Theme::dark()
            } else {
                cosmic::Theme::light()
            };
            let size = (1_280, 1_500);
            let rgb = render_window(&runtime.app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!("window-{name}-{theme_name}.ppm"));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    };
    shoot("merge-panel", &mut runtime);
    runtime.send(Message::Merge(A::Preview(false)));
    runtime.send(Message::Merge(A::Step(crate::merge_panel::Step::DataFile)));
    shoot("merge-step-data", &mut runtime);
    runtime.send(Message::Merge(A::Step(crate::merge_panel::Step::Merge)));
    shoot("merge-step-merge", &mut runtime);
    runtime.send(Message::Merge(A::Options(true)));
    runtime.send(Message::Merge(A::Labels(true)));
    runtime.send(Message::Merge(A::LabelStart(5)));
    shoot("merge-labels", &mut runtime);
    runtime.send(Message::Merge(A::Labels(false)));
    runtime.send(Message::Merge(A::ViewAndPrint));
    shoot("merge-viewing", &mut runtime);
    runtime.send(Message::Merge(A::CancelViewing));
    let path = folder.join("SHOTDATA.SAM");
    runtime.send(Message::Merge(A::NewDataFile));
    runtime.send(Message::Merge(A::NewDataNames(
        "Title, First, Last, City".into(),
    )));
    runtime.send(Message::Merge(A::NewDataFileChosen(path)));
    type_records(
        &mut runtime,
        &[
            &["Mr.", "Terry", "Allen", "Boston"],
            &["Ms.", "Pam", "Beck", "Seattle"],
        ],
    );
    shoot("record-editor", &mut runtime);
    runtime.app.active_session = 0;
    runtime.send(Message::Command(CommandId::PrintEnvelope));
    runtime.send(Message::Envelope(
        crate::envelope_panel::EnvelopeAction::Recipient(0, "Ms. Pam Beck".into()),
    ));
    shoot("envelope", &mut runtime);
}

/// Release timings (ME.12): a 1,000-record merge to a new document and to
/// a PDF, the record editor's change on 1,000 records, and a keystroke
/// (refused, the preview going off) and a preview step on a letter.
#[test]
#[ignore = "release timing baseline"]
fn merge_performance_baseline() {
    use crate::merge_panel::MergeAction as A;
    let report = |name: &str, mut times: Vec<Duration>| {
        times.sort();
        eprintln!(
            "{name}: median {:.2} ms, worst {:.2} ms ({} runs)",
            times[times.len() / 2].as_secs_f64() * 1_000.0,
            times[times.len() - 1].as_secs_f64() * 1_000.0,
            times.len()
        );
    };
    let folder = merge_scratch("performance");
    let mut csv = String::from("NAME,CITY\n");
    for record in 0..1_000 {
        csv.push_str(&format!("Person {record},City {}\n", record % 37));
    }
    let mut runtime = merge_letter_with_field(&folder);
    std::fs::write(folder.join("clients.csv"), csv).unwrap();
    runtime.send(Message::Merge(A::Chosen(folder.join("clients.csv"))));
    assert_eq!(runtime.app.merge_records().len(), 1_000);
    runtime.send(Message::Merge(A::Insert("CITY".into())));
    let mut merging = Vec::new();
    for _ in 0..3 {
        let started = Instant::now();
        runtime.send(Message::Merge(A::ToNewDocument));
        merging.push(started.elapsed());
        runtime.app.active_session = 0;
    }
    report("1,000 records to a new document", merging);
    let mut pdf = Vec::new();
    for round in 0..2 {
        let path = folder.join(format!("merged-{round}.pdf"));
        let started = Instant::now();
        runtime.send(Message::Merge(A::ToPdfChosen(path)));
        pdf.push(started.elapsed());
    }
    report("1,000 records to one PDF", pdf);
    let mut steps = Vec::new();
    runtime.send(Message::Merge(A::Preview(true)));
    for _ in 0..50 {
        let started = Instant::now();
        runtime.send(Message::Merge(A::Next));
        runtime.redraw();
        steps.push(started.elapsed());
    }
    report("a preview step (merge, layout, frame)", steps);
    let mut keys = Vec::new();
    for _ in 0..20 {
        runtime.send(Message::Merge(A::Preview(true)));
        let started = Instant::now();
        runtime.send(Message::Page(PageAction::Insert("x".into())));
        runtime.redraw();
        keys.push(started.elapsed());
    }
    report("a keystroke with the preview on", keys);
    // A data file of 1,000 records as a table (MR1): opening it, and a
    // keystroke in a cell.
    let table = amipro_editing::merge::data::DataTable {
        names: vec!["Name".into(), "City".into()],
        records: (0..1_000)
            .map(|record| vec![format!("Person {record}"), format!("City {}", record % 37)])
            .collect(),
    };
    let data = amipro_editing::merge::data::data_document(&table, Default::default());
    let path = folder.join("BIGDATA.SAM");
    let _ = std::fs::remove_file(&path);
    std::fs::write(
        &path,
        amipro_formats::sam_document(&data, &amipro_formats::SamOptions::at(0)),
    )
    .unwrap();
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("")));
    let started = Instant::now();
    runtime.app.edit_records(&path);
    runtime.redraw();
    report(
        "1,000 records opened as a table (convert, layout, frame)",
        vec![started.elapsed()],
    );
    let mut keys = Vec::new();
    for _ in 0..20 {
        let started = Instant::now();
        runtime.send(Message::Page(PageAction::Insert("x".into())));
        runtime.redraw();
        keys.push(started.elapsed());
    }
    report("a keystroke in a 1,000-record table", keys);
}

/// The merge run's twins (ME.13): the app's data file (made in the record
/// editor), merge letter (fields from the Merge panel), and label document
/// written by Save As SAM into `AMIPRO_TWIN_OUT` for Ami Pro in the VM,
/// with the app's own merge of the letter (`AMM1.SAM`). With
/// `AMIPRO_VM_LAB` holding Ami Pro's merges of `AML1.SAM` (`AMT1.SAM`, and
/// `AMT2.SAM` from the table data file since MR1), the
/// two merges' text is compared.
#[test]
#[ignore = "writes the merge twins for the VM"]
fn merge_twins() {
    use crate::merge_panel::MergeAction as A;
    let Some(out) = std::env::var_os("AMIPRO_TWIN_OUT") else {
        eprintln!("AMIPRO_TWIN_OUT is not set; skipping");
        return;
    };
    let out = std::path::PathBuf::from(out);
    for name in ["AMD1.SAM", "AML1.SAM", "AMLB.SAM", "AMM1.SAM"] {
        let _ = std::fs::remove_file(out.join(name));
    }
    let options = amipro_formats::SamOptions::at(1_793_000_000);
    let write = |document: &Document, name: &str| {
        let bytes = amipro_formats::sam_document(document, &options);
        std::fs::write(out.join(name), bytes).unwrap();
        eprintln!("wrote {}", out.join(name).display());
    };
    // The data file: New Data File… and the record editor.
    let folder = merge_scratch("twins");
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("Dear ")));
    runtime.send(Message::Command(CommandId::Merge));
    runtime.send(Message::Merge(A::NewDataFile));
    runtime.send(Message::Merge(A::NewDataNames(
        "Title, First, Last, City, Amount".into(),
    )));
    let data_path = folder.join("AMD1.SAM");
    runtime.send(Message::Merge(A::NewDataFileChosen(data_path.clone())));
    let records = [
        ["Mr.", "Terry", "Allen", "Boston", "150"],
        ["Ms.", "Pam", "Beck", "Seattle", "75.50"],
        ["Dr.", "Joe", "Black", "", "$1,200.00"],
    ];
    let rows: Vec<&[&str]> = records.iter().map(|record| record.as_slice()).collect();
    type_records(&mut runtime, &rows);
    let data_tab = runtime.app.active_session;
    write(&runtime.app.sessions[data_tab].document, "AMD1.SAM");
    // The letter: fields clicked in turn in the Merge panel.
    runtime.app.active_session = 0;
    let main = runtime.app.document.main_flow_id();
    let place = |runtime: &mut Runtime, paragraph: usize| {
        let byte = runtime.app.document.main_flow().paragraphs[paragraph]
            .text
            .len();
        runtime.send(Message::Page(PageAction::MoveTo {
            position: amipro_editing::TextPosition {
                flow: main,
                paragraph,
                byte,
            },
            page: 1,
            extend: false,
        }));
    };
    place(&mut runtime, 0);
    runtime.send(Message::Merge(A::Insert("TITLE".into())));
    runtime.send(Message::Page(PageAction::Insert(" ".into())));
    runtime.send(Message::Merge(A::Insert("LAST".into())));
    runtime.send(Message::Page(PageAction::Insert(":\nFrom ".into())));
    runtime.send(Message::Merge(A::Insert("CITY".into())));
    runtime.send(Message::Page(PageAction::Insert(", record ".into())));
    runtime.send(Message::Merge(A::InsertMergeRec));
    runtime.send(Message::Page(PageAction::Insert(".\nEnd.".into())));
    // `[recfile]` as the VM sees the data file.
    let mut settings = runtime.app.document.merge().clone();
    settings.record_file.as_mut().unwrap().data_file = "l:\\in\\amd1.sam".into();
    runtime
        .app
        .execute_document_command(&amipro_editing::merge::set_merge_command(settings));
    write(&runtime.app.document, "AML1.SAM");
    runtime.send(Message::Merge(A::ToAmiProFileChosen(out.join("AMM1.SAM"))));
    let app_merge: Vec<String> = amipro_formats::import_sam_path(out.join("AMM1.SAM"))
        .unwrap()
        .document
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect();
    eprintln!("the app's merge: {app_merge:?}");
    // The label document: Avery 5160, the name on the label.
    let mut labels = Runtime::new(page_app(Document::from_plain_text("")));
    labels.send(Message::Command(CommandId::Merge));
    labels.send(Message::Merge(A::Chosen(data_path)));
    labels.send(Message::Merge(A::Insert("FIRST".into())));
    labels.send(Message::Page(PageAction::Insert(" ".into())));
    labels.send(Message::Merge(A::Insert("LAST".into())));
    labels.send(Message::Merge(A::Labels(true)));
    let mut settings = labels.app.document.merge().clone();
    settings.record_file.as_mut().unwrap().data_file = "l:\\in\\amd1.sam".into();
    labels
        .app
        .execute_document_command(&amipro_editing::merge::set_merge_command(settings));
    write(&labels.app.document, "AMLB.SAM");
    // Ami Pro's merge of the app's letter, when it is there.
    // AMT1 from the delimited data file AMD1 was (ME.13), AMT2 from the
    // table data file it is since MR1.
    for name in ["AMT1.SAM", "AMT2.SAM"] {
        let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
            break;
        };
        let theirs = std::path::PathBuf::from(lab).join(name);
        if let Ok(import) = amipro_formats::import_sam_path(&theirs) {
            let text: Vec<String> = import
                .document
                .main_flow()
                .paragraphs
                .iter()
                .map(|paragraph| paragraph.text.clone())
                .collect();
            let shown = |texts: &[String]| -> Vec<String> {
                texts
                    .iter()
                    .filter(|text| !text.trim().is_empty())
                    .map(|text| text.replace('\u{FFFC}', ""))
                    .filter(|text| !text.trim().is_empty())
                    .collect()
            };
            assert_eq!(
                shown(&app_merge),
                shown(&text),
                "Ami Pro's merge of AML1 ({name})"
            );
        } else {
            eprintln!("{} not there yet", theirs.display());
        }
    }
}

/// The user's second use of merge (2026-11-04): a data file of First to
/// Zip, the address block typed with "Preview a record" checked between
/// steps and the caret placed by a click in the preview each time. The
/// letter keeps its six fields in place, nothing inside another; typing in
/// the preview is made in the letter; the preview shows the record; the
/// fields the data file has give no warning.
fn address_block_runtime(folder: &std::path::Path) -> (Runtime, usize) {
    use crate::merge_panel::MergeAction as A;
    let mut runtime = Runtime::new(page_app(Document::from_plain_text("")));
    runtime.redraw();
    let letter = runtime.app.active_session;
    runtime.send(Message::Command(CommandId::Merge));
    runtime.send(Message::Merge(A::NewDataFile));
    runtime.send(Message::Merge(A::NewDataNames(
        "First, Last, Address, City, State, Zip".into(),
    )));
    runtime.send(Message::Merge(A::NewDataFileChosen(
        folder.join("DATA.SAM"),
    )));
    for (i, value) in ["Charlie", "Day", "1", "Hickory", "NC", "28602"]
        .iter()
        .enumerate()
    {
        if i > 0 {
            runtime.send(Message::Page(PageAction::CellStep { forward: true }));
        }
        type_in(&mut runtime, value);
    }
    runtime.app.active_session = letter;
    runtime.redraw();
    (runtime, letter)
}

fn main_texts(app: &AmiProApp) -> Vec<String> {
    app.document
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect()
}

fn power_marks(app: &AmiProApp) -> usize {
    app.document
        .main_flow()
        .paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.marks)
        .filter(|mark| amipro_document::PowerMark::parse(&mark.raw).is_some())
        .count()
}

#[test]
fn an_address_block_typed_around_the_preview_stays_whole() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("address-block");
    let (mut runtime, _) = address_block_runtime(&folder);
    let flow = runtime.app.document.main_flow_id();
    let steps: &[&str] = &[
        "F:First",
        " ",
        "F:Last",
        "\n",
        "F:Address",
        "\n",
        "F:City",
        " ",
        "F:State",
        " ",
        "F:Zip",
    ];
    for step in steps {
        // Preview on, a click at the end of the last line as shown.
        runtime.send(Message::Merge(A::Preview(true)));
        let paragraphs = &runtime.app.document.main_flow().paragraphs;
        let at = TextPosition {
            flow,
            paragraph: paragraphs.len() - 1,
            byte: paragraphs.last().unwrap().text.len(),
        };
        runtime.send(Message::Page(PageAction::MoveTo {
            position: at,
            page: 1,
            extend: false,
        }));
        if let Some(name) = step.strip_prefix("F:") {
            runtime.send(Message::Merge(A::Insert(name.into())));
            assert!(
                !runtime.app.status.contains("not a valid field name"),
                "{}",
                runtime.app.status
            );
        } else if *step == "\n" {
            runtime.send(Message::Page(PageAction::ParagraphBreak));
        } else {
            type_in(&mut runtime, step);
        }
    }
    assert_eq!(
        main_texts(&runtime.app),
        ["<First> <Last>", "<Address>", "<City> <State> <Zip>"]
    );
    assert_eq!(power_marks(&runtime.app), 12, "six fields, none nested");
    runtime.send(Message::Merge(A::Preview(true)));
    assert_eq!(
        main_texts(&runtime.app),
        ["Charlie Day", "1", "Hickory NC 28602"]
    );
    // A click in a value, then a field: it goes after that field.
    runtime.send(Message::Page(PageAction::MoveTo {
        position: TextPosition {
            flow,
            paragraph: 0,
            byte: 3,
        },
        page: 1,
        extend: false,
    }));
    runtime.send(Message::Merge(A::Insert("Zip".into())));
    assert_eq!(main_texts(&runtime.app)[0], "<First><Zip> <Last>");
    runtime.send(Message::Command(CommandId::Undo));
    assert_eq!(main_texts(&runtime.app)[0], "<First> <Last>");
}

/// Backspace after a field, or Delete before one, takes it whole; empty
/// fields (they show nothing) first, as the user's saved letter had after
/// Zip; a selection holding fields takes them too.
#[test]
fn backspace_and_delete_take_whole_fields_and_leave_none_invisible() {
    use crate::merge_panel::MergeAction as A;
    let folder = merge_scratch("field-keys");
    let (mut runtime, _) = address_block_runtime(&folder);
    for name in ["City", "Zip"] {
        runtime.send(Message::Merge(A::Insert(name.into())));
    }
    assert_eq!(main_texts(&runtime.app), ["<City><Zip>"]);
    // The user's letter: two empty fields after Zip.
    let mut document = runtime.app.document.clone();
    {
        let paragraph = document.paragraph_mut(0);
        let end = paragraph.text.len();
        for name in ["LAST", "FIRST"] {
            for mark in [
                format!("<:X12,-32768;MergeField {name}>"),
                format!("<:X~12,-32768;MergeField {name}>"),
            ] {
                paragraph.marks.push(amipro_document::LegacyMark {
                    byte: end,
                    raw: mark,
                });
            }
        }
    }
    runtime.app.document = document;
    runtime.send(Message::Merge(A::Preview(true)));
    assert_eq!(main_texts(&runtime.app), ["Hickory28602DayCharlie"]);
    runtime.send(Message::Merge(A::Preview(false)));
    let end = TextPosition {
        flow: runtime.app.document.main_flow_id(),
        paragraph: 0,
        byte: 11,
    };
    runtime.send(Message::Page(PageAction::MoveTo {
        position: end,
        page: 1,
        extend: false,
    }));
    // The empty ones first, then Zip whole, each one undo step.
    runtime.send(Message::Page(PageAction::Backspace));
    assert_eq!(main_texts(&runtime.app), ["<City><Zip>"]);
    assert_eq!(power_marks(&runtime.app), 4);
    runtime.send(Message::Merge(A::Preview(true)));
    assert_eq!(main_texts(&runtime.app), ["Hickory28602"]);
    runtime.send(Message::Page(PageAction::Backspace));
    assert_eq!(main_texts(&runtime.app), ["<City>"]);
    assert_eq!(power_marks(&runtime.app), 2);
    runtime.send(Message::Command(CommandId::Undo));
    assert_eq!(main_texts(&runtime.app), ["<City><Zip>"]);
    // Delete before a field takes it whole.
    runtime.send(Message::Page(PageAction::MoveTo {
        position: TextPosition { byte: 0, ..end },
        page: 1,
        extend: false,
    }));
    runtime.send(Message::Page(PageAction::Delete));
    assert_eq!(main_texts(&runtime.app), ["<Zip>"]);
    assert_eq!(power_marks(&runtime.app), 2);
    // A selection holding the field, typed over: no marks left.
    runtime.send(Message::Page(PageAction::SelectAll));
    type_in(&mut runtime, "x");
    assert_eq!(main_texts(&runtime.app), ["x"]);
    assert_eq!(power_marks(&runtime.app), 0);
}

/// Whole tasks as a user does them, with one checker after every step.
#[path = "scenario_tests/mod.rs"]
mod scenarios;

/// Line Numbering (CA.13): Number text lines in the margin, with Page
/// Layout Properties open at its section, light and dark.
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn line_numbering_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    send(&mut app, Message::Command(CommandId::LineNumbering));
    send(
        &mut app,
        Message::PageLayoutProperties(crate::page_layout_properties::LayoutAction::NumberingMode(
            2,
        )),
    );
    assert!(app.document.line_numbering().is_on());
    for (dark, name) in [(false, "light"), (true, "dark")] {
        let theme = if dark {
            cosmic::Theme::dark()
        } else {
            cosmic::Theme::light()
        };
        let size = (1_280, 800);
        let rgb = render_window(&app, size, &theme, false);
        let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
        ppm.extend(rgb);
        let path = directory.join(format!("window-line-numbers-{name}.ppm"));
        std::fs::write(&path, ppm).unwrap();
        eprintln!("wrote {}", path.display());
    }
}

/// The password box (CA.14): File › Password… with a refused verify, and
/// File › Open's prompt with Ami Pro's wrong-password message, light and
/// dark.
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn password_dialog_screenshots() {
    use crate::password_dialog::{PasswordAction, PasswordDialog};
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(page_fixture());
    drop(app.update_title());
    send(&mut app, Message::Command(CommandId::Password));
    send(
        &mut app,
        Message::Password(PasswordAction::Password("Lotus1".into())),
    );
    send(
        &mut app,
        Message::Password(PasswordAction::Verify("lotus".into())),
    );
    send(&mut app, Message::Password(PasswordAction::Submit));
    assert!(app.password_dialog.as_ref().unwrap().error.is_some());
    let mut shots = vec![("password-set", app)];
    let mut open = page_app(page_fixture());
    drop(open.update_title());
    let mut dialog = PasswordDialog::open("LETTER.SAM".into());
    dialog.password = "wrong".into();
    dialog.error = Some(amipro_formats::ImportError::WrongPassword.to_string());
    open.password_dialog = Some(dialog);
    shots.push(("password-open", open));
    for (name, app) in &shots {
        for (dark, theme_name) in [(false, "light"), (true, "dark")] {
            let theme = if dark {
                cosmic::Theme::dark()
            } else {
                cosmic::Theme::light()
            };
            let size = (1_280, 800);
            let rgb = render_window(app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!("window-{name}-{theme_name}.ppm"));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    }
}

/// CA.14's live check: with the page holding the keyboard, File ›
/// Password… opened its box and the typed password went into the page
/// behind it. A dialog over the window now takes the keys: the page is
/// unchanged, whatever dialog is open.
#[test]
fn typing_under_a_dialog_does_not_reach_the_page() {
    use cosmic::iced::keyboard::{Key, Modifiers};
    use cosmic::iced::mouse::Button;
    let mut app = page_app(Document::from_plain_text("Page text"));
    send(&mut app, Message::ZoomTo(100));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    let on_text = runtime.on_page(1_500.0, 1_560.0);
    runtime.click(Button::Left, on_text);
    runtime.events(&[key_press(Key::Character("a".into()), Modifiers::default())]);
    let typed = runtime.app.document.plain_text();
    assert_ne!(typed, "Page text", "the page had the keyboard");
    for open in [CommandId::Password, CommandId::SpecialCharacter] {
        runtime.send(Message::Command(open));
        assert!(
            cosmic::Application::dialog(&runtime.app).is_some(),
            "{open:?} opens a dialog"
        );
        for character in ["L", "i", "v", "e"] {
            runtime.events(&[key_press(
                Key::Character(character.into()),
                Modifiers::default(),
            )]);
        }
        assert_eq!(
            runtime.app.document.plain_text(),
            typed,
            "{open:?}: the page is untouched"
        );
        match open {
            CommandId::Password => runtime.send(Message::Password(
                crate::password_dialog::PasswordAction::Cancel,
            )),
            _ => runtime.send(Message::SpecialCharacter(
                crate::special_character::CharacterAction::Cancel,
            )),
        };
        assert!(cosmic::Application::dialog(&runtime.app).is_none());
    }
}

/// A document with Title (level 1) and Subhead (level 2) headings and body
/// text: Outline mode's fixture.
#[cfg(test)]
pub(crate) fn outline_fixture() -> Document {
    use amipro_document::{Alignment, StyleId};
    let mut document = Document::from_plain_text(
        "First title\nSubhead A\nBody under A, one, long enough to wrap onto a second line of the page in the outline view as well.\nBody under A, two.\nSubhead B\nBody under B.\nSecond title\nSubhead C\nBody under C.",
    );
    let body = document.body_text_style();
    for (id, name, level, size) in [
        (StyleId(951), "Title", 1, 18.0),
        (StyleId(952), "Subhead", 2, 12.0),
    ] {
        let mut style = document.styles()[&body].clone();
        style.id = id;
        style.name = name.into();
        style.bullets.level = level;
        style.font_size_points = size;
        style.bold = true;
        style.italic = level == 2;
        style.alignment = if level == 1 {
            Alignment::Centre
        } else {
            Alignment::Left
        };
        style.spacing.above = 144;
        document.styles_mut().insert(id, style);
    }
    for (index, style) in [(0, 951), (1, 952), (4, 952), (6, 951), (7, 952)] {
        document.paragraph_mut(index).style = StyleId(style);
    }
    document
}

/// Outline mode (OB.3), light and dark.
#[test]
#[ignore = "writes screenshots to AMIPRO_SCREENSHOT_DIR"]
fn outline_mode_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let mut app = page_app(outline_fixture());
    drop(app.update_title());
    send(&mut app, Message::Command(CommandId::OutlineMode));
    assert!(app.outline.is_some());
    let shoot = |app: &AmiProApp, shot: &str| {
        for (dark, name) in [(false, "light"), (true, "dark")] {
            let theme = if dark {
                cosmic::Theme::dark()
            } else {
                cosmic::Theme::light()
            };
            let size = (1_280, 800);
            let rgb = render_window(app, size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!("window-{shot}-{name}.ppm"));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    };
    shoot(&app, "outline");
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 2), 1, false);
    send(&mut app, Message::Command(CommandId::OutlineContract));
    shoot(&app, "outline-contracted");
    send(&mut app, Message::Command(CommandId::ShowLevel2));
    shoot(&app, "outline-levels-2");
    send(&mut app, Message::Command(CommandId::ShowAllLevels));
    send(&mut app, Message::Command(CommandId::OutlineNavigator));
    move_to(&mut app, at(main, 4, 2), 1, false);
    shoot(&app, "outline-navigator");
}

/// Outline mode (OB.3): a click on an indented line puts the caret in that
/// paragraph, typing goes there in the document, and Layout mode shows the
/// same text; the icons stand left of each paragraph's first line.
#[test]
fn outline_mode_lines_take_clicks_and_typing_where_they_show() {
    use cosmic::iced::keyboard::{Key, Modifiers};
    use cosmic::iced::mouse::Button;
    let mut app = page_app(outline_fixture());
    send(&mut app, Message::ZoomTo(100));
    send(&mut app, Message::Command(CommandId::OutlineMode));
    let mut runtime = Runtime::new(app);
    runtime.redraw();
    use crate::outline_view::OutlineIcon::*;
    assert_eq!(
        runtime.app.outline_icons,
        [Plus, Plus, Body, Body, Plus, Body, Plus, Plus, Body]
    );
    // "Body under B." (paragraph 5): its line in the outline layout.
    let line = runtime.app.pages.layout().pages[0]
        .lines
        .iter()
        .find(|line| line.container == amipro_layout::Container::Main && line.paragraph_index == 5)
        .cloned()
        .expect("the line shows");
    let text_x = line.words.first().unwrap().x;
    assert_eq!(
        text_x,
        crate::outline_view::MARGIN as f32 + 720.0,
        "half a step past Subhead"
    );
    let at = runtime.on_page(text_x + 40.0, line.top + line.height / 2.0);
    runtime.click(Button::Left, at);
    assert_eq!(runtime.app.page_selection.focus.paragraph, 5);
    runtime.events(&[key_press(Key::Character("X".into()), Modifiers::default())]);
    let text = &runtime.app.document.paragraphs()[5].text;
    assert!(text.starts_with('X') || text.contains('X'), "{text}");
    assert_eq!(runtime.app.document.paragraphs().len(), 9);
    // Layout mode again: the same document.
    runtime.send(Message::Command(CommandId::OutlineMode));
    assert!(runtime.app.outline.is_none());
    assert!(runtime.app.document.paragraphs()[5].text.contains('X'));
}

/// Use Outline Styles (OB.5): Outline mode shows each level in
/// `_OUTLINE.STY`'s font for every open document, the document keeping its
/// own styles; the menu check follows; off again, the paragraphs' own.
#[test]
fn use_outline_styles_changes_only_what_outline_mode_shows() {
    use amipro_document::StyleId;
    let mut app = page_app(outline_fixture());
    send(&mut app, Message::Command(CommandId::OutlineMode));
    let title_line = |app: &AmiProApp| {
        app.pages.layout().pages[0]
            .lines
            .iter()
            .find(|line| line.paragraph_index == 1)
            .map(|line| line.height)
            .unwrap()
    };
    let own = title_line(&app);
    send(&mut app, Message::Command(CommandId::UseOutlineStyles));
    assert!(app.preferences.outline_styles);
    assert_eq!(
        crate::menu_check(&app, CommandId::UseOutlineStyles),
        Some(true)
    );
    let copy = app.outline_copy.as_ref().expect("the outline copy");
    assert_eq!(copy.styles()[&StyleId(952)].font_size_points, 14.0);
    assert_eq!(app.document.styles()[&StyleId(952)].font_size_points, 12.0);
    assert!(title_line(&app) > own, "Subhead A is set in 14 pt");
    let saved = app.document.clone();
    send(&mut app, Message::Command(CommandId::UseOutlineStyles));
    assert!(!app.preferences.outline_styles);
    assert_eq!(title_line(&app), own);
    assert_eq!(app.document, saved, "the document never changed");
}

/// Found by run B's pre-handoff check: in Draft or Outline mode, Print and
/// Export PDF took the display pages. They take the document's own, and
/// the Print panel's preview returns the tab to Layout mode.
#[test]
fn print_and_pdf_take_the_documents_pages_in_draft_and_outline_mode() {
    let mut app = page_app(outline_fixture());
    let own = app.pdf_job(print::PdfOptions::default()).layout;
    let first_x = |layout: &amipro_layout::DocumentLayout| layout.pages[0].lines[0].x;
    for mode in [CommandId::OutlineMode, CommandId::DraftMode] {
        send(&mut app, Message::Command(mode));
        assert!(app.outline.is_some() || app.draft.is_some(), "{mode:?} on");
        assert_ne!(
            first_x(app.pages.layout()),
            first_x(&own),
            "{mode:?}: the display pages differ"
        );
        let job = app.pdf_job(print::PdfOptions::default());
        assert_eq!(job.layout.pages.len(), own.pages.len(), "{mode:?}");
        assert_eq!(
            first_x(&job.layout),
            first_x(&own),
            "{mode:?}: the document's own pages"
        );
        assert_eq!(job.layout.pages[0].width, own.pages[0].width, "{mode:?}");
        send(&mut app, Message::Command(mode));
    }
    send(&mut app, Message::Command(CommandId::OutlineMode));
    app.open_print_panel();
    assert!(
        app.outline.is_none() && app.draft.is_none(),
        "the preview shows the printed pages"
    );
    assert_eq!(first_x(app.pages.layout()), first_x(&own));
}

/// Outline mode has no pages, so the ruler goes with it and returns with
/// Layout mode (the live check: the ruler stood offset over the outline).
#[test]
fn the_ruler_is_hidden_in_outline_mode_only() {
    let mut app = page_app(outline_fixture());
    assert!(crate::ruler_shown(true, &app.sessions[app.active_session]));
    send(&mut app, Message::Command(CommandId::OutlineMode));
    assert!(!crate::ruler_shown(true, &app.sessions[app.active_session]));
    send(&mut app, Message::Command(CommandId::DraftMode));
    assert!(crate::ruler_shown(true, &app.sessions[app.active_session]));
    assert!(!crate::ruler_shown(
        false,
        &app.sessions[app.active_session]
    ));
}

/// What Esc must leave alone: the document, the tabs, their unsaved marks.
fn escape_snapshot(app: &AmiProApp) -> (String, usize, Vec<bool>) {
    (
        format!("{:?}", app.document),
        app.sessions.len(),
        app.sessions.iter().map(|session| session.dirty).collect(),
    )
}

/// Every dialog `dialog()` can return, opened as the app opens it.
type OpenDialog = fn(&mut AmiProApp);

fn escape_dialogs() -> Vec<(&'static str, OpenDialog)> {
    fn command(app: &mut AmiProApp, id: CommandId) {
        send(app, Message::Command(id));
    }
    vec![
        ("import options", |app| {
            app.ask_import(
                crate::import::Source::Clipboard(Vec::new()),
                amipro_exchange::import::SourceFormat::Rtf,
                crate::import::Purpose::Open,
            );
        }),
        ("field prompt", |app| {
            use crate::field_commands::{FieldPrompt, FieldQuestion};
            app.field_prompt = Some(FieldPrompt {
                scope: amipro_editing::fields::UpdateScope::All,
                question: FieldQuestion::Decide {
                    message: "Go on?".into(),
                },
                text: String::new(),
                queries: Default::default(),
                decisions: Default::default(),
            });
        }),
        ("discard unsaved changes", |app| {
            app.sessions[0].dirty = true;
            command(app, CommandId::CloseDocument);
        }),
        ("rename style", |app| {
            let style = *app.document.styles().keys().next().unwrap();
            app.rename_style = Some((style, "Other".into()));
        }),
        ("style management", |app| {
            command(app, CommandId::StyleManagement)
        }),
        ("use style sheet", |app| {
            command(app, CommandId::UseAnotherStyleSheet);
        }),
        ("create style", |app| command(app, CommandId::CreateStyle)),
        ("save style sheet", |app| {
            command(app, CommandId::SaveAsStyleSheet);
        }),
        ("floating header", |app| {
            command(app, CommandId::HeaderFooter)
        }),
        ("page number", |app| command(app, CommandId::PageNumber)),
        ("create frame", |app| command(app, CommandId::Frame)),
        ("insert table", |app| command(app, CommandId::Table)),
        ("insert bullet", |app| command(app, CommandId::InsertBullet)),
        ("outline numbering", |app| {
            command(app, CommandId::OutlineNumbering);
        }),
        ("go to", |app| command(app, CommandId::GoToPage)),
        ("delete frame", |app| {
            let frame = text_frame(&app.document);
            app.frame_selection = Some(FrameSelection::one(frame, 1));
            command(app, CommandId::Delete);
        }),
        ("special effects", |app| {
            command(app, CommandId::SpecialEffects)
        }),
        ("font", |app| command(app, CommandId::Font)),
        ("special character", |app| {
            command(app, CommandId::SpecialCharacter);
        }),
        ("revert to saved", |app| app.revert_pending = true),
        ("password", |app| command(app, CommandId::Password)),
        ("fast format ask", |app| {
            app.fast_format_ask = Some(crate::fast_format::FastFormatAsk { style: true });
        }),
        ("new document", |app| {
            command(app, CommandId::NewFromStyleSheet)
        }),
        ("preferences", |app| command(app, CommandId::Preferences)),
        ("toolbar editor", |app| {
            command(app, CommandId::CustomizeToolbar)
        }),
        ("save problem", |app| {
            app.save_problem = Some(SaveProblem {
                title: "Not saved".into(),
                body: "No.".into(),
                retry: SaveRetry::SaveAs,
            });
        }),
        ("save as Ami Pro with losses", |app| {
            app.sam_save = Some(PendingSamSave {
                path: std::env::temp_dir().join("escape-never-written.sam"),
                losses: vec!["something".into()],
                saved_at: 0,
                replace: false,
            });
        }),
        ("export with losses", |app| {
            let path = std::env::temp_dir().join("escape-never-written.docx");
            let mut prepared = prepared_export(app, amipro_exchange::Format::Docx, &path);
            prepared.losses = vec!["something".into()];
            app.export_pending = Some(prepared);
        }),
        ("replace existing file", |app| {
            app.overwrite_confirmation
                .begin(std::env::temp_dir().join("escape-never-written.sam"));
        }),
    ]
}

fn escape_app() -> AmiProApp {
    let mut app = page_app(page_fixture());
    app.password = None;
    app
}

#[test]
fn escape_cancels_each_dialog_and_applies_nothing() {
    let mut cancelled = 0;
    for (name, open) in escape_dialogs() {
        let mut app = escape_app();
        let before = escape_snapshot(&app);
        open(&mut app);
        assert!(
            cosmic::Application::dialog(&app).is_some(),
            "{name} did not open"
        );
        assert!(app.dialog_cancel().is_some(), "{name} has no Esc message");
        send(&mut app, Message::Escape);
        assert!(
            cosmic::Application::dialog(&app).is_none(),
            "Esc did not close {name}"
        );
        let mut after = escape_snapshot(&app);
        if name == "discard unsaved changes" {
            // The tab was marked unsaved to ask; it stays open and unsaved.
            after.2 = vec![false];
        }
        assert_eq!(before, after, "Esc changed something in {name}");
        assert!(app.password.is_none(), "{name} set a password");
        assert!(!app.revert_pending, "{name}");
        cancelled += 1;
    }
    assert_eq!(cancelled, escape_dialogs().len());
}

#[test]
fn escape_stacked_dialogs_cancel_one_per_press_topmost_first() {
    let mut app = escape_app();
    // Open lowest first: the overwrite question, then a save problem, then
    // Go To (the order `dialog` shows them in is the reverse).
    app.overwrite_confirmation
        .begin(std::env::temp_dir().join("escape-never-written.sam"));
    app.save_problem = Some(SaveProblem {
        title: "Not saved".into(),
        body: "No.".into(),
        retry: SaveRetry::SaveAs,
    });
    send(&mut app, Message::Command(CommandId::GoToPage));
    assert!(app.go_to.is_some());
    send(&mut app, Message::Escape);
    assert!(app.go_to.is_none());
    assert!(app.save_problem.is_some() && app.overwrite_confirmation.pending().is_some());
    send(&mut app, Message::Escape);
    assert!(app.save_problem.is_none());
    assert!(app.overwrite_confirmation.pending().is_some());
    send(&mut app, Message::Escape);
    assert!(app.overwrite_confirmation.pending().is_none());
    assert!(cosmic::Application::dialog(&app).is_none());
}

#[test]
fn escape_without_a_dialog_ends_clean_screen_then_closes_a_focused_find_bar() {
    use crate::find_bar::{Field, FindAction};
    let mut app = escape_app();
    send(&mut app, Message::Command(CommandId::Find));
    find(&mut app, FindAction::Focus(Field::Find, true));
    send(&mut app, Message::Command(CommandId::FullScreen));
    assert!(app.clean_screen);
    send(&mut app, Message::Escape);
    assert!(!app.clean_screen);
    assert!(app.find.is_some());
    send(&mut app, Message::Escape);
    assert!(app.find.is_none());
    // A find bar whose fields are not focused stays.
    send(&mut app, Message::Command(CommandId::Find));
    find(&mut app, FindAction::Focus(Field::Find, false));
    send(&mut app, Message::Escape);
    assert!(app.find.is_some());
}

#[test]
fn escape_with_a_dialog_over_the_find_bar_leaves_the_bar() {
    use crate::find_bar::{Field, FindAction};
    let mut app = escape_app();
    send(&mut app, Message::Command(CommandId::Find));
    find(&mut app, FindAction::Focus(Field::Find, true));
    send(&mut app, Message::Command(CommandId::GoToPage));
    send(&mut app, Message::Escape);
    assert!(app.go_to.is_none());
    assert!(app.find.is_some());
}

/// TD.12: typing in a table cell that 51 formulas read (`B1..B50 = A1*k`,
/// `C1 = @sum(B1..B50)`), and the step that leaves the cell (the number's
/// rewrite and `RecalculateTable`), against the same table without
/// formulas. `AMIPRO_PERF_TABLE=1`; release.
#[test]
#[ignore = "release-mode formula performance baseline"]
fn table_formula_performance_baseline() {
    use amipro_editing::tables::{NewTable, TableTarget};
    const SAMPLES: usize = 40;
    if std::env::var_os("AMIPRO_PERF_TABLE").is_none() {
        eprintln!("AMIPRO_PERF_TABLE is not set; skipping");
        return;
    }
    for with_formulas in [false, true] {
        let document = Document::from_plain_text("Prices");
        let start = at(document.main_flow_id(), 0, 6);
        let mut document = amipro_editing::apply(
            &document,
            &amipro_editing::Command::InsertTable {
                target: TableTarget::Text(start),
                table: NewTable::fitting(3, 52, 9_360, 284),
            },
        )
        .unwrap()
        .document;
        let frame = *document.frame_stack().last().unwrap();
        let flow = cell(&document, frame, 0, 0);
        let style = document.flow(flow).unwrap().paragraphs[0].style;
        {
            let format = &mut document
                .styles_mut()
                .get_mut(&style)
                .unwrap()
                .kept
                .number_format;
            format.kind = amipro_document::NumberKind::Fixed;
            format.flags = 0x18;
            format.decimals = 2;
        }
        let set = |document: Document, row: usize, column: usize, formula: Option<String>| {
            amipro_editing::apply(
                &document,
                &amipro_editing::Command::SetCellFormula {
                    frame,
                    row,
                    column,
                    formula,
                },
            )
            .unwrap()
            .document
        };
        let put = |document: Document, row: usize, text: &str| {
            let flow = cell(&document, frame, row, 0);
            amipro_editing::apply(
                &document,
                &amipro_editing::Command::ReplaceText {
                    start: at(flow, 0, 0),
                    end: at(flow, 0, 0),
                    text: text.into(),
                },
            )
            .unwrap()
            .document
        };
        for row in 0..52 {
            document = put(document, row, &format!("{}.25", row + 1));
        }
        if with_formulas {
            for row in 0..50 {
                document = set(document, row, 1, Some(format!("A1*{}", row + 2)));
            }
            document = set(document, 0, 2, Some("@sum(B1..B50)".into()));
        }
        let window = cosmic::iced::Size::new(1_280.0, 800.0);
        let mut app = page_app(document);
        let mut harness = FrameHarness::new(window);
        harness.frame(&app);
        let a1 = cell(&app.document, frame, 0, 0);
        let a2 = cell(&app.document, frame, 1, 0);
        let (mut typing, mut leaving) = (Vec::new(), Vec::new());
        for round in 0..SAMPLES {
            let length = app.document.flow(a1).unwrap().paragraphs[0].text.len();
            move_to(&mut app, at(a1, 0, 0), 1, false);
            move_to(&mut app, at(a1, 0, length), 1, true);
            harness.frame(&app);
            let started = Instant::now();
            page(&mut app, PageAction::Insert(format!("{}", 1 + round % 9)));
            harness.frame(&app);
            typing.push(started.elapsed());
            let started = Instant::now();
            move_to(&mut app, at(a2, 0, 0), 1, false);
            harness.frame(&app);
            leaving.push(started.elapsed());
        }
        let c1 = app
            .document
            .flow(cell(&app.document, frame, 0, 2))
            .unwrap()
            .paragraphs[0]
            .text
            .clone();
        eprintln!(
            "table {} formulas, {SAMPLES} samples (C1 ends {c1:?}):",
            if with_formulas { "with 51" } else { "without" }
        );
        eprintln!("  typing in A1: {}", summary(&mut typing));
        eprintln!("  leaving A1:   {}", summary(&mut leaving));
    }
}

/// CP.1: an input method's commit reaches the document as one `Insert` and
/// is one undo step, whole (the surface publishes it from
/// `InputMethod::Commit`; `ime_commit_inserts_and_navigation_keys_stay_in_their_flow`).
#[test]
fn an_input_method_commit_is_one_undo_step() {
    let mut app = page_app(page_fixture());
    let main = app.document.main_flow_id();
    move_to(&mut app, at(main, 0, 0), 1, false);
    let before = app.document.clone();
    let steps = app.history.undo_len();
    page(&mut app, PageAction::Insert("日本語".into()));
    assert_eq!(app.history.undo_len(), steps + 1, "one step");
    assert!(main_texts(&app)[0].starts_with("日本語"));
    send(&mut app, Message::Command(CommandId::Undo));
    assert_eq!(app.document, before);
}
