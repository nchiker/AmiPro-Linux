//! Writing Ami Pro documents (`.SAM`, Phase 4): the sections Ami Pro 3.1's
//! document writer emits, in its order (decompile `11c8:1c7b`;
//! `docs/research/SAM_FORMAT_NOTES.md`, "Writing documents"). Values the
//! model does not hold come from what the importer kept (`build.rs`); a
//! new document gets what Ami Pro writes for one.
//!
//! Written so far (slices 4.3–4.5): the header sections, the styles stored
//! in the document, its page layouts (the standard one unless it is the
//! style sheet's and unchanged), `[l1]`, and the main text. The page table
//! is never written: Ami Pro rebuilds it (VM fixtures K1–K6).

use std::collections::BTreeMap;

use amipro_document::{
    Document, FrameContent, LayoutId, PageLayout, ParagraphStyle, PictureSource, StyleOrigin,
};

use crate::body::Body;
use crate::frames::{Context, anchor_numbers, write_page_frames};
use crate::is_standard_layout;
use crate::sheet::{Sequence, section_values, write_layout};
use crate::writer::{Lines, windows_1252, write_style_record};

/// What saving a SAM needs besides the document.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SamOptions {
    /// The save time in Ami Pro's time base (`[desc]` value 6;
    /// `WallClock::to_ami_pro`).
    pub saved_at: u32,
    /// Minutes of editing since the document was opened or last saved,
    /// added to its total (`[desc]` value 9, as Ami Pro adds them).
    pub editing_minutes: u32,
    /// The statistics counted for this save, when counted (`[desc]` values
    /// 10 to 14); otherwise the document's as they stand.
    pub statistics: Option<amipro_document::Statistics>,
}

impl SamOptions {
    /// A save at `saved_at` with nothing else counted.
    pub const fn at(saved_at: u32) -> Self {
        Self {
            saved_at,
            editing_minutes: 0,
            statistics: None,
        }
    }
}

/// The document as an Ami Pro 3.1 document (`.SAM`), lines ending in CR LF.
/// With statistics counted for the save, the size is the written file's in
/// K (bytes / 1000, `12a0:101b`), so it is written again until it is.
pub fn sam_document(document: &Document, options: &SamOptions) -> Vec<u8> {
    let Some(mut statistics) = options.statistics else {
        return written_document(document, options);
    };
    let mut bytes = Vec::new();
    for _ in 0..4 {
        let counted = SamOptions {
            statistics: Some(statistics),
            ..options.clone()
        };
        bytes = written_document(document, &counted);
        let size = u32::try_from(bytes.len() / 1000).unwrap_or(u32::MAX);
        if size == statistics.size {
            break;
        }
        // The style sheet and linked files Ami Pro adds to value 14 are
        // not counted here (FX2): it shows that value nowhere.
        statistics.size = size;
        statistics.size_with_links = size;
    }
    bytes
}

fn written_document(document: &Document, options: &SamOptions) -> Vec<u8> {
    let shortened = crate::writer::shortened_style_names(document);
    let document = shortened.as_ref().unwrap_or(document);
    // The pictures' bytes are most of a large document.
    let objects: usize = document
        .objects()
        .values()
        .map(|object| object.bytes.len())
        .sum();
    let text: usize = document
        .flows()
        .flat_map(|(_, flow)| flow.paragraphs.iter())
        .map(|paragraph| paragraph.text.len() + 32)
        .sum();
    let mut out = Lines(Vec::with_capacity(objects + 2 * text + 64 * 1024));
    let kept = Kept::new(document);
    header_sections(&mut out, document, options, &kept);
    for style in stored_styles(document) {
        write_style_record(&mut out, style, document.styles());
    }
    let layouts = written_layouts(document);
    let names: BTreeMap<LayoutId, String> = layouts
        .iter()
        .map(|(layout, name)| (layout.id, name.clone()))
        .collect();
    let anchors = anchor_numbers(document);
    let context = Context {
        document,
        contents: true,
        sheet: false,
        sheet_frames_text: false,
        layout_names: &names,
        anchors: &anchors,
    };
    // Page frames, then layouts (`11c8:3ba0`), one sequence of frame
    // numbers over both (`11c8:07dd`).
    let mut sequence = Sequence::new(document);
    write_page_frames(&mut out, &context, &mut sequence);
    // Each layout ends with its own `[elay]` (`11c8:236e`): with one at
    // the end only, Ami Pro loses every layout after the first (VM, G21).
    for (layout, name) in &layouts {
        write_layout(&mut out, &context, layout, name, &mut sequence);
        out.line(0, b"[elay]");
    }
    // `[l1]` (`11c8:3ab2`): the layout the document starts in, `0` for the
    // standard one.
    let start = document.start_layout();
    let first = if is_standard_layout(&start.name) {
        "0".to_owned()
    } else {
        names
            .get(&start.id)
            .cloned()
            .unwrap_or_else(|| start.name.clone())
    };
    section(&mut out, "l1", &[first]);
    kept.unknown_after(&mut out, "l1");
    out.line(0, b"[edoc]");
    Body::new(document, &anchors, &names).flow(&mut out.0, document.main_flow_id());
    write_end(
        &mut out,
        document,
        &picture_frames(document, &layouts, false),
        &mut sequence,
    );
    out.0
}

