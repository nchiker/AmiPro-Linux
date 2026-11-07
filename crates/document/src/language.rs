//! The document's language (`[lang]`) and Ami Pro's language numbers
//! (`docs/research/PROOFING_NOTES.md`, "Languages"; decision SP6).
//!
//! Ami Pro stores one language per document, a number from 1 to 28 (0 is
//! none, and its spell check then refuses to run). The app checks spelling
//! with any installed Hunspell dictionary, named by its locale (`en_US`);
//! a document in a language Ami Pro has no number for keeps the locale in
//! the native format and writes the nearest number to SAM.

/// Ami Pro's languages by number (1–28): the name its dialogs show and the
/// Hunspell locale the app checks with.
pub const AMI_PRO_LANGUAGES: [(u16, &str, &str); 28] = [
    (1, "American", "en_US"),
    (2, "British", "en_GB"),
    (3, "French", "fr_FR"),
    (4, "Canadian-French", "fr_CA"),
    (5, "Italian", "it_IT"),
    (6, "Spanish", "es_ES"),
    (7, "German", "de_DE"),
    (8, "Dutch General", "nl_NL"),
    (9, "Norwegian", "nb_NO"),
    (10, "Swedish", "sv_SE"),
    (11, "Danish", "da_DK"),
    (12, "Portuguese", "pt_PT"),
    (13, "Finnish", "fi_FI"),
    (14, "American Medical", "en_US"),
    (15, "Swiss German", "de_CH"),
    (16, "Greek", "el_GR"),
    (17, "Brazilian", "pt_BR"),
    (18, "Australian", "en_AU"),
    (19, "Polish", "pl_PL"),
    (20, "Russian", "ru_RU"),
    (21, "Catalan", "ca_ES"),
    (22, "Nynorsk", "nn_NO"),
    (23, "Dutch Preferred", "nl_NL"),
    (24, "British Medical", "en_GB"),
    (25, "Czech", "cs_CZ"),
    (26, "Russian Jo", "ru_RU"),
    (27, "British IZE", "en_GB"),
    (28, "British Medical IZE", "en_GB"),
];

/// Ami Pro's American English, the language of new documents
/// (`[AmiPro] dictlang`, default 1).
pub const AMERICAN: u16 = 1;

/// A document's language.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Language {
    /// Ami Pro's number, written as `[lang]`: 0 is none.
    pub number: u16,
    /// A locale Ami Pro has no number for (native format only); `number`
    /// is then the nearest Ami Pro language.
    pub locale: Option<String>,
}

impl Default for Language {
    fn default() -> Self {
        Self::ami_pro(AMERICAN)
    }
}

impl Language {
    /// Ami Pro's language `number`.
    pub fn ami_pro(number: u16) -> Self {
        Self {
            number,
            locale: None,
        }
    }

    /// The language checked with `locale` (`en_US`, `fr_BE`): Ami Pro's own
    /// number when one has that locale, else the nearest number with the
    /// locale kept.
    pub fn from_locale(locale: &str) -> Self {
        if let Some(number) = ami_pro_number(locale) {
            return Self::ami_pro(number);
        }
        Self {
            number: nearest_number(locale),
            locale: Some(locale.to_string()),
        }
    }

    /// The Hunspell locale to check with, if any (none for Ami Pro's 0 or
    /// an unknown number).
    pub fn locale(&self) -> Option<&str> {
        self.locale
            .as_deref()
            .or_else(|| ami_pro_locale(self.number))
    }

    /// Whether SAM can hold this language exactly.
    pub fn is_ami_pros(&self) -> bool {
        self.locale.is_none()
    }

    /// The name shown for the language: Ami Pro's, or the locale's.
    pub fn name(&self) -> String {
        match &self.locale {
            Some(locale) => locale_name(locale),
            None => ami_pro_name(self.number)
                .map(str::to_string)
                .unwrap_or_else(|| "None".into()),
        }
    }
}

/// Ami Pro's name of language `number`.
pub fn ami_pro_name(number: u16) -> Option<&'static str> {
    AMI_PRO_LANGUAGES
        .iter()
        .find(|(n, _, _)| *n == number)
        .map(|(_, name, _)| *name)
}

