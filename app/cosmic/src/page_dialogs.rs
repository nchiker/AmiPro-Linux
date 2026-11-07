//! The one-shot page dialogs (`UI_SPEC.md` §18, decision L3; §20,
//! decision F1; §21, decision B4): Insert › Floating Header/Footer…,
//! Insert › Page Number…, Insert › Frame…, and Insert › Bullet…. Each does
//! one thing and closes, as Ami Pro's Floating Header/Footer, Page
//! Numbering, Create Frame, and Insert Bullet dialogs do
//! (`docs/research/PAGE_LAYOUT_NOTES.md`, VM fixtures FH1–FH3, PN1–PN3;
//! `docs/research/FRAME_CREATION_NOTES.md`, FC1–FC9;
//! `docs/research/BULLETS_NOTES.md`, BL1).

use amipro_document::{
    Document, HeaderFooterKind, InlineObject, OBJECT_REPLACEMENT_CHARACTER, PageNumberField,
    PageSides, Rect, Twips,
};
use amipro_editing::TextPosition;
use amipro_editing::frame_creation::FrameGeometry;
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::page_editing::BULLET_NAMES;

/// The objects whose character is next to `at` (before or after it): the
/// ones Ami Pro counts as "at the caret".
fn objects_at(document: &Document, at: TextPosition) -> Vec<(TextPosition, InlineObject)> {
    let Some(paragraph) = document
        .flow(at.flow)
        .and_then(|flow| flow.paragraphs.get(at.paragraph))
    else {
        return Vec::new();
    };
    let width = OBJECT_REPLACEMENT_CHARACTER.len_utf8();
    paragraph
        .object_offsets()
        .zip(&paragraph.objects)
        .filter(|(offset, _)| *offset == at.byte || offset + width == at.byte)
        .map(|(byte, object)| (TextPosition { byte, ..at }, object.clone()))
        .collect()
}

// ----- Floating Header/Footer -----

/// Insert › Floating Header/Footer… (Ami Pro's dialog, `13c0:0000`).
#[derive(Clone, Debug, PartialEq)]
pub struct FloatingDialog {
    /// Insert (false) or Remove (true).
    pub remove: bool,
    pub kind: HeaderFooterKind,
    pub pages: PageSides,
    /// The floating header and footer at the caret, which Remove takes.
    pub here: Vec<(HeaderFooterKind, TextPosition)>,
    pub remove_header: bool,
    pub remove_footer: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FloatingAction {
    Remove(bool),
    Kind(HeaderFooterKind),
    Pages(PageSides),
    RemoveHeader(bool),
    RemoveFooter(bool),
    Ok,
    Cancel,
}

/// What the dialog asks the app to do when it closes.
#[derive(Clone, Debug, PartialEq)]
pub enum FloatingOutcome {
    Stay,
    Cancel,
    Insert {
        kind: HeaderFooterKind,
        pages: PageSides,
    },
    /// The marks to delete.
    Remove(Vec<TextPosition>),
}

impl FloatingDialog {
    /// The dialog with the caret at `at`: on Remove, with each kind there
    /// checked, when a floating header or footer is at the caret (FH3);
    /// otherwise on Insert, Floating header, All pages.
    pub fn new(document: &Document, at: TextPosition) -> Self {
        let here: Vec<_> = objects_at(document, at)
            .into_iter()
            .filter_map(|(position, object)| match object {
                InlineObject::FloatingHeaderFooter { kind, .. } => Some((kind, position)),
                _ => None,
            })
            .collect();
        let has = |wanted| here.iter().any(|(kind, _)| *kind == wanted);
        Self {
            remove: !here.is_empty(),
            kind: HeaderFooterKind::Header,
            pages: PageSides::All,
            remove_header: has(HeaderFooterKind::Header),
            remove_footer: has(HeaderFooterKind::Footer),
            here,
        }
    }

    fn has(&self, wanted: HeaderFooterKind) -> bool {
        self.here.iter().any(|(kind, _)| *kind == wanted)
    }

    pub fn update(&mut self, action: FloatingAction) -> FloatingOutcome {
        match action {
            FloatingAction::Remove(remove) => self.remove = remove && !self.here.is_empty(),
            FloatingAction::Kind(kind) => self.kind = kind,
            FloatingAction::Pages(pages) => self.pages = pages,
            FloatingAction::RemoveHeader(on) => {
                self.remove_header = on && self.has(HeaderFooterKind::Header);
            }
            FloatingAction::RemoveFooter(on) => {
                self.remove_footer = on && self.has(HeaderFooterKind::Footer);
            }
            FloatingAction::Cancel => return FloatingOutcome::Cancel,
            FloatingAction::Ok if self.remove => {
                let marks = self
                    .here
                    .iter()
                    .filter(|(kind, _)| match kind {
                        HeaderFooterKind::Header => self.remove_header,
                        HeaderFooterKind::Footer => self.remove_footer,
                    })
                    .map(|(_, position)| *position)
                    .collect();
                return FloatingOutcome::Remove(marks);
            }
            FloatingAction::Ok => {
                return FloatingOutcome::Insert {
                    kind: self.kind,
                    pages: self.pages,
                };
            }
        }
        FloatingOutcome::Stay
    }

