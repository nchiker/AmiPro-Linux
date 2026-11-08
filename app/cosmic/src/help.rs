//! The in-app Help (`docs/Design/UI_SPEC.md` §45): topics written as
//! Markdown files in `app/cosmic/help/<area>/<topic>.md`, embedded when the
//! program is built (`build.rs` lists the files, so nothing is read from
//! disk or the network at run time).
//!
//! This file is the core and has no window in it: it reads a topic, indexes
//! the topics, searches them, finds the topic for a context, and keeps the
//! Back and Forward history. `help_panel.rs` is the rail panel over it.
//!
//! A topic file starts with a front matter between two `---` lines:
//!
//! ```text
//! id: styles                      (the file name without .md)
//! title: Styles
//! area: document                  (the folder; one of AREAS)
//! keywords: style sheet, STY      (comma separated; searched)
//! see_also: text-formatting       (topic ids)
//! context: command:Styles, panel:StyleProperties   (F1 on these)
//! ```
//!
//! The body is a small Markdown: `##` and `###` headings, paragraphs,
//! `-` and `1.` lists, fenced code, `**bold**`, `*italic*`, `` `code` ``, and
//! `[label](topic-id)` links to another topic. `docs/HELP_AUTHORING.md` says
//! how to write one, and that the wording must be original.

use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/help_index.rs"));

/// The areas, in the order the contents shows them: (folder, title).
pub const AREAS: &[(&str, &str)] = &[
    ("start", "Getting started"),
    ("document", "Writing and layout"),
    ("objects", "Tables, pictures, and drawings"),
    ("files", "Files and output"),
    ("reference", "Reference"),
];

/// The title of an area, or its folder name when it is not one of [`AREAS`].
pub fn area_title(area: &str) -> &str {
    AREAS
        .iter()
        .find(|(folder, _)| *folder == area)
        .map_or(area, |(_, title)| title)
}

/// A piece of a line of text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Inline {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    /// A link to the topic with id `topic`.
    Link {
        label: String,
        topic: String,
    },
}

impl Inline {
    /// The words without the marks.
    pub fn plain(&self) -> &str {
        match self {
            Self::Text(text) | Self::Bold(text) | Self::Italic(text) | Self::Code(text) => text,
            Self::Link { label, .. } => label,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Block {
    /// Level 2 or 3.
    Heading(u8, Vec<Inline>),
    Paragraph(Vec<Inline>),
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    Code(String),
}

/// One Help topic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Topic {
    /// Where the file is under `help/` (`document/styles.md`).
    pub path: String,
    pub id: String,
    pub title: String,
    pub area: String,
    pub keywords: Vec<String>,
    pub see_also: Vec<String>,
    /// The contexts F1 opens this topic for: `command:Name` (a command id of
    /// the registry) or `panel:Name` (a rail panel).
    pub contexts: Vec<String>,
    pub blocks: Vec<Block>,
    /// The body's words in lower case, for the search.
    text: String,
}

impl Topic {
    /// The first paragraph as plain text, cut at `limit` characters.
    pub fn summary(&self, limit: usize) -> String {
        let first = self.blocks.iter().find_map(|block| match block {
            Block::Paragraph(inlines) => Some(plain_text(inlines)),
            _ => None,
        });
        shorten(&first.unwrap_or_default(), limit)
    }

    /// Every link in the body, in order.
    #[cfg(test)]
    pub fn links(&self) -> Vec<&str> {
        let mut links = Vec::new();
        for block in &self.blocks {
            match block {
                Block::Heading(_, inlines) | Block::Paragraph(inlines) => {
                    for inline in inlines {
                        if let Inline::Link { topic, .. } = inline {
                            links.push(topic.as_str());
                        }
                    }
                }
                Block::List { items, .. } => {
                    for item in items {
                        for inline in item {
                            if let Inline::Link { topic, .. } = inline {
                                links.push(topic.as_str());
                            }
                        }
                    }
                }
                Block::Code(_) => {}
            }
        }
        links
    }
}

fn plain_text(inlines: &[Inline]) -> String {
    inlines.iter().map(Inline::plain).collect()
}

fn shorten(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_owned();
    }
    let cut: String = text.chars().take(limit).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([',', ';', ':', '.']))
}

