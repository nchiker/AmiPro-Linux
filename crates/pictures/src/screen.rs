//! Ami Pro's screen image of a picture (`1168:2016` writes it, `1168:26e7`
//! reads it; `docs/research/PICTURE_NOTES.md`): the picture area as Ami Pro
//! last drew it on screen, kept beside the picture as a cache. An 18-byte
//! header (`SS`, width and height in pixels, bytes a row, bits a pixel,
//! planes, the screen's dots per inch across and down, the view's window
//! extent) and the device bitmap's rows from the top.

use crate::{Bytes, Raster};

/// The 16 colours of the screens that wrote four-plane images, by index
/// (red 4, green 2, blue 1, intensity 8).
const VGA: [[u8; 3]; 16] = [
    [0, 0, 0],
    [0, 0, 128],
    [0, 128, 0],
    [0, 128, 128],
    [128, 0, 0],
    [128, 0, 128],
    [128, 128, 0],
    [192, 192, 192],
    [128, 128, 128],
    [0, 0, 255],
    [0, 255, 0],
    [0, 255, 255],
    [255, 0, 0],
    [255, 0, 255],
    [255, 255, 0],
    [255, 255, 255],
];

/// Ami Pro's screen image as a raster: four planes of one bit (each row's
/// planes in turn: red, green, blue, intensity; calibrated against 22 of
/// the corpus's bitmaps), 16 bits, or 32 bits (blue, green, red). Sixteen
/// bits are 5-6-5 or 5-5-5 as the writing screen's were, which the header
/// does not say: an image no pixel of which sets the top bit is 5-5-5 (in
/// the corpora, 13 of 191, 11 of them holding 5-5-5 white; the other 178
/// set it, 176 holding 5-6-5 white).
/// `None` for eight bits, whose colours are indices into the palette of the
/// screen that wrote them, which the file does not keep, and for anything
/// malformed.
pub fn decode(bytes: &[u8]) -> Option<Raster> {
    let read = Bytes(bytes);
    if bytes.get(..2)? != b"SS" {
        return None;
    }
    let width = u32::from(read.u16(2)?);
    let height = u32::from(read.u16(4)?);
    let stride = usize::from(read.u16(6)?);
    let (bits, planes) = (read.u8(8)?, read.u8(9)?);
    let data = bytes.get(18..)?;
    let row_bytes = stride.checked_mul(usize::from(planes))?;
    if data.len() < row_bytes.checked_mul(height as usize)? {
        return None;
    }
    let needed = match (bits, planes) {
        (1, 4) => (width as usize).div_ceil(8),
        (16, 1) => width as usize * 2,
        (32, 1) => width as usize * 4,
        _ => return None,
    };
    if needed > stride {
        return None;
    }
    let mut raster = Raster::filled(width, height, [0, 0, 0, 255])?;
    let fifteen = bits == 16
        && (0..height as usize).all(|y| {
            let row = &data[y * row_bytes..][..width as usize * 2];
            row.chunks_exact(2).all(|pixel| pixel[1] & 0x80 == 0)
        });
    for y in 0..height {
        let row = &data[y as usize * row_bytes..][..row_bytes];
        for x in 0..width {
            let at = x as usize;
            let [r, g, b] = match bits {
                1 => {
                    let bit = |plane: usize| row[plane * stride + at / 8] >> (7 - at % 8) & 1;
                    VGA[usize::from(bit(0) << 2 | bit(1) << 1 | bit(2) | bit(3) << 3)]
                }
                16 => {
                    let value = u16::from_le_bytes([row[2 * at], row[2 * at + 1]]);
                    let channel =
                        |value: u16, max: u16| (u32::from(value) * 255 / u32::from(max)) as u8;
                    if fifteen {
                        [
                            channel(value >> 10 & 31, 31),
                            channel(value >> 5 & 31, 31),
                            channel(value & 31, 31),
                        ]
                    } else {
                        [
                            channel(value >> 11 & 31, 31),
                            channel(value >> 5 & 63, 63),
                            channel(value & 31, 31),
                        ]
                    }
                }
                _ => [row[4 * at + 2], row[4 * at + 1], row[4 * at]],
            };
            raster.put(x, y, [r, g, b, 255]);
        }
    }
    Some(raster)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(width: u16, height: u16, stride: u16, bits: u8, planes: u8, data: &[u8]) -> Vec<u8> {
        let mut out = b"SS".to_vec();
        for value in [width, height, stride] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&[bits, planes]);
        for value in [96u16, 96, 1_440, (-1_440i16) as u16] {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(data);
        out
    }

    #[test]
    fn screen_images_decode_by_their_depth() {
        // Four planes, 2 × 1: pixel 0 bright red (red and intensity),
        // pixel 1 light grey (red, green, blue).
        let planes = [0b1100_0000, 0b0100_0000, 0b0100_0000, 0b1000_0000];
        let raster = decode(&image(2, 1, 1, 1, 4, &planes)).unwrap();
        assert_eq!(raster.pixel(0, 0), [255, 0, 0, 255]);
        assert_eq!(raster.pixel(1, 0), [192, 192, 192, 255]);
        // 16 bits, 5-6-5 (white sets the top bit): pure green.
        let raster = decode(&image(2, 1, 4, 16, 1, &[0xe0, 0x07, 0xff, 0xff])).unwrap();
        assert_eq!(raster.pixel(0, 0), [0, 255, 0, 255]);
        assert_eq!(raster.pixel(1, 0), [255, 255, 255, 255]);
        // 5-5-5, as no pixel sets the top bit: white and pure green.
        let raster = decode(&image(2, 1, 4, 16, 1, &[0xff, 0x7f, 0xe0, 0x03])).unwrap();
        assert_eq!(raster.pixel(0, 0), [255, 255, 255, 255]);
        assert_eq!(raster.pixel(1, 0), [0, 255, 0, 255]);
        // 32 bits: blue, green, red, unused.
        let raster = decode(&image(1, 1, 4, 32, 1, &[1, 2, 3, 0])).unwrap();
        assert_eq!(raster.pixel(0, 0), [3, 2, 1, 255]);
        // Eight bits need a palette the file does not keep.
        assert!(decode(&image(1, 1, 2, 8, 1, &[0, 0])).is_none());
        // Short or malformed.
        assert!(decode(&image(2, 2, 1, 1, 4, &planes)).is_none());
        assert!(decode(&image(64, 1, 1, 16, 1, &[0; 2])).is_none());
        assert!(decode(b"SX").is_none());
    }
}
