//! Images and charts on a sheet: the media entries and the sheet's `drawing<n>.xml`
//! (`rule:core-classes/spreadsheet-sheet-end-parts`).
//!
//! [`read`] checks an image's bytes and finds its format and its size in pixels from the file's
//! own header: PNG's `IHDR`, GIF's logical screen and JPEG's first start-of-frame segment. Any
//! other bytes are an error, and nothing is converted. The image is the zip entry
//! `xl/media/image<n>.<ext>`, numbered across the workbook, and [`Drawing`] keeps only where each
//! one is placed: its cell, an offset from the cell's top-left corner and a size, in pixels. A
//! size the program leaves out is the image's own, and one side given alone scales the other by
//! the image's aspect ratio. A chart is placed the same way, as a `graphicFrame` pointing at its
//! own `xl/charts/chart<n>.xml` (`chart`'s module doc).
//!
//! [`Drawing::write`] is the `xl/drawings/drawing<sheet>.xml` part, written when the sheet ends,
//! with one `twoCellAnchor` per image or chart, as Excel writes them: from its cell and offset to
//! the cell its size ends at and the offset in that cell, in EMU (9,525 to a pixel). An image's
//! anchor is `editAs="oneCell"` and carries its position on the sheet, and a chart's carries
//! none. The end cell comes from the sheet's [`Grid`]: each column's width and each row's height
//! in pixels, as Excel turns them into pixels at its default font. [`Drawing::rels`] is the
//! part's relationships to the media and the charts.
//!
//! Memory: per image, its anchor and its entry's name, and per chart its anchor and what
//! `chart`'s module doc keeps. The image bytes go to the archive at once, or, when the sheet's own
//! entry is open, wait for it to end (`writer`'s module doc). Per open sheet, the [`Grid`]: one
//! width per column up to the last one with its own width, one run of 8 bytes per change of row
//! height, and one flag per column, until the sheet ends. A sheet with no custom row height keeps
//! no run.

use std::fmt::Write as _;

use crate::chart::{self, Chart, Planned};
use crate::tail::{bad_cell, cell_at};
use crate::{Error, Row, cells};

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

/// One image's or chart's place on the sheet: its column and row from 0, and every figure in
/// pixels.
#[derive(Debug, Clone, PartialEq)]
struct Anchor {
    column: u32,
    row: u32,
    offset_x: f64,
    offset_y: f64,
    width: f64,
    height: f64,
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

/// The width in pixels of a column with no width of its own, Excel's 8.43 characters.
const DEFAULT_COLUMN: f64 = 64.0;
/// The height in pixels of a row with no height of its own, the sheet's 15 points.
const DEFAULT_ROW: u32 = 20;
/// The last column and the last row a sheet has, from 0.
const LAST_COLUMN: u32 = 16_383;
const LAST_ROW: u32 = 1_048_575;

/// What the open sheet's anchors and charts need of its grid, kept as its head and its rows are
/// written: each column's width in pixels, the rows' heights as runs, and which columns have a
/// value that is not a number, a date or a time below the header.
#[derive(Debug, Default)]
pub struct Grid {
    /// The width of each column from the first up to the last that has a width of its own.
    widths: Vec<f64>,
    /// `(row, height)`: from the row, counted from 0, each row is the height in pixels, until the
    /// next run. A row before the first run is the default height.
    heights: Vec<(u32, u32)>,
    /// The number of rows written; every row from here on is the default height.
    written: u32,
    /// Per column, whether a written value below the header is not stored as a number.
    text: Vec<bool>,
}

impl Grid {
    /// Sets the width of the column `column`, from 0, to `width` characters as the sheet writes
    /// it, or to nothing when `hidden`. Excel's pixels at its default font, whose digits are 7
    /// pixels wide, are `trunc((256 * width + trunc(128 / 7)) / 256 * 7)`.
    pub fn width(&mut self, column: u32, width: f64, hidden: bool) {
        let at = column as usize;
        if self.widths.len() <= at {
            self.widths.resize(at + 1, DEFAULT_COLUMN);
        }
        self.widths[at] = if hidden {
            0.0
        } else {
            ((256.0 * width + 18.0) / 256.0 * 7.0).trunc()
        };
    }

    /// Notes the row written next: its height in points, if it has its own, whether it is hidden,
    /// and, unless it is the header row, each of its values.
    pub fn row(&mut self, row: &Row, hidden: bool) {
        let pixels = match row.height {
            _ if hidden => 0,
            Some(points) => (points * 4.0 / 3.0).round() as u32,
            None => DEFAULT_ROW,
        };
        let current = self
            .heights
            .last()
            .map_or(DEFAULT_ROW, |&(_, height)| height);
        if pixels != current {
            self.heights.push((self.written, pixels));
        }
        if !row.header {
            for (column, cell) in row.cells.iter().enumerate() {
                let Some(value) = &cell.value else { continue };
                if cells::stored_number(value).is_none() {
                    if self.text.len() <= column {
                        self.text.resize(column + 1, false);
                    }
                    self.text[column] = true;
                }
            }
        }
        self.written += 1;
    }

