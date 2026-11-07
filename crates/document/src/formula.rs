//! Table formulas as Ami Pro 3.1 has them (run C, TD.4;
//! `docs/research/TABLE_DATA_NOTES.md` sections 1 and 10). A pure module:
//! the lexer and compiler (a shunting-yard to postfix, as Ami Pro's), the
//! evaluator on doubles, the recalculation of a whole table in dependency
//! order with REF, divide-by-zero, and loop detection, and the rewriting of a
//! formula's text when rows or columns are inserted or deleted or a formula
//! is copied.
//!
//! It lives in the document crate because the editing, format, and layout
//! crates all depend on that one and nothing here needs more than a table's
//! size and cell values handed in.
//!
//! Ami Pro's rules:
//! - a leading `=` is skipped; `+ - * /` with `* /` above `+ -`, left to
//!   right; one unary sign at the start, after an operator, or after `(`;
//!   no implicit multiplication;
//! - `N%` is `N/100`; `A1%` pops the stacked `* /` and divides by 100, so
//!   `B1*A1%` is `(B1*A1)/100`;
//! - numbers are digits with at most one decimal mark: no exponent, no
//!   thousands separators;
//! - cell names (case-insensitive): optional `$`, one or two letters,
//!   optional `$`, a row number, all inside the table; the word `REF` is
//!   accepted wherever a name is and makes the result REF;
//! - one function, `sum` (an optional `@` before it, case-insensitive) of
//!   two cell names in one row or one column separated by `:` or dots; any
//!   other function is "cannot understand" (3125). `avg`, `min`, `max`, and
//!   `count` are native-only extras, accepted when asked for and flagged.

use std::collections::BTreeSet;

/// Columns run A to IV: 256.
pub const MAX_COLUMNS: usize = 256;

/// The size of the table a formula lives in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dims {
    pub columns: usize,
    pub rows: usize,
}

impl Dims {
    pub fn index(self, column: usize, row: usize) -> usize {
        row * self.columns + column
    }
}

/// A cell address, zero-based, with the `$` marks it was written with.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CellRef {
    pub column: usize,
    pub row: usize,
    pub column_absolute: bool,
    pub row_absolute: bool,
}

impl CellRef {
    pub fn new(column: usize, row: usize) -> Self {
        Self {
            column,
            row,
            column_absolute: false,
            row_absolute: false,
        }
    }

    /// The canonical spelling: upper-case letters, `$` where it was.
    pub fn name(&self) -> String {
        format!(
            "{}{}{}{}",
            if self.column_absolute { "$" } else { "" },
            column_name(self.column),
            if self.row_absolute { "$" } else { "" },
            self.row + 1
        )
    }
}

/// A column's letters: A to Z, then AA onward.
pub fn column_name(column: usize) -> String {
    if column < 26 {
        ((b'A' + column as u8) as char).to_string()
    } else {
        let high = column / 26 - 1;
        let low = column % 26;
        format!(
            "{}{}",
            (b'A' + high as u8) as char,
            (b'A' + low as u8) as char
        )
    }
}

fn column_index(letters: &str) -> Option<usize> {
    let bytes = letters.as_bytes();
    let index = match bytes {
        [one] if one.is_ascii_alphabetic() => usize::from(one.to_ascii_uppercase() - b'A'),
        [high, low] if high.is_ascii_alphabetic() && low.is_ascii_alphabetic() => {
            (usize::from(high.to_ascii_uppercase() - b'A') + 1) * 26
                + usize::from(low.to_ascii_uppercase() - b'A')
        }
        _ => return None,
    };
    (index < MAX_COLUMNS).then_some(index)
}

/// Why a formula is not understood (message 3125).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CannotUnderstand {
    pub reason: &'static str,
}

impl CannotUnderstand {
    /// Ami Pro's message number.
    pub const MESSAGE: u32 = 3125;
}

/// Why a formula has no value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Failure {
    /// 3125.
    Syntax,
    /// 3126.
    DivideByZero,
    /// 3127.
    Loop,
}

