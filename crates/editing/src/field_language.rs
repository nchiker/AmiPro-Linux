//! Ami Pro's power-field language, which its macro interpreter runs in a
//! field mode (`AMIMACRO.DLL`; `docs/research/FIELDS_NOTES.md`, "The
//! language"; VM fixtures FD3, FD6, FD7): expressions over numbers and
//! text, `IF`, names and `Set` variables, the predefined fields, Ami Pro's
//! text functions, and the `%D`, `%T`, and `%N` formats. Fields that need
//! the macro engine, merge data, or bookmarks keep their result (decision
//! FL1); `Include`, `Query$`, and `Decide` reach the files and the user
//! through a [`FieldHost`].

use std::collections::{BTreeMap, BTreeSet};

use amipro_document::{
    Document, FieldContext, WallClock, count_words_and_characters, format_date, number_in_style,
    power_kind,
};

/// What fields reach outside the document through.
pub trait FieldHost {
    /// The text `Include` brings in: the main text of an Ami Pro document
    /// or a text file (or a bookmark's text in it), as paragraphs; `%P` reads
    /// a text file's line ends as paragraph ends, `%W` reads it from the IBM
    /// PC character set. `None` when it cannot be read.
    fn include(
        &mut self,
        file: &str,
        bookmark: Option<&str>,
        paragraph_ends: bool,
        oem: bool,
    ) -> Option<Vec<String>>;

    /// The answer to `Query$`, once the user has given one; `None` until
    /// then.
    fn query(&mut self, message: &str, default: &str) -> Option<String>;

    /// The answer to `Decide`, once given.
    fn decide(&mut self, message: &str) -> Option<bool>;

    /// A warning Ami Pro shows in a message box (warning 553 for a name
    /// that is not defined).
    fn warn(&mut self, message: String);

    /// The page a bookmark is on (`PageRef`), from the layout; `None`
    /// keeps the field's result.
    fn bookmark_page(&mut self, _name: &str) -> Option<u32> {
        None
    }

    /// The current record's value of merge field `name` during a merge or
    /// the preview (`1398:076f`: names without regard to case); `None`
    /// outside one, or for a name the data file lacks.
    fn merge_value(&mut self, _name: &str) -> Option<String> {
        None
    }

    /// `MergeRec`: the record's number in the merge (records left out by
    /// conditions not counted; skipped ones are); `None` outside a merge
    /// (0, G ch. 32).
    fn merge_record(&mut self) -> Option<u32> {
        None
    }

    /// `NextRec`: the fields after it take the next record.
    fn next_record(&mut self) {}

    /// `Skip`: the record is left out of the merge.
    fn skip_record(&mut self) {}
}

/// A host for fields that need nothing outside the document: `Include`
/// reads nothing, and prompts are never answered.
#[derive(Debug, Default)]
pub struct NoHost {
    pub warnings: Vec<String>,
}

impl FieldHost for NoHost {
    fn include(&mut self, _: &str, _: Option<&str>, _: bool, _: bool) -> Option<Vec<String>> {
        None
    }

    fn query(&mut self, _: &str, _: &str) -> Option<String> {
        None
    }

    fn decide(&mut self, _: &str) -> Option<bool> {
        None
    }

    fn warn(&mut self, message: String) {
        self.warnings.push(message);
    }
}

/// What a field's code is worked out in.
pub struct FieldEnvironment<'a> {
    pub document: &'a Document,
    pub context: &'a FieldContext,
    /// `Set` variables, by name in small letters: Ami Pro's macro globals,
    /// which live while it runs, across documents (VM fixture FD6).
    pub variables: &'a mut BTreeMap<String, Value>,
    /// The document's bookmark names, in small letters: a name that is one
    /// keeps its field's result until bookmarks are built (FL1).
    pub bookmarks: &'a BTreeSet<String>,
    pub host: &'a mut dyn FieldHost,
}

/// A value of the language.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
}

impl Value {
    fn number(&self) -> f64 {
        match self {
            Self::Number(number) => *number,
            // Text reads as the number it starts with ("10" + 5 is 15; FD7).
            Self::Text(text) => leading_number(text),
        }
    }

    fn truth(&self) -> bool {
        match self {
            Self::Number(number) => *number != 0.0,
            Self::Text(text) => !text.is_empty() && leading_number(text) != 0.0,
        }
    }

    /// The value as a field shows it (AMIMACRO `1080:0b9a`).
    pub fn text(&self) -> String {
        match self {
            Self::Number(number) => number_text(*number),
            Self::Text(text) => text.clone(),
        }
    }
}

/// What working out a field gives.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// The new result: its paragraphs (one but for `Include`).
    Result(Vec<String>),
    /// The field shows nothing (`Set`, `Void`, `User`, `Index`, `TOC`,
    /// `PrintEscape`, `NextRec`, `Skip`).
    Nothing,
    /// The field is not run here and keeps its result (macros, bookmarks,
    /// `Message`, `Exec`, `Call`).
    Kept,
    /// It waits for an answer to `Query$` or `Decide`.
    Waiting,
}

