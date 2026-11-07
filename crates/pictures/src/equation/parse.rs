//! The parser of Ami Pro's TeX dialect (`docs/research/OBJECTS_NOTES.md`
//! §11.6.1): plain TeX tokens, AMS-style `\root`, `\matrix`, `\Sb`,
//! `\overset`, big operators with `\limits`. Text outside `$` is text mode.

use super::symbols::{self, Class};
use super::{EquationError, MAX_DEPTH, MAX_MATRIX};

/// A font switch inside a group (`{\BF x}`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontKind {
    Bold,
    Italic,
    Roman,
    Slanted,
    SmallCaps,
    Typewriter,
    Calligraphic,
    Fraktur,
    Blackboard,
    Negative,
}

impl FontKind {
    pub fn from_word(word: &str) -> Option<Self> {
        Some(match word {
            "BF" => Self::Bold,
            "IT" => Self::Italic,
            "RM" => Self::Roman,
            "SL" => Self::Slanted,
            "SMC" => Self::SmallCaps,
            "TT" => Self::Typewriter,
            "CAL" => Self::Calligraphic,
            "FRAK" => Self::Fraktur,
            "BBB" => Self::Blackboard,
            "NEG" => Self::Negative,
            _ => return None,
        })
    }

    pub fn word(self) -> &'static str {
        match self {
            Self::Bold => "BF",
            Self::Italic => "IT",
            Self::Roman => "RM",
            Self::Slanted => "SL",
            Self::SmallCaps => "SMC",
            Self::Typewriter => "TT",
            Self::Calligraphic => "CAL",
            Self::Fraktur => "FRAK",
            Self::Blackboard => "BBB",
            Self::Negative => "NEG",
        }
    }
}

/// What a stack or matrix is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridKind {
    Matrix,
    /// `\Sb ... \endSb`: lines stacked under an operator.
    Sub,
    /// `\Sp ... \endSp`: lines stacked over one.
    Sup,
}

/// A node of the equation tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// A character in math mode; its class says how it is spaced.
    Ch(char),
    /// A symbol named by a control word, as written.
    Sym(String),
    /// Text outside `$` (or in `\text`).
    Text(String),
    /// A font switch: applies to the rest of its group.
    Font(FontKind),
    Space(String),
    Group(Vec<Node>),
    /// A run of math between `$` signs (top level only).
    Math(Vec<Node>),
    Frac {
        word: String,
        num: Vec<Node>,
        den: Vec<Node>,
    },
    Sqrt(Vec<Node>),
    Root {
        index: Vec<Node>,
        body: Vec<Node>,
    },
    Scripts {
        base: Box<Node>,
        sub: Option<Vec<Node>>,
        sup: Option<Vec<Node>>,
    },
    BigOp {
        word: String,
        limits: Option<bool>,
        sub: Option<Vec<Node>>,
        sup: Option<Vec<Node>>,
    },
    Func {
        name: String,
        /// `\func{name}` for a name Ami Pro has no word for.
        unnamed: bool,
    },
    Grid {
        kind: GridKind,
        format: Option<String>,
        rows: Vec<Vec<Vec<Node>>>,
    },
    Delimited {
        left: String,
        right: String,
        body: Vec<Node>,
    },
    /// `\overline{x}`, `\hat{x}`, `\underbrace{y}` ...
    Decor {
        word: String,
        arg: Vec<Node>,
    },
    /// `\overset{top}\to{base}` and `\underset`.
    Stack {
        under: bool,
        note: Vec<Node>,
        base: Vec<Node>,
    },
}

impl Node {
    /// Whether this is a single atom: an argument of one needs no braces.
    pub fn is_atom(&self) -> bool {
        matches!(self, Self::Ch(_) | Self::Sym(_))
    }
}

/// The result of parsing: the segments of text and math.
pub type Tree = Vec<Node>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Stop {
    Eof,
    Dollar,
    Brace,
    Right,
    Amp,
    Row,
    EndMatrix,
    EndSb,
    EndSp,
}

pub struct Parser<'a> {
    src: &'a str,
    pos: usize,
    depth: usize,
    /// Count of nodes made, bounded so that no input makes a huge tree.
    nodes: usize,
}

const MAX_NODES: usize = 20_000;

/// Parses the stored text of an equation (`$` toggles math).
pub fn parse(src: &str) -> Result<Tree, EquationError> {
    let mut parser = Parser {
        src,
        pos: 0,
        depth: 0,
        nodes: 0,
    };
    parser.top(false)
}