    /// Whether every value the columns from `left` to `right` hold below the header is a number,
    /// a date or a time.
    pub fn numbers(&self, left: u32, right: u32) -> bool {
        (left..=right).all(|column| !self.text.get(column as usize).copied().unwrap_or(false))
    }

    fn column(&self, column: u32) -> f64 {
        self.widths
            .get(column as usize)
            .copied()
            .unwrap_or(DEFAULT_COLUMN)
    }

    fn row_height(&self, row: u32) -> u32 {
        if row >= self.written {
            return DEFAULT_ROW;
        }
        let after = self.heights.partition_point(|&(start, _)| start <= row);
        after
            .checked_sub(1)
            .map_or(DEFAULT_ROW, |at| self.heights[at].1)
    }

    /// The first row after `row` whose height may differ from it.
    fn next_change(&self, row: u32) -> u32 {
        let after = self.heights.partition_point(|&(start, _)| start <= row);
        match self.heights.get(after) {
            Some(&(start, _)) => start,
            None if row < self.written => self.written,
            None => LAST_ROW + 1,
        }
    }

    /// The distance in pixels from the sheet's left edge to the column `column`.
    fn left(&self, column: u32) -> f64 {
        (0..column).map(|at| self.column(at)).sum()
    }

    /// The distance in pixels from the sheet's top edge to the row `row`.
    fn top(&self, row: u32) -> f64 {
        let mut total = 0.0;
        let (mut at, mut height) = (0, DEFAULT_ROW);
        for &(start, next) in &self.heights {
            if start >= row {
                break;
            }
            total += f64::from(start - at) * f64::from(height);
            (at, height) = (start, next);
        }
        let end = row.min(self.written).max(at);
        total += f64::from(end - at) * f64::from(height);
        total + f64::from(row.saturating_sub(self.written)) * f64::from(DEFAULT_ROW)
    }

    /// The column at or right of `column` that the distance `x` from its left edge falls in, and
    /// the distance left inside it.
    fn across(&self, mut column: u32, mut x: f64) -> (u32, f64) {
        while column < LAST_COLUMN && x >= self.column(column) {
            x -= self.column(column);
            column += 1;
        }
        (column, x)
    }

