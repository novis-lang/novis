//! Decoding an encoded input to RGBA8 pixels, with the pixel cap checked first
//! (`rule:core-classes/image-format-roster`, `rule:core-classes/image-pixel-cap`).
//!
//! **The cap is read off the header.** Each format's decoder is constructed, which parses the
//! header and nothing past it, and the declared width times height times the frame count `info`
//! reads is held to the cap before a pixel buffer exists, whether one frame decodes or every one.
//! An image over it returns `Runtime` naming the numbers. The decoders are
//! also handed an allocation limit of two RGBA8 frames at the cap, so a header that lies about its
//! size still cannot make one allocate past it.
//!
//! **Bytes that are not an image the roster decodes return `Parse`**, and so do a header that does
//! not parse, a malformed EXIF block and pixel data that ends early. The EXIF block is parsed in
//! full, because the orientation an upright result needs is in it.
//!
//! JPEG, PNG, WebP and GIF decode through `image`; JPEG XL through `jxl-oxide`; AVIF through
//! `rav1d` (`avif`'s module doc); SVG through `resvg` (`svg`'s module doc), which reads nothing but
//! the bytes it is given. A CMYK JPEG XL returns `Runtime`.
//!
//! **Every frame** of a GIF, an APNG and an animated WebP decodes through [`decode_frames`], each
//! composited onto the canvas by `image`'s animation decoder and kept with its delay, at most as
//! many as the header counted. A JPEG XL animation renders each keyframe through `jxl-oxide`,
//! its delay the frame's duration in ticks over the animation's tick rate, and an AVIF image
//! sequence decodes its track sample by sample (`avif`'s module doc). [`decode`] decodes the
//! first frame alone.
//!
//! **Auto-orient** (`rule:core-classes/image-correct-by-default`) applies the EXIF orientation to
//! the formats `image` decodes, and resets the tag to `1` in the EXIF block `Pixels` carries, so a
//! kept block does not turn the image a second time. An AVIF is turned by its `irot` and `imir`
//! properties (`avif`'s module doc), and a JPEG XL by its codestream's orientation field. Neither
//! carries an EXIF block in `Pixels`.
//!
//! **To sRGB** (`rule:core-classes/image-correct-by-default`) converts the pixels of an input whose
//! embedded ICC profile is RGB or CMYK to sRGB through `moxcms`, for the formats `image` decodes.
//! An AVIF's RGB `colr` profile and a JPEG XL's embedded RGB profile convert the same way. An XYB
//! JPEG XL, or one with no profile, asks `jxl-oxide` for sRGB output, which needs no profile. A
//! CMYK or YCCK JPEG with a CMYK profile is decoded to its raw channels through `zune-jpeg`, read
//! as Adobe stores them (inverted, so `255` is no ink), and converted from ink amounts. Without
//! the option, or without a profile, a CMYK JPEG gets `zune-jpeg`'s conversion, which multiplies
//! each channel by the black one. A profile that does not parse, that is neither RGB nor CMYK, or
//! that `moxcms` cannot connect to sRGB is ignored, as a browser ignores it. The conversion holds
//! a second frame beside the decoded one while it runs.

use std::io::Cursor;

use image::codecs::gif::GifDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::metadata::Orientation;
use image::{
    AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits, RgbaImage,
};
use jxl_oxide::{EnumColourEncoding, JxlImage, PixelFormat, RenderingIntent};
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

/// One decoded frame: its pixels, and how long it shows in milliseconds, `0` for a still image.
/// Only an animation's first frame carries the EXIF block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub pixels: Pixels,
    pub delay_ms: u32,
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
        _ if crate::svg::sniff(data) => Ok(Format::Svg),
        _ if crate::pdf::sniff(data) => Ok(Format::Pdf),
        _ => Err(Error::Parse(
            "the data is not an image in a format the image component reads".to_string(),
        )),
    }
}

