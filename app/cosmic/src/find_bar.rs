//! The find bar (`UI_SPEC.md` §27; decisions Find F1–F4): Find and Replace
//! fields over the pages, the matches in the streams searched with the
//! current one, Ami Pro's options in a menu, and recent searches. The
//! search itself is `amipro_editing::find`.

use std::path::PathBuf;

use amipro_document::{Document, FlowId};
use amipro_editing::TextPosition;
use amipro_editing::find::{self, Attributes, FindOptions, Match};
use amipro_layout::DocumentLayout;

use crate::page_editing::PageSelection;

/// The fields' ids, for focusing them.
pub const FIND_FIELD: &str = "find-field";
pub const REPLACE_FIELD: &str = "replace-field";
/// How many recent searches are kept (F4).
pub const RECENT_LIMIT: usize = 10;
/// A selection this long or longer, or over paragraphs, limits the search
/// (Ami Pro's rule) rather than filling Find (FD4).
const FILL_LIMIT: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Field {
    Find,
    Replace,
}

/// The bar's state in a tab.
#[derive(Clone, Debug)]
pub struct FindBar {
    pub find: String,
    pub replace: String,
    /// The Replace row shows (Edit › Replace…).
    pub replacing: bool,
    /// The field with the keyboard, if one has it.
    pub focused: Option<Field>,
    /// The field that had it last, where Insert Tab and Insert Return go.
    pub last_field: Field,
    /// The stream the caret was in when the search began (Ami Pro searches
    /// it and those after it).
    pub origin: FlowId,
    /// A selection the search is limited to (Ami Pro's rule; FD4).
    pub within: Option<(TextPosition, TextPosition)>,
    pub matches: Vec<Match>,
    /// The matches by flow and paragraph, for the page surface to draw.
    pub highlights: crate::page_surface::Highlights,
    /// The match selected last, while it is still selected.
    pub current: Option<usize>,
    /// The streams searched, in order: the matches' order.
    order: Vec<FlowId>,
    /// What the matches were found for.
    key: Option<SearchKey>,
    /// In All Files of the Master (MS11): the search goes on into the
    /// master's other files.
    pub all_files: bool,
    /// With it, each file of the set and its matches, in the set's order.
    pub files: Vec<SetFileMatches>,
    /// What `files` was counted for.
    pub files_key: Option<SetKey>,
}

/// What the set's matches were counted for: the text, the options, and
/// each file with its state.
pub type SetKey = (String, FindOptions, Vec<(PathBuf, u64)>);

/// One file of a master's set and how many matches it has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetFileMatches {
    pub path: PathBuf,
    pub name: String,
    pub count: usize,
}

/// What a search's matches depend on: the document's generation, the text,
/// the options, the selection it is limited to, and the stream it began
/// in.
type SearchKey = (
    u64,
    String,
    FindOptions,
    Option<(TextPosition, TextPosition)>,
    FlowId,
);

impl FindBar {
    /// Opens the bar at `selection`: a short selection in one paragraph
    /// fills Find; a longer one limits the search to it (FD4).
    pub fn open(
        document: &Document,
        selection: PageSelection,
        replacing: bool,
        previous: Option<&FindBar>,
    ) -> Self {
        let (start, end) = selection.ordered();
        let mut find = previous.map_or_else(String::new, |bar| bar.find.clone());
        let mut within = None;
        if !selection.is_caret() {
            let selected = selected_text(document, start, end);
            if start.paragraph == end.paragraph && selected.chars().count() < FILL_LIMIT {
                find = selected;
            } else {
                within = Some((start, end));
            }
        }
        Self {
            find,
            replace: previous.map_or_else(String::new, |bar| bar.replace.clone()),
            replacing: replacing || previous.is_some_and(|bar| bar.replacing),
            focused: None,
            last_field: if replacing {
                Field::Replace
            } else {
                Field::Find
            },
            origin: start.flow,
            within,
            matches: Vec::new(),
            highlights: Default::default(),
            current: None,
            order: Vec::new(),
            key: None,
            all_files: previous.is_some_and(|bar| bar.all_files),
            files: Vec::new(),
            files_key: None,
        }
    }

