//! Equations (`AmiEqn`, `.tex`; notes §11.2, §11.6, §12.2): Ami Pro's TeX
//! dialect parsed into a tree ([`parse`]), laid out by TeX's rules with
//! constants fitted to Ami Pro's natural sizes ([`layout`]), written back
//! as Ami Pro writes it ([`write`]), and drawn as a display list
//! ([`Layout::to_drawing`]). The stored form is two text lines (the font
//! settings, then the TeX); Ami Pro lays it out again at every repaint, so
//! no picture is stored.

pub mod layout;
pub mod metrics;
pub mod parse;
pub mod symbols;
pub mod write;

#[cfg(test)]
mod tests;

pub use layout::{Ink, Item, Layout, Params, Seg, layout};
pub use parse::{Node, Tree};

/// An error in the equation, with Ami Pro's message number: 5100 structure
/// too complex, 5102 matrices over 10 by 10, 5105 equation too big, 5106
/// error parsing TeX.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquationError {
    pub code: u16,
    pub message: String,
    /// The byte offset in the source text where it was found.
    pub position: usize,
}

impl std::fmt::Display for EquationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

/// The deepest nesting of groups.
pub const MAX_DEPTH: usize = 40;
/// The largest matrix in rows and in columns.
pub const MAX_MATRIX: usize = 10;
/// The longest equation text, in bytes.
pub const MAX_SOURCE: usize = 8_000;

/// The settings line Ami Pro writes first (`FontFace,pitch,size,colour,
/// attributes,charset,rotation,0`).
pub const DEFAULT_SETTINGS: &str = "TimesNewRomanPS,18,12,0,0,0,0,0";

/// The stored form of an equation: the settings line and the TeX text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stored {
    /// The first line, without its line break.
    pub settings: String,
    /// What follows the first CRLF, as it was.
    pub tex: String,
}

impl Stored {
    pub fn new(tex: &str) -> Self {
        Self {
            settings: DEFAULT_SETTINGS.to_string(),
            tex: tex.to_string(),
        }
    }

    /// Reads the object's bytes; `None` when they are not two text lines.
    pub fn read(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?;
        let (settings, tex) = text.split_once("\r\n")?;
        if settings.split(',').count() < 3 || settings.contains('\n') {
            return None;
        }
        Some(Self {
            settings: settings.to_string(),
            tex: tex.to_string(),
        })
    }

    pub fn bytes(&self) -> Vec<u8> {
        format!("{}\r\n{}", self.settings, self.tex).into_bytes()
    }

    /// The face name, the point size from the settings line.
    pub fn face(&self) -> (String, f64) {
        let mut parts = self.settings.split(',');
        let face = parts.next().unwrap_or("TimesNewRomanPS").to_string();
        let size = parts
            .nth(1)
            .and_then(|size| size.trim().parse::<f64>().ok())
            .filter(|size| (1.0..=400.0).contains(size))
            .unwrap_or(12.0);
        (face, size)
    }

    /// Replaces the face and size, keeping the other settings words.
    pub fn set_face(&mut self, face: &str, size: f64) {
        let mut parts: Vec<String> = self.settings.split(',').map(str::to_string).collect();
        while parts.len() < 8 {
            parts.push("0".into());
        }
        parts[0] = face.to_string();
        parts[2] = format!("{}", size.round() as i64);
        self.settings = parts.join(",");
    }

    /// The tree of the stored text (`$` toggles math).
    pub fn parse(&self) -> Result<Tree, EquationError> {
        parse::parse(&self.tex)
    }

    /// The tex, normalised as Ami Pro writes it.
    pub fn normalised(&self) -> Result<String, EquationError> {
        Ok(write::write(&self.parse()?))
    }

    /// The laid-out equation at the stored point size.
    pub fn layout(&self) -> Result<Layout, EquationError> {
        let tree = self.parse()?;
        let (_, size) = self.face();
        layout(&tree, &Params::at_points(size))
    }
}

/// The stored text for what the user typed in the editor: with no `$` in
/// it the whole text is one math run; it is then normalised as Ami Pro
/// writes it. The error has the position in `source`.
pub fn from_source(source: &str) -> Result<String, EquationError> {
    if source.len() > MAX_SOURCE {
        return Err(EquationError {
            code: 5105,
            message: "The equation is too big".into(),
            position: MAX_SOURCE,
        });
    }
    let tree = if source.contains('$') {
        parse::parse(source)?
    } else if source.trim().is_empty() {
        Vec::new()
    } else {
        parse::parse_math(source)?
    };
    Ok(write::write(&tree))
}

/// The text the editor shows for stored text: a single math run shows
/// without its `$` signs.
pub fn to_source(stored: &str) -> String {
    let trimmed = stored.trim_end();
    if let Some(inner) = trimmed.strip_prefix('$').and_then(|r| r.strip_suffix('$'))
        && !inner.contains('$')
    {
        return inner.to_string();
    }
    stored.to_string()
}

