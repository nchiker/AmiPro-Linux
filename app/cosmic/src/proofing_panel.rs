//! The Proofing panel in the rail (the proofing run; `docs/Design/UI_SPEC.md`
//! §35, SP3): Ami Pro's Spell Check, Thesaurus, and Grammar Check as pages
//! of one non-modal panel, the document live beside it. Its behaviour is in
//! `proofing_commands.rs`.

use std::sync::Arc;

use amipro_document::FlowId;
use amipro_editing::TextPosition;
use amipro_editing::proofing::spelling::{Problem, ProblemKind};
use cosmic::iced::Length;
use cosmic::widget;

use crate::Message;

pub const WIDTH: f32 = 300.0;

type Element<'a> = cosmic::Element<'a, Message>;

/// The panel's pages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    Spelling,
    Thesaurus,
    Grammar,
}

/// Where a spell check is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpellState {
    /// The dictionary is loading; the check starts when it arrives.
    Loading,
    /// A problem is shown.
    Checking,
    /// Ami Pro's "Spell check complete.": nothing more to the end.
    Complete,
    /// No dictionary for the document's language: the package to install.
    NoDictionary(String),
    /// The dictionary could not be read.
    Failed(String),
}

/// A spell check under way (Ami Pro's correction dialog's state).
#[derive(Clone, Debug)]
pub struct SpellCheck {
    pub state: SpellState,
    /// The streams the check goes through, in Ami Pro's order.
    pub streams: Vec<FlowId>,
    /// Where the next search starts.
    pub from: TextPosition,
    /// A selection's end: the check stops there (Ami Pro checks only a
    /// selection, PSP2).
    pub until: Option<TextPosition>,
    pub problem: Option<Problem>,
    /// Replace with, as typed or chosen.
    pub replace_with: String,
    pub alternatives: Vec<String>,
    pub chosen: Option<usize>,
    /// Words found and replaced so far, for the panel's count.
    pub found: usize,
    pub replaced: usize,
}

/// The Thesaurus page: Ami Pro's dialog (dialog 1093) as state.
#[derive(Clone, Debug, Default)]
pub struct ThesaurusLookup {
    /// Word looked up.
    pub looked_up: String,
    /// The word's place in the document, which Replace replaces.
    pub place: Option<(TextPosition, TextPosition)>,
    pub entry: Option<amipro_editing::proofing::thesaurus::Entry>,
    /// The meaning chosen, and the synonym chosen in it.
    pub meaning: usize,
    pub chosen: Option<usize>,
    pub replace_with: String,
    /// Words looked up before (Previous).
    pub history: Vec<String>,
    /// "x is not in the thesaurus.", or why there is no thesaurus.
    pub message: Option<String>,
    pub loading: bool,
}

/// Paragraph texts' grammar problems from the worker: (the text's hash and
/// the set) and the problems.
pub type GrammarResults = Vec<((u64, String), Vec<crate::grammar::GrammarProblem>)>;

/// Where a grammar check is.
#[derive(Clone, Debug, PartialEq)]
pub enum GrammarState {
    Loading,
    Checking,
    /// Ami Pro's end, with its Readability Statistics when they show.
    Complete,
    /// Grammar is English only (SP8).
    NotEnglish,
}

/// A grammar problem found by the walk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrammarFound {
    pub start: TextPosition,
    pub end: TextPosition,
    /// Its sentence's bytes, for Next Sentence and the rewritten options.
    pub sentence: std::ops::Range<usize>,
    pub sentence_text: String,
    pub problem: crate::grammar::GrammarProblem,
}

/// A grammar check under way (Ami Pro's Grammar Checker's state).
#[derive(Clone, Debug)]
pub struct GrammarCheck {
    pub state: GrammarState,
    pub streams: Vec<FlowId>,
    pub from: TextPosition,
    pub until: Option<TextPosition>,
    /// Where the check began, for the statistics' range.
    pub began: TextPosition,
    pub found: Option<GrammarFound>,
    pub chosen: usize,
    /// Skip Rule: the rules skipped for this check.
    pub skipped: std::collections::BTreeSet<String>,
    pub statistics: Option<amipro_editing::proofing::readability::Statistics>,
    pub problems: usize,
    pub replaced: usize,
}

