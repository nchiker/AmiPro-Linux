//! Tables: inserted, typed in, Tab rows, joined, sorted, rows and columns, properties, deleted.
//! (`UI_SPEC.md` §32, §26, §24.)

use super::*;
use crate::table_commands::InsertTableAction;
use crate::table_properties::{Field, Side, TableAction};

/// The table made last.
fn table_frame(s: &Scenario) -> FrameId {
    *s.document()
        .frame_stack()
        .iter()
        .rev()
        .find(|id| {
            matches!(
                s.document().frame(**id).unwrap().content,
                FrameContent::Table(_)
            )
        })
        .unwrap_or_else(|| s.fail("table", "the document has no table"))
}

fn table_of(s: &Scenario) -> amipro_document::Table {
    match &s.document().frame(table_frame(s)).unwrap().content {
        FrameContent::Table(table) => table.clone(),
        _ => unreachable!(),
    }
}

fn text_of(s: &Scenario, row: usize, column: usize) -> String {
    let frame = table_frame(s);
    flow_text(s.app(), cell(s.document(), frame, row, column))
}

/// The table's cell texts, a vector per row.
fn grid(s: &Scenario) -> Vec<Vec<String>> {
    let table = table_of(s);
    let columns = table.columns.len();
    (0..table.rows.len())
        .map(|row| (0..columns).map(|column| text_of(s, row, column)).collect())
        .collect()
}

fn texts(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows.iter()
        .map(|row| row.iter().map(|text| (*text).to_owned()).collect())
        .collect()
}

/// Puts the caret at the start of a cell's text (a click in it).
fn go_to_cell(s: &mut Scenario, row: usize, column: usize) {
    let frame = table_frame(s);
    let flow = cell(s.document(), frame, row, column);
    let page = page_of_row(s, frame, row);
    move_to(s, flow, 0, 0, page);
}

/// A caret move that may rewrite the number in the cell it leaves (TD.5).
fn move_to(s: &mut Scenario, flow: FlowId, paragraph: usize, byte: usize, page: u32) {
    s.act("caret move", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(flow, paragraph, byte),
            page,
            extend: false,
        }));
    });
}

/// The caret at the end of a cell's text.
fn go_to_cell_end(s: &mut Scenario, row: usize, column: usize) {
    let frame = table_frame(s);
    let flow = cell(s.document(), frame, row, column);
    let last = s.document().flow(flow).unwrap().paragraphs.len() - 1;
    let byte = s.document().flow(flow).unwrap().paragraphs[last].text.len();
    let page = page_of_row(s, frame, row);
    move_to(s, flow, last, byte, page);
}

/// The page a table's row is on (its first part holding it).
fn page_of_row(s: &Scenario, frame: FrameId, row: usize) -> u32 {
    table_rows_by_page(s.app(), frame)
        .into_iter()
        .find(|(_, rows)| rows.contains(&row))
        .map_or(1, |(page, _)| page)
}

fn expect_caret(s: &Scenario, row: usize, column: usize) {
    if caret_cell(s.app()) != Some((row, column)) {
        s.fail(
            "caret",
            &format!(
                "the caret is in {:?}, expected cell ({row}, {column})",
                caret_cell(s.app())
            ),
        );
    }
}

/// Tab / Shift+Tab as the page surface sends them.
fn tab(s: &mut Scenario, forward: bool) {
    // Leaving a cell with a number rewrites it (TD.5), so Tab is no
    // `no_change` step.
    s.act(if forward { "Tab" } else { "Shift+Tab" }, move |rt| {
        rt.send(Message::Page(PageAction::CellStep { forward }));
    });
}

