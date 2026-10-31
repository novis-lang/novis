//! Style layers, the cell formats they combine into, and `xl/styles.xml`.
//!
//! [`Styles::layer`] checks a [`Style`], turns a format shortcut such as `'money'` into its code,
//! and gives the layer an id, the same id for an equal layer. A
//! cell's format is its column's layer, then its row's, then its own, each field of a later layer
//! replacing the earlier one's (`rule:core-classes/spreadsheet-styles-and-defaults`), with the
//! format the write table gives its value when no layer sets `format`
//! (`rule:core-classes/spreadsheet-value-types`). [`Styles::xf`] turns that combination into an
//! index of `cellXfs`, and equal combinations share one entry. Index 0 is the default format, so
//! a cell no layer styles carries no `s` attribute. A combination is resolved once and then read
//! from a cache keyed by the three layer ids and the value's format, so a row costs one lookup per
//! cell.
//!
//! [`Styles::write`] writes `styles.xml` at the end. Fonts, fills, borders and number formats are
//! shared between entries by their written XML, in the order the entries first use them, so the
//! file depends only on the order of the writes. A number format that is one of Excel's built-in
//! ids for numbers is written as that id. Every other code, and every date and time code, is a
//! custom `numFmt` from id 164. The first two fills are the `none` and `gray125` fills Excel
//! requires.
//!
//! [`Styles::dxf`] keeps the style of a conditional format as a `dxf`, the differential format
//! `styles.xml` lists after the cell formats. Equal styles share one, and its number format is
//! one of the same `numFmt` entries the cell formats use.
//!
//! The maps here are for lookup only. Nothing is written in a map's order.

use std::collections::HashMap;
use std::fmt::Write as _;

use crate::{Align, Error, Line, Style, Underline, Valign, xml};

/// The most cell formats Excel opens.
const MAX_XFS: usize = 65_490;

/// The format the write table gives a value, used when no layer sets `format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Implicit {
    None,
    Date,
    Time,
    Duration,
    Clock,
}

impl Implicit {
    pub(crate) fn code(self) -> Option<&'static str> {
        match self {
            Implicit::None => None,
            Implicit::Date => Some("yyyy-mm-dd"),
            Implicit::Time => Some("hh:mm:ss"),
            Implicit::Duration => Some("[h]:mm:ss"),
            Implicit::Clock => Some("yyyy-mm-dd hh:mm"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct XfKey {
    column: Option<u64>,
    row: Option<u64>,
    cell: Option<u64>,
    implicit: Implicit,
}

/// The style layers of one workbook and the cell formats written so far.
#[derive(Default)]
pub struct Styles {
    layers: Vec<Style>,
    layer_ids: HashMap<String, u64>,
    xfs: Vec<Style>,
    xf_ids: HashMap<String, u32>,
    cache: HashMap<XfKey, u32>,
    dxfs: Vec<Style>,
    dxf_ids: HashMap<String, u32>,
}

/// `'#RRGGBB'` as the ARGB text `styles.xml` writes.
pub fn argb(color: &str) -> Result<String, Error> {
    let hex = color.strip_prefix('#').unwrap_or("");
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::Invalid(format!(
            "`{color}` is not a color. Write a color as `#RRGGBB`, such as `#FF0000` for red."
        )));
    }
    Ok(format!("FF{}", hex.to_ascii_uppercase()))
}

fn check(style: &Style) -> Result<(), Error> {
    for color in [&style.color, &style.fill, &style.border_color]
        .into_iter()
        .flatten()
    {
        argb(color)?;
    }
    if let Some(size) = style.size
        && !(1.0..=409.0).contains(&size)
    {
        return Err(Error::Invalid(format!(
            "The font size {size} is not allowed. A font size is from 1 to 409."
        )));
    }
    if let Some(font) = &style.font
        && (font.is_empty() || font.chars().count() > 31)
    {
        return Err(Error::Invalid(format!(
            "`{font}` is not a font name. A font name has 1 to 31 characters."
        )));
    }
    if let Some(rotate) = style.rotate
        && !(-90..=90).contains(&rotate)
    {
        return Err(Error::Invalid(format!(
            "The rotation {rotate} is not allowed. Text rotates from -90 to 90 degrees."
        )));
    }
    if let Some(indent) = style.indent
        && indent > 250
    {
        return Err(Error::Invalid(format!(
            "The indent {indent} is not allowed. An indent is from 0 to 250."
        )));
    }
    if let Some(format) = &style.format
        && (format.is_empty() || format.chars().count() > 255)
    {
        return Err(Error::Invalid(
            "A number format has 1 to 255 characters.".to_owned(),
        ));
    }
    Ok(())
}

/// The code a format shortcut stands for.
fn shortcut(format: &str) -> Option<&'static str> {
    Some(match format {
        "date" => "yyyy-mm-dd",
        "datetime" => "yyyy-mm-dd hh:mm",
        "time" => "hh:mm:ss",
        "duration" => "[h]:mm:ss",
        "integer" => "0",
        "decimal" => "0.00",
        "money" => "#,##0.00",
        "percent" => "0%",
        "text" => "@",
        _ => return None,
    })
}

