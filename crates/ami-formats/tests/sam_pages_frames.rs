//! SAM import of pages, layouts, frames, and marks
//! (`docs/research/SAM_PAGE_FRAME_SPEC.md`), on a synthetic fixture written
//! from the specification, on truncated and garbled input, and, when
//! `AMIPRO_VM_LAB` names the private fixture directory, on files Ami Pro 3.1
//! wrote in the XP VM.

use std::path::PathBuf;

use amipro_document::{
    Alignment, BreakKind, CellContent, CellMargins, CellSpan, Document, FlowId, FrameContent,
    FrameId, HeaderFooterKind, InlineObject, LinePosition, LineSpacing, PageLayout, PageSide,
    PageSides, PictureSource, Placement, Rect, RepeatPages, Size, TabKind, TabStop, Wrap,
};
use amipro_formats::{SamImport, import_sam, import_sam_path};

fn fixture() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages-and-frames.sam"),
    )
    .unwrap()
}

fn text(document: &Document, flow: FlowId) -> String {
    document.flow(flow).unwrap().plain_text()
}

fn text_flow(document: &Document, frame: FrameId) -> FlowId {
    match document.frame(frame).unwrap().content {
        FrameContent::Text(flow) => flow,
        ref other => panic!("expected text, found {other:?}"),
    }
}

fn layout<'a>(document: &'a Document, name: &str) -> &'a PageLayout {
    document
        .layouts()
        .iter()
        .find(|layout| layout.name == name)
        .unwrap_or_else(|| panic!("layout {name}"))
}

