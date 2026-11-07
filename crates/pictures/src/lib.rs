//! Readers of the pictures Ami Pro stores (`docs/research/PICTURE_NOTES.md`,
//! `docs/Design/UI_SPEC.md` §25), written for this project from the
//! published formats and the behaviour of Ami Pro's drivers, without
//! dependencies:
//!
//! - [`dib`]: Windows bitmaps (`AmiPaint`): decoded to [`Raster`], their
//!   natural size as Ami Pro gives it, and a writer of 24-bit bitmaps for
//!   pictures Save As converts (decision P8).
//! - [`wmf`]: Windows metafiles (`AmiWMF`): played into a [`wmf::Drawing`],
//!   a display list in the metafile window's own proportions, which the app
//!   draws at any size, so metafiles stay sharp at every zoom.
//! - [`emf`]: enhanced metafiles (Word's and LibreOffice's drawings, in
//!   imported documents), played into the same display list.
//! - [`ole`]: OLE 1.0 object streams (`AmiOLE`): the class, the native data,
//!   and the presentation metafile the object is drawn from.
//! - [`screen`]: Ami Pro's own screen image of a picture (`1168:2016`),
//!   drawn for kinds no reader takes yet (decision P4).
//!
//! Every reader checks its input's sizes and returns `None` for what it
//! cannot read; none panics on malformed bytes.

pub mod adjust;
pub mod chart;
pub mod chart_gen;
pub mod dib;
pub mod emf;
pub mod equation;
pub mod ole;
pub mod screen;
pub mod sdw;
pub mod svg;
pub mod wmf;

/// The most pixels a reader decodes: 50 million, 200 MB as RGBA.
pub const MAX_PIXELS: u64 = 50_000_000;

/// A decoded picture: RGBA, eight bits a channel, alpha not premultiplied,
/// rows from the top.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Raster {
    /// A raster of `width` by `height` pixels, all `rgba`, or `None` past
    /// [`MAX_PIXELS`].
    pub fn filled(width: u32, height: u32, rgba: [u8; 4]) -> Option<Self> {
        let count = u64::from(width) * u64::from(height);
        if width == 0 || height == 0 || count > MAX_PIXELS {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels: rgba.repeat(count as usize),
        })
    }

    pub fn pixel(&self, x: u32, y: u32) -> [u8; 4] {
        let at = (y as usize * self.width as usize + x as usize) * 4;
        [
            self.pixels[at],
            self.pixels[at + 1],
            self.pixels[at + 2],
            self.pixels[at + 3],
        ]
    }

    fn put(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        let at = (y as usize * self.width as usize + x as usize) * 4;
        self.pixels[at..at + 4].copy_from_slice(&rgba);
    }
}

/// A size in twips, as Ami Pro measures pictures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Twips {
    pub width: i32,
    pub height: i32,
}

/// Little-endian reads that fail instead of panicking.
pub(crate) struct Bytes<'a>(pub &'a [u8]);

impl Bytes<'_> {
    pub fn u8(&self, at: usize) -> Option<u8> {
        self.0.get(at).copied()
    }

    pub fn u16(&self, at: usize) -> Option<u16> {
        let bytes = self.0.get(at..at.checked_add(2)?)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn i16(&self, at: usize) -> Option<i16> {
        self.u16(at).map(|value| value as i16)
    }

    pub fn u32(&self, at: usize) -> Option<u32> {
        let bytes = self.0.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn i32(&self, at: usize) -> Option<i32> {
        self.u32(at).map(|value| value as i32)
    }
}