/// The layer every header row has by default: bold, with a thin bottom border.
pub fn header() -> Style {
    Style {
        bold: Some(true),
        border: Some(crate::Border {
            bottom: Some(Line::Thin),
            ..crate::Border::default()
        }),
        ..Style::default()
    }
}

impl Styles {
    /// The id of `style`, the same id for an equal style. A format shortcut becomes its code
    /// here, so `'money'` and `'#,##0.00'` are the same layer.
    pub fn layer(&mut self, mut style: Style) -> Result<u64, Error> {
        if let Some(code) = style.format.as_deref().and_then(shortcut) {
            style.format = Some(code.to_owned());
        }
        check(&style)?;
        let key = format!("{style:?}");
        if let Some(&id) = self.layer_ids.get(&key) {
            return Ok(id);
        }
        let id = self.layers.len() as u64;
        self.layers.push(style);
        self.layer_ids.insert(key, id);
        Ok(id)
    }

    /// The layer `id` names.
    pub fn get(&self, id: u64) -> Result<&Style, Error> {
        usize::try_from(id)
            .ok()
            .and_then(|at| self.layers.get(at))
            .ok_or_else(|| {
                Error::Invalid(format!("The style id {id} was not made by this workbook."))
            })
    }

    /// The `cellXfs` index of a cell styled by these layers, holding a value whose write-table
    /// format is `implicit`.
    pub fn xf(
        &mut self,
        column: Option<u64>,
        row: Option<u64>,
        cell: Option<u64>,
        implicit: Implicit,
    ) -> Result<u32, Error> {
        let key = XfKey {
            column,
            row,
            cell,
            implicit,
        };
        if let Some(&xf) = self.cache.get(&key) {
            return Ok(xf);
        }
        let mut merged = Style::default();
        for id in [column, row, cell].into_iter().flatten() {
            merged = self.get(id)?.over(&merged);
        }
        if merged.format.is_none() {
            merged.format = implicit.code().map(str::to_owned);
        }
        let xf = if merged == Style::default() {
            0
        } else {
            let text = format!("{merged:?}");
            match self.xf_ids.get(&text) {
                Some(&xf) => xf,
                None => {
                    if self.xfs.len() + 1 >= MAX_XFS {
                        return Err(Error::Invalid(
                            "The workbook has more than 65,490 cell formats. That is the most Excel allows."
                                .to_owned(),
                        ));
                    }
                    self.xfs.push(merged);
                    let xf = self.xfs.len() as u32;
                    self.xf_ids.insert(text, xf);
                    xf
                }
            }
        };
        self.cache.insert(key, xf);
        Ok(xf)
    }

    /// The format code of a cell styled by these layers, holding a value whose write-table format
    /// is `implicit`, or `None` for General. The most specific layer that sets `format` wins.
    pub fn code(
        &self,
        column: Option<u64>,
        row: Option<u64>,
        cell: Option<u64>,
        implicit: Implicit,
    ) -> Result<Option<&str>, Error> {
        for id in [cell, row, column].into_iter().flatten() {
            if let Some(code) = self.get(id)?.format.as_deref() {
                return Ok(Some(code));
            }
        }
        Ok(implicit.code())
    }

    /// Appends a rich-text run's `<rPr>` for the layer `id`.
    pub fn run_properties(&self, out: &mut String, id: u64) -> Result<(), Error> {
        let style = self.get(id)?;
        out.push_str("<rPr>");
        if let Some(font) = &style.font {
            out.push_str("<rFont val=\"");
            xml::attr(out, font);
            out.push_str("\"/>");
        }
        flags(out, style);
        if let Some(color) = &style.color {
            let _ = write!(out, "<color rgb=\"{}\"/>", argb(color)?);
        }
        if let Some(size) = style.size {
            out.push_str("<sz val=\"");
            crate::cells::number(out, size);
            out.push_str("\"/>");
        }
        underline(out, style);
        out.push_str("</rPr>");
        Ok(())
    }