impl Failure {
    pub fn message_number(self) -> u32 {
        match self {
            Self::Syntax => 3125,
            Self::DivideByZero => 3126,
            Self::Loop => 3127,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Function {
    Sum,
    /// Native-only extras.
    Average,
    Min,
    Max,
    Count,
}

impl Function {
    pub fn is_extra(self) -> bool {
        self != Self::Sum
    }

    fn from_name(name: &str, extras: bool) -> Option<Self> {
        match name {
            "sum" => Some(Self::Sum),
            "avg" if extras => Some(Self::Average),
            "min" if extras => Some(Self::Min),
            "max" if extras => Some(Self::Max),
            "count" if extras => Some(Self::Count),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Op {
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// One token of the postfix form.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Token {
    Number(f64),
    Cell(CellRef),
    Range(Function, CellRef, CellRef),
    Negate,
    Op(Op),
    /// The word REF: the result is REF.
    Ref,
}

/// A compiled formula.
#[derive(Clone, Debug, PartialEq)]
pub struct Formula {
    pub tokens: Vec<Token>,
    /// Uses `avg`, `min`, `max`, or `count`, which Ami Pro lacks.
    pub uses_extras: bool,
}

/// Options of a compile.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Accept the native-only extra functions.
    pub extras: bool,
    /// The style's decimal symbol when it is not `.`.
    pub decimal: Option<char>,
}

/// What a cell holds as a formula reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CellValue {
    /// No text: counts as 0.
    Empty,
    Number(f64),
    /// Text, REF, or an error: makes the formula REF.
    NotNumber,
}

pub trait CellSource {
    fn cell(&self, column: usize, row: usize) -> CellValue;
}

/// A name found in a formula's text.
#[derive(Clone, Copy, Debug, PartialEq)]
struct NameUse {
    start: usize,
    end: usize,
    cell: CellRef,
    percent: bool,
}

/// What the parser found: tokens, and names with their places for rewriting.
#[derive(Default)]
struct Parsed {
    tokens: Vec<Token>,
    uses_extras: bool,
    /// Single names, and ranges as two names.
    singles: Vec<NameUse>,
    ranges: Vec<(NameUse, NameUse)>,
}

fn err<T>(reason: &'static str) -> Result<T, CannotUnderstand> {
    Err(CannotUnderstand { reason })
}

struct Scanner<'a> {
    text: &'a str,
    position: usize,
    decimal: Option<char>,
}

impl Scanner<'_> {
    fn peek(&self) -> Option<char> {
        self.text[self.position..].chars().next()
    }

    fn is_word(&self, c: char) -> bool {
        c.is_ascii_alphanumeric() || matches!(c, '%' | '$' | '.' | '@') || self.decimal == Some(c)
    }

    fn skip_spaces(&mut self) {
        while self.peek().is_some_and(|c| c == ' ' || c == '\t') {
            self.position += 1;
        }
    }

    /// A word: word characters, with spaces inside it ignored (`119f`).
    /// Returns its extent in the text and its characters without spaces.
    fn word(&mut self) -> (usize, usize, String) {
        let start = self.position;
        let mut word = String::new();
        let mut end = start;
        loop {
            match self.peek() {
                Some(c) if self.is_word(c) => {
                    word.push(c);
                    self.position += c.len_utf8();
                    end = self.position;
                }
                Some(' ' | '\t') => {
                    let mut ahead = self.position;
                    while self.text[ahead..].starts_with([' ', '\t']) {
                        ahead += 1;
                    }
                    match self.text[ahead..].chars().next() {
                        Some(c) if self.is_word(c) => self.position = ahead,
                        _ => break,
                    }
                }
                _ => break,
            }
        }
        self.position = end;
        (start, end, word)
    }
}

fn priority(token: Stacked) -> u8 {
    match token {
        Stacked::Open => 0,
        Stacked::Op(Op::Add | Op::Subtract) => 1,
        Stacked::Op(Op::Multiply | Op::Divide) => 2,
        Stacked::Negate => 3,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Stacked {
    Open,
    Op(Op),
    Negate,
}

fn emit(token: Stacked) -> Token {
    match token {
        Stacked::Op(op) => Token::Op(op),
        Stacked::Negate => Token::Negate,
        Stacked::Open => unreachable!("an open bracket is never emitted"),
    }
}

/// `$`, letters, `$`, digits, an optional `%`, as a name; `bounds` checks it
/// lies in the table.
fn cell_name(word: &str, start: usize, end: usize, allow_percent: bool) -> Option<NameUse> {
    let mut rest = word;
    let mut percent = false;
    if allow_percent && let Some(stripped) = rest.strip_suffix('%') {
        rest = stripped;
        percent = true;
    }
    let (column_absolute, rest) = match rest.strip_prefix('$') {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    let letters = rest.chars().take_while(char::is_ascii_alphabetic).count();
    let (letters_text, rest) = rest.split_at(letters);
    let (row_absolute, digits) = match rest.strip_prefix('$') {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    if digits.is_empty() || digits.len() > 6 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let column = column_index(letters_text)?;
    let row: usize = digits.parse().ok()?;
    if row == 0 {
        return None;
    }
    Some(NameUse {
        start,
        end,
        cell: CellRef {
            column,
            row: row - 1,
            column_absolute,
            row_absolute,
        },
        percent,
    })
}

fn is_ref_word(word: &str) -> bool {
    word.eq_ignore_ascii_case("ref")
}

fn number_value(word: &str, decimal: Option<char>) -> Option<f64> {
    let (digits, percent) = match word.strip_suffix('%') {
        Some(rest) => (rest, true),
        None => (word, false),
    };
    let mut marks = 0;
    let mut count = 0;
    let mut clean = String::new();
    for c in digits.chars() {
        if c.is_ascii_digit() {
            count += 1;
            clean.push(c);
        } else if c == '.' || Some(c) == decimal {
            marks += 1;
            clean.push('.');
        } else {
            return None;
        }
    }
    if marks > 1 || count == 0 {
        return None;
    }
    let value: f64 = clean.parse().ok()?;
    Some(if percent { value / 100.0 } else { value })
}

fn parse(text: &str, dims: Option<Dims>, options: Options) -> Result<Parsed, CannotUnderstand> {
    let mut scanner = Scanner {
        text,
        position: 0,
        decimal: options.decimal.filter(|c| *c != '.'),
    };
    let mut parsed = Parsed::default();
    let mut stack: Vec<Stacked> = Vec::new();
    let mut expect_operand = true;
    let mut signs = 0;
    scanner.skip_spaces();
    if scanner.peek() == Some('=') {
        scanner.position += 1;
    }
    // Ami Pro's quirk (VM TDF1): a sign right after a `(` that opens the
    // formula is lost, so `(-A1)` is +A1 and `(-3)` is 3, while `2*(-A2)`,
    // `-A1` and `-(A1)` are right.
    scanner.skip_spaces();
    let formula_start = scanner.position;
    let mut opens_formula = false;
    let inside = |name: &NameUse| {
        dims.is_none_or(|dims| name.cell.column < dims.columns && name.cell.row < dims.rows)
    };
    loop {
        scanner.skip_spaces();
        let Some(c) = scanner.peek() else { break };
        let sign_is_lost = std::mem::take(&mut opens_formula);
        if expect_operand {
            match c {
                '(' => {
                    opens_formula = scanner.position == formula_start;
                    scanner.position += 1;
                    stack.push(Stacked::Open);
                    signs = 0;
                }
                '-' | '+' => {
                    if signs > 0 {
                        return err("two signs in a row");
                    }
                    signs = 1;
                    scanner.position += 1;
                    if c == '-' && !sign_is_lost {
                        stack.push(Stacked::Negate);
                    }
                }
                c if scanner.is_word(c) => {
                    let (start, end, word) = scanner.word();
                    let lower = word.to_ascii_lowercase();
                    let name = lower.strip_prefix('@').unwrap_or(&lower);
                    let function = {
                        scanner.skip_spaces();
                        if scanner.peek() == Some('(') {
                            Function::from_name(name, options.extras)
                        } else {
                            None
                        }
                    };
                    if let Some(function) = function {
                        scanner.position += 1;
                        parse_range(&mut scanner, function, dims, &mut parsed)?;
                        if function.is_extra() {
                            parsed.uses_extras = true;
                        }
                    } else if word.starts_with(|c: char| c.is_ascii_digit() || c == '.')
                        || (scanner.decimal.is_some() && word.starts_with(scanner.decimal.unwrap()))
                    {
                        let Some(value) = number_value(&word, scanner.decimal) else {
                            return err("not a number");
                        };
                        parsed.tokens.push(Token::Number(value));
                    } else if is_ref_word(&word) {
                        parsed.tokens.push(Token::Ref);
                    } else {
                        let Some(name) = cell_name(&word, start, end, true) else {
                            return err("not a cell name");
                        };
                        if !inside(&name) {
                            return err("a cell outside the table");
                        }
                        parsed.tokens.push(Token::Cell(name.cell));
                        parsed.singles.push(name);
                        if name.percent {
                            // Pop every stacked operator of priority 2 or
                            // more, then divide by 100.
                            while let Some(top) = stack.last().copied() {
                                if top != Stacked::Open && priority(top) >= 2 {
                                    parsed.tokens.push(emit(top));
                                    stack.pop();
                                } else {
                                    break;
                                }
                            }
                            parsed.tokens.push(Token::Number(100.0));
                            parsed.tokens.push(Token::Op(Op::Divide));
                        }
                    }
                    expect_operand = false;
                    signs = 0;
                }
                _ => return err("an operand was expected"),
            }
        } else {
            match c {
                '+' | '-' | '*' | '/' => {
                    scanner.position += 1;
                    let op = match c {
                        '+' => Op::Add,
                        '-' => Op::Subtract,
                        '*' => Op::Multiply,
                        _ => Op::Divide,
                    };
                    let incoming = priority(Stacked::Op(op));
                    while let Some(top) = stack.last().copied() {
                        if top != Stacked::Open && priority(top) >= incoming {
                            parsed.tokens.push(emit(top));
                            stack.pop();
                        } else {
                            break;
                        }
                    }
                    stack.push(Stacked::Op(op));
                    expect_operand = true;
                    signs = 0;
                }
                ')' => {
                    scanner.position += 1;
                    loop {
                        match stack.pop() {
                            Some(Stacked::Open) => break,
                            Some(top) => parsed.tokens.push(emit(top)),
                            None => return err("unbalanced brackets"),
                        }
                    }
                }
                '(' => return err("implicit multiplication"),
                _ => return err("an operator was expected"),
            }
        }
    }
    if expect_operand {
        return err("the formula is incomplete");
    }
    while let Some(top) = stack.pop() {
        if top == Stacked::Open {
            return err("unbalanced brackets");
        }
        parsed.tokens.push(emit(top));
    }
    Ok(parsed)
}

/// The arguments of a function, after its `(`: two names and a separator.
fn parse_range(
    scanner: &mut Scanner,
    function: Function,
    dims: Option<Dims>,
    parsed: &mut Parsed,
) -> Result<(), CannotUnderstand> {
    // The argument up to `)`, with each character's place; spaces ignored.
    let mut chars: Vec<(usize, char)> = Vec::new();
    let mut closed = false;
    while let Some(c) = scanner.peek() {
        let at = scanner.position;
        scanner.position += c.len_utf8();
        match c {
            ')' => {
                closed = true;
                break;
            }
            ' ' | '\t' => {}
            '(' => return err("a bracket in a function"),
            c => chars.push((at, c)),
        }
    }
    if !closed {
        return err("the function is not closed");
    }
    let word: String = chars.iter().map(|(_, c)| *c).collect();
    if is_ref_word(&word) {
        parsed.tokens.push(Token::Ref);
        return Ok(());
    }
    let Some(split) = chars.iter().position(|(_, c)| *c == ':' || *c == '.') else {
        return err("a range needs two names");
    };
    let mut second = split;
    while chars
        .get(second)
        .is_some_and(|(_, c)| *c == ':' || *c == '.')
    {
        second += 1;
    }
    let (left, right) = (&chars[..split], &chars[second..]);
    let text = |part: &[(usize, char)]| -> String { part.iter().map(|(_, c)| *c).collect() };
    let (left_text, right_text) = (text(left), text(right));
    if is_ref_word(&left_text) || is_ref_word(&right_text) {
        parsed.tokens.push(Token::Ref);
        return Ok(());
    }
    let extent = |part: &[(usize, char)]| {
        part.first().map_or((0, 0), |(first, _)| {
            let (last, c) = part[part.len() - 1];
            (*first, last + c.len_utf8())
        })
    };
    let (ls, le) = extent(left);
    let (rs, re) = extent(right);
    let (Some(a), Some(b)) = (
        cell_name(&left_text, ls, le, false),
        cell_name(&right_text, rs, re, false),
    ) else {
        return err("not a cell name in a range");
    };
    if let Some(dims) = dims
        && [a, b]
            .iter()
            .any(|name| name.cell.column >= dims.columns || name.cell.row >= dims.rows)
    {
        return err("a cell outside the table");
    }
    if a.cell.row != b.cell.row && a.cell.column != b.cell.column {
        return err("a range is one row or one column");
    }
    parsed.tokens.push(Token::Range(function, a.cell, b.cell));
    parsed.ranges.push((a, b));
    Ok(())
}

impl Formula {
    /// Compiles the text of a formula for a table of this size.
    pub fn compile(text: &str, dims: Dims, options: Options) -> Result<Self, CannotUnderstand> {
        let parsed = parse(text, Some(dims), options)?;
        Ok(Self {
            tokens: parsed.tokens,
            uses_extras: parsed.uses_extras,
        })
    }

    /// The cells the formula reads, ranges expanded, each once.
    pub fn reads(&self, dims: Dims) -> BTreeSet<usize> {
        let mut cells = BTreeSet::new();
        for token in &self.tokens {
            match token {
                Token::Cell(cell) => {
                    cells.insert(dims.index(cell.column, cell.row));
                }
                Token::Range(_, a, b) => {
                    for (column, row) in range_cells(*a, *b) {
                        cells.insert(dims.index(column, row));
                    }
                }
                _ => {}
            }
        }
        cells
    }

    /// The value on doubles: `Ok(None)` is REF (a cell that is not a number,
    /// or the word REF), an error is a divide by zero.
    pub fn evaluate(&self, source: &dyn CellSource) -> Result<Option<f64>, Failure> {
        let mut stack: Vec<Option<f64>> = Vec::new();
        for token in &self.tokens {
            match *token {
                Token::Number(value) => stack.push(Some(value)),
                Token::Ref => stack.push(None),
                Token::Cell(cell) => stack.push(match source.cell(cell.column, cell.row) {
                    CellValue::Empty => Some(0.0),
                    CellValue::Number(value) => Some(value),
                    CellValue::NotNumber => None,
                }),
                Token::Range(function, a, b) => {
                    stack.push(range_value(function, a, b, source));
                }
                Token::Negate => {
                    let value = stack.pop().ok_or(Failure::Syntax)?;
                    stack.push(value.map(|value| -value));
                }
                Token::Op(op) => {
                    let right = stack.pop().ok_or(Failure::Syntax)?;
                    let left = stack.pop().ok_or(Failure::Syntax)?;
                    let (Some(left), Some(right)) = (left, right) else {
                        stack.push(None);
                        continue;
                    };
                    stack.push(Some(match op {
                        Op::Add => left + right,
                        Op::Subtract => left - right,
                        Op::Multiply => left * right,
                        Op::Divide => {
                            if right == 0.0 {
                                return Err(Failure::DivideByZero);
                            }
                            left / right
                        }
                    }));
                }
            }
        }
        match (stack.pop(), stack.is_empty()) {
            (Some(value), true) => Ok(value.filter(|value| value.is_finite())),
            _ => Err(Failure::Syntax),
        }
    }
}

fn range_cells(a: CellRef, b: CellRef) -> Vec<(usize, usize)> {
    let (c0, c1) = (a.column.min(b.column), a.column.max(b.column));
    let (r0, r1) = (a.row.min(b.row), a.row.max(b.row));
    let mut cells = Vec::new();
    for row in r0..=r1 {
        for column in c0..=c1 {
            cells.push((column, row));
        }
    }
    cells
}

/// `sum` adds along a row or column (empty cells count 0, a cell that is not
/// a number gives REF). The extras skip empty cells: `avg`, `min`, and `max`
/// of no numbers are 0, `count` counts the numbers and never gives REF.
fn range_value(function: Function, a: CellRef, b: CellRef, source: &dyn CellSource) -> Option<f64> {
    let mut numbers: Vec<f64> = Vec::new();
    let mut bad = false;
    for (column, row) in range_cells(a, b) {
        match source.cell(column, row) {
            CellValue::Empty => {
                if function == Function::Sum {
                    numbers.push(0.0);
                }
            }
            CellValue::Number(value) => numbers.push(value),
            CellValue::NotNumber => bad = true,
        }
    }
    if function == Function::Count {
        return Some(numbers.len() as f64);
    }
    if bad {
        return None;
    }
    Some(match function {
        Function::Sum => numbers.iter().sum(),
        Function::Average if numbers.is_empty() => 0.0,
        Function::Average => numbers.iter().sum::<f64>() / numbers.len() as f64,
        Function::Min => numbers
            .iter()
            .copied()
            .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.min(v))))
            .unwrap_or(0.0),
        Function::Max => numbers
            .iter()
            .copied()
            .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |m| m.max(v))))
            .unwrap_or(0.0),
        Function::Count => unreachable!(),
    })
}

