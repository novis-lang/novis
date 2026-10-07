//! The image component: `nvs:image/codec` from `wit/image.wit`, built into every `nvs` binary
//! (`rule:packaging/the-first-party-components-are-built-in`, `rule:core-classes/image-pipeline`).
//!
//! The crate has two halves. The codec core is plain Rust that builds for the host too, so its
//! tests run under an ordinary `cargo test` in this directory. The WIT glue in `guest` is compiled
//! only for a wasm target, and maps each export onto the core and the core's types onto WIT's.
//!
//! `info`, `run`, `variants`, `compare`, `hash`, `placeholder`, `palette` and `qr` are the
//! exports that are implemented; `compare`, `hash`, `summary` and `qr` are their modules' docs.
//! `run` starts
//! from an
//! [`Input`] — encoded bytes it decodes (`decode`'s module doc), a blank canvas or RGBA8 rows —
//! runs the plan's pixel steps on the frame in order (`ops`'s module doc), and returns the pixels,
//! their size or the encoded file (`encode`'s module doc); `variants` makes the frame once and
//! runs each of its plans on a copy of it. A canvas or a pixel source has no input format, so an
//! encoded output with no `format` step is PNG, and its size is held to the same pixel cap as a
//! decode. A `composite` step names a row of the plan's overlays, each a source and steps of its
//! own: the row's frame is made and run when the step runs, and drawn over the frame
//! (`ops::composite`). A row's own `composite` steps name only earlier rows, and one plan draws
//! at most [`MAX_OVERLAY_DRAWS`] overlays counting every level, both checked before anything is
//! decoded. So is the size of every frame the plan and each overlay row makes, from the source's
//! size and `ops::size`, up to the first step whose size depends on the pixels: a plan that grows
//! its frame past the cap returns `Runtime` before its first step runs, not after its last one
//! under the cap. An encoded source's size is its header's, and a `text` source's is its laid-out
//! box (`text`'s module doc, which says what the `text` step draws too).

mod avif;
pub mod compare;
mod decode;
mod encode;
pub mod hash;
mod info;
pub mod ops;
pub mod qr;
pub mod summary;
pub mod text;
mod webp;

pub use decode::{DEFAULT_MAX_PIXELS, Pixels, decode, sniff};
pub use encode::{DEFAULT_JPEG_QUALITY, encode};
pub use info::{Info, info};
pub use ops::Op;

/// The most overlays one plan draws, counting each draw of a row and every level inside it. An
/// overlay drawn twice inside one drawn twice is four draws, so this bounds the work a plan of a
/// few rows could otherwise double at every level.
pub const MAX_OVERLAY_DRAWS: u64 = 64;

/// What `run` returns, the WIT `output` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    Encoded,
    Raw,
    Size,
}

/// How an `encoded` output is written, read off the plan's `format` and `metadata` steps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Encoding {
    /// The `format` step's format. Without one, the output is in the input's format.
    pub format: Option<Format>,
    pub quality: Option<u64>,
    pub lossless: Option<bool>,
    /// Set by `metadata({keep: true})`. Without it, the output carries no metadata.
    pub keep_metadata: bool,
}

/// A plan: the pixel steps in order, the overlays its `composite` steps name, how the result is
/// encoded, and what `run` returns.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan<'a> {
    pub steps: Vec<Op<'a>>,
    pub overlays: Vec<Overlay<'a>>,
    pub encoding: Encoding,
    pub output: Output,
}

/// A row of a plan's overlays: a pipeline of its own, held to `cap` pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Overlay<'a> {
    pub input: Input<'a>,
    pub cap: u64,
    pub steps: Vec<Op<'a>>,
}

