//! The pixel steps of a plan (`rule:core-classes/image-pipeline`): each `Op` runs on the decoded
//! RGBA8 frame, straight alpha, in the order the plan lists it.
//!
//! **Every step is checked before anything is decoded** (`check`), so an option out of range
//! returns `Invalid` and costs no decode. **Every frame a step makes is held to the pixel cap
//! before it is allocated**, so a `resize` or a `rotate` cannot grow a frame past what `open`
//! would have accepted (`rule:core-classes/image-pixel-cap`).
//!
//! What each step means where the record leaves it open:
//!
//! - `resize` scales through `fast_image_resize`, in sRGB, premultiplying alpha while it filters.
//!   With one dimension it keeps the aspect. With two, `fit` decides: `Cover` (the default) fills
//!   the box and crops the overflow at `gravity`; `Contain` fits inside the box and places the
//!   result at `gravity` on a transparent canvas the size of the box; `Fill` stretches to the box;
//!   `Inside` and `Outside` keep the aspect and return the scaled frame, no larger or no smaller
//!   than the box. Without `upscale`, no step scales by more than 1, so `Cover` on a small image
//!   returns the uncropped box at its own scale.
//! - `rotate` turns clockwise. A multiple of 90 moves pixels and is exact; any other angle samples
//!   bilinearly onto a canvas that holds the whole turned frame, filled with `background`.
//! - `flip(Horizontal)` swaps left and right, `flip(Vertical)` top and bottom.
//! - `crop` takes a box that lies inside the frame and is not empty.
//! - `trim` removes the border whose pixels are within `threshold` (0 to 255, default 10) of the
//!   top-left pixel in every channel. A frame that is all border is kept whole.
//! - `blur` is a Gaussian of `sigma` over the premultiplied frame. `sharpen` is an unsharp mask of
//!   `sigma` (default 1) with an amount of 1, over the colour channels alone.
//! - `brightness`, `contrast` and `gamma` are each 1 for no change, on the sRGB values of the
//!   colour channels: `brightness` multiplies, `contrast` scales the distance from 128, and
//!   `gamma` raises to `1 / gamma`, so above 1 lightens the mid-tones.
//! - `grayscale` is the Rec. 709 luma of the sRGB values. `tint` multiplies each colour channel
//!   by the tint's, as much as the tint's alpha says. `flatten` composites the frame onto the
//!   background's colour and leaves it opaque; the background's own alpha is not read.
//! - `composite` draws an overlay's frame over the frame, clipped to it. `x` and `y` are the
//!   overlay's top-left corner, either may be negative, and an axis without one is placed at
//!   `gravity` (default `Center`). `opacity` (0 to 1, default 1) scales the overlay's alpha.
//!   `blend` mixes each colour channel `b` below with `s` above, all from 0 to 1: `Normal` is `s`,
//!   `Multiply` is `b·s`, `Screen` is `b + s − b·s`, `Overlay` is `2·b·s` where `b ≤ 0.5` and
//!   `1 − 2·(1 − b)·(1 − s)` elsewhere, `Darken` the smaller and `Lighten` the larger. The mixed
//!   colour then goes over the frame by source-over (the W3C compositing model), so where the
//!   frame is transparent the overlay's own colour shows, and the alpha is `a + b·(1 − a)`. The
//!   overlay is its own pipeline, made by the plan (`crate::variants`).
//!
//! Memory: `resize`, `rotate`, `crop` and `trim` build the result beside the frame, and `blur`
//! and `sharpen` hold one intermediate frame, so a step holds at most two frames at once.
//! `composite` holds the overlay's frame beside the frame, and an overlay that composites
//! another holds one frame more per level. The rest work in place.

use fast_image_resize as fr;

use crate::decode::check as within_cap;
use crate::{Error, Pixels};

/// The `sharpen` sigma with none given.
pub const DEFAULT_SHARPEN_SIGMA: f64 = 1.0;
/// The `trim` threshold with none given.
pub const DEFAULT_TRIM_THRESHOLD: f64 = 10.0;

/// How a `resize` with both dimensions fits the box.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Fit {
    #[default]
    Cover,
    Contain,
    Fill,
    Inside,
    Outside,
}

/// Where a `Cover` resize crops and a `Contain` resize places the frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Gravity {
    #[default]
    Center,
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Gravity {
    /// The share of the spare width and height that goes left of and above the frame.
    fn shares(self) -> (f64, f64) {
        match self {
            Gravity::Center => (0.5, 0.5),
            Gravity::North => (0.5, 0.0),
            Gravity::NorthEast => (1.0, 0.0),
            Gravity::East => (1.0, 0.5),
            Gravity::SouthEast => (1.0, 1.0),
            Gravity::South => (0.5, 1.0),
            Gravity::SouthWest => (0.0, 1.0),
            Gravity::West => (0.0, 0.5),
            Gravity::NorthWest => (0.0, 0.0),
        }
    }
}

/// The resampling filter of a `resize`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Filter {
    Nearest,
    Bilinear,
    CatmullRom,
    Mitchell,
    #[default]
    Lanczos3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// How a `composite` mixes the overlay's colour with the frame's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Blend {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
}