    pub fn view(&self) -> Element<'static, Message> {
        let send = Message::FloatingHeaderFooter;
        let remove = self.remove;
        let choice = |text: &'static str,
                      value: bool,
                      enabled: bool|
         -> Element<'static, Message> {
            let radio = widget::radio(text, value, Some(remove), move |value| {
                send(FloatingAction::Remove(value))
            });
            if enabled {
                radio.into()
            } else {
                widget::text::caption("Remove: there is no floating header or footer at the caret.")
                    .into()
            }
        };
        let kind = self.kind;
        let kinds = |text: &'static str, value| -> Element<'static, Message> {
            widget::radio(text, value, Some(kind), move |value| {
                send(FloatingAction::Kind(value))
            })
            .into()
        };
        let pages = self.pages;
        let sides = |text: &'static str, value| -> Element<'static, Message> {
            widget::radio(text, value, Some(pages), move |value| {
                send(FloatingAction::Pages(value))
            })
            .into()
        };
        let mut controls: Vec<Element<'static, Message>> =
            vec![choice("Insert at the caret", false, true)];
        if !remove {
            controls.push(
                widget::container(
                    widget::column::with_children(vec![
                        kinds("Floating header", HeaderFooterKind::Header),
                        kinds("Floating footer", HeaderFooterKind::Footer),
                        widget::text::caption("Apply to").into(),
                        sides("All pages", PageSides::All),
                        sides("Right pages", PageSides::Right),
                        sides("Left pages", PageSides::Left),
                    ])
                    .spacing(6),
                )
                .padding([0, 0, 0, 24])
                .into(),
            );
        }
        let removable = !self.here.is_empty();
        controls.push(choice("Remove the one at the caret", true, removable));
        if remove {
            let check = |text: &'static str,
                         on: bool,
                         enabled: bool,
                         action: fn(bool) -> FloatingAction|
             -> Element<'static, Message> {
                let check = widget::checkbox(on).label(text);
                if enabled {
                    check.on_toggle(move |on| send(action(on))).into()
                } else {
                    check.into()
                }
            };
            controls.push(
                widget::container(
                    widget::column::with_children(vec![
                        check(
                            "Floating header",
                            self.remove_header,
                            self.has(HeaderFooterKind::Header),
                            FloatingAction::RemoveHeader,
                        ),
                        check(
                            "Floating footer",
                            self.remove_footer,
                            self.has(HeaderFooterKind::Footer),
                            FloatingAction::RemoveFooter,
                        ),
                    ])
                    .spacing(6),
                )
                .padding([0, 0, 0, 24])
                .into(),
            );
        }
        let ready = !remove || self.remove_header || self.remove_footer;
        widget::dialog()
            .title("Floating Header/Footer")
            .body(
                "A floating header or footer replaces the page layout's own from the \
                 next page on.",
            )
            .control(widget::column::with_children(controls).spacing(8))
            .primary_action(
                widget::button::suggested(if remove { "Remove" } else { "Insert" })
                    .on_press_maybe(ready.then_some(send(FloatingAction::Ok))),
            )
            .secondary_action(
                widget::button::standard("Cancel").on_press(send(FloatingAction::Cancel)),
            )
            .into()
    }
}

// ----- Page Number -----

