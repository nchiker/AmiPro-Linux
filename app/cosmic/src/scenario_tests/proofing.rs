//! Proofing (`UI_SPEC.md` §35): Spell Check, Thesaurus, Grammar Check,
//! Readability Statistics, and Language, through the Proofing panel.
//!
//! The dictionary is the tests' small one (kept in memory, so no user
//! dictionary file is ever written), the thesaurus a small MyThes pair in a
//! scratch folder, and Harper the shared one.

use super::*;
use crate::proofing_panel::{GrammarState, Page, ProofingAction as P, SpellOption, SpellState};

fn proof(s: &mut Scenario, what: &str, action: P) {
    s.act(what, move |rt| {
        rt.send(Message::Proofing(action));
    });
}

fn proof_edit(s: &mut Scenario, what: &str, action: P) {
    s.edit(what, move |rt| {
        rt.send(Message::Proofing(action));
    });
}

fn spelling_scenario(name: &'static str) -> Scenario {
    let mut s = Scenario::new_document(name);
    s.clean_folder();
    s.app_mut().spelling = crate::spelling::tests::test_spelling();
    s
}

fn word(s: &Scenario) -> Option<String> {
    s.app()
        .proofing
        .as_ref()?
        .spelling
        .as_ref()?
        .problem
        .as_ref()
        .map(|problem| problem.word.clone())
}

fn spell_state(s: &Scenario) -> SpellState {
    s.app()
        .proofing
        .as_ref()
        .and_then(|panel| panel.spelling.as_ref())
        .map(|check| check.state.clone())
        .unwrap_or_else(|| s.fail("spelling", "no spell check is open"))
}

fn caret_at(s: &mut Scenario, paragraph: usize, byte: usize) {
    let main = s.main_flow();
    s.caret(main, paragraph, byte, 1, false);
}

#[test]
fn spell_check_replaces_skips_adds_and_ignores_through_the_proofing_panel() {
    let mut s = spelling_scenario("proofing-spell");
    s.type_text("Teh cat sat on the the mat. We recieve Xyzzy today.");
    s.enter();
    s.type_text("Another Teh line, and teh end.");
    caret_at(&mut s, 0, 0);

    // Tools › Spell Check…: the first problem is selected on the page.
    s.command_act(CommandId::Spelling);
    assert_eq!(word(&s).as_deref(), Some("Teh"), "{}", s.app().status);
    let selection = s.app().page_selection.ordered();
    assert_eq!((selection.0.byte, selection.1.byte), (0, 3));
    assert_eq!(
        s.app()
            .proofing
            .as_ref()
            .unwrap()
            .spelling
            .as_ref()
            .unwrap()
            .alternatives
            .first()
            .map(String::as_str),
        Some("The"),
        "alternatives in the word's capitals"
    );

    // Choosing an alternative twice replaces (Ami Pro's double-click): one
    // undo step.
    s.edit("Choose The twice", |rt| {
        rt.send(Message::Proofing(P::Choose(0)));
        rt.send(Message::Proofing(P::Choose(0)));
    });
    assert!(s.main_text().starts_with("The cat"), "{}", s.main_text());
    // The repeated "the the": Replace with nothing removes the repeat.
    assert_eq!(word(&s).as_deref(), Some("the"));
    proof_edit(&mut s, "Replace with nothing", P::Replace);
    assert!(
        s.main_text().starts_with("The cat sat on the mat."),
        "{}",
        s.main_text()
    );
    // Replace All for "recieve" typed in the field.
    assert_eq!(word(&s).as_deref(), Some("recieve"));
    proof(&mut s, "type receive", P::ReplaceWith("receive".into()));
    proof_edit(&mut s, "Replace All", P::ReplaceAll);
    assert!(s.main_text().contains("We receive Xyzzy"));
    // Add to Dictionary: Xyzzy is known from now on.
    assert_eq!(word(&s).as_deref(), Some("Xyzzy"));
    proof(&mut s, "Add to Dictionary", P::AddToDictionary);
    assert!(s.app().spelling.user["en_US"].words.contains("Xyzzy"));
    // Skip, then Skip All: the Tehs of the second paragraph are passed.
    assert_eq!(word(&s).as_deref(), Some("Teh"));
    proof(&mut s, "Skip", P::Skip);
    assert_eq!(word(&s).as_deref(), Some("teh"));
    proof(&mut s, "Skip All", P::SkipAll);
    // (The small test dictionary lacks other words too: skip them.)
    for _ in 0..6 {
        if spell_state(&s) == SpellState::Complete {
            break;
        }
        proof(&mut s, "Skip", P::Skip);
    }
    assert_eq!(spell_state(&s), SpellState::Complete);
    assert_eq!(s.app().status, "Spell check complete.");

    // Typing in the page with the panel open; Start again finds it.
    s.caret_to_end();
    s.type_text(" Anotherr misspelling.");
    caret_at(&mut s, 0, 0);
    proof(&mut s, "Start again", P::Start);
    assert!(word(&s).is_some(), "{}", s.app().status);
    // Check words with initial caps off leaves "Xyzzy" out (and options are
    // Ami Pro's).
    proof(
        &mut s,
        "Initial caps off",
        P::SpellOption(SpellOption::InitialCaps, false),
    );
    // The user dictionary page: add and remove a word.
    proof(&mut s, "show the dictionary", P::ToggleDictionary);
    proof(&mut s, "type a word", P::UserWord("Frobnicate".into()));
    proof(&mut s, "add it", P::AddUserWord);
    assert!(s.app().spelling.user["en_US"].words.contains("Frobnicate"));
    proof(&mut s, "remove it", P::RemoveUserWord("Frobnicate".into()));
    assert!(!s.app().spelling.user["en_US"].words.contains("Frobnicate"));

    // A selection is checked alone.
    s.select_main((0, 0), (0, 3));
    proof(&mut s, "Start on a selection", P::Start);
    let path = s.save_sam("SPELL.SAM");
    let s = s.reopen(&path);
    assert!(s.main_text().contains("receive"));
}

