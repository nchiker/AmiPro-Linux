//! Charts in documents (`docs/OBJECTS_RUNBOOK.md`, DO.6 and DO.7): a chart
//! made by the generator sits in a frame, is written as an Ami Pro
//! document and read back with its data record and objects, and is kept by
//! the native format. The twin for Ami Pro (CHV1: does it accept objects
//! the app wrote, and open the Charting dialog on the record?) is written
//! when `AMIPRO_TWIN_OUT` names a folder.

use amipro_document::{Document, FrameContent, FrameStyle, PictureFormat, Placement, Rect};
use amipro_editing::drawing;
use amipro_editing::{Command, NewFrameContent, apply};
use amipro_formats::{SamOptions, import_sam, sam_document, sam_losses};
use amipro_pictures::chart::{Chart, Icon, chart_of};
use amipro_pictures::chart_gen::{Metrics, generate_drawing};

/// The frame's width and height in twips.
fn frame_sizes(document: &Document, frame: amipro_document::FrameId) -> (i32, i32) {
    match &document.frame(frame).unwrap().placement {
        Placement::Fixed { rect, .. } => (rect.right - rect.left, rect.bottom - rect.top),
        _ => (4608, 3888),
    }
}
use amipro_pictures::sdw;

const OPTIONS: SamOptions = SamOptions::at(1_791_958_088);

/// A 3.2 by 2.7 inch frame (CHV2's size) holding `chart`.
fn with_chart(chart: &Chart) -> (Document, amipro_document::FrameId) {
    let metrics = Metrics::for_picture_area(4608 - 288, 3888 - 288);
    let bytes = sdw::write(&generate_drawing(chart, &metrics));
    let document = Document::from_plain_text("A chart follows.");
    let applied = apply(
        &document,
        &Command::CreateFrame {
            placement: Placement::Fixed {
                page: 1,
                rect: Rect::new(1296, 1152, 1296 + 4608, 1152 + 3888),
            },
            style: FrameStyle::default(),
            content: NewFrameContent::EmbeddedPicture {
                bytes: bytes.into(),
                extension: ".sct".into(),
            },
            anchor: None,
            layout: None,
        },
    )
    .unwrap();
    let frame = *applied.report.frames.iter().next().unwrap();
    (applied.document, frame)
}

/// Data 12, 23, 5, 14 as a 3D clustered column (the twin for CHV1).
fn twin_chart() -> Chart {
    let mut chart = Chart::new(Icon::Column, 4, 1);
    chart.values = vec![12.0, 23.0, 5.0, 14.0];
    chart.oblique_depth = 50;
    chart.legend = 0;
    chart.grid = 1;
    chart
}

#[test]
fn a_generated_chart_is_written_for_ami_pro_and_read_back() {
    let (document, frame) = with_chart(&twin_chart());
    let FrameContent::Picture(picture) = &document.frame(frame).unwrap().content else {
        panic!();
    };
    assert_eq!(picture.format(), PictureFormat::Chart);
    let made = drawing::drawing_of(&document, frame).unwrap();
    let (index, record) = chart_of(&made).unwrap();
    // The record is the last object, flag 4, and carries the axis (23 in a
    // 2627-twip plot: five-unit steps up to 25).
    assert_eq!(index, made.objects.len() - 1);
    assert_eq!(made.objects[index].flags, sdw::CHART_FLAG);
    assert_eq!(
        (record.axis.min, record.axis.max, record.axis.step),
        (0.0, 25.0, 5.0)
    );
    assert!(made.objects.iter().all(|o| o.flags == sdw::CHART_FLAG));

    let written = sam_document(&document, &OPTIONS);
    let copy = import_sam(&written, None, "Chart").unwrap().document;
    let copied = copy.frame_stack()[0];
    let again = drawing::drawing_of(&copy, copied).unwrap();
    assert_eq!(again, made);
    let FrameContent::Picture(read) = &copy.frame(copied).unwrap().content else {
        panic!();
    };
    assert_eq!(read.extension, ".sdw");
    assert_eq!(read.format(), PictureFormat::Chart);
    assert!(sam_losses(&document, &OPTIONS).unwrap().is_empty());

    // For the VM (CHV1): Ami Pro opens the written document.
    if let Some(out) = std::env::var_os("AMIPRO_TWIN_OUT") {
        let out = std::path::PathBuf::from(out);
        std::fs::write(out.join("CHT1.SAM"), &written).unwrap();
    }
}