    /// Finds the matches again when the document, the text, the options,
    /// or the scope changed. Returns whether it did.
    pub fn refresh(&mut self, document: &Document, generation: u64, options: &FindOptions) -> bool {
        let key = (
            generation,
            self.find.clone(),
            options.clone(),
            self.within,
            self.origin,
        );
        if self.key.as_ref() == Some(&key) {
            return false;
        }
        let kept = self
            .current
            .and_then(|index| self.matches.get(index).cloned());
        self.order = find::search_streams(document, self.origin, options);
        self.matches = find::find_all(document, &self.find, &self.order, options, self.within);
        self.highlights = highlights(document, &self.matches);
        // The current match stays current while it is still found.
        self.current = kept.and_then(|kept| {
            self.matches
                .iter()
                .position(|found| found.start == kept.start && found.end == kept.end)
        });
        self.key = Some(key);
        true
    }

    /// Where a position falls in the search's order.
    fn key_of(&self, at: TextPosition) -> (usize, usize, usize) {
        let stream = self
            .order
            .iter()
            .position(|flow| *flow == at.flow)
            .unwrap_or(0);
        (stream, at.paragraph, at.byte)
    }

    /// Whether `selection` is match `index`.
    fn selects(&self, selection: PageSelection, index: usize) -> bool {
        self.matches
            .get(index)
            .is_some_and(|found| selection.ordered() == (found.start, found.end))
    }

    /// The match Find Next goes to from `selection`: the one after the
    /// current, or the first at or after the caret (the first of all with
    /// Beginning of document, before a match was found), round to the
    /// first.
    pub fn next(&self, selection: PageSelection, options: &FindOptions) -> Option<usize> {
        let count = self.matches.len();
        if count == 0 {
            return None;
        }
        if let Some(index) = self.current.filter(|index| self.selects(selection, *index)) {
            return Some((index + 1) % count);
        }
        if options.from_beginning && self.current.is_none() {
            return Some(0);
        }
        let caret = self.key_of(selection.ordered().0);
        Some(
            self.matches
                .iter()
                .position(|found| self.key_of(found.start) >= caret)
                .unwrap_or(0),
        )
    }

    /// The match Find Next goes to without going round: `None` when the
    /// next would be the first again (In All Files of the Master goes on
    /// into the next file then).
    pub fn next_in_file(&self, selection: PageSelection, options: &FindOptions) -> Option<usize> {
        if let Some(index) = self.current.filter(|index| self.selects(selection, *index)) {
            return (index + 1 < self.matches.len()).then_some(index + 1);
        }
        if options.from_beginning && self.current.is_none() && !self.matches.is_empty() {
            return Some(0);
        }
        let caret = self.key_of(selection.ordered().0);
        self.matches
            .iter()
            .position(|found| self.key_of(found.start) >= caret)
    }

    /// The match Find Previous goes to without going round.
    pub fn previous_in_file(&self, selection: PageSelection) -> Option<usize> {
        if let Some(index) = self.current.filter(|index| self.selects(selection, *index)) {
            return index.checked_sub(1);
        }
        let caret = self.key_of(selection.ordered().0);
        self.matches
            .iter()
            .rposition(|found| self.key_of(found.start) < caret)
    }

    /// The matches in every file of the set, when In All Files of the
    /// Master is on and the set has been counted.
    pub fn set_total(&self) -> Option<(usize, usize)> {
        (self.all_files && !self.files.is_empty()).then(|| {
            (
                self.files.iter().map(|file| file.count).sum(),
                self.files.iter().filter(|file| file.count > 0).count(),
            )
        })
    }

    /// The match Find Previous goes to: the one before the current, or the
    /// last before the caret, round to the last.
    pub fn previous(&self, selection: PageSelection) -> Option<usize> {
        let count = self.matches.len();
        if count == 0 {
            return None;
        }
        if let Some(index) = self.current.filter(|index| self.selects(selection, *index)) {
            return Some((index + count - 1) % count);
        }
        let caret = self.key_of(selection.ordered().0);
        Some(
            self.matches
                .iter()
                .rposition(|found| self.key_of(found.start) < caret)
                .unwrap_or(count - 1),
        )
    }

