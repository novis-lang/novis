//! The image component: `nvs:image/codec` from `wit/image.wit`, built into every `nvs` binary
//! (`rule:packaging/the-first-party-components-are-built-in`, `rule:core-classes/image-pipeline`).
//!
//! The crate has two halves. The codec core is plain Rust that builds for the host too, so its
//! tests run under an ordinary `cargo test` in this directory. The WIT glue in `guest` is compiled
//! only for a wasm target, and maps each export onto the core and the core's types onto WIT's.
//!
//! `info`, `run` and `variants` are the exports that are implemented. `run` starts from an
//! [`Input`] — encoded bytes it decodes (`decode`'s module doc), a blank canvas or RGBA8 rows —
//! runs the plan's pixel steps on the frame in order (`ops`'s module doc), and returns the pixels,
//! their size or the encoded file (`encode`'s module doc); `variants` makes the frame once and
//! runs each of its plans on a copy of it. A canvas or a pixel source has no input format, so an
//! encoded output with no `format` step is PNG, and its size is held to the same pixel cap as a
//! decode. A `composite` or `text` step, a `text` source and an overlay each return `runtime`
//! naming what is missing. Every other export returns `runtime` naming it, until the slice of goal
//! `ext-image` that writes it lands.

mod avif;
mod decode;
mod encode;
mod info;
pub mod ops;
mod webp;

pub use decode::{DEFAULT_MAX_PIXELS, Pixels, decode, sniff};
pub use encode::{DEFAULT_JPEG_QUALITY, encode};
pub use info::{Info, info};
pub use ops::Op;

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

/// A plan without its overlays: the pixel steps in order, how the result is encoded, and what
/// `run` returns.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub steps: Vec<Op>,
    pub encoding: Encoding,
    pub output: Output,
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
    Canvas { width: u64, height: u64, fill: [u8; 4] },
    /// RGBA8 rows, four bytes per pixel.
    Pixels { width: u64, height: u64, rgba: &'a [u8] },
}

/// Runs `plan` on `input` under `cap` pixels and returns its output. A format the component does
/// not encode and a step option out of range return `Invalid` before anything is decoded.
pub fn run(input: &Input<'_>, cap: u64, plan: &Plan) -> Result<Vec<u8>, Error> {
    let mut outs = variants(input, cap, std::slice::from_ref(plan))?;
    Ok(outs.pop().unwrap_or_default())
}

/// Runs each of `plans` as `run` does, on one frame made from `input`, and returns their outputs
/// in order. Every plan is checked before anything is decoded. Each plan but the last works on
/// its own copy of the frame, so a call holds two frames at once, and a step that builds its
/// result beside the frame holds a third while it runs.
pub fn variants(input: &Input<'_>, cap: u64, plans: &[Plan]) -> Result<Vec<Vec<u8>>, Error> {
    let format = match *input {
        Input::Encoded { data, .. } => sniff(data)?,
        Input::Canvas { .. } | Input::Pixels { .. } => Format::Png,
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
        plan.steps.iter().try_for_each(ops::check)?;
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
    }
}

/// `pixels` after `plan`'s steps, returned as its output: encoded in the plan's format, or else
/// `input`'s, or as the size header with or without the RGBA8 rows behind it.
fn finish(mut pixels: Pixels, input: Format, plan: &Plan, cap: u64) -> Result<Vec<u8>, Error> {
    for op in &plan.steps {
        ops::apply(&mut pixels, *op, cap)?;
    }
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
        Axis, Color, CompareOptions, Diff, Error, Filter, Fit, Format, Gravity, Guest, HashKind,
        ImageInfo, Output, PlaceholderKind, Plan, QrOptions, Source, Step,
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

    fn missing<T>(export: &str) -> Result<T, Error> {
        Err(Error::Runtime(format!(
            "`{export}` is not available in this build of the image component"
        )))
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
            Source::Text(_) => return missing("a `text` source"),
        })
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
    fn op(step: &Step) -> Result<Option<Op>, Error> {
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
        } else {
            return Ok(None);
        };
        Ok(Some(op))
    }

    /// `plan` as the codec core runs it. A `composite` or `text` step and any overlay return
    /// `runtime` naming what is missing.
    fn plan(plan: &Plan) -> Result<crate::Plan, Error> {
        if !plan.overlays.is_empty() {
            return missing("an overlay");
        }
        let mut encoding = crate::Encoding::default();
        let mut steps = Vec::with_capacity(plan.steps.len());
        for step in &plan.steps {
            match operations(step).as_slice() {
                ["format"] => {
                    if let Some(options) = &step.format {
                        encoding.format = Some(match options.format {
                            Format::Jpeg => crate::Format::Jpeg,
                            Format::Png => crate::Format::Png,
                            Format::Webp => crate::Format::Webp,
                            Format::Gif => crate::Format::Gif,
                            Format::Avif => crate::Format::Avif,
                            Format::Jxl => crate::Format::Jxl,
                            Format::Svg => crate::Format::Svg,
                            Format::Pdf => crate::Format::Pdf,
                        });
                        encoding.quality = options.quality;
                        encoding.lossless = options.lossless;
                    }
                }
                ["metadata"] => {
                    if let Some(options) = &step.metadata {
                        encoding.keep_metadata = options.keep;
                    }
                }
                [operation @ ("composite" | "text")] => {
                    return missing(&format!("the `{operation}` step"));
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
        let output = match plan.output {
            Output::Encoded => crate::Output::Encoded,
            Output::Raw => crate::Output::Raw,
            Output::Size => crate::Output::Size,
        };
        Ok(crate::Plan {
            steps,
            encoding,
            output,
        })
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

        fn compare(_a: Vec<u8>, _b: Vec<u8>, _options: CompareOptions) -> Result<Diff, Error> {
            missing("compare")
        }

        fn hash(_data: Vec<u8>, _kind: HashKind) -> Result<Vec<u8>, Error> {
            missing("hash")
        }

        fn placeholder(_data: Vec<u8>, _kind: PlaceholderKind) -> Result<String, Error> {
            missing("placeholder")
        }

        fn palette(_data: Vec<u8>, _count: u64) -> Result<Vec<Color>, Error> {
            missing("palette")
        }

        fn qr(_data: String, _options: QrOptions) -> Result<Vec<u8>, Error> {
            missing("qr")
        }
    }

    export!(Component);
}