#[test]
fn the_native_format_keeps_the_chart_state() {
    let mut chart = twin_chart();
    chart.row_labels = vec![amipro_pictures::chart::Label::new("Big Sales")];
    chart.column_labels = ["Q1", "Q2", "Q3", "Q4"]
        .iter()
        .map(|name| amipro_pictures::chart::Label::new(name))
        .collect();
    chart.palette[2] = [9, 8, 7, 6];
    chart.last_word = 0x2003;
    let (document, frame) = with_chart(&chart);
    let before = drawing::drawing_of(&document, frame).unwrap();
    let encoded = document.encode_native().unwrap();
    let decoded = Document::decode_native(&encoded).unwrap();
    let decoded_frame = decoded.frame_stack()[0];
    let after = drawing::drawing_of(&decoded, decoded_frame).unwrap();
    assert_eq!(after, before);
    let (_, record) = chart_of(&after).unwrap();
    assert_eq!(record.column_labels[3].shown(), "Q4");
    assert_eq!(record.palette[2], [9, 8, 7, 6]);
    assert_eq!(record.last_word, 0x2003);
    // Everything in the record that is not regenerated is the same.
    assert_eq!(record.values, chart.values);
    assert_eq!(record.row_labels, chart.row_labels);
}

// ---- CHV1 twins: one file per CHV2 file, the same settings -------------

/// One chart in a frame: (type, variation, legend, grid, oblique depth,
/// perspective depth, last word), the frame (left, top, width, height).
type Setting = (u16, u16, u16, u16, u16, u16, u16);
/// A frame: left, top, width, height in twips.
type Place = (i32, i32, i32, i32);
/// Six settings with an optional frame size.
type Sheet = [(Setting, Option<(i32, i32)>); 6];

fn labelled(rows: &[(&str, &[f32])], columns: &[&str]) -> Chart {
    let mut chart = Chart::new(Icon::Column, columns.len(), rows.len());
    chart.values = rows.iter().flat_map(|(_, v)| v.iter().copied()).collect();
    chart.row_labels = rows
        .iter()
        .map(|(n, _)| amipro_pictures::chart::Label::new(n))
        .collect();
    chart.column_labels = columns
        .iter()
        .map(|n| amipro_pictures::chart::Label::new(n))
        .collect();
    chart
}

/// CHV2's 3 x 2 data set.
fn data_3x2() -> Chart {
    labelled(
        &[("East", &[10.0, 20.0, 30.0]), ("West", &[15.0, 5.0, 25.0])],
        &["Jan", "Feb", "Mar"],
    )
}

fn set(mut chart: Chart, s: Setting) -> Chart {
    chart.chart_type = amipro_pictures::chart::ChartType::from_code(s.0);
    chart.variation = s.1;
    chart.legend = s.2;
    chart.grid = s.3;
    chart.oblique_depth = s.4;
    chart.perspective_depth = s.5;
    chart.last_word = s.6;
    chart
}

/// A document with each chart in a fixed frame of page 1.
fn with_charts(charts: &[(Chart, Place)]) -> Document {
    let mut document = Document::from_plain_text("Charts follow.");
    for (chart, (left, top, width, height)) in charts {
        let metrics = Metrics::for_picture_area(*width - 288, *height - 288);
        let bytes = sdw::write(&generate_drawing(chart, &metrics));
        document = apply(
            &document,
            &Command::CreateFrame {
                placement: Placement::Fixed {
                    page: 1,
                    rect: Rect::new(*left, *top, left + width, top + height),
                },
                style: FrameStyle::default(),
                content: NewFrameContent::EmbeddedPicture {
                    bytes: bytes.into(),
                    extension: ".sct".into(),
                },
                anchor: None,
                layout: None,
            },
        )
        .unwrap()
        .document;
    }
    document
}

