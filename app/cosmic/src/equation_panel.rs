//! The Equation editor (`docs/Design/UI_SPEC.md` §42.5, decisions OD2, OD6,
//! OD11; run D, DO.9): Tools › Equations… and a double-click on an
//! equation frame edit the equation in place. The TeX source is typed in a
//! card at the frame (the equation redrawn as it is typed, a parse error
//! shown with its position under the card while the last good drawing
//! stays); the rail holds the template palette, the face and size, Import
//! Equation…, Save As Equation…, and the LaTeX commands.
//!
//! The source is Ami Pro's dialect without the outer `$` signs; a text
//! with `$` in it is the stored form as it is (`amipro_pictures::equation`).

use amipro_document::FrameId;
use amipro_pictures::equation::EquationError;
use cosmic::iced::Length;
use cosmic::iced::widget::text_editor;
use cosmic::widget;

use crate::Message;

/// The panel's width in the rail.
pub const WIDTH: f32 = 320.0;
/// The mark a template leaves where something is to be typed.
pub const HOLE: char = '⬚';
/// The faces the chooser offers (the first is Ami Pro's default).
pub const FACES: [&str; 4] = ["TimesNewRomanPS", "Times New Roman", "Arial", "Courier New"];
/// The id of the source field, to focus it.
pub const SOURCE_ID: &str = "equation-source";

type Element<'a> = cosmic::Element<'a, Message>;

/// What the panel and the card show of an equation being edited.
pub struct EquationPanel {
    pub frame: FrameId,
    pub source: text_editor::Content,
    /// The last parse or layout error, with its position in the source.
    pub error: Option<EquationError>,
    /// The face's place in [`FACES`], or its name when it is another.
    pub face: String,
    /// The size as typed (points).
    pub size: String,
    pub size_error: bool,
    pub notice: Option<String>,
    /// Whether typed edits join the open undo step.
    pub run_open: bool,
    /// The stored text last put on the frame, to skip what changes nothing.
    pub last_applied: String,
    /// The settings line last put on the frame.
    pub last_settings: String,
    /// The stored text could not be read: the equation is kept as it is.
    pub unreadable: bool,
}

impl EquationPanel {
    pub fn new(frame: FrameId) -> Self {
        Self {
            frame,
            source: text_editor::Content::new(),
            error: None,
            face: FACES[0].to_string(),
            size: "12".into(),
            size_error: false,
            notice: None,
            run_open: false,
            last_applied: String::new(),
            last_settings: String::new(),
            unreadable: false,
        }
    }

    /// The source as typed, without the trailing line break.
    pub fn source_text(&self) -> String {
        let mut text = self.source.text();
        while text.ends_with(['\n', '\r']) {
            text.pop();
        }
        text
    }

    /// The error as a line: its message and where it is.
    pub fn error_line(&self) -> Option<String> {
        let error = self.error.as_ref()?;
        let text = self.source_text();
        let before = &text[..error.position.min(text.len())];
        let line = before.matches('\n').count() + 1;
        let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
        Some(format!(
            "{} (message {}) at line {line}, column {column}",
            error.message.trim_end_matches('.'),
            error.code
        ))
    }
}

