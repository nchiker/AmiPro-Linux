//! Page Layout Properties in the context rail (`UI_SPEC.md` §18, approved
//! 2026-10-13, decisions L1–L4): Ami Pro's Modify Page Layout edited beside
//! the document, as Style Properties edits a style (§15). Each change
//! applies at once as one `ModifyLayout` command and one undo step, so the
//! document is the preview; a typed value applies on Enter or when its box
//! loses focus, and a value Ami Pro would refuse is refused in place with
//! the reason. The panel follows the caret's page (L4); the Layout list
//! picks another layout until the caret reaches a page with another one.
//!
//! What each setting means, its limits, and how Pages and Mirror work come
//! from the decompiled `AMIPROUI.DLL` (`docs/research/PAGE_LAYOUT_NOTES.md`)
//! and are applied by `amipro_editing::layout_setup`. Lengths are shown and
//! typed in inches, as Ami Pro's dialog shows them by default.

use std::collections::{BTreeMap, BTreeSet};

use amipro_document::{
    Colour, Document, LayoutId, LineNumbering, LineNumberingEvery, LineNumberingMode, LinePosition,
    Sides, TabStop, Twips,
};
use amipro_editing::LayoutSettings;
use amipro_editing::layout_setup::{Band, CUSTOM_PAPER, MarginSide, PAPERS, Pages, Refusal};
use cosmic::iced::Length;
use cosmic::{Element, widget};

use crate::Message;
use crate::colour_control::{Context, Target};
use crate::style_properties::{LEADERS, RULE_STYLES, TAB_KINDS};

/// The sides of the lines around the page, as Ami Pro lists them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineSide {
    All,
    Left,
    Right,
    Top,
    Bottom,
}

/// Ami Pro's line positions, in its order (Inside is the page's default).
const POSITIONS: [(LinePosition, &str); 5] = [
    (LinePosition::Inside, "Inside"),
    (LinePosition::CloseToInside, "Close to inside"),
    (LinePosition::Middle, "Middle"),
    (LinePosition::CloseToOutside, "Close to outside"),
    (LinePosition::Outside, "Outside"),
];

/// The panel's folding sections, in Ami Pro's panel order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Section {
    Page,
    Margins,
    Tabs,
    Lines,
    Header,
    Footer,
    /// Page › Line Numbering (CM7): the document's, whatever the layout.
    LineNumbering,
}

impl Section {
    pub const ALL: [(Self, &'static str); 7] = [
        (Self::Page, "Page"),
        (Self::Margins, "Margins and columns"),
        (Self::Tabs, "Tabs"),
        (Self::Lines, "Lines"),
        (Self::Header, "Header"),
        (Self::Footer, "Footer"),
        (Self::LineNumbering, "Line numbering"),
    ];
}

/// A box the user types a value into; its text is kept as typed.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Field {
    Width,
    Height,
    Margin(MarginSide),
    Gutter,
    Tab(Option<Band>, usize),
    BandMargin(Band, MarginSide),
    BandGutter(Band),
    /// Line numbering's Every N lines.
    LineInterval,
}

/// The panel's state: the layout shown, which pages changes go to, and
/// what is typed in its boxes. The settings themselves are the document's.
#[derive(Clone, Debug, PartialEq)]
pub struct PageLayoutProperties {
    pub layout: LayoutId,
    /// The caret page's layout when last seen: when it changes, the panel
    /// follows it.
    pub caret_layout: Option<LayoutId>,
    /// The header or footer holding the caret when last seen: when the
    /// caret enters one, its section opens (`UI_SPEC.md` §18).
    pub caret_band: Option<Band>,
    /// Ami Pro's Pages group: All, or Right or Left pages.
    pub pages: Pages,
    /// The layout's settings as the panel last saw them in the document.
    pub settings: LayoutSettings,
    /// What is typed in a box, as typed, until it applies.
    pub texts: BTreeMap<Field, String>,
    pub open: BTreeSet<Section>,
    pub error: Option<String>,
    /// What Insert Page Layout gives the next pages: 0 a new copy of the
    /// layout shown, else the document's layout at that place less one
    /// (an earlier layout again, Ami Pro's Revert).
    pub insert_choice: usize,
    /// The document's Line Numbering as the panel last saw it.
    pub numbering: LineNumbering,
}

/// What the panel asks the app to do after an action.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Stay,
    /// The layout's new settings: one `ModifyLayout` command.
    Apply(LayoutId, Box<LayoutSettings>),
    /// Insert Page Layout: a page break at the caret that gives the next
    /// pages a new copy of this layout.
    InsertCopy(LayoutId),
    /// Insert Page Layout with an earlier layout chosen: a page break that
    /// goes back to it (Ami Pro's Revert).
    InsertExisting(LayoutId),
    /// Remove: the inserted layout and the break that switches to it.
    Remove(LayoutId),
    /// Line numbering's new settings: one `SetLineNumbering` command.
    LineNumbering(LineNumbering),
    /// ‹ Styles: back to the Styles list.
    Back,
    /// ×: close the rail.
    Close,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutAction {
    /// Show the document's layout at this index (it stays until the caret
    /// reaches a page with another layout).
    Layout(usize),
    Pages(Pages),
    Mirror(bool),
    Section(Section),
    Text(Field, String),
    Submit(Field),
    /// A paper size by its place in the Size list (Ami Pro's sizes, then
    /// Custom).
    Paper(usize),
    Landscape(bool),
    /// Columns, from 1, for the page (`None`) or a band.
    Columns(Option<Band>, u8),
    /// Uneven columns made even again (`UI_SPEC.md` §19).
    EvenColumns(Option<Band>),
    Balance(Option<Band>, bool),
    TabKind(Option<Band>, usize, usize),
    TabLeader(Option<Band>, usize, usize),
    AddTab(Option<Band>),
    RemoveTab(Option<Band>, usize),
    LineSide(LineSide, bool),
    LineStyle(usize),
    LinePosition(usize),
    ColumnLine(bool),
    ColumnLineStyle(usize),
    LineColour(Colour),
    FromSecondPage(Band, bool),
    InsertChoice(usize),
    Insert,
    Remove,
    /// Line numbering: None, Number all lines, Number text lines (the
    /// dialog's list order).
    NumberingMode(usize),
    NumberingEvery(LineNumberingEvery),
    NumberingReset(bool),
    /// The "Based on" style by its place in the document's style list.
    NumberingStyle(usize),
    Back,
    Close,
}

