//! Merge, labels, envelopes, and data files, as a user does them
//! (`UI_SPEC.md` §38; `UI_ACCEPTANCE.md` §10, "Merge, labels, envelopes").

use super::*;
use crate::merge_panel::{MergeAction as A, StartFrom, Step};
use cosmic::iced::keyboard::{Key, Modifiers, key::Named};
use cosmic::iced::mouse::Button;
use std::path::Path;

// ---- What a user does with the keyboard and the mouse ---------------------

/// Keys typed one by one, as the window delivers them.
fn typed(rt: &mut Runtime, text: &str) {
    for character in text.chars() {
        rt.events(&[key_press(
            Key::Character(character.to_string().into()),
            Modifiers::default(),
        )]);
    }
}

fn named(rt: &mut Runtime, key: Named) {
    rt.events(&[key_press(Key::Named(key), Modifiers::default())]);
}

pub(super) fn lines(s: &Scenario) -> Vec<String> {
    s.document()
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.clone())
        .collect()
}

/// A step like `act`, but with the checker skipped while the preview
/// shows a record: the hidden editor does not follow the preview (see
/// `the_hidden_editor_follows_the_preview`), which would hide everything
/// after it.
pub(super) fn step(s: &mut Scenario, what: &str, f: impl FnOnce(&mut Runtime)) {
    s.begin(what);
    f(&mut s.rt);
    if s.app().merge_preview.is_none() {
        s.check(what);
    }
}

/// The number of power-field marks in the main text.
fn marks(s: &Scenario) -> usize {
    s.document()
        .main_flow()
        .paragraphs
        .iter()
        .flat_map(|paragraph| &paragraph.marks)
        .filter(|mark| amipro_document::PowerMark::parse(&mark.raw).is_some())
        .count()
}

/// A click on page 1 at the end of the main text's paragraph `paragraph`
/// (right of its last line), as a user puts the caret there.
fn click_end_of(s: &mut Scenario, paragraph: usize) {
    step(
        s,
        &format!("click at the end of line {paragraph}"),
        move |rt| {
            rt.redraw();
            let main = rt.app.document.main_flow_id();
            let (x, y) = {
                let layout = rt.app.pages.layout();
                let line = layout.pages[0]
                    .lines
                    .iter()
                    .rfind(|line| line.flow == main && line.paragraph_index == paragraph)
                    .unwrap_or_else(|| panic!("no line for paragraph {paragraph} on page 1"));
                (
                    line.x + line.slot_width - 20.0,
                    line.top + line.height / 2.0,
                )
            };
            let point = rt.on_page(x, y);
            rt.click(Button::Left, point);
        },
    );
}

/// A click in the cell the caret is in (the first record's first cell
/// after New Data File…), as a user clicks a cell to type there.
fn click_caret_cell(s: &mut Scenario) {
    s.act("click in the cell", |rt| {
        rt.redraw();
        let focus = rt.app.page_selection.focus;
        let (x, y) = {
            let layout = rt.app.pages.layout();
            let line = layout
                .pages
                .iter()
                .flat_map(|page| &page.lines)
                .find(|line| line.flow == focus.flow && line.paragraph_index == focus.paragraph)
                .unwrap_or_else(|| panic!("no line for the caret's cell"));
            (
                (line.x + line.width + 150.0).min(line.x + line.slot_width - 20.0),
                line.top + line.height / 2.0,
            )
        };
        let point = rt.on_page(x, y);
        rt.click(Button::Left, point);
    });
}

/// The panel of the active tab.
pub(super) fn panel(s: &Scenario) -> &crate::merge_panel::MergePanel {
    s.app()
        .merge_panel
        .as_ref()
        .unwrap_or_else(|| s.fail("panel", "the Merge panel is not open"))
}

fn panel_message(s: &Scenario) -> Option<String> {
    s.app()
        .merge_panel
        .as_ref()
        .and_then(|panel| panel.message.clone())
}

pub(super) fn go_to_tab(s: &mut Scenario, index: usize) {
    s.act(&format!("click tab {index}"), move |rt| {
        rt.send(Message::ActivateTab(index));
    });
    assert_eq!(s.app().active_session, index);
}

/// Moves to the next cell of the data file's table: Tab; in the last cell
/// it adds a record.
fn tab_in_data(s: &mut Scenario) {
    let form = s.app().data_form.as_ref().expect("a data file's tab");
    let (row, column) = crate::table_editing::caret_cell(&s.app().document, s.app().page_selection)
        .map(|(_, row, column)| (row, column))
        .expect("the caret is in a cell");
    let last = row == form.table.records.len() && column + 1 == form.table.names.len();
    if last {
        // Not `edit`: Redo of this step puts the caret elsewhere (see
        // `undo_and_redo_in_a_data_files_cells_keep_the_caret`).
        s.act("Tab in the last cell adds a record", |rt| {
            named(rt, Named::Tab);
        });
    } else {
        s.no_change("Tab to the next cell", |rt| {
            named(rt, Named::Tab);
        });
    }
}

/// New Data File… with `names`, written to `name` in the scratch folder,
/// then `records` typed into its table key by key (Tab between cells, a
/// new record from the last cell). Leaves the data file's tab active and
/// returns its path and the letter's tab.
pub(super) fn make_data_file(
    s: &mut Scenario,
    name: &str,
    names: &str,
    records: &[&[&str]],
) -> (PathBuf, usize) {
    let letter = s.app().active_session;
    s.command_act(CommandId::Merge);
    assert!(panel(s).shown, "File > Merge... opens the panel");
    s.act("New Data File…", |rt| {
        rt.send(Message::Merge(A::NewDataFile));
    });
    let names = names.to_owned();
    s.act("type the field names", move |rt| {
        rt.send(Message::Merge(A::NewDataNames(names)));
    });
    let path = s.folder().join(name);
    let target = path.clone();
    s.act("the chooser returns the new file", move |rt| {
        rt.send(Message::Merge(A::NewDataFileChosen(target)));
    });
    assert!(path.is_file(), "the data file was written");
    assert_ne!(
        s.app().active_session,
        letter,
        "the data file opens in a tab"
    );
    let columns = s
        .app()
        .data_form
        .as_ref()
        .expect("the table")
        .table
        .names
        .len();
    click_caret_cell(s);
    for (index, record) in records.iter().enumerate() {
        assert_eq!(record.len(), columns);
        for (column, value) in record.iter().enumerate() {
            if !value.is_empty() {
                let value = (*value).to_owned();
                // Not `edit`, for the same reason as Tab below.
                s.act(&format!("type {value:?}"), move |rt| {
                    rt.send(Message::Page(PageAction::Insert(value)));
                });
            }
            if index + 1 < records.len() || column + 1 < columns {
                tab_in_data(s);
            }
        }
    }
    (path, letter)
}