/// Insert › Page Number… (Ami Pro's Page Numbering, `1140:0000`): inserts
/// a page-number field at the caret, or changes the one there.
#[derive(Clone, Debug, PartialEq)]
pub struct PageNumberDialog {
    /// The field at the caret, which the dialog changes, if any.
    pub existing: Option<TextPosition>,
    /// Index into `PageNumberField::STYLES`.
    pub style: usize,
    pub start_page_on: bool,
    pub start_page: String,
    pub start_number_on: bool,
    pub start_number: String,
    pub leading_text: String,
    /// The page the caret is on, for the sample.
    pub page: u32,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PageNumberAction {
    Style(usize),
    StartPageOn(bool),
    StartPage(String),
    StartNumberOn(bool),
    StartNumber(String),
    LeadingText(String),
    Ok,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PageNumberOutcome {
    Stay,
    Cancel,
    Insert(PageNumberField),
    Change(TextPosition, PageNumberField),
}

/// Ami Pro's styles with what each shows for 1 to 3.
const STYLE_NAMES: [&str; 5] = ["1, 2, 3", "I, II, III", "i, ii, iii", "A, B, C", "a, b, c"];

impl PageNumberDialog {
    /// The dialog with the caret at `at` on page `page`: the field there
    /// to change, or Ami Pro's defaults (style 1, start boxes clear and
    /// showing the current page and 1).
    pub fn new(document: &Document, at: TextPosition, page: u32) -> Self {
        let existing = objects_at(document, at)
            .into_iter()
            .find_map(|(position, object)| match object {
                InlineObject::PageNumber(field) => Some((position, field)),
                _ => None,
            });
        let field = existing
            .as_ref()
            .map_or_else(PageNumberField::plain, |(_, field)| field.clone());
        Self {
            existing: existing.map(|(position, _)| position),
            style: PageNumberField::STYLES
                .iter()
                .position(|style| *style == field.style)
                .unwrap_or(0),
            start_page_on: field.start_page != 0,
            start_page: if field.start_page != 0 {
                field.start_page.to_string()
            } else {
                page.to_string()
            },
            start_number_on: field.start_number != 0,
            start_number: field.start_number.max(1).to_string(),
            leading_text: field.leading_text,
            page,
            error: None,
        }
    }

    /// The field the dialog describes, or why it cannot be made.
    pub fn field(&self) -> Result<PageNumberField, String> {
        let number = |on: bool, text: &str, what: &str| -> Result<u32, String> {
            if !on {
                return Ok(0);
            }
            match text.trim().parse::<u32>() {
                Ok(value) if (1..=9_999).contains(&value) => Ok(value),
                _ => Err(format!("{what} is a number from 1 to 9999.")),
            }
        };
        if self.leading_text.chars().count() > 30 {
            return Err("Leading text is at most 30 characters.".into());
        }
        Ok(PageNumberField {
            style: PageNumberField::STYLES[self.style.min(4)],
            start_page: number(self.start_page_on, &self.start_page, "Start on page")?,
            start_number: number(
                self.start_number_on,
                &self.start_number,
                "Start with number",
            )?,
            leading_text: self.leading_text.clone(),
        })
    }

    pub fn update(&mut self, action: PageNumberAction) -> PageNumberOutcome {
        self.error = None;
        match action {
            PageNumberAction::Style(style) => self.style = style.min(4),
            PageNumberAction::StartPageOn(on) => self.start_page_on = on,
            PageNumberAction::StartPage(text) => {
                self.start_page = text;
                self.start_page_on = true;
            }
            PageNumberAction::StartNumberOn(on) => self.start_number_on = on,
            PageNumberAction::StartNumber(text) => {
                self.start_number = text;
                self.start_number_on = true;
            }
            PageNumberAction::LeadingText(text) => {
                self.leading_text = text.chars().filter(|c| !c.is_control()).take(30).collect();
            }
            PageNumberAction::Cancel => return PageNumberOutcome::Cancel,
            PageNumberAction::Ok => match self.field() {
                Ok(field) => {
                    return match self.existing {
                        Some(at) => PageNumberOutcome::Change(at, field),
                        None => PageNumberOutcome::Insert(field),
                    };
                }
                Err(reason) => self.error = Some(reason),
            },
        }
        PageNumberOutcome::Stay
    }

    pub fn view(&self) -> Element<'static, Message> {
        let send = Message::PageNumber;
        let sample = match self.field() {
            Ok(field) => {
                let shown = field.number_on(self.page).map_or_else(
                    || "nothing (before the start page)".to_owned(),
                    |number| {
                        let code = match field.style {
                            'I' => '\u{1B}',
                            'i' => '\u{1C}',
                            'A' => '\u{1D}',
                            'a' => '\u{1E}',
                            _ => '\u{1A}',
                        };
                        format!(
                            "“{}{}”",
                            field.leading_text,
                            amipro_layout::bullets::number_text(code, number).unwrap_or_default()
                        )
                    },
                );
                format!("On this page (page {}) it shows {shown}.", self.page)
            }
            Err(reason) => reason,
        };
        let number_row = |text: &'static str,
                          on: bool,
                          value: String,
                          toggle: fn(bool) -> PageNumberAction,
                          edit: fn(String) -> PageNumberAction|
         -> Element<'static, Message> {
            widget::row::with_children(vec![
                widget::checkbox(on)
                    .label(text)
                    .on_toggle(move |on| send(toggle(on)))
                    .width(Length::Fixed(170.0))
                    .into(),
                widget::text_input("", value)
                    .on_input(move |text| send(edit(text)))
                    .on_submit(move |_| send(PageNumberAction::Ok))
                    .width(Length::Fixed(80.0))
                    .into(),
            ])
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .into()
        };
        let mut controls: Vec<Element<'static, Message>> = vec![
            widget::row::with_children(vec![
                widget::text::body("Style")
                    .width(Length::Fixed(170.0))
                    .into(),
                widget::dropdown(STYLE_NAMES.to_vec(), Some(self.style), move |index| {
                    send(PageNumberAction::Style(index))
                })
                .width(Length::Fixed(120.0))
                .into(),
            ])
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .into(),
            number_row(
                "Start on page",
                self.start_page_on,
                self.start_page.clone(),
                PageNumberAction::StartPageOn,
                PageNumberAction::StartPage,
            ),
            number_row(
                "Start with number",
                self.start_number_on,
                self.start_number.clone(),
                PageNumberAction::StartNumberOn,
                PageNumberAction::StartNumber,
            ),
            widget::text::body("Leading text").into(),
            widget::text_input("", self.leading_text.clone())
                .on_input(move |text| send(PageNumberAction::LeadingText(text)))
                .on_submit(move |_| send(PageNumberAction::Ok))
                .width(Length::Fill)
                .into(),
            widget::text::caption(sample).into(),
        ];
        if let Some(error) = &self.error {
            controls.push(widget::text::caption(error.clone()).into());
        }
        let changing = self.existing.is_some();
        widget::dialog()
            .title(if changing {
                "Change Page Number"
            } else {
                "Insert Page Number"
            })
            .body(
                "The page number goes in at the caret, usually in a header or footer, and \
                 shows each page's number.",
            )
            .control(widget::column::with_children(controls).spacing(8))
            .primary_action(
                widget::button::suggested(if changing { "Change" } else { "Insert" })
                    .on_press(send(PageNumberAction::Ok)),
            )
            .secondary_action(
                widget::button::standard("Cancel").on_press(send(PageNumberAction::Cancel)),
            )
            .into()
    }
}

// ----- Create Frame -----

/// One of the Create Frame dialog's values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometryField {
    Width,
    Height,
    Top,
    Left,
}

