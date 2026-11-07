//! Mark Glossary Record against Ami Pro 3.1's own file (VM fixture GLO0:
//! `IN/GLO0IN.SAM` before, `OUT/GLO0.SAM` after Ami Pro marked "sy").
//! `AMIPRO_VM_LAB=…/vm-lab`.

use amipro_formats::glossary::{append_record, read_glossary};

#[test]
#[ignore = "needs the VM folder (AMIPRO_VM_LAB)"]
fn mark_glossary_record_writes_ami_pros_bytes_glo0() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let lab = std::path::PathBuf::from(lab);
    let before = std::fs::read(lab.join("IN/GLO0IN.SAM")).unwrap();
    let ami_pro = std::fs::read(lab.join("OUT/GLO0.SAM")).unwrap();
    let ours = append_record(&before, "sy", b"Sincerely yours, Ami.").unwrap();
    assert_eq!(
        String::from_utf8_lossy(&ours),
        String::from_utf8_lossy(&ami_pro)
    );
    assert_eq!(read_glossary(&ami_pro).unwrap().names(), ["hi", "sy"]);
}