/// `Invalid` unless every `composite` step in `steps` names a row before `rows`, and the draws
/// it makes; `draws[row]` is what drawing that row once costs.
fn draws(steps: &[Op], rows: usize, draws: &[u64]) -> Result<u64, Error> {
    let mut total = 0u64;
    for op in steps {
        ops::check(op)?;
        if let Op::Composite(composite) = op {
            let row = usize::try_from(composite.overlay)
                .ok()
                .filter(|&row| row < rows)
                .ok_or_else(|| {
                    Error::Invalid(format!(
                        "`composite` names overlay {}, and only {rows} come before it",
                        composite.overlay
                    ))
                })?;
            total = total.saturating_add(draws[row]);
        }
    }
    Ok(total)
}

/// `Invalid` when a step of `plan` or of one of its overlays is out of range, a `composite` step
/// names a row that is not before it, or the plan draws more than [`MAX_OVERLAY_DRAWS`] overlays.
fn check(plan: &Plan<'_>) -> Result<(), Error> {
    let mut costs = Vec::with_capacity(plan.overlays.len());
    for (row, overlay) in plan.overlays.iter().enumerate() {
        let cost = draws(&overlay.steps, row, &costs)?.saturating_add(1);
        costs.push(cost);
    }
    let total = draws(&plan.steps, plan.overlays.len(), &costs)?;
    if total > MAX_OVERLAY_DRAWS {
        return Err(Error::Invalid(format!(
            "a plan draws at most {MAX_OVERLAY_DRAWS} overlays, counting the ones inside other overlays, and this one draws {total}"
        )));
    }
    Ok(())
}

/// Where a pipeline starts: the WIT `source` cases this component runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Input<'a> {
    /// Encoded bytes, decoded with the EXIF orientation applied when `auto_orient` is set and an
    /// embedded ICC profile converted to sRGB when `to_srgb` is.
    Encoded {
        data: &'a [u8],
        auto_orient: bool,
        to_srgb: bool,
    },
    /// A blank canvas in one colour.
    Canvas {
        width: u64,
        height: u64,
        fill: [u8; 4],
    },
    /// RGBA8 rows, four bytes per pixel.
    Pixels {
        width: u64,
        height: u64,
        rgba: &'a [u8],
    },
    /// Text set in a font, on a transparent canvas the size of its box.
    Text(text::Layout<'a>),
}

/// Runs `plan` on `input` under `cap` pixels and returns its output. A format the component does
/// not encode and a step option out of range return `Invalid` before anything is decoded.
pub fn run(input: &Input<'_>, cap: u64, plan: &Plan<'_>) -> Result<Vec<u8>, Error> {
    let mut outs = variants(input, cap, std::slice::from_ref(plan))?;
    Ok(outs.pop().unwrap_or_default())
}

/// Runs each of `plans` as `run` does, on one frame made from `input`, and returns their outputs
/// in order. Every plan is checked before anything is decoded. Each plan but the last works on
/// its own copy of the frame, so a call holds two frames at once, and a step that builds its
/// result beside the frame holds a third while it runs.
pub fn variants(input: &Input<'_>, cap: u64, plans: &[Plan<'_>]) -> Result<Vec<Vec<u8>>, Error> {
    let format = match *input {
        Input::Encoded { data, .. } => sniff(data)?,
        Input::Canvas { .. } | Input::Pixels { .. } | Input::Text(_) => Format::Png,
    };
    for plan in plans {
        let target = plan.encoding.format.unwrap_or(format);
        if plan.output == Output::Encoded
            && matches!(target, Format::Jxl | Format::Svg | Format::Pdf)
        {
            return Err(Error::Invalid(format!(
                "{target:?} is a format the image component decodes and does not encode"
            )));
        }
        check(plan)?;
    }
    for plan in plans {
        sizes(known_size(input, cap), &plan.steps, cap)?;
        for overlay in &plan.overlays {
            sizes(
                known_size(&overlay.input, overlay.cap),
                &overlay.steps,
                overlay.cap,
            )?;
        }
    }
    let Some((last, rest)) = plans.split_last() else {
        return Ok(Vec::new());
    };
    let pixels = frame(input, cap)?;
    let mut outs = Vec::with_capacity(plans.len());
    for plan in rest {
        outs.push(finish(pixels.clone(), format, plan, cap)?);
    }
    outs.push(finish(pixels, format, last, cap)?);
    Ok(outs)
}

