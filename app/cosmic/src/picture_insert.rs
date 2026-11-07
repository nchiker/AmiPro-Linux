//! Insert › Image… (`docs/Design/UI_SPEC.md` §25; Ami Pro's File › Import
//! Picture, `10d8:126c`): where a picture goes, and the frame made for it.

use amipro_document::{
    CellContent, Document, FlowId, FrameContent, FrameId, PictureFormat, Rect, Size, Twips,
};
use amipro_editing::TextPosition;
use amipro_editing::frame_creation::FrameDefaults;

use crate::frame_editing::FrameSelection;

/// An image on the clipboard, for Paste (P7): the first of the formats
/// Insert › Image… takes that the clipboard offers, or else the files a
/// file manager copied (`text/uri-list`), which paste as a drop puts them.
#[derive(Clone, Debug)]
pub struct ClipboardImage {
    pub bytes: Vec<u8>,
    pub mime: String,
}

/// The clipboard's image types, most wanted first: PNG keeps the most.
const IMAGE_TYPES: [&str; 6] = [
    "image/png",
    "image/jpeg",
    "image/gif",
    "image/bmp",
    "image/tiff",
    "image/svg+xml",
];

/// Copied files, after the images.
const FILE_LIST: &str = "text/uri-list";

/// What the page takes in a drop, most wanted first: the image a browser
/// drags, then files.
pub fn drop_types() -> Vec<std::borrow::Cow<'static, str>> {
    IMAGE_TYPES
        .iter()
        .chain([&FILE_LIST])
        .map(|&mime| std::borrow::Cow::Borrowed(mime))
        .collect()
}

impl TryFrom<(Vec<u8>, String)> for ClipboardImage {
    type Error = ();

    fn try_from((bytes, mime): (Vec<u8>, String)) -> Result<Self, ()> {
        (!bytes.is_empty() && (IMAGE_TYPES.contains(&mime.as_str()) || mime == FILE_LIST))
            .then_some(Self { bytes, mime })
            .ok_or(())
    }
}

impl cosmic::iced::clipboard::mime::AllowedMimeTypes for ClipboardImage {
    fn allowed() -> std::borrow::Cow<'static, [String]> {
        IMAGE_TYPES
            .iter()
            .chain([&FILE_LIST])
            .map(|&mime| mime.to_owned())
            .collect()
    }
}

impl ClipboardImage {
    /// The picture files copied, when the clipboard holds files rather
    /// than an image.
    pub fn files(&self) -> Option<Vec<std::path::PathBuf>> {
        (self.mime == FILE_LIST).then(|| dropped_pictures(&self.bytes))
    }

    /// Whether Paste would put a picture in.
    pub fn is_picture(&self) -> bool {
        self.files().is_none_or(|files| !files.is_empty())
    }

    /// The file extension of its type.
    pub fn extension(&self) -> &'static str {
        match self.mime.as_str() {
            "image/jpeg" => ".jpg",
            "image/gif" => ".gif",
            "image/bmp" => ".bmp",
            "image/tiff" => ".tif",
            "image/svg+xml" => ".svg",
            _ => ".png",
        }
    }
}

/// Whether the clipboard's text is only where a picture is: one web
/// address or file path. A browser's Copy Image puts the image's address
/// there beside the image, and a file manager the file's path, so Paste
/// looks for the picture first (user, 2026-10-18).
pub fn link_text(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && !text.contains(char::is_whitespace)
        && (["http://", "https://", "file://", "data:image/"]
            .iter()
            .any(|scheme| text.starts_with(scheme))
            || text.starts_with('/'))
}

/// The picture files of a drop's `text/uri-list` (local `file:` URIs of the
/// formats Insert › Image… takes), in order.
pub fn dropped_pictures(data: &[u8]) -> Vec<std::path::PathBuf> {
    String::from_utf8_lossy(data)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(file_path)
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(takes)
        })
        .collect()
}

/// The documents File › Open reads among dropped files (IM1).
pub fn dropped_documents(data: &[u8]) -> Vec<std::path::PathBuf> {
    String::from_utf8_lossy(data)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(file_path)
        .filter(|path| crate::import::is_document(path))
        .collect()
}

