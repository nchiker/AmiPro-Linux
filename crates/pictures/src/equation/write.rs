//! The tree written back as Ami Pro writes it (notes §12.2): math runs in
//! `$ ... $`, single-atom arguments without braces, `\frac ab`, a space
//! after a control word only where a letter follows, `\matrix a & b \\ c &
//! d\endmatrix `, `\left( x\right) `.

use super::parse::{GridKind, Node};

struct Out {
    text: String,
    /// The last thing written was a control word ending in a letter.
    after_word: bool,
}

impl Out {
    fn put(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if self.after_word && text.starts_with(|c: char| c.is_ascii_alphabetic()) {
            self.text.push(' ');
        }
        self.after_word = false;
        self.text.push_str(text);
    }

    fn word(&mut self, name: &str) {
        self.after_word = false;
        self.text.push('\\');
        self.text.push_str(name);
        self.after_word = name.ends_with(|c: char| c.is_ascii_alphabetic());
    }

    /// Text that carries its own spacing.
    fn raw(&mut self, text: &str) {
        self.after_word = false;
        self.text.push_str(text);
    }
}

/// An equation tree as stored text.
pub fn write(tree: &[Node]) -> String {
    let mut out = Out {
        text: String::new(),
        after_word: false,
    };
    for node in tree {
        top(node, &mut out);
    }
    out.text
}

fn top(node: &Node, out: &mut Out) {
    match node {
        Node::Math(nodes) => {
            out.raw("$");
            seq(nodes, out);
            out.raw("$");
        }
        Node::Text(text) => out.put(text),
        Node::Font(kind) => out.word(kind.word()),
        other => seq(std::slice::from_ref(other), out),
    }
}

fn seq(nodes: &[Node], out: &mut Out) {
    for node in nodes {
        one(node, out);
    }
}

/// An argument: braces unless it is a single atom.
fn arg(nodes: &[Node], out: &mut Out) {
    match nodes {
        [only] if only.is_atom() => one(only, out),
        _ => braced(nodes, out),
    }
}

/// An argument that is always braced.
fn braced(nodes: &[Node], out: &mut Out) {
    out.raw("{");
    seq(nodes, out);
    out.raw("}");
}

fn scripts(sub: &Option<Vec<Node>>, sup: &Option<Vec<Node>>, out: &mut Out) {
    if let Some(sub) = sub {
        out.raw("_");
        arg(sub, out);
    }
    if let Some(sup) = sup {
        out.raw("^");
        arg(sup, out);
    }
}

fn one(node: &Node, out: &mut Out) {
    match node {
        Node::Ch(c) => out.put(&c.to_string()),
        Node::Sym(name) => out.word(name),
        Node::Text(text) => {
            out.word("text");
            out.raw("{");
            out.raw(text);
            out.raw("}");
        }
        Node::Font(kind) => out.word(kind.word()),
        Node::Space(name) => out.word(name),
        Node::Group(nodes) => braced(nodes, out),
        Node::Math(nodes) => {
            out.raw("$");
            seq(nodes, out);
            out.raw("$");
        }
        Node::Frac { word, num, den } => {
            if word == "atopfrac" {
                // The infix family is written with braces dropped.
                seq(num, out);
                out.word("atop");
                seq(den, out);
            } else {
                out.word(word);
                arg(num, out);
                arg(den, out);
            }
        }
        Node::Sqrt(body) => {
            out.word("sqrt");
            braced(body, out);
        }
        Node::Root { index, body } => {
            out.word("root");
            braced(index, out);
            out.raw(" ");
            out.word("of");
            braced(body, out);
        }
        Node::Scripts { base, sub, sup } => {
            // A base that is an empty group is no base at all.
            if !matches!(&**base, Node::Group(nodes) if nodes.is_empty()) {
                one(base, out);
            }
            scripts(sub, sup, out);
        }
        Node::BigOp {
            word,
            limits,
            sub,
            sup,
        } => {
            out.word(word);
            if let Some(limits) = limits {
                out.raw(" ");
                out.word(if *limits { "limits" } else { "nolimits" });
            }
            if sub.is_none() && sup.is_none() {
                out.raw(" ");
            }
            scripts(sub, sup, out);
        }
        Node::Func { name, unnamed } => {
            if *unnamed {
                out.word("func");
                out.raw(&format!("{{{name}}}"));
            } else {
                out.word(name);
                out.raw(" ");
            }
        }
        Node::Grid { kind, format, rows } => {
            let (open, close) = match kind {
                GridKind::Matrix => ("matrix", "endmatrix"),
                GridKind::Sub => ("Sb", "endSb"),
                GridKind::Sup => ("Sp", "endSp"),
            };
            out.word(open);
            out.raw(" ");
            if let Some(format) = format {
                out.word("format");
                out.raw(&format!(" {format} \\\\ "));
            }
            for (r, row) in rows.iter().enumerate() {
                if r > 0 {
                    out.raw(" \\\\ ");
                }
                for (c, cell) in row.iter().enumerate() {
                    if c > 0 {
                        out.raw(" & ");
                    }
                    seq(cell, out);
                }
            }
            out.word(close);
            out.raw(" ");
        }
        Node::Delimited { left, right, body } => {
            out.word("left");
            out.raw(left);
            out.raw(" ");
            seq(body, out);
            out.word("right");
            out.raw(right);
            out.raw(" ");
        }
        Node::Decor { word, arg } => {
            out.word(word);
            braced(arg, out);
        }
        Node::Stack { under, note, base } => {
            out.word(if *under { "underset" } else { "overset" });
            braced(note, out);
            out.word("to");
            braced(base, out);
        }
    }
}