#[test]
fn spell_check_through_typed_text_with_undo_after_each_replacement() {
    let mut s = spelling_scenario("proofing-spell-undo");
    s.type_text("Thsi is teh test of wrods.");
    caret_at(&mut s, 0, 0);
    s.command_act(CommandId::Spelling);
    let mut fixed = 0;
    for _ in 0..6 {
        let Some(wrong) = word(&s) else { break };
        let replacement = s
            .app()
            .proofing
            .as_ref()
            .unwrap()
            .spelling
            .as_ref()
            .unwrap()
            .alternatives
            .first()
            .cloned();
        match replacement {
            Some(replacement) => {
                proof(&mut s, "type the replacement", P::ReplaceWith(replacement));
                proof_edit(&mut s, &format!("Replace {wrong}"), P::Replace);
                fixed += 1;
            }
            None => proof(&mut s, &format!("Skip {wrong}"), P::Skip),
        }
    }
    assert!(fixed >= 2, "{fixed} replaced; {}", s.main_text());
    let path = s.save_sam("SPELL2.SAM");
    s.reopen(&path);
}

#[test]
fn the_thesaurus_looks_up_and_replaces_synonyms() {
    let mut s = spelling_scenario("proofing-thesaurus");
    s.type_text("A Big house where we sat.");
    super::super::test_thesaurus(s.app_mut(), "scenario");
    // The caret just after the word: Ami Pro looks up the word before it.
    caret_at(&mut s, 0, 5);
    s.command_act(CommandId::Thesaurus);
    let panel = s.app().proofing.as_ref().unwrap();
    assert_eq!(panel.page, Page::Thesaurus);
    assert_eq!(panel.thesaurus.looked_up, "Big");
    // Capitals follow the word; the first meaning's first synonym is ready.
    assert_eq!(panel.thesaurus.replace_with, "Large");
    proof(&mut s, "second meaning", P::Meaning(1));
    assert_eq!(
        s.app().proofing.as_ref().unwrap().thesaurus.replace_with,
        "Important"
    );
    s.edit("press the synonym twice", |rt| {
        rt.send(Message::Proofing(P::Synonym(0)));
        rt.send(Message::Proofing(P::Synonym(0)));
    });
    assert_eq!(s.main_text(), "A Important house where we sat.");
    // Typing a word to look up, then Replace with.
    proof(
        &mut s,
        "type large",
        P::ThesaurusReplaceWith("large".into()),
    );
    proof(&mut s, "Look Up", P::LookUp);
    assert_eq!(
        s.app().proofing.as_ref().unwrap().thesaurus.looked_up,
        "large"
    );
    proof(
        &mut s,
        "type great",
        P::ThesaurusReplaceWith("great".into()),
    );
    proof_edit(&mut s, "Replace", P::ThesaurusReplace);
    assert_eq!(s.main_text(), "A great house where we sat.");
    // An unknown word says so.
    proof(
        &mut s,
        "type xyzzy",
        P::ThesaurusReplaceWith("xyzzy".into()),
    );
    proof(&mut s, "Look Up", P::LookUp);
    assert_eq!(
        s.app()
            .proofing
            .as_ref()
            .unwrap()
            .thesaurus
            .message
            .as_deref(),
        Some("xyzzy is not in the thesaurus.")
    );
    proof(&mut s, "Previous", P::Previous);
    // An inflected word: "sat" finds sit's verbs.
    let sat = s.main_text().find("sat").unwrap();
    caret_at(&mut s, 0, sat + 1);
    proof(&mut s, "Look up the caret word", P::LookUpCaret);
    assert_eq!(
        s.app().proofing.as_ref().unwrap().thesaurus.looked_up,
        "sat"
    );
    let path = s.save_sam("THES.SAM");
    s.reopen(&path);
}

