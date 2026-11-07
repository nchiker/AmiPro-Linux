use super::*;

fn key(field: u8, kind: SortKind, word: SortWord) -> SortKey {
    SortKey { field, kind, word }
}

fn settings(keys: Vec<SortKey>, descending: bool) -> SortSettings {
    SortSettings {
        keys,
        descending,
        ..SortSettings::default()
    }
}

/// Sorts single-paragraph records and returns them in their new order.
fn sort(texts: &[&str], settings: &SortSettings) -> Vec<String> {
    let records: Vec<Vec<String>> = texts.iter().map(|t| vec![(*t).to_owned()]).collect();
    order_of_records(&records, settings)
        .into_iter()
        .map(|i| texts[i].to_owned())
        .collect()
}

fn strings(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|t| (*t).to_owned()).collect()
}

const ALL: SortWord = SortWord::All;
const FIRST: SortWord = SortWord::First;
const SECOND: SortWord = SortWord::Second;
const ALPHA: SortKind = SortKind::Alphanumeric;
const NUMERIC: SortKind = SortKind::Numeric;

#[test]
fn alphanumeric_keys_follow_the_sorttab_weights_then_bytes() {
    // TDV8: punctuation first (by the next character), digits before
    // letters, case-blind with upper before lower on a tie of weights, a
    // space after punctuation.
    let input = [
        "Zed", "banana", "apple", "10", "Banana", "a b", "#hash", "5", "cherry", "A", "a", "Apple",
        "9", "a-b", "ab", "aardvark", "-5", "1abc", "a,b", "a.b", "_under", "[bracket", "~tilde",
    ];
    let sorted = sort(&input, &settings(vec![key(1, ALPHA, ALL)], false));
    assert_eq!(
        sorted,
        strings(&[
            "-5", "[bracket", "#hash", "~tilde", "_under", "10", "1abc", "5", "9", "A", "a", "a,b",
            "a-b", "a.b", "a b", "aardvark", "ab", "Apple", "apple", "Banana", "banana", "cherry",
            "Zed"
        ])
    );
}

#[test]
fn numeric_keys_reduce_to_their_digits() {
    let input = [
        "1,000", "1e2", "007", "+4", "$3.5", "3.25", ".5", "-0.5", "-5", "(7)", "zz", "abc",
    ];
    let ascending = sort(&input, &settings(vec![key(1, NUMERIC, ALL)], false));
    // -7, -5, -0.5, .5, 3.25, 3.5, 4, 7, 12, 1000, then text without a digit.
    assert_eq!(
        ascending,
        strings(&[
            "(7)", "-5", "-0.5", ".5", "3.25", "$3.5", "+4", "007", "1e2", "1,000", "abc", "zz"
        ])
    );
    let descending = sort(&input, &settings(vec![key(1, NUMERIC, ALL)], true));
    let mut reversed = ascending;
    reversed.reverse();
    assert_eq!(
        descending, reversed,
        "without ties Descending is the reverse"
    );
}

#[test]
fn a_word_key_takes_the_first_or_second_word_or_all() {
    let input = ["x 12", "b 7", "a 30", "c 9"];
    let first = sort(&input, &settings(vec![key(1, ALPHA, FIRST)], false));
    assert_eq!(first, strings(&["a 30", "b 7", "c 9", "x 12"]));
    let second = sort(&input, &settings(vec![key(1, NUMERIC, SECOND)], false));
    assert_eq!(second, strings(&["b 7", "c 9", "x 12", "a 30"]));
    // Numeric on the first word: no digit in any, nothing differs; a
    // single word is the whole key.
    let none = sort(
        &["b 7", "a 5"],
        &settings(vec![key(1, NUMERIC, FIRST)], false),
    );
    assert_eq!(none.len(), 2);
}