fn is_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Reads one topic file; `path` is its place under `help/`.
pub fn parse_topic(path: &str, source: &str) -> Result<Topic, String> {
    let mut lines = source.lines();
    if lines.next().map(str::trim) != Some("---") {
        return Err("the file must start with a --- line".into());
    }
    let (mut id, mut title, mut area) = (None, None, None);
    let (mut keywords, mut see_also, mut contexts) = (Vec::new(), Vec::new(), Vec::new());
    let mut closed = false;
    for line in lines.by_ref() {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("front matter line without a key: {line:?}"));
        };
        let value = value.trim();
        match key.trim() {
            "id" => id = Some(value.to_owned()),
            "title" => title = Some(value.to_owned()),
            "area" => area = Some(value.to_owned()),
            "keywords" => keywords = list(value),
            "see_also" => see_also = list(value),
            "context" => contexts = list(value),
            other => return Err(format!("unknown front matter key {other:?}")),
        }
    }
    if !closed {
        return Err("the front matter has no closing --- line".into());
    }
    let id = id.ok_or("the front matter has no id")?;
    if !is_id(&id) {
        return Err(format!(
            "the id {id:?} must be lower-case letters, digits, and hyphens"
        ));
    }
    let title = title.ok_or("the front matter has no title")?;
    let area = area.ok_or("the front matter has no area")?;
    let body: Vec<&str> = lines.collect();
    let blocks = parse_blocks(&body)?;
    let mut text = String::new();
    for block in &blocks {
        match block {
            Block::Heading(_, inlines) | Block::Paragraph(inlines) => {
                text.push_str(&plain_text(inlines));
                text.push('\n');
            }
            Block::List { items, .. } => {
                for item in items {
                    text.push_str(&plain_text(item));
                    text.push('\n');
                }
            }
            Block::Code(code) => {
                text.push_str(code);
                text.push('\n');
            }
        }
    }
    Ok(Topic {
        path: path.to_owned(),
        id,
        title,
        area,
        keywords,
        see_also,
        contexts,
        blocks,
        text: text.to_lowercase(),
    })
}

fn parse_blocks(lines: &[&str]) -> Result<Vec<Block>, String> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut index = 0;
    fn flush(paragraph: &mut Vec<&str>, blocks: &mut Vec<Block>) -> Result<(), String> {
        if !paragraph.is_empty() {
            let text = paragraph.join(" ");
            blocks.push(Block::Paragraph(parse_inlines(&text)?));
            paragraph.clear();
        }
        Ok(())
    }
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush(&mut paragraph, &mut blocks)?;
        } else if trimmed.starts_with("```") {
            flush(&mut paragraph, &mut blocks)?;
            let mut code = Vec::new();
            index += 1;
            loop {
                let Some(line) = lines.get(index) else {
                    return Err("a code block is not closed".into());
                };
                if line.trim().starts_with("```") {
                    break;
                }
                code.push(*line);
                index += 1;
            }
            blocks.push(Block::Code(code.join("\n")));
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            flush(&mut paragraph, &mut blocks)?;
            blocks.push(Block::Heading(3, parse_inlines(rest.trim())?));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            flush(&mut paragraph, &mut blocks)?;
            blocks.push(Block::Heading(2, parse_inlines(rest.trim())?));
        } else if trimmed.starts_with("# ") {
            return Err("a topic's title is in the front matter; headings start at ##".into());
        } else if let Some((ordered, rest)) = list_item(trimmed) {
            flush(&mut paragraph, &mut blocks)?;
            let mut items = vec![rest.to_owned()];
            while let Some(next) = lines.get(index + 1) {
                let next_trimmed = next.trim();
                if let Some((next_ordered, rest)) = list_item(next_trimmed) {
                    if next_ordered != ordered {
                        break;
                    }
                    items.push(rest.to_owned());
                } else if next.starts_with("  ") && !next_trimmed.is_empty() {
                    let last = items.last_mut().expect("an item");
                    last.push(' ');
                    last.push_str(next_trimmed);
                } else {
                    break;
                }
                index += 1;
            }
            let items = items
                .iter()
                .map(|item| parse_inlines(item))
                .collect::<Result<Vec<_>, _>>()?;
            blocks.push(Block::List { ordered, items });
        } else {
            paragraph.push(trimmed);
        }
        index += 1;
    }
    flush(&mut paragraph, &mut blocks)?;
    Ok(blocks)
}

