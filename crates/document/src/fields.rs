//! Ami Pro's dates, times, and Doc Info entries, as its line builder works
//! them out (`docs/research/FIELDS_NOTES.md`, "Dates and times" and "Doc
//! Info"): the date formatter (`1308:0619`), the time formatter
//! (`1280:021f`), and its time base, the wall clock read as US Pacific time
//! by its C runtime (`1020:06bc`, `1020:1798`).

/// Where a date comes from: Ami Pro's Insert Date/Time choices but Today's
/// date, which it inserts as text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DateSource {
    /// System date, `<:Dx>`: today.
    Today,
    /// Date of last revision, `<:rx>`: the last save.
    LastSaved,
    /// Date created, `<:kx>`.
    Created,
    /// System time, `<:b…>`: now.
    Time,
}

/// A date or the time, worked out each time its line is laid out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DateField {
    pub source: DateSource,
    /// The byte after the control's letter: a date's format letter (`a` to
    /// `l`, a capital for capitals; any other shows as `a`), or for the time
    /// the list position Ami Pro stored there, which it ignores.
    pub format: u8,
}

/// A Doc Info entry put into the text (Insert Doc Info Field, `<:d…>`),
/// worked out each time its line is laid out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocInfoField {
    /// Ami Pro's code for the entry ([`DocInfoEntry::from_code`]); codes
    /// with no entry show nothing and are kept.
    pub code: u16,
    /// The date format letter, which the two date entries use.
    pub format: u8,
}

/// Ami Pro's Doc Info entries by their codes (`1338:039d`, `1308:035d`; VM
/// fixture FD2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocInfoEntry {
    FileName,
    Path,
    StyleSheet,
    DateCreated,
    DateLastRevised,
    TotalRevisions,
    /// The description: Ami Pro's evaluator has the code, but Insert Doc
    /// Info Field inserts a power field for it.
    Description,
    /// One of the eight user fields, 0 to 7.
    Field(u8),
    TimeCreated,
    TimeLastRevised,
    EditingTime,
    Pages,
    Words,
    Characters,
    Size,
}

impl DocInfoEntry {
    /// The entries in Ami Pro's Insert Doc Info Field order, with the
    /// description, which it inserts as a power field, left out.
    pub const LISTED: [Self; 21] = [
        Self::FileName,
        Self::Path,
        Self::StyleSheet,
        Self::DateCreated,
        Self::TimeCreated,
        Self::DateLastRevised,
        Self::TimeLastRevised,
        Self::TotalRevisions,
        Self::EditingTime,
        Self::Pages,
        Self::Words,
        Self::Characters,
        Self::Size,
        Self::Field(0),
        Self::Field(1),
        Self::Field(2),
        Self::Field(3),
        Self::Field(4),
        Self::Field(5),
        Self::Field(6),
        Self::Field(7),
    ];

    pub fn from_code(code: u16) -> Option<Self> {
        Some(match code {
            1 => Self::FileName,
            2 => Self::Path,
            3 => Self::StyleSheet,
            4 => Self::DateCreated,
            5 => Self::DateLastRevised,
            6 => Self::TotalRevisions,
            7 => Self::Description,
            8..=15 => Self::Field((code - 8) as u8),
            0x14 => Self::TimeCreated,
            0x15 => Self::TimeLastRevised,
            0x16 => Self::EditingTime,
            0x17 => Self::Pages,
            0x18 => Self::Words,
            0x19 => Self::Characters,
            0x1A => Self::Size,
            _ => return None,
        })
    }

    pub fn code(self) -> u16 {
        match self {
            Self::FileName => 1,
            Self::Path => 2,
            Self::StyleSheet => 3,
            Self::DateCreated => 4,
            Self::DateLastRevised => 5,
            Self::TotalRevisions => 6,
            Self::Description => 7,
            Self::Field(index) => 8 + u16::from(index.min(7)),
            Self::TimeCreated => 0x14,
            Self::TimeLastRevised => 0x15,
            Self::EditingTime => 0x16,
            Self::Pages => 0x17,
            Self::Words => 0x18,
            Self::Characters => 0x19,
            Self::Size => 0x1A,
        }
    }
}

