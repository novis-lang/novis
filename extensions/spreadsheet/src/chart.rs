//! Charts on a sheet: the `xl/charts/chart<n>.xml` part and its place in the sheet's drawing
//! (`rule:core-classes/spreadsheet-sheet-end-parts`).
//!
//! The chart XML is ported from `rust_xlsxwriter`'s output for the same chart
//! (<https://github.com/jmcnamara/rust_xlsxwriter>, by John McNamara, MIT or Apache-2.0), for the
//! subset only: column, bar, line, area, pie, doughnut and scatter, and a combination of two of
//! column, line and area over one pair of axes. A chart has a title, axis titles, a legend, and
//! per series a name, a colour and data labels. Nothing else of `rust_xlsxwriter` is used, and it
//! is not a dependency.
//!
//! **A series points at its rows by reference, and the chart keeps no value.** [`check`] reads
//! each range when the chart is added, through the reference parser `Workbook::define` uses
//! (`reference`'s module doc), so a range stays in the workbook. A range with no sheet name is on the sheet the chart is
//! added to. **A range that ends at the row `Writer::column` gives at that moment grows with the
//! sheet**: the chart is written when the sheet ends, and such a range ends at the sheet's last row
//! then. So a chart added after the first rows of a stream covers every row the sheet ends with.
//! A range that ends at any other row, or names another sheet, is written as given. Excel reads
//! the values when it opens the file, so the part has no cached values. A series' categories are
//! a number reference when they are columns of the chart's own sheet whose every value below the
//! header is a number, a date or a time (`drawing::Grid`), and a text reference otherwise.
//!
//! The chart is anchored from its cell to the cell its size ends at, 480 by 288 pixels unless the
//! program sets a size, as Excel's own default chart is (`drawing`'s module doc).
//!
//! Memory: per chart of the open sheet, its ranges and texts until the sheet ends.

use std::fmt::Write as _;

use crate::drawing::Grid;
use crate::reference::{self, Cells};
use crate::styles::argb;
use crate::tail::{Area, cell_at};
use crate::{Error, cells, xml};

/// The chart's size in pixels when the program sets none.
pub const DEFAULT_WIDTH: f64 = 480.0;
pub const DEFAULT_HEIGHT: f64 = 288.0;

/// The ids of the two axes; each chart is its own part, so every chart uses the same pair.
const X_AXIS: u32 = 50_010_001;
const Y_AXIS: u32 = 50_010_002;

/// The chart types of the subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Column,
    Bar,
    Line,
    Area,
    Pie,
    Doughnut,
    Scatter,
}

impl Kind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Column => "column",
            Kind::Bar => "bar",
            Kind::Line => "line",
            Kind::Area => "area",
            Kind::Pie => "pie",
            Kind::Doughnut => "doughnut",
            Kind::Scatter => "scatter",
        }
    }

    pub(crate) fn has_axes(self) -> bool {
        !matches!(self, Kind::Pie | Kind::Doughnut)
    }

    /// Whether a series is a line, so its colour is the line's and not a fill.
    pub(crate) fn is_line(self) -> bool {
        matches!(self, Kind::Line | Kind::Scatter)
    }
}

/// Where the legend is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Legend {
    Hidden,
    Top,
    #[default]
    Right,
    Bottom,
    Left,
}

/// One series: the range of its values, of its categories (a scatter's x values), its name and
/// its colour as `#RRGGBB`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Series {
    pub values: String,
    pub categories: Option<String>,
    pub name: Option<String>,
    pub color: Option<String>,
}

/// One chart type and its series. A combination is two layers.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub kind: Kind,
    pub series: Vec<Series>,
}

/// A chart the program adds at the cell `placed`, with an optional size in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Chart {
    pub placed: String,
    pub layers: Vec<Layer>,
    pub title: Option<String>,
    pub x_title: Option<String>,
    pub y_title: Option<String>,
    pub legend: Legend,
    pub labels: bool,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// A series range read: its sheet, quoted when it needs to be, its corners, whether it is on the
