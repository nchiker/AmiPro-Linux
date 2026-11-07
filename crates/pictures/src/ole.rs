//! OLE 1.0 object streams (Ami Pro's `AmiOLE` driver; `OBJECTS_NOTES.md`
//! section 4): the bytes of an OLE frame, read for the class, the native data,
//! and the presentation Ami Pro draws it from, a `METAFILEPICT` holding a
//! playable metafile.
//!
//! The layout (all numbers little-endian 32 bit, strings length-prefixed with
//! their NUL): version `0x501`, format (1 linked, 2 embedded, 3 static), class,
//! then for embedded and linked objects the topic and item names, then the
//! native data (size and bytes; a linked object has a network name, three
//! words and the update option instead), then the presentation: version,
//! format id (5: a class string follows), the string (`METAFILEPICT`), width,
//! height, data size, data. Bytes after the presentation are kept by the
//! document and only counted here.
//!
//! The reader never panics: what is out of bounds ends the reading and is
//! named in `problems`.

use crate::Bytes;

/// What the object stream holds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Linked,
    Embedded,
    Static,
}

/// The presentation object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Presentation {
    /// The clipboard format's name (`METAFILEPICT`, `BITMAP`, `DIB`), or
    /// the standard format's number for a presentation that gives one.
    pub format: String,
    /// The width and height the stream gives.
    pub width: u32,
    pub height: u32,
    /// Where the presentation's data is in the stream.
    pub data: (usize, usize),
    /// The `METAFILEPICT`'s mapping mode and extents, when the data is one.
    pub mapping_mode: Option<u16>,
    pub extent: Option<(u16, u16)>,
    /// Where the metafile starts and how long it is in the stream, when the
    /// data holds one.
    pub metafile: Option<(usize, usize)>,
}

/// An OLE 1.0 object stream, as far as it could be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OleObject {
    pub kind: Kind,
    pub class: String,
    pub topic: String,
    pub item: String,
    /// The native data (offset and length in the stream); none for a static
    /// object and a linked one.
    pub native: Option<(usize, usize)>,
    pub presentation: Option<Presentation>,
    /// Bytes after the presentation (or after the native data when it has
    /// none).
    pub trailing: usize,
    /// What could not be read, in the order met.
    pub problems: Vec<&'static str>,
}

const VERSION: u32 = 0x0000_0501;

