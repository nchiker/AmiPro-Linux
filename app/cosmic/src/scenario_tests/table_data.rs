//! Table data (run C): numbers in table cells shown and typed through the
//! paragraph style's number format (TD.5; `UI_SPEC.md` §41,
//! `docs/research/TABLE_DATA_NOTES.md` section 11). One checker runs after
//! every step; the later slices (formulas, Quick Add, Sort) add their
//! scenarios here.

use super::*;
use crate::style_properties::{Field as StyleField, StyleAction};
use amipro_document::{Alignment, NumberKind};

/// A 2 × 3 table of empty cells whose Table Text style has `flags` and
/// `kind`, 2 decimals.
fn numbers_document(kind: NumberKind, flags: u32) -> (Document, FrameId) {
    numbers_table(kind, flags, 2, 3)
}

/// A `columns` × `rows` table of empty cells whose Table Text style has
/// `flags` and `kind`, 2 decimals.
fn numbers_table(kind: NumberKind, flags: u32, columns: usize, rows: usize) -> (Document, FrameId) {
    use amipro_editing::tables::{NewTable, TableTarget};
    let document = Document::from_plain_text("Prices");
    let start = at(document.main_flow_id(), 0, 6);
    let mut document = amipro_editing::apply(
        &document,
        &amipro_editing::Command::InsertTable {
            target: TableTarget::Text(start),
            table: NewTable::fitting(columns, rows, 9_360, 284),
        },
    )
    .unwrap()
    .document;
    let frame = *document.frame_stack().last().unwrap();
    let flow = cell(&document, frame, 0, 0);
    let style = document.flow(flow).unwrap().paragraphs[0].style;
    let format = &mut document
        .styles_mut()
        .get_mut(&style)
        .unwrap()
        .kept
        .number_format;
    format.kind = kind;
    format.flags = flags;
    format.decimals = 2;
    (document, frame)
}

fn flow_of(s: &Scenario, frame: FrameId, row: usize, column: usize) -> FlowId {
    cell(s.document(), frame, row, column)
}

fn paragraph_of(s: &Scenario, flow: FlowId) -> std::sync::Arc<amipro_document::Paragraph> {
    s.document().flow(flow).unwrap().paragraphs[0].clone()
}

/// What the page draws for a cell's number: its shown text and whether it
/// is red.
fn shown(s: &Scenario, flow: FlowId) -> Option<(String, bool)> {
    s.app().pages.layout().pages.iter().find_map(|page| {
        page.lines
            .iter()
            .filter(|line| line.flow == flow)
            .find_map(|line| {
                line.words
                    .iter()
                    .find_map(|word| word.text.clone().map(|text| (text, word.red)))
            })
    })
}

fn into_cell(s: &mut Scenario, frame: FrameId, row: usize, column: usize) {
    let flow = flow_of(s, frame, row, column);
    let end = paragraph_of(s, flow).text.len();
    s.caret(flow, 0, end, 1, false);
}

/// Keys typed one by one, as the window delivers them: a run of typing.
fn keys(rt: &mut Runtime, text: &str) {
    rt.app.page_keyboard += 1;
    for character in text.chars() {
        rt.events(&[key_press(
            cosmic::iced::keyboard::Key::Character(character.to_string().into()),
            cosmic::iced::keyboard::Modifiers::default(),
        )]);
    }
}

/// Tab as the page surface sends it.
fn tab(s: &mut Scenario) {
    s.act("Tab", |rt| {
        rt.send(Message::Page(PageAction::CellStep { forward: true }));
    });
}

fn expect_text(s: &Scenario, flow: FlowId, text: &str, right: bool) {
    let paragraph = paragraph_of(s, flow);
    let aligned = paragraph.format.alignment == Some(Alignment::Right);
    if paragraph.text != text || aligned != right {
        s.fail(
            "cell",
            &format!(
                "the cell holds {:?} (right {aligned}), expected {text:?} (right {right})",
                paragraph.text
            ),
        );
    }
}

/// Typing numbers under Currency (leading symbol, thousands, leading minus,
/// red): the number is rewritten as `%.*f` and right-aligned when the caret
/// leaves, in the typing's own undo step; the page shows it through the
/// style; text stays as typed; the stored text is what Save As keeps.
#[test]
fn typed_numbers_are_stored_plain_and_shown_through_the_style_td5() {
    let (document, frame) = numbers_document(NumberKind::Currency, 0x198);
    let mut s = Scenario::new("table-data-numbers", document);
    s.clean_folder();
    let (a1, b1, a2, b2) = (
        flow_of(&s, frame, 0, 0),
        flow_of(&s, frame, 0, 1),
        flow_of(&s, frame, 1, 0),
        flow_of(&s, frame, 1, 1),
    );

    let before = s.app().history.undo_len();
    into_cell(&mut s, frame, 0, 0);
    s.act("type 1234.5", |rt| keys(rt, "1234.5"));
    // While the caret is in the cell the number is as typed.
    expect_text(&s, a1, "1234.5", false);
    tab(&mut s);
    expect_text(&s, a1, "1234.50", true);
    if s.app().history.undo_len() != before + 1 {
        s.fail("Tab", "typing and the rewrite are not one undo step");
    }
    if shown(&s, a1) != Some(("$1,234.50".into(), false)) {
        s.fail("Tab", &format!("A1 shows {:?}", shown(&s, a1)));
    }

    // A negative: red under this format, rounded to two decimals.
    s.act("type -7.256", |rt| keys(rt, "-7.256"));
    tab(&mut s);
    expect_text(&s, b1, "-7.26", true);
    if shown(&s, b1) != Some(("-$7.26".into(), true)) {
        s.fail("Tab", &format!("B1 shows {:?}", shown(&s, b1)));
    }

    // Text stays as typed: a percent sign, a comma, a leading space.
    s.act("type 5%", |rt| keys(rt, "5%"));
    tab(&mut s);
    expect_text(&s, a2, "5%", false);
    if shown(&s, a2).is_some() {
        s.fail("Tab", "a text cell is drawn through the style");
    }

    // Leading zeros and a plus sign go; the caret leaving by a click.
    s.act("type 007", |rt| keys(rt, "007"));
    let target = flow_of(&s, frame, 0, 0);
    s.act("click away to A1", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(target, 0, 0),
            page: 1,
            extend: false,
        }));
    });
    expect_text(&s, b2, "7.00", true);
    if shown(&s, b2) != Some(("$7.00".into(), false)) {
        s.fail("click away", &format!("B2 shows {:?}", shown(&s, b2)));
    }

    // Undo takes the last cell back to empty in one step; Redo returns it.
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    expect_text(&s, b2, "", false);
    s.act("Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    expect_text(&s, b2, "7.00", true);

    // A real click on the formatted number puts the caret in its stored
    // text, at the end; the cell then shows as stored.
    s.act("caret to B1", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(b1, 0, 0),
            page: 1,
            extend: false,
        }));
    });
    s.act("click on A1's number", |rt| {
        let line = rt.app.pages.layout().pages[0]
            .lines
            .iter()
            .find(|line| line.flow == a1)
            .cloned()
            .unwrap();
        let word = &line.words[0];
        let point = rt.on_page(word.x + 20.0, line.top + line.height / 2.0);
        rt.click(cosmic::iced::mouse::Button::Left, point);
    });
    let focus = s.app().page_selection.focus;
    if (focus.flow, focus.byte) != (a1, "1234.50".len()) {
        s.fail("click", &format!("the caret is at {focus:?}"));
    }

    // Saved: the file keeps the plain text and the right-alignment code;
    // it reads back equal, and a text cell is not marked.
    let path = s.save_sam("Numbers.SAM");
    let bytes = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    for stored in ["<+A>1234.50", "<+A>-7.26", "<+A>7.00"] {
        if !text.contains(stored) {
            s.fail("Save As", &format!("the file lacks {stored}"));
        }
    }
    if text.contains("<+A>5%") {
        s.fail("Save As", "a text cell was given the alignment code");
    }
    s.reopen(&path);
}

/// Changing the Table Text style's Table format in the Styles rail changes
/// how every number in the table shows, one undo step each, the stored
/// text untouched.
#[test]
fn the_styles_table_format_changes_every_cell_that_uses_it_td5() {
    let (mut document, frame) = numbers_document(NumberKind::General, 0x118);
    for (row, column, text) in [(0, 0, "1234.50"), (0, 1, "-7.25"), (1, 0, "abc")] {
        let flow = cell(&document, frame, row, column);
        document
            .flow_mut(flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = text.into();
    }
    let mut s = Scenario::new("table-data-format", document);
    s.clean_folder();
    let (a1, b1, a2) = (
        flow_of(&s, frame, 0, 0),
        flow_of(&s, frame, 0, 1),
        flow_of(&s, frame, 1, 0),
    );
    if shown(&s, a1) != Some(("1,234.5".into(), false)) {
        s.fail("General", &format!("A1 shows {:?}", shown(&s, a1)));
    }
    // The Styles rail for the cells' style.
    into_cell(&mut s, frame, 0, 0);
    s.command_act(CommandId::ModifyCurrentStyle);
    let style = paragraph_of(&s, a1).style;
    if s.app().style_properties.as_ref().map(|panel| panel.style) != Some(style) {
        s.fail(
            "Style Properties",
            "the rail does not show the cells' style",
        );
    }
    for (what, action, expected) in [
        (
            "Currency",
            StyleAction::CellFormat(2),
            ("$1,234.50", "-$7.25"),
        ),
        (
            "Parentheses",
            StyleAction::Parentheses(true),
            ("$1,234.50", "-$(7.25)"),
        ),
        (
            "Trailing minus",
            StyleAction::TrailingMinus(true),
            ("$1,234.50", "$(7.25)-"),
        ),
        ("red", StyleAction::Red(true), ("$1,234.50", "$(7.25)-")),
    ] {
        s.edit(what, move |rt| {
            rt.send(Message::StyleProperties(action));
        });
        if shown(&s, a1).map(|(text, _)| text).as_deref() != Some(expected.0)
            || shown(&s, b1).map(|(text, _)| text).as_deref() != Some(expected.1)
        {
            s.fail(
                what,
                &format!("A1 {:?}, B1 {:?}", shown(&s, a1), shown(&s, b1)),
            );
        }
        if what == "red" && !shown(&s, b1).is_some_and(|(_, red)| red) {
            s.fail(what, "a negative is not red");
        }
    }
    s.edit("decimals 0", |rt| {
        rt.send(Message::StyleProperties(StyleAction::Text(
            StyleField::Decimals,
            "0".into(),
        )));
        rt.send(Message::StyleProperties(StyleAction::Submit(
            StyleField::Decimals,
        )));
    });
    if shown(&s, a1).map(|(text, _)| text).as_deref() != Some("$1,235") {
        s.fail("decimals", &format!("A1 shows {:?}", shown(&s, a1)));
    }
    // The text cell and the stored text are untouched.
    expect_text(&s, a1, "1234.50", false);
    expect_text(&s, a2, "abc", false);
    if shown(&s, a2).is_some() {
        s.fail("decimals", "a text cell is drawn through the style");
    }
    // The style sheet record keeps what Ami Pro writes: six values.
    let format = &s.document().styles()[&style].kept.number_format;
    if format.to_values()[1] != "3" || format.to_values()[2] != "0" {
        s.fail("nfmt", &format!("{:?}", format.to_values()));
    }
    let path = s.save_sam("Format.SAM");
    s.reopen(&path);
}

// ---- TD.7: formulas, Quick Add, errors, gestures --------------------------

use crate::table_properties::TableAction;

/// The page a table's row is on (page 1 when it is not laid out).
fn page_of(s: &Scenario, frame: FrameId, row: usize) -> u32 {
    table_rows_by_page(s.app(), frame)
        .into_iter()
        .find(|(_, rows)| rows.contains(&row))
        .map_or(1, |(page, _)| page)
}

/// The caret at the end of a cell's text, on the page the cell is on.
fn to_cell(s: &mut Scenario, frame: FrameId, row: usize, column: usize) {
    let flow = flow_of(s, frame, row, column);
    let page = page_of(s, frame, row);
    let end = paragraph_of(s, flow).text.len();
    s.act(&format!("caret to ({row}, {column})"), move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(flow, 0, end),
            page,
            extend: false,
        }));
    });
}