/// What a table's cell is, as the calculator sees it.
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Empty,
    Number(f64),
    /// Text that is not a number (also REF).
    Text,
    /// A formula's text; its value is computed.
    Formula(String),
}

/// A formula cell's result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    Number(f64),
    /// REF: it read something that is not a number.
    Ref,
    /// It could not be calculated.
    Failed(Failure),
}

/// Cell record F3 bit 0x2000: the formula's result is not a number (REF).
pub const NOT_A_NUMBER_BIT: i64 = 0x2000;

impl Outcome {
    /// What a formula reading this cell sees (VM TDF1): a divide by zero or a
    /// loop leaves an empty result, which counts as 0; REF, and a formula
    /// that was not understood, are not numbers. (A loop's reader was not
    /// observed: taken as the empty result it stores.)
    fn as_value(self) -> CellValue {
        match self {
            Self::Number(value) => CellValue::Number(value),
            Self::Failed(Failure::DivideByZero | Failure::Loop) => CellValue::Number(0.0),
            _ => CellValue::NotNumber,
        }
    }

    /// The result text Ami Pro stores: the number at the style's decimals;
    /// empty after a divide by zero, a loop, or REF (the screen draws REF
    /// for the last, from F3 bit [`NOT_A_NUMBER_BIT`]).
    pub fn stored_text(self, format: &crate::NumberFormat) -> String {
        match self {
            Self::Number(value) => format.stored_text(value),
            _ => String::new(),
        }
    }