/// The metafile's two signatures: a plain header (type 1 or 2, header size
/// 9 words) and Aldus' placeable key.
fn metafile_at(bytes: &[u8], at: usize) -> bool {
    let b = Bytes(bytes);
    match (b.u16(at), b.u16(at + 2)) {
        (Some(1 | 2), Some(9)) => true,
        _ => b.u32(at) == Some(0x9ac6_cdd7),
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u32(&mut self) -> Option<u32> {
        let value = Bytes(self.bytes).u32(self.at)?;
        self.at += 4;
        Some(value)
    }

    /// A length-prefixed string (the NUL dropped, bytes read as Latin-1).
    /// A length past the stream is an error.
    fn string(&mut self) -> Option<String> {
        let length = self.u32()? as usize;
        let end = self.at.checked_add(length)?;
        let raw = self.bytes.get(self.at..end)?;
        self.at = end;
        let text: String = raw
            .iter()
            .take_while(|&&byte| byte != 0)
            .map(|&byte| char::from(byte))
            .collect();
        Some(text)
    }
}

/// Reads `bytes` as an OLE 1.0 object stream, or `None` when it does not
/// start as one (the version and format words, a class string within the
/// stream).
pub fn read(bytes: &[u8]) -> Option<OleObject> {
    let mut reader = Reader { bytes, at: 0 };
    if reader.u32()? != VERSION {
        return None;
    }
    let kind = match reader.u32()? {
        1 => Kind::Linked,
        2 => Kind::Embedded,
        3 => Kind::Static,
        _ => return None,
    };
    let class = reader.string()?;
    let mut object = OleObject {
        kind,
        class,
        topic: String::new(),
        item: String::new(),
        native: None,
        presentation: None,
        trailing: 0,
        problems: Vec::new(),
    };
    if kind != Kind::Static {
        let names = reader.string().zip(reader.string());
        let Some((topic, item)) = names else {
            object
                .problems
                .push("the topic and item names run past the stream");
            object.trailing = bytes.len().saturating_sub(reader.at);
            return Some(object);
        };
        object.topic = topic;
        object.item = item;
    }
    match kind {
        Kind::Embedded => {
            let size = reader.u32().map(|size| size as usize);
            match size.and_then(|size| Some((size, reader.at.checked_add(size)?))) {
                Some((size, end)) if end <= bytes.len() => {
                    object.native = Some((reader.at, size));
                    reader.at = end;
                }
                _ => {
                    object.problems.push("the native data runs past the stream");
                    object.trailing = bytes.len().saturating_sub(reader.at);
                    return Some(object);
                }
            }
        }
        Kind::Linked => {
            // Network name, reserved, network type, network version, update
            // options.
            if reader.string().is_none() || (0..4).any(|_| reader.u32().is_none()) {
                object
                    .problems
                    .push("the link's network data runs past the stream");
                object.trailing = bytes.len().saturating_sub(reader.at);
                return Some(object);
            }
        }
        Kind::Static => {}
    }
    object.presentation = presentation(&mut reader, &mut object.problems);
    object.trailing = bytes.len().saturating_sub(reader.at);
    Some(object)
}

fn presentation(reader: &mut Reader<'_>, problems: &mut Vec<&'static str>) -> Option<Presentation> {
    if reader.at >= reader.bytes.len() {
        problems.push("the stream has no presentation");
        return None;
    }
    if reader.u32() != Some(VERSION) {
        problems.push("the presentation does not start with its version");
        return None;
    }
    // 0: none, 5: a class string follows, other: a standard format number.
    let format = match reader.u32() {
        Some(0) => {
            problems.push("the presentation is empty");
            return None;
        }
        Some(5) => match reader.string() {
            Some(name) => name,
            None => {
                problems.push("the presentation's format name runs past the stream");
                return None;
            }
        },
        Some(number) if number < 0x100 => format!("{number}"),
        _ => {
            problems.push("the presentation's format is not one Ami Pro writes");
            return None;
        }
    };
    let (Some(width), Some(height), Some(size)) = (reader.u32(), reader.u32(), reader.u32()) else {
        problems.push("the presentation's header runs past the stream");
        return None;
    };
    let start = reader.at;
    let end = start.checked_add(size as usize);
    let Some(end) = end.filter(|&end| end <= reader.bytes.len()) else {
        problems.push("the presentation's data runs past the stream");
        return None;
    };
    reader.at = end;
    let data = &reader.bytes[start..end];
    let is_metafile = format == "METAFILEPICT" || format == "3";
    let (mut mapping_mode, mut extent, mut metafile) = (None, None, None);
    if is_metafile {
        let b = Bytes(data);
        // The 16-bit `METAFILEPICT` (mode, x and y extents, handle) is eight
        // bytes; a 32-bit one 16. The metafile follows.
        for header in [8usize, 16] {
            if metafile_at(data, header) {
                mapping_mode = b.u16(0);
                extent = b.u16(2).zip(b.u16(4));
                metafile = Some((start + header, data.len() - header));
                break;
            }
        }
        if metafile.is_none() {
            problems.push("the presentation holds no metafile");
        }
    }
    Some(Presentation {
        format,
        width,
        height,
        data: (start, size as usize),
        mapping_mode,
        extent,
        metafile,
    })
}

impl OleObject {
    /// The presentation's metafile bytes in `stream` (the bytes this object
    /// was read from).
    pub fn metafile<'a>(&self, stream: &'a [u8]) -> Option<&'a [u8]> {
        let (start, length) = self.presentation.as_ref()?.metafile?;
        stream.get(start..start.checked_add(length)?)
    }

    /// The native data's length, or 0.
    pub fn native_len(&self) -> usize {
        self.native.map_or(0, |(_, length)| length)
    }

    /// A friendly label for the class: the program that made the object.
    pub fn label(&self) -> String {
        let class = self.class.as_str();
        let lower = class.to_ascii_lowercase();
        if lower.contains("wordart") {
            "WordArt".into()
        } else if lower.contains("clipart") || lower.contains("clip_art") {
            "ClipArt Gallery".into()
        } else if lower.starts_with("123worksheet") {
            "1-2-3 Worksheet".into()
        } else if lower.starts_with("excel") {
            "Excel worksheet".into()
        } else if class.is_empty() {
            "OLE object".into()
        } else {
            class.to_owned()
        }
    }
}