/// Types `text` over what a cell holds (it stays in the cell; the next
/// step that moves the caret leaves it).
fn put(s: &mut Scenario, frame: FrameId, row: usize, column: usize, text: &str) {
    let flow = flow_of(s, frame, row, column);
    let page = page_of(s, frame, row);
    let length = paragraph_of(s, flow).text.len();
    s.act(&format!("select ({row}, {column})"), move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(flow, 0, 0),
            page,
            extend: false,
        }));
        if length > 0 {
            rt.send(Message::Page(PageAction::MoveTo {
                position: at(flow, 0, length),
                page,
                extend: true,
            }));
        }
    });
    let text = text.to_owned();
    s.act(&format!("type {text:?}"), move |rt| keys(rt, &text));
}

/// The caret leaves the table for the start of the main text.
fn leave_table(s: &mut Scenario) {
    let main = s.main_flow();
    s.act("caret to the main text", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(main, 0, 0),
            page: 1,
            extend: false,
        }));
    });
}

/// The Formula section's box typed in and applied (Enter).
fn set_formula(s: &mut Scenario, frame: FrameId, row: usize, column: usize, text: &str) {
    to_cell(s, frame, row, column);
    let text = text.to_owned();
    s.act(&format!("formula {text:?}"), move |rt| {
        rt.send(Message::TableProperties(TableAction::FormulaText(text)));
        rt.send(Message::TableProperties(TableAction::FormulaSubmit));
    });
}

/// Table Properties in the rail, for the table.
fn open_properties(s: &mut Scenario, frame: FrameId) {
    to_cell(s, frame, 0, 0);
    s.command_act(CommandId::TableProperties);
    if s.app().table_properties.is_none() {
        s.fail("Table Properties", "the panel did not open");
    }
}

fn text_in(s: &Scenario, frame: FrameId, row: usize, column: usize) -> String {
    paragraph_of(s, flow_of(s, frame, row, column)).text.clone()
}

fn formula_in(s: &Scenario, frame: FrameId, row: usize, column: usize) -> Option<String> {
    let Some(FrameContent::Table(table)) = s.document().frame(frame).map(|frame| &frame.content)
    else {
        s.fail("table", "the frame holds no table");
    };
    table.cells[row * table.columns.len() + column]
        .formula
        .clone()
}

fn expect_cells(s: &Scenario, frame: FrameId, expected: &[(usize, usize, &str)]) {
    for (row, column, text) in expected {
        let now = text_in(s, frame, *row, *column);
        if now != *text {
            s.fail(
                "cells",
                &format!("({row}, {column}) holds {now:?}, expected {text:?}"),
            );
        }
    }
}

fn expect_formula(s: &Scenario, frame: FrameId, row: usize, column: usize, want: Option<&str>) {
    let now = formula_in(s, frame, row, column);
    if now.as_deref() != want {
        s.fail(
            "formula",
            &format!("({row}, {column}) has {now:?}, expected {want:?}"),
        );
    }
}

fn expect_status_has(s: &Scenario, words: &str) {
    if !s.app().status.contains(words) {
        s.fail(
            "status",
            &format!(
                "the status bar reads {:?}, expected {words:?}",
                s.app().status
            ),
        );
    }
}

/// Every cell of the document's last table: its text and formula.
fn snapshot(document: &Document) -> Vec<(String, Option<String>)> {
    let frame = document
        .frame_stack()
        .iter()
        .rev()
        .find(|id| {
            matches!(
                document.frame(**id).map(|frame| &frame.content),
                Some(FrameContent::Table(_))
            )
        })
        .copied()
        .expect("a table");
    let Some(FrameContent::Table(table)) = document.frame(frame).map(|frame| &frame.content) else {
        unreachable!()
    };
    table
        .cells
        .iter()
        .map(|cell| {
            let amipro_document::CellContent::Text(flow) = cell.content;
            (
                document.flow(flow).unwrap().paragraphs[0].text.clone(),
                cell.formula.clone(),
            )
        })
        .collect()
}

/// An invoice in a 4 × 5 table: item, quantity, price, a product formula
/// per line, and a total (`UI_SPEC.md` §41).
fn build_invoice(s: &mut Scenario, frame: FrameId) {
    for (column, head) in ["Item", "Qty", "Price", "Total"].iter().enumerate() {
        put(s, frame, 0, column, head);
    }
    for (line, row) in [
        ["Pens", "3", "19.99"],
        ["Ink", "10", "5"],
        ["Paper", "2", "12.5"],
    ]
    .iter()
    .zip(1..)
    {
        for (column, text) in line.iter().enumerate() {
            put(s, frame, row, column, text);
        }
    }
    put(s, frame, 4, 0, "Total");
    open_properties(s, frame);
    for row in 1..=3 {
        set_formula(s, frame, row, 3, &format!("B{0}*C{0}", row + 1));
    }
    set_formula(s, frame, 4, 3, "@sum(D2..D4)");
}

/// Scenario 1: quantities and prices typed, a product per row and a total
/// by formula; a changed price changes its dependants in the same undo
/// step; Save As SAM, reopen, the results equal.
#[test]
fn an_invoice_calculates_and_follows_a_changed_price_td7() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 4, 5);
    let mut s = Scenario::new("table-data-invoice", document);
    s.clean_folder();
    build_invoice(&mut s, frame);
    expect_cells(
        &s,
        frame,
        &[
            (1, 1, "3.00"),
            (1, 2, "19.99"),
            (3, 2, "12.50"),
            (1, 3, "59.97"),
            (2, 3, "50.00"),
            (3, 3, "25.00"),
            (4, 3, "134.97"),
        ],
    );
    expect_formula(&s, frame, 4, 3, Some("@sum(D2..D4)"));

    // A price changed: the product and the total follow in the same step.
    let undo = s.app().history.undo_len();
    put(&mut s, frame, 1, 2, "25");
    expect_cells(&s, frame, &[(1, 2, "25"), (1, 3, "59.97")]);
    tab(&mut s);
    expect_cells(
        &s,
        frame,
        &[(1, 2, "25.00"), (1, 3, "75.00"), (4, 3, "150.00")],
    );
    if s.app().history.undo_len() != undo + 1 {
        s.fail("Tab", "the price and its dependants are not one undo step");
    }
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    expect_cells(
        &s,
        frame,
        &[(1, 2, "19.99"), (1, 3, "59.97"), (4, 3, "134.97")],
    );
    if s.app().history.undo_len() != undo {
        s.fail("Undo", "one Undo did not return the price and the totals");
    }
    s.act("Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    expect_cells(&s, frame, &[(1, 2, "25.00"), (1, 3, "75.00")]);

    // The caret leaving by a click: the same.
    put(&mut s, frame, 2, 1, "4");
    to_cell(&mut s, frame, 4, 0);
    expect_cells(
        &s,
        frame,
        &[(2, 1, "4.00"), (2, 3, "20.00"), (4, 3, "120.00")],
    );

    let before = snapshot(s.document());
    let path = s.save_sam("Invoice.SAM");
    let fresh = s.reopen(&path);
    if snapshot(fresh.document()) != before {
        s.fail("reopen", "the reopened results or formulas differ");
    }
    if fresh.app().dirty {
        s.fail("reopen", "the opened file is marked changed");
    }
}

/// Scenario 12: a file as Ami Pro leaves it, results stored (one of them
/// not what the formula gives, VM TDF6): opened as it is, nothing marked
/// changed, Save As lists nothing.
#[test]
fn a_file_with_stored_results_opens_unchanged_td7() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 4, 5);
    let mut s = Scenario::new("table-data-stored", document);
    s.clean_folder();
    build_invoice(&mut s, frame);
    let path = s.save_sam("Original.SAM");
    let bytes = std::fs::read(&path).unwrap();
    let needle = b"59.97";
    let spot = bytes
        .windows(needle.len())
        .position(|window| window == needle)
        .expect("the stored result");
    let mut stale = bytes.clone();
    stale[spot..spot + 2].copy_from_slice(b"99");
    let patched = s.folder().join("Stale.SAM");
    std::fs::write(&patched, stale).unwrap();
    let mut fresh = s.reopen(&patched);
    let frame = *fresh
        .document()
        .frame_stack()
        .iter()
        .find(|id| {
            matches!(
                fresh.document().frame(**id).map(|frame| &frame.content),
                Some(FrameContent::Table(_))
            )
        })
        .unwrap();
    expect_cells(&fresh, frame, &[(1, 3, "99.97"), (4, 3, "134.97")]);
    expect_formula(&fresh, frame, 1, 3, Some("B2*C2"));
    if fresh.app().dirty {
        fresh.fail("Open", "the document is marked changed");
    }
    fresh.save_sam("Resaved.SAM");
}