/// `Runtime` when `frames` frames of `width` by `height` are over `cap` pixels together.
pub(crate) fn check(width: u64, height: u64, frames: u64, cap: u64) -> Result<(), Error> {
    let pixels = width.saturating_mul(height).saturating_mul(frames);
    if pixels > cap {
        let frames = if frames > 1 {
            format!(" in {frames} frames")
        } else {
            String::new()
        };
        return Err(Error::Runtime(format!(
            "the image is {width}x{height}{frames}, which is {pixels} pixels, over the cap of {cap} pixels"
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
/// ICC profile is converted to sRGB. A PDF is its first page at the default resolution.
pub fn decode(data: &[u8], cap: u64, auto_orient: bool, to_srgb: bool) -> Result<Pixels, Error> {
    decode_page(data, cap, auto_orient, to_srgb, 1, crate::DEFAULT_DPI)
}

/// Decodes `data` as [`decode`] does, and a PDF as its `page`, counted from 1, drawn at `dpi`.
/// A `page` other than 1 of a format with one page returns `Runtime` naming the count.
pub fn decode_page(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
    page: u64,
    dpi: f64,
) -> Result<Pixels, Error> {
    let format = sniff(data)?;
    if format == Format::Pdf {
        return crate::pdf::decode(data, cap, page, dpi);
    }
    if page != 1 {
        return Err(crate::past_end(page, 1));
    }
    if let Some(known) = image_format(format) {
        let first = decode_with_image(data, format, known, cap, auto_orient, to_srgb, false)?;
        return first
            .into_iter()
            .next()
            .map(|frame| frame.pixels)
            .ok_or_else(no_frame);
    }
    match format {
        Format::Jxl => decode_jxl(data, cap, auto_orient, to_srgb, false)?
            .into_iter()
            .next()
            .map(|frame| frame.pixels)
            .ok_or_else(no_frame),
        Format::Avif => crate::avif::decode(data, cap, auto_orient, to_srgb),
        Format::Svg => crate::svg::decode(data, cap),
        _ => Err(Error::Invalid(format!(
            "{format:?} is not a format the image component decodes"
        ))),
    }
}

/// Decodes every frame of `data` as [`decode`] decodes the first, the cap counting all of them.
/// A GIF, an APNG, an animated WebP, an AVIF image sequence and a JPEG XL animation give each
/// frame with its delay; every other format gives its first frame alone.
pub fn decode_frames(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
) -> Result<Vec<Frame>, Error> {
    let format = sniff(data)?;
    match image_format(format) {
        Some(known) => decode_with_image(data, format, known, cap, auto_orient, to_srgb, true),
        None if format == Format::Jxl => decode_jxl(data, cap, auto_orient, to_srgb, true),
        None if format == Format::Avif => {
            crate::avif::decode_frames(data, cap, auto_orient, to_srgb)
        }
        None => Ok(vec![Frame {
            pixels: decode(data, cap, auto_orient, to_srgb)?,
            delay_ms: 0,
        }]),
    }
}

fn no_frame() -> Error {
    Error::Parse("the image has no frame".to_string())
}

/// Decodes `data`, which is in `format` and which `image` reads as `known`: every frame of an
/// animation when `every` is set, and otherwise the first one.
fn decode_with_image(
    data: &[u8],
    format: Format,
    known: ImageFormat,
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
    every: bool,
) -> Result<Vec<Frame>, Error> {
    let parse = |err: image::ImageError| Error::Parse(err.to_string());
    let mut reader = ImageReader::with_format(Cursor::new(data), known);
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
    let count = crate::info::frames(format, data);
    check(u64::from(width), u64::from(height), count, cap)?;
    let mut limits = Limits::no_limits();
    limits.max_alloc = Some(cap.saturating_mul(8));
    decoder.set_limits(limits.clone()).map_err(parse)?;
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
    if auto_orient && let Some(raw) = exif.as_mut() {
        let _ = Orientation::remove_from_exif_chunk(raw);
    }
    let rgb = profile
        .as_ref()
        .filter(|profile| profile.color_space == DataColorSpace::Rgb);
    let finish = |rgba: RgbaImage, exif: Option<Vec<u8>>| {
        let rgba = upright(rgba, rgb, orientation);
        Pixels {
            width: rgba.width(),
            height: rgba.height(),
            rgba: rgba.into_raw(),
            exif,
        }
    };
    if every && count > 1 {
        // `info::frames` counts more than one frame only for a GIF, a PNG or a WebP.
        let cursor = Cursor::new(data);
        let animation = match format {
            Format::Gif => {
                let mut gif = GifDecoder::new(cursor).map_err(parse)?;
                gif.set_limits(limits).map_err(parse)?;
                gif.into_frames()
            }
            Format::Png => {
                let mut png = PngDecoder::new(cursor).map_err(parse)?;
                png.set_limits(limits).map_err(parse)?;
                png.apng().map_err(parse)?.into_frames()
            }
            _ => {
                let mut webp = WebPDecoder::new(cursor).map_err(parse)?;
                webp.set_limits(limits).map_err(parse)?;
                webp.into_frames()
            }
        };
        let mut frames = Vec::new();
        for frame in animation.take(usize::try_from(count).unwrap_or(usize::MAX)) {
            let frame = frame.map_err(parse)?;
            let (numer, denom) = frame.delay().numer_denom_ms();
            frames.push(Frame {
                pixels: finish(frame.into_buffer(), exif.take()),
                delay_ms: numer.checked_div(denom).unwrap_or(0),
            });
        }
        if frames.is_empty() {
            return Err(no_frame());
        }
        return Ok(frames);
    }
    let cmyk = profile.as_ref().filter(|profile| {
        known == ImageFormat::Jpeg && profile.color_space == DataColorSpace::Cmyk
    });
    let rgba = match cmyk.and_then(|profile| cmyk_jpeg(data, profile).transpose()) {
        Some(converted) => converted?,
        None => DynamicImage::from_decoder(decoder)
            .map_err(parse)?
            .into_rgba8(),
    };
    Ok(vec![Frame {
        pixels: finish(rgba, exif),
        delay_ms: 0,
    }])
}

/// `rgba` converted from the RGB `profile` to sRGB when there is one, then turned by
/// `orientation`.
fn upright(
    mut rgba: RgbaImage,
    profile: Option<&ColorProfile>,
    orientation: Orientation,
) -> RgbaImage {
    if let Some(converted) = profile.and_then(|profile| srgb(profile, &rgba)) {
        rgba = RgbaImage::from_raw(rgba.width(), rgba.height(), converted)
            .expect("the conversion keeps the frame's size");
    }
    let mut image = DynamicImage::ImageRgba8(rgba);
    image.apply_orientation(orientation);
    image.into_rgba8()
}

/// The four-channel `pixels` converted from `profile` to sRGB RGBA8: RGBA for an RGB profile, ink
/// amounts for a CMYK one, whose result is opaque. `None` when `moxcms` cannot connect the profile
/// to sRGB.
pub(crate) fn srgb(profile: &ColorProfile, pixels: &[u8]) -> Option<Vec<u8>> {
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
            pixel[1] = (y - 0.344_136 * cb - 0.714_136 * cr)
                .round()
                .clamp(0.0, 255.0) as u8;
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

/// Decodes the JPEG XL `data`: every keyframe of an animation when `every` is set, each with its
/// duration in ticks over the animation's tick rate, and otherwise the first one. `jxl-oxide`
/// applies the codestream's orientation to every frame it renders, so without `auto_orient` the
/// inverse turn is applied after it.
fn decode_jxl(
    data: &[u8],
    cap: u64,
    auto_orient: bool,
    to_srgb: bool,
    every: bool,
) -> Result<Vec<Frame>, Error> {
    fn parse(err: impl std::fmt::Display) -> Error {
        Error::Parse(format!("the JPEG XL file does not parse: {err}"))
    }
    let mut image = JxlImage::builder().read(Cursor::new(data)).map_err(parse)?;
    let (width, height) = (image.width(), image.height());
    let frames = u64::try_from(image.num_loaded_keyframes().max(1)).unwrap_or(u64::MAX);
    check(u64::from(width), u64::from(height), frames, cap)?;
    // `jxl-oxide` renders an XYB image, or one with no profile, to sRGB itself. Any other image
    // renders in its embedded profile, which converts below as the other formats' does.
    let embedded = image.original_icc().is_some() && !image.image_header().metadata.xyb_encoded;
    let profile = if to_srgb && embedded {
        image
            .original_icc()
            .and_then(|raw| ColorProfile::new_from_slice(raw).ok())
            .filter(|profile| profile.color_space == DataColorSpace::Rgb)
    } else {
        None
    };
    if to_srgb && !embedded {
        match image.pixel_format() {
            PixelFormat::Gray | PixelFormat::Graya => {
                image.request_color_encoding(EnumColourEncoding::gray_srgb(
                    RenderingIntent::Relative,
                ));
            }
            PixelFormat::Rgb | PixelFormat::Rgba => {
                image.request_color_encoding(EnumColourEncoding::srgb(RenderingIntent::Relative));
            }
            _ => {}
        }
    }
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
    let orientation = image.image_header().metadata.orientation;
    // A tick lasts `tps_denominator / tps_numerator` seconds.
    let ticks = image
        .image_header()
        .metadata
        .animation
        .as_ref()
        .map(|animation| (animation.tps_numerator, animation.tps_denominator));
    let render = |keyframe: usize| -> Result<Pixels, Error> {
        let render = image.render_frame(keyframe).map_err(parse)?;
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
        drop(render);
        if let Some(converted) = profile.as_ref().and_then(|profile| srgb(profile, &rgba)) {
            rgba = converted;
        }
        if auto_orient || orientation == 1 {
            return Ok(Pixels {
                width,
                height,
                rgba,
                exif: None,
            });
        }
        // Orientations 6 and 8 turn by a quarter in opposite directions. Each other one undoes
        // itself.
        let inverse = match orientation {
            6 => 8,
            8 => 6,
            other => other,
        };
        let rgba = RgbaImage::from_raw(width, height, rgba).expect("the frame is width by height");
        let mut stored = DynamicImage::ImageRgba8(rgba);
        if let Some(turn) = u8::try_from(inverse).ok().and_then(Orientation::from_exif) {
            stored.apply_orientation(turn);
        }
        let stored = stored.into_rgba8();
        Ok(Pixels {
            width: stored.width(),
            height: stored.height(),
            rgba: stored.into_raw(),
            exif: None,
        })
    };
    let count = if every {
        image.num_loaded_keyframes().max(1)
    } else {
        1
    };
    let mut out = Vec::new();
    for keyframe in 0..count {
        let delay_ms = match (ticks, image.frame_header(keyframe)) {
            (Some((numerator, denominator)), Some(header)) => u64::from(header.duration)
                .saturating_mul(u64::from(denominator))
                .saturating_mul(1000)
                .checked_div(u64::from(numerator))
                .map_or(0, |ms| u32::try_from(ms).unwrap_or(u32::MAX)),
            _ => 0,
        };
        out.push(Frame {
            pixels: render(keyframe)?,
            delay_ms,
        });
    }
    Ok(out)
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
        assert!(matches!(
            decode(b"not an image", 100, true, true),
            Err(Error::Parse(_))
        ));
    }
}
