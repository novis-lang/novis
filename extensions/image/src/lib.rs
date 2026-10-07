//! The image component: `nvs:image/codec` from `wit/image.wit`, built into every `nvs` binary
//! (`rule:packaging/the-first-party-components-are-built-in`, `rule:core-classes/image-pipeline`).
//!
//! The crate has two halves. The codec core is plain Rust that builds for the host too, so its
//! tests run under an ordinary `cargo test` in this directory. The WIT glue in `guest` is compiled
//! only for a wasm target, and maps each export onto the core and the core's types onto WIT's.
//!
//! `info`, `run` and `variants` are the exports that are implemented. `run` decodes encoded bytes
//! (`decode`'s module doc) and returns the pixels, their size or the encoded file (`encode`'s
//! module doc); `variants` decodes once and runs each of its plans on a copy of that frame. A
//! step other than `format` and `metadata`, a source other than encoded bytes and an overlay each
//! return `runtime` naming what is missing. Every other export returns `runtime` naming it, until
//! the slice of goal `ext-image` that writes it lands.

mod avif;
mod decode;
mod encode;
mod info;
mod webp;

pub use decode::{DEFAULT_MAX_PIXELS, Pixels, decode, sniff};
pub use encode::{DEFAULT_JPEG_QUALITY, encode};
pub use info::{Info, info};

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

/// Runs a plan with no pixel step on the encoded `data`: decodes it under `cap` pixels, applying
/// the EXIF orientation when `auto_orient` is set and converting an embedded ICC profile to sRGB
/// when `to_srgb` is, and returns `output`. A format the component does not encode returns
/// `Invalid` before anything is decoded.
pub fn run(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
    encoding: Encoding,
    output: Output,
) -> Result<Vec<u8>, Error> {
    let mut outs = variants(data, cap, auto_orient, to_srgb, &[(encoding, output)])?;
    Ok(outs.pop().unwrap_or_default())
}

/// Runs each of `plans` as `run` does, on one decode of `data`, and returns their outputs in
/// order. Every plan's format is checked before anything is decoded. Each plan but the last works
/// on its own copy of the frame, so a call holds at most two frames at once.
pub fn variants(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
    plans: &[(Encoding, Output)],
) -> Result<Vec<Vec<u8>>, Error> {
    let input = sniff(data)?;
    for (encoding, output) in plans {
        let target = encoding.format.unwrap_or(input);
        if *output == Output::Encoded && matches!(target, Format::Jxl | Format::Svg | Format::Pdf) {
            return Err(Error::Invalid(format!(
                "{target:?} is a format the image component decodes and does not encode"
            )));
        }
    }
    let Some(((encoding, output), rest)) = plans.split_last() else {
        return Ok(Vec::new());
    };
    let pixels = decode(data, cap, auto_orient, to_srgb)?;
    let mut outs = Vec::with_capacity(plans.len());
    for (encoding, output) in rest {
        outs.push(finish(pixels.clone(), input, *encoding, *output)?);
    }
    outs.push(finish(pixels, input, *encoding, *output)?);
    Ok(outs)
}

/// `pixels` returned as `output`: encoded in the plan's format, or else `input`'s, or as the
/// size header with or without the RGBA8 rows behind it.
fn finish(pixels: Pixels, input: Format, encoding: Encoding, output: Output) -> Result<Vec<u8>, Error> {
    if output == Output::Encoded {
        return encode(pixels, encoding.format.unwrap_or(input), encoding);
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
        Color, CompareOptions, Diff, Error, Format, Guest, HashKind, ImageInfo, Output,
        PlaceholderKind, Plan, QrOptions, Source, Step,
    };

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

    /// How `plan` writes its result, read off its `format` and `metadata` steps. Any other step
    /// and any overlay return `runtime` naming what is missing.
    fn encoding(plan: &Plan) -> Result<(crate::Encoding, crate::Output), Error> {
        if !plan.overlays.is_empty() {
            return missing("an overlay");
        }
        let mut encoding = crate::Encoding::default();
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
                [operation] => return missing(&format!("the `{operation}` step")),
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
        Ok((encoding, output))
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
            let (encoding, output) = encoding(&plan)?;
            let cap = source.max_pixels.unwrap_or(crate::DEFAULT_MAX_PIXELS);
            crate::run(
                &source.data,
                cap,
                source.auto_orient,
                source.to_srgb,
                encoding,
                output,
            )
            .map_err(error)
        }

        fn variants(source: Source, plans: Vec<Plan>) -> Result<Vec<Vec<u8>>, Error> {
            let Source::Encoded(source) = source else {
                return missing("a source other than encoded bytes");
            };
            let plans = plans.iter().map(encoding).collect::<Result<Vec<_>, _>>()?;
            let cap = source.max_pixels.unwrap_or(crate::DEFAULT_MAX_PIXELS);
            crate::variants(&source.data, cap, source.auto_orient, source.to_srgb, &plans)
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
