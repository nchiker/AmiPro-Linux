//! What a merge document keeps about its merge (`docs/research/MERGE_NOTES.md`,
//! section 2): Ami Pro's `[recfile]` (the data file, the description file,
//! the last mode, As labels, and the label values) and the conditions,
//! which Ami Pro does not store and the native format does (decision MG6).

/// Ami Pro's `[recfile]`: written only when the document has a data file
/// (`11c8:1c7b`), read by `11a0:0418`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordFile {
    /// The data file's path as stored (Ami Pro's, often lower case).
    pub data_file: String,
    /// The description file's path, or `*` and a filter's type for an
    /// external database (`*dBase`); may be empty.
    pub description: String,
    /// The last mode and the options ([`record_flags`]).
    pub flags: u16,
    /// The Labels dialog's unit: 1 in., 2 cm., 3 picas, 4 pt. (VM twin
    /// AMLB and MGL1: 1 shows in., 0 shows none).
    pub unit: i32,
    /// Labels across and down.
    pub across: i32,
    pub down: i32,
    /// The first label's indent down and right, in twips.
    pub indent_down: i32,
    pub indent_right: i32,
}

/// `[recfile]`'s flag bits.
pub mod record_flags {
    /// Merge & print.
    pub const PRINT: u16 = 0x01;
    /// Merge, view & print.
    pub const VIEW: u16 = 0x02;
    /// Macro mode.
    pub const MACRO: u16 = 0x04;
    /// As labels: the label values mean something.
    pub const LABELS: u16 = 0x10;
}

impl Default for RecordFile {
    /// The values Ami Pro's reader takes when there are none (`11a0:0418`).
    fn default() -> Self {
        Self {
            data_file: String::new(),
            description: String::new(),
            flags: record_flags::PRINT,
            unit: 1,
            across: 3,
            down: 12,
            indent_down: 1440,
            indent_right: 1440,
        }
    }
}

impl RecordFile {
    /// A record file naming `data_file`, with Ami Pro's defaults.
    pub fn new(data_file: impl Into<String>) -> Self {
        Self {
            data_file: data_file.into(),
            ..Self::default()
        }
    }

    /// Reads `[recfile]`'s eight values; `None` when they are not in Ami
    /// Pro's form, so the section is kept as read. Values 5–8 are kept as
    /// read even when As labels is off (they are then what Ami Pro's memory
    /// held, and its writer writes them back).
    pub fn from_values(values: &[String]) -> Option<Self> {
        if values.len() != 8 {
            return None;
        }
        let number = |index: usize| values[index].trim().parse::<i32>().ok();
        Some(Self {
            data_file: values[0].clone(),
            description: values[1].clone(),
            flags: values[2].trim().parse::<u16>().ok()?,
            unit: number(3)?,
            across: number(4)?,
            down: number(5)?,
            indent_down: number(6)?,
            indent_right: number(7)?,
        })
    }

    /// The eight values as Ami Pro writes them.
    pub fn values(&self) -> Vec<String> {
        vec![
            self.data_file.clone(),
            self.description.clone(),
            self.flags.to_string(),
            self.unit.to_string(),
            self.across.to_string(),
            self.down.to_string(),
            self.indent_down.to_string(),
            self.indent_right.to_string(),
        ]
    }

    pub fn as_labels(&self) -> bool {
        self.flags & record_flags::LABELS != 0
    }
}

/// A condition's comparison (`13e8:072a`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Operator {
    #[default]
    Equal,
    Less,
    Greater,
    NotEqual,
    LessOrEqual,
    GreaterOrEqual,
}

impl Operator {
    pub const ALL: [Self; 6] = [
        Self::Equal,
        Self::Less,
        Self::Greater,
        Self::NotEqual,
        Self::LessOrEqual,
        Self::GreaterOrEqual,
    ];

    /// The operator as Ami Pro's Conditions dialog shows it.
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::Less => "<",
            Self::Greater => ">",
            Self::NotEqual => "!=",
            Self::LessOrEqual => "<=",
            Self::GreaterOrEqual => ">=",
        }
    }

    pub fn from_symbol(symbol: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operator| operator.symbol() == symbol)
    }

    /// Whether an empty value is allowed (only with `=` and `!=`).
    pub fn takes_empty(self) -> bool {
        matches!(self, Self::Equal | Self::NotEqual)
    }
}

/// How a condition joins the next one.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Join {
    #[default]
    And,
    Or,
}

/// One row of the Conditions dialog: If FIELD OPERATOR VALUE, then And or
/// Or to the next row.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Condition {
    pub field: String,
    pub operator: Operator,
    pub value: String,
    pub join: Join,
}

/// The document's merge settings.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MergeSettings {
    /// `[recfile]`, when the document has a data file.
    pub record_file: Option<RecordFile>,
    /// The conditions (native only; SAM cannot hold them).
    pub conditions: Vec<Condition>,
    /// With conditions: whether they select the records.
    pub with_conditions: bool,
}

impl MergeSettings {
    /// What the native format holds that SAM cannot: the conditions.
    pub fn has_conditions(&self) -> bool {
        !self.conditions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn record_files_read_and_write_ami_pros_eight_values() {
        let values = strings(&[
            "c:\\amipro\\docs\\mergtest.sam",
            "",
            "1",
            "1",
            "3",
            "12",
            "1440",
            "1440",
        ]);
        let file = RecordFile::from_values(&values).unwrap();
        assert_eq!(file, RecordFile::new("c:\\amipro\\docs\\mergtest.sam"));
        assert_eq!(file.values(), values);
        // Memory's values without As labels are kept as read.
        let odd = strings(&["a.sam", "*dBase", "2", "26352", "30616", "14636", "4", "0"]);
        assert_eq!(RecordFile::from_values(&odd).unwrap().values(), odd);
        assert!(
            RecordFile::from_values(&strings(&["a.sam", "", "x", "1", "3", "12", "1", "1"]))
                .is_none()
        );
        assert!(RecordFile::from_values(&strings(&["a.sam"])).is_none());
    }

    #[test]
    fn merge_settings_survive_the_native_format() {
        let mut document = crate::Document::from_plain_text("Dear <NAME>");
        document.set_merge(MergeSettings {
            record_file: Some(RecordFile::new("l:\\in\\mgd1.sam")),
            conditions: vec![
                Condition {
                    field: "CITY".into(),
                    operator: Operator::Equal,
                    value: "Boston\tnorth".into(),
                    join: Join::Or,
                },
                Condition {
                    field: "AMOUNT".into(),
                    operator: Operator::GreaterOrEqual,
                    value: String::new(),
                    join: Join::And,
                },
            ],
            with_conditions: true,
        });
        let text = document.encode_native().unwrap();
        let back = crate::Document::decode_native(&text).unwrap();
        assert_eq!(back.merge(), document.merge());
        assert!(document.merge().has_conditions());
    }

    #[test]
    fn operators_are_ami_pros_six() {
        for operator in Operator::ALL {
            assert_eq!(Operator::from_symbol(operator.symbol()), Some(operator));
        }
        assert!(Operator::Equal.takes_empty() && !Operator::Less.takes_empty());
    }
}
