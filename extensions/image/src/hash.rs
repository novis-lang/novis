//! The `hash` export: a perceptual hash of one encoded image, for finding near-duplicates
//! (`rule:core-classes/image-one-entry-point-per-job`).
//!
//! The input is decoded as `compare` decodes it — the EXIF orientation applied and an embedded
//! profile converted to sRGB — under the default pixel cap, and bytes that do not decode return
//! the decoder's error. Each kind reads the frame's luma as `compare` does (Rec. 601 weights, the
//! colour multiplied by its alpha), averaged into a small grid of cells: a cell is the mean of
//! the pixels that fall in it, and a cell no pixel falls in, because the frame is narrower than
//! the grid, is the pixel at its corner. A hash is the grid's bits packed most significant first.
//!
//! Each kind has a length of its own, so two hashes of different kinds never compare:
//!
//! - **`Perceptual`**, 8 bytes: a 32×32 grid, its two-dimensional DCT-II, and one bit per
//!   coefficient of the lowest 8×8 frequencies, set when the coefficient is above their median.
//! - **`Difference`**, 16 bytes: a 9×8 grid with a bit set where a cell is brighter than the one
//!   to its right, then an 8×9 grid with a bit set where a cell is brighter than the one below.
//! - **`Average`**, 32 bytes: a 16×16 grid with a bit set where a cell is brighter than the mean.
//!
//! Every kind is written here over the decoded frame, so the component links no hashing crate;
//! the DCT keeps only the frequencies the hash reads, which is a few thousand multiplications
//! whatever the frame's size. Nothing is allocated beyond the luma plane and the
//! grid, and both are freed when the export returns.

use crate::compare::luma;
use crate::{DEFAULT_MAX_PIXELS, Error, Pixels, decode};

/// The three kinds of hash, the WIT `hash-kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Perceptual,
    Difference,
    Average,
}

impl Kind {
    /// The length in bytes of every hash of this kind.
    #[must_use]
    pub const fn bytes(self) -> usize {
        match self {
            Kind::Perceptual => 8,
            Kind::Difference => 16,
            Kind::Average => 32,
        }
    }
}

/// Decodes `data` and hashes it, as the module doc says.
pub fn hash(data: &[u8], kind: Kind) -> Result<Vec<u8>, Error> {
    let frame = decode(data, DEFAULT_MAX_PIXELS, true, true)?;
    Ok(pixels(&frame, kind))
}

/// Hashes a decoded frame, as the module doc says.
#[must_use]
pub fn pixels(frame: &Pixels, kind: Kind) -> Vec<u8> {
    let plane = luma(frame);
    let (width, height) = (frame.width as usize, frame.height as usize);
    match kind {
        Kind::Perceptual => {
            const SIDE: usize = 32;
            const KEEP: usize = 8;
            let low = dct(&grid(&plane, width, height, SIDE, SIDE), SIDE, KEEP);
            let mut sorted = low.clone();
            sorted.sort_by(f64::total_cmp);
            let median = (sorted[KEEP * KEEP / 2 - 1] + sorted[KEEP * KEEP / 2]) / 2.0;
            pack(low.iter().map(|&c| c > median))
        }
        Kind::Difference => {
            let across = grid(&plane, width, height, 9, 8);
            let down = grid(&plane, width, height, 8, 9);
            let rows = (0..8).flat_map(|y| (0..8).map(move |x| (y, x)));
            let right = rows
                .clone()
                .map(|(y, x)| across[y * 9 + x] > across[y * 9 + x + 1]);
            let below = rows.map(|(y, x)| down[y * 8 + x] > down[(y + 1) * 8 + x]);
            pack(right.chain(below))
        }
        Kind::Average => {
            let cells = grid(&plane, width, height, 16, 16);
            let mean = cells.iter().sum::<f64>() / cells.len() as f64;
            pack(cells.iter().map(|&c| c > mean))
        }
    }
}

/// `plane`, `width` by `height`, averaged into `nx` by `ny` cells in row order.
fn grid(plane: &[f64], width: usize, height: usize, nx: usize, ny: usize) -> Vec<f64> {
    let mut sums = vec![0.0; nx * ny];
    let mut counts = vec![0u64; nx * ny];
    if width == 0 || height == 0 {
        return sums;
    }
    // Products in `u64`: a side times a grid side can pass `u32::MAX` on `wasm32`.
    let cell = |at: usize, len: usize, n: usize| (at as u64 * n as u64 / len as u64) as usize;
    for y in 0..height {
        let row = cell(y, height, ny) * nx;
        for x in 0..width {
            let at = row + cell(x, width, nx);
            sums[at] += plane[y * width + x];
            counts[at] += 1;
        }
    }
    for cy in 0..ny {
        for cx in 0..nx {
            let at = cy * nx + cx;
            if counts[at] == 0 {
                let y = cell(cy, ny, height);
                let x = cell(cx, nx, width);
                sums[at] = plane[y * width + x];
            } else {
                sums[at] /= counts[at] as f64;
            }
        }
    }
    sums
}