impl Blend {
    /// The mixed channel for `below` under `above`, each from 0 to 1.
    fn mix(self, below: f32, above: f32) -> f32 {
        match self {
            Blend::Normal => above,
            Blend::Multiply => below * above,
            Blend::Screen => below + above - below * above,
            Blend::Overlay if below <= 0.5 => 2.0 * below * above,
            Blend::Overlay => 1.0 - 2.0 * (1.0 - below) * (1.0 - above),
            Blend::Darken => below.min(above),
            Blend::Lighten => below.max(above),
        }
    }
}

/// A `composite` step: where the plan's overlay row `overlay` is drawn, and how.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Composite {
    pub overlay: u64,
    pub gravity: Gravity,
    pub x: Option<i64>,
    pub y: Option<i64>,
    pub opacity: f64,
    pub blend: Blend,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resize {
    pub width: Option<u64>,
    pub height: Option<u64>,
    pub fit: Fit,
    pub gravity: Gravity,
    pub filter: Filter,
    pub upscale: bool,
}

/// One pixel step. A colour is straight-alpha RGBA8.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Resize(Resize),
    Crop {
        x: u64,
        y: u64,
        width: u64,
        height: u64,
    },
    Trim {
        threshold: f64,
    },
    Rotate {
        degrees: f64,
        background: [u8; 4],
    },
    Flip(Axis),
    /// Run by the plan, which makes the overlay's frame and hands it to [`composite`].
    Composite(Composite),
    Flatten([u8; 4]),
    Sharpen {
        sigma: f64,
    },
    Blur {
        sigma: f64,
    },
    Grayscale,
    Brightness(f64),
    Contrast(f64),
    Gamma(f64),
    Tint([u8; 4]),
}

/// The colour `r`, `g`, `b` with `alpha` from 0.0 to 1.0, as RGBA8. A channel over 255 or an
/// alpha outside 0.0 to 1.0 returns `Invalid`.
pub fn color(r: u64, g: u64, b: u64, alpha: f64) -> Result<[u8; 4], Error> {
    let channel = |value: u64| {
        u8::try_from(value).map_err(|_| {
            Error::Invalid(format!(
                "a colour channel is 0 to 255, and this one is {value}"
            ))
        })
    };
    if !(0.0..=1.0).contains(&alpha) {
        return Err(Error::Invalid(format!(
            "a colour's alpha is 0.0 to 1.0, and this one is {alpha}"
        )));
    }
    let alpha = (alpha * 255.0).round() as u8;
    Ok([channel(r)?, channel(g)?, channel(b)?, alpha])
}

/// `Invalid` unless `value` is finite and at least `min`, or above it when `open` is set.
fn bounded(step: &str, value: f64, min: f64, open: bool) -> Result<(), Error> {
    let fits = value.is_finite() && if open { value > min } else { value >= min };
    if fits {
        return Ok(());
    }
    let relation = if open { "above" } else { "at least" };
    Err(Error::Invalid(format!(
        "`{step}` needs a finite number {relation} {min}, and was given {value}"
    )))
}

/// `Invalid` when `op`'s options are out of range. Nothing about the frame is known yet, so a
/// `crop` box is held to the frame by `apply`.
pub fn check(op: &Op) -> Result<(), Error> {
    match *op {
        Op::Resize(resize) => match (resize.width, resize.height) {
            (None, None) => Err(Error::Invalid(
                "`resize` needs a `width`, a `height` or both".to_owned(),
            )),
            (Some(0), _) | (_, Some(0)) => Err(Error::Invalid(
                "`resize` needs a `width` and a `height` above 0".to_owned(),
            )),
            _ => Ok(()),
        },
        Op::Crop { width, height, .. } if width == 0 || height == 0 => Err(Error::Invalid(
            "`crop` needs a `width` and a `height` above 0".to_owned(),
        )),
        Op::Crop { .. } | Op::Flip(_) | Op::Flatten(_) | Op::Grayscale | Op::Tint(_) => Ok(()),
        Op::Trim { threshold } if !(0.0..=255.0).contains(&threshold) => Err(Error::Invalid(
            format!("`trim` needs a threshold of 0 to 255, and was given {threshold}"),
        )),
        Op::Trim { .. } => Ok(()),
        Op::Composite(composite) if !(0.0..=1.0).contains(&composite.opacity) => {
            Err(Error::Invalid(format!(
                "`composite` needs an opacity of 0 to 1, and was given {}",
                composite.opacity
            )))
        }
        Op::Composite(_) => Ok(()),
        Op::Rotate { degrees, .. } => bounded("rotate", degrees, f64::MIN, false),
        Op::Sharpen { sigma } => bounded("sharpen", sigma, 0.0, true),
        Op::Blur { sigma } => bounded("blur", sigma, 0.0, true),
        Op::Brightness(value) => bounded("brightness", value, 0.0, false),
        Op::Contrast(value) => bounded("contrast", value, 0.0, false),
        Op::Gamma(value) => bounded("gamma", value, 0.0, true),
    }
}