/// The locale the app checks Ami Pro's language `number` with.
pub fn ami_pro_locale(number: u16) -> Option<&'static str> {
    AMI_PRO_LANGUAGES
        .iter()
        .find(|(n, _, _)| *n == number)
        .map(|(_, _, locale)| *locale)
}

/// The first Ami Pro number with exactly `locale` (its main variant: en_GB
/// is British, not British Medical).
pub fn ami_pro_number(locale: &str) -> Option<u16> {
    let wanted = normalise(locale);
    AMI_PRO_LANGUAGES
        .iter()
        .find(|(_, _, own)| normalise(own) == wanted)
        .map(|(number, _, _)| *number)
}

/// The nearest Ami Pro number for `locale`: the first with its language
/// (fr_BE is French), else American, so Ami Pro still checks the document
/// (decision SK1).
pub fn nearest_number(locale: &str) -> u16 {
    let language = |locale: &str| {
        normalise(locale)
            .split('_')
            .next()
            .unwrap_or("")
            .to_string()
    };
    let wanted = language(locale);
    AMI_PRO_LANGUAGES
        .iter()
        .find(|(_, _, own)| language(own) == wanted)
        .map_or(AMERICAN, |(number, _, _)| *number)
}

/// `en-us`, `en_US.UTF-8` → `en_US`.
fn normalise(locale: &str) -> String {
    let base = locale.split(['.', '@']).next().unwrap_or(locale);
    let mut parts = base.split(['_', '-']);
    let language = parts.next().unwrap_or("").to_ascii_lowercase();
    let rest: Vec<String> = parts.map(str::to_ascii_uppercase).collect();
    if rest.is_empty() {
        language
    } else {
        format!("{language}_{}", rest.join("_"))
    }
}

/// A readable name for a locale: Ami Pro's when one has it, else the
/// language and region in English where known, else the locale itself.
pub fn locale_name(locale: &str) -> String {
    if let Some(name) = ami_pro_number(locale).and_then(ami_pro_name) {
        return name.to_string();
    }
    const LANGUAGES: [(&str, &str); 30] = [
        ("af", "Afrikaans"),
        ("ar", "Arabic"),
        ("bg", "Bulgarian"),
        ("ca", "Catalan"),
        ("cs", "Czech"),
        ("cy", "Welsh"),
        ("da", "Danish"),
        ("de", "German"),
        ("el", "Greek"),
        ("en", "English"),
        ("es", "Spanish"),
        ("et", "Estonian"),
        ("fi", "Finnish"),
        ("fr", "French"),
        ("ga", "Irish"),
        ("he", "Hebrew"),
        ("hr", "Croatian"),
        ("hu", "Hungarian"),
        ("is", "Icelandic"),
        ("it", "Italian"),
        ("lt", "Lithuanian"),
        ("lv", "Latvian"),
        ("nb", "Norwegian"),
        ("nl", "Dutch"),
        ("pl", "Polish"),
        ("pt", "Portuguese"),
        ("ro", "Romanian"),
        ("ru", "Russian"),
        ("sk", "Slovak"),
        ("sv", "Swedish"),
    ];
    let normal = normalise(locale);
    let mut parts = normal.splitn(2, '_');
    let language = parts.next().unwrap_or("");
    let region = parts.next();
    match LANGUAGES.iter().find(|(code, _)| *code == language) {
        Some((_, name)) => match region {
            Some(region) => format!("{name} ({region})"),
            None => name.to_string(),
        },
        None => locale.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ami_pros_numbers_map_to_locales_and_back() {
        assert_eq!(Language::default().locale(), Some("en_US"));
        assert_eq!(Language::ami_pro(2).locale(), Some("en_GB"));
        assert_eq!(Language::ami_pro(0).locale(), None);
        assert_eq!(Language::from_locale("en_GB"), Language::ami_pro(2));
        assert_eq!(Language::from_locale("pt-BR"), Language::ami_pro(17));
        assert_eq!(Language::ami_pro(27).name(), "British IZE");
    }

    #[test]
    fn other_locales_keep_their_locale_and_the_nearest_number() {
        let belgian = Language::from_locale("fr_BE");
        assert_eq!(belgian.number, 3);
        assert_eq!(belgian.locale(), Some("fr_BE"));
        assert!(!belgian.is_ami_pros());
        assert_eq!(belgian.name(), "French (BE)");
        assert_eq!(Language::from_locale("ar_EG").number, AMERICAN);
    }
}
