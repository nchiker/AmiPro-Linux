//! Writing Ami Pro style records (decision D3, Save as a Style Sheet): a
//! style as the `[tag]` record Ami Pro's writer makes (decompile
//! `11c8:26b7`), in its field order and number formats, with its text
//! escaping (`11c8:0bf5`). Every value the reader takes is written back,
//! and the values the model does not use come from `ParagraphStyle::kept`.

use std::collections::BTreeMap;

use amipro_document::{
    Alignment, Caps, LineSpacing, ParagraphStyle, StyleId, TabKind, TabLeader, TabStop,
};

/// Ami Pro's longest style name: it cuts names to 13 characters
/// (`1000:80cd`, `1318:21b2`), and a longer name in a file is a style it
/// cannot find (twin AIM1, 2026-10-24).
pub const STYLE_NAME_LENGTH: usize = 13;

/// `document` with every style name longer than [`STYLE_NAME_LENGTH`]
/// shortened, each still unique (`Body Text (fi`, then `Body Text ( 2`);
/// `None` when no name is too long. Save As SAM and Save as a Style Sheet
/// write this copy (IM4).
pub fn shortened_style_names(
    document: &amipro_document::Document,
) -> Option<amipro_document::Document> {
    let long = |name: &str| name.chars().count() > STYLE_NAME_LENGTH;
    if !document.styles().values().any(|style| long(&style.name)) {
        return None;
    }
    let mut taken: Vec<String> = document
        .styles()
        .values()
        .filter(|style| !long(&style.name))
        .map(|style| style.name.to_lowercase())
        .collect();
    let mut shortened = document.clone();
    for style in shortened.styles_mut().values_mut() {
        if !long(&style.name) {
            continue;
        }
        let cut = |length: usize| -> String {
            style
                .name
                .chars()
                .take(length)
                .collect::<String>()
                .trim_end()
                .to_owned()
        };
        let mut name = cut(STYLE_NAME_LENGTH);
        let mut number = 2;
        while taken.contains(&name.to_lowercase()) {
            let suffix = format!(" {number}");
            name = format!("{}{suffix}", cut(STYLE_NAME_LENGTH - suffix.len()));
            number += 1;
        }
        taken.push(name.to_lowercase());
        style.name = name;
    }
    Some(shortened)
}

/// How many style names Save As SAM shortens.
pub fn long_style_names(document: &amipro_document::Document) -> usize {
    document
        .styles()
        .values()
        .filter(|style| style.name.chars().count() > STYLE_NAME_LENGTH)
        .count()
}

/// `character` in Windows-1252, or `?` where it has no byte.
pub(crate) fn windows_1252(character: char) -> u8 {
    if character == crate::text::AMI_PRO_HYPHEN {
        return 0xAD;
    }
    amipro_document::windows_1252(character)
}

/// Text as Ami Pro writes it in a record (`11c8:0bf5`): Windows-1252, with
/// control characters (a tab aside) as `<*X>` (X − 0x20), bytes 0x80–0xBF as
/// `</X>` (X + 0x40), bytes from 0xC0 as `<\X>` (X − 0x80), and `<` doubled.
pub fn escape_text(text: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    for character in text.chars() {
        let byte = windows_1252(character);
        match byte {
            b'\t' => out.push(byte),
            0x00..=0x1F => out.extend([b'<', b'*', byte + 0x20, b'>']),
            0x80..=0xBF => out.extend([b'<', b'/', byte - 0x40, b'>']),
            0xC0..=0xFF => out.extend([b'<', b'\\', byte.wrapping_add(0x80), b'>']),
            b'<' => out.extend(b"<<"),
            _ => out.push(byte),
        }
    }
    out
}

/// A control's argument bytes as Ami Pro's writer escapes them
/// (`11b0:05cf`, `11b0:120f`): a tab as it is, other bytes below 0x20 as
/// `<*X>`, `<` doubled, `>` as `<;>`, 0x80 to 0xBF as `</X>`, 0xC0 to 0xFF
/// as `<\X>`.
pub(crate) fn escape_control_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    for &byte in bytes {
        match byte {
            b'\t' => out.push(byte),
            0x00..=0x1F => out.extend([b'<', b'*', byte + 0x20, b'>']),
            0x80..=0xBF => out.extend([b'<', b'/', byte - 0x40, b'>']),
            0xC0..=0xFF => out.extend([b'<', b'\\', byte.wrapping_add(0x80), b'>']),
            b'<' => out.extend(b"<<"),
            b'>' => out.extend(b"<;>"),
            _ => out.push(byte),
        }
    }
}

