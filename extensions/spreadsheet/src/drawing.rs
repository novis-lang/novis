//! Images and charts on a sheet: the media entries and the sheet's `drawing<n>.xml`
//! (`rule:core-classes/spreadsheet-sheet-end-parts`).
//!
//! [`read`] checks an image's bytes and finds its format and its size in pixels from the file's
//! own header: PNG's `IHDR`, GIF's logical screen and JPEG's first start-of-frame segment. Any
//! other bytes are an error, and nothing is converted. The image is the zip entry
//! `xl/media/image<n>.<ext>`, numbered across the workbook, and [`Drawing`] keeps only where each
//! one is anchored: one cell, an offset from its top-left corner and a size, in pixels, each
//! written in EMU (9,525 to a pixel). A size the program leaves out is the image's own, and one
//! side given alone scales the other by the image's aspect ratio. A chart is anchored the same
//! way, as a `graphicFrame` pointing at its own `xl/charts/chart<n>.xml` (`chart`'s module doc).
//! [`Drawing::write`] is the `xl/drawings/drawing<sheet>.xml` part with one `oneCellAnchor` per
//! image or chart, and [`Drawing::rels`] its relationships to the media and the charts.
//!
//! Memory: per image, its anchor and its entry's name, and per chart its anchor and what
//! `chart`'s module doc keeps. The image bytes go to the archive at once, or, when the sheet's own
//! entry is open, wait for it to end (`writer`'s module doc).

use std::fmt::Write as _;

use crate::Error;
use crate::chart::{self, Chart, Planned};
use crate::tail::{bad_cell, cell_at};

const RELS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const PACKAGE_RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";

/// EMU in one pixel at 96 dots per inch.
const EMU_PER_PIXEL: f64 = 9_525.0;

/// The image formats a sheet takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Format {
    Png,
    Jpeg,
    Gif,
}

impl Format {
    /// The extension of its media entry.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpeg",
            Format::Gif => "gif",
        }
    }

    /// The content type `[Content_Types].xml` gives its extension.
    pub fn content_type(self) -> &'static str {
        match self {
            Format::Png => "image/png",
            Format::Jpeg => "image/jpeg",
            Format::Gif => "image/gif",
        }
    }
}

/// An image the program adds at the cell `at`, with an optional size and offset in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub at: String,
    pub image: Vec<u8>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub offset_x: Option<f64>,
    pub offset_y: Option<f64>,
}

/// The format of `bytes` and the image's width and height in pixels, or `None` when they are not
/// a PNG, JPEG or GIF with a size.
pub fn read(bytes: &[u8]) -> Option<(Format, u32, u32)> {
    let u16_be = |at: usize| {
        Some(u32::from(u16::from_be_bytes([
            *bytes.get(at)?,
            *bytes.get(at + 1)?,
        ])))
    };
    let u16_le = |at: usize| {
        Some(u32::from(u16::from_le_bytes([
            *bytes.get(at)?,
            *bytes.get(at + 1)?,
        ])))
    };
    let u32_be = |at: usize| Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?));
    let (format, width, height) = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        if bytes.get(12..16)? != b"IHDR" {
            return None;
        }
        (Format::Png, u32_be(16)?, u32_be(20)?)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        (Format::Gif, u16_le(6)?, u16_le(8)?)
    } else if bytes.starts_with(&[0xFF, 0xD8]) {
        let mut at = 2;
        loop {
            if *bytes.get(at)? != 0xFF {
                return None;
            }
            let marker = *bytes.get(at + 1)?;
            match marker {
                0xFF => at += 1,
                0x01 | 0xD0..=0xD7 => at += 2,
                0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                    break (Format::Jpeg, u16_be(at + 7)?, u16_be(at + 5)?);
                }
                0xD9 | 0xDA => return None,
                _ => at += 2 + usize::try_from(u16_be(at + 2)?).ok()?,
            }
        }
    } else {
        return None;
    };
    (width > 0 && height > 0).then_some((format, width, height))
}