    /// The `dxfs` index of `style` as a conditional format applies it, the same index for an
    /// equal style. A `dxf` has the font's bold, italic, strike, underline and colour, the fill,
    /// the border and the format, and the style's other fields are not written.
    pub fn dxf(&mut self, mut style: Style) -> Result<u32, Error> {
        if let Some(code) = style.format.as_deref().and_then(shortcut) {
            style.format = Some(code.to_owned());
        }
        check(&style)?;
        let key = format!("{style:?}");
        if let Some(&id) = self.dxf_ids.get(&key) {
            return Ok(id);
        }
        let id = self.dxfs.len() as u32;
        self.dxf_ids.insert(key, id);
        self.dxfs.push(style);
        Ok(id)
    }

    /// Writes `xl/styles.xml`.
    pub fn write(&self, out: &mut String) -> Result<(), Error> {
        let mut formats = Shared::default();
        let mut fonts = Shared::default();
        let mut fills = Shared::default();
        let mut borders = Shared::default();
        fonts.id(default_font());
        fills.id("<fill><patternFill patternType=\"none\"/></fill>".to_owned());
        fills.id("<fill><patternFill patternType=\"gray125\"/></fill>".to_owned());
        borders.id("<border><left/><right/><top/><bottom/><diagonal/></border>".to_owned());
        let mut entries = String::new();
        entries
            .push_str("<xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/>");
        for style in &self.xfs {
            let format = match style.format.as_deref() {
                None => 0,
                Some(code) => match builtin_format(code) {
                    Some(id) => id,
                    None => formats.id(code.to_owned()) + 164,
                },
            };
            let font = fonts.id(font(style)?);
            let fill = match &style.fill {
                None => 0,
                Some(color) => fills.id(format!(
                    "<fill><patternFill patternType=\"solid\"><fgColor rgb=\"{}\"/><bgColor indexed=\"64\"/></patternFill></fill>",
                    argb(color)?
                )),
            };
            let border = match &style.border {
                None => 0,
                Some(_) => borders.id(border(style, false)?),
            };
            let _ = write!(
                entries,
                "<xf numFmtId=\"{format}\" fontId=\"{font}\" fillId=\"{fill}\" borderId=\"{border}\" xfId=\"0\""
            );
            if format != 0 {
                entries.push_str(" applyNumberFormat=\"1\"");
            }
            if font != 0 {
                entries.push_str(" applyFont=\"1\"");
            }
            if fill != 0 {
                entries.push_str(" applyFill=\"1\"");
            }
            if border != 0 {
                entries.push_str(" applyBorder=\"1\"");
            }
            let alignment = alignment(style);
            let protection = protection(style);
            if !alignment.is_empty() {
                entries.push_str(" applyAlignment=\"1\"");
            }
            if !protection.is_empty() {
                entries.push_str(" applyProtection=\"1\"");
            }
            if alignment.is_empty() && protection.is_empty() {
                entries.push_str("/>");
            } else {
                entries.push('>');
                entries.push_str(&alignment);
                entries.push_str(&protection);
                entries.push_str("</xf>");
            }
        }
        let mut dxfs = String::new();
        for style in &self.dxfs {
            dxfs.push_str("<dxf>");
            if style.color.is_some()
                || style.underline.is_some()
                || [style.bold, style.italic, style.strike].contains(&Some(true))
            {
                dxfs.push_str("<font>");
                flags(&mut dxfs, style);
                underline(&mut dxfs, style);
                if let Some(color) = &style.color {
                    let _ = write!(dxfs, "<color rgb=\"{}\"/>", argb(color)?);
                }
                dxfs.push_str("</font>");
            }
            if let Some(code) = &style.format {
                let id = match builtin_format(code) {
                    Some(id) => id,
                    None => formats.id(code.clone()) + 164,
                };
                let _ = write!(dxfs, "<numFmt numFmtId=\"{id}\" formatCode=\"");
                xml::attr(&mut dxfs, code);
                dxfs.push_str("\"/>");
            }
            if let Some(color) = &style.fill {
                let _ = write!(
                    dxfs,
                    "<fill><patternFill><bgColor rgb=\"{}\"/></patternFill></fill>",
                    argb(color)?
                );
            }
            if style.border.is_some() {
                dxfs.push_str(&border(style, true)?);
            }
            dxfs.push_str("</dxf>");
        }
        out.push_str(crate::writer::XML_HEAD);
        out.push_str(
            "<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
        );
        if !formats.order.is_empty() {
            let _ = write!(out, "<numFmts count=\"{}\">", formats.order.len());
            for (at, code) in formats.order.iter().enumerate() {
                let _ = write!(out, "<numFmt numFmtId=\"{}\" formatCode=\"", at + 164);
                xml::attr(out, code);
                out.push_str("\"/>");
            }
            out.push_str("</numFmts>");
        }
        for (tag, shared) in [("fonts", &fonts), ("fills", &fills), ("borders", &borders)] {
            let _ = write!(out, "<{tag} count=\"{}\">", shared.order.len());
            for part in &shared.order {
                out.push_str(part);
            }
            let _ = write!(out, "</{tag}>");
        }
        out.push_str(
            "<cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>",
        );
        let _ = write!(out, "<cellXfs count=\"{}\">", self.xfs.len() + 1);
        out.push_str(&entries);
        out.push_str("</cellXfs>");
        out.push_str(
            "<cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles>",
        );
        if self.dxfs.is_empty() {
            out.push_str("<dxfs count=\"0\"/>");
        } else {
            let _ = write!(out, "<dxfs count=\"{}\">{dxfs}</dxfs>", self.dxfs.len());
        }
        out.push_str(
            "<tableStyles count=\"0\" defaultTableStyle=\"TableStyleMedium9\" defaultPivotStyle=\"PivotStyleLight16\"/>\
             </styleSheet>",
        );
        Ok(())
    }
}

