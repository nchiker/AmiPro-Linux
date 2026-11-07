//! The thesaurus (`UI_SPEC.md` §35, SP7): MyThes thesauri (LibreOffice's
//! format, read here without dependencies) and Ami Pro's way with a word
//! looked up (`docs/research/PROOFING_NOTES.md`, "Thesaurus"; VM PSP1): its
//! base form found when the word is inflected ("sat" is "sit"), the
//! synonyms inflected as the word was ("posed"), and set in its capitals.

use std::collections::BTreeSet;
use std::io::{BufRead, Read, Seek, SeekFrom};

/// A thesaurus's index: each entry's word (in small letters) and where its
/// meanings are in the data file.
#[derive(Clone, Debug, Default)]
pub struct Index {
    entries: Vec<(String, u64)>,
    /// The data file's encoding: UTF-8, else ISO 8859-1.
    latin1: bool,
}

/// One meaning: its part of speech ("noun", "verb", "adj", "adv") and its
/// synonyms, each without MyThes's "(generic term)" notes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Meaning {
    pub part_of_speech: String,
    pub synonyms: Vec<String>,
}

/// What a lookup found: the meanings, with the word they belong to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    /// The form found in the thesaurus (the base of an inflected word).
    pub base: String,
    pub meanings: Vec<Meaning>,
}