/// A TeX file's text as Ami Pro's Import Equation stores it.
pub fn import_tex(text: &str) -> Result<String, EquationError> {
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let tree = parse::parse_import(flat.trim())?;
    Ok(write::write(&tree))
}

/// The text Save As Equation writes: the stored text, broken at the last
/// space before column 79 once a line reaches 70 (`11a8:012e`).
pub fn save_as_text(stored: &str) -> String {
    let mut out = String::new();
    let mut line = String::new();
    for word in stored.split_inclusive(' ') {
        if line.chars().count() >= 70 {
            out.push_str(line.trim_end());
            out.push_str("\r\n");
            line.clear();
        }
        line.push_str(word);
        if line.chars().count() >= 79 {
            out.push_str(line.trim_end());
            out.push_str("\r\n");
            line.clear();
        }
    }
    out.push_str(&line);
    out
}

/// LaTeX (`$...$`, `\[...\]`, an `equation` environment) as the editor's
/// source, or `None` when the text is not a LaTeX equation.
pub fn latex_to_source(text: &str) -> Option<String> {
    let text = text.trim();
    let inner = if let Some(rest) = text.strip_prefix("$$") {
        rest.strip_suffix("$$")?
    } else if let Some(rest) = text.strip_prefix('$') {
        rest.strip_suffix('$')?
    } else if let Some(rest) = text.strip_prefix("\\[") {
        rest.strip_suffix("\\]")?
    } else if let Some(rest) = text.strip_prefix("\\(") {
        rest.strip_suffix("\\)")?
    } else if let Some(rest) = text.strip_prefix("\\begin{equation}") {
        rest.strip_suffix("\\end{equation}")?
    } else {
        text.strip_prefix("\\begin{equation*}")?
            .strip_suffix("\\end{equation*}")?
    };
    let inner = inner.trim();
    if inner.is_empty() || inner.contains('$') {
        return None;
    }
    Some(latex_dialect(inner))
}

/// LaTeX's spellings turned into Ami Pro's dialect (`\sqrt[n]{x}` into
/// `\root{n}\of{x}`, `\begin{matrix}` into `\matrix`, `\mathbf` into
/// `{\BF }`).
pub fn latex_dialect(latex: &str) -> String {
    let mut text = latex.to_string();
    for (from, to) in [
        ("\\begin{pmatrix}", "\\left( \\matrix "),
        ("\\end{pmatrix}", "\\endmatrix \\right) "),
        ("\\begin{bmatrix}", "\\left[ \\matrix "),
        ("\\end{bmatrix}", "\\endmatrix \\right] "),
        ("\\begin{vmatrix}", "\\left| \\matrix "),
        ("\\end{vmatrix}", "\\endmatrix \\right| "),
        ("\\begin{matrix}", "\\matrix "),
        ("\\end{matrix}", "\\endmatrix "),
        ("\\mathbf{", "{\\BF "),
        ("\\mathrm{", "{\\RM "),
        ("\\mathit{", "{\\IT "),
        ("\\mathbb{", "{\\BBB "),
        ("\\mathcal{", "{\\CAL "),
        ("\\mathfrak{", "{\\FRAK "),
        ("\\mathtt{", "{\\TT "),
        ("\\textbf{", "{\\BF "),
        ("\\operatorname{", "\\func{"),
    ] {
        text = text.replace(from, to);
    }
    // `\sqrt[n]{x}` to `\root{n} \of{x}`.
    while let Some(at) = text.find("\\sqrt[") {
        let Some(close) = text[at..].find(']') else {
            break;
        };
        let index = text[at + 6..at + close].to_string();
        text.replace_range(at..at + close + 1, &format!("\\root{{{index}}} \\of"));
    }
    text
}

/// LaTeX for the editor's source (Copy as LaTeX): Ami Pro's dialect with
/// `\root` and `\matrix` turned into LaTeX's environments.
pub fn source_to_latex(source: &str) -> String {
    let mut text = source.trim().to_string();
    while let Some(at) = text.find("\\root") {
        let rest = &text[at + 5..];
        let Some(open) = rest.find('{') else { break };
        let Some(close) = rest.find('}') else { break };
        let index = rest[open + 1..close].trim().to_string();
        let after = rest[close + 1..].trim_start();
        let Some(body) = after.strip_prefix("\\of") else {
            break;
        };
        let consumed = text.len() - body.len();
        text.replace_range(at..consumed, &format!("\\sqrt[{index}]"));
    }
    text = text
        .replace("\\matrix ", "\\begin{matrix} ")
        .replace("\\endmatrix", "\\end{matrix}");
    text
}
