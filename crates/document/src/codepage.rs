//! Windows-1252, the code page Ami Pro's files are written in.

const HIGH_CONTROLS: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];

/// `bytes` read as Windows-1252.
pub fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| match *byte {
            0x80..=0x9F => HIGH_CONTROLS[usize::from(*byte - 0x80)],
            byte => char::from(byte),
        })
        .collect()
}

/// `character` in Windows-1252, or `?` where it has no byte.
pub fn windows_1252(character: char) -> u8 {
    if let Ok(byte) = u8::try_from(u32::from(character))
        && !(0x80..=0x9F).contains(&byte)
    {
        return byte;
    }
    HIGH_CONTROLS
        .iter()
        .position(|high| *high == character)
        .map_or(b'?', |index| 0x80 + index as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_byte_round_trips() {
        for byte in 0..=255_u8 {
            let text = decode_windows_1252(&[byte]);
            assert_eq!(windows_1252(text.chars().next().unwrap()), byte);
        }
        assert_eq!(windows_1252('€'), 0x80);
        assert_eq!(windows_1252('\u{2603}'), b'?');
    }
}