fn decode(bytes: &[u8], latin1: bool) -> String {
    if latin1 {
        bytes.iter().map(|&byte| byte as char).collect()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

impl Index {
    /// Reads a MyThes `.idx` file: the encoding, the count, then `word|offset`
    /// lines.
    pub fn read(bytes: &[u8]) -> Self {
        let mut lines = bytes.split(|&byte| byte == b'\n');
        let encoding = lines
            .next()
            .map(|line| String::from_utf8_lossy(line).trim().to_ascii_uppercase())
            .unwrap_or_default();
        let latin1 = encoding != "UTF-8";
        let mut entries: Vec<(String, u64)> = lines
            .skip(1)
            .filter_map(|line| {
                let line = decode(line, latin1);
                let (word, offset) = line.trim_end_matches('\r').rsplit_once('|')?;
                Some((word.to_lowercase(), offset.trim().parse().ok()?))
            })
            .collect();
        entries.sort();
        entries.dedup_by(|a, b| a.0 == b.0);
        Self { entries, latin1 }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn offset(&self, word: &str) -> Option<u64> {
        let word = word.to_lowercase();
        self.entries
            .binary_search_by(|(own, _)| own.as_str().cmp(&word))
            .ok()
            .map(|index| self.entries[index].1)
    }

    /// The meanings of `word` exactly, read from the data file.
    pub fn meanings<R: Read + Seek>(&self, data: &mut R, word: &str) -> Option<Vec<Meaning>> {
        let offset = self.offset(word)?;
        data.seek(SeekFrom::Start(offset)).ok()?;
        let mut reader = std::io::BufReader::new(data);
        let mut line = Vec::new();
        reader.read_until(b'\n', &mut line).ok()?;
        let head = decode(&line, self.latin1);
        let count: usize = head.trim_end().rsplit_once('|')?.1.trim().parse().ok()?;
        let mut meanings = Vec::new();
        for _ in 0..count.min(64) {
            line.clear();
            if reader.read_until(b'\n', &mut line).ok()? == 0 {
                break;
            }
            let text = decode(&line, self.latin1);
            let mut fields = text.trim_end_matches(['\r', '\n']).split('|');
            let part = fields
                .next()
                .unwrap_or_default()
                .trim_matches(['(', ')', ' '])
                .to_string();
            let mut seen = BTreeSet::new();
            let synonyms: Vec<String> = fields
                .map(|synonym| match synonym.find(" (") {
                    Some(at) if synonym.ends_with(')') => synonym[..at].trim().to_string(),
                    _ => synonym.trim().to_string(),
                })
                .filter(|synonym| !synonym.is_empty() && !synonym.eq_ignore_ascii_case(word))
                .filter(|synonym| seen.insert(synonym.to_lowercase()))
                .collect();
            if !synonyms.is_empty() {
                meanings.push(Meaning {
                    part_of_speech: part,
                    synonyms,
                });
            }
        }
        Some(meanings)
    }

    /// Ami Pro's lookup: the word's own meanings, or its base form's with
    /// the synonyms inflected as the word is ("sat": sit's verbs, past
    /// tense), every synonym in the word's capitals. `None` when neither is
    /// in the thesaurus ("%s is not in the thesaurus.").
    pub fn look_up<R: Read + Seek>(&self, data: &mut R, word: &str) -> Option<Entry> {
        let word = word.trim();
        if word.is_empty() {
            return None;
        }
        let lower = word.to_lowercase();
        let mut found: Option<Entry> = None;
        for (base, inflection) in bases(&lower) {
            let Some(meanings) = self.meanings(data, &base) else {
                continue;
            };
            let meanings: Vec<Meaning> = meanings
                .into_iter()
                .filter(|meaning| inflection.fits(&meaning.part_of_speech))
                .map(|meaning| Meaning {
                    synonyms: meaning
                        .synonyms
                        .iter()
                        .map(|synonym| in_capitals_of(&inflection.apply(synonym), word))
                        .collect(),
                    part_of_speech: meaning.part_of_speech,
                })
                .collect();
            if meanings.is_empty() {
                continue;
            }
            match found.as_mut() {
                // The word's own meanings first, then its base's (a noun
                // "sat" is Saturday; the verb is sit's).
                Some(entry) => entry.meanings.extend(meanings),
                None => {
                    found = Some(Entry {
                        base: base.clone(),
                        meanings,
                    })
                }
            }
        }
        found
    }
}

/// How a word is inflected from its base.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Inflection {
    None,
    /// -s: a plural or a verb's third person.
    S,
    /// -ed: the past tense or participle (verbs).
    Past,
    /// -ing (verbs).
    Ing,
    /// -er (adjectives).
    Er,
    /// -est (adjectives).
    Est,
    /// An irregular verb's past form ("went" of "go"): synonyms take their
    /// own past.
    IrregularPast,
}

impl Inflection {
    /// Whether a meaning with this part of speech can be inflected so.
    fn fits(self, part: &str) -> bool {
        match self {
            Inflection::None => true,
            Inflection::S => part.starts_with("noun") || part.starts_with("verb"),
            Inflection::Past | Inflection::Ing | Inflection::IrregularPast => {
                part.starts_with("verb")
            }
            Inflection::Er | Inflection::Est => part.starts_with("adj"),
        }
    }

    /// `synonym` inflected so: its first word (a phrasal verb's verb).
    pub fn apply(self, synonym: &str) -> String {
        let (head, rest) = match synonym.split_once(' ') {
            Some((head, rest)) => (head, Some(rest)),
            None => (synonym, None),
        };
        let inflected = match self {
            Inflection::None => head.to_string(),
            Inflection::S => plural(head),
            Inflection::Past | Inflection::IrregularPast => past(head),
            Inflection::Ing => ing(head),
            Inflection::Er => comparative(head, "er"),
            Inflection::Est => comparative(head, "est"),
        };
        match rest {
            Some(rest) => format!("{inflected} {rest}"),
            None => inflected,
        }
    }
}

/// Irregular verbs: base, past, past participle (common English verbs).
const IRREGULAR: &[(&str, &str, &str)] = &[
    ("arise", "arose", "arisen"),
    ("awake", "awoke", "awoken"),
    ("be", "was", "been"),
    ("bear", "bore", "borne"),
    ("beat", "beat", "beaten"),
    ("become", "became", "become"),
    ("begin", "began", "begun"),
    ("bend", "bent", "bent"),
    ("bet", "bet", "bet"),
    ("bind", "bound", "bound"),
    ("bite", "bit", "bitten"),
    ("bleed", "bled", "bled"),
    ("blow", "blew", "blown"),
    ("break", "broke", "broken"),
    ("breed", "bred", "bred"),
    ("bring", "brought", "brought"),
    ("build", "built", "built"),
    ("burn", "burnt", "burnt"),
    ("burst", "burst", "burst"),
    ("buy", "bought", "bought"),
    ("cast", "cast", "cast"),
    ("catch", "caught", "caught"),
    ("choose", "chose", "chosen"),
    ("cling", "clung", "clung"),
    ("come", "came", "come"),
    ("cost", "cost", "cost"),
    ("creep", "crept", "crept"),
    ("cut", "cut", "cut"),
    ("deal", "dealt", "dealt"),
    ("dig", "dug", "dug"),
    ("do", "did", "done"),
    ("draw", "drew", "drawn"),
    ("dream", "dreamt", "dreamt"),
    ("drink", "drank", "drunk"),
    ("drive", "drove", "driven"),
    ("eat", "ate", "eaten"),
    ("fall", "fell", "fallen"),
    ("feed", "fed", "fed"),
    ("feel", "felt", "felt"),
    ("fight", "fought", "fought"),
    ("find", "found", "found"),
    ("flee", "fled", "fled"),
    ("fling", "flung", "flung"),
    ("fly", "flew", "flown"),
    ("forbid", "forbade", "forbidden"),
    ("forget", "forgot", "forgotten"),
    ("forgive", "forgave", "forgiven"),
    ("freeze", "froze", "frozen"),
    ("get", "got", "gotten"),
    ("give", "gave", "given"),
    ("go", "went", "gone"),
    ("grind", "ground", "ground"),
    ("grow", "grew", "grown"),
    ("hang", "hung", "hung"),
    ("have", "had", "had"),
    ("hear", "heard", "heard"),
    ("hide", "hid", "hidden"),
    ("hit", "hit", "hit"),
    ("hold", "held", "held"),
    ("hurt", "hurt", "hurt"),
    ("keep", "kept", "kept"),
    ("kneel", "knelt", "knelt"),
    ("know", "knew", "known"),
    ("lay", "laid", "laid"),
    ("lead", "led", "led"),
    ("leap", "leapt", "leapt"),
    ("leave", "left", "left"),
    ("lend", "lent", "lent"),
    ("let", "let", "let"),
    ("lie", "lay", "lain"),
    ("light", "lit", "lit"),
    ("lose", "lost", "lost"),
    ("make", "made", "made"),
    ("mean", "meant", "meant"),
    ("meet", "met", "met"),
    ("pay", "paid", "paid"),
    ("put", "put", "put"),
    ("quit", "quit", "quit"),
    ("read", "read", "read"),
    ("ride", "rode", "ridden"),
    ("ring", "rang", "rung"),
    ("rise", "rose", "risen"),
    ("run", "ran", "run"),
    ("say", "said", "said"),
    ("see", "saw", "seen"),
    ("seek", "sought", "sought"),
    ("sell", "sold", "sold"),
    ("send", "sent", "sent"),
    ("set", "set", "set"),
    ("shake", "shook", "shaken"),
    ("shine", "shone", "shone"),
    ("shoot", "shot", "shot"),
    ("show", "showed", "shown"),
    ("shrink", "shrank", "shrunk"),
    ("shut", "shut", "shut"),
    ("sing", "sang", "sung"),
    ("sink", "sank", "sunk"),
    ("sit", "sat", "sat"),
    ("sleep", "slept", "slept"),
    ("slide", "slid", "slid"),
    ("speak", "spoke", "spoken"),
    ("speed", "sped", "sped"),
    ("spend", "spent", "spent"),
    ("spin", "spun", "spun"),
    ("split", "split", "split"),
    ("spread", "spread", "spread"),
    ("spring", "sprang", "sprung"),
    ("stand", "stood", "stood"),
    ("steal", "stole", "stolen"),
    ("stick", "stuck", "stuck"),
    ("sting", "stung", "stung"),
    ("strike", "struck", "struck"),
    ("swear", "swore", "sworn"),
    ("sweep", "swept", "swept"),
    ("swim", "swam", "swum"),
    ("swing", "swung", "swung"),
    ("take", "took", "taken"),
    ("teach", "taught", "taught"),
    ("tear", "tore", "torn"),
    ("tell", "told", "told"),
    ("think", "thought", "thought"),
    ("throw", "threw", "thrown"),
    ("understand", "understood", "understood"),
    ("wake", "woke", "woken"),
    ("wear", "wore", "worn"),
    ("weep", "wept", "wept"),
    ("win", "won", "won"),
    ("wind", "wound", "wound"),
    ("write", "wrote", "written"),
];

/// Irregular comparatives: base, -er, -est.
const IRREGULAR_COMPARISON: &[(&str, &str, &str)] = &[
    ("good", "better", "best"),
    ("bad", "worse", "worst"),
    ("far", "farther", "farthest"),
    ("little", "less", "least"),
    ("many", "more", "most"),
];

fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'e' | 'i' | 'o' | 'u')
}

