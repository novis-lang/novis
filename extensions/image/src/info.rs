//! `info`: what an encoded image's header says, read without decoding a pixel.
//!
//! The decoder is constructed, which parses the header and the metadata segments before the pixel
//! data, and is dropped before it reads further. Two fields are not read yet and carry a fixed
//! value: `frames` is `1`, and `exif` is `None`. The EXIF reader and the animation frame count
//! are later slices of goal `ext-image`.

use std::io::Cursor;

use image::{ImageDecoder, ImageFormat, ImageReader};

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

/// Reads the header of `data`. A format the component does not decode returns `Invalid`; a
/// header that does not parse returns `Parse`.
pub fn info(data: &[u8]) -> Result<Info, Error> {
    let reader = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|err| Error::Runtime(err.to_string()))?;
    let format = match reader.format() {
        Some(ImageFormat::Jpeg) => Format::Jpeg,
        Some(ImageFormat::Png) => Format::Png,
        Some(ImageFormat::WebP) => Format::Webp,
        Some(ImageFormat::Gif) => Format::Gif,
        Some(other) => {
            return Err(Error::Invalid(format!(
                "{other:?} is not a format the image component reads"
            )));
        }
        None => {
            return Err(Error::Invalid(
                "the data is not an image in a known format".to_string(),
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
    Ok(Info {
        format,
        width: u64::from(width),
        height: u64::from(height),
        has_alpha,
        frames: 1,
        orientation: u64::from(orientation.to_exif()),
        exif: None,
        has_icc: icc.is_some_and(|profile| !profile.is_empty()),
    })
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
    fn bytes_that_are_no_image_are_invalid() {
        assert!(matches!(
            info(b"not an image at all"),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn a_truncated_header_does_not_parse() {
        let whole = png(4, 4, ExtendedColorType::Rgb8, 3);
        assert!(matches!(info(&whole[..12]), Err(Error::Parse(_))));
    }
}