/// A local `file:` URI's path, percent-decoded.
fn file_path(uri: &str) -> Option<std::path::PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let path = rest.strip_prefix("localhost").unwrap_or(rest);
    if !path.starts_with('/') {
        return None;
    }
    let mut bytes = Vec::with_capacity(path.len());
    let mut iter = path.bytes();
    while let Some(byte) = iter.next() {
        if byte == b'%' {
            let high = (iter.next()? as char).to_digit(16)?;
            let low = (iter.next()? as char).to_digit(16)?;
            bytes.push((high * 16 + low) as u8);
        } else {
            bytes.push(byte);
        }
    }
    use std::os::unix::ffi::OsStringExt;
    Some(std::path::PathBuf::from(std::ffi::OsString::from_vec(
        bytes,
    )))
}

/// Where Insert › Image… puts a picture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// The selected picture frame's picture is replaced (Replace Picture).
    Replace(FrameId),
    /// Into the selected empty text frame (P02).
    Frame(FrameId),
    /// Into the caret's empty table cell (`14d0:05ec`, T3).
    Cell { table: FrameId, cell: usize },
    /// Into a new Flow with text frame at the caret (`1260:0509`, P3).
    NewFrame(TextPosition),
}

/// The formats Insert › Image… takes (decision P1).
pub const FORMATS: [&str; 11] = [
    "bmp", "wmf", "emf", "png", "jpg", "jpeg", "gif", "tif", "tiff", "pcx", "svg",
];

/// Whether Insert › Image… takes a file with this extension.
pub fn takes(extension: &str) -> bool {
    let extension = extension.trim_start_matches('.').to_ascii_lowercase();
    FORMATS.contains(&extension.as_str())
        && PictureFormat::from_extension(&extension) != PictureFormat::Other
}

/// The table frame and cell whose text is `flow`, if any.
pub fn cell_of_flow(document: &Document, flow: FlowId) -> Option<(FrameId, usize)> {
    document.frames().find_map(|frame| match &frame.content {
        FrameContent::Table(table) => table
            .cells
            .iter()
            .position(|cell| matches!(cell.content, CellContent::Text(text) if text == flow))
            .map(|cell| (frame.id, cell)),
        _ => None,
    })
}

/// Whether `flow` is one empty paragraph.
fn empty(document: &Document, flow: FlowId) -> bool {
    document.flow(flow).is_some_and(
        |flow| matches!(&flow.paragraphs[..], [paragraph] if paragraph.text.is_empty()),
    )
}

/// Where a picture goes, as Ami Pro's Import Picture decides it: into the
/// one selected frame when it is empty text (or replacing its picture);
/// into the caret's table cell when it is empty; else at the caret in the
/// main text. Refused in a cell with text or a picture (TF12, T1) and in
/// other text (headers, footers, frames), where a frame cannot be anchored.
pub fn target(
    document: &Document,
    frames: Option<&FrameSelection>,
    caret: TextPosition,
) -> Result<Target, &'static str> {
    if let Some(selection) = frames {
        let [frame] = selection.frames.as_slice() else {
            return Err("Select one frame, or click in the text, to insert a picture");
        };
        let Some(found) = document.frame(*frame) else {
            return Err("The frame no longer exists");
        };
        return match found.content {
            FrameContent::Picture(_) => Ok(Target::Replace(*frame)),
            FrameContent::Text(flow) if empty(document, flow) => Ok(Target::Frame(*frame)),
            _ => Err("A picture goes into an empty frame; this one holds text or a table"),
        };
    }
    if let Some((table, cell)) = cell_of_flow(document, caret.flow) {
        let FrameContent::Table(grid) = &document.frame(table).expect("found").content else {
            unreachable!("a table")
        };
        if grid.cells[cell].frame.is_some() {
            return Err("This cell holds a picture; Delete removes it first");
        }
        if !empty(document, caret.flow) {
            return Err("A picture goes into an empty cell; this one holds text");
        }
        return Ok(Target::Cell { table, cell });
    }
    if caret.flow != document.main_flow_id() {
        return Err("A picture goes in the main text, an empty frame, or an empty table cell");
    }
    Ok(Target::NewFrame(caret))
}