/// A short word ending consonant–vowel–consonant doubles its last letter
/// before a suffix ("stop", "stopped"; not "visit", "fix", "show").
fn doubles(word: &str) -> bool {
    let letters: Vec<char> = word.chars().collect();
    let n = letters.len();
    if !(3..=4).contains(&n) {
        return false;
    }
    let (a, b, c) = (letters[n - 3], letters[n - 2], letters[n - 1]);
    !is_vowel(a)
        && is_vowel(b)
        && !is_vowel(c)
        && !matches!(c, 'w' | 'x' | 'y')
        && letters[..n - 2].iter().filter(|c| is_vowel(**c)).count() == 0
}

/// The past tense of `verb`.
pub fn past(verb: &str) -> String {
    if let Some((_, past, _)) = IRREGULAR.iter().find(|(base, _, _)| *base == verb) {
        return past.to_string();
    }
    if verb.ends_with('e') {
        format!("{verb}d")
    } else if let Some(stem) = verb.strip_suffix('y')
        && stem.chars().last().is_some_and(|c| !is_vowel(c))
    {
        format!("{stem}ied")
    } else if doubles(verb) {
        format!("{verb}{}ed", verb.chars().last().unwrap_or_default())
    } else {
        format!("{verb}ed")
    }
}

/// The -ing form of `verb`.
pub fn ing(verb: &str) -> String {
    if let Some(stem) = verb.strip_suffix("ie") {
        format!("{stem}ying")
    } else if verb.ends_with('e') && !verb.ends_with("ee") && verb.len() > 2 {
        format!("{}ing", &verb[..verb.len() - 1])
    } else if doubles(verb) {
        format!("{verb}{}ing", verb.chars().last().unwrap_or_default())
    } else {
        format!("{verb}ing")
    }
}