/// Ami Pro's Doc Info (`[desc]` and `[fldnames]`, `11c8:342b`,
/// `1058:2732`; `docs/research/FIELDS_NOTES.md`, "Doc Info").
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocumentInfo {
    /// Up to 119 characters in Ami Pro; longer ones are kept.
    pub description: String,
    pub keywords: String,
    /// The eight user fields' values (31 characters each in Ami Pro).
    pub fields: [String; 8],
    /// Their names as stored; an empty one is the default, FieldN.
    pub field_names: [String; 8],
    /// When the document was created and last saved, in Ami Pro's time
    /// base ([`WallClock::from_ami_pro`]); `None` until it is.
    pub created: Option<u32>,
    pub saved: Option<u32>,
    /// Total revisions as Ami Pro holds them: one more each time a document
    /// is opened (`1058:2732`), 1 for a new one.
    pub revisions: u32,
    /// Total editing time in minutes, as of the last save.
    pub editing_minutes: u32,
    pub statistics: Statistics,
    /// The user who locked the document for annotations, when locked.
    pub annotation_lock: String,
    /// Bit 0: Run frame macros.
    pub flags: u32,
}

/// Doc Info's statistics as of the last count (Ami Pro's Update).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Statistics {
    pub pages: u32,
    pub words: u32,
    pub characters: u32,
    /// The saved file's size in K (bytes / 1000).
    pub size: u32,
    /// The same with the style sheet and linked files, which Ami Pro
    /// writes and shows nowhere.
    pub size_with_links: u32,
}

impl DocumentInfo {
    /// A new document's Doc Info as Ami Pro makes it (`1058:0120`):
    /// created now, one revision, Run frame macros on.
    pub fn new(now: u32) -> Self {
        Self {
            created: Some(now),
            revisions: 1,
            flags: 1,
            ..Self::default()
        }
    }

    /// Field `index`'s name, or its default, FieldN (`1128:1e4d`).
    pub fn field_name(&self, index: usize) -> String {
        match self.field_names.get(index) {
            Some(name) if !name.is_empty() => name.clone(),
            _ => format!("Field{}", index + 1),
        }
    }

    pub fn runs_frame_macros(&self) -> bool {
        self.flags & 1 != 0
    }
}

/// What dates and Doc Info entries show besides the document: the clock,
/// the file, and this session's saves (Ami Pro's line builder reads them
/// each time it builds a line, `10d0:2d0e`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldContext {
    /// The local wall clock, to the minute: System date and System time.
    pub now: WallClock,
    pub settings: DateSettings,
    /// The file's name and folder; empty for a document never saved.
    pub file_name: String,
    pub directory: String,
    /// This session's last save, in Ami Pro's time base, which Date of
    /// last revision shows once it is made.
    pub saved: Option<u32>,
    /// Minutes of editing since the document was opened.
    pub session_minutes: u32,
    /// The page count of the last layout, which Number of Pages shows.
    pub pages: u32,
}

impl Default for FieldContext {
    fn default() -> Self {
        Self {
            now: WallClock::from_unix(0),
            settings: DateSettings::US,
            file_name: String::new(),
            directory: String::new(),
            saved: None,
            session_minutes: 0,
            pages: 0,
        }
    }
}

impl FieldContext {
    /// A date or the time as Ami Pro shows it (`1308:02cc` to `032e`): the
    /// last revision is the creation until the document is saved.
    pub fn date_text(&self, field: &DateField, info: &DocumentInfo) -> String {
        let clock = match field.source {
            DateSource::Today => self.now,
            DateSource::Time => return format_time(&self.now, &self.settings),
            DateSource::LastSaved => self.last_saved(info),
            DateSource::Created => self.created(info),
        };
        format_date(&clock, field.format, &self.settings)
    }

