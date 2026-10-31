//! A [`Style`] as an ods automatic style: a cell style for a cell format, and a text style for
//! a rich-text run.
//!
//! Each field goes to the property element ODF gives it:
//!
//! | Field | ods |
//! |---|---|
//! | `fill`, `border`, `borderColor`, `valign`, `wrap`, `rotate`, `locked`, `hideFormula` | `style:table-cell-properties` |
//! | `align`, `indent` | `style:paragraph-properties` |
//! | `font`, `size`, `bold`, `italic`, `strike`, `underline`, `color` | `style:text-properties` |
//!
//! `align: Fill` repeats the text across the cell (`style:repeat-content`). An indent is 10
//! points a level. A rotation counts counterclockwise, as Excel's does, from 0 to 359 degrees.
//! A border side with no `borderColor` is black. `format` is the cell style's number style,
//! which `numbers` writes.

use std::fmt::Write as _;

use crate::styles::argb;
use crate::{Align, Error, Line, Style, Underline, Valign, cells, xml};

/// `'#RRGGBB'` as ods writes a colour.
fn color(color: &str) -> Result<String, Error> {
    Ok(format!("#{}", &argb(color)?[2..]))
}

fn line(line: Line) -> &'static str {
    match line {
        Line::Thin => "0.75pt solid",
        Line::Medium => "1.75pt solid",
        Line::Thick => "2.5pt solid",
        Line::Dashed => "0.75pt dashed",
        Line::Dotted => "0.75pt dotted",
        Line::Double => "2.25pt double",
    }
}

/// Appends the cell style `name` for `style`, with the number style `data` when it has one.
pub fn cell(out: &mut String, name: &str, style: &Style, data: Option<&str>) -> Result<(), Error> {
    let _ = write!(
        out,
        "<style:style style:name=\"{name}\" style:family=\"table-cell\" style:parent-style-name=\"Default\""
    );
    if let Some(data) = data {
        let _ = write!(out, " style:data-style-name=\"{data}\"");
    }
    out.push('>');
    let mut cell = String::new();
    if let Some(fill) = &style.fill {
        let _ = write!(cell, " fo:background-color=\"{}\"", color(fill)?);
    }
    if let Some(border) = style.border {
        let ink = match &style.border_color {
            Some(ink) => color(ink)?,
            None => "#000000".to_owned(),
        };
        for (side, value) in [
            ("top", border.top),
            ("bottom", border.bottom),
            ("left", border.left),
            ("right", border.right),
        ] {
            if let Some(value) = value {
                let _ = write!(cell, " fo:border-{side}=\"{} {ink}\"", line(value));
            }
        }
    }
    if let Some(valign) = style.valign {
        let value = match valign {
            Valign::Top => "top",
            Valign::Center => "middle",
            Valign::Bottom => "bottom",
        };
        let _ = write!(cell, " style:vertical-align=\"{value}\"");
    }
    if style.wrap == Some(true) {
        cell.push_str(" fo:wrap-option=\"wrap\"");
    }
    if let Some(rotate) = style.rotate {
        let angle = rotate.rem_euclid(360);
        let _ = write!(
            cell,
            " style:rotation-angle=\"{angle}\" style:rotation-align=\"none\""
        );
    }
    if style.align == Some(Align::Fill) {
        cell.push_str(" style:repeat-content=\"true\"");
    }
    if style.locked.is_some() || style.hide_formula.is_some() {
        let value = match (
            style.locked.unwrap_or(true),
            style.hide_formula.unwrap_or(false),
        ) {
            (true, false) => "protected",
            (false, false) => "none",
            (true, true) => "protected formula-hidden",
            (false, true) => "formula-hidden",
        };
        let _ = write!(cell, " style:cell-protect=\"{value}\"");
    }
    if !cell.is_empty() {
        let _ = write!(out, "<style:table-cell-properties{cell}/>");
    }
    let mut paragraph = String::new();
    if let Some(align) = style.align {
        let value = match align {
            Align::Left => Some("start"),
            Align::Center => Some("center"),
            Align::Right => Some("end"),
            Align::Justify => Some("justify"),
            Align::Fill => None,
        };
        if let Some(value) = value {
            let _ = write!(paragraph, " fo:text-align=\"{value}\"");
        }
    }
    if let Some(indent) = style.indent.filter(|&indent| indent > 0) {
        let _ = write!(paragraph, " fo:margin-left=\"{}pt\"", indent * 10);
    }
    if !paragraph.is_empty() {
        let _ = write!(out, "<style:paragraph-properties{paragraph}/>");
    }
    text(out, style)?;
    out.push_str("</style:style>");
    Ok(())
}

/// Appends the text style `name` for a rich-text run styled by `style`.
pub fn run(out: &mut String, name: &str, style: &Style) -> Result<(), Error> {
    let _ = write!(
        out,
        "<style:style style:name=\"{name}\" style:family=\"text\">"
    );
    text(out, style)?;
    out.push_str("</style:style>");
    Ok(())
}

/// Appends `style:text-properties` for the font fields of `style`, when it sets any.
fn text(out: &mut String, style: &Style) -> Result<(), Error> {
    let mut props = String::new();
    if let Some(font) = &style.font {
        props.push_str(" fo:font-family=\"");
        xml::odf(&mut props, font, true);
        props.push('"');
    }
    if let Some(size) = style.size {
        props.push_str(" fo:font-size=\"");
        cells::number(&mut props, size);
        props.push_str("pt\"");
    }
    if let Some(bold) = style.bold {
        let value = if bold { "bold" } else { "normal" };
        let _ = write!(props, " fo:font-weight=\"{value}\"");
    }
    if let Some(italic) = style.italic {
        let value = if italic { "italic" } else { "normal" };
        let _ = write!(props, " fo:font-style=\"{value}\"");
    }
    if let Some(strike) = style.strike {
        let value = if strike { "solid" } else { "none" };
        let _ = write!(props, " style:text-line-through-style=\"{value}\"");
    }
    if let Some(underline) = style.underline {
        props.push_str(
            " style:text-underline-style=\"solid\" style:text-underline-width=\"auto\" style:text-underline-color=\"font-color\"",
        );
        if underline == Underline::Double {
            props.push_str(" style:text-underline-type=\"double\"");
        }
    }
    if let Some(ink) = &style.color {
        let _ = write!(props, " fo:color=\"{}\"", color(ink)?);
    }
    if !props.is_empty() {
        let _ = write!(out, "<style:text-properties{props}/>");
    }
    Ok(())
}