/// The -s form of a noun or verb.
pub fn plural(word: &str) -> String {
    if ["s", "x", "z", "ch", "sh"]
        .iter()
        .any(|end| word.ends_with(end))
    {
        format!("{word}es")
    } else if let Some(stem) = word.strip_suffix('y')
        && stem.chars().last().is_some_and(|c| !is_vowel(c))
    {
        format!("{stem}ies")
    } else {
        format!("{word}s")
    }
}

/// The -er or -est form of an adjective (`suffix` "er" or "est"); longer
/// adjectives take "more" and "most".
pub fn comparative(adjective: &str, suffix: &str) -> String {
    if let Some((_, er, est)) = IRREGULAR_COMPARISON
        .iter()
        .find(|(base, _, _)| *base == adjective)
    {
        return if suffix == "er" { er } else { est }.to_string();
    }
    let vowels = adjective
        .chars()
        .collect::<Vec<_>>()
        .windows(2)
        .filter(|pair| is_vowel(pair[0]) && !is_vowel(pair[1]))
        .count();
    if vowels > 1 && !adjective.ends_with('y') {
        return format!(
            "{} {adjective}",
            if suffix == "er" { "more" } else { "most" }
        );
    }
    if adjective.ends_with('e') {
        format!("{adjective}{}", &suffix[1..])
    } else if let Some(stem) = adjective.strip_suffix('y') {
        format!("{stem}i{suffix}")
    } else if doubles(adjective) {
        format!(
            "{adjective}{}{suffix}",
            adjective.chars().last().unwrap_or_default()
        )
    } else {
        format!("{adjective}{suffix}")
    }
}

