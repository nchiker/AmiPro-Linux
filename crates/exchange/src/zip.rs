//! A zip archive writer for the formats that are zips of parts (DOCX, ODT):
//! entries stored as they are or compressed by the caller's deflate, with
//! their CRC-32 (APPNOTE 6.3.9). No dependencies: the app passes `flate2`'s
//! raw deflate.

/// A raw deflate compressor, or `None` to store every entry.
pub type Deflate<'a> = Option<&'a dyn Fn(&[u8]) -> Vec<u8>>;

/// One file in the archive.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub bytes: Vec<u8>,
    /// Never compressed (ODT's `mimetype`, which must come first stored).
    pub stored: bool,
}

impl Entry {
    pub fn new(name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            bytes: bytes.into(),
            stored: false,
        }
    }

    pub fn stored(name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            stored: true,
            ..Self::new(name, bytes)
        }
    }
}

/// The archive of `entries`, in their order, dated 1 January 1980 (zip's
/// first date, so that the same document gives the same bytes).
pub fn write(entries: &[Entry], deflate: Deflate) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let crc = crc32(&entry.bytes);
        let compressed = match deflate {
            Some(deflate) if !entry.stored && !entry.bytes.is_empty() => {
                let packed = deflate(&entry.bytes);
                (packed.len() < entry.bytes.len()).then_some(packed)
            }
            _ => None,
        };
        let (method, data): (u16, &[u8]) = match &compressed {
            Some(packed) => (8, packed),
            None => (0, &entry.bytes),
        };
        let offset = out.len() as u32;
        let name = entry.name.as_bytes();
        // Local file header.
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 names
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&0x0021u16.to_le_bytes()); // 1980-01-01
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(entry.bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        // Its central directory record.
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0x0021u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(entry.bytes.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 8]); // extra, comment, disk, internal
        central.extend_from_slice(&0u32.to_le_bytes()); // external attributes
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// The CRC-32 zip uses (polynomial 0xEDB88320).
pub fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut table = [0u32; 256];
        for (index, slot) in table.iter_mut().enumerate() {
            let mut value = index as u32;
            for _ in 0..8 {
                value = if value & 1 != 0 {
                    0xEDB8_8320 ^ (value >> 1)
                } else {
                    value >> 1
                };
            }
            *slot = value;
        }
        table
    });
    !bytes.iter().fold(!0u32, |crc, byte| {
        table[((crc ^ u32::from(*byte)) & 0xFF) as usize] ^ (crc >> 8)
    })
}

/// A raw inflater: the deflated bytes and the size they inflate to, or
/// `None` when they do not inflate to exactly that many bytes. The app
/// passes `flate2`'s; it must never produce more than the size given.
pub type Inflate<'a> = &'a dyn Fn(&[u8], usize) -> Option<Vec<u8>>;

/// The most entries an archive may list.
pub const MAX_ENTRIES: usize = 50_000;
/// The largest an entry may inflate to.
pub const MAX_ENTRY_SIZE: usize = 256 << 20;

/// Why an archive or one of its entries cannot be read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZipError {
    /// No end of central directory: not a zip, or cut short.
    NotAZip,
    /// A record lies outside the file or contradicts another.
    Damaged(&'static str),
    /// Encrypted entries (a password-protected package).
    Encrypted,
    /// A compression method other than stored and deflated.
    Method(u16),
    /// More entries, or a larger entry, than the limits allow.
    TooLarge,
    /// The entry's data does not inflate to its size.
    Inflate,
    /// Its CRC-32 differs from the recorded one.
    Checksum,
    NotFound,
}

impl std::fmt::Display for ZipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAZip => write!(f, "not a zip package"),
            Self::Damaged(what) => write!(f, "the package is damaged ({what})"),
            Self::Encrypted => write!(f, "the package is password-protected"),
            Self::Method(method) => write!(f, "compression method {method} is not supported"),
            Self::TooLarge => write!(f, "the package is too large"),
            Self::Inflate => write!(f, "a part of the package is damaged"),
            Self::Checksum => write!(f, "a part of the package fails its checksum"),
            Self::NotFound => write!(f, "a part is missing"),
        }
    }
}

/// One entry of the central directory.
#[derive(Clone, Debug)]
struct Listed {
    name: String,
    method: u16,
    flags: u16,
    crc: u32,
    compressed: u64,
    size: u64,
    offset: u64,
}

