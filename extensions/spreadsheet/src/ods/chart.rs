//! A chart in an ods: the embedded chart object `Object <n>/content.xml`, written from the same
//! [`Planned`] chart the xlsx writer writes (`rule:core-classes/spreadsheet-sheet-end-parts`).
//!
//! The object is an ODF chart document with one `chart:chart`. Its class is the first layer's
//! type, and each series names its own class, so a combination is one plot area with the series
//! of both types over one pair of axes. The types map to ODF's classes: column and bar are
//! `chart:bar`, a bar chart with `chart:vertical`, pie is `chart:circle` and doughnut is
//! `chart:ring`. A series points at its cells by `chart:values-cell-range-address`, and its
//! categories are the x axis's `chart:categories`, taken from the first series that has them. A
//! scatter series' x values are its `chart:domain`. A series name is text, so it is LibreOffice's
//! `loext:label-string`, because ODF 1.3 names a series only by a cell. Its colour fills a column,
//! bar, area or slice, and is the line of a line, and a scatter series is markers with no line.
//!
//! The object keeps no values: as in xlsx, the program opening the file reads them from the
//! ranges. The sheet's `draw:frame` with its `draw:object` is written in the anchor cell by
//! `super`, beside the images.
//!
//! Memory: each chart's checked ranges and texts, per writing request, until `finish`.

use std::fmt::Write as _;

use crate::chart::{Checked, Kind, Legend, Planned};
use crate::xml;

use super::{HEAD, pixel_inches};

/// The namespaces a chart object's `content.xml` uses.
const NAMESPACES: &str = "xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
     xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\" \
     xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" \
     xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" \
     xmlns:chart=\"urn:oasis:names:tc:opendocument:xmlns:chart:1.0\" \
     xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\" \
     xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\" \
     xmlns:loext=\"urn:org:documentfoundation:names:experimental:office:xmlns:loext:1.0\"";

/// The ODF chart class of `kind`.
fn class(kind: Kind) -> &'static str {
    match kind {
        Kind::Column | Kind::Bar => "chart:bar",
        Kind::Line => "chart:line",
        Kind::Area => "chart:area",
        Kind::Pie => "chart:circle",
        Kind::Doughnut => "chart:ring",
        Kind::Scatter => "chart:scatter",
    }
}

/// Appends a `chart:title` of `text`.
fn title(out: &mut String, text: &str) {
    out.push_str("<chart:title><text:p>");
    xml::odf(out, text, false);
    out.push_str("</text:p></chart:title>");
}

/// The ranges the chart reads, each as an ods cell range address with its growing ranges ending
/// at the row `last`, in series order: the frame's `draw:notify-on-update-of-ranges`.
pub fn ranges(planned: &Planned, last: u32) -> Vec<String> {
    let mut out = Vec::new();
    for (_, series) in &planned.layers {
        for one in series {
            out.extend(one.categories.as_ref().map(|range| range.odf(last)));
            out.push(one.values.odf(last));
        }
    }
    out
}

