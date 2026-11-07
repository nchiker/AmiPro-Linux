//! Editing on the page surface: a selection in any flow (main text, a text
//! frame, a header or footer, a table cell) and the actions the surface
//! publishes. Every change is an `amipro_editing::Command` that the session's
//! history executes; this module holds only what does not need geometry.

use amipro_document::{CharacterFormat, Colour, Document, FlowId, FrameId, InlineObject};
use amipro_editing::{CharacterAttribute, Command, TextPosition};

use cosmic::iced::Size;

/// A selection within one flow. `page` is the page instance the caret is on:
/// it matters for flows shown on several pages (headers, footers, repeating
/// frames) and keeps the caret on the page where the user clicked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageSelection {
    pub anchor: TextPosition,
    pub focus: TextPosition,
    /// 1-based page number.
    pub page: u32,
}

impl PageSelection {
    pub fn caret(position: TextPosition, page: u32) -> Self {
        Self {
            anchor: position,
            focus: position,
            page,
        }
    }

    /// The start of the main text.
    pub fn start(document: &Document) -> Self {
        Self::caret(
            TextPosition {
                flow: document.main_flow_id(),
                paragraph: 0,
                byte: 0,
            },
            1,
        )
    }

    pub fn flow(self) -> FlowId {
        self.focus.flow
    }

    pub fn is_caret(self) -> bool {
        self.anchor == self.focus
    }

