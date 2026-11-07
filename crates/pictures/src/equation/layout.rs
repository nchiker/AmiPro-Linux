//! The layout engine, by TeX's rules (own implementation): styles
//! (display, text, script, scriptscript), fractions with a rule between
//! shifted numerator and denominator, radicals, scripts, big operators
//! with limits, matrices, delimiters sized to their contents, decorations.
//! The constants are in [`Params`], fitted to the natural sizes of 27
//! equations Ami Pro laid out (`OBJECTS_NOTES.md` §12.2, §15).
//!
//! Everything is in twips. The result is deterministic and the code never
//! panics: an input too big or too deep is an error.

use super::EquationError;
use super::metrics::{self, Face};
use super::parse::{FontKind, GridKind, Node};
use super::symbols::{self, Class};
use crate::wmf;

/// Which colour class an item has (Ami Pro: math red on screen, text
/// black, functions gray; `[EQColor]`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ink {
    Math,
    Text,
    Function,
}

/// A path segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Seg {
    Move(f64, f64),
    Line(f64, f64),
    Cubic(f64, f64, f64, f64, f64, f64),
    Close,
}

/// A positioned piece of the drawn equation (x from the left, y from the
/// top, in twips).
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// Text whose baseline is at `y` and whose em is `size` twips.
    Glyphs {
        x: f64,
        y: f64,
        text: String,
        size: f64,
        face: Face,
        ink: Ink,
        width: f64,
    },
    Rule {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        ink: Ink,
    },
    Path {
        segs: Vec<Seg>,
        fill: bool,
        width: f64,
        ink: Ink,
    },
}

/// A laid-out equation: its natural size, the baseline (the ascent from
/// the top) and its items.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub width: i32,
    pub height: i32,
    pub baseline: i32,
    pub items: Vec<Item>,
}

/// The constants of the layout, in shares of the em unless noted.
#[derive(Clone, Debug)]
pub struct Params {
    /// The base em in twips (the point size times 20).
    pub em: f64,
    /// Total horizontal padding around the equation, twips at 12 pt.
    pub pad: f64,
    pub strut_asc: f64,
    pub strut_desc: f64,
    pub roman_scale: f64,
    pub italic_scale: f64,
    /// The em of display/text/script/scriptscript styles.
    pub style_scale: [f64; 4],
    pub axis: f64,
    pub rule: f64,
    pub frac_num_gap: f64,
    pub frac_den_gap: f64,
    pub frac_side: f64,
    /// The fraction's side space inside a script style.
    pub frac_side_script: f64,
    pub text_space: f64,
    pub paren_width: f64,
    pub binom_width: f64,
    pub sup_shift: f64,
    pub sup_shift_both: f64,
    pub sub_shift: f64,
    pub script_pad: f64,
    pub script_pad_both: f64,
    pub op_script_pad: f64,
    pub radical_extra: f64,
    pub radical_gap: f64,
    pub delim_width: f64,
    pub cell_pad: f64,
    pub col_gap: f64,
    pub row_gap: f64,
    pub matrix_side: f64,
    pub matrix_vert: f64,
    pub empty: (f64, f64),
    pub decor_side: f64,
    pub brace_height: f64,
    pub decor_gap: f64,
    /// Thin, medium and thick spaces (shares of the em, TeX's 3, 4, 5 of 18).
    pub spaces: (f64, f64, f64),
}

impl Params {
    /// The constants at a point size.
    pub fn at_points(points: f64) -> Self {
        Self {
            em: points.clamp(1.0, 400.0) * 20.0,
            ..Self::default()
        }
    }
}

impl Default for Params {
    fn default() -> Self {
        Self {
            em: 240.0,
            pad: 44.0,
            strut_asc: 0.8,
            strut_desc: 0.2708,
            roman_scale: 0.985,
            italic_scale: 0.9,
            style_scale: [1.0, 1.0, 0.7, 0.5],
            axis: 0.29,
            rule: 0.04,
            frac_num_gap: 0.13,
            frac_den_gap: 0.185,
            frac_side: 0.215,
            frac_side_script: 0.1,
            text_space: 0.49,
            paren_width: 0.42,
            binom_width: 0.11,
            sup_shift: 0.41,
            sup_shift_both: 0.49,
            sub_shift: 0.2,
            script_pad: 0.1375,
            script_pad_both: 0.2375,
            op_script_pad: 0.43,
            radical_extra: 1.12,
            radical_gap: 0.083,
            delim_width: 0.31,
            cell_pad: 0.93,
            col_gap: 0.75,
            row_gap: 0.083,
            matrix_side: 0.5,
            matrix_vert: 0.167,
            empty: (220.0, 300.0),
            decor_side: 0.08,
            brace_height: 0.5,
            decor_gap: 0.15,
            spaces: (0.1667, 0.2222, 0.2778),
        }
    }
}

/// Lays out a parsed equation.
pub fn layout(tree: &[Node], params: &Params) -> Result<Layout, EquationError> {
    let scale = params.em / 240.0;
    let mut engine = Engine { p: params };
    let ctx = Ctx {
        style: 1,
        font: None,
        ink: Ink::Text,
        math: false,
    };
    let body = engine.top(tree, ctx)?;
    if body.items.is_empty() && body.w == 0.0 {
        let (w, h) = params.empty;
        return Ok(Layout {
            width: (w * scale).round() as i32,
            height: (h * scale).round() as i32,
            baseline: (h * scale * 0.75).round() as i32,
            items: Vec::new(),
        });
    }
    let strut_asc = params.strut_asc * params.em;
    let strut_desc = params.strut_desc * params.em;
    let asc = body.asc.max(strut_asc);
    let desc = body.desc.max(strut_desc);
    let pad = params.pad * scale;
    let width = body.w + pad;
    let height = asc + desc;
    if width > 31_680.0 || height > 31_680.0 || !width.is_finite() || !height.is_finite() {
        return Err(EquationError {
            code: 5105,
            message: "The equation is too big".into(),
            position: 0,
        });
    }
    let mut items = body.items;
    shift_items(&mut items, pad / 2.0, asc);
    Ok(Layout {
        width: width.round() as i32,
        height: height.round() as i32,
        baseline: asc.round() as i32,
        items,
    })
}