/// Twips as inches with two decimals, rounded half up, as Ami Pro's boxes
/// show them (`AMIUICNT.DLL` `1060:0261`).
pub fn inches(twips: Twips) -> String {
    crate::style_properties::inches(twips)
}

/// A typed length in twips as Ami Pro's boxes read one (`lengths.rs`).
pub fn parse_length(text: &str) -> Option<Twips> {
    crate::lengths::parse_length(text)
}

impl PageLayoutProperties {
    /// The panel on `layout`, with the caret on a page of `caret_layout`.
    pub fn new(
        document: &Document,
        layout: LayoutId,
        caret_layout: Option<LayoutId>,
    ) -> Option<Self> {
        let settings = LayoutSettings::of(document, layout)?;
        Some(Self {
            layout,
            caret_layout,
            caret_band: None,
            pages: settings.pages(),
            settings,
            texts: BTreeMap::new(),
            open: BTreeSet::from([Section::Page]),
            error: None,
            insert_choice: 0,
            numbering: document.line_numbering().clone(),
        })
    }

    /// Shows `layout`.
    pub fn show(&mut self, document: &Document, layout: LayoutId) {
        let Some(settings) = LayoutSettings::of(document, layout) else {
            return;
        };
        self.layout = layout;
        self.pages = settings.pages();
        self.settings = settings;
        self.texts.clear();
        self.error = None;
    }

    /// After every change to the document or the caret: follows the caret
    /// to a page with another layout, and takes the layout's settings as
    /// the document has them (after an undo, say). Returns `false` when
    /// nothing can be shown.
    pub fn sync(
        &mut self,
        document: &Document,
        caret_layout: Option<LayoutId>,
        caret_band: Option<Band>,
    ) -> bool {
        if *document.line_numbering() != self.numbering {
            self.numbering = document.line_numbering().clone();
            self.texts.remove(&Field::LineInterval);
        }
        if caret_band != self.caret_band {
            self.caret_band = caret_band;
            if let Some(band) = caret_band {
                self.open.insert(match band {
                    Band::Header => Section::Header,
                    Band::Footer => Section::Footer,
                });
            }
        }
        if caret_layout != self.caret_layout {
            self.caret_layout = caret_layout;
            if let Some(caret) = caret_layout
                && caret != self.layout
            {
                self.show(document, caret);
            }
        }
        match LayoutSettings::of(document, self.layout) {
            Some(current) if current == self.settings => true,
            Some(current) => {
                self.settings = current;
                self.texts.clear();
                // Pages shows Right while left pages differ, and All once
                // they are gone (`1248:046c`).
                match (self.pages, self.settings.left.is_some()) {
                    (Pages::All, true) => self.pages = Pages::Right,
                    (Pages::Right | Pages::Left, false) => self.pages = Pages::All,
                    _ => {}
                }
                true
            }
            None => {
                let fallback = caret_layout
                    .filter(|caret| document.layout(*caret).is_some())
                    .unwrap_or(document.document_layout().id);
                self.show(document, fallback);
                document.layout(self.layout).is_some()
            }
        }
    }

    /// The text a box shows: as typed, or the setting.
    pub fn text(&self, field: Field) -> String {
        if let Some(text) = self.texts.get(&field) {
            return text.clone();
        }
        let page = self.settings.page(self.pages);
        match field {
            Field::Width => inches(page.width),
            Field::Height => inches(page.height),
            Field::Margin(side) => inches(margin(&page.layout.margins, side)),
            Field::Gutter => LayoutSettings::gutter_shown(&page.layout)
                .map(inches)
                .unwrap_or_default(),
            Field::Tab(band, index) => self
                .tab_box(band)
                .and_then(|layout| layout.tab_stops().get(index))
                .map(|stop| inches(stop.position))
                .unwrap_or_default(),
            Field::BandMargin(band, side) => self
                .settings
                .band(band, self.pages)
                .map(|layout| inches(margin(&layout.margins, side)))
                .unwrap_or_default(),
            Field::BandGutter(band) => self
                .settings
                .band(band, self.pages)
                .and_then(LayoutSettings::gutter_shown)
                .map(inches)
                .unwrap_or_default(),
            Field::LineInterval => self.numbering.interval.to_string(),
        }
    }

    /// Line numbering with one choice changed: the command's value, or
    /// nothing when it is as it was.
    fn numbering(&mut self, next: LineNumbering) -> Outcome {
        self.error = None;
        if next == self.numbering {
            Outcome::Stay
        } else {
            Outcome::LineNumbering(next)
        }
    }

    /// The layout box whose tabs a tab row edits: the page's, or a band's.
    pub fn tab_box(&self, band: Option<Band>) -> Option<&amipro_document::LayoutBox> {
        match band {
            None => Some(&self.settings.page(self.pages).layout),
            Some(band) => self.settings.band(band, self.pages),
        }
    }

    /// Runs one of `layout_setup`'s changes on a copy of the settings: the
    /// copy applies, or the reason shows and nothing changes.
    fn change(&mut self, edit: impl FnOnce(&mut LayoutSettings) -> Result<(), Refusal>) -> Outcome {
        let mut next = self.settings.clone();
        match edit(&mut next) {
            Ok(()) if next == self.settings => {
                self.error = None;
                Outcome::Stay
            }
            Ok(()) => {
                self.error = None;
                Outcome::Apply(self.layout, Box::new(next))
            }
            Err(reason) => {
                self.error = Some(format!("{reason}."));
                Outcome::Stay
            }
        }
    }