    /// Start and end in document order.
    pub fn ordered(self) -> (TextPosition, TextPosition) {
        let key = |position: TextPosition| (position.paragraph, position.byte);
        if key(self.anchor) <= key(self.focus) {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    /// Moves the focus; with `extend` the anchor stays.
    pub fn moved(self, focus: TextPosition, page: u32, extend: bool) -> Self {
        if extend && focus.flow == self.anchor.flow {
            Self {
                anchor: self.anchor,
                focus,
                page,
            }
        } else {
            Self::caret(focus, page)
        }
    }
}

/// What the page surface asks the application to do.
#[derive(Clone, Debug, PartialEq)]
pub enum PageAction {
    /// The left button went up after a press in text while Fast Format is
    /// on: the selection made with the mouse takes its format (CM4).
    PointerReleased,
    /// Put the caret (or extend the selection) at a hit position.
    MoveTo {
        position: TextPosition,
        page: u32,
        extend: bool,
    },
    Insert(String),
    Backspace,
    Delete,
    ParagraphBreak,
    MoveLeft {
        extend: bool,
    },
    MoveRight {
        extend: bool,
    },
    SelectAll,
    /// Leave frame, header, footer, or cell text for the main text.
    Escape,
    /// Tab and Shift+Tab in a table: the next or previous cell; past the
    /// last one a new row (TD5).
    CellStep {
        forward: bool,
    },
    /// Ctrl+arrows in a table: a cell up, down, left, or right; with Shift
    /// a cell selection (`1408:085a`).
    CellMove {
        down: isize,
        right: isize,
        extend: bool,
    },
    /// Alt+Up and Alt+Down in a table: the caret's (or the selected) rows
    /// move up or down.
    MoveRows {
        down: bool,
    },
    /// A table's grid line dragged and released (`1498:2d2a`, TD6): the
    /// column (or row) before it takes `size`, as Column/Row Size does.
    ResizeTrack {
        frame: amipro_document::FrameId,
        column: bool,
        index: usize,
        size: amipro_document::Twips,
    },
    /// Selected cells dropped on the cell `to` (`14e0:0000`, TD9): whole
    /// rows or columns move there, a block of cells is moved over the cells
    /// there; `copy` (Ctrl) copies instead.
    DropCells {
        frame: amipro_document::FrameId,
        to: (usize, usize),
        copy: bool,
    },
    /// A press on the strip above a column or left of a row (`1498:151b`),
    /// or Table › Select ▸: the cells from `anchor` to `focus`.
    SelectCells {
        frame: amipro_document::FrameId,
        anchor: (usize, usize),
        focus: (usize, usize),
    },
    /// The visible part of the workspace changed size.
    Viewport(Size),
    /// A right press, before its menu opens: whether it was in a page's
    /// left or right margin, where the menu offers Page Layout… first
    /// (`UI_SPEC.md` §16, §18).
    ContextAt {
        margin: bool,
    },
    /// A double-click on a note's mark: the Notes panel at that note (Ami
    /// Pro's note window; `UI_SPEC.md` §34, NR1).
    OpenNote(TextPosition),
    /// A double-click in a page's left or right margin: Page Layout
    /// Properties (`UI_SPEC.md` §18).
    OpenPageLayout,
    /// A double-click on a formula cell: Table › Edit Formula… (`UI_SPEC.md`
    /// §41).
    EditFormula,
    /// The end of drawing a frame (Insert › Draw Frame, `UI_SPEC.md` §20):
    /// the rectangle a drag drew on a page (its number and page twips), or
    /// `None` when drawing ended without a frame (a click, Esc, or a press
    /// outside every page).
    FrameDrawn(Option<(u32, amipro_document::Rect)>),
}

fn paragraph_text(document: &Document, flow: FlowId, paragraph: usize) -> Option<&str> {
    document
        .flow(flow)?
        .paragraphs
        .get(paragraph)
        .map(|paragraph| paragraph.text.as_str())
}

/// The end of a flow.
pub fn flow_end(document: &Document, flow: FlowId) -> TextPosition {
    let paragraphs = document.flow(flow).map_or(0, |flow| flow.paragraphs.len());
    let paragraph = paragraphs.saturating_sub(1);
    TextPosition {
        flow,
        paragraph,
        byte: paragraph_text(document, flow, paragraph).map_or(0, str::len),
    }
}

/// The position one character before `position`, crossing to the end of
/// the previous paragraph.
pub fn previous_position(document: &Document, position: TextPosition) -> Option<TextPosition> {
    let text = paragraph_text(document, position.flow, position.paragraph)?;
    if position.byte > 0 {
        let byte = text[..position.byte].char_indices().next_back()?.0;
        return Some(TextPosition { byte, ..position });
    }
    let paragraph = position.paragraph.checked_sub(1)?;
    Some(TextPosition {
        paragraph,
        byte: paragraph_text(document, position.flow, paragraph)?.len(),
        ..position
    })
}

/// The position one character after `position`, crossing to the start of
/// the next paragraph.
pub fn next_position(document: &Document, position: TextPosition) -> Option<TextPosition> {
    let text = paragraph_text(document, position.flow, position.paragraph)?;
    if let Some(character) = text[position.byte..].chars().next() {
        return Some(TextPosition {
            byte: position.byte + character.len_utf8(),
            ..position
        });
    }
    let paragraph = position.paragraph + 1;
    paragraph_text(document, position.flow, paragraph)?;
    Some(TextPosition {
        paragraph,
        byte: 0,
        ..position
    })
}

/// The protected text of a paragraph: byte ranges whose format over the
/// paragraph's style is protected, neighbouring runs joined.
pub fn protected_stretches(
    document: &Document,
    flow: FlowId,
    paragraph: usize,
) -> Vec<std::ops::Range<usize>> {
    let Some(paragraph) = document
        .flow(flow)
        .and_then(|flow| flow.paragraph(paragraph))
    else {
        return Vec::new();
    };
    let Some(style) = document.styles().get(&paragraph.style) else {
        return Vec::new();
    };
    let mut bounds: Vec<usize> = paragraph
        .runs
        .iter()
        .flat_map(|run| [run.start, run.end])
        .chain([0, paragraph.text.len()])
        .filter(|byte| *byte <= paragraph.text.len())
        .collect();
    bounds.sort_unstable();
    bounds.dedup();
    let mut stretches: Vec<std::ops::Range<usize>> = Vec::new();
    for pair in bounds.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        if !paragraph.format_at(start).attributes(style).protected {
            continue;
        }
        match stretches.last_mut() {
            Some(last) if last.end == start => last.end = end,
            _ => stretches.push(start..end),
        }
    }
    stretches
}

/// `position`, or the end of the protected text it would fall inside: in
/// Ami Pro a click anywhere in protected text puts the caret after it (VM
/// fixture PT1).
pub fn outside_protected(document: &Document, position: TextPosition) -> TextPosition {
    protected_stretches(document, position.flow, position.paragraph)
        .into_iter()
        .find(|stretch| stretch.start < position.byte && position.byte < stretch.end)
        .map_or(position, |stretch| TextPosition {
            byte: stretch.end,
            ..position
        })
}

/// A position that exists in `document`: the flow, paragraph, and byte are
/// clamped (to the main text when the flow is gone), and the byte moves back
/// to a character boundary.
pub fn valid_position(document: &Document, position: TextPosition) -> TextPosition {
    let Some(flow) = document.flow(position.flow) else {
        return PageSelection::start(document).focus;
    };
    let paragraph = position
        .paragraph
        .min(flow.paragraphs.len().saturating_sub(1));
    let text = flow
        .paragraphs
        .get(paragraph)
        .map_or("", |paragraph| paragraph.text.as_str());
    let mut byte = position.byte.min(text.len());
    while !text.is_char_boundary(byte) {
        byte -= 1;
    }
    TextPosition {
        flow: position.flow,
        paragraph,
        byte,
    }
}

/// The selection made valid for `document` (after undo, redo, or an edit
/// elsewhere). Anchor and focus stay in one flow.
pub fn valid_selection(document: &Document, selection: PageSelection) -> PageSelection {
    let focus = valid_position(document, selection.focus);
    let anchor = if selection.anchor.flow == focus.flow {
        valid_position(document, selection.anchor)
    } else {
        focus
    };
    PageSelection {
        anchor,
        focus,
        page: selection.page.max(1),
    }
}

/// Ami Pro's bullets in its list order (`amipro_document::BULLET_CODES`),
/// with the names the app shows for them (VM fixture BL1's list).
pub const BULLET_NAMES: [(char, &str); 17] = [
    ('\u{10}', "Small dot"),
    ('\u{11}', "Dot"),
    ('\u{12}', "Small square"),
    ('\u{13}', "Square"),
    ('\u{14}', "Open square"),
    ('\u{15}', "Small diamond"),
    ('\u{16}', "Diamond"),
    ('\u{17}', "Small ring"),
    ('\u{18}', "Ring"),
    ('\u{19}', "Check mark"),
    ('\u{8}', "Pointing hand"),
    ('\u{A}', "Box with a shadow"),
    ('\u{B}', "Box with a thick top"),
    ('\u{C}', "Checked box"),
    ('\u{D}', "Crossed box"),
    ('\u{E}', "Arrowhead"),
    ('\u{F}', "Arrowhead, lit from below"),
];

/// The character a bullet put into the text (`InlineObject::Bullet`) copies
/// as in plain text: the nearest Unicode character to what is drawn.
pub fn bullet_character(code: char) -> char {
    match code {
        '\u{10}' => '\u{2219}',
        '\u{11}' => '\u{2022}',
        '\u{12}' => '\u{25AA}',
        '\u{13}' => '\u{25A0}',
        '\u{14}' | '\u{B}' => '\u{25A1}',
        '\u{15}' => '\u{2B29}',
        '\u{16}' => '\u{25C6}',
        '\u{17}' => '\u{25E6}',
        '\u{18}' => '\u{25CB}',
        '\u{19}' => '\u{2713}',
        '\u{8}' => '\u{261B}',
        '\u{A}' => '\u{274F}',
        '\u{C}' => '\u{2611}',
        '\u{D}' => '\u{2612}',
        '\u{E}' => '\u{27A2}',
        '\u{F}' => '\u{27A3}',
        other => other,
    }
}

/// The selected text, paragraphs separated by `\n`; a bullet put into the
/// text copies as its nearest Unicode character.
pub fn selected_text(document: &Document, selection: PageSelection) -> Option<String> {
    if selection.is_caret() {
        return None;
    }
    let (start, end) = selection.ordered();
    let flow = document.flow(start.flow)?;
    let mut text = String::new();
    for paragraph in start.paragraph..=end.paragraph {
        let source = flow.paragraphs.get(paragraph)?;
        let from = if paragraph == start.paragraph {
            start.byte
        } else {
            0
        };
        let to = if paragraph == end.paragraph {
            end.byte
        } else {
            source.text.len()
        };
        if paragraph > start.paragraph {
            text.push('\n');
        }
        let mut objects = source.object_offsets().zip(&source.objects);
        for (offset, character) in source.text.get(from..to)?.char_indices() {
            let object = (character == amipro_document::OBJECT_REPLACEMENT_CHARACTER)
                .then(|| objects.find(|(at, _)| *at == from + offset))
                .flatten();
            match object {
                Some((_, InlineObject::Bullet(code))) => text.push(bullet_character(*code)),
                _ => text.push(character),
            }
        }
    }
    Some(text)
}

/// The frame (a picture) the table cell whose text is `flow` holds, if any
/// (`UI_SPEC.md` §24).
pub fn cell_frame(document: &Document, flow: FlowId) -> Option<FrameId> {
    document.frames().find_map(|frame| match &frame.content {
        amipro_document::FrameContent::Table(table) => table
            .cells
            .iter()
            .find(|cell| matches!(cell.content, amipro_document::CellContent::Text(text) if text == flow))
            .and_then(|cell| cell.frame),
        _ => None,
    })
}

/// The anchored table and row whose cell holds `flow`.
pub fn table_row_of(document: &Document, flow: FlowId) -> Option<(FrameId, usize)> {
    document.frames().find_map(|frame| match &frame.content {
        amipro_document::FrameContent::Table(table) if frame.placement.is_anchored() => table
            .cells
            .iter()
            .position(|cell| matches!(cell.content, amipro_document::CellContent::Text(text) if text == flow))
            .map(|index| (frame.id, index / table.columns.len().max(1))),
        _ => None,
    })
}

/// Insert › Page Break with the caret in a table cell (§26, TP1): the
/// command that sets the row's page break, or takes it away when the row
/// has one.
pub fn row_break_command(document: &Document, flow: FlowId) -> Option<Command> {
    let (frame, row) = table_row_of(document, flow)?;
    let amipro_document::FrameContent::Table(table) = &document.frame(frame)?.content else {
        return None;
    };
    let mut rows = table.row_options.clone();
    if !rows.page_breaks.remove(&row) {
        rows.page_breaks.insert(row);
    }
    Some(Command::SetTableRows { frame, rows })
}

/// Whether the caret's row starts a page with a break (Insert › Page
/// Break's check mark in a table cell).
pub fn row_has_break(document: &Document, flow: FlowId) -> bool {
    table_row_of(document, flow).is_some_and(|(frame, row)| {
        matches!(
            document.frame(frame).map(|frame| &frame.content),
            Some(amipro_document::FrameContent::Table(table))
                if table.row_options.page_breaks.contains(&row)
        )
    })
}

/// Whether `action` would change the text of a table cell that holds a
/// formula (TD.7, TNK-TD7a): printable typing, Enter, Backspace and Delete
/// there do nothing, and the status line says how to change the formula.
/// Delete on selected cells empties them (the formula goes, as in Ami Pro).
pub fn formula_cell_refuses(
    document: &Document,
    selection: PageSelection,
    action: &PageAction,
) -> bool {
    let edits = match action {
        PageAction::Insert(text) => !text.is_empty(),
        PageAction::ParagraphBreak | PageAction::Backspace | PageAction::Delete => true,
        _ => false,
    };
    if !edits || crate::table_editing::cell_selection(document, selection).is_some() {
        return false;
    }
    // A selection is the cell's own text (the caret's cell is the focus's).
    let Some((frame, row, column)) = crate::table_editing::caret_cell(document, selection) else {
        return false;
    };
    crate::table_editing::table_of(document, frame).is_some_and(|table| {
        table
            .cells
            .get(row * table.columns.len().max(1) + column)
            .is_some_and(|cell| cell.formula.is_some())
    })
}

/// Whether the selection covers the whole text of a formula cell (Delete or
/// Backspace then takes the formula out, as Ami Pro's Del does).
pub fn selection_covers_formula_cell(document: &Document, selection: PageSelection) -> bool {
    if selection.is_caret() || selection.anchor.flow != selection.focus.flow {
        return false;
    }
    let Some((frame, row, column)) = crate::table_editing::caret_cell(document, selection) else {
        return false;
    };
    let (start, end) = selection.ordered();
    let whole = document.flow(start.flow).is_some_and(|flow| {
        flow.paragraphs.len() == 1
            && start.paragraph == 0
            && end.paragraph == 0
            && start.byte == 0
            && end.byte == flow.paragraphs[0].text.len()
    });
    whole
        && crate::table_editing::table_of(document, frame).is_some_and(|table| {
            table
                .cells
                .get(row * table.columns.len().max(1) + column)
                .is_some_and(|cell| cell.formula.is_some())
        })
}

/// The command for a text action at `selection`, if it changes text.
pub fn edit_command(
    document: &Document,
    selection: PageSelection,
    action: &PageAction,
) -> Option<Command> {
    // In a picture's cell (T1, VM TF7, TF12): Delete removes the picture as
    // one undo step; typing, Enter, and Backspace do nothing, as in Ami Pro.
    if let Some(frame) = cell_frame(document, selection.focus.flow) {
        return matches!(action, PageAction::Delete).then_some(Command::DeleteFrame { frame });
    }
    // Selected cells (`UI_SPEC.md` §32): Delete and Backspace empty them,
    // rows and columns kept (`1480:01b6`); typing drops the selection and
    // types in the focus's cell, as Ami Pro does.
    if let Some(cells) = crate::table_editing::cell_selection(document, selection) {
        return match action {
            PageAction::Delete | PageAction::Backspace => Some(Command::EditTable {
                frame: cells.frame,
                edit: amipro_editing::tables::TableEdit::ClearCells(cells.range()),
            }),
            PageAction::Insert(_) | PageAction::ParagraphBreak => edit_command(
                document,
                PageSelection::caret(selection.focus, selection.page),
                action,
            ),
            _ => None,
        };
    }
    let (start, end) = selection.ordered();
    let replace = |text: &str| Command::ReplaceText {
        start,
        end,
        text: text.to_owned(),
    };
    // A selection holding whole power fields takes them, marks and all, so
    // none is left behind showing nothing (the user's second use of merge).
    let ranged =
        !selection.is_caret() && amipro_editing::fields::holds_whole_field(document, start, end);
    if ranged {
        let delete = Command::DeleteFields { start, end };
        let then = |text: &str| Command::ReplaceText {
            start,
            end: start,
            text: text.to_owned(),
        };
        return match action {
            PageAction::Insert(text) if !text.is_empty() => {
                Some(Command::Several(vec![delete, then(text)]))
            }
            PageAction::ParagraphBreak => Some(Command::Several(vec![delete, then("\n")])),
            PageAction::Backspace | PageAction::Delete => Some(delete),
            _ => None,
        };
    }
    // Backspace after a power field, or Delete before one, takes the whole
    // field (an empty one, which shows nothing, first).
    if selection.is_caret()
        && let PageAction::Backspace | PageAction::Delete = action
        && let Some((start, end)) = amipro_editing::fields::field_beside(
            document,
            selection.focus,
            matches!(action, PageAction::Delete),
        )
    {
        return Some(Command::DeleteFields { start, end });
    }
    match action {
        PageAction::Insert(text) if !text.is_empty() => Some(replace(text)),
        // Enter in an empty list paragraph ends the list (decision B3).
        PageAction::ParagraphBreak if selection.is_caret() => Some(
            amipro_editing::lists::ends_list(document, selection.focus)
                .unwrap_or_else(|| replace("\n")),
        ),
        PageAction::ParagraphBreak => Some(replace("\n")),
        PageAction::Backspace | PageAction::Delete if !selection.is_caret() => Some(replace("")),
        // A single Backspace or Delete is one command, so a frame anchor's
        // character is never removed as part of a text replacement (V05).
        PageAction::Backspace => {
            previous_position(document, selection.focus)?;
            Some(Command::DeleteCharacter {
                at: selection.focus,
                forward: false,
            })
        }
        PageAction::Delete => {
            next_position(document, selection.focus)?;
            Some(Command::DeleteCharacter {
                at: selection.focus,
                forward: true,
            })
        }
        _ => None,
    }
}

/// Whether `attribute` shows at a caret: when the character on either side
/// of it has it, as Ami Pro's Text menu checks it (VM fixture C1: Bold is
/// checked just before and just after a bold word, not before the space
/// ahead of it); in an empty paragraph, when its end format over its style
/// has it, as text typed there will.
pub fn caret_attribute(
    document: &Document,
    at: TextPosition,
    attribute: CharacterAttribute,
) -> bool {
    let Some(paragraph) = document
        .flow(at.flow)
        .and_then(|flow| flow.paragraph(at.paragraph))
    else {
        return false;
    };
    let Some(style) = document.styles().get(&paragraph.style) else {
        return false;
    };
    let plain = CharacterFormat::default();
    let shows = |byte: usize| {
        let format = paragraph
            .runs
            .iter()
            .find(|run| run.start <= byte && byte < run.end)
            .map_or(&plain, |run| &run.format);
        attribute.is_on(format, style)
    };
    let byte = at.byte.min(paragraph.text.len());
    let before = paragraph.text[..byte]
        .chars()
        .next_back()
        .map(|character| byte - character.len_utf8());
    let after = (byte < paragraph.text.len()).then_some(byte);
    if before.is_none() && after.is_none() {
        return attribute.is_on(&paragraph.end_format, style);
    }
    before.into_iter().chain(after).any(shows)
}

/// A face, size, and colour from the Font dialog; `None` is the style's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FontChoice {
    pub family: Option<String>,
    pub size: Option<f32>,
    pub colour: Option<Colour>,
}