#[derive(Clone, Copy)]
struct Ctx {
    /// 0 display, 1 text, 2 script, 3 scriptscript.
    style: u8,
    font: Option<FontKind>,
    ink: Ink,
    math: bool,
}

/// A box: a width, the extent above and below the baseline, and items
/// placed relative to its left edge and baseline (y down).
#[derive(Clone, Debug, Default)]
struct Bx {
    w: f64,
    asc: f64,
    desc: f64,
    items: Vec<Item>,
}

fn shift_items(items: &mut [Item], dx: f64, dy: f64) {
    for item in items {
        match item {
            Item::Glyphs { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
            Item::Rule { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
            Item::Path { segs, .. } => {
                for seg in segs {
                    match seg {
                        Seg::Move(x, y) | Seg::Line(x, y) => {
                            *x += dx;
                            *y += dy;
                        }
                        Seg::Cubic(a, b, c, d, e, f) => {
                            *a += dx;
                            *b += dy;
                            *c += dx;
                            *d += dy;
                            *e += dx;
                            *f += dy;
                        }
                        Seg::Close => {}
                    }
                }
            }
        }
    }
}

impl Bx {
    fn place(&mut self, other: Bx, dx: f64, dy: f64) {
        let mut items = other.items;
        shift_items(&mut items, dx, dy);
        for item in items {
            self.push(item);
        }
        self.asc = self.asc.max(other.asc - dy);
        self.desc = self.desc.max(other.desc + dy);
        self.w = self.w.max(dx + other.w);
    }

    /// Adds an item, joining text that continues the last item's.
    fn push(&mut self, item: Item) {
        if let (
            Some(Item::Glyphs {
                x: px,
                y: py,
                text: ptext,
                size: psize,
                face: pface,
                ink: pink,
                width: pwidth,
            }),
            Item::Glyphs {
                x,
                y,
                text,
                size,
                face,
                ink,
                width,
            },
        ) = (self.items.last_mut(), &item)
            && (*py - *y).abs() < 1e-6
            && *psize == *size
            && *pface == *face
            && *pink == *ink
            && (*px + *pwidth - *x).abs() < 1e-6
        {
            ptext.push_str(text);
            *pwidth += *width;
            return;
        }
        self.items.push(item);
    }

    fn space(w: f64) -> Bx {
        Bx { w, ..Bx::default() }
    }
}

struct Engine<'a> {
    p: &'a Params,
}

/// How a node takes part in the spacing of a math list.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Ord,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Op,
}

