use super::*;

fn size(tex: &str) -> (i32, i32, i32) {
    let stored = Stored::new(tex);
    let l = stored.layout().expect("lays out");
    (l.width, l.height, l.baseline)
}

fn near(ours: i32, theirs: i32, percent: f64) -> bool {
    (f64::from(ours) - f64::from(theirs)).abs() <= f64::from(theirs) * percent / 100.0
}

#[test]
fn ami_pro_normal_form_of_its_own_texts() {
    // The inputs and the texts Save As Equation wrote (VM, EQV1).
    for (input, normal) in [
        ("\\frac{a}{b}", "$\\frac ab$"),
        ("\\binom{n}{k}", "$\\binom nk$"),
        ("\\sqrt{x}", "$\\sqrt{x}$"),
        ("\\root{3} \\of{x}", "$\\root{3} \\of{x}$"),
        ("x_{i}^{2}", "x$_i^2$"),
        ("\\sum_{i=1}^{n} i", "$\\sum_{i=1}^n$ i"),
        ("\\int_{0}^{1} f(x) dx", "$\\int_0^1$ f(x) dx"),
        (
            "\\matrix a & b \\\\ c & d \\endmatrix",
            "$\\matrix a & b \\\\ c & d\\endmatrix $",
        ),
        ("\\left( x \\right)", "$\\left( x\\right) $"),
        (
            "\\overline{x} \\underbrace{y}",
            "$\\overline{x}$ $\\underbrace{y}$",
        ),
        ("\\text{abc} {\\BF x}", "$\\text{abc}$ \\BF x"),
        ("x", "x"),
        ("$x$", "$x$"),
        ("$\\frac{x}{x}$", "$\\frac xx$"),
        ("$\\frac{xxx}{x}$", "$\\frac{xxx}x$"),
        ("$\\sum$", "$\\sum $"),
        ("$\\int$", "$\\int $"),
        ("$\\frac{\\frac{x}{x}}{x}$", "$\\frac{\\frac xx}x$"),
        (
            "$\\matrix x & x \\\\ x & x \\endmatrix$",
            "$\\matrix x & x \\\\ x & x\\endmatrix $",
        ),
        ("$\\left[ x\\right]$", "$\\left[ x\\right] $"),
    ] {
        assert_eq!(import_tex(input).expect(input), normal, "{input}");
        // Writing the normal form again changes nothing.
        assert_eq!(
            Stored::new(normal).normalised().expect(normal),
            normal,
            "{normal}"
        );
    }
}

#[test]
fn natural_sizes_follow_ami_pros() {
    // (tex, width, height, baseline) from the notes' §12.2.
    for (tex, w, h, b) in [
        ("x", 162, 257, 192),
        ("$x$", 138, 257, 192),
        ("$xyz$", 371, 257, 192),
        ("$x^2$", 255, 268, 203),
        ("$x_2$", 255, 257, 192),
        ("$x_2^2$", 279, 286, 221),
        ("$\\frac ab$", 209, 287, 192),
        ("$\\frac xx$", 228, 257, 192),
        ("$\\frac{xxx}x$", 346, 257, 192),
        ("$\\frac{\\frac xx}x$", 225, 332, 267),
        ("$\\sqrt{x}$", 423, 258, 193),
        ("$\\sqrt{xxx}$", 610, 257, 192),
        ("$\\root{3} \\of{x}$", 504, 259, 192),
        ("$\\sum $", 172, 257, 192),
        ("$\\sum_a^b$", 351, 266, 201),
        ("$\\int $", 90, 257, 192),
        ("$\\int_a^b$", 269, 330, 240),
        ("$\\matrix x & x \\\\ x & x\\endmatrix $", 1098, 614, 377),
        ("$\\matrix a & b \\\\ c & d\\endmatrix $", 1126, 614, 377),
        ("$\\left[ x\\right] $", 288, 257, 192),
        ("$\\left( x\\right) $", 337, 257, 192),
    ] {
        let (ow, oh, ob) = size(tex);
        assert!(near(ow, w, 8.0), "{tex}: width {ow} against {w}");
        assert!(near(oh, h, 8.0), "{tex}: height {oh} against {h}");
        assert!(near(ob, b, 8.0), "{tex}: baseline {ob} against {b}");
    }
}