#[test]
fn synthetic_fixture_imports_layouts_frames_and_marks() {
    let SamImport { document, report } = import_sam(&fixture(), None, "Fixture").unwrap();

    // Layouts.
    assert_eq!(document.layouts().len(), 2);
    let standard = layout(&document, "Standard");
    assert!(standard.header_from_second_page && !standard.footer_from_second_page);
    assert_eq!(standard.right.layout.margins.left, 1_440);
    assert_eq!(
        standard.right.layout.tabs,
        [vec![TabStop::new(TabKind::Left, 720)]]
    );
    assert_eq!(standard.left.as_ref().unwrap().layout.margins.left, 2_160);
    let header = standard.header.right.unwrap();
    assert_eq!(
        document.frame(header).unwrap().placement,
        Placement::Header {
            side: PageSide::Right
        }
    );
    let header_flow = text_flow(&document, header);
    assert_eq!(text(&document, header_flow), "Header text.\t\u{FFFC}");
    assert!(matches!(
        &document.flow(header_flow).unwrap().paragraphs[0].objects[..],
        [InlineObject::PageNumber(field)] if field.style == '1' && field.start_page == 0 && field.start_number == 0 && field.leading_text == "Page "
    ));
    let header_style = &document.frame(header).unwrap().style;
    assert_eq!(header_style.interior.margins.top, 792);
    assert_eq!(header_style.interior.margins.bottom, 72);
    let left_header = standard.header.left.unwrap();
    assert_eq!(
        text(&document, text_flow(&document, left_header)),
        "Left header."
    );
    assert_eq!(standard.repeating.len(), 1);
    let repeating = document.frame(standard.repeating[0]).unwrap();
    assert_eq!(
        repeating.placement,
        Placement::Repeating {
            pages: RepeatPages::RightLeft,
            rect: Rect::new(720, 720, 1_440, 1_440)
        }
    );
    let inserted = layout(&document, "1");
    assert_eq!(inserted.right.layout.margins.left, 2_880);
    assert!(inserted.left.is_none());

    // Page frames, back to front.
    let stack = document.frame_stack().to_vec();
    assert_eq!(stack.len(), 6);
    let first = document.frame(stack[0]).unwrap();
    assert_eq!(
        first.placement,
        Placement::Fixed {
            page: 1,
            rect: Rect::new(2_160, 2_880, 5_040, 5_040)
        }
    );
    assert_eq!(first.style.name.as_deref(), Some("Frame3"));
    assert_eq!(first.style.macro_name.as_deref(), Some("RUN.SMM"));
    assert_eq!(first.style.wrap, Wrap::Around);
    assert!(first.style.opaque);
    assert_eq!(first.style.corners, 18);
    assert_eq!(
        (first.style.shadow.right, first.style.shadow.bottom),
        (100, 100)
    );
    let interior = &first.style.interior;
    assert_eq!((interior.columns, interior.gutter), (2, 240));
    assert_eq!(interior.margins.top, 144);
    assert_eq!(
        interior.tabs,
        [Vec::new(), vec![TabStop::new(TabKind::Left, 720)]]
    );
    let frame_text = document.flow(text_flow(&document, stack[0])).unwrap();
    assert_eq!(frame_text.plain_text(), "Frame text here.");
    assert_eq!(
        (
            frame_text.paragraphs[0].runs[0].start,
            frame_text.paragraphs[0].runs[0].end
        ),
        (6, 10)
    );

    let group = document.frame(stack[1]).unwrap().group;
    assert!(group.is_some());
    assert_eq!(document.frame(stack[2]).unwrap().group, group);

    let below = document.frame(stack[3]).unwrap();
    assert!(matches!(
        below.placement,
        Placement::BelowParagraph { x_offset: 720, size } if size.width == 2_880 && size.height == 1_440
    ));
    assert_eq!(below.style.wrap, Wrap::NoWrapAround);
    let border = below.style.border;
    assert!(border.sides.top && border.sides.bottom && !border.sides.left);
    assert_eq!((border.style, border.position), (2, LinePosition::Inside));
    assert_eq!(below.style.shadow.colour, 8_421_504);
    assert_eq!(below.style.background, 12_632_256);

    let table_frame = document.frame(stack[4]).unwrap();
    // A table frame's field 16 is stale (`0 65535` as TB1 writes it); its
    // size is the stored rectangle.
    assert_eq!(
        table_frame.placement,
        Placement::Inline {
            y_offset: 0,
            size: Size {
                width: 9_353,
                height: 800,
            },
        }
    );
    let FrameContent::Table(table) = &table_frame.content else {
        panic!("expected a table");
    };
    assert_eq!(table.columns, [4_569, 4_784]);
    assert_eq!(table.rows, [284, 568]);
    let cell = |index: usize| {
        let CellContent::Text(flow) = table.cells[index].content;
        document.flow(flow).unwrap()
    };
    assert_eq!(cell(0).plain_text(), "A1");
    assert_eq!(cell(1).plain_text(), "", "the covered cell is empty");
    assert_eq!(cell(2).plain_text(), "3250.00");
    assert_eq!(table.cells[2].legacy.get("cell.body"), Some("<+A>a1*2"));
    assert_eq!(cell(3).plain_text(), "B2\nsecond paragraph");

    let picture = document.frame(stack[5]).unwrap();
    let FrameContent::Picture(picture) = &picture.content else {
        panic!("expected a picture");
    };
    let PictureSource::Embedded(bytes) = picture.source else {
        panic!("expected embedded bytes");
    };
    assert_eq!(
        &*document.objects()[&bytes].bytes,
        b"BM synthetic picture bytes"
    );
    assert_eq!(
        &*document.objects()[&picture.rendering.unwrap()].bytes,
        b"SS synthetic rendering"
    );
    assert_eq!(picture.extension, ".bmp");

    // The main text.
    let paragraphs = document.paragraphs();
    assert_eq!(
        paragraphs[0].text,
        "Pages and frames\u{2028}second line co\u{AD}op.\u{FFFC}"
    );
    assert_eq!(document.styles()[&paragraphs[0].style].name, "Heading");
    assert_eq!(paragraphs[0].objects, [InlineObject::FrameAnchor(stack[3])]);
    assert!(
        paragraphs[0].marks.is_empty(),
        "the layout cache is dropped"
    );
    assert_eq!(
        paragraphs[1].text,
        "\u{FFFC}Table paragraph with a font change.\u{FFFC}"
    );
    assert_eq!(
        paragraphs[1].objects,
        [
            InlineObject::FrameAnchor(stack[4]),
            InlineObject::Break {
                kind: BreakKind::Column,
                centre: false,
                layout: None,
                restart: false,
            }
        ]
    );
    // `<:f140, Palatino,0,0,0>`: the face's kind is a space (no kind).
    assert!(paragraphs[1].marks.is_empty());
    let font = &paragraphs[1].runs[0];
    assert_eq!(
        &paragraphs[1].text[font.start..font.end],
        "font change.\u{FFFC}"
    );
    assert_eq!(
        (
            font.format.font_family.as_deref(),
            font.format.font_size_points,
            font.format.colour
        ),
        (Some("Palatino"), Some(7.0), Some(0))
    );
    let InlineObject::FloatingHeaderFooter { kind, pages, flow } = &paragraphs[2].objects[0] else {
        panic!("expected a floating footer");
    };
    assert_eq!((*kind, *pages), (HeaderFooterKind::Footer, PageSides::All));
    assert_eq!(text(&document, *flow), "Floating footer text.");
    assert_eq!(
        paragraphs[3].objects,
        [InlineObject::Break {
            kind: BreakKind::Page,
            centre: false,
            layout: Some(inserted.id),
            restart: false,
        }]
    );
    assert_eq!(paragraphs[4].text, "Last <literal> text.");

    // Document-level data and the report.
    let legacy: Vec<_> = document
        .legacy()
        .values
        .iter()
        .map(|value| value.key.as_str())
        .collect();
    assert!(legacy.contains(&"section [ver]"));
    assert!(
        !legacy.contains(&"section [pg]"),
        "the page table is a cache"
    );
    assert!(
        !legacy.contains(&"section [elay]") && !legacy.contains(&"section [tag]"),
        "style records and the layouts' end are structure, not kept text"
    );
    let stats = &report.structure;
    assert_eq!((stats.anchors_resolved, stats.anchors_unresolved), (2, 0));
    assert_eq!(stats.layouts, 2);
    assert_eq!(stats.dropped_caches, 1);
    assert_eq!(stats.embedded_objects, 2);
    // 0x10000 is lines on: the frame's sides count only with it.
    assert_eq!(stats.unknown_frame_flags & 0x1_0000, 0);

    // Everything survives native v3.
    let reopened = Document::decode_native(&document.encode_native().unwrap()).unwrap();
    assert_eq!(reopened, document);
}

