//! The spelling engine (`UI_SPEC.md` §35; decisions SP1, SP5): Hunspell
//! dictionaries found on the system and the bundled en-US one, read with
//! `spellbook`, and the user's own dictionaries in Ami Pro's format.
//!
//! Dictionaries load off the interface's thread (the en-US one takes a few
//! tens of milliseconds) and are kept for the session; the checks
//! themselves are `amipro_editing::proofing::spelling`'s.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use amipro_editing::proofing::spelling::{self, Speller};

/// The bundled en-US dictionary (SCOWL; `dictionaries/README.md`).
const BUNDLED_AFF: &str = include_str!("../dictionaries/en_US.aff");
const BUNDLED_DIC: &str = include_str!("../dictionaries/en_US.dic");

/// Where a dictionary's files are.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Source {
    Files { aff: PathBuf, dic: PathBuf },
    Bundled,
}

/// A dictionary that can be loaded: its locale (the file's name, `en_US`)
/// and where it is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Available {
    pub locale: String,
    pub source: Source,
}

/// The folders searched for Hunspell dictionaries, the user's first.
pub fn dictionary_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    if let Some(data) = data {
        folders.push(data.join("hunspell"));
    }
    folders.extend(
        [
            "/usr/share/hunspell",
            "/usr/share/myspell",
            "/usr/share/myspell/dicts",
            "/usr/local/share/hunspell",
        ]
        .map(PathBuf::from),
    );
    folders
}

/// Every dictionary in `folders` (a `.dic` with its `.aff`), the first
/// found for each locale, and the bundled en-US one when no folder has it;
/// sorted by locale.
pub fn available_in(folders: &[PathBuf]) -> Vec<Available> {
    let mut found: BTreeMap<String, Available> = BTreeMap::new();
    for folder in folders {
        let Ok(entries) = std::fs::read_dir(folder) else {
            continue;
        };
        let mut names: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
        names.sort();
        for dic in names {
            if dic.extension().is_none_or(|extension| extension != "dic") {
                continue;
            }
            let aff = dic.with_extension("aff");
            let Some(locale) = dic.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            // Hyphenation patterns share the folder in some systems.
            if locale.starts_with("hyph_") || !aff.is_file() {
                continue;
            }
            found.entry(locale.to_string()).or_insert(Available {
                locale: locale.to_string(),
                source: Source::Files { aff, dic },
            });
        }
    }
    found.entry("en_US".into()).or_insert(Available {
        locale: "en_US".into(),
        source: Source::Bundled,
    });
    found.into_values().collect()
}

/// The dictionaries on this system.
pub fn available() -> Vec<Available> {
    available_in(&dictionary_folders())
}

/// The dictionary to check `locale` with: that locale, else one of its
/// language (`fr` for `fr_FR`, `fr_FR` for `fr_BE`), else none.
pub fn dictionary_for<'a>(available: &'a [Available], locale: &str) -> Option<&'a Available> {
    let language = locale.split(['_', '-']).next().unwrap_or(locale);
    available
        .iter()
        .find(|dictionary| dictionary.locale == locale)
        .or_else(|| {
            available
                .iter()
                .find(|dictionary| dictionary.locale == language)
        })
        .or_else(|| {
            available.iter().find(|dictionary| {
                dictionary.locale.split(['_', '-']).next() == Some(language)
                    && !dictionary.locale.contains("frami")
            })
        })
        .or_else(|| {
            available
                .iter()
                .find(|dictionary| dictionary.locale.split(['_', '-']).next() == Some(language))
        })
}

/// A dictionary file's text: UTF-8, or ISO 8859-1 (Hunspell's default) when
/// the affix file names no other encoding.
fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => error
            .into_bytes()
            .iter()
            .map(|&byte| byte as char)
            .collect(),
    })
}

/// A loaded dictionary.
pub struct Dictionary {
    pub locale: String,
    inner: spellbook::Dictionary,
}

impl std::fmt::Debug for Dictionary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Dictionary")
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