    /// A Doc Info entry as Ami Pro shows it (`1308:035d`; VM fixture FD2):
    /// the words, characters, and size as of the last count.
    pub fn doc_info_text(&self, field: &DocInfoField, document: &crate::Document) -> String {
        let info = document.info();
        let Some(entry) = DocInfoEntry::from_code(field.code) else {
            return String::new();
        };
        let settings = &self.settings;
        match entry {
            DocInfoEntry::FileName if self.file_name.is_empty() => "(Untitled)".into(),
            DocInfoEntry::FileName => self.file_name.clone(),
            DocInfoEntry::Path => self.directory.clone(),
            DocInfoEntry::StyleSheet => document
                .style_sheet()
                .map_or_else(|| "None".to_owned(), |link| link.name.clone()),
            DocInfoEntry::DateCreated => format_date(&self.created(info), field.format, settings),
            DocInfoEntry::TimeCreated => format_time(&self.created(info), settings),
            DocInfoEntry::DateLastRevised => {
                format_date(&self.last_saved(info), field.format, settings)
            }
            DocInfoEntry::TimeLastRevised => format_time(&self.last_saved(info), settings),
            DocInfoEntry::TotalRevisions => info.revisions.to_string(),
            DocInfoEntry::Description => info.description.clone(),
            DocInfoEntry::Field(index) => info.fields[usize::from(index.min(7))].clone(),
            DocInfoEntry::EditingTime => info
                .editing_minutes
                .saturating_add(self.session_minutes)
                .to_string(),
            DocInfoEntry::Pages if self.pages > 0 => self.pages.to_string(),
            DocInfoEntry::Pages => info.statistics.pages.to_string(),
            DocInfoEntry::Words => info.statistics.words.to_string(),
            DocInfoEntry::Characters => info.statistics.characters.to_string(),
            DocInfoEntry::Size => info.statistics.size.to_string(),
        }
    }

    fn created(&self, info: &DocumentInfo) -> WallClock {
        info.created.map_or(self.now, WallClock::from_ami_pro)
    }

    fn last_saved(&self, info: &DocumentInfo) -> WallClock {
        match self.saved.or(info.saved) {
            Some(saved) => WallClock::from_ami_pro(saved),
            None => self.created(info),
        }
    }
}

/// Ami Pro's count of words and characters (`12a0:0e4f`, `12a0:0f9a`;
/// VM fixture FD2): every text stream; characters are the text's, spaces
/// and tabs included, objects (dates, fields, anchors) and paragraph ends
/// not; words end at a space, a tab, a no-break space, or a paragraph's
/// end, and an object does not split one.
pub fn count_words_and_characters(document: &crate::Document) -> (u32, u32) {
    let (mut words, mut characters) = (0_u32, 0_u32);
    // Ami Pro's count skips notes (`FIELDS_NOTES.md`).
    let notes: std::collections::BTreeSet<crate::FlowId> = document
        .flows()
        .flat_map(|(_, flow)| flow.paragraphs.iter())
        .flat_map(|paragraph| &paragraph.objects)
        .filter_map(|object| match object {
            crate::InlineObject::Note { flow, .. } => Some(*flow),
            _ => None,
        })
        .collect();
    for (id, flow) in document.flows() {
        if notes.contains(&id) {
            continue;
        }
        for paragraph in &flow.paragraphs {
            let mut in_word = false;
            for character in paragraph.text.chars() {
                match character {
                    crate::OBJECT_REPLACEMENT_CHARACTER => {}
                    ' ' | '\t' | '\u{A0}' => {
                        characters = characters.saturating_add(1);
                        in_word = false;
                    }
                    _ => {
                        characters = characters.saturating_add(1);
                        if !in_word {
                            words = words.saturating_add(1);
                            in_word = true;
                        }
                    }
                }
            }
        }
    }
    (words, characters)
}

/// A wall-clock time: a date and the time of day, with no time zone.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WallClock {
    pub year: i32,
    /// 1 to 12.
    pub month: u8,
    /// 1 to 31.
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl WallClock {
    /// The day of the week, 0 for Sunday.
    pub fn weekday(&self) -> u8 {
        // 1 January 1970 was a Thursday.
        (days_from_civil(self.year, self.month, self.day) + 4).rem_euclid(7) as u8
    }

    /// The clock `seconds` after midnight on 1 January 1970.
    pub fn from_unix(seconds: i64) -> Self {
        let days = seconds.div_euclid(86_400);
        let time = seconds.rem_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Self {
            year,
            month,
            day,
            hour: (time / 3600) as u8,
            minute: (time / 60 % 60) as u8,
            second: (time % 60) as u8,
        }
    }

    /// Seconds from midnight on 1 January 1970 to this clock, both read in
    /// one time zone.
    pub fn to_unix(&self) -> i64 {
        days_from_civil(self.year, self.month, self.day) * 86_400
            + i64::from(self.hour) * 3600
            + i64::from(self.minute) * 60
            + i64::from(self.second)
    }

    /// The wall clock Ami Pro shows for `seconds` it stored (`[desc]`'s
    /// save and creation times, CreateDate, EditDate): its runtime reads
    /// them as US Pacific time, 8 hours behind, 7 in its summer time
    /// (`localtime`, `1020:1798`; VM fixtures FD2, FD5).
    pub fn from_ami_pro(seconds: u32) -> Self {
        let standard = Self::from_unix(i64::from(seconds) - PACIFIC);
        if in_summer_time(&standard) {
            Self::from_unix(i64::from(seconds) - PACIFIC + 3600)
        } else {
            standard
        }
    }

    /// The seconds Ami Pro stores for this wall clock (`time`,
    /// `1020:06bc`): the reverse of [`Self::from_ami_pro`]. Clocks Ami Pro
    /// cannot store give 0 or the last second it can.
    pub fn to_ami_pro(&self) -> u32 {
        let mut seconds = self.to_unix() + PACIFIC;
        if in_summer_time(self) {
            seconds -= 3600;
        }
        seconds.clamp(0, i64::from(u32::MAX)) as u32
    }
}