/// The kind of field `code` makes, from its first word (AMIMACRO
/// `1080:0060`).
pub fn field_kind(code: &str) -> u8 {
    let first = code
        .trim_start()
        .split(|character: char| !character.is_alphanumeric() && character != '$')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match first.as_str() {
        "seq" if code.split_whitespace().nth(1).is_some() => power_kind::SEQ,
        "set" => power_kind::SET,
        "button" => power_kind::BUTTON,
        "printescape" => power_kind::PRINT_ESCAPE,
        "index" => power_kind::INDEX,
        "user" => power_kind::USER,
        "toc" => power_kind::TOC,
        "mergefield" => power_kind::MERGE_FIELD,
        _ => power_kind::GENERAL,
    }
}

/// Works out a field's code.
pub fn evaluate(code: &str, environment: &mut FieldEnvironment) -> Outcome {
    // Marks and controls that show nothing; what follows their first word
    // is not an expression (`Index "memory" # "extended"`).
    let first = code
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if matches!(
        first.as_str(),
        "user" | "index" | "toc" | "printescape" | "nextrec" | "skip"
    ) {
        match first.as_str() {
            "nextrec" => environment.host.next_record(),
            "skip" => environment.host.skip_record(),
            _ => {}
        }
        return Outcome::Nothing;
    }
    let (expression, format) = split_format(code);
    let Some(tokens) = tokenize(expression) else {
        return syntax_error(code, environment);
    };
    let mut parser = Parser {
        tokens: &tokens,
        at: 0,
        environment,
        waiting: false,
        kept: false,
        nothing: false,
        inert: false,
    };
    let value = parser.statement();
    if parser.waiting {
        return Outcome::Waiting;
    }
    if parser.kept {
        return Outcome::Kept;
    }
    let value = match value {
        Some(value) if parser.at == tokens.len() => value,
        _ => return syntax_error(code, parser.environment),
    };
    if parser.nothing {
        return Outcome::Nothing;
    }
    match value {
        Evaluated::Value(value) => {
            Outcome::Result(vec![formatted(&value, format, parser.environment.context)])
        }
        Evaluated::Paragraphs(paragraphs) => Outcome::Result(paragraphs),
    }
}

fn syntax_error(code: &str, environment: &mut FieldEnvironment) -> Outcome {
    environment
        .host
        .warn(format!("The power field \"{code}\" cannot be worked out."));
    Outcome::Kept
}

/// The expression and the format code after the first `%` outside quotes:
/// the format comes last (the guide, ch. 32).
fn split_format(code: &str) -> (&str, Option<&str>) {
    let mut quoted = false;
    for (index, character) in code.char_indices() {
        match character {
            '"' => quoted = !quoted,
            '%' if !quoted => return (&code[..index], Some(code[index + 1..].trim())),
            _ => {}
        }
    }
    (code, None)
}

/// `value` in `format` (AMIMACRO `1080:1118`): `%Dx` a date, `%Tn` a time,
/// `%Nn` a number style; numbers are seconds in Ami Pro's time base.
fn formatted(value: &Value, format: Option<&str>, context: &FieldContext) -> String {
    let Some(format) = format else {
        return value.text();
    };
    let mut chars = format.chars();
    let kind = chars.next().map(|kind| kind.to_ascii_uppercase());
    let argument = chars.next();
    match (kind, argument) {
        (Some('D'), Some(letter)) => {
            let clock = WallClock::from_ami_pro(value.number().max(0.0) as u32);
            format_date(&clock, letter as u8, &context.settings)
        }
        (Some('T'), Some(digit)) => {
            let clock = WallClock::from_ami_pro(value.number().max(0.0) as u32);
            time_code(&clock, digit.to_digit(10).unwrap_or(0))
        }
        (Some('N'), Some(digit)) => number_in_style(
            value.number().max(0.0) as u32,
            digit.to_digit(10).unwrap_or(1) as u8,
        ),
        _ => value.text(),
    }
}

/// The time in power-field format `%Tn` (AMIMACRO `10a0:0374`; VM fixture
/// FD6): T1 24-hour; T2 to T9 12-hour with AM, A, am, or a, the odd ones
/// with the hour zero-padded; T0 nothing.
pub fn time_code(clock: &WallClock, code: u32) -> String {
    let (morning, afternoon) = match code {
        1 => {
            return format!("{:02}:{:02}", clock.hour, clock.minute);
        }
        2 | 3 => ("AM", "PM"),
        4 | 5 => ("A", "P"),
        6 | 7 => ("am", "pm"),
        8 | 9 => ("a", "p"),
        _ => return String::new(),
    };
    let hour = match clock.hour % 12 {
        0 => 12,
        hour => hour,
    };
    let suffix = if clock.hour < 12 { morning } else { afternoon };
    if code % 2 == 1 {
        format!("{hour:02}:{:02} {suffix}", clock.minute)
    } else {
        format!("{hour}:{:02} {suffix}", clock.minute)
    }
}

/// A number as the language shows it (FD7): a whole number as one; a
/// fraction of up to two places with two (2.50, 0.30); others with sixteen
/// (10 / 3 is 3.3333333333333330), each read to sixteen significant digits.
pub fn number_text(number: f64) -> String {
    if !number.is_finite() {
        return "0".into();
    }
    let rounded: f64 = format!("{number:.15e}").parse().unwrap_or(number);
    if rounded.fract() == 0.0 && rounded.abs() < 1e15 {
        return format!("{}", rounded as i64);
    }
    let hundredths = rounded * 100.0;
    if (hundredths - hundredths.round()).abs() < 1e-6 * hundredths.abs().max(1.0) {
        return format!("{rounded:.2}");
    }
    format!("{rounded:.16}")
}