/// Runs `op` on `pixels`. `op` has passed `check`; a frame it would make over `cap` pixels
/// returns `Runtime` before it is allocated.
pub fn apply(pixels: &mut Pixels, op: Op, cap: u64) -> Result<(), Error> {
    match op {
        Op::Resize(resize) => self::resize(pixels, resize, cap),
        Op::Crop {
            x,
            y,
            width,
            height,
        } => {
            let fits = x
                .checked_add(width)
                .is_some_and(|right| right <= u64::from(pixels.width))
                && y.checked_add(height)
                    .is_some_and(|bottom| bottom <= u64::from(pixels.height));
            if !fits {
                return Err(Error::Invalid(format!(
                    "`crop` of {width}x{height} at {x}, {y} does not lie inside the {}x{} image",
                    pixels.width, pixels.height
                )));
            }
            crop(pixels, to_u32(x), to_u32(y), to_u32(width), to_u32(height));
            Ok(())
        }
        Op::Trim { threshold } => {
            trim(pixels, threshold);
            Ok(())
        }
        Op::Rotate {
            degrees,
            background,
        } => rotate(pixels, degrees, background, cap),
        Op::Flip(axis) => {
            flip(pixels, axis);
            Ok(())
        }
        Op::Composite(_) => Err(Error::Invalid(
            "a `composite` step runs only inside a plan, which has its overlays".to_owned(),
        )),
        Op::Flatten(background) => {
            for px in pixels.rgba.chunks_exact_mut(4) {
                let alpha = f32::from(px[3]) / 255.0;
                for (c, bg) in px[..3].iter_mut().zip(background) {
                    *c = to_u8(f32::from(*c) * alpha + f32::from(bg) * (1.0 - alpha));
                }
                px[3] = 255;
            }
            Ok(())
        }
        Op::Sharpen { sigma } => {
            gaussian(pixels, sigma, true);
            Ok(())
        }
        Op::Blur { sigma } => {
            gaussian(pixels, sigma, false);
            Ok(())
        }
        Op::Grayscale => {
            for px in pixels.rgba.chunks_exact_mut(4) {
                let luma = 0.2126 * f32::from(px[0])
                    + 0.7152 * f32::from(px[1])
                    + 0.0722 * f32::from(px[2]);
                let luma = to_u8(luma);
                px[..3].fill(luma);
            }
            Ok(())
        }
        Op::Brightness(factor) => {
            curve(pixels, |c| c * factor);
            Ok(())
        }
        Op::Contrast(factor) => {
            curve(pixels, |c| (c - 128.0) * factor + 128.0);
            Ok(())
        }
        Op::Gamma(gamma) => {
            curve(pixels, |c| 255.0 * (c / 255.0).powf(1.0 / gamma));
            Ok(())
        }
        Op::Tint(tint) => {
            let strength = f32::from(tint[3]) / 255.0;
            let gains = [0, 1, 2].map(|at| 1.0 - strength + strength * f32::from(tint[at]) / 255.0);
            for px in pixels.rgba.chunks_exact_mut(4) {
                for (c, gain) in px[..3].iter_mut().zip(gains) {
                    *c = to_u8(f32::from(*c) * gain);
                }
            }
            Ok(())
        }
    }
}

/// Draws `overlay` over `pixels` where `options` places it, clipped to `pixels`.
pub fn composite(pixels: &mut Pixels, overlay: &Pixels, options: Composite) {
    let (share_x, share_y) = options.gravity.shares();
    let at = |given: Option<i64>, size: u32, inner: u32, share: f64| {
        given.unwrap_or_else(|| ((f64::from(size) - f64::from(inner)) * share).round() as i64)
    };
    let left = at(options.x, pixels.width, overlay.width, share_x);
    let top = at(options.y, pixels.height, overlay.height, share_y);
    let opacity = options.opacity as f32;
    let width = usize::try_from(pixels.width).expect("a u32 fits in a usize");
    let inner = usize::try_from(overlay.width).expect("a u32 fits in a usize");
    for (row, line) in overlay.rgba.chunks_exact(inner * 4).enumerate() {
        let Ok(y) = u32::try_from(top + row as i64) else {
            continue;
        };
        if y >= pixels.height {
            break;
        }
        for (column, above) in line.chunks_exact(4).enumerate() {
            let Ok(x) = u32::try_from(left + column as i64) else {
                continue;
            };
            if x >= pixels.width {
                break;
            }
            let at = (y as usize * width + x as usize) * 4;
            let below = &mut pixels.rgba[at..at + 4];
            let a = f32::from(above[3]) / 255.0 * opacity;
            let b = f32::from(below[3]) / 255.0;
            let out = a + b * (1.0 - a);
            if out == 0.0 {
                below.fill(0);
                continue;
            }
            for c in 0..3 {
                let s = f32::from(above[c]) / 255.0;
                let d = f32::from(below[c]) / 255.0;
                let mixed = options.blend.mix(d, s);
                let premultiplied = a * (1.0 - b) * s + a * b * mixed + (1.0 - a) * b * d;
                below[c] = to_u8(premultiplied / out * 255.0);
            }
            below[3] = to_u8(out * 255.0);
        }
    }
}

fn to_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

/// A size already held to a `u32` frame.
fn to_u32(value: u64) -> u32 {
    u32::try_from(value).expect("a size inside the frame fits in a u32")
}

