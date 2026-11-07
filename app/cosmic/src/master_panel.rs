//! The Master Document panel in the rail (`docs/Design/UI_SPEC.md` §37,
//! MS1, MS4, MS9): Ami Pro's Master Document and Master Document Options
//! dialogs as one panel that acts on the document live. Each change to the
//! list or the options is one command and one undo step.

use amipro_editing::master::Check;
use cosmic::iced::Length;
use cosmic::widget;

use crate::Message;
use crate::master::{MasterSet, MemberState};

pub const WIDTH: f32 = 300.0;

type Element<'a> = cosmic::Element<'a, Message>;

#[derive(Clone, Debug)]
pub enum MasterAction {
    Close,
    /// Add… (the desktop's dialog), before the selected file (Ami Pro's
    /// Include).
    Add,
    /// The files chosen in the dialog.
    Added(Vec<std::path::PathBuf>),
    Select(usize),
    Remove,
    Up,
    Down,
    /// Opens the file in a tab (or goes to its tab).
    Open(usize),
    OpenAll,
    /// Locate… a missing file (the desktop's dialog).
    Locate(usize),
    Located(usize, std::path::PathBuf),
    TocFile(String),
    IndexFile(String),
    /// The output names typed are kept (one undo step).
    CommitOutputs,
    Separators(bool),
    GenerateContents,
    GenerateIndex,
    /// On a member: Treat as Single Document (Ami Pro's Yes).
    TreatAsSingle,
    /// On a member: opens the master that lists it (MS4).
    OpenMaster,
    /// Goes to the file of a check (opens it).
    CheckGo(usize),
}

/// What the panel keeps between messages: the output names as typed and
/// its last message.
#[derive(Clone, Debug, Default)]
pub struct MasterPanel {
    pub toc_file: String,
    pub index_file: String,
    pub message: Option<String>,
}

fn send(action: MasterAction) -> Message {
    Message::Master(action)
}

/// What the panel shows besides the set.
pub struct Shown<'a> {
    pub set: Option<&'a MasterSet>,
    /// The output names as typed.
    pub toc_file: &'a str,
    pub index_file: &'a str,
    pub separators: bool,
    /// The document is a member: where its numbers start (page, footnote).
    pub member: Option<(u32, u32)>,
    /// The master that lists this member, if found (MS4).
    pub offer: Option<&'a std::path::Path>,
    /// The member's numbers are the set's, live (its master is open).
    pub live: bool,
    pub message: Option<&'a str>,
}

