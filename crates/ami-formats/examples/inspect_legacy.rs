use std::{env, fs, path::Path, process::ExitCode};

use amipro_formats::{LegacyKind, parse};

fn main() -> ExitCode {
    let mut arguments = env::args_os().skip(1);
    let Some(path) = arguments.next() else {
        eprintln!("usage: inspect_legacy FILE.SAM|FILE.STY");
        return ExitCode::from(2);
    };
    if arguments.next().is_some() {
        eprintln!("inspect_legacy accepts exactly one file");
        return ExitCode::from(2);
    }
    let path = Path::new(&path);
    let kind = match path.extension().and_then(|value| value.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("sam") => LegacyKind::Document,
        Some(extension) if extension.eq_ignore_ascii_case("sty") => LegacyKind::StyleSheet,
        _ => {
            eprintln!("expected a .SAM or .STY file");
            return ExitCode::from(2);
        }
    };
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    match parse(&bytes, kind) {
        Ok((legacy, report)) => {
            println!("kind: {:?}", legacy.kind);
            println!("format version: {:?}", legacy.format_version);
            println!("sections: {}", legacy.sections.len());
            println!("style names: {}", legacy.style_names.len());
            println!("recognized sections: {}", report.recognized_sections);
            println!("unknown sections: {}", report.unknown_sections.len());
            for warning in report.warnings {
                println!("warning: {warning}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
}
