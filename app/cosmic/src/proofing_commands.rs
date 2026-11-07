//! The app's side of proofing (the proofing run; `docs/Design/UI_SPEC.md`
//! §35): Tools › Spell Check… (F7), Language…, and the Proofing panel's
//! actions. Every change to the document is one command and one undo step
//! (SP4); the checks are `amipro_editing::proofing::spelling`'s and the
//! dictionaries `spelling.rs`'s.

use std::sync::Arc;

use amipro_document::Language;
use amipro_editing::find::{StreamKind, streams};
use amipro_editing::proofing::spelling::{
    self, Ignored, ProblemKind, SpellOptions, Speller, check_streams, next_problem,
};
use amipro_editing::{Command, TextPosition};

use crate::proofing_panel::{
    Page, ProofingAction, ProofingPanel, SpellCheck, SpellOption, SpellState, SpellingShown,
};
use crate::spelling::{Dictionary, SessionSpeller};
use crate::{AmiProApp, FileDialogResult, Message, PageSelection};

type Task = cosmic::app::Task<Message>;

/// The pause after a change before the words are checked as you type.
const IDLE: std::time::Duration = std::time::Duration::from_millis(350);

/// `AMIPRO_TRACE_SPELL=FILE`: each step of the as-you-type check written
/// to FILE, for finding where it stops on the desktop.
fn trace_spell(line: impl FnOnce() -> String) {
    use std::io::Write;
    static FILE: std::sync::OnceLock<Option<std::sync::Mutex<std::fs::File>>> =
        std::sync::OnceLock::new();
    let file = FILE.get_or_init(|| {
        let path = std::env::var_os("AMIPRO_TRACE_SPELL")?;
        std::fs::File::create(path).ok().map(std::sync::Mutex::new)
    });
    if let Some(file) = file
        && let Ok(mut file) = file.lock()
    {
        let _ = writeln!(file, "{:?} {}", std::time::SystemTime::now(), line());
    }
}

/// A paragraph text's hash, the as-you-type cache's key.
fn text_hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Ami Pro's names for the stream a check is in ("Checking Main Document
/// Text"; AMIPROUI strings 0x854–0x859).
pub fn stream_name(kind: Option<StreamKind>) -> &'static str {
    match kind {
        Some(StreamKind::Main) | None => "the main text",
        Some(StreamKind::Footnote) => "footnotes",
        Some(StreamKind::PlacedFrame | StreamKind::AnchoredFrame) => "frames",
        Some(StreamKind::RepeatingFrame) => "repeating frames",
        Some(StreamKind::FloatingHeaderFooter | StreamKind::Header | StreamKind::Footer) => {
            "headers and footers"
        }
    }
}

/// Loads a dictionary on a worker thread, then tells the panel.
fn load_task(available: crate::spelling::Available) -> Task {
    cosmic::task::future(async move {
        let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
        std::thread::spawn(move || {
            let _ = done.send(crate::spelling::load(available));
        });
        let (locale, result) = wait
            .await
            .unwrap_or_else(|_| (String::new(), Err("the loader stopped".into())));
        Message::Proofing(ProofingAction::DictionaryLoaded(locale, result))
    })
}

/// Whether `c` belongs to a word, for finding the caret's word.
fn in_word(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '\'' | '\u{2019}' | '-')
}

impl AmiProApp {
    /// The spell check options (Ami Pro's `SpellOpts` in the settings).
    pub(crate) fn spell_options(&self) -> SpellOptions {
        SpellOptions::from_bits(self.preferences.spell_options)
    }

    /// The locale the document is checked in.
    fn proofing_locale(&self) -> Option<String> {
        self.document.language().locale().map(str::to_string)
    }

    /// The dictionary's locale for the document (its user dictionary's key).
    fn dictionary_locale(&self) -> Option<String> {
        let locale = self.proofing_locale()?;
        crate::spelling::dictionary_for(&self.spelling.available, &locale)
            .map(|available| available.locale.clone())
    }

    /// The loaded dictionary for the document's language.
    pub(crate) fn proofing_dictionary(&self) -> Option<Arc<Dictionary>> {
        self.spelling.dictionary(&self.proofing_locale()?)
    }

    /// Runs `check` with the document's speller (its dictionary and the
    /// user's words), if the dictionary is loaded.
    pub(crate) fn with_speller<R>(
        &mut self,
        check: impl FnOnce(&dyn Speller, &amipro_document::Document, &Ignored) -> R,
    ) -> Option<R> {
        let dictionary = self.proofing_dictionary()?;
        let user_locale = dictionary.locale.clone();
        self.spelling.user(&user_locale);
        let options = self.spell_options();
        let user = &self.spelling.user[&user_locale];
        let speller = SessionSpeller {
            dictionary: &dictionary,
            user,
            user_alternatives: options.user_alternatives,
        };
        let session = &self.sessions[self.active_session];
        Some(check(&speller, &session.document, &session.spell_ignored))
    }

    /// The languages the panel offers: every installed dictionary, with
    /// the document's own first if it has none.
    fn proofing_languages(&self) -> Vec<(String, String)> {
        let mut languages: Vec<(String, String)> = self
            .spelling
            .available
            .iter()
            .map(|available| {
                (
                    available.locale.clone(),
                    amipro_document::language::locale_name(&available.locale),
                )
            })
            .collect();
        if let Some(locale) = self.proofing_locale()
            && !self.spelling.has_dictionary(&locale)
        {
            languages.insert(0, (locale.clone(), self.document.language().name()));
        }
        languages
    }

    /// Opens the Proofing panel at `page`; another rail panel closes. With
    /// `start`, Spell Check starts (F7), as Ami Pro's command does.
    pub(crate) fn open_proofing(&mut self, page: Page, start: bool) -> Task {
        let languages = self.proofing_languages();
        let panel = match self.proofing.take() {
            Some(mut panel) => {
                panel.page = page;
                panel.language_names = languages.iter().map(|(_, name)| name.clone()).collect();
                panel.languages = languages;
                panel
            }
            None => ProofingPanel::new(page, languages),
        };
        self.close_rail_panels();
        self.proofing = Some(panel);
        self.open_rail();
        if start && page == Page::Spelling {
            return self.start_spell_check();
        }
        if start && page == Page::Thesaurus {
            return self.look_up_caret();
        }
        if start && page == Page::Grammar {
            return self.start_grammar();
        }
        Task::none()
    }

    /// The grammar set in use: the document's (`[GramStyle]`) when it is
    /// one of Ami Pro's, else Business Writing (Ami Pro's default).
    pub(crate) fn grammar_set_name(&self) -> String {
        let own = self.document.grammar_set();
        if crate::grammar::SETS.contains(&own) {
            own.to_string()
        } else {
            crate::grammar::SETS[0].to_string()
        }
    }

    /// Whether the document is in English, which grammar needs (SP8).
    fn grammar_english(&self) -> bool {
        self.proofing_locale()
            .is_some_and(|locale| locale.starts_with("en"))
    }

