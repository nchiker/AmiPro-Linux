//! Ami Pro's table of contents and index as its files keep them
//! (`docs/research/TOC_INDEX_NOTES.md`): the TOC levels by style
//! (`[toc]`), the TOC, Index dialog's output files and options
//! (`[master]`), and the two marks, index entries and TOC entries, which are
//! power fields without a result (`<:X8,V;Index …>`, `<:X11,V;TOC …>`).

use crate::power::{escape, unescape};

/// The most TOC levels Ami Pro has.
pub const TOC_LEVELS: usize = 9;

/// The frame flag that marks a generated TOC's page table: Ami Pro finds
/// the table again by it and replaces its rows (`1588:0694`; VM TI1T2).
/// Kept among a frame's unknown flags (`frame.flags.unknown`).
pub const TOC_TABLE_FLAG: u32 = 0x400_0000;

const UNKNOWN_FLAGS: &str = "frame.flags.unknown";

/// The frame flags a frame keeps beyond those the model maps.
fn unknown_flags(frame: &crate::Frame) -> u32 {
    frame
        .legacy
        .values
        .iter()
        .filter(|value| value.key == UNKNOWN_FLAGS)
        .filter_map(|value| u32::from_str_radix(value.value.trim_start_matches("0x"), 16).ok())
        .fold(0, |flags, value| flags | value)
}

/// Whether `frame` is a generated TOC's table.
pub fn is_toc_table(frame: &crate::Frame) -> bool {
    unknown_flags(frame) & TOC_TABLE_FLAG != 0
}

/// Marks `frame` as a generated TOC's table, keeping its other kept bits.
pub fn mark_toc_table(frame: &mut crate::Frame) {
    let flags = unknown_flags(frame) | TOC_TABLE_FLAG;
    frame.legacy.remove(UNKNOWN_FLAGS);
    frame.legacy.push(UNKNOWN_FLAGS, format!("{flags:#x}"));
}

/// One TOC level's flag bits in `[toc]` (`1450:0211`, `1588:24ed`).
pub mod toc_flag {
    pub const PAGE_NUMBER: u8 = 1;
    pub const RIGHT_ALIGN: u8 = 2;
}

/// What stands between an entry and its page (TOC Options' Leader or
/// Separator box).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Leader {
    #[default]
    None,
    /// `.....` (right aligned pages).
    Dots,
    /// `-----` (right aligned pages).
    Hyphens,
    /// `_____` (right aligned pages).
    Line,
    /// `, ` (pages after the entry).
    Comma,
}

impl Leader {
    /// The leader a stored separator names: its first byte (`1450:07ec`).
    pub fn of_separator(separator: &str) -> Self {
        match separator.chars().next() {
            Some('.') => Self::Dots,
            Some('-') => Self::Hyphens,
            Some('_') => Self::Line,
            Some(',') => Self::Comma,
            _ => Self::None,
        }
    }

    /// The separator TOC Options stores for this leader (`1450:0824`).
    pub fn separator(self) -> &'static str {
        match self {
            Self::None => "  ",
            Self::Dots => ".",
            Self::Hyphens => "-",
            Self::Line => "_",
            Self::Comma => ", ",
        }
    }
}

/// One TOC level as TOC Options shows it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TocLevel {
    /// The paragraph style whose paragraphs are this level's entries.
    pub style: String,
    pub page_number: bool,
    pub right_align: bool,
    pub leader: Leader,
}

/// One stored place of `[toc]`: a style name, its separator, its flags.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TocSlot {
    pub style: String,
    pub separator: String,
    pub flags: u8,
}

/// `[toc]`, kept place by place so that it is written back as read: the
/// first three places' names, then their separators, then one word of
/// their flags (two bits a place), then a name, separator, and flags for
/// each place after the third (`1588:261d`, `1588:24ed`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TocSettings {
    pub slots: Vec<TocSlot>,
    /// The flags word's bits above the first three places' (older files
    /// carry `0x4000`; Ami Pro's reader ignores them).
    pub extra_flags: u16,
}