impl Engine<'_> {
    fn em(&self, ctx: Ctx) -> f64 {
        self.p.em * self.p.style_scale[usize::from(ctx.style.min(3))]
    }

    fn script_style(ctx: Ctx) -> Ctx {
        Ctx {
            style: if ctx.style < 2 { 2 } else { 3 },
            ..ctx
        }
    }

    fn top(&mut self, tree: &[Node], ctx: Ctx) -> Result<Bx, EquationError> {
        let mut out = Bx::default();
        let mut ctx = ctx;
        for node in tree {
            match node {
                Node::Text(text) => {
                    let b = self.text(text, ctx, Ink::Text);
                    let at = out.w;
                    out.place(b, at, 0.0);
                }
                Node::Font(kind) => ctx.font = Some(*kind),
                Node::Space(name) => out.w += self.space_of(name, ctx),
                Node::Math(nodes) => {
                    let mctx = Ctx {
                        math: true,
                        ink: Ink::Math,
                        ..ctx
                    };
                    let b = self.list(nodes, mctx)?;
                    let at = out.w;
                    out.place(b, at, 0.0);
                }
                other => {
                    let b = self.list(std::slice::from_ref(other), ctx)?;
                    let at = out.w;
                    out.place(b, at, 0.0);
                }
            }
        }
        Ok(out)
    }

    fn space_of(&self, name: &str, ctx: Ctx) -> f64 {
        f64::from(symbols::space_width(name).unwrap_or(0)) / 1000.0 * self.em(ctx)
    }

    fn face_for(&self, c: char, ctx: Ctx) -> (Face, char) {
        let letter = c.is_alphabetic();
        let italic_default = ctx.math && letter && !matches!(c, 'ℏ' | '∞');
        match ctx.font {
            Some(FontKind::Bold) => (
                Face {
                    bold: true,
                    italic: false,
                    mono: false,
                },
                c,
            ),
            Some(FontKind::Italic | FontKind::Slanted) => (Face::ITALIC, c),
            Some(FontKind::Roman) => (Face::ROMAN, c),
            Some(FontKind::Typewriter) => (
                Face {
                    bold: false,
                    italic: false,
                    mono: true,
                },
                c,
            ),
            Some(FontKind::Calligraphic) => (Face::ROMAN, alphabet(c, 0x1d49c)),
            Some(FontKind::Fraktur) => (Face::ROMAN, alphabet(c, 0x1d504)),
            Some(FontKind::Blackboard) => (Face::ROMAN, alphabet(c, 0x1d538)),
            _ => (
                Face {
                    bold: false,
                    italic: italic_default,
                    mono: false,
                },
                c,
            ),
        }
    }

    fn scale_of(&self, face: Face) -> f64 {
        if face.italic {
            self.p.italic_scale
        } else {
            self.p.roman_scale
        }
    }

    /// A run of characters as one box.
    fn glyph(&self, c: char, ctx: Ctx, ink: Ink) -> Bx {
        let (face, shown) = self.face_for(c, ctx);
        let em = self.em(ctx);
        let width = if c == ' ' && !ctx.math {
            self.p.text_space * em
        } else {
            metrics::advance(shown, face) * self.scale_of(face) * em
        };
        let (a, d) = metrics::ink(c, face);
        let mut bx = Bx {
            w: width,
            asc: a * em,
            desc: d * em,
            items: Vec::new(),
        };
        if c != ' ' {
            bx.items.push(Item::Glyphs {
                x: 0.0,
                y: 0.0,
                text: shown.to_string(),
                size: em,
                face,
                ink,
                width,
            });
        } else {
            bx.asc = 0.0;
            bx.desc = 0.0;
        }
        bx
    }

    fn text(&self, text: &str, ctx: Ctx, ink: Ink) -> Bx {
        let mut out = Bx::default();
        let tctx = Ctx { math: false, ..ctx };
        for c in text.chars() {
            let b = self.glyph(c, tctx, ink);
            let at = out.w;
            out.place(b, at, 0.0);
        }
        out
    }

    fn kind_of(&self, node: &Node) -> Kind {
        match node {
            Node::Ch(c) => match c {
                '+' | '-' | '*' | '−' => Kind::Bin,
                '=' | '<' | '>' | ':' => Kind::Rel,
                '(' | '[' => Kind::Open,
                ')' | ']' => Kind::Close,
                ',' | ';' => Kind::Punct,
                _ => Kind::Ord,
            },
            Node::Sym(name) => match symbols::symbol(name).map(|s| s.1) {
                Some(Class::Bin) => Kind::Bin,
                Some(Class::Rel) => Kind::Rel,
                Some(Class::Open) => Kind::Open,
                Some(Class::Close) => Kind::Close,
                Some(Class::Punct) => Kind::Punct,
                _ => Kind::Ord,
            },
            Node::BigOp { .. } | Node::Func { .. } => Kind::Op,
            Node::Scripts { base, .. } => self.kind_of(base),
            _ => Kind::Ord,
        }
    }

    /// A list of math nodes, with TeX's spacing between atoms.
    fn list(&mut self, nodes: &[Node], ctx: Ctx) -> Result<Bx, EquationError> {
        let mut out = Bx::default();
        let mut ctx = ctx;
        let mut prev: Option<Kind> = None;
        for node in nodes {
            if let Node::Font(kind) = node {
                ctx.font = Some(*kind);
                continue;
            }
            let mut kind = self.kind_of(node);
            if kind == Kind::Bin
                && matches!(
                    prev,
                    None | Some(Kind::Bin | Kind::Rel | Kind::Open | Kind::Punct | Kind::Op)
                )
            {
                kind = Kind::Ord;
            }
            let b = self.node(node, ctx)?;
            if ctx.math {
                let em = self.em(ctx);
                let (thin, med, thick) = self.p.spaces;
                let gap = match (prev, kind) {
                    (Some(Kind::Ord | Kind::Close), Kind::Bin)
                    | (Some(Kind::Bin), Kind::Ord | Kind::Open | Kind::Op) => med,
                    (Some(Kind::Ord | Kind::Close | Kind::Op), Kind::Rel)
                    | (Some(Kind::Rel), Kind::Ord | Kind::Open | Kind::Op) => thick,
                    (Some(Kind::Punct), Kind::Ord | Kind::Open | Kind::Op | Kind::Rel) => thin,
                    (Some(Kind::Op), Kind::Ord | Kind::Open) | (Some(Kind::Ord), Kind::Op) => thin,
                    _ => 0.0,
                };
                out.w += gap * em;
            }
            let at = out.w;
            out.place(b, at, 0.0);
            out.w = at + out.items_width_hint(at);
            prev = Some(kind);
        }
        Ok(out)
    }

    fn arg(&mut self, nodes: &[Node], ctx: Ctx) -> Result<Bx, EquationError> {
        self.list(nodes, ctx)
    }

    fn node(&mut self, node: &Node, ctx: Ctx) -> Result<Bx, EquationError> {
        let ink = ctx.ink;
        Ok(match node {
            Node::Ch(c) => self.glyph(*c, ctx, ink),
            Node::Sym(name) => match symbols::symbol(name) {
                Some((c, _)) => self.glyph(c, ctx, ink),
                None => Bx::default(),
            },
            Node::Text(text) => self.text(text, ctx, Ink::Text),
            Node::Font(_) => Bx::default(),
            Node::Space(name) => Bx::space(self.space_of(name, ctx)),
            Node::Group(nodes) => self.list(nodes, ctx)?,
            Node::Math(nodes) => self.list(nodes, Ctx { math: true, ..ctx })?,
            Node::Frac { word, num, den } => self.frac(word, num, den, ctx)?,
            Node::Sqrt(body) => self.radical(None, body, ctx)?,
            Node::Root { index, body } => self.radical(Some(index), body, ctx)?,
            Node::Scripts { base, sub, sup } => {
                self.scripts(base, sub.as_deref(), sup.as_deref(), ctx)?
            }
            Node::BigOp {
                word,
                limits,
                sub,
                sup,
            } => self.big_op(word, *limits, sub.as_deref(), sup.as_deref(), ctx)?,
            Node::Func { name, .. } => {
                let mut b = self.text(
                    name,
                    Ctx {
                        font: Some(FontKind::Roman),
                        ..ctx
                    },
                    Ink::Function,
                );
                b.w += 0.0;
                b
            }
            Node::Grid { kind, rows, .. } => self.grid(*kind, rows, ctx)?,
            Node::Delimited { left, right, body } => self.delimited(left, right, body, ctx)?,
            Node::Decor { word, arg } => self.decor(word, arg, ctx)?,
            Node::Stack { under, note, base } => self.stack(*under, note, base, ctx)?,
        })
    }

    fn frac(
        &mut self,
        word: &str,
        num: &[Node],
        den: &[Node],
        ctx: Ctx,
    ) -> Result<Bx, EquationError> {
        let binom = word.ends_with("binom") || word == "binom";
        let mut inner = ctx;
        if word.starts_with('d') {
            inner.style = 0;
        } else if word.starts_with('t') {
            inner.style = 1;
        }
        let sctx = Self::script_style(Ctx {
            style: if inner.style == 0 { 0 } else { inner.style },
            ..inner
        });
        // In display style the parts are in text style.
        let sctx = if inner.style == 0 {
            Ctx { style: 1, ..sctx }
        } else {
            sctx
        };
        let n = self.arg(num, sctx)?;
        let d = self.arg(den, sctx)?;
        let em = self.em(ctx);
        let axis = self.p.axis * em;
        let rule = self.p.rule * em;
        let side = if ctx.style >= 2 {
            self.p.frac_side_script
        } else {
            self.p.frac_side
        } * em;
        let width = n.w.max(d.w) + 2.0 * side;
        let mut out = Bx::default();
        let num_up = axis + rule / 2.0 + self.p.frac_num_gap * em - rule / 2.0 + n.desc;
        let den_down = d.asc + self.p.frac_den_gap * em - axis + rule / 2.0;
        let num_up = num_up.max(axis + rule / 2.0 + n.desc);
        let den_down = den_down.max(0.0).max(d.asc - axis + rule);
        out.place(n.clone(), (width - n.w) / 2.0, -num_up);
        out.place(d.clone(), (width - d.w) / 2.0, den_down);
        if !binom {
            out.items.push(Item::Rule {
                x: 0.0,
                y: -axis - rule / 2.0,
                w: width,
                h: rule,
                ink: ctx.ink,
            });
        }
        out.w = width;
        if binom {
            // Brackets around the stack.
            let inner_box = out;
            let h_up = inner_box.asc;
            let h_down = inner_box.desc;
            let dw = self.p.binom_width * em;
            let mut result = Bx::default();
            let mut left = self.delim('(', h_up, h_down, em, ctx.ink);
            let mut right = self.delim(')', h_up, h_down, em, ctx.ink);
            left.w = dw;
            right.w = dw;
            result.place(left, 0.0, 0.0);
            result.place(inner_box, dw, 0.0);
            let at = result.w;
            result.place(right, at, 0.0);
            result.w = at + dw;
            return Ok(result);
        }
        Ok(out)
    }

    fn radical(
        &mut self,
        index: Option<&[Node]>,
        body: &[Node],
        ctx: Ctx,
    ) -> Result<Bx, EquationError> {
        let em = self.em(ctx);
        let b = self.arg(body, ctx)?;
        let extra = self.p.radical_extra * em;
        let rule = self.p.rule * em;
        let gap = self.p.radical_gap * em;
        let top = b.asc.max(self.p.strut_asc * em * 0.0) + gap;
        let mut out = Bx::default();
        let idx = match index {
            Some(index) => Some(self.arg(index, Ctx { style: 2, ..ctx })?),
            None => None,
        };
        let index_w = idx.as_ref().map_or(0.0, |i| i.w);
        let left = index_w;
        let sign_w = extra;
        out.place(b.clone(), left + sign_w, 0.0);
        // The sign: a tick, a long stroke down, a stroke up, the bar.
        let x0 = left + sign_w * 0.1;
        let bottom = b.desc;
        let segs = vec![
            Seg::Move(x0, -top * 0.4),
            Seg::Line(left + sign_w * 0.3, -top * 0.45),
            Seg::Line(left + sign_w * 0.55, bottom.max(0.02 * em)),
            Seg::Line(left + sign_w * 0.95, -top - rule / 2.0),
            Seg::Line(left + sign_w + b.w, -top - rule / 2.0),
        ];
        out.items.push(Item::Path {
            segs,
            fill: false,
            width: rule,
            ink: ctx.ink,
        });
        if let Some(idx) = idx {
            let up = top * 0.55 + idx.desc;
            out.place(idx, 0.0, -up);
        }
        out.asc = out.asc.max(top + rule);
        out.desc = out.desc.max(b.desc);
        out.w = left + sign_w + b.w + self.p.rule * em;
        Ok(out)
    }

    fn scripts(
        &mut self,
        base: &Node,
        sub: Option<&[Node]>,
        sup: Option<&[Node]>,
        ctx: Ctx,
    ) -> Result<Bx, EquationError> {
        let em = self.em(ctx);
        // A base that is a big operator or limits function is handled there.
        if let Node::BigOp { .. } = base {
            return self.node(base, ctx);
        }
        let below = matches!(base, Node::Func { name, .. } if symbols::function_has_limits(name))
            && ctx.style == 0;
        let b = self.node(base, ctx)?;
        let sctx = Self::script_style(ctx);
        let s_sub = match sub {
            Some(s) => Some(self.arg(s, sctx)?),
            None => None,
        };
        let s_sup = match sup {
            Some(s) => Some(self.arg(s, sctx)?),
            None => None,
        };
        let mut out = Bx::default();
        if below {
            return self.limits_stack(b, s_sub, s_sup, em);
        }
        let both = s_sub.is_some() && s_sup.is_some();
        let base_w = b.w;
        out.place(b, 0.0, 0.0);
        let w_sub = s_sub.as_ref().map_or(0.0, |s| s.w);
        let w_sup = s_sup.as_ref().map_or(0.0, |s| s.w);
        let pad = if both {
            self.p.script_pad_both
        } else {
            self.p.script_pad
        } * em;
        let wide = w_sub.max(w_sup);
        if let Some(s) = s_sup {
            let raise = if both {
                self.p.sup_shift_both
            } else {
                self.p.sup_shift
            } * em;
            out.place(s, base_w, -raise);
        }
        if let Some(s) = s_sub {
            let drop = self.p.sub_shift * em;
            out.place(s, base_w, drop);
        }
        out.w = base_w + wide + pad;
        Ok(out)
    }

    /// Limits above and below a base, centred.
    fn limits_stack(
        &mut self,
        base: Bx,
        sub: Option<Bx>,
        sup: Option<Bx>,
        em: f64,
    ) -> Result<Bx, EquationError> {
        let width = base
            .w
            .max(sub.as_ref().map_or(0.0, |s| s.w))
            .max(sup.as_ref().map_or(0.0, |s| s.w));
        let gap = 0.04 * em;
        let mut out = Bx::default();
        let (basea, based) = (base.asc, base.desc);
        out.place(base.clone(), (width - base.w) / 2.0, 0.0);
        if let Some(s) = sup {
            let up = basea + gap + s.desc;
            out.place(s.clone(), (width - s.w) / 2.0, -up);
        }
        if let Some(s) = sub {
            let down = based + gap + s.asc;
            out.place(s.clone(), (width - s.w) / 2.0, down);
        }
        out.w = width;
        Ok(out)
    }

    fn big_op(
        &mut self,
        word: &str,
        limits: Option<bool>,
        sub: Option<&[Node]>,
        sup: Option<&[Node]>,
        ctx: Ctx,
    ) -> Result<Bx, EquationError> {
        let (ch, above) = symbols::big_operator(word).unwrap_or(('∑', true));
        let display =
            word.starts_with('d') && symbols::big_operator(&word[1..]).is_some() || ctx.style == 0;
        let em = self.em(ctx);
        let size = if display { 1.15 * em } else { em };
        let integral = !above;
        let width = metrics::advance(ch, Face::ROMAN)
            * size
            * match ch {
                '∑' => 0.84,
                '∫' => 0.7,
                _ => 1.0,
            };
        let (ink_a, ink_d) = metrics::ink(ch, Face::ROMAN);
        let mut op = Bx {
            w: width,
            asc: ink_a * size,
            desc: ink_d * size,
            items: vec![Item::Glyphs {
                x: 0.0,
                y: 0.0,
                text: ch.to_string(),
                size,
                face: Face::ROMAN,
                ink: ctx.ink,
                width,
            }],
        };
        let sctx = Self::script_style(ctx);
        let s_sub = match sub {
            Some(s) => Some(self.arg(s, sctx)?),
            None => None,
        };
        let s_sup = match sup {
            Some(s) => Some(self.arg(s, sctx)?),
            None => None,
        };
        if s_sub.is_none() && s_sup.is_none() {
            return Ok(op);
        }
        let script_w = s_sub
            .as_ref()
            .map_or(0.0, |s| s.w)
            .max(s_sup.as_ref().map_or(0.0, |s| s.w));
        // Limits go above and below when asked, in display style, or (as
        // Ami Pro does) when they are wider than the operator.
        let stacked = limits.unwrap_or((above && display) || (above && script_w > op.w));
        if stacked {
            return self.limits_stack(op, s_sub, s_sup, em);
        }
        let (raise, drop) = if integral { (0.52, 0.375) } else { (0.36, 0.2) };
        let base_w = op.w;
        let mut out = Bx::default();
        let w_sub = s_sub.as_ref().map_or(0.0, |s| s.w);
        let w_sup = s_sup.as_ref().map_or(0.0, |s| s.w);
        op.asc = op.asc.max(0.0);
        out.place(op, 0.0, 0.0);
        if let Some(s) = s_sup {
            out.place(s, base_w, -raise * em);
        }
        if let Some(s) = s_sub {
            out.place(s, base_w, drop * em);
        }
        out.w = base_w + w_sub.max(w_sup) + self.p.op_script_pad * em;
        Ok(out)
    }

    fn grid(
        &mut self,
        kind: GridKind,
        rows: &[Vec<Vec<Node>>],
        ctx: Ctx,
    ) -> Result<Bx, EquationError> {
        let em = self.em(ctx);
        let cctx = match kind {
            GridKind::Matrix => ctx,
            _ => Self::script_style(ctx),
        };
        let mut cells: Vec<Vec<Bx>> = Vec::new();
        let mut columns = 0;
        for row in rows {
            let mut line = Vec::new();
            for cell in row {
                line.push(self.arg(cell, cctx)?);
            }
            columns = columns.max(line.len());
            cells.push(line);
        }
        let mut widths = vec![0.0f64; columns];
        for line in &cells {
            for (c, cell) in line.iter().enumerate() {
                widths[c] = widths[c].max(cell.w);
            }
        }
        let stacked = kind != GridKind::Matrix;
        let pad = if stacked { 0.0 } else { self.p.cell_pad * em };
        let col_gap = if stacked { 0.0 } else { self.p.col_gap * em };
        let side = if stacked {
            0.0
        } else {
            self.p.matrix_side * em
        };
        let vert = if stacked {
            0.0
        } else {
            self.p.matrix_vert * em
        };
        let row_gap = self.p.row_gap * em;
        let strut_a = self.p.strut_asc * em * if stacked { 0.6 } else { 1.0 };
        let strut_d = self.p.strut_desc * em * if stacked { 0.6 } else { 1.0 };
        let mut out = Bx::default();
        let mut y = vert;
        for line in &cells {
            let asc = line.iter().map(|c| c.asc).fold(strut_a, f64::max);
            let desc = line.iter().map(|c| c.desc).fold(strut_d, f64::max);
            let mut x = side;
            for (c, cell) in line.iter().enumerate() {
                let col_w = widths[c] + pad;
                out.place(cell.clone(), x + (col_w - cell.w) / 2.0, y + asc);
                x += col_w + col_gap;
            }
            y += asc + desc + row_gap;
        }
        let height = if cells.is_empty() {
            0.0
        } else {
            y - row_gap + vert
        };
        let total_w = side * 2.0
            + widths.iter().map(|w| w + pad).sum::<f64>()
            + col_gap * (columns.saturating_sub(1)) as f64;
        // Make the box's reference line the axis: the items are relative to
        // the top, so move them up by the baseline.
        let baseline = height / 2.0 + self.p.axis * em;
        let mut items = std::mem::take(&mut out.items);
        shift_items(&mut items, 0.0, -baseline);
        Ok(Bx {
            w: total_w,
            asc: baseline,
            desc: height - baseline,
            items,
        })
    }

    fn delimited(
        &mut self,
        left: &str,
        right: &str,
        body: &[Node],
        ctx: Ctx,
    ) -> Result<Bx, EquationError> {
        let em = self.em(ctx);
        let b = self.list(body, ctx)?;
        let asc = b.asc.max(self.p.strut_asc * em);
        let desc = b.desc.max(self.p.strut_desc * em);
        let mut out = Bx::default();
        let l = delim_char(left);
        let r = delim_char(right);
        let lw = l.map_or(0.0, |c| self.delim_width(c) * em);
        let rw = r.map_or(0.0, |c| self.delim_width(c) * em);
        if let Some(c) = l {
            let d = self.delim(c, asc, desc, em, ctx.ink);
            out.place(d, 0.0, 0.0);
        }
        out.place(b.clone(), lw, 0.0);
        if let Some(c) = r {
            let d = self.delim(c, asc, desc, em, ctx.ink);
            out.place(d, lw + b.w, 0.0);
        }
        out.w = lw + b.w + rw;
        out.asc = asc;
        out.desc = desc;
        Ok(out)
    }

    /// A delimiter drawn to span `asc` above and `desc` below the baseline.
    fn delim_width(&self, c: char) -> f64 {
        match c {
            '(' | ')' => self.p.paren_width,
            '|' => 0.2,
            _ => self.p.delim_width,
        }
    }

    fn delim(&self, c: char, asc: f64, desc: f64, em: f64, ink: Ink) -> Bx {
        let w = self.delim_width(c) * em;
        let (top, bottom) = (-asc, desc);
        let h = bottom - top;
        let t = 0.045 * em;
        let (x0, x1, xm) = (w * 0.2, w * 0.8, w * 0.5);
        let mut segs: Vec<Seg> = Vec::new();
        let width = t;
        match c {
            '(' => {
                segs.push(Seg::Move(x1, top));
                segs.push(Seg::Cubic(
                    x0,
                    top + h * 0.3,
                    x0,
                    bottom - h * 0.3,
                    x1,
                    bottom,
                ));
            }
            ')' => {
                segs.push(Seg::Move(x0, top));
                segs.push(Seg::Cubic(
                    x1,
                    top + h * 0.3,
                    x1,
                    bottom - h * 0.3,
                    x0,
                    bottom,
                ));
            }
            '[' => {
                segs.push(Seg::Move(x1, top));
                segs.push(Seg::Line(x0, top));
                segs.push(Seg::Line(x0, bottom));
                segs.push(Seg::Line(x1, bottom));
            }
            ']' => {
                segs.push(Seg::Move(x0, top));
                segs.push(Seg::Line(x1, top));
                segs.push(Seg::Line(x1, bottom));
                segs.push(Seg::Line(x0, bottom));
            }
            '⌊' => {
                segs.push(Seg::Move(x0, top));
                segs.push(Seg::Line(x0, bottom));
                segs.push(Seg::Line(x1, bottom));
            }
            '⌋' => {
                segs.push(Seg::Move(x1, top));
                segs.push(Seg::Line(x1, bottom));
                segs.push(Seg::Line(x0, bottom));
            }
            '⌈' => {
                segs.push(Seg::Move(x0, bottom));
                segs.push(Seg::Line(x0, top));
                segs.push(Seg::Line(x1, top));
            }
            '⌉' => {
                segs.push(Seg::Move(x1, bottom));
                segs.push(Seg::Line(x1, top));
                segs.push(Seg::Line(x0, top));
            }
            '|' => {
                segs.push(Seg::Move(xm, top));
                segs.push(Seg::Line(xm, bottom));
            }
            '‖' => {
                segs.push(Seg::Move(w * 0.35, top));
                segs.push(Seg::Line(w * 0.35, bottom));
                segs.push(Seg::Move(w * 0.65, top));
                segs.push(Seg::Line(w * 0.65, bottom));
            }
            '⟨' => {
                segs.push(Seg::Move(x1, top));
                segs.push(Seg::Line(x0, top + h / 2.0));
                segs.push(Seg::Line(x1, bottom));
            }
            '⟩' => {
                segs.push(Seg::Move(x0, top));
                segs.push(Seg::Line(x1, top + h / 2.0));
                segs.push(Seg::Line(x0, bottom));
            }
            '/' => {
                segs.push(Seg::Move(x1, top));
                segs.push(Seg::Line(x0, bottom));
            }
            '\\' => {
                segs.push(Seg::Move(x0, top));
                segs.push(Seg::Line(x1, bottom));
            }
            '{' | '}' => {
                let (outer, inner) = if c == '{' { (x1, x0) } else { (x0, x1) };
                let mid = top + h / 2.0;
                segs.push(Seg::Move(outer, top));
                segs.push(Seg::Cubic(xm, top, xm, top + h * 0.1, xm, top + h * 0.25));
                segs.push(Seg::Line(xm, mid - h * 0.12));
                segs.push(Seg::Cubic(
                    xm,
                    mid - h * 0.05,
                    inner,
                    mid - h * 0.02,
                    inner,
                    mid,
                ));
                segs.push(Seg::Cubic(
                    inner,
                    mid + h * 0.02,
                    xm,
                    mid + h * 0.05,
                    xm,
                    mid + h * 0.12,
                ));
                segs.push(Seg::Line(xm, bottom - h * 0.25));
                segs.push(Seg::Cubic(xm, bottom - h * 0.1, xm, bottom, outer, bottom));
            }
            other => {
                let mut b = Bx {
                    w,
                    asc,
                    desc,
                    items: Vec::new(),
                };
                b.items.push(Item::Glyphs {
                    x: 0.0,
                    y: 0.0,
                    text: other.to_string(),
                    size: em,
                    face: Face::ROMAN,
                    ink,
                    width: w,
                });
                return b;
            }
        }
        Bx {
            w,
            asc,
            desc,
            items: vec![Item::Path {
                segs,
                fill: false,
                width,
                ink,
            }],
        }
    }

    fn decor(&mut self, word: &str, arg: &[Node], ctx: Ctx) -> Result<Bx, EquationError> {
        let em = self.em(ctx);
        let b = self.arg(arg, ctx)?;
        let side = self.p.decor_side * em;
        let gap = self.p.decor_gap * em;
        let t = self.p.rule * em;
        let width = b.w + 2.0 * side;
        let mut out = Bx::default();
        out.place(b.clone(), side, 0.0);
        out.w = width;
        let mut over_extent = b.asc;
        let mut under_extent = b.desc;
        match word {
            "overline" => {
                over_extent = b.asc + gap * 0.5;
                out.items.push(Item::Rule {
                    x: 0.0,
                    y: -over_extent - t,
                    w: width,
                    h: t,
                    ink: ctx.ink,
                });
                out.asc = out.asc.max(over_extent + t);
            }
            "underline" => {
                under_extent = b.desc + gap * 0.5;
                out.items.push(Item::Rule {
                    x: 0.0,
                    y: under_extent,
                    w: width,
                    h: t,
                    ink: ctx.ink,
                });
                out.desc = out.desc.max(under_extent + t);
            }
            "overbrace" | "underbrace" => {
                let height = self.p.brace_height * em * 0.5;
                let top = word == "overbrace";
                let y0 = if top { -(b.asc + gap) } else { b.desc + gap };
                let dir = if top { -1.0 } else { 1.0 };
                let (x0, x1, xm) = (0.0, width, width / 2.0);
                let y1 = y0 + dir * height;
                let segs = vec![
                    Seg::Move(x0, y0),
                    Seg::Cubic(
                        x0,
                        y0 + dir * height * 0.5,
                        x0 + height,
                        y1,
                        x0 + height * 2.0,
                        y1,
                    ),
                    Seg::Line(xm - height, y1),
                    Seg::Cubic(
                        xm - height * 0.3,
                        y1,
                        xm,
                        y1 + dir * height * 0.4,
                        xm,
                        y1 + dir * height * 0.9,
                    ),
                    Seg::Cubic(
                        xm,
                        y1 + dir * height * 0.4,
                        xm + height * 0.3,
                        y1,
                        xm + height,
                        y1,
                    ),
                    Seg::Line(x1 - height * 2.0, y1),
                    Seg::Cubic(x1 - height, y1, x1, y0 + dir * height * 0.5, x1, y0),
                ];
                out.items.push(Item::Path {
                    segs,
                    fill: false,
                    width: t,
                    ink: ctx.ink,
                });
                if top {
                    over_extent = b.asc + gap + height * 2.0;
                    out.asc = out.asc.max(over_extent);
                } else {
                    under_extent = b.desc + gap + height * 2.0;
                    out.desc = out.desc.max(under_extent);
                }
            }
            "widehat" | "widetilde" => {
                let y = -(b.asc + gap * 0.5);
                let h = gap * 1.4;
                let segs = if word == "widehat" {
                    vec![
                        Seg::Move(0.0, y),
                        Seg::Line(width / 2.0, y - h),
                        Seg::Line(width, y),
                    ]
                } else {
                    vec![
                        Seg::Move(0.0, y),
                        Seg::Cubic(
                            width * 0.25,
                            y - h * 1.5,
                            width * 0.4,
                            y - h * 0.2,
                            width * 0.5,
                            y - h * 0.6,
                        ),
                        Seg::Cubic(
                            width * 0.6,
                            y - h,
                            width * 0.75,
                            y - h * 1.2,
                            width,
                            y - h * 0.4,
                        ),
                    ]
                };
                out.items.push(Item::Path {
                    segs,
                    fill: false,
                    width: t,
                    ink: ctx.ink,
                });
                out.asc = out.asc.max(b.asc + gap * 0.5 + h * 1.2);
            }
            w if w.contains("arrow") => {
                let over = w.starts_with("over");
                let y = if over { -(b.asc + gap) } else { b.desc + gap };
                let head = gap * 0.8;
                let mut segs = vec![Seg::Move(0.0, y), Seg::Line(width, y)];
                if !w.contains("right") || w.contains("leftright") {
                    segs.push(Seg::Move(head, y - head / 2.0));
                    segs.push(Seg::Line(0.0, y));
                    segs.push(Seg::Line(head, y + head / 2.0));
                }
                if w.contains("right") {
                    segs.push(Seg::Move(width - head, y - head / 2.0));
                    segs.push(Seg::Line(width, y));
                    segs.push(Seg::Line(width - head, y + head / 2.0));
                }
                out.items.push(Item::Path {
                    segs,
                    fill: false,
                    width: t,
                    ink: ctx.ink,
                });
                if over {
                    out.asc = out.asc.max(b.asc + gap + head);
                } else {
                    out.desc = out.desc.max(b.desc + gap + head);
                }
            }
            _ => {
                // An accent drawn over the argument's first character.
                let mark = match word {
                    "hat" => '^',
                    "check" => 'ˇ',
                    "tilde" => '~',
                    "acute" => '´',
                    "grave" => '`',
                    "dot" => '˙',
                    "ddot" => '¨',
                    "breve" => '˘',
                    "bar" => '¯',
                    _ => '→',
                };
                let size = em * 0.8;
                let w = metrics::advance(mark, Face::ROMAN) * size;
                let lift = b.asc.min(0.5 * em);
                let accent_w = w.min(b.w);
                out.items.push(Item::Glyphs {
                    x: side + (b.w - accent_w) / 2.0,
                    y: -lift - 0.05 * em,
                    text: mark.to_string(),
                    size,
                    face: Face::ROMAN,
                    ink: ctx.ink,
                    width: w,
                });
                out.asc = out.asc.max(lift + 0.35 * em);
                out.w = b.w;
                out.items.iter_mut().for_each(|item| {
                    if let Item::Glyphs { x, .. } = item {
                        *x -= side;
                    }
                });
                let mut moved = Bx::default();
                moved.place(b.clone(), 0.0, 0.0);
                moved
                    .items
                    .extend(out.items.into_iter().skip(b.items.len()));
                moved.w = b.w;
                moved.asc = out.asc;
                return Ok(moved);
            }
        }
        let _ = (over_extent, under_extent);
        Ok(out)
    }

    fn stack(
        &mut self,
        under: bool,
        note: &[Node],
        base: &[Node],
        ctx: Ctx,
    ) -> Result<Bx, EquationError> {
        let em = self.em(ctx);
        let b = self.arg(base, ctx)?;
        let n = self.arg(note, Self::script_style(ctx))?;
        let width = b.w.max(n.w);
        let gap = 0.08 * em;
        let mut out = Bx::default();
        out.place(b.clone(), (width - b.w) / 2.0, 0.0);
        if under {
            out.place(n.clone(), (width - n.w) / 2.0, b.desc + gap + n.asc);
        } else {
            out.place(n.clone(), (width - n.w) / 2.0, -(b.asc + gap + n.desc));
        }
        out.w = width;
        Ok(out)
    }
}