fn leading_number(text: &str) -> f64 {
    let text = text.trim_start();
    let end = text
        .char_indices()
        .take_while(|(index, character)| {
            character.is_ascii_digit() || *character == '.' || (*index == 0 && *character == '-')
        })
        .count();
    text[..end].parse().unwrap_or(0.0)
}

// ----------------------------------------------------------------- tokens

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Text(String),
    Name(String),
    Symbol(&'static str),
}

fn tokenize(code: &str) -> Option<Vec<Token>> {
    const SYMBOLS: [&str; 15] = [
        "<=", ">=", "!=", "<>", "=", "<", ">", "+", "-", "*", "/", "(", ")", ",", "&",
    ];
    let mut tokens = Vec::new();
    let mut rest = code;
    loop {
        rest = rest.trim_start();
        let Some(first) = rest.chars().next() else {
            return Some(tokens);
        };
        if first == '"' {
            let mut text = String::new();
            let mut chars = rest[1..].char_indices();
            let mut end = None;
            while let Some((index, character)) = chars.next() {
                if character == '"' {
                    if rest[1 + index + 1..].starts_with('"') {
                        text.push('"');
                        chars.next();
                    } else {
                        end = Some(1 + index + 1);
                        break;
                    }
                } else {
                    text.push(character);
                }
            }
            tokens.push(Token::Text(text));
            rest = &rest[end?..];
        } else if first.is_ascii_digit()
            || (first == '.' && rest[1..].starts_with(|c: char| c.is_ascii_digit()))
        {
            let end = rest
                .find(|character: char| !character.is_ascii_digit() && character != '.')
                .unwrap_or(rest.len());
            tokens.push(Token::Number(rest[..end].parse().ok()?));
            rest = &rest[end..];
        } else if first.is_alphabetic() || first == '_' {
            let end = rest
                .find(|character: char| {
                    !character.is_alphanumeric() && character != '_' && character != '$'
                })
                .unwrap_or(rest.len());
            tokens.push(Token::Name(rest[..end].to_owned()));
            rest = &rest[end..];
        } else if first == '|' {
            tokens.push(Token::Symbol("|"));
            rest = &rest[1..];
        } else {
            let symbol = SYMBOLS.iter().find(|symbol| rest.starts_with(**symbol))?;
            tokens.push(Token::Symbol(symbol));
            rest = &rest[symbol.len()..];
        }
    }
}

// ----------------------------------------------------------------- parser

enum Evaluated {
    Value(Value),
    /// `Include`'s paragraphs.
    Paragraphs(Vec<String>),
}

struct Parser<'t, 'e, 'a> {
    tokens: &'t [Token],
    at: usize,
    environment: &'e mut FieldEnvironment<'a>,
    /// A prompt is not answered yet.
    waiting: bool,
    /// Something here is not run (a macro, a bookmark).
    kept: bool,
    /// The field shows nothing.
    nothing: bool,
    /// In an `IF` branch not taken: `NextRec` and `Skip` do nothing.
    inert: bool,
}

fn keyword(token: Option<&Token>, word: &str) -> bool {
    matches!(token, Some(Token::Name(name)) if name.eq_ignore_ascii_case(word))
}

