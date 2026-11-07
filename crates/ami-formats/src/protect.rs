//! Ami Pro 3.1's password-protected files (`[encrypt]`;
//! `docs/research/COMMANDS_NOTES.md` §9, VM fixtures PWA–PWSV2).
//!
//! A protected file is the line `[encrypt]`, then the file's text up to the
//! body's closing `>` line through an adaptive splay-tree prefix coder (the
//! kind D. W. Jones described in 1988), preceded by a check line made from
//! the password; then the end symbol, padding to a byte, and the rest of
//! the file plain: `>`, the pictures' bytes, and `[Embedded]`, whose
//! offsets are those of the protected file. The password's only part in the
//! coding is to splay the model once for each of its bytes before the first
//! symbol, so a wrong password decodes noise and fails the check line.
//!
//! This is obfuscation, not security: the check line is known text.

use crate::writer::{escape_text, windows_1252};

/// The first line of a protected file.
pub const ENCRYPT_LINE: &[u8] = b"[encrypt]\r\n";

/// Ami Pro's Password Protect box takes 14 characters (a 16-byte buffer).
pub const PASSWORD_LIMIT: usize = 14;

/// Whether `bytes` is a protected file.
pub fn is_protected(bytes: &[u8]) -> bool {
    bytes.starts_with(ENCRYPT_LINE)
        || bytes
            .strip_prefix(b"[encrypt]")
            .is_some_and(|rest| rest.starts_with(b"\n"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtectError {
    /// Not a protected file.
    NotProtected,
    /// The check line does not match: Ami Pro's message 3147.
    WrongPassword,
}

impl std::fmt::Display for ProtectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotProtected => "the file is not password protected",
            Self::WrongPassword => {
                "Invalid Password. Encrypted file will not open with incorrect password."
            }
        })
    }
}

impl std::error::Error for ProtectError {}

/// Internal nodes are 1 to 257; a node above that is a leaf, symbol `s`
/// at `s + 259` (0 to 255 the bytes, 256 the end), and node 258 a leaf no
/// symbol has, which decodes as byte 0xFF (as the decompiled test reads).
const INTERNAL: usize = 257;
const LEAF_BASE: usize = 259;
const END: usize = 256;
const ROOT: usize = 1;

struct Model {
    left: [u16; INTERNAL + 1],
    right: [u16; INTERNAL + 1],
    up: [u16; 2 * INTERNAL + 2],
}

impl Model {
    /// The complete tree over heap-numbered nodes, primed with `password`.
    fn new(password: &[u8]) -> Self {
        let mut model = Self {
            left: [0; INTERNAL + 1],
            right: [0; INTERNAL + 1],
            up: [0; 2 * INTERNAL + 2],
        };
        for node in 2..model.up.len() {
            model.up[node] = (node / 2) as u16;
        }
        for node in 1..=INTERNAL {
            model.left[node] = (2 * node) as u16;
            model.right[node] = (2 * node + 1) as u16;
        }
        for &byte in password {
            model.splay(usize::from(byte));
        }
        model
    }

    /// Splays the leaf of `symbol`: up two levels at a time, swapping the
    /// grandparent's far child with the path (Jones's semi-splaying).
    fn splay(&mut self, symbol: usize) {
        let mut a = symbol + LEAF_BASE;
        loop {
            let c = usize::from(self.up[a]);
            if c == ROOT {
                break;
            }
            let d = usize::from(self.up[c]);
            let mut b = usize::from(self.left[d]);
            if c == b {
                b = usize::from(self.right[d]);
                self.right[d] = a as u16;
            } else {
                self.left[d] = a as u16;
            }
            if a == usize::from(self.left[c]) {
                self.left[c] = b as u16;
            } else {
                self.right[c] = b as u16;
            }
            self.up[a] = d as u16;
            self.up[b] = c as u16;
            a = d;
            if a == ROOT {
                break;
            }
        }
    }

    /// `symbol`'s code, root first; the model then splays it.
    fn code(&mut self, symbol: usize) -> Vec<bool> {
        let mut path = Vec::with_capacity(32);
        let mut a = symbol + LEAF_BASE;
        while a != ROOT {
            let parent = usize::from(self.up[a]);
            path.push(usize::from(self.right[parent]) == a);
            a = parent;
        }
        path.reverse();
        self.splay(symbol);
        path
    }
}

#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    used: u8,
}

impl Bits {
    fn push(&mut self, bit: bool) {
        if self.used == 0 {
            self.bytes.push(0);
        }
        if bit {
            *self.bytes.last_mut().unwrap() |= 0x80 >> self.used;
        }
        self.used = (self.used + 1) % 8;
    }

    fn at_byte(&self) -> bool {
        self.used == 0
    }
}

/// The check line the writer codes first: the password as SAM text, a
/// space, and each byte's decimal code run together (`\tAB 6566`).
fn check_line(password: &str) -> Vec<u8> {
    let mut line = vec![b'\t'];
    line.extend(escape_text(password));
    line.push(b' ');
    for character in password.chars() {
        line.extend(windows_1252(character).to_string().bytes());
    }
    line.extend(b"\r\n");
    line
}