/// CHV2's six frames of a sheet: 3.2 x 2.7 inches in two columns of three.
fn six(sheet: Sheet, base: &Chart) -> Vec<(Chart, Place)> {
    let spots = [
        (1296, 1152),
        (6192, 1152),
        (1296, 5328),
        (6192, 5328),
        (1296, 9504),
        (6192, 9504),
    ];
    sheet
        .iter()
        .zip(spots)
        .map(|((setting, size), (left, top))| {
            let (w, h) = size.unwrap_or((4608, 3888));
            (set(base.clone(), *setting), (left, top, w, h))
        })
        .collect()
}

/// The records of CHV2A, CHV2B and CHV2C as read back from Ami Pro.
fn chv2_sheets() -> [Sheet; 3] {
    [
        [
            ((0, 6, 1, 1, 0, 0, 0x1006), None),
            ((0, 0, 1, 0, 50, 0, 0x0100), None),
            ((3, 6, 0, 0, 0, 0, 0x0006), None),
            ((3, 0, 1, 0, 0, 50, 0x0100), None),
            ((1, 0, 1, 1, 0, 0, 0x0000), None),
            ((1, 6, 0, 0, 50, 0, 0x0106), None),
        ],
        [
            ((4, 0, 1, 0, 0, 0, 0x0000), None),
            ((4, 3, 1, 0, 0, 0, 0x2003), None),
            ((2, 0, 1, 0, 0, 0, 0x0000), None),
            ((2, 0, 1, 0, 50, 0, 0x0100), None),
            ((5, 6, 1, 0, 0, 0, 0x0006), None),
            ((5, 0, 1, 0, 0, 0, 0x0100), None),
        ],
        [
            ((0, 6, 0, 0, 50, 0, 0x1006), None),
            ((0, 0, 0, 0, 0, 0, 0x1000), None),
            ((1, 6, 0, 0, 0, 0, 0x0106), None),
            ((2, 0, 0, 0, 0, 0, 0x0100), None),
            ((0, 0, 0, 0, 0, 0, 0x0100), None),
            ((0, 6, 0, 0, 0, 0, 0x1006), Some((2880, 2160))),
        ],
    ]
}

/// The files for CHV1 (`AMIPRO_TWIN_OUT=DIR`): CHT1 (the twin of notes 13.4),
/// CHT2A to C (CHV2's eighteen charts with the same records), CHT3 (three
/// series), CHT4 (negative values), CHT5 (four frame sizes).
fn twin_files() -> Vec<(&'static str, Document)> {
    let mut files = vec![("CHT1.SAM", with_chart(&twin_chart()).0)];
    for (name, sheet) in ["CHT2A.SAM", "CHT2B.SAM", "CHT2C.SAM"]
        .into_iter()
        .zip(chv2_sheets())
    {
        files.push((name, with_charts(&six(sheet, &data_3x2()))));
    }
    let three = labelled(
        &[
            ("East", &[10.0, 20.0, 30.0]),
            ("West", &[15.0, 5.0, 25.0]),
            ("North", &[8.0, 12.0, 18.0]),
        ],
        &["Jan", "Feb", "Mar"],
    );
    let negative = labelled(
        &[
            ("East", &[10.0, -20.0, 30.0]),
            ("West", &[-15.0, 5.0, 25.0]),
        ],
        &["Jan", "Feb", "Mar"],
    );
    let sheet = |types: [Setting; 6]| types.map(|s| (s, None));
    // Column (grid, legend), bar, stacked 3D column, line, area, pie.
    let kinds = sheet([
        (0, 6, 1, 1, 0, 0, 0x1006),
        (3, 6, 1, 0, 0, 0, 0x0006),
        (0, 0, 1, 0, 50, 0, 0x0100),
        (1, 0, 1, 1, 0, 0, 0x0000),
        (1, 6, 1, 0, 0, 0, 0x0106),
        (2, 0, 1, 0, 0, 0, 0x0000),
    ]);
    files.push(("CHT3.SAM", with_charts(&six(kinds, &three))));
    let kinds = sheet([
        (0, 6, 1, 1, 0, 0, 0x1006),
        (3, 6, 1, 0, 0, 0, 0x0006),
        (0, 0, 1, 0, 50, 0, 0x0100),
        (1, 0, 1, 1, 0, 0, 0x0000),
        (1, 6, 1, 0, 0, 0, 0x0106),
        (5, 6, 1, 0, 0, 0, 0x0006),
    ]);
    files.push(("CHT4.SAM", with_charts(&six(kinds, &negative))));
    let column = set(data_3x2(), (0, 6, 1, 1, 0, 0, 0x1006));
    files.push((
        "CHT5.SAM",
        with_charts(&[
            (column.clone(), (720, 720, 8640, 5760)),
            (column.clone(), (720, 6480, 5760, 2880)),
            (column.clone(), (720, 10080, 4320, 4320)),
            (column, (5400, 10080, 2160, 4320)),
        ]),
    ));
    files
}

