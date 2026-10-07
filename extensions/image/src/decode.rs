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
//! JPEG, PNG, WebP and GIF decode through `image`; JPEG XL through `jxl-oxide`; AVIF through
//! `rav1d` (`avif`'s module doc). Only the first frame of an animated input is decoded, so `frames`
//! is `1` in the cap. A CMYK JPEG XL returns `Runtime`.
//!
//! **Auto-orient** (`rule:core-classes/image-correct-by-default`) applies the EXIF orientation to
//! the formats `image` decodes, and resets the tag to `1` in the EXIF block `Pixels` carries, so a
//! kept block does not turn the image a second time. AVIF and JPEG XL return their frame as
//! `rav1d` and `jxl-oxide` give it, with no EXIF block.
//!
//! **To sRGB** (`rule:core-classes/image-correct-by-default`) converts the pixels of an input whose
//! embedded ICC profile is RGB or CMYK to sRGB through `moxcms`, for the formats `image` decodes. A
//! CMYK or YCCK JPEG with a CMYK profile is decoded to its raw channels through `zune-jpeg`, read
//! as Adobe stores them (inverted, so `255` is no ink), and converted from ink amounts. Without
//! the option, or without a profile, a CMYK JPEG gets `zune-jpeg`'s conversion, which multiplies
//! each channel by the black one. A profile that does not parse, that is neither RGB nor CMYK, or
//! that `moxcms` cannot connect to sRGB is ignored, as a browser ignores it. The conversion holds
//! a second frame beside the decoded one while it runs.

use std::io::Cursor;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbaImage};
use jxl_oxide::{JxlImage, PixelFormat};
use moxcms::{ColorProfile, DataColorSpace, Layout, TransformOptions};
use zune_core::bytestream::ZCursor;
use zune_core::colorspace::ColorSpace;
use zune_core::options::DecoderOptions;

use crate::{Error, Format};

/// The cap with nothing written in `[image] max_pixels`: `"24M"`, which is 2²⁰ · 24 pixels.
pub const DEFAULT_MAX_PIXELS: u64 = 24 << 20;

