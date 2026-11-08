//! Help › Help (F1), Contents, and Search… in the rail
//! (`docs/Design/UI_SPEC.md` §45): the topics of `help.rs` as a contents
//! tree by area, search results, and a topic with links, See also, and
//! Back and Forward. F1 opens the topic for what the user is in (the open
//! panel, the selected frame's kind, the find bar, a table, Outline mode),
//! or the contents.

use cosmic::iced::font::{Style as FontStyle, Weight};
use cosmic::iced::widget::{rich_text, span};
use cosmic::iced::{Alignment, Color, Font, Length};
use cosmic::{Element, Task, widget};

use crate::help::{AREAS, Block, Inline, Library, Navigator, Page, Topic, area_title};
use crate::{AmiProApp, Message};

/// The panel's width in the rail.
pub const WIDTH: f32 = 340.0;
/// The search box's id, to focus it.
pub const SEARCH_FIELD: &str = "help-search";
/// The scrollable's id, to go to the top of a new page.
pub const SCROLL: &str = "help-scroll";

/// The rail panel's own state in a session: open or not. Where the reader
/// is lives in the app ([`HelpState`]), so that the panel reopens there.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HelpPanel;

/// What the app keeps for Help, whichever tab shows the panel.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HelpState {
    pub nav: Navigator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PanelAction {
    Close,
    /// Opens the topic with this id (a link, a result, a contents entry).
    Open(String),
    Contents,
    Back,
    Forward,
    Query(String),
    ClearQuery,
}

fn send(action: PanelAction) -> Message {
    Message::HelpPanel(action)
}

fn accent() -> Color {
    cosmic::theme::active().cosmic().accent_text_color().into()
}

/// The words of `inlines` as one wrapping paragraph; a link sends its topic.
fn rich<'a>(inlines: &'a [Inline], size: f32) -> Element<'a, Message> {
    let default = cosmic::font::default();
    let spans: Vec<_> = inlines
        .iter()
        .map(|inline| match inline {
            Inline::Text(text) => span::<String, Font>(text.as_str()),
            Inline::Bold(text) => span(text.as_str()).font(Font {
                weight: Weight::Bold,
                ..default
            }),
            Inline::Italic(text) => span(text.as_str()).font(Font {
                style: FontStyle::Italic,
                ..default
            }),
            Inline::Code(text) => span(text.as_str()).font(cosmic::font::mono()),
            Inline::Link { label, topic } => span(label.as_str())
                .underline(true)
                .color(accent())
                .link(topic.clone()),
        })
        .collect();
    rich_text(spans)
        .on_link_click(|topic| send(PanelAction::Open(topic)))
        .size(size)
        .width(Length::Fill)
        .into()
}

fn topic_link<'a>(topic: &'a Topic) -> Element<'a, Message> {
    widget::button::link(topic.title.clone())
        .on_press(send(PanelAction::Open(topic.id.clone())))
        .padding(0)
        .into()
}

fn entry<'a>(topic: &'a Topic) -> Element<'a, Message> {
    widget::column::with_children(vec![
        topic_link(topic),
        widget::text::caption(topic.summary(90)).into(),
    ])
    .spacing(2)
    .into()
}

fn contents_view<'a>(library: &'a Library) -> Vec<Element<'a, Message>> {
    let mut children = Vec::new();
    for (area, title) in AREAS {
        let topics = library.in_area(area);
        if topics.is_empty() {
            continue;
        }
        children.push(widget::text::heading(*title).into());
        for topic in topics {
            children.push(entry(topic));
        }
    }
    children
}