    /// Harper's checker, or the task making it.
    fn grammar_ready(&mut self) -> Result<Arc<crate::grammar::Grammar>, Task> {
        if let Some(grammar) = &self.spelling.grammar {
            return Ok(grammar.clone());
        }
        if self.spelling.grammar_loading {
            return Err(Task::none());
        }
        self.spelling.grammar_loading = true;
        Err(cosmic::task::future(async move {
            let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let _ = done.send(crate::grammar::load());
            });
            let grammar = wait.await.unwrap_or_else(|_| crate::grammar::load());
            Message::Proofing(ProofingAction::GrammarLoaded(grammar))
        }))
    }

    /// The grammar problems of a paragraph, cached by its text and the set.
    fn grammar_problems(
        &mut self,
        grammar: &crate::grammar::Grammar,
        text: &str,
        set: &str,
    ) -> Vec<crate::grammar::GrammarProblem> {
        let dialect = crate::grammar::dialect(self.proofing_locale().as_deref().unwrap_or("en_US"));
        let key = (text_hash(text), set.to_string());
        let session = &mut self.sessions[self.active_session];
        if let Some(found) = session.grammar_cache.get(&key) {
            return found.clone();
        }
        let found = grammar.check(text, set, dialect);
        if session.grammar_cache.len() > 20_000 {
            session.grammar_cache.clear();
        }
        session.grammar_cache.insert(key, found.clone());
        found
    }

    /// Tools › Grammar Check…: from the caret, the beginning, or over the
    /// selection, with the document's set recorded as Ami Pro records it
    /// (`[GramStyle]`, one undo step when it changes).
    pub(crate) fn start_grammar(&mut self) -> Task {
        let options = self.preferences.grammar_options;
        let selection = self.page_selection;
        let (from, until, streams) = if !selection.is_caret() {
            let (start, end) = selection.ordered();
            (start, Some(end), vec![start.flow])
        } else {
            let caret = selection.focus;
            let spell = SpellOptions {
                from_beginning: options & 2 != 0,
                other_streams: options & 4 != 0,
                ..SpellOptions::default()
            };
            let streams = check_streams(&self.document, caret.flow, &spell);
            let from = if spell.from_beginning {
                TextPosition {
                    flow: streams.first().copied().unwrap_or(caret.flow),
                    paragraph: 0,
                    byte: 0,
                }
            } else {
                TextPosition { byte: 0, ..caret }
            };
            (from, None, streams)
        };
        let english = self.grammar_english();
        let mut check = crate::proofing_panel::GrammarCheck {
            state: crate::proofing_panel::GrammarState::Loading,
            streams,
            from,
            until,
            began: from,
            found: None,
            chosen: 0,
            skipped: Default::default(),
            statistics: None,
            problems: 0,
            replaced: 0,
        };
        if !english {
            check.state = crate::proofing_panel::GrammarState::NotEnglish;
        }
        if let Some(panel) = self.proofing.as_mut() {
            panel.page = Page::Grammar;
            panel.grammar = Some(check);
        }
        if !english {
            return Task::none();
        }
        let set = self.grammar_set_name();
        if self.document.grammar_set() != set {
            let _ = self.execute_frame_commands(&[Command::SetGrammarSet(set)], false);
        }
        match self.grammar_ready() {
            Ok(_) => self.grammar_next(),
            Err(task) => task,
        }
    }

    fn grammar_check_mut(&mut self) -> Option<&mut crate::proofing_panel::GrammarCheck> {
        self.proofing.as_mut()?.grammar.as_mut()
    }

    /// The next problem from the check's place, selected on the page; or
    /// the end, with Ami Pro's Readability Statistics when they show.
    fn grammar_next(&mut self) -> Task {
        let Some(check) = self
            .proofing
            .as_ref()
            .and_then(|panel| panel.grammar.clone())
        else {
            return Task::none();
        };
        let Some(grammar) = self.spelling.grammar.clone() else {
            return Task::none();
        };
        let set = self.grammar_set_name();
        let mut found = None;
        // From the stream the check is in: the ones before it are done.
        let at = check
            .streams
            .iter()
            .position(|flow| *flow == check.from.flow)
            .unwrap_or(0);
        'streams: for (index, flow) in check.streams.iter().enumerate().skip(at) {
            let index = index - at;
            let count = self
                .document
                .flow(*flow)
                .map_or(0, |flow| flow.paragraphs.len());
            let first = if index == 0 && *flow == check.from.flow {
                check.from.paragraph
            } else {
                0
            };
            for number in first..count {
                if let Some(until) = check.until
                    && until.flow == *flow
                    && number > until.paragraph
                {
                    break 'streams;
                }
                let text = self
                    .document
                    .flow(*flow)
                    .and_then(|flow| flow.paragraphs.get(number))
                    .map(|paragraph| paragraph.text.clone())
                    .unwrap_or_default();
                for problem in self.grammar_problems(&grammar, &text, &set) {
                    if check.skipped.contains(&problem.rule()) {
                        continue;
                    }
                    if index == 0
                        && *flow == check.from.flow
                        && number == check.from.paragraph
                        && problem.range.start < check.from.byte
                    {
                        continue;
                    }
                    if let Some(until) = check.until
                        && until.flow == *flow
                        && number == until.paragraph
                        && problem.range.end > until.byte
                    {
                        break 'streams;
                    }
                    let sentence = amipro_editing::proofing::readability::sentences(&text)
                        .into_iter()
                        .map(|sentence| {
                            let at = sentence.as_ptr() as usize - text.as_ptr() as usize;
                            at..at + sentence.len()
                        })
                        .find(|range| {
                            range.start <= problem.range.start
                                && problem.range.start < range.end.max(range.start + 1)
                        })
                        .unwrap_or(0..text.len());
                    found = Some(crate::proofing_panel::GrammarFound {
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
                        sentence_text: text[sentence.clone()].to_string(),
                        sentence,
                        problem,
                    });
                    break 'streams;
                }
            }
            if check.until.is_some() {
                break;
            }
        }
        let statistics =
            (found.is_none() && self.preferences.grammar_options & 1 != 0).then(|| {
                self.statistics_of(
                    &check.streams,
                    check.until.map(|until| (check.began, until)),
                )
            });
        let Some(state) = self.grammar_check_mut() else {
            return Task::none();
        };
        state.state = crate::proofing_panel::GrammarState::Checking;
        match found {
            Some(found) => {
                state.problems += 1;
                state.chosen = 0;
                let (start, end) = (found.start, found.end);
                state.found = Some(found);
                let page = crate::reference_commands::page_showing(
                    self.pages.layout(),
                    start.flow,
                    start.paragraph,
                    start.byte,
                )
                .unwrap_or(self.page_selection.page);
                self.page_selection = PageSelection {
                    anchor: start,
                    focus: end,
                    page,
                };
                self.sync_editor_to_page();
                self.scroll_to_caret()
            }
            None => {
                state.state = crate::proofing_panel::GrammarState::Complete;
                state.found = None;
                state.statistics = statistics;
                self.status = "Grammar check complete.".into();
                Task::none()
            }
        }
    }

    /// Replace: the chosen option in the problem's place, one undo step;
    /// the sentence is checked again (VM PGR1).
    fn grammar_replace(&mut self) -> Task {
        let Some(check) = self
            .proofing
            .as_ref()
            .and_then(|panel| panel.grammar.clone())
        else {
            return Task::none();
        };
        let Some(found) = check.found else {
            return Task::none();
        };
        let Some(text) = found.problem.replacements.get(check.chosen).cloned() else {
            return Task::none();
        };
        let command = Command::ReplaceText {
            start: found.start,
            end: found.end,
            text,
        };
        if self.execute_frame_commands(&[command], false).is_err() {
            return Task::none();
        }
        if let Some(check) = self.grammar_check_mut() {
            check.replaced += 1;
            check.from = TextPosition {
                byte: found.sentence.start,
                ..found.start
            };
        }
        self.grammar_next()
    }

    /// Readability statistics over `streams` (within a selection when
    /// given): Ami Pro's figures.
    fn statistics_of(
        &self,
        streams: &[amipro_document::FlowId],
        within: Option<(TextPosition, TextPosition)>,
    ) -> amipro_editing::proofing::readability::Statistics {
        let mut figures = amipro_editing::proofing::readability::Statistics::default();
        for flow_id in streams {
            let Some(flow) = self.document.flow(*flow_id) else {
                continue;
            };
            for (index, paragraph) in flow.paragraphs.iter().enumerate() {
                let text = match within {
                    Some((start, end)) if start.flow == *flow_id => {
                        if index < start.paragraph || index > end.paragraph {
                            continue;
                        }
                        let from = if index == start.paragraph {
                            start.byte
                        } else {
                            0
                        };
                        let to = if index == end.paragraph {
                            end.byte
                        } else {
                            paragraph.text.len()
                        };
                        paragraph.text.get(from..to).unwrap_or_default()
                    }
                    Some(_) => continue,
                    None => paragraph.text.as_str(),
                };
                figures.add_paragraph(&text.replace('\u{FFFC}', " "));
            }
        }
        figures
    }

    /// Tools › Readability Statistics…: the document's (or the selection's)
    /// figures in the Grammar page.
    pub(crate) fn show_statistics(&mut self) {
        let selection = self.page_selection;
        let figures = if selection.is_caret() {
            let streams: Vec<_> = streams(&self.document)
                .into_iter()
                .map(|stream| stream.flow)
                .collect();
            self.statistics_of(&streams, None)
        } else {
            let (start, end) = selection.ordered();
            self.statistics_of(&[start.flow], Some((start, end)))
        };
        if let Some(panel) = self.proofing.as_mut() {
            panel.page = Page::Grammar;
            panel.statistics = Some(figures);
        }
    }

    /// The Grammar page's view.
    fn grammar_view(&self) -> cosmic::Element<'_, Message> {
        let panel = self.proofing.as_ref().expect("the Proofing panel shows");
        let set = self.grammar_set_name();
        crate::proofing_panel::grammar_page(
            panel,
            crate::proofing_panel::GrammarShown {
                set: panel.grammar_sets.iter().position(|name| *name == set),
                options: self.preferences.grammar_options,
                as_you_type: self.preferences.grammar_as_you_type,
            },
        )
    }

    /// The document's thesaurus, or the task loading it (SP7).
    fn thesaurus_ready(&mut self) -> Result<Arc<crate::spelling::Thesaurus>, Task> {
        let Some(locale) = self.proofing_locale() else {
            return Err(Task::none());
        };
        if let Some(thesaurus) = self.spelling.thesaurus(&locale) {
            return Ok(thesaurus);
        }
        match self.spelling.begin_thesaurus(&locale) {
            Some((idx, dat)) => Err(cosmic::task::future(async move {
                let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
                let wanted = locale.clone();
                std::thread::spawn(move || {
                    let result =
                        crate::spelling::Thesaurus::load(&wanted, &idx, &dat).map(Arc::new);
                    let _ = done.send(result);
                });
                let result = wait
                    .await
                    .unwrap_or_else(|_| Err("the loader stopped".into()));
                Message::Proofing(ProofingAction::ThesaurusLoaded(locale, result))
            })),
            None => Err(Task::none()),
        }
    }

    /// Thesaurus… (Shift+F7): the word at the caret (the selection's first
    /// word), selected, looked up (Ami Pro, `1380:0000`; VM PSP1).
    pub(crate) fn look_up_caret(&mut self) -> Task {
        let (start, _) = self.page_selection.ordered();
        let place = self
            .document
            .flow(start.flow)
            .and_then(|flow| flow.paragraphs.get(start.paragraph))
            .and_then(|paragraph| {
                amipro_editing::proofing::thesaurus::word_at(&paragraph.text, start.byte)
                    .map(|range| (paragraph.text[range.clone()].to_string(), range))
            });
        let Some((word, range)) = place else {
            if let Some(panel) = self.proofing.as_mut() {
                panel.thesaurus.message = Some("Click on a word to look up and try again.".into());
            }
            return Task::none();
        };
        let begin = TextPosition {
            byte: range.start,
            ..start
        };
        let end = TextPosition {
            byte: range.end,
            ..start
        };
        self.page_selection = PageSelection {
            anchor: begin,
            focus: end,
            page: self.page_selection.page,
        };
        self.sync_editor_to_page();
        if let Some(panel) = self.proofing.as_mut() {
            panel.page = Page::Thesaurus;
            panel.thesaurus = crate::proofing_panel::ThesaurusLookup {
                place: Some((begin, end)),
                replace_with: word.clone(),
                ..Default::default()
            };
        }
        self.look_up(word, false)
    }

    /// Looks `word` up and shows its meanings; with `remember`, the word
    /// shown before goes on Previous's list (Lookup).
    fn look_up(&mut self, word: String, remember: bool) -> Task {
        let thesaurus = match self.thesaurus_ready() {
            Ok(thesaurus) => thesaurus,
            Err(task) => {
                let problem = self.proofing_locale().and_then(|locale| {
                    self.spelling.thesaurus_problem(&locale).map(str::to_string)
                });
                let name = self.document.language().name();
                if let Some(panel) = self.proofing.as_mut() {
                    let lookup = &mut panel.thesaurus;
                    lookup.replace_with = word.clone();
                    match problem {
                        Some(_) => {
                            lookup.loading = false;
                            lookup.message = Some(format!(
                                "No thesaurus is installed for {name}. Install a mythes package for it."
                            ));
                        }
                        None => {
                            lookup.loading = true;
                            lookup.message = None;
                            lookup.looked_up = word;
                        }
                    }
                }
                return task;
            }
        };
        let entry = thesaurus.look_up(&word);
        let Some(panel) = self.proofing.as_mut() else {
            return Task::none();
        };
        let lookup = &mut panel.thesaurus;
        lookup.loading = false;
        match entry {
            Some(entry) => {
                if remember && !lookup.looked_up.is_empty() && lookup.looked_up != word {
                    lookup.history.push(lookup.looked_up.clone());
                }
                lookup.looked_up = word;
                lookup.meaning = 0;
                lookup.chosen = None;
                lookup.replace_with = entry
                    .meanings
                    .first()
                    .and_then(|meaning| meaning.synonyms.first())
                    .cloned()
                    .unwrap_or_default();
                lookup.entry = Some(entry);
                lookup.message = None;
            }
            None => {
                lookup.message = Some(format!("{word} is not in the thesaurus."));
                if lookup.looked_up.is_empty() {
                    lookup.looked_up = word;
                }
            }
        }
        Task::none()
    }

    /// Replace: the looked-up word becomes Replace with, one undo step; the
    /// panel stays, the new word selected (VM PSP1).
    fn thesaurus_replace(&mut self) -> Task {
        let Some(lookup) = self.proofing.as_ref().map(|panel| panel.thesaurus.clone()) else {
            return Task::none();
        };
        let Some((start, end)) = lookup.place else {
            return Task::none();
        };
        let text = lookup.replace_with.trim().to_string();
        if text.is_empty() {
            return Task::none();
        }
        let command = Command::ReplaceText {
            start,
            end,
            text: text.clone(),
        };
        if self.execute_frame_commands(&[command], false).is_err() {
            return Task::none();
        }
        let end = TextPosition {
            byte: start.byte + text.len(),
            ..start
        };
        self.page_selection = PageSelection {
            anchor: start,
            focus: end,
            page: self.page_selection.page,
        };
        self.sync_editor_to_page();
        if let Some(panel) = self.proofing.as_mut() {
            panel.thesaurus.place = Some((start, end));
            panel.thesaurus.looked_up = text;
        }
        Task::none()
    }

    /// Spell Check from the caret, the beginning, or over the selection
    /// (Ami Pro's rules; PSP1, PSP2).
    pub(crate) fn start_spell_check(&mut self) -> Task {
        let options = self.spell_options();
        let selection = self.page_selection;
        let (from, until, streams) = if !selection.is_caret() {
            let (start, end) = selection.ordered();
            (start, Some(end), vec![start.flow])
        } else {
            let caret = selection.focus;
            let streams = check_streams(&self.document, caret.flow, &options);
            let from = if options.from_beginning {
                TextPosition {
                    flow: streams.first().copied().unwrap_or(caret.flow),
                    paragraph: 0,
                    byte: 0,
                }
            } else {
                // From the start of the word the caret is in (1328:01e2).
                let text = self
                    .document
                    .flow(caret.flow)
                    .and_then(|flow| flow.paragraphs.get(caret.paragraph))
                    .map(|paragraph| paragraph.text.as_str())
                    .unwrap_or_default();
                let mut byte = caret.byte.min(text.len());
                while let Some(c) = text[..byte].chars().next_back()
                    && in_word(c)
                {
                    byte -= c.len_utf8();
                }
                TextPosition { byte, ..caret }
            };
            (from, None, streams)
        };
        let mut check = SpellCheck {
            state: SpellState::Loading,
            streams,
            from,
            until,
            problem: None,
            replace_with: String::new(),
            alternatives: Vec::new(),
            chosen: None,
            found: 0,
            replaced: 0,
        };
        let locale = self.proofing_locale();
        let task = match &locale {
            None => {
                check.state = SpellState::NoDictionary(crate::spelling::package_for("en_US"));
                Task::none()
            }
            Some(locale) if !self.spelling.has_dictionary(locale) => {
                check.state = SpellState::NoDictionary(crate::spelling::package_for(locale));
                Task::none()
            }
            Some(locale) => match self.spelling.begin_load(locale) {
                Some(available) => load_task(available),
                None => Task::none(),
            },
        };
        if let Some(panel) = self.proofing.as_mut() {
            panel.page = Page::Spelling;
            panel.spelling = Some(check);
        }
        if self.proofing_dictionary().is_some() {
            return self.spell_next();
        }
        // A dictionary that failed before says why.
        if let Some(error) = locale
            .as_deref()
            .and_then(|locale| crate::spelling::dictionary_for(&self.spelling.available, locale))
            .and_then(|available| {
                self.spelling
                    .pending
                    .get(&available.locale)
                    .cloned()
                    .flatten()
            })
            && let Some(check) = self.spell_check_mut()
        {
            check.state = SpellState::Failed(error);
        }
        task
    }

    fn spell_check_mut(&mut self) -> Option<&mut SpellCheck> {
        self.proofing.as_mut()?.spelling.as_mut()
    }

    /// Finds the next word from the check's place and shows it, selected
    /// on the page, or ends the check (Ami Pro's "Spell check complete.").
    pub(crate) fn spell_next(&mut self) -> Task {
        let Some(check) = self
            .proofing
            .as_ref()
            .and_then(|panel| panel.spelling.clone())
        else {
            return Task::none();
        };
        let options = self.spell_options();
        let found = self.with_speller(|speller, document, ignored| {
            let problem = next_problem(
                document,
                &check.streams,
                check.from,
                check.until,
                speller,
                &options,
                ignored,
            )?;
            let alternatives = match problem.kind {
                ProblemKind::Misspelled => spelling::alternatives(speller, &problem.word),
                ProblemKind::Repeated => Vec::new(),
            };
            Some((problem, alternatives))
        });
        let Some(found) = found else {
            return Task::none();
        };
        let Some(check) = self.spell_check_mut() else {
            return Task::none();
        };
        match found {
            Some((problem, alternatives)) => {
                check.state = SpellState::Checking;
                check.found += 1;
                // A repeat's Replace with starts empty: Delete Repeat.
                check.replace_with = match problem.kind {
                    ProblemKind::Misspelled => problem.word.clone(),
                    ProblemKind::Repeated => String::new(),
                };
                check.alternatives = alternatives;
                check.chosen = None;
                let (start, end) = (problem.start, problem.end);
                check.problem = Some(problem);
                let page = crate::reference_commands::page_showing(
                    self.pages.layout(),
                    start.flow,
                    start.paragraph,
                    start.byte,
                )
                .unwrap_or(self.page_selection.page);
                self.page_selection = PageSelection {
                    anchor: start,
                    focus: end,
                    page,
                };
                self.sync_editor_to_page();
                self.scroll_to_caret()
            }
            None => {
                check.state = SpellState::Complete;
                check.problem = None;
                self.status = "Spell check complete.".into();
                Task::none()
            }
        }
    }

    /// Replace: the word becomes Replace with, one undo step; the check
    /// goes on from the replacement, which is checked in its turn (Ami
    /// Pro, `1320:0b12`).
    fn spell_replace(&mut self) -> Task {
        let Some(check) = self
            .proofing
            .as_ref()
            .and_then(|panel| panel.spelling.clone())
        else {
            return Task::none();
        };
        let Some(problem) = check.problem else {
            return Task::none();
        };
        let command = Command::ReplaceText {
            start: problem.start,
            end: problem.end,
            text: check.replace_with.clone(),
        };
        if self.execute_frame_commands(&[command], false).is_err() {
            return Task::none();
        }
        let length = check.replace_with.len();
        if let Some(check) = self.spell_check_mut() {
            check.replaced += 1;
            check.from = problem.start;
            if let Some(until) = check.until.as_mut()
                && until.flow == problem.start.flow
                && until.paragraph == problem.start.paragraph
            {
                until.byte =
                    (until.byte + length).saturating_sub(problem.end.byte - problem.start.byte);
            }
        }
        self.spell_next()
    }

    /// Replace All (SP4): every occurrence in the check's streams (or its
    /// selection) at once, each in its capitals, one undo step.
    fn spell_replace_all(&mut self) -> Task {
        let Some(check) = self
            .proofing
            .as_ref()
            .and_then(|panel| panel.spelling.clone())
        else {
            return Task::none();
        };
        let Some(problem) = check.problem else {
            return Task::none();
        };
        let earlier = |a: TextPosition, b: TextPosition| {
            if (a.paragraph, a.byte) <= (b.paragraph, b.byte) {
                a
            } else {
                b
            }
        };
        let within = check
            .until
            .map(|until| (earlier(check.from, problem.start), until));
        let Some((command, count)) = spelling::replace_all(
            &self.document,
            &check.streams,
            within,
            &problem.word,
            &check.replace_with,
        ) else {
            return self.spell_replace();
        };
        if self.execute_frame_commands(&[command], false).is_err() {
            return Task::none();
        }
        self.status = format!(
            "Replaced {count} {}",
            if count == 1 { "word" } else { "words" }
        );
        if let Some(check) = self.spell_check_mut() {
            check.replaced += count;
            check.from = problem.start;
        }
        self.spell_next()
    }

    /// Skip, Skip All, Add to Dictionary: on past the word.
    fn spell_past(&mut self) -> Task {
        if let Some(check) = self.spell_check_mut()
            && let Some(problem) = check.problem.as_ref()
        {
            check.from = problem.end;
        }
        self.spell_next()
    }

    /// The user dictionary of the document's language, if it has one.
    fn user_dictionary(&mut self) -> Option<&mut crate::spelling::UserDictionary> {
        let locale = self.dictionary_locale()?;
        Some(self.spelling.user(&locale))
    }

    pub(crate) fn update_proofing(&mut self, action: ProofingAction) -> Task {
        match action {
            ProofingAction::Close => {
                self.proofing = None;
            }
            ProofingAction::Page(page) => {
                if let Some(panel) = self.proofing.as_mut() {
                    panel.page = page;
                }
            }
            ProofingAction::Start => return self.start_spell_check(),
            ProofingAction::ReplaceWith(text) => {
                if let Some(check) = self.spell_check_mut() {
                    check.replace_with = text;
                    check.chosen = None;
                }
            }
            ProofingAction::Choose(index) => {
                let again = self
                    .proofing
                    .as_ref()
                    .and_then(|panel| panel.spelling.as_ref())
                    .is_some_and(|check| check.chosen == Some(index));
                if again {
                    return self.spell_replace();
                }
                if let Some(check) = self.spell_check_mut()
                    && let Some(alternative) = check.alternatives.get(index).cloned()
                {
                    check.replace_with = alternative;
                    check.chosen = Some(index);
                }
            }
            ProofingAction::Replace => return self.spell_replace(),
            ProofingAction::ReplaceAll => return self.spell_replace_all(),
            ProofingAction::Skip => return self.spell_past(),
            ProofingAction::SkipAll => {
                let word = self
                    .proofing
                    .as_ref()
                    .and_then(|panel| panel.spelling.as_ref())
                    .and_then(|check| check.problem.as_ref())
                    .map(|problem| problem.word.clone());
                if let Some(word) = word {
                    self.spell_ignored.add(&word);
                }
                let task = self.spell_past();
                return Task::batch([task, self.check_as_you_type()]);
            }
            ProofingAction::AddToDictionary => {
                let word = self
                    .proofing
                    .as_ref()
                    .and_then(|panel| panel.spelling.as_ref())
                    .and_then(|check| check.problem.as_ref())
                    .map(|problem| problem.word.clone());
                if let Some(word) = word {
                    self.add_to_user_dictionary(word);
                }
                let task = self.spell_past();
                return Task::batch([task, self.check_as_you_type()]);
            }
            ProofingAction::SpellOption(which, on) => {
                let mut options = self.spell_options();
                match which {
                    SpellOption::FromBeginning => options.from_beginning = on,
                    SpellOption::OtherStreams => options.other_streams = on,
                    SpellOption::RepeatedWords => options.repeated_words = on,
                    SpellOption::WordsWithNumbers => options.words_with_numbers = on,
                    SpellOption::InitialCaps => options.initial_caps = on,
                    SpellOption::UserAlternatives => options.user_alternatives = on,
                }
                self.preferences.spell_options = options.bits();
                self.save_preferences();
                return self.check_as_you_type();
            }
            ProofingAction::ToggleOptions => {
                if let Some(panel) = self.proofing.as_mut() {
                    panel.show_options = !panel.show_options;
                }
            }
            ProofingAction::ToggleDictionary => {
                if let Some(panel) = self.proofing.as_mut() {
                    panel.show_dictionary = !panel.show_dictionary;
                }
            }
            ProofingAction::Language(index) => {
                let Some(locale) = self
                    .proofing
                    .as_ref()
                    .and_then(|panel| panel.languages.get(index))
                    .map(|(locale, _)| locale.clone())
                else {
                    return Task::none();
                };
                return self.set_document_language(Language::from_locale(&locale));
            }
            ProofingAction::DefaultLanguage => {
                let language = self.document.language().clone();
                self.preferences.default_language = match language.number {
                    amipro_document::language::AMERICAN if language.is_ami_pros() => String::new(),
                    _ => language.locale().unwrap_or_default().to_string(),
                };
                self.save_preferences();
                self.status = format!("New documents will be in {}", language.name());
            }
            ProofingAction::UserWord(text) => {
                if let Some(panel) = self.proofing.as_mut() {
                    panel.user_word = text;
                }
            }
            ProofingAction::AddUserWord => {
                let word = self
                    .proofing
                    .as_mut()
                    .map(|panel| std::mem::take(&mut panel.user_word))
                    .unwrap_or_default();
                for word in word.split_whitespace() {
                    self.add_to_user_dictionary(word.to_string());
                }
                return self.check_as_you_type();
            }
            ProofingAction::RemoveUserWord(word) => {
                if let Some(user) = self.user_dictionary()
                    && let Err(error) = user.remove(&word)
                {
                    self.status = format!("The user dictionary could not be saved: {error}");
                }
                return self.check_as_you_type();
            }
            ProofingAction::ImportDictionary => return self.choose_dictionary_file(true),
            ProofingAction::ExportDictionary => return self.choose_dictionary_file(false),
            ProofingAction::DictionaryFile { import, result } => {
                if let FileDialogResult::Selected(path) = result {
                    self.dictionary_file(import, &path);
                    return self.check_as_you_type();
                }
            }
            ProofingAction::Context(entry) => return self.proofing_context_entry(entry),
            ProofingAction::LookUpCaret => return self.look_up_caret(),
            ProofingAction::StartGrammar => return self.start_grammar(),
            ProofingAction::GrammarSet(index) => {
                let Some(name) = crate::grammar::SETS.get(index) else {
                    return Task::none();
                };
                if self.document.grammar_set() != *name {
                    let _ = self
                        .execute_frame_commands(&[Command::SetGrammarSet(name.to_string())], false);
                }
                return self.check_as_you_type();
            }
            ProofingAction::GrammarOption(which, on) => {
                let bit = which.bit();
                if on {
                    self.preferences.grammar_options |= bit;
                } else {
                    self.preferences.grammar_options &= !bit;
                }
                self.save_preferences();
            }
            ProofingAction::GrammarAsYouType(on) => {
                self.preferences.grammar_as_you_type = on;
                self.save_preferences();
                return self.check_as_you_type();
            }
            ProofingAction::ChooseReplacement(index) => {
                let again = self
                    .proofing
                    .as_ref()
                    .and_then(|panel| panel.grammar.as_ref())
                    .is_some_and(|check| check.chosen == index);
                if let Some(check) = self.grammar_check_mut() {
                    check.chosen = index;
                }
                if again {
                    return self.grammar_replace();
                }
            }
            ProofingAction::GrammarReplace => return self.grammar_replace(),
            ProofingAction::GrammarSkip => {
                if let Some(check) = self.grammar_check_mut()
                    && let Some(found) = check.found.as_ref()
                {
                    check.from = found.end;
                }
                return self.grammar_next();
            }
            ProofingAction::SkipRule => {
                if let Some(check) = self.grammar_check_mut()
                    && let Some(found) = check.found.clone()
                {
                    check.skipped.insert(found.problem.rule());
                    check.from = found.start;
                }
                return self.grammar_next();
            }
            ProofingAction::NextSentence => {
                if let Some(check) = self.grammar_check_mut()
                    && let Some(found) = check.found.as_ref()
                {
                    check.from = TextPosition {
                        byte: found.sentence.end,
                        ..found.start
                    };
                }
                return self.grammar_next();
            }
            ProofingAction::Statistics => self.show_statistics(),
            ProofingAction::GrammarChecked {
                session,
                generation,
                found,
            } => {
                let Some(target) = self.sessions.iter_mut().find(|own| own.id == session) else {
                    return Task::none();
                };
                for (key, problems) in found.iter() {
                    target.grammar_cache.insert(key.clone(), problems.clone());
                }
                if self.id == session {
                    self.show_grammar_marks();
                    // A change since: its pause checks what it added.
                    if self.spell_generation != generation {
                        return Task::none();
                    }
                }
            }
            ProofingAction::GrammarLoaded(grammar) => {
                self.spelling.grammar = Some(grammar);
                self.spelling.grammar_loading = false;
                let waiting = self
                    .proofing
                    .as_ref()
                    .and_then(|panel| panel.grammar.as_ref())
                    .is_some_and(|check| {
                        check.state == crate::proofing_panel::GrammarState::Loading
                    });
                let typing = self.check_as_you_type();
                if waiting {
                    return Task::batch([self.grammar_next(), typing]);
                }
                return typing;
            }
            ProofingAction::Meaning(index) => {
                if let Some(panel) = self.proofing.as_mut() {
                    let lookup = &mut panel.thesaurus;
                    lookup.meaning = index;
                    lookup.chosen = None;
                    if let Some(first) = lookup
                        .entry
                        .as_ref()
                        .and_then(|entry| entry.meanings.get(index))
                        .and_then(|meaning| meaning.synonyms.first())
                    {
                        lookup.replace_with = first.clone();
                    }
                }
            }
            ProofingAction::Synonym(index) => {
                let again = self
                    .proofing
                    .as_ref()
                    .is_some_and(|panel| panel.thesaurus.chosen == Some(index));
                if again {
                    return self.thesaurus_replace();
                }
                if let Some(panel) = self.proofing.as_mut() {
                    let lookup = &mut panel.thesaurus;
                    if let Some(synonym) = lookup
                        .entry
                        .as_ref()
                        .and_then(|entry| entry.meanings.get(lookup.meaning))
                        .and_then(|meaning| meaning.synonyms.get(index))
                    {
                        lookup.replace_with = synonym.clone();
                        lookup.chosen = Some(index);
                    }
                }
            }
            ProofingAction::ThesaurusReplaceWith(text) => {
                if let Some(panel) = self.proofing.as_mut() {
                    panel.thesaurus.replace_with = text;
                    panel.thesaurus.chosen = None;
                }
            }
            ProofingAction::LookUp => {
                let word = self
                    .proofing
                    .as_ref()
                    .map(|panel| panel.thesaurus.replace_with.trim().to_string())
                    .unwrap_or_default();
                if !word.is_empty() {
                    return self.look_up(word, true);
                }
            }
            ProofingAction::Previous => {
                let previous = self
                    .proofing
                    .as_mut()
                    .and_then(|panel| panel.thesaurus.history.pop());
                if let Some(word) = previous {
                    return self.look_up(word, false);
                }
            }
            ProofingAction::ThesaurusReplace => return self.thesaurus_replace(),
            ProofingAction::ThesaurusLoaded(locale, result) => {
                match result {
                    Ok(thesaurus) => {
                        self.spelling.thesauri_pending.remove(&locale);
                        self.spelling.thesauri.insert(locale, thesaurus);
                    }
                    Err(error) => {
                        self.spelling.thesauri_pending.insert(locale, Some(error));
                    }
                }
                let waiting = self
                    .proofing
                    .as_ref()
                    .filter(|panel| panel.thesaurus.loading)
                    .map(|panel| panel.thesaurus.looked_up.clone());
                if let Some(word) = waiting {
                    return self.look_up(word, false);
                }
            }
            ProofingAction::Idle {
                session,
                generation,
            } => return self.spelling_idle(session, generation),
            ProofingAction::DictionaryLoaded(locale, result) => {
                trace_spell(|| format!("loaded {locale}: {:?}", result.as_ref().err()));
                let failed = result.as_ref().err().cloned();
                self.spelling.loaded(locale, result);
                let loading = self
                    .proofing
                    .as_ref()
                    .and_then(|panel| panel.spelling.as_ref())
                    .is_some_and(|check| check.state == SpellState::Loading);
                let typing = self.check_as_you_type();
                if loading {
                    if let Some(error) = failed {
                        if let Some(check) = self.spell_check_mut() {
                            check.state = SpellState::Failed(error);
                        }
                        return typing;
                    }
                    return Task::batch([self.spell_next(), typing]);
                }
                return typing;
            }
        }
        Task::none()
    }

    /// Adds `word` to the user dictionary of the document's language.
    pub(crate) fn add_to_user_dictionary(&mut self, word: String) {
        let Some(user) = self.user_dictionary() else {
            self.status = "No dictionary for the document's language".into();
            return;
        };
        match user.add([word.clone()]) {
            Ok(true) => self.status = format!("“{word}” added to your dictionary"),
            Ok(false) => {}
            Err(error) => {
                self.status = format!("The user dictionary could not be saved: {error}");
            }
        }
    }

    /// Tools › Language… and the panel's Language: the document's language,
    /// one undo step (SP6); a check under way starts again in it.
    pub(crate) fn set_document_language(&mut self, language: Language) -> Task {
        if *self.document.language() == language {
            return Task::none();
        }
        let name = language.name();
        if self
            .execute_frame_commands(&[Command::SetLanguage(language)], false)
            .is_err()
        {
            return Task::none();
        }
        self.status = format!("Language: {name}");
        let languages = self.proofing_languages();
        let restart = match self.proofing.as_mut() {
            Some(panel) => {
                panel.language_names = languages.iter().map(|(_, name)| name.clone()).collect();
                panel.languages = languages;
                panel.spelling.is_some()
            }
            None => false,
        };
        let typing = self.check_as_you_type();
        if restart {
            return Task::batch([self.start_spell_check(), typing]);
        }
        typing
    }

    /// Import… and Export… of the user dictionary (Ami Pro's format).
    fn choose_dictionary_file(&self, import: bool) -> Task {
        use cosmic::dialog::file_chooser::{self, FileFilter};
        let name = self.dictionary_locale().map_or_else(
            || "Dictionary.dic".to_string(),
            |locale| format!("{locale}.dic"),
        );
        cosmic::task::future(async move {
            let filter = FileFilter::new("Word lists (.dic, .txt)")
                .glob("*.dic")
                .glob("*.DIC")
                .glob("*.txt")
                .glob("*.TXT");
            let result = if import {
                match file_chooser::open::Dialog::new()
                    .title("Import User Dictionary".to_owned())
                    .filter(filter)
                    .open_file()
                    .await
                {
                    Ok(response) => response.url().to_file_path().map_or_else(
                        |()| FileDialogResult::Failed("not a local file".into()),
                        FileDialogResult::Selected,
                    ),
                    Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                    Err(error) => FileDialogResult::Failed(error.to_string()),
                }
            } else {
                match file_chooser::save::Dialog::new()
                    .title("Export User Dictionary".to_owned())
                    .file_name(name)
                    .filter(filter)
                    .save_file()
                    .await
                {
                    Ok(response) => response
                        .url()
                        .and_then(|url| url.to_file_path().ok())
                        .map_or_else(
                            || FileDialogResult::Failed("not a local file".into()),
                            FileDialogResult::Selected,
                        ),
                    Err(file_chooser::Error::Cancelled) => FileDialogResult::Cancelled,
                    Err(error) => FileDialogResult::Failed(error.to_string()),
                }
            };
            Message::Proofing(ProofingAction::DictionaryFile { import, result })
        })
    }

    /// Reads a word list into the user dictionary, or writes it out.
    pub(crate) fn dictionary_file(&mut self, import: bool, path: &std::path::Path) {
        let Some(user) = self.user_dictionary() else {
            self.status = "No dictionary for the document's language".into();
            return;
        };
        if import {
            match std::fs::read(path) {
                Ok(bytes) => {
                    let words = spelling::read_word_list(&bytes);
                    let before = user.words.len();
                    match user.add(words) {
                        Ok(_) => {
                            let added = user.words.len() - before;
                            self.status = format!(
                                "{added} {} imported",
                                if added == 1 { "word" } else { "words" }
                            );
                        }
                        Err(error) => {
                            self.status =
                                format!("The user dictionary could not be saved: {error}");
                        }
                    }
                }
                Err(error) => self.status = format!("Could not read the word list: {error}"),
            }
        } else {
            let text = user.text();
            self.status = match std::fs::write(path, text) {
                Ok(()) => "User dictionary exported".into(),
                Err(error) => format!("Could not write the word list: {error}"),
            };
        }
    }

    /// After every message: when the document changed, the as-you-type
    /// check waits for a pause (SP2), off the typing path.
    pub(crate) fn sync_spelling(&mut self) -> Option<Task> {
        // Without any dictionary there is nothing to underline (and the
        // tests' window runs every task to its end: no pause per message).
        if !self.page_surface || self.spelling.available.is_empty() {
            trace_spell(|| {
                format!(
                    "sync: off (page surface {}, {} dictionaries)",
                    self.page_surface,
                    self.spelling.available.len()
                )
            });
            return None;
        }
        if !self.preferences.spell_as_you_type && !self.preferences.grammar_as_you_type {
            if !self.spell_marks.is_empty() || !self.grammar_marks.is_empty() {
                self.spell_marks.clear();
                self.grammar_marks.clear();
            }
            self.spell_root = None;
            return None;
        }
        if self.spell_root.as_ref() == Some(&self.document) {
            return None;
        }
        self.spell_root = Some(self.document.clone());
        self.spell_generation += 1;
        let (session, generation) = (self.id, self.spell_generation);
        trace_spell(|| format!("sync: scheduled session {session} generation {generation}"));
        Some(cosmic::task::future(async move {
            let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                std::thread::sleep(IDLE);
                let _ = done.send(());
            });
            let _ = wait.await;
            Message::Proofing(ProofingAction::Idle {
                session,
                generation,
            })
        }))
    }

    /// The idle check came: underline the words if nothing changed since.
    fn spelling_idle(&mut self, session: u64, generation: u64) -> Task {
        trace_spell(|| {
            format!(
                "idle: session {session} generation {generation}; now session {} generation {}",
                self.id, self.spell_generation
            )
        });
        if self.id != session {
            // Another tab: checked again when it is active.
            if let Some(other) = self.sessions.iter_mut().find(|other| other.id == session) {
                other.spell_root = None;
            }
            return Task::none();
        }
        if self.spell_generation != generation {
            // A newer change: its own pause decides. (Clearing the tab's
            // checked root here scheduled a new pause after every stale
            // one, so none was ever current on the desktop.)
            return Task::none();
        }
        self.underline_misspelled()
    }

    /// Checks every paragraph's words now, as after a change of options,
    /// dictionary, or language (SP2).
    pub(crate) fn check_as_you_type(&mut self) -> Task {
        self.spell_cache.clear();
        self.underline_misspelled()
    }

    /// The words to underline in every stream, each paragraph's text
    /// checked once (cached by its text).
    fn underline_misspelled(&mut self) -> Task {
        if !self.preferences.spell_as_you_type {
            self.spell_marks.clear();
            return self.underline_grammar();
        }
        if self.proofing_dictionary().is_none() {
            self.spell_marks.clear();
            trace_spell(|| {
                format!(
                    "underline: no dictionary loaded for {:?}",
                    self.proofing_locale()
                )
            });
            return match self.proofing_locale() {
                Some(locale) => {
                    let load = self.spelling.begin_load(&locale);
                    trace_spell(|| match &load {
                        Some(available) => format!("underline: loading {:?}", available.source),
                        None => format!("underline: {locale} loading already or unavailable"),
                    });
                    load.map_or_else(Task::none, load_task)
                }
                None => Task::none(),
            };
        }
        let options = self.spell_options();
        let mut cache = std::mem::take(&mut self.sessions[self.active_session].spell_cache);
        let marks = self
            .with_speller(|speller, document, ignored| {
                let mut marks = crate::page_surface::Highlights::new();
                for stream in streams(document) {
                    let Some(flow) = document.flow(stream.flow) else {
                        continue;
                    };
                    for (index, paragraph) in flow.paragraphs.iter().enumerate() {
                        if paragraph.text.is_empty() {
                            continue;
                        }
                        let key = text_hash(&paragraph.text);
                        let ranges = cache.entry(key).or_insert_with(|| {
                            spelling::paragraph_problems(
                                &paragraph.text,
                                speller,
                                &options,
                                ignored,
                            )
                            .into_iter()
                            .map(|problem| problem.range)
                            .collect()
                        });
                        if !ranges.is_empty() {
                            marks.insert((stream.flow, index), ranges.clone());
                        }
                    }
                }
                marks
            })
            .unwrap_or_default();
        // A cache that grew past a long document's worth starts again.
        if cache.len() > 20_000 {
            cache.clear();
        }
        let session = &mut self.sessions[self.active_session];
        session.spell_cache = cache;
        trace_spell(|| {
            format!(
                "underline: {} paragraphs marked, {} words",
                marks.len(),
                marks.values().map(Vec::len).sum::<usize>()
            )
        });
        session.spell_marks = marks;
        self.underline_grammar()
    }

    /// Grammar underlines as you type (SP8, off by default): each
    /// paragraph's problems in the document's set, in blue. Paragraphs not
    /// yet checked are checked on a worker thread (Harper takes about 10 ms
    /// a paragraph); the underlines follow when it is done.
    fn underline_grammar(&mut self) -> Task {
        if !self.preferences.grammar_as_you_type || !self.grammar_english() {
            self.grammar_marks.clear();
            return Task::none();
        }
        let grammar = match self.grammar_ready() {
            Ok(grammar) => grammar,
            Err(task) => return task,
        };
        let set = self.grammar_set_name();
        let mut missing: Vec<((u64, String), String)> = Vec::new();
        for stream in streams(&self.document) {
            if let Some(flow) = self.document.flow(stream.flow) {
                for paragraph in &flow.paragraphs {
                    let key = (text_hash(&paragraph.text), set.clone());
                    if !paragraph.text.trim().is_empty()
                        && !self.grammar_cache.contains_key(&key)
                        && !missing.iter().any(|(own, _)| *own == key)
                    {
                        missing.push((key, paragraph.text.clone()));
                    }
                }
            }
        }
        self.show_grammar_marks();
        if missing.is_empty() {
            return Task::none();
        }
        let dialect = crate::grammar::dialect(self.proofing_locale().as_deref().unwrap_or("en_US"));
        let (session, generation) = (self.id, self.spell_generation);
        cosmic::task::future(async move {
            let (done, wait) = cosmic::iced::futures::channel::oneshot::channel();
            std::thread::spawn(move || {
                let found: Vec<_> = missing
                    .into_iter()
                    .map(|(key, text)| {
                        let problems = grammar.check(&text, &key.1, dialect);
                        (key, problems)
                    })
                    .collect();
                let _ = done.send(found);
            });
            let found = wait.await.unwrap_or_default();
            Message::Proofing(ProofingAction::GrammarChecked {
                session,
                generation,
                found: Arc::new(found),
            })
        })
    }

    /// The grammar underlines from the cache: the paragraphs checked so far.
    fn show_grammar_marks(&mut self) {
        let set = self.grammar_set_name();
        let mut marks = crate::page_surface::Highlights::new();
        for stream in streams(&self.document) {
            let Some(flow) = self.document.flow(stream.flow) else {
                continue;
            };
            for (index, paragraph) in flow.paragraphs.iter().enumerate() {
                if let Some(problems) = self
                    .grammar_cache
                    .get(&(text_hash(&paragraph.text), set.clone()))
                    .filter(|problems| !problems.is_empty())
                {
                    marks.insert(
                        (stream.flow, index),
                        problems
                            .iter()
                            .map(|problem| problem.range.clone())
                            .collect(),
                    );
                }
            }
        }
        let session = &mut self.sessions[self.active_session];
        session.grammar_marks = marks;
    }

    /// At a right press: the word under the caret, with its suggestions
    /// when it is underlined (SP2) and its synonyms (SP7), for the
    /// right-click menu. A thesaurus not loaded yet starts loading.
    pub(crate) fn proofing_context(&mut self) -> Task {
        self.context_word = None;
        let caret = self.page_selection.focus;
        let Some(text) = self
            .document
            .flow(caret.flow)
            .and_then(|flow| flow.paragraphs.get(caret.paragraph))
            .map(|paragraph| paragraph.text.clone())
        else {
            return Task::none();
        };
        let flagged = self
            .spell_marks
            .get(&(caret.flow, caret.paragraph))
            .and_then(|ranges| {
                ranges
                    .iter()
                    .find(|range| range.start <= caret.byte && caret.byte <= range.end)
            })
            .cloned();
        let range = match flagged.clone() {
            Some(range) => range,
            None => match amipro_editing::proofing::thesaurus::word_at(&text, caret.byte) {
                // Only a word the caret is in or touches.
                Some(range) if range.start <= caret.byte && caret.byte <= range.end => range,
                _ => return Task::none(),
            },
        };
        let Some(slice) = text.get(range.clone()) else {
            return Task::none();
        };
        let repeated = flagged.is_some() && slice.starts_with(char::is_whitespace);
        let word = slice.trim_start().to_string();
        let suggestions = if flagged.is_none() || repeated {
            Vec::new()
        } else {
            self.with_speller(|speller, _, _| {
                let mut found = spelling::alternatives(speller, &word);
                found.truncate(5);
                found
            })
            .unwrap_or_default()
        };
        let (synonyms, task) = if repeated {
            (Vec::new(), Task::none())
        } else {
            match self.thesaurus_ready() {
                Ok(thesaurus) => (
                    thesaurus
                        .look_up(&word)
                        .and_then(|entry| entry.meanings.into_iter().next())
                        .map(|meaning| meaning.synonyms.into_iter().take(8).collect())
                        .unwrap_or_default(),
                    Task::none(),
                ),
                Err(task) => (Vec::new(), task),
            }
        };
        self.context_word = Some(crate::proofing_panel::ContextWord {
            start: TextPosition {
                byte: range.start,
                ..caret
            },
            end: TextPosition {
                byte: range.end,
                ..caret
            },
            word,
            flagged: flagged.is_some(),
            repeated,
            suggestions,
            synonyms,
        });
        task
    }

    /// The right-click menu's entries for the word under the pointer.
    pub(crate) fn proofing_context_entries(&self) -> Vec<crate::MenuEntry> {
        use crate::proofing_panel::ContextEntry;
        use crate::{MenuCommand, MenuEntry};
        let Some(word) = self.context_word.as_ref() else {
            return Vec::new();
        };
        let item = |label: String, entry: ContextEntry| MenuEntry::Item {
            label,
            command: MenuCommand::Proofing(entry),
            enabled: true,
            checked: None,
        };
        let mut entries = Vec::new();
        let synonyms = (!word.synonyms.is_empty()).then(|| MenuEntry::Folder {
            label: "Synonyms".into(),
            indented: false,
            entries: word
                .synonyms
                .iter()
                .enumerate()
                .map(|(index, synonym)| item(synonym.clone(), ContextEntry::Synonym(index as u8)))
                .collect(),
        });
        if !word.flagged {
            if let Some(synonyms) = synonyms {
                entries.push(synonyms);
                entries.push(MenuEntry::Divider);
            }
            return entries;
        }
        if word.repeated {
            entries.push(item(
                "Delete Repeated Word".into(),
                ContextEntry::DeleteRepeat,
            ));
        } else if word.suggestions.is_empty() {
            entries.push(MenuEntry::Item {
                label: "No suggestions".into(),
                command: MenuCommand::Command(crate::CommandId::Spelling),
                enabled: false,
                checked: None,
            });
        }
        for (index, suggestion) in word.suggestions.iter().enumerate() {
            entries.push(item(
                suggestion.clone(),
                ContextEntry::Suggestion(index as u8),
            ));
        }
        entries.push(MenuEntry::Divider);
        entries.push(item("Ignore All".into(), ContextEntry::IgnoreAll));
        if !word.repeated {
            entries.push(item(
                "Add to Dictionary".into(),
                ContextEntry::AddToDictionary,
            ));
        }
        entries.push(MenuEntry::Item {
            label: "Spell Check…".into(),
            command: MenuCommand::Command(crate::CommandId::Spelling),
            enabled: true,
            checked: None,
        });
        if let Some(synonyms) = synonyms {
            entries.push(synonyms);
        }
        entries.push(MenuEntry::Divider);
        entries
    }

    /// A right-click entry chosen: one undo step for a replacement.
    fn proofing_context_entry(&mut self, entry: crate::proofing_panel::ContextEntry) -> Task {
        use crate::proofing_panel::ContextEntry;
        let Some(word) = self.context_word.take() else {
            return Task::none();
        };
        let replacement = match entry {
            ContextEntry::Suggestion(index) => word.suggestions.get(usize::from(index)).cloned(),
            ContextEntry::Synonym(index) => word.synonyms.get(usize::from(index)).cloned(),
            ContextEntry::DeleteRepeat => Some(String::new()),
            ContextEntry::IgnoreAll => {
                self.spell_ignored.add(&word.word);
                return self.check_as_you_type();
            }
            ContextEntry::AddToDictionary => {
                self.add_to_user_dictionary(word.word.clone());
                return self.check_as_you_type();
            }
        };
        if let Some(text) = replacement {
            let command = Command::ReplaceText {
                start: word.start,
                end: word.end,
                text: text.clone(),
            };
            if self.execute_frame_commands(&[command], false).is_ok() {
                let caret = TextPosition {
                    byte: word.start.byte + text.len(),
                    ..word.start
                };
                self.page_selection = PageSelection::caret(caret, self.page_selection.page);
                self.sync_editor_to_page();
            }
        }
        Task::none()
    }

    pub(crate) fn proofing_view(&self) -> cosmic::Element<'_, Message> {
        let panel = self.proofing.as_ref().expect("the Proofing panel shows");
        let stream = panel
            .spelling
            .as_ref()
            .and_then(|check| check.problem.as_ref())
            .and_then(|problem| {
                streams(&self.document)
                    .into_iter()
                    .find(|stream| stream.flow == problem.start.flow)
                    .map(|stream| stream.kind)
            });
        let locale = self.proofing_locale();
        let language = locale.as_ref().and_then(|locale| {
            panel
                .languages
                .iter()
                .position(|(own, _)| own == locale)
                .or_else(|| {
                    crate::spelling::dictionary_for(&self.spelling.available, locale).and_then(
                        |available| {
                            panel
                                .languages
                                .iter()
                                .position(|(own, _)| *own == available.locale)
                        },
                    )
                })
        });
        let user_words: Vec<&str> = self
            .dictionary_locale()
            .and_then(|locale| self.spelling.user.get(&locale))
            .map(|user| user.words.iter().map(String::as_str).collect())
            .unwrap_or_default();
        if panel.page == Page::Thesaurus {
            return crate::proofing_panel::thesaurus_page(panel);
        }
        if panel.page == Page::Grammar {
            return self.grammar_view();
        }
        crate::proofing_panel::spelling_page(
            panel,
            SpellingShown {
                options: self.spell_options(),
                stream: stream_name(stream),
                language,
                language_name: self.document.language().name(),
                is_default_language: {
                    let default = if self.preferences.default_language.is_empty() {
                        Language::default()
                    } else {
                        Language::from_locale(&self.preferences.default_language)
                    };
                    default.locale() == self.document.language().locale()
                },
                user_words,
            },
        )
    }
}