    /// The match `selection` is, if it is one: the current match (Replace
    /// replaces it), however it came to be selected.
    pub fn selected_index(&self, selection: PageSelection) -> Option<usize> {
        let (start, end) = selection.ordered();
        if self
            .current
            .is_some_and(|index| self.selects(selection, index))
        {
            return self.current;
        }
        self.matches
            .iter()
            .position(|found| (found.start, found.end) == (start, end))
    }

    /// The current match, while it is selected.
    pub fn current_selected(&self, selection: PageSelection) -> Option<&Match> {
        self.selected_index(selection)
            .and_then(|index| self.matches.get(index))
    }

    /// "3 of 12", "12 matches", "No matches", or nothing before a search.
    pub fn count_label(&self) -> String {
        if self.find.is_empty() {
            return String::new();
        }
        // In All Files of the Master: the set's matches until one is
        // selected, then its place in this file.
        if let (None, Some((total, files))) = (self.current, self.set_total()) {
            return match (total, files) {
                (0, _) => "No matches".into(),
                (1, _) => "1 in 1 file".into(),
                (total, 1) => format!("{total} in 1 file"),
                (total, files) => format!("{total} in {files} files"),
            };
        }
        match (self.current, self.matches.len()) {
            (_, 0) => "No matches".into(),
            (Some(index), count) => format!("{} of {count}", index + 1),
            (None, 1) => "1 match".into(),
            (None, count) => format!("{count} matches"),
        }
    }

    /// The text of the field `field`.
    pub fn text_mut(&mut self, field: Field) -> &mut String {
        match field {
            Field::Find => &mut self.find,
            Field::Replace => &mut self.replace,
        }
    }
}

/// Each match's bytes, paragraph by paragraph.
fn highlights(document: &Document, matches: &[Match]) -> crate::page_surface::Highlights {
    let mut map = crate::page_surface::Highlights::new();
    for found in matches {
        let (start, end) = (found.start, found.end);
        for paragraph in start.paragraph..=end.paragraph {
            let from = if paragraph == start.paragraph {
                start.byte
            } else {
                0
            };
            let to = if paragraph == end.paragraph {
                end.byte
            } else {
                document
                    .flow(start.flow)
                    .and_then(|flow| flow.paragraphs.get(paragraph))
                    .map_or(from, |paragraph| paragraph.text.len())
            };
            if from < to {
                map.entry((start.flow, paragraph))
                    .or_default()
                    .push(from..to);
            }
        }
    }
    map
}

/// The text between two positions of one flow, paragraphs joined by
/// returns.
fn selected_text(document: &Document, start: TextPosition, end: TextPosition) -> String {
    let Some(flow) = document.flow(start.flow) else {
        return String::new();
    };
    let mut text = String::new();
    for index in start.paragraph..=end.paragraph.min(flow.paragraphs.len().saturating_sub(1)) {
        let paragraph = &flow.paragraphs[index].text;
        let from = if index == start.paragraph {
            start.byte
        } else {
            0
        };
        let to = if index == end.paragraph {
            end.byte
        } else {
            paragraph.len()
        };
        if index > start.paragraph {
            text.push(find::RETURN_MARK);
        }
        text.push_str(paragraph.get(from..to).unwrap_or_default());
    }
    text.replace(amipro_document::OBJECT_REPLACEMENT_CHARACTER, "")
}

/// The page that shows a match: `current` when it does (a header shows on
/// every page), else the first that does.
pub fn match_page(layout: &DocumentLayout, found: &Match, current: u32) -> u32 {
    let shows = |page: &amipro_layout::PageBox| {
        page.lines.iter().any(|line| {
            line.flow == found.start.flow
                && line.paragraph_index == found.start.paragraph
                && line.bytes.start <= found.start.byte
                && (found.start.byte < line.bytes.end || line.bytes.is_empty())
        })
    };
    if layout
        .pages
        .iter()
        .any(|page| page.number == current && shows(page))
    {
        return current;
    }
    layout
        .pages
        .iter()
        .find(|page| shows(page))
        .map_or(current, |page| page.number)
}

