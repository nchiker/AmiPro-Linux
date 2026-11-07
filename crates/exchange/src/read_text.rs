//! Plain text read as Ami Pro's ASCII import reads it (`15e0:0000`; VM
//! IMT, IMT3; `IMPORT_NOTES.md`), with the app's defaults (IM9): every line
//! a paragraph, or lines joined into paragraphs that a blank line ends; with
//! Keep style names, `@Name@` (or `<Name>`) at a paragraph's start applies
//! that style; the characters read as Unicode, Windows ANSI, PC-ASCII
//! (code page 437), or 7-bit ASCII.

use crate::import::{Characters, LineEnds, TextOptions};
use crate::{
    Block, CharProps, Columns, ExchangeDocument, Inline, PageSetup, Paragraph, ParagraphProps,
    Section, Style,
};

/// `bytes` as an exchange document.
pub fn read_text(bytes: &[u8], options: &TextOptions) -> ExchangeDocument {
    let text = decode(bytes, options.characters);
    let lines = split_lines(&text);
    let mut paragraphs: Vec<String> = match options.line_ends {
        LineEnds::Paragraphs => lines.into_iter().map(str::to_owned).collect(),
        LineEnds::Joined => joined(&lines),
    };
    // The line end closing the file's last line makes no paragraph.
    if options.line_ends == LineEnds::Paragraphs
        && paragraphs.len() > 1
        && paragraphs.last().is_some_and(String::is_empty)
    {
        paragraphs.pop();
    }
    let mut styles = vec![plain_style("Body Text")];
    let mut blocks = Vec::with_capacity(paragraphs.len());
    for line in paragraphs {
        let (style, rest) = if options.keep_style_names {
            match style_name(&line) {
                Some((name, rest)) => {
                    let index = match styles
                        .iter()
                        .position(|style| style.name.eq_ignore_ascii_case(name))
                    {
                        Some(index) => index,
                        None => {
                            styles.push(plain_style(name));
                            styles.len() - 1
                        }
                    };
                    (index, rest.to_owned())
                }
                None => (0, line),
            }
        } else {
            (0, line)
        };
        blocks.push(Block::Paragraph(paragraph(style, &rest)));
    }
    if blocks.is_empty() {
        blocks.push(Block::Paragraph(Paragraph::default()));
    }
    ExchangeDocument {
        styles,
        sections: vec![Section {
            page: letter(),
            new_page: false,
            headers: Default::default(),
            footers: Default::default(),
            page_numbers: None,
            blocks,
        }],
        ..ExchangeDocument::default()
    }
}

/// A paragraph of plain text: tabs as tabs, a form feed as a page break.
fn paragraph(style: usize, text: &str) -> Paragraph {
    let mut inlines = Vec::new();
    let mut run = String::new();
    let flush = |run: &mut String, inlines: &mut Vec<Inline>| {
        if !run.is_empty() {
            inlines.push(Inline::Text {
                text: std::mem::take(run),
                props: CharProps::default(),
            });
        }
    };
    for character in text.chars() {
        match character {
            '\t' => {
                flush(&mut run, &mut inlines);
                inlines.push(Inline::Tab {
                    props: CharProps::default(),
                });
            }
            '\u{C}' => {
                flush(&mut run, &mut inlines);
                inlines.push(Inline::PageBreak);
            }
            character if character.is_control() => {}
            character => run.push(character),
        }
    }
    flush(&mut run, &mut inlines);
    Paragraph {
        style,
        inlines,
        ..Paragraph::default()
    }
}

/// The file's lines, at a CR/LF, a LF, or a CR.
fn split_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                lines.push(&text[start..index]);
                if bytes.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
                start = index + 1;
            }
            b'\n' => {
                lines.push(&text[start..index]);
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    lines.push(&text[start..]);
    lines
}