/// A zip archive read through its central directory (APPNOTE 6.3.9), so
/// that entries written with data descriptors (sizes after the data, as
/// LibreOffice and many others write them) are found.
#[derive(Clone, Debug)]
pub struct Archive<'a> {
    bytes: &'a [u8],
    entries: Vec<Listed>,
}

fn word(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn long(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn quad(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

impl<'a> Archive<'a> {
    /// Reads the central directory of `bytes`.
    pub fn open(bytes: &'a [u8]) -> Result<Self, ZipError> {
        // The end of central directory record: 22 bytes and a comment of up
        // to 64 KiB, so it starts in the last 65,557 bytes.
        let earliest = bytes.len().saturating_sub(22 + 0xFFFF);
        let end = (earliest..=bytes.len().saturating_sub(22))
            .rev()
            .find(|&at| long(bytes, at) == Some(0x0605_4b50))
            .ok_or(ZipError::NotAZip)?;
        let mut count = u64::from(word(bytes, end + 10).ok_or(ZipError::NotAZip)?);
        let mut start = u64::from(long(bytes, end + 16).ok_or(ZipError::NotAZip)?);
        // Zip64: a locator 20 bytes before points at the larger record.
        if (count == 0xFFFF || start == 0xFFFF_FFFF)
            && end >= 20
            && long(bytes, end - 20) == Some(0x0706_4b50)
        {
            let record = quad(bytes, end - 12).ok_or(ZipError::Damaged("zip64 locator"))?;
            let record = usize::try_from(record).map_err(|_| ZipError::TooLarge)?;
            if long(bytes, record) != Some(0x0606_4b50) {
                return Err(ZipError::Damaged("zip64 end record"));
            }
            count = quad(bytes, record + 32).ok_or(ZipError::Damaged("zip64 end record"))?;
            start = quad(bytes, record + 48).ok_or(ZipError::Damaged("zip64 end record"))?;
        }
        if count > MAX_ENTRIES as u64 {
            return Err(ZipError::TooLarge);
        }
        let mut at = usize::try_from(start).map_err(|_| ZipError::TooLarge)?;
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let damaged = ZipError::Damaged("central directory");
            if long(bytes, at) != Some(0x0201_4b50) {
                return Err(damaged);
            }
            let flags = word(bytes, at + 8).ok_or(damaged.clone())?;
            let method = word(bytes, at + 10).ok_or(damaged.clone())?;
            let crc = long(bytes, at + 16).ok_or(damaged.clone())?;
            let mut compressed = u64::from(long(bytes, at + 20).ok_or(damaged.clone())?);
            let mut size = u64::from(long(bytes, at + 24).ok_or(damaged.clone())?);
            let name_length = usize::from(word(bytes, at + 28).ok_or(damaged.clone())?);
            let extra_length = usize::from(word(bytes, at + 30).ok_or(damaged.clone())?);
            let comment_length = usize::from(word(bytes, at + 32).ok_or(damaged.clone())?);
            let mut offset = u64::from(long(bytes, at + 42).ok_or(damaged.clone())?);
            let name_bytes = bytes
                .get(at + 46..at + 46 + name_length)
                .ok_or(damaged.clone())?;
            // Names are UTF-8 when flagged, else code page 437; the formats'
            // part names are ASCII either way.
            let name = String::from_utf8_lossy(name_bytes).into_owned();
            let extra = bytes
                .get(at + 46 + name_length..at + 46 + name_length + extra_length)
                .ok_or(damaged.clone())?;
            // The zip64 extra field holds, in order, whichever of the sizes
            // and the offset were too large for their place.
            let mut field = 0;
            while field + 4 <= extra.len() {
                let id = word(extra, field).unwrap_or(0);
                let length = usize::from(word(extra, field + 2).unwrap_or(0));
                if id == 1 {
                    let mut value = field + 4;
                    for target in [&mut size, &mut compressed, &mut offset] {
                        if *target == 0xFFFF_FFFF {
                            *target = quad(extra, value).ok_or(damaged.clone())?;
                            value += 8;
                        }
                    }
                }
                field += 4 + length;
            }
            entries.push(Listed {
                name,
                method,
                flags,
                crc,
                compressed,
                size,
                offset,
            });
            at += 46 + name_length + extra_length + comment_length;
        }
        Ok(Self { bytes, entries })
    }

    /// The entries' names, in the directory's order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|entry| entry.name.as_str())
    }

    /// The entry named `name`, or else one whose name differs only in case
    /// (package part names are compared without case).
    fn find(&self, name: &str) -> Option<&Listed> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(name))
            })
    }

    pub fn contains(&self, name: &str) -> bool {
        self.find(name).is_some()
    }

    /// The bytes of the entry `name`, inflated by `inflate`.
    pub fn read(&self, name: &str, inflate: Inflate) -> Result<Vec<u8>, ZipError> {
        let entry = self.find(name).ok_or(ZipError::NotFound)?;
        if entry.flags & 1 != 0 {
            return Err(ZipError::Encrypted);
        }
        let size = usize::try_from(entry.size).map_err(|_| ZipError::TooLarge)?;
        if size > MAX_ENTRY_SIZE {
            return Err(ZipError::TooLarge);
        }
        let damaged = ZipError::Damaged("local header");
        let at = usize::try_from(entry.offset).map_err(|_| damaged.clone())?;
        if long(self.bytes, at) != Some(0x0403_4b50) {
            return Err(damaged);
        }
        // The local header's own name and extra lengths: its extra field may
        // differ from the central one.
        let name_length = usize::from(word(self.bytes, at + 26).ok_or(damaged.clone())?);
        let extra_length = usize::from(word(self.bytes, at + 28).ok_or(damaged.clone())?);
        let data = at + 30 + name_length + extra_length;
        let compressed = usize::try_from(entry.compressed).map_err(|_| damaged.clone())?;
        let packed = self
            .bytes
            .get(data..data.checked_add(compressed).ok_or(damaged.clone())?)
            .ok_or(ZipError::Damaged("entry data"))?;
        let bytes = match entry.method {
            0 if compressed == size => packed.to_vec(),
            0 => return Err(ZipError::Damaged("stored entry size")),
            8 => inflate(packed, size).ok_or(ZipError::Inflate)?,
            method => return Err(ZipError::Method(method)),
        };
        if bytes.len() != size {
            return Err(ZipError::Inflate);
        }
        if crc32(&bytes) != entry.crc {
            return Err(ZipError::Checksum);
        }
        Ok(bytes)
    }
}

