//! The image component's `nvs:image/codec`, called typed: `wit/image.wit`'s types as `bindgen!`
//! writes them, and the conversion of each to and from its Novis value.

use self::bindings::exports::nvs::image::codec::{
    Align, Axis, BarcodeOptions, Blend, Canvas, Color, CompareOptions, CompositeOptions, CropBox,
    Diff, EncodedSource, Filter, Fit, Format, FormatOptions, Gravity, HashKind, ImageInfo,
    MetadataOptions, Output, Overlay, PlaceholderKind, Plan, QrLevel, QrOptions, RawImage,
    ResizeOptions, RotateOptions, SharpenOptions, Source, Step, Symbology, TextOptions, TextSource,
    TrimOptions,
};
use self::bindings::nvs::ext::types::Error as WitError;
use crate::call::Error;

/// The types `bindgen!` writes for the world that exports `nvs:image/codec` alone. Its calls are
/// made through [`crate::typed::call`], so only the types are used.
#[allow(
    missing_docs,
    unreachable_pub,
    dead_code,
    clippy::all,
    reason = "generated code, of which only the types are named"
)]
mod bindings {
    wasmtime::component::bindgen!({
        path: ["../../wit/nvs-ext", "../../wit/image.wit"],
        inline: "
            package nvs:typed;

            world image {
                export nvs:image/codec@1.0.0;
            }
        ",
        world: "nvs:typed/image",
    });
}

impl From<WitError> for Error {
    fn from(err: WitError) -> Self {
        match err {
            WitError::Invalid(message) => Self::Invalid(message),
            WitError::Parse(message) => Self::Parse(message),
            WitError::Runtime(message) => Self::Runtime(message),
        }
    }
}

world! {
    error: WitError;
    info(Vec<u8>) -> ImageInfo;
    run(Source, Plan) -> Vec<u8>;
    variants(Source, Vec<Plan>) -> Vec<Vec<u8>>;
    compare(Vec<u8>, Vec<u8>, CompareOptions) -> Diff;
    hash(Vec<u8>, HashKind) -> Vec<u8>;
    placeholder(Vec<u8>, PlaceholderKind) -> String;
    palette(Vec<u8>, u64) -> Vec<Color>;
    qr(String, QrOptions) -> Vec<u8>;
    barcode(String, Symbology, BarcodeOptions) -> Vec<u8>;
}

records! {
    Color { r, g, b, alpha }
    ImageInfo { format, width, height, has_alpha, frames, orientation, exif, has_icc }
    EncodedSource { data, auto_orient, to_srgb, max_pixels, page, dpi }
    Canvas { width, height, fill }
    RawImage { width, height, pixels }
    TextSource { text, font, size, max_width, align }
    ResizeOptions { width, height, fit, gravity, filter, upscale }
    CropBox { x, y, width, height }
    TrimOptions { threshold }
    RotateOptions { degrees, background }
    CompositeOptions { overlay, gravity, x, y, opacity, blend }
    SharpenOptions { sigma }
    TextOptions { text, font, size, color, gravity, x, y, max_width, align }
    FormatOptions { format, quality, progressive, lossless, effort }
    MetadataOptions { keep }
    Step {
        resize, crop, trim, rotate, flip, composite, flatten, sharpen, blur, grayscale,
        brightness, contrast, gamma, tint, text, format, metadata,
    }
    Overlay { source, steps }
    Plan { steps, overlays, output }
    CompareOptions { tolerance, render }
    Diff { identical, differing_pixels, max_delta, ssim, diff }
    QrOptions { size, margin, level, format }
    BarcodeOptions { height, module, margin, text, format }
}

enums! {
    Format [Jpeg, Png, Webp, Gif, Avif, Jxl, Svg, Pdf]
    Gravity [Center, North, NorthEast, East, SouthEast, South, SouthWest, West, NorthWest]
    Fit [Cover, Contain, Fill, Inside, Outside]
    Filter [Nearest, Bilinear, CatmullRom, Mitchell, Lanczos3]
    Axis [Horizontal, Vertical]
    Blend [Normal, Multiply, Screen, Overlay, Darken, Lighten]
    Align [Left, Center, Right]
    HashKind [Perceptual, Difference, Average]
    PlaceholderKind [BlurHash, ThumbHash]
    QrLevel [Low, Medium, Quartile, High]
    Symbology [Code128, Gs1128, Ean13, Ean8, UpcA, Code39, Itf14, DataMatrix]
    Output [Encoded, Raw, Size]
}

variants! {
    Source [Encoded, Canvas, Pixels, Text]
}
