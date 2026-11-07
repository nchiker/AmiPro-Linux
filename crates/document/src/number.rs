//! Numbers in table cells as Ami Pro 3.1 has them (run C, TD.3;
//! `docs/research/TABLE_DATA_NOTES.md` sections 3 and 10): the paragraph
//! style's number format (`[nfmt]`), the scanner that decides whether a
//! cell's text is a number, and the stored `%.*f` text of a number.

/// Thousands separator on (`[nfmt]` flag word).
pub const THOUSANDS: u32 = 0x08;
/// Negative format: leading minus.
pub const LEADING_MINUS: u32 = 0x10;
/// Negative format: trailing minus (exclusive with the leading minus).
pub const TRAILING_MINUS: u32 = 0x20;
/// Negative format: parentheses.
pub const PARENTHESES: u32 = 0x40;
/// Negative format: red.
pub const RED: u32 = 0x80;
/// The currency symbol goes before the number (clear: after).
pub const SYMBOL_LEADING: u32 = 0x100;

/// Modify Style › Table format's Cell format (`[nfmt]` value 2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumberKind {
    General,
    Fixed,
    Currency,
    Percent,
    /// A value Ami Pro's dialog does not offer, kept as read.
    Other(u32),
}

impl NumberKind {
    pub fn from_value(value: u32) -> Self {
        match value {
            1 => Self::General,
            2 => Self::Fixed,
            3 => Self::Currency,
            4 => Self::Percent,
            other => Self::Other(other),
        }
    }

    pub fn value(self) -> u32 {
        match self {
            Self::General => 1,
            Self::Fixed => 2,
            Self::Currency => 3,
            Self::Percent => 4,
            Self::Other(other) => other,
        }
    }
}

/// A paragraph style's number format: Table format in Modify Style, kept
/// in a style's `[nfmt]` section as six values (the flag word, the cell
/// format, the decimal places, then the decimal, thousands, and currency
/// symbols). The symbols are kept as the strings read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumberFormat {
    /// The flag word: [`THOUSANDS`], [`LEADING_MINUS`], [`TRAILING_MINUS`],
    /// [`PARENTHESES`], [`RED`], [`SYMBOL_LEADING`]; other bits are kept.
    pub flags: u32,
    pub kind: NumberKind,
    pub decimals: u32,
    pub decimal_symbol: String,
    pub thousands_symbol: String,
    pub currency_symbol: String,
}

impl Default for NumberFormat {
    /// What Ami Pro writes for a new style (and the corpus's 2,421 most
    /// common record, `272 1 2 . , $`).
    fn default() -> Self {
        Self {
            flags: 272,
            kind: NumberKind::General,
            decimals: 2,
            decimal_symbol: ".".into(),
            thousands_symbol: ",".into(),
            currency_symbol: "$".into(),
        }
    }
}

impl NumberFormat {
    /// The six values as the style record writes them.
    pub fn to_values(&self) -> [String; 6] {
        [
            self.flags.to_string(),
            self.kind.value().to_string(),
            self.decimals.to_string(),
            self.decimal_symbol.clone(),
            self.thousands_symbol.clone(),
            self.currency_symbol.clone(),
        ]
    }

    /// The format of six record values; `None` when a number is not one.
    pub fn from_values(values: &[String; 6]) -> Option<Self> {
        let number = |text: &str| text.trim().parse::<u32>().ok();
        Some(Self {
            flags: number(&values[0])?,
            kind: NumberKind::from_value(number(&values[1])?),
            decimals: number(&values[2])?,
            decimal_symbol: values[3].clone(),
            thousands_symbol: values[4].clone(),
            currency_symbol: values[5].clone(),
        })
    }

    pub fn thousands(&self) -> bool {
        self.flags & THOUSANDS != 0
    }

    pub fn leading_minus(&self) -> bool {
        self.flags & LEADING_MINUS != 0
    }

    pub fn trailing_minus(&self) -> bool {
        self.flags & TRAILING_MINUS != 0
    }

    pub fn parentheses(&self) -> bool {
        self.flags & PARENTHESES != 0
    }

    pub fn red(&self) -> bool {
        self.flags & RED != 0
    }