/// (ordered, the item's text) for a list line.
fn list_item(line: &str) -> Option<(bool, &str)> {
    if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return Some((false, rest.trim()));
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && line[digits..].starts_with(". ") {
        return Some((true, line[digits + 2..].trim()));
    }
    None
}

/// Reads `**bold**`, `*italic*`, `` `code` ``, and `[label](topic)`; a
/// backslash makes the next character plain. A mark with no end is plain.
pub fn parse_inlines(text: &str) -> Result<Vec<Inline>, String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut plain = String::new();
    let mut i = 0;
    // The end of an emphasis or code mark: the mark again, not right after a
    // space (so `2 * 3 * 4` stays plain). A link's brackets may follow a space.
    let find = |from: usize, pattern: &[char]| -> Option<usize> {
        (from..chars.len().saturating_sub(pattern.len() - 1)).find(|at| {
            chars[*at..*at + pattern.len()] == *pattern
                && (!matches!(pattern[0], '*') || *at == 0 || !chars[*at - 1].is_whitespace())
        })
    };
    fn flush(plain: &mut String, out: &mut Vec<Inline>) {
        if !plain.is_empty() {
            out.push(Inline::Text(std::mem::take(plain)));
        }
    }
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' && i + 1 < chars.len() {
            plain.push(chars[i + 1]);
            i += 2;
        } else if c == '*' && chars.get(i + 1).is_none_or(|next| next.is_whitespace()) {
            plain.push(c);
            i += 1;
        } else if c == '*' && chars.get(i + 1) == Some(&'*') {
            if let Some(end) = find(i + 2, &['*', '*']).filter(|end| *end > i + 2) {
                flush(&mut plain, &mut out);
                out.push(Inline::Bold(chars[i + 2..end].iter().collect()));
                i = end + 2;
            } else {
                plain.push_str("**");
                i += 2;
            }
        } else if c == '*' {
            if let Some(end) = find(i + 1, &['*']).filter(|end| *end > i + 1) {
                flush(&mut plain, &mut out);
                out.push(Inline::Italic(chars[i + 1..end].iter().collect()));
                i = end + 1;
            } else {
                plain.push(c);
                i += 1;
            }
        } else if c == '`' {
            if let Some(end) = find(i + 1, &['`']).filter(|end| *end > i + 1) {
                flush(&mut plain, &mut out);
                out.push(Inline::Code(chars[i + 1..end].iter().collect()));
                i = end + 1;
            } else {
                plain.push(c);
                i += 1;
            }
        } else if c == '[' {
            let label_end = find(i + 1, &[']']);
            let link = label_end.and_then(|close| {
                (chars.get(close + 1) == Some(&'('))
                    .then(|| find(close + 2, &[')']).map(|end| (close, end)))
                    .flatten()
            });
            if let Some((close, end)) = link {
                let label: String = chars[i + 1..close].iter().collect();
                let target: String = chars[close + 2..end].iter().collect();
                if !is_id(&target) {
                    return Err(format!(
                        "the link target {target:?} must be a topic id (lower-case letters, digits, hyphens)"
                    ));
                }
                if label.trim().is_empty() {
                    return Err(format!("the link to {target:?} has no label"));
                }
                flush(&mut plain, &mut out);
                out.push(Inline::Link {
                    label,
                    topic: target,
                });
                i = end + 1;
            } else {
                plain.push(c);
                i += 1;
            }
        } else {
            plain.push(c);
            i += 1;
        }
    }
    flush(&mut plain, &mut out);
    Ok(out)
}