/// The size of the frame `input` makes, when it is known before anything is decoded and within
/// `cap`. An encoded source's size is read from its header, turned as `auto_orient` turns it.
fn known_size(input: &Input<'_>, cap: u64) -> Option<(u32, u32)> {
    let (width, height) = match *input {
        Input::Canvas { width, height, .. } | Input::Pixels { width, height, .. } => {
            (width, height)
        }
        Input::Text(layout) => text::measure(&layout).ok()?,
        Input::Encoded {
            data, auto_orient, ..
        } => {
            let info = info(data).ok()?;
            if auto_orient && info.orientation >= 5 {
                (info.height, info.width)
            } else {
                (info.width, info.height)
            }
        }
    };
    if width == 0 || height == 0 || decode::check(width, height, cap).is_err() {
        return None;
    }
    Some((u32::try_from(width).ok()?, u32::try_from(height).ok()?))
}

/// `Runtime` when a step of `steps` would make a frame over `cap` from a frame of `start`, found
/// from the sizes alone before any step runs. It stops at the first step whose size depends on
/// the pixels, and does nothing when `start` is not known.
fn sizes(start: Option<(u32, u32)>, steps: &[Op], cap: u64) -> Result<(), Error> {
    let Some((mut width, mut height)) = start else {
        return Ok(());
    };
    for op in steps {
        match ops::size(op, width, height, cap)? {
            Some(next) => (width, height) = next,
            None => break,
        }
    }
    Ok(())
}

/// The frame `input` starts from, held to `cap` pixels. A canvas or a pixel source with no pixel
/// returns `Invalid`, and so does a pixel source whose rows are not `width` by `height` by four
/// bytes.
fn frame(input: &Input<'_>, cap: u64) -> Result<Pixels, Error> {
    let size = |width: u64, height: u64| -> Result<(u32, u32, usize), Error> {
        if width == 0 || height == 0 {
            return Err(Error::Invalid(format!(
                "an image of {width}x{height} has no pixel"
            )));
        }
        decode::check(width, height, cap)?;
        let too_big = || Error::Invalid(format!("an image of {width}x{height} is too big"));
        let w = u32::try_from(width).map_err(|_| too_big())?;
        let h = u32::try_from(height).map_err(|_| too_big())?;
        let count = usize::try_from(width * height).map_err(|_| too_big())?;
        Ok((w, h, count))
    };
    match *input {
        Input::Encoded {
            data,
            auto_orient,
            to_srgb,
        } => decode(data, cap, auto_orient, to_srgb),
        Input::Canvas {
            width,
            height,
            fill,
        } => {
            let (width, height, count) = size(width, height)?;
            Ok(Pixels {
                width,
                height,
                rgba: fill.repeat(count),
                exif: None,
            })
        }
        Input::Pixels {
            width,
            height,
            rgba,
        } => {
            let (w, h, count) = size(width, height)?;
            if rgba.len() / 4 != count || rgba.len() % 4 != 0 {
                return Err(Error::Invalid(format!(
                    "{} bytes are not the RGBA8 rows of an image of {width}x{height}, which are {} bytes",
                    rgba.len(),
                    count.saturating_mul(4)
                )));
            }
            Ok(Pixels {
                width: w,
                height: h,
                rgba: rgba.to_vec(),
                exif: None,
            })
        }
        Input::Text(layout) => text::render(&layout, [0, 0, 0, 255], cap),
    }
}

/// `pixels` after `plan`'s steps, returned as its output: encoded in the plan's format, or else
/// `input`'s, or as the size header with or without the RGBA8 rows behind it.
fn finish(mut pixels: Pixels, input: Format, plan: &Plan<'_>, cap: u64) -> Result<Vec<u8>, Error> {
    steps(&mut pixels, &plan.steps, &plan.overlays, cap)?;
    let output = plan.output;
    if output == Output::Encoded {
        return encode(pixels, plan.encoding.format.unwrap_or(input), plan.encoding);
    }
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(&u64::from(pixels.width).to_be_bytes());
    out.extend_from_slice(&u64::from(pixels.height).to_be_bytes());
    if output == Output::Raw {
        out.extend_from_slice(&pixels.rgba);
    }
    Ok(out)
}

