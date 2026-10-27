//! `info`: what an encoded image's header says, read without decoding a pixel.
//!
//! The decoder is constructed, which parses the header and the metadata segments before the pixel
//! data, and is dropped before it reads further. An AVIF is read off its container (`avif`'s
//! `info`) and a JPEG XL off its codestream header, and neither frame is decoded. `frames` is read
//! off the container too: an APNG's `acTL` count, a GIF's image descriptors, a WebP's `ANMF`
//! chunks, the sample count of an AVIF's sequence track and a JPEG XL's keyframe headers. A still
//! image is `1`. An SVG document is parsed whole,
//! because its size is computed from its root element's attributes, and nothing is drawn.
//!
//! **`exif` is every field of the file's EXIF block**, each as its tag's name and its value as
//! `kamadak-exif` names and prints them, in file order. The block is JPEG's APP1 segment, PNG's
//! `eXIf` chunk and WebP's `EXIF` chunk through `image`, an AVIF's `Exif` item and a JPEG XL's
//! `Exif` box. Only the image's own IFDs are listed, so the thumbnail's IFD and the MakerNote,
//! which is a vendor's private format, are left out. A text value is its text, without the
//! quotes `kamadak-exif` prints around it, so a date is `2024:01:02 03:04:05` as the file
//! stores it. `None` is a file with no block. A block that does not parse returns `Parse`, as
//! `decode` does.

use std::io::Cursor;

use exif::{In, Tag, Value};
use image::{ImageDecoder, ImageFormat, ImageReader};
use jxl_oxide::{AuxBoxData, JxlImage, PixelFormat};

use crate::{Error, Format};

/// The header of one encoded image, field for field the WIT `image-info` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Info {
    pub format: Format,
    pub width: u64,
    pub height: u64,
    pub has_alpha: bool,
    pub frames: u64,
    /// The EXIF orientation, `1` to `8`; `1` when the file names none.
    pub orientation: u64,
    pub exif: Option<Vec<(String, String)>>,
    pub has_icc: bool,
}

/// Reads the header of `data`. Bytes in a format the component does not read, and a header that
/// does not parse, return `Parse`.
pub fn info(data: &[u8]) -> Result<Info, Error> {
    match crate::decode::sniff(data) {
        Ok(Format::Svg) => return crate::svg::info(data),
        Ok(Format::Pdf) => return crate::pdf::info(data),
        Ok(Format::Avif) => return crate::avif::info(data),
        Ok(Format::Jxl) => return jxl(data),
        _ => {}
    }
    let mut reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|err| Error::Runtime(err.to_string()))?;
    reader.no_limits();
    let format = match reader.format() {
        Some(ImageFormat::Jpeg) => Format::Jpeg,
        Some(ImageFormat::Png) => Format::Png,
        Some(ImageFormat::WebP) => Format::Webp,
        Some(ImageFormat::Gif) => Format::Gif,
        _ => {
            return Err(Error::Parse(
                "the data is not an image in a format the image component reads".to_string(),
            ));
        }
    };
    let mut decoder = reader
        .into_decoder()
        .map_err(|err| Error::Parse(err.to_string()))?;
    let (width, height) = decoder.dimensions();
    let has_alpha = decoder.color_type().has_alpha();
    let orientation = decoder
        .orientation()
        .map_err(|err| Error::Parse(err.to_string()))?;
    let icc = decoder
        .icc_profile()
        .map_err(|err| Error::Parse(err.to_string()))?;
    let exif = decoder
        .exif_metadata()
        .map_err(|err| Error::Parse(err.to_string()))?
        .filter(|raw| !raw.is_empty())
        .map(|raw| exif_fields(&raw))
        .transpose()?;
    Ok(Info {
        format,
        width: u64::from(width),
        height: u64::from(height),
        has_alpha,
        frames: frames(format, data),
        orientation: u64::from(orientation.to_exif()),
        exif,
        has_icc: icc.is_some_and(|profile| !profile.is_empty()),
    })
}

/// The header of the JPEG XL file `data`. Its size is the stored one, before the orientation
/// field turns it, as every other format's is.
fn jxl(data: &[u8]) -> Result<Info, Error> {
    fn parse(err: impl std::fmt::Display) -> Error {
        Error::Parse(format!("the JPEG XL file does not parse: {err}"))
    }
    let image = JxlImage::builder().read(Cursor::new(data)).map_err(parse)?;
    let header = image.image_header();
    let exif = match image.aux_boxes().first_exif().map_err(parse)? {
        AuxBoxData::Data(raw) => {
            let tiff = &raw.payload()[raw.tiff_header_offset() as usize..];
            Some(exif_fields(tiff)?)
        }
        AuxBoxData::Decoding | AuxBoxData::NotFound => None,
    };
    Ok(Info {
        format: Format::Jxl,
        width: u64::from(header.size.width),
        height: u64::from(header.size.height),
        has_alpha: matches!(image.pixel_format(), PixelFormat::Graya | PixelFormat::Rgba),
        frames: u64::try_from(image.num_loaded_keyframes().max(1)).unwrap_or(u64::MAX),
        orientation: u64::from(header.metadata.orientation),
        exif,
        has_icc: image.original_icc().is_some(),
    })
}