#[test]
fn the_chv1_twins_are_written_and_read_back() {
    let files = twin_files();
    assert_eq!(files.len(), 7);
    for (name, document) in &files {
        let written = sam_document(document, &OPTIONS);
        let copy = import_sam(&written, None, "Chart").unwrap().document;
        assert_eq!(
            copy.frame_stack().len(),
            document.frame_stack().len(),
            "{name}"
        );
        for (frame, copied) in document.frame_stack().iter().zip(copy.frame_stack()) {
            let made = drawing::drawing_of(document, *frame).unwrap();
            assert_eq!(drawing::drawing_of(&copy, *copied).unwrap(), made, "{name}");
            assert!(chart_of(&made).is_some());
        }
        if let Some(out) = std::env::var_os("AMIPRO_TWIN_OUT") {
            std::fs::write(std::path::PathBuf::from(out).join(name), &written).unwrap();
        }
    }
}

// ---- CHV1: Ami Pro's regenerated objects against ours (reads the VM files)

fn flat(objects: &[amipro_pictures::sdw::Object], out: &mut Vec<amipro_pictures::sdw::Object>) {
    for o in objects {
        out.push(o.clone());
        if let amipro_pictures::sdw::Shape::Group { objects, .. } = &o.shape {
            flat(objects, out);
        }
    }
}

fn describe_object(o: &amipro_pictures::sdw::Object) -> String {
    use amipro_pictures::sdw::Shape;
    match &o.shape {
        Shape::Line { points, .. } => {
            let p: Vec<String> = points
                .iter()
                .map(|p| format!("({},{})", p.x, p.y))
                .collect();
            format!("t{} n2 {}", o.kind, p.join(""))
        }
        Shape::Polygon { points, .. } | Shape::Polyline { points, .. } => {
            let p: Vec<String> = points
                .iter()
                .take(if std::env::var_os("CHV1_FULL").is_some() {
                    usize::MAX
                } else {
                    8
                })
                .map(|p| format!("({},{})", p.x, p.y))
                .collect();
            format!("t{} n{} {}", o.kind, points.len(), p.join(""))
        }
        Shape::Text(t) => format!(
            "t{} text '{}' origin({},{})",
            o.kind, t.text, t.origin.x, t.origin.y
        ),
        Shape::Group { objects, .. } => format!("t{} group of {}", o.kind, objects.len()),
        _ => format!("t{}", o.kind),
    }
}