/// The forms `word` (in small letters) may come from, the word itself
/// first: its own entry, an irregular verb's base, and the bases regular
/// suffixes undo.
pub fn bases(word: &str) -> Vec<(String, Inflection)> {
    let mut found: Vec<(String, Inflection)> = vec![(word.to_string(), Inflection::None)];
    let mut push = |base: String, inflection: Inflection| {
        if base.chars().count() >= 2
            && !found
                .iter()
                .any(|(own, i)| *own == base && *i == inflection)
        {
            found.push((base, inflection));
        }
    };
    for (base, past_form, participle) in IRREGULAR {
        if word == *past_form || word == *participle {
            push(base.to_string(), Inflection::IrregularPast);
        }
    }
    for (base, er, est) in IRREGULAR_COMPARISON {
        if word == *er {
            push(base.to_string(), Inflection::Er);
        }
        if word == *est {
            push(base.to_string(), Inflection::Est);
        }
    }
    let undo = |stem: &str| -> Vec<String> {
        let mut forms = vec![stem.to_string(), format!("{stem}e")];
        let letters: Vec<char> = stem.chars().collect();
        if letters.len() >= 2 && letters[letters.len() - 1] == letters[letters.len() - 2] {
            forms.push(letters[..letters.len() - 1].iter().collect());
        }
        forms
    };
    if let Some(stem) = word.strip_suffix("ies") {
        push(format!("{stem}y"), Inflection::S);
    } else if let Some(stem) = word.strip_suffix("es") {
        push(stem.to_string(), Inflection::S);
        push(format!("{stem}e"), Inflection::S);
    }
    if let Some(stem) = word.strip_suffix('s')
        && !word.ends_with("ss")
    {
        push(stem.to_string(), Inflection::S);
    }
    if let Some(stem) = word.strip_suffix("ied") {
        push(format!("{stem}y"), Inflection::Past);
    } else if let Some(stem) = word.strip_suffix("ed") {
        for base in undo(stem) {
            push(base, Inflection::Past);
        }
    }
    if let Some(stem) = word.strip_suffix("ying") {
        push(format!("{stem}ie"), Inflection::Ing);
    }
    if let Some(stem) = word.strip_suffix("ing") {
        for base in undo(stem) {
            push(base, Inflection::Ing);
        }
    }
    if let Some(stem) = word.strip_suffix("iest") {
        push(format!("{stem}y"), Inflection::Est);
    } else if let Some(stem) = word.strip_suffix("est") {
        for base in undo(stem) {
            push(base, Inflection::Est);
        }
    }
    if let Some(stem) = word.strip_suffix("ier") {
        push(format!("{stem}y"), Inflection::Er);
    } else if let Some(stem) = word.strip_suffix("er") {
        for base in undo(stem) {
            push(base, Inflection::Er);
        }
    }
    found
}

/// A synonym in the looked-up word's capitals (Ami Pro's engine: an
/// initial capital and all capitals carry over; VM PSP1).
pub fn in_capitals_of(synonym: &str, word: &str) -> String {
    let letters: Vec<char> = word.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase()) {
        return synonym.to_uppercase();
    }
    if letters.first().is_some_and(|c| c.is_uppercase()) {
        let mut chars = synonym.chars();
        return match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        };
    }
    synonym.to_string()
}