#[test]
fn constructs_parse_into_their_nodes() {
    let tree =
        parse::parse("$\\frac{a}{b}\\sqrt{x}\\sum\\limits_{i=1}^n\\alpha\\text{t}$").unwrap();
    let Node::Math(nodes) = &tree[0] else {
        panic!("math run")
    };
    assert!(matches!(nodes[0], Node::Frac { .. }));
    assert!(matches!(nodes[1], Node::Sqrt(_)));
    assert!(matches!(
        &nodes[2],
        Node::BigOp {
            limits: Some(true),
            sub: Some(_),
            sup: Some(_),
            ..
        }
    ));
    assert!(matches!(&nodes[3], Node::Sym(name) if name == "alpha"));
    assert!(matches!(&nodes[4], Node::Text(text) if text == "t"));
}

#[test]
fn subscript_first_and_infix_over() {
    assert_eq!(from_source("x^2_i").unwrap(), "$x_i^2$");
    assert_eq!(
        from_source("{a \\over b}").unwrap(),
        "$\\frac ab$".replace("$\\frac ab$", "${\\frac ab}$")
    );
}

#[test]
fn stacks_and_formats() {
    let text = "$\\sum_{\\Sb i=1 \\\\ j=2\\endSb}$";
    assert!(layout_of(text).width > 0);
    let text = "$\\matrix \\format \\l & \\r \\\\ a & b \\\\ c & d\\endmatrix $";
    assert_eq!(Stored::new(text).normalised().unwrap(), text);
}

fn layout_of(tex: &str) -> Layout {
    Stored::new(tex).layout().expect(tex)
}

#[test]
fn errors_have_codes_and_positions() {
    let error = parse::parse("$\\frac{a}$").unwrap_err();
    assert_eq!(error.code, 5106);
    assert!(error.position <= 10);
    let error = parse::parse("$\\bogus x$").unwrap_err();
    assert_eq!(error.code, 5106);
    assert_eq!(error.position, 1, "at the backslash");
    let error = parse::parse("$x").unwrap_err();
    assert_eq!(error.code, 5106);
    let rows = ["a"; 11].join(" \\\\ ");
    let error = parse::parse(&format!("$\\matrix {rows}\\endmatrix $")).unwrap_err();
    assert_eq!(error.code, 5102);
    let deep = format!("${}x{}$", "{".repeat(100), "}".repeat(100));
    assert_eq!(parse::parse(&deep).unwrap_err().code, 5100);
    let big = format!("$\\quad{}$", "\\qquad".repeat(90));
    assert_eq!(Stored::new(&big).layout().unwrap_err().code, 5105);
    assert_eq!(
        from_source(&"x".repeat(MAX_SOURCE + 1)).unwrap_err().code,
        5105
    );
}

#[test]
fn never_panics_on_odd_input() {
    let pieces = [
        "\\frac",
        "{",
        "}",
        "$",
        "^",
        "_",
        "\\sqrt",
        "\\root",
        "\\of",
        "\\matrix",
        "\\endmatrix",
        "&",
        "\\\\",
        "\\left",
        "\\right",
        "(",
        ")",
        ".",
        "\\sum",
        "\\int",
        "\\limits",
        "x",
        "1",
        " ",
        "\\text",
        "é",
        "\\BF",
        "\\overline",
        "\\Sb",
        "\\endSb",
        "\\format",
        "\\l",
        "\\over",
        "\\binom",
        "\\underset",
        "\\to",
        "\\alpha",
        "\\",
        "%",
        "'",
        "\u{1d49c}",
        "\\func",
    ];
    let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..4_000 {
        let length = (next() % 14) as usize;
        let mut text = String::new();
        for _ in 0..length {
            text.push_str(pieces[(next() % pieces.len() as u64) as usize]);
        }
        for tree in [
            parse::parse(&text),
            parse::parse_import(&text),
            parse::parse_math(&text),
        ]
        .into_iter()
        .flatten()
        {
            let first = layout(&tree, &Params::default());
            let second = layout(&tree, &Params::default());
            assert_eq!(first.is_ok(), second.is_ok(), "{text:?}");
            if let (Ok(a), Ok(b)) = (first, second) {
                assert_eq!(a, b, "deterministic: {text:?}");
            }
            let _ = write::write(&tree);
        }
        let _ = from_source(&text);
        let _ = import_tex(&text);
    }
}