impl Dictionary {
    /// Reads `available`'s files.
    pub fn load(available: &Available) -> Result<Self, String> {
        let (aff, dic) = match &available.source {
            Source::Bundled => (BUNDLED_AFF.to_string(), BUNDLED_DIC.to_string()),
            Source::Files { aff, dic } => (read_text(aff)?, read_text(dic)?),
        };
        let aff = aff.trim_start_matches('\u{FEFF}');
        let encoding = aff
            .lines()
            .find_map(|line| line.strip_prefix("SET "))
            .map(|set| set.trim().to_ascii_uppercase());
        if let Some(encoding) = &encoding
            && !matches!(encoding.as_str(), "UTF-8" | "ISO8859-1" | "ISO-8859-1")
        {
            return Err(format!("{}: the {encoding} encoding", available.locale));
        }
        let inner = spellbook::Dictionary::new(aff, dic.trim_start_matches('\u{FEFF}'))
            .map_err(|error| format!("{}: {error}", available.locale))?;
        Ok(Self {
            locale: available.locale.clone(),
            inner,
        })
    }

    pub fn check(&self, word: &str) -> bool {
        self.inner.check(word)
    }

    pub fn suggest(&self, word: &str) -> Vec<String> {
        let mut found = Vec::new();
        self.inner.suggest(word, &mut found);
        found
    }
}

/// A user dictionary: the words the user added for one language (SP5), in
/// Ami Pro's format, in the app's data folder (none in tests: kept in
/// memory).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UserDictionary {
    pub words: BTreeSet<String>,
    pub path: Option<PathBuf>,
}

impl UserDictionary {
    /// The user dictionary for `locale` in `folder`, read if it exists.
    pub fn open(folder: Option<&Path>, locale: &str) -> Self {
        let path = folder.map(|folder| folder.join(format!("{locale}.dic")));
        let words = path
            .as_ref()
            .and_then(|path| std::fs::read(path).ok())
            .map(|bytes| spelling::read_word_list(&bytes).into_iter().collect())
            .unwrap_or_default();
        Self { words, path }
    }

    /// Whether the user added `word`, or its small-letter form when it
    /// starts a sentence or is in capitals.
    pub fn knows(&self, word: &str) -> bool {
        self.words.contains(word) || self.words.contains(&word.to_lowercase())
    }

    /// Adds words and writes the file; `false` if none was new.
    pub fn add(&mut self, words: impl IntoIterator<Item = String>) -> Result<bool, String> {
        let mut added = false;
        for word in words {
            let word = word.trim().to_string();
            if !word.is_empty() && !word.contains(char::is_whitespace) {
                added |= self.words.insert(word);
            }
        }
        if added {
            self.save()?;
        }
        Ok(added)
    }

    pub fn remove(&mut self, word: &str) -> Result<(), String> {
        if self.words.remove(word) {
            self.save()?;
        }
        Ok(())
    }

    /// The file's text: Ami Pro's format.
    pub fn text(&self) -> String {
        spelling::write_word_list(&self.words.iter().cloned().collect::<Vec<_>>())
    }

    fn save(&self) -> Result<(), String> {
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(folder) = path.parent() {
            std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        }
        let temporary = path.with_extension("dic.new");
        std::fs::write(&temporary, self.text()).map_err(|error| error.to_string())?;
        std::fs::rename(&temporary, path).map_err(|error| error.to_string())
    }
}

/// The app's user dictionaries' folder: `$XDG_DATA_HOME/amipro-linux/dictionaries`.
pub fn user_dictionary_folder() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .map(|data| data.join("amipro-linux").join("dictionaries"))
}

/// A dictionary with the user's words: what a check asks.
pub struct SessionSpeller<'a> {
    pub dictionary: &'a Dictionary,
    pub user: &'a UserDictionary,
    /// Include user dictionary alternatives (Ami Pro's option): the user's
    /// words close to the word follow the dictionary's.
    pub user_alternatives: bool,
}