    pub fn update(&mut self, action: LayoutAction, document: &Document) -> Outcome {
        match action {
            LayoutAction::Layout(index) => {
                if let Some(layout) = document.layouts().get(index) {
                    self.show(document, layout.id);
                }
                Outcome::Stay
            }
            LayoutAction::Pages(pages) => {
                if pages == Pages::All && self.settings.left.is_some() {
                    // Every page alike again: the left pages give way.
                    let outcome = self.change(LayoutSettings::set_all_pages);
                    if matches!(outcome, Outcome::Apply(..)) {
                        self.pages = Pages::All;
                    }
                    return outcome;
                }
                self.pages = pages;
                self.texts.clear();
                self.error = None;
                Outcome::Stay
            }
            LayoutAction::Mirror(on) => {
                let pages = self.pages;
                self.change(|settings| settings.set_mirror(pages, on))
            }
            LayoutAction::Section(section) => {
                if !self.open.remove(&section) {
                    self.open.insert(section);
                }
                Outcome::Stay
            }
            LayoutAction::Text(field, text) => {
                self.texts.insert(field, text);
                Outcome::Stay
            }
            LayoutAction::Submit(field) => self.submit(field),
            LayoutAction::NumberingMode(index) => {
                let mode = match index {
                    1 => LineNumberingMode::AllLines,
                    2 => LineNumberingMode::TextLines,
                    _ => LineNumberingMode::None,
                };
                let numbering = &self.numbering;
                let next = numbering.with(mode, numbering.every(), numbering.reset_each_page());
                self.numbering(next)
            }
            LayoutAction::NumberingEvery(every) => {
                let numbering = &self.numbering;
                let next = numbering.with(numbering.mode(), every, numbering.reset_each_page());
                self.numbering(next)
            }
            LayoutAction::NumberingReset(reset) => {
                let numbering = &self.numbering;
                let next = numbering.with(numbering.mode(), numbering.every(), reset);
                self.numbering(next)
            }
            LayoutAction::NumberingStyle(index) => {
                let Some(style) = numbering_styles(document).into_iter().nth(index) else {
                    return Outcome::Stay;
                };
                let next = LineNumbering {
                    style,
                    ..self.numbering.clone()
                };
                self.numbering(next)
            }
            LayoutAction::Paper(index) => match PAPERS.get(index) {
                Some(paper) => {
                    let code = paper.code;
                    self.change(|settings| settings.set_paper(code))
                }
                None => {
                    let (width, height) = (self.settings.right.width, self.settings.right.height);
                    self.change(|settings| settings.set_custom_size(width, height))
                }
            },
            LayoutAction::Landscape(on) => self.change(|settings| settings.set_landscape(on)),
            LayoutAction::Columns(band, columns) => {
                let pages = self.pages;
                self.change(|settings| match band {
                    None => settings.set_columns(pages, columns),
                    Some(band) => settings.set_band_columns(band, pages, columns),
                })
            }
            LayoutAction::EvenColumns(band) => {
                let pages = self.pages;
                self.change(|settings| match band {
                    None => settings.set_even_columns(pages),
                    Some(band) => settings.set_band_even_columns(band, pages),
                })
            }
            LayoutAction::Balance(band, on) => {
                let pages = self.pages;
                self.change(|settings| match band {
                    None => settings.set_balance(pages, on),
                    Some(band) => settings.set_band_balance(band, pages, on),
                })
            }
            LayoutAction::TabKind(band, index, kind) => self.edit_tabs(band, |stops| {
                if let (Some(stop), Some((kind, _))) = (stops.get_mut(index), TAB_KINDS.get(kind)) {
                    stop.kind = *kind;
                }
            }),
            LayoutAction::TabLeader(band, index, leader) => self.edit_tabs(band, |stops| {
                if let (Some(stop), Some((leader, _))) = (stops.get_mut(index), LEADERS.get(leader))
                {
                    stop.leader = *leader;
                }
            }),
            LayoutAction::AddTab(band) => self.edit_tabs(band, |stops| {
                let position = stops.last().map_or(720, |stop| stop.position + 720);
                stops.push(TabStop::new(amipro_document::TabKind::Left, position));
            }),
            LayoutAction::RemoveTab(band, index) => self.edit_tabs(band, |stops| {
                if index < stops.len() {
                    stops.remove(index);
                }
            }),
            LayoutAction::LineSide(side, on) => {
                let pages = self.pages;
                let current = self
                    .settings
                    .page(pages)
                    .border
                    .map_or(Sides::NONE, |border| border.sides);
                // All clears the four sides; a side clears All (`1250:0000`).
                let sides = match (side, on) {
                    (LineSide::All, true) => Sides::ALL,
                    (LineSide::All, false) => Sides::NONE,
                    (side, on) => {
                        let mut sides = if current == Sides::ALL && on {
                            Sides::NONE
                        } else {
                            current
                        };
                        match side {
                            LineSide::Left => sides.left = on,
                            LineSide::Right => sides.right = on,
                            LineSide::Top => sides.top = on,
                            LineSide::Bottom => sides.bottom = on,
                            LineSide::All => {}
                        }
                        sides
                    }
                };
                self.change(|settings| settings.set_page_line_sides(pages, sides))
            }
            LayoutAction::LineStyle(index) => {
                let pages = self.pages;
                self.change(|settings| settings.set_page_line_style(pages, index as u16 + 1))
            }
            LayoutAction::LinePosition(index) => {
                let pages = self.pages;
                let position = POSITIONS
                    .get(index)
                    .map_or(LinePosition::Inside, |(p, _)| *p);
                self.change(|settings| settings.set_page_line_position(pages, position))
            }
            LayoutAction::ColumnLine(on) => {
                let pages = self.pages;
                self.change(|settings| settings.set_column_line(pages, on))
            }
            LayoutAction::ColumnLineStyle(index) => {
                let pages = self.pages;
                self.change(|settings| settings.set_column_line_style(pages, index as u16 + 1))
            }
            LayoutAction::LineColour(colour) => {
                let pages = self.pages;
                self.change(|settings| settings.set_line_colour(pages, colour))
            }
            LayoutAction::FromSecondPage(band, on) => self.change(|settings| {
                settings.set_from_second_page(band, on);
                Ok(())
            }),
            LayoutAction::InsertChoice(choice) => {
                self.insert_choice = choice;
                Outcome::Stay
            }
            LayoutAction::Insert => match self.insert_choice.checked_sub(1) {
                None => Outcome::InsertCopy(self.layout),
                Some(index) => match document.layouts().get(index) {
                    Some(layout) => Outcome::InsertExisting(layout.id),
                    None => Outcome::InsertCopy(self.layout),
                },
            },
            LayoutAction::Remove => {
                if self.layout == document.document_layout().id {
                    self.error = Some("The document's own page layout stays.".into());
                    Outcome::Stay
                } else {
                    Outcome::Remove(self.layout)
                }
            }
            LayoutAction::Back => Outcome::Back,
            LayoutAction::Close => Outcome::Close,
        }
    }