/// Adds `text` to the front of the recent searches, once, keeping
/// [`RECENT_LIMIT`].
pub fn remember(recent: &mut Vec<String>, text: &str) {
    if text.is_empty() {
        return;
    }
    recent.retain(|kept| kept != text);
    recent.insert(0, text.to_owned());
    recent.truncate(RECENT_LIMIT);
}

/// The options menu's switches (Ami Pro's Find & Replace Options).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Toggle {
    WholeWord,
    MatchCase,
    KeepCapitals,
    FromBeginning,
    OtherStreams,
    Styles,
    InSelection,
    /// In All Files of the Master (MS11): the bar's, while it is open.
    AllFiles,
}

/// One of Ami Pro's six attributes, or Any (Exact attributes off).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttributeChoice {
    Any,
    Normal,
    Bold,
    Italic,
    Underline,
    WordUnderline,
    SmallCaps,
}

pub const ATTRIBUTE_CHOICES: [AttributeChoice; 7] = [
    AttributeChoice::Any,
    AttributeChoice::Normal,
    AttributeChoice::Bold,
    AttributeChoice::Italic,
    AttributeChoice::Underline,
    AttributeChoice::WordUnderline,
    AttributeChoice::SmallCaps,
];

impl AttributeChoice {
    pub fn label(self, replacing: bool) -> &'static str {
        match self {
            Self::Any if replacing => "As Found",
            Self::Any => "Any",
            Self::Normal => "Normal",
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Underline => "Underline",
            Self::WordUnderline => "Word Underline",
            Self::SmallCaps => "Small Caps",
        }
    }

    /// Whether the choice is checked in `attributes`.
    pub fn checked(self, attributes: Option<Attributes>) -> bool {
        match (self, attributes) {
            (Self::Any, None) => true,
            (_, None) | (Self::Any, Some(_)) => false,
            (Self::Normal, Some(set)) => set == Attributes::default(),
            (choice, Some(set)) => *flag(&mut set.clone(), choice),
        }
    }

    /// The attributes after choosing this: Any ends Exact attributes,
    /// Normal clears the others, another switches itself (Ami Pro's check
    /// boxes).
    pub fn choose(self, attributes: Option<Attributes>) -> Option<Attributes> {
        match self {
            Self::Any => None,
            Self::Normal => Some(Attributes::default()),
            choice => {
                let mut set = attributes.unwrap_or_default();
                let on = flag(&mut set, choice);
                *on = !*on;
                Some(set)
            }
        }
    }
}

fn flag(set: &mut Attributes, choice: AttributeChoice) -> &mut bool {
    match choice {
        AttributeChoice::Bold => &mut set.bold,
        AttributeChoice::Italic => &mut set.italic,
        AttributeChoice::Underline => &mut set.underline,
        AttributeChoice::WordUnderline => &mut set.word_underline,
        AttributeChoice::SmallCaps | AttributeChoice::Any | AttributeChoice::Normal => {
            &mut set.small_caps
        }
    }
}

/// What the bar's controls and keys do.
#[derive(Clone, Debug)]
pub enum FindAction {
    /// A field's text changed.
    Text(Field, String),
    /// Enter in a field: Find Next, or Find Previous with Shift; in Replace,
    /// Replace.
    Submit(Field),
    Next,
    Previous,
    ReplaceOne,
    ReplaceAll,
    Menu(FindMenu),
    /// A field gained or lost the keyboard.
    Focus(Field, bool),
    /// Shows or hides the Replace row.
    ShowReplace,
    Close,
    /// With Paragraph styles, a style chosen in a field's list.
    Style(Field, usize),
    /// Esc anywhere: closes the bar while one of its fields has the
    /// keyboard.
    Escape,
}

/// What the options and recent menus do.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindMenu {
    Toggle(Toggle),
    FindAttribute(AttributeChoice),
    ReplaceAttribute(AttributeChoice),
    InsertTab,
    InsertReturn,
    Recent(usize),
    /// In All Files of the Master: a file of the set, by its place, and
    /// its first match.
    File(usize),
}