/// Parses text as Ami Pro's Import does: with no `$` in it, a construct
/// (a control word that builds math, or a script) is a math run of its own
/// and the plain letters around it stay text.
pub fn parse_import(src: &str) -> Result<Tree, EquationError> {
    let smart = !src.contains('$');
    let mut parser = Parser {
        src,
        pos: 0,
        depth: 0,
        nodes: 0,
    };
    parser.top(smart)
}

/// Parses the whole text as one math expression.
pub fn parse_math(src: &str) -> Result<Tree, EquationError> {
    let mut parser = Parser {
        src,
        pos: 0,
        depth: 0,
        nodes: 0,
    };
    let (nodes, stop) = parser.seq(&[Stop::Eof])?;
    if stop != Stop::Eof {
        return Err(parser.error(5106, "Error parsing TeX"));
    }
    Ok(vec![Node::Math(nodes)])
}

/// Control words that begin a construct (and so a math run when the text
/// has no `$`).
fn starts_math(word: &str) -> bool {
    matches!(
        word,
        "frac"
            | "tfrac"
            | "dfrac"
            | "binom"
            | "tbinom"
            | "dbinom"
            | "sqrt"
            | "root"
            | "matrix"
            | "left"
            | "overset"
            | "underset"
            | "text"
            | "func"
            | "limfunc"
            | "over"
            | "atop"
            | "choose"
    ) || symbols::big_operator(word).is_some()
        || symbols::DECORATIONS.contains(&word)
        || symbols::ACCENTS.iter().any(|accent| accent.0 == word)
        || symbols::FUNCTIONS.contains(&word)
        || symbols::symbol(word).is_some()
}