/// Scenario 2: Quick Add Row and Column in a table with a heading row and
/// blank cells; the formula written; Edit Formula; Del takes it out.
#[test]
fn quick_add_writes_the_sums_and_edit_formula_changes_them_td7() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 4, 4);
    let mut s = Scenario::new("table-data-quick-add", document);
    s.clean_folder();
    for (column, head) in ["Name", "Jan", "Feb", "Total"].iter().enumerate() {
        put(&mut s, frame, 0, column, head);
    }
    for (column, text) in ["a", "10", "20"].iter().enumerate() {
        put(&mut s, frame, 1, column, text);
    }
    for (column, text) in ["b", "5"].iter().enumerate() {
        put(&mut s, frame, 2, column, text);
    }
    open_properties(&mut s, frame);

    // When each command is on.
    let on = |s: &Scenario, command| s.app().command_enabled(command);
    to_cell(&mut s, frame, 0, 0);
    if !on(&s, CommandId::EditFormula)
        || on(&s, CommandId::QuickAddRow)
        || on(&s, CommandId::QuickAddColumn)
        || on(&s, CommandId::RecalculateTable)
    {
        s.fail("menu", "the first cell: only Edit Formula is on");
    }
    to_cell(&mut s, frame, 1, 0);
    if on(&s, CommandId::QuickAddRow) || !on(&s, CommandId::QuickAddColumn) {
        s.fail("menu", "the first column has no cell to its left");
    }
    to_cell(&mut s, frame, 0, 2);
    if !on(&s, CommandId::QuickAddRow) || on(&s, CommandId::QuickAddColumn) {
        s.fail("menu", "the first row has no cell above");
    }
    leave_table(&mut s);
    if on(&s, CommandId::EditFormula) || on(&s, CommandId::QuickAddRow) {
        s.fail("menu", "outside a table the formula commands are off");
    }

    // Quick Add Row sums leftwards from after the text cell.
    to_cell(&mut s, frame, 1, 3);
    s.command(CommandId::QuickAddRow);
    expect_formula(&s, frame, 1, 3, Some("@sum(B2..C2)"));
    expect_cells(&s, frame, &[(1, 3, "30.00")]);
    to_cell(&mut s, frame, 2, 3);
    s.command(CommandId::QuickAddRow);
    expect_formula(&s, frame, 2, 3, Some("@sum(B3..C3)"));
    expect_cells(&s, frame, &[(2, 3, "5.00")]);
    // Quick Add Column sums upwards, a blank cell in the range.
    to_cell(&mut s, frame, 3, 1);
    s.command(CommandId::QuickAddColumn);
    expect_formula(&s, frame, 3, 1, Some("@sum(B2..B3)"));
    expect_cells(&s, frame, &[(3, 1, "15.00")]);
    to_cell(&mut s, frame, 3, 2);
    s.command(CommandId::QuickAddColumn);
    expect_formula(&s, frame, 3, 2, Some("@sum(C2..C3)"));
    expect_cells(&s, frame, &[(3, 2, "20.00")]);
    to_cell(&mut s, frame, 3, 3);
    s.command(CommandId::QuickAddColumn);
    expect_formula(&s, frame, 3, 3, Some("@sum(D2..D3)"));
    expect_cells(&s, frame, &[(3, 3, "35.00")]);

    // Edit Formula…: the Formula box has the keys, its text selected.
    s.act("Edit Formula…", |rt| {
        rt.send(Message::Command(CommandId::EditFormula));
    });
    if !s.app().table_properties.as_ref().is_some_and(|state| {
        state
            .open
            .contains(&crate::table_properties::Section::Formula)
    }) {
        s.fail("Edit Formula", "the Formula section is not open");
    }
    s.act("type into the box", |rt| {
        use cosmic::iced::keyboard::{Key, Modifiers};
        let mut sent = Vec::new();
        for character in "B4*2".chars() {
            sent.extend(rt.events(&[key_press(
                Key::Character(character.to_string().into()),
                Modifiers::default(),
            )]));
        }
        assert!(
            sent.iter().any(|message| matches!(
                message,
                Message::TableProperties(TableAction::FormulaText(text)) if text == "B4*2"
            )),
            "the keys did not replace the selected formula: {sent:?}"
        );
    });
    s.edit("Enter in the box", |rt| {
        rt.send(Message::TableProperties(TableAction::FormulaSubmit));
    });
    expect_formula(&s, frame, 3, 3, Some("B4*2"));
    expect_cells(&s, frame, &[(3, 3, "30.00")]);

    // Del on selected cells takes their formulas out; the cells are empty.
    s.act("select two cells", move |rt| {
        rt.send(Message::Page(PageAction::SelectCells {
            frame,
            anchor: (3, 2),
            focus: (3, 3),
        }));
    });
    s.edit("Del", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    expect_formula(&s, frame, 3, 2, None);
    expect_formula(&s, frame, 3, 3, None);
    expect_cells(&s, frame, &[(3, 2, ""), (3, 3, "")]);
    let path = s.save_sam("QuickAdd.SAM");
    s.reopen(&path);
}

/// Scenario 5: a divide by zero and a loop say so in the status bar and
/// leave the cell blank; the rest still calculates; fixing the cause clears
/// the message; a formula that reads text is REF, drawn, without a message.
#[test]
fn errors_go_to_the_status_bar_and_the_rest_still_calculates_td7() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 4, 4);
    let mut s = Scenario::new("table-data-messages", document);
    s.clean_folder();
    put(&mut s, frame, 0, 1, "8");
    put(&mut s, frame, 0, 2, "0");
    put(&mut s, frame, 1, 0, "abc");
    open_properties(&mut s, frame);

    set_formula(&mut s, frame, 0, 3, "B1/C1");
    expect_status_has(&s, "divide by zero");
    expect_cells(&s, frame, &[(0, 3, "")]);
    expect_formula(&s, frame, 0, 3, Some("B1/C1"));
    // The rest of the table calculates.
    set_formula(&mut s, frame, 1, 3, "@sum(B1..C1)");
    expect_cells(&s, frame, &[(1, 3, "8.00")]);
    // Fixing the cause clears the message.
    put(&mut s, frame, 0, 2, "4");
    tab(&mut s);
    expect_cells(&s, frame, &[(0, 3, "2.00")]);
    if s.app().status.contains("divide") {
        s.fail("fix", "the divide by zero message stayed");
    }

    // A loop: both cells are blank and the message is shown.
    set_formula(&mut s, frame, 2, 0, "A4");
    set_formula(&mut s, frame, 3, 0, "A3");
    expect_status_has(&s, "circular formula");
    expect_cells(&s, frame, &[(2, 0, ""), (3, 0, "")]);
    set_formula(&mut s, frame, 1, 3, "@sum(B1..C1)+1");
    expect_cells(&s, frame, &[(1, 3, "13.00")]);
    to_cell(&mut s, frame, 3, 0);
    s.act("Remove formula", |rt| {
        rt.send(Message::TableProperties(TableAction::FormulaRemove));
    });
    expect_formula(&s, frame, 3, 0, None);
    expect_cells(&s, frame, &[(2, 0, "0.00")]);
    if s.app().status.contains("circular") {
        s.fail("fix", "the loop message stayed");
    }

    // A formula the compiler refuses: the message in the status bar and
    // under the box, nothing stored, Esc gives the old text back.
    s.expect_status("cannot understand");
    set_formula(&mut s, frame, 2, 2, "foo(");
    expect_status_has(&s, "Cannot understand the formula in cell 'C3'");
    expect_formula(&s, frame, 2, 2, None);
    if !s
        .app()
        .table_properties
        .as_ref()
        .is_some_and(|state| state.error.is_some() && state.formula.as_deref() == Some("foo("))
    {
        s.fail("refused", "the Formula section does not show the refusal");
    }
    s.act("Esc", |rt| {
        rt.send(Message::Escape);
    });
    if s.app()
        .table_properties
        .as_ref()
        .is_some_and(|state| state.formula.is_some())
    {
        s.fail("Esc", "the typed formula stayed");
    }
    s.clear_expected_status();

    // REF: a formula reading a text cell stores an empty result, which the
    // page draws as the word REF; no message.
    set_formula(&mut s, frame, 1, 2, "A2+1");
    expect_cells(&s, frame, &[(1, 2, "")]);
    let flow = flow_of(&s, frame, 1, 2);
    if shown(&s, flow) != Some(("REF".into(), false)) {
        s.fail("REF", &format!("the cell shows {:?}", shown(&s, flow)));
    }
    if s.app().status.contains("REF") || s.app().status.contains("refers") {
        s.fail("REF", "Ami Pro says nothing about REF");
    }
    leave_table(&mut s);
    let path = s.save_sam("Messages.SAM");
    let fresh = s.reopen(&path);
    let frame = *fresh
        .document()
        .frame_stack()
        .iter()
        .find(|id| {
            matches!(
                fresh.document().frame(**id).map(|frame| &frame.content),
                Some(FrameContent::Table(_))
            )
        })
        .unwrap();
    let flow = flow_of(&fresh, frame, 1, 2);
    if shown(&fresh, flow) != Some(("REF".into(), false)) {
        fresh.fail("reopen", &format!("REF shows {:?}", shown(&fresh, flow)));
    }
}