impl Bx {
    /// The width a placed box added: used to advance past it.
    fn items_width_hint(&self, at: f64) -> f64 {
        (self.w - at).max(0.0)
    }
}

fn delim_char(delimiter: &str) -> Option<char> {
    if delimiter == "." {
        return None;
    }
    if let Some(word) = delimiter.strip_prefix('\\') {
        return symbols::delimiter_word(word);
    }
    delimiter.chars().next()
}

/// A letter in a mathematical alphabet that starts at `base` for `A`.
fn alphabet(c: char, base: u32) -> char {
    let offset = match c {
        'A'..='Z' => c as u32 - 'A' as u32,
        'a'..='z' => 26 + c as u32 - 'a' as u32,
        _ => return c,
    };
    // The few letters that live elsewhere in Unicode.
    let special = match (base, c) {
        (0x1d538, 'C') => Some('ℂ'),
        (0x1d538, 'H') => Some('ℍ'),
        (0x1d538, 'N') => Some('ℕ'),
        (0x1d538, 'P') => Some('ℙ'),
        (0x1d538, 'Q') => Some('ℚ'),
        (0x1d538, 'R') => Some('ℝ'),
        (0x1d538, 'Z') => Some('ℤ'),
        (0x1d49c, 'B') => Some('ℬ'),
        (0x1d49c, 'E') => Some('ℰ'),
        (0x1d49c, 'F') => Some('ℱ'),
        (0x1d49c, 'H') => Some('ℋ'),
        (0x1d49c, 'I') => Some('ℐ'),
        (0x1d49c, 'L') => Some('ℒ'),
        (0x1d49c, 'M') => Some('ℳ'),
        (0x1d49c, 'R') => Some('ℛ'),
        (0x1d49c, 'e') => Some('ℯ'),
        (0x1d49c, 'g') => Some('ℊ'),
        (0x1d49c, 'o') => Some('ℴ'),
        (0x1d504, 'C') => Some('ℭ'),
        (0x1d504, 'H') => Some('ℌ'),
        (0x1d504, 'I') => Some('ℑ'),
        (0x1d504, 'R') => Some('ℜ'),
        (0x1d504, 'Z') => Some('ℨ'),
        _ => None,
    };
    special.unwrap_or_else(|| char::from_u32(base + offset).unwrap_or(c))
}