impl TocSettings {
    /// `[toc]`'s values (unescaped by [`TocSettings::from_values`]), or
    /// `None` when they are not Ami Pro's form.
    pub fn from_values(values: &[String]) -> Option<Self> {
        if values.len() < 7 || (values.len() - 7) % 3 != 0 {
            return None;
        }
        let word: u16 = values[6].trim().parse::<i32>().ok()? as u16;
        let mut slots = Vec::new();
        for index in 0..3 {
            slots.push(TocSlot {
                style: unescape(&values[index])?,
                separator: unescape(&values[3 + index])?,
                flags: ((word >> (2 * index)) & 3) as u8,
            });
        }
        for triple in values[7..].chunks(3) {
            slots.push(TocSlot {
                style: unescape(&triple[0])?,
                separator: unescape(&triple[1])?,
                flags: triple[2].trim().parse::<u8>().ok()?,
            });
        }
        Some(Self {
            slots,
            extra_flags: word & !0x3F,
        })
    }

    /// The values Ami Pro writes, escaped as body text.
    pub fn values(&self) -> Vec<String> {
        let slot = |index: usize| self.slots.get(index).cloned().unwrap_or_default();
        let mut values: Vec<String> = (0..3).map(|index| escape(&slot(index).style)).collect();
        values.extend((0..3).map(|index| escape(&slot(index).separator)));
        let word = (0..3).fold(self.extra_flags, |word, index| {
            word | (u16::from(slot(index).flags & 3) << (2 * index))
        });
        values.push(word.to_string());
        for slot in self.slots.iter().skip(3) {
            values.push(escape(&slot.style));
            values.push(escape(&slot.separator));
            values.push(slot.flags.to_string());
        }
        values
    }

    /// The levels in order: the places that name a style (a style's level
    /// is its place among them).
    pub fn levels(&self) -> Vec<TocLevel> {
        self.slots
            .iter()
            .filter(|slot| !slot.style.is_empty())
            .take(TOC_LEVELS)
            .map(|slot| TocLevel {
                style: slot.style.clone(),
                page_number: slot.flags & toc_flag::PAGE_NUMBER != 0,
                right_align: slot.flags & toc_flag::RIGHT_ALIGN != 0,
                leader: Leader::of_separator(&slot.separator),
            })
            .collect()
    }

    /// The settings TOC Options writes for `levels` (in level order; the
    /// first three places always written, as Ami Pro writes them).
    pub fn from_levels(levels: &[TocLevel]) -> Self {
        let mut slots: Vec<TocSlot> = levels
            .iter()
            .take(TOC_LEVELS)
            .map(|level| {
                let mut flags = 0;
                if level.page_number {
                    flags |= toc_flag::PAGE_NUMBER;
                    if level.right_align {
                        flags |= toc_flag::RIGHT_ALIGN;
                    }
                }
                TocSlot {
                    style: level.style.clone(),
                    separator: level.leader.separator().to_string(),
                    flags,
                }
            })
            .collect();
        while slots.len() < 3 {
            slots.push(TocSlot::default());
        }
        Self {
            slots,
            extra_flags: 0,
        }
    }
}

/// `[master]`: the TOC, Index dialog's output files and options, then the
/// master document's file list (`1660:0000` writer, `1660:00fe` reader;
/// `docs/research/MASTER_NOTES.md` §4).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MasterSettings {
    /// The TOC's output file; empty: the TOC goes into the document.
    pub toc_file: String,
    /// The index's output file.
    pub index_file: String,
    pub flags: u16,
    /// The master document's files in order, as stored: Ami Pro keeps full,
    /// lower-cased DOS paths (`l:\out\md2\ma.sam`). A document is a master
    /// when this is not empty.
    pub files: Vec<String>,
}

impl MasterSettings {
    /// Include alphabetical separators.
    pub const SEPARATORS: u16 = 1;

    /// Reads the section's values. The file list is taken only in the form
    /// Ami Pro writes it: the byte count is the files' lengths plus one each
    /// and the count their number (or both 0); any other list is not Ami
    /// Pro's and the section stays as read.
    pub fn from_values(values: &[String]) -> Option<Self> {
        if values.len() < 3 {
            return None;
        }
        let number =
            |index: usize| -> Option<usize> { values.get(index)?.trim().parse::<usize>().ok() };
        let files = match (values.len(), number(3), number(4)) {
            (3, _, _) => Vec::new(),
            (_, Some(bytes), Some(count)) => {
                let files = values[5..].to_vec();
                if files.len() != count || bytes != Self::byte_count_of(&files) {
                    return None;
                }
                files
            }
            _ => return None,
        };
        Some(Self {
            toc_file: unescape(&values[0])?,
            index_file: unescape(&values[1])?,
            flags: values[2].trim().parse::<i32>().ok()? as u16,
            files,
        })
    }