/// Scenario 6: a formula copied right and down keeps its `$` parts and
/// shifts the rest; Delete Column inside a range and Insert Row above one
/// rewrite the formulas.
#[test]
fn copied_formulas_shift_and_row_and_column_edits_rewrite_them_td7() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 6, 6);
    let mut s = Scenario::new("table-data-rewrite", document);
    s.clean_folder();
    for row in 0..5 {
        put(&mut s, frame, row, 0, &(row + 1).to_string());
    }
    for (column, text) in [(1, "10"), (2, "20"), (3, "30")] {
        put(&mut s, frame, 4, column, text);
    }
    open_properties(&mut s, frame);
    set_formula(&mut s, frame, 0, 1, "A$1+$A2");
    set_formula(&mut s, frame, 0, 2, "@sum(A1..A2)");
    set_formula(&mut s, frame, 4, 5, "@sum(A5..E5)");
    set_formula(&mut s, frame, 0, 5, "@sum(A2..A5)");
    expect_cells(
        &s,
        frame,
        &[
            (0, 1, "3.00"),
            (0, 2, "3.00"),
            (4, 5, "65.00"),
            (0, 5, "14.00"),
        ],
    );

    // B1 and C1 together: down two rows, the `$` parts stay.
    let select = |s: &mut Scenario| {
        s.act("select B1:C1", move |rt| {
            rt.send(Message::Page(PageAction::SelectCells {
                frame,
                anchor: (0, 1),
                focus: (0, 2),
            }));
        });
    };
    select(&mut s);
    s.command_act(CommandId::Copy);
    to_cell(&mut s, frame, 2, 1);
    s.command(CommandId::Paste);
    expect_formula(&s, frame, 2, 1, Some("A$1+$A4"));
    expect_formula(&s, frame, 2, 2, Some("@sum(A3..A4)"));
    expect_cells(&s, frame, &[(2, 1, "5.00"), (2, 2, "7.00")]);
    // The same one right and one down.
    select(&mut s);
    s.command_act(CommandId::Copy);
    to_cell(&mut s, frame, 1, 2);
    s.command(CommandId::Paste);
    expect_formula(&s, frame, 1, 2, Some("B$1+$A3"));
    expect_formula(&s, frame, 1, 3, Some("@sum(B2..B3)"));
    expect_cells(&s, frame, &[(1, 2, "6.00"), (1, 3, "5.00")]);

    // Delete Column C, inside the row 5 range and holding formulas.
    to_cell(&mut s, frame, 3, 2);
    s.command(CommandId::DeleteColumn);
    expect_formula(&s, frame, 4, 4, Some("@sum(A5..D5)"));
    expect_cells(&s, frame, &[(4, 4, "45.00")]);
    // Insert Row Above row 3: ranges reaching past it stretch.
    to_cell(&mut s, frame, 2, 0);
    s.command(CommandId::InsertRowAbove);
    expect_formula(&s, frame, 0, 4, Some("@sum(A2..A6)"));
    expect_formula(&s, frame, 5, 4, Some("@sum(A6..D6)"));
    expect_formula(&s, frame, 3, 1, Some("A$1+$A5"));
    expect_formula(&s, frame, 1, 2, Some("@sum(B2..B4)"));
    expect_cells(&s, frame, &[(0, 4, "14.00"), (5, 4, "45.00")]);
    let path = s.save_sam("Rewrite.SAM");
    s.reopen(&path);
}

/// Scenario 7: a formula before its data: empty cells count as 0 and the
/// result fills in as the data is typed.
#[test]
fn a_formula_before_its_data_fills_in_td7() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 3, 4);
    let mut s = Scenario::new("table-data-before", document);
    s.clean_folder();
    open_properties(&mut s, frame);
    set_formula(&mut s, frame, 0, 0, "@sum(B1..B3)");
    expect_cells(&s, frame, &[(0, 0, "0.00")]);
    put(&mut s, frame, 0, 1, "4");
    put(&mut s, frame, 1, 1, "6");
    expect_cells(&s, frame, &[(0, 0, "4.00")]);
    leave_table(&mut s);
    expect_cells(&s, frame, &[(0, 0, "10.00"), (1, 1, "6.00")]);
    let path = s.save_sam("Before.SAM");
    s.reopen(&path);
}

/// A table in a frame, with a text frame made in the document.
fn frame_table_document() -> (Document, FrameId) {
    use amipro_editing::tables::{NewTable, TableTarget};
    let document = Document::from_plain_text("A frame holds the table");
    let made = amipro_editing::apply(
        &document,
        &amipro_editing::Command::CreateFrame {
            placement: amipro_document::Placement::Fixed {
                page: 1,
                rect: amipro_document::Rect::new(1_440, 2_880, 7_200, 5_760),
            },
            style: Default::default(),
            content: amipro_editing::NewFrameContent::Text,
            anchor: None,
            layout: None,
        },
    )
    .unwrap()
    .document;
    let frame = made.frame_stack()[0];
    let mut with = amipro_editing::apply(
        &made,
        &amipro_editing::Command::InsertTable {
            target: TableTarget::Frame(frame),
            table: NewTable::fitting(3, 3, 5_760 - 288, 284),
        },
    )
    .unwrap()
    .document;
    let flow = cell(&with, frame, 0, 0);
    let style = with.flow(flow).unwrap().paragraphs[0].style;
    let format = &mut with
        .styles_mut()
        .get_mut(&style)
        .unwrap()
        .kept
        .number_format;
    format.kind = NumberKind::Fixed;
    format.flags = 0x18;
    format.decimals = 2;
    (with, frame)
}

/// Scenario 8: formulas in a frame's table and in a page table that runs
/// over two pages.
#[test]
fn formulas_work_in_a_frames_table_and_across_pages_td7() {
    let (document, frame) = frame_table_document();
    let mut s = Scenario::new("table-data-frame", document);
    s.clean_folder();
    put(&mut s, frame, 0, 0, "5");
    put(&mut s, frame, 0, 1, "7");
    open_properties(&mut s, frame);
    set_formula(&mut s, frame, 0, 2, "A1+B1");
    expect_cells(
        &s,
        frame,
        &[(0, 0, "5.00"), (0, 1, "7.00"), (0, 2, "12.00")],
    );
    put(&mut s, frame, 0, 0, "1");
    tab(&mut s);
    expect_cells(&s, frame, &[(0, 2, "8.00")]);
    let path = s.save_sam("FrameTable.SAM");
    s.reopen(&path);

    // A page table of 60 rows: the total on the second page.
    let (mut document, frame) = numbers_table(NumberKind::Fixed, 0x18, 2, 60);
    for row in 0..59 {
        let flow = cell(&document, frame, row, 0);
        document
            .flow_mut(flow)
            .unwrap()
            .paragraph_mut(0)
            .unwrap()
            .text = "1.00".into();
    }
    let mut s = Scenario::new("table-data-pages", document);
    s.clean_folder();
    if page_of(&s, frame, 59) < 2 {
        s.fail("layout", "the table does not reach a second page");
    }
    open_properties(&mut s, frame);
    set_formula(&mut s, frame, 59, 0, "@sum(A1..A59)");
    expect_cells(&s, frame, &[(59, 0, "59.00")]);
    let undo = s.app().history.undo_len();
    put(&mut s, frame, 0, 0, "11");
    tab(&mut s);
    expect_cells(&s, frame, &[(0, 0, "11.00"), (59, 0, "69.00")]);
    if s.app().history.undo_len() != undo + 1 {
        s.fail(
            "Tab",
            "the change on page 1 and the total on page 2 are two steps",
        );
    }
    let path = s.save_sam("PageTable.SAM");
    s.reopen(&path);
}

/// The Formula section's and the cell's gestures: typing in a formula cell
/// is refused with the status bar's hint, a double-click is Edit Formula,
/// Esc gives the old formula back, Remove formula and Recalculate Table.
#[test]
fn a_formula_cell_refuses_typing_and_takes_gestures_td7() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 3, 2);
    let mut s = Scenario::new("table-data-gestures", document);
    s.clean_folder();
    put(&mut s, frame, 0, 0, "4");
    open_properties(&mut s, frame);
    set_formula(&mut s, frame, 0, 1, "A1*2");
    expect_cells(&s, frame, &[(0, 1, "8.00")]);

    // Typing, Enter, Backspace, and Delete do nothing there (TNK-TD7a).
    to_cell(&mut s, frame, 0, 1);
    s.no_change("type a digit", |rt| keys(rt, "7"));
    expect_status_has(&s, "This cell holds a formula");
    for (what, action) in [
        ("Backspace", PageAction::Backspace),
        ("Delete", PageAction::Delete),
        ("Enter", PageAction::ParagraphBreak),
    ] {
        s.act("clear the status", |rt| {
            rt.app.sessions[rt.app.active_session].status.clear()
        });
        s.no_change(what, move |rt| {
            rt.send(Message::Page(action));
        });
        expect_status_has(&s, "This cell holds a formula");
    }
    // Part of the result selected: Delete does nothing either.
    let flow = flow_of(&s, frame, 0, 1);
    s.act("select part of the result", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(flow, 0, 0),
            page: 1,
            extend: false,
        }));
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(flow, 0, 2),
            page: 1,
            extend: true,
        }));
    });
    s.no_change("Delete selected characters", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    expect_cells(&s, frame, &[(0, 1, "8.00")]);

    // A double-click on the formula cell: Edit Formula.
    to_cell(&mut s, frame, 1, 0);
    s.act("close the panel", |rt| {
        rt.send(Message::TableProperties(TableAction::Close));
    });
    s.act("double-click the formula cell", move |rt| {
        let line = rt.app.pages.layout().pages[0]
            .lines
            .iter()
            .find(|line| line.flow == flow)
            .cloned()
            .expect("the formula cell's line");
        let point = rt.on_page(line.x + 3.0, line.top + line.height / 2.0);
        let sent = rt.double_click(point);
        assert!(
            sent.iter()
                .any(|message| matches!(message, Message::Page(PageAction::EditFormula))),
            "{sent:?}"
        );
    });
    if !s.app().table_properties.as_ref().is_some_and(|state| {
        state
            .open
            .contains(&crate::table_properties::Section::Formula)
    }) {
        s.fail("double-click", "the Formula section is not open");
    }

    // The box: typed, applied as one step; Esc restores.
    let undo = s.app().history.undo_len();
    s.act("type a formula", |rt| {
        rt.send(Message::TableProperties(TableAction::FormulaText(
            "A1*3".into(),
        )));
    });
    s.act("Esc", |rt| {
        rt.send(Message::Escape);
    });
    expect_formula(&s, frame, 0, 1, Some("A1*2"));
    s.act("type a formula again", |rt| {
        rt.send(Message::TableProperties(TableAction::FormulaText(
            "A1*3".into(),
        )));
        rt.send(Message::TableProperties(TableAction::FormulaSubmit));
    });
    expect_formula(&s, frame, 0, 1, Some("A1*3"));
    expect_cells(&s, frame, &[(0, 1, "12.00")]);
    if s.app().history.undo_len() != undo + 1 {
        s.fail("Enter", "the formula is not one undo step");
    }
    s.act("Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    expect_formula(&s, frame, 0, 1, Some("A1*2"));
    expect_cells(&s, frame, &[(0, 1, "8.00")]);

    // Recalculate Table: nothing to do when every result stands.
    to_cell(&mut s, frame, 0, 1);
    s.act("Recalculate Table", |rt| {
        rt.send(Message::Command(CommandId::RecalculateTable));
    });
    // Remove formula keeps nothing of the formula.
    s.edit("Remove formula", |rt| {
        rt.send(Message::TableProperties(TableAction::FormulaRemove));
    });
    expect_formula(&s, frame, 0, 1, None);
    expect_cells(&s, frame, &[(0, 1, "")]);
}

/// The Formula section, an error, a REF cell, and the headings, light and
/// dark, into `AMIPRO_SCREENSHOT_DIR` (as `window_screenshots`).
#[test]
fn table_data_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 4, 5);
    let mut s = Scenario::new("table-data-screens", document);
    s.clean_folder();
    build_invoice(&mut s, frame);
    let write = |s: &Scenario, name: &str| {
        for (dark, theme) in [
            (false, cosmic::Theme::light()),
            (true, cosmic::Theme::dark()),
        ] {
            let size = (1_280, 800);
            let rgb = render_window(s.app(), size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!(
                "td7-{name}-{}.ppm",
                if dark { "dark" } else { "light" }
            ));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    };
    // A divide by zero.
    put(&mut s, frame, 2, 1, "0");
    set_formula(&mut s, frame, 2, 3, "C3/B3");
    write(&s, "divide");
    // A refused formula.
    s.expect_status("cannot understand");
    set_formula(&mut s, frame, 3, 3, "foo(");
    write(&s, "refused");
    s.act("Esc", |rt| {
        rt.send(Message::Escape);
    });
    s.clear_expected_status();
    // REF.
    set_formula(&mut s, frame, 3, 3, "A4+1");
    write(&s, "ref");
    // The headings.
    s.act("headings on", |rt| {
        rt.send(Message::Command(CommandId::TableAddresses));
    });
    write(&s, "headings");
    // A formula cell with the caret in it: the box round it and the section
    // (the box keeps the keys, so this comes last).
    to_cell(&mut s, frame, 1, 3);
    s.act("Edit Formula…", |rt| {
        rt.send(Message::Command(CommandId::EditFormula));
    });
    write(&s, "formula");
}