/// Decoded pixels: `height` rows of `width` RGBA8 pixels, straight alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    /// The input's raw EXIF block, from the TIFF header on, for an encoder that keeps metadata.
    /// When the orientation was applied to the pixels, the block's orientation tag says `1`.
    pub exif: Option<Vec<u8>>,
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
pub(crate) fn check(width: u64, height: u64, cap: u64) -> Result<(), Error> {
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
fn check_exif(exif: Option<&[u8]>) -> Result<(), Error> {
    match exif {
        Some(raw) if !raw.is_empty() => exif::Reader::new()
            .read_raw(raw.to_vec())
            .map(|_| ())
            .map_err(|err| Error::Parse(format!("the EXIF block is malformed: {err}"))),
        _ => Ok(()),
    }
}

/// Decodes `data` to RGBA8, refusing an image over `cap` pixels before a buffer is allocated.
/// With `auto_orient`, the EXIF orientation is applied to the pixels; with `to_srgb`, an embedded
/// ICC profile is converted to sRGB.
pub fn decode(data: &[u8], cap: u64, auto_orient: bool, to_srgb: bool) -> Result<Pixels, Error> {
    let format = sniff(data)?;
    if let Some(known) = image_format(format) {
        return decode_with_image(data, known, cap, auto_orient, to_srgb);
    }
    match format {
        Format::Jxl => decode_jxl(data, cap),
        Format::Avif => crate::avif::decode(data, cap),
        _ => Err(Error::Invalid(format!(
            "{format:?} is not a format the image component decodes"
        ))),
    }
}

fn decode_with_image(
    data: &[u8],
    format: ImageFormat,
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
) -> Result<Pixels, Error> {
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
    let mut exif = decoder
        .exif_metadata()
        .map_err(parse)?
        .filter(|raw| !raw.is_empty());
    check_exif(exif.as_deref())?;
    let orientation = if auto_orient {
        decoder.orientation().map_err(parse)?
    } else {
        Orientation::NoTransforms
    };
    let profile = if to_srgb {
        decoder
            .icc_profile()
            .map_err(parse)?
            .and_then(|raw| ColorProfile::new_from_slice(&raw).ok())
    } else {
        None
    };
    let cmyk = profile
        .as_ref()
        .filter(|profile| format == ImageFormat::Jpeg && profile.color_space == DataColorSpace::Cmyk);
    let rgba = match cmyk.and_then(|profile| cmyk_jpeg(data, profile).transpose()) {
        Some(converted) => converted?,
        None => {
            let mut rgba = DynamicImage::from_decoder(decoder).map_err(parse)?.into_rgba8();
            if let Some(profile) = profile.filter(|profile| profile.color_space == DataColorSpace::Rgb) {
                if let Some(converted) = srgb(&profile, &rgba) {
                    rgba = RgbaImage::from_raw(rgba.width(), rgba.height(), converted)
                        .expect("the conversion keeps the frame's size");
                }
            }
            rgba
        }
    };
    let mut image = DynamicImage::ImageRgba8(rgba);
    if auto_orient {
        image.apply_orientation(orientation);
        if let Some(raw) = exif.as_mut() {
            let _ = Orientation::remove_from_exif_chunk(raw);
        }
    }
    let rgba = image.into_rgba8();
    Ok(Pixels {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
        exif,
    })
}

/// The four-channel `pixels` converted from `profile` to sRGB RGBA8: RGBA for an RGB profile, ink
/// amounts for a CMYK one, whose result is opaque. `None` when `moxcms` cannot connect the profile
/// to sRGB.
fn srgb(profile: &ColorProfile, pixels: &[u8]) -> Option<Vec<u8>> {
    let transform = profile
        .create_transform_8bit(
            Layout::Rgba,
            &ColorProfile::new_srgb(),
            Layout::Rgba,
            TransformOptions::default(),
        )
        .ok()?;
    let mut out = vec![0u8; pixels.len()];
    transform.transform(pixels, &mut out).ok()?;
    if profile.color_space == DataColorSpace::Cmyk {
        for pixel in out.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }
    Some(out)
}

/// The CMYK or YCCK JPEG `data` decoded to its raw channels and converted from `profile` to sRGB.
/// `None` when the JPEG is not stored as CMYK or YCCK, or when the conversion is not possible, so
/// the caller decodes it the ordinary way.
fn cmyk_jpeg(data: &[u8], profile: &ColorProfile) -> Result<Option<RgbaImage>, Error> {
    let parse = |err: zune_jpeg::errors::DecodeErrors| Error::Parse(err.to_string());
    let options = DecoderOptions::default()
        .set_strict_mode(false)
        .set_max_width(usize::MAX)
        .set_max_height(usize::MAX);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(data), options);
    decoder.decode_headers().map_err(parse)?;
    let stored = match decoder.input_colorspace() {
        Some(stored @ (ColorSpace::CMYK | ColorSpace::YCCK)) => stored,
        _ => return Ok(None),
    };
    let (width, height) = decoder.dimensions().expect("the headers were decoded");
    decoder.set_options(options.jpeg_set_out_colorspace(stored));
    let mut channels = decoder.decode().map_err(parse)?;
    if channels.len() != width * height * 4 {
        return Err(Error::Parse(
            "the JPEG's pixel data is shorter than its header says".to_string(),
        ));
    }
    // A YCCK JPEG encodes `255 - stored` for cyan, magenta and yellow as YCbCr, so converting
    // that back to RGB gives their ink amounts directly. Black is stored as it is in CMYK.
    for pixel in channels.chunks_exact_mut(4) {
        if stored == ColorSpace::YCCK {
            let (y, cb, cr) = (
                f32::from(pixel[0]),
                f32::from(pixel[1]) - 128.0,
                f32::from(pixel[2]) - 128.0,
            );
            pixel[0] = (y + 1.402 * cr).round().clamp(0.0, 255.0) as u8;
            pixel[1] = (y - 0.344_136 * cb - 0.714_136 * cr).round().clamp(0.0, 255.0) as u8;
            pixel[2] = (y + 1.772 * cb).round().clamp(0.0, 255.0) as u8;
        } else {
            for channel in &mut pixel[..3] {
                *channel = 255 - *channel;
            }
        }
        pixel[3] = 255 - pixel[3];
    }
    let Some(rgba) = srgb(profile, &channels) else {
        return Ok(None);
    };
    drop(channels);
    let width = u32::try_from(width).expect("a JPEG is at most 65535 pixels wide");
    let height = u32::try_from(height).expect("a JPEG is at most 65535 pixels high");
    Ok(RgbaImage::from_raw(width, height, rgba))
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
        exif: None,
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
        let got = decode(&png(3, 2), DEFAULT_MAX_PIXELS, true, true).unwrap();
        assert_eq!((got.width, got.height), (3, 2));
        assert_eq!(got.rgba.len(), 24);
        assert_eq!(&got.rgba[..4], &[200, 200, 200, 255]);
    }

    #[test]
    fn an_image_over_the_cap_is_refused() {
        let refused = decode(&png(10, 10), 99, true, true).unwrap_err();
        assert_eq!(
            refused,
            Error::Runtime(
                "the image is 10x10, which is 100 pixels, over the cap of 99 pixels".to_string()
            )
        );
        assert!(decode(&png(10, 10), 100, true, true).is_ok());
    }

    #[test]
    fn bytes_that_are_no_image_do_not_parse() {
        assert!(matches!(decode(b"not an image", 100, true, true), Err(Error::Parse(_))));
    }
}