/// The end of a file with contents (`11c8:3d76`): `>`, each picture's
/// bytes and then its rendering's (`11c8:4155`), an empty line, and the
/// `[Embedded]` directory, one line per picture (`11c8:4521`), which ends
/// with the offset of its own first line; Ami Pro writes the directory
/// even when it is empty.
pub(crate) fn write_end(
    out: &mut Lines,
    document: &Document,
    frames: &[&amipro_document::Frame],
    sequence: &mut Sequence,
) {
    out.line(0, b">");
    let mut entries = Vec::new();
    for frame in frames {
        let FrameContent::Picture(picture) = &frame.content else {
            continue;
        };
        let mut store = |object: Option<amipro_document::ObjectId>| match object
            .and_then(|object| document.objects().get(&object))
        {
            Some(object) => {
                let offset = out.0.len();
                out.0.extend_from_slice(&object.bytes);
                (offset, object.bytes.len())
            }
            None => (0, 0),
        };
        let (original, path) = match &picture.source {
            PictureSource::Embedded(object) => (store(Some(*object)), None),
            PictureSource::Linked(path) => ((0, 0), Some(path.clone())),
        };
        // Ami Pro's screen image shows the picture as it was last drawn,
        // so it goes with a record written anew (Ami Pro draws it again).
        let rendering = store(picture.rendering.filter(|_| picture.kept_is_current()));
        let mut entry = format!(
            "{} {} {} {} {} {} ",
            sequence.of(frame).trim(),
            crate::frames::stored_extension(picture),
            original.0,
            original.1,
            rendering.0,
            rendering.1
        )
        .into_bytes();
        if let Some(path) = path {
            entry.extend(path.chars().map(windows_1252));
        }
        entries.push(entry);
    }
    out.0.extend(b"\r\n");
    let directory = out.0.len();
    out.line(0, b"[Embedded]");
    for entry in entries {
        out.line(0, &entry);
    }
    out.line(0, format!("{directory:08}").as_bytes());
}

/// The frames whose pictures the file stores, in Ami Pro's order: the page
/// frames as written (`page_frames_in_file_order`), then each written
/// layout's frames. A document
/// leaves out a style sheet's layout frames (0x8), whose pictures are the
/// sheet's; a style sheet with contents stores them all (`11c8:3d76`).
pub(crate) fn picture_frames<'a>(
    document: &'a Document,
    layouts: &[(&PageLayout, String)],
    sheet: bool,
) -> Vec<&'a amipro_document::Frame> {
    let layout_frames = layouts.iter().flat_map(|(layout, _)| {
        crate::sheet::layout_frames(layout)
            .into_iter()
            .map(|(_, id)| id)
    });
    crate::frames::page_frames_in_file_order(document)
        .into_iter()
        .chain(layout_frames)
        .filter_map(|id| document.frame(id))
        .filter(|frame| {
            matches!(frame.content, FrameContent::Picture(_))
                && (sheet || crate::frames::kept_flags(frame) & crate::frames::LAYOUT_FRAME == 0)
        })
        .collect()
}

