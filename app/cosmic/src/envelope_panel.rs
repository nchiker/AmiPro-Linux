//! File › Print Envelope… (`docs/Design/UI_SPEC.md` §38, MG8): the
//! Envelope panel in the rail in place of Ami Pro's `AMIENV.SMM` macro:
//! Ami Pro's sizes and Custom, the recipient (the selection, else the
//! first address on page 1), return addresses saved by name in the
//! settings, Print return address, the feed position and offsets, and the
//! outputs: Print…, PDF…, New Document, and Merge Envelopes with the
//! document's data file.

use std::path::PathBuf;

use amipro_editing::merge::envelopes::{
    CUSTOM_RANGE, DEFAULT_SIZE, Envelope, RETURN_LINES, SIZES, envelope_document, fill,
    first_address, main_lines,
};
use cosmic::dialog::file_chooser::{self, FileFilter};
use cosmic::iced::Length;
use cosmic::widget;

use crate::rail_panels::RailPanel;
use crate::{AmiProApp, Message};

pub const WIDTH: f32 = 300.0;
/// The recipient's lines the panel shows.
const RECIPIENT_LINES: usize = 6;

type Element<'a> = cosmic::Element<'a, Message>;
type Task = cosmic::app::Task<Message>;

/// Where the printer takes an envelope in: Ami Pro's Options feed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Feed {
    #[default]
    Left,
    Centre,
    Right,
}

impl Feed {
    pub fn number(self) -> i64 {
        match self {
            Self::Left => 0,
            Self::Centre => 1,
            Self::Right => 2,
        }
    }

    pub fn from_number(number: i64) -> Self {
        match number {
            1 => Self::Centre,
            2 => Self::Right,
            _ => Self::Left,
        }
    }
}

/// What the settings keep between sessions (Ami Pro's `AMIPRO2.INI`
/// `[AmiEnv]`): the size, Custom's, Print return address, the feed, the
/// offsets, and the return address last used.
#[derive(Clone, Debug, PartialEq)]
pub struct EnvelopeOptions {
    /// Its place in [`SIZES`], or `SIZES.len()` for Custom.
    pub size: usize,
    pub custom: (i32, i32),
    pub print_return: bool,
    pub feed: Feed,
    pub recipient_offset: (i32, i32),
    pub return_offset: (i32, i32),
    pub last_return: String,
}

impl Default for EnvelopeOptions {
    fn default() -> Self {
        Self {
            size: DEFAULT_SIZE,
            custom: (SIZES[DEFAULT_SIZE].width, SIZES[DEFAULT_SIZE].height),
            print_return: true,
            feed: Feed::Left,
            recipient_offset: (0, 0),
            return_offset: (0, 0),
            last_return: String::new(),
        }
    }
}

impl EnvelopeOptions {
    pub fn to_numbers(&self) -> Vec<i64> {
        [
            self.size as i64,
            i64::from(self.custom.0),
            i64::from(self.custom.1),
            i64::from(self.print_return),
            self.feed.number(),
            i64::from(self.recipient_offset.0),
            i64::from(self.recipient_offset.1),
            i64::from(self.return_offset.0),
            i64::from(self.return_offset.1),
        ]
        .to_vec()
    }

    pub fn from_numbers(numbers: &[i64], last_return: String) -> Option<Self> {
        let [size, width, height, print_return, feed, rx, ry, tx, ty] = numbers else {
            return None;
        };
        let length = |value: i64| i32::try_from(value).ok();
        Some(Self {
            size: (*size as usize).min(SIZES.len()),
            custom: (length(*width)?, length(*height)?),
            print_return: *print_return != 0,
            feed: Feed::from_number(*feed),
            recipient_offset: (length(*rx)?, length(*ry)?),
            return_offset: (length(*tx)?, length(*ty)?),
            last_return,
        })
    }

    /// The envelope's width and height.
    pub fn size(&self) -> (i32, i32) {
        match SIZES.get(self.size) {
            Some(size) => (size.width, size.height),
            None => self.custom,
        }
    }
}

#[derive(Clone, Debug)]
pub enum EnvelopeAction {
    Close,
    Size(usize),
    CustomWidth(String),
    CustomHeight(String),
    Recipient(usize, String),
    ReturnLine(usize, String),
    /// A saved return address chosen by its place in the list.
    ReturnChosen(usize),
    ReturnName(String),
    SaveReturn,
    DeleteReturn,
    PrintReturn(bool),
    Feed(Feed),
    /// An offset typed, in inches: 0 recipient right, 1 recipient down, 2
    /// return right, 3 return down.
    Offset(usize, String),
    Print,
    Pdf,
    PdfChosen(PathBuf),
    NewDocument,
    /// Every record of the document's data file on its own envelope.
    MergeEnvelopes,
}

