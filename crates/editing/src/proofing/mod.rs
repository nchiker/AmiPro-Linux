//! Proofing: spelling, the user dictionary, the thesaurus, readability, and
//! the document's language and grammar set (`docs/Design/UI_SPEC.md` §35;
//! `docs/research/PROOFING_NOTES.md`; decisions SP1–SP8).
//!
//! Everything here is independent of the engines: the app checks words with
//! Hunspell dictionaries and grammar with Harper behind the traits below,
//! as the layout shapes and hyphenates through `Shaper`.

use amipro_document::{Document, Language};

pub mod readability;
pub mod spelling;
pub mod thesaurus;

use crate::{Applied, ChangeReport, EditError};

/// Tools › Language…: the document's language (`[lang]`). Hyphenation
/// follows it, so every paragraph may look different.
pub(crate) fn set_language(document: &Document, language: &Language) -> Result<Applied, EditError> {
    if language.number > 28 {
        return Err("Ami Pro's languages are 0 to 28");
    }
    let mut next = document.clone();
    next.set_language(language.clone());
    Ok(Applied {
        document: next,
        caret: None,
        report: ChangeReport {
            styles: true,
            ..ChangeReport::default()
        },
    })
}

/// The grammar and style set a grammar check used (`[GramStyle]`), as Ami
/// Pro records it at the check's start (`1330:004b`).
pub(crate) fn set_grammar_set(document: &Document, name: &str) -> Result<Applied, EditError> {
    let name: String = name.chars().filter(|c| !c.is_control()).take(24).collect();
    let mut next = document.clone();
    next.set_grammar_set(name);
    Ok(Applied {
        document: next,
        caret: None,
        report: ChangeReport {
            info: true,
            ..ChangeReport::default()
        },
    })
}
