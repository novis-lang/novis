//! The `compare` export: two encoded images of one size, compared pixel by pixel
//! (`rule:core-classes/image-one-entry-point-per-job`).
//!
//! Both inputs are decoded as `open` decodes by default — the EXIF orientation applied and an
//! embedded profile converted to sRGB — under the default pixel cap. Bytes that do not decode
//! return the decoder's error with the input named. Two frames of different sizes return
//! `Invalid` naming both sizes.
//!
//! **A pixel's delta** is the largest difference between its four RGBA8 channels in the two
//! frames, straight alpha. `max_delta` is the largest delta in the image, `identical` is that it
//! is `0`, and `differing_pixels` counts the pixels whose delta is above `tolerance`, so a
//! tolerance of `255` or more counts none. `identical` does not read the tolerance.
//!
//! **SSIM** is Wang et al.'s structural similarity over each frame's luma (Rec. 601 weights, the
//! colour multiplied by its alpha, as the pixel looks over black), in 8×8 windows at a stride of
//! 4 with the window's plain mean and variance, and averaged over the windows. A window is the
//! whole side of a frame narrower than 8, and the last window in each direction ends at the
//! frame's edge. Identical frames score exactly `1.0`.
//!
//! **The rendered diff** is a PNG the size of the frames: the first image in grey, faded towards
//! white, with every pixel `differing_pixels` counts in opaque red. It is made only with
//! `render`, and it is a third frame beside the two decoded ones while it is encoded.

use crate::{DEFAULT_MAX_PIXELS, Encoding, Error, Format, Pixels, decode, encode};

/// The result of `compare`, the WIT `diff` record.
#[derive(Debug, Clone, PartialEq)]
pub struct Diff {
    pub identical: bool,
    pub differing_pixels: u64,
    pub max_delta: u64,
    pub ssim: f64,
    /// The rendered diff, encoded as PNG, when `render` was set.
    pub diff: Option<Vec<u8>>,
}

/// Decodes `a` and `b` and compares them, as the module doc says.
pub fn compare(a: &[u8], b: &[u8], tolerance: u64, render: bool) -> Result<Diff, Error> {
    let first = decode(a, DEFAULT_MAX_PIXELS, true, true).map_err(|err| named("first", err))?;
    let second = decode(b, DEFAULT_MAX_PIXELS, true, true).map_err(|err| named("second", err))?;
    pixels(&first, &second, tolerance, render)
}

/// Compares two decoded frames, as the module doc says.
pub fn pixels(a: &Pixels, b: &Pixels, tolerance: u64, render: bool) -> Result<Diff, Error> {
    if (a.width, a.height) != (b.width, b.height) {
        return Err(Error::Invalid(format!(
            "the images are {}x{} and {}x{}, and `compare` needs two images of the same size",
            a.width, a.height, b.width, b.height
        )));
    }
    let mut differing_pixels = 0;
    let mut max_delta = 0;
    let mut marks = Vec::new();
    for (x, y) in a.rgba.chunks_exact(4).zip(b.rgba.chunks_exact(4)) {
        let delta = x
            .iter()
            .zip(y)
            .map(|(x, y)| u64::from(x.abs_diff(*y)))
            .max()
            .unwrap_or(0);
        max_delta = max_delta.max(delta);
        let differs = delta > tolerance;
        differing_pixels += u64::from(differs);
        if render {
            marks.push(differs);
        }
    }
    let identical = max_delta == 0;
    let ssim = if identical {
        1.0
    } else {
        ssim(&luma(a), &luma(b), a.width as usize, a.height as usize)
    };
    let diff = if render {
        Some(rendered(a, &marks)?)
    } else {
        None
    };
    Ok(Diff {
        identical,
        differing_pixels,
        max_delta,
        ssim,
        diff,
    })
}