/// Inserts merge field `name` from the panel at the caret (one undo step).
pub(super) fn insert_field(s: &mut Scenario, name: &str) {
    let name = name.to_owned();
    s.edit(&format!("Insert merge field {name}"), move |rt| {
        rt.send(Message::Merge(A::Insert(name)));
    });
}

// ---- Scenarios -------------------------------------------------------------

/// The whole task: a data file made as a table (records typed with Tab,
/// sorted, saved), a form letter with fields typed around, the preview
/// stepped through, merged to a new document, everything saved and
/// reopened.
#[test]
fn a_form_letter_is_made_on_a_new_table_data_file_and_merged_to_a_new_document() {
    let mut s = Scenario::new_document("merge-form-letter");
    s.clean_folder();
    s.type_text("Dear ");
    let (data, letter) = make_data_file(
        &mut s,
        "CLIENTS.SAM",
        "First, Last, Address, City, State, Zip",
        &[
            &["Charlie", "Day", "1 Elm St", "Hickory", "NC", "28602"],
            &["Ann", "Brown", "22 Oak Ave", "Boston", "MA", "02134"],
            &["Pam", "Beck", "41 West St", "Seattle", "WA", "98101"],
        ],
    );
    assert_eq!(s.app().data_form.as_ref().unwrap().table.records.len(), 3);
    // Sort by the caret's column (Zip): the records change order.
    s.command(CommandId::SortAscending);
    let zips: Vec<String> = s
        .app()
        .data_form
        .as_ref()
        .unwrap()
        .table
        .records
        .iter()
        .map(|record| record[5].clone())
        .collect();
    assert_eq!(zips, ["02134", "28602", "98101"]);
    // Saved over the file the panel made: the first original is kept.
    let original = std::fs::read(&data).unwrap();
    s.save();
    assert_eq!(
        std::fs::read(crate::file_workflow::sam_backup_path(&data)).unwrap(),
        original
    );
    // The letter follows the saved records.
    go_to_tab(&mut s, letter);
    assert_eq!(panel(&s).data.as_ref().unwrap().table.records.len(), 3);
    assert_eq!(
        panel(&s).keys,
        ["FIRST", "LAST", "ADDRESS", "CITY", "STATE", "ZIP"].map(String::from)
    );

    // The letter: fields built at the caret, text typed around them.
    s.caret_to_end();
    insert_field(&mut s, "First");
    s.type_text(" ");
    insert_field(&mut s, "Last");
    s.type_text(":");
    s.enter();
    s.enter();
    s.type_text("Your file for ");
    insert_field(&mut s, "Address");
    s.type_text(" in ");
    insert_field(&mut s, "City");
    s.type_text(", ");
    insert_field(&mut s, "State");
    s.type_text(" is complete.");
    assert_eq!(
        lines(&s),
        [
            "Dear <First> <Last>:",
            "",
            "Your file for <Address> in <City>, <State> is complete."
        ]
    );
    assert_eq!(marks(&s), 10, "five fields, none inside another");

    // The preview: the first record, then through them, changing nothing.
    step(&mut s, "Preview a record", |rt| {
        rt.send(Message::Merge(A::Preview(true)));
    });
    assert_eq!(lines(&s)[0], "Dear Ann Brown:");
    step(&mut s, "Next record", |rt| {
        rt.send(Message::Merge(A::Next));
    });
    assert_eq!(lines(&s)[0], "Dear Charlie Day:");
    step(&mut s, "Next record", |rt| {
        rt.send(Message::Merge(A::Next));
    });
    assert_eq!(
        lines(&s)[2],
        "Your file for 41 West St in Seattle, WA is complete."
    );
    step(&mut s, "Previous record", |rt| {
        rt.send(Message::Merge(A::Previous));
    });
    assert_eq!(lines(&s)[0], "Dear Charlie Day:");
    step(&mut s, "Preview off", |rt| {
        rt.send(Message::Merge(A::Preview(false)));
    });
    assert_eq!(lines(&s)[0], "Dear <First> <Last>:");

    // Merge to a new document: every record, in the new tab.
    let tabs = s.app().sessions.len();
    s.act("Merge to New Document", |rt| {
        rt.send(Message::Merge(A::ToNewDocument));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1);
    let merged = s.main_text();
    for want in [
        "Dear Ann Brown:",
        "Dear Charlie Day:",
        "Dear Pam Beck:",
        "Your file for 22 Oak Ave in Boston, MA is complete.",
    ] {
        assert!(merged.contains(want), "{want:?} in {merged:?}");
    }
    assert!(merged.find("Ann Brown").unwrap() < merged.find("Charlie Day").unwrap());
    // The letter, back in its tab, is saved with its data file named and
    // reopened: the panel reads the data file again.
    go_to_tab(&mut s, letter);
    let letter_path = s.save_sam("LETTER.SAM");
    let mut reopened = s.reopen(&letter_path);
    reopened.command_act(CommandId::Merge);
    assert_eq!(
        reopened
            .app()
            .merge_panel
            .as_ref()
            .unwrap()
            .data
            .as_ref()
            .map(|d| d.table.records.len()),
        Some(3),
        "{:?}",
        panel_message(&reopened)
    );
    step(&mut reopened, "Preview a record", |rt| {
        rt.send(Message::Merge(A::Preview(true)));
    });
    assert_eq!(lines(&reopened)[0], "Dear Ann Brown:");

    // The merged document is typed in and saved like any other.
    let merged_tab = s.app().sessions.len() - 1;
    go_to_tab(&mut s, merged_tab);
    s.caret_to_end();
    s.type_text(" Thank you.");
    let merged_path = s.save_sam("MERGED.SAM");
    let mut merged_window = s.reopen(&merged_path);
    assert!(merged_window.main_text().contains("Thank you."));
    merged_window.act("quit", |_| {});
}

/// The user's second use (2026-11-04): the address block typed with the
/// preview checked between steps and the caret put by a click in the
/// preview; then typing, Enter, Backspace, and Delete around the fields.
#[test]
fn an_address_block_is_typed_and_edited_around_the_preview_with_keys_and_clicks() {
    let mut s = Scenario::new_document("merge-address-block");
    s.clean_folder();
    let (_, letter) = make_data_file(
        &mut s,
        "DATA.SAM",
        "First, Last, Address, City, State, Zip",
        &[&["Charlie", "Day", "1", "Hickory", "NC", "28602"]],
    );
    s.save();
    go_to_tab(&mut s, letter);
    s.click_page_text();
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
    for part in steps {
        // Preview on, then a click at the end of the last line as shown.
        step(&mut s, "Preview a record", |rt| {
            rt.send(Message::Merge(A::Preview(true)));
        });
        let last = lines(&s).len() - 1;
        click_end_of(&mut s, last);
        // The step is made in the letter: the preview goes off with it. The
        // step is one undo step, but from the preview's document, so Undo
        // is checked by hand below.
        let shown_before = lines(&s);
        if let Some(name) = part.strip_prefix("F:") {
            let name = name.to_owned();
            step(&mut s, &format!("Insert merge field {name}"), move |rt| {
                rt.send(Message::Merge(A::Insert(name)));
            });
        } else if *part == "\n" {
            step(&mut s, "Enter", |rt| named(rt, Named::Enter));
        } else {
            let text = (*part).to_owned();
            step(&mut s, &format!("type {text:?}"), move |rt| {
                typed(rt, &text)
            });
        }
        assert!(
            s.app().merge_preview.is_none(),
            "an edit turns the preview off"
        );
        assert_ne!(lines(&s), shown_before);
        assert_eq!(marks(&s) % 2, 0);
    }
    assert_eq!(
        lines(&s),
        ["<First> <Last>", "<Address>", "<City> <State> <Zip>"]
    );
    assert_eq!(marks(&s), 12, "six fields, none inside another");
    step(&mut s, "Preview a record", |rt| {
        rt.send(Message::Merge(A::Preview(true)));
    });
    assert_eq!(lines(&s), ["Charlie Day", "1", "Hickory NC 28602"]);

    // A click inside a shown value, typing: made in the letter, after the
    // field the click was in, never inside it.
    let main = s.main_flow();
    step(&mut s, "click in the value Charlie", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(main, 0, 3),
            page: 1,
            extend: false,
        }));
    });
    step(&mut s, "type in the preview", |rt| typed(rt, "x"));
    assert_eq!(lines(&s)[0], "<First>x <Last>");
    assert_eq!(marks(&s), 12);
    step(&mut s, "Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(lines(&s)[0], "<First> <Last>");
    step(&mut s, "Redo", |rt| {
        rt.send(Message::Command(CommandId::Redo));
    });
    assert_eq!(lines(&s)[0], "<First>x <Last>");

    // Enter in the preview at the end of a value, then in a letter.
    step(&mut s, "Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    step(&mut s, "Preview a record", |rt| {
        rt.send(Message::Merge(A::Preview(true)));
    });
    click_end_of(&mut s, 1);
    step(&mut s, "Enter in the preview", |rt| named(rt, Named::Enter));
    assert_eq!(
        lines(&s),
        ["<First> <Last>", "<Address>", "", "<City> <State> <Zip>"]
    );
    assert_eq!(marks(&s), 12);
    step(&mut s, "Undo", |rt| {
        rt.send(Message::Command(CommandId::Undo));
    });
    assert_eq!(lines(&s).len(), 3);

    // Backspace after a field and Delete before one take the field whole,
    // one undo step each, leaving nothing empty behind.
    step(&mut s, "Preview off", |rt| {
        rt.send(Message::Merge(A::Preview(false)));
    });
    click_end_of(&mut s, 1);
    s.edit("Backspace after <Address>", |rt| {
        named(rt, Named::Backspace)
    });
    assert_eq!(lines(&s), ["<First> <Last>", "", "<City> <State> <Zip>"]);
    assert_eq!(marks(&s), 10);
    s.caret(main, 2, 0, 1, false);
    s.edit("Delete before <City>", |rt| named(rt, Named::Delete));
    assert_eq!(lines(&s)[2], " <State> <Zip>");
    assert_eq!(marks(&s), 8);
    // Typing over a selection that holds fields leaves no field behind.
    s.select_main((2, 0), (2, 14));
    s.type_text("ZZ");
    assert_eq!(lines(&s)[2], "ZZ");
    assert_eq!(marks(&s), 4);

    let path = s.save_sam("BLOCK.SAM");
    let mut reopened = s.reopen(&path);
    reopened.command_act(CommandId::Merge);
    step(&mut reopened, "Preview a record", |rt| {
        rt.send(Message::Merge(A::Preview(true)));
    });
    assert_eq!(lines(&reopened)[0], "Charlie Day");
}

