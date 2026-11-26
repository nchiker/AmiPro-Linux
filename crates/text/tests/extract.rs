//! Synthetic inputs only: the SAM fixture is the hand-written one in `ami-formats/tests/fixtures`; native files are
//! made here with the model; bad inputs are generated.
use amipro_document::{Document, default_styles};
use amipro_text::{Extracted, Kind, Limits, Reason, VERSION, extract_text, sniff};
use std::panic::catch_unwind;

const SAM: &[u8] = include_bytes!("../../ami-formats/tests/fixtures/pages-and-frames.sam");

fn native(title: &str, paragraphs: &[&str]) -> Vec<u8> {
    let mut document = Document::empty(title, default_styles());
    let style = *document.styles().keys().next().unwrap();
    for text in paragraphs {
        document.push_paragraph(*text, style);
    }
    document.encode_native().unwrap().into_bytes()
}

fn text_of(result: Extracted) -> (Option<String>, String, bool) {
    match result {
        Extracted::Text {
            title,
            text,
            truncated,
        } => (title, text, truncated),
        other => panic!("expected text, got {other:?}"),
    }
}

/// Runs the case plainly and inside `catch_unwind`, which must not catch anything.
fn safe(bytes: &[u8], limits: &Limits) -> Extracted {
    let caught = catch_unwind(|| extract_text(bytes, limits));
    caught.expect("extract_text panicked")
}

#[test]
fn version_is_one() {
    assert_eq!(VERSION, 1);
    assert_eq!(Limits::default().max_bytes, 8 << 20);
    assert_eq!(Limits::default().max_chars, 1_000_000);
}

#[test]
fn sniffs_by_content() {
    assert_eq!(sniff(&native("T", &["a"])), Kind::Native);
    assert_eq!(sniff(b"AMIPRO-LINUX/3"), Kind::Native);
    assert_eq!(sniff(b"AMIPRO-LINUX/30\n"), Kind::NotAmiPro);
    assert_eq!(sniff(SAM), Kind::Sam);
    assert_eq!(sniff(b"[ver]\r\n\t4\r\n"), Kind::Sam);
    assert_eq!(sniff(b"[encrypt]\r\nxx"), Kind::Protected);
    assert_eq!(sniff(b"[encrypt]\nxx"), Kind::Protected);
    assert_eq!(sniff(b"[encryptx]\n"), Kind::NotAmiPro);
    assert_eq!(sniff(b""), Kind::NotAmiPro);
    assert_eq!(sniff(b"hello"), Kind::NotAmiPro);
    assert_eq!(sniff(b"WordPro\0\0"), Kind::NotAmiPro);
}

#[test]
fn native_text_and_title() {
    let bytes = native("My title", &["First line", "Second line"]);
    let (title, text, truncated) = text_of(safe(&bytes, &Limits::default()));
    assert_eq!(title.as_deref(), Some("My title"));
    assert_eq!(text, "First line\nSecond line");
    assert!(!truncated);
    let (title, ..) = text_of(safe(&native("Untitled", &["x"]), &Limits::default()));
    assert_eq!(title, None);
}

#[test]
fn sam_with_frames_and_table_reads_main_text_first() {
    let (title, text, truncated) = text_of(safe(SAM, &Limits::default()));
    assert_eq!(title, None);
    assert!(!truncated);
    let anchored = text.find("Anchored").expect("anchored frame text");
    let grouped = text.find("Grouped").expect("grouped frame text");
    assert!(text.lines().count() > 3, "{text}");
    // The body (first lines) comes before the frame flows.
    let first_line_end = text.find('\n').unwrap();
    assert!(
        anchored > first_line_end && grouped > first_line_end,
        "{text}"
    );
    assert!(!text.contains("\n\n"), "blank flows are left out: {text:?}");
    // Stable for the same input.
    assert_eq!(text_of(safe(SAM, &Limits::default())).1, text);
}

#[test]
fn protected_never_gives_text() {
    let protected = amipro_formats::protect::protect(SAM, "secret");
    assert_eq!(sniff(&protected), Kind::Protected);
    assert_eq!(safe(&protected, &Limits::default()), Extracted::Protected);
    assert_eq!(
        safe(b"[encrypt]\n", &Limits::default()),
        Extracted::Protected
    );
}

#[test]
fn empty_and_random_bytes_are_not_ami_pro() {
    let limits = Limits::default();
    assert_eq!(safe(b"", &limits), Extracted::Unreadable(Reason::NotAmiPro));
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    for length in [1usize, 7, 64, 4096, 100_000] {
        let bytes: Vec<u8> = (0..length)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 24) as u8
            })
            .collect();
        assert_eq!(
            safe(&bytes, &limits),
            Extracted::Unreadable(Reason::NotAmiPro)
        );
        // Random bytes behind a valid first line must not panic either.
        for head in [&b"[ver]\n"[..], b"AMIPRO-LINUX/3\n"] {
            let mut mixed = head.to_vec();
            mixed.extend_from_slice(&bytes);
            assert!(matches!(
                safe(&mixed, &limits),
                Extracted::Unreadable(Reason::Damaged) | Extracted::Text { .. }
            ));
        }
    }
}

