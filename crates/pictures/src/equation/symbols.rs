//! The control sequences of Ami Pro's TeX dialect that name a symbol,
//! an operator or a function (the dialect's table is 887 names in
//! `amieqn.dll`'s resource; this is the common set of it, written for this
//! project from the names the decompile lists and standard TeX's).

/// What a symbol is for spacing, as TeX classes atoms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Class {
    Ord,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

const fn s(name: &'static str, ch: char, class: Class) -> (&'static str, char, Class) {
    (name, ch, class)
}

use Class::{Bin, Close, Open, Ord, Punct, Rel};

static SYMBOLS: &[(&str, char, Class)] = &[
    // Greek.
    s("alpha", 'α', Ord),
    s("beta", 'β', Ord),
    s("gamma", 'γ', Ord),
    s("delta", 'δ', Ord),
    s("epsilon", 'ϵ', Ord),
    s("varepsilon", 'ε', Ord),
    s("zeta", 'ζ', Ord),
    s("eta", 'η', Ord),
    s("theta", 'θ', Ord),
    s("vartheta", 'ϑ', Ord),
    s("iota", 'ι', Ord),
    s("kappa", 'κ', Ord),
    s("lambda", 'λ', Ord),
    s("mu", 'μ', Ord),
    s("nu", 'ν', Ord),
    s("xi", 'ξ', Ord),
    s("omicron", 'ο', Ord),
    s("pi", 'π', Ord),
    s("varpi", 'ϖ', Ord),
    s("rho", 'ρ', Ord),
    s("varrho", 'ϱ', Ord),
    s("sigma", 'σ', Ord),
    s("varsigma", 'ς', Ord),
    s("tau", 'τ', Ord),
    s("upsilon", 'υ', Ord),
    s("phi", 'ϕ', Ord),
    s("varphi", 'φ', Ord),
    s("chi", 'χ', Ord),
    s("psi", 'ψ', Ord),
    s("omega", 'ω', Ord),
    s("Gamma", 'Γ', Ord),
    s("Delta", 'Δ', Ord),
    s("Theta", 'Θ', Ord),
    s("Lambda", 'Λ', Ord),
    s("Xi", 'Ξ', Ord),
    s("Pi", 'Π', Ord),
    s("Sigma", 'Σ', Ord),
    s("Upsilon", 'Υ', Ord),
    s("Phi", 'Φ', Ord),
    s("Psi", 'Ψ', Ord),
    s("Omega", 'Ω', Ord),
    // Binary operators.
    s("pm", '±', Bin),
    s("mp", '∓', Bin),
    s("times", '×', Bin),
    s("div", '÷', Bin),
    s("cdot", '·', Bin),
    s("ast", '∗', Bin),
    s("star", '⋆', Bin),
    s("circ", '∘', Bin),
    s("bullet", '•', Bin),
    s("oplus", '⊕', Bin),
    s("ominus", '⊖', Bin),
    s("otimes", '⊗', Bin),
    s("oslash", '⊘', Bin),
    s("odot", '⊙', Bin),
    s("cap", '∩', Bin),
    s("cup", '∪', Bin),
    s("sqcap", '⊓', Bin),
    s("sqcup", '⊔', Bin),
    s("vee", '∨', Bin),
    s("wedge", '∧', Bin),
    s("lor", '∨', Bin),
    s("land", '∧', Bin),
    s("setminus", '∖', Bin),
    s("wr", '≀', Bin),
    s("dagger", '†', Bin),
    s("ddagger", '‡', Bin),
    s("amalg", '⨿', Bin),
    s("uplus", '⊎', Bin),
    s("triangleleft", '◁', Bin),
    s("triangleright", '▷', Bin),
    s("diamond", '⋄', Bin),
    // Relations.
    s("leq", '≤', Rel),
    s("le", '≤', Rel),
    s("geq", '≥', Rel),
    s("ge", '≥', Rel),
    s("neq", '≠', Rel),
    s("ne", '≠', Rel),
    s("equiv", '≡', Rel),
    s("approx", '≈', Rel),
    s("sim", '∼', Rel),
    s("simeq", '≃', Rel),
    s("cong", '≅', Rel),
    s("propto", '∝', Rel),
    s("ll", '≪', Rel),
    s("gg", '≫', Rel),
    s("prec", '≺', Rel),
    s("succ", '≻', Rel),
    s("preceq", '⪯', Rel),
    s("succeq", '⪰', Rel),
    s("subset", '⊂', Rel),
    s("supset", '⊃', Rel),
    s("subseteq", '⊆', Rel),
    s("supseteq", '⊇', Rel),
    s("in", '∈', Rel),
    s("ni", '∋', Rel),
    s("notin", '∉', Rel),
    s("perp", '⊥', Rel),
    s("parallel", '∥', Rel),
    s("mid", '∣', Rel),
    s("vdash", '⊢', Rel),
    s("dashv", '⊣', Rel),
    s("models", '⊨', Rel),
    s("asymp", '≍', Rel),
    s("doteq", '≐', Rel),
    s("bowtie", '⋈', Rel),
    // Arrows.
    s("leftarrow", '←', Rel),
    s("gets", '←', Rel),
    s("rightarrow", '→', Rel),
    s("to", '→', Rel),
    s("leftrightarrow", '↔', Rel),
    s("Leftarrow", '⇐', Rel),
    s("Rightarrow", '⇒', Rel),
    s("Leftrightarrow", '⇔', Rel),
    s("iff", '⇔', Rel),
    s("implies", '⇒', Rel),
    s("mapsto", '↦', Rel),
    s("uparrow", '↑', Rel),
    s("downarrow", '↓', Rel),
    s("updownarrow", '↕', Rel),
    s("Uparrow", '⇑', Rel),
    s("Downarrow", '⇓', Rel),
    s("nearrow", '↗', Rel),
    s("searrow", '↘', Rel),
    s("swarrow", '↙', Rel),
    s("nwarrow", '↖', Rel),
    s("hookrightarrow", '↪', Rel),
    s("hookleftarrow", '↩', Rel),
    s("longrightarrow", '⟶', Rel),
    s("longleftarrow", '⟵', Rel),
    s("rightleftharpoons", '⇌', Rel),
    // Ordinary symbols.
    s("infty", '∞', Ord),
    s("partial", '∂', Ord),
    s("nabla", '∇', Ord),
    s("forall", '∀', Ord),
    s("exists", '∃', Ord),
    s("nexists", '∄', Ord),
    s("emptyset", '∅', Ord),
    s("varnothing", '∅', Ord),
    s("neg", '¬', Ord),
    s("lnot", '¬', Ord),
    s("angle", '∠', Ord),
    s("triangle", '△', Ord),
    s("prime", '′', Ord),
    s("hbar", 'ℏ', Ord),
    s("ell", 'ℓ', Ord),
    s("Re", 'ℜ', Ord),
    s("Im", 'ℑ', Ord),
    s("aleph", 'ℵ', Ord),
    s("wp", '℘', Ord),
    s("imath", 'ı', Ord),
    s("jmath", 'ȷ', Ord),
    s("ldots", '…', Ord),
    s("dots", '…', Ord),
    s("cdots", '⋯', Ord),
    s("vdots", '⋮', Ord),
    s("ddots", '⋱', Ord),
    s("top", '⊤', Ord),
    s("bot", '⊥', Ord),
    s("flat", '♭', Ord),
    s("natural", '♮', Ord),
    s("sharp", '♯', Ord),
    s("clubsuit", '♣', Ord),
    s("diamondsuit", '♢', Ord),
    s("heartsuit", '♡', Ord),
    s("spadesuit", '♠', Ord),
    s("surd", '√', Ord),
    s("degree", '°', Ord),
    s("S", '§', Ord),
    s("P", '¶', Ord),
    s("copyright", '©', Ord),
    s("pounds", '£', Ord),
    s("backslash", '\\', Ord),
    s("lbrace", '{', Open),
    s("rbrace", '}', Close),
    s("langle", '⟨', Open),
    s("rangle", '⟩', Close),
    s("lfloor", '⌊', Open),
    s("rfloor", '⌋', Close),
    s("lceil", '⌈', Open),
    s("rceil", '⌉', Close),
    s("vert", '|', Ord),
    s("Vert", '‖', Ord),
    s("colon", ':', Punct),
    // Control symbols.
    s("{", '{', Open),
    s("}", '}', Close),
    s("|", '‖', Ord),
    s("%", '%', Ord),
    s("&", '&', Ord),
    s("#", '#', Ord),
    s("$", '$', Ord),
    s("_", '_', Ord),
];

