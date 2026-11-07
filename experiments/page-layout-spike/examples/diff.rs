use cosmic_text::{Attrs, AttrsList, Family, FontSystem};
use page_layout_spike::*;
fn main() {
    let mut fonts = FontSystem::new();
    let list = AttrsList::new(&Attrs::new().family(Family::Name(
        &std::env::args().nth(2).unwrap_or("Liberation Serif".into()),
    )));
    let path = std::env::args().nth(1).unwrap();
    let width: f32 = std::env::args()
        .nth(3)
        .map_or(624.0, |w| w.parse().unwrap());
    let texts: Vec<String> = if path.ends_with(".SAM") {
        amipro_formats::import_sam_path(&path)
            .unwrap()
            .document
            .paragraphs()
            .iter()
            .map(|p| p.text.clone())
            .collect()
    } else {
        vec![path.clone()]
    };
    let mut shown = 0;
    for text in texts
        .iter()
        .filter(|t| !t.contains(['\u{FFFC}', '\u{2028}', '\u{AD}']))
    {
        let line = shape(&mut fonts, text, &list);
        let words = logical_words(&line, text, 16.0, 0.0);
        let ours = break_lines(&words, |_| width, 0.0);
        let reference = reference_lines(&line, 16.0, width);
        let ours_ranges: Vec<_> = ours
            .iter()
            .map(|l| covered(&place(&line, &words, l, 16.0)))
            .collect();
        let ref_ranges: Vec<_> = reference.iter().map(|l| covered(l)).collect();
        if ours_ranges != ref_ranges && shown < 4 {
            shown += 1;
            println!("TEXT {:?}", text);
            println!(
                "  ours {:?}",
                ours_ranges
                    .iter()
                    .map(|r| r.clone().map(|r| &text[r]))
                    .collect::<Vec<_>>()
            );
            println!(
                "  ref  {:?}",
                ref_ranges
                    .iter()
                    .map(|r| r.clone().map(|r| &text[r]))
                    .collect::<Vec<_>>()
            );
            for w in &words {
                if w.blank {
                    print!("[{:?} {:.1}]", &text[w.bytes.clone()], w.width)
                } else {
                    print!("{:?}{:.1} ", &text[w.bytes.clone()], w.width)
                }
            }
            println!();
        }
    }
}
