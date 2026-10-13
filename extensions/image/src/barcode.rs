//! The `barcode` export: a 1D code or a DataMatrix of a string, as an image or an SVG document
//! (`rule:core-classes/image-barcode-roster`).
//!
//! Each symbology checks its data before anything is drawn, and returns `invalid` naming what is
//! wrong: a character the symbology cannot encode and its position, a length it does not take, or
//! a check digit that is not the one the digits before it give. A check digit is never computed
//! for the program, so EAN-13 takes 13 digits, EAN-8 8, UPC-A 12 and ITF-14 14, each ending in its
//! check digit. Code 128 carries printable ASCII. Code 39 carries the digits, the capital letters,
//! the space and `-.$/+%`. GS1-128 takes element strings written `(01)09501101530003(10)ABC`: each
//! application identifier is 2 to 4 digits in brackets, a value is printable ASCII, and the GTIN
//! of `(01)` and `(02)` and the SSCC of `(00)` are checked. Every value after the first follows
//! an FNC1. The 1D codes are `barcoders`'s and DataMatrix is `datamatrix`'s, in the smallest
//! square symbol that holds the data.
//!
//! A code is its modules with `margin` light modules on every side: [`MARGIN_1D`] for a 1D code
//! and [`MARGIN_2D`] for DataMatrix when the options name none. A module is `module` pixels wide,
//! [`MODULE_1D`] or [`MODULE_2D`] by default, and a DataMatrix module is as high as it is wide. A
//! 1D code's bars are `height` pixels high, [`DEFAULT_HEIGHT`] by default. With `text`, a 1D code
//! prints its data under the bars in Liberation Sans, [`TEXT_PER_MODULE`] times `module` pixels
//! high; DataMatrix prints no text. A code over the default pixel cap returns `invalid`.
//!
//! Dark modules are opaque black and light ones opaque white. The output is PNG unless the
//! options name another format, and `svg` is an SVG document, white, with the dark modules as
//! one path and the text as a `text` element; [`svg`] writes it, and `qr`'s SVG output too.
//!
//! The export allocates the code's modules and, for a raster format, one RGBA8 frame of the image,
//! and frees both when it returns.

use std::fmt::Write as _;

use barcoders::sym::{code39::Code39, code128::Code128, ean8::EAN8, ean13::EAN13, tf::TF};
use datamatrix::{DataMatrix, SymbolList};

use crate::ops::{self, Blend, Composite, Gravity};
use crate::text::{self, Align, Layout};
use crate::{DEFAULT_MAX_PIXELS, Encoding, Error, Format, Pixels, encode};

/// The codes the export draws, the WIT `symbology`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbology {
    Code128,
    Gs1128,
    Ean13,
    Ean8,
    UpcA,
    Code39,
    Itf14,
    DataMatrix,
}

/// A 1D code's bar height in pixels when the options name none.
pub const DEFAULT_HEIGHT: u64 = 80;

/// The pixels of a 1D code's narrowest bar when the options name no `module`.
pub const MODULE_1D: u64 = 2;

/// The pixels of a DataMatrix module's side when the options name no `module`.
pub const MODULE_2D: u64 = 4;

/// The light modules on every side of a 1D code when the options name no `margin`.
pub const MARGIN_1D: u64 = 10;

/// The light modules on every side of a DataMatrix when the options name no `margin`.
pub const MARGIN_2D: u64 = 2;

/// The height of a 1D code's text, in modules.
pub const TEXT_PER_MODULE: u64 = 8;

/// The options of one code, the WIT `barcode-options`; an absent one takes the module doc's
/// default for the symbology.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    pub height: Option<u64>,
    pub module: Option<u64>,
    pub margin: Option<u64>,
    pub text: bool,
    pub format: Option<Format>,
}

/// The modules of a code, row by row, `true` for dark. A 1D code is one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    pub width: u64,
    pub height: u64,
    pub dark: Vec<bool>,
}

impl Grid {
    fn at(&self, x: u64, y: u64) -> bool {
        self.dark[(y * self.width + x) as usize]
    }