/// A With para above table's field 15 is not an offset: MERCURY.SAM writes
/// 20000 for a table that Ami Pro shows centred at its stored rectangle, and
/// corpus tables at the margin write values up to 19456. Its offset comes
/// from the rectangle, like its size.
#[test]
fn with_para_above_tables_take_their_offset_from_the_stored_rectangle() {
    let table = |left: u32, right: u32, field15: u32| {
        format!(
            "[frm]\n\t1\n\t1572932\n\t{left}\n\t5223\n\t{right}\n\t7707\n\t1\n\t1\n\t1\n\
             \t1 0 0 0 0 0 0\n\t0\n\t16777215\n\t1\n\t0\n\t{field15}\n\t2109 21024\n\
             \t[tbl]\n\t\t 1 1 284 86 {} 86 1 43 43\n\t\t[data]\n\
             \t\t\t 0 0 0 0 0 0 0 1 0 0 0 0\nCell\n\n>\n\t\t[e]\n\t[tble]\n",
            right - left
        )
    };
    // Letter with 1-inch margins: the column is 1,440 to 10,800, so a
    // 5,488-twip table is centred at 3,376.
    let sam = format!(
        "[ver]\n\t4\n{}{}[edoc]\nBefore.\n\n<:t0>Middle.\n\n<:t1>After.\n\n>\n",
        table(3376, 8864, 20000),
        table(1440, 6928, 19456)
    );
    let SamImport { document, .. } = import_sam(sam.as_bytes(), None, "Tables").unwrap();
    let offsets: Vec<_> = document
        .frame_stack()
        .iter()
        .map(|id| document.frame(*id).unwrap().placement)
        .collect();
    let size = Size {
        width: 5_488,
        height: 2_484,
    };
    assert_eq!(
        offsets,
        [
            Placement::BelowParagraph {
                x_offset: 1_936,
                size
            },
            Placement::BelowParagraph { x_offset: 0, size },
        ]
    );
}