impl Speller for SessionSpeller<'_> {
    fn check(&self, word: &str) -> bool {
        self.user.knows(word) || self.dictionary.check(word)
    }

    fn suggest(&self, word: &str) -> Vec<String> {
        let mut found = self.dictionary.suggest(word);
        if self.user_alternatives {
            let lower = word.to_lowercase();
            found.extend(
                self.user
                    .words
                    .iter()
                    .filter(|known| edit_distance(&known.to_lowercase(), &lower) <= 2)
                    .cloned(),
            );
        }
        found
    }
}

/// Levenshtein distance over characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let current = row[j + 1];
            row[j + 1] = (previous + usize::from(ca != *cb))
                .min(row[j] + 1)
                .min(current + 1);
            previous = current;
        }
    }
    row[b.len()]
}

/// The session's dictionaries: those found, those loaded, and the user's
/// (decisions SP1, SP5).
#[derive(Debug, Default)]
pub struct Spelling {
    pub available: Vec<Available>,
    pub loaded: BTreeMap<String, Arc<Dictionary>>,
    /// Locales whose dictionary is loading or failed to load (the error).
    pub pending: BTreeMap<String, Option<String>>,
    pub user: BTreeMap<String, UserDictionary>,
    /// The user dictionaries' folder; none in tests (kept in memory).
    pub user_folder: Option<PathBuf>,
    /// The thesauri's folders (none in tests), the loaded thesauri by the
    /// document's locale, and those loading or failed.
    pub thesaurus_folders: Vec<PathBuf>,
    pub thesauri: BTreeMap<String, Arc<Thesaurus>>,
    pub thesauri_pending: BTreeMap<String, Option<String>>,
    /// Harper's grammar checker, once made (SP8), and whether it is being
    /// made.
    pub grammar: Option<Arc<crate::grammar::Grammar>>,
    pub grammar_loading: bool,
}

impl Spelling {
    pub fn new(available: Vec<Available>, user_folder: Option<PathBuf>) -> Self {
        Self {
            available,
            user_folder,
            ..Self::default()
        }
    }

    /// The thesaurus for `locale`, if loaded.
    pub fn thesaurus(&self, locale: &str) -> Option<Arc<Thesaurus>> {
        self.thesauri.get(locale).cloned()
    }

    /// The files to load for `locale`'s thesaurus, if it is neither loaded
    /// nor loading; marks it loading. A locale without one is recorded so.
    pub fn begin_thesaurus(&mut self, locale: &str) -> Option<(PathBuf, PathBuf)> {
        if self.thesauri.contains_key(locale) || self.thesauri_pending.contains_key(locale) {
            return None;
        }
        match thesaurus_files(&self.thesaurus_folders, locale) {
            Some(files) => {
                self.thesauri_pending.insert(locale.to_string(), None);
                Some(files)
            }
            None => {
                self.thesauri_pending
                    .insert(locale.to_string(), Some("no thesaurus".into()));
                None
            }
        }
    }

    /// Why `locale` has no thesaurus, once known.
    pub fn thesaurus_problem(&self, locale: &str) -> Option<&str> {
        self.thesauri_pending.get(locale)?.as_deref()
    }

    /// The dictionary for `locale` if it is loaded.
    pub fn dictionary(&self, locale: &str) -> Option<Arc<Dictionary>> {
        let available = dictionary_for(&self.available, locale)?;
        self.loaded.get(&available.locale).cloned()
    }

    /// The dictionary to load for `locale`, if it is neither loaded nor
    /// loading (`None` also when there is none).
    pub fn begin_load(&mut self, locale: &str) -> Option<Available> {
        let available = dictionary_for(&self.available, locale)?.clone();
        if self.loaded.contains_key(&available.locale)
            || self.pending.contains_key(&available.locale)
        {
            return None;
        }
        self.pending.insert(available.locale.clone(), None);
        Some(available)
    }

