//! Decoding an encoded input to RGBA8 pixels, with the pixel cap checked first
//! (`rule:core-classes/image-format-roster`, `rule:core-classes/image-pixel-cap`).
//!
//! **The cap is read off the header.** Each format's decoder is constructed, which parses the
//! header and nothing past it, and the declared width times height is held to the cap before a
//! pixel buffer exists. An image over it returns `Runtime` naming both numbers. The decoders are
//! also handed an allocation limit of two RGBA8 frames at the cap, so a header that lies about its
//! size still cannot make one allocate past it.
//!
//! **Bytes that are not an image the roster decodes return `Parse`**, and so do a header that does
//! not parse, a malformed EXIF block and pixel data that ends early. The EXIF block is parsed in
//! full, because the orientation an upright result needs is in it.
//!
//! JPEG, PNG, WebP and GIF decode through `image`; JPEG XL through `jxl-oxide`. Only the first frame
//! of an animated input is decoded, so `frames` is `1` in the cap. AVIF's header is read for the
//! cap, and its pixel data is not decoded yet: that returns `Runtime`. A CMYK JPEG XL returns
//! `Runtime` too.

use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use jxl_oxide::{JxlImage, PixelFormat};

use crate::{Error, Format};

/// The cap with nothing written in `[image] max_pixels`: `"24M"`, which is 2²⁰ · 24 pixels.
pub const DEFAULT_MAX_PIXELS: u64 = 24 << 20;

/// Decoded pixels: `height` rows of `width` RGBA8 pixels, straight alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The format `data` is in, read from its first bytes. Bytes in no format the roster decodes
/// return `Parse`.
pub fn sniff(data: &[u8]) -> Result<Format, Error> {
    const JXL_CONTAINER: &[u8] = b"\0\0\0\x0cJXL \r\n\x87\n";
    if data.starts_with(&[0xff, 0x0a]) || data.starts_with(JXL_CONTAINER) {
        return Ok(Format::Jxl);
    }
    if data.len() >= 12 && &data[4..8] == b"ftyp" && matches!(&data[8..12], b"avif" | b"avis") {
        return Ok(Format::Avif);
    }
    match image::guess_format(data) {
        Ok(ImageFormat::Jpeg) => Ok(Format::Jpeg),
        Ok(ImageFormat::Png) => Ok(Format::Png),
        Ok(ImageFormat::WebP) => Ok(Format::Webp),
        Ok(ImageFormat::Gif) => Ok(Format::Gif),
        _ => Err(Error::Parse(
            "the data is not an image in a format the image component reads".to_string(),
        )),
    }
}

/// `Runtime` when `width` by `height` is over `cap` pixels.
fn check(width: u64, height: u64, cap: u64) -> Result<(), Error> {
    let pixels = width.saturating_mul(height);
    if pixels > cap {
        return Err(Error::Runtime(format!(
            "the image is {width}x{height}, which is {pixels} pixels, over the cap of {cap} pixels"
        )));
    }
    Ok(())
}

/// The `image` crate's format for one of the formats it decodes here.
fn image_format(format: Format) -> Option<ImageFormat> {
    match format {
        Format::Jpeg => Some(ImageFormat::Jpeg),
        Format::Png => Some(ImageFormat::Png),
        Format::Webp => Some(ImageFormat::WebP),
        Format::Gif => Some(ImageFormat::Gif),
        _ => None,
    }
}

/// Parses the raw EXIF block `exif` in full. A malformed one returns `Parse`.
pub(crate) fn check_exif(exif: Option<Vec<u8>>) -> Result<(), Error> {
    match exif {
        Some(raw) if !raw.is_empty() => exif::Reader::new()
            .read_raw(raw)
            .map(|_| ())
            .map_err(|err| Error::Parse(format!("the EXIF block is malformed: {err}"))),
        _ => Ok(()),
    }
}

/// Decodes `data` to RGBA8, refusing an image over `cap` pixels before a buffer is allocated.
pub fn decode(data: &[u8], cap: u64) -> Result<Pixels, Error> {
    let format = sniff(data)?;
    if let Some(known) = image_format(format) {
        return decode_with_image(data, known, cap);
    }
    match format {
        Format::Jxl => decode_jxl(data, cap),
        Format::Avif => {
            let avif = avif_parse::read_avif(&mut Cursor::new(data))
                .map_err(|err| Error::Parse(format!("the AVIF file does not parse: {err:?}")))?;
            let meta = avif
                .primary_item_metadata()
                .map_err(|err| Error::Parse(format!("the AVIF file does not parse: {err:?}")))?;
            check(
                u64::from(meta.max_frame_width.get()),
                u64::from(meta.max_frame_height.get()),
                cap,
            )?;
            Err(Error::Runtime(
                "AVIF decoding is not available in this build of the image component".to_string(),
            ))
        }
        _ => Err(Error::Invalid(format!(
            "{format:?} is not a format the image component decodes"
        ))),
    }
}