/// Ami Pro's Grammar Check preferences (dialog 1092), as its `GrammarOpts`
/// bits: 1 statistics, 2 from beginning, 4 other text streams, 8
/// explanations, 16 draft mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GrammarOption {
    Statistics,
    FromBeginning,
    OtherStreams,
    Explanations,
}

impl GrammarOption {
    pub fn bit(self) -> u32 {
        match self {
            GrammarOption::Statistics => 1,
            GrammarOption::FromBeginning => 2,
            GrammarOption::OtherStreams => 4,
            GrammarOption::Explanations => 8,
        }
    }
}

/// Ami Pro's six options, by name, for the panel's check boxes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpellOption {
    FromBeginning,
    OtherStreams,
    RepeatedWords,
    WordsWithNumbers,
    InitialCaps,
    UserAlternatives,
}

/// The word under the right button: its place, what it is, and what the
/// menu offers for it.
#[derive(Clone, Debug)]
pub struct ContextWord {
    pub start: TextPosition,
    pub end: TextPosition,
    pub word: String,
    /// Underlined: misspelled or repeated.
    pub flagged: bool,
    pub repeated: bool,
    pub suggestions: Vec<String>,
    /// The first meaning's synonyms (Synonyms ▸).
    pub synonyms: Vec<String>,
}

/// The right-click menu's proofing entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextEntry {
    Suggestion(u8),
    DeleteRepeat,
    IgnoreAll,
    AddToDictionary,
    Synonym(u8),
}

/// What the panel keeps between messages.
pub struct ProofingPanel {
    pub page: Page,
    pub spelling: Option<SpellCheck>,
    pub thesaurus: ThesaurusLookup,
    pub grammar: Option<GrammarCheck>,
    /// Ami Pro's grammar sets' names, for the drop-down.
    pub grammar_sets: Vec<String>,
    /// The readability statistics Tools › Readability Statistics… shows.
    pub statistics: Option<amipro_editing::proofing::readability::Statistics>,
    pub show_options: bool,
    pub show_dictionary: bool,
    /// The user dictionary's add field.
    pub user_word: String,
    /// The languages offered: (locale, name), and their names for the
    /// drop-down.
    pub languages: Vec<(String, String)>,
    pub language_names: Vec<String>,
}

impl ProofingPanel {
    pub fn new(page: Page, languages: Vec<(String, String)>) -> Self {
        let language_names = languages.iter().map(|(_, name)| name.clone()).collect();
        Self {
            page,
            spelling: None,
            thesaurus: ThesaurusLookup::default(),
            grammar: None,
            grammar_sets: crate::grammar::SETS
                .iter()
                .map(|name| name.to_string())
                .collect(),
            statistics: None,
            show_options: false,
            show_dictionary: false,
            user_word: String::new(),
            languages,
            language_names,
        }
    }
}

#[derive(Clone, Debug)]
pub enum ProofingAction {
    Close,
    Page(Page),
    // Spelling.
    Start,
    ReplaceWith(String),
    Choose(usize),
    Replace,
    ReplaceAll,
    Skip,
    SkipAll,
    AddToDictionary,
    SpellOption(SpellOption, bool),
    ToggleOptions,
    ToggleDictionary,
    Language(usize),
    /// The document's language becomes new documents' (Ami Pro's Default
    /// language for new documents).
    DefaultLanguage,
    UserWord(String),
    AddUserWord,
    RemoveUserWord(String),
    ImportDictionary,
    ExportDictionary,
    DictionaryFile {
        import: bool,
        result: crate::FileDialogResult,
    },
    DictionaryLoaded(String, Result<Arc<crate::spelling::Dictionary>, String>),
    // Thesaurus.
    LookUpCaret,
    Meaning(usize),
    Synonym(usize),
    ThesaurusReplaceWith(String),
    LookUp,
    Previous,
    ThesaurusReplace,
    ThesaurusLoaded(String, Result<Arc<crate::spelling::Thesaurus>, String>),
    // Grammar.
    StartGrammar,
    GrammarSet(usize),
    GrammarOption(GrammarOption, bool),
    GrammarAsYouType(bool),
    ChooseReplacement(usize),
    GrammarReplace,
    GrammarSkip,
    SkipRule,
    NextSentence,
    Statistics,
    GrammarLoaded(Arc<crate::grammar::Grammar>),
    /// Grammar underlines worked out on a worker thread: the session, its
    /// change's generation, and each paragraph text's problems by key.
    GrammarChecked {
        session: u64,
        generation: u64,
        found: Arc<GrammarResults>,
    },
    /// A right-click menu entry on the word under the pointer.
    Context(ContextEntry),
    /// The pause after a change: the session and the change's generation.
    Idle {
        session: u64,
        generation: u64,
    },
}