/// Ami Pro's CR/LF at lines (`15e0:04f2`, VM IMT): lines joined with a
/// space into paragraphs that a blank line ends; a line that starts with a
/// tab starts a paragraph (the tab dropped); a line that starts with spaces
/// keeps them in place of the joining space; a word hyphenated at a line's
/// end joined. (Ami Pro's leading space after a second blank line is not
/// copied.)
fn joined(lines: &[&str]) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut current: Option<String> = None;
    for line in lines {
        if line.trim().is_empty() {
            if let Some(done) = current.take() {
                paragraphs.push(done);
            }
            continue;
        }
        let line = line.trim_end_matches([' ', '\t']);
        if line.starts_with('\t')
            && let Some(done) = current.take()
        {
            paragraphs.push(done);
        }
        match &mut current {
            None => current = Some(line.trim_start_matches('\t').to_owned()),
            Some(text) => {
                let starts_blank = line.starts_with(' ');
                let hyphenated = text.ends_with('-')
                    && text[..text.len() - 1]
                        .chars()
                        .next_back()
                        .is_some_and(char::is_alphabetic)
                    && line.chars().next().is_some_and(char::is_lowercase);
                if hyphenated {
                    text.pop();
                } else if !starts_blank {
                    text.push(' ');
                }
                text.push_str(line);
            }
        }
    }
    paragraphs.extend(current);
    if paragraphs.is_empty() {
        paragraphs.push(String::new());
    }
    paragraphs
}

/// `@Name@` or `<Name>` at a paragraph's start (`15e0:0827`): the name and
/// the text after it.
fn style_name(line: &str) -> Option<(&str, &str)> {
    let (open, close) = match line.chars().next()? {
        '@' => ('@', '@'),
        '<' => ('<', '>'),
        _ => return None,
    };
    let rest = &line[open.len_utf8()..];
    let end = rest.find(close)?;
    let name = rest[..end].trim();
    (!name.is_empty() && name.len() <= 64).then(|| (name, &rest[end + close.len_utf8()..]))
}

/// A style the text names: Body Text's look (the document defines it).
fn plain_style(name: &str) -> Style {
    Style {
        name: name.to_owned(),
        based_on: None,
        next: None,
        paragraph: ParagraphProps::default(),
        character: CharProps::default(),
        outline_level: None,
        list: None,
    }
}

/// Ami Pro's new document's page: Letter with one-inch margins.
pub(crate) fn letter() -> PageSetup {
    PageSetup {
        width: 12_240,
        height: 15_840,
        landscape: false,
        margin_top: 1_440,
        margin_bottom: 1_440,
        margin_left: 1_440,
        margin_right: 1_440,
        header_distance: 792,
        footer_distance: 792,
        columns: Columns {
            count: 1,
            gap: 720,
            widths: None,
            line_between: false,
        },
        border: None,
    }
}

