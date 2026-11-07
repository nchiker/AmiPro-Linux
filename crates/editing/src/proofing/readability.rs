//! Readability statistics as Ami Pro's grammar checker gives them at the end
//! of a check (dialog 3053; `docs/research/PROOFING_NOTES.md`, "Readability
//! statistics"; VM PGR1): words, sentences, paragraphs, syllables,
//! three-syllable words, averages, passive sentences, Gunning's Fog Index,
//! Flesch-Kincaid, Flesch Reading Ease and its grade level, by Ami Pro's
//! formulas. The counts are the project's own rules: Ami Pro's came from
//! its grammar engine.

/// The figures Ami Pro's Readability Statistics shows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Statistics {
    pub words: usize,
    pub sentences: usize,
    pub paragraphs: usize,
    pub syllables: usize,
    pub three_syllable_words: usize,
    pub passive_sentences: usize,
}

impl Statistics {
    /// Adds a paragraph's figures (an empty paragraph adds nothing).
    pub fn add_paragraph(&mut self, text: &str) {
        let sentences = sentences(text);
        if sentences.is_empty() {
            return;
        }
        self.paragraphs += 1;
        for sentence in &sentences {
            let words = words(sentence);
            if words.is_empty() {
                continue;
            }
            self.sentences += 1;
            if is_passive(&words) {
                self.passive_sentences += 1;
            }
            for (index, word) in words.iter().enumerate() {
                self.words += 1;
                let count = syllables(word);
                self.syllables += count;
                // Ami Pro leaves out proper nouns and words that reach
                // three syllables only through -es or -ed.
                let proper = index > 0 && word.chars().next().is_some_and(char::is_uppercase);
                let inflected = (word.ends_with("es") || word.ends_with("ed"))
                    && syllables(&word[..word.len() - 2]) < 3;
                if count >= 3 && !proper && !inflected {
                    self.three_syllable_words += 1;
                }
            }
        }
    }

    pub fn words_per_sentence(&self) -> f64 {
        ratio(self.words, self.sentences)
    }

    pub fn sentences_per_paragraph(&self) -> f64 {
        ratio(self.sentences, self.paragraphs)
    }

    /// Passive sentences, as a percentage of the sentences.
    pub fn passive_percent(&self) -> f64 {
        100.0 * ratio(self.passive_sentences, self.sentences)
    }

    /// Gunning's Fog Index: 0.4 × (words per sentence + percentage of
    /// three-syllable words).
    pub fn fog_index(&self) -> f64 {
        0.4 * (self.words_per_sentence() + 100.0 * ratio(self.three_syllable_words, self.words))
    }

    /// Flesch-Kincaid grade: 0.39 × words per sentence + 11.8 × syllables
    /// per word − 15.59, at least 0.
    pub fn flesch_kincaid(&self) -> f64 {
        if self.words == 0 {
            return 0.0;
        }
        (0.39 * self.words_per_sentence() + 11.8 * ratio(self.syllables, self.words) - 15.59)
            .max(0.0)
    }

    /// Flesch Reading Ease: 206.835 − 0.846 × syllables per 100 words −
    /// 1.015 × words per sentence, at least 0.
    pub fn reading_ease(&self) -> f64 {
        if self.words == 0 {
            return 0.0;
        }
        (206.835
            - 0.846 * 100.0 * ratio(self.syllables, self.words)
            - 1.015 * self.words_per_sentence())
        .max(0.0)
    }

    /// Flesch Reading Ease Grade Level, Ami Pro's table from the score:
    /// below 30 is 17; 30–50 from 16 to 13, 50–60 from 13 to 10, 60–70 from
    /// 9 to 8, 70–100 from 8 to 5, straight within each band; above 100, 0.
    pub fn reading_ease_grade(&self) -> f64 {
        let score = self.reading_ease();
        let band = |from: f64, to: f64, high: f64, low: f64| {
            high - (high - low) * (score - from) / (to - from)
        };
        match score {
            s if s < 30.0 => 17.0,
            s if s < 50.0 => band(30.0, 50.0, 16.0, 13.0),
            s if s < 60.0 => band(50.0, 60.0, 13.0, 10.0),
            s if s < 70.0 => band(60.0, 70.0, 9.0, 8.0),
            s if s <= 100.0 => band(70.0, 100.0, 8.0, 5.0),
            _ => 0.0,
        }
    }
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 0.0 } else { a as f64 / b as f64 }
}

/// A paragraph's sentences: text up to each `.`, `!`, or `?` followed by a
/// space or the end.
pub fn sentences(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut start = 0;
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    for (index, (at, c)) in chars.iter().enumerate() {
        if matches!(c, '.' | '!' | '?') {
            let next = chars.get(index + 1).map(|(_, c)| *c);
            if next.is_none_or(|c| c.is_whitespace() || matches!(c, '"' | '\u{201D}' | ')')) {
                let end = at + c.len_utf8();
                if !text[start..end].trim().is_empty() {
                    found.push(&text[start..end]);
                }
                start = end;
            }
        }
    }
    if !text[start..].trim().is_empty() {
        found.push(&text[start..]);
    }
    found
}