    pub fn symbol_leading(&self) -> bool {
        self.flags & SYMBOL_LEADING != 0
    }

    /// The decimal places a stored number has: none when the style has no
    /// decimal symbol (`1400:0d98`).
    pub fn stored_decimals(&self) -> usize {
        if self.decimal_symbol.is_empty() {
            0
        } else {
            self.decimals.min(15) as usize
        }
    }

    /// A number as a cell stores it: `%.*f` at the style's decimals with
    /// the decimal symbol put in; no thousands separator, currency symbol,
    /// or percent sign. A result that prints as `-0.00` loses its minus
    /// (`1400:0a2f`). A value exactly half way rounds up (VM TDT4: 1234.5
    /// at no decimals is 1235, not the C library's 1234).
    pub fn stored_text(&self, value: f64) -> String {
        let (negative, digits) = fixed_digits(value, self.stored_decimals());
        let mut text = String::with_capacity(digits.len() + 1);
        if negative {
            text.push('-');
        }
        text.push_str(&digits);
        if self.decimal_symbol != "." && !self.decimal_symbol.is_empty() {
            text = text.replacen('.', &self.decimal_symbol, 1);
        }
        text
    }

    /// How a cell's text is shown through this style (VM TDA1, TDA2;
    /// `TABLE_DATA_NOTES.md` section 11.1): `None` when the text is not a
    /// number (it is drawn as typed), else the decorated text and whether
    /// it is drawn red. The stored text is never changed by it, and a text
    /// that is already decorated (`$1,200.00`) is not a number.
    pub fn display(&self, text: &str) -> Option<Displayed> {
        let value = self.scan(text)?;
        let decimals = self.stored_decimals();
        let (mut negative, mut digits) = fixed_digits(value, decimals);
        let currency = match self.kind {
            NumberKind::General | NumberKind::Fixed => false,
            NumberKind::Currency => true,
            NumberKind::Percent => false,
            NumberKind::Other(_) => return None,
        };
        if self.kind == NumberKind::General && digits.contains('.') {
            // General drops the trailing zeros and then a bare point.
            digits.truncate(digits.trim_end_matches('0').trim_end_matches('.').len());
        }
        if digits.chars().all(|c| matches!(c, '0' | '.')) {
            negative = false;
        }
        let (whole, fraction) = match digits.split_once('.') {
            Some((whole, fraction)) => (whole.to_owned(), Some(fraction.to_owned())),
            None => (digits, None),
        };
        let mut number = if self.thousands() && !self.thousands_symbol.is_empty() {
            group(&whole, &self.thousands_symbol)
        } else {
            whole
        };
        if let Some(fraction) = fraction {
            number.push_str(&self.decimal_symbol);
            number.push_str(&fraction);
        }
        let mut shown = String::new();
        // Without a mark for negatives set, a minus leads.
        let marks = self.flags & (LEADING_MINUS | TRAILING_MINUS | PARENTHESES);
        let leading_minus = negative && (self.leading_minus() || marks == 0);
        let trailing_minus = negative && self.trailing_minus() && !self.leading_minus();
        let parentheses = negative && self.parentheses();
        if leading_minus {
            shown.push('-');
        }
        let symbol = &self.currency_symbol;
        if currency && self.symbol_leading() {
            shown.push_str(symbol);
        }
        if parentheses {
            shown.push('(');
        }
        shown.push_str(&number);
        if parentheses {
            shown.push(')');
        }
        if trailing_minus {
            shown.push('-');
        }
        if currency && !self.symbol_leading() {
            shown.push_str(symbol);
        }
        if self.kind == NumberKind::Percent {
            shown.push('%');
        }
        Some(Displayed {
            text: shown,
            red: negative && self.red(),
        })
    }
    /// Whether a cell's text reads as a number under this style.
    pub fn scan(&self, text: &str) -> Option<f64> {
        scan_number(text, &self.decimal_symbol)
    }
}

/// A cell's text as the style shows it ([`NumberFormat::display`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Displayed {
    pub text: String,
    /// The whole text is drawn red (a negative under the Red format).
    pub red: bool,
}