fn search_view<'a>(library: &'a Library, query: &str) -> Vec<Element<'a, Message>> {
    if query.trim().is_empty() {
        return contents_view(library);
    }
    let hits = library.search(query);
    let mut children: Vec<Element<'a, Message>> = Vec::new();
    if hits.is_empty() {
        children.push(widget::text::body("No topic matches. Try fewer or shorter words.").into());
        return children;
    }
    children.push(
        widget::text::caption(format!(
            "{} {} for “{}”",
            hits.len(),
            if hits.len() == 1 { "topic" } else { "topics" },
            query.trim()
        ))
        .into(),
    );
    for hit in hits {
        children.push(
            widget::column::with_children(vec![
                topic_link(hit.topic),
                widget::text::caption(format!(
                    "{} · {}",
                    area_title(&hit.topic.area),
                    hit.topic.summary(70)
                ))
                .into(),
            ])
            .spacing(2)
            .into(),
        );
    }
    children
}

fn topic_view<'a>(library: &'a Library, topic: &'a Topic) -> Vec<Element<'a, Message>> {
    let mut children: Vec<Element<'a, Message>> = vec![
        widget::text::caption(area_title(&topic.area)).into(),
        widget::text::title4(topic.title.as_str()).into(),
    ];
    for block in &topic.blocks {
        match block {
            Block::Heading(2, inlines) => {
                children.push(widget::text::heading(plain(inlines)).into());
            }
            Block::Heading(_, inlines) => {
                children.push(
                    widget::text::body(plain(inlines))
                        .font(Font {
                            weight: Weight::Bold,
                            ..cosmic::font::default()
                        })
                        .into(),
                );
            }
            Block::Paragraph(inlines) => children.push(rich(inlines, 14.0)),
            Block::List { ordered, items } => {
                for (number, item) in items.iter().enumerate() {
                    let mark = if *ordered {
                        format!("{}.", number + 1)
                    } else {
                        "•".to_owned()
                    };
                    children.push(
                        widget::row::with_children(vec![
                            widget::text::body(mark).width(Length::Fixed(22.0)).into(),
                            rich(item, 14.0),
                        ])
                        .into(),
                    );
                }
            }
            Block::Code(code) => children.push(
                widget::text::body(code.as_str())
                    .font(cosmic::font::mono())
                    .into(),
            ),
        }
    }
    let related: Vec<&Topic> = topic
        .see_also
        .iter()
        .filter_map(|id| library.get(id))
        .collect();
    if !related.is_empty() {
        children.push(widget::divider::horizontal::default().into());
        children.push(widget::text::heading("See also").into());
        for other in related {
            children.push(topic_link(other));
        }
    }
    children
}

fn plain(inlines: &[Inline]) -> String {
    inlines.iter().map(Inline::plain).collect()
}

/// The panel.
pub fn panel<'a>(state: &'a HelpState) -> Element<'a, Message> {
    let library = Library::builtin();
    let nav = &state.nav;
    let header = widget::row::with_children(vec![
        widget::text::heading("Help").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close the Help panel")
            .on_press(send(PanelAction::Close))
            .into(),
    ])
    .align_y(Alignment::Center);
    let mut back = widget::button::icon(widget::icon::from_name("go-previous-symbolic"))
        .extra_small()
        .name("Back");
    if nav.can_back() {
        back = back.on_press(send(PanelAction::Back));
    }
    let mut forward = widget::button::icon(widget::icon::from_name("go-next-symbolic"))
        .extra_small()
        .name("Forward");
    if nav.can_forward() {
        forward = forward.on_press(send(PanelAction::Forward));
    }
    let mut contents = widget::button::standard("Contents");
    if nav.page != Page::Contents {
        contents = contents.on_press(send(PanelAction::Contents));
    }
    let bar = widget::row::with_children(vec![
        back.into(),
        forward.into(),
        widget::space::horizontal().into(),
        contents.into(),
    ])
    .spacing(8)
    .align_y(Alignment::Center);
    let search = widget::search_input("Search Help", nav.query.clone())
        .id(widget::Id::new(SEARCH_FIELD))
        .on_input(|text| send(PanelAction::Query(text)))
        .on_clear(send(PanelAction::ClearQuery))
        .width(Length::Fill);
    let body = match &nav.page {
        Page::Contents => contents_view(library),
        Page::Search => search_view(library, &nav.query),
        Page::Topic(id) => match library.get(id) {
            Some(topic) => topic_view(library, topic),
            None => vec![widget::text::body("That topic is not in this version of Help.").into()],
        },
    };
    widget::column::with_children(vec![
        widget::column::with_children(vec![header.into(), bar.into(), search.into()])
            .spacing(8)
            .padding([8, 8, 0, 8])
            .into(),
        widget::scrollable(
            widget::column::with_children(body)
                .spacing(8)
                .padding([8, 8])
                .width(Length::Fill),
        )
        .id(widget::Id::new(SCROLL))
        .height(Length::Fill)
        .into(),
    ])
    .height(Length::Fill)
    .into()
}

