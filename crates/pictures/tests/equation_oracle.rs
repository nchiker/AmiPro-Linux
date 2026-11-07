//! Equations against Ami Pro's own (`docs/research/OBJECTS_NOTES.md` §12.2,
//! §15): the 27 equations of the EQV1 fixtures, read-only inputs, so these
//! tests are `#[ignore]`d and skip where the folder is absent (commands in
//! `docs/TEST_COMMANDS.md`). Nothing here keeps the fixtures' bytes.

use std::path::PathBuf;

use amipro_document::{FrameContent, PictureFormat, PictureSource};
use amipro_pictures::equation::{Params, Stored, layout};

struct Found {
    stored: Stored,
    bytes: Vec<u8>,
    width: i32,
    height: i32,
    baseline: i32,
}

fn folder() -> PathBuf {
    let base = std::env::var("AMIPRO_REFERENCE_DIR").map_or_else(
        |_| PathBuf::from("../../../AmiPro-Reference"),
        PathBuf::from,
    );
    base.join("vm-lab/OUT/objects")
}

fn found() -> Vec<Found> {
    let mut all = Vec::new();
    for name in ["EQV1A.SAM", "EQV1B.SAM"] {
        let path = folder().join(name);
        let Ok(import) = amipro_formats::import_sam_path(&path) else {
            continue;
        };
        let document = import.document;
        for frame in document.frames() {
            let FrameContent::Picture(picture) = &frame.content else {
                continue;
            };
            if picture.format() != PictureFormat::Equation {
                continue;
            }
            let PictureSource::Embedded(object) = &picture.source else {
                continue;
            };
            let Some(object) = document.objects().get(object) else {
                continue;
            };
            let Some(stored) = Stored::read(&object.bytes) else {
                continue;
            };
            let Some(isd) = picture.legacy.get("isd") else {
                continue;
            };
            let words: Vec<i32> = isd
                .split('\n')
                .map(|w| w.trim().parse::<i64>().map_or(0, |v| v as i32))
                .collect();
            let signed = |at: usize| i32::from(words.get(at).copied().unwrap_or(0) as u16 as i16);
            all.push(Found {
                stored,
                bytes: object.bytes.to_vec(),
                width: signed(7),
                height: -signed(8),
                baseline: -signed(14),
            });
        }
    }
    all
}

fn percent(ours: i32, theirs: i32) -> f64 {
    if theirs == 0 {
        return 0.0;
    }
    (f64::from(ours) - f64::from(theirs)) / f64::from(theirs) * 100.0
}

/// Prints the table of natural sizes against Ami Pro's and the mean and
/// worst error.
#[test]
#[ignore = "reads the EQV1 fixtures"]
fn natural_sizes_against_ami_pro() {
    let all = found();
    if all.is_empty() {
        eprintln!("no EQV1 fixtures: skipped");
        return;
    }
    let (mut sum_w, mut sum_h, mut worst_w, mut worst_h) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
    println!(
        "{:<34} {:>11} {:>11} {:>9} {:>9}",
        "tex", "W ours/ami", "H ours/ami", "B", "err% W,H"
    );
    for f in &all {
        let tree = f.stored.parse().expect("the oracle text parses");
        let ours = layout(&tree, &Params::default()).expect("lays out");
        let (ew, eh) = (percent(ours.width, f.width), percent(ours.height, f.height));
        println!(
            "{:<34} {:>5}/{:<5} {:>5}/{:<5} {:>4}/{:<4} {:>+6.1} {:>+6.1}",
            f.stored.tex.replace('\r', "").replace('\n', " "),
            ours.width,
            f.width,
            ours.height,
            f.height,
            ours.baseline,
            f.baseline,
            ew,
            eh
        );
        sum_w += ew.abs();
        sum_h += eh.abs();
        worst_w = worst_w.max(ew.abs());
        worst_h = worst_h.max(eh.abs());
    }
    let n = all.len() as f64;
    println!(
        "{} equations: mean |error| W {:.1}% H {:.1}%, worst W {:.1}% H {:.1}%",
        all.len(),
        sum_w / n,
        sum_h / n,
        worst_w,
        worst_h
    );
}

/// The stored text is normalised as Ami Pro wrote it (parse then write is
/// the identity) and the object's bytes read and written back equal.
#[test]
#[ignore = "reads the EQV1 fixtures"]
fn stored_form_round_trips() {
    let all = found();
    if all.is_empty() {
        eprintln!("no EQV1 fixtures: skipped");
        return;
    }
    for f in &all {
        assert_eq!(f.stored.bytes(), f.bytes, "bytes of {:?}", f.stored.tex);
        assert_eq!(
            f.stored.normalised().expect("parses"),
            f.stored.tex,
            "normal form of {:?}",
            f.stored.tex
        );
    }
    println!("{} equations round trip", all.len());
}