/// `value` as `digits.digits` at `decimals` places, rounded half up on its
/// exact decimal expansion, and whether it is negative after rounding (a
/// result that rounds to zero has no sign).
fn fixed_digits(value: f64, decimals: usize) -> (bool, String) {
    let exact = format!("{:.*}", decimals + 40, value.abs());
    let (whole, fraction) = exact.split_once('.').unwrap_or((&exact, ""));
    let mut kept: Vec<u8> = whole
        .bytes()
        .chain(fraction.bytes().take(decimals))
        .collect();
    if fraction
        .as_bytes()
        .get(decimals)
        .is_some_and(|d| *d >= b'5')
    {
        let mut at = kept.len();
        loop {
            if at == 0 {
                kept.insert(0, b'1');
                break;
            }
            at -= 1;
            if kept[at] == b'9' {
                kept[at] = b'0';
            } else {
                kept[at] += 1;
                break;
            }
        }
    }
    let point = kept.len() - decimals;
    let mut digits = String::from_utf8(kept[..point].to_vec()).unwrap_or_default();
    if decimals > 0 {
        digits.push('.');
        digits.push_str(&String::from_utf8(kept[point..].to_vec()).unwrap_or_default());
    }
    let negative = value < 0.0 && digits.chars().any(|c| c.is_ascii_digit() && c != '0');
    (negative, digits)
}

/// `whole` (digits) in groups of three from the right.
fn group(whole: &str, separator: &str) -> String {
    let mut grouped = String::with_capacity(whole.len() * 2);
    for (index, digit) in whole.chars().enumerate() {
        if index > 0 && (whole.len() - index) % 3 == 0 {
            grouped.push_str(separator);
        }
        grouped.push(digit);
    }
    grouped
}