/// `metafile` with Aldus' placeable header giving it `width` by `height`
/// twips (bounds in twips, 1,440 units to the inch): a plain metafile
/// carries no size, so a picture made from an object's presentation takes
/// the presentation's.
pub fn placeable_metafile(metafile: &[u8], width: i32, height: i32) -> Vec<u8> {
    let clamp = |value: i32| value.clamp(1, i32::from(i16::MAX)) as i16;
    let mut out = Vec::with_capacity(22 + metafile.len());
    out.extend_from_slice(&0x9ac6_cdd7u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    for value in [0, 0, clamp(width), clamp(height)] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&1_440u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let checksum = out.chunks(2).fold(0u16, |sum, word| {
        sum ^ u16::from_le_bytes([word[0], word[1]])
    });
    out.extend_from_slice(&checksum.to_le_bytes());
    out.extend_from_slice(metafile);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placeable_header_gives_the_size_asked() {
        let placeable = placeable_metafile(&tiny_metafile(), 2_880, 1_440);
        assert_eq!(placeable.len(), 22 + tiny_metafile().len());
        let size = crate::wmf::placeable_size(&placeable).unwrap();
        assert_eq!((size.width, size.height), (2_880, 1_440));
        assert!(metafile_at(&placeable, 0));
        assert_eq!(&placeable[22..], tiny_metafile());
    }

    /// A minimal plain metafile: header (type 1, size 9, version 0x300,
    /// size in words, no objects, largest record, no members) and the end
    /// record.
    pub(crate) fn tiny_metafile() -> Vec<u8> {
        let mut out = Vec::new();
        for word in [1u16, 9, 0x300, 0, 0, 0, 3, 0, 0] {
            out.extend(word.to_le_bytes());
        }
        out.extend([3, 0, 0, 0, 0, 0]);
        let words = (out.len() / 2) as u32;
        out[6..10].copy_from_slice(&words.to_le_bytes());
        out
    }

    fn string(out: &mut Vec<u8>, text: &str) {
        if text.is_empty() {
            out.extend(0u32.to_le_bytes());
        } else {
            out.extend((text.len() as u32 + 1).to_le_bytes());
            out.extend(text.as_bytes());
            out.push(0);
        }
    }

    fn presentation_bytes(out: &mut Vec<u8>, metafile: &[u8], wide_header: bool) {
        out.extend(VERSION.to_le_bytes());
        out.extend(5u32.to_le_bytes());
        string(out, "METAFILEPICT");
        out.extend(2000u32.to_le_bytes());
        out.extend(1000u32.to_le_bytes());
        let mut data = Vec::new();
        data.extend(8u16.to_le_bytes());
        data.extend(4000u16.to_le_bytes());
        data.extend(2000u16.to_le_bytes());
        data.extend(0u16.to_le_bytes());
        if wide_header {
            data.extend([0; 8]);
        }
        data.extend(metafile);
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
    }

    pub(crate) fn embedded(class: &str, native: &[u8], trailing: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(VERSION.to_le_bytes());
        out.extend(2u32.to_le_bytes());
        string(&mut out, class);
        string(&mut out, "");
        string(&mut out, "");
        out.extend((native.len() as u32).to_le_bytes());
        out.extend(native);
        presentation_bytes(&mut out, &tiny_metafile(), false);
        out.extend(trailing);
        out
    }

    #[test]
    fn an_embedded_object_gives_its_class_native_data_and_metafile() {
        let stream = embedded("MSWordArt.2", &[1, 2, 3], &[]);
        let object = read(&stream).unwrap();
        assert_eq!(object.kind, Kind::Embedded);
        assert_eq!(object.class, "MSWordArt.2");
        assert_eq!(object.label(), "WordArt");
        assert_eq!(object.native_len(), 3);
        assert_eq!(object.trailing, 0);
        assert!(object.problems.is_empty(), "{:?}", object.problems);
        let presentation = object.presentation.as_ref().unwrap();
        assert_eq!(presentation.format, "METAFILEPICT");
        assert_eq!((presentation.width, presentation.height), (2000, 1000));
        assert_eq!(presentation.mapping_mode, Some(8));
        assert_eq!(presentation.extent, Some((4000, 2000)));
        assert_eq!(object.metafile(&stream).unwrap(), tiny_metafile());
    }

    #[test]
    fn trailing_bytes_are_counted_and_the_other_classes_are_labelled() {
        let stream = embedded("123Worksheet", &[], &[9; 160]);
        let object = read(&stream).unwrap();
        assert_eq!(object.trailing, 160);
        assert_eq!(object.label(), "1-2-3 Worksheet");
        let clip = read(&embedded("MS_ClipArt_Gallery", &[0; 5], &[])).unwrap();
        assert_eq!(clip.label(), "ClipArt Gallery");
        let other = read(&embedded("Package", &[], &[])).unwrap();
        assert_eq!(other.label(), "Package");
    }

    #[test]
    fn a_sixteen_byte_metafile_header_is_found() {
        let mut stream = Vec::new();
        stream.extend(VERSION.to_le_bytes());
        stream.extend(3u32.to_le_bytes());
        string(&mut stream, "Static");
        presentation_bytes(&mut stream, &tiny_metafile(), true);
        let object = read(&stream).unwrap();
        assert_eq!(object.kind, Kind::Static);
        assert_eq!(object.metafile(&stream).unwrap(), tiny_metafile());
    }

    #[test]
    fn a_linked_object_reads_its_names_and_skips_the_network_data() {
        let mut stream = Vec::new();
        stream.extend(VERSION.to_le_bytes());
        stream.extend(1u32.to_le_bytes());
        string(&mut stream, "123Worksheet");
        string(&mut stream, "C:\\SHEET.WK4");
        string(&mut stream, "A:A1..B2");
        string(&mut stream, "");
        stream.extend([0; 16]);
        presentation_bytes(&mut stream, &tiny_metafile(), false);
        let object = read(&stream).unwrap();
        assert_eq!(object.kind, Kind::Linked);
        assert_eq!(object.topic, "C:\\SHEET.WK4");
        assert_eq!(object.item, "A:A1..B2");
        assert!(object.native.is_none());
        assert!(object.metafile(&stream).is_some());
    }

    #[test]
    fn what_is_not_an_object_stream_is_refused() {
        assert!(read(&[]).is_none());
        assert!(read(&[1, 5, 0]).is_none());
        let mut wrong = embedded("X", &[], &[]);
        wrong[0] = 9;
        assert!(read(&wrong).is_none());
        let mut format = embedded("X", &[], &[]);
        format[4] = 7;
        assert!(read(&format).is_none());
    }

    #[test]
    fn every_truncation_reads_without_panicking_and_names_the_problem() {
        let stream = embedded("MSWordArt.2", &[7; 20], &[1, 2, 3]);
        for length in 0..stream.len() {
            let cut = &stream[..length];
            if let Some(object) = read(cut) {
                if let Some(presentation) = &object.presentation {
                    assert!(presentation.data.0 + presentation.data.1 <= cut.len());
                    if let Some(metafile) = object.metafile(cut) {
                        assert!(!metafile.is_empty());
                    }
                } else {
                    assert!(!object.problems.is_empty(), "length {length}");
                }
            }
        }
    }

    #[test]
    fn lengths_past_the_stream_are_problems_not_panics() {
        let mut stream = embedded("A", &[0; 4], &[]);
        // Class length 0xffffffff.
        stream[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(read(&stream).is_none());
        // Native size past the end.
        let mut stream = embedded("A", &[0; 4], &[]);
        let at = 12 + 2 + 4 + 4;
        stream[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let object = read(&stream).unwrap();
        assert!(object.presentation.is_none());
        assert_eq!(object.problems, ["the native data runs past the stream"]);
        // Presentation data size past the end.
        let mut stream = embedded("A", &[], &[]);
        let at = stream.len() - tiny_metafile().len() - 8 - 4;
        stream[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let object = read(&stream).unwrap();
        assert!(object.presentation.is_none());
        assert!(!object.problems.is_empty());
    }

    #[test]
    fn a_presentation_without_a_metafile_is_named() {
        let mut stream = embedded("A", &[], &[]);
        // Spoil the metafile header's signature.
        let at = stream.len() - tiny_metafile().len();
        stream[at] = 0x77;
        let object = read(&stream).unwrap();
        assert!(object.metafile(&stream).is_none());
        assert_eq!(object.problems, ["the presentation holds no metafile"]);
    }
}
