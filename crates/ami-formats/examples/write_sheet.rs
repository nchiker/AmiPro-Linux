//! Writes an Ami Pro document as a style sheet, as Save as a Style Sheet
//! does, for checking in Ami Pro (`docs/research/VM_LAB.md`).
//!
//! Usage: `write_sheet SAM OUT.STY [--without-contents] [--description TEXT]
//! [--saved-at SECONDS]`. The SAM's own style sheet is looked for as the
//! importer does, and in `AMIPRO_STYLES`.

use std::path::PathBuf;

use amipro_formats::{SheetOptions, import_sam_path_with_style_directories, style_sheet};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let (Some(source), Some(target)) = (arguments.next(), arguments.next()) else {
        eprintln!(
            "usage: write_sheet SAM OUT.STY [--without-contents] [--description TEXT] [--saved-at SECONDS]"
        );
        std::process::exit(2);
    };
    let mut options = SheetOptions {
        contents: true,
        description: String::new(),
        saved_at: 1_791_958_088,
    };
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--without-contents" => options.contents = false,
            "--description" => options.description = arguments.next().unwrap_or_default(),
            "--saved-at" => {
                options.saved_at = arguments
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(options.saved_at);
            }
            other => {
                eprintln!("unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    let imported = match import_sam_path_with_style_directories(&source, &styles) {
        Ok(imported) => imported,
        Err(error) => {
            eprintln!("{source}: {error}");
            std::process::exit(1);
        }
    };
    let bytes = style_sheet(&imported.document, &options);
    if let Err(error) = std::fs::write(&target, &bytes) {
        eprintln!("{target}: {error}");
        std::process::exit(1);
    }
    println!("{target}: {} bytes", bytes.len());
}