/// Runs `steps` on `pixels` under `cap`. A `composite` step makes its row of `overlays` from the
/// row's source, runs the row's steps with the rows before it, and draws the result.
fn steps(
    pixels: &mut Pixels,
    steps: &[Op],
    overlays: &[Overlay<'_>],
    cap: u64,
) -> Result<(), Error> {
    for op in steps {
        let Op::Composite(composite) = *op else {
            ops::apply(pixels, *op, cap)?;
            continue;
        };
        let row = usize::try_from(composite.overlay).expect("`check` held the row to the table");
        let overlay = &overlays[row];
        let mut layer = frame(&overlay.input, overlay.cap)?;
        self::steps(&mut layer, &overlay.steps, &overlays[..row], overlay.cap)?;
        ops::composite(pixels, &layer, composite);
    }
    Ok(())
}

/// The formats of the WIT `format` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Jpeg,
    Png,
    Webp,
    Gif,
    Avif,
    Jxl,
    Svg,
    Pdf,
}

/// The `nvs:ext` `error` variant: the host throws `LogicError` for `Invalid`, `ParseError` for
/// `Parse` and `RuntimeError` for `Runtime`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Invalid(String),
    Parse(String),
    Runtime(String),
}

#[cfg(target_family = "wasm")]
mod guest {
    wit_bindgen::generate!({
        path: ["../../wit/nvs-ext", "../../wit/image.wit"],
        world: "nvs:image/image",
        generate_all,
    });

    use exports::nvs::image::codec::{
        Align, Axis, Blend, Color, CompareOptions, Diff, Error, Filter, Fit, Format, Gravity,
        Guest, HashKind, ImageInfo, Output, PlaceholderKind, Plan, QrLevel, QrOptions, Source,
        Step,
    };

    use crate::ops::{self, Op};

    struct Component;

    fn error(err: crate::Error) -> Error {
        match err {
            crate::Error::Invalid(message) => Error::Invalid(message),
            crate::Error::Parse(message) => Error::Parse(message),
            crate::Error::Runtime(message) => Error::Runtime(message),
        }
    }

    fn format(format: crate::Format) -> Format {
        match format {
            crate::Format::Jpeg => Format::Jpeg,
            crate::Format::Png => Format::Png,
            crate::Format::Webp => Format::Webp,
            crate::Format::Gif => Format::Gif,
            crate::Format::Avif => Format::Avif,
            crate::Format::Jxl => Format::Jxl,
            crate::Format::Svg => Format::Svg,
            crate::Format::Pdf => Format::Pdf,
        }
    }

    fn from_format(format: Format) -> crate::Format {
        match format {
            Format::Jpeg => crate::Format::Jpeg,
            Format::Png => crate::Format::Png,
            Format::Webp => crate::Format::Webp,
            Format::Gif => crate::Format::Gif,
            Format::Avif => crate::Format::Avif,
            Format::Jxl => crate::Format::Jxl,
            Format::Svg => crate::Format::Svg,
            Format::Pdf => crate::Format::Pdf,
        }
    }