/// Paste (text and rich text) and Cut in a formula cell are refused like
/// typing, Copy stays allowed, a paste of cells stays a table command, and
/// Delete or Backspace with the cell's whole text selected takes the
/// formula out as one undo step (Ami Pro's Del).
#[test]
fn paste_and_cut_are_refused_in_a_formula_cell_and_a_whole_cell_delete_removes_it_td7() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 3, 2);
    let mut s = Scenario::new("table-data-clipboard", document);
    s.clean_folder();
    put(&mut s, frame, 0, 0, "4");
    open_properties(&mut s, frame);
    set_formula(&mut s, frame, 0, 1, "A1*2");
    let flow = flow_of(&s, frame, 0, 1);
    let hint = |s: &mut Scenario| {
        expect_status_has(s, "This cell holds a formula");
        s.act("clear the status", |rt| {
            rt.app.sessions[rt.app.active_session].status.clear();
        });
    };

    to_cell(&mut s, frame, 0, 1);
    s.no_change("Paste text", |rt| {
        rt.clipboard = Some("77".into());
        rt.send(Message::Command(CommandId::Paste));
    });
    hint(&mut s);
    s.no_change("Paste rich text", |rt| {
        rt.clipboard_data = Some((b"{\\rtf1 77}".to_vec(), "text/rtf".into()));
        rt.send(Message::Command(CommandId::Paste));
        rt.clipboard_data = None;
    });
    hint(&mut s);
    s.act("select part of the result", move |rt| {
        for (byte, extend) in [(0, false), (2, true)] {
            rt.send(Message::Page(PageAction::MoveTo {
                position: at(flow, 0, byte),
                page: 1,
                extend,
            }));
        }
    });
    s.no_change("Cut", |rt| {
        rt.clipboard = None;
        rt.send(Message::Command(CommandId::Cut));
        assert!(rt.clipboard.is_none(), "Cut put the text on the clipboard");
    });
    hint(&mut s);
    s.no_change("Copy", |rt| {
        rt.send(Message::Command(CommandId::Copy));
    });
    if s.rt.clipboard.as_deref() != Some("8.") {
        s.fail("Copy", &format!("copied {:?}", s.rt.clipboard));
    }
    expect_cells(&s, frame, &[(0, 1, "8.00")]);
    expect_formula(&s, frame, 0, 1, Some("A1*2"));

    // Cells copied and pasted over a formula cell: a table command, the
    // formula arrives with the cells.
    s.act("select A1:B1", move |rt| {
        rt.send(Message::Page(PageAction::SelectCells {
            frame,
            anchor: (0, 0),
            focus: (0, 1),
        }));
    });
    s.command_act(CommandId::Copy);
    to_cell(&mut s, frame, 1, 0);
    s.command(CommandId::Paste);
    expect_formula(&s, frame, 1, 1, Some("A2*2"));

    // The whole text of the cell selected: Delete removes the formula.
    for (what, action) in [
        ("Delete", PageAction::Delete),
        ("Backspace", PageAction::Backspace),
    ] {
        let undo = s.app().history.undo_len();
        let end = text_in(&s, frame, 0, 1).len();
        s.act("select the whole result", move |rt| {
            for (byte, extend) in [(0, false), (end, true)] {
                rt.send(Message::Page(PageAction::MoveTo {
                    position: at(flow, 0, byte),
                    page: 1,
                    extend,
                }));
            }
        });
        s.edit(what, move |rt| {
            rt.send(Message::Page(action));
        });
        expect_formula(&s, frame, 0, 1, None);
        expect_cells(&s, frame, &[(0, 1, "")]);
        if s.app().history.undo_len() != undo + 1 {
            s.fail(what, "removing the formula is not one undo step");
        }
        // Undo, to try the other key on the same cell.
        s.act("Undo", |rt| {
            rt.send(Message::Command(CommandId::Undo));
        });
        expect_formula(&s, frame, 0, 1, Some("A1*2"));
    }
}

// ---- Tools › Sort… (TD.8; `UI_SPEC.md` §41, TN3) -------------------------

use crate::sort_panel::{Ask, Notice, SortAction};
use amipro_editing::sort::{SortKind, SortWord};

fn sort_notice(s: &Scenario) -> Option<Notice> {
    s.app()
        .sort_panel
        .as_ref()
        .and_then(|panel| panel.notice.clone())
}

/// Scenario 9: a range with a formula cell is refused, a selection of the
/// rows below sorts (numeric, on column B), Undo is one step, Save As SAM
/// and reopening keep the sorted rows.
#[test]
fn sort_refuses_a_formula_and_sorts_the_rows_without_one_td8() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 3, 6);
    let mut s = Scenario::new("table-data-sort-table", document);
    s.clean_folder();
    let names = ["pear", "fig", "apple", "kiwi", "date", "plum"];
    let numbers = ["5", "3", "9", "1", "7", "2"];
    for (row, (name, number)) in names.iter().zip(numbers).enumerate() {
        put(&mut s, frame, row, 0, name);
        put(&mut s, frame, row, 1, number);
    }
    open_properties(&mut s, frame);
    set_formula(&mut s, frame, 2, 2, "B3*2");
    expect_formula(&s, frame, 2, 2, Some("B3*2"));
    to_cell(&mut s, frame, 4, 0);
    s.command_act(CommandId::Sort);
    if s.app().sort_panel.is_none() {
        s.fail("Tools › Sort…", "the panel did not open");
    }
    // The caret in the table: the whole table is the scope, and the
    // formula in it stops the sort.
    s.no_change("Sort the whole table", |rt| {
        rt.send(Message::Sort(SortAction::Sort));
    });
    match sort_notice(&s) {
        Some(Notice::Refused(text)) if text.contains("formula") => {}
        other => s.fail("Sort", &format!("the panel shows {other:?}")),
    }
    // The rows below the formula's: column B by value.
    s.act("select rows 4 to 6", move |rt| {
        rt.send(Message::Page(PageAction::SelectCells {
            frame,
            anchor: (3, 0),
            focus: (5, 2),
        }));
    });
    for action in [
        SortAction::Field(0, "2".into()),
        SortAction::Kind(0, SortKind::Numeric),
    ] {
        s.act("set the key", move |rt| {
            rt.send(Message::Sort(action));
        });
    }
    s.edit("Sort rows 4 to 6", |rt| {
        rt.send(Message::Sort(SortAction::Sort));
    });
    expect_cells(
        &s,
        frame,
        &[
            (0, 0, "pear"),
            (1, 0, "fig"),
            (2, 0, "apple"),
            (3, 0, "kiwi"),
            (4, 0, "plum"),
            (5, 0, "date"),
            (3, 1, "1.00"),
            (4, 1, "2.00"),
            (5, 1, "7.00"),
        ],
    );
    expect_formula(&s, frame, 2, 2, Some("B3*2"));
    if s.app().history.undo_len() == 0 {
        s.fail("Sort", "no undo step");
    }
    // The settings are the next sort's, as Ami Pro's dialog keeps them.
    s.act("close the panel", |rt| {
        rt.send(Message::Sort(SortAction::Close));
    });
    s.command_act(CommandId::Sort);
    let panel = s.app().sort_panel.clone().expect("the panel is open");
    if panel.form.levels[0].field != "2" || panel.form.levels[0].kind != SortKind::Numeric {
        s.fail("Sort", &format!("the settings were not kept: {panel:?}"));
    }
    let path = s.save_sam("sorted-table.sam");
    let fresh = s.reopen(&path);
    let frame = *fresh.document().frame_stack().last().unwrap();
    expect_cells(
        &fresh,
        frame,
        &[(3, 0, "kiwi"), (4, 0, "plum"), (5, 0, "date")],
    );
    expect_formula(&fresh, frame, 2, 2, Some("B3*2"));
}