/// A scaled size, at least 1.
fn scale(size: u32, by: f64) -> u64 {
    ((f64::from(size) * by).round() as u64).max(1)
}

/// `Runtime` when a `width` by `height` frame is over `cap`, and the size as `u32`s otherwise.
fn frame(width: u64, height: u64, cap: u64) -> Result<(u32, u32), Error> {
    within_cap(width, height, cap)?;
    match (u32::try_from(width), u32::try_from(height)) {
        (Ok(width), Ok(height)) => Ok((width, height)),
        _ => Err(Error::Runtime(format!(
            "the image would be {width}x{height}, too large"
        ))),
    }
}

/// Applies `map` to every colour channel through a table of the 256 values.
fn curve(pixels: &mut Pixels, map: impl Fn(f64) -> f64) {
    let table: [u8; 256] = std::array::from_fn(|at| to_u8(map(at as f64) as f32));
    for px in pixels.rgba.chunks_exact_mut(4) {
        for c in &mut px[..3] {
            *c = table[usize::from(*c)];
        }
    }
}

fn resize(pixels: &mut Pixels, options: Resize, cap: u64) -> Result<(), Error> {
    let (width, height) = (pixels.width, pixels.height);
    let limit = |by: f64| if options.upscale { by } else { by.min(1.0) };
    let both = |by: f64| (scale(width, by), scale(height, by));
    // The size of the resized frame, the source box it is read from when that is not the whole
    // frame, and the canvas a `Contain` result is placed on.
    let (size, source, canvas) = match (options.width, options.height) {
        (Some(w), None) => (both(limit(w as f64 / f64::from(width))), None, None),
        (None, Some(h)) => (both(limit(h as f64 / f64::from(height))), None, None),
        (Some(w), Some(h)) => {
            let (by_x, by_y) = (w as f64 / f64::from(width), h as f64 / f64::from(height));
            match options.fit {
                Fit::Fill if options.upscale => ((w, h), None, None),
                Fit::Fill => (
                    (w.min(u64::from(width)), h.min(u64::from(height))),
                    None,
                    None,
                ),
                Fit::Inside => (both(limit(by_x.min(by_y))), None, None),
                Fit::Outside => (both(limit(by_x.max(by_y))), None, None),
                Fit::Contain => (both(limit(by_x.min(by_y))), None, Some((w, h))),
                Fit::Cover => {
                    let by = by_x.max(by_y);
                    let (box_w, box_h) = (w as f64 / by, h as f64 / by);
                    let (share_x, share_y) = options.gravity.shares();
                    let left = (f64::from(width) - box_w) * share_x;
                    let top = (f64::from(height) - box_h) * share_y;
                    let size = if by > 1.0 && !options.upscale {
                        ((box_w.round() as u64).max(1), (box_h.round() as u64).max(1))
                    } else {
                        (w, h)
                    };
                    (size, Some((left, top, box_w, box_h)), None)
                }
            }
        }
        (None, None) => unreachable!("`check` refuses a resize with no dimension"),
    };
    let (out_w, out_h) = frame(size.0, size.1, cap)?;
    if let Some((canvas_w, canvas_h)) = canvas {
        frame(canvas_w, canvas_h, cap)?;
    }
    if (out_w, out_h) != (width, height) || source.is_some() {
        let algorithm = match options.filter {
            Filter::Nearest => fr::ResizeAlg::Nearest,
            Filter::Bilinear => fr::ResizeAlg::Convolution(fr::FilterType::Bilinear),
            Filter::CatmullRom => fr::ResizeAlg::Convolution(fr::FilterType::CatmullRom),
            Filter::Mitchell => fr::ResizeAlg::Convolution(fr::FilterType::Mitchell),
            Filter::Lanczos3 => fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3),
        };
        let mut resize_options = fr::ResizeOptions::new().resize_alg(algorithm);
        if let Some((left, top, box_w, box_h)) = source {
            resize_options = resize_options.crop(left, top, box_w, box_h);
        }
        let rgba = std::mem::take(&mut pixels.rgba);
        let from = fr::images::Image::from_vec_u8(width, height, rgba, fr::PixelType::U8x4)
            .map_err(|err| Error::Runtime(format!("the frame does not resize: {err}")))?;
        let mut to = fr::images::Image::new(out_w, out_h, fr::PixelType::U8x4);
        fr::Resizer::new()
            .resize(&from, &mut to, &resize_options)
            .map_err(|err| Error::Runtime(format!("the frame does not resize: {err}")))?;
        drop(from);
        pixels.rgba = to.into_vec();
        pixels.width = out_w;
        pixels.height = out_h;
    }
    if let Some((canvas_w, canvas_h)) = canvas {
        place(pixels, to_u32(canvas_w), to_u32(canvas_h), options.gravity);
    }
    Ok(())
}