/// Insert › Table… with `columns` × `rows` picked or typed in the dialog.
fn insert_table_dialog(s: &mut Scenario, columns: usize, rows: usize) {
    s.command_act(CommandId::Table);
    if s.app().insert_table_dialog.is_none() {
        s.fail("Insert Table", "Insert › Table… did not open its dialog");
    }
    s.act("type the columns and rows", move |rt| {
        rt.send(Message::InsertTable(InsertTableAction::Columns(
            columns.to_string(),
        )));
        rt.send(Message::InsertTable(InsertTableAction::Rows(
            rows.to_string(),
        )));
    });
    // (Undo and Redo of the insertion are checked by their own scenario.)
    s.act("Insert the table", |rt| {
        rt.send(Message::InsertTable(InsertTableAction::Insert));
    });
    if s.app().insert_table_dialog.is_some() {
        s.fail("Insert Table", "the dialog stayed open after Insert");
    }
    let table = table_of(s);
    if (table.columns.len(), table.rows.len()) != (columns, rows) {
        s.fail(
            "Insert Table",
            &format!(
                "inserted {} × {}, wanted {columns} × {rows}",
                table.columns.len(),
                table.rows.len()
            ),
        );
    }
    expect_caret(s, 0, 0);
}

/// The first page's table cell rectangles, from the layout.
fn cell_rectangles(s: &mut Scenario, frame: FrameId) -> Vec<amipro_layout::Bounds> {
    s.rt.redraw();
    let session = &s.rt.app.sessions[s.rt.app.active_session];
    session
        .pages
        .layout()
        .pages
        .iter()
        .flat_map(|page| page.frames.iter())
        .find_map(|frame_box| match &frame_box.kind {
            amipro_layout::FrameBoxKind::Table { cells, .. } if frame_box.frame == frame => {
                Some(cells.clone())
            }
            _ => None,
        })
        .unwrap_or_else(|| s.fail("layout", "the table has no frame box"))
}

