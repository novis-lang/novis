//! The `placeholder` and `palette` exports: a short string a front end draws while the image
//! loads, and the colours an image is mostly made of (`rule:core-classes/image-one-entry-point-per-job`).
//!
//! Both decode their input as `hash` does — the EXIF orientation applied and an embedded profile
//! converted to sRGB — under the default pixel cap, and bytes that do not decode return the
//! decoder's error.
//!
//! A placeholder is made from the frame shrunk to fit 100×100: each cell is the mean of the
//! pixels that fall in it, its colour weighted by their alpha, so a transparent pixel adds no
//! colour. A cell with no colour at all is black. Each kind is written here, so the component
//! links no placeholder crate:
//!
//! - **`BlurHash`**, the format of blurha.sh: 4×3 components for a landscape or square frame and
//!   3×4 for a portrait one, in base 83, 28 characters. Its first component is the frame's mean
//!   colour in linear light, so a decoder's average colour is the image's. Alpha is not encoded.
//! - **`ThumbHash`**, the format of evanw.github.io/thumbhash: the reference encoder's bytes over
//!   the shrunk frame, in standard base64 with padding. Its header carries the frame's mean colour
//!   in sRGB, weighted by alpha, and its mean alpha when any cell is not opaque.
//!
//! The palette runs `color_quant`'s NeuQuant over at most [`SAMPLES`] of the frame's visible
//! pixels, taken at an even stride; a pixel with an alpha of 0 is not visible and not counted.
//! Each sampled pixel joins the cluster of the colour it maps to, and the two clusters whose
//! merge moves their pixels least are merged until at most `count` are left. A cluster's colour
//! is the mean of its own pixels, so an image of a few flat colours returns those colours
//! exactly. The palette is most frequent first, has no empty cluster, and is empty for a `count`
//! of 0 or an image with nothing visible; a `count` above [`MAX_PALETTE`] returns `invalid`.
//!
//! Each export allocates the shrunk frame or the sample beside the decoded frame, and frees all
//! of it when it returns.

use color_quant::NeuQuant;

use crate::{DEFAULT_MAX_PIXELS, Error, Pixels, decode};

/// The two kinds of placeholder, the WIT `placeholder-kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    BlurHash,
    ThumbHash,
}

/// The largest `count` the palette takes, which is the most colours NeuQuant makes.
pub const MAX_PALETTE: u64 = 256;

/// The most pixels the palette samples from one frame.
pub const SAMPLES: usize = 65_536;

/// The longest side a placeholder is made from.
const SIDE: u64 = 100;

/// Decodes `data` and makes its placeholder, as the module doc says.
pub fn placeholder(data: &[u8], kind: Kind) -> Result<String, Error> {
    let frame = decode(data, DEFAULT_MAX_PIXELS, true, true)?;
    Ok(pixels(&frame, kind))
}

/// The placeholder of a decoded frame, as the module doc says.
#[must_use]
pub fn pixels(frame: &Pixels, kind: Kind) -> String {
    let small = shrink(frame);
    match kind {
        Kind::BlurHash => blurhash(&small),
        Kind::ThumbHash => base64(&thumbhash(&small)),
    }
}

/// Decodes `data` and returns its palette, as the module doc says.
pub fn palette(data: &[u8], count: u64) -> Result<Vec<[u8; 4]>, Error> {
    if count > MAX_PALETTE {
        return Err(Error::Invalid(format!(
            "a palette has at most {MAX_PALETTE} colours, and {count} were asked for"
        )));
    }
    let frame = decode(data, DEFAULT_MAX_PIXELS, true, true)?;
    Ok(colours(&frame, count as usize))
}