/// chart's own sheet and whether it grows with the sheet.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Range {
    sheet: String,
    area: Area,
    own: bool,
    grows: bool,
}

impl Range {
    /// Its corners, the bottom moved to `last` when it grows.
    fn reach(&self, last: u32) -> Area {
        let mut at = self.area;
        if self.grows {
            at.bottom = at.bottom.max(last);
        }
        at
    }

    /// The reference the part writes as element text, its bottom moved to `last` when it grows.
    /// The sheet name is the program's text, so it is escaped like any other.
    fn write(&self, out: &mut String, last: u32) {
        let at = self.reach(last);
        xml::text(out, &self.sheet);
        let _ = write!(
            out,
            "!{}:{}",
            absolute(at.left, at.top),
            absolute(at.right, at.bottom)
        );
    }

    /// The reference as an ods cell range address, such as `Shop.$B$2:Shop.$B$9`, its bottom
    /// moved to `last` when it grows. The caller escapes it as attribute text.
    pub(crate) fn odf(&self, last: u32) -> String {
        let at = self.reach(last);
        format!(
            "{0}.{1}:{0}.{2}",
            self.sheet,
            absolute(at.left, at.top),
            absolute(at.right, at.bottom)
        )
    }
}

fn absolute(column: u32, row: u32) -> String {
    let mut out = String::from("$");
    cells::column_letters(&mut out, column);
    let _ = write!(out, "${row}");
    out
}

/// The most characters Excel allows in a chart title or an axis title.
const MAX_TITLE: usize = 255;

/// A series checked: its ranges read and its colour as `RRGGBB`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Checked {
    pub(crate) values: Range,
    pub(crate) categories: Option<Range>,
    pub(crate) name: Option<String>,
    pub(crate) color: Option<String>,
}

/// A chart checked and kept until its sheet ends.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub(crate) layers: Vec<(Kind, Vec<Checked>)>,
    pub(crate) title: Option<String>,
    pub(crate) x_title: Option<String>,
    pub(crate) y_title: Option<String>,
    pub(crate) legend: Legend,
    pub(crate) labels: bool,
}

/// `name` as a reference writes it: quoted, as `'Shop orders'`, when Excel quotes it. That is a
/// name with a character other than a letter, a digit, `_` and `.`, a name that starts with a
/// digit or `.`, and a name that reads as a cell, such as `A1`, `R2` or `R1C1`.
pub(crate) fn quoted(name: &str) -> String {
    let plain = name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        && !name.starts_with(|c: char| c.is_ascii_digit() || c == '.')
        && cell_at(name).is_none()
        && !is_r1c1(name);
    if plain {
        name.to_owned()
    } else {
        format!("'{}'", name.replace('\'', "''"))
    }
}

/// Whether `name` reads as a cell in the R1C1 style: `R`, `C`, `RC`, `R2`, `C3` or `R2C3`, in
/// either case.
pub(crate) fn is_r1c1(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    let digits = |text: &str| text.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let rest = match upper.strip_prefix('R') {
        Some(after) => &after[after.len() - digits(after)..],
        None => upper.as_str(),
    };
    let rest = match rest.strip_prefix('C') {
        Some(after) => &after[after.len() - digits(after)..],
        None => rest,
    };
    !name.is_empty() && rest.is_empty()
}

/// Reads the series range `text` of the sheet `sheet` through [`reference::parse`]. `reach` is
/// the row `Writer::column` gives as a column's last row now.
fn range(text: &str, sheet: &str, reach: u32) -> Result<Range, Error> {
    let bad = || {
        Error::Invalid(format!(
            "The chart range `{text}` is not a range of cells in this workbook, such as `B2:B10` or `'Shop'!B2:B10`."
        ))
    };
    let read = reference::parse(text, false, bad)?;
    let Cells::Area(at) = read.cells else {
        return Err(bad());
    };
    let named = read.sheet;
    let own = named.as_deref().is_none_or(|name| name == sheet);
    Ok(Range {
        sheet: quoted(named.as_deref().unwrap_or(sheet)),
        area: at,
        own,
        grows: own && at.bottom == reach,
    })
}