#[test]
fn grammar_check_replaces_skips_and_ends_with_the_readability_statistics() {
    let mut s = spelling_scenario("proofing-grammar");
    s.app_mut().spelling.grammar = Some(super::super::test_grammar());
    s.type_text("This is a apple. The cat sat on the mat.");
    s.enter();
    s.type_text("It were a good day.");
    caret_at(&mut s, 0, 0);
    assert_eq!(s.document().grammar_set(), "");
    s.command_act(CommandId::GrammarCheck);
    // Ami Pro records the set the check uses.
    assert_eq!(s.document().grammar_set(), "Business Writing");
    let check = s.app().proofing.as_ref().unwrap().grammar.as_ref().unwrap();
    assert_eq!(check.state, GrammarState::Checking);
    let found = check.found.as_ref().unwrap();
    let chosen = found
        .problem
        .replacements
        .iter()
        .position(|replacement| replacement == "an")
        .unwrap_or_else(|| s.fail("grammar", &format!("no 'an' among {found:?}")));
    // (A second press on the highlighted alternative replaces at once, as
    // a double-click does; the first alternative is highlighted already.)
    let highlighted = check.chosen;
    s.edit("Choose an and Replace", move |rt| {
        if highlighted != chosen {
            rt.send(Message::Proofing(P::ChooseReplacement(chosen)));
        }
        rt.send(Message::Proofing(P::GrammarReplace));
    });
    assert!(
        s.main_text().starts_with("This is an apple."),
        "{}",
        s.main_text()
    );
    // Next sentence and Skip Rule, then skip to the end.
    for _ in 0..12 {
        let done = s
            .app()
            .proofing
            .as_ref()
            .unwrap()
            .grammar
            .as_ref()
            .is_some_and(|check| check.state == GrammarState::Complete);
        if done {
            break;
        }
        proof(&mut s, "Skip", P::GrammarSkip);
    }
    let check = s.app().proofing.as_ref().unwrap().grammar.as_ref().unwrap();
    assert_eq!(check.state, GrammarState::Complete);
    let figures = check.statistics.expect("the statistics come at the end");
    assert!(figures.words >= 15, "{figures:?}");
    // Tools › Readability Statistics… alone.
    caret_at(&mut s, 0, 0);
    s.command_act(CommandId::ReadabilityStatistics);
    assert_eq!(
        s.app().proofing.as_ref().unwrap().statistics.unwrap().words,
        figures.words
    );
    // Grammar as you type underlines the problem; off, they go.
    proof(&mut s, "grammar as you type on", P::GrammarAsYouType(true));
    proof(
        &mut s,
        "grammar as you type off",
        P::GrammarAsYouType(false),
    );
    assert!(s.app().grammar_marks.is_empty());
    // Typing a new problem in the page, then checking from the start.
    s.caret_to_end();
    s.type_text(" He go to a school.");
    caret_at(&mut s, 0, 0);
    proof(&mut s, "Start the grammar check again", P::StartGrammar);
    let path = s.save_sam("GRAMMAR.SAM");
    s.reopen(&path);
}

#[test]
fn language_is_set_in_one_step_and_a_missing_dictionary_is_named() {
    let mut s = spelling_scenario("proofing-language");
    s.app_mut().spelling.grammar = Some(super::super::test_grammar());
    s.type_text("Teh cat sat.");
    s.command_act(CommandId::Language);
    if !s.app().proofing.as_ref().unwrap().show_options {
        s.fail("Language", "the options are not shown");
    }
    // The language is a fact in the document's settings: a French check
    // names the dictionary to install.
    s.edit("set French", |rt| {
        drop(
            rt.app
                .set_document_language(amipro_document::Language::ami_pro(3)),
        );
    });
    assert_eq!(s.document().language().number, 3);
    proof(&mut s, "Start", P::Start);
    assert_eq!(
        spell_state(&s),
        SpellState::NoDictionary("hunspell-fr-fr".into())
    );
    // No grammar in French.
    proof(&mut s, "Start the grammar check", P::StartGrammar);
    let check = s.app().proofing.as_ref().unwrap().grammar.as_ref().unwrap();
    assert_eq!(check.state, GrammarState::NotEnglish);
    // The language is saved with the document.
    let path = s.save_sam("FRENCH.SAM");
    let s = s.reopen(&path);
    assert_eq!(s.document().language().number, 3);
    // Back to American English in the reopened one: spelling works again.
    let mut s = s;
    s.app_mut().spelling = crate::spelling::tests::test_spelling();
    s.edit("set American English", |rt| {
        drop(
            rt.app
                .set_document_language(amipro_document::Language::ami_pro(1)),
        );
    });
    caret_at(&mut s, 0, 0);
    s.command_act(CommandId::Spelling);
    assert_eq!(
        word(&s).as_deref(),
        Some("Teh"),
        "state {:?}, status {:?}, language {:?}",
        s.app()
            .proofing
            .as_ref()
            .and_then(|p| p.spelling.as_ref().map(|c| c.state.clone())),
        s.app().status,
        s.document().language()
    );
}