/// The word Ami Pro's thesaurus looks up in `text` at `byte`
/// (`1380:0000`): the word the caret is in, or the one just before it when
/// the caret is on a space or punctuation; letters with apostrophes and
/// hyphens inside. Its byte range.
pub fn word_at(text: &str, byte: usize) -> Option<std::ops::Range<usize>> {
    let byte = byte.min(text.len());
    let is_letter = |c: char| c.is_alphabetic();
    let joins = |c: char| matches!(c, '\'' | '\u{2019}' | '-');
    let mut at = byte;
    // On a space or punctuation (or the end): step back one character.
    let here = text[at..].chars().next();
    if here.is_none_or(|c| !is_letter(c)) {
        let previous = text[..at].chars().next_back()?;
        if !is_letter(previous) {
            // In a run of spaces: the next word.
            let next = text[at..].char_indices().find(|(_, c)| is_letter(*c))?.0;
            at += next;
        } else {
            at -= previous.len_utf8();
        }
    }
    let mut start = at;
    while let Some(c) = text[..start].chars().next_back()
        && (is_letter(c)
            || (joins(c)
                && text[..start - c.len_utf8()]
                    .chars()
                    .next_back()
                    .is_some_and(is_letter)))
    {
        start -= c.len_utf8();
    }
    let mut end = at;
    while let Some(c) = text[end..].chars().next()
        && (is_letter(c)
            || (joins(c)
                && text[end + c.len_utf8()..]
                    .chars()
                    .next()
                    .is_some_and(is_letter)))
    {
        end += c.len_utf8();
    }
    (start < end).then_some(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small thesaurus written for the tests, with its index.
    fn fixture() -> (Index, std::io::Cursor<Vec<u8>>) {
        let mut text = String::from("UTF-8\n");
        let mut index = String::from("UTF-8\n4\n");
        for (word, entry) in [
            (
                "big",
                "big|2\n(adj)|large|great (similar term)\n(adv)|boastfully\n",
            ),
            ("large", "large|1\n(adj)|big|great\n"),
            ("sat", "sat|1\n(noun)|Saturday|weekday (generic term)\n"),
            (
                "sit",
                "sit|2\n(verb)|sit down|pose|model\n(verb)|baby-sit\n",
            ),
        ] {
            index.push_str(&format!("{word}|{}\n", text.len()));
            text.push_str(entry);
        }
        (
            Index::read(index.as_bytes()),
            std::io::Cursor::new(text.into_bytes()),
        )
    }

    #[test]
    fn a_word_is_looked_up_with_its_meanings_and_synonyms() {
        let (index, mut data) = fixture();
        assert_eq!(index.len(), 4);
        let entry = index.look_up(&mut data, "big").unwrap();
        assert_eq!(entry.meanings[0].part_of_speech, "adj");
        assert_eq!(entry.meanings[0].synonyms, ["large", "great"]);
        // Capitals carry over (PSP1: "Another" gave "Like").
        let entry = index.look_up(&mut data, "Big").unwrap();
        assert_eq!(entry.meanings[0].synonyms, ["Large", "Great"]);
        assert_eq!(
            index.look_up(&mut data, "BIG").unwrap().meanings[0].synonyms[0],
            "LARGE"
        );
        assert!(index.look_up(&mut data, "xyzzy").is_none());
    }

    #[test]
    fn an_inflected_word_finds_its_base_and_inflects_the_synonyms() {
        // PSP1: "sat" gave "posed": the noun Sat. first, then sit's verbs in
        // the past.
        let (index, mut data) = fixture();
        let entry = index.look_up(&mut data, "sat").unwrap();
        let parts: Vec<&str> = entry
            .meanings
            .iter()
            .map(|m| m.part_of_speech.as_str())
            .collect();
        assert_eq!(parts, ["noun", "verb", "verb"]);
        assert_eq!(entry.meanings[1].synonyms, ["sat down", "posed", "modeled"]);
        let entry = index.look_up(&mut data, "sitting").unwrap();
        assert_eq!(
            entry.meanings[0].synonyms,
            ["sitting down", "posing", "modeling"]
        );
    }

    #[test]
    fn english_inflections() {
        assert_eq!(past("pose"), "posed");
        assert_eq!(past("stop"), "stopped");
        assert_eq!(past("carry"), "carried");
        assert_eq!(past("go"), "went");
        assert_eq!(ing("make"), "making");
        assert_eq!(ing("run"), "running");
        assert_eq!(ing("lie"), "lying");
        assert_eq!(plural("box"), "boxes");
        assert_eq!(plural("city"), "cities");
        assert_eq!(comparative("big", "er"), "bigger");
        assert_eq!(comparative("happy", "est"), "happiest");
        assert_eq!(comparative("good", "er"), "better");
        assert!(bases("bigger").contains(&("big".into(), Inflection::Er)));
        assert!(bases("went").contains(&("go".into(), Inflection::IrregularPast)));
    }

    #[test]
    fn the_word_at_the_caret_is_ami_pros() {
        let text = "The cat's well-known mat.  Next";
        let word = |byte| word_at(text, byte).map(|range| &text[range]);
        assert_eq!(word(1), Some("The"));
        assert_eq!(word(3), Some("The"), "on the space after: the word before");
        assert_eq!(word(6), Some("cat's"));
        assert_eq!(word(12), Some("well-known"));
        assert_eq!(word(24), Some("mat"), "on the full stop");
        assert_eq!(word(26), Some("Next"), "in a run of spaces: the next");
    }
}