#[test]
fn fields_are_cut_at_the_delimiter_and_numbered_through_the_record() {
    let comma = SortSettings {
        keys: vec![key(2, ALPHA, ALL)],
        delimiter: Some(','),
        ..SortSettings::default()
    };
    assert_eq!(
        sort(&["x,pear,1", "y,apple,2", "z,fig,3"], &comma),
        strings(&["y,apple,2", "z,fig,3", "x,pear,1"])
    );
    // A tab is a field break by default.
    assert_eq!(
        sort(
            &["x\tpear", "y\tapple"],
            &settings(vec![key(2, ALPHA, ALL)], false)
        ),
        strings(&["y\tapple", "x\tpear"])
    );
    // A record of two paragraphs: field 2 is the second paragraph's first.
    let two = SortSettings {
        keys: vec![key(2, ALPHA, ALL)],
        record: 2,
        ..SortSettings::default()
    };
    let records = vec![
        vec!["Smith".to_owned(), "Zoe".to_owned()],
        vec!["Jones".to_owned(), "Adam".to_owned()],
    ];
    assert_eq!(order_of_records(&records, &two), [1, 0]);
}

#[test]
fn later_levels_break_ties_of_earlier_ones() {
    let two = settings(vec![key(1, ALPHA, FIRST), key(1, NUMERIC, SECOND)], false);
    assert_eq!(
        sort(&["k2 9", "k1 30", "k2 10", "k1 5"], &two),
        strings(&["k1 5", "k1 30", "k2 9", "k2 10"])
    );
}

/// Ami Pro's saved results (TDV8, `TDX5.SAM` and `TDY1.SAM`): equal keys
/// come out in the heap sort's order, which is not the text's.
#[test]
fn equal_keys_come_out_in_ami_pros_heap_order() {
    // TDY1's rows sorted Ascending on column 1, Word 1st, three levels the
    // same: the second column shows where each tie went.
    let rows = [
        "k2 d09", "k1 d05", "k2 d03", "k1 d08", "k3 d01", "k1 d02", "k2 d07", "k3 d06", "k1 d04",
        "k2 d10", "k3 d11", "k1 d12",
    ];
    let by_first_word = settings(vec![key(1, ALPHA, FIRST)], false);
    let sorted = sort(&rows, &by_first_word);
    let seconds: Vec<&str> = sorted.iter().map(|row| &row[3..]).collect();
    assert_eq!(
        seconds,
        [
            "d05", "d08", "d04", "d02", "d12", "d03", "d09", "d10", "d07", "d06", "d11", "d01"
        ]
    );
    // The same keys with the whole text: no ties (TDX5's paragraphs).
    let whole = sort(&rows, &settings(vec![key(1, ALPHA, ALL)], false));
    let mut expected = rows.to_vec();
    expected.sort_unstable();
    assert_eq!(whole, strings(&expected));
    let down = sort(&rows, &settings(vec![key(1, ALPHA, ALL)], true));
    expected.reverse();
    assert_eq!(down, strings(&expected));
}

#[test]
fn descending_is_the_other_heap_and_reads_backwards() {
    // Descending with ties: the heap the other way round, written back
    // from the last record. (Not the exact reverse of Ascending's ties:
    // inferred from the decompile, `1360:259c`; the VM saw no tie case.)
    let rows = ["k2 a", "k1 b", "k2 c", "k1 d", "k3 e", "k1 f"];
    let by_first_word = settings(vec![key(1, ALPHA, FIRST)], true);
    let sorted = sort(&rows, &by_first_word);
    let keys: Vec<&str> = sorted.iter().map(|row| &row[..2]).collect();
    assert_eq!(keys, ["k3", "k2", "k2", "k1", "k1", "k1"]);
}

#[test]
fn a_missing_field_or_word_is_an_empty_key() {
    let by_second = settings(vec![key(1, ALPHA, SECOND)], false);
    assert_eq!(
        sort(&["b z", "single", "c a"], &by_second),
        strings(&["single", "c a", "b z"])
    );
}
