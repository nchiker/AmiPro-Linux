//! Image Processing's filters (`amiimage.dll`, `docs/research/OBJECTS_NOTES.md`
//! 11.3 and 12.3): a lookup curve for brightness and contrast, smoothing,
//! edge enhancement, and invert, applied in Ami Pro's order (Lookup,
//! Smooth, Edge, Negate) to a raster's colour channels; alpha is kept.
//!
//! The lookup curve is the decompile's polyline, checked against the
//! VM's end points (IMV1). Ami Pro's smoothing and edge kernels were not
//! found; the ones here are the app's choice (OK-DO5-1, OK-DO5-2): smoothing
//! blends the pixel with the [1 2 1] x [1 2 1] / 16 blur by the strength,
//! and edge enhancement adds the strength times the four-neighbour
//! Laplacian, which rings one source pixel on each side of a step and
//! overshoots to saturation at 100, as the VM showed.

use crate::Raster;

/// The 0 to 100 settings (50, 50, 0, 0, off is the identity).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Settings {
    pub brightness: u8,
    pub contrast: u8,
    pub edge: u8,
    pub smoothing: u8,
    pub invert: bool,
}

impl Settings {
    pub fn is_neutral(&self) -> bool {
        *self
            == Self {
                brightness: 50,
                contrast: 50,
                edge: 0,
                smoothing: 0,
                invert: false,
            }
    }
}

/// The 256-entry table of brightness `b` and contrast `c` (`1018:` lookup
/// builder): the polyline (0,0), (x1,y1), (x2,y2), (255,255) with
/// truncating integer arithmetic.
pub fn lookup(brightness: u8, contrast: u8) -> [u8; 256] {
    let b = i32::from(brightness.min(100));
    let c = i32::from(contrast.min(100));
    let (mut x1, mut y1, mut x2, mut y2);
    if c == 100 {
        x1 = 255 * (100 - b) / 100;
        x2 = x1;
        y1 = 0;
        y2 = 255;
    } else if c > 50 {
        let s = 510 - 255 * c / 50;
        let u = (s + 255) * (100 - b) / 100;
        x1 = u - s;
        x2 = u;
        y1 = 0;
        y2 = 255;
        if x1 < 0 {
            y1 = 255 * (-x1) / s;
            x1 = 0;
        }
        if x2 > 255 {
            y2 = 255 * (255 - x1) / s;
            x2 = 255;
        }
    } else {
        let s = 255 * c / 50;
        y2 = b * (s + 255) / 100;
        y1 = y2 - s;
        x1 = 0;
        x2 = 255;
    }
    let points = [(0, 0), (x1, y1), (x2, y2), (255, 255)];
    let mut table = [0u8; 256];
    for (x, out) in table.iter_mut().enumerate() {
        let x = x as i32;
        let mut y = x;
        for pair in points.windows(2) {
            let ((xa, ya), (xb, yb)) = (pair[0], pair[1]);
            if xa < xb && (xa..=xb).contains(&x) {
                y = ya + (yb - ya) * (x - xa) / (xb - xa);
                break;
            }
        }
        *out = y.clamp(0, 255) as u8;
    }
    table
}

fn blur(source: &[u8], width: usize, height: usize) -> Vec<u8> {
    let at = |x: isize, y: isize, channel: usize| {
        let x = x.clamp(0, width as isize - 1) as usize;
        let y = y.clamp(0, height as isize - 1) as usize;
        u32::from(source[(y * width + x) * 4 + channel])
    };
    let mut out = source.to_vec();
    const W: [u32; 3] = [1, 2, 1];
    for y in 0..height {
        for x in 0..width {
            for channel in 0..3 {
                let mut sum = 0;
                for (dy, wy) in W.iter().enumerate() {
                    for (dx, wx) in W.iter().enumerate() {
                        sum += wy
                            * wx
                            * at(
                                x as isize + dx as isize - 1,
                                y as isize + dy as isize - 1,
                                channel,
                            );
                    }
                }
                out[(y * width + x) * 4 + channel] = ((sum + 8) / 16) as u8;
            }
        }
    }
    out
}

fn laplacian(source: &[u8], width: usize, height: usize, strength: i32) -> Vec<u8> {
    let at = |x: isize, y: isize, channel: usize| {
        let x = x.clamp(0, width as isize - 1) as usize;
        let y = y.clamp(0, height as isize - 1) as usize;
        i32::from(source[(y * width + x) * 4 + channel])
    };
    let mut out = source.to_vec();
    for y in 0..height as isize {
        for x in 0..width as isize {
            for channel in 0..3 {
                let centre = at(x, y, channel);
                let lap = 4 * centre
                    - at(x - 1, y, channel)
                    - at(x + 1, y, channel)
                    - at(x, y - 1, channel)
                    - at(x, y + 1, channel);
                let value = centre + lap * strength / 100;
                out[(y as usize * width + x as usize) * 4 + channel] = value.clamp(0, 255) as u8;
            }
        }
    }
    out
}