/// Insert › Frame… (Ami Pro's Create Frame dialog, `ADDFRAMEDLG`
/// `1360:0000`): a frame's size and position from the paper's top-left
/// corner, starting from the last frame made.
#[derive(Clone, Debug, PartialEq)]
pub struct CreateFrameDialog {
    /// The page the frame goes on: the page in view (FC9).
    pub page: u32,
    /// That page's paper.
    pub paper: (Twips, Twips),
    pub width: String,
    pub height: String,
    pub top: String,
    pub left: String,
    /// The value refused last, and why.
    pub error: Option<(GeometryField, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CreateFrameAction {
    Input(GeometryField, String),
    Create,
    /// Draw on Page: Ami Pro's Manual.
    Draw,
    Cancel,
}

/// What the dialog asks the app to do.
#[derive(Clone, Debug, PartialEq)]
pub enum CreateFrameOutcome {
    Stay,
    Cancel,
    Create { page: u32, rect: Rect },
    Draw,
}

impl CreateFrameDialog {
    /// The dialog for page `page` of paper `paper`, with the last frame made
    /// (`remembered`) or Ami Pro's first values, kept on the paper
    /// (`1128:0000`, `1360:01d7`).
    pub fn new(page: u32, paper: (Twips, Twips), remembered: Option<FrameGeometry>) -> Self {
        let start = FrameGeometry::starting(remembered, paper.0, paper.1);
        let show = crate::style_properties::inches;
        Self {
            page,
            paper,
            width: show(start.width),
            height: show(start.height),
            top: show(start.top),
            left: show(start.left),
            error: None,
        }
    }

    /// The frame the values describe, or the first value refused and why:
    /// each within the limits that keep the frame on the paper
    /// (`1360:0382`; FC5).
    pub fn geometry(&self) -> Result<FrameGeometry, (GeometryField, String)> {
        use GeometryField::{Height, Left, Top, Width};
        let read = |field: GeometryField, text: &str| {
            crate::lengths::parse_length(text).ok_or_else(|| {
                (
                    field,
                    "Type a length such as 2.00, 5 cm, 50 mm, or 144 pt.".to_owned(),
                )
            })
        };
        let geometry = FrameGeometry {
            width: read(Width, &self.width)?,
            height: read(Height, &self.height)?,
            top: read(Top, &self.top)?,
            left: read(Left, &self.left)?,
        };
        let limits = geometry.limits(self.paper.0, self.paper.1);
        let show = crate::style_properties::inches;
        let check =
            |field: GeometryField, value: Twips, (low, high): (Twips, Twips), name: &str| {
                if value < low {
                    Err((field, format!("{name} must be more than 0.")))
                } else if value > high {
                    Err((
                        field,
                        format!(
                            "{name} is at most {} in here, so that the frame stays on the page.",
                            show(high.max(0))
                        ),
                    ))
                } else {
                    Ok(())
                }
            };
        check(Width, geometry.width, limits.width, "Width")?;
        check(Height, geometry.height, limits.height, "Height")?;
        check(Top, geometry.top, limits.top, "Down from top")?;
        check(Left, geometry.left, limits.left, "In from left")?;
        Ok(geometry)
    }

    pub fn update(&mut self, action: CreateFrameAction) -> CreateFrameOutcome {
        match action {
            CreateFrameAction::Input(field, text) => {
                if self
                    .error
                    .as_ref()
                    .is_some_and(|(failed, _)| *failed == field)
                {
                    self.error = None;
                }
                *match field {
                    GeometryField::Width => &mut self.width,
                    GeometryField::Height => &mut self.height,
                    GeometryField::Top => &mut self.top,
                    GeometryField::Left => &mut self.left,
                } = text;
            }
            CreateFrameAction::Create => match self.geometry() {
                Ok(geometry) => {
                    return CreateFrameOutcome::Create {
                        page: self.page,
                        rect: geometry.rect(),
                    };
                }
                Err(refused) => self.error = Some(refused),
            },
            CreateFrameAction::Draw => return CreateFrameOutcome::Draw,
            CreateFrameAction::Cancel => return CreateFrameOutcome::Cancel,
        }
        CreateFrameOutcome::Stay
    }

    pub fn view(&self) -> Element<'static, Message> {
        let send = Message::CreateFrame;
        let error = self.error.clone();
        let row = |label: &'static str, field: GeometryField, value: String| {
            let mut column = vec![
                widget::row::with_children(vec![
                    widget::text::body(label).width(Length::Fixed(120.0)).into(),
                    widget::text_input("", value)
                        .on_input(move |text| send(CreateFrameAction::Input(field, text)))
                        .on_submit(move |_| send(CreateFrameAction::Create))
                        .width(Length::Fixed(110.0))
                        .into(),
                ])
                .spacing(8)
                .align_y(cosmic::iced::Alignment::Center)
                .into(),
            ];
            if let Some((failed, reason)) = &error
                && *failed == field
            {
                column.push(widget::text::caption(reason.clone()).into());
            }
            widget::column::with_children(column).spacing(4).into()
        };
        let controls: Vec<Element<'static, Message>> = vec![
            widget::text::heading("Size").into(),
            row("Width", GeometryField::Width, self.width.clone()),
            row("Height", GeometryField::Height, self.height.clone()),
            widget::text::heading("Position on page").into(),
            row("Down from top", GeometryField::Top, self.top.clone()),
            row("In from left", GeometryField::Left, self.left.clone()),
        ];
        widget::dialog()
            .title("Create Frame")
            .body(format!(
                "A new text frame on page {}, measured in inches from the paper's top \
                 and left edges. Draw on Page draws one with the pointer instead.",
                self.page
            ))
            .control(widget::column::with_children(controls).spacing(8))
            .primary_action(
                widget::button::suggested("Create").on_press(send(CreateFrameAction::Create)),
            )
            .secondary_action(
                widget::button::standard("Cancel").on_press(send(CreateFrameAction::Cancel)),
            )
            .tertiary_action(
                widget::button::text("Draw on Page").on_press(send(CreateFrameAction::Draw)),
            )
            .into()
    }
}

// ----- Insert Bullet -----

