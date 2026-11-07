//! Grammar (`UI_SPEC.md` §35, SP8): Harper's checker behind Ami Pro's
//! eight grammar and style sets (`docs/research/PROOFING_NOTES.md`,
//! "Grammar": what each set turns off, decoded from `[Grammar]`'s masks).
//! Harper's lints come in categories; each Ami Pro set keeps the
//! categories that answer to the rules it keeps.

use std::ops::Range;
use std::sync::{Arc, Mutex};

use harper_core::linting::{LintGroup, LintKind, Linter, Suggestion};
use harper_core::spell::FstDictionary;
use harper_core::{Dialect, Document};

/// Ami Pro's sets in its order (VM PGR1), the first its default.
pub const SETS: [&str; 8] = [
    "Business Writing",
    "All Rules",
    "Legal Writing",
    "Technical Writing",
    "Fiction Writing",
    "Academic Writing",
    "Formal Writing",
    "Casual Writing",
];

/// Harper's categories that answer to Ami Pro's grammar rules (noun phrase
/// and verb group consistency, agreement, pronouns, commonly confused
/// words, word usage, double negatives, clause, format, and punctuation
/// errors).
const GRAMMAR: [LintKind; 11] = [
    LintKind::Agreement,
    LintKind::Grammar,
    LintKind::BoundaryError,
    LintKind::Malapropism,
    LintKind::Eggcorn,
    LintKind::Typo,
    LintKind::Punctuation,
    LintKind::Capitalization,
    LintKind::Formatting,
    LintKind::WordOrder,
    LintKind::Repetition,
];

/// Those that answer to its style rules (wordy, redundant, informal,
/// clichés, weak modifiers, jargon, pretentious, archaic, misused words…).
const STYLE: [LintKind; 8] = [
    LintKind::Redundancy,
    LintKind::Usage,
    LintKind::WordChoice,
    LintKind::Nonstandard,
    LintKind::Style,
    LintKind::Enhancement,
    LintKind::Readability,
    LintKind::Regionalism,
];

/// Whether set `name` checks Harper's category `kind`: Ami Pro's sets turn
/// off style rules (Business: wordy, clichés, weak modifiers, jargon,
/// pretentious; Casual and Fiction: most style rules and informal usage).
pub fn checks(name: &str, kind: LintKind) -> bool {
    if kind == LintKind::Spelling {
        return false;
    }
    let grammar = GRAMMAR.contains(&kind);
    let style = STYLE.contains(&kind);
    match name {
        "All Rules" | "Academic Writing" | "Formal Writing" => true,
        "Business Writing" | "Legal Writing" => {
            grammar || !matches!(kind, LintKind::Enhancement | LintKind::Style)
        }
        "Technical Writing" => grammar || kind != LintKind::Regionalism,
        "Fiction Writing" | "Casual Writing" => {
            grammar && !matches!(kind, LintKind::Formatting)
                || kind == LintKind::Redundancy && name == "Fiction Writing"
        }
        _ => grammar || style || kind == LintKind::Miscellaneous,
    }
}

/// Ami Pro's Explanation for a category (its rule explanations are the
/// engine's; these are the app's own words).
pub fn explanation(kind: &str) -> &'static str {
    match kind {
        "Agreement" => {
            "Subjects and verbs, and nouns and the words that go with them, must agree in person and number."
        }
        "Grammar" => "The sentence's grammar looks wrong here.",
        "BoundaryError" => "Words may be joined or split in the wrong place.",
        "Malapropism" | "Eggcorn" => "A word may be confused with one that sounds like it.",
        "Typo" => "This looks like a slip of the fingers.",
        "Punctuation" => "The punctuation may be wrong here.",
        "Capitalization" => "The capitals may be wrong here.",
        "Formatting" => "The spacing or formatting may be wrong here.",
        "WordOrder" => "The words may be in an unusual order.",
        "Repetition" => "A word or phrase is repeated.",
        "Redundancy" => "Some words repeat a meaning already given.",
        "Usage" => "Standard usage prefers another expression.",
        "WordChoice" => "Another word may fit better.",
        "Nonstandard" => "This expression is not standard.",
        "Style" => "A plainer or more usual expression may read better.",
        "Enhancement" => "The sentence could be clearer or stronger.",
        "Readability" => "The sentence may be hard to read: consider shortening it.",
        "Regionalism" => "This is usual in some regions only.",
        _ => "Consider rewording.",
    }
}