#[derive(Clone, Debug)]
pub enum EquationAction {
    /// An edit or move in the source field.
    Edit(text_editor::Action),
    /// Enter in the source field: done (the panel closes).
    Done,
    /// A palette button: the template goes in at the caret.
    Insert(&'static str),
    Face(usize),
    Size(String),
    SizeSubmit,
    /// Import Equation…: the dialog (a TeX file).
    Import,
    /// Save As Equation…: the dialog.
    SaveAs,
    /// Copy as LaTeX.
    CopyLatex,
    /// Paste LaTeX: the clipboard's text if it is a LaTeX equation.
    PasteLatex,
    Close,
}

fn send(action: EquationAction) -> Message {
    Message::Equation(action)
}

// ---------------------------------------------------------------- palette

/// A template button: its label, its tooltip, the text it inserts. The
/// first [`HOLE`] is selected after the insertion.
type Template = (&'static str, &'static str, &'static str);

pub const FRACTIONS: &[Template] = &[
    ("a/b", "Fraction", "\\frac{⬚}{⬚}"),
    ("(a b)", "Binomial", "\\binom{⬚}{⬚}"),
    ("a⁄b", "Fraction in text size", "\\tfrac{⬚}{⬚}"),
    ("a⁄b", "Fraction in display size", "\\dfrac{⬚}{⬚}"),
];
pub const RADICALS: &[Template] = &[
    ("√", "Square root", "\\sqrt{⬚}"),
    ("ⁿ√", "Root with an index", "\\root{⬚} \\of{⬚}"),
];
pub const SCRIPTS: &[Template] = &[
    ("xⁿ", "Superscript", "^{⬚}"),
    ("xₙ", "Subscript", "_{⬚}"),
    ("xₙⁿ", "Subscript and superscript", "_{⬚}^{⬚}"),
    ("x′", "Prime", "'"),
];
pub const OPERATORS: &[Template] = &[
    ("∑", "Sum", "\\sum_{⬚}^{⬚}"),
    (
        "∑ᵃᵇ",
        "Sum with limits above and below",
        "\\sum\\limits_{⬚}^{⬚}",
    ),
    ("∏", "Product", "\\prod_{⬚}^{⬚}"),
    ("∫", "Integral", "\\int_{⬚}^{⬚}"),
    ("∬", "Double integral", "\\iint_{⬚}"),
    ("∮", "Contour integral", "\\oint_{⬚}"),
    ("⋃", "Union", "\\bigcup_{⬚}^{⬚}"),
    ("⋂", "Intersection", "\\bigcap_{⬚}^{⬚}"),
    ("lim", "Limit", "\\lim_{⬚}"),
];
pub const BRACKETS: &[Template] = &[
    ("( )", "Parentheses", "\\left( ⬚\\right) "),
    ("[ ]", "Square brackets", "\\left[ ⬚\\right] "),
    ("{ }", "Braces", "\\left\\{ ⬚\\right\\} "),
    ("⟨ ⟩", "Angle brackets", "\\left\\langle ⬚\\right\\rangle "),
    ("| |", "Bars", "\\left| ⬚\\right| "),
    ("⌊ ⌋", "Floor", "\\left\\lfloor ⬚\\right\\rfloor "),
];
pub const MATRICES: &[Template] = &[
    (
        "2×2",
        "Matrix of 2 by 2",
        "\\matrix ⬚ & ⬚ \\\\ ⬚ & ⬚\\endmatrix ",
    ),
    (
        "3×3",
        "Matrix of 3 by 3",
        "\\matrix ⬚ & ⬚ & ⬚ \\\\ ⬚ & ⬚ & ⬚ \\\\ ⬚ & ⬚ & ⬚\\endmatrix ",
    ),
    (
        "[2×2]",
        "Matrix of 2 by 2 in brackets",
        "\\left[ \\matrix ⬚ & ⬚ \\\\ ⬚ & ⬚\\endmatrix \\right] ",
    ),
    (
        "det",
        "Determinant of 2 by 2",
        "\\left| \\matrix ⬚ & ⬚ \\\\ ⬚ & ⬚\\endmatrix \\right| ",
    ),
    (
        "(2×1)",
        "Column vector",
        "\\left( \\matrix ⬚ \\\\ ⬚\\endmatrix \\right) ",
    ),
];
pub const DECORATIONS: &[Template] = &[
    ("x̄", "Bar over", "\\overline{⬚}"),
    ("x̲", "Bar under", "\\underline{⬚}"),
    ("x̂", "Hat", "\\hat{⬚}"),
    ("x⃗", "Vector", "\\vec{⬚}"),
    ("x̃", "Tilde", "\\tilde{⬚}"),
    ("ẋ", "Dot", "\\dot{⬚}"),
    ("⏞", "Brace over", "\\overbrace{⬚}"),
    ("⏟", "Brace under", "\\underbrace{⬚}"),
    ("x↑", "Text over", "\\overset{⬚}\\to{⬚}"),
];
pub const GREEK: &[Template] = &[
    ("α", "alpha", "\\alpha "),
    ("β", "beta", "\\beta "),
    ("γ", "gamma", "\\gamma "),
    ("δ", "delta", "\\delta "),
    ("ε", "epsilon", "\\epsilon "),
    ("θ", "theta", "\\theta "),
    ("λ", "lambda", "\\lambda "),
    ("μ", "mu", "\\mu "),
    ("π", "pi", "\\pi "),
    ("ρ", "rho", "\\rho "),
    ("σ", "sigma", "\\sigma "),
    ("φ", "phi", "\\phi "),
    ("ω", "omega", "\\omega "),
    ("Γ", "Gamma", "\\Gamma "),
    ("Δ", "Delta", "\\Delta "),
    ("Θ", "Theta", "\\Theta "),
    ("Λ", "Lambda", "\\Lambda "),
    ("Π", "Pi", "\\Pi "),
    ("Σ", "Sigma", "\\Sigma "),
    ("Φ", "Phi", "\\Phi "),
    ("Ψ", "Psi", "\\Psi "),
    ("Ω", "Omega", "\\Omega "),
];
pub const SYMBOLS: &[Template] = &[
    ("±", "plus or minus", "\\pm "),
    ("×", "times", "\\times "),
    ("÷", "divided by", "\\div "),
    ("·", "dot", "\\cdot "),
    ("∘", "circle", "\\circ "),
    ("∪", "union", "\\cup "),
    ("∩", "intersection", "\\cap "),
    ("∞", "infinity", "\\infty "),
    ("∂", "partial", "\\partial "),
    ("∇", "nabla", "\\nabla "),
    ("∀", "for all", "\\forall "),
    ("∃", "there exists", "\\exists "),
    ("…", "dots", "\\ldots "),
    ("ℏ", "h bar", "\\hbar "),
];
pub const RELATIONS: &[Template] = &[
    ("≤", "less or equal", "\\leq "),
    ("≥", "greater or equal", "\\geq "),
    ("≠", "not equal", "\\neq "),
    ("≈", "approximately", "\\approx "),
    ("≡", "equivalent", "\\equiv "),
    ("∼", "similar", "\\sim "),
    ("∝", "proportional", "\\propto "),
    ("∈", "element of", "\\in "),
    ("⊂", "subset", "\\subset "),
    ("⊆", "subset or equal", "\\subseteq "),
];
pub const ARROWS: &[Template] = &[
    ("→", "right arrow", "\\rightarrow "),
    ("←", "left arrow", "\\leftarrow "),
    ("↔", "left and right arrow", "\\leftrightarrow "),
    ("⇒", "implies", "\\Rightarrow "),
    ("⇔", "if and only if", "\\Leftrightarrow "),
    ("↦", "maps to", "\\mapsto "),
];
pub const FUNCTIONS: &[Template] = &[
    ("sin", "sine", "\\sin "),
    ("cos", "cosine", "\\cos "),
    ("tan", "tangent", "\\tan "),
    ("log", "logarithm", "\\log "),
    ("ln", "natural logarithm", "\\ln "),
    ("exp", "exponential", "\\exp "),
    ("max", "maximum", "\\max "),
    ("min", "minimum", "\\min "),
];

/// Every group, with its title, in the panel's order.
pub const GROUPS: &[(&str, &[Template])] = &[
    ("Fractions", FRACTIONS),
    ("Radicals", RADICALS),
    ("Scripts", SCRIPTS),
    ("Sums, integrals, products", OPERATORS),
    ("Brackets", BRACKETS),
    ("Matrices", MATRICES),
    ("Over and under", DECORATIONS),
    ("Functions", FUNCTIONS),
    ("Greek", GREEK),
    ("Operators", SYMBOLS),
    ("Relations", RELATIONS),
    ("Arrows", ARROWS),
];

// ------------------------------------------------------------------- view

fn section(title: &str) -> Element<'static> {
    widget::text::heading(title.to_owned()).into()
}

fn wide(label: &str, message: Option<Message>) -> Element<'static> {
    widget::button::custom(
        widget::container(widget::text::body(label.to_owned())).center_x(Length::Fill),
    )
    .class(cosmic::theme::Button::Standard)
    .width(Length::Fill)
    .on_press_maybe(message)
    .into()
}