fn priming(password: &str) -> Vec<u8> {
    password.chars().map(windows_1252).collect()
}

/// Where the plain tail of an Ami Pro file begins: the body's closing `>`
/// line, found from the `[Embedded]` directory (its last line is its own
/// offset; its entries give the pictures' offsets, which follow `>`).
fn tail_start(plain: &[u8]) -> usize {
    let text = plain;
    let directory = (|| {
        let trimmed = text.strip_suffix(b"\r\n").unwrap_or(text);
        let last = trimmed.rsplit(|&byte| byte == b'\n').next()?;
        let offset: usize = std::str::from_utf8(last).ok()?.trim().parse().ok()?;
        text.get(offset..)?
            .starts_with(b"[Embedded]")
            .then_some(offset)
    })();
    if let Some(directory) = directory {
        let mut first = directory.saturating_sub(2);
        for (offset, _) in embedded_entries(&text[directory..]) {
            if offset > 0 {
                first = first.min(offset);
            }
        }
        if let Some(start) = first.checked_sub(3)
            && text.get(start..first) == Some(b">\r\n")
        {
            return start;
        }
    }
    // No directory: the last `>` line.
    text.windows(5)
        .rposition(|window| window == b"\r\n>\r\n")
        .map_or(text.len(), |at| at + 2)
}

/// The `[Embedded]` directory's picture offsets: (offset, length) pairs,
/// two per entry (`seq ext offset length offset length [path]`).
fn embedded_entries(directory: &[u8]) -> Vec<(usize, usize)> {
    let text = String::from_utf8_lossy(directory);
    let mut lines: Vec<&str> = text.lines().skip(1).collect();
    lines.pop();
    let mut pairs = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(' ').collect();
        for index in [2, 4] {
            if let (Some(offset), Some(length)) = (
                fields.get(index).and_then(|value| value.parse().ok()),
                fields.get(index + 1).and_then(|value| value.parse().ok()),
            ) {
                pairs.push((offset, length));
            }
        }
    }
    pairs
}

/// The tail with every `[Embedded]` offset moved by `to - from` (the
/// directory's lines are rewritten; the pictures' bytes stay).
fn moved_tail(tail: &[u8], from: usize, to: usize) -> Vec<u8> {
    let Some(at) = tail.windows(10).rposition(|window| window == b"[Embedded]") else {
        return tail.to_vec();
    };
    let shift = |offset: usize| (offset + to).saturating_sub(from);
    let mut out = tail[..at].to_vec();
    let directory = String::from_utf8_lossy(&tail[at..]).into_owned();
    let lines: Vec<&str> = directory.split("\r\n").collect();
    let count = lines.len();
    for (index, line) in lines.iter().enumerate() {
        let last_line = index + 2 == count && lines[count - 1].is_empty()
            || index + 1 == count && !line.is_empty();
        let rewritten = if index == 0 {
            (*line).to_owned()
        } else if last_line {
            match line.trim().parse::<usize>() {
                Ok(offset) => format!("{:08}", shift(offset)),
                Err(_) => (*line).to_owned(),
            }
        } else {
            let mut fields: Vec<String> = line.split(' ').map(str::to_owned).collect();
            for index in [2, 4] {
                let length = fields
                    .get(index + 1)
                    .and_then(|value| value.parse::<usize>().ok());
                if let Some(field) = fields.get_mut(index)
                    && let Ok(offset) = field.parse::<usize>()
                    && length.is_some_and(|length| length > 0)
                {
                    *field = shift(offset).to_string();
                }
            }
            fields.join(" ")
        };
        out.extend(rewritten.bytes());
        if index + 1 < count {
            out.extend(b"\r\n");
        }
    }
    out
}

/// `plain` (an Ami Pro file as the writers write it) protected with
/// `password`, as Ami Pro 3.1 writes it (PWA: byte for byte).
pub fn protect(plain: &[u8], password: &str) -> Vec<u8> {
    let split = tail_start(plain);
    let mut model = Model::new(&priming(password));
    let mut bits = Bits::default();
    for &byte in check_line(password).iter().chain(&plain[..split]) {
        for bit in model.code(usize::from(byte)) {
            bits.push(bit);
        }
    }
    for bit in model.code(END) {
        bits.push(bit);
    }
    // Spaces until the output reaches the next byte boundary, cut there:
    // 1 to 8 bits, a whole byte when the end symbol ends on one (PWA,
    // PWAB).
    'padding: loop {
        for bit in model.code(usize::from(b' ')) {
            bits.push(bit);
            if bits.at_byte() {
                break 'padding;
            }
        }
    }
    let mut out = ENCRYPT_LINE.to_vec();
    out.extend(&bits.bytes);
    let tail = moved_tail(&plain[split..], split, out.len());
    out.extend(tail);
    out
}