/// Parts shared between cell formats by their XML, numbered in the order of first use.
#[derive(Default)]
struct Shared {
    order: Vec<String>,
    ids: HashMap<String, u32>,
}

impl Shared {
    fn id(&mut self, part: String) -> u32 {
        if let Some(&id) = self.ids.get(&part) {
            return id;
        }
        let id = self.order.len() as u32;
        self.ids.insert(part.clone(), id);
        self.order.push(part);
        id
    }
}

/// Excel's built-in number formats that are the same in every locale, by code.
fn builtin_format(code: &str) -> Option<u32> {
    Some(match code {
        "General" => 0,
        "0" => 1,
        "0.00" => 2,
        "#,##0" => 3,
        "#,##0.00" => 4,
        "0%" => 9,
        "0.00%" => 10,
        "0.00E+00" => 11,
        "# ?/?" => 12,
        "# ??/??" => 13,
        "@" => 49,
        _ => return None,
    })
}

fn default_font() -> String {
    "<font><sz val=\"11\"/><color theme=\"1\"/><name val=\"Calibri\"/><family val=\"2\"/><scheme val=\"minor\"/></font>".to_owned()
}

fn flags(out: &mut String, style: &Style) {
    if style.bold == Some(true) {
        out.push_str("<b/>");
    }
    if style.italic == Some(true) {
        out.push_str("<i/>");
    }
    if style.strike == Some(true) {
        out.push_str("<strike/>");
    }
}

fn underline(out: &mut String, style: &Style) {
    match style.underline {
        Some(Underline::Single) => out.push_str("<u/>"),
        Some(Underline::Double) => out.push_str("<u val=\"double\"/>"),
        None => {}
    }
}

fn font(style: &Style) -> Result<String, Error> {
    let plain = style.font.is_none()
        && style.size.is_none()
        && style.color.is_none()
        && style.underline.is_none()
        && style.bold != Some(true)
        && style.italic != Some(true)
        && style.strike != Some(true);
    if plain {
        return Ok(default_font());
    }
    let mut out = String::from("<font>");
    flags(&mut out, style);
    underline(&mut out, style);
    out.push_str("<sz val=\"");
    crate::cells::number(&mut out, style.size.unwrap_or(11.0));
    out.push_str("\"/>");
    match &style.color {
        Some(color) => {
            let _ = write!(out, "<color rgb=\"{}\"/>", argb(color)?);
        }
        None => out.push_str("<color theme=\"1\"/>"),
    }
    out.push_str("<name val=\"");
    xml::attr(&mut out, style.font.as_deref().unwrap_or("Calibri"));
    out.push_str("\"/><family val=\"2\"/>");
    if style.font.is_none() {
        out.push_str("<scheme val=\"minor\"/>");
    }
    out.push_str("</font>");
    Ok(out)
}

