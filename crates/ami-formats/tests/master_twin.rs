//! Master documents against Ami Pro 3.1 (`docs/research/MASTER_NOTES.md`):
//! the members of VM fixtures MD2 and MD3, laid out, show Ami Pro's page
//! numbers and pass on the `[sequence]` Ami Pro wrote into the next member.
//! `AMIPRO_VM_LAB=…/vm-lab`.

use amipro_document::Document;
use amipro_formats::import_sam_path;
use amipro_layout::master::next_sequence;
use amipro_layout::{FixedShaper, ShapeCache, layout_document};

fn member(path: &std::path::Path) -> Document {
    import_sam_path(path).unwrap().document
}

#[test]
#[ignore = "needs the VM folder (AMIPRO_VM_LAB)"]
fn members_continue_and_pass_on_ami_pros_numbers_md2_md3() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        eprintln!("AMIPRO_VM_LAB is not set; skipping");
        return;
    };
    let out = std::path::PathBuf::from(lab).join("OUT");
    for set in ["MD2GEN", "MD3"] {
        let names = ["MA", "MB", "MC"];
        let members: Vec<Document> = names
            .iter()
            .map(|name| member(&out.join(set).join(format!("{name}.SAM"))))
            .collect();
        assert!(
            members[0].contents().sequence.is_none(),
            "{set}: the first has none"
        );
        for (index, document) in members.iter().enumerate() {
            let layout = layout_document(document, &mut FixedShaper, &mut ShapeCache::default());
            let shown: Vec<u32> = layout.pages.iter().map(|page| page.shown).collect();
            let first = 2 * index as u32 + 1;
            assert_eq!(
                shown,
                [first, first + 1],
                "{set} {}: pages shown",
                names[index]
            );
            if let Some(next) = members.get(index + 1) {
                let ours = next_sequence(document, &layout, &[]);
                assert_eq!(
                    Some(&ours),
                    next.contents().sequence.as_ref(),
                    "{set}: the sequence {} passes on",
                    names[index]
                );
            }
        }
    }
}