    pub fn values(&self) -> Vec<String> {
        let mut values = vec![
            escape(&self.toc_file),
            escape(&self.index_file),
            self.flags.to_string(),
            self.byte_count().to_string(),
            self.files.len().to_string(),
        ];
        values.extend(self.files.iter().cloned());
        values
    }

    /// The list's byte count as Ami Pro stores it: each path's length and
    /// its terminating NUL (MD2: three 17-character paths make 54).
    pub fn byte_count(&self) -> usize {
        Self::byte_count_of(&self.files)
    }

    fn byte_count_of(files: &[String]) -> usize {
        if files.is_empty() {
            return 0;
        }
        files.iter().map(|file| file.len() + 1).sum()
    }

    pub fn separators(&self) -> bool {
        self.flags & Self::SEPARATORS != 0
    }

    /// Whether the document is a master document: it lists files.
    pub fn is_master(&self) -> bool {
        !self.files.is_empty()
    }
}

/// One counter `[sequence]` carries: a Seq field's or a numbered style's
/// name and its value.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Counter {
    pub value: u32,
    pub name: String,
}

/// `[sequence]`: the numbers a member of a master document continues from,
/// which a master run gives it (`1660:054d`, written by `1660:0748`, read
/// by `1660:05fb`; `docs/research/MASTER_NOTES.md` §4; VM MD2, MD3).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Sequence {
    /// The page number shown on the last page of the members before; this
    /// member's shown page numbers add it.
    pub page_base: u32,
    /// The footnotes in the members before; this member's footnotes start
    /// after them.
    pub footnotes: u32,
    /// A word Ami Pro keeps and never changes (always 0).
    pub reserved: u32,
    /// Seq power fields: the highest value of each name before (names
    /// compared without case).
    pub seq: Vec<Counter>,
    /// Numbered styles: each style's counter at the end of the member
    /// before (names compared with case).
    pub styles: Vec<Counter>,
}

impl Sequence {
    pub fn from_values(values: &[String]) -> Option<Self> {
        let number = |index: usize| -> Option<u32> { values.get(index)?.trim().parse().ok() };
        let seq_count = number(3)? as usize;
        let style_count = number(4)? as usize;
        if values.len() != 5 + 2 * (seq_count + style_count) {
            return None;
        }
        let mut counters = Vec::with_capacity(seq_count + style_count);
        for index in 0..seq_count + style_count {
            counters.push(Counter {
                value: number(5 + 2 * index)?,
                name: values[6 + 2 * index].clone(),
            });
        }
        let styles = counters.split_off(seq_count);
        Some(Self {
            page_base: number(0)?,
            footnotes: number(1)?,
            reserved: number(2)?,
            seq: counters,
            styles,
        })
    }

    pub fn values(&self) -> Vec<String> {
        let mut values = vec![
            self.page_base.to_string(),
            self.footnotes.to_string(),
            self.reserved.to_string(),
            self.seq.len().to_string(),
            self.styles.len().to_string(),
        ];
        for counter in self.seq.iter().chain(&self.styles) {
            values.push(counter.value.to_string());
            values.push(counter.name.clone());
        }
        values
    }

    /// A Seq field's carried value (names without case).
    pub fn seq_value(&self, name: &str) -> Option<u32> {
        self.seq
            .iter()
            .find(|counter| counter.name.eq_ignore_ascii_case(name))
            .map(|counter| counter.value)
    }

    /// A numbered style's carried counter (names with case).
    pub fn style_value(&self, name: &str) -> Option<u32> {
        self.styles
            .iter()
            .find(|counter| counter.name == name)
            .map(|counter| counter.value)
    }
}

/// The document's TOC and index settings, and its place in a master
/// document.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContentsSettings {
    pub toc: Option<TocSettings>,
    pub master: Option<MasterSettings>,
    /// The numbers it continues from as a member of a master document.
    pub sequence: Option<Sequence>,
}

/// An index entry: the code of a power field of kind 8.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IndexEntry {
    pub primary: String,
    pub secondary: String,
    /// Text written after the pages (See, See also), as typed.
    pub other: String,
    /// The entry gets the page it is on (`#`).
    pub page_number: bool,
}