    /// Each run of dark modules in a row: its row, its first column and its length.
    fn runs(&self) -> impl Iterator<Item = (u64, u64, u64)> + '_ {
        (0..self.height).flat_map(move |y| {
            let mut x = 0;
            std::iter::from_fn(move || {
                while x < self.width && !self.at(x, y) {
                    x += 1;
                }
                let start = x;
                while x < self.width && self.at(x, y) {
                    x += 1;
                }
                (x > start).then_some((y, start, x - start))
            })
        })
    }
}

/// An SVG document of `size` pixels whose user units are `view`, white, with each dark module
/// of `grid` a `cell` wide and high from `origin`, both in user units, and `extra` after them.
pub fn svg(
    size: (u64, u64),
    view: (u64, u64),
    grid: &Grid,
    origin: (u64, u64),
    cell: (u64, u64),
    extra: &str,
) -> Vec<u8> {
    let mut path = String::new();
    for (y, x, run) in grid.runs() {
        let (left, top) = (origin.0 + x * cell.0, origin.1 + y * cell.1);
        let _ = write!(
            path,
            "M{left} {top}h{}v{}h-{}z",
            run * cell.0,
            cell.1,
            run * cell.0
        );
    }
    format!(
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" shape-rendering="crispEdges">"#,
            r##"<rect width="{}" height="{}" fill="#fff"/><path d="{}" fill="#000"/>{}</svg>"##
        ),
        size.0, size.1, view.0, view.1, view.0, view.1, path, extra
    )
    .into_bytes()
}

/// `data` as a `symbology` code, encoded as the module doc says.
pub fn render(data: &str, symbology: Symbology, options: Options) -> Result<Vec<u8>, Error> {
    let (grid, caption) = modules(data, symbology)?;
    let linear = symbology != Symbology::DataMatrix;
    let module = options
        .module
        .unwrap_or(if linear { MODULE_1D } else { MODULE_2D });
    let margin = options
        .margin
        .unwrap_or(if linear { MARGIN_1D } else { MARGIN_2D });
    if module == 0 || (linear && options.height == Some(0)) {
        return Err(Error::Invalid(
            "a barcode needs a `module` and a `height` above 0".to_owned(),
        ));
    }
    let bars = if linear {
        options.height.unwrap_or(DEFAULT_HEIGHT)
    } else {
        grid.height.saturating_mul(module)
    };
    let text = (linear && options.text).then(|| module.saturating_mul(TEXT_PER_MODULE));
    let pad = margin.saturating_mul(module);
    let width = grid
        .width
        .saturating_add(margin.saturating_mul(2))
        .saturating_mul(module);
    let height = pad
        .saturating_mul(2)
        .saturating_add(bars)
        .saturating_add(text.unwrap_or(0));
    if width.saturating_mul(height) > DEFAULT_MAX_PIXELS {
        return Err(Error::Invalid(format!(
            "a barcode of {width} by {height} pixels has more pixels than the cap of {DEFAULT_MAX_PIXELS}"
        )));
    }
    let cell = (module, if linear { bars } else { module });
    if options.format == Some(Format::Svg) {
        let caption = text.map_or(String::new(), |size| {
            format!(
                r#"<text x="{}" y="{}" font-family="Liberation Sans, Arial, sans-serif" font-size="{size}" text-anchor="middle">{}</text>"#,
                width / 2,
                pad + bars + size * 4 / 5,
                escape(&caption)
            )
        });
        return Ok(svg(
            (width, height),
            (width, height),
            &grid,
            (pad, pad),
            cell,
            &caption,
        ));
    }
    let mut frame = Pixels {
        width: u32::try_from(width).expect("the pixel cap holds the width to a u32"),
        height: u32::try_from(height).expect("the pixel cap holds the height to a u32"),
        rgba: [255; 4].repeat((width * height) as usize),
        exif: None,
    };
    for (y, x, run) in grid.runs() {
        let top = pad + y * cell.1;
        let left = (pad + x * cell.0) as usize;
        for row in top..top + cell.1 {
            let start = (row * width) as usize + left;
            for px in
                frame.rgba[start * 4..(start + (run * cell.0) as usize) * 4].chunks_exact_mut(4)
            {
                px.copy_from_slice(&[0, 0, 0, 255]);
            }
        }
    }
    if let Some(size) = text {
        let layout = Layout {
            text: &caption,
            font: crate::pdf::sans(),
            size: size as f64,
            max_width: None,
            align: Align::Center,
        };
        let layer = text::render(&layout, [0, 0, 0, 255], DEFAULT_MAX_PIXELS)?;
        let x = (i64::from(frame.width) - i64::from(layer.width)) / 2;
        let composite = Composite {
            overlay: 0,
            gravity: Gravity::Center,
            x: Some(x),
            y: Some((pad + bars) as i64),
            opacity: 1.0,
            blend: Blend::Normal,
        };
        ops::composite(&mut frame, &layer, composite);
    }
    encode(
        frame,
        options.format.unwrap_or(Format::Png),
        Encoding::default(),
    )
}