fn palette_group(templates: &'static [Template]) -> Element<'static> {
    const PER_ROW: usize = 7;
    let mut rows: Vec<Element<'static>> = Vec::new();
    for line in templates.chunks(PER_ROW) {
        let buttons = line
            .iter()
            .map(|(label, tip, text)| -> Element<'static> {
                let button = widget::button::custom(
                    widget::container(widget::text::body((*label).to_owned()))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                )
                .class(cosmic::theme::Button::Standard)
                .width(Length::Fixed(38.0))
                .height(Length::Fixed(32.0))
                .padding(0)
                .on_press(send(EquationAction::Insert(text)));
                widget::tooltip(
                    button,
                    widget::text::caption((*tip).to_owned()),
                    widget::tooltip::Position::Bottom,
                )
                .into()
            })
            .collect::<Vec<_>>();
        rows.push(widget::row::with_children(buttons).spacing(2).into());
    }
    widget::column::with_children(rows).spacing(2).into()
}

/// The rail panel: the palette and the options.
pub fn panel(state: &EquationPanel) -> Element<'_> {
    let mut body: Vec<Element<'_>> = vec![
        widget::row::with_children(vec![
            widget::text::heading("Equation").into(),
            widget::space::horizontal().into(),
            widget::button::icon(widget::icon::from_name("window-close-symbolic"))
                .extra_small()
                .name("Close the Equation panel")
                .on_press(send(EquationAction::Close))
                .into(),
        ])
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    ];
    if let Some(notice) = &state.notice {
        body.push(widget::text::caption(notice.clone()).into());
    }
    if state.unreadable {
        body.push(
            widget::text::body(
                "This equation's text is not in a form the app can read; it is kept as it is.",
            )
            .into(),
        );
        return finish(body);
    }
    body.push(
        widget::text::caption(
            "Type the equation in the card on the page. The buttons put a template at the caret.",
        )
        .into(),
    );
    for (title, templates) in GROUPS {
        body.push(section(title));
        body.push(palette_group(templates));
    }
    body.push(widget::divider::horizontal::default().into());
    body.push(section("Face and size"));
    let faces: Vec<Element<'_>> = FACES
        .iter()
        .enumerate()
        .map(|(index, face)| {
            widget::radio(
                widget::text::body((*face).to_owned()),
                index,
                FACES.iter().position(|candidate| *candidate == state.face),
                |chosen| send(EquationAction::Face(chosen)),
            )
            .into()
        })
        .collect();
    body.push(widget::column::with_children(faces).spacing(2).into());
    let size = widget::text_input("12", state.size.as_str())
        .on_input(|text| send(EquationAction::Size(text)))
        .on_submit(|_| send(EquationAction::SizeSubmit))
        .on_unfocus(send(EquationAction::SizeSubmit))
        .width(Length::Fixed(72.0));
    body.push(
        widget::row::with_children(vec![
            widget::text::body("Size (points)").into(),
            size.into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center)
        .into(),
    );
    if state.size_error {
        body.push(
            widget::text::caption("The size is 1 to 400 points")
                .class(cosmic::theme::Text::Accent)
                .into(),
        );
    }
    body.push(widget::divider::horizontal::default().into());
    body.push(section("Files and LaTeX"));
    body.push(wide("Import Equation…", Some(send(EquationAction::Import))));
    body.push(wide(
        "Save As Equation…",
        Some(send(EquationAction::SaveAs)),
    ));
    body.push(wide("Copy as LaTeX", Some(send(EquationAction::CopyLatex))));
    body.push(wide("Paste LaTeX", Some(send(EquationAction::PasteLatex))));
    finish(body)
}

