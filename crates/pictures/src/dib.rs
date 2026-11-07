//! Windows bitmaps (Ami Pro's `AmiPaint` driver; the published BMP and DIB
//! formats): the core and info headers and their later versions; 1, 4, 8,
//! 16, 24, and 32 bits a pixel; RLE8, RLE4, and bit-field compression;
//! rows from the bottom or the top. Also the natural size `AmiPaint` gives
//! a bitmap, and a writer of 24-bit bitmaps.

use crate::{Bytes, MAX_PIXELS, Raster, Twips};

/// A colour as red, green, blue.
pub type Rgb = [u8; 3];

/// How a packed bitmap's colour table is to be read (`ColorUsage` in a
/// metafile's bitmap records).
#[derive(Clone, Copy, Debug)]
pub enum ColourTable<'a> {
    /// Colours, as in every bitmap file.
    Rgb,
    /// Indices into this logical palette (`DIB_PAL_COLORS`).
    Palette(&'a [Rgb]),
}

/// The fields of a bitmap header the readers use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Header {
    pub width: u32,
    pub height: u32,
    pub top_down: bool,
    pub bit_count: u16,
    pub compression: u32,
    /// Pixels per metre, 0 when not given.
    pub x_ppm: i32,
    pub y_ppm: i32,
    /// Where the colour table starts and how many entries it has.
    table_at: usize,
    colours: usize,
    /// Bytes per colour table entry: 3 in core headers, else 4.
    entry: usize,
    masks: Option<[u32; 3]>,
}

const RLE8: u32 = 1;
const RLE4: u32 = 2;
const BITFIELDS: u32 = 3;

impl Header {
    /// Reads a `BITMAPCOREHEADER` (12 bytes) or a `BITMAPINFOHEADER` and
    /// its later versions (40, 52, 56, 64, 108, 124 bytes).
    pub fn read(info: &[u8]) -> Option<Self> {
        let bytes = Bytes(info);
        let size = bytes.u32(0)? as usize;
        if size == 12 {
            let bit_count = bytes.u16(10)?;
            return Self::finish(Header {
                width: u32::from(bytes.u16(4)?),
                height: u32::from(bytes.u16(6)?),
                top_down: false,
                bit_count,
                compression: 0,
                x_ppm: 0,
                y_ppm: 0,
                table_at: 12,
                colours: if bit_count <= 8 { 1 << bit_count } else { 0 },
                entry: 3,
                masks: None,
            });
        }
        if !(40..=124).contains(&size) {
            return None;
        }
        let width = bytes.i32(4)?;
        let height = bytes.i32(8)?;
        let bit_count = bytes.u16(14)?;
        let compression = bytes.u32(16)?;
        let used = bytes.u32(32)? as usize;
        let mut table_at = size;
        let masks = if compression == BITFIELDS {
            if size >= 52 {
                Some([bytes.u32(40)?, bytes.u32(44)?, bytes.u32(48)?])
            } else {
                table_at += 12;
                Some([bytes.u32(40)?, bytes.u32(44)?, bytes.u32(48)?])
            }
        } else {
            None
        };
        let colours = match bit_count {
            1 | 4 | 8 if used == 0 => 1 << bit_count,
            1 | 4 | 8 => used.min(1 << bit_count),
            // Bitmaps of more bits may carry a table for palettes; skip it.
            _ => used.min(256),
        };
        if width <= 0 || height == 0 || height == i32::MIN {
            return None;
        }
        Self::finish(Header {
            width: width as u32,
            height: height.unsigned_abs(),
            top_down: height < 0,
            bit_count,
            compression,
            x_ppm: bytes.i32(24)?,
            y_ppm: bytes.i32(28)?,
            table_at,
            colours,
            entry: 4,
            masks,
        })
    }

    fn finish(header: Header) -> Option<Header> {
        let pixels = u64::from(header.width) * u64::from(header.height);
        (header.width > 0
            && header.height > 0
            && pixels <= MAX_PIXELS
            && matches!(header.bit_count, 1 | 4 | 8 | 16 | 24 | 32))
        .then_some(header)
    }