/// The fields of the raw EXIF block `raw`, from its TIFF header on, as the module doc says. A
/// block that does not parse returns `Parse`.
pub(crate) fn exif_fields(raw: &[u8]) -> Result<Vec<(String, String)>, Error> {
    let exif = exif::Reader::new()
        .read_raw(raw.to_vec())
        .map_err(|err| Error::Parse(format!("the EXIF block is malformed: {err}")))?;
    Ok(exif
        .fields()
        .filter(|field| field.ifd_num == In::PRIMARY && field.tag != Tag::MakerNote)
        .map(|field| {
            let value = match &field.value {
                Value::Ascii(lines) => lines
                    .iter()
                    .map(|line| String::from_utf8_lossy(line))
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => field.display_value().with_unit(&exif).to_string(),
            };
            (field.tag.to_string(), value)
        })
        .collect())
}

/// The frame count of a PNG, GIF or WebP `data` in `format`, read off its chunks or blocks
/// without decoding a frame. Every other format is `1`.
pub(crate) fn frames(format: Format, data: &[u8]) -> u64 {
    match format {
        Format::Png => apng_frames(data),
        Format::Gif => gif_frames(data),
        Format::Webp => webp_frames(data),
        _ => 1,
    }
}

/// The frame count an APNG's `acTL` chunk states, or `1` for a PNG without one. Only the chunk
/// headers are read, up to the first `IDAT`, which the `acTL` comes before.
fn apng_frames(data: &[u8]) -> u64 {
    let mut at = 8usize;
    while let Some(head) = data.get(at..at.saturating_add(8)) {
        let length = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        match &head[4..8] {
            b"acTL" => {
                return data
                    .get(at + 8..at + 12)
                    .map_or(1, |n| {
                        u64::from(u32::from_be_bytes([n[0], n[1], n[2], n[3]]))
                    })
                    .max(1);
            }
            b"IDAT" | b"IEND" => break,
            _ => at = at.saturating_add(length).saturating_add(12),
        }
    }
    1
}

/// The number of image descriptors in a GIF. Each one's colour table and LZW data are skipped
/// by their stated sizes, and the walk ends at the trailer, at an unknown block or at the end of
/// the data, so a truncated file counts the frames it has.
fn gif_frames(data: &[u8]) -> u64 {
    let table = |packed: u8| {
        if packed & 0x80 != 0 {
            3usize << ((packed & 7) + 1)
        } else {
            0
        }
    };
    let skip_sub_blocks = |mut at: usize| -> Option<usize> {
        loop {
            let size = *data.get(at)? as usize;
            at += 1 + size;
            if size == 0 {
                return Some(at);
            }
        }
    };
    let Some(&packed) = data.get(10) else {
        return 1;
    };
    let mut at = 13 + table(packed);
    let mut frames = 0u64;
    while let Some(&block) = data.get(at) {
        let next = match block {
            0x2c => data.get(at + 9).and_then(|&packed| {
                frames += 1;
                // The descriptor, the local colour table and the LZW minimum code size.
                skip_sub_blocks(at + 10 + table(packed) + 1)
            }),
            0x21 => skip_sub_blocks(at + 2),
            _ => None,
        };
        match next {
            Some(next) => at = next,
            None => break,
        }
    }
    frames.max(1)
}

/// The number of `ANMF` chunks in a WebP, or `1` for a still image, read off the RIFF chunk
/// headers alone.
fn webp_frames(data: &[u8]) -> u64 {
    let mut at = 12usize;
    let mut frames = 0u64;
    while let Some(head) = data.get(at..at.saturating_add(8)) {
        if &head[..4] == b"ANMF" {
            frames += 1;
        }
        let size = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        at = at
            .saturating_add(8)
            .saturating_add(size)
            .saturating_add(size & 1);
    }
    frames.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    fn png(width: u32, height: u32, color: ExtendedColorType, channels: usize) -> Vec<u8> {
        let pixels = vec![0u8; width as usize * height as usize * channels];
        let mut out = Vec::new();
        PngEncoder::new(&mut out)
            .write_image(&pixels, width, height, color)
            .unwrap();
        out
    }

    #[test]
    fn a_png_header_gives_its_size_and_its_alpha() {
        let got = info(&png(3, 2, ExtendedColorType::Rgba8, 4)).unwrap();
        assert_eq!(
            got,
            Info {
                format: Format::Png,
                width: 3,
                height: 2,
                has_alpha: true,
                frames: 1,
                orientation: 1,
                exif: None,
                has_icc: false,
            }
        );
        assert!(
            !info(&png(1, 1, ExtendedColorType::Rgb8, 3))
                .unwrap()
                .has_alpha
        );
    }

    #[test]
    fn bytes_that_are_no_image_do_not_parse() {
        assert!(matches!(info(b"not an image at all"), Err(Error::Parse(_))));
    }

    #[test]
    fn a_truncated_header_does_not_parse() {
        let whole = png(4, 4, ExtendedColorType::Rgb8, 3);
        assert!(matches!(info(&whole[..12]), Err(Error::Parse(_))));
    }
}