/// A search result.
#[derive(Clone, Debug)]
pub struct Hit<'a> {
    pub topic: &'a Topic,
    pub score: u32,
}

/// All the topics.
#[derive(Debug)]
pub struct Library {
    topics: Vec<Topic>,
}

impl Library {
    /// Reads `(path, source)` pairs; the first error names its file.
    #[cfg(test)]
    pub fn parse(sources: &[(&str, &str)]) -> Result<Self, String> {
        let mut topics = Vec::new();
        for (path, source) in sources {
            topics.push(parse_topic(path, source).map_err(|error| format!("{path}: {error}"))?);
        }
        Ok(Self { topics })
    }

    /// The topics built into the program. A file that does not parse is left
    /// out (a test fails on it).
    pub fn builtin() -> &'static Library {
        static LIBRARY: OnceLock<Library> = OnceLock::new();
        LIBRARY.get_or_init(|| Library {
            topics: SOURCES
                .iter()
                .filter_map(|(path, source)| parse_topic(path, source).ok())
                .collect(),
        })
    }

    #[cfg(test)]
    pub fn topics(&self) -> &[Topic] {
        &self.topics
    }

    pub fn get(&self, id: &str) -> Option<&Topic> {
        self.topics.iter().find(|topic| topic.id == id)
    }

    /// The topics of an area, by title.
    pub fn in_area(&self, area: &str) -> Vec<&Topic> {
        let mut topics: Vec<&Topic> = self
            .topics
            .iter()
            .filter(|topic| topic.area == area)
            .collect();
        topics.sort_by_key(|topic| topic.title.to_lowercase());
        topics
    }

    /// What is wrong with the library as a whole: duplicate ids, a file in
    /// the wrong place, an unknown area, a link or See also that names no
    /// topic. Empty when it is sound.
    #[cfg(test)]
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        for (position, topic) in self.topics.iter().enumerate() {
            if self.topics[..position]
                .iter()
                .any(|other| other.id == topic.id)
            {
                problems.push(format!(
                    "{}: the id {:?} is used twice",
                    topic.path, topic.id
                ));
            }
            if !AREAS.iter().any(|(folder, _)| *folder == topic.area) {
                problems.push(format!("{}: unknown area {:?}", topic.path, topic.area));
            }
            let expected = format!("{}/{}.md", topic.area, topic.id);
            if topic.path != expected {
                problems.push(format!("{}: should be {expected}", topic.path));
            }
            for target in topic
                .links()
                .into_iter()
                .chain(topic.see_also.iter().map(String::as_str))
            {
                if self.get(target).is_none() {
                    problems.push(format!("{}: no topic {target:?}", topic.path));
                }
                if target == topic.id {
                    problems.push(format!("{}: links to itself", topic.path));
                }
            }
        }
        problems
    }

    /// The topic F1 opens for the first of `contexts` that has one.
    pub fn for_contexts<S: AsRef<str>>(&self, contexts: &[S]) -> Option<&Topic> {
        contexts.iter().find_map(|context| {
            self.topics
                .iter()
                .find(|topic| topic.contexts.iter().any(|own| own == context.as_ref()))
        })
    }

    /// Topics whose title, keywords, area, or text contain every word of
    /// `query` (any case), best first: a word in the title ranks above one
    /// in the keywords, which ranks above one in the text.
    pub fn search(&self, query: &str) -> Vec<Hit<'_>> {
        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if terms.is_empty() {
            return Vec::new();
        }
        let whole = terms.join(" ");
        let mut hits: Vec<Hit<'_>> = self
            .topics
            .iter()
            .filter_map(|topic| {
                let title = topic.title.to_lowercase();
                let title_words: Vec<&str> = title
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|word| !word.is_empty())
                    .collect();
                let area = area_title(&topic.area).to_lowercase();
                let mut score = 0;
                for term in &terms {
                    let mut best = 0;
                    if title_words.contains(&term.as_str()) {
                        best = 12;
                    } else if title.contains(term.as_str()) {
                        best = 8;
                    }
                    for keyword in &topic.keywords {
                        let keyword = keyword.to_lowercase();
                        if keyword == *term {
                            best = best.max(8);
                        } else if keyword.contains(term.as_str()) {
                            best = best.max(5);
                        }
                    }
                    if area.contains(term.as_str()) {
                        best = best.max(2);
                    }
                    let in_text = topic.text.matches(term.as_str()).count();
                    if in_text > 0 {
                        best = best.max(1) + 1 + in_text.min(3) as u32;
                    }
                    if best == 0 {
                        return None;
                    }
                    score += best;
                }
                if title.contains(&whole) {
                    score += 10;
                }
                Some(Hit { topic, score })
            })
            .collect();
        hits.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.topic.title.cmp(&b.topic.title))
        });
        hits
    }
}