    /// Whether the cell is drawn as REF and stored with F3 bit 0x2000.
    pub fn is_not_a_number(self) -> bool {
        self == Self::Ref
    }
}

/// A table's cells as the calculator sees them, from the stored text: a
/// cell is empty (no text), a number (the style's scanner), or text; a
/// formula cell is its text (the value is calculated).
pub fn table_inputs(document: &crate::Document, table: &crate::Table) -> (Dims, Vec<Input>) {
    let dims = Dims {
        columns: table.columns.len().max(1),
        rows: table.rows.len().max(1),
    };
    let inputs = table
        .cells
        .iter()
        .map(|cell| {
            if let Some(formula) = &cell.formula {
                return Input::Formula(formula.clone());
            }
            let crate::CellContent::Text(flow) = cell.content;
            let Some(flow) = document.flow(flow) else {
                return Input::Empty;
            };
            let text = flow.plain_text();
            if text.is_empty() {
                return Input::Empty;
            }
            let format = flow
                .paragraphs
                .first()
                .and_then(|paragraph| document.styles().get(&paragraph.style))
                .map(|style| style.kept.number_format.clone())
                .unwrap_or_default();
            format.scan(&text).map_or(Input::Text, Input::Number)
        })
        .collect();
    (dims, inputs)
}

struct Grid<'a> {
    dims: Dims,
    inputs: &'a [Input],
    outcomes: &'a [Option<Outcome>],
}

impl CellSource for Grid<'_> {
    fn cell(&self, column: usize, row: usize) -> CellValue {
        let index = self.dims.index(column, row);
        match self.inputs.get(index) {
            None | Some(Input::Empty) => CellValue::Empty,
            Some(Input::Number(value)) => CellValue::Number(*value),
            Some(Input::Text) => CellValue::NotNumber,
            Some(Input::Formula(_)) => self
                .outcomes
                .get(index)
                .copied()
                .flatten()
                .map_or(CellValue::NotNumber, Outcome::as_value),
        }
    }
}

struct Calculation<'a> {
    dims: Dims,
    inputs: &'a [Input],
    formulas: Vec<Option<Result<Formula, CannotUnderstand>>>,
    outcomes: Vec<Option<Outcome>>,
    visiting: Vec<bool>,
    done: Vec<bool>,
    stack: Vec<usize>,
    in_loop: Vec<bool>,
    order: Vec<usize>,
    round: &'a dyn Fn(usize, f64) -> f64,
}

impl Calculation<'_> {
    fn visit(&mut self, index: usize) {
        if self.done[index] {
            return;
        }
        self.visiting[index] = true;
        self.stack.push(index);
        let reads: Vec<usize> = match &self.formulas[index] {
            Some(Ok(formula)) => formula.reads(self.dims).into_iter().collect(),
            _ => Vec::new(),
        };
        for read in reads {
            if !matches!(self.inputs.get(read), Some(Input::Formula(_))) {
                continue;
            }
            if self.visiting[read] {
                // Every cell on the path from `read` up to here is on the loop.
                if let Some(from) = self.stack.iter().position(|cell| *cell == read) {
                    for position in from..self.stack.len() {
                        self.in_loop[self.stack[position]] = true;
                    }
                }
            } else {
                self.visit(read);
            }
        }
        self.stack.pop();
        self.visiting[index] = false;
        self.done[index] = true;
        self.order.push(index);
        let outcome = if self.in_loop[index] {
            Outcome::Failed(Failure::Loop)
        } else {
            match &self.formulas[index] {
                Some(Ok(formula)) => {
                    let grid = Grid {
                        dims: self.dims,
                        inputs: self.inputs,
                        outcomes: &self.outcomes,
                    };
                    match formula.evaluate(&grid) {
                        Ok(Some(value)) => Outcome::Number((self.round)(index, value)),
                        Ok(None) => Outcome::Ref,
                        Err(failure) => Outcome::Failed(failure),
                    }
                }
                _ => Outcome::Failed(Failure::Syntax),
            }
        };
        self.outcomes[index] = Some(outcome);
    }
}

fn prepare<'a>(
    dims: Dims,
    inputs: &'a [Input],
    options: Options,
    round: &'a dyn Fn(usize, f64) -> f64,
) -> Calculation<'a> {
    let formulas = inputs
        .iter()
        .map(|input| match input {
            Input::Formula(text) => Some(Formula::compile(text, dims, options)),
            _ => None,
        })
        .collect();
    Calculation {
        dims,
        inputs,
        formulas,
        outcomes: vec![None; inputs.len()],
        visiting: vec![false; inputs.len()],
        done: vec![false; inputs.len()],
        stack: Vec::new(),
        in_loop: vec![false; inputs.len()],
        order: Vec::new(),
        round,
    }
}

/// Calculates every formula cell of a table; `None` for the other cells.
/// A cell on a loop is `Failed(Loop)`; a cell that reads one is REF.
/// `round` (given the cell's index and the result) is applied to each result before the formulas that read it see
/// it (the identity for Ami Pro's doubles; the stored text's rounding to
/// calculate from what a file holds).
pub fn calculate(
    dims: Dims,
    inputs: &[Input],
    options: Options,
    round: &dyn Fn(usize, f64) -> f64,
) -> Vec<Option<Outcome>> {
    let mut work = prepare(dims, inputs, options, round);
    for (index, input) in inputs.iter().enumerate() {
        if matches!(input, Input::Formula(_)) {
            work.visit(index);
        }
    }
    work.outcomes
}

/// The formula cells that depend, directly or through others, on the changed
/// cells, in the order to calculate them (each after what it reads). A
/// changed cell is included only if it depends on a changed cell too.
pub fn dependants(dims: Dims, inputs: &[Input], changed: &[usize], options: Options) -> Vec<usize> {
    let identity = |_: usize, value: f64| value;
    let mut work = prepare(dims, inputs, options, &identity);
    let reads: Vec<BTreeSet<usize>> = work
        .formulas
        .iter()
        .map(|formula| match formula {
            Some(Ok(formula)) => formula.reads(dims),
            _ => BTreeSet::new(),
        })
        .collect();
    let mut reached: BTreeSet<usize> = changed.iter().copied().collect();
    let mut found: BTreeSet<usize> = BTreeSet::new();
    let mut grew = true;
    while grew {
        grew = false;
        for (index, read) in reads.iter().enumerate() {
            if !found.contains(&index) && read.iter().any(|cell| reached.contains(cell)) {
                found.insert(index);
                reached.insert(index);
                grew = true;
            }
        }
    }
    for index in 0..inputs.len() {
        if found.contains(&index) {
            work.visit(index);
        }
    }
    work.order.retain(|index| found.contains(index));
    work.order
}

/// A change of a table's shape (or a copy) that formulas must follow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Change {
    InsertRows {
        at: usize,
        count: usize,
    },
    InsertColumns {
        at: usize,
        count: usize,
    },
    DeleteRows {
        at: usize,
        count: usize,
    },
    DeleteColumns {
        at: usize,
        count: usize,
    },
    /// A copy or move by this offset: relative parts shift, `$` parts stay.
    Shift {
        rows: isize,
        columns: isize,
    },
}

#[derive(Clone, Copy)]
enum Axis {
    Row,
    Column,
}

fn get(cell: CellRef, axis: Axis) -> usize {
    match axis {
        Axis::Row => cell.row,
        Axis::Column => cell.column,
    }
}

fn put(cell: &mut CellRef, axis: Axis, value: usize) {
    match axis {
        Axis::Row => cell.row = value,
        Axis::Column => cell.column = value,
    }
}

fn insert_one(value: usize, at: usize, count: usize) -> usize {
    if value >= at { value + count } else { value }
}