/// `CHV1_OUT=dir` holds the files Ami Pro saved (`CHT1R.SAM` and so on);
/// `CHV1_DIFF=NAME:n` lists the objects of frame n side by side.
#[test]
#[ignore = "reads the VM's saved files"]
fn chv1_ami_pro_regenerations_against_the_app() {
    let dir = std::path::PathBuf::from(
        std::env::var("CHV1_OUT")
            .unwrap_or_else(|_| "../../../AmiPro-Reference/vm-lab/OUT/objects".into()),
    );
    let want = std::env::var("CHV1_DIFF").ok();
    let mut files = twin_files();
    // CHT6: CHT2A's frames 0, 1 and 4 with Flip data ticked in Ami Pro.
    let flipped = {
        let mut sheet = six(chv2_sheets()[0], &data_3x2());
        for index in [0, 1, 4] {
            sheet[index].0.flip();
        }
        with_charts(&sheet)
    };
    files.push(("CHT6.SAM", flipped));
    // CHT7: CHT2A regenerated at the 91 % view (frame 0 only).
    files.push(("CHT7.SAM", files[1].1.clone()));
    for (name, document) in files {
        let stem = name.trim_end_matches(".SAM");
        let path = dir.join(format!("{stem}R.SAM"));
        let Ok(bytes) = std::fs::read(&path) else {
            eprintln!("{stem}: no Ami Pro file");
            continue;
        };
        let theirs = import_sam(&bytes, None, "x").unwrap().document;
        let ours: Vec<_> = document.frame_stack().to_vec();
        let stack = theirs.frame_stack();
        eprintln!(
            "--- {stem}: {} frames ours, {} theirs",
            ours.len(),
            stack.len()
        );
        for (index, (a, b)) in ours.iter().zip(stack.iter()).enumerate() {
            if (stem == "CHT6" && ![0, 1, 4].contains(&index)) || (stem == "CHT7" && index != 0) {
                continue;
            }
            let mine = drawing::drawing_of(&document, *a).unwrap();
            let Some(other) = drawing::drawing_of(&theirs, *b) else {
                eprintln!("{stem}:{index} no drawing");
                continue;
            };
            let (_, ours_chart) = chart_of(&mine).unwrap();
            let Some((_, their_chart)) = chart_of(&other) else {
                eprintln!("{stem}:{index} no chart record in Ami Pro's");
                continue;
            };
            let same_record = ours_chart.to_body() == their_chart.to_body();
            let result = amipro_pictures::chart_gen::compare(&mine.objects, &other.objects, 3);
            // The rectangle Ami Pro used: searched around the picture area
            // (the frame less 144 twips a side), whole twips.
            let (fw, fh) = frame_sizes(&document, *a);
            let start = Metrics::for_frame(fw, fh);
            let (inner_w, inner_h) = (fw - 288, fh - 288);
            let score = |w: i32, h: i32| {
                let m = Metrics {
                    width: w,
                    height: h,
                    text: start.text,
                };
                let r = amipro_pictures::chart_gen::compare(
                    &amipro_pictures::chart_gen::generate(&ours_chart, &m),
                    &other.objects,
                    3,
                );
                (r.within, -(r.mean * 100.0) as i64, r)
            };
            let mut best = (inner_w, inner_h);
            let mut top = score(best.0, best.1);
            for w in (inner_w - 500)..=(inner_w + 500) {
                let t = score(w, best.1);
                if (t.0, t.1) > (top.0, top.1) {
                    top = t;
                    best.0 = w;
                }
            }
            for h in (inner_h - 500)..=(inner_h + 500) {
                let t = score(best.0, h);
                if (t.0, t.1) > (top.0, top.1) {
                    top = t;
                    best.1 = h;
                }
            }
            for w in (best.0 - 6)..=(best.0 + 6) {
                let t = score(w, best.1);
                if (t.0, t.1) > (top.0, top.1) {
                    top = t;
                    best.0 = w;
                }
            }
            if std::env::var_os("CHV1_RULE").is_some() {
                let rule = Metrics::for_picture_area(fw - 288, fh - 288);
                best = (rule.width, rule.height);
                top = score(best.0, best.1);
            }
            eprintln!(
                "{stem}:{index} frame {fw}x{fh} app-rect {}x{} fit-rect {}x{} aligned {} within3 {} worst {} mean {:.1}",
                start.width,
                start.height,
                best.0,
                best.1,
                top.2.aligned,
                top.2.within,
                top.2.worst,
                top.2.mean
            );
            let fitted = Metrics {
                width: best.0,
                height: best.1,
                text: start.text,
            };
            let generated_fit = amipro_pictures::chart_gen::generate(&ours_chart, &fitted);
            let g = &result.generated;
            let s = &result.stored;
            eprintln!(
                "{stem}:{index} record_same {same_record} gen L{} P{} T{} G{} / ami L{} P{} T{} G{} aligned {} within3 {} worst {} mean {:.1}",
                g.lines,
                g.polygons,
                g.texts,
                g.groups,
                s.lines,
                s.polygons,
                s.texts,
                s.groups,
                result.aligned,
                result.within,
                result.worst,
                result.mean
            );
            if !same_record {
                let (x, y) = (ours_chart.to_body(), their_chart.to_body());
                let diffs: Vec<String> = x
                    .iter()
                    .zip(&y)
                    .enumerate()
                    .filter(|(_, (p, q))| p != q)
                    .map(|(i, (p, q))| format!("+{i:#x}:{p:02x}->{q:02x}"))
                    .collect();
                eprintln!(
                    "   record differs (len {} vs {}): {}",
                    x.len(),
                    y.len(),
                    diffs.join(" ")
                );
            }
            if want.as_deref() == Some("all") || want.as_deref() == Some(&format!("{stem}:{index}"))
            {
                eprintln!("=== DIFF {stem}:{index} (A = app at the fitted rectangle, O = Ami Pro)");
                let (mut p, mut q) = (Vec::new(), Vec::new());
                flat(&generated_fit, &mut p);
                flat(&other.objects, &mut q);
                for i in 0..p.len().max(q.len()) {
                    eprintln!(
                        "{i:3} A {}\n    O {}",
                        p.get(i).map(describe_object).unwrap_or_default(),
                        q.get(i).map(describe_object).unwrap_or_default()
                    );
                }
            }
        }
    }
}