    /// The row at or below `row` that the distance `y` from its top edge falls in, and the
    /// distance left inside it. A run of hidden rows is passed in one step.
    fn down(&self, mut row: u32, mut y: f64) -> (u32, f64) {
        while row < LAST_ROW {
            let height = self.row_height(row);
            if height == 0 {
                row = self.next_change(row).min(LAST_ROW);
                continue;
            }
            if y < f64::from(height) {
                break;
            }
            y -= f64::from(height);
            row += 1;
        }
        (row, y)
    }
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

/// Where an image goes and how big it is: its column and row from 0, and every figure in
/// pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub format: Format,
    pub column: u32,
    pub row: u32,
    pub offset_x: f64,
    pub offset_y: f64,
    pub width: f64,
    pub height: f64,
}

/// Checks `image` and gives its format, its cell and its size. A size left out is the image's
/// own, and one side given alone scales the other by the image's aspect ratio. The xlsx and the
/// ods writer both place an image with this.
pub fn place(image: &Image) -> Result<Placement, Error> {
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
    Ok(Placement {
        format,
        column,
        row: row - 1,
        offset_x,
        offset_y,
        width,
        height,
    })
}

impl Drawing {
    /// Checks `image` and keeps its anchor. Returns its format, `media` being the number its
    /// entry takes in the workbook. The image's bytes are the caller's to write.
    pub fn add(&mut self, image: &Image, media: usize) -> Result<Format, Error> {
        let placed = place(image)?;
        self.anchors.push(Anchor {
            column: placed.column,
            row: placed.row,
            offset_x: placed.offset_x,
            offset_y: placed.offset_y,
            width: placed.width,
            height: placed.height,
            placed: Placed::Picture(format!("image{media}.{}", placed.format.extension())),
        });
        Ok(placed.format)
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
            row: row - 1,
            offset_x: 0.0,
            offset_y: 0.0,
            width,
            height,
            placed: Placed::Chart(number, planned),
        });
        Ok(())
    }

    /// Whether the sheet has images or charts.
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }

    /// The chart parts, `xl/charts/chart<n>.xml` by name, their growing ranges ending at the
    /// row `last`, over the sheet's `grid`.
    pub fn charts(&self, last: u32, grid: &Grid) -> Vec<(String, String)> {
        self.anchors
            .iter()
            .filter_map(|anchor| match &anchor.placed {
                Placed::Chart(number, planned) => Some((
                    format!("xl/charts/chart{number}.xml"),
                    planned.write(last, grid),
                )),
                Placed::Picture(_) => None,
            })
            .collect()
    }

    /// The `drawing<n>.xml` part over the sheet's `grid`, one `twoCellAnchor` per image or chart
    /// in the order they were added.
    pub fn write(&self, grid: &Grid) -> String {
        let mut out = String::from(crate::writer::XML_HEAD);
        out.push_str(
            "<xdr:wsDr xmlns:xdr=\"http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing\" \
             xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">",
        );
        for (at, anchor) in self.anchors.iter().enumerate() {
            let (column, x) = grid.across(anchor.column, anchor.offset_x);
            let (row, y) = grid.down(anchor.row, anchor.offset_y);
            let (end_column, end_x) = grid.across(column, x + anchor.width);
            let (end_row, end_y) = grid.down(row, y + anchor.height);
            let edit = match anchor.placed {
                Placed::Picture(_) => " editAs=\"oneCell\"",
                Placed::Chart(..) => "",
            };
            let _ = write!(
                out,
                "<xdr:twoCellAnchor{edit}><xdr:from><xdr:col>{column}</xdr:col><xdr:colOff>{}</xdr:colOff>\
                 <xdr:row>{row}</xdr:row><xdr:rowOff>{}</xdr:rowOff></xdr:from>\
                 <xdr:to><xdr:col>{end_column}</xdr:col><xdr:colOff>{}</xdr:colOff>\
                 <xdr:row>{end_row}</xdr:row><xdr:rowOff>{}</xdr:rowOff></xdr:to>",
                emu(x),
                emu(y),
                emu(end_x),
                emu(end_y),
            );
            if let Placed::Chart(number, _) = &anchor.placed {
                let _ = write!(
                    out,
                    "<xdr:graphicFrame macro=\"\"><xdr:nvGraphicFramePr><xdr:cNvPr id=\"{}\" name=\"Chart {number}\"/>\
                     <xdr:cNvGraphicFramePr/></xdr:nvGraphicFramePr>\
                     <xdr:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/></xdr:xfrm>\
                     <a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\">\
                     <c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:r=\"{RELS}\" r:id=\"rId{}\"/>\
                     </a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor>",
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
                 <xdr:spPr><a:xfrm><a:off x=\"{}\" y=\"{}\"/><a:ext cx=\"{}\" cy=\"{}\"/></a:xfrm>\
                 <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></xdr:spPr></xdr:pic>\
                 <xdr:clientData/></xdr:twoCellAnchor>",
                at + 2,
                at + 1,
                at + 1,
                emu(grid.left(column) + x),
                emu(grid.top(row) + y),
                emu(anchor.width).max(1),
                emu(anchor.height).max(1),
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
    use crate::{Cell, Value};

    #[test]
    fn a_jpeg_size_is_its_first_frame_header() {
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        jpeg.extend([
            0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x30, 0x00, 0x40, 0x01, 0x00,
        ]);
        assert_eq!(read(&jpeg), Some((Format::Jpeg, 64, 48)));
        assert_eq!(read(&jpeg[..10]), None);
    }

    #[test]
    fn the_grid_turns_widths_and_heights_into_pixels_and_passes_hidden_rows() {
        let mut grid = Grid::default();
        grid.width(1, 20.0, false);
        grid.width(2, 20.0, true);
        assert_eq!(grid.left(3), 64.0 + 140.0);
        assert_eq!(
            grid.across(0, 300.0),
            (4, 32.0),
            "the hidden column is passed"
        );

        let tall = Row {
            height: Some(30.0),
            ..Row::default()
        };
        grid.row(&Row::default(), false);
        grid.row(&tall, false);
        for _ in 0..3 {
            grid.row(&Row::default(), true);
        }
        grid.row(&Row::default(), false);
        assert_eq!(grid.heights, vec![(1, 40), (2, 0), (5, 20)]);
        assert_eq!(grid.down(0, 100.0), (7, 0.0));
        assert_eq!(grid.top(7), 100.0);
        assert_eq!(grid.row_height(9), DEFAULT_ROW);
    }

    #[test]
    fn a_column_with_text_below_its_header_is_not_a_number_column() {
        let mut grid = Grid::default();
        let row = |values: Vec<Value>, header: bool| Row {
            cells: values
                .into_iter()
                .map(|value| Cell {
                    value: Some(value),
                    style: None,
                })
                .collect(),
            header,
            ..Row::default()
        };
        grid.row(
            &row(
                vec![Value::Text("Month".into()), Value::Text("Sum".into())],
                true,
            ),
            false,
        );
        grid.row(
            &row(vec![Value::Integer(1), Value::Text("n/a".into())], false),
            false,
        );
        assert!(grid.numbers(0, 0));
        assert!(!grid.numbers(0, 1));
    }
}