/// The palette of a decoded frame, `count` at most [`MAX_PALETTE`], as the module doc says.
#[must_use]
pub fn colours(frame: &Pixels, count: usize) -> Vec<[u8; 4]> {
    let visible = || frame.rgba.chunks_exact(4).filter(|p| p[3] > 0);
    let total = visible().count();
    if count == 0 || total == 0 {
        return Vec::new();
    }
    let sample: Vec<u8> = visible()
        .step_by(total.div_ceil(SAMPLES))
        .flatten()
        .copied()
        .collect();
    // NeuQuant is written for 64 colours or more, so it makes at least 64 and the merge below
    // brings them down to `count`.
    let net = NeuQuant::new(10, count.max(64), &sample);
    let mut clusters = vec![Cluster::default(); count.max(64)];
    for pixel in sample.chunks_exact(4) {
        let cluster = &mut clusters[net.index_of(pixel)];
        for (sum, &c) in cluster.sum.iter_mut().zip(pixel) {
            *sum += f64::from(c);
        }
        cluster.n += 1.0;
    }
    clusters.retain(|c| c.n > 0.0);
    while clusters.len() > count {
        let mut best = (f64::INFINITY, 0, 1);
        for a in 0..clusters.len() {
            for b in a + 1..clusters.len() {
                let cost = clusters[a].cost(&clusters[b]);
                if cost < best.0 {
                    best = (cost, a, b);
                }
            }
        }
        let gone = clusters.remove(best.2);
        let kept = &mut clusters[best.1];
        for (sum, add) in kept.sum.iter_mut().zip(gone.sum) {
            *sum += add;
        }
        kept.n += gone.n;
    }
    clusters.sort_by(|a, b| b.n.total_cmp(&a.n));
    clusters.iter().map(Cluster::colour).collect()
}

/// Sampled pixels that map to one colour: their channel sums and how many there are.
#[derive(Debug, Clone, Copy, Default)]
struct Cluster {
    sum: [f64; 4],
    n: f64,
}

impl Cluster {
    fn mean(&self) -> [f64; 4] {
        self.sum.map(|s| s / self.n)
    }

    fn colour(&self) -> [u8; 4] {
        self.mean().map(|c| c.round().clamp(0.0, 255.0) as u8)
    }

    /// How far merging `other` into this cluster moves their pixels in total, squared: Ward's
    /// criterion, so a small cluster merges before a large one the same distance away.
    fn cost(&self, other: &Cluster) -> f64 {
        let (a, b) = (self.mean(), other.mean());
        let distance: f64 = a.iter().zip(&b).map(|(a, b)| (a - b) * (a - b)).sum();
        distance * self.n * other.n / (self.n + other.n)
    }
}

/// A frame shrunk to fit [`SIDE`]: straight sRGB colour and alpha, each 0.0 to 1.0, in row order.
struct Small {
    width: usize,
    height: usize,
    rgba: Vec<[f64; 4]>,
}

/// `frame` averaged into cells, as the module doc says. A frame that fits is one pixel a cell.
fn shrink(frame: &Pixels) -> Small {
    let (width, height) = (u64::from(frame.width), u64::from(frame.height));
    if width == 0 || height == 0 {
        return Small {
            width: 1,
            height: 1,
            rgba: vec![[0.0; 4]],
        };
    }
    let longest = width.max(height);
    let side = |len: u64| {
        if longest <= SIDE {
            len
        } else {
            ((len * SIDE + longest / 2) / longest).max(1)
        }
    };
    let (nx, ny) = (side(width), side(height));
    let mut sums = vec![[0.0; 4]; (nx * ny) as usize];
    let mut counts = vec![0u64; (nx * ny) as usize];
    for (y, row) in frame.rgba.chunks_exact(width as usize * 4).enumerate() {
        let cy = y as u64 * ny / height;
        for (x, p) in row.chunks_exact(4).enumerate() {
            let at = (cy * nx + x as u64 * nx / width) as usize;
            let alpha = f64::from(p[3]) / 255.0;
            for c in 0..3 {
                sums[at][c] += f64::from(p[c]) / 255.0 * alpha;
            }
            sums[at][3] += alpha;
            counts[at] += 1;
        }
    }
    let rgba = sums
        .iter()
        .zip(&counts)
        .map(|(s, &n)| {
            let colour = |c: f64| if s[3] > 0.0 { c / s[3] } else { 0.0 };
            [
                colour(s[0]),
                colour(s[1]),
                colour(s[2]),
                s[3] / n.max(1) as f64,
            ]
        })
        .collect();
    Small {
        width: nx as usize,
        height: ny as usize,
        rgba,
    }
}

const BASE83: &[u8; 83] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz#$%*+,-.:;=?@[]^_{|}~";