/// `err` with its message saying which input it is about.
fn named(which: &str, err: Error) -> Error {
    match err {
        Error::Invalid(m) => Error::Invalid(format!("the {which} image: {m}")),
        Error::Parse(m) => Error::Parse(format!("the {which} image: {m}")),
        Error::Runtime(m) => Error::Runtime(format!("the {which} image: {m}")),
    }
}

/// Each pixel's Rec. 601 luma, its colour multiplied by its alpha.
fn luma(pixels: &Pixels) -> Vec<f64> {
    pixels
        .rgba
        .chunks_exact(4)
        .map(|p| {
            let y = 0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2]);
            y * f64::from(p[3]) / 255.0
        })
        .collect()
}

/// The start of each window along a side of `len`, for windows of `size`.
fn starts(len: usize, size: usize) -> Vec<usize> {
    let mut starts: Vec<usize> = (0..=len - size).step_by(4).collect();
    if starts.last() != Some(&(len - size)) {
        starts.push(len - size);
    }
    starts
}

/// The mean SSIM of two luma planes of `width` by `height`.
fn ssim(a: &[f64], b: &[f64], width: usize, height: usize) -> f64 {
    const C1: f64 = (0.01 * 255.0) * (0.01 * 255.0);
    const C2: f64 = (0.03 * 255.0) * (0.03 * 255.0);
    let (w, h) = (width.min(8), height.min(8));
    let n = (w * h) as f64;
    let (xs, ys) = (starts(width, w), starts(height, h));
    let mut total = 0.0;
    for &y0 in &ys {
        for &x0 in &xs {
            let rows = (y0..y0 + h).map(|y| y * width + x0..y * width + x0 + w);
            let (mut sa, mut sb) = (0.0, 0.0);
            for row in rows.clone() {
                sa += a[row.clone()].iter().sum::<f64>();
                sb += b[row].iter().sum::<f64>();
            }
            let (ma, mb) = (sa / n, sb / n);
            let (mut va, mut vb, mut cov) = (0.0, 0.0, 0.0);
            for row in rows {
                for (x, y) in a[row.clone()].iter().zip(&b[row]) {
                    let (dx, dy) = (x - ma, y - mb);
                    va += dx * dx;
                    vb += dy * dy;
                    cov += dx * dy;
                }
            }
            let (va, vb, cov) = (va / n, vb / n, cov / n);
            total += ((2.0 * ma * mb + C1) * (2.0 * cov + C2))
                / ((ma * ma + mb * mb + C1) * (va + vb + C2));
        }
    }
    total / (xs.len() * ys.len()) as f64
}

