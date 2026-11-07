//! Writing XML as text.

/// `text` with XML's special characters escaped, and the characters XML
/// cannot hold (controls other than tab, line feed, and return) left out.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' | '\n' | '\r' => out.push(character),
            '\u{0}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}' => {}
            _ => out.push(character),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn escapes_and_drops_what_xml_cannot_hold() {
        assert_eq!(
            super::escape("a<b & \"c\"\u{1}"),
            "a&lt;b &amp; &quot;c&quot;"
        );
    }
}