/// Whether a switch is on.
pub fn toggled(options: &FindOptions, bar: &FindBar, toggle: Toggle) -> bool {
    match toggle {
        Toggle::WholeWord => options.whole_word,
        Toggle::MatchCase => options.match_case,
        Toggle::KeepCapitals => options.keep_capitals,
        Toggle::FromBeginning => options.from_beginning,
        Toggle::OtherStreams => options.other_streams,
        Toggle::Styles => options.styles,
        Toggle::InSelection => bar.within.is_some(),
        Toggle::AllFiles => bar.all_files,
    }
}

/// Flips a switch kept in the options (In the selection is the bar's).
pub fn toggle(options: &mut FindOptions, toggle: Toggle) {
    let flag = match toggle {
        Toggle::WholeWord => &mut options.whole_word,
        Toggle::MatchCase => &mut options.match_case,
        Toggle::KeepCapitals => &mut options.keep_capitals,
        Toggle::FromBeginning => &mut options.from_beginning,
        Toggle::OtherStreams => &mut options.other_streams,
        Toggle::Styles => &mut options.styles,
        Toggle::InSelection | Toggle::AllFiles => return,
    };
    *flag = !*flag;
}

/// The options as the settings keep them: six switches, then each
/// attribute set as a bit mask, or -1 without Exact attributes.
pub fn options_to_numbers(options: &FindOptions) -> Vec<i64> {
    let mask = |attributes: Option<Attributes>| {
        attributes.map_or(-1, |set| {
            i64::from(set.bold)
                | i64::from(set.italic) << 1
                | i64::from(set.underline) << 2
                | i64::from(set.word_underline) << 3
                | i64::from(set.small_caps) << 4
        })
    };
    vec![
        i64::from(options.whole_word),
        i64::from(options.match_case),
        i64::from(options.keep_capitals),
        i64::from(options.from_beginning),
        i64::from(options.other_streams),
        i64::from(options.styles),
        mask(options.find_attributes),
        mask(options.replace_attributes),
    ]
}