    /// Applies a typed value, or refuses it in place.
    fn submit(&mut self, field: Field) -> Outcome {
        let Some(text) = self.texts.get(&field).cloned() else {
            return Outcome::Stay;
        };
        if field == Field::LineInterval {
            // Typing a count chooses Every N lines (dialog 1130); 0 and
            // above 99 are refused (VM LN18).
            let Some(interval) = text
                .trim()
                .parse::<u16>()
                .ok()
                .filter(|count| LineNumbering::INTERVAL_RANGE.contains(count))
            else {
                self.error = Some(format!(
                    "“{}” is not a whole number from 1 to 99.",
                    text.trim()
                ));
                return Outcome::Stay;
            };
            self.texts.remove(&field);
            let numbering = &self.numbering;
            let next = LineNumbering {
                interval,
                ..numbering.with(
                    numbering.mode(),
                    LineNumberingEvery::Lines,
                    numbering.reset_each_page(),
                )
            };
            return self.numbering(next);
        }
        if text == self.text_from_settings(field) {
            self.texts.remove(&field);
            return Outcome::Stay;
        }
        let Some(value) = parse_length(&text) else {
            self.error = Some(format!(
                "“{}” is not a length from 0 to 22 inches.",
                text.trim()
            ));
            return Outcome::Stay;
        };
        let pages = self.pages;
        let outcome = self.change(|settings| apply_length(settings, pages, field, value));
        if !matches!(outcome, Outcome::Stay) || self.error.is_none() {
            self.texts.remove(&field);
        }
        outcome
    }

    /// A change to the page's or a band's tab list.
    fn edit_tabs(&mut self, band: Option<Band>, edit: impl FnOnce(&mut Vec<TabStop>)) -> Outcome {
        let Some(layout) = self.tab_box(band) else {
            return Outcome::Stay;
        };
        let mut stops = layout.tab_stops().to_vec();
        edit(&mut stops);
        let pages = self.pages;
        self.texts
            .retain(|field, _| !matches!(field, Field::Tab(of, _) if *of == band));
        self.change(|settings| match band {
            None => settings.set_tabs(pages, stops),
            Some(band) => settings.set_band_tabs(band, pages, stops),
        })
    }

    fn text_from_settings(&self, field: Field) -> String {
        let mut shown = self.clone();
        shown.texts.clear();
        shown.text(field)
    }
}

fn margin(margins: &amipro_document::Margins, side: MarginSide) -> Twips {
    match side {
        MarginSide::Left => margins.left,
        MarginSide::Right => margins.right,
        MarginSide::Top => margins.top,
        MarginSide::Bottom => margins.bottom,
    }
}

/// A typed length's change through Ami Pro's rules.
fn apply_length(
    settings: &mut LayoutSettings,
    pages: Pages,
    field: Field,
    value: Twips,
) -> Result<(), Refusal> {
    match field {
        Field::Width => settings.set_custom_size(value, settings.page(pages).height),
        Field::Height => settings.set_custom_size(settings.page(pages).width, value),
        Field::Margin(side) => settings.set_margin(pages, side, value),
        Field::Gutter => settings.set_gutter(pages, value),
        Field::BandMargin(band, side) => settings.set_band_margin(band, pages, side, value),
        Field::BandGutter(band) => settings.set_band_gutter(band, pages, value),
        Field::Tab(band, index) => {
            let layout = match band {
                None => &settings.page(pages).layout,
                Some(band) => settings
                    .band(band, pages)
                    .ok_or("This layout has no such header or footer")?,
            };
            let mut stops = layout.tab_stops().to_vec();
            let stop = stops.get_mut(index).ok_or("There is no such tab")?;
            stop.position = value;
            match band {
                None => settings.set_tabs(pages, stops),
                Some(band) => settings.set_band_tabs(band, pages, stops),
            }
        }
        // Not a length: `submit` takes it before.
        Field::LineInterval => Ok(()),
    }
}

// ----- View: the rail, about 300 px wide -----

/// The rail's width while Page Layout Properties shows (`UI_SPEC.md` §18).
pub const WIDTH: f32 = 300.0;

/// The styles Line numbering's Based on lists: the document's, by name
/// (the dialog's list, 13 characters at most).
pub fn numbering_styles(document: &Document) -> Vec<String> {
    let mut names: Vec<String> = document
        .styles()
        .values()
        .map(|style| style.name.clone())
        .filter(|name| name.chars().count() <= 13)
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names
}