/// Checks `chart`, added to the sheet `sheet` when `Writer::column` gives `reach` as a column's
/// last row.
pub fn check(chart: &Chart, sheet: &str, reach: u32) -> Result<Planned, Error> {
    let cell = chart.placed.as_str();
    let kinds: Vec<Kind> = chart.layers.iter().map(|layer| layer.kind).collect();
    match kinds[..] {
        [] => {
            return Err(Error::Invalid(format!(
                "The chart at `{cell}` has no type. Start it with a type, such as `Chart::column()`."
            )));
        }
        [_] => {}
        [first, second]
            if first != second
                && [first, second]
                    .iter()
                    .all(|kind| matches!(kind, Kind::Column | Kind::Line | Kind::Area)) => {}
        _ => {
            let names: Vec<&str> = kinds.iter().map(|kind| kind.name()).collect();
            return Err(Error::Invalid(format!(
                "The chart at `{cell}` combines {}. A chart combines two different types of column, line and area.",
                names.join(" and ")
            )));
        }
    }
    for (what, text) in [
        ("title", &chart.title),
        ("x-axis title", &chart.x_title),
        ("y-axis title", &chart.y_title),
    ] {
        if let Some(text) = text
            && text.chars().count() > MAX_TITLE
        {
            return Err(Error::Invalid(format!(
                "The {what} of the chart at `{cell}` has {} characters. A chart title has at most 255.",
                text.chars().count()
            )));
        }
    }
    let mut layers = Vec::new();
    for layer in &chart.layers {
        if layer.series.is_empty() {
            return Err(Error::Invalid(format!(
                "The {} chart at `{cell}` has no series. Add one with `series()`.",
                layer.kind.name()
            )));
        }
        let mut checked = Vec::new();
        for series in &layer.series {
            checked.push(Checked {
                values: range(&series.values, sheet, reach)?,
                categories: series
                    .categories
                    .as_deref()
                    .map(|text| range(text, sheet, reach))
                    .transpose()?,
                name: series.name.clone(),
                color: series
                    .color
                    .as_deref()
                    .map(|color| argb(color).map(|argb| argb[2..].to_owned()))
                    .transpose()?,
            });
        }
        layers.push((layer.kind, checked));
    }
    Ok(Planned {
        layers,
        title: chart.title.clone(),
        x_title: chart.x_title.clone(),
        y_title: chart.y_title.clone(),
        legend: chart.legend,
        labels: chart.labels,
    })
}

/// A title of rich text, turned to read upwards when `upright` is set.
fn title(out: &mut String, text: &str, upright: bool) {
    out.push_str("<c:title><c:tx><c:rich>");
    out.push_str(if upright {
        "<a:bodyPr rot=\"-5400000\" vert=\"horz\"/>"
    } else {
        "<a:bodyPr/>"
    });
    out.push_str("<a:lstStyle/><a:p><a:pPr><a:defRPr/></a:pPr><a:r><a:t>");
    xml::text(out, text);
    out.push_str("</a:t></a:r></a:p></c:rich></c:tx><c:overlay val=\"0\"/></c:title>");
}

fn fill(out: &mut String, color: &str) {
    let _ = write!(
        out,
        "<a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>"
    );
}

