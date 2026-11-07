//! Typed lengths, read the way Ami Pro's number boxes read them
//! (`AMIUICNT.DLL` `1060:004e`; `docs/research/PAGE_LAYOUT_NOTES.md`,
//! "Typed values and units"): Style Properties, Frame Properties, and Page
//! Layout Properties all take lengths this way, as Ami Pro's Modify Style,
//! Modify Frame Layout, and Modify Page Layout share one kind of box.

use amipro_document::Twips;

/// A typed length in twips as Ami Pro's boxes read one (`AMIUICNT.DLL`
/// `1060:004e`): a number in inches, or ending in `in`, `"`, `cm`, `mm`, or
/// `pt`; at most two decimals are read and the twips are truncated (8.27
/// in is 11,908); from 0 to 22 inches.
pub fn parse_length(text: &str) -> Option<Twips> {
    let text = text.trim().to_lowercase();
    // Ami Pro's units in twips per unit, as integers (`1060:0000`);
    // millimetres are tenths of its centimetre.
    let (number, per_unit, tenths) = [
        ("in", 1_440, false),
        ("\"", 1_440, false),
        ("cm", 567, false),
        ("mm", 567, true),
        ("pt", 20, false),
    ]
    .into_iter()
    .find_map(|(suffix, per_unit, tenths)| {
        text.strip_suffix(suffix)
            .map(|number| (number.trim(), per_unit, tenths))
    })
    .unwrap_or((text.as_str(), 1_440, false));
    let (whole, fraction) = number.split_once('.').unwrap_or((number, ""));
    if (whole.is_empty() && fraction.is_empty())
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let hundredths: i64 = format!("{fraction:0<2}")[..2].parse().ok()?;
    let mut twips = whole * per_unit + hundredths * per_unit / 100;
    if tenths {
        twips /= 10;
    }
    (0..=i64::from(amipro_editing::layout_setup::MAX_LENGTH))
        .contains(&twips)
        .then_some(twips as Twips)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_read_as_ami_pros_boxes_read_them() {
        // Two decimals, truncated (`1060:004e`): A4's 8.27 in is 11,908.
        assert_eq!(parse_length("8.27"), Some(11_908));
        assert_eq!(parse_length("11.69 in"), Some(16_833));
        assert_eq!(parse_length("1.505"), Some(2_160));
        assert_eq!(parse_length(".5\""), Some(720));
        assert_eq!(parse_length("2.54cm"), Some(1_440));
        assert_eq!(parse_length("10 mm"), Some(567));
        assert_eq!(parse_length("12pt"), Some(240));
        assert_eq!(parse_length("22"), Some(31_680));
        assert_eq!(parse_length("22.01"), None);
        assert_eq!(parse_length("-1"), None);
        assert_eq!(parse_length("1,5"), None);
        assert_eq!(parse_length(""), None);
    }
}