/// What the panel keeps between messages.
#[derive(Clone, Debug, Default)]
pub struct EnvelopePanel {
    pub recipient: Vec<String>,
    pub return_lines: Vec<String>,
    pub return_name: String,
    pub custom: (String, String),
    pub offsets: [String; 4],
    pub message: Option<String>,
    /// The envelope's print job waits for the desktop's print dialog.
    pub printing: bool,
}

fn send(action: EnvelopeAction) -> Message {
    Message::Envelope(action)
}

fn inches_text(twips: i32) -> String {
    let inches = f64::from(twips) / 1_440.0;
    let text = format!("{inches:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn inches_of(text: &str) -> Option<i32> {
    let value: f64 = text
        .trim()
        .trim_end_matches('"')
        .trim_end_matches("in")
        .trim()
        .parse()
        .ok()?;
    value.is_finite().then(|| (value * 1_440.0).round() as i32)
}

fn size_names() -> &'static [String] {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        SIZES
            .iter()
            .map(|size| size.name.to_string())
            .chain(std::iter::once("Custom".to_string()))
            .collect()
    })
}

fn small(label: &str, message: Option<Message>) -> Element<'static> {
    widget::button::text(label.to_owned())
        .on_press_maybe(message)
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

/// The panel.
pub fn panel<'a>(
    state: &'a EnvelopePanel,
    options: &'a EnvelopeOptions,
    saved: &'a [(String, String)],
    can_merge: bool,
) -> Element<'a> {
    let mut body: Vec<Element<'a>> = Vec::new();
    body.push(widget::text::heading("Size").into());
    body.push(
        widget::dropdown(
            size_names(),
            Some(options.size.min(SIZES.len())),
            |chosen| send(EnvelopeAction::Size(chosen)),
        )
        .width(Length::Fill)
        .into(),
    );
    if options.size >= SIZES.len() {
        body.push(
            widget::row::with_children(vec![
                widget::text::caption("Length").into(),
                widget::text_input("9.5", state.custom.0.clone())
                    .on_input(|text| send(EnvelopeAction::CustomWidth(text)))
                    .width(Length::Fixed(64.0))
                    .into(),
                widget::text::caption("Width").into(),
                widget::text_input("4.13", state.custom.1.clone())
                    .on_input(|text| send(EnvelopeAction::CustomHeight(text)))
                    .width(Length::Fixed(64.0))
                    .into(),
                widget::text::caption("in").into(),
            ])
            .spacing(4)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
    }
    body.push(widget::divider::horizontal::light().into());
    body.push(widget::text::heading("Recipient").into());
    for index in 0..RECIPIENT_LINES {
        let line = state.recipient.get(index).cloned().unwrap_or_default();
        body.push(
            widget::text_input("", line)
                .on_input(move |text| send(EnvelopeAction::Recipient(index, text)))
                .width(Length::Fill)
                .into(),
        );
    }
    body.push(widget::divider::horizontal::light().into());
    body.push(widget::text::heading("Return address").into());
    let names: Vec<String> = saved.iter().map(|(name, _)| name.clone()).collect();
    let chosen = saved
        .iter()
        .position(|(name, _)| *name == options.last_return);
    body.push(
        widget::row::with_children(vec![
            widget::dropdown(leak(names), chosen, |chosen| {
                send(EnvelopeAction::ReturnChosen(chosen))
            })
            .width(Length::Fill)
            .into(),
            small("Delete", chosen.map(|_| send(EnvelopeAction::DeleteReturn))),
        ])
        .spacing(4)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    );
    body.push(
        widget::checkbox(options.print_return)
            .label("Print return address")
            .on_toggle(|on| send(EnvelopeAction::PrintReturn(on)))
            .into(),
    );
    for index in 0..RETURN_LINES {
        let line = state.return_lines.get(index).cloned().unwrap_or_default();
        body.push(
            widget::text_input("", line)
                .on_input(move |text| send(EnvelopeAction::ReturnLine(index, text)))
                .width(Length::Fill)
                .into(),
        );
    }
    body.push(
        widget::row::with_children(vec![
            widget::text_input("Name to save as", state.return_name.clone())
                .on_input(|text| send(EnvelopeAction::ReturnName(text)))
                .on_submit(|_| send(EnvelopeAction::SaveReturn))
                .width(Length::Fill)
                .into(),
            small(
                "Save",
                (!state.return_name.trim().is_empty()).then(|| send(EnvelopeAction::SaveReturn)),
            ),
        ])
        .spacing(4)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    );
    body.push(widget::divider::horizontal::light().into());
    body.push(widget::text::heading("Feed and offsets").into());
    let feed = |label: &str, value: Feed| -> Element<'static> {
        widget::button::text(label.to_owned())
            .class(if options.feed == value {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            })
            .on_press(send(EnvelopeAction::Feed(value)))
            .into()
    };
    body.push(
        widget::row::with_children(vec![
            feed("Left", Feed::Left),
            feed("Centre", Feed::Centre),
            feed("Right", Feed::Right),
        ])
        .spacing(4)
        .into(),
    );
    for (index, label) in ["Recipient →", "Recipient ↓", "Return →", "Return ↓"]
        .into_iter()
        .enumerate()
    {
        body.push(
            widget::row::with_children(vec![
                widget::text::caption(label)
                    .width(Length::Fixed(96.0))
                    .into(),
                widget::text_input("0", state.offsets[index].clone())
                    .on_input(move |text| send(EnvelopeAction::Offset(index, text)))
                    .width(Length::Fixed(64.0))
                    .into(),
                widget::text::caption("in").into(),
            ])
            .spacing(4)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
        );
    }
    body.push(widget::divider::horizontal::light().into());
    let ready = state.recipient.iter().any(|line| !line.trim().is_empty());
    body.push(wide(
        "Print…",
        (ready && !state.printing).then(|| send(EnvelopeAction::Print)),
    ));
    body.push(wide("PDF…", ready.then(|| send(EnvelopeAction::Pdf))));
    body.push(wide(
        "New Document",
        ready.then(|| send(EnvelopeAction::NewDocument)),
    ));
    body.push(wide(
        "Merge Envelopes",
        (ready && can_merge).then(|| send(EnvelopeAction::MergeEnvelopes)),
    ));
    if !can_merge {
        body.push(
            widget::text::caption(
                "Merge Envelopes takes the document's data file (File › Merge…); the recipient names fields as <NAME>.",
            )
            .into(),
        );
    }
    if let Some(message) = &state.message {
        body.push(widget::text::caption(message.clone()).into());
    }
    let header = widget::row::with_children(vec![
        widget::text::heading("Envelope").into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Envelope")
            .on_press(send(EnvelopeAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

/// The dropdown keeps a borrowed list for the view's life; the saved
/// names change rarely, so each list is kept once per distinct value.
fn leak(names: Vec<String>) -> &'static [String] {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static LISTS: OnceLock<Mutex<HashMap<Vec<String>, &'static [String]>>> = OnceLock::new();
    let lists = LISTS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lists = lists.lock().expect("the lists");
    if let Some(list) = lists.get(&names) {
        return list;
    }
    let list: &'static [String] = Box::leak(names.clone().into_boxed_slice());
    lists.insert(names, list);
    list
}

impl AmiProApp {
    /// File › Print Envelope…: the panel, the recipient taken from the
    /// selection, else the first address on page 1 (`AMIENV.SMM`).
    pub(crate) fn open_envelope_panel(&mut self) -> Task {
        let selected = self.selected_text().filter(|text| !text.trim().is_empty());
        let recipient = match selected {
            Some(text) => text
                .split(['\n', '\u{2028}', '\r'])
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect(),
            None => first_address(&main_lines(self.merge_document())).unwrap_or_default(),
        };
        let options = self.preferences.envelope.clone();
        let return_lines = self
            .preferences
            .return_addresses
            .iter()
            .find(|(name, _)| *name == options.last_return)
            .map(|(_, lines)| lines.lines().map(str::to_string).collect())
            .unwrap_or_default();
        let message = recipient
            .is_empty()
            .then(|| "No address found: type the recipient's.".to_string());
        // A shown Merge panel stays set behind this one.
        let merge_behind = self.rail_state_set(RailPanel::Merge);
        self.close_rail_panels();
        if let Some(panel) = self.merge_panel.as_mut() {
            panel.shown = merge_behind;
        }
        self.envelope_panel = Some(EnvelopePanel {
            recipient,
            return_lines,
            return_name: options.last_return.clone(),
            custom: (inches_text(options.custom.0), inches_text(options.custom.1)),
            offsets: [
                inches_text(options.recipient_offset.0),
                inches_text(options.recipient_offset.1),
                inches_text(options.return_offset.0),
                inches_text(options.return_offset.1),
            ],
            message,
            printing: false,
        });
        self.open_rail();
        // The document's data file, for Merge Envelopes: read as the Merge
        // panel reads it, the panel kept behind this one.
        self.read_stored_merge_data()
    }

    pub(crate) fn envelope_view(&self) -> Element<'_> {
        let Some(state) = self.envelope_panel.as_ref() else {
            return widget::space::horizontal().into();
        };
        panel(
            state,
            &self.preferences.envelope,
            &self.preferences.return_addresses,
            self.merge_panel_data().is_some(),
        )
    }

    /// The document's data file as its Merge panel last read it.
    fn merge_panel_data(&self) -> Option<std::sync::Arc<crate::merge::MergeData>> {
        self.merge_panel
            .as_ref()
            .and_then(|panel| panel.data.clone())
    }

    /// The envelope as the panel describes it.
    fn envelope(&self) -> Option<Envelope> {
        let state = self.envelope_panel.as_ref()?;
        let options = &self.preferences.envelope;
        let (width, height) = options.size();
        let lines = |lines: &[String]| -> Vec<String> {
            let mut lines: Vec<String> = lines
                .iter()
                .map(|line| line.trim_end().to_string())
                .collect();
            while lines.last().is_some_and(|line| line.trim().is_empty()) {
                lines.pop();
            }
            lines
        };
        Some(Envelope {
            width,
            height,
            recipient: lines(&state.recipient),
            return_address: if options.print_return {
                lines(&state.return_lines)
            } else {
                Vec::new()
            },
            recipient_offset: options.recipient_offset,
            return_offset: options.return_offset,
        })
    }

    fn set_envelope_message(&mut self, message: impl Into<String>) {
        if let Some(panel) = self.envelope_panel.as_mut() {
            panel.message = Some(message.into());
        }
    }

    /// The envelope's PDF job: its one page, on larger paper at the feed's
    /// place when the print dialog chose larger paper (MC10).
    fn envelope_job(&self, paper: Option<(f32, f32)>) -> Option<crate::print::PdfJob> {
        let envelope = self.envelope()?;
        let document = self.envelope_on_sheet(&envelope);
        let layout = crate::page_layout::with_private_shaper(|shaper| {
            amipro_layout::layout_document(
                &document,
                shaper,
                &mut amipro_layout::ShapeCache::default(),
            )
        });
        let mut options = crate::print::PdfOptions {
            additions: self.preferences.pdf_additions,
            ..crate::print::PdfOptions::default()
        };
        if let Some((paper_width, paper_height)) = paper {
            let (width, height) = (envelope.width as f32, envelope.height as f32);
            if paper_width > width + 1.0 || paper_height > height + 1.0 {
                let left = match self.preferences.envelope.feed {
                    Feed::Left => 0.0,
                    Feed::Centre => ((paper_width - width) / 2.0).max(0.0),
                    Feed::Right => (paper_width - width).max(0.0),
                };
                options.placed =
                    Some([paper_width.max(width), paper_height.max(height), left, 0.0]);
            }
        }
        Some(crate::print::PdfJob {
            document,
            layout,
            folder: None,
            set: Vec::new(),
            options,
            labels: None,
        })
    }

    /// The envelope on Ami Pro's `_AMIENV.STY` when the style-sheet folders
    /// hold it (its `Recipient` style), else on the app's own.
    fn envelope_on_sheet(&self, envelope: &Envelope) -> amipro_document::Document {
        let (base, found) = self.sheet_document("_AMIENV.STY", "Envelope");
        if found {
            amipro_editing::merge::envelopes::envelope_on(base, envelope)
        } else {
            envelope_document(envelope)
        }
    }

    /// Merged envelopes: one per record the Merge panel's conditions select.
    fn merged_envelopes(&self) -> Vec<amipro_document::Document> {
        let (Some(envelope), Some(data)) = (self.envelope(), self.merge_panel_data()) else {
            return Vec::new();
        };
        self.merge_records()
            .into_iter()
            .map(|record| {
                self.envelope_on_sheet(&Envelope {
                    recipient: fill(&envelope.recipient, &data.table, record),
                    ..envelope.clone()
                })
            })
            .collect()
    }

    pub(crate) fn update_envelope(&mut self, action: EnvelopeAction) -> Task {
        use EnvelopeAction as A;
        match action {
            A::Close => {
                self.envelope_panel = None;
                // The Merge panel, when it was open behind, shows again.
                if self.merge_panel.as_ref().is_none_or(|panel| !panel.shown) && self.rail_open() {
                    self.merge_panel = None;
                    self.toggle_rail();
                }
            }
            A::Size(size) => {
                self.preferences.envelope.size = size.min(SIZES.len());
            }
            A::CustomWidth(text) | A::CustomHeight(text) if self.envelope_panel.is_none() => {
                let _ = text;
            }
            A::CustomWidth(text) => {
                let value = inches_of(&text);
                if let Some(panel) = self.envelope_panel.as_mut() {
                    panel.custom.0 = text;
                }
                if let Some(value) =
                    value.filter(|value| (CUSTOM_RANGE.0..=CUSTOM_RANGE.1).contains(value))
                {
                    self.preferences.envelope.custom.0 = value;
                }
            }
            A::CustomHeight(text) => {
                let value = inches_of(&text);
                if let Some(panel) = self.envelope_panel.as_mut() {
                    panel.custom.1 = text;
                }
                if let Some(value) =
                    value.filter(|value| (CUSTOM_RANGE.0..=CUSTOM_RANGE.1).contains(value))
                {
                    self.preferences.envelope.custom.1 = value;
                }
            }
            A::Recipient(index, text) => {
                if let Some(panel) = self.envelope_panel.as_mut() {
                    if panel.recipient.len() <= index {
                        panel.recipient.resize(index + 1, String::new());
                    }
                    panel.recipient[index] = text;
                }
            }
            A::ReturnLine(index, text) => {
                if let Some(panel) = self.envelope_panel.as_mut() {
                    if panel.return_lines.len() <= index {
                        panel.return_lines.resize(index + 1, String::new());
                    }
                    panel.return_lines[index] = text;
                }
            }
            A::ReturnChosen(index) => {
                if let Some((name, lines)) = self.preferences.return_addresses.get(index).cloned() {
                    self.preferences.envelope.last_return = name.clone();
                    if let Some(panel) = self.envelope_panel.as_mut() {
                        panel.return_lines = lines.lines().map(str::to_string).collect();
                        panel.return_name = name;
                    }
                }
            }
            A::ReturnName(text) => {
                if let Some(panel) = self.envelope_panel.as_mut() {
                    panel.return_name = text;
                }
            }
            A::SaveReturn => {
                let Some((name, lines)) = self.envelope_panel.as_ref().map(|panel| {
                    (
                        panel.return_name.trim().to_string(),
                        panel
                            .return_lines
                            .iter()
                            .take(RETURN_LINES)
                            .map(|line| line.trim_end().to_string())
                            .collect::<Vec<_>>()
                            .join("\n")
                            .trim_end()
                            .to_string(),
                    )
                }) else {
                    return Task::none();
                };
                if name.is_empty() {
                    return Task::none();
                }
                let saved = &mut self.preferences.return_addresses;
                match saved.iter_mut().find(|(saved, _)| *saved == name) {
                    Some(entry) => entry.1 = lines,
                    None => saved.push((name.clone(), lines)),
                }
                saved.sort_by_key(|(name, _)| name.to_lowercase());
                self.preferences.envelope.last_return = name.clone();
                self.set_envelope_message(format!("Saved the return address {name}"));
            }
            A::DeleteReturn => {
                let name = self.preferences.envelope.last_return.clone();
                self.preferences
                    .return_addresses
                    .retain(|(saved, _)| *saved != name);
                self.preferences.envelope.last_return.clear();
                if let Some(panel) = self.envelope_panel.as_mut() {
                    panel.return_name.clear();
                }
            }
            A::PrintReturn(on) => self.preferences.envelope.print_return = on,
            A::Feed(feed) => self.preferences.envelope.feed = feed,
            A::Offset(index, text) => {
                let value = inches_of(&text);
                if let Some(panel) = self.envelope_panel.as_mut() {
                    panel.offsets[index] = text;
                }
                if let Some(value) = value.filter(|value| value.abs() <= 2_880) {
                    let options = &mut self.preferences.envelope;
                    match index {
                        0 => options.recipient_offset.0 = value,
                        1 => options.recipient_offset.1 = value,
                        2 => options.return_offset.0 = value,
                        _ => options.return_offset.1 = value,
                    }
                }
            }
            A::NewDocument => {
                if let Some(envelope) = self.envelope() {
                    let document = self.envelope_on_sheet(&envelope);
                    self.open_in_new_tab(document, "The envelope");
                }
            }
            A::MergeEnvelopes => {
                let envelopes = self.merged_envelopes();
                let count = envelopes.len();
                let records: Vec<amipro_editing::merge::MergedRecord> = envelopes
                    .into_iter()
                    .enumerate()
                    .map(|(index, document)| amipro_editing::merge::MergedRecord {
                        document,
                        records: vec![index],
                        number: index as u32 + 1,
                    })
                    .collect();
                match amipro_editing::merge::joined(&records) {
                    Some(document) => self.open_in_new_tab(
                        document,
                        &format!(
                            "Merged {count} envelope{}",
                            if count == 1 { "" } else { "s" }
                        ),
                    ),
                    None => self.set_envelope_message("No record to merge."),
                }
            }
            A::Pdf => {
                let folder = self.start_folder();
                return cosmic::task::future(async move {
                    let mut dialog = file_chooser::save::Dialog::new()
                        .title("Envelope PDF".to_owned())
                        .file_name("Envelope.pdf".to_owned())
                        .filter(FileFilter::new("PDF documents (.pdf)").glob("*.pdf"));
                    if let Some(folder) = folder {
                        dialog = dialog.directory(folder);
                    }
                    let path = dialog.save_file().await.ok().and_then(|response| {
                        response.url().and_then(|url| url.to_file_path().ok())
                    });
                    match path {
                        Some(path) => Message::Envelope(EnvelopeAction::PdfChosen(path)),
                        None => Message::Repaint,
                    }
                });
            }
            A::PdfChosen(path) => {
                let Some(job) = self.envelope_job(None) else {
                    return Task::none();
                };
                let job = crate::print::ExportJob { job, path };
                let session_id = self.id;
                return cosmic::task::future(async move {
                    let result = crate::print::in_background(move || job.run()).await;
                    Message::PdfExported { session_id, result }
                });
            }
            A::Print => {
                let Some(envelope) = self.envelope() else {
                    return Task::none();
                };
                if let Some(panel) = self.envelope_panel.as_mut() {
                    panel.printing = true;
                }
                let first = (envelope.width as f32, envelope.height as f32);
                let session_id = self.id;
                return cosmic::task::future(async move {
                    let result =
                        crate::print_portal::prepare("Envelope".into(), first, 1, 0, false).await;
                    Message::PrintPrepared { session_id, result }
                });
            }
        }
        Task::none()
    }

    /// The print dialog answered while the envelope waits: its page sent.
    pub(crate) fn print_envelope(
        &mut self,
        prepared: &crate::print_portal::Prepared,
    ) -> Option<Task> {
        let panel = self.envelope_panel.as_mut()?;
        if !panel.printing {
            return None;
        }
        panel.printing = false;
        let job = self.envelope_job(prepared.paper)?;
        let token = prepared.token;
        self.status = "Printing the envelope…".into();
        let session_id = self.id;
        Some(cosmic::task::future(async move {
            let result = match crate::print::in_background(move || job.render()).await {
                Ok(pdf) => crate::print_portal::send("Envelope".into(), pdf, token)
                    .await
                    .map(|()| 1),
                Err(error) => Err(error),
            };
            Message::Printed { session_id, result }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_survive_the_settings() {
        let options = EnvelopeOptions {
            size: 7,
            custom: (10_000, 5_000),
            print_return: false,
            feed: Feed::Centre,
            recipient_offset: (144, -72),
            return_offset: (0, 36),
            last_return: "Home".into(),
        };
        assert_eq!(
            EnvelopeOptions::from_numbers(&options.to_numbers(), "Home".into()),
            Some(options)
        );
        assert_eq!(inches_of("0.25"), Some(360));
        assert_eq!(inches_text(360), "0.25");
        assert_eq!(inches_text(0), "0");
    }

    #[test]
    fn custom_sizes_keep_ami_pros_range() {
        let options = EnvelopeOptions {
            size: SIZES.len(),
            custom: (9_000, 4_000),
            ..EnvelopeOptions::default()
        };
        assert_eq!(options.size(), (9_000, 4_000));
        assert!(CUSTOM_RANGE.0 <= 4_320 && CUSTOM_RANGE.1 == 15_840);
    }
}