fn decode_with_image(data: &[u8], format: ImageFormat, cap: u64) -> Result<Pixels, Error> {
    let parse = |err: image::ImageError| Error::Parse(err.to_string());
    let mut reader = ImageReader::with_format(Cursor::new(data), format);
    reader.no_limits();
    // A header whose buffer would not fit in the guest's address space is refused by the decoder
    // itself, before it reports a size.
    let mut decoder = reader.into_decoder().map_err(|err| match err {
        image::ImageError::Limits(_) => Error::Runtime(format!(
            "the image declares more pixels than the cap of {cap} pixels"
        )),
        other => parse(other),
    })?;
    let (width, height) = decoder.dimensions();
    check(u64::from(width), u64::from(height), cap)?;
    let mut limits = Limits::no_limits();
    limits.max_alloc = Some(cap.saturating_mul(8));
    decoder.set_limits(limits).map_err(parse)?;
    check_exif(decoder.exif_metadata().map_err(parse)?)?;
    let rgba = DynamicImage::from_decoder(decoder)
        .map_err(parse)?
        .into_rgba8();
    Ok(Pixels {
        width,
        height,
        rgba: rgba.into_raw(),
    })
}

fn decode_jxl(data: &[u8], cap: u64) -> Result<Pixels, Error> {
    fn parse(err: impl std::fmt::Display) -> Error {
        Error::Parse(format!("the JPEG XL file does not parse: {err}"))
    }
    let image = JxlImage::builder().read(Cursor::new(data)).map_err(parse)?;
    let (width, height) = (image.width(), image.height());
    check(u64::from(width), u64::from(height), cap)?;
    let format = image.pixel_format();
    let color = match format {
        PixelFormat::Gray | PixelFormat::Graya => 1,
        PixelFormat::Rgb | PixelFormat::Rgba => 3,
        _ => {
            return Err(Error::Runtime(
                "a CMYK JPEG XL is not available in this build of the image component".to_string(),
            ));
        }
    };
    let alpha = matches!(format, PixelFormat::Graya | PixelFormat::Rgba);
    let render = image.render_frame(0).map_err(parse)?;
    let frame = render.image_all_channels();
    let stride = frame.channels();
    let pixel_count = width as usize * height as usize;
    if frame.buf().len() < pixel_count * stride {
        return Err(Error::Parse(
            "the JPEG XL frame is shorter than its header says".to_string(),
        ));
    }
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for pixel in frame.buf().chunks_exact(stride).take(pixel_count) {
        let (r, g, b) = if color == 1 {
            (pixel[0], pixel[0], pixel[0])
        } else {
            (pixel[0], pixel[1], pixel[2])
        };
        let a = if alpha { pixel[color] } else { 1.0 };
        rgba.extend_from_slice(&[byte(r), byte(g), byte(b), byte(a)]);
    }
    Ok(Pixels {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let pixels = vec![200u8; width as usize * height as usize * 3];
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(&pixels, width, height, ExtendedColorType::Rgb8)
            .unwrap();
        out
    }

    #[test]
    fn a_png_decodes_to_rgba() {
        let got = decode(&png(3, 2), DEFAULT_MAX_PIXELS).unwrap();
        assert_eq!((got.width, got.height), (3, 2));
        assert_eq!(got.rgba.len(), 24);
        assert_eq!(&got.rgba[..4], &[200, 200, 200, 255]);
    }

    #[test]
    fn an_image_over_the_cap_is_refused() {
        let refused = decode(&png(10, 10), 99).unwrap_err();
        assert_eq!(
            refused,
            Error::Runtime(
                "the image is 10x10, which is 100 pixels, over the cap of 99 pixels".to_string()
            )
        );
        assert!(decode(&png(10, 10), 100).is_ok());
    }

    #[test]
    fn bytes_that_are_no_image_do_not_parse() {
        assert!(matches!(decode(b"not an image", 100), Err(Error::Parse(_))));
    }
}