/// `text` with the five characters XML reserves escaped.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// The modules of `data`'s code and the text printed under it, or `Invalid` naming what the
/// symbology does not take.
fn modules(data: &str, symbology: Symbology) -> Result<(Grid, String), Error> {
    let bars = match symbology {
        Symbology::Code128 => {
            printable(data, "Code 128", |c| (' '..='~').contains(&c))?;
            Code128::new(format!("\u{181}{data}")).map(|code| code.encode())
        }
        Symbology::Gs1128 => {
            let groups = gs1(data)?;
            let joined: Vec<String> = groups
                .iter()
                .map(|(ai, value)| format!("{ai}{value}"))
                .collect();
            Code128::new(format!("\u{181}\u{179}{}", joined.join("\u{179}")))
                .map(|code| code.encode())
        }
        Symbology::Ean13 => {
            digits(data, "EAN-13", 13)?;
            EAN13::new(data).map(|code| code.encode())
        }
        Symbology::Ean8 => {
            digits(data, "EAN-8", 8)?;
            EAN8::new(data).map(|code| code.encode())
        }
        Symbology::UpcA => {
            digits(data, "UPC-A", 12)?;
            EAN13::new(format!("0{data}")).map(|code| code.encode())
        }
        Symbology::Code39 => {
            printable(data, "Code 39", |c| {
                c.is_ascii_digit() || c.is_ascii_uppercase() || " -.$/+%".contains(c)
            })?;
            Code39::new(data).map(|code| code.encode())
        }
        Symbology::Itf14 => {
            digits(data, "ITF-14", 14)?;
            TF::interleaved(data).map(|code| code.encode())
        }
        Symbology::DataMatrix => {
            let code = DataMatrix::encode_str(data, SymbolList::default()).map_err(|err| {
                Error::Invalid(format!(
                    "{} bytes do not fit in a DataMatrix: {err:?}",
                    data.len()
                ))
            })?;
            let bitmap = code.bitmap();
            let grid = Grid {
                width: bitmap.width() as u64,
                height: bitmap.height() as u64,
                dark: bitmap.bits().to_vec(),
            };
            return Ok((grid, data.to_owned()));
        }
    }
    .map_err(|err| Error::Invalid(format!("{symbology:?} cannot encode {data:?}: {err}")))?;
    let grid = Grid {
        width: bars.len() as u64,
        height: 1,
        dark: bars.iter().map(|&bar| bar == 1).collect(),
    };
    Ok((grid, data.to_owned()))
}

/// `Invalid` naming the first character of `data` that `allowed` refuses, or an empty `data`.
fn printable(data: &str, name: &str, allowed: impl Fn(char) -> bool) -> Result<(), Error> {
    if data.is_empty() {
        return Err(Error::Invalid(format!(
            "a {name} code needs at least one character"
        )));
    }
    match data.chars().enumerate().find(|&(_, c)| !allowed(c)) {
        Some((at, c)) => Err(Error::Invalid(format!(
            "{name} cannot encode the character {c:?} at position {}",
            at + 1
        ))),
        None => Ok(()),
    }
}

/// `Invalid` unless `data` is `count` digits whose last is the GS1 check digit of the others.
fn digits(data: &str, name: &str, count: usize) -> Result<(), Error> {
    if data.len() != count || !data.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Invalid(format!(
            "{name} takes {count} digits, its check digit last, and was given {data:?}"
        )));
    }
    check_digit(data, name)
}