/// `value` as `digits` base-83 digits, the most significant first.
fn base83(out: &mut String, value: u32, digits: u32) {
    for at in (0..digits).rev() {
        out.push(char::from(BASE83[(value / 83u32.pow(at) % 83) as usize]));
    }
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f64) -> u32 {
    let v = v.clamp(0.0, 1.0);
    let c = if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0 + 0.5) as u32
}

/// The BlurHash of `small`, as blurha.sh's reference encoder writes it.
fn blurhash(small: &Small) -> String {
    let (nx, ny) = if small.width >= small.height {
        (4, 3)
    } else {
        (3, 4)
    };
    let (w, h) = (small.width, small.height);
    let linear: Vec<[f64; 3]> = small
        .rgba
        .iter()
        .map(|p| [p[0], p[1], p[2]].map(srgb_to_linear))
        .collect();
    let mut factors = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let fx: Vec<f64> = (0..w)
                .map(|x| (std::f64::consts::PI * i as f64 * x as f64 / w as f64).cos())
                .collect();
            let mut f = [0.0; 3];
            for y in 0..h {
                let fy = (std::f64::consts::PI * j as f64 * y as f64 / h as f64).cos();
                for x in 0..w {
                    let basis = fx[x] * fy;
                    for (f, c) in f.iter_mut().zip(linear[y * w + x]) {
                        *f += basis * c;
                    }
                }
            }
            let scale = if i == 0 && j == 0 { 1.0 } else { 2.0 } / (w * h) as f64;
            factors.push(f.map(|f| f * scale));
        }
    }
    let mut out = String::with_capacity(28);
    base83(&mut out, (nx - 1 + (ny - 1) * 9) as u32, 1);
    let largest = factors[1..]
        .iter()
        .flatten()
        .fold(0.0f64, |m, f| m.max(f.abs()));
    let quantised = ((largest * 166.0 - 0.5).floor()).clamp(0.0, 82.0) as u32;
    let maximum = f64::from(quantised + 1) / 166.0;
    base83(&mut out, quantised, 1);
    let dc = factors[0].map(linear_to_srgb);
    base83(&mut out, (dc[0] << 16) | (dc[1] << 8) | dc[2], 4);
    for f in &factors[1..] {
        let q = f.map(|v| {
            let v = v / maximum;
            (v.signum() * v.abs().sqrt() * 9.0 + 9.5)
                .floor()
                .clamp(0.0, 18.0) as u32
        });
        base83(&mut out, q[0] * 19 * 19 + q[1] * 19 + q[2], 2);
    }
    out
}