/// A With para above frame's height is its stored rectangle's: field 16's
/// height goes stale (corpus text frames store 0 to 360 for rectangles
/// thousands of twips tall, and Ami Pro lays pictures out at the
/// rectangle's height). A degenerate rectangle keeps field 16.
#[test]
fn with_para_above_frames_take_their_height_from_the_stored_rectangle() {
    let frame = |top: u32, bottom: u32| {
        format!(
            "[frm]\n\t1\n\t524288\n\t2160\n\t{top}\n\t5040\n\t{bottom}\n\t1\n\t1\n\t1\n\
             \t1 0 0 0 0 0 0\n\t0\n\t16777215\n\t1\n\t0\n\t720\n\t2880 360\n\
             \t[txt]\nFrame.\n\n>\n"
        )
    };
    let sam = format!(
        "[ver]\n\t4\n{}{}[edoc]\nBefore.\n\n<:A0>Middle.\n\n<:A1>After.\n\n>\n",
        frame(4000, 6160),
        frame(4000, 4000)
    );
    let SamImport { document, .. } = import_sam(sam.as_bytes(), None, "Heights").unwrap();
    let sizes: Vec<_> = document
        .frame_stack()
        .iter()
        .map(|id| document.frame(*id).unwrap().placement)
        .collect();
    let size = |height| Size {
        width: 2_880,
        height,
    };
    assert_eq!(
        sizes,
        [
            Placement::BelowParagraph {
                x_offset: 720,
                size: size(2_160)
            },
            Placement::BelowParagraph {
                x_offset: 720,
                size: size(360)
            },
        ]
    );
}

/// A Flow with text frame's field 16 holds its width and the distance from
/// its top to the baseline (F18, F19): one dragged down 725 twips hangs that
/// far below the baseline, and the rectangle gives its height.
#[test]
fn flow_with_text_frames_hang_below_the_baseline_by_their_stored_distance() {
    let frame = |field16: &str| {
        format!(
            "[frm]\n\t1\n\t590528\n\t3758\n\t2008\n\t6638\n\t4893\n\t1\n\t1\n\t1\n\
             \t1 0 0 100 100 18 0\n\t0\n\t16777215\n\t3\n\t0\n\t2318\n\t{field16}\n\
             \t[frmlay]\n\t\t2885\n\t\t2880\n\t\t1\n\t\t144\n\t\t144\n\t\t1\n\t\t2592\n\
             \t\t144\n\t\t0\n\t\t2\n\t\t0\n\t\t160\n\t\t5\n\t\t14472\n\t\t1\n\
             \t\t3902\n\t\t6494\n\t\t0\n\t[txt]\n\n>\n"
        )
    };
    let sam = format!(
        "[ver]\n\t4\n{}{}[edoc]\nOne.<:A0>\n\nTwo.<:A1>\n\n>\n",
        frame("2880 1435"),
        frame("2880 2885")
    );
    let SamImport { document, .. } = import_sam(sam.as_bytes(), None, "Inline").unwrap();
    let placements: Vec<_> = document
        .frame_stack()
        .iter()
        .map(|id| document.frame(*id).unwrap().placement)
        .collect();
    let size = Size {
        width: 2_880,
        height: 2_885,
    };
    assert_eq!(
        placements,
        [
            Placement::Inline {
                y_offset: 1_450,
                size
            },
            Placement::Inline { y_offset: 0, size },
        ]
    );
}