impl AmiProApp {
    /// What the user is in, most specific first, as the context ids of
    /// `help.rs` (`command:Name`, `panel:Name`). F1 opens the first one that
    /// has a topic.
    pub(crate) fn help_contexts(&self) -> Vec<String> {
        use crate::rail_panels::RailPanel;
        let mut contexts = Vec::new();
        if let Some(drawer) = &self.help_drawer {
            contexts.push(
                match drawer {
                    crate::help_drawers::HelpDrawer::Shortcuts { .. } => {
                        "command:KeyboardShortcuts"
                    }
                    crate::help_drawers::HelpDrawer::Compatibility { .. } => {
                        "command:Compatibility"
                    }
                }
                .to_owned(),
            );
        }
        if self.page_surface {
            if self.drawing_open() {
                contexts.push("panel:Drawing".into());
            }
            match self.rail_panel() {
                RailPanel::Frame => {
                    if let Some((frame, _)) = self.inspected_frame()
                        && let Some(frame) = self.document.frame(frame)
                    {
                        use amipro_document::FrameContent;
                        contexts.push(
                            match frame.content {
                                FrameContent::Picture(_) => "command:Image",
                                FrameContent::Table(_) => "command:Table",
                                FrameContent::Text(_) => "command:Frame",
                                FrameContent::Legacy(_) => "command:Drawing",
                            }
                            .into(),
                        );
                    }
                    contexts.push("panel:Frame".into());
                }
                RailPanel::Styles | RailPanel::Drawing => {}
                panel => contexts.push(format!("panel:{panel:?}")),
            }
            if self.find.is_some() {
                contexts.push("command:Find".into());
            }
            if crate::table_editing::caret_cell(&self.document, self.page_selection).is_some() {
                contexts.push("command:Table".into());
            }
            if self.outline.is_some() {
                contexts.push("command:OutlineMode".into());
            }
        }
        contexts
    }

    /// Shows the panel, at `page`, and focuses the search box when asked.
    pub(crate) fn open_help(
        &mut self,
        page: Page,
        focus_search: bool,
    ) -> Task<cosmic::Action<Message>> {
        let session = &mut self.sessions[self.active_session];
        session.close_rail_panels();
        session.help_panel = Some(HelpPanel);
        self.help.nav.go(page);
        self.open_rail();
        let mut tasks = vec![scroll_to_top()];
        if focus_search {
            let id = widget::Id::new(SEARCH_FIELD);
            tasks.push(
                widget::text_input::focus::<Message>(id.clone())
                    .chain(widget::text_input::select_all::<Message>(id))
                    .chain(Task::done(Message::Repaint))
                    .map(cosmic::Action::from),
            );
        }
        Task::batch(tasks)
    }

    /// Help › Help (F1): the topic for the context, or the contents.
    pub(crate) fn open_help_for_context(&mut self) -> Task<cosmic::Action<Message>> {
        let contexts = self.help_contexts();
        let page = Library::builtin()
            .for_contexts(&contexts)
            .map_or(Page::Contents, |topic| Page::Topic(topic.id.clone()));
        self.open_help(page, false)
    }

    fn close_help_panel(&mut self) {
        self.sessions[self.active_session].help_panel = None;
        if self.rail_open() {
            self.toggle_rail();
        }
    }