impl Scenario {
    /// A click on the page's text so that the surface has the keys.
    fn click_page_text(&mut self) {
        self.act("click on the page", |rt| {
            rt.redraw();
            let point = rt.on_page(2_000.0, 1_700.0);
            rt.click(Button::Left, point);
        });
    }
}

/// New Data File… says "Type in the cells" (the status bar): a user who
/// clicked the panel's buttons and then types right away expects the first
/// cell to take the keys, without a click on the table first.
#[test]
fn typing_right_after_new_data_file_reaches_the_first_cell() {
    let mut s = Scenario::new_document("merge-focus-after-new-data");
    s.clean_folder();
    s.command_act(CommandId::Merge);
    s.click_page_text();
    s.act("New Data File…", |rt| {
        rt.send(Message::Merge(A::NewDataFile));
    });
    s.act("type the field names", |rt| {
        rt.send(Message::Merge(A::NewDataNames("Name, City".into())));
    });
    // The panel's Create button is a click outside the page.
    s.act("click in the rail", |rt| {
        rt.redraw();
        rt.click(Button::Left, cosmic::iced::Point::new(150.0, 760.0));
    });
    let target = s.folder().join("DATA.SAM");
    s.act("the chooser returns the new file", move |rt| {
        rt.send(Message::Merge(A::NewDataFileChosen(target)));
    });
    // Each key is its own undo step on the page surface.
    s.edit("type A in the first cell", |rt| typed(rt, "A"));
    s.edit("type n", |rt| typed(rt, "n"));
    s.edit("type n", |rt| typed(rt, "n"));
    assert_eq!(
        s.app().data_form.as_ref().unwrap().table.records[0][0],
        "Ann"
    );
}