/// `raster` with the settings applied, in Ami Pro's order.
pub fn apply(raster: &Raster, settings: Settings) -> Raster {
    let (width, height) = (raster.width as usize, raster.height as usize);
    let mut pixels = raster.pixels.clone();
    if settings.is_neutral() || width == 0 || height == 0 || pixels.len() < width * height * 4 {
        return Raster {
            width: raster.width,
            height: raster.height,
            pixels,
        };
    }
    if settings.brightness != 50 || settings.contrast != 50 {
        let table = lookup(settings.brightness, settings.contrast);
        for pixel in pixels.chunks_exact_mut(4) {
            for channel in &mut pixel[..3] {
                *channel = table[usize::from(*channel)];
            }
        }
    }
    if settings.smoothing > 0 {
        let blurred = blur(&pixels, width, height);
        let strength = u32::from(settings.smoothing.min(100));
        for (pixel, soft) in pixels.chunks_exact_mut(4).zip(blurred.chunks_exact(4)) {
            for channel in 0..3 {
                let (a, b) = (u32::from(pixel[channel]), u32::from(soft[channel]));
                pixel[channel] = ((a * (100 - strength) + b * strength + 50) / 100) as u8;
            }
        }
    }
    if settings.edge > 0 {
        pixels = laplacian(&pixels, width, height, i32::from(settings.edge.min(100)));
    }
    if settings.invert {
        for pixel in pixels.chunks_exact_mut(4) {
            for channel in &mut pixel[..3] {
                *channel = 255 - *channel;
            }
        }
    }
    Raster {
        width: raster.width,
        height: raster.height,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gray(values: &[u8], width: u32, height: u32) -> Raster {
        Raster {
            width,
            height,
            pixels: values.iter().flat_map(|v| [*v, *v, *v, 255]).collect(),
        }
    }

    fn row(raster: &Raster) -> Vec<u8> {
        raster.pixels.chunks(4).map(|p| p[0]).collect()
    }

    const NEUTRAL: Settings = Settings {
        brightness: 50,
        contrast: 50,
        edge: 0,
        smoothing: 0,
        invert: false,
    };

    #[test]
    fn fifty_fifty_is_the_identity() {
        let table = lookup(50, 50);
        assert!((0..256).all(|x| usize::from(table[x]) == x));
        let image = gray(&[0, 85, 170, 255], 4, 1);
        assert_eq!(apply(&image, NEUTRAL), image);
    }

    #[test]
    fn the_curve_matches_the_vm_end_points() {
        // Brightness 100 all white, 0 all black (contrast 50).
        assert!(lookup(100, 50).iter().all(|v| *v == 255));
        assert!(lookup(0, 50).iter().all(|v| *v == 0));
        // Contrast 0 at brightness 49: flat 124.
        assert!(lookup(49, 0).iter().all(|v| *v == 124));
        // Contrast 100 at brightness 49: a threshold at 130.
        let table = lookup(49, 100);
        assert_eq!((table[129], table[161]), (0, 255));
        assert_eq!(table[130], 0);
        assert_eq!(table[131], 255);
        // Brightness 30, contrast 70: a ramp from 0 at 132 to 205 at 255.
        let table = lookup(30, 70);
        assert_eq!((table[100], table[132], table[255]), (0, 0, 205));
        assert!(table[200] > table[150] && table[150] > 0);
    }

    #[test]
    fn invert_is_255_minus_x_and_keeps_alpha() {
        let mut image = gray(&[0, 85, 255], 3, 1);
        image.pixels[3] = 77;
        let out = apply(
            &image,
            Settings {
                invert: true,
                ..NEUTRAL
            },
        );
        assert_eq!(row(&out), [255, 170, 0]);
        assert_eq!(out.pixels[3], 77);
    }

    #[test]
    fn smoothing_blurs_about_a_pixel_and_zero_is_off() {
        let image = gray(&[0, 0, 0, 200, 200, 200], 6, 1);
        let out = apply(
            &image,
            Settings {
                smoothing: 100,
                ..NEUTRAL
            },
        );
        let v = row(&out);
        assert_eq!((v[0], v[1]), (0, 0), "{v:?}");
        assert!(v[2] > 0 && v[2] < 100, "{v:?}");
        assert!(v[3] > 100 && v[3] < 200, "{v:?}");
        assert_eq!((v[4], v[5]), (200, 200));
        let half = apply(
            &image,
            Settings {
                smoothing: 50,
                ..NEUTRAL
            },
        );
        assert!(row(&half)[2] < v[2]);
    }

    #[test]
    fn edge_enhancement_rings_one_pixel_to_saturation() {
        let image = gray(&[100, 100, 100, 200, 200, 200], 6, 1);
        let out = apply(
            &image,
            Settings {
                edge: 100,
                ..NEUTRAL
            },
        );
        let v = row(&out);
        // One-dimensionally clamped rows: only the horizontal neighbours.
        assert_eq!((v[0], v[1]), (100, 100), "{v:?}");
        assert!(v[2] < 100 && v[3] > 200, "{v:?}");
        assert_eq!((v[4], v[5]), (200, 200));
        // A flat area is unchanged and a step of 139 saturates.
        let step = gray(&[0, 0, 139, 139], 4, 1);
        let v = row(&apply(
            &step,
            Settings {
                edge: 100,
                ..NEUTRAL
            },
        ));
        assert_eq!(v, [0, 0, 255, 139]);
    }

    #[test]
    fn the_order_is_lookup_smooth_edge_negate() {
        let image = gray(&[10, 10, 240, 240], 4, 1);
        let all = Settings {
            brightness: 60,
            contrast: 60,
            edge: 50,
            smoothing: 50,
            invert: true,
        };
        let by_hand = {
            let step = apply(
                &image,
                Settings {
                    brightness: 60,
                    contrast: 60,
                    ..NEUTRAL
                },
            );
            let step = apply(
                &step,
                Settings {
                    smoothing: 50,
                    ..NEUTRAL
                },
            );
            let step = apply(
                &step,
                Settings {
                    edge: 50,
                    ..NEUTRAL
                },
            );
            apply(
                &step,
                Settings {
                    invert: true,
                    ..NEUTRAL
                },
            )
        };
        assert_eq!(apply(&image, all), by_hand);
    }
}