/// Tables keep their cell margins (header values 4 and 6) and joined cells
/// (cell field 3: 0x180 starts a block of fields 4 × 5 cells, 0x80 is
/// covered), and their untagged text is Table Text, as the user's guide says
/// Ami Pro assigns to all cells (README31.SAM's merge-field table).
#[test]
fn tables_keep_cell_margins_joined_cells_and_table_text() {
    let frame = |rows: &str, data: &str| {
        format!(
            "[frm]\n\t1\n\t1572932\n\t1440\n\t3320\n\t7199\n\t5091\n\t1\n\t1\n\t1\n\
             \t1 0 0 0 0 0 0\n\t0\n\t16777215\n\t1\n\t0\n\t0\n\t0 65535\n\
             \t[tbl]\n\t\t {rows} 2 312 0 2772 86 1 43 43\n\t\t[data]\n{data}\t\t[e]\n\t[tble]\n"
        )
    };
    let joined = frame(
        "3",
        "\t\t\t 0 0 0 0 0 0 512 1 0 0 0 0\nMerge Field\n\n>\n\
         \t\t\t 0 1 0 0 0 0 512 1 0 0 0 0\n@Body Text@Tagged\n\n>\n\
         \t\t\t 1 0 384 1 2 0 0 1 0 0 0 0\nInserts the merge field name\n\n>\n\
         \t\t\t 1 1 128 0 1 0 0 0 0 0 0 0\n",
    );
    // A covered cell with no joined cell over it.
    let orphan = frame("1", "\t\t\t 0 1 128 0 1 0 0 0 0 0 0 0\n");
    let sam = format!(
        "[ver]\n\t4\n[tag]\n\tBody Text\n\t2\n\t[fnt]\n\t\tArial\n\t\t200\n\t\t0\n\t\t16384\n\
         [tag]\n\tTable Text\n\t5\n\t[fnt]\n\t\tArial\n\t\t180\n\t\t0\n\t\t16384\n\
         {joined}{orphan}[edoc]\nBefore.\n\n<:t0>After.\n\n<:t1>End.\n\n>\n"
    );
    let SamImport { document, report } = import_sam(sam.as_bytes(), None, "Joined").unwrap();
    let table_of = |index: usize| {
        let FrameContent::Table(table) = &document
            .frame(document.frame_stack()[index])
            .unwrap()
            .content
        else {
            panic!("table {index}");
        };
        table
    };
    let table = table_of(0);
    assert_eq!(
        table.cell_margins,
        CellMargins {
            horizontal: 86,
            vertical: 0
        }
    );
    let spans: Vec<_> = table.cells.iter().map(|cell| cell.span).collect();
    let joined = CellSpan {
        rows: 1,
        columns: 2,
    };
    let single = CellSpan::SINGLE;
    assert_eq!(
        spans,
        [single, single, joined, CellSpan::COVERED, single, single]
    );
    let style_of = |cell: usize| {
        let CellContent::Text(flow) = table.cells[cell].content;
        let paragraph = &document.flow(flow).unwrap().paragraphs[0];
        let style = &document.styles()[&paragraph.style];
        (style.name.as_str(), style.spacing.below)
    };
    // Untagged and empty cells are Table Text, which Ami Pro generates with
    // no space above or below; a tagged cell keeps its style.
    assert_eq!(style_of(0), ("Table Text", 0));
    assert_eq!(style_of(1), ("Body Text", 0));
    assert_eq!(style_of(4), ("Table Text", 0));

    assert!(table_of(1).cells.iter().all(|cell| cell.span == single));
    assert!(
        report
            .warnings
            .iter()
            .any(|warning| warning.contains("joined cells do not fit"))
    );
    let reopened = Document::decode_native(&document.encode_native().unwrap()).unwrap();
    assert_eq!(reopened, document);
}

/// Truncated and garbled files must import or fail with an error, never
/// panic, and anything imported must be a valid document.
#[test]
fn truncated_and_garbled_input_never_panics() {
    let mut inputs = vec![fixture()];
    if let Some(directory) = std::env::var_os("AMIPRO_VM_LAB") {
        for name in ["F01", "G21", "P02", "TB1", "V20", "Z04"] {
            if let Ok(bytes) = std::fs::read(PathBuf::from(&directory).join(format!("{name}.SAM")))
            {
                inputs.push(bytes);
            }
        }
    }
    let check = |bytes: &[u8]| {
        if let Ok(import) = import_sam(bytes, None, "Fuzz") {
            import.document.validate().unwrap();
        }
    };
    for input in &inputs {
        for length in (0..input.len()).step_by((input.len() / 400).max(13)) {
            check(&input[..length]);
        }
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        for _ in 0..400 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let mut garbled = input.clone();
            for change in 0..4 {
                let position = ((state >> (change * 13)) as usize) % garbled.len();
                garbled[position] = match (state >> (change * 7)) % 6 {
                    0 => b'\n',
                    1 => b'<',
                    2 => b'>',
                    3 => b'[',
                    4 => b'9',
                    _ => (state >> 3) as u8,
                };
            }
            check(&garbled);
        }
    }
}