impl IndexEntry {
    /// Reads an index entry's code as Ami Pro's generator does
    /// (`1650:038e`): the keyword, the first quoted string the primary, a
    /// `#` anywhere outside quotes the page number, the next quoted
    /// strings the secondary and Other. Nothing inside quotes is escaped.
    pub fn parse(code: &str) -> Option<Self> {
        let rest = code.trim_start();
        let keyword = rest.get(..5)?;
        if !keyword.eq_ignore_ascii_case("index") {
            return None;
        }
        let mut entry = Self::default();
        let mut strings = Vec::new();
        let mut rest = &rest[5..];
        while let Some(at) = rest.find(['"', '#']) {
            if rest[at..].starts_with('#') {
                entry.page_number = true;
                rest = &rest[at + 1..];
                continue;
            }
            let after = &rest[at + 1..];
            let end = after.find('"').unwrap_or(after.len());
            strings.push(after[..end].to_string());
            rest = after.get(end + 1..).unwrap_or("");
        }
        let mut strings = strings.into_iter();
        entry.primary = strings.next()?;
        entry.secondary = strings.next().unwrap_or_default();
        entry.other = strings.next().unwrap_or_default();
        Some(entry)
    }

    /// The code Mark Index Entry writes (`1350:0695`): the primary, `#`
    /// for the page number, then the secondary and Other, cut after the
    /// last that is not empty.
    pub fn code(&self) -> String {
        let mut code = format!("Index \"{}\"", self.primary);
        if self.page_number {
            code.push_str(" #");
        }
        if !self.secondary.is_empty() || !self.other.is_empty() {
            code.push_str(&format!(" \"{}\"", self.secondary));
        }
        if !self.other.is_empty() {
            code.push_str(&format!(" \"{}\"", self.other));
        }
        code
    }

    /// The primary, secondary, and Other as the index uses them: trailing
    /// spaces dropped (`1650:038e`).
    pub fn trimmed(&self) -> Self {
        Self {
            primary: self.primary.trim_end().to_string(),
            secondary: self.secondary.trim_end().to_string(),
            other: self.other.trim_end().to_string(),
            page_number: self.page_number,
        }
    }
}

/// A TOC entry: the code of a power field of kind 11.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TocMark {
    /// 1 to 9.
    pub level: u8,
    pub text: String,
}

impl TocMark {
    /// Reads `TOC n "text"` as Ami Pro does (`1630:1f0e`): the level, then
    /// the rest, its quotes removed when it starts and ends with one.
    pub fn parse(code: &str) -> Option<Self> {
        let rest = code.trim_start();
        if !rest.get(..3)?.eq_ignore_ascii_case("toc") {
            return None;
        }
        let rest = rest[3..].trim_start();
        let digits = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        let level: u8 = rest[..digits].parse().ok()?;
        if !(1..=TOC_LEVELS as u8).contains(&level) {
            return None;
        }
        let mut text = &rest[digits..];
        text = text.strip_prefix(' ').unwrap_or(text);
        let text = match text.strip_prefix('"') {
            Some(inner) if inner.ends_with('"') => &inner[..inner.len() - 1],
            _ => text,
        };
        Some(Self {
            level,
            text: text.to_string(),
        })
    }