/// The size of the frame made for a picture of `natural` size (P3): the
/// picture's own size inside Ami Pro's default frame, its margins and
/// corner inset around it, reduced with its aspect ratio kept to fit
/// `column` (the column's width and height). Returns the frame's size and
/// the picture's.
pub fn new_frame_size(natural: Size, defaults: &FrameDefaults, column: Size) -> (Size, Size) {
    let insets = |frame: Size| {
        let style = defaults.frame_style(Rect::new(0, 0, frame.width, frame.height));
        style.text_insets(frame.width, frame.height)
    };
    let mut picture = Size {
        width: natural.width.max(1),
        height: natural.height.max(1),
    };
    let mut frame = picture;
    // Twice: the corner inset depends on the frame's size.
    for _ in 0..2 {
        let around = insets(frame);
        let extra = Size {
            width: around.left + around.right,
            height: around.top + around.bottom,
        };
        let room = Size {
            width: (column.width - extra.width).max(1),
            height: (column.height - extra.height).max(1),
        };
        let scale = (f64::from(room.width) / f64::from(picture.width))
            .min(f64::from(room.height) / f64::from(picture.height))
            .min(1.0);
        let scaled = |value: Twips| ((f64::from(value) * scale).floor() as Twips).max(1);
        picture = Size {
            width: scaled(picture.width),
            height: scaled(picture.height),
        };
        frame = Size {
            width: picture.width + extra.width,
            height: picture.height + extra.height,
        };
    }
    (frame, picture)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P3: PI1's bitmap (1,800 × 1,200) in Ami Pro's default frame (0.10 in
    /// margins, 18 % corners) fits a 6.5 in column as it is; one too large
    /// is reduced with its aspect ratio kept.
    #[test]
    fn a_new_pictures_frame_is_the_pictures_size_within_the_column() {
        let defaults = FrameDefaults::AMI_PRO;
        let column = Size {
            width: 9_360,
            height: 12_960,
        };
        let natural = Size {
            width: 1_800,
            height: 1_200,
        };
        let (frame, picture) = new_frame_size(natural, &defaults, column);
        assert_eq!(picture, natural);
        assert!(frame.width > picture.width + 288 && frame.height > picture.height + 288);
        let wide = Size {
            width: 30_000,
            height: 15_000,
        };
        let (frame, picture) = new_frame_size(wide, &defaults, column);
        assert!(frame.width <= column.width, "{frame:?}");
        assert!(
            (picture.width - 2 * picture.height).abs() <= 2,
            "{picture:?}"
        );
    }

    #[test]
    fn drops_and_the_clipboard_give_pictures() {
        let list = b"# from the file manager\r\nfile:///home/me/Logo%20one.PNG\r\nfile:///home/me/notes.txt\r\n";
        assert_eq!(
            dropped_pictures(list),
            [std::path::PathBuf::from("/home/me/Logo one.PNG")]
        );
        let image = ClipboardImage::try_from((vec![1], "image/jpeg".to_owned())).unwrap();
        assert_eq!(image.extension(), ".jpg");
        assert!(ClipboardImage::try_from((vec![1], "text/plain".to_owned())).is_err());
        assert!(ClipboardImage::try_from((Vec::new(), "image/png".to_owned())).is_err());
        // Copied files: a picture pastes, other files do not.
        let files = ClipboardImage::try_from((
            b"file:///home/a/logo.png\r\n".to_vec(),
            "text/uri-list".to_owned(),
        ))
        .unwrap();
        assert!(files.is_picture());
        let other = ClipboardImage::try_from((
            b"file:///home/a/notes.txt\r\n".to_vec(),
            "text/uri-list".to_owned(),
        ))
        .unwrap();
        assert!(!other.is_picture());
        assert!(image.is_picture());
        // What a browser's Copy Image and a file manager leave as text.
        assert!(link_text("https://example.com/a/logo.png"));
        assert!(link_text("/home/a/logo.png\n"));
        assert!(link_text("file:///home/a/logo.png"));
        assert!(!link_text("Some words"));
        assert!(!link_text("123"));
        assert!(!link_text("a\tb"));
        assert!(!link_text(""));
    }

    #[test]
    fn insert_image_takes_the_decided_formats() {
        for taken in ["bmp", ".WMF", "png", "jpeg", "gif", "tif", "pcx", "svg"] {
            assert!(takes(taken), "{taken}");
        }
        for refused in ["sdw", "doc", "eps"] {
            assert!(!takes(refused), "{refused}");
        }
    }
}
