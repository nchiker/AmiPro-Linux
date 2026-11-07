//! Spell Check's walk and Replace All on documents (`UI_SPEC.md` §35;
//! `docs/research/PROOFING_NOTES.md`; VM PSP1).

use amipro_document::Document;
use amipro_editing::proofing::spelling::{
    Ignored, ProblemKind, SpellOptions, Speller, check_streams, next_problem, replace_all,
};
use amipro_editing::{TextPosition, apply};
use amipro_formats::import_sam;

struct List(&'static [&'static str]);

impl Speller for List {
    fn check(&self, word: &str) -> bool {
        self.0.iter().any(|known| known.eq_ignore_ascii_case(word))
    }
    fn suggest(&self, _: &str) -> Vec<String> {
        Vec::new()
    }
}

const WORDS: &[&str] = &[
    "the", "cat", "sat", "on", "mat", "we", "receive", "and", "today", "another", "line", "with",
    "word",
];

fn psp1() -> Document {
    let sam = "[ver]\r\n\t4\r\n[edoc]\r\nTeh cat sat on the the mat. We recieve and Xyzzy today.\r\n\r\nAnother Teh line with teh word.\r\n\r\n>\r\n\r\n";
    import_sam(sam.as_bytes(), None, "PSP1").unwrap().document
}

fn at(document: &Document, paragraph: usize, byte: usize) -> TextPosition {
    TextPosition {
        flow: document.main_flow_id(),
        paragraph,
        byte,
    }
}

fn found(document: &Document, from: TextPosition, ignored: &Ignored) -> Vec<(String, ProblemKind)> {
    let options = SpellOptions::default();
    let streams = check_streams(document, from.flow, &options);
    let mut at = from;
    let mut all = Vec::new();
    while let Some(problem) = next_problem(
        document,
        &streams,
        at,
        None,
        &List(WORDS),
        &options,
        ignored,
    ) {
        at = problem.end;
        all.push((problem.word, problem.kind));
    }
    all
}

#[test]
fn a_check_walks_from_the_caret_to_the_end_without_wrapping_psp1() {
    let document = psp1();
    let all = found(&document, at(&document, 0, 0), &Ignored::default());
    assert_eq!(
        all.iter()
            .map(|(word, _)| word.as_str())
            .collect::<Vec<_>>(),
        ["Teh", "the", "recieve", "Xyzzy", "Teh", "teh"]
    );
    assert_eq!(all[1].1, ProblemKind::Repeated);
    // From the second paragraph: the first is never checked.
    assert_eq!(
        found(&document, at(&document, 1, 0), &Ignored::default()).len(),
        2
    );
    // Skip All: the word as written, for the rest of the session.
    let mut ignored = Ignored::default();
    ignored.add("Teh");
    assert_eq!(
        found(&document, at(&document, 0, 0), &ignored)
            .iter()
            .map(|(word, _)| word.as_str())
            .collect::<Vec<_>>(),
        ["the", "recieve", "Xyzzy", "teh"]
    );
}

#[test]
fn a_check_of_a_selection_stops_at_its_end_psp2() {
    let document = psp1();
    let options = SpellOptions::default();
    let streams = check_streams(&document, document.main_flow_id(), &options);
    let until = Some(at(&document, 0, 26));
    let first = next_problem(
        &document,
        &streams,
        at(&document, 0, 4),
        until,
        &List(WORDS),
        &options,
        &Ignored::default(),
    )
    .unwrap();
    assert_eq!(first.word, "the");
    assert!(
        next_problem(
            &document,
            &streams,
            first.end,
            until,
            &List(WORDS),
            &options,
            &Ignored::default()
        )
        .is_none()
    );
}

#[test]
fn replace_all_replaces_every_occurrence_keeping_its_capitals_in_one_step_sp4() {
    let document = psp1();
    let streams = check_streams(&document, document.main_flow_id(), &SpellOptions::default());
    let (command, count) = replace_all(&document, &streams, None, "Teh", "The").unwrap();
    assert_eq!(count, 3);
    let replaced = apply(&document, &command).unwrap().document;
    let texts: Vec<&str> = replaced
        .main_flow()
        .paragraphs
        .iter()
        .map(|paragraph| paragraph.text.as_str())
        .collect();
    assert_eq!(
        texts,
        [
            "The cat sat on the the mat. We recieve and Xyzzy today.",
            "Another The line with the word."
        ]
    );
}

#[test]
fn a_check_goes_on_through_the_other_streams_and_ends() {
    // A word in a frame after the main text's: once the check is in the
    // frame, the main text is not searched again (it once went round).
    use amipro_editing::{Command, NewFrameContent, apply};
    let document = psp1();
    let main = document.main_flow_id();
    let document = apply(
        &document,
        &Command::CreateFrame {
            placement: amipro_document::Placement::Inline {
                y_offset: 0,
                size: amipro_document::Size {
                    width: 4320,
                    height: 1440,
                },
            },
            style: amipro_document::FrameStyle::default(),
            content: NewFrameContent::Text,
            anchor: Some(TextPosition {
                flow: main,
                paragraph: 0,
                byte: 3,
            }),
            layout: None,
        },
    )
    .unwrap()
    .document;
    let options = SpellOptions::default();
    let streams = check_streams(&document, document.main_flow_id(), &options);
    assert!(streams.len() >= 2);
    let frame_flow = streams[1];
    let typed = apply(
        &document,
        &Command::ReplaceText {
            start: TextPosition {
                flow: frame_flow,
                paragraph: 0,
                byte: 0,
            },
            end: TextPosition {
                flow: frame_flow,
                paragraph: 0,
                byte: 0,
            },
            text: "Frameword zzqq".into(),
        },
    )
    .unwrap()
    .document;
    let mut at = TextPosition {
        flow: typed.main_flow_id(),
        paragraph: 0,
        byte: 0,
    };
    let mut seen = Vec::new();
    while let Some(problem) = next_problem(
        &typed,
        &streams,
        at,
        None,
        &List(WORDS),
        &options,
        &Ignored::default(),
    ) {
        assert!(seen.len() < 20, "the check goes round: {seen:?}");
        seen.push(problem.word.clone());
        at = problem.end;
    }
    assert_eq!(seen.last().map(String::as_str), Some("zzqq"));
}