pub(crate) use amipro_document::SHEET_LAYOUT_KEY;

/// The page layouts a SAM stores, each with the name Ami Pro writes it
/// under (`11c8:3ba0`): the standard layout unless it is the style sheet's
/// and unchanged, then every inserted layout, which Ami Pro numbers `1`,
/// `2`, … in order. Ami Pro cannot lay out a document that has neither a
/// style sheet nor a layout (it stops responding).
fn written_layouts(document: &Document) -> Vec<(&PageLayout, String)> {
    let standard_from_sheet = document.style_sheet().is_some_and(|link| {
        link.layout_from_sheet
            && document
                .legacy()
                .get(SHEET_LAYOUT_KEY)
                .is_none_or(|was| *was == layout_fingerprint(document))
    });
    let mut number = 0;
    document
        .layouts()
        .iter()
        .enumerate()
        .filter(|(index, _)| !(*index == 0 && standard_from_sheet))
        .map(|(_, layout)| {
            let name = if is_standard_layout(&layout.name) {
                layout.name.clone()
            } else {
                number += 1;
                number.to_string()
            };
            (&**layout, name)
        })
        .collect()
}

/// What the document's standard layout says, as a hash of the record the
/// writer makes for it with its frames' text: equal while it is unchanged.
pub(crate) fn layout_fingerprint(document: &Document) -> String {
    let mut out = Lines::default();
    let layout = document.document_layout();
    write_layout(
        &mut out,
        &Context::layout_only(document),
        layout,
        &layout.name,
        &mut Sequence::new(document),
    );
    for id in layout.frames() {
        if let Some(FrameContent::Text(flow)) = document.frame(id).map(|frame| &frame.content) {
            Body::new(document, &BTreeMap::new(), &BTreeMap::new()).flow(&mut out.0, *flow);
        }
    }
    format!("{:016x}", crate::roundtrip::fnv1a(&out.0))
}

/// The document's kept top-level sections, as read, and the unknown ones
/// by the known section they followed.
struct Kept<'a> {
    document: &'a Document,
    /// Unknown sections (raw values) by the known section before them;
    /// `""` for those before any.
    unknown: BTreeMap<String, Vec<(String, String)>>,
}

/// The sections Ami Pro 3.1's writer knows, in its order.
const KNOWN: [&str; 24] = [
    "ver",
    "sty",
    "files",
    "charset",
    "revisions",
    "recfile",
    "toc",
    "master",
    "sequence",
    "docvars",
    "book",
    "prn",
    "port",
    "lang",
    "fldnames",
    "desc",
    "fopts",
    "auto",
    "lnopts",
    "docopts",
    "gramstyle",
    "paranum",
    "newmac",
    "l1",
];

impl<'a> Kept<'a> {
    fn new(document: &'a Document) -> Self {
        let mut unknown: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut previous = String::new();
        for value in &document.legacy().values {
            let Some(name) = value
                .key
                .strip_prefix("section [")
                .and_then(|rest| rest.strip_suffix(']'))
            else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            if KNOWN.contains(&lower.as_str()) {
                previous = lower;
            } else {
                unknown
                    .entry(previous.clone())
                    .or_default()
                    .push((name.to_owned(), value.value.clone()));
            }
        }
        Self { document, unknown }
    }

    /// A known section's values as read, or `None` when the file had none.
    fn values(&self, name: &str) -> Option<Vec<String>> {
        let key = format!("section [{name}]");
        self.document
            .legacy()
            .values
            .iter()
            .find(|value| value.key.eq_ignore_ascii_case(&key))
            .map(|value| section_values(&value.value))
    }

    /// Writes the unknown sections that followed `known` in the original.
    fn unknown_after(&self, out: &mut Lines, known: &str) {
        for (name, raw) in self.unknown.get(known).into_iter().flatten() {
            section(out, name, &section_values(raw));
        }
    }
}