/// The characters of `bytes`.
pub fn decode(bytes: &[u8], characters: Characters) -> String {
    match characters {
        Characters::Detect => match bytes {
            [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
            [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
            [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
            _ => match std::str::from_utf8(bytes) {
                Ok(text) => text.to_owned(),
                Err(_) => amipro_document::decode_windows_1252(bytes),
            },
        },
        Characters::Utf8 => {
            let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
            String::from_utf8_lossy(bytes).into_owned()
        }
        Characters::Utf16Le => utf16(
            bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes),
            u16::from_le_bytes,
        ),
        Characters::Utf16Be => utf16(
            bytes.strip_prefix(&[0xFE, 0xFF]).unwrap_or(bytes),
            u16::from_be_bytes,
        ),
        Characters::WindowsAnsi => amipro_document::decode_windows_1252(bytes),
        Characters::PcAscii => bytes.iter().map(|byte| cp437(*byte)).collect(),
        Characters::Ascii7 => bytes.iter().map(|byte| char::from(byte & 0x7F)).collect(),
    }
}

fn utf16(bytes: &[u8], word: fn([u8; 2]) -> u16) -> String {
    char::decode_utf16(bytes.chunks_exact(2).map(|pair| word([pair[0], pair[1]])))
        .map(|unit| unit.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// Code page 437's characters from 0x80 (the IBM PC's, Ami Pro's
/// PC-ASCII).
const CP437_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', //
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', //
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', //
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐', //
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧', //
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀', //
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', //
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{A0}',
];

fn cp437(byte: u8) -> char {
    if byte < 0x80 {
        char::from(byte)
    } else {
        CP437_HIGH[usize::from(byte - 0x80)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(document: &ExchangeDocument) -> Vec<(String, String)> {
        document.sections[0]
            .blocks
            .iter()
            .map(|block| match block {
                Block::Paragraph(paragraph) => (
                    document.styles[paragraph.style].name.clone(),
                    paragraph
                        .inlines
                        .iter()
                        .map(|inline| match inline {
                            Inline::Text { text, .. } => text.clone(),
                            Inline::Tab { .. } => "\t".into(),
                            Inline::PageBreak => "\u{C}".into(),
                            _ => String::new(),
                        })
                        .collect(),
                ),
                Block::Table(_) => unreachable!(),
            })
            .collect()
    }

    fn plain(document: &ExchangeDocument) -> Vec<String> {
        texts(document).into_iter().map(|(_, text)| text).collect()
    }

    #[test]
    fn every_line_is_a_paragraph() {
        let options = TextOptions::default();
        let document = read_text(b"one\r\ntwo\r\n\r\n\tthree\r\n", &options);
        assert_eq!(plain(&document), ["one", "two", "", "\tthree"]);
        let document = read_text(b"a\nb\rc", &options);
        assert_eq!(plain(&document), ["a", "b", "c"]);
    }

    /// Ami Pro's CR/LF at lines on the lines of VM fixture IM2 (IMT).
    #[test]
    fn joined_lines_follow_ami_pro() {
        let options = TextOptions {
            line_ends: LineEnds::Joined,
            ..TextOptions::default()
        };
        let document = read_text(
            b"Line one of a paragraph\r\nline two of the same paragraph\r\n\r\nSecond paragraph after a blank line\r\n\tIndented with a tab\r\n    Indented with four spaces\r\n\r\n\r\nAfter two blank lines\r\n",
            &options,
        );
        assert_eq!(
            plain(&document),
            [
                "Line one of a paragraph line two of the same paragraph",
                "Second paragraph after a blank line",
                "Indented with a tab    Indented with four spaces",
                "After two blank lines",
            ]
        );
        let document = read_text(b"a hyphen-\nated word, well-\nKnown", &options);
        assert_eq!(plain(&document), ["a hyphenated word, well- Known"]);
    }

    /// VM fixture IM3's lines with paragraph ends only and Keep style names
    /// (IMT3), read as PC-ASCII.
    #[test]
    fn style_names_and_pc_ascii() {
        let options = TextOptions {
            keep_style_names: true,
            characters: Characters::PcAscii,
            ..TextOptions::default()
        };
        let document = read_text(
            b"@Title@A titled line\r\nA plain line\r\n\tA tabbed line\r\n\r\n@Subhead@Sub line\r\nLast line \x82\r\n",
            &options,
        );
        assert_eq!(
            texts(&document),
            [
                ("Title".into(), "A titled line".into()),
                ("Body Text".into(), "A plain line".into()),
                ("Body Text".into(), "\tA tabbed line".into()),
                ("Body Text".into(), String::new()),
                ("Subhead".into(), "Sub line".into()),
                ("Body Text".into(), "Last line é".into()),
            ]
        );
    }

    #[test]
    fn characters_are_detected() {
        assert_eq!(decode(b"caf\xc3\xa9", Characters::Detect), "café");
        assert_eq!(decode(b"caf\xe9", Characters::Detect), "café");
        assert_eq!(
            decode(&[0xFF, 0xFE, b'a', 0, 0xE9, 0], Characters::Detect),
            "aé"
        );
        assert_eq!(decode(b"\xe9", Characters::PcAscii), "Θ");
        assert_eq!(decode(b"\xe9", Characters::Ascii7), "i");
    }
}
