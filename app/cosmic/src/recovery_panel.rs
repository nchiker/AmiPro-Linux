//! File › Recover Documents… in the rail (`docs/Design/UI_SPEC.md` §44,
//! P6.3): the documents a program that ended without saving left as
//! recovery copies. The store is in `recovery.rs`; this file holds the
//! state the app keeps, the actions, and the view.

use std::collections::{HashMap, HashSet};

use amipro_document::Document;
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::recovery::{Entry, Store};

/// The panel's width in the rail.
pub const WIDTH: f32 = 300.0;

/// The rail panel's own state in a session: open or not.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RecoveryPanel;

/// What the app keeps for crash recovery, whichever tab shows the panel.
#[derive(Default)]
pub struct RecoveryState {
    /// The store; `None` turns recovery off (tests that do not use it).
    pub store: Option<Store>,
    /// The copies on offer.
    pub found: Vec<Entry>,
    /// Copies that could not be read, with the reason (they stay on disk).
    pub problems: Vec<String>,
    /// A line for the panel about the last action.
    pub notice: Option<String>,
    /// What each session's copy on disk holds, to write only changes.
    pub last: HashMap<u64, Document>,
    /// Sessions whose copy is on disk now.
    pub written: HashSet<u64>,
    /// Sessions a write is under way for.
    pub pending: HashSet<u64>,
    /// A recovered session's source copy, deleted once the session has a
    /// copy of its own (or is saved or closed).
    pub inherited: HashMap<u64, Entry>,
}

impl RecoveryState {
    pub fn with_store(store: Store) -> Self {
        Self {
            store: Some(store),
            ..Self::default()
        }
    }

    /// Looks for copies again.
    pub fn rescan(&mut self) {
        if let Some(store) = &self.store {
            let scan = store.scan();
            let open: HashSet<String> = self.inherited.values().map(Entry::key).collect();
            self.found = scan
                .entries
                .into_iter()
                .filter(|entry| !open.contains(&entry.key()))
                .collect();
            self.problems = scan.problems;
        }
    }
}

#[derive(Clone, Debug)]
pub enum RecoveryAction {
    Close,
    /// Opens the copy with this key as an unsaved document.
    Recover(String),
    /// Deletes the copy with this key.
    Discard(String),
    DiscardAll,
    /// The autosave interval passed.
    Tick,
    /// A copy was written (or not) from `snapshot`.
    Written {
        id: u64,
        snapshot: Box<Document>,
        result: Result<u64, String>,
    },
}

fn send(action: RecoveryAction) -> Message {
    Message::Recovery(action)
}

/// "2026-11-07 14:03" in the local time zone.
pub fn time_text(seconds: u64) -> String {
    i64::try_from(seconds)
        .ok()
        .and_then(|seconds| jiff::Timestamp::from_second(seconds).ok())
        .map(|time| {
            time.to_zoned(jiff::tz::TimeZone::system())
                .strftime("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_default()
}

/// "12 KB" or "3.4 MB".
pub fn size_text(bytes: u64) -> String {
    if bytes < 1_000_000 {
        format!("{} KB", bytes.div_ceil(1000).max(1))
    } else {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    }
}

/// What a copy is called in the list.
pub fn entry_title(entry: &Entry) -> String {
    entry
        .meta
        .path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map_or_else(|| entry.meta.title.clone(), str::to_owned)
}

/// The panel.
pub fn panel<'a>(state: &'a RecoveryState) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Recover documents").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close the Recover documents panel")
            .on_press(send(RecoveryAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let mut children: Vec<Element<'a, Message>> = vec![header.into()];
    if state.found.is_empty() {
        children.push(widget::text::body("No documents to recover.").into());
    } else {
        children.push(
            widget::text::caption(
                "The program ended before these documents were saved. A recovered document opens unsaved; the files you saved are not changed.",
            )
            .into(),
        );
    }
    for entry in &state.found {
        let place = entry.meta.path.as_ref().map_or_else(
            || "Never saved".to_owned(),
            |path| path.display().to_string(),
        );
        children.push(
            widget::column::with_children(vec![
                widget::text::heading(entry_title(entry)).into(),
                widget::text::caption(place).into(),
                widget::text::caption(format!(
                    "Last copy {}, {}",
                    time_text(entry.meta.time),
                    size_text(entry.meta.bytes)
                ))
                .into(),
                widget::row::with_children(vec![
                    widget::button::suggested("Recover")
                        .on_press(send(RecoveryAction::Recover(entry.key())))
                        .into(),
                    widget::button::destructive("Discard")
                        .on_press(send(RecoveryAction::Discard(entry.key())))
                        .into(),
                ])
                .spacing(8)
                .into(),
            ])
            .spacing(4)
            .into(),
        );
        children.push(widget::divider::horizontal::default().into());
    }
    if state.found.len() > 1 {
        children.push(
            widget::button::destructive("Discard all")
                .on_press(send(RecoveryAction::DiscardAll))
                .into(),
        );
    }
    if let Some(notice) = &state.notice {
        children.push(widget::text::body(notice.clone()).into());
    }
    for problem in &state.problems {
        children.push(widget::text::caption(problem.clone()).into());
    }
    children.push(
        widget::text::caption(
            "Copies are kept while you edit (Preferences › General) and removed when you save or close without saving. Esc closes this panel and keeps them.",
        )
        .into(),
    );
    widget::scrollable(
        widget::column::with_children(children)
            .spacing(8)
            .padding([8, 8])
            .width(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}