    /// The names of the operations `step` sets. A well-formed step sets one.
    fn operations(step: &Step) -> Vec<&'static str> {
        [
            ("resize", step.resize.is_some()),
            ("crop", step.crop.is_some()),
            ("trim", step.trim.is_some()),
            ("rotate", step.rotate.is_some()),
            ("flip", step.flip.is_some()),
            ("composite", step.composite.is_some()),
            ("flatten", step.flatten.is_some()),
            ("sharpen", step.sharpen.is_some()),
            ("blur", step.blur.is_some()),
            ("grayscale", step.grayscale.is_some()),
            ("brightness", step.brightness.is_some()),
            ("contrast", step.contrast.is_some()),
            ("gamma", step.gamma.is_some()),
            ("tint", step.tint.is_some()),
            ("text", step.text.is_some()),
            ("format", step.format.is_some()),
            ("metadata", step.metadata.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, set)| set.then_some(name))
        .collect()
    }

    fn color(color: &Color) -> Result<[u8; 4], Error> {
        ops::color(color.r, color.g, color.b, color.alpha).map_err(error)
    }

    /// The core's input for `source`, and the pixel cap it is held to: the source's own
    /// `max-pixels` for encoded bytes, and the default for a canvas or pixels.
    fn input(source: &Source) -> Result<(crate::Input<'_>, u64), Error> {
        Ok(match source {
            Source::Encoded(source) => (
                crate::Input::Encoded {
                    data: &source.data,
                    auto_orient: source.auto_orient,
                    to_srgb: source.to_srgb,
                },
                source.max_pixels.unwrap_or(crate::DEFAULT_MAX_PIXELS),
            ),
            Source::Canvas(canvas) => (
                crate::Input::Canvas {
                    width: canvas.width,
                    height: canvas.height,
                    fill: color(&canvas.fill)?,
                },
                crate::DEFAULT_MAX_PIXELS,
            ),
            Source::Pixels(pixels) => (
                crate::Input::Pixels {
                    width: pixels.width,
                    height: pixels.height,
                    rgba: &pixels.pixels,
                },
                crate::DEFAULT_MAX_PIXELS,
            ),
            Source::Text(text) => (
                crate::Input::Text(crate::text::Layout {
                    text: &text.text,
                    font: &text.font,
                    size: text.size,
                    max_width: text.max_width,
                    align: text.align.map(align).unwrap_or_default(),
                }),
                crate::DEFAULT_MAX_PIXELS,
            ),
        })
    }

    fn align(align: Align) -> crate::text::Align {
        match align {
            Align::Left => crate::text::Align::Left,
            Align::Center => crate::text::Align::Center,
            Align::Right => crate::text::Align::Right,
        }
    }

    fn gravity(gravity: Gravity) -> ops::Gravity {
        match gravity {
            Gravity::Center => ops::Gravity::Center,
            Gravity::North => ops::Gravity::North,
            Gravity::NorthEast => ops::Gravity::NorthEast,
            Gravity::East => ops::Gravity::East,
            Gravity::SouthEast => ops::Gravity::SouthEast,
            Gravity::South => ops::Gravity::South,
            Gravity::SouthWest => ops::Gravity::SouthWest,
            Gravity::West => ops::Gravity::West,
            Gravity::NorthWest => ops::Gravity::NorthWest,
        }
    }

    /// The pixel step `step` sets, or `None` for a `format`, a `metadata` or a
    /// `grayscale(false)` step.
    fn op(step: &Step) -> Result<Option<Op<'_>>, Error> {
        let op = if let Some(resize) = &step.resize {
            Op::Resize(ops::Resize {
                width: resize.width,
                height: resize.height,
                fit: match resize.fit.unwrap_or(Fit::Cover) {
                    Fit::Cover => ops::Fit::Cover,
                    Fit::Contain => ops::Fit::Contain,
                    Fit::Fill => ops::Fit::Fill,
                    Fit::Inside => ops::Fit::Inside,
                    Fit::Outside => ops::Fit::Outside,
                },
                gravity: resize.gravity.map(gravity).unwrap_or_default(),
                filter: match resize.filter.unwrap_or(Filter::Lanczos3) {
                    Filter::Nearest => ops::Filter::Nearest,
                    Filter::Bilinear => ops::Filter::Bilinear,
                    Filter::CatmullRom => ops::Filter::CatmullRom,
                    Filter::Mitchell => ops::Filter::Mitchell,
                    Filter::Lanczos3 => ops::Filter::Lanczos3,
                },
                upscale: resize.upscale.unwrap_or(false),
            })
        } else if let Some(crop) = &step.crop {
            Op::Crop {
                x: crop.x,
                y: crop.y,
                width: crop.width,
                height: crop.height,
            }
        } else if let Some(trim) = &step.trim {
            Op::Trim {
                threshold: trim.threshold.unwrap_or(ops::DEFAULT_TRIM_THRESHOLD),
            }
        } else if let Some(rotate) = &step.rotate {
            Op::Rotate {
                degrees: rotate.degrees,
                background: rotate.background.as_ref().map_or(Ok([0; 4]), color)?,
            }
        } else if let Some(axis) = step.flip {
            Op::Flip(match axis {
                Axis::Horizontal => ops::Axis::Horizontal,
                Axis::Vertical => ops::Axis::Vertical,
            })
        } else if let Some(composite) = &step.composite {
            Op::Composite(ops::Composite {
                overlay: composite.overlay,
                gravity: composite.gravity.map(gravity).unwrap_or_default(),
                x: composite.x,
                y: composite.y,
                opacity: composite.opacity.unwrap_or(1.0),
                blend: match composite.blend.unwrap_or(Blend::Normal) {
                    Blend::Normal => ops::Blend::Normal,
                    Blend::Multiply => ops::Blend::Multiply,
                    Blend::Screen => ops::Blend::Screen,
                    Blend::Overlay => ops::Blend::Overlay,
                    Blend::Darken => ops::Blend::Darken,
                    Blend::Lighten => ops::Blend::Lighten,
                },
            })
        } else if let Some(background) = &step.flatten {
            Op::Flatten(color(background)?)
        } else if let Some(sharpen) = &step.sharpen {
            Op::Sharpen {
                sigma: sharpen.sigma.unwrap_or(ops::DEFAULT_SHARPEN_SIGMA),
            }
        } else if let Some(sigma) = step.blur {
            Op::Blur { sigma }
        } else if let Some(on) = step.grayscale {
            if !on {
                return Ok(None);
            }
            Op::Grayscale
        } else if let Some(factor) = step.brightness {
            Op::Brightness(factor)
        } else if let Some(factor) = step.contrast {
            Op::Contrast(factor)
        } else if let Some(gamma) = step.gamma {
            Op::Gamma(gamma)
        } else if let Some(tint) = &step.tint {
            Op::Tint(color(tint)?)
        } else if let Some(text) = &step.text {
            Op::Text(crate::text::Text {
                layout: crate::text::Layout {
                    text: &text.text,
                    font: &text.font,
                    size: text.size,
                    max_width: text.max_width,
                    align: text.align.map(align).unwrap_or_default(),
                },
                color: color(&text.color)?,
                gravity: text.gravity.map(gravity).unwrap_or_default(),
                x: text.x,
                y: text.y,
            })
        } else {
            return Ok(None);
        };
        Ok(Some(op))
    }

    /// `plan` as the codec core runs it.
    fn plan(plan: &Plan) -> Result<crate::Plan<'_>, Error> {
        let (steps, encoding) = self::steps(&plan.steps)?;
        let overlays = plan
            .overlays
            .iter()
            .map(|overlay| {
                let (input, cap) = input(&overlay.source)?;
                let (steps, _) = self::steps(&overlay.steps)?;
                Ok(crate::Overlay { input, cap, steps })
            })
            .collect::<Result<Vec<_>, Error>>()?;
        let output = match plan.output {
            Output::Encoded => crate::Output::Encoded,
            Output::Raw => crate::Output::Raw,
            Output::Size => crate::Output::Size,
        };
        Ok(crate::Plan {
            steps,
            overlays,
            encoding,
            output,
        })
    }

    /// The pixel steps of `list` in order, and the encoding its `format` and `metadata` steps
    /// set. An overlay's list is read the same way, and its encoding is not used.
    fn steps(list: &[Step]) -> Result<(Vec<Op<'_>>, crate::Encoding), Error> {
        let mut encoding = crate::Encoding::default();
        let mut steps = Vec::with_capacity(list.len());
        for step in list {
            match operations(step).as_slice() {
                ["format"] => {
                    if let Some(options) = &step.format {
                        encoding.format = Some(from_format(options.format));
                        encoding.quality = options.quality;
                        encoding.lossless = options.lossless;
                    }
                }
                ["metadata"] => {
                    if let Some(options) = &step.metadata {
                        encoding.keep_metadata = options.keep;
                    }
                }
                [_] => steps.extend(op(step)?),
                set => {
                    return Err(Error::Invalid(format!(
                        "a step sets exactly one operation, and this one sets {}",
                        set.len()
                    )));
                }
            }
        }
        Ok((steps, encoding))
    }

    impl Guest for Component {
        fn info(data: Vec<u8>) -> Result<ImageInfo, Error> {
            let info = crate::info(&data).map_err(error)?;
            Ok(ImageInfo {
                format: format(info.format),
                width: info.width,
                height: info.height,
                has_alpha: info.has_alpha,
                frames: info.frames,
                orientation: info.orientation,
                exif: info.exif,
                has_icc: info.has_icc,
            })
        }

        fn run(source: Source, plan: Plan) -> Result<Vec<u8>, Error> {
            let plan = self::plan(&plan)?;
            let (input, cap) = input(&source)?;
            crate::run(&input, cap, &plan).map_err(error)
        }

        fn variants(source: Source, plans: Vec<Plan>) -> Result<Vec<Vec<u8>>, Error> {
            let plans = plans.iter().map(plan).collect::<Result<Vec<_>, _>>()?;
            let (input, cap) = input(&source)?;
            crate::variants(&input, cap, &plans).map_err(error)
        }

        fn compare(a: Vec<u8>, b: Vec<u8>, options: CompareOptions) -> Result<Diff, Error> {
            let diff =
                crate::compare::compare(&a, &b, options.tolerance.unwrap_or(0), options.render)
                    .map_err(error)?;
            Ok(Diff {
                identical: diff.identical,
                differing_pixels: diff.differing_pixels,
                max_delta: diff.max_delta,
                ssim: diff.ssim,
                diff: diff.diff,
            })
        }

        fn hash(data: Vec<u8>, kind: HashKind) -> Result<Vec<u8>, Error> {
            let kind = match kind {
                HashKind::Perceptual => crate::hash::Kind::Perceptual,
                HashKind::Difference => crate::hash::Kind::Difference,
                HashKind::Average => crate::hash::Kind::Average,
            };
            crate::hash::hash(&data, kind).map_err(error)
        }

        fn placeholder(data: Vec<u8>, kind: PlaceholderKind) -> Result<String, Error> {
            let kind = match kind {
                PlaceholderKind::BlurHash => crate::summary::Kind::BlurHash,
                PlaceholderKind::ThumbHash => crate::summary::Kind::ThumbHash,
            };
            crate::summary::placeholder(&data, kind).map_err(error)
        }

        fn palette(data: Vec<u8>, count: u64) -> Result<Vec<Color>, Error> {
            let colours = crate::summary::palette(&data, count).map_err(error)?;
            Ok(colours
                .into_iter()
                .map(|[r, g, b, a]| Color {
                    r: r.into(),
                    g: g.into(),
                    b: b.into(),
                    alpha: f64::from(a) / 255.0,
                })
                .collect())
        }

        fn qr(data: String, options: QrOptions) -> Result<Vec<u8>, Error> {
            let defaults = crate::qr::Options::default();
            let options = crate::qr::Options {
                size: options.size.unwrap_or(defaults.size),
                margin: options.margin.unwrap_or(defaults.margin),
                level: options.level.map_or(defaults.level, |level| match level {
                    QrLevel::Low => crate::qr::Level::Low,
                    QrLevel::Medium => crate::qr::Level::Medium,
                    QrLevel::Quartile => crate::qr::Level::Quartile,
                    QrLevel::High => crate::qr::Level::High,
                }),
                format: options.format.map_or(defaults.format, from_format),
            };
            crate::qr::render(&data, options).map_err(error)
        }
    }

    export!(Component);
}