/// A symbol's character and class by its control word (without the
/// backslash).
pub fn symbol(name: &str) -> Option<(char, Class)> {
    SYMBOLS
        .iter()
        .find(|entry| entry.0 == name)
        .map(|entry| (entry.1, entry.2))
}

/// The control word of a symbol's character, for writing back.
pub fn symbol_name(ch: char) -> Option<&'static str> {
    SYMBOLS
        .iter()
        .find(|entry| entry.1 == ch && entry.0.len() > 1)
        .map(|entry| entry.0)
}

/// Big operators: name, character, and whether limits go above and below
/// in display style.
static BIG_OPERATORS: &[(&str, char, bool)] = &[
    ("sum", '∑', true),
    ("prod", '∏', true),
    ("coprod", '∐', true),
    ("int", '∫', false),
    ("iint", '∬', false),
    ("iiint", '∭', false),
    ("iiiint", '⨌', false),
    ("idotsint", '∫', false),
    ("oint", '∮', false),
    ("bigcap", '⋂', true),
    ("bigcup", '⋃', true),
    ("bigvee", '⋁', true),
    ("bigwedge", '⋀', true),
    ("bigoplus", '⨁', true),
    ("bigotimes", '⨂', true),
    ("bigodot", '⨀', true),
    ("biguplus", '⨄', true),
    ("bigsqcup", '⨆', true),
];