#[test]
fn stored_form_is_two_lines() {
    let stored = Stored::new("$\\frac ab$");
    assert_eq!(
        stored.bytes(),
        b"TimesNewRomanPS,18,12,0,0,0,0,0\r\n$\\frac ab$".to_vec()
    );
    let read = Stored::read(&stored.bytes()).unwrap();
    assert_eq!(read, stored);
    assert_eq!(read.face(), ("TimesNewRomanPS".to_string(), 12.0));
    let mut bigger = read;
    bigger.set_face("TimesNewRomanPS", 18.0);
    assert_eq!(bigger.settings, "TimesNewRomanPS,18,18,0,0,0,0,0");
    assert!(Stored::read(b"no settings here").is_none());
    assert!(Stored::read(&[0xff, 0xfe, 0, 1]).is_none());
}

#[test]
fn size_scales_with_the_point_size() {
    let mut stored = Stored::new("$x$");
    stored.set_face("TimesNewRomanPS", 24.0);
    let big = stored.layout().unwrap();
    let small = Stored::new("$x$").layout().unwrap();
    assert!(
        big.width > small.width * 3 / 2,
        "{} {}",
        big.width,
        small.width
    );
    assert!(big.height > small.height * 3 / 2);
}

#[test]
fn source_and_stored_forms() {
    assert_eq!(to_source("$\\frac ab$"), "\\frac ab");
    assert_eq!(to_source("x$_i^2$"), "x$_i^2$");
    assert_eq!(from_source("\\frac{a}{b}").unwrap(), "$\\frac ab$");
    assert_eq!(from_source("").unwrap(), "");
    assert!(layout_of("").width == 220);
    assert_eq!(from_source("a $+$ b").unwrap(), "a $+$ b");
}

#[test]
fn latex_pastes_and_copies() {
    assert_eq!(
        latex_to_source("$\\sqrt[3]{x}$").unwrap(),
        "\\root{3} \\of{x}"
    );
    assert_eq!(latex_to_source("\\[ x^2 \\]").unwrap(), "x^2");
    assert_eq!(
        latex_to_source("\\begin{equation} a=b \\end{equation}").unwrap(),
        "a=b"
    );
    assert!(latex_to_source("just words").is_none());
    assert_eq!(source_to_latex("\\root{3} \\of{x}"), "\\sqrt[3]{x}");
    let m = latex_to_source("$\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}$").unwrap();
    assert!(from_source(&m).is_ok(), "{m}");
}

#[test]
fn save_as_text_breaks_long_lines() {
    let long = format!("${}$", "x + ".repeat(40));
    let saved = save_as_text(&long);
    assert!(saved.contains("\r\n"));
    for line in saved.split("\r\n") {
        assert!(line.chars().count() <= 80, "{line}");
    }
    assert_eq!(save_as_text("$x$"), "$x$");
}

#[test]
fn drawing_has_text_rules_and_paths() {
    let l = layout_of("$\\frac{a}{\\sqrt{b}} + \\left( x\\right) $");
    let drawing = l.to_drawing("TimesNewRomanPS", layout::Colours::BLACK);
    assert_eq!(drawing.window, (l.width, l.height));
    assert!(
        drawing
            .ops
            .iter()
            .any(|op| matches!(op, crate::wmf::Op::Text { .. }))
    );
    assert!(
        drawing
            .ops
            .iter()
            .any(|op| matches!(op, crate::wmf::Op::Path { .. }))
    );
}

#[test]
fn big_operators_function_and_decorations() {
    for tex in [
        "$\\lim_{x\\to 0} \\sin x$",
        "$\\dsum_{i=1}^n i$",
        "$\\prod\\limits_a^b$",
        "$\\hat{x} \\vec{v} \\widehat{xy} \\overrightarrow{AB}$",
        "$\\overset{a}\\to{b} \\underset{c}\\to{d}$",
        "${\\BBB R} {\\CAL L} {\\FRAK g} {\\TT t}$",
        "$\\alpha+\\beta\\leq\\gamma, \\infty$",
        "$\\func{foo}(x)$",
    ] {
        let l = layout_of(tex);
        assert!(l.width > 40 && l.height >= 257, "{tex}");
        assert!(!Stored::new(tex).normalised().unwrap().is_empty(), "{tex}");
    }
}