/// What an anchor places: an image by its media entry's name under `xl/media/`, or a chart by
/// its number in the workbook.
#[derive(Debug, Clone, PartialEq)]
enum Placed {
    Picture(String),
    Chart(usize, Planned),
}

/// One image's or chart's place on the sheet, every figure in EMU.
#[derive(Debug, Clone, PartialEq)]
struct Anchor {
    column: u32,
    row: u32,
    offset_x: u64,
    offset_y: u64,
    width: u64,
    height: u64,
    placed: Placed,
}

/// The images and charts of one sheet, until it ends.
#[derive(Debug, Default)]
pub struct Drawing {
    anchors: Vec<Anchor>,
}

fn emu(pixels: f64) -> u64 {
    (pixels * EMU_PER_PIXEL).round() as u64
}

/// Checks a figure in pixels of the image or chart named `part`, as "image at `C3`".
fn pixels(
    what: &str,
    part: &str,
    value: Option<f64>,
    positive: bool,
) -> Result<Option<f64>, Error> {
    match value {
        Some(value) if !value.is_finite() || value < 0.0 || (positive && value == 0.0) => {
            Err(Error::Invalid(format!(
                "The {part} has the {what} {value}. It must be a number of pixels{}.",
                if positive { " above 0" } else { ", 0 or more" }
            )))
        }
        value => Ok(value),
    }
}

impl Drawing {
    /// Checks `image` and keeps its anchor. Returns its format, `media` being the number its
    /// entry takes in the workbook. The image's bytes are the caller's to write.
    pub fn add(&mut self, image: &Image, media: usize) -> Result<Format, Error> {
        let cell = image.at.as_str();
        let (column, row) = cell_at(cell).ok_or_else(|| bad_cell("image", cell))?;
        let Some((format, own_width, own_height)) = read(&image.image) else {
            return Err(Error::Invalid(format!(
                "The image at `{cell}` is not a PNG, JPEG or GIF file. Convert it to one of these formats first."
            )));
        };
        let part = format!("image at `{cell}`");
        let width = pixels("width", &part, image.width, true)?;
        let height = pixels("height", &part, image.height, true)?;
        let ratio = f64::from(own_height) / f64::from(own_width);
        let (width, height) = match (width, height) {
            (Some(width), Some(height)) => (width, height),
            (Some(width), None) => (width, width * ratio),
            (None, Some(height)) => (height / ratio, height),
            (None, None) => (f64::from(own_width), f64::from(own_height)),
        };
        let offset_x = pixels("horizontal offset", &part, image.offset_x, false)?.unwrap_or(0.0);
        let offset_y = pixels("vertical offset", &part, image.offset_y, false)?.unwrap_or(0.0);
        self.anchors.push(Anchor {
            column,
            row,
            offset_x: emu(offset_x),
            offset_y: emu(offset_y),
            width: emu(width).max(1),
            height: emu(height).max(1),
            placed: Placed::Picture(format!("image{media}.{}", format.extension())),
        });
        Ok(format)
    }

    /// Checks `chart`, added to the sheet `sheet` when `Writer::column` gives `reach` as a
    /// column's last row, and keeps it with its anchor, `number` being its part's number in the
    /// workbook (`chart`'s module doc).
    pub fn chart(
        &mut self,
        chart: &Chart,
        number: usize,
        sheet: &str,
        reach: u32,
    ) -> Result<(), Error> {
        let cell = chart.placed.as_str();
        let (column, row) = cell_at(cell).ok_or_else(|| bad_cell("chart", cell))?;
        let part = format!("chart at `{cell}`");
        let width = pixels("width", &part, chart.width, true)?.unwrap_or(chart::DEFAULT_WIDTH);
        let height = pixels("height", &part, chart.height, true)?.unwrap_or(chart::DEFAULT_HEIGHT);
        let planned = chart::check(chart, sheet, reach)?;
        self.anchors.push(Anchor {
            column,
            row,
            offset_x: 0,
            offset_y: 0,
            width: emu(width).max(1),
            height: emu(height).max(1),
            placed: Placed::Chart(number, planned),
        });
        Ok(())
    }