/// `[fnt]` value 4: the attributes as flags, with the face-kind bits kept.
fn font_flags(style: &ParagraphStyle) -> u32 {
    let attributes = style.attributes;
    let mut flags = u32::from(style.kept.face_bits);
    for (on, bit) in [
        (style.bold, 1),
        (style.italic, 2),
        (attributes.underline, 4),
        (attributes.word_underline, 8),
        (attributes.caps == Some(Caps::Upper), 16),
        (attributes.caps == Some(Caps::Small), 32),
        (attributes.double_underline, 64),
        (attributes.strikethrough, 128),
        (attributes.superscript, 256),
        (attributes.subscript, 512),
        (attributes.caps == Some(Caps::Lower), 1024),
        (attributes.caps == Some(Caps::Initial), 2048),
        (attributes.protected, 4096),
    ] {
        if on {
            flags |= bit;
        }
    }
    flags
}

/// A tab stop's type as written: 1 left, 2 centre, 3 right, 4 decimal, the
/// leader in the top two bits.
pub(crate) fn tab_type(stop: &TabStop) -> u16 {
    let kind = match stop.kind {
        TabKind::Left => 1,
        TabKind::Centre => 2,
        TabKind::Right => 3,
        TabKind::Decimal => 4,
    };
    let leader = match stop.leader {
        TabLeader::None => 0,
        TabLeader::Hyphens => 1,
        TabLeader::Dots => 2,
        TabLeader::Line => 3,
    };
    kind | leader << 14
}

/// Ami Pro's record lines: each value on its own line, indented by one tab
/// per nesting level, ending in CR LF (the writer's `\t%u\r\n` formats).
#[derive(Default)]
pub(crate) struct Lines(pub Vec<u8>);

impl Lines {
    pub(crate) fn line(&mut self, depth: usize, value: &[u8]) {
        self.0.extend(std::iter::repeat_n(b'\t', depth));
        self.0.extend_from_slice(value);
        self.0.extend(b"\r\n");
    }

    pub(crate) fn number(&mut self, depth: usize, value: i64) {
        self.line(depth, value.to_string().as_bytes());
    }

    /// Text escaped as a record value.
    pub(crate) fn text(&mut self, depth: usize, value: &str) {
        self.line(depth, &escape_text(value));
    }
}

/// `style` as Ami Pro's `[tag]` record, lines ending in CR LF. `styles`
/// names its next style.
pub fn style_record(style: &ParagraphStyle, styles: &BTreeMap<StyleId, ParagraphStyle>) -> Vec<u8> {
    let mut lines = Lines::default();
    write_style_record(&mut lines, style, styles);
    lines.0
}