impl Parser<'_> {
    fn error(&self, code: u16, message: &str) -> EquationError {
        EquationError {
            code,
            message: message.to_string(),
            position: self.pos.min(self.src.len()),
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    fn make(&mut self) -> Result<(), EquationError> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(self.error(5100, "The equation structure is too complex"));
        }
        Ok(())
    }

    /// The control word or symbol after a backslash at `self.pos`.
    fn control(&mut self) -> Option<String> {
        let first = self.peek()?;
        if first.is_ascii_alphabetic() {
            let start = self.pos;
            while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                self.pos += 1;
            }
            Some(self.src[start..self.pos].to_string())
        } else {
            self.bump();
            Some(first.to_string())
        }
    }

    fn peek_control(&self) -> Option<String> {
        let rest = self.src[self.pos..].strip_prefix('\\')?;
        let first = rest.chars().next()?;
        if first.is_ascii_alphabetic() {
            Some(
                rest.chars()
                    .take_while(char::is_ascii_alphabetic)
                    .collect::<String>(),
            )
        } else {
            Some(first.to_string())
        }
    }

    /// Top level: text mode with `$` toggling math.
    fn top(&mut self, smart: bool) -> Result<Tree, EquationError> {
        let mut out: Tree = Vec::new();
        let mut text = String::new();
        let flush = |text: &mut String, out: &mut Tree| {
            if !text.is_empty() {
                out.push(Node::Text(std::mem::take(text)));
            }
        };
        while let Some(c) = self.peek() {
            match c {
                '$' => {
                    self.bump();
                    // `$$` is display math: read as one run.
                    let display = self.peek() == Some('$');
                    if display {
                        self.bump();
                    }
                    flush(&mut text, &mut out);
                    let (nodes, stop) = self.seq(&[Stop::Dollar])?;
                    if stop != Stop::Dollar {
                        return Err(self.error(5106, "Error parsing TeX: a $ is missing"));
                    }
                    if display && self.peek() == Some('$') {
                        self.bump();
                    }
                    self.make()?;
                    out.push(Node::Math(nodes));
                }
                '\\' => {
                    let at = self.pos;
                    self.bump();
                    let Some(word) = self.control() else {
                        return Err(self.error(5106, "Error parsing TeX"));
                    };
                    if let Some(kind) = FontKind::from_word(&word) {
                        flush(&mut text, &mut out);
                        out.push(Node::Font(kind));
                        // A control word swallows the spaces after it.
                        self.skip_space();
                    } else if smart && starts_math(&word) {
                        flush(&mut text, &mut out);
                        self.pos = at;
                        let nodes = self.smart_atom()?;
                        out.push(Node::Math(nodes));
                    } else if word == "\\" {
                        text.push('\n');
                    } else if let Some((ch, _)) = symbols::symbol(&word) {
                        text.push(ch);
                    } else if symbols::space_width(&word).is_some() {
                        flush(&mut text, &mut out);
                        out.push(Node::Space(word));
                    } else {
                        return Err(EquationError {
                            code: 5106,
                            message: format!("Error parsing TeX: unknown command \\{word}"),
                            position: at,
                        });
                    }
                }
                '^' | '_' if smart => {
                    flush(&mut text, &mut out);
                    let mut nodes: Vec<Node> = Vec::new();
                    self.scripts_into(&mut nodes)?;
                    self.scripts_after(&mut nodes)?;
                    out.push(Node::Math(nodes));
                }
                '{' if smart => {
                    // A group of plain text: its braces are dropped.
                    self.bump();
                    self.depth += 1;
                    if self.depth > MAX_DEPTH {
                        return Err(self.error(5100, "The equation structure is too complex"));
                    }
                    let inner = self.top_group(smart)?;
                    self.depth -= 1;
                    flush(&mut text, &mut out);
                    out.extend(inner);
                }
                '}' if smart => {
                    return Err(self.error(5106, "Error parsing TeX: unmatched }"));
                }
                _ => {
                    self.bump();
                    if c == '\r' {
                        continue;
                    }
                    text.push(if c == '\n' { ' ' } else { c });
                }
            }
        }
        flush(&mut text, &mut out);
        Ok(out)
    }

    /// The text of a `{...}` group in smart import, up to its `}`.
    fn top_group(&mut self, smart: bool) -> Result<Tree, EquationError> {
        let mut out: Tree = Vec::new();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Err(self.error(5106, "Error parsing TeX: } is missing")),
                Some('}') => {
                    self.bump();
                    break;
                }
                Some('\\') => {
                    let at = self.pos;
                    self.bump();
                    let word = self
                        .control()
                        .ok_or_else(|| self.error(5106, "Error parsing TeX"))?;
                    if !text.is_empty() {
                        out.push(Node::Text(std::mem::take(&mut text)));
                    }
                    if let Some(kind) = FontKind::from_word(&word) {
                        out.push(Node::Font(kind));
                    } else if starts_math(&word) {
                        self.pos = at;
                        let nodes = self.smart_atom()?;
                        out.push(Node::Math(nodes));
                    } else {
                        return Err(EquationError {
                            code: 5106,
                            message: format!("Error parsing TeX: unknown command \\{word}"),
                            position: at,
                        });
                    }
                }
                Some(c) => {
                    self.bump();
                    text.push(c);
                }
            }
        }
        if !text.is_empty() {
            out.push(Node::Text(text));
        }
        let _ = smart;
        Ok(out)
    }

    /// One construct of a smart import, with the scripts after it.
    fn smart_atom(&mut self) -> Result<Vec<Node>, EquationError> {
        let mut out: Vec<Node> = Vec::new();
        let (_, _) = self.atom(&mut out)?;
        self.scripts_after(&mut out)?;
        Ok(out)
    }

    fn scripts_after(&mut self, out: &mut Vec<Node>) -> Result<(), EquationError> {
        loop {
            // Limits words and scripts belong to the operator before them.
            let mark = self.pos;
            self.skip_space();
            match self.peek() {
                Some('^' | '_' | '\'') => self.scripts_into(out)?,
                Some('\\') => {
                    if matches!(self.peek_control().as_deref(), Some("limits" | "nolimits")) {
                        self.bump();
                        let word = self.control().unwrap_or_default();
                        if let Some(Node::BigOp { limits, .. }) = out.last_mut() {
                            *limits = Some(word == "limits");
                        }
                    } else {
                        self.pos = mark;
                        return Ok(());
                    }
                }
                _ => {
                    self.pos = mark;
                    return Ok(());
                }
            }
        }
    }

    /// A sequence of math until one of `stops`; returns what stopped it.
    fn seq(&mut self, stops: &[Stop]) -> Result<(Vec<Node>, Stop), EquationError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error(5100, "The equation structure is too complex"));
        }
        let mut out: Vec<Node> = Vec::new();
        // The position of the last `\over`-family word, for infix forms.
        let mut infix: Option<(usize, String)> = None;
        let stop = loop {
            self.skip_space();
            let Some(c) = self.peek() else {
                if stops.contains(&Stop::Eof) {
                    break Stop::Eof;
                }
                return Err(self.error(5106, "Error parsing TeX: the text ends too soon"));
            };
            let found = match c {
                '$' => Some(Stop::Dollar),
                '}' => Some(Stop::Brace),
                '&' => Some(Stop::Amp),
                '\\' => match self.peek_control().as_deref() {
                    Some("\\") => Some(Stop::Row),
                    Some("right") => Some(Stop::Right),
                    Some("endmatrix") => Some(Stop::EndMatrix),
                    Some("endSb") => Some(Stop::EndSb),
                    Some("endSp") => Some(Stop::EndSp),
                    _ => None,
                },
                _ => None,
            };
            if let Some(stop) = found {
                if !stops.contains(&stop) {
                    return Err(self.error(5106, "Error parsing TeX: misplaced delimiter"));
                }
                // Consume the stopper (the caller reads what follows).
                match stop {
                    Stop::Dollar | Stop::Brace | Stop::Amp => {
                        self.bump();
                    }
                    Stop::Row => {
                        self.pos += 2;
                    }
                    Stop::Right => self.pos += "\\right".len(),
                    Stop::EndMatrix => self.pos += "\\endmatrix".len(),
                    Stop::EndSb => self.pos += "\\endSb".len(),
                    Stop::EndSp => self.pos += "\\endSp".len(),
                    Stop::Eof => {}
                }
                break stop;
            }
            if c == '\\'
                && let Some(word) = self.peek_control()
                && matches!(word.as_str(), "over" | "atop" | "choose")
            {
                self.pos += 1 + word.len();
                infix = Some((out.len(), word));
                continue;
            }
            self.atom(&mut out)?;
        };
        self.depth -= 1;
        if let Some((at, word)) = infix {
            let den = out.split_off(at.min(out.len()));
            let num = std::mem::take(&mut out);
            self.make()?;
            let word = match word.as_str() {
                "over" => "frac",
                "atop" => "atopfrac",
                _ => "binom",
            };
            out.push(Node::Frac {
                word: word.into(),
                num,
                den,
            });
        }
        Ok((out, stop))
    }

    /// One argument: a braced group or a single token.
    fn arg(&mut self) -> Result<Vec<Node>, EquationError> {
        self.skip_space();
        match self.peek() {
            None => Err(self.error(5106, "Error parsing TeX: an argument is missing")),
            Some('{') => {
                self.bump();
                let (nodes, stop) = self.seq(&[Stop::Brace])?;
                if stop != Stop::Brace {
                    return Err(self.error(5106, "Error parsing TeX"));
                }
                Ok(nodes)
            }
            Some(_) => {
                let mut out = Vec::new();
                let c = self.peek().unwrap_or(' ');
                if matches!(c, '}' | '&' | '$' | '^' | '_') {
                    return Err(self.error(5106, "Error parsing TeX: an argument is missing"));
                }
                self.single(&mut out)?;
                Ok(out)
            }
        }
    }

    /// A single token as a node (no scripts).
    fn single(&mut self, out: &mut Vec<Node>) -> Result<(), EquationError> {
        let c = self.peek().unwrap_or(' ');
        if c == '\\' {
            self.atom(out)?;
        } else {
            self.bump();
            self.make()?;
            out.push(Node::Ch(c));
        }
        Ok(())
    }

    /// The text between braces, nesting braces kept (for `\text`).
    fn raw_group(&mut self) -> Result<String, EquationError> {
        self.skip_space();
        if self.peek() != Some('{') {
            return Err(self.error(5106, "Error parsing TeX: { expected"));
        }
        self.bump();
        let start = self.pos;
        let mut level = 1;
        while let Some(c) = self.bump() {
            match c {
                '{' => level += 1,
                '}' => {
                    level -= 1;
                    if level == 0 {
                        return Ok(self.src[start..self.pos - 1].to_string());
                    }
                }
                _ => {}
            }
        }
        Err(self.error(5106, "Error parsing TeX: } is missing"))
    }

    /// `_x` and `^y` (and primes) onto the last node of `out`.
    fn scripts_into(&mut self, out: &mut Vec<Node>) -> Result<(), EquationError> {
        let mark = self.peek().unwrap_or('^');
        let (sub, sup) = match mark {
            '\'' => {
                self.bump();
                (None, Some(vec![Node::Sym("prime".into())]))
            }
            _ => {
                self.bump();
                let arg = self.script_arg()?;
                if mark == '_' {
                    (Some(arg), None)
                } else {
                    (None, Some(arg))
                }
            }
        };
        self.make()?;
        match out.pop() {
            Some(Node::BigOp {
                word,
                limits,
                sub: old_sub,
                sup: old_sup,
            }) => out.push(Node::BigOp {
                word,
                limits,
                sub: sub.or(old_sub),
                sup: sup.or(old_sup),
            }),
            Some(Node::Scripts {
                base,
                sub: old_sub,
                sup: old_sup,
            }) => out.push(Node::Scripts {
                base,
                sub: sub.or(old_sub),
                sup: sup.or(old_sup),
            }),
            Some(base) => out.push(Node::Scripts {
                base: Box::new(base),
                sub,
                sup,
            }),
            None => out.push(Node::Scripts {
                base: Box::new(Node::Group(Vec::new())),
                sub,
                sup,
            }),
        }
        Ok(())
    }

    /// A script's argument: `\Sb`/`\Sp` stacks are allowed.
    fn script_arg(&mut self) -> Result<Vec<Node>, EquationError> {
        self.skip_space();
        if matches!(self.peek_control().as_deref(), Some("Sb" | "Sp")) {
            let mut out = Vec::new();
            self.atom(&mut out)?;
            return Ok(out);
        }
        self.arg()
    }

    /// The delimiter after `\left` or `\right`.
    fn delimiter(&mut self) -> Result<String, EquationError> {
        self.skip_space();
        match self.peek() {
            None => Err(self.error(5106, "Error parsing TeX: a delimiter is missing")),
            Some('\\') => {
                self.bump();
                let word = self
                    .control()
                    .ok_or_else(|| self.error(5106, "Error parsing TeX"))?;
                if symbols::delimiter_word(&word).is_some() {
                    Ok(format!("\\{word}"))
                } else {
                    Err(self.error(5106, "Error parsing TeX: not a delimiter"))
                }
            }
            Some(c) if "()[]|/.<>".contains(c) => {
                self.bump();
                Ok(c.to_string())
            }
            Some(_) => Err(self.error(5106, "Error parsing TeX: not a delimiter")),
        }
    }

    /// One atom (with its arguments) onto `out`.
    fn atom(&mut self, out: &mut Vec<Node>) -> Result<(Class, ()), EquationError> {
        self.make()?;
        let Some(c) = self.peek() else {
            return Err(self.error(5106, "Error parsing TeX"));
        };
        match c {
            '{' => {
                self.bump();
                let (nodes, stop) = self.seq(&[Stop::Brace])?;
                if stop != Stop::Brace {
                    return Err(self.error(5106, "Error parsing TeX"));
                }
                out.push(Node::Group(nodes));
            }
            '^' | '_' | '\'' => self.scripts_into(out)?,
            '\\' => {
                let at = self.pos;
                self.bump();
                let word = self
                    .control()
                    .ok_or_else(|| self.error(5106, "Error parsing TeX"))?;
                self.control_word(&word, at, out)?;
            }
            _ => {
                self.bump();
                out.push(Node::Ch(c));
            }
        }
        Ok((Class::Ord, ()))
    }

    fn grid(&mut self, kind: GridKind) -> Result<Node, EquationError> {
        let (end, other) = match kind {
            GridKind::Matrix => (Stop::EndMatrix, Stop::EndMatrix),
            GridKind::Sub => (Stop::EndSb, Stop::EndSb),
            GridKind::Sup => (Stop::EndSp, Stop::EndSp),
        };
        let _ = other;
        let mut format = None;
        self.skip_space();
        if self.peek_control().as_deref() == Some("format") {
            // `\format \l & \c & \r \\ `.
            self.pos += "\\format".len();
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c == '\\' && self.peek_control().as_deref() == Some("\\") {
                    break;
                }
                self.bump();
            }
            format = Some(self.src[start..self.pos].trim().to_string());
            if self.peek_control().as_deref() == Some("\\") {
                self.pos += 2;
            }
        }
        let mut rows: Vec<Vec<Vec<Node>>> = vec![Vec::new()];
        loop {
            let (cell, stop) = self.seq(&[Stop::Amp, Stop::Row, end])?;
            rows.last_mut().expect("a row").push(cell);
            match stop {
                Stop::Amp => {}
                Stop::Row => rows.push(Vec::new()),
                _ => break,
            }
            if rows.len() > MAX_MATRIX || rows.iter().any(|row| row.len() > MAX_MATRIX) {
                return Err(self.error(5102, "Matrices are restricted to 10 x 10"));
            }
        }
        if rows
            .last()
            .is_some_and(|row| row.len() == 1 && row[0].is_empty())
            && rows.len() > 1
        {
            rows.pop();
        }
        if rows.len() > MAX_MATRIX || rows.iter().any(|row| row.len() > MAX_MATRIX) {
            return Err(self.error(5102, "Matrices are restricted to 10 x 10"));
        }
        Ok(Node::Grid { kind, format, rows })
    }

    fn control_word(
        &mut self,
        word: &str,
        at: usize,
        out: &mut Vec<Node>,
    ) -> Result<(), EquationError> {
        match word {
            "frac" | "tfrac" | "dfrac" | "binom" | "tbinom" | "dbinom" => {
                let num = self.arg()?;
                let den = self.arg()?;
                out.push(Node::Frac {
                    word: word.into(),
                    num,
                    den,
                });
            }
            "sqrt" => {
                let body = self.arg()?;
                out.push(Node::Sqrt(body));
            }
            "root" => {
                let index = self.arg()?;
                self.skip_space();
                if self.peek_control().as_deref() != Some("of") {
                    return Err(self.error(5106, "Error parsing TeX: \\of expected"));
                }
                self.pos += "\\of".len();
                let body = self.arg()?;
                out.push(Node::Root { index, body });
            }
            "matrix" => {
                let grid = self.grid(GridKind::Matrix)?;
                out.push(grid);
            }
            "Sb" => {
                let grid = self.grid(GridKind::Sub)?;
                out.push(grid);
            }
            "Sp" => {
                let grid = self.grid(GridKind::Sup)?;
                out.push(grid);
            }
            "left" => {
                let left = self.delimiter()?;
                let (body, stop) = self.seq(&[Stop::Right])?;
                if stop != Stop::Right {
                    return Err(self.error(5106, "Error parsing TeX: \\right expected"));
                }
                let right = self.delimiter()?;
                out.push(Node::Delimited { left, right, body });
            }
            "overset" | "underset" => {
                let note = self.arg()?;
                self.skip_space();
                if self.peek_control().as_deref() == Some("to") {
                    self.pos += "\\to".len();
                }
                let base = self.arg()?;
                out.push(Node::Stack {
                    under: word == "underset",
                    note,
                    base,
                });
            }
            "text" => {
                let text = self.raw_group()?;
                out.push(Node::Text(text));
            }
            "func" | "limfunc" => {
                let name = self.raw_group()?;
                out.push(Node::Func {
                    name,
                    unnamed: true,
                });
            }
            "limits" | "nolimits" => {
                if let Some(Node::BigOp { limits, .. }) = out.last_mut() {
                    *limits = Some(word == "limits");
                }
            }
            _ if FontKind::from_word(word).is_some() => {
                out.push(Node::Font(FontKind::from_word(word).expect("a font word")));
            }
            _ if symbols::big_operator(word).is_some() => {
                out.push(Node::BigOp {
                    word: word.into(),
                    limits: None,
                    sub: None,
                    sup: None,
                });
            }
            _ if symbols::FUNCTIONS.contains(&word) => {
                out.push(Node::Func {
                    name: word.into(),
                    unnamed: false,
                });
            }
            _ if symbols::DECORATIONS.contains(&word)
                || symbols::ACCENTS.iter().any(|accent| accent.0 == word) =>
            {
                let arg = self.arg()?;
                out.push(Node::Decor {
                    word: word.into(),
                    arg,
                });
            }
            _ if symbols::space_width(word).is_some() => out.push(Node::Space(word.into())),
            _ if symbols::symbol(word).is_some() => out.push(Node::Sym(word.into())),
            // Words with no layout of their own (labels, references) are
            // read and dropped.
            "label" | "ref" | "pageref" | "cite" | "glossary" => {
                let _ = self.raw_group()?;
            }
            "limitsfmt" => {}
            "\\" | "of" | "endmatrix" | "right" | "to" | "format" => {
                self.pos = at;
                return Err(self.error(5106, "Error parsing TeX: misplaced command"));
            }
            _ => {
                return Err(EquationError {
                    code: 5106,
                    message: format!("Error parsing TeX: unknown command \\{word}"),
                    position: at,
                });
            }
        }
        Ok(())
    }
}