/// The entries of an archive this writer wrote with stored entries, for
/// tests: names and bytes.
pub fn read_stored(archive: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut at = 0;
    while archive.get(at..at + 4) == Some(&0x0403_4b50u32.to_le_bytes()[..]) {
        let word =
            |offset: usize| u16::from_le_bytes([archive[at + offset], archive[at + offset + 1]]);
        let long = |offset: usize| {
            u32::from_le_bytes(archive[at + offset..at + offset + 4].try_into().unwrap())
        };
        let method = word(8);
        let size = long(18) as usize;
        let name_length = usize::from(word(26));
        let extra = usize::from(word(28));
        let name = String::from_utf8_lossy(&archive[at + 30..at + 30 + name_length]).into_owned();
        let data = at + 30 + name_length + extra;
        if method == 0 {
            out.push((name, archive[data..data + size].to_vec()));
        }
        at = data + size;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_is_zips() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    fn no_inflate(_: &[u8], _: usize) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn the_central_directory_finds_every_entry() {
        let archive = write(
            &[
                Entry::stored("mimetype", "x"),
                Entry::new("a/b.xml", "<a/>"),
            ],
            None,
        );
        let read = Archive::open(&archive).unwrap();
        assert_eq!(read.names().collect::<Vec<_>>(), ["mimetype", "a/b.xml"]);
        assert_eq!(read.read("A/B.XML", &no_inflate).unwrap(), b"<a/>");
        assert_eq!(read.read("c", &no_inflate), Err(ZipError::NotFound));
    }

    #[test]
    fn damaged_archives_are_refused_without_a_panic() {
        let archive = write(&[Entry::new("a.xml", "<a>text</a>")], None);
        for cut in 0..archive.len() {
            if let Ok(read) = Archive::open(&archive[..cut]) {
                let _ = read.read("a.xml", &no_inflate);
            }
        }
        let mut flipped = archive.clone();
        let data = flipped.len() / 3;
        flipped[data] ^= 0x55;
        let read = Archive::open(&flipped).unwrap();
        assert!(read.read("a.xml", &no_inflate).is_err());
        assert_eq!(Archive::open(b"PK").err(), Some(ZipError::NotAZip));
    }

    #[test]
    fn stored_entries_read_back() {
        let archive = write(
            &[
                Entry::stored("mimetype", "x"),
                Entry::new("a/b.xml", "<a/>"),
            ],
            None,
        );
        assert_eq!(
            read_stored(&archive),
            vec![
                ("mimetype".into(), b"x".to_vec()),
                ("a/b.xml".into(), b"<a/>".to_vec())
            ]
        );
    }
}