pub(crate) fn write_style_record(
    out: &mut Lines,
    style: &ParagraphStyle,
    styles: &BTreeMap<StyleId, ParagraphStyle>,
) {
    let mut line = |depth: usize, value: &[u8]| out.line(depth, value);
    let number = |value: i64| value.to_string().into_bytes();
    let kept = &style.kept;
    line(0, b"[tag]");
    line(1, &escape_text(&style.name));
    line(1, &number(kept.function_key.into()));
    line(1, b"[fnt]");
    line(2, &escape_text(&style.font_family));
    line(2, &number((style.font_size_points * 20.0).round() as i64));
    line(2, &number(style.attributes.colour.into()));
    line(2, &number(font_flags(style).into()));
    let alignment = match style.alignment {
        Alignment::Left => 1,
        Alignment::Right => 2,
        Alignment::Centre => 4,
        Alignment::Justify => 8,
    } | if style.both_sides { 16 } else { 0 }
        | if style.hyphenation { 256 } else { 0 }
        | if style.hanging { 512 } else { 0 }
        | i64::from(kept.alignment_bits);
    line(1, b"[algn]");
    for value in [
        alignment,
        kept.units[0].into(),
        style.indents.all.into(),
        style.indents.first.into(),
        style.indents.rest.into(),
    ] {
        line(2, &number(value));
    }
    let spacing = &style.spacing;
    let (mode, height) = match spacing.line {
        LineSpacing::Single => (1, kept.single_height),
        LineSpacing::OneAndAHalf(height) => (2, height),
        LineSpacing::Double(height) => (4, height),
        LineSpacing::Custom(height) => (8, height),
    };
    let add_in = if !spacing.add_at_break {
        32
    } else if kept.add_in == 32 {
        16
    } else {
        kept.add_in
    };
    line(1, b"[spc]");
    for value in [
        i64::from(mode | add_in),
        height.into(),
        kept.units[1].into(),
        spacing.above.into(),
        spacing.below.into(),
        kept.units[2].into(),
        spacing.tightness.into(),
    ] {
        line(2, &number(value));
    }
    line(1, b"[brk]");
    line(2, &number(style.breaks.0.into()));
    let rules = &style.rules;
    line(1, b"[line]");
    for value in [
        i64::from(rules.flags()),
        rules.other_length.into(),
        rules.unknown[0].into(),
        rules.colour.into(),
        rules.unknown[1].into(),
        rules.above.line.into(),
        rules.below.line.into(),
        rules.above.space.into(),
        rules.below.space.into(),
        rules.unit.into(),
    ] {
        line(2, &number(value));
    }
    let bullets = &style.bullets;
    line(1, b"[spec]");
    line(2, &number(bullets.kept[0].into()));
    line(2, &number(bullets.level.into()));
    line(2, &escape_text(&bullets.text));
    for value in [
        bullets.kept[1],
        bullets.kept[2],
        bullets.kept[3],
        bullets.attributes.into(),
        bullets.flags.into(),
        bullets.kept[4],
        bullets.kept[5],
    ] {
        line(2, &number(value.into()));
    }
    line(1, b"[nfmt]");
    for value in &kept.number_format.to_values() {
        line(2, &escape_text(value));
    }
    let next = style
        .next
        .and_then(|next| styles.get(&next))
        .map(|next| next.name.as_str())
        .or(kept.next_name.as_deref())
        .unwrap_or(style.name.as_str());
    line(1, &escape_text(next));
    line(1, &number(style.tabs.len() as i64));
    for stop in &style.tabs {
        line(1, &number(tab_type(stop).into()));
        line(1, &number(stop.position.into()));
    }
    line(1, &number(style.indents.right.into()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LegacyKind, collect_style_specs, import_sam, parse};

    #[test]
    fn style_names_are_shortened_to_ami_pros_thirteen_characters() {
        let mut document = amipro_document::Document::default();
        let body = *document.styles().keys().next().unwrap();
        let add = |document: &mut amipro_document::Document, id: u64, name: &str| {
            let mut style = document.styles()[&body].clone();
            style.id = StyleId(id);
            style.name = name.into();
            document.styles_mut().insert(StyleId(id), style);
        };
        assert!(shortened_style_names(&document).is_none());
        add(&mut document, 50, "Body Text (file)");
        add(&mut document, 51, "Body Text (fi");
        add(&mut document, 52, "Body Text (first)");
        add(&mut document, 53, "Quotation, indented");
        let shortened = shortened_style_names(&document).unwrap();
        let names: Vec<&str> = [50, 51, 52, 53]
            .iter()
            .map(|id| shortened.styles()[&StyleId(*id)].name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "Body Text ( 2",
                "Body Text (fi",
                "Body Text ( 3",
                "Quotation, in"
            ]
        );
        assert_eq!(long_style_names(&document), 3);
        // Save As SAM writes the short names and lists them once (IM4).
        let options = crate::SamOptions::at(0);
        let losses = crate::sam_losses(&document, &options).unwrap();
        assert!(
            losses
                .iter()
                .any(|loss| loss.kind == crate::LossKind::StyleNames && loss.count == 3),
            "{losses:?}"
        );
        assert!(
            losses
                .iter()
                .all(|loss| loss.kind != crate::LossKind::Styles),
            "{losses:?}"
        );
        let copy = crate::read_back(&document, &options).unwrap();
        assert!(
            copy.styles()
                .values()
                .any(|style| style.name == "Quotation, in")
        );
    }
    use amipro_document::StyleOrigin;

    #[test]
    fn text_escapes_as_ami_pro_writes_it() {
        assert_eq!(escape_text("a<b>c\tdé"), b"a<<b>c\td<\\i>");
        assert_eq!(escape_text("\u{1E}."), b"<*>>.");
        assert_eq!(escape_text("®"), b"</n>");
        assert_eq!(escape_text("€"), b"</@>");
        assert_eq!(escape_text("\u{4E00}"), b"?");
    }

    #[test]
    fn a_style_record_is_written_back_as_it_was_read() {
        // A record in Ami Pro's own layout, with every section; tabs,
        // bullets, rules, and the values the model does not use.
        let record = "[tag]\r\n\tOutline 5\r\n\t8\r\n\t[fnt]\r\n\t\tTimes New Roman\r\n\t\t220\r\n\t\t255\r\n\t\t49153\r\n\
            \t[algn]\r\n\t\t273\r\n\t\t3\r\n\t\t1368\r\n\t\t432\r\n\t\t432\r\n\
            \t[spc]\r\n\t\t17\r\n\t\t259\r\n\t\t1\r\n\t\t28\r\n\t\t43\r\n\t\t1\r\n\t\t100\r\n\
            \t[brk]\r\n\t\t36\r\n\
            \t[line]\r\n\t\t9\r\n\t\t0\r\n\t\t1\r\n\t\t255\r\n\t\t1\r\n\t\t2\r\n\t\t2\r\n\t\t20\r\n\t\t20\r\n\t\t1\r\n\
            \t[spec]\r\n\t\t0\r\n\t\t5\r\n\t\t<*>>.\r\n\t\t28948\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t0\r\n\
            \t[nfmt]\r\n\t\t280\r\n\t\t3\r\n\t\t2\r\n\t\t.\r\n\t\t,\r\n\t\t\r\n\
            \tBody Text\r\n\t2\r\n\t1\r\n\t540\r\n\t49155\r\n\t9360\r\n\t1368\r\n";
        let sam = format!(
            "[ver]\r\n\t4\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n{record}[edoc]\r\n>\r\n"
        );
        let imported = import_sam(sam.as_bytes(), None, "Record").unwrap();
        let styles = imported.document.styles();
        let style = styles
            .values()
            .find(|style| style.name == "Outline 5")
            .unwrap();
        assert_eq!(style.origin, StyleOrigin::Document);
        assert_eq!(
            String::from_utf8(style_record(style, styles)).unwrap(),
            record
        );
        // The reader takes back what the writer wrote.
        let (legacy, _) = parse(&style_record(style, styles), LegacyKind::StyleSheet).unwrap();
        let mut specs = BTreeMap::new();
        collect_style_specs(&legacy, &mut specs, StyleOrigin::Sheet);
        assert_eq!(specs["outline 5"].kept, style.kept);
    }

    #[test]
    fn unknown_next_names_bracketed_texts_and_old_alignment_bits_survive() {
        // An outline style the sheet lacks as the next style, a bullet text
        // in brackets with a trailing space (written unescaped, so not a
        // section), and `[algn]` bits Ami Pro's reader clears (225 = 1 +
        // 32 + 64 + 128): all written back as read.
        let record = "[tag]\r\n\tOutline 1\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t280\r\n\t\t0\r\n\t\t16385\r\n\
            \t[algn]\r\n\t\t225\r\n\t\t1\r\n\t\t0\r\n\t\t360\r\n\t\t360\r\n\
            \t[spc]\r\n\t\t33\r\n\t\t273\r\n\t\t1\r\n\t\t0\r\n\t\t72\r\n\t\t1\r\n\t\t100\r\n\
            \t[brk]\r\n\t\t4\r\n\
            \t[line]\r\n\t\t8\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\t\t1\r\n\t\t1\r\n\t\t10\r\n\t\t10\r\n\t\t1\r\n\
            \t[spec]\r\n\t\t0\r\n\t\t1\r\n\t\t[<*:>] \r\n\t\t28948\r\n\t\t1\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t0\r\n\t\t0\r\n\
            \t[nfmt]\r\n\t\t272\r\n\t\t1\r\n\t\t2\r\n\t\t.\r\n\t\t,\r\n\t\t$\r\n\
            \tOutline I\r\n\t0\r\n\t0\r\n";
        let sam = format!("[ver]\r\n\t4\r\n{record}[edoc]\r\n>\r\n");
        let imported = import_sam(sam.as_bytes(), None, "Record").unwrap();
        let styles = imported.document.styles();
        let style = styles
            .values()
            .find(|style| style.name == "Outline 1")
            .unwrap();
        assert_eq!(style.bullets.text, "[\u{1A}] ");
        assert_eq!(style.kept.next_name.as_deref(), Some("Outline I"));
        assert_eq!(style.kept.alignment_bits, 224);
        assert_eq!(style.next, None);
        assert_eq!(
            String::from_utf8(style_record(style, styles)).unwrap(),
            record
        );
    }
}