fn send(action: ProofingAction) -> Message {
    Message::Proofing(action)
}

/// What the Spelling page shows besides the panel's state.
pub struct SpellingShown<'a> {
    pub options: amipro_editing::proofing::spelling::SpellOptions,
    /// The stream the current word is in, as Ami Pro's status names it.
    pub stream: &'static str,
    /// The document's language's index in the panel's list.
    pub language: Option<usize>,
    pub language_name: String,
    /// Whether the document's language is new documents' already.
    pub is_default_language: bool,
    pub user_words: Vec<&'a str>,
}

fn header(title: &'static str) -> Element<'static> {
    widget::row::with_children(vec![
        widget::text::heading(title).into(),
        widget::space::horizontal().into(),
        widget::button::icon(widget::icon::from_name("window-close-symbolic"))
            .extra_small()
            .name("Close Proofing")
            .on_press(send(ProofingAction::Close))
            .into(),
    ])
    .align_y(cosmic::iced::Alignment::Center)
    .into()
}

fn row(children: Vec<Element<'_>>) -> Element<'_> {
    widget::row::with_children(children)
        .spacing(6)
        .align_y(cosmic::iced::Alignment::Center)
        .into()
}

/// A full-width button (libcosmic's text buttons size their label only).
fn wide(label: &str, message: Option<Message>) -> Element<'static> {
    widget::button::custom(
        widget::container(widget::text::body(label.to_owned())).center_x(Length::Fill),
    )
    .class(cosmic::theme::Button::Standard)
    .width(Length::Fill)
    .on_press_maybe(message)
    .into()
}

/// A section that folds: its title as a toggle.
fn section<'a>(
    title: String,
    open: bool,
    toggle: ProofingAction,
    body: Vec<Element<'a>>,
) -> Element<'a> {
    let mut children: Vec<Element<'a>> = vec![
        widget::button::custom(widget::text::body(format!(
            "{} {title}",
            if open { "▾" } else { "▸" }
        )))
        .class(cosmic::theme::Button::Text)
        .width(Length::Fill)
        .on_press(send(toggle))
        .into(),
    ];
    if open {
        children.push(
            widget::column::with_children(body)
                .spacing(6)
                .padding([0, 0, 0, 12])
                .into(),
        );
    }
    widget::column::with_children(children).spacing(4).into()
}

/// The page selector: a segmented row of the panel's pages.
fn pages(page: Page) -> Element<'static> {
    let button = |label: &'static str, this: Page| -> Element<'static> {
        let selected = page == this;
        widget::button::custom(widget::container(widget::text::body(label)).center_x(Length::Fill))
            .class(if selected {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            })
            .width(Length::Fill)
            .on_press(send(ProofingAction::Page(this)))
            .into()
    };
    widget::row::with_children(vec![
        button("Spelling", Page::Spelling),
        button("Thesaurus", Page::Thesaurus),
        button("Grammar", Page::Grammar),
    ])
    .spacing(2)
    .into()
}

