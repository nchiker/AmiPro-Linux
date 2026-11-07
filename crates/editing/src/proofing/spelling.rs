//! Spell Check by Ami Pro's rules (`docs/research/PROOFING_NOTES.md`,
//! "Spelling"; VM PSP1–PSP4B): the words a paragraph holds, the options,
//! the walk through the text streams, repeated words, Skip All, and Replace
//! All (decision SP4). The words themselves are checked by a [`Speller`]:
//! the app's Hunspell dictionaries, or a list in tests.

use std::collections::BTreeSet;
use std::ops::Range;

use amipro_document::Document;

use crate::find::{self, FindOptions};
use crate::{Command, TextPosition};

/// What checks a word and suggests others: a dictionary with the user's
/// words.
pub trait Speller {
    /// Whether `word` is spelled correctly.
    fn check(&self, word: &str) -> bool;
    /// Alternatives for `word`, best first.
    fn suggest(&self, word: &str) -> Vec<String>;
}

/// Ami Pro's Spell Check options (`SpellOpts`; dialogs 1099 and 1030),
/// with its defaults (VM PSP1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SpellOptions {
    /// Check from beginning of document (off).
    pub from_beginning: bool,
    /// Include other text streams (on).
    pub other_streams: bool,
    /// Check for repeated words (on).
    pub repeated_words: bool,
    /// Check words with numbers (on).
    pub words_with_numbers: bool,
    /// Check words with initial caps (on).
    pub initial_caps: bool,
    /// Include user dictionary alternatives (off).
    pub user_alternatives: bool,
}

impl Default for SpellOptions {
    fn default() -> Self {
        Self {
            from_beginning: false,
            other_streams: true,
            repeated_words: true,
            words_with_numbers: true,
            initial_caps: true,
            user_alternatives: false,
        }
    }
}

impl SpellOptions {
    /// Ami Pro's `SpellOpts` number (`1328:0032`): 1 from beginning, 2 other
    /// streams off, 4 repeated words off, 8 numbers off, 16 initial caps off,
    /// 32 user dictionary alternatives on.
    pub fn bits(&self) -> u32 {
        u32::from(self.from_beginning)
            | u32::from(!self.other_streams) << 1
            | u32::from(!self.repeated_words) << 2
            | u32::from(!self.words_with_numbers) << 3
            | u32::from(!self.initial_caps) << 4
            | u32::from(self.user_alternatives) << 5
    }

    pub fn from_bits(bits: u32) -> Self {
        Self {
            from_beginning: bits & 1 != 0,
            other_streams: bits & 2 == 0,
            repeated_words: bits & 4 == 0,
            words_with_numbers: bits & 8 == 0,
            initial_caps: bits & 16 == 0,
            user_alternatives: bits & 32 != 0,
        }
    }
}

/// What is wrong with a word.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProblemKind {
    /// Not in the dictionary.
    Misspelled,
    /// The same word as the one before it ("the the"). Its range starts at
    /// the space before it, so an empty replacement deletes both (VM PSP1).
    Repeated,
}

/// A word in a paragraph: its bytes and the word as checked.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Word {
    pub range: Range<usize>,
    pub text: String,
}

/// A problem in a paragraph's text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParagraphProblem {
    pub range: Range<usize>,
    /// The word as it stands in the text.
    pub word: String,
    pub kind: ProblemKind,
}

/// A problem found by the walk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Problem {
    pub start: TextPosition,
    pub end: TextPosition,
    pub word: String,
    pub kind: ProblemKind,
}

/// Words the user chose to skip for the session (Skip All, Ignore All),
/// exactly as written (Ami Pro's list compares the word as typed).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Ignored {
    words: BTreeSet<String>,
}

impl Ignored {
    pub fn add(&mut self, word: &str) {
        self.words.insert(word.to_string());
    }