/// Formatting chosen at a caret with nothing selected, for the text typed
/// there next: in Ami Pro, Ctrl+B and then typing types bold, and Ctrl+B
/// again ends it (VM fixture C1). It lapses when the caret moves or the
/// document changes otherwise (`at` and the undo depth `depth`).
#[derive(Clone, Debug, PartialEq)]
pub struct TypingFormat {
    pub at: TextPosition,
    pub depth: usize,
    /// In the order chosen; a later choice of the same attribute replaces
    /// an earlier one.
    pub attributes: Vec<(CharacterAttribute, bool)>,
    pub font: Option<FontChoice>,
}

impl TypingFormat {
    pub fn new(at: TextPosition, depth: usize) -> Self {
        Self {
            at,
            depth,
            attributes: Vec::new(),
            font: None,
        }
    }

    /// The attribute's state for typing, if one was chosen.
    pub fn attribute(&self, attribute: CharacterAttribute) -> Option<bool> {
        self.attributes
            .iter()
            .rev()
            .find(|(chosen, _)| chosen.replaces(attribute))
            .map(|(_, on)| *on)
    }

    pub fn set(&mut self, attribute: CharacterAttribute, on: bool) {
        self.attributes
            .retain(|(chosen, _)| !chosen.replaces(attribute));
        self.attributes.push((attribute, on));
    }