pub fn spelling_page<'a>(state: &'a ProofingPanel, shown: SpellingShown<'a>) -> Element<'a> {
    let mut body: Vec<Element<'a>> = vec![pages(state.page)];
    match state.spelling.as_ref() {
        None => body.push(wide("Start Spell Check", Some(send(ProofingAction::Start)))),
        Some(check) => match &check.state {
            SpellState::Loading => {
                body.push(widget::text::body("Loading the dictionary…").into());
            }
            SpellState::NoDictionary(package) => {
                body.push(
                    widget::text::body(format!(
                        "No spelling dictionary is installed for {}. Install the {package} package, or choose another language below.",
                        shown.language_name
                    ))
                    .into(),
                );
            }
            SpellState::Failed(error) => {
                body.push(
                    widget::text::body(format!("The dictionary could not be read: {error}")).into(),
                );
            }
            SpellState::Complete => {
                body.push(widget::text::heading("Spell check complete.").into());
                body.push(
                    widget::text::caption(match (check.found, check.replaced) {
                        (0, _) => "No words were found to correct.".to_owned(),
                        (found, 0) => format!("{found} found."),
                        (found, replaced) => format!("{found} found, {replaced} replaced."),
                    })
                    .into(),
                );
                body.push(wide("Check Again", Some(send(ProofingAction::Start))));
            }
            SpellState::Checking => body.extend(problem(check, shown.stream)),
        },
    }
    // Ami Pro's options (dialogs 1099 and 1030).
    let option = |label: &'static str, which: SpellOption, on: bool| -> Element<'a> {
        widget::checkbox(on)
            .label(label)
            .on_toggle(move |on| send(ProofingAction::SpellOption(which, on)))
            .into()
    };
    let options = shown.options;
    body.push(section(
        "Options".into(),
        state.show_options,
        ProofingAction::ToggleOptions,
        vec![
            option(
                "Check from the beginning",
                SpellOption::FromBeginning,
                options.from_beginning,
            ),
            option(
                "Include headers, footers, frames, and footnotes",
                SpellOption::OtherStreams,
                options.other_streams,
            ),
            option(
                "Check for repeated words",
                SpellOption::RepeatedWords,
                options.repeated_words,
            ),
            option(
                "Check words with numbers",
                SpellOption::WordsWithNumbers,
                options.words_with_numbers,
            ),
            option(
                "Check words with initial capitals",
                SpellOption::InitialCaps,
                options.initial_caps,
            ),
            option(
                "Include user dictionary alternatives",
                SpellOption::UserAlternatives,
                options.user_alternatives,
            ),
        ],
    ));
    body.push(row(vec![
        widget::text::body("Language").into(),
        widget::dropdown(&state.language_names, shown.language, |index| {
            send(ProofingAction::Language(index))
        })
        .width(Length::Fill)
        .into(),
    ]));
    body.push(
        widget::checkbox(shown.is_default_language)
            .label("Use for new documents")
            .on_toggle_maybe(
                (!shown.is_default_language).then_some(|_| send(ProofingAction::DefaultLanguage)),
            )
            .into(),
    );
    let mut dictionary: Vec<Element<'a>> = vec![row(vec![
        widget::text_input("Add a word", state.user_word.as_str())
            .on_input(|text| send(ProofingAction::UserWord(text)))
            .on_submit(|_| send(ProofingAction::AddUserWord))
            .width(Length::Fill)
            .into(),
        widget::button::text("Add")
            .on_press_maybe(
                (!state.user_word.trim().is_empty()).then(|| send(ProofingAction::AddUserWord)),
            )
            .into(),
    ])];
    if shown.user_words.is_empty() {
        dictionary.push(widget::text::caption("No words added yet.").into());
    }
    for word in &shown.user_words {
        dictionary.push(row(vec![
            widget::text::body(word.to_string())
                .width(Length::Fill)
                .into(),
            widget::button::icon(widget::icon::from_name("edit-delete-symbolic"))
                .extra_small()
                .name(format!("Remove {word}"))
                .on_press(send(ProofingAction::RemoveUserWord(word.to_string())))
                .into(),
        ]));
    }
    dictionary.push(row(vec![
        widget::button::text("Import…")
            .on_press(send(ProofingAction::ImportDictionary))
            .into(),
        widget::button::text("Export…")
            .on_press_maybe(
                (!shown.user_words.is_empty()).then(|| send(ProofingAction::ExportDictionary)),
            )
            .into(),
    ]));
    body.push(section(
        format!("User dictionary ({} words)", shown.user_words.len()),
        state.show_dictionary,
        ProofingAction::ToggleDictionary,
        dictionary,
    ));
    wrap(header("Proofing"), body)
}

