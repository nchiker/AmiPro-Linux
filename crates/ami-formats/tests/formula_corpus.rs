//! The formula audit (run C, TD.4): every calculated cell of the corpora is
//! recomputed with the formula engine and compared with the result Ami Pro
//! stored, per class. Aggregate numbers only; `cargo test -p amipro-formats
//! --test formula_corpus -- --ignored --nocapture` (see
//! `docs/TEST_COMMANDS.md`). Skips what is absent: the user's corpus
//! (`AMIPRO_REFERENCE_DIR` or `../AmiPro-Reference`, `Charlie-Sample/SAM`),
//! the ISO corpus (`/tmp/amipro-phase1-corpus`), and the VM folder
//! (`vm-lab/OUT`). `AMIPRO_STYLES` names the style sheets;
//! `AMIPRO_FORMULA_DETAIL=1` lists the VM folder's cells that are not equal.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use amipro_document::formula::{
    CellSource, CellValue, Failure, Formula, NOT_A_NUMBER_BIT, Options, Outcome, Token, calculate,
    table_inputs,
};
use amipro_document::{CellContent, Document, FrameContent, NumberFormat, Table};
use amipro_formats::import_sam_path_with_style_directories;

fn visit(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
        {
            files.push(path);
        }
    }
}

fn cell_text(document: &Document, table: &Table, index: usize) -> String {
    let CellContent::Text(flow) = table.cells[index].content;
    document
        .flow(flow)
        .map(|flow| flow.plain_text())
        .unwrap_or_default()
}

fn cell_format(document: &Document, table: &Table, index: usize) -> NumberFormat {
    let CellContent::Text(flow) = table.cells[index].content;
    document
        .flow(flow)
        .and_then(|flow| flow.paragraphs.first())
        .and_then(|paragraph| document.styles().get(&paragraph.style))
        .map(|style| style.kept.number_format.clone())
        .unwrap_or_default()
}

/// F3 of the cell record as read.
fn f3(table: &Table, index: usize) -> i64 {
    table.cells[index]
        .legacy
        .get("cell.record")
        .and_then(|record| record.split_whitespace().nth(2))
        .and_then(|f3| f3.parse().ok())
        .unwrap_or(0)
}

/// Every cell at its stored value, formula cells too; a cell stored as REF
/// (F3 bit 0x2000) is not a number.
struct Stored<'a> {
    document: &'a Document,
    table: &'a Table,
}

impl CellSource for Stored<'_> {
    fn cell(&self, column: usize, row: usize) -> CellValue {
        let index = row * self.table.columns.len() + column;
        if index >= self.table.cells.len() {
            return CellValue::Empty;
        }
        if f3(self.table, index) & NOT_A_NUMBER_BIT != 0 {
            return CellValue::NotNumber;
        }
        let text = cell_text(self.document, self.table, index);
        if text.is_empty() {
            return CellValue::Empty;
        }
        match cell_format(self.document, self.table, index).scan(&text) {
            Some(value) => CellValue::Number(value),
            None => CellValue::NotNumber,
        }
    }
}

#[derive(Default)]
struct Tally {
    files: usize,
    files_with_formulas: usize,
    formulas: usize,
    unread: usize,
    /// (mode, class) -> cells.
    classes: BTreeMap<(&'static str, String), usize>,
    detail: bool,
}

fn name(outcome: Option<Outcome>) -> &'static str {
    match outcome {
        Some(Outcome::Number(_)) => "number",
        Some(Outcome::Ref) => "REF",
        Some(Outcome::Failed(Failure::DivideByZero)) => "divide by zero",
        Some(Outcome::Failed(Failure::Loop)) => "loop",
        Some(Outcome::Failed(Failure::Syntax)) | None => "not understood",
    }
}

fn classify(
    outcome: Option<Outcome>,
    stored: &str,
    format: &NumberFormat,
    ref_word: bool,
    bit: bool,
) -> String {
    let expected = outcome.map_or(String::new(), |outcome| outcome.stored_text(format));
    let kind = name(outcome);
    let bit_agrees = bit == matches!(outcome, Some(Outcome::Ref));
    if expected == stored && bit_agrees {
        format!("equal, {kind}")
    } else if expected == stored {
        format!("{kind}: text equal, bit 0x2000 differs")
    } else if matches!(outcome, Some(Outcome::Ref)) && ref_word && bit && !stored.is_empty() {
        "REF word: result left as a number, bit 0x2000 set".to_owned()
    } else if let (Some(a), Some(Outcome::Number(b))) = (format.scan(stored), outcome)
        && (a - b).abs() <= 0.5 * 10f64.powi(-(format.stored_decimals() as i32)) + 1e-9
    {
        "rounding only".to_owned()
    } else {
        format!("other ({kind} against a stored result)")
    }
}