    /// The commands that give text typed from `start` to `end` this
    /// formatting.
    pub fn commands(&self, start: TextPosition, end: TextPosition) -> Vec<Command> {
        let mut commands: Vec<Command> = self
            .attributes
            .iter()
            .map(|(attribute, on)| Command::SetCharacterAttribute {
                start,
                end,
                attribute: *attribute,
                on: *on,
            })
            .collect();
        if let Some(font) = &self.font {
            commands.push(Command::SetFont {
                start,
                end,
                family: font.family.clone(),
                size: font.size,
                colour: font.colour,
            });
        }
        commands
    }
}

/// Shift+arrows at a cell's edge go on into the next or previous cell, which
/// makes a cell selection (TD3).
fn crossed_cell(document: &Document, focus: TextPosition, forward: bool) -> Option<TextPosition> {
    let (frame, row, column) = crate::table_editing::cell_of(document, focus.flow)?;
    match crate::table_editing::step(document, frame, row, column, forward) {
        crate::table_editing::Step::Cell(position) => Some(position),
        _ => None,
    }
}

/// The selection after a non-editing action, or `None` when the action is
/// not a pure selection change.
pub fn moved_selection(
    document: &Document,
    selection: PageSelection,
    action: &PageAction,
) -> Option<PageSelection> {
    let page = selection.page;
    match action {
        PageAction::MoveTo {
            position,
            page,
            extend,
        } => Some(crate::table_editing::extended(
            document,
            selection,
            outside_protected(document, valid_position(document, *position)),
            *page,
            *extend,
        )),
        // The arrows step over protected text whole, and Shift selects it
        // whole (PT1).
        PageAction::MoveLeft { extend } => {
            if !extend && !selection.is_caret() {
                return Some(PageSelection::caret(selection.ordered().0, page));
            }
            let focus = selection.focus;
            let target = protected_stretches(document, focus.flow, focus.paragraph)
                .into_iter()
                .find(|stretch| stretch.end == focus.byte)
                .map(|stretch| TextPosition {
                    byte: stretch.start,
                    ..focus
                })
                .or_else(|| previous_position(document, focus))
                .or_else(|| {
                    extend
                        .then(|| crossed_cell(document, focus, false))
                        .flatten()
                })
                .unwrap_or(focus);
            Some(crate::table_editing::extended(
                document, selection, target, page, *extend,
            ))
        }
        PageAction::MoveRight { extend } => {
            if !extend && !selection.is_caret() {
                return Some(PageSelection::caret(selection.ordered().1, page));
            }
            let focus = selection.focus;
            let target = protected_stretches(document, focus.flow, focus.paragraph)
                .into_iter()
                .find(|stretch| stretch.start == focus.byte)
                .map(|stretch| TextPosition {
                    byte: stretch.end,
                    ..focus
                })
                .or_else(|| next_position(document, focus))
                .or_else(|| {
                    extend
                        .then(|| crossed_cell(document, focus, true))
                        .flatten()
                })
                .unwrap_or(focus);
            Some(crate::table_editing::extended(
                document, selection, target, page, *extend,
            ))
        }
        PageAction::SelectCells {
            frame,
            anchor,
            focus,
        } => {
            let start = crate::table_editing::cell_start(document, *frame, anchor.0, anchor.1)?;
            let end = crate::table_editing::cell_start(document, *frame, focus.0, focus.1)?;
            Some(if start.flow == end.flow {
                PageSelection::caret(start, page)
            } else {
                PageSelection {
                    anchor: start,
                    focus: end,
                    page,
                }
            })
        }
        PageAction::SelectAll => {
            let flow = selection.flow();
            Some(PageSelection {
                anchor: TextPosition {
                    flow,
                    paragraph: 0,
                    byte: 0,
                },
                focus: flow_end(document, flow),
                page,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use amipro_document::{FrameStyle, Placement, Rect};
    use amipro_editing::apply;

    use super::*;

    fn at(flow: FlowId, paragraph: usize, byte: usize) -> TextPosition {
        TextPosition {
            flow,
            paragraph,
            byte,
        }
    }

    fn document() -> (Document, FlowId) {
        let mut document = Document::from_plain_text("Café\nSecond");
        let frame = document.create_text_frame(
            Placement::Fixed {
                page: 1,
                rect: Rect::new(1_440, 1_440, 4_320, 2_880),
            },
            FrameStyle::default(),
        );
        document.frame_stack_mut().push(frame);
        let amipro_document::FrameContent::Text(flow) = document.frame(frame).unwrap().content
        else {
            unreachable!()
        };
        document
            .flow_mut(flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "Frame".into();
        (document, flow)
    }

    #[test]
    fn the_caret_steps_over_protected_text_and_never_stops_inside_it_pt1() {
        // VM fixture PT1: Right at protected text's start goes to its end,
        // Left at its end to its start, Shift selects it whole, and a click
        // inside it puts the caret after it.
        let mut document = Document::from_plain_text("ab cd ef");
        document
            .paragraph_mut(0)
            .runs
            .push(amipro_document::CharacterRun {
                start: 3,
                end: 5,
                format: CharacterFormat {
                    protected: Some(true),
                    ..CharacterFormat::default()
                },
            });
        let main = document.main_flow_id();
        assert_eq!(
            protected_stretches(&document, main, 0),
            vec![std::ops::Range { start: 3, end: 5 }]
        );
        let caret = |byte| PageSelection::caret(at(main, 0, byte), 1);
        let moved = |selection, action| moved_selection(&document, selection, &action).unwrap();
        let right = PageAction::MoveRight { extend: false };
        let left = PageAction::MoveLeft { extend: false };
        assert_eq!(moved(caret(2), right.clone()).focus.byte, 3);
        assert_eq!(moved(caret(3), right).focus.byte, 5);
        assert_eq!(moved(caret(5), left.clone()).focus.byte, 3);
        assert_eq!(moved(caret(6), left).focus.byte, 5);
        let selected = moved(caret(3), PageAction::MoveRight { extend: true });
        assert_eq!((selected.anchor.byte, selected.focus.byte), (3, 5));
        let clicked = moved(
            caret(0),
            PageAction::MoveTo {
                position: at(main, 0, 4),
                page: 1,
                extend: false,
            },
        );
        assert_eq!(clicked.focus.byte, 5);
    }

    #[test]
    fn moves_step_over_utf8_and_paragraph_boundaries_within_a_flow() {
        let (document, frame) = document();
        let main = document.main_flow_id();
        assert_eq!(
            previous_position(&document, at(main, 0, 5)),
            Some(at(main, 0, 3))
        );
        assert_eq!(
            next_position(&document, at(main, 0, 3)),
            Some(at(main, 0, 5))
        );
        assert_eq!(
            next_position(&document, at(main, 0, 5)),
            Some(at(main, 1, 0))
        );
        assert_eq!(
            previous_position(&document, at(main, 1, 0)),
            Some(at(main, 0, 5))
        );
        assert_eq!(previous_position(&document, at(main, 0, 0)), None);
        assert_eq!(
            next_position(&document, at(frame, 0, 5)),
            None,
            "a frame's text ends"
        );
        assert_eq!(flow_end(&document, frame), at(frame, 0, 5));
    }

    #[test]
    fn selections_stay_in_one_flow_and_survive_undo() {
        let (document, frame) = document();
        let main = document.main_flow_id();
        let selection = PageSelection::caret(at(main, 0, 1), 1);
        // Extending into another flow starts a new caret there.
        let moved = selection.moved(at(frame, 0, 2), 1, true);
        assert_eq!(moved, PageSelection::caret(at(frame, 0, 2), 1));
        let all = moved_selection(&document, moved, &PageAction::SelectAll).unwrap();
        assert_eq!(all.ordered(), (at(frame, 0, 0), at(frame, 0, 5)));
        assert_eq!(selected_text(&document, all).as_deref(), Some("Frame"));
        // A position past the text, or inside a character, is clamped.
        let stale = PageSelection {
            anchor: at(main, 7, 0),
            focus: at(main, 0, 4),
            page: 0,
        };
        let valid = valid_selection(&document, stale);
        assert_eq!(valid.focus, at(main, 0, 3));
        assert_eq!(valid.anchor, at(main, 1, 0));
        assert_eq!(valid.page, 1);
    }

    #[test]
    fn text_actions_become_commands_in_the_caret_flow() {
        let (document, frame) = document();
        let caret = PageSelection::caret(at(frame, 0, 5), 1);
        let command = edit_command(&document, caret, &PageAction::Insert("!".into())).unwrap();
        let applied = apply(&document, &command).unwrap();
        assert_eq!(
            applied.document.flow(frame).unwrap().paragraphs[0].text,
            "Frame!"
        );
        assert_eq!(applied.caret, Some(at(frame, 0, 6)));

        let command = edit_command(&document, caret, &PageAction::ParagraphBreak).unwrap();
        let applied = apply(&document, &command).unwrap();
        assert_eq!(applied.document.flow(frame).unwrap().paragraphs.len(), 2);

        let backspace = edit_command(&document, caret, &PageAction::Backspace).unwrap();
        assert_eq!(
            backspace,
            Command::DeleteCharacter {
                at: at(frame, 0, 5),
                forward: false
            }
        );
        assert_eq!(
            edit_command(
                &document,
                PageSelection::caret(at(frame, 0, 0), 1),
                &PageAction::Backspace
            ),
            None,
            "nothing before the start of a frame's text"
        );
        let selected = PageSelection {
            anchor: at(document.main_flow_id(), 1, 3),
            focus: at(document.main_flow_id(), 0, 1),
            page: 1,
        };
        assert_eq!(
            edit_command(&document, selected, &PageAction::Delete),
            Some(Command::ReplaceText {
                start: at(document.main_flow_id(), 0, 1),
                end: at(document.main_flow_id(), 1, 3),
                text: String::new()
            })
        );
        assert_eq!(
            selected_text(&document, selected).as_deref(),
            Some("afé\nSec")
        );
    }

    #[test]
    fn inserted_bullets_are_one_character_and_copy_as_text_bl1() {
        let mut document = Document::from_plain_text("A dot");
        let paragraph = document.paragraph_mut(0);
        paragraph.text = "A\u{FFFC}\u{FFFC}dot".into();
        paragraph.objects = vec![
            InlineObject::Bullet('\u{11}'),
            InlineObject::Bullet('\u{C}'),
        ];
        let flow = document.main_flow_id();
        let all = PageSelection {
            anchor: at(flow, 0, 0),
            focus: at(flow, 0, document.paragraphs()[0].text.len()),
            page: 1,
        };
        assert_eq!(
            selected_text(&document, all).as_deref(),
            Some("A\u{2022}\u{2611}dot")
        );
        // Backspace after the second bullet removes it alone.
        let after = 1 + 2 * '\u{FFFC}'.len_utf8();
        let command = edit_command(
            &document,
            PageSelection::caret(at(flow, 0, after), 1),
            &PageAction::Backspace,
        )
        .unwrap();
        let applied = amipro_editing::apply(&document, &command).unwrap().document;
        let paragraph = &applied.paragraphs()[0];
        assert_eq!(paragraph.text, "A\u{FFFC}dot");
        assert_eq!(paragraph.objects, [InlineObject::Bullet('\u{11}')]);
    }
}