    pub(crate) fn update_help(&mut self, action: PanelAction) -> Task<cosmic::Action<Message>> {
        match action {
            PanelAction::Close => {
                self.close_help_panel();
                return Task::none();
            }
            PanelAction::Open(id) => {
                if Library::builtin().get(&id).is_some() {
                    self.help.nav.go(Page::Topic(id));
                } else {
                    self.status = format!("Help has no topic {id}");
                    return Task::none();
                }
            }
            PanelAction::Contents => self.help.nav.go(Page::Contents),
            PanelAction::Back => self.help.nav.back(),
            PanelAction::Forward => self.help.nav.forward(),
            PanelAction::Query(text) => {
                self.help.nav.set_query(text);
                return Task::none();
            }
            PanelAction::ClearQuery => {
                self.help.nav.query.clear();
                return Task::none();
            }
        }
        scroll_to_top()
    }
}

fn scroll_to_top() -> Task<cosmic::Action<Message>> {
    cosmic::iced::widget::scrollable::scroll_to::<Message>(
        widget::Id::new(SCROLL),
        cosmic::iced::widget::scrollable::AbsoluteOffset {
            x: None,
            y: Some(0.0),
        },
    )
    .chain(Task::done(Message::Repaint))
    .map(cosmic::Action::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{self, CommandId, Implementation, MENU_LAYOUTS, MenuSlot};
    use crate::page_surface_app_tests::{
        at, cell, draw_window, frame_action, magenta_picture_document, move_to, page_app, send,
        table_app,
    };
    use crate::rail_panels::RailPanel;
    use amipro_document::{Document, Offset, PictureScaling, Size};
    use std::collections::BTreeSet;

    /// Implemented menu commands that have no topic yet. Content passes
    /// shrink this list: a command that gains a `context:` entry must be
    /// taken out (the test says so), and a name here must be a real,
    /// implemented menu command.
    const WITHOUT_HELP: &[&str] = &[];

    fn library() -> &'static Library {
        Library::builtin()
    }

    fn command_names() -> BTreeSet<String> {
        command::COMMANDS
            .iter()
            .map(|metadata| format!("{:?}", metadata.id))
            .collect()
    }

    fn panel_names() -> BTreeSet<String> {
        RailPanel::ALL
            .iter()
            .chain([&RailPanel::Styles])
            .map(|panel| format!("{panel:?}"))
            .collect()
    }

    fn open(app: &mut AmiProApp) {
        send(app, Message::Command(CommandId::Help));
    }

    fn page(app: &AmiProApp) -> &Page {
        &app.help.nav.page
    }

    #[test]
    fn f1_is_the_help_shortcut() {
        use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
        assert_eq!(
            command::command_for_shortcut(Modifiers::empty(), &Key::Named(Named::F1), None),
            Some(CommandId::Help)
        );
        for id in [
            CommandId::Help,
            CommandId::HelpContents,
            CommandId::HelpSearch,
        ] {
            assert_eq!(
                command::metadata(id).implementation,
                Implementation::Implemented
            );
        }
    }

    #[test]
    fn every_context_id_names_a_real_command_or_panel() {
        let commands = command_names();
        let panels = panel_names();
        for topic in library().topics() {
            for context in &topic.contexts {
                let (kind, name) = context
                    .split_once(':')
                    .unwrap_or_else(|| panic!("{}: context {context:?} has no kind", topic.path));
                let known = match kind {
                    "command" => commands.contains(name),
                    "panel" => panels.contains(name),
                    _ => false,
                };
                assert!(known, "{}: context {context:?} names nothing", topic.path);
            }
        }
    }

    #[test]
    fn a_context_belongs_to_one_topic() {
        let mut seen = BTreeSet::new();
        for topic in library().topics() {
            for context in &topic.contexts {
                assert!(
                    seen.insert(context.clone()),
                    "{context} is the F1 help of two topics"
                );
            }
        }
    }

    #[test]
    fn every_implemented_menu_command_has_help_or_is_exempt() {
        let covered: BTreeSet<String> = library()
            .topics()
            .iter()
            .flat_map(|topic| topic.contexts.iter())
            .filter_map(|context| context.strip_prefix("command:"))
            .map(str::to_owned)
            .collect();
        let mut menu = BTreeSet::new();
        for (_, slots) in MENU_LAYOUTS {
            for id in MenuSlot::commands(slots) {
                if command::metadata(id).implementation == Implementation::Implemented {
                    menu.insert(format!("{id:?}"));
                }
            }
        }
        let missing: Vec<&String> = menu
            .iter()
            .filter(|name| !covered.contains(*name) && !WITHOUT_HELP.contains(&name.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "menu commands with no help and no exemption: {missing:?}"
        );
        for name in WITHOUT_HELP {
            assert!(
                menu.contains(*name),
                "{name} is not an implemented menu command"
            );
            assert!(
                !covered.contains(*name),
                "{name} has help now: take it out of WITHOUT_HELP"
            );
        }
    }

    #[test]
    fn f1_with_nothing_special_opens_the_contents() {
        let mut app = page_app(Document::default());
        open(&mut app);
        assert!(app.help_panel.is_some());
        assert_eq!(app.rail_panel(), RailPanel::Help);
        assert_eq!(*page(&app), Page::Contents);
        draw_window(&app);
    }

    #[test]
    fn f1_opens_the_topic_of_the_open_panel() {
        let mut app = page_app(Document::default());
        for (command, topic) in [
            (CommandId::ModifyCurrentStyle, "modify-style"),
            (CommandId::PageLayout, "page-layout-panel"),
            (CommandId::CheckForUpdates, "updates"),
            (CommandId::RecoverDocuments, "crash-recovery"),
            (CommandId::Sort, "sorting"),
            (CommandId::Charting, "charts"),
            (CommandId::Equations, "equations"),
            (CommandId::MasterDocument, "master-documents"),
            (CommandId::ContentsIndex, "toc-index"),
        ] {
            send(&mut app, Message::Command(command));
            open(&mut app);
            assert_eq!(*page(&app), Page::Topic(topic.into()), "{command:?}");
            assert_eq!(app.rail_panel(), RailPanel::Help);
            assert!(app.updates_panel.is_none() && app.recovery_panel.is_none());
        }
    }

    #[test]
    fn f1_inside_help_opens_how_to_use_it() {
        let mut app = page_app(Document::default());
        open(&mut app);
        open(&mut app);
        assert_eq!(*page(&app), Page::Topic("using-help".into()));
    }

    #[test]
    fn f1_follows_the_kind_of_frame_selected_and_the_table_and_find_bar() {
        use crate::frame_editing::FrameAction;
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
        open(&mut app);
        assert_eq!(*page(&app), Page::Topic("pictures".into()));

        let (mut app, frame) = table_app();
        let target = at(cell(&app.document, frame, 0, 0), 0, 0);
        move_to(&mut app, target, 1, false);
        open(&mut app);
        assert_eq!(*page(&app), Page::Topic("tables".into()));

        let mut app = page_app(Document::default());
        send(&mut app, Message::Command(CommandId::Find));
        open(&mut app);
        assert_eq!(*page(&app), Page::Topic("find-replace".into()));
    }

    #[test]
    fn f1_on_the_shortcuts_drawer_opens_its_topic() {
        let mut app = page_app(Document::default());
        send(&mut app, Message::Command(CommandId::KeyboardShortcuts));
        open(&mut app);
        assert_eq!(*page(&app), Page::Topic("keyboard-shortcuts".into()));
    }

    #[test]
    fn contents_and_search_commands_open_their_pages() {
        let mut app = page_app(Document::default());
        send(&mut app, Message::Command(CommandId::HelpSearch));
        assert_eq!(*page(&app), Page::Search);
        assert_eq!(app.rail_panel(), RailPanel::Help);
        send(&mut app, Message::Command(CommandId::HelpContents));
        assert_eq!(*page(&app), Page::Contents);
    }

    #[test]
    fn search_links_back_forward_and_escape_work_through_messages() {
        let mut app = page_app(Document::default());
        send(&mut app, Message::Command(CommandId::HelpSearch));
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Query("table".into())),
        );
        assert_eq!(*page(&app), Page::Search);
        assert_eq!(app.help.nav.query, "table");
        draw_window(&app);
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Open("tables".into())),
        );
        assert_eq!(*page(&app), Page::Topic("tables".into()));
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Open("pictures".into())),
        );
        draw_window(&app);
        send(&mut app, Message::HelpPanel(PanelAction::Back));
        assert_eq!(*page(&app), Page::Topic("tables".into()));
        send(&mut app, Message::HelpPanel(PanelAction::Back));
        assert_eq!(*page(&app), Page::Search);
        assert_eq!(app.help.nav.query, "table", "the query is kept");
        send(&mut app, Message::HelpPanel(PanelAction::Forward));
        assert_eq!(*page(&app), Page::Topic("tables".into()));
        // A topic that does not exist is refused and says so.
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Open("nothing".into())),
        );
        assert_eq!(*page(&app), Page::Topic("tables".into()));
        assert!(app.status.contains("nothing"), "{}", app.status);
        send(&mut app, Message::HelpPanel(PanelAction::Contents));
        assert_eq!(*page(&app), Page::Contents);
        send(&mut app, Message::HelpPanel(PanelAction::ClearQuery));
        assert!(app.help.nav.query.is_empty());
        // Esc closes the panel; the reader's place is kept.
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Open("styles".into())),
        );
        send(&mut app, Message::Escape);
        assert!(app.help_panel.is_none());
        assert_eq!(app.rail_panel(), RailPanel::Styles);
        send(&mut app, Message::Command(CommandId::HelpContents));
        send(&mut app, Message::HelpPanel(PanelAction::Back));
        assert_eq!(*page(&app), Page::Topic("styles".into()));
    }

    #[test]
    fn opening_another_panel_closes_help() {
        let mut app = page_app(Document::default());
        open(&mut app);
        send(&mut app, Message::Command(CommandId::CheckForUpdates));
        assert_eq!(app.rail_panel(), RailPanel::Updates);
        assert!(app.help_panel.is_none());
    }

    #[test]
    fn every_topic_and_the_search_results_build_in_the_window() {
        let mut app = page_app(Document::default());
        open(&mut app);
        for topic in library().topics() {
            send(
                &mut app,
                Message::HelpPanel(PanelAction::Open(topic.id.clone())),
            );
            draw_window(&app);
        }
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Query("zzzzqq".into())),
        );
        draw_window(&app);
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Query("page".into())),
        );
        draw_window(&app);
    }

    /// The panel in light and dark into `AMIPRO_SCREENSHOT_DIR`, as
    /// `window_screenshots` does.
    #[test]
    fn help_panel_screenshots() {
        let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
            eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
            return;
        };
        let directory = std::path::PathBuf::from(directory);
        let mut app = page_app(Document::from_plain_text(
            "Annual report\nThe quick brown fox jumps over the lazy dog.",
        ));
        drop(app.update_title());
        let write = |app: &AmiProApp, name: &str, dark: bool| {
            crate::page_surface_app_tests::write_window_ppm(
                app,
                &directory.join(format!("{name}.ppm")),
                dark,
            );
        };
        open(&mut app);
        write(&app, "help-contents-light", false);
        write(&app, "help-contents-dark", true);
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Open("tables".into())),
        );
        write(&app, "help-topic-light", false);
        write(&app, "help-topic-dark", true);
        send(
            &mut app,
            Message::HelpPanel(PanelAction::Query("frame".into())),
        );
        write(&app, "help-search-light", false);
        write(&app, "help-search-dark", true);
    }
}
