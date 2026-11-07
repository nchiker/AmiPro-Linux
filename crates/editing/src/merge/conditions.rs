//! Merge conditions as Ami Pro evaluates them (`docs/research/MERGE_NOTES.md`,
//! section 5; decision MG6): rows of field, operator, and value, read left
//! to right with And binding tighter than Or (`13a0:0b69`); each compares
//! as numbers when the condition's value is one, else as dates when both
//! sides are, else as text (`13a0:0bbc`).

use std::cmp::Ordering;

use amipro_document::merge::{Condition, Join, Operator};

use super::data::DataTable;

/// Whether text comparisons ignore case: they do not (VM fixtures MGM3,
/// MGM5: `smith` is not `Smith`, `boston` not `Boston`).
pub const TEXT_IGNORES_CASE: bool = false;

/// Reads `text` as Ami Pro reads a number (`1538:0000`): trimmed, with a
/// currency sign, thousands separators, the decimal point, a leading minus
/// sign, parentheses, or a trailing minus.
pub fn ami_pro_number(text: &str) -> Option<f64> {
    let mut text = text.trim();
    let mut negative = false;
    if let Some(inner) = text
        .strip_prefix('(')
        .and_then(|text| text.strip_suffix(')'))
    {
        negative = true;
        text = inner.trim();
    }
    if let Some(rest) = text.strip_suffix('-') {
        negative = !negative;
        text = rest.trim_end();
    }
    if let Some(rest) = text.strip_prefix('-') {
        negative = !negative;
        text = rest.trim_start();
    }
    let text = text
        .strip_prefix(['$', '£', '€', '¥'])
        .unwrap_or(text)
        .trim_start();
    let text = text.strip_prefix('-').map_or(text, |rest| {
        negative = !negative;
        rest
    });
    if text.is_empty() {
        return None;
    }
    let mut digits = String::new();
    let mut point = false;
    let mut any = false;
    for character in text.chars() {
        match character {
            '0'..='9' => {
                digits.push(character);
                any = true;
            }
            ',' if !point => {}
            '.' if !point => {
                point = true;
                digits.push('.');
            }
            _ => return None,
        }
    }
    if !any {
        return None;
    }
    let value: f64 = digits.parse().ok()?;
    Some(if negative { -value } else { value })
}