/// The ThumbHash bytes of `small`, as evanw.github.io/thumbhash's reference encoder writes them.
fn thumbhash(small: &Small) -> Vec<u8> {
    let (w, h) = (small.width, small.height);
    let mut avg = [0.0; 4];
    for p in &small.rgba {
        for c in 0..3 {
            avg[c] += p[3] * p[c];
        }
        avg[3] += p[3];
    }
    if avg[3] > 0.0 {
        for c in 0..3 {
            avg[c] /= avg[3];
        }
    }
    let has_alpha = avg[3] < (w * h) as f64;
    let l_limit = if has_alpha { 5.0 } else { 7.0 };
    let longest = w.max(h) as f64;
    let lx = ((l_limit * w as f64 / longest).round() as usize).max(1);
    let ly = ((l_limit * h as f64 / longest).round() as usize).max(1);
    let (mut l, mut p, mut q, mut a) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for px in &small.rgba {
        let alpha = px[3];
        let [r, g, b] = [0, 1, 2].map(|c| avg[c] * (1.0 - alpha) + alpha * px[c]);
        l.push((r + g + b) / 3.0);
        p.push((r + g) / 2.0 - b);
        q.push(r - g);
        a.push(alpha);
    }
    let channel = |values: &[f64], nx: usize, ny: usize| {
        let (mut dc, mut ac, mut scale) = (0.0, Vec::new(), 0.0f64);
        for cy in 0..ny {
            let mut cx = 0;
            while cx * ny < nx * (ny - cy) {
                let fx: Vec<f64> = (0..w)
                    .map(|x| (std::f64::consts::PI / w as f64 * cx as f64 * (x as f64 + 0.5)).cos())
                    .collect();
                let mut f = 0.0;
                for y in 0..h {
                    let fy = (std::f64::consts::PI / h as f64 * cy as f64 * (y as f64 + 0.5)).cos();
                    for x in 0..w {
                        f += values[x + y * w] * fx[x] * fy;
                    }
                }
                f /= (w * h) as f64;
                if cx > 0 || cy > 0 {
                    ac.push(f);
                    scale = scale.max(f.abs());
                } else {
                    dc = f;
                }
                cx += 1;
            }
        }
        if scale > 0.0 {
            for f in &mut ac {
                *f = 0.5 + 0.5 / scale * *f;
            }
        }
        (dc, ac, scale)
    };
    let (l_dc, l_ac, l_scale) = channel(&l, lx.max(3), ly.max(3));
    let (p_dc, p_ac, p_scale) = channel(&p, 3, 3);
    let (q_dc, q_ac, q_scale) = channel(&q, 3, 3);
    let alpha = has_alpha.then(|| channel(&a, 5, 5));
    let round = |v: f64| v.round() as u32;
    let landscape = w > h;
    let header24 = round(63.0 * l_dc)
        | (round(31.5 + 31.5 * p_dc) << 6)
        | (round(31.5 + 31.5 * q_dc) << 12)
        | (round(31.0 * l_scale) << 18)
        | (u32::from(has_alpha) << 23);
    let header16 = (if landscape { ly } else { lx }) as u32
        | (round(63.0 * p_scale) << 3)
        | (round(63.0 * q_scale) << 9)
        | (u32::from(landscape) << 15);
    let mut out = vec![
        header24 as u8,
        (header24 >> 8) as u8,
        (header24 >> 16) as u8,
        header16 as u8,
        (header16 >> 8) as u8,
    ];
    if let Some((a_dc, _, a_scale)) = &alpha {
        out.push((round(15.0 * a_dc) | (round(15.0 * a_scale) << 4)) as u8);
    }
    let start = out.len();
    let mut acs = vec![&l_ac, &p_ac, &q_ac];
    if let Some((_, a_ac, _)) = &alpha {
        acs.push(a_ac);
    }
    for (at, f) in acs.into_iter().flatten().enumerate() {
        if start + at / 2 == out.len() {
            out.push(0);
        }
        out[start + at / 2] |= (round(15.0 * f) << ((at & 1) * 4)) as u8;
    }
    out
}