/// The diff PNG: `base` in grey faded towards white, and red wherever `marks` is set.
fn rendered(base: &Pixels, marks: &[bool]) -> Result<Vec<u8>, Error> {
    let mut rgba = Vec::with_capacity(base.rgba.len());
    for (p, &mark) in base.rgba.chunks_exact(4).zip(marks) {
        if mark {
            rgba.extend_from_slice(&[255, 0, 0, 255]);
        } else {
            let y = (u32::from(p[0]) * 299 + u32::from(p[1]) * 587 + u32::from(p[2]) * 114) / 1000;
            let y = y * u32::from(p[3]) / 255;
            let grey = (192 + y / 4) as u8;
            rgba.extend_from_slice(&[grey, grey, grey, 255]);
        }
    }
    encode(
        Pixels {
            width: base.width,
            height: base.height,
            rgba,
            exif: None,
        },
        Format::Png,
        Encoding::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `width` by `height` frame whose pixel at `x`, `y` is `[x * 16, y * 16, 128, 255]`.
    fn gradient(width: u32, height: u32) -> Pixels {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&[(x * 16) as u8, (y * 16) as u8, 128, 255]);
            }
        }
        Pixels {
            width,
            height,
            rgba,
            exif: None,
        }
    }

    fn png(pixels: &Pixels) -> Vec<u8> {
        encode(pixels.clone(), Format::Png, Encoding::default()).unwrap()
    }

    #[test]
    fn an_image_compared_with_itself_is_identical_with_an_ssim_of_one() {
        let bytes = png(&gradient(12, 10));
        let diff = compare(&bytes, &bytes, 0, false).unwrap();
        assert_eq!(
            diff,
            Diff {
                identical: true,
                differing_pixels: 0,
                max_delta: 0,
                ssim: 1.0,
                diff: None,
            }
        );
    }

    #[test]
    fn one_changed_pixel_is_one_differing_pixel_with_its_delta() {
        let a = gradient(12, 10);
        let mut b = a.clone();
        let at = (3 * 12 + 5) * 4;
        b.rgba[at + 2] = 128 + 40;
        let diff = compare(&png(&a), &png(&b), 0, false).unwrap();
        assert!(!diff.identical);
        assert_eq!((diff.differing_pixels, diff.max_delta), (1, 40));
        assert!(diff.ssim < 1.0 && diff.ssim > 0.9, "{}", diff.ssim);
    }

    #[test]
    fn a_delta_at_the_tolerance_is_not_counted_and_one_above_it_is() {
        let a = gradient(4, 4);
        let mut b = a.clone();
        b.rgba[0] += 10;
        b.rgba[4] += 11;
        let diff = pixels(&a, &b, 10, false).unwrap();
        assert_eq!((diff.differing_pixels, diff.max_delta), (1, 11));
        assert!(!diff.identical);
        assert_eq!(pixels(&a, &b, 255, false).unwrap().differing_pixels, 0);
    }

    #[test]
    fn a_size_mismatch_is_invalid_and_names_both_sizes() {
        let err = pixels(&gradient(3, 2), &gradient(2, 3), 0, false).unwrap_err();
        let Error::Invalid(message) = err else {
            panic!("{err:?}")
        };
        assert!(
            message.contains("3x2") && message.contains("2x3"),
            "{message}"
        );
    }

    #[test]
    fn bytes_that_are_not_an_image_name_which_input_they_are() {
        let bytes = png(&gradient(2, 2));
        let err = compare(&bytes, b"not an image", 0, false).unwrap_err();
        let Error::Parse(message) = err else {
            panic!("{err:?}")
        };
        assert!(message.starts_with("the second image: "), "{message}");
    }

    #[test]
    fn the_rendered_diff_is_a_png_with_the_differing_pixels_in_red() {
        let a = gradient(9, 5);
        let mut b = a.clone();
        b.rgba[(2 * 9 + 7) * 4] ^= 0xff;
        let diff = pixels(&a, &b, 0, true).unwrap();
        let shown = decode(&diff.diff.unwrap(), DEFAULT_MAX_PIXELS, false, false).unwrap();
        assert_eq!((shown.width, shown.height), (9, 5));
        let red = shown
            .rgba
            .chunks_exact(4)
            .enumerate()
            .filter(|(_, p)| *p == [255, 0, 0, 255])
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        assert_eq!(red, [2 * 9 + 7]);
    }

    #[test]
    fn ssim_falls_as_the_second_image_moves_further_from_the_first() {
        let a = gradient(16, 16);
        let scores = [8u8, 32, 96].map(|noise| {
            let mut b = a.clone();
            for (i, v) in b.rgba.iter_mut().enumerate() {
                if i % 4 != 3 && (i / 4) % 2 == 0 {
                    *v = v.saturating_add(noise);
                }
            }
            pixels(&a, &b, 0, false).unwrap().ssim
        });
        assert!(scores[0] > scores[1] && scores[1] > scores[2], "{scores:?}");
        assert!(scores.iter().all(|s| *s < 1.0), "{scores:?}");
    }

    #[test]
    fn a_frame_narrower_than_a_window_is_one_window_wide() {
        let a = gradient(3, 20);
        let mut b = a.clone();
        b.rgba[0] = 255;
        let diff = pixels(&a, &b, 0, false).unwrap();
        assert!(diff.ssim.is_finite() && diff.ssim < 1.0, "{}", diff.ssim);
    }
}