/// The `content.xml` of the chart object, `width` by `height` pixels, its growing ranges ending
/// at the row `last`.
pub fn object(planned: &Planned, width: f64, height: f64, last: u32) -> String {
    let first = planned.layers[0].0;
    let series: Vec<(Kind, &Checked)> = planned
        .layers
        .iter()
        .flat_map(|(kind, series)| series.iter().map(move |one| (*kind, one)))
        .collect();
    let mut out = String::from(HEAD);
    let _ = write!(
        out,
        "<office:document-content {NAMESPACES} office:version=\"1.3\"><office:automatic-styles>"
    );
    out.push_str("<style:style style:name=\"pa\" style:family=\"chart\"><style:chart-properties");
    if first == Kind::Bar {
        out.push_str(" chart:vertical=\"true\"");
    }
    out.push_str("/></style:style>");
    for (index, (kind, one)) in series.iter().enumerate() {
        series_style(&mut out, index, *kind, one, planned.labels);
    }
    let _ = write!(
        out,
        "</office:automatic-styles><office:body><office:chart>\
         <chart:chart svg:width=\"{}\" svg:height=\"{}\" chart:class=\"{}\">",
        pixel_inches(width),
        pixel_inches(height),
        class(first)
    );
    if let Some(text) = &planned.title {
        title(&mut out, text);
    }
    let position = match planned.legend {
        Legend::Hidden => None,
        Legend::Top => Some("top"),
        Legend::Right => Some("end"),
        Legend::Bottom => Some("bottom"),
        Legend::Left => Some("start"),
    };
    if let Some(position) = position {
        let _ = write!(out, "<chart:legend chart:legend-position=\"{position}\"/>");
    }
    out.push_str("<chart:plot-area chart:style-name=\"pa\">");
    let categories = series
        .iter()
        .filter(|(kind, _)| *kind != Kind::Scatter)
        .find_map(|(_, one)| one.categories.as_ref());
    if first.has_axes() || categories.is_some() {
        out.push_str("<chart:axis chart:dimension=\"x\" chart:name=\"primary-x\">");
        if first.has_axes()
            && let Some(text) = &planned.x_title
        {
            title(&mut out, text);
        }
        if let Some(range) = categories {
            out.push_str("<chart:categories table:cell-range-address=\"");
            xml::odf(&mut out, &range.odf(last), true);
            out.push_str("\"/>");
        }
        out.push_str("</chart:axis>");
    }
    if first.has_axes() {
        out.push_str("<chart:axis chart:dimension=\"y\" chart:name=\"primary-y\">");
        if let Some(text) = &planned.y_title {
            title(&mut out, text);
        }
        out.push_str("<chart:grid chart:class=\"major\"/></chart:axis>");
    }
    for (index, (kind, one)) in series.iter().enumerate() {
        let _ = write!(
            out,
            "<chart:series chart:style-name=\"s{index}\" chart:class=\"{}\" chart:values-cell-range-address=\"",
            class(*kind)
        );
        xml::odf(&mut out, &one.values.odf(last), true);
        out.push('"');
        if let Some(name) = &one.name {
            out.push_str(" loext:label-string=\"");
            xml::odf(&mut out, name, true);
            out.push('"');
        }
        out.push('>');
        if *kind == Kind::Scatter
            && let Some(range) = &one.categories
        {
            out.push_str("<chart:domain table:cell-range-address=\"");
            xml::odf(&mut out, &range.odf(last), true);
            out.push_str("\"/>");
        }
        out.push_str("</chart:series>");
    }
    out.push_str(
        "</chart:plot-area></chart:chart></office:chart></office:body></office:document-content>",
    );
    out
}

/// Appends the style `s<index>` of one series: its data labels, its markers and its colour.
fn series_style(out: &mut String, index: usize, kind: Kind, series: &Checked, labels: bool) {
    let _ = write!(
        out,
        "<style:style style:name=\"s{index}\" style:family=\"chart\"><style:chart-properties"
    );
    if labels {
        out.push_str(" chart:data-label-number=\"value\"");
    }
    match kind {
        Kind::Line => out.push_str(" chart:symbol-type=\"none\""),
        Kind::Scatter => out.push_str(" chart:symbol-type=\"automatic\""),
        _ => {}
    }
    out.push_str("/>");
    let color = series.color.as_deref();
    match (kind, color) {
        (Kind::Scatter, color) => {
            out.push_str("<style:graphic-properties draw:stroke=\"none\"");
            if let Some(color) = color {
                let _ = write!(out, " draw:fill-color=\"#{color}\"");
            }
            out.push_str("/>");
        }
        (kind, Some(color)) if kind.is_line() => {
            let _ = write!(
                out,
                "<style:graphic-properties svg:stroke-color=\"#{color}\" svg:stroke-width=\"0.0313in\"/>"
            );
        }
        (_, Some(color)) => {
            let _ = write!(
                out,
                "<style:graphic-properties draw:fill-color=\"#{color}\"/>"
            );
        }
        (_, None) => {}
    }
    out.push_str("</style:style>");
}