impl Planned {
    /// The `chart<n>.xml` part, its growing ranges ending at the row `last`, over the sheet's
    /// `grid`.
    pub fn write(&self, last: u32, grid: &Grid) -> String {
        let mut out = String::from(crate::writer::XML_HEAD);
        out.push_str(
            "<c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" \
             xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
             xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
             <c:lang val=\"en-US\"/><c:chart>",
        );
        if let Some(text) = &self.title {
            title(&mut out, text, false);
        }
        out.push_str("<c:plotArea><c:layout/>");
        let mut index = 0;
        for (kind, series) in &self.layers {
            self.layer(&mut out, *kind, series, &mut index, last, grid);
        }
        let first = self.layers[0].0;
        if first.has_axes() {
            self.axes(&mut out);
        }
        out.push_str("</c:plotArea>");
        if let Some(position) = match self.legend {
            Legend::Hidden => None,
            Legend::Top => Some("t"),
            Legend::Right => Some("r"),
            Legend::Bottom => Some("b"),
            Legend::Left => Some("l"),
        } {
            let _ = write!(
                out,
                "<c:legend><c:legendPos val=\"{position}\"/><c:layout/>"
            );
            if !first.has_axes() {
                out.push_str(
                    "<c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr rtl=\"0\"><a:defRPr/></a:pPr>\
                     <a:endParaRPr lang=\"en-US\"/></a:p></c:txPr>",
                );
            }
            out.push_str("</c:legend>");
        }
        out.push_str(
            "<c:plotVisOnly val=\"1\"/></c:chart>\
             <c:printSettings><c:headerFooter/>\
             <c:pageMargins b=\"0.75\" l=\"0.7\" r=\"0.7\" t=\"0.75\" header=\"0.3\" footer=\"0.3\"/>\
             <c:pageSetup/></c:printSettings></c:chartSpace>",
        );
        out
    }

    /// One chart type's element with its series, `index` counting series across the chart.
    fn layer(
        &self,
        out: &mut String,
        kind: Kind,
        series: &[Checked],
        index: &mut usize,
        last: u32,
        grid: &Grid,
    ) {
        let element = match kind {
            Kind::Column | Kind::Bar => "barChart",
            Kind::Line => "lineChart",
            Kind::Area => "areaChart",
            Kind::Pie => "pieChart",
            Kind::Doughnut => "doughnutChart",
            Kind::Scatter => "scatterChart",
        };
        let _ = write!(out, "<c:{element}>");
        out.push_str(match kind {
            Kind::Column => "<c:barDir val=\"col\"/><c:grouping val=\"clustered\"/>",
            Kind::Bar => "<c:barDir val=\"bar\"/><c:grouping val=\"clustered\"/>",
            Kind::Line | Kind::Area => "<c:grouping val=\"standard\"/>",
            Kind::Scatter => "<c:scatterStyle val=\"lineMarker\"/>",
            Kind::Pie | Kind::Doughnut => "",
        });
        if !kind.has_axes() {
            out.push_str("<c:varyColors val=\"1\"/>");
        }
        for one in series {
            self.series(out, kind, one, *index, last, grid);
            *index += 1;
        }
        out.push_str(match kind {
            Kind::Line => "<c:marker val=\"1\"/>",
            Kind::Pie => "<c:firstSliceAng val=\"0\"/>",
            Kind::Doughnut => "<c:firstSliceAng val=\"0\"/><c:holeSize val=\"50\"/>",
            _ => "",
        });
        if kind.has_axes() {
            let _ = write!(out, "<c:axId val=\"{X_AXIS}\"/><c:axId val=\"{Y_AXIS}\"/>");
        }
        let _ = write!(out, "</c:{element}>");
    }

