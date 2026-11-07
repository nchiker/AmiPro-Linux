//! TD.12: the app's twins (`vm-lab/IN/ATD*.SAM`) against what Ami Pro 3.1
//! saved of them (`vm-lab/OUT/td12/TDT*.SAM`): each table's cell texts,
//! formulas, and number format. `AMIPRO_VM_LAB=DIR`; ignored; prints the
//! differences.

use amipro_document::{CellContent, Document, FrameContent};
use amipro_formats::import_sam_path;
use std::path::Path;

fn cells(document: &Document) -> Vec<(String, Option<String>, String)> {
    let mut out = Vec::new();
    for id in document.frame_stack() {
        if let Some(FrameContent::Table(table)) = document.frame(*id).map(|f| &f.content) {
            for cell in &table.cells {
                let CellContent::Text(flow) = cell.content;
                let paragraph = &document.flow(flow).unwrap().paragraphs[0];
                let format = document
                    .styles()
                    .get(&paragraph.style)
                    .map(|style| format!("{:?}", style.kept.number_format))
                    .unwrap_or_default();
                out.push((paragraph.text.clone(), cell.formula.clone(), format));
            }
        }
    }
    out
}

#[test]
#[ignore = "needs the VM lab folder"]
fn ami_pros_saves_equal_the_twins() {
    let Some(lab) = std::env::var_os("AMIPRO_VM_LAB") else {
        return;
    };
    let lab = Path::new(&lab);
    for (twin, saved) in [
        ("ATD1", "TDT1S"),
        ("ATD2C", "TDT2C"),
        ("ATD2F", "TDT2F"),
        ("ATD2G", "TDT2G"),
        ("ATD2N", "TDT2N"),
        ("ATD2P", "TDT2P"),
        ("ATD3", "TDT3S"),
        ("ATD4", "TDT4S"),
        ("ATD5", "TDT5S"),
    ] {
        let a = import_sam_path(lab.join("IN").join(format!("{twin}.SAM"))).unwrap();
        let b = import_sam_path(lab.join("OUT/td12").join(format!("{saved}.SAM"))).unwrap();
        let (a, b) = (cells(&a.document), cells(&b.document));
        let mut differences = Vec::new();
        if a.len() != b.len() {
            differences.push(format!("cell count {} vs {}", a.len(), b.len()));
        }
        for (index, (x, y)) in a.iter().zip(&b).enumerate() {
            if x != y {
                differences.push(format!("cell {index}: {x:?} -> {y:?}"));
            }
        }
        eprintln!(
            "{twin} -> {saved}: {} cells, {} differ",
            a.len(),
            differences.len()
        );
        for line in differences {
            eprintln!("    {line}");
        }
    }
}