/// Undo and Redo of a typed value or a new record in the data file's table
/// leave the caret where the step left it, as they do in the main text.
#[test]
fn undo_and_redo_in_a_data_files_cells_keep_the_caret() {
    let mut s = Scenario::new_document("merge-data-undo-caret");
    s.clean_folder();
    make_data_file(&mut s, "DATA.SAM", "Name, City", &[]);
    click_caret_cell(&mut s);
    s.type_text("Ann");
    s.no_change("Tab", |rt| named(rt, Named::Tab));
    s.type_text("Boston");
    s.edit("Tab in the last cell adds a record", |rt| {
        named(rt, Named::Tab)
    });
}

fn pdf_len(path: &Path) -> usize {
    std::fs::read(path).map(|bytes| bytes.len()).unwrap_or(0)
}

/// A letter on a delimited (CSV) data file, with conditions, merged to
/// every output the panel has.
#[test]
fn a_letter_on_a_delimited_data_file_is_merged_with_conditions_to_every_output() {
    let mut s = Scenario::new_document("merge-outputs");
    let folder = s.clean_folder();
    let csv = folder.join("CLIENTS.CSV");
    std::fs::write(
        &csv,
        "First,Last,City,Amount\nAnn,Brown,Boston,150\nBob,Cole,Dallas,75.50\nCy,Dunn,,\"1,200.00\"\nDee,Eve,Boston,9\n",
    )
    .unwrap();
    s.type_text("Dear ");
    s.command_act(CommandId::Merge);
    s.act("Open the data file (the chooser returns it)", {
        let csv = csv.clone();
        move |rt| {
            rt.send(Message::Merge(A::Chosen(csv)));
        }
    });
    assert_eq!(
        panel(&s).data.as_ref().map(|d| d.table.records.len()),
        Some(4),
        "{:?}",
        panel_message(&s)
    );
    assert_eq!(
        panel(&s).keys,
        ["FIRST", "LAST", "CITY", "AMOUNT"].map(String::from)
    );
    s.caret_to_end();
    insert_field(&mut s, "First");
    s.type_text(" ");
    insert_field(&mut s, "Last");
    s.type_text(":");
    s.enter();
    s.type_text("Your balance is ");
    insert_field(&mut s, "Amount");
    s.type_text(" at ");
    insert_field(&mut s, "City");
    s.type_text(".");
    s.act("the merge-record field", |rt| {
        rt.send(Message::Merge(A::InsertMergeRec));
    });

    // Conditions: City = Boston, and Last = Cole.
    s.act("Add a condition", |rt| {
        rt.send(Message::Merge(A::AddCondition));
    });
    s.act("its field is City", |rt| {
        rt.send(Message::Merge(A::ConditionField(0, 2)));
    });
    s.act("its value Boston", |rt| {
        rt.send(Message::Merge(A::ConditionValue(0, "Boston".into())));
    });
    s.act("commit the value", |rt| {
        rt.send(Message::Merge(A::CommitValue(0)));
    });
    assert_eq!(s.app().merge_records(), [0, 3]);
    s.act("Add a second condition", |rt| {
        rt.send(Message::Merge(A::AddCondition));
    });
    s.act("its field is Last", |rt| {
        rt.send(Message::Merge(A::ConditionField(1, 1)));
    });
    s.act("its value Cole", |rt| {
        rt.send(Message::Merge(A::ConditionValue(1, "Cole".into())));
    });
    s.act("commit the value", |rt| {
        rt.send(Message::Merge(A::CommitValue(1)));
    });
    assert_eq!(
        s.app().merge_records(),
        Vec::<usize>::new(),
        "Boston And Cole"
    );
    s.act("the first joined to the second with Or", |rt| {
        rt.send(Message::Merge(A::ConditionJoin(
            0,
            amipro_document::merge::Join::Or,
        )));
    });
    assert_eq!(s.app().merge_records(), [0, 1, 3], "Boston, or Cole");
    s.act("remove the second condition", |rt| {
        rt.send(Message::Merge(A::RemoveCondition(1)));
    });
    assert_eq!(s.app().merge_records(), [0, 3]);

    // New Document with the conditions: two letters, a page break between.
    let tabs = s.app().sessions.len();
    s.act("Merge to New Document", |rt| {
        rt.send(Message::Merge(A::ToNewDocument));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1);
    let merged = s.main_text();
    assert!(
        merged.contains("Dear Ann Brown:") && merged.contains("Dear Dee Eve:"),
        "{merged:?}"
    );
    assert!(!merged.contains("Bob"), "{merged:?}");
    assert!(
        merged.contains("Your balance is 150 at Boston."),
        "{merged:?}"
    );
    assert!(
        merged.find("Dear Dee").unwrap() > merged.find("Dear Ann").unwrap(),
        "{merged:?}"
    );
    go_to_tab(&mut s, 0);

    // Without the conditions: every record. The empty City merges empty.
    s.act("With conditions off", |rt| {
        rt.send(Message::Merge(A::WithConditions(false)));
    });
    assert_eq!(s.app().merge_records(), [0, 1, 2, 3]);

    // One PDF.
    let pdf = folder.join("ALL.pdf");
    s.act("Merge to PDF (the chooser returns the file)", {
        let pdf = pdf.clone();
        move |rt| {
            rt.send(Message::Merge(A::ToPdfChosen(pdf)));
        }
    });
    let bytes = std::fs::read(&pdf).unwrap_or_default();
    assert!(
        bytes.starts_with(b"%PDF"),
        "{:?} {}",
        panel_message(&s),
        s.app().status
    );
    assert_eq!(pdf_pages(&pdf), 4, "one page a record");

    // One PDF per record named from a field: the folder needs the field.
    s.act("One PDF per record", |rt| {
        rt.send(Message::Merge(A::PdfEach(true)));
    });
    s.act("Merge to PDF with no naming field", |rt| {
        rt.send(Message::Merge(A::ToPdf));
    });
    assert!(
        panel_message(&s).is_some_and(|message| message.contains("Choose the field")),
        "{:?}",
        panel_message(&s)
    );
    s.act("name each PDF by Last", |rt| {
        rt.send(Message::Merge(A::PdfNameField(1)));
    });
    let out = folder.join("pdfs");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("Cole.pdf"), b"older").unwrap();
    s.act("the folder is chosen", {
        let out = out.clone();
        move |rt| {
            rt.send(Message::Merge(A::ToPdfFolderChosen(out)));
        }
    });
    for name in ["Brown.pdf", "Cole (2).pdf", "Dunn.pdf", "Eve.pdf"] {
        assert!(
            pdf_len(&out.join(name)) > 100,
            "{name}: {:?}",
            panel_message(&s)
        );
    }
    assert_eq!(std::fs::read(out.join("Cole.pdf")).unwrap(), b"older");
    assert!(
        panel_message(&s).is_some_and(|message| message.contains("Wrote 4 PDFs")),
        "{:?}",
        panel_message(&s)
    );
    s.act("one PDF per record off", |rt| {
        rt.send(Message::Merge(A::PdfEach(false)));
    });

    // Merge to an Ami Pro file: one document, a page break between records,
    // to a new file only.
    let sam = folder.join("MERGED.SAM");
    s.act("Merge to Ami Pro File", {
        let sam = sam.clone();
        move |rt| {
            rt.send(Message::Merge(A::ToAmiProFileChosen(sam)));
        }
    });
    let merged_file = amipro_formats::import_sam_path(&sam).unwrap().document;
    let text = merged_file.plain_text();
    for want in [
        "Dear Ann Brown:",
        "Dear Bob Cole:",
        "Dear Cy Dunn:",
        "Dear Dee Eve:",
    ] {
        assert!(text.contains(want), "{want}");
    }
    assert!(text.contains("Your balance is 1,200.00 at ."), "{text:?}");
    let first = std::fs::read(&sam).unwrap();
    s.act("Merge to Ami Pro File over the same file", {
        let sam = sam.clone();
        move |rt| {
            rt.send(Message::Merge(A::ToAmiProFileChosen(sam)));
        }
    });
    assert_eq!(std::fs::read(&sam).unwrap(), first, "never over a file");

    // Merge, view & print: each record in place, edits carried, the empty
    // City a genuinely empty value; ends with the printer's dialog (unrun).
    s.allow_empty_fields = true;
    s.act("View & Print", |rt| {
        rt.send(Message::Merge(A::ViewAndPrint));
    });
    assert_eq!(lines(&s)[0], "Dear Ann Brown:");
    let main = s.main_flow();
    s.act("click at the start of the last line", move |rt| {
        rt.send(Message::Page(PageAction::MoveTo {
            position: at(main, 1, 0),
            page: 1,
            extend: false,
        }));
    });
    s.edit("an edit while viewing", |rt| {
        rt.send(Message::Page(PageAction::Insert("* ".into())));
    });
    s.act("Print and View Next", |rt| {
        rt.send(Message::Merge(A::PrintAndViewNext));
    });
    assert_eq!(lines(&s)[0], "Dear Bob Cole:");
    assert!(
        lines(&s)[1].starts_with("* "),
        "the edit is carried: {:?}",
        lines(&s)
    );
    s.act("Skip and View Next", |rt| {
        rt.send(Message::Merge(A::SkipAndViewNext));
    });
    assert_eq!(lines(&s)[0], "Dear Cy Dunn:");
    assert!(lines(&s)[1].contains("1,200.00"), "{:?}", lines(&s));
    s.act("Print All (the printer dialog is not run)", |rt| {
        update_unrun(rt, Message::Merge(A::PrintAll));
    });
    assert!(s.app().merge_viewing.is_none());
    assert_eq!(
        s.app().viewing_job.as_ref().map(Vec::len),
        Some(3),
        "Ann, Cy, Dee"
    );
    s.allow_empty_fields = false;
    assert!(
        lines(&s)[0].contains("<First>"),
        "fields again: {:?}",
        lines(&s)
    );
    s.act("the printer's answer", |rt| {
        let task = rt.app.print_merged(&crate::print_portal::Prepared {
            pages: vec![0, 1, 2],
            paper: None,
            token: 0,
        });
        assert!(task.is_some());
        drop(task);
    });
    assert!(s.app().viewing_job.is_none());

    // Merge to Printer…: the dialog is asked for once.
    s.act("Merge to Printer (not run)", |rt| {
        update_unrun(rt, Message::Merge(A::ToPrinter));
    });
    assert!(panel(&s).printing);
    s.act("the printer's answer", |rt| {
        let task = rt.app.print_merged(&crate::print_portal::Prepared {
            pages: vec![1],
            paper: None,
            token: 0,
        });
        assert!(task.is_some());
        drop(task);
    });
    assert!(!panel(&s).printing);

    // Saved: the conditions are listed (Ami Pro does not store them).
    s.act("With conditions on again", |rt| {
        rt.send(Message::Merge(A::WithConditions(true)));
    });
    let path = s.save_sam_expecting("LETTER.SAM", &["merge condition"]);
    let mut reopened = s.reopen(&path);
    reopened.allow_empty_fields = true;
    reopened.command_act(CommandId::Merge);
    assert_eq!(
        reopened
            .app()
            .merge_panel
            .as_ref()
            .and_then(|p| p.data.as_ref())
            .map(|d| d.table.records.len()),
        Some(4),
        "{:?}",
        panel_message(&reopened)
    );
    reopened.act("Merge to New Document", |rt| {
        rt.send(Message::Merge(A::ToNewDocument));
    });
    assert!(reopened.main_text().contains("Dear Cy Dunn:"));
}