/// Ami Pro's runtime's time zone when `TZ` is unset: US Pacific, in seconds.
const PACIFIC: i64 = 8 * 3600;

/// Whether `clock`, in standard time, is in the summer time of Ami Pro's
/// runtime (`_isindst`): from 2:00 on the first Sunday in April (the last
/// Sunday in April before 1987) to 1:00 standard time on the last Sunday in
/// October.
fn in_summer_time(clock: &WallClock) -> bool {
    if !(4..=10).contains(&clock.month) {
        return false;
    }
    if (5..=9).contains(&clock.month) {
        return true;
    }
    let (sunday, hour) = if clock.month == 4 {
        let sunday = if clock.year >= 1987 {
            first_sunday(clock.year, 4)
        } else {
            last_sunday(clock.year, 4)
        };
        (sunday, 2)
    } else {
        (last_sunday(clock.year, 10), 1)
    };
    let after = (clock.day, clock.hour) >= (sunday, hour);
    if clock.month == 4 { after } else { !after }
}

fn first_sunday(year: i32, month: u8) -> u8 {
    let first = WallClock {
        year,
        month,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
    };
    1 + (7 - first.weekday()) % 7
}

fn last_sunday(year: i32, month: u8) -> u8 {
    let last = days_in_month(year, month);
    let clock = WallClock {
        year,
        month,
        day: last,
        hour: 0,
        minute: 0,
        second: 0,
    };
    last - clock.weekday()
}

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 31,
    }
}

/// Days from 1 January 1970 to a date (Howard Hinnant's algorithm).
fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i32, u8, u8) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_index + 2) / 5 + 1) as u8;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    } as u8;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year as i32, month, day)
}

/// The order of a numeric date (WIN.INI `[intl] iDate`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DateOrder {
    MonthDayYear,
    DayMonthYear,
    YearMonthDay,
}

/// The Windows settings Ami Pro's formatters read (WIN.INI `[intl]`,
/// `1018:0275`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DateSettings {
    /// `iDate`.
    pub order: DateOrder,
    /// `sDate`.
    pub separator: &'static str,
    /// `iLzero`, the number format's leading zero, which pads every part of
    /// a numeric date to two digits.
    pub leading_zero: bool,
    /// `iTime`.
    pub twenty_four_hour: bool,
    /// `sTime`.
    pub time_separator: &'static str,
    /// `s1159` and `s2359`.
    pub morning: &'static str,
    pub afternoon: &'static str,
}