/// Scenario 9, text: paragraphs sort by their second word, the header
/// stays, Undo is one step, and Save As SAM keeps the order.
#[test]
fn sort_orders_selected_paragraphs_by_their_second_word_td8() {
    let document = Document::from_plain_text("Names\nx gamma 3\nx alpha 1\nx beta 2\nTotal: three");
    let mut s = Scenario::new("table-data-sort-text", document);
    s.clean_folder();
    s.select_main((1, 0), (3, 7));
    s.command_act(CommandId::Sort);
    for action in [
        SortAction::Word(0, SortWord::Second),
        SortAction::Field(0, "1".into()),
    ] {
        s.act("set the key", move |rt| {
            rt.send(Message::Sort(action));
        });
    }
    s.edit("Sort", |rt| {
        rt.send(Message::Sort(SortAction::Sort));
    });
    if s.main_text() != "Names\nx alpha 1\nx beta 2\nx gamma 3\nTotal: three" {
        s.fail("Sort", &format!("the text reads {:?}", s.main_text()));
    }
    // The sorted paragraphs stay selected, the others untouched.
    let selection = s.app().page_selection;
    let (start, end) = selection.ordered();
    if (start.paragraph, end.paragraph) != (1, 3) {
        s.fail("Sort", &format!("the selection is {selection:?}"));
    }
    // Sorted already: the panel says so and nothing changes.
    s.no_change("Sort again", |rt| {
        rt.send(Message::Sort(SortAction::Sort));
    });
    if !matches!(sort_notice(&s), Some(Notice::Info(text)) if text.contains("Already")) {
        s.fail("Sort", "the panel did not say the text is in order");
    }
    // Descending, on the numbers in the last word.
    for action in [
        SortAction::Word(0, SortWord::All),
        SortAction::Kind(0, SortKind::Numeric),
        SortAction::Descending(true),
    ] {
        s.act("set the key", move |rt| {
            rt.send(Message::Sort(action));
        });
    }
    // Numeric on All of "x alpha 1": no digit before... the digits of the
    // whole key: 1, 2, 3 descending.
    s.edit("Sort descending", |rt| {
        rt.send(Message::Sort(SortAction::Sort));
    });
    if s.main_text() != "Names\nx gamma 3\nx beta 2\nx alpha 1\nTotal: three" {
        s.fail("Sort", &format!("the text reads {:?}", s.main_text()));
    }
    let path = s.save_sam("sorted-text.sam");
    let fresh = s.reopen(&path);
    if fresh.main_text() != "Names\nx gamma 3\nx beta 2\nx alpha 1\nTotal: three" {
        s.fail("reopen", "the order was not kept");
    }
}

/// The panel's questions and its Escape, through the messages the widgets
/// send: a page break asks, Continue sorts across it, Cancel drops the
/// question; Esc closes the panel.
#[test]
fn the_sort_panel_asks_about_a_break_and_escape_closes_it_td8() {
    let document = Document::from_plain_text("c\nb\na");
    let main = document.main_flow_id();
    let document = amipro_editing::apply(
        &document,
        &amipro_editing::Command::InsertBreak {
            at: at(main, 1, 1),
            kind: amipro_document::BreakKind::Page,
            centre: false,
            layout: None,
        },
    )
    .unwrap()
    .document;
    let mut s = Scenario::new("table-data-sort-asks", document);
    s.clean_folder();
    s.select_main((0, 0), (2, 1));
    s.command_act(CommandId::Sort);
    s.no_change("Sort", |rt| {
        rt.send(Message::Sort(SortAction::Sort));
    });
    if sort_notice(&s) != Some(Notice::Ask(Ask::Break)) {
        s.fail("Sort", &format!("the panel shows {:?}", sort_notice(&s)));
    }
    s.no_change("Cancel", |rt| {
        rt.send(Message::Sort(SortAction::CancelAsk));
    });
    if sort_notice(&s).is_some() {
        s.fail("Cancel", "the question stayed");
    }
    s.no_change("Sort", |rt| {
        rt.send(Message::Sort(SortAction::Sort));
    });
    s.edit("Continue", |rt| {
        rt.send(Message::Sort(SortAction::Continue));
    });
    if !s.main_text().starts_with('a') {
        s.fail("Continue", &format!("the text reads {:?}", s.main_text()));
    }
    s.act("Esc", |rt| {
        rt.send(Message::Escape);
    });
    if s.app().sort_panel.is_some() {
        s.fail("Esc", "the panel stayed open");
    }
}

/// The widgets: a click scan over the open panel finds Sort, the radios,
/// the fields, and the close button, as in the other rail panels.
#[test]
fn the_sort_panels_widgets_take_clicks_td8() {
    let mut s = Scenario::new(
        "table-data-sort-widgets",
        Document::from_plain_text("b\na\nc"),
    );
    s.select_main((0, 0), (2, 1));
    s.command_act(CommandId::Sort);
    s.act("redraw", |rt| {
        rt.redraw();
    });
    let mut found = Vec::new();
    let mut y = 100.0;
    while y < Runtime::WINDOW.height - 40.0 {
        for x in (24..300).step_by(24) {
            for message in s.rt.click_unrun(
                cosmic::iced::mouse::Button::Left,
                cosmic::iced::Point::new(x as f32, y),
            ) {
                if let Message::Sort(action) = message {
                    let name = format!("{action:?}");
                    if !found.contains(&name) {
                        found.push(name);
                    }
                }
            }
        }
        y += 10.0;
    }
    for wanted in [
        "Sort",
        "Close",
        "Kind(0, Numeric)",
        "Word(0, Second)",
        "Descending(true)",
        "TabDelimited(false)",
    ] {
        if !found.iter().any(|name| name.starts_with(wanted)) {
            s.fail("clicks", &format!("no click sent {wanted}: {found:?}"));
        }
    }
}

/// `AMIPRO_SCREENSHOT_DIR=DIR`: the Sort panel on a table and on text, in
/// both themes.
#[test]
fn sort_panel_screenshots() {
    let Some(directory) = std::env::var_os("AMIPRO_SCREENSHOT_DIR") else {
        eprintln!("AMIPRO_SCREENSHOT_DIR is not set; skipping");
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 3, 6);
    let mut s = Scenario::new("table-data-sort-screens", document);
    s.clean_folder();
    let names = ["pear", "fig", "apple", "kiwi", "date", "plum"];
    for (row, name) in names.iter().enumerate() {
        put(&mut s, frame, row, 0, name);
    }
    to_cell(&mut s, frame, 0, 0);
    s.command_act(CommandId::Sort);
    let write = |s: &Scenario, name: &str| {
        for (dark, theme) in [
            (false, cosmic::Theme::light()),
            (true, cosmic::Theme::dark()),
        ] {
            let size = (1_280, 800);
            let rgb = render_window(s.app(), size, &theme, false);
            let mut ppm = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
            ppm.extend(rgb);
            let path = directory.join(format!(
                "td8-{name}-{}.ppm",
                if dark { "dark" } else { "light" }
            ));
            std::fs::write(&path, ppm).unwrap();
            eprintln!("wrote {}", path.display());
        }
    };
    write(&s, "table");
    open_properties(&mut s, frame);
    set_formula(&mut s, frame, 2, 2, "A1");
    to_cell(&mut s, frame, 0, 0);
    s.command_act(CommandId::Sort);
    s.act("Sort", |rt| {
        rt.send(Message::Sort(SortAction::Sort));
    });
    write(&s, "refused");
    leave_table(&mut s);
    s.select_main((0, 0), (0, 3));
    write(&s, "text");
}

// ---- TD.9: numbers merged from a table data file (scenario 10) -------------

use super::merge::{go_to_tab, insert_field, lines, make_data_file, panel, step};
use crate::merge_panel::MergeAction as A;

