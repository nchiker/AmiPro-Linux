//! Password-protected files against Ami Pro 3.1's (`docs/research/
//! COMMANDS_NOTES.md` §9.6; VM fixtures PWA–PWEMP, `AMIPRO_VM_LAB`).

use amipro_formats::import_sam;
use amipro_formats::protect::{ProtectError, protect, unprotect};

const FIXTURES: [(&str, &str); 7] = [
    ("PWA", "A"),
    ("PWAB", "AB"),
    ("PWBA", "BA"),
    ("PWLT", "<A"),
    ("PW14", "ABCDEFGHIJKLMN"),
    ("PWHI", "\u{e9}"),
    ("PWEMP", ""),
];

#[test]
#[ignore = "needs the VM folder (AMIPRO_VM_LAB)"]
fn ami_pros_protected_files_open_and_are_written_byte_for_byte() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let out = std::path::PathBuf::from(lab).join("OUT");
    let plain = std::fs::read(out.join("PWPLN.SAM")).unwrap();
    let plain_text = import_sam(&plain, None, "Plain").unwrap().document;
    for (name, password) in FIXTURES {
        let protected = std::fs::read(out.join(format!("{name}.SAM"))).unwrap();
        let opened =
            unprotect(&protected, password).unwrap_or_else(|error| panic!("{name}: {error}"));
        let document = import_sam(&opened, None, name).unwrap().document;
        // PWAB on were saved from a copy with " x" typed after the text.
        assert!(
            document.paragraphs()[0]
                .text
                .starts_with(&plain_text.paragraphs()[0].text),
            "{name}: the text"
        );
        assert_eq!(
            protect(&opened, password),
            protected,
            "{name}: Ami Pro's bytes"
        );
        if !password.is_empty() {
            assert_eq!(
                unprotect(&protected, &password.to_lowercase().replace('\u{e9}', "e")),
                Err(ProtectError::WrongPassword),
                "{name}: another password"
            );
        }
    }
}

/// Writes the twins Ami Pro opens: PWPLN's document protected with
/// "Tw1n", and PI3's (a bitmap in a frame) protected with "pic", as
/// `IN/PWT1.SAM` and `IN/PWT2.SAM` (`AMIPRO_VM_LAB`).
#[test]
#[ignore = "writes into the VM folder (AMIPRO_VM_LAB)"]
fn writes_the_protected_twins_ami_pro_opens() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let options = amipro_formats::SamOptions::at(1_793_900_000);
    for (source, name, password) in [("PWPLN", "PWT1", "Tw1n"), ("PI3", "PWT2", "pic")] {
        let document =
            amipro_formats::import_sam_path(lab.join("OUT").join(format!("{source}.SAM")))
                .unwrap()
                .document;
        let plain = amipro_formats::sam_document(&document, &options);
        let protected = protect(&plain, password);
        assert_eq!(unprotect(&protected, password).unwrap(), plain);
        let path = lab.join("IN").join(format!("{name}.SAM"));
        assert!(!path.exists(), "{} exists", path.display());
        std::fs::write(&path, protected).unwrap();
        eprintln!("wrote {}", path.display());
    }
}