fn delete_one(value: usize, at: usize, count: usize) -> Option<usize> {
    if value < at {
        Some(value)
    } else if value < at + count {
        None
    } else {
        Some(value - count)
    }
}

/// A range's ends on the deleted axis (`1420:04a8`, notes 10.6; rows confirmed
/// by VM TDF3, columns **unconfirmed**: Insert/Delete Column was refused for
/// lack of room, so the symmetric rule is applied): the low end
/// on the band's first line stays (the range shrinks), the high end inside
/// the band moves to the line before it, a low end inside the band beyond its
/// first line is REF.
fn delete_range(low: usize, high: usize, at: usize, count: usize) -> Option<(usize, usize)> {
    if low == high {
        let value = delete_one(low, at, count)?;
        return Some((value, value));
    }
    let new_low = if low <= at {
        low
    } else if low < at + count {
        return None;
    } else {
        low - count
    };
    let new_high = if high < at {
        high
    } else if high < at + count {
        at.checked_sub(1)?
    } else {
        high - count
    };
    (new_high >= new_low).then_some((new_low, new_high))
}

fn shift_part(value: usize, absolute: bool, delta: isize) -> Option<usize> {
    if absolute {
        Some(value)
    } else {
        value.checked_add_signed(delta)
    }
}

fn map_single(cell: CellRef, change: Change) -> Option<CellRef> {
    let mut out = cell;
    match change {
        Change::InsertRows { at, count } => out.row = insert_one(cell.row, at, count),
        Change::InsertColumns { at, count } => out.column = insert_one(cell.column, at, count),
        Change::DeleteRows { at, count } => out.row = delete_one(cell.row, at, count)?,
        Change::DeleteColumns { at, count } => out.column = delete_one(cell.column, at, count)?,
        Change::Shift { rows, columns } => {
            out.row = shift_part(cell.row, cell.row_absolute, rows)?;
            out.column = shift_part(cell.column, cell.column_absolute, columns)?;
        }
    }
    Some(out)
}

fn map_range(a: CellRef, b: CellRef, change: Change) -> Option<(CellRef, CellRef)> {
    let deleted = match change {
        Change::DeleteRows { at, count } => Some((Axis::Row, at, count)),
        Change::DeleteColumns { at, count } => Some((Axis::Column, at, count)),
        _ => None,
    };
    let Some((axis, at, count)) = deleted else {
        return Some((map_single(a, change)?, map_single(b, change)?));
    };
    let (mut a, mut b) = (a, b);
    let (low, high) = (
        get(a, axis).min(get(b, axis)),
        get(a, axis).max(get(b, axis)),
    );
    let (new_low, new_high) = delete_range(low, high, at, count)?;
    let a_is_low = get(a, axis) <= get(b, axis);
    put(&mut a, axis, if a_is_low { new_low } else { new_high });
    put(&mut b, axis, if a_is_low { new_high } else { new_low });
    Some((a, b))
}