    pub fn contains(&self, word: &str) -> bool {
        self.words.contains(word)
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

/// Whether `c` ends a token: white space, an object, a line break, and the
/// no-break space (Ami Pro's scanner, `1320:0541`).
fn separates(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{FFFC}' | '\u{2028}' | '\u{00A0}')
}

/// Characters a word keeps inside it: apostrophes and hyphens join.
fn joins(c: char) -> bool {
    matches!(
        c,
        '\'' | '\u{2019}' | '-' | '\u{2010}' | '\u{2011}' | '\u{00AD}' | '.'
    )
}

/// The words of a paragraph's text. A token is a run of characters between
/// spaces, objects, and line breaks; its word is the token without the
/// punctuation around it (Ami Pro's engine trims it the same way). Web and
/// e-mail addresses are no words (decision SK2), nor tokens without a
/// letter.
pub fn words(text: &str) -> Vec<Word> {
    let mut found = Vec::new();
    let mut start = None;
    let push = |token: Range<usize>, found: &mut Vec<Word>| {
        let slice = &text[token.clone()];
        let lead = slice
            .char_indices()
            .find(|(_, c)| c.is_alphanumeric())
            .map(|(at, _)| at);
        let Some(lead) = lead else {
            return;
        };
        let trail = slice
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_alphanumeric())
            .map(|(at, c)| at + c.len_utf8())
            .unwrap_or(slice.len());
        let word = &slice[lead..trail];
        if !word.chars().any(char::is_alphabetic)
            || slice.contains("://")
            || slice.starts_with("www.")
            || (word.contains('@') && word.contains('.'))
        {
            return;
        }
        // A word with other punctuation inside (a slash, a bracket) is two
        // words: only apostrophes, hyphens, and full stops join.
        let base = token.start + lead;
        let mut part_start = None;
        for (at, c) in word
            .char_indices()
            .chain(std::iter::once((word.len(), ' ')))
        {
            let inside = c.is_alphanumeric() || (joins(c) && part_start.is_some());
            match (inside, part_start) {
                (true, None) => part_start = Some(at),
                (false, Some(from)) => {
                    let part = word[from..at].trim_end_matches(joins);
                    if part.chars().any(char::is_alphabetic) {
                        found.push(Word {
                            range: base + from..base + from + part.len(),
                            text: part.replace('\u{00AD}', ""),
                        });
                    }
                    part_start = None;
                }
                _ => {}
            }
        }
    };
    for (at, c) in text.char_indices() {
        if separates(c) {
            if let Some(from) = start.take() {
                push(from..at, &mut found);
            }
        } else if start.is_none() {
            start = Some(at);
        }
    }
    if let Some(from) = start {
        push(from..text.len(), &mut found);
    }
    found
}

/// Whether the options leave `word` unchecked: one-letter words (Ami Pro
/// never sends them, `1320:00c2`), words with digits when Check words with
/// numbers is off, and capitalized words when Check words with initial caps
/// is off (`1320:0eac`, `0ed6`).
fn unchecked(word: &str, options: &SpellOptions) -> bool {
    let mut letters = word.chars().filter(|c| c.is_alphanumeric());
    let Some(first) = letters.next() else {
        return true;
    };
    if word.chars().count() < 2 {
        return true;
    }
    if !options.words_with_numbers && word.chars().any(|c| c.is_ascii_digit()) {
        return true;
    }
    if !options.initial_caps && first.is_uppercase() && letters.all(|c| !c.is_uppercase()) {
        return true;
    }
    false
}

/// Whether `speller` knows `word`: as written, or each part of a hyphenated
/// word (Ami Pro looks the whole up; Hunspell dictionaries hold compounds
/// in parts), or with its first letter small at a sentence's start.
pub fn spelled(speller: &dyn Speller, word: &str) -> bool {
    if speller.check(word) {
        return true;
    }
    let trimmed = word.trim_end_matches('.');
    if trimmed != word && !trimmed.is_empty() && speller.check(trimmed) {
        return true;
    }
    if word.contains('-') {
        return word
            .split('-')
            .filter(|part| !part.is_empty())
            .all(|part| part.chars().count() < 2 || speller.check(part));
    }
    false
}

/// The problems in one paragraph's text: unknown words and repeated words,
/// by `options`, without the words in `ignored`.
pub fn paragraph_problems(
    text: &str,
    speller: &dyn Speller,
    options: &SpellOptions,
    ignored: &Ignored,
) -> Vec<ParagraphProblem> {
    let words = words(text);
    let mut problems = Vec::new();
    for (index, word) in words.iter().enumerate() {
        if options.repeated_words
            && index > 0
            && let Some(previous) = words.get(index - 1)
            && previous.text.to_lowercase() == word.text.to_lowercase()
            && text[previous.range.end..word.range.start]
                .chars()
                .all(|c| c == ' ' || c == '\t' || c == '\u{00A0}')
            && !ignored.contains(&word.text)
        {
            problems.push(ParagraphProblem {
                range: previous.range.end..word.range.end,
                word: word.text.clone(),
                kind: ProblemKind::Repeated,
            });
            continue;
        }
        if unchecked(&word.text, options) || ignored.contains(&word.text) {
            continue;
        }
        if !spelled(speller, &word.text) {
            problems.push(ParagraphProblem {
                range: word.range.clone(),
                word: word.text.clone(),
                kind: ProblemKind::Misspelled,
            });
        }
    }
    problems
}

/// The streams a check goes through, by Ami Pro's rules: from the caret's
/// stream (or the first, with Check from beginning) to the last, never
/// wrapping; only the caret's without Include other text streams.
pub fn check_streams(
    document: &Document,
    from: amipro_document::FlowId,
    options: &SpellOptions,
) -> Vec<amipro_document::FlowId> {
    find::search_streams(
        document,
        from,
        &FindOptions {
            from_beginning: options.from_beginning,
            other_streams: options.other_streams,
            ..FindOptions::default()
        },
    )
}

/// The next problem at or after `from` (a position in the first of
/// `streams`), up to `until` in its flow when the check is of a selection.
/// A check from the caret starts at the beginning of the caret's word (Ami
/// Pro, `1328:01e2`).
pub fn next_problem(
    document: &Document,
    streams: &[amipro_document::FlowId],
    from: TextPosition,
    until: Option<TextPosition>,
    speller: &dyn Speller,
    options: &SpellOptions,
    ignored: &Ignored,
) -> Option<Problem> {
    // From the stream the check is in: the ones before it are done.
    let at = streams
        .iter()
        .position(|flow| *flow == from.flow)
        .unwrap_or(0);
    for (index, flow) in streams.iter().enumerate().skip(at) {
        let index = index - at;
        let Some(paragraphs) = document.flow(*flow).map(|flow| &flow.paragraphs) else {
            continue;
        };
        let first = if index == 0 && *flow == from.flow {
            from.paragraph
        } else {
            0
        };
        for (number, paragraph) in paragraphs.iter().enumerate().skip(first) {
            if let Some(until) = until
                && until.flow == *flow
                && number > until.paragraph
            {
                return None;
            }
            for problem in paragraph_problems(&paragraph.text, speller, options, ignored) {
                if index == 0
                    && *flow == from.flow
                    && number == from.paragraph
                    && problem.range.end <= from.byte
                {
                    continue;
                }
                if let Some(until) = until
                    && until.flow == *flow
                    && number == until.paragraph
                    && problem.range.end > until.byte
                {
                    return None;
                }
                return Some(Problem {
                    start: TextPosition {
                        flow: *flow,
                        paragraph: number,
                        byte: problem.range.start,
                    },
                    end: TextPosition {
                        flow: *flow,
                        paragraph: number,
                        byte: problem.range.end,
                    },
                    word: problem.word,
                    kind: problem.kind,
                });
            }
        }
        if until.is_some() {
            return None;
        }
    }
    None
}

/// Every problem in `streams` (for counting and as you type).
pub fn all_problems(
    document: &Document,
    streams: &[amipro_document::FlowId],
    speller: &dyn Speller,
    options: &SpellOptions,
    ignored: &Ignored,
) -> usize {
    streams
        .iter()
        .filter_map(|flow| document.flow(*flow))
        .flat_map(|flow| flow.paragraphs.iter())
        .map(|paragraph| paragraph_problems(&paragraph.text, speller, options, ignored).len())
        .sum()
}

/// Suggestions in the flagged word's capitals (VM PSP1: "Teh" offers The,
/// "NASAA" offers NASA): a word in capitals gives them in capitals, a
/// capitalized word capitalizes their first letter; otherwise they are as
/// the dictionary gives them ("teh" offers Ted).
pub fn in_capitals_of(suggestion: &str, word: &str) -> String {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase()) {
        return suggestion.to_uppercase();
    }
    if letters.first().is_some_and(|c| c.is_uppercase()) {
        let mut chars = suggestion.chars();
        return match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        };
    }
    suggestion.to_string()
}