/// Ami Pro's Page › Line Numbering (dialog 1130) as a section: the list,
/// Every line, other, fifth, or N (the count typed where it shows), Reset
/// each page, and Based on, which only Number all lines uses (the dialog
/// greys it for Number text lines). The document's, for every layout.
fn line_numbering(
    state: &PageLayoutProperties,
    document: &Document,
) -> Vec<Element<'static, Message>> {
    let numbering = &state.numbering;
    let mode = numbering.mode();
    let mut children = vec![
        widget::text::caption("For the whole document").into(),
        widget::dropdown(
            vec![
                "None".to_owned(),
                "Number all lines".to_owned(),
                "Number text lines".to_owned(),
            ],
            Some(match mode {
                LineNumberingMode::None => 0,
                LineNumberingMode::AllLines => 1,
                LineNumberingMode::TextLines => 2,
            }),
            |index| send(LayoutAction::NumberingMode(index)),
        )
        .width(Length::Fill)
        .into(),
    ];
    let every = numbering.every();
    let radio = |label: &'static str, value: LineNumberingEvery| -> Element<'static, Message> {
        widget::radio(label, value, Some(every), |value| {
            send(LayoutAction::NumberingEvery(value))
        })
        .into()
    };
    children.push(radio("Every line", LineNumberingEvery::Line));
    children.push(radio("Every other line", LineNumberingEvery::OtherLine));
    children.push(radio("Every fifth line", LineNumberingEvery::FifthLine));
    children.push(row(vec![
        radio("Every", LineNumberingEvery::Lines),
        input(state, Field::LineInterval, 48.0),
        widget::text::body("lines").into(),
    ]));
    children.push(check(
        numbering.reset_each_page(),
        "Reset each page",
        true,
        LayoutAction::NumberingReset,
    ));
    let styles = numbering_styles(document);
    let based_on: Element<'static, Message> = if mode == LineNumberingMode::TextLines {
        widget::text::body("Body Text (text lines use it)").into()
    } else {
        let selected = styles.iter().position(|name| *name == numbering.style);
        widget::dropdown(styles, selected, |index| {
            send(LayoutAction::NumberingStyle(index))
        })
        .width(Length::Fill)
        .into()
    };
    children.push(row(vec![
        widget::text::body("Based on")
            .width(Length::Fixed(64.0))
            .into(),
        based_on,
    ]));
    children
}

fn send(action: LayoutAction) -> Message {
    Message::PageLayoutProperties(action)
}

fn row(children: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    widget::row::with_children(children)
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

fn input(state: &PageLayoutProperties, field: Field, width: f32) -> Element<'static, Message> {
    widget::text_input("", state.text(field))
        .width(Length::Fixed(width))
        .on_input(move |text| send(LayoutAction::Text(field, text)))
        .on_submit(move |_| send(LayoutAction::Submit(field)))
        .on_unfocus(send(LayoutAction::Submit(field)))
        .into()
}

fn check(
    on: bool,
    text: &'static str,
    enabled: bool,
    action: impl Fn(bool) -> LayoutAction + 'static,
) -> Element<'static, Message> {
    let check = widget::checkbox(on).label(text);
    if enabled {
        check.on_toggle(move |on| send(action(on))).into()
    } else {
        check.into()
    }
}

fn caption(text: &'static str) -> Element<'static, Message> {
    widget::text::caption(text).into()
}

/// A labelled length box: "Left [1.00] in."
fn length_row(
    state: &PageLayoutProperties,
    text: &'static str,
    field: Field,
) -> Element<'static, Message> {
    row(vec![
        widget::text::body(text).width(Length::Fixed(88.0)).into(),
        input(state, field, 72.0),
        caption("in."),
    ])
}

fn dropdown<T: AsRef<str> + Send + Sync + Clone + 'static>(
    labels: Vec<T>,
    selected: Option<usize>,
    on_select: impl Fn(usize) -> LayoutAction + Send + Sync + 'static,
) -> widget::Dropdown<'static, T, Message, Message> {
    widget::dropdown(labels, selected, move |index| send(on_select(index)))
}

/// Page: Ami Pro's paper sizes or a typed one, and the orientation.
fn page(state: &PageLayoutProperties) -> Vec<Element<'static, Message>> {
    let settings = &state.settings;
    let mut sizes: Vec<&'static str> = PAPERS.iter().map(|paper| paper.name).collect();
    sizes.push("Custom");
    let selected = PAPERS
        .iter()
        .position(|paper| paper.code == settings.paper && settings.paper != CUSTOM_PAPER)
        .unwrap_or(PAPERS.len());
    let landscape = settings.landscape;
    let orientation = |text: &'static str, value: bool| -> Element<'static, Message> {
        widget::radio(text, value, Some(landscape), |value| {
            send(LayoutAction::Landscape(value))
        })
        .into()
    };
    vec![
        row(vec![
            widget::text::body("Size").width(Length::Fixed(88.0)).into(),
            dropdown(sizes, Some(selected), LayoutAction::Paper)
                .width(Length::Fixed(120.0))
                .into(),
        ]),
        length_row(state, "Width", Field::Width),
        length_row(state, "Height", Field::Height),
        row(vec![
            orientation("Portrait", false),
            orientation("Landscape", true),
        ]),
    ]
}

/// The margins, columns, gutter, and balance of the page or of a band.
fn columns_rows(
    state: &PageLayoutProperties,
    band: Option<Band>,
    layout: &amipro_document::LayoutBox,
) -> Vec<Element<'static, Message>> {
    let margin = |side| match band {
        None => Field::Margin(side),
        Some(band) => Field::BandMargin(band, side),
    };
    let gutter = match band {
        None => Field::Gutter,
        Some(band) => Field::BandGutter(band),
    };
    let several = layout.columns > 1;
    let mut children = vec![
        length_row(state, "Left", margin(MarginSide::Left)),
        length_row(state, "Right", margin(MarginSide::Right)),
        length_row(state, "Top", margin(MarginSide::Top)),
        length_row(state, "Bottom", margin(MarginSide::Bottom)),
        row(vec![
            widget::text::body("Columns")
                .width(Length::Fixed(88.0))
                .into(),
            dropdown(
                (1..=8).map(|count| count.to_string()).collect(),
                Some(usize::from(layout.columns.clamp(1, 8)) - 1),
                move |index| LayoutAction::Columns(band, index as u8 + 1),
            )
            .width(Length::Fixed(72.0))
            .into(),
        ]),
    ];
    if layout.column_edges.is_some() {
        children.push(crate::even_columns_row(send(LayoutAction::EvenColumns(
            band,
        ))));
    }
    if several {
        children.push(length_row(state, "Gutter", gutter));
    }
    children.push(check(
        layout.balance,
        "Column balance",
        several,
        move |on| LayoutAction::Balance(band, on),
    ));
    children
}