    /// A load finished.
    pub fn loaded(&mut self, locale: String, result: Result<Arc<Dictionary>, String>) {
        match result {
            Ok(dictionary) => {
                self.pending.remove(&locale);
                self.loaded.insert(locale, dictionary);
            }
            Err(error) => {
                self.pending.insert(locale, Some(error));
            }
        }
    }

    /// The user's dictionary for `locale` (its language's: en_US and en_GB
    /// share none).
    pub fn user(&mut self, locale: &str) -> &mut UserDictionary {
        let folder = self.user_folder.clone();
        self.user
            .entry(locale.to_string())
            .or_insert_with(|| UserDictionary::open(folder.as_deref(), locale))
    }

    /// Whether any dictionary serves `locale`.
    pub fn has_dictionary(&self, locale: &str) -> bool {
        dictionary_for(&self.available, locale).is_some()
    }
}

/// The package that would give `locale` a dictionary (Debian and Ubuntu's
/// names), for the panel's message.
pub fn package_for(locale: &str) -> String {
    let normal = locale.to_lowercase().replace('_', "-");
    format!("hunspell-{normal}")
}

/// Loads `available` (on a worker thread).
pub fn load(available: Available) -> (String, Result<Arc<Dictionary>, String>) {
    let locale = available.locale.clone();
    (locale, Dictionary::load(&available).map(Arc::new))
}

/// The folders searched for MyThes thesauri, the user's first (SP7).
pub fn thesaurus_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    if let Some(data) = data {
        folders.push(data.join("mythes"));
    }
    folders.extend(
        [
            "/usr/share/mythes",
            "/usr/share/myspell/dicts",
            "/usr/local/share/mythes",
        ]
        .map(PathBuf::from),
    );
    folders
}

/// A thesaurus's files for `locale` in `folders`: `th_LOCALE_v2.idx` and
/// `.dat` (or without `_v2`), else one of its language.
pub fn thesaurus_files(folders: &[PathBuf], locale: &str) -> Option<(PathBuf, PathBuf)> {
    let language = locale.split(['_', '-']).next().unwrap_or(locale);
    let pair = |folder: &Path, name: &str| {
        let idx = folder.join(format!("{name}.idx"));
        let dat = folder.join(format!("{name}.dat"));
        (idx.is_file() && dat.is_file()).then_some((idx, dat))
    };
    for folder in folders {
        for name in [format!("th_{locale}_v2"), format!("th_{locale}")] {
            if let Some(found) = pair(folder, &name) {
                return Some(found);
            }
        }
    }
    for folder in folders {
        let Ok(entries) = std::fs::read_dir(folder) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with(&format!("th_{language}_")) && name.ends_with(".idx"))
            .collect();
        names.sort();
        if let Some(name) = names.first()
            && let Some(found) = pair(folder, name.trim_end_matches(".idx"))
        {
            return Some(found);
        }
    }
    None
}

/// A loaded thesaurus: its index in memory, its data read as looked up.
pub struct Thesaurus {
    pub locale: String,
    index: amipro_editing::proofing::thesaurus::Index,
    data: PathBuf,
}

impl std::fmt::Debug for Thesaurus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Thesaurus")
            .field("locale", &self.locale)
            .finish_non_exhaustive()
    }
}

impl Thesaurus {
    pub fn load(locale: &str, idx: &Path, dat: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(idx).map_err(|error| format!("{}: {error}", idx.display()))?;
        let index = amipro_editing::proofing::thesaurus::Index::read(&bytes);
        if index.is_empty() {
            return Err(format!("{}: no entries", idx.display()));
        }
        Ok(Self {
            locale: locale.to_string(),
            index,
            data: dat.to_path_buf(),
        })
    }