// ---------------------------------------------------------------------
// Drawing

/// The colours a drawing is made in.
#[derive(Clone, Copy, Debug)]
pub struct Colours {
    pub math: (u8, u8, u8),
    pub text: (u8, u8, u8),
    pub function: (u8, u8, u8),
}

impl Colours {
    /// Everything black (printing and the page).
    pub const BLACK: Self = Self {
        math: (0, 0, 0),
        text: (0, 0, 0),
        function: (0, 0, 0),
    };
    /// Ami Pro's `[EQColor]` defaults (the editing display).
    pub const EDITING: Self = Self {
        math: (255, 0, 0),
        text: (0, 0, 0),
        function: (128, 128, 128),
    };

    fn of(self, ink: Ink) -> crate::dib::Rgb {
        let (r, g, b) = match ink {
            Ink::Math => self.math,
            Ink::Text => self.text,
            Ink::Function => self.function,
        };
        [r, g, b]
    }
}

impl Layout {
    /// The display list for the equation, in the picture frame's window.
    pub fn to_drawing(&self, face: &str, colours: Colours) -> wmf::Drawing {
        let w = f64::from(self.width.max(1));
        let h = f64::from(self.height.max(1));
        let unit = |x: f64, y: f64| wmf::Point {
            x: (x / w) as f32,
            y: (y / h) as f32,
        };
        let mut ops = Vec::new();
        let face = if face.eq_ignore_ascii_case("TimesNewRomanPS") || face.is_empty() {
            "Times New Roman"
        } else {
            face
        };
        for item in &self.items {
            match item {
                Item::Glyphs {
                    x,
                    y,
                    text,
                    size,
                    face: f,
                    ink,
                    ..
                } => ops.push(wmf::Op::Text {
                    origin: unit(*x, *y),
                    text: text.clone(),
                    font: wmf::Font {
                        face: if f.mono {
                            "Courier New".into()
                        } else {
                            face.to_string()
                        },
                        height: (*size / h) as f32,
                        cell_height: false,
                        escapement: 0,
                        weight: if f.bold { 700 } else { 400 },
                        italic: f.italic,
                        underline: false,
                        strike_out: false,
                    },
                    colour: colours.of(*ink),
                    background: None,
                    align: 24,
                    advances: None,
                    spread: None,
                    clip: None,
                    opaque: None,
                }),
                Item::Rule {
                    x,
                    y,
                    w: rw,
                    h: rh,
                    ink,
                } => {
                    let a = unit(*x, *y);
                    let b = unit(x + rw, y + rh);
                    ops.push(wmf::Op::Path {
                        path: vec![
                            wmf::PathElement::MoveTo(a),
                            wmf::PathElement::LineTo(wmf::Point { x: b.x, y: a.y }),
                            wmf::PathElement::LineTo(b),
                            wmf::PathElement::LineTo(wmf::Point { x: a.x, y: b.y }),
                            wmf::PathElement::Close,
                        ],
                        fill: Some(wmf::Fill::Solid(colours.of(*ink))),
                        nonzero: true,
                        stroke: None,
                    });
                }
                Item::Path {
                    segs,
                    fill,
                    width,
                    ink,
                } => {
                    let path = segs
                        .iter()
                        .map(|seg| match *seg {
                            Seg::Move(x, y) => wmf::PathElement::MoveTo(unit(x, y)),
                            Seg::Line(x, y) => wmf::PathElement::LineTo(unit(x, y)),
                            Seg::Cubic(a, b, c, d, e, f) => {
                                wmf::PathElement::CubicTo(unit(a, b), unit(c, d), unit(e, f))
                            }
                            Seg::Close => wmf::PathElement::Close,
                        })
                        .collect();
                    let colour = colours.of(*ink);
                    ops.push(wmf::Op::Path {
                        path,
                        fill: fill.then(|| wmf::Fill::Solid(colour)),
                        nonzero: true,
                        stroke: (!*fill).then_some(wmf::Stroke {
                            colour,
                            width: (*width / w) as f32,
                            style: 0,
                        }),
                    });
                }
            }
        }
        wmf::Drawing {
            ops,
            window: (self.width.max(1), self.height.max(1)),
            placeable: Some(crate::Twips {
                width: self.width.max(1),
                height: self.height.max(1),
            }),
        }
    }
}