/// What the Help window shows.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Page {
    /// The topics by area.
    #[default]
    Contents,
    /// The topic with this id.
    Topic(String),
    /// The results of the query (the contents, while the query is empty).
    Search,
}

/// Where the reader is, and where Back and Forward go.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Navigator {
    pub page: Page,
    pub query: String,
    back: Vec<Page>,
    forward: Vec<Page>,
}

/// How many pages Back remembers.
const HISTORY: usize = 50;

impl Navigator {
    /// Goes to `page`; the page left is what Back returns to, and Forward is
    /// forgotten. Going to the page shown changes nothing.
    pub fn go(&mut self, page: Page) {
        if page == self.page {
            return;
        }
        let left = std::mem::replace(&mut self.page, page);
        self.back.push(left);
        if self.back.len() > HISTORY {
            self.back.remove(0);
        }
        self.forward.clear();
    }

    pub fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub fn can_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    pub fn back(&mut self) {
        if let Some(page) = self.back.pop() {
            let left = std::mem::replace(&mut self.page, page);
            self.forward.push(left);
        }
    }

    pub fn forward(&mut self) {
        if let Some(page) = self.forward.pop() {
            let left = std::mem::replace(&mut self.page, page);
            self.back.push(left);
        }
    }

    /// The search box changed: a query shows the results (the page left is
    /// kept for Back once, however many letters are typed).
    pub fn set_query(&mut self, query: String) {
        let typing = !query.trim().is_empty();
        self.query = query;
        if typing {
            self.go(Page::Search);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library() -> &'static Library {
        Library::builtin()
    }

    fn sample(id: &str, title: &str, keywords: &str, body: &str) -> String {
        format!(
            "---\nid: {id}\ntitle: {title}\narea: document\nkeywords: {keywords}\n---\n{body}\n"
        )
    }

    #[test]
    fn a_topic_parses_its_front_matter_and_blocks() {
        let source = "---\nid: sample\ntitle: A sample\narea: document\nkeywords: one, two words\nsee_also: other\ncontext: command:Find, panel:Sort\n---\nFirst *line* with **bold** and `code`.\n\n## Steps\n\n1. Open it\n   and read.\n2. Close it\n\n- a [link](other)\n- plain\n\n```\nverbatim * text\n```\n";
        let topic = parse_topic("document/sample.md", source).unwrap();
        assert_eq!(topic.id, "sample");
        assert_eq!(topic.keywords, ["one", "two words"]);
        assert_eq!(topic.see_also, ["other"]);
        assert_eq!(topic.contexts, ["command:Find", "panel:Sort"]);
        assert_eq!(topic.blocks.len(), 5);
        assert_eq!(
            topic.blocks[0],
            Block::Paragraph(vec![
                Inline::Text("First ".into()),
                Inline::Italic("line".into()),
                Inline::Text(" with ".into()),
                Inline::Bold("bold".into()),
                Inline::Text(" and ".into()),
                Inline::Code("code".into()),
                Inline::Text(".".into()),
            ])
        );
        let Block::List { ordered, items } = &topic.blocks[2] else {
            panic!("a list");
        };
        assert!(*ordered);
        assert_eq!(plain_text(&items[0]), "Open it and read.");
        assert_eq!(topic.links(), ["other"]);
        assert_eq!(topic.blocks[4], Block::Code("verbatim * text".into()));
    }

    #[test]
    fn bad_topics_are_refused_with_a_reason() {
        for (source, words) in [
            ("no front matter", "---"),
            ("---\nid: a\ntitle: A\n---\n", "area"),
            ("---\nid: A B\ntitle: A\narea: x\n---\n", "id"),
            ("---\nid: a\ntitle: A\narea: x\nmood: y\n---\n", "mood"),
            ("---\nid: a\ntitle: A\narea: x\n", "closing"),
            (
                "---\nid: a\ntitle: A\narea: x\n---\n# Title\n",
                "front matter",
            ),
            (
                "---\nid: a\ntitle: A\narea: x\n---\n[x](http://a)\n",
                "topic id",
            ),
            (
                "---\nid: a\ntitle: A\narea: x\n---\n```\nopen\n",
                "not closed",
            ),
        ] {
            let error = parse_topic("x/a.md", source).unwrap_err();
            assert!(error.contains(words), "{error:?} should mention {words:?}");
        }
    }

    #[test]
    fn marks_without_an_end_and_escapes_are_plain() {
        assert_eq!(
            parse_inlines("2 * 3 and **open and [no link] and \\*star\\*").unwrap(),
            [Inline::Text(
                "2 * 3 and **open and [no link] and *star*".into()
            )]
        );
    }

    #[test]
    fn the_library_finds_problems_between_topics() {
        let a = sample("a", "A", "", "See [b](b) and [c](c).");
        let b = sample("b", "B", "", "Text.");
        let library = Library::parse(&[
            ("document/a.md", &a),
            ("document/b.md", &b),
            ("document/b.md", &b),
        ])
        .unwrap();
        let problems = library.problems().join("\n");
        assert!(problems.contains("no topic \"c\""), "{problems}");
        assert!(problems.contains("used twice"), "{problems}");
        let wrong = sample("x", "X", "", "Text.");
        let library = Library::parse(&[("files/x.md", &wrong)]).unwrap();
        assert!(
            library
                .problems()
                .join("\n")
                .contains("should be document/x.md")
        );
    }

    #[test]
    fn search_ranks_titles_above_keywords_above_text() {
        let title = sample("t", "Frames", "boxes", "Nothing here.");
        let keyword = sample("k", "Layout", "frames", "Nothing here.");
        let text = sample("x", "Other", "", "A frames paragraph. Frames again.");
        let none = sample("n", "None", "", "Unrelated.");
        let library = Library::parse(&[
            ("document/t.md", &title),
            ("document/k.md", &keyword),
            ("document/x.md", &text),
            ("document/n.md", &none),
        ])
        .unwrap();
        let order: Vec<&str> = library
            .search("FRAMES")
            .iter()
            .map(|hit| hit.topic.id.as_str())
            .collect();
        assert_eq!(order, ["t", "k", "x"]);
        // Every word must be found somewhere in the topic.
        assert_eq!(library.search("frames boxes").len(), 1);
        assert!(library.search("frames zzz").is_empty());
        assert!(library.search("   ").is_empty());
    }

    #[test]
    fn contexts_find_their_topic_and_the_first_candidate_wins() {
        let a = "---\nid: a\ntitle: A\narea: document\ncontext: command:Find\n---\nText.\n";
        let b =
            "---\nid: b\ntitle: B\narea: document\ncontext: panel:Sort, command:Sort\n---\nText.\n";
        let library = Library::parse(&[("document/a.md", a), ("document/b.md", b)]).unwrap();
        assert_eq!(library.for_contexts(&["command:Find"]).unwrap().id, "a");
        assert_eq!(
            library
                .for_contexts(&["panel:Nothing", "panel:Sort", "command:Find"])
                .unwrap()
                .id,
            "b"
        );
        assert!(library.for_contexts(&["command:Nothing"]).is_none());
        assert!(library.for_contexts::<&str>(&[]).is_none());
    }

    #[test]
    fn history_goes_back_and_forward_and_forgets_forward_on_a_new_page() {
        let mut nav = Navigator::default();
        assert!(!nav.can_back() && !nav.can_forward());
        nav.go(Page::Topic("a".into()));
        nav.go(Page::Topic("a".into()));
        nav.go(Page::Topic("b".into()));
        assert_eq!(nav.page, Page::Topic("b".into()));
        nav.back();
        assert_eq!(nav.page, Page::Topic("a".into()));
        assert!(nav.can_forward());
        nav.forward();
        assert_eq!(nav.page, Page::Topic("b".into()));
        nav.back();
        nav.go(Page::Topic("c".into()));
        assert!(!nav.can_forward());
        nav.back();
        nav.back();
        assert_eq!(nav.page, Page::Contents);
        assert!(!nav.can_back());
        nav.back();
        assert_eq!(nav.page, Page::Contents);
    }

    #[test]
    fn typing_a_query_keeps_one_history_step() {
        let mut nav = Navigator::default();
        nav.go(Page::Topic("a".into()));
        for query in ["f", "fr", "fra"] {
            nav.set_query(query.into());
        }
        assert_eq!(nav.page, Page::Search);
        nav.back();
        assert_eq!(nav.page, Page::Topic("a".into()));
        nav.set_query(String::new());
        assert_eq!(
            nav.page,
            Page::Topic("a".into()),
            "an empty query goes nowhere"
        );
    }

    #[test]
    fn long_summaries_end_at_a_word() {
        let topic = parse_topic(
            "document/s.md",
            &sample("s", "S", "", "Alpha beta gamma delta epsilon zeta."),
        )
        .unwrap();
        assert_eq!(topic.summary(100), "Alpha beta gamma delta epsilon zeta.");
        assert_eq!(topic.summary(18), "Alpha beta gamma…");
    }

    // ---- The topics that ship ------------------------------------------

    #[test]
    fn every_shipped_topic_parses_and_the_library_is_sound() {
        for (path, source) in SOURCES {
            if let Err(error) = parse_topic(path, source) {
                panic!("{path}: {error}");
            }
        }
        assert_eq!(library().topics().len(), SOURCES.len());
        assert!(library().topics().len() >= 25, "the starter set");
        let problems = library().problems();
        assert!(problems.is_empty(), "{}", problems.join("\n"));
        for topic in library().topics() {
            assert!(!topic.blocks.is_empty(), "{} has a body", topic.path);
            assert!(!topic.title.is_empty());
            assert!(
                topic.summary(200).chars().count() > 20,
                "{} opens with a paragraph",
                topic.path
            );
        }
    }

    #[test]
    fn the_help_folder_and_the_embedded_list_agree() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("help");
        let mut on_disk = 0;
        for area in std::fs::read_dir(&root).unwrap().flatten() {
            for file in std::fs::read_dir(area.path()).unwrap().flatten() {
                if file.path().extension().is_some_and(|e| e == "md") {
                    on_disk += 1;
                }
            }
        }
        assert_eq!(on_disk, SOURCES.len());
    }

    #[test]
    fn the_shipped_topics_are_found_by_their_words() {
        let hits = library().search("find replace");
        assert_eq!(hits[0].topic.id, "find-replace");
        assert!(
            library()
                .search("table")
                .iter()
                .any(|h| h.topic.id == "tables")
        );
    }
}