/// A sentence's words: runs of letters, digits, apostrophes, and hyphens.
pub fn words(sentence: &str) -> Vec<&str> {
    sentence
        .split(|c: char| !(c.is_alphanumeric() || matches!(c, '\'' | '\u{2019}' | '-')))
        .map(|word| word.trim_matches(['\'', '\u{2019}', '-']))
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .collect()
}

/// A word's syllables by English spelling rules: vowel groups (y a vowel
/// after a consonant), a silent final e (not -le), at least one; a
/// hyphenated word's parts each at least one.
pub fn syllables(word: &str) -> usize {
    if word.contains('-') {
        return word
            .split('-')
            .filter(|part| !part.is_empty())
            .map(syllables)
            .sum::<usize>()
            .max(1);
    }
    let lower: Vec<char> = word
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic())
        .collect();
    if lower.is_empty() {
        return 1;
    }
    let vowel = |index: usize| {
        let c = lower[index];
        matches!(
            c,
            'a' | 'e' | 'i' | 'o' | 'u' | 'é' | 'à' | 'è' | 'ü' | 'ö' | 'ä'
        ) || (c == 'y' && index > 0 && !matches!(lower[index - 1], 'a' | 'e' | 'i' | 'o' | 'u'))
    };
    let mut count = 0;
    let mut previous = false;
    for index in 0..lower.len() {
        let is = vowel(index);
        if is && !previous {
            count += 1;
        }
        previous = is;
    }
    let n = lower.len();
    // A silent final e: "line", "receive", not "the", "able".
    if n > 2
        && lower[n - 1] == 'e'
        && !vowel(n - 2)
        && count > 1
        && !(lower[n - 2] == 'l' && !vowel(n - 3))
    {
        count -= 1;
    }
    // -ed after a consonant other than t or d is no syllable: "walked".
    if n > 3
        && lower[n - 2] == 'e'
        && lower[n - 1] == 'd'
        && !matches!(lower[n - 3], 't' | 'd')
        && count > 1
        && !vowel(n - 3)
    {
        count -= 1;
    }
    count.max(1)
}

/// The forms of "to be" a passive takes.
const BE: [&str; 8] = ["is", "are", "was", "were", "be", "been", "being", "am"];

/// Irregular past participles common in passives.
const PARTICIPLES: [&str; 24] = [
    "done", "made", "given", "taken", "seen", "known", "shown", "written", "built", "found",
    "held", "kept", "left", "paid", "put", "sent", "set", "sold", "told", "thought", "brought",
    "chosen", "born", "led",
];

/// Whether a sentence reads as passive: a form of "to be" followed,
/// perhaps after an adverb, by a past participle.
pub fn is_passive(words: &[&str]) -> bool {
    words.windows(3).any(|window| {
        let be = BE.contains(&window[0].to_lowercase().as_str());
        let participle = |word: &str| {
            let word = word.to_lowercase();
            (word.len() > 3 && word.ends_with("ed")) || PARTICIPLES.contains(&word.as_str())
        };
        be && (participle(window[1]) || (window[1].ends_with("ly") && participle(window[2])))
    }) || words.len() == 2
        && BE.contains(&words[0].to_lowercase().as_str())
        && words[1].to_lowercase().ends_with("ed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ami_pros_formulas_reproduce_pgr1() {
        // VM PGR1: 20 words, 3 sentences, 2 paragraphs, 25 syllables, no
        // three-syllable words: Fog 2.7, Flesch-Kincaid 1.8, Reading Ease
        // 94.3, its grade 5.6.
        let figures = Statistics {
            words: 20,
            sentences: 3,
            paragraphs: 2,
            syllables: 25,
            three_syllable_words: 0,
            passive_sentences: 0,
        };
        assert_eq!(format!("{:.1}", figures.words_per_sentence()), "6.7");
        assert_eq!(format!("{:.1}", figures.sentences_per_paragraph()), "1.5");
        assert_eq!(format!("{:.1}", figures.fog_index()), "2.7");
        assert_eq!(format!("{:.1}", figures.flesch_kincaid()), "1.8");
        assert_eq!(format!("{:.1}", figures.reading_ease()), "94.3");
        assert_eq!(format!("{:.1}", figures.reading_ease_grade()), "5.6");
    }

    #[test]
    fn counts_come_from_the_text() {
        let mut figures = Statistics::default();
        figures.add_paragraph("The cat sat on the mat. We receive letters today.");
        figures.add_paragraph("");
        figures.add_paragraph("It was decided by the committee that the plan was approved.");
        assert_eq!(
            (
                figures.words,
                figures.sentences,
                figures.paragraphs,
                figures.passive_sentences
            ),
            (21, 3, 2, 1)
        );
        assert_eq!(syllables("the"), 1);
        assert_eq!(syllables("receive"), 2);
        assert_eq!(syllables("today"), 2);
        assert_eq!(syllables("committee"), 3);
        assert_eq!(syllables("table"), 2);
        assert_eq!(syllables("walked"), 1);
        assert_eq!(syllables("decided"), 3);
        assert_eq!(syllables("well-known"), 2);
        // "committee" has three syllables; "decided" reaches three only
        // through -ed.
        assert_eq!(figures.three_syllable_words, 1);
    }
}