/// A problem in a paragraph: its bytes, its category, Harper's message,
/// and the text each replacement option puts in its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrammarProblem {
    pub range: Range<usize>,
    pub kind: String,
    pub message: String,
    pub replacements: Vec<String>,
}

impl GrammarProblem {
    /// Skip Rule's key: the category and message (Harper names no rule).
    pub fn rule(&self) -> String {
        format!("{}: {}", self.kind, self.message)
    }
}

/// Harper's checker, made once (its dictionary and rules take a moment).
pub struct Grammar {
    dictionary: Arc<FstDictionary>,
    linters: Mutex<Vec<(Dialect, LintGroup)>>,
}

impl std::fmt::Debug for Grammar {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Grammar").finish_non_exhaustive()
    }
}

/// The dialect a locale writes.
pub fn dialect(locale: &str) -> Dialect {
    match locale {
        "en_GB" => Dialect::British,
        "en_AU" | "en_NZ" => Dialect::Australian,
        "en_CA" => Dialect::Canadian,
        "en_IN" => Dialect::Indian,
        _ => Dialect::American,
    }
}

impl Grammar {
    pub fn new() -> Self {
        Self {
            dictionary: FstDictionary::curated(),
            linters: Mutex::new(Vec::new()),
        }
    }

    /// The problems set `set` finds in a paragraph's text, in order.
    pub fn check(&self, text: &str, set: &str, dialect: Dialect) -> Vec<GrammarProblem> {
        if text.trim().is_empty() {
            return Vec::new();
        }
        let document = Document::new_plain_english(text, self.dictionary.as_ref());
        let lints = {
            let mut linters = self
                .linters
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let index = match linters.iter().position(|(own, _)| *own == dialect) {
                Some(index) => index,
                None => {
                    linters.push((
                        dialect,
                        LintGroup::new_curated(self.dictionary.clone(), dialect),
                    ));
                    linters.len() - 1
                }
            };
            linters[index].1.lint(&document)
        };
        // Characters to bytes.
        let mut bytes: Vec<usize> = text.char_indices().map(|(at, _)| at).collect();
        bytes.push(text.len());
        let byte = |index: usize| bytes.get(index).copied().unwrap_or(text.len());
        let mut found: Vec<GrammarProblem> = lints
            .into_iter()
            .filter(|lint| checks(set, lint.lint_kind))
            .filter_map(|lint| {
                let range = byte(lint.span.start)..byte(lint.span.end);
                if range.start >= range.end {
                    return None;
                }
                let original = &text[range.clone()];
                let replacements = lint
                    .suggestions
                    .iter()
                    .map(|suggestion| match suggestion {
                        Suggestion::ReplaceWith(chars) => chars.iter().collect(),
                        Suggestion::InsertAfter(chars) => {
                            format!("{original}{}", chars.iter().collect::<String>())
                        }
                        Suggestion::Remove => String::new(),
                    })
                    .filter(|replacement: &String| replacement != original)
                    .take(6)
                    .collect();
                Some(GrammarProblem {
                    range,
                    kind: format!("{:?}", lint.lint_kind),
                    message: lint.message,
                    replacements,
                })
            })
            .collect();
        found.sort_by_key(|problem| (problem.range.start, problem.range.end));
        found
    }
}

/// Made on a worker thread (the first grammar check).
pub fn load() -> Arc<Grammar> {
    Arc::new(Grammar::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harper_finds_ami_pros_kind_of_problem_in_its_sets() {
        let grammar = Grammar::new();
        // PGR1's sentences: "This are a apple." (agreement, a/an).
        let found = grammar.check(
            "This is a apple. He went to the the store.",
            "All Rules",
            Dialect::American,
        );
        assert!(
            found
                .iter()
                .any(|problem| problem.replacements.iter().any(|r| r == "an")),
            "{found:?}"
        );
        assert!(found.iter().all(|problem| problem.range.end <= 42));
        // Every set checks grammar; none checks spelling (Spell Check does).
        for set in SETS {
            assert!(checks(set, LintKind::Agreement), "{set}");
            assert!(!checks(set, LintKind::Spelling), "{set}");
        }
        assert!(checks("All Rules", LintKind::Style));
        assert!(!checks("Business Writing", LintKind::Style));
        assert!(!checks("Casual Writing", LintKind::WordChoice));
    }
}
