//! Compares two Ami Pro documents as this project reads them: the facts of
//! `roundtrip::describe` that differ, leaving out what the writer adds and
//! the values measured from the caches. For checking a file Ami Pro saved
//! again against the one it opened (`docs/research/VM_LAB.md`).
//!
//! Usage: `compare_sam A.SAM B.SAM`; style sheets as the importer finds
//! them, and in `AMIPRO_STYLES`.

use std::path::PathBuf;

use amipro_formats::import_sam_path_with_style_directories;
use amipro_formats::roundtrip::{differences, is_writer_addition};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let (Some(first), Some(second)) = (arguments.next(), arguments.next()) else {
        eprintln!("usage: compare_sam A.SAM B.SAM");
        std::process::exit(2);
    };
    let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    let read = |path: &str| match import_sam_path_with_style_directories(path, &styles) {
        Ok(imported) => imported.document,
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(1);
        }
    };
    let (a, b) = (read(&first), read(&second));
    let found: Vec<_> = differences(&a, &b)
        .into_iter()
        .filter(|difference| {
            !is_writer_addition(difference)
                && !matches!(
                    difference.aspect.as_str(),
                    "line heights" | "face kinds" | "cache"
                )
        })
        .collect();
    for difference in &found {
        println!(
            "{:?} {} / {}",
            difference.change, difference.place, difference.aspect
        );
    }
    println!("{} differences", found.len());
}