/// The Thesaurus page: Ami Pro's dialog in the rail (SP7).
pub fn thesaurus_page<'a>(state: &'a ProofingPanel) -> Element<'a> {
    let lookup = &state.thesaurus;
    let mut body: Vec<Element<'a>> = vec![pages(state.page)];
    if lookup.loading {
        body.push(widget::text::body("Loading the thesaurus…").into());
    }
    if let Some(message) = &lookup.message {
        body.push(widget::text::body(message.clone()).into());
    }
    if !lookup.looked_up.is_empty() {
        body.push(widget::text::caption("Word looked up:").into());
        body.push(widget::text::heading(lookup.looked_up.clone()).into());
    }
    body.push(row(vec![
        widget::text_input("Replace with", lookup.replace_with.as_str())
            .on_input(|text| send(ProofingAction::ThesaurusReplaceWith(text)))
            .on_submit(|_| send(ProofingAction::LookUp))
            .width(Length::Fill)
            .into(),
        widget::button::text("Look Up")
            .on_press_maybe(
                (!lookup.replace_with.trim().is_empty()).then(|| send(ProofingAction::LookUp)),
            )
            .into(),
    ]));
    if let Some(entry) = &lookup.entry {
        body.push(widget::text::caption("Meanings:").into());
        let meanings: Vec<Element<'a>> = entry
            .meanings
            .iter()
            .enumerate()
            .map(|(index, meaning)| {
                let shown: Vec<&str> = meaning
                    .synonyms
                    .iter()
                    .take(3)
                    .map(String::as_str)
                    .collect();
                widget::button::custom(widget::text::body(format!(
                    "{}) {}  {}",
                    index + 1,
                    meaning.part_of_speech,
                    shown.join(", ")
                )))
                .class(if index == lookup.meaning {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Text
                })
                .width(Length::Fill)
                .on_press(send(ProofingAction::Meaning(index)))
                .into()
            })
            .collect();
        body.push(
            widget::container(widget::column::with_children(meanings))
                .padding(2)
                .class(cosmic::theme::Container::Card)
                .into(),
        );
        if let Some(meaning) = entry.meanings.get(lookup.meaning) {
            body.push(widget::text::caption("Synonyms:").into());
            let synonyms: Vec<Element<'a>> = meaning
                .synonyms
                .iter()
                .enumerate()
                .map(|(index, synonym)| {
                    widget::button::custom(widget::text::body(synonym.clone()))
                        .class(if lookup.chosen == Some(index) {
                            cosmic::theme::Button::Suggested
                        } else {
                            cosmic::theme::Button::Text
                        })
                        .width(Length::Fill)
                        .on_press(send(ProofingAction::Synonym(index)))
                        .into()
                })
                .collect();
            body.push(
                widget::container(widget::column::with_children(synonyms))
                    .padding(2)
                    .class(cosmic::theme::Container::Card)
                    .into(),
            );
        }
    }
    body.push(row(vec![
        wide(
            "Replace",
            (lookup.place.is_some()
                && !lookup.replace_with.trim().is_empty()
                && lookup.replace_with != lookup.looked_up)
                .then(|| send(ProofingAction::ThesaurusReplace)),
        ),
        wide(
            "Previous",
            (!lookup.history.is_empty()).then(|| send(ProofingAction::Previous)),
        ),
    ]));
    body.push(wide(
        "Look Up the Word at the Caret",
        Some(send(ProofingAction::LookUpCaret)),
    ));
    wrap(header("Proofing"), body)
}

/// What the Grammar page shows besides the panel's state.
pub struct GrammarShown {
    pub set: Option<usize>,
    pub options: u32,
    pub as_you_type: bool,
}

/// Ami Pro's Readability Statistics (dialog 3053) as rows.
fn statistics_rows<'a>(figures: &amipro_editing::proofing::readability::Statistics) -> Element<'a> {
    let line = |label: &str, value: String| -> Element<'a> {
        row(vec![
            widget::text::body(label.to_owned())
                .width(Length::Fill)
                .into(),
            widget::text::body(value).into(),
        ])
    };
    widget::container(
        widget::column::with_children(vec![
            widget::text::heading("Readability statistics").into(),
            line("Words", figures.words.to_string()),
            line("Sentences", figures.sentences.to_string()),
            line("Paragraphs", figures.paragraphs.to_string()),
            line("Syllables", figures.syllables.to_string()),
            line("3-syllable words", figures.three_syllable_words.to_string()),
            line(
                "Words per sentence",
                format!("{:.1}", figures.words_per_sentence()),
            ),
            line(
                "Sentences per paragraph",
                format!("{:.1}", figures.sentences_per_paragraph()),
            ),
            line(
                "Passive sentences",
                format!("{:.0}%", figures.passive_percent()),
            ),
            line("Gunning's Fog Index", format!("{:.1}", figures.fog_index())),
            line(
                "Flesch-Kincaid Score",
                format!("{:.1}", figures.flesch_kincaid()),
            ),
            line(
                "Flesch Reading Ease Score",
                format!("{:.1}", figures.reading_ease()),
            ),
            line(
                "Flesch Reading Ease Grade Level",
                format!("{:.1}", figures.reading_ease_grade()),
            ),
        ])
        .spacing(2),
    )
    .padding(6)
    .class(cosmic::theme::Container::Card)
    .into()
}

