//! The `qr` export: a QR code of a string, as an image (`rule:core-classes/image-one-entry-point-per-job`).
//!
//! The string's UTF-8 bytes are encoded by the `qrcode` crate in the smallest version that holds
//! them at the error correction `level`, which is `Medium` when the options name none. Data too
//! long for any version at that level returns `invalid`.
//!
//! The image is `size` pixels square, [`DEFAULT_SIZE`] when the options name none, and is the
//! code's modules with `margin` light modules on every side, [`DEFAULT_MARGIN`] by default: the
//! quiet zone a reader needs. Each pixel is the module its centre falls in, so the modules are
//! as even as `size` allows; a `size` that is not a whole multiple of the modules makes some one
//! pixel wider than others. A `size` below the module count, margin included, returns `invalid`,
//! and so does a `size` whose square is over the default pixel cap. Dark modules are opaque
//! black and light ones opaque white, written as PNG unless the options name another format;
//! a decode-only format returns `encode`'s `invalid`.
//!
//! The export allocates the code's modules and one RGBA8 frame of `size`², and frees both when
//! it returns.

use qrcode::{Color, EcLevel, QrCode};

use crate::{DEFAULT_MAX_PIXELS, Encoding, Error, Format, Pixels, encode};

/// The four error correction levels, the WIT `qr-level`: the share of the code a reader can
/// still recover when it is damaged, about 7, 15, 25 and 30 percent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Low,
    Medium,
    Quartile,
    High,
}

/// The side of the image when the options name no `size`.
pub const DEFAULT_SIZE: u64 = 256;

/// The light modules on every side when the options name no `margin`.
pub const DEFAULT_MARGIN: u64 = 4;

/// The options of one QR code, the WIT `qr-options` with their defaults applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub size: u64,
    pub margin: u64,
    pub level: Level,
    pub format: Format,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            size: DEFAULT_SIZE,
            margin: DEFAULT_MARGIN,
            level: Level::Medium,
            format: Format::Png,
        }
    }
}

/// `data` as a QR code, encoded as the module doc says.
pub fn render(data: &str, options: Options) -> Result<Vec<u8>, Error> {
    encode(frame(data, options)?, options.format, Encoding::default())
}

/// The RGBA8 frame of `data`'s QR code, before it is encoded.
pub fn frame(data: &str, options: Options) -> Result<Pixels, Error> {
    let level = match options.level {
        Level::Low => EcLevel::L,
        Level::Medium => EcLevel::M,
        Level::Quartile => EcLevel::Q,
        Level::High => EcLevel::H,
    };
    let code = QrCode::with_error_correction_level(data.as_bytes(), level).map_err(|err| {
        Error::Invalid(format!(
            "{} bytes do not fit in a QR code at level {:?}: {err}",
            data.len(),
            options.level
        ))
    })?;
    let modules = code.width() as u64;
    let colors = code.to_colors();
    let side = modules.saturating_add(options.margin.saturating_mul(2));
    let size = options.size;
    if size < side {
        return Err(Error::Invalid(format!(
            "this QR code is {side} modules wide with its margin, so `size` must be at least {side}, and it is {size}"
        )));
    }
    if size.saturating_mul(size) > DEFAULT_MAX_PIXELS {
        return Err(Error::Invalid(format!(
            "a QR code of size {size} has more pixels than the cap of {DEFAULT_MAX_PIXELS}"
        )));
    }
    let width = u32::try_from(size).expect("the pixel cap holds `size` to a u32");
    // The module each pixel's centre falls in, counted from the margin's outer edge.
    let module = |pixel: u64| (pixel * 2 + 1) * side / (size * 2);
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        let row = module(y)
            .checked_sub(options.margin)
            .filter(|&m| m < modules);
        for x in 0..size {
            let column = module(x)
                .checked_sub(options.margin)
                .filter(|&m| m < modules);
            let dark = match (row, column) {
                (Some(row), Some(column)) => {
                    colors[(row * modules + column) as usize] == Color::Dark
                }
                _ => false,
            };
            let value = if dark { 0 } else { 255 };
            rgba.extend_from_slice(&[value, value, value, 255]);
        }
    }
    Ok(Pixels {
        width,
        height: width,
        rgba,
        exif: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dark(frame: &Pixels, x: u32, y: u32) -> bool {
        frame.rgba[((y * frame.width + x) * 4) as usize] == 0
    }

    #[test]
    fn the_frame_is_size_square_with_a_light_margin_and_a_dark_finder_corner() {
        let options = Options {
            size: 29,
            margin: 4,
            ..Options::default()
        };
        // "Shop" is a version 1 code, 21 modules wide, so 29 pixels is one per module.
        let frame = frame("Shop", options).unwrap();
        assert_eq!((frame.width, frame.height), (29, 29));
        assert!((0..29).all(|i| !dark(&frame, i, 0) && !dark(&frame, 0, i)));
        assert!(!dark(&frame, 3, 3));
        assert!(dark(&frame, 4, 4));
        assert!(dark(&frame, 10, 4));
        assert!(!dark(&frame, 5, 5));
    }

    #[test]
    fn a_size_below_the_modules_returns_invalid() {
        let options = Options {
            size: 28,
            ..Options::default()
        };
        assert!(matches!(frame("Shop", options), Err(Error::Invalid(_))));
    }

    #[test]
    fn data_too_long_for_its_level_returns_invalid() {
        let data = "x".repeat(2_000);
        let low = Options {
            level: Level::Low,
            ..Options::default()
        };
        let high = Options {
            level: Level::High,
            ..Options::default()
        };
        assert!(frame(&data, low).is_ok());
        assert!(matches!(frame(&data, high), Err(Error::Invalid(_))));
    }

    #[test]
    fn render_returns_a_png() {
        let png = render("https://example.com/", Options::default()).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
    }
}