    /// Whether the sheet has images or charts.
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// The chart parts, `xl/charts/chart<n>.xml` by name, their growing ranges ending at the
    /// row `last`.
    pub fn charts(&self, last: u32) -> Vec<(String, String)> {
        self.anchors
            .iter()
            .filter_map(|anchor| match &anchor.placed {
                Placed::Chart(number, planned) => {
                    Some((format!("xl/charts/chart{number}.xml"), planned.write(last)))
                }
                Placed::Picture(_) => None,
            })
            .collect()
    }

    /// The `drawing<n>.xml` part, one `oneCellAnchor` per image or chart in the order they were
    /// added.
    pub fn write(&self) -> String {
        let mut out = String::from(crate::writer::XML_HEAD);
        out.push_str(
            "<xdr:wsDr xmlns:xdr=\"http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing\" \
             xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">",
        );
        for (at, anchor) in self.anchors.iter().enumerate() {
            let Anchor {
                column,
                row,
                offset_x,
                offset_y,
                width,
                height,
                placed,
            } = anchor;
            let _ = write!(
                out,
                "<xdr:oneCellAnchor><xdr:from><xdr:col>{column}</xdr:col><xdr:colOff>{offset_x}</xdr:colOff>\
                 <xdr:row>{}</xdr:row><xdr:rowOff>{offset_y}</xdr:rowOff></xdr:from>\
                 <xdr:ext cx=\"{width}\" cy=\"{height}\"/>",
                row - 1,
            );
            if let Placed::Chart(number, _) = placed {
                let _ = write!(
                    out,
                    "<xdr:graphicFrame macro=\"\"><xdr:nvGraphicFramePr><xdr:cNvPr id=\"{}\" name=\"Chart {number}\"/>\
                     <xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>\
                     <xdr:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/></xdr:xfrm>\
                     <a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\">\
                     <c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:r=\"{RELS}\" r:id=\"rId{}\"/>\
                     </a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:oneCellAnchor>",
                    at + 2,
                    at + 1
                );
                continue;
            }
            let _ = write!(
                out,
                "<xdr:pic><xdr:nvPicPr><xdr:cNvPr id=\"{}\" name=\"Picture {}\"/>\
                 <xdr:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></xdr:cNvPicPr></xdr:nvPicPr>\
                 <xdr:blipFill><a:blip xmlns:r=\"{RELS}\" r:embed=\"rId{}\"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill>\
                 <xdr:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{width}\" cy=\"{height}\"/></a:xfrm>\
                 <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></xdr:spPr></xdr:pic>\
                 <xdr:clientData/></xdr:oneCellAnchor>",
                at + 2,
                at + 1,
                at + 1
            );
        }
        out.push_str("</xdr:wsDr>");
        out
    }

    /// The drawing's relationships part: `rId<n>` is the `n`th anchor's media entry or chart part.
    pub fn rels(&self) -> String {
        let mut out = String::from(crate::writer::XML_HEAD);
        let _ = write!(out, "<Relationships xmlns=\"{PACKAGE_RELS}\">");
        for (at, anchor) in self.anchors.iter().enumerate() {
            let _ = match &anchor.placed {
                Placed::Picture(media) => write!(
                    out,
                    "<Relationship Id=\"rId{}\" Type=\"{RELS}/image\" Target=\"../media/{media}\"/>",
                    at + 1
                ),
                Placed::Chart(number, _) => write!(
                    out,
                    "<Relationship Id=\"rId{}\" Type=\"{RELS}/chart\" Target=\"../charts/chart{number}.xml\"/>",
                    at + 1
                ),
            };
        }
        out.push_str("</Relationships>");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_jpeg_size_is_its_first_frame_header() {
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        jpeg.extend([
            0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x30, 0x00, 0x40, 0x01, 0x00,
        ]);
        assert_eq!(read(&jpeg), Some((Format::Jpeg, 64, 48)));
        assert_eq!(read(&jpeg[..10]), None);
    }
}