/// Insert › Bullet… (Ami Pro's Edit › Insert › Bullet…, dialog 1116,
/// `1560:0122`): one of Ami Pro's 17 bullets goes into the text at the
/// caret, drawn as the page draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct BulletDialog {
    /// Index into `BULLET_NAMES`.
    pub chosen: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BulletAction {
    Choose(usize),
    Insert,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BulletOutcome {
    Stay,
    Cancel,
    Insert(char),
}

impl BulletDialog {
    /// The dialog with the last bullet inserted chosen, or Ami Pro's first
    /// (its list starts on the small dot).
    pub fn new(last: Option<char>) -> Self {
        Self {
            chosen: last
                .and_then(|last| BULLET_NAMES.iter().position(|(code, _)| *code == last))
                .unwrap_or(0),
        }
    }

    pub fn code(&self) -> char {
        BULLET_NAMES[self.chosen.min(BULLET_NAMES.len() - 1)].0
    }

    pub fn update(&mut self, action: BulletAction) -> BulletOutcome {
        match action {
            BulletAction::Choose(index) => {
                self.chosen = index.min(BULLET_NAMES.len() - 1);
                BulletOutcome::Stay
            }
            BulletAction::Insert => BulletOutcome::Insert(self.code()),
            BulletAction::Cancel => BulletOutcome::Cancel,
        }
    }

    pub fn view(&self) -> Element<'static, Message> {
        let send = Message::InsertBullet;
        let rows: Vec<Element<'static, Message>> = BULLET_NAMES
            .chunks(6)
            .enumerate()
            .map(|(row, chunk)| {
                widget::row::with_children(
                    chunk
                        .iter()
                        .enumerate()
                        .map(|(column, (code, name))| {
                            let index = row * 6 + column;
                            widget::tooltip(
                                widget::button::custom(BulletSwatch::new(*code, 28.0))
                                    .class(cosmic::theme::Button::ListItem([6.0; 4]))
                                    .selected(index == self.chosen)
                                    .padding(4)
                                    .on_press(send(BulletAction::Choose(index))),
                                widget::text::caption(*name),
                                widget::tooltip::Position::Bottom,
                            )
                            .into()
                        })
                        .collect::<Vec<_>>(),
                )
                .spacing(4)
                .into()
            })
            .collect();
        let example = widget::row::with_children(vec![
            widget::container(BulletSwatch::new(self.code(), 48.0))
                .padding(4)
                .class(cosmic::theme::Container::Card)
                .into(),
            widget::text::body(BULLET_NAMES[self.chosen].1).into(),
        ])
        .spacing(12)
        .align_y(cosmic::iced::Alignment::Center);
        widget::dialog()
            .title("Insert Bullet")
            .body(
                "The bullet goes in at the caret, in the size and colour of the text there. \
                 For a bullet on every paragraph in a style, use Format › Bullets or the \
                 style's Bullets and numbers.",
            )
            .control(
                widget::column::with_children(vec![
                    widget::column::with_children(rows).spacing(4).into(),
                    example.into(),
                ])
                .spacing(12),
            )
            .primary_action(
                widget::button::suggested("Insert").on_press(send(BulletAction::Insert)),
            )
            .secondary_action(
                widget::button::standard("Cancel").on_press(send(BulletAction::Cancel)),
            )
            .into()
    }
}

/// One of Ami Pro's bullets drawn as the page draws it
/// (`page_surface::shape_marks`), centred in a square `size` pixels wide, in
/// the text colour.
struct BulletSwatch {
    code: char,
    size: f32,
}

impl BulletSwatch {
    fn new(code: char, size: f32) -> Self {
        Self { code, size }
    }
}

impl<Message, Theme, Renderer> cosmic::iced::advanced::Widget<Message, Theme, Renderer>
    for BulletSwatch
where
    Renderer: cosmic::iced::advanced::renderer::Renderer,
{
    fn size(&self) -> cosmic::iced::Size<Length> {
        cosmic::iced::Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(
        &mut self,
        _tree: &mut cosmic::iced::advanced::widget::Tree,
        _renderer: &Renderer,
        _limits: &cosmic::iced::advanced::layout::Limits,
    ) -> cosmic::iced::advanced::layout::Node {
        cosmic::iced::advanced::layout::Node::new(cosmic::iced::Size::new(self.size, self.size))
    }

    fn draw(
        &self,
        _tree: &cosmic::iced::advanced::widget::Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &cosmic::iced::advanced::renderer::Style,
        layout: cosmic::iced::advanced::Layout<'_>,
        _cursor: cosmic::iced::mouse::Cursor,
        _viewport: &cosmic::iced::Rectangle,
    ) {
        let bounds = layout.bounds();
        // A font whose drawn bullets fill most of the square: the page
        // draws a bullet in three quarters of the font's ascent (0.9 em).
        let em = self.size * 0.95;
        let width = amipro_layout::inline_bullet_width(self.code, 1.0) / 20.0 * em;
        let drawn = em * 0.9 * 0.75;
        let start = bounds.x + (bounds.width - width) / 2.0;
        let baseline = bounds.y + (bounds.height + drawn) / 2.0;
        for mark in crate::page_surface::shape_marks(self.code, start, width, drawn, baseline) {
            crate::page_surface::fill_bullet_mark(renderer, mark, style.text_color);
        }
    }
}

impl<'a, Message: 'a> From<BulletSwatch> for Element<'a, Message> {
    fn from(swatch: BulletSwatch) -> Self {
        Element::new(swatch)
    }
}

// ----- Outline Numbering -----