/// A letter merged from a table data file whose cells are numbers: Ami Pro's
/// own text rule (AMT3, TDV7) applies to every value, `75.50` merges as
/// `75.5`, `$1,200.00` as `$1,200`, `150` as `150`; the data file's cells
/// are not changed by it, and a delimited file merges as typed (see the
/// merge scenarios).
#[test]
fn a_table_data_files_numbers_merge_as_ami_pro_merged_them_td9() {
    let mut s = Scenario::new_document("table-data-merge");
    s.clean_folder();
    s.type_text("Dear ");
    let (_, letter) = make_data_file(
        &mut s,
        "AMD1.SAM",
        "Title, Last, City, Amount",
        &[
            &["Mr.", "Allen", "Boston", "150"],
            &["Ms.", "Beck", "Seattle", "75.50"],
            &["Dr.", "Black", "", "$1,200.00"],
            &["Mrs.", "Cole", "Reno", "007"],
        ],
    );
    let stored: Vec<String> = s
        .app()
        .data_form
        .as_ref()
        .unwrap()
        .table
        .records
        .iter()
        .map(|record| record[3].clone())
        .collect();
    // Typed in the table, a number is rewritten as the style's decimals;
    // text stays as typed.
    assert_eq!(stored[1], "75.50", "{stored:?}");
    assert_eq!(stored[2], "$1,200.00", "{stored:?}");
    go_to_tab(&mut s, letter);
    s.caret_to_end();
    insert_field(&mut s, "Title");
    s.type_text(" ");
    insert_field(&mut s, "Last");
    s.type_text(": you owe ");
    insert_field(&mut s, "Amount");
    s.type_text(" in ");
    insert_field(&mut s, "City");
    s.type_text(".");
    // The preview shows the record as it merges.
    let mut previews = Vec::new();
    step(&mut s, "Preview a record", |rt| {
        rt.send(Message::Merge(A::Preview(true)));
    });
    previews.push(lines(&s)[0].clone());
    for _ in 0..3 {
        step(&mut s, "Next record", |rt| {
            rt.send(Message::Merge(A::Next));
        });
        previews.push(lines(&s)[0].clone());
    }
    assert_eq!(
        previews,
        [
            "Dear Mr. Allen: you owe 150 in Boston.",
            "Dear Ms. Beck: you owe 75.5 in Seattle.",
            "Dear Dr. Black: you owe $1,200 in .",
            "Dear Mrs. Cole: you owe 007 in Reno.",
        ]
    );
    step(&mut s, "Preview off", |rt| {
        rt.send(Message::Merge(A::Preview(false)));
    });
    assert_eq!(
        panel(&s).data.as_ref().unwrap().table.records[1][3],
        "75.50"
    );
    let tabs = s.app().sessions.len();
    s.allow_empty_fields = true;
    s.act("Merge to New Document", |rt| {
        rt.send(Message::Merge(A::ToNewDocument));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1);
    let merged = s.main_text();
    for want in [
        "you owe 150 in Boston.",
        "you owe 75.5 in Seattle.",
        "you owe $1,200 in .",
        "you owe 007 in Reno.",
    ] {
        assert!(merged.contains(want), "{want:?} in {merged:?}");
    }
    assert!(!merged.contains("75.50") && !merged.contains("1,200.00"));
    s.save_sam("MERGED.SAM");
}

// ---- TD.10: the form recipe (scenario 11) ----------------------------------

use amipro_document::FrameContent;

fn form_table(s: &Scenario, frame: FrameId) -> amipro_document::Table {
    match &s.document().frame(frame).unwrap().content {
        FrameContent::Table(table) => table.clone(),
        _ => unreachable!(),
    }
}

fn caret_at(s: &Scenario) -> Option<(usize, usize)> {
    crate::table_editing::caret_cell(s.document(), s.app().page_selection)
        .map(|(_, row, column)| (row, column))
}

/// The text of a flow's spans that carry Protected Text, over the flow's
/// style.
fn protected_text(s: &Scenario, flow: FlowId) -> String {
    let document = s.document();
    let paragraph = &document.flow(flow).unwrap().paragraphs[0];
    crate::page_editing::protected_stretches(document, flow, 0)
        .into_iter()
        .map(|range| paragraph.text[range].to_string())
        .collect()
}

/// The recipe of the guide's chapter 16 (TN7: no new feature): a table with
/// labels and blanks; the labels' cells protected, Honor protection on, the
/// caret skipping them; Protected Text on the labels; printed on a
/// preprinted form without the labels and the lines; saved as a style sheet
/// with its contents and made again by File › New with contents.
#[test]
fn the_form_recipe_protects_labels_and_prints_on_a_preprinted_form_td10() {
    let (document, frame) = numbers_table(NumberKind::General, 0, 2, 3);
    let mut s = Scenario::new("table-data-form", document);
    let folder = s.clean_folder();
    let labels = ["Name", "Amount", "Date"];
    for (row, label) in labels.iter().enumerate() {
        put(&mut s, frame, row, 0, label);
    }
    leave_table(&mut s);
    assert_eq!(text_in(&s, frame, 2, 0), "Date");

    // Lines round every cell, as a form has them.
    to_cell(&mut s, frame, 0, 0);
    s.command_act(CommandId::TableStyleGrid);
    // Protect the labels' cells: select the column's three cells.
    for _ in 0..2 {
        s.act("select the next cell down", |rt| {
            rt.send(Message::Page(PageAction::CellMove {
                down: 1,
                right: 0,
                extend: true,
            }));
        });
    }
    s.command_act(CommandId::ProtectCells);
    let table = form_table(&s, frame);
    for row in 0..3 {
        assert!(table.cells[row * 2].format.protected, "label {row}");
        assert!(!table.cells[row * 2 + 1].format.protected, "blank {row}");
    }

    // Protected Text on each label (before Honor protection keeps the
    // caret out of its cell).
    for (row, label) in labels.iter().enumerate() {
        let flow = flow_of(&s, frame, row, 0);
        let end = paragraph_of(&s, flow).text.len();
        let page = page_of(&s, frame, row);
        s.act(&format!("select label {row}"), move |rt| {
            rt.send(Message::Page(PageAction::MoveTo {
                position: at(flow, 0, 0),
                page,
                extend: false,
            }));
            rt.send(Message::Page(PageAction::MoveTo {
                position: at(flow, 0, end),
                page,
                extend: true,
            }));
        });
        s.command_act(CommandId::ProtectedText);
        assert_eq!(protected_text(&s, flow), *label, "label {row}");
    }

    // Honor protection: Table Properties, one click.
    open_properties(&mut s, frame);
    s.act("Honor protection on", |rt| {
        rt.send(Message::TableProperties(TableAction::HonorProtection(true)));
    });
    assert!(form_table(&s, frame).layout.honor_protection);
    leave_table(&mut s);

    // The caret skips the protected cells: Tab from the first blank goes to
    // the next blank, past the label that starts the row.
    to_cell(&mut s, frame, 0, 1);
    assert_eq!(caret_at(&s), Some((0, 1)));
    for want in [(1, 1), (2, 1)] {
        s.act("Tab", |rt| {
            rt.send(Message::Page(PageAction::CellStep { forward: true }));
        });
        assert_eq!(caret_at(&s), Some(want), "Tab skips the labels");
    }
    s.act("Shift+Tab", |rt| {
        rt.send(Message::Page(PageAction::CellStep { forward: false }));
    });
    assert_eq!(caret_at(&s), Some((1, 1)), "Shift+Tab skips them too");
    // A blank takes typing.
    put(&mut s, frame, 1, 1, "42");
    leave_table(&mut s);
    assert_eq!(text_in(&s, frame, 1, 1), "42.00");
    assert_eq!(text_in(&s, frame, 1, 0), "Amount", "labels unchanged");

    // Printed on a preprinted form: no labels, no lines; the blanks' text
    // still prints.
    let layout = {
        let mut pages = crate::page_layout::SessionPages::default();
        pages.update(s.document());
        pages.layout().clone()
    };
    let paper = crate::print::drawn_page(
        s.document(),
        &layout,
        Some(crate::page_surface::PrintMode::default()),
        0,
    );
    let form = crate::print::drawn_page(
        s.document(),
        &layout,
        Some(crate::page_surface::PrintMode {
            preprinted_form: true,
            ..Default::default()
        }),
        0,
    );
    let black = |colours: &[cosmic::iced::Color]| {
        colours
            .iter()
            .filter(|colour| **colour == cosmic::iced::Color::BLACK)
            .count()
    };
    assert!(
        black(&paper.0) > 0,
        "the table's lines print on paper: {:?}",
        (paper.0.len(), form.0.len(), &paper.0)
    );
    assert!(
        black(&form.0) < black(&paper.0),
        "no lines on a preprinted form: {} of {}",
        black(&form.0),
        black(&paper.0)
    );
    let has = |texts: &[String], word: &str| texts.iter().any(|text| text.contains(word));
    for label in labels {
        assert!(has(&paper.1, label), "{label} prints on paper");
        assert!(!has(&form.1, label), "{label} is left out of the form");
    }
    assert!(has(&form.1, "42"), "the filled blank prints: {:?}", form.1);
    // The PDF with the same option is made.
    let options = crate::print::PrintOptions {
        preprinted_form: true,
        ..Default::default()
    }
    .pdf_options(s.document(), None, None);
    assert!(
        crate::print::document_pdf(s.document(), &layout, None, &options).is_ok(),
        "the preprinted-form PDF"
    );

    // Save as a style sheet, With contents: the dialog offers it (the file
    // chooser itself is never opened), and the sheet is written as the
    // action writes it.
    s.command_act(CommandId::SaveAsStyleSheet);
    assert!(s.app().save_sheet_contents, "With contents is on");
    s.act("Cancel", |rt| {
        rt.send(Message::SaveSheet(crate::SaveSheetAction::Cancel));
    });
    let sheets = folder.join("styles");
    std::fs::create_dir_all(&sheets).unwrap();
    let bytes = amipro_formats::style_sheet(
        s.document(),
        &amipro_formats::SheetOptions {
            description: "Order form".into(),
            saved_at: 0,
            contents: true,
        },
    );
    std::fs::write(sheets.join("FORM.STY"), bytes).unwrap();

    // File › New with contents makes the form again, labels protected.
    s.command_act(CommandId::Preferences);
    let chosen = sheets.clone();
    s.act("add the style-sheet folder", move |rt| {
        rt.send(Message::Preferences(
            crate::preferences_dialog::PreferencesAction::FolderChosen(Some(chosen)),
        ));
        rt.send(Message::Preferences(
            crate::preferences_dialog::PreferencesAction::Close,
        ));
    });
    s.command_act(CommandId::NewFromStyleSheet);
    let index = s
        .app()
        .new_document_dialog
        .as_ref()
        .and_then(|dialog| {
            dialog
                .sheets
                .iter()
                .position(|sheet| sheet.name == "FORM.STY")
        })
        .unwrap_or_else(|| s.fail("File > New", "FORM.STY is not listed"));
    s.act("select the form and Create", move |rt| {
        rt.send(Message::NewDocumentDialog(
            crate::new_document_dialog::NewAction::Select(index),
        ));
        rt.send(Message::NewDocumentDialog(
            crate::new_document_dialog::NewAction::Create,
        ));
    });
    let made = s
        .document()
        .frame_stack()
        .iter()
        .find(|id| {
            matches!(
                s.document().frame(**id).unwrap().content,
                FrameContent::Table(_)
            )
        })
        .copied()
        .unwrap_or_else(|| s.fail("File > New", "the new form has no table"));
    let table = form_table(&s, made);
    assert!(table.layout.honor_protection, "Honor protection kept");
    assert!(table.cells[0].format.protected && table.cells[4].format.protected);
    assert!(!table.cells[1].format.protected);
    for (row, label) in labels.iter().enumerate() {
        let flow = flow_of(&s, made, row, 0);
        assert_eq!(paragraph_of(&s, flow).text, *label);
        assert_eq!(protected_text(&s, flow), *label, "Protected Text kept");
    }
    // Saved as SAM and reopened: the same.
    let path = s.save_sam("FORM.SAM");
    let reopened = s.reopen(&path);
    assert!(reopened.document().frames().any(
        |frame| matches!(&frame.content, FrameContent::Table(t) if t.layout.honor_protection)
    ));
}

// ---- TD.11: the native-only functions (TN4) --------------------------------

/// `@avg @min @max @count` typed in the Formula box calculate; the Formula
/// section's hint lists them as extras; Save As SAM lists the formulas that
/// use them (the sum is not listed), and the saved file reads back with the
/// formula text and result.
#[test]
fn the_extra_functions_calculate_and_save_as_lists_them_td11() {
    let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 3, 4);
    let mut s = Scenario::new("table-data-extras", document);
    s.clean_folder();
    for (row, value) in ["2", "4", "9"].iter().enumerate() {
        put(&mut s, frame, row, 0, value);
    }
    open_properties(&mut s, frame);
    assert!(
        crate::table_properties::FORMULA_HINT.contains("@avg @min @max @count")
            && crate::table_properties::FORMULA_HINT.contains("Extras"),
        "the hint lists the extras as extras"
    );
    set_formula(&mut s, frame, 3, 0, "@avg(A1..A3)");
    set_formula(&mut s, frame, 3, 1, "@max(A1..A3)");
    set_formula(&mut s, frame, 0, 2, "@min(A1..A3)");
    set_formula(&mut s, frame, 1, 2, "@count(A1..A3)");
    set_formula(&mut s, frame, 2, 2, "@sum(A1..A3)");
    leave_table(&mut s);
    expect_cells(
        &s,
        frame,
        &[
            (3, 0, "5.00"),
            (3, 1, "9.00"),
            (0, 2, "2.00"),
            (1, 2, "3.00"),
            (2, 2, "15.00"),
        ],
    );
    // A changed number reaches them all in the same step.
    put(&mut s, frame, 1, 0, "20");
    leave_table(&mut s);
    expect_cells(
        &s,
        frame,
        &[(3, 0, "10.33"), (3, 1, "20.00"), (2, 2, "31.00")],
    );
    // Save As SAM: the four extras' formulas are listed once, together; the
    // file keeps their text and results.
    let path = s.save_sam_expecting(
        "EXTRAS.SAM",
        &["4 formulas with a function Ami Pro lacks (@avg, @min, @max, @count)"],
    );
    let reopened = s.reopen(&path);
    let other = reopened
        .document()
        .frame_stack()
        .iter()
        .copied()
        .find(|id| {
            matches!(
                reopened.document().frame(*id).unwrap().content,
                FrameContent::Table(_)
            )
        })
        .unwrap();
    let kept = formula_in_document(reopened.document(), other, 3, 0);
    assert_eq!(kept.as_deref(), Some("@avg(A1..A3)"));
}