    fn series(
        &self,
        out: &mut String,
        kind: Kind,
        series: &Checked,
        index: usize,
        last: u32,
        grid: &Grid,
    ) {
        let _ = write!(
            out,
            "<c:ser><c:idx val=\"{index}\"/><c:order val=\"{index}\"/>"
        );
        if let Some(name) = &series.name {
            out.push_str("<c:tx><c:v>");
            xml::text(out, name);
            out.push_str("</c:v></c:tx>");
        }
        match (kind, &series.color) {
            (Kind::Scatter, color) => {
                out.push_str("<c:spPr><a:ln w=\"28575\"><a:noFill/></a:ln></c:spPr>");
                if let Some(color) = color {
                    out.push_str("<c:marker><c:symbol val=\"circle\"/><c:size val=\"7\"/><c:spPr>");
                    fill(out, color);
                    out.push_str("<a:ln>");
                    fill(out, color);
                    out.push_str("</a:ln></c:spPr></c:marker>");
                }
            }
            (kind, Some(color)) if kind.is_line() => {
                out.push_str("<c:spPr><a:ln w=\"28575\" cap=\"rnd\">");
                fill(out, color);
                out.push_str("<a:round/></a:ln></c:spPr>");
            }
            (_, Some(color)) => {
                out.push_str("<c:spPr>");
                fill(out, color);
                out.push_str("</c:spPr>");
            }
            (_, None) => {}
        }
        if kind == Kind::Line {
            out.push_str("<c:marker><c:symbol val=\"none\"/></c:marker>");
        }
        if self.labels {
            out.push_str(
                "<c:dLbls><c:showLegendKey val=\"0\"/><c:showVal val=\"1\"/><c:showCatName val=\"0\"/>\
                 <c:showSerName val=\"0\"/><c:showPercent val=\"0\"/><c:showBubbleSize val=\"0\"/></c:dLbls>",
            );
        }
        let (categories, values) = if kind == Kind::Scatter {
            ("xVal", "yVal")
        } else {
            ("cat", "val")
        };
        if let Some(range) = &series.categories {
            let numbers = kind == Kind::Scatter
                || (range.own && grid.numbers(range.area.left, range.area.right));
            let reference = if numbers { "numRef" } else { "strRef" };
            let _ = write!(out, "<c:{categories}><c:{reference}><c:f>");
            range.write(out, last);
            let _ = write!(out, "</c:f></c:{reference}></c:{categories}>");
        }
        let _ = write!(out, "<c:{values}><c:numRef><c:f>");
        series.values.write(out, last);
        let _ = write!(out, "</c:f></c:numRef></c:{values}></c:ser>");
    }

    /// The two axes: categories and values, a scatter's two value axes, and a bar chart's turned
    /// so its categories are on the left.
    fn axes(&self, out: &mut String) {
        let first = self.layers[0].0;
        let (category_at, value_at) = if first == Kind::Bar {
            ("l", "b")
        } else {
            ("b", "l")
        };
        let between = if self
            .layers
            .iter()
            .any(|(kind, _)| matches!(kind, Kind::Column | Kind::Bar | Kind::Line))
        {
            "between"
        } else {
            "midCat"
        };
        let scaling = "<c:scaling><c:orientation val=\"minMax\"/></c:scaling>";
        let format = "<c:numFmt formatCode=\"General\" sourceLinked=\"1\"/>";
        let ticks = "<c:tickLblPos val=\"nextTo\"/>";
        // Excel gives the category axis a number format only when a series has categories.
        let categories = first == Kind::Scatter
            || self
                .layers
                .iter()
                .any(|(_, series)| series.iter().any(|one| one.categories.is_some()));
        if first == Kind::Scatter {
            let _ = write!(
                out,
                "<c:valAx><c:axId val=\"{X_AXIS}\"/>{scaling}<c:axPos val=\"b\"/>"
            );
        } else {
            let _ = write!(
                out,
                "<c:catAx><c:axId val=\"{X_AXIS}\"/>{scaling}<c:axPos val=\"{category_at}\"/>"
            );
        }
        if let Some(text) = &self.x_title {
            title(out, text, category_at == "l");
        }
        if categories {
            out.push_str(format);
        }
        let _ = write!(
            out,
            "{ticks}<c:crossAx val=\"{Y_AXIS}\"/><c:crosses val=\"autoZero\"/>"
        );
        if first == Kind::Scatter {
            out.push_str("<c:crossBetween val=\"midCat\"/></c:valAx>");
        } else {
            out.push_str(
                "<c:auto val=\"1\"/><c:lblAlgn val=\"ctr\"/><c:lblOffset val=\"100\"/></c:catAx>",
            );
        }
        let _ = write!(
            out,
            "<c:valAx><c:axId val=\"{Y_AXIS}\"/>{scaling}<c:axPos val=\"{value_at}\"/><c:majorGridlines/>"
        );
        if let Some(text) = &self.y_title {
            title(out, text, value_at == "l");
        }
        let _ = write!(
            out,
            "{format}{ticks}<c:crossAx val=\"{X_AXIS}\"/><c:crosses val=\"autoZero\"/><c:crossBetween val=\"{}\"/></c:valAx>",
            if first == Kind::Scatter {
                "midCat"
            } else {
                between
            }
        );
    }
}
