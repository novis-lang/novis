//! Encoding RGBA8 pixels to a file (`rule:core-classes/image-format-roster`,
//! `rule:core-classes/image-correct-by-default`).
//!
//! JPEG, PNG, lossless WebP and GIF encode through `image`. Lossy WebP and AVIF return `Runtime`
//! naming what is missing; JPEG XL, SVG and PDF are refused by `run` before anything decodes.
//! A JPEG has no alpha channel, so the encoder drops it, and a GIF is quantised to a palette.
//!
//! **Metadata is stripped unless the plan keeps it.** Every encoder writes the pixels and no
//! profile. With `keep_metadata`, the input's EXIF block is written back where the format has a
//! place for it (JPEG, PNG and WebP), with the orientation tag `decode` already reset.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::{DynamicImage, ImageEncoder, ImageFormat, RgbaImage};

use crate::{Encoding, Error, Format, Pixels};

/// The JPEG quality with no `quality` option, on the 1 to 100 scale.
pub const DEFAULT_JPEG_QUALITY: u8 = 85;

/// `pixels` written as a `format` file, under the plan's `encoding`.
pub fn encode(pixels: Pixels, format: Format, encoding: Encoding) -> Result<Vec<u8>, Error> {
    let fail = |err: image::ImageError| Error::Runtime(format!("the {format:?} encoder failed: {err}"));
    let Pixels {
        width,
        height,
        rgba,
        exif,
    } = pixels;
    let image = RgbaImage::from_raw(width, height, rgba).ok_or_else(|| {
        Error::Runtime("the pixel buffer is shorter than its width and height".to_string())
    })?;
    let image = DynamicImage::ImageRgba8(image);
    let exif = exif.filter(|_| encoding.keep_metadata);
    let mut out = Vec::new();
    match format {
        Format::Jpeg => {
            let quality = encoding
                .quality
                .map_or(DEFAULT_JPEG_QUALITY, |quality| quality.clamp(1, 100) as u8);
            let mut encoder = JpegEncoder::new_with_quality(&mut out, quality);
            keep(&mut encoder, exif)?;
            image.write_with_encoder(encoder).map_err(fail)?;
        }
        Format::Png => {
            let mut encoder = PngEncoder::new(&mut out);
            keep(&mut encoder, exif)?;
            image.write_with_encoder(encoder).map_err(fail)?;
        }
        Format::Webp if encoding.lossless == Some(true) => {
            let mut encoder = WebPEncoder::new_lossless(&mut out);
            keep(&mut encoder, exif)?;
            image.write_with_encoder(encoder).map_err(fail)?;
        }
        Format::Webp => {
            return Err(Error::Runtime(
                "lossy WebP is not available in this build of the image component".to_string(),
            ));
        }
        Format::Gif => image
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Gif)
            .map_err(fail)?,
        Format::Avif => {
            return Err(Error::Runtime(
                "AVIF encoding is not available in this build of the image component".to_string(),
            ));
        }
        Format::Jxl | Format::Svg | Format::Pdf => {
            return Err(Error::Invalid(format!(
                "{format:?} is a format the image component decodes and does not encode"
            )));
        }
    }
    Ok(out)
}

/// Hands `encoder` the EXIF block to write, when there is one.
fn keep(encoder: &mut impl ImageEncoder, exif: Option<Vec<u8>>) -> Result<(), Error> {
    match exif {
        Some(raw) => encoder
            .set_exif_metadata(raw)
            .map_err(|err| Error::Runtime(format!("the encoder cannot keep EXIF: {err}"))),
        None => Ok(()),
    }
}