    /// The code TOC Entry writes (`1440:019b`).
    pub fn code(&self) -> String {
        format!("TOC {} \"{}\"", self.level, self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn toc_settings_read_and_write_as_ami_pro_writes_them() {
        // TI1: Title and Subhead, page numbers right aligned with dots.
        let values = strings(&["Title", "Subhead", "", ".", ".", "", "15"]);
        let toc = TocSettings::from_values(&values).unwrap();
        assert_eq!(toc.values(), values);
        let levels = toc.levels();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[1].style, "Subhead");
        assert!(levels[1].right_align && levels[1].page_number);
        assert_eq!(levels[1].leader, Leader::Dots);
        assert_eq!(TocSettings::from_levels(&levels).values(), values);
        // Older files' extra bit and a fourth level.
        let values = strings(&["A", "B", "C", ".", ".", ".", "16447", "D", ", ", "1"]);
        let toc = TocSettings::from_values(&values).unwrap();
        assert_eq!(toc.values(), values);
        assert_eq!(toc.levels()[3].leader, Leader::Comma);
        assert!(TocSettings::from_values(&strings(&["A"])).is_none());
    }

    #[test]
    fn a_master_list_is_read_and_written_as_ami_pro_writes_it_md2() {
        let values = strings(&[
            "L:\\OUT\\MD2\\MTOC.SAM",
            "L:\\OUT\\MD2\\MIDX.SAM",
            "1",
            "54",
            "3",
            "l:\\out\\md2\\ma.sam",
            "l:\\out\\md2\\mb.sam",
            "l:\\out\\md2\\mc.sam",
        ]);
        let master = MasterSettings::from_values(&values).unwrap();
        assert!(master.is_master());
        assert_eq!(master.files.len(), 3);
        assert_eq!(master.byte_count(), 54);
        assert_eq!(master.values(), values);
        // A list not in Ami Pro's form stays as read.
        let mut wrong = values.clone();
        wrong[3] = "53".into();
        assert!(MasterSettings::from_values(&wrong).is_none());
        // Three values: an older form without the list.
        let short = MasterSettings::from_values(&strings(&["", "", "0"])).unwrap();
        assert!(!short.is_master());
    }

    #[test]
    fn a_sequence_is_read_and_written_as_ami_pro_writes_it_md2() {
        let values = strings(&[
            "2",
            "2",
            "0",
            "0",
            "4",
            "0",
            "Bullet",
            "0",
            "Bullet 1",
            "1",
            "Number List",
            "0",
            "Footnote",
        ]);
        let sequence = Sequence::from_values(&values).unwrap();
        assert_eq!(sequence.page_base, 2);
        assert_eq!(sequence.footnotes, 2);
        assert_eq!(sequence.style_value("Number List"), Some(1));
        assert_eq!(sequence.style_value("number list"), None);
        assert_eq!(sequence.values(), values);
        let with_seq = strings(&["4", "0", "0", "1", "0", "7", "Fig"]);
        let sequence = Sequence::from_values(&with_seq).unwrap();
        assert_eq!(sequence.seq_value("FIG"), Some(7));
        assert_eq!(sequence.values(), with_seq);
        assert!(Sequence::from_values(&strings(&["1", "2", "0", "0", "1"])).is_none());
    }

    #[test]
    fn master_settings_keep_the_files_and_options() {
        let values = strings(&["", "L:\\OUT\\TI1IDX.SAM", "1", "0", "0"]);
        let master = MasterSettings::from_values(&values).unwrap();
        assert!(master.separators());
        assert_eq!(master.index_file, "L:\\OUT\\TI1IDX.SAM");
        assert_eq!(master.values(), values);
    }

    #[test]
    fn index_entries_read_and_write_as_ami_pro_writes_them() {
        for (code, primary, secondary, other, page) in [
            ("Index \"Printer\" #", "Printer", "", "", true),
            (
                "Index \"Printer\" # \"setup\"",
                "Printer",
                "setup",
                "",
                true,
            ),
            (
                "Index \"Zebra\" # \"\" \" See Animals\"",
                "Zebra",
                "",
                " See Animals",
                true,
            ),
            ("Index \"No page\"", "No page", "", "", false),
            ("Index \"p\" \"s\"", "p", "s", "", false),
        ] {
            let entry = IndexEntry::parse(code).unwrap();
            assert_eq!(
                (
                    entry.primary.as_str(),
                    entry.secondary.as_str(),
                    entry.other.as_str(),
                    entry.page_number
                ),
                (primary, secondary, other, page),
                "{code}"
            );
            assert_eq!(entry.code(), code);
        }
        // The corpus's trailing `#` and a lower-case keyword.
        let entry = IndexEntry::parse("index \"A\" # \"B\" #").unwrap();
        assert_eq!((entry.secondary.as_str(), entry.page_number), ("B", true));
        assert!(IndexEntry::parse("TOC 1 \"x\"").is_none());
    }

    #[test]
    fn toc_marks_read_their_level_and_text() {
        let mark = TocMark::parse("TOC 3 \"Marked entry\"").unwrap();
        assert_eq!((mark.level, mark.text.as_str()), (3, "Marked entry"));
        assert_eq!(mark.code(), "TOC 3 \"Marked entry\"");
        // An unclosed quote keeps the rest as it is.
        let mark = TocMark::parse("TOC 4 \"Half").unwrap();
        assert_eq!(mark.text, "\"Half");
        assert!(TocMark::parse("TOC 0 \"x\"").is_none());
        assert!(TocMark::parse("Index \"x\"").is_none());
    }
}