fn audit(document: &Document, tally: &mut Tally) -> usize {
    let mut found = 0;
    for frame in document.frames() {
        let FrameContent::Table(table) = &frame.content else {
            continue;
        };
        for (index, cell) in table.cells.iter().enumerate() {
            if cell.formula.is_none() && f3(table, index) & 0x40 != 0 {
                tally.unread += 1;
            }
        }
        if table.cells.iter().all(|cell| cell.formula.is_none()) {
            continue;
        }
        let (dims, inputs) = table_inputs(document, table);
        let options = Options::default();
        let source = Stored { document, table };
        let chain = calculate(dims, &inputs, options, &|_, value| value);
        for (index, cell) in table.cells.iter().enumerate() {
            let Some(text) = &cell.formula else { continue };
            found += 1;
            tally.formulas += 1;
            let stored = cell_text(document, table, index);
            let format = cell_format(document, table, index);
            let bit = f3(table, index) & NOT_A_NUMBER_BIT != 0;
            let Ok(formula) = Formula::compile(text, dims, options) else {
                *tally
                    .classes
                    .entry(("alone", "not understood".into()))
                    .or_default() += 1;
                if tally.detail {
                    println!("    {text:?} stored {stored:?} not understood");
                }
                continue;
            };
            let ref_word = formula.tokens.contains(&Token::Ref);
            let local = match formula.evaluate(&source) {
                Ok(Some(value)) => Some(Outcome::Number(value)),
                Ok(None) => Some(Outcome::Ref),
                Err(failure) => Some(Outcome::Failed(failure)),
            };
            for (mode, outcome) in [("alone", local), ("chain", chain[index])] {
                let class = classify(outcome, &stored, &format, ref_word, bit);
                if tally.detail && mode == "chain" && !class.starts_with("equal") {
                    println!(
                        "    {text:?} stored {stored:?} bit {bit} computed {:?}: {class}",
                        outcome
                    );
                }
                *tally.classes.entry((mode, class)).or_default() += 1;
            }
        }
    }
    found
}

fn run(name: &str, root: &Path, styles: &[PathBuf]) {
    if !root.is_dir() {
        println!("{name}: absent, skipped");
        return;
    }
    let mut files = Vec::new();
    visit(root, &mut files);
    files.sort();
    let mut tally = Tally {
        detail: name == "VM folder" && env::var_os("AMIPRO_FORMULA_DETAIL").is_some(),
        ..Tally::default()
    };
    for path in &files {
        let Ok(import) = import_sam_path_with_style_directories(path, styles) else {
            continue;
        };
        tally.files += 1;
        if audit(&import.document, &mut tally) > 0 {
            tally.files_with_formulas += 1;
        }
    }
    println!(
        "{name}: {} files read, {} with formulas, {} formula cells, {} calculated cells not read as formulas",
        tally.files, tally.files_with_formulas, tally.formulas, tally.unread
    );
    for ((mode, class), count) in &tally.classes {
        println!("  {mode}: {class}: {count}");
    }
}

#[test]
#[ignore = "reads the private corpora; run with --ignored --nocapture"]
fn every_corpus_formula_recomputes_to_ami_pros_stored_result() {
    let reference = env::var_os("AMIPRO_REFERENCE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../AmiPro-Reference")
        });
    let styles: Vec<PathBuf> = env::var_os("AMIPRO_STYLES")
        .map(|paths| env::split_paths(&paths).collect())
        .unwrap_or_default();
    run(
        "user's corpus",
        &reference.join("Charlie-Sample/SAM"),
        &styles,
    );
    run(
        "ISO corpus",
        Path::new("/tmp/amipro-phase1-corpus"),
        &styles,
    );
    run("VM folder", &reference.join("vm-lab/OUT"), &styles);
}