/// Alternatives for `word`: the speller's, in its capitals, without
/// duplicates, at most ten (Ami Pro's list, `1320:09a5`).
pub fn alternatives(speller: &dyn Speller, word: &str) -> Vec<String> {
    let mut seen = BTreeSet::new();
    speller
        .suggest(word)
        .into_iter()
        .map(|suggestion| in_capitals_of(&suggestion, word))
        .filter(|suggestion| suggestion != word && seen.insert(suggestion.clone()))
        .take(10)
        .collect()
}

/// Replace All (decision SP4): every occurrence of `word` in `streams`
/// (within a selection's range when given) as one step, each keeping its
/// capitals (Teh → The, teh → the); the occurrence written exactly as the
/// flagged word takes the replacement as typed. The count is the number of
/// occurrences; `None` when there are none.
pub fn replace_all(
    document: &Document,
    streams: &[amipro_document::FlowId],
    within: Option<(TextPosition, TextPosition)>,
    word: &str,
    replacement: &str,
) -> Option<(Command, usize)> {
    let options = FindOptions {
        whole_word: true,
        match_case: false,
        keep_capitals: true,
        ..FindOptions::default()
    };
    let escaped = escape_pattern(word);
    let found: Vec<find::Match> = find::find_all(document, &escaped, streams, &options, within)
        .into_iter()
        .filter(|found| found.start.paragraph == found.end.paragraph)
        .collect();
    let commands: Vec<Command> = found
        .iter()
        .rev()
        .map(|found| Command::ReplaceText {
            start: found.start,
            end: found.end,
            text: if found.text == word {
                replacement.to_string()
            } else {
                find::set_in_capitals_of(replacement, &found.text)
            },
        })
        .collect();
    let count = found.len();
    (!commands.is_empty()).then_some((Command::Several(commands), count))
}