/// Reads `text` as a date (`1538:02db`): month, day, and year separated by
/// `/` or `-`; a two-digit year is in the 1900s. The result counts days,
/// for comparing.
pub fn ami_pro_date(text: &str) -> Option<i64> {
    let parts: Vec<&str> = text.trim().split(['/', '-']).collect();
    let [month, day, year] = parts.as_slice() else {
        return None;
    };
    let number = |part: &str| -> Option<i64> {
        (!part.is_empty() && part.len() <= 4 && part.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| part.parse().ok())
            .flatten()
    };
    let (month, day, mut year) = (number(month)?, number(day)?, number(year)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    if year < 100 {
        year += 1900;
    }
    // Days from a fixed origin (the proleptic Gregorian calendar).
    let (y, m) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y.div_euclid(400);
    let of_era = y - era * 400;
    let of_year = (153 * m + 2) / 5 + day - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    Some(era * 146_097 + of_era_days)
}

/// How a record's `value` compares with a condition's `against`.
pub fn compare(value: &str, against: &str) -> Ordering {
    let value = value.trim();
    let against = against.trim();
    if let Some(number) = ami_pro_number(against) {
        if let Some(record) = ami_pro_number(value) {
            return record.partial_cmp(&number).unwrap_or(Ordering::Equal);
        }
    }
    if let (Some(record), Some(date)) = (ami_pro_date(value), ami_pro_date(against)) {
        return record.cmp(&date);
    }
    if TEXT_IGNORES_CASE {
        value.to_lowercase().cmp(&against.to_lowercase())
    } else {
        value.cmp(against)
    }
}

/// Whether one condition holds for a record's value.
pub fn holds(condition: &Condition, value: &str) -> bool {
    let ordering = compare(value, &condition.value);
    match condition.operator {
        Operator::Equal => ordering == Ordering::Equal,
        Operator::Less => ordering == Ordering::Less,
        Operator::Greater => ordering == Ordering::Greater,
        Operator::NotEqual => ordering != Ordering::Equal,
        Operator::LessOrEqual => ordering != Ordering::Greater,
        Operator::GreaterOrEqual => ordering != Ordering::Less,
    }
}

/// Whether record `record` of `table` passes `conditions`: And binds
/// tighter than Or. A condition on a field the data file lacks holds for
/// no record. No conditions select every record.
pub fn passes(conditions: &[Condition], table: &DataTable, record: usize) -> bool {
    if conditions.is_empty() {
        return true;
    }
    let mut any = false;
    let mut group = true;
    for (index, condition) in conditions.iter().enumerate() {
        let value = table.value(record, &condition.field);
        group &= value.is_some_and(|value| holds(condition, value));
        let last = index + 1 == conditions.len();
        if last || condition.join == Join::Or {
            any |= group;
            group = true;
        }
    }
    any
}

/// The records `conditions` select, in order.
pub fn selected(conditions: &[Condition], table: &DataTable) -> Vec<usize> {
    (0..table.records.len())
        .filter(|record| passes(conditions, table, *record))
        .collect()
}

/// Why a condition row is not usable: an empty value with an operator that
/// needs one, or no field.
pub fn problem(condition: &Condition, table: &DataTable) -> Option<String> {
    if condition.field.trim().is_empty() {
        return Some("Choose a field.".into());
    }
    if table.field(&condition.field).is_none() {
        return Some(format!(
            "{} is not a valid field name.",
            condition.field.trim()
        ));
    }
    if condition.value.trim().is_empty() && !condition.operator.takes_empty() {
        return Some("Only = and != take an empty value.".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> DataTable {
        let row = |values: &[&str]| values.iter().map(|value| value.to_string()).collect();
        DataTable {
            names: row(&["LAST", "CITY", "AMOUNT", "DUE"]),
            records: vec![
                row(&["Allen", "Boston", "150", "3/15/94"]),
                row(&["Beck", "Seattle", "75.50", "1/2/94"]),
                row(&["smith", "Boston", "$1,200.00", "12/31/93"]),
                row(&["Smith", "Dallas", "(20)", "2/28/94"]),
                row(&["Black", "boston", "100", "3/15/94"]),
            ],
        }
    }

    fn condition(field: &str, operator: &str, value: &str, join: Join) -> Condition {
        Condition {
            field: field.into(),
            operator: Operator::from_symbol(operator).unwrap(),
            value: value.into(),
            join,
        }
    }

    #[test]
    fn numbers_read_as_ami_pros() {
        assert_eq!(ami_pro_number(" $1,200.00 "), Some(1200.0));
        assert_eq!(ami_pro_number("(20)"), Some(-20.0));
        assert_eq!(ami_pro_number("20-"), Some(-20.0));
        assert_eq!(ami_pro_number("-3.5"), Some(-3.5));
        assert_eq!(ami_pro_number("3/15/94"), None);
        assert_eq!(ami_pro_number("Boston"), None);
        assert_eq!(ami_pro_number(""), None);
    }

    #[test]
    fn dates_compare_as_dates() {
        assert!(ami_pro_date("3/15/94").unwrap() > ami_pro_date("12/31/93").unwrap());
        assert_eq!(ami_pro_date("3/15/1994"), ami_pro_date("3-15-94"));
        assert_eq!(ami_pro_date("13/1/94"), None);
        assert_eq!(compare("3/15/94", "3/1/94"), Ordering::Greater);
    }

    #[test]
    fn conditions_select_as_ami_pro_does() {
        let table = table();
        let one = |c: Condition| selected(&[c], &table);
        assert_eq!(one(condition("AMOUNT", ">", "100", Join::And)), [0, 2]);
        assert_eq!(one(condition("DUE", ">=", "3/1/94", Join::And)), [0, 4]);
        assert_eq!(one(condition("LAST", "=", "smith", Join::And)), [2]);
        // And binds tighter than Or.
        let conditions = [
            condition("CITY", "=", "Boston", Join::And),
            condition("AMOUNT", ">", "100", Join::Or),
            condition("LAST", "=", "Beck", Join::And),
        ];
        assert_eq!(selected(&conditions, &table), [0, 1, 2]);
        assert_eq!(selected(&[], &table).len(), 5);
        assert!(one(condition("NOPE", "=", "x", Join::And)).is_empty());
    }

    #[test]
    fn unusable_rows_say_why() {
        let table = table();
        assert!(problem(&condition("LAST", "<", "", Join::And), &table).is_some());
        assert!(problem(&condition("LAST", "=", "", Join::And), &table).is_none());
        assert!(problem(&condition("ZIP", "=", "1", Join::And), &table).is_some());
    }
}