/// The before and after of the CHV1 corrections over the 38 charts Ami Pro
/// regenerated: per file, the charts whose counts by kind equal Ami Pro's
/// and the objects within 3 twips of Ami Pro's (of Ami Pro's total), at the
/// rectangle the app used before (`Metrics::for_frame`) and at the picture
/// area (`Metrics::for_picture_area`). `CHV1_SUMMARY=1` also lists charts.
#[test]
#[ignore = "reads the VM's saved files"]
fn chv1_summary_before_and_after() {
    let dir = std::path::PathBuf::from(
        std::env::var("CHV1_OUT")
            .unwrap_or_else(|_| "../../../AmiPro-Reference/vm-lab/OUT/objects".into()),
    );
    let verbose = std::env::var_os("CHV1_SUMMARY").is_some();
    let mut files = twin_files();
    let flipped = {
        let mut sheet = six(chv2_sheets()[0], &data_3x2());
        for index in [0, 1, 4] {
            sheet[index].0.flip();
        }
        with_charts(&sheet)
    };
    files.push(("CHT6.SAM", flipped));
    files.push(("CHT7.SAM", files[1].1.clone()));
    // (charts, counts equal, objects, within) before and after.
    let mut total = [(0usize, 0usize, 0usize, 0usize); 2];
    for (name, document) in files {
        let stem = name.trim_end_matches(".SAM");
        let Ok(bytes) = std::fs::read(dir.join(format!("{stem}R.SAM"))) else {
            continue;
        };
        let theirs = import_sam(&bytes, None, "x").unwrap().document;
        let mut file = [(0usize, 0usize, 0usize, 0usize); 2];
        for (index, (a, b)) in document
            .frame_stack()
            .iter()
            .zip(theirs.frame_stack())
            .enumerate()
        {
            if (stem == "CHT6" && ![0, 1, 4].contains(&index)) || (stem == "CHT7" && index != 0) {
                continue;
            }
            let mine = drawing::drawing_of(&document, *a).unwrap();
            let Some(other) = drawing::drawing_of(&theirs, *b) else {
                continue;
            };
            let (_, chart) = chart_of(&mine).unwrap();
            let (fw, fh) = frame_sizes(&document, *a);
            let before = Metrics::for_frame(fw, fh);
            let after = Metrics::for_picture_area(fw - 288, fh - 288);
            for (slot, metrics) in [before, after].into_iter().enumerate() {
                let r = amipro_pictures::chart_gen::compare(
                    &amipro_pictures::chart_gen::generate(&chart, &metrics),
                    &other.objects,
                    3,
                );
                let t = &mut file[slot];
                t.0 += 1;
                t.1 += usize::from(r.generated == r.stored);
                t.2 += r.stored.total();
                t.3 += r.within;
                if verbose {
                    eprintln!(
                        "SUM {stem}:{index} {} counts_eq {} within {}/{}",
                        ["before", "after"][slot],
                        r.generated == r.stored,
                        r.within,
                        r.stored.total()
                    );
                }
            }
        }
        for slot in 0..2 {
            let t = file[slot];
            eprintln!(
                "SUMMARY {stem} {}: charts {} counts_equal {} within3 {}/{}",
                ["before", "after "][slot],
                t.0,
                t.1,
                t.3,
                t.2
            );
            total[slot].0 += t.0;
            total[slot].1 += t.1;
            total[slot].2 += t.2;
            total[slot].3 += t.3;
        }
    }
    for slot in 0..2 {
        let t = total[slot];
        eprintln!(
            "SUMMARY TOTAL {}: charts {} counts_equal {} within3 {}/{}",
            ["before", "after "][slot],
            t.0,
            t.1,
            t.3,
            t.2
        );
    }
}