    /// Bytes per row of uncompressed bits, padded to four.
    fn stride(&self) -> usize {
        (self.width as usize * usize::from(self.bit_count)).div_ceil(32) * 4
    }

    /// The size Ami Pro's bitmap driver gives the picture
    /// ([`raster_natural_size`]).
    pub fn natural_size(&self) -> Twips {
        let ppm = (self.x_ppm > 0 && self.y_ppm > 0).then_some((self.x_ppm, self.y_ppm));
        raster_natural_size(self.width, self.height, ppm)
    }
}

/// The size Ami Pro's bitmap driver gives a picture of `width` by `height`
/// pixels (`AMIPAINT.DLL`, read when the bitmap is loaded): at its stored
/// resolution in pixels a metre, its pixels × 56,693 twips a metre ÷
/// pixels a metre, both sides halved together until neither passes
/// 32,767; without one, at the screen's 96 dpi, lowered so that neither
/// side is more than 21 inches (PI1: 120 × 80 pixels without a resolution
/// are 1,800 × 1,200 twips). The app sizes every raster picture so.
pub fn raster_natural_size(width: u32, height: u32, ppm: Option<(i32, i32)>) -> Twips {
    let (width, height) = (i64::from(width), i64::from(height));
    if let Some((x_ppm, y_ppm)) = ppm.filter(|(x, y)| *x > 0 && *y > 0) {
        let mut twips_wide = width * 56_693 / i64::from(x_ppm);
        let mut twips_high = height * 56_693 / i64::from(y_ppm);
        while twips_wide > 0x7fff || twips_high > 0x7fff {
            twips_wide /= 2;
            twips_high /= 2;
        }
        return Twips {
            width: twips_wide as i32,
            height: twips_high as i32,
        };
    }
    let (mut dpi_x, mut dpi_y) = (96_i64, 96_i64);
    if width / dpi_x > 20 {
        dpi_y = dpi_y * width / (dpi_x * 21);
        dpi_x = width / 21;
    }
    if height / dpi_y > 20 {
        dpi_x = dpi_x * height / (dpi_y * 21);
        dpi_y = height / 21;
    }
    Twips {
        width: (width * 1_440 / dpi_x.max(1)) as i32,
        height: (height * 1_440 / dpi_y.max(1)) as i32,
    }
}

/// A bitmap file (`BM` and its 14-byte file header).
pub fn decode_bmp(file: &[u8]) -> Option<Raster> {
    let (header, info, bits) = split_file(file)?;
    decode(&header, info, bits, ColourTable::Rgb)
}

/// A bitmap file's header, its `BITMAPINFO`, and its bits.
fn split_file(file: &[u8]) -> Option<(Header, &[u8], &[u8])> {
    if file.get(..2)? != b"BM" {
        return None;
    }
    let bits_at = Bytes(file).u32(10)? as usize;
    let info = file.get(14..)?;
    let header = Header::read(info)?;
    Some((header, info, file.get(bits_at..)?))
}

/// A bitmap file's header, for its natural size without decoding it.
pub fn bmp_header(file: &[u8]) -> Option<Header> {
    split_file(file).map(|(header, _, _)| header)
}

/// A packed bitmap, as metafile records hold one: the `BITMAPINFO` and the
/// bits right after its colour table.
pub fn decode_packed(dib: &[u8], table: ColourTable<'_>) -> Option<Raster> {
    let header = Header::read(dib)?;
    let entry = match table {
        ColourTable::Rgb => header.entry,
        ColourTable::Palette(_) => 2,
    };
    let bits_at = header
        .table_at
        .checked_add(header.colours.checked_mul(entry)?)?;
    decode(&header, dib, dib.get(bits_at..)?, table)
}

/// The colour table as colours.
fn palette(header: &Header, info: &[u8], table: ColourTable<'_>) -> Option<Vec<Rgb>> {
    let bytes = Bytes(info);
    (0..header.colours)
        .map(|index| match table {
            ColourTable::Rgb => {
                let at = header.table_at + index * header.entry;
                Some([bytes.u8(at + 2)?, bytes.u8(at + 1)?, bytes.u8(at)?])
            }
            ColourTable::Palette(logical) => {
                let at = header.table_at + index * 2;
                let index = usize::from(bytes.u16(at)?);
                Some(logical.get(index).copied().unwrap_or([0, 0, 0]))
            }
        })
        .collect()
}