/// `bytes` in standard base64 with padding.
fn base64(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (at, &b)| n | (u32::from(b) << (16 - 8 * at)));
        for at in 0..4 {
            if at <= chunk.len() {
                out.push(char::from(DIGITS[(n >> (18 - 6 * at) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Encoding, Format, encode};

    fn plain(width: u32, height: u32, colour: [u8; 4]) -> Pixels {
        Pixels {
            width,
            height,
            rgba: colour.repeat((width * height) as usize),
            exif: None,
        }
    }

    /// A `width` by `height` frame whose left `left` columns are `a` and the rest `b`.
    fn split(width: u32, height: u32, left: u32, a: [u8; 4], b: [u8; 4]) -> Pixels {
        let mut rgba = Vec::new();
        for _ in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(if x < left { &a } else { &b });
            }
        }
        Pixels {
            width,
            height,
            rgba,
            exif: None,
        }
    }

    /// The mean colour a BlurHash decodes to.
    fn blurhash_average(hash: &str) -> [u32; 3] {
        let n = hash.as_bytes()[2..6].iter().fold(0u32, |n, c| {
            n * 83 + BASE83.iter().position(|d| d == c).unwrap() as u32
        });
        [n >> 16, (n >> 8) & 255, n & 255]
    }

    /// The mean colour and alpha a ThumbHash decodes to, each 0 to 255.
    fn thumbhash_average(hash: &[u8]) -> [f64; 4] {
        let header = u32::from(hash[0]) | u32::from(hash[1]) << 8 | u32::from(hash[2]) << 16;
        let l = f64::from(header & 63) / 63.0;
        let p = f64::from((header >> 6) & 63) / 31.5 - 1.0;
        let q = f64::from((header >> 12) & 63) / 31.5 - 1.0;
        let a = if header >> 23 == 1 {
            f64::from(hash[5] & 15) / 15.0
        } else {
            1.0
        };
        let b = l - 2.0 / 3.0 * p;
        let r = (3.0 * l - b + q) / 2.0;
        [r, r - q, b, a].map(|c| c.clamp(0.0, 1.0) * 255.0)
    }

    fn near(a: [f64; 4], b: [f64; 4], by: f64) -> bool {
        a.iter().zip(&b).all(|(a, b)| (a - b).abs() <= by)
    }

    #[test]
    fn a_plain_image_has_a_blurhash_of_its_colour() {
        let hash = pixels(&plain(30, 20, [40, 80, 120, 255]), Kind::BlurHash);
        assert_eq!(hash.len(), 28);
        assert!(hash.starts_with('L'), "4×3 components: {hash}");
        assert_eq!(blurhash_average(&hash), [40, 80, 120]);
        assert!(pixels(&plain(20, 30, [0; 4]), Kind::BlurHash).starts_with('T'));
    }

    #[test]
    fn a_blurhash_is_the_mean_in_linear_light() {
        let frame = split(240, 120, 120, [255, 0, 0, 255], [0, 0, 255, 255]);
        let [r, g, b] = blurhash_average(&pixels(&frame, Kind::BlurHash));
        assert!((187..=189).contains(&r) && g == 0 && (187..=189).contains(&b));
    }

    #[test]
    fn a_thumbhash_decodes_to_the_mean_colour_and_alpha() {
        let colour = [40, 80, 120, 255];
        let hash = thumbhash(&shrink(&plain(30, 20, colour)));
        // 7×5 luminance and two 3×3 colour channels keep 22, 5 and 5 AC terms, two to a byte.
        assert_eq!(hash.len(), 5 + 16);
        assert!(near(thumbhash_average(&hash), colour.map(f64::from), 3.0));
        let frame = split(300, 200, 150, [255, 0, 0, 255], [0, 0, 255, 255]);
        let hash = thumbhash(&shrink(&frame));
        assert!(near(
            thumbhash_average(&hash),
            [127.5, 0.0, 127.5, 255.0],
            3.0
        ));
        let half = split(40, 40, 20, [200, 100, 0, 255], [0, 0, 0, 0]);
        let hash = thumbhash(&shrink(&half));
        assert_eq!(hash[2] >> 7, 1);
        assert!(near(
            thumbhash_average(&hash),
            [200.0, 100.0, 0.0, 127.5],
            10.0
        ));
    }

    #[test]
    fn base64_pads_to_a_multiple_of_four() {
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(&[0xfb, 0xff]), "+/8=");
    }

    #[test]
    fn the_palette_of_a_two_colour_image_is_those_colours_most_frequent_first() {
        let (red, blue) = ([220, 20, 60, 255], [30, 60, 200, 255]);
        let frame = split(40, 30, 10, red, blue);
        for count in [2, 5, 64, 256] {
            assert_eq!(colours(&frame, count), [blue, red], "{count}");
        }
        assert_eq!(colours(&frame, 1).len(), 1);
    }

    #[test]
    fn the_palette_has_at_most_count_colours() {
        let mut rgba = Vec::new();
        for y in 0..64u32 {
            for x in 0..64u32 {
                rgba.extend_from_slice(&[(x * 4) as u8, (y * 4) as u8, 128, 255]);
            }
        }
        let frame = Pixels {
            width: 64,
            height: 64,
            rgba,
            exif: None,
        };
        assert!(colours(&frame, 0).is_empty());
        for count in [1, 3, 5, 100] {
            let palette = colours(&frame, count);
            assert!(!palette.is_empty() && palette.len() <= count, "{count}");
        }
        assert!(colours(&plain(4, 4, [9, 9, 9, 0]), 5).is_empty());
    }

    #[test]
    fn both_exports_decode_their_input() {
        let frame = split(48, 32, 16, [255, 255, 0, 255], [0, 128, 0, 255]);
        let png = encode(frame.clone(), Format::Png, Encoding::default()).unwrap();
        for kind in [Kind::BlurHash, Kind::ThumbHash] {
            assert_eq!(placeholder(&png, kind).unwrap(), pixels(&frame, kind));
        }
        assert_eq!(palette(&png, 5).unwrap(), colours(&frame, 5));
        assert!(matches!(palette(&png, 257), Err(Error::Invalid(_))));
        assert!(matches!(
            placeholder(b"not an image", Kind::BlurHash),
            Err(Error::Parse(_))
        ));
    }
}