fn finish<'a>(body: Vec<Element<'a>>) -> Element<'a> {
    widget::scrollable(
        widget::column::with_children(body)
            .spacing(6)
            .padding([8, 8]),
    )
    .height(Length::Fill)
    .into()
}

/// The card at the frame: the source field and, under it, the error.
pub fn card(state: &EquationPanel, width: f32) -> Element<'_> {
    let editor = text_editor(&state.source)
        .id(SOURCE_ID)
        .placeholder("Type TeX, for example \\frac{a}{b}")
        .on_action(|action| send(EquationAction::Edit(action)))
        .key_binding(|press| {
            use cosmic::iced::keyboard::{Key, key::Named};
            if matches!(press.key, Key::Named(Named::Enter))
                && !press.modifiers.shift()
                && matches!(press.status, text_editor::Status::Focused { .. })
            {
                return Some(text_editor::Binding::Custom(send(EquationAction::Done)));
            }
            text_editor::Binding::from_key_press(press)
        })
        .min_height(34.0)
        .max_height(120.0)
        .width(width);
    let mut column: Vec<Element<'_>> = vec![editor.into()];
    if let Some(line) = state.error_line() {
        column.push(
            widget::text::caption(line)
                .class(cosmic::theme::Text::Accent)
                .into(),
        );
    }
    widget::container(widget::column::with_children(column).spacing(4))
        .padding(6)
        .class(cosmic::theme::Container::Card)
        .into()
}