/// A whole numbering scheme for the document's levelled styles
/// (`UI_SPEC.md` §22, O3; Ami Pro's Quick Numbering, `12c8:06d6`,
/// `12c8:0829`, `12c8:0661`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumberingScheme {
    /// 1. A. i. a. 1) a) i) (1) (a), as nested lists number (O2).
    Lists,
    /// Ami Pro's I, A, 1, i, a: I. A. 1. i. a. 1) i) a) (1).
    Outline,
    /// Ami Pro's 1, 1.1, 1.1.1: cumulative.
    Legal,
    /// No numbers, no reset options.
    Clear,
}

impl NumberingScheme {
    pub const ALL: [(Self, &'static str); 4] = [
        (Self::Lists, "1.  A.  i.  a. …"),
        (Self::Outline, "I.  A.  1.  i.  a. … (Ami Pro's)"),
        (Self::Legal, "1  1.1  1.1.1 … (Ami Pro's)"),
        (Self::Clear, "No numbers"),
    ];

    /// The number (code and what follows it) and the cumulative prefix this
    /// scheme gives `level` (1–9).
    fn number(self, level: u8) -> Option<(String, char, String)> {
        let index = usize::from(level.clamp(1, 9) - 1);
        let (before, code, after) = match self {
            Self::Lists => [
                ("", '\u{1A}', "."),
                ("", '\u{1D}', "."),
                ("", '\u{1C}', "."),
                ("", '\u{1E}', "."),
                ("", '\u{1A}', ")"),
                ("", '\u{1E}', ")"),
                ("", '\u{1C}', ")"),
                ("(", '\u{1A}', ")"),
                ("(", '\u{1E}', ")"),
            ][index],
            Self::Outline => [
                ("", '\u{1B}', "."),
                ("", '\u{1D}', "."),
                ("", '\u{1A}', "."),
                ("", '\u{1C}', "."),
                ("", '\u{1E}', "."),
                ("", '\u{1A}', ")"),
                ("", '\u{1C}', ")"),
                ("", '\u{1E}', ")"),
                ("(", '\u{1A}', ")"),
            ][index],
            Self::Legal => ("", '\u{1A}', "."),
            Self::Clear => return None,
        };
        Some((before.to_owned(), code, after.to_owned()))
    }

    /// `style` with this scheme's number for its level (its Text and bullet
    /// kept, as Ami Pro replaces only the number), or `None` when it has no
    /// level.
    pub fn applied(
        self,
        style: &amipro_document::ParagraphStyle,
    ) -> Option<amipro_document::ParagraphStyle> {
        let level = style.bullets.level;
        if level == 0 {
            return None;
        }
        let mut parts =
            crate::style_properties::BulletParts::split(&style.bullets.text).unwrap_or_default();
        parts.number = self.number(level);
        parts.prefix = if self == Self::Legal {
            "\u{1A}.".repeat(usize::from(level - 1))
        } else {
            String::new()
        };
        let mut changed = style.clone();
        changed.bullets.text = parts.join();
        changed.bullets.flags &= !(2 | 4 | 8);
        changed.bullets.flags |= match self {
            Self::Lists | Self::Outline => 2,
            Self::Legal => 2 | 8,
            Self::Clear => 0,
        };
        Some(changed)
    }
}

/// Format › Styles ▸ Outline Numbering…: one scheme for every style with an
/// outline level, one undo step.
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineNumberingDialog {
    pub scheme: NumberingScheme,
    /// The levelled styles it changes, as "Name (level)".
    pub styles: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutlineNumberingAction {
    Choose(NumberingScheme),
    Apply,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OutlineNumberingOutcome {
    Stay,
    Cancel,
    Apply(NumberingScheme),
}

impl OutlineNumberingDialog {
    pub fn new(document: &Document) -> Self {
        let mut styles: Vec<(u8, String)> = document
            .styles()
            .values()
            .filter(|style| style.bullets.level > 0)
            .map(|style| (style.bullets.level, style.name.clone()))
            .collect();
        styles.sort();
        Self {
            scheme: NumberingScheme::Lists,
            styles: styles
                .into_iter()
                .map(|(level, name)| format!("{name} ({level})"))
                .collect(),
        }
    }

    pub fn update(&mut self, action: OutlineNumberingAction) -> OutlineNumberingOutcome {
        match action {
            OutlineNumberingAction::Choose(scheme) => {
                self.scheme = scheme;
                OutlineNumberingOutcome::Stay
            }
            OutlineNumberingAction::Apply if !self.styles.is_empty() => {
                OutlineNumberingOutcome::Apply(self.scheme)
            }
            OutlineNumberingAction::Apply => OutlineNumberingOutcome::Stay,
            OutlineNumberingAction::Cancel => OutlineNumberingOutcome::Cancel,
        }
    }

    pub fn view(&self) -> Element<'static, Message> {
        let send = Message::OutlineNumbering;
        let mut controls: Vec<Element<'static, Message>> = NumberingScheme::ALL
            .iter()
            .map(|(scheme, label)| {
                widget::radio(
                    widget::text::body(*label),
                    *scheme,
                    Some(self.scheme),
                    move |scheme| send(OutlineNumberingAction::Choose(scheme)),
                )
                .into()
            })
            .collect();
        controls.push(
            widget::text::caption(if self.styles.is_empty() {
                "No style has an outline level yet: set one in Style Properties › Bullets and \
                 numbers, or demote a paragraph."
                    .to_owned()
            } else {
                format!("Styles with a level: {}.", self.styles.join(", "))
            })
            .into(),
        );
        widget::dialog()
            .title("Outline Numbering")
            .body(
                "Numbers every style that has an outline level, by its level, and restarts each \
                 level after a higher one. Styles without a level keep their numbers.",
            )
            .control(widget::column::with_children(controls).spacing(8))
            .primary_action(widget::button::suggested("Apply").on_press_maybe(
                (!self.styles.is_empty()).then_some(send(OutlineNumberingAction::Apply)),
            ))
            .secondary_action(
                widget::button::standard("Cancel").on_press(send(OutlineNumberingAction::Cancel)),
            )
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document_with(objects: &[InlineObject]) -> (Document, TextPosition) {
        let mut document = Document::from_plain_text("Text");
        let paragraph = document.paragraph_mut(0);
        for object in objects {
            paragraph.text.push(OBJECT_REPLACEMENT_CHARACTER);
            paragraph.objects.push(object.clone());
        }
        let at = TextPosition {
            flow: document.main_flow_id(),
            paragraph: 0,
            byte: 4,
        };
        (document, at)
    }

    #[test]
    fn floating_headers_insert_or_remove_where_ami_pro_does() {
        // FH1–FH3: Insert with Ami Pro's defaults; at a mark, Remove with
        // its kind checked.
        let (document, at) = document_with(&[]);
        let mut dialog = FloatingDialog::new(&document, at);
        assert!(!dialog.remove);
        dialog.update(FloatingAction::Kind(HeaderFooterKind::Footer));
        dialog.update(FloatingAction::Pages(PageSides::Right));
        dialog.update(FloatingAction::Remove(true));
        assert!(!dialog.remove, "nothing to remove here");
        assert_eq!(
            dialog.update(FloatingAction::Ok),
            FloatingOutcome::Insert {
                kind: HeaderFooterKind::Footer,
                pages: PageSides::Right
            }
        );
        let mut document = Document::from_plain_text("Text");
        let flow = document.create_flow(document.body_text_style());
        let (document, at) = {
            let paragraph = document.paragraph_mut(0);
            paragraph.text.push(OBJECT_REPLACEMENT_CHARACTER);
            paragraph.objects.push(InlineObject::FloatingHeaderFooter {
                kind: HeaderFooterKind::Header,
                pages: PageSides::All,
                flow,
            });
            let at = TextPosition {
                flow: document.main_flow_id(),
                paragraph: 0,
                byte: 7,
            };
            (document, at)
        };
        let mut dialog = FloatingDialog::new(&document, at);
        assert!(dialog.remove && dialog.remove_header && !dialog.remove_footer);
        assert_eq!(
            dialog.update(FloatingAction::Ok),
            FloatingOutcome::Remove(vec![TextPosition { byte: 4, ..at }])
        );
    }

    #[test]
    fn page_numbers_take_what_ami_pros_page_numbering_takes() {
        let (document, at) = document_with(&[]);
        let mut dialog = PageNumberDialog::new(&document, at, 3);
        assert_eq!(
            (dialog.start_page.as_str(), dialog.start_number.as_str()),
            ("3", "1")
        );
        assert_eq!(dialog.field(), Ok(PageNumberField::plain()));
        dialog.update(PageNumberAction::Style(2));
        dialog.update(PageNumberAction::StartPage("2".into()));
        dialog.update(PageNumberAction::LeadingText("Page ".into()));
        assert_eq!(
            dialog.update(PageNumberAction::Ok),
            PageNumberOutcome::Insert(PageNumberField {
                style: 'i',
                start_page: 2,
                start_number: 0,
                leading_text: "Page ".into(),
            })
        );
        dialog.update(PageNumberAction::StartNumber("0".into()));
        assert_eq!(dialog.update(PageNumberAction::Ok), PageNumberOutcome::Stay);
        assert!(dialog.error.as_deref().unwrap().contains("1 to 9999"));
        // At a field, the dialog changes it.
        let field = PageNumberField {
            style: 'A',
            start_page: 2,
            start_number: 5,
            leading_text: "- ".into(),
        };
        let (document, at) = document_with(&[InlineObject::PageNumber(field.clone())]);
        let mut dialog = PageNumberDialog::new(&document, TextPosition { byte: 7, ..at }, 1);
        assert_eq!(dialog.field(), Ok(field.clone()));
        assert_eq!(
            dialog.update(PageNumberAction::Ok),
            PageNumberOutcome::Change(TextPosition { byte: 4, ..at }, field)
        );
    }

    const LETTER: (Twips, Twips) = (12_240, 15_840);

    /// FC1: 2.50 × 1.00 in at 3.00 in down and 1.25 in in, after F-series
    /// frames of 2.00 × 1.50 at 2.00 and 1.50.
    #[test]
    fn create_frame_starts_from_the_last_frame_and_makes_fc1() {
        let remembered = FrameGeometry {
            width: 2_880,
            height: 2_160,
            top: 2_880,
            left: 2_160,
        };
        let mut dialog = CreateFrameDialog::new(1, LETTER, Some(remembered));
        assert_eq!(
            (
                dialog.width.as_str(),
                dialog.height.as_str(),
                dialog.top.as_str(),
                dialog.left.as_str()
            ),
            ("2.00", "1.50", "2.00", "1.50")
        );
        for (field, text) in [
            (GeometryField::Width, "2.50"),
            (GeometryField::Height, "1"),
            (GeometryField::Top, "3 in"),
            (GeometryField::Left, "1.25\""),
        ] {
            assert_eq!(
                dialog.update(CreateFrameAction::Input(field, text.into())),
                CreateFrameOutcome::Stay
            );
        }
        assert_eq!(
            dialog.update(CreateFrameAction::Create),
            CreateFrameOutcome::Create {
                page: 1,
                rect: Rect::new(1_800, 4_320, 5_400, 5_760)
            }
        );
        // Never used: Ami Pro's first values.
        let first = CreateFrameDialog::new(2, LETTER, None);
        assert_eq!(
            (
                first.width.as_str(),
                first.top.as_str(),
                first.left.as_str()
            ),
            ("2.00", "4.50", "3.25")
        );
        assert_eq!(
            first.clone().update(CreateFrameAction::Draw),
            CreateFrameOutcome::Draw
        );
    }

    /// FC5: a 9.00 in width at 1.25 in leaves the page, so it is refused in
    /// place with the reason; nonsense and zero are refused too.
    #[test]
    fn create_frame_refuses_a_frame_off_the_paper_fc5() {
        let mut dialog = CreateFrameDialog::new(1, LETTER, None);
        dialog.update(CreateFrameAction::Input(GeometryField::Left, "1.25".into()));
        dialog.update(CreateFrameAction::Input(
            GeometryField::Width,
            "9.00".into(),
        ));
        assert_eq!(
            dialog.update(CreateFrameAction::Create),
            CreateFrameOutcome::Stay
        );
        let (field, reason) = dialog.error.clone().unwrap();
        assert_eq!(field, GeometryField::Width);
        assert!(reason.contains("at most 7.25 in"), "{reason}");
        // Typing in the refused box clears its reason.
        dialog.update(CreateFrameAction::Input(
            GeometryField::Width,
            "7.25".into(),
        ));
        assert_eq!(dialog.error, None);
        assert!(matches!(
            dialog.update(CreateFrameAction::Create),
            CreateFrameOutcome::Create { .. }
        ));
        dialog.update(CreateFrameAction::Input(GeometryField::Height, "0".into()));
        dialog.update(CreateFrameAction::Create);
        assert_eq!(dialog.error.as_ref().unwrap().0, GeometryField::Height);
        dialog.update(CreateFrameAction::Input(
            GeometryField::Height,
            "two".into(),
        ));
        dialog.update(CreateFrameAction::Create);
        assert_eq!(dialog.error.as_ref().unwrap().0, GeometryField::Height);
        assert_eq!(
            dialog.update(CreateFrameAction::Cancel),
            CreateFrameOutcome::Cancel
        );
    }

    #[test]
    fn insert_bullet_offers_the_last_bullet_and_inserts_the_chosen_one_bl1() {
        // Ami Pro's list starts on the small dot; the dialog offers the
        // bullet put in last.
        let mut dialog = BulletDialog::new(None);
        assert_eq!(dialog.code(), '\u{10}');
        assert_eq!(BulletDialog::new(Some('\u{D}')).code(), '\u{D}');
        assert_eq!(BulletDialog::new(Some('A')).code(), '\u{10}');
        assert_eq!(dialog.update(BulletAction::Choose(13)), BulletOutcome::Stay);
        assert_eq!(dialog.code(), '\u{C}', "the checked box");
        assert_eq!(dialog.update(BulletAction::Choose(99)), BulletOutcome::Stay);
        assert_eq!(dialog.code(), '\u{F}', "the last one");
        assert_eq!(
            dialog.update(BulletAction::Insert),
            BulletOutcome::Insert('\u{F}')
        );
        assert_eq!(dialog.update(BulletAction::Cancel), BulletOutcome::Cancel);
        // Every bullet's name, in Ami Pro's order.
        let codes: Vec<char> = BULLET_NAMES.iter().map(|(code, _)| *code).collect();
        assert_eq!(codes, amipro_document::BULLET_CODES);
    }

    #[test]
    fn numbering_schemes_number_levelled_styles_as_ami_pros_quick_numbering() {
        use amipro_document::{Bullets, ParagraphStyle, StyleId, default_styles};
        let body = default_styles()[&StyleId(1)].clone();
        let at = |level: u8, text: &str| ParagraphStyle {
            bullets: Bullets {
                text: text.into(),
                level,
                flags: 16,
                ..Bullets::default()
            },
            ..body.clone()
        };
        // Ami Pro's I, A, 1, i, a on levels 1, 3, 9 (`_OUTLIN1.STY`).
        let outline = |level: u8| {
            NumberingScheme::Outline
                .applied(&at(level, ""))
                .unwrap()
                .bullets
        };
        assert_eq!(outline(1).text, "\u{1B}.");
        assert_eq!(outline(3).text, "\u{1A}.");
        assert_eq!(outline(9).text, "(\u{1A})");
        assert_eq!(
            outline(1).flags,
            16 | 2,
            "Right align kept, After higher level"
        );
        // 1, 1.1, 1.1.1 (OL2): cumulative.
        let legal = NumberingScheme::Legal
            .applied(&at(3, "\u{1C}."))
            .unwrap()
            .bullets;
        assert_eq!(legal.text, "\u{1A}.\u{1A}.\u{1A}.");
        assert_eq!(legal.flags, 16 | 2 | 8);
        // The lists' 1. A. i.; a style's Text and bullet stay.
        let lists = NumberingScheme::Lists
            .applied(&at(2, "\u{1A}.Note\u{11}"))
            .unwrap()
            .bullets;
        assert_eq!(lists.text, "\u{1D}.Note\u{11}");
        // Clear: the number and the reset options go.
        let clear = NumberingScheme::Clear
            .applied(&at(2, "\u{1A}."))
            .unwrap()
            .bullets;
        assert_eq!((clear.text.as_str(), clear.flags), ("", 16));
        // Styles without a level are left alone (unlike Ami Pro's buttons).
        assert!(NumberingScheme::Lists.applied(&at(0, "\u{1A}.")).is_none());
    }
}