/// CHV1: Ami Pro shows a chart frame only when its picture type is `.sdw`.
/// The twin's frame is written the way Ami Pro writes its own chart: the
/// type `.sdw` in `[isd]` and in `[Embedded]`, the driver `.sct` after the
/// saved state, the stored name equal to the frame's, the same flags and
/// unit. (The size, the saved state's words and the data differ.)
#[test]
fn a_chart_frame_is_written_as_ami_pro_writes_its_own() {
    let (document, frame) = with_chart(&twin_chart());
    let written = sam_document(&document, &OPTIONS);
    let ours = isd_of(&written);
    // The type and the driver, in the app's own bytes.
    assert_eq!(ours[1], ".sdw");
    assert_eq!(ours[2], ours[0], "the stored name is the frame's");
    let count: usize = ours[11].parse().unwrap();
    assert_eq!(ours[12 + count], ".sct", "the driver follows the state");
    let embedded = String::from_utf8_lossy(&written);
    let entry = embedded.split("[Embedded]\r\n").nth(1).unwrap();
    assert!(entry.lines().next().unwrap().contains(" .sdw "));
    // Against Ami Pro's own file (CHV2A), where it is present.
    let path = std::path::Path::new("../../../AmiPro-Reference/vm-lab/OUT/objects/CHV2A.SAM");
    if let Ok(theirs) = std::fs::read(path) {
        let theirs_isd = isd_of(&theirs);
        assert_eq!(ours[1], theirs_isd[1], "type");
        assert_eq!(
            theirs_isd[2], theirs_isd[0],
            "Ami Pro's stored name is the frame's too"
        );
        assert_eq!(&ours[3..5], &theirs_isd[3..5], "flags, unit");
        assert_eq!(ours[5..7], theirs_isd[5..7]);
        let their_count: usize = theirs_isd[11].parse().unwrap();
        assert_eq!(ours[12 + count], theirs_isd[12 + their_count]);
        let text = String::from_utf8_lossy(&theirs);
        let line = text
            .split("[Embedded]\r\n")
            .nth(1)
            .unwrap()
            .lines()
            .next()
            .unwrap();
        assert_eq!(
            line.split(' ').nth(1),
            entry.lines().next().unwrap().split(' ').nth(1)
        );
    }
    let _ = frame;
}

/// The values of the first `[isd]` record in a written SAM file.
fn isd_of(bytes: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let start = text.find("[isd]\r\n").expect("an [isd] record") + "[isd]\r\n".len();
    let mut values = Vec::new();
    for line in text[start..].split("\r\n") {
        if !line.starts_with("\t\t") || line.starts_with("\t\t\t") {
            break;
        }
        values.push(line.trim().to_owned());
    }
    values
}