fn formula_in_document(
    document: &Document,
    frame: FrameId,
    row: usize,
    column: usize,
) -> Option<String> {
    match &document.frame(frame).unwrap().content {
        FrameContent::Table(table) => table.cells[row * table.columns.len() + column]
            .formula
            .clone(),
        _ => None,
    }
}

// ---- TD.12: the twins for Ami Pro in the VM ------------------------------

/// The scenario's saved file copied into `AMIPRO_TWIN_OUT` under `name`.
fn twin_out(path: &Path, name: &str) {
    let out =
        std::path::PathBuf::from(std::env::var_os("AMIPRO_TWIN_OUT").expect("AMIPRO_TWIN_OUT"));
    std::fs::create_dir_all(&out).unwrap();
    std::fs::copy(path, out.join(name)).unwrap();
    eprintln!("wrote {}", out.join(name).display());
}

/// The table twins ATD1..ATD5, TW1..TW5 of TD.12, written by Save As SAM
/// through the app's own commands: `AMIPRO_TWIN_OUT=DIR`.
#[test]
#[ignore = "writes the table data twins for the VM"]
fn table_data_twins() {
    if std::env::var_os("AMIPRO_TWIN_OUT").is_none() {
        eprintln!("AMIPRO_TWIN_OUT is not set; skipping");
        return;
    }
    // TW1: an invoice (product per row, @sum total) with a Quick Add column
    // and row.
    {
        let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 5, 5);
        let mut s = Scenario::new("table-data-twin1", document);
        s.clean_folder();
        build_invoice(&mut s, frame);
        put(&mut s, frame, 0, 4, "Sum");
        for row in 1..=3 {
            to_cell(&mut s, frame, row, 4);
            s.command(CommandId::QuickAddRow);
        }
        for column in 1..=2 {
            to_cell(&mut s, frame, 4, column);
            s.command(CommandId::QuickAddColumn);
        }
        leave_table(&mut s);
        expect_formula(&s, frame, 1, 4, Some("@sum(B2..D2)"));
        expect_formula(&s, frame, 4, 1, Some("@sum(B2..B4)"));
        let path = s.save_sam("ATD1.SAM");
        twin_out(&path, "ATD1.SAM");
        eprintln!("TW1 cells: {:?}", snapshot(s.document()));
    }
    // TW2: number formats, one file each (the format is the Table Text
    // style's). A number typed and rewritten in each; text stays.
    for (name, kind, flags) in [
        ("ATD2G.SAM", NumberKind::General, 0x118),
        ("ATD2F.SAM", NumberKind::Fixed, 0x18),
        ("ATD2C.SAM", NumberKind::Currency, 0x198),
        ("ATD2N.SAM", NumberKind::Currency, 0x148),
        ("ATD2P.SAM", NumberKind::Percent, 0x10),
    ] {
        let (document, frame) = numbers_table(kind, flags, 2, 4);
        let mut s = Scenario::new("table-data-twin2", document);
        s.clean_folder();
        for (row, column, text) in [
            (0, 0, "1234.5"),
            (0, 1, "-7.256"),
            (1, 0, "0.5"),
            (1, 1, "007"),
            (2, 0, "abc"),
            (2, 1, "12"),
            (3, 0, "-1500"),
        ] {
            put(&mut s, frame, row, column, text);
        }
        leave_table(&mut s);
        let path = s.save_sam(name);
        twin_out(&path, name);
        eprintln!("{name} cells: {:?}", snapshot(s.document()));
    }
    // TW3: a sorted table (rows by column B, numeric) and a sorted
    // paragraph list (second word), one document.
    {
        let document = Document::from_plain_text("List\nx gamma 3\nx alpha 1\nx beta 2\nPrices");
        let start = at(document.main_flow_id(), 4, 6);
        let mut document = amipro_editing::apply(
            &document,
            &amipro_editing::Command::InsertTable {
                target: amipro_editing::tables::TableTarget::Text(start),
                table: amipro_editing::tables::NewTable::fitting(3, 5, 9_360, 284),
            },
        )
        .unwrap()
        .document;
        let frame = *document.frame_stack().last().unwrap();
        let flow = cell(&document, frame, 0, 0);
        let style = document.flow(flow).unwrap().paragraphs[0].style;
        let format = &mut document
            .styles_mut()
            .get_mut(&style)
            .unwrap()
            .kept
            .number_format;
        format.kind = NumberKind::Fixed;
        format.flags = 0x18;
        format.decimals = 2;
        let mut s = Scenario::new("table-data-twin3", document);
        s.clean_folder();
        for (row, (name, number)) in ["pear", "fig", "apple", "kiwi", "date"]
            .iter()
            .zip(["5", "3", "9", "1", "7"])
            .enumerate()
        {
            put(&mut s, frame, row, 0, name);
            put(&mut s, frame, row, 1, number);
            put(&mut s, frame, row, 2, name);
        }
        s.act("select the rows", move |rt| {
            rt.send(Message::Page(PageAction::SelectCells {
                frame,
                anchor: (0, 0),
                focus: (4, 2),
            }));
        });
        s.command_act(CommandId::Sort);
        for action in [
            SortAction::Field(0, "2".into()),
            SortAction::Kind(0, SortKind::Numeric),
        ] {
            s.act("set the key", move |rt| {
                rt.send(Message::Sort(action));
            });
        }
        s.edit("Sort the table", |rt| {
            rt.send(Message::Sort(SortAction::Sort));
        });
        s.act("close the panel", |rt| {
            rt.send(Message::Sort(SortAction::Close));
        });
        s.select_main((1, 0), (3, 7));
        s.command_act(CommandId::Sort);
        for action in [
            SortAction::Word(0, SortWord::Second),
            SortAction::Field(0, "1".into()),
        ] {
            s.act("set the key", move |rt| {
                rt.send(Message::Sort(action));
            });
        }
        s.edit("Sort the paragraphs", |rt| {
            rt.send(Message::Sort(SortAction::Sort));
        });
        s.act("close the panel", |rt| {
            rt.send(Message::Sort(SortAction::Close));
        });
        let path = s.save_sam("ATD3.SAM");
        twin_out(&path, "ATD3.SAM");
        eprintln!(
            "TW3 text {:?}; cells: {:?}",
            s.main_text(),
            snapshot(s.document())
        );
    }
    // TW4: divide by zero, a loop, REF.
    {
        let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 4, 4);
        let mut s = Scenario::new("table-data-twin4", document);
        s.clean_folder();
        put(&mut s, frame, 0, 1, "8");
        put(&mut s, frame, 0, 2, "0");
        put(&mut s, frame, 1, 0, "abc");
        put(&mut s, frame, 3, 3, "6");
        open_properties(&mut s, frame);
        set_formula(&mut s, frame, 0, 3, "B1/C1");
        set_formula(&mut s, frame, 1, 3, "@sum(B1..C1)");
        set_formula(&mut s, frame, 2, 0, "A4");
        set_formula(&mut s, frame, 3, 0, "A3");
        set_formula(&mut s, frame, 1, 2, "A2+1");
        set_formula(&mut s, frame, 2, 3, "D4*2");
        leave_table(&mut s);
        let path = s.save_sam("ATD4.SAM");
        twin_out(&path, "ATD4.SAM");
        eprintln!("TW4 cells: {:?}", snapshot(s.document()));
    }
    // TW5: @avg and the other extras, saved with the loss list.
    {
        let (document, frame) = numbers_table(NumberKind::Fixed, 0x18, 3, 4);
        let mut s = Scenario::new("table-data-twin5", document);
        s.clean_folder();
        for (row, value) in ["2", "4", "9"].iter().enumerate() {
            put(&mut s, frame, row, 0, value);
        }
        open_properties(&mut s, frame);
        set_formula(&mut s, frame, 3, 0, "@avg(A1..A3)");
        set_formula(&mut s, frame, 2, 2, "@sum(A1..A3)");
        leave_table(&mut s);
        let path = s.save_sam_expecting(
            "ATD5.SAM",
            &["1 formula with a function Ami Pro lacks (@avg, @min, @max, @count)"],
        );
        twin_out(&path, "ATD5.SAM");
        eprintln!("TW5 cells: {:?}", snapshot(s.document()));
    }
}