/// Labels: an Avery sheet, the start label, copies, the sheets as a
/// document and as a PDF, another sheet, and off again.
#[test]
fn merged_labels_are_set_up_on_a_sheet_and_made_into_a_document_and_a_pdf() {
    let mut s = Scenario::new_document("merge-labels");
    let folder = s.clean_folder();
    let (_, letter) = make_data_file(
        &mut s,
        "ADDR.SAM",
        "First, Last, City",
        &[&["Ann", "Brown", "Boston"], &["Bob", "Cole", "Dallas"]],
    );
    go_to_tab(&mut s, letter);
    s.caret_to_end();
    insert_field(&mut s, "First");
    s.type_text(" ");
    insert_field(&mut s, "Last");
    s.enter();
    insert_field(&mut s, "City");
    s.act("Merge as labels", |rt| {
        rt.send(Message::Merge(A::Labels(true)));
    });
    let sheet = amipro_editing::merge::labels::LabelSheet::of(s.document()).expect("a sheet");
    assert_eq!(sheet, amipro_editing::merge::labels::SHEETS[0]);
    // The labels' text still reads as the fields.
    assert_eq!(lines(&s), ["<First> <Last>", "<City>"]);
    s.act("3 copies of each label", |rt| {
        rt.send(Message::Merge(A::LabelCopies("3".into())));
    });
    s.act("start at label 29", |rt| {
        rt.send(Message::Merge(A::LabelStart(29)));
    });
    let pdf = folder.join("LABELS.pdf");
    s.act("Merge to PDF", {
        let pdf = pdf.clone();
        move |rt| {
            rt.send(Message::Merge(A::ToPdfChosen(pdf)));
        }
    });
    assert_eq!(
        pdf_pages(&pdf),
        2,
        "label 29 of 30, then 5 more: two sheets"
    );
    let tabs = s.app().sessions.len();
    s.act("Merge to New Document", |rt| {
        rt.send(Message::Merge(A::ToNewDocument));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1);
    assert_eq!(s.app().pages.layout().pages.len(), 2);
    assert!(
        s.app().status.contains("6 labels on 2 sheets"),
        "{}",
        s.app().status
    );
    // The sheets are a document: saved and reopened.
    let labels_path = s.save_sam("SHEETS.SAM");
    let labels_window = s.reopen(&labels_path);
    assert_eq!(labels_window.app().pages.layout().pages.len(), 2);
    go_to_tab(&mut s, letter);
    // Another sheet; its page size changes; then labels off.
    s.act("another sheet from the list", |rt| {
        rt.send(Message::Merge(A::LabelSheet(1)));
    });
    assert_eq!(
        amipro_editing::merge::labels::LabelSheet::of(s.document()),
        Some(amipro_editing::merge::labels::SHEETS[1])
    );
    s.act("Merge as labels off", |rt| {
        rt.send(Message::Merge(A::Labels(false)));
    });
    assert!(amipro_editing::merge::labels::LabelSheet::of(s.document()).is_none());
    let path = s.save_sam("LETTER.SAM");
    s.reopen(&path);
}

/// File › Print Envelope…: the address found in the letter, edited in the
/// panel, a return address saved, the envelope as a document and a PDF,
/// the printer asked (unrun), then every record on its own envelope.
#[test]
fn an_envelope_is_made_from_the_letters_address_and_for_every_record() {
    use crate::envelope_panel::{EnvelopeAction as E, Feed};
    let mut s = Scenario::new(
        "merge-envelope",
        Document::from_plain_text(
            "November 3, 2026\n\nMs. Pam Beck\n41 West St.\nSeattle, WA 98101\n\nDear Ms. Beck:",
        ),
    );
    let folder = s.clean_folder();
    s.command_act(CommandId::PrintEnvelope);
    let recipient = s
        .app()
        .envelope_panel
        .as_ref()
        .expect("the panel")
        .recipient
        .clone();
    assert_eq!(
        recipient,
        ["Ms. Pam Beck", "41 West St.", "Seattle, WA 98101"]
    );
    s.act("fix the recipient's first line", |rt| {
        rt.send(Message::Envelope(E::Recipient(0, "Dr. Pam Beck".into())));
    });
    s.act("return address line 1", |rt| {
        rt.send(Message::Envelope(E::ReturnLine(0, "Terry Allen".into())));
    });
    s.act("return address line 2", |rt| {
        rt.send(Message::Envelope(E::ReturnLine(1, "1 Main St.".into())));
    });
    s.act("name it Home", |rt| {
        rt.send(Message::Envelope(E::ReturnName("Home".into())));
    });
    s.act("Save the return address", |rt| {
        rt.send(Message::Envelope(E::SaveReturn));
    });
    assert_eq!(
        s.app().preferences.return_addresses,
        [("Home".to_string(), "Terry Allen\n1 Main St.".to_string())]
    );
    s.act("size DL", |rt| {
        rt.send(Message::Envelope(E::Size(6)));
    });
    let tabs = s.app().sessions.len();
    s.act("New Document", |rt| {
        rt.send(Message::Envelope(E::NewDocument));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1);
    let page = &s.document().layouts()[0].right;
    assert_eq!((page.width, page.height), (12_472, 6_236), "DL");
    let text = s.main_text();
    assert!(
        text.contains("Terry Allen") || s.document().frames().count() > 0,
        "{text:?}"
    );
    let envelope_text: String = s
        .document()
        .flows()
        .map(|(_, flow)| flow.plain_text())
        .collect::<Vec<_>>()
        .join("|");
    for want in [
        "Dr. Pam Beck",
        "41 West St.",
        "Seattle, WA 98101",
        "Terry Allen",
        "1 Main St.",
    ] {
        assert!(envelope_text.contains(want), "{want}: {envelope_text:?}");
    }
    let path = s.save_sam("ENVELOPE.SAM");
    s.reopen(&path);
    go_to_tab(&mut s, 0);
    let pdf = folder.join("Envelope.pdf");
    s.act("PDF (the chooser returns the file)", {
        let pdf = pdf.clone();
        move |rt| {
            rt.send(Message::Envelope(E::PdfChosen(pdf)));
        }
    });
    assert_eq!(pdf_pages(&pdf), 1);
    s.act("feed at the centre", |rt| {
        rt.send(Message::Envelope(E::Feed(Feed::Centre)));
    });
    s.act("Print… (the dialog is not run)", |rt| {
        update_unrun(rt, Message::Envelope(E::Print));
    });
    s.act("the printer's answer", |rt| {
        let task = rt.app.print_envelope(&crate::print_portal::Prepared {
            pages: vec![0],
            paper: Some((12_240.0, 15_840.0)),
            token: 0,
        });
        assert!(task.is_some());
        drop(task);
    });
    s.act("close the panel", |rt| {
        rt.send(Message::Envelope(E::Close));
    });
    assert!(s.app().envelope_panel.is_none());
}

/// Envelopes for every record of the letter's data file, and Start from.
#[test]
fn envelopes_are_merged_from_the_data_file_and_start_from_opens_the_merge_sheets() {
    use crate::envelope_panel::EnvelopeAction as E;
    let mut s = Scenario::new_document("merge-envelopes-all");
    s.clean_folder();
    let (_, letter) = make_data_file(
        &mut s,
        "ADDR.SAM",
        "Name, Street, City",
        &[
            &["Ann Brown", "1 Elm", "Boston"],
            &["Bob Cole", "2 Oak", "Dallas"],
        ],
    );
    go_to_tab(&mut s, letter);
    s.caret_to_end();
    insert_field(&mut s, "Name");
    s.command_act(CommandId::PrintEnvelope);
    for (line, text) in ["<Name>", "<Street>", "<City>"].iter().enumerate() {
        s.act("recipient line", move |rt| {
            rt.send(Message::Envelope(E::Recipient(line, (*text).into())));
        });
    }
    let tabs = s.app().sessions.len();
    s.act("Merge Envelopes", |rt| {
        rt.send(Message::Envelope(E::MergeEnvelopes));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1);
    let texts: Vec<String> = s
        .document()
        .flows()
        .map(|(_, flow)| flow.plain_text())
        .collect();
    let joined = texts.join("|");
    for want in [
        "Ann Brown",
        "1 Elm",
        "Boston",
        "Bob Cole",
        "2 Oak",
        "Dallas",
    ] {
        assert!(joined.contains(want), "{want}: {joined:?}");
    }
    let path = s.save_sam("ENVS.SAM");
    s.reopen(&path);

    // Start from: Labels, Letter, Envelope each open their sheet.
    go_to_tab(&mut s, letter);
    s.command_act(CommandId::Merge);
    let tabs = s.app().sessions.len();
    s.act("Start from Labels", |rt| {
        rt.send(Message::Merge(A::StartFrom(StartFrom::Labels)));
    });
    assert_eq!(s.app().sessions.len(), tabs + 1);
    assert!(amipro_editing::merge::labels::LabelSheet::of(s.document()).is_some());
    assert!(
        s.app()
            .merge_panel
            .as_ref()
            .is_some_and(|panel| panel.shown)
    );
    s.act("Start from Letter", |rt| {
        rt.send(Message::Merge(A::StartFrom(StartFrom::Letter)));
    });
    assert_eq!(s.app().sessions.len(), tabs + 2);
    s.act("Start from Envelope", |rt| {
        rt.send(Message::Merge(A::StartFrom(StartFrom::Envelope)));
    });
    assert!(s.app().envelope_panel.is_some());
    // The step titles of the panel take the user to each step.
    for step in [Step::DataFile, Step::MergeDocument, Step::Merge] {
        s.act("open a step", move |rt| {
            rt.send(Message::Merge(A::Step(step)));
        });
    }
}

/// A letter whose data file is named saves as Ami Pro's SAM with no loss
/// list: `[recfile]` is a field Ami Pro stores.
#[test]
fn a_letter_with_a_data_file_saves_as_sam_without_a_loss_list() {
    let mut s = Scenario::new_document("merge-recfile-saves");
    let folder = s.clean_folder();
    let csv = folder.join("CLIENTS.CSV");
    std::fs::write(&csv, "First\nAnn\n").unwrap();
    s.type_text("Dear ");
    s.command_act(CommandId::Merge);
    s.act("Open the data file", move |rt| {
        rt.send(Message::Merge(A::Chosen(csv)));
    });
    s.caret_to_end();
    insert_field(&mut s, "First");
    let path = s.save_sam("LETTER.SAM");
    s.reopen(&path);
}

/// The window's hidden editor holds the main text as shown, also while the
/// preview shows a record (the checker's rule, after every step).
#[test]
fn the_hidden_editor_follows_the_preview() {
    let mut s = Scenario::new_document("merge-preview-editor");
    let folder = s.clean_folder();
    let csv = folder.join("C.CSV");
    std::fs::write(&csv, "First\nAnn\nBob\n").unwrap();
    s.type_text("Dear ");
    s.command_act(CommandId::Merge);
    s.act("Open the data file", move |rt| {
        rt.send(Message::Merge(A::Chosen(csv)));
    });
    s.caret_to_end();
    insert_field(&mut s, "First");
    s.act("Preview a record", |rt| {
        rt.send(Message::Merge(A::Preview(true)));
    });
    s.act("Next record", |rt| {
        rt.send(Message::Merge(A::Next));
    });
    s.act("Preview off", |rt| {
        rt.send(Message::Merge(A::Preview(false)));
    });
}

/// The user's first use (2026-11-04): New Data File… onto a file the
/// chooser confirmed (Replace) writes over it, keeping NAME.SAM.bak; a tab
/// with unsaved changes holds it back; a delimited data file is edited as a
/// table and saved in its own form.
#[test]
fn a_new_data_file_replaces_a_confirmed_file_and_a_delimited_one_is_edited_in_its_form() {
    use amipro_editing::merge::data::{Delimiters, new_data_document, read_ami_data};
    let mut s = Scenario::new_document("merge-replace-data");
    let folder = s.clean_folder();
    let path = folder.join("OLDDATA.SAM");
    let old = new_data_document(&["Old".into(), "Place".into()], Delimiters::default());
    let (mut table, delimiters) = read_ami_data(&old).unwrap();
    table.insert_record(0);
    table.set_value(0, 0, "Zed");
    table.set_value(0, 1, "Reno");
    let old = amipro_editing::merge::data::data_document(&table, delimiters);
    std::fs::write(
        &path,
        amipro_formats::sam_document(&old, &amipro_formats::SamOptions::at(0)),
    )
    .unwrap();
    let original = std::fs::read(&path).unwrap();

    s.type_text("Dear ");
    s.command_act(CommandId::Merge);
    s.act("Open the old data file", {
        let path = path.clone();
        move |rt| {
            rt.send(Message::Merge(A::Chosen(path)));
        }
    });
    assert_eq!(
        panel(&s).keys,
        ["OLD", "PLACE"].map(String::from),
        "{:?}",
        panel_message(&s)
    );
    s.caret_to_end();
    insert_field(&mut s, "Old");
    let letter = s.app().active_session;
    s.act("Edit records", |rt| {
        rt.send(Message::Merge(A::EditRecords));
    });
    let old_tab = s.app().active_session;
    assert_ne!(old_tab, letter);
    assert_eq!(
        s.app().data_form.as_ref().unwrap().table.records,
        [["Zed", "Reno"]]
    );
    click_caret_cell(&mut s);
    s.act("type in the first record", |rt| {
        rt.send(Message::Page(PageAction::Insert(" Jr".into())));
    });
    assert!(s.app().dirty);

    // New Data File… onto the open, changed file: held back, both places say so.
    go_to_tab(&mut s, letter);
    s.act("New Data File…", |rt| {
        rt.send(Message::Merge(A::NewDataFile));
    });
    s.act("names", |rt| {
        rt.send(Message::Merge(A::NewDataNames("Name, City".into())));
    });
    s.expect_status("unsaved");
    s.act("the chooser returns the open file", {
        let path = path.clone();
        move |rt| {
            rt.send(Message::Merge(A::NewDataFileChosen(path)));
        }
    });
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(
        s.app().status.contains("unsaved changes"),
        "{}",
        s.app().status
    );
    assert!(panel_message(&s).is_some_and(|m| m.contains("unsaved changes")));
    s.clear_expected_status();

    // The old tab saved in its delimited form, the original kept.
    go_to_tab(&mut s, old_tab);
    // Not `save`: the tab shows the table while the file is Ami Pro's
    // delimited form, which `save`'s read-back (checker suspect) compares
    // with the table.
    s.act("Save", |rt| {
        rt.send(Message::Command(CommandId::Save));
    });
    assert!(!s.app().dirty, "{}", s.app().status);
    let saved = amipro_formats::import_sam(&std::fs::read(&path).unwrap(), None, "x")
        .unwrap()
        .document;
    let (back, kept) = read_ami_data(&saved).unwrap();
    assert_eq!(kept, delimiters);
    assert_eq!(back.records[0], ["Zed Jr", "Reno"]);
    assert_eq!(
        std::fs::read(crate::file_workflow::sam_backup_path(&path)).unwrap(),
        original
    );

    // Now, clean, the chooser's Replace writes the new table over it; the
    // tab shows it; the first original stays as the backup.
    go_to_tab(&mut s, letter);
    s.act("the chooser returns the file (Replace)", {
        let path = path.clone();
        move |rt| {
            rt.send(Message::Merge(A::NewDataFileChosen(path)));
        }
    });
    assert_eq!(
        std::fs::read(crate::file_workflow::sam_backup_path(&path)).unwrap(),
        original,
        "the first original is kept"
    );
    assert_eq!(s.app().active_session, old_tab);
    assert_eq!(s.app().sessions.len(), 2);
    assert_eq!(
        s.app().data_form.as_ref().unwrap().table.names,
        ["Name", "City"]
    );
    assert!(
        s.app().status.contains("Type in the cells"),
        "{}",
        s.app().status
    );
    click_caret_cell(&mut s);
    s.act("type", |rt| {
        rt.send(Message::Page(PageAction::Insert("Ann".into())));
    });
    s.save();
    // The letter's panel follows the new file's names. (Its status bar
    // says "Edit failed: the merge settings are unchanged": a finding, the
    // letter already named this file.)
    s.expect_status("merge settings are unchanged");
    go_to_tab(&mut s, letter);
    s.clear_expected_status();
    assert_eq!(panel(&s).keys, ["NAME", "CITY"].map(String::from));

    // The merge document itself is never the data file.
    let letter_path = folder.join("LETTER.SAM");
    let _ = std::fs::remove_file(&letter_path);
    s.app_mut().sessions[letter].location = crate::DocumentLocation::Sam(letter_path.clone());
    s.expect_status("merge document");
    s.act("New Data File… onto the letter's own file", {
        let letter_path = letter_path.clone();
        move |rt| {
            rt.send(Message::Merge(A::NewDataFile));
            rt.send(Message::Merge(A::NewDataNames("Name".into())));
            rt.send(Message::Merge(A::NewDataFileChosen(letter_path)));
        }
    });
    assert!(!letter_path.exists());
    s.clear_expected_status();
}

/// New Data File… onto a file a clean tab already shows (the chooser's
/// Replace), then typing a record and Save: the table data file is written
/// as it is shown, with no loss list.
#[test]
fn a_replaced_data_files_tab_saves_without_losses() {
    use amipro_editing::merge::data::{Delimiters, new_data_document};
    let mut s = Scenario::new_document("merge-replaced-tab-saves");
    let folder = s.clean_folder();
    let path = folder.join("OLDDATA.SAM");
    let old = new_data_document(&["Old".into()], Delimiters::default());
    std::fs::write(
        &path,
        amipro_formats::sam_document(&old, &amipro_formats::SamOptions::at(0)),
    )
    .unwrap();
    s.command_act(CommandId::Merge);
    s.act("Open the old data file", {
        let path = path.clone();
        move |rt| {
            rt.send(Message::Merge(A::Chosen(path)));
        }
    });
    let letter = s.app().active_session;
    s.act("Edit records", |rt| {
        rt.send(Message::Merge(A::EditRecords));
    });
    go_to_tab(&mut s, letter);
    s.act("New Data File…", |rt| {
        rt.send(Message::Merge(A::NewDataFile));
    });
    s.act("names", |rt| {
        rt.send(Message::Merge(A::NewDataNames("Name, City".into())));
    });
    s.act("the chooser returns the file (Replace)", {
        let path = path.clone();
        move |rt| {
            rt.send(Message::Merge(A::NewDataFileChosen(path)));
        }
    });
    click_caret_cell(&mut s);
    s.act("type", |rt| {
        rt.send(Message::Page(PageAction::Insert("Ann".into())));
    });
    s.save();
}

/// Replacing the very data file the letter already names (the usual case)
/// leaves no error on the letter's status bar.
#[test]
fn replacing_the_data_file_the_letter_already_names_shows_no_error() {
    use amipro_editing::merge::data::{Delimiters, new_data_document};
    let mut s = Scenario::new_document("merge-replace-named-file");
    let folder = s.clean_folder();
    let path = folder.join("DATA.SAM");
    let old = new_data_document(&["Old".into()], Delimiters::default());
    std::fs::write(
        &path,
        amipro_formats::sam_document(&old, &amipro_formats::SamOptions::at(0)),
    )
    .unwrap();
    s.command_act(CommandId::Merge);
    s.act("Open the data file", {
        let path = path.clone();
        move |rt| {
            rt.send(Message::Merge(A::Chosen(path)));
        }
    });
    let letter = s.app().active_session;
    s.act("New Data File…", |rt| {
        rt.send(Message::Merge(A::NewDataFile));
    });
    s.act("names", |rt| {
        rt.send(Message::Merge(A::NewDataNames("Name, City".into())));
    });
    s.act("the chooser returns the file (Replace)", move |rt| {
        rt.send(Message::Merge(A::NewDataFileChosen(path)));
    });
    go_to_tab(&mut s, letter);
}