/// The Grammar page: Ami Pro's Grammar Check and Grammar Checker (SP8).
pub fn grammar_page<'a>(state: &'a ProofingPanel, shown: GrammarShown) -> Element<'a> {
    let mut body: Vec<Element<'a>> = vec![pages(state.page)];
    body.push(row(vec![
        widget::text::body("Set").into(),
        widget::dropdown(&state.grammar_sets, shown.set, |index| {
            send(ProofingAction::GrammarSet(index))
        })
        .width(Length::Fill)
        .into(),
    ]));
    let explanations = shown.options & GrammarOption::Explanations.bit() != 0;
    match state.grammar.as_ref() {
        None => body.push(wide(
            "Start Grammar Check",
            Some(send(ProofingAction::StartGrammar)),
        )),
        Some(check) => match &check.state {
            GrammarState::Loading => {
                body.push(widget::text::body("Loading the grammar checker…").into())
            }
            GrammarState::NotEnglish => body.push(
                widget::text::body("Grammar Check is available for English documents only.").into(),
            ),
            GrammarState::Complete => {
                body.push(widget::text::heading("Grammar check complete.").into());
                body.push(
                    widget::text::caption(match (check.problems, check.replaced) {
                        (0, _) => "No problems were found.".to_owned(),
                        (found, 0) => format!("{found} found."),
                        (found, replaced) => format!("{found} found, {replaced} replaced."),
                    })
                    .into(),
                );
                if let Some(figures) = &check.statistics {
                    body.push(statistics_rows(figures));
                }
                body.push(wide(
                    "Check Again",
                    Some(send(ProofingAction::StartGrammar)),
                ));
            }
            GrammarState::Checking => {
                if let Some(found) = &check.found {
                    let problem = &found.problem;
                    body.push(widget::text::caption("Suggestion:").into());
                    body.push(widget::text::body(problem.message.clone()).into());
                    body.push(
                        widget::text::caption(format!(
                            "“{}”",
                            found
                                .sentence_text
                                .get(
                                    problem.range.start.saturating_sub(found.sentence.start)
                                        ..problem.range.end.saturating_sub(found.sentence.start)
                                )
                                .unwrap_or_default()
                        ))
                        .into(),
                    );
                    if explanations {
                        body.push(
                            widget::text::caption(crate::grammar::explanation(&problem.kind))
                                .into(),
                        );
                    }
                    if problem.replacements.is_empty() {
                        body.push(widget::text::caption("Sentence:").into());
                        body.push(widget::text::body(found.sentence_text.clone()).into());
                    } else {
                        body.push(widget::text::caption("Replacement options:").into());
                        let options: Vec<Element<'a>> = problem
                            .replacements
                            .iter()
                            .enumerate()
                            .map(|(index, replacement)| {
                                // The sentence rewritten, as Ami Pro lists them.
                                let at = problem.range.start - found.sentence.start;
                                let to = problem.range.end - found.sentence.start;
                                let rewritten = format!(
                                    "{}{}{}",
                                    &found.sentence_text[..at.min(found.sentence_text.len())],
                                    replacement,
                                    &found.sentence_text[to.min(found.sentence_text.len())..]
                                );
                                widget::button::custom(widget::text::body(
                                    rewritten.trim().to_owned(),
                                ))
                                .class(if check.chosen == index {
                                    cosmic::theme::Button::Suggested
                                } else {
                                    cosmic::theme::Button::Text
                                })
                                .width(Length::Fill)
                                .on_press(send(ProofingAction::ChooseReplacement(index)))
                                .into()
                            })
                            .collect();
                        body.push(
                            widget::container(widget::column::with_children(options))
                                .padding(2)
                                .class(cosmic::theme::Container::Card)
                                .into(),
                        );
                    }
                    body.push(row(vec![
                        wide(
                            "Replace",
                            (!problem.replacements.is_empty())
                                .then(|| send(ProofingAction::GrammarReplace)),
                        ),
                        wide("Skip", Some(send(ProofingAction::GrammarSkip))),
                    ]));
                    body.push(row(vec![
                        wide("Skip Rule", Some(send(ProofingAction::SkipRule))),
                        wide("Next Sentence", Some(send(ProofingAction::NextSentence))),
                    ]));
                    body.push(widget::text::caption(format!("{} found", check.problems)).into());
                }
            }
        },
    }
    if let Some(figures) = &state.statistics
        && state
            .grammar
            .as_ref()
            .is_none_or(|check| check.state != GrammarState::Complete)
    {
        body.push(statistics_rows(figures));
    }
    let option = |label: &'static str, which: GrammarOption| -> Element<'a> {
        widget::checkbox(shown.options & which.bit() != 0)
            .label(label)
            .on_toggle(move |on| send(ProofingAction::GrammarOption(which, on)))
            .into()
    };
    body.push(section(
        "Options".into(),
        state.show_options,
        ProofingAction::ToggleOptions,
        vec![
            option("Show readability statistics", GrammarOption::Statistics),
            option("Show explanations", GrammarOption::Explanations),
            option("Check from the beginning", GrammarOption::FromBeginning),
            option(
                "Include headers, footers, frames, and footnotes",
                GrammarOption::OtherStreams,
            ),
            widget::checkbox(shown.as_you_type)
                .label("Underline problems as you type")
                .on_toggle(|on| send(ProofingAction::GrammarAsYouType(on)))
                .into(),
        ],
    ));
    body.push(wide(
        "Readability Statistics",
        Some(send(ProofingAction::Statistics)),
    ));
    wrap(header("Proofing"), body)
}