/// The options from the settings, or `None` when they are not eight
/// numbers in range.
pub fn options_from_numbers(numbers: &[i64]) -> Option<FindOptions> {
    let [
        whole,
        case,
        capitals,
        beginning,
        others,
        styles,
        find,
        replace,
    ] = *numbers
    else {
        return None;
    };
    let switch = |value: i64| match value {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };
    let set = |value: i64| match value {
        -1 => Some(None),
        0..=31 => Some(Some(Attributes {
            bold: value & 1 != 0,
            italic: value & 2 != 0,
            underline: value & 4 != 0,
            word_underline: value & 8 != 0,
            small_caps: value & 16 != 0,
        })),
        _ => None,
    };
    Some(FindOptions {
        whole_word: switch(whole)?,
        match_case: switch(case)?,
        keep_capitals: switch(capitals)?,
        from_beginning: switch(beginning)?,
        other_streams: switch(others)?,
        styles: switch(styles)?,
        find_attributes: set(find)?,
        replace_attributes: set(replace)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caret(document: &Document, paragraph: usize, byte: usize) -> PageSelection {
        PageSelection::caret(
            TextPosition {
                flow: document.main_flow_id(),
                paragraph,
                byte,
            },
            1,
        )
    }

    fn selecting(found: &Match) -> PageSelection {
        PageSelection {
            anchor: found.start,
            focus: found.end,
            page: 1,
        }
    }

    #[test]
    fn next_and_previous_go_round_from_the_caret() {
        let document = Document::from_plain_text("one two one\nthree one");
        let options = FindOptions::default();
        let mut bar = FindBar::open(&document, caret(&document, 0, 4), false, None);
        bar.find = "one".into();
        assert!(bar.refresh(&document, 0, &options));
        assert!(!bar.refresh(&document, 0, &options));
        assert_eq!(bar.matches.len(), 3);
        assert_eq!(bar.count_label(), "3 matches");
        // From the caret, the next is the second "one".
        let next = bar.next(caret(&document, 0, 4), &options).unwrap();
        assert_eq!(next, 1);
        bar.current = Some(next);
        assert_eq!(bar.count_label(), "2 of 3");
        let selection = selecting(&bar.matches[1]);
        assert_eq!(bar.next(selection, &options), Some(2));
        assert_eq!(bar.previous(selection), Some(0));
        // Round the end.
        bar.current = Some(2);
        assert_eq!(bar.next(selecting(&bar.matches[2]), &options), Some(0));
        // Previous from the caret: the last before it.
        bar.current = None;
        assert_eq!(bar.previous(caret(&document, 1, 0)), Some(1));
        assert_eq!(bar.previous(caret(&document, 0, 0)), Some(2));
        // Beginning of document: the first of all before any match.
        let beginning = FindOptions {
            from_beginning: true,
            ..FindOptions::default()
        };
        assert_eq!(bar.next(caret(&document, 1, 0), &beginning), Some(0));
        // A changed document finds them again; the current one stays.
        bar.current = Some(1);
        assert!(bar.refresh(&document, 1, &options));
        assert_eq!(bar.current, Some(1));
        bar.find = "three".into();
        bar.refresh(&document, 1, &options);
        assert_eq!((bar.current, bar.count_label().as_str()), (None, "1 match"));
        bar.find = "four".into();
        bar.refresh(&document, 1, &options);
        assert_eq!(bar.count_label(), "No matches");
    }

    #[test]
    fn a_selection_fills_find_or_limits_the_search() {
        let document = Document::from_plain_text("alpha beta\ngamma");
        let main = document.main_flow_id();
        let at = |paragraph, byte| TextPosition {
            flow: main,
            paragraph,
            byte,
        };
        let word = PageSelection {
            anchor: at(0, 6),
            focus: at(0, 10),
            page: 1,
        };
        let bar = FindBar::open(&document, word, false, None);
        assert_eq!((bar.find.as_str(), bar.within), ("beta", None));
        let across = PageSelection {
            anchor: at(1, 3),
            focus: at(0, 2),
            page: 1,
        };
        let previous = bar.clone();
        let bar = FindBar::open(&document, across, true, Some(&previous));
        assert_eq!(bar.find, "beta");
        assert_eq!(bar.within, Some((at(0, 2), at(1, 3))));
        assert!(bar.replacing);
    }

    #[test]
    fn recent_searches_keep_ten_newest_first() {
        let mut recent = Vec::new();
        for text in ["a", "b", "a", ""] {
            remember(&mut recent, text);
        }
        assert_eq!(recent, ["a", "b"]);
        for index in 0..20 {
            remember(&mut recent, &index.to_string());
        }
        assert_eq!(recent.len(), RECENT_LIMIT);
        assert_eq!(recent[0], "19");
    }

    #[test]
    fn options_round_trip_in_the_settings_and_attributes_choose_as_ami_pros() {
        let mut options = FindOptions::default();
        assert_eq!(
            options_from_numbers(&options_to_numbers(&options)),
            Some(options.clone())
        );
        toggle(&mut options, Toggle::WholeWord);
        toggle(&mut options, Toggle::Styles);
        options.find_attributes = AttributeChoice::Italic.choose(None);
        options.replace_attributes = AttributeChoice::Normal.choose(Some(Attributes {
            bold: true,
            ..Attributes::default()
        }));
        let numbers = options_to_numbers(&options);
        assert_eq!(numbers, [0, 0, 1, 0, 1, 1, 2, 0]);
        assert_eq!(options_from_numbers(&numbers), Some(options.clone()));
        assert_eq!(options_from_numbers(&[1, 2, 3]), None);
        assert_eq!(options_from_numbers(&[2, 0, 1, 0, 1, 1, -1, -1]), None);
        assert_eq!(options_from_numbers(&[1, 0, 1, 0, 1, 0, 32, -1]), None);
        // Checks: Any without Exact attributes; Normal with none set.
        assert!(AttributeChoice::Any.checked(None));
        assert!(AttributeChoice::Italic.checked(options.find_attributes));
        assert!(!AttributeChoice::Bold.checked(options.find_attributes));
        assert!(AttributeChoice::Normal.checked(options.replace_attributes));
        // Choosing an attribute again switches it off; Any ends it.
        let twice = AttributeChoice::Italic.choose(options.find_attributes);
        assert_eq!(twice, Some(Attributes::default()));
        assert_eq!(AttributeChoice::Any.choose(twice), None);
    }
}