fn decode(header: &Header, info: &[u8], bits: &[u8], table: ColourTable<'_>) -> Option<Raster> {
    let colours = palette(header, info, table)?;
    let mut raster = Raster::filled(header.width, header.height, [0, 0, 0, 0])?;
    // Row `row` of the bits, counted as stored, is this raster row.
    let target = |row: u32| {
        if header.top_down {
            row
        } else {
            header.height - 1 - row
        }
    };
    let colour = |index: usize| {
        let [r, g, b] = colours.get(index).copied().unwrap_or([0, 0, 0]);
        [r, g, b, 255]
    };
    match header.compression {
        RLE8 | RLE4 if !header.top_down => {
            run_lengths(header, bits, &mut raster, &colour)?;
            return Some(raster);
        }
        0 | BITFIELDS => {}
        _ => return None,
    }
    let stride = header.stride();
    if bits.len() < stride.checked_mul(header.height as usize)? {
        return None;
    }
    let masks = header.masks.unwrap_or(match header.bit_count {
        16 => [0x7c00, 0x03e0, 0x001f],
        _ => [0xff_0000, 0xff00, 0xff],
    });
    for row in 0..header.height {
        let line = &bits[row as usize * stride..][..stride];
        let y = target(row);
        for x in 0..header.width {
            let at = x as usize;
            let rgba = match header.bit_count {
                1 => colour(usize::from(line[at / 8] >> (7 - at % 8) & 1)),
                4 => colour(usize::from(line[at / 2] >> (4 * (1 - at % 2)) & 0xf)),
                8 => colour(usize::from(line[at])),
                16 => {
                    let value = u32::from(u16::from_le_bytes([line[2 * at], line[2 * at + 1]]));
                    masked(value, masks)
                }
                24 => [line[3 * at + 2], line[3 * at + 1], line[3 * at], 255],
                _ => {
                    let value = u32::from_le_bytes([
                        line[4 * at],
                        line[4 * at + 1],
                        line[4 * at + 2],
                        line[4 * at + 3],
                    ]);
                    masked(value, masks)
                }
            };
            raster.put(x, y, rgba);
        }
    }
    Some(raster)
}

/// A pixel by its bit-field masks, each channel scaled to eight bits.
fn masked(value: u32, masks: [u32; 3]) -> [u8; 4] {
    let channel = |mask: u32| {
        if mask == 0 {
            return 0;
        }
        let shift = mask.trailing_zeros();
        let max = mask >> shift;
        (((value & mask) >> shift) * 255 / max) as u8
    };
    [channel(masks[0]), channel(masks[1]), channel(masks[2]), 255]
}

/// RLE8 and RLE4 bits: runs, then escapes (end of line, end of bitmap, a
/// move, or pixels as they are, padded to a word); rows from the bottom.
/// Pixels a move skips stay transparent.
fn run_lengths(
    header: &Header,
    bits: &[u8],
    raster: &mut Raster,
    colour: &dyn Fn(usize) -> [u8; 4],
) -> Option<()> {
    let four = header.compression == RLE4;
    let (mut x, mut row) = (0u32, 0u32);
    let mut at = 0usize;
    let put = |x: u32, row: u32, index: usize, raster: &mut Raster| {
        if x < header.width && row < header.height {
            raster.put(x, header.height - 1 - row, colour(index));
        }
    };
    while at + 1 < bits.len() {
        let (count, value) = (bits[at], bits[at + 1]);
        at += 2;
        if count > 0 {
            for n in 0..u32::from(count) {
                let index = if four {
                    if n % 2 == 0 { value >> 4 } else { value & 0xf }
                } else {
                    value
                };
                put(x + n, row, usize::from(index), raster);
            }
            x += u32::from(count);
            continue;
        }
        match value {
            0 => {
                x = 0;
                row += 1;
            }
            1 => return Some(()),
            2 => {
                x += u32::from(*bits.get(at)?);
                row += u32::from(*bits.get(at + 1)?);
                at += 2;
            }
            n => {
                let n = usize::from(n);
                let length = if four { n.div_ceil(2) } else { n };
                let run = bits.get(at..at + length)?;
                for i in 0..n {
                    let index = if four {
                        if i % 2 == 0 {
                            run[i / 2] >> 4
                        } else {
                            run[i / 2] & 0xf
                        }
                    } else {
                        run[i]
                    };
                    put(x + i as u32, row, usize::from(index), raster);
                }
                x += n as u32;
                at += length + length % 2;
            }
        }
        if row >= header.height {
            break;
        }
    }
    Some(())
}

