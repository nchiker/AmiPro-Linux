//! Lists the Help topics in `help/<area>/<topic>.md` for `src/help.rs`, which
//! embeds each file with `include_str!`. Nothing here reads the network.

use std::fmt::Write;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let help = root.join("help");
    println!("cargo:rerun-if-changed=help");
    let mut files = Vec::new();
    if let Ok(areas) = std::fs::read_dir(&help) {
        for area in areas.flatten().filter(|entry| entry.path().is_dir()) {
            println!("cargo:rerun-if-changed={}", area.path().display());
            if let Ok(topics) = std::fs::read_dir(area.path()) {
                for topic in topics.flatten() {
                    let path = topic.path();
                    if path.extension().is_some_and(|extension| extension == "md") {
                        files.push(path);
                    }
                }
            }
        }
    }
    files.sort();
    let mut out = String::from("pub const SOURCES: &[(&str, &str)] = &[\n");
    for path in &files {
        let relative = path
            .strip_prefix(&help)
            .expect("under help")
            .to_string_lossy()
            .replace('\\', "/");
        writeln!(
            out,
            "    ({relative:?}, include_str!({:?})),",
            path.to_string_lossy()
        )
        .expect("write to a string");
    }
    out.push_str("];\n");
    let target = PathBuf::from(std::env::var_os("OUT_DIR").expect("out dir")).join("help_index.rs");
    std::fs::write(target, out).expect("write the help index");
}
