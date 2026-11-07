//! What File › Save As would list before writing a document as an Ami Pro
//! document (decision B): the loss check's kinds, and the differences
//! behind them.
//!
//! Usage: `loss_check SAM`; the SAM's style sheet is found as the app
//! finds it, in the folders `AMIPRO_STYLES` names (none when unset).

use std::path::PathBuf;

use amipro_formats::roundtrip::{differences, is_writer_addition};
use amipro_formats::{
    SamOptions, import_sam, import_sam_path_with_style_directories, read_back, sam_document,
    sam_losses,
};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: loss_check SAM");
        std::process::exit(2);
    };
    let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    let imported = match import_sam_path_with_style_directories(&path, &styles) {
        Ok(imported) => imported,
        Err(error) => {
            eprintln!("{path}: {error}");
            std::process::exit(1);
        }
    };
    println!(
        "style sheet: {:?}",
        imported
            .report
            .associated_style_path
            .as_ref()
            .map(|path| path.display().to_string())
    );
    let document = imported.document;
    let options = SamOptions::at(1_791_958_088);
    for loss in sam_losses(&document, &options).unwrap() {
        println!("loss: {}", loss.message());
    }
    let copy = read_back(&document, &options).unwrap();
    if std::env::var_os("LOSS_CHECK_PLAIN").is_some() {
        let written = sam_document(&document, &options);
        let plain = import_sam(&written, None, document.title.clone())
            .unwrap()
            .document;
        println!("read back without a sheet:");
        let names = |document: &amipro_document::Document| {
            document
                .layouts()
                .iter()
                .map(|layout| layout.name.clone())
                .collect::<Vec<_>>()
        };
        println!(
            "  layouts {:?} / {:?}; sheet {:?} / {:?}",
            names(&document),
            names(&plain),
            document.style_sheet(),
            plain.style_sheet()
        );
        for difference in differences(&document, &plain)
            .into_iter()
            .filter(|difference| !is_writer_addition(difference))
        {
            println!(
                "  {:?} {} / {}",
                difference.change, difference.place, difference.aspect
            );
        }
    }
    let facts = |document: &amipro_document::Document| {
        amipro_formats::roundtrip::describe(document)
            .into_iter()
            .map(|fact| ((fact.place, fact.aspect), fact.value))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let (before, after) = (facts(&document), facts(&copy));
    let values = std::env::var_os("LOSS_CHECK_VALUES").is_some();
    for difference in differences(&document, &copy)
        .into_iter()
        .filter(|difference| !is_writer_addition(difference))
    {
        println!(
            "  {:?} {} / {}",
            difference.change, difference.place, difference.aspect
        );
        if values {
            let key = (difference.place.clone(), difference.aspect.clone());
            println!("    before: {:?}", before.get(&key));
            println!("    after:  {:?}", after.get(&key));
        }
    }
}