/// The page's or a band's tab stops, as Style Properties edits a style's.
fn tab_rows(
    state: &PageLayoutProperties,
    band: Option<Band>,
    layout: &amipro_document::LayoutBox,
) -> Vec<Element<'static, Message>> {
    let stops = layout.tab_stops();
    let mut children = Vec::new();
    for (index, stop) in stops.iter().enumerate() {
        children.push(row(vec![
            dropdown(
                TAB_KINDS.iter().map(|(_, name)| *name).collect(),
                TAB_KINDS.iter().position(|(kind, _)| *kind == stop.kind),
                move |kind| LayoutAction::TabKind(band, index, kind),
            )
            .width(Length::Fixed(96.0))
            .into(),
            input(state, Field::Tab(band, index), 60.0),
            caption("in."),
            widget::button::icon(widget::icon::from_name("list-remove-symbolic"))
                .extra_small()
                .name("Remove this tab")
                .tooltip("Remove this tab")
                .on_press(send(LayoutAction::RemoveTab(band, index)))
                .into(),
        ]));
        children.push(
            dropdown(
                LEADERS.iter().map(|(_, name)| *name).collect(),
                LEADERS
                    .iter()
                    .position(|(leader, _)| *leader == stop.leader),
                move |leader| LayoutAction::TabLeader(band, index, leader),
            )
            .width(Length::Fixed(160.0))
            .into(),
        );
    }
    children.push(
        widget::button::standard("Add tab")
            .on_press_maybe(
                (stops.len() < amipro_editing::layout_setup::MAX_TABS)
                    .then_some(send(LayoutAction::AddTab(band))),
            )
            .into(),
    );
    children
}

/// Lines around the page, the line between columns, and their colour.
fn lines<'a>(state: &PageLayoutProperties, colours: Context<'a>) -> Vec<Element<'a, Message>> {
    let page = state.settings.page(state.pages);
    let border = page.border;
    let sides = border.map_or(Sides::NONE, |border| border.sides);
    let all = sides == Sides::ALL;
    let side = |text: &'static str, which: LineSide, on: bool| {
        check(on, text, true, move |on| LayoutAction::LineSide(which, on))
    };
    let column_line = page.layout.line_between;
    let colour = border
        .map(|border| border.colour)
        .or(column_line.map(|line| line.colour));
    let mut children = vec![
        caption("Around page"),
        row(vec![
            side("All", LineSide::All, all),
            side("Left", LineSide::Left, sides.left && !all),
            side("Right", LineSide::Right, sides.right && !all),
        ]),
        row(vec![
            side("Top", LineSide::Top, sides.top && !all),
            side("Bottom", LineSide::Bottom, sides.bottom && !all),
        ]),
    ];
    if let Some(border) = border {
        children.push(
            dropdown(
                RULE_STYLES.to_vec(),
                usize::from(border.style.clamp(1, 12)).checked_sub(1),
                LayoutAction::LineStyle,
            )
            .width(Length::Fill)
            .into(),
        );
        children.push(row(vec![
            widget::text::body("Position")
                .width(Length::Fixed(88.0))
                .into(),
            dropdown(
                POSITIONS.iter().map(|(_, name)| *name).collect(),
                POSITIONS
                    .iter()
                    .position(|(position, _)| *position == border.position),
                LayoutAction::LinePosition,
            )
            .width(Length::Fill)
            .into(),
        ]));
    }
    children.push(check(
        column_line.is_some(),
        "Line between columns",
        true,
        LayoutAction::ColumnLine,
    ));
    if let Some(line) = column_line {
        children.push(
            dropdown(
                RULE_STYLES.to_vec(),
                usize::from(line.style.clamp(1, 12)).checked_sub(1),
                LayoutAction::ColumnLineStyle,
            )
            .width(Length::Fill)
            .into(),
        );
    }
    if let Some(colour) = colour {
        children.push(caption("Colour"));
        children.push(colours.control(Target::PageLines, colour));
    }
    children
}

/// A header's or footer's margins within its band, columns, tabs, and
/// Begin on second page.
fn band(state: &PageLayoutProperties, band: Band) -> Vec<Element<'static, Message>> {
    let Some(layout) = state.settings.band(band, state.pages).cloned() else {
        return vec![caption("This layout has none.")];
    };
    let mut children = vec![caption(match band {
        Band::Header => "Margins within the top margin",
        Band::Footer => "Margins within the bottom margin",
    })];
    children.extend(columns_rows(state, Some(band), &layout));
    children.push(caption("Tabs"));
    children.extend(tab_rows(state, Some(band), &layout));
    let from_second = match band {
        Band::Header => state.settings.header_from_second_page,
        Band::Footer => state.settings.footer_from_second_page,
    };
    children.push(check(
        from_second,
        "Begin on second page",
        true,
        move |on| LayoutAction::FromSecondPage(band, on),
    ));
    children
}