/// Places `pixels` on a transparent `width` by `height` canvas at `gravity`. The frame is no
/// larger than the canvas.
fn place(pixels: &mut Pixels, width: u32, height: u32, gravity: Gravity) {
    if (width, height) == (pixels.width, pixels.height) {
        return;
    }
    let (share_x, share_y) = gravity.shares();
    let left = (f64::from(width - pixels.width) * share_x).round() as usize;
    let top = (f64::from(height - pixels.height) * share_y).round() as usize;
    let (row, frame_row) = (width as usize * 4, pixels.width as usize * 4);
    let mut canvas = vec![0; row * height as usize];
    for (y, line) in pixels.rgba.chunks_exact(frame_row).enumerate() {
        let at = (top + y) * row + left * 4;
        canvas[at..at + frame_row].copy_from_slice(line);
    }
    pixels.rgba = canvas;
    pixels.width = width;
    pixels.height = height;
}

fn crop(pixels: &mut Pixels, x: u32, y: u32, width: u32, height: u32) {
    if (x, y, width, height) == (0, 0, pixels.width, pixels.height) {
        return;
    }
    let row = pixels.width as usize * 4;
    let (left, part) = (x as usize * 4, width as usize * 4);
    let mut out = Vec::with_capacity(part * height as usize);
    for line in pixels
        .rgba
        .chunks_exact(row)
        .skip(y as usize)
        .take(height as usize)
    {
        out.extend_from_slice(&line[left..left + part]);
    }
    pixels.rgba = out;
    pixels.width = width;
    pixels.height = height;
}