fn header() -> Element<'static> {
    widget::row::with_children(vec![
        widget::text::heading("Master Document").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Master Document")
            .on_press(send(MasterAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn wide(label: &str, message: Option<Message>) -> Element<'static> {
    widget::button::custom(
        widget::container(widget::text::body(label.to_owned())).center_x(Length::Fill),
    )
    .class(cosmic::theme::Button::Standard)
    .width(Length::Fill)
    .on_press_maybe(message)
    .into()
}

fn small(label: &str, message: Option<Message>) -> Element<'static> {
    widget::button::text(label.to_owned())
        .on_press_maybe(message)
        .into()
}

/// The words for a check (MS9).
pub fn check_text(check: &Check) -> String {
    match check {
        Check::PageNumberStart => "its page numbers have a start page or number".into(),
        Check::LeadingText => "its page numbers have leading text".into(),
        Check::FootnoteStart(start) => format!("its footnotes start at {start}"),
        Check::NumbersRestart => "a TOC break starts its page numbers again".into(),
    }
}

pub fn panel<'a>(shown: Shown<'a>) -> Element<'a> {
    let mut body: Vec<Element<'a>> = Vec::new();
    if let Some((page, footnote)) = shown.member {
        body.push(widget::text::heading("Part of a master document").into());
        body.push(
            widget::text::body(format!(
                "Pages start at {page}, footnotes at {footnote}{}.",
                if shown.live { "" } else { " (as last saved)" }
            ))
            .into(),
        );
        if let Some(master) = shown.offer {
            let name = master
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            body.push(widget::text::caption(format!("Listed by {name}")).into());
            body.push(wide("Open Master", Some(send(MasterAction::OpenMaster))));
        }
        body.push(wide(
            "Treat as Single Document",
            Some(send(MasterAction::TreatAsSingle)),
        ));
        body.push(widget::divider::horizontal::light().into());
    }
    body.push(widget::text::heading("Files (in order)").into());
    let set = shown.set;
    let members = set.map(|set| set.members.as_slice()).unwrap_or_default();
    let selected = set.and_then(|set| set.selected);
    if members.is_empty() {
        body.push(
            widget::text::caption("No files yet: Add… the documents of the book in their order.")
                .into(),
        );
    }
    for (index, member) in members.iter().enumerate() {
        let state = match (&member.state, member.pages) {
            (MemberState::Missing, _) => "missing".to_string(),
            (MemberState::Unreadable(_), _) => "cannot be read".to_string(),
            (_, Some((first, last))) if first <= last => format!("p {first}–{last}"),
            (_, Some(_)) => "no pages".to_string(),
            (MemberState::Unread, None) => "reading…".to_string(),
            (MemberState::Ready, None) => "after a missing file".to_string(),
        };
        let label = format!("{} {}", index + 1, member.name());
        let line = widget::row::with_children(vec![
            widget::text::body(label).width(Length::Fill).into(),
            widget::text::caption(state).into(),
        ])
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center);
        body.push(
            widget::button::custom(line)
                .class(if selected == Some(index) {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::MenuItem
                })
                .width(Length::Fill)
                .on_press(send(MasterAction::Select(index)))
                .into(),
        );
    }
    let picked = selected.is_some();
    let picked_index = selected.unwrap_or(0);
    let missing = selected
        .and_then(|index| members.get(index))
        .is_some_and(|member| member.state == MemberState::Missing);
    body.push(
        widget::row::with_children(vec![
            small("Add…", Some(send(MasterAction::Add))),
            small("Remove", picked.then(|| send(MasterAction::Remove))),
            small(
                "↑",
                (picked && picked_index > 0).then(|| send(MasterAction::Up)),
            ),
            small(
                "↓",
                (picked && picked_index + 1 < members.len()).then(|| send(MasterAction::Down)),
            ),
        ])
        .spacing(4)
        .into(),
    );
    body.push(
        widget::row::with_children(vec![
            small(
                "Open",
                (picked && !missing).then(|| send(MasterAction::Open(picked_index))),
            ),
            small(
                "Open All",
                (!members.is_empty()).then(|| send(MasterAction::OpenAll)),
            ),
            small(
                "Locate…",
                (picked && missing).then(|| send(MasterAction::Locate(picked_index))),
            ),
        ])
        .spacing(4)
        .into(),
    );
    // The guide's warnings (MS9).
    for (index, member) in members.iter().enumerate() {
        for check in &member.checks {
            body.push(
                widget::button::custom(widget::text::caption(format!(
                    "⚠ {}: {}",
                    member.name(),
                    check_text(check)
                )))
                .class(cosmic::theme::Button::Link)
                .on_press(send(MasterAction::CheckGo(index)))
                .into(),
            );
        }
    }
    if let Some(set) = set.filter(|set| !set.members.is_empty()) {
        body.push(
            widget::text::caption(format!(
                "{} files · {} pages · {} footnotes",
                set.members.len(),
                set.total_pages(),
                set.total_footnotes()
            ))
            .into(),
        );
    }
    body.push(widget::divider::horizontal::light().into());
    body.push(widget::text::heading("Contents and index").into());
    body.push(widget::text::caption("TOC file").into());
    body.push(
        widget::text_input("", shown.toc_file)
            .on_input(|text| send(MasterAction::TocFile(text)))
            .on_submit(|_| send(MasterAction::CommitOutputs))
            .width(Length::Fill)
            .into(),
    );
    body.push(widget::text::caption("Index file").into());
    body.push(
        widget::text_input("", shown.index_file)
            .on_input(|text| send(MasterAction::IndexFile(text)))
            .on_submit(|_| send(MasterAction::CommitOutputs))
            .width(Length::Fill)
            .into(),
    );
    body.push(
        widget::checkbox(shown.separators)
            .label("Alphabetical separators")
            .on_toggle(|on| send(MasterAction::Separators(on)))
            .into(),
    );
    let any = !members.is_empty();
    body.push(wide(
        "Generate Contents",
        any.then(|| send(MasterAction::GenerateContents)),
    ));
    body.push(wide(
        "Generate Index",
        any.then(|| send(MasterAction::GenerateIndex)),
    ));
    if let Some(message) = shown.message {
        body.push(widget::text::caption(message.to_owned()).into());
    }
    widget::column::with_children(vec![
        header(),
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}