    /// Ami Pro's lookup of `word` (SP7).
    pub fn look_up(&self, word: &str) -> Option<amipro_editing::proofing::thesaurus::Entry> {
        let mut data = std::fs::File::open(&self.data).ok()?;
        self.index.look_up(&mut data, word)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small dictionary written for the tests (not from any word list).
    pub(crate) const TEST_AFF: &str =
        "SET UTF-8\nTRY esianrtolcdugmphbyfvkwz\nSFX S Y 1\nSFX S 0 s .\n";
    pub(crate) const TEST_DIC: &str =
        "12\nthe\ncat/S\nsat\non\nmat/S\nwe\nreceive\nand\ntoday\nanother\nline/S\nword/S\n";

    pub(crate) fn test_dictionary() -> Arc<Dictionary> {
        Arc::new(Dictionary {
            locale: "en_US".into(),
            inner: spellbook::Dictionary::new(TEST_AFF, TEST_DIC).unwrap(),
        })
    }

    /// The session's spelling with the test dictionary loaded as en-US and
    /// user dictionaries kept in memory.
    pub(crate) fn test_spelling() -> Spelling {
        let mut spelling = Spelling::new(
            vec![Available {
                locale: "en_US".into(),
                source: Source::Bundled,
            }],
            None,
        );
        spelling.loaded("en_US".into(), Ok(test_dictionary()));
        spelling
    }

    #[test]
    fn a_dictionary_checks_and_suggests() {
        let dictionary = test_dictionary();
        assert!(dictionary.check("cats"));
        assert!(dictionary.check("The"));
        assert!(!dictionary.check("Teh"));
        assert!(dictionary.suggest("teh").iter().any(|word| word == "the"));
        let user = UserDictionary {
            words: ["Xyzzy".to_string()].into(),
            path: None,
        };
        let speller = SessionSpeller {
            dictionary: &dictionary,
            user: &user,
            user_alternatives: true,
        };
        assert!(speller.check("Xyzzy"));
        assert!(speller.suggest("Xyzzz").contains(&"Xyzzy".to_string()));
    }

    #[test]
    fn dictionaries_are_found_by_locale_with_the_bundled_one_last() {
        let folder = std::env::temp_dir().join(format!("amipro-dicts-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        for name in ["fr_FR", "de_DE_frami", "hyph_en_US"] {
            std::fs::write(folder.join(format!("{name}.aff")), TEST_AFF).unwrap();
            std::fs::write(folder.join(format!("{name}.dic")), TEST_DIC).unwrap();
        }
        let available = available_in(std::slice::from_ref(&folder));
        let locales: Vec<&str> = available.iter().map(|a| a.locale.as_str()).collect();
        assert_eq!(locales, ["de_DE_frami", "en_US", "fr_FR"]);
        assert_eq!(available[1].source, Source::Bundled);
        assert_eq!(dictionary_for(&available, "fr_BE").unwrap().locale, "fr_FR");
        assert_eq!(
            dictionary_for(&available, "de_DE").unwrap().locale,
            "de_DE_frami"
        );
        assert!(dictionary_for(&available, "it_IT").is_none());
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn the_bundled_dictionary_loads() {
        let dictionary = Dictionary::load(&Available {
            locale: "en_US".into(),
            source: Source::Bundled,
        })
        .unwrap();
        assert!(dictionary.check("receive"));
        assert!(!dictionary.check("recieve"));
        assert!(
            dictionary
                .suggest("recieve")
                .contains(&"receive".to_string())
        );
    }

    #[test]
    fn user_dictionaries_are_written_in_ami_pros_format() {
        let folder = std::env::temp_dir().join(format!("amipro-user-{}", std::process::id()));
        let mut user = UserDictionary::open(Some(&folder), "en_US");
        assert!(
            user.add(["abc123".to_string(), "Xyzzy".to_string()])
                .unwrap()
        );
        assert!(!user.add(["abc123".to_string()]).unwrap());
        assert_eq!(
            std::fs::read_to_string(folder.join("en_US.dic")).unwrap(),
            "Xyzzy\r\nabc123\r\n"
        );
        user.remove("Xyzzy").unwrap();
        assert_eq!(UserDictionary::open(Some(&folder), "en_US").words.len(), 1);
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
