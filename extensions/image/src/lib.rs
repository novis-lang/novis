//! The image component: `nvs:image/codec` from `wit/image.wit`, built into every `nvs` binary
//! (`rule:packaging/the-first-party-components-are-built-in`, `rule:core-classes/image-pipeline`).
//!
//! The crate has two halves. The codec core is plain Rust that builds for the host too, so its
//! tests run under an ordinary `cargo test` in this directory. The WIT glue in `guest` is compiled
//! only for a wasm target, and maps each export onto the core and the core's types onto WIT's.
//!
//! `info` is the export that is implemented. Every other export returns `runtime` with a message
//! naming it, until the slice of goal `ext-image` that writes it lands.

mod info;

pub use info::{Info, info};

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
        Color, CompareOptions, Diff, Error, Format, Guest, HashKind, ImageInfo, PlaceholderKind,
        Plan, QrOptions, Source,
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

    fn missing<T>(export: &str) -> Result<T, Error> {
        Err(Error::Runtime(format!(
            "`{export}` is not available in this build of the image component"
        )))
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

        fn run(_source: Source, _plan: Plan) -> Result<Vec<u8>, Error> {
            missing("run")
        }

        fn variants(_source: Source, _plans: Vec<Plan>) -> Result<Vec<Vec<u8>>, Error> {
            missing("variants")
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