#[test]
fn a_table_from_the_dialog_filled_with_tab_and_typed_around() {
    let mut s = Scenario::new_document("table-fill");
    s.clean_folder();

    s.type_loose("Quarterly figures");
    s.enter_loose();
    s.type_loose("Totals follow below.");
    // The caret at the end of the first line, then Insert › Table….
    let main = s.main_flow();
    s.caret(main, 0, "Quarterly figures".len(), 1, false);
    insert_table_dialog(&mut s, 3, 2);

    // Headings by Tab, then a new row from the last cell by Tab.
    s.type_loose("Region");
    tab(&mut s, true);
    expect_caret(&s, 0, 1);
    s.type_loose("Q1");
    tab(&mut s, true);
    s.type_loose("Q2");
    tab(&mut s, true);
    expect_caret(&s, 1, 0);
    s.type_loose("North");
    tab(&mut s, true);
    s.type_loose("10");
    tab(&mut s, true);
    s.type_loose("12");
    // Tab in the last cell adds a row, as one undo step.
    s.edit_loose("Tab in the last cell adds a row", |rt| {
        rt.send(Message::Page(PageAction::CellStep { forward: true }));
    });
    expect_caret(&s, 2, 0);
    if table_of(&s).rows.len() != 3 {
        s.fail("Tab", "the table did not get a third row");
    }
    s.type_loose("South");
    tab(&mut s, true);
    s.type_loose("7");
    tab(&mut s, true);
    s.type_loose("9");
    assert_eq!(
        grid(&s),
        texts(&[
            &["Region", "Q1", "Q2"],
            &["North", "10.00", "12.00"],
            &["South", "7.00", "9"]
        ])
    );

    // Shift+Tab walks back; Ctrl+arrows move by cells.
    tab(&mut s, false);
    expect_caret(&s, 2, 1);
    s.no_change("Ctrl+Up", |rt| {
        rt.send(Message::Page(PageAction::CellMove {
            down: -1,
            right: 0,
            extend: false,
        }));
    });
    expect_caret(&s, 1, 1);
    s.no_change("Ctrl+Left", |rt| {
        rt.send(Message::Page(PageAction::CellMove {
            down: 0,
            right: -1,
            extend: false,
        }));
    });
    expect_caret(&s, 1, 0);
    s.no_change("Ctrl+Down", |rt| {
        rt.send(Message::Page(PageAction::CellMove {
            down: 1,
            right: 0,
            extend: false,
        }));
    });
    expect_caret(&s, 2, 0);

    // Type right before and right after what a cell holds, then delete it.
    go_to_cell(&mut s, 1, 1);
    s.type_loose("~");
    go_to_cell_end(&mut s, 1, 1);
    s.type_loose("%");
    if text_of(&s, 1, 1) != "~10.00%" {
        s.fail("typing", &format!("the cell holds {:?}", text_of(&s, 1, 1)));
    }
    s.edit_loose("Backspace", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    go_to_cell(&mut s, 1, 1);
    s.edit_loose("Delete", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    assert_eq!(text_of(&s, 1, 1), "10.00");

    // Backspace at the start of a cell and Delete at the end of one change
    // nothing and never join the neighbours.
    go_to_cell(&mut s, 1, 2);
    s.no_change("Backspace at the start of a cell", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    go_to_cell_end(&mut s, 1, 1);
    s.no_change("Delete at the end of a cell", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    expect_caret(&s, 1, 1);

    // Enter in a cell makes a second line in it.
    go_to_cell_end(&mut s, 2, 0);
    s.enter_loose();
    s.type_loose("Africa");
    if text_of(&s, 2, 0) != "South\nAfrica" {
        s.fail("Enter", &format!("the cell holds {:?}", text_of(&s, 2, 0)));
    }
    let frame = table_frame(&s);
    let flow = cell(s.document(), frame, 2, 0);
    s.caret(flow, 0, 5, 1, false);
    s.caret(flow, 1, 6, 1, true);
    s.edit_loose("Backspace over the selected new line", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    assert_eq!(text_of(&s, 2, 0), "South");

    // Escape leaves the cell for the main text; typing there goes after.
    s.no_change("Escape", |rt| {
        rt.send(Message::Page(PageAction::Escape));
    });
    s.caret_to_end();
    s.type_loose(" Thank you.");
    if !s.main_text().contains("Totals follow below. Thank you.") {
        s.fail("typing", &format!("the main text is {:?}", s.main_text()));
    }

    // A click in a cell puts the caret there (TK6), in the window.
    let frame = table_frame(&s);
    let cells = cell_rectangles(&mut s, frame);
    let third = cells[2];
    let point = s.rt.on_page(third.x + 150.0, third.y + third.height / 2.0);
    s.act("click in the Q2 heading", move |rt| {
        rt.click(cosmic::iced::mouse::Button::Left, point);
    });
    expect_caret(&s, 0, 2);
    s.type_loose("!");
    assert!(text_of(&s, 0, 2).contains('!'), "{}", text_of(&s, 0, 2));
    s.edit_loose("Backspace the !", |rt| {
        rt.send(Message::Page(PageAction::Backspace));
    });
    assert_eq!(text_of(&s, 0, 2), "Q2");

    // Select all text of a cell; Delete empties it and keeps the row.
    let flow = cell(s.document(), frame, 2, 1);
    s.caret(flow, 0, 0, 1, false);
    s.caret(flow, 0, 1, 1, true);
    s.edit_loose("Delete the selected 7", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });
    s.type_loose("8");

    // Undo all the way back to the empty document, then Redo it all.
    let made = grid(&s);
    let steps = s.app().history.undo_len();
    s.act("Undo everything", move |rt| {
        for _ in 0..steps {
            rt.send(Message::Command(CommandId::Undo));
        }
    });
    if s.main_text() != "" || s.app().history.can_undo() {
        s.fail(
            "Undo everything",
            &format!("text left: {:?}", s.main_text()),
        );
    }
    s.act("Redo everything", move |rt| {
        for _ in 0..steps {
            rt.send(Message::Command(CommandId::Redo));
        }
    });
    assert_eq!(grid(&s), made);

    let path = s.save_sam("TABFILL.SAM");
    let s = s.reopen(&path);
    assert_eq!(
        grid(&s),
        texts(&[
            &["Region", "Q1", "Q2"],
            &["North", "10.00", "12.00"],
            &["South", "8.00", "9.00"]
        ])
    );
}

#[test]
fn rows_and_columns_joined_cells_headings_sorted_and_deleted() {
    let mut s = Scenario::new_document("table-structure");
    s.clean_folder();

    s.type_loose("Fruit list");
    insert_table_dialog(&mut s, 2, 3);
    for (row, (name, count)) in [("pear", "3"), ("apple", "12"), ("fig", "7")]
        .into_iter()
        .enumerate()
    {
        go_to_cell(&mut s, row, 0);
        s.type_loose(name);
        go_to_cell(&mut s, row, 1);
        s.type_loose(count);
    }

    // Rows and columns.
    go_to_cell(&mut s, 1, 0);
    s.command_loose(CommandId::InsertRowAbove);
    assert_eq!(table_of(&s).rows.len(), 4);
    assert_eq!(text_of(&s, 2, 0), "apple");
    s.command_loose(CommandId::InsertRowBelow);
    assert_eq!(table_of(&s).rows.len(), 5);
    assert_eq!(text_of(&s, 2, 0), "apple");
    assert_eq!(text_of(&s, 4, 0), "fig");
    s.command_loose(CommandId::InsertColumnRight);
    assert_eq!(table_of(&s).columns.len(), 3);
    s.command_loose(CommandId::InsertColumnLeft);
    assert_eq!(table_of(&s).columns.len(), 4);
    // The columns are now: new, fruit, new, count.
    assert_eq!(text_of(&s, 0, 1), "pear");
    assert_eq!(text_of(&s, 0, 3), "3.00");
    // The new cells take typing.
    go_to_cell(&mut s, 1, 0);
    s.type_loose("first");
    go_to_cell(&mut s, 1, 1);
    s.type_loose("red");
    // Delete Column (the empty one) and Delete Row (the empty one).
    go_to_cell(&mut s, 0, 2);
    s.command_loose(CommandId::DeleteColumn);
    assert_eq!(table_of(&s).columns.len(), 3);
    go_to_cell(&mut s, 3, 1);
    s.command_loose(CommandId::DeleteRow);
    assert_eq!(table_of(&s).rows.len(), 4);
    assert_eq!(
        grid(&s),
        texts(&[
            &["", "pear", "3.00"],
            &["first", "red", ""],
            &["", "apple", "12.00"],
            &["", "fig", "7.00"]
        ])
    );

    // Connect two cells, then Disconnect.
    go_to_cell(&mut s, 0, 1);
    s.act("select two cells", |rt| {
        rt.send(Message::Page(PageAction::CellMove {
            down: 0,
            right: 1,
            extend: true,
        }));
    });
    s.command_loose(CommandId::ConnectCells);
    go_to_cell(&mut s, 0, 1);
    if s.app().table_menu_label(CommandId::ConnectCells) != Some("Disconnect Cells") {
        s.fail("Connect", "the menu does not read Disconnect Cells");
    }
    s.type_loose("Joined: ");
    s.command_loose(CommandId::ConnectCells);
    let g = grid(&s);
    assert_eq!(g[0][1], "Joined: pear", "{g:?}");
    assert_eq!(g[0][2], "3.00", "hidden cells keep their text: {g:?}");

    // Headings, leaders, protection (toggled on, then off).
    s.command_loose(CommandId::TableHeadings);
    if menu_check(s.app(), CommandId::TableHeadings) != Some(true) {
        s.fail("Headings", "the check mark is not on");
    }
    s.command_loose(CommandId::TableHeadings);
    s.command_loose(CommandId::LeaderDots);
    s.command_loose(CommandId::LeaderNone);
    s.command_loose(CommandId::ProtectCells);
    s.command_loose(CommandId::ProtectCells);
    s.command_loose(CommandId::DistributeColumns);
    s.command_act(CommandId::DistributeRows);
    for style in [
        CommandId::TableStyleGrid,
        CommandId::TableStyleHeaderRow,
        CommandId::TableStyleBandedRows,
        CommandId::TableStyleOutline,
        CommandId::TableStylePlain,
    ] {
        s.command_loose(style);
    }

    // The Header style left row 0 a heading, which sorting leaves in place;
    // take that away so the whole column sorts.
    if !table_of(&s).row_options.headings.is_empty() {
        go_to_cell(&mut s, 0, 1);
        s.command_loose(CommandId::TableHeadings);
        assert!(table_of(&s).row_options.headings.is_empty());
    }
    // Sort by the fruit column, up and down.
    go_to_cell(&mut s, 2, 1);
    s.command_loose(CommandId::SortAscending);
    let names: Vec<_> = grid(&s).iter().map(|row| row[1].clone()).collect();
    let mut expected = names.clone();
    expected.sort_by_key(|name| name.to_lowercase());
    if names != expected {
        s.fail("Sort Ascending", &format!("the fruit column is {names:?}"));
    }
    // Rows move whole: the count stays with its fruit.
    let g = grid(&s);
    let apple = g.iter().find(|row| row[1] == "apple").unwrap();
    assert_eq!(apple[2], "12.00", "{g:?}");
    s.command_loose(CommandId::SortDescending);
    let names: Vec<_> = grid(&s).iter().map(|row| row[1].clone()).collect();
    let mut reversed = expected.clone();
    reversed.reverse();
    if names != reversed {
        s.fail("Sort Descending", &format!("the fruit column is {names:?}"));
    }

    // Table to text and back to a table.
    s.command_loose(CommandId::TableToText);
    if s.document().frame_stack().iter().any(|id| {
        matches!(
            s.document().frame(*id).map(|frame| &frame.content),
            Some(FrameContent::Table(_))
        )
    }) {
        s.fail("Table to text", "a table is left");
    }
    let main = s.main_flow();
    let last = s.document().paragraphs().len() - 1;
    let end = s.document().paragraphs()[last].text.len();
    s.caret(main, 1, 0, 1, false);
    s.caret(main, last, end, 1, true);
    s.command_loose(CommandId::TextToTable);
    assert!(table_of(&s).rows.len() >= 3);

    // Delete the table with the caret in it, then undo it all the way.
    go_to_cell(&mut s, 0, 0);
    s.command_loose(CommandId::DeleteTable);
    s.type_loose(" After the table.");

    let path = s.save_sam("TABSTRU.SAM");
    s.reopen(&path);
}

#[test]
fn table_properties_in_the_rail_change_the_table_live() {
    let mut s = Scenario::new_document("table-properties");
    s.clean_folder();
    s.type_loose("Prices");
    insert_table_dialog(&mut s, 3, 3);
    for row in 0..3 {
        for column in 0..3 {
            go_to_cell(&mut s, row, column);
            s.type_loose(&format!("{}{}", ['A', 'B', 'C'][column], row + 1));
        }
    }
    go_to_cell(&mut s, 0, 0);
    s.command_act(CommandId::TableProperties);
    if s.app().table_properties.is_none() {
        s.fail("Table Properties", "the rail panel did not open");
    }
    s.edit_loose("column width 1.5 in", |rt| {
        rt.send(Message::TableProperties(TableAction::Text(
            Field::Width,
            "1.5".into(),
        )));
        rt.send(Message::TableProperties(TableAction::Submit(Field::Width)));
    });
    assert_eq!(table_of(&s).columns[0], 2_160);
    // Typing in the cell goes on with the panel open.
    s.type_loose("x");
    s.edit_loose("all lines", |rt| {
        rt.send(Message::TableProperties(TableAction::Lines(Side::All)));
    });
    s.edit_loose("centre the table", |rt| {
        rt.send(Message::TableProperties(TableAction::Center(true)));
    });
    s.edit_loose("line around", |rt| {
        rt.send(Message::TableProperties(TableAction::LineAround(true)));
    });
    s.edit_loose("Automatic off", |rt| {
        rt.send(Message::TableProperties(TableAction::Automatic(false)));
    });
    s.edit_loose("Automatic on", |rt| {
        rt.send(Message::TableProperties(TableAction::Automatic(true)));
    });
    s.edit_loose("insert a row after", |rt| {
        rt.send(Message::TableProperties(TableAction::Insert {
            rows: true,
            after: true,
        }));
    });
    s.edit_loose("distribute columns", |rt| {
        rt.send(Message::TableProperties(TableAction::DistributeColumns));
    });
    s.edit_loose("leader dots", |rt| {
        rt.send(Message::TableProperties(TableAction::Leader(1)));
    });
    s.edit_loose("remove the lines", |rt| {
        rt.send(Message::TableProperties(TableAction::RemoveLines));
    });
    s.edit_loose("fill", |rt| {
        rt.send(Message::Colour(
            crate::colour_control::ColourAction::Toggle(
                crate::colour_control::Target::TableFill,
                amipro_document::WHITE,
            ),
        ));
        rt.send(Message::Colour(
            crate::colour_control::ColourAction::Choose(0x00_FFFF),
        ));
    });
    assert_eq!(table_of(&s).cells[0].format.fill, Some(0x00_FFFF));
    // Type in the row inserted, so every cell holds text.
    for column in 0..3 {
        go_to_cell(&mut s, 1, column);
        s.type_loose(&format!("n{column}"));
    }
    // Moving the caret out of the table closes the panel.
    s.caret_to_end_of_main();
    if s.app().table_properties.is_some() {
        s.fail("caret out", "Table Properties stayed open");
    }

    let columns = table_of(&s).columns;
    let path = s.save_sam("TABPROP.SAM");
    let s = s.reopen(&path);
    assert_eq!(table_of(&s).columns, columns);
    assert_eq!(table_of(&s).cells[0].format.fill, Some(0x00_FFFF));
}

impl Scenario {
    /// As [`Scenario::edit`] but without the caret comparison after Redo
    /// (Redo of an edit in a table cell leaves the caret elsewhere; the
    /// dedicated scenarios at the end of this file check that on its own).
    fn edit_loose(&mut self, what: &str, step: impl FnOnce(&mut Runtime)) {
        self.begin(what);
        let before = describe(self.document());
        step(&mut self.rt);
        let after = describe(self.document());
        let selection = self.rt.app.page_selection;
        if before == after {
            self.fail(what, "the step changed nothing in the document");
        }
        self.check(what);
        self.rt.send(Message::Command(CommandId::Undo));
        let undone = describe(self.document());
        if undone != before {
            let diff = fact_diff(&before, &undone);
            self.fail(
                what,
                &format!("Undo did not return the document before the step:\n{diff}"),
            );
        }
        self.check(&format!("{what} (undone)"));
        self.rt.send(Message::Command(CommandId::Redo));
        let redone = describe(self.document());
        if redone != after {
            let diff = fact_diff(&after, &redone);
            self.fail(
                what,
                &format!("Redo did not return the document after the step:\n{diff}"),
            );
        }
        // The caret goes back to where the step left it.
        self.rt.send(Message::Page(PageAction::MoveTo {
            position: selection.anchor,
            page: selection.page,
            extend: false,
        }));
        if selection.anchor != selection.focus {
            self.rt.send(Message::Page(PageAction::MoveTo {
                position: selection.focus,
                page: selection.page,
                extend: true,
            }));
        }
        self.check(&format!("{what} (redone)"));
    }

    fn type_loose(&mut self, text: &str) {
        let text = text.to_owned();
        self.edit_loose(&format!("type {text:?}"), move |rt| {
            rt.send(Message::Page(PageAction::Insert(text)));
        });
    }

    fn enter_loose(&mut self) {
        self.edit_loose("Enter", |rt| {
            rt.send(Message::Page(PageAction::ParagraphBreak));
        });
    }

    fn command_loose(&mut self, command: CommandId) {
        self.edit_loose(&format!("{command:?}"), move |rt| {
            rt.send(Message::Command(command));
        });
    }

    /// The caret at the end of the main text's first paragraph.
    fn caret_to_end_of_main(&mut self) {
        let main = self.main_flow();
        let byte = self.document().paragraphs()[0].text.len();
        self.caret(main, 0, byte, 1, false);
    }
}

#[test]
fn cells_copied_cut_pasted_and_dropped() {
    let mut s = Scenario::new_document("table-clipboard");
    s.clean_folder();
    s.type_loose("Cells");
    insert_table_dialog(&mut s, 3, 3);
    for row in 0..3 {
        for column in 0..3 {
            go_to_cell(&mut s, row, column);
            s.type_loose(&format!("{}{}", ['A', 'B', 'C'][column], row + 1));
        }
    }
    let frame = table_frame(&s);
    let select = |s: &mut Scenario, anchor: (usize, usize), focus: (usize, usize)| {
        s.act("select cells", move |rt| {
            rt.send(Message::Page(PageAction::SelectCells {
                frame,
                anchor,
                focus,
            }));
        });
    };

    // Copy A1:B2, paste at C3's column… (to row 2, column 1).
    select(&mut s, (0, 0), (1, 1));
    s.command_act(CommandId::Copy);
    go_to_cell(&mut s, 2, 1);
    s.command_loose(CommandId::Paste);
    let g = grid(&s);
    assert_eq!(g[2][1], "A1", "{g:?}");
    assert_eq!(g[2][2], "B1", "{g:?}");

    // Cut a column's cells: they empty; Paste puts them back elsewhere.
    select(&mut s, (0, 2), (1, 2));
    s.command_loose(CommandId::Cut);
    assert_eq!(text_of(&s, 0, 2), "");
    assert_eq!(text_of(&s, 1, 2), "");
    go_to_cell(&mut s, 0, 0);
    s.command_loose(CommandId::Paste);
    assert_eq!(text_of(&s, 0, 0), "C1");
    assert_eq!(text_of(&s, 1, 0), "C2");

    // Delete selected cells; the rows stay.
    select(&mut s, (1, 1), (2, 2));
    s.edit_loose("Delete the selected cells", |rt| {
        rt.send(Message::Page(PageAction::Delete));
    });

    // Tabbed text pasted in fills cells, a line a row.
    s.edit_loose("tabbed text arrives on the clipboard and is pasted", |rt| {
        rt.clipboard = Some("p\tq\nr\ts".into());
        rt.send(Message::Command(CommandId::Paste));
    });
    // (the caret is the focus cell of the last selection)

    // Drag cells onto another cell: a row moved.
    s.edit_loose("drop row 0 on row 2", |rt| {
        rt.send(Message::Page(PageAction::SelectCells {
            frame,
            anchor: (0, 0),
            focus: (0, 2),
        }));
        rt.send(Message::Page(PageAction::DropCells {
            frame,
            to: (2, 0),
            copy: false,
        }));
    });
    // Alt+Down moves the caret's row.
    go_to_cell(&mut s, 0, 0);
    s.edit_loose("Alt+Down moves the row", |rt| {
        rt.send(Message::Page(PageAction::MoveRows { down: true }));
    });
    expect_caret(&s, 1, 0);

    // A whole table copied is pasted in the text as a new table.
    s.act("select the whole table", |rt| {
        rt.send(Message::Command(CommandId::SelectTable));
    });
    s.command_act(CommandId::Copy);
    let main = s.main_flow();
    let end = s.document().paragraphs()[0].text.len();
    s.caret(main, 0, end, 1, false);
    s.enter_loose();
    s.command_loose(CommandId::Paste);
    let tables = s
        .document()
        .frame_stack()
        .iter()
        .filter(|id| {
            matches!(
                s.document().frame(**id).map(|frame| &frame.content),
                Some(FrameContent::Table(_))
            )
        })
        .count();
    if tables != 2 {
        s.fail(
            "Paste",
            &format!("{tables} tables after pasting a whole table"),
        );
    }

    let path = s.save_sam("TABCLIP.SAM");
    s.reopen(&path);
}

#[test]
fn a_long_table_runs_across_pages_and_is_edited_there() {
    let mut s = Scenario::new_document("table-pages");
    s.clean_folder();
    s.type_loose("A long list follows.");
    // 60 rows of two columns, from the dialog.
    s.command_act(CommandId::Table);
    s.act("type 2 columns and 60 rows", |rt| {
        rt.send(Message::InsertTable(InsertTableAction::Columns("2".into())));
        rt.send(Message::InsertTable(InsertTableAction::Rows("60".into())));
    });
    s.act("Insert", |rt| {
        rt.send(Message::InsertTable(InsertTableAction::Insert));
    });
    let pages = s.app().pages.layout().pages.len();
    if pages < 2 {
        s.fail("layout", &format!("60 rows laid out on {pages} page(s)"));
    }
    // Fill rows near the start, the middle and the end.
    for row in [0, 29, 59] {
        go_to_cell(&mut s, row, 0);
        s.type_loose(&format!("row {row}"));
        go_to_cell(&mut s, row, 1);
        s.type_loose(&format!("{}", row * 10));
    }
    // The caret moved to a row on a later page reports that page.
    go_to_cell(&mut s, 59, 0);
    let page = s.app().caret_page_number();
    if page < 2 {
        s.fail("caret", &format!("the caret in row 59 is on page {page}"));
    }
    // Tab from the last cell adds a row, which goes on the last page.
    go_to_cell(&mut s, 59, 1);
    s.edit_loose("Tab adds a row", |rt| {
        rt.send(Message::Page(PageAction::CellStep { forward: true }));
    });
    s.type_loose("added");
    // Ctrl+Up across the page break.
    s.no_change("Ctrl+Up", |rt| {
        rt.send(Message::Page(PageAction::CellMove {
            down: -1,
            right: 0,
            extend: false,
        }));
    });
    // A page break set in a cell starts its row on the next page (§26).
    go_to_cell(&mut s, 10, 0);
    s.command_loose(CommandId::PageBreak);
    if menu_check(s.app(), CommandId::PageBreak) != Some(true) {
        s.fail("Page Break", "the check mark is not on in the row");
    }
    s.command_loose(CommandId::PageBreak);
    // Delete a row on a later page, and headings repeat.
    go_to_cell(&mut s, 0, 0);
    s.command_loose(CommandId::TableHeadings);
    go_to_cell(&mut s, 40, 0);
    s.command_loose(CommandId::DeleteRow);
    assert_eq!(table_of(&s).rows.len(), 60);
    // Sorting a long table.
    go_to_cell(&mut s, 5, 0);
    s.command_loose(CommandId::SortDescending);
    // Page count: the end of the main text after the table.
    s.caret_to_end();
    s.type_loose(" The end.");

    let path = s.save_sam("TABPAGES.SAM");
    let s = s.reopen(&path);
    assert_eq!(table_of(&s).rows.len(), 60);
}

/// Undo and Redo of Insert › Table… (the checker's rule: the caret after
/// Redo is where the step left it, in the first cell).
#[test]
fn insert_table_undone_and_redone_keeps_the_caret_in_the_first_cell() {
    let mut s = Scenario::new_document("table-insert-redo");
    s.type_text("Above");
    s.command_act(CommandId::Table);
    s.edit("Insert the table", |rt| {
        rt.send(Message::InsertTable(InsertTableAction::Pick(2, 2)));
        rt.send(Message::InsertTable(InsertTableAction::Insert));
    });
    expect_caret(&s, 0, 0);
}

/// Typing in a cell, Undo, Redo: the caret after Redo is after the typed
/// text (as it is in the main text).
#[test]
fn typing_in_a_cell_undone_and_redone_keeps_the_caret_after_the_text() {
    let mut s = Scenario::new_document("table-type-redo");
    s.type_text("Above");
    s.command_act(CommandId::Table);
    s.act("Insert", |rt| {
        rt.send(Message::InsertTable(InsertTableAction::Pick(2, 2)));
        rt.send(Message::InsertTable(InsertTableAction::Insert));
    });
    s.type_text("cell text");
}

/// A user draws the grid before typing: Table Properties › Lines on all
/// the cells of a new table, then Save As SAM. Empty cells with lines are
/// dropped by the writer (`frames.rs` `write_table` skips an empty cell
/// with no kept record, whatever its `CellFormat`).
#[test]
fn lines_and_fill_on_empty_cells_survive_save_as_sam() {
    let mut s = Scenario::new_document("table-empty-lines");
    s.clean_folder();
    s.type_text("Grid first");
    insert_table_dialog(&mut s, 2, 2);
    let frame = table_frame(&s);
    s.act("select all cells", move |rt| {
        rt.send(Message::Page(PageAction::SelectCells {
            frame,
            anchor: (0, 0),
            focus: (1, 1),
        }));
    });
    s.command_act(CommandId::TableProperties);
    s.edit_loose("all lines", |rt| {
        rt.send(Message::TableProperties(TableAction::Lines(Side::All)));
    });
    let path = s.save_sam("GRID.SAM");
    let s = s.reopen(&path);
    assert_eq!(table_of(&s).cells[0].format.lines.word(), 0x1111);
}

/// Tab in the last cell adds a row; Undo and Redo of that: the caret is in
/// the new row's first cell afterwards.
#[test]
fn tab_in_the_last_cell_undone_and_redone_keeps_the_caret_in_the_new_row() {
    let mut s = Scenario::new_document("table-tab-redo");
    s.type_text("Above");
    s.command_act(CommandId::Table);
    s.act("Insert", |rt| {
        rt.send(Message::InsertTable(InsertTableAction::Pick(2, 1)));
        rt.send(Message::InsertTable(InsertTableAction::Insert));
    });
    go_to_cell(&mut s, 0, 1);
    s.edit("Tab adds a row", |rt| {
        rt.send(Message::Page(PageAction::CellStep { forward: true }));
    });
    expect_caret(&s, 1, 0);
}