/// The lowest `keep` by `keep` coefficients of the DCT-II of an `n` by `n` plane, in row order.
fn dct(plane: &[f64], n: usize, keep: usize) -> Vec<f64> {
    let basis: Vec<f64> = (0..keep)
        .flat_map(|k| {
            (0..n).map(move |i| {
                (std::f64::consts::PI * (2 * i + 1) as f64 * k as f64 / (2 * n) as f64).cos()
            })
        })
        .collect();
    let mut rows = vec![0.0; n * keep];
    for r in 0..n {
        for k in 0..keep {
            rows[r * keep + k] = (0..n).map(|i| plane[r * n + i] * basis[k * n + i]).sum();
        }
    }
    let mut out = vec![0.0; keep * keep];
    for u in 0..keep {
        for k in 0..keep {
            out[u * keep + k] = (0..n).map(|r| basis[u * n + r] * rows[r * keep + k]).sum();
        }
    }
    out
}

/// `bits` packed eight to a byte, the first bit the most significant.
fn pack(bits: impl Iterator<Item = bool>) -> Vec<u8> {
    let bits: Vec<bool> = bits.collect();
    bits.chunks(8)
        .map(|byte| {
            byte.iter()
                .enumerate()
                .fold(0u8, |acc, (at, &bit)| acc | (u8::from(bit) << (7 - at)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Encoding, Format, encode};

    const KINDS: [Kind; 3] = [Kind::Perceptual, Kind::Difference, Kind::Average];

    /// A `width` by `height` frame of one picture at any size: red rises to the right, green
    /// rises downwards, and blue fills the top left and the bottom right.
    fn picture(width: u32, height: u32) -> Pixels {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let (u, v) = (
                    f64::from(x) / f64::from(width),
                    f64::from(y) / f64::from(height),
                );
                let blue = if (u < 0.5) == (v < 0.3) { 255 } else { 0 };
                rgba.extend_from_slice(&[(u * 255.0) as u8, (v * 255.0) as u8, blue, 255]);
            }
        }
        Pixels {
            width,
            height,
            rgba,
            exif: None,
        }
    }

    fn mirrored(frame: &Pixels) -> Pixels {
        let mut rgba = Vec::with_capacity(frame.rgba.len());
        for row in frame.rgba.chunks_exact(frame.width as usize * 4) {
            for pixel in row.chunks_exact(4).rev() {
                rgba.extend_from_slice(pixel);
            }
        }
        Pixels {
            rgba,
            ..frame.clone()
        }
    }

    fn distance(a: &[u8], b: &[u8]) -> u32 {
        a.iter().zip(b).map(|(a, b)| (a ^ b).count_ones()).sum()
    }

    #[test]
    fn each_kind_has_its_own_length() {
        let frame = picture(40, 30);
        for kind in KINDS {
            assert_eq!(pixels(&frame, kind).len(), kind.bytes(), "{kind:?}");
        }
        assert_eq!(pixels(&picture(1, 1), Kind::Perceptual).len(), 8);
    }

    #[test]
    fn a_resized_copy_hashes_near_its_source_and_a_mirror_far_from_it() {
        let large = picture(240, 180);
        let small = picture(120, 90);
        let flipped = mirrored(&large);
        for kind in KINDS {
            let bits = kind.bytes() as u32 * 8;
            let source = pixels(&large, kind);
            let near = distance(&source, &pixels(&small, kind));
            let far = distance(&source, &pixels(&flipped, kind));
            assert!(
                near <= bits / 8,
                "{kind:?}: a resized copy is {near} of {bits} bits away"
            );
            assert!(
                far >= bits / 4,
                "{kind:?}: the mirror is only {far} of {bits} bits away"
            );
        }
    }

    #[test]
    fn a_plain_image_has_an_average_hash_of_zeros() {
        let frame = Pixels {
            width: 5,
            height: 3,
            rgba: [40, 80, 120, 255].repeat(15),
            exif: None,
        };
        assert_eq!(pixels(&frame, Kind::Average), [0; 32]);
    }

    #[test]
    fn hash_decodes_its_input_and_returns_what_the_frame_hashes_to() {
        let frame = picture(48, 32);
        let png = encode(frame.clone(), Format::Png, Encoding::default()).unwrap();
        for kind in KINDS {
            assert_eq!(hash(&png, kind).unwrap(), pixels(&frame, kind));
        }
        assert!(matches!(
            hash(b"not an image", Kind::Average),
            Err(Error::Parse(_))
        ));
    }
}
