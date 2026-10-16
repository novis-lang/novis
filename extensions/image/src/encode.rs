//! Encoding RGBA8 pixels to a file (`rule:core-classes/image-format-roster`,
//! `rule:core-classes/image-correct-by-default`).
//!
//! JPEG, PNG, lossless WebP and GIF encode through `image`, lossy WebP through the linked libwebp
//! (`webp`'s module doc) and AVIF through `ravif`. JPEG XL, SVG and PDF are refused by `run`
//! before anything decodes. A JPEG has no alpha channel, so the encoder drops it, and a GIF is
//! quantised to a palette. WebP is lossy unless the `format` step sets `lossless`.
//!
//! **Metadata is stripped unless the plan keeps it.** Every encoder writes the pixels and no
//! profile. With `keep_metadata`, the input's EXIF block is written back where the format has a
//! place for it (JPEG, PNG and WebP), with the orientation tag `decode` already reset. An AVIF
//! carries none, because `ravif` writes no metadata.

use std::io::Cursor;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::codecs::webp::WebPEncoder;
use image::{DynamicImage, ImageEncoder, ImageFormat, RgbaImage};

use crate::{Encoding, Error, Format, Pixels, webp};

/// The JPEG quality with no `quality` option, on the 1 to 100 scale.
pub const DEFAULT_JPEG_QUALITY: u8 = 85;

/// The lossy WebP and AVIF quality with no `quality` option, on the 1 to 100 scale.
pub const DEFAULT_LOSSY_QUALITY: u8 = 80;

/// `ravif`'s speed, from 1 (slowest, smallest file) to 10. A request waits on the encode, so
/// this trades some file size for a shorter wait.
const AVIF_SPEED: u8 = 7;

/// `pixels` written as a `format` file, under the plan's `encoding`.
pub fn encode(pixels: Pixels, format: Format, encoding: Encoding) -> Result<Vec<u8>, Error> {
    let fail =
        |err: image::ImageError| Error::Runtime(format!("the {format:?} encoder failed: {err}"));
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
            let quality = quality(encoding, DEFAULT_JPEG_QUALITY);
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
            let quality = quality(encoding, DEFAULT_LOSSY_QUALITY);
            out = webp::lossy(image.as_bytes(), width, height, quality)?;
            if let Some(exif) = exif {
                out = webp::with_exif(&out, &exif, width, height)?;
            }
        }
        Format::Gif => image
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Gif)
            .map_err(fail)?,
        Format::Avif => {
            let quality = quality(encoding, DEFAULT_LOSSY_QUALITY);
            let pixels: Vec<ravif::RGBA8> = image
                .as_bytes()
                .chunks_exact(4)
                .map(|px| ravif::RGBA8::new(px[0], px[1], px[2], px[3]))
                .collect();
            let frame = ravif::Img::new(pixels.as_slice(), width as usize, height as usize);
            out = ravif::Encoder::new()
                .with_quality(f32::from(quality))
                .with_alpha_quality(f32::from(quality))
                .with_speed(AVIF_SPEED)
                .encode_rgba(frame)
                .map_err(|err| Error::Runtime(format!("the Avif encoder failed: {err}")))?
                .avif_file;
        }
        Format::Jxl | Format::Svg | Format::Pdf => {
            return Err(Error::Invalid(format!(
                "{format:?} is a format the image component decodes and does not encode"
            )));
        }
    }
    Ok(out)
}

/// The plan's `quality`, clamped to 1 to 100, or `default` without one.
fn quality(encoding: Encoding, default: u8) -> u8 {
    encoding
        .quality
        .map_or(default, |quality| quality.clamp(1, 100) as u8)
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