fn trim(pixels: &mut Pixels, threshold: f64) {
    let corner: [u8; 4] = pixels.rgba[..4].try_into().expect("a frame has a pixel");
    let row = pixels.width as usize * 4;
    let mut bounds: Option<(usize, usize, usize, usize)> = None;
    for (y, line) in pixels.rgba.chunks_exact(row).enumerate() {
        for (x, px) in line.chunks_exact(4).enumerate() {
            let differs = px
                .iter()
                .zip(corner)
                .any(|(c, k)| f64::from(c.abs_diff(k)) > threshold);
            if differs {
                bounds = Some(match bounds {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
        }
    }
    if let Some((x0, y0, x1, y1)) = bounds {
        let size = |value: usize| u32::try_from(value).expect("inside the frame");
        crop(
            pixels,
            size(x0),
            size(y0),
            size(x1 - x0 + 1),
            size(y1 - y0 + 1),
        );
    }
}

fn flip(pixels: &mut Pixels, axis: Axis) {
    let row = pixels.width as usize * 4;
    match axis {
        Axis::Horizontal => {
            for line in pixels.rgba.chunks_exact_mut(row) {
                let width = line.len() / 4;
                for x in 0..width / 2 {
                    for c in 0..4 {
                        line.swap(x * 4 + c, (width - 1 - x) * 4 + c);
                    }
                }
            }
        }
        Axis::Vertical => {
            let height = pixels.height as usize;
            for y in 0..height / 2 {
                let (top, bottom) = pixels.rgba.split_at_mut((height - 1 - y) * row);
                top[y * row..(y + 1) * row].swap_with_slice(&mut bottom[..row]);
            }
        }
    }
}

fn rotate(pixels: &mut Pixels, degrees: f64, background: [u8; 4], cap: u64) -> Result<(), Error> {
    let turn = degrees.rem_euclid(360.0);
    let quarters = (turn / 90.0).round();
    if (turn - quarters * 90.0).abs() < 1e-9 {
        match quarters as u32 % 4 {
            0 => {}
            2 => {
                let mut quads: Vec<[u8; 4]> = pixels
                    .rgba
                    .chunks_exact(4)
                    .map(|px| px.try_into().expect("four bytes"))
                    .collect();
                quads.reverse();
                pixels.rgba = quads.concat();
            }
            quarter => {
                let (width, height) = (pixels.width as usize, pixels.height as usize);
                let mut out = vec![0; pixels.rgba.len()];
                for y in 0..height {
                    for x in 0..width {
                        // Clockwise, a source pixel's row becomes its column counted from the
                        // right; anticlockwise, its column becomes its row counted from the bottom.
                        let (to_x, to_y) = if quarter == 1 {
                            (height - 1 - y, x)
                        } else {
                            (y, width - 1 - x)
                        };
                        let from = (y * width + x) * 4;
                        let to = (to_y * height + to_x) * 4;
                        out[to..to + 4].copy_from_slice(&pixels.rgba[from..from + 4]);
                    }
                }
                pixels.rgba = out;
                std::mem::swap(&mut pixels.width, &mut pixels.height);
            }
        }
        return Ok(());
    }
    let (sin, cos) = turn.to_radians().sin_cos();
    let (width, height) = (f64::from(pixels.width), f64::from(pixels.height));
    let (out_w, out_h) = (
        ((width * cos.abs() + height * sin.abs()) - 1e-6)
            .ceil()
            .max(1.0) as u64,
        ((width * sin.abs() + height * cos.abs()) - 1e-6)
            .ceil()
            .max(1.0) as u64,
    );
    let (out_w, out_h) = frame(out_w, out_h, cap)?;
    let premultiplied = |px: &[u8]| {
        let alpha = f32::from(px[3]) / 255.0;
        [
            f32::from(px[0]) * alpha,
            f32::from(px[1]) * alpha,
            f32::from(px[2]) * alpha,
            f32::from(px[3]),
        ]
    };
    let fill = premultiplied(&background);
    let (w, h) = (pixels.width as i64, pixels.height as i64);
    let sample = |x: i64, y: i64| {
        if (0..w).contains(&x) && (0..h).contains(&y) {
            let at = ((y * w + x) * 4) as usize;
            premultiplied(&pixels.rgba[at..at + 4])
        } else {
            fill
        }
    };
    let (sin, cos) = (sin as f32, cos as f32);
    let (half_w, half_h) = (width as f32 / 2.0, height as f32 / 2.0);
    let (out_half_w, out_half_h) = (out_w as f32 / 2.0, out_h as f32 / 2.0);
    let mut out = Vec::with_capacity(out_w as usize * out_h as usize * 4);
    for j in 0..out_h {
        for i in 0..out_w {
            // The centre of the output pixel, turned back anticlockwise into the source frame,
            // in coordinates where a pixel's centre is a whole number.
            let (dx, dy) = (i as f32 + 0.5 - out_half_w, j as f32 + 0.5 - out_half_h);
            let sx = dx * cos + dy * sin + half_w - 0.5;
            let sy = -dx * sin + dy * cos + half_h - 0.5;
            let (x0, y0) = (sx.floor(), sy.floor());
            let (fx, fy) = (sx - x0, sy - y0);
            let (x0, y0) = (x0 as i64, y0 as i64);
            let corners = [
                (sample(x0, y0), (1.0 - fx) * (1.0 - fy)),
                (sample(x0 + 1, y0), fx * (1.0 - fy)),
                (sample(x0, y0 + 1), (1.0 - fx) * fy),
                (sample(x0 + 1, y0 + 1), fx * fy),
            ];
            let mut px = [0.0f32; 4];
            for (corner, weight) in corners {
                for (sum, c) in px.iter_mut().zip(corner) {
                    *sum += c * weight;
                }
            }
            out.extend_from_slice(&straight(px));
        }
    }
    pixels.rgba = out;
    pixels.width = out_w;
    pixels.height = out_h;
    Ok(())
}

/// A premultiplied pixel, as straight RGBA8.
fn straight(px: [f32; 4]) -> [u8; 4] {
    let alpha = to_u8(px[3]);
    if alpha == 0 {
        return [0; 4];
    }
    let scale = 255.0 / px[3];
    [
        to_u8(px[0] * scale),
        to_u8(px[1] * scale),
        to_u8(px[2] * scale),
        alpha,
    ]
}

/// The normalised weights of a Gaussian of `sigma`, `radius` taps either side of the centre.
fn kernel(sigma: f64, radius: usize) -> Vec<f32> {
    let weights: Vec<f32> = (0..=2 * radius)
        .map(|at| {
            let distance = at.abs_diff(radius) as f64;
            (-(distance * distance) / (2.0 * sigma * sigma)).exp() as f32
        })
        .collect();
    let total: f32 = weights.iter().sum();
    weights.into_iter().map(|weight| weight / total).collect()
}

/// A Gaussian blur of `sigma`, or with `sharpen` an unsharp mask that adds back the difference
/// between each colour channel and its blur. Both run over premultiplied pixels when any pixel
/// is translucent, and read the edge pixel past each edge.
fn gaussian(pixels: &mut Pixels, sigma: f64, sharpen: bool) {
    let (width, height) = (pixels.width as usize, pixels.height as usize);
    let radius = ((3.0 * sigma).ceil() as usize).clamp(1, width.max(height));
    let weights = kernel(sigma, radius);
    let translucent = pixels.rgba.chunks_exact(4).any(|px| px[3] != 255);
    if translucent {
        for px in pixels.rgba.chunks_exact_mut(4) {
            let alpha = f32::from(px[3]) / 255.0;
            for c in &mut px[..3] {
                *c = to_u8(f32::from(*c) * alpha);
            }
        }
    }
    let row = width * 4;
    // The index `at - radius`, held inside `0..len`.
    let edge = |at: usize, len: usize| at.saturating_sub(radius).min(len - 1);
    // Across each row, into `across`.
    let mut across = vec![0u8; pixels.rgba.len()];
    let mut sums = vec![0.0f32; row];
    for (line, out) in pixels
        .rgba
        .chunks_exact(row)
        .zip(across.chunks_exact_mut(row))
    {
        sums.fill(0.0);
        for x in 0..width {
            for (tap, weight) in weights.iter().enumerate() {
                let from = edge(x + tap, width) * 4;
                for c in 0..4 {
                    sums[x * 4 + c] += f32::from(line[from + c]) * weight;
                }
            }
        }
        for (o, sum) in out.iter_mut().zip(&sums) {
            *o = to_u8(*sum);
        }
    }
    // Down each column of `across`, a row at a time, back into the frame.
    for y in 0..height {
        sums.fill(0.0);
        for (tap, weight) in weights.iter().enumerate() {
            let from = edge(y + tap, height) * row;
            for (sum, c) in sums.iter_mut().zip(&across[from..from + row]) {
                *sum += f32::from(*c) * weight;
            }
        }
        let line = &mut pixels.rgba[y * row..(y + 1) * row];
        for (at, (c, blurred)) in line.iter_mut().zip(&sums).enumerate() {
            if !sharpen {
                *c = to_u8(*blurred);
            } else if at % 4 != 3 {
                let value = f32::from(*c);
                *c = to_u8(value + (value - blurred));
            }
        }
    }
    if translucent {
        for px in pixels.rgba.chunks_exact_mut(4) {
            let [r, g, b, a] = [px[0], px[1], px[2], px[3]].map(f32::from);
            px.copy_from_slice(&straight([r, g, b, a]));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `width` by `height` frame whose pixel at `x`, `y` is `[x, y, 0, 255]`.
    fn numbered(width: u32, height: u32) -> Pixels {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                rgba.extend_from_slice(&[x as u8, y as u8, 0, 255]);
            }
        }
        Pixels {
            width,
            height,
            rgba,
            exif: None,
        }
    }

    fn at(pixels: &Pixels, x: u32, y: u32) -> [u8; 4] {
        let from = ((y * pixels.width + x) * 4) as usize;
        pixels.rgba[from..from + 4].try_into().unwrap()
    }

    fn sized(width: Option<u64>, height: Option<u64>, fit: Fit) -> Op {
        Op::Resize(Resize {
            width,
            height,
            fit,
            ..Resize::default()
        })
    }

    #[test]
    fn a_quarter_turn_clockwise_moves_the_top_left_to_the_top_right() {
        let mut pixels = numbered(3, 2);
        apply(
            &mut pixels,
            Op::Rotate {
                degrees: 90.0,
                background: [0; 4],
            },
            1 << 20,
        )
        .unwrap();
        assert_eq!((pixels.width, pixels.height), (2, 3));
        assert_eq!(at(&pixels, 1, 0), [0, 0, 0, 255]);
        assert_eq!(at(&pixels, 0, 2), [2, 1, 0, 255]);
        let mut back = pixels.clone();
        apply(
            &mut back,
            Op::Rotate {
                degrees: -90.0,
                background: [0; 4],
            },
            1 << 20,
        )
        .unwrap();
        assert_eq!(back.rgba, numbered(3, 2).rgba);
    }

    #[test]
    fn a_half_turn_and_two_flips_agree() {
        let mut turned = numbered(4, 3);
        apply(
            &mut turned,
            Op::Rotate {
                degrees: 180.0,
                background: [0; 4],
            },
            1 << 20,
        )
        .unwrap();
        let mut flipped = numbered(4, 3);
        apply(&mut flipped, Op::Flip(Axis::Horizontal), 1 << 20).unwrap();
        apply(&mut flipped, Op::Flip(Axis::Vertical), 1 << 20).unwrap();
        assert_eq!(turned.rgba, flipped.rgba);
        assert_eq!(at(&turned, 0, 0), [3, 2, 0, 255]);
    }

    #[test]
    fn an_odd_angle_grows_the_canvas_and_fills_the_corners() {
        let mut pixels = Pixels {
            width: 10,
            height: 10,
            rgba: [255, 0, 0, 255].repeat(100),
            exif: None,
        };
        apply(
            &mut pixels,
            Op::Rotate {
                degrees: 45.0,
                background: [0, 0, 255, 255],
            },
            1 << 20,
        )
        .unwrap();
        assert_eq!((pixels.width, pixels.height), (15, 15));
        assert_eq!(at(&pixels, 0, 0), [0, 0, 255, 255]);
        assert_eq!(at(&pixels, 7, 7), [255, 0, 0, 255]);
    }

    #[test]
    fn resize_fits_the_box_as_each_fit_says() {
        let cases = [
            (Fit::Cover, (8, 8)),
            (Fit::Contain, (8, 8)),
            (Fit::Fill, (8, 8)),
            (Fit::Inside, (8, 4)),
            (Fit::Outside, (16, 8)),
        ];
        for (fit, want) in cases {
            let mut pixels = numbered(40, 20);
            apply(&mut pixels, sized(Some(8), Some(8), fit), 1 << 20).unwrap();
            assert_eq!((pixels.width, pixels.height), want, "{fit:?}");
        }
        let mut pixels = numbered(40, 20);
        apply(&mut pixels, sized(Some(8), Some(8), Fit::Contain), 1 << 20).unwrap();
        assert_eq!(at(&pixels, 0, 0)[3], 0, "the band above is transparent");
        assert_eq!(at(&pixels, 0, 4)[3], 255);
    }

    #[test]
    fn one_dimension_keeps_the_aspect_and_no_upscale_keeps_the_size() {
        let mut pixels = numbered(40, 20);
        apply(&mut pixels, sized(Some(10), None, Fit::Cover), 1 << 20).unwrap();
        assert_eq!((pixels.width, pixels.height), (10, 5));
        let mut small = numbered(4, 2);
        apply(&mut small, sized(None, Some(20), Fit::Cover), 1 << 20).unwrap();
        assert_eq!((small.width, small.height), (4, 2));
        apply(
            &mut small,
            Op::Resize(Resize {
                height: Some(20),
                upscale: true,
                ..Resize::default()
            }),
            1 << 20,
        )
        .unwrap();
        assert_eq!((small.width, small.height), (40, 20));
    }

    #[test]
    fn a_frame_over_the_cap_is_refused_before_it_is_made() {
        let mut pixels = numbered(4, 4);
        let grow = Op::Resize(Resize {
            width: Some(100),
            upscale: true,
            ..Resize::default()
        });
        assert!(matches!(
            apply(&mut pixels, grow, 1000),
            Err(Error::Runtime(_))
        ));
        assert_eq!((pixels.width, pixels.height), (4, 4));
    }

    #[test]
    fn crop_takes_the_box_and_refuses_one_outside_the_frame() {
        let mut pixels = numbered(6, 6);
        apply(
            &mut pixels,
            Op::Crop {
                x: 2,
                y: 1,
                width: 3,
                height: 2,
            },
            1 << 20,
        )
        .unwrap();
        assert_eq!((pixels.width, pixels.height), (3, 2));
        assert_eq!(at(&pixels, 0, 0), [2, 1, 0, 255]);
        let outside = Op::Crop {
            x: 2,
            y: 0,
            width: 2,
            height: 1,
        };
        assert!(matches!(
            apply(&mut pixels, outside, 1 << 20),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn trim_removes_a_uniform_border() {
        let mut rgba = [255, 255, 255, 255].repeat(36);
        for (x, y) in [(2, 1), (3, 4)] {
            rgba[(y * 6 + x) * 4..(y * 6 + x) * 4 + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
        let mut pixels = Pixels {
            width: 6,
            height: 6,
            rgba,
            exif: None,
        };
        apply(
            &mut pixels,
            Op::Trim {
                threshold: DEFAULT_TRIM_THRESHOLD,
            },
            1 << 20,
        )
        .unwrap();
        assert_eq!((pixels.width, pixels.height), (2, 4));
    }

    #[test]
    fn blur_keeps_a_flat_frame_and_softens_an_edge() {
        let mut flat = Pixels {
            width: 5,
            height: 5,
            rgba: [40, 80, 120, 255].repeat(25),
            exif: None,
        };
        apply(&mut flat, Op::Blur { sigma: 2.0 }, 1 << 20).unwrap();
        assert!(flat.rgba.chunks_exact(4).all(|px| px == [40, 80, 120, 255]));
        let mut edge = Pixels {
            width: 8,
            height: 1,
            rgba: Vec::new(),
            exif: None,
        };
        for x in 0..8 {
            edge.rgba.extend_from_slice(if x < 4 {
                &[0, 0, 0, 255]
            } else {
                &[255, 255, 255, 255]
            });
        }
        let mut sharp = edge.clone();
        apply(&mut edge, Op::Blur { sigma: 1.0 }, 1 << 20).unwrap();
        assert!(
            (1..255).contains(&at(&edge, 3, 0)[0]),
            "{:?}",
            at(&edge, 3, 0)
        );
        apply(&mut sharp, Op::Sharpen { sigma: 1.0 }, 1 << 20).unwrap();
        assert_eq!(at(&sharp, 3, 0)[0], 0);
        assert_eq!(at(&sharp, 0, 0), [0, 0, 0, 255]);
    }

    #[test]
    fn the_adjustments_leave_alpha_and_one_leaves_the_colour() {
        for op in [
            Op::Brightness(1.0),
            Op::Contrast(1.0),
            Op::Gamma(1.0),
            Op::Tint([9, 9, 9, 0]),
        ] {
            let mut pixels = numbered(4, 4);
            apply(&mut pixels, op, 1 << 20).unwrap();
            assert_eq!(pixels.rgba, numbered(4, 4).rgba, "{op:?}");
        }
        let mut pixels = Pixels {
            width: 1,
            height: 1,
            rgba: vec![200, 100, 50, 128],
            exif: None,
        };
        apply(&mut pixels, Op::Brightness(0.5), 1 << 20).unwrap();
        assert_eq!(pixels.rgba, [100, 50, 25, 128]);
        apply(&mut pixels, Op::Grayscale, 1 << 20).unwrap();
        assert_eq!(pixels.rgba, [59, 59, 59, 128]);
        apply(&mut pixels, Op::Flatten([255, 255, 255, 255]), 1 << 20).unwrap();
        assert_eq!(pixels.rgba, [157, 157, 157, 255]);
    }

    #[test]
    fn an_option_out_of_range_is_invalid() {
        for op in [
            Op::Blur { sigma: 0.0 },
            Op::Gamma(0.0),
            Op::Brightness(-1.0),
            Op::Contrast(f64::NAN),
            Op::Trim { threshold: 300.0 },
            Op::Rotate {
                degrees: f64::INFINITY,
                background: [0; 4],
            },
            sized(None, None, Fit::Cover),
            sized(Some(0), Some(4), Fit::Cover),
            Op::Crop {
                x: 0,
                y: 0,
                width: 0,
                height: 1,
            },
        ] {
            assert!(matches!(check(&op), Err(Error::Invalid(_))), "{op:?}");
        }
        assert!(matches!(color(256, 0, 0, 1.0), Err(Error::Invalid(_))));
        assert_eq!(color(1, 2, 3, 1.0), Ok([1, 2, 3, 255]));
    }
}