/// The plain file inside `protected`, if `password` is right: its text and
/// tail, with the `[Embedded]` offsets as a plain file has them.
pub fn unprotect(protected: &[u8], password: &str) -> Result<Vec<u8>, ProtectError> {
    if !is_protected(protected) {
        return Err(ProtectError::NotProtected);
    }
    let start = protected
        .iter()
        .position(|&byte| byte == b'\n')
        .map_or(protected.len(), |at| at + 1);
    let coded = &protected[start..];
    let mut model = Model::new(&priming(password));
    let mut decoded = Vec::with_capacity(coded.len() * 2);
    let mut bit = 0usize;
    let total = coded.len() * 8;
    let mut ended = false;
    let expected = check_line(password);
    let mut checked = false;
    'symbols: while bit < total {
        let mut node = ROOT;
        while node <= INTERNAL {
            if bit >= total {
                break 'symbols;
            }
            let on = coded[bit / 8] & (0x80 >> (bit % 8)) != 0;
            bit += 1;
            node = usize::from(if on {
                model.right[node]
            } else {
                model.left[node]
            });
        }
        let symbol = node.wrapping_sub(LEAF_BASE);
        if symbol == END {
            model.splay(END);
            ended = true;
            break;
        }
        // Node 258 decodes as 0xFF without being a symbol of its own.
        let byte = (node.wrapping_sub(3) & 0xFF) as u8;
        decoded.push(byte);
        if symbol < END {
            model.splay(symbol);
        }
        // The check line, as soon as it is whole: a wrong password fails
        // within a few bytes (Ami Pro reads one line, then `[ver]`).
        if !checked && byte == b'\n' {
            checked = true;
            let bare = [b"\t".as_slice(), &escape_text(password), b"\r\n"].concat();
            if decoded != expected && decoded != bare {
                return Err(ProtectError::WrongPassword);
            }
        }
        if !checked && decoded.len() > expected.len() + 4 && decoded.len() > 64 {
            return Err(ProtectError::WrongPassword);
        }
    }
    if !checked {
        return Err(ProtectError::WrongPassword);
    }
    let check = decoded
        .iter()
        .position(|&byte| byte == b'\n')
        .map_or(0, |at| at + 1);
    let mut plain = decoded[check..].to_vec();
    if !plain.starts_with(b"[ver]") {
        return Err(ProtectError::WrongPassword);
    }
    // The padding after the end symbol is 1 to 8 bits.
    let tail_from = if ended {
        start + bit / 8 + 1
    } else {
        protected.len()
    };
    let tail = &protected[tail_from.min(protected.len())..];
    let moved = moved_tail(tail, tail_from, plain.len());
    plain.extend(moved);
    Ok(plain)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: &[u8] =
        b"[ver]\r\n\t4\r\n[edoc]\r\nHello\r\n\r\n>\r\n\r\n[Embedded]\r\n00000033\r\n";

    #[test]
    fn protected_text_reads_back_with_its_password_only() {
        assert_eq!(tail_start(PLAIN), 28);
        for password in ["A", "AB", "<A", "é", "", "ABCDEFGHIJKLMN"] {
            let protected = protect(PLAIN, password);
            assert!(is_protected(&protected));
            assert_eq!(
                unprotect(&protected, password).unwrap(),
                PLAIN,
                "{password:?}"
            );
            let text = String::from_utf8_lossy(&protected);
            let trailer: usize = text
                .trim_end()
                .rsplit("\r\n")
                .next()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(
                &protected[trailer..trailer + 10],
                b"[Embedded]",
                "{password:?}"
            );
        }
        let protected = protect(PLAIN, "A");
        assert_eq!(unprotect(&protected, "a"), Err(ProtectError::WrongPassword));
        assert_eq!(unprotect(&protected, "B"), Err(ProtectError::WrongPassword));
        assert_eq!(unprotect(PLAIN, "A"), Err(ProtectError::NotProtected));
    }

    #[test]
    fn the_check_line_is_the_escaped_password_and_its_codes() {
        assert_eq!(check_line("AB"), b"\tAB 6566\r\n");
        assert_eq!(check_line("<A"), b"\t<<A 6065\r\n");
        assert_eq!(check_line("\u{e9}"), b"\t<\\i> 233\r\n");
        assert_eq!(check_line(""), b"\t \r\n");
    }

    #[test]
    fn picture_offsets_move_with_the_coded_part() {
        let plain = b"[ver]\r\n\t4\r\n[edoc]\r\nHi\r\n\r\n>\r\nPICTURE!\r\n[Embedded]\r\n1 .bmp 28 8 0 0 \r\n00000038\r\n";
        assert_eq!(&plain[28..36], b"PICTURE!");
        assert_eq!(tail_start(plain), 25);
        let protected = protect(plain, "pw");
        let text = String::from_utf8_lossy(&protected);
        let entry = text.lines().rev().nth(1).unwrap();
        let offset: usize = entry.split(' ').nth(2).unwrap().parse().unwrap();
        assert_eq!(&protected[offset..offset + 8], b"PICTURE!");
        assert_eq!(unprotect(&protected, "pw").unwrap(), plain);
    }
}