/// The text of a formula after a change of the table (`dims` is the table's
/// size after it). `None` when the text stays as it is: it names nothing the
/// change moves, or it is not a formula Ami Pro would read. A name that
/// leaves the table or lies in deleted cells becomes the word REF, which
/// makes the formula REF; names in a rewritten formula are written in the
/// canonical form (upper-case letters, `$` kept) and the rest of its text
/// is kept.
pub fn rewrite(text: &str, change: Change, dims: Dims, options: Options) -> Option<String> {
    let parsed = parse(text, None, options).ok()?;
    // (start, end, replacement)
    let mut edits: Vec<(usize, usize, String)> = Vec::new();
    let mut touched = false;
    for name in &parsed.singles {
        let mapped = map_single(name.cell, change)
            .filter(|cell| cell.column < dims.columns && cell.row < dims.rows);
        match mapped {
            Some(cell) => {
                touched |= cell != name.cell;
                let mut spelled = cell.name();
                if name.percent {
                    spelled.push('%');
                }
                edits.push((name.start, name.end, spelled));
            }
            None => {
                touched = true;
                edits.push((name.start, name.end, "REF".to_owned()));
            }
        }
    }
    for (a, b) in &parsed.ranges {
        let mapped = map_range(a.cell, b.cell, change).filter(|(x, y)| {
            [x, y]
                .iter()
                .all(|cell| cell.column < dims.columns && cell.row < dims.rows)
        });
        match mapped {
            Some((x, y)) => {
                touched |= x != a.cell || y != b.cell;
                edits.push((a.start, a.end, x.name()));
                edits.push((b.start, b.end, y.name()));
            }
            None => {
                touched = true;
                edits.push((a.start, b.end, "REF".to_owned()));
            }
        }
    }
    if !touched {
        return None;
    }
    edits.sort_by_key(|edit| edit.0);
    let mut out = String::new();
    let mut at = 0;
    for (start, end, replacement) in edits {
        out.push_str(&text[at..start]);
        out.push_str(&replacement);
        at = end;
    }
    out.push_str(&text[at..]);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIMS: Dims = Dims {
        columns: 6,
        rows: 10,
    };

    struct Cells(Vec<(usize, usize, CellValue)>);

    impl CellSource for Cells {
        fn cell(&self, column: usize, row: usize) -> CellValue {
            self.0
                .iter()
                .find(|(c, r, _)| (*c, *r) == (column, row))
                .map_or(CellValue::Empty, |(_, _, value)| *value)
        }
    }

    fn run(text: &str) -> Result<Option<f64>, Failure> {
        run_with(text, &Cells(Vec::new()))
    }

    fn run_with(text: &str, cells: &Cells) -> Result<Option<f64>, Failure> {
        Formula::compile(text, DIMS, Options::default())
            .map_err(|_| Failure::Syntax)?
            .evaluate(cells)
    }

    fn numbers(values: &[(&str, f64)]) -> Cells {
        Cells(
            values
                .iter()
                .map(|(name, value)| {
                    let cell = cell_name(name, 0, name.len(), false).unwrap().cell;
                    (cell.column, cell.row, CellValue::Number(*value))
                })
                .collect(),
        )
    }

    #[test]
    fn precedence_and_association() {
        assert_eq!(run("2+3*4"), Ok(Some(14.0)));
        assert_eq!(run("(2+3)*4"), Ok(Some(20.0)));
        assert_eq!(run("10-4-3"), Ok(Some(3.0)));
        assert_eq!(run("100/10/5"), Ok(Some(2.0)));
        assert_eq!(run("2*3/4"), Ok(Some(1.5)));
        assert_eq!(run("2 + 3 * 4"), Ok(Some(14.0)), "spaces between words");
        assert_eq!(
            run("1 000"),
            Ok(Some(1000.0)),
            "a space inside a word is ignored"
        );
    }

    #[test]
    fn prefix_equals_is_skipped() {
        assert_eq!(run("=2+3"), Ok(Some(5.0)));
        assert_eq!(run(" =2+3"), Ok(Some(5.0)));
        assert!(run("==2").is_err());
        assert!(run("@2+3").is_err(), "@ is only part of a function name");
    }

    #[test]
    fn unary_minus_one_sign_only() {
        assert_eq!(run("-3"), Ok(Some(-3.0)));
        assert_eq!(run("+3"), Ok(Some(3.0)));
        assert_eq!(run("-3*2"), Ok(Some(-6.0)));
        assert_eq!(run("2*-3"), Ok(Some(-6.0)));
        assert_eq!(run("2--3"), Ok(Some(5.0)));
        assert_eq!(run("-(3)"), Ok(Some(-3.0)));
        assert_eq!(run("2*(-3+1)"), Ok(Some(-4.0)));
        assert_eq!(run("-(2+3)"), Ok(Some(-5.0)));
        assert_eq!(run("-2*-3"), Ok(Some(6.0)));
        assert_eq!(run("--3"), Err(Failure::Syntax));
        assert_eq!(run("3*-+2"), Err(Failure::Syntax));
        assert_eq!(run("-+3"), Err(Failure::Syntax));
    }

    #[test]
    fn a_sign_after_the_opening_bracket_of_the_formula_is_lost() {
        // Ami Pro's behaviour (VM TDF1), reproduced so that results equal its own.
        let cells = numbers(&[("A1", 10.0), ("A2", 4.0)]);
        assert_eq!(run("(-3)"), Ok(Some(3.0)));
        assert_eq!(run_with("(-A1)", &cells), Ok(Some(10.0)));
        assert_eq!(run_with("= ( -A1)", &cells), Ok(Some(10.0)));
        assert_eq!(run_with("(-A1)*2", &cells), Ok(Some(20.0)));
        // Everything else keeps its sign.
        assert_eq!(run_with("2*(-A2)", &cells), Ok(Some(-8.0)));
        assert_eq!(run_with("-A1", &cells), Ok(Some(-10.0)));
        assert_eq!(run_with("-(A1)", &cells), Ok(Some(-10.0)));
        assert_eq!(run_with("0-(-A2)", &cells), Ok(Some(4.0)));
        assert_eq!(run_with("3*-A2", &cells), Ok(Some(-12.0)));
        assert_eq!(run_with("(2+(-A2))", &cells), Ok(Some(-2.0)));
        assert_eq!(
            run_with("((-A1))", &cells),
            Ok(Some(-10.0)),
            "only the bracket that opens the formula"
        );
        assert_eq!(run("(--3)"), Err(Failure::Syntax));
    }

    #[test]
    fn no_implicit_multiplication_and_no_junk() {
        for text in [
            "2(3)", "(2)(3)", "(2)3", "A1(B1)", "", "   ", "=", "2+", "*2", "(2", "2)", "()",
            "2 3x", "1e5", "1,5", "1.2.3", "5%%", "(2)%", "$5", "2^3", "A1 B1",
        ] {
            assert_eq!(run(text), Err(Failure::Syntax), "{text:?}");
        }
    }

    #[test]
    fn percent_on_numbers_and_cells() {
        assert_eq!(run("25%"), Ok(Some(0.25)));
        assert_eq!(run("200*25%"), Ok(Some(50.0)));
        let cells = numbers(&[("A1", 50.0), ("B1", 3.0)]);
        assert_eq!(run_with("A1%", &cells), Ok(Some(0.5)));
        assert_eq!(run_with("B1*A1%", &cells), Ok(Some(1.5)), "(B1*A1)/100");
        assert_eq!(run_with("10+A1%", &cells), Ok(Some(10.5)));
        assert_eq!(run_with("10+B1*A1%", &cells), Ok(Some(11.5)));
        assert_eq!(run_with("-A1%", &cells), Ok(Some(-0.5)));
        assert_eq!(run_with("2*A1%*10", &cells), Ok(Some(10.0)));
        assert_eq!(run_with("a1 %", &cells), Ok(Some(0.5)));
    }

    #[test]
    fn numbers_have_no_exponent_or_thousands() {
        assert_eq!(run(".5+.5"), Ok(Some(1.0)));
        assert_eq!(run("5.+1"), Ok(Some(6.0)));
        let comma = Options {
            decimal: Some(','),
            ..Options::default()
        };
        let formula = Formula::compile("1,5*2", DIMS, comma).unwrap();
        assert_eq!(formula.evaluate(&Cells(vec![])), Ok(Some(3.0)));
    }

    #[test]
    fn cell_names() {
        let cells = numbers(&[("A1", 2.0), ("C4", 5.0), ("F10", 7.0)]);
        assert_eq!(run_with("a1*c4", &cells), Ok(Some(10.0)));
        assert_eq!(
            run_with("$A$1+C$4+$F10", &cells),
            Ok(Some(14.0)),
            "$ per part"
        );
        assert_eq!(run_with("=F10", &cells), Ok(Some(7.0)));
        // Outside the table: not understood.
        for text in ["G1", "A11", "A0", "AA1", "A1234567", "AAA1", "A", "1A"] {
            assert_eq!(run(text), Err(Failure::Syntax), "{text:?}");
        }
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(255), "IV");
        assert_eq!(column_index("IV"), Some(255));
        assert_eq!(column_index("IW"), None);
        assert_eq!(column_index("ab"), Some(27));
    }

    #[test]
    fn empty_cells_are_zero_and_text_makes_ref() {
        assert_eq!(run("A1+5"), Ok(Some(5.0)));
        let text = Cells(vec![(0, 0, CellValue::NotNumber)]);
        assert_eq!(run_with("A1+5", &text), Ok(None));
        assert_eq!(run_with("-A1", &text), Ok(None));
        assert_eq!(run("REF"), Ok(None));
        assert_eq!(run("ref+1"), Ok(None));
        assert_eq!(run("2*(REF)"), Ok(None));
        assert_eq!(run("sum(REF)"), Ok(None));
        assert_eq!(run("@sum(REF..A3)"), Ok(None));
        assert_eq!(
            run_with("1/0+A1", &text),
            Err(Failure::DivideByZero),
            "the zero comes first"
        );
    }

    #[test]
    fn sum_ranges() {
        let cells = numbers(&[
            ("A1", 1.0),
            ("A2", 2.0),
            ("A3", 3.0),
            ("B1", 10.0),
            ("C1", 20.0),
        ]);
        for text in [
            "@sum(A1..A3)",
            "@SUM(a1:a3)",
            "sum(A1.A3)",
            "Sum (A1 .. A3)",
            "=@sum(A1...A3)",
            "@sum(A1::A3)",
            "@sum($A$1:$A$3)",
            "@sum(A3..A1)",
        ] {
            assert_eq!(run_with(text, &cells), Ok(Some(6.0)), "{text:?}");
        }
        assert_eq!(run_with("@sum(A1..C1)", &cells), Ok(Some(31.0)), "a row");
        assert_eq!(run_with("@sum(A1..A3)*2+1", &cells), Ok(Some(13.0)));
        assert_eq!(
            run_with("(@sum(A1..A3)+@sum(B1..C1))/2", &cells),
            Ok(Some(18.0))
        );
        assert_eq!(
            run_with("@sum(A1..A5)", &cells),
            Ok(Some(6.0)),
            "empty cells are 0"
        );
        let text = Cells(vec![(0, 1, CellValue::NotNumber)]);
        assert_eq!(
            run_with("@sum(A1..A3)", &text),
            Ok(None),
            "a text cell gives REF"
        );
        assert_eq!(run_with("sum(A1:A1)", &cells), Ok(Some(1.0)));
        for bad in [
            "sum(A1)",
            "sum(A1..B2)",
            "sum(A1..A3",
            "sum()",
            "sum(A1,A2)",
            "sum(A1..A3)5",
            "sum(1..3)",
            "sum(G1..G2)",
            "sum A1..A3",
            "sum(A1..A(3))",
            "sum(sum(A1..A2)..A3)",
        ] {
            assert_eq!(run_with(bad, &cells), Err(Failure::Syntax), "{bad:?}");
        }
    }

    #[test]
    fn other_functions_are_not_understood_unless_extras() {
        for text in [
            "avg(A1..A3)",
            "@max(A1..A3)",
            "@min(A1..A3)",
            "count(A1..A3)",
            "round(A1)",
            "if(A1)",
        ] {
            assert!(
                Formula::compile(text, DIMS, Options::default()).is_err(),
                "{text:?}"
            );
        }
        let extras = Options {
            extras: true,
            ..Options::default()
        };
        let cells = numbers(&[("A1", 1.0), ("A2", 2.0), ("A3", 6.0)]);
        let check = |text: &str, want: f64| {
            let formula = Formula::compile(text, DIMS, extras).unwrap();
            assert!(formula.uses_extras, "{text}");
            assert_eq!(formula.evaluate(&cells), Ok(Some(want)), "{text}");
        };
        check("@avg(A1..A3)", 3.0);
        check("@AVG(A1:A4)", 3.0);
        check("@min(A1..A3)", 1.0);
        check("@max(A1..A3)", 6.0);
        check("@count(A1..A5)", 3.0);
        check("@avg(B1..B3)", 0.0);
        assert!(
            !Formula::compile("@sum(A1..A3)", DIMS, extras)
                .unwrap()
                .uses_extras
        );
        assert!(!Formula::compile("1+2", DIMS, extras).unwrap().uses_extras);
        assert!(Formula::compile("round(A1)", DIMS, extras).is_err());
    }

    #[test]
    fn division_by_zero() {
        assert_eq!(run("8/0"), Err(Failure::DivideByZero));
        assert_eq!(run("8/(2-2)"), Err(Failure::DivideByZero));
        assert_eq!(
            run("8/A1"),
            Err(Failure::DivideByZero),
            "an empty cell is 0"
        );
        assert_eq!(run("0/8"), Ok(Some(0.0)));
        assert_eq!(Failure::DivideByZero.message_number(), 3126);
        assert_eq!(Failure::Loop.message_number(), 3127);
        assert_eq!(Failure::Syntax.message_number(), CannotUnderstand::MESSAGE);
    }

    fn grid(cells: &[(&str, &str)]) -> Vec<Input> {
        let mut inputs = vec![Input::Empty; DIMS.columns * DIMS.rows];
        for (name, value) in cells {
            let cell = cell_name(name, 0, name.len(), false).unwrap().cell;
            inputs[DIMS.index(cell.column, cell.row)] = match *value {
                "text" => Input::Text,
                formula
                    if formula.starts_with(['=', '@', '(', '-'])
                        || formula.contains(['+', '*', '/', '(']) =>
                {
                    Input::Formula(formula.to_owned())
                }
                number => Input::Number(number.parse().unwrap()),
            };
        }
        inputs
    }

    fn at(name: &str) -> usize {
        let cell = cell_name(name, 0, name.len(), false).unwrap().cell;
        DIMS.index(cell.column, cell.row)
    }

    #[test]
    fn calculation_follows_dependencies_in_any_order() {
        // C1 reads B1 which reads A1; written backwards on purpose.
        let inputs = grid(&[
            ("A1", "2"),
            ("B1", "=A1*3"),
            ("C1", "=B1+A1"),
            ("D1", "=@sum(A1..C1)"),
        ]);
        let out = calculate(DIMS, &inputs, Options::default(), &|_, v| v);
        assert_eq!(out[at("B1")], Some(Outcome::Number(6.0)));
        assert_eq!(out[at("C1")], Some(Outcome::Number(8.0)));
        assert_eq!(out[at("D1")], Some(Outcome::Number(16.0)));
        assert_eq!(out[at("A1")], None);
        // A formula before its data: an empty cell is 0.
        let inputs = grid(&[("B1", "=A1*3")]);
        let out = calculate(DIMS, &inputs, Options::default(), &|_, v| v);
        assert_eq!(out[at("B1")], Some(Outcome::Number(0.0)));
    }

    #[test]
    fn dependants_come_in_calculation_order() {
        let inputs = grid(&[
            ("A1", "2"),
            ("D1", "=@sum(A1..C1)"),
            ("C1", "=B1+A1"),
            ("B1", "=A1*3"),
            ("A2", "=A1+0"),
            ("E1", "=A2*1"),
            ("F1", "=1+1"),
        ]);
        let order = dependants(DIMS, &inputs, &[at("A1")], Options::default());
        let position = |name: &str| order.iter().position(|i| *i == at(name)).unwrap();
        assert!(position("B1") < position("C1") && position("C1") < position("D1"));
        assert!(position("A2") < position("E1"));
        assert!(!order.contains(&at("F1")), "F1 reads nothing that changed");
        assert_eq!(order.len(), 5);
        assert!(dependants(DIMS, &inputs, &[at("F1")], Options::default()).is_empty());
    }

    #[test]
    fn errors_loops_and_ref_spread_to_readers() {
        let inputs = grid(&[
            ("A1", "=B1+1"),
            ("B1", "=A1+1"),
            ("C1", "=A1+5"),
            ("D1", "=D1+1"),
            ("A2", "text"),
            ("B2", "=A2+1"),
            ("C2", "=B2*2"),
            ("A3", "=1/0"),
            ("B3", "=A3+1"),
            ("A4", "=1+"),
            ("B4", "=A4+1"),
            ("A5", "=5"),
        ]);
        let out = calculate(DIMS, &inputs, Options::default(), &|_, v| v);
        assert_eq!(out[at("A1")], Some(Outcome::Failed(Failure::Loop)));
        assert_eq!(out[at("B1")], Some(Outcome::Failed(Failure::Loop)));
        assert_eq!(
            out[at("C1")],
            Some(Outcome::Number(5.0)),
            "a loop leaves an empty result: 0"
        );
        assert_eq!(
            out[at("D1")],
            Some(Outcome::Failed(Failure::Loop)),
            "itself"
        );
        assert_eq!(out[at("B2")], Some(Outcome::Ref));
        assert_eq!(out[at("C2")], Some(Outcome::Ref));
        assert_eq!(out[at("A3")], Some(Outcome::Failed(Failure::DivideByZero)));
        assert_eq!(
            out[at("B3")],
            Some(Outcome::Number(1.0)),
            "a divide by zero counts as 0"
        );
        assert_eq!(out[at("A4")], Some(Outcome::Failed(Failure::Syntax)));
        assert_eq!(out[at("B4")], Some(Outcome::Ref));
        assert_eq!(out[at("A5")], Some(Outcome::Number(5.0)));
    }

    #[test]
    fn a_long_cycle_marks_only_its_members() {
        // D1 reads the loop A1-B1-C1 from outside; E1 is above it.
        let inputs = grid(&[
            ("E1", "=D1*2"),
            ("D1", "=A1+0"),
            ("A1", "=B1+0"),
            ("B1", "=C1+0"),
            ("C1", "=A1+0"),
        ]);
        let out = calculate(DIMS, &inputs, Options::default(), &|_, v| v);
        for name in ["A1", "B1", "C1"] {
            assert_eq!(
                out[at(name)],
                Some(Outcome::Failed(Failure::Loop)),
                "{name}"
            );
        }
        assert_eq!(out[at("D1")], Some(Outcome::Number(0.0)));
        assert_eq!(out[at("E1")], Some(Outcome::Number(0.0)));
    }

    #[test]
    fn rounding_applies_before_readers() {
        let inputs = grid(&[("A1", "1"), ("B1", "=A1/3"), ("C1", "=B1*3")]);
        let exact = calculate(DIMS, &inputs, Options::default(), &|_, v| v);
        let Some(Outcome::Number(c)) = exact[at("C1")] else {
            panic!()
        };
        assert!((c - 1.0).abs() < 1e-12);
        let two = |_: usize, v: f64| (v * 100.0).round() / 100.0;
        let rounded = calculate(DIMS, &inputs, Options::default(), &two);
        assert_eq!(rounded[at("C1")], Some(Outcome::Number(0.99)));
    }

    #[test]
    fn stored_forms_of_the_outcomes() {
        let format = crate::NumberFormat::default();
        assert_eq!(Outcome::Number(3.0).stored_text(&format), "3.00");
        for empty in [
            Outcome::Ref,
            Outcome::Failed(Failure::DivideByZero),
            Outcome::Failed(Failure::Loop),
        ] {
            assert_eq!(empty.stored_text(&format), "");
        }
        assert!(Outcome::Ref.is_not_a_number());
        assert!(!Outcome::Failed(Failure::DivideByZero).is_not_a_number());
        assert!(!Outcome::Failed(Failure::Loop).is_not_a_number());
    }

    #[test]
    fn row_rewrites_as_ami_pro_did_them() {
        // VM TDF3: A1..A6 = 1..6, delete the row of A3.
        let delete = Change::DeleteRows { at: 2, count: 1 };
        assert_eq!(rw("@sum(A2..A5)", delete).as_deref(), Some("@sum(A2..A4)"));
        assert_eq!(rw("@sum(A1..A3)", delete).as_deref(), Some("@sum(A1..A2)"));
        assert_eq!(rw("@sum(A3..A6)", delete).as_deref(), Some("@sum(A3..A5)"));
        assert_eq!(rw("A3*2", delete).as_deref(), Some("REF*2"));
        assert_eq!(rw("A2+A4", delete).as_deref(), Some("A2+A3"));
        assert_eq!(rw("A$1+$A2", delete), None);
        // Insert after row 3: a range spanning the point stretches.
        let insert = Change::InsertRows { at: 3, count: 1 };
        assert_eq!(rw("@sum(A2..A4)", insert).as_deref(), Some("@sum(A2..A5)"));
        assert_eq!(rw("@sum(A3..A5)", insert).as_deref(), Some("@sum(A3..A6)"));
        assert_eq!(rw("A2*1+A$1", insert), None);
        // Copy and paste.
        assert_eq!(
            rw(
                "A$1+$A2",
                Change::Shift {
                    rows: 2,
                    columns: 0
                }
            )
            .as_deref(),
            Some("A$1+$A4")
        );
        assert_eq!(
            rw(
                "@sum(A1..A2)",
                Change::Shift {
                    rows: 1,
                    columns: 1
                }
            )
            .as_deref(),
            Some("@sum(B2..B3)")
        );
    }

    fn rw(text: &str, change: Change) -> Option<String> {
        rewrite(text, change, DIMS, Options::default())
    }

    #[test]
    fn insert_moves_names_at_or_past_the_place() {
        let rows = Change::InsertRows { at: 2, count: 1 };
        assert_eq!(rw("A1+A2", rows), None, "nothing at or past row 3");
        assert_eq!(rw("A1+A3", rows).as_deref(), Some("A1+A4"));
        assert_eq!(rw("a3*2", rows).as_deref(), Some("A4*2"));
        assert_eq!(
            rw("$A$3+B$3+$C3", rows).as_deref(),
            Some("$A$4+B$4+$C4"),
            "$ does not stop an insert"
        );
        assert_eq!(
            rw("@sum(A1..A5)", rows).as_deref(),
            Some("@sum(A1..A6)"),
            "an insert inside a range stretches it"
        );
        assert_eq!(rw("@sum(A3..A5)", rows).as_deref(), Some("@sum(A4..A6)"));
        assert_eq!(rw("@sum(A1..A2)", rows), None);
        assert_eq!(rw("@sum(A1:C1)", rows), None);
        let columns = Change::InsertColumns { at: 1, count: 2 };
        assert_eq!(rw("A1+B1+C1", columns).as_deref(), Some("A1+D1+E1"));
        assert_eq!(rw("@sum(A1:C1)", columns).as_deref(), Some("@sum(A1:E1)"));
        assert_eq!(rw("25%+B2%", columns).as_deref(), Some("25%+D2%"));
        assert_eq!(rw("not a formula (", columns), None);
    }

    #[test]
    fn delete_makes_ref_of_deleted_cells() {
        let rows = Change::DeleteRows { at: 2, count: 1 };
        assert_eq!(rw("A1+A2", rows), None);
        assert_eq!(rw("A1+A4", rows).as_deref(), Some("A1+A3"));
        assert_eq!(rw("A1+A3", rows).as_deref(), Some("A1+REF"));
        assert_eq!(rw("A3%", rows).as_deref(), Some("REF"));
        // Ranges: the high end in the band moves to the line before it.
        assert_eq!(rw("@sum(A1..A3)", rows).as_deref(), Some("@sum(A1..A2)"));
        assert_eq!(rw("@sum(A1..A5)", rows).as_deref(), Some("@sum(A1..A4)"));
        assert_eq!(
            rw("@sum(A3..A5)", rows).as_deref(),
            Some("@sum(A3..A4)"),
            "a low end on the band's first line stays"
        );
        assert_eq!(rw("@sum(A3..A3)", rows).as_deref(), Some("@sum(REF)"));
        assert_eq!(rw("@sum(A4..A6)", rows).as_deref(), Some("@sum(A3..A5)"));
        assert_eq!(
            rw("@sum(C3:E3)", rows).as_deref(),
            Some("@sum(REF)"),
            "a row range on the deleted row"
        );
        assert_eq!(rw("@sum(C1:E1)", rows), None);
        let two = Change::DeleteRows { at: 2, count: 2 };
        assert_eq!(rw("@sum(A2..A5)", two).as_deref(), Some("@sum(A2..A3)"));
        assert_eq!(
            rw("@sum(A4..A6)", two).as_deref(),
            Some("@sum(REF)"),
            "a low end strictly inside the band"
        );
        let columns = Change::DeleteColumns { at: 1, count: 1 };
        assert_eq!(rw("A1+B1+C1", columns).as_deref(), Some("A1+REF+B1"));
        assert_eq!(rw("@sum(A1:D1)", columns).as_deref(), Some("@sum(A1:C1)"));
        assert_eq!(
            rw("@sum(D1..A1)", columns).as_deref(),
            Some("@sum(C1..A1)"),
            "end order is kept"
        );
        // The rewritten formula is REF when read.
        let rewritten = rw("A1+A3", rows).unwrap();
        assert_eq!(run(&rewritten), Ok(None));
    }

    #[test]
    fn copy_shifts_relative_parts_only() {
        let right = Change::Shift {
            rows: 0,
            columns: 1,
        };
        assert_eq!(rw("A1*2", right).as_deref(), Some("B1*2"));
        assert_eq!(rw("$A1*B$2+$C$3", right).as_deref(), Some("$A1*C$2+$C$3"));
        assert_eq!(rw("$A$1+2", right), None, "all absolute: unchanged");
        let down = Change::Shift {
            rows: 2,
            columns: 0,
        };
        assert_eq!(rw("@sum(A1..A3)", down).as_deref(), Some("@sum(A3..A5)"));
        assert_eq!(
            rw("@sum($A1..A$3)", down).as_deref(),
            Some("@sum($A3..A$3)")
        );
        assert_eq!(rw("C1%", down).as_deref(), Some("C3%"));
        // Off the table: REF.
        let left = Change::Shift {
            rows: 0,
            columns: -1,
        };
        assert_eq!(rw("A1+B1", left).as_deref(), Some("REF+A1"));
        assert_eq!(rw("@sum(A1:C1)", left), Some("@sum(REF)".into()));
        let far = Change::Shift {
            rows: 20,
            columns: 0,
        };
        assert_eq!(rw("A1", far).as_deref(), Some("REF"));
        let up = Change::Shift {
            rows: -1,
            columns: 0,
        };
        assert_eq!(rw("A1+A5", up).as_deref(), Some("REF+A4"));
    }

    #[test]
    fn rewriting_keeps_the_other_text() {
        let rows = Change::InsertRows { at: 0, count: 1 };
        assert_eq!(
            rw("= a1 + ( b2 *3 ) / @SUM( c1 : c4 )", rows).as_deref(),
            Some("= A2 + ( B3 *3 ) / @SUM( C2 : C5 )")
        );
    }
}