/// `text` as a Find pattern matching itself: Ami Pro's wildcards and codes
/// written as their literals.
fn escape_pattern(text: &str) -> String {
    let mut pattern = String::new();
    for c in text.chars() {
        match c {
            '?' | '*' | find::TAB_MARK | find::RETURN_MARK => {
                pattern.push('<');
                pattern.push(c);
                pattern.push('>');
            }
            _ => pattern.push(c),
        }
    }
    pattern
}

/// Reads a user dictionary in Ami Pro's format (`LTSUSER1.DIC`; VM PSP1):
/// one word per line, its first token, lines starting with Ctrl+Z ignored;
/// UTF-8, or Windows-1252 as Ami Pro wrote it.
pub fn read_word_list(bytes: &[u8]) -> Vec<String> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => amipro_document::decode_windows_1252(bytes),
    };
    let mut words = Vec::new();
    for line in text.lines() {
        if line.starts_with('\u{1A}') {
            continue;
        }
        if let Some(word) = line.split([' ', '\t', '\r']).find(|word| !word.is_empty())
            && !words.iter().any(|known| known == word)
        {
            words.push(word.to_string());
        }
    }
    words
}

/// Writes a user dictionary as Ami Pro writes one: each word and a CR LF.
pub fn write_word_list(words: &[String]) -> String {
    words.iter().map(|word| format!("{word}\r\n")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A speller knowing a fixed list.
    pub(crate) struct List(pub Vec<&'static str>);

    impl Speller for List {
        fn check(&self, word: &str) -> bool {
            self.0.iter().any(|known| {
                *known == word
                    || (known.chars().next().is_some_and(char::is_lowercase)
                        && known.eq_ignore_ascii_case(word))
            })
        }

        fn suggest(&self, word: &str) -> Vec<String> {
            let lower = word.to_lowercase();
            self.0
                .iter()
                .filter(|known| {
                    known.len().abs_diff(lower.len()) <= 1
                        && known.chars().filter(|c| lower.contains(*c)).count() + 1 >= lower.len()
                })
                .map(|known| known.to_string())
                .collect()
        }
    }

    fn texts(text: &str) -> Vec<&str> {
        words(text)
            .into_iter()
            .map(|word| &text[word.range])
            .collect()
    }

    #[test]
    fn words_are_tokens_without_the_punctuation_around_them() {
        assert_eq!(
            texts("\"Teh\" cat's well-known (mat), e.g. abc123 and/or 1993."),
            [
                "Teh",
                "cat's",
                "well-known",
                "mat",
                "e.g",
                "abc123",
                "and",
                "or"
            ]
        );
        assert_eq!(
            texts("see https://example.com or me@example.com"),
            ["see", "or"]
        );
        assert_eq!(
            texts("one\u{FFFC}two\u{00A0}three"),
            ["one", "two", "three"]
        );
    }

    #[test]
    fn the_options_choose_which_words_are_checked() {
        let speller = List(vec!["the", "cat", "sat", "on", "mat", "we"]);
        let text = "Teh cat sat on the the mat. We abc123 Xyzzy NASAA a";
        let found = |options: SpellOptions| -> Vec<(String, ProblemKind)> {
            paragraph_problems(text, &speller, &options, &Ignored::default())
                .into_iter()
                .map(|problem| (problem.word, problem.kind))
                .collect()
        };
        let all = found(SpellOptions::default());
        assert_eq!(
            all,
            [
                ("Teh".to_string(), ProblemKind::Misspelled),
                ("the".to_string(), ProblemKind::Repeated),
                ("abc123".to_string(), ProblemKind::Misspelled),
                ("Xyzzy".to_string(), ProblemKind::Misspelled),
                ("NASAA".to_string(), ProblemKind::Misspelled),
            ]
        );
        // Without numbers, initial caps, and repeats (NASAA is all
        // capitals: still checked).
        let fewer = found(SpellOptions {
            words_with_numbers: false,
            initial_caps: false,
            repeated_words: false,
            ..SpellOptions::default()
        });
        assert_eq!(fewer, [("NASAA".to_string(), ProblemKind::Misspelled)]);
        // A repeat's range starts at the space before it.
        let repeat = paragraph_problems(
            text,
            &speller,
            &SpellOptions::default(),
            &Ignored::default(),
        )
        .into_iter()
        .find(|problem| problem.kind == ProblemKind::Repeated)
        .unwrap();
        assert_eq!(&text[repeat.range], " the");
    }

    #[test]
    fn options_round_trip_ami_pros_bits() {
        assert_eq!(SpellOptions::default().bits(), 0);
        for bits in 0..64 {
            assert_eq!(SpellOptions::from_bits(bits).bits(), bits);
        }
    }

    #[test]
    fn suggestions_take_the_words_capitals() {
        assert_eq!(in_capitals_of("the", "Teh"), "The");
        assert_eq!(in_capitals_of("nasa", "NASAA"), "NASA");
        assert_eq!(in_capitals_of("Ted", "teh"), "Ted");
        assert_eq!(in_capitals_of("the", "teh"), "the");
    }

    #[test]
    fn user_dictionaries_are_ami_pros_plain_text() {
        let words =
            read_word_list(b"abc123\r\nMulti-colonnage\r\n\x1aEOF\r\nabc123\r\nxyz extra\r\n");
        assert_eq!(words, ["abc123", "Multi-colonnage", "xyz"]);
        assert_eq!(
            write_word_list(&words),
            "abc123\r\nMulti-colonnage\r\nxyz\r\n"
        );
        assert_eq!(read_word_list(b"caf\xe9\r\n"), ["café"]);
    }
}