#[test]
fn truncated_files_do_not_panic() {
    let limits = Limits::default();
    let bytes = native("T", &["one", "two", "three"]);
    assert_eq!(
        safe(&bytes[..bytes.len() / 2], &limits),
        Extracted::Unreadable(Reason::Damaged)
    );
    for cut in (0..SAM.len())
        .step_by(97)
        .chain([SAM.len() / 2, SAM.len() - 1])
    {
        let result = safe(&SAM[..cut], &limits);
        assert!(!matches!(result, Extracted::Protected), "cut {cut}");
    }
    // Cut inside a UTF-8 sequence.
    let mut utf8 = native("T", &["caf\u{e9}"]);
    let at = utf8.iter().position(|b| *b == 0xC3).unwrap();
    utf8.truncate(at + 1);
    assert_eq!(safe(&utf8, &limits), Extracted::Unreadable(Reason::Damaged));
}

#[test]
fn over_max_bytes_is_too_large() {
    let limits = Limits {
        max_bytes: SAM.len() - 1,
        max_chars: 100,
    };
    assert_eq!(safe(SAM, &limits), Extracted::Unreadable(Reason::TooLarge));
    let limits = Limits {
        max_bytes: SAM.len(),
        max_chars: 100,
    };
    assert!(matches!(safe(SAM, &limits), Extracted::Text { .. }));
    // Size is checked before anything else, even for protected or foreign bytes.
    let small = Limits {
        max_bytes: 3,
        max_chars: 10,
    };
    assert_eq!(
        safe(b"[encrypt]\n", &small),
        Extracted::Unreadable(Reason::TooLarge)
    );
}

#[test]
fn max_chars_cuts_on_a_char_boundary() {
    let bytes = native("T", &["h\u{e9}llo w\u{f6}rld", "second \u{4e16}\u{754c}"]);
    let all = text_of(safe(&bytes, &Limits::default())).1;
    for cut in 0..=all.chars().count() + 1 {
        let limits = Limits {
            max_bytes: usize::MAX,
            max_chars: cut,
        };
        let (_, text, truncated) = text_of(safe(&bytes, &limits));
        let expected: String = all.chars().take(cut).collect();
        assert_eq!(text, expected, "cut {cut}");
        assert_eq!(truncated, cut < all.chars().count(), "cut {cut}");
    }
}

fn sam_with(extra: &str) -> Vec<u8> {
    let text = std::str::from_utf8(SAM).unwrap();
    text.replacen("[ver]\n\t4\n", &format!("[ver]\n\t4\n{extra}"), 1)
        .into_bytes()
}

#[test]
fn headers_claiming_huge_counts_do_not_allocate_or_panic() {
    let limits = Limits::default();
    let big = ["4294967295", "2147483648", "99999999999999999999", "-1"];
    for count in big {
        // A native file with huge counts in place of real ones.
        let good = String::from_utf8(native("T", &["a", "b"])).unwrap();
        let mut broken = Vec::new();
        for line in good.lines() {
            broken.push(
                line.split('\t')
                    .map(|field| {
                        if field.parse::<u64>().is_ok_and(|n| n > 0 && n < 50) {
                            count
                        } else {
                            field
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\t"),
            );
        }
        let broken = broken.join("\n");
        assert!(!matches!(
            safe(broken.as_bytes(), &limits),
            Extracted::Protected
        ));
        // SAM: every small number in the fixture replaced.
        let text = std::str::from_utf8(SAM).unwrap();
        let replaced: Vec<String> = text
            .lines()
            .map(|line| {
                let t = line.trim();
                if t.parse::<u32>().is_ok_and(|n| (2..200).contains(&n)) {
                    format!("\t{count}")
                } else {
                    line.to_owned()
                }
            })
            .collect();
        assert!(!matches!(
            safe(replaced.join("\n").as_bytes(), &limits),
            Extracted::Protected
        ));
    }
    assert!(!matches!(
        safe(&sam_with("[frm]\n\t4294967295\n"), &limits),
        Extracted::Protected
    ));
}

#[test]
fn deep_nesting_and_long_lines_do_not_panic() {
    let limits = Limits::default();
    let mut deep = String::from("[ver]\n\t4\n[edoc]\n");
    for _ in 0..20_000 {
        deep.push_str("<[><[><{><(>\n");
    }
    assert!(!matches!(
        safe(deep.as_bytes(), &limits),
        Extracted::Protected
    ));
    let mut nested = String::from("AMIPRO-LINUX/3\n");
    nested.push_str(&"frame\t".repeat(50_000));
    assert!(!matches!(
        safe(nested.as_bytes(), &limits),
        Extracted::Protected
    ));
    let mut long = b"[ver]\n\t4\n[edoc]\n".to_vec();
    long.extend(std::iter::repeat_n(b'a', 3 << 20));
    assert!(matches!(
        safe(
            &long,
            &Limits {
                max_bytes: 8 << 20,
                max_chars: 10
            }
        ),
        Extracted::Unreadable(Reason::Damaged) | Extracted::Text { .. }
    ));
}

#[test]
fn non_utf8_native_is_damaged() {
    let mut bytes = b"AMIPRO-LINUX/3\n".to_vec();
    bytes.extend_from_slice(&[0xFF, 0xFE, 0x00]);
    assert_eq!(
        safe(&bytes, &Limits::default()),
        Extracted::Unreadable(Reason::Damaged)
    );
}