/// A big operator by name; the text-style (`tsum`) and display-style
/// (`dsum`) spellings give the same operator.
pub fn big_operator(name: &str) -> Option<(char, bool)> {
    let find = |name: &str| {
        BIG_OPERATORS
            .iter()
            .find(|entry| entry.0 == name)
            .map(|entry| (entry.1, entry.2))
    };
    find(name).or_else(|| {
        let rest = name.strip_prefix('t').or_else(|| name.strip_prefix('d'))?;
        find(rest)
    })
}

/// The 27 function names Ami Pro writes as `\name `.
pub static FUNCTIONS: &[&str] = &[
    "arccos", "arcsin", "arctan", "arg", "cos", "cosh", "cot", "coth", "csc", "deg", "det", "dim",
    "exp", "gcd", "hom", "inf", "ker", "lg", "lim", "liminf", "limsup", "ln", "log", "max", "min",
    "sec", "sin", "sinh", "sup", "tan", "tanh", "Pr",
];

/// Functions that take their limits below (as `lim` does).
pub fn function_has_limits(name: &str) -> bool {
    matches!(
        name,
        "lim" | "liminf" | "limsup" | "max" | "min" | "sup" | "inf" | "det" | "gcd" | "Pr"
    )
}

/// Font switches (`{\BF x}`).
pub static FONT_SWITCHES: &[&str] = &[
    "BF", "IT", "RM", "SL", "SMC", "TT", "CAL", "FRAK", "BBB", "NEG",
];

/// Accents drawn over one character, with the combining mark that draws
/// them.
pub static ACCENTS: &[(&str, char)] = &[
    ("hat", '\u{302}'),
    ("check", '\u{30c}'),
    ("tilde", '\u{303}'),
    ("acute", '\u{301}'),
    ("grave", '\u{300}'),
    ("dot", '\u{307}'),
    ("ddot", '\u{308}'),
    ("breve", '\u{306}'),
    ("bar", '\u{304}'),
    ("vec", '\u{20d7}'),
];

/// Decorations over or under a whole argument.
pub static DECORATIONS: &[&str] = &[
    "overline",
    "underline",
    "overbrace",
    "underbrace",
    "widehat",
    "widetilde",
    "overleftarrow",
    "overrightarrow",
    "underleftarrow",
    "underrightarrow",
    "overleftrightarrow",
    "underleftrightarrow",
];

/// Spacing control words, in thousandths of an em.
pub fn space_width(name: &str) -> Option<i32> {
    match name {
        "thinspace" | "," => Some(167),
        "negthinspace" | "!" => Some(-167),
        "medspace" | ":" => Some(222),
        "thickspace" | ";" => Some(278),
        "enspace" => Some(500),
        "quad" => Some(1000),
        "qquad" => Some(2000),
        "space" | " " => Some(250),
        _ => None,
    }
}

/// The delimiters `\left` and `\right` take, by their control word.
pub fn delimiter_word(name: &str) -> Option<char> {
    match name {
        "langle" => Some('⟨'),
        "rangle" => Some('⟩'),
        "lfloor" => Some('⌊'),
        "rfloor" => Some('⌋'),
        "lceil" => Some('⌈'),
        "rceil" => Some('⌉'),
        "lbrace" | "{" => Some('{'),
        "rbrace" | "}" => Some('}'),
        "vert" | "|" => Some('|'),
        "Vert" => Some('‖'),
        "backslash" => Some('\\'),
        "uparrow" => Some('↑'),
        "downarrow" => Some('↓'),
        "updownarrow" => Some('↕'),
        "Uparrow" => Some('⇑'),
        "Downarrow" => Some('⇓'),
        _ => None,
    }
}