/// The current word: Ami Pro's correction dialog.
fn problem<'a>(check: &'a SpellCheck, stream: &'static str) -> Vec<Element<'a>> {
    let mut body: Vec<Element<'a>> = Vec::new();
    let Some(problem) = check.problem.as_ref() else {
        return body;
    };
    let repeated = problem.kind == ProblemKind::Repeated;
    body.push(
        widget::text::caption(if repeated {
            "Repeated word:"
        } else {
            "Not in the dictionary:"
        })
        .into(),
    );
    body.push(widget::text::heading(problem.word.clone()).into());
    body.push(
        widget::text_input("Replace with", check.replace_with.as_str())
            .on_input(|text| send(ProofingAction::ReplaceWith(text)))
            .on_submit(|_| send(ProofingAction::Replace))
            .width(Length::Fill)
            .into(),
    );
    if !repeated {
        if check.alternatives.is_empty() {
            body.push(widget::text::caption("No alternative found.").into());
        } else {
            let list: Vec<Element<'a>> = check
                .alternatives
                .iter()
                .enumerate()
                .map(|(index, alternative)| {
                    widget::button::custom(widget::text::body(alternative.clone()))
                        .class(if check.chosen == Some(index) {
                            cosmic::theme::Button::Suggested
                        } else {
                            cosmic::theme::Button::Text
                        })
                        .width(Length::Fill)
                        .on_press(send(ProofingAction::Choose(index)))
                        .into()
                })
                .collect();
            body.push(
                widget::container(widget::column::with_children(list).spacing(0))
                    .padding(2)
                    .class(cosmic::theme::Container::Card)
                    .into(),
            );
        }
    }
    // Replace and Replace All wait until Replace with differs (Ami Pro's
    // dialog), except a repeat, whose empty Replace with deletes it.
    let changed = check.replace_with != problem.word || repeated;
    let replace_label = if repeated && check.replace_with.is_empty() {
        "Delete Repeat"
    } else {
        "Replace"
    };
    body.push(row(vec![
        wide(
            replace_label,
            changed.then(|| send(ProofingAction::Replace)),
        ),
        wide(
            "Replace All",
            (changed && !repeated && !check.replace_with.is_empty())
                .then(|| send(ProofingAction::ReplaceAll)),
        ),
    ]));
    body.push(row(vec![
        wide("Skip", Some(send(ProofingAction::Skip))),
        wide("Skip All", Some(send(ProofingAction::SkipAll))),
    ]));
    if !repeated {
        body.push(wide(
            "Add to Dictionary",
            Some(send(ProofingAction::AddToDictionary)),
        ));
    }
    body.push(widget::text::caption(format!("Checking {stream} · {} found", check.found)).into());
    body
}

fn wrap<'a>(title: Element<'a>, body: Vec<Element<'a>>) -> Element<'a> {
    widget::column::with_children(vec![
        title,
        crate::rail_scroller(widget::column::with_children(body).spacing(8).into()),
    ])
    .spacing(8)
    .padding([8, 8, 8, 8])
    .height(Length::Fill)
    .into()
}