/// `Invalid` naming both digits when the last digit of `data` is not the GS1 check digit of the
/// digits before it: weights 3 and 1 in turn from the right, and the step to the next ten.
fn check_digit(data: &str, name: &str) -> Result<(), Error> {
    let digits: Vec<u32> = data.chars().filter_map(|c| c.to_digit(10)).collect();
    let (given, body) = digits.split_last().expect("a code has at least one digit");
    let sum: u32 = body
        .iter()
        .rev()
        .enumerate()
        .map(|(at, d)| if at % 2 == 0 { d * 3 } else { *d })
        .sum();
    let expected = (10 - sum % 10) % 10;
    if *given != expected {
        return Err(Error::Invalid(format!(
            "the check digit of the {name} {data} is {expected}, and the data ends in {given}"
        )));
    }
    Ok(())
}

/// `data`'s GS1 element strings, each application identifier with its value.
fn gs1(data: &str) -> Result<Vec<(&str, &str)>, Error> {
    let fail = |why: String| Error::Invalid(format!("GS1-128 cannot encode {data:?}: {why}"));
    let mut groups = Vec::new();
    let mut rest = data;
    while !rest.is_empty() {
        let Some(open) = rest.strip_prefix('(') else {
            return Err(fail(
                "each element string starts with its identifier in brackets".to_owned(),
            ));
        };
        let close = open
            .find(')')
            .ok_or_else(|| fail("an identifier has no closing bracket".to_owned()))?;
        let ai = &open[..close];
        if !(2..=4).contains(&ai.len()) || !ai.bytes().all(|b| b.is_ascii_digit()) {
            return Err(fail(format!("the identifier ({ai}) is not 2 to 4 digits")));
        }
        let after = &open[close + 1..];
        let end = after.find('(').unwrap_or(after.len());
        let value = &after[..end];
        if value.is_empty() {
            return Err(fail(format!("the identifier ({ai}) has no value")));
        }
        printable(value, "GS1-128", |c| ('!'..='~').contains(&c) && c != '(')?;
        match ai {
            "00" => digits(value, "SSCC", 18)?,
            "01" | "02" => digits(value, "GTIN", 14)?,
            _ => {}
        }
        groups.push((ai, value));
        rest = &after[end..];
    }
    if groups.is_empty() {
        return Err(fail("there is no element string".to_owned()));
    }
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wrong_check_digit_returns_invalid_naming_both_digits() {
        match render("4006381333932", Symbology::Ean13, Options::default()) {
            Err(Error::Invalid(message)) => {
                assert!(
                    message.contains("is 1") && message.contains("ends in 2"),
                    "{message}"
                );
            }
            other => panic!("{other:?}"),
        }
        assert!(render("4006381333931", Symbology::Ean13, Options::default()).is_ok());
        assert!(matches!(
            render("400638133393", Symbology::Ean13, Options::default()),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn a_character_the_symbology_cannot_carry_is_named() {
        match render("Shop", Symbology::Code39, Options::default()) {
            Err(Error::Invalid(message)) => {
                assert!(message.contains("'h' at position 2"), "{message}")
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn gs1_checks_the_gtin_and_reads_each_element_string() {
        assert_eq!(
            gs1("(01)09501101530003(10)ABC").unwrap(),
            vec![("01", "09501101530003"), ("10", "ABC")]
        );
        assert!(matches!(gs1("(01)09501101530004"), Err(Error::Invalid(_))));
        assert!(matches!(gs1("01)0950"), Err(Error::Invalid(_))));
    }

    #[test]
    fn a_1d_code_is_its_modules_and_margin_wide() {
        let (grid, _) = modules("96385074", Symbology::Ean8).unwrap();
        assert_eq!(grid.width, 67);
        let png = render("96385074", Symbology::Ean8, Options::default()).unwrap();
        assert!(png.starts_with(b"\x89PNG"));
    }

    #[test]
    fn svg_output_is_one_path_and_the_text() {
        let options = Options {
            text: true,
            format: Some(Format::Svg),
            ..Options::default()
        };
        let svg =
            String::from_utf8(render("Shop & Blog", Symbology::Code128, options).unwrap()).unwrap();
        assert!(
            svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""),
            "{svg}"
        );
        assert!(
            svg.contains("<path d=\"M") && svg.contains(">Shop &amp; Blog</text>"),
            "{svg}"
        );
    }
}