pub fn panel<'a>(
    state: &PageLayoutProperties,
    document: &Document,
    colours: Context<'a>,
) -> Element<'a, Message> {
    let header = widget::row::with_children(vec![
        widget::text::heading("Page Layout Properties").into(),
        widget::space::horizontal().into(),
        widget::button::text("‹ Styles")
            .on_press(send(LayoutAction::Back))
            .into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Page Layout Properties")
            .tooltip(crate::chrome::command_tooltip(
                crate::CommandId::StylesPanel,
            ))
            .on_press(send(LayoutAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center);
    let names: Vec<String> = document
        .layouts()
        .iter()
        .map(|layout| layout.name.clone())
        .collect();
    let mut body: Vec<Element<'a, Message>> = vec![row(vec![
        widget::text::body("Layout")
            .width(Length::Fixed(56.0))
            .into(),
        widget::dropdown(
            names,
            document
                .layouts()
                .iter()
                .position(|layout| layout.id == state.layout),
            |index| send(LayoutAction::Layout(index)),
        )
        .width(Length::Fill)
        .into(),
    ])];
    let pages = state.pages;
    let radio = |label: &'static str, value: Pages| -> Element<'static, Message> {
        widget::radio(label, value, Some(pages), |value| {
            send(LayoutAction::Pages(value))
        })
        .into()
    };
    // Mirrored pages differ, so All is refused with the reason until Mirror
    // is off (Ami Pro greys it, `1248:046c`).
    let mirror = state.settings.mirror;
    body.push(row(vec![
        widget::text::body("Pages")
            .width(Length::Fixed(56.0))
            .into(),
        radio("All", Pages::All),
        radio("Right", Pages::Right),
        radio("Left", Pages::Left),
    ]));
    let mirror_check = widget::checkbox(mirror).label("Mirror");
    body.push(
        widget::container(if pages == Pages::All {
            mirror_check
        } else {
            mirror_check.on_toggle(|on| send(LayoutAction::Mirror(on)))
        })
        .padding([0, 0, 0, 62])
        .into(),
    );
    if let Some(problem) = &state.error {
        body.push(widget::text::caption(problem.clone()).into());
    }
    for (section, title) in Section::ALL {
        let open = state.open.contains(&section);
        body.push(
            widget::button::custom(
                widget::row::with_children(vec![
                    widget::icon::from_name(if open {
                        "pan-down-symbolic"
                    } else {
                        "pan-end-symbolic"
                    })
                    .size(16)
                    .icon()
                    .into(),
                    widget::text::heading(title).into(),
                ])
                .spacing(6)
                .align_y(cosmic::iced::Alignment::Center),
            )
            .class(cosmic::theme::Button::Text)
            .padding([4, 0])
            .width(Length::Fill)
            .on_press(send(LayoutAction::Section(section)))
            .into(),
        );
        if open {
            let page_layout = state.settings.page(state.pages).layout.clone();
            let fields = match section {
                Section::Page => page(state),
                Section::Margins => columns_rows(state, None, &page_layout),
                Section::Tabs => tab_rows(state, None, &page_layout),
                Section::Lines => lines(state, colours),
                Section::Header => band(state, Band::Header),
                Section::Footer => band(state, Band::Footer),
                Section::LineNumbering => line_numbering(state, document),
            };
            body.push(
                widget::container(widget::column::with_children(fields).spacing(6))
                    .padding([0, 0, 8, 22])
                    .into(),
            );
        }
    }
    // Ami Pro's Page › Insert Page Layout: Insert (a copy from the next
    // page), Revert (an earlier layout again), and Remove.
    let mut choices = vec![format!("A copy of {}", state.settings.name)];
    choices.extend(
        document
            .layouts()
            .iter()
            .map(|layout| format!("{} again", layout.name)),
    );
    let own = state.layout == document.document_layout().id;
    body.push(widget::text::caption("From the next page").into());
    body.push(
        widget::dropdown(choices, Some(state.insert_choice), |choice| {
            send(LayoutAction::InsertChoice(choice))
        })
        .width(Length::Fill)
        .into(),
    );
    body.push(row(vec![
        widget::button::standard("Insert Page Layout")
            .on_press(send(LayoutAction::Insert))
            .into(),
        widget::button::standard("Remove")
            .on_press_maybe((!own).then_some(send(LayoutAction::Remove)))
            .into(),
    ]));
    widget::column::with_children(vec![
        header.into(),
        crate::rail_scroller(widget::column::with_children(body).spacing(6).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_show_as_ami_pros_boxes_show_them() {
        assert_eq!(inches(11_905), "8.27");
        assert_eq!(parse_length("8.27"), Some(11_908));
    }

    #[test]
    fn uneven_columns_show_their_gap_while_alike_and_even_out_on_request() {
        let mut document = Document::default();
        let standard = document.document_layout().id;
        // RU2's two columns: one gap of 240, widths 5610 and 3510.
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
        let mut state = PageLayoutProperties::new(&document, standard, Some(standard)).unwrap();
        assert_eq!(state.text(Field::Gutter), "0.17");
        let Outcome::Apply(_, settings) = state.update(LayoutAction::EvenColumns(None), &document)
        else {
            panic!("applies");
        };
        assert_eq!(settings.right.layout.column_edges, None);
        assert_eq!(settings.right.layout.gutter, 240);
        // Gaps that differ show no gutter, as Ami Pro's dialog.
        document.document_layout_mut().right.layout.set_edges(
            vec![
                amipro_document::ColumnEdges {
                    left: 0,
                    right: 2_000,
                },
                amipro_document::ColumnEdges {
                    left: 2_240,
                    right: 5_000,
                },
                amipro_document::ColumnEdges {
                    left: 5_750,
                    right: 9_360,
                },
            ],
            9_360,
        );
        let state = PageLayoutProperties::new(&document, standard, Some(standard)).unwrap();
        assert_eq!(state.text(Field::Gutter), "");
    }

    #[test]
    fn the_panel_follows_the_caret_page_and_the_documents_settings() {
        let mut document = Document::default();
        let standard = document.document_layout().id;
        let mut state = PageLayoutProperties::new(&document, standard, Some(standard)).unwrap();
        assert_eq!(state.pages, Pages::All);
        assert_eq!(state.text(Field::Margin(MarginSide::Left)), "1.00");
        // A typed margin applies through Ami Pro's rules...
        state.update(
            LayoutAction::Text(Field::Margin(MarginSide::Left), "1.5".into()),
            &document,
        );
        let Outcome::Apply(layout, settings) = state.update(
            LayoutAction::Submit(Field::Margin(MarginSide::Left)),
            &document,
        ) else {
            panic!("applies");
        };
        assert_eq!(
            (layout, settings.right.layout.margins.left),
            (standard, 2_160)
        );
        // ... and one that does not fit is refused in place with the reason.
        state.update(
            LayoutAction::Text(Field::Margin(MarginSide::Left), "12".into()),
            &document,
        );
        assert_eq!(
            state.update(
                LayoutAction::Submit(Field::Margin(MarginSide::Left)),
                &document
            ),
            Outcome::Stay
        );
        assert!(state.error.as_deref().unwrap().contains("no room"));
        assert_eq!(state.text(Field::Margin(MarginSide::Left)), "12");
        // The document's settings show after every change (an undo, say).
        document.document_layout_mut().right.layout.margins.left = 1_800;
        assert!(state.sync(&document, Some(standard), None));
        assert!(state.sync(&document, Some(standard), Some(Band::Footer)));
        assert!(
            state.open.contains(&Section::Footer),
            "the caret's band opens"
        );
        assert_eq!(state.text(Field::Margin(MarginSide::Left)), "1.25");
        // Right pages: nothing changes until a setting does.
        assert_eq!(
            state.update(LayoutAction::Pages(Pages::Right), &document),
            Outcome::Stay
        );
        assert_eq!(state.pages, Pages::Right);
        assert!(matches!(
            state.update(LayoutAction::Mirror(true), &document),
            Outcome::Apply(..)
        ));
    }
}

/// Line Numbering against Ami Pro 3.1's prints of VM fixtures LN2–LN20
/// (`docs/research/COMMANDS_NOTES.md` §8.5): each page's first and last
/// number and how many, laid out with COSMIC Text (`AMIPRO_VM_LAB`).
#[cfg(test)]
mod line_numbering_twins {
    use amipro_layout::{ShapeCache, layout_document};

    /// A page's first and last number and how many it shows.
    type Page = (u32, u32, usize);

    /// (fixture, its pages) as Ami Pro printed them.
    const PRINTED: &[(&str, &[Page])] = &[
        ("LN2", &[(1, 45, 45), (46, 90, 45), (91, 135, 45)]),
        ("LN3", &[(1, 45, 45), (46, 90, 45), (91, 132, 42)]),
        ("LN4", &[(1, 45, 23), (47, 89, 22), (91, 131, 21)]),
        ("LN5", &[(5, 45, 9), (50, 90, 9), (95, 130, 8)]),
        ("LN6", &[(7, 42, 6), (49, 84, 6), (91, 126, 6)]),
        ("LN7", &[(1, 45, 45), (1, 45, 45), (1, 42, 42)]),
        ("LN8", &[(1, 45, 45), (1, 45, 45), (1, 45, 45)]),
        ("LN9", &[(1, 27, 27)]),
        ("LN10", &[(1, 45, 45)]),
        ("LN13", &[(1, 30, 30)]),
        ("LN14", &[(1, 27, 27)]),
        ("LN15", &[(1, 45, 45), (46, 90, 45), (91, 132, 42)]),
        ("LN20", &[(1, 44, 44), (45, 89, 45)]),
    ];

    /// Differences that are not the numbering's: LN9 was saved for the XPS
    /// printer and printed on PS File, whose line heights put one line
    /// less beside the frame (LN14, the same page saved for PS File,
    /// matches); LN13's 18 pt Arial line is 21.35 pt on Ami Pro's printer
    /// and about 20.7 pt in the font's metrics, so 31 slots fit, not 30.
    const KNOWN: &[&str] = &["LN9", "LN13"];

    #[test]
    #[ignore = "reads Ami Pro's fixtures from the VM lab (AMIPRO_VM_LAB)"]
    fn line_numbers_match_ami_pros_prints() {
        let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
            eprintln!("AMIPRO_VM_LAB is not set; skipping");
            return;
        };
        let out = std::path::PathBuf::from(lab).join("OUT");
        let mut differing = Vec::new();
        for (name, printed) in PRINTED {
            let document = amipro_formats::import_sam_path(out.join(format!("{name}.SAM")))
                .expect("Ami Pro's fixture")
                .document;
            let layout = crate::page_layout::with_shaper(|shaper| {
                layout_document(&document, shaper, &mut ShapeCache::default())
            });
            let ours: Vec<Page> = layout
                .line_numbers
                .iter()
                .map(|page| {
                    let number = |index: Option<&amipro_layout::LineNumberBox>| {
                        index.map_or(0, |number| number.text.parse().unwrap())
                    };
                    (number(page.first()), number(page.last()), page.len())
                })
                .collect();
            eprintln!("{name}: ours {ours:?}, Ami Pro {printed:?}");
            if std::env::var_os("AMIPRO_TWIN_LINES").is_some() {
                for (page, numbers) in layout.pages.iter().zip(&layout.line_numbers) {
                    for number in numbers {
                        let line = page
                            .lines
                            .iter()
                            .find(|line| (line.baseline - number.baseline).abs() < 0.5)
                            .map(|line| {
                                let text = &document.flow(line.flow).unwrap().paragraphs
                                    [line.paragraph_index]
                                    .text;
                                format!("{:?} {}", line.container, &text[line.bytes.clone()])
                            });
                        eprintln!("  {} @{} {line:?}", number.text, number.baseline);
                    }
                }
            }
            if ours != *printed && !KNOWN.contains(name) {
                differing.push(*name);
            }
            // Each number and its space end at its page's left margin.
            for (page, numbers) in layout.pages.iter().zip(&layout.line_numbers) {
                let margin = page.columns[0].x;
                assert!(
                    numbers.iter().all(|number| number.x < margin - 60.0),
                    "{name} page {}: in the margin",
                    page.number
                );
            }
        }
        assert!(differing.is_empty(), "differ from Ami Pro: {differing:?}");
    }
}