impl Parser<'_, '_, '_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        self.at += 1;
        token
    }

    fn eat_symbol(&mut self, symbol: &str) -> bool {
        if self.peek() == Some(&Token::Symbol(leak(symbol))) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn eat_keyword(&mut self, word: &str) -> bool {
        if keyword(self.peek(), word) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    /// A field's code: a statement keyword, or an expression.
    fn statement(&mut self) -> Option<Evaluated> {
        let Some(Token::Name(first)) = self.peek().cloned() else {
            return self.expression().map(Evaluated::Value);
        };
        match first.to_ascii_lowercase().as_str() {
            "set" => {
                self.at += 1;
                let Some(Token::Name(name)) = self.next() else {
                    return None;
                };
                let value = self.expression()?;
                if !self.waiting && !self.kept {
                    self.environment
                        .variables
                        .insert(name.to_ascii_lowercase(), value);
                }
                self.nothing = true;
                Some(Evaluated::Value(Value::Text(String::new())))
            }
            "void" => {
                self.at += 1;
                self.expression()?;
                self.nothing = true;
                Some(Evaluated::Value(Value::Text(String::new())))
            }
            "include" => {
                self.at += 1;
                let Some(Token::Text(target)) = self.next() else {
                    return None;
                };
                let (file, bookmark) = match target.split_once('!') {
                    Some((file, bookmark)) => (file, Some(bookmark)),
                    None => (target.as_str(), None),
                };
                // `%P` and `%W` were split off as the format.
                let paragraphs = self.environment.host.include(file, bookmark, false, false);
                match paragraphs {
                    Some(paragraphs) => Some(Evaluated::Paragraphs(paragraphs)),
                    None => {
                        self.environment
                            .host
                            .warn(format!("The file {target} cannot be included."));
                        self.kept = true;
                        Some(Evaluated::Value(Value::Text(String::new())))
                    }
                }
            }
            "button" => {
                self.at += 1;
                let Some(Token::Text(text)) = self.next() else {
                    return None;
                };
                // The action runs on a double-click, never here (FL1).
                self.at = self.tokens.len();
                Some(Evaluated::Value(Value::Text(text)))
            }
            _ => self.expression().map(Evaluated::Value),
        }
    }

    fn expression(&mut self) -> Option<Value> {
        self.or()
    }

    fn or(&mut self) -> Option<Value> {
        let mut left = self.and()?;
        loop {
            if self.eat_keyword("or") {
                let right = self.and()?;
                left = boolean(left.truth() || right.truth());
            } else if self.eat_symbol("|") {
                let right = self.and()?;
                left = Value::Number(f64::from((left.number() as i32) | (right.number() as i32)));
            } else {
                return Some(left);
            }
        }
    }

    fn and(&mut self) -> Option<Value> {
        let mut left = self.not()?;
        loop {
            if self.eat_keyword("and") {
                let right = self.not()?;
                left = boolean(left.truth() && right.truth());
            } else if self.eat_symbol("&") {
                let right = self.not()?;
                left = Value::Number(f64::from((left.number() as i32) & (right.number() as i32)));
            } else {
                return Some(left);
            }
        }
    }

    fn not(&mut self) -> Option<Value> {
        if self.eat_keyword("not") {
            let value = self.not()?;
            return Some(boolean(!value.truth()));
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Option<Value> {
        let left = self.sum()?;
        for symbol in ["<=", ">=", "!=", "<>", "=", "<", ">"] {
            if self.eat_symbol(symbol) {
                let right = self.sum()?;
                // A text ordered against a number holds for none: Ami Pro
                // skipped no record by `MergeField AMOUNT < 100` (VM MGM7,
                // MGS4; the merge run, MC1).
                if matches!(symbol, "<=" | ">=" | "<" | ">")
                    && matches!(
                        (&left, &right),
                        (Value::Text(_), Value::Number(_)) | (Value::Number(_), Value::Text(_))
                    )
                {
                    return Some(boolean(false));
                }
                let order = match (&left, &right) {
                    (Value::Text(left), Value::Text(right)) => left.cmp(right),
                    _ => left
                        .number()
                        .partial_cmp(&right.number())
                        .unwrap_or(std::cmp::Ordering::Equal),
                };
                use std::cmp::Ordering::{Equal, Greater, Less};
                return Some(boolean(match symbol {
                    "<=" => order != Greater,
                    ">=" => order != Less,
                    "!=" | "<>" => order != Equal,
                    "=" => order == Equal,
                    "<" => order == Less,
                    _ => order == Greater,
                }));
            }
        }
        Some(left)
    }

    fn sum(&mut self) -> Option<Value> {
        let mut left = self.product()?;
        loop {
            if self.eat_symbol("+") {
                let right = self.product()?;
                left = Value::Number(left.number() + right.number());
            } else if self.eat_symbol("-") {
                let right = self.product()?;
                left = Value::Number(left.number() - right.number());
            } else {
                return Some(left);
            }
        }
    }

    fn product(&mut self) -> Option<Value> {
        let mut left = self.unary()?;
        loop {
            if self.eat_symbol("*") {
                let right = self.unary()?;
                left = Value::Number(left.number() * right.number());
            } else if self.eat_symbol("/") {
                let right = self.unary()?;
                let divisor = right.number();
                left = Value::Number(if divisor == 0.0 {
                    0.0
                } else {
                    left.number() / divisor
                });
            } else {
                return Some(left);
            }
        }
    }

    fn unary(&mut self) -> Option<Value> {
        if self.eat_symbol("-") {
            let value = self.unary()?;
            return Some(Value::Number(-value.number()));
        }
        self.primary()
    }

    fn primary(&mut self) -> Option<Value> {
        match self.next()? {
            Token::Number(number) => Some(Value::Number(number)),
            Token::Text(text) => Some(Value::Text(self.interpolated(&text))),
            Token::Symbol("(") => {
                let value = self.expression()?;
                self.eat_symbol(")").then_some(value)
            }
            Token::Symbol(_) => None,
            Token::Name(name) => self.named(&name),
        }
    }

    /// `{expression}` inside a quoted text is worked out (`1078:0232`).
    fn interpolated(&mut self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(open) = rest.find('{') {
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            out.push_str(&rest[..open]);
            let inner = &rest[open + 1..open + close];
            match evaluate(inner, self.environment) {
                Outcome::Result(paragraphs) => out.push_str(&paragraphs.join(" ")),
                Outcome::Waiting => self.waiting = true,
                Outcome::Kept => self.kept = true,
                Outcome::Nothing => {}
            }
            rest = &rest[open + close + 1..];
        }
        out.push_str(rest);
        out
    }

    fn arguments(&mut self) -> Option<Vec<Value>> {
        if !self.eat_symbol("(") {
            return Some(Vec::new());
        }
        let mut values = Vec::new();
        if self.eat_symbol(")") {
            return Some(values);
        }
        loop {
            values.push(self.expression()?);
            if self.eat_symbol(")") {
                return Some(values);
            }
            if !self.eat_symbol(",") {
                return None;
            }
        }
    }

    fn name_argument(&mut self) -> Option<String> {
        match self.next()? {
            Token::Name(name) | Token::Text(name) => Some(name),
            _ => None,
        }
    }

    fn named(&mut self, name: &str) -> Option<Value> {
        let environment = &mut *self.environment;
        let info = environment.document.info();
        let context = environment.context;
        let seconds = |value: Option<u32>| Value::Number(f64::from(value.unwrap_or(0)));
        Some(match name.to_ascii_lowercase().as_str() {
            "if" => return self.conditional(),
            "createdate" => seconds(info.created.or(Some(context.now.to_ami_pro()))),
            "editdate" => seconds(
                context
                    .saved
                    .or(info.saved)
                    .or(info.created)
                    .or(Some(context.now.to_ami_pro())),
            ),
            "now" => {
                self.arguments()?;
                Value::Number(f64::from(context.now.to_ami_pro()))
            }
            "numedits" => Value::Number(f64::from(info.revisions)),
            "totaleditingtime" => Value::Number(f64::from(
                info.editing_minutes.saturating_add(context.session_minutes),
            )),
            "numpages" => Value::Number(f64::from(if context.pages > 0 {
                context.pages
            } else {
                info.statistics.pages
            })),
            "numwords" => Value::Number(f64::from(
                count_words_and_characters(environment.document).0,
            )),
            "numchars" => Value::Number(f64::from(
                count_words_and_characters(environment.document).1,
            )),
            "filesize" => Value::Number(f64::from(info.statistics.size)),
            "description" => Value::Text(info.description.clone()),
            "mergerec" => {
                Value::Number(f64::from(self.environment.host.merge_record().unwrap_or(0)))
            }
            "mergefield" => {
                let field = self.name_argument()?;
                match self.environment.host.merge_value(&field) {
                    Some(value) => Value::Text(value),
                    // In a merge a name the data file lacks comes out empty
                    // with Ami Pro's message (3098; VM MGM8).
                    None if self.environment.host.merge_record().is_some() => {
                        self.environment.host.warn(format!(
                            "{} is not a valid field name.",
                            field.to_uppercase()
                        ));
                        Value::Text(String::new())
                    }
                    None => {
                        self.environment.host.warn(possibly_undefined(&field));
                        Value::Text(format!("<{field}>"))
                    }
                }
            }
            // `NextRec` and `Skip` inside an expression (`IF … Skip ENDIF`).
            "nextrec" => {
                if !self.inert {
                    self.environment.host.next_record();
                }
                Value::Text(String::new())
            }
            "skip" => {
                if !self.inert {
                    self.environment.host.skip_record();
                }
                Value::Text(String::new())
            }
            "defined" => {
                let name = self.name_argument()?;
                let merge = self.environment.host.merge_value(&name).is_some();
                let name = name.to_ascii_lowercase();
                boolean(
                    merge
                        || self.environment.variables.contains_key(&name)
                        || self.environment.bookmarks.contains(&name),
                )
            }
            // A bookmark's text and page (the notes run, NR6;
            // AMIMACRO `1580:0594`, `1580:10c8`).
            "bookmark" | "getbookmark" => {
                let name = self.name_argument()?;
                match amipro_document::bookmarks::bookmark_named(self.environment.document, &name) {
                    Some(bookmark) => Value::Text(
                        amipro_document::bookmarks::bookmark_text(
                            self.environment.document,
                            &bookmark,
                        )
                        .join(" "),
                    ),
                    None => {
                        self.environment.host.warn(possibly_undefined(&name));
                        self.kept = true;
                        Value::Text(String::new())
                    }
                }
            }
            "pageref" => {
                let name = self.name_argument()?;
                match self.environment.host.bookmark_page(&name) {
                    Some(page) => Value::Number(f64::from(page)),
                    None => {
                        self.kept = true;
                        Value::Text(String::new())
                    }
                }
            }
            // The macro engine is not here (FL1): the field keeps its
            // result.
            "message" | "exec" | "call" => {
                self.kept = true;
                self.at = self.tokens.len();
                Value::Text(String::new())
            }
            "len" => {
                let arguments = self.arguments()?;
                Value::Number(text_argument(&arguments, 0).chars().count() as f64)
            }
            "left$" => {
                let arguments = self.arguments()?;
                let count = number_argument(&arguments, 1).max(0.0) as usize;
                Value::Text(text_argument(&arguments, 0).chars().take(count).collect())
            }
            "right$" => {
                let arguments = self.arguments()?;
                let text = text_argument(&arguments, 0);
                let count = number_argument(&arguments, 1).max(0.0) as usize;
                let skip = text.chars().count().saturating_sub(count);
                Value::Text(text.chars().skip(skip).collect())
            }
            "mid$" => {
                let arguments = self.arguments()?;
                let start = (number_argument(&arguments, 1).max(1.0) as usize) - 1;
                let count = number_argument(&arguments, 2).max(0.0) as usize;
                Value::Text(
                    text_argument(&arguments, 0)
                        .chars()
                        .skip(start)
                        .take(count)
                        .collect(),
                )
            }
            "lcase$" => Value::Text(text_argument(&self.arguments()?, 0).to_lowercase()),
            "ucase$" => Value::Text(text_argument(&self.arguments()?, 0).to_uppercase()),
            "strcat$" => Value::Text(
                self.arguments()?
                    .iter()
                    .map(Value::text)
                    .collect::<String>(),
            ),
            "strfield$" => {
                let arguments = self.arguments()?;
                let index = number_argument(&arguments, 1).max(1.0) as usize;
                let separator = text_argument(&arguments, 2);
                let text = text_argument(&arguments, 0);
                Value::Text(
                    text.split(separator.as_str())
                        .nth(index - 1)
                        .unwrap_or_default()
                        .to_owned(),
                )
            }
            "formatnum$" => {
                let arguments = self.arguments()?;
                let decimals = number_argument(&arguments, 2).clamp(0.0, 15.0) as usize;
                Value::Text(format!(
                    "{}{:.*}{}",
                    text_argument(&arguments, 0),
                    decimals,
                    number_argument(&arguments, 3),
                    text_argument(&arguments, 1)
                ))
            }
            "query$" => {
                let arguments = self.arguments()?;
                match self
                    .environment
                    .host
                    .query(&text_argument(&arguments, 0), &text_argument(&arguments, 1))
                {
                    Some(answer) => Value::Text(answer),
                    None => {
                        self.waiting = true;
                        Value::Text(String::new())
                    }
                }
            }
            "decide" => {
                let arguments = self.arguments()?;
                match self.environment.host.decide(&text_argument(&arguments, 0)) {
                    Some(answer) => boolean(answer),
                    None => {
                        self.waiting = true;
                        boolean(false)
                    }
                }
            }
            lower => {
                // A function the field mode does not know is a macro's.
                if self.peek() == Some(&Token::Symbol("(")) {
                    self.kept = true;
                    self.at = self.tokens.len();
                    return Some(Value::Text(String::new()));
                }
                // A bare name: a bookmark's text, then a global variable,
                // then a merge field (AMIMACRO `1090:02ea`).
                if let Some(value) = self.environment.variables.get(lower) {
                    value.clone()
                } else if self.environment.bookmarks.contains(lower) {
                    match amipro_document::bookmarks::bookmark_named(
                        self.environment.document,
                        lower,
                    ) {
                        Some(bookmark) => Value::Text(
                            amipro_document::bookmarks::bookmark_text(
                                self.environment.document,
                                &bookmark,
                            )
                            .join(" "),
                        ),
                        None => {
                            self.kept = true;
                            Value::Text(String::new())
                        }
                    }
                } else if let Some(value) = self.environment.host.merge_value(name) {
                    Value::Text(value)
                } else {
                    self.environment.host.warn(possibly_undefined(name));
                    Value::Text(format!("<{name}>"))
                }
            }
        })
    }

    /// `IF condition expression [ELSEIF condition expression] [ELSE
    /// expression] ENDIF`.
    fn conditional(&mut self) -> Option<Value> {
        let mut chosen: Option<Value> = None;
        let outer = self.inert;
        loop {
            let condition = self.expression()?;
            let taken = chosen.is_none() && condition.truth();
            self.inert = outer || !taken;
            let value = self.expression();
            self.inert = outer;
            let value = value?;
            if taken {
                chosen = Some(value);
            }
            if self.eat_keyword("elseif") {
                continue;
            }
            if self.eat_keyword("else") {
                self.inert = outer || chosen.is_some();
                let value = self.expression();
                self.inert = outer;
                let value = value?;
                if chosen.is_none() {
                    chosen = Some(value);
                }
            }
            if !self.eat_keyword("endif") {
                return None;
            }
            return Some(chosen.unwrap_or(Value::Text(String::new())));
        }
    }
}

fn leak(symbol: &str) -> &'static str {
    [
        "<=", ">=", "!=", "<>", "=", "<", ">", "+", "-", "*", "/", "(", ")", ",", "&", "|",
    ]
    .into_iter()
    .find(|known| *known == symbol)
    .unwrap_or("")
}

fn boolean(value: bool) -> Value {
    Value::Number(f64::from(u8::from(value)))
}

fn text_argument(arguments: &[Value], index: usize) -> String {
    arguments.get(index).map(Value::text).unwrap_or_default()
}

fn number_argument(arguments: &[Value], index: usize) -> f64 {
    arguments.get(index).map_or(0.0, Value::number)
}

/// Ami Pro's warning 553.
fn possibly_undefined(name: &str) -> String {
    format!("{name} is not a valid field name.")
}

// --------------------------------------------------------------------- Seq

/// The number a Seq field shows after the previous one of its name in its
/// text stream (`1640:0170`; VM fixture FD6): no operator or `+` adds one,
/// `+e` adds e, `-` takes one away, `-e` takes e, `=e` sets e, a bare `=`
/// repeats the previous.
pub fn seq_number(code: &str, previous: u32, environment: &mut FieldEnvironment) -> u32 {
    let (expression, _) = split_format(code);
    let rest = expression
        .trim_start()
        .get(3..)
        .unwrap_or_default()
        .trim_start();
    let name_end = rest
        .find(|character: char| !character.is_alphanumeric() && character != '_')
        .unwrap_or(rest.len());
    let operation: String = rest[name_end..]
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    let amount = |text: &str, environment: &mut FieldEnvironment| -> Option<i64> {
        if text.is_empty() {
            return None;
        }
        match evaluate(text, environment) {
            Outcome::Result(paragraphs) => Some(leading_number(&paragraphs.join("")) as i64),
            _ => Some(0),
        }
    };
    let previous = i64::from(previous);
    let number = if let Some(value) = operation.strip_prefix('=') {
        amount(value, environment).unwrap_or(previous)
    } else if let Some(value) = operation.strip_prefix('-') {
        previous - amount(value, environment).unwrap_or(1)
    } else if let Some(value) = operation.strip_prefix('+') {
        previous + amount(value, environment).unwrap_or(1)
    } else {
        previous + 1
    };
    number.clamp(0, 0x3FFF) as u32
}

/// A Seq field's name, which counts on from the last of the same name.
pub fn seq_name(code: &str) -> String {
    let rest = code.trim_start().get(3..).unwrap_or_default().trim_start();
    rest.chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Answers {
        query: Option<String>,
        decide: Option<bool>,
        included: Option<Vec<String>>,
        warnings: Vec<String>,
    }

    impl FieldHost for Answers {
        fn include(&mut self, _: &str, _: Option<&str>, _: bool, _: bool) -> Option<Vec<String>> {
            self.included.clone()
        }
        fn query(&mut self, _: &str, _: &str) -> Option<String> {
            self.query.clone()
        }
        fn decide(&mut self, _: &str) -> Option<bool> {
            self.decide
        }
        fn warn(&mut self, message: String) {
            self.warnings.push(message);
        }
    }

    fn document() -> Document {
        let mut document = Document::from_plain_text("One two three");
        let info = document.info_mut();
        *info = amipro_document::DocumentInfo::new(1_792_520_631);
        info.revisions = 1;
        info.editing_minutes = 1;
        info.description = "Quarterly".into();
        document
    }

    fn context() -> FieldContext {
        FieldContext {
            now: WallClock {
                year: 2026,
                month: 10,
                day: 20,
                hour: 11,
                minute: 34,
                second: 0,
            },
            pages: 1,
            ..FieldContext::default()
        }
    }

    fn results(codes: &[&str]) -> Vec<String> {
        let document = document();
        let context = context();
        let mut variables = BTreeMap::new();
        let bookmarks = BTreeSet::from(["totalsales".to_owned()]);
        let mut host = NoHost::default();
        codes
            .iter()
            .map(|code| {
                let mut environment = FieldEnvironment {
                    document: &document,
                    context: &context,
                    variables: &mut variables,
                    bookmarks: &bookmarks,
                    host: &mut host,
                };
                match evaluate(code, &mut environment) {
                    Outcome::Result(paragraphs) => paragraphs.join("\n"),
                    Outcome::Nothing => "(nothing)".into(),
                    Outcome::Kept => "(kept)".into(),
                    Outcome::Waiting => "(waiting)".into(),
                }
            })
            .collect()
    }

    #[test]
    fn fd3s_fields_give_ami_pros_results() {
        // VM fixture FD3, as Ami Pro saved them (FileSize 0 unsaved, the
        // statistics counted live).
        assert_eq!(
            results(&[
                "NumPages",
                "CreateDate %Db",
                "EditDate %Dd",
                "Now() %Da",
                "Now() %T2",
                "FileSize",
                "NumEdits",
                "TotalEditingTime",
                "Set total 3",
                "total",
                "total * 2 + 1",
                "IF total > 2 \"big\" Else \"small\" EndIf",
                "\"literal text\"",
                "UCase$(\"abc\")",
                "FormatNum$(\"$\",\"\",2,235.8)",
                "MergeRec",
                "Description",
                "CreateDate",
                "Index \"memory\" # \"extended\"",
                "TOC 1 \"Printer\"",
                "undefinedvar",
            ]),
            [
                "1",
                "October 20, 2026",
                "Tuesday, October 20, 2026",
                "10/20/26",
                "11:34 AM",
                "0",
                "1",
                "1",
                "(nothing)",
                "3",
                "7",
                "big",
                "literal text",
                "ABC",
                "$235.80",
                "0",
                "Quarterly",
                "1792520631",
                "(nothing)",
                "(nothing)",
                "<undefinedvar>",
            ]
        );
        assert_eq!(results(&["NumWords", "NumChars"]), ["3", "13"]);
    }

    #[test]
    fn fd6_and_fd7s_expressions_give_ami_pros_results() {
        assert_eq!(
            results(&[
                "Now() %T0",
                "Now() %T1",
                "Now() %T3",
                "Now() %T4",
                "Now() %T6",
                "Now() %T9",
                "\"x{NumPages}y\"",
                "3 / 2",
                "10 > 2 AND 1 = 1",
                "\"a\" & \"b\"",
                "4 / 2",
                "10 / 3",
                "2.5",
                "1 + 1.5",
                "\"a\" + \"b\"",
                "5 - 7",
                "-3",
                "\"10\" + 5",
                "Len(\"abc\")",
                "Left$(\"abcdef\",2)",
                "Right$(\"123456789\",5)",
                "Mid$(\"(555) 010-0011\",2,3)",
                "StrField$(\"one,two,three\",2,\",\")",
                "LCase$(\"ABC\")",
                "StrCat$(\"a\",\"b\",\"c\")",
                "3 = 3",
                "3 != 3",
                "NOT 1",
                "1 OR 0",
                "5 | 2",
                "\"x\" = \"x\"",
                "Defined nothing",
                "IF 0 \"a\" ELSEIF 1 \"b\" ELSE \"c\" ENDIF",
                "Set s \"text\"",
                "s",
                "\"{3+4}\"",
                "1000000 * 1000",
                "7 * 0.5",
                "0.1 + 0.2",
                "FormatNum$(\"\",\"\",0,2.5)",
            ]),
            [
                "",
                "11:34",
                "11:34 AM",
                "11:34 A",
                "11:34 am",
                "11:34 a",
                "x1y",
                "1.50",
                "1",
                "0",
                "2",
                "3.3333333333333330",
                "2.50",
                "2.50",
                "0",
                "-2",
                "-3",
                "15",
                "3",
                "ab",
                "56789",
                "555",
                "two",
                "abc",
                "abc",
                "1",
                "0",
                "0",
                "1",
                "7",
                "1",
                "0",
                "b",
                "(nothing)",
                "text",
                "7",
                "1000000000",
                "3.50",
                "0.30",
                "2",
            ]
        );
    }

    #[test]
    fn fields_that_need_more_keep_their_result_or_wait() {
        assert_eq!(
            results(&[
                "Message(\"Hello\")",
                "Exec(\"clock.exe\",\"\",2)",
                "getpageno()",
                "totalsales",
                "PageRef Chapter5",
                "Defined totalsales",
                "MergeField ADDRESS",
                "Button \"HELP\" Message(\"Refer to Chapter 3.\")",
                "User TOA,1,People",
                "Query$(\"Name?\")",
                "Include \"a.sam\"",
                "IF 1 \"unclosed\"",
            ]),
            [
                "(kept)",
                "(kept)",
                "(kept)",
                "(kept)",
                "(kept)",
                "1",
                "<ADDRESS>",
                "HELP",
                "(nothing)",
                "(waiting)",
                "(kept)",
                "(kept)",
            ]
        );
        // With answers: the prompt's answer, Decide, an included file.
        let document = document();
        let context = context();
        let mut variables = BTreeMap::new();
        let bookmarks = BTreeSet::new();
        let mut host = Answers {
            query: Some("Ann".into()),
            decide: Some(true),
            included: Some(vec!["First".into(), "Second".into()]),
            warnings: Vec::new(),
        };
        let mut environment = FieldEnvironment {
            document: &document,
            context: &context,
            variables: &mut variables,
            bookmarks: &bookmarks,
            host: &mut host,
        };
        assert_eq!(
            evaluate("Query$(\"Name?\", \"Bob\")", &mut environment),
            Outcome::Result(vec!["Ann".into()])
        );
        assert_eq!(
            evaluate(
                "IF Decide(\"Use creation date?\") CreateDate ELSE EditDate ENDIF %Db",
                &mut environment
            ),
            Outcome::Result(vec!["October 20, 2026".into()])
        );
        assert_eq!(
            evaluate("Include \"standard.sam!Greeting\"", &mut environment),
            Outcome::Result(vec!["First".into(), "Second".into()])
        );
        assert_eq!(
            evaluate("Set name Query$(\"Who?\")", &mut environment),
            Outcome::Nothing
        );
        assert_eq!(
            environment.variables.get("name"),
            Some(&Value::Text("Ann".into()))
        );
        assert!(host.warnings.is_empty());
    }

    #[test]
    fn seq_counts_as_fd6_shows() {
        let document = document();
        let context = context();
        let mut variables = BTreeMap::new();
        let bookmarks = BTreeSet::new();
        let mut host = NoHost::default();
        let mut environment = FieldEnvironment {
            document: &document,
            context: &context,
            variables: &mut variables,
            bookmarks: &bookmarks,
            host: &mut host,
        };
        let mut number = 0;
        let mut shown = Vec::new();
        for code in [
            "Seq A",
            "Seq A",
            "Seq A=10",
            "Seq A",
            "Seq A-",
            "Seq A=",
            "Seq A+3 %N2",
            "Seq A %N4",
            "Seq A %N5",
            "Seq A %N3",
        ] {
            number = seq_number(code, number, &mut environment);
            shown.push(number_in_style(number, amipro_document::number_style(code)));
        }
        assert_eq!(
            shown,
            ["1", "2", "10", "11", "10", "10", "XIII", "N", "o", "xvi"]
        );
        assert_eq!(seq_name("Seq Figure+2 %N2"), "figure");
        assert_eq!(seq_number("Seq Table=5", 0, &mut environment), 5);
    }

    #[test]
    fn kinds_follow_the_first_word() {
        assert_eq!(field_kind("Seq Figure"), power_kind::SEQ);
        assert_eq!(field_kind("Seq"), power_kind::GENERAL);
        assert_eq!(field_kind("set total 3"), power_kind::SET);
        assert_eq!(field_kind("Index \"a\""), power_kind::INDEX);
        assert_eq!(field_kind("TOC 1 \"x\""), power_kind::TOC);
        assert_eq!(field_kind("MergeField NAME"), power_kind::MERGE_FIELD);
        assert_eq!(field_kind("NumPages"), power_kind::GENERAL);
    }
}