/// A section and its values, each on its own line after one tab. Kept
/// values are as the file had them, escapes included: written back in
/// Windows-1252.
fn section(out: &mut Lines, name: &str, values: &[String]) {
    out.line(0, format!("[{name}]").as_bytes());
    for value in values {
        let bytes: Vec<u8> = value.chars().map(windows_1252).collect();
        out.line(1, &bytes);
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

/// The sections before the styles, in Ami Pro 3.1's order (`11c8:1c7b`).
/// Each comes from the document's kept section when it has one, else Ami
/// Pro's value for a new document; the ones only some documents have are
/// written when kept.
fn header_sections(out: &mut Lines, document: &Document, options: &SamOptions, kept: &Kept<'_>) {
    kept.unknown_after(out, "");
    section(out, "ver", &strings(&["4"]));
    kept.unknown_after(out, "ver");
    // The style sheet's name; empty for a document that keeps its format.
    let sheet = document
        .style_sheet()
        .map(|link| crate::decode_windows_1252(&crate::escape_text(&link.name)))
        .unwrap_or_default();
    section(out, "sty", &[sheet]);
    kept.unknown_after(out, "sty");
    section(out, "files", &linked_files(document));
    kept.unknown_after(out, "files");
    // Ami Pro writes its resource strings whatever the file said.
    section(
        out,
        "charset",
        &strings(&["82", "ANSI (Windows, IBM CP 1252)"]),
    );
    kept.unknown_after(out, "charset");
    section(
        out,
        "revisions",
        &kept.values("revisions").unwrap_or_else(|| strings(&["0"])),
    );
    kept.unknown_after(out, "revisions");
    for name in ["recfile", "toc", "master", "sequence", "docvars", "book"] {
        if let Some(values) = crate::info::contents_values(document, name, kept.values(name)) {
            section(out, name, &values);
        }
        kept.unknown_after(out, name);
    }
    // The printer the document was set up for; a new document has the one
    // Ami Pro's default sheet names.
    section(
        out,
        "prn",
        &kept
            .values("prn")
            .unwrap_or_else(|| strings(&["QMS-PS 810"])),
    );
    kept.unknown_after(out, "prn");
    section(
        out,
        "port",
        &kept.values("port").unwrap_or_else(|| strings(&["LPT1:"])),
    );
    kept.unknown_after(out, "port");
    section(
        out,
        "lang",
        &crate::info::language_values(document, kept.values("lang")),
    );
    kept.unknown_after(out, "lang");
    section(
        out,
        "fldnames",
        &crate::info::field_name_values(document.info()),
    );
    kept.unknown_after(out, "fldnames");
    section(out, "desc", &description(document, options));
    kept.unknown_after(out, "desc");
    section(
        out,
        "fopts",
        &crate::info::footnote_option_values(document.footnote_options()),
    );
    kept.unknown_after(out, "fopts");
    if let Some(values) = kept.values("auto") {
        section(out, "auto", &values);
    }
    kept.unknown_after(out, "auto");
    section(out, "lnopts", &crate::info::line_numbering_values(document));
    kept.unknown_after(out, "lnopts");
    let mut document_options = kept
        .values("docopts")
        .unwrap_or_else(|| strings(&["5", "2"]));
    document_options.resize(2, "2".into());
    document_options[0] = document.hyphenation_zone().to_string();
    section(out, "docopts", &document_options);
    kept.unknown_after(out, "docopts");
    section(out, "GramStyle", &[document.grammar_set().to_string()]);
    kept.unknown_after(out, "gramstyle");
    if let Some(values) = kept.values("ParaNum") {
        section(out, "ParaNum", &values);
    }
    kept.unknown_after(out, "paranum");
    kept.unknown_after(out, "newmac");
}

/// `[files]`: the path of each linked picture, page frames first (as
/// written), then each layout's frames (`11c8:07dd`).
pub(crate) fn linked_files(document: &Document) -> Vec<String> {
    let layout_frames = document
        .layouts()
        .iter()
        .flat_map(|layout| layout.frames().collect::<Vec<_>>());
    crate::frames::page_frames_in_file_order(document)
        .into_iter()
        .chain(layout_frames)
        .filter_map(|id| match &document.frame(id)?.content {
            FrameContent::Picture(picture) => match &picture.source {
                PictureSource::Linked(path) => Some(path.clone()),
                PictureSource::Embedded(_) => None,
            },
            _ => None,
        })
        .collect()
}

/// `[desc]` (`11c8:342b`): the Doc Info as it stands, with this save's
/// time, the minutes of editing added, and the statistics counted for the
/// save if they were. The revisions are as the document holds them: one
/// more than its file said, as Ami Pro counts each open (`1058:2732`). A
/// document without Doc Info is created and saved now.
fn description(document: &Document, options: &SamOptions) -> Vec<String> {
    let info = document.info();
    let mut values = crate::info::description_values(info, options.saved_at);
    values[5] = options.saved_at.to_string();
    let minutes = info.editing_minutes.saturating_add(options.editing_minutes);
    values[8] = minutes.to_string();
    if let Some(statistics) = options.statistics {
        for (index, number) in [
            statistics.pages,
            statistics.words,
            statistics.characters,
            statistics.size,
            statistics.size_with_links,
        ]
        .into_iter()
        .enumerate()
        {
            values[9 + index] = number.to_string();
        }
    }
    values
}

/// The styles a SAM stores (`11c8:1c7b`): those stored in the document, or
/// every style a document without a style sheet has, in function-key
/// order with the styles without a key last, in the order they were read.
fn stored_styles(document: &Document) -> Vec<&ParagraphStyle> {
    let detached = document.style_sheet().is_none();
    let order: Vec<String> = document
        .legacy()
        .get("style order")
        .map(|names| names.split('\n').map(str::to_lowercase).collect())
        .unwrap_or_default();
    let mut styles: Vec<&ParagraphStyle> = document
        .styles()
        .values()
        .filter(|style| match style.origin {
            StyleOrigin::Document => true,
            StyleOrigin::Sheet => detached,
            StyleOrigin::Missing => false,
        })
        .collect();
    styles.sort_by_key(|style| {
        let key = style.kept.function_key;
        let read = order
            .iter()
            .position(|name| *name == style.name.to_lowercase())
            .unwrap_or(usize::MAX);
        (key == 0, key, read, style.id)
    });
    styles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_sam;
    use crate::roundtrip::{Difference, differences, is_writer_addition, read_back};

    const OPTIONS: SamOptions = SamOptions::at(1_791_958_088);

    /// The differences other than what Ami Pro 3.1 adds to every file and
    /// the face kinds its style records give.
    fn beyond_additions(document: &Document, copy: &Document) -> Vec<Difference> {
        differences(document, copy)
            .into_iter()
            .filter(|difference| {
                !is_writer_addition(difference) && difference.aspect != "face kinds"
            })
            .collect()
    }

    fn top_sections(sam: &[u8]) -> Vec<String> {
        String::from_utf8_lossy(sam)
            .split("\r\n")
            .filter(|line| line.starts_with('[') && *line != "[tag]")
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn a_new_document_has_ami_pros_sections_and_reads_back_alike() {
        let mut document = Document::from_plain_text("One <two>\nThree [four] @five");
        document.set_style_sheet(None);
        let sam = sam_document(&document, &OPTIONS);
        assert_eq!(
            top_sections(&sam),
            [
                "[ver]",
                "[sty]",
                "[files]",
                "[charset]",
                "[revisions]",
                "[prn]",
                "[port]",
                "[lang]",
                "[fldnames]",
                "[desc]",
                "[fopts]",
                "[lnopts]",
                "[docopts]",
                "[GramStyle]",
                "[lay]",
                "[elay]",
                "[l1]",
                "[edoc]",
                "[Embedded]"
            ]
        );
        let text = String::from_utf8(sam).unwrap();
        assert!(text.contains("[sty]\r\n\t\r\n"), "no style sheet");
        assert!(text.contains(
            "[desc]\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t1791958088\r\n\t1\r\n\t1791958088\r\n\t0\r\n"
        ));
        let end = text.find("[Embedded]").unwrap();
        assert!(text.ends_with(&format!(
            "[edoc]\r\nOne <<two<;>\r\n\r\nThree <[>four] @five\r\n\r\n>\r\n\r\n[Embedded]\r\n{end:08}\r\n"
        )));
        let copy = read_back(&document, &OPTIONS).unwrap();
        assert_eq!(beyond_additions(&document, &copy), []);
    }

    #[test]
    fn counted_statistics_are_written_with_the_files_own_size() {
        let mut document = Document::from_plain_text(&"Some words here.\n".repeat(400));
        *document.info_mut() = amipro_document::DocumentInfo::new(5);
        let options = SamOptions {
            statistics: Some(amipro_document::Statistics {
                pages: 3,
                words: 1200,
                characters: 6400,
                size: 0,
                size_with_links: 0,
            }),
            ..SamOptions::at(9)
        };
        let written = sam_document(&document, &options);
        let size = written.len() / 1000;
        assert!(size > 5);
        let text = String::from_utf8_lossy(&written);
        assert!(text.contains(&format!(
            "\t3\r\n\t1200\r\n\t6400\r\n\t{size}\r\n\t{size}\r\n"
        )));
        // Without counted statistics, the document's stand.
        let plain =
            String::from_utf8_lossy(&sam_document(&document, &SamOptions::at(9))).into_owned();
        assert!(plain.contains("\t0\r\n\t0\r\n\t0\r\n\t0\r\n\t0\r\n"));
    }

    #[test]
    fn kept_sections_stay_in_place_and_desc_counts_this_save() {
        let sam = "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[files]\r\n[charset]\r\n\t82\r\n\tANSI (Windows, IBM CP 1252)\r\n\
            [panosefonttable]\r\n\tkept 1\r\n\tkept 2\r\n[prn]\r\n\tPrinter\r\n[lang]\r\n\t1\r\n\
            [desc]\r\n\tDescribed\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t700\r\n\t3\r\n\t600\r\n\
            [fopts]\r\n\t0\r\n\t1\r\n\t0\r\n\t0\r\n[lnopts]\r\n\t2\r\n\tBody Text\r\n\t1\r\n[docopts]\r\n\t5\r\n\t2\r\n\
            [tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
            [edoc]\r\nText\r\n\r\n>\r\n";
        let document = import_sam(sam.as_bytes(), None, "Kept").unwrap().document;
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        // The unknown section follows the section it followed; the sections
        // 3.1 always writes are added with their defaults.
        assert!(written.contains(
            "[charset]\r\n\t82\r\n\tANSI (Windows, IBM CP 1252)\r\n[panosefonttable]\r\n\tkept 1\r\n\tkept 2\r\n[revisions]\r\n\t0\r\n"
        ));
        assert!(written.contains("[prn]\r\n\tPrinter\r\n[port]\r\n\tLPT1:\r\n"));
        // This save's time and one more save; what an older file lacks is
        // what Ami Pro's reader takes it as.
        assert!(written.contains(
            "[desc]\r\n\tDescribed\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t1791958088\r\n\t4\r\n\t600\r\n\t0\r\n\t0\r\n\t0\r\n\t0\r\n\t0\r\n\t0\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t\r\n\t0\r\n[fopts]\r\n\t0\r\n\t1\r\n\t0\r\n\t0\r\n\t\r\n\t\r\n"
        ));
        let copy = read_back(&document, &OPTIONS).unwrap();
        assert_eq!(beyond_additions(&document, &copy), []);
    }

    #[test]
    fn inserted_layouts_are_renamed_as_ami_pro_writes_them() {
        // A layout inserted at the start (`[l1]`) and named "7": written
        // as layout 1, which the page break and `[l1]` then name; each
        // layout ends with its own `[elay]`.
        let layout = |name: &str, left: i64| {
            format!(
                "[lay]\r\n\t{name}\r\n\t513\r\n\t[rght]\r\n\t\t15840\r\n\t\t12240\r\n\t\t1\r\n\t\t{left}\r\n\t\t1440\r\n\t\t1\r\n\t\t1440\r\n\t\t1440\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t1\r\n\t\t{left}\r\n\t\t10800\r\n\t\t0\r\n[elay]\r\n"
            )
        };
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t\r\n[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n\
             {}{}[l1]\r\n\t7\r\n[edoc]\r\nOne<:p</@>Standard>\r\n\r\nTwo<:p</@>7>\r\n\r\n>\r\n",
            layout("Standard", 1440),
            layout("7", 2880)
        );
        let document = import_sam(sam.as_bytes(), None, "Layouts")
            .unwrap()
            .document;
        assert_eq!(document.start_layout().name, "7");
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert_eq!(written.matches("[elay]").count(), 2);
        assert!(written.contains("[lay]\r\n\t1\r\n\t513\r\n"));
        assert!(written.contains("[l1]\r\n\t1\r\n"));
        assert!(written.contains("One<:p</@>Standard>\r\n\r\nTwo<:p</@>1>\r\n"));
        let copy = read_back(&document, &OPTIONS).unwrap();
        let mut renamed = document.clone();
        let inserted = renamed.layouts()[1].id;
        renamed.layout_mut(inserted).unwrap().name = "1".into();
        assert_eq!(beyond_additions(&renamed, &copy), []);
    }

    #[test]
    fn a_sheets_standard_layout_is_kept_out_until_it_changes() {
        // A linked document that stores only an inserted layout (7 corpus
        // files) and starts in it: the sheet's Standard comes first, and
        // only the inserted layout is written back, until Standard changes.
        let layout = |name: &str, left: i64| {
            format!(
                "[lay]\r\n\t{name}\r\n\t513\r\n\t[rght]\r\n\t\t15840\r\n\t\t12240\r\n\t\t1\r\n\t\t{left}\r\n\t\t1440\r\n\t\t1\r\n\t\t1440\r\n\t\t1440\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t1\r\n\t\t0\r\n\t\t2\r\n\t\t1\r\n\t\t{left}\r\n\t\t10800\r\n\t\t0\r\n[elay]\r\n"
            )
        };
        let body = "[tag]\r\n\tBody Text\r\n\t2\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n";
        let sheet = format!(
            "[ver]\r\n\t4\r\n{body}{}[edoc]\r\n>\r\n",
            layout("Standard", 1440)
        );
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t_x.sty\r\n{}[l1]\r\n\t1\r\n[edoc]\r\nText\r\n\r\n>\r\n",
            layout("1", 2880)
        );
        let mut document = import_sam(sam.as_bytes(), Some(sheet.as_bytes()), "Sheet")
            .unwrap()
            .document;
        let names: Vec<&str> = document.layouts().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Standard", "1"]);
        assert!(document.style_sheet().unwrap().layout_from_sheet);
        assert_eq!(document.start_layout().name, "1");
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert_eq!(written.matches("[lay]").count(), 1);
        assert!(written.contains("[lay]\r\n\t1\r\n") && written.contains("[l1]\r\n\t1\r\n"));
        // Changed, Standard is the document's now, as Ami Pro writes it.
        document.document_layout_mut().right.layout.margins.top = 2_000;
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        assert_eq!(written.matches("[lay]").count(), 2);
        assert!(written.contains("[lay]\r\n\tStandard\r\n"));
    }

    #[test]
    fn a_linked_document_stores_its_own_styles_in_ami_pros_order() {
        // Function keys first, by key; then the others as they were read.
        let record = |name: &str, key: u8| {
            format!(
                "[tag]\r\n\t{name}\r\n\t{key}\r\n\t[fnt]\r\n\t\tArial\r\n\t\t240\r\n\t\t0\r\n\t\t16384\r\n"
            )
        };
        let sheet = format!("[ver]\r\n\t4\r\n{}[edoc]\r\n>\r\n", record("Body Text", 2));
        let sam = format!(
            "[ver]\r\n\t4\r\n[sty]\r\n\t_x.sty\r\n{}{}{}{}[edoc]\r\n@Zeta@One\r\n\r\n>\r\n",
            record("Zeta", 0),
            record("Alpha", 0),
            record("Third", 3),
            record("Second", 2)
        );
        let document = import_sam(sam.as_bytes(), Some(sheet.as_bytes()), "Linked")
            .unwrap()
            .document;
        let written = String::from_utf8(sam_document(&document, &OPTIONS)).unwrap();
        let names: Vec<&str> = written
            .split("[tag]\r\n\t")
            .skip(1)
            .map(|record| record.split("\r\n").next().unwrap())
            .collect();
        assert_eq!(names, ["Second", "Third", "Zeta", "Alpha"]);
        assert!(written.contains("[sty]\r\n\t_x.sty\r\n"));
        assert!(!written.contains("[lay]"), "the page layout is the sheet's");
    }
}