/// Private: files Ami Pro 3.1 wrote in the XP VM (`docs/research/VM_LAB.md`).
#[test]
fn vm_fixtures_import_as_ami_pro_wrote_them() {
    let Some(directory) = std::env::var_os("AMIPRO_VM_LAB") else {
        println!("AMIPRO_VM_LAB is not set; skipping the private VM-fixture test");
        return;
    };
    let import = |name: &str| {
        import_sam_path(PathBuf::from(&directory).join(format!("{name}.SAM")))
            .unwrap_or_else(|error| panic!("{name}: {error}"))
    };

    // F01: one Where placed frame, 2.00 × 1.50 in at 1.50, 2.00 in.
    let f01 = import("F01").document;
    let frame = f01.frame(f01.frame_stack()[0]).unwrap();
    assert_eq!(
        frame.placement,
        Placement::Fixed {
            page: 1,
            rect: Rect::new(2_160, 2_880, 5_040, 5_040)
        }
    );
    assert_eq!(text(&f01, text_flow(&f01, frame.id)), "Frame text here.");

    // I1 (2026-10-12): Body Text indented All 0.50, First 1.00, Rest 0.25,
    // From right 1.50 in Ami Pro's Modify Style.
    let i1 = import("I1").document;
    let body = i1
        .styles()
        .values()
        .find(|style| style.name == "Body Text")
        .unwrap();
    assert_eq!(
        body.indents,
        amipro_document::Indents {
            all: 720,
            first: 1_440,
            rest: 360,
            right: 2_160
        }
    );

    // O3 (2026-10-12): Text › Indention (All 0.25, First 0.50, Rest 0.75,
    // From right 1.00 in) and Text › Spacing (1 1/2, Double, Custom 0.30 in,
    // Single) on single paragraphs.
    let o3 = import("O3").document;
    let formats: Vec<_> = o3.paragraphs().iter().map(|p| p.format.clone()).collect();
    assert_eq!(
        formats[0].indents,
        Some(amipro_document::Indents {
            all: 360,
            first: 720,
            rest: 1_080,
            right: 1_440
        })
    );
    assert_eq!(
        formats
            .iter()
            .map(|format| format.line_spacing)
            .collect::<Vec<_>>(),
        [
            None,
            Some(LineSpacing::OneAndAHalf(0)),
            Some(LineSpacing::Double(0)),
            Some(LineSpacing::Custom(432)),
            Some(LineSpacing::Single),
        ]
    );

    // O2 (2026-10-12): Text › Alignment Center, Right, Justify, and Left on
    // four paragraphs; the fifth keeps its style's.
    let o2 = import("O2").document;
    assert_eq!(
        o2.paragraphs()
            .iter()
            .map(|p| p.format.alignment)
            .collect::<Vec<_>>(),
        [
            Some(Alignment::Centre),
            Some(Alignment::Right),
            Some(Alignment::Justify),
            Some(Alignment::Left),
            None,
        ]
    );
    // I2: Modify Style › Alignment Justify (with Both sides and Hanging
    // indent: 536).
    let i2 = import("I2").document;
    assert!(
        i2.styles()
            .values()
            .any(|style| style.name == "Body Text" && style.alignment == Alignment::Justify)
    );

    // F14–F20 (2026-10-12): frame gestures in Ami Pro. A drag from inside
    // moves a Where placed frame by the pointer's offset (88, 44 pixels at
    // 87.4 per inch); a corner handle moves only its two edges.
    let placement_of = |name: &str| {
        let document = import(name).document;
        let id = document.frame_stack()[0];
        let frame = document.frame(id).unwrap();
        (frame.placement, frame.style.wrap)
    };
    assert_eq!(
        placement_of("F14").0,
        Placement::Fixed {
            page: 1,
            rect: Rect::new(3_610, 3_605, 6_490, 5_765)
        }
    );
    assert_eq!(
        placement_of("F15").0,
        Placement::Fixed {
            page: 1,
            rect: Rect::new(3_610, 3_605, 7_215, 6_128)
        }
    );
    // With para above: a diagonal drag moves it across only, and its left
    // handle moves the left edge (offset and width together). Ami Pro chose
    // No wrap beside when the placement changed.
    let size = |width, height| Size { width, height };
    assert_eq!(
        placement_of("F16"),
        (
            Placement::BelowParagraph {
                x_offset: 2_170,
                size: size(2_880, 2_160)
            },
            Wrap::NoWrapBeside
        )
    );
    assert_eq!(
        placement_of("F17").0,
        Placement::BelowParagraph {
            x_offset: 1_445,
            size: size(3_605, 2_160)
        }
    );
    // Flow with text: dragged down 725 twips it hangs below the baseline;
    // made taller from the bottom, it keeps its top's height above it.
    assert_eq!(
        placement_of("F18").0,
        Placement::Inline {
            y_offset: 725,
            size: size(2_880, 2_160)
        }
    );
    assert_eq!(
        placement_of("F19").0,
        Placement::Inline {
            y_offset: 1_450,
            size: size(2_880, 2_885)
        }
    );

    // G21: an inserted layout named "1" and the break that switches to it.
    let g21 = import("G21").document;
    let names: Vec<_> = g21
        .layouts()
        .iter()
        .map(|layout| layout.name.as_str())
        .collect();
    assert_eq!(names, ["Standard", "1"]);
    let inserted = g21.layouts()[1].id;
    let breaks: Vec<_> = g21
        .paragraphs()
        .iter()
        .flat_map(|paragraph| paragraph.objects.clone())
        .collect();
    assert_eq!(
        breaks,
        [
            InlineObject::Break {
                kind: BreakKind::Page,
                centre: false,
                layout: None,
                restart: false,
            },
            InlineObject::Break {
                kind: BreakKind::Page,
                centre: false,
                layout: Some(inserted),
                restart: false,
            }
        ]
    );

    // V20: `<:t n>` and `<:A n>` share one numbering over anchored records.
    let v20 = import("V20").document;
    let stack = v20.frame_stack().to_vec();
    let paragraphs = v20.paragraphs();
    assert_eq!(
        paragraphs[1].objects,
        [
            InlineObject::FrameAnchor(stack[2]),
            InlineObject::FrameAnchor(stack[1])
        ]
    );
    assert_eq!(paragraphs[3].objects, [InlineObject::FrameAnchor(stack[0])]);
    assert!(matches!(
        v20.frame(stack[1]).unwrap().placement,
        Placement::Inline { .. }
    ));

    // P02: the copied 64 × 48 BMP is stored byte for byte.
    let p02 = import("P02").document;
    let picture = p02.frame(p02.frame_stack()[0]).unwrap();
    let FrameContent::Picture(picture) = &picture.content else {
        panic!("P02 holds a picture");
    };
    let PictureSource::Embedded(bytes) = picture.source else {
        panic!("P02's picture is copied in");
    };
    assert_eq!(p02.objects()[&bytes].bytes.len(), 9_270);
    assert_eq!(&p02.objects()[&bytes].bytes[..2], b"BM");
    assert_eq!(
        p02.objects()[&picture.rendering.unwrap()].bytes.len(),
        64_646
    );

    // V17: a floating footer for all pages.
    let v17 = import("V17").document;
    let floating = v17
        .paragraphs()
        .iter()
        .flat_map(|paragraph| &paragraph.objects)
        .find_map(|object| match object {
            InlineObject::FloatingHeaderFooter { kind, pages, flow } => {
                Some((*kind, *pages, *flow))
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(
        (floating.0, floating.1),
        (HeaderFooterKind::Footer, PageSides::All)
    );
    assert_eq!(text(&v17, floating.2), "FOOT ALL");

    // TB1: a 2 × 2 table in the main text.
    let tb1 = import("TB1").document;
    let FrameContent::Table(table) = &tb1.frame(tb1.frame_stack()[0]).unwrap().content else {
        panic!("TB1 holds a table");
    };
    let cells: Vec<_> = table
        .cells
        .iter()
        .map(|cell| {
            let CellContent::Text(flow) = cell.content;
            text(&tb1, flow)
        })
        .collect();
    assert_eq!(cells, ["A1", "B1", "A2", "B2"]);
    assert!(matches!(
        tb1.frame(tb1.frame_stack()[0]).unwrap().placement,
        Placement::Inline {
            size: Size {
                width: 9_353,
                height: 783,
            },
            ..
        }
    ));

    // G15: left pages with a 1.50 in left margin; Z04: one group of three.
    let g15 = import("G15").document;
    let standard = g15.document_layout();
    assert_eq!(standard.left.as_ref().unwrap().layout.margins.left, 2_160);
    assert!(standard.header.left.is_some() && standard.footer.left.is_some());
    let z04 = import("Z04").document;
    let groups: Vec<_> = z04
        .frame_stack()
        .iter()
        .map(|frame| z04.frame(*frame).unwrap().group)
        .collect();
    assert!(
        groups
            .iter()
            .all(|group| group.is_some() && *group == groups[0])
    );
    assert_eq!(groups.len(), 3);

    for name in ["F01", "G21", "V20", "P02", "V17", "TB1", "G15", "Z04"] {
        let document = import(name).document;
        let reopened = Document::decode_native(&document.encode_native().unwrap()).unwrap();
        assert_eq!(reopened, document, "{name}: v3 round trip");
    }
}
