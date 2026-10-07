//! The image component: `nvs:image/codec` from `wit/image.wit`, built into every `nvs` binary
//! (`rule:packaging/the-first-party-components-are-built-in`, `rule:core-classes/image-pipeline`).
//!
//! The crate has two halves. The codec core is plain Rust that builds for the host too, so its
//! tests run under an ordinary `cargo test` in this directory. The WIT glue in `guest` is compiled
//! only for a wasm target, and maps each export onto the core and the core's types onto WIT's.
//!
//! `info`, `run` and `variants` are the exports that are implemented. `run` decodes encoded bytes
//! (`decode`'s module doc), runs the plan's pixel steps on the frame in order (`ops`'s module
//! doc), and returns the pixels, their size or the encoded file (`encode`'s module doc);
//! `variants` decodes once and runs each of its plans on a copy of that frame. A `composite` or
//! `text` step, a source other than encoded bytes and an overlay each return `runtime` naming
//! what is missing. Every other export returns `runtime` naming it, until the slice of goal
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

/// Runs `plan` on the encoded `data`: decodes it under `cap` pixels, applying the EXIF
/// orientation when `auto_orient` is set and converting an embedded ICC profile to sRGB when
/// `to_srgb` is, runs the plan's steps, and returns its output. A format the component does not
/// encode and a step option out of range return `Invalid` before anything is decoded.
pub fn run(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
    plan: &Plan,
) -> Result<Vec<u8>, Error> {
    let mut outs = variants(data, cap, auto_orient, to_srgb, std::slice::from_ref(plan))?;
    Ok(outs.pop().unwrap_or_default())
}

/// Runs each of `plans` as `run` does, on one decode of `data`, and returns their outputs in
/// order. Every plan is checked before anything is decoded. Each plan but the last works on its
/// own copy of the frame, so a call holds two frames at once, and a step that builds its result
/// beside the frame holds a third while it runs.
pub fn variants(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
    plans: &[Plan],
) -> Result<Vec<Vec<u8>>, Error> {
    let input = sniff(data)?;
    for plan in plans {
        let target = plan.encoding.format.unwrap_or(input);
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
    let pixels = decode(data, cap, auto_orient, to_srgb)?;
    let mut outs = Vec::with_capacity(plans.len());
    for plan in rest {
        outs.push(finish(pixels.clone(), input, plan, cap)?);
    }
    outs.push(finish(pixels, input, last, cap)?);
    Ok(outs)
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
            let Source::Encoded(source) = source else {
                return missing("a source other than encoded bytes");
            };
            let plan = self::plan(&plan)?;
            let cap = source.max_pixels.unwrap_or(crate::DEFAULT_MAX_PIXELS);
            crate::run(&source.data, cap, source.auto_orient, source.to_srgb, &plan).map_err(error)
        }

        fn variants(source: Source, plans: Vec<Plan>) -> Result<Vec<Vec<u8>>, Error> {
            let Source::Encoded(source) = source else {
                return missing("a source other than encoded bytes");
            };
            let plans = plans.iter().map(plan).collect::<Result<Vec<_>, _>>()?;
            let cap = source.max_pixels.unwrap_or(crate::DEFAULT_MAX_PIXELS);
            crate::variants(
                &source.data,
                cap,
                source.auto_orient,
                source.to_srgb,
                &plans,
            )
            .map_err(error)
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