/// The `<border>` of `style`, for a `dxf` when `dxf` is set.
fn border(style: &Style, dxf: bool) -> Result<String, Error> {
    let color = style.border_color.as_deref().map(argb).transpose()?;
    let mut out = String::from("<border>");
    let border = style.border.unwrap_or_default();
    for (tag, line) in [
        ("left", border.left),
        ("right", border.right),
        ("top", border.top),
        ("bottom", border.bottom),
    ] {
        match line {
            None => {
                let _ = write!(out, "<{tag}/>");
            }
            Some(line) => {
                let name = match line {
                    Line::Thin => "thin",
                    Line::Medium => "medium",
                    Line::Thick => "thick",
                    Line::Dashed => "dashed",
                    Line::Dotted => "dotted",
                    Line::Double => "double",
                };
                match &color {
                    Some(rgb) => {
                        let _ = write!(
                            out,
                            "<{tag} style=\"{name}\"><color rgb=\"{rgb}\"/></{tag}>"
                        );
                    }
                    None => {
                        let _ = write!(out, "<{tag} style=\"{name}\"><color auto=\"1\"/></{tag}>");
                    }
                }
            }
        }
    }
    // A cell format's border ends in its diagonal. A `dxf`'s border has no diagonal, and ends in
    // the inside lines of a range instead, as Excel writes it.
    if dxf {
        out.push_str("<vertical/><horizontal/></border>");
    } else {
        out.push_str("<diagonal/></border>");
    }
    Ok(out)
}

fn alignment(style: &Style) -> String {
    let mut attrs = String::new();
    if let Some(align) = style.align {
        let name = match align {
            Align::Left => "left",
            Align::Center => "center",
            Align::Right => "right",
            Align::Fill => "fill",
            Align::Justify => "justify",
        };
        let _ = write!(attrs, " horizontal=\"{name}\"");
    }
    // Bottom is Excel's default, and Excel writes no attribute for it.
    if let Some(valign) = style.valign {
        let name = match valign {
            Valign::Top => Some("top"),
            Valign::Center => Some("center"),
            Valign::Bottom => None,
        };
        if let Some(name) = name {
            let _ = write!(attrs, " vertical=\"{name}\"");
        }
    }
    if let Some(rotate) = style.rotate
        && rotate != 0
    {
        // Excel counts a downward rotation from 91 to 180.
        let degrees = if rotate < 0 { 90 - rotate } else { rotate };
        let _ = write!(attrs, " textRotation=\"{degrees}\"");
    }
    if style.wrap == Some(true) {
        attrs.push_str(" wrapText=\"1\"");
    }
    if let Some(indent) = style.indent
        && indent > 0
    {
        let _ = write!(attrs, " indent=\"{indent}\"");
    }
    if attrs.is_empty() {
        attrs
    } else {
        format!("<alignment{attrs}/>")
    }
}

fn protection(style: &Style) -> String {
    let mut attrs = String::new();
    if style.locked == Some(false) {
        attrs.push_str(" locked=\"0\"");
    }
    if style.hide_formula == Some(true) {
        attrs.push_str(" hidden=\"1\"");
    }
    if attrs.is_empty() {
        attrs
    } else {
        format!("<protection{attrs}/>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold() -> Style {
        Style {
            bold: Some(true),
            ..Style::default()
        }
    }

    #[test]
    fn an_equal_layer_has_the_same_id_and_an_equal_combination_one_entry() {
        let mut styles = Styles::default();
        let a = styles.layer(bold()).unwrap();
        assert_eq!(styles.layer(bold()).unwrap(), a);
        let date = styles.xf(None, None, Some(a), Implicit::Date).unwrap();
        assert_eq!(
            styles.xf(None, Some(a), None, Implicit::Date).unwrap(),
            date
        );
        assert_eq!(styles.xf(None, None, None, Implicit::None).unwrap(), 0);
        assert_ne!(styles.xf(None, None, None, Implicit::Date).unwrap(), date);
    }

    #[test]
    fn a_bad_color_is_refused_naming_it() {
        let mut styles = Styles::default();
        let err = styles
            .layer(Style {
                color: Some("red".to_owned()),
                ..Style::default()
            })
            .unwrap_err();
        assert!(matches!(err, Error::Invalid(m) if m.contains("`red`")));
    }

    #[test]
    fn styles_xml_has_the_required_defaults_and_the_custom_formats() {
        let mut styles = Styles::default();
        styles.xf(None, None, None, Implicit::Clock).unwrap();
        let mut out = String::new();
        styles.write(&mut out).unwrap();
        assert!(out.contains("<numFmt numFmtId=\"164\" formatCode=\"yyyy-mm-dd hh:mm\"/>"));
        assert!(out.contains("patternType=\"gray125\""));
        assert!(out.contains("<cellXfs count=\"2\">"));
    }
}
