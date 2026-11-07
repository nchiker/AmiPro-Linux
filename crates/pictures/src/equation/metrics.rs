//! Glyph metrics for the layout: advance widths and ink heights as shares
//! of the em. The widths are those of the Times family (a Times-compatible
//! face is what Ami Pro uses and what the system provides: Liberation
//! Serif and Tinos have the same advances), so the layout needs no font
//! file and gives the same answer on every machine.

/// A face as the layout needs to tell them apart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Face {
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
}

impl Face {
    pub const ROMAN: Self = Self {
        bold: false,
        italic: false,
        mono: false,
    };
    pub const ITALIC: Self = Self {
        bold: false,
        italic: true,
        mono: false,
    };
}

const ROMAN: [u16; 95] = [
    250, 333, 408, 500, 500, 833, 778, 180, 333, 333, 500, 564, 250, 333, 250,
    278, // space..slash
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, // digits
    278, 278, 564, 564, 564, 444, 921, // : ; < = > ? @
    722, 667, 667, 722, 611, 556, 722, 722, 333, 389, 722, 611, 889, 722, 722, 556, 722, 667, 556,
    611, 722, 722, 944, 722, 722, 611, // A-Z
    333, 278, 333, 469, 500, 333, // [ \ ] ^ _ `
    444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500, 278, 778, 500, 500, 500, 500, 333, 389,
    278, 500, 500, 722, 500, 500, 444, // a-z
    480, 200, 480, 541, // { | } ~
];

const ITALIC: [u16; 95] = [
    250, 333, 420, 500, 500, 833, 778, 214, 333, 333, 500, 675, 250, 333, 250, 278, 500, 500, 500,
    500, 500, 500, 500, 500, 500, 500, 333, 333, 675, 675, 675, 500, 920, 611, 611, 667, 722, 611,
    611, 722, 722, 333, 444, 667, 556, 833, 667, 722, 611, 722, 611, 500, 556, 722, 611, 833, 611,
    556, 556, 389, 278, 389, 422, 500, 333, 500, 500, 444, 500, 444, 278, 500, 500, 278, 278, 444,
    278, 722, 500, 500, 500, 500, 389, 389, 278, 500, 444, 667, 444, 444, 389, 400, 275, 400, 541,
];

/// The advance of a character as a share of the em, before the layout's
/// calibration scale.
pub fn advance(c: char, face: Face) -> f64 {
    if face.mono {
        return 0.6;
    }
    // Fitted to Ami Pro's natural sizes (`OBJECTS_NOTES.md` §15): its italic
    // x, y and z differ from Times Italic's.
    if face.italic && !face.bold {
        match c {
            'x' => return 0.433,
            'y' => return 0.52,
            'z' => return 0.56,
            _ => {}
        }
    }
    let table = if face.italic { &ITALIC } else { &ROMAN };
    let code = c as u32;
    let base = if (32..127).contains(&code) {
        f64::from(table[(code - 32) as usize]) / 1000.0
    } else {
        match c {
            '\u{3b1}'..='\u{3c9}' | '\u{3d1}' | '\u{3d5}' | '\u{3d6}' | '\u{3f1}' | '\u{3f5}' => {
                0.56
            }
            '\u{391}'..='\u{3a9}' => 0.7,
            '+' | '=' | '<' | '>' | '−' | '±' | '∓' | '×' | '÷' | '≤' | '≥' | '≠' | '≡' | '≈'
            | '∼' | '≃' | '≅' | '∝' | '∈' | '∋' | '⊂' | '⊃' | '⊆' | '⊇' => {
                0.549
            }
            '∑' => 0.64,
            '∏' => 0.72,
            '∫' => 0.27,
            '∞' => 0.71,
            '·' | '∘' | '•' | '∗' | '⋆' => 0.4,
            '…' => 1.0,
            '⋯' => 1.0,
            '←' | '→' | '↔' | '⇐' | '⇒' | '⇔' | '↦' | '↑' | '↓' => 0.99,
            '′' => 0.25,
            '°' => 0.4,
            '‖' => 0.4,
            '⟨' | '⟩' => 0.33,
            '⌊' | '⌋' | '⌈' | '⌉' => 0.4,
            c if c.is_ascii() => 0.5,
            _ => 0.6,
        }
    };
    if face.bold && !face.italic {
        base * 1.04
    } else {
        base
    }
}

/// The ink above and below the baseline as shares of the em.
pub fn ink(c: char, face: Face) -> (f64, f64) {
    let _ = face;
    match c {
        'a' | 'c' | 'e' | 'm' | 'n' | 'o' | 'r' | 's' | 'u' | 'v' | 'w' | 'x' | 'z' => (0.46, 0.01),
        'g' | 'p' | 'q' | 'y' => (0.46, 0.22),
        'j' => (0.68, 0.22),
        'b' | 'd' | 'f' | 'h' | 'i' | 'k' | 'l' | 't' => (0.68, 0.01),
        '0'..='9' => (0.62, 0.01),
        'A'..='Z' => (0.662, 0.01),
        'α' | 'ε' | 'ι' | 'κ' | 'μ' | 'ν' | 'ο' | 'π' | 'σ' | 'τ' | 'υ' | 'ω' | 'ϵ' => {
            (0.46, 0.22)
        }
        'β' | 'δ' | 'ζ' | 'θ' | 'λ' | 'ξ' | 'φ' | 'ψ' | 'ϕ' | 'ℓ' => (0.7, 0.22),
        'γ' | 'η' | 'ρ' | 'χ' | 'ς' => (0.46, 0.22),
        ' ' | '.' | ',' | '_' => (0.1, 0.1),
        '+' | '-' | '=' | '−' | '×' | '·' | '<' | '>' => (0.55, 0.05),
        '(' | ')' | '[' | ']' | '{' | '}' | '|' => (0.72, 0.2),
        '∑' | '∏' => (0.73, 0.2),
        '∫' => (0.8, 0.27),
        _ => (0.66, 0.02),
    }
}