impl DateSettings {
    /// Ami Pro on US Windows, as the reference VM has it (decision FL5):
    /// 03/02/05, ` 9:05 AM`.
    pub const US: Self = Self {
        order: DateOrder::MonthDayYear,
        separator: "/",
        leading_zero: true,
        twenty_four_hour: false,
        time_separator: ":",
        morning: "AM",
        afternoon: "PM",
    };
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Insert Date/Time's 21 formats in its list order (`1308:0df6`).
pub const DATE_FORMATS: [u8; 21] = *b"abBcCdDeEfFghiIjJkKlL";

/// `clock` in Ami Pro's date format `letter` (`1308:0619`; VM fixtures FD1,
/// FD4): the layout by the letter with 0x20 set, then capitals when the
/// letter is a capital. Unknown letters show as `a`.
pub fn format_date(clock: &WallClock, letter: u8, settings: &DateSettings) -> String {
    let month = MONTHS[usize::from(clock.month.clamp(1, 12) - 1)];
    let weekday = WEEKDAYS[usize::from(clock.weekday())];
    let (day, year) = (clock.day, clock.year);
    let day_first = settings.order == DateOrder::DayMonthYear;
    let text = match letter | 0x20 {
        b'b' => format!("{month} {day}, {year}"),
        b'c' => format!("{day} {month} {year}"),
        b'd' => format!("{weekday}, {month} {day}, {year}"),
        b'e' if day_first => format!("{day} {month}"),
        b'e' => format!("{month} {day}"),
        b'f' if day_first => format!("{weekday} {day} {month}"),
        b'f' => format!("{weekday} {month} {day}"),
        b'g' => {
            let parts = if day_first {
                [clock.day, clock.month]
            } else {
                [clock.month, clock.day]
            };
            numeric(&parts.map(i32::from), settings)
        }
        b'h' => numeric_date(clock, year, settings),
        b'i' => format!("{day}. {month}"),
        b'j' => format!("{day}. {month} {year}"),
        b'k' => format!("{year} {month} {day}"),
        b'l' => format!("{month}, {year}"),
        _ => numeric_date(clock, year.rem_euclid(100), settings),
    };
    if letter.is_ascii_uppercase() {
        text.to_ascii_uppercase()
    } else {
        text
    }
}

fn numeric_date(clock: &WallClock, year: i32, settings: &DateSettings) -> String {
    let (month, day) = (i32::from(clock.month), i32::from(clock.day));
    let parts = match settings.order {
        DateOrder::MonthDayYear => [month, day, year],
        DateOrder::DayMonthYear => [day, month, year],
        DateOrder::YearMonthDay => [year, month, day],
    };
    numeric(&parts, settings)
}

fn numeric(parts: &[i32], settings: &DateSettings) -> String {
    parts
        .iter()
        .map(|part| {
            if settings.leading_zero {
                format!("{part:02}")
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(settings.separator)
}

/// The time as Ami Pro's System time and Doc Info times show it
/// (`1280:021f`; VM fixtures FD1, FD5): the hour space-padded to two places,
/// two-digit minutes, a space, and the morning or afternoon text.
pub fn format_time(clock: &WallClock, settings: &DateSettings) -> String {
    let hour = if settings.twenty_four_hour {
        clock.hour
    } else {
        match clock.hour % 12 {
            0 => 12,
            hour => hour,
        }
    };
    let suffix = if clock.hour < 12 {
        settings.morning
    } else {
        settings.afternoon
    };
    format!(
        "{hour:2}{}{:02} {suffix}",
        settings.time_separator, clock.minute
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(year: i32, month: u8, day: u8, hour: u8, minute: u8) -> WallClock {
        WallClock {
            year,
            month,
            day,
            hour,
            minute,
            second: 0,
        }
    }

    #[test]
    fn every_format_is_as_ami_pro_shows_it_fd4() {
        // VM fixture FD4: 2 March 2005, a Wednesday, the guest's US settings.
        let date = clock(2005, 3, 2, 9, 5);
        let shown: Vec<String> = DATE_FORMATS
            .iter()
            .map(|letter| format_date(&date, *letter, &DateSettings::US))
            .collect();
        assert_eq!(
            shown,
            [
                "03/02/05",
                "March 2, 2005",
                "MARCH 2, 2005",
                "2 March 2005",
                "2 MARCH 2005",
                "Wednesday, March 2, 2005",
                "WEDNESDAY, MARCH 2, 2005",
                "March 2",
                "MARCH 2",
                "Wednesday March 2",
                "WEDNESDAY MARCH 2",
                "03/02",
                "03/02/2005",
                "2. March",
                "2. MARCH",
                "2. March 2005",
                "2. MARCH 2005",
                "2005 March 2",
                "2005 MARCH 2",
                "March, 2005",
                "MARCH, 2005",
            ]
        );
        // FD1: 20 October 2026, a Tuesday.
        let date = clock(2026, 10, 20, 11, 25);
        assert_eq!(format_date(&date, b'a', &DateSettings::US), "10/20/26");
        assert_eq!(
            format_date(&date, b'd', &DateSettings::US),
            "Tuesday, October 20, 2026"
        );
        // Unknown letters show as `a`.
        assert_eq!(format_date(&date, b'z', &DateSettings::US), "10/20/26");
        assert_eq!(format_date(&date, 1, &DateSettings::US), "10/20/26");
    }

    #[test]
    fn numeric_dates_follow_the_windows_settings_as_the_decompile_reads_them() {
        let date = clock(2005, 3, 2, 0, 0);
        let plain = DateSettings {
            leading_zero: false,
            ..DateSettings::US
        };
        // Without the number format's leading zero, nothing is padded and
        // the two-digit year is the year modulo 100.
        assert_eq!(format_date(&date, b'a', &plain), "3/2/5");
        assert_eq!(format_date(&date, b'h', &plain), "3/2/2005");
        let european = DateSettings {
            order: DateOrder::DayMonthYear,
            separator: ".",
            ..DateSettings::US
        };
        assert_eq!(format_date(&date, b'a', &european), "02.03.05");
        assert_eq!(format_date(&date, b'g', &european), "02.03");
        assert_eq!(format_date(&date, b'e', &european), "2 March");
        assert_eq!(format_date(&date, b'f', &european), "Wednesday 2 March");
        // Year first puts the year first in a and h, but g stays month/day.
        let year_first = DateSettings {
            order: DateOrder::YearMonthDay,
            ..DateSettings::US
        };
        assert_eq!(format_date(&date, b'a', &year_first), "05/03/02");
        assert_eq!(format_date(&date, b'g', &year_first), "03/02");
    }

    #[test]
    fn the_time_pads_the_hour_with_a_space_fd5() {
        let us = DateSettings::US;
        assert_eq!(format_time(&clock(2026, 10, 20, 9, 5), &us), " 9:05 AM");
        assert_eq!(format_time(&clock(2026, 10, 20, 11, 25), &us), "11:25 AM");
        assert_eq!(format_time(&clock(2026, 10, 20, 0, 30), &us), "12:30 AM");
        assert_eq!(format_time(&clock(2026, 10, 20, 12, 30), &us), "12:30 PM");
        assert_eq!(format_time(&clock(2026, 10, 20, 23, 59), &us), "11:59 PM");
        let day = DateSettings {
            twenty_four_hour: true,
            morning: "",
            afternoon: "",
            ..us
        };
        assert_eq!(format_time(&clock(2026, 10, 20, 13, 5), &day), "13:05 ");
    }

    #[test]
    fn stored_times_are_the_wall_clock_read_as_pacific_time_fd2_fd5() {
        // FD2: created at 11:23:51 on 20 October 2026 (US summer time by
        // Ami Pro's runtime), stored as 18:23:51 UTC.
        let created = clock(2026, 10, 20, 11, 23);
        let created = WallClock {
            second: 51,
            ..created
        };
        assert_eq!(created.to_ami_pro(), 1_792_520_631);
        assert_eq!(WallClock::from_ami_pro(1_792_520_631), created);
        // FD5: 16:05 UTC in October and 17:05 UTC in January both show 9:05.
        assert_eq!(
            WallClock::from_ami_pro(1_792_512_300),
            clock(2026, 10, 20, 9, 5)
        );
        assert_eq!(
            WallClock::from_ami_pro(1_105_808_700),
            clock(2005, 1, 15, 9, 5)
        );
        assert_eq!(clock(2005, 1, 15, 9, 5).to_ami_pro(), 1_105_808_700);
    }

    #[test]
    fn summer_time_is_ami_pro_runtimes_old_us_rule() {
        // From 1987: the first Sunday in April to the last Sunday in October.
        // 2026: 5 April and 25 October (not the 2007 rule's March and
        // November).
        assert!(!in_summer_time(&clock(2026, 3, 20, 12, 0)));
        assert!(!in_summer_time(&clock(2026, 4, 5, 1, 59)));
        assert!(in_summer_time(&clock(2026, 4, 5, 2, 0)));
        assert!(in_summer_time(&clock(2026, 10, 25, 0, 59)));
        assert!(!in_summer_time(&clock(2026, 10, 25, 1, 0)));
        assert!(!in_summer_time(&clock(2026, 11, 1, 12, 0)));
        // Before 1987: the last Sunday in April (1986: 27 April).
        assert!(!in_summer_time(&clock(1986, 4, 6, 12, 0)));
        assert!(in_summer_time(&clock(1986, 4, 27, 3, 0)));
    }

    #[test]
    fn the_calendar_round_trips_and_knows_its_weekdays() {
        for seconds in [0_i64, 951_782_400, 1_792_520_631, -86_400, 4_102_444_800] {
            assert_eq!(WallClock::from_unix(seconds).to_unix(), seconds);
        }
        assert_eq!(clock(2005, 3, 2, 0, 0).weekday(), 3);
        assert_eq!(clock(2026, 10, 20, 0, 0).weekday(), 2);
        assert_eq!(clock(2000, 2, 29, 0, 0).to_unix(), 951_782_400);
    }

    #[test]
    fn dates_and_doc_info_entries_show_what_ami_pro_shows_fd1_fd2() {
        // FD2's Doc Info, shown at 11:30 on 20 October 2026 for FD1.SAM.
        let mut document = crate::Document::default();
        let info = document.info_mut();
        *info = DocumentInfo::new(1_792_520_631);
        info.saved = Some(1_792_520_772);
        info.description = "My description".into();
        info.fields[0] = "Alpha".into();
        info.revisions = 1;
        info.editing_minutes = 2;
        info.statistics.words = 6;
        info.statistics.characters = 23;
        info.statistics.size = 4;
        let context = FieldContext {
            now: clock(2026, 10, 20, 11, 30),
            file_name: "FD1.SAM".into(),
            directory: "L:\\OUT".into(),
            session_minutes: 3,
            pages: 1,
            ..FieldContext::default()
        };
        let date =
            |source, format| context.date_text(&DateField { source, format }, document.info());
        assert_eq!(date(DateSource::Today, b'b'), "October 20, 2026");
        assert_eq!(date(DateSource::Time, 1), "11:30 AM");
        assert_eq!(
            date(DateSource::LastSaved, b'd'),
            "Tuesday, October 20, 2026"
        );
        assert_eq!(date(DateSource::Created, b'c'), "20 October 2026");
        let shown: Vec<String> = DocInfoEntry::LISTED
            .iter()
            .map(|entry| {
                context.doc_info_text(
                    &DocInfoField {
                        code: entry.code(),
                        format: b'c',
                    },
                    &document,
                )
            })
            .collect();
        assert_eq!(
            shown[..15],
            [
                "FD1.SAM",
                "L:\\OUT",
                "None",
                "20 October 2026",
                "11:23 AM",
                "20 October 2026",
                "11:26 AM",
                "1",
                "5",
                "1",
                "6",
                "23",
                "4",
                "Alpha",
                "",
            ]
        );
        // Before a save: "(Untitled)", and the last revision is the creation.
        let untitled = FieldContext::default();
        let entry = |code| untitled.doc_info_text(&DocInfoField { code, format: b'a' }, &document);
        assert_eq!(entry(1), "(Untitled)");
        assert_eq!(entry(0x13), "");
        document.info_mut().saved = None;
        assert_eq!(
            untitled.date_text(
                &DateField {
                    source: DateSource::LastSaved,
                    format: b'a',
                },
                document.info(),
            ),
            "10/20/26"
        );
    }

    #[test]
    fn words_and_characters_are_counted_as_ami_pro_counts_them_fd2() {
        // FD2 at its Update: "T: 10/20/26" and four lines of a label and a
        // date field: 6 words, 23 characters (fields count nothing).
        let mut document = crate::Document::from_plain_text("T: 10/20/26\nS: x\nH: x\nR: x\nC: x");
        for index in 1..5 {
            let paragraph = document.paragraph_mut(index);
            paragraph.text = format!("{}: \u{FFFC}", &paragraph.text[..1]);
            paragraph.objects.push(crate::InlineObject::Date(DateField {
                source: DateSource::Today,
                format: b'b',
            }));
        }
        assert_eq!(count_words_and_characters(&document), (6, 23));
        // Tabs and no-break spaces end words; hyphens, hard returns, and
        // discretionary hyphens are inside them; an object joins its sides.
        let document = crate::Document::from_plain_text(
            "one two\tthree-four\u{A0}five six\u{2028}seven\u{AD}eight a\u{FFFC}b",
        );
        assert_eq!(count_words_and_characters(&document).0, 6);
    }

    #[test]
    fn doc_info_codes_are_ami_pros() {
        for entry in DocInfoEntry::LISTED {
            assert_eq!(DocInfoEntry::from_code(entry.code()), Some(entry));
        }
        assert_eq!(DocInfoEntry::from_code(7), Some(DocInfoEntry::Description));
        assert_eq!(DocInfoEntry::from_code(9), Some(DocInfoEntry::Field(1)));
        assert_eq!(DocInfoEntry::from_code(0), None);
        assert_eq!(DocInfoEntry::from_code(0x13), None);
    }
}