/// Ami Pro's scanner (`1400:0ab6`, `095c`, `09f0`): a cell's text is a
/// number only when it is 1 to 255 characters of digits, a sign first (with
/// at least one more character after it), and at most one decimal mark
/// (`.` or the style's decimal symbol). A comma, `$`, `%`, a space, a
/// letter, or anything else makes the text of the cell text. Needs a digit.
pub fn scan_number(text: &str, decimal_symbol: &str) -> Option<f64> {
    let length = text.chars().count();
    if !(1..=255).contains(&length) {
        return None;
    }
    let mut marks = 0;
    let mut digits = 0;
    let mut clean = String::with_capacity(text.len());
    let mut first = true;
    let symbol: Vec<char> = decimal_symbol.chars().collect();
    for c in text.chars() {
        match c {
            '0'..='9' => {
                digits += 1;
                clean.push(c);
            }
            '+' | '-' if first && length >= 2 => clean.push(c),
            '.' => {
                marks += 1;
                clean.push('.');
            }
            c if !symbol.is_empty() && c == symbol[0] && symbol.len() == 1 => {
                marks += 1;
                clean.push('.');
            }
            _ => return None,
        }
        first = false;
    }
    if marks > 1 || digits == 0 {
        return None;
    }
    let value: f64 = clean.parse().ok()?;
    // No negative zero.
    Some(if value == 0.0 { 0.0 } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_word_reads_as_the_dialog_does() {
        let table_dollar = NumberFormat {
            flags: 0x118,
            kind: NumberKind::Currency,
            ..NumberFormat::default()
        };
        assert!(table_dollar.thousands() && table_dollar.leading_minus());
        assert!(table_dollar.symbol_leading());
        assert!(
            !table_dollar.trailing_minus() && !table_dollar.parentheses() && !table_dollar.red()
        );
        let brackets = NumberFormat {
            flags: 0x158,
            ..NumberFormat::default()
        };
        assert!(brackets.parentheses() && brackets.leading_minus());
        assert!(
            NumberFormat {
                flags: 0x128,
                ..NumberFormat::default()
            }
            .trailing_minus()
        );
        assert!(
            NumberFormat {
                flags: 0x80,
                ..NumberFormat::default()
            }
            .red()
        );
    }

    #[test]
    fn six_values_round_trip() {
        let values = ["280", "3", "0", ".", ",", ""].map(String::from);
        let format = NumberFormat::from_values(&values).unwrap();
        assert_eq!(format.kind, NumberKind::Currency);
        assert_eq!(format.to_values(), values);
        let odd = ["16", "9", "2", ".", ",", "$"].map(String::from);
        let format = NumberFormat::from_values(&odd).unwrap();
        assert_eq!(format.kind, NumberKind::Other(9));
        assert_eq!(format.to_values(), odd);
        assert!(
            NumberFormat::from_values(&["x", "1", "2", ".", ",", "$"].map(String::from)).is_none()
        );
        assert_eq!(
            NumberFormat::default().to_values(),
            ["272", "1", "2", ".", ",", "$"]
        );
    }

    #[test]
    fn the_scanner_takes_only_digits_signs_and_the_mark() {
        assert_eq!(scan_number("5.00", "."), Some(5.0));
        assert_eq!(scan_number("-7.25", "."), Some(-7.25));
        assert_eq!(scan_number("+3", "."), Some(3.0));
        assert_eq!(scan_number("007", "."), Some(7.0));
        assert_eq!(scan_number(".5", "."), Some(0.5));
        assert_eq!(scan_number("-0", "."), Some(0.0));
        for text in [
            "", "-", "+", ".", "1,200.00", "$5", "5%", " 5", "5 ", "1e5", "1.2.3", "--3", "3-",
            "a", "5\n6",
        ] {
            assert_eq!(scan_number(text, "."), None, "{text:?}");
        }
        assert_eq!(scan_number("3,5", ","), Some(3.5));
        assert_eq!(
            scan_number("3.5", ","),
            Some(3.5),
            "the system mark counts too"
        );
        assert_eq!(scan_number("3,5.1", ","), None, "one mark only");
        assert_eq!(
            scan_number(&"1".repeat(255), "."),
            Some("1".repeat(255).parse().unwrap())
        );
        assert_eq!(scan_number(&"1".repeat(256), "."), None);
    }

    #[test]
    fn stored_text_is_printf_at_the_styles_decimals() {
        let two = NumberFormat::default();
        assert_eq!(two.stored_text(3250.0), "3250.00");
        assert_eq!(two.stored_text(1234.5), "1234.50");
        assert_eq!(two.stored_text(-0.001), "0.00", "-0.00 loses its minus");
        assert_eq!(two.stored_text(-7.255), "-7.25");
        let none = NumberFormat {
            decimals: 0,
            ..NumberFormat::default()
        };
        assert_eq!(none.stored_text(2.5), "3", "half way rounds up (TDT4)");
        assert_eq!(none.stored_text(-7.25), "-7");
        let comma = NumberFormat {
            decimal_symbol: ",".into(),
            ..NumberFormat::default()
        };
        assert_eq!(comma.stored_text(1.5), "1,50");
        let blank = NumberFormat {
            decimal_symbol: String::new(),
            ..NumberFormat::default()
        };
        assert_eq!(blank.stored_text(9.99), "10", "no decimal symbol: integers");
    }

    fn style(flags: u32, kind: NumberKind, decimals: u32) -> NumberFormat {
        NumberFormat {
            flags,
            kind,
            decimals,
            ..NumberFormat::default()
        }
    }

    fn shown(format: &NumberFormat, text: &str) -> String {
        format.display(text).map_or("<text>".into(), |d| {
            if d.red {
                format!("{}!", d.text)
            } else {
                d.text
            }
        })
    }

    #[test]
    fn general_drops_trailing_zeros_and_keeps_thousands_tda1() {
        let two = style(0x118, NumberKind::General, 2);
        for (stored, text) in [
            ("1234.50", "1,234.5"),
            ("-7.25", "-7.25"),
            ("0.05", "0.05"),
            ("1234567.89", "1,234,567.89"),
            ("0.00", "0"),
            ("007", "7"),
            ("12.00", "12"),
            ("5", "5"),
            ("-0.00", "0"),
        ] {
            assert_eq!(shown(&two, stored), text, "{stored}");
        }
        let none = style(0x118, NumberKind::General, 0);
        for (stored, text) in [
            ("1234.50", "1,235"),
            ("-7.25", "-7"),
            ("0.05", "0"),
            ("1234567.89", "1,234,568"),
            ("0.5", "1"),
        ] {
            assert_eq!(shown(&none, stored), text, "{stored}");
        }
    }

    #[test]
    fn fixed_shows_the_decimals_tda1() {
        let two = style(0x18, NumberKind::Fixed, 2);
        assert_eq!(shown(&two, "5"), "5.00");
        assert_eq!(shown(&two, "1234.5"), "1,234.50");
        assert_eq!(shown(&two, "-7.25"), "-7.25");
        assert_eq!(
            shown(&style(0x18, NumberKind::Fixed, 3), "1234.5"),
            "1,234.500"
        );
        let flat = style(0x20, NumberKind::Fixed, 2);
        assert_eq!(shown(&flat, "-7.25"), "7.25-");
        assert_eq!(shown(&flat, "1234.5"), "1234.50");
    }

    #[test]
    fn currency_places_the_symbol_and_the_negative_tda1() {
        let leading = style(0x118, NumberKind::Currency, 2);
        assert_eq!(shown(&leading, "1234.5"), "$1,234.50");
        assert_eq!(shown(&leading, "0"), "$0.00");
        assert_eq!(shown(&leading, "-7.25"), "-$7.25");
        let trailing = style(0x18, NumberKind::Currency, 2);
        assert_eq!(shown(&trailing, "1234.5"), "1,234.50$");
        let after = style(0x28, NumberKind::Currency, 2);
        assert_eq!(shown(&after, "-7.25"), "7.25-$");
        let leading_after = style(0x128, NumberKind::Currency, 2);
        assert_eq!(shown(&leading_after, "-7.25"), "$7.25-");
        let brackets = style(0x148, NumberKind::Currency, 2);
        assert_eq!(shown(&brackets, "-7.25"), "$(7.25)");
        assert_eq!(shown(&brackets, "-1234.5"), "$(1,234.50)");
        assert_eq!(shown(&brackets, "7.25"), "$7.25");
        let red = style(0x198, NumberKind::Currency, 2);
        assert_eq!(shown(&red, "-7.25"), "-$7.25!");
        assert_eq!(shown(&red, "7.25"), "$7.25");
        let plain = style(0x110, NumberKind::Currency, 2);
        assert_eq!(shown(&plain, "1234.5"), "$1234.50");
    }

    #[test]
    fn percent_is_not_multiplied_and_the_marks_add_tda2() {
        let percent = style(0x18, NumberKind::Percent, 2);
        assert_eq!(shown(&percent, "1234.5"), "1,234.50%");
        assert_eq!(shown(&percent, "5"), "5.00%");
        assert_eq!(shown(&percent, "0.05"), "0.05%");
        assert_eq!(shown(&percent, "-7.25"), "-7.25%");
        let both = style(0x58, NumberKind::Percent, 2);
        assert_eq!(shown(&both, "-7.25"), "-(7.25)%");
    }

    #[test]
    fn text_cells_are_drawn_as_typed_and_symbols_follow_the_style() {
        let currency = style(0x118, NumberKind::Currency, 2);
        for text in ["$1,200.00", "5%", " 5", "5 ", "1,5", "abc", "", "-"] {
            assert_eq!(currency.display(text), None, "{text:?}");
        }
        let french = NumberFormat {
            decimal_symbol: ",".into(),
            thousands_symbol: " ".into(),
            currency_symbol: "EUR".into(),
            ..style(0x08, NumberKind::Currency, 2)
        };
        assert_eq!(shown(&french, "1234567,5"), "1 234 567,50EUR");
        assert_eq!(shown(&french, "1234567.5"), "1 234 567,50EUR");
        let no_mark = NumberFormat {
            decimal_symbol: String::new(),
            ..style(0x18, NumberKind::Fixed, 2)
        };
        assert_eq!(shown(&no_mark, "1234.5"), "1,235");
        assert_eq!(style(0x18, NumberKind::Other(9), 2).display("5"), None);
        // No mark for negatives set: a leading minus.
        assert_eq!(shown(&style(0x08, NumberKind::Fixed, 2), "-3"), "-3.00");
    }
}
