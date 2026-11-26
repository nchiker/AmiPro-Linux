//! The plain text of an Ami Pro document, read from bytes, for search and indexing.
//!
//! Reads SAM files (`[ver]` first line) and this project's native files (`AMIPRO-LINUX/3`). Nothing is written, no
//! file name is looked at, and a password-protected file never yields text. Bad input gives
//! [`Extracted::Unreadable`], never a panic.

use amipro_document::Document;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Bump when the extracted text changes for the same input.
pub const VERSION: u32 = 1;

/// What may be read and returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    /// Input larger than this is [`Reason::TooLarge`].
    pub max_bytes: usize,
    /// Text is cut after this many characters.
    pub max_chars: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: 8 << 20,
            max_chars: 1_000_000,
        }
    }
}

/// What a file's first bytes say it is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Native,
    Sam,
    Protected,
    NotAmiPro,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reason {
    TooLarge,
    NotAmiPro,
    Damaged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Extracted {
    Text {
        title: Option<String>,
        text: String,
        truncated: bool,
    },
    Protected,
    Unreadable(Reason),
}

/// Classifies a file from its first bytes (at most 4096 are looked at); content only, never the name.
pub fn sniff(head: &[u8]) -> Kind {
    let head = &head[..head.len().min(4096)];
    if amipro_formats::protect::is_protected(head) {
        return Kind::Protected;
    }
    let native = b"AMIPRO-LINUX/3";
    if head.starts_with(native) && matches!(head.get(native.len()), None | Some(b'\n' | b'\r')) {
        return Kind::Native;
    }
    // A SAM starts with its `[ver]` section (the reader skips blank lines and a byte order mark before it).
    let body = head.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(head);
    let start = body
        .iter()
        .position(|byte| !matches!(byte, b'\r' | b'\n' | b' ' | b'\t'))
        .unwrap_or(body.len());
    let body = &body[start..];
    if body.len() >= 5 && body[..5].eq_ignore_ascii_case(b"[ver]") {
        return Kind::Sam;
    }
    Kind::NotAmiPro
}

/// All text of the document, one line per paragraph: the main flow, then the other flows (frames, table cells,
/// notes) in the model's flow order; blank flows are left out.
pub fn extract_text(bytes: &[u8], limits: &Limits) -> Extracted {
    if bytes.len() > limits.max_bytes {
        return Extracted::Unreadable(Reason::TooLarge);
    }
    let kind = sniff(bytes);
    match kind {
        Kind::NotAmiPro => return Extracted::Unreadable(Reason::NotAmiPro),
        Kind::Protected => return Extracted::Protected,
        Kind::Native | Kind::Sam => {}
    }
    let read = catch_unwind(AssertUnwindSafe(|| read_document(bytes, kind)));
    match read {
        Ok(Some(document)) => build(&document, kind, limits),
        _ => Extracted::Unreadable(Reason::Damaged),
    }
}

fn read_document(bytes: &[u8], kind: Kind) -> Option<Document> {
    match kind {
        Kind::Native => Document::decode_native(std::str::from_utf8(bytes).ok()?).ok(),
        _ => amipro_formats::import_sam(bytes, None, "Untitled")
            .ok()
            .map(|imported| imported.document),
    }
}

fn build(document: &Document, kind: Kind, limits: &Limits) -> Extracted {
    let main = document.main_flow_id();
    let mut parts = vec![document.main_flow().plain_text()];
    parts.extend(
        document
            .flows()
            .filter(|(id, _)| *id != main)
            .map(|(_, flow)| flow.plain_text()),
    );
    let mut text = String::new();
    let mut truncated = false;
    let mut chars = 0usize;
    'parts: for part in parts.iter().filter(|part| !part.trim().is_empty()) {
        if chars > 0 {
            if chars >= limits.max_chars {
                truncated = true;
                break;
            }
            text.push('\n');
            chars += 1;
        }
        for character in part.chars() {
            if chars >= limits.max_chars {
                truncated = true;
                break 'parts;
            }
            text.push(character);
            chars += 1;
        }
    }
    Extracted::Text {
        title: title_of(document, kind),
        text,
        truncated,
    }
}

/// The Doc Info description; else a native file's own title (a SAM has none: the importer's is a placeholder).
fn title_of(document: &Document, kind: Kind) -> Option<String> {
    let description = document.info().description.trim();
    if !description.is_empty() {
        return Some(description.to_owned());
    }
    let title = document.title.trim();
    (kind == Kind::Native && !title.is_empty() && title != "Untitled").then(|| title.to_owned())
}
