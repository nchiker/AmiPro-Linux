//! Exports SAM files with the test shaper (frame places approximate; the
//! app's export uses its own fonts): `export OUT_DIR FILE.SAM…` or `export
//! OUT_DIR DIR` for every SAM below DIR (aggregate counts only). Pictures
//! go in as their stored bytes where a format takes them (metafiles,
//! bitmaps). `AMIPRO_STYLES` adds style-sheet folders.

use std::path::{Path, PathBuf};

use amipro_document::{Document, Picture, PictureFormat, PictureSource, Size};
use amipro_exchange::build::{BuildOptions, PictureSink, SunkPicture};
use amipro_exchange::{Format, Image, ImageKind, build, losses};
use amipro_layout::{FixedShaper, ShapeCache, layout_document};

struct Stored;

impl PictureSink for Stored {
    fn picture(&mut self, document: &Document, picture: &Picture) -> Option<SunkPicture> {
        let PictureSource::Embedded(object) = picture.source else {
            return None;
        };
        let bytes = document.objects().get(&object)?.bytes.clone();
        let (kind, natural, bytes) = match picture.format() {
            PictureFormat::Metafile => (ImageKind::Wmf, wmf_size(&bytes)?, bytes),
            PictureFormat::Bitmap => {
                use amipro_pictures::dib;
                let (header, raster) = if bytes.starts_with(b"BM") {
                    dib::bmp_header(&bytes).zip(dib::decode_bmp(&bytes))?
                } else {
                    dib::Header::read(&bytes)
                        .zip(dib::decode_packed(&bytes, dib::ColourTable::Rgb))?
                };
                let natural = header.natural_size();
                let bmp = dib::encode_bmp24(&raster, (3780, 3780));
                (
                    ImageKind::Bmp,
                    Size {
                        width: natural.width,
                        height: natural.height,
                    },
                    bmp.into(),
                )
            }
            _ => return None,
        };
        Some(SunkPicture {
            image: Image {
                bytes,
                kind,
                fallback: None,
                pixels: None,
            },
            natural: Some(natural),
        })
    }
}

/// A placeable metafile's size in twips.
fn wmf_size(bytes: &[u8]) -> Option<Size> {
    if bytes.get(0..4)? != [0xD7, 0xCD, 0xC6, 0x9A] {
        return None;
    }
    let word = |at: usize| i32::from(i16::from_le_bytes([bytes[at], bytes[at + 1]]));
    let inch = i32::from(u16::from_le_bytes([bytes[14], bytes[15]])).max(1);
    Some(Size {
        width: (word(10) - word(6)).abs() * 1440 / inch,
        height: (word(12) - word(8)).abs() * 1440 / inch,
    })
}

fn sams(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .collect();
        entries.sort();
        for entry in entries {
            sams(&entry, out);
        }
    } else if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("sam"))
    {
        out.push(path.to_owned());
    }
}

fn main() {
    let mut arguments = std::env::args().skip(1);
    let out_dir = PathBuf::from(arguments.next().expect("OUT_DIR"));
    std::fs::create_dir_all(&out_dir).expect("output folder");
    let inputs: Vec<PathBuf> = arguments.map(PathBuf::from).collect();
    let many = inputs.len() == 1 && inputs[0].is_dir();
    let mut files = Vec::new();
    for input in &inputs {
        sams(input, &mut files);
    }
    let styles: Vec<PathBuf> = std::env::var_os("AMIPRO_STYLES")
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    let (mut read, mut written, mut lossless) = (0, 0, [0usize; Format::ALL.len()]);
    let mut failed: Vec<usize> = Vec::new();
    let started = std::time::Instant::now();
    for (index, file) in files.iter().enumerate() {
        let Ok(import) = amipro_formats::import_sam_path_with_style_directories(file, &styles)
        else {
            continue;
        };
        read += 1;
        let document = import.document;
        let layout = layout_document(&document, &mut FixedShaper, &mut ShapeCache::default());
        let title = file
            .file_stem()
            .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
        let options = BuildOptions {
            layout: Some(&layout),
            title: title.clone(),
            ..BuildOptions::default()
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let built = build(&document, &options, &mut Stored);
            let outputs: Vec<Vec<u8>> = Format::ALL
                .into_iter()
                .map(|format| match format {
                    Format::Docx => amipro_exchange::docx::write(&built.document, None),
                    Format::Odt => amipro_exchange::odt::write(&built.document, None),
                    Format::Rtf => amipro_exchange::rtf::write(&built.document),
                    Format::Text => amipro_exchange::text::write(&built.document),
                    Format::Markdown => amipro_exchange::markdown::write(&built.document, "x")
                        .text
                        .into_bytes(),
                })
                .collect();
            (built, outputs)
        }));
        let Ok((built, outputs)) = outcome else {
            failed.push(index);
            continue;
        };
        for (slot, format) in Format::ALL.into_iter().enumerate() {
            let bytes = &outputs[slot];
            let losses = losses::losses(&built.features, format);
            if losses.is_empty() {
                lossless[slot] += 1;
            }
            let name = if many {
                format!("{index:04}.{}", format.extension())
            } else {
                format!("{title}.{}", format.extension())
            };
            std::fs::write(out_dir.join(name), bytes).expect("write");
            if !many && format == Format::Docx {
                for loss in &losses {
                    println!("{title}: {}", loss.message());
                }
            }
        }
        written += 1;
    }
    if !failed.is_empty() {
        // Indexes only (the corpora's names stay private).
        std::fs::write(
            out_dir.join("failed.txt"),
            failed
                .iter()
                .map(|index| format!("{index}\n"))
                .collect::<String>(),
        )
        .expect("write");
    }
    println!(
        "SAM files {}; read {read}; exported {written}; failed {}; nothing lost: DOCX {}, ODT {}, RTF {}, text {}, Markdown {}; {:.1} s",
        files.len(),
        failed.len(),
        lossless[0],
        lossless[1],
        lossless[2],
        lossless[3],
        lossless[4],
        started.elapsed().as_secs_f32()
    );
}