/// A 24-bit bitmap file of `raster` at `ppm` pixels a metre, its
/// transparency laid on white, as Save As writes pictures Ami Pro cannot
/// store (P8).
pub fn encode_bmp24(raster: &Raster, ppm: (i32, i32)) -> Vec<u8> {
    let stride = (raster.width as usize * 3).div_ceil(4) * 4;
    let image = stride * raster.height as usize;
    let mut out = Vec::with_capacity(54 + image);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((54 + image) as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(raster.width as i32).to_le_bytes());
    out.extend_from_slice(&(raster.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(image as u32).to_le_bytes());
    out.extend_from_slice(&ppm.0.to_le_bytes());
    out.extend_from_slice(&ppm.1.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    for y in (0..raster.height).rev() {
        let start = out.len();
        for x in 0..raster.width {
            let [r, g, b, a] = raster.pixel(x, y);
            let over = |channel: u8| {
                ((u32::from(channel) * u32::from(a) + 255 * (255 - u32::from(a))) / 255) as u8
            };
            out.extend_from_slice(&[over(b), over(g), over(r)]);
        }
        out.resize(start + stride, 0);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bitmap file of `width` × `height` with `bits` per pixel, the given
    /// colour table, compression, resolution, and pixel bytes.
    pub(crate) fn bmp(
        width: i32,
        height: i32,
        bit_count: u16,
        compression: u32,
        ppm: i32,
        table: &[[u8; 4]],
        bits: &[u8],
    ) -> Vec<u8> {
        let at = 54 + 4 * table.len();
        let mut out = b"BM".to_vec();
        out.extend_from_slice(&((at + bits.len()) as u32).to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(at as u32).to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&width.to_le_bytes());
        out.extend_from_slice(&height.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&bit_count.to_le_bytes());
        out.extend_from_slice(&compression.to_le_bytes());
        out.extend_from_slice(&(bits.len() as u32).to_le_bytes());
        out.extend_from_slice(&ppm.to_le_bytes());
        out.extend_from_slice(&ppm.to_le_bytes());
        out.extend_from_slice(&(table.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        for entry in table {
            out.extend_from_slice(entry);
        }
        out.extend_from_slice(bits);
        out
    }

    const RED: [u8; 4] = [0, 0, 255, 0];
    const BLUE: [u8; 4] = [255, 0, 0, 0];

    #[test]
    fn bitmaps_of_every_depth_decode_bottom_up() {
        // 2 × 2, 1 bit: bottom row red then blue, top row blue then red.
        let file = bmp(
            2,
            2,
            1,
            0,
            0,
            &[RED, BLUE],
            &[0b0100_0000, 0, 0, 0, 0b1000_0000, 0, 0, 0],
        );
        let raster = decode_bmp(&file).unwrap();
        assert_eq!(raster.pixel(0, 1), [255, 0, 0, 255]);
        assert_eq!(raster.pixel(1, 1), [0, 0, 255, 255]);
        assert_eq!(raster.pixel(0, 0), [0, 0, 255, 255]);
        // 4 bits, 8 bits, 24 bits, one row.
        let file = bmp(3, 1, 4, 0, 0, &[RED, BLUE], &[0x01, 0x00, 0, 0]);
        let raster = decode_bmp(&file).unwrap();
        assert_eq!(raster.pixel(1, 0), [0, 0, 255, 255]);
        let file = bmp(1, 1, 8, 0, 0, &[RED, BLUE], &[1, 0, 0, 0]);
        assert_eq!(decode_bmp(&file).unwrap().pixel(0, 0), [0, 0, 255, 255]);
        let file = bmp(1, -1, 24, 0, 0, &[], &[1, 2, 3, 0]);
        assert_eq!(decode_bmp(&file).unwrap().pixel(0, 0), [3, 2, 1, 255]);
        // 16 bits: 5-5-5 by default.
        let file = bmp(1, 1, 16, 0, 0, &[], &[0x00, 0x7c, 0, 0]);
        assert_eq!(decode_bmp(&file).unwrap().pixel(0, 0), [255, 0, 0, 255]);
    }

    #[test]
    fn run_lengths_decode_rle8_and_rle4() {
        // RLE8, 4 × 2: a run of three of colour 1, end of line; then the
        // pixels 0 1 0 as they are (padded to a word), end of bitmap.
        let bits = [3, 1, 0, 0, 0, 3, 0, 1, 0, 0, 0, 1];
        let file = bmp(4, 2, 8, RLE8, 0, &[RED, BLUE], &bits);
        let raster = decode_bmp(&file).unwrap();
        assert_eq!(raster.pixel(2, 1), [0, 0, 255, 255], "bottom row");
        assert_eq!(raster.pixel(3, 1)[3], 0, "not set");
        assert_eq!(raster.pixel(1, 0), [0, 0, 255, 255], "top row");
        // RLE4: four pixels alternating 1 and 0.
        let file = bmp(4, 1, 4, RLE4, 0, &[RED, BLUE], &[4, 0x10, 0, 1]);
        let raster = decode_bmp(&file).unwrap();
        assert_eq!(raster.pixel(0, 0), [0, 0, 255, 255]);
        assert_eq!(raster.pixel(1, 0), [255, 0, 0, 255]);
    }

    /// `AmiPaint`'s sizes: PI1 (120 × 80, no resolution) 1,800 × 1,200;
    /// PIC2's 72 dpi (2,835 per metre) 2,399 × 1,599; wide bitmaps without
    /// a resolution at most 21 in; large ones halved below 32,768.
    #[test]
    fn natural_sizes_are_ami_pros() {
        let header = |width, height, ppm| {
            Header::read(&bmp(width, height, 24, 0, ppm, &[], &[])[14..]).unwrap()
        };
        assert_eq!(
            header(120, 80, 0).natural_size(),
            Twips {
                width: 1_800,
                height: 1_200
            }
        );
        assert_eq!(
            header(120, 80, 2_835).natural_size(),
            Twips {
                width: 2_399,
                height: 1_599
            }
        );
        let wide = header(4_032, 3_024, 0).natural_size();
        assert_eq!(wide.width, 4_032 * 1_440 / 192);
        assert_eq!(wide.width, 21 * 1_440, "at most 21 in");
        let big = header(4_032, 3_024, 2_835).natural_size();
        assert!(big.width <= 32_767 && big.height <= 32_767);
        assert_eq!(big.width, 4_032 * 56_693 / 2_835 / 4);
    }

    #[test]
    fn malformed_bitmaps_are_refused() {
        let file = bmp(2, 2, 24, 0, 0, &[], &[0; 16]);
        assert!(decode_bmp(&file[..30]).is_none());
        assert!(decode_bmp(&file[..file.len() - 1]).is_none(), "short bits");
        assert!(decode_bmp(&bmp(100_000, 100_000, 24, 0, 0, &[], &[])).is_none());
        assert!(decode_bmp(&bmp(1, 1, 7, 0, 0, &[], &[0; 4])).is_none());
        assert!(decode_bmp(b"BM").is_none());
        for cut in 0..file.len() {
            let _ = decode_bmp(&file[..cut]);
        }
    }

    #[test]
    fn written_bitmaps_read_back() {
        let mut raster = Raster::filled(3, 2, [10, 20, 30, 255]).unwrap();
        raster.put(2, 1, [0, 0, 0, 0]);
        let file = encode_bmp24(&raster, (11_811, 11_811));
        let back = decode_bmp(&file).unwrap();
        assert_eq!(back.pixel(0, 0), [10, 20, 30, 255]);
        assert_eq!(
            back.pixel(2, 1),
            [255, 255, 255, 255],
            "transparency on white"
        );
        assert_eq!(bmp_header(&file).unwrap().x_ppm, 11_811);
    }
}
