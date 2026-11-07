//! A master document's files joined into one export (MS10).

use amipro_document::contents::Sequence;
use amipro_document::{Document, StyleId};
use amipro_exchange::build::{BuildOptions, NoPictures, build};
use amipro_exchange::losses::Feature;
use amipro_exchange::set::{SetFile, join};
use amipro_exchange::{Block, ExchangeDocument, Format, Inline, Properties, docx, odt, rtf};

fn file(document: &Document) -> SetFile {
    let built = build(document, &BuildOptions::default(), &mut NoPictures);
    SetFile {
        document: built.document,
        features: built.features,
    }
}

fn member(text: &str, page_base: u32, footnotes: u32) -> Document {
    let mut document = Document::with_paragraphs([(text, StyleId(1))]);
    if page_base > 0 || footnotes > 0 {
        let mut contents = document.contents().clone();
        contents.sequence = Some(Sequence {
            page_base,
            footnotes,
            ..Sequence::default()
        });
        document.set_contents(contents);
    }
    document
}

fn texts(blocks: &[Block], out: &mut String) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                for inline in &paragraph.inlines {
                    if let Inline::Text { text, .. } = inline {
                        out.push_str(text);
                    }
                }
                out.push('\n');
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        texts(&cell.blocks, out);
                    }
                }
            }
        }
    }
}

fn all_text(document: &ExchangeDocument) -> String {
    let mut out = String::new();
    for section in &document.sections {
        texts(&section.blocks, &mut out);
    }
    out
}

#[test]
fn a_member_alone_shows_its_numbers_in_the_set() {
    let built = build(
        &member("Second", 2, 3),
        &BuildOptions::default(),
        &mut NoPictures,
    )
    .document;
    assert_eq!(built.sections[0].page_numbers.unwrap().start, Some(3));
    assert_eq!(built.settings.footnote_start, 4);
}

#[test]
fn the_files_join_in_order_each_a_section_with_its_own_numbers() {
    let first = member("First file", 0, 0);
    let second = member("Second file", 2, 0);
    let mut third = member("Third file", 4, 0);
    third
        .styles_mut()
        .get_mut(&StyleId(1))
        .unwrap()
        .font_size_points += 6.0;
    let (joined, features) = join(
        vec![file(&first), file(&second), file(&third)],
        Properties {
            title: "Master".into(),
            ..Properties::default()
        },
    );
    assert_eq!(joined.properties.title, "Master");
    let firsts: Vec<_> = joined
        .sections
        .iter()
        .enumerate()
        .filter(|(at, section)| *at == 0 || section.new_page)
        .map(|(_, section)| section.page_numbers.and_then(|numbers| numbers.start))
        .collect();
    assert_eq!(firsts, [None, Some(3), Some(5)]);
    // One style per name; the third file's Body Text differs.
    let names: Vec<_> = joined.styles.iter().map(|style| &style.name).collect();
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(names.len(), unique.len());
    assert!(features.count(Feature::DifferingStyles) >= 1);
    assert!(
        amipro_exchange::losses::losses(&features, Format::Docx)
            .iter()
            .any(|loss| loss.feature == Feature::DifferingStyles)
    );
    assert!(
        amipro_exchange::losses::losses(&features, Format::Text)
            .iter()
            .all(|loss| loss.feature != Feature::DifferingStyles)
    );
    for paragraph in joined
        .sections
        .iter()
        .flat_map(|section| &section.blocks)
        .filter_map(|block| match block {
            Block::Paragraph(paragraph) => Some(paragraph),
            Block::Table(_) => None,
        })
    {
        assert!(paragraph.style < joined.styles.len());
    }

    // Each format reads back with every file's text, in order.
    let deflate = |bytes: &[u8]| miniz_oxide::deflate::compress_to_vec(bytes, 6);
    let inflate = |data: &[u8], size: usize| -> Option<Vec<u8>> {
        miniz_oxide::inflate::decompress_to_vec_with_limit(data, size).ok()
    };
    let mut findings = amipro_exchange::import::Findings::default();
    let read = [
        amipro_exchange::read_docx::read_docx(
            &docx::write(&joined, Some(&deflate)),
            &inflate,
            &mut findings,
        )
        .unwrap(),
        amipro_exchange::read_odt::read_odt(
            &odt::write(&joined, Some(&deflate)),
            &inflate,
            &mut findings,
        )
        .unwrap(),
        amipro_exchange::read_rtf::read_rtf(&rtf::write(&joined), &mut findings).unwrap(),
    ];
    for document in read {
        let text = all_text(&document);
        let at = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{needle} in {text:?}"))
        };
        assert!(at("First file") < at("Second file"));
        assert!(at("Second file") < at("Third file"));
        let starts: Vec<_> = document
            .sections
            .iter()
            .filter_map(|section| section.page_numbers.and_then(|numbers| numbers.start))
            .collect();
        assert_eq!(starts, [3, 5], "the files' page numbers read back");
    }
}
